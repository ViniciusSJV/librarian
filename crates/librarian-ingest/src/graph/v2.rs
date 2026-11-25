//! Neutral graph contract: a projection of the verified graph, never a source of new relations.
//! Every v2 edge corresponds to exactly one edge of the graph built from the same catalog.
use crate::resolve::{Resolution, Resolver};
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
    /// Step 2.3a: on a receiver_type occurrence, the rule that gave the written type (`parametro`,
    /// `anotacao`, `self`, `construtor`, `caminho`, `retorno`, `campo`, `item`, `variante`).
    /// Absent in graphs of profiles before v9 and on every other node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_rule: Option<String>,
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
                    type_rule: n.type_rule.clone(),
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
    /// By the written name alone. "Where is it used?" reads the rule instead (`uses`, step 2.4d); the
    /// envelope and "Medir tudo" still read this one.
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
/// `facts` reads what the rule needs from one file's preserved bytes (step 2.2); without it, the
/// rule uses the graph alone.
#[derive(Debug, Clone, Default)]
pub struct CallSyntax {
    pub languages: BTreeMap<String, (Vec<&'static str>, Option<&'static str>)>,
    pub facts: Option<crate::resolve::FileFacts>,
}

/// P1b: what a symbol calls, forward, in layers, by the one rule of `resolve.rs`. A call is
/// followed only when the rule settles it on one declaration: the person chose it (`chosen`), the
/// written name matches only it among the candidates the call's form reaches (`unique`), the
/// receiver is a module of the project and it is that module's only one (`import`), it is
/// `self.m()` and the only homonym in the
/// caller's container (`receiver`), it is a method of the receiver's written type (`type`) or
/// the trait default that type gets (`trait_default`). Several targets (`trait`, a type that
/// names several impls) are reported as today: every candidate as `homonym`, none followed.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Calls {
    pub seed: String,
    pub depth: usize,
    pub max_nodes: usize,
    /// True when a node was left out because of `max_nodes`.
    pub truncated: bool,
    pub nodes: Vec<UsedNode>,
    pub links: Vec<CallLink>,
    /// G3: the call occurrences of the walked symbols that are not in `links`, each with why.
    /// Absence from `links` is not absence of a call: these are calls the graph observed.
    pub unresolved: Vec<UnresolvedCall>,
}

