//! Projects, runs and questions under `.studio/`. Nothing is written inside analyzed projects.
//! Layout: projects/<key>/project.json, runs/<run>/{edition, graph-v2.json, run.json},
//! runs/<run>/turns/<NN>/{question.json, search.json, prepare-request.json, prepared/, sends/}.
use librarian_ingest::{
    graph::v2,
    hash,
    prepare::{self, Request},
    search::{Lexicon, SearchIndex},
    Catalog, Config, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Everything a question needs besides the question itself; edited from the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub name: String,
    pub root: String,
    pub inputs: Vec<String>,
    pub lexicon: Lexicon,
    /// None uses the Graph Engine default instructions.
    pub instructions: Option<Vec<String>>,
    pub top: usize,
    pub depth: usize,
    pub hits: usize,
    pub radius: usize,
    pub numbered_lines: bool,
    pub accept_diagnostics: bool,
    /// Profile the first settings were read from, if any; later edits stay in `.studio`.
    #[serde(default)]
    pub imported_from: Option<String>,
}

pub struct Store {
    pub dir: PathBuf,
    pub modelfiles: PathBuf,
}

impl Store {
    fn project_dir(&self, key: &str) -> Result<PathBuf> {
        let dir = self.dir.join("projects").join(safe_id(key)?);
        if !dir.join("project.json").is_file() {
            return Err(format!("Projeto desconhecido: {key}").into());
        }
        Ok(dir)
    }
    fn run_dir(&self, key: &str, run: &str) -> Result<PathBuf> {
        let dir = self.project_dir(key)?.join("runs").join(safe_id(run)?);
        if !dir.join("run.json").is_file() {
            return Err(format!("Execução desconhecida: {run}").into());
        }
        Ok(dir)
    }
    pub fn turn_dir(&self, key: &str, run: &str, turn: &str) -> Result<PathBuf> {
        let dir = self.run_dir(key, run)?.join("turns").join(safe_id(turn)?);
        if !dir.join("question.json").is_file() {
            return Err(format!("Pergunta desconhecida: {turn}").into());
        }
        Ok(dir)
    }
    pub fn edition(&self, key: &str, run: &str) -> Result<PathBuf> {
        Ok(self.run_dir(key, run)?.join("edition"))
    }

