//! R3 of the renderer phase: tests per unit, checked by mutation. The Librarian breaks a unit on
//! purpose, one written change at a time (an operator, a rounding, a literal), runs the project's
//! tests on each broken version and keeps the ones no test notices: the survivors. A survivor is
//! a concrete question for a model ("a test that passes now and fails with this line"), and the
//! answer is checked the same way: it only adds lines, it calls the unit, the tests pass with it,
//! and it makes the survivors fail.
//! Layout: runs/<run>/mutations/<NN-name>/{mutation.json, stdout-<mutant>.txt, check-*.json}.
use crate::execution::ChangeRef;
use crate::store::{reserve, safe_id, stamp, Store};
use librarian_ingest::{Catalog, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;

/// Mutants per unit, at most: the first ones in the order written.
const MAX_MUTANTS: usize = 12;

const LIMITS: &[&str] = &[
    "Mutantes são trocas escritas no texto da unidade (operador, arredondamento, literal), uma por vez; não são todas as maneiras de errar.",
    "Um mutante que sobrevive pode ser equivalente (calcula o mesmo, como x + y e x - y na paridade): nenhum teste o mata, e isso não é lacuna.",
    "Um mutante que não compila conta como morto pelos testes; a saída guardada mostra qual foi o caso.",
    "Que o teste novo matou um mutante é lido no texto da saída: o comando falhou e a saída nomeia a função de teste.",
    "O teste chama a unidade pelo nome escrito, não por resolução de tipo.",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Mutant {
    pub id: String,
    pub path: String,
    pub line: usize,
    pub operator: String,
    pub before: String,
    pub after: String,
}

/// Written replacements, each tried at every place it occurs in a line of the body.
const OPERATORS: &[(&str, &str)] = &[
    (" + ", " - "), (" - ", " + "), (" * ", " / "), (" / ", " * "),
    (" < ", " <= "), (" <= ", " < "), (" > ", " >= "), (" >= ", " > "), (" == ", " != "), (" != ", " == "),
    (" && ", " || "), (" || ", " && "),
    (".floor()", ".ceil()"), (".ceil()", ".floor()"), (".abs()", ""),
];

/// The mutants of a declaration's body (its lines between the first and the last), in the
/// order written: each operator at each place, then each floating literal plus one. Comment
/// lines and lines with text literals are left alone.
pub fn mutants(path: &str, lines: &[(usize, String)]) -> Vec<Mutant> {
    let mut found = vec![];
    let body = lines.get(1..lines.len().saturating_sub(1)).unwrap_or_default();
    for (number, text) in body {
        let trimmed = text.trim_start();
        if trimmed.starts_with("//") || text.contains('"') {
            continue;
        }
        let mut changed: Vec<(String, String)> = vec![];
        for (from, to) in OPERATORS {
            for (at, _) in text.match_indices(from) {
                changed.push((format!("{} → {}", from.trim(), if to.is_empty() { "∅" } else { to.trim() }), format!("{}{to}{}", &text[..at], &text[at + from.len()..])));
            }
        }
        for (at, literal) in float_literals(text) {
            let value: f64 = literal.trim_end_matches('.').parse().unwrap_or(0.0);
            let next = if literal.ends_with('.') { format!("{}.", value + 1.0) } else { format!("{}", value + 1.0) };
            changed.push((format!("{literal} → {next}"), format!("{}{next}{}", &text[..at], &text[at + literal.len()..])));
        }
        for (operator, after) in changed {
            if after != *text && !found.iter().any(|m: &Mutant| m.line == *number && m.after == after) {
                found.push(Mutant { id: String::new(), path: path.into(), line: *number, operator, before: text.clone(), after });
            }
        }
    }
    found.truncate(MAX_MUTANTS);
    for (i, m) in found.iter_mut().enumerate() {
        m.id = format!("m{}", i + 1);
    }
    found
}

/// Floating literals as written (`2.`, `0.5`), with where they start; not parts of names or
/// of method calls on integers.
fn float_literals(text: &str) -> Vec<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut found = vec![];
    let mut i = 0;
    while i < bytes.len() {
        let starts = bytes[i].is_ascii_digit() && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_' || bytes[i - 1] == b'.'));
        if !starts {
            i += 1;
            continue;
        }
        let begin = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'.' && !bytes.get(i + 1).is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_' || *b == b'.') {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            found.push((begin, &text[begin..i]));
        }
    }
    found
}

/// The one-line unified diff of a mutant, with no context.
pub fn diff(m: &Mutant) -> String {
    format!("--- a/{0}\n+++ b/{0}\n@@ -{1},1 +{1},1 @@\n-{2}\n+{3}\n", m.path, m.line, m.before, m.after)
}

