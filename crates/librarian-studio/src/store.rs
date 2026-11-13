//! Projects, runs and questions under `.studio/`. Nothing is written inside analyzed projects.
//! Layout: projects/<key>/project.json, runs/<run>/{edition, graph-v2.json, run.json},
//! runs/<run>/turns/<NN>/{question.json, search.json, prepare-request.json, prepared/, claims/, sends/},
//! runs/<run>/suggestions/<sha256>.json; each new question that reached the preparation also has
//! turns/<NN>/evidence.json, named by its hash in question.json, and one asked with a selection has
//! turns/<NN>/selection.json, named by its hash in the package.
use librarian_llm_engine::claims::{self, compare::compare, Claim, Policy};
use librarian_ingest::{
    envelope,
    evidence::{
        self, BoundArtifacts, ClaimsEvidence, CompositionInput, EditionIdentity, EvidencePackage, FactRecord, Form, InventoryRuleEntry, InventoryRuleOrigin,
        MeasureAtQuestion, MeasureReferences, MeasureSelectionBasis, Normalization, OriginKind, PackageFormat, PackageKind, ProvenanceInput, QuestionInput,
        Representation, RetryRule, Route, RulesVersion, SelectionEdition, SelectionInput, SelectionRecord, SelectionRecordFormat, SelectionReference,
        ServerAddition, ServerDeclared, Sha256Hex, SnapshotInput, SuffixInput, SuggestionInput, SuggestionInventoryRules, SuggestionMeasure, SuggestionRecord, Versions,
    },
    inventory,
    graph::v2,
    hash,
    prepare::{self, Request, ReviewedFact, Target},
    search::{Lexicon, SearchIndex},
    Catalog, Config, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

/// Optional profile in the project root: sources, lexicon, instructions, criteria, search.
pub const PROFILE: &str = "librarian.json";
const CRITERIA: &str = "criteria-before.md";
/// A new question's evidence package, beside its question.json.
const EVIDENCE: &str = "evidence.json";
/// The record of the selection a new question was asked with, beside its package.
const SELECTION: &str = "selection.json";

/// A selection the server checked at the entry of a question, before anything was created: its
/// record and the bytes it is kept in. Only `Store::accept_selection` makes one.
pub struct AcceptedSelection {
    record: SelectionRecord,
    bytes: Vec<u8>,
    sha256: Sha256Hex,
}

/// What a server retry added to a client's question, with the rule that added it.
#[derive(Debug, Clone)]
pub struct Suffix {
    pub text: String,
    pub rule: RetryRule,
    pub attempt: usize,
}

/// A question as asked: the client's text and what the server added to it, in order.
#[derive(Debug, Clone, Copy)]
pub struct Asked<'a> {
    pub original: &'a str,
    pub suffixes: &'a [Suffix],
}

impl<'a> Asked<'a> {
    fn client(original: &'a str) -> Self {
        Self { original, suffixes: &[] }
    }
    /// The question prepared: the texts joined, nothing else.
    fn prepared(&self) -> String {
        std::iter::once(self.original).chain(self.suffixes.iter().map(|s| s.text.as_str())).collect()
    }
}

/// Where a run keeps the record of each suggestion, by the hash of its bytes.
const SUGGESTIONS: &str = "suggestions";
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

/// What a send delivers once its question was re-verified. Only `Store::verify_for_send` makes
/// one, after every check it runs; nothing else can, since its fields are its own.
pub struct Verified {
    query: PathBuf,
    context: Option<librarian_llm_engine::claims::Context>,
    criteria_sha256: Option<String>,
    /// Recorded with the send (`send.json`, and the Ollama attempt's `result.json`).
    link: Value,
}

