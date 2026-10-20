//! Bridge from verified search to a validated query bundle (G4).
//! prepare-camera/ was generated once in G4 from prepare-request.json; never regenerate to pass.
use librarian_ingest::{
    prepare::{self, Request},
    Catalog,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1")
}

fn load() -> Catalog {
    Catalog::load(&fixture().join("edition")).unwrap()
}

fn request_bytes() -> Vec<u8> {
    fs::read(fixture().join("prepare-request.json")).unwrap()
}

fn request() -> Request {
    serde_json::from_slice(&request_bytes()).unwrap()
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "librarian-prepare-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn prepared_bundle_matches_frozen_golden() {
    let prepared = prepare::prepare(&load(), &request()).unwrap();
    let golden = |name: &str| fs::read(fixture().join("prepare-camera").join(name)).unwrap();
    assert_eq!(prepared.search, golden("search.json"));
    assert_eq!(prepared.dossier.as_bytes(), golden("dossier.json"));
    let query = format!("{}\n", prepared.query.query_json().unwrap());
    assert_eq!(query.as_bytes(), golden("query.json"));
}

#[test]
fn facts_are_structural_ranges_backed_by_snapshots() {
    let catalog = load();
    let prepared = prepare::prepare(&catalog, &request()).unwrap();
    let dossier: Value = serde_json::from_str(&prepared.dossier).unwrap();
    let e1 = &dossier["facts"][0];
    assert_eq!(e1["id"], "E1");
    assert_eq!(
        e1["statement"],
        "method `impl Camera::ray_from_pixel` declarado em src/camera.rs, linhas 18–23."
    );
    let source = &dossier["sources"][0];
    assert_eq!(source["content"], "code");
    assert_eq!(source["kind"], "rust");
    // The exported context covers the whole declaration plus the requested radius.
    let query: Value = serde_json::from_str(prepared.query.query_json().unwrap()).unwrap();
    let selection = &query["evidence"]["selections"][0]["source"];
    assert_eq!(selection["end_line"], 23);
    let context_id = &selection["context"]["context_id"];
    let context = query["evidence"]["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| &c["id"] == context_id)
        .unwrap();
    assert_eq!((context["start_line"].as_u64(), context["end_line"].as_u64()), (Some(17), Some(24)));
    let camera = catalog.source_by_path("src/camera.rs").unwrap();
    let declaration = catalog
        .symbols_named("ray_from_pixel")
        .into_iter()
        .find(|s| s.source_id == camera.id)
        .unwrap();
    let excerpt = std::str::from_utf8(catalog.excerpt(&declaration.chunk_id).unwrap()).unwrap();
    let lines: Vec<&str> = context["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert!(lines.join("\n").contains(excerpt));
    assert_eq!(query["instructions"][0], "Responda citando E1 e E2.");
}

#[test]
fn written_bundle_verifies_and_detects_tampering() {
    let catalog = load();
    for (tampered, edit) in [
        ("search.json", "\"candidates\""),
        ("bundle/query.json", "Responda citando"),
        ("bundle/dossier.json", "declarado em"),
        ("request.json", "câmera"),
    ] {
        let scratch = Scratch::new();
        let dir = scratch.0.join("prepared");
        prepare::write(&catalog, &request_bytes(), &dir).unwrap();
        prepare::verify(&catalog, &dir).unwrap();
        let path = dir.join(tampered);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(edit), "{tampered}");
        fs::write(&path, text.replacen(edit, &format!("{edit} "), 1)).unwrap();
        assert!(prepare::verify(&catalog, &dir).is_err(), "accepted: {tampered}");
    }
}

#[test]
fn refuses_incomplete_editions_missing_candidates_and_bad_requests() {
    let catalog = load();
    let mut strict = request();
    strict.accept_diagnostics = false;
    let error = prepare::prepare(&catalog, &strict).err().unwrap().to_string();
    assert!(error.contains("diagnostics"), "{error}");

    let mut abstain = request();
    abstain.question = "quantum database".into();
    assert!(prepare::prepare(&catalog, &abstain).is_err());

    let mut too_many = request();
    too_many.hits = too_many.top + 1;
    assert!(prepare::prepare(&catalog, &too_many).is_err());

    let mut empty_policy = request();
    empty_policy.instructions = Some(vec![]);
    assert!(prepare::prepare(&catalog, &empty_policy).is_err());

    let unknown = String::from_utf8(request_bytes())
        .unwrap()
        .replacen("\"top\"", "\"approved\": true,\n  \"top\"", 1);
    assert!(serde_json::from_str::<Request>(&unknown).is_err());
}

#[test]
fn defaults_keep_graph_engine_instructions() {
    let mut defaults = request();
    defaults.instructions = None;
    let prepared = prepare::prepare(&load(), &defaults).unwrap();
    let query: Value = serde_json::from_str(prepared.query.query_json().unwrap()).unwrap();
    assert_eq!(
        query["instructions"],
        serde_json::json!(librarian_graph_engine::DEFAULT_INSTRUCTIONS)
    );
}
