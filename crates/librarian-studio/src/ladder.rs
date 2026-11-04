//! The ladder: one question over an envelope, to the lightest model first and to the next only
//! when the gate refuses. A change passes when every gate command succeeds with it, every
//! output is the same as the baseline's, a count or a measured number improves and no count grows. A LACUNA does not
//! climb the ladder: the names it says are missing are looked up in the catalog, gathered, and
//! the same model is asked again. Nothing here judges meaning; every step is recorded.
//!
//! What is measured cannot be what is changed. A command that counts pays for its counting, so it
//! proves counts and never time; a counter the baseline had and the change deleted is refused,
//! not read as a drop; and a change may not touch what the project declares as its measure. This
//! reads the printed series, the diff and the profile, so it holds for any language that prints
//! them, whether or not "Medir tudo" can place the counters there.
use librarian_ingest::Catalog;
use serde_json::{json, Value};

/// Names a LACUNA text asks for: `written in backticks`, `A::b`, `name()` and CamelCase words.
pub fn lacuna_names(text: &str) -> Vec<String> {
    let mut found: Vec<String> = vec![];
    let mut push = |n: &str| {
        let n = n.trim_matches(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':' || c == '<' || c == '>'))
            .trim_end_matches("()").trim_start_matches("self.").to_owned();
        if n.len() > 1 && !found.contains(&n) {
            found.push(n);
        }
    };
    for (k, part) in text.split('`').enumerate() {
        if k % 2 == 1 {
            push(part.split('(').next().unwrap_or(part));
        }
    }
    for word in text.split(|c: char| c.is_whitespace() || ",.;()".contains(c)) {
        let rest = &text[text.find(word).map_or(0, |i| i + word.len())..];
        let camel = word.len() > 2 && word.starts_with(|c: char| c.is_ascii_uppercase())
            && word.chars().any(|c| c.is_ascii_lowercase()) && word.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':');
        if camel || word.contains("::") || (rest.starts_with("()") && word.chars().all(|c| c.is_alphanumeric() || c == '_')) {
            push(word);
        }
    }
    found
}

/// The declarations those names point to, at most `limit`, each with the name that found it.
/// Types first, only those whose methods ran in the measure (`ran`, qualified names; all when
/// nothing was measured): a type (two homonyms at most) with its own `impl` and the `impl` of a
/// trait already gathered and, for an enum, the types of its variants that ran. Then a qualified
/// name exactly, then a method when at most three have the name (a trait's own declaration
/// first). Tests and what is already gathered stay out.
pub fn resolve(catalog: &Catalog, names: &[String], gathered: &[String], ran: &[String], limit: usize) -> Vec<(String, String)> {
    let symbols: Vec<&librarian_ingest::Symbol> = catalog.symbols().values().filter(|s| !s.is_test).collect();
    let mut out: Vec<(String, String)> = vec![];
    let add = |id: &str, name: &str, out: &mut Vec<(String, String)>| {
        if out.len() < limit && !gathered.iter().any(|g| g == id) && !out.iter().any(|(o, _)| o == id) {
            out.push((id.to_owned(), name.to_owned()));
        }
    };
    let types_named = |t: &str| -> Vec<&librarian_ingest::Symbol> {
        symbols.iter().filter(|s| matches!(s.kind.as_str(), "struct" | "enum" | "trait") && s.name == t).copied().collect()
    };
    // The traits of the `impl Trait for T` blocks already gathered: a type brings its impl of those.
    let traits: Vec<String> = gathered.iter().filter_map(|id| catalog.symbols().get(id))
        .filter_map(|s| s.qualified_name.strip_prefix("impl ").and_then(|r| r.split_once(" for ")).map(|(t, _)| t.to_owned())).collect();
    let with_impls = |t: &str, name: &str, out: &mut Vec<(String, String)>| {
        for s in types_named(t) {
            add(&s.id, name, out);
        }
        let impls = symbols.iter().filter(|s| s.kind == "impl"
            && (s.qualified_name == format!("impl {t}") || traits.iter().any(|tr| s.qualified_name == format!("impl {tr} for {t}"))));
        for s in impls {
            add(&s.id, name, out);
        }
    };
    let ran_type = |t: &str| ran.is_empty() || ran.iter().any(|q| q.starts_with(&format!("impl {t}::")) || q.contains(&format!(" for {t}::")));
    let (types, others): (Vec<&String>, Vec<&String>) = names.iter().partition(|n| !n.contains("::") && (1..=2).contains(&types_named(n).len()));
    for name in types.into_iter().filter(|n| ran_type(n)) {
        with_impls(name, name, &mut out);
        for e in types_named(name).into_iter().filter(|s| s.kind == "enum") {
            let prefix = format!("{}::", e.qualified_name);
            let variants: Vec<String> = symbols.iter().filter(|s| s.kind == "variant" && s.qualified_name.starts_with(&prefix)).map(|s| s.name.clone()).collect();
            for v in variants.iter().filter(|v| ran_type(v)) {
                with_impls(v, &format!("{name}::{v}"), &mut out);
            }
        }
    }
    for name in others {
        let last = name.rsplit("::").next().unwrap_or(name);
        let exact: Vec<&&librarian_ingest::Symbol> = symbols.iter().filter(|s| s.qualified_name == *name || s.qualified_name.ends_with(&format!(" {name}"))).collect();
        if !exact.is_empty() && exact.len() <= 2 {
            for s in exact {
                add(&s.id, name, &mut out);
            }
            continue;
        }
        let methods: Vec<&&librarian_ingest::Symbol> = symbols.iter().filter(|s| matches!(s.kind.as_str(), "method" | "trait_method" | "function") && s.name == last).collect();
        if let Some(own) = methods.iter().find(|s| s.kind == "trait_method") {
            add(&own.id, name, &mut out);
        } else if methods.len() <= 3 {
            for s in methods {
                add(&s.id, name, &mut out);
            }
        }
    }
    out
}