/// A call occurrence the walk knows and does not present as a link.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UnresolvedCall {
    /// The symbol whose body writes the call.
    pub from: String,
    pub occurrence: String,
    pub occurrence_kind: String,
    /// The written text, e.g. `ch.notify(flip)` or `handlers[0]`.
    pub text: String,
    /// `above_candidate_limit`: more same-named declarations than the profile links; the graph
    /// keeps only their count, and none is chosen. `no_candidate_in_project`: no declaration of the
    /// project matches the occurrence (its name, when one is written); this does not say the call is
    /// external. `node_limit`: a link the graph has, left out by `max_nodes`; not an ambiguity.
    pub reason: String,
    /// How many declarations the written name matched: the recorded count above the limit, 0 with
    /// no candidate, the link's own count at the node limit.
    pub candidates: usize,
    /// `node_limit` only: the link left out, as it would have been (`resolution`, `to`, `candidate_edge`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_edge: Option<String>,
    /// Step 2.3b: the field whose value writes the call, when `from` is the method that contains
    /// that field (`self.x = T()` in `__init__`). Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    /// Where the call is written: the edge observing it, or the candidate edge left out.
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CallLink {
    /// The symbol whose body writes the call; since step 2.3b, also the method that contains the
    /// field whose value writes it (then `via` names the field).
    pub from: String,
    /// A declaration the written name matches.
    pub to: String,
    pub occurrence: String,
    pub occurrence_kind: String,
    /// The written text, e.g. `self.shade_hit(comps, remaining)`.
    pub text: String,
    /// How many declarations the written name matched.
    pub candidates: usize,
    /// unique, receiver, type, trait_default, chosen or homonym; homonym links are reported,
    /// never followed.
    pub resolution: String,
    /// The name_candidate edge to `to`. Null only for a `type`, `trait_default` or `receiver` link, or an
    /// abstract method's override (step 2.4b), of a call above the candidate limit: the graph kept only
    /// the count of its candidates. And for an inherited method the written name does not match.
    pub candidate_edge: Option<String>,
    /// Where the resolution comes from: `graph_edge` (the candidate edges alone: unique,
    /// receiver, chosen, homonym) or `type` (the receiver's written type, a layer over the graph).
    pub basis: String,
    /// Step 2.3a (DEC-39): `instantiates` when the one target settled by the rule is a kind of
    /// declaration that the call builds (`resolve::INSTANTIATED`); `calls` otherwise, homonyms included.
    /// Step 2.3b (DEC-40): `decorates` when the call is written inside a decorator, whatever its
    /// target: it runs when the `def` runs; never followed.
    pub relation: String,
    /// Step 2.3a: for an instantiation, the declaration it runs (`Resolver::instance_entry`), which
    /// the walk follows instead of the target. Absent otherwise; an instantiation without it is not followed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runs: Option<String>,
    /// Step 2.3b: as in `UnresolvedCall`, the field whose value writes the call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    /// Step 2.3c (DEC-41): for a link settled by the receiver's written type (`basis: type`), the
    /// rule that gave the type: the one its receiver_type recorded (`parametro`, `self`, ...),
    /// `caminho` for `T::f(..)`, or `retorno` and `campo` when the 2.3c decided. Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_rule: Option<String>,
    /// Step 2.4c (DEC-44): for the bodies of a dispatched call, the static type's method, when it is
    /// abstract: it does not run, and for that it is not among them. Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_method: Option<String>,
    pub evidence: Evidence,
}

/// What the rule says of one call, for both directions: `calls_from` reads it forward and
/// `uses::UseIndex` (step 2.4d) inverts it, so the two cannot differ.
pub(crate) struct Answer<'g> {
    /// The one target settled; or every candidate, when `resolution` is `homonym`. With the candidate edge.
    pub targets: Vec<(&'g str, Option<&'g Edge>)>,
    pub resolution: &'static str,
    pub basis: &'static str,
    /// How many declarations the call counts as its candidates.
    pub count: usize,
    pub decorates: bool,
    pub abstract_method: Option<&'g str>,
    /// Without targets: the reason and the candidates it counts.
    pub unresolved: Option<(&'static str, usize)>,
}

impl<'g> Answer<'g> {
    /// The relation to a target and, for an instantiation, the declaration it runs.
    pub fn relation(&self, graph: &Graph, resolver: &Resolver<'g>, to: &str) -> (&'static str, Option<&'g str>) {
        if self.decorates {
            return ("decorates", None);
        }
        if self.resolution != "homonym" && crate::resolve::INSTANTIATED.contains(&graph.nodes[to].kind.as_str()) {
            return ("instantiates", resolver.instance_entry(to));
        }
        ("calls", None)
    }

    pub fn type_rule(&self, resolver: &Resolver<'g>, observer: &str, occurrence: &str) -> Option<String> {
        (self.basis == "type").then(|| resolver.type_rule(observer, occurrence)).flatten()
    }
}

