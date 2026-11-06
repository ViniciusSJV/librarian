//! "Medir tudo": the project says how it counts a passage through a function (one statement,
//! in the profile's `instrument`), and the Librarian decides where: at the start of every Rust
//! function body of the edition, in a copy. The statement goes right after the opening brace,
//! on the same line, so no line of the file moves and each count's `at=` is the line where the
//! function's body opens. Nobody has to know beforehand where the time goes.
use librarian_ingest::{Catalog, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// The profile's `instrument`: the statement, the approved command that prints the counts, path
/// prefixes left out (binaries, the counting module itself) and the paths a proposed change may
/// not touch, because they are part of the measure.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    pub statement: String,
    pub command: String,
    #[serde(default)]
    pub skip: Vec<String>,
    /// Read by the gate (execution.rs), which also takes a profile "Medir tudo" could not use.
    #[serde(default)]
    #[allow(dead_code)]
    pub protect: Vec<String>,
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
    /// Each function that did not get the statement, with the reason.
    pub excluded: Vec<Value>,
}

pub(crate) const KINDS: [&str; 3] = ["function", "method", "trait_method"];
/// The one language whose function bodies this module knows how to open.
const LANGUAGE: &str = "rust";
/// A function with no reason in the record: a measure from before they were kept.
const NOT_RECORDED: &str = "not_recorded";
/// A function with no reason in a record that keeps them: it could not be classified. A loaded
/// edition always gives a function its source and its chunk, so this is not expected to appear.
const NOT_CLASSIFIED: &str = "not_classified";
/// A function that got the statement on a source line another one also got it, with passes counted
/// on that line: they may be either's.
const SHARED_LINE: &str = "shared_line";

const NO_COVERAGE_LIMIT: &str = "Sem cobertura quer dizer que esta medição não observou a função; não quer dizer que ela não rodou.";
const NO_PASSES_LIMIT: &str = "Contada e sem passagens quer dizer que esta medição não observou passagens positivas, não que ela não rodou: o programa pode zerar as contagens durante a execução, ou não imprimir uma linha.";

/// The reason of a function with none, by whether its record keeps reasons at all.
fn missing_reason(recorded: bool) -> &'static str {
    if recorded { NOT_CLASSIFIED } else { NOT_RECORDED }
}

/// Whether the edition has a source the statement can be placed in.
pub fn can_place(catalog: &Catalog) -> bool {
    catalog.sources().values().any(|s| s.language == LANGUAGE)
}

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
    let mut excluded: Vec<Value> = vec![];
    for symbol in catalog.symbols().values().filter(|s| KINDS.contains(&s.kind.as_str())) {
        let Some(source) = catalog.sources().get(&symbol.source_id) else {
            excluded.push(json!({ "symbol": symbol.id, "reason": NOT_CLASSIFIED }));
            continue;
        };
        if source.language != LANGUAGE {
            excluded.push(json!({ "symbol": symbol.id, "reason": "language_not_supported" }));
            continue;
        }
        let (Some(chunk), Some(bytes)) = (catalog.chunk(&symbol.chunk_id), catalog.source_bytes(&source.path)) else {
            excluded.push(json!({ "symbol": symbol.id, "reason": NOT_CLASSIFIED }));
            continue;
        };
        // The label the totals in `skipped` have always used, and the reason kept per function.
        let mut leave = |label: &'static str, reason: &str| {
            *skipped.entry(label).or_default() += 1;
            excluded.push(json!({ "symbol": symbol.id, "reason": reason }));
        };
        if symbol.is_test {
            leave("teste", "test");
            continue;
        }
        if instrument.skip.iter().any(|p| source.path.starts_with(p.as_str())) {
            leave("caminho deixado de fora", "skipped_path");
            continue;
        }
        let text = &bytes[chunk.start_byte..chunk.end_byte.min(bytes.len())];
        let Some(brace) = body_brace(text) else {
            leave("sem corpo", "no_body");
            continue;
        };
        let head = String::from_utf8_lossy(&text[..brace]);
        if head.split_whitespace().collect::<Vec<_>>().windows(2).any(|w| w == ["const", "fn"]) {
            leave("const fn", "const_fn");
            continue;
        }
        if String::from_utf8_lossy(&text[brace..]).contains(marker) {
            leave("já conta", "already_counts");
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
    Ok(Plan { diff, functions, skipped, excluded })
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

/// The order of a measure's lines: most calls first. Among equal counts, the most instrumented
/// time and then the place, so that a tie always falls the same way. A missing number comes last.
fn by_calls(a: &Value, b: &Value) -> Ordering {
    let number = |row: &Value, field: &str| row[field].as_f64().unwrap_or(f64::NEG_INFINITY);
    number(b, "calls").total_cmp(&number(a, "calls"))
        .then(number(b, "cpu_ms").total_cmp(&number(a, "cpu_ms")))
        .then_with(|| a["at"].as_str().cmp(&b["at"].as_str()))
}

/// What a line's count can be said to be, from what the record keeps. The statement goes right after
/// the brace that opens the body, on the line the plan records, so a count printed for that line and
/// that function alone is its calls. A line off the plan did not get the statement: the project's own
/// counter, wherever it is in the function. Several functions on one line share one series, and a
/// record with no usable plan cannot tell: neither says more than that a count was observed.
fn count_kind(row: &Value, planned: Option<&BTreeMap<String, Vec<String>>>, lines: usize) -> &'static str {
    let (Some(symbol), Some(at)) = (row["symbol"].as_str(), row["at"].as_str()) else { return "unknown" };
    if lines > 1 {
        return "several_lines";
    }
    match planned.map(|p| p.get(at)) {
        None => "unknown",
        Some(None) => "own_counter",
        Some(Some(functions)) if functions.len() == 1 && functions[0] == symbol => "entry_calls",
        Some(Some(_)) => "unknown",
    }
}

