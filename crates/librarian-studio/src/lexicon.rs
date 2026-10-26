//! Lexicon proposals: the code vocabulary comes from the edition (no LLM); an LLM only
//! proposes question words for it, as synonym groups and as stop words; every proposal is
//! checked mechanically before anyone sees it, and nothing is applied without the user's
//! approval. Stop word candidates come from the project's own questions: the terms that
//! touched no result.
use librarian_ingest::{
    search::{tokens, vocabulary, Lexicon, SearchIndex, BUILTIN_STOP_WORDS},
    Catalog,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// How many of the most frequent code terms go into the request.
pub const TERMS: usize = 80;

/// Languages a lexicon request can ask the question words in.
pub const LANGUAGES: [&str; 2] = ["pt", "en"];

/// The query sent to the LLM, in the shape the transports accept (question, evidence, instructions).
/// `language` is the language of the proposed question words, and of the query itself.
/// `missed` are the question terms that touched no result, with how many questions used them.
pub fn request(catalog: &Catalog, current: &Lexicon, language: &str, missed: &[(String, usize)]) -> Value {
    let terms: Vec<Value> = vocabulary(catalog)
        .into_iter()
        .take(TERMS)
        .map(|(term, symbols)| json!({ "term": term, "symbols": symbols }))
        .collect();
    let missed: Vec<Value> = missed.iter().map(|(term, questions)| json!({ "term": term, "questions": questions })).collect();
    if language == "en" {
        return json!({
            "question": "Propose synonym groups and stop words for this project's search lexicon: English words a person would use in a question, each group linked to terms that exist in the code, and the question words that name nothing in the code.",
            "evidence": {
                "code_terms": terms,
                "current_lexicon": current,
                "missed_terms": missed,
                "scope": "code_terms are tokens of the names declared in the code, with how many symbols use them. They do not describe behavior. missed_terms are words of earlier questions that touched no result, with how many questions used them.",
            },
            "instructions": [
                "Answer only with a JSON object, without Markdown: {\"groups\": [[\"word\", \"synonym\", \"code_term\"], ...], \"stop_words\": [\"word\", ...]}.",
                "Each group must contain at least one term copied exactly from evidence.code_terms[].term and at least one English word that is not a code term.",
                "Use single words, without spaces, hyphens or underscores. Include singular and plural when it makes sense (ray, rays).",
                "Propose at most 40 groups, starting with the most important concepts of the domain.",
                "Do not repeat a word in two groups, nor words already in current_lexicon.",
                "Propose only synonyms with the same meaning. Do not invent code terms or group different concepts.",
                "Prefer code terms that name domain concepts; ignore generic names such as get, set, self, test, init.",
                "For each word in evidence.missed_terms decide: a connecting word of questions (is, there, which, located) goes to stop_words; a domain word goes into a group with a code term; otherwise leave it out.",
                "A stop word is removed from every question before the search: never propose a word that names something in the code or that is in a group.",
            ]
        });
    }
    json!({
        "question": "Proponha grupos de sinônimos e stop words para o léxico de busca deste projeto: palavras em português que uma pessoa usaria numa pergunta, cada grupo ligado a termos que existem no código, e as palavras de pergunta que não nomeiam nada do código.",
        "evidence": {
            "code_terms": terms,
            "current_lexicon": current,
            "missed_terms": missed,
            "scope": "code_terms são tokens dos nomes declarados no código, com quantos símbolos os usam. Não descrevem comportamento. missed_terms são palavras de perguntas anteriores que não tocaram nenhum resultado, com quantas perguntas as usaram.",
        },
        "instructions": [
            "Responda somente com um objeto JSON, sem Markdown: {\"groups\": [[\"palavra\", \"sinonimo\", \"termo_do_codigo\"], ...], \"stop_words\": [\"palavra\", ...]}.",
            "Cada grupo precisa conter ao menos um termo copiado exatamente de evidence.code_terms[].term e ao menos uma palavra em português.",
            "Use palavras únicas, sem espaços, hífens nem sublinhados. Inclua singular e plural quando fizer sentido (raio, raios). Acentos são opcionais.",
            "Proponha no máximo 40 grupos, começando pelos conceitos mais importantes do domínio.",
            "Não repita uma palavra em dois grupos nem palavras que já estão em current_lexicon.",
            "Proponha só sinônimos com o mesmo significado. Não invente termos de código nem agrupe conceitos diferentes.",
            "Prefira os termos de código que nomeiam conceitos do domínio; ignore nomes genéricos como get, set, self, test, init.",
            "Para cada palavra de evidence.missed_terms, decida: palavra de ligação de pergunta (fica, existe, sao, quais) vai para stop_words; palavra do domínio entra num grupo com um termo do código; nos outros casos, deixe de fora.",
            "Uma stop word é tirada de toda pergunta antes da busca: nunca proponha palavra que nomeia algo do código ou que está num grupo.",
        ]
    })
}

/// First JSON object in a model's text, tolerating Markdown fences or prose around it.
/// A cut-off answer (token limit) keeps only its complete groups and is marked `recovered`.
pub fn parse(text: &str) -> Result<Value, String> {
    let whole = text.find('{').zip(text.rfind('}')).filter(|(s, e)| s < e);
    if let Some(value) = whole.and_then(|(s, e)| serde_json::from_str::<Value>(&text[s..=e]).ok()) {
        if value["groups"].is_array() || value["stop_words"].is_array() {
            return Ok(value);
        }
    }
    let groups = complete_groups(text);
    if groups.is_empty() {
        return Err("A resposta não tem um JSON com \"groups\" nem grupos completos para aproveitar".into());
    }
    Ok(json!({ "groups": groups, "recovered": true }))
}

/// Innermost `[...]` lists that parse as lists of strings, in order, before `stop_words`
/// (a cut-off stop word list is dropped, never read as a group).
fn complete_groups(text: &str) -> Vec<Vec<String>> {
    let text = text.find("\"stop_words\"").map_or(text, |end| &text[..end]);
    let mut groups = vec![];
    let mut open = None;
    for (i, c) in text.char_indices() {
        match c {
            '[' => open = Some(i),
            ']' => {
                if let Some(start) = open.take() {
                    if let Ok(group) = serde_json::from_str::<Vec<String>>(&text[start..=i]) {
                        groups.push(group);
                    }
                }
            }
            _ => {}
        }
    }
    groups
}

/// Accepts a group only if every kept word is one lexical token, it has two or more words,
/// at least one exists in the code, and no word is already used. Stop words (built in or the
/// project's) are dropped from groups: a question never reaches them, so they would only make
/// connecting words look like synonyms of code terms. The proposal is the
/// current lexicon plus the accepted groups; it must still build a search index.
pub fn check(catalog: &Catalog, current: &Lexicon, proposal: &Value) -> Result<Value, String> {
    let code: BTreeSet<String> = vocabulary(catalog).into_iter().map(|(t, _)| t).collect();
    let mut taken: BTreeSet<String> = current.groups.iter().flatten().flat_map(|w| tokens(w)).collect();
    let mut stops: BTreeSet<String> = tokens(BUILTIN_STOP_WORDS).into_iter().collect();
    stops.extend(current.stop_words.iter().flat_map(|w| tokens(w)));
    let mut merged = current.clone();
    let mut report = vec![];
    for group in proposal["groups"].as_array().into_iter().flatten() {
        let mut words: Vec<String> = vec![];
        let mut notes: Vec<String> = vec![];
        for raw in group.as_array().into_iter().flatten() {
            let Some(raw) = raw.as_str() else {
                notes.push("item que não é texto ignorado".into());
                continue;
            };
            let found = tokens(raw);
            if found.len() != 1 {
                notes.push(format!("\"{raw}\" não é uma palavra única"));
                continue;
            }
            let word = found[0].clone();
            if words.contains(&word) {
                continue;
            }
            if taken.contains(&word) {
                notes.push(format!("\"{word}\" já está em outro grupo"));
                continue;
            }
            if stops.contains(&word) {
                notes.push(format!("\"{word}\" é stop word"));
                continue;
            }
            words.push(word);
        }
        let in_code: Vec<&String> = words.iter().filter(|w| code.contains(*w)).collect();
        let status = if words.len() < 2 {
            notes.push("menos de duas palavras".into());
            "rejected"
        } else if in_code.is_empty() {
            notes.push("nenhuma palavra existe no código".into());
            "rejected"
        } else {
            taken.extend(words.iter().cloned());
            merged.groups.push(words.clone());
            "accepted"
        };
        report.push(json!({ "group": group, "words": words, "code_terms": in_code, "status": status, "notes": notes }));
    }
    let stop_report = check_stop_words(catalog, &mut merged, &taken, proposal);
    SearchIndex::new(catalog, merged.clone()).map_err(|e| format!("Léxico resultante inválido: {e}"))?;
    let accepted = report.iter().filter(|r| r["status"] == "accepted").count();
    let stops_accepted = stop_report.iter().filter(|r| r["status"] == "accepted").count();
    let truncated = proposal["recovered"] == true;
    Ok(json!({ "lexicon": merged, "report": report, "accepted": accepted,
        "rejected": report.len() - accepted, "truncated": truncated,
        "stop_report": stop_report, "stop_accepted": stops_accepted, "stop_rejected": stop_report.len() - stops_accepted }))
}

/// A stop word is dropped from every question, so it must not hide code: it is refused when it
/// is a token of a symbol name, qualified name or file path (what the search weighs), or a word
/// of a synonym group. Appearing only inside code bodies is allowed, with a note.
fn check_stop_words(catalog: &Catalog, merged: &mut Lexicon, taken: &BTreeSet<String>, proposal: &Value) -> Vec<Value> {
    let mut named: BTreeSet<String> = BTreeSet::new();
    let mut bodies: BTreeSet<String> = BTreeSet::new();
    for symbol in catalog.symbols().values() {
        named.extend(tokens(&symbol.name));
        named.extend(tokens(&symbol.qualified_name));
        bodies.extend(tokens(&String::from_utf8_lossy(catalog.excerpt(&symbol.chunk_id).unwrap_or_default())));
    }
    for source in catalog.sources().values() {
        named.extend(tokens(&source.path));
    }
    let mut stops: BTreeSet<String> = tokens(BUILTIN_STOP_WORDS).into_iter().collect();
    stops.extend(merged.stop_words.iter().flat_map(|w| tokens(w)));
    let mut report = vec![];
    for raw in proposal["stop_words"].as_array().into_iter().flatten() {
        let Some(raw) = raw.as_str() else { continue };
        let found = tokens(raw);
        let (status, note) = match found.as_slice() {
            [word] if stops.contains(word) => ("rejected", "já é stop word".to_owned()),
            [word] if taken.contains(word) => ("rejected", format!("\"{word}\" está num grupo de sinônimos")),
            [word] if named.contains(word) => ("rejected", format!("\"{word}\" existe em nomes ou caminhos do código")),
            [word] if bodies.contains(word) => ("accepted", format!("\"{word}\" aparece no corpo do código")),
            [_] => ("accepted", String::new()),
            _ => ("rejected", format!("\"{raw}\" não é uma palavra única")),
        };
        let word = if found.len() == 1 { found[0].clone() } else { raw.to_owned() };
        if status == "accepted" {
            stops.insert(word.clone());
            merged.stop_words.push(word.clone());
        }
        report.push(json!({ "word": word, "status": status, "notes": if note.is_empty() { vec![] } else { vec![note] } }));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::parse;
    use serde_json::json;

    #[test]
    fn cut_off_answers_keep_only_complete_groups() {
        // Shape of a real answer cut by num_predict (done_reason "length").
        let cut = "```json\n{\n  \"groups\": [\n    [\"árvore\", \"tree\"],\n    [\"grafo\", \"graph\"],\n    [\"peso\", \"massa\", \"weight\"],\n    [\"quadrado\", \"retâ";
        let parsed = parse(cut).unwrap();
        assert_eq!(parsed["recovered"], true);
        assert_eq!(parsed["groups"], json!([["árvore", "tree"], ["grafo", "graph"], ["peso", "massa", "weight"]]));
        // A complete answer is read as is, without the recovered mark.
        let whole = parse("{\"groups\": [[\"a\", \"b\"]]}").unwrap();
        assert!(whole.get("recovered").is_none());
        assert!(parse("nada aqui").is_err());
        assert!(parse("{\"outra\": 1}").is_err());
    }
}
