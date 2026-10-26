//! Neutral graph contract: a projection of the verified graph, never a source of new relations.
//! Every v2 edge corresponds to exactly one edge of the graph built from the same catalog.
use super::Evidence;
use crate::{hash, Catalog, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const VERSION: &str = "syntax-graph-v2";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    /// source, symbol or occurrence. Chunks stay inspection records, not nodes.
    pub category: String,
    /// Vocabulary of the extractor that observed it; `file` for sources.
    pub kind: String,
    /// Language of the source the node belongs to, as classified by the edition profile.
    pub language: String,
    pub label: String,
    /// Occurrence whose written name matched more declarations than the profile links:
    /// how many. Absent in graphs of profiles before G6 and when candidates were drawn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambiguous_candidates: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    /// Hash of every other field: the same relation always has the same ID.
    pub id: String,
    pub from: String,
    pub to: String,
    /// declares, contains, observes or name_candidate.
    pub kind: String,
    /// structural, syntax_only or name_candidate (never compiler resolution).
    pub resolution: String,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub version: String,
    /// Graph version of the edition profile this projection was derived from.
    pub derived_from: String,
    /// Edition profile whose extractors define the node kinds.
    pub profile: String,
    pub catalog_sha256: String,
    pub nodes: BTreeMap<String, Node>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn build(catalog: &Catalog) -> Self {
        let base = super::Graph::build(catalog);
        let language = |source_id: &str| catalog.sources[source_id].language.clone();
        // Evidence of the edge reaching a node names the source it was observed in.
        let observed_in: BTreeMap<&str, &str> = base
            .edges
            .iter()
            .map(|e| (e.to.as_str(), e.evidence.source_id.as_str()))
            .collect();
        let nodes = base
            .nodes
            .iter()
            .map(|(id, n)| {
                let (category, kind, language) = if let Some(s) = catalog.sources.get(id) {
                    ("source", "file".to_owned(), s.language.clone())
                } else if let Some(s) = catalog.symbols.get(id) {
                    ("symbol", n.kind.clone(), language(&s.source_id))
                } else {
                    // Only occurrences remain; each is attached to its owner by one edge.
                    let source_id = observed_in[id.as_str()];
                    ("occurrence", n.kind.clone(), language(source_id))
                };
                let node = Node {
                    id: id.clone(),
                    category: category.into(),
                    kind,
                    language,
                    label: n.label.clone(),
                    ambiguous_candidates: n.ambiguous_candidates,
                };
                (id.clone(), node)
            })
            .collect::<BTreeMap<_, _>>();
        let edges = base
            .edges
            .into_iter()
            .map(|e| {
                // An edge reaching an occurrence is an observation; its kind stays on the node.
                let kind = if nodes[&e.to].category == "occurrence" {
                    "observes".into()
                } else {
                    e.kind
                };
                edge(e.from, e.to, kind, e.resolution, e.evidence)
            })
            .collect();
        Self {
            version: VERSION.into(),
            derived_from: base.version,
            profile: catalog.manifest().extractor.clone(),
            catalog_sha256: base.catalog_sha256,
            nodes,
            edges,
        }
    }

    pub fn verify(&self, catalog: &Catalog) -> Result<()> {
        if *self != Self::build(catalog) {
            return Err("Graph does not match verified snapshots and graph version".into());
        }
        Ok(())
    }
}

fn edge(from: String, to: String, kind: String, resolution: String, evidence: Evidence) -> Edge {
    let identity = serde_json::to_vec(&(&from, &to, &kind, &resolution, &evidence))
        .expect("serializable edge");
    Edge {
        id: format!("edge:{}", hash(&identity)),
        from,
        to,
        kind,
        resolution,
        evidence,
    }
}

/// G6c: who uses a symbol. From the seed, each step goes back through a written name that
/// matches it (a `name_candidate` edge) to the occurrence and from there to the symbol that
/// wrote it. Every link is a candidate: a homonym is reported, never chosen.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UsedBy {
    pub seed: String,
    pub depth: usize,
    pub max_nodes: usize,
    /// True when a node was left out because of `max_nodes`.
    pub truncated: bool,
    pub nodes: Vec<UsedNode>,
    pub links: Vec<UseLink>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UsedNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub language: String,
    /// Steps from the seed (0 for the seed itself).
    pub layer: usize,
    /// Label of the enclosing declaration (class, impl), from the `contains` edge.
    pub container: Option<String>,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UseLink {
    /// The symbol that wrote the name.
    pub from: String,
    /// The symbol whose name was written.
    pub to: String,
    pub occurrence: String,
    pub occurrence_kind: String,
    /// The written text, e.g. `crud.get_user_by_email(...)`.
    pub text: String,
    /// How many declarations the written name matched; above 1 this link is one of homonyms.
    pub candidates: usize,
    pub candidate_edge: String,
    pub evidence: Evidence,
}