/// The answer to a call, for the symbol that observes it.
pub(crate) fn answer<'g>(graph: &'g Graph, resolver: &Resolver<'g>, observer: &str, occurrence: &str, chosen: Option<&str>) -> Answer<'g> {
    let written = &graph.nodes[occurrence];
    // The candidates the call's form reaches (step 2.2).
    let edges = resolver.admitted(observer, occurrence);
    let resolved = resolver.resolve(observer, occurrence, chosen);
    let decorates = resolver.decorates(occurrence);
    // One target settled by the rule; or every candidate edge, as homonyms.
    let out_of_chain = matches!(resolved, Resolution::OutOfChain);
    let overrides = matches!(resolved, Resolution::Overrides(_));
    let (targets, resolution, basis): (Vec<(&'g str, Option<&'g Edge>)>, &'static str, &'static str) = match resolved {
        Resolution::Linked(how, targets) if targets.len() == 1 => {
            (vec![(targets[0], edges.iter().copied().find(|e| e.to == targets[0]))], how.as_str(), how.basis())
        }
        // Ruled out by the receiver's written type (step 2.2): no link.
        Resolution::NoCandidate | Resolution::OutOfChain => (vec![], "homonym", "graph_edge"),
        // Steps 2.4b and 2.4c (DEC-43, DEC-44): the bodies that may run, as homonyms, by the type.
        Resolution::Overrides(targets) => {
            (targets.into_iter().map(|t| (t, resolver.candidates(occurrence).iter().copied().find(|e| e.to == t))).collect(), "homonym", "type")
        }
        _ => (edges.iter().map(|e| (e.to.as_str(), Some(*e))).collect(), "homonym", "graph_edge"),
    };
    // No candidate edge and nothing settled: what the graph recorded says why.
    let unresolved = targets.is_empty().then_some(match written.ambiguous_candidates {
        Some(count) if !out_of_chain => ("above_candidate_limit", count),
        _ => ("no_candidate_in_project", 0),
    });
    // Above the limit, a type link counts the declarations the graph recorded.
    // A dispatched call counts its bodies (steps 2.4b and 2.4c).
    let count = if overrides { targets.len() } else if edges.is_empty() { written.ambiguous_candidates.unwrap_or(0) } else { edges.len() };
    let abstract_method = if overrides {
        let bodies: Vec<&str> = targets.iter().map(|(t, _)| *t).collect();
        resolver.abstract_left_out(observer, occurrence, &bodies)
    } else {
        None
    };
    Answer { targets, resolution, basis, count, decorates, abstract_method, unresolved }
}