/// What a diff removes and adds in one file; a removed line keeps its number in the old file.
struct FileChange {
    path: String,
    removed: Vec<(usize, String)>,
    added: Vec<String>,
}

fn file_changes(diff: &str) -> Vec<FileChange> {
    let strip = |p: &str| p.strip_prefix("a/").or_else(|| p.strip_prefix("b/")).unwrap_or(p).to_owned();
    let mut files: Vec<FileChange> = vec![];
    let (mut old_path, mut line_at, mut old_left, mut new_left) = (String::new(), 0usize, 0usize, 0usize);
    for line in diff.lines() {
        if old_left == 0 && new_left == 0 {
            if let Some(path) = line.strip_prefix("--- ") {
                old_path = strip(path.trim());
            } else if let Some(path) = line.strip_prefix("+++ ") {
                let path = path.trim();
                files.push(FileChange { path: if path == "/dev/null" { old_path.clone() } else { strip(path) }, removed: vec![], added: vec![] });
            } else if let Some(header) = line.strip_prefix("@@ -") {
                let range = |r: &str| { let mut p = r.split(','); (p.next().and_then(|n| n.parse().ok()).unwrap_or(0usize), p.next().and_then(|n| n.parse().ok()).unwrap_or(1usize)) };
                let mut parts = header.split(' ');
                let (start, count) = range(parts.next().unwrap_or(""));
                let (_, added) = range(parts.next().unwrap_or("").trim_start_matches('+'));
                (line_at, old_left, new_left) = (start, count, added);
            }
            continue;
        }
        let Some(file) = files.last_mut() else { continue };
        match line.as_bytes().first() {
            Some(b'-') => { file.removed.push((line_at, line[1..].to_owned())); line_at += 1; old_left = old_left.saturating_sub(1); }
            Some(b'+') => { file.added.push(line[1..].to_owned()); new_left = new_left.saturating_sub(1); }
            Some(b'\\') => {}
            _ => { line_at += 1; old_left = old_left.saturating_sub(1); new_left = new_left.saturating_sub(1); }
        }
    }
    files
}

/// A counter's own line gone along with the function that held it is a removed function, not a
/// tampered measure.
fn goes_with_function(file: &FileChange, line: usize, holder_start: &impl Fn(&str, usize) -> Option<usize>) -> bool {
    holder_start(&file.path, line).is_some_and(|start| start != line && file.removed.iter().any(|(l, _)| *l == start))
}

