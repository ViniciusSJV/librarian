//! LACUNA: what a model's answer says is missing, and the declarations of the edition that answer
//! it. The texts are the consumer's to read (a send, an agent's request); the names in them, the
//! declarations they point to and whether what a question added is what they asked for are the
//! builder's. Nothing here judges the answer itself.
use crate::keeper::Keeper;
use librarian_ingest::{evidence::ServerAddition, Catalog, Result};

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
/// first; when two traits declare it, nothing is guessed). Tests and what is already gathered
/// stay out.
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
    // A trait ran through its own methods or through an `impl` of it.
    let ran_type = |t: &str| ran.is_empty() || ran.iter().any(|q| q.starts_with(&format!("impl {t}::")) || q.contains(&format!(" for {t}::"))
        || q.starts_with(&format!("{t}::")) || q.starts_with(&format!("impl {t} for ")));
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
        let own: Vec<&&&librarian_ingest::Symbol> = methods.iter().filter(|s| s.kind == "trait_method").collect();
        if let [one] = own.as_slice() {
            add(&one.id, name, &mut out);
        } else if own.is_empty() && methods.len() <= 3 {
            for s in methods {
                add(&s.id, name, &mut out);
            }
        }
    }
    out
}

/// What a question's lacuna rounds added, as it records them: each the server's, once, after the
/// client's (the selection's included sources, when there is one), in the order added; each asked
/// for by a LACUNA of the send it names, and found for it in this edition by `resolve`. The rule is
/// read with nothing excluded by the measure, which only ever leaves out: a declaration it gave then
/// is one it gives now. The LACUNA texts come from the keeper.
pub fn check_additions(keeper: &dyn Keeper, catalog: &Catalog, nodes: &[String], included: Option<&[String]>, additions: &[ServerAddition]) -> Result<()> {
    let ids: Vec<&String> = additions.iter().map(|a| &a.symbol_id).collect();
    let Some(split) = nodes.len().checked_sub(ids.len()) else { return Err("mais complementações do que declarações".into()) };
    let (client, added) = nodes.split_at(split);
    // With a selection, what is not the server's is exactly what the client sent: nothing more.
    if included.is_some_and(|included| included != client) {
        return Err("as declarações da cliente não são as da seleção".into());
    }
    if additions.is_empty() {
        return Ok(());
    }
    if added.iter().ne(ids.iter().copied()) {
        return Err("as declarações acrescentadas não são as últimas da pergunta, na ordem".into());
    }
    for (n, id) in ids.iter().enumerate() {
        if ids[..n].contains(id) || client.contains(id) {
            return Err(format!("{id} já estava na pergunta").into());
        }
    }
    // Round by round: what one send's LACUNA asked for, over what was gathered before it.
    let mut start = 0;
    while start < additions.len() {
        let first = &additions[start];
        let end = start + additions[start..].iter().take_while(|a| (a.round, &a.from_turn, &a.from_send) == (first.round, &first.from_turn, &first.from_send)).count();
        let lacunas = keeper.lacunas(&first.from_turn, &first.from_send)?;
        let names: Vec<String> = lacunas.iter().flat_map(|t| lacuna_names(t)).collect();
        let before: Vec<String> = client.iter().chain(&added[..start]).cloned().collect();
        let found = resolve(catalog, &names, &before, &[], usize::MAX);
        for addition in &additions[start..end] {
            let named = names.contains(&addition.for_name) || addition.for_name.split_once("::").is_some_and(|(t, _)| names.iter().any(|n| n == t));
            if addition.round == 0 || !named || !found.iter().any(|(id, _)| *id == addition.symbol_id) {
                return Err(format!("{} não é o que a LACUNA de {}/{} pediu", addition.symbol_id, first.from_turn, first.from_send).into());
            }
        }
        start = end;
    }
    Ok(())
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
}
