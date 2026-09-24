use librarian_ingest::{generate, hash, Catalog, Config};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

#[test]
fn lexical_tokens_handle_portuguese_and_identifier_boundaries() {
    use librarian_ingest::search::tokens;
    assert_eq!(
        tokens("Câmera ray_from_pixel HTTPServer direção"),
        ["camera", "ray", "from", "pixel", "http", "server", "direcao"]
    );
    assert_eq!(tokens("direc\u{0327}a\u{0303}o"), ["direcao"]);
}

#[test]
fn lexical_search_expands_explicit_aliases_and_abstains() {
    use librarian_ingest::search::{Lexicon, SearchIndex};
    let f = Fixture::new();
    fs::write(f.0.join("src/lib.rs"), "struct Camera; struct Ray; impl Camera { fn ray_from_pixel(&self) -> Ray { let direction = 1; Ray } } #[test] fn camera_ray_direction() {}").unwrap();
    f.generate();
    let catalog = Catalog::load(&f.edition()).unwrap();
    let lexicon = Lexicon {
        groups: vec![
            vec!["raios".into(), "ray".into()],
            vec!["direcao".into(), "direction".into()],
        ],
        stop_words: vec!["calcula".into()],
    };
    let index = SearchIndex::new(&catalog, lexicon).unwrap();
    let report = index
        .search("Onde a câmera calcula a direção dos raios?", 5, 2)
        .unwrap();
    assert_eq!(report.hits[0].name, "impl Camera::ray_from_pixel");
    assert_eq!(report.hits[0].reasons.len(), 3);
    assert!(report.hits[0].excerpt.contains("let direction"));
    assert_eq!(
        index.search("quantum database", 3, 1).unwrap().status,
        "no_lexical_evidence"
    );
    assert!(index.search("onde a", 3, 1).unwrap().hits.is_empty());
    assert!(index.search("", 3, 1).is_err());
    assert!(index.search("camera", 0, 1).is_err());
    assert!(index.search("camera", 3, 4).is_err());
    let again = index
        .search("Onde a câmera calcula a direção dos raios?", 5, 2)
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&report).unwrap(),
        serde_json::to_vec(&again).unwrap()
    );
}

#[test]
fn graph_keeps_calls_unresolved_and_ambiguous_types_as_candidates() {
    use librarian_ingest::graph::Graph;
    let f = Fixture::new();
    let code = "\u{feff}// café\r\nstruct Camera; struct Ray; impl Camera { fn shoot(&self) -> Ray { let direction = 1; Ray::new(direction) } }";
    fs::write(f.0.join("src/lib.rs"), code).unwrap();
    fs::write(f.0.join("src/other.rs"), "struct Ray;").unwrap();
    f.generate();
    let catalog = Catalog::load(&f.edition()).unwrap();
    let graph = Graph::build(&catalog);
    graph.verify(&catalog).unwrap();
    let call = graph
        .nodes
        .values()
        .find(|n| n.kind == "call_observed" && n.label == "Ray::new")
        .unwrap();
    assert!(!graph.edges.iter().any(|e| e.from == call.id));
    let ret = graph
        .nodes
        .values()
        .find(|n| n.kind == "type_reference" && n.label == "Ray")
        .unwrap();
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(|e| e.from == ret.id && e.resolution == "name_candidate")
            .count(),
        2
    );
    for edge in &graph.edges {
        let evidence = &edge.evidence;
        let bytes = if evidence.path == "src/lib.rs" {
            code.as_bytes()
        } else {
            b"struct Ray;"
        };
        assert_eq!(
            hash(&bytes[evidence.start_byte..evidence.end_byte]),
            evidence.excerpt_sha256
        );
    }
    let seed = &catalog.symbols_named("shoot")[0].id;
    let neighborhood = graph.neighborhood(seed, 3, 2, 1).unwrap();
    assert!(neighborhood.truncated);
    assert!(neighborhood.nodes.len() <= 2 && neighborhood.edges.len() <= 1);
    assert_eq!(graph.neighborhood(seed, 0, 2, 1).unwrap().nodes.len(), 1);
    let mut forged = graph.clone();
    forged.edges[0].resolution = "compiler_resolved".into();
    assert!(forged.verify(&catalog).is_err());
}

