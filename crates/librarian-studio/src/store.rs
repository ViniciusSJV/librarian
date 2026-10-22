//! Projects, runs and questions under `.studio/`. Nothing is written inside analyzed projects.
//! Layout: projects/<key>/project.json, runs/<run>/{edition, graph-v2.json, run.json},
//! runs/<run>/turns/<NN>/{question.json, search.json, prepare-request.json, prepared/, claims/, sends/}.
use librarian_llm_engine::claims::Policy;
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

/// Optional profile in the project root: sources, lexicon, instructions, criteria, search.
pub const PROFILE: &str = "librarian.json";
const CRITERIA: &str = "criteria-before.md";
/// Highest question number handed out in a run, so numbers are never reused.
const LAST_TURN: &str = "last-turn.txt";

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
    /// Prior criteria, written with each question before any answer exists.
    #[serde(default)]
    pub criteria: Option<String>,
    /// Profile the first settings were read from, if any; later edits stay in `.studio`.
    #[serde(default)]
    pub imported_from: Option<String>,
    /// Fixed with each question. Projects saved before this field keep `prose`.
    #[serde(default)]
    pub answer_format: AnswerFormat,
    /// Claims policy; INFERENCE and HYPOTHESIS are refused by the checker unless allowed here.
    #[serde(default)]
    pub allow_inference: bool,
    #[serde(default)]
    pub allow_hypothesis: bool,
}

