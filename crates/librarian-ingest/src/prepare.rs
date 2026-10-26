//! Bridge from a verified edition and its lexical search to a validated query bundle.
//! Facts here are structural records (where a declaration is), never answers to the question.
use crate::{
    graph::Graph,
    hash,
    owner::{owner_context, Added},
    search::{chain_report, Lexicon, SearchIndex},
    Catalog, Result,
};
use librarian_graph_engine::{prepare_query_formatted, LineFormat, PreparedQuery};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::Path};

/// Self-contained request: re-running it against the same edition reproduces every byte.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub question: String,
    #[serde(default)]
    pub lexicon: Lexicon,
    pub top: usize,
    pub depth: usize,
    /// How many leading hits become facts; the rest stay in search.json only.
    pub hits: usize,
    /// Extra lines around each declaration in the exported context.
    #[serde(default)]
    pub radius: usize,
    /// Context lines as {number, text} with original file lines; false keeps plain strings.
    #[serde(default)]
    pub numbered_lines: bool,
    /// Consumer explanation policy; None uses the Graph Engine defaults.
    #[serde(default)]
    pub instructions: Option<Vec<String>>,
    /// Editions with diagnostics are incomplete; they are refused unless explicitly accepted.
    #[serde(default)]
    pub accept_diagnostics: bool,
    /// Chain step: declarations named by checked answers to an earlier question replace the
    /// lexical search; `top`, `hits` and `lexicon` are then unused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<Target>>,
    /// Facts a person reviewed about the targets (chain step only). Each quote must still be
    /// the edition's text at the same lines, and each fact must lie inside a target.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewed_facts: Vec<ReviewedFact>,
    /// Chain step (G6d): each method target also brings its owner type and the declarations
    /// that write the fields it reads. Needs an edition of profile v4.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub owner_context: bool,
    /// Lexical search only: leaves out test symbols and the declarations inside them. Absent,
    /// the request is the same as before.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exclude_tests: bool,
    /// The targets were picked by the person on a "what does it call?" path (P1b), not named by
    /// checked answers; the report says so. Absent, the request is the same as before.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub picked: bool,
}

