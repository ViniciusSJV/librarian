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

#[test]
fn limits_and_project_instructions_are_added_only_when_present() {
    let plain: Value = serde_json::from_str(&claims::query(&context(), "Where?").unwrap()).unwrap();
    assert!(plain["evidence"].get("limits").is_none());
    assert!(plain.get("project_instructions").is_none());
    let mut c = context();
    c.limits = vec!["Edition com 2 diagnósticos.".into()];
    c.project_instructions = vec!["Cite o arquivo.".into()];
    let q: Value = serde_json::from_str(&claims::query(&c, "Where?").unwrap()).unwrap();
    assert_eq!(q["evidence"]["limits"], json!(["Edition com 2 diagnósticos."]));
    assert_eq!(q["project_instructions"], json!(["Cite o arquivo."]));
    // The fixed contract keeps its order; the notes about the extra parts come after it.
    let fixed = plain["instructions"].as_array().unwrap();
    let extended = q["instructions"].as_array().unwrap();
    assert_eq!(&extended[..fixed.len()], &fixed[..]);
    assert_eq!(extended.len(), fixed.len() + 2);
    // A context round-trips with its new fields and stays valid.
    let back: Context = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(back.project_instructions, c.project_instructions);
}

#[test]
fn only_a_single_plain_or_json_fence_is_extracted() {
    assert_eq!(claims::extract_fenced("```json\n{\"a\":1}\n```"), Some("{\"a\":1}\n"));
    assert_eq!(claims::extract_fenced("  ```\n{}\n```\n"), Some("{}\n"));
    for refused in [
        "{}",
        "Aqui está:\n```json\n{}\n```",
        "```json\n{}\n```\nPronto.",
        "```rust\n{}\n```",
        "```json\n{}\n```\n```json\n{}\n```",
        "```json {}```",
    ] {
        assert_eq!(claims::extract_fenced(refused), None, "{refused:?}");
    }
    // check stays strict: a fenced answer is rejected unless the caller extracts it first.
    let fenced = format!("```json\n{}\n```", response());
    assert_eq!(claims::check(&context(), &fenced).unwrap().mechanical_status, "rejected");
    let body = claims::extract_fenced(&fenced).unwrap();
    assert_eq!(claims::check(&context(), body).unwrap().mechanical_status, "passed");
}

#[test]
fn strict_text_rejects_lines_and_code_the_claim_does_not_cite() {
    // A real Sonnet answer: line 88 and `Ray::new(origin, direction)` are stated, never cited.
    let strict: Context = serde_json::from_str(include_str!("fixtures/camera-studio-context.json")).unwrap();
    let sonnet = include_str!("fixtures/camera-studio-sonnet.txt");
    let report = claims::check(&strict, sonnet).unwrap();
    let issues: Vec<_> = report.issues.iter().map(|i| i.code.as_str()).collect();
    assert_eq!(issues, ["text_cites_uncited_line", "text_quotes_uncited_code"]);
    // Without strict_text the same answer passes, as it did in the studio.
    let mut lenient: Context = serde_json::from_str(include_str!("fixtures/camera-studio-context.json")).unwrap();
    lenient.strict_text = false;
    assert_eq!(claims::check(&lenient, sonnet).unwrap().mechanical_status, "passed");

    // Citing line 88 fixes both: the line is covered and the call appears in the cited lines.
    let mut fixed: Value = serde_json::from_str(sonnet).unwrap();
    fixed["claims"][0]["citations"].as_array_mut().unwrap().push(json!({
        "evidence_id": "E1", "path": "src/camera.rs", "start_line": 88, "end_line": 88,
        "quote": "        Ray::new(origin, direction)"}));
    assert_eq!(claims::check(&strict, &fixed.to_string()).unwrap().mechanical_status, "passed");

    // The cited source decides, not the model's quote: a wrong quote cannot vouch for the text.
    let mut r = response();
    r["claims"][0]["text"] = json!("A linha 5 faz `let x = 2;`.");
    r["claims"][0]["citations"][0]["quote"] = json!("let x = 2;");
    let mut c = context();
    c.strict_text = true;
    assert_eq!(codes(&c, &r), ["text_quotes_uncited_code", "quote_mismatch"]);
    // The query announces the rule only when it applies.
    let q: Value = serde_json::from_str(&claims::query(&c, "Where?").unwrap()).unwrap();
    assert_eq!(q["strict_text"], true);
    assert!(q["instructions"].as_array().unwrap().last().unwrap().as_str().unwrap().starts_with("Com strict_text=true"));
    assert!(serde_json::from_str::<Value>(&claims::query(&context(), "Where?").unwrap()).unwrap().get("strict_text").is_none());
}

