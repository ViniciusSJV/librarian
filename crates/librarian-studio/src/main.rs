//! Librarian Studio: local server for the page in web/index.html. Select a project, generate its
//! verified graph, ask questions that light the graph, and send prepared queries to an LLM.
//! Listens on 127.0.0.1 only; requests from other origins are refused.

mod claims;
mod execution;
mod facts;
mod http;
mod instrument;
mod ladder;
mod lexicon;
mod llm;
mod questions;
mod store;

use http::{Request, Response};
#[cfg(test)]
use librarian_ingest::evidence::RetryRule;
use librarian_ingest::graph::{uses::UseIndex, v2};
use librarian_llm_engine::models;
use serde_json::{json, Value};
use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};
use store::{Settings, Store};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct App {
    port: u16,
    viewer: PathBuf,
    store: Store,
    /// Last graph parsed for "Onde é usado?", by (project, run): large graphs take seconds to parse.
    graph: Mutex<Option<CachedGraph>>,
    /// Step 2.4d: the answers of every call of the last run asked, by target. In memory only.
    uses: Mutex<Option<CachedUses>>,
}

/// (project key, run id) and that run's parsed graph-v2.
type CachedGraph = ((String, String), Arc<v2::Graph>);
type CachedUses = ((String, String), Arc<UseIndex>);

fn main() {
    let mut port = 8765u16;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
            ("--port", Some(value)) => port = value.parse().expect("--port precisa de um número"),
            _ => {
                eprintln!("Uso: librarian_studio [--port 8765]");
                std::process::exit(2);
            }
        }
    }
    // The studio is a workspace tool: data and Modelfiles live next to the crates.
    let root = std::env::var_os("LIBRARIAN_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let root = root.canonicalize().expect("Raiz do Librarian não encontrada");
    let app = Arc::new(App {
        port,
        viewer: root.join("crates/librarian-studio/web/index.html"),
        store: Store { dir: root.join(".studio"), modelfiles: root.join("modelfiles") },
        graph: Mutex::new(None),
        uses: Mutex::new(None),
    });
    let listener = TcpListener::bind(("127.0.0.1", port)).unwrap_or_else(|e| {
        eprintln!("Não foi possível escutar em 127.0.0.1:{port}: {e}");
        std::process::exit(1);
    });
    println!("Librarian Studio em http://127.0.0.1:{port}/  (Ctrl+C encerra)");
    println!("Dados: {}", store::display(&app.store.dir));
    for stream in listener.incoming().flatten() {
        let app = Arc::clone(&app);
        thread::spawn(move || {
            let response = match http::read(&stream) {
                Ok(request) => handle(&app, &request),
                Err(e) => Response::error(400, e),
            };
            let _ = http::write(&stream, &response);
        });
    }
}

fn handle(app: &App, request: &Request) -> Response {
    // Same-machine only: a page from another site must not drive the studio (DNS rebinding, CSRF).
    let own = [format!("127.0.0.1:{}", app.port), format!("localhost:{}", app.port)];
    if !request.header("host").is_some_and(|h| own.iter().any(|o| o == h)) {
        return Response::error(403, "Host não permitido");
    }
    if let Some(origin) = request.header("origin") {
        if !own.iter().any(|o| origin == format!("http://{o}")) {
            return Response::error(403, "Origem não permitida");
        }
    }
    match route(app, request) {
        Ok(response) => response,
        Err(e) => Response::error(400, e.to_string()),
    }
}

