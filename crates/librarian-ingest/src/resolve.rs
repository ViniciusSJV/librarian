//! The one rule that settles a call: which declaration a written call reaches, and how. Used by
//! "what does it call?" (`Graph::calls_from`), by the operation inventory, and, through the
//! links of `calls_from`, by the envelope and the owner context.
//!
//! A layer over the v2 graph: it reads nodes and edges and adds none. The graph's edges stay
//! name candidates, never compiler resolution; `type` and `trait_default` use the receiver's
//! written type (a `receiver_type` occurrence, or a `T::f` path), which the Rust extractor
//! records and the Python one does not.
//!
//! Precedence: chosen > unique > receiver > type > trait_default > trait > homonym. `unique`
//! and `receiver` come before `type` because here the type never contradicts them (a unique
//! name can only be confirmed; `self.m()` is the impl's own type), and their labels stay.
use crate::graph::v2::{CallSyntax, Edge, Graph};
use crate::graph::Evidence;
use std::collections::{BTreeMap, BTreeSet};

/// Resolutions a consumer may take as the one declaration a call reaches. `calls_from` reports
/// them only with a single target.
pub const SETTLED: [&str; 4] = ["unique", "receiver", "type", "trait_default"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// The person picked it among the candidates.
    Chosen,
    /// The written name matches only this declaration.
    Unique,
    /// `self.m()` (the language's receiver, nothing else before the name) and only this
    /// candidate shares the caller's container.
    Receiver,
    /// A method of the receiver's written type.
    Type,
    /// The receiver's type has no such method; the default of a trait it implements.
    TraitDefault,
    /// No written type, and every candidate belongs to one trait: all its implementations.
    Trait,
}