/// A declaration as a checked claim locates it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub path: String,
    pub symbol_name: String,
    pub declaration_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedFact {
    pub id: String,
    pub statement: String,
    pub authorship: String,
    pub path: String,
    pub line: usize,
    pub end_line: usize,
    pub citations: Vec<Quote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quote {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub quote: String,
}

pub struct Prepared {
    pub search: Vec<u8>,
    pub dossier: String,
    pub query: PreparedQuery,
}

pub fn prepare(catalog: &Catalog, request: &Request) -> Result<Prepared> {
    if !catalog.diagnostics.is_empty() && !request.accept_diagnostics {
        return Err("Edition has diagnostics; set accept_diagnostics to use it".into());
    }
    let (report, chosen) = match &request.targets {
        Some(targets) => {
            let mut ids = targets.iter().map(|t| resolve(catalog, t)).collect::<Result<Vec<_>>>()?;
            if ids.is_empty() || (1..ids.len()).any(|i| ids[..i].contains(&ids[i])) {
                return Err("A chain step needs distinct targets".into());
            }
            let graph = Graph::build(catalog);
            let context = if request.owner_context {
                let context = owner_context(catalog, &graph, &ids)?;
                ids.extend(context.added.iter().map(|a| a.symbol_id.clone()));
                Some(context)
            } else {
                None
            };
            (chain_report(catalog, &graph, &request.question, &ids, request.depth, context, request.picked)?, ids.len())
        }
        None => {
            if request.hits == 0 || request.hits > request.top {
                return Err("hits must be between 1 and top".into());
            }
            if !request.reviewed_facts.is_empty() || request.owner_context {
                return Err("Reviewed facts and owner context are carried only by a chain step".into());
            }
            let mut index = SearchIndex::new(catalog, request.lexicon.clone())?;
            if request.exclude_tests {
                index = index.without_tests();
            }
            let report = index.search(&request.question, request.top, request.depth)?;
            if report.hits.len() < request.hits {
                return Err(format!(
                    "Search returned {} candidates; {} requested",
                    report.hits.len(),
                    request.hits
                )
                .into());
            }
            (report, request.hits)
        }
    };
    let mut search = serde_json::to_vec_pretty(&report)?;
    search.push(b'\n');

    let profile = &catalog.manifest.extractor;
    let mut sources: Vec<(String, &crate::Source)> = vec![];
    let mut facts = vec![];
    for (index, hit) in report.hits.iter().take(chosen).enumerate() {
        let symbol = &catalog.symbols[&hit.symbol_id];
        let source = &catalog.sources[&symbol.source_id];
        let source_key = match sources.iter().find(|(_, s)| s.id == source.id) {
            Some((key, _)) => key.clone(),
            None => {
                let key = format!("S{}", sources.len() + 1);
                sources.push((key.clone(), source));
                key
            }
        };
        let chunk = &catalog.chunks[&symbol.chunk_id];
        let mut statement = format!(
            "{} `{}` declarado em {}, linhas {}–{}.",
            symbol.kind, symbol.qualified_name, source.path, chunk.start_line, chunk.end_line
        );
        let added = report.owner_context.iter().flat_map(|c| &c.added).find(|a| a.symbol_id == hit.symbol_id);
        if let Some(added) = added {
            statement.push(' ');
            statement.push_str(&owner_statement(added));
        }
        facts.push(json!({
            "id": format!("E{}", index + 1),
            "statement": statement,
            "authorship": format!("{profile}: registro estrutural da edition verificada"),
            "source_id": source_key,
            "line": chunk.start_line,
            "end_line": chunk.end_line,
        }));
    }
    for fact in &request.reviewed_facts {
        let inside = report.hits.iter().take(chosen).any(|h| {
            h.path == fact.path && h.start_line <= fact.line && fact.end_line <= h.end_line
        });
        if !fact.id.starts_with('F') || fact.statement.trim().is_empty() || fact.end_line < fact.line || !inside {
            return Err(format!("{}: reviewed fact outside the chain targets or malformed", fact.id).into());
        }
        for quote in &fact.citations {
            check_quote(catalog, &quote.path, quote.start_line, quote.end_line, &quote.quote)
                .map_err(|e| format!("{}: {e}", fact.id))?;
        }
        let key = sources
            .iter()
            .find(|(_, s)| s.path == fact.path)
            .map(|(k, _)| k.clone())
            .ok_or("Reviewed fact source missing")?;
        facts.push(json!({
            "id": fact.id,
            "statement": fact.statement,
            "authorship": fact.authorship,
            "source_id": key,
            "line": fact.line,
            "end_line": fact.end_line,
        }));
    }
    let sources: Vec<Value> = sources
        .iter()
        .map(|(key, source)| {
            json!({
                "id": key,
                "kind": source.language,
                "content": "code",
                "path": source.path,
                "sha256": source.sha256,
                "git_commit": catalog.manifest.git.head,
                "lines": snapshot_lines(catalog, &source.id),
            })
        })
        .collect();
    let mut unknowns: Vec<String> = report.limits.clone();
    unknowns.push(
        "Fichas descrevem onde estão declarações; não respondem à pergunta nem afirmam comportamento."
            .into(),
    );
    if let Some(context) = &report.owner_context {
        unknowns.push(
            "Contexto do dono: o tipo é a declaração que contém o método ou o tipo cujo nome ela escreve, e quem escreve os campos vem de atribuições pelo próprio receptor (por exemplo `self.campo = …`) e de literais do tipo; nada foi resolvido pelo compilador."
                .into(),
        );
        if context.not_followed_total > 0 {
            unknowns.push(format!(
                "{} escritas de campos lidos por outro receptor (por exemplo `x.campo = …`) não foram seguidas: o tipo do receptor não é conhecido. Estão listadas no search.json.",
                context.not_followed_total
            ));
        }
        if context.truncated {
            unknowns.push(format!("Contexto do dono truncado em {} declarações.", crate::owner::MAX_ADDED));
        }
        unknowns.extend(context.notes.iter().cloned());
    }
    if !request.reviewed_facts.is_empty() {
        unknowns.push(
            "Fichas F são afirmações de uma pergunta anterior revisadas por uma pessoa, com os trechos conferidos contra esta edition; não são registros estruturais."
                .into(),
        );
    }
    if !catalog.diagnostics.is_empty() {
        unknowns.push(format!(
            "Edition com {} diagnósticos: extração incompleta em algumas fontes.",
            catalog.diagnostics.len()
        ));
    }
    let dossier = serde_json::to_string_pretty(&json!({
        "id": format!("{}:{}", if request.targets.is_some() { "chain" } else { "search" }, hash(&search)),
        "unknowns": unknowns,
        "sources": sources,
        "facts": facts,
    }))?;
    let mut fact_ids: Vec<String> = (1..=chosen).map(|i| format!("E{i}")).collect();
    fact_ids.extend(request.reviewed_facts.iter().map(|f| f.id.clone()));
    let fact_ids: Vec<&str> = fact_ids.iter().map(String::as_str).collect();
    let format = if request.numbered_lines {
        LineFormat::Numbered
    } else {
        LineFormat::Plain
    };
    let query = prepare_query_formatted(
        &dossier,
        Some(&request.question),
        &fact_ids,
        request.radius,
        request.instructions.as_deref(),
        format,
        |source| check_source(catalog, source),
    )?;
    Ok(Prepared {
        search,
        dossier,
        query,
    })
}

/// Why an owner-context declaration is in the step, with the lines where the link is written.
fn owner_statement(added: &Added) -> String {
    let fields = added.fields.iter().map(|f| format!("`{f}`")).collect::<Vec<_>>().join(", ");
    let lines = added.evidence.iter().map(|e| e.start_line.to_string()).collect::<Vec<_>>().join(", ");
    let lines = format!("{} {lines}", if added.evidence.len() == 1 { "linha" } else { "linhas" });
    let read = if fields.is_empty() { "nenhum".to_owned() } else { fields.clone() };
    match (added.reason.as_str(), added.resolution.as_str()) {
        ("owner_type", "container") => format!(
            "Contexto do dono: declaração que contém `{}`; campos que ele lê: {read}.",
            added.for_target
        ),
        ("owner_type", _) => format!(
            "Contexto do dono: tipo com o nome escrito no bloco que contém `{}` ({lines}); campos que ele lê: {read}.",
            added.for_target
        ),
        (_, "owner_literal") => format!(
            "Contexto do dono: preenche {fields} num literal do tipo ({lines}); `{}` lê esses campos.",
            added.for_target
        ),
        _ => format!(
            "Contexto do dono: escreve {fields} pelo próprio receptor ({lines}); `{}` lê esses campos.",
            added.for_target
        ),
    }
}

/// The declaration a target names: same file, same name, its chunk covering the declaration
/// line. Nested symbols resolve to the innermost; none is an error, never a guess.
fn resolve(catalog: &Catalog, target: &Target) -> Result<String> {
    let source = catalog
        .source_by_path(&target.path)
        .ok_or_else(|| format!("{}: not in the edition", target.path))?;
    let found = catalog
        .symbols
        .values()
        .filter(|s| s.source_id == source.id && s.name == target.symbol_name)
        .map(|s| (s, &catalog.chunks[&s.chunk_id]))
        .filter(|(_, c)| c.start_line <= target.declaration_line && target.declaration_line <= c.end_line)
        .min_by_key(|(s, c)| (c.end_line - c.start_line, s.id.clone()));
    match found {
        Some((symbol, _)) => Ok(symbol.id.clone()),
        None => Err(format!(
            "{}: no `{}` declared at line {}",
            target.path, target.symbol_name, target.declaration_line
        )
        .into()),
    }
}

/// The quote is still the text of lines `start`–`end` of `path` in this edition, joined by LF.
/// Nothing is searched elsewhere: a shifted line is a mismatch.
pub fn check_quote(catalog: &Catalog, path: &str, start: usize, end: usize, quote: &str) -> std::result::Result<(), String> {
    let source = catalog.source_by_path(path).ok_or_else(|| format!("{path} não está nesta edition"))?;
    let lines = snapshot_lines(catalog, &source.id);
    let current = (start >= 1 && end >= start && end <= lines.len()).then(|| lines[start - 1..end].join("\n"));
    if current.as_deref() != Some(quote) {
        return Err(format!("{path}:{start}–{end} mudou desde a revisão"));
    }
    Ok(())
}

/// Lines exactly as the dossier sees them; the snapshot was already verified by the catalog.
fn snapshot_lines(catalog: &Catalog, source_id: &str) -> Vec<String> {
    String::from_utf8_lossy(&catalog.snapshots[source_id])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Dossier sources must be the edition's verified snapshots, byte-identical by hash and lines.
fn check_source(
    catalog: &Catalog,
    source: &librarian_graph_engine::Source,
) -> std::result::Result<Option<Value>, String> {
    let path = source.path.as_deref().ok_or("Dossier source without path")?;
    let recorded = catalog
        .source_by_path(path)
        .ok_or_else(|| format!("{path}: not in the edition"))?;
    if source.sha256.as_deref() != Some(recorded.sha256.as_str())
        || source.lines != snapshot_lines(catalog, &recorded.id)
    {
        return Err(format!("{path}: dossier source differs from the edition snapshot"));
    }
    Ok(None)
}

/// Writes request.json (exact bytes), search.json and the validated bundle into a new directory.
pub fn write(catalog: &Catalog, request_bytes: &[u8], dir: &Path) -> Result<Prepared> {
    let request: Request = serde_json::from_slice(request_bytes)?;
    let prepared = prepare(catalog, &request)?;
    fs::create_dir(dir)?;
    crate::write_new(&dir.join("request.json"), request_bytes)?;
    crate::write_new(&dir.join("search.json"), &prepared.search)?;
    let origin = format!("edition:{}", hash(&serde_json::to_vec(&catalog.manifest)?));
    prepared
        .query
        .write_bundle(&dir.join("bundle"), &origin, "request.json")?;
    Ok(prepared)
}

/// Re-derives everything from request.json and the edition; any differing byte is refused.
pub fn verify(catalog: &Catalog, dir: &Path) -> Result<()> {
    let request: Request = serde_json::from_slice(&fs::read(dir.join("request.json"))?)?;
    let prepared = prepare(catalog, &request)?;
    let query = format!("{}\n", prepared.query.query_json().ok_or("Missing query")?);
    let bundle = dir.join("bundle");
    for (path, expected) in [
        (dir.join("search.json"), prepared.search.as_slice()),
        (bundle.join("dossier.json"), prepared.dossier.as_bytes()),
        (bundle.join("question.txt"), request.question.as_bytes()),
        (bundle.join("query.json"), query.as_bytes()),
    ] {
        if fs::read(&path)? != expected {
            return Err(format!("Prepared file differs: {}", path.display()).into());
        }
    }
    let origin: Value = serde_json::from_slice(&fs::read(bundle.join("origin.json"))?)?;
    if origin["dossier"]["sha256"] != hash(prepared.dossier.as_bytes())
        || origin["query"]["sha256"] != hash(query.as_bytes())
        || origin["selection"]["fact_ids"] != json!(prepared.query.selected_fact_ids())
    {
        return Err("origin.json does not match the prepared bundle".into());
    }
    Ok(())
}