fn route(app: &App, req: &Request) -> Result<Response> {
    let store = &app.store;
    let param = |name: &str| req.param(name).ok_or(format!("Parâmetro ausente: {name}"));
    let field = |body: &Value, name: &str| -> Result<String> {
        Ok(body[name].as_str().ok_or(format!("Campo ausente: {name}"))?.to_owned())
    };
    let ok = |value: Value| Ok(Response::json(200, &value));
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => Ok(Response::html(std::fs::read(&app.viewer)?)),
        ("GET", "/api/status") => ok(json!({
            "studio": true,
            "data": store::display(&store.dir),
            "llm": llm::status(&store.library()?),
            "models": store.library()?.summary()?,
            "skills": store.skills()?,
        })),
        ("GET", "/api/browse") => ok(store::browse(req.param("path"))?),
        ("POST", "/api/project") => ok(store.open(&field(&req.json()?, "root")?)?),
        ("GET", "/api/project") => ok(store.project(param("key")?)?),
        ("PUT", "/api/project") => {
            let body = req.json()?;
            let settings: Settings = serde_json::from_value(body["settings"].clone())?;
            ok(store.save_settings(&field(&body, "key")?, &settings)?)
        }
        ("POST", "/api/generate") => ok(store.generate(&field(&req.json()?, "key")?)?),
        ("GET", "/api/used-by") => {
            let depth = param("depth")?.parse()?;
            let (key, run) = (param("key")?, param("run")?);
            let graph = graph(app, key, run)?;
            let uses = uses(app, key, run, &graph)?;
            ok(serde_json::to_value(uses.used_by(&graph, param("node")?, depth, USED_BY_NODES)?)?)
        }
        ("POST", "/api/calls") => {
            // P1b: what a symbol calls; `chosen` maps a call occurrence to the homonym the person picked.
            let body = req.json()?;
            let (key, run) = (field(&body, "key")?, field(&body, "run")?);
            let depth = body["depth"].as_u64().ok_or("depth precisa ser um número")? as usize;
            let chosen: std::collections::BTreeMap<String, String> = serde_json::from_value(body["chosen"].clone()).unwrap_or_default();
            let syntax = librarian_ingest::Catalog::load(&store.edition(&key, &run)?)?.call_syntax();
            let graph = graph(app, &key, &run)?;
            ok(serde_json::to_value(graph.calls_from(&field(&body, "node")?, depth, USED_BY_NODES, &syntax, &chosen)?)?)
        }
        ("POST", "/api/path-questions") => {
            // P1b: one question per point picked on a "what does it call?" path.
            let body = req.json()?;
            store::refuse_notes(&body)?;
            let nodes: Vec<String> = serde_json::from_value(body["nodes"].clone())?;
            ok(store.ask_path(&field(&body, "key")?, &field(&body, "run")?, &field(&body, "question")?, &nodes,
                &field(&body, "seed")?, &body["chosen"])?)
        }
        ("POST", "/api/gathered-question") => {
            // P4: one question over the declarations the person gathered one by one.
            let body = req.json()?;
            store::refuse_notes(&body)?;
            let nodes: Vec<String> = serde_json::from_value(body["nodes"].clone())?;
            let with_facts = body["with_facts"].as_bool().unwrap_or(false);
            let origin = body.get("origin").filter(|o| !o.is_null()).cloned();
            ok(store.ask_gathered(&field(&body, "key")?, &field(&body, "run")?, &field(&body, "question")?, &nodes, with_facts, origin)?)
        }
        ("POST", "/api/envelope/suggest") => {
            // P5: what a change to the chosen declarations is likely to need in view, with why.
            let body = req.json()?;
            let nodes: Vec<String> = serde_json::from_value(body["nodes"].clone())?;
            ok(store.suggest_envelope(&field(&body, "key")?, &field(&body, "run")?, &nodes)?)
        }
        ("GET", "/api/inventory") => ok(store.inventory(param("key")?, param("run")?)?),
        ("POST", "/api/inventory/investigate") => {
            // From a finding of the inventory: the evidence the librarian proposes, each with why.
            let body = req.json()?;
            ok(store.investigate(&field(&body, "key")?, &field(&body, "run")?, &field(&body, "target")?)?)
        }
        ("GET", "/api/graph") => Ok(Response {
            status: 200,
            content_type: "application/json; charset=utf-8",
            body: store.graph(param("key")?, param("run")?)?,
        }),
        ("GET", "/api/turns") => ok(store.turns(param("key")?, param("run")?)?),
        // A question's package as recorded, H1 to H3 as checked now, H4 as made again now: reading only.
        ("GET", "/api/evidence") => ok(store.evidence(param("key")?, param("run")?, param("turn")?)?),
        ("POST", "/api/ask") => {
            let body = req.json()?;
            let question = field(&body, "question")?;
            ok(store.ask(&field(&body, "key")?, &field(&body, "run")?, &question)?)
        }
        ("POST", "/api/send") => send(store, &req.json()?),
        ("POST", "/api/ladder") => ok(ladder(store, &req.json()?)?),
        ("POST", "/api/paste") => {
            let body = req.json()?;
            let (key, run, turn) = (field(&body, "key")?, field(&body, "run")?, field(&body, "turn")?);
            ok(store.paste_answer(&key, &run, &turn, &field(&body, "text")?)?)
        }
        ("POST", "/api/continue") => {
            let body = req.json()?;
            let (key, run, from) = (field(&body, "key")?, field(&body, "run")?, field(&body, "turn")?);
            ok(store.continue_turn(&key, &run, &from, &field(&body, "question")?)?)
        }
        ("POST", "/api/review") => {
            let body = req.json()?;
            let approved = match body["decision"].as_str() {
                Some("approved") => true,
                Some("rejected") => false,
                _ => return Err("decision deve ser approved ou rejected".into()),
            };
            let (key, run, turn) = (field(&body, "key")?, field(&body, "run")?, field(&body, "turn")?);
            let note = body["note"].as_str().unwrap_or_default();
            ok(store.review(&key, &run, &turn, &field(&body, "send")?, &field(&body, "claim")?, approved, note)?)
        }
        ("GET", "/api/facts") => ok(store.facts(param("key")?, param("run")?)?),
        // P2: the profile's commands; approval is for one exact command, running needs it.
        ("GET", "/api/commands") => {
            let mut commands = store.commands(param("key")?)?;
            commands["measure_all"] = store.measure_all(param("key")?, req.param("run"))?;
            // Null when the records cannot be read: not the same as none, and the rest still answers.
            commands["manual_counts"] = store.manual_counts(param("key")?, req.param("run")).unwrap_or(Value::Null);
            ok(commands)
        }
        ("POST", "/api/commands/approve") => {
            let body = req.json()?;
            ok(store.approve_command(&field(&body, "key")?, &field(&body, "name")?, &field(&body, "sha256")?)?)
        }
        ("POST", "/api/commands/run") => {
            let body = req.json()?;
            // With `change` ({turn, send, claim}), the run is in a copy with that answer's diff applied.
            let change: Option<execution::ChangeRef> = match body.get("change") {
                Some(c) if !c.is_null() => Some(serde_json::from_value(c.clone())?),
                _ => None,
            };
            ok(store.execute(&field(&body, "key")?, &field(&body, "run")?, &field(&body, "name")?, change.as_ref())?)
        }
        ("GET", "/api/executions") => ok(json!(store.executions(param("key")?, param("run")?)?)),
        ("POST", "/api/instrument/run") => {
            let body = req.json()?;
            ok(store.execute_instrumented(&field(&body, "key")?, &field(&body, "run")?)?)
        }
        ("GET", "/api/hotspots") => {
            // No measure yet is an answer, not a failure: the page tells the two apart.
            if store.latest_measure(param("key")?, param("run")?)?.is_none() {
                return Ok(Response::error(404, "Ainda não há medição de tudo nesta edition"));
            }
            ok(store.hotspots(param("key")?, param("run")?)?)
        }
        ("GET", "/api/measure-detail") => ok(store.measure_detail(param("key")?, param("run")?)?),
        ("POST", "/api/performance-question") => {
            let body = req.json()?;
            ok(store.performance_question(&field(&body, "key")?, &field(&body, "run")?, &field(&body, "question")?)?)
        }
        ("DELETE", "/api/turn") => {
            let body = req.json()?;
            ok(store.delete_turn(&field(&body, "key")?, &field(&body, "run")?, &field(&body, "turn")?)?)
        }
        ("POST", "/api/lexicon/suggest") => {
            let body = req.json()?;
            // The words are proposed in the project's language; the profile answers, or "manual" only writes the request.
            let choice = choice(store, &body)?;
            if choice.profile != "manual" {
                llm::provider_of(&store.library()?, &choice.profile)?;
            }
            ok(store.suggest_lexicon(&field(&body, "key")?, &field(&body, "run")?, choice)?)
        }
        ("POST", "/api/lexicon/check") => {
            let body = req.json()?;
            let (key, run, id) = (field(&body, "key")?, field(&body, "run")?, field(&body, "id")?);
            ok(store.check_lexicon(&key, &run, &id, &field(&body, "text")?)?)
        }
        ("POST", "/api/project/export") => {
            let body = req.json()?;
            ok(store.export_profile(&field(&body, "key")?, body["dry_run"] != false)?)
        }
        // The library, read only: each task × profile × language as the send renders it.
        ("GET", "/api/modelfiles") => ok(store.library()?.summary()?),
        _ => Ok(Response::error(404, format!("Rota desconhecida: {} {}", req.method, req.path))),
    }
}