/// Each planned line with the functions that got the statement on it; none if the record has no plan
/// it can be read from.
fn planned_lines(execution: &Value) -> Option<BTreeMap<String, Vec<String>>> {
    let mut planned: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for f in execution["instrumentation"]["functions"].as_array()? {
        planned.entry(format!("{}:{}", f["path"].as_str()?, f["line"].as_u64()?)).or_default().push(f["symbol"].as_str()?.to_owned());
    }
    Some(planned)
}

/// One number per function: its largest count when it has several lines. That is a fact about the
/// lines, and it says whether the function ran. It is not how many times the function was entered:
/// a line counts the passes through it, and the record does not say where in the function it is.
/// Adding them up would count one call once per line it went through.
pub fn calls_by_symbol(rows: &[Value]) -> BTreeMap<String, f64> {
    let mut calls: BTreeMap<String, f64> = BTreeMap::new();
    for row in rows {
        let (Some(symbol), Some(count)) = (row["symbol"].as_str(), row["calls"].as_f64()) else { continue };
        let most = calls.entry(symbol.to_owned()).or_insert(0.);
        *most = most.max(count);
    }
    calls
}

/// "Onde pesa": every counted line of an instrumented run, with its function, calls and time,
/// most calls first. The counts are exact; the time is inclusive and inflated by the counting
/// itself, which can change the order between functions, so it does not decide the order.
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
                "at": at, "calls": s["median"], "calls_max": s["max"], "cpu_ms": stats[format!("cpu_ms@{at}")]["median"],
                "symbol": found.map(|f| f.id.clone()), "name": found.map(|f| f.qualified_name.clone()),
                "path": path, "start_line": chunk.map(|c| c.start_line), "end_line": chunk.map(|c| c.end_line),
            })
        })
        .collect();
    rows.sort_by(by_calls);
    let planned = planned_lines(execution);
    let kinds: Vec<&str> = rows.iter().map(|r| {
        let lines = rows.iter().filter(|o| r["symbol"].is_string() && o["symbol"] == r["symbol"]).count();
        count_kind(r, planned.as_ref(), lines)
    }).collect();
    for (row, kind) in rows.iter_mut().zip(kinds) {
        row["count_kind"] = json!(kind);
    }
    // Who calls each function, by the written name, with what the measure counted for that
    // caller: where a function's calls come from, without reading the code.
    let graph = librarian_ingest::graph::v2::Graph::build(catalog);
    let syntax = catalog.call_syntax();
    let counted = calls_by_symbol(&rows);
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
    // `body_brace` reads one language's syntax: for any other it cannot say a body is missing.
    let bodiless: Vec<&str> = catalog.symbols().values().filter(|s| KINDS.contains(&s.kind.as_str()))
        .filter(|s| catalog.sources().get(&s.source_id).is_some_and(|src| src.language == LANGUAGE))
        .filter(|s| catalog.chunk(&s.chunk_id).zip(catalog.sources().get(&s.source_id).and_then(|src| catalog.source_bytes(&src.path)))
            .is_some_and(|(c, bytes)| body_brace(&bytes[c.start_byte..c.end_byte.min(bytes.len())]).is_none()))
        .map(|s| s.id.as_str()).collect();
    let coverage = coverage(catalog, execution, &rows);
    let plain: serde_json::Map<String, Value> = stats.as_object().into_iter().flatten()
        .filter(|(_, s)| s["at"].is_null()).map(|(k, s)| (k.clone(), s["median"].clone())).collect();
    json!({
        "execution": execution["id"], "started": execution["started"], "command": execution["name"],
        "functions": execution["instrumentation"]["functions"].as_array().map_or(0, |f| f.len()),
        // Every function that got the count: one of them missing from `rows` printed no line.
        "counted": execution["instrumentation"]["functions"].as_array().into_iter().flatten().filter_map(|f| f["symbol"].as_str()).collect::<Vec<_>>(),
        "bodiless": bodiless,
        "coverage": coverage,
        "skipped": execution["instrumentation"]["skipped"], "measures": plain, "rows": rows,
        "limits": ["A contagem é exata e é ela que ordena a lista: cada número é o que esta medição observou naquela linha. Na linha da instrução que o Medir tudo pôs na entrada da função, é o número de chamadas observadas, a frequência observada neste comando; nos outros casos, não se pode dizer tanto.",
            "Uma função com mais de uma linha contada aparece uma vez por linha; onde ela tem um número só, é a maior contagem, que pode não ser quantas vezes ela foi chamada.",
            "O tempo é tempo instrumentado, não o custo próprio da função: é inclusivo (conta o que a função chama), somado das threads e inflado pela própria contagem, o que pode mudar a ordem entre as funções. Serve de referência, e só desempata contagens iguais.",
            NO_COVERAGE_LIMIT,
            NO_PASSES_LIMIT,
            "Cada linha é onde o corpo da função abre, ou o contador que o projeto já tinha nela; nesse caso, conta as passagens por aquela linha.",
            "Quem chama é pelo nome escrito, com a contagem observada daquele chamador; com homônimos, a ligação é candidata."],
    })
}

