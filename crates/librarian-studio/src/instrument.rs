//! "Medir tudo": the project says how it counts a passage through a function (one statement,
//! in the profile's `instrument`), and the Librarian decides where: at the start of every Rust
//! function body of the edition, in a copy. The statement goes right after the opening brace,
//! on the same line, so no line of the file moves and each count's `at=` is the line where the
//! function's body opens. Nobody has to know beforehand where the time goes.
use librarian_ingest::{Catalog, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The profile's `instrument`: the statement, the approved command that prints the counts and
/// path prefixes left out (binaries, the counting module itself).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    pub statement: String,
    pub command: String,
    #[serde(default)]
    pub skip: Vec<String>,
}

impl Instrument {
    pub fn check(&self) -> Result<()> {
        let s = self.statement.trim();
        if s.is_empty() || s.contains('\n') || s.contains('{') || s.contains('}') || !s.ends_with(';') {
            return Err("instrument.statement precisa ser uma instrução de uma linha, terminada em ';', sem chaves".into());
        }
        Ok(())
    }
}

/// The diff that puts the statement in every function, and what was left out and why.
pub struct Plan {
    pub diff: String,
    pub functions: Vec<Value>,
    pub skipped: BTreeMap<&'static str, usize>,
}

const KINDS: [&str; 3] = ["function", "method", "trait_method"];