impl Graph {
    /// `chosen` maps a call occurrence to the candidate the person picked among its homonyms.
    pub fn calls_from(&self, seed: &str, depth: usize, max_nodes: usize, syntax: &CallSyntax,
        chosen: &BTreeMap<String, String>) -> Result<Calls> {
        if self.nodes.get(seed).is_none_or(|n| n.category != "symbol") || !(1..=6).contains(&depth) || !(1..=300).contains(&max_nodes) {
            return Err("The seed must be a symbol; depth 1..6, nodes 1..300".into());
        }
        let resolver = Resolver::new(self, syntax);
        let mut observed: BTreeMap<&str, Vec<&Edge>> = BTreeMap::new();
        let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
        let mut declared: BTreeMap<&str, &Evidence> = BTreeMap::new();
        for e in &self.edges {
            match e.kind.as_str() {
                "observes" => observed.entry(&e.from).or_default().push(e),
                "contains" => {
                    parent.insert(&e.to, &e.from);
                }
                "declares" => {
                    declared.insert(&e.to, &e.evidence);
                }
                _ => {}
            }
        }
        // Step 2.3b (DEC-40): the fields a function or method declares in its body (`self.x = ..`
        // in `__init__`). Their values run when the method runs, so it answers for them too.
        let mut fields_in: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (field, method) in &parent {
            if self.nodes[*field].kind == "field" && matches!(self.nodes[*method].kind.as_str(), "method" | "function") {
                fields_in.entry(method).or_default().push(field);
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
        let mut unresolved = vec![];
        let mut truncated = false;
        let mut frontier = vec![seed.to_owned()];
        // Each call occurrence is reported once in an answer, whichever way it is reached.
        let mut reported_once: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for layer in 1..=depth {
            let mut next = vec![];
            for caller in &frontier {
                let Some((kinds, _)) = syntax.languages.get(&self.nodes[caller].language) else { continue };
                // The caller's own calls, then those of the fields it declares (step 2.3b), each
                // with the symbol the graph says observes it.
                let own = observed.get(caller.as_str()).into_iter().flatten().map(|o| (caller.as_str(), o));
                let of_fields = fields_in.get(caller.as_str()).into_iter().flatten()
                    .flat_map(|f| observed.get(f).into_iter().flatten().map(move |o| (*f, o)));
                for (observer, observation) in own.chain(of_fields) {
                    let occurrence = &observation.to.as_str();
                    let written = &self.nodes[*occurrence];
                    if !kinds.contains(&written.kind.as_str()) || !reported_once.insert(occurrence) {
                        continue;
                    }
                    let via = (observer != caller.as_str()).then(|| observer.to_owned());
                    // One answer per call, decided for the symbol that observes it: a field's call gets
                    // the same answer in its method. "Where is it used?" inverts this same answer (2.4d).
                    let answer = answer(self, &resolver, observer, occurrence, chosen.get(*occurrence).map(String::as_str));
                    if let Some((reason, candidates)) = answer.unresolved {
                        unresolved.push(UnresolvedCall {
                            from: caller.clone(),
                            occurrence: (*occurrence).to_owned(),
                            occurrence_kind: written.kind.clone(),
                            text: written.label.clone(),
                            reason: reason.into(),
                            candidates,
                            resolution: None,
                            to: None,
                            candidate_edge: None,
                            via: via.clone(),
                            evidence: observation.evidence.clone(),
                        });
                        continue;
                    }
                    let Answer { resolution, basis, count, abstract_method, .. } = answer;
                    for &(to, edge) in &answer.targets {
                        let evidence = edge.map_or(&observation.evidence, |e| &e.evidence);
                        if !layer_of.contains_key(to) {
                            if nodes.len() >= max_nodes {
                                truncated = true;
                                // The link exists; only its presentation is left out.
                                unresolved.push(UnresolvedCall {
                                    from: caller.clone(),
                                    occurrence: (*occurrence).to_owned(),
                                    occurrence_kind: written.kind.clone(),
                                    text: written.label.clone(),
                                    reason: "node_limit".into(),
                                    candidates: count,
                                    resolution: Some(resolution.into()),
                                    to: Some(to.to_owned()),
                                    candidate_edge: edge.map(|e| e.id.clone()),
                                    via: via.clone(),
                                    evidence: evidence.clone(),
                                });
                                continue;
                            }
                            layer_of.insert(to.to_owned(), layer);
                            nodes.push(node(to, layer));
                        }
                        let (relation, runs) = answer.relation(self, &resolver, to);
                        let (decorates, instantiates) = (relation == "decorates", relation == "instantiates");
                        // An instantiation is followed into what it runs, never into the body of the
                        // declaration it builds (DEC-39). A decorator's call runs when the `def`
                        // runs, not when the decorated declaration does: never followed (DEC-40).
                        let walk = if decorates { None } else if instantiates { runs } else { (resolution != "homonym").then_some(to) };
                        if let Some(walk) = walk {
                            if runs.is_some() && !layer_of.contains_key(walk) {
                                if nodes.len() < max_nodes {
                                    layer_of.insert(walk.to_owned(), layer);
                                    nodes.push(node(walk, layer));
                                } else {
                                    truncated = true;
                                }
                            }
                            if layer_of.contains_key(walk) && followed.insert(walk.to_owned()) {
                                next.push(walk.to_owned());
                            }
                        }
                        links.push(CallLink {
                            from: caller.clone(),
                            to: to.to_owned(),
                            occurrence: (*occurrence).to_owned(),
                            occurrence_kind: written.kind.clone(),
                            text: written.label.clone(),
                            candidates: count,
                            resolution: resolution.into(),
                            candidate_edge: edge.map(|e| e.id.clone()),
                            basis: basis.into(),
                            relation: relation.into(),
                            runs: runs.map(str::to_owned),
                            via: via.clone(),
                            type_rule: answer.type_rule(&resolver, observer, occurrence),
                            abstract_method: abstract_method.map(str::to_owned),
                            evidence: evidence.clone(),
                        });
                    }
                }
            }
            frontier = next;
        }
        Ok(Calls { seed: seed.into(), depth, max_nodes, truncated, nodes, links, unresolved })
    }
}