/// Node cap of the "Onde é usado?" view; past it the answer says it was truncated.
const USED_BY_NODES: usize = 120;

/// The run's graph-v2, parsed once and kept while the same run is asked about.
fn graph(app: &App, key: &str, run: &str) -> Result<Arc<v2::Graph>> {
    let id = (key.to_owned(), run.to_owned());
    let mut cached = app.graph.lock().map_err(|_| "cache do grafo indisponível")?;
    if let Some((cached_id, graph)) = cached.as_ref() {
        if *cached_id == id {
            return Ok(Arc::clone(graph));
        }
    }
    let graph: Arc<v2::Graph> = Arc::new(serde_json::from_slice(&app.store.graph(key, run)?)?);
    *cached = Some((id, Arc::clone(&graph)));
    Ok(graph)
}

/// The inverse of the rule over a run's calls, built on the first "Onde é usado?" of the run.
fn uses(app: &App, key: &str, run: &str, graph: &v2::Graph) -> Result<Arc<UseIndex>> {
    let id = (key.to_owned(), run.to_owned());
    let mut cached = app.uses.lock().map_err(|_| "cache dos usos indisponível")?;
    if let Some((cached_id, uses)) = cached.as_ref() {
        if *cached_id == id {
            return Ok(Arc::clone(uses));
        }
    }
    let syntax = librarian_ingest::Catalog::load(&app.store.edition(key, run)?)?.call_syntax();
    let uses = Arc::new(UseIndex::build(graph, &syntax));
    *cached = Some((id, Arc::clone(&uses)));
    Ok(uses)
}