#[derive(Default, Serialize)]
struct Tally {
    total: usize,
    counted: usize,
    counted_and_ran: usize,
    counted_no_passes: usize,
    no_coverage: usize,
}

impl Tally {
    fn add(&mut self, state: &str) {
        self.total += 1;
        match state {
            "counted_and_ran" => { self.counted += 1; self.counted_and_ran += 1 }
            "counted_no_passes" => { self.counted += 1; self.counted_no_passes += 1 }
            _ => self.no_coverage += 1,
        }
    }
}

/// What a zero says, by the origin of its line: `count_kind` of the function's first line.
fn zero_of(rows: &[Value], id: &str) -> &'static str {
    match rows.iter().find(|r| r["symbol"] == id).and_then(|r| r["count_kind"].as_str()) {
        Some("entry_calls") => "entry_zero",
        Some("own_counter") => "own_zero",
        Some("several_lines") => "several_zero",
        _ => "unknown_zero",
    }
}

/// The printed line a function that got the statement shares with another function on the same
/// source line: one series, kept under one of them, and the record cannot split it.
fn shared_series<'a>(planned: Option<&BTreeMap<String, Vec<String>>>, rows: &'a [Value], id: &str) -> Option<&'a Value> {
    planned?.iter().filter(|(_, functions)| functions.len() > 1 && functions.iter().any(|f| f == id))
        .find_map(|(at, _)| rows.iter().find(|r| r["at"] == at.as_str()))
}

