//! "Eu pergunto, o Librarian separa": a question typed about performance is recognized by rules
//! over its words (after the project's lexicon), answered from the latest "Medir tudo" and split
//! into small questions: facts for the light model, changes for the ladder. No LLM decides the
//! route; every number in the text comes from the measure.
use crate::instrument::KINDS;
use crate::store::Store;
use librarian_ingest::{search::{tokens, Lexicon}, Catalog, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// The kinds of question, tried in this order; the first whose rule matches wins.
const INTENTS: [&str; 3] = ["optimize", "where", "heavy"];
/// How many functions an answer lists. Only the length of the text: nothing is classified by it.
const LISTED: usize = 5;
/// How to read a measure's numbers: it goes with every answer and every change sent to a model.
const READING: &str = "Como ler: chamadas são a contagem da instrução que o Medir tudo pôs na entrada da função; passagens e contagem observada são de uma linha e podem não ser quantas vezes a função foi chamada; o tempo instrumentado é inclusivo (conta o que a função chama), somado das threads e inflado pela própria contagem, então não é o custo próprio da função nem diz quantas vezes ela rodou.";

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

/// A function a question can name, whether or not any measure has counted it.
pub struct Candidate {
    pub symbol: String,
    pub name: String,
    pub path: String,
}

/// The qualified names the question writes (`Stripe::default`, `Settings.url`). A file is not one:
/// a word with a path separator, or the path of a source of the edition (`src/app.py`, `app.py`).
fn written_qualified<'a>(question: &'a str, functions: &[Candidate]) -> Vec<&'a str> {
    let is_file = |w: &str| w.contains(['/', '\\']) || functions.iter().any(|f| f.path == w || f.path.ends_with(&format!("/{w}")));
    question.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '>')).filter(|w| {
        w.starts_with(char::is_alphabetic) && (w.contains("::") || w.split_once('.').is_some_and(|(_, after)| !after.is_empty())) && !is_file(w)
    }).collect()
}