    /// Opens (or creates) the project for a folder. A `playme.json` there seeds the settings.
    pub fn open(&self, root: &str) -> Result<Value> {
        let root = display(&Path::new(root.trim()).canonicalize()?);
        let name = Path::new(&root)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "projeto".into());
        let key = format!("{}-{}", slug(&name), &hash(root.to_lowercase().as_bytes())[..8]);
        let dir = self.dir.join("projects").join(&key);
        let file = dir.join("project.json");
        if !file.is_file() {
            fs::create_dir_all(dir.join("runs"))?;
            let settings = initial_settings(&name, &root)?;
            fs::write(&file, serde_json::to_vec_pretty(&settings)?)?;
        }
        self.project(&key)
    }

    pub fn project(&self, key: &str) -> Result<Value> {
        let dir = self.project_dir(key)?;
        let settings: Settings = serde_json::from_slice(&fs::read(dir.join("project.json"))?)?;
        let mut runs = vec![];
        for entry in fs::read_dir(dir.join("runs"))? {
            let path = entry?.path();
            if let Ok(bytes) = fs::read(path.join("run.json")) {
                runs.push(serde_json::from_slice::<Value>(&bytes)?);
            }
        }
        runs.sort_by(|a, b| b["id"].as_str().cmp(&a["id"].as_str()));
        Ok(json!({ "key": key, "settings": settings, "runs": runs }))
    }

    pub fn save_settings(&self, key: &str, settings: &Settings) -> Result<Value> {
        let dir = self.project_dir(key)?;
        let current: Settings = serde_json::from_slice(&fs::read(dir.join("project.json"))?)?;
        if settings.root != current.root {
            return Err("A pasta do projeto não muda; abra outra pasta como outro projeto".into());
        }
        if settings.inputs.is_empty() || settings.hits == 0 || settings.hits > settings.top {
            return Err("Informe entradas e 1 <= hits <= top".into());
        }
        // The lexicon is checked by the search itself, on the next question.
        fs::write(dir.join("project.json"), serde_json::to_vec_pretty(settings)?)?;
        self.project(key)
    }

    /// Generates and verifies a new edition and its syntax-graph-v2, as the CLI does.
    pub fn generate(&self, key: &str) -> Result<Value> {
        let dir = self.project_dir(key)?;
        let settings: Settings = serde_json::from_slice(&fs::read(dir.join("project.json"))?)?;
        let (id, run) = reserve(&dir.join("runs"), &stamp())?;
        let started = now_ms();
        let config = Config { project: settings.name.clone(), inputs: settings.inputs.clone() };
        let edition = run.join("edition");
        let manifest = librarian_ingest::generate(Path::new(&settings.root), &config, &edition)?;
        let catalog = Catalog::load(&edition)?;
        let graph = v2::Graph::build(&catalog);
        let bytes = serde_json::to_vec_pretty(&graph)?;
        fs::write(run.join("graph-v2.json"), &bytes)?;
        serde_json::from_slice::<v2::Graph>(&bytes)?.verify(&catalog)?;
        let record = json!({
            "id": id,
            "root": settings.root,
            "inputs": settings.inputs,
            "git": manifest.git,
            "sources": manifest.source_count,
            "symbols": manifest.symbol_count,
            "diagnostics": manifest.diagnostic_count,
            "nodes": graph.nodes.len(),
            "edges": graph.edges.len(),
            "graph_sha256": hash(&bytes),
            "elapsed_ms": now_ms() - started,
            "verified": "edition recarregada do disco e grafo v2 conferido contra os snapshots",
        });
        fs::write(run.join("run.json"), serde_json::to_vec_pretty(&record)?)?;
        Ok(record)
    }

    pub fn graph(&self, key: &str, run: &str) -> Result<Vec<u8>> {
        Ok(fs::read(self.run_dir(key, run)?.join("graph-v2.json"))?)
    }

    pub fn turns(&self, key: &str, run: &str) -> Result<Value> {
        let dir = self.run_dir(key, run)?.join("turns");
        let mut turns = vec![];
        if dir.is_dir() {
            let mut ids: Vec<String> = fs::read_dir(&dir)?
                .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
                .collect();
            ids.sort();
            for id in ids {
                turns.push(self.turn(key, run, &id)?);
            }
        }
        Ok(json!(turns))
    }

    pub fn turn(&self, key: &str, run: &str, id: &str) -> Result<Value> {
        let dir = self.turn_dir(key, run, id)?;
        let read_json = |path: PathBuf| -> Option<Value> {
            fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok())
        };
        let mut sends = vec![];
        if let Ok(entries) = fs::read_dir(dir.join("sends")) {
            let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
            paths.sort();
            sends.extend(paths.into_iter().filter_map(|p| read_json(p.join("send.json"))));
        }
        let question = read_json(dir.join("question.json")).unwrap_or(Value::Null);
        Ok(json!({
            "id": id,
            "question": question["question"],
            "created": question["created"],
            "search": read_json(dir.join("search.json")),
            "request": read_json(dir.join("prepare-request.json")),
            "query": fs::read_to_string(dir.join("prepared/bundle/query.json")).ok(),
            "prepare_error": question["prepare_error"],
            "sends": sends,
        }))
    }

    /// Search lights the graph even when prepare refuses; the refusal is kept with the question.
    pub fn ask(&self, key: &str, run: &str, question: &str) -> Result<Value> {
        let project = self.project_dir(key)?;
        let settings: Settings = serde_json::from_slice(&fs::read(project.join("project.json"))?)?;
        let run_dir = self.run_dir(key, run)?;
        let catalog = Catalog::load(&run_dir.join("edition"))?;
        let report = SearchIndex::new(&catalog, settings.lexicon.clone())?.search(
            question,
            settings.top,
            settings.depth,
        )?;
        fs::create_dir_all(run_dir.join("turns"))?;
        let turns = run_dir.join("turns");
        let next = fs::read_dir(&turns)?.count() + 1;
        let (id, dir) = reserve(&turns, &format!("{next:02}"))?;
        let mut search = serde_json::to_vec_pretty(&report)?;
        search.push(b'\n');
        fs::write(dir.join("search.json"), &search)?;
        let request = Request {
            question: question.into(),
            lexicon: settings.lexicon.clone(),
            top: settings.top,
            depth: settings.depth,
            hits: settings.hits,
            radius: settings.radius,
            numbered_lines: settings.numbered_lines,
            instructions: settings.instructions.clone(),
            accept_diagnostics: settings.accept_diagnostics,
        };
        let request_bytes = serde_json::to_vec_pretty(&request)?;
        fs::write(dir.join("prepare-request.json"), &request_bytes)?;
        let prepared = dir.join("prepared");
        let prepare_error = prepare::write(&catalog, &request_bytes, &prepared)
            .and_then(|_| prepare::verify(&catalog, &prepared))
            .err()
            .map(|e| e.to_string());
        let record = json!({ "question": question, "created": stamp(), "prepare_error": prepare_error });
        fs::write(dir.join("question.json"), serde_json::to_vec_pretty(&record)?)?;
        self.turn(key, run, &id)
    }

    pub fn modelfiles(&self) -> Result<Value> {
        let mut list = vec![];
        if self.modelfiles.is_dir() {
            for entry in fs::read_dir(&self.modelfiles)? {
                let path = entry?.path();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if let Some(stem) = name.strip_suffix(".Modelfile") {
                    let content = fs::read_to_string(&path)?;
                    list.push(json!({ "name": stem, "content": content, "model": model_name(stem) }));
                }
            }
        }
        list.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        Ok(json!(list))
    }

    pub fn modelfile_path(&self, name: &str) -> Result<PathBuf> {
        let name = safe_id(name)?;
        let path = self.modelfiles.join(format!("{name}.Modelfile"));
        if !path.is_file() {
            return Err(format!("Modelfile desconhecido: {name}").into());
        }
        Ok(path)
    }

    pub fn save_modelfile(&self, name: &str, content: &str) -> Result<Value> {
        let name = safe_id(name)?;
        if !content.lines().any(|l| l.trim_start().to_ascii_uppercase().starts_with("FROM ")) {
            return Err("O Modelfile precisa de uma linha FROM".into());
        }
        fs::create_dir_all(&self.modelfiles)?;
        fs::write(self.modelfiles.join(format!("{name}.Modelfile")), content)?;
        self.modelfiles()
    }
}