#[test]
fn precise_location_refuses_the_whole_function_as_the_operation() {
    // The sphere case: the answer named the function and called all of it the operation.
    let (mut c, mut r) = located();
    r["claims"][0]["location"]["operation_start_line"] = json!(10);
    r["claims"][0]["location"]["operation_end_line"] = json!(12);
    r["claims"][0]["citations"] = json!([{"evidence_id":"S","path":"other.lang","start_line":10,"end_line":12,
        "quote":"function first() {\n  act();\n}"}]);
    assert!(codes(&c, &r).is_empty(), "without the rule the whole function passes");
    c.precise_location = true;
    assert_eq!(codes(&c, &r), ["location_operation_includes_declaration"]);
    // Dropping only the declaration line is still the whole function (the real qwen retry).
    r["claims"][0]["location"]["operation_start_line"] = json!(11);
    assert_eq!(codes(&c, &r), ["location_operation_is_whole_body"]);
    // Pointing at the body passes.
    let (mut c, r) = located();
    c.precise_location = true;
    assert!(codes(&c, &r).is_empty());
    // A one-line symbol has no body apart from its declaration line.
    let one_line: Context = serde_json::from_value(json!({
        "schema_version":1,"require_location":true,"precise_location":true,
        "policy":{"allow_inference":false,"allow_hypothesis":false},
        "evidence":[{"id":"S","path":"o.lang","start_line":1,"end_line":1,"excerpt":"function f() { return 1; }",
            "symbols":[{"name":"f","declaration_line":1,"name_column":10,"end_line":1}]}]})).unwrap();
    let answer = json!({"schema_version":1,"claims":[{"id":"C1","kind":"FACT","text":"f returns 1.","premises":[],"verification_plan":null,
        "location":{"evidence_id":"S","path":"o.lang","symbol_name":"f","declaration_line":1,"operation_start_line":1,"operation_end_line":1},
        "citations":[{"evidence_id":"S","path":"o.lang","start_line":1,"end_line":1,"quote":"function f() { return 1; }"}]}]});
    assert!(codes(&one_line, &answer).is_empty());
    // A one-line body (Python `def f():` then `return 1`) is the only possible operation.
    let short: Context = serde_json::from_value(json!({
        "schema_version":1,"require_location":true,"precise_location":true,
        "policy":{"allow_inference":false,"allow_hypothesis":false},
        "evidence":[{"id":"S","path":"f.py","start_line":1,"end_line":2,"excerpt":"def f():\n    return 1",
            "symbols":[{"name":"f","declaration_line":1,"name_column":5,"end_line":2}]}]})).unwrap();
    let answer = json!({"schema_version":1,"claims":[{"id":"C1","kind":"FACT","text":"f returns 1.","premises":[],"verification_plan":null,
        "location":{"evidence_id":"S","path":"f.py","symbol_name":"f","declaration_line":1,"operation_start_line":2,"operation_end_line":2},
        "citations":[{"evidence_id":"S","path":"f.py","start_line":1,"end_line":2,"quote":"def f():\n    return 1"}]}]});
    assert!(codes(&short, &answer).is_empty());
    // The query announces the rule only when it applies.
    let q: Value = serde_json::from_str(&claims::query(&one_line, "Where?").unwrap()).unwrap();
    assert_eq!(q["precise_location"], true);
    assert!(q["instructions"].as_array().unwrap().iter().any(|i| i.as_str().unwrap().starts_with("Com precise_location=true")));
    assert!(serde_json::from_str::<Value>(&claims::query(&located().0, "Where?").unwrap()).unwrap().get("precise_location").is_none());
}

