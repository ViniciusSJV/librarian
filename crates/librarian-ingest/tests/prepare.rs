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

fn numbered_request_bytes() -> Vec<u8> {
    fs::read(fixture().join("prepare-request-numbered.json")).unwrap()
}

/// prepare-camera-numbered/ was generated once in L1; only query.json differs from prepare-camera/.
#[test]
fn numbered_bundle_matches_frozen_golden_and_snapshot_lines() {
    let catalog = load();
    let request: Request = serde_json::from_slice(&numbered_request_bytes()).unwrap();
    assert!(request.numbered_lines);
    let prepared = prepare::prepare(&catalog, &request).unwrap();
    let plain = |name: &str| fs::read(fixture().join("prepare-camera").join(name)).unwrap();
    assert_eq!(prepared.search, plain("search.json"));
    assert_eq!(prepared.dossier.as_bytes(), plain("dossier.json"));
    let query = format!("{}\n", prepared.query.query_json().unwrap());
    let golden = fs::read(fixture().join("prepare-camera-numbered/query.json")).unwrap();
    assert_eq!(query.as_bytes(), golden);

    // Every number is the original file line of its text; dossier lines are the checked snapshot.
    let query: Value = serde_json::from_str(&query).unwrap();
    let dossier: Value = serde_json::from_str(&prepared.dossier).unwrap();
    for context in query["evidence"]["contexts"].as_array().unwrap() {
        let source = dossier["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == context["source_id"])
            .unwrap();
        let lines = context["lines"].as_array().unwrap();
        assert_eq!(lines[0]["number"], context["start_line"]);
        assert_eq!(lines.last().unwrap()["number"], context["end_line"]);
        for line in lines {
            let number = line["number"].as_u64().unwrap() as usize;
            assert_eq!(line["text"], source["lines"][number - 1]);
        }
    }
    let operation = r#""number": 21,
            "text": "        let direction = normalize(subtract(pixel, &origin));""#;
    assert!(String::from_utf8(golden).unwrap().contains(operation));
}

#[test]
fn numbered_bundle_refuses_shifted_line_numbers() {
    let catalog = load();
    let scratch = Scratch::new();
    let dir = scratch.0.join("prepared");
    prepare::write(&catalog, &numbered_request_bytes(), &dir).unwrap();
    prepare::verify(&catalog, &dir).unwrap();
    let path = dir.join("bundle/query.json");
    let text = fs::read_to_string(&path).unwrap();
    // The failure seen in G4: a position in the list instead of the file line.
    fs::write(&path, text.replacen("\"number\": 21,", "\"number\": 11,", 1)).unwrap();
    assert!(prepare::verify(&catalog, &dir).is_err());
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

/// A chain step: the target replaces the search, and a reviewed fact rides along as F1.
fn chain_request() -> Value {
    serde_json::json!({
        "question": "Como reduzir alocações em ray_from_pixel?",
        "top": 5, "depth": 1, "hits": 1, "radius": 0, "numbered_lines": true, "accept_diagnostics": true,
        "targets": [{"path": "src/camera.rs", "symbol_name": "ray_from_pixel", "declaration_line": 18}],
        "reviewed_facts": [{"id": "F1", "statement": "ray_from_pixel calcula direction na linha 21.",
            "authorship": "revisão humana no Librarian Studio", "path": "src/camera.rs", "line": 21, "end_line": 21,
            "citations": [{"path": "src/camera.rs", "start_line": 21, "end_line": 21,
                "quote": "        let direction = normalize(subtract(pixel, &origin));"}]}]
    })
}

#[test]
fn chain_steps_take_targets_instead_of_search_and_recheck_reviewed_facts() {
    let catalog = load();
    let bytes = serde_json::to_vec_pretty(&chain_request()).unwrap();
    let prepared = prepare::prepare(&catalog, &serde_json::from_slice(&bytes).unwrap()).unwrap();
    let search: Value = serde_json::from_slice(&prepared.search).unwrap();
    assert_eq!((search["version"].as_str(), search["status"].as_str()), (Some("chain-step-v1"), Some("chain")));
    assert_eq!(search["hits"][0]["name"], "impl Camera::ray_from_pixel");
    assert_eq!(search["terms"], serde_json::json!([]));
    let dossier: Value = serde_json::from_str(&prepared.dossier).unwrap();
    assert!(dossier["id"].as_str().unwrap().starts_with("chain:"));
    let ids: Vec<&str> = dossier["facts"].as_array().unwrap().iter().map(|f| f["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["E1", "F1"]);
    assert_eq!(dossier["facts"][1]["authorship"], "revisão humana no Librarian Studio");
    assert_eq!(prepared.query.selected_fact_ids(), ["E1", "F1"]);
    // The F1 window lies inside E1's, so the export keeps one context block.
    let query: Value = serde_json::from_str(prepared.query.query_json().unwrap()).unwrap();
    assert_eq!(query["evidence"]["contexts"].as_array().unwrap().len(), 1);

    let scratch = Scratch::new();
    let dir = scratch.0.join("prepared");
    prepare::write(&catalog, &bytes, &dir).unwrap();
    prepare::verify(&catalog, &dir).unwrap();

    let refused = |edit: &dyn Fn(&mut Value)| {
        let mut request = chain_request();
        edit(&mut request);
        prepare::prepare(&catalog, &serde_json::from_value(request).unwrap()).err().unwrap().to_string()
    };
    // A quote that is no longer the edition's text, a fact outside the target, a missing
    // declaration, repeated targets, and reviewed facts without a chain are all refused.
    assert!(refused(&|r| r["reviewed_facts"][0]["citations"][0]["quote"] = "let direction = 1;".into()).contains("mudou"));
    assert!(refused(&|r| { r["reviewed_facts"][0]["line"] = 15.into(); r["reviewed_facts"][0]["end_line"] = 15.into(); }).contains("outside"));
    assert!(refused(&|r| r["targets"][0]["declaration_line"] = 5.into()).contains("no `ray_from_pixel`"));
    assert!(refused(&|r| { let t = r["targets"][0].clone(); r["targets"].as_array_mut().unwrap().push(t); }).contains("distinct"));
    assert!(refused(&|r| { r.as_object_mut().unwrap().remove("targets"); }).contains("only by a chain"));
    // A source edited after the question makes the recorded chain step fail verification.
    let edited = std::fs::read_to_string(dir.join("request.json")).unwrap().replace("pixel, &origin", "pixel, origin");
    fs::write(dir.join("request.json"), edited).unwrap();
    assert!(prepare::verify(&catalog, &dir).is_err());
}