impl How {
    pub fn as_str(self) -> &'static str {
        match self {
            How::Chosen => "chosen",
            How::Unique => "unique",
            How::Receiver => "receiver",
            How::Type => "type",
            How::TraitDefault => "trait_default",
            How::Trait => "trait",
        }
    }
    /// Where the resolution comes from: the graph's candidate edges alone (`graph_edge`), or
    /// the receiver's written type (`type`).
    pub fn basis(self) -> &'static str {
        match self {
            How::Type | How::TraitDefault => "type",
            _ => "graph_edge",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution<'g> {
    /// Followed: one target, or several (`trait`, or a type that names several impls).
    Linked(How, Vec<&'g str>),
    /// Several candidate edges and nothing settles them.
    Homonym(Vec<&'g str>),
    /// More same-named declarations than the profile links, and no written type settles it.
    AboveLimit(usize),
    NoCandidate,
}

pub struct Resolver<'g> {
    graph: &'g Graph,
    receivers: BTreeMap<String, Option<&'static str>>,
    candidates: BTreeMap<&'g str, Vec<&'g Edge>>,
    parent: BTreeMap<&'g str, &'g str>,
    methods_by_name: BTreeMap<&'g str, Vec<&'g str>>,
    /// receiver_type occurrences by where they start: (end, occurrence).
    receiver_types: BTreeMap<(&'g str, usize), Vec<(usize, &'g str)>>,
    call_evidence: BTreeMap<&'g str, &'g Evidence>,
    /// (type, trait) of every `impl Trait for Type`.
    impl_traits: Vec<(String, String)>,
}

impl<'g> Resolver<'g> {
    pub fn new(graph: &'g Graph, syntax: &CallSyntax) -> Self {
        let mut r = Resolver {
            graph,
            receivers: syntax.languages.iter().map(|(l, (_, receiver))| (l.clone(), *receiver)).collect(),
            candidates: BTreeMap::new(),
            parent: BTreeMap::new(),
            methods_by_name: BTreeMap::new(),
            receiver_types: BTreeMap::new(),
            call_evidence: BTreeMap::new(),
            impl_traits: vec![],
        };
        for e in &graph.edges {
            match e.kind.as_str() {
                "name_candidate" => r.candidates.entry(&e.from).or_default().push(e),
                "contains" => {
                    r.parent.insert(&e.to, &e.from);
                }
                "observes" => {
                    r.call_evidence.insert(&e.to, &e.evidence);
                    if graph.nodes.get(&e.to).is_some_and(|n| n.kind == "receiver_type") {
                        r.receiver_types.entry((e.evidence.path.as_str(), e.evidence.start_byte)).or_default().push((e.evidence.end_byte, &e.to));
                    }
                }
                _ => {}
            }
        }
        for (id, node) in &graph.nodes {
            if node.category != "symbol" {
                continue;
            }
            if matches!(node.kind.as_str(), "method" | "trait_method") {
                r.methods_by_name.entry(node.label.rsplit("::").next().unwrap_or_default()).or_default().push(id);
            }
            if node.kind == "impl" {
                if let (Some(ty), Some(tr)) = impl_parts(&node.label) {
                    r.impl_traits.push((ty, tr));
                }
            }
        }
        r
    }

    /// The candidate edges the graph drew for a call occurrence.
    pub fn candidates(&self, occurrence: &str) -> &[&'g Edge] {
        self.candidates.get(occurrence).map_or(&[], Vec::as_slice)
    }

    fn label(&self, id: &str) -> &'g str {
        &self.graph.nodes[id].label
    }

    fn kind(&self, id: &str) -> &'g str {
        &self.graph.nodes[id].kind
    }

    fn container(&self, id: &str) -> Option<&'g str> {
        self.parent.get(id).copied()
    }

    /// The type and the trait an impl or a trait names, for a method in it.
    fn owner_of(&self, method: &str) -> Option<(Option<String>, Option<String>)> {
        let owner = self.container(method)?;
        match self.kind(owner) {
            "impl" => Some(impl_parts(self.label(owner))),
            "trait" => Some((None, Some(last_segment(self.label(owner))))),
            _ => None,
        }
    }

    /// The written type of a call's receiver: `T::f` and `Self::f` by their path, `x.f()` by the
    /// receiver_type the extractor recorded for the receiver that ends where the call's written
    /// receiver ends (`self` and `self.transform` both start where `self.transform.inverse()` does).
    pub fn receiver_types(&self, caller: &str, occurrence: &str) -> Vec<String> {
        let text = self.label(occurrence);
        if self.kind(occurrence) == "call_observed" {
            let Some((receiver, _)) = call_parts(text) else { return vec![] };
            if receiver == "Self" {
                return self.container(caller).filter(|c| self.kind(c) == "impl").and_then(|c| impl_parts(self.label(c)).0).into_iter().collect();
            }
            return if receiver.starts_with(|c: char| c.is_uppercase()) { vec![receiver] } else { vec![] };
        }
        let Some(evidence) = self.call_evidence.get(occurrence) else { return vec![] };
        let Some(end) = written_receiver(text).map(|r| evidence.start_byte + r.len()) else { return vec![] };
        let Some(written) = self
            .receiver_types
            .get(&(evidence.path.as_str(), evidence.start_byte))
            .and_then(|found| found.iter().find(|(e, _)| *e == end).map(|(_, id)| *id))
        else {
            return vec![];
        };
        let names: BTreeSet<String> = self.candidates(written).iter().map(|t| last_segment(self.label(&t.to))).collect();
        names.into_iter().collect()
    }

    /// The rule. `caller` is the symbol whose body writes the call; `chosen`, the candidate the
    /// person picked for this occurrence, if any.
    pub fn resolve(&self, caller: &str, occurrence: &str, chosen: Option<&str>) -> Resolution<'g> {
        let edges = self.candidates(occurrence);
        if let Some(e) = chosen.and_then(|c| edges.iter().find(|e| e.to == c)) {
            return Resolution::Linked(How::Chosen, vec![&e.to]);
        }
        if edges.len() == 1 {
            return Resolution::Linked(How::Unique, vec![&edges[0].to]);
        }
        let text = self.label(occurrence);
        let receiver = self.receivers.get(&self.graph.nodes[caller].language).copied().flatten();
        let own = receiver.is_some_and(|r| written_receiver(text) == Some(r));
        let same: Vec<&'g str> = edges.iter().map(|e| e.to.as_str()).filter(|t| self.container(t) == self.container(caller)).collect();
        if own && same.len() == 1 && self.kind(same[0]) != "trait_method" {
            return Resolution::Linked(How::Receiver, same);
        }
        let name = call_parts(text).map(|(_, n)| n).unwrap_or_default();
        let methods = self.methods_by_name.get(name.as_str()).map_or(&[][..], Vec::as_slice);
        let types = self.receiver_types(caller, occurrence);
        if !types.is_empty() {
            let own: Vec<&'g str> =
                methods.iter().copied().filter(|m| self.owner_of(m).is_some_and(|(t, _)| t.is_some_and(|t| types.contains(&t)))).collect();
            if !own.is_empty() {
                return Resolution::Linked(How::Type, own);
            }
            let traits: BTreeSet<&str> = self.impl_traits.iter().filter(|(t, _)| types.contains(t)).map(|(_, tr)| tr.as_str()).collect();
            let defaults: Vec<&'g str> = methods
                .iter()
                .copied()
                .filter(|m| self.kind(m) == "trait_method" && self.owner_of(m).is_some_and(|(_, tr)| tr.is_some_and(|tr| traits.contains(tr.as_str()))))
                .collect();
            if !defaults.is_empty() {
                return Resolution::Linked(How::TraitDefault, defaults);
            }
        }
        let traits: BTreeSet<Option<String>> = edges.iter().map(|e| self.owner_of(&e.to).and_then(|(_, tr)| tr)).collect();
        if edges.len() > 1 && traits.len() == 1 && traits.iter().all(Option::is_some) {
            return Resolution::Linked(How::Trait, edges.iter().map(|e| e.to.as_str()).collect());
        }
        if edges.is_empty() {
            return match self.graph.nodes[occurrence].ambiguous_candidates {
                Some(count) => Resolution::AboveLimit(count),
                None => Resolution::NoCandidate,
            };
        }
        Resolution::Homonym(edges.iter().map(|e| e.to.as_str()).collect())
    }
}