impl Graph {
    pub fn used_by(&self, seed: &str, depth: usize, max_nodes: usize) -> Result<UsedBy> {
        if self.nodes.get(seed).is_none_or(|n| n.category != "symbol") || !(1..=6).contains(&depth) || !(1..=300).contains(&max_nodes) {
            return Err("The seed must be a symbol; depth 1..6, nodes 1..300".into());
        }
        let mut candidates_into: BTreeMap<&str, Vec<&Edge>> = BTreeMap::new();
        let mut candidate_count: BTreeMap<&str, usize> = BTreeMap::new();
        let mut owner: BTreeMap<&str, &str> = BTreeMap::new();
        let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
        let mut declared: BTreeMap<&str, &Evidence> = BTreeMap::new();
        for e in &self.edges {
            match e.kind.as_str() {
                "name_candidate" => {
                    candidates_into.entry(&e.to).or_default().push(e);
                    *candidate_count.entry(&e.from).or_default() += 1;
                }
                "observes" => {
                    owner.insert(&e.to, &e.from);
                }
                "contains" => {
                    parent.insert(&e.to, &e.from);
                }
                "declares" => {
                    declared.insert(&e.to, &e.evidence);
                }
                _ => {}
            }
        }
        let node = |id: &str, layer: usize| {
            let n = &self.nodes[id];
            let place = declared.get(id);
            UsedNode {
                id: id.into(),
                label: n.label.clone(),
                kind: n.kind.clone(),
                language: n.language.clone(),
                layer,
                container: parent.get(id).map(|p| self.nodes[*p].label.clone()),
                path: place.map_or_else(String::new, |e| e.path.clone()),
                line: place.map_or(0, |e| e.start_line),
            }
        };
        let mut nodes = vec![node(seed, 0)];
        let mut layer_of: BTreeMap<String, usize> = BTreeMap::from([(seed.to_owned(), 0)]);
        let mut links = vec![];
        let mut truncated = false;
        let mut frontier = vec![seed.to_owned()];
        for layer in 1..=depth {
            let mut next = vec![];
            for used in &frontier {
                for edge in candidates_into.get(used.as_str()).into_iter().flatten() {
                    let Some(user) = owner.get(edge.from.as_str()) else { continue };
                    if !layer_of.contains_key(*user) {
                        if nodes.len() >= max_nodes {
                            truncated = true;
                            continue;
                        }
                        layer_of.insert((*user).to_owned(), layer);
                        nodes.push(node(user, layer));
                        next.push((*user).to_owned());
                    }
                    let occurrence = &self.nodes[&edge.from];
                    links.push(UseLink {
                        from: (*user).to_owned(),
                        to: used.clone(),
                        occurrence: edge.from.clone(),
                        occurrence_kind: occurrence.kind.clone(),
                        text: occurrence.label.clone(),
                        candidates: candidate_count[edge.from.as_str()],
                        candidate_edge: edge.id.clone(),
                        evidence: edge.evidence.clone(),
                    });
                }
            }
            frontier = next;
        }
        Ok(UsedBy { seed: seed.into(), depth, max_nodes, truncated, nodes, links })
    }
}

/// What "what does it call?" needs from the edition profile, by language: the occurrence kinds
/// that are calls and the receiver of a method's own calls (`<receiver>.f()`), when recorded.
#[derive(Debug, Clone, Default)]
pub struct CallSyntax {
    pub languages: BTreeMap<String, (Vec<&'static str>, Option<&'static str>)>,
}

/// P1b: what a symbol calls, forward, in layers. A call is followed only when its written name
/// points to one declaration: it matches only one (`unique`), it is the receiver's own call and
/// only one homonym shares the caller's container (`receiver`), or the person chose one
/// (`chosen`). Otherwise every candidate is reported (`homonym`) and none is followed.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Calls {
    pub seed: String,
    pub depth: usize,
    pub max_nodes: usize,
    /// True when a node was left out because of `max_nodes`.
    pub truncated: bool,
    pub nodes: Vec<UsedNode>,
    pub links: Vec<CallLink>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CallLink {
    /// The symbol whose body writes the call.
    pub from: String,
    /// A declaration the written name matches.
    pub to: String,
    pub occurrence: String,
    pub occurrence_kind: String,
    /// The written text, e.g. `self.shade_hit(comps, remaining)`.
    pub text: String,
    /// How many declarations the written name matched.
    pub candidates: usize,
    /// unique, receiver, chosen or homonym; homonym links are reported, never followed.
    pub resolution: String,
    pub candidate_edge: String,
    pub evidence: Evidence,
}

