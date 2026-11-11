//! Librarian Studio: local server for the page in web/index.html. Select a project, generate its
//! verified graph, ask questions that light the graph, and send prepared queries to an LLM.
//! Listens on 127.0.0.1 only; requests from other origins are refused.
mod builder;
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
mod values;

use http::{Request, Response};
use librarian_ingest::{
    evidence::{AdditionReason, AdditionRule, RetryRule, ServerAddition, ServerLacunaDeclared},
    graph::v2,
};
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
}

/// (project key, run id) and that run's parsed graph-v2.
type CachedGraph = ((String, String), Arc<v2::Graph>);

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
            "llm": llm::status(),
            "modelfiles": store.modelfiles()?,
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
            let graph = graph(app, param("key")?, param("run")?)?;
            ok(serde_json::to_value(graph.used_by(param("node")?, depth, USED_BY_NODES)?)?)
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
        ("GET", "/api/value-sources") => ok(store.value_sources(param("key")?, param("run")?, param("symbol")?, param("text")?)?),
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
            let provider = field(&body, "provider")?;
            if provider != "manual" && !implemented(&provider) {
                return Ok(Response::error(501, format!("Provedor sem envio implementado: {provider}")));
            }
            let choice = choice(store, &body)?;
            // The language of the proposed words; the page takes it from the Modelfile name (-en).
            let language = body["language"].as_str().unwrap_or("pt");
            ok(store.suggest_lexicon(&field(&body, "key")?, &field(&body, "run")?, &provider, language, choice)?)
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
        ("GET", "/api/modelfiles") => ok(store.modelfiles()?),
        ("PUT", "/api/modelfiles") => {
            let body = req.json()?;
            ok(store.save_modelfile(&field(&body, "name")?, &field(&body, "content")?)?)
        }
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

/// Re-verifies the question immediately before every send; the send records the link (L3).
fn send(store: &Store, body: &Value) -> Result<Response> {
    let provider = body["provider"].as_str().unwrap_or("");
    // Tests reach the route with the provider that pastes an answer (see `send_value`).
    if !provider.is_empty() && !implemented(provider) && !(cfg!(test) && provider == "test") {
        return Ok(Response::error(501, format!("Provedor sem envio implementado: {provider}")));
    }
    Ok(Response::json(200, &send_value(store, body)?))
}

fn send_value(store: &Store, body: &Value) -> Result<Value> {
    let get = |name: &str| body[name].as_str();
    let (Some(key), Some(run), Some(turn), Some(provider)) =
        (get("key"), get("run"), get("turn"), get("provider"))
    else {
        return Err("Informe key, run, turn e provider".into());
    };
    let testing = cfg!(test) && provider == "test";
    if !implemented(provider) && !testing {
        return Err(format!("Provedor sem envio implementado: {provider}").into());
    }
    let verified = store.verify_for_send(key, run, turn)?;
    // Tests with `answers` stand an answer pasted as a person would for a provider's, by the question
    // it answers, after the check every send has: the same files after it. Without them, the send
    // goes on to `llm`'s test provider. No model is called either way.
    #[cfg(test)]
    if testing && body.get("answers").is_some() {
        return store.paste_answer(key, run, turn, body["answers"][turn].as_str().unwrap_or_default());
    }
    let choice = choice(store, body)?;
    let destination = store::next_send(&store.turn_dir(key, run, turn)?, provider)?;
    let mut summary = llm::send_question(provider, &choice, &verified, &destination)?;
    if let (Some(context), Some(text)) = (verified.context(), summary["text"].as_str()) {
        summary["check"] = claims::check(context, &destination, text)?;
        std::fs::write(destination.join("send.json"), serde_json::to_vec_pretty(&summary)?)?;
    }
    Ok(summary)
}