/// The test functions a diff adds (`#[test]` followed by `fn name`), and whether it only adds.
fn added_tests(diff: &str) -> (Vec<String>, bool) {
    let added: Vec<&str> = diff.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).map(|l| &l[1..]).collect();
    let only_adds = !diff.lines().any(|l| l.starts_with('-') && !l.starts_with("---"));
    let mut names = vec![];
    let mut marked = false;
    for line in &added {
        let t = line.trim();
        if t.starts_with("#[test]") {
            marked = true;
        } else if let (true, Some(rest)) = (marked, t.strip_prefix("fn ").or_else(|| t.strip_prefix("pub fn "))) {
            names.push(rest.split(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or_default().to_owned());
            marked = false;
        }
    }
    (names.into_iter().filter(|n| !n.is_empty()).collect(), only_adds)
}

impl Store {
    /// Breaks the unit `symbol` once per mutant and runs `command` on each version: survivors are
    /// the mutants the command passes with. Proposes, for the survivors, the question for a model
    /// and the declarations to send (the unit and the test module of its file).
    pub fn mutation_run(&self, key: &str, run: &str, symbol: &str, command: &str) -> Result<Value> {
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let unit = catalog.symbols().get(symbol).ok_or("Declaração desconhecida nesta edition")?;
        let chunk = catalog.chunk(&unit.chunk_id).ok_or("Declaração sem trecho")?;
        let path = catalog.sources()[&unit.source_id].path.clone();
        let text = String::from_utf8_lossy(catalog.excerpt(&unit.chunk_id).ok_or("Trecho sem bytes")?).into_owned();
        let lines: Vec<(usize, String)> = text.lines().enumerate().map(|(i, l)| (chunk.start_line + i, l.trim_end_matches('\r').to_owned())).collect();
        let found = mutants(&path, &lines);
        if found.is_empty() {
            return Err("Nenhuma troca escrita a fazer no corpo desta declaração".into());
        }
        let (id, dir) = reserve(&self.run_dir(key, run)?.join("mutations"), &format!("{:02}-{}", self.mutations(key, run)?.len() + 1, safe_name(&unit.name)))?;
        let diffs: Vec<(String, String)> = found.iter().map(|m| (m.id.clone(), diff(m))).collect();
        let results = self.execute_mutants(key, run, command, None, &diffs, &[], &dir)?;
        let mut survivors = vec![];
        let runs: Vec<Value> = found.iter().zip(&results).map(|(m, r)| {
            let survived = r["applied"] == true && r["exit_code"] == 0 && r["timed_out"] == false;
            if survived {
                survivors.push(m.id.clone());
            }
            json!({ "mutant": m, "result": r, "survived": survived })
        }).collect();
        // The test module the unit's file holds: its tests are the example and where a new one goes.
        let module = catalog.symbols().values()
            .filter(|s| s.source_id == unit.source_id && s.kind == "module" && s.name.starts_with("test"))
            .max_by_key(|s| catalog.chunk(&s.chunk_id).map_or(0, |c| c.start_line));
        let survivors_text: Vec<String> = found.iter().filter(|m| survivors.contains(&m.id))
            .map(|m| format!("- {} linha {}: `{}` vira `{}`", m.id, m.line, m.before.trim(), m.after.trim())).collect();
        let question = (!survivors.is_empty()).then(|| format!(
            "Os testes do projeto passam mesmo com estas mudanças em `{}` ({path}):\n{}\nEscreva um teste #[test] novo que passe com o código atual e falhe com essas mudanças. \
             Insira o teste depois da última linha do último teste da evidência (insert_after), sem mudar nenhuma outra linha.",
            unit.qualified_name, survivors_text.join("\n")));
        let nodes: Vec<String> = [Some(symbol.to_owned()), module.map(|m| m.id.clone())].into_iter().flatten().collect();
        let record = json!({
            "format_version": 1, "id": id, "unit": { "symbol": symbol, "name": unit.qualified_name, "path": path,
                "start_line": chunk.start_line, "end_line": chunk.end_line },
            "command": command, "started": stamp(), "mutants": runs, "survivors": survivors,
            "question": question, "nodes": nodes,
            "limits": LIMITS,
        });
        fs::write(dir.join("mutation.json"), serde_json::to_vec_pretty(&record)?)?;
        Ok(record)
    }