/// The counters of the baseline the change no longer reports, file by file. A line that only moved
/// keeps its count in the file, so it is not missing. What is missing is either deleted by the
/// diff (with its function or alone) or still written and simply no longer reached.
fn counters_gone(baseline: &Value, current: &Value, diff: &str, holder_start: &impl Fn(&str, usize) -> Option<usize>) -> Value {
    let sites = |stats: &Value| -> std::collections::BTreeMap<String, Vec<(usize, String)>> {
        let mut found: std::collections::BTreeMap<String, Vec<(usize, String)>> = Default::default();
        for key in stats.as_object().into_iter().flatten().map(|(k, _)| k) {
            if let Some((path, line)) = key.strip_prefix("calls@").and_then(|s| s.rsplit_once(':')) {
                found.entry(path.to_owned()).or_default().push((line.parse().unwrap_or(0), key.clone()));
            }
        }
        found
    };
    let (before, after, files) = (sites(baseline), sites(current), file_changes(diff));
    let (mut deleted, mut with_function, mut stopped) = (vec![], vec![], 0usize);
    for (path, had) in &before {
        let has = after.get(path).map_or(&[][..], |v| &v[..]);
        let missing: Vec<&(usize, String)> = had.iter().filter(|(_, key)| !has.iter().any(|(_, k)| k == key)).collect();
        let added = has.iter().filter(|(_, key)| !had.iter().any(|(_, k)| k == key)).count();
        let gone = missing.len().saturating_sub(added);
        if gone == 0 {
            continue;
        }
        let mut removed = 0;
        if let Some(file) = files.iter().find(|f| f.path == *path) {
            for (line, key) in missing.iter().filter(|(line, _)| file.removed.iter().any(|(l, _)| l == line)).map(|m| (m.0, &m.1)).take(gone) {
                removed += 1;
                if goes_with_function(file, line, holder_start) { with_function.push(key.clone()); } else { deleted.push(key.clone()); }
            }
        }
        stopped += gone - removed;
    }
    json!({ "deleted": deleted, "with_function": with_function, "stopped": stopped })
}

/// A change's touches on the measure the project declares: its protected paths and its statement.
fn measurement_issues(diff: &str, statement: Option<&str>, protect: &[String], holder_start: &impl Fn(&str, usize) -> Option<usize>) -> Vec<String> {
    let statement = statement.map(str::trim).filter(|s| !s.is_empty());
    let mut issues = vec![];
    for file in file_changes(diff) {
        if protect.iter().any(|p| file.path == *p || file.path.starts_with(p.as_str())) {
            issues.push(format!("{} faz parte da medição e não pode mudar", file.path));
        }
        let Some(statement) = statement else { continue };
        for (line, _) in file.removed.iter().filter(|(_, text)| text.contains(statement)) {
            if !goes_with_function(&file, *line, holder_start) {
                issues.push(format!("{}:{line} apaga a instrução de contagem", file.path));
            }
        }
        if file.added.iter().any(|text| text.contains(statement)) {
            issues.push(format!("{} acrescenta a instrução de contagem", file.path));
        }
    }
    issues
}

/// Adds to an execution of a change what the gate needs to know about its measure.
pub fn review(record: &mut Value, baseline_stats: Option<&Value>, diff: &str, statement: Option<&str>, protect: &[String], holder_start: impl Fn(&str, usize) -> Option<usize>) {
    if let Some(baseline) = baseline_stats {
        let counters = counters_gone(baseline, &record["summary"]["stats"], diff, &holder_start);
        record["comparison"]["counters"] = counters;
    }
    record["measurement_issues"] = json!(measurement_issues(diff, statement, protect, &holder_start));
}