#[test]
fn lexical_excerpts_respect_utf8_and_report_truncation() {
    use librarian_ingest::search::{Lexicon, SearchIndex};
    let f = Fixture::new();
    fs::write(
        f.0.join("src/lib.rs"),
        format!(
            "fn huge() {{ let text = \"{}\".to_owned(); }}",
            "🦀".repeat(2000)
        ),
    )
    .unwrap();
    f.generate();
    let catalog = Catalog::load(&f.edition()).unwrap();
    let report = SearchIndex::new(&catalog, Lexicon::default())
        .unwrap()
        .search("huge", 1, 1)
        .unwrap();
    assert!(report.hits[0].excerpt_truncated);
    assert!(report.hits[0].excerpt.len() <= 4000);
    let neighborhood = report.graph.unwrap();
    assert!(neighborhood.labels_truncated > 0);
    assert!(neighborhood.nodes.iter().all(|n| n.label.len() <= 256));
    let invalid = Lexicon {
        groups: vec![vec!["ray".into()], vec!["ray".into()]],
        stop_words: vec![],
    };
    assert!(SearchIndex::new(&catalog, invalid).is_err());
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "librarian-ingest-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        fs::create_dir(dir.join("src")).unwrap();
        Self(dir)
    }
    fn config(&self) -> Config {
        Config {
            project: "fixture".into(),
            inputs: vec!["src".into()],
        }
    }
    fn edition(&self) -> PathBuf {
        self.0.join("edition")
    }
    fn generate(&self) {
        generate(&self.0, &self.config(), &self.edition()).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn extracts_real_syntax_and_exact_unicode_crlf_bom_shebang_spans() {
    let f = Fixture::new();
    let code = "\u{feff}#!/usr/bin/env rust-script\r\n// café 🦀\r\npub struct Café;\r\nimpl Café {\r\n    /// Olá\r\n    pub fn ação(&self) -> &str { \"} não é código\" }\r\n}\r\n#[cfg(test)]\r\nmod tests { #[test] fn exemplo() { assert!(true); } }\r\n";
    fs::write(f.0.join("src/lib.rs"), code).unwrap();
    f.generate();
    let catalog = Catalog::load(&f.edition()).unwrap();
    assert!(catalog.diagnostics().is_empty());
    let method = catalog.symbols_named("ação")[0];
    assert_eq!(method.kind, "method");
    assert_eq!(method.qualified_name, "impl Café::ação");
    let chunk = catalog.chunk(&method.chunk_id).unwrap();
    let excerpt = std::str::from_utf8(catalog.excerpt(&method.chunk_id).unwrap()).unwrap();
    assert!(excerpt.starts_with("/// Olá"), "{excerpt}");
    assert!(excerpt.ends_with("\"} não é código\" }"), "{excerpt}");
    assert_eq!(chunk.start_line, 5);
    assert_eq!(chunk.end_line, 6);
    assert_eq!(
        &code.as_bytes()[chunk.start_byte..chunk.end_byte],
        excerpt.as_bytes()
    );
    assert_eq!(chunk.sha256, hash(excerpt.as_bytes()));
    let test = catalog.symbols_named("exemplo")[0];
    assert!(test.is_test);
    assert_eq!(test.qualified_name, "tests::exemplo");
}

#[test]
fn reproducible_records_and_snapshots_with_deduplicated_inputs() {
    let f = Fixture::new();
    fs::write(f.0.join("src/lib.rs"), "fn one() {}\n").unwrap();
    fs::write(f.0.join("src/other.rs"), "fn two() {}\n").unwrap();
    fs::write(
        f.0.join("src/ignored.txt"),
        "auxiliary only if explicitly selected",
    )
    .unwrap();
    let config = Config {
        project: "fixture".into(),
        inputs: vec!["src".into(), "src/lib.rs".into()],
    };
    let a = generate(&f.0, &config, &f.edition()).unwrap();
    let other = f.0.join("other");
    let b = generate(&f.0, &config, &other).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.source_count, 2);
    for name in a
        .files
        .keys()
        .chain(std::iter::once(&"manifest.json".to_string()))
    {
        assert_eq!(
            fs::read(f.edition().join(name)).unwrap(),
            fs::read(other.join(name)).unwrap()
        );
    }
    let before = fs::read(f.edition().join("manifest.json")).unwrap();
    assert!(generate(&f.0, &config, &f.edition()).is_err());
    assert_eq!(before, fs::read(f.edition().join("manifest.json")).unwrap());
}

