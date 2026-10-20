//! Librarian Studio: local server behind the Graph Viewer. Select a project, generate its
//! verified graph, ask questions that light the graph, and send prepared queries to an LLM.
//! Listens on 127.0.0.1 only; requests from other origins are refused.
mod http;
mod llm;
mod store;

use http::{Request, Response};
use serde_json::{json, Value};
use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};
use store::{Settings, Store};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct App {
    port: u16,
    viewer: PathBuf,
    store: Store,
}

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
        viewer: root.join("tools/graph-viewer/index.html"),
        store: Store { dir: root.join(".studio"), modelfiles: root.join("modelfiles") },
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
        ("GET", "/api/modelfiles") => ok(store.modelfiles()?),
        ("PUT", "/api/modelfiles") => {
            let body = req.json()?;
            ok(store.save_modelfile(&field(&body, "name")?, &field(&body, "content")?)?)
        }
        _ => Ok(Response::error(404, format!("Rota desconhecida: {} {}", req.method, req.path))),
    }
}

/// Re-verifies the prepared bundle against the edition immediately before sending, as PLAYME does.
fn send(store: &Store, body: &Value) -> Result<Response> {
    let get = |name: &str| body[name].as_str();
    let (Some(key), Some(run), Some(turn), Some(provider)) =
        (get("key"), get("run"), get("turn"), get("provider"))
    else {
        return Err("Informe key, run, turn e provider".into());
    };
    let implemented = llm::providers()
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["id"] == provider && p["implemented"] == true && provider != "manual");
    if !implemented {
        return Ok(Response::error(501, format!("Provedor sem envio implementado: {provider}")));
    }
    let dir = store.turn_dir(key, run, turn)?;
    let prepared = dir.join("prepared");
    let catalog = librarian_ingest::Catalog::load(&store.edition(key, run)?)?;
    librarian_ingest::prepare::verify(&catalog, &prepared)?;
    let modelfile = get("modelfile").map(|name| store.modelfile_path(name)).transpose()?;
    let sends = dir.join("sends");
    std::fs::create_dir_all(&sends)?;
    let next = std::fs::read_dir(&sends)?.count() + 1;
    let summary = llm::send(
        provider,
        llm::Send {
            query: &prepared.join("bundle/query.json"),
            dir: &sends.join(format!("{next:02}-{provider}")),
            modelfile: modelfile.as_deref(),
            modelfile_name: get("modelfile"),
            claude_model: get("claude_model"),
        },
    )?;
    Ok(Response::json(200, &summary))
}