pub fn model_name(modelfile: &str) -> String {
    format!("librarian-{modelfile}")
}

/// Folder picker data; browsers do not expose absolute paths, so the server lists them.
pub fn browse(path: Option<&str>) -> Result<Value> {
    let entry = |path: &Path, name: String| {
        json!({ "name": name, "path": display(path), "cargo": path.join("Cargo.toml").is_file() })
    };
    let Some(path) = path.filter(|p| !p.trim().is_empty()) else {
        let mut dirs = vec![];
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            dirs.push(entry(Path::new(&home), "Pasta do usuário".into()));
        }
        for letter in b'A'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            if Path::new(&drive).is_dir() {
                dirs.push(entry(Path::new(&drive), drive.clone()));
            }
        }
        if cfg!(not(windows)) {
            dirs.push(entry(Path::new("/"), "/".into()));
        }
        return Ok(json!({ "path": null, "parent": null, "cargo": false, "dirs": dirs }));
    };
    let path = Path::new(path.trim()).canonicalize()?;
    let mut dirs = vec![];
    for item in fs::read_dir(&path)? {
        let Ok(item) = item else { continue };
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.starts_with('$') || !item.path().is_dir() {
            continue;
        }
        dirs.push((name.to_lowercase(), entry(&item.path(), name)));
    }
    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(json!({
        "path": display(&path),
        "parent": path.parent().map(display),
        "cargo": path.join("Cargo.toml").is_file(),
        "dirs": dirs.into_iter().take(1000).map(|(_, d)| d).collect::<Vec<_>>(),
    }))
}

fn initial_settings(name: &str, root: &str) -> Result<Settings> {
    let mut settings = Settings {
        name: name.into(),
        root: root.into(),
        inputs: ["src", "tests", "benches", "examples", "Cargo.toml", "Cargo.lock"]
            .iter()
            .filter(|p| Path::new(root).join(p).exists())
            .map(|p| p.to_string())
            .collect(),
        lexicon: Lexicon::default(),
        instructions: None,
        top: 5,
        depth: 2,
        hits: 1,
        radius: 0,
        numbered_lines: true,
        accept_diagnostics: false,
        imported_from: None,
    };
    let profile = Path::new(root).join("playme.json");
    if let Ok(bytes) = fs::read(&profile) {
        let p: Value = serde_json::from_slice(&bytes)?;
        let base = Path::new(root).join(p["project_root"].as_str().unwrap_or("."));
        let file = |field: &str| p[field].as_str().map(|f| Path::new(root).join(f));
        if let Some(path) = file("sources_config") {
            let config: Config = serde_json::from_slice(&fs::read(path)?)?;
            settings.inputs = config.inputs;
        }
        if let Some(path) = file("lexicon") {
            settings.lexicon = serde_json::from_slice(&fs::read(path)?)?;
        }
        if let Some(path) = file("instructions") {
            settings.instructions = Some(serde_json::from_slice(&fs::read(path)?)?);
        }
        let number = |v: &Value, default: usize| v.as_u64().map_or(default, |n| n as usize);
        settings.top = number(&p["search"]["top"], settings.top);
        settings.depth = number(&p["search"]["depth"], settings.depth);
        settings.hits = number(&p["search"]["hits"], settings.hits);
        settings.radius = number(&p["prepare"]["radius"], settings.radius);
        settings.numbered_lines = p["prepare"]["numbered_lines"].as_bool().unwrap_or(false);
        settings.root = display(&base.canonicalize()?);
        settings.imported_from = Some(display(&profile));
    }
    Ok(settings)
}

