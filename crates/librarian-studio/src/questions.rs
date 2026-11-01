//! "Eu pergunto, o Librarian separa": a question typed about performance is recognized by rules
//! over its words (after the project's lexicon), answered from the latest "Medir tudo" and the
//! operation inventory, and split into small questions: facts for the light model, changes for
//! the ladder. No LLM decides the route; every number in the text comes from the measure.
use crate::store::Store;
use librarian_ingest::{search::{tokens, Lexicon}, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// The kinds of question, tried in this order; the first whose rule matches wins.
const INTENTS: [&str; 5] = ["cores", "per_unit", "optimize", "where", "heavy"];
/// A function is "frequent" from this many calls in a frame (as on the page).
const FREQUENT: f64 = 1e6;

const CORES: &[&str] = &["nucleo", "nucleos", "core", "cores", "thread", "threads", "paralelo", "paralela", "paralelismo", "paralelizar", "parallel", "cpu", "cpus"];
const EACH: &[&str] = &["cada", "each", "every", "per"];
const CREATE: &[&str] = &["criado", "criada", "criados", "criadas", "cria", "criar", "crio", "aloca", "alocado", "alocada", "alocacao", "alocar",
    "created", "creates", "allocated", "allocates", "allocation", "new", "novo", "nova"];
const SORT: &[&str] = &["ordenado", "ordenada", "ordenados", "ordena", "ordenar", "ordenacao", "sort", "sorted", "sorts"];
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

/// The intent, the words that decided it and, for "a cada X", the unit X with its code terms.
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
    // "a cada raio", "per ray": the word after the marker.
    let unit = words.iter().position(|w| w.iter().any(|x| EACH.contains(&x.as_str())))
        .and_then(|i| words.get(i + 1))
        .map(|w| w.clone());
    for intent in INTENTS {
        let matched: Vec<String> = match intent {
            "cores" => hit(CORES),
            "per_unit" => {
                let (made, sorted) = (hit(CREATE), hit(SORT));
                if unit.is_none() || (made.is_empty() && sorted.is_empty()) { vec![] } else { [hit(EACH), made, sorted].concat() }
            }
            "optimize" => if named.is_some() { hit(OPTIMIZE) } else { vec![] },
            "where" => { let (w, i) = (hit(WHERE), hit(IMPROVE)); if w.is_empty() || i.is_empty() { vec![] } else { [w, i].concat() } }
            _ => hit(HEAVY),
        };
        if !matched.is_empty() {
            let first = |list: &[&str]| words.iter().flatten().any(|x| list.contains(&x.as_str()));
            return json!({ "intent": intent, "matched": matched, "function": named,
                "unit": unit.as_ref().map(|u| json!({ "word": tokens(question)[words.iter().position(|w| w == u).unwrap()], "terms": u })),
                "creates": first(CREATE), "sorts": first(SORT) });
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
    pub fn performance_question(&self, key: &str, run: &str, question: &str) -> Result<Value> {
        let hot = self.hotspots(key, run)?;
        let rows: Vec<&Value> = hot["rows"].as_array().into_iter().flatten().filter(|r| r["symbol"].is_string()).collect();
        let names: Vec<String> = rows.iter().filter_map(|r| r["name"].as_str().map(|n| short(n).to_owned())).collect();
        let lexicon = self.settings(key)?.lexicon;
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
                answer.push(format!("As que mais pesam no render medido (tempo somado das threads): {}.",
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
                    format!("{name} roda {} vezes num frame, {} ms somados nas threads.", int(calls(target)), int(ms(target)))
                } else {
                    format!("{name} é a que mais roda entre as frequentes: {} vezes num frame, {} ms somados nas threads.", int(calls(target)), int(ms(target)))
                });
                if !callers.is_empty() {
                    answer.push(format!("Quem mais a chama: {}.", callers.iter().map(|c| format!("{} ({})", short(c["name"].as_str().unwrap_or("?")), int(calls(c)))).collect::<Vec<_>>().join(", ")));
                }
                for c in &callers {
                    let Some(row) = rows.iter().find(|r| r["symbol"] == c["symbol"]) else { continue };
                    let caller = short(row["name"].as_str().unwrap_or("?"));
                    for line in row["calls_out"].as_array().into_iter().flatten().filter(|l| l["targets"].as_array().into_iter().flatten().any(|t| t["symbol"] == target["symbol"])) {
                        facts.push(fact(&row["symbol"], format!(
                            "{caller} chama {name} na linha {} (`{}`). {caller} roda {} vezes num frame e {name}, {}. O que essa linha calcula a cada vez, e de quais valores ela depende? Cite a linha {}.",
                            line["line"], line["text"].as_str().unwrap_or(""), int(calls(row)), int(calls(target)), line["line"])));
                    }
                }
                let mut nodes = vec![target["symbol"].clone()];
                nodes.extend(callers.iter().map(|c| c["symbol"].clone()));
                changes.push(json!({ "to": "ladder", "symbol": target["symbol"], "nodes": nodes, "text": format!(
                    "{name} é a função que mais roda no render: {} vezes num frame{}. O que dá para mudar para ela trabalhar menos, sem mudar a imagem nem o resultado dos testes? Proponha até três mudanças diferentes, uma por HYPOTHESIS.",
                    int(calls(target)),
                    if callers.is_empty() { String::new() } else { format!(", chamada principalmente por {}", callers.iter().map(|c| short(c["name"].as_str().unwrap_or("?"))).collect::<Vec<_>>().join(" e ")) }) }));
            }
            Some("cores") => {
                // The wall time of the work the project measures (render_ms, or another *_ms that is
                // not the whole command), against the busiest function's time summed over threads.
                let measures = hot["measures"].as_object().cloned().unwrap_or_default();
                let wall = measures.get("render_ms").or_else(|| measures.iter().find(|(k, _)| k.ends_with("_ms") && *k != "wall_ms").map(|(_, v)| v)).and_then(Value::as_f64);
                let busiest = rows.iter().max_by(|a, b| ms(a).total_cmp(&ms(b))).copied();
                let cores = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
                let threads = match (busiest, wall) {
                    (Some(b), Some(w)) if w > 0. => Some(ms(b) / w),
                    _ => None,
                };
                if let (Some(b), Some(w), Some(t)) = (busiest, wall, threads) {
                    answer.push(format!("No render medido, {} somou {} ms de tempo nas threads em {} ms de parede: em média {:.1} threads ocupadas, numa máquina de {} núcleos lógicos ({:.0}%).",
                        short(b["name"].as_str().unwrap_or("?")), int(ms(b)), int(w), t, cores, 100. * t / cores));
                } else {
                    answer.push("A medida não traz o tempo de parede do render (render_ms): não dá para comparar com os núcleos.".to_owned());
                }
                let waits: Vec<&Value> = hot["structural"].as_array().into_iter().flatten().filter(|o| ["lock", "sleep"].contains(&o["operation"].as_str().unwrap_or(""))).collect();
                if waits.is_empty() {
                    answer.push("Nenhuma trava ou espera dentro de laço numa função que rodou.".to_owned());
                }
                for o in &waits {
                    let name = short(o["name"].as_str().unwrap_or("?"));
                    answer.push(format!("{name}, linha {}: `{}` ({}) dentro de um laço.", o["line"], o["text"].as_str().unwrap_or(""), if o["operation"] == "lock" { "toma uma trava" } else { "espera parada" }));
                    facts.push(fact(&o["symbol"], format!(
                        "Em {name}, a linha {} faz `{}` dentro de um laço. O que essa trava protege, quem mais a toma, e o que cada volta do laço faz enquanto a segura? Cite a linha {}.",
                        o["line"], o["text"].as_str().unwrap_or(""), o["line"])));
                    changes.push(json!({ "to": "ladder", "symbol": o["symbol"], "nodes": [o["symbol"]], "text": format!(
                        "O render ocupa em média {} de {} núcleos. Em {name}, a linha {} faz `{}` dentro do laço paralelo. Dá para cada volta guardar o seu resultado sem passar por essa trava, sem mudar a imagem nem o resultado dos testes? Proponha até três mudanças diferentes, uma por HYPOTHESIS.",
                        threads.map_or("?".into(), |t| format!("{t:.1}")), cores, o["line"], o["text"].as_str().unwrap_or("")) }));
                }
            }
            Some("per_unit") => {
                let unit = &route["unit"];
                let word = unit["word"].as_str().unwrap_or("?").to_owned();
                let terms: Vec<String> = unit["terms"].as_array().into_iter().flatten().filter_map(|t| t.as_str().map(String::from)).collect();
                // How many units a frame has: the calls of the unit's constructor (Ray::new for raio).
                let units = rows.iter().filter(|r| {
                    let t = tokens(r["name"].as_str().unwrap_or(""));
                    t.last().is_some_and(|l| l == "new") && t.iter().any(|x| terms.contains(x))
                }).map(|r| calls(r)).fold(0., f64::max);
                if units > 0. {
                    answer.push(format!("Um frame medido cria {} {word}s (as chamadas do construtor).", int(units)));
                }
                let floor = if units > 0. { units / 4. } else { FREQUENT };
                let wanted: Vec<&str> = [route["creates"].as_bool() == Some(true), route["sorts"].as_bool() == Some(true)].iter().zip(["create", "sort"])
                    .filter(|(on, _)| **on).map(|(_, k)| k).collect();
                let wanted = if wanted.is_empty() { vec!["create", "sort"] } else { wanted };
                let per: BTreeMap<String, &Value> = rows.iter().filter(|r| calls(r) >= floor).map(|r| (r["symbol"].as_str().unwrap().to_owned(), *r)).collect();
                let (inventory, _, _) = self.inventory_of(key, run)?;
                // An operator (ops::Add, ops::Mul...) creates its result by definition: not a finding.
                let mut found: Vec<(&Value, &librarian_ingest::inventory::Operation)> = inventory.operations.iter()
                    .filter(|o| wanted.contains(&o.operation.as_str()))
                    .filter_map(|o| per.get(&o.symbol).map(|r| (*r, o)))
                    .filter(|(r, _)| !short(r["name"].as_str().unwrap_or("")).starts_with("ops::")).collect();
                found.sort_by(|a, b| calls(b.0).total_cmp(&calls(a.0)).then(a.1.line.cmp(&b.1.line)));
                if found.is_empty() {
                    answer.push(format!("Nas funções que rodam a cada {word}, o inventário não achou nada criado nem ordenado."));
                }
                let verb = |o: &librarian_ingest::inventory::Operation| if o.operation == "sort" { "ordena" } else { "cria" };
                // The same text in the same function, once, with all its lines.
                let mut grouped: Vec<(&Value, &librarian_ingest::inventory::Operation, Vec<usize>)> = vec![];
                for (row, o) in &found {
                    match grouped.iter_mut().find(|(r, g, _)| r["symbol"] == row["symbol"] && g.text == o.text) {
                        Some(g) => g.2.push(o.line),
                        None => grouped.push((row, o, vec![o.line])),
                    }
                }
                for (row, o, lines) in grouped.iter().take(10) {
                    let per_unit = if units > 0. { format!(", {:.1} por {word}", calls(row) / units) } else { String::new() };
                    let at = if lines.len() == 1 { format!("linha {}", lines[0]) } else { format!("linhas {}", lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(", ")) };
                    answer.push(format!("{} ({at}) {} `{}`: {} vezes num frame{per_unit}.", short(row["name"].as_str().unwrap_or("?")), verb(o),
                        o.text.split_whitespace().collect::<Vec<_>>().join(" "), int(calls(row))));
                }
                // One fact and one change per function, by its first operation: the three that run most.
                let mut seen = BTreeSet::new();
                for (row, o) in found.iter().filter(|(r, _)| seen.insert(r["symbol"].as_str().unwrap_or("").to_owned())).take(3) {
                    let name = short(row["name"].as_str().unwrap_or("?"));
                    let text = o.text.split_whitespace().collect::<Vec<_>>().join(" ");
                    facts.push(fact(&row["symbol"], format!(
                        "{name} roda {} vezes num frame. Na linha {} ele {} `{text}`. O que é {} ali, e quem usa isso depois? Cite a linha {}.",
                        int(calls(row)), o.line, verb(o), if o.operation == "sort" { "ordenado" } else { "criado" }, o.line)));
                    changes.push(json!({ "to": "ladder", "symbol": row["symbol"], "nodes": [row["symbol"]], "text": format!(
                        "A cada {word}, {name} {} `{text}` na linha {} ({} vezes num frame). Dá para não {} isso a cada {word} (reaproveitar, adiar ou pular quando não precisa), sem mudar a imagem nem o resultado dos testes? Proponha até três mudanças diferentes, uma por HYPOTHESIS.",
                        verb(o), o.line, int(calls(row)), if o.operation == "sort" { "ordenar" } else { "criar" }) }));
                }
            }
            _ => answer.push("Não reconheci a pergunta. Pergunte, por exemplo: quais funções pesam? onde otimizar? o render usa todos os núcleos? o que é criado a cada raio?".to_owned()),
        }
        Ok(json!({ "route": route, "answer": answer, "facts": facts, "changes": changes, "execution": hot["execution"],
            "limits": ["A rota sai de regras sobre as palavras da pergunta, depois do léxico do projeto; nenhum modelo escolhe.",
                "O tempo somado das threads conta a espera numa trava como tempo ocupado.",
                "Criar ou ordenar é achado pelo nome escrito da chamada (new, collect, clone, sort...); macros como vec![] não aparecem."] }))
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
        assert_eq!(r("O render usa todos os núcleos?")["intent"], "cores");
        let per = r("O que é criado ou ordenado a cada raio?");
        assert_eq!(per["intent"], "per_unit");
        assert_eq!(per["unit"]["word"], "raio");
        assert!(per["unit"]["terms"].as_array().unwrap().contains(&"ray".into()));
        assert_eq!(r("Quais funções pesam?")["intent"], "heavy");
        assert_eq!(r("Onde dá para otimizar?")["intent"], "where");
        let named = r("Otimize o Groups::intersect");
        assert_eq!((named["intent"].as_str(), named["function"].as_str()), (Some("optimize"), Some("Groups::intersect")));
        assert_eq!(r("Bom dia")["intent"], serde_json::Value::Null);
    }
}
