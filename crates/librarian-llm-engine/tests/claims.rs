use librarian_llm_engine::claims::{self, Context};
use serde_json::{json, Value};

#[test]
fn structured_format_is_optional_and_does_not_rewrite_prompt() {
    use librarian_llm_engine::ollama::request;
    let q = claims::query(&context(), "Where?").unwrap();
    let value: Value = serde_json::from_str(&q).unwrap();
    let body = request(&q, "model-a").unwrap();
    assert_eq!(body["prompt"], q);
    assert_eq!(body["format"], value["response_schema"]);
    assert_eq!(request(&q, "model-b").unwrap()["format"], body["format"]);
    assert_eq!(
        body["format"]["properties"]["claims"]["items"]["properties"]["kind"]["enum"],
        json!(["FACT", "LACUNA"])
    );
    let mut c = context();
    c.policy.allow_hypothesis = true;
    let schema = claims::response_schema(&c.policy);
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["kind"]["enum"],
        json!(["FACT", "LACUNA", "HYPOTHESIS"])
    );
    let legacy = r#"{"question":"Q","evidence":{},"instructions":["I"]}"#;
    assert_eq!(
        request(legacy, "M").unwrap(),
        json!({"model":"M","prompt":legacy,"stream":false})
    );
    for invalid in [
        Value::Null,
        json!("json"),
        json!([]),
        json!({"type":"array"}),
    ] {
        let mut v: Value = serde_json::from_str(legacy).unwrap();
        v["response_schema"] = invalid;
        assert!(request(&v.to_string(), "M").is_err());
    }
}

#[test]
fn cli_rejects_original_prose_preserves_input_and_refuses_overwrite() {
    use std::{fs, process::Command};
    let dir = std::env::temp_dir().join(format!(
        "claims-cli-{}-{}",
        std::process::id(),
        librarian_llm_engine::ollama::client::now_ms()
    ));
    fs::create_dir(&dir).unwrap();
    let context_path = dir.join("context.json");
    let response_path = dir.join("response.txt");
    let report_path = dir.join("report.json");
    fs::write(
        &context_path,
        include_bytes!("fixtures/camera-context.json"),
    )
    .unwrap();
    let original = include_bytes!("fixtures/camera-legacy-02.txt");
    fs::write(&response_path, original).unwrap();
    let invoke = || {
        Command::new(env!("CARGO_BIN_EXE_claims"))
            .arg("check")
            .arg(&context_path)
            .arg(&response_path)
            .arg(&report_path)
            .output()
            .unwrap()
    };
    assert_eq!(invoke().status.code(), Some(2));
    let report = fs::read(&report_path).unwrap();
    let value: Value = serde_json::from_slice(&report).unwrap();
    assert_eq!(
        value["response_sha256"],
        librarian_llm_engine::ollama::client::hash(original)
    );
    assert_eq!(value["report"]["accepted"], false);
    assert_eq!(invoke().status.code(), Some(1));
    assert_eq!(fs::read(&report_path).unwrap(), report);
    assert_eq!(fs::read(&response_path).unwrap(), original);
    fs::remove_dir_all(&dir).unwrap();
}

fn context() -> Context {
    serde_json::from_value(json!({"schema_version":1,"policy":{"allow_inference":false,"allow_hypothesis":false},"evidence":[{"id":"E1","path":"sample.rs","start_line":5,"end_line":6,"excerpt":"let x = 1;\r\nuse_value(x);\r\n"}]})).unwrap()
}
fn response() -> Value {
    json!({"schema_version":1,"claims":[{"id":"C1","kind":"FACT","text":"The source assigns 1 to x.","citations":[{"evidence_id":"E1","path":"sample.rs","start_line":5,"end_line":5,"quote":"let x = 1;"}],"premises":[],"verification_plan":null}]})
}
fn codes(context: &Context, value: &Value) -> Vec<String> {
    claims::check(context, &value.to_string())
        .unwrap()
        .issues
        .into_iter()
        .map(|i| i.code)
        .collect()
}

#[test]
fn exact_citation_passes_without_semantic_approval() {
    let report = claims::check(&context(), &response().to_string()).unwrap();
    assert_eq!(report.mechanical_status, "passed");
    assert_eq!(report.semantic_status, "pending");
    assert!(!report.accepted);
}