/// Where a function's body opens, from the start of its declaration: the first `{` outside
/// parentheses, brackets, comments, strings and character literals. A `;` first means no body.
fn body_brace(text: &[u8]) -> Option<usize> {
    let (mut depth, mut i) = (0i32, 0usize);
    while i < text.len() {
        match text[i] {
            b'/' if text.get(i + 1) == Some(&b'/') => {
                while i < text.len() && text[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if text.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < text.len() && !(text[i] == b'*' && text[i + 1] == b'/') {
                    i += 1;
                }
                i += 1;
            }
            b'"' => {
                i += 1;
                while i < text.len() && text[i] != b'"' {
                    if text[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            // A character literal ('{'), not a lifetime ('a): the quote closes two or three bytes on.
            b'\'' if text.get(i + 2) == Some(&b'\'') => i += 2,
            b'\'' if text.get(i + 1) == Some(&b'\\') && text.get(i + 3) == Some(&b'\'') => i += 3,
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b';' if depth == 0 => return None,
            b'{' if depth == 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

pub fn plan(catalog: &Catalog, instrument: &Instrument) -> Result<Plan> {
    instrument.check()?;
    let statement = instrument.statement.trim();
    let marker = statement.trim_end_matches(';').trim();
    // Insertions per file: byte offset of the brace, and the function.
    let mut inserts: BTreeMap<String, Vec<(usize, Value)>> = BTreeMap::new();
    let mut skipped: BTreeMap<&'static str, usize> = BTreeMap::new();
    for symbol in catalog.symbols().values().filter(|s| KINDS.contains(&s.kind.as_str())) {
        let Some(source) = catalog.sources().get(&symbol.source_id) else { continue };
        if source.language != "rust" {
            continue;
        }
        let why = if symbol.is_test {
            Some("teste")
        } else if instrument.skip.iter().any(|p| source.path.starts_with(p.as_str())) {
            Some("caminho deixado de fora")
        } else {
            None
        };
        if let Some(why) = why {
            *skipped.entry(why).or_default() += 1;
            continue;
        }
        let (Some(chunk), Some(bytes)) = (catalog.chunk(&symbol.chunk_id), catalog.source_bytes(&source.path)) else { continue };
        let text = &bytes[chunk.start_byte..chunk.end_byte.min(bytes.len())];
        let Some(brace) = body_brace(text) else {
            *skipped.entry("sem corpo").or_default() += 1;
            continue;
        };
        let head = String::from_utf8_lossy(&text[..brace]);
        if head.split_whitespace().collect::<Vec<_>>().windows(2).any(|w| w == ["const", "fn"]) {
            *skipped.entry("const fn").or_default() += 1;
            continue;
        }
        if String::from_utf8_lossy(&text[brace..]).contains(marker) {
            *skipped.entry("já conta").or_default() += 1;
            continue;
        }
        let at = chunk.start_byte + brace;
        let line = bytes[..at].iter().filter(|&&b| b == b'\n').count() + 1;
        inserts.entry(source.path.clone()).or_default().push((at, json!({
            "symbol": symbol.id, "name": symbol.qualified_name, "path": source.path, "line": line,
        })));
    }
    let mut diff = String::new();
    let mut functions = vec![];
    for (path, mut found) in inserts {
        let bytes = catalog.source_bytes(&path).ok_or("fonte sumiu da edition")?;
        found.sort_by_key(|(at, _)| *at);
        diff.push_str(&format!("--- a/{path}\n+++ b/{path}\n"));
        // One hunk per line; several braces on one line all get the statement.
        let mut by_line: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (at, f) in &found {
            by_line.entry(f["line"].as_u64().unwrap() as usize).or_default().push(*at);
        }
        for (line, ats) in by_line {
            let start = bytes[..ats[0]].iter().rposition(|&b| b == b'\n').map_or(0, |p| p + 1);
            let end = bytes[ats[0]..].iter().position(|&b| b == b'\n').map_or(bytes.len(), |p| ats[0] + p);
            let old = String::from_utf8_lossy(&bytes[start..end]).trim_end_matches('\r').to_owned();
            let mut new = String::new();
            let mut from = start;
            for at in &ats {
                new.push_str(&String::from_utf8_lossy(&bytes[from..=*at]));
                new.push_str(&format!(" {statement}"));
                from = at + 1;
            }
            new.push_str(&String::from_utf8_lossy(&bytes[from..end]));
            let new = new.trim_end_matches('\r');
            diff.push_str(&format!("@@ -{line},1 +{line},1 @@\n-{old}\n+{new}\n"));
        }
        functions.extend(found.into_iter().map(|(_, f)| f));
    }
    if functions.is_empty() {
        return Err("Nenhuma função para contar nesta edition".into());
    }
    Ok(Plan { diff, functions, skipped })
}

/// The function that holds a counted line: the innermost one whose lines contain it.
pub fn holder<'a>(catalog: &'a Catalog, path: &str, line: usize) -> Option<&'a librarian_ingest::Symbol> {
    catalog.symbols().values()
        .filter(|s| KINDS.contains(&s.kind.as_str()) && catalog.sources().get(&s.source_id).is_some_and(|src| src.path == path))
        .filter_map(|s| catalog.chunk(&s.chunk_id).map(|c| (s, c)))
        .filter(|(_, c)| c.start_line <= line && line <= c.end_line)
        .min_by_key(|(_, c)| c.end_line - c.start_line)
        .map(|(s, _)| s)
}

/// "Onde pesa": every counted line of an instrumented run, with its function, calls and time,
/// heaviest first. The counts are exact; the time is inclusive and inflated by the counting
/// itself, so the order is what it tells.
pub fn hotspots(catalog: &Catalog, execution: &Value) -> Value {
    let stats = &execution["summary"]["stats"];
    let mut rows: Vec<Value> = stats.as_object().into_iter().flatten()
        .filter(|(_, s)| s["measure"] == "calls" && s["at"].is_string())
        .map(|(_, s)| {
            let at = s["at"].as_str().unwrap();
            let (path, line) = at.rsplit_once(':').map(|(p, l)| (p, l.parse::<usize>().unwrap_or(0))).unwrap_or((at, 0));
            let found = holder(catalog, path, line);
            let chunk = found.and_then(|f| catalog.chunk(&f.chunk_id));
            json!({
                "at": at, "calls": s["median"], "cpu_ms": stats[format!("cpu_ms@{at}")]["median"],
                "symbol": found.map(|f| f.id.clone()), "name": found.map(|f| f.qualified_name.clone()),
                "path": path, "start_line": chunk.map(|c| c.start_line), "end_line": chunk.map(|c| c.end_line),
            })
        })
        .collect();
    rows.sort_by(|a, b| b["cpu_ms"].as_f64().unwrap_or(0.).total_cmp(&a["cpu_ms"].as_f64().unwrap_or(0.)));
    // Who calls each function, by the written name, with what the measure counted for that
    // caller: where a frequent function's calls come from, without reading the code.
    let graph = librarian_ingest::graph::v2::Graph::build(catalog);
    let syntax = catalog.call_syntax();
    let counted: BTreeMap<String, f64> = rows.iter().filter_map(|r| Some((r["symbol"].as_str()?.to_owned(), r["calls"].as_f64()?))).collect();
    for row in rows.iter_mut() {
        let Some(id) = row["symbol"].as_str().map(String::from) else { continue };
        let Ok(used) = graph.used_by(&id, 1, 64) else { continue };
        let mut callers: Vec<Value> = used.links.iter()
            .filter(|l| l.to == id && l.from != id)
            .filter_map(|l| counted.get(&l.from).map(|calls| (l, *calls)))
            .map(|(l, calls)| json!({ "symbol": l.from, "name": catalog.symbols().get(&l.from).map(|s| s.qualified_name.clone()), "calls": calls,
                "line": l.evidence.start_line, "homonyms": l.candidates > 1 }))
            .collect();
        callers.sort_by(|a, b| b["calls"].as_f64().unwrap_or(0.).total_cmp(&a["calls"].as_f64().unwrap_or(0.)));
        callers.dedup_by(|a, b| a["symbol"] == b["symbol"]);
        row["callers"] = json!(callers);
        // The calls written in its body that reach a counted function, one per written call
        // (a name with homonyms lists the counted ones): the small pieces a question can ask about.
        let Ok(calls) = graph.calls_from(&id, 1, 64, &syntax, &BTreeMap::new()) else { continue };
        let mut by_occurrence: BTreeMap<&str, (usize, &str, Vec<Value>)> = BTreeMap::new();
        for l in calls.links.iter().filter(|l| l.from == id && l.to != id) {
            let Some(calls) = counted.get(&l.to) else { continue };
            let entry = by_occurrence.entry(&l.occurrence).or_insert((l.evidence.start_line, &l.text, vec![]));
            entry.2.push(json!({ "symbol": l.to, "name": catalog.symbols().get(&l.to).map(|s| s.qualified_name.clone()), "calls": calls }));
        }
        let mut out: Vec<Value> = by_occurrence.into_values()
            .map(|(line, text, targets)| json!({ "line": line, "text": text, "targets": targets })).collect();
        out.sort_by_key(|c| c["line"].as_u64());
        row["calls_out"] = json!(out);
    }
    // Declarations without a body (a trait's signature): a call reaches one of its impls, never them.
    let bodiless: Vec<&str> = catalog.symbols().values().filter(|s| KINDS.contains(&s.kind.as_str()))
        .filter(|s| catalog.chunk(&s.chunk_id).zip(catalog.sources().get(&s.source_id).and_then(|src| catalog.source_bytes(&src.path)))
            .is_some_and(|(c, bytes)| body_brace(&bytes[c.start_byte..c.end_byte.min(bytes.len())]).is_none()))
        .map(|s| s.id.as_str()).collect();
    let plain: serde_json::Map<String, Value> = stats.as_object().into_iter().flatten()
        .filter(|(_, s)| s["at"].is_null()).map(|(k, s)| (k.clone(), s["median"].clone())).collect();
    json!({
        "execution": execution["id"], "started": execution["started"], "command": execution["name"],
        "functions": execution["instrumentation"]["functions"].as_array().map_or(0, |f| f.len()),
        // Every function that got the count: one of them missing from `rows` did not run.
        "counted": execution["instrumentation"]["functions"].as_array().into_iter().flatten().filter_map(|f| f["symbol"].as_str()).collect::<Vec<_>>(),
        "bodiless": bodiless,
        "skipped": execution["instrumentation"]["skipped"], "measures": plain, "rows": rows,
        "limits": ["A contagem é exata; o tempo é inclusivo (conta o que a função chama), somado das threads e inflado pela própria contagem: a ordem é o que vale.",
            "Cada linha é onde o corpo da função abre (ou o contador que o projeto já tinha nela).",
            "Quem chama é pelo nome escrito, com as chamadas contadas daquele chamador; com homônimos, a ligação é candidata."],
    })
}

#[cfg(test)]
mod tests {
    use super::body_brace;

    #[test]
    fn finds_the_body_brace_past_comments_strings_and_brackets() {
        assert_eq!(body_brace(b"fn a(x: [u8; 2]) -> f64 { 1. }"), Some(24));
        assert_eq!(body_brace(b"/// a { in a comment\nfn a() {}"), Some(28));
        assert_eq!(body_brace(b"#[doc = \"{\"]\nfn a() {}"), Some(20));
        assert_eq!(body_brace(b"fn a<'b>(c: &'b str) where Self: Sized {"), Some(39));
        assert_eq!(body_brace(b"fn a(&self) -> Intersections;"), None);
    }
}