/// `prose` sends the Graph Engine query; `claims` sends a claims query and checks each answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnswerFormat {
    #[default]
    Prose,
    Claims,
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

    /// Opens (or creates) the project for a folder. A `librarian.json` there seeds the settings.
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
        let claims = question["answer_format"] == "claims";
        // `query` is what a send delivers: the claims query in claims mode.
        let query = if claims { dir.join(crate::claims::DIR).join("query.json") } else { dir.join("prepared/bundle/query.json") };
        Ok(json!({
            "id": id,
            "question": question["question"],
            "created": question["created"],
            "answer_format": if claims { "claims" } else { "prose" },
            "claims_error": question["claims_error"],
            "search": read_json(dir.join("search.json")),
            "request": read_json(dir.join("prepare-request.json")),
            "query": fs::read_to_string(query).ok(),
            "criteria": fs::read_to_string(dir.join(CRITERIA)).ok(),
            "criteria_sha256": question["criteria_sha256"],
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
        // After the highest number ever used, kept beside the turns: deleting every question must
        // not hand out 01 again, and a lost record still never goes below the existing folders.
        let last = run_dir.join(LAST_TURN);
        let used = fs::read_to_string(&last).ok().and_then(|s| s.trim().parse::<usize>().ok()).unwrap_or(0);
        let existing = fs::read_dir(&turns)?
            .filter_map(|e| e.ok()?.file_name().to_str()?.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        let next = used.max(existing) + 1;
        let (id, dir) = reserve(&turns, &format!("{next:02}"))?;
        fs::write(&last, format!("{next}\n"))?;
        // Criteria are fixed first, before the query exists and long before any answer.
        let criteria_sha256 = match settings.criteria.as_deref().filter(|c| !c.trim().is_empty()) {
            Some(criteria) => {
                fs::write(dir.join(CRITERIA), criteria)?;
                Some(hash(criteria.as_bytes()))
            }
            None => None,
        };
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
        let format = settings.answer_format;
        let claims_error = match (format, &prepare_error) {
            (AnswerFormat::Claims, None) => {
                let policy = Policy { allow_inference: settings.allow_inference, allow_hypothesis: settings.allow_hypothesis };
                let instructions = settings.instructions.clone().unwrap_or_default();
                crate::claims::write(&catalog, &dir, policy, instructions).err().map(|e| e.to_string())
            }
            _ => None,
        };
        let record = json!({ "question": question, "created": stamp(), "prepare_error": prepare_error,
            "criteria_sha256": criteria_sha256, "answer_format": format, "claims_error": claims_error });
        fs::write(dir.join("question.json"), serde_json::to_vec_pretty(&record)?)?;
        self.turn(key, run, &id)
    }

    /// Checks an answer pasted from a manual send of a claims question; it is kept as a send.
    pub fn paste_answer(&self, key: &str, run: &str, turn: &str, text: &str) -> Result<Value> {
        let dir = self.turn_dir(key, run, turn)?;
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json"))?)?;
        if question["answer_format"] != "claims" {
            return Err("Esta pergunta está em prosa; só respostas em claims são conferidas".into());
        }
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        prepare::verify(&catalog, &dir.join("prepared"))?;
        let context = crate::claims::verify(&catalog, &dir)?;
        let send = next_send(&dir, "manual")?;
        fs::create_dir(&send)?;
        fs::write(send.join("response.txt"), text)?;
        let query = fs::read(dir.join(crate::claims::DIR).join("query.json"))?;
        let mut summary = json!({
            "state": "pasted",
            "provider": "manual",
            "created": stamp(),
            "directory": display(&send),
            "text": text,
            "query_sha256": hash(&query),
            "response_sha256": hash(text.as_bytes()),
            "semantic_evaluation": "pending",
            "criteria_sha256": question["criteria_sha256"],
        });
        summary["check"] = crate::claims::check(&context, &send, text)?;
        fs::write(send.join("send.json"), serde_json::to_vec_pretty(&summary)?)?;
        Ok(summary)
    }

    fn settings(&self, key: &str) -> Result<Settings> {
        Ok(serde_json::from_slice(&fs::read(self.project_dir(key)?.join("project.json"))?)?)
    }

    /// Asks an LLM for lexicon groups over the edition's code vocabulary and checks them.
    /// `provider: manual` only writes and returns the request, for a pasted answer later.
    pub fn suggest_lexicon(&self, key: &str, run: &str, provider: &str, send: crate::llm::Choice) -> Result<Value> {
        let settings = self.settings(key)?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let (id, dir) = reserve(&self.project_dir(key)?.join("lexicon"), &stamp())?;
        let request = crate::lexicon::request(&catalog, &settings.lexicon);
        let mut bytes = serde_json::to_vec_pretty(&request)?;
        bytes.push(b'\n');
        fs::write(dir.join("query.json"), &bytes)?;
        let query = String::from_utf8(bytes)?;
        if provider == "manual" {
            return Ok(json!({ "id": id, "query": query, "directory": display(&dir) }));
        }
        let summary = crate::llm::send(provider, send.request(&dir.join("query.json"), &dir.join("send")))?;
        let checked = match summary["text"].as_str() {
            Some(text) => self.check_text(&catalog, &settings, &dir, text),
            None => json!({ "error": summary["diagnostic"].as_str().unwrap_or("O LLM não devolveu texto") }),
        };
        Ok(json!({ "id": id, "query": query, "send": summary, "proposal": checked, "directory": display(&dir) }))
    }

    /// Checks an answer pasted from a manual send of a previous `suggest_lexicon` request.
    pub fn check_lexicon(&self, key: &str, run: &str, id: &str, text: &str) -> Result<Value> {
        let settings = self.settings(key)?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let dir = self.project_dir(key)?.join("lexicon").join(safe_id(id)?);
        if !dir.join("query.json").is_file() {
            return Err(format!("Pedido de léxico desconhecido: {id}").into());
        }
        fs::write(dir.join("pasted-response.txt"), text)?;
        Ok(json!({ "id": id, "proposal": self.check_text(&catalog, &settings, &dir, text), "directory": display(&dir) }))
    }

    fn check_text(&self, catalog: &Catalog, settings: &Settings, dir: &Path, text: &str) -> Value {
        let checked = crate::lexicon::parse(text)
            .and_then(|proposal| crate::lexicon::check(catalog, &settings.lexicon, &proposal))
            .unwrap_or_else(|e| json!({ "error": e }));
        let _ = fs::write(dir.join("proposal.json"), serde_json::to_vec_pretty(&checked).unwrap_or_default());
        checked
    }

    /// Writes the current settings into the analyzed project as `librarian.json` and the files
    /// it points to. Only on request; `dry_run` lists what would be written and what exists.
    pub fn export_profile(&self, key: &str, dry_run: bool) -> Result<Value> {
        let settings = self.settings(key)?;
        let root = Path::new(&settings.root);
        let existing: Value = fs::read(root.join(PROFILE))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or(Value::Null);
        if existing["project_root"].as_str().is_some_and(|p| p != ".") {
            return Err("O librarian.json existente usa project_root diferente de \".\"; edite-o à mão".into());
        }
        // Keep the paths an existing profile chose; new profiles use ai/librarian/.
        let path = |field: &str, default: &str| -> Result<String> {
            let p = existing[field].as_str().unwrap_or(default).replace('\\', "/");
            if p.starts_with('/') || p.contains(':') || p.split('/').any(|s| s == ".." || s.is_empty()) {
                return Err(format!("{field}: caminho fora do projeto: {p}").into());
            }
            Ok(p)
        };
        let mut profile = json!({
            "project_root": ".",
            "sources_config": path("sources_config", "ai/librarian/sources.json")?,
            "lexicon": path("lexicon", "ai/librarian/lexicon.json")?,
            "search": { "top": settings.top, "depth": settings.depth, "hits": settings.hits },
            "prepare": { "radius": settings.radius, "numbered_lines": settings.numbered_lines,
                "accept_diagnostics": settings.accept_diagnostics },
            "answer": { "format": settings.answer_format, "allow_inference": settings.allow_inference,
                "allow_hypothesis": settings.allow_hypothesis },
        });
        let mut files: Vec<(String, Vec<u8>)> = vec![];
        let pretty = |v: &Value| -> Result<Vec<u8>> {
            let mut b = serde_json::to_vec_pretty(v)?;
            b.push(b'\n');
            Ok(b)
        };
        files.push((profile["sources_config"].as_str().unwrap().into(),
            pretty(&json!({ "project": settings.name, "inputs": settings.inputs }))?));
        files.push((profile["lexicon"].as_str().unwrap().into(), pretty(&json!(settings.lexicon))?));
        if let Some(instructions) = &settings.instructions {
            profile["instructions"] = json!(path("instructions", "ai/librarian/instructions.json")?);
            files.push((profile["instructions"].as_str().unwrap().into(), pretty(&json!(instructions))?));
        }
        if let Some(criteria) = &settings.criteria {
            profile["criteria"] = json!(path("criteria", "ai/librarian/criteria-before.md")?);
            files.push((profile["criteria"].as_str().unwrap().into(), criteria.as_bytes().to_vec()));
        }
        files.insert(0, (PROFILE.into(), pretty(&profile)?));
        let listing: Vec<Value> = files
            .iter()
            .map(|(p, bytes)| {
                let current = fs::read(root.join(p)).ok();
                json!({ "path": p, "exists": current.is_some(), "changes": current.as_deref() != Some(bytes.as_slice()) })
            })
            .collect();
        if !dry_run {
            for (p, bytes) in &files {
                let target = root.join(p);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(target, bytes)?;
            }
        }
        Ok(json!({ "root": settings.root, "files": listing, "written": !dry_run }))
    }

    /// Removes one question with everything sent for it; asked from the page, after confirmation.
    pub fn delete_turn(&self, key: &str, run: &str, id: &str) -> Result<Value> {
        fs::remove_dir_all(self.turn_dir(key, run, id)?)?;
        self.turns(key, run)
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

/// The next numbered send directory of a turn (not created), e.g. sends/03-ollama.
pub fn next_send(turn: &Path, provider: &str) -> Result<PathBuf> {
    let sends = turn.join("sends");
    fs::create_dir_all(&sends)?;
    let next = fs::read_dir(&sends)?.count() + 1;
    Ok(sends.join(format!("{next:02}-{provider}")))
}

pub fn model_name(modelfile: &str) -> String {
    format!("librarian-{modelfile}")
}

/// Folder picker data; browsers do not expose absolute paths, so the server lists them.
pub fn browse(path: Option<&str>) -> Result<Value> {
    let entry = |path: &Path, name: String| {
        json!({ "name": name, "path": display(path), "marker": marker(path) })
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
        "marker": marker(&path),
        "dirs": dirs.into_iter().take(1000).map(|(_, d)| d).collect::<Vec<_>>(),
    }))
}

/// What kind of project a folder looks like, from its manifest files; only a hint for the picker.
fn marker(path: &Path) -> Option<&'static str> {
    if path.join("Cargo.toml").is_file() {
        Some("Cargo")
    } else if ["pyproject.toml", "setup.py", "setup.cfg", "requirements.txt"].iter().any(|f| path.join(f).is_file()) {
        Some("Python")
    } else {
        None
    }
}

fn initial_settings(name: &str, root: &str) -> Result<Settings> {
    // A Cargo project keeps its usual layout; anything else (e.g. Python) uses the whole folder.
    let inputs = if Path::new(root).join("Cargo.toml").is_file() {
        ["src", "tests", "benches", "examples", "Cargo.toml", "Cargo.lock"]
            .iter()
            .filter(|p| Path::new(root).join(p).exists())
            .map(|p| p.to_string())
            .collect()
    } else {
        librarian_ingest::suggested_inputs(Path::new(root))?
    };
    let mut settings = Settings {
        name: name.into(),
        root: root.into(),
        inputs,
        lexicon: Lexicon::default(),
        instructions: None,
        top: 5,
        depth: 2,
        hits: 1,
        radius: 0,
        numbered_lines: true,
        accept_diagnostics: false,
        criteria: None,
        imported_from: None,
        // New projects ask for checkable claims; `prose` is one setting away.
        answer_format: AnswerFormat::Claims,
        allow_inference: false,
        allow_hypothesis: false,
    };
    let profile = Path::new(root).join(PROFILE);
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
        if let Some(path) = file("criteria") {
            settings.criteria = Some(fs::read_to_string(path)?);
        }
        let number = |v: &Value, default: usize| v.as_u64().map_or(default, |n| n as usize);
        settings.top = number(&p["search"]["top"], settings.top);
        settings.depth = number(&p["search"]["depth"], settings.depth);
        settings.hits = number(&p["search"]["hits"], settings.hits);
        settings.radius = number(&p["prepare"]["radius"], settings.radius);
        settings.numbered_lines = p["prepare"]["numbered_lines"].as_bool().unwrap_or(false);
        settings.accept_diagnostics = p["prepare"]["accept_diagnostics"].as_bool().unwrap_or(false);
        if let Some(format) = p["answer"].get("format") {
            settings.answer_format = serde_json::from_value(format.clone())?;
        }
        settings.allow_inference = p["answer"]["allow_inference"].as_bool().unwrap_or(false);
        settings.allow_hypothesis = p["answer"]["allow_hypothesis"].as_bool().unwrap_or(false);
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
        // New projects ask for claims; this test follows the prose query.
        assert_eq!(settings.answer_format, AnswerFormat::Claims);
        settings.answer_format = AnswerFormat::Prose;
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

        // Deleting keeps the other questions, and the next one gets a fresh number.
        let left = store.delete_turn(&key, run_id, "01").unwrap();
        assert_eq!(left.as_array().unwrap().len(), 1);
        assert_eq!(left[0]["id"], "02");
        assert!(store.delete_turn(&key, run_id, "01").is_err());
        assert_eq!(store.ask(&key, run_id, "direção dos raios").unwrap()["id"], "03");
        // Even with every question deleted, numbers already used are not handed out again.
        store.delete_turn(&key, run_id, "02").unwrap();
        assert_eq!(store.delete_turn(&key, run_id, "03").unwrap(), json!([]));
        assert_eq!(store.ask(&key, run_id, "direção dos raios").unwrap()["id"], "04");
    }

    #[test]
    fn claims_questions_derive_a_checked_context_and_pasted_answers_are_checked() {
        let scratch = project();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let opened = store.open(&display(&scratch.0.join("project"))).unwrap();
        let key = opened["key"].as_str().unwrap().to_owned();
        let mut settings: Settings = serde_json::from_value(opened["settings"].clone()).unwrap();
        settings.lexicon.groups = vec![vec!["raios".into(), "ray".into()], vec!["direcao".into(), "direction".into()]];
        store.save_settings(&key, &settings).unwrap();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();

        let turn = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        assert_eq!((turn["answer_format"].as_str(), &turn["claims_error"]), (Some("claims"), &Value::Null));
        // The query sent is the claims query; its evidence is the bundle window, with the symbol.
        let query: Value = serde_json::from_str(turn["query"].as_str().unwrap()).unwrap();
        let item = &query["evidence"]["items"][0];
        assert_eq!((item["id"].as_str(), item["start_line"].as_u64(), item["end_line"].as_u64()), (Some("E1"), Some(4), Some(7)));
        assert_eq!(item["symbols"], json!([{"name": "ray_from_pixel", "declaration_line": 4, "name_column": 12, "end_line": 7}]));
        assert_eq!(query["require_location"], true);
        assert_eq!(query["strict_text"], true);
        assert!(query["evidence"]["limits"][0].as_str().unwrap().starts_with("Lexical candidates"));

        let answer = json!({"schema_version": 1, "claims": [{
            "id": "C1", "kind": "FACT", "text": "`ray_from_pixel` calcula `direction` na linha 5 de src/lib.rs.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 4, "end_line": 5,
                "quote": "    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;"}],
            "premises": [], "verification_plan": null,
            "location": {"evidence_id": "E1", "path": "src/lib.rs", "symbol_name": "ray_from_pixel",
                "declaration_line": 4, "operation_start_line": 5, "operation_end_line": 5}}]});
        let fenced = format!("```json\n{answer}\n```");
        let passed = store.paste_answer(&key, &run, "01", &fenced).unwrap();
        assert_eq!(passed["check"]["report"]["mechanical_status"], "passed");
        assert_eq!(passed["check"]["report"]["accepted"], false);
        assert_eq!(passed["check"]["extracted_from_fence"], true);
        assert_eq!(passed["check"]["claims"][0]["location"]["symbol_name"], "ray_from_pixel");

        let mut wrong = answer.clone();
        wrong["claims"][0]["citations"][0]["quote"] = json!("let direction = x * 2.0;");
        let rejected = store.paste_answer(&key, &run, "01", &wrong.to_string()).unwrap();
        assert_eq!(rejected["check"]["report"]["mechanical_status"], "rejected");
        assert_eq!(rejected["check"]["report"]["issues"][0]["code"], "quote_mismatch");
        // strict_text: the text may only state lines and code its own citations show.
        let mut beyond = answer.clone();
        beyond["claims"][0]["text"] = json!("`ray_from_pixel` devolve `direction` na linha 6.");
        let issues = &store.paste_answer(&key, &run, "01", &beyond.to_string()).unwrap()["check"]["report"]["issues"];
        assert_eq!(issues, &json!([{"claim_id": "C1", "code": "text_cites_uncited_line"}]));
        let sends = &store.turn(&key, &run, "01").unwrap()["sends"];
        assert_eq!(sends.as_array().unwrap().len(), 3);
        assert!(sends[1]["directory"].as_str().unwrap().ends_with("02-manual"));
        assert!(store.turn_dir(&key, &run, "01").unwrap().join("sends/01-manual/check.json").is_file());

        // A context edited after the question no longer matches the bundle and is refused.
        let context = store.turn_dir(&key, &run, "01").unwrap().join("claims/context.json");
        let original = fs::read_to_string(&context).unwrap();
        fs::write(&context, original.replace("\"allow_hypothesis\": false", "\"allow_hypothesis\": true")).unwrap();
        assert!(store.paste_answer(&key, &run, "01", &answer.to_string()).is_err());
        fs::write(&context, &original).unwrap();
        assert!(store.paste_answer(&key, &run, "01", &answer.to_string()).is_ok());

        // Prose questions keep the Graph Engine query and are not checked.
        settings.answer_format = AnswerFormat::Prose;
        store.save_settings(&key, &settings).unwrap();
        let prose = store.ask(&key, &run, "direção dos raios").unwrap();
        assert_eq!(prose["answer_format"], "prose");
        assert!(serde_json::from_str::<Value>(prose["query"].as_str().unwrap()).unwrap()["evidence"]["source"].is_object());
        assert!(store.paste_answer(&key, &run, "02", &answer.to_string()).is_err());
    }

    #[test]
    fn profile_seeds_settings_and_criteria_are_fixed_with_each_question() {
        let scratch = project();
        let root = scratch.0.join("project");
        fs::create_dir_all(root.join("ai")).unwrap();
        fs::write(root.join("ai/sources.json"), r#"{"project": "demo", "inputs": ["src"]}"#).unwrap();
        fs::write(root.join("ai/lexicon.json"), r#"{"groups": [["raios", "ray"], ["direcao", "direction"]]}"#).unwrap();
        fs::write(root.join("ai/instructions.json"), r#"["Cite E1."]"#).unwrap();
        fs::write(root.join("ai/criteria.md"), "- Cita E1, arquivo e linha.\n").unwrap();
        fs::write(
            root.join(PROFILE),
            r#"{"project_root": ".", "sources_config": "ai/sources.json", "lexicon": "ai/lexicon.json",
                "instructions": "ai/instructions.json", "criteria": "ai/criteria.md",
                "search": {"top": 3, "depth": 1, "hits": 1}, "prepare": {"radius": 1, "numbered_lines": true},
                "answer": {"format": "prose", "allow_hypothesis": true}}"#,
        )
        .unwrap();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let opened = store.open(&display(&root)).unwrap();
        let settings: Settings = serde_json::from_value(opened["settings"].clone()).unwrap();
        assert_eq!(settings.inputs, ["src"]);
        assert_eq!((settings.top, settings.depth, settings.radius), (3, 1, 1));
        assert_eq!(settings.instructions.as_deref(), Some(&["Cite E1.".to_string()][..]));
        assert_eq!(
            (settings.answer_format, settings.allow_inference, settings.allow_hypothesis),
            (AnswerFormat::Prose, false, true)
        );
        assert!(settings.imported_from.unwrap().ends_with(PROFILE));
        let key = opened["key"].as_str().unwrap();
        let run = store.generate(key).unwrap();
        let turn = store.ask(key, run["id"].as_str().unwrap(), "direção dos raios").unwrap();
        assert_eq!(turn["criteria"], "- Cita E1, arquivo e linha.\n");
        assert_eq!(turn["criteria_sha256"], hash(b"- Cita E1, arquivo e linha.\n"));
        let query: Value = serde_json::from_str(turn["query"].as_str().unwrap()).unwrap();
        assert_eq!(query["instructions"], json!(["Cite E1."]));
    }

    #[test]
    fn lexicon_proposals_are_checked_and_profiles_exported_on_request() {
        let scratch = project();
        let root = scratch.0.join("project");
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();

        // Manual: the request is written and returned; nothing is sent.
        let manual = store.suggest_lexicon(&key, &run, "manual", crate::llm::Choice {
            modelfile: None, modelfile_name: None, claude_model: None }).unwrap();
        let query: Value = serde_json::from_str(manual["query"].as_str().unwrap()).unwrap();
        let terms: Vec<_> = query["evidence"]["code_terms"].as_array().unwrap().iter().map(|t| t["term"].clone()).collect();
        assert!(terms.contains(&json!("camera")) && terms.contains(&json!("pixel")));

        let answer = "Aqui está:\n```json\n{\"groups\": [[\"raio\", \"raios\", \"ray\"], [\"banco\", \"dados\"], \
            [\"duas palavras\", \"câmera\", \"camera\"], [\"ray\", \"feixe\"]]}\n```";
        let checked = store.check_lexicon(&key, &run, manual["id"].as_str().unwrap(), answer).unwrap();
        let proposal = &checked["proposal"];
        assert_eq!((proposal["accepted"].as_u64(), proposal["rejected"].as_u64()), (Some(1), Some(3)));
        let report = proposal["report"].as_array().unwrap();
        assert_eq!(report[0]["words"], json!(["raio", "raios", "ray"]));
        assert_eq!(report[1]["notes"], json!(["nenhuma palavra existe no código"]));
        // Multi-word items are dropped, accents folded; the rest of the group survives.
        assert_eq!(report[2]["words"], json!(["camera"]));
        assert_eq!(report[2]["status"], "rejected");
        assert!(report[3]["notes"][0].as_str().unwrap().contains("já está em outro grupo"));
        assert_eq!(proposal["lexicon"]["groups"], json!([["raio", "raios", "ray"]]));
        assert!(store.check_lexicon(&key, &run, "../x", answer).is_err());
        assert!(store.check_lexicon(&key, &run, manual["id"].as_str().unwrap(), "sem json").unwrap()["proposal"]["error"].is_string());

        // Export: a dry run lists the files; only the real run writes them into the project.
        let plan = store.export_profile(&key, true).unwrap();
        let paths: Vec<_> = plan["files"].as_array().unwrap().iter().map(|f| f["path"].as_str().unwrap().to_owned()).collect();
        assert_eq!(paths, ["librarian.json", "ai/librarian/sources.json", "ai/librarian/lexicon.json"]);
        assert!(!root.join(PROFILE).exists());
        store.export_profile(&key, false).unwrap();
        let written: Value = serde_json::from_slice(&fs::read(root.join(PROFILE)).unwrap()).unwrap();
        assert_eq!(written["sources_config"], "ai/librarian/sources.json");
        let sources: Value = serde_json::from_slice(&fs::read(root.join("ai/librarian/sources.json")).unwrap()).unwrap();
        assert_eq!(sources["inputs"], json!(["src", "Cargo.toml"]));
        // An existing profile keeps the paths it chose.
        let mut moved = written.clone();
        moved["lexicon"] = json!("ai/acervo/meu-lexico.json");
        fs::write(root.join(PROFILE), serde_json::to_vec(&moved).unwrap()).unwrap();
        let again = store.export_profile(&key, true).unwrap();
        assert_eq!(again["files"][2]["path"], "ai/acervo/meu-lexico.json");
        assert_eq!(again["files"][2]["exists"], false);
        moved["lexicon"] = json!("../fora.json");
        fs::write(root.join(PROFILE), serde_json::to_vec(&moved).unwrap()).unwrap();
        assert!(store.export_profile(&key, true).is_err());
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