/// A written path's last segment without generic arguments or references: `ops::Mul` ->
/// `Mul`, `Matrix<4>` -> `Matrix`, `&'a Sphere` -> `Sphere`.
pub(crate) fn last_segment(text: &str) -> String {
    let text = text.split('<').next().unwrap_or_default();
    let last = text.rsplit("::").next().unwrap_or_default();
    last.rsplit(|c: char| c.is_whitespace() || c == '&').next().unwrap_or_default().to_owned()
}

/// The type and the trait of a Rust impl's name: `impl Intersectable for Sphere` ->
/// (`Sphere`, `Intersectable`), `impl Matrix<4>` -> (`Matrix`, none).
pub(crate) fn impl_parts(label: &str) -> (Option<String>, Option<String>) {
    let Some(rest) = label.strip_prefix("impl ") else { return (None, None) };
    match rest.split_once(" for ") {
        Some((tr, ty)) => (Some(last_segment(ty)), Some(last_segment(tr))),
        None => (Some(last_segment(rest)), None),
    }
}

/// A call's text without its last arguments and turbofish: `a.b::<T>(x)` -> `a.b`.
fn call_head(text: &str) -> Option<&str> {
    let mut head = text.trim_end();
    // A method call's text has its arguments: drop the last balanced parentheses.
    if head.ends_with(')') {
        head = &head[..matching_open(head, '(', ')')?];
    }
    // Turbofish: `collect::<Vec<_>>`.
    if head.ends_with('>') {
        head = head[..matching_open(head, '<', '>')?].trim_end_matches("::");
    }
    Some(head)
}

/// The written name of a call and the name written just before it, from the call's text:
/// `self.session.exec(s)` -> (`session`, `exec`), `File::open` -> (`File`, `open`),
/// `open` -> (``, `open`). None when the text ends in no name.
pub fn call_parts(text: &str) -> Option<(String, String)> {
    let head = call_head(text)?;
    let name = trailing_ident(head)?;
    let before = head[..head.len() - name.len()].trim_end();
    // A receiver that is itself a call's result or an index (`a.b(x).c()`) has no written
    // name: "()", never the "" of a call with no receiver.
    let receiver = match before.strip_suffix('.').or_else(|| before.strip_suffix("::")) {
        Some(rest) => trailing_ident(rest.trim_end()).unwrap_or("()"),
        None => "",
    };
    Some((receiver.to_owned(), name.to_owned()))
}

/// A method call's whole written receiver: `self.transform.inverse()` -> `self.transform`,
/// `self.f(x).g()` -> `self.f(x)`. None without a receiver.
pub fn written_receiver(text: &str) -> Option<&str> {
    let head = call_head(text)?;
    let name = trailing_ident(head)?;
    let receiver = head[..head.len() - name.len()].trim_end().strip_suffix('.')?.trim_end();
    (!receiver.is_empty()).then_some(receiver)
}

pub(crate) fn matching_open(text: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in text.char_indices().rev() {
        if c == close {
            depth += 1;
        } else if c == open {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

pub(crate) fn trailing_ident(text: &str) -> Option<&str> {
    let start = text.char_indices().rev().take_while(|(_, c)| c.is_alphanumeric() || *c == '_').last().map(|(i, _)| i)?;
    Some(&text[start..])
}
