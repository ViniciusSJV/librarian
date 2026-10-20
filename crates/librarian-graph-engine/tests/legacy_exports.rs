//! Exports of v0.1 dossiers frozen before G3 (source nature declared by `content`).
//! Legacy `kind` values must keep producing these bytes; never regenerate them to pass.
use librarian_graph_engine::prepare_query;
use serde_json::{json, Value};
use std::{fs, path::Path};

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/legacy-v0.1")
        .join(name);
    String::from_utf8(fs::read(path).unwrap()).unwrap()
}

/// (golden, facts, radius, question, source whose capture check reports a validation)
type Case = (&'static str, &'static [&'static str], usize, Option<&'static str>, Option<&'static str>);

const CASES: [Case; 4] = [
    ("selection-code.json", &["F-CODE"], 1, None, None),
    ("selection-all.json", &["F-CODE", "F-RUN", "F-RUN-LEGACY", "F-NOTE"], 1, None, None),
    ("query-all.json", &["F-NOTE", "F-RUN", "F-CODE"], 2, Some("O que os registros mostram?"), None),
    ("selection-capture.json", &["F-CODE"], 0, None, Some("CODE")),
];

fn export(case: &Case) -> String {
    let (_, facts, radius, question, captured) = *case;
    let prepared = prepare_query(&fixture("dossier.json"), question, facts, radius, |source| {
        Ok((Some(source.id.as_str()) == captured)
            .then(|| json!({"status": "matches", "basis": "synthetic"})))
    })
    .unwrap();
    match question {
        Some(_) => prepared.query_json().unwrap().to_owned(),
        None => prepared.selection_json().to_owned(),
    }
}

#[test]
fn legacy_kinds_export_frozen_bytes() {
    for case in &CASES {
        assert_eq!(export(case), fixture(case.0), "{} changed", case.0);
    }
}

#[test]
fn legacy_code_sources_never_export_execution_claims() {
    let selection: Value = serde_json::from_str(&fixture("selection-code.json")).unwrap();
    let source = &selection["source"];
    assert!(source.get("executed").is_none());
    assert!(source["execution_scope"]
        .as_str()
        .unwrap()
        .contains("não informa se, quando ou com qual resultado"));
}