impl Graph {
    /// `chosen` maps a call occurrence to the candidate the person picked among its homonyms.
    pub fn calls_from(&self, seed: &str, depth: usize, max_nodes: usize, syntax: &CallSyntax,
        chosen: &BTreeMap<String, String>) -> Result<Calls> {
        if self.nodes.get(seed).is_none_or(|n| n.category != "symbol") || !(1..=6).contains(&depth) || !(1..=300).contains(&max_nodes) {
            return Err("The seed must be a symbol; depth 1..6, nodes 1..300".into());
        }
        let mut candidates_from: BTreeMap<&str, Vec<&Edge>> = BTreeMap::new();
        let mut observed: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
        let mut declared: BTreeMap<&str, &Evidence> = BTreeMap::new();
        for e in &self.edges {
            match e.kind.as_str() {
                "name_candidate" => candidates_from.entry(&e.from).or_default().push(e),
                "observes" => observed.entry(&e.from).or_default().push(&e.to),
                "contains" => {
                    parent.insert(&e.to, &e.from);
                }
                "declares" => {
                    declared.insert(&e.to, &e.evidence);
                }
                _ => {}
            }
        }
        let container = |id: &str| parent.get(id).map(|p| self.nodes[*p].label.as_str());
        let node = |id: &str, layer: usize| {
            let n = &self.nodes[id];
            let place = declared.get(id);
            UsedNode {
                id: id.into(),
                label: n.label.clone(),
                kind: n.kind.clone(),
                language: n.language.clone(),
                layer,
                container: container(id).map(str::to_owned),
                path: place.map_or_else(String::new, |e| e.path.clone()),
                line: place.map_or(0, |e| e.start_line),
            }
        };
        let mut nodes = vec![node(seed, 0)];
        let mut layer_of: BTreeMap<String, usize> = BTreeMap::from([(seed.to_owned(), 0)]);
        let mut followed: std::collections::BTreeSet<String> = std::collections::BTreeSet::from([seed.to_owned()]);
        let mut links = vec![];
        let mut truncated = false;
        let mut frontier = vec![seed.to_owned()];
        for layer in 1..=depth {
            let mut next = vec![];
            for caller in &frontier {
                let Some((kinds, receiver)) = syntax.languages.get(&self.nodes[caller].language) else { continue };
                for occurrence in observed.get(caller.as_str()).into_iter().flatten() {
                    let written = &self.nodes[*occurrence];
                    if !kinds.contains(&written.kind.as_str()) {
                        continue;
                    }
                    let edges = candidates_from.get(occurrence).cloned().unwrap_or_default();
                    let own = receiver.is_some_and(|r| written.label.starts_with(&format!("{r}.")));
                    let same: Vec<&Edge> = edges.iter().copied().filter(|e| container(&e.to) == container(caller)).collect();
                    let pick = chosen.get(*occurrence).and_then(|c| edges.iter().find(|e| &e.to == c));
                    let (reported, resolution): (Vec<&Edge>, &str) = match (pick, edges.len(), own && same.len() == 1) {
                        (Some(e), _, _) => (vec![*e], "chosen"),
                        (None, 1, _) => (edges.clone(), "unique"),
                        (None, _, true) => (same, "receiver"),
                        _ => (edges.clone(), "homonym"),
                    };
                    for edge in reported {
                        if !layer_of.contains_key(&edge.to) {
                            if nodes.len() >= max_nodes {
                                truncated = true;
                                continue;
                            }
                            layer_of.insert(edge.to.clone(), layer);
                            nodes.push(node(&edge.to, layer));
                        }
                        if resolution != "homonym" && followed.insert(edge.to.clone()) {
                            next.push(edge.to.clone());
                        }
                        links.push(CallLink {
                            from: caller.clone(),
                            to: edge.to.clone(),
                            occurrence: (*occurrence).to_owned(),
                            occurrence_kind: written.kind.clone(),
                            text: written.label.clone(),
                            candidates: edges.len(),
                            resolution: resolution.into(),
                            candidate_edge: edge.id.clone(),
                            evidence: edge.evidence.clone(),
                        });
                    }
                }
            }
            frontier = next;
        }
        Ok(Calls { seed: seed.into(), depth, max_nodes, truncated, nodes, links })
    }
}