/// What a measure observed about each function of the edition, from the record alone. A printed
/// count above zero, in any repetition and in any language, is a pass. No pass observed is not "did
/// not run": a program can zero its counts while it runs, or not print a line; `zero_evidence` says
/// which case it is. About the others the measure says nothing, and only then there is a reason.
/// `instrumented` is what got the statement, the same as `functions` and the list `counted` in the
/// answer around it. The number `counted` here is every function with a count: those, plus what
/// printed one without getting the statement. So it can be larger, and they are not interchangeable.
fn coverage(catalog: &Catalog, execution: &Value, rows: &[Value]) -> Value {
    let instrumentation = &execution["instrumentation"];
    let received: BTreeSet<&str> = instrumentation["functions"].as_array().into_iter().flatten().filter_map(|f| f["symbol"].as_str()).collect();
    // Absent is an old record; a list without a function is a new record that could not classify it.
    let reasons: Option<BTreeMap<&str, &str>> = instrumentation["excluded"].as_array().map(|list| list.iter()
        .filter_map(|e| Some((e["symbol"].as_str()?, e["reason"].as_str()?))).collect());
    let calls = calls_by_symbol(rows);
    // Any observation above zero proves a pass: the median of the repetitions can be zero with one
    // above it. The largest says only that; it is not shown as the function's count.
    let passed: BTreeSet<&str> = rows.iter().filter(|r| r["calls"].as_f64().unwrap_or(0.) > 0. || r["calls_max"].as_f64().unwrap_or(0.) > 0.)
        .filter_map(|r| r["symbol"].as_str()).collect();
    let planned = planned_lines(execution);
    let mut zeros = BTreeMap::<&str, usize>::new();
    let (mut all, mut by_language, mut excluded, mut functions) = (Tally::default(), BTreeMap::<&str, Tally>::new(), BTreeMap::<&str, usize>::new(), vec![]);
    for symbol in catalog.symbols().values().filter(|s| KINDS.contains(&s.kind.as_str())) {
        let source = catalog.sources().get(&symbol.source_id);
        let id = symbol.id.as_str();
        let (state, reason, zero) = match calls.get(id) {
            _ if passed.contains(id) => ("counted_and_ran", None, None),
            Some(_) => ("counted_no_passes", None, Some(zero_of(rows, id))),
            None if received.contains(id) => match shared_series(planned.as_ref(), rows, id) {
                // Passes on a line it shares: whose they are, the record cannot say.
                Some(row) if row["calls"].as_f64().unwrap_or(0.) > 0. || row["calls_max"].as_f64().unwrap_or(0.) > 0. => ("no_coverage", Some(SHARED_LINE), None),
                Some(_) => ("counted_no_passes", None, Some("unknown_zero")),
                None => ("counted_no_passes", None, Some("no_line")),
            },
            None => ("no_coverage", Some(reasons.as_ref().and_then(|r| r.get(id).copied()).unwrap_or(missing_reason(reasons.is_some()))), None),
        };
        if let Some(zero) = zero {
            *zeros.entry(zero).or_default() += 1;
        }
        all.add(state);
        by_language.entry(source.map_or("unknown", |s| s.language.as_str())).or_default().add(state);
        if let Some(reason) = reason {
            *excluded.entry(reason).or_default() += 1;
        }
        functions.push(json!({ "symbol": id, "name": symbol.qualified_name, "path": source.map(|s| &s.path), "language": source.map(|s| &s.language),
            "coverage_state": state, "exclusion_reason": reason, "zero_evidence": zero }));
    }
    let mut limits = vec![NO_COVERAGE_LIMIT, NO_PASSES_LIMIT];
    if excluded.contains_key(NOT_RECORDED) {
        limits.push("Esta medição é anterior ao registro por função: o motivo de quem ficou de fora não foi guardado.");
    }
    if excluded.contains_key(NOT_CLASSIFIED) {
        limits.push("Há funções sem classificação nesta medição: o registro guarda os motivos, mas não o delas.");
    }
    if all.counted > received.len() {
        limits.push("Contadas inclui quem imprimiu uma contagem sem ter recebido a instrução; por isso o número pode passar o de instrumentadas.");
    }
    let mut out = json!(all);
    out["instrumented"] = json!(received.len());
    out["reasons_recorded"] = json!(reasons.is_some());
    out["partial"] = json!(all.no_coverage > 0);
    out["excluded"] = json!(excluded);
    out["zero_evidence"] = json!(zeros);
    out["by_language"] = json!(by_language);
    out["functions"] = json!(functions);
    out["limits"] = json!(limits);
    out
}