/// What the project measured for the whole run (no `at=`), with how much each number improved, in
/// percent: a time gets better when it drops, and the profile's `measures` say the direction of any
/// other number. `wall_ms` is the Régua's own clock of the process (a test run, a build), not a
/// measure of the change. A command that counts says nothing here: its numbers include its
/// counters, and one counter less is faster.
fn project_numbers(x: &Value) -> Vec<(&String, &Value, f64)> {
    let counts = |stats: &Value| stats.as_object().into_iter().flatten().any(|(k, _)| k.starts_with("calls@"));
    if x["counts_itself"] == true || counts(&x["summary"]["stats"]) || counts(&x["comparison"]["stats"]) {
        return vec![];
    }
    x["comparison"]["stats"].as_object().into_iter().flatten().filter(|(k, _)| !k.contains('@') && *k != "wall_ms").filter_map(|(series, s)| {
        let change = s["change_pct"].as_f64().unwrap_or(0.);
        match x["measures"][series]["better"].as_str() {
            Some("higher") => Some((series, s, change)),
            Some(_) => Some((series, s, -change)),
            None => series.ends_with("_ms").then_some((series, s, -change)),
        }
    }).collect()
}

/// A number moved by a tenth, its ranges apart: what the repetitions can tell from noise.
fn moved(s: &Value, improved: f64) -> bool {
    improved.abs() >= 10. && s["ranges_overlap"] == false
}

/// What got worse with a change, which the gate does not refuse: a cache may spend memory on
/// purpose, so this is shown beside the decision, and the person weighs it.
pub fn worse(executions: &[Value]) -> Vec<String> {
    let mut found = vec![];
    for x in executions {
        let name = x["name"].as_str().unwrap_or("?");
        for (series, s, improved) in project_numbers(x) {
            if improved < 0. && moved(s, improved) {
                let means = x["measures"][series]["means"].as_str().map(|m| format!(" ({m})")).unwrap_or_default();
                found.push(format!("{name}: {series}{means} piorou {:.0}%", -improved));
            }
        }
    }
    found
}

/// The gate over the executions of one change: what passed and, when not, why.
pub fn verdict(executions: &[Value]) -> (bool, Vec<String>) {
    let mut why = vec![];
    let mut dropped = false;
    for x in executions {
        let name = x["name"].as_str().unwrap_or("?");
        if let Some(refused) = x["refused"].as_str() {
            why.push(format!("{name}: {refused}"));
            continue;
        }
        let (ok, tried) = (x["summary"]["succeeded"].as_u64().unwrap_or(0), x["summary"]["attempted"].as_u64().unwrap_or(0));
        if tried == 0 || ok < tried {
            let error = x["runs"].as_array().into_iter().flatten().filter_map(|r| r["stderr_tail"].as_str())
                .flat_map(|t| t.lines()).find(|l| l.contains("error")).unwrap_or("falhou").trim().to_owned();
            why.push(format!("{name}: {error}"));
            continue;
        }
        let comparison = &x["comparison"];
        for issue in x["measurement_issues"].as_array().into_iter().flatten().filter_map(Value::as_str) {
            let issue = issue.to_owned();
            if !why.contains(&issue) {
                why.push(issue);
            }
        }
        for series in comparison["counters"]["deleted"].as_array().into_iter().flatten().filter_map(Value::as_str) {
            why.push(format!("{name}: {series} sumiu da medição: o contador foi apagado"));
        }
        // Still written and no longer reached: its count went to zero.
        if comparison["counters"]["stopped"].as_u64().unwrap_or(0) > 0 {
            dropped = true;
        }
        if let Some(outputs) = comparison["outputs"].as_object() {
            for (file, same) in outputs.iter().filter(|(_, v)| *v != "same") {
                why.push(format!("{name}: {file} mudou ({same})"));
            }
        }
        if project_numbers(x).iter().any(|(_, s, improved)| *improved > 0. && moved(s, *improved)) {
            dropped = true;
        }
        for (series, s) in comparison["stats"].as_object().into_iter().flatten().filter(|(k, _)| k.starts_with("calls@")) {
            let change = s["change_pct"].as_f64().unwrap_or(0.);
            if change <= -5. {
                dropped = true;
            } else if change >= 5. {
                why.push(format!("{name}: {series} subiu {change:+}%"));
            }
        }
    }
    if why.is_empty() && !dropped {
        let declared = executions.iter().any(|x| x["measures"].as_object().is_some_and(|m| !m.is_empty()));
        why.push(if declared { "nem contagem, nem tempo, nem medida declarada melhoraram" } else { "nem contagem nem tempo caíram" }.into());
    }
    (why.is_empty(), why)
}

