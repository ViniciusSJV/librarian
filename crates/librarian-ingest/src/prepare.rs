//! Bridge from a verified edition and its lexical search to a validated query bundle.
//! Facts here are structural records (where a declaration is), never answers to the question.
use crate::{
    hash,
    search::{Lexicon, SearchIndex},
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
}

pub struct Prepared {
    pub search: Vec<u8>,
    pub dossier: String,
    pub query: PreparedQuery,
}

pub fn prepare(catalog: &Catalog, request: &Request) -> Result<Prepared> {
    if request.hits == 0 || request.hits > request.top {
        return Err("hits must be between 1 and top".into());
    }
    if !catalog.diagnostics.is_empty() && !request.accept_diagnostics {
        return Err("Edition has diagnostics; set accept_diagnostics to use it".into());
    }
    let report = SearchIndex::new(catalog, request.lexicon.clone())?.search(
        &request.question,
        request.top,
        request.depth,
    )?;
    if report.hits.len() < request.hits {
        return Err(format!(
            "Search returned {} candidates; {} requested",
            report.hits.len(),
            request.hits
        )
        .into());
    }
    let mut search = serde_json::to_vec_pretty(&report)?;
    search.push(b'\n');

    let profile = &catalog.manifest.extractor;
    let mut sources: Vec<(String, &crate::Source)> = vec![];
    let mut facts = vec![];
    for (index, hit) in report.hits.iter().take(request.hits).enumerate() {
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
        facts.push(json!({
            "id": format!("E{}", index + 1),
            "statement": format!(
                "{} `{}` declarado em {}, linhas {}–{}.",
                symbol.kind, symbol.qualified_name, source.path, chunk.start_line, chunk.end_line
            ),
            "authorship": format!("{profile}: registro estrutural da edition verificada"),
            "source_id": source_key,
            "line": chunk.start_line,
            "end_line": chunk.end_line,
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
    if !catalog.diagnostics.is_empty() {
        unknowns.push(format!(
            "Edition com {} diagnósticos: extração incompleta em algumas fontes.",
            catalog.diagnostics.len()
        ));
    }
    let dossier = serde_json::to_string_pretty(&json!({
        "id": format!("search:{}", hash(&search)),
        "unknowns": unknowns,
        "sources": sources,
        "facts": facts,
    }))?;
    let fact_ids: Vec<String> = (1..=request.hits).map(|i| format!("E{i}")).collect();
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
