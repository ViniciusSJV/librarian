//! P5: a suggested envelope. From the declarations a person chose, the declarations a change to
//! them is likely to need in view, each with why and where the link is written: the owner type
//! and the writers of the fields it reads (G6d), what they call and who calls them without
//! homonyms (P1b), the types they write by name, and the head of their files (the lines before
//! the first declaration, where imports live). Written names only, nothing resolved by a
//! compiler; the person keeps or drops each item before asking.
use crate::{graph::v2, graph::Evidence, owner, Catalog, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// At most this many declarations are suggested beyond the chosen ones; the rest is truncated.
pub const MAX_SUGGESTED: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Suggestion {
    pub items: Vec<Item>,
    /// True when a declaration was left out because of `MAX_SUGGESTED`.
    pub truncated: bool,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Item {
    /// The symbol, or `header:<path>` for the head of a file.
    pub id: String,
    pub path: String,
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub reasons: Vec<Reason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reason {
    /// `chosen`, `owner_type`, `field_writer`, `called`, `return_type` and `called_owner` (of a
    /// called declaration), `caller`, `type_reference` or `header`.
    pub code: String,
    /// The chosen declaration this item is in view for.
    pub for_target: String,
    /// Fields read (owner type) or written (writer).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// Lines where the link is written: the call, the type name, the field write.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<usize>,
}

pub const HEADER_PREFIX: &str = "header:";

/// The head of a file: from its first line to the last non-blank line before its first
/// declaration. None when a declaration starts the file, or the file declares nothing.
pub fn header(catalog: &Catalog, path: &str) -> Option<(usize, usize)> {
    let source = catalog.source_by_path(path)?;
    let first = catalog
        .symbols
        .values()
        .filter(|s| s.source_id == source.id)
        .map(|s| catalog.chunks[&s.chunk_id].start_line)
        .min()?;
    let text = String::from_utf8_lossy(&catalog.snapshots[&source.id]);
    let lines: Vec<&str> = text.lines().collect();
    let end = (1..first).rev().find(|n| lines.get(n - 1).is_some_and(|l| !l.trim().is_empty()))?;
    Some((1, end))
}

pub fn suggest(catalog: &Catalog, chosen: &[String]) -> Result<Suggestion> {
    if chosen.is_empty() || chosen.iter().any(|id| !catalog.symbols.contains_key(id)) {
        return Err("Choose at least one declaration of this edition".into());
    }
    let mut suggestion = Suggestion { items: vec![], truncated: false, limits: vec![] };
    let qualified = |id: &str| catalog.symbols[id].qualified_name.clone();
    for id in chosen {
        push(catalog, &mut suggestion, id, Reason { code: "chosen".into(), for_target: qualified(id), fields: vec![], lines: vec![] }, true);
    }
    let lines = |evidence: &[Evidence]| {
        let mut l: Vec<usize> = evidence.iter().map(|e| e.start_line).collect();
        l.sort_unstable();
        l.dedup();
        l
    };

    // Owner type and field writers, where the profile records owner syntax.
    let base = crate::graph::Graph::build(catalog);
    if catalog.registry.owner_context {
        let context = owner::owner_context(catalog, &base, chosen)?;
        for added in context.added {
            let reason = Reason { code: added.reason.clone(), for_target: added.for_target.clone(), fields: added.fields.clone(), lines: lines(&added.evidence) };
            push(catalog, &mut suggestion, &added.symbol_id, reason, false);
        }
        if context.truncated {
            suggestion.truncated = true;
        }
    }

    let graph = v2::Graph::build(catalog);
    let syntax = catalog.call_syntax();
    for id in chosen {
        let target = qualified(id);
        // What it calls, when the written name points to one declaration.
        let calls = graph.calls_from(id, 1, 64, &syntax, &BTreeMap::new())?;
        for link in calls.links.iter().filter(|l| &l.from == id && matches!(l.resolution.as_str(), "unique" | "receiver")) {
            let reason = Reason { code: "called".into(), for_target: target.clone(), fields: vec![], lines: vec![link.evidence.start_line] };
            push(catalog, &mut suggestion, &link.to, reason, false);
            // A change that keeps what the call returns needs the type it returns.
            for (returned, line) in return_types(&graph, &link.to) {
                if returned != *id {
                    let reason = Reason { code: "return_type".into(), for_target: qualified(&link.to), fields: vec![], lines: vec![line] };
                    push(catalog, &mut suggestion, &returned, reason, false);
                }
            }
            // A called method's own type: what a returned `Self`, or a stored result, is.
            if let Some((owner, evidence)) = owner::owner_type(catalog, &base, &link.to) {
                if owner != *id {
                    let reason = Reason { code: "called_owner".into(), for_target: qualified(&link.to), fields: vec![], lines: lines(&evidence) };
                    push(catalog, &mut suggestion, &owner, reason, false);
                }
            }
        }
        // Who calls it, when the call's name matches it alone; tests are left out.
        let language = &catalog.sources[&catalog.symbols[id].source_id].language;
        let kinds = syntax.languages.get(language).map(|(k, _)| k.clone()).unwrap_or_default();
        let users = graph.used_by(id, 1, 64)?;
        for link in users.links.iter().filter(|l| l.to == *id && l.candidates == 1 && kinds.contains(&l.occurrence_kind.as_str())) {
            if catalog.symbols.get(&link.from).is_none_or(|s| s.is_test) {
                continue;
            }
            let reason = Reason { code: "caller".into(), for_target: target.clone(), fields: vec![], lines: vec![link.evidence.start_line] };
            push(catalog, &mut suggestion, &link.from, reason, false);
        }
        // Types it writes by name (signature, annotations, paths), when one declaration has the name.
        for edge in graph.edges.iter().filter(|e| e.kind == "observes" && e.from == *id) {
            let occurrence = &graph.nodes[&edge.to];
            if occurrence.kind != "type_reference" {
                continue;
            }
            let candidates: Vec<&v2::Edge> = graph.edges.iter().filter(|e| e.kind == "name_candidate" && e.from == edge.to).collect();
            if let [one] = candidates.as_slice() {
                if one.to != *id {
                    let reason = Reason { code: "type_reference".into(), for_target: target.clone(), fields: vec![], lines: vec![one.evidence.start_line] };
                    push(catalog, &mut suggestion, &one.to, reason, false);
                }
            }
        }
    }

    // The head of each chosen declaration's file: a change may need an import there.
    let mut paths: Vec<String> = vec![];
    for id in chosen {
        let path = catalog.sources[&catalog.symbols[id].source_id].path.clone();
        if paths.contains(&path) {
            continue;
        }
        if let Some((start, end)) = header(catalog, &path) {
            suggestion.items.push(Item {
                id: format!("{HEADER_PREFIX}{path}"), path: path.clone(), name: path.clone(), kind: "header".into(),
                start_line: start, end_line: end,
                reasons: vec![Reason { code: "header".into(), for_target: qualified(id), fields: vec![], lines: vec![] }],
            });
        }
        paths.push(path);
    }
    suggestion.limits = vec![
        "Suggested envelope: written names only (owner, field writes, calls, callers, type names, file heads); nothing was resolved by a compiler, and what is not written by name is not found.".into(),
        "Callers and types are added only when the written name matches one declaration; homonyms and tests are left out.".into(),
    ];
    Ok(suggestion)
}

/// The types a declaration's written return type names, when one declaration has the name: the
/// type references inside its `declared_return` occurrence. With the line they are written on.
fn return_types(graph: &v2::Graph, symbol: &str) -> Vec<(String, usize)> {
    let observed: Vec<&v2::Edge> = graph.edges.iter().filter(|e| e.kind == "observes" && e.from == symbol).collect();
    let returns: Vec<&Evidence> = observed.iter().filter(|e| graph.nodes[&e.to].kind == "declared_return").map(|e| &e.evidence).collect();
    let mut found = vec![];
    for edge in observed.iter().filter(|e| graph.nodes[&e.to].kind == "type_reference") {
        let inside = returns.iter().any(|r| r.start_byte <= edge.evidence.start_byte && edge.evidence.end_byte <= r.end_byte);
        if !inside {
            continue;
        }
        let candidates: Vec<&v2::Edge> = graph.edges.iter().filter(|e| e.kind == "name_candidate" && e.from == edge.to).collect();
        if let [one] = candidates.as_slice() {
            found.push((one.to.clone(), edge.evidence.start_line));
        }
    }
    found
}

/// Adds an item, or another reason to an item already suggested.
/// A chosen declaration keeps only that reason: links between chosen ones add nothing to see.
fn push(catalog: &Catalog, suggestion: &mut Suggestion, id: &str, reason: Reason, chosen: bool) {
    if let Some(item) = suggestion.items.iter_mut().find(|i| i.id == id) {
        let was_chosen = item.reasons.iter().any(|r| r.code == "chosen");
        if !was_chosen && !item.reasons.contains(&reason) {
            item.reasons.push(reason);
        }
        return;
    }
    let added = suggestion.items.iter().filter(|i| !i.reasons.iter().any(|r| r.code == "chosen")).count();
    if !chosen && added >= MAX_SUGGESTED {
        suggestion.truncated = true;
        return;
    }
    let Some(symbol) = catalog.symbols.get(id) else { return };
    let chunk = &catalog.chunks[&symbol.chunk_id];
    suggestion.items.push(Item {
        id: id.to_owned(),
        path: catalog.sources[&symbol.source_id].path.clone(),
        name: symbol.qualified_name.clone(),
        kind: symbol.kind.clone(),
        start_line: chunk.start_line,
        end_line: chunk.end_line,
        reasons: vec![reason],
    });
}