fn patching() -> (Context, Value) {
    let c: Context = serde_json::from_value(json!({
        "schema_version": 1, "allow_patch": true,
        "policy": {"allow_inference": false, "allow_hypothesis": true},
        "evidence": [{"id": "E1", "path": "src/a.rs", "start_line": 10, "end_line": 13,
            "excerpt": "fn total(xs: &[u64]) -> u64 {\n    let v: Vec<u64> = xs.to_vec();\n    v.iter().sum()\n}"}]
    }))
    .unwrap();
    let r = json!({"schema_version": 1, "claims": [{
        "id": "H1", "kind": "HYPOTHESIS", "text": "Somar direto em xs evita a cópia de to_vec.",
        "citations": [{"evidence_id": "E1", "path": "src/a.rs", "start_line": 11, "end_line": 12,
            "quote": "    let v: Vec<u64> = xs.to_vec();\n    v.iter().sum()"}],
        "premises": [], "verification_plan": "cargo bench --bench total, antes e depois",
        "patch": "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -10,4 +10,3 @@\n fn total(xs: &[u64]) -> u64 {\n-    let v: Vec<u64> = xs.to_vec();\n-    v.iter().sum()\n+    xs.iter().sum()\n }\n"}]});
    (c, r)
}

#[test]
fn a_patch_is_checked_line_by_line_against_the_evidence_and_never_approved() {
    let (c, r) = patching();
    let report = claims::check(&c, &r.to_string()).unwrap();
    assert_eq!((report.mechanical_status, report.accepted, report.semantic_status), ("passed", false, "pending"));
    let edit = |f: &dyn Fn(&mut Value)| {
        let mut v = r.clone();
        f(&mut v);
        codes(&c, &v)
    };
    let patch = |v: &mut Value, from: &str, to: &str| {
        let p = v["claims"][0]["patch"].as_str().unwrap().replace(from, to);
        v["claims"][0]["patch"] = json!(p);
    };
    // An old line that is not the evidence text, a hunk beyond the evidence, another file,
    // counts that disagree with the header and text that is not a diff.
    assert_eq!(edit(&|v| patch(v, "-    v.iter().sum()", "-    v.iter().sum::<u64>()")), ["patch_does_not_apply"]);
    assert_eq!(edit(&|v| patch(v, "@@ -10,4 +10,3 @@", "@@ -11,4 +11,3 @@")), ["patch_outside_evidence"]);
    assert_eq!(edit(&|v| patch(v, "src/a.rs", "src/b.rs")), ["patch_unknown_path"]);
    assert_eq!(edit(&|v| patch(v, "@@ -10,4 +10,3 @@", "@@ -10,5 +10,3 @@")), ["patch_malformed"]);
    assert_eq!(edit(&|v| v["claims"][0]["patch"] = json!("troque to_vec por iter")), ["patch_malformed"]);
    // Removed lines must be cited by the same claim.
    assert_eq!(edit(&|v| {
        v["claims"][0]["citations"][0]["end_line"] = json!(11);
        v["claims"][0]["citations"][0]["quote"] = json!("    let v: Vec<u64> = xs.to_vec();");
    }), ["patch_changes_uncited_lines"]);
    // Only a HYPOTHESIS proposes a change, and only when the context allows it.
    assert_eq!(edit(&|v| { v["claims"][0]["kind"] = json!("FACT"); v["claims"][0]["verification_plan"] = Value::Null; }), ["patch_requires_hypothesis"]);
    let mut closed = patching().0;
    closed.allow_patch = false;
    assert_eq!(codes(&closed, &r), ["patch_not_allowed"]);
    // The query announces the field only when allowed; otherwise it stays as before.
    let open: Value = serde_json::from_str(&claims::query(&c, "Como evitar a cópia?").unwrap()).unwrap();
    assert_eq!(open["allow_patch"], true);
    assert_eq!(open["response_schema"]["properties"]["claims"]["items"]["properties"]["patch"]["type"], json!(["string", "null"]));
    let plain: Value = serde_json::from_str(&claims::query(&closed, "Como evitar a cópia?").unwrap()).unwrap();
    assert!(plain.get("allow_patch").is_none());
    assert!(plain["response_schema"]["properties"]["claims"]["items"]["properties"].get("patch").is_none());
}

