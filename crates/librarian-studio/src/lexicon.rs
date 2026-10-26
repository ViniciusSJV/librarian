//! Lexicon proposals: the code vocabulary comes from the edition (no LLM); an LLM only
//! proposes question words for it; every group is checked mechanically before anyone
//! sees it, and nothing is applied without the user's approval.
use librarian_ingest::{
    search::{tokens, vocabulary, Lexicon, SearchIndex},
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
pub fn request(catalog: &Catalog, current: &Lexicon, language: &str) -> Value {
    let terms: Vec<Value> = vocabulary(catalog)
        .into_iter()
        .take(TERMS)
        .map(|(term, symbols)| json!({ "term": term, "symbols": symbols }))
        .collect();
    if language == "en" {
        return json!({
            "question": "Propose synonym groups for this project's search lexicon: English words a person would use in a question, each group linked to terms that exist in the code.",
            "evidence": {
                "code_terms": terms,
                "current_lexicon": current,
                "scope": "code_terms are tokens of the names declared in the code, with how many symbols use them. They do not describe behavior.",
            },
            "instructions": [
                "Answer only with a JSON object, without Markdown: {\"groups\": [[\"word\", \"synonym\", \"code_term\"], ...]}.",
                "Each group must contain at least one term copied exactly from evidence.code_terms[].term and at least one English word that is not a code term.",
                "Use single words, without spaces, hyphens or underscores. Include singular and plural when it makes sense (ray, rays).",
                "Propose at most 40 groups, starting with the most important concepts of the domain.",
                "Do not repeat a word in two groups, nor words already in current_lexicon.",
                "Propose only synonyms with the same meaning. Do not invent code terms or group different concepts.",
                "Prefer code terms that name domain concepts; ignore generic names such as get, set, self, test, init.",
            ]
        });
    }
    json!({
        "question": "Proponha grupos de sinônimos para o léxico de busca deste projeto: palavras em português que uma pessoa usaria numa pergunta, cada grupo ligado a termos que existem no código.",
        "evidence": {
            "code_terms": terms,
            "current_lexicon": current,
            "scope": "code_terms são tokens dos nomes declarados no código, com quantos símbolos os usam. Não descrevem comportamento.",
        },
        "instructions": [
            "Responda somente com um objeto JSON, sem Markdown: {\"groups\": [[\"palavra\", \"sinonimo\", \"termo_do_codigo\"], ...]}.",
            "Cada grupo precisa conter ao menos um termo copiado exatamente de evidence.code_terms[].term e ao menos uma palavra em português.",
            "Use palavras únicas, sem espaços, hífens nem sublinhados. Inclua singular e plural quando fizer sentido (raio, raios). Acentos são opcionais.",
            "Proponha no máximo 40 grupos, começando pelos conceitos mais importantes do domínio.",
            "Não repita uma palavra em dois grupos nem palavras que já estão em current_lexicon.",
            "Proponha só sinônimos com o mesmo significado. Não invente termos de código nem agrupe conceitos diferentes.",
            "Prefira os termos de código que nomeiam conceitos do domínio; ignore nomes genéricos como get, set, self, test, init.",
        ]
    })
}

/// First JSON object in a model's text, tolerating Markdown fences or prose around it.
/// A cut-off answer (token limit) keeps only its complete groups and is marked `recovered`.
pub fn parse(text: &str) -> Result<Value, String> {
    let whole = text.find('{').zip(text.rfind('}')).filter(|(s, e)| s < e);
    if let Some(value) = whole.and_then(|(s, e)| serde_json::from_str::<Value>(&text[s..=e]).ok()) {
        if value["groups"].is_array() {
            return Ok(value);
        }
    }
    let groups = complete_groups(text);
    if groups.is_empty() {
        return Err("A resposta não tem um JSON com \"groups\" nem grupos completos para aproveitar".into());
    }
    Ok(json!({ "groups": groups, "recovered": true }))
}

/// Innermost `[...]` lists that parse as lists of strings, in order.
fn complete_groups(text: &str) -> Vec<Vec<String>> {
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
/// at least one exists in the code, and no word is already used. The proposal is the
/// current lexicon plus the accepted groups; it must still build a search index.
pub fn check(catalog: &Catalog, current: &Lexicon, proposal: &Value) -> Result<Value, String> {
    let code: BTreeSet<String> = vocabulary(catalog).into_iter().map(|(t, _)| t).collect();
    let mut taken: BTreeSet<String> = current.groups.iter().flatten().flat_map(|w| tokens(w)).collect();
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
    SearchIndex::new(catalog, merged.clone()).map_err(|e| format!("Léxico resultante inválido: {e}"))?;
    let accepted = report.iter().filter(|r| r["status"] == "accepted").count();
    let truncated = proposal["recovered"] == true;
    Ok(json!({ "lexicon": merged, "report": report, "accepted": accepted,
        "rejected": report.len() - accepted, "truncated": truncated }))
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
