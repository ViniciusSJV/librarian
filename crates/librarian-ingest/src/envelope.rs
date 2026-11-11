//! P5: the envelope. From the declarations a person chose, every declaration a change to them
//! may need in view, each with why and where the link is written: the owner type and the writers
//! of the fields it reads (G6d), what they call and who calls them without homonyms (P1b), the
//! types they write by name, and the head of their files (the lines before the first declaration,
//! where imports live). Written names only, nothing resolved by a compiler.
//!
//! `select` then decides what goes: each candidate gets a role by fixed rules over the graph and
//! the measure, the roles fill a budget in bytes, and what stays out says why. The person keeps
//! or drops each item before asking.
use crate::{graph::v2, graph::Evidence, inventory::Inventory, owner, Catalog, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The version of the rules below, as a suggestion's record names it. A change in what `select`,
/// `candidates` or `behavior` decide for the same input is a new version; on its own it does not
/// bring back the rules it names.
pub const RULES_VERSION: &str = "envelope-1";

/// The code a selection may carry, in bytes. The chosen declarations always go, even past it.
pub const BUDGET_BYTES: usize = 6 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Candidates {
    pub items: Vec<Item>,
    /// What the graph could not settle about the chosen declarations.
    pub gaps: Vec<Gap>,
    pub limits: Vec<String>,
}

/// Something a change may need that the selection cannot vouch for, found without a model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gap {
    /// `homonym_call`, `homonym_type` (the written name matches several declarations),
    /// `write_not_followed` (a read field written through a receiver of unknown type),
    /// `field_without_writer` or `origin_left_out` (an origin that did not fit).
    pub code: String,
    pub for_target: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// The declarations it may be; with a measure, those that ran when any did.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
    /// How many declarations the name matches, also past the graph's cap on candidates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

/// Something written in a declaration that a change must preserve or may depend on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Behavior {
    pub name: String,
    /// `loop`, `operation` (lock, I/O, a module write…), `state_write` or `test`.
    pub kind: String,
    pub path: String,
    pub line: usize,
    pub text: String,
    /// The operation's kind, or the loops around a loop.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
}

/// What a "Medir tudo" says about the functions of an edition.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Measure {
    /// Every function that got the count: one with no calls did not run in the measure.
    pub counted: BTreeSet<String>,
    pub calls: BTreeMap<String, f64>,
}

impl Measure {
    pub fn ran(&self, id: &str) -> bool {
        self.calls.get(id).is_some_and(|c| *c > 0.)
    }
    fn did_not_run(&self, id: &str) -> bool {
        self.counted.contains(id) && !self.ran(id)
    }
}

/// A line for the model about the evidence. With `id`, it holds only while that declaration is
/// in the envelope; a person may have taken it out.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Note {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    pub text: String,
}

/// A candidate with what the selection decided about it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    #[serde(flatten)]
    pub item: Item,
    /// `target` (chosen), `origin` (where its values come from and can be kept), `cost` (what it
    /// calls that ran), `path` (who reaches it), `support` (types, heads, what it calls with no
    /// measure) or `constraint` (a test that calls it).
    pub role: String,
    pub bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calls: Option<f64>,
    /// Why it stayed out: `did_not_run`, `test` (the gate runs the tests), `budget` or `max_items`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_out: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Selection {
    pub items: Vec<Entry>,
    pub left_out: Vec<Entry>,
    pub gaps: Vec<Gap>,
    /// Filled by `behavior`, for the declarations being changed and what they cost.
    pub behavior: Vec<Behavior>,
    /// Filled by `notes`: what of this selection goes to the model.
    pub notes: Vec<Note>,
    pub budget_bytes: usize,
    pub used_bytes: usize,
    /// False when no measure was available and the order comes from the structure alone.
    pub measured: bool,
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
    /// called declaration), `caller`, `test_caller`, `type_reference`, `trait_impl` or `header`.
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