/// The ladder (see ladder.rs): `models` in order, the gate's commands, and how many times a
/// LACUNA may gather what it asks for. Baselines missing for a gate command are run first.
fn ladder(store: &Store, body: &Value) -> Result<Value> {
    // Before anything runs or is created: no `notes`, anywhere in what the client sent.
    store::refuse_notes(body)?;
    let field = |name: &str| -> Result<String> { body[name].as_str().map(str::to_owned).ok_or_else(|| format!("Informe {name}").into()) };
    let (key, run, question) = (field("key")?, field("run")?, field("question")?);
    let mut nodes: Vec<String> = serde_json::from_value(body["nodes"].clone())?;
    let models: Vec<Value> = serde_json::from_value(body["models"].clone())?;
    let gate: Vec<String> = serde_json::from_value(body["gate"].clone())?;
    let rounds = body["lacuna_rounds"].as_u64().unwrap_or(1);
    // A refused answer gets this many new tries from the same model, with why it was refused.
    let retries = body["retries"].as_u64().unwrap_or(1);
    let with_facts = body["with_facts"].as_bool().unwrap_or(false);
    let origin = body.get("origin").filter(|o| !o.is_null()).cloned();
    if models.is_empty() || gate.is_empty() {
        return Err("A escada precisa de ao menos um modelo e um comando no portão".into());
    }
    // The client's selection is checked once, over the nodes it sent, before any baseline runs or
    // any round starts. The rounds go with this record; the nodes a LACUNA adds are the server's.
    let selection = store.accept_selection(&key, &run, &nodes, origin.as_ref())?;
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
    let catalog = librarian_ingest::Catalog::load(&store.edition(&key, &run)?)?;
    // Every question of the ladder goes with the record accepted at its entry, the rounds too, and
    // with what the server added so far, in the order added.
    let ask = |suffix: &[store::Suffix], nodes: &[String], additions: &[ServerAddition]| -> Result<String> {
        Ok(store.ask_gathered_parts(&key, &run, &question, suffix, nodes, with_facts, origin.clone(), selection.as_ref(), additions)?["id"].as_str().ok_or("pergunta sem id")?.to_owned())
    };
    let mut additions: Vec<ServerAddition> = vec![];
    let mut turn = ask(&[], &nodes, &additions)?;
    for model in &models {
        let (mut asked, mut tried) = (0, 0);
        // What a retry added to the client's question, kept apart from it.
        let mut suffix: Option<store::Suffix> = None;
        loop {
            let mut request = model.clone();
            for (k, v) in [("key", &key), ("run", &run), ("turn", &turn)] {
                request[k] = json!(v);
            }
            let summary = match send_value(store, &request) {
                Ok(s) => s,
                Err(e) => {
                    steps.push(ladder::step(model, &turn, "", "provider_failed", json!(e.to_string())));
                    break;
                }
            };
            let send = summary["directory"].as_str().and_then(|d| std::path::Path::new(d).file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let (usable, lacunas, codes) = ladder::outcome(&summary);
            if !usable.is_empty() {
                // Each proposed change through the gate, in the answer's order; of those that pass,
                // the one that gains the most is the answer's: the first to pass is often a small
                // one, and a larger one after it would never be seen. The cost of the send is
                // counted once, on its first change.
                let mut refused = vec![];
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
                    if passed {
                        if best.as_ref().is_none_or(|(_, _, g)| gain > *g) {
                            best = Some((steps.len() - 1, claim.clone(), gain));
                        }
                        continue;
                    }
                    let proposed = summary["check"]["claims"].as_array().into_iter().flatten().find(|c| c["id"] == claim.as_str())
                        .and_then(|c| c["text"].as_str()).unwrap_or("").trim_end_matches('.').to_owned();
                    refused.push(format!("{proposed}: {}", why.join("; ")));
                }
                if let Some((chosen, claim, _)) = best {
                    for (i, step) in steps.iter_mut().enumerate() {
                        if i != chosen && step["outcome"] == "passed" && step["send"] == send.as_str() {
                            step["outcome"] = json!("passed_smaller");
                        }
                    }
                    return Ok(json!({ "passed": true, "turn": turn, "send": send, "claim": claim, "nodes": nodes, "steps": steps }));
                }
                if tried < retries {
                    tried += 1;
                    // What was proposed, so the next try does not repeat it.
                    suffix = Some(gate_suffix(&refused, tried as usize));
                    turn = ask(suffix.as_slice(), &nodes, &additions)?;
                    continue;
                }
                break;
            }
            if !lacunas.is_empty() && asked < rounds {
                let names: Vec<String> = lacunas.iter().flat_map(|t| ladder::lacuna_names(t)).collect();
                let ran: Vec<String> = store.hotspots(&key, &run).ok().map(|h| h["rows"].as_array().into_iter().flatten()
                    .filter(|r| r["calls"].as_f64().unwrap_or(0.) > 0.).filter_map(|r| r["name"].as_str().map(String::from)).collect()).unwrap_or_default();
                let room = ladder::lacuna_room(nodes.len());
                let added = ladder::resolve(&catalog, &names, &nodes, &ran, room);
                // Nothing leaves a full question to make room: the record only says so.
                steps.push(ladder::step(model, &turn, &send, "lacuna", json!({ "cost_usd": summary["cost_usd"], "text": lacunas, "names": names, "no_room": room == 0,
                    "gathered": added.iter().map(|(id, n)| json!({ "symbol": id, "name": catalog.symbols().get(id).map(|s| &s.qualified_name), "for": n })).collect::<Vec<_>>() })));
                if added.is_empty() {
                    break;
                }
                // Each one the server's, with the round, the send whose LACUNA asked for it and the
                // name that found it: the questions from now on carry them apart from the client's.
                asked += 1;
                additions.extend(added.iter().map(|(id, name)| ServerAddition {
                    declared_by: ServerLacunaDeclared::ServerLacuna, round: asked as usize, from_turn: turn.clone(), from_send: send.clone(),
                    symbol_id: id.clone(), for_name: name.clone(), reason: AdditionReason::LacunaName, decided_by_rule: AdditionRule::LadderResolve,
                }));
                nodes.extend(added.into_iter().map(|(id, _)| id));
                turn = ask(suffix.as_slice(), &nodes, &additions)?;
                continue;
            }
            steps.push(ladder::step(model, &turn, &send, if lacunas.is_empty() { "refused" } else { "lacuna" },
                json!({ "cost_usd": summary["cost_usd"], "issues": codes, "text": lacunas })));
            // A cloud model whose answer the check refused tries again, told which rules failed;
            // the local one is not retried (it is the free first step, not the one that must pass).
            if lacunas.is_empty() && tried < retries && model["provider"] != "ollama" {
                tried += 1;
                suffix = Some(mechanical_suffix(&codes, tried as usize));
                turn = ask(suffix.as_slice(), &nodes, &additions)?;
                continue;
            }
            break;
        }
    }
    Ok(json!({ "passed": false, "turn": turn, "nodes": nodes, "steps": steps }))
}

