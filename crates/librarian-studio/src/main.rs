//! Librarian Studio: local server for the page in web/index.html. Select a project, generate its
//! verified graph, ask questions that light the graph, and send prepared queries to an LLM.
//! Listens on 127.0.0.1 only; requests from other origins are refused.
mod claims;
mod facts;
mod http;
mod lexicon;
mod llm;
mod store;

use http::{Request, Response};
use librarian_ingest::graph::v2;
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
        ("GET", "/api/graph") => Ok(Response {
            status: 200,
            content_type: "application/json; charset=utf-8",
            body: store.graph(param("key")?, param("run")?)?,
        }),
        ("GET", "/api/turns") => ok(store.turns(param("key")?, param("run")?)?),
        ("POST", "/api/ask") => {
            let body = req.json()?;
            let question = field(&body, "question")?;
            ok(store.ask(&field(&body, "key")?, &field(&body, "run")?, &question)?)
        }
        ("POST", "/api/send") => send(store, &req.json()?),
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
            ok(store.suggest_lexicon(&field(&body, "key")?, &field(&body, "run")?, &provider, choice)?)
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
    let get = |name: &str| body[name].as_str();
    let (Some(key), Some(run), Some(turn), Some(provider)) =
        (get("key"), get("run"), get("turn"), get("provider"))
    else {
        return Err("Informe key, run, turn e provider".into());
    };
    if !implemented(provider) {
        return Ok(Response::error(501, format!("Provedor sem envio implementado: {provider}")));
    }
    let verified = store.verify_for_send(key, run, turn)?;
    let choice = choice(store, body)?;
    let destination = store::next_send(&store.turn_dir(key, run, turn)?, provider)?;
    let mut request = choice.request(&verified.query, &destination);
    request.criteria_sha256 = verified.criteria_sha256.as_deref();
    request.link = Some(&verified.link);
    let mut summary = llm::send(provider, request)?;
    if let (Some(context), Some(text)) = (&verified.context, summary["text"].as_str()) {
        summary["check"] = claims::check(context, &destination, text)?;
        std::fs::write(destination.join("send.json"), serde_json::to_vec_pretty(&summary)?)?;
    }
    Ok(Response::json(200, &summary))
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
    })
}
