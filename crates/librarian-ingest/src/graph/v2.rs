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
