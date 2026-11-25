//! Step 2.4d (DEC-46): "where is it used?" by the inverse of the rule of "what does it call?". Every call
//! of the graph is answered once, by the same function that answers it forward (`v2::answer`), and the
//! answers are read by target. Nothing is recorded: the index is derived from a graph and its edition.
//!
//! Three kinds of use, never added up: `linked` (the rule settles the call on the declaration; that is the
//! rule's answer, not a verification), `possible` (it is one of the bodies that may run, or one of the
//! candidates nothing settles) and `name` (the name is written where nothing is called). Aside, and not
//! uses: calls above the limit of homonyms, calls written against an abstract method, and the names the
//! rule rules out.
use super::{
    v2::{answer, CallSyntax, Edge, Graph, UsedNode},
    Evidence,
};
use crate::{resolve::Resolver, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// What the rule said of one call, towards one declaration.
#[derive(Debug, Clone)]
struct Reached {
    call: usize,
    candidate_edge: Option<usize>,
    /// The `__init__` an instantiation runs is used through the class it builds.
    instance_of: Option<String>,
    relation: &'static str,
}

#[derive(Debug, Clone)]
struct Call {
    /// The edge observing the call.
    observation: usize,
    /// The method that declares the field whose value writes the call (step 2.3b).
    method: Option<String>,
    resolution: &'static str,
    basis: &'static str,
    count: usize,
    type_rule: Option<String>,
}

/// The answers of every call of a graph, by target. Valid for the graph it was built from.
#[derive(Debug, Clone)]
pub struct UseIndex {
    calls: Vec<Call>,
    reached: BTreeMap<String, Vec<Reached>>,
    /// Occurrences that are not calls, by the declaration their name matches: the candidate edges.
    names: BTreeMap<String, Vec<usize>>,
    above_limit: BTreeMap<String, Vec<usize>>,
    abstract_calls: BTreeMap<String, Vec<usize>>,
    ruled_out: BTreeMap<String, Vec<usize>>,
    /// How many declarations an occurrence's written name matches.
    matches: BTreeMap<String, usize>,
    observed: BTreeMap<String, usize>,
    parent: BTreeMap<String, String>,
    declared: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Use {
    /// The symbol that writes it; for a field's value (`via`), the method that declares the field.
    pub from: String,
    pub to: String,
    pub occurrence: String,
    pub occurrence_kind: String,
    pub text: String,
    /// linked, possible or name.
    #[serde(rename = "use")]
    pub kind: &'static str,
    /// How many declarations it may be, as "what does it call?" counts them; for `name`, the written name's.
    pub candidates: usize,
    pub candidate_edge: Option<String>,
    /// As in `/api/calls`; absent for `name`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<&'static str>,
    /// For `possible`: `type` (one of the bodies that may run) or `graph_edge` (a homonym).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basis: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<&'static str>,
    /// `to` is the `__init__` that building this class runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_rule: Option<String>,
    pub evidence: Evidence,
}

/// A written call that is not a use: above the limit, against an abstract method, or ruled out.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Aside {
    pub from: String,
    pub occurrence: String,
    pub occurrence_kind: String,
    pub text: String,
    pub candidates: usize,
    pub evidence: Evidence,
}

/// The direct uses of the seed, by kind; never a total.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Counts {
    pub linked: usize,
    pub possible_type: usize,
    pub possible_homonym: usize,
    pub name: usize,
    pub above_limit: usize,
    pub abstract_calls: usize,
    pub ruled_out: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Uses {
    pub seed: String,
    pub depth: usize,
    pub max_nodes: usize,
    /// True when a node was left out because of `max_nodes`. The seed's own uses are always all in `links`.
    pub truncated: bool,
    pub nodes: Vec<UsedNode>,
    pub links: Vec<Use>,
    pub counts: Counts,
    /// Calls with the seed's name the rule does not settle, above the limit of homonyms: they may use it.
    pub above_limit: Vec<Aside>,
    /// Calls written against the seed, when it is an abstract method: a subclass's body runs.
    pub abstract_calls: Vec<Aside>,
    /// Calls whose name matches the seed and that the rule links elsewhere or rules out.
    pub ruled_out: Vec<Aside>,
}

/// The kinds of declaration a call may name (the extractors' candidate kinds for calls).
const CALLED: [&str; 6] = ["function", "method", "trait_method", "class", "struct", "variant"];

impl UseIndex {
    pub fn build(graph: &Graph, syntax: &CallSyntax) -> Self {
        let resolver = Resolver::new(graph, syntax);
        let mut index = UseIndex {
            calls: vec![],
            reached: BTreeMap::new(),
            names: BTreeMap::new(),
            above_limit: BTreeMap::new(),
            abstract_calls: BTreeMap::new(),
            ruled_out: BTreeMap::new(),
            matches: BTreeMap::new(),
            observed: BTreeMap::new(),
            parent: BTreeMap::new(),
            declared: BTreeMap::new(),
        };
        let mut edge_at: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        for (i, e) in graph.edges.iter().enumerate() {
            match e.kind.as_str() {
                "name_candidate" => {
                    edge_at.insert((&e.from, &e.to), i);
                    *index.matches.entry(e.from.clone()).or_default() += 1;
                }
                "observes" => {
                    index.observed.insert(e.to.clone(), i);
                }
                "contains" => {
                    index.parent.insert(e.to.clone(), e.from.clone());
                }
                "declares" => {
                    index.declared.insert(e.to.clone(), i);
                }
                _ => {}
            }
        }
        let mut named: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
        for (id, n) in &graph.nodes {
            if n.category == "symbol" && CALLED.contains(&n.kind.as_str()) {
                named.entry((&n.language, n.label.rsplit([':', '.']).next().unwrap_or_default())).or_default().push(id);
            }
        }
        let is_call = |occurrence: &str| {
            let n = &graph.nodes[occurrence];
            syntax.languages.get(&n.language).is_some_and(|(kinds, _)| kinds.contains(&n.kind.as_str()))
        };
        // The declarations each call reaches, and the call.
        let mut answered: BTreeMap<&str, (BTreeSet<&str>, usize)> = BTreeMap::new();
        for (i, e) in graph.edges.iter().enumerate().filter(|(_, e)| e.kind == "observes" && is_call(&e.to)) {
            let (observer, occurrence) = (e.from.as_str(), e.to.as_str());
            let said = answer(graph, &resolver, observer, occurrence, None);
            let call = index.calls.len();
            let method = index.parent.get(observer).filter(|m| {
                graph.nodes[observer].kind == "field" && matches!(graph.nodes[m.as_str()].kind.as_str(), "method" | "function")
            });
            let entry = Call {
                observation: i,
                method: method.cloned(),
                resolution: said.resolution,
                basis: said.basis,
                count: said.unresolved.map_or(said.count, |(_, count)| count),
                type_rule: if said.targets.is_empty() { None } else { said.type_rule(&resolver, observer, occurrence) },
            };
            index.calls.push(entry);
            let mut reached: BTreeSet<&str> = BTreeSet::new();
            for &(to, edge) in &said.targets {
                let (relation, runs) = said.relation(graph, &resolver, to);
                let candidate_edge = edge.and_then(|e| edge_at.get(&(e.from.as_str(), e.to.as_str())).copied());
                index.reached.entry(to.to_owned()).or_default().push(Reached { call, candidate_edge, instance_of: None, relation });
                reached.insert(to);
                if let Some(init) = runs {
                    index.reached.entry(init.to_owned()).or_default().push(Reached { call, candidate_edge: None, instance_of: Some(to.to_owned()), relation });
                    reached.insert(init);
                }
            }
            if let Some(method) = said.abstract_method {
                index.abstract_calls.entry(method.to_owned()).or_default().push(call);
                reached.insert(method);
            }
            if let Some(("above_candidate_limit", _)) = said.unresolved {
                if let Some((_, name)) = crate::resolve::call_parts(&graph.nodes[occurrence].label) {
                    for target in named.get(&(graph.nodes[occurrence].language.as_str(), name.as_str())).into_iter().flatten() {
                        if resolver.may_reach(observer, occurrence, target) {
                            index.above_limit.entry((*target).to_owned()).or_default().push(call);
                        }
                    }
                }
            }
            answered.insert(occurrence, (reached, call));
        }
        for (i, e) in graph.edges.iter().enumerate().filter(|(_, e)| e.kind == "name_candidate") {
            match answered.get(e.from.as_str()) {
                Some((reached, call)) => {
                    if !reached.contains(e.to.as_str()) {
                        index.ruled_out.entry(e.to.clone()).or_default().push(*call);
                    }
                }
                None => {
                    if index.observed.contains_key(&e.from) {
                        index.names.entry(e.to.clone()).or_default().push(i);
                    }
                }
            }
        }
        index
    }

    /// Every use of one declaration: linked, the bodies that may run, the homonyms, the names.
    fn uses(&self, graph: &Graph, to: &str) -> Vec<Use> {
        let mut out: Vec<Use> = vec![];
        for r in self.reached.get(to).into_iter().flatten() {
            let call = &self.calls[r.call];
            let observation = &graph.edges[call.observation];
            let edge: Option<&Edge> = r.candidate_edge.map(|i| &graph.edges[i]);
            let written = &graph.nodes[&observation.to];
            out.push(Use {
                from: call.method.clone().unwrap_or_else(|| observation.from.clone()),
                to: to.to_owned(),
                occurrence: observation.to.clone(),
                occurrence_kind: written.kind.clone(),
                text: written.label.clone(),
                kind: if call.resolution == "homonym" { "possible" } else { "linked" },
                candidates: call.count,
                candidate_edge: edge.map(|e| e.id.clone()),
                resolution: Some(call.resolution),
                basis: Some(call.basis),
                relation: Some(r.relation),
                instance_of: r.instance_of.clone(),
                via: call.method.is_some().then(|| observation.from.clone()),
                type_rule: call.type_rule.clone(),
                evidence: edge.map_or(&observation.evidence, |e| &e.evidence).clone(),
            });
        }
        for i in self.names.get(to).into_iter().flatten() {
            let edge = &graph.edges[*i];
            let written = &graph.nodes[&edge.from];
            out.push(Use {
                from: graph.edges[self.observed[&edge.from]].from.clone(),
                to: to.to_owned(),
                occurrence: edge.from.clone(),
                occurrence_kind: written.kind.clone(),
                text: written.label.clone(),
                kind: "name",
                candidates: self.matches[&edge.from],
                candidate_edge: Some(edge.id.clone()),
                resolution: None,
                basis: None,
                relation: None,
                instance_of: None,
                via: None,
                type_rule: None,
                evidence: edge.evidence.clone(),
            });
        }
        out.sort_by_key(rank);
        out
    }

    fn aside(&self, graph: &Graph, calls: Option<&Vec<usize>>) -> Vec<Aside> {
        let one = |i: &usize| {
            let call = &self.calls[*i];
            let observation = &graph.edges[call.observation];
            let written = &graph.nodes[&observation.to];
            Aside {
                from: call.method.clone().unwrap_or_else(|| observation.from.clone()),
                occurrence: observation.to.clone(),
                occurrence_kind: written.kind.clone(),
                text: written.label.clone(),
                candidates: call.count,
                evidence: observation.evidence.clone(),
            }
        };
        calls.into_iter().flatten().map(one).collect()
    }

    /// Who uses `seed`, in layers. Only a linked use is gone up from, and never one in a decorator
    /// (it runs when the `def` does); the others are shown and stop. `max_nodes` cuts the drawing, the
    /// linked uses last; the seed's own uses are all in `links`, drawn or not. A homonym is never drawn for
    /// itself: it is counted and listed (DEC-46: measured on a draw, 0 of 20 and 4 of 20 may run it).
    pub fn used_by(&self, graph: &Graph, seed: &str, depth: usize, max_nodes: usize) -> Result<Uses> {
        if graph.nodes.get(seed).is_none_or(|n| n.category != "symbol") || !(1..=6).contains(&depth) || !(1..=300).contains(&max_nodes) {
            return Err("The seed must be a symbol; depth 1..6, nodes 1..300".into());
        }
        let node = |id: &str, layer: usize| {
            let n = &graph.nodes[id];
            let place = self.declared.get(id).map(|i| &graph.edges[*i].evidence);
            UsedNode {
                id: id.into(),
                label: n.label.clone(),
                kind: n.kind.clone(),
                language: n.language.clone(),
                layer,
                container: self.parent.get(id).map(|p| graph.nodes[p].label.clone()),
                path: place.map_or_else(String::new, |e| e.path.clone()),
                line: place.map_or(0, |e| e.start_line),
            }
        };
        let mut nodes = vec![node(seed, 0)];
        let mut drawn: BTreeSet<String> = BTreeSet::from([seed.to_owned()]);
        let mut walked: BTreeSet<String> = BTreeSet::from([seed.to_owned()]);
        let mut truncated = false;
        let mut frontier = vec![seed.to_owned()];
        let mut found: Vec<(usize, Vec<Use>)> = vec![];
        for layer in 1..=depth {
            let mut next = vec![];
            for used in &frontier {
                let uses = self.uses(graph, used);
                for u in uses.iter().filter(|u| u.kind == "linked") {
                    if !drawn.contains(&u.from) {
                        if nodes.len() >= max_nodes {
                            truncated = true;
                            continue;
                        }
                        drawn.insert(u.from.clone());
                        nodes.push(node(&u.from, layer));
                    }
                    if u.relation != Some("decorates") && walked.insert(u.from.clone()) {
                        next.push(u.from.clone());
                    }
                }
                found.push((layer, uses));
            }
            frontier = next;
        }
        // The rest of the drawing, kind by kind, after every linked use.
        for kind in [1, 3] {
            for (layer, uses) in &found {
                for u in uses.iter().filter(|u| rank(u) == kind) {
                    if drawn.contains(&u.from) {
                        continue;
                    }
                    if nodes.len() >= max_nodes {
                        truncated = true;
                        continue;
                    }
                    drawn.insert(u.from.clone());
                    nodes.push(node(&u.from, *layer));
                }
            }
        }
        let above_limit = self.aside(graph, self.above_limit.get(seed));
        let abstract_calls = self.aside(graph, self.abstract_calls.get(seed));
        let ruled_out = self.aside(graph, self.ruled_out.get(seed));
        let direct = |kind: usize| found[0].1.iter().filter(|u| rank(u) == kind).count();
        let counts = Counts {
            linked: direct(0),
            possible_type: direct(1),
            possible_homonym: direct(2),
            name: direct(3),
            above_limit: above_limit.len(),
            abstract_calls: abstract_calls.len(),
            ruled_out: ruled_out.len(),
        };
        let links = found.into_iter().flat_map(|(_, uses)| uses).filter(|u| u.to == seed || (rank(u) != 2 && drawn.contains(&u.from))).collect();
        Ok(Uses { seed: seed.into(), depth, max_nodes, truncated, nodes, links, counts, above_limit, abstract_calls, ruled_out })
    }
}

/// The order of the kinds: linked, possible by the type, possible homonym, name.
fn rank(u: &Use) -> usize {
    match (u.kind, u.basis) {
        ("linked", _) => 0,
        ("possible", Some("type")) => 1,
        ("possible", _) => 2,
        _ => 3,
    }
}