/// The functions the question asks for by qualified name, one entry each. A name written whole can
/// hold several such words (`ops::Mul<Tuple> for Matrix<4>::mul`): they are that one name.
fn asked_targets(question: &str, functions: &[Candidate]) -> Vec<String> {
    let written = tokens(question);
    let inside = |outer: &[String], inner: &[String]| !inner.is_empty() && outer.windows(inner.len()).any(|w| w == inner);
    let mut targets: Vec<String> = vec![];
    for word in written_qualified(question, functions) {
        let want = tokens(word);
        let whole = functions.iter().filter(|f| split_last(&f.name).is_some())
            .map(|f| (tokens(&f.name), f))
            .filter(|(name, _)| name.len() > want.len() && inside(name, &want) && inside(&written, name))
            .max_by_key(|(name, _)| name.len()).map(|(_, f)| f.name.clone());
        let target = whole.unwrap_or_else(|| word.to_owned());
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets
}

/// A name's parts around its last separator (`::` in Rust, `.` in Python).
fn split_last(name: &str) -> Option<(&str, &str)> {
    name.rsplit_once("::").or_else(|| name.rsplit_once('.'))
}

/// How a function's name can be written with its owner: the type alone, with and without generics,
/// as in `Stripe::default` for `Default for Stripe::default`.
fn owner_forms(name: &str) -> Vec<String> {
    let Some((owner, last)) = split_last(name) else { return vec![] };
    let owner = owner.rsplit_once(" for ").map_or(owner, |(_, kind)| kind);
    let bare = owner.split('<').next().unwrap_or(owner);
    let tail = |o: &str| split_last(o).map_or(o.to_owned(), |(_, t)| t.to_owned());
    [owner, bare].iter().map(|o| format!("{}::{last}", tail(o))).collect()
}

/// Which function the question names, from every function of the edition and nothing else: a measure
/// plays no part, so a function with no count is a target like any other. The name written whole
/// comes first, then with its owner, then its last part alone, which a qualified name never falls
/// back to. Among equals, the longest; several left is an ambiguity, for the person to settle. Several
/// qualified names are settled one by one, and none is dropped on the way.
fn resolve(question: &str, functions: &[Candidate]) -> Value {
    let asked = asked_targets(question, functions);
    if asked.len() > 1 {
        return resolve_each(&asked, functions);
    }
    resolve_text(question, asked.first().map(String::as_str), functions)
}

/// Each name asked for, on its own. All of them settled on one function is that function; anything
/// else is left for the person, with every name accounted for, and nothing is built on only a part.
fn resolve_each(asked: &[String], functions: &[Candidate]) -> Value {
    let parts: Vec<Value> = asked.iter().map(|name| {
        let mut part = resolve_text(name, Some(name), functions);
        part["asked"] = json!(name);
        part
    }).collect();
    let with = |status: &str| parts.iter().filter(|p| p["status"] == status).collect::<Vec<_>>();
    let (missing, open, found) = (with("not_found"), with("ambiguous"), with("resolved"));
    let names = |some: &[&Value]| some.iter().filter_map(|p| p["asked"].as_str()).collect::<Vec<_>>().join(" e ");
    let candidates: Vec<Value> = parts.iter().flat_map(|p| p["candidates"].as_array().cloned().unwrap_or_default()).collect();
    if !missing.is_empty() {
        return json!({ "status": "not_found", "written": names(&missing), "candidates": candidates, "targets": parts });
    }
    if !open.is_empty() {
        return json!({ "status": "ambiguous", "written": names(&open), "candidates": candidates, "targets": parts });
    }
    let symbols: BTreeSet<&str> = found.iter().filter_map(|p| p["symbol"].as_str()).collect();
    if symbols.len() == 1 {
        return parts[0].clone();
    }
    let named: Vec<Value> = found.iter().map(|p| json!({ "symbol": p["symbol"], "name": p["name"], "path": p["path"] })).collect();
    json!({ "status": "ambiguous", "written": names(&found), "candidates": named, "targets": parts })
}

fn resolve_text(question: &str, qualified: Option<&str>, functions: &[Candidate]) -> Value {
    let written = tokens(question);
    let writes = |text: &str| -> Option<usize> { let want = tokens(text); (!want.is_empty() && written.windows(want.len()).any(|w| w == want.as_slice())).then_some(want.len()) };
    let score = |f: &Candidate| -> Option<(u8, usize)> {
        if split_last(&f.name).is_some() {
            if let Some(len) = writes(&f.name) {
                return Some((3, len));
            }
            if let Some(len) = owner_forms(&f.name).iter().filter_map(|o| writes(o)).max() {
                return Some((2, len));
            }
        }
        if qualified.is_some() {
            return None;
        }
        writes(split_last(&f.name).map_or(f.name.as_str(), |(_, last)| last)).map(|len| (1, len))
    };
    let scored: Vec<(&Candidate, (u8, usize))> = functions.iter().filter_map(|f| score(f).map(|s| (f, s))).collect();
    let best = scored.iter().map(|(_, s)| *s).max();
    let chosen: Vec<&Candidate> = scored.iter().filter(|(_, s)| Some(*s) == best).map(|(f, _)| *f).collect();
    let list = |found: &[&Candidate]| found.iter().map(|f| json!({ "symbol": f.symbol, "name": f.name, "path": f.path })).collect::<Vec<_>>();
    match (chosen.as_slice(), qualified) {
        ([only], _) => json!({ "status": "resolved", "symbol": only.symbol, "name": only.name, "path": only.path }),
        ([], Some(written)) => {
            // What shares its last part goes in the answer as a suggestion, never as the target.
            let last = split_last(written).map_or(written, |(_, l)| l);
            let want = tokens(last);
            let near: Vec<&Candidate> = functions.iter().filter(|f| tokens(split_last(&f.name).map_or(f.name.as_str(), |(_, l)| l)) == want).collect();
            json!({ "status": "not_found", "written": written, "candidates": list(&near) })
        }
        ([], None) => json!({ "status": "none" }),
        (many, written) => {
            let shown = written.map(str::to_owned).unwrap_or_else(|| split_last(&many[0].name).map_or(many[0].name.clone(), |(_, l)| l.to_owned()));
            json!({ "status": "ambiguous", "written": shown, "candidates": list(many) })
        }
    }
}

/// The answer when the question names no one function: nothing is measured or sent for it. With
/// several names, each one is answered for.
fn unresolved(target: &Value) -> Vec<String> {
    if let Some(parts) = target["targets"].as_array() {
        let found: Vec<String> = parts.iter().filter(|p| p["status"] == "resolved")
            .map(|p| format!("{} ({})", p["name"].as_str().unwrap_or("?"), p["path"].as_str().unwrap_or("?"))).collect();
        if found.len() == parts.len() {
            return vec![format!("A pergunta nomeia {} funções: {}. O Librarian trata uma por vez: pergunte sobre cada uma.", found.len(), found.join(", "))];
        }
        let mut out: Vec<String> = parts.iter().filter(|p| p["status"] != "resolved").flat_map(unresolved).collect();
        if !found.is_empty() {
            out.push(format!("Também pedida, e encontrada: {}. Nada foi montado com só uma parte do pedido: pergunte sobre uma função por vez.", found.join(", ")));
        }
        return out;
    }
    let found: Vec<&Value> = target["candidates"].as_array().into_iter().flatten().collect();
    let written = target["written"].as_str().unwrap_or("?");
    let names: Vec<String> = found.iter().take(LISTED).map(|f| format!("{} ({})", f["name"].as_str().unwrap_or("?"), f["path"].as_str().unwrap_or("?"))).collect();
    let more = if found.len() > LISTED { format!(", e mais {}", found.len() - LISTED) } else { String::new() };
    if target["status"] == "not_found" {
        let mut out = vec![format!("Não encontrei a função {written} nesta edition. O Librarian não troca o alvo por outra função com nome parecido.")];
        if !names.is_empty() {
            out.push(format!("Com o mesmo último nome: {}{more}. Diga qual, com o nome inteiro.", names.join(", ")));
        }
        return out;
    }
    let same = found.windows(2).all(|w| w[0]["name"] == w[1]["name"]);
    vec![format!("O nome {written} combina com {} funções: {}{more}. {}", found.len(), names.join(", "),
        if same { "Elas têm o mesmo nome, e a pergunta não consegue separá-las pelo arquivo.".to_owned() }
        else { format!("O Librarian não escolhe: diga qual, por exemplo: otimize o {}.", found[0]["name"].as_str().unwrap_or("?")) })]
}

/// The intent, the words that decided it and the function the question names, if any.
pub fn route(question: &str, lexicon: &Lexicon, functions: &[Candidate]) -> Value {
    let words = expand(question, lexicon);
    let hit = |list: &[&str]| -> Vec<String> {
        words.iter().filter_map(|w| w.iter().find(|x| list.contains(&x.as_str())).cloned()).collect()
    };
    let target = resolve(question, functions);
    let named = target["name"].as_str();
    for intent in INTENTS {
        let matched: Vec<String> = match intent {
            "optimize" => if target["status"] != "none" { hit(OPTIMIZE) } else { vec![] },
            "where" => { let (w, i) = (hit(WHERE), hit(IMPROVE)); if w.is_empty() || i.is_empty() { vec![] } else { [w, i].concat() } }
            _ => hit(HEAVY),
        };
        if !matched.is_empty() {
            return json!({ "intent": intent, "matched": matched, "function": named, "target": target });
        }
    }
    json!({ "intent": null, "matched": [], "function": named, "target": target })
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

fn calls(row: &Value) -> f64 {
    row["calls"].as_f64().unwrap_or(0.)
}

/// A count as written in a text, by what its line can be said to be (`count_kind`, in instrument.rs):
/// only the statement the measure put at a function's entry reads as its calls.
fn counted(row: &Value, rows: &[&Value], seen: &str) -> String {
    let at = row["at"].as_str().and_then(|a| a.rsplit_once(':')).map_or("?", |(_, line)| line);
    let n = int(calls(row));
    match row["count_kind"].as_str() {
        Some("entry_calls") => format!("{n} chamadas{seen}"),
        Some("own_counter") => format!("{n} passagens{seen} pela linha {at}, de um contador do próprio projeto (pode não ser quantas vezes a função foi chamada)"),
        Some("several_lines") => {
            let lines = rows.iter().filter(|r| r["symbol"] == row["symbol"]).count();
            format!("{n} passagens{seen} pela linha {at} (uma de {lines} linhas contadas; pode não ser quantas vezes a função foi chamada)")
        }
        _ => format!("contagem observada de {n}"),
    }
}

/// The instrumented time as written in a text. A line with none has none: not zero.
fn time(row: &Value) -> String {
    row["cpu_ms"].as_f64().map_or("tempo instrumentado não registrado".to_owned(), |ms| format!("{} ms de tempo instrumentado", int(ms)))
}

/// What the measure observed in the whole project, for a text that lists only part of it.
fn coverage_note(coverage: &Value) -> String {
    let n = |field: &str| coverage[field].as_u64().unwrap_or(0);
    let no_line = coverage["zero_evidence"]["no_line"].as_u64().unwrap_or(0);
    format!("Esta medição tem contagem ou instrução em {} de {} funções: {} com contagem acima de zero; {} com zero impresso; {no_line} que receberam a instrução e não imprimiram linha; {} sem cobertura. Nenhum desses casos prova que a função não rodou.",
        n("counted"), n("total"), n("counted_and_ran"), n("counted_no_passes").saturating_sub(no_line), n("no_coverage"))
}

/// The two rankings a measure allows, each under its own name; no function here is called the
/// heaviest, and one with no calls observed is in neither.
fn rankings(rows: &[&Value]) -> Vec<String> {
    let ran: Vec<&Value> = rows.iter().copied().filter(|r| calls(r) > 0.).collect();
    if ran.is_empty() {
        return vec!["Nenhuma função teve contagem acima de zero nesta medição.".to_owned()];
    }
    let name = |r: &Value| short(r["name"].as_str().unwrap_or("?")).to_owned();
    let mut timed: Vec<&Value> = ran.iter().copied().filter(|r| r["cpu_ms"].is_number()).collect();
    timed.sort_by(|a, b| b["cpu_ms"].as_f64().unwrap_or(0.).total_cmp(&a["cpu_ms"].as_f64().unwrap_or(0.)));
    let mut out = vec![format!("Maior contagem observada: {}.",
        ran.iter().take(LISTED).map(|r| format!("{} ({}, {})", name(r), counted(r, rows, ""), time(r))).collect::<Vec<_>>().join(", "))];
    if !timed.is_empty() {
        out.push(format!("Maior tempo instrumentado observado, como referência: {}.",
            timed.iter().take(LISTED).map(|r| format!("{} ({}, {})", name(r), time(r), counted(r, rows, ""))).collect::<Vec<_>>().join(", ")));
    }
    out
}

/// What the measure says of a function with no count above zero, by its evidence: never that it did
/// not run, which no measure can say.
fn without_calls(symbol: &str, name: &str, coverage: &Value) -> Vec<String> {
    let found: Vec<&Value> = coverage["functions"].as_array().into_iter().flatten().filter(|f| f["symbol"] == symbol).collect();
    let zeros: BTreeSet<&str> = found.iter().filter_map(|f| f["zero_evidence"].as_str()).collect();
    let states: BTreeSet<&str> = found.iter().filter_map(|f| f["coverage_state"].as_str()).collect();
    let mut out: Vec<String> = zeros.iter().map(|zero| match *zero {
        "entry_zero" => format!("A instrução de entrada de {name} contou zero nesta medição; isso não prova que a função não rodou, porque o programa pode zerar as contagens durante a execução."),
        "no_line" => format!("{name} recebeu a instrução e a medição não imprimiu contagem para ela; isso não prova que a função não rodou."),
        "own_zero" => format!("A linha contada de {name} não teve passagens nesta medição, o que não prova que a função não rodou."),
        "several_zero" => format!("Nenhuma das linhas contadas de {name} teve passagens nesta medição, o que não prova que a função não rodou."),
        _ => format!("{name} tem contagem observada de zero nesta medição, e a origem não permite dizer se a função rodou."),
    }).collect();
    // The median of the repetitions is zero and one of them is not.
    if states.contains("counted_and_ran") {
        out.push(format!("A mediana das repetições de {name} é zero, mas ao menos uma repetição observou passagens. Isso prova que houve passagem; o maior valor não é quantas vezes a função foi chamada."));
    }
    if states.contains("no_coverage") || out.is_empty() {
        out.push(format!("{name} está sem cobertura nesta medição: a medição não observou essa função, o que não quer dizer que ela não rodou, nem que rodou."));
    }
    out
}

impl Store {
    /// The typed question, recognized, answered from the latest "Medir tudo" and split into the
    /// small questions the light model and the ladder receive.
    /// A question the rules do not recognize comes back with no intent: it is for the model. One
    /// they recognize, before any measure, comes back asking for "Medir tudo".
    pub fn performance_question(&self, key: &str, run: &str, question: &str) -> Result<Value> {
        let lexicon = self.settings(key)?.lexicon;
        // The target comes first, from the edition: the measure is read only after it is known.
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let mut functions: Vec<Candidate> = catalog.symbols().values().filter(|s| KINDS.contains(&s.kind.as_str()))
            .map(|s| Candidate { symbol: s.id.clone(), name: short(&s.qualified_name).to_owned(), path: catalog.sources().get(&s.source_id).map_or(String::new(), |src| src.path.clone()) })
            .collect();
        functions.sort_by(|a, b| (&a.name, &a.path, &a.symbol).cmp(&(&b.name, &b.path, &b.symbol)));
        let route = route(question, &lexicon, &functions);
        if matches!(route["intent"].as_str(), Some("where" | "optimize")) && matches!(route["target"]["status"].as_str(), Some("ambiguous" | "not_found")) {
            return Ok(json!({ "route": route, "answer": unresolved(&route["target"]), "facts": [], "changes": [] }));
        }
        if self.latest_measure(key, run)?.is_none() {
            let needs_measure = route["intent"].is_string();
            let manual = self.manual_counts(key, Some(run))?;
            let manual = manual.as_array().map_or(&[][..], |m| m.as_slice());
            let lines: u64 = manual.iter().filter_map(|m| m["lines"].as_u64()).sum();
            let commands: Vec<&str> = manual.iter().filter_map(|m| m["command"].as_str()).collect();
            let answer = if manual.is_empty() {
                "Para responder, o Librarian precisa saber quantas vezes cada função roda, e esta edition não tem essa medição.".to_owned()
            } else {
                format!("Esta edition não tem Medir tudo, de onde esta resposta sai. {} imprimiu contagens manuais em {lines} {}, que ficam fora desta resposta: veja-as em Investigar.",
                    commands.iter().map(|c| format!("«{c}»")).collect::<Vec<_>>().join(", "), if lines == 1 { "linha" } else { "linhas" })
            };
            return Ok(json!({ "route": route, "needs_measure": needs_measure, "facts": [], "changes": [], "answer": [answer] }));
        }
        let hot = self.hotspots(key, run)?;
        let rows: Vec<&Value> = hot["rows"].as_array().into_iter().flatten().filter(|r| r["symbol"].is_string()).collect();
        let coverage = &hot["coverage"];
        let mut facts: Vec<Value> = vec![];
        let mut changes: Vec<Value> = vec![];
        let mut answer: Vec<String> = vec![];
        let fact = |symbol: &Value, text: String| json!({ "to": "qwen", "symbol": symbol, "nodes": [symbol], "text": text });
        match route["intent"].as_str() {
            Some("heavy") => {
                answer.extend(rankings(&rows));
                answer.push(coverage_note(coverage));
            }
            // No function named: the measure gives the numbers, and the choice is the person's.
            Some("where") if route["function"].is_null() => {
                answer.extend(rankings(&rows));
                if let Some(first) = rows.iter().find(|r| calls(r) > 0.).and_then(|r| r["name"].as_str()) {
                    answer.push(format!("O Librarian não escolhe o alvo: diga qual função, por exemplo: otimize o {}.", short(first)));
                }
                answer.push(coverage_note(coverage));
            }
            Some("where") | Some("optimize") => {
                let name = route["function"].as_str().unwrap_or("?");
                let symbol = &route["target"]["symbol"];
                let Some(target) = rows.iter().find(|r| &r["symbol"] == symbol).copied().filter(|r| calls(r) > 0.) else {
                    answer.extend(without_calls(symbol.as_str().unwrap_or_default(), name, coverage));
                    return Ok(json!({ "route": route, "answer": answer, "facts": [], "changes": [], "execution": hot["execution"], "limits": [READING] }));
                };
                // Callers counted at least 5% of its calls; a recursive function may have none, so
                // the most frequent one stays. The two numbers are not the calls from one to the
                // other, and the answer says what it left out.
                let all: Vec<&Value> = target["callers"].as_array().into_iter().flatten().collect();
                let mut callers: Vec<&Value> = all.iter().copied().filter(|c| calls(c) >= calls(target) * 0.05).collect();
                if callers.is_empty() {
                    callers.extend(all.first().copied());
                }
                answer.push(format!("{name}: {} nesta medição, {}.", counted(target, &rows, " observadas"), time(target)));
                if !callers.is_empty() {
                    let left = all.len() - callers.len();
                    answer.push(format!("Quem a chama, pelo nome escrito, com a contagem observada de cada chamador (não as chamadas feitas a ela): {}.{}",
                        callers.iter().map(|c| format!("{} ({})", short(c["name"].as_str().unwrap_or("?")), int(calls(c)))).collect::<Vec<_>>().join(", "),
                        if left > 0 { format!(" Outros {left} chamadores têm contagem menor que 5% da dela e ficaram de fora.") } else { String::new() }));
                }
                for c in &callers {
                    let Some(row) = rows.iter().find(|r| r["symbol"] == c["symbol"]) else { continue };
                    let caller = short(row["name"].as_str().unwrap_or("?"));
                    for line in row["calls_out"].as_array().into_iter().flatten().filter(|l| l["targets"].as_array().into_iter().flatten().any(|t| t["symbol"] == target["symbol"])) {
                        let reaches = if c["homonyms"] == true { "pode chamar, pelo nome escrito," } else { "chama" };
                        let of_target = if target["count_kind"] == "entry_calls" { int(calls(target)) } else { counted(target, &rows, "") };
                        facts.push(fact(&row["symbol"], format!(
                            "{caller} {reaches} {name} na linha {} (`{}`). A medição contou {} de {caller} e {of_target} de {name}. O que essa linha calcula a cada vez, e de quais valores ela depende? Cite a linha {}.",
                            line["line"], line["text"].as_str().unwrap_or(""), counted(row, &rows, ""), line["line"])));
                    }
                }
                let mut nodes = vec![target["symbol"].clone()];
                nodes.extend(callers.iter().map(|c| c["symbol"].clone()));
                changes.push(json!({ "to": "ladder", "symbol": target["symbol"], "nodes": nodes, "text": format!(
                    "{name}: {} nesta medição, {}{}. {READING} {} O que dá para mudar para ela trabalhar menos, sem mudar as saídas nem o resultado dos testes? Proponha até três mudanças diferentes, uma por HYPOTHESIS.",
                    counted(target, &rows, " observadas"), time(target),
                    if callers.is_empty() { String::new() } else { format!("; quem a chama, pelo nome escrito: {}", callers.iter().map(|c| short(c["name"].as_str().unwrap_or("?"))).collect::<Vec<_>>().join(" e ")) },
                    coverage_note(coverage)) }));
            }
            _ => answer.push("Não reconheci a pergunta. Pergunte, por exemplo: quais funções pesam? onde otimizar? otimize o Groups::intersect.".to_owned()),
        }
        Ok(json!({ "route": route, "answer": answer, "facts": facts, "changes": changes, "execution": hot["execution"],
            "limits": ["A rota sai de regras sobre as palavras da pergunta, depois do léxico do projeto; nenhum modelo escolhe.", READING] }))
    }
}

#[cfg(test)]
mod tests {
    use super::{route, Candidate};
    use librarian_ingest::search::Lexicon;
    use serde_json::Value;

    fn functions(names: &[&str]) -> Vec<Candidate> {
        names.iter().enumerate().map(|(i, n)| Candidate { symbol: format!("s{i}"), name: (*n).to_owned(), path: format!("f{i}.rs") }).collect()
    }

    /// The rule on names alone: nothing here knows of any measure.
    #[test]
    fn the_target_is_resolved_from_the_names_alone() {
        let lexicon = Lexicon { groups: vec![], stop_words: vec![] };
        let names = functions(&["Default for Sphere::default", "Default for Stripe::default", "Stripe::color", "Groups::intersect", "intersect", "unica", "Matrix<4>::inverse", "Settings.url", "Mul<Tuple> for Matrix<4>::mul"]);
        let target = |q: &str| route(q, &lexicon, &names)["target"].clone();
        let name = |q: &str| target(q)["name"].as_str().map(str::to_owned);
        // With its owner: the type of an impl, and only that.
        assert_eq!(name("Otimize o Stripe::default").as_deref(), Some("Default for Stripe::default"));
        assert_eq!(name("Otimize o Default for Sphere::default").as_deref(), Some("Default for Sphere::default"));
        assert_eq!(name("Otimize o Matrix::inverse").as_deref(), Some("Matrix<4>::inverse"));
        assert_eq!(name("Otimize o Matrix<4>::mul").as_deref(), Some("Mul<Tuple> for Matrix<4>::mul"));
        // Written whole beats its last part, and a free function of that name.
        assert_eq!(name("Otimize o Groups::intersect?").as_deref(), Some("Groups::intersect"));
        assert_eq!(name("Otimize o Settings.url").as_deref(), Some("Settings.url"));
        // A qualified name never falls back to its last part.
        for q in ["Otimize o Cube::default", "Otimize o Stripe::nada", "Otimize o Outra.url"] {
            assert_eq!(target(q)["status"], "not_found", "{q}");
            assert!(target(q)["symbol"].is_null(), "{q}");
        }
        assert_eq!(target("Otimize o Cube::default")["written"], "Cube::default");
        assert_eq!(target("Otimize o Cube::default")["candidates"].as_array().unwrap().len(), 2);
        assert_eq!(route("Otimize o Cube::default", &lexicon, &names)["intent"], "optimize");
        // Unqualified: several is an ambiguity, one is the target.
        assert_eq!(target("Otimize default")["status"], "ambiguous");
        assert_eq!(target("Otimize default")["candidates"].as_array().unwrap().len(), 2);
        assert_eq!(target("Otimize intersect")["status"], "ambiguous");
        assert_eq!(name("Otimize a unica").as_deref(), Some("unica"));
        assert_eq!(name("Otimize color").as_deref(), Some("Stripe::color"));
        assert_eq!(name("Otimize url").as_deref(), Some("Settings.url"));
        // A part of a name is not the name, and no name at all stays for the model.
        assert_eq!(target("Otimize a unic")["status"], "none");
        assert_eq!(route("Otimize a unic", &lexicon, &names)["intent"], Value::Null);
        assert_eq!(target("A versão 3.5 é boa, otimize")["status"], "none");
        // The same name twice is two functions.
        let twice = functions(&["main", "main"]);
        assert_eq!(route("Otimize main", &lexicon, &twice)["target"]["status"], "ambiguous");
    }

    /// One qualified name asked for, whatever else the question writes.
    #[test]
    fn a_name_written_whole_is_one_target_and_a_file_is_none() {
        let lexicon = Lexicon { groups: vec![], stop_words: vec![] };
        let mut names = functions(&["ops::Mul<Tuple> for Matrix<4>::mul", "Default for Stripe::default", "Tuple::dot"]);
        names[2].path = "src/tuple.rs".to_owned();
        let target = |q: &str| route(q, &lexicon, &names)["target"].clone();
        // A name with `::` in two of its words, as the answers themselves write it.
        let full = target("Otimize o ops::Mul<Tuple> for Matrix<4>::mul");
        assert_eq!((full["status"].as_str(), full["name"].as_str()), (Some("resolved"), Some("ops::Mul<Tuple> for Matrix<4>::mul")));
        assert!(full.get("targets").is_none());
        assert_eq!(target("Otimize o Default for Stripe::default")["name"], "Default for Stripe::default");
        // The file says where, and it is not a function nobody found.
        for q in ["Otimize dot em src/tuple.rs", "Otimize dot em tuple.rs", "Otimize Tuple::dot em src/tuple.rs", "Otimize dot em outro/lugar.rs"] {
            assert_eq!(target(q)["name"], "Tuple::dot", "{q}");
        }
        // Written twice, or in both forms: one function.
        assert_eq!(target("Otimize Tuple::dot e tuple::dot")["name"], "Tuple::dot");
        assert_eq!(target("Otimize Stripe::default, ou Default for Stripe::default")["name"], "Default for Stripe::default");
        // A whole name beside one that is not there is still two names.
        let both = target("Otimize ops::Mul<Tuple> for Matrix<4>::mul e Cube::nada");
        assert_eq!((both["status"].as_str(), both["written"].as_str()), (Some("not_found"), Some("Cube::nada")));
        assert_eq!(both["targets"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn routes_typed_questions_by_words_and_lexicon() {
        let lexicon = Lexicon { groups: vec![vec!["raio".into(), "ray".into()]], stop_words: vec![] };
        let functions = functions(&["Groups::intersect", "World::color_at"]);
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
