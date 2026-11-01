//! Where a hot line's value comes from. A line that calls the heaviest function on something it
//! reads from `self` (\`self.transform.inverse()\`, \`self.transform().inverse()\`) depends on a
//! field; the declarations that write that field (constructors included, which run while loading) are the
//! next small questions: what value does the field get there? A field read through a method
//! (\`self.transform()\`) is followed into the implementations of that method that ran.
use librarian_ingest::{graph::Graph, owner, Catalog, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// The names read from `self` in a written call: (name, called as a method).
fn self_reads(text: &str) -> Vec<(String, bool)> {
    let mut found = vec![];
    for (i, _) in text.match_indices("self.") {
        let rest = &text[i + 5..];
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if name.is_empty() {
            continue;
        }
        let method = rest[name.len()..].starts_with('(');
        if !found.iter().any(|(n, _)| *n == name) {
            found.push((name, method));
        }
    }
    found
}

/// The writes of the fields `targets` read, each
/// with the line of the write and its text.
fn writers(catalog: &Catalog, graph: &Graph, targets: &[String], field: Option<&str>, through: &str) -> Result<Vec<Value>> {
    let context = owner::owner_context(catalog, graph, targets)?;
    let mut out = vec![];
    // Writers count whether or not they ran in the measure: a constructor runs while loading,
    // before what the measure counts, and is exactly where the value comes from.
    for added in context.added.iter().filter(|a| a.reason == "field_writer") {
        let fields: Vec<&String> = added.fields.iter().filter(|f| field.is_none_or(|w| w == f.as_str())).collect();
        if fields.is_empty() {
            continue;
        }
        // A literal's evidence spans its fields: only the lines that write this one.
        for e in added.evidence.iter().filter(|e| line_of(catalog, &e.path, e.start_line).contains(fields[0].as_str())) {
            let text = line_of(catalog, &e.path, e.start_line);
            out.push(json!({ "symbol": added.symbol_id, "name": added.name, "field": fields[0], "path": e.path, "line": e.start_line,
                "text": text, "through": through }));
        }
    }
    Ok(out)
}

fn line_of(catalog: &Catalog, path: &str, line: usize) -> String {
    catalog.source_bytes(path).map(|b| String::from_utf8_lossy(b).lines().nth(line.saturating_sub(1)).unwrap_or("").trim().to_owned()).unwrap_or_default()
}

/// For the call written at `line` in `caller`: the writes of the fields it reads from `self`.
/// `rows` are the measure's hotspots: they choose the types a trait method is followed into.
pub fn sources(catalog: &Catalog, rows: &[Value], caller: &str, text: &str) -> Result<Value> {
    let graph = Graph::build(catalog);
    let mut found = vec![];
    for (name, method) in self_reads(text) {
        if !method {
            found.extend(writers(catalog, &graph, &[caller.to_owned()], Some(&name), &name)?);
            continue;
        }
        // A method of the caller's trait: the types that ran through that trait the most (the
        // calls counted in their implementations), and, for each, who writes the fields its
        // implementation of the method reads (it may not run itself: an enum reads the field
        // of its variant directly).
        let Some(qualified) = catalog.symbols().get(caller).map(|s| s.qualified_name.clone()) else { continue };
        let tr = qualified.strip_prefix("impl ").and_then(|r| r.split_once(" for ")).map(|(t, _)| t.to_owned())
            .unwrap_or_else(|| qualified.split("::").next().unwrap_or("").to_owned());
        let prefix = format!("impl {tr} for ");
        let mut by_type: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
        for r in rows {
            let Some(rest) = r["name"].as_str().and_then(|n| n.strip_prefix(&prefix)) else { continue };
            let Some((t, _)) = rest.split_once("::") else { continue };
            *by_type.entry(t.to_owned()).or_default() += r["calls"].as_f64().unwrap_or(0.);
        }
        let mut types: Vec<(String, f64)> = by_type.into_iter().filter(|(_, c)| *c > 0.).collect();
        types.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (t, _) in types.into_iter().take(3) {
            let label = format!("impl {tr} for {t}::{name}");
            let Some(own) = catalog.symbols().values().find(|s| s.qualified_name == label) else { continue };
            found.extend(writers(catalog, &graph, &[own.id.clone()], None, &label)?);
        }
    }
    let mut seen = BTreeSet::new();
    found.retain(|w| seen.insert((w["symbol"].as_str().unwrap_or("").to_owned(), w["line"].as_u64())));
    Ok(json!({ "reads": self_reads(text).into_iter().map(|(n, m)| json!({ "name": n, "method": m })).collect::<Vec<_>>(), "writes": found,
        "limits": ["Pelo nome escrito: quem escreve o campo lido, entre as declarações que rodaram na medida. Um campo escrito por outro receptor não é seguido."] }))
}

#[cfg(test)]
mod tests {
    use super::self_reads;

    #[test]
    fn reads_fields_and_methods_from_self() {
        assert_eq!(self_reads("ray.set_transform(self.transform.inverse())"), vec![("transform".into(), false)]);
        assert_eq!(self_reads("original_ray.set_transform(self.transform().inverse())"), vec![("transform".into(), true)]);
        assert!(self_reads("child.intersect(local_ray)").is_empty());
    }
}
