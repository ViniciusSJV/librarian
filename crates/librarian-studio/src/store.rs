//! Projects, runs and questions under `.studio/`. Nothing is written inside analyzed projects.
//! Layout: projects/<key>/project.json, runs/<run>/{edition, graph-v2.json, run.json},
//! runs/<run>/turns/<NN>/{question.json, search.json, prepare-request.json, prepared/, claims/, sends/}.
use librarian_llm_engine::claims::{self, compare::compare, Claim, Policy};
use librarian_ingest::{
    graph::v2,
    hash,
    prepare::{self, Request, ReviewedFact, Target},
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
/// At most this many questions from one path at once (P1b).
const MAX_PATH_QUESTIONS: usize = 40;

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
    /// A HYPOTHESIS may carry a unified diff, checked against the evidence (performance step).
    #[serde(default)]
    pub allow_patch: bool,
    /// The search leaves out test symbols and the declarations inside them (P1b).
    #[serde(default)]
    pub exclude_tests: bool,
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

/// What a send delivers once its question was re-verified (`Store::verify_for_send`).
pub struct Verified {
    pub query: PathBuf,
    pub context: Option<librarian_llm_engine::claims::Context>,
    pub criteria_sha256: Option<String>,
    /// Recorded with the send (`send.json`, and the Ollama attempt's `result.json`).
    pub link: Value,
}

impl Store {
    pub(crate) fn project_dir(&self, key: &str) -> Result<PathBuf> {
        let dir = self.dir.join("projects").join(safe_id(key)?);
        if !dir.join("project.json").is_file() {
            return Err(format!("Projeto desconhecido: {key}").into());
        }
        Ok(dir)
    }
    pub(crate) fn run_dir(&self, key: &str, run: &str) -> Result<PathBuf> {
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

    /// Terms of the project's questions, in every run, that touched no search result, with how
    /// many questions used them; words the lexicon already covers (stop words, groups) are left out.
    /// Most used first.
    pub fn missed_terms(&self, key: &str, lexicon: &librarian_ingest::search::Lexicon) -> Result<Vec<(String, usize)>> {
        use librarian_ingest::search::{tokens, BUILTIN_STOP_WORDS};
        use std::collections::{BTreeMap, BTreeSet};
        let mut known: BTreeSet<String> = tokens(BUILTIN_STOP_WORDS).into_iter().collect();
        known.extend(lexicon.stop_words.iter().chain(lexicon.groups.iter().flatten()).flat_map(|w| tokens(w)));
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        let runs = self.project_dir(key)?.join("runs");
        for run in fs::read_dir(&runs).into_iter().flatten().flatten() {
            for turn in fs::read_dir(run.path().join("turns")).into_iter().flatten().flatten() {
                let Some(search) = fs::read(turn.path().join("search.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else {
                    continue;
                };
                let touched: BTreeSet<&str> = search["hits"].as_array().into_iter().flatten()
                    .flat_map(|h| h["reasons"].as_array().into_iter().flatten().filter_map(|r| r["term"].as_str()))
                    .collect();
                for term in search["terms"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                    if !touched.contains(term) && !known.contains(term) {
                        *counts.entry(term.to_owned()).or_default() += 1;
                    }
                }
            }
        }
        let mut missed: Vec<(String, usize)> = counts.into_iter().collect();
        missed.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Ok(missed)
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
            sends.extend(paths.into_iter().filter_map(|p| {
                let mut send = read_json(p.join("send.json"))?;
                send["id"] = json!(p.file_name()?.to_string_lossy());
                send["reviews"] = Store::reviews(&p);
                Some(send)
            }));
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
            "chain": question["chain"],
            "path": question["path"],
            "prepare_error": question["prepare_error"],
            "comparison": compare_sends(&sends),
            "sends": sends,
        }))
    }

    /// Search lights the graph even when prepare refuses; the refusal is kept with the question.
    pub fn ask(&self, key: &str, run: &str, question: &str) -> Result<Value> {
        let settings = self.settings(key)?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let mut index = SearchIndex::new(&catalog, settings.lexicon.clone())?;
        if settings.exclude_tests {
            index = index.without_tests();
        }
        let report = index.search(question, settings.top, settings.depth)?;
        let mut search = serde_json::to_vec_pretty(&report)?;
        search.push(b'\n');
        let request = request(&settings, question, None, vec![], false);
        self.new_turn(key, run, &settings, &catalog, question, Some(search), request, None)
    }

    /// P1b: one question per declaration the person picked on a "what does it call?" path, each
    /// with that declaration alone as evidence (like `hits: 1`). Nothing is searched; the path
    /// (seed, position, the choices among homonyms) is recorded with each question.
    pub fn ask_path(&self, key: &str, run: &str, question: &str, nodes: &[String], seed: &str, chosen: &Value) -> Result<Value> {
        if nodes.is_empty() || nodes.len() > MAX_PATH_QUESTIONS {
            return Err(format!("Escolha de 1 a {MAX_PATH_QUESTIONS} pontos do caminho").into());
        }
        let settings = self.settings(key)?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let symbols = catalog.symbols();
        let mut turns = vec![];
        for (index, id) in nodes.iter().enumerate() {
            let symbol = symbols.get(id).ok_or_else(|| format!("Ponto desconhecido: {id}"))?;
            let target = Target {
                path: catalog.sources()[&symbol.source_id].path.clone(),
                symbol_name: symbol.name.clone(),
                declaration_line: catalog.chunks().get(&symbol.chunk_id).ok_or("Ponto sem trecho")?.start_line,
            };
            let path = json!({ "seed": seed, "node": id, "index": index + 1, "total": nodes.len(), "chosen": chosen });
            let mut request = request(&settings, question, Some(vec![target]), vec![], false);
            request.picked = true;
            turns.push(self.new_turn(key, run, &settings, &catalog, question, None, request, Some(("path", path)))?);
        }
        Ok(json!(turns))
    }

    /// Chain step: a new question whose evidence is the declarations located by the answers to
    /// `from` that pass the mechanical check again now, plus the current reviewed facts about
    /// them. The earlier answers' text is not carried; they are linked by hash.
    pub fn continue_turn(&self, key: &str, run: &str, from: &str, question: &str) -> Result<Value> {
        let settings = self.settings(key)?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let verified = self.verify_for_send(key, run, from)?;
        let context = verified.context.ok_or("Só perguntas em claims continuam em cadeia")?;
        let mut targets: Vec<Target> = vec![];
        let mut answers = vec![];
        let mut sends: Vec<PathBuf> = fs::read_dir(self.turn_dir(key, run, from)?.join("sends"))
            .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        sends.sort();
        for send in sends {
            let Some(text) = ["response.txt", "attempt/response.txt"].iter().find_map(|f| fs::read_to_string(send.join(f)).ok()) else {
                continue;
            };
            // Read as the check read it: out of a fence, with the citations it completed.
            let (body, _, _) = crate::claims::resolved(&context, &text);
            if claims::check(&context, &body)?.mechanical_status != "passed" {
                continue;
            }
            let response: claims::Response = serde_json::from_str(&body)?;
            let mut used = vec![];
            for claim in response.claims.iter().filter(|c| matches!(c.kind, claims::Kind::Fact)) {
                let Some(location) = &claim.location else { continue };
                let target = Target { path: location.path.clone(), symbol_name: location.symbol_name.clone(),
                    declaration_line: location.declaration_line };
                if !targets.contains(&target) {
                    targets.push(target);
                }
                used.push(claim.id.clone());
            }
            if !used.is_empty() {
                answers.push(json!({ "send": send.file_name().map(|n| n.to_string_lossy().into_owned()),
                    "response_sha256": hash(text.as_bytes()), "claims": used }));
            }
        }
        if targets.is_empty() {
            return Err(format!("Nenhuma resposta da pergunta {from} passa na conferência mecânica com location; nada para continuar").into());
        }
        let facts = self.facts(key, run)?;
        let reviewed: Vec<ReviewedFact> = facts["facts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|f| f["status"] == "current")
            .filter(|f| targets.iter().any(|t| f["path"] == t.path && f["symbol_name"] == t.symbol_name && f["declaration_line"] == t.declaration_line))
            .map(|f| serde_json::from_value(json!({ "id": f["id"], "statement": f["statement"], "authorship": f["authorship"],
                "path": f["path"], "line": f["line"], "end_line": f["end_line"], "citations": f["citations"] })))
            .collect::<std::result::Result<_, _>>()?;
        // G6d: on editions that record impl types and field writes, a method target also brings
        // its owner type and the declarations that write the fields it reads.
        let owner_context = catalog.owner_context();
        let chain = json!({ "from_turn": from, "from_query_sha256": verified.link["query_sha256"], "answers": answers,
            "targets": targets, "reviewed_facts": reviewed.iter().map(|f| &f.id).collect::<Vec<_>>(),
            "owner_context": owner_context });
        let request = request(&settings, question, Some(targets), reviewed, owner_context);
        self.new_turn(key, run, &settings, &catalog, question, None, request, Some(("chain", chain)))
    }

    /// Fixes the criteria, prepares and verifies the bundle and, in claims, its claims files.
    /// A refusal is kept with the question. Without `search`, the prepared report is recorded.
    #[allow(clippy::too_many_arguments)]
    fn new_turn(&self, key: &str, run: &str, settings: &Settings, catalog: &Catalog, question: &str,
        search: Option<Vec<u8>>, request: Request, origin: Option<(&str, Value)>) -> Result<Value> {
        let run_dir = self.run_dir(key, run)?;
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
        if let Some(search) = &search {
            fs::write(dir.join("search.json"), search)?;
        }
        let request_bytes = serde_json::to_vec_pretty(&request)?;
        fs::write(dir.join("prepare-request.json"), &request_bytes)?;
        let prepared = dir.join("prepared");
        let prepare_error = prepare::write(catalog, &request_bytes, &prepared)
            .and_then(|_| prepare::verify(catalog, &prepared))
            .err()
            .map(|e| e.to_string());
        if search.is_none() && prepare_error.is_none() {
            fs::copy(prepared.join("search.json"), dir.join("search.json"))?;
        }
        let format = settings.answer_format;
        let claims_error = match (format, &prepare_error) {
            (AnswerFormat::Claims, None) => {
                let policy = Policy { allow_inference: settings.allow_inference, allow_hypothesis: settings.allow_hypothesis };
                let instructions = settings.instructions.clone().unwrap_or_default();
                crate::claims::write(catalog, &dir, policy, instructions, settings.allow_patch).err().map(|e| e.to_string())
            }
            _ => None,
        };
        let mut record = json!({ "question": question, "created": stamp(), "prepare_error": prepare_error,
            "criteria_sha256": criteria_sha256, "answer_format": format, "claims_error": claims_error });
        // Where the evidence came from when nothing was searched: a chain step or a picked path.
        if let Some((field, value)) = origin {
            record[field] = value;
        }
        fs::write(dir.join("question.json"), serde_json::to_vec_pretty(&record)?)?;
        self.turn(key, run, &id)
    }

    /// An answer pasted by the person (provider `manual`), for a claims question: verified and
    /// linked like a send, then checked. The query copied to the send is the one verified now;
    /// who wrote the answer is not known, only that it was pasted.
    pub fn paste_answer(&self, key: &str, run: &str, turn: &str, text: &str) -> Result<Value> {
        self.record_answer(key, run, turn, "manual", text)
    }

    /// Records an answer as a send would, with the same checks and files. Tests also use it to
    /// stand in for a provider.
    fn record_answer(&self, key: &str, run: &str, turn: &str, provider: &str, text: &str) -> Result<Value> {
        if text.trim().is_empty() {
            return Err("Cole a resposta antes de conferir".into());
        }
        let dir = self.turn_dir(key, run, turn)?;
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json"))?)?;
        if question["answer_format"] != "claims" {
            return Err("Só perguntas em claims recebem resposta colada".into());
        }
        let verified = self.verify_for_send(key, run, turn)?;
        let context = verified.context.ok_or("Pergunta em claims sem contexto")?;
        let send = next_send(&dir, provider)?;
        fs::create_dir(&send)?;
        // Preserved as the Claude CLI send does: the query delivered and the answer received.
        fs::copy(&verified.query, send.join("query.json"))?;
        fs::write(send.join("response.txt"), text)?;
        let mut summary = json!({ "state": "completed", "provider": provider, "text": text, "directory": display(&send),
            "link": verified.link, "pasted": provider == "manual" });
        summary["check"] = crate::claims::check(&context, &send, text)?;
        fs::write(send.join("send.json"), serde_json::to_vec_pretty(&summary)?)?;
        Ok(summary)
    }

    /// Re-verifies a question immediately before a send and links the send to what was verified:
    /// the bundle against the edition, the claims files against the bundle, the criteria against
    /// the hash fixed with the question. The link is metadata written after those checks, not a
    /// signature; the provider refuses a query whose bytes no longer have `query_sha256`.
    pub fn verify_for_send(&self, key: &str, run: &str, turn: &str) -> Result<Verified> {
        let dir = self.turn_dir(key, run, turn)?;
        let prepared = dir.join("prepared");
        if !prepared.join("bundle/query.json").is_file() {
            return Err("Esta pergunta não tem query preparada (o prepare recusou); nada foi enviado".into());
        }
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        prepare::verify(&catalog, &prepared)?;
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json"))?)?;
        let criteria_sha256 = question["criteria_sha256"].as_str().map(str::to_owned);
        if let Some(expected) = &criteria_sha256 {
            if hash(&fs::read(dir.join(CRITERIA))?) != *expected {
                return Err("Critérios prévios diferem dos gravados com a pergunta; nada foi enviado".into());
            }
        }
        // A claims question sends its own query, re-derived from the bundle just verified.
        let context = match question["answer_format"] == "claims" {
            true => Some(crate::claims::verify(&catalog, &dir)?),
            false => None,
        };
        let query_file = match context {
            Some(_) => format!("{}/query.json", crate::claims::DIR),
            None => "prepared/bundle/query.json".to_owned(),
        };
        let query = dir.join(&query_file);
        let origin_bytes = fs::read(prepared.join("bundle/origin.json"))?;
        let origin: Value = serde_json::from_slice(&origin_bytes)?;
        let claims = match context {
            Some(_) => json!({ "context_sha256": hash(&fs::read(dir.join(crate::claims::DIR).join("context.json"))?) }),
            None => Value::Null,
        };
        let mut checks = vec!["prepare::verify"];
        checks.extend(context.is_some().then_some("claims::verify"));
        checks.extend(criteria_sha256.is_some().then_some("criteria_sha256"));
        let link = json!({
            "format_version": 1,
            "kind": "studio_send_link",
            "turn": turn,
            "query_file": query_file,
            "query_sha256": hash(&fs::read(&query)?),
            "dossier_origin": origin["dossier"]["source_path"],
            "selection": origin["selection"],
            "bundle": { "origin_sha256": hash(&origin_bytes), "dossier_sha256": origin["dossier"]["sha256"],
                "query_sha256": origin["query"]["sha256"] },
            "claims": claims,
            "criteria_sha256": criteria_sha256,
            "checks": checks,
            "verified_unix_ms": now_ms(),
        });
        Ok(Verified { query, context, criteria_sha256, link })
    }

    pub(crate) fn settings(&self, key: &str) -> Result<Settings> {
        Ok(serde_json::from_slice(&fs::read(self.project_dir(key)?.join("project.json"))?)?)
    }

    /// Asks an LLM for lexicon groups over the edition's code vocabulary and checks them.
    /// `provider: manual` only writes and returns the request, for a pasted answer later.
    pub fn suggest_lexicon(&self, key: &str, run: &str, provider: &str, language: &str, send: crate::llm::Choice) -> Result<Value> {
        if !crate::lexicon::LANGUAGES.contains(&language) {
            return Err(format!("Idioma do léxico desconhecido: {language}").into());
        }
        let settings = self.settings(key)?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let missed = self.missed_terms(key, &settings.lexicon)?;
        let (id, dir) = reserve(&self.project_dir(key)?.join("lexicon"), &stamp())?;
        let request = crate::lexicon::request(&catalog, &settings.lexicon, language, &missed);
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
            "search": { "top": settings.top, "depth": settings.depth, "hits": settings.hits, "exclude_tests": settings.exclude_tests },
            "prepare": { "radius": settings.radius, "numbered_lines": settings.numbered_lines,
                "accept_diagnostics": settings.accept_diagnostics },
            "answer": { "format": settings.answer_format, "allow_inference": settings.allow_inference,
                "allow_hypothesis": settings.allow_hypothesis, "allow_patch": settings.allow_patch },
        });
        // Commands are written by hand in the profile and approved one by one; the studio
        // never writes them, so an export keeps them exactly as they are.
        if let Some(commands) = existing.get("commands") {
            profile["commands"] = commands.clone();
        }
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

/// Checked answers of one question side by side, by content (L2b). All of them count, rejected
/// ones included and marked: a rejected answer may still point at the right place.
/// Recomputed from the stored checks; nothing new is written.
fn compare_sends(sends: &[Value]) -> Value {
    let checked: Vec<&Value> = sends.iter().filter(|s| s["check"].is_object()).collect();
    if checked.len() < 2 {
        return Value::Null;
    }
    let parsed: Vec<Option<Vec<Claim>>> =
        checked.iter().map(|s| serde_json::from_value(s["check"]["claims"].clone()).ok()).collect();
    let answers: Vec<Option<&[Claim]>> = parsed.iter().map(|p| p.as_deref()).collect();
    let columns: Vec<Value> = checked
        .iter()
        .zip(&parsed)
        .map(|(s, claims)| {
            json!({ "directory": s["directory"], "provider": s["provider"], "model": s["model"],
                "modelfile": s["modelfile"], "readable": claims.is_some(),
                "passed": s["check"]["report"]["mechanical_status"] == "passed" })
        })
        .collect();
    json!({ "sends": columns, "rows": compare(&answers) })
}

/// The prepare request of a question with the project's settings; a chain step adds targets.
fn request(settings: &Settings, question: &str, targets: Option<Vec<Target>>, reviewed_facts: Vec<ReviewedFact>,
    owner_context: bool) -> Request {
    Request {
        question: question.into(),
        lexicon: settings.lexicon.clone(),
        top: settings.top,
        depth: settings.depth,
        hits: settings.hits,
        radius: settings.radius,
        numbered_lines: settings.numbered_lines,
        instructions: settings.instructions.clone(),
        accept_diagnostics: settings.accept_diagnostics,
        // A chain step does not search; the option only shapes a lexical search.
        exclude_tests: settings.exclude_tests && targets.is_none(),
        picked: false,
        targets,
        reviewed_facts,
        owner_context,
        // New chain steps also bring what each target calls without ambiguity (P4); a
        // recorded question keeps its request as it was written, so its bytes still verify.
        callees: owner_context,
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
        allow_patch: false,
        exclude_tests: false,
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
        settings.exclude_tests = p["search"]["exclude_tests"].as_bool().unwrap_or(false);
        settings.radius = number(&p["prepare"]["radius"], settings.radius);
        settings.numbered_lines = p["prepare"]["numbered_lines"].as_bool().unwrap_or(false);
        settings.accept_diagnostics = p["prepare"]["accept_diagnostics"].as_bool().unwrap_or(false);
        if let Some(format) = p["answer"].get("format") {
            settings.answer_format = serde_json::from_value(format.clone())?;
        }
        settings.allow_inference = p["answer"]["allow_inference"].as_bool().unwrap_or(false);
        settings.allow_hypothesis = p["answer"]["allow_hypothesis"].as_bool().unwrap_or(false);
        settings.allow_patch = p["answer"]["allow_patch"].as_bool().unwrap_or(false);
        settings.root = display(&base.canonicalize()?);
        settings.imported_from = Some(display(&profile));
    }
    Ok(settings)
}

/// Creates a new directory named `base`, or `base-2`, `base-3`... if taken.
pub(crate) fn reserve(parent: &Path, base: &str) -> Result<(String, PathBuf)> {
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

    fn record_answer(store: &Store, key: &str, run: &str, turn: &str, provider: &str, text: &str) -> Result<Value> {
        store.record_answer(key, run, turn, provider, text)
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
        // The bundle written for the page verifies against the edition, as before a send, and the
        // send is linked to the exact query and bundle verified (L3).
        let dir = store.turn_dir(&key, run_id, "01").unwrap();
        let verified = store.verify_for_send(&key, run_id, "01").unwrap();
        let link = &verified.link;
        assert!(verified.context.is_none());
        assert_eq!(verified.query, dir.join("prepared/bundle/query.json"));
        assert_eq!(link["query_file"], "prepared/bundle/query.json");
        assert_eq!(link["query_sha256"], hash(&fs::read(&verified.query).unwrap()));
        assert_eq!(link["bundle"]["query_sha256"], link["query_sha256"]);
        let origin: Value = serde_json::from_slice(&fs::read(dir.join("prepared/bundle/origin.json")).unwrap()).unwrap();
        assert_eq!(link["selection"], origin["selection"]);
        assert!(link["dossier_origin"].as_str().unwrap().starts_with("edition:"));
        assert_eq!((&link["claims"], &link["checks"]), (&Value::Null, &json!(["prepare::verify"])));
        // A query edited after the question is refused before any send.
        let original = fs::read(&verified.query).unwrap();
        fs::write(&verified.query, b"{}\n").unwrap();
        assert!(store.verify_for_send(&key, run_id, "01").is_err());
        fs::write(&verified.query, &original).unwrap();

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
        // An empty paste records nothing; a pasted answer is linked and checked like a send.
        assert!(store.paste_answer(&key, &run, "01", "  \n").is_err());
        let passed = store.paste_answer(&key, &run, "01", &fenced).unwrap();
        assert_eq!((passed["provider"].as_str(), &passed["pasted"]), (Some("manual"), &json!(true)));
        assert!(passed["link"]["query_sha256"].is_string());
        assert_eq!(passed["check"]["report"]["mechanical_status"], "passed");
        assert_eq!(passed["check"]["report"]["accepted"], false);
        assert_eq!(passed["check"]["extracted_from_fence"], true);
        assert_eq!(passed["check"]["claims"][0]["location"]["symbol_name"], "ray_from_pixel");

        let mut wrong = answer.clone();
        wrong["claims"][0]["citations"][0]["quote"] = json!("let direction = x * 2.0;");
        let rejected = record_answer(&store, &key, &run, "01", "manual", &wrong.to_string()).unwrap();
        assert_eq!(rejected["check"]["report"]["mechanical_status"], "rejected");
        assert_eq!(rejected["check"]["report"]["issues"][0]["code"], "quote_mismatch");
        // strict_text: the text may only state lines and code its own citations show.
        let mut beyond = answer.clone();
        beyond["claims"][0]["text"] = json!("`ray_from_pixel` devolve `direction` na linha 6.");
        let issues = &record_answer(&store, &key, &run, "01", "manual", &beyond.to_string()).unwrap()["check"]["report"]["issues"];
        assert_eq!(issues, &json!([{"claim_id": "C1", "code": "text_cites_uncited_line"}]));
        let sends = &store.turn(&key, &run, "01").unwrap()["sends"];
        assert_eq!(sends.as_array().unwrap().len(), 3);
        assert!(sends[1]["directory"].as_str().unwrap().ends_with("02-manual"));
        assert!(store.turn_dir(&key, &run, "01").unwrap().join("sends/01-manual/check.json").is_file());
        // In claims mode the link names the claims query and the context it was derived with.
        let link = &sends[0]["link"];
        assert_eq!(link["query_file"], "claims/query.json");
        assert_eq!(link["query_sha256"], hash(turn["query"].as_str().unwrap().as_bytes()));
        assert_ne!(link["query_sha256"], link["bundle"]["query_sha256"]);
        assert_eq!(link["checks"], json!(["prepare::verify", "claims::verify"]));
        assert!(link["claims"]["context_sha256"].is_string());

        // A context edited after the question no longer matches the bundle and is refused.
        let context = store.turn_dir(&key, &run, "01").unwrap().join("claims/context.json");
        let original = fs::read_to_string(&context).unwrap();
        fs::write(&context, original.replace("\"allow_hypothesis\": false", "\"allow_hypothesis\": true")).unwrap();
        assert!(record_answer(&store, &key, &run, "01", "manual", &answer.to_string()).is_err());
        fs::write(&context, &original).unwrap();
        assert!(record_answer(&store, &key, &run, "01", "manual", &answer.to_string()).is_ok());

        // L2b: the four answers differ in wording and in mechanical result, not in content.
        let comparison = &store.turn(&key, &run, "01").unwrap()["comparison"];
        let passed: Vec<_> = comparison["sends"].as_array().unwrap().iter().map(|s| s["passed"].as_bool().unwrap()).collect();
        assert_eq!(passed, [true, false, false, true]);
        assert_eq!(comparison["rows"].as_array().unwrap().len(), 1);
        assert_eq!(comparison["rows"][0]["content"]["symbol"], json!(["E1", "ray_from_pixel", 4]));
        assert!(comparison["rows"][0]["cells"].as_array().unwrap().iter().all(|c| c["match"] == "equal"));
        // A different operation line is another content: equal where it was said, partial nowhere else.
        let mut elsewhere = answer.clone();
        elsewhere["claims"][0]["location"]["operation_start_line"] = json!(6);
        elsewhere["claims"][0]["location"]["operation_end_line"] = json!(6);
        record_answer(&store, &key, &run, "01", "ollama", &elsewhere.to_string()).unwrap();
        let rows = store.turn(&key, &run, "01").unwrap()["comparison"]["rows"].clone();
        assert_eq!(rows.as_array().unwrap().len(), 2);
        let matches = |row: &Value| row["cells"].as_array().unwrap().iter().map(|c| c["match"].as_str().unwrap().to_owned()).collect::<Vec<_>>();
        assert_eq!(matches(&rows[0]), ["equal", "equal", "equal", "equal", "absent"]);
        assert_eq!(matches(&rows[1]), ["absent", "absent", "absent", "absent", "equal"]);

        // precise_location: the whole function, from its declaration line, is not an operation.
        let mut whole = answer.clone();
        whole["claims"][0]["location"]["operation_start_line"] = json!(4);
        let issues = &record_answer(&store, &key, &run, "01", "manual", &whole.to_string()).unwrap()["check"]["report"]["issues"];
        assert_eq!(issues, &json!([{"claim_id": "C1", "code": "location_operation_includes_declaration"}]));
        assert_eq!(query["precise_location"], true);

        // Prose questions keep the Graph Engine query and are not checked.
        settings.answer_format = AnswerFormat::Prose;
        store.save_settings(&key, &settings).unwrap();
        let prose = store.ask(&key, &run, "direção dos raios").unwrap();
        assert_eq!(prose["answer_format"], "prose");
        assert!(serde_json::from_str::<Value>(prose["query"].as_str().unwrap()).unwrap()["evidence"]["source"].is_object());
        assert!(record_answer(&store, &key, &run, "02", "manual", &answer.to_string()).is_err());
    }

    #[test]
    fn approved_facts_pass_every_gate_and_are_rechecked_against_later_editions() {
        let scratch = project();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let opened = store.open(&display(&scratch.0.join("project"))).unwrap();
        let key = opened["key"].as_str().unwrap().to_owned();
        let mut settings: Settings = serde_json::from_value(opened["settings"].clone()).unwrap();
        settings.lexicon.groups = vec![vec!["raios".into(), "ray".into()], vec!["direcao".into(), "direction".into()]];
        store.save_settings(&key, &settings).unwrap();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        let answer = json!({"schema_version": 1, "claims": [{
            "id": "C1", "kind": "FACT", "text": "`ray_from_pixel` calcula `direction` na linha 5 de src/lib.rs.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 4, "end_line": 5,
                "quote": "    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;"}],
            "premises": [], "verification_plan": null,
            "location": {"evidence_id": "E1", "path": "src/lib.rs", "symbol_name": "ray_from_pixel",
                "declaration_line": 4, "operation_start_line": 5, "operation_end_line": 5}}]});
        let mut wrong = answer.clone();
        wrong["claims"][0]["citations"][0]["quote"] = json!("let direction = x * 2.0;");
        record_answer(&store, &key, &run, "01", "manual", &answer.to_string()).unwrap();
        record_answer(&store, &key, &run, "01", "manual", &wrong.to_string()).unwrap();
        let review = |send: &str, approved: bool, note: &str| store.review(&key, &run, "01", send, "C1", approved, note);
        let refusal = |result: Result<Value>| result.err().unwrap().to_string();

        // Gate 1 runs the check again; a person cannot approve an answer the checker refused.
        assert!(refusal(review("02-manual", true, "Parece certo.")).starts_with("Portão 1"));
        assert!(refusal(review("01-manual", true, "  ")).contains("nota"));
        assert!(refusal(store.review(&key, &run, "01", "01-manual", "C9", true, "x")).contains("C9"));

        let promoted = review("01-manual", true, "A linha 5 calcula direction a partir de x.").unwrap();
        assert_eq!(promoted["fact_id"], "F1");
        assert!(refusal(review("01-manual", true, "De novo.")).contains("já foi promovido como F1"));
        let facts = store.facts(&key, &run).unwrap();
        let fact = &facts["facts"][0];
        assert_eq!((fact["id"].as_str(), fact["status"].as_str()), (Some("F1"), Some("current")));
        assert_eq!((fact["path"].as_str(), fact["line"].as_u64(), fact["end_line"].as_u64()), (Some("src/lib.rs"), Some(5), Some(5)));
        assert_eq!(fact["authorship"], crate::facts::AUTHORSHIP);
        assert_eq!(fact["edition"], facts["edition"]);
        assert_eq!(fact["link"]["query_file"], "claims/query.json");
        let turn = store.turn(&key, &run, "01").unwrap();
        assert_eq!(turn["sends"][0]["id"], "01-manual");
        assert_eq!(turn["sends"][0]["reviews"]["C1"]["decision"], "approved");

        // Gate 2: a send without the link, or whose preserved query was edited, is not promoted.
        let third = record_answer(&store, &key, &run, "01", "manual", &answer.to_string()).unwrap();
        let third = Path::new(third["directory"].as_str().unwrap()).to_path_buf();
        let summary = third.join("send.json");
        let original = fs::read(&summary).unwrap();
        let mut unlinked: Value = serde_json::from_slice(&original).unwrap();
        unlinked["link"] = Value::Null;
        fs::write(&summary, serde_json::to_vec(&unlinked).unwrap()).unwrap();
        assert!(refusal(review("03-manual", true, "Igual ao F1.")).contains("sem vínculo"));
        fs::write(&summary, &original).unwrap();
        fs::write(third.join("query.json"), b"{}\n").unwrap();
        assert!(refusal(review("03-manual", true, "Igual ao F1.")).contains("query preservada"));
        // An answer edited after its check is refused before any gate.
        fs::write(third.join("response.txt"), wrong.to_string()).unwrap();
        assert!(refusal(review("03-manual", false, "x")).contains("difere da que foi conferida"));

        // Rejecting revokes the promotion; approving again makes a new fact, never reuses F1.
        assert_eq!(review("01-manual", false, "A direção não é só x * 2.").unwrap()["revoked"], "F1");
        assert_eq!(store.facts(&key, &run).unwrap()["facts"][0]["status"], "revoked");
        assert_eq!(review("01-manual", true, "Revisto: a linha 5 é a operação.").unwrap()["fact_id"], "F2");

        // A later edition rechecks by text at the same lines: an edited line makes the fact stale.
        let source = scratch.0.join("project/src/lib.rs");
        fs::write(&source, fs::read_to_string(&source).unwrap().replace("x * 2.0", "x * 3.0")).unwrap();
        let later = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let rechecked = store.facts(&key, &later).unwrap();
        assert_ne!(rechecked["edition"], facts["edition"]);
        let statuses: Vec<_> = rechecked["facts"].as_array().unwrap().iter().map(|f| f["status"].as_str().unwrap().to_owned()).collect();
        assert_eq!(statuses, ["revoked", "stale"]);
        assert!(rechecked["facts"][1]["status_reason"].as_str().unwrap().contains("src/lib.rs:4–5"));
        assert_eq!(store.facts(&key, &run).unwrap()["facts"][1]["status"], "current");

        // The declaration the model left uncited is cited by the Librarian, and the fact says so.
        let mut short = answer.clone();
        short["claims"][0]["citations"][0] = json!({"evidence_id": "E1", "path": "src/lib.rs", "start_line": 5, "end_line": 5,
            "quote": "        let direction = x * 2.0;"});
        let sent = record_answer(&store, &key, &run, "01", "manual", &short.to_string()).unwrap();
        let check: Value = serde_json::from_slice(&fs::read(Path::new(sent["directory"].as_str().unwrap()).join("check.json")).unwrap()).unwrap();
        assert_eq!(check["report"]["mechanical_status"], "passed");
        assert_eq!(check["resolutions"][0]["code"], "declaration_citation_added");
        assert_eq!(check["claims"][0]["citations"].as_array().unwrap().len(), 2);
        let id = review("04-manual", true, "A linha 5 calcula direction.").unwrap()["fact_id"].clone();
        let facts = store.facts(&key, &run).unwrap();
        let fact = facts["facts"].as_array().unwrap().iter().find(|f| f["id"] == id).unwrap();
        assert_eq!(fact["resolutions"][0]["code"], "declaration_citation_added");
        assert_eq!(fact["citations"].as_array().unwrap().len(), 2);
        assert_eq!(fact["status"], "current");
        // Continuing in chain reads an answer as the check did, with the completed citations.
        let next = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap()["id"].as_str().unwrap().to_owned();
        record_answer(&store, &key, &run, &next, "manual", &short.to_string()).unwrap();
        assert!(store.continue_turn(&key, &run, &next, "E o que vem depois?").is_ok());
    }

    #[test]
    fn chain_steps_carry_checked_locations_and_current_reviewed_facts_never_the_text() {
        let scratch = project();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let opened = store.open(&display(&scratch.0.join("project"))).unwrap();
        let key = opened["key"].as_str().unwrap().to_owned();
        let mut settings: Settings = serde_json::from_value(opened["settings"].clone()).unwrap();
        settings.lexicon.groups = vec![vec!["raios".into(), "ray".into()], vec!["direcao".into(), "direction".into()]];
        store.save_settings(&key, &settings).unwrap();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        // Nothing passed yet: there is nothing to continue from.
        assert!(store.continue_turn(&key, &run, "01", "E agora?").err().unwrap().to_string().contains("nada para continuar"));
        let answer = json!({"schema_version": 1, "claims": [{
            "id": "C1", "kind": "FACT", "text": "`ray_from_pixel` calcula `direction` na linha 5 de src/lib.rs.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 4, "end_line": 5,
                "quote": "    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;"}],
            "premises": [], "verification_plan": null,
            "location": {"evidence_id": "E1", "path": "src/lib.rs", "symbol_name": "ray_from_pixel",
                "declaration_line": 4, "operation_start_line": 5, "operation_end_line": 5}}]});
        let mut wrong = answer.clone();
        wrong["claims"][0]["citations"][0]["quote"] = json!("let direction = x * 2.0;");
        record_answer(&store, &key, &run, "01", "manual", &wrong.to_string()).unwrap();
        record_answer(&store, &key, &run, "01", "manual", &answer.to_string()).unwrap();
        store.review(&key, &run, "01", "02-manual", "C1", true, "A linha 5 calcula direction.").unwrap();

        let step = store.continue_turn(&key, &run, "01", "Como evitar a multiplicação em ray_from_pixel?").unwrap();
        assert_eq!((step["id"].as_str(), &step["prepare_error"], &step["claims_error"]), (Some("02"), &Value::Null, &Value::Null));
        let chain = &step["chain"];
        assert_eq!(chain["from_turn"], "01");
        // Only the answer that passes the check again is used, linked by hash, without its text.
        assert_eq!(chain["answers"], json!([{"send": "02-manual", "claims": ["C1"],
            "response_sha256": hash(answer.to_string().as_bytes())}]));
        assert_eq!(chain["targets"], json!([{"path": "src/lib.rs", "symbol_name": "ray_from_pixel", "declaration_line": 4}]));
        assert_eq!(chain["reviewed_facts"], json!(["F1"]));
        assert!(step["search"]["hits"][0]["name"].as_str().unwrap().contains("ray_from_pixel"));
        // G6d: the edition is v4, so the method also brings its owner type (a unit struct here).
        assert_eq!(chain["owner_context"], true);
        assert_eq!(step["search"]["hits"][1]["name"], "Camera");
        assert_eq!(step["search"]["owner_context"]["added"][0]["reason"], "owner_type");
        // The approved text travels only as the reviewed fact F1.
        let query: Value = serde_json::from_str(step["query"].as_str().unwrap()).unwrap();
        let reviewed = &query["evidence"]["reviewed_facts"][0];
        assert_eq!((reviewed["id"].as_str(), reviewed["start_line"].as_u64()), (Some("F1"), Some(5)));
        assert_eq!(reviewed["authorship"], crate::facts::AUTHORSHIP);
        assert!(query["instructions"].as_array().unwrap().iter().any(|i| i.as_str().unwrap().starts_with("evidence.reviewed_facts")));
        // The step is a question like any other: it verifies before a send and links its query.
        let verified = store.verify_for_send(&key, &run, "02").unwrap();
        assert_eq!(verified.link["query_file"], "claims/query.json");
        assert!(verified.link["dossier_origin"].as_str().unwrap().starts_with("edition:"));

        // A revoked fact is no longer carried, and with it the text; the location still is.
        store.review(&key, &run, "01", "02-manual", "C1", false, "Revisto.").unwrap();
        let again = store.continue_turn(&key, &run, "01", "De novo?").unwrap();
        assert_eq!((again["chain"]["reviewed_facts"].clone(), again["prepare_error"].clone()), (json!([]), Value::Null));
        assert!(!again["query"].as_str().unwrap().contains("calcula `direction` na linha 5"), "unreviewed claim text leaked");
        // Prose questions have no mechanical check to continue from.
        settings.answer_format = AnswerFormat::Prose;
        store.save_settings(&key, &settings).unwrap();
        store.ask(&key, &run, "direção dos raios").unwrap();
        assert!(store.continue_turn(&key, &run, "04", "E?").err().unwrap().to_string().contains("claims"));
    }

    #[test]
    fn allow_patch_is_fixed_with_the_question_and_checks_proposed_edits() {
        let scratch = project();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let opened = store.open(&display(&scratch.0.join("project"))).unwrap();
        let key = opened["key"].as_str().unwrap().to_owned();
        let mut settings: Settings = serde_json::from_value(opened["settings"].clone()).unwrap();
        settings.lexicon.groups = vec![vec!["raios".into(), "ray".into()], vec!["direcao".into(), "direction".into()]];
        (settings.allow_hypothesis, settings.allow_patch) = (true, true);
        store.save_settings(&key, &settings).unwrap();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let turn = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        let query: Value = serde_json::from_str(turn["query"].as_str().unwrap()).unwrap();
        // New questions ask for line edits; the diff is derived by the checker.
        assert_eq!((&query["allow_edits"], query.get("allow_patch")), (&json!(true), None));
        let answer = json!({"schema_version": 1, "claims": [{
            "id": "H1", "kind": "HYPOTHESIS", "text": "Devolver a expressão direto dispensa a variável direction.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 5, "end_line": 6,
                "quote": "        let direction = x * 2.0;\n        direction"}],
            "premises": [], "verification_plan": "Medir ray_from_pixel antes e depois com um benchmark.", "location": null,
            "edits": [{"path": "src/lib.rs", "op": "replace", "start_line": 5, "end_line": 6, "new_lines": ["        x * 2.0"]}]}]});
        let check = &record_answer(&store, &key, &run, "01", "manual", &answer.to_string()).unwrap()["check"];
        assert_eq!(check["report"]["mechanical_status"], "passed");
        assert_eq!(check["diffs"]["H1"], "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -4,4 +4,3 @@\n     pub fn ray_from_pixel(&self, x: f64) -> f64 {\n-        let direction = x * 2.0;\n-        direction\n+        x * 2.0\n     }\n");
        // The exported file is one `git apply` accepts against the project, when git is here.
        let send = store.turn_dir(&key, &run, "01").unwrap().join("sends");
        let file = fs::read_dir(&send).unwrap().filter_map(|e| e.ok()).map(|e| e.path().join("patch-H1.diff")).find(|p| p.exists()).unwrap();
        if let Ok(out) = std::process::Command::new("git").arg("apply").arg("--check").arg(&file).current_dir(scratch.0.join("project")).output() {
            assert!(out.status.success(), "git apply --check: {}", String::from_utf8_lossy(&out.stderr));
        }
        // The choice is recorded with the question: editing it afterwards is refused.
        let context = store.turn_dir(&key, &run, "01").unwrap().join("claims/context.json");
        let original = fs::read_to_string(&context).unwrap();
        assert!(original.contains("\"allow_edits\": true"));
        fs::write(&context, original.replace("\"allow_edits\": true", "\"allow_edits\": false")).unwrap();
        assert!(store.verify_for_send(&key, &run, "01").is_err());
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
                "search": {"top": 3, "depth": 1, "hits": 1, "exclude_tests": true}, "prepare": {"radius": 1, "numbered_lines": true},
                "answer": {"format": "prose", "allow_hypothesis": true}}"#,
        )
        .unwrap();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let opened = store.open(&display(&root)).unwrap();
        let settings: Settings = serde_json::from_value(opened["settings"].clone()).unwrap();
        assert_eq!(settings.inputs, ["src"]);
        assert_eq!((settings.top, settings.depth, settings.radius, settings.exclude_tests), (3, 1, 1, true));
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
        // The link carries the criteria hash; criteria edited after the question block the send.
        let run = run["id"].as_str().unwrap();
        let link = store.verify_for_send(key, run, "01").unwrap().link;
        assert_eq!(link["criteria_sha256"], turn["criteria_sha256"]);
        assert_eq!(link["checks"], json!(["prepare::verify", "criteria_sha256"]));
        let criteria = store.turn_dir(key, run, "01").unwrap().join(CRITERIA);
        fs::write(&criteria, "- Qualquer resposta serve.\n").unwrap();
        assert!(store.verify_for_send(key, run, "01").err().unwrap().to_string().contains("Critérios"));
    }

    #[test]
    fn path_questions_ask_one_picked_declaration_at_a_time() {
        let scratch = project();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&scratch.0.join("project"))).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        let id = |name: &str| catalog.symbols().values().find(|s| s.name == name).unwrap().id.clone();
        let nodes = [id("ray_from_pixel"), id("Camera")];
        let chosen = json!({ "occ:x": "symbol:y" });
        let turns = store.ask_path(&key, &run, "Onde este trecho calcula algo?", &nodes, &nodes[0], &chosen).unwrap();
        let turns = turns.as_array().unwrap();
        assert_eq!(turns.len(), 2);
        // Each question carries one declaration, says it was picked on a path, and records where.
        for (i, turn) in turns.iter().enumerate() {
            assert_eq!(turn["prepare_error"], Value::Null, "{}", turn["prepare_error"]);
            assert_eq!(turn["search"]["status"], "path");
            assert!(turn["search"]["limits"][0].as_str().unwrap().starts_with("Path step"));
            assert_eq!(turn["search"]["hits"].as_array().unwrap().len(), 1);
            assert_eq!(turn["search"]["hits"][0]["symbol_id"], json!(nodes[i]));
            assert_eq!((turn["path"]["index"].as_u64(), turn["path"]["total"].as_u64()), (Some(i as u64 + 1), Some(2)));
            assert_eq!(turn["path"]["chosen"], chosen);
            assert_eq!(turn["chain"], Value::Null);
            assert_eq!(turn["request"]["picked"], true);
        }
        assert!(store.ask_path(&key, &run, "x", &[], &nodes[0], &chosen).is_err());
        assert!(store.ask_path(&key, &run, "x", &["symbol:nada".into()], &nodes[0], &chosen).is_err());
    }

    #[test]
    fn lexicon_proposals_are_checked_and_profiles_exported_on_request() {
        let scratch = project();
        let root = scratch.0.join("project");
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        // A question whose words "fica" and "hoje" touch no result: candidates for stop words.
        store.ask(&key, &run, "Onde fica o ray_from_pixel da Camera hoje?").unwrap();

        // Manual: the request is written and returned; nothing is sent.
        let manual = store.suggest_lexicon(&key, &run, "manual", "pt", crate::llm::Choice {
            modelfile: None, modelfile_name: None, claude_model: None }).unwrap();
        let query: Value = serde_json::from_str(manual["query"].as_str().unwrap()).unwrap();
        let terms: Vec<_> = query["evidence"]["code_terms"].as_array().unwrap().iter().map(|t| t["term"].clone()).collect();
        assert!(terms.contains(&json!("camera")) && terms.contains(&json!("pixel")));
        assert_eq!(query["evidence"]["missed_terms"], json!([{ "term": "fica", "questions": 1 }, { "term": "hoje", "questions": 1 }]));
        // In English the same code terms go with a request for English words; others are refused.
        let none = || crate::llm::Choice { modelfile: None, modelfile_name: None, claude_model: None };
        let english = store.suggest_lexicon(&key, &run, "manual", "en", none()).unwrap();
        let english: Value = serde_json::from_str(english["query"].as_str().unwrap()).unwrap();
        assert!(english["question"].as_str().unwrap().contains("English words"));
        assert_eq!(english["evidence"]["code_terms"], query["evidence"]["code_terms"]);
        assert!(store.suggest_lexicon(&key, &run, "manual", "fr", none()).is_err());

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
        assert_eq!(proposal["stop_report"], json!([]));

        // Stop words never hide code: names and paths, group words and existing stops are refused;
        // a word only inside code bodies passes with a note. A cut-off list is not read as a group.
        let stops = "{\"groups\": [[\"raio\", \"ray\"], [\"para\", \"for\", \"camera\"]], \"stop_words\": [\"fica\", \"camera\", \"let\", \"raio\", \"duas palavras\", \"onde\"]}";
        let proposal = store.check_lexicon(&key, &run, manual["id"].as_str().unwrap(), stops).unwrap()["proposal"].clone();
        let verdict: Vec<_> = proposal["stop_report"].as_array().unwrap().iter()
            .map(|r| (r["word"].as_str().unwrap().to_owned(), r["status"].as_str().unwrap().to_owned())).collect();
        let pair = |w: &str, s: &str| (w.to_owned(), s.to_owned());
        assert_eq!(verdict, [pair("fica", "accepted"), pair("camera", "rejected"), pair("let", "accepted"),
            pair("raio", "rejected"), pair("duas palavras", "rejected"), pair("onde", "rejected")]);
        assert_eq!(proposal["stop_report"][2]["notes"], json!(["\"let\" aparece no corpo do código"]));
        // A stop word inside a group is dropped from it: "para" never reaches the search.
        assert_eq!(proposal["report"][1]["words"], json!(["for", "camera"]));
        assert_eq!(proposal["report"][1]["notes"], json!(["\"para\" é stop word"]));
        assert_eq!(proposal["lexicon"]["stop_words"], json!(["fica", "let"]));
        assert_eq!((proposal["stop_accepted"].as_u64(), proposal["stop_rejected"].as_u64()), (Some(2), Some(4)));
        let cut = "{\"groups\": [[\"raio\", \"ray\"]], \"stop_words\": [\"fica\", \"ho";
        let proposal = store.check_lexicon(&key, &run, manual["id"].as_str().unwrap(), cut).unwrap()["proposal"].clone();
        assert_eq!((proposal["accepted"].as_u64(), proposal["rejected"].as_u64()), (Some(1), Some(0)));
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
    fn profile_commands_run_only_approved_in_a_verified_copy() {
        let scratch = project();
        let root = scratch.0.join("project");
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root)
                .args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        // The command prints one measure line; nothing in the Librarian knows what it runs.
        let profile = json!({ "project_root": ".", "commands": { "probe": {
            "argv": ["git", "log", "-1", "--format=tformat:librarian-measure answer=42 units"], "repeat": 2 },
            "write": { "argv": ["git", "log", "-1", "--format=tformat:%s", "--output=made.txt"], "repeat": 2, "outputs": ["made.txt"] } } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        git(&["init", "-q"]);
        // The repository's own line-ending setting governs both the project and its copy.
        git(&["config", "core.autocrlf", "false"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let refusal = |r: Result<Value>| r.err().unwrap().to_string();

        let listed = store.commands(&key).unwrap();
        let sha = listed["commands"][0]["sha256"].as_str().unwrap().to_owned();
        assert_eq!(listed["commands"][0]["approved"], Value::Null);
        assert!(refusal(store.execute(&key, &run, "probe", None)).contains("sem aprovação"));
        assert!(refusal(store.approve_command(&key, "probe", "0")).contains("mudou"));
        assert!(refusal(store.approve_command(&key, "other", &sha)).contains("não declarado"));
        store.approve_command(&key, "probe", &sha).unwrap();

        let done = store.execute(&key, &run, "probe", None).unwrap();
        assert_eq!(done["summary"]["succeeded"], 2, "{:#}", done["runs"]);
        assert_eq!(done["summary"]["stats"]["answer"]["median"], 42.0);
        assert_eq!(done["summary"]["stats"]["answer"]["unit"], "units");
        assert!(done["sources_verified"].as_u64().unwrap() > 0);
        assert!(Path::new(done["copy"].as_str().unwrap()).join("src/lib.rs").is_file());
        // The copy lives outside the studio data, so no tool takes it for part of another project.
        assert!(!done["copy"].as_str().unwrap().starts_with(&display(&store.dir)));
        assert_eq!(store.executions(&key, &run).unwrap().len(), 1);
        // A declared output is hashed after each repetition; the same bytes give one hash.
        let sha = store.commands(&key).unwrap()["commands"][1]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "write", &sha).unwrap();
        let wrote = store.execute(&key, &run, "write", None).unwrap();
        assert_eq!(wrote["summary"]["outputs"]["made.txt"].as_array().unwrap().len(), 1);
        assert_eq!(wrote["runs"][0]["outputs"]["made.txt"], librarian_ingest::hash(b"x\n"));

        // A change an answer proposes runs in its own copy, compared with the latest baseline.
        let mut settings = store.settings(&key).unwrap();
        settings.lexicon.groups = vec![vec!["raios".into(), "ray".into()], vec!["direcao".into(), "direction".into()]];
        (settings.allow_hypothesis, settings.allow_patch) = (true, true);
        store.save_settings(&key, &settings).unwrap();
        let turn = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap()["id"].as_str().unwrap().to_owned();
        let answer = json!({"schema_version": 1, "claims": [{
            "id": "H1", "kind": "HYPOTHESIS", "text": "Triplicar no lugar de dobrar.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 5, "end_line": 5, "quote": "        let direction = x * 2.0;"}],
            "premises": [], "verification_plan": "Medir antes e depois.", "location": null,
            "edits": [{"path": "src/lib.rs", "op": "replace", "start_line": 5, "end_line": 5, "new_lines": ["        let direction = x * 3.0;"]}]}]});
        let sent = record_answer(&store, &key, &run, &turn, "manual", &answer.to_string()).unwrap();
        let send = Path::new(sent["directory"].as_str().unwrap()).file_name().unwrap().to_string_lossy().into_owned();
        let change = crate::execution::ChangeRef { turn: turn.clone(), send: send.clone(), claim: "H1".into() };
        let changed = store.execute(&key, &run, "probe", Some(&change)).unwrap();
        assert_eq!(changed["change"]["files"], json!(["src/lib.rs"]));
        assert_eq!(changed["comparison"]["baseline"], store.executions(&key, &run).unwrap()[0]["id"]);
        assert_eq!(changed["comparison"]["stats"]["answer"]["change_pct"], 0.0);
        assert_eq!(changed["comparison"]["stats"]["answer"]["ranges_overlap"], true);
        let changed_copy = Path::new(changed["copy"].as_str().unwrap()).to_path_buf();
        assert!(fs::read_to_string(changed_copy.join("src/lib.rs")).unwrap().contains("x * 3.0"));
        assert!(!fs::read_to_string(Path::new(done["copy"].as_str().unwrap()).join("src/lib.rs")).unwrap().contains("x * 3.0"));
        // The same change again reuses its copy; a claim with no derived change is refused.
        store.execute(&key, &run, "probe", Some(&change)).unwrap();
        let none = crate::execution::ChangeRef { turn, send, claim: "C9".into() };
        assert!(refusal(store.execute(&key, &run, "probe", Some(&none))).contains("não tem mudança derivada"));

        // A source that differs only in line endings is the same text, recorded as such.
        let lib = root.join("src/lib.rs");
        fs::write(&lib, fs::read_to_string(&lib).unwrap().replace('\n', "\r\n")).unwrap();
        let crlf = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let same = store.execute(&key, &crlf, "probe", None).unwrap();
        assert_eq!(same["sources_line_endings_only"], json!(["src/lib.rs"]));
        git(&["checkout", "--", "src/lib.rs"]);

        // Editing the command in the profile voids the approval.
        let mut edited = profile.clone();
        edited["commands"]["probe"]["repeat"] = json!(3);
        fs::write(root.join(PROFILE), edited.to_string()).unwrap();
        assert!(refusal(store.execute(&key, &run, "probe", None)).contains("sem aprovação"));
        // An edition read from uncommitted changes has no faithful copy.
        fs::write(root.join("src/lib.rs"), "pub struct Camera;\n").unwrap();
        let dirty = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "probe", &sha).unwrap();
        assert!(refusal(store.execute(&key, &dirty, "probe", None)).contains("difere da edition"));
        // Outputs stay inside the copy.
        let mut outside = edited.clone();
        outside["commands"]["write"]["outputs"] = json!(["../x.txt"]);
        fs::write(root.join(PROFILE), outside.to_string()).unwrap();
        assert!(refusal(store.commands(&key)).contains("fora da cópia"));
        let _ = fs::remove_dir_all(Path::new(done["copy"].as_str().unwrap()).parent().unwrap());
    }

    #[test]
    fn stamp_is_sortable_utc() {
        let s = stamp();
        assert_eq!(s.len(), 15);
        assert_eq!(&s[8..9], "-");
        assert!(s.starts_with("20"));
    }
}