    /// Mutation records of `run`, oldest first.
    pub fn mutations(&self, key: &str, run: &str) -> Result<Vec<Value>> {
        let dir = self.run_dir(key, run)?.join("mutations");
        let Ok(entries) = fs::read_dir(&dir) else { return Ok(vec![]) };
        let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).filter(|p| p.join("mutation.json").is_file()).collect();
        paths.sort();
        paths.iter().map(|p| Ok(serde_json::from_slice(&fs::read(p.join("mutation.json"))?)?)).collect()
    }

    /// Checks an answer's test against a mutation record: the diff only adds lines, adds a
    /// #[test] that calls the unit by name, `command` passes with it, and with it each
    /// survivor makes `command` fail naming the new test.
    pub fn mutation_check(&self, key: &str, run: &str, mutation: &str, change: &ChangeRef) -> Result<Value> {
        let dir = self.run_dir(key, run)?.join("mutations").join(safe_id(mutation)?);
        let record: Value = serde_json::from_slice(&fs::read(dir.join("mutation.json")).map_err(|_| "Mutação desconhecida")?)?;
        let command = record["command"].as_str().unwrap_or_default().to_owned();
        let diff = self.diff(key, run, change)?;
        let (tests, only_adds) = added_tests(&diff);
        let unit_name = record["unit"]["name"].as_str().unwrap_or_default().rsplit("::").next().unwrap_or_default().to_owned();
        let calls_unit = diff.lines().any(|l| l.starts_with('+') && l.contains(&format!("{unit_name}(")));
        let mut checks = json!({ "only_adds": only_adds, "adds_tests": tests, "calls_unit": calls_unit, "unit_name": unit_name });
        let ready = only_adds && !tests.is_empty() && calls_unit;
        let mut verdict = if ready { "pending" } else { "refused" };
        if ready {
            let passes = self.execute(key, run, &command, Some(change))?;
            let ok = passes["summary"]["succeeded"].as_u64() > Some(0);
            checks["passes_now"] = json!({ "ok": ok, "execution": passes["id"] });
            if !ok {
                verdict = "refused";
            } else {
                let survivors: Vec<Mutant> = record["mutants"].as_array().into_iter().flatten()
                    .filter(|m| m["survived"] == true).filter_map(|m| serde_json::from_value(m["mutant"].clone()).ok()).collect();
                let diffs: Vec<(String, String)> = survivors.iter().map(|m| (m.id.clone(), diff_of(m))).collect();
                let out = dir.join(format!("check-{}-{}-{}", change.turn, change.send, change.claim));
                fs::create_dir_all(&out)?;
                let results = self.execute_mutants(key, run, &command, Some(change), &diffs, &tests, &out)?;
                let killed: Vec<Value> = survivors.iter().zip(&results).map(|(m, r)| {
                    let failed = r["applied"] == true && r["exit_code"] != 0;
                    json!({ "mutant": m.id, "killed": failed && r["named"].as_array().is_some_and(|n| !n.is_empty()), "failed": failed, "named": r["named"] })
                }).collect();
                let count = killed.iter().filter(|k| k["killed"] == true).count();
                checks["mutants"] = json!(killed);
                checks["killed"] = json!(count);
                checks["survivors"] = json!(survivors.len());
                verdict = if count == survivors.len() { "passed" } else if count > 0 { "partial" } else { "refused" };
            }
        }
        let result = json!({ "format_version": 1, "mutation": mutation, "change": { "turn": change.turn, "send": change.send, "claim": change.claim },
            "checked": stamp(), "verdict": verdict, "checks": checks, "limits": LIMITS });
        fs::write(dir.join(format!("check-{}-{}-{}.json", change.turn, change.send, change.claim)), serde_json::to_vec_pretty(&result)?)?;
        Ok(result)
    }
}

fn diff_of(m: &Mutant) -> String {
    diff(m)
}

fn safe_name(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(lines: &[&str]) -> Vec<(usize, String)> {
        lines.iter().enumerate().map(|(i, l)| (10 + i, l.to_string())).collect()
    }

    #[test]
    fn mutants_change_one_written_thing_at_a_time_in_the_body() {
        let found = mutants("src/p.rs", &body(&[
            "    fn color_at(&self, point: Tuple) -> Color {",
            "        if ((point.x.floor() + point.z.floor()) % 2.).equivalent(0.) {",
            "            // a + b",
            "            self.a",
            "        }",
            "    }",
        ]));
        let shown: Vec<String> = found.iter().map(|m| format!("{} {} {}", m.id, m.line, m.operator)).collect();
        assert_eq!(shown, ["m1 11 + → -", "m2 11 .floor() → .ceil()", "m3 11 .floor() → .ceil()", "m4 11 2. → 3.", "m5 11 0. → 1."]);
        assert_eq!(found[0].after, "        if ((point.x.floor() - point.z.floor()) % 2.).equivalent(0.) {");
        assert_eq!(diff(&found[3]), "--- a/src/p.rs\n+++ b/src/p.rs\n@@ -11,1 +11,1 @@\n-        if ((point.x.floor() + point.z.floor()) % 2.).equivalent(0.) {\n+        if ((point.x.floor() + point.z.floor()) % 3.).equivalent(0.) {\n");
    }

    #[test]
    fn literals_skip_names_and_method_calls() {
        assert_eq!(float_literals("let v2 = 1.5 * x.0 + 2.max(3) + 10."), [(9, "1.5"), (32, "10.")]);
    }

    #[test]
    fn a_test_answer_is_read_from_its_diff() {
        let diff = "--- a/src/p.rs\n+++ b/src/p.rs\n@@ -40,0 +41,5 @@\n+\n+    #[test]\n+    fn rings_alternate() {\n+        assert!(true);\n+    }\n";
        assert_eq!(added_tests(diff), (vec!["rings_alternate".to_owned()], true));
        let changes = "--- a/src/p.rs\n+++ b/src/p.rs\n@@ -40,1 +40,1 @@\n-x\n+y\n";
        assert_eq!(added_tests(changes), (vec![], false));
    }
}
