//! Frozen v1 edition generated once by `librarian-ingest/0.1.0;syn/3.0.5;rust-structural-v1`.
//! These fixtures are never regenerated to make a test pass; a mismatch means behavior changed.
use librarian_ingest::{
    generate_with_profile,
    graph::Graph,
    hash,
    search::{Lexicon, SearchIndex},
    Catalog, Config, EXTRACTOR,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
const RECORDS: [&str; 5] = [
    "manifest.json",
    "sources.jsonl",
    "symbols.jsonl",
    "chunks.jsonl",
    "diagnostics.json",
];

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1")
}

fn read(path: impl AsRef<Path>) -> Vec<u8> {
    fs::read(path).unwrap()
}

fn load() -> Catalog {
    Catalog::load(&fixture().join("edition")).unwrap()
}

/// Temporary copy outside any Git checkout, so generated metadata stays `null`.
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "librarian-v1-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
    fn copy(&self, from: &Path, name: &str) -> PathBuf {
        let to = self.0.join(name);
        copy_dir(from, &to);
        to
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Rewrites one record and recomputes its manifest hash, as a forger would.
fn forge(edition: &Path, record: &str, edit: impl FnOnce(String) -> String) {
    let original = String::from_utf8(read(edition.join(record))).unwrap();
    let forged = edit(original.clone());
    assert_ne!(original, forged, "forgery did not change {record}");
    fs::write(edition.join(record), &forged).unwrap();
    let mut manifest: Value = serde_json::from_slice(&read(edition.join("manifest.json"))).unwrap();
    manifest["files"][record] = hash(forged.as_bytes()).into();
    fs::write(
        edition.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

#[test]
fn frozen_v1_edition_is_still_accepted() {
    let catalog = load();
    let manifest = catalog.manifest();
    assert_eq!(manifest.extractor, EXTRACTOR);
    assert_eq!(
        (
            manifest.source_count,
            manifest.symbol_count,
            manifest.diagnostic_count
        ),
        (9, 34, 2)
    );
    let codes: Vec<_> = catalog
        .diagnostics()
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert_eq!(codes, ["rust_parse_error", "invalid_utf8"]);
}

#[test]
fn regenerating_the_fixture_project_reproduces_every_record_and_snapshot() {
    let scratch = Scratch::new();
    let project = scratch.copy(&fixture().join("project"), "project");
    let config: Config = serde_json::from_slice(&read(fixture().join("config.json"))).unwrap();
    let edition = scratch.0.join("edition");
    generate_with_profile(EXTRACTOR, &project, &config, &edition).unwrap();
    for record in RECORDS {
        assert_eq!(
            read(edition.join(record)),
            read(fixture().join("edition").join(record)),
            "{record} changed"
        );
    }
    let names = |dir: &Path| {
        let mut names: Vec<_> = fs::read_dir(dir.join("snapshots"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        names
    };
    assert_eq!(names(&edition), names(&fixture().join("edition")));
}

#[test]
fn v1_discovery_and_classification_stay_frozen() {
    let catalog = load();
    let helper = catalog.source_by_path("scripts/helper.py").unwrap();
    // A future Python extractor must not reinterpret this historical auxiliary file.
    assert_eq!(helper.language, "auxiliary");
    assert!(catalog.symbols().values().all(|s| s.source_id != helper.id));
    // Directories contribute only .rs files under v1.
    assert!(catalog.source_by_path("src/ignored.py").is_none());
    let scratch = Scratch::new();
    let project = scratch.copy(&fixture().join("project"), "project");
    assert!(catalog.compare_current(&project).unwrap().is_empty());
}

#[test]
fn graph_matches_frozen_v1_golden() {
    let catalog = load();
    let graph = Graph::build(&catalog);
    assert_eq!(graph.version, "rust-syntax-graph-v1");
    assert_eq!(
        serde_json::to_vec_pretty(&graph).unwrap(),
        read(fixture().join("graph.json"))
    );
    let golden: Graph = serde_json::from_slice(&read(fixture().join("graph.json"))).unwrap();
    golden.verify(&catalog).unwrap();
}

#[test]
fn graph_nodes_serialize_as_object_indexed_by_id() {
    // Consumers such as the graph visualizer must not treat nodes as an array.
    let golden: Value = serde_json::from_slice(&read(fixture().join("graph.json"))).unwrap();
    let nodes = golden["nodes"].as_object().unwrap();
    assert!(nodes.iter().all(|(id, node)| node["id"] == id.as_str()));
    assert!(golden["edges"].is_array());
}

#[test]
fn search_matches_frozen_v1_goldens() {
    let catalog = load();
    let lexicon: Lexicon = serde_json::from_slice(&read(fixture().join("lexicon.json"))).unwrap();
    let index = SearchIndex::new(&catalog, lexicon).unwrap();
    let plain = SearchIndex::new(&catalog, Lexicon::default()).unwrap();
    for (golden, index, question, top, depth) in [
        (
            "search-camera.json",
            &index,
            "Onde a câmera calcula a direção dos raios?",
            5,
            2,
        ),
        ("search-abstain.json", &index, "quantum database", 3, 1),
        // Includes a #[test] symbol, so the v1 is_test penalty is part of the golden.
        ("search-test-penalty.json", &plain, "render hit", 5, 1),
    ] {
        let report = index.search(question, top, depth).unwrap();
        let printed = serde_json::to_string_pretty(&report).unwrap() + "\n";
        assert_eq!(
            printed.as_bytes(),
            read(fixture().join(golden)).as_slice(),
            "{golden} changed"
        );
    }
}

#[test]
fn tampered_copies_of_the_frozen_edition_are_rejected() {
    type Tamper = fn(&Path);
    let cases: [(&str, Tamper); 6] = [
        ("snapshot byte", |e| {
            let catalog = Catalog::load(e).unwrap();
            let camera = catalog.source_by_path("src/camera.rs").unwrap();
            let path = e.join("snapshots").join(&camera.sha256);
            let mut bytes = read(&path);
            bytes[0] ^= 1;
            fs::write(path, bytes).unwrap();
        }),
        ("forged range with recomputed record hash", |e| {
            forge(e, "chunks.jsonl", |text| {
                let line = text.lines().next().unwrap();
                let chunk: Value = serde_json::from_str(line).unwrap();
                let end = chunk["end_byte"].as_u64().unwrap();
                let forged = line.replace(
                    &format!("\"end_byte\":{end}"),
                    &format!("\"end_byte\":{}", end - 1),
                );
                text.replacen(line, &forged, 1)
            })
        }),
        ("auxiliary relabeled as python", |e| {
            forge(e, "sources.jsonl", |text| {
                let line = text
                    .lines()
                    .find(|l| l.contains("scripts/helper.py"))
                    .unwrap();
                text.replacen(line, &line.replace("\"auxiliary\"", "\"python\""), 1)
            })
        }),
        ("dropped diagnostic", |e| {
            forge(e, "diagnostics.json", |text| {
                let mut diagnostics: Vec<Value> = serde_json::from_str(&text).unwrap();
                diagnostics.pop();
                String::from_utf8(serde_json::to_vec_pretty(&diagnostics).unwrap()).unwrap()
            })
        }),
        ("unknown extractor", |e| {
            let path = e.join("manifest.json");
            let text = String::from_utf8(read(&path)).unwrap();
            fs::write(
                path,
                text.replace("rust-structural-v1", "rust-structural-v2"),
            )
            .unwrap();
        }),
        ("unknown schema", |e| {
            let path = e.join("manifest.json");
            let text = String::from_utf8(read(&path)).unwrap();
            fs::write(
                path,
                text.replace("\"schema_version\": 1", "\"schema_version\": 2"),
            )
            .unwrap();
        }),
    ];
    let scratch = Scratch::new();
    Catalog::load(&scratch.copy(&fixture().join("edition"), "edition")).unwrap();
    for (name, tamper) in cases {
        let scratch = Scratch::new();
        let edition = scratch.copy(&fixture().join("edition"), "edition");
        tamper(&edition);
        assert!(Catalog::load(&edition).is_err(), "accepted: {name}");
    }
}