/// How much a change that passed the gate gained, in percent, to choose among those of one answer:
/// the largest improvement of a number the project measured, which is what the change is for. A
/// command that counts does not say it, because its numbers include its counters; a project that
/// only counts is compared by the total of its counts. The Régua's own `wall_ms` says nothing.
pub fn gain(executions: &[Value]) -> f64 {
    let mut time: Option<f64> = None;
    let (mut before, mut after) = (0., 0.);
    for x in executions {
        for (_, s) in x["comparison"]["stats"].as_object().into_iter().flatten().filter(|(k, _)| k.starts_with("calls@")) {
            before += s["baseline_median"].as_f64().unwrap_or(0.);
            after += s["median"].as_f64().unwrap_or(0.);
        }
        for (_, _, improved) in project_numbers(x) {
            time = Some(time.map_or(improved, |t: f64| t.max(improved)));
        }
    }
    time.unwrap_or(if before > 0. { (before - after) / before * 100. } else { 0. })
}

/// A send's usable changes (each HYPOTHESIS with a derived diff, no issue of its own and no
/// issue of the whole answer), its LACUNA texts and its issue codes.
pub fn outcome(summary: &Value) -> (Vec<String>, Vec<String>, Vec<String>) {
    let check = &summary["check"];
    let issues: Vec<&Value> = check["report"]["issues"].as_array().into_iter().flatten().collect();
    let general = issues.iter().any(|i| i["claim_id"].is_null());
    let claims: Vec<&Value> = check["claims"].as_array().into_iter().flatten().collect();
    let usable = claims.iter().filter(|c| !general && c["kind"] == "HYPOTHESIS" && check["diffs"][c["id"].as_str().unwrap_or("")].is_string()
        && !issues.iter().any(|i| i["claim_id"] == c["id"])).filter_map(|c| c["id"].as_str().map(String::from)).collect();
    let lacunas = claims.iter().filter(|c| c["kind"] == "LACUNA").filter_map(|c| c["text"].as_str().map(String::from)).collect();
    let codes = issues.iter().filter_map(|i| i["code"].as_str().map(String::from)).collect();
    (usable, lacunas, codes)
}