/// A suggested envelope with what `coverage` says, on each function it mentions and in a summary.
/// With no measure (`None`) a function has no coverage and nothing was left out of anything; what
/// is not a function has no such state.
pub fn annotate_suggestion(suggestion: &mut Value, coverage: Option<&Value>) {
    let functions: BTreeMap<&str, &Value> = coverage.and_then(|c| c["functions"].as_array()).into_iter().flatten()
        .filter_map(|f| Some((f["symbol"].as_str()?, f))).collect();
    for list in ["items", "left_out"] {
        for entry in suggestion[list].as_array_mut().into_iter().flatten() {
            let (state, reason, zero) = match functions.get(entry["id"].as_str().unwrap_or_default()) {
                Some(f) => (f["coverage_state"].clone(), f["exclusion_reason"].clone(), f["zero_evidence"].clone()),
                None if KINDS.contains(&entry["kind"].as_str().unwrap_or_default()) => (json!("no_coverage"), coverage.map_or(Value::Null, |c| json!(missing_reason(c["reasons_recorded"] == true))), Value::Null),
                None => (Value::Null, Value::Null, Value::Null),
            };
            entry["coverage_state"] = state;
            entry["exclusion_reason"] = reason;
            entry["zero_evidence"] = zero;
        }
    }
    let mut summary = coverage.cloned().unwrap_or(Value::Null);
    if let Some(fields) = summary.as_object_mut() {
        fields.remove("functions");
    }
    suggestion["coverage"] = summary;
}

#[cfg(test)]
mod tests {
    use super::{annotate_suggestion, body_brace, by_calls, calls_by_symbol, count_kind, planned_lines};
    use serde_json::json;

    /// The calls decide, whatever the time says; a tie falls the same way from any starting order.
    #[test]
    fn lines_are_ordered_by_calls_with_a_fixed_tie_break() {
        let row = |at: &str, calls: f64, cpu: serde_json::Value| json!({ "at": at, "calls": calls, "cpu_ms": cpu });
        let lines = [
            row("a.rs:1", 10., json!(9000.)), // Few calls and the most time.
            row("a.rs:2", 500., json!(1.)),   // The most calls and little time.
            row("a.rs:4", 50., json!(7.)),    // A tie on calls and on time: the place decides.
            row("a.rs:3", 50., json!(7.)),
            row("a.rs:5", 50., json!(8.)),    // A tie on calls: the time decides.
            row("a.rs:6", 50., serde_json::Value::Null), // No time recorded: not the same as zero, but after it.
            row("a.rs:7", 50., json!(0.)),
        ];
        let expected = ["a.rs:2", "a.rs:5", "a.rs:3", "a.rs:4", "a.rs:7", "a.rs:6", "a.rs:1"];
        let sorted = |mut rows: Vec<serde_json::Value>| {
            rows.sort_by(by_calls);
            rows.iter().map(|r| r["at"].as_str().unwrap().to_owned()).collect::<Vec<_>>()
        };
        assert_eq!(sorted(lines.to_vec()), expected);
        let mut backwards = lines.to_vec();
        backwards.reverse();
        assert_eq!(sorted(backwards), expected);
    }

    /// The origin decides, not how many lines there are: one line off the plan is not a function's calls.
    #[test]
    fn a_count_is_read_by_where_it_came_from() {
        let record = json!({ "instrumentation": { "functions": [
            { "symbol": "a", "path": "x.rs", "line": 1 }, { "symbol": "b", "path": "x.rs", "line": 5 }, { "symbol": "c", "path": "x.rs", "line": 5 },
        ] } });
        let planned = planned_lines(&record);
        let kind = |symbol: &str, at: &str, lines: usize| count_kind(&json!({ "symbol": symbol, "at": at }), planned.as_ref(), lines);
        assert_eq!(kind("a", "x.rs:1", 1), "entry_calls");
        assert_eq!(kind("a", "x.rs:2", 1), "own_counter");
        assert_eq!(kind("d", "x.rs:9", 1), "own_counter");
        // The planned line, but of another function: nothing says whose count it is.
        assert_eq!(kind("d", "x.rs:1", 1), "unknown");
        // Two functions on one line share one series.
        assert_eq!(kind("b", "x.rs:5", 1), "unknown");
        assert_eq!(kind("a", "x.rs:1", 2), "several_lines");
        assert_eq!(count_kind(&json!({ "symbol": null, "at": "x.rs:1" }), planned.as_ref(), 1), "unknown");
        // A record with no plan, or one it cannot read, proves nothing either way.
        assert_eq!(count_kind(&json!({ "symbol": "a", "at": "x.rs:1" }), planned_lines(&json!({})).as_ref(), 1), "unknown");
        let broken = json!({ "instrumentation": { "functions": [{ "symbol": "a", "path": "x.rs" }] } });
        assert_eq!(count_kind(&json!({ "symbol": "a", "at": "x.rs:1" }), planned_lines(&broken).as_ref(), 1), "unknown");
    }

