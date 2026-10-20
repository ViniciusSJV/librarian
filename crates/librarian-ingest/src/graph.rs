//! Graph derived from verified snapshots. Syntax and name candidates are not type resolution.
use crate::{extractors::Occurrence, hash, Catalog, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub mod v2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub source_id: String,
    pub path: String,
    pub source_sha256: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub excerpt_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,
    /// structural, syntax_only, or name_candidate (never compiler resolution).
    pub resolution: String,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Graph {
    pub version: String,
    pub catalog_sha256: String,
    pub nodes: BTreeMap<String, Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neighborhood {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub depth: usize,
    pub max_nodes: usize,
    pub max_edges: usize,
    pub truncated: bool,
    pub label_bytes_limit: usize,
    pub labels_truncated: usize,
}

impl Graph {
    pub fn build(catalog: &Catalog) -> Self {
        let mut graph = Self {
            version: catalog.registry.graph_version.into(),
            catalog_sha256: hash(
                &serde_json::to_vec(catalog.manifest()).expect("serializable manifest"),
            ),
            nodes: BTreeMap::new(),
            edges: vec![],
        };
        for source in catalog.sources.values() {
            graph.node(source.id.clone(), "source", source.path.clone());
        }
        for symbol in catalog.symbols.values() {
            graph.node(
                symbol.id.clone(),
                &symbol.kind,
                symbol.qualified_name.clone(),
            );
            let chunk = &catalog.chunks[&symbol.chunk_id];
            let evidence = evidence(catalog, &symbol.source_id, chunk.start_byte, chunk.end_byte);
            graph.edges.push(Edge {
                from: symbol.source_id.clone(),
                to: symbol.id.clone(),
                kind: "declares".into(),
                resolution: "structural".into(),
                evidence: evidence.clone(),
            });
            if let Some(parent) = catalog
                .symbols
                .values()
                .filter(|s| s.source_id == symbol.source_id && s.id != symbol.id)
                .filter(|s| {
                    let c = &catalog.chunks[&s.chunk_id];
                    c.start_byte <= chunk.start_byte && c.end_byte >= chunk.end_byte
                })
                .min_by_key(|s| {
                    let c = &catalog.chunks[&s.chunk_id];
                    c.end_byte - c.start_byte
                })
            {
                graph.edges.push(Edge {
                    from: parent.id.clone(),
                    to: symbol.id.clone(),
                    kind: "contains".into(),
                    resolution: "structural".into(),
                    evidence,
                });
            }
        }
        for source in catalog.sources.values() {
            let Some(extractor) = catalog.registry.extractor(&source.language) else {
                continue;
            };
            for found in extractor.observe(&catalog.snapshots[&source.id]) {
                graph.occurrence(catalog, &source.id, found, extractor.candidate_kinds());
            }
        }
        graph.edges.sort_by(|a, b| {
            (&a.from, &a.kind, &a.to, a.evidence.start_byte).cmp(&(
                &b.from,
                &b.kind,
                &b.to,
                b.evidence.start_byte,
            ))
        });
        graph.edges.dedup();
        graph
    }

    fn node(&mut self, id: String, kind: &str, label: String) {
        self.nodes.insert(
            id.clone(),
            Node {
                id,
                kind: kind.into(),
                label,
            },
        );
    }

    /// Bidirectional navigation preserves directed edges. Source nodes are leaves,
    /// preventing file membership from flooding a neighborhood with unrelated symbols.
    pub fn neighborhood(
        &self,
        seed: &str,
        depth: usize,
        max_nodes: usize,
        max_edges: usize,
    ) -> Result<Neighborhood> {
        if !self.nodes.contains_key(seed)
            || depth > 3
            || !(1..=128).contains(&max_nodes)
            || !(1..=256).contains(&max_edges)
        {
            return Err(
                "Unknown seed or limits outside depth 0..3, nodes 1..128, edges 1..256".into(),
            );
        }
        let mut adjacent: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, e) in self.edges.iter().enumerate() {
            adjacent.entry(&e.from).or_default().push(i);
            adjacent.entry(&e.to).or_default().push(i);
        }
        let mut queue = VecDeque::from([(seed.to_owned(), 0)]);
        let mut seen = BTreeSet::from([seed.to_owned()]);
        let mut selected = BTreeSet::new();
        let mut truncated = false;
        while let Some((id, level)) = queue.pop_front() {
            if level >= depth || self.nodes[&id].kind == "source" {
                continue;
            }
            for index in adjacent.get(id.as_str()).into_iter().flatten() {
                let edge = &self.edges[*index];
                let neighbor = if edge.from == id {
                    &edge.to
                } else {
                    &edge.from
                };
                if (!seen.contains(neighbor) && seen.len() >= max_nodes)
                    || (!selected.contains(index) && selected.len() >= max_edges)
                {
                    truncated = true;
                    continue;
                }
                selected.insert(*index);
                if seen.insert(neighbor.clone()) {
                    queue.push_back((neighbor.clone(), level + 1));
                }
            }
        }
        let mut labels_truncated = 0;
        let nodes = seen
            .iter()
            .map(|id| {
                let mut node = self.nodes[id].clone();
                if node.label.len() > 256 {
                    let mut end = 256;
                    while !node.label.is_char_boundary(end) {
                        end -= 1;
                    }
                    node.label.truncate(end);
                    labels_truncated += 1;
                }
                node
            })
            .collect();
        Ok(Neighborhood {
            nodes,
            edges: selected
                .into_iter()
                .map(|i| self.edges[i].clone())
                .collect(),
            depth,
            max_nodes,
            max_edges,
            truncated,
            label_bytes_limit: 256,
            labels_truncated,
        })
    }

    pub fn verify(&self, catalog: &Catalog) -> Result<()> {
        if *self != Self::build(catalog) {
            return Err("Graph does not match verified snapshots and graph version".into());
        }
        Ok(())
    }
}

fn evidence(catalog: &Catalog, source_id: &str, start: usize, end: usize) -> Evidence {
    let s = &catalog.sources[source_id];
    let bytes = &catalog.snapshots[source_id];
    Evidence {
        source_id: source_id.into(),
        path: s.path.clone(),
        source_sha256: s.sha256.clone(),
        start_byte: start,
        end_byte: end,
        start_line: crate::line_at(bytes, start),
        end_line: crate::line_at(bytes, end.saturating_sub(1)),
        excerpt_sha256: hash(&bytes[start..end]),
    }
}

impl Graph {
    /// Attaches an extractor occurrence to its innermost declaration, with byte evidence.
    fn occurrence(
        &mut self,
        catalog: &Catalog,
        source_id: &str,
        found: Occurrence,
        candidate_kinds: &[&str],
    ) {
        let Occurrence {
            kind,
            start,
            end,
            candidate,
        } = found;
        if start >= end {
            return;
        }
        let owner = catalog
            .symbols
            .values()
            .filter(|s| s.source_id == source_id)
            .filter(|s| {
                let c = &catalog.chunks[&s.chunk_id];
                c.start_byte <= start && c.end_byte >= end
            })
            .min_by_key(|s| {
                let c = &catalog.chunks[&s.chunk_id];
                c.end_byte - c.start_byte
            });
        let Some(owner) = owner else { return };
        let id = format!("{source_id}:occurrence:{kind}:{start}:{end}");
        let label =
            String::from_utf8_lossy(&catalog.snapshots[source_id][start..end]).into_owned();
        self.node(id.clone(), kind, label);
        let proof = evidence(catalog, source_id, start, end);
        self.edges.push(Edge {
            from: owner.id.clone(),
            to: id.clone(),
            kind: kind.into(),
            resolution: "syntax_only".into(),
            evidence: proof.clone(),
        });
        // Name matches are navigational candidates, including ambiguities. Never a resolved call/type.
        // Equal names in other languages are not candidates: kinds are only comparable within one.
        let language = &catalog.sources[source_id].language;
        if let Some(name) = candidate {
            for symbol in catalog
                .symbols_named(&name)
                .into_iter()
                .filter(|s| candidate_kinds.contains(&s.kind.as_str()))
                .filter(|s| catalog.sources[&s.source_id].language == *language)
            {
                self.edges.push(Edge {
                    from: id.clone(),
                    to: symbol.id.clone(),
                    kind: "name_candidate".into(),
                    resolution: "name_candidate".into(),
                    evidence: proof.clone(),
                });
            }
        }
    }
}
