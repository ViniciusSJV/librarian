//! The ladder: one question over an envelope, to the lightest model first and to the next only
//! when the gate refuses. A change passes when every gate command succeeds with it, every
//! output is the same as the baseline's, a count or a measured time drops and no count grows. A LACUNA does not
//! climb the ladder: the names it says are missing are looked up in the catalog, gathered, and
//! the same model is asked again. Nothing here judges meaning; every step is recorded.
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
        if let Some(outputs) = comparison["outputs"].as_object() {
            for (file, same) in outputs.iter().filter(|(_, v)| *v != "same") {
                why.push(format!("{name}: {file} mudou ({same})"));
            }
        }
        // A time of the whole run (no `at=`) that drops by a tenth, its ranges apart, also counts.
        for (_, s) in comparison["stats"].as_object().into_iter().flatten().filter(|(k, _)| !k.contains('@') && k.ends_with("_ms")) {
            if s["change_pct"].as_f64().unwrap_or(0.) <= -10. && s["ranges_overlap"] == false {
                dropped = true;
            }
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
        why.push("nem contagem nem tempo caíram".into());
    }
    (why.is_empty(), why)
}

/// A send's usable change (its first HYPOTHESIS with a derived diff and no issue of its own),
/// or its LACUNA texts.
pub fn outcome(summary: &Value) -> (Option<String>, Vec<String>, Vec<String>) {
    let check = &summary["check"];
    let issues: Vec<&Value> = check["report"]["issues"].as_array().into_iter().flatten().collect();
    let passed = check["report"]["mechanical_status"] == "passed";
    let claims: Vec<&Value> = check["claims"].as_array().into_iter().flatten().collect();
    let usable = claims.iter().find(|c| passed && c["kind"] == "HYPOTHESIS" && check["diffs"][c["id"].as_str().unwrap_or("")].is_string()
        && !issues.iter().any(|i| i["claim_id"] == c["id"])).and_then(|c| c["id"].as_str().map(String::from));
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
    }
}