/// A field cached in a struct: one insertion in the struct, one replacement in the method.
fn editing() -> (Context, Value) {
    let c: Context = serde_json::from_value(json!({
        "schema_version": 1, "allow_edits": true,
        "policy": {"allow_inference": false, "allow_hypothesis": true},
        "evidence": [
            {"id": "E1", "path": "src/a.rs", "start_line": 1, "end_line": 3,
                "excerpt": "pub struct Cam {\n    pub m: f64,\n}"},
            {"id": "E2", "path": "src/a.rs", "start_line": 10, "end_line": 12,
                "excerpt": "    fn ray(&self) -> f64 {\n        1.0 / self.m\n    }"}]
    }))
    .unwrap();
    let r = json!({"schema_version": 1, "claims": [{
        "id": "H1", "kind": "HYPOTHESIS", "text": "Guardar o inverso num campo evita a divisão a cada chamada.",
        "citations": [{"evidence_id": "E2", "path": "src/a.rs", "start_line": 11, "end_line": 11, "quote": "        1.0 / self.m"}],
        "premises": [], "verification_plan": "Medir ray antes e depois e comparar os resultados.",
        "edits": [
            {"path": "src/a.rs", "op": "replace", "start_line": 11, "end_line": 11, "new_lines": ["        self.inv"]},
            {"path": "src/a.rs", "op": "insert_after", "start_line": 2, "end_line": 2, "new_lines": ["    pub inv: f64,"]}]}]});
    (c, r)
}

#[test]
fn edits_are_checked_against_the_evidence_and_derive_a_diff() {
    let (c, r) = editing();
    let report = claims::check(&c, &r.to_string()).unwrap();
    assert_eq!((report.mechanical_status, report.accepted), ("passed", false));
    // The diff follows file order, shifting new line numbers by the edits above.
    let response: claims::Response = serde_json::from_value(r.clone()).unwrap();
    let diff = claims::edits_diff(&c, &response.claims[0]).unwrap();
    // Hunks carry up to three context lines on both sides, within the evidence block, as
    // `git apply` expects.
    assert_eq!(diff, "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,3 +1,4 @@\n pub struct Cam {\n     pub m: f64,\n+    pub inv: f64,\n }\n@@ -10,3 +11,3 @@\n     fn ray(&self) -> f64 {\n-        1.0 / self.m\n+        self.inv\n     }\n");
    // The derived diff is one the patch checker accepts against the same evidence.
    let mut as_patch = r.clone();
    as_patch["claims"][0].as_object_mut().unwrap().remove("edits");
    as_patch["claims"][0]["patch"] = json!(diff);
    as_patch["claims"][0]["citations"].as_array_mut().unwrap().push(json!(
        {"evidence_id": "E1", "path": "src/a.rs", "start_line": 2, "end_line": 2, "quote": "    pub m: f64,"}));
    let mut patching = editing().0;
    patching.allow_patch = true;
    assert_eq!(codes(&patching, &as_patch), Vec::<String>::new());

    let edit = |f: &dyn Fn(&mut Value)| {
        let mut v = r.clone();
        f(&mut v);
        codes(&c, &v)
    };
    let e = |v: &mut Value, i: usize, field: &str, value: Value| v["claims"][0]["edits"][i][field] = value;
    assert_eq!(edit(&|v| e(v, 0, "op", json!("swap"))), ["edits_malformed"]);
    assert_eq!(edit(&|v| e(v, 1, "end_line", json!(3))), ["edits_malformed"]);
    assert_eq!(edit(&|v| e(v, 0, "new_lines", json!(["a\nb"]))), ["edits_malformed"]);
    assert_eq!(edit(&|v| { e(v, 0, "start_line", json!(5)); e(v, 0, "end_line", json!(5)); }), ["edits_outside_evidence"]);
    assert_eq!(edit(&|v| e(v, 1, "path", json!("src/b.rs"))), ["edits_unknown_path"]);
    // Replaced lines must be cited; a replacement must change something; edits do not overlap.
    assert_eq!(edit(&|v| { e(v, 0, "start_line", json!(10)); }), ["edits_change_uncited_lines", "edits_unbalanced"]);
    // Replacing the line that closes a block with a field, a slip a local model made: the brace
    // goes missing and the edit is refused even when the line is cited.
    assert_eq!(edit(&|v| {
        e(v, 0, "start_line", json!(12)); e(v, 0, "end_line", json!(12)); e(v, 0, "new_lines", json!(["        inv: 1.0,"]));
        v["claims"][0]["citations"][0] = json!({"evidence_id": "E2", "path": "src/a.rs", "start_line": 12, "end_line": 12, "quote": "    }"});
    }), ["edits_unbalanced"]);
    // insert_before puts the lines above the anchor, which stays.
    let mut before = r.clone();
    before["claims"][0]["edits"][1] = json!({"path": "src/a.rs", "op": "insert_before", "start_line": 3, "end_line": 3, "new_lines": ["    pub inv: f64,"]});
    let response: claims::Response = serde_json::from_value(before).unwrap();
    // Before the closing brace is the same place as after the last field: the same diff.
    assert_eq!(claims::edits_diff(&c, &response.claims[0]).unwrap(), diff);
    assert_eq!(edit(&|v| e(v, 0, "new_lines", json!(["        1.0 / self.m"]))), ["edits_no_change"]);
    assert_eq!(edit(&|v| { e(v, 1, "start_line", json!(11)); e(v, 1, "end_line", json!(11)); }), ["edits_overlap"]);
    // Only a HYPOTHESIS proposes edits, and only when the context allows them.
    assert_eq!(edit(&|v| { v["claims"][0]["kind"] = json!("FACT"); v["claims"][0]["verification_plan"] = Value::Null; }), ["edits_require_hypothesis"]);
    let mut closed = editing().0;
    closed.allow_edits = false;
    assert_eq!(codes(&closed, &r), ["edits_not_allowed"]);
    // The query announces edits, and that they count as the explicit request for a hypothesis.
    let open: Value = serde_json::from_str(&claims::query(&c, "Como evitar a divisão?").unwrap()).unwrap();
    assert_eq!(open["allow_edits"], true);
    assert_eq!(open["response_schema"]["properties"]["claims"]["items"]["properties"]["edits"]["items"]["properties"]["op"]["enum"], json!(["replace", "rewrite", "insert_before", "insert_after"]));
    assert!(open["instructions"].as_array().unwrap().iter().any(|i| i.as_str().unwrap().contains("vale como o pedido explícito de hipótese")));
    let plain: Value = serde_json::from_str(&claims::query(&closed, "Como evitar a divisão?").unwrap()).unwrap();
    assert!(plain.get("allow_edits").is_none());
}

