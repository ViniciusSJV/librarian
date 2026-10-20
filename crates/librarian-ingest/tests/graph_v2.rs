//! Neutral graph contract over the frozen v1 edition. graph-v2.json was generated once in G2;
//! a mismatch means the projection changed, not that the golden should be rewritten.
use librarian_ingest::{
    graph::{v2, Graph},
    hash, Catalog,
};
use std::{collections::BTreeSet, fs, path::Path};

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1")
}

fn load() -> Catalog {
    Catalog::load(&fixture().join("edition")).unwrap()
}

#[test]
fn v2_matches_its_golden_and_verifies() {
    let catalog = load();
    let graph = v2::Graph::build(&catalog);
    let golden = fs::read(fixture().join("graph-v2.json")).unwrap();
    assert_eq!(serde_json::to_vec_pretty(&graph).unwrap(), golden);
    let parsed: v2::Graph = serde_json::from_slice(&golden).unwrap();
    parsed.verify(&catalog).unwrap();
    assert_eq!(parsed.version, v2::VERSION);
    assert_eq!(parsed.derived_from, "rust-syntax-graph-v1");
    assert_eq!(parsed.profile, catalog.manifest().extractor);
}

#[test]
fn v2_projects_v1_without_creating_relations() {
    let catalog = load();
    let base = Graph::build(&catalog);
    let graph = v2::Graph::build(&catalog);
    assert_eq!(
        graph.nodes.keys().collect::<Vec<_>>(),
        base.nodes.keys().collect::<Vec<_>>()
    );
    assert_eq!(graph.edges.len(), base.edges.len());
    assert_eq!(graph.catalog_sha256, base.catalog_sha256);
    for (new, old) in graph.edges.iter().zip(&base.edges) {
        assert_eq!(
            (&new.from, &new.to, &new.resolution, &new.evidence),
            (&old.from, &old.to, &old.resolution, &old.evidence)
        );
        let target = &graph.nodes[&new.to];
        if target.category == "occurrence" {
            assert_eq!(new.kind, "observes");
            // The occurrence kind moved to the node; it is not lost.
            assert_eq!(target.kind, old.kind);
        } else {
            assert_eq!(new.kind, old.kind);
        }
    }
}

#[test]
fn v2_categories_and_languages_come_from_the_catalog() {
    let catalog = load();
    let graph = v2::Graph::build(&catalog);
    for node in graph.nodes.values() {
        let expected = if catalog.sources().contains_key(&node.id) {
            "source"
        } else if catalog.symbols().contains_key(&node.id) {
            "symbol"
        } else {
            "occurrence"
        };
        assert_eq!(node.category, expected, "{}", node.id);
    }
    let helper = catalog.source_by_path("scripts/helper.py").unwrap();
    assert_eq!(graph.nodes[&helper.id].language, "auxiliary");
    assert_eq!(graph.nodes[&helper.id].kind, "file");
    let kinds: BTreeSet<_> = graph.edges.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(
        kinds,
        BTreeSet::from(["contains", "declares", "name_candidate", "observes"])
    );
}

#[test]
fn v2_edge_ids_are_unique_and_recomputable() {
    let graph = v2::Graph::build(&load());
    let mut seen = BTreeSet::new();
    for e in &graph.edges {
        let identity =
            serde_json::to_vec(&(&e.from, &e.to, &e.kind, &e.resolution, &e.evidence)).unwrap();
        assert_eq!(e.id, format!("edge:{}", hash(&identity)));
        assert!(seen.insert(&e.id), "duplicate edge id {}", e.id);
    }
}

#[test]
fn v2_rejects_forged_relations_and_unknown_fields() {
    let catalog = load();
    let mut forged = v2::Graph::build(&catalog);
    let candidate = forged
        .edges
        .iter_mut()
        .find(|e| e.kind == "name_candidate")
        .unwrap();
    // Promoting a name match to a resolution is exactly what the graph must refuse.
    candidate.resolution = "structural".into();
    assert!(forged.verify(&catalog).is_err());
    let golden = String::from_utf8(fs::read(fixture().join("graph-v2.json")).unwrap()).unwrap();
    let extra = golden.replacen("\"version\"", "\"resolved\": true,\n  \"version\"", 1);
    assert!(serde_json::from_str::<v2::Graph>(&extra).is_err());
}