#[test]
fn historical_verification_survives_current_changes_and_detects_tampering() {
    let f = Fixture::new();
    fs::write(f.0.join("src/lib.rs"), "fn old() {}\n").unwrap();
    f.generate();
    let catalog = Catalog::load(&f.edition()).unwrap();
    assert!(catalog.compare_current(&f.0).unwrap().is_empty());
    fs::write(f.0.join("src/lib.rs"), "fn new() {}\n").unwrap();
    fs::write(f.0.join("src/new.rs"), "fn added() {}\n").unwrap();
    let changes = catalog.compare_current(&f.0).unwrap();
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].status, "changed");
    assert_eq!(changes[1].status, "added");
    Catalog::load(&f.edition()).unwrap();
    let source = catalog.source_by_path("src/lib.rs").unwrap();
    fs::write(
        f.edition().join("snapshots").join(&source.sha256),
        "fn tampered() {}",
    )
    .unwrap();
    assert!(Catalog::load(&f.edition()).is_err());
}

#[test]
fn rejects_forged_ranges_even_if_record_hash_was_recomputed() {
    let f = Fixture::new();
    fs::write(f.0.join("src/lib.rs"), "fn one() {}\n").unwrap();
    f.generate();
    let path = f.edition().join("chunks.jsonl");
    let mut chunk: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    chunk["end_byte"] = serde_json::json!(9999);
    let bytes = serde_json::to_vec(&chunk).unwrap();
    fs::write(&path, &bytes).unwrap();
    let manifest_path = f.edition().join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["files"]["chunks.jsonl"] = serde_json::json!(hash(&bytes));
    fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(Catalog::load(&f.edition()).is_err());
}

#[test]
fn parse_and_encoding_failures_are_preserved_and_reported() {
    let f = Fixture::new();
    fs::write(f.0.join("src/broken.rs"), "fn broken( {").unwrap();
    fs::write(f.0.join("src/encoding.rs"), [0xff, 0xfe]).unwrap();
    fs::write(f.0.join("src/good.rs"), "fn good() {}").unwrap();
    f.generate();
    let catalog = Catalog::load(&f.edition()).unwrap();
    assert_eq!(catalog.sources().len(), 3);
    assert_eq!(catalog.diagnostics().len(), 2);
    assert_eq!(catalog.symbols_named("good").len(), 1);
}

#[test]
fn refuses_unsafe_paths_incomplete_editions_and_output_in_input() {
    let f = Fixture::new();
    fs::write(f.0.join("src/lib.rs"), "fn one() {}").unwrap();
    for path in [
        "../outside",
        "C:/outside",
        "/outside",
        "src/../src",
        "src\\lib.rs",
    ] {
        let config = Config {
            project: "x".into(),
            inputs: vec![path.into()],
        };
        assert!(generate(&f.0, &config, &f.edition()).is_err());
    }
    assert!(generate(&f.0, &f.config(), &f.0.join("src/edition")).is_err());
    fs::create_dir(f.edition()).unwrap();
    assert!(Catalog::load(&f.edition()).is_err());
}

#[test]
fn cfg_items_are_observed_but_macros_are_not_expanded() {
    let f = Fixture::new();
    fs::write(f.0.join("src/lib.rs"), "#[cfg(any())] fn disabled() {} macro_rules! make { () => { fn generated() {} } } make!(); trait T { fn go(&self); } impl T for X { fn go(&self) {} }").unwrap();
    f.generate();
    let catalog = Catalog::load(&f.edition()).unwrap();
    assert_eq!(catalog.symbols_named("disabled").len(), 1);
    assert_eq!(catalog.symbols_named("generated").len(), 0);
    assert_eq!(catalog.symbols_named("go").len(), 2);
    assert!(catalog
        .symbols_named("go")
        .iter()
        .any(|s| s.qualified_name == "impl T for X::go"));
}
