//! Answer format `claims`: a claims::Context derived from the verified prepare bundle and the
//! edition, its query, and the mechanical check of each answer. A passed check means the answer
//! points at real lines in the requested shape; its meaning stays pending.
//! Layout inside a turn: claims/{context.json, query.json}; each send gets a check.json.
use librarian_ingest::{hash, Catalog, Result};
use librarian_llm_engine::claims::{self, Context, Evidence, Policy, ReviewedFact, Symbol};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub const DIR: &str = "claims";
const LIMITS: &str = "Conferência mecânica: a resposta aponta para linhas reais da edition no formato pedido. O sentido continua pendente; accepted é sempre false.";

/// Evidence blocks are the bundle's context windows, as the prose query shows them; symbols are
/// the edition's declarations whose chunk fits inside a window.
pub fn context(catalog: &Catalog, query: &Value, policy: Policy, project_instructions: Vec<String>) -> Result<Context> {
    let evidence = &query["evidence"];
    // One selection keeps its window inline; several share numbered blocks in `contexts`.
    let (windows, selections) = match evidence["contexts"].as_array() {
        Some(blocks) => {
            let selections: Vec<&Value> =
                evidence["selections"].as_array().ok_or("Query sem selections")?.iter().collect();
            let mut windows = vec![];
            for block in blocks {
                let source = selections
                    .iter()
                    .map(|s| &s["source"])
                    .find(|s| s["id"] == block["source_id"])
                    .ok_or("Bloco de contexto sem fonte")?;
                let id = block["id"].as_str().ok_or("Bloco de contexto sem id")?;
                windows.push((id.to_owned(), text(&source["path"])?, block));
            }
            (windows, selections)
        }
        // A single window is named after its fact (E1), as the project instructions call it.
        None => {
            let (id, path) = (text(&evidence["fact_id"])?, text(&evidence["source"]["path"])?);
            (vec![(id, path, &evidence["source"]["context"])], vec![evidence])
        }
    };
    let mut items = vec![];
    for (id, path, window) in windows {
        let start = window["start_line"].as_u64().ok_or("Janela sem start_line")? as usize;
        let mut lines = vec![];
        for (offset, line) in window["lines"].as_array().ok_or("Janela sem lines")?.iter().enumerate() {
            // Numbered lines must agree with the window; plain ones are counted from start_line.
            if line.get("number").is_some_and(|n| n.as_u64() != Some((start + offset) as u64)) {
                return Err(format!("{id}: numeração diferente da janela").into());
            }
            lines.push(text(line.get("text").unwrap_or(line))?);
        }
        // A claims excerpt cannot end in an empty line; empty lines carry no evidence.
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        if lines.is_empty() {
            continue;
        }
        let end = start + lines.len() - 1;
        let symbols = symbols(catalog, &path, start, &lines);
        items.push(Evidence { id, path, start_line: start, end_line: end, excerpt: lines.join("\n"), symbols });
    }
    let mut limits: Vec<String> = vec![];
    for unknown in selections.iter().flat_map(|s| s["evidence_unknowns"].as_array().into_iter().flatten()) {
        let unknown = text(unknown)?;
        if !limits.contains(&unknown) {
            limits.push(unknown);
        }
    }
    // Chain steps carry reviewed facts as F facts (prepare allows that prefix only for them).
    let mut reviewed_facts = vec![];
    for selection in selections.iter().filter(|s| s["fact_id"].as_str().is_some_and(|id| id.starts_with('F'))) {
        let source = &selection["source"];
        let line = source["line"].as_u64().ok_or("Ficha F sem linha")? as usize;
        reviewed_facts.push(ReviewedFact {
            id: text(&selection["fact_id"])?,
            statement: text(&selection["statement"])?,
            authorship: text(&selection["authorship"])?,
            path: text(&source["path"])?,
            start_line: line,
            end_line: source["end_line"].as_u64().map_or(line, |e| e as usize),
        });
    }
    let context = Context {
        schema_version: 1,
        require_location: items.iter().any(|e| !e.symbols.is_empty()),
        policy,
        evidence: items,
        limits,
        project_instructions,
        strict_text: true,
        precise_location: true,
        reviewed_facts,
        allow_patch: false,
    };
    context.validate()?;
    Ok(context)
}

/// Declaration line: the first code line of the chunk naming the symbol as a whole identifier.
/// Chunks start at their attributes, doc comments or decorators, so those lines are skipped.
/// Still a heuristic over text, not a parser.
fn symbols(catalog: &Catalog, path: &str, start: usize, lines: &[String]) -> Vec<Symbol> {
    let Some(source) = catalog.source_by_path(path) else { return vec![] };
    let end = start + lines.len() - 1;
    let mut found: Vec<Symbol> = vec![];
    for symbol in catalog.symbols().values().filter(|s| s.source_id == source.id) {
        let Some(chunk) = catalog.chunk(&symbol.chunk_id) else { continue };
        if chunk.start_line < start || chunk.end_line > end {
            continue;
        }
        let declaration = (chunk.start_line..=chunk.end_line)
            .filter(|n| !annotation(&lines[n - start]))
            .find_map(|n| Some((n, identifier_column(&lines[n - start], &symbol.name)?)));
        let Some((line, column)) = declaration else { continue };
        if !found.iter().any(|s| s.name == symbol.name && s.declaration_line == line) {
            found.push(Symbol { name: symbol.name.clone(), declaration_line: line, name_column: column, end_line: chunk.end_line });
        }
    }
    found.sort_by(|a, b| (a.declaration_line, &a.name).cmp(&(b.declaration_line, &b.name)));
    found
}