#[test]
fn a_rewrite_restates_a_declaration_and_shows_only_what_changed() {
    let (c, mut r) = editing();
    // The whole method restated, with one line changed; only its first line is cited.
    r["claims"][0]["citations"] = json!([{"evidence_id": "E2", "path": "src/a.rs", "start_line": 10, "end_line": 10, "quote": "    fn ray(&self) -> f64 {"}]);
    r["claims"][0]["edits"] = json!([{"path": "src/a.rs", "op": "rewrite", "start_line": 10, "end_line": 12,
        "new_lines": ["    fn ray(&self) -> f64 {", "        self.inv", "    }"]}]);
    assert_eq!(codes(&c, &r), Vec::<String>::new());
    let response: claims::Response = serde_json::from_value(r.clone()).unwrap();
    assert_eq!(claims::edits_diff(&c, &response.claims[0]).unwrap(),
        "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -10,3 +10,3 @@\n     fn ray(&self) -> f64 {\n-        1.0 / self.m\n+        self.inv\n     }\n");
    // The claim still cites something, as any hypothesis, but not each rewritten line.
    let mut elsewhere = r.clone();
    elsewhere["claims"][0]["citations"] = json!([{"evidence_id": "E2", "path": "src/a.rs", "start_line": 11, "end_line": 11, "quote": "        1.0 / self.m"}]);
    assert_eq!(codes(&c, &elsewhere), Vec::<String>::new());
    // A rewrite that drops the closing brace is refused.
    let mut open = r.clone();
    open["claims"][0]["edits"][0]["new_lines"] = json!(["    fn ray(&self) -> f64 {", "        self.inv"]);
    assert_eq!(codes(&c, &open), ["edits_unbalanced"]);
    // A rewrite that changes nothing proposes nothing.
    let mut same = r.clone();
    same["claims"][0]["edits"][0]["new_lines"] = json!(["    fn ray(&self) -> f64 {", "        1.0 / self.m", "    }"]);
    assert_eq!(codes(&c, &same), ["edits_no_change"]);
}