pub fn step(model: &Value, turn: &str, send: &str, what: &str, detail: Value) -> Value {
    json!({ "provider": model["provider"], "modelfile": model["modelfile"], "claude_model": model["claude_model"],
        "turn": turn, "send": send, "outcome": what, "detail": detail })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_a_lacuna_asks_for() {
        let text = "Para guardar a inversa faltam o método transform() e as declarações das structs das formas (como Triangle), onde o campo `inverse_transform` seria criado; também `Object::intersect`.";
        let names = lacuna_names(text);
        for n in ["transform", "Triangle", "inverse_transform", "Object::intersect"] {
            assert!(names.contains(&n.to_owned()), "{n} em {names:?}");
        }
    }

    /// The records of the video's recording: what the gate really saw, round by round.
    fn round(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    /// The round's executions as the studio records them now, with the renderer's declared measure.
    fn reviewed(r: &Value, statement: Option<&str>, protect: &[String]) -> Vec<Value> {
        r["executions"].as_array().unwrap().iter().map(|x| {
            let mut x = x.clone();
            let baseline = x["baseline_stats"].clone();
            review(&mut x, Some(&baseline), r["diff"].as_str().unwrap(), statement, protect, |_, line| Some(line - 1));
            x
        }).collect()
    }
    const STATEMENT: Option<&str> = Some("crate::count!();");
    fn renderer_protect() -> Vec<String> { vec!["librarian.json".into(), "src/count.rs".into(), "src/bin/cena_video.rs".into()] }

    #[test]
    fn round_4_removing_a_counter_is_not_an_optimization() {
        let r4 = round(include_str!("fixtures/rodada4.json"));
        let (passed, why) = verdict(r4["executions"].as_array().unwrap());
        assert!(!passed, "a rodada 4 só apagou um contador e passou no portão: {why:?}");
    }

    #[test]
    fn round_4_is_caught_by_each_layer_alone() {
        let r4 = round(include_str!("fixtures/rodada4.json"));
        // Evidence: the command that counts proves nothing about time.
        let (_, why) = verdict(r4["executions"].as_array().unwrap());
        assert_eq!(why, ["nem contagem nem tempo caíram"]);
        // The vanished series, with no statement declared anywhere.
        let (passed, why) = verdict(&reviewed(&r4, None, &[]));
        assert!(!passed);
        assert!(why.iter().any(|w| w == "medir-contagem: calls@src/groups.rs:89 sumiu da medição: o contador foi apagado"), "{why:?}");
        // The declared statement, even with no series to compare.
        let mut alone = r4["executions"][0].clone();
        review(&mut alone, None, r4["diff"].as_str().unwrap(), STATEMENT, &[], |_, line| Some(line - 1));
        let (passed, why) = verdict(&[alone]);
        assert!(!passed);
        assert!(why.iter().any(|w| w == "src/groups.rs:89 apaga a instrução de contagem"), "{why:?}");
    }

    #[test]
    fn rounds_1_2_and_3_still_pass_each_for_its_own_reason() {
        for text in [include_str!("fixtures/rodada1.json"), include_str!("fixtures/rodada2.json"), include_str!("fixtures/rodada3.json")] {
            let r = round(text);
            assert_eq!(verdict(&reviewed(&r, STATEMENT, &renderer_protect())), (true, vec![]), "{}", r["origin"]);
        }
        // Rounds 1 and 2: a measured improvement, in the command that does not count.
        for text in [include_str!("fixtures/rodada1.json"), include_str!("fixtures/rodada2.json")] {
            let r = round(text);
            assert!(verdict(&[r["executions"][0].clone()]).0, "{}", r["origin"]);
        }
        // Round 3: fewer operations; its time alone, within the repetitions' range, proves nothing.
        // Its counter in groups.rs moved a line down and did not vanish.
        let r3 = round(include_str!("fixtures/rodada3.json"));
        assert!(!verdict(&[r3["executions"][0].clone()]).0);
        assert!(verdict(&[r3["executions"][1].clone()]).0);
        let done = reviewed(&r3, STATEMENT, &renderer_protect());
        assert_eq!(done[1]["comparison"]["counters"], json!({ "deleted": [], "with_function": [], "stopped": 0 }));
    }

    #[test]
    fn the_same_false_positive_in_python() {
        // No recording here: the shape of round 4 in a project that prints its own counts, whose
        // statement "Medir tudo" could not place.
        let diff = "--- a/app/routes.py\n+++ b/app/routes.py\n@@ -40,3 +40,2 @@ def plan_route(stops):\n     ordered = sorted(stops)\n-    librarian_count()\n     return ordered\n";
        let series = |route: bool| {
            let mut s = json!({ "calls@app/models.py:8": { "median": 100. }, "total_ms": { "median": 85. } });
            if route { s["calls@app/routes.py:41"] = json!({ "median": 5000. }); }
            s
        };
        let mut counting = json!({ "name": "contagem", "summary": { "succeeded": 2, "attempted": 2, "stats": series(false) },
            "comparison": { "stats": { "calls@app/models.py:8": { "change_pct": 0. }, "total_ms": { "change_pct": -15., "ranges_overlap": false } } } });
        let mut real = json!({ "name": "tempo", "summary": { "succeeded": 5, "attempted": 5, "stats": { "total_ms": { "median": 40. } } },
            "comparison": { "stats": { "total_ms": { "change_pct": -8., "ranges_overlap": true } } } });
        // Before any review: only the evidence of each command.
        assert_eq!(verdict(&[counting.clone(), real.clone()]).1, ["nem contagem nem tempo caíram"]);
        review(&mut counting, Some(&series(true)), diff, Some("librarian_count()"), &[], |_, line| Some(line - 1));
        review(&mut real, Some(&json!({ "total_ms": { "median": 43. } })), diff, Some("librarian_count()"), &[], |_, line| Some(line - 1));
        let (passed, why) = verdict(&[counting, real]);
        assert!(!passed);
        assert!(why.iter().any(|w| w == "app/routes.py:41 apaga a instrução de contagem"), "{why:?}");
        assert!(why.iter().any(|w| w == "contagem: calls@app/routes.py:41 sumiu da medição: o contador foi apagado"), "{why:?}");
    }

    #[test]
    fn a_counter_can_leave_for_honest_reasons() {
        let base = json!({ "calls@src/a.rs:10": {}, "calls@src/b.rs:5": {} });
        let now = json!({ "calls@src/b.rs:5": {} });
        // Its whole function went away: nothing to refuse, and no drop either.
        let whole = "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -9,3 +9,0 @@\n-fn old() {\n-    count();\n-}\n";
        let gone = counters_gone(&base, &now, whole, &|_, line| Some(line - 1));
        assert_eq!(gone, json!({ "deleted": [], "with_function": ["calls@src/a.rs:10"], "stopped": 0 }));
        assert!(measurement_issues(whole, Some("count();"), &[], &|_, line| Some(line - 1)).is_empty());
        // Still written, no longer reached: the count went to zero, a real drop.
        let elsewhere = "--- a/src/c.rs\n+++ b/src/c.rs\n@@ -1 +1 @@\n-if a.ready() { a.old(); }\n+if a.ready() { }\n";
        let gone = counters_gone(&base, &now, elsewhere, &|_, line| Some(line - 1));
        assert_eq!(gone["stopped"], 1);
        let stopped = json!({ "name": "contagem", "summary": { "succeeded": 1, "attempted": 1 }, "comparison": { "counters": gone, "stats": {} } });
        assert!(verdict(&[stopped]).0);
    }

    #[test]
    fn a_change_may_not_touch_the_declared_measure() {
        let protect = renderer_protect();
        let touch = |path: &str| format!("--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-a\n+b\n");
        for path in ["librarian.json", "src/count.rs", "src/bin/cena_video.rs"] {
            assert_eq!(measurement_issues(&touch(path), None, &protect, &|_, _| None), [format!("{path} faz parte da medição e não pode mudar")]);
        }
        assert!(measurement_issues(&touch("src/matrix.rs"), None, &protect, &|_, _| None).is_empty());
        let added = "--- a/src/m.rs\n+++ b/src/m.rs\n@@ -1 +1,2 @@\n fn a() {\n+    crate::count!();\n";
        assert_eq!(measurement_issues(added, STATEMENT, &[], &|_, _| None), ["src/m.rs acrescenta a instrução de contagem"]);
        // A statement in the context lines of the diff is not touched.
        let context = "--- a/src/m.rs\n+++ b/src/m.rs\n@@ -1,2 +1,2 @@\n     crate::count!();\n-    a();\n+    b();\n";
        assert!(measurement_issues(context, STATEMENT, &[], &|_, _| None).is_empty());
    }

    #[test]
    fn among_the_changes_that_pass_the_largest_time_drop_wins() {
        let of = |text: &str| gain(&reviewed(&round(text), None, &[]));
        let (first, second, third, fourth) = (of(include_str!("fixtures/rodada1.json")), of(include_str!("fixtures/rodada2.json")), of(include_str!("fixtures/rodada3.json")), of(include_str!("fixtures/rodada4.json")));
        // What the renders of the recording measured: −15%, −51%, −5% and −9%.
        assert!(second > first && first > fourth && fourth > third, "{first} {second} {third} {fourth}");
        // A project that only counts is compared by its counts, and a time it did not measure is no gain.
        let counting = |after: f64| json!([{ "name": "c", "comparison": { "stats": { "calls@a:1": { "baseline_median": 100., "median": after, "change_pct": after - 100. }, "wall_ms": { "change_pct": -90. } } } }]);
        assert_eq!(gain(counting(40.).as_array().unwrap()), 60.);
        assert_eq!(gain(counting(100.).as_array().unwrap()), 0.);
    }

    #[test]
    fn a_measure_the_project_declares_counts_in_its_direction() {
        let x = |series: &str, change: f64, overlap: bool, better: Option<&str>| {
            let mut x = json!({ "name": "medir", "summary": { "succeeded": 3, "attempted": 3 },
                "comparison": { "stats": { series: { "change_pct": change, "ranges_overlap": overlap } } } });
            if let Some(better) = better {
                x["measures"] = json!({ series: { "better": better, "means": "o que mede" } });
            }
            x
        };
        // What Python showed: memory −77% refused because nobody said memory is a measure.
        assert_eq!(verdict(&[x("memory_end_mb", -77., false, None)]).1, ["nem contagem nem tempo caíram"]);
        assert!(verdict(&[x("memory_end_mb", -77., false, Some("lower"))]).0);
        assert!(verdict(&[x("queries_per_plan", -49., false, Some("lower"))]).0);
        assert_eq!(gain(&[x("memory_end_mb", -77., false, Some("lower"))]), 77.);
        // The direction is the project's; noise and a small change prove nothing.
        assert!(verdict(&[x("plans_per_s", 30., false, Some("higher"))]).0);
        assert!(!verdict(&[x("plans_per_s", -30., false, Some("higher"))]).0);
        assert_eq!(verdict(&[x("memory_end_mb", -77., true, Some("lower"))]).1, ["nem contagem, nem tempo, nem medida declarada melhoraram"]);
        assert!(!verdict(&[x("memory_end_mb", -8., false, Some("lower"))]).0);
        // A number in a command that counts includes its counters.
        let mut counting = x("memory_end_mb", -77., false, Some("lower"));
        counting["counts_itself"] = json!(true);
        assert!(!verdict(std::slice::from_ref(&counting)).0);
        assert!(worse(std::slice::from_ref(&counting)).is_empty());
    }

    #[test]
    fn what_got_worse_is_shown_and_does_not_refuse() {
        let change = json!({ "name": "medir", "summary": { "succeeded": 3, "attempted": 3 },
            "measures": { "memory_end_mb": { "better": "lower", "means": "memória no fim" }, "plans_per_s": { "better": "higher" } },
            "comparison": { "stats": {
                "plan_p50_ms": { "change_pct": -34., "ranges_overlap": false },
                "read_p50_ms": { "change_pct": 38., "ranges_overlap": false },
                "memory_end_mb": { "change_pct": 22., "ranges_overlap": false },
                "plans_per_s": { "change_pct": -12., "ranges_overlap": false },
                "memory_start_mb": { "change_pct": 40., "ranges_overlap": false },
                "plan_p95_ms": { "change_pct": 30., "ranges_overlap": true },
                "wall_ms": { "change_pct": 90., "ranges_overlap": false } } } });
        assert_eq!(verdict(std::slice::from_ref(&change)), (true, vec![]));
        assert_eq!(worse(&[change]), ["medir: memory_end_mb (memória no fim) piorou 22%", "medir: plans_per_s piorou 12%", "medir: read_p50_ms piorou 38%"]);
    }

    #[test]
    fn the_gate_needs_a_drop_and_the_same_outputs() {
        let x = |name: &str, change: f64, output: &str| json!({ "name": name, "summary": { "succeeded": 1, "attempted": 1 },
            "comparison": { "outputs": { "a.png": output }, "stats": { "calls@src/m.rs:1": { "change_pct": change } } } });
        assert_eq!(verdict(&[x("contagem", -60., "same")]), (true, vec![]));
        assert!(!verdict(&[x("contagem", 0., "same")]).0);
        assert!(!verdict(&[x("contagem", -60., "different")]).0);
        assert!(!verdict(&[x("contagem", 40., "same")]).0);
        let faster = json!({ "name": "medir", "summary": { "succeeded": 5, "attempted": 5 },
            "comparison": { "outputs": { "a.png": "same" }, "stats": { "render_ms": { "change_pct": -30., "ranges_overlap": false } } } });
        assert!(verdict(&[faster]).0);
        let wall = json!({ "name": "testes", "summary": { "succeeded": 1, "attempted": 1 },
            "comparison": { "stats": { "wall_ms": { "change_pct": -30., "ranges_overlap": false } } } });
        assert!(!verdict(&[wall]).0);
    }
}