#[test]
fn rejects_wrong_identity_path_range_and_quote() {
    for (field, value, expected) in [
        ("evidence_id", json!("E2"), "unknown_evidence"),
        ("path", json!("other.rs"), "wrong_path"),
        ("start_line", json!(0), "citation_out_of_range"),
        ("end_line", json!(usize::MAX), "citation_out_of_range"),
        ("quote", json!("let x = 2;"), "quote_mismatch"),
    ] {
        let mut r = response();
        r["claims"][0]["citations"][0][field] = value;
        assert!(codes(&context(), &r).contains(&expected.to_owned()));
    }
}

#[test]
fn permissions_are_independent_and_require_supporting_fields() {
    let mut r = response();
    r["claims"][0]["kind"] = json!("INFERENCE");
    assert!(codes(&context(), &r).contains(&"inference_not_allowed".into()));
    let mut c = context();
    c.policy.allow_inference = true;
    assert_eq!(codes(&c, &r), vec!["missing_premises"]);
    r["claims"][0]["premises"] = json!(["Explicit premise"]);
    assert!(codes(&c, &r).is_empty());
    r["claims"][0]["kind"] = json!("HYPOTHESIS");
    r["claims"][0]["premises"] = json!([]);
    assert!(codes(&c, &r).contains(&"hypothesis_not_allowed".into()));
    c.policy.allow_hypothesis = true;
    assert_eq!(codes(&c, &r), vec!["missing_verification_plan"]);
    r["claims"][0]["verification_plan"] = json!("Inspect another selected source.");
    assert!(codes(&c, &r).is_empty());
}

#[test]
fn lacuna_only_is_valid_but_fact_requires_citation() {
    let mut r = response();
    r["claims"][0]["citations"] = json!([]);
    assert_eq!(codes(&context(), &r), vec!["missing_citation"]);
    r["claims"][0]["kind"] = json!("LACUNA");
    assert!(codes(&context(), &r).is_empty());
}

#[test]
fn rejects_prose_extra_fields_empty_and_duplicate_claims() {
    for text in ["FACT: plausible text", "```json\n{}\n```", "{}"] {
        assert_eq!(
            claims::check(&context(), text).unwrap().mechanical_status,
            "rejected"
        );
    }
    let mut r = response();
    r["conclusion"] = json!("An uncited addition");
    assert_eq!(codes(&context(), &r), vec!["invalid_response_schema"]);
    let mut r = response();
    let duplicate = r["claims"][0].clone();
    r["claims"].as_array_mut().unwrap().push(duplicate);
    assert!(codes(&context(), &r).contains(&"invalid_or_duplicate_claim".into()));
    r["claims"] = json!([]);
    assert_eq!(
        codes(&context(), &r),
        vec!["invalid_version_or_empty_claims"]
    );
}

#[test]
fn rejects_invalid_context_and_preserves_query_selection() {
    let mut c = context();
    c.evidence[0].end_line = 8;
    assert!(claims::check(&c, "{}").is_err());
    assert!(claims::query(&c, "Where?").is_err());
    let c = context();
    let query = claims::query(&c, "Where?").unwrap();
    let q: Value = serde_json::from_str(&query).unwrap();
    assert_eq!(q["evidence"]["items"][0]["excerpt"], c.evidence[0].excerpt);
    assert_eq!(q["policy"]["allow_inference"], false);
    assert_eq!(
        q["evidence"]["line_map"][0]["lines"],
        json!([
            {"number":5,"text":"let x = 1;"},
            {"number":6,"text":"use_value(x);"}
        ])
    );
    assert_eq!(
        librarian_llm_engine::ollama::request(&query, "any-model").unwrap()["prompt"],
        query
    );
}

#[test]
fn historical_camera_failures_remain_semantically_unapproved_even_with_valid_citations() {
    let camera: Context =
        serde_json::from_str(include_str!("fixtures/camera-context.json")).unwrap();
    for original in [
        include_str!("fixtures/camera-legacy-01.txt"),
        include_str!("fixtures/camera-legacy-02.txt"),
    ] {
        assert_eq!(
            claims::check(&camera, original).unwrap().mechanical_status,
            "rejected"
        );
    }
    // These assertions model the documented failures, not a semantic classifier.
    for unsupported in [
        "Converte para coordenadas no mundo.",
        "Obtém a origem no espaço da câmera.",
    ] {
        let mut r = response();
        r["claims"][0]["citations"] = json!([{"evidence_id":"E1","path":"src/camera.rs","start_line":86,"end_line":86,"quote":"        let direction = (pixel - origin).normalize();"}]);
        r["claims"][0]["text"] = json!(unsupported);
        let report = claims::check(&camera, &r.to_string()).unwrap();
        assert_eq!(report.mechanical_status, "passed");
        assert_eq!(report.semantic_status, "pending");
        assert!(!report.accepted);
    }
}
