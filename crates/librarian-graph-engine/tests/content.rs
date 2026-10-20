//! Source nature declared by `content`, independent of language. Legacy kinds are in
//! legacy_exports.rs; here the engine must behave the same for any language's code.
use librarian_graph_engine::prepare_query;
use serde_json::{json, Value};

fn dossier(sources: Value) -> String {
    let first = sources[0]["id"].as_str().unwrap().to_owned();
    json!({
        "id": "CONTENT",
        "sources": sources,
        "facts": [{"id": "F1", "statement": "linha", "source_id": first, "line": 1}]
    })
    .to_string()
}

fn select(sources: Value) -> Result<Value, String> {
    let prepared = prepare_query(&dossier(sources), None, &["F1"], 0, |_| Ok(None))?;
    Ok(serde_json::from_str(prepared.selection_json()).unwrap())
}

#[test]
fn code_in_any_language_never_exports_execution_claims() {
    for kind in ["python_source", "java_source", "typescript_source", "notes"] {
        let selection = select(json!([{
            "id": "S", "kind": kind, "content": "code", "executed": true,
            "lines": ["def run(): pass"]
        }]))
        .unwrap();
        let source = &selection["source"];
        assert!(source.get("executed").is_none(), "{kind}");
        assert_eq!(source["content"], "code");
        assert!(source["execution_scope"]
            .as_str()
            .unwrap()
            .contains("não informa se, quando ou com qual resultado"));
        let scope = selection["metadata_scope"].as_str().unwrap();
        assert!(scope.ends_with("omitido para fontes de código."), "{scope}");
    }
}

#[test]
fn execution_records_are_declared_by_content() {
    let selection = select(json!([{
        "id": "R", "kind": "pytest_log", "content": "execution_record",
        "execution": {"id": "R1", "command": "pytest", "started_at": "a", "finished_at": "b", "exit_code": 0},
        "lines": ["Comando: pytest", "Início UTC: a", "Fim UTC: b", "Código de término: 0", "", "1 passed"]
    }]))
    .unwrap();
    assert_eq!(
        selection["source"]["execution_validation"]["status"],
        "matches_report_header"
    );
    // An execution record on a source that is not declared as one is refused.
    let refused = select(json!([{
        "id": "D", "kind": "pytest_log", "content": "document",
        "execution": {"id": "R1", "command": "pytest", "started_at": "a", "finished_at": "b", "exit_code": 0},
        "lines": ["Comando: pytest"]
    }]));
    assert!(refused.unwrap_err().contains("execution exige"));
}

#[test]
fn documents_and_undeclared_sources_keep_declared_metadata() {
    for content in [Some("document"), None] {
        let mut source = json!({"id": "N", "kind": "notes", "executed": false, "lines": ["texto"]});
        if let Some(c) = content {
            source["content"] = json!(c);
        }
        let selection = select(json!([source])).unwrap();
        assert_eq!(selection["source"]["executed"], false);
        assert!(selection["source"].get("execution_scope").is_none());
        assert_eq!(selection["source"].get("content").is_some(), content.is_some());
    }
}

#[test]
fn unknown_or_contradictory_content_is_refused() {
    let unknown = select(json!([{"id": "S", "content": "codigo", "lines": ["x"]}]));
    assert!(unknown.unwrap_err().contains("content desconhecido"));
    // A legacy kind keeps its meaning; content may only agree with it.
    let contradiction = select(json!([{"id": "S", "kind": "test_run", "content": "code", "lines": ["x"]}]));
    assert!(contradiction.unwrap_err().contains("contradiz"));
    let agreement = select(json!([{"id": "S", "kind": "test_run", "content": "execution_record", "lines": ["x"]}]));
    assert!(agreement.is_ok());
}
