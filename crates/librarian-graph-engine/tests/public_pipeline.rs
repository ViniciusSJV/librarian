use librarian_core::LibrarianFact;
use librarian_graph_engine::{filesystem::SourceChecks, prepare_query, Evidence, Source};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture {
    dir: PathBuf,
    evidence: Evidence,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "public-graph-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let bytes = b"Manual ficticio\nInspecao mensal\nRegistrar resultado\n";
        let path = dir.join("manual.txt");
        fs::write(&path, bytes).unwrap();
        let source: Source = serde_json::from_value(json!({
            "id":"S1", "kind":"maintenance_manual", "path":path,
            "sha256":format!("{:x}",Sha256::digest(bytes)),
            "lines":["Manual ficticio", "Inspecao mensal", "Registrar resultado"]
        }))
        .unwrap();
        Self {
            dir,
            evidence: Evidence {
                id: Some("MAINTENANCE".into()),
                unknowns: Some(vec!["Synthetic data".into()]),
                sources: vec![source],
                facts: vec![
                    LibrarianFact::new("F1", "Inspecao mensal", "S1", 2),
                    LibrarianFact::new("F2", "Registrar resultado", "S1", 3),
                ],
            },
        }
    }
    fn dossier(&self) -> String {
        serde_json::to_string(&self.evidence).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn other_domain_exports_shared_core_contracts_in_selected_order() {
    let f = Fixture::new();
    let dossier = f.dossier();
    let mut checks = SourceChecks::default();
    let prepared = prepare_query(&dossier, Some("Cite as fontes."), &["F2", "F1"], 1, |s| {
        checks.validate(s)
    })
    .unwrap();
    assert_eq!(prepared.selected_fact_ids(), &["F2", "F1"]);
    let query: Value = serde_json::from_str(prepared.query_json().unwrap()).unwrap();
    assert_eq!(query["evidence"]["selections"][0]["fact_id"], "F2");
    assert_eq!(query["evidence"]["contexts"].as_array().unwrap().len(), 1);
    assert_eq!(
        query["evidence"]["selections"][0]["evidence_unknowns"],
        json!(["Synthetic data"])
    );
    let dir = f.dir.join("bundle");
    let origin = prepared
        .write_bundle(&dir, "memory:dossier", "memory:question")
        .unwrap();
    assert_eq!(
        fs::read_to_string(dir.join("dossier.json")).unwrap(),
        dossier
    );
    let bytes = fs::read(dir.join("query.json")).unwrap();
    assert_eq!(
        origin["query"]["sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert!(prepared.write_bundle(&dir, "other", "other").is_err());
    assert_eq!(fs::read(dir.join("query.json")).unwrap(), bytes);
}

#[test]
fn changed_unselected_source_blocks_query_and_preserves_old_bundle() {
    let mut f = Fixture::new();
    let mut other = f.evidence.sources[0].clone();
    other.id = "UNSELECTED".into();
    let path = f.dir.join("other.txt");
    fs::copy(f.dir.join("manual.txt"), &path).unwrap();
    other.path = Some(path.to_string_lossy().into_owned());
    f.evidence.sources.push(other);
    let dossier = f.dossier();
    let mut checks = SourceChecks::default();
    let prepared = prepare_query(&dossier, Some("Pergunta"), &["F1"], 0, |s| {
        checks.validate(s)
    })
    .unwrap();
    let bundle = f.dir.join("baseline");
    prepared.write_bundle(&bundle, "d", "q").unwrap();
    let baseline = fs::read(bundle.join("dossier.json")).unwrap();
    fs::write(path, b"Changed edition").unwrap();
    let mut checks = SourceChecks::default();
    assert!(
        prepare_query(&dossier, Some("Pergunta"), &["F1"], 0, |s| checks
            .validate(s))
        .unwrap_err()
        .contains("SHA-256 diferente")
    );
    assert_eq!(fs::read(bundle.join("dossier.json")).unwrap(), baseline);
}

#[test]
fn rejects_invalid_references_selections_and_report_outside_selection() {
    let mut f = Fixture::new();
    f.evidence.facts[1].line = 99;
    assert!(prepare_query(&f.dossier(), None, &["F1"], 0, |_| Ok(None)).is_err());
    f.evidence.facts[1].line = 3;
    for ids in [vec![], vec!["missing"], vec!["F1", "F1"]] {
        assert!(prepare_query(&f.dossier(), None, &ids, 0, |_| Ok(None)).is_err());
    }
    let mut report = f.evidence.sources[0].clone();
    report.id = "UNSELECTED_REPORT".into();
    report.execution = Some(
        serde_json::from_value(
            json!({"id":"E", "command":"cmd", "started_at":"a", "finished_at":"b", "exit_code":0}),
        )
        .unwrap(),
    );
    f.evidence.sources.push(report);
    assert!(prepare_query(&f.dossier(), None, &["F1"], 0, |_| Ok(None))
        .unwrap_err()
        .contains("execution exige kind test_run"));
}

#[test]
fn checks_each_source_once_and_requires_question_for_bundle() {
    let f = Fixture::new();
    let mut ids = Vec::new();
    let prepared = prepare_query(&f.dossier(), None, &["F2", "F1"], 0, |s| {
        ids.push(s.id.clone());
        Ok(None)
    })
    .unwrap();
    assert_eq!(ids, vec!["S1"]);
    let destination = f.dir.join("no-question");
    assert!(prepared.write_bundle(&destination, "d", "q").is_err());
    assert!(!destination.exists());
}

#[test]
fn rejects_empty_fields_even_on_unselected_facts() {
    for field in ["id", "statement", "source_id"] {
        let f = Fixture::new();
        let mut dossier: Value = serde_json::from_str(&f.dossier()).unwrap();
        dossier["facts"][1][field] = json!(" \t");
        let error = prepare_query(&dossier.to_string(), Some("Question"), &["F1"], 0, |_| {
            Ok(None)
        })
        .unwrap_err();
        assert!(
            error.contains(&format!("fact.{field} must not be empty")),
            "{error}"
        );
    }
    let mut f = Fixture::new();
    f.evidence.sources[0].id = " ".into();
    f.evidence
        .facts
        .iter_mut()
        .for_each(|fact| fact.source_id = " ".into());
    assert!(prepare_query(&f.dossier(), None, &["F1"], 0, |_| Ok(None))
        .unwrap_err()
        .contains("source.id must not be empty"));
}

#[test]
fn public_boundary_rejects_duplicate_sources_empty_question_and_callback_failure() {
    let mut f = Fixture::new();
    f.evidence.sources.push(f.evidence.sources[0].clone());
    assert!(prepare_query(&f.dossier(), None, &["F1"], 0, |_| Ok(None)).is_err());
    f.evidence.sources.pop();
    assert!(prepare_query(&f.dossier(), Some(" \n"), &["F1"], 0, |_| Ok(None)).is_err());
    assert!(prepare_query(&f.dossier(), Some("Q"), &["F1"], 0, |_| Err(
        "storage unavailable".into()
    ))
    .unwrap_err()
    .contains("storage unavailable"));
    let prepared =
        prepare_query(&f.dossier(), Some("Q"), &["F1"], usize::MAX, |_| Ok(None)).unwrap();
    let query: Value = serde_json::from_str(prepared.query_json().unwrap()).unwrap();
    assert_eq!(
        query["evidence"]["source"]["context"]["lines"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}