impl Verified {
    pub fn query(&self) -> &Path {
        &self.query
    }
    pub fn context(&self) -> Option<&librarian_llm_engine::claims::Context> {
        self.context.as_ref()
    }
    pub fn into_context(self) -> Option<librarian_llm_engine::claims::Context> {
        self.context
    }
    pub fn criteria_sha256(&self) -> Option<&str> {
        self.criteria_sha256.as_deref()
    }
    pub fn link(&self) -> &Value {
        &self.link
    }
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
        self.new_turn(key, run, &settings, &catalog, Asked::client(question), Some(search), request, None)
    }

    /// P1b: one question per declaration the person picked on a "what does it call?" path, each
    /// with that declaration alone as evidence (like `hits: 1`). Nothing is searched; the path
    /// (seed, position, the choices among homonyms) is recorded with each question.
    pub fn ask_path(&self, key: &str, run: &str, question: &str, nodes: &[String], seed: &str, chosen: &Value) -> Result<Value> {
        refuse_notes(chosen)?;
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
            turns.push(self.new_turn(key, run, &settings, &catalog, Asked::client(question), None, request, Some(("path", path)))?);
        }
        Ok(json!(turns))
    }

    /// P4: one question whose evidence is the declarations the person gathered one by one (in
    /// the order gathered), with nothing searched and nothing added; the step says so. P5: the
    /// envelope may hold file heads (`header:<path>`), and, with `with_facts`, the question also
    /// carries the current reviewed facts about the gathered declarations.
    /// An investigation from the inventory records its `origin` (the finding it came from).
    /// A selection in `origin` is checked here, once, before anything is created.
    pub fn ask_gathered(&self, key: &str, run: &str, question: &str, nodes: &[String], with_facts: bool, origin: Option<Value>) -> Result<Value> {
        let selection = self.accept_selection(key, run, nodes, origin.as_ref())?;
        self.ask_gathered_parts(key, run, question, &[], nodes, with_facts, origin, selection.as_ref(), &[])
    }

    /// `ask_gathered` for a question the server added to: the client's text, then what each retry
    /// added, in order. The prepared question is those texts joined, nothing else. The selection is
    /// the one accepted at the client's entry (`accept_selection`), not checked again here: the nodes
    /// a ladder round adds are the server's, not the client's statement. Those are `additions`, in
    /// the order the rounds added them, and they are the last of `nodes`; each is checked against
    /// the LACUNA that asked for it before anything is created.
    #[allow(clippy::too_many_arguments)]
    pub fn ask_gathered_parts(&self, key: &str, run: &str, original: &str, suffixes: &[Suffix], nodes: &[String], with_facts: bool, origin: Option<Value>,
        selection: Option<&AcceptedSelection>, additions: &[ServerAddition]) -> Result<Value> {
        if let Some(origin) = &origin {
            refuse_notes(origin)?;
        }
        // An origin with a selection goes with the record accepted for it, and only then.
        if selection.is_some() != origin.as_ref().is_some_and(|o| o.get("selection").is_some()) {
            return Err("A seleção da origem não foi conferida na entrada; nenhuma pergunta foi criada".into());
        }
        let asked = Asked { original, suffixes };
        let joined = asked.prepared();
        let question = joined.as_str();
        if nodes.is_empty() || nodes.len() > MAX_GATHERED {
            return Err(format!("Junte de 1 a {MAX_GATHERED} declarações").into());
        }
        let settings = self.settings(key)?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let included: Option<Vec<String>> = selection.map(|s| s.record.received.included.iter().map(|e| e.id.clone()).collect());
        self.check_additions(key, run, &catalog, nodes, included.as_deref(), additions)
            .map_err(|e| format!("Complementação recusada ({e}); nenhuma pergunta foi criada"))?;
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
        // `notes` are refused at the entry: nothing a client sends goes to the model as a note.
        let mut gathered = json!({ "nodes": nodes, "with_facts": with_facts, "reviewed_facts": reviewed.iter().map(|f| &f.id).collect::<Vec<_>>() });
        if let Some(origin) = origin {
            gathered["origin"] = origin;
        }
        // What the server added, apart from what the client sent; a question with none says nothing.
        if !additions.is_empty() {
            gathered["server_additions"] = serde_json::to_value(additions)?;
        }
        let mut request = request(&settings, question, Some(targets), reviewed, false);
        request.gathered = true;
        request.headers = headers;
        self.new_turn_with(key, run, &settings, &catalog, asked, None, request, Some(("gathered", gathered)), selection)
    }

    /// The declarations a ladder's LACUNA rounds added, as the question says: each the server's,
    /// once, after the client's (the selection's included sources, when there is one), in the order
    /// added; each asked for by a LACUNA of the send it names, and found for it in this edition by
    /// `ladder::resolve`. The rule is read with nothing excluded by the measure, which only ever
    /// leaves out: a declaration it gave then is one it gives now.
    fn check_additions(&self, key: &str, run: &str, catalog: &Catalog, nodes: &[String], included: Option<&[String]>, additions: &[ServerAddition]) -> Result<()> {
        let ids: Vec<&String> = additions.iter().map(|a| &a.symbol_id).collect();
        let Some(split) = nodes.len().checked_sub(ids.len()) else { return Err("mais complementações do que declarações".into()) };
        let (client, added) = nodes.split_at(split);
        // With a selection, what is not the server's is exactly what the client sent: nothing more.
        if included.is_some_and(|included| included != client) {
            return Err("as declarações da cliente não são as da seleção".into());
        }
        if additions.is_empty() {
            return Ok(());
        }
        if added.iter().ne(ids.iter().copied()) {
            return Err("as declarações acrescentadas não são as últimas da pergunta, na ordem".into());
        }
        for (n, id) in ids.iter().enumerate() {
            if ids[..n].contains(id) || client.contains(id) {
                return Err(format!("{id} já estava na pergunta").into());
            }
        }
        // Round by round: what one send's LACUNA asked for, over what was gathered before it.
        let mut start = 0;
        while start < additions.len() {
            let first = &additions[start];
            let end = start + additions[start..].iter().take_while(|a| (a.round, &a.from_turn, &a.from_send) == (first.round, &first.from_turn, &first.from_send)).count();
            let sent = self.turn_dir(key, run, &first.from_turn)?.join("sends").join(safe_id(&first.from_send)?).join("send.json");
            let summary: Value = serde_json::from_slice(&fs::read(&sent).map_err(|_| format!("envio {}/{} indisponível", first.from_turn, first.from_send))?)?;
            let (_, lacunas, _) = crate::ladder::outcome(&summary);
            let names: Vec<String> = lacunas.iter().flat_map(|t| crate::ladder::lacuna_names(t)).collect();
            let before: Vec<String> = client.iter().chain(&added[..start]).cloned().collect();
            let found = crate::ladder::resolve(catalog, &names, &before, &[], usize::MAX);
            for addition in &additions[start..end] {
                let named = names.contains(&addition.for_name) || addition.for_name.split_once("::").is_some_and(|(t, _)| names.iter().any(|n| n == t));
                if addition.round == 0 || !named || !found.iter().any(|(id, _)| *id == addition.symbol_id) {
                    return Err(format!("{} não é o que a LACUNA de {}/{} pediu", addition.symbol_id, first.from_turn, first.from_send).into());
                }
            }
            start = end;
        }
        Ok(())
    }

    /// A selection sent with a question, checked against the suggestion records of this run and its
    /// edition, before anything is created. None when the origin has no selection. `notes`, anywhere
    /// in the origin, is refused first.
    pub fn accept_selection(&self, key: &str, run: &str, nodes: &[String], origin: Option<&Value>) -> Result<Option<AcceptedSelection>> {
        let Some(origin) = origin else { return Ok(None) };
        refuse_notes(origin)?;
        let refuse = |why: String| -> Box<dyn std::error::Error> { format!("Seleção recusada ({why}); nenhuma pergunta foi criada").into() };
        let kind = origin.get("kind").and_then(Value::as_str);
        let Some(selection) = origin.get("selection") else {
            if kind == Some("selection") {
                return Err(refuse("origem selection sem seleção".into()));
            }
            return Ok(None);
        };
        if !matches!(kind, Some("selection" | "ladder")) {
            return Err(refuse("só uma origem selection ou ladder traz seleção".into()));
        }
        let input = SelectionInput::from_value(selection).map_err(|e| refuse(e.to_string()))?;
        self.accept_input(key, run, nodes, input).map(Some).map_err(|e| refuse(e.to_string()))
    }

    /// The record of a selection: its suggestions read back and checked, then the selection checked
    /// against them and the edition. The same input gives the same bytes.
    fn accept_input(&self, key: &str, run: &str, nodes: &[String], input: SelectionInput) -> Result<AcceptedSelection> {
        let mut records = vec![];
        for sha256 in &input.suggestions {
            let suggestion = |why: String| evidence::SelectionRefusal::Suggestion { sha256: sha256.as_str().to_owned(), why };
            let record = self.verify_suggestion(key, run, sha256.as_str()).map_err(|e| suggestion(e.to_string()))?;
            if record.rules_version.0 != envelope::RULES_VERSION {
                return Err(suggestion(format!("regras {} não são as desta versão", record.rules_version.0)).into());
            }
            records.push(record);
        }
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let edition = self.selection_edition(key, run, &catalog, &input)?;
        let cited: Vec<&SuggestionRecord> = records.iter().collect();
        evidence::check_selection(&input, nodes, &cited, &edition)?;
        let record = SelectionRecord::new(edition_origin(&catalog)?, envelope::RULES_VERSION, input, &cited);
        let bytes = record.to_bytes()?;
        let sha256 = Sha256Hex::try_from(hash(&bytes))?;
        Ok(AcceptedSelection { record, bytes, sha256 })
    }

    /// What the edition says of the sources a selection names: the catalog's data, as a suggestion
    /// gives them (a file head by the envelope's rule), and the graph's label, as the page shows it.
    fn selection_edition(&self, key: &str, run: &str, catalog: &Catalog, input: &SelectionInput) -> Result<SelectionEdition> {
        let graph: v2::Graph = serde_json::from_slice(&self.graph(key, run)?)?;
        let mut edition = SelectionEdition::default();
        let ids = [&input.included, &input.removed, &input.left_out, &input.not_joined].into_iter().flatten().map(|e| e.id.as_str());
        for id in ids {
            let source = match id.strip_prefix(envelope::HEADER_PREFIX) {
                Some(path) => envelope::header(catalog, path).and_then(|(start, end)| {
                    let source = catalog.source_by_path(path)?;
                    let bytes = String::from_utf8_lossy(catalog.source_bytes(&source.path)?).lines().take(end).map(|l| l.len() + 1).sum();
                    Some(evidence::EditionSource { name: path.to_owned(), path: path.to_owned(), kind: "header".into(), start_line: start, end_line: end, bytes })
                }),
                None => catalog.symbols().get(id).map(|symbol| {
                    let chunk = &catalog.chunks()[&symbol.chunk_id];
                    evidence::EditionSource { name: symbol.qualified_name.clone(), path: catalog.sources()[&symbol.source_id].path.clone(), kind: symbol.kind.clone(),
                        start_line: chunk.start_line, end_line: chunk.end_line, bytes: chunk.end_byte - chunk.start_byte }
                }),
            };
            if let Some(source) = source {
                edition.sources.insert(id.to_owned(), source);
            }
            if let Some(node) = graph.nodes.get(id) {
                edition.labels.insert(id.to_owned(), (node.label.clone(), node.kind.clone()));
            }
        }
        Ok(edition)
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
        // The measure is read once: what the selection uses and the hash the record names come from
        // the same bytes.
        self.suggest_with(key, run, nodes, self.latest_measure_read(key, run))
    }

    /// A suggestion over a measure already read. A file changed after that read is not what the
    /// record names; the check before answering finds it, and nothing is answered.
    fn suggest_with(&self, key: &str, run: &str, nodes: &[String], reading: Result<Option<crate::execution::ReadExecution>>) -> Result<Value> {
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let hot = match &reading {
            Ok(Some(read)) => self.hotspots_of(key, run, &read.record).ok(),
            _ => None,
        };
        let (found, _, _, rules) = self.inventory_with_rules(key, run)?;
        let (mut out, measured) = Self::suggestion_answer(&catalog, &found, nodes, hot.as_ref(), envelope::BUDGET_BYTES, MAX_GATHERED)?;
        let facts = self.facts(key, run)?;
        let current: Vec<&Value> = facts["facts"].as_array().into_iter().flatten().filter(|f| f["status"] == "current").collect();
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
        let read = measure_read(&reading, measured)?;
        let builtin = inventory::builtin();
        let entries = |origin, set: &inventory::RuleSet| set.rules.iter().map(|rule| Ok(InventoryRuleEntry { origin, rule: serde_json::to_value(rule)? })).collect::<Result<Vec<_>>>();
        let mut applied = entries(InventoryRuleOrigin::Builtin, &builtin)?;
        applied.extend(entries(InventoryRuleOrigin::Project, &rules)?);
        let record = SuggestionRecord::new(
            edition_origin(&catalog)?,
            envelope::RULES_VERSION,
            SuggestionInput { nodes: nodes.to_vec(), budget_bytes: envelope::BUDGET_BYTES, max_items: MAX_GATHERED },
            SuggestionInventoryRules::new(inventory::VERSION, applied)?,
            read,
            &out,
        )?;
        let bytes = record.to_bytes()?;
        let sha256 = hash(&bytes);
        publish_immutable(&self.run_dir(key, run)?.join(SUGGESTIONS), &format!("{sha256}.json"), &bytes)?;
        // Only what can be read back and checked is answered as recorded.
        self.verify_suggestion(key, run, &sha256)?;
        out["suggestion"] = json!(sha256);
        Ok(out)
    }

    /// What a suggestion answers, but for the facts of each item: the selection over the edition, with
    /// the inventory found and the measure read (`hot`), its coverage told. It reads no acervo and
    /// writes nothing. True when the measure was used.
    fn suggestion_answer(catalog: &Catalog, found: &inventory::Inventory, nodes: &[String], hot: Option<&Value>, budget: usize, max_items: usize) -> Result<(Value, bool)> {
        let measure = hot.and_then(Self::measure_of);
        let mut candidates = envelope::candidates(catalog, nodes, measure.as_ref())?;
        envelope::trait_impls(catalog, found, nodes, measure.as_ref(), &mut candidates);
        let mut selection = envelope::select(catalog, &candidates, measure.as_ref(), budget, max_items);
        selection.behavior = envelope::behavior(catalog, found, &selection);
        selection.notes = envelope::notes(&selection);
        let mut out = serde_json::to_value(&selection)?;
        crate::instrument::annotate_suggestion(&mut out, hot.map(|h| &h["coverage"]));
        Ok((out, measure.is_some()))
    }

    /// H4 of a question: each suggestion its selection cites, made again by this code from what its
    /// record names, and compared with the record but for the facts. Told, never enforced, and it
    /// writes nothing: no suggestion, no question, no record, nothing in the acervo.
    pub fn reproduce(&self, key: &str, run: &str, turn: &str) -> Result<evidence::Reproduction> {
        let dir = self.turn_dir(key, run, turn)?;
        // An older question has no package, and a package with no selection no suggestion behind it.
        let Ok(bytes) = fs::read(dir.join(EVIDENCE)) else { return Ok(evidence::Reproduction::NotApplicable {}) };
        let package: EvidencePackage = serde_json::from_slice(&bytes)?;
        let Some(selection) = &package.selection else { return Ok(evidence::Reproduction::NotApplicable {}) };
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let mut replays = vec![];
        for sha256 in &selection.suggestions {
            replays.push(match self.replay_suggestion(key, run, &catalog, sha256)? {
                None => evidence::SuggestionReplay::Reproduced { suggestion: sha256.clone() },
                Some(reason) => evidence::SuggestionReplay::NotReplayable { suggestion: sha256.clone(), reason },
            });
        }
        Ok(evidence::Reproduction::of(replays, &package.versions.builder))
    }

    /// What `GET /api/evidence` answers for one question of one run: what was recorded with it, as
    /// recorded (its mark, its package, its selection's record), and what is checked now: H1 to H3 as
    /// a send checks them, H4 as made again, and the latest measure of the edition, which no package
    /// records. It reads only. A file that cannot be read is null here and refused by the check that
    /// needs it; it is never taken for a question with nothing to check. H4 tells whether the
    /// suggestions come again from their references, not whether they were true or enough.
    pub fn evidence(&self, key: &str, run: &str, turn: &str) -> Result<Value> {
        let dir = self.turn_dir(key, run, turn)?;
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json"))?)?;
        let recorded = |name: &str| fs::read(dir.join(name)).ok().and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok()).unwrap_or(Value::Null);
        let check = |status: &str, why: Option<String>| json!({ "status": status, "why": why });
        let not_run = || check("not_run", None);
        // H1 and H2 as the package's own check gives them, H3 as the send's check gives it after them.
        let checks = match question.get("evidence") {
            None => {
                let older = Some("pergunta anterior às marcas: não tem pacote para conferir".to_owned());
                json!({ "h1": check("not_applicable", older.clone()), "h2": check("not_applicable", older.clone()), "h3": check("not_applicable", older) })
            }
            Some(mark) => match self.check_package(key, run, &dir, &question, mark) {
                Err(e) if e.to_string().starts_with("H2:") => json!({ "h1": check("passed", None), "h2": check("refused", Some(e.to_string())), "h3": not_run() }),
                Err(e) => json!({ "h1": check("refused", Some(e.to_string())), "h2": not_run(), "h3": not_run() }),
                Ok(_) => json!({ "h1": check("passed", None), "h2": check("passed", None), "h3": match self.verify_for_send(key, run, turn) {
                    Ok(_) => check("passed", None),
                    Err(e) => check("refused", Some(e.to_string())),
                } }),
            },
        };
        // H4 needs the package read as one; when it cannot be, there is no H4 to tell, and H1 says why.
        let h4 = self.reproduce(key, run, turn).ok().map(serde_json::to_value).transpose()?.unwrap_or(Value::Null);
        let latest = match self.latest_measure_read(key, run) {
            Ok(Some(read)) => json!({ "read_status": "ok", "execution": read.record["id"], "execution_sha256": read.sha256 }),
            Ok(None) => json!({ "read_status": "none" }),
            Err(_) => json!({ "read_status": "unreadable" }),
        };
        // What a recorded field does not show although it seems to: a format 1 package's
        // `measure_diverges` was always written false, without comparing anything, so its false
        // proves nothing. Format 2 records `measure_divergence`, worked out, and needs no warning.
        let package = recorded(EVIDENCE);
        let warnings: Vec<&str> = if package.get("format_version") == Some(&json!(1)) { vec!["measure_diverges_not_computed"] } else { vec![] };
        Ok(json!({ "turn": turn, "mark": question.get("evidence").cloned().unwrap_or(Value::Null), "package": package,
            "selection": recorded(SELECTION), "checks": checks, "h4": h4, "latest_measure": latest, "warnings": warnings }))
    }

    /// One suggestion made again, or why not: its record, then the rules it names, then the
    /// execution it read, then the suggestion itself.
    fn replay_suggestion(&self, key: &str, run: &str, catalog: &Catalog, sha256: &Sha256Hex) -> Result<Option<evidence::NotReplayable>> {
        use evidence::NotReplayable as Not;
        let run_dir = self.run_dir(key, run)?;
        let read = fs::read(run_dir.join(SUGGESTIONS).join(format!("{}.json", sha256.as_str())));
        let Some(record) = read.ok().and_then(|bytes| SuggestionRecord::from_bytes(&bytes, sha256.as_str()).ok()) else { return Ok(Some(Not::SuggestionMissing)) };
        // The rules: of the selection, and of the inventory, the built-in ones as this code has them.
        let builtin = inventory::builtin();
        let entries = |origin, set: &inventory::RuleSet| set.rules.iter().map(|rule| Ok(InventoryRuleEntry { origin, rule: serde_json::to_value(rule)? })).collect::<Result<Vec<_>>>();
        let builtin_entries = entries(InventoryRuleOrigin::Builtin, &builtin)?;
        let recorded_builtin: Vec<&InventoryRuleEntry> = record.inventory_rules.rules.iter().filter(|r| r.origin == InventoryRuleOrigin::Builtin).collect();
        if record.rules_version.0 != envelope::RULES_VERSION || record.inventory_rules.version != inventory::VERSION || recorded_builtin.into_iter().ne(builtin_entries.iter()) {
            return Ok(Some(Not::RulesVersionDiffers));
        }
        // The execution read then, with the bytes it had then.
        let hot = match &record.measure {
            SuggestionMeasure::Ok { execution, execution_sha256 } => {
                let Ok(bytes) = fs::read(run_dir.join("executions").join(safe_id(execution)?).join("execution.json")) else { return Ok(Some(Not::ExecutionMissing)) };
                if hash(&bytes) != execution_sha256.as_str() {
                    return Ok(Some(Not::ExecutionDiffers));
                }
                self.hotspots_of(key, run, &serde_json::from_slice(&bytes)?).ok()
            }
            SuggestionMeasure::NoMeasure {} | SuggestionMeasure::Unreadable {} => None,
        };
        // The project's rules as the record copied them, not as the profile has them now.
        let project: Vec<&InventoryRuleEntry> = record.inventory_rules.rules.iter().filter(|r| r.origin == InventoryRuleOrigin::Project).collect();
        let project_rules: inventory::RuleSet = serde_json::from_value(json!({ "rules": project.iter().map(|r| &r.rule).collect::<Vec<_>>() }))?;
        let found = inventory::inventory(catalog, &project_rules)?;
        let (mut out, measured) = Self::suggestion_answer(catalog, &found, &record.input.nodes, hot.as_ref(), record.input.budget_bytes, record.input.max_items)?;
        // The facts of each item are the record's, as given: not made again.
        let recorded_facts = |id: &Value| record.output.items.iter().chain(&record.output.left_out).find(|i| i["id"] == *id).map_or(json!([]), |i| i["facts"].clone());
        for list in ["items", "left_out"] {
            for value in out[list].as_array_mut().into_iter().flatten() {
                value["facts"] = recorded_facts(&value["id"]);
            }
        }
        let measure = match (&record.measure, measured) {
            (SuggestionMeasure::Ok { .. }, true) => record.measure.clone(),
            (SuggestionMeasure::Ok { .. }, false) | (SuggestionMeasure::Unreadable {}, _) => SuggestionMeasure::Unreadable {},
            (SuggestionMeasure::NoMeasure {}, _) => SuggestionMeasure::NoMeasure {},
        };
        let mut applied = builtin_entries;
        applied.extend(project.into_iter().cloned());
        let again = SuggestionRecord::new(edition_origin(catalog)?, envelope::RULES_VERSION, record.input.clone(), SuggestionInventoryRules::new(inventory::VERSION, applied)?, measure, &out)?;
        Ok((!evidence::same_but_facts(&record, &again)).then_some(Not::OutputDiffers))
    }

    fn execution_sha256(&self, key: &str, run: &str, execution: &str) -> Result<Sha256Hex> {
        let file = self.run_dir(key, run)?.join("executions").join(safe_id(execution)?).join("execution.json");
        Ok(Sha256Hex::try_from(hash(&fs::read(file)?))?)
    }

    /// A suggestion's record, if it is what its name says and still fits this run: the bytes are the
    /// ones the hash names, the edition is the run's, and the execution it used has the same bytes.
    pub(crate) fn verify_suggestion(&self, key: &str, run: &str, sha256: &str) -> Result<SuggestionRecord> {
        let sha256 = Sha256Hex::try_from(sha256.to_owned())?;
        let file = self.run_dir(key, run)?.join(SUGGESTIONS).join(format!("{}.json", sha256.as_str()));
        let record = SuggestionRecord::from_bytes(&fs::read(file)?, sha256.as_str())?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        if record.edition_origin != edition_origin(&catalog)? {
            return Err(format!("Sugestão {} é de outra edition", sha256.as_str()).into());
        }
        if let SuggestionMeasure::Ok { execution, execution_sha256 } = &record.measure {
            if self.execution_sha256(key, run, execution)? != *execution_sha256 {
                return Err(format!("Sugestão {}: a execução {execution} mudou", sha256.as_str()).into());
            }
        }
        Ok(record)
    }

    /// Steps 2 and 3: the operation inventory of a run's edition, with the built-in rules and
    /// those of the profile's `inventory`, read from the file at each use like its commands.
    pub fn inventory(&self, key: &str, run: &str) -> Result<Value> {
        let (found, profile, _) = self.inventory_of(key, run)?;
        Ok(json!({ "profile": display(&profile), "inventory": found }))
    }

    pub(crate) fn inventory_of(&self, key: &str, run: &str) -> Result<(inventory::Inventory, PathBuf, Catalog)> {
        let (found, profile, catalog, _) = self.inventory_with_rules(key, run)?;
        Ok((found, profile, catalog))
    }

    /// The inventory with the project's rules it applied, as they were read for it.
    fn inventory_with_rules(&self, key: &str, run: &str) -> Result<(inventory::Inventory, PathBuf, Catalog, inventory::RuleSet)> {
        let root = PathBuf::from(self.settings(key)?.root);
        let profile = root.join(PROFILE);
        let rules: inventory::RuleSet = match fs::read(&profile).ok().map(|b| serde_json::from_slice::<Value>(&b)).transpose()? {
            Some(p) if p.get("inventory").is_some() => serde_json::from_value(p["inventory"].clone()).map_err(|e| format!("{PROFILE}: inventory inválido: {e}"))?,
            _ => inventory::RuleSet::default(),
        };
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let found = inventory::inventory(&catalog, &rules)?;
        Ok((found, profile, catalog, rules))
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
        let from_query_sha256 = verified.link()["query_sha256"].clone();
        let context = verified.into_context().ok_or("Só perguntas em claims continuam em cadeia")?;
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
        // The functions the new question names by qualified name come too: asking to go deeper
        // into `World::intersect_world` must bring its declaration, not only what was answered.
        let mut named = vec![];
        for symbol in crate::questions::named_functions(question, &catalog) {
            let target = target_of(&catalog, catalog.symbols(), &symbol)?;
            if !targets.contains(&target) {
                named.push(target.clone());
                targets.push(target);
            }
        }
        let reviewed = self.reviewed_about(key, run, &targets)?;
        // G6d: on editions that record impl types and field writes, a method target also brings
        // its owner type and the declarations that write the fields it reads.
        let owner_context = catalog.owner_context();
        let mut chain = json!({ "from_turn": from, "from_query_sha256": from_query_sha256, "answers": answers,
            "targets": targets, "reviewed_facts": reviewed.iter().map(|f| &f.id).collect::<Vec<_>>(),
            "owner_context": owner_context });
        if !named.is_empty() {
            chain["named_in_question"] = json!(named);
        }
        let request = request(&settings, question, Some(targets), reviewed, owner_context);
        self.new_turn(key, run, &settings, &catalog, Asked::client(question), None, request, Some(("chain", chain)))
    }

    /// Fixes the criteria, prepares and verifies the bundle and, in claims, its claims files.
    /// A refusal is kept with the question. Without `search`, the prepared report is recorded.
    /// A question that reaches the preparation gets its evidence package, or an error marked on it
    /// that keeps it from being sent; a question refused before has neither.
    #[allow(clippy::too_many_arguments)]
    fn new_turn(&self, key: &str, run: &str, settings: &Settings, catalog: &Catalog, asked: Asked,
        search: Option<Vec<u8>>, request: Request, origin: Option<(&str, Value)>) -> Result<Value> {
        self.new_turn_with(key, run, settings, catalog, asked, search, request, origin, None)
    }

    /// `new_turn` for a question asked with a selection accepted at its entry: its record is kept in
    /// the question's folder, once and for good, and the package names it by hash. A record not kept
    /// leaves the question with no package, and it is not sent.
    #[allow(clippy::too_many_arguments)]
    fn new_turn_with(&self, key: &str, run: &str, settings: &Settings, catalog: &Catalog, asked: Asked,
        search: Option<Vec<u8>>, request: Request, origin: Option<(&str, Value)>, selection: Option<&AcceptedSelection>) -> Result<Value> {
        let joined = asked.prepared();
        let question = joined.as_str();
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
        let written = prepare::write(catalog, &request_bytes, &prepared).and_then(|p| prepare::verify(catalog, &prepared).map(|_| p));
        let (composition, prepare_error) = match written {
            Ok(p) => (Some(p.composition), None),
            Err(e) => (None, Some(e.to_string())),
        };
        if search.is_none() && prepare_error.is_none() {
            fs::copy(prepared.join("search.json"), dir.join("search.json"))?;
        }
        let format = settings.answer_format;
        let claims_error = match (format, &prepare_error) {
            // Tests stand a refusal of the claims files in for one that is hard to cause.
            (AnswerFormat::Claims, None) if failing_at("claims") => Some("claims recusado no teste".to_owned()),
            (AnswerFormat::Claims, None) => {
                let policy = Policy { allow_inference: settings.allow_inference, allow_hypothesis: settings.allow_hypothesis };
                let instructions = settings.instructions.clone().unwrap_or_default();
                crate::claims::write(catalog, &dir, policy, instructions, settings.allow_patch, &settings.language, settings.compact_items).err().map(|e| e.to_string())
            }
            _ => None,
        };
        let mut record = json!({ "question": question, "created": stamp(), "prepare_error": prepare_error,
            "criteria_sha256": criteria_sha256, "answer_format": format, "claims_error": claims_error });
        // The selection's record, inside this question's folder only: a refusal before the folder
        // existed left nothing behind.
        let kept = match selection {
            Some(_) if failing_at("selection") => Err("selection_persistence_failed"),
            Some(s) => publish_immutable(&dir, SELECTION, &s.bytes).map_err(|_| "selection_persistence_failed"),
            None => Ok(()),
        };
        // The package describes what was prepared and written, so it comes after all of it.
        let package = match (&composition, &claims_error, kept) {
            (_, _, Err(code)) => Some(Err(code)),
            (Some(composition), None, Ok(())) => Some(self.evidence_package(key, run, catalog, &dir, composition, &asked, origin.as_ref(),
                settings.answer_format == AnswerFormat::Claims, settings.instructions.as_deref(), None, selection.map(|s| (&s.sha256, &s.record)), PackageFormat::WRITTEN)
                .and_then(|bytes| publish_package(&dir, &bytes))),
            _ => None,
        };
        // Where the evidence came from when nothing was searched: a chain step or a picked path.
        if let Some((field, value)) = origin {
            record[field] = value;
        }
        // What the client asked and what the server added, kept apart: the package has their hashes.
        record["asked"] = json!({ "original": asked.original, "suffixes": asked.suffixes.iter().map(|s| json!({ "text": s.text, "rule": s.rule, "attempt": s.attempt })).collect::<Vec<_>>() });
        if let Some(package) = &package {
            record["evidence"] = match package {
                Ok(sha256) => json!({ "format_version": u32::from(PackageFormat::WRITTEN), "sha256": sha256 }),
                Err(code) => json!({ "error": code }),
            };
        } else if claims_error.is_some() {
            // No claims query, no package for it: the mark says so, and the question is not taken
            // for an older one.
            record["evidence"] = json!({ "error": "claims_not_prepared" });
        }
        fs::write(dir.join("question.json"), serde_json::to_vec_pretty(&record)?)?;
        if let Some(Err(code)) = package {
            return Err(format!("A pergunta {id} não tem pacote de evidências ({code}); ela não será enviada").into());
        }
        self.turn(key, run, &id)
    }

    /// The bytes of the evidence package of a question just prepared, from what was written for it,
    /// or the stable code of what failed: nothing half made comes out. `format` is the package's
    /// format: a new question's is the one written now; a package made again keeps the one it has.
    #[allow(clippy::too_many_arguments)]
    fn evidence_package(&self, key: &str, run: &str, catalog: &Catalog, dir: &Path, composition: &evidence::Composition, asked: &Asked,
        origin: Option<&(&str, Value)>, claims: bool, project_instructions: Option<&[String]>, at_question: Option<MeasureAtQuestion>,
        selection: Option<(&Sha256Hex, &SelectionRecord)>, format: PackageFormat) -> std::result::Result<Vec<u8>, &'static str> {
        // What a lacuna round added, as the question records it: those fichas are the server's.
        let additions: Vec<ServerAddition> = match origin.filter(|(field, _)| *field == "gathered").and_then(|(_, value)| value.get("server_additions")) {
            Some(recorded) => serde_json::from_value(recorded.clone()).map_err(|_| "server_additions_unreadable")?,
            None => vec![],
        };
        let mut composition = composition.clone();
        for seed in &mut composition.fichas {
            if seed.symbol_id.as_ref().is_some_and(|id| additions.iter().any(|a| a.symbol_id == *id)) {
                seed.declared_by = evidence::DeclaredBy::ServerLacuna;
            }
        }
        let composition = &composition;
        let read = |path: PathBuf| fs::read(path).map_err(|_| "artifact_unreadable");
        let prepared = dir.join("prepared");
        let request_bytes = read(prepared.join("request.json"))?;
        let search = read(prepared.join("search.json"))?;
        let dossier = read(prepared.join("bundle/dossier.json"))?;
        let bundle_query = String::from_utf8(read(prepared.join("bundle/query.json"))?).map_err(|_| "artifact_unreadable")?;
        let (context_bytes, claims_query) = if claims {
            (Some(read(dir.join(crate::claims::DIR).join("context.json"))?), Some(String::from_utf8(read(dir.join(crate::claims::DIR).join("query.json"))?).map_err(|_| "artifact_unreadable")?))
        } else {
            (None, None)
        };
        let criteria = match dir.join(CRITERIA).is_file() {
            true => Some(read(dir.join(CRITERIA))?),
            false => None,
        };
        // The sources the fichas and the quotes are in, as the edition has them.
        let mut paths: Vec<&str> = composition.fichas.iter().map(|f| f.path.as_str()).collect();
        paths.extend(composition.reviewed.iter().flat_map(|f| f.citations.iter().map(|c| c.path.as_str())));
        let mut snapshots: Vec<SnapshotInput> = vec![];
        for path in paths {
            if snapshots.iter().any(|s| s.path == path) {
                continue;
            }
            let source = catalog.source_by_path(path).ok_or("snapshot_unavailable")?;
            let bytes = catalog.source_bytes(path).ok_or("snapshot_unavailable")?;
            snapshots.push(SnapshotInput { source_id: &source.id, path: &source.path, sha256: &source.sha256, bytes });
        }
        // Claims: the context's evidence is what the form is checked against.
        let context: Option<Value> = context_bytes.as_deref().map(serde_json::from_slice).transpose().map_err(|_| "claims_context_unreadable")?;
        let items: Vec<ClaimsEvidence> = match &context {
            Some(context) => context["evidence"].as_array().ok_or("claims_context_unreadable")?.iter().map(|e| Some(ClaimsEvidence {
                id: e["id"].as_str()?, path: e["path"].as_str()?, start_line: e["start_line"].as_u64()? as usize, end_line: e["end_line"].as_u64()? as usize, excerpt: e["excerpt"].as_str()?,
            })).collect::<Option<Vec<_>>>().ok_or("claims_context_unreadable")?,
            None => vec![],
        };
        let form = if claims { Form::Claims(&items) } else { Form::Prose };
        let input = CompositionInput { query_json: &bundle_query, seeds: &composition.fichas, radius: composition.radius, form, snapshots: &snapshots };
        if failing_at("composition") {
            return Err("composition_failed");
        }
        let composed = evidence::compose(&input).map_err(|_| "composition_failed")?;
        // Each reviewed fact's line in the acervo, in its exact bytes, when it is there.
        let lines = self.promotion_lines(key).map_err(|_| "acervo_unreadable")?;
        let records: Vec<FactRecord> = composition.reviewed.iter()
            .filter_map(|fact| lines.iter().find(|(id, _)| *id == fact.id).map(|(_, line)| FactRecord { fact_id: &fact.id, bytes: line }))
            .collect();
        let suffixes: Vec<SuffixInput> = asked.suffixes.iter().map(|s| SuffixInput { text: &s.text, rule: s.rule, attempt: s.attempt }).collect();
        let query_json = claims_query.as_deref().unwrap_or(&bundle_query);
        let query = evidence::build_provenance(&ProvenanceInput {
            query_json,
            question: QuestionInput { original: asked.original, suffixes: &suffixes },
            search: &search,
            composition,
            composed: &composed,
            snapshots: &snapshots,
            fact_records: &records,
            project_instructions,
        }).map_err(|_| "provenance_failed")?;
        // The measure there was when the question was made, read once: what is named is what was read.
        // A package made again keeps the one it was made with.
        let at_question = match at_question {
            Some(recorded) => recorded,
            None => match self.latest_measure_read(key, run) {
                Ok(Some(read)) => MeasureAtQuestion::Ok {
                    execution: read.record["id"].as_str().ok_or("measure_unidentified")?.to_owned(),
                    execution_sha256: Sha256Hex::try_from(read.sha256).map_err(|_| "measure_unidentified")?,
                },
                Ok(None) => MeasureAtQuestion::NoMeasure {},
                Err(_) => MeasureAtQuestion::Unreadable {},
            },
        };
        // A selection checked at the entry is named by its record, and rests on its last suggestion's
        // measure. A package made before selections were checked said only that none was tied to it.
        let gathered_origin = origin.filter(|(field, _)| *field == "gathered").map(|(_, value)| &value["origin"]);
        let (selection, selection_basis) = match selection {
            Some((sha256, record)) => (Some(SelectionReference { record_format: SelectionRecordFormat, record_sha256: sha256.clone(),
                suggestions: record.suggestions(), declared_by: ServerDeclared::Server }), record.selection_basis()),
            None => (None, match gathered_origin.is_some_and(|o| o.get("selection").is_some_and(|s| !s.is_null())) {
                true => MeasureSelectionBasis::NotRecorded {},
                false => MeasureSelectionBasis::NotApplicable {},
            }),
        };
        let origin_kind = gathered_origin.and_then(|o| o["kind"].as_str()).and_then(|kind| match kind {
            "selection" => Some(OriginKind::Selection),
            "ladder" => Some(OriginKind::Ladder),
            "hot" => Some(OriginKind::Hot),
            "loop" => Some(OriginKind::Loop),
            "operation" => Some(OriginKind::Operation),
            _ => None,
        });
        let route = match origin.map(|(field, _)| *field) {
            Some("gathered") => Route::Gathered,
            Some("chain") => Route::Chain,
            Some("path") => Route::Path,
            _ => Route::Search,
        };
        let manifest = catalog.manifest();
        let report: Value = serde_json::from_slice(&search).map_err(|_| "artifact_unreadable")?;
        let sha = |bytes: &[u8]| Sha256Hex::try_from(hash(bytes)).map_err(|_| "artifact_unreadable");
        let package = EvidencePackage {
            kind: PackageKind::EvidencePackage,
            format_version: format,
            versions: Versions { normalization: Normalization::LinesV1, rules: RulesVersion(envelope::RULES_VERSION.to_owned()), builder: crate::builder::identity() },
            edition: EditionIdentity {
                origin: edition_origin(catalog).map_err(|_| "edition_unreadable")?,
                catalog_sha256: Sha256Hex::try_from(report["catalog_sha256"].as_str().unwrap_or_default().to_owned()).map_err(|_| "edition_unreadable")?,
                extractor: manifest.extractor.clone(),
                git_head: manifest.git.head.clone(),
                git_dirty: manifest.git.dirty,
            },
            route,
            origin_kind,
            representation: if claims { Representation::Claims } else { Representation::Prose },
            fichas: composed.fichas,
            units: composed.units,
            query,
            // Format 1 wrote false and compared nothing; it is made again as it was. Format 2 compares.
            measure: match format {
                PackageFormat::V1 => MeasureReferences { at_question, selection_basis, measure_diverges: Some(false), measure_divergence: None },
                PackageFormat::V2 => {
                    let divergence = evidence::measure_divergence(&at_question, &selection_basis);
                    MeasureReferences { at_question, selection_basis, measure_diverges: None, measure_divergence: Some(divergence) }
                }
            },
            selection,
            server_additions: additions,
            bound: BoundArtifacts {
                request_sha256: sha(&request_bytes)?,
                search_sha256: sha(&search)?,
                dossier_sha256: sha(&dossier)?,
                bundle_query_sha256: sha(bundle_query.as_bytes())?,
                claims_context_sha256: context_bytes.as_deref().map(sha).transpose()?,
                criteria_sha256: criteria.as_deref().map(sha).transpose()?,
            },
        };
        let mut bytes = serde_json::to_vec_pretty(&package).map_err(|_| "serialization_failed")?;
        bytes.push(b'\n');
        Ok(bytes)
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
        let (query, link) = (verified.query().to_path_buf(), verified.link().clone());
        let context = verified.into_context().ok_or("Pergunta em claims sem contexto")?;
        let send = next_send(&dir, provider)?;
        fs::create_dir(&send)?;
        // Preserved as the Claude CLI send does: the query delivered and the answer received.
        fs::copy(&query, send.join("query.json"))?;
        fs::write(send.join("response.txt"), text)?;
        let mut summary = json!({ "state": "completed", "provider": provider, "text": text, "directory": display(&send),
            "link": link, "pasted": provider == "manual", "purpose": "question" });
        summary["check"] = crate::claims::check(&context, &send, text)?;
        // Not recorded is not linked: the answer stands only once its send.json is written.
        if failing_at("send_record") {
            return Err("O registro do envio não foi gravado; a resposta não está vinculada".into());
        }
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
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json"))?)?;
        // A new question carries a mark: its package passes H1 and H2 here, H3 below. An older one
        // has none, and keeps the checks it always had. A package and its mark, both taken away,
        // make a new question look older: hashes show consistency, not who wrote a file.
        let evidence = match question.get("evidence") {
            Some(mark) => Some(self.check_package(key, run, &dir, &question, mark)?),
            None => None,
        };
        // H3 starts here for a new question: the query made again from the bundle, byte for byte.
        let h3 = |e: Box<dyn std::error::Error>| -> Box<dyn std::error::Error> {
            match evidence { Some(_) => format!("H3: {e}").into(), None => e }
        };
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        prepare::verify(&catalog, &prepared).map_err(h3)?;
        let criteria_sha256 = question["criteria_sha256"].as_str().map(str::to_owned);
        if let Some(expected) = &criteria_sha256 {
            if hash(&fs::read(dir.join(CRITERIA))?) != *expected {
                return Err("Critérios prévios diferem dos gravados com a pergunta; nada foi enviado".into());
            }
        }
        // A claims question sends its own query, re-derived from the bundle just verified.
        let context = match question["answer_format"] == "claims" {
            true => Some(crate::claims::verify(&catalog, &dir).map_err(h3)?),
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
        // H3, the rest: the package made again by this code from what was prepared is the one stored,
        // whoever built it. Its builder and the measure taken with the question are not made again.
        if let Some((_, stored, bytes)) = &evidence {
            self.rederive_package(key, run, &catalog, &dir, &question, stored, bytes, context.as_ref())?;
        }
        let mut checks = vec!["prepare::verify"];
        checks.extend(context.is_some().then_some("claims::verify"));
        checks.extend(criteria_sha256.is_some().then_some("criteria_sha256"));
        checks.extend(evidence.as_ref().map(|_| ["evidence_h1", "evidence_h2", "evidence_h3"]).into_iter().flatten());
        let mut link = json!({
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
        // A new question's link names the package checked; an older one has none.
        if let Some((sha256, _, _)) = evidence {
            link["evidence_sha256"] = json!(sha256);
        }
        Ok(Verified { query, context, criteria_sha256, link })
    }

    /// H1 and H2 of a new question's package: the mark, the bytes, the package with itself and with
    /// the files of its turn (H1), then its units again from the edition's snapshots (H2). Nothing
    /// is rewritten; what does not agree is refused as it is.
    fn check_package(&self, key: &str, run: &str, dir: &Path, question: &Value, mark: &Value) -> Result<(String, EvidencePackage, Vec<u8>)> {
        let refuse = |level: &str, why: String| -> Box<dyn std::error::Error> { format!("{level}: {why}; nada foi enviado").into() };
        if let Some(code) = mark.get("error") {
            return Err(format!("Esta pergunta não tem pacote de evidências ({}); nada foi enviado", code.as_str().unwrap_or("?")).into());
        }
        let fields = mark.as_object().ok_or_else(|| refuse("H1", "marca do pacote ilegível".into()))?;
        // Format 1 or 2: the mark names the format, and the package has to be in that same format.
        let format = mark["format_version"].as_u64().and_then(|v| u32::try_from(v).ok()).and_then(|v| PackageFormat::try_from(v).ok());
        let Some(format) = format.filter(|_| fields.len() == 2) else {
            return Err(refuse("H1", "marca do pacote em formato desconhecido".into()));
        };
        let sha256 = Sha256Hex::try_from(mark["sha256"].as_str().unwrap_or_default().to_owned()).map_err(|_| refuse("H1", "hash da marca inválido".into()))?;
        let bytes = fs::read(dir.join(EVIDENCE)).map_err(|_| refuse("H1", "pacote ausente".into()))?;
        if hash(&bytes) != sha256.as_str() {
            return Err(refuse("H1", "o pacote não é o que a marca nomeia".into()));
        }
        let package: EvidencePackage = serde_json::from_slice(&bytes).map_err(|e| refuse("H1", format!("pacote ilegível ({e})")))?;
        let mut again = serde_json::to_vec_pretty(&package)?;
        again.push(b'\n');
        if again != bytes {
            return Err(refuse("H1", "pacote fora da forma em que é gravado".into()));
        }
        if package.format_version != format {
            return Err(refuse("H1", "o formato da marca não é o do pacote".into()));
        }
        evidence::check_structure(&package).map_err(|e| refuse("H1", e.to_string()))?;
        // The versions it names are ones this code knows; the builder is the one recorded, whatever it is.
        if package.versions.rules.0 != envelope::RULES_VERSION {
            return Err(refuse("H1", format!("regras {} desconhecidas", package.versions.rules.0)));
        }
        // The files of this turn are the ones bound, and only those.
        let of = |path: PathBuf| fs::read(path).map(|b| hash(&b)).ok();
        let claims = package.representation == Representation::Claims;
        let prepared = dir.join("prepared");
        let bound = &package.bound;
        let claims_context = of(dir.join(crate::claims::DIR).join("context.json"));
        let criteria = of(dir.join(CRITERIA));
        let checks = [
            ("request", Some(bound.request_sha256.as_str().to_owned()) == of(prepared.join("request.json"))),
            ("search", Some(bound.search_sha256.as_str().to_owned()) == of(prepared.join("search.json"))),
            ("dossier", Some(bound.dossier_sha256.as_str().to_owned()) == of(prepared.join("bundle/dossier.json"))),
            ("bundle_query", Some(bound.bundle_query_sha256.as_str().to_owned()) == of(prepared.join("bundle/query.json"))),
            // A claims question has its context, a prose one has none: a null is no stand-in for it.
            ("claims_context", claims == bound.claims_context_sha256.is_some()
                && bound.claims_context_sha256.as_ref().map(|h| h.as_str().to_owned()) == if claims { claims_context } else { None }),
            ("criteria", bound.criteria_sha256.as_ref().map(|h| h.as_str().to_owned()) == criteria && question["criteria_sha256"].as_str().map(str::to_owned) == criteria),
            ("query", Some(package.query.sha256.as_str().to_owned()) == of(if claims { dir.join(crate::claims::DIR).join("query.json") } else { prepared.join("bundle/query.json") })),
            ("representation", claims == (question["answer_format"] == "claims")),
        ];
        if let Some((name, _)) = checks.iter().find(|(_, same)| !same) {
            return Err(refuse("H1", format!("{name} difere do vinculado no pacote")));
        }
        // The question as asked, and the edition the package names.
        let asked = asked_of(question)?;
        let prepared_text = question["question"].as_str().unwrap_or_default();
        let suffixes: Vec<(String, RetryRule, usize)> = package.query.server_suffixes.iter().map(|s| (s.sha256.as_str().to_owned(), s.rule, s.attempt)).collect();
        let asked_suffixes: Vec<(String, RetryRule, usize)> = asked.1.iter().map(|s| (hash(s.text.as_bytes()), s.rule, s.attempt)).collect();
        let joined: String = std::iter::once(asked.0.as_str()).chain(asked.1.iter().map(|s| s.text.as_str())).collect();
        if package.query.prepared_question_sha256.as_str() != hash(prepared_text.as_bytes()) || joined != prepared_text
            || package.query.person_text.sha256.as_str() != hash(asked.0.as_bytes()) || suffixes != asked_suffixes {
            return Err(refuse("H1", "a pergunta registrada não é a do pacote".into()));
        }
        let origin: Value = serde_json::from_slice(&fs::read(prepared.join("bundle/origin.json"))?)?;
        let search: Value = serde_json::from_slice(&fs::read(prepared.join("search.json"))?)?;
        if origin["dossier"]["source_path"] != package.edition.origin.as_str() || search["catalog_sha256"] != package.edition.catalog_sha256.as_str() {
            return Err(refuse("H1", "a edition do pacote não é a do bundle".into()));
        }
        // The selection's record is the one the package names, and says what the question says it sent.
        let selection_file = dir.join(SELECTION);
        let included: Option<Vec<String>> = match &package.selection {
            Some(reference) => {
                let bytes = fs::read(&selection_file).map_err(|_| refuse("H1", "registro de seleção ausente".into()))?;
                let record = SelectionRecord::from_bytes(&bytes, reference.record_sha256.as_str()).map_err(|e| refuse("H1", format!("registro de seleção: {e}")))?;
                if record.suggestions() != reference.suggestions || record.selection_basis() != package.measure.selection_basis {
                    return Err(refuse("H1", "o registro de seleção não é o que o pacote diz".into()));
                }
                let sent = SelectionInput::from_value(&question["gathered"]["origin"]["selection"]).ok();
                if sent.as_ref() != Some(&record.received) || record.edition_origin != package.edition.origin {
                    return Err(refuse("H1", "a seleção da pergunta não é a do registro".into()));
                }
                Some(record.received.included.iter().map(|e| e.id.clone()).collect())
            }
            None if selection_file.exists() => return Err(refuse("H1", "registro de seleção que o pacote não nomeia".into())),
            None => None,
        };
        // What the server added is what the question records, in its order.
        let recorded: Vec<ServerAddition> = match question["gathered"].get("server_additions") {
            Some(value) => serde_json::from_value(value.clone()).map_err(|_| refuse("H1", "complementações ilegíveis".into()))?,
            None => vec![],
        };
        if recorded != package.server_additions {
            return Err(refuse("H1", "as complementações não são as da pergunta".into()));
        }
        // H2: from the edition kept with the run, never from the analyzed project.
        let catalog = self.edition(key, run).and_then(|edition| Catalog::load(&edition)).map_err(|e| refuse("H2", format!("edition indisponível ({e})")))?;
        // Each one asked for by its LACUNA and found for it in the edition, after the client's.
        let nodes: Vec<String> = serde_json::from_value(question["gathered"]["nodes"].clone()).unwrap_or_default();
        self.check_additions(key, run, &catalog, &nodes, included.as_deref(), &package.server_additions).map_err(|e| refuse("H1", format!("complementação: {e}")))?;
        let mut snapshots: Vec<SnapshotInput> = vec![];
        for unit in &package.units {
            if snapshots.iter().any(|s| s.path == unit.path) {
                continue;
            }
            let (Some(source), Some(bytes)) = (catalog.source_by_path(&unit.path), catalog.source_bytes(&unit.path)) else {
                return Err(refuse("H2", format!("snapshot de {} indisponível", unit.path)));
            };
            snapshots.push(SnapshotInput { source_id: &source.id, path: &source.path, sha256: &source.sha256, bytes });
        }
        evidence::check_sources(&package, &snapshots).map_err(|e| refuse("H2", e.to_string()))?;
        Ok((sha256.as_str().to_owned(), package, bytes))
    }

    /// H3: the package this code makes from what was prepared and recorded is the stored one, except
    /// for the code that built it and the measure read when the question was made.
    #[allow(clippy::too_many_arguments)]
    fn rederive_package(&self, key: &str, run: &str, catalog: &Catalog, dir: &Path, question: &Value, stored: &EvidencePackage,
        stored_bytes: &[u8], context: Option<&librarian_llm_engine::claims::Context>) -> Result<()> {
        let refuse = |why: String| -> Box<dyn std::error::Error> { format!("H3: {why}; nada foi enviado").into() };
        let request: Request = serde_json::from_slice(&fs::read(dir.join("prepared/request.json"))?)?;
        let composition = prepare::prepare(catalog, &request).map_err(|e| refuse(format!("preparação não refeita ({e})")))?.composition;
        let (original, suffixes) = asked_of(question)?;
        let asked = Asked { original: &original, suffixes: &suffixes };
        let origin = ["gathered", "chain", "path"].into_iter().find(|f| question.get(*f).is_some()).map(|f| (f, question[f].clone()));
        // The instructions the query was written with: the claims context records them.
        let recorded: Option<Vec<String>> = context.map(|c| c.project_instructions.clone());
        let claims = stored.representation == Representation::Claims;
        // The selection's record made again by this code, from what the client sent, the suggestion
        // records of the run and the edition: checked as at the entry, against the client's nodes.
        let selection = match &stored.selection {
            Some(reference) => {
                let bytes = fs::read(dir.join(SELECTION))?;
                let record = SelectionRecord::from_bytes(&bytes, reference.record_sha256.as_str())?;
                let nodes: Vec<String> = record.received.included.iter().map(|e| e.id.clone()).collect();
                let again = self.accept_input(key, run, &nodes, record.received.clone()).map_err(|e| refuse(format!("registro de seleção não refeito ({e})")))?;
                if again.bytes != bytes {
                    return Err(refuse("o código atual não refaz o registro de seleção".into()));
                }
                Some(again)
            }
            None => None,
        };
        let bytes = self.evidence_package(key, run, catalog, dir, &composition, &asked, origin.as_ref(), claims, recorded.as_deref(), Some(stored.measure.at_question.clone()),
            selection.as_ref().map(|s| (&s.sha256, &s.record)), stored.format_version)
            .map_err(|code| refuse(format!("pacote não refeito ({code})")))?;
        // The builder is the one field allowed to differ: what it built is compared byte for byte.
        let mut again: EvidencePackage = serde_json::from_slice(&bytes)?;
        again.versions.builder = stored.versions.builder.clone();
        let mut again_bytes = serde_json::to_vec_pretty(&again)?;
        again_bytes.push(b'\n');
        if again_bytes == stored_bytes {
            return Ok(());
        }
        // Different bytes: the first part that differs names the refusal.
        let parts: [(&str, bool); 10] = [
            ("versions", again.versions == stored.versions),
            ("edition", again.edition == stored.edition),
            ("route", (again.route, again.origin_kind, again.representation) == (stored.route, stored.origin_kind, stored.representation)),
            ("fichas", again.fichas == stored.fichas),
            ("units", again.units == stored.units),
            ("query", again.query == stored.query),
            ("measure", again.measure == stored.measure),
            ("selection", again.selection == stored.selection),
            ("server_additions", again.server_additions == stored.server_additions),
            ("bound", again.bound == stored.bound),
        ];
        let part = parts.iter().find(|(_, same)| !same).map_or("a forma", |(part, _)| part);
        Err(refuse(format!("o código atual não refaz {part} do pacote")))
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
        let summary = crate::llm::send_lexicon(provider, &send, &dir.join("query.json"), &dir.join("send"))?;
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

/// The measure a suggestion used, from the one read it was made from. No measure at all and one
/// that could not be read, or not used, are different things; only one used is named, by the hash of
/// the bytes it was read from.
fn measure_read(reading: &Result<Option<crate::execution::ReadExecution>>, used: bool) -> Result<SuggestionMeasure> {
    Ok(match reading {
        Ok(None) => SuggestionMeasure::NoMeasure {},
        Ok(Some(read)) if used => {
            let execution = read.record["id"].as_str().ok_or("Medição sem execução identificada: sugestão não registrada")?;
            SuggestionMeasure::Ok { execution: execution.to_owned(), execution_sha256: Sha256Hex::try_from(read.sha256.clone())? }
        }
        Ok(Some(_)) | Err(_) => SuggestionMeasure::Unreadable {},
    })
}

#[cfg(test)]
thread_local! {
    /// The stage at which a test makes the package of the next question fail, on this thread.
    static FAIL_EVIDENCE_AT: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}

/// `notes`, at any depth of what a client sent, empty or not, of any type: refused before anything is
/// created. Notes would reach the model as lines of the evidence; nothing a client writes may.
pub fn refuse_notes(value: &Value) -> Result<()> {
    let found = match value {
        Value::Object(fields) => fields.contains_key("notes") || fields.values().any(|v| refuse_notes(v).is_err()),
        Value::Array(items) => items.iter().any(|v| refuse_notes(v).is_err()),
        _ => false,
    };
    match found {
        true => Err("O campo notes não é aceito; nada foi criado nem enviado".into()),
        false => Ok(()),
    }
}

/// A failure tests cause at one stage, for one that is hard to cause on purpose.
pub(crate) fn failing_at(stage: &str) -> bool {
    #[cfg(test)]
    return FAIL_EVIDENCE_AT.with(|f| f.get() == Some(stage));
    #[cfg(not(test))]
    {
        let _ = stage;
        false
    }
}

/// The question as asked, from question.json: the client's text and what the server added. A
/// question with no record of that is its own text, and nothing added.
fn asked_of(question: &Value) -> Result<(String, Vec<Suffix>)> {
    let Some(asked) = question.get("asked") else {
        return Ok((question["question"].as_str().unwrap_or_default().to_owned(), vec![]));
    };
    let original = asked["original"].as_str().ok_or("Pergunta com registro do texto ilegível")?.to_owned();
    let suffixes = asked["suffixes"].as_array().ok_or("Pergunta com registro dos acréscimos ilegível")?.iter().map(|s| {
        Ok(Suffix {
            text: s["text"].as_str().ok_or("Acréscimo ilegível")?.to_owned(),
            rule: serde_json::from_value(s["rule"].clone())?,
            attempt: s["attempt"].as_u64().ok_or("Acréscimo ilegível")? as usize,
        })
    }).collect::<Result<Vec<_>>>()?;
    Ok((original, suffixes))
}

/// Publishes a package beside its question, once, and gives its hash.
fn publish_package(dir: &Path, bytes: &[u8]) -> std::result::Result<String, &'static str> {
    if failing_at("persistence") {
        return Err("persistence_failed");
    }
    publish_immutable(dir, EVIDENCE, bytes).map_err(|_| "persistence_failed")?;
    Ok(hash(bytes))
}

/// An edition's identity, as a prepared bundle's `origin.json` names it.
fn edition_origin(catalog: &Catalog) -> Result<String> {
    Ok(format!("edition:{}", hash(&serde_json::to_vec(catalog.manifest())?)))
}

/// Publishes `bytes` under `name`, once and for good. A hard link is how: it fails when the name
/// exists, on every system, where a rename may replace it; and the name only ever appears over a file
/// already whole and on disk. The temporary file's name is never a record's. Equal bytes already
/// there are the same record; different ones are an error and stay as they are.
fn publish_immutable(dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    publish_with(dir, name, bytes, |file, bytes| file.write_all(bytes))
}

fn publish_with(dir: &Path, name: &str, bytes: &[u8], write: impl FnOnce(&mut fs::File, &[u8]) -> io::Result<()>) -> Result<()> {
    static NEXT_TEMPORARY: AtomicUsize = AtomicUsize::new(0);
    fs::create_dir_all(dir)?;
    let target = dir.join(name);
    let same = |target: &Path| -> Result<()> {
        if fs::read(target)? != bytes {
            return Err(format!("{}: já existe com outros bytes; nada foi sobrescrito", display(target)).into());
        }
        Ok(())
    };
    if target.exists() {
        return same(&target);
    }
    let temporary = dir.join(format!("{name}.{}-{}.tmp", std::process::id(), NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)));
    let written = (|| {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        write(&mut file, bytes)?;
        file.sync_all()?;
        drop(file);
        fs::hard_link(&temporary, &target)
    })();
    // Whatever happened, the temporary file has no more use: on failure, nothing valid is left.
    let _ = fs::remove_file(&temporary);
    match written {
        Ok(()) => {}
        // Another writer published first: its bytes have to be these.
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(format!("{}: não foi possível publicar: {error}", display(&target)).into()),
    }
    same(&target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_ingest::evidence::{Incomparable, MeasureDivergence, MeasureSide, NotComparable};

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

    /// A project in git with these files and profile, opened, and with no other source.
    fn project_with(files: &[(&str, &str)], profile: Value) -> (Scratch, Store, String, PathBuf) {
        let scratch = project();
        let root = scratch.0.join("project");
        fs::remove_file(root.join("src/lib.rs")).unwrap();
        for (path, text) in files {
            fs::write(root.join(path), text).unwrap();
        }
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        (scratch, store, key, root)
    }

    fn approve(store: &Store, key: &str, name: &str) {
        let commands = store.commands(key).unwrap();
        let sha = commands["commands"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap()["sha256"].as_str().unwrap().to_owned();
        store.approve_command(key, name, &sha).unwrap();
    }

    const PY_APP: &str = "def demo():\n    return 1\n";
    const PY_PRINTED: &str = "librarian-measure calls=7 at=src/app.py:1\nlibrarian-measure calls=2 at=src/app.py:2\nlibrarian-measure outro=1\nlibrarian-measure calls=5\nlibrarian-measure tempo=3 at=src/app.py:1\n";

    #[test]
    fn with_no_source_it_can_go_in_declaring_the_statement_is_not_offered_as_the_way_out() {
        let python = [("src/app.py", PY_APP)];
        // Python alone, with no statement declared or with an invalid one: the source is what is missing.
        for profile in [json!({ "project_root": "." }), json!({ "project_root": ".", "instrument": { "statement": "count()", "command": "c" } })] {
            let (_scratch, store, key, _root) = project_with(&python, profile);
            let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
            assert_eq!(store.measure_all(&key, Some(&run)).unwrap(), json!({ "available": false, "reason": "no_source" }));
            // With no edition to look at, or one that cannot be read, nothing is known about its sources.
            assert_eq!(store.measure_all(&key, None).unwrap()["reason"], "no_instrument");
            assert_eq!(store.measure_all(&key, Some("nao-existe")).unwrap()["reason"], "no_instrument");
        }
        // With a Rust source, declaring it is the way out, in a mixed project too.
        let (_scratch, store, key, _root) = project_with(&[("src/app.py", PY_APP), ("src/lib.rs", "pub fn a() {}\n")], json!({ "project_root": "." }));
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        assert_eq!(store.measure_all(&key, Some(&run)).unwrap()["reason"], "no_instrument");
    }

    #[test]
    fn manual_counts_are_told_apart_and_stay_out_of_the_measure() {
        const NO_MEASURE: &str = "esta edition não tem essa medição";
        let profile = json!({ "project_root": ".", "commands": {
            "manual": { "argv": ["git", "grep", "-h", "librarian-measure", "--", "src/printed.txt"], "repeat": 1 },
            "sem_contagem": { "argv": ["git", "status"], "repeat": 1 },
            "falha": { "argv": ["git", "nao-existe"], "repeat": 1 } } });
        let (_scratch, store, key, _root) = project_with(&[("src/app.py", PY_APP), ("src/printed.txt", PY_PRINTED)], profile);
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let answer = |store: &Store| store.performance_question(&key, &run, "Quais funções pesam?").unwrap();
        // Nothing counted yet: the answer of always.
        assert_eq!(store.manual_counts(&key, Some(&run)).unwrap(), json!([]));
        assert!(answer(&store)["answer"][0].as_str().unwrap().contains(NO_MEASURE));
        // A failed run and a run that counts nothing do not count.
        for name in ["falha", "sem_contagem"] {
            approve(&store, &key, name);
            let _ = store.execute(&key, &run, name, None);
        }
        assert_eq!(store.manual_counts(&key, Some(&run)).unwrap(), json!([]));
        approve(&store, &key, "manual");
        let ran = store.execute(&key, &run, "manual", None).unwrap();
        // Two lines: a count with no `at` says nothing about a line, and only `calls` is a count.
        assert_eq!(store.manual_counts(&key, Some(&run)).unwrap(), json!([{ "command": "manual", "execution": ran["id"], "lines": 2 }]));
        let said = answer(&store);
        let text = said["answer"][0].as_str().unwrap();
        assert!(!text.contains(NO_MEASURE), "{text}");
        assert!(text.contains("manual") && text.contains("2 linhas") && text.contains("Investigar"), "{text}");
        assert_eq!(said["needs_measure"], true);
        // They stay out of the measure: the same 404 answer, and nothing measured for the selection.
        assert!(store.latest_measure(&key, &run).unwrap().is_none());
        assert!(store.hotspots(&key, &run).unwrap_err().to_string().starts_with("Ainda não há medição"));
        let demo = symbol_id(&store, &key, &run, "demo");
        let suggested = store.suggest_envelope(&key, &run, std::slice::from_ref(&demo)).unwrap();
        assert_eq!(suggested["measured"], false);
        assert!(suggested["coverage"].is_null(), "{suggested:#}");
        // Another run has none of them, and with no run there is nothing to say.
        let other = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        assert_eq!(store.manual_counts(&key, Some(&other)).unwrap(), json!([]));
        assert_eq!(store.manual_counts(&key, None).unwrap(), json!([]));
        // A run that did not succeed says nothing, and neither does a run with a change: it is not
        // what the project counts.
        let dir = store.run_dir(&key, &run).unwrap().join("executions").join(ran["id"].as_str().unwrap()).join("execution.json");
        let mut record: Value = serde_json::from_slice(&fs::read(&dir).unwrap()).unwrap();
        record["summary"]["succeeded"] = json!(0);
        fs::write(&dir, record.to_string()).unwrap();
        assert_eq!(store.manual_counts(&key, Some(&run)).unwrap(), json!([]));
        record["summary"]["succeeded"] = json!(1);
        record["change"] = json!({ "turn": "01" });
        fs::write(&dir, record.to_string()).unwrap();
        assert_eq!(store.manual_counts(&key, Some(&run)).unwrap(), json!([]));
    }

    /// The note points at Investigar, so it reads the run Investigar reads: the latest one without
    /// a change, whatever its result (`measuredOrigins`, in the page). Found in the audit of steps 6 and 7.
    #[test]
    fn manual_counts_read_the_run_investigar_shows() {
        const NO_MEASURE: &str = "esta edition não tem essa medição";
        let profile = |argv: Value| json!({ "project_root": ".", "commands": { "manual": { "argv": argv, "repeat": 1 } } });
        let good = json!(["git", "grep", "-h", "librarian-measure", "--", "src/printed.txt"]);
        let (_scratch, store, key, root) = project_with(&[("src/app.py", PY_APP), ("src/lib.rs", "pub fn a() {}
"), ("src/printed.txt", PY_PRINTED)], profile(good.clone()));
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        // Executions are ordered by when they started, to the second.
        let run_with = |argv: &Value| {
            std::thread::sleep(std::time::Duration::from_millis(1100));
            fs::write(root.join(PROFILE), profile(argv.clone()).to_string()).unwrap();
            approve(&store, &key, "manual");
            store.execute(&key, &run, "manual", None)
        };
        let answer = || store.performance_question(&key, &run, "Quais funções pesam?").unwrap()["answer"][0].as_str().unwrap().to_owned();
        let first = run_with(&good).unwrap();
        assert_eq!(store.manual_counts(&key, Some(&run)).unwrap(), json!([{ "command": "manual", "execution": first["id"], "lines": 2 }]));
        assert!(!answer().contains(NO_MEASURE));
        // The same command fails: Investigar shows none of its lines, and nothing points there.
        let _ = run_with(&json!(["git", "nao-existe"]));
        assert_eq!(store.executions(&key, &run).unwrap().len(), 2);
        assert_eq!(store.manual_counts(&key, Some(&run)).unwrap(), json!([]));
        assert!(answer().contains(NO_MEASURE), "{}", answer());
        // It counts again: the newer run is the one both read.
        let third = run_with(&good).unwrap();
        assert_eq!(store.manual_counts(&key, Some(&run)).unwrap(), json!([{ "command": "manual", "execution": third["id"], "lines": 2 }]));
        // A "Medir tudo" of the same command that fails is the latest run too, and it is not a manual count.
        fs::write(root.join(PROFILE), json!({ "project_root": ".", "commands": { "manual": { "argv": ["git", "nao-existe"], "repeat": 1 } },
            "instrument": { "statement": "crate::count!();", "command": "manual" } }).to_string()).unwrap();
        approve(&store, &key, "manual");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let _ = store.execute_instrumented(&key, &run);
        let all = store.executions(&key, &run).unwrap();
        assert_eq!((all.len(), all[3]["instrumentation"].is_null(), &all[3]["summary"]["succeeded"]), (4, false, &json!(0)));
        assert_eq!(store.manual_counts(&key, Some(&run)).unwrap(), json!([]));
    }

    /// The route keeps `measure_all` and `manual_counts` apart, and records it cannot read are not "none".
    #[test]
    fn the_commands_route_tells_unread_records_from_no_counts() {
        let profile = json!({ "project_root": ".", "commands": { "manual": { "argv": ["git", "grep", "-h", "librarian-measure", "--", "src/printed.txt"], "repeat": 1 } } });
        let (_scratch, store, key, _root) = project_with(&[("src/app.py", PY_APP), ("src/printed.txt", PY_PRINTED)], profile);
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let get = || {
            use std::io::{Read, Write};
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let mut client = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
            write!(client, "GET /api/commands?key={key}&run={run} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
            let (server, _) = listener.accept().unwrap();
            let request = crate::http::read(&server).unwrap();
            let app = crate::App { port: 0, viewer: PathBuf::new(), store: Store { dir: store.dir.clone(), modelfiles: store.modelfiles.clone() }, graph: std::sync::Mutex::new(None) };
            let response = crate::route(&app, &request).unwrap();
            crate::http::write(&server, &response).unwrap();
            drop(server);
            let mut raw = vec![];
            client.read_to_end(&mut raw).unwrap();
            let text = String::from_utf8(raw).unwrap();
            serde_json::from_str::<Value>(text.split_once("\r\n\r\n").unwrap().1).unwrap()
        };
        assert_eq!(get()["manual_counts"], json!([]));
        approve(&store, &key, "manual");
        let ran = store.execute(&key, &run, "manual", None).unwrap();
        let answer = get();
        assert_eq!(answer["manual_counts"][0]["lines"], 2);
        assert!(answer["measure_all"].get("manual_counts").is_none(), "{answer:#}");
        let file = store.run_dir(&key, &run).unwrap().join("executions").join(ran["id"].as_str().unwrap()).join("execution.json");
        fs::write(&file, "{ corrompido").unwrap();
        let answer = get();
        assert!(answer["manual_counts"].is_null(), "{answer:#}");
        assert_eq!(answer["commands"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn a_question_takes_the_same_route_before_and_after_the_measure() {
        let lib = "pub fn roda() -> u32 {\n    nao_roda()\n}\npub fn nao_roda() -> u32 {\n    1\n}\npub const P: &str = \"\nlibrarian-measure calls=7 at=src/lib.rs:1\n\";\n";
        let profile = json!({ "project_root": ".", "commands": { "contagem": { "argv": ["git", "grep", "-h", "librarian-measure", "--", "src/lib.rs"], "repeat": 1 } },
            "instrument": { "statement": "crate::count!();", "command": "contagem" } });
        let (_scratch, store, key, _root) = project_with(&[("src/app.py", PY_APP), ("src/lib.rs", lib)], profile);
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        // A function with passes, one with none, one of a language with no counter, none at all, and no name.
        let questions = ["Otimize o roda", "Otimize o nao_roda", "Otimize o demo", "Otimize o relatorio", "Quais funções pesam?", "Onde otimizar?"];
        let routes = || questions.map(|q| store.performance_question(&key, &run, q).unwrap()["route"].clone());
        let before = routes();
        approve(&store, &key, "contagem");
        store.execute_instrumented(&key, &run).unwrap();
        assert_eq!(before, routes());
        // What "Medir tudo" counted is the measure, not a manual count.
        assert_eq!(store.manual_counts(&key, Some(&run)).unwrap(), json!([]));
        for (question, route) in questions.iter().zip(&before) {
            let known = question.starts_with("Otimize o") && !question.ends_with("relatorio");
            assert_eq!(route["target"]["status"], if known { "resolved" } else { "none" }, "{question}: {route:#}");
            assert_eq!(route["intent"].is_null(), question.ends_with("relatorio"), "{question}: {route:#}");
        }
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
        assert_eq!((&link["claims"], &link["checks"]), (&Value::Null, &json!(["prepare::verify", "evidence_h1", "evidence_h2", "evidence_h3"])));
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
        assert_eq!(link["checks"], json!(["prepare::verify", "claims::verify", "evidence_h1", "evidence_h2", "evidence_h3"]));
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
    fn a_chain_step_also_brings_the_functions_its_question_names_by_qualified_name() {
        let scratch = project();
        // Two `trace`: only the qualified name settles which one.
        fs::write(scratch.0.join("project/src/lib.rs"), "pub struct Camera;\n\nimpl Camera {\n    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;\n        direction\n    }\n\n    pub fn trace(&self) -> f64 {\n        1.0\n    }\n}\n\npub struct World;\n\nimpl World {\n    pub fn trace(&self) -> f64 {\n        0.0\n    }\n}\n").unwrap();
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
        record_answer(&store, &key, &run, "01", "manual", &answer.to_string()).unwrap();
        let from_answer = json!({"path": "src/lib.rs", "symbol_name": "ray_from_pixel", "declaration_line": 4});
        let camera_trace = json!({"path": "src/lib.rs", "symbol_name": "trace", "declaration_line": 9});

        // Named whole: it joins the targets and its declaration is in the evidence.
        let step = store.continue_turn(&key, &run, "01", "Aprofunde Camera::trace e Nada::existe.").unwrap();
        assert_eq!(step["prepare_error"], Value::Null);
        assert_eq!(step["chain"]["targets"], json!([from_answer, camera_trace]));
        assert_eq!(step["chain"]["named_in_question"], json!([camera_trace]));
        assert!(step["query"].as_str().unwrap().contains("        1.0"));
        // Already a target, a bare last part or a homonym: nothing more, and no record of it.
        for question in ["Aprofunde Camera::ray_from_pixel.", "Aprofunde trace.", "E agora?"] {
            let step = store.continue_turn(&key, &run, "01", question).unwrap();
            assert_eq!(step["chain"]["targets"], json!([from_answer]), "{question}");
            assert!(step["chain"].get("named_in_question").is_none(), "{question}");
        }
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
        assert_eq!(link["checks"], json!(["prepare::verify", "criteria_sha256", "evidence_h1", "evidence_h2", "evidence_h3"]));
        let criteria = store.turn_dir(key, run, "01").unwrap().join(CRITERIA);
        fs::write(&criteria, "- Qualquer resposta serve.\n").unwrap();
        // A new question binds its criteria in its package: H1 finds them changed first.
        assert!(store.verify_for_send(key, run, "01").err().unwrap().to_string().contains("H1: criteria"));
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

    // The record of a suggestion: what it says, where it is, and what happens when it cannot be written.

    fn record_files(store: &Store, key: &str, run: &str) -> Vec<PathBuf> {
        let dir = store.run_dir(key, run).unwrap().join(SUGGESTIONS);
        let Ok(entries) = fs::read_dir(dir) else { return vec![] };
        let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        files.sort();
        files
    }

    fn only_record(store: &Store, key: &str, run: &str, out: &Value) -> (PathBuf, Vec<u8>, SuggestionRecord) {
        let files = record_files(store, key, run);
        assert_eq!(files.len(), 1, "{files:?}");
        let sha = out["suggestion"].as_str().unwrap();
        assert_eq!(files[0].file_name().unwrap().to_string_lossy(), format!("{sha}.json"));
        let bytes = fs::read(&files[0]).unwrap();
        let record = store.verify_suggestion(key, run, sha).unwrap();
        (files[0].clone(), bytes, record)
    }

    fn no_measure() -> (Scratch, Store, String, String, String) {
        let scratch = project();
        let root = scratch.0.join("project");
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let node = symbol_id(&store, &key, &run, "ray_from_pixel");
        (scratch, store, key, run, node)
    }

    #[test]
    fn a_suggestion_is_recorded_under_the_hash_of_its_bytes_and_the_record_is_what_was_answered() {
        let (_scratch, store, key, run) = measured_mix();
        let node = symbol_id(&store, &key, &run, "roda");
        let out = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        let (_, bytes, record) = only_record(&store, &key, &run, &out);
        let sha = out["suggestion"].as_str().unwrap();
        assert_eq!(hash(&bytes), sha);
        assert!(!String::from_utf8_lossy(&bytes).contains(sha), "a record holds no hash of itself");
        assert_eq!(record.to_bytes().unwrap(), bytes);
        // The output, field by field, is the answer's, and in the same order.
        let written = serde_json::to_value(&record.output).unwrap();
        for field in ["items", "left_out", "gaps", "behavior", "measured", "used_bytes", "coverage", "limits"] {
            assert_eq!(written[field], out[field], "{field}");
        }
        assert_eq!(written.as_object().unwrap().len(), 8);
        assert_eq!(serde_json::to_value(&record.input).unwrap(), json!({ "nodes": [node], "budget_bytes": envelope::BUDGET_BYTES, "max_items": MAX_GATHERED }));
        assert_eq!(out["budget_bytes"], envelope::BUDGET_BYTES);
        assert_eq!(record.rules_version.0, envelope::RULES_VERSION);
        // The same edition a prepared bundle names: the identity is the one already in use.
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        assert_eq!(record.edition_origin, edition_origin(&catalog).unwrap());
    }

    #[test]
    fn the_answer_only_gains_the_reference_to_its_record() {
        let (_scratch, store, key, run) = measured_mix();
        let node = symbol_id(&store, &key, &run, "roda");
        let out = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        let names: Vec<&String> = out.as_object().unwrap().keys().collect();
        assert_eq!(names, ["behavior", "budget_bytes", "coverage", "gaps", "items", "left_out", "limits", "measured", "notes", "suggestion", "used_bytes"]);
        // A record that left a field of the answer out would be refused, so a new one is noticed.
        let mut with_more = out.clone();
        with_more.as_object_mut().unwrap().remove("suggestion");
        with_more["novo"] = json!(1);
        let rules = SuggestionInventoryRules::new("v", vec![]).unwrap();
        let input = SuggestionInput { nodes: vec![], budget_bytes: envelope::BUDGET_BYTES, max_items: 1 };
        assert!(SuggestionRecord::new("edition:x".into(), "r", input, rules, SuggestionMeasure::NoMeasure {}, &with_more).unwrap_err().contains("novo"));
    }

    #[test]
    fn notes_are_in_the_answer_and_never_in_the_record() {
        let (_scratch, store, key, run) = measured_mix();
        let node = symbol_id(&store, &key, &run, "metodo");
        let out = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        let notes = out["notes"].as_array().unwrap();
        assert!(!notes.is_empty(), "{out:#}");
        let (_, bytes, _) = only_record(&store, &key, &run, &out);
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains("\"notes\""));
        for note in notes {
            assert!(!text.contains(note["text"].as_str().unwrap()), "{note}");
        }
    }

    #[test]
    fn the_same_suggestion_is_the_same_record_and_nothing_else_enters_its_hash() {
        let (_scratch, store, key, run, node) = no_measure();
        let one = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        let (file, bytes, _) = only_record(&store, &key, &run, &one);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let two = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        assert_eq!(one, two);
        assert_eq!(fs::read(&file).unwrap(), bytes);
        assert_eq!(record_files(&store, &key, &run).len(), 1);
        // Another run of the same project at another moment: the same edition, the same record.
        let other = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        assert_ne!(other, run);
        let elsewhere = store.suggest_envelope(&key, &other, std::slice::from_ref(&node)).unwrap();
        assert_eq!(elsewhere["suggestion"], one["suggestion"]);
        assert_eq!(fs::read(&record_files(&store, &key, &other)[0]).unwrap(), bytes);
        // Other nodes: another record, beside the first.
        let more = symbol_id(&store, &key, &run, "Camera");
        let different = store.suggest_envelope(&key, &run, std::slice::from_ref(&more)).unwrap();
        assert_ne!(different["suggestion"], one["suggestion"]);
        assert_eq!(record_files(&store, &key, &run).len(), 2);
        // The order asked for is part of what was asked.
        let both = store.suggest_envelope(&key, &run, &[node.clone(), more.clone()]).unwrap();
        let swapped = store.suggest_envelope(&key, &run, &[more.clone(), node.clone()]).unwrap();
        assert_ne!(both["suggestion"], swapped["suggestion"]);
        for (answer, nodes) in [(&both, [node.clone(), more.clone()]), (&swapped, [more, node])] {
            assert_eq!(store.verify_suggestion(&key, &run, answer["suggestion"].as_str().unwrap()).unwrap().input.nodes, nodes);
        }
    }

    #[test]
    fn a_record_says_the_measure_it_used_and_tells_none_from_unreadable() {
        // None: nothing to read.
        let (_scratch, store, key, run, node) = no_measure();
        let none = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        let (_, _, record) = only_record(&store, &key, &run, &none);
        assert_eq!(record.measure, SuggestionMeasure::NoMeasure {});
        assert_eq!(serde_json::to_value(&record.measure).unwrap(), json!({ "read_status": "none" }));
        assert_eq!(none["measured"], false);
        // Unreadable: an execution.json that does not parse. The selection is the same as with none,
        // as it always was; only the record tells them apart.
        let dir = store.run_dir(&key, &run).unwrap().join("executions/99-quebrada");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("execution.json"), b"{ nao e json").unwrap();
        let unreadable = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        let record = store.verify_suggestion(&key, &run, unreadable["suggestion"].as_str().unwrap()).unwrap();
        assert_eq!(serde_json::to_value(&record.measure).unwrap(), json!({ "read_status": "unreadable" }));
        assert_ne!(unreadable["suggestion"], none["suggestion"]);
        let without_reference = |v: &Value| { let mut v = v.clone(); v.as_object_mut().unwrap().remove("suggestion"); v };
        assert_eq!(without_reference(&unreadable), without_reference(&none));
    }

    #[test]
    fn with_a_measure_the_record_names_the_execution_used_and_the_hash_of_its_bytes() {
        let (_scratch, store, key, run) = measured_mix();
        let node = symbol_id(&store, &key, &run, "roda");
        let out = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        assert_eq!(out["measured"], true);
        let (_, _, record) = only_record(&store, &key, &run, &out);
        // The one the selection read: the hotspots name it, from the same reading of the executions.
        let used = store.hotspots(&key, &run).unwrap()["execution"].as_str().unwrap().to_owned();
        let file = store.run_dir(&key, &run).unwrap().join("executions").join(&used).join("execution.json");
        let SuggestionMeasure::Ok { execution, execution_sha256 } = &record.measure else { panic!("{:?}", record.measure) };
        assert_eq!(*execution, used);
        assert_eq!(execution_sha256.as_str(), hash(&fs::read(&file).unwrap()));
        // Another execution beside it, that is not a measure, is not the one named.
        let ran = store.execute(&key, &run, "manual", None);
        if ran.is_ok() {
            let again = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
            let SuggestionMeasure::Ok { execution, .. } = store.verify_suggestion(&key, &run, again["suggestion"].as_str().unwrap()).unwrap().measure else { panic!() };
            assert_eq!(execution, used);
        }
        // An execution changed after the fact is noticed when the record is read back.
        let mut bytes = fs::read(&file).unwrap();
        bytes.push(b'\n');
        fs::write(&file, bytes).unwrap();
        let error = store.verify_suggestion(&key, &run, out["suggestion"].as_str().unwrap()).unwrap_err().to_string();
        assert!(error.contains("mudou"), "{error}");
    }

    #[test]
    fn the_inventory_rules_can_be_brought_back_from_the_record() {
        let rule = json!({ "id": "minha", "operation": "database", "match": "call", "names": ["consulta"] });
        let (_scratch, store, key, root) = project_with(&[("src/lib.rs", "pub fn demo() -> u32 {\n    consulta()\n}\nfn consulta() -> u32 {\n    1\n}\n")], json!({ "inventory": { "rules": [rule] } }));
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let node = symbol_id(&store, &key, &run, "demo");
        let out = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        let (_, _, record) = only_record(&store, &key, &run, &out);
        assert_eq!(record.inventory_rules.version, inventory::VERSION);
        let of = |origin| -> Vec<Value> { record.inventory_rules.rules.iter().filter(|r| r.origin == origin).map(|r| r.rule.clone()).collect() };
        assert_eq!(of(InventoryRuleOrigin::Project).len(), 1);
        assert_eq!(serde_json::to_value(inventory::builtin()).unwrap()["rules"], json!(of(InventoryRuleOrigin::Builtin)));
        // Read back as rules, they give the inventory the suggestion had.
        let again: inventory::RuleSet = serde_json::from_value(json!({ "rules": of(InventoryRuleOrigin::Project) })).unwrap();
        let (found, _, catalog) = store.inventory_of(&key, &run).unwrap();
        assert_eq!(inventory::inventory(&catalog, &again).unwrap(), found);
        // The hash is of the rules, and other rules are another record.
        assert_eq!(record.inventory_rules.sha256.as_str(), hash(serde_json::to_string(&record.inventory_rules.rules).unwrap().as_bytes()));
        fs::write(root.join(PROFILE), json!({ "inventory": { "rules": [] } }).to_string()).unwrap();
        let other = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        assert_ne!(other["suggestion"], out["suggestion"]);
    }

    #[test]
    fn a_record_changed_on_disk_is_refused_and_never_overwritten() {
        let (_scratch, store, key, run, node) = no_measure();
        let out = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        let (file, bytes, _) = only_record(&store, &key, &run, &out);
        let sha = out["suggestion"].as_str().unwrap();
        // Any change of a byte.
        let mut changed = bytes.clone();
        let at = changed.len() / 2;
        changed[at] = if changed[at] == b'x' { b'y' } else { b'x' };
        fs::write(&file, &changed).unwrap();
        let error = store.verify_suggestion(&key, &run, sha).unwrap_err().to_string();
        assert!(error.contains("holds bytes of"), "{error}");
        // The route finds another file where its record goes: an error, and the file stays.
        let error = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap_err().to_string();
        assert!(error.contains("outros bytes"), "{error}");
        assert_eq!(fs::read(&file).unwrap(), changed);
        // A name that is not a hash reads nothing.
        for name in ["../../run".to_owned(), sha.to_uppercase()] {
            let error = store.verify_suggestion(&key, &run, &name).unwrap_err().to_string();
            assert!(error.contains("not a lowercase SHA-256"), "{error}");
        }
        // A record that is another record's bytes under this name.
        fs::write(&file, b"{}\n").unwrap();
        assert!(store.verify_suggestion(&key, &run, sha).is_err());
        fs::write(&file, &bytes).unwrap();
        assert!(store.verify_suggestion(&key, &run, sha).is_ok());
        // Of another edition: the run's own is what it has to be of.
        let mut record: SuggestionRecord = serde_json::from_slice(&bytes).unwrap();
        record.edition_origin = "edition:outra".into();
        let other = record.to_bytes().unwrap();
        let other_sha = hash(&other);
        fs::write(file.with_file_name(format!("{other_sha}.json")), &other).unwrap();
        let error = store.verify_suggestion(&key, &run, &other_sha).unwrap_err().to_string();
        assert!(error.contains("outra edition"), "{error}");
    }

    #[test]
    fn when_the_record_cannot_be_written_the_route_fails_and_answers_no_suggestion() {
        let (_scratch, store, key, run, node) = no_measure();
        // A file where the folder goes: nothing can be published under it.
        fs::write(store.run_dir(&key, &run).unwrap().join(SUGGESTIONS), b"um arquivo").unwrap();
        assert!(store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).is_err());
    }

    #[test]
    fn a_failure_while_writing_leaves_nothing_that_could_be_read_as_a_record() {
        let scratch = project();
        let dir = scratch.0.join("publicar");
        let error = publish_with(&dir, "a.json", b"inteiro", |file, bytes| {
            file.write_all(&bytes[..3])?;
            Err(io::Error::other("disco cheio"))
        });
        assert!(error.unwrap_err().to_string().contains("disco cheio"));
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0, "neither the name nor the temporary file stays");
        // And nothing stands in the way of doing it again.
        publish_immutable(&dir, "a.json", b"inteiro").unwrap();
        assert_eq!(fs::read(dir.join("a.json")).unwrap(), b"inteiro");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[test]
    fn publishing_never_replaces_what_is_there_even_past_the_first_look() {
        let scratch = project();
        let dir = scratch.0.join("publicar");
        publish_immutable(&dir, "a.json", b"primeiro").unwrap();
        publish_immutable(&dir, "a.json", b"primeiro").unwrap();
        assert!(publish_immutable(&dir, "a.json", b"segundo").unwrap_err().to_string().contains("outros bytes"));
        assert_eq!(fs::read(dir.join("a.json")).unwrap(), b"primeiro");
        // Another writer gets there after the first look and before the link, whatever it wrote.
        let target = dir.join("b.json");
        let error = publish_with(&dir, "b.json", b"meu", |file, bytes| {
            fs::write(&target, b"do outro")?;
            file.write_all(bytes)
        });
        assert!(error.unwrap_err().to_string().contains("outros bytes"));
        assert_eq!(fs::read(&target).unwrap(), b"do outro");
        let same = dir.join("c.json");
        publish_with(&dir, "c.json", b"igual", |file, bytes| {
            fs::write(&same, b"igual")?;
            file.write_all(bytes)
        })
        .unwrap();
        let mut names: Vec<String> = fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, ["a.json", "b.json", "c.json"], "no temporary file stays");
    }

    #[test]
    fn writers_at_the_same_time_end_with_one_whole_record() {
        let scratch = project();
        let dir = scratch.0.join("publicar");
        let bytes = vec![b'x'; 256 * 1024];
        for round in 0..16 {
            let name = format!("{round}.json");
            let results: Vec<_> = std::thread::scope(|scope| {
                let writers: Vec<_> = (0..8).map(|_| scope.spawn(|| publish_immutable(&dir, &name, &bytes).map_err(|e| e.to_string()))).collect();
                writers.into_iter().map(|w| w.join().unwrap()).collect()
            });
            assert!(results.iter().all(|r| r.is_ok()), "{results:?}");
            assert_eq!(fs::read(dir.join(&name)).unwrap(), bytes);
        }
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 16, "one file a name, no temporary file");
    }

    #[test]
    fn suggestions_asked_at_the_same_time_end_with_one_record() {
        let (_scratch, store, key, run, node) = no_measure();
        let answers: Vec<Value> = std::thread::scope(|scope| {
            let asking: Vec<_> = (0..6).map(|_| scope.spawn(|| store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).map_err(|e| e.to_string()).unwrap())).collect();
            asking.into_iter().map(|a| a.join().unwrap()).collect()
        });
        assert!(answers.iter().all(|a| *a == answers[0]));
        assert_eq!(record_files(&store, &key, &run).len(), 1);
        only_record(&store, &key, &run, &answers[0]);
    }

    // The measure a suggestion used is named by the bytes it was read from. A and B are two versions
    // of one execution.json; the steps run in a fixed order, so nothing depends on timing.

    /// The measured project, its execution file, A's bytes and B's: B adds 1000 to every count.
    fn two_versions() -> (Scratch, Store, String, String, PathBuf, Vec<u8>, Vec<u8>) {
        let (scratch, store, key, run) = measured_mix();
        let id = store.latest_measure(&key, &run).unwrap().unwrap()["id"].as_str().unwrap().to_owned();
        let file = store.run_dir(&key, &run).unwrap().join("executions").join(id).join("execution.json");
        let a = fs::read(&file).unwrap();
        let mut record: Value = serde_json::from_slice(&a).unwrap();
        for stat in record["summary"]["stats"].as_object_mut().unwrap().values_mut() {
            if stat["measure"] == "calls" {
                stat["median"] = json!(stat["median"].as_f64().unwrap() + 1000.);
            }
        }
        let b = serde_json::to_vec_pretty(&record).unwrap();
        assert_ne!(a, b);
        (scratch, store, key, run, file, a, b)
    }

    fn calls_of(out: &Value, id: &str) -> Value {
        out["items"].as_array().unwrap().iter().find(|e| e["id"] == id).unwrap()["calls"].clone()
    }

    #[test]
    fn the_hash_recorded_is_of_the_bytes_the_measure_was_read_from() {
        let (_scratch, store, key, run, file, a, _) = two_versions();
        let read = store.latest_measure_read(&key, &run).unwrap().unwrap();
        // One read gives both: the record is A's and the hash is A's.
        assert_eq!(read.sha256, hash(&a));
        let mut parsed: Value = serde_json::from_slice(&a).unwrap();
        parsed["id"] = read.record["id"].clone();
        assert_eq!(read.record, parsed);
        // And it is the one the old reading picked: the rule did not change.
        assert_eq!(Some(read.record.clone()), store.latest_measure(&key, &run).unwrap());
        assert_eq!(store.hotspots_of(&key, &run, &read.record).unwrap(), store.hotspots(&key, &run).unwrap());
        // With the file as it was, the suggestion is answered and names A.
        let node = symbol_id(&store, &key, &run, "roda");
        let out = store.suggest_with(&key, &run, std::slice::from_ref(&node), Ok(Some(read))).unwrap();
        let SuggestionMeasure::Ok { execution_sha256, .. } = store.verify_suggestion(&key, &run, out["suggestion"].as_str().unwrap()).unwrap().measure else { panic!() };
        assert_eq!(execution_sha256.as_str(), hash(&fs::read(&file).unwrap()));
        assert_eq!(out, store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap());
    }

    #[test]
    fn a_measure_changed_after_it_was_read_is_never_answered_and_never_given_the_new_hash() {
        let (_scratch, store, key, run, file, a, b) = two_versions();
        let node = symbol_id(&store, &key, &run, "roda");
        // 1. A is read. 2. The file becomes B. 3. The suggestion is made from what was read.
        let read = store.latest_measure_read(&key, &run).unwrap().unwrap();
        fs::write(&file, &b).unwrap();
        let error = store.suggest_with(&key, &run, std::slice::from_ref(&node), Ok(Some(read))).unwrap_err().to_string();
        // 4. The check before answering finds the change, and nothing is answered.
        assert!(error.contains("mudou"), "{error}");
        // 5. What was published is A's record: A's counts, A's hash; B's hash is nowhere in it.
        let files = record_files(&store, &key, &run);
        assert_eq!(files.len(), 1);
        let bytes = fs::read(&files[0]).unwrap();
        let record: SuggestionRecord = serde_json::from_slice(&bytes).unwrap();
        let SuggestionMeasure::Ok { execution_sha256, .. } = &record.measure else { panic!("{:?}", record.measure) };
        assert_eq!(execution_sha256.as_str(), hash(&a));
        assert!(!String::from_utf8_lossy(&bytes).contains(&hash(&b)));
        // It is read back as valid only while the execution is A again.
        let sha = files[0].file_stem().unwrap().to_string_lossy().into_owned();
        assert!(store.verify_suggestion(&key, &run, &sha).is_err());
        fs::write(&file, &a).unwrap();
        let back = store.verify_suggestion(&key, &run, &sha).unwrap();
        let with_a = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        assert_eq!(with_a["suggestion"], sha.as_str());
        assert_eq!(serde_json::to_value(&back.output.items).unwrap(), with_a["items"]);
        // Read after the change, B is used and B is named: other counts, another record.
        fs::write(&file, &b).unwrap();
        let with_b = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        let SuggestionMeasure::Ok { execution_sha256, .. } = store.verify_suggestion(&key, &run, with_b["suggestion"].as_str().unwrap()).unwrap().measure else { panic!() };
        assert_eq!(execution_sha256.as_str(), hash(&b));
        assert_ne!(calls_of(&with_b, &node), calls_of(&with_a, &node));
        assert_eq!(calls_of(&with_b, &node).as_f64().unwrap(), calls_of(&with_a, &node).as_f64().unwrap() + 1000.);
    }

    #[test]
    fn a_measure_that_is_gone_or_unreadable_by_the_check_is_not_answered() {
        let (_scratch, store, key, run, file, _, _) = two_versions();
        let node = symbol_id(&store, &key, &run, "roda");
        let read = store.latest_measure_read(&key, &run).unwrap().unwrap();
        fs::remove_file(&file).unwrap();
        assert!(store.suggest_with(&key, &run, std::slice::from_ref(&node), Ok(Some(read))).is_err());
        let read = { fs::write(&file, b"{ metade").unwrap(); store.latest_measure_read(&key, &run) };
        // Read now, it is not a measure: unreadable, and the selection is made without it.
        assert!(read.is_err());
        let out = store.suggest_with(&key, &run, std::slice::from_ref(&node), read).unwrap();
        assert_eq!(out["measured"], false);
        let record = store.verify_suggestion(&key, &run, out["suggestion"].as_str().unwrap()).unwrap();
        assert_eq!(record.measure, SuggestionMeasure::Unreadable {});
    }

    #[test]
    fn the_states_of_a_reading_give_the_states_of_the_record() {
        let read = |sha: &str| crate::execution::ReadExecution { record: json!({ "id": "01-tudo" }), sha256: sha.to_owned() };
        let sha = "a".repeat(64);
        assert_eq!(measure_read(&Ok(None), false).unwrap(), SuggestionMeasure::NoMeasure {});
        assert_eq!(measure_read(&Ok(None), true).unwrap(), SuggestionMeasure::NoMeasure {});
        assert_eq!(measure_read(&Err("ilegível".into()), false).unwrap(), SuggestionMeasure::Unreadable {});
        // Read and not used: it was there, and is not what the selection rests on.
        assert_eq!(measure_read(&Ok(Some(read(&sha))), false).unwrap(), SuggestionMeasure::Unreadable {});
        assert_eq!(measure_read(&Ok(Some(read(&sha))), true).unwrap(), SuggestionMeasure::Ok { execution: "01-tudo".into(), execution_sha256: Sha256Hex::try_from(sha).unwrap() });
        let mut no_id = read(&"b".repeat(64));
        no_id.record = json!({});
        assert!(measure_read(&Ok(Some(no_id)), true).is_err());
    }

    #[test]
    fn with_two_measures_the_reading_is_the_one_the_current_rule_picks() {
        // Both started in the same second (the copy keeps it): the folder reserved later is the latest.
        let (_scratch, store, key, run, file, a, b) = two_versions();
        let executions = store.run_dir(&key, &run).unwrap().join("executions");
        let first = file.parent().unwrap().file_name().unwrap().to_string_lossy().into_owned();
        let later = executions.join("99-depois-tudo");
        fs::create_dir_all(&later).unwrap();
        fs::write(later.join("execution.json"), &b).unwrap();
        let read = store.latest_measure_read(&key, &run).unwrap().unwrap();
        assert_eq!((read.record["id"].as_str().unwrap(), read.sha256.as_str()), ("99-depois-tudo", hash(&b).as_str()));
        assert_eq!(store.latest_measure(&key, &run).unwrap().unwrap()["id"], "99-depois-tudo");
        let node = symbol_id(&store, &key, &run, "roda");
        let out = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
        let SuggestionMeasure::Ok { execution, execution_sha256 } = store.verify_suggestion(&key, &run, out["suggestion"].as_str().unwrap()).unwrap().measure else { panic!() };
        assert_eq!((execution.as_str(), execution_sha256.as_str()), ("99-depois-tudo", hash(&b).as_str()));
        // The other one is still there, untouched, and is not what was read.
        assert_ne!(first, execution);
        assert_eq!(fs::read(&file).unwrap(), a);
    }

    // Which execution is the latest: by when it started, never by how its folder's name sorts.

    /// A record as the selection reads it: id, start, and what makes it eligible.
    fn execution(id: &str, started: &str, kind: &str) -> Value {
        let mut x = json!({ "id": id, "started": started, "name": "probe", "command_sha256": "s", "change": null, "instrumentation": null, "summary": { "succeeded": 1 } });
        match kind {
            "measure" => x["instrumentation"] = json!({ "functions": [] }),
            "failed" => x["summary"]["succeeded"] = json!(0),
            "changed" => x["change"] = json!({ "turn": "01" }),
            "other_command" => x["name"] = json!("outra"),
            "other_hash" => x["command_sha256"] = json!("t"),
            _ => {}
        }
        x
    }

    fn picked(list: &[Value], path: &str) -> Option<String> {
        let found = match path {
            "measure" => crate::execution::latest_measure_of(list.to_vec()),
            _ => crate::execution::baseline_of(list.to_vec(), "probe", "s"),
        };
        found.map(|x| x["id"].as_str().unwrap().to_owned())
    }

    /// Every order of a small list, and the list reversed for a long one.
    fn orders(list: &[Value]) -> Vec<Vec<Value>> {
        let mut out = vec![list.to_vec(), list.iter().rev().cloned().collect()];
        if list.len() <= 4 {
            fn permute(rest: Vec<Value>, done: Vec<Value>, out: &mut Vec<Vec<Value>>) {
                if rest.is_empty() {
                    out.push(done);
                    return;
                }
                for i in 0..rest.len() {
                    let (mut r, mut d) = (rest.clone(), done.clone());
                    d.push(r.remove(i));
                    permute(r, d, out);
                }
            }
            permute(list.to_vec(), vec![], &mut out);
        }
        out
    }

    #[test]
    fn the_latest_is_the_one_that_started_last_for_the_measure_and_for_the_gate_alike() {
        for (path, kind) in [("measure", "measure"), ("gate", "plain")] {
            // 100 executions: 01 to 98 earlier, then 99 and 100.
            let mut list: Vec<Value> = (1..=98).map(|n| execution(&format!("{n:02}-x"), &format!("20261101-{:06}", n), kind)).collect();
            // 99 before 100 in time: the name of 99 sorts last, and 100 is the answer.
            let mut later = list.clone();
            later.push(execution("99-x", "20261102-000000", kind));
            later.push(execution("100-x", "20261102-000001", kind));
            let mut names: Vec<&str> = later.iter().map(|x| x["id"].as_str().unwrap()).collect();
            names.sort();
            assert_eq!(*names.last().unwrap(), "99-x", "the folder order is the defect");
            for order in orders(&later) {
                assert_eq!(picked(&order, path).as_deref(), Some("100-x"), "{path}");
            }
            // 99 after 100 in time: the number does not stand in for the time.
            list.push(execution("99-x", "20261102-000005", kind));
            list.push(execution("100-x", "20261102-000001", kind));
            for order in orders(&list) {
                assert_eq!(picked(&order, path).as_deref(), Some("99-x"), "{path}");
            }
        }
    }

    #[test]
    fn within_one_second_the_number_reserved_later_wins_whatever_the_order() {
        for (path, kind) in [("measure", "measure"), ("gate", "plain")] {
            let tie = [execution("99-x", "20261102-000000", kind), execution("100-x", "20261102-000000", kind), execution("07-x", "20261101-000000", kind)];
            for order in orders(&tie) {
                assert_eq!(picked(&order, path).as_deref(), Some("100-x"), "{path}");
            }
            // The same number twice (a folder reserved again gets a suffix): the id settles it.
            let same = [execution("05-a", "20261102-000000", kind), execution("05-a-2", "20261102-000000", kind)];
            for order in orders(&same) {
                assert_eq!(picked(&order, path).as_deref(), Some("05-a-2"), "{path}");
            }
            // A start not in the form `stamp` writes is older than any that is.
            let odd = [execution("50-x", "ontem", kind), execution("02-x", "20200101-000000", kind)];
            for order in orders(&odd) {
                assert_eq!(picked(&order, path).as_deref(), Some("02-x"), "{path}");
            }
        }
    }

    #[test]
    fn only_eligible_executions_are_picked_and_the_rules_are_the_ones_there_were() {
        let measure_list = [
            execution("10-x", "20261102-000010", "plain"),
            execution("11-x", "20261102-000011", "failed"),
            execution("09-x", "20261102-000009", "measure"),
            execution("08-x", "20261102-000008", "measure"),
        ];
        // The newest are a plain run and a failed measure: neither is a measure that ran.
        let mut failed_measure = measure_list.to_vec();
        failed_measure[1]["instrumentation"] = json!({ "functions": [] });
        for order in orders(&failed_measure) {
            assert_eq!(picked(&order, "measure").as_deref(), Some("09-x"));
        }
        assert_eq!(picked(&[execution("10-x", "20261102-000010", "plain")], "measure"), None);
        let gate_list = [
            execution("20-x", "20261102-000020", "measure"),
            execution("19-x", "20261102-000019", "failed"),
            execution("18-x", "20261102-000018", "changed"),
            execution("17-x", "20261102-000017", "other_command"),
            execution("16-x", "20261102-000016", "other_hash"),
            execution("15-x", "20261102-000015", "plain"),
            execution("14-x", "20261102-000014", "plain"),
        ];
        for order in orders(&gate_list) {
            assert_eq!(picked(&order, "gate").as_deref(), Some("15-x"));
        }
        assert_eq!(picked(&gate_list[..5], "gate"), None);
    }

    /// The real measure of a project, copied into folders 02 to 100, each one starting later than
    /// the one before, except for the two given.
    fn hundred_measures(store: &Store, key: &str, run: &str, starts_99_100: (&str, &str)) -> PathBuf {
        let first = store.latest_measure(key, run).unwrap().unwrap();
        let executions = store.run_dir(key, run).unwrap().join("executions");
        let bytes = fs::read(executions.join(first["id"].as_str().unwrap()).join("execution.json")).unwrap();
        for n in 2..=100 {
            let mut record: Value = serde_json::from_slice(&bytes).unwrap();
            record["started"] = json!(match n {
                99 => starts_99_100.0.to_owned(),
                100 => starts_99_100.1.to_owned(),
                _ => format!("20991101-{n:06}"),
            });
            let dir = executions.join(format!("{n:02}-medir-tudo"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("execution.json"), serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        }
        executions
    }

    #[test]
    fn with_a_hundred_measures_the_suggestion_reads_and_names_the_one_that_started_last() {
        for (starts, expected) in [(("20991102-000000", "20991102-000001"), "100-medir-tudo"), (("20991102-000005", "20991102-000001"), "99-medir-tudo")] {
            let (_scratch, store, key, run) = measured_mix();
            let executions = hundred_measures(&store, &key, &run, starts);
            let read = store.latest_measure_read(&key, &run).unwrap().unwrap();
            assert_eq!(read.record["id"], expected);
            // The value and the hash are still one read, of the file that was picked.
            let file = executions.join(expected).join("execution.json");
            assert_eq!(read.sha256, hash(&fs::read(&file).unwrap()));
            let node = symbol_id(&store, &key, &run, "roda");
            let out = store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap();
            let SuggestionMeasure::Ok { execution, execution_sha256 } = store.verify_suggestion(&key, &run, out["suggestion"].as_str().unwrap()).unwrap().measure else { panic!() };
            assert_eq!((execution.as_str(), execution_sha256.as_str()), (expected, read.sha256.as_str()));
            assert_eq!(store.hotspots(&key, &run).unwrap()["execution"], expected);
            // The listing keeps its order, the one of the folders, with nothing added.
            let listed: Vec<String> = store.executions(&key, &run).unwrap().iter().map(|x| x["id"].as_str().unwrap().to_owned()).collect();
            let mut folders: Vec<String> = fs::read_dir(&executions).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
            folders.sort();
            assert_eq!(listed, folders);
            assert_eq!(listed.last().unwrap(), "99-medir-tudo");
            let fields: Vec<String> = store.executions(&key, &run).unwrap()[5].as_object().unwrap().keys().cloned().collect();
            let mut on_disk: Value = serde_json::from_slice(&fs::read(executions.join(&listed[5]).join("execution.json")).unwrap()).unwrap();
            on_disk["id"] = json!(listed[5]);
            assert_eq!(fields, on_disk.as_object().unwrap().keys().cloned().collect::<Vec<_>>());
            // A change after the read is still found before answering.
            let read = store.latest_measure_read(&key, &run).unwrap().unwrap();
            let mut changed = fs::read(&file).unwrap();
            changed.push(b'\n');
            fs::write(&file, &changed).unwrap();
            assert!(store.suggest_with(&key, &run, std::slice::from_ref(&node), Ok(Some(read))).unwrap_err().to_string().contains("mudou"));
        }
    }

    #[test]
    fn with_a_hundred_runs_the_gate_compares_a_change_with_the_run_that_started_last() {
        let scratch = project();
        let root = scratch.0.join("project");
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        let profile = json!({ "project_root": ".", "commands": { "probe": { "argv": ["git", "log", "-1", "--format=tformat:librarian-measure answer=42 units"], "repeat": 2 } } });
        fs::write(root.join(PROFILE), profile.to_string()).unwrap();
        git(&["init", "-q"]);
        git(&["config", "core.autocrlf", "false"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let sha = store.commands(&key).unwrap()["commands"][0]["sha256"].as_str().unwrap().to_owned();
        store.approve_command(&key, "probe", &sha).unwrap();
        let first = store.execute(&key, &run, "probe", None).unwrap();
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
        // Without anything about the order, the gate compares with the one run there is, as before.
        let plain = store.execute(&key, &run, "probe", Some(&change)).unwrap();
        assert_eq!(plain["comparison"]["baseline"], first["id"]);
        assert_eq!(plain["comparison"]["stats"]["answer"]["baseline_median"], 42.0);
        // A hundred baselines, each with its own number for the answer, so the comparison says which one it read.
        let executions = store.run_dir(&key, &run).unwrap().join("executions");
        let record: Value = serde_json::from_slice(&fs::read(executions.join(first["id"].as_str().unwrap()).join("execution.json")).unwrap()).unwrap();
        for (starts, expected) in [(("20991102-000000", "20991102-000001"), "100-probe"), (("20991102-000005", "20991102-000001"), "99-probe")] {
            for n in 3..=100 {
                let mut copy = record.clone();
                copy["started"] = json!(match n { 99 => starts.0.to_owned(), 100 => starts.1.to_owned(), _ => format!("20991101-{n:06}") });
                copy["summary"]["stats"]["answer"]["median"] = json!(n as f64);
                let dir = executions.join(format!("{n:02}-probe"));
                fs::create_dir_all(&dir).unwrap();
                fs::write(dir.join("execution.json"), serde_json::to_vec_pretty(&copy).unwrap()).unwrap();
            }
            let compared = store.execute(&key, &run, "probe", Some(&change)).unwrap();
            assert_eq!(compared["comparison"]["baseline"], expected);
            let number: f64 = expected.split('-').next().unwrap().parse().unwrap();
            assert_eq!(compared["comparison"]["stats"]["answer"]["baseline_median"], number);
            // The run this one made carries a change, so it never becomes a baseline itself.
            fs::remove_dir_all(executions.join(compared["id"].as_str().unwrap())).unwrap();
        }
    }

    // The evidence package of each new question: one per question, beside it, named in question.json.

    /// The package of a turn, read back the way any reader would: the mark, the bytes, the hash.
    fn package_of(store: &Store, key: &str, run: &str, turn: &str) -> (Value, Vec<u8>, EvidencePackage) {
        let dir = store.turn_dir(key, run, turn).unwrap();
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json")).unwrap()).unwrap();
        let bytes = fs::read(dir.join(EVIDENCE)).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(&hash(&bytes)), "a package holds no hash of itself");
        let package: EvidencePackage = serde_json::from_slice(&bytes).unwrap();
        // The mark names the package's own format (2 for every new question; 1 in the format 1 fixture).
        assert_eq!(question["evidence"], json!({ "format_version": u32::from(package.format_version), "sha256": hash(&bytes) }), "{turn}");
        let mut again = serde_json::to_vec_pretty(&package).unwrap();
        again.push(b'\n');
        assert_eq!(again, bytes, "{turn}: the bytes are the package's own");
        (question, bytes, package)
    }

    /// Every field of a query file, by path, as the classification names them.
    fn leaves(value: &Value, path: &str, out: &mut std::collections::BTreeSet<String>) {
        match value {
            Value::Object(map) => map.iter().for_each(|(k, v)| leaves(v, &format!("{path}.{k}"), out)),
            Value::Array(list) if !list.is_empty() => list.iter().for_each(|v| leaves(v, &format!("{path}[]"), out)),
            _ => {
                out.insert(path.to_owned());
            }
        }
    }

    fn classified_paths(package: &EvidencePackage) -> std::collections::BTreeSet<String> {
        package.query.classified().into_iter().map(|f| f.path).collect()
    }

    /// The package's hashes are of the files this turn has, and of no other turn's.
    fn bound_to_its_turn(store: &Store, key: &str, run: &str, turn: &str, package: &EvidencePackage) {
        let dir = store.turn_dir(key, run, turn).unwrap();
        let of = |path: &str| hash(&fs::read(dir.join(path)).unwrap());
        let bound = &package.bound;
        assert_eq!(bound.request_sha256.as_str(), of("prepared/request.json"));
        assert_eq!(bound.search_sha256.as_str(), of("prepared/search.json"));
        assert_eq!(bound.dossier_sha256.as_str(), of("prepared/bundle/dossier.json"));
        assert_eq!(bound.bundle_query_sha256.as_str(), of("prepared/bundle/query.json"));
        match package.representation {
            Representation::Claims => {
                assert_eq!(bound.claims_context_sha256.as_ref().unwrap().as_str(), of("claims/context.json"));
                assert_eq!(package.query.sha256.as_str(), of("claims/query.json"));
            }
            Representation::Prose => {
                assert_eq!(bound.claims_context_sha256, None);
                assert_eq!(package.query.sha256.as_str(), of("prepared/bundle/query.json"));
            }
        }
        match dir.join(CRITERIA).is_file() {
            true => assert_eq!(bound.criteria_sha256.as_ref().unwrap().as_str(), of(CRITERIA)),
            false => assert_eq!(bound.criteria_sha256, None),
        }
    }

    fn opened() -> (Scratch, Store, String, Settings) {
        let scratch = project();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let opened = store.open(&display(&scratch.0.join("project"))).unwrap();
        let key = opened["key"].as_str().unwrap().to_owned();
        let mut settings: Settings = serde_json::from_value(opened["settings"].clone()).unwrap();
        settings.lexicon.groups = vec![vec!["raios".into(), "ray".into()], vec!["direcao".into(), "direction".into()]];
        store.save_settings(&key, &settings).unwrap();
        (scratch, store, key, settings)
    }

    #[test]
    fn a_question_asked_gets_its_package_in_claims_and_in_prose() {
        let (_scratch, store, key, mut settings) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let asked = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        let (_, _, claims) = package_of(&store, &key, &run, asked["id"].as_str().unwrap());
        assert_eq!((claims.route, claims.representation, claims.origin_kind), (Route::Search, Representation::Claims, None));
        assert_eq!(claims.versions.builder, crate::builder::identity());
        assert_eq!(claims.versions.rules.0, envelope::RULES_VERSION);
        assert!(claims.units.iter().all(|u| u.excerpt_sha256.is_some()), "claims units carry their excerpt");
        bound_to_its_turn(&store, &key, &run, asked["id"].as_str().unwrap(), &claims);
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        assert_eq!(claims.edition.origin, edition_origin(&catalog).unwrap());
        assert_eq!((claims.edition.git_head.clone(), claims.edition.git_dirty), (catalog.manifest().git.head.clone(), catalog.manifest().git.dirty));
        settings.answer_format = AnswerFormat::Prose;
        store.save_settings(&key, &settings).unwrap();
        let prose = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        let (_, _, package) = package_of(&store, &key, &run, prose["id"].as_str().unwrap());
        assert_eq!(package.representation, Representation::Prose);
        assert!(package.units.iter().all(|u| u.excerpt_sha256.is_none()), "prose has no excerpt");
        bound_to_its_turn(&store, &key, &run, prose["id"].as_str().unwrap(), &package);
        // Every field of the query the package describes is classified, and nothing else is.
        let query: Value = serde_json::from_slice(&fs::read(store.turn_dir(&key, &run, prose["id"].as_str().unwrap()).unwrap().join("prepared/bundle/query.json")).unwrap()).unwrap();
        let mut real = std::collections::BTreeSet::new();
        leaves(&query, "$", &mut real);
        assert_eq!(classified_paths(&package), real);
    }

    /// U6 over the claims query the Studio really writes, in each form its settings give it.
    #[test]
    fn every_field_of_every_claims_query_the_studio_writes_is_classified() {
        let (_scratch, store, key, mut settings) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let forms: [&dyn Fn(&mut Settings); 5] = [
            &|_| {},
            &|s| (s.allow_patch, s.allow_hypothesis) = (true, true),
            &|s| (s.compact_items, s.language) = (true, "en".into()),
            &|s| s.instructions = Some(vec!["Responda curto.".into(), "Cite as linhas.".into()]),
            &|s| (s.allow_inference, s.allow_hypothesis, s.allow_patch, s.compact_items) = (true, true, false, false),
        ];
        let mut seen = std::collections::BTreeSet::new();
        for change in forms {
            change(&mut settings);
            store.save_settings(&key, &settings).unwrap();
            let asked = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
            let turn = asked["id"].as_str().unwrap();
            let (_, _, package) = package_of(&store, &key, &run, turn);
            assert_eq!(package.representation, Representation::Claims);
            let query: Value = serde_json::from_slice(&fs::read(store.turn_dir(&key, &run, turn).unwrap().join("claims/query.json")).unwrap()).unwrap();
            let mut real = std::collections::BTreeSet::new();
            leaves(&query, "$", &mut real);
            assert_eq!(classified_paths(&package), real, "{turn}");
            seen.extend(real);
        }
        // The forms above did reach the optional parts of the query.
        for part in ["$.allow_edits", "$.project_instructions[]", "$.evidence.items[].symbols[].name", "$.policy.allow_inference"] {
            assert!(seen.contains(part), "{part}");
        }
        assert!(!seen.contains("$.evidence.items[].excerpt") || seen.iter().any(|p| p == "$.evidence.items[].id"));
    }

    #[test]
    fn each_question_of_a_path_has_its_own_package() {
        let (_scratch, store, key, _) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let nodes = vec![symbol_id(&store, &key, &run, "ray_from_pixel"), symbol_id(&store, &key, &run, "Camera")];
        let turns = store.ask_path(&key, &run, "O que isto faz?", &nodes, &nodes[0], &json!([])).unwrap();
        let ids: Vec<&str> = turns.as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap()).collect();
        assert_eq!(ids.len(), 2);
        let mut seen = std::collections::BTreeSet::new();
        for turn in &ids {
            let (question, bytes, package) = package_of(&store, &key, &run, turn);
            assert_eq!(package.route, Route::Path);
            bound_to_its_turn(&store, &key, &run, turn, &package);
            assert!(seen.insert(hash(&bytes)), "{turn}: a package is one question's");
            assert_eq!(question["path"]["node"], package.fichas[0].symbol_id.clone().unwrap());
        }
    }

    #[test]
    fn a_gathered_question_tells_its_origin_and_names_its_checked_selection() {
        let (_scratch, store, key, _) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let nodes = vec![symbol_id(&store, &key, &run, "ray_from_pixel")];
        // A selection of the format before is not read as a new one: nothing is created.
        let older = json!({ "kind": "selection", "summary": "x", "selection": { "format_version": 1, "seeds": [nodes[0]] } });
        let refused = store.ask_gathered(&key, &run, "O que muda?", &nodes, false, Some(older)).unwrap_err().to_string();
        assert!(refused.starts_with("Seleção recusada (forma: kind não é selection_record)"), "{refused}");
        assert!(!store.run_dir(&key, &run).unwrap().join("turns").exists());
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&nodes[0]);
        let answer = page.suggest(&store, &key, &run, &[nodes[0].clone()]);
        let (origin, nodes) = (page.origin(), page.gathered.clone());
        let with = store.ask_gathered(&key, &run, "O que muda?", &nodes, false, Some(origin)).unwrap();
        let (_, _, package) = package_of(&store, &key, &run, with["id"].as_str().unwrap());
        assert_eq!((package.route, package.origin_kind), (Route::Gathered, Some(OriginKind::Selection)));
        let suggestion = Sha256Hex::try_from(answer["suggestion"].as_str().unwrap().to_owned()).unwrap();
        assert_eq!(package.measure.selection_basis, MeasureSelectionBasis::NoMeasure { suggestion: suggestion.clone() });
        let reference = package.selection.clone().unwrap();
        assert_eq!((reference.suggestions, reference.declared_by), (vec![suggestion], ServerDeclared::Server));
        // No measure at the question nor under the suggestion: the same, by the two readings compared.
        assert_eq!((package.format_version, package.measure.measure_diverges, package.measure.measure_divergence.clone()), (PackageFormat::V2, None, Some(MeasureDivergence::Same {})));
        // From a hotspot: no selection behind it.
        let hot = store.ask_gathered(&key, &run, "O que muda?", &nodes, false, Some(json!({ "kind": "hot", "summary": "y" }))).unwrap();
        let (_, _, package) = package_of(&store, &key, &run, hot["id"].as_str().unwrap());
        assert_eq!((package.origin_kind, package.measure.selection_basis.clone()), (Some(OriginKind::Hot), MeasureSelectionBasis::NotApplicable {}));
        // A kind the contract does not know is not taken for one it does.
        let other = store.ask_gathered(&key, &run, "O que muda?", &nodes, false, Some(json!({ "kind": "outra" }))).unwrap();
        assert_eq!(package_of(&store, &key, &run, other["id"].as_str().unwrap()).2.origin_kind, None);
        // Asked with no origin, and searched: nothing to say of a selection.
        let plain = store.ask_gathered(&key, &run, "O que muda?", &nodes, false, None).unwrap();
        assert_eq!(package_of(&store, &key, &run, plain["id"].as_str().unwrap()).2.measure.selection_basis, MeasureSelectionBasis::NotApplicable {});
    }

    #[test]
    fn what_a_retry_added_is_the_servers_and_the_rest_is_the_clients() {
        let (_scratch, store, key, _) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let nodes = vec![symbol_id(&store, &key, &run, "ray_from_pixel")];
        let original = "Como acelerar?";
        // The ladder's own texts, which a question always got after the client's.
        let gate = crate::gate_suffix(&["Dobrar: piorou".into()], 1);
        let mechanical = crate::mechanical_suffix(&["quote_mismatch".into(), "no_location".into()], 2);
        assert_eq!(gate.text, " Propostas anteriores recusadas pelo portão: Dobrar: piorou. Proponha outras mudanças.");
        assert_eq!(mechanical.text, " Uma resposta anterior foi recusada na conferência mecânica: quote_mismatch, no_location.");
        let none = store.ask_gathered_parts(&key, &run, original, &[], &nodes, false, None, None, &[]).unwrap();
        let one = store.ask_gathered_parts(&key, &run, original, std::slice::from_ref(&gate), &nodes, false, None, None, &[]).unwrap();
        let two = store.ask_gathered_parts(&key, &run, original, &[gate.clone(), mechanical.clone()], &nodes, false, None, None, &[]).unwrap();
        for (turn, suffixes) in [(&none, vec![]), (&one, vec![&gate]), (&two, vec![&gate, &mechanical])] {
            let turn = turn["id"].as_str().unwrap();
            let (question, _, package) = package_of(&store, &key, &run, turn);
            let prepared: String = std::iter::once(original).chain(suffixes.iter().map(|s| s.text.as_str())).collect();
            // The text sent is the joined one, as before; the package keeps the parts apart.
            assert_eq!(question["question"], prepared.as_str());
            assert_eq!(package.query.person_text.sha256.as_str(), hash(original.as_bytes()));
            assert_eq!(package.query.prepared_question_sha256.as_str(), hash(prepared.as_bytes()));
            let recorded: Vec<(usize, String, RetryRule, usize)> = package.query.server_suffixes.iter().map(|s| (s.order, s.sha256.as_str().to_owned(), s.rule, s.attempt)).collect();
            let expected: Vec<(usize, String, RetryRule, usize)> = suffixes.iter().enumerate().map(|(n, s)| (n + 1, hash(s.text.as_bytes()), s.rule, s.attempt)).collect();
            assert_eq!(recorded, expected, "{turn}");
            let field = package.query.classified().into_iter().find(|f| f.path == "$.question").unwrap();
            let class = if suffixes.is_empty() { evidence::QueryFieldClass::ClientText } else { evidence::QueryFieldClass::ClientTextWithServerSuffixes };
            assert_eq!(field.class, class);
        }
    }

    #[test]
    fn the_measure_at_the_question_is_the_one_read_then_or_says_there_was_none_or_it_could_not_be_read() {
        let (_scratch, store, key, run) = measured_mix();
        let node = symbol_id(&store, &key, &run, "roda");
        let asked = store.ask_gathered(&key, &run, "Por que roda?", std::slice::from_ref(&node), false, None).unwrap();
        let (_, _, package) = package_of(&store, &key, &run, asked["id"].as_str().unwrap());
        let read = store.latest_measure_read(&key, &run).unwrap().unwrap();
        assert_eq!(package.measure.at_question, MeasureAtQuestion::Ok { execution: read.record["id"].as_str().unwrap().to_owned(), execution_sha256: Sha256Hex::try_from(read.sha256.clone()).unwrap() });
        // Unreadable: an execution that does not parse is not the absence of a measure.
        let broken = store.run_dir(&key, &run).unwrap().join("executions/99-quebrada");
        fs::create_dir_all(&broken).unwrap();
        fs::write(broken.join("execution.json"), b"{ metade").unwrap();
        let unreadable = store.ask_gathered(&key, &run, "Por que roda?", std::slice::from_ref(&node), false, None).unwrap();
        assert_eq!(package_of(&store, &key, &run, unreadable["id"].as_str().unwrap()).2.measure.at_question, MeasureAtQuestion::Unreadable {});
        // A measure taken later does not change what an earlier question recorded.
        let (_, earlier, _) = package_of(&store, &key, &run, asked["id"].as_str().unwrap());
        fs::remove_dir_all(&broken).unwrap();
        assert_eq!(package_of(&store, &key, &run, asked["id"].as_str().unwrap()).1, earlier);
        // None: no measure at all.
        let (_scratch2, store, key, _) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let asked = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        assert_eq!(package_of(&store, &key, &run, asked["id"].as_str().unwrap()).2.measure.at_question, MeasureAtQuestion::NoMeasure {});
    }

    #[test]
    fn the_projects_instructions_are_those_the_query_carries_or_there_are_none() {
        let (_scratch, store, key, mut settings) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let none = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        assert_eq!(package_of(&store, &key, &run, none["id"].as_str().unwrap()).2.query.project_instructions_sha256, None);
        let given = vec!["Responda curto.".to_owned()];
        settings.instructions = Some(given.clone());
        for format in [AnswerFormat::Claims, AnswerFormat::Prose] {
            settings.answer_format = format;
            store.save_settings(&key, &settings).unwrap();
            let asked = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
            let (_, _, package) = package_of(&store, &key, &run, asked["id"].as_str().unwrap());
            assert_eq!(package.query.project_instructions_sha256.as_ref().unwrap().as_str(), hash(json!(given).to_string().as_bytes()), "{format:?}");
            let fixed = package.query.fixed_contract_sha256.clone();
            assert_ne!(Some(&fixed), package.query.project_instructions_sha256.as_ref());
        }
    }

    #[test]
    fn a_reviewed_fact_carries_its_line_in_the_acervo_or_says_it_is_not_there() {
        let (_scratch, store, key, settings) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        let answer = json!({"schema_version": 1, "claims": [{
            "id": "C1", "kind": "FACT", "text": "`ray_from_pixel` calcula `direction` na linha 5 de src/lib.rs.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 4, "end_line": 5,
                "quote": "    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;"}],
            "premises": [], "verification_plan": null,
            "location": {"evidence_id": "E1", "path": "src/lib.rs", "symbol_name": "ray_from_pixel",
                "declaration_line": 4, "operation_start_line": 5, "operation_end_line": 5}}]});
        record_answer(&store, &key, &run, "01", "manual", &answer.to_string()).unwrap();
        store.review(&key, &run, "01", "01-manual", "C1", true, "A linha 5 calcula direction.").unwrap();
        // /api/continue: a chain step with the reviewed fact.
        let step = store.continue_turn(&key, &run, "01", "Como evitar a multiplicação?").unwrap();
        let (_, _, package) = package_of(&store, &key, &run, step["id"].as_str().unwrap());
        assert_eq!(package.route, Route::Chain);
        let line = store.promotion_lines(&key).unwrap().into_iter().find(|(id, _)| id == "F1").unwrap().1;
        let fact = &package.query.reviewed_facts[0];
        assert_eq!((fact.fact_id.as_str(), fact.record_sha256.as_ref().map(|h| h.as_str().to_owned())), ("F1", Some(hash(&line))));
        assert_eq!(fact.provenance, evidence::ReviewedFactProvenance::AcervoQuotesChecked);
        let raw = fs::read(store.project_dir(&key).unwrap().join("facts.jsonl")).unwrap();
        assert!(raw.split(|b| *b == b'\n').any(|l| l == line.as_slice()), "the very line, byte for byte");
        // The same fact with no line in the acervo: the request carries it, the package says so.
        let targets = vec![Target { path: "src/lib.rs".into(), symbol_name: "ray_from_pixel".into(), declaration_line: 4 }];
        let reviewed = store.reviewed_about(&key, &run, &targets).unwrap();
        fs::write(store.project_dir(&key).unwrap().join("facts.jsonl"), b"").unwrap();
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        let turn = store.new_turn(&key, &run, &settings, &catalog, Asked::client("De novo?"), None, request(&settings, "De novo?", Some(targets), reviewed, false), Some(("chain", json!({}))))
            .unwrap()["id"].as_str().unwrap().to_owned();
        let fact = package_of(&store, &key, &run, &turn).2.query.reviewed_facts[0].clone();
        assert_eq!((fact.record_sha256, fact.provenance), (None, evidence::ReviewedFactProvenance::RecordNotAvailable));
        assert_eq!(fact.citations.len(), 1, "the quotes are still checked against the edition");
    }

    #[test]
    fn the_same_question_twice_gives_the_same_package_bytes() {
        let (_scratch, store, key, _) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let one = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        let two = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        assert_ne!(one["id"], two["id"]);
        assert_eq!(package_of(&store, &key, &run, one["id"].as_str().unwrap()).1, package_of(&store, &key, &run, two["id"].as_str().unwrap()).1);
        // The fields are grouped, and expanding the groups gives back one entry per place.
        let (_, _, package) = package_of(&store, &key, &run, one["id"].as_str().unwrap());
        let expanded = package.query.classified();
        assert_eq!(evidence::group_fields(expanded.clone()), package.query.fields);
        assert!(expanded.len() >= package.query.fields.len());
        // With several fichas a field is at several places, written in one group.
        let nodes = vec![symbol_id(&store, &key, &run, "ray_from_pixel"), symbol_id(&store, &key, &run, "Camera")];
        let several = store.ask_gathered(&key, &run, "O que muda?", &nodes, false, None).unwrap();
        let (_, _, package) = package_of(&store, &key, &run, several["id"].as_str().unwrap());
        assert!(package.query.classified().len() > package.query.fields.len());
        assert!(package.query.fields.iter().any(|g| g.places.len() > 1));
    }

    #[test]
    fn a_package_that_cannot_be_made_is_marked_and_its_question_is_never_sent() {
        for (stage, code) in [("composition", "composition_failed"), ("persistence", "persistence_failed")] {
            let (_scratch, store, key, _) = opened();
            let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
            FAIL_EVIDENCE_AT.with(|f| f.set(Some(stage)));
            let error = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap_err().to_string();
            FAIL_EVIDENCE_AT.with(|f| f.set(None));
            assert!(error.contains(code), "{error}");
            // The question is there, prepared, with the error marked and no package.
            let dir = store.turn_dir(&key, &run, "01").unwrap();
            let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json")).unwrap()).unwrap();
            assert_eq!(question["evidence"], json!({ "error": code }));
            assert!(dir.join("prepared/bundle/query.json").is_file());
            assert!(!dir.join(EVIDENCE).exists(), "nothing half made is published");
            // No way to send it: not a send, not a pasted answer.
            assert!(store.verify_for_send(&key, &run, "01").err().unwrap().to_string().contains(code));
            assert!(record_answer(&store, &key, &run, "01", "manual", "{}").is_err());
            // Another question after it is made as usual.
            let next = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
            package_of(&store, &key, &run, next["id"].as_str().unwrap());
        }
    }

    #[test]
    fn an_older_question_with_no_mark_is_sent_as_before() {
        let (_scratch, store, key, _) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        let dir = store.turn_dir(&key, &run, "01").unwrap();
        let mut question: Value = serde_json::from_slice(&fs::read(dir.join("question.json")).unwrap()).unwrap();
        question.as_object_mut().unwrap().remove("evidence");
        fs::write(dir.join("question.json"), serde_json::to_vec_pretty(&question).unwrap()).unwrap();
        fs::remove_file(dir.join(EVIDENCE)).unwrap();
        assert!(store.verify_for_send(&key, &run, "01").is_ok());
        // A question refused by the preparation has no package and no mark, as before.
        let refused = store.ask(&key, &run, "xyzzy plugh").unwrap();
        assert!(!refused["prepare_error"].is_null());
        let dir = store.turn_dir(&key, &run, refused["id"].as_str().unwrap()).unwrap();
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json")).unwrap()).unwrap();
        assert!(question.get("evidence").is_none());
        assert!(!dir.join(EVIDENCE).exists());
    }

    /// The package the stored inputs of a turn give now: the bytes `new_turn` published, if nothing changed.
    fn rebuilt(store: &Store, key: &str, run: &str, turn: &str, asked: Asked) -> Vec<u8> {
        let dir = store.turn_dir(key, run, turn).unwrap();
        let settings = store.settings(key).unwrap();
        let catalog = Catalog::load(&store.edition(key, run).unwrap()).unwrap();
        let request: Request = serde_json::from_slice(&fs::read(dir.join("prepared/request.json")).unwrap()).unwrap();
        let composition = prepare::prepare(&catalog, &request).unwrap().composition;
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json")).unwrap()).unwrap();
        let origin = ["gathered", "chain", "path"].into_iter().find(|f| question.get(*f).is_some()).map(|f| (f, question[f].clone()));
        // The selection's record as kept with the question, read back by the hash of its bytes.
        let selection = fs::read(dir.join(SELECTION)).ok().map(|bytes| (Sha256Hex::try_from(hash(&bytes)).unwrap(), SelectionRecord::from_bytes(&bytes, &hash(&bytes)).unwrap()));
        store.evidence_package(key, run, &catalog, &dir, &composition, &asked, origin.as_ref(), settings.answer_format == AnswerFormat::Claims,
            settings.instructions.as_deref(), None, selection.as_ref().map(|(s, r)| (s, r)), PackageFormat::WRITTEN).unwrap()
    }

    #[test]
    fn a_stored_package_is_the_one_its_inputs_give_and_says_what_its_files_say() {
        let (_scratch, store, key, mut settings) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let nodes = vec![symbol_id(&store, &key, &run, "ray_from_pixel"), symbol_id(&store, &key, &run, "Camera")];
        let suffixes = [crate::gate_suffix(&["x".into()], 1), crate::mechanical_suffix(&["y".into()], 2)];
        // The page's envelope: both gathered by hand, a suggestion from the first, kept as they are.
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&nodes[0]);
        page.toggle(&nodes[1]);
        page.toggle_target(&nodes[1]);
        page.suggest(&store, &key, &run, &[nodes[0].clone()]);
        assert_eq!(page.gathered, nodes);
        let origin = page.origin();
        for format in [AnswerFormat::Claims, AnswerFormat::Prose] {
            settings.answer_format = format;
            store.save_settings(&key, &settings).unwrap();
            let selection = store.accept_selection(&key, &run, &nodes, Some(&origin)).unwrap();
            let asked = store.ask_gathered_parts(&key, &run, "O que muda?", &suffixes, &nodes, false, Some(origin.clone()), selection.as_ref(), &[]).unwrap();
            let turn = asked["id"].as_str().unwrap();
            let (_, bytes, package) = package_of(&store, &key, &run, turn);
            assert_eq!(rebuilt(&store, &key, &run, turn, Asked { original: "O que muda?", suffixes: &suffixes }), bytes, "{format:?}");
            // A stored package changed in any part is not the one its inputs give.
            let stored: Value = serde_json::from_slice(&bytes).unwrap();
            let changes: [(&str, Value); 14] = [
                ("/units/0/text_sha256", json!("0".repeat(64))),
                ("/units/0/source_sha256", json!("0".repeat(64))),
                ("/fichas/0/unit", json!("CTX_9")),
                ("/query/sha256", json!("0".repeat(64))),
                ("/query/fields/0/class", json!("code")),
                ("/query/fields/0/origin", json!("client")),
                ("/query/limits/0/producer", json!("diagnostics")),
                ("/query/limits/0/rule_version", json!("outra")),
                ("/query/person_text/sha256", json!(hash(b"outro"))),
                ("/query/server_suffixes/0/declared_by", json!("client")),
                ("/query/server_suffixes/0/sha256", json!(hash(b"outro"))),
                ("/measure/at_question", json!({ "read_status": "unreadable" })),
                ("/measure/selection_basis", json!({ "read_status": "not_applicable" })),
                ("/bound/dossier_sha256", json!("0".repeat(64))),
            ];
            for (place, value) in changes {
                let mut changed = stored.clone();
                *changed.pointer_mut(place).unwrap_or_else(|| panic!("{place}")) = value;
                let mut changed_bytes = serde_json::to_vec_pretty(&changed).unwrap();
                changed_bytes.push(b'\n');
                assert_ne!(changed_bytes, bytes, "{place}");
            }
            let mut fewer = stored.clone();
            fewer["fichas"].as_array_mut().unwrap().pop();
            assert_ne!(serde_json::to_vec_pretty(&fewer).unwrap(), bytes);
            let mut swapped = stored.clone();
            swapped["query"]["server_suffixes"].as_array_mut().unwrap().swap(0, 1);
            assert_ne!(serde_json::to_vec_pretty(&swapped).unwrap(), bytes);
            // What it says of fichas and units is what the prepared query and the snapshots say.
            let dir = store.turn_dir(&key, &run, turn).unwrap();
            let query: Value = serde_json::from_slice(&fs::read(dir.join("prepared/bundle/query.json")).unwrap()).unwrap();
            let refs: Vec<&str> = query["evidence"]["selections"].as_array().unwrap().iter().map(|s| s["fact_id"].as_str().unwrap()).collect();
            assert_eq!(package.fichas.iter().map(|f| f.reference.0.as_str()).collect::<Vec<_>>(), refs);
            let blocks: Vec<&str> = query["evidence"]["contexts"].as_array().unwrap().iter().map(|b| b["id"].as_str().unwrap()).collect();
            assert_eq!(package.units.iter().map(|u| u.evidence_id.0.as_str()).collect::<Vec<_>>(), blocks);
            assert!(package.fichas.iter().all(|f| package.units.iter().any(|u| u.evidence_id == f.unit && u.path == f.path)));
            let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
            for unit in &package.units {
                let bytes = catalog.source_bytes(&unit.path).unwrap();
                assert_eq!(unit.source_sha256.as_str(), hash(bytes));
                let window = evidence::normalize_window(bytes, unit.window_start_line, unit.window_end_line, package.representation).unwrap();
                assert_eq!((window.text_sha256, window.excerpt_sha256), (unit.text_sha256.clone(), unit.excerpt_sha256.clone()));
            }
            assert_eq!(package.query.server_suffixes.iter().map(|s| (s.declared_by, s.order)).collect::<Vec<_>>(),
                [(evidence::ServerRetryDeclared::ServerRetry, 1), (evidence::ServerRetryDeclared::ServerRetry, 2)]);
            assert_eq!(package.query.person_text.declared_by, evidence::ClientDeclared::Client);
        }
    }

    #[test]
    fn criteria_and_the_projects_commit_are_bound_when_there_are_some() {
        let (scratch, store, key, mut settings) = opened();
        // The analyzed project in Git: the edition records its commit, and so does the package.
        let root = scratch.0.join("project");
        let git = |args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(&root).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).status().unwrap().success());
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "x"]);
        settings.criteria = Some("A resposta cita a linha.".into());
        store.save_settings(&key, &settings).unwrap();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let asked = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        let turn = asked["id"].as_str().unwrap();
        let (_, _, package) = package_of(&store, &key, &run, turn);
        bound_to_its_turn(&store, &key, &run, turn, &package);
        assert!(package.bound.criteria_sha256.is_some());
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        assert!(catalog.manifest().git.head.is_some());
        assert_eq!(package.edition.git_head, catalog.manifest().git.head);
    }

    #[test]
    fn instructions_the_query_does_not_carry_are_not_taken_for_its_own() {
        let (_scratch, store, key, mut settings) = opened();
        settings.instructions = Some(vec!["Responda curto.".into()]);
        store.save_settings(&key, &settings).unwrap();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let asked = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap();
        let turn = asked["id"].as_str().unwrap().to_owned();
        assert_eq!(rebuilt(&store, &key, &run, &turn, Asked::client("Onde a câmera calcula a direção dos raios?")), package_of(&store, &key, &run, &turn).1);
        // The project now says other instructions than the claims query was written with.
        settings.instructions = Some(vec!["Outra.".into()]);
        store.save_settings(&key, &settings).unwrap();
        let dir = store.turn_dir(&key, &run, &turn).unwrap();
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        let request: Request = serde_json::from_slice(&fs::read(dir.join("prepared/request.json")).unwrap()).unwrap();
        let composition = prepare::prepare(&catalog, &request).unwrap().composition;
        let refused = store.evidence_package(&key, &run, &catalog, &dir, &composition, &Asked::client("Onde a câmera calcula a direção dos raios?"), None,
            settings.answer_format == AnswerFormat::Claims, settings.instructions.as_deref(), None, None, PackageFormat::WRITTEN);
        assert_eq!(refused, Err("provenance_failed"));
    }

    #[test]
    fn each_retry_names_the_rule_that_made_it() {
        assert_eq!((crate::gate_suffix(&[], 3).rule, crate::gate_suffix(&[], 3).attempt), (RetryRule::GateRefused, 3));
        assert_eq!((crate::mechanical_suffix(&[], 4).rule, crate::mechanical_suffix(&[], 4).attempt), (RetryRule::MechanicalRefused, 4));
    }

    #[test]
    fn only_a_promotion_is_a_facts_line_and_it_is_read_byte_for_byte() {
        let (_scratch, store, key, _) = opened();
        let file = store.project_dir(&key).unwrap().join("facts.jsonl");
        let promoted = r#"{"event":"promoted","id":"F1","statement":"s"}"#;
        fs::write(&file, format!("{}\n{promoted}\n{}\n", r#"{"event":"revoked","id":"F1","note":"n"}"#, r#"{"event":"promoted","id":"F2"}"#)).unwrap();
        let lines = store.promotion_lines(&key).unwrap();
        assert_eq!(lines.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), ["F1", "F2"]);
        assert_eq!(lines[0].1, promoted.as_bytes());
        fs::remove_file(&file).unwrap();
        assert!(store.promotion_lines(&key).unwrap().is_empty());
    }

    // Step 9: a new question's package passes H1, H2 and H3 before its question is sent, pasted,
    // reviewed or continued. Each check is shown alone: a forger keeps the rest consistent.

    const RAY: &str = "Onde a câmera calcula a direção dos raios?";

    /// A FACT the edition of the test project confirms.
    fn ray_fact() -> String {
        json!({"schema_version": 1, "claims": [{
            "id": "C1", "kind": "FACT", "text": "`ray_from_pixel` calcula `direction` na linha 5 de src/lib.rs.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 4, "end_line": 5,
                "quote": "    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;"}],
            "premises": [], "verification_plan": null,
            "location": {"evidence_id": "E1", "path": "src/lib.rs", "symbol_name": "ray_from_pixel",
                "declaration_line": 4, "operation_start_line": 5, "operation_end_line": 5}}]}).to_string()
    }

    /// One more question of `format`, with its package; its turn id.
    fn fresh(store: &Store, key: &str, run: &str, format: AnswerFormat) -> String {
        let mut settings = store.settings(key).unwrap();
        settings.answer_format = format;
        store.save_settings(key, &settings).unwrap();
        store.ask(key, run, RAY).unwrap()["id"].as_str().unwrap().to_owned()
    }

    fn opened_run() -> (Scratch, Store, String, String) {
        let (scratch, store, key, _) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        (scratch, store, key, run)
    }

    /// Changes `question.json` as it is stored.
    fn remark(store: &Store, key: &str, run: &str, turn: &str, change: impl FnOnce(&mut Value)) {
        let file = store.turn_dir(key, run, turn).unwrap().join("question.json");
        let mut question: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        change(&mut question);
        fs::write(&file, serde_json::to_vec_pretty(&question).unwrap()).unwrap();
    }

    /// Rewrites a turn's package as a forger would: well formed, and with the mark naming it.
    fn forge(store: &Store, key: &str, run: &str, turn: &str, change: impl FnOnce(&mut EvidencePackage)) {
        let dir = store.turn_dir(key, run, turn).unwrap();
        let mut package: EvidencePackage = serde_json::from_slice(&fs::read(dir.join(EVIDENCE)).unwrap()).unwrap();
        change(&mut package);
        let mut bytes = serde_json::to_vec_pretty(&package).unwrap();
        bytes.push(b'\n');
        fs::write(dir.join(EVIDENCE), &bytes).unwrap();
        remark(store, key, run, turn, |q| q["evidence"]["sha256"] = json!(hash(&bytes)));
    }

    /// Changes a file of the turn and binds the package to the new bytes, as a forger would.
    fn rebind(store: &Store, key: &str, run: &str, turn: &str, file: &str, change: impl FnOnce(String) -> String) {
        let dir = store.turn_dir(key, run, turn).unwrap();
        let text = change(fs::read_to_string(dir.join(file)).unwrap());
        fs::write(dir.join(file), &text).unwrap();
        let sha = Sha256Hex::try_from(hash(text.as_bytes())).unwrap();
        forge(store, key, run, turn, |p| {
            match file {
                "prepared/bundle/query.json" => p.bound.bundle_query_sha256 = sha.clone(),
                "prepared/request.json" => p.bound.request_sha256 = sha.clone(),
                _ => {}
            }
            if file.ends_with("query.json") && (file.starts_with("claims") == (p.representation == Representation::Claims)) {
                p.query.sha256 = sha;
            }
        });
    }

    /// Why a send of the turn is refused ("" when it is not), and the package bytes untouched.
    fn refusal_of(store: &Store, key: &str, run: &str, turn: &str) -> String {
        let file = store.turn_dir(key, run, turn).unwrap().join(EVIDENCE);
        let before = fs::read(&file).ok();
        let why = store.verify_for_send(key, run, turn).err().map(|e| e.to_string()).unwrap_or_default();
        assert_eq!(fs::read(&file).ok(), before, "a check rewrites nothing");
        why
    }

    #[test]
    fn h1_a_package_agrees_with_its_mark_and_with_the_files_of_its_turn() {
        let (_scratch, store, key, run) = opened_run();
        // 1. Whole, in both forms: the link names the package checked.
        for format in [AnswerFormat::Claims, AnswerFormat::Prose] {
            let turn = fresh(&store, &key, &run, format);
            let (question, bytes, _) = package_of(&store, &key, &run, &turn);
            let link = store.verify_for_send(&key, &run, &turn).unwrap().link;
            assert_eq!(link["evidence_sha256"], json!(hash(&bytes)));
            assert_eq!(question["evidence"]["sha256"], link["evidence_sha256"]);
            assert!(link["checks"].as_array().unwrap().ends_with(&[json!("evidence_h1"), json!("evidence_h2"), json!("evidence_h3")]));
        }
        let other = store.ask(&key, &run, &format!("{RAY} Em detalhe.")).unwrap()["id"].as_str().unwrap().to_owned();
        let other_request = hash(&fs::read(store.turn_dir(&key, &run, &other).unwrap().join("prepared/request.json")).unwrap());
        type Case = (&'static str, AnswerFormat, Box<dyn Fn(&Store, &str, &str, &str)>, &'static str);
        let cases: Vec<Case> = vec![
            // 2–4: the mark and the bytes.
            ("pacote ausente", AnswerFormat::Claims, Box::new(|s, k, r, t| fs::remove_file(s.turn_dir(k, r, t).unwrap().join(EVIDENCE)).unwrap()), "H1: pacote ausente"),
            ("pacote adulterado", AnswerFormat::Claims, Box::new(|s, k, r, t| {
                let file = s.turn_dir(k, r, t).unwrap().join(EVIDENCE);
                let text = fs::read_to_string(&file).unwrap().replacen("\"route\": \"search\"", "\"route\": \"chain\"", 1);
                fs::write(file, text).unwrap();
            }), "H1: o pacote não é o que a marca nomeia"),
            ("hash da marca", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["evidence"]["sha256"] = json!(hash(b"x")))), "H1: o pacote não é o que a marca nomeia"),
            ("hash da marca inválido", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["evidence"]["sha256"] = json!("zz"))), "H1: hash da marca inválido"),
            // 5–6: the package's own form.
            ("campo removido", AnswerFormat::Claims, Box::new(|s, k, r, t| forge_value(s, k, r, t, |p| { p.as_object_mut().unwrap().remove("selection"); })), "H1: pacote ilegível"),
            ("campo desconhecido", AnswerFormat::Claims, Box::new(|s, k, r, t| forge_value(s, k, r, t, |p| p["extra"] = json!(1))), "H1: pacote"),
            ("forma de gravação", AnswerFormat::Claims, Box::new(|s, k, r, t| forge_bytes(s, k, r, t, |b| b.extend_from_slice(b"\n"))), "H1: pacote fora da forma"),
            // 7–9: what it binds.
            ("request vinculado", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.bound.request_sha256 = Sha256Hex::try_from(hash(b"x")).unwrap())), "H1: request difere"),
            ("busca ausente", AnswerFormat::Prose, Box::new(|s, k, r, t| fs::remove_file(s.turn_dir(k, r, t).unwrap().join("prepared/search.json")).unwrap()), "H1: search difere"),
            ("contexto claims ausente", AnswerFormat::Claims, Box::new(|s, k, r, t| fs::remove_file(s.turn_dir(k, r, t).unwrap().join("claims/context.json")).unwrap()), "H1: claims_context difere"),
            ("contexto claims nulo", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.bound.claims_context_sha256 = None)), "H1: claims_context difere"),
            ("contexto em prosa", AnswerFormat::Prose, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.bound.claims_context_sha256 = Some(p.bound.dossier_sha256.clone()))), "H1: claims_context difere"),
            ("critérios inventados", AnswerFormat::Prose, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.bound.criteria_sha256 = Some(p.bound.dossier_sha256.clone()))), "H1: criteria difere"),
            ("query de prosa no lugar da de claims", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.query.sha256 = p.bound.bundle_query_sha256.clone())), "H1: query difere"),
            ("representação trocada", AnswerFormat::Prose, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["answer_format"] = json!("claims"))), "H1: representation difere"),
            ("request de outro turno", AnswerFormat::Claims, { let h = other_request.clone(); Box::new(move |s, k, r, t| forge(s, k, r, t, |p| p.bound.request_sha256 = Sha256Hex::try_from(h.clone()).unwrap())) }, "H1: request difere"),
            // 10–12: fichas and units.
            ("unidade removida", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| { p.units.pop(); })), "H1: UnknownReference"),
            ("ficha removida", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| { let last = p.units.last().unwrap().evidence_id.clone(); p.fichas.retain(|f| f.unit != last); })), "H1: Unlinked"),
            ("ficha em unidade inexistente", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.fichas[0].unit = evidence::UnitId("E99".into()))), "H1: "),
            ("ficha fora da janela", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| { let end = p.units[0].window_end_line; let f = p.fichas.iter_mut().find(|f| f.unit == p.units[0].evidence_id).unwrap(); f.start_line = end + 1; f.end_line = end + 1; })), "H1: FichaOutside"),
            ("ficha em outro caminho", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.fichas[0].path = "src/outro.rs".into())), "H1: FichaOutside"),
            ("unidade duplicada", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| { let u = p.units[0].clone(); p.units.push(u); })), "H1: Duplicate"),
            ("excerpt em prosa", AnswerFormat::Prose, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.units[0].excerpt_sha256 = Some(p.units[0].text_sha256.clone()))), "H1: Shape"),
            // 5.4: the edition and the versions it names.
            ("edition de outra origem", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.edition.origin = format!("edition:{}", hash(b"x")))), "H1: a edition do pacote"),
            ("catálogo de outra edition", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.edition.catalog_sha256 = Sha256Hex::try_from(hash(b"x")).unwrap())), "H1: a edition do pacote"),
            ("regras desconhecidas", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.versions.rules = evidence::RulesVersion("outra".into()))), "H1: regras outra desconhecidas"),
            ("pergunta registrada outra", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["asked"]["original"] = json!("Outra?"))), "H1: a pergunta registrada não é a do pacote"),
            // 13–14: marks that are not a package's.
            ("evidence.error", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["evidence"] = json!({ "error": "persistence_failed" }))), "não tem pacote de evidências (persistence_failed)"),
            ("formato desconhecido", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["evidence"]["format_version"] = json!(3))), "H1: marca do pacote em formato desconhecido"),
            ("formato em texto", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["evidence"]["format_version"] = json!("2"))), "H1: marca do pacote em formato desconhecido"),
            // P1: the mark's format is the package's, and format 2 tells the two measures compared.
            ("marca formato 1 de pacote formato 2", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["evidence"]["format_version"] = json!(1))), "H1: o formato da marca não é o do pacote"),
            ("pacote e marca formato 1 com campo do formato 2", AnswerFormat::Claims, Box::new(|s, k, r, t| {
                forge(s, k, r, t, |p| p.format_version = PackageFormat::V1);
                remark(s, k, r, t, |q| q["evidence"]["format_version"] = json!(1));
            }), "H1: Shape { what: \"measure divergence\""),
            ("divergência adulterada", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.measure.measure_divergence = Some(MeasureDivergence::Diverges {}))), "H1: Shape { what: \"measure divergence\""),
            ("divergência ausente", AnswerFormat::Prose, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.measure.measure_divergence = None)), "H1: pacote ilegível (measure has measure_diverges (format 1) or measure_divergence (format 2), exactly one"),
            ("os dois campos", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.measure.measure_diverges = Some(false))), "H1: pacote ilegível (measure has measure_diverges (format 1) or measure_divergence (format 2), exactly one"),
            ("measure_diverges no lugar do campo do formato 2", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| {
                p.measure.measure_divergence = None;
                p.measure.measure_diverges = Some(false);
            })), "H1: Shape { what: \"measure divergence\""),
            ("divergência nula no arquivo", AnswerFormat::Claims, Box::new(|s, k, r, t| forge_value(s, k, r, t, |v| v["measure"]["measure_divergence"] = Value::Null)), "H1: pacote ilegível (invalid type: null"),
            ("marca com campo a mais", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["evidence"]["extra"] = json!(1))), "H1: marca do pacote em formato desconhecido"),
            ("marca sem hash", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["evidence"] = json!({ "format_version": 1, "sha": "x" }))), "H1: hash da marca inválido"),
            ("marca que não é objeto", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["evidence"] = json!("ok"))), "H1: marca do pacote ilegível"),
            ("marca nula", AnswerFormat::Claims, Box::new(|s, k, r, t| remark(s, k, r, t, |q| q["evidence"] = Value::Null)), "H1: marca do pacote ilegível"),
        ];
        for (name, format, change, expected) in cases {
            let turn = fresh(&store, &key, &run, format);
            assert_eq!(refusal_of(&store, &key, &run, &turn), "", "{name}: whole before");
            change(&store, &key, &run, &turn);
            let why = refusal_of(&store, &key, &run, &turn);
            assert!(why.contains(expected) && why.ends_with("nada foi enviado"), "{name}: {why}");
        }
    }

    /// The package as JSON, changed and re-marked: what a hand edit of the file gives.
    fn forge_value(store: &Store, key: &str, run: &str, turn: &str, change: impl FnOnce(&mut Value)) {
        forge_bytes(store, key, run, turn, |bytes| {
            let mut value: Value = serde_json::from_slice(bytes).unwrap();
            change(&mut value);
            *bytes = serde_json::to_vec_pretty(&value).unwrap();
            bytes.push(b'\n');
        });
    }

    fn forge_bytes(store: &Store, key: &str, run: &str, turn: &str, change: impl FnOnce(&mut Vec<u8>)) {
        let file = store.turn_dir(key, run, turn).unwrap().join(EVIDENCE);
        let mut bytes = fs::read(&file).unwrap();
        change(&mut bytes);
        fs::write(&file, &bytes).unwrap();
        remark(store, key, run, turn, |q| q["evidence"]["sha256"] = json!(hash(&bytes)));
    }

    #[test]
    fn h1_an_older_question_keeps_its_checks_and_a_stripped_one_looks_older() {
        let (_scratch, store, key, run) = opened_run();
        // 15 and 44: no mark, no package: the checks it always had, and no hash invented.
        for format in [AnswerFormat::Claims, AnswerFormat::Prose] {
            let turn = fresh(&store, &key, &run, format);
            fs::remove_file(store.turn_dir(&key, &run, &turn).unwrap().join(EVIDENCE)).unwrap();
            remark(&store, &key, &run, &turn, |q| {
                q.as_object_mut().unwrap().remove("evidence");
                q.as_object_mut().unwrap().remove("asked");
            });
            let link = store.verify_for_send(&key, &run, &turn).unwrap().link;
            assert!(link.get("evidence_sha256").is_none());
            assert!(!link["checks"].as_array().unwrap().iter().any(|c| c.as_str().unwrap().starts_with("evidence")));
            if format == AnswerFormat::Claims {
                let pasted = store.paste_answer(&key, &run, &turn, &ray_fact()).unwrap();
                assert!(pasted["link"].get("evidence_sha256").is_none());
                assert_eq!(pasted["purpose"], "question");
            }
        }
        // The limit, kept honest: without the mark, a package left behind is not looked at.
        let turn = fresh(&store, &key, &run, AnswerFormat::Claims);
        remark(&store, &key, &run, &turn, |q| { q.as_object_mut().unwrap().remove("evidence"); });
        assert!(store.verify_for_send(&key, &run, &turn).unwrap().link.get("evidence_sha256").is_none());
    }

    #[test]
    fn h2_units_come_again_from_the_editions_snapshots() {
        let (_scratch, store, key, run) = opened_run();
        // 16: whole.
        let turn = fresh(&store, &key, &run, AnswerFormat::Claims);
        let (_, _, package) = package_of(&store, &key, &run, &turn);
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        let snapshots: Vec<SnapshotInput> = package.units.iter().map(|u| {
            let source = catalog.source_by_path(&u.path).unwrap();
            SnapshotInput { source_id: &source.id, path: &source.path, sha256: &source.sha256, bytes: catalog.source_bytes(&u.path).unwrap() }
        }).collect();
        assert_eq!(evidence::check_sources(&package, &snapshots), Ok(()));
        let sha = |b: &[u8]| Sha256Hex::try_from(hash(b)).unwrap();
        type Case = (&'static str, AnswerFormat, Box<dyn Fn(&mut EvidencePackage)>, &'static str);
        let cases: Vec<Case> = vec![
            ("fonte divergente", AnswerFormat::Claims, Box::new(move |p| p.units[0].source_sha256 = sha(b"x")), "H2: SourceDiffers"),
            ("fonte de outro id", AnswerFormat::Claims, Box::new(|p| p.units[0].source_id = "S99".into()), "H2: SourceDiffers"),
            ("excerpt alterado", AnswerFormat::Claims, Box::new(move |p| p.units[0].excerpt_sha256 = Some(sha(b"x"))), "H2: TextDiffers"),
            ("texto alterado", AnswerFormat::Prose, Box::new(move |p| p.units[0].text_sha256 = sha(b"x")), "H2: TextDiffers"),
            ("texto de claims alterado", AnswerFormat::Claims, Box::new(move |p| p.units[0].text_sha256 = sha(b"x")), "H2: TextDiffers"),
            ("linhas deslocadas", AnswerFormat::Prose, Box::new(|p| {
                let unit = &mut p.units[0];
                (unit.window_start_line, unit.window_end_line, unit.start_line, unit.end_line) = (unit.window_start_line + 1, unit.window_end_line + 1, unit.start_line + 1, unit.end_line + 1);
                let id = unit.evidence_id.clone();
                p.fichas.iter_mut().filter(|f| f.unit == id).for_each(|f| (f.start_line, f.end_line) = (f.start_line + 1, f.end_line + 1));
            }), "H2: "),
            ("fim de claims deslocado", AnswerFormat::Claims, Box::new(|p| p.units[0].end_line -= 1), "H2: TextDiffers"),
            ("janela fora do arquivo", AnswerFormat::Prose, Box::new(|p| { p.units[0].window_end_line = 100_000; p.units[0].end_line = 100_000; }), "H2: Normalization"),
            ("snapshot inexistente", AnswerFormat::Claims, Box::new(|p| {
                let unit = p.units[0].evidence_id.clone();
                p.units[0].path = "src/nao.rs".into();
                p.fichas.iter_mut().filter(|f| f.unit == unit).for_each(|f| f.path = "src/nao.rs".into());
            }), "H2: snapshot de src/nao.rs indisponível"),
        ];
        for (name, format, change, expected) in cases {
            let turn = fresh(&store, &key, &run, format);
            forge(&store, &key, &run, &turn, |p| change(p));
            let why = refusal_of(&store, &key, &run, &turn);
            assert!(why.contains(expected), "{name}: {why}");
        }
        // 17 and 27: the edition's snapshot changed, or the edition gone.
        let turn = fresh(&store, &key, &run, AnswerFormat::Claims);
        let edition = store.edition(&key, &run).unwrap();
        let snapshot = edition.join("snapshots").join(&catalog.source_by_path(&package.units[0].path).unwrap().sha256);
        let kept = fs::read(&snapshot).unwrap();
        fs::write(&snapshot, [kept.as_slice(), b"\n"].concat()).unwrap();
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H2: edition indisponível"));
        fs::write(&snapshot, &kept).unwrap();
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        let away = edition.with_extension("fora");
        fs::rename(&edition, &away).unwrap();
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H2: edition indisponível"));
        fs::rename(&away, &edition).unwrap();
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
    }

    /// A package of one unit over `bytes`, lines `start..=end`, as `lines-v1` makes it.
    fn over(package: &EvidencePackage, representation: Representation, bytes: &[u8], start: usize, end: usize) -> EvidencePackage {
        let mut p = package.clone();
        p.representation = representation;
        let window = evidence::normalize_window(bytes, start, end, representation).unwrap();
        let unit = &mut p.units[0];
        (unit.window_start_line, unit.window_end_line, unit.start_line, unit.end_line) = (start, end, window.start_line, window.end_line);
        (unit.text_sha256, unit.excerpt_sha256, unit.source_sha256) = (window.text_sha256, window.excerpt_sha256, Sha256Hex::try_from(hash(bytes)).unwrap());
        let id = unit.evidence_id.clone();
        p.units.truncate(1);
        p.fichas.retain(|f| f.unit == id);
        p
    }

    #[test]
    fn h2_follows_lines_v1_in_each_representation() {
        let (_scratch, store, key, run) = opened_run();
        let turn = fresh(&store, &key, &run, AnswerFormat::Claims);
        let (_, _, package) = package_of(&store, &key, &run, &turn);
        let unit = &package.units[0];
        let input = |bytes: &'static [u8], sha: &'static str| [SnapshotInput { source_id: &unit.source_id, path: &unit.path, sha256: sha, bytes }];
        let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
        // 23: CRLF is the text LF gives; the source is its own bytes.
        let (crlf, lf): (&[u8], &[u8]) = (b"fn a() {\r\n    1\r\n}\r\n", b"fn a() {\n    1\n}\n");
        for representation in [Representation::Claims, Representation::Prose] {
            let p = over(&package, representation, crlf, 1, 3);
            assert_eq!(evidence::check_sources(&p, &input(crlf, leak(hash(crlf)))), Ok(()));
            assert_eq!(p.units[0].text_sha256, over(&package, representation, lf, 1, 3).units[0].text_sha256);
            let mut swapped = p.clone();
            swapped.units[0].source_sha256 = Sha256Hex::try_from(hash(lf)).unwrap();
            assert!(matches!(evidence::check_sources(&swapped, &input(lf, leak(hash(lf)))), Ok(())), "same text, its own source");
            assert!(matches!(evidence::check_sources(&p, &input(lf, leak(hash(lf)))), Err(evidence::PackageCheck::SourceDiffers { .. })));
        }
        // 24: a \r\r\n ends one line, as lines-v1 says. Claims drop the stray \r (N5, N6), prose keeps
        // it: the check agrees with each, and a prose text taken as LF is refused.
        let rr: &[u8] = b"fn a() {\r\r\n    1\r\n}\r\n";
        for representation in [Representation::Claims, Representation::Prose] {
            let p = over(&package, representation, rr, 1, 3);
            assert_eq!(evidence::check_sources(&p, &input(rr, leak(hash(rr)))), Ok(()));
            assert_eq!(p.units[0].end_line, 3, "{representation:?}: one line, not two");
            let as_lf = over(&package, representation, lf, 1, 3).units[0].text_sha256.clone();
            assert_eq!(as_lf == p.units[0].text_sha256, representation == Representation::Claims, "{representation:?}");
        }
        let mut prose_as_lf = over(&package, Representation::Prose, rr, 1, 3);
        prose_as_lf.units[0].text_sha256 = over(&package, Representation::Prose, lf, 1, 3).units[0].text_sha256.clone();
        assert!(matches!(evidence::check_sources(&prose_as_lf, &input(rr, leak(hash(rr)))), Err(evidence::PackageCheck::TextDiffers { .. })));
        // 25: a claims window that N4 leaves empty has no text; prose keeps it.
        let blank: &[u8] = b"fn a() {}\n\n\n";
        let mut empty = over(&package, Representation::Prose, blank, 2, 3);
        assert_eq!(evidence::check_sources(&empty, &input(blank, leak(hash(blank)))), Ok(()));
        empty.representation = Representation::Claims;
        empty.units[0].excerpt_sha256 = Some(empty.units[0].text_sha256.clone());
        assert!(matches!(evidence::check_sources(&empty, &input(blank, leak(hash(blank)))), Err(evidence::PackageCheck::Normalization { .. })));
        // 26: prose keeps the empty lines at the end of its window; claims drops them.
        let prose = over(&package, Representation::Prose, blank, 1, 3);
        assert_eq!(prose.units[0].end_line, 3);
        assert_ne!(prose.units[0].text_sha256, over(&package, Representation::Prose, blank, 1, 1).units[0].text_sha256);
        let mut trimmed = prose.clone();
        trimmed.units[0].text_sha256 = over(&package, Representation::Prose, blank, 1, 1).units[0].text_sha256.clone();
        assert!(matches!(evidence::check_sources(&trimmed, &input(blank, leak(hash(blank)))), Err(evidence::PackageCheck::TextDiffers { .. })));
        assert_eq!(over(&package, Representation::Claims, blank, 1, 3).units[0].end_line, 1);
    }

    #[test]
    fn h3_the_current_code_makes_the_same_package_and_query_again() {
        let (_scratch, store, key, run) = opened_run();
        let sha = |b: &[u8]| Sha256Hex::try_from(hash(b)).unwrap();
        // 29, 34, 38: a package changed where only making it again tells.
        type Case = (&'static str, AnswerFormat, Box<dyn Fn(&mut EvidencePackage)>, &'static str);
        let cases: Vec<Case> = vec![
            ("campos reordenados", AnswerFormat::Claims, Box::new(|p| p.query.fields.swap(0, 1)), "H3: o código atual não refaz query"),
            ("campo a menos", AnswerFormat::Prose, Box::new(|p| { p.query.fields.pop(); }), "H3: o código atual não refaz query"),
            ("limite alterado", AnswerFormat::Claims, Box::new(move |p| p.query.limits[0].text_sha256 = sha(b"x")), "H3: o código atual não refaz query"),
            ("contrato fixo alterado", AnswerFormat::Claims, Box::new(move |p| p.query.fixed_contract_sha256 = sha(b"x")), "H3: o código atual não refaz query"),
            ("commit do projeto", AnswerFormat::Claims, Box::new(|p| p.edition.git_head = Some("0".repeat(40))), "H3: o código atual não refaz edition"),
            ("rota", AnswerFormat::Claims, Box::new(|p| p.route = evidence::Route::Chain), "H3: o código atual não refaz route"),
            ("fichas invertidas", AnswerFormat::Claims, Box::new(|p| p.fichas.reverse()), "H3: o código atual não refaz fichas"),
        ];
        for (name, format, change, expected) in cases {
            let turn = fresh(&store, &key, &run, format);
            let (_, _, package) = package_of(&store, &key, &run, &turn);
            forge(&store, &key, &run, &turn, |p| change(p));
            let (_, _, forged) = package_of(&store, &key, &run, &turn);
            if forged == package {
                continue; // the change is no change for this package
            }
            let why = refusal_of(&store, &key, &run, &turn);
            assert!(why.contains(expected), "{name}: {why}");
        }
        // 37: another builder, the same package otherwise: it passes, and is still the stored one.
        let turn = fresh(&store, &key, &run, AnswerFormat::Claims);
        forge(&store, &key, &run, &turn, |p| p.versions.builder = evidence::BuilderIdentity::Recorded { git_commit: "0".repeat(40), git_dirty: true });
        let (_, bytes, package) = package_of(&store, &key, &run, &turn);
        assert_eq!(store.verify_for_send(&key, &run, &turn).unwrap().link["evidence_sha256"], json!(hash(&bytes)));
        assert_eq!(package.versions.builder, evidence::BuilderIdentity::Recorded { git_commit: "0".repeat(40), git_dirty: true });
        // 30, 35, 36: the query files changed, the package bound to them: H1 and H2 pass, H3 does not.
        let turn = fresh(&store, &key, &run, AnswerFormat::Prose);
        rebind(&store, &key, &run, &turn, "prepared/bundle/query.json", |t| t.replacen("\"instructions\": [\n    \"", "\"instructions\": [\n    \"Outra. ", 1));
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H3: "), "instrução fixa");
        let turn = fresh(&store, &key, &run, AnswerFormat::Prose);
        rebind(&store, &key, &run, &turn, "prepared/bundle/query.json", |t| { assert!(t.contains(RAY)); t.replacen(RAY, "Outra?", 1) });
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H3: "), "query do bundle");
        let turn = fresh(&store, &key, &run, AnswerFormat::Claims);
        rebind(&store, &key, &run, &turn, "claims/query.json", |t| t.replacen(RAY, "Outra?", 1));
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H3: "), "query de claims");
        // 28: whole, the query and the package are made again as they are.
        let turn = fresh(&store, &key, &run, AnswerFormat::Claims);
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
    }

    /// Makes the question as asked say something else, with its package's hashes following: H1
    /// agrees, and only making the query again shows it was not what was prepared.
    fn reask(store: &Store, key: &str, run: &str, turn: &str, change: impl FnOnce(&mut String, &mut Vec<Suffix>)) {
        let dir = store.turn_dir(key, run, turn).unwrap();
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json")).unwrap()).unwrap();
        let (mut original, mut suffixes) = asked_of(&question).unwrap();
        change(&mut original, &mut suffixes);
        let joined: String = std::iter::once(original.as_str()).chain(suffixes.iter().map(|s| s.text.as_str())).collect();
        remark(store, key, run, turn, |q| {
            q["question"] = json!(joined);
            q["asked"] = json!({ "original": original, "suffixes": suffixes.iter().map(|s| json!({ "text": s.text, "rule": s.rule, "attempt": s.attempt })).collect::<Vec<_>>() });
        });
        let sha = |t: &str| Sha256Hex::try_from(hash(t.as_bytes())).unwrap();
        forge(store, key, run, turn, |p| {
            p.query.person_text.sha256 = sha(&original);
            p.query.prepared_question_sha256 = sha(&joined);
            for (entry, suffix) in p.query.server_suffixes.iter_mut().zip(&suffixes) {
                (entry.sha256, entry.rule, entry.attempt) = (sha(&suffix.text), suffix.rule, suffix.attempt);
            }
        });
    }

    #[test]
    fn h3_the_question_as_asked_is_the_one_prepared() {
        let (_scratch, store, key, run) = opened_run();
        let nodes = vec![symbol_id(&store, &key, &run, "ray_from_pixel")];
        let suffixes = [crate::gate_suffix(&["x".into()], 1), crate::mechanical_suffix(&["y".into()], 2)];
        let ask = || store.ask_gathered_parts(&key, &run, "O que muda?", &suffixes, &nodes, false, None, None, &[]).unwrap()["id"].as_str().unwrap().to_owned();
        // The ladder's question, with what the server added, is checked and linked to its package.
        let turn = ask();
        let (_, bytes, _) = package_of(&store, &key, &run, &turn);
        assert_eq!(store.verify_for_send(&key, &run, &turn).unwrap().link["evidence_sha256"], json!(hash(&bytes)));
        type Change = Box<dyn Fn(&mut String, &mut Vec<Suffix>)>;
        let cases: [(&str, Change); 3] = [
            ("31 texto da pessoa", Box::new(|o, _| o.push('!'))),
            ("32 acréscimo alterado", Box::new(|_, s| s[0].text.push('!'))),
            ("33 ordem trocada", Box::new(|_, s| s.swap(0, 1))),
        ];
        for (name, change) in cases {
            let turn = ask();
            reask(&store, &key, &run, &turn, |o, s| change(o, s));
            let why = refusal_of(&store, &key, &run, &turn);
            assert!(why.starts_with("H3: "), "{name}: {why}");
        }
        // The rule is the server's declaration: the package must say the one recorded (H1); that the
        // text is the one the rule writes is checked with the ladder's inputs, not here.
        let turn = ask();
        forge(&store, &key, &run, &turn, |p| p.query.server_suffixes[0].rule = RetryRule::MechanicalRefused);
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: a pergunta registrada"));
        // Without the forger's care, H1 already sees it.
        let turn = ask();
        remark(&store, &key, &run, &turn, |q| q["asked"]["suffixes"].as_array_mut().unwrap().swap(0, 1));
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: a pergunta registrada"));
    }

    #[test]
    fn a_send_is_linked_to_the_package_checked_and_says_what_it_was_for() {
        let (_scratch, store, key, run) = opened_run();
        let turn = fresh(&store, &key, &run, AnswerFormat::Claims);
        let (_, bytes, _) = package_of(&store, &key, &run, &turn);
        let choice = || crate::llm::Choice { modelfile: None, modelfile_name: None, claude_model: None, skills: vec![] };
        // 39: by a provider and by paste, the link has the hash of the package checked.
        let verified = store.verify_for_send(&key, &run, &turn).unwrap();
        let dir = next_send(&store.turn_dir(&key, &run, &turn).unwrap(), "test").unwrap();
        let sent = crate::llm::send_question("test", &choice(), &verified, &dir).unwrap();
        let recorded: Value = serde_json::from_slice(&fs::read(dir.join("send.json")).unwrap()).unwrap();
        assert_eq!((&recorded["link"]["evidence_sha256"], &recorded["purpose"]), (&json!(hash(&bytes)), &json!("question")));
        assert_eq!(recorded["link"], verified.link);
        assert_eq!(sent["link"], recorded["link"]);
        let pasted = store.paste_answer(&key, &run, &turn, &ray_fact()).unwrap();
        assert_eq!((&pasted["link"]["evidence_sha256"], &pasted["purpose"]), (&json!(hash(&bytes)), &json!("question")));
        // 45: the lexicon sends with no package and says so.
        let lexicon = store.suggest_lexicon(&key, &run, "test", crate::lexicon::LANGUAGES[0], choice()).unwrap();
        assert_eq!(lexicon["send"]["purpose"], "lexicon");
        assert!(lexicon["send"]["link"].is_null());
        assert!(!Path::new(lexicon["directory"].as_str().unwrap()).join(EVIDENCE).exists());
        // 47: a send whose record is not written is not confirmed, and earlier records stay.
        let sends = store.turn_dir(&key, &run, &turn).unwrap().join("sends");
        let before: Vec<_> = fs::read_dir(&sends).unwrap().map(|e| e.unwrap().path()).collect();
        let kept: Vec<_> = before.iter().map(|d| fs::read(d.join("send.json")).unwrap()).collect();
        FAIL_EVIDENCE_AT.with(|f| f.set(Some("send_record")));
        let pasted = store.paste_answer(&key, &run, &turn, &ray_fact());
        let dir = next_send(&store.turn_dir(&key, &run, &turn).unwrap(), "test").unwrap();
        let provider = crate::llm::send_question("test", &choice(), &store.verify_for_send(&key, &run, &turn).unwrap(), &dir);
        FAIL_EVIDENCE_AT.with(|f| f.set(None));
        assert!(pasted.unwrap_err().to_string().contains("não está vinculada"));
        assert!(provider.is_err());
        let after: Vec<_> = fs::read_dir(&sends).unwrap().map(|e| e.unwrap().path()).collect();
        assert!(after.iter().filter(|d| !before.contains(d)).all(|d| !d.join("send.json").exists()));
        assert_eq!(before.iter().map(|d| fs::read(d.join("send.json")).unwrap()).collect::<Vec<_>>(), kept);
    }

    #[test]
    fn a_question_reaches_a_model_only_through_its_check() {
        // 46: one way to send a question, taking what only the check makes; the free send is private.
        let llm = include_str!("llm.rs");
        assert!(llm.contains("pub fn send_question(provider: &str, choice: &Choice, verified: &crate::store::Verified, dir: &Path)"));
        assert!(llm.contains("\nfn send(provider: &str, request: Send, purpose: &str)") && !llm.contains("pub fn send("));
        assert!(llm.contains("\n    fn request<'a>"), "a request is made only here");
        let store = include_str!("store.rs");
        let verified = &store[store.find("pub struct Verified {").unwrap() + "pub struct Verified {".len()..];
        let verified = &verified[..verified.find("\n}").unwrap()];
        assert!(!verified.contains("pub "), "{verified}");
        assert_eq!(store.matches(concat!("Ok(Verified", " {")).count(), 1, "made in one place");
        for source in [include_str!("main.rs"), store, include_str!("facts.rs")] {
            assert!(!source.contains(concat!("llm::", "send(")) && !source.contains(concat!(".", "request(")));
        }
    }

    #[test]
    fn a_changed_package_stops_every_route_that_uses_its_question() {
        let (_scratch, store, key, run) = opened_run();
        let turn = fresh(&store, &key, &run, AnswerFormat::Claims);
        store.paste_answer(&key, &run, &turn, &ray_fact()).unwrap();
        let tampered = |turn: &str| forge(&store, &key, &run, turn, |p| p.units[0].text_sha256 = Sha256Hex::try_from(hash(b"x")).unwrap());
        let sends = |turn: &str| fs::read_dir(store.turn_dir(&key, &run, turn).unwrap().join("sends")).map(|d| d.count()).unwrap_or(0);
        // 42 first, while the send is there: the approval stops at gate 2; a rejection is only recorded.
        tampered(&turn);
        let review = store.review(&key, &run, &turn, "01-manual", "C1", true, "A linha 5 calcula direction.");
        assert!(review.unwrap_err().to_string().starts_with("Portão 2: a pergunta não confere mais: H2:"));
        assert!(store.facts(&key, &run).unwrap()["facts"].as_array().unwrap().is_empty());
        // 40: /api/send stops before any provider runs.
        let body = json!({ "key": key, "run": run, "turn": turn, "provider": "ollama" });
        assert!(crate::send_value(&store, &body).unwrap_err().to_string().starts_with("H2:"));
        assert_eq!(sends(&turn), 1);
        // 41: /api/paste.
        assert!(store.paste_answer(&key, &run, &turn, &ray_fact()).unwrap_err().to_string().starts_with("H2:"));
        assert_eq!(sends(&turn), 1);
        // 43: /api/continue checks the question it continues from, and makes no new one.
        let turns = store.turns(&key, &run).unwrap().as_array().unwrap().len();
        assert!(store.continue_turn(&key, &run, &turn, "E o que vem depois?").unwrap_err().to_string().starts_with("H2:"));
        assert_eq!(store.turns(&key, &run).unwrap().as_array().unwrap().len(), turns);
        // Whole, the same routes go on: a continuation has its own package.
        let whole = fresh(&store, &key, &run, AnswerFormat::Claims);
        store.paste_answer(&key, &run, &whole, &ray_fact()).unwrap();
        let next = store.continue_turn(&key, &run, &whole, "E o que vem depois?").unwrap();
        let next = next["id"].as_str().unwrap();
        let (_, bytes, package) = package_of(&store, &key, &run, next);
        assert_eq!(package.route, evidence::Route::Chain);
        assert_eq!(store.verify_for_send(&key, &run, next).unwrap().link["evidence_sha256"], json!(hash(&bytes)));
        // A send linked to another package than the one checked now is not promoted.
        let promoted = store.review(&key, &run, &whole, "01-manual", "C1", true, "A linha 5 calcula direction.").unwrap();
        assert_eq!(promoted["fact_id"], "F1");
        let file = store.turn_dir(&key, &run, &whole).unwrap().join("sends/01-manual/send.json");
        let mut summary: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        summary["link"]["evidence_sha256"] = json!(hash(b"x"));
        fs::write(&file, serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
        store.review(&key, &run, &whole, "01-manual", "C1", false, "Revisão.").unwrap();
        let again = store.review(&key, &run, &whole, "01-manual", "C1", true, "A linha 5 calcula direction.");
        assert!(again.unwrap_err().to_string().contains("o pacote de evidências da pergunta não é o vinculado ao envio"));
    }

    #[test]
    fn a_claims_question_without_its_claims_files_is_never_sent() {
        // 48: no claims query, no package, and an error mark that no reader takes for a package.
        let (_scratch, store, key, run) = opened_run();
        FAIL_EVIDENCE_AT.with(|f| f.set(Some("claims")));
        let asked = store.ask(&key, &run, RAY);
        FAIL_EVIDENCE_AT.with(|f| f.set(None));
        let turn = asked.unwrap()["id"].as_str().unwrap().to_owned();
        let dir = store.turn_dir(&key, &run, &turn).unwrap();
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json")).unwrap()).unwrap();
        assert!(question["claims_error"].is_string());
        assert_eq!(question["evidence"], json!({ "error": "claims_not_prepared" }));
        assert!(!dir.join(EVIDENCE).exists() && !dir.join("claims/query.json").exists());
        assert!(dir.join("prepared/bundle/query.json").is_file(), "the bundle was prepared");
        let refused = |store: &Store| {
            assert!(store.verify_for_send(&key, &run, &turn).err().unwrap().to_string().contains("claims_not_prepared"));
            let body = json!({ "key": key, "run": run, "turn": turn, "provider": "ollama" });
            assert!(crate::send_value(store, &body).is_err());
            assert!(store.paste_answer(&key, &run, &turn, &ray_fact()).is_err());
            assert!(store.continue_turn(&key, &run, &turn, "E depois?").is_err());
            assert!(!dir.join("sends").exists());
        };
        refused(&store);
        // Stripped of its mark it looks older, and the older checks still find no claims query.
        remark(&store, &key, &run, &turn, |q| { q.as_object_mut().unwrap().remove("evidence"); });
        assert!(store.verify_for_send(&key, &run, &turn).is_err());
        assert!(store.paste_answer(&key, &run, &turn, &ray_fact()).is_err());
        assert!(!dir.join("sends").exists());
    }

    #[test]
    fn each_part_a_package_binds_is_checked_on_its_own() {
        let (_scratch, store, key, run) = opened_run();
        let x = || Sha256Hex::try_from(hash(b"x")).unwrap();
        type Case = (&'static str, AnswerFormat, Box<dyn Fn(&Store, &str, &str, &str)>, &'static str);
        let cases: Vec<Case> = vec![
            ("dossiê vinculado", AnswerFormat::Claims, Box::new(move |s, k, r, t| forge(s, k, r, t, |p| p.bound.dossier_sha256 = x())), "H1: dossier difere"),
            ("query do bundle vinculada", AnswerFormat::Claims, Box::new(move |s, k, r, t| forge(s, k, r, t, |p| p.bound.bundle_query_sha256 = x())), "H1: bundle_query difere"),
            ("contexto claims ausente e nulo", AnswerFormat::Claims, Box::new(|s, k, r, t| {
                fs::remove_file(s.turn_dir(k, r, t).unwrap().join("claims/context.json")).unwrap();
                forge(s, k, r, t, |p| p.bound.claims_context_sha256 = None);
            }), "H1: claims_context difere"),
            ("texto da pessoa no pacote", AnswerFormat::Claims, Box::new(move |s, k, r, t| forge(s, k, r, t, |p| p.query.person_text.sha256 = x())), "H1: a pergunta registrada"),
            ("registro do texto sem a pergunta", AnswerFormat::Claims, Box::new(|s, k, r, t| {
                remark(s, k, r, t, |q| q["asked"]["original"] = json!("Outra?"));
                forge(s, k, r, t, |p| p.query.person_text.sha256 = Sha256Hex::try_from(hash(b"Outra?")).unwrap());
            }), "H1: a pergunta registrada"),
            ("ficha duplicada", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| { let f = p.fichas[0].clone(); p.fichas.push(f); })), "H1: Duplicate"),
            ("fim da prosa antes da janela", AnswerFormat::Prose, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.units[0].end_line -= 1)), "H1: Shape"),
            ("início fora da janela", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| p.units[0].start_line += 1)), "H1: Shape"),
            ("classe code sem a unidade", AnswerFormat::Claims, Box::new(|s, k, r, t| forge(s, k, r, t, |p| { p.query.code.units.pop(); })), "H1: Unlinked"),
            // The bytes of the query, beyond what its fields say, are the prepared ones (H3).
            ("query do bundle com outros bytes", AnswerFormat::Prose, Box::new(|s, k, r, t| rebind(s, k, r, t, "prepared/bundle/query.json", |q| format!("{} \n", q.trim_end()))), "H3: "),
        ];
        for (name, format, change, expected) in cases {
            let turn = fresh(&store, &key, &run, format);
            change(&store, &key, &run, &turn);
            let why = refusal_of(&store, &key, &run, &turn);
            assert!(why.contains(expected), "{name}: {why}");
        }
        // An older question edited after it was asked keeps being refused by the checks it had.
        let turn = fresh(&store, &key, &run, AnswerFormat::Prose);
        fs::remove_file(store.turn_dir(&key, &run, &turn).unwrap().join(EVIDENCE)).unwrap();
        remark(&store, &key, &run, &turn, |q| { q.as_object_mut().unwrap().remove("evidence"); });
        let file = store.turn_dir(&key, &run, &turn).unwrap().join("prepared/bundle/query.json");
        let text = fs::read_to_string(&file).unwrap();
        fs::write(&file, format!("{} \n", text.trim_end())).unwrap();
        assert!(store.verify_for_send(&key, &run, &turn).is_err());
    }

    #[test]
    fn what_the_question_was_made_with_is_kept_and_not_read_again() {
        let (_scratch, store, key, mut settings) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        // The instructions the claims query was written with, not the project's now.
        settings.instructions = Some(vec!["Responda curto.".into()]);
        store.save_settings(&key, &settings).unwrap();
        let turn = store.ask(&key, &run, RAY).unwrap()["id"].as_str().unwrap().to_owned();
        settings.instructions = Some(vec!["Outra.".into()]);
        store.save_settings(&key, &settings).unwrap();
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // The measure read with the question is the one it names; a check does not read it again.
        // What it names is not made again either: that is the limit of H3 for this part.
        forge(&store, &key, &run, &turn, |p| p.measure.at_question = MeasureAtQuestion::Unreadable {});
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
    }

    /// The page's envelope as web/index.html keeps it (`toggleGathered`, `toggleTarget`, `includeBulk`,
    /// the join button, `absorbSuggestion` and `selectionOnRecord`), written again step for step, so a
    /// test sends what the page sends and not what the server's own check would make.
    struct Page {
        sources: Vec<(String, Value)>,
        decisions: Vec<(String, Value)>,
        gaps: Vec<Value>,
        gathered: Vec<String>,
        seeds: Vec<String>,
        suggestions: Vec<Value>,
        last: Option<Value>,
        nodes: Value,
    }

    impl Page {
        fn new(store: &Store, key: &str, run: &str) -> Self {
            let graph: Value = serde_json::from_slice(&store.graph(key, run).unwrap()).unwrap();
            Page { sources: vec![], decisions: vec![], gaps: vec![], gathered: vec![], seeds: vec![], suggestions: vec![], last: None, nodes: graph["nodes"].clone() }
        }
        fn source(&self, id: &str) -> Option<&Value> {
            self.sources.iter().find(|(k, _)| k == id).map(|(_, v)| v)
        }
        /// A Map's `set`: a known key keeps its place.
        fn set(&mut self, id: &str, value: Value) {
            match self.sources.iter_mut().find(|(k, _)| k == id) {
                Some((_, v)) => *v = value,
                None => self.sources.push((id.to_owned(), value)),
            }
        }
        fn decide(&mut self, id: &str, decision: &str, scope: &str) {
            let d = json!({ "decision": decision, "scope": scope });
            match self.decisions.iter_mut().find(|(k, _)| k == id) {
                Some((_, v)) => *v = d,
                None => self.decisions.push((id.to_owned(), d)),
            }
        }
        fn decision(&self, id: &str) -> Option<&Value> {
            self.decisions.iter().find(|(k, _)| k == id).map(|(_, v)| v)
        }
        /// `manualSource`: what the graph shows of a source, nothing more.
        fn manual(&self, id: &str) -> Value {
            let node = self.nodes.get(id);
            json!({ "id": id, "name": node.map_or(json!(id), |n| n["label"].clone()), "path": null, "kind": node.map_or(Value::Null, |n| n["kind"].clone()),
                "start_line": null, "end_line": null, "role": null, "bytes": null, "calls": null, "reasons": [], "facts": [], "left_out_why": null, "kept_out": null, "provenance": "manual" })
        }
        fn toggle(&mut self, id: &str) {
            let removing = self.gathered.iter().any(|g| g == id);
            if removing { self.gathered.retain(|g| g != id) } else { self.gathered.push(id.to_owned()) }
            if self.source(id).is_none() {
                let manual = self.manual(id);
                self.set(id, manual);
            }
            self.decide(id, if removing { "removed" } else { "included" }, "single");
            if removing { self.seeds.retain(|g| g != id) } else if self.last.is_none() { self.seeds.push(id.to_owned()) }
        }
        fn toggle_target(&mut self, id: &str) {
            if self.seeds.iter().any(|s| s == id) { self.seeds.retain(|s| s != id) } else { self.seeds.push(id.to_owned()) }
        }
        fn bulk(&mut self, ids: &[String]) {
            for id in ids {
                if self.source(id).is_none() {
                    let manual = self.manual(id);
                    self.set(id, manual);
                }
                self.decide(id, "included", "bulk");
            }
        }
        fn learn(&mut self, e: &Value, provenance: &str) {
            let id = e["id"].as_str().unwrap();
            let before = self.source(id).cloned();
            let given = |v: Option<&Value>| v.filter(|v| !v.is_null()).cloned();
            let mut reasons: Vec<Value> = before.as_ref().map_or(vec![], |b| b["reasons"].as_array().unwrap().clone());
            let new: Vec<Value> = e["reasons"].as_array().unwrap().iter().filter(|r| !reasons.contains(r)).cloned().collect();
            reasons.extend(new);
            let left_out = given(e.get("left_out")).unwrap_or(Value::Null);
            let value = json!({ "id": id, "name": e["name"], "path": e["path"], "kind": e["kind"], "start_line": e["start_line"], "end_line": e["end_line"], "role": e["role"],
                "bytes": e["bytes"], "calls": given(e.get("calls")).unwrap_or(Value::Null), "facts": given(e.get("facts")).unwrap_or(json!([])), "reasons": reasons,
                "left_out_why": before.as_ref().and_then(|b| given(b.get("left_out_why"))).unwrap_or(left_out.clone()), "kept_out": left_out,
                "provenance": before.as_ref().and_then(|b| given(b.get("provenance"))).unwrap_or(json!(provenance)) });
            self.set(id, value);
        }
        /// `absorbSuggestion`, and the hash the route answered with.
        fn absorb(&mut self, s: &Value, nodes: &[String]) {
            for e in s["items"].as_array().unwrap() {
                self.learn(e, "suggested");
            }
            for e in s["left_out"].as_array().unwrap() {
                self.learn(e, "left_out");
            }
            let new: Vec<Value> = s["gaps"].as_array().unwrap().iter().filter(|g| !self.gaps.contains(g)).cloned().collect();
            self.gaps.extend(new);
            self.last = Some(json!({ "suggested_from": nodes, "budget_bytes": s["budget_bytes"], "suggested_bytes": s["used_bytes"], "measured": s["measured"] }));
            self.suggestions.push(s["suggestion"].clone());
        }
        fn suggest(&mut self, store: &Store, key: &str, run: &str, nodes: &[String]) -> Value {
            let answer = store.suggest_envelope(key, run, nodes).unwrap();
            self.absorb(&answer, nodes);
            answer
        }
        /// The join button: what the suggestion adds that is not in and was not taken out.
        fn join(&mut self, s: &Value) {
            let to_join: Vec<String> = s["items"].as_array().unwrap().iter().map(|i| i["id"].as_str().unwrap().to_owned())
                .filter(|id| !self.gathered.contains(id) && self.decision(id).is_none_or(|d| d["decision"] != "removed")).collect();
            self.bulk(&to_join);
            self.gathered.extend(to_join);
        }
        /// `selectionOnRecord`.
        fn record(&self) -> Value {
            let decided = |e: &Value| {
                let mut e = e.clone();
                let fields = e.as_object_mut().unwrap();
                fields.remove("kept_out");
                let d = self.decision(fields["id"].as_str().unwrap());
                fields.insert("decision".into(), d.map_or(Value::Null, |d| d["decision"].clone()));
                fields.insert("decision_scope".into(), d.map_or(Value::Null, |d| d["scope"].clone()));
                e
            };
            let removed = |e: &Value| self.decision(e["id"].as_str().unwrap()).is_some_and(|d| d["decision"] == "removed");
            let others: Vec<&Value> = self.sources.iter().filter(|(id, _)| !self.gathered.contains(id)).map(|(_, v)| v).collect();
            let included: Vec<Value> = self.gathered.iter().map(|id| decided(self.source(id).unwrap())).collect();
            let last = self.last.as_ref().unwrap();
            json!({ "kind": "selection_record", "format_version": 2, "suggestions": self.suggestions, "seeds": self.seeds,
                "suggested_from": last["suggested_from"], "measured": last["measured"], "budget_bytes": last["budget_bytes"], "suggested_bytes": last["suggested_bytes"],
                "included_bytes": included.iter().map(|e| e["bytes"].as_u64().unwrap_or(0)).sum::<u64>(), "included": included,
                "removed": others.iter().filter(|e| removed(e)).map(|e| decided(e)).collect::<Vec<_>>(),
                "left_out": others.iter().filter(|e| !removed(e) && !e["kept_out"].is_null()).map(|e| { let mut d = decided(e); d["why"] = e["kept_out"].clone(); d }).collect::<Vec<_>>(),
                "not_joined": others.iter().filter(|e| !removed(e) && e["kept_out"].is_null()).map(|e| decided(e)).collect::<Vec<_>>(),
                "gaps": self.gaps })
        }
        fn origin(&self) -> Value {
            json!({ "kind": "selection", "summary": "x", "selection": self.record() })
        }
    }

    /// A request as the page sends it: through the socket, the parser, the route and `handle`, and
    /// read back from the bytes answered. The server side runs apart, so a long answer never blocks.
    fn api(store: &Store, path: &str, body: &Value) -> (u16, Value) {
        api_with(store, "POST", path, body)
    }

    /// `api` with another method: GET with the query in `path`, DELETE with a body.
    fn api_with(store: &Store, method: &str, path: &str, body: &Value) -> (u16, Value) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let text = if body.is_null() { String::new() } else { body.to_string() };
        let mut client = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        write!(client, "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:0\r\nContent-Length: {}\r\n\r\n{text}", text.len()).unwrap();
        let raw = std::thread::scope(|scope| {
            scope.spawn(|| {
                let (server, _) = listener.accept().unwrap();
                let request = crate::http::read(&server).unwrap();
                let app = crate::App { port: 0, viewer: PathBuf::new(), store: Store { dir: store.dir.clone(), modelfiles: store.modelfiles.clone() }, graph: std::sync::Mutex::new(None) };
                crate::http::write(&server, &crate::handle(&app, &request)).unwrap();
            });
            let mut raw = vec![];
            client.read_to_end(&mut raw).unwrap();
            raw
        });
        let text = String::from_utf8(raw).unwrap();
        let (head, body) = text.split_once("\r\n\r\n").unwrap();
        (head.split(' ').nth(1).unwrap().parse().unwrap(), serde_json::from_str(body).unwrap_or(Value::String(body.to_owned())))
    }

    /// Every file under the run's questions and suggestions, with its bytes.
    fn files_of(store: &Store, key: &str, run: &str) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() { walk(&path, out) } else { out.push((path.clone(), fs::read(&path).unwrap())) }
            }
        }
        let mut out = vec![];
        let run_dir = store.run_dir(key, run).unwrap();
        walk(&run_dir.join("turns"), &mut out);
        walk(&run_dir.join(SUGGESTIONS), &mut out);
        out.sort();
        out
    }

    /// A declaration gathered by hand, a suggestion from it joined whole, and what it added (its owner
    /// type) taken out again: one source in `included`, by hand, and one in `removed`, suggested.
    fn gathered_page() -> (Scratch, Store, String, String, Page) {
        let (scratch, store, key, _) = opened();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let (ray, camera) = (symbol_id(&store, &key, &run, "ray_from_pixel"), symbol_id(&store, &key, &run, "Camera"));
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&ray);
        let answer = page.suggest(&store, &key, &run, std::slice::from_ref(&ray));
        page.join(&answer);
        assert_eq!(page.gathered, [ray.clone(), camera.clone()], "the suggestion this project gives");
        page.toggle(&camera);
        let record = page.record();
        assert_eq!((record["included"][0]["provenance"].as_str(), record["removed"][0]["provenance"].as_str()), (Some("manual"), Some("suggested")));
        (scratch, store, key, run, page)
    }

    #[test]
    fn the_pages_selection_is_accepted_and_kept_with_its_question() {
        let (_scratch, store, key, run, page) = gathered_page();
        let body = json!({ "key": key, "run": run, "question": "O que muda?", "nodes": page.gathered, "with_facts": false, "origin": page.origin() });
        let (status, turn) = api(&store, "/api/gathered-question", &body);
        assert_eq!(status, 200, "{turn}");
        let id = turn["id"].as_str().unwrap();
        let dir = store.turn_dir(&key, &run, id).unwrap();
        let (question, _, package) = package_of(&store, &key, &run, id);
        // Kept beside the question, by the hash of its exact bytes, which the package names.
        let bytes = fs::read(dir.join(SELECTION)).unwrap();
        let reference = package.selection.clone().unwrap();
        assert_eq!(reference.record_sha256.as_str(), hash(&bytes));
        let record = SelectionRecord::from_bytes(&bytes, &hash(&bytes)).unwrap();
        assert_eq!(serde_json::to_value(reference.record_format).unwrap(), json!(2));
        assert_eq!(reference.suggestions, page.suggestions.iter().map(|s| Sha256Hex::try_from(s.as_str().unwrap().to_owned()).unwrap()).collect::<Vec<_>>());
        assert_eq!(Some(record.received.clone()), SelectionInput::from_value(&question["gathered"]["origin"]["selection"]).ok());
        // What the server checked is kept apart from what only the client says.
        let written: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(written["checked_by_server"]["declared_by"], "server");
        assert_eq!(written["declared_by_client"]["declared_by"], "client");
        assert_eq!(written["checked_by_server"]["checks"], json!(["shape", "included_is_nodes", "partition", "seeds_included", "edition", "suggestions", "absorption", "included_bytes"]));
        assert_eq!(written["declared_by_client"]["seeds"], json!(page.seeds));
        assert!(!String::from_utf8_lossy(&bytes).contains("\"created\""), "no clock in a record named by its hash");
        // The same selection again: the same bytes, a second copy in its own question.
        let (_, again) = api(&store, "/api/gathered-question", &body);
        assert_eq!(fs::read(store.turn_dir(&key, &run, again["id"].as_str().unwrap()).unwrap().join(SELECTION)).unwrap(), bytes);
        // Kept once and for good: equal bytes are the same record, so publishing them again is no change.
        publish_immutable(&dir, SELECTION, &bytes).unwrap();
        assert!(store.verify_for_send(&key, &run, id).is_ok());
    }

    #[test]
    fn a_selection_is_refused_before_anything_is_created() {
        let (_scratch, store, key, run, page) = gathered_page();
        let origin = page.origin();
        let refused = |origin: &Value, nodes: &[String]| -> String {
            let before = files_of(&store, &key, &run);
            let (status, body) = api(&store, "/api/gathered-question", &json!({ "key": key, "run": run, "question": "O que muda?", "nodes": nodes, "with_facts": false, "origin": origin }));
            assert_eq!(files_of(&store, &key, &run), before, "nothing created or changed");
            assert_eq!(status, 400, "{body}");
            body.to_string()
        };
        // Its nodes in another order are not its included sources.
        let mut reversed = page.gathered.clone();
        reversed.reverse();
        if reversed != page.gathered {
            assert!(refused(&origin, &reversed).contains("included não é a lista de nodes"));
        }
        let change = |edit: &dyn Fn(&mut Value)| { let mut o = origin.clone(); edit(&mut o["selection"]); o };
        assert!(refused(&change(&|s| s["format_version"] = json!(1)), &page.gathered).contains("format_version não é 2"));
        assert!(refused(&change(&|s| { s.as_object_mut().unwrap().remove("format_version"); }), &page.gathered).contains("format_version não é 2"));
        assert!(refused(&change(&|s| s["included"][0]["role"] = json!("dono")), &page.gathered).contains("forma"));
        // The suggestions it cites must be this run's records, as written.
        let record_file = |sha: &str| store.run_dir(&key, &run).unwrap().join(SUGGESTIONS).join(format!("{sha}.json"));
        let cited = page.suggestions[0].as_str().unwrap().to_owned();
        let stored = fs::read(record_file(&cited)).unwrap();
        assert!(refused(&change(&|s| s["suggestions"][0] = json!("abc")), &page.gathered).contains("forma"));
        assert!(refused(&change(&|s| s["suggestions"][0] = json!("0".repeat(64))), &page.gathered).contains(&format!("sugestão {}", "0".repeat(64))));
        // A file named by a hash its bytes do not have.
        let other = "1".repeat(64);
        fs::write(record_file(&other), &stored).unwrap();
        assert!(refused(&change(&|s| s["suggestions"][0] = json!(other)), &page.gathered).contains("holds bytes of"));
        fs::remove_file(record_file(&other)).unwrap();
        // A record of another edition, or of other rules, written as a record is and named by its own
        // hash; one of another format, which does not read as a record.
        let typed = SuggestionRecord::from_bytes(&stored, &cited).unwrap();
        let mut other_edition = typed.clone();
        other_edition.edition_origin = "edition:outra".into();
        let mut other_rules = typed.clone();
        other_rules.rules_version = RulesVersion("envelope-0".into());
        let mut other_format: Value = serde_json::from_slice(&stored).unwrap();
        other_format["format_version"] = json!(2);
        let format_bytes = { let mut b = serde_json::to_vec_pretty(&other_format).unwrap(); b.push(b'\n'); b };
        for (bytes, why) in [(other_edition.to_bytes().unwrap(), "é de outra edition"), (other_rules.to_bytes().unwrap(), "regras envelope-0"), (format_bytes, "malformed suggestion record")] {
            let sha = hash(&bytes);
            fs::write(record_file(&sha), &bytes).unwrap();
            assert!(refused(&change(&|s| s["suggestions"][0] = json!(sha)), &page.gathered).contains(why), "{why}");
            fs::remove_file(record_file(&sha)).unwrap();
        }
        // A record of another run of the same edition is not this run's.
        let other_run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let before = files_of(&store, &key, &other_run);
        let (status, body) = api(&store, "/api/gathered-question", &json!({ "key": key, "run": other_run, "question": "O que muda?", "nodes": page.gathered, "origin": origin }));
        assert_eq!((status, files_of(&store, &key, &other_run) == before), (400, true), "{body}");
        assert!(body.to_string().contains(&format!("sugestão {cited}")), "{body}");
        // An origin that says selection has one; one with a selection says selection or ladder.
        assert!(refused(&json!({ "kind": "selection", "summary": "x" }), &page.gathered).contains("origem selection sem seleção"));
        assert!(refused(&json!({ "kind": "hot", "summary": "x", "selection": origin["selection"] }), &page.gathered).contains("só uma origem selection ou ladder"));
        assert!(refused(&json!({ "kind": "selection", "summary": "x", "selection": null }), &page.gathered).contains("não é um objeto"));
    }

    #[test]
    fn the_selection_record_is_checked_before_a_send_and_made_again() {
        let (_scratch, store, key, run, page) = gathered_page();
        let turn = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        let file = store.turn_dir(&key, &run, &turn).unwrap().join(SELECTION);
        let kept = fs::read(&file).unwrap();
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // Bytes changed under the same name: not the record the package names.
        let mut changed = kept.clone();
        changed.insert(1, b' ');
        fs::write(&file, &changed).unwrap();
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: registro de seleção: o registro"), "{}", refusal_of(&store, &key, &run, &turn));
        fs::remove_file(&file).unwrap();
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: registro de seleção ausente"));
        fs::write(&file, &kept).unwrap();
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // The question saying it sent another selection.
        remark(&store, &key, &run, &turn, |q| q["gathered"]["origin"]["selection"]["seeds"] = json!([]));
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: a seleção da pergunta não é a do registro"));
        remark(&store, &key, &run, &turn, |q| q["gathered"]["origin"]["selection"]["seeds"] = json!(page.seeds));
        // A record rewritten whole and bound by the forger: H1 agrees, the code does not make it again.
        let mut forged = SelectionRecord::from_bytes(&kept, &hash(&kept)).unwrap();
        forged.checked_by_server.included_bytes += 1;
        let bytes = forged.to_bytes().unwrap();
        fs::write(&file, &bytes).unwrap();
        forge(&store, &key, &run, &turn, |p| p.selection.as_mut().unwrap().record_sha256 = Sha256Hex::try_from(hash(&bytes)).unwrap());
        assert_eq!(refusal_of(&store, &key, &run, &turn), "H3: o código atual não refaz o registro de seleção; nada foi enviado");
        // A suggestion it cites, gone since: the record cannot be made again.
        fs::write(&file, &kept).unwrap();
        forge(&store, &key, &run, &turn, |p| p.selection.as_mut().unwrap().record_sha256 = Sha256Hex::try_from(hash(&kept)).unwrap());
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        let suggestion = store.run_dir(&key, &run).unwrap().join(SUGGESTIONS).join(format!("{}.json", page.suggestions[0].as_str().unwrap()));
        let saved = fs::read(&suggestion).unwrap();
        fs::remove_file(&suggestion).unwrap();
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H3: registro de seleção não refeito"));
        fs::write(&suggestion, saved).unwrap();
        // A package naming the record's suggestions in another way than the record does.
        forge(&store, &key, &run, &turn, |p| p.selection.as_mut().unwrap().suggestions.insert(0, Sha256Hex::try_from("0".repeat(64)).unwrap()));
        assert_eq!(refusal_of(&store, &key, &run, &turn), "H1: o registro de seleção não é o que o pacote diz; nada foi enviado");
        forge(&store, &key, &run, &turn, |p| { p.selection.as_mut().unwrap().suggestions.remove(0); });
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // A record of another edition, bound by the forger.
        let mut elsewhere = SelectionRecord::from_bytes(&kept, &hash(&kept)).unwrap();
        elsewhere.edition_origin = "edition:outra".into();
        let bytes = elsewhere.to_bytes().unwrap();
        fs::write(&file, &bytes).unwrap();
        forge(&store, &key, &run, &turn, |p| p.selection.as_mut().unwrap().record_sha256 = Sha256Hex::try_from(hash(&bytes)).unwrap());
        assert_eq!(refusal_of(&store, &key, &run, &turn), "H1: a seleção da pergunta não é a do registro; nada foi enviado");
        fs::write(&file, &kept).unwrap();
        forge(&store, &key, &run, &turn, |p| p.selection.as_mut().unwrap().record_sha256 = Sha256Hex::try_from(hash(&kept)).unwrap());
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // A record no package names, or a package naming none with a basis that needs one.
        forge(&store, &key, &run, &turn, |p| p.selection = None);
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: Shape { what: \"selection basis\""));
        // A forger who also makes the comparison of the two measures agree with the basis (format 2).
        forge(&store, &key, &run, &turn, |p| p.measure.selection_basis = MeasureSelectionBasis::NotApplicable {});
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: Shape { what: \"measure divergence\""));
        forge(&store, &key, &run, &turn, |p| p.measure.measure_divergence = Some(MeasureDivergence::NotApplicable {}));
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: registro de seleção que o pacote não nomeia"));
    }

    #[test]
    fn a_file_head_is_checked_by_the_envelopes_rule() {
        // A file whose head (its imports) comes before the first declaration: the suggestion brings it.
        let lib = "use std::fmt;\nuse std::io;\n\npub struct Camera;\n\nimpl Camera {\n    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;\n        direction\n    }\n}\n";
        let (_scratch, store, key, _root) = project_with(&[("src/lib.rs", lib)], json!({ "project_root": "." }));
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let ray = symbol_id(&store, &key, &run, "ray_from_pixel");
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&ray);
        let answer = page.suggest(&store, &key, &run, std::slice::from_ref(&ray));
        page.join(&answer);
        assert!(page.gathered.iter().any(|id| id == "header:src/lib.rs"), "{:?}", page.gathered);
        let head = page.record()["included"].as_array().unwrap().iter().find(|e| e["id"] == "header:src/lib.rs").unwrap().clone();
        assert_eq!((head["start_line"].as_u64(), head["end_line"].as_u64(), head["bytes"].as_u64()), (Some(1), Some(2), Some(14 + 13)));
        let turn = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap();
        assert!(package_of(&store, &key, &run, turn["id"].as_str().unwrap()).2.selection.is_some());
        assert_eq!(refusal_of(&store, &key, &run, turn["id"].as_str().unwrap()), "");
    }

    #[test]
    fn a_selection_record_is_never_overwritten() {
        let (_scratch, store, key, run, page) = gathered_page();
        let turn = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        let dir = store.turn_dir(&key, &run, &turn).unwrap();
        let kept = fs::read(dir.join(SELECTION)).unwrap();
        assert!(publish_immutable(&dir, SELECTION, b"{}\n").is_err());
        assert_eq!(fs::read(dir.join(SELECTION)).unwrap(), kept);
    }

    #[test]
    fn a_selection_record_not_kept_leaves_its_question_with_no_package() {
        let (_scratch, store, key, run, page) = gathered_page();
        FAIL_EVIDENCE_AT.with(|f| f.set(Some("selection")));
        let error = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap_err().to_string();
        FAIL_EVIDENCE_AT.with(|f| f.set(None));
        assert!(error.contains("selection_persistence_failed"), "{error}");
        let dir = store.turn_dir(&key, &run, "01").unwrap();
        let question: Value = serde_json::from_slice(&fs::read(dir.join("question.json")).unwrap()).unwrap();
        assert_eq!(question["evidence"], json!({ "error": "selection_persistence_failed" }));
        assert!(!dir.join(EVIDENCE).exists() && !dir.join(SELECTION).exists());
        assert!(store.verify_for_send(&key, &run, "01").err().unwrap().to_string().contains("selection_persistence_failed"));
        assert!(!dir.join("sends").exists());
    }

    #[test]
    fn older_packages_and_questions_stay_as_they_were() {
        let (_scratch, store, key, run, page) = gathered_page();
        let settings = store.settings(&key).unwrap();
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        // A question of the step before: a format 1 selection taken as it came, no record, `not_recorded`.
        let targets = vec![target_of(&catalog, catalog.symbols(), &page.gathered[0]).unwrap()];
        let mut older = request(&settings, "O que muda?", Some(targets), vec![], false);
        older.gathered = true;
        let origin = json!({ "kind": "selection", "summary": "x", "selection": { "format_version": 1, "seeds": [page.gathered[0]] } });
        let gathered = json!({ "nodes": [page.gathered[0]], "with_facts": false, "reviewed_facts": [], "origin": origin });
        let turn = store.new_turn(&key, &run, &settings, &catalog, Asked::client("O que muda?"), None, older, Some(("gathered", gathered))).unwrap()["id"].as_str().unwrap().to_owned();
        let (_, _, package) = package_of(&store, &key, &run, &turn);
        assert_eq!((package.selection, package.measure.selection_basis), (None, MeasureSelectionBasis::NotRecorded {}));
        // A plain one, with no mark at all.
        let plain = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap()["id"].as_str().unwrap().to_owned();
        remark(&store, &key, &run, &plain, |q| { q.as_object_mut().unwrap().remove("evidence"); });
        fs::remove_file(store.turn_dir(&key, &run, &plain).unwrap().join(EVIDENCE)).unwrap();
        let before = files_of(&store, &key, &run);
        // New questions, with and without a selection, and their checks, rewrite none of them.
        store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap();
        for id in [&turn, &plain] {
            assert_eq!(refusal_of(&store, &key, &run, id), "", "{id}");
        }
        let after = files_of(&store, &key, &run);
        assert!(before.iter().all(|file| after.contains(file)), "every earlier file is there, byte for byte");
    }

    #[test]
    fn notes_are_refused_wherever_a_client_could_send_them() {
        let (_scratch, store, key, run, page) = gathered_page();
        let gathered = |origin: Value| json!({ "key": key, "run": run, "question": "O que muda?", "nodes": page.gathered, "with_facts": false, "origin": origin });
        let ladder = |origin: Value| json!({ "key": key, "run": run, "question": "O que muda?", "nodes": page.gathered, "with_facts": false, "origin": origin,
            "gate": ["probe"], "models": [{ "provider": "test" }], "retries": 0, "lacuna_rounds": 0 });
        let with = |place: &str, value: Value| {
            let mut origin = page.origin();
            match place {
                "selection" => origin["selection"]["notes"] = value,
                "entry" => origin["selection"]["included"][0]["notes"] = value,
                "reason" => origin["selection"]["included"][0]["reasons"][0]["notes"] = value,
                _ => origin["notes"] = value,
            }
            origin
        };
        let mut cases: Vec<(&str, Value)> = vec![];
        for value in [json!(""), json!("uma nota"), json!([]), json!(["uma"]), json!({}), json!({ "id": "", "text": "x" }), Value::Null] {
            for place in ["selection", "entry", "reason", "origin"] {
                cases.push(("/api/gathered-question", gathered(with(place, value.clone()))));
                cases.push(("/api/ladder", ladder(with(place, value.clone()))));
            }
            let mut top = gathered(page.origin());
            top["notes"] = value.clone();
            cases.push(("/api/gathered-question", top));
            let mut top = ladder(page.origin());
            top["models"][0]["notes"] = value.clone();
            cases.push(("/api/ladder", top));
            cases.push(("/api/path-questions", json!({ "key": key, "run": run, "question": "x", "nodes": [page.gathered[0]], "seed": page.gathered[0], "chosen": { "notes": value } })));
            cases.push(("/api/path-questions", json!({ "key": key, "run": run, "question": "x", "nodes": [page.gathered[0]], "seed": page.gathered[0], "chosen": {}, "notes": value })));
            cases.push(("/api/gathered-question", gathered(json!({ "kind": "hot", "summary": "x", "notes": value }))));
        }
        let before = files_of(&store, &key, &run);
        for (path, body) in &cases {
            let (status, answer) = api(&store, path, body);
            assert_eq!((status, answer["error"].as_str().unwrap_or_default()), (400, "O campo notes não é aceito; nada foi criado nem enviado"), "{path} {body}");
        }
        // No question, no record, no package, no send, no execution.
        assert_eq!(files_of(&store, &key, &run), before);
        assert!(!store.run_dir(&key, &run).unwrap().join("executions").exists());
        // Without notes, the same requests are taken.
        assert_eq!(api(&store, "/api/gathered-question", &gathered(page.origin())).0, 200);
        assert!(refuse_notes(&json!({ "nota": 1, "a": [{ "b": "notes" }] })).is_ok(), "the word as a value is not the field");
        // The check of a selection, which the ladder calls itself, refuses them too, wherever they are.
        let mut origin = page.origin();
        origin["notes"] = json!([]);
        assert_eq!(store.accept_selection(&key, &run, &page.gathered, Some(&origin)).err().map(|e| e.to_string()).as_deref(), Some("O campo notes não é aceito; nada foi criado nem enviado"));
        // And so does a round, which the ladder asks with an origin it already took.
        let before = files_of(&store, &key, &run);
        let round = store.ask_gathered_parts(&key, &run, "x", &[], &page.gathered, false, Some(json!({ "kind": "hot", "notes": "x" })), None, &[]);
        assert_eq!(round.err().map(|e| e.to_string()).as_deref(), Some("O campo notes não é aceito; nada foi criado nem enviado"));
        let path = store.ask_path(&key, &run, "x", std::slice::from_ref(&page.gathered[0]), &page.gathered[0], &json!({ "a": { "notes": [] } }));
        assert_eq!(path.err().map(|e| e.to_string()).as_deref(), Some("O campo notes não é aceito; nada foi criado nem enviado"));
        assert_eq!(files_of(&store, &key, &run), before);
    }

    /// A project with a gate command, approved, and its run.
    fn with_a_gate() -> (Scratch, Store, String, String) {
        let camera = "pub struct Camera;\n\nimpl Camera {\n    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;\n        direction\n    }\n}\n";
        let profile = json!({ "project_root": ".", "commands": { "probe": { "argv": ["git", "status", "--short"], "repeat": 1 } } });
        let (scratch, store, key, _root) = project_with(&[("src/lib.rs", camera)], profile);
        approve(&store, &key, "probe");
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        (scratch, store, key, run)
    }

    #[test]
    fn the_ladder_checks_the_clients_selection_once_before_anything_runs() {
        let (_scratch, store, key, run) = with_a_gate();
        let ray = symbol_id(&store, &key, &run, "ray_from_pixel");
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&ray);
        let answer = page.suggest(&store, &key, &run, std::slice::from_ref(&ray));
        page.join(&answer);
        let body = |selection: Value, nodes: &[String]| json!({ "key": key, "run": run, "question": "Como acelerar?", "nodes": nodes, "with_facts": false,
            "gate": ["probe"], "models": [{ "provider": "test" }], "retries": 0, "lacuna_rounds": 0, "origin": { "kind": "ladder", "summary": "x", "selection": selection } });
        // Refused: no baseline ran, no question was made.
        let mut wrong = page.record();
        wrong["included_bytes"] = json!(wrong["included_bytes"].as_u64().unwrap() + 1);
        let before = files_of(&store, &key, &run);
        let (status, refused) = api(&store, "/api/ladder", &body(wrong, &page.gathered));
        assert_eq!(status, 400, "{refused}");
        assert!(refused["error"].as_str().unwrap().contains("included_bytes não é a soma"), "{refused}");
        assert_eq!(files_of(&store, &key, &run), before);
        assert!(store.executions(&key, &run).unwrap().is_empty(), "no baseline ran");
        // Taken: the baseline, then the question with the client's record.
        let (status, done) = api(&store, "/api/ladder", &body(page.record(), &page.gathered));
        assert_eq!(status, 200, "{done}");
        assert_eq!(done["steps"][0]["outcome"], "baseline");
        let (_, _, package) = package_of(&store, &key, &run, done["turn"].as_str().unwrap());
        assert_eq!(package.origin_kind, Some(OriginKind::Ladder));
        assert!(package.selection.is_some());
        assert!(package.server_additions.is_empty(), "the rounds' additions are the next step's");
    }

    #[test]
    fn the_rounds_of_the_ladder_go_with_the_clients_record_and_not_its_nodes() {
        let (_scratch, store, key, run, page) = gathered_page();
        let mut origin = page.origin();
        origin["kind"] = json!("ladder");
        let accepted = store.accept_selection(&key, &run, &page.gathered, Some(&origin)).unwrap().unwrap();
        // The first question, and a LACUNA answered to it that asks for `Camera`.
        let first = store.ask_gathered_parts(&key, &run, "Como acelerar?", &[], &page.gathered, false, Some(origin.clone()), Some(&accepted), &[]).unwrap()["id"].as_str().unwrap().to_owned();
        record_answer(&store, &key, &run, &first, "manual", &lacuna_answer("Falta ver `Camera` para responder.")).unwrap();
        // Checked once: the suggestion read at the entry may be gone, and the rounds still go on.
        let suggestion = store.run_dir(&key, &run).unwrap().join(SUGGESTIONS).join(format!("{}.json", page.suggestions[0].as_str().unwrap()));
        let saved = fs::read(&suggestion).unwrap();
        fs::remove_file(&suggestion).unwrap();
        // A round with what the server added for that LACUNA, as the ladder adds it.
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        let additions = additions_for(&catalog, &first, "01-manual", 1, &["Camera".into()], &page.gathered);
        let added = additions[0].symbol_id.clone();
        let mut nodes = page.gathered.clone();
        nodes.extend(additions.iter().map(|a| a.symbol_id.clone()));
        let suffix = crate::gate_suffix(&["x".into()], 1);
        // A node beyond the client's that is not recorded as the server's is not taken for the client's.
        let unrecorded = store.ask_gathered_parts(&key, &run, "Como acelerar?", std::slice::from_ref(&suffix), &nodes, false, Some(origin.clone()), Some(&accepted), &[]);
        assert_eq!(unrecorded.err().map(|e| e.to_string()).as_deref(), Some("Complementação recusada (as declarações da cliente não são as da seleção); nenhuma pergunta foi criada"));
        let round = store.ask_gathered_parts(&key, &run, "Como acelerar?", std::slice::from_ref(&suffix), &nodes, false, Some(origin.clone()), Some(&accepted), &additions).unwrap();
        let turn = round["id"].as_str().unwrap();
        fs::write(&suggestion, saved).unwrap();
        // The record is the client's statement as sent: the added node is in no list and has no decision.
        let (question, _, package) = package_of(&store, &key, &run, turn);
        assert_eq!(question["gathered"]["nodes"], json!(nodes));
        let record = SelectionRecord::from_bytes(&fs::read(store.turn_dir(&key, &run, turn).unwrap().join(SELECTION)).unwrap(), package.selection.as_ref().unwrap().record_sha256.as_str()).unwrap();
        // `Camera` is one the client took out: the record says that still, and nothing more.
        assert_eq!(fs::read(store.turn_dir(&key, &run, turn).unwrap().join(SELECTION)).unwrap(), accepted.bytes, "the record kept is the one accepted at the entry");
        for addition in &additions {
            assert!(record.received.included.iter().all(|e| e.id != addition.symbol_id));
            assert!(record.declared_by_client.decisions.iter().all(|d| d.id != addition.symbol_id || d.decision == evidence::Decision::Removed));
        }
        assert_eq!(record.received.included.iter().map(|e| e.id.clone()).collect::<Vec<_>>(), page.gathered);
        // The package: the additions as made, their fichas the server's, the client's the client's.
        assert_eq!(package.server_additions, additions);
        for ficha in package.fichas.iter().filter(|f| f.symbol_id.is_some()) {
            let id = ficha.symbol_id.as_ref().unwrap();
            let expected = if *id == added || additions.iter().any(|a| a.symbol_id == *id) { evidence::DeclaredBy::ServerLacuna } else { evidence::DeclaredBy::Client };
            assert_eq!(ficha.declared_by, expected, "{id}");
        }
        assert!(package.fichas.iter().any(|f| f.declared_by == evidence::DeclaredBy::Client));
        // And it is checked before a send, against the client's nodes, not the round's.
        assert_eq!(refusal_of(&store, &key, &run, turn), "");
        // A round that would carry a selection it was not given, or none for the one it says, is not made.
        assert!(store.ask_gathered_parts(&key, &run, "x", &[], &nodes, false, Some(origin.clone()), None, &additions).unwrap_err().to_string().contains("não foi conferida"));
        assert!(store.ask_gathered_parts(&key, &run, "x", &[], &nodes, false, None, Some(&accepted), &additions).unwrap_err().to_string().contains("não foi conferida"));
    }

    /// A claims answer with one LACUNA and nothing else.
    fn lacuna_answer(text: &str) -> String {
        json!({ "schema_version": 1, "claims": [{ "id": "C1", "kind": "LACUNA", "text": text, "citations": [], "premises": [], "verification_plan": null, "location": null }] }).to_string()
    }

    /// What a lacuna round adds for these names, as the ladder adds it: `ladder::resolve` over what
    /// is gathered, nothing measured, with the round and the send that asked.
    fn additions_for(catalog: &Catalog, turn: &str, send: &str, round: usize, names: &[String], gathered: &[String]) -> Vec<ServerAddition> {
        crate::ladder::resolve(catalog, names, gathered, &[], crate::ladder::lacuna_room(gathered.len())).into_iter().map(|(id, name)| ServerAddition {
            declared_by: evidence::ServerLacunaDeclared::ServerLacuna, round, from_turn: turn.into(), from_send: send.into(), symbol_id: id, for_name: name,
            reason: evidence::AdditionReason::LacunaName, decided_by_rule: evidence::AdditionRule::LadderResolve,
        }).collect()
    }

    /// A project with a gate command and declarations a LACUNA can name, its run, and the page's
    /// envelope over `ray_from_pixel` alone, from one suggestion.
    fn ladder_project() -> (Scratch, Store, String, String, Page) {
        let lib = "pub struct Camera;\n\nimpl Camera {\n    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;\n        direction\n    }\n}\n\npub fn helper() -> f64 {\n    1.0\n}\n\npub fn scale(x: f64) -> f64 {\n    x * 3.0\n}\n";
        let profile = json!({ "project_root": ".", "commands": { "probe": { "argv": ["git", "status", "--short"], "repeat": 1 } } });
        let (scratch, store, key, _root) = project_with(&[("src/lib.rs", lib)], profile);
        approve(&store, &key, "probe");
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let ray = symbol_id(&store, &key, &run, "ray_from_pixel");
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&ray);
        page.suggest(&store, &key, &run, std::slice::from_ref(&ray));
        (scratch, store, key, run, page)
    }

    #[test]
    fn the_ladder_records_what_its_lacuna_rounds_add_and_only_that() {
        let (_scratch, store, key, run, page) = ladder_project();
        let id = |name: &str| symbol_id(&store, &key, &run, name);
        // Each question answered with a LACUNA, as a person pastes one: no model is called.
        let answers = json!({
            "01": lacuna_answer("Faltam `helper` e `scale` para responder."),
            "02": lacuna_answer("Falta `Camera`, e ainda `helper`."),
            "03": lacuna_answer("Falta `helper`."),
        });
        let body = json!({ "key": key, "run": run, "question": "Como acelerar?", "nodes": page.gathered, "with_facts": false, "gate": ["probe"],
            "models": [{ "provider": "test", "answers": answers }], "retries": 0, "lacuna_rounds": 3,
            "origin": { "kind": "ladder", "summary": "x", "selection": page.record() } });
        let (status, done) = api(&store, "/api/ladder", &body);
        assert_eq!(status, 200, "{done}");
        let outcomes: Vec<&str> = done["steps"].as_array().unwrap().iter().map(|s| s["outcome"].as_str().unwrap()).collect();
        assert_eq!(outcomes, ["baseline", "lacuna", "lacuna", "lacuna"], "{done:#}");
        assert_eq!(done["turn"], "03");
        let addition = |round: usize, turn: &str, symbol: String, name: &str| ServerAddition {
            declared_by: evidence::ServerLacunaDeclared::ServerLacuna, round, from_turn: turn.into(), from_send: "01-manual".into(), symbol_id: symbol, for_name: name.into(),
            reason: evidence::AdditionReason::LacunaName, decided_by_rule: evidence::AdditionRule::LadderResolve,
        };
        // Round 1 adds what the LACUNA of 01 asked for, in the order resolved; round 2 what that of 02
        // asked for and was not there yet (`helper` was): `Camera` and its `impl`.
        let first_round = vec![addition(1, "01", id("helper"), "helper"), addition(1, "01", id("scale"), "scale")];
        let impl_camera = store.graph(&key, &run).ok().and_then(|_| {
            let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
            catalog.symbols().values().find(|s| s.qualified_name == "impl Camera").map(|s| s.id.clone())
        }).unwrap();
        let mut both_rounds = first_round.clone();
        both_rounds.extend([addition(2, "02", id("Camera"), "Camera"), addition(2, "02", impl_camera, "Camera")]);
        let selection_of = |turn: &str| package_of(&store, &key, &run, turn).2.selection.unwrap().record_sha256;
        for (turn, expected) in [("01", vec![]), ("02", first_round), ("03", both_rounds)] {
            let (question, bytes, package) = package_of(&store, &key, &run, turn);
            assert_eq!(package.server_additions, expected, "{turn}");
            let nodes: Vec<String> = page.gathered.iter().cloned().chain(expected.iter().map(|a| a.symbol_id.clone())).collect();
            assert_eq!(question["gathered"]["nodes"], json!(nodes), "{turn}");
            assert_eq!(question["gathered"].get("server_additions").is_some(), !expected.is_empty(), "{turn}: a question with none says nothing");
            // The client's declaration is the client's; what the server added is the server's.
            for ficha in package.fichas.iter().filter(|f| f.symbol_id.is_some()) {
                let added = expected.iter().any(|a| Some(&a.symbol_id) == ficha.symbol_id.as_ref());
                assert_eq!(ficha.declared_by, if added { evidence::DeclaredBy::ServerLacuna } else { evidence::DeclaredBy::Client }, "{turn} {}", ficha.reference.0);
            }
            // Every question goes with the record the client's selection was accepted as.
            assert_eq!(selection_of(turn), selection_of("01"));
            assert_eq!(rebuilt(&store, &key, &run, turn, Asked::client("Como acelerar?")), bytes, "{turn}: made again, the same bytes");
            assert_eq!(refusal_of(&store, &key, &run, turn), "", "{turn}");
        }
        assert!(store.turn_dir(&key, &run, "04").is_err(), "a name already there is not added again");
    }

    #[test]
    fn an_addition_the_lacuna_did_not_ask_for_is_never_recorded() {
        let (_scratch, store, key, run, page) = ladder_project();
        let mut origin = page.origin();
        origin["kind"] = json!("ladder");
        let accepted = store.accept_selection(&key, &run, &page.gathered, Some(&origin)).unwrap();
        let first = store.ask_gathered_parts(&key, &run, "x", &[], &page.gathered, false, Some(origin.clone()), accepted.as_ref(), &[]).unwrap()["id"].as_str().unwrap().to_owned();
        record_answer(&store, &key, &run, &first, "manual", &lacuna_answer("Faltam `helper` e `scale`.")).unwrap();
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        let good = additions_for(&catalog, &first, "01-manual", 1, &["helper".into(), "scale".into()], &page.gathered);
        assert_eq!(good.len(), 2);
        let ids = |additions: &[ServerAddition]| page.gathered.iter().cloned().chain(additions.iter().map(|a| a.symbol_id.clone())).collect::<Vec<_>>();
        let refused = |nodes: Vec<String>, additions: Vec<ServerAddition>| -> String {
            let before = files_of(&store, &key, &run);
            let why = store.ask_gathered_parts(&key, &run, "x", &[], &nodes, false, Some(origin.clone()), accepted.as_ref(), &additions).err().map(|e| e.to_string()).unwrap_or_default();
            assert_eq!(files_of(&store, &key, &run), before, "nothing made");
            why
        };
        let changed = |edit: &dyn Fn(&mut Vec<ServerAddition>)| { let mut a = good.clone(); edit(&mut a); a };
        // Out of the order added, twice, or one of the client's: not what a round adds.
        let mut swapped = ids(&good);
        swapped.swap(1, 2);
        assert!(refused(swapped, good.clone()).contains("não são as últimas da pergunta"));
        let twice = changed(&|a| { let x = a[0].clone(); a.push(x); });
        assert!(refused(ids(&twice), twice).contains("já estava na pergunta"));
        let clients = changed(&|a| a[0].symbol_id = page.gathered[0].clone());
        let mut nodes = page.gathered.clone();
        nodes.extend(clients.iter().map(|a| a.symbol_id.clone()));
        assert_eq!(refused(nodes, clients), format!("Complementação recusada ({} já estava na pergunta); nenhuma pergunta foi criada", page.gathered[0]));
        // Not asked for by that LACUNA, not found for the name, not from a send there is, round zero.
        let unasked = changed(&|a| a[1].for_name = "outro".into());
        assert!(refused(ids(&unasked), unasked).contains("não é o que a LACUNA de 01/01-manual pediu"));
        let camera = symbol_id(&store, &key, &run, "Camera");
        let unfound = changed(&|a| a[1].symbol_id = camera.clone());
        assert!(refused(ids(&unfound), unfound).contains("não é o que a LACUNA"));
        let unsent = changed(&|a| for x in a.iter_mut() { x.from_send = "02-manual".into(); });
        assert!(refused(ids(&unsent), unsent).contains("envio 01/02-manual indisponível"));
        let zero = changed(&|a| for x in a.iter_mut() { x.round = 0; });
        assert!(refused(ids(&zero), zero).contains("não é o que a LACUNA"));
        // A node beyond the client's with no record of who added it.
        assert!(refused(ids(&good), vec![]).contains("as declarações da cliente não são as da seleção"));
        // The real ones are taken.
        let round = store.ask_gathered_parts(&key, &run, "x", &[], &ids(&good), false, Some(origin.clone()), accepted.as_ref(), &good).unwrap()["id"].as_str().unwrap().to_owned();
        assert_eq!(package_of(&store, &key, &run, &round).2.server_additions, good);
    }

    #[test]
    fn additions_changed_after_the_question_are_refused_before_a_send() {
        let (_scratch, store, key, run, page) = ladder_project();
        let mut origin = page.origin();
        origin["kind"] = json!("ladder");
        let accepted = store.accept_selection(&key, &run, &page.gathered, Some(&origin)).unwrap();
        let first = store.ask_gathered_parts(&key, &run, "x", &[], &page.gathered, false, Some(origin.clone()), accepted.as_ref(), &[]).unwrap()["id"].as_str().unwrap().to_owned();
        record_answer(&store, &key, &run, &first, "manual", &lacuna_answer("Faltam `helper` e `scale`.")).unwrap();
        let catalog = Catalog::load(&store.edition(&key, &run).unwrap()).unwrap();
        let good = additions_for(&catalog, &first, "01-manual", 1, &["helper".into(), "scale".into()], &page.gathered);
        let nodes: Vec<String> = page.gathered.iter().cloned().chain(good.iter().map(|a| a.symbol_id.clone())).collect();
        let turn = store.ask_gathered_parts(&key, &run, "x", &[], &nodes, false, Some(origin.clone()), accepted.as_ref(), &good).unwrap()["id"].as_str().unwrap().to_owned();
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        let as_value = |a: &[ServerAddition]| serde_json::to_value(a).unwrap();
        // The question saying other additions than its package.
        remark(&store, &key, &run, &turn, |q| q["gathered"]["server_additions"].as_array_mut().unwrap().reverse());
        assert_eq!(refusal_of(&store, &key, &run, &turn), "H1: as complementações não são as da pergunta; nada foi enviado");
        remark(&store, &key, &run, &turn, |q| q["gathered"]["server_additions"] = as_value(&good));
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // Both changed alike, to what the LACUNA did not ask for: its check refuses it.
        let mut other = good.clone();
        other[1].for_name = "outro".into();
        remark(&store, &key, &run, &turn, |q| q["gathered"]["server_additions"] = as_value(&other));
        forge(&store, &key, &run, &turn, |p| p.server_additions = other.clone());
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: complementação: "), "{}", refusal_of(&store, &key, &run, &turn));
        remark(&store, &key, &run, &turn, |q| q["gathered"]["server_additions"] = as_value(&good));
        forge(&store, &key, &run, &turn, |p| p.server_additions = good.clone());
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // An added declaration's ficha said to be the client's.
        forge(&store, &key, &run, &turn, |p| { p.fichas.iter_mut().find(|f| f.declared_by == evidence::DeclaredBy::ServerLacuna).unwrap().declared_by = evidence::DeclaredBy::Client; });
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: Shape { what: \"ficha declared_by\""));
        forge(&store, &key, &run, &turn, |p| for f in p.fichas.iter_mut() { if good.iter().any(|a| Some(&a.symbol_id) == f.symbol_id.as_ref()) { f.declared_by = evidence::DeclaredBy::ServerLacuna; } });
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // Additions taken out of both the question and the package, the nodes kept: the client's
        // part would hold the server's declarations.
        remark(&store, &key, &run, &turn, |q| { q["gathered"].as_object_mut().unwrap().remove("server_additions"); });
        forge(&store, &key, &run, &turn, |p| { p.server_additions.clear(); for f in p.fichas.iter_mut() { f.declared_by = if f.kind == evidence::FichaKind::ReviewedFact { f.declared_by } else { evidence::DeclaredBy::Client }; } });
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: complementação: as declarações da cliente não são as da seleção"), "{}", refusal_of(&store, &key, &run, &turn));
        remark(&store, &key, &run, &turn, |q| q["gathered"]["server_additions"] = as_value(&good));
        forge(&store, &key, &run, &turn, |p| { p.server_additions = good.clone(); for f in p.fichas.iter_mut() { if good.iter().any(|a| Some(&a.symbol_id) == f.symbol_id.as_ref()) { f.declared_by = evidence::DeclaredBy::ServerLacuna; } } });
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // The send whose LACUNA asked for them, gone: what it asked for can no longer be shown.
        let sent = store.turn_dir(&key, &run, &first).unwrap().join("sends/01-manual/send.json");
        let kept = fs::read(&sent).unwrap();
        fs::remove_file(&sent).unwrap();
        assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: complementação: envio 01/01-manual indisponível"));
        fs::write(&sent, kept).unwrap();
        // The package made from the question's record: the additions and the fichas, byte for byte.
        let (_, bytes, _) = package_of(&store, &key, &run, &turn);
        assert_eq!(rebuilt(&store, &key, &run, &turn, Asked::client("x")), bytes);
    }

    #[test]
    fn a_question_without_additions_is_made_as_before() {
        // A question of the ladder with nothing added, and a gathered one with no origin: no record of
        // additions, every ficha the client's, as before this step.
        let (_scratch, store, key, run, page) = ladder_project();
        for origin in [None, Some(page.origin())] {
            let turn = store.ask_gathered(&key, &run, "x", &page.gathered, false, origin).unwrap()["id"].as_str().unwrap().to_owned();
            let (question, _, package) = package_of(&store, &key, &run, &turn);
            assert!(question["gathered"].get("server_additions").is_none());
            assert!(package.server_additions.is_empty());
            assert!(package.fichas.iter().all(|f| f.declared_by == evidence::DeclaredBy::Client), "{:?}", package.fichas);
        }
        // The client cannot send additions: the route takes none, and a body that names them adds none.
        let (status, turn) = api(&store, "/api/gathered-question", &json!({ "key": key, "run": run, "question": "x", "nodes": page.gathered,
            "server_additions": [{ "symbol_id": "x" }] }));
        assert_eq!(status, 200, "{turn}");
        assert!(package_of(&store, &key, &run, turn["id"].as_str().unwrap()).2.server_additions.is_empty());
    }

    // Passo 12: the route matrix of the plan (section 6), one row at a time, through the API.

    /// A FACT about `ray_from_pixel` that passes the mechanical check in the project of `project()`.
    fn fact_answer(id: &str) -> String {
        json!({"schema_version": 1, "claims": [{
            "id": id, "kind": "FACT", "text": "`ray_from_pixel` calcula `direction` na linha 5 de src/lib.rs.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 4, "end_line": 5,
                "quote": "    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;"}],
            "premises": [], "verification_plan": null,
            "location": {"evidence_id": "E1", "path": "src/lib.rs", "symbol_name": "ray_from_pixel",
                "declaration_line": 4, "operation_start_line": 5, "operation_end_line": 5}}]}).to_string()
    }

    /// The turn ids of a run, as the page reads them.
    fn turn_ids(store: &Store, key: &str, run: &str) -> Vec<String> {
        let (status, turns) = api_with(store, "GET", &format!("/api/turns?key={key}&run={run}"), &Value::Null);
        assert_eq!(status, 200, "{turns}");
        turns.as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap().to_owned()).collect()
    }

    /// A question made by a route: its mark names its package, and H1 to H3 pass on it now.
    fn a_valid_package(store: &Store, key: &str, run: &str, turn: &str) {
        let (question, bytes, _) = package_of(store, key, run, turn);
        assert_eq!(question["evidence"], json!({ "format_version": 2, "sha256": hash(&bytes) }), "{turn}");
        assert_eq!(refusal_of(store, key, run, turn), "", "{turn}");
    }

    #[test]
    fn b1_every_route_that_makes_a_question_makes_a_valid_package() {
        // ask, gathered (with and without a selection), path (one question per point) and continue.
        let (_scratch, store, key, run, page) = gathered_page();
        let mut made: Vec<String> = vec![];
        let (status, asked) = api(&store, "/api/ask", &json!({ "key": key, "run": run, "question": "Onde a câmera calcula a direção dos raios?" }));
        assert_eq!(status, 200, "{asked}");
        made.push(asked["id"].as_str().unwrap().into());
        for origin in [Value::Null, page.origin()] {
            let (status, turn) = api(&store, "/api/gathered-question", &json!({ "key": key, "run": run, "question": "O que muda?", "nodes": page.gathered, "origin": origin }));
            assert_eq!(status, 200, "{turn}");
            made.push(turn["id"].as_str().unwrap().into());
        }
        let camera = symbol_id(&store, &key, &run, "Camera");
        let (status, path) = api(&store, "/api/path-questions", &json!({ "key": key, "run": run, "question": "x", "nodes": [page.gathered[0], camera], "seed": page.gathered[0], "chosen": {} }));
        assert_eq!((status, path.as_array().map(Vec::len)), (200, Some(2)), "{path}");
        made.extend(path.as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap().to_owned()));
        let (status, _) = api(&store, "/api/paste", &json!({ "key": key, "run": run, "turn": made[0], "text": fact_answer("C1") }));
        assert_eq!(status, 200);
        let (status, step) = api(&store, "/api/continue", &json!({ "key": key, "run": run, "turn": made[0], "question": "Como evitar a multiplicação?" }));
        assert_eq!(status, 200, "{step}");
        made.push(step["id"].as_str().unwrap().into());
        for turn in &made {
            a_valid_package(&store, &key, &run, turn);
        }
        // One package per question: the two points of the path have two.
        assert_ne!(package_of(&store, &key, &run, &made[3]).1, package_of(&store, &key, &run, &made[4]).1);
        assert_eq!(turn_ids(&store, &key, &run), made);
        // The ladder: every question it makes, the rounds too.
        let (_scratch2, store, key, run, page) = ladder_project();
        let answers = json!({ "01": lacuna_answer("Falta `helper`."), "02": lacuna_answer("Falta `helper`.") });
        let (status, done) = api(&store, "/api/ladder", &json!({ "key": key, "run": run, "question": "Como acelerar?", "nodes": page.gathered, "gate": ["probe"],
            "models": [{ "provider": "test", "answers": answers }], "retries": 0, "lacuna_rounds": 1, "origin": { "kind": "ladder", "summary": "x", "selection": page.record() } }));
        assert_eq!(status, 200, "{done}");
        assert_eq!(turn_ids(&store, &key, &run), ["01", "02"]);
        for turn in ["01", "02"] {
            a_valid_package(&store, &key, &run, turn);
        }
    }

    #[test]
    fn send_paste_review_and_continue_check_the_question_first_and_send_nothing_when_it_fails() {
        let (_scratch, store, key, run, _page) = gathered_page();
        let (_, asked) = api(&store, "/api/ask", &json!({ "key": key, "run": run, "question": "Onde a câmera calcula a direção dos raios?" }));
        let turn = asked["id"].as_str().unwrap().to_owned();
        let sends = |store: &Store| fs::read_dir(store.turn_dir(&key, &run, &turn).unwrap().join("sends")).map(|d| d.count()).unwrap_or(0);
        let evidence = hash(&fs::read(store.turn_dir(&key, &run, &turn).unwrap().join(EVIDENCE)).unwrap());
        // /api/send: the question checked, then sent (to the test provider, which answers nothing)
        // and linked to the package checked.
        let send = json!({ "key": key, "run": run, "turn": turn, "provider": "test" });
        let (status, sent) = api(&store, "/api/send", &send);
        assert_eq!(status, 200, "{sent}");
        assert!(sent["directory"].as_str().unwrap().ends_with("01-test"), "{sent}");
        assert_eq!((&sent["link"]["evidence_sha256"], &sent["purpose"]), (&json!(evidence), &json!("question")));
        assert_eq!(sent["link"]["checks"], json!(["prepare::verify", "claims::verify", "evidence_h1", "evidence_h2", "evidence_h3"]));
        // A provider that does not send is refused before anything, the question untouched.
        let (status, refused) = api(&store, "/api/send", &json!({ "key": key, "run": run, "turn": turn, "provider": "claude-api" }));
        assert_eq!(status, 501, "{refused}");
        // /api/paste: the same check and link.
        let (status, pasted) = api(&store, "/api/paste", &json!({ "key": key, "run": run, "turn": turn, "text": fact_answer("C1") }));
        assert_eq!(status, 200, "{pasted}");
        assert_eq!((&pasted["link"]["evidence_sha256"], &pasted["purpose"]), (&json!(evidence), &json!("question")));
        // /api/review: an approval passes the gates, the package among them.
        let review = |send: &str, decision: &str| api(&store, "/api/review", &json!({ "key": key, "run": run, "turn": turn, "send": send, "claim": "C1", "decision": decision, "note": "A linha 5 calcula direction." }));
        let (status, approved) = review("02-manual", "approved");
        assert_eq!(status, 200, "{approved}");
        assert_eq!(approved["fact_id"], "F1");
        assert_eq!(api(&store, "/api/paste", &json!({ "key": key, "run": run, "turn": turn, "text": fact_answer("C1") })).0, 200);
        let before = sends(&store);
        // The package changed after the question: every route that uses it refuses, and nothing is sent.
        forge(&store, &key, &run, &turn, |p| p.units[0].text_sha256 = Sha256Hex::try_from("0".repeat(64)).unwrap());
        let why = |(status, body): (u16, Value)| { assert_eq!(status, 400, "{body}"); body["error"].as_str().unwrap().to_owned() };
        assert!(why(api(&store, "/api/send", &send)).starts_with("H2: TextDiffers"));
        assert!(why(api(&store, "/api/paste", &json!({ "key": key, "run": run, "turn": turn, "text": fact_answer("C1") }))).starts_with("H2: TextDiffers"));
        assert_eq!(sends(&store), before, "no send was made");
        assert!(why(review("03-manual", "approved")).starts_with("Portão 2: a pergunta não confere mais: H2: TextDiffers"));
        // A rejection is only recorded: it promotes nothing, so it needs no check (Passo 9).
        assert_eq!(review("03-manual", "rejected").0, 200);
        let turns = turn_ids(&store, &key, &run);
        assert!(why(api(&store, "/api/continue", &json!({ "key": key, "run": run, "turn": turn, "question": "E depois?" }))).starts_with("H2: TextDiffers"));
        assert_eq!(turn_ids(&store, &key, &run), turns, "no question was made");
    }

    #[test]
    fn listing_and_deleting_questions_keep_each_package_with_its_question() {
        let (_scratch, store, key, run, page) = ladder_project();
        let answers = json!({ "01": lacuna_answer("Falta `helper`."), "02": lacuna_answer("Falta `helper`.") });
        let (status, _) = api(&store, "/api/ladder", &json!({ "key": key, "run": run, "question": "Como acelerar?", "nodes": page.gathered, "gate": ["probe"],
            "models": [{ "provider": "test", "answers": answers }], "retries": 0, "lacuna_rounds": 1, "origin": { "kind": "ladder", "summary": "x", "selection": page.record() } }));
        assert_eq!(status, 200);
        // Listed as the page reads them; the package is not part of the listing.
        let (_, listed) = api_with(&store, "GET", &format!("/api/turns?key={key}&run={run}"), &Value::Null);
        assert_eq!(listed.as_array().unwrap().len(), 2);
        assert!(listed[0].get("evidence").is_none());
        // Deleted, with its package and its selection's record.
        let dir = store.turn_dir(&key, &run, "02").unwrap();
        assert!(dir.join(EVIDENCE).is_file() && dir.join(SELECTION).is_file());
        let (status, left) = api_with(&store, "DELETE", "/api/turn", &json!({ "key": key, "run": run, "turn": "02" }));
        assert_eq!(status, 200, "{left}");
        assert_eq!(left.as_array().unwrap().iter().map(|t| t["id"].clone()).collect::<Vec<_>>(), [json!("01")]);
        assert!(!dir.exists());
        // A number is never handed out again.
        let (_, next) = api(&store, "/api/gathered-question", &json!({ "key": key, "run": run, "question": "x", "nodes": page.gathered, "origin": page.origin() }));
        assert_eq!(next["id"], "03");
        // The question a lacuna round came from, deleted: what it added can no longer be shown (Passo 11).
        let (_scratch2, store, key, run, page) = ladder_project();
        let (status, _) = api(&store, "/api/ladder", &json!({ "key": key, "run": run, "question": "Como acelerar?", "nodes": page.gathered, "gate": ["probe"],
            "models": [{ "provider": "test", "answers": answers }], "retries": 0, "lacuna_rounds": 1, "origin": { "kind": "ladder", "summary": "x", "selection": page.record() } }));
        assert_eq!(status, 200);
        assert_eq!(refusal_of(&store, &key, &run, "02"), "");
        assert_eq!(api_with(&store, "DELETE", "/api/turn", &json!({ "key": key, "run": run, "turn": "01" })).0, 200);
        assert!(refusal_of(&store, &key, &run, "02").starts_with("H1: complementação: Pergunta desconhecida: 01"), "{}", refusal_of(&store, &key, &run, "02"));
    }

    #[test]
    fn routes_that_make_no_question_make_no_package() {
        // The suggestion keeps its record and makes no question.
        let (_scratch, store, key, run, page) = gathered_page();
        let turns = |store: &Store, key: &str, run: &str| store.run_dir(key, run).unwrap().join("turns").read_dir().map(|d| d.count()).unwrap_or(0);
        let before = turns(&store, &key, &run);
        let (status, suggested) = api(&store, "/api/envelope/suggest", &json!({ "key": key, "run": run, "nodes": [page.gathered[0]] }));
        assert_eq!(status, 200, "{suggested}");
        assert!(store.verify_suggestion(&key, &run, suggested["suggestion"].as_str().unwrap()).is_ok());
        // The performance question proposes, and makes no question.
        let (status, _) = api(&store, "/api/performance-question", &json!({ "key": key, "run": run, "question": "Quais funções pesam?" }));
        assert_eq!(status, 200);
        // The lexicon is not a question: its request is written apart, with no package (excluded by the plan).
        let (status, lexicon) = api(&store, "/api/lexicon/suggest", &json!({ "key": key, "run": run, "provider": "manual", "language": crate::lexicon::LANGUAGES[0] }));
        assert_eq!(status, 200, "{lexicon}");
        assert_eq!(turns(&store, &key, &run), before);
        let lexicon_dir = store.project_dir(&key).unwrap().join("lexicon");
        let written: Vec<PathBuf> = fs::read_dir(&lexicon_dir).unwrap().flatten().flat_map(|d| fs::read_dir(d.path()).unwrap().flatten().map(|f| f.path())).collect();
        assert!(!written.is_empty() && written.iter().all(|f| !f.ends_with(EVIDENCE) && !f.ends_with(SELECTION)), "{written:?}");
        // An investigation from the inventory proposes evidence and makes no question.
        let scratch = project();
        let root = scratch.0.join("project");
        fs::write(root.join("src/scan.rs"), "fn norm(v: f64) -> f64 {\n    v.sqrt()\n}\n\npub fn scan(xs: &[f64]) -> f64 {\n    let mut t = 0.0;\n    for x in xs {\n        t += norm(*x);\n    }\n    t\n}\n").unwrap();
        let store = Store { dir: scratch.0.join("studio"), modelfiles: scratch.0.join("modelfiles") };
        let key = store.open(&display(&root)).unwrap()["key"].as_str().unwrap().to_owned();
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let found = store.inventory(&key, &run).unwrap();
        let target = found["inventory"]["loops"].as_array().unwrap().iter().find(|l| l["symbol_name"] == "scan").unwrap()["occurrence"].clone();
        let (status, proposed) = api(&store, "/api/inventory/investigate", &json!({ "key": key, "run": run, "target": target }));
        assert_eq!(status, 200, "{proposed}");
        assert_eq!(proposed["items"][0]["name"], "scan");
        assert_eq!(turns(&store, &key, &run), 0);
    }

    // Passo 13: H4, the suggestion made again from what its record names.

    /// A suggestion record kept under its own hash in the run, as the route keeps one, and its hash.
    fn keep_record(store: &Store, key: &str, run: &str, record: &SuggestionRecord) -> Sha256Hex {
        let bytes = record.to_bytes().unwrap();
        let sha = hash(&bytes);
        fs::write(store.run_dir(key, run).unwrap().join(SUGGESTIONS).join(format!("{sha}.json")), &bytes).unwrap();
        Sha256Hex::try_from(sha).unwrap()
    }

    /// What a route answers for a record: the answer the page absorbs.
    fn answer_of(record: &SuggestionRecord, sha: &Sha256Hex) -> Value {
        let mut answer = serde_json::to_value(&record.output).unwrap();
        answer["budget_bytes"] = json!(record.input.budget_bytes);
        answer["suggestion"] = json!(sha.as_str());
        answer
    }

    fn record_of(store: &Store, key: &str, run: &str, sha: &str) -> SuggestionRecord {
        SuggestionRecord::from_bytes(&fs::read(store.run_dir(key, run).unwrap().join(SUGGESTIONS).join(format!("{sha}.json"))).unwrap(), sha).unwrap()
    }

    fn reasons(found: &evidence::Reproduction) -> Vec<evidence::NotReplayable> {
        match found {
            evidence::Reproduction::NotReplayable { suggestions, .. } => suggestions.iter().filter_map(|s| match s {
                evidence::SuggestionReplay::NotReplayable { reason, .. } => Some(*reason),
                evidence::SuggestionReplay::Reproduced { .. } => None,
            }).collect(),
            other => panic!("not replayable expected: {other:?}"),
        }
    }

    #[test]
    fn h4_reproduces_the_suggestions_of_a_selection_and_says_the_facts_are_the_records() {
        let (_scratch, store, key, run, page) = gathered_page();
        let turn = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        let sha = Sha256Hex::try_from(page.suggestions[0].as_str().unwrap().to_owned()).unwrap();
        let reproduced = evidence::Reproduction::Reproduced { suggestions: vec![evidence::SuggestionReplay::Reproduced { suggestion: sha.clone() }], facts: evidence::FactsInReplay::RecordedNotReproduced };
        assert_eq!(store.reproduce(&key, &run, &turn).unwrap(), reproduced);
        assert_eq!(serde_json::to_value(&reproduced).unwrap(), json!({ "status": "reproduced", "suggestions": [{ "status": "reproduced", "suggestion": sha.as_str() }], "facts": "recorded_not_reproduced" }));
        // It writes nothing: no suggestion, no question, no record, nothing in the acervo.
        let acervo = store.project_dir(&key).unwrap().join("facts.jsonl");
        let before = (files_of(&store, &key, &run), fs::read(&acervo).ok());
        store.reproduce(&key, &run, &turn).unwrap();
        assert_eq!((files_of(&store, &key, &run), fs::read(&acervo).ok()), before);
        // Only the facts differ: still reproduced, the facts being the record's and said not made again.
        let mut other_facts = record_of(&store, &key, &run, sha.as_str());
        for item in other_facts.output.items.iter_mut() {
            item["facts"] = json!(["F9"]);
        }
        assert!(evidence::same_but_facts(&record_of(&store, &key, &run, sha.as_str()), &other_facts));
        let mut with_more = other_facts.clone();
        with_more.output.used_bytes += 1;
        assert!(!evidence::same_but_facts(&other_facts, &with_more), "everything else is compared");
        let mut renamed = other_facts.clone();
        renamed.output.items[0]["role"] = json!("support");
        assert!(!evidence::same_but_facts(&other_facts, &renamed));
    }

    #[test]
    fn h4_is_not_applicable_where_no_suggestion_is_behind_the_question() {
        let (_scratch, store, key, run, page) = gathered_page();
        let asked = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap()["id"].as_str().unwrap().to_owned();
        let plain = store.ask_gathered(&key, &run, "x", &page.gathered, false, None).unwrap()["id"].as_str().unwrap().to_owned();
        let hot = store.ask_gathered(&key, &run, "x", &page.gathered, false, Some(json!({ "kind": "hot", "summary": "x" }))).unwrap()["id"].as_str().unwrap().to_owned();
        // An older question, with no mark and no package.
        let older = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap()["id"].as_str().unwrap().to_owned();
        remark(&store, &key, &run, &older, |q| { q.as_object_mut().unwrap().remove("evidence"); });
        fs::remove_file(store.turn_dir(&key, &run, &older).unwrap().join(EVIDENCE)).unwrap();
        for turn in [&asked, &plain, &hot, &older] {
            assert_eq!(store.reproduce(&key, &run, turn).unwrap(), evidence::Reproduction::NotApplicable {}, "{turn}");
        }
        assert_eq!(serde_json::to_value(evidence::Reproduction::NotApplicable {}).unwrap(), json!({ "status": "not_applicable" }));
    }

    #[test]
    fn h4_tells_each_reason_a_suggestion_cannot_be_made_again_and_never_simulates_one() {
        // A suggestion made with a measure: it names the execution it read.
        let (_scratch, store, key, run) = measured_mix();
        let node = symbol_id(&store, &key, &run, "roda");
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&node);
        let answer = page.suggest(&store, &key, &run, std::slice::from_ref(&node));
        let sha = Sha256Hex::try_from(answer["suggestion"].as_str().unwrap().to_owned()).unwrap();
        let record = record_of(&store, &key, &run, sha.as_str());
        let SuggestionMeasure::Ok { execution, .. } = record.measure.clone() else { panic!("{:?}", record.measure) };
        let turn = store.ask_gathered(&key, &run, "Por que roda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        assert!(matches!(store.reproduce(&key, &run, &turn).unwrap(), evidence::Reproduction::Reproduced { .. }));
        let (_, _, package) = package_of(&store, &key, &run, &turn);
        let cite = |sha: &Sha256Hex| forge(&store, &key, &run, &turn, |p| p.selection.as_mut().unwrap().suggestions = vec![sha.clone()]);
        let reason = || reasons(&store.reproduce(&key, &run, &turn).unwrap());
        // Rules of another version, of the selection or of the inventory: told, never run.
        type Change = Box<dyn Fn(&mut SuggestionRecord)>;
        let changes: Vec<Change> = vec![
            Box::new(|r| r.rules_version = RulesVersion("envelope-0".into())),
            Box::new(|r| r.inventory_rules = SuggestionInventoryRules::new("operation-inventory-v0", r.inventory_rules.rules.clone()).unwrap()),
            Box::new(|r| {
                let mut rules = r.inventory_rules.rules.clone();
                let builtin = rules.iter().position(|e| e.origin == InventoryRuleOrigin::Builtin).unwrap();
                rules.remove(builtin);
                r.inventory_rules = SuggestionInventoryRules::new(inventory::VERSION, rules).unwrap();
            }),
        ];
        for change in &changes {
            let mut other = record.clone();
            change(&mut other);
            cite(&keep_record(&store, &key, &run, &other));
            assert_eq!(reason(), [evidence::NotReplayable::RulesVersionDiffers]);
        }
        // Same rules, another output: told apart from a version that differs.
        let mut other = record.clone();
        other.output.used_bytes += 1;
        cite(&keep_record(&store, &key, &run, &other));
        assert_eq!(reason(), [evidence::NotReplayable::OutputDiffers]);
        // The record itself missing, or not the bytes its name says.
        cite(&Sha256Hex::try_from("0".repeat(64)).unwrap());
        assert_eq!(reason(), [evidence::NotReplayable::SuggestionMissing]);
        let file = store.run_dir(&key, &run).unwrap().join(SUGGESTIONS).join(format!("{}.json", sha.as_str()));
        let kept = fs::read(&file).unwrap();
        cite(&sha);
        fs::write(&file, b"{}").unwrap();
        assert_eq!(reason(), [evidence::NotReplayable::SuggestionMissing]);
        fs::write(&file, &kept).unwrap();
        // The execution it read, changed or gone: no replay with another measure, or with none.
        let executed = store.run_dir(&key, &run).unwrap().join("executions").join(&execution).join("execution.json");
        let original = fs::read(&executed).unwrap();
        fs::write(&executed, [original.as_slice(), b" "].concat()).unwrap();
        assert_eq!(reason(), [evidence::NotReplayable::ExecutionDiffers]);
        fs::remove_file(&executed).unwrap();
        assert_eq!(reason(), [evidence::NotReplayable::ExecutionMissing]);
        fs::write(&executed, &original).unwrap();
        assert!(matches!(store.reproduce(&key, &run, &turn).unwrap(), evidence::Reproduction::Reproduced { .. }));
        // Several cited, each told in its order; the code the question was made with is named.
        cite(&sha);
        forge(&store, &key, &run, &turn, |p| p.selection.as_mut().unwrap().suggestions = vec![sha.clone(), Sha256Hex::try_from("0".repeat(64)).unwrap()]);
        let found = store.reproduce(&key, &run, &turn).unwrap();
        let builder = package.versions.builder.clone();
        assert_eq!(found, evidence::Reproduction::NotReplayable {
            suggestions: vec![evidence::SuggestionReplay::Reproduced { suggestion: sha.clone() },
                evidence::SuggestionReplay::NotReplayable { suggestion: Sha256Hex::try_from("0".repeat(64)).unwrap(), reason: evidence::NotReplayable::SuggestionMissing }],
            facts: evidence::FactsInReplay::RecordedNotReproduced, builder_at_question: builder.clone(),
            code_recoverable: matches!(builder, evidence::BuilderIdentity::Recorded { git_dirty: false, .. }) });
        let value = serde_json::to_value(&found).unwrap();
        assert_eq!(value["suggestions"][1], json!({ "status": "not_replayable", "suggestion": "0".repeat(64), "reason": "suggestion_missing" }));
        for (reason, name) in [(evidence::NotReplayable::SuggestionMissing, "suggestion_missing"), (evidence::NotReplayable::ExecutionMissing, "execution_missing"),
            (evidence::NotReplayable::ExecutionDiffers, "execution_differs"), (evidence::NotReplayable::RulesVersionDiffers, "rules_version_differs"),
            (evidence::NotReplayable::OutputDiffers, "output_differs")] {
            assert_eq!(serde_json::to_value(reason).unwrap(), json!(name));
        }
    }

    #[test]
    fn h4_makes_a_suggestion_again_with_the_rules_its_record_copied_not_the_profiles_now() {
        // A project rule the suggestion used: the behavior it found is the rule's.
        let rule = json!({ "id": "minha", "operation": "database", "match": "call", "names": ["consulta"] });
        let (_scratch, store, key, root) = project_with(&[("src/lib.rs", "pub fn demo() -> u32 {\n    consulta()\n}\nfn consulta() -> u32 {\n    1\n}\n")], json!({ "inventory": { "rules": [rule] } }));
        let run = store.generate(&key).unwrap()["id"].as_str().unwrap().to_owned();
        let node = symbol_id(&store, &key, &run, "demo");
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&node);
        let answer = page.suggest(&store, &key, &run, std::slice::from_ref(&node));
        assert!(answer["behavior"].as_array().unwrap().iter().any(|b| b["detail"] == "database"), "{answer:#}");
        let turn = store.ask_gathered(&key, &run, "x", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        // The profile no longer has the rule: the record's copy is what is made again.
        fs::write(root.join(PROFILE), json!({ "inventory": { "rules": [] } }).to_string()).unwrap();
        assert!(store.suggest_envelope(&key, &run, std::slice::from_ref(&node)).unwrap()["behavior"].as_array().unwrap().iter().all(|b| b["detail"] != "database"));
        assert!(matches!(store.reproduce(&key, &run, &turn).unwrap(), evidence::Reproduction::Reproduced { .. }));
    }

    #[test]
    fn h4_is_told_and_never_stops_a_question_that_passes_h1_to_h3() {
        // A record of the same rules whose output this code does not make: a selection over it is valid
        // against it (H1 to H3 pass), and H4 says the suggestion is not made again.
        let (_scratch, store, key, run, _) = gathered_page();
        let ray = symbol_id(&store, &key, &run, "ray_from_pixel");
        let real = store.suggest_envelope(&key, &run, std::slice::from_ref(&ray)).unwrap();
        let mut other = record_of(&store, &key, &run, real["suggestion"].as_str().unwrap());
        other.output.used_bytes += 7;
        let sha = keep_record(&store, &key, &run, &other);
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&ray);
        page.absorb(&answer_of(&other, &sha), std::slice::from_ref(&ray));
        let turn = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        assert_eq!(reasons(&store.reproduce(&key, &run, &turn).unwrap()), [evidence::NotReplayable::OutputDiffers]);
        // The send goes on H1 to H3: H4 is not one of its checks.
        let verified = store.verify_for_send(&key, &run, &turn).unwrap();
        assert_eq!(verified.link()["checks"], json!(["prepare::verify", "claims::verify", "evidence_h1", "evidence_h2", "evidence_h3"]));
        assert!(record_answer(&store, &key, &run, &turn, "manual", &fact_answer("C1")).is_ok());
    }

    // Passo 14: GET /api/evidence, what was recorded with a question and what is checked now.

    /// The route's answer for a question, and every file the studio keeps, before and after: asking
    /// it writes nothing anywhere (package, selection, question, suggestions, sends, acervo, executions).
    fn evidence_of(store: &Store, key: &str, run: &str, turn: &str) -> (u16, Value) {
        fn every_file(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() { every_file(&path, out) } else { out.push((path.clone(), fs::read(&path).unwrap())) }
            }
        }
        let mut before = vec![];
        every_file(&store.dir, &mut before);
        let answer = api_with(store, "GET", &format!("/api/evidence?key={key}&run={run}&turn={turn}"), &Value::Null);
        let mut after = vec![];
        every_file(&store.dir, &mut after);
        before.sort();
        after.sort();
        assert_eq!(after, before, "asking for the evidence of {turn} wrote nothing");
        answer
    }

    fn recorded(store: &Store, key: &str, run: &str, turn: &str, name: &str) -> Value {
        serde_json::from_slice(&fs::read(store.turn_dir(key, run, turn).unwrap().join(name)).unwrap()).unwrap()
    }

    fn statuses(answer: &Value) -> [String; 3] {
        ["h1", "h2", "h3"].map(|h| answer["checks"][h]["status"].as_str().unwrap().to_owned())
    }

    fn h4_reasons(answer: &Value) -> Vec<String> {
        answer["h4"]["suggestions"].as_array().unwrap().iter().filter_map(|s| s["reason"].as_str().map(str::to_owned)).collect()
    }

    #[test]
    fn evidence_shows_what_was_recorded_with_each_question_and_what_is_checked_now() {
        let (_scratch, store, key, run, page) = gathered_page();
        let with = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        let asked = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap()["id"].as_str().unwrap().to_owned();
        // A question with a selection: as recorded, all three checks passed now, its suggestion made again.
        let (status, answer) = evidence_of(&store, &key, &run, &with);
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer["turn"], json!(with));
        assert_eq!(answer["package"], recorded(&store, &key, &run, &with, EVIDENCE));
        assert_eq!(answer["selection"], recorded(&store, &key, &run, &with, SELECTION));
        assert_eq!(answer["mark"], recorded(&store, &key, &run, &with, "question.json")["evidence"]);
        assert_eq!(statuses(&answer), ["passed", "passed", "passed"]);
        assert_eq!(answer["checks"]["h1"]["why"], Value::Null);
        assert_eq!(answer["h4"], json!({ "status": "reproduced", "facts": "recorded_not_reproduced",
            "suggestions": [{ "status": "reproduced", "suggestion": page.suggestions[0] }] }));
        assert_eq!(answer["latest_measure"], json!({ "read_status": "none" }));
        // A format 2 package: the two readings compared (no measure on either side), no format 1 field,
        // and so no warning about one.
        assert_eq!(answer["package"]["format_version"], json!(2));
        assert_eq!(answer["package"]["measure"]["measure_divergence"], json!({ "status": "same" }));
        assert_eq!(answer["package"]["measure"].get("measure_diverges"), None);
        assert_eq!(answer["warnings"], json!([]));
        // The same answer when asked again.
        assert_eq!(evidence_of(&store, &key, &run, &with).1, answer);
        // Another question of the same run: its own package, no selection, H4 not applicable.
        let (_, other) = evidence_of(&store, &key, &run, &asked);
        assert_eq!(other["package"], recorded(&store, &key, &run, &asked, EVIDENCE));
        assert_ne!(other["package"], answer["package"]);
        assert_eq!((&other["selection"], &other["h4"]), (&Value::Null, &json!({ "status": "not_applicable" })));
        assert_eq!(statuses(&other), ["passed", "passed", "passed"]);
        // A question, a run or a project that is not there: the route's usual refusal, nothing else.
        let (status, missing) = evidence_of(&store, &key, &run, "99");
        assert_eq!((status, missing["error"].as_str()), (400, Some("Pergunta desconhecida: 99")));
        assert_eq!(api_with(&store, "GET", &format!("/api/evidence?key={key}&run=nada&turn={with}"), &Value::Null).0, 400);
        assert_eq!(api_with(&store, "GET", &format!("/api/evidence?key={key}&run={run}"), &Value::Null).0, 400);
    }

    #[test]
    fn evidence_tells_an_older_question_an_unreadable_file_and_a_changed_package_apart() {
        let (_scratch, store, key, run, page) = gathered_page();
        // An older question, with no mark and no package: nothing to check, which is not a check passed.
        let older = store.ask(&key, &run, "Onde a câmera calcula a direção dos raios?").unwrap()["id"].as_str().unwrap().to_owned();
        remark(&store, &key, &run, &older, |q| { q.as_object_mut().unwrap().remove("evidence"); });
        fs::remove_file(store.turn_dir(&key, &run, &older).unwrap().join(EVIDENCE)).unwrap();
        let (_, answer) = evidence_of(&store, &key, &run, &older);
        assert_eq!((&answer["mark"], &answer["package"], &answer["h4"]), (&Value::Null, &Value::Null, &json!({ "status": "not_applicable" })));
        assert_eq!(statuses(&answer), ["not_applicable", "not_applicable", "not_applicable"]);
        assert_eq!(answer["warnings"], json!([]), "no package, nothing it could seem to say");
        // A package changed after the question: H1 passes, H2 refuses, H3 is not run; H4 is still told.
        let changed = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        forge(&store, &key, &run, &changed, |p| p.units[0].text_sha256 = Sha256Hex::try_from("0".repeat(64)).unwrap());
        let (_, answer) = evidence_of(&store, &key, &run, &changed);
        assert_eq!(statuses(&answer), ["passed", "refused", "not_run"]);
        assert!(answer["checks"]["h2"]["why"].as_str().unwrap().starts_with("H2: TextDiffers"));
        assert_eq!(answer["h4"]["status"], "reproduced", "H4 says nothing of the package's integrity");
        // A package that is not one: null as recorded, refused by H1, and no H4 rather than "not applicable".
        let unreadable = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        fs::write(store.turn_dir(&key, &run, &unreadable).unwrap().join(EVIDENCE), b"isto nao e um pacote").unwrap();
        let (_, answer) = evidence_of(&store, &key, &run, &unreadable);
        assert_eq!((&answer["package"], &answer["h4"]), (&Value::Null, &Value::Null));
        assert_eq!(statuses(&answer), ["refused", "not_run", "not_run"]);
        assert!(answer["checks"]["h1"]["why"].as_str().unwrap().starts_with("H1: o pacote não é o que a marca nomeia"));
        // A selection's record that is not one: null as recorded, refused by H1; the suggestions are still made again.
        let record = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        fs::write(store.turn_dir(&key, &run, &record).unwrap().join(SELECTION), b"{").unwrap();
        let (_, answer) = evidence_of(&store, &key, &run, &record);
        assert_eq!(answer["selection"], Value::Null);
        assert_eq!(statuses(&answer), ["refused", "not_run", "not_run"]);
        assert!(answer["checks"]["h1"]["why"].as_str().unwrap().starts_with("H1: registro de seleção: o registro"));
        assert_eq!(answer["h4"]["status"], "reproduced");
        // A question whose package could not be made: its mark says so, and H1 refuses with it.
        FAIL_EVIDENCE_AT.with(|f| f.set(Some("composition")));
        let failed = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, None).unwrap_err().to_string();
        FAIL_EVIDENCE_AT.with(|f| f.set(None));
        let failed = failed.split_whitespace().nth(2).unwrap().to_owned();
        let (_, answer) = evidence_of(&store, &key, &run, &failed);
        assert_eq!((&answer["mark"], &answer["package"]), (&json!({ "error": "composition_failed" }), &Value::Null));
        assert_eq!(statuses(&answer), ["refused", "not_run", "not_run"]);
        assert!(answer["checks"]["h1"]["why"].as_str().unwrap().contains("composition_failed"));
    }

    #[test]
    fn evidence_tells_each_h4_reason_and_never_as_a_failure_of_the_package() {
        // A selection over a suggestion made with a measure.
        let (_scratch, store, key, run) = measured_mix();
        let node = symbol_id(&store, &key, &run, "roda");
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&node);
        let answer = page.suggest(&store, &key, &run, std::slice::from_ref(&node));
        let sha = Sha256Hex::try_from(answer["suggestion"].as_str().unwrap().to_owned()).unwrap();
        let record = record_of(&store, &key, &run, sha.as_str());
        let SuggestionMeasure::Ok { execution, execution_sha256 } = record.measure.clone() else { panic!("{:?}", record.measure) };
        let turn = store.ask_gathered(&key, &run, "Por que roda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        let (_, now) = evidence_of(&store, &key, &run, &turn);
        assert_eq!((statuses(&now), now["h4"]["status"].as_str()), (["passed", "passed", "passed"].map(String::from), Some("reproduced")));
        let latest = store.latest_measure_read(&key, &run).unwrap().unwrap();
        assert_eq!(now["latest_measure"], json!({ "read_status": "ok", "execution": latest.record["id"], "execution_sha256": latest.sha256 }));
        assert_eq!((latest.record["id"].as_str(), latest.sha256.as_str()), (Some(execution.as_str()), execution_sha256.as_str()));
        // The execution read then, changed or gone: H4 says which, and H3, which reads it too, refuses on its own.
        let executed = store.run_dir(&key, &run).unwrap().join("executions").join(&execution).join("execution.json");
        let original = fs::read(&executed).unwrap();
        fs::write(&executed, [original.as_slice(), b" "].concat()).unwrap();
        let (_, answer) = evidence_of(&store, &key, &run, &turn);
        assert_eq!(h4_reasons(&answer), ["execution_differs"]);
        assert_eq!(statuses(&answer), ["passed", "passed", "refused"]);
        fs::remove_file(&executed).unwrap();
        assert_eq!(h4_reasons(&evidence_of(&store, &key, &run, &turn).1), ["execution_missing"]);
        // The latest measure that cannot be read is told as such, not as no measure at all.
        fs::write(&executed, b"{ metade").unwrap();
        assert_eq!(evidence_of(&store, &key, &run, &turn).1["latest_measure"], json!({ "read_status": "unreadable" }));
        fs::write(&executed, &original).unwrap();
        // The suggestion's record gone.
        let file = store.run_dir(&key, &run).unwrap().join(SUGGESTIONS).join(format!("{}.json", sha.as_str()));
        let kept = fs::read(&file).unwrap();
        fs::remove_file(&file).unwrap();
        let (_, answer) = evidence_of(&store, &key, &run, &turn);
        assert_eq!(h4_reasons(&answer), ["suggestion_missing"]);
        assert_eq!(answer["h4"]["status"], "not_replayable");
        fs::write(&file, &kept).unwrap();
        // Rules of another version, cited by a package changed to cite them: H1 refuses the change, H4 the rules.
        let mut other = record.clone();
        other.rules_version = RulesVersion("envelope-0".into());
        let other_sha = keep_record(&store, &key, &run, &other);
        forge(&store, &key, &run, &turn, |p| p.selection.as_mut().unwrap().suggestions = vec![other_sha.clone()]);
        let (_, answer) = evidence_of(&store, &key, &run, &turn);
        assert_eq!(h4_reasons(&answer), ["rules_version_differs"]);
        assert_eq!(statuses(&answer), ["refused", "not_run", "not_run"]);
        let builder = package_of(&store, &key, &run, &turn).2.versions.builder;
        assert_eq!(answer["h4"]["builder_at_question"], serde_json::to_value(&builder).unwrap());
    }

    #[test]
    fn evidence_of_a_suggestion_this_code_does_not_make_is_told_and_the_send_still_passes() {
        let (_scratch, store, key, run, _) = gathered_page();
        let ray = symbol_id(&store, &key, &run, "ray_from_pixel");
        let real = store.suggest_envelope(&key, &run, std::slice::from_ref(&ray)).unwrap();
        let mut other = record_of(&store, &key, &run, real["suggestion"].as_str().unwrap());
        other.output.used_bytes += 7;
        let sha = keep_record(&store, &key, &run, &other);
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&ray);
        page.absorb(&answer_of(&other, &sha), std::slice::from_ref(&ray));
        let turn = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        let (_, answer) = evidence_of(&store, &key, &run, &turn);
        assert_eq!(statuses(&answer), ["passed", "passed", "passed"]);
        assert_eq!(h4_reasons(&answer), ["output_differs"]);
        assert!(store.verify_for_send(&key, &run, &turn).is_ok(), "H4 is not a check of the send");
    }

    #[test]
    fn evidence_of_several_suggestions_and_of_a_ladder_round_keeps_each_part_apart() {
        // Two suggestions cited: each made again, in the order cited.
        let (_scratch, store, key, run, _) = gathered_page();
        let (ray, camera) = (symbol_id(&store, &key, &run, "ray_from_pixel"), symbol_id(&store, &key, &run, "Camera"));
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&ray);
        let first = page.suggest(&store, &key, &run, std::slice::from_ref(&ray));
        page.join(&first);
        page.toggle_target(&camera);
        page.suggest(&store, &key, &run, &[ray.clone(), camera.clone()]);
        assert_eq!(page.suggestions.len(), 2);
        assert_ne!(page.suggestions[0], page.suggestions[1]);
        let turn = store.ask_gathered(&key, &run, "O que muda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        let (_, answer) = evidence_of(&store, &key, &run, &turn);
        assert_eq!(answer["h4"]["suggestions"], json!(page.suggestions.iter().map(|s| json!({ "status": "reproduced", "suggestion": s })).collect::<Vec<_>>()));
        assert_eq!(statuses(&answer), ["passed", "passed", "passed"]);
        // A ladder round: the client's selection, the server's additions, each where it belongs.
        let (_scratch2, store, key, run, page) = ladder_project();
        let answers = json!({ "01": lacuna_answer("Falta `helper`."), "02": lacuna_answer("Falta `helper`.") });
        let (status, _) = api(&store, "/api/ladder", &json!({ "key": key, "run": run, "question": "Como acelerar?", "nodes": page.gathered, "gate": ["probe"],
            "models": [{ "provider": "test", "answers": answers }], "retries": 0, "lacuna_rounds": 1, "origin": { "kind": "ladder", "summary": "x", "selection": page.record() } }));
        assert_eq!(status, 200);
        let (_, round) = evidence_of(&store, &key, &run, "02");
        let helper = symbol_id(&store, &key, &run, "helper");
        assert_eq!(round["package"]["server_additions"].as_array().unwrap().iter().map(|a| a["symbol_id"].clone()).collect::<Vec<_>>(), [json!(helper)]);
        assert_eq!(round["selection"], recorded(&store, &key, &run, "01", SELECTION), "the client's record, the same in every round");
        assert!(round["selection"]["received"]["included"].as_array().unwrap().iter().all(|e| e["id"] != json!(helper)));
        assert_eq!(statuses(&round), ["passed", "passed", "passed"]);
        assert_eq!(round["h4"]["status"], "reproduced");
    }

    #[test]
    fn routes_without_a_selection_make_none() {
        let (_scratch, store, key, run, page) = gathered_page();
        let none = |turn: &str| {
            let (_, _, package) = package_of(&store, &key, &run, turn);
            assert_eq!((package.selection, package.measure.selection_basis), (None, MeasureSelectionBasis::NotApplicable {}), "{turn}");
            assert!(!store.turn_dir(&key, &run, turn).unwrap().join(SELECTION).exists(), "{turn}");
        };
        let (status, asked) = api(&store, "/api/ask", &json!({ "key": key, "run": run, "question": "Onde a câmera calcula a direção dos raios?" }));
        assert_eq!(status, 200, "{asked}");
        none(asked["id"].as_str().unwrap());
        let (status, path) = api(&store, "/api/path-questions", &json!({ "key": key, "run": run, "question": "x", "nodes": [page.gathered[0]], "seed": page.gathered[0], "chosen": {} }));
        assert_eq!(status, 200, "{path}");
        none(path[0]["id"].as_str().unwrap());
        for origin in [Value::Null, json!({ "kind": "hot", "summary": "y" })] {
            let (status, turn) = api(&store, "/api/gathered-question", &json!({ "key": key, "run": run, "question": "x", "nodes": [page.gathered[0]], "origin": origin }));
            assert_eq!(status, 200, "{turn}");
            none(turn["id"].as_str().unwrap());
        }
        // A chain step from an answer that passed: its own package, with no selection.
        let answer = json!({"schema_version": 1, "claims": [{
            "id": "C1", "kind": "FACT", "text": "`ray_from_pixel` calcula `direction` na linha 5 de src/lib.rs.",
            "citations": [{"evidence_id": "E1", "path": "src/lib.rs", "start_line": 4, "end_line": 5,
                "quote": "    pub fn ray_from_pixel(&self, x: f64) -> f64 {\n        let direction = x * 2.0;"}],
            "premises": [], "verification_plan": null,
            "location": {"evidence_id": "E1", "path": "src/lib.rs", "symbol_name": "ray_from_pixel",
                "declaration_line": 4, "operation_start_line": 5, "operation_end_line": 5}}]});
        record_answer(&store, &key, &run, asked["id"].as_str().unwrap(), "manual", &answer.to_string()).unwrap();
        let (status, step) = api(&store, "/api/continue", &json!({ "key": key, "run": run, "turn": asked["id"], "question": "Como evitar a multiplicação?" }));
        assert_eq!(status, 200, "{step}");
        none(step["id"].as_str().unwrap());
        // The performance question and an investigation make no question with a selection.
        let before = files_of(&store, &key, &run);
        api(&store, "/api/performance-question", &json!({ "key": key, "run": run, "question": "Quais funções pesam?" }));
        assert!(files_of(&store, &key, &run).iter().all(|(path, _)| !path.ends_with(SELECTION) || before.iter().any(|(p, _)| p == path)));
        // A suggestion is still recorded, as before.
        let (status, suggested) = api(&store, "/api/envelope/suggest", &json!({ "key": key, "run": run, "nodes": [page.gathered[0]] }));
        assert_eq!(status, 200, "{suggested}");
        assert!(store.verify_suggestion(&key, &run, suggested["suggestion"].as_str().unwrap()).is_ok());
    }

    // P1: `measure_divergence` (format 2) and format 1 kept as it was.

    /// A gathered question from a selection over `roda`'s suggestion, made after `between` runs on
    /// the store. Returns the turn.
    fn selected_question(store: &Store, key: &str, run: &str, between: impl FnOnce()) -> String {
        let node = symbol_id(store, key, run, "roda");
        let mut page = Page::new(store, key, run);
        page.toggle(&node);
        page.suggest(store, key, run, std::slice::from_ref(&node));
        between();
        store.ask_gathered(key, run, "Por que roda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned()
    }

    /// The execution `basis` copied as a later one, `id`, in the same second: the folder reserved later wins.
    fn later_measure(store: &Store, key: &str, run: &str, basis: &str, id: &str) {
        let executions = store.run_dir(key, run).unwrap().join("executions");
        let mut later: Value = serde_json::from_slice(&fs::read(executions.join(basis).join("execution.json")).unwrap()).unwrap();
        later["id"] = json!(id);
        fs::create_dir_all(executions.join(id)).unwrap();
        fs::write(executions.join(id).join("execution.json"), serde_json::to_vec_pretty(&later).unwrap()).unwrap();
    }

    #[test]
    fn a_question_whose_selection_rested_on_an_earlier_measure_records_that_it_diverges_and_is_still_sent() {
        // The audit's probe: the suggestion read 01-contagem-tudo, the question 99-depois.
        let (_scratch, store, key, run) = measured_mix();
        let turn = selected_question(&store, &key, &run, || later_measure(&store, &key, &run, "01-contagem-tudo", "99-depois"));
        let (question, bytes, package) = package_of(&store, &key, &run, &turn);
        assert_eq!((package.format_version, &question["evidence"]["format_version"]), (PackageFormat::V2, &json!(2)));
        let MeasureAtQuestion::Ok { execution: at, .. } = &package.measure.at_question else { panic!("{:?}", package.measure.at_question) };
        let MeasureSelectionBasis::Ok { execution: basis, .. } = &package.measure.selection_basis else { panic!("{:?}", package.measure.selection_basis) };
        assert_eq!((at.as_str(), basis.as_str()), ("99-depois", "01-contagem-tudo"));
        assert_eq!((package.measure.measure_diverges, package.measure.measure_divergence.clone()), (None, Some(MeasureDivergence::Diverges {})));
        // H1 to H3 pass, H4 is told apart, and there is no format 1 warning.
        let (_, now) = evidence_of(&store, &key, &run, &turn);
        assert_eq!(statuses(&now), ["passed", "passed", "passed"]);
        assert_eq!((&now["h4"]["status"], &now["warnings"]), (&json!("reproduced"), &json!([])));
        assert_eq!(now["package"]["measure"]["measure_divergence"], json!({ "status": "diverges" }));
        // The divergence blocks nothing: the question is sent, linked to this package.
        let (status, sent) = api(&store, "/api/send", &json!({ "key": key, "run": run, "turn": turn, "provider": "test" }));
        assert_eq!(status, 200, "{sent}");
        assert_eq!(sent["link"]["evidence_sha256"], json!(hash(&bytes)));
        assert_eq!(sent["link"]["checks"], json!(["prepare::verify", "claims::verify", "evidence_h1", "evidence_h2", "evidence_h3"]));
        // H3 made the same package again: the stored bytes did not change.
        assert_eq!(package_of(&store, &key, &run, &turn).1, bytes);
    }

    #[test]
    fn each_state_of_the_comparison_comes_from_a_real_question_and_is_made_again_by_h3() {
        // Same measure on both sides.
        let (_scratch, store, key, run) = measured_mix();
        let same = selected_question(&store, &key, &run, || {});
        // No selection.
        let node = symbol_id(&store, &key, &run, "roda");
        let plain = store.ask_gathered(&key, &run, "Por que roda?", std::slice::from_ref(&node), false, None).unwrap()["id"].as_str().unwrap().to_owned();
        // The question's measure unreadable: an execution that does not parse is the latest.
        let broken = store.run_dir(&key, &run).unwrap().join("executions/99-quebrada");
        let at_unreadable = selected_question(&store, &key, &run, || {
            fs::create_dir_all(&broken).unwrap();
            fs::write(broken.join("execution.json"), b"{ metade").unwrap();
        });
        // The suggestion's measure unreadable, the question's read once the broken one is gone.
        let mut page = Page::new(&store, &key, &run);
        page.toggle(&node);
        page.suggest(&store, &key, &run, std::slice::from_ref(&node));
        fs::remove_dir_all(&broken).unwrap();
        let basis_unreadable = store.ask_gathered(&key, &run, "Por que roda?", &page.gathered, false, Some(page.origin())).unwrap()["id"].as_str().unwrap().to_owned();
        let reason = |side, read_status| NotComparable { side, read_status };
        let expected = [
            (&same, MeasureDivergence::Same {}),
            (&plain, MeasureDivergence::NotApplicable {}),
            (&at_unreadable, MeasureDivergence::NotComparable { reasons: vec![reason(MeasureSide::AtQuestion, Incomparable::Unreadable)] }),
            (&basis_unreadable, MeasureDivergence::NotComparable { reasons: vec![reason(MeasureSide::SelectionBasis, Incomparable::Unreadable)] }),
        ];
        for (turn, divergence) in expected {
            let (_, bytes, package) = package_of(&store, &key, &run, turn);
            assert_eq!(package.measure.measure_divergence, Some(divergence.clone()), "{turn}");
            assert_eq!(package.measure.measure_divergence, Some(evidence::measure_divergence(&package.measure.at_question, &package.measure.selection_basis)), "{turn}");
            // H1 to H3 pass, and H3 made the same bytes again.
            assert_eq!(refusal_of(&store, &key, &run, turn), "", "{turn}");
            assert_eq!(package_of(&store, &key, &run, turn).1, bytes, "{turn}");
        }
        // The reasons as written: which side, and why.
        let (_, _, package) = package_of(&store, &key, &run, &basis_unreadable);
        assert_eq!(serde_json::to_value(package.measure.measure_divergence).unwrap(),
            json!({ "status": "not_comparable", "reasons": [{ "side": "selection_basis", "read_status": "unreadable" }] }));
    }

    #[test]
    fn a_format_2_divergence_changed_consistently_is_refused_and_one_changed_to_agree_is_made_again() {
        let (_scratch, store, key, run) = measured_mix();
        let turn = selected_question(&store, &key, &run, || later_measure(&store, &key, &run, "01-contagem-tudo", "99-depois"));
        let reason = |side, read_status| NotComparable { side, read_status };
        // Each other state, written with the mark made again: H1 refuses it.
        for wrong in [MeasureDivergence::Same {}, MeasureDivergence::NotApplicable {},
            MeasureDivergence::NotComparable { reasons: vec![reason(MeasureSide::AtQuestion, Incomparable::Unreadable)] },
            MeasureDivergence::NotComparable { reasons: vec![] }] {
            forge(&store, &key, &run, &turn, |p| p.measure.measure_divergence = Some(wrong.clone()));
            assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: Shape { what: \"measure divergence\""), "{wrong:?}");
        }
        forge(&store, &key, &run, &turn, |p| p.measure.measure_divergence = Some(MeasureDivergence::Diverges {}));
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // The question's measure changed and the state made to agree with it: H1 passes, and H3 keeps
        // the measure as recorded (it is not read again; a limitation since the Passo 9).
        forge(&store, &key, &run, &turn, |p| {
            let MeasureSelectionBasis::Ok { execution, execution_sha256, .. } = p.measure.selection_basis.clone() else { panic!() };
            p.measure.at_question = MeasureAtQuestion::Ok { execution, execution_sha256 };
            p.measure.measure_divergence = Some(MeasureDivergence::Same {});
        });
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        // A reason changed in a not comparable package: the side, or the cause.
        let (_scratch2, store, key, run) = measured_mix();
        let broken = store.run_dir(&key, &run).unwrap().join("executions/99-quebrada");
        let turn = selected_question(&store, &key, &run, || {
            fs::create_dir_all(&broken).unwrap();
            fs::write(broken.join("execution.json"), b"{ metade").unwrap();
        });
        assert_eq!(refusal_of(&store, &key, &run, &turn), "");
        for wrong in [vec![reason(MeasureSide::SelectionBasis, Incomparable::Unreadable)], vec![reason(MeasureSide::AtQuestion, Incomparable::NotRecorded)],
            vec![reason(MeasureSide::AtQuestion, Incomparable::Unreadable), reason(MeasureSide::SelectionBasis, Incomparable::Unreadable)]] {
            forge(&store, &key, &run, &turn, |p| p.measure.measure_divergence = Some(MeasureDivergence::NotComparable { reasons: wrong.clone() }));
            assert!(refusal_of(&store, &key, &run, &turn).starts_with("H1: Shape { what: \"measure divergence\""), "{wrong:?}");
        }
    }

    /// The format 1 store written by the code before format 2 (P1), copied to a scratch folder. The
    /// files are text: a checkout that wrote CRLF gets back the LF they were written with.
    fn format_1_store() -> (Scratch, Store, Value) {
        let scratch = project();
        fn copy(from: &Path, to: &Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap().flatten() {
                let (a, b) = (entry.path(), to.join(entry.file_name()));
                if a.is_dir() { copy(&a, &b) } else { fs::write(&b, String::from_utf8(fs::read(&a).unwrap()).unwrap().replace("\r\n", "\n")).unwrap(); }
            }
        }
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/evidence-v1");
        copy(&fixture, &scratch.0.join("v1"));
        let manifest: Value = serde_json::from_slice(&fs::read(scratch.0.join("v1/manifest.json")).unwrap()).unwrap();
        let store = Store { dir: scratch.0.join("v1/studio"), modelfiles: scratch.0.join("modelfiles") };
        (scratch, store, manifest)
    }

    #[test]
    fn a_format_1_package_written_before_format_2_passes_h1_to_h3_as_it_was_and_keeps_its_bytes() {
        let (_scratch, store, manifest) = format_1_store();
        let (key, run) = (manifest["key"].as_str().unwrap(), manifest["run"].as_str().unwrap());
        // The packages as the earlier code wrote them: these bytes, pinned here.
        let pinned = [("01", "b7fa86ffa485ad8f0167a9c1a7b3aedfdcbee752fcdf8780e26752fa93dcf9b1"), ("02", "b0147ed7a7305492994e8a0bcfa35a9832e78893f628cc13f401886869c9c9ac")];
        assert_eq!((manifest["divergent"].as_str(), manifest["plain"].as_str()), (Some("01"), Some("02")));
        for (turn, sha) in pinned {
            let (question, bytes, package) = package_of(&store, key, run, turn);
            assert_eq!((hash(&bytes).as_str(), &question["evidence"]), (sha, &json!({ "format_version": 1, "sha256": sha })), "{turn}");
            assert_eq!((package.format_version, package.measure.measure_diverges, &package.measure.measure_divergence), (PackageFormat::V1, Some(false), &None), "{turn}");
            // H1 to H3 pass, by the format 1 rule; nothing is rewritten (`evidence_of` compares every
            // file of the store before and after).
            assert_eq!(refusal_of(&store, key, run, turn), "", "{turn}");
            let (_, now) = evidence_of(&store, key, run, turn);
            assert_eq!(statuses(&now), ["passed", "passed", "passed"], "{turn}");
            // What the earlier code answered: the same checks, the same H4, the same measure.
            let old = &manifest["old_code"][turn];
            assert_eq!((&now["checks"], &now["h4"], &now["package"]["measure"]), (&old["checks"], &old["h4"], &old["measure"]), "{turn}");
            // Its false proves nothing, and the answer says so.
            assert_eq!(now["warnings"], json!(["measure_diverges_not_computed"]), "{turn}");
            assert_eq!(package_of(&store, key, run, turn).1, bytes, "{turn}");
        }
        // The divergent one: its two measures differ, yet format 1 wrote false, and is checked as format 1.
        let (_, _, package) = package_of(&store, key, run, "01");
        assert_eq!(evidence::measure_divergence(&package.measure.at_question, &package.measure.selection_basis), MeasureDivergence::Diverges {});
        let (status, sent) = api(&store, "/api/send", &json!({ "key": key, "run": run, "turn": "01", "provider": "test" }));
        assert_eq!((status, &sent["link"]["evidence_sha256"]), (200, &json!(pinned[0].1)), "{sent}");
    }

    #[test]
    fn a_format_1_package_is_made_again_by_its_own_rule_and_not_read_as_format_2() {
        let (_scratch, store, manifest) = format_1_store();
        let (key, run) = (manifest["key"].as_str().unwrap(), manifest["run"].as_str().unwrap());
        // A format 1 package saying true: that format never wrote it, and H3 makes false again.
        forge(&store, key, run, "01", |p| p.measure.measure_diverges = Some(true));
        assert_eq!(refusal_of(&store, key, run, "01"), "H3: o código atual não refaz measure do pacote; nada foi enviado");
        forge(&store, key, run, "01", |p| p.measure.measure_diverges = Some(false));
        assert_eq!(refusal_of(&store, key, run, "01"), "");
        // Turned into format 2 with the state worked out, and its mark with it: H3 makes format 2 again,
        // which this package's question did not have when it was written.
        forge(&store, key, run, "02", |p| {
            p.format_version = PackageFormat::V2;
            p.measure.measure_diverges = None;
            p.measure.measure_divergence = Some(evidence::measure_divergence(&p.measure.at_question, &p.measure.selection_basis));
        });
        remark(&store, key, run, "02", |q| q["evidence"]["format_version"] = json!(2));
        assert_eq!(refusal_of(&store, key, run, "02"), "");
        // The mark alone says 2: refused before anything else.
        let (_scratch, store, _) = format_1_store();
        remark(&store, key, run, "01", |q| q["evidence"]["format_version"] = json!(2));
        assert_eq!(refusal_of(&store, key, run, "01"), "H1: o formato da marca não é o do pacote; nada foi enviado");
        // H2 as before: a unit whose text the snapshot does not give is refused by H2, in format 1 too.
        let (_scratch, store, _) = format_1_store();
        forge(&store, key, run, "01", |p| p.units[0].text_sha256 = Sha256Hex::try_from("0".repeat(64)).unwrap());
        assert!(refusal_of(&store, key, run, "01").starts_with("H2: TextDiffers"));
    }
}