/// What a gate refusal adds to the client's question: what was proposed, so the next try does not
/// repeat it. Its text is the one the question always got.
pub(crate) fn gate_suffix(refused: &[String], attempt: usize) -> store::Suffix {
    store::Suffix { text: format!(" Propostas anteriores recusadas pelo portão: {}. Proponha outras mudanças.", refused.join(" | ")), rule: RetryRule::GateRefused, attempt }
}

/// What a refusal of the mechanical check adds to the client's question: which rules failed.
pub(crate) fn mechanical_suffix(codes: &[String], attempt: usize) -> store::Suffix {
    store::Suffix { text: format!(" Uma resposta anterior foi recusada na conferência mecânica: {}.", codes.join(", ")), rule: RetryRule::MechanicalRefused, attempt }
}

/// Providers that actually send; "manual" and the listed-but-pending ones do not.
fn implemented(provider: &str) -> bool {
    let providers = llm::providers();
    let found = providers.as_array().unwrap().iter().any(|p| p["id"] == provider && p["implemented"] == true);
    found && provider != "manual"
}

fn choice(store: &Store, body: &Value) -> Result<llm::Choice> {
    let name = body["modelfile"].as_str().filter(|n| !n.is_empty());
    Ok(llm::Choice {
        modelfile: name.map(|n| store.modelfile_path(n)).transpose()?,
        modelfile_name: name.map(str::to_owned),
        claude_model: body["claude_model"].as_str().map(str::to_owned),
        skills: body["skills"].as_array().into_iter().flatten().filter_map(Value::as_str).map(|n| store.skill(n)).collect::<Result<_>>()?,
    })
}