/// Re-verifies the question immediately before every send; the send records the link (L3).
fn send(store: &Store, body: &Value) -> Result<Response> {
    let profile = body["profile"].as_str().unwrap_or("");
    // Tests reach the route with the profile that pastes an answer (see `send_value`).
    if !profile.is_empty() && llm::provider_of(&store.library()?, profile).is_err() {
        return Ok(Response::error(501, format!("Perfil sem envio: {profile}")));
    }
    Ok(Response::json(200, &send_value(store, body)?))
}

fn send_value(store: &Store, body: &Value) -> Result<Value> {
    let get = |name: &str| body[name].as_str();
    let (Some(key), Some(run), Some(turn), Some(profile)) =
        (get("key"), get("run"), get("turn"), get("profile"))
    else {
        return Err("Informe key, run, turn e profile".into());
    };
    let library = store.library()?;
    let provider = llm::provider_of(&library, profile).map_err(|_| format!("Perfil sem envio: {profile}"))?;
    let verified = store.verify_for_send(key, run, turn)?;
    // Tests with `answers` stand an answer pasted as a person would for a provider's, by the question
    // it answers, after the check every send has: the same files after it. Without them, the send
    // goes on to `llm`'s test provider. No model is called either way.
    #[cfg(test)]
    if profile == "test" && body.get("answers").is_some() {
        return store.paste_answer(key, run, turn, body["answers"][turn].as_str().unwrap_or_default());
    }
    let choice = choice(store, body)?;
    // The task follows what the question asks (its claims policy); the language, the project.
    let task = models::task_for_question(verified.context().map(|c| (c.allow_patch, c.allow_edits)));
    let language = store::language_of(&store.settings(key)?)?;
    let destination = store::next_send(&store.turn_dir(key, run, turn)?, &provider)?;
    let mut summary = llm::send_question(&library, &choice, task, language, &verified, &destination)?;
    if let (Some(context), Some(text)) = (verified.context(), summary["text"].as_str()) {
        summary["check"] = claims::check(context, &destination, text)?;
        std::fs::write(destination.join("send.json"), serde_json::to_vec_pretty(&summary)?)?;
    }
    Ok(summary)
}

/// What every refusal of the ladder ends with: it is checked before anything runs, is created or is sent.
const LADDER_NOTHING: &str = "nada foi criado, executado nem enviado";