/// Creates a new directory named `base`, or `base-2`, `base-3`... if taken.
fn reserve(parent: &Path, base: &str) -> Result<(String, PathBuf)> {
    fs::create_dir_all(parent)?;
    for n in 1..1000 {
        let id = if n == 1 { base.to_owned() } else { format!("{base}-{n}") };
        let dir = parent.join(&id);
        match fs::create_dir(&dir) {
            Ok(()) => return Ok((id, dir)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err("Não foi possível reservar um diretório novo".into())
}

pub fn safe_id(id: &str) -> Result<&str> {
    let ok = !id.is_empty()
        && id.len() <= 80
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if ok {
        Ok(id)
    } else {
        Err(format!("Identificador inválido: {id:?}").into())
    }
}

fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.trim_matches('-');
    if s.is_empty() { "projeto".into() } else { s.chars().take(40).collect() }
}

/// Paths without the Windows verbatim prefix, as the user typed them.
pub fn display(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
}

pub fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

/// UTC timestamp `YYYYMMDD-HHMMSS`, ordered like the runs it names.
pub fn stamp() -> String {
    let secs = (now_ms() / 1000) as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // Civil-from-days (Howard Hinnant), valid for the proleptic Gregorian calendar.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn project() -> Scratch {
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-studio-{}-{n}", std::process::id()));
        fs::create_dir_all(dir.join("project/src")).unwrap();
        fs::write(dir.join("project/Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        fs::write(
            dir.join("project/src/lib.rs"),
            "pub struct Camera;\n\nimpl Camera {\n    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;\n        direction\n    }\n}\n",
        )
        .unwrap();
        Scratch(dir)
    }

    #[test]
    fn open_generate_and_ask_light_the_graph_and_prepare_a_numbered_query() {
        let scratch = project();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let opened = store.open(&display(&scratch.0.join("project"))).unwrap();
        let key = opened["key"].as_str().unwrap().to_owned();
        assert!(key.starts_with("project-"));
        // Reopening the same folder finds the same project.
        assert_eq!(store.open(&display(&scratch.0.join("project"))).unwrap()["key"], key);
        let mut settings: Settings = serde_json::from_value(opened["settings"].clone()).unwrap();
        assert_eq!(settings.inputs, ["src", "Cargo.toml"]);
        settings.lexicon.groups = vec![
            vec!["raios".into(), "ray".into()],
            vec!["direcao".into(), "direction".into()],
        ];
        store.save_settings(&key, &settings).unwrap();
        let mut moved = settings.clone();
        moved.root = "outro-lugar".into();
        assert!(store.save_settings(&key, &moved).is_err());

        let run = store.generate(&key).unwrap();
        let run_id = run["id"].as_str().unwrap();
        assert_eq!(run["diagnostics"], 0);
        let graph: Value = serde_json::from_slice(&store.graph(&key, run_id).unwrap()).unwrap();
        assert_eq!(graph["version"], "syntax-graph-v2");

        let turn = store.ask(&key, run_id, "Onde a câmera calcula a direção dos raios?").unwrap();
        assert_eq!(turn["id"], "01");
        assert_eq!(turn["prepare_error"], Value::Null);
        assert!(turn["search"]["hits"][0]["name"].as_str().unwrap().contains("ray_from_pixel"));
        let query: Value = serde_json::from_str(turn["query"].as_str().unwrap()).unwrap();
        let lines = &query["evidence"]["source"]["context"]["lines"];
        assert_eq!(lines[0]["number"], 4);
        // The bundle written for the page verifies against the edition, as before a send.
        let catalog = Catalog::load(&store.edition(&key, run_id).unwrap()).unwrap();
        prepare::verify(&catalog, &store.turn_dir(&key, run_id, "01").unwrap().join("prepared")).unwrap();

        // A question the lexicon cannot reach still records the search, with prepare's refusal.
        let miss = store.ask(&key, run_id, "banco de dados quântico").unwrap();
        assert_eq!(miss["id"], "02");
        assert!(miss["prepare_error"].as_str().is_some());
        assert_eq!(store.turns(&key, run_id).unwrap().as_array().unwrap().len(), 2);
        assert!(store.turn_dir(&key, run_id, "../01").is_err());
    }

    #[test]
    fn modelfiles_require_from_and_safe_names() {
        let scratch = project();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        assert!(store.save_modelfile("cenas", "PARAMETER seed 1\n").is_err());
        assert!(store.save_modelfile("../fora", "FROM qwen\n").is_err());
        let list = store.save_modelfile("cenas", "FROM qwen\nSYSTEM Crie cenas.\n").unwrap();
        assert_eq!(list[0]["name"], "cenas");
        assert_eq!(list[0]["model"], "librarian-cenas");
        assert!(store.modelfile_path("cenas").is_ok());
    }

    #[test]
    fn stamp_is_sortable_utc() {
        let s = stamp();
        assert_eq!(s.len(), 15);
        assert_eq!(&s[8..9], "-");
        assert!(s.starts_with("20"));
    }
}
