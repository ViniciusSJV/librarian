//! Projects, runs and questions under `.studio/`. Nothing is written inside analyzed projects.
//! Layout: projects/<key>/project.json, runs/<run>/{edition, graph-v2.json, run.json},
//! runs/<run>/turns/<NN>/{question.json, search.json, prepare-request.json, prepared/, claims/, sends/}.
use librarian_llm_engine::claims::{self, compare::compare, Claim, Policy};
use librarian_ingest::{
    envelope, inventory,
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
/// Declarations one gathered question may carry; each one is a whole excerpt in the query.
const MAX_GATHERED: usize = 16;

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
    /// The language of the claims contract: "en" for English; empty or "pt" for Portuguese.
    #[serde(default)]
    pub language: String,
    /// Each excerpt once in the query of a new question, in `evidence.line_map`. On by default,
    /// also for projects saved before it: a recorded question keeps what it was asked with.
    #[serde(default = "compact_by_default")]
    pub compact_items: bool,
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
            "gathered": question["gathered"],
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
            let target = target_of(&catalog, symbols, id)?;
            let path = json!({ "seed": seed, "node": id, "index": index + 1, "total": nodes.len(), "chosen": chosen });
            let mut request = request(&settings, question, Some(vec![target]), vec![], false);
            request.picked = true;
            turns.push(self.new_turn(key, run, &settings, &catalog, question, None, request, Some(("path", path)))?);
        }
        Ok(json!(turns))
    }

    /// P4: one question whose evidence is the declarations the person gathered one by one (in
    /// the order gathered), with nothing searched and nothing added; the step says so. P5: the
    /// envelope may hold file heads (`header:<path>`), and, with `with_facts`, the question also
    /// carries the current reviewed facts about the gathered declarations.
    /// An investigation from the inventory records its `origin` (the finding it came from).
    pub fn ask_gathered(&self, key: &str, run: &str, question: &str, nodes: &[String], with_facts: bool, origin: Option<Value>) -> Result<Value> {
        if nodes.is_empty() || nodes.len() > MAX_GATHERED {
            return Err(format!("Junte de 1 a {MAX_GATHERED} declarações").into());
        }
        let settings = self.settings(key)?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let headers: Vec<String> = nodes.iter().filter_map(|id| id.strip_prefix(envelope::HEADER_PREFIX).map(str::to_owned)).collect();
        let targets = nodes
            .iter()
            .filter(|id| !id.starts_with(envelope::HEADER_PREFIX))
            .map(|id| target_of(&catalog, catalog.symbols(), id))
            .collect::<Result<Vec<_>>>()?;
        if targets.is_empty() {
            return Err("Junte ao menos uma declaração além dos topos de arquivo".into());
        }
        let reviewed = if with_facts { self.reviewed_about(key, run, &targets)? } else { vec![] };
        // What the selection found goes with the evidence, for the declarations still in the envelope.
        let notes: Vec<String> = origin.as_ref().and_then(|o| serde_json::from_value::<Vec<envelope::Note>>(o["selection"]["notes"].clone()).ok()).unwrap_or_default()
            .into_iter().filter(|n| n.id.is_empty() || nodes.contains(&n.id)).map(|n| n.text).collect();
        let mut gathered = json!({ "nodes": nodes, "with_facts": with_facts, "reviewed_facts": reviewed.iter().map(|f| &f.id).collect::<Vec<_>>() });
        if let Some(origin) = origin {
            gathered["origin"] = origin;
        }
        let mut request = request(&settings, question, Some(targets), reviewed, false);
        request.gathered = true;
        request.headers = headers;
        request.notes = notes;
        self.new_turn(key, run, &settings, &catalog, question, None, request, Some(("gathered", gathered)))
    }

    /// What a "Medir tudo" says about each function.
    fn measure_of(hot: &Value) -> Option<envelope::Measure> {
        let rows = hot["rows"].as_array()?;
        let mut measure = envelope::Measure::default();
        measure.counted.extend(hot["counted"].as_array().into_iter().flatten().filter_map(|s| s.as_str().map(String::from)));
        for row in rows {
            let (Some(symbol), Some(calls)) = (row["symbol"].as_str(), row["calls"].as_f64()) else { continue };
            measure.counted.insert(symbol.to_owned());
            let most = measure.calls.entry(symbol.to_owned()).or_insert(0.);
            *most = most.max(calls);
        }
        Some(measure)
    }

    /// P5: the envelope suggested for the chosen declarations: what was selected, each with its
    /// role and why, what stayed out and why, and the current reviewed facts about each one.
    pub fn suggest_envelope(&self, key: &str, run: &str, nodes: &[String]) -> Result<Value> {
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let hot = self.hotspots(key, run).ok();
        let measure = hot.as_ref().and_then(Self::measure_of);
        let mut candidates = envelope::candidates(&catalog, nodes, measure.as_ref())?;
        let (found, _, _) = self.inventory_of(key, run)?;
        envelope::trait_impls(&catalog, &found, nodes, measure.as_ref(), &mut candidates);
        let mut selection = envelope::select(&catalog, &candidates, measure.as_ref(), envelope::BUDGET_BYTES, MAX_GATHERED);
        selection.behavior = envelope::behavior(&catalog, &found, &selection);
        selection.notes = envelope::notes(&selection);
        let facts = self.facts(key, run)?;
        let current: Vec<&Value> = facts["facts"].as_array().into_iter().flatten().filter(|f| f["status"] == "current").collect();
        let mut out = serde_json::to_value(&selection)?;
        for list in ["items", "left_out"] {
            for value in out[list].as_array_mut().into_iter().flatten() {
                let id = value["id"].as_str().unwrap_or_default().to_owned();
                let line = (!id.starts_with(envelope::HEADER_PREFIX)).then(|| target_of(&catalog, catalog.symbols(), &id)).transpose()?;
                let ids: Vec<&Value> = current
                    .iter()
                    .filter(|f| line.as_ref().is_some_and(|t| f["path"] == t.path && f["symbol_name"] == t.symbol_name && f["declaration_line"] == t.declaration_line))
                    .map(|f| &f["id"])
                    .collect();
                value["facts"] = json!(ids);
            }
        }
        crate::instrument::annotate_suggestion(&mut out, hot.as_ref().map(|h| &h["coverage"]));
        Ok(out)
    }

    /// Steps 2 and 3: the operation inventory of a run's edition, with the built-in rules and
    /// those of the profile's `inventory`, read from the file at each use like its commands.
    pub fn inventory(&self, key: &str, run: &str) -> Result<Value> {
        let (found, profile, _) = self.inventory_of(key, run)?;
        Ok(json!({ "profile": display(&profile), "inventory": found }))
    }

    pub(crate) fn inventory_of(&self, key: &str, run: &str) -> Result<(inventory::Inventory, PathBuf, Catalog)> {
        let root = PathBuf::from(self.settings(key)?.root);
        let profile = root.join(PROFILE);
        let rules: inventory::RuleSet = match fs::read(&profile).ok().map(|b| serde_json::from_slice::<Value>(&b)).transpose()? {
            Some(p) if p.get("inventory").is_some() => serde_json::from_value(p["inventory"].clone()).map_err(|e| format!("{PROFILE}: inventory inválido: {e}"))?,
            _ => inventory::RuleSet::default(),
        };
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let found = inventory::inventory(&catalog, &rules)?;
        Ok((found, profile, catalog))
    }

    /// An investigation from the inventory: for a loop (its occurrence id) or an operation
    /// (`op:...`), the declarations the evidence should hold, each with why, in the order a
    /// reader follows them: the loop's declaration, each step of each chain, the field a
    /// relationship rule matched, the declarations of loops reached. The question itself is
    /// written by the page; nothing is asked here.
    pub fn investigate(&self, key: &str, run: &str, target: &str) -> Result<Value> {
        let (found, _, catalog) = self.inventory_of(key, run)?;
        let ops: std::collections::BTreeMap<&str, &inventory::Operation> = found.operations.iter().map(|o| (o.id.as_str(), o)).collect();
        let mut items: Vec<(String, Vec<Value>)> = vec![];
        let mut add = |id: &str, reason: Value| match items.iter_mut().find(|(i, _)| i == id) {
            Some((_, reasons)) => reasons.push(reason),
            None => items.push((id.to_owned(), vec![reason])),
        };
        let (kind, finding) = if let Some(op) = ops.get(target) {
            add(&op.symbol, json!({ "code": "operation", "rule": op.rule, "line": op.line, "text": op.text }));
            for field in &op.through {
                add(field, json!({ "code": "matched", "rule": op.rule, "line": op.line }));
            }
            ("operation", serde_json::to_value(op)?)
        } else {
            let l = found.loops.iter().find(|l| l.occurrence == target).ok_or("Achado desconhecido: monte o inventário de novo")?;
            add(&l.symbol, json!({ "code": "loop", "line": l.line, "head": l.head }));
            for id in &l.direct {
                for field in &ops[id.as_str()].through {
                    add(field, json!({ "code": "matched", "rule": ops[id.as_str()].rule, "line": ops[id.as_str()].line }));
                }
            }
            let steps = l.through_calls.iter().map(|c| (&c.steps, Some(c.operation.as_str()))).chain(l.loops_through_calls.iter().map(|c| (&c.steps, None)));
            for (chain, operation) in steps {
                for s in chain.iter() {
                    let from = catalog.symbols().get(&s.from).map_or_else(|| s.from.clone(), |x| x.qualified_name.clone());
                    add(&s.to, json!({ "code": "step", "from": from, "line": s.line, "path": s.path }));
                }
                for field in operation.map(|o| &ops[o].through).into_iter().flatten() {
                    let op = ops[operation.expect("an operation")];
                    add(field, json!({ "code": "matched", "rule": op.rule, "line": op.line }));
                }
            }
            ("loop", serde_json::to_value(l)?)
        };
        // A reason repeated (the same step from two chains) is said once.
        let truncated = items.len() > MAX_GATHERED;
        let items: Vec<Value> = items
            .into_iter()
            .take(MAX_GATHERED)
            .map(|(id, mut reasons)| {
                reasons.dedup();
                let symbol = &catalog.symbols()[&id];
                let chunk = &catalog.chunks()[&symbol.chunk_id];
                json!({ "id": id, "name": symbol.qualified_name, "kind": symbol.kind, "path": catalog.sources()[&symbol.source_id].path,
                    "start_line": chunk.start_line, "end_line": chunk.end_line, "reasons": reasons })
            })
            .collect();
        let reached: Vec<&inventory::Operation> = match kind {
            "loop" => {
                let l = found.loops.iter().find(|l| l.occurrence == target).expect("found above");
                l.direct.iter().map(String::as_str).chain(l.through_calls.iter().map(|c| c.operation.as_str())).map(|id| ops[id]).collect()
            }
            _ => vec![ops[target]],
        };
        Ok(json!({ "target": target, "kind": kind, "finding": finding, "operations": reached, "items": items, "truncated": truncated }))
    }

    /// The current reviewed facts about these declarations, as a question carries them.
    fn reviewed_about(&self, key: &str, run: &str, targets: &[Target]) -> Result<Vec<ReviewedFact>> {
        let facts = self.facts(key, run)?;
        Ok(facts["facts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|f| f["status"] == "current")
            .filter(|f| targets.iter().any(|t| f["path"] == t.path && f["symbol_name"] == t.symbol_name && f["declaration_line"] == t.declaration_line))
            .map(|f| serde_json::from_value(json!({ "id": f["id"], "statement": f["statement"], "authorship": f["authorship"],
                "path": f["path"], "line": f["line"], "end_line": f["end_line"], "citations": f["citations"] })))
            .collect::<std::result::Result<_, _>>()?)
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
        let reviewed = self.reviewed_about(key, run, &targets)?;
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
                crate::claims::write(catalog, &dir, policy, instructions, settings.allow_patch, &settings.language, settings.compact_items).err().map(|e| e.to_string())
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
        // Commands and inventory rules are written by hand in the profile (commands approved
        // one by one); the studio never writes them, so an export keeps them exactly as they are.
        for kept in ["commands", "inventory", "instrument", "measures"] {
            if let Some(value) = existing.get(kept) {
                profile[kept] = value.clone();
            }
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

    /// Skills live next to the Modelfiles: skills/<name>/SKILL.md, with a front matter.
    fn skills_dir(&self) -> PathBuf {
        self.modelfiles.with_file_name("skills")
    }

    /// The skills a send can take, by name, with the description of their front matter.
    pub fn skills(&self) -> Result<Value> {
        let mut list = vec![];
        if self.skills_dir().is_dir() {
            for entry in fs::read_dir(self.skills_dir())? {
                let dir = entry?.path();
                let Ok(content) = fs::read_to_string(dir.join("SKILL.md")) else { continue };
                let (front, _) = split_front_matter(&content);
                // `description: >` folds the indented lines below it into one.
                let description = front.split_once("description:").map(|(_, rest)| rest.lines()
                    .enumerate().take_while(|(i, l)| *i == 0 || l.starts_with(' ')).map(|(_, l)| l.trim().trim_start_matches('>').trim())
                    .filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" "));
                list.push(json!({ "name": dir.file_name().unwrap().to_string_lossy(), "description": description }));
            }
        }
        list.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        Ok(json!(list))
    }

    /// A skill's text for the SYSTEM: SKILL.md without its front matter.
    pub fn skill(&self, name: &str) -> Result<crate::llm::Skill> {
        let name = safe_id(name)?;
        let content = fs::read_to_string(self.skills_dir().join(name).join("SKILL.md")).map_err(|_| format!("Skill desconhecida: {name}"))?;
        Ok(crate::llm::Skill { name: name.to_owned(), text: split_front_matter(&content).1.trim().to_owned() })
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

/// The target a graph node names: its file, name and the first line of its excerpt.
fn target_of(catalog: &Catalog, symbols: &std::collections::BTreeMap<String, librarian_ingest::Symbol>, id: &str) -> Result<Target> {
    let symbol = symbols.get(id).ok_or_else(|| format!("Ponto desconhecido: {id}"))?;
    Ok(Target {
        path: catalog.sources()[&symbol.source_id].path.clone(),
        symbol_name: symbol.name.clone(),
        declaration_line: catalog.chunks().get(&symbol.chunk_id).ok_or("Ponto sem trecho")?.start_line,
    })
}

/// The prepare request of a question with the project's settings; a chain step adds targets.
fn compact_by_default() -> bool {
    true
}

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
        gathered: false,
        headers: vec![],
        notes: vec![],
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
        language: String::new(),
        compact_items: true,
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

/// A Markdown file's front matter (between the opening `---` and the next) and the rest.
fn split_front_matter(content: &str) -> (&str, &str) {
    content.trim_start().strip_prefix("---").and_then(|rest| rest.split_once("\n---"))
        .map_or(("", content), |(front, body)| (front, body.split_once('\n').map_or("", |(_, b)| b)))
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
    fn measure_all_is_available_only_with_instrument_and_a_source_it_can_be_placed_in() {
        let scratch = project();
        let root = scratch.0.join("project");
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q"]);
        let commit = || { git(&["add", "-A"]); git(&["commit", "-q", "-m", "x", "--allow-empty"]); };
        let profile = |value: Value| fs::write(root.join(PROFILE), value.to_string()).unwrap();
        // A profile without the statement: not available, whatever the run.
        profile(json!({ "project_root": "." }));
        commit();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        assert_eq!(store.measure_all(&key, Some(&run)).unwrap()["reason"], "no_instrument");
        profile(json!({ "project_root": ".", "commands": { "c": { "argv": ["git", "status"] } }, "instrument": { "statement": "count();", "command": "c" } }));
        // Possible only with its command declared, and being approved is a separate answer.
        assert_eq!(store.measure_all(&key, Some(&run)).unwrap()["available"], true);
        profile(json!({ "project_root": ".", "instrument": { "statement": "count();", "command": "falta" } }));
        assert_eq!(store.measure_all(&key, Some(&run)).unwrap(), json!({ "available": false, "reason": "no_command", "command": "falta" }));
        profile(json!({ "project_root": ".", "commands": { "c": { "argv": ["git", "status"] } }, "instrument": { "statement": "count();", "command": "c" } }));
        assert_eq!(store.measure_all(&key, None).unwrap()["reason"], "no_run");
        assert_eq!(store.measure_all(&key, Some(&run)).unwrap(), json!({ "available": true, "approved": false, "command": "c" }));
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "c", &sha).unwrap();
        assert_eq!(store.measure_all(&key, Some(&run)).unwrap(), json!({ "available": true, "approved": true, "command": "c" }));
        // An edition that cannot be read: not measurable, and the commands still answer.
        let unknown = store.measure_all(&key, Some("nao-existe")).unwrap();
        assert_eq!((&unknown["available"], &unknown["reason"]), (&json!(false), &json!("no_edition")));
        assert_eq!(store.commands(&key).unwrap()["commands"].as_array().unwrap().len(), 1);
        // The statement declared, in a project with no source it can go in.
        fs::remove_file(root.join("src/lib.rs")).unwrap();
        fs::write(root.join("src/app.py"), "def demo():\n    return 1\n").unwrap();
        commit();
        let python = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        assert_eq!(store.measure_all(&key, Some(&python)).unwrap(), json!({ "available": false, "reason": "no_source", "command": "c" }));
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
        if let Ok(out) = std::process::Command::new("git").args(["apply", "--check", "--unidiff-zero"]).arg(&file).current_dir(scratch.0.join("project")).output() {
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

        // Gathered: one question with every declaration the person gathered, in that order.
        let gathered = store.ask_gathered(&key, &run, "Como guardar o valor?", &[nodes[1].clone(), nodes[0].clone()], false, None).unwrap();
        assert_eq!(gathered["prepare_error"], Value::Null, "{}", gathered["prepare_error"]);
        assert_eq!(gathered["search"]["status"], "gathered");
        assert!(gathered["search"]["limits"][0].as_str().unwrap().starts_with("Gathered step"));
        let hits: Vec<&Value> = gathered["search"]["hits"].as_array().unwrap().iter().map(|h| &h["symbol_id"]).collect();
        assert_eq!(hits, [&json!(nodes[1]), &json!(nodes[0])]);
        assert_eq!(gathered["gathered"]["nodes"], json!([nodes[1], nodes[0]]));
        assert_eq!(gathered["request"]["gathered"], true);
        assert_eq!(gathered["request"].get("picked"), None);
        assert!(store.ask_gathered(&key, &run, "x", &[], false, None).is_err());

        // P5: the suggested envelope lists each declaration with why, and the facts about it.
        let suggested = store.suggest_envelope(&key, &run, &[nodes[0].clone()]).unwrap();
        assert_eq!(suggested["items"][0]["id"], json!(nodes[0]));
        assert_eq!(suggested["items"][0]["reasons"][0]["code"], "chosen");
        assert!(suggested["items"].as_array().unwrap().iter().all(|i| i["facts"].is_array()));
        // A file head needs a declaration beside it, and lines before the file's first declaration.
        assert!(store.ask_gathered(&key, &run, "x", &["header:src/lib.rs".into()], false, None).is_err());
        let headless = store.ask_gathered(&key, &run, "x", &[nodes[0].clone(), "header:src/lib.rs".into()], true, None).unwrap();
        assert!(headless["prepare_error"].as_str().unwrap().contains("no lines before"));
        assert_eq!(headless["gathered"]["with_facts"], true);
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
            modelfile: None, modelfile_name: None, claude_model: None, skills: vec![] }).unwrap();
        let query: Value = serde_json::from_str(manual["query"].as_str().unwrap()).unwrap();
        let terms: Vec<_> = query["evidence"]["code_terms"].as_array().unwrap().iter().map(|t| t["term"].clone()).collect();
        assert!(terms.contains(&json!("camera")) && terms.contains(&json!("pixel")));
        assert_eq!(query["evidence"]["missed_terms"], json!([{ "term": "fica", "questions": 1 }, { "term": "hoje", "questions": 1 }]));
        // In English the same code terms go with a request for English words; others are refused.
        let none = || crate::llm::Choice { modelfile: None, modelfile_name: None, claude_model: None, skills: vec![] };
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
    fn the_inventory_reads_the_profile_rules_at_each_use() {
        let scratch = project();
        let root = scratch.0.join("project");
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let sources = |v: &Value| -> Vec<String> { v["inventory"]["rules"].as_array().unwrap().iter().map(|r| r["source"].as_str().unwrap().to_owned()).collect() };

        // Without `inventory` in the profile, only the built-in rules apply.
        let plain = store.inventory(&key, &run).unwrap();
        assert!(sources(&plain).iter().all(|s| s == "builtin"));
        assert_eq!(plain["inventory"]["version"], "operation-inventory-v1");
        // The profile is read again at each use: a rule written now applies now.
        fs::write(root.join(PROFILE), json!({ "inventory": { "rules": [
            { "id": "project.double", "operation": "math", "match": "call", "names": ["direction"] }] } }).to_string()).unwrap();
        assert!(sources(&store.inventory(&key, &run).unwrap()).contains(&"project".to_owned()));
        // A rule the inventory cannot read is refused with the profile's name.
        fs::write(root.join(PROFILE), json!({ "inventory": { "rules": [{ "id": "x", "operation": "x", "match": "guess" }] } }).to_string()).unwrap();
        assert!(store.inventory(&key, &run).unwrap_err().to_string().contains("inventory inválido"));
    }

    #[test]
    fn an_investigation_from_the_inventory_proposes_the_evidence_and_records_its_origin() {
        let scratch = project();
        let root = scratch.0.join("project");
        fs::write(root.join("src/scan.rs"), "fn norm(v: f64) -> f64 {\n    v.sqrt()\n}\n\npub fn scan(xs: &[f64]) -> f64 {\n    let mut t = 0.0;\n    for x in xs {\n        t += norm(*x);\n    }\n    t\n}\n").unwrap();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let found = store.inventory(&key, &run).unwrap();
        let looped = found["inventory"]["loops"].as_array().unwrap().iter().find(|l| l["symbol_name"] == "scan").unwrap().clone();

        // The loop's declaration first, then each step of the chain, each with why.
        let proposed = store.investigate(&key, &run, looped["occurrence"].as_str().unwrap()).unwrap();
        assert_eq!(proposed["kind"], "loop");
        let items = proposed["items"].as_array().unwrap();
        assert_eq!(items.iter().map(|i| i["name"].as_str().unwrap()).collect::<Vec<_>>(), ["scan", "norm"]);
        assert_eq!(items[0]["reasons"][0]["code"], "loop");
        assert_eq!((items[1]["reasons"][0]["code"].as_str(), items[1]["reasons"][0]["from"].as_str()), (Some("step"), Some("scan")));
        assert_eq!(proposed["operations"][0]["text"], "v.sqrt()");
        // An operation alone brings its own declaration.
        let op = proposed["operations"][0]["id"].as_str().unwrap();
        assert_eq!(store.investigate(&key, &run, op).unwrap()["items"][0]["name"], "norm");
        assert!(store.investigate(&key, &run, "nada").is_err());

        // The question over that evidence records where it came from.
        let nodes: Vec<String> = items.iter().map(|i| i["id"].as_str().unwrap().to_owned()).collect();
        let origin = json!({ "inventory": looped["occurrence"], "summary": "laço de scan" });
        let turn = store.ask_gathered(&key, &run, "O que acontece a cada volta?", &nodes, false, Some(origin.clone())).unwrap();
        assert_eq!(turn["prepare_error"], Value::Null, "{}", turn["prepare_error"]);
        assert_eq!(turn["gathered"]["origin"], origin);
    }

    #[test]
    fn a_project_in_a_folder_of_its_repository_runs_in_the_same_folder_of_the_copy() {
        // The repository is one level up: the project is its `project/` folder, as a `backend/`.
        let scratch = project();
        let root = scratch.0.join("project");
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&scratch.0)
                .args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        let profile = json!({ "project_root": ".", "commands": { "probe": {
            "argv": ["git", "log", "-1", "--format=tformat:librarian-measure answer=42 units"] } } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        git(&["init", "-q"]);
        git(&["config", "core.autocrlf", "false"]);
        git(&["add", "project"]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "probe", &sha).unwrap();

        let done = store.execute(&key, &run, "probe", None).unwrap();
        assert_eq!(done["summary"]["succeeded"], 1, "{:#}", done["runs"]);
        let copy = Path::new(done["copy"].as_str().unwrap()).to_path_buf();
        assert_eq!(copy.file_name().unwrap(), "project");
        assert!(copy.join("src/lib.rs").is_file());

        // The change's paths are relative to the project, and land in the same folder of the copy.
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
        let change = crate::execution::ChangeRef { turn, send, claim: "H1".into() };
        let changed = store.execute(&key, &run, "probe", Some(&change)).unwrap();
        assert_eq!(changed["summary"]["succeeded"], 1, "{:#}", changed["runs"]);
        let changed_copy = Path::new(changed["copy"].as_str().unwrap()).to_path_buf();
        assert!(fs::read_to_string(changed_copy.join("src/lib.rs")).unwrap().contains("x * 3.0"));
        let _ = fs::remove_dir_all(copy.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn a_change_that_deletes_a_counter_reaches_the_gate_refused() {
        let scratch = project();
        let root = scratch.0.join("project");
        // The line that counts is also what the command prints, so deleting it makes the series
        // vanish from a real run. The statement is not one "Medir tudo" could place.
        fs::write(root.join("src/lib.rs"), "pub struct Camera;

impl Camera {
    pub fn ray_from_pixel(&self, x: f64) -> f64 {
        let direction = x * 2.0;
        let counted = \"
librarian-measure calls=5 at=src/lib.rs:7
\";
        direction + counted.len() as f64
    }
}
").unwrap();
        let profile = json!({ "project_root": ".", "commands": { "contagem": { "argv": ["git", "grep", "-h", "librarian-measure"], "repeat": 1 } },
            "instrument": { "statement": "librarian-measure calls=5", "command": "contagem" } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q"]);
        git(&["config", "core.autocrlf", "false"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "contagem", &sha).unwrap();
        let base = store.execute(&key, &run, "contagem", None).unwrap();
        assert_eq!(base["summary"]["stats"]["calls@src/lib.rs:7"]["median"], 5.0, "{:#}", base["runs"]);
        let mut settings = store.settings(&key).unwrap();
        settings.lexicon.groups = vec![vec!["raios".into(), "ray".into()], vec!["direcao".into(), "direction".into()]];
        (settings.allow_hypothesis, settings.allow_patch) = (true, true);
        store.save_settings(&key, &settings).unwrap();
        let turn = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap()["id"].as_str().unwrap().to_owned();
        let answer = json!({"schema_version": 1, "claims": [{
            "id": "H1", "kind": "HYPOTHESIS", "text": "Tirar o contador.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 7, "end_line": 7, "quote": "librarian-measure calls=5 at=src/lib.rs:7"}],
            "premises": [], "verification_plan": "Medir antes e depois.", "location": null,
            "edits": [{"path": "src/lib.rs", "op": "replace", "start_line": 7, "end_line": 7, "new_lines": ["sem contador"]}]}]});
        let sent = record_answer(&store, &key, &run, &turn, "manual", &answer.to_string()).unwrap();
        let send = Path::new(sent["directory"].as_str().unwrap()).file_name().unwrap().to_string_lossy().into_owned();
        let change = crate::execution::ChangeRef { turn, send, claim: "H1".into() };
        let x = store.execute(&key, &run, "contagem", Some(&change)).unwrap();
        assert_eq!(x["counts_itself"], true);
        assert_eq!(x["comparison"]["counters"]["deleted"], json!(["calls@src/lib.rs:7"]), "{x:#}");
        let (passed, why) = crate::ladder::verdict(std::slice::from_ref(&x));
        assert!(!passed);
        assert!(why.iter().any(|w| w == "src/lib.rs:7 apaga a instrução de contagem"), "{why:?}");
        assert!(why.iter().any(|w| w == "contagem: calls@src/lib.rs:7 sumiu da medição: o contador foi apagado"), "{why:?}");
        // What was recorded is what the gate read.
        let saved = store.executions(&key, &run).unwrap().into_iter().last().unwrap();
        assert_eq!(saved["measurement_issues"], x["measurement_issues"]);
        assert_eq!(saved["comparison"]["counters"], x["comparison"]["counters"]);
    }

    #[test]
    fn a_change_is_judged_by_the_measures_the_profile_declares() {
        let scratch = project();
        let root = scratch.0.join("project");
        fs::write(root.join("src/lib.rs"), "pub struct Camera;

impl Camera {
    pub fn ray_from_pixel(&self, x: f64) -> f64 {
        let direction = x * 2.0;
        let printed = \"
librarian-measure memory_mb=100
\";
        direction + printed.len() as f64
    }
}
").unwrap();
        let profile = json!({ "project_root": ".", "commands": { "medir": { "argv": ["git", "grep", "-h", "librarian-measure"], "repeat": 1 } },
            "measures": { "memory_mb": { "better": "lower", "means": "memória no fim" } } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q"]);
        git(&["config", "core.autocrlf", "false"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "medir", &sha).unwrap();
        store.execute(&key, &run, "medir", None).unwrap();
        let mut settings = store.settings(&key).unwrap();
        settings.lexicon.groups = vec![vec!["raios".into(), "ray".into()], vec!["direcao".into(), "direction".into()]];
        (settings.allow_hypothesis, settings.allow_patch) = (true, true);
        store.save_settings(&key, &settings).unwrap();
        let turn = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap()["id"].as_str().unwrap().to_owned();
        let answer = json!({"schema_version": 1, "claims": [{
            "id": "H1", "kind": "HYPOTHESIS", "text": "Gastar menos memória.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 7, "end_line": 7, "quote": "librarian-measure memory_mb=100"}],
            "premises": [], "verification_plan": "Medir antes e depois.", "location": null,
            "edits": [{"path": "src/lib.rs", "op": "replace", "start_line": 7, "end_line": 7, "new_lines": ["librarian-measure memory_mb=20"]}]}]});
        let sent = record_answer(&store, &key, &run, &turn, "manual", &answer.to_string()).unwrap();
        let send = Path::new(sent["directory"].as_str().unwrap()).file_name().unwrap().to_string_lossy().into_owned();
        let change = crate::execution::ChangeRef { turn, send, claim: "H1".into() };
        let x = store.execute(&key, &run, "medir", Some(&change)).unwrap();
        assert_eq!(x["measures"]["memory_mb"]["better"], "lower", "{x:#}");
        assert_eq!(crate::ladder::verdict(std::slice::from_ref(&x)), (true, vec![]), "{x:#}");
        // A copy that went away (a cleaned temporary folder) comes back with its change applied,
        // though the diff beside it stayed: otherwise the baseline would be measured as the change.
        git(&["worktree", "remove", "--force", x["copy"].as_str().unwrap()]);
        let again = store.execute(&key, &run, "medir", Some(&change)).unwrap();
        assert_eq!(again["summary"]["stats"]["memory_mb"]["median"], 20.0, "{again:#}");
        // A declaration the gate could not read stops the run instead of being ignored.
        fs::write(root.join(PROFILE), json!({ "project_root": ".", "commands": profile["commands"], "measures": { "memory_mb": { "better": "menos" } } }).to_string()).unwrap();
        let wrong = store.execute(&key, &run, "medir", Some(&change)).unwrap_err().to_string();
        assert!(wrong.contains("measures inválido"), "{wrong}");
    }

    #[test]
    fn a_typed_question_is_answered_from_the_measure_and_the_measure_shows_how_it_counted() {
        let scratch = project();
        let root = scratch.0.join("project");
        // The string holds what the command prints: the count of the function that opens at line 1.
        fs::write(root.join("src/lib.rs"), "pub fn demo() -> u32 {\n    1\n}\npub const PRINTED: &str = \"\nlibrarian-measure calls=5 at=src/lib.rs:1\n\";\n").unwrap();
        let profile = json!({ "project_root": ".", "commands": { "contagem": { "argv": ["git", "grep", "-h", "librarian-measure"], "repeat": 1 } },
            "instrument": { "statement": "crate::count!();", "command": "contagem" } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q"]);
        git(&["config", "core.autocrlf", "false"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        // Before any measure: a question the rules know asks for it, any other is for the model.
        let early = store.performance_question(&key, &run, "Quais funções pesam?").unwrap();
        assert_eq!((early["route"]["intent"].as_str(), early["needs_measure"].as_bool()), (Some("heavy"), Some(true)));
        let named = store.performance_question(&key, &run, "Otimize o demo").unwrap();
        assert_eq!((named["route"]["intent"].as_str(), named["needs_measure"].as_bool()), (Some("optimize"), Some(true)));
        let other = store.performance_question(&key, &run, "Bom dia").unwrap();
        assert_eq!((other["route"]["intent"].as_str(), other["needs_measure"].as_bool()), (None, Some(false)));
        assert!(store.measure_detail(&key, &run).is_err());
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "contagem", &sha).unwrap();
        let measured = store.execute_instrumented(&key, &run).unwrap();
        assert_eq!(measured["summary"]["succeeded"], 1, "{:#}", measured["runs"]);
        // After it: the answer comes from the count.
        let heavy = store.performance_question(&key, &run, "Quais funções pesam?").unwrap();
        assert!(heavy["needs_measure"].is_null());
        assert!(heavy["answer"][0].as_str().unwrap().contains("demo (5 chamadas"), "{heavy:#}");
        assert_eq!(store.performance_question(&key, &run, "Bom dia").unwrap()["route"]["intent"], Value::Null);
        // And the page can show what was inserted and what the program printed.
        let how = store.measure_detail(&key, &run).unwrap();
        assert_eq!((how["statement"].as_str(), how["functions"].as_u64()), (Some("crate::count!();"), Some(1)));
        assert!(how["diff"].as_str().unwrap().contains("+pub fn demo() -> u32 { crate::count!();"), "{how:#}");
        assert!(how["stdout"].as_str().unwrap().contains("librarian-measure calls=5 at=src/lib.rs:1"), "{how:#}");
    }

    /// A project in two languages, measured through the real flow. What the command prints is in
    /// the string: two Rust functions and one that already counted, and a Python one whose count
    /// the project printed by itself.
    fn measured_mix() -> (Scratch, Store, String, String) {
        let scratch = project();
        let root = scratch.0.join("project");
        fs::write(root.join("src/lib.rs"), concat!(
            "pub fn roda() -> u32 {\n    1\n}\n",
            "pub fn contada_sem_rodar() -> u32 {\n    2\n}\n",
            "pub trait Forma {\n    fn area(&self) -> f64;\n    fn nome(&self) -> u32 {\n        3\n    }\n}\n",
            "pub struct C;\nimpl C {\n    pub fn metodo(&self) -> u32 {\n        4\n    }\n}\n",
            "pub const fn constante() -> u32 {\n    5\n}\n",
            "pub fn ja_conta() -> u32 {\n    crate::count!();\n    6\n}\n",
            "pub fn ja_conta_sem_linha() -> u32 {\n    crate::count!();\n    7\n}\n",
            "#[test]\nfn um_teste() {}\n",
            "pub const PRINTED: &str = \"\n",
            "librarian-measure calls=10 at=src/lib.rs:1\n",
            "librarian-measure calls=7 at=src/lib.rs:15\n",
            "librarian-measure calls=5 at=src/lib.rs:23\n",
            "librarian-measure calls=3 at=src/app.py:4\n",
            "\";\n",
        )).unwrap();
        fs::create_dir_all(root.join("src/bin")).unwrap();
        fs::write(root.join("src/bin/fora.rs"), "pub fn fora() -> u32 {\n    8\n}\n").unwrap();
        fs::write(root.join("src/app.py"), concat!(
            "def sem_chave(x):\n    return ajuda(x)\n\n",
            "def ajuda(x):\n    return x + 1\n\n",
            "def com_dict(x):\n    return {\"a\": x}\n\n",
            "def com_fstring(x):\n    return f\"{x}\"\n\n",
            "def comentario_com_chave(x):\n    # um { aqui\n    return x\n\n",
            "def condicional(x):\n    if x:\n        return {1: x}\n    return {}\n\n",
            "def stub(x): ...\n\n",
            "def test_algo():\n    assert {}\n\n",
            "class K:\n    def em_classe(self):\n        return {\"b\": 1}\n",
        )).unwrap();
        fs::write(root.join("src/manual.txt"), concat!(
            "librarian-measure calls=999 at=src/app.py:1\n",
            "librarian-measure calls=12345 at=src/lib.rs:1\n",
            "librarian-measure cpu_ms=777 ms at=src/lib.rs:1\n",
        )).unwrap();
        let profile = json!({ "project_root": ".", "commands": { "contagem": { "argv": ["git", "grep", "-h", "librarian-measure", "--", "src/lib.rs"], "repeat": 1 },
            "manual": { "argv": ["git", "grep", "-h", "librarian-measure", "--", "src/manual.txt"], "repeat": 1 } },
            "instrument": { "statement": "crate::count!();", "command": "contagem", "skip": ["src/bin/"] } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q"]);
        git(&["config", "core.autocrlf", "false"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "contagem", &sha).unwrap();
        let measured = store.execute_instrumented(&key, &run).unwrap();
        assert_eq!(measured["summary"]["succeeded"], 1, "{:#}", measured["runs"]);
        (scratch, store, key, run)
    }

    fn symbol_id(store: &Store, key: &str, run: &str, name: &str) -> String {
        let catalog = Catalog::load(&store.edition(key, run).unwrap()).unwrap();
        let found = catalog.symbols().values().find(|s| s.name == name).unwrap_or_else(|| panic!("{name} não está na edition"));
        found.id.clone()
    }

    /// The state and the reason `/api/hotspots` gives a function.
    fn coverage_of(hot: &Value, id: &str) -> (Value, Value) {
        let found = hot["coverage"]["functions"].as_array().into_iter().flatten().find(|f| f["symbol"] == id);
        let found = found.unwrap_or_else(|| panic!("{id} não está em coverage.functions: {:#}", hot["coverage"]));
        (found["coverage_state"].clone(), found["exclusion_reason"].clone())
    }

    /// A record from before the reasons were kept: the same execution without them.
    fn forget_reasons(store: &Store, key: &str, run: &str) {
        let id = store.latest_measure(key, run).unwrap().unwrap()["id"].as_str().unwrap().to_owned();
        let file = store.run_dir(key, run).unwrap().join("executions").join(id).join("execution.json");
        let mut record: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        record["instrumentation"].as_object_mut().unwrap().remove("excluded");
        fs::write(&file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    }

    const PYTHON_FUNCTIONS: [&str; 9] = ["sem_chave", "ajuda", "com_dict", "com_fstring", "comentario_com_chave", "condicional", "stub", "test_algo", "em_classe"];

    #[test]
    fn a_python_function_is_never_bodiless_whatever_braces_it_writes() {
        let (_scratch, store, key, run) = measured_mix();
        let hot = store.hotspots(&key, &run).unwrap();
        let bodiless = hot["bodiless"].as_array().unwrap();
        for name in PYTHON_FUNCTIONS {
            let id = symbol_id(&store, &key, &run, name);
            assert!(!bodiless.iter().any(|b| b == id.as_str()), "{name} aparece como sem corpo");
        }
    }

    #[test]
    fn rust_keeps_its_bodiless_signature_and_its_reasons() {
        let (_scratch, store, key, run) = measured_mix();
        let hot = store.hotspots(&key, &run).unwrap();
        let id = |name| symbol_id(&store, &key, &run, name);
        // Only the trait's signature has no body, and it stays out of the count.
        assert_eq!(hot["bodiless"], json!([id("area")]));
        // The fields that were there keep their meaning: what got the statement, and the totals.
        assert_eq!(hot["functions"], 4);
        assert_eq!(hot["skipped"], json!({ "caminho deixado de fora": 1, "const fn": 1, "já conta": 2, "sem corpo": 1, "teste": 1 }));
        for (name, state, reason) in [
            ("roda", "counted_and_ran", Value::Null),
            ("metodo", "counted_and_ran", Value::Null),
            // It counted by itself, and the line it printed is the evidence.
            ("ja_conta", "counted_and_ran", Value::Null),
            ("contada_sem_rodar", "counted_no_passes", Value::Null),
            // A trait's default method has a body: it got the count and did not run.
            ("nome", "counted_no_passes", Value::Null),
            ("area", "no_coverage", json!("no_body")),
            ("constante", "no_coverage", json!("const_fn")),
            ("ja_conta_sem_linha", "no_coverage", json!("already_counts")),
            ("um_teste", "no_coverage", json!("test")),
            ("fora", "no_coverage", json!("skipped_path")),
        ] {
            assert_eq!(coverage_of(&hot, &id(name)), (json!(state), reason), "{name}");
        }
    }

    #[test]
    fn a_mixed_project_shows_a_partial_coverage_with_its_python_functions_in_it() {
        let (_scratch, store, key, run) = measured_mix();
        let hot = store.hotspots(&key, &run).unwrap();
        let coverage = &hot["coverage"];
        assert_eq!(coverage["total"], 19, "{coverage:#}");
        assert_eq!(coverage["instrumented"], 4);
        assert_eq!(coverage["counted"], 6);
        assert_eq!(coverage["counted_and_ran"], 4);
        assert_eq!(coverage["counted_no_passes"], 2);
        assert_eq!(coverage["no_coverage"], 13);
        assert_eq!(coverage["partial"], true);
        assert_eq!(coverage["excluded"], json!({ "already_counts": 1, "const_fn": 1, "language_not_supported": 8, "no_body": 1, "skipped_path": 1, "test": 1 }));
        assert_eq!(coverage["by_language"]["rust"], json!({ "total": 10, "counted": 5, "counted_and_ran": 3, "counted_no_passes": 2, "no_coverage": 5 }));
        assert_eq!(coverage["by_language"]["python"], json!({ "total": 9, "counted": 1, "counted_and_ran": 1, "counted_no_passes": 0, "no_coverage": 8 }));
        // Coverage comes from the measure, not the language: a Python count the command printed is a count.
        assert_eq!(coverage_of(&hot, &symbol_id(&store, &key, &run, "ajuda")), (json!("counted_and_ran"), Value::Null));
        for name in PYTHON_FUNCTIONS.into_iter().filter(|n| *n != "ajuda") {
            let id = symbol_id(&store, &key, &run, name);
            assert_eq!(coverage_of(&hot, &id), (json!("no_coverage"), json!("language_not_supported")), "{name}");
            let function = coverage["functions"].as_array().unwrap().iter().find(|f| f["symbol"] == id.as_str()).unwrap();
            assert_eq!((function["language"].as_str(), function["path"].as_str()), (Some("python"), Some("src/app.py")));
        }
    }

    #[test]
    fn a_new_measure_keeps_the_reason_of_each_function_left_out() {
        let (_scratch, store, key, run) = measured_mix();
        let record = store.latest_measure(&key, &run).unwrap().unwrap();
        let excluded = record["instrumentation"]["excluded"].as_array().expect("instrumentation.excluded");
        assert_eq!(excluded.len(), 15);
        let area = symbol_id(&store, &key, &run, "area");
        assert_eq!(excluded.iter().find(|e| e["symbol"] == area.as_str()).unwrap()["reason"], "no_body");
        let ajuda = symbol_id(&store, &key, &run, "ajuda");
        assert_eq!(excluded.iter().find(|e| e["symbol"] == ajuda.as_str()).unwrap()["reason"], "language_not_supported");
    }

    #[test]
    fn a_record_without_reasons_says_not_recorded_and_invents_none() {
        let (_scratch, store, key, run) = measured_mix();
        let before = store.hotspots(&key, &run).unwrap();
        forget_reasons(&store, &key, &run);
        let hot = store.hotspots(&key, &run).unwrap();
        let coverage = &hot["coverage"];
        assert_eq!(before["coverage"]["reasons_recorded"], true);
        assert_eq!(coverage["reasons_recorded"], false);
        assert_eq!(coverage["excluded"], json!({ "not_recorded": 13 }), "{coverage:#}");
        assert_eq!(coverage_of(&hot, &symbol_id(&store, &key, &run, "area")), (json!("no_coverage"), json!("not_recorded")));
        assert_eq!(coverage_of(&hot, &symbol_id(&store, &key, &run, "com_dict")), (json!("no_coverage"), json!("not_recorded")));
        // What was counted is in the record either way.
        assert_eq!(coverage_of(&hot, &symbol_id(&store, &key, &run, "roda")), (json!("counted_and_ran"), Value::Null));
        assert_eq!(coverage_of(&hot, &symbol_id(&store, &key, &run, "nome")), (json!("counted_no_passes"), Value::Null));
        assert_eq!(coverage["counted"], before["coverage"]["counted"]);
        assert!(coverage["limits"].as_array().unwrap().iter().any(|l| l.as_str().unwrap().contains("não foi guardado")), "{coverage:#}");
        // The totals it did keep stay as they were, and the other fields do not move.
        assert_eq!(hot["skipped"], before["skipped"]);
        assert_eq!(hot["bodiless"], before["bodiless"]);
        assert_eq!(hot["rows"], before["rows"]);
    }

    #[test]
    fn a_new_measure_with_a_function_left_unclassified_does_not_pass_for_an_old_record() {
        let (_scratch, store, key, run) = measured_mix();
        // A real new measure classifies every function: neither absence appears.
        let hot = store.hotspots(&key, &run).unwrap();
        let excluded = hot["coverage"]["excluded"].as_object().unwrap();
        assert!(!excluded.contains_key("not_recorded") && !excluded.contains_key("not_classified"), "{excluded:#?}");
        // The record keeps reasons, and has none for this one function.
        let area = symbol_id(&store, &key, &run, "area");
        let id = store.latest_measure(&key, &run).unwrap().unwrap()["id"].as_str().unwrap().to_owned();
        let file = store.run_dir(&key, &run).unwrap().join("executions").join(id).join("execution.json");
        let mut record: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        record["instrumentation"]["excluded"].as_array_mut().unwrap().retain(|e| e["symbol"] != area.as_str());
        fs::write(&file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(coverage_of(&hot, &area), (json!("no_coverage"), json!("not_classified")));
        // The others keep the reason on record, and nothing says the record is an old one.
        assert_eq!(coverage_of(&hot, &symbol_id(&store, &key, &run, "constante")), (json!("no_coverage"), json!("const_fn")));
        let coverage = &hot["coverage"];
        assert!(!coverage["excluded"].as_object().unwrap().contains_key("not_recorded"), "{coverage:#}");
        let limits = coverage["limits"].to_string();
        assert!(!limits.contains("não foi guardado") && limits.contains("sem classificação"), "{limits}");
    }

    #[test]
    fn counted_and_instrumented_are_different_numbers() {
        let (_scratch, store, key, run) = measured_mix();
        let hot = store.hotspots(&key, &run).unwrap();
        let coverage = &hot["coverage"];
        // Instrumented: what got the statement, the same list as `functions` and `counted` beside it.
        assert_eq!(coverage["instrumented"], hot["functions"]);
        assert_eq!(coverage["instrumented"].as_u64(), hot["counted"].as_array().map(|c| c.len() as u64));
        // Counted: also what printed a count without getting the statement, here in both languages.
        let got_it = |name| hot["counted"].as_array().unwrap().iter().any(|c| c == symbol_id(&store, &key, &run, name).as_str());
        assert!(!got_it("ja_conta") && !got_it("ajuda"));
        assert_eq!(coverage["counted"].as_u64().unwrap() - coverage["instrumented"].as_u64().unwrap(), 2);
        assert!(coverage.get("eligible").is_none(), "{coverage:#}");
        assert!(coverage["limits"].to_string().contains("sem ter recebido a instrução"), "{:#}", coverage["limits"]);
    }

    #[test]
    fn a_suggestion_tells_counted_without_running_from_no_coverage_and_selects_the_same() {
        let (_scratch, store, key, run) = measured_mix();
        let python = symbol_id(&store, &key, &run, "com_dict");
        let rust = symbol_id(&store, &key, &run, "contada_sem_rodar");
        let items_of = |nodes: &[String]| store.suggest_envelope(&key, &run, nodes).unwrap();
        let on_python = items_of(std::slice::from_ref(&python));
        let on_rust = items_of(std::slice::from_ref(&rust));
        // Both have a measure and no calls: `measured` and `calls` alone cannot tell them apart.
        assert_eq!((&on_python["measured"], &on_rust["measured"]), (&json!(true), &json!(true)));
        let item = |out: &Value, id: &str| out["items"].as_array().unwrap().iter().find(|e| e["id"] == id).unwrap().clone();
        assert_eq!(item(&on_python, &python)["calls"], item(&on_rust, &rust)["calls"]);
        assert_eq!(item(&on_python, &python)["coverage_state"], "no_coverage");
        assert_eq!(item(&on_python, &python)["exclusion_reason"], "language_not_supported");
        assert_eq!(item(&on_rust, &rust)["coverage_state"], "counted_no_passes");
        assert_eq!(item(&on_rust, &rust)["exclusion_reason"], Value::Null);
        // What is not a function has no such state: a file's head is not "without coverage".
        for out in [&on_python, &on_rust] {
            for entry in out["items"].as_array().unwrap().iter().chain(out["left_out"].as_array().unwrap()) {
                assert!(entry.get("coverage_state").is_some(), "{entry:#}");
                if !["function", "method", "trait_method"].contains(&entry["kind"].as_str().unwrap_or_default()) {
                    assert_eq!(entry["coverage_state"], Value::Null, "{entry:#}");
                }
            }
        }
        // The summary is the one `/api/hotspots` gives, without the list of every function.
        let mut summary = store.hotspots(&key, &run).unwrap()["coverage"].clone();
        summary.as_object_mut().unwrap().remove("functions");
        assert_eq!(on_python["coverage"], summary);
        // Knowing the reasons or not changes no selection.
        let picked = |out: &Value, list: &str| out[list].as_array().unwrap().iter().map(|e| (e["id"].clone(), e["role"].clone(), e["calls"].clone(), e["left_out"].clone())).collect::<Vec<_>>();
        forget_reasons(&store, &key, &run);
        let forgotten = items_of(std::slice::from_ref(&python));
        assert_eq!(picked(&forgotten, "items"), picked(&on_python, "items"));
        assert_eq!(picked(&forgotten, "left_out"), picked(&on_python, "left_out"));
        assert_eq!(item(&forgotten, &python)["exclusion_reason"], "not_recorded");
    }

    #[test]
    fn with_no_measure_a_function_has_no_coverage_and_nothing_was_left_out() {
        let scratch = project();
        let root = scratch.0.join("project");
        fs::remove_file(root.join("src/lib.rs")).unwrap();
        fs::write(root.join("src/app.py"), "def demo():\n    return {\"a\": 1}\n").unwrap();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        // Still the same "no measure" answer, the one the route turns into a 404.
        assert!(store.hotspots(&key, &run).unwrap_err().to_string().starts_with("Ainda não há medição"));
        let demo = symbol_id(&store, &key, &run, "demo");
        let out = store.suggest_envelope(&key, &run, std::slice::from_ref(&demo)).unwrap();
        assert_eq!(out["measured"], false);
        assert!(out.get("coverage").is_some_and(Value::is_null), "{out:#}");
        let item = out["items"].as_array().unwrap().iter().find(|e| e["id"] == demo.as_str()).unwrap();
        assert_eq!(item["coverage_state"], "no_coverage");
        assert!(item.get("exclusion_reason").is_some_and(Value::is_null), "{item:#}");
    }

    /// Replaces what the recorded measure counted: `(at, calls, instrumented time)`.
    fn record_counts(store: &Store, key: &str, run: &str, lines: &[(&str, f64, Option<f64>)]) {
        let id = store.latest_measure(key, run).unwrap().unwrap()["id"].as_str().unwrap().to_owned();
        let file = store.run_dir(key, run).unwrap().join("executions").join(id).join("execution.json");
        let mut record: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        let mut stats = serde_json::Map::new();
        for (at, calls, cpu) in lines {
            stats.insert(format!("calls@{at}"), json!({ "measure": "calls", "at": at, "median": calls }));
            if let Some(cpu) = cpu {
                stats.insert(format!("cpu_ms@{at}"), json!({ "measure": "cpu_ms", "at": at, "median": cpu }));
            }
        }
        record["summary"]["stats"] = Value::Object(stats);
        fs::write(&file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    }

    fn places(hot: &Value) -> Vec<String> {
        hot["rows"].as_array().unwrap().iter().map(|r| r["at"].as_str().unwrap().to_owned()).collect()
    }

    /// Everything a question's answer gives the person and sends to a model.
    fn all_text(answer: &Value) -> String {
        ["answer", "facts", "changes", "limits"].iter().map(|k| answer[*k].to_string()).collect::<Vec<_>>().join(" ")
    }

    /// Words no text of a measure can use: what it has is calls and instrumented time.
    fn claims_nothing_it_has_no_data_for(text: &str) {
        for word in ["mais roda", "mais pesa", "mais pesada", "mais chama", "principalmente", "frequentes", "um milhão", "somados nas threads"] {
            assert!(!text.contains(word), "{word:?} em: {text}");
        }
    }

    #[test]
    fn more_calls_come_before_more_time() {
        let (_scratch, store, key, run) = measured_mix();
        record_counts(&store, &key, &run, &[("src/lib.rs:1", 10., Some(9000.)), ("src/lib.rs:15", 700., Some(2.)), ("src/lib.rs:23", 50., Some(40.))]);
        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(places(&hot), ["src/lib.rs:15", "src/lib.rs:23", "src/lib.rs:1"]);
        // The time stays beside the count, as it was recorded.
        assert_eq!((hot["rows"][0]["calls"].as_f64(), hot["rows"][0]["cpu_ms"].as_f64()), (Some(700.), Some(2.)));
    }

    #[test]
    fn the_function_with_the_most_time_is_not_described_as_the_most_frequent() {
        let (_scratch, store, key, run) = measured_mix();
        record_counts(&store, &key, &run, &[("src/lib.rs:1", 10., Some(9000.)), ("src/lib.rs:15", 700., Some(2.))]);
        let answer = store.performance_question(&key, &run, "Quais funções pesam?").unwrap();
        let lines: Vec<&str> = answer["answer"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
        assert!(lines[0].starts_with("Maior contagem observada: C::metodo (700 chamadas"), "{lines:#?}");
        assert!(lines[1].starts_with("Maior tempo instrumentado observado, como referência: roda (9.000 ms de tempo instrumentado, 10 chamadas)"), "{lines:#?}");
        claims_nothing_it_has_no_data_for(&all_text(&answer));
    }

    #[test]
    fn a_tie_on_the_calls_always_falls_the_same_way() {
        let (_scratch, store, key, run) = measured_mix();
        record_counts(&store, &key, &run, &[("src/lib.rs:23", 10., Some(5.)), ("src/lib.rs:1", 10., Some(5.)), ("src/lib.rs:15", 10., Some(6.))]);
        // Equal calls: the time, and with equal time too, the place.
        let first = places(&store.hotspots(&key, &run).unwrap());
        assert_eq!(first, ["src/lib.rs:15", "src/lib.rs:1", "src/lib.rs:23"]);
        assert_eq!(places(&store.hotspots(&key, &run).unwrap()), first);
    }

    #[test]
    fn no_coverage_is_not_zero_calls() {
        let (_scratch, store, key, run) = measured_mix();
        let hot = store.hotspots(&key, &run).unwrap();
        let python = symbol_id(&store, &key, &run, "com_dict");
        assert_eq!(coverage_of(&hot, &python).0, "no_coverage");
        // No line was made up for it, not even with zero.
        assert!(!hot["rows"].as_array().unwrap().iter().any(|r| r["symbol"] == python.as_str()), "{:#}", hot["rows"]);
        let answer = store.performance_question(&key, &run, "Otimize o com_dict").unwrap();
        assert_eq!((answer["route"]["intent"].as_str(), answer["route"]["function"].as_str()), (Some("optimize"), Some("com_dict")));
        let text = answer["answer"].to_string();
        assert!(text.contains("com_dict está sem cobertura nesta medição") && text.contains("não quer dizer que ela não rodou"), "{text}");
        assert!(!text.contains("0 chamadas") && !text.contains("não rodou neste comando"), "{text}");
        // There is nothing to send about a function the measure did not see.
        assert_eq!((answer["facts"].as_array().unwrap().len(), answer["changes"].as_array().unwrap().len()), (0, 0));
    }

    #[test]
    fn counted_without_calls_is_not_the_same_as_no_coverage() {
        let (_scratch, store, key, run) = measured_mix();
        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(coverage_of(&hot, &symbol_id(&store, &key, &run, "contada_sem_rodar")).0, "counted_no_passes");
        let idle = store.performance_question(&key, &run, "Otimize o contada_sem_rodar").unwrap()["answer"].to_string();
        assert!(idle.contains("recebeu a instrução e a medição não imprimiu contagem para ela; isso não prova que a função não rodou"), "{idle}");
        assert!(!idle.contains("sem cobertura"), "{idle}");
        let unseen = store.performance_question(&key, &run, "Otimize o com_dict").unwrap()["answer"].to_string();
        assert!(unseen.contains("sem cobertura") && !unseen.contains("recebeu a instrução"), "{unseen}");
        // A printed zero is a line and no pass: another reading, and neither ranking lists it.
        record_counts(&store, &key, &run, &[("src/lib.rs:1", 0., Some(3.)), ("src/lib.rs:15", 4., Some(1.))]);
        let zero = store.performance_question(&key, &run, "Otimize o roda").unwrap();
        assert!(zero["answer"].to_string().contains("A instrução de entrada de roda contou zero nesta medição; isso não prova que a função não rodou"), "{zero:#}");
        assert_eq!(zero["changes"].as_array().unwrap().len(), 0);
        let heavy = store.performance_question(&key, &run, "Quais funções pesam?").unwrap()["answer"].to_string();
        assert!(!heavy.contains("roda ("), "{heavy}");
    }

    /// A count from a common command belongs to that command: it does not reach the ranking, however
    /// large, and with no "Medir tudo" there is no ranking at all.
    #[test]
    fn a_manual_count_stays_out_of_the_ranking() {
        let (_scratch, store, key, run) = measured_mix();
        let before = store.hotspots(&key, &run).unwrap();
        let commands = store.commands(&key).unwrap();
        let sha = commands["commands"].as_array().unwrap().iter().find(|c| c["name"] == "manual").unwrap()["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "manual", &sha).unwrap();
        let manual = store.execute(&key, &run, "manual", None).unwrap();
        assert_eq!(manual["summary"]["stats"]["calls@src/app.py:1"]["median"], 999.);
        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(hot["rows"], before["rows"]);
        assert_eq!(hot["execution"], before["execution"]);
        let sem_chave = symbol_id(&store, &key, &run, "sem_chave");
        assert_eq!(coverage_of(&hot, &sem_chave).0, "no_coverage");
        assert!(!store.performance_question(&key, &run, "Quais funções pesam?").unwrap()["answer"].to_string().contains("999"));

        let scratch = project();
        let root = scratch.0.join("project");
        fs::remove_file(root.join("src/lib.rs")).unwrap();
        fs::write(root.join("src/app.py"), "def demo():\n    return 1\n").unwrap();
        fs::write(root.join("src/manual.txt"), "librarian-measure calls=999 at=src/app.py:1\n").unwrap();
        let profile = json!({ "project_root": ".", "commands": { "manual": { "argv": ["git", "grep", "-h", "librarian-measure", "--", "src/manual.txt"], "repeat": 1 } } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "manual", &sha).unwrap();
        let manual = store.execute(&key, &run, "manual", None).unwrap();
        assert_eq!(manual["summary"]["stats"]["calls@src/app.py:1"]["median"], 999.);
        assert!(store.hotspots(&key, &run).unwrap_err().to_string().starts_with("Ainda não há medição"));
        assert_eq!(store.performance_question(&key, &run, "Quais funções pesam?").unwrap()["needs_measure"], true);
    }

    /// Investigar reads what a command counted from `/api/executions`. A count made outside "Medir
    /// tudo" stays there, even on a line the ranking has, and the ranking keeps the measure's number.
    #[test]
    fn a_manual_count_is_there_for_investigar_and_not_in_the_ranking() {
        let (_scratch, store, key, run) = measured_mix();
        let before = store.hotspots(&key, &run).unwrap();
        let row = |hot: &Value| hot["rows"].as_array().unwrap().iter().find(|r| r["at"] == "src/lib.rs:1").unwrap().clone();
        assert_eq!(row(&before)["calls"], 10.);
        let commands = store.commands(&key).unwrap();
        let sha = commands["commands"].as_array().unwrap().iter().find(|c| c["name"] == "manual").unwrap()["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "manual", &sha).unwrap();
        store.execute(&key, &run, "manual", None).unwrap();

        let executions = store.executions(&key, &run).unwrap();
        let manual = executions.iter().find(|x| x["name"] == "manual").unwrap();
        assert!(manual["instrumentation"].is_null());
        let stats = &manual["summary"]["stats"];
        assert_eq!((stats["calls@src/lib.rs:1"]["median"].as_f64(), stats["calls@src/lib.rs:1"]["at"].as_str()), (Some(12345.), Some("src/lib.rs:1")));
        assert_eq!((stats["cpu_ms@src/lib.rs:1"]["median"].as_f64(), stats["cpu_ms@src/lib.rs:1"]["at"].as_str()), (Some(777.), Some("src/lib.rs:1")));
        assert_eq!(stats["calls@src/app.py:1"]["median"].as_f64(), Some(999.));

        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(hot["rows"], before["rows"]);
        assert_eq!(row(&hot)["calls"], 10.);
        assert!(row(&hot)["cpu_ms"].is_null());
        assert_ne!(hot["execution"], manual["id"]);
        let text = all_text(&store.performance_question(&key, &run, "Quais funções pesam?").unwrap());
        assert!(!text.contains("12.345") && !text.contains("777") && !text.contains("999"), "{text}");
    }

    /// A project's own counter printed while "Medir tudo" runs is in the ranking because it belongs
    /// to that measure: the row names the execution, whose output has that line and whose diff left
    /// it alone. Nothing is brought in from another execution.
    #[test]
    fn a_counter_printed_during_the_measure_belongs_to_that_measure() {
        let (_scratch, store, key, run) = measured_mix();
        let hot = store.hotspots(&key, &run).unwrap();
        let record = store.latest_measure(&key, &run).unwrap().unwrap();
        assert!(!record["instrumentation"].is_null());
        assert_eq!(hot["execution"], record["id"]);
        let own = hot["rows"].as_array().unwrap().iter().find(|r| r["at"] == "src/lib.rs:23").unwrap();
        assert_eq!(own["symbol"], symbol_id(&store, &key, &run, "ja_conta").as_str());
        // It did not get the statement; the program printed the line, in that execution.
        let got_it = record["instrumentation"]["functions"].as_array().unwrap().iter().any(|f| f["symbol"] == own["symbol"]);
        assert!(!got_it);
        assert_eq!(own["calls"], record["summary"]["stats"]["calls@src/lib.rs:23"]["median"]);
        let how = store.measure_detail(&key, &run).unwrap();
        assert_eq!(how["execution"], record["id"]);
        assert!(how["stdout"].as_str().unwrap().contains("librarian-measure calls=5 at=src/lib.rs:23"), "{how:#}");
        assert!(!how["diff"].as_str().unwrap().contains("@@ -23,"), "{how:#}");
        // Every row is one of that record's series, and no other execution has any.
        let series = record["summary"]["stats"].as_object().unwrap();
        assert!(hot["rows"].as_array().unwrap().iter().all(|r| series.contains_key(&format!("calls@{}", r["at"].as_str().unwrap()))));
        assert_eq!(store.executions(&key, &run).unwrap().len(), 1);
    }

    /// With no automatic target, no count makes a function the target: not a small one, and not one
    /// past the limit there used to be. The numbers are given, and the choice is the person's.
    #[test]
    fn no_count_picks_a_target_by_itself() {
        let (_scratch, store, key, run) = measured_mix();
        for calls in [3., 5e6] {
            record_counts(&store, &key, &run, &[("src/lib.rs:1", calls, Some(1.)), ("src/lib.rs:15", 2., Some(1.))]);
            let answer = store.performance_question(&key, &run, "Onde dá para otimizar?").unwrap();
            assert_eq!(answer["route"]["intent"], "where");
            assert!(answer["route"]["function"].is_null());
            assert_eq!((answer["facts"].as_array().unwrap().len(), answer["changes"].as_array().unwrap().len()), (0, 0), "{answer:#}");
            let lines = answer["answer"].to_string();
            assert!(lines.contains("Maior contagem observada: roda") && lines.contains("não escolhe o alvo: diga qual função, por exemplo: otimize o roda"), "{lines}");
            claims_nothing_it_has_no_data_for(&all_text(&answer));
        }
        // No calls at all: it says so, instead of comparing with a number.
        record_counts(&store, &key, &run, &[]);
        let none = store.performance_question(&key, &run, "Onde dá para otimizar?").unwrap();
        assert!(none["answer"].to_string().contains("Nenhuma função teve contagem acima de zero nesta medição"), "{none:#}");
        // And the limit is gone from what decides it, in the server and in the page.
        for (file, text) in [("questions.rs", include_str!("questions.rs")), ("index.html", include_str!("../web/index.html"))] {
            assert!(!text.contains("FREQUENT") && !text.contains("1e6"), "{file}");
        }
    }

    #[test]
    fn a_mixed_project_keeps_its_languages_apart_in_the_new_order() {
        let (_scratch, store, key, run) = measured_mix();
        let before = store.hotspots(&key, &run).unwrap()["coverage"].clone();
        record_counts(&store, &key, &run, &[("src/lib.rs:1", 2., Some(90.)), ("src/lib.rs:15", 7., Some(1.)), ("src/lib.rs:23", 5., None), ("src/app.py:4", 40., None)]);
        let hot = store.hotspots(&key, &run).unwrap();
        // The order is the count's, in any language; the coverage per language does not move with it.
        assert_eq!(places(&hot), ["src/app.py:4", "src/lib.rs:15", "src/lib.rs:23", "src/lib.rs:1"]);
        assert_eq!(hot["coverage"]["by_language"], before["by_language"]);
        assert_eq!(hot["coverage"]["instrumented"], before["instrumented"]);
        assert_eq!(hot["coverage"]["by_language"]["python"]["counted_and_ran"], 1);
        assert!(hot["coverage"]["by_language"]["python"]["no_coverage"].as_u64().unwrap() > 0);
        let python = store.performance_question(&key, &run, "Quais funções pesam?").unwrap();
        let text = python["answer"].to_string();
        // A line without time says so, and the answer does not stop at the functions it lists.
        // Printed by the project: its passes, whatever the language, and with no time on record.
        assert!(text.contains("ajuda (40 passagens pela linha 4, de um contador do próprio projeto (pode não ser quantas vezes a função foi chamada), tempo instrumentado não registrado)"), "{text}");
        assert!(text.contains(&format!("{} sem cobertura", hot["coverage"]["no_coverage"])), "{text}");
    }

    /// What goes to a model: counts and instrumented time under their names, what the time is not,
    /// and what the measure did not see. The caller's count is not the calls it makes.
    #[test]
    fn what_a_model_receives_is_no_stronger_than_the_measure() {
        let (_scratch, store, key, run) = measured_mix();
        record_counts(&store, &key, &run, &[("src/app.py:4", 40., Some(9000.)), ("src/app.py:1", 2., Some(1.)), ("src/lib.rs:1", 500., Some(3.))]);
        let answer = store.performance_question(&key, &run, "Otimize o ajuda").unwrap();
        let change = answer["changes"][0]["text"].as_str().unwrap();
        assert!(change.starts_with("ajuda: 40 passagens observadas pela linha 4, de um contador do próprio projeto (pode não ser quantas vezes a função foi chamada) nesta medição, 9.000 ms de tempo instrumentado; quem a chama, pelo nome escrito: sem_chave."), "{change}");
        assert!(change.contains("não é o custo próprio da função nem diz quantas vezes ela rodou"), "{change}");
        // The summary keeps a printed zero, no line and no coverage apart, and none is "did not run".
        let coverage = store.hotspots(&key, &run).unwrap()["coverage"].clone();
        let n = |field: &str| coverage[field].as_u64().unwrap();
        let no_line = coverage["zero_evidence"]["no_line"].as_u64().unwrap();
        assert!(no_line > 0, "{coverage:#}");
        assert!(change.contains(&format!("Esta medição tem contagem ou instrução em {} de {} funções: {} com contagem acima de zero; {} com zero impresso; {no_line} que receberam a instrução e não imprimiram linha; {} sem cobertura. Nenhum desses casos prova que a função não rodou.",
            n("counted"), n("total"), n("counted_and_ran"), n("counted_no_passes") - no_line, n("no_coverage"))), "{change}");
        for false_claim in ["não rodaram neste comando", "não rodou neste comando", "não executaram", "não foram chamadas"] {
            assert!(!change.contains(false_claim), "{false_claim:?} em {change}");
        }
        let fact = answer["facts"][0]["text"].as_str().unwrap();
        assert!(fact.starts_with("sem_chave chama ajuda na linha 2 (`ajuda`). A medição contou 2 passagens pela linha 1, de um contador do próprio projeto (pode não ser quantas vezes a função foi chamada) de sem_chave e 40 passagens pela linha 4,"), "{fact}");
        let told = answer["answer"].to_string();
        assert!(told.contains("com a contagem observada de cada chamador (não as chamadas feitas a ela): sem_chave (2)"), "{told}");
        claims_nothing_it_has_no_data_for(&all_text(&answer));
        // With no time on record, no number stands in for it.
        record_counts(&store, &key, &run, &[("src/lib.rs:1", 500., None)]);
        let change = store.performance_question(&key, &run, "Otimize o roda").unwrap()["changes"][0]["text"].as_str().unwrap().to_owned();
        assert!(change.contains("tempo instrumentado não registrado") && !change.contains("0 ms"), "{change}");
    }

    /// With several counted lines, no text calls a line's count the function's calls; with one, the
    /// text stays as it is. The number used for the function is still the largest.
    #[test]
    fn a_function_with_several_lines_is_not_said_to_have_been_called_that_many_times() {
        let (_scratch, store, key, run) = measured_mix();
        record_counts(&store, &key, &run, &[("src/lib.rs:1", 700., Some(5.)), ("src/lib.rs:2", 30., Some(4.)), ("src/lib.rs:15", 9., Some(1.)),
            ("src/app.py:4", 40., None), ("src/app.py:1", 80., None), ("src/app.py:2", 6., None)]);
        let hot = store.hotspots(&key, &run).unwrap();
        // Two lines of one function: each keeps its own count, and the function's number is the largest.
        let roda = symbol_id(&store, &key, &run, "roda");
        let of_roda: Vec<_> = hot["rows"].as_array().unwrap().iter().filter(|r| r["symbol"] == roda.as_str()).map(|r| r["calls"].as_f64().unwrap()).collect();
        assert_eq!(of_roda, [700., 30.]);
        assert_eq!(crate::instrument::calls_by_symbol(hot["rows"].as_array().unwrap())[&roda], 700.);

        let warned = "pode não ser quantas vezes a função foi chamada";
        let heavy = store.performance_question(&key, &run, "Quais funções pesam?").unwrap()["answer"].to_string();
        assert!(heavy.contains(&format!("roda (700 passagens pela linha 1 (uma de 2 linhas contadas; {warned})")), "{heavy}");
        assert!(!heavy.contains("700 chamadas") && !heavy.contains("30 chamadas"), "{heavy}");
        // One line: as before.
        assert!(heavy.contains("C::metodo (9 chamadas"), "{heavy}");

        let named = store.performance_question(&key, &run, "Otimize o roda").unwrap();
        let change = named["changes"][0]["text"].as_str().unwrap();
        assert!(change.starts_with(&format!("roda: 700 passagens observadas pela linha 1 (uma de 2 linhas contadas; {warned}) nesta medição")), "{change}");
        assert!(!change.contains("700 chamadas"), "{change}");
        assert!(named["answer"][0].as_str().unwrap().contains(warned), "{named:#}");

        // A question about a line: the caller has two, the target one.
        let fact = store.performance_question(&key, &run, "Otimize o ajuda").unwrap()["facts"][0]["text"].as_str().unwrap().to_owned();
        assert!(fact.contains(&format!("A medição contou 80 passagens pela linha 1 (uma de 2 linhas contadas; {warned}) de sem_chave e 40 passagens pela linha 4,")), "{fact}");
        let single = store.performance_question(&key, &run, "Otimize o C::metodo").unwrap()["changes"][0]["text"].as_str().unwrap().to_owned();
        assert!(single.starts_with("C::metodo: 9 chamadas observadas nesta medição"), "{single}");
    }

    /// The same record through the real flow: the statement at the entry reads as calls, and a line
    /// off the plan does not, though it is the only one its function has.
    #[test]
    fn one_line_is_not_enough_to_call_a_count_the_functions_calls() {
        let (_scratch, store, key, run) = measured_mix();
        let hot = store.hotspots(&key, &run).unwrap();
        let kind = |at: &str| hot["rows"].as_array().unwrap().iter().find(|r| r["at"] == at).unwrap()["count_kind"].clone();
        assert_eq!(kind("src/lib.rs:1"), "entry_calls");
        assert_eq!(kind("src/lib.rs:15"), "entry_calls");
        assert_eq!(kind("src/lib.rs:23"), "own_counter");
        assert_eq!(kind("src/app.py:4"), "own_counter");
        let entry = store.performance_question(&key, &run, "Otimize o roda").unwrap();
        assert!(entry["answer"][0].as_str().unwrap().starts_with("roda: 10 chamadas observadas nesta medição"), "{entry:#}");
        assert!(entry["changes"][0]["text"].as_str().unwrap().starts_with("roda: 10 chamadas observadas nesta medição"), "{entry:#}");
        let own = store.performance_question(&key, &run, "Otimize o ja_conta").unwrap();
        for text in [own["answer"][0].as_str().unwrap(), own["changes"][0]["text"].as_str().unwrap()] {
            assert!(text.starts_with("ja_conta: 5 passagens observadas pela linha 23, de um contador do próprio projeto"), "{text}");
            assert!(!text.contains("5 chamadas"), "{text}");
        }
        // A zero off the entry is not "did not run".
        record_counts(&store, &key, &run, &[("src/lib.rs:23", 0., None), ("src/lib.rs:1", 3., None)]);
        let zero = store.performance_question(&key, &run, "Otimize o ja_conta").unwrap()["answer"].to_string();
        assert!(zero.contains("A linha contada de ja_conta não teve passagens nesta medição, o que não prova que a função não rodou"), "{zero}");
        assert!(!zero.contains("não rodou neste comando"), "{zero}");
    }

    /// A project's counter inside a loop, measured for real: 30 passes through the line of a function
    /// entered 3 times. One line, and still not its calls.
    #[test]
    fn a_counter_inside_a_loop_is_read_as_passes() {
        let scratch = project();
        let root = scratch.0.join("project");
        fs::write(root.join("src/lib.rs"), concat!(
            "pub fn percorre() -> u32 {\n    for _ in 0..10 {\n        crate::count!();\n    }\n    0\n}\n",
            "pub const PRINTED: &str = \"\nlibrarian-measure calls=30 at=src/lib.rs:3\n\";\n",
        )).unwrap();
        let profile = json!({ "project_root": ".", "commands": { "contagem": { "argv": ["git", "grep", "-h", "librarian-measure", "--", "src/lib.rs"], "repeat": 1 } },
            "instrument": { "statement": "crate::count!();", "command": "contagem" } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        // The plan needs one function to place the statement in.
        fs::write(root.join("src/outra.rs"), "pub fn outra() -> u32 {\n    1\n}\n").unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q"]);
        git(&["config", "core.autocrlf", "false"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "contagem", &sha).unwrap();
        assert_eq!(store.execute_instrumented(&key, &run).unwrap()["summary"]["succeeded"], 1);
        let hot = store.hotspots(&key, &run).unwrap();
        let row = hot["rows"].as_array().unwrap().iter().find(|r| r["at"] == "src/lib.rs:3").unwrap();
        assert_eq!((row["calls"].as_f64(), row["count_kind"].as_str()), (Some(30.), Some("own_counter")));
        let answer = store.performance_question(&key, &run, "Otimize o percorre").unwrap();
        let text = all_text(&answer);
        assert!(text.contains("percorre: 30 passagens observadas pela linha 3"), "{text}");
        assert!(!text.contains("30 chamadas"), "{text}");
        claims_nothing_it_has_no_data_for(&text);
    }

    /// A record that cannot say where its counts came from: nothing is called calls or passes, in the
    /// answer or in what goes to a model. The number kept for the function does not move.
    #[test]
    fn a_count_of_unknown_origin_says_only_that_it_was_observed() {
        let (_scratch, store, key, run) = measured_mix();
        let before = store.hotspots(&key, &run).unwrap();
        let id = store.latest_measure(&key, &run).unwrap().unwrap()["id"].as_str().unwrap().to_owned();
        let file = store.run_dir(&key, &run).unwrap().join("executions").join(id).join("execution.json");
        let mut record: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        let functions = record["instrumentation"]["functions"].take();
        fs::write(&file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        let hot = store.hotspots(&key, &run).unwrap();
        assert!(hot["rows"].as_array().unwrap().iter().all(|r| r["count_kind"] == "unknown"), "{:#}", hot["rows"]);
        let calls_of = |h: &Value| h["rows"].as_array().unwrap().iter().map(|r| (r["at"].clone(), r["calls"].clone())).collect::<Vec<_>>();
        assert_eq!(calls_of(&hot), calls_of(&before));
        let answer = store.performance_question(&key, &run, "Otimize o roda").unwrap();
        for text in [answer["answer"][0].as_str().unwrap(), answer["changes"][0]["text"].as_str().unwrap()] {
            assert!(text.starts_with("roda: contagem observada de 10 nesta medição"), "{text}");
            assert!(!text.contains("10 chamadas") && !text.contains("10 passagens"), "{text}");
        }
        // Two functions on one line of the plan share one series: not the calls of either.
        record["instrumentation"]["functions"] = functions;
        let roda = symbol_id(&store, &key, &run, "roda");
        let sharing = record["instrumentation"]["functions"].as_array().unwrap().iter().find(|f| f["symbol"] == roda.as_str()).unwrap().clone();
        let mut other = sharing.clone();
        other["symbol"] = json!("outra");
        record["instrumentation"]["functions"].as_array_mut().unwrap().push(other);
        fs::write(&file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        let hot = store.hotspots(&key, &run).unwrap();
        let row = hot["rows"].as_array().unwrap().iter().find(|r| r["at"] == "src/lib.rs:1").unwrap();
        assert_eq!(row["count_kind"], "unknown");
    }

    /// A project with homonyms, measured through the real flow: `Sphere::default` has passes,
    /// `Stripe::default` got the statement and printed nothing, and `igual` exists twice.
    fn measured_homonyms() -> (Scratch, Store, String, String) {
        let scratch = project();
        let root = scratch.0.join("project");
        fs::write(root.join("src/lib.rs"), concat!(
            "pub struct Stripe;\nimpl Default for Stripe {\n    fn default() -> Self {\n        Stripe\n    }\n}\n",
            "pub struct Sphere;\nimpl Default for Sphere {\n    fn default() -> Self {\n        Sphere\n    }\n}\n",
            "pub fn unica() -> u32 {\n    1\n}\n",
            "pub trait Mostra {\n    fn nome(&self) -> u32;\n}\npub trait Conta {\n    fn nome(&self) -> u32;\n}\n",
            "impl Mostra for Stripe {\n    fn nome(&self) -> u32 {\n        1\n    }\n}\nimpl Conta for Stripe {\n    fn nome(&self) -> u32 {\n        2\n    }\n}\n",
            "impl Mostra for Sphere {\n    fn nome(&self) -> u32 {\n        3\n    }\n}\nimpl Conta for Sphere {\n    fn nome(&self) -> u32 {\n        4\n    }\n}\n",
            "pub const PRINTED: &str = \"\nlibrarian-measure calls=500 at=src/lib.rs:9\n\";\n",
        )).unwrap();
        fs::write(root.join("src/a.rs"), "pub fn igual() -> u32 {\n    1\n}\n").unwrap();
        fs::write(root.join("src/b.rs"), "pub fn igual() -> u32 {\n    2\n}\n").unwrap();
        let profile = json!({ "project_root": ".", "commands": { "contagem": { "argv": ["git", "grep", "-h", "librarian-measure", "--", "src/lib.rs"], "repeat": 1 } },
            "instrument": { "statement": "crate::count!();", "command": "contagem" } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q"]);
        git(&["config", "core.autocrlf", "false"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        (scratch, store, key, run)
    }

    fn measure_it(store: &Store, key: &str, run: &str) {
        let sha = store.commands(key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(key, "contagem", &sha).unwrap();
        assert_eq!(store.execute_instrumented(key, run).unwrap()["summary"]["succeeded"], 1);
    }

    fn id_of(store: &Store, key: &str, run: &str, qualified: &str) -> String {
        let catalog = Catalog::load(&store.edition(key, run).unwrap()).unwrap();
        let found = catalog.symbols().values().find(|s| s.qualified_name == qualified).unwrap_or_else(|| panic!("{qualified} não está na edition"));
        found.id.clone()
    }

    /// The case that was wrong: a function with no count asked for by name, and another with the
    /// same last part and passes. The answer is about the one asked for.
    #[test]
    fn a_qualified_name_gets_that_function_whatever_the_counts_say() {
        let (_scratch, store, key, run) = measured_homonyms();
        let stripe = id_of(&store, &key, &run, "impl Default for Stripe::default");
        let sphere = id_of(&store, &key, &run, "impl Default for Sphere::default");
        measure_it(&store, &key, &run);
        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(hot["rows"][0]["symbol"], sphere.as_str());
        assert_eq!(hot["rows"][0]["calls"], 500.);
        for question in ["Otimize o Stripe::default", "Otimize o Default for Stripe::default"] {
            let answer = store.performance_question(&key, &run, question).unwrap();
            assert_eq!(answer["route"]["target"]["symbol"], stripe.as_str(), "{question}");
            assert_eq!(answer["route"]["function"], "Default for Stripe::default", "{question}");
            let text = answer["answer"].to_string();
            assert!(text.contains("Default for Stripe::default recebeu a instrução e a medição não imprimiu contagem"), "{text}");
            assert!(!text.contains("Sphere") && !text.contains("500"), "{text}");
            // Only its own evidence: Sphere's passes would add a sentence here, under this name.
            assert_eq!(answer["answer"].as_array().unwrap().len(), 1, "{text}");
            assert!(!text.contains("mediana"), "{text}");
            assert_eq!((answer["facts"].as_array().unwrap().len(), answer["changes"].as_array().unwrap().len()), (0, 0));
        }
    }

    /// The measure is used once the target is known, on that symbol: the text and what goes to the
    /// model are about the same function.
    #[test]
    fn the_measure_is_read_only_for_the_target_already_resolved() {
        let (_scratch, store, key, run) = measured_homonyms();
        let stripe = id_of(&store, &key, &run, "impl Default for Stripe::default");
        let sphere = id_of(&store, &key, &run, "impl Default for Sphere::default");
        let asked = |store: &Store, question: &str| store.performance_question(&key, &run, question).unwrap();
        // Before any measure, the same target.
        let early = asked(&store, "Otimize o Stripe::default");
        assert_eq!((early["route"]["target"]["symbol"].as_str(), early["needs_measure"].as_bool()), (Some(stripe.as_str()), Some(true)));
        measure_it(&store, &key, &run);
        let on_sphere = asked(&store, "Otimize o Sphere::default");
        assert_eq!(on_sphere["route"]["target"]["symbol"], sphere.as_str());
        assert!(on_sphere["answer"][0].as_str().unwrap().starts_with("Default for Sphere::default: 500 chamadas observadas nesta medição"), "{on_sphere:#}");
        let change = &on_sphere["changes"][0];
        assert_eq!((change["symbol"].as_str(), change["nodes"][0].as_str()), (Some(sphere.as_str()), Some(sphere.as_str())));
        assert!(change["text"].as_str().unwrap().starts_with("Default for Sphere::default: 500 chamadas"), "{change:#}");
        // The counts change hands, and no target does.
        for (question, id) in [("Otimize o Stripe::default", &stripe), ("Otimize o Sphere::default", &sphere)] {
            record_counts(&store, &key, &run, &[("src/lib.rs:3", 900., None), ("src/lib.rs:9", 0., None)]);
            assert_eq!(asked(&store, question)["route"]["target"]["symbol"], id.as_str(), "{question}");
            record_counts(&store, &key, &run, &[("src/lib.rs:3", 0., None), ("src/lib.rs:9", 900., None)]);
            assert_eq!(asked(&store, question)["route"]["target"]["symbol"], id.as_str(), "{question}");
            record_counts(&store, &key, &run, &[]);
            assert_eq!(asked(&store, question)["route"]["target"]["symbol"], id.as_str(), "{question}");
        }
        record_counts(&store, &key, &run, &[("src/lib.rs:3", 900., None), ("src/lib.rs:9", 0., None)]);
        let moved = asked(&store, "Otimize o Stripe::default");
        assert_eq!(moved["changes"][0]["symbol"], stripe.as_str());
        assert!(moved["answer"][0].as_str().unwrap().starts_with("Default for Stripe::default: 900 chamadas"), "{moved:#}");
    }

    #[test]
    fn a_name_that_does_not_settle_on_one_function_asks_the_person() {
        let (_scratch, store, key, run) = measured_homonyms();
        let asked = |question: &str| store.performance_question(&key, &run, question).unwrap();
        let unresolved = |answer: &Value, status: &str| {
            assert_eq!(answer["route"]["target"]["status"], status, "{answer:#}");
            assert!(answer["route"]["function"].is_null() && answer["route"]["target"]["symbol"].is_null(), "{answer:#}");
            assert_eq!((answer["facts"].as_array().unwrap().len(), answer["changes"].as_array().unwrap().len()), (0, 0));
            assert!(answer["needs_measure"].is_null(), "{answer:#}");
        };
        // Whether or not there is a measure, and whichever has a count.
        for measured in [false, true] {
            if measured {
                measure_it(&store, &key, &run);
            }
            let many = asked("Otimize default");
            unresolved(&many, "ambiguous");
            let text = many["answer"].to_string();
            assert!(text.contains("O nome default combina com 2 funções: Default for Sphere::default (src/lib.rs), Default for Stripe::default (src/lib.rs)."), "{text}");
            assert!(text.contains("não escolhe: diga qual"), "{text}");
            // Not found: said so, with what shares the last part as a suggestion and not as an answer.
            let none = asked("Otimize o Cube::default");
            unresolved(&none, "not_found");
            let text = none["answer"].to_string();
            assert!(text.contains("Não encontrei a função Cube::default nesta edition") && text.contains("Com o mesmo último nome: Default for Sphere::default"), "{text}");
            unresolved(&asked("Otimize o Stripe::nada"), "not_found");
            assert!(!asked("Otimize o Stripe::nada")["answer"].to_string().contains("mesmo último nome"));
            // The same name twice: there is no way to write the difference, and the answer says so.
            let twice = asked("Otimize igual");
            unresolved(&twice, "ambiguous");
            let text = twice["answer"].to_string();
            assert!(text.contains("igual (src/a.rs), igual (src/b.rs)") && text.contains("não consegue separá-las pelo arquivo"), "{text}");
            // One function with the name: it is the target. A part of a name is not.
            assert_eq!(asked("Otimize a unica")["route"]["function"], "unica");
            assert_eq!(asked("Otimize a unic")["route"]["intent"], Value::Null);
        }
        // Where to optimize, with a name that does not settle: the same.
        unresolved(&asked("Onde melhorar o default?"), "ambiguous");
        // A question about the whole measure does not depend on any name.
        assert_eq!(asked("Quais funções pesam?")["route"]["intent"], "heavy");
    }

    /// The question as a person sends it: through the socket, the request parser, the route and the
    /// serialized body, and read back from those bytes.
    fn asked_through_the_api(store: &Store, key: &str, run: &str, question: &str) -> Value {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let body = json!({ "key": key, "run": run, "question": question }).to_string();
        let mut client = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        write!(client, "POST /api/performance-question HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        let (server, _) = listener.accept().unwrap();
        let request = crate::http::read(&server).unwrap();
        let app = crate::App { port: 0, viewer: PathBuf::new(), store: Store { dir: store.dir.clone(), modelfiles: store.modelfiles.clone() }, graph: std::sync::Mutex::new(None) };
        let response = crate::route(&app, &request).unwrap_or_else(|e| panic!("{question}: {e}"));
        crate::http::write(&server, &response).unwrap();
        drop(server);
        let mut raw = vec![];
        client.read_to_end(&mut raw).unwrap();
        let text = String::from_utf8(raw).unwrap();
        let (head, body) = text.split_once("\r\n\r\n").unwrap();
        assert!(head.starts_with("HTTP/1.1 200"), "{question}: {head}");
        serde_json::from_str(body).unwrap_or_else(|e| panic!("{question}: {e}: {body}"))
    }

    /// `execution` comes in the response exactly when the answer was read from a measure: the page
    /// says where an answer came from by this field alone.
    #[test]
    fn execution_comes_only_with_an_answer_read_from_a_measure() {
        let (_scratch, store, key, run) = measured_homonyms();
        let asked = |question: &str| asked_through_the_api(&store, &key, &run, question);
        let without = |question: &str, status: &str| {
            let answer = asked(question);
            assert_eq!(answer["route"]["target"]["status"], status, "{question}: {answer:#}");
            assert!(answer.get("execution").is_none(), "{question}: {answer:#}");
            answer
        };
        // No measure yet: it asks for one, and nothing was read.
        for (question, status) in [("Otimize a unica", "resolved"), ("Quais funções pesam?", "none")] {
            assert_eq!(without(question, status)["needs_measure"], true, "{question}");
        }
        // A name that settles on no function is answered before the measure, with or without one.
        let unsettled = || {
            for (question, status) in [("Otimize default", "ambiguous"), ("Otimize o Cube::nada", "not_found"), ("Otimize Stripe::default e Cube::nada", "not_found"), ("Otimize Stripe::default e Sphere::default", "ambiguous")] {
                assert!(without(question, status).get("needs_measure").is_none(), "{question}");
            }
        };
        unsettled();
        measure_it(&store, &key, &run);
        unsettled();
        // The measure was read: the field names that very execution.
        let id = store.latest_measure(&key, &run).unwrap().unwrap()["id"].clone();
        for (question, status) in [
            ("Otimize o Sphere::default", "resolved"),
            ("Otimize o Stripe::default", "resolved"),
            ("Quais funções pesam?", "none"),
            ("Onde dá para otimizar?", "none"),
            // Ambiguous, and still answered from the measure: the status does not tell where from.
            ("Quais default pesam?", "ambiguous"),
        ] {
            let answer = asked(question);
            assert_eq!(answer["route"]["target"]["status"], status, "{question}: {answer:#}");
            assert_eq!(answer["execution"], id, "{question}: {answer:#}");
            assert!(answer.get("needs_measure").is_none(), "{question}: {answer:#}");
        }
    }

    /// Several names in one question: every one is accounted for, and nothing goes to a model on the
    /// strength of only some of them.
    #[test]
    fn no_name_asked_for_is_dropped_on_the_way() {
        let (_scratch, store, key, run) = measured_homonyms();
        let stripe = id_of(&store, &key, &run, "impl Default for Stripe::default");
        let sphere = id_of(&store, &key, &run, "impl Default for Sphere::default");
        measure_it(&store, &key, &run);
        let asked = |question: &str| asked_through_the_api(&store, &key, &run, question);
        let nothing_built = |answer: &Value| {
            assert!(answer["route"]["function"].is_null() && answer["route"]["target"]["symbol"].is_null(), "{answer:#}");
            assert_eq!((answer["facts"].as_array().unwrap().len(), answer["changes"].as_array().unwrap().len()), (0, 0), "{answer:#}");
            assert!(answer.get("execution").is_none(), "{answer:#}");
        };
        let part = |answer: &Value, name: &str| answer["route"]["target"]["targets"].as_array().unwrap().iter().find(|p| p["asked"] == name).unwrap().clone();

        // One that exists and one that does not, in either order.
        for question in ["Otimize Stripe::default e Cube::nada", "Otimize Cube::nada e Stripe::default"] {
            let answer = asked(question);
            nothing_built(&answer);
            assert_eq!((answer["route"]["target"]["status"].as_str(), answer["route"]["target"]["written"].as_str()), (Some("not_found"), Some("Cube::nada")), "{question}");
            assert_eq!(part(&answer, "Stripe::default")["symbol"], stripe.as_str());
            let text = answer["answer"].to_string();
            assert!(text.contains("Não encontrei a função Cube::nada nesta edition"), "{text}");
            assert!(text.contains("Também pedida, e encontrada: Default for Stripe::default (src/lib.rs)") && text.contains("Nada foi montado com só uma parte do pedido"), "{text}");
        }
        // Two that exist: both named, one at a time, with the same reading as before this rule.
        let two = asked("Otimize Stripe::default e Sphere::default");
        nothing_built(&two);
        assert_eq!(two["route"]["target"]["status"], "ambiguous");
        assert_eq!((part(&two, "Stripe::default")["symbol"].as_str(), part(&two, "Sphere::default")["symbol"].as_str()), (Some(stripe.as_str()), Some(sphere.as_str())));
        assert!(two["answer"].to_string().contains("A pergunta nomeia 2 funções: Default for Stripe::default (src/lib.rs), Default for Sphere::default (src/lib.rs). O Librarian trata uma por vez"), "{two:#}");
        // Two that do not.
        let none = asked("Otimize Cube::nada e Bola::nada");
        nothing_built(&none);
        assert_eq!((none["route"]["target"]["status"].as_str(), none["route"]["target"]["written"].as_str()), (Some("not_found"), Some("Cube::nada e Bola::nada")));
        let text = none["answer"].to_string();
        assert!(text.contains("Não encontrei a função Cube::nada") && text.contains("Não encontrei a função Bola::nada"), "{text}");
        assert!(!text.contains("Também pedida"), "{text}");
        // Two that are each ambiguous: `Stripe::nome` is in two impls, and so is `Sphere::nome`.
        let open = asked("Otimize Stripe::nome e Sphere::nome");
        nothing_built(&open);
        assert_eq!((open["route"]["target"]["status"].as_str(), open["route"]["target"]["written"].as_str()), (Some("ambiguous"), Some("Stripe::nome e Sphere::nome")));
        let text = open["answer"].to_string();
        assert!(text.contains("O nome Stripe::nome combina com 2 funções") && text.contains("O nome Sphere::nome combina com 2 funções"), "{text}");
        // One that exists and one that is ambiguous.
        let mixed = asked("Otimize Stripe::default e Sphere::nome");
        nothing_built(&mixed);
        assert_eq!((mixed["route"]["target"]["status"].as_str(), mixed["route"]["target"]["written"].as_str()), (Some("ambiguous"), Some("Sphere::nome")));
        let text = mixed["answer"].to_string();
        assert!(text.contains("O nome Sphere::nome combina com 2 funções") && text.contains("Também pedida, e encontrada: Default for Stripe::default"), "{text}");
        // One name: as it was, in the answer and in what goes to a model.
        let one = asked("Otimize o Sphere::default");
        assert_eq!(one["route"]["target"]["symbol"], sphere.as_str());
        assert!(one["route"]["target"].get("targets").is_none());
        assert_eq!(one["changes"].as_array().unwrap().len(), 1);
        assert!(one["answer"][0].as_str().unwrap().starts_with("Default for Sphere::default: 500 chamadas observadas"), "{one:#}");
        // A file beside the name is not a second name, and neither is the same name written twice.
        for question in ["Otimize Sphere::default em src/lib.rs", "Otimize Sphere::default em lib.rs", "Otimize Sphere::default e Default for Sphere::default"] {
            assert_eq!(asked(question)["route"]["target"]["symbol"], sphere.as_str(), "{question}");
        }
    }

    /// Python names have their owner after a dot, and the rule is the same.
    #[test]
    fn a_python_function_is_found_by_its_own_name() {
        let (_scratch, store, key, run) = measured_mix();
        let asked = |question: &str| store.performance_question(&key, &run, question).unwrap();
        let method = id_of(&store, &key, &run, "K.em_classe");
        assert_eq!(asked("Otimize o K.em_classe")["route"]["target"]["symbol"], method.as_str());
        assert_eq!(asked("Otimize em_classe")["route"]["target"]["symbol"], method.as_str());
        assert_eq!(asked("Otimize o Outra.em_classe")["route"]["target"]["status"], "not_found");
        assert!(asked("Otimize em_classe")["answer"].to_string().contains("K.em_classe está sem cobertura"));
    }

    /// The evidence of a function with no pass observed, as `coverage.functions` gives it.
    fn zero_of_function(hot: &Value, id: &str) -> (Value, Value, Value) {
        let found = hot["coverage"]["functions"].as_array().unwrap().iter().find(|f| f["symbol"] == id).unwrap();
        (found["coverage_state"].clone(), found["zero_evidence"].clone(), found["exclusion_reason"].clone())
    }

    /// What the renderer does, through the real flow: the program's report prints the line of a
    /// function at its entry with zero, because it ran before the counts were zeroed. The function
    /// ran; the measure only saw no passes, and no text may say it did not run.
    #[test]
    fn a_zero_at_the_entry_after_a_reset_is_not_did_not_run() {
        let scratch = project();
        let root = scratch.0.join("project");
        fs::write(root.join("src/lib.rs"), concat!(
            "pub fn monta() -> u32 {\n    1\n}\n",
            "pub fn trabalha() -> u32 {\n    2\n}\n",
            "pub fn nunca() -> u32 {\n    3\n}\n",
            "pub const PRINTED: &str = \"\nlibrarian-measure calls=0 at=src/lib.rs:1\nlibrarian-measure calls=9 at=src/lib.rs:4\n\";\n",
        )).unwrap();
        let profile = json!({ "project_root": ".", "commands": { "contagem": { "argv": ["git", "grep", "-h", "librarian-measure", "--", "src/lib.rs"], "repeat": 1 } },
            "instrument": { "statement": "crate::count!();", "command": "contagem" } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q"]);
        git(&["config", "core.autocrlf", "false"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "contagem", &sha).unwrap();
        assert_eq!(store.execute_instrumented(&key, &run).unwrap()["summary"]["succeeded"], 1);
        let hot = store.hotspots(&key, &run).unwrap();
        let of = |name| zero_of_function(&hot, &symbol_id(&store, &key, &run, name));
        assert_eq!(of("monta"), (json!("counted_no_passes"), json!("entry_zero"), Value::Null));
        assert_eq!(of("trabalha"), (json!("counted_and_ran"), Value::Null, Value::Null));
        assert_eq!(of("nunca"), (json!("counted_no_passes"), json!("no_line"), Value::Null));
        assert_eq!(hot["coverage"]["zero_evidence"], json!({ "entry_zero": 1, "no_line": 1 }));
        let answer = store.performance_question(&key, &run, "Otimize o monta").unwrap();
        let text = answer["answer"].to_string();
        assert!(text.contains("A instrução de entrada de monta contou zero nesta medição; isso não prova que a função não rodou, porque o programa pode zerar as contagens"), "{text}");
        assert!(!text.contains("não rodou neste comando"), "{text}");
        assert_eq!(answer["changes"].as_array().unwrap().len(), 0);
        // The two have different words, and neither is said to have not run.
        let silent = store.performance_question(&key, &run, "Otimize o nunca").unwrap()["answer"].to_string();
        assert!(silent.contains("nunca recebeu a instrução e a medição não imprimiu contagem para ela"), "{silent}");
        let summary = store.performance_question(&key, &run, "Otimize o trabalha").unwrap()["changes"][0]["text"].as_str().unwrap().to_owned();
        assert!(summary.contains("1 com contagem acima de zero; 1 com zero impresso; 1 que receberam a instrução e não imprimiram linha;"), "{summary}");
    }

    #[test]
    fn each_kind_of_zero_has_its_own_evidence() {
        let (_scratch, store, key, run) = measured_mix();
        let id = |name| symbol_id(&store, &key, &run, name);
        // Got the statement and printed nothing; printed zero at the entry; the project's own counter
        // with zero, in Rust and in Python; and neither a line nor the statement.
        record_counts(&store, &key, &run, &[("src/lib.rs:1", 0., None), ("src/lib.rs:23", 0., None), ("src/app.py:4", 0., None), ("src/lib.rs:15", 3., None)]);
        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(zero_of_function(&hot, &id("roda")).1, "entry_zero");
        assert_eq!(zero_of_function(&hot, &id("contada_sem_rodar")).1, "no_line");
        assert_eq!(zero_of_function(&hot, &id("ja_conta")).1, "own_zero");
        assert_eq!(zero_of_function(&hot, &id("ajuda")).1, "own_zero");
        for name in ["roda", "contada_sem_rodar", "ja_conta", "ajuda"] {
            assert_eq!(zero_of_function(&hot, &id(name)).0, "counted_no_passes", "{name}");
        }
        assert_eq!(zero_of_function(&hot, &id("metodo")), (json!("counted_and_ran"), Value::Null, Value::Null));
        // No line and no statement stays no coverage, with its reason and no evidence of a zero.
        assert_eq!(zero_of_function(&hot, &id("com_dict")), (json!("no_coverage"), Value::Null, json!("language_not_supported")));
        assert_eq!(zero_of_function(&hot, &id("constante")), (json!("no_coverage"), Value::Null, json!("const_fn")));
        let total: u64 = hot["coverage"]["zero_evidence"].as_object().unwrap().values().filter_map(Value::as_u64).sum();
        assert_eq!(hot["coverage"]["counted_no_passes"].as_u64(), Some(total));
        // The suggestion says the same of a function it mentions.
        let out = store.suggest_envelope(&key, &run, &[id("sem_chave")]).unwrap();
        let mentioned = out["items"].as_array().unwrap().iter().chain(out["left_out"].as_array().unwrap()).find(|e| e["id"] == id("ajuda").as_str()).expect("ajuda na sugestão");
        assert_eq!((mentioned["coverage_state"].as_str(), mentioned["zero_evidence"].as_str()), (Some("counted_no_passes"), Some("own_zero")));

        // Several lines of a function, all zero; one above zero is a pass.
        record_counts(&store, &key, &run, &[("src/lib.rs:1", 0., None), ("src/lib.rs:2", 0., None)]);
        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(zero_of_function(&hot, &id("roda")).1, "several_zero");
        let text = store.performance_question(&key, &run, "Otimize o roda").unwrap()["answer"].to_string();
        assert!(text.contains("Nenhuma das linhas contadas de roda teve passagens nesta medição, o que não prova"), "{text}");
        record_counts(&store, &key, &run, &[("src/lib.rs:1", 0., None), ("src/lib.rs:2", 7., None)]);
        assert_eq!(zero_of_function(&store.hotspots(&key, &run).unwrap(), &id("roda")).0, "counted_and_ran");
    }

    #[test]
    fn a_zero_of_unknown_origin_stays_a_count_and_says_nothing_about_running() {
        let (_scratch, store, key, run) = measured_mix();
        let roda = symbol_id(&store, &key, &run, "roda");
        let contada = symbol_id(&store, &key, &run, "contada_sem_rodar");
        let measure = store.latest_measure(&key, &run).unwrap().unwrap()["id"].as_str().unwrap().to_owned();
        let file = store.run_dir(&key, &run).unwrap().join("executions").join(measure).join("execution.json");
        record_counts(&store, &key, &run, &[("src/lib.rs:1", 0., None), ("src/lib.rs:15", 4., None)]);
        let mut record: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        let functions = record["instrumentation"]["functions"].take();
        fs::write(&file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        // No plan to read. The zero is evidence of a count, not of no coverage; with no line and no
        // plan there is no evidence at all.
        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(zero_of_function(&hot, &roda), (json!("counted_no_passes"), json!("unknown_zero"), Value::Null));
        assert_eq!(zero_of_function(&hot, &contada).0, "no_coverage");
        assert_eq!(zero_of_function(&hot, &contada).1, Value::Null);
        let text = store.performance_question(&key, &run, "Otimize o roda").unwrap()["answer"].to_string();
        assert!(text.contains("roda tem contagem observada de zero nesta medição, e a origem não permite dizer se a função rodou"), "{text}");

        // Two functions on one source line, one series. With a zero: unknown. With passes: whose they
        // are the record cannot say, so the other one has no evidence of its own.
        record["instrumentation"]["functions"] = functions;
        for f in record["instrumentation"]["functions"].as_array_mut().unwrap() {
            if f["symbol"] == contada.as_str() {
                f["line"] = json!(1);
            }
        }
        fs::write(&file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(zero_of_function(&hot, &contada), (json!("counted_no_passes"), json!("unknown_zero"), Value::Null));
        record["summary"]["stats"]["calls@src/lib.rs:1"]["median"] = json!(6.);
        fs::write(&file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        let hot = store.hotspots(&key, &run).unwrap();
        assert_eq!(zero_of_function(&hot, &contada), (json!("no_coverage"), Value::Null, json!("shared_line")));
    }

    /// One repetition above zero is a pass, whatever the median. The median stays the number shown,
    /// and the largest is not presented as the function's calls.
    #[test]
    fn a_median_of_zero_with_a_positive_maximum_ran() {
        let (_scratch, store, key, run) = measured_mix();
        let measure = store.latest_measure(&key, &run).unwrap().unwrap()["id"].as_str().unwrap().to_owned();
        let file = store.run_dir(&key, &run).unwrap().join("executions").join(measure).join("execution.json");
        let mut record: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        let before = fs::read(&file).unwrap();
        record["summary"]["stats"]["calls@src/lib.rs:1"] = json!({ "measure": "calls", "at": "src/lib.rs:1", "median": 0., "min": 0., "max": 5., "n": 3 });
        fs::write(&file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        assert_ne!(before, fs::read(&file).unwrap());
        let on_disk = fs::read(&file).unwrap();
        let hot = store.hotspots(&key, &run).unwrap();
        let roda = symbol_id(&store, &key, &run, "roda");
        assert_eq!(zero_of_function(&hot, &roda), (json!("counted_and_ran"), Value::Null, Value::Null));
        let row = hot["rows"].as_array().unwrap().iter().find(|r| r["at"] == "src/lib.rs:1").unwrap();
        assert_eq!((row["calls"].as_f64(), row["calls_max"].as_f64()), (Some(0.), Some(5.)));
        assert_eq!(crate::instrument::calls_by_symbol(hot["rows"].as_array().unwrap())[&roda], 0.);
        let answer = store.performance_question(&key, &run, "Otimize o roda").unwrap();
        let text = answer["answer"].to_string();
        assert!(text.contains("A mediana das repetições de roda é zero, mas ao menos uma repetição observou passagens"), "{text}");
        assert!(!text.contains("5 chamadas") && !text.contains("contou zero"), "{text}");
        // Reading does not touch what was recorded.
        assert_eq!(on_disk, fs::read(&file).unwrap());
    }

    /// The page describes counts, not execution, wherever it speaks of functions with no passes. The
    /// page itself is exercised on a real measure by the screen checks.
    #[test]
    fn the_page_does_not_say_a_function_did_not_run() {
        let page = include_str!("../web/index.html");
        for claim in ["não rodou", "não rodaram nenhuma", "que rodaram em", "só o que rodou", "só este rodou", "só um deles rodou", "did not run", "never ran", "only what ran", "only this one ran", "functions that ran", "counted_not_run"] {
            assert!(!page.lines().any(|l| l.contains(claim) && !l.contains("o que não prova que não rodaram") && !l.contains("does not prove they did not run")), "{claim:?}");
        }
        for label in ["'envelope.out.did_not_run': ['sem passagens na medição'", "'path.notRan': ['sem contagem'", "'path.ran': ['só o que teve contagem'", "funções com contagem acima de zero em {districts}"] {
            assert!(page.contains(label), "{label:?}");
        }
    }

    #[test]
    fn the_limits_of_a_measure_say_what_each_number_is() {
        let (_scratch, store, key, run) = measured_mix();
        let hot = store.hotspots(&key, &run).unwrap();
        let limits = hot["limits"].to_string();
        for phrase in ["cada número é o que esta medição observou naquela linha", "a frequência observada neste comando", "é o número de chamadas observadas", "a maior contagem, que pode não ser quantas vezes ela foi chamada",
            "conta as passagens por aquela linha", "tempo instrumentado, não o custo próprio", "pode mudar a ordem",
            "não quer dizer que ela não rodou", "não observou passagens positivas, não que ela não rodou", "zerar as contagens durante a execução"] {
            assert!(limits.contains(phrase), "{phrase:?} em {limits}");
        }
        // The fields of the coverage stay as they were.
        for field in ["instrumented", "counted", "counted_and_ran", "counted_no_passes", "no_coverage", "partial", "excluded", "by_language", "limits", "reasons_recorded", "zero_evidence"] {
            assert!(hot["coverage"].get(field).is_some(), "{field}");
        }
        assert!(!limits.contains("não rodou neste comando"), "{limits}");
        assert!(hot["coverage"].get("counted_not_run").is_none());
    }

    #[test]
    fn stamp_is_sortable_utc() {
        let s = stamp();
        assert_eq!(s.len(), 15);
        assert_eq!(&s[8..9], "-");
        assert!(s.starts_with("20"));
    }

    /// A real edition of this source, for what a LACUNA finds in it.
    fn catalog_of(source: &str) -> (Scratch, Catalog) {
        let scratch = project();
        fs::write(scratch.0.join("project/src/lib.rs"), source).unwrap();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&scratch.0.join("project"))).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        (scratch, catalog)
    }

    fn found(catalog: &Catalog, name: &str, ran: &[&str]) -> Vec<String> {
        let ran: Vec<String> = ran.iter().map(|r| (*r).to_owned()).collect();
        crate::ladder::resolve(catalog, &[name.to_owned()], &[], &ran, 10).iter().map(|(id, _)| catalog.symbols()[id].qualified_name.clone()).collect()
    }

    #[test]
    fn a_method_two_traits_declare_is_not_guessed() {
        let (_scratch, catalog) = catalog_of("pub trait Incuse {\n    fn transform(&self) -> f64;\n}\n\npub trait Intersectable {\n    fn transform(&self) -> f64;\n    fn local(&self) -> f64;\n}\n");
        assert_eq!(found(&catalog, "transform", &[]), Vec::<String>::new());
        // Written with its trait there is nothing to guess, and a method of one trait still comes.
        assert_eq!(found(&catalog, "Intersectable::transform", &[]), ["Intersectable::transform"]);
        assert_eq!(found(&catalog, "local", &[]), ["Intersectable::local"]);
    }

    #[test]
    fn a_trait_asked_by_name_comes_when_it_ran() {
        let (_scratch, catalog) = catalog_of("pub trait Shape {\n    fn area(&self) -> f64 { 1.0 }\n}\n\npub struct Square;\n\nimpl Shape for Square {}\n\npub trait Idle {\n    fn nap(&self) -> f64 { 0.0 }\n}\n");
        assert_eq!(found(&catalog, "Shape", &["Shape::area"]), ["Shape"]);
        assert_eq!(found(&catalog, "Shape", &["impl Shape for Square::area"]), ["Shape"]);
        // What did not run in the measure still stays out, and with no measure everything can come.
        assert_eq!(found(&catalog, "Idle", &["Shape::area"]), Vec::<String>::new());
        assert_eq!(found(&catalog, "Idle", &[]), ["Idle"]);
    }
}