/// Every declaration a change to the chosen ones may need in view, with why; nothing is cut here.
pub fn candidates(catalog: &Catalog, chosen: &[String], measure: Option<&Measure>) -> Result<Candidates> {
    if chosen.is_empty() || chosen.iter().any(|id| !catalog.symbols.contains_key(id)) {
        return Err("Choose at least one declaration of this edition".into());
    }
    let mut suggestion = Candidates { items: vec![], gaps: vec![], limits: vec![] };
    let qualified = |id: &str| catalog.symbols[id].qualified_name.clone();
    for id in chosen {
        push(catalog, &mut suggestion, id, Reason { code: "chosen".into(), for_target: qualified(id), fields: vec![], lines: vec![] });
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
        let context = owner::owner_context_up_to(catalog, &base, chosen, usize::MAX)?;
        let is_test = |path: &str, line: usize| catalog.symbols.values().any(|s| s.is_test && catalog.sources[&s.source_id].path == path
            && catalog.chunks[&s.chunk_id].start_line <= line && line <= catalog.chunks[&s.chunk_id].end_line);
        // One gap per field: the writes are often many, and the point is that they exist.
        let mut unfollowed: BTreeMap<&str, Vec<&owner::NotFollowed>> = BTreeMap::new();
        for write in context.not_followed.iter().filter(|w| !is_test(&w.path, w.line)) {
            unfollowed.entry(&write.field).or_default().push(write);
        }
        for (field, writes) in unfollowed {
            let shown: Vec<String> = writes.iter().take(3).map(|w| format!("{}:{} {}", w.path, w.line, w.text)).collect();
            suggestion.gaps.push(Gap { code: "write_not_followed".into(), for_target: field.into(), text: shown.join(", "), path: String::new(), line: None, candidates: vec![], count: Some(writes.len()) });
        }
        // A field read whose writers were all looked for and none was found.
        for owner in context.added.iter().filter(|a| a.reason == "owner_type") {
            for field in &owner.fields {
                if !context.added.iter().any(|a| a.reason == "field_writer" && a.for_target == owner.for_target && a.fields.contains(field)) {
                    suggestion.gaps.push(Gap { code: "field_without_writer".into(), for_target: owner.for_target.clone(), text: field.clone(), path: String::new(), line: None, candidates: vec![], count: None });
                }
            }
        }
        for added in context.added {
            let reason = Reason { code: added.reason.clone(), for_target: added.for_target.clone(), fields: added.fields.clone(), lines: lines(&added.evidence) };
            push(catalog, &mut suggestion, &added.symbol_id, reason);
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
            push(catalog, &mut suggestion, &link.to, reason);
            // A change that keeps what the call returns needs the type it returns.
            for (returned, line) in return_types(&graph, &link.to) {
                if returned != *id {
                    let reason = Reason { code: "return_type".into(), for_target: qualified(&link.to), fields: vec![], lines: vec![line] };
                    push(catalog, &mut suggestion, &returned, reason);
                }
            }
            // A called method's own type: what a returned `Self`, or a stored result, is.
            if let Some((owner, evidence)) = owner::owner_type(catalog, &base, &link.to) {
                if owner != *id {
                    let reason = Reason { code: "called_owner".into(), for_target: qualified(&link.to), fields: vec![], lines: lines(&evidence) };
                    push(catalog, &mut suggestion, &owner, reason);
                }
            }
        }
        // A call whose name has homonyms: settled when the measure saw one of them run, else a gap.
        let mut homonyms: BTreeMap<&str, Vec<&v2::CallLink>> = BTreeMap::new();
        for link in calls.links.iter().filter(|l| &l.from == id && l.resolution == "homonym") {
            homonyms.entry(&link.occurrence).or_default().push(link);
        }
        for links in homonyms.values() {
            let ran: Vec<&&v2::CallLink> = links.iter().filter(|l| measure.is_some_and(|m| m.ran(&l.to))).collect();
            if let [one] = ran.as_slice() {
                let reason = Reason { code: "called_measured".into(), for_target: target.clone(), fields: vec![], lines: vec![one.evidence.start_line] };
                push(catalog, &mut suggestion, &one.to, reason);
                continue;
            }
            let shown: Vec<&&v2::CallLink> = if ran.is_empty() { links.iter().collect() } else { ran };
            suggestion.gaps.push(Gap { code: "homonym_call".into(), for_target: target.clone(), text: links[0].text.clone(), path: links[0].evidence.path.clone(), line: Some(links[0].evidence.start_line),
                candidates: shown.iter().filter_map(|l| catalog.symbols.get(&l.to).map(|s| s.qualified_name.clone())).collect(), count: Some(links[0].candidates) });
        }
        // Names past the graph's cap on homonyms have no candidates to choose from at all.
        let call_kinds = syntax.languages.get(&catalog.sources[&catalog.symbols[id].source_id].language).map(|(k, _)| k.clone()).unwrap_or_default();
        for edge in graph.edges.iter().filter(|e| e.kind == "observes" && e.from == *id) {
            let occurrence = &graph.nodes[&edge.to];
            if let (Some(count), true) = (occurrence.ambiguous_candidates, call_kinds.contains(&occurrence.kind.as_str())) {
                suggestion.gaps.push(Gap { code: "homonym_call".into(), for_target: target.clone(), text: occurrence.label.clone(), path: edge.evidence.path.clone(), line: Some(edge.evidence.start_line), candidates: vec![], count: Some(count) });
            }
        }
        // Who calls it, when the call's name matches it alone.
        let language = &catalog.sources[&catalog.symbols[id].source_id].language;
        let kinds = syntax.languages.get(language).map(|(k, _)| k.clone()).unwrap_or_default();
        let users = graph.used_by(id, 1, 64)?;
        for link in users.links.iter().filter(|l| l.to == *id && l.candidates == 1 && kinds.contains(&l.occurrence_kind.as_str())) {
            let Some(caller) = catalog.symbols.get(&link.from) else { continue };
            let code = if caller.is_test { "test_caller" } else { "caller" };
            let reason = Reason { code: code.into(), for_target: target.clone(), fields: vec![], lines: vec![link.evidence.start_line] };
            push(catalog, &mut suggestion, &link.from, reason);
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
                    push(catalog, &mut suggestion, &one.to, reason);
                }
            } else if candidates.len() > 1 {
                suggestion.gaps.push(Gap { code: "homonym_type".into(), for_target: target.clone(), text: occurrence.label.clone(), path: edge.evidence.path.clone(), line: Some(edge.evidence.start_line),
                    candidates: candidates.iter().filter_map(|c| catalog.symbols.get(&c.to).map(|s| s.qualified_name.clone())).collect(), count: Some(candidates.len()) });
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
    // The same name written twice in a declaration is one question about it.
    let mut seen = BTreeSet::new();
    suggestion.gaps.retain(|g| seen.insert((g.code.clone(), g.for_target.clone(), g.text.clone())));
    suggestion.limits = vec![
        "Suggested envelope: written names only (owner, field writes, calls, callers, type names, file heads); nothing was resolved by a compiler, and what is not written by name is not found.".into(),
        "Callers and types are added only when the written name matches one declaration; homonyms are left out.".into(),
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
fn push(catalog: &Catalog, suggestion: &mut Candidates, id: &str, reason: Reason) {
    if let Some(item) = suggestion.items.iter_mut().find(|i| i.id == id) {
        let was_chosen = item.reasons.iter().any(|r| r.code == "chosen");
        if !was_chosen && !item.reasons.contains(&reason) {
            item.reasons.push(reason);
        }
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

/// A trait's default method runs for the types that reach it: for each chosen default, the
/// implementation of that trait for every receiver type the inventory's chains reach it by
/// (`child.intersect` with `child` in `Vec<Object>` gives `impl Intersectable for Object`).
/// Without it, a change to the default cannot see where the type's own data lives. With a
/// measure, only the calls made from a function that ran count.
pub fn trait_impls(catalog: &Catalog, found: &Inventory, chosen: &[String], measure: Option<&Measure>, candidates: &mut Candidates) {
    let steps: Vec<&crate::inventory::Step> = found.loops.iter()
        .flat_map(|l| l.through_calls.iter().map(|c| &c.steps).chain(l.loops_through_calls.iter().map(|c| &c.steps)))
        .flatten().collect();
    for default in chosen.iter().filter_map(|id| catalog.symbols.get(id)).filter(|s| s.kind == "trait_method") {
        let Some((trait_name, _)) = default.qualified_name.split_once("::") else { continue };
        let mut seen = vec![];
        for step in steps.iter().filter(|s| s.to == default.id && s.resolution == "trait_default" && measure.is_none_or(|m| m.ran(&s.from))) {
            for t in &step.receiver_types {
                let label = format!("impl {trait_name} for {t}");
                if seen.contains(&label) {
                    continue;
                }
                seen.push(label.clone());
                let Some(implementation) = catalog.symbols.values().find(|s| s.kind == "impl" && s.qualified_name == label) else { continue };
                if candidates.items.iter().any(|i| i.id == implementation.id) {
                    continue;
                }
                let chunk = &catalog.chunks[&implementation.chunk_id];
                candidates.items.push(Item {
                    id: implementation.id.clone(), path: catalog.sources[&implementation.source_id].path.clone(), name: label,
                    kind: "impl".into(), start_line: chunk.start_line, end_line: chunk.end_line,
                    reasons: vec![Reason { code: "trait_impl".into(), for_target: default.qualified_name.clone(), fields: vec![t.clone()], lines: vec![step.line] }],
                });
            }
        }
    }
}

/// The role of a candidate and its place in the order: the best one among its reasons.
fn role_of(catalog: &Catalog, item: &Item, measure: Option<&Measure>) -> (&'static str, u8) {
    // A test stays a test, even when it builds the type or writes the field it checks.
    if catalog.symbols.get(&item.id).is_some_and(|s| s.is_test) {
        return ("constraint", 5);
    }
    item.reasons.iter().map(|r| match r.code.as_str() {
        "chosen" => ("target", 0),
        "owner_type" | "field_writer" | "trait_impl" => ("origin", 1),
        "header" => ("support", 1),
        "called" | "called_measured" if measure.is_some_and(|m| m.ran(&item.id)) => ("cost", 2),
        "caller" => ("path", 3),
        _ => ("support", 4),
    }).min_by_key(|(_, rank)| *rank).unwrap_or(("support", 4))
}

fn bytes_of(catalog: &Catalog, item: &Item) -> usize {
    let Some(symbol) = catalog.symbols.get(&item.id) else {
        // The head of a file is a range of lines.
        let Some(source) = catalog.source_by_path(&item.path) else { return 0 };
        return String::from_utf8_lossy(&catalog.snapshots[&source.id]).lines().take(item.end_line).map(|l| l.len() + 1).sum();
    };
    let chunk = &catalog.chunks[&symbol.chunk_id];
    chunk.end_byte - chunk.start_byte
}

/// What goes to the model. The rules, in order: the chosen declarations always go; a function
/// that the measure counted and that did not run stays out unless it is the origin of a value (a
/// constructor runs before the measured part, and it is where a computed value can be kept); tests
/// stay out because the gate runs them; the rest fills the budget by role, the most called first
/// among costs and paths, skipping what does not fit. Nothing is scored by a model.
pub fn select(catalog: &Catalog, candidates: &Candidates, measure: Option<&Measure>, budget: usize, max_items: usize) -> Selection {
    let mut entries: Vec<(u8, Entry)> = candidates.items.iter().map(|item| {
        let (role, rank) = role_of(catalog, item, measure);
        let calls = measure.and_then(|m| m.calls.get(&item.id).copied());
        (rank, Entry { item: item.clone(), role: role.into(), bytes: bytes_of(catalog, item), calls, left_out: None })
    }).collect();
    // The sort is stable: within a rank, candidates keep the order the graph gave them.
    entries.sort_by(|(a, x), (b, y)| a.cmp(b).then(y.calls.unwrap_or(0.).total_cmp(&x.calls.unwrap_or(0.))));
    let mut selection = Selection { items: vec![], left_out: vec![], gaps: candidates.gaps.clone(), behavior: vec![], notes: vec![], budget_bytes: budget, used_bytes: 0, measured: measure.is_some(), limits: candidates.limits.clone() };
    for (rank, mut entry) in entries {
        let cold = rank >= 2 && measure.is_some_and(|m| m.did_not_run(&entry.item.id));
        entry.left_out = if rank == 0 {
            None
        } else if rank == 5 {
            Some("test".into())
        } else if cold {
            Some("did_not_run".into())
        } else if selection.items.len() >= max_items {
            Some("max_items".into())
        } else if selection.used_bytes + entry.bytes > budget {
            Some("budget".into())
        } else {
            None
        };
        if entry.left_out.is_some() {
            if rank == 1 && entry.item.kind != "header" {
                selection.gaps.push(Gap { code: "origin_left_out".into(), for_target: entry.item.reasons[0].for_target.clone(), text: entry.item.name.clone(), path: entry.item.path.clone(), line: Some(entry.item.start_line), candidates: vec![], count: None });
            }
            selection.left_out.push(entry);
        } else {
            selection.used_bytes += entry.bytes;
            selection.items.push(entry);
        }
    }
    selection.limits.push("Behavior covers loops, inventory operations, field writes and tests the graph sees; conditions, alternative paths and invariants are not extracted, and calls inside macros are not seen.".into());
    if measure.is_none() {
        selection.limits.push("No measure of this edition: the order comes from the structure alone, and what did not run cannot be told apart.".into());
    }
    selection
}

/// What the selected targets and costs do that a change must keep in mind, as written: their
/// loops, the inventory's operations other than arithmetic, the fields they write and the tests
/// that call them. Nothing here says the behavior matters, only that it is there.
pub fn behavior(catalog: &Catalog, found: &Inventory, selection: &Selection) -> Vec<Behavior> {
    let graph = v2::Graph::build(catalog);
    let mut out = vec![];
    for entry in selection.items.iter().filter(|e| matches!(e.role.as_str(), "target" | "cost")) {
        let id = &entry.item.id;
        let name = &entry.item.name;
        // From the graph: the inventory lists only the loops that hold an operation.
        let observed: Vec<&v2::Edge> = graph.edges.iter().filter(|e| e.kind == "observes" && e.from == *id).collect();
        let loops: Vec<&&v2::Edge> = observed.iter().filter(|e| graph.nodes[&e.to].kind == "loop").collect();
        for edge in &loops {
            let (start, end) = (edge.evidence.start_byte, edge.evidence.end_byte);
            let nesting = loops.iter().filter(|o| o.evidence.start_byte < start && end <= o.evidence.end_byte).count();
            let detail = if nesting > 0 { format!("nesting {nesting}") } else { String::new() };
            let head: String = graph.nodes[&edge.to].label.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(100).collect();
            out.push(Behavior { name: name.clone(), kind: "loop".into(), path: edge.evidence.path.clone(), line: edge.evidence.start_line, text: head, detail });
        }
        for o in found.operations.iter().filter(|o| &o.symbol == id && o.operation != "math") {
            out.push(Behavior { name: name.clone(), kind: "operation".into(), path: o.path.clone(), line: o.line, text: o.text.clone(), detail: o.operation.clone() });
        }
        let Some(symbol) = catalog.symbols.get(id) else { continue };
        let write = catalog.registry.owner_syntax(&catalog.sources[&symbol.source_id].language).and_then(|s| s.write);
        for edge in &observed {
            let occurrence = &graph.nodes[&edge.to];
            if write == Some(occurrence.kind.as_str()) {
                out.push(Behavior { name: name.clone(), kind: "state_write".into(), path: edge.evidence.path.clone(), line: edge.evidence.start_line, text: occurrence.label.clone(), detail: String::new() });
            }
        }
    }
    for test in selection.left_out.iter().filter(|e| e.role == "constraint") {
        for reason in test.item.reasons.iter().filter(|r| r.code == "test_caller") {
            out.push(Behavior { name: reason.for_target.clone(), kind: "test".into(), path: test.item.path.clone(), line: reason.lines.first().copied().unwrap_or(test.item.start_line), text: test.item.name.clone(), detail: String::new() });
        }
    }
    out
}

const NOTE_ROLES: [(&str, &str); 6] = [("target", "alvo da mudança"), ("origin", "origem: declara ou escreve o que um alvo lê"), ("cost", "custo: rodou na medição, é chamado por um alvo"),
    ("path", "caminho: chama um alvo"), ("support", "apoio: tipo ou topo de arquivo para compreender os alvos"), ("constraint", "restrição")];

/// What the model reads about the selection, as lines of the evidence's limits: the role of each
/// source, what the graph did not settle, what is written and must be kept. Roles come from fixed
/// rules: they say why a source is there, never that it behaves in some way.
pub fn notes(selection: &Selection) -> Vec<Note> {
    let role = |code: &str| NOTE_ROLES.iter().find(|(c, _)| *c == code).map_or(code.to_owned(), |(_, text)| (*text).to_owned());
    let mut out = vec![Note { id: String::new(), text: "Os papéis abaixo vêm de regras sobre o grafo e a medição: dizem por que cada trecho está aqui, não como ele se comporta.".into() }];
    for entry in &selection.items {
        let calls = entry.calls.map_or(String::new(), |c| format!("; {c} chamadas na medição"));
        out.push(Note { id: entry.item.id.clone(), text: format!("Papel de {} ({}:{}-{}): {}{calls}.", entry.item.name, entry.item.path, entry.item.start_line, entry.item.end_line, role(&entry.role)) });
    }
    for gap in &selection.gaps {
        let place = gap.line.map_or(String::new(), |l| format!(" em {}:{l}", gap.path));
        let text = match gap.code.as_str() {
            "homonym_call" if gap.candidates.is_empty() => format!("Não resolvido: em {}, a chamada `{}`{place} combina com {} declarações.", gap.for_target, gap.text, gap.count.unwrap_or(0)),
            "homonym_call" => format!("Não resolvido: em {}, a chamada `{}`{place} pode ser {}.", gap.for_target, gap.text, gap.candidates.join(" ou ")),
            "homonym_type" => format!("Não resolvido: em {}, o tipo `{}`{place} pode ser {}.", gap.for_target, gap.text, gap.candidates.join(" ou ")),
            "write_not_followed" => format!("Não resolvido: o campo `{}` também é escrito por receptores de tipo desconhecido ({} lugares, como {}).", gap.for_target, gap.count.unwrap_or(0), gap.text),
            "field_without_writer" => format!("Não resolvido: {} lê `{}`, e não foi achado quem o escreve.", gap.for_target, gap.text),
            "origin_left_out" => format!("Fora da evidência por falta de espaço: {}, origem de um valor de {}{place}. Se precisar dele, responda com uma LACUNA.", gap.text, gap.for_target),
            _ => continue,
        };
        out.push(Note { id: String::new(), text });
    }
    for b in selection.behavior.iter().filter(|b| b.kind != "test") {
        let what = match b.kind.as_str() { "loop" => "laço", "operation" => "operação", _ => "escreve" };
        let detail = if b.detail.is_empty() { String::new() } else { format!(" ({})", b.detail) };
        out.push(Note { id: String::new(), text: format!("Escrito em {}, {}:{}: {what}{detail} `{}`.", b.name, b.path, b.line, b.text.split_whitespace().collect::<Vec<_>>().join(" ")) });
    }
    let tests: BTreeSet<&str> = selection.behavior.iter().filter(|b| b.kind == "test").map(|b| b.text.as_str()).collect();
    if !tests.is_empty() {
        out.push(Note { id: String::new(), text: format!("O portão roda os testes que chamam estes trechos, entre eles: {}.", tests.iter().take(5).copied().collect::<Vec<_>>().join(", ")) });
    }
    out
}