    /// What the largest count means, by where the lines are. It is right in the first case only;
    /// the record cannot tell the cases apart, so this describes the rule and does not defend it.
    #[test]
    fn the_largest_count_is_not_always_the_times_a_function_was_entered() {
        let largest = |lines: &[f64]| {
            let rows: Vec<_> = lines.iter().map(|c| json!({ "symbol": "f", "calls": c })).collect();
            calls_by_symbol(&rows)["f"]
        };
        // Entered 100 times, a line at the entry and one in a branch: right. Adding gives 130.
        assert_eq!(largest(&[100., 30.]), 100.);
        // Entered 100 times, a line at the entry and one in a loop of 5: says 500. Adding gives 600.
        assert_eq!(largest(&[100., 500.]), 500.);
        // Entered 100 times, one line in each of two exclusive branches: says 60. Adding gives the 100.
        assert_eq!(largest(&[60., 40.]), 60.);
        // Whatever the lines are, one above zero means it ran.
        assert!(largest(&[0., 7.]) > 0.);
    }

    /// A function with two counted lines keeps the larger count, wherever the order puts it.
    #[test]
    fn a_function_with_several_lines_keeps_its_largest_count() {
        let rows = |first: f64, second: f64| vec![json!({ "symbol": "f", "calls": first }), json!({ "symbol": "f", "calls": second }), json!({ "symbol": null, "calls": 99. })];
        assert_eq!(calls_by_symbol(&rows(7., 2.))["f"], 7.);
        assert_eq!(calls_by_symbol(&rows(2., 7.))["f"], 7.);
        assert_eq!(calls_by_symbol(&rows(7., 2.)).len(), 1);
    }

    /// A function the measure's list does not have: the reason depends on the kind of record,
    /// as it does in `coverage`, and with no measure there is none.
    #[test]
    fn a_function_missing_from_the_coverage_follows_the_kind_of_record() {
        let annotated = |coverage: Option<serde_json::Value>| {
            let mut suggestion = json!({ "items": [{ "id": "x", "kind": "function" }, { "id": "y", "kind": "struct" }], "left_out": [] });
            annotate_suggestion(&mut suggestion, coverage.as_ref());
            assert_eq!(suggestion["items"][0]["coverage_state"], "no_coverage");
            assert_eq!(suggestion["items"][1]["coverage_state"], serde_json::Value::Null);
            assert_eq!(suggestion["items"][1]["exclusion_reason"], serde_json::Value::Null);
            suggestion["items"][0]["exclusion_reason"].clone()
        };
        assert_eq!(annotated(Some(json!({ "functions": [], "reasons_recorded": false }))), "not_recorded");
        assert_eq!(annotated(Some(json!({ "functions": [], "reasons_recorded": true }))), "not_classified");
        assert_eq!(annotated(None), serde_json::Value::Null);
    }

    #[test]
    fn finds_the_body_brace_past_comments_strings_and_brackets() {
        assert_eq!(body_brace(b"fn a(x: [u8; 2]) -> f64 { 1. }"), Some(24));
        assert_eq!(body_brace(b"/// a { in a comment\nfn a() {}"), Some(28));
        assert_eq!(body_brace(b"#[doc = \"{\"]\nfn a() {}"), Some(20));
        assert_eq!(body_brace(b"fn a<'b>(c: &'b str) where Self: Sized {"), Some(39));
        assert_eq!(body_brace(b"fn a(&self) -> Intersections;"), None);
    }
}