/// Lines that annotate a declaration instead of declaring: `#[attr]`, `///` and other comments,
/// Python `@decorator` and `# comment`.
fn annotation(line: &str) -> bool {
    let line = line.trim_start();
    ["#", "//", "/*", "*", "@"].iter().any(|p| line.starts_with(p))
}

/// One-based Unicode scalar column of `name` as a whole identifier in `line`.
fn identifier_column(line: &str, name: &str) -> Option<usize> {
    let chars: Vec<char> = line.chars().collect();
    let target: Vec<char> = name.chars().collect();
    let part = |c: &char| c.is_alphanumeric() || *c == '_';
    (0..chars.len().checked_sub(target.len())? + 1).find_map(|i| {
        let fits = chars[i..i + target.len()] == target[..]
            && (i == 0 || !part(&chars[i - 1]))
            && chars.get(i + target.len()).is_none_or(|c| !part(c));
        fits.then_some(i + 1)
    })
}

fn text(value: &Value) -> Result<String> {
    Ok(value.as_str().ok_or("Texto esperado na query preparada")?.to_owned())
}

fn pretty(value: &impl serde::Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Opt-in rules of the claims contract. New questions get all of them; a recorded question
/// keeps the ones it was asked with, so its bytes still verify.
struct Rules {
    strict_text: bool,
    precise_location: bool,
    /// Chosen per project, like the policy, and recorded with each question.
    allow_patch: bool,
}


fn derive(catalog: &Catalog, turn: &Path, policy: Policy, project_instructions: Vec<String>, rules: Rules) -> Result<(Vec<u8>, Vec<u8>)> {
    let bundle = turn.join("prepared/bundle");
    let query: Value = serde_json::from_slice(&fs::read(bundle.join("query.json"))?)?;
    let question = fs::read_to_string(bundle.join("question.txt"))?;
    let mut context = context(catalog, &query, policy, project_instructions)?;
    context.strict_text = rules.strict_text;
    context.precise_location = rules.precise_location;
    context.allow_patch = rules.allow_patch;
    let mut claims_query = claims::query(&context, &question)?.into_bytes();
    claims_query.push(b'\n');
    Ok((pretty(&context)?, claims_query))
}

/// Writes claims/context.json and claims/query.json from a bundle already prepared and verified.
pub fn write(catalog: &Catalog, turn: &Path, policy: Policy, project_instructions: Vec<String>, allow_patch: bool) -> Result<()> {
    let rules = Rules { strict_text: true, precise_location: true, allow_patch };
    let (context, query) = derive(catalog, turn, policy, project_instructions, rules)?;
    let dir = turn.join(DIR);
    fs::create_dir(&dir)?;
    fs::write(dir.join("context.json"), context)?;
    fs::write(dir.join("query.json"), query)?;
    Ok(())
}

/// Re-derives both files with the recorded policy and instructions; any differing byte is refused.
/// Call after `prepare::verify`, which vouches for the bundle this derives from.
pub fn verify(catalog: &Catalog, turn: &Path) -> Result<Context> {
    let dir = turn.join(DIR);
    let recorded: Context = serde_json::from_slice(&fs::read(dir.join("context.json"))?)?;
    let policy = Policy { allow_inference: recorded.policy.allow_inference, allow_hypothesis: recorded.policy.allow_hypothesis };
    let (context, query) = derive(catalog, turn, policy, recorded.project_instructions.clone(), Rules { strict_text: recorded.strict_text, precise_location: recorded.precise_location, allow_patch: recorded.allow_patch })?;
    if fs::read(dir.join("context.json"))? != context || fs::read(dir.join("query.json"))? != query {
        return Err("Contexto ou query de claims difere do bundle conferido; nada foi enviado".into());
    }
    Ok(recorded)
}

/// Checks one answer and writes check.json next to it. One Markdown fence around the JSON is
/// accepted and recorded as `extracted_from_fence`; the raw answer is kept by the send itself.
pub fn check(context: &Context, send: &Path, answer: &str) -> Result<Value> {
    let fenced = claims::extract_fenced(answer);
    let body = fenced.unwrap_or(answer);
    let report = claims::check(context, body)?;
    let parsed: Option<claims::Response> = serde_json::from_str(body).ok();
    let value = json!({
        "response_sha256": hash(answer.as_bytes()),
        "extracted_from_fence": fenced.is_some(),
        "report": report,
        "claims": parsed.map(|r| r.claims),
        "limits": LIMITS,
    });
    fs::write(send.join("check.json"), pretty(&value)?)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{annotation, identifier_column};

    #[test]
    fn attributes_comments_and_decorators_are_not_declarations() {
        for line in ["    #[inline]", "/// ray_from_pixel builds a ray", "  // ray_from_pixel", " * ray_from_pixel",
            "@lru_cache  # dijkstra", "# dijkstra helper", "/* dijkstra */"] {
            assert!(annotation(line), "{line}");
        }
        for line in ["    pub fn ray_from_pixel(&self) {", "def dijkstra(graph):", "class Graph:", "async def run():"] {
            assert!(!annotation(line), "{line}");
        }
    }

    #[test]
    fn identifier_column_counts_unicode_scalars_and_whole_words() {
        assert_eq!(identifier_column("    pub fn ray_from_pixel(&self)", "ray_from_pixel"), Some(12));
        assert_eq!(identifier_column("def dijkstra(graph):", "dijkstra"), Some(5));
        assert_eq!(identifier_column("def dijkstra_2(g):", "dijkstra"), None);
        assert_eq!(identifier_column("# câmera\tfn câmera()", "câmera"), Some(3));
        assert_eq!(identifier_column("x", "longer"), None);
    }
}
