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

fn located() -> (Context, Value) {
    let c = serde_json::from_value(json!({
        "schema_version":1,"require_location":true,
        "policy":{"allow_inference":false,"allow_hypothesis":false},
        "evidence":[{"id":"S","path":"other.lang","start_line":10,"end_line":14,
            "excerpt":"function first() {\n  act();\n}\nfunction second() {\n}",
            "symbols":[{"name":"first","declaration_line":10,"name_column":10,"end_line":12},
                       {"name":"second","declaration_line":13,"name_column":10,"end_line":14}]}]
    }))
    .unwrap();
    let r = json!({"schema_version":1,"claims":[{
        "id":"C1","kind":"FACT","text":"first calls act.","premises":[],"verification_plan":null,
        "location":{"evidence_id":"S","path":"other.lang","symbol_name":"first","declaration_line":10,"operation_start_line":11,"operation_end_line":11},
        "citations":[{"evidence_id":"S","path":"other.lang","start_line":10,"end_line":11,"quote":"function first() {\n  act();"}]
    }]});
    (c, r)
}

#[test]
fn location_is_generic_and_still_not_semantic_approval() {
    let (c, r) = located();
    let report = claims::check(&c, &r.to_string()).unwrap();
    assert_eq!(report.mechanical_status, "passed");
    assert!(!report.accepted);
    assert_eq!(report.semantic_status, "pending");
    let query: Value = serde_json::from_str(&claims::query(&c, "Where?").unwrap()).unwrap();
    assert_eq!(query["require_location"], true);
    assert!(
        query["response_schema"]["properties"]["claims"]["items"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("location"))
    );
}

#[test]
fn rejects_missing_or_fabricated_location_and_other_function_body() {
    let (c, r) = located();
    for (field, value, expected) in [
        ("evidence_id", json!("unknown"), "location_unknown_evidence"),
        ("path", json!("wrong"), "location_wrong_path"),
        ("symbol_name", json!("fir"), "location_unknown_symbol"),
        ("declaration_line", json!(13), "location_unknown_symbol"),
        (
            "operation_start_line",
            json!(0),
            "location_operation_out_of_range",
        ),
        (
            "operation_end_line",
            json!(usize::MAX),
            "location_operation_out_of_range",
        ),
        (
            "operation_end_line",
            json!(13),
            "location_operation_out_of_range",
        ),
    ] {
        let mut bad = r.clone();
        bad["claims"][0]["location"][field] = value;
        assert!(codes(&c, &bad).contains(&expected.to_owned()), "{field}");
    }
    for missing in [true, false] {
        let mut bad = r.clone();
        if missing {
            bad["claims"][0].as_object_mut().unwrap().remove("location");
        } else {
            bad["claims"][0]["location"] = Value::Null;
        }
        assert!(codes(&c, &bad).contains(&"missing_location".into()));
    }
}

#[test]
fn location_needs_both_citations_and_literal_quotes() {
    let (c, mut r) = located();
    r["claims"][0]["citations"][0] = json!({"evidence_id":"S","path":"other.lang","start_line":11,"end_line":11,"quote":"  act();"});
    assert_eq!(codes(&c, &r), vec!["location_declaration_not_cited"]);
    r["claims"][0]["citations"][0] = json!({"evidence_id":"S","path":"other.lang","start_line":10,"end_line":10,"quote":"function first() {"});
    assert_eq!(codes(&c, &r), vec!["location_operation_not_cited"]);
    r["claims"][0]["citations"][0]["end_line"] = json!(11);
    assert_eq!(codes(&c, &r), vec!["quote_mismatch"]);
}

#[test]
fn missing_symbols_allow_lacuna_without_fabricating_location() {
    let (mut c, mut r) = located();
    c.evidence[0].symbols.clear();
    assert_eq!(codes(&c, &r), vec!["location_unknown_symbol"]);
    r["claims"][0]["kind"] = json!("LACUNA");
    assert_eq!(codes(&c, &r), vec!["location_requires_fact"]);
    r["claims"][0]["location"] = Value::Null;
    r["claims"][0]["citations"] = json!([]);
    assert!(codes(&c, &r).is_empty()); // Pertinence remains a semantic decision.
}

#[test]
fn symbol_metadata_is_checked_against_source_without_parsing_a_language() {
    for (field, value) in [
        ("name", json!("invented")),
        ("name_column", json!(0)),
        ("name_column", json!(usize::MAX)),
        ("declaration_line", json!(9)),
        ("end_line", json!(15)),
        ("end_line", json!(9)),
    ] {
        let (c, _) = located();
        let mut v = serde_json::to_value(c).unwrap();
        v["evidence"][0]["symbols"][0][field] = value;
        let bad: Context = serde_json::from_value(v).unwrap();
        assert!(bad.validate().is_err(), "{field}");
    }
    let (mut c, _) = located();
    c.evidence[0].excerpt = "é function first() {\n  act();\n}\nfunction second() {\n}".into();
    c.evidence[0].symbols[0].name_column = 12;
    assert!(c.validate().is_ok());
    let mut v = serde_json::to_value(c).unwrap();
    let duplicate = v["evidence"][0]["symbols"][0].clone();
    v["evidence"][0]["symbols"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(serde_json::from_value::<Context>(v)
        .unwrap()
        .validate()
        .is_err());
}

#[test]
fn old_camera_answer_is_now_incomplete_under_location_profile() {
    let mut camera: Context =
        serde_json::from_str(include_str!("fixtures/camera-context.json")).unwrap();
    camera.require_location = true;
    let original =
        include_str!("../../../experiments/claims-camera-20260928-04/attempt-1/response.txt");
    let report = claims::check(&camera, original).unwrap();
    assert!(report.issues.iter().any(|i| i.code == "missing_location"));
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