/// The ladder (see ladder.rs) over one question already prepared and reviewed in the Pacote view
/// (`turn`): that package goes once to one model, and the gate's commands judge the changes it
/// proposes. Baselines missing for a gate command are run first. Nothing more is asked: a refusal or
/// a LACUNA stays in the record, and a new question is a new package the person reviews (18/11,
/// option A). The request is refused, before anything runs, when it asks for anything else.
fn ladder(store: &Store, body: &Value) -> Result<Value> {
    // Before anything runs or is created: no `notes`, anywhere in what the client sent.
    store::refuse_notes(body)?;
    let field = |name: &str| -> Result<String> { body[name].as_str().map(str::to_owned).ok_or_else(|| format!("Informe {name}").into()) };
    let (key, run) = (field("key")?, field("run")?);
    // No question is made here: without the reviewed one, there is nothing to send.
    let Some(turn) = body["turn"].as_str().map(str::to_owned) else {
        return Err(format!("A escada só envia uma pergunta já preparada e revisada na vista Pacote: informe turn; {LADDER_NOTHING}").into());
    };
    for name in ["question", "nodes", "with_facts", "origin"] {
        if body.get(name).is_some() {
            return Err(format!("A escada não recebe {name}: a pergunta, as declarações e a origem são as da pergunta revisada (turn); {LADDER_NOTHING}").into());
        }
    }
    // No automatic rounds: absent or 0, nothing else (no default that climbs on its own).
    for name in ["retries", "lacuna_rounds"] {
        if body.get(name).is_some_and(|v| v.as_u64() != Some(0)) {
            return Err(format!("A escada não faz rodadas automáticas ({name} deve ser 0 ou ausente): cada nova pergunta passa pela vista Pacote; {LADDER_NOTHING}").into());
        }
    }
    // One model: the one the request names, or the ladder's default in the library. A second one would
    // be another call made after an answer, without the person asking for it.
    let models: Vec<Value> = match body.get("models").filter(|m| !m.is_null()) {
        Some(models) => serde_json::from_value(models.clone())?,
        None => {
            let library = store.library()?;
            let default = library.action("ladder").ok_or("A biblioteca não define a escada")?;
            vec![json!({ "profile": default.profile, "claude_model": default.claude_model })]
        }
    };
    let [model] = models.as_slice() else {
        return Err(format!("A escada envia a um único modelo por pedido (models com {} itens); {LADDER_NOTHING}", models.len()).into());
    };
    let gate: Vec<String> = serde_json::from_value(body["gate"].clone())?;
    if gate.is_empty() {
        return Err(format!("A escada precisa de ao menos um comando no portão; {LADDER_NOTHING}").into());
    }
    // The reviewed package: it exists, it is a gathered question, and it passes the check every send
    // has (H1 to H3), before any baseline runs. The send checks it again right before it goes.
    let recorded: Value = match std::fs::read(store.turn_dir(&key, &run, &turn)?.join("question.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(_) => return Err(format!("Pergunta {turn} sem pacote preparado; {LADDER_NOTHING}").into()),
    };
    if recorded.get("gathered").is_none() {
        return Err(format!("A escada só sobe sobre uma pergunta reunida (com declarações juntadas); {LADDER_NOTHING}").into());
    }
    store.verify_for_send(&key, &run, &turn)?;
    let nodes: Vec<String> = serde_json::from_value(recorded["gathered"]["nodes"].clone())?;
    let mut steps = vec![];
    let executions = store.executions(&key, &run)?;
    for name in &gate {
        let has = executions.iter().any(|x| x["name"] == name.as_str() && x["change"].is_null() && x["instrumentation"].is_null()
            && x["summary"]["succeeded"].as_u64() > Some(0));
        if !has {
            let x = store.execute(&key, &run, name, None)?;
            steps.push(json!({ "outcome": "baseline", "execution": x["id"], "name": name }));
        }
    }
    // The one send: the reviewed package, to the one model. What comes back is judged and recorded;
    // no other call follows it.
    let mut request = model.clone();
    for (k, v) in [("key", &key), ("run", &run), ("turn", &turn)] {
        request[k] = json!(v);
    }
    let summary = match send_value(store, &request) {
        Ok(s) => s,
        Err(e) => {
            steps.push(ladder::step(model, &turn, "", "provider_failed", json!(e.to_string())));
            return Ok(json!({ "passed": false, "turn": turn, "nodes": nodes, "steps": steps }));
        }
    };
    let send = summary["directory"].as_str().and_then(|d| std::path::Path::new(d).file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let (usable, lacunas, codes) = ladder::outcome(&summary);
    if !usable.is_empty() {
        // Each proposed change through the gate, in the answer's order; of those that pass, the one
        // that gains the most is the answer's: the first to pass is often a small one, and a larger
        // one after it would never be seen. The cost of the send is counted once, on its first change.
        let mut best: Option<(usize, String, f64)> = None;
        for (k, claim) in usable.iter().enumerate() {
            let change = execution::ChangeRef { turn: turn.clone(), send: send.clone(), claim: claim.clone() };
            let runs: Vec<Value> = gate.iter().map(|name| store.execute(&key, &run, name, Some(&change))
                .unwrap_or_else(|e| json!({ "name": name, "refused": e.to_string() }))).collect();
            let (passed, why) = ladder::verdict(&runs);
            let cost = if k == 0 { summary["cost_usd"].clone() } else { Value::Null };
            let gain = ladder::gain(&runs);
            steps.push(ladder::step(model, &turn, &send, if passed { "passed" } else { "refused_by_gate" },
                json!({ "claim": claim, "cost_usd": cost, "executions": runs.iter().map(|x| &x["id"]).collect::<Vec<_>>(), "why": why, "worse": ladder::worse(&runs), "gain_pct": gain })));
            if passed && best.as_ref().is_none_or(|(_, _, g)| gain > *g) {
                best = Some((steps.len() - 1, claim.clone(), gain));
            }
        }
        if let Some((chosen, claim, _)) = best {
            for (i, step) in steps.iter_mut().enumerate() {
                if i != chosen && step["outcome"] == "passed" && step["send"] == send.as_str() {
                    step["outcome"] = json!("passed_smaller");
                }
            }
            return Ok(json!({ "passed": true, "turn": turn, "send": send, "claim": claim, "nodes": nodes, "steps": steps }));
        }
    } else if !lacunas.is_empty() {
        // A LACUNA is recorded with what it names and what the catalog would find for it (`suggested`),
        // for the person to gather in a new package; nothing is added and nothing is asked.
        let catalog = librarian_ingest::Catalog::load(&store.edition(&key, &run)?)?;
        let names: Vec<String> = lacunas.iter().flat_map(|t| ladder::lacuna_names(t)).collect();
        let ran: Vec<String> = store.hotspots(&key, &run).ok().map(|h| h["rows"].as_array().into_iter().flatten()
            .filter(|r| r["calls"].as_f64().unwrap_or(0.) > 0.).filter_map(|r| r["name"].as_str().map(String::from)).collect()).unwrap_or_default();
        let room = ladder::lacuna_room(nodes.len());
        let found = ladder::resolve(&catalog, &names, &nodes, &ran, room);
        steps.push(ladder::step(model, &turn, &send, "lacuna", json!({ "cost_usd": summary["cost_usd"], "issues": codes, "text": lacunas, "names": names, "no_room": room == 0,
            "suggested": found.iter().map(|(id, n)| json!({ "symbol": id, "name": catalog.symbols().get(id).map(|s| &s.qualified_name), "for": n })).collect::<Vec<_>>() })));
    } else {
        steps.push(ladder::step(model, &turn, &send, "refused", json!({ "cost_usd": summary["cost_usd"], "issues": codes, "text": lacunas })));
    }
    Ok(json!({ "passed": false, "turn": turn, "nodes": nodes, "steps": steps }))
}

/// What a gate refusal added to the client's question in a ladder retry. No route retries since 18/11;
/// the text stays for the tests of the questions recorded with it, whose packages are still read.
#[cfg(test)]
pub(crate) fn gate_suffix(refused: &[String], attempt: usize) -> store::Suffix {
    store::Suffix { text: format!(" Propostas anteriores recusadas pelo portão: {}. Proponha outras mudanças.", refused.join(" | ")), rule: RetryRule::GateRefused, attempt }
}

/// What a refusal of the mechanical check added to the client's question in a ladder retry (see `gate_suffix`).
#[cfg(test)]
pub(crate) fn mechanical_suffix(codes: &[String], attempt: usize) -> store::Suffix {
    store::Suffix { text: format!(" Uma resposta anterior foi recusada na conferência mecânica: {}.", codes.join(", ")), rule: RetryRule::MechanicalRefused, attempt }
}

/// Who answers, as the page or the ladder chose it: a profile of the library (or "manual"), the
/// Claude model and the skills. A request that still names a Modelfile is from before the library.
fn choice(store: &Store, body: &Value) -> Result<llm::Choice> {
    if body.get("modelfile").is_some_and(|m| !m.is_null()) {
        return Err("O Modelfile não é mais escolhido: escolha um perfil (profile); a tarefa vem da pergunta".into());
    }
    Ok(llm::Choice {
        profile: body["profile"].as_str().unwrap_or("manual").to_owned(),
        claude_model: body["claude_model"].as_str().map(str::to_owned),
        skills: body["skills"].as_array().into_iter().flatten().filter_map(Value::as_str).map(|n| store.skill(n)).collect::<Result<_>>()?,
    })
}
