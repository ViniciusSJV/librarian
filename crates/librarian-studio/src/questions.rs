//! "Eu pergunto, o Librarian separa": a question typed about performance is recognized by rules
//! over its words (after the project's lexicon), answered from the latest "Medir tudo" and split
//! into small questions: facts for the light model, changes for the ladder. No LLM decides the
//! route; every number in the text comes from the measure.
use crate::store::Store;
use librarian_ingest::{search::{tokens, Lexicon}, Catalog, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// The kinds of question, tried in this order; the first whose rule matches wins.
const INTENTS: [&str; 3] = ["optimize", "where", "heavy"];
/// A function is "frequent" from this many calls in a frame (as on the page).
const FREQUENT: f64 = 1e6;

const OPTIMIZE: &[&str] = &["otimize", "otimizar", "otimiza", "otimizacao", "melhore", "acelere", "acelerar", "optimize", "speed"];
const WHERE: &[&str] = &["onde", "where"];
const IMPROVE: &[&str] = &["otimizar", "melhorar", "ganhar", "acelerar", "optimize", "improve"];
const HEAVY: &[&str] = &["pesa", "pesam", "pesado", "pesada", "pesados", "lento", "lenta", "lentas", "demora", "demoram", "gargalo", "custa", "caro",
    "heavy", "slow", "slowest", "hot", "expensive", "bottleneck"];

/// Each token of the question with the members of its lexicon group (raio → ray, rays).
fn expand(question: &str, lexicon: &Lexicon) -> Vec<BTreeSet<String>> {
    tokens(question).into_iter().map(|t| {
        let mut all = BTreeSet::from([t.clone()]);
        for group in &lexicon.groups {
            let folded: Vec<String> = group.iter().flat_map(|w| tokens(w)).collect();
            if folded.contains(&t) {
                all.extend(folded);
            }
        }
        all
    }).collect()
}

/// The intent, the words that decided it and the function the question names, if any.
pub fn route(question: &str, lexicon: &Lexicon, functions: &[String]) -> Value {
    let words = expand(question, lexicon);
    let hit = |list: &[&str]| -> Vec<String> {
        words.iter().filter_map(|w| w.iter().find(|x| list.contains(&x.as_str())).cloned()).collect()
    };
    // The function the question names: written whole (Groups::intersect) beats only its last part
    // (intersect); among equals, the longest name.
    let written = tokens(question);
    let writes = |text: &str| { let want = tokens(text); !want.is_empty() && written.windows(want.len()).any(|w| w == want.as_slice()) };
    let named = functions.iter()
        .filter_map(|f| if writes(f) { Some((2, f)) } else if writes(f.rsplit("::").next().unwrap_or(f)) { Some((1, f)) } else { None })
        .max_by_key(|(score, f)| (*score, f.len())).map(|(_, f)| f.clone());
    for intent in INTENTS {
        let matched: Vec<String> = match intent {
            "optimize" => if named.is_some() { hit(OPTIMIZE) } else { vec![] },
            "where" => { let (w, i) = (hit(WHERE), hit(IMPROVE)); if w.is_empty() || i.is_empty() { vec![] } else { [w, i].concat() } }
            _ => hit(HEAVY),
        };
        if !matched.is_empty() {
            return json!({ "intent": intent, "matched": matched, "function": named });
        }
    }
    json!({ "intent": null, "matched": [], "function": named })
}

fn int(v: f64) -> String {
    let digits = format!("{}", v.round() as i64);
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    out
}

fn short(name: &str) -> &str {
    name.strip_prefix("impl ").unwrap_or(name)
}

impl Store {
    /// The typed question, recognized, answered from the latest "Medir tudo" and split into the
    /// small questions the light model and the ladder receive.
    /// A question the rules do not recognize comes back with no intent: it is for the model. One
    /// they recognize, before any measure, comes back asking for "Medir tudo".
    pub fn performance_question(&self, key: &str, run: &str, question: &str) -> Result<Value> {
        let lexicon = self.settings(key)?.lexicon;
        if self.latest_measure(key, run)?.is_none() {
            let catalog = Catalog::load(&self.edition(key, run)?)?;
            let names: Vec<String> = catalog.symbols().values().map(|s| short(&s.qualified_name).to_owned()).collect();
            let route = route(question, &lexicon, &names);
            let needs_measure = route["intent"].is_string();
            return Ok(json!({ "route": route, "needs_measure": needs_measure, "facts": [], "changes": [],
                "answer": ["Para responder, o Librarian precisa saber quantas vezes cada função roda, e esta edition não tem essa medição."] }));
        }
        let hot = self.hotspots(key, run)?;
        let rows: Vec<&Value> = hot["rows"].as_array().into_iter().flatten().filter(|r| r["symbol"].is_string()).collect();
        let names: Vec<String> = rows.iter().filter_map(|r| r["name"].as_str().map(|n| short(n).to_owned())).collect();
        let route = route(question, &lexicon, &names);
        let calls = |r: &Value| r["calls"].as_f64().unwrap_or(0.);
        let ms = |r: &Value| r["cpu_ms"].as_f64().unwrap_or(0.);
        let by_name = |n: &str| rows.iter().find(|r| r["name"].as_str().map(short) == Some(n)).copied();
        let mut facts: Vec<Value> = vec![];
        let mut changes: Vec<Value> = vec![];
        let mut answer: Vec<String> = vec![];
        let fact = |symbol: &Value, text: String| json!({ "to": "qwen", "symbol": symbol, "nodes": [symbol], "text": text });
        match route["intent"].as_str() {
            Some("heavy") => {
                let mut top = rows.clone();
                top.sort_by(|a, b| ms(b).total_cmp(&ms(a)));
                answer.push(format!("As que mais pesam na medição (tempo somado das threads): {}.",
                    top.iter().take(5).map(|r| format!("{} ({} chamadas, {} ms)", short(r["name"].as_str().unwrap_or("?")), int(calls(r)), int(ms(r)))).collect::<Vec<_>>().join(", ")));
            }
            Some("where") | Some("optimize") => {
                let target = match route["function"].as_str() {
                    Some(n) => by_name(n),
                    None => rows.iter().find(|r| calls(r) >= FREQUENT).copied(),
                };
                let Some(target) = target else {
                    return Ok(json!({ "route": route, "answer": ["Nenhuma função roda um milhão de vezes nesta medida."], "facts": [], "changes": [] }));
                };
                let name = short(target["name"].as_str().unwrap_or("?"));
                // Callers with at least 5% of its calls; a recursive function may have none, so the
                // most frequent one stays.
                let all: Vec<&Value> = target["callers"].as_array().into_iter().flatten().collect();
                let mut callers: Vec<&Value> = all.iter().copied().filter(|c| calls(c) >= calls(target) * 0.05).collect();
                if callers.is_empty() {
                    callers.extend(all.first().copied());
                }
                answer.push(if route["function"].is_string() {
                    format!("{name} roda {} vezes na medição, {} ms somados nas threads.", int(calls(target)), int(ms(target)))
                } else {
                    format!("{name} é a que mais roda entre as frequentes: {} vezes na medição, {} ms somados nas threads.", int(calls(target)), int(ms(target)))
                });
                if !callers.is_empty() {
                    answer.push(format!("Quem mais a chama: {}.", callers.iter().map(|c| format!("{} ({})", short(c["name"].as_str().unwrap_or("?")), int(calls(c)))).collect::<Vec<_>>().join(", ")));
                }
                for c in &callers {
                    let Some(row) = rows.iter().find(|r| r["symbol"] == c["symbol"]) else { continue };
                    let caller = short(row["name"].as_str().unwrap_or("?"));
                    for line in row["calls_out"].as_array().into_iter().flatten().filter(|l| l["targets"].as_array().into_iter().flatten().any(|t| t["symbol"] == target["symbol"])) {
                        facts.push(fact(&row["symbol"], format!(
                            "{caller} chama {name} na linha {} (`{}`). {caller} roda {} vezes na medição e {name}, {}. O que essa linha calcula a cada vez, e de quais valores ela depende? Cite a linha {}.",
                            line["line"], line["text"].as_str().unwrap_or(""), int(calls(row)), int(calls(target)), line["line"])));
                    }
                }
                let mut nodes = vec![target["symbol"].clone()];
                nodes.extend(callers.iter().map(|c| c["symbol"].clone()));
                changes.push(json!({ "to": "ladder", "symbol": target["symbol"], "nodes": nodes, "text": format!(
                    "{name} é a função que mais roda na medição: {} vezes{}. O que dá para mudar para ela trabalhar menos, sem mudar as saídas nem o resultado dos testes? Proponha até três mudanças diferentes, uma por HYPOTHESIS.",
                    int(calls(target)),
                    if callers.is_empty() { String::new() } else { format!(", chamada principalmente por {}", callers.iter().map(|c| short(c["name"].as_str().unwrap_or("?"))).collect::<Vec<_>>().join(" e ")) }) }));
            }
            _ => answer.push("Não reconheci a pergunta. Pergunte, por exemplo: quais funções pesam? onde otimizar? otimize o Groups::intersect.".to_owned()),
        }
        Ok(json!({ "route": route, "answer": answer, "facts": facts, "changes": changes, "execution": hot["execution"],
            "limits": ["A rota sai de regras sobre as palavras da pergunta, depois do léxico do projeto; nenhum modelo escolhe."] }))
    }
}

#[cfg(test)]
mod tests {
    use super::route;
    use librarian_ingest::search::Lexicon;

    #[test]
    fn routes_typed_questions_by_words_and_lexicon() {
        let lexicon = Lexicon { groups: vec![vec!["raio".into(), "ray".into()]], stop_words: vec![] };
        let functions = vec!["Groups::intersect".to_owned(), "World::color_at".to_owned()];
        let r = |q: &str| route(q, &lexicon, &functions);
        assert_eq!(r("Quais funções pesam?")["intent"], "heavy");
        // The lexicon widens the words: a synonym of a rule word counts as that word.
        let slow = Lexicon { groups: vec![vec!["devagar".into(), "lento".into()]], stop_words: vec![] };
        assert_eq!(route("Qual parte anda devagar?", &slow, &functions)["intent"], "heavy");
        assert_eq!(r("Onde dá para otimizar?")["intent"], "where");
        let named = r("Otimize o Groups::intersect");
        assert_eq!((named["intent"].as_str(), named["function"].as_str()), (Some("optimize"), Some("Groups::intersect")));
        assert_eq!(r("Bom dia")["intent"], serde_json::Value::Null);
    }
}
