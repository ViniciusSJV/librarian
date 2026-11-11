//! Fence: the graph builder and search know records and occurrences, never a language.
//! Language knowledge belongs to `src/extractors/` and its versioned registry profiles.
//! The evidence package's contract is inside this fence as a whole file, so its tests live below,
//! not beside it: a test attribute in that file would be a word the fence forbids.
use librarian_ingest::evidence::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::fmt::Debug;

const AGNOSTIC: [(&str, &str); 6] = [
    ("evidence.rs", include_str!("../src/evidence.rs")),
    ("graph.rs", include_str!("../src/graph.rs")),
    ("graph/v2.rs", include_str!("../src/graph/v2.rs")),
    ("owner.rs", include_str!("../src/owner.rs")),
    ("prepare.rs", include_str!("../src/prepare.rs")),
    ("search.rs", include_str!("../src/search.rs")),
];

// Language names, parser crates and syntax that only one language has.
const FORBIDDEN: [&str; 12] = [
    "rust", "syn::", "proc_macro", "python", "java", "typescript", "macro", "cfg", "#[test]",
    "\"impl\"", "\"struct\"", "\"trait\"",
];

#[test]
fn graph_and_search_do_not_name_languages() {
    for (file, text) in AGNOSTIC {
        let lower = text.to_lowercase();
        for word in FORBIDDEN {
            assert!(!lower.contains(word), "{file} mentions {word:?}");
        }
    }
}

#[test]
fn the_evidence_contract_is_fenced_as_a_whole_and_names_no_project() {
    let text = AGNOSTIC.iter().find(|(file, _)| *file == "evidence.rs").unwrap().1;
    assert_eq!(text, include_str!("../src/evidence.rs"));
    let lower = text.to_lowercase();
    for word in ["renderer", "logistica", "backend", "teapot", "matrix", "camera", "::inverse"] {
        assert!(!lower.contains(word), "evidence.rs mentions {word:?}");
    }
}

fn hash(digit: char) -> Sha256Hex {
    Sha256Hex::try_from(digit.to_string().repeat(64)).unwrap()
}

fn text(digit: char) -> String {
    digit.to_string().repeat(64)
}

/// A package with every kind of object in it, built from the types alone. Its values are made up:
/// nothing here is derived from an edition, which is the job of a later step.
fn package() -> EvidencePackage {
    EvidencePackage {
        kind: PackageKind::EvidencePackage,
        format_version: PackageFormat::V1,
        versions: Versions {
            normalization: Normalization::LinesV1,
            rules: RulesVersion("regras-1".into()),
            builder: BuilderIdentity::Recorded { git_commit: "c0".into(), git_dirty: false },
        },
        edition: EditionIdentity { origin: format!("edition:{}", text('1')), catalog_sha256: hash('2'), extractor: "perfil".into(), git_head: Some("h0".into()), git_dirty: Some(false) },
        route: Route::Gathered,
        origin_kind: Some(OriginKind::Selection),
        representation: Representation::Claims,
        fichas: vec![
            Ficha {
                reference: FichaRef("E1".into()), kind: FichaKind::Symbol, unit: UnitId("CTX_1".into()),
                symbol_id: Some("sym:a".into()), chunk_id: Some("chunk:a".into()), path: "src/a".into(), start_line: 3, end_line: 9,
                role: Some(FichaRole::Target),
                reasons: vec![FichaReason { code: ReasonCode::Chosen, for_target: "a".into(), fields: vec![], lines: vec![] }],
                reason_status: ReasonStatus::VerifiedAgainstSuggestion, declared_by: DeclaredBy::Client,
            },
            Ficha {
                reference: FichaRef("E2".into()), kind: FichaKind::Symbol, unit: UnitId("CTX_1".into()),
                symbol_id: Some("sym:b".into()), chunk_id: Some("chunk:b".into()), path: "src/a".into(), start_line: 5, end_line: 6,
                role: None, reasons: vec![], reason_status: ReasonStatus::NotRecorded, declared_by: DeclaredBy::ServerLacuna,
            },
            Ficha {
                reference: FichaRef("F1".into()), kind: FichaKind::ReviewedFact, unit: UnitId("CTX_1".into()),
                symbol_id: None, chunk_id: None, path: "src/a".into(), start_line: 5, end_line: 5,
                role: None, reasons: vec![], reason_status: ReasonStatus::ServerRecorded, declared_by: DeclaredBy::Server,
            },
        ],
        units: vec![Unit {
            evidence_id: UnitId("CTX_1".into()), source_id: "src:a".into(), source_sha256: hash('3'), path: "src/a".into(),
            window_start_line: 3, window_end_line: 11, start_line: 3, end_line: 9, text_sha256: hash('4'), excerpt_sha256: Some(hash('5')),
        }],
        query: QueryProvenance {
            sha256: hash('6'),
            code: CodeClass { units: vec![UnitId("CTX_1".into())], provenance: CodeProvenance::EditionVerified },
            person_text: PersonText { sha256: hash('7'), declared_by: ClientDeclared::Client, provenance: ClientTextProvenance::UnverifiedClientText },
            server_suffixes: vec![
                ServerSuffix { order: 1, sha256: hash('8'), declared_by: ServerRetryDeclared::ServerRetry, rule: RetryRule::GateRefused, attempt: 1 },
                ServerSuffix { order: 2, sha256: hash('9'), declared_by: ServerRetryDeclared::ServerRetry, rule: RetryRule::MechanicalRefused, attempt: 2 },
            ],
            prepared_question_sha256: hash('a'),
            limits: vec![
                LimitEntry { index: 0, text_sha256: hash('b'), producer: LimitProducer::StepKind, input: LimitInput::Route, rule_version: Some("chain-step-v1".into()), basis: None, class: LimitClass::SystemText, provenance: LimitProvenance::SystemGenerated },
                LimitEntry { index: 1, text_sha256: hash('c'), producer: LimitProducer::Diagnostics, input: LimitInput::Edition, rule_version: None,
                    basis: Some(LimitBasis::Execution { execution: "01-x".into(), execution_sha256: hash('d') }), class: LimitClass::SystemText, provenance: LimitProvenance::SystemGenerated },
                LimitEntry { index: 2, text_sha256: hash('e'), producer: LimitProducer::RequestNotes, input: LimitInput::Client, rule_version: None,
                    basis: Some(LimitBasis::Source { source_id: "src:a".into(), source_sha256: hash('3') }), class: LimitClass::ClientText, provenance: LimitProvenance::UnverifiedClientText },
            ],
            reviewed_facts: vec![ReviewedFactRef {
                fact_id: "F1".into(), record_sha256: Some(hash('f')), statement_sha256: hash('0'), ficha: FichaRef("F1".into()), unit: UnitId("CTX_1".into()),
                citations: vec![CitationCheck { path: "src/a".into(), start_line: 5, end_line: 5, quote_sha256: hash('1'), checked_against: QuoteCheckedAgainst::Edition }],
                provenance: ReviewedFactProvenance::AcervoQuotesChecked,
            }],
            fixed_contract_sha256: hash('2'),
            project_instructions_sha256: Some(hash('3')),
            fields: vec![FieldGroup {
                path: "$.evidence.items[].excerpt".into(), class: QueryFieldClass::Code, origin: FieldOrigin::Edition, producer: FieldProducer::ClaimsContext, rule_version: Some("lines-v1".into()),
                provenance: FieldProvenance::EditionVerified,
                places: vec![FieldPlace(Some(FichaRef("E1".into())), Some(UnitId("CTX_1".into())), Some(hash('4'))), FieldPlace(None, None, None)],
            }],
        },
        measure: MeasureReferences {
            at_question: MeasureAtQuestion::Ok { execution: "02-x".into(), execution_sha256: hash('4') },
            selection_basis: MeasureSelectionBasis::Ok { suggestion: hash('5'), execution: "01-x".into(), execution_sha256: hash('6') },
            measure_diverges: Some(true),
            measure_divergence: None,
        },
        selection: Some(SelectionReference { record_format: SelectionRecordFormat, record_sha256: hash('7'), suggestions: vec![hash('5')], declared_by: ServerDeclared::Server }),
        server_additions: vec![ServerAddition {
            declared_by: ServerLacunaDeclared::ServerLacuna, round: 1, from_turn: "12".into(), from_send: "01-provedor".into(), symbol_id: "sym:b".into(), for_name: "b".into(),
            reason: AdditionReason::LacunaName, decided_by_rule: AdditionRule::LadderResolve,
        }],
        bound: BoundArtifacts { request_sha256: hash('8'), search_sha256: hash('9'), dossier_sha256: hash('a'), bundle_query_sha256: hash('b'), claims_context_sha256: Some(hash('c')), criteria_sha256: Some(hash('d')) },
    }
}

fn parse(value: Value) -> Result<EvidencePackage, serde_json::Error> {
    serde_json::from_value(value)
}

/// Every object in the value, by the path of keys and positions that reach it.
fn object_paths(value: &Value, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match value {
        Value::Object(map) => {
            out.push(path.clone());
            for (key, child) in map {
                path.push(key.clone());
                object_paths(child, path, out);
                path.pop();
            }
        }
        Value::Array(list) => {
            for (index, child) in list.iter().enumerate() {
                path.push(index.to_string());
                object_paths(child, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

fn at<'a>(value: &'a mut Value, path: &[String]) -> &'a mut Value {
    path.iter().fold(value, |current, step| match current {
        Value::Array(list) => &mut list[step.parse::<usize>().unwrap()],
        other => &mut other[step.as_str()],
    })
}

fn keys(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                out.push(key.clone());
                keys(child, out);
            }
        }
        Value::Array(list) => list.iter().for_each(|child| keys(child, out)),
        _ => {}
    }
}

#[test]
fn the_package_serializes_to_the_names_and_values_of_the_contract() {
    let value = serde_json::to_value(package()).unwrap();
    assert_eq!(value["kind"], "evidence_package");
    assert_eq!(value["format_version"], 1);
    assert_eq!(value["versions"], json!({ "normalization": "lines-v1", "rules": "regras-1", "builder": { "status": "recorded", "git_commit": "c0", "git_dirty": false } }));
    assert_eq!(value["edition"], json!({ "origin": format!("edition:{}", text('1')), "catalog_sha256": text('2'), "extractor": "perfil", "git_head": "h0", "git_dirty": false }));
    assert_eq!((&value["route"], &value["origin_kind"], &value["representation"]), (&json!("gathered"), &json!("selection"), &json!("claims")));
    assert_eq!(value["fichas"][0], json!({
        "ref": "E1", "kind": "symbol", "unit": "CTX_1", "symbol_id": "sym:a", "chunk_id": "chunk:a", "path": "src/a", "start_line": 3, "end_line": 9,
        "role": "target", "reasons": [{ "code": "chosen", "for_target": "a", "fields": [], "lines": [] }],
        "reason_status": "verified_against_suggestion", "declared_by": "client",
    }));
    assert_eq!(value["units"][0], json!({
        "evidence_id": "CTX_1", "source_id": "src:a", "source_sha256": text('3'), "path": "src/a",
        "window_start_line": 3, "window_end_line": 11, "start_line": 3, "end_line": 9, "text_sha256": text('4'), "excerpt_sha256": text('5'),
    }));
    assert_eq!(value["query"]["code"], json!({ "units": ["CTX_1"], "provenance": "edition_verified" }));
    assert_eq!(value["query"]["person_text"], json!({ "sha256": text('7'), "declared_by": "client", "provenance": "unverified_client_text" }));
    assert_eq!(value["query"]["server_suffixes"][1], json!({ "order": 2, "sha256": text('9'), "declared_by": "server_retry", "rule": "mechanical_refused", "attempt": 2 }));
    assert_eq!(value["query"]["limits"][0], json!({ "index": 0, "text_sha256": text('b'), "producer": "step_kind", "input": "route", "rule_version": "chain-step-v1", "basis": null, "class": "system_text", "provenance": "system_generated" }));
    assert_eq!(value["query"]["limits"][1]["basis"], json!({ "kind": "execution", "execution": "01-x", "execution_sha256": text('d') }));
    assert_eq!(value["query"]["reviewed_facts"][0]["citations"][0], json!({ "path": "src/a", "start_line": 5, "end_line": 5, "quote_sha256": text('1'), "checked_against": "edition" }));
    assert_eq!(value["query"]["reviewed_facts"][0]["provenance"], "acervo_quotes_checked");
    assert_eq!(value["measure"], json!({
        "at_question": { "read_status": "ok", "execution": "02-x", "execution_sha256": text('4') },
        "selection_basis": { "read_status": "ok", "suggestion": text('5'), "execution": "01-x", "execution_sha256": text('6') },
        "measure_diverges": true,
    }));
    assert_eq!(value["selection"], json!({ "record_format": 2, "record_sha256": text('7'), "suggestions": [text('5')], "declared_by": "server" }));
    assert_eq!(value["server_additions"][0], json!({
        "declared_by": "server_lacuna", "round": 1, "from_turn": "12", "from_send": "01-provedor", "symbol_id": "sym:b", "for_name": "b",
        "reason": "lacuna_name", "decided_by_rule": "ladder::resolve",
    }));
    assert_eq!(value["bound"], json!({ "request_sha256": text('8'), "search_sha256": text('9'), "dossier_sha256": text('a'), "bundle_query_sha256": text('b'), "claims_context_sha256": text('c'), "criteria_sha256": text('d') }));
}

#[test]
fn a_package_comes_back_the_same_from_json() {
    let original = package();
    let bytes = serde_json::to_vec(&original).unwrap();
    let back: EvidencePackage = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(back, original);
    assert_eq!(serde_json::to_vec(&back).unwrap(), bytes);
    // And with every nullable field null, and every list empty.
    let mut small = package();
    small.edition.git_head = None;
    small.edition.git_dirty = None;
    small.origin_kind = None;
    small.selection = None;
    small.server_additions = vec![];
    small.query.server_suffixes = vec![];
    small.query.project_instructions_sha256 = None;
    small.bound.claims_context_sha256 = None;
    small.bound.criteria_sha256 = None;
    small.units[0].excerpt_sha256 = None;
    let value = serde_json::to_value(&small).unwrap();
    assert_eq!(value["selection"], Value::Null);
    assert_eq!(value["edition"]["git_head"], Value::Null);
    assert_eq!(parse(value).unwrap(), small);
}

#[test]
fn every_object_of_the_contract_refuses_a_field_it_does_not_know() {
    let value = serde_json::to_value(package()).unwrap();
    let mut paths = vec![];
    object_paths(&value, &mut vec![], &mut paths);
    assert!(paths.len() > 25, "{}", paths.len());
    for path in paths {
        let mut changed = value.clone();
        at(&mut changed, &path).as_object_mut().unwrap().insert("campo_novo".into(), json!(1));
        assert!(parse(changed).is_err(), "{path:?} accepted an unknown field");
    }
}

#[test]
fn every_field_is_required_so_absent_is_not_mistaken_for_null() {
    let value = serde_json::to_value(package()).unwrap();
    let mut paths = vec![];
    object_paths(&value, &mut vec![], &mut paths);
    for path in paths {
        let names: Vec<String> = { let mut copy = value.clone(); at(&mut copy, &path).as_object().unwrap().keys().cloned().collect() };
        for name in names {
            let mut changed = value.clone();
            at(&mut changed, &path).as_object_mut().unwrap().remove(&name);
            assert!(parse(changed).is_err(), "{path:?} accepted the absence of {name}");
        }
    }
}

#[test]
fn the_versions_of_the_package_and_of_the_selection_record_are_not_interchangeable() {
    let value = serde_json::to_value(package()).unwrap();
    let with = |path: &[&str], new: Value| {
        let mut changed = value.clone();
        let path: Vec<String> = path.iter().map(|s| s.to_string()).collect();
        *at(&mut changed, &path) = new;
        parse(changed)
    };
    assert!(with(&["format_version"], json!(3)).is_err());
    assert!(with(&["format_version"], json!(0)).is_err());
    assert!(with(&["kind"], json!("selection_record")).is_err());
    assert!(with(&["selection", "record_format"], json!(1)).is_err());
    assert!(with(&["versions", "normalization"], json!("lines-v2")).is_err());
    assert_eq!(PACKAGE_FORMAT_VERSION, 2);
    assert_eq!(u32::from(PackageFormat::WRITTEN), PACKAGE_FORMAT_VERSION);
    assert_eq!(SELECTION_RECORD_FORMAT, 2);
}

#[test]
fn a_hash_is_well_formed_or_it_is_refused() {
    assert!(Sha256Hex::try_from(text('a')).is_ok());
    assert_eq!(hash('a').as_str(), text('a'));
    for bad in [text('A'), "a".repeat(63), "a".repeat(65), "g".repeat(64), String::new()] {
        assert!(Sha256Hex::try_from(bad.clone()).is_err(), "{bad}");
    }
    let mut value = serde_json::to_value(package()).unwrap();
    value["query"]["sha256"] = json!("nao-e-um-hash");
    assert!(parse(value).is_err());
}

#[test]
fn the_states_of_a_measure_are_told_apart_and_the_three_concepts_do_not_mix() {
    let reading = |value: Value| serde_json::from_value::<MeasureAtQuestion>(value);
    let basis = |value: Value| serde_json::from_value::<MeasureSelectionBasis>(value);
    let sha = text('a');
    // At the question: ok, none or unreadable; never "not applicable".
    assert_eq!(reading(json!({ "read_status": "none" })).unwrap(), MeasureAtQuestion::NoMeasure {});
    assert_eq!(reading(json!({ "read_status": "unreadable" })).unwrap(), MeasureAtQuestion::Unreadable {});
    assert!(reading(json!({ "read_status": "ok", "execution": "01", "execution_sha256": sha })).is_ok());
    assert!(reading(json!({ "read_status": "not_applicable" })).is_err());
    assert!(reading(json!({ "read_status": "ok" })).is_err(), "ok needs the execution");
    assert!(reading(json!({ "read_status": "none", "execution": "01", "execution_sha256": sha })).is_err(), "none has no execution");
    assert!(reading(json!({ "read_status": "unreadable", "execution": "01", "execution_sha256": sha })).is_err());
    assert_ne!(serde_json::to_value(MeasureAtQuestion::NoMeasure {}).unwrap(), serde_json::to_value(MeasureAtQuestion::Unreadable {}).unwrap());
    // The basis of a selection: its suggestion comes with it, and "not applicable" is its own state.
    assert_eq!(basis(json!({ "read_status": "not_applicable" })).unwrap(), MeasureSelectionBasis::NotApplicable {});
    assert!(basis(json!({ "read_status": "none", "suggestion": sha })).is_ok());
    assert!(basis(json!({ "read_status": "unreadable", "suggestion": sha })).is_ok());
    assert!(basis(json!({ "read_status": "none" })).is_err(), "none of a selection names its suggestion");
    assert!(basis(json!({ "read_status": "not_applicable", "suggestion": sha })).is_err());
    assert!(basis(json!({ "read_status": "ok", "suggestion": sha })).is_err());
    // A selection whose suggestion is not tied to it yet: its own state, with nothing to name.
    assert_eq!(basis(json!({ "read_status": "not_recorded" })).unwrap(), MeasureSelectionBasis::NotRecorded {});
    assert!(basis(json!({ "read_status": "not_recorded", "suggestion": sha })).is_err());
    assert_ne!(serde_json::to_value(MeasureSelectionBasis::NotRecorded {}).unwrap(), serde_json::to_value(MeasureSelectionBasis::NotApplicable {}).unwrap());
    // The latest measure of an edition is not a field: it changes, and is worked out when asked for.
    let mut names = vec![];
    keys(&serde_json::to_value(package()).unwrap()["measure"], &mut names);
    assert!(!names.iter().any(|n| n.contains("latest") || n.contains("recent")), "{names:?}");
    assert!(names.contains(&"at_question".to_owned()) && names.contains(&"selection_basis".to_owned()));
}

#[test]
fn several_fichas_share_a_unit_and_they_describe_different_things() {
    let value = package();
    assert_eq!(value.units.len(), 1);
    assert_eq!(value.fichas.len(), 3);
    assert!(value.fichas.iter().all(|f| f.unit == value.units[0].evidence_id));
    let json_value = serde_json::to_value(&value).unwrap();
    let ficha: Vec<&String> = json_value["fichas"][0].as_object().unwrap().keys().collect();
    let unit: Vec<&String> = json_value["units"][0].as_object().unwrap().keys().collect();
    // A ficha says what was selected and why, with no text; a unit says what is in the query, with no role.
    for name in ["ref", "role", "reasons", "reason_status", "declared_by", "unit"] {
        assert!(ficha.iter().any(|k| *k == name) && !unit.iter().any(|k| *k == name), "{name}");
    }
    for name in ["evidence_id", "source_sha256", "text_sha256", "excerpt_sha256", "window_start_line"] {
        assert!(unit.iter().any(|k| *k == name) && !ficha.iter().any(|k| *k == name), "{name}");
    }
    // A unit has lines, and no range of bytes: the text sent is not a slice of the source's bytes.
    let mut names = vec![];
    keys(&json_value, &mut names);
    assert!(!names.iter().any(|n| n.contains("byte")), "{names:?}");
    // In prose there is no excerpt; a hash is there or it is null, and is never left out.
    let mut prose = json_value["units"][0].clone();
    prose["excerpt_sha256"] = Value::Null;
    assert_eq!(serde_json::from_value::<Unit>(prose).unwrap().excerpt_sha256, None);
}

#[test]
fn who_declared_each_thing_is_kept_apart() {
    let value = serde_json::to_value(package()).unwrap();
    let with = |path: &[&str], new: Value| {
        let mut changed = value.clone();
        let path: Vec<String> = path.iter().map(|s| s.to_string()).collect();
        *at(&mut changed, &path) = new;
        parse(changed)
    };
    // What only the client can have written cannot be recorded as the server's, and the reverse.
    assert!(with(&["query", "person_text", "declared_by"], json!("server")).is_err());
    // The link to a selection record is the server's: it checked the record; a client cannot be its author.
    assert!(with(&["selection", "declared_by"], json!("client")).is_err());
    assert!(with(&["query", "server_suffixes", "0", "declared_by"], json!("client")).is_err());
    assert!(with(&["query", "server_suffixes", "0", "declared_by"], json!("server_lacuna")).is_err());
    assert!(with(&["server_additions", "0", "declared_by"], json!("client")).is_err());
    assert!(with(&["server_additions", "0", "declared_by"], json!("server_retry")).is_err());
    assert!(with(&["query", "person_text", "provenance"], json!("edition_verified")).is_err());
    assert!(with(&["fichas", "0", "declared_by"], json!("server_lacuna")).is_ok());
    assert!(with(&["fichas", "0", "declared_by"], json!("alguem")).is_err());
    // Several additions keep their order and their own origin.
    let suffixes = package().query.server_suffixes;
    assert_eq!(suffixes.iter().map(|s| s.order).collect::<Vec<_>>(), [1, 2]);
    assert_eq!(suffixes.iter().map(|s| s.rule).collect::<Vec<_>>(), [RetryRule::GateRefused, RetryRule::MechanicalRefused]);
    assert_ne!(package().query.person_text.sha256, package().query.prepared_question_sha256);
}

#[test]
fn the_builder_is_known_with_its_state_or_it_is_unknown() {
    let identity = |value: Value| serde_json::from_value::<BuilderIdentity>(value);
    assert_eq!(identity(json!({ "status": "recorded", "git_commit": "c0", "git_dirty": true })).unwrap(), BuilderIdentity::Recorded { git_commit: "c0".into(), git_dirty: true });
    assert_eq!(identity(json!({ "status": "unknown" })).unwrap(), BuilderIdentity::Unknown {});
    assert!(identity(json!({ "status": "recorded" })).is_err(), "recorded needs the commit and its state");
    assert!(identity(json!({ "status": "recorded", "git_commit": "c0" })).is_err());
    assert!(identity(json!({ "status": "unknown", "git_commit": "c0" })).is_err(), "unknown carries no commit");
    assert_eq!(serde_json::to_value(BuilderIdentity::Unknown {}).unwrap(), json!({ "status": "unknown" }));
}

#[test]
fn the_contract_has_no_field_of_cause_quality_time_or_human_action() {
    let mut names = vec![];
    keys(&serde_json::to_value(package()).unwrap(), &mut names);
    names.sort();
    names.dedup();
    for word in ["cause", "causa", "weight", "peso", "score", "quality", "qualidade", "suffic", "suficien", "confidence", "relevan", "created", "timestamp", "stamp", "unix", "clicked", "viewed_by", "seen_by", "approved", "person_decision", "status_refused"] {
        assert!(!names.iter().any(|n| n.contains(word)), "{word} in {names:?}");
    }
    // A package exists for a question that reached the preparation: it has no refused state.
    let mut value = serde_json::to_value(package()).unwrap();
    value.as_object_mut().unwrap().insert("status".into(), json!("refused"));
    assert!(parse(value).is_err());
}

// The names of the contract, written out by hand. A round trip cannot defend them: a name changed
// on the way out changes on the way in too, so these compare with literals, in both directions.

/// The value is exactly this JSON, and this JSON is exactly the value.
fn pinned<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: T, literal: Value) {
    assert_eq!(serde_json::to_value(&value).unwrap(), literal);
    assert_eq!(serde_json::from_value::<T>(literal).unwrap(), value);
}

/// `package()` as JSON, literally: every name of every object it holds.
fn package_literal() -> Value {
    json!({
        "kind": "evidence_package",
        "format_version": 1,
        "versions": { "normalization": "lines-v1", "rules": "regras-1", "builder": { "status": "recorded", "git_commit": "c0", "git_dirty": false } },
        "edition": { "origin": format!("edition:{}", text('1')), "catalog_sha256": text('2'), "extractor": "perfil", "git_head": "h0", "git_dirty": false },
        "route": "gathered",
        "origin_kind": "selection",
        "representation": "claims",
        "fichas": [
            { "ref": "E1", "kind": "symbol", "unit": "CTX_1", "symbol_id": "sym:a", "chunk_id": "chunk:a", "path": "src/a", "start_line": 3, "end_line": 9, "role": "target",
              "reasons": [{ "code": "chosen", "for_target": "a", "fields": [], "lines": [] }], "reason_status": "verified_against_suggestion", "declared_by": "client" },
            { "ref": "E2", "kind": "symbol", "unit": "CTX_1", "symbol_id": "sym:b", "chunk_id": "chunk:b", "path": "src/a", "start_line": 5, "end_line": 6, "role": null,
              "reasons": [], "reason_status": "not_recorded", "declared_by": "server_lacuna" },
            { "ref": "F1", "kind": "reviewed_fact", "unit": "CTX_1", "symbol_id": null, "chunk_id": null, "path": "src/a", "start_line": 5, "end_line": 5, "role": null,
              "reasons": [], "reason_status": "server_recorded", "declared_by": "server" },
        ],
        "units": [{ "evidence_id": "CTX_1", "source_id": "src:a", "source_sha256": text('3'), "path": "src/a", "window_start_line": 3, "window_end_line": 11, "start_line": 3, "end_line": 9, "text_sha256": text('4'), "excerpt_sha256": text('5') }],
        "query": {
            "sha256": text('6'),
            "code": { "units": ["CTX_1"], "provenance": "edition_verified" },
            "person_text": { "sha256": text('7'), "declared_by": "client", "provenance": "unverified_client_text" },
            "server_suffixes": [
                { "order": 1, "sha256": text('8'), "declared_by": "server_retry", "rule": "gate_refused", "attempt": 1 },
                { "order": 2, "sha256": text('9'), "declared_by": "server_retry", "rule": "mechanical_refused", "attempt": 2 },
            ],
            "prepared_question_sha256": text('a'),
            "limits": [
                { "index": 0, "text_sha256": text('b'), "producer": "step_kind", "input": "route", "rule_version": "chain-step-v1", "basis": null, "class": "system_text", "provenance": "system_generated" },
                { "index": 1, "text_sha256": text('c'), "producer": "diagnostics", "input": "edition", "rule_version": null, "basis": { "kind": "execution", "execution": "01-x", "execution_sha256": text('d') }, "class": "system_text", "provenance": "system_generated" },
                { "index": 2, "text_sha256": text('e'), "producer": "request_notes", "input": "client", "rule_version": null, "basis": { "kind": "source", "source_id": "src:a", "source_sha256": text('3') }, "class": "client_text", "provenance": "unverified_client_text" },
            ],
            "reviewed_facts": [{ "fact_id": "F1", "record_sha256": text('f'), "statement_sha256": text('0'), "ficha": "F1", "unit": "CTX_1",
                "citations": [{ "path": "src/a", "start_line": 5, "end_line": 5, "quote_sha256": text('1'), "checked_against": "edition" }], "provenance": "acervo_quotes_checked" }],
            "fixed_contract_sha256": text('2'),
            "project_instructions_sha256": text('3'),
            "fields": [{ "path": "$.evidence.items[].excerpt", "class": "code", "origin": "edition", "producer": "claims_context", "rule_version": "lines-v1", "provenance": "edition_verified", "places": [["E1", "CTX_1", text('4')], [null, null, null]] }],
        },
        "measure": {
            "at_question": { "read_status": "ok", "execution": "02-x", "execution_sha256": text('4') },
            "selection_basis": { "read_status": "ok", "suggestion": text('5'), "execution": "01-x", "execution_sha256": text('6') },
            "measure_diverges": true,
        },
        "selection": { "record_format": 2, "record_sha256": text('7'), "suggestions": [text('5')], "declared_by": "server" },
        "server_additions": [{ "declared_by": "server_lacuna", "round": 1, "from_turn": "12", "from_send": "01-provedor", "symbol_id": "sym:b", "for_name": "b", "reason": "lacuna_name", "decided_by_rule": "ladder::resolve" }],
        "bound": { "request_sha256": text('8'), "search_sha256": text('9'), "dossier_sha256": text('a'), "bundle_query_sha256": text('b'), "claims_context_sha256": text('c'), "criteria_sha256": text('d') },
    })
}

#[test]
fn every_field_name_of_the_contract_is_exactly_the_written_one() {
    pinned(package(), package_literal());
    // The objects a whole package cannot hold at once: the other states of the discriminated ones.
    pinned(BuilderIdentity::Recorded { git_commit: "c0".into(), git_dirty: true }, json!({ "status": "recorded", "git_commit": "c0", "git_dirty": true }));
    pinned(BuilderIdentity::Unknown {}, json!({ "status": "unknown" }));
    pinned(LimitBasis::Execution { execution: "01-x".into(), execution_sha256: hash('a') }, json!({ "kind": "execution", "execution": "01-x", "execution_sha256": text('a') }));
    pinned(LimitBasis::Source { source_id: "src:a".into(), source_sha256: hash('a') }, json!({ "kind": "source", "source_id": "src:a", "source_sha256": text('a') }));
    pinned(MeasureAtQuestion::Ok { execution: "01-x".into(), execution_sha256: hash('a') }, json!({ "read_status": "ok", "execution": "01-x", "execution_sha256": text('a') }));
    pinned(MeasureAtQuestion::NoMeasure {}, json!({ "read_status": "none" }));
    pinned(MeasureAtQuestion::Unreadable {}, json!({ "read_status": "unreadable" }));
    pinned(MeasureSelectionBasis::Ok { suggestion: hash('a'), execution: "01-x".into(), execution_sha256: hash('b') }, json!({ "read_status": "ok", "suggestion": text('a'), "execution": "01-x", "execution_sha256": text('b') }));
    pinned(MeasureSelectionBasis::NoMeasure { suggestion: hash('a') }, json!({ "read_status": "none", "suggestion": text('a') }));
    pinned(MeasureSelectionBasis::Unreadable { suggestion: hash('a') }, json!({ "read_status": "unreadable", "suggestion": text('a') }));
    pinned(MeasureSelectionBasis::NotApplicable {}, json!({ "read_status": "not_applicable" }));
    pinned(MeasureSelectionBasis::NotRecorded {}, json!({ "read_status": "not_recorded" }));
}

#[test]
fn every_value_of_every_enum_is_exactly_the_written_one() {
    fn each<T: Serialize + DeserializeOwned + PartialEq + Debug>(values: Vec<(T, &str)>) {
        for (value, name) in values {
            pinned(value, json!(name));
        }
    }
    each(vec![(PackageKind::EvidencePackage, "evidence_package")]);
    each(vec![(Normalization::LinesV1, "lines-v1")]);
    each(vec![(Route::Search, "search"), (Route::Gathered, "gathered"), (Route::Chain, "chain"), (Route::Path, "path")]);
    each(vec![(OriginKind::Selection, "selection"), (OriginKind::Ladder, "ladder"), (OriginKind::Hot, "hot"), (OriginKind::Loop, "loop"), (OriginKind::Operation, "operation")]);
    each(vec![(Representation::Claims, "claims"), (Representation::Prose, "prose")]);
    each(vec![(DeclaredBy::Client, "client"), (DeclaredBy::Server, "server"), (DeclaredBy::ServerLacuna, "server_lacuna")]);
    each(vec![(ClientDeclared::Client, "client")]);
    each(vec![(ServerRetryDeclared::ServerRetry, "server_retry")]);
    each(vec![(ServerLacunaDeclared::ServerLacuna, "server_lacuna")]);
    each(vec![(FichaKind::Symbol, "symbol"), (FichaKind::FileHead, "file_head"), (FichaKind::ReviewedFact, "reviewed_fact")]);
    each(vec![(FichaRole::Target, "target"), (FichaRole::Origin, "origin"), (FichaRole::Cost, "cost"), (FichaRole::Path, "path"), (FichaRole::Support, "support"), (FichaRole::Constraint, "constraint")]);
    each(vec![
        (ReasonCode::Chosen, "chosen"), (ReasonCode::OwnerType, "owner_type"), (ReasonCode::FieldWriter, "field_writer"), (ReasonCode::Called, "called"),
        (ReasonCode::CalledMeasured, "called_measured"), (ReasonCode::ReturnType, "return_type"), (ReasonCode::CalledOwner, "called_owner"), (ReasonCode::Caller, "caller"),
        (ReasonCode::TestCaller, "test_caller"), (ReasonCode::TypeReference, "type_reference"), (ReasonCode::TraitImpl, "trait_impl"), (ReasonCode::Header, "header"),
    ]);
    each(vec![(ReasonStatus::VerifiedAgainstSuggestion, "verified_against_suggestion"), (ReasonStatus::ServerRecorded, "server_recorded"), (ReasonStatus::NotRecorded, "not_recorded")]);
    each(vec![(CodeProvenance::EditionVerified, "edition_verified")]);
    each(vec![(ClientTextProvenance::UnverifiedClientText, "unverified_client_text")]);
    each(vec![(RetryRule::GateRefused, "gate_refused"), (RetryRule::MechanicalRefused, "mechanical_refused")]);
    each(vec![
        (LimitProducer::StepKind, "step_kind"), (LimitProducer::OwnerStep, "owner_step"), (LimitProducer::ProfileSearch, "profile_search"), (LimitProducer::FichaNature, "ficha_nature"),
        (LimitProducer::OwnerContext, "owner_context"), (LimitProducer::Callees, "callees"), (LimitProducer::ReviewedNote, "reviewed_note"), (LimitProducer::HeaderNote, "header_note"),
        (LimitProducer::Diagnostics, "diagnostics"), (LimitProducer::RequestNotes, "request_notes"), (LimitProducer::ExcludeTests, "exclude_tests"),
    ]);
    each(vec![
        (LimitInput::None, "none"), (LimitInput::Route, "route"), (LimitInput::Request, "request"), (LimitInput::EditionProfile, "edition_profile"),
        (LimitInput::Edition, "edition"), (LimitInput::SearchReport, "search_report"), (LimitInput::Client, "client"),
    ]);
    each(vec![(LimitClass::SystemText, "system_text"), (LimitClass::ClientText, "client_text")]);
    each(vec![(LimitProvenance::SystemFixed, "system_fixed"), (LimitProvenance::SystemGenerated, "system_generated"), (LimitProvenance::UnverifiedClientText, "unverified_client_text"), (LimitProvenance::NotAvailable, "not_available")]);
    each(vec![(QuoteCheckedAgainst::Edition, "edition")]);
    each(vec![(ReviewedFactProvenance::AcervoQuotesChecked, "acervo_quotes_checked"), (ReviewedFactProvenance::RecordNotAvailable, "record_not_available")]);
    each(vec![(AdditionReason::LacunaName, "lacuna_name")]);
    each(vec![(AdditionRule::LadderResolve, "ladder::resolve")]);
    // The versions are numbers, not names, and each artifact has its own.
    pinned(PackageFormat::V1, json!(1));
    pinned(PackageFormat::V2, json!(2));
    pinned(SelectionRecordFormat, json!(2));
}

/// One optional field: absent is refused, null and a good value are accepted and come back as they
/// went, a value of another type is refused.
fn optional_field<T: Serialize + DeserializeOwned>(object: &Value, name: &str, good: Value, wrong: Value) {
    let read = |value: Value| serde_json::from_value::<T>(value);
    let mut absent = object.clone();
    assert!(absent.as_object_mut().unwrap().remove(name).is_some(), "{name} is not in the object");
    assert!(read(absent).is_err(), "{name}: absent was accepted");
    for value in [Value::Null, good] {
        let mut present = object.clone();
        present[name] = value.clone();
        let back = serde_json::to_value(read(present).unwrap_or_else(|e| panic!("{name} = {value}: {e}"))).unwrap();
        assert_eq!(back[name], value, "{name}");
    }
    let mut other = object.clone();
    other[name] = wrong.clone();
    assert!(read(other).is_err(), "{name}: {wrong} was accepted");
}

#[test]
fn each_of_the_fifteen_optional_fields_tells_absent_from_null_and_refuses_another_type() {
    let whole = package_literal();
    let edition = &whole["edition"];
    optional_field::<EditionIdentity>(edition, "git_head", json!("h1"), json!(7));
    optional_field::<EditionIdentity>(edition, "git_dirty", json!(true), json!("sim"));
    let ficha = &whole["fichas"][0];
    optional_field::<Ficha>(ficha, "symbol_id", json!("sym:c"), json!(7));
    optional_field::<Ficha>(ficha, "chunk_id", json!("chunk:c"), json!(false));
    optional_field::<Ficha>(ficha, "role", json!("cost"), json!("importante"));
    optional_field::<Unit>(&whole["units"][0], "excerpt_sha256", json!(text('c')), json!("curto"));
    let limit = &whole["query"]["limits"][0];
    optional_field::<LimitEntry>(limit, "rule_version", json!("v2"), json!(1));
    optional_field::<LimitEntry>(limit, "basis", json!({ "kind": "execution", "execution": "01-x", "execution_sha256": text('a') }), json!({ "kind": "outro" }));
    optional_field::<QueryProvenance>(&whole["query"], "project_instructions_sha256", json!(text('c')), json!("X"));
    optional_field::<ReviewedFactRef>(&whole["query"]["reviewed_facts"][0], "record_sha256", json!(text('c')), json!("curto"));
    let field = &whole["query"]["fields"][0];
    optional_field::<FieldGroup>(field, "rule_version", json!("v2"), json!(1));
    // A place is three values in a fixed order: each may be null, none may be left out or added.
    let place = |v: Value| serde_json::from_value::<FieldPlace>(v);
    assert_eq!(place(json!([null, null, null])).unwrap(), FieldPlace(None, None, None));
    assert_eq!(place(json!(["E2", "CTX_2", text('c')])).unwrap(), FieldPlace(Some(FichaRef("E2".into())), Some(UnitId("CTX_2".into())), Some(hash('c'))));
    for wrong in [json!(["E2", "CTX_2"]), json!(["E2", "CTX_2", text('c'), 1]), json!([1, null, null]), json!([null, null, "curto"]), json!({ "ficha": "E2" })] {
        assert!(place(wrong.clone()).is_err(), "{wrong}");
    }
    let bound = &whole["bound"];
    optional_field::<BoundArtifacts>(bound, "claims_context_sha256", json!(text('c')), json!(1));
    optional_field::<BoundArtifacts>(bound, "criteria_sha256", json!(text('c')), json!([]));
    optional_field::<EvidencePackage>(&whole, "origin_kind", json!("hot"), json!("inventario"));
    optional_field::<EvidencePackage>(&whole, "selection", json!({ "record_format": 2, "record_sha256": text('a'), "suggestions": [], "declared_by": "server" }), json!({ "record_format": 1 }));
}


// `lines-v1`: fixed vectors. Bytes, lines, texts and hashes are literals produced by a separate
// implementation of the specification, not by the code under test.
enum Expected {
    OnlyLines,
    Refused(&'static str),
    Window { end_line: usize, excerpt: Option<&'static str>, excerpt_sha256: Option<&'static str>, line_map: &'static [(usize, &'static str)], text_sha256: &'static str },
}

struct Vector {
    name: &'static str,
    bytes: &'static [u8],
    /// None when the bytes are not UTF-8: there are no lines to speak of.
    lines: Option<&'static [&'static str]>,
    source_sha256: &'static str,
    window: Option<(usize, usize, Representation)>,
    expected: Expected,
}

const VECTORS: &[Vector] = &[
    Vector { name: "arquivo_vazio", bytes: b"", lines: Some(&[]), source_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", window: None, expected: Expected::OnlyLines },
    Vector { name: "lf", bytes: b"a\nb\n", lines: Some(&["a", "b"]), source_sha256: "911169ddaaf146aff539f58c26c489af3b892dff0fe283c1c264c65ae5aa59a2", window: Some((1, 2, Representation::Claims)), expected: Expected::Window { end_line: 2, excerpt: Some("a\nb"), excerpt_sha256: Some("7e18f737311b2dc3b2f269dd78396b0351f14fb66efa879f768cb23181883c78"), line_map: &[(1, "a"), (2, "b")], text_sha256: "7e18f737311b2dc3b2f269dd78396b0351f14fb66efa879f768cb23181883c78" } },
    Vector { name: "crlf", bytes: b"a\r\nb\r\n", lines: Some(&["a", "b"]), source_sha256: "58055bdcc73787eb88c78d36f0b4939e9c5dc1c3ad17e25cc85a6833cf1a0cab", window: Some((1, 2, Representation::Claims)), expected: Expected::Window { end_line: 2, excerpt: Some("a\nb"), excerpt_sha256: Some("7e18f737311b2dc3b2f269dd78396b0351f14fb66efa879f768cb23181883c78"), line_map: &[(1, "a"), (2, "b")], text_sha256: "7e18f737311b2dc3b2f269dd78396b0351f14fb66efa879f768cb23181883c78" } },
    Vector { name: "sem_quebra_final", bytes: b"a\nb", lines: Some(&["a", "b"]), source_sha256: "7e18f737311b2dc3b2f269dd78396b0351f14fb66efa879f768cb23181883c78", window: Some((1, 2, Representation::Claims)), expected: Expected::Window { end_line: 2, excerpt: Some("a\nb"), excerpt_sha256: Some("7e18f737311b2dc3b2f269dd78396b0351f14fb66efa879f768cb23181883c78"), line_map: &[(1, "a"), (2, "b")], text_sha256: "7e18f737311b2dc3b2f269dd78396b0351f14fb66efa879f768cb23181883c78" } },
    Vector { name: "ultima_termina_em_cr", bytes: b"a\nb\r", lines: Some(&["a", "b\r"]), source_sha256: "26cb9af06e7a20999b9b050ffbf2cf5d7d5744497167089bf2319fd60872b902", window: Some((1, 2, Representation::Claims)), expected: Expected::Window { end_line: 2, excerpt: Some("a\nb\r"), excerpt_sha256: Some("26cb9af06e7a20999b9b050ffbf2cf5d7d5744497167089bf2319fd60872b902"), line_map: &[(1, "a"), (2, "b\r")], text_sha256: "26cb9af06e7a20999b9b050ffbf2cf5d7d5744497167089bf2319fd60872b902" } },
    Vector { name: "so_cr_final", bytes: b"a\r", lines: Some(&["a\r"]), source_sha256: "961a57df036f6c4f44ca8a054271c45e823f469bcc439f0255c40974c3e3d131", window: Some((1, 1, Representation::Claims)), expected: Expected::Window { end_line: 1, excerpt: Some("a\r"), excerpt_sha256: Some("961a57df036f6c4f44ca8a054271c45e823f469bcc439f0255c40974c3e3d131"), line_map: &[(1, "a\r")], text_sha256: "961a57df036f6c4f44ca8a054271c45e823f469bcc439f0255c40974c3e3d131" } },
    Vector { name: "cr_solto", bytes: b"a\rb\nc\n", lines: Some(&["a\rb", "c"]), source_sha256: "0252f0b9ed9f1a78a16cc53f77522edcc06346c533d15afabc22afd913c8c7ae", window: Some((1, 2, Representation::Claims)), expected: Expected::Window { end_line: 2, excerpt: Some("a\rb\nc"), excerpt_sha256: Some("82525e553e1563543b84f4f22b8885b51f55908af1b60364e2fcd9147b21ead1"), line_map: &[(1, "a\rb"), (2, "c")], text_sha256: "82525e553e1563543b84f4f22b8885b51f55908af1b60364e2fcd9147b21ead1" } },
    Vector { name: "cr_solto_sem_lf", bytes: b"a\rb", lines: Some(&["a\rb"]), source_sha256: "af9081672dd5ef3247a30c2db5b0dafcc9bcf981a26aefb3c55d210d43fcc14e", window: Some((1, 1, Representation::Claims)), expected: Expected::Window { end_line: 1, excerpt: Some("a\rb"), excerpt_sha256: Some("af9081672dd5ef3247a30c2db5b0dafcc9bcf981a26aefb3c55d210d43fcc14e"), line_map: &[(1, "a\rb")], text_sha256: "af9081672dd5ef3247a30c2db5b0dafcc9bcf981a26aefb3c55d210d43fcc14e" } },
    Vector { name: "cr_cr_lf", bytes: b"x\r\r\ny\n", lines: Some(&["x\r", "y"]), source_sha256: "58db6fe73bfdd39e0835278f99c43bc51e91d2a23129080fb2f2ba5609348c51", window: Some((1, 2, Representation::Claims)), expected: Expected::Window { end_line: 2, excerpt: Some("x\r\ny"), excerpt_sha256: Some("b81d54de3d39c210c9579f8b7f4c0cf68d4394a068c150ae6d2556371675b32d"), line_map: &[(1, "x"), (2, "y")], text_sha256: "9ab9de25768ac172235e119b76362ecddad33878fe9a7792cdddbe47236f9a87" } },
    Vector { name: "cr_cr_lf_ultima", bytes: b"a\r\r\n", lines: Some(&["a\r"]), source_sha256: "9b03de62b82ec74f8a4266c87c17fd01910ad11179392bdf2d2bd9806771ef78", window: Some((1, 1, Representation::Claims)), expected: Expected::Window { end_line: 1, excerpt: Some("a\r"), excerpt_sha256: Some("961a57df036f6c4f44ca8a054271c45e823f469bcc439f0255c40974c3e3d131"), line_map: &[(1, "a\r")], text_sha256: "961a57df036f6c4f44ca8a054271c45e823f469bcc439f0255c40974c3e3d131" } },
    Vector { name: "bom", bytes: b"\xef\xbb\xbfa\nb\n", lines: Some(&["\u{feff}a", "b"]), source_sha256: "1220caf912196af70b1b0625cd99989bbb360bf911a109927854227ddbe76e86", window: Some((1, 2, Representation::Claims)), expected: Expected::Window { end_line: 2, excerpt: Some("\u{feff}a\nb"), excerpt_sha256: Some("312eaf073847a5f4d40be348b83d1eab0da5c9bebbf796905c24ce4eee59a219"), line_map: &[(1, "\u{feff}a"), (2, "b")], text_sha256: "312eaf073847a5f4d40be348b83d1eab0da5c9bebbf796905c24ce4eee59a219" } },
    Vector { name: "fffd_literal", bytes: b"a\xef\xbf\xbdb\n", lines: Some(&["a\u{fffd}b"]), source_sha256: "fbd11afb8989888bd4d2a4eb5021ade171ab8b1e0e0033da75ca967b1b0ec3fc", window: Some((1, 1, Representation::Claims)), expected: Expected::Window { end_line: 1, excerpt: Some("a\u{fffd}b"), excerpt_sha256: Some("05087813392efc16fe8ff448920c6328e53af865df39419436659d9ffda90f7b"), line_map: &[(1, "a\u{fffd}b")], text_sha256: "05087813392efc16fe8ff448920c6328e53af865df39419436659d9ffda90f7b" } },
    Vector { name: "multibyte", bytes: b"a\xc3\xa7\xc3\xa3o\n\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e\n\xf0\x9f\x98\x80 fim\n", lines: Some(&["a\u{e7}\u{e3}o", "\u{65e5}\u{672c}\u{8a9e}", "\u{1f600} fim"]), source_sha256: "54248853d67bade98e591df1027ea7ac1831318358dc9d2098299ef059236fbb", window: Some((1, 3, Representation::Claims)), expected: Expected::Window { end_line: 3, excerpt: Some("a\u{e7}\u{e3}o\n\u{65e5}\u{672c}\u{8a9e}\n\u{1f600} fim"), excerpt_sha256: Some("8ff03874954408c4b73389cf6a7d751e01a5b75eceedb43bd878082123b970e3"), line_map: &[(1, "a\u{e7}\u{e3}o"), (2, "\u{65e5}\u{672c}\u{8a9e}"), (3, "\u{1f600} fim")], text_sha256: "8ff03874954408c4b73389cf6a7d751e01a5b75eceedb43bd878082123b970e3" } },
    Vector { name: "utf8_invalido", bytes: b"a\xff\nb\n", lines: None, source_sha256: "b2cd65a872238fd5c8789a10cb20d92a111a1b867eef7ec15f5b319a11e9cebf", window: Some((1, 2, Representation::Claims)), expected: Expected::Refused("InvalidUtf8") },
    Vector { name: "vazias_finais_claims", bytes: b"a\n\n\n", lines: Some(&["a", "", ""]), source_sha256: "ca8b171a24094e738954a2c932d1f8fd04a5a4261ab9caef6a63e7f6f7a4ae9f", window: Some((1, 3, Representation::Claims)), expected: Expected::Window { end_line: 1, excerpt: Some("a"), excerpt_sha256: Some("ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"), line_map: &[(1, "a")], text_sha256: "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb" } },
    Vector { name: "vazias_finais_prosa", bytes: b"a\n\n\n", lines: Some(&["a", "", ""]), source_sha256: "ca8b171a24094e738954a2c932d1f8fd04a5a4261ab9caef6a63e7f6f7a4ae9f", window: Some((1, 3, Representation::Prose)), expected: Expected::Window { end_line: 3, excerpt: None, excerpt_sha256: None, line_map: &[(1, "a"), (2, ""), (3, "")], text_sha256: "a7da489976d0047490617adb4f7a1f27f7af8b52a5176fd002ffe471863520ab" } },
    Vector { name: "so_espacos", bytes: b"a\n   \n\n", lines: Some(&["a", "   ", ""]), source_sha256: "b3462e6375dad033f74eec14cc8c6c7d674f8b32e6f26c231fde21866a1dd89a", window: Some((1, 3, Representation::Claims)), expected: Expected::Window { end_line: 2, excerpt: Some("a\n   "), excerpt_sha256: Some("5c34dae8fa5414b843390f127ecb86cf5635d78ce3765e658720edf28b13f937"), line_map: &[(1, "a"), (2, "   ")], text_sha256: "5c34dae8fa5414b843390f127ecb86cf5635d78ce3765e658720edf28b13f937" } },
    Vector { name: "so_tabs", bytes: b"a\n\t\t\n\n", lines: Some(&["a", "\t\t", ""]), source_sha256: "e8056ba501a7ad569844314a270d94c4a016635c19be308121d8831b7df0eff0", window: Some((1, 3, Representation::Claims)), expected: Expected::Window { end_line: 2, excerpt: Some("a\n\t\t"), excerpt_sha256: Some("0a9e301742b45354fc453a2429f22327e013745bc80628f334d081958abee608"), line_map: &[(1, "a"), (2, "\t\t")], text_sha256: "0a9e301742b45354fc453a2429f22327e013745bc80628f334d081958abee608" } },
    Vector { name: "vazias_internas", bytes: b"a\n\nb\n\n", lines: Some(&["a", "", "b", ""]), source_sha256: "6b628ed55871130e689633f4dd5980dde47e308e5c352784162796e56b73863c", window: Some((1, 4, Representation::Claims)), expected: Expected::Window { end_line: 3, excerpt: Some("a\n\nb"), excerpt_sha256: Some("38022fd2b8dbc5cb3d2cee74e083edbf59e3d4e13d067ebcb5db633d4cff4d8c"), line_map: &[(1, "a"), (2, ""), (3, "b")], text_sha256: "38022fd2b8dbc5cb3d2cee74e083edbf59e3d4e13d067ebcb5db633d4cff4d8c" } },
    Vector { name: "janela_parcial", bytes: b"l1\nl2\nl3\nl4\nl5\n", lines: Some(&["l1", "l2", "l3", "l4", "l5"]), source_sha256: "7b4d7795f2964691768ffa4bf908374a8c4d01a04196703ce99669740a96c019", window: Some((2, 4, Representation::Claims)), expected: Expected::Window { end_line: 4, excerpt: Some("l2\nl3\nl4"), excerpt_sha256: Some("eb179cccc000cf0b2f032cda9ffd74baa45ba3e3abe1f7ef241c4d9bf37571e7"), line_map: &[(2, "l2"), (3, "l3"), (4, "l4")], text_sha256: "eb179cccc000cf0b2f032cda9ffd74baa45ba3e3abe1f7ef241c4d9bf37571e7" } },
    Vector { name: "numeros_originais", bytes: b"1\r\n2\r\n3\r\n4\r\n5\r\n6\r\n7\r\n8\r\n9\r\n10\r\n", lines: Some(&["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]), source_sha256: "c6011831661eaa30565bf87a2793de08bec53ff0e8f29c4404869c049925066b", window: Some((7, 9, Representation::Claims)), expected: Expected::Window { end_line: 9, excerpt: Some("7\n8\n9"), excerpt_sha256: Some("7bc59c70c09c6c9ab081e038b2eadfcbae09e8926df1bcc2eefc9e7cb13f70a3"), line_map: &[(7, "7"), (8, "8"), (9, "9")], text_sha256: "7bc59c70c09c6c9ab081e038b2eadfcbae09e8926df1bcc2eefc9e7cb13f70a3" } },
    Vector { name: "cr_cr_lf_prosa", bytes: b"x\r\r\ny\n", lines: Some(&["x\r", "y"]), source_sha256: "58db6fe73bfdd39e0835278f99c43bc51e91d2a23129080fb2f2ba5609348c51", window: Some((1, 2, Representation::Prose)), expected: Expected::Window { end_line: 2, excerpt: None, excerpt_sha256: None, line_map: &[(1, "x\r"), (2, "y")], text_sha256: "b81d54de3d39c210c9579f8b7f4c0cf68d4394a068c150ae6d2556371675b32d" } },
    Vector { name: "crlf_prosa", bytes: b"a\r\nb\r\n", lines: Some(&["a", "b"]), source_sha256: "58055bdcc73787eb88c78d36f0b4939e9c5dc1c3ad17e25cc85a6833cf1a0cab", window: Some((1, 2, Representation::Prose)), expected: Expected::Window { end_line: 2, excerpt: None, excerpt_sha256: None, line_map: &[(1, "a"), (2, "b")], text_sha256: "7e18f737311b2dc3b2f269dd78396b0351f14fb66efa879f768cb23181883c78" } },
    Vector { name: "parcial_prosa", bytes: b"l1\nl2\n\nl4\n", lines: Some(&["l1", "l2", "", "l4"]), source_sha256: "2a16a5010b222d06e564ad14eaa2049181170e8f615c172d11a22b334ff1af10", window: Some((2, 3, Representation::Prose)), expected: Expected::Window { end_line: 3, excerpt: None, excerpt_sha256: None, line_map: &[(2, "l2"), (3, "")], text_sha256: "59ad463057c640a69660aeeac8e49bafb8dabf9fda264129265a0a831eba1548" } },
    Vector { name: "so_vazias_prosa", bytes: b"\n\n", lines: Some(&["", ""]), source_sha256: "75a11da44c802486bc6f65640aa48a730f0f684c5c07a42ba3cd1735eb3fb070", window: Some((1, 2, Representation::Prose)), expected: Expected::Window { end_line: 2, excerpt: None, excerpt_sha256: None, line_map: &[(1, ""), (2, "")], text_sha256: "01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b" } },
    Vector { name: "so_vazias_claims", bytes: b"\n\n", lines: Some(&["", ""]), source_sha256: "75a11da44c802486bc6f65640aa48a730f0f684c5c07a42ba3cd1735eb3fb070", window: Some((1, 2, Representation::Claims)), expected: Expected::Refused("EmptyAfterTrim") },
    Vector { name: "fora_do_fim", bytes: b"a\n", lines: Some(&["a"]), source_sha256: "87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7", window: Some((1, 2, Representation::Claims)), expected: Expected::Refused("WindowOutOfRange") },
    Vector { name: "linha_zero", bytes: b"a\n", lines: Some(&["a"]), source_sha256: "87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7", window: Some((0, 1, Representation::Claims)), expected: Expected::Refused("WindowOutOfRange") },
    Vector { name: "fim_antes", bytes: b"a\nb\n", lines: Some(&["a", "b"]), source_sha256: "911169ddaaf146aff539f58c26c489af3b892dff0fe283c1c264c65ae5aa59a2", window: Some((2, 1, Representation::Claims)), expected: Expected::Refused("WindowOutOfRange") },
    Vector { name: "janela_em_vazio", bytes: b"", lines: Some(&[]), source_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", window: Some((1, 1, Representation::Claims)), expected: Expected::Refused("WindowOutOfRange") },
];

#[test]
fn the_vectors_of_lines_v1_are_reproduced_to_the_last_character_and_hash() {
    assert_eq!(VECTORS.len(), 30);
    for v in VECTORS {
        assert_eq!(source_sha256(v.bytes).as_str(), v.source_sha256, "{}: source", v.name);
        match v.lines {
            Some(lines) => assert_eq!(snapshot_lines(v.bytes).unwrap(), lines, "{}: lines", v.name),
            None => assert!(snapshot_lines(v.bytes).is_err(), "{}: lines", v.name),
        }
        let Some((start, end, representation)) = v.window else { continue };
        let result = normalize_window(v.bytes, start, end, representation);
        match &v.expected {
            Expected::OnlyLines => unreachable!("{}", v.name),
            Expected::Refused(kind) => {
                let found = match result.unwrap_err() {
                    NormalizationError::InvalidUtf8 { .. } => "InvalidUtf8",
                    NormalizationError::WindowOutOfRange { .. } => "WindowOutOfRange",
                    NormalizationError::EmptyAfterTrim { .. } => "EmptyAfterTrim",
                };
                assert_eq!(found, *kind, "{}", v.name);
            }
            Expected::Window { end_line, excerpt, excerpt_sha256, line_map, text_sha256 } => {
                let w = result.unwrap_or_else(|e| panic!("{}: {e}", v.name));
                assert_eq!((w.window_start_line, w.window_end_line, w.start_line, w.end_line), (start, end, start, *end_line), "{}: lines of the window", v.name);
                assert_eq!(w.excerpt.as_deref(), *excerpt, "{}: excerpt", v.name);
                assert_eq!(w.excerpt_sha256.as_ref().map(Sha256Hex::as_str), *excerpt_sha256, "{}: excerpt hash", v.name);
                let map: Vec<(usize, &str)> = w.line_map.iter().map(|l| (l.number, l.text.as_str())).collect();
                assert_eq!(map, *line_map, "{}: line map", v.name);
                assert_eq!(w.text_sha256.as_str(), *text_sha256, "{}: text hash", v.name);
            }
        }
    }
}

#[test]
fn invalid_bytes_are_refused_and_say_where_never_replaced() {
    assert_eq!(snapshot_lines(b"a\xFF\nb\n"), Err(NormalizationError::InvalidUtf8 { valid_up_to: 1 }));
    assert_eq!(normalize_window(b"ok\n\xC3\x28\n", 1, 1, Representation::Claims), Err(NormalizationError::InvalidUtf8 { valid_up_to: 3 }));
    assert_eq!(normalize_window(b"ok\n\xC3\x28\n", 1, 1, Representation::Prose), Err(NormalizationError::InvalidUtf8 { valid_up_to: 3 }));
    assert!(NormalizationError::InvalidUtf8 { valid_up_to: 1 }.to_string().contains("not UTF-8"));
}

#[test]
fn a_window_is_never_clamped_to_fit() {
    assert_eq!(normalize_window(b"a\n", 1, 2, Representation::Prose), Err(NormalizationError::WindowOutOfRange { start_line: 1, end_line: 2, lines: 1 }));
    assert_eq!(normalize_window(b"a\n", 0, 1, Representation::Prose), Err(NormalizationError::WindowOutOfRange { start_line: 0, end_line: 1, lines: 1 }));
    assert_eq!(normalize_window(b"a\nb\n", 2, 1, Representation::Claims), Err(NormalizationError::WindowOutOfRange { start_line: 2, end_line: 1, lines: 2 }));
    assert_eq!(normalize_window(b"", 1, 1, Representation::Claims), Err(NormalizationError::WindowOutOfRange { start_line: 1, end_line: 1, lines: 0 }));
    // Only empty lines: claims has nothing left and says so, with no made-up range; prose keeps them.
    assert_eq!(normalize_window(b"\n\n", 1, 2, Representation::Claims), Err(NormalizationError::EmptyAfterTrim { start_line: 1, end_line: 2 }));
    assert_eq!(normalize_window(b"\n\n", 1, 2, Representation::Prose).unwrap().line_map.len(), 2);
}

// The real path of the prose form: what `prepare` writes into the query, over the same bytes.
const REAL_SOURCES: [(&str, &[u8]); 5] = [
    ("src/crlf.rs", b"// topo\r\npub fn a() -> u32 {\r\n    1   \r\n}\r\npub fn b() -> u32 {\r\n\t2\r\n}\r\n\r\npub fn c() -> u32 { 3 }\r\n"),
    ("src/mistura.rs", b"pub fn d() -> u32 {\n    // cr solto\rno meio\n    // dois cr no fim\r\r\n    4\r\n}\n"),
    ("src/semfim.rs", b"pub fn e() -> u32 {\n    5\n}"),
    ("src/bom.rs", b"\xEF\xBB\xBFpub fn g() -> u32 { 7 }\n"),
    ("src/vazias.rs", b"pub fn h() -> u32 { 8 }\n\n\n   \n\n"),
];

#[test]
fn prose_windows_are_the_lines_prepare_writes_into_the_query() {
    let dir = std::env::temp_dir().join(format!("librarian-lines-v1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    for (path, bytes) in REAL_SOURCES {
        std::fs::write(dir.join(path), bytes).unwrap();
    }
    let edition = dir.join("edition");
    librarian_ingest::generate(&dir, &librarian_ingest::Config { project: "lines".into(), inputs: vec!["src".into()] }, &edition).unwrap();
    let catalog = librarian_ingest::Catalog::load(&edition).unwrap();
    let target = |path: &str, name: &str, line: usize| json!({ "path": path, "symbol_name": name, "declaration_line": line });
    let cases = [
        vec![target("src/crlf.rs", "a", 2), target("src/crlf.rs", "b", 5)],
        vec![target("src/crlf.rs", "a", 2), target("src/crlf.rs", "c", 9)],
        vec![target("src/mistura.rs", "d", 1)],
        vec![target("src/semfim.rs", "e", 1)],
        vec![target("src/bom.rs", "g", 1)],
        vec![target("src/vazias.rs", "h", 1)],
        vec![target("src/mistura.rs", "d", 1), target("src/vazias.rs", "h", 1), target("src/bom.rs", "g", 1)],
    ];
    let mut windows = 0;
    for targets in &cases {
        for radius in [0, 2] {
            let request: librarian_ingest::prepare::Request = serde_json::from_value(json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "radius": radius, "numbered_lines": true, "targets": targets, "gathered": true })).unwrap();
            let prepared = librarian_ingest::prepare::prepare(&catalog, &request).unwrap();
            let query: Value = serde_json::from_str(prepared.query.query_json().unwrap()).unwrap();
            let evidence = &query["evidence"];
            // One ficha keeps its lines inline; several share blocks.
            let blocks: Vec<(Value, Value)> = match evidence["contexts"].as_array() {
                Some(blocks) => blocks.iter().map(|b| {
                    let source = evidence["selections"].as_array().unwrap().iter().map(|s| &s["source"]).find(|s| s["id"] == b["source_id"]).unwrap();
                    (source.clone(), b.clone())
                }).collect(),
                None => vec![(evidence["source"].clone(), evidence["source"]["context"].clone())],
            };
            for (source, block) in blocks {
                let path = source["path"].as_str().unwrap();
                let bytes = REAL_SOURCES.iter().find(|(p, _)| *p == path).unwrap().1;
                assert_eq!(source["sha256"].as_str().unwrap(), source_sha256(bytes).as_str(), "{path}");
                let (start, end) = (block["start_line"].as_u64().unwrap() as usize, block["end_line"].as_u64().unwrap() as usize);
                let window = normalize_window(bytes, start, end, Representation::Prose).unwrap();
                let lines: Vec<(u64, String)> = block["lines"].as_array().unwrap().iter().map(|l| (l["number"].as_u64().unwrap(), l["text"].as_str().unwrap().to_owned())).collect();
                let ours: Vec<(u64, String)> = window.line_map.iter().map(|l| (l.number as u64, l.text.clone())).collect();
                assert_eq!(ours, lines, "{path}:{start}-{end}, radius {radius}");
                windows += 1;
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(windows >= 16, "{windows}");
}

// Fichas and units, from real preparations of a small project wherever the case can happen, and from
// a changed query or a changed result where it cannot, because the point is that it gets refused.

const A_RS: &[u8] = b"fn um() {\n    1;\n}\nfn dois() {\n    2;\n}\n\nfn tres() {\n    3;\n}\n\n\n";
const B_RS: &[u8] = b"fn outra() {}\n\n\n";
const C_PY: &[u8] = b"class C:\n    def m(self):\n        return 1\n\n    def n(self):\n        return 2\n";
const D_RS: &[u8] = b"struct Ponto {\n    x: u8,\n}\nimpl Ponto {\n    fn ler(&self) -> u8 {\n        self.x\n    }\n    fn gravar(&mut self) {\n        self.x = 1;\n    }\n}\n";
const H_RS: &[u8] = b"use std::fmt;\n\nfn depois() {}\n";
const R_RS: &[u8] = b"fn com_cr() {\r\r\n    1;\r\n}\r\n";
const K_RS: &[u8] = b"fn chamada_unica() {}\nfn chamadora() {\n    chamada_unica();\n}\n";
const PROJECT: [(&str, &[u8]); 7] = [("src/a.rs", A_RS), ("src/b.rs", B_RS), ("src/c.py", C_PY), ("src/d.rs", D_RS), ("src/h.rs", H_RS), ("src/k.rs", K_RS), ("src/r.rs", R_RS)];

static NEXT_PROJECT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn edition(files: &[(&str, &[u8])]) -> librarian_ingest::Catalog {
    let n = NEXT_PROJECT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("librarian-composicao-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    for (path, bytes) in files {
        std::fs::write(dir.join(path), bytes).unwrap();
    }
    let out = dir.join("edition");
    librarian_ingest::generate(&dir, &librarian_ingest::Config { project: "composicao".into(), inputs: vec!["src".into()] }, &out).unwrap();
    let catalog = librarian_ingest::Catalog::load(&out).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    catalog
}

fn target(path: &str, name: &str, line: usize) -> Value {
    json!({ "path": path, "symbol_name": name, "declaration_line": line })
}

/// A prepared query, what it needs, and nothing that changes after it was made.
struct Case {
    query: String,
    search: Vec<u8>,
    composition: Composition,
    snapshots: Vec<(String, String, String, Vec<u8>)>,
}

impl Case {
    fn prepare(catalog: &librarian_ingest::Catalog, request: Value) -> Case {
        let request: librarian_ingest::prepare::Request = serde_json::from_value(request).unwrap();
        let prepared = librarian_ingest::prepare::prepare(catalog, &request).unwrap();
        let mut snapshots: Vec<(String, String, String, Vec<u8>)> = vec![];
        for seed in &prepared.composition.fichas {
            if !snapshots.iter().any(|s| s.1 == seed.path) {
                let source = catalog.source_by_path(&seed.path).unwrap();
                snapshots.push((source.id.clone(), source.path.clone(), source.sha256.clone(), catalog.source_bytes(&seed.path).unwrap().to_vec()));
            }
        }
        for fact in &prepared.composition.reviewed {
            for quote in &fact.citations {
                if !snapshots.iter().any(|s| s.1 == quote.path) {
                    let source = catalog.source_by_path(&quote.path).unwrap();
                    snapshots.push((source.id.clone(), source.path.clone(), source.sha256.clone(), catalog.source_bytes(&quote.path).unwrap().to_vec()));
                }
            }
        }
        Case { query: prepared.query.query_json().unwrap().to_owned(), search: prepared.search.clone(), composition: prepared.composition, snapshots }
    }
    fn of(catalog: &librarian_ingest::Catalog, targets: Vec<Value>, radius: usize) -> Case {
        Case::prepare(catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "radius": radius, "numbered_lines": true, "targets": targets, "gathered": true }))
    }
    fn inputs(&self) -> Vec<SnapshotInput<'_>> {
        self.snapshots.iter().map(|(source_id, path, sha256, bytes)| SnapshotInput { source_id, path, sha256, bytes }).collect()
    }
    fn query_value(&self) -> Value {
        serde_json::from_str(&self.query).unwrap()
    }
    /// The claims form's items, by the rule its context follows: the query's windows, with their last
    /// empty lines gone, and a window with nothing left is not an item.
    fn claims(&self) -> Vec<(String, String, usize, usize, String)> {
        let query = self.query_value();
        let evidence = &query["evidence"];
        let windows: Vec<(String, String, Value)> = match evidence["contexts"].as_array() {
            Some(blocks) => blocks
                .iter()
                .map(|b| {
                    let source = evidence["selections"].as_array().unwrap().iter().map(|s| &s["source"]).find(|s| s["id"] == b["source_id"]).unwrap();
                    (b["id"].as_str().unwrap().to_owned(), source["path"].as_str().unwrap().to_owned(), b.clone())
                })
                .collect(),
            None => vec![(evidence["fact_id"].as_str().unwrap().to_owned(), evidence["source"]["path"].as_str().unwrap().to_owned(), evidence["source"]["context"].clone())],
        };
        let mut items = vec![];
        for (id, path, window) in windows {
            let mut lines: Vec<String> = window["lines"].as_array().unwrap().iter().map(|l| l.get("text").unwrap_or(l).as_str().unwrap().to_owned()).collect();
            while lines.last().is_some_and(|l| l.is_empty()) {
                lines.pop();
            }
            if !lines.is_empty() {
                let start = window["start_line"].as_u64().unwrap() as usize;
                let end = start + lines.len() - 1;
                items.push((id, path, start, end, lines.join("\n")));
            }
        }
        items
    }
}

fn claims_view(items: &[(String, String, usize, usize, String)]) -> Vec<ClaimsEvidence<'_>> {
    items.iter().map(|(id, path, start_line, end_line, excerpt)| ClaimsEvidence { id, path, start_line: *start_line, end_line: *end_line, excerpt }).collect()
}

fn prose(case: &Case) -> Result<ComposedEvidence, CompositionError> {
    let snapshots = case.inputs();
    compose(&CompositionInput { query_json: &case.query, seeds: &case.composition.fichas, radius: case.composition.radius, form: Form::Prose, snapshots: &snapshots })
}

fn claims(case: &Case) -> Result<ComposedEvidence, CompositionError> {
    let snapshots = case.inputs();
    let items = case.claims();
    let view = claims_view(&items);
    compose(&CompositionInput { query_json: &case.query, seeds: &case.composition.fichas, radius: case.composition.radius, form: Form::Claims(&view), snapshots: &snapshots })
}

fn check_prose(case: &Case, composed: &ComposedEvidence) -> Result<(), CompositionError> {
    let snapshots = case.inputs();
    check(composed, &CompositionInput { query_json: &case.query, seeds: &case.composition.fichas, radius: case.composition.radius, form: Form::Prose, snapshots: &snapshots })
}

fn with_query(case: &mut Case, change: impl FnOnce(&mut Value)) {
    let mut query = case.query_value();
    change(&mut query);
    case.query = serde_json::to_string_pretty(&query).unwrap();
}

fn violation(result: Result<ComposedEvidence, CompositionError>) -> Violation {
    match result {
        Err(CompositionError::Violation(violation)) => violation,
        other => panic!("expected a violation: {other:?}"),
    }
}

fn violation_of(result: Result<(), CompositionError>) -> Violation {
    match result {
        Err(CompositionError::Violation(violation)) => violation,
        other => panic!("expected a violation: {other:?}"),
    }
}

fn links(composed: &ComposedEvidence) -> Vec<(&str, &str)> {
    composed.fichas.iter().map(|f| (f.reference.0.as_str(), f.unit.0.as_str())).collect()
}

fn windows(composed: &ComposedEvidence) -> Vec<(&str, &str, usize, usize)> {
    composed.units.iter().map(|u| (u.evidence_id.0.as_str(), u.path.as_str(), u.window_start_line, u.window_end_line)).collect()
}

fn sha(bytes: &[u8]) -> String {
    librarian_ingest::hash(bytes)
}

#[test]
fn a_single_ficha_is_its_own_unit_named_after_it() {
    let catalog = edition(&PROJECT);
    let case = Case::of(&catalog, vec![target("src/a.rs", "um", 1)], 0);
    let composed = prose(&case).unwrap();
    assert_eq!(windows(&composed), [("E1", "src/a.rs", 1, 3)]);
    assert_eq!(links(&composed), [("E1", "E1")]);
    let (ficha, unit) = (&composed.fichas[0], &composed.units[0]);
    let source = catalog.source_by_path("src/a.rs").unwrap();
    assert_eq!((unit.source_id.as_str(), unit.source_sha256.as_str()), (source.id.as_str(), sha(A_RS).as_str()));
    assert_eq!((ficha.kind, ficha.path.as_str(), ficha.start_line, ficha.end_line, ficha.role), (FichaKind::Symbol, "src/a.rs", 1, 3, None));
    assert!(catalog.symbols()[ficha.symbol_id.as_ref().unwrap()].name == "um");
    assert_eq!(catalog.symbols()[ficha.symbol_id.as_ref().unwrap()].chunk_id, *ficha.chunk_id.as_ref().unwrap());
}

#[test]
fn several_fichas_go_into_one_unit_when_the_query_put_them_there() {
    let catalog = edition(&PROJECT);
    let reviewed = json!({ "id": "F1", "statement": "s", "authorship": "a", "path": "src/a.rs", "line": 5, "end_line": 5, "citations": [{ "path": "src/a.rs", "start_line": 5, "end_line": 5, "quote": "    2;" }] });
    let case = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "targets": [target("src/a.rs", "dois", 4)], "reviewed_facts": [reviewed] }));
    let composed = prose(&case).unwrap();
    assert_eq!(windows(&composed), [("CTX_1", "src/a.rs", 4, 6)]);
    assert_eq!(links(&composed), [("E1", "CTX_1"), ("F1", "CTX_1")]);
    let f = &composed.fichas[1];
    assert_eq!((f.kind, f.start_line, f.end_line, f.symbol_id.clone(), f.chunk_id.clone(), f.declared_by, f.reason_status), (FichaKind::ReviewedFact, 5, 5, None, None, DeclaredBy::Server, ReasonStatus::NotRecorded));
}

#[test]
fn different_sources_are_different_units() {
    let catalog = edition(&PROJECT);
    let case = Case::of(&catalog, vec![target("src/a.rs", "um", 1), target("src/b.rs", "outra", 1)], 0);
    let composed = prose(&case).unwrap();
    assert_eq!(windows(&composed), [("CTX_1", "src/a.rs", 1, 3), ("CTX_2", "src/b.rs", 1, 1)]);
    assert_eq!(links(&composed), [("E1", "CTX_1"), ("E2", "CTX_2")]);
    assert_eq!(composed.units[0].source_sha256.as_str(), sha(A_RS));
    assert_eq!(composed.units[1].source_sha256.as_str(), sha(B_RS));
    assert_ne!(composed.units[0].source_id, composed.units[1].source_id);
}

fn two_units(catalog: &librarian_ingest::Catalog) -> Case {
    Case::of(catalog, vec![target("src/a.rs", "um", 1), target("src/a.rs", "tres", 8)], 0)
}

#[test]
fn a_ficha_the_query_ties_to_no_unit_is_refused() {
    let mut case = two_units(&edition(&PROJECT));
    with_query(&mut case, |q| {
        q["evidence"]["selections"][0]["source"]["context"].as_object_mut().unwrap().remove("context_id");
    });
    assert_eq!(violation(prose(&case)), Violation::FichaWithoutUnit { ficha: "E1".into() });
}

#[test]
fn a_unit_no_ficha_points_to_is_refused() {
    let mut case = two_units(&edition(&PROJECT));
    with_query(&mut case, |q| {
        let mut extra = q["evidence"]["contexts"][0].clone();
        extra["id"] = json!("CTX_9");
        q["evidence"]["contexts"].as_array_mut().unwrap().push(extra);
    });
    assert_eq!(violation(prose(&case)), Violation::UnitWithoutFicha { unit: "CTX_9".into() });
}

#[test]
fn a_ficha_naming_a_unit_that_does_not_exist_is_refused() {
    let mut case = two_units(&edition(&PROJECT));
    with_query(&mut case, |q| {
        q["evidence"]["selections"][1]["source"]["context"]["context_id"] = json!("CTX_9");
    });
    assert_eq!(violation(prose(&case)), Violation::FichaUnitUnknown { ficha: "E2".into(), unit: "CTX_9".into() });
}

#[test]
fn a_ficha_and_its_unit_in_different_sources_are_refused() {
    let catalog = edition(&PROJECT);
    let mut case = Case::of(&catalog, vec![target("src/a.rs", "um", 1), target("src/b.rs", "outra", 1)], 0);
    let composed = prose(&case).unwrap();
    let expected = Violation::FichaOtherSource { ficha: "E1".into(), unit: "CTX_1".into() };
    let mut moved = composed.clone();
    moved.fichas[0].path = "src/b.rs".into();
    assert_eq!(violation_of(check_prose(&case, &moved)), expected);
    // The link says the other source, and only the link: nothing about the lines is wrong.
    let mut linked = composed.clone();
    linked.fichas[0].unit = UnitId("CTX_2".into());
    linked.fichas[0].start_line = 1;
    linked.fichas[0].end_line = 1;
    assert_eq!(violation_of(check_prose(&case, &linked)), Violation::FichaOtherSource { ficha: "E1".into(), unit: "CTX_2".into() });
    // What the preparation knows says another path, and the result does not.
    let original = case.composition.fichas[0].path.clone();
    case.composition.fichas[0].path = "src/b.rs".into();
    assert_eq!(violation_of(check_prose(&case, &composed)), expected);
    case.composition.fichas[0].path = original;
    // Same path, another source id or another edition's hash for it.
    case.composition.fichas[0].source_id = "src:outra".into();
    assert_eq!(violation_of(check_prose(&case, &composed)), expected);
    case.composition.fichas[0].source_id = composed.units[0].source_id.clone();
    case.composition.fichas[0].source_sha256 = text('e');
    assert_eq!(violation_of(check_prose(&case, &composed)), expected);
}

#[test]
fn a_ficha_outside_its_window_is_refused_on_either_side() {
    let catalog = edition(&PROJECT);
    let case = Case::of(&catalog, vec![target("src/a.rs", "tres", 8), target("src/a.rs", "um", 1)], 0);
    let composed = prose(&case).unwrap();
    assert_eq!(windows(&composed), [("CTX_1", "src/a.rs", 1, 3), ("CTX_2", "src/a.rs", 8, 10)]);
    let expected = Violation::FichaOutsideWindow { ficha: "E1".into(), unit: "CTX_2".into() };
    let mut past = composed.clone();
    past.fichas[0].end_line = 11;
    assert_eq!(violation_of(check_prose(&case, &past)), expected);
    let mut before = composed.clone();
    before.fichas[0].start_line = 7;
    assert_eq!(violation_of(check_prose(&case, &before)), expected);
    let mut upside_down = composed.clone();
    upside_down.fichas[0].end_line = 7;
    upside_down.fichas[0].start_line = 8;
    assert_eq!(violation_of(check_prose(&case, &upside_down)), expected);
}

#[test]
fn contiguous_windows_are_one_unit_and_separate_ones_are_not() {
    let catalog = edition(&PROJECT);
    let touching = prose(&Case::of(&catalog, vec![target("src/a.rs", "um", 1), target("src/a.rs", "dois", 4)], 0)).unwrap();
    assert_eq!(windows(&touching), [("CTX_1", "src/a.rs", 1, 6)]);
    assert_eq!(links(&touching), [("E1", "CTX_1"), ("E2", "CTX_1")]);
    // One empty line between them is enough to keep them apart.
    let apart = prose(&Case::of(&catalog, vec![target("src/a.rs", "dois", 4), target("src/a.rs", "tres", 8)], 0)).unwrap();
    assert_eq!(windows(&apart), [("CTX_1", "src/a.rs", 4, 6), ("CTX_2", "src/a.rs", 8, 10)]);
    assert_eq!(links(&apart), [("E1", "CTX_1"), ("E2", "CTX_2")]);
}

#[test]
fn overlapping_windows_are_one_unit() {
    let catalog = edition(&PROJECT);
    let composed = prose(&Case::of(&catalog, vec![target("src/a.rs", "dois", 4), target("src/a.rs", "tres", 8)], 1)).unwrap();
    assert_eq!(windows(&composed), [("CTX_1", "src/a.rs", 3, 11)]);
    assert_eq!(links(&composed), [("E1", "CTX_1"), ("E2", "CTX_1")]);
}

#[test]
fn a_method_inside_its_class_shares_the_unit_of_the_class() {
    let catalog = edition(&PROJECT);
    let composed = prose(&Case::of(&catalog, vec![target("src/c.py", "C", 1), target("src/c.py", "m", 2)], 0)).unwrap();
    assert_eq!(windows(&composed), [("CTX_1", "src/c.py", 1, 6)]);
    assert_eq!(links(&composed), [("E1", "CTX_1"), ("E2", "CTX_1")]);
    assert_eq!((composed.fichas[0].start_line, composed.fichas[0].end_line), (1, 6));
    assert_eq!((composed.fichas[1].start_line, composed.fichas[1].end_line), (2, 3));
}

#[test]
fn radius_zero_is_the_ficha_and_nothing_else() {
    let case = Case::of(&edition(&PROJECT), vec![target("src/a.rs", "dois", 4)], 0);
    assert_eq!(case.composition.radius, 0);
    let composed = prose(&case).unwrap();
    assert_eq!(windows(&composed), [("E1", "src/a.rs", 4, 6)]);
    assert_eq!(composed.units[0].text_sha256.as_str(), sha(b"fn dois() {\n    2;\n}"));
}

#[test]
fn a_radius_widens_up_to_the_ends_of_the_source_and_must_be_the_one_the_query_used() {
    let catalog = edition(&PROJECT);
    let case = Case::of(&catalog, vec![target("src/a.rs", "dois", 4)], 1);
    assert_eq!(case.composition.radius, 1);
    assert_eq!(windows(&prose(&case).unwrap()), [("E1", "src/a.rs", 3, 7)]);
    assert_eq!(case.query_value()["evidence"]["source"]["context"]["requested_radius"], 1);
    // Stops at the first line and at the last.
    assert_eq!(windows(&prose(&Case::of(&catalog, vec![target("src/a.rs", "um", 1)], 2)).unwrap()), [("E1", "src/a.rs", 1, 5)]);
    assert_eq!(windows(&prose(&Case::of(&catalog, vec![target("src/a.rs", "tres", 8)], 50)).unwrap()), [("E1", "src/a.rs", 1, 12)]);
    // Saying another radius is not believed, and not ignored, in either direction.
    for wrong in [0, 2] {
        let snapshots = case.inputs();
        let result = compose(&CompositionInput { query_json: &case.query, seeds: &case.composition.fichas, radius: wrong, form: Form::Prose, snapshots: &snapshots });
        assert_eq!(violation(result), Violation::RadiusDiffers { ficha: "E1".into() });
    }
}

#[test]
fn a_query_with_no_radius_recorded_is_refused_not_taken_as_zero() {
    let mut case = Case::of(&edition(&PROJECT), vec![target("src/a.rs", "dois", 4)], 0);
    with_query(&mut case, |q| {
        q["evidence"]["source"]["context"].as_object_mut().unwrap().remove("requested_radius");
    });
    assert_eq!(violation(prose(&case)), Violation::RadiusDiffers { ficha: "E1".into() });
}

fn block_lines(start: usize, end: usize) -> Value {
    let text = std::str::from_utf8(A_RS).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    json!((start..=end).map(|n| json!({ "number": n, "text": lines[n - 1] })).collect::<Vec<_>>())
}

#[test]
fn a_window_must_be_the_union_of_its_fichas_widened_with_no_line_more_or_less() {
    let catalog = edition(&PROJECT);
    // One line more than the fichas reach.
    let mut more = two_units(&catalog);
    with_query(&mut more, |q| {
        q["evidence"]["contexts"][0]["end_line"] = json!(4);
        q["evidence"]["contexts"][0]["lines"] = block_lines(1, 4);
    });
    assert_eq!(violation(prose(&more)), Violation::CoverageDiffers { unit: "CTX_1".into(), expected: (1, 4), found: vec![(1, 3)] });
    // Lines in the middle that no ficha reaches.
    let mut gap = two_units(&catalog);
    with_query(&mut gap, |q| {
        q["evidence"]["contexts"][0]["end_line"] = json!(10);
        q["evidence"]["contexts"][0]["lines"] = block_lines(1, 10);
        q["evidence"]["contexts"].as_array_mut().unwrap().truncate(1);
        q["evidence"]["selections"][1]["source"]["context"]["context_id"] = json!("CTX_1");
    });
    assert_eq!(violation(prose(&gap)), Violation::CoverageDiffers { unit: "CTX_1".into(), expected: (1, 10), found: vec![(1, 3), (8, 10)] });
    // A single line that no ficha reaches is already a gap.
    let mut one_line = Case::of(&catalog, vec![target("src/a.rs", "dois", 4), target("src/a.rs", "tres", 8)], 0);
    with_query(&mut one_line, |q| {
        q["evidence"]["contexts"][0]["end_line"] = json!(10);
        q["evidence"]["contexts"][0]["lines"] = block_lines(4, 10);
        q["evidence"]["contexts"].as_array_mut().unwrap().truncate(1);
        q["evidence"]["selections"][1]["source"]["context"]["context_id"] = json!("CTX_1");
    });
    assert_eq!(violation(prose(&one_line)), Violation::CoverageDiffers { unit: "CTX_1".into(), expected: (4, 10), found: vec![(4, 6), (8, 10)] });
    // One line less: the ficha's own window in the query says so, and the block cannot hide it.
    let mut less = two_units(&catalog);
    with_query(&mut less, |q| {
        q["evidence"]["contexts"][0]["end_line"] = json!(2);
        q["evidence"]["contexts"][0]["lines"] = block_lines(1, 2);
    });
    assert_eq!(violation(prose(&less)), Violation::FichaOutsideWindow { ficha: "E1".into(), unit: "CTX_1".into() });
    // The individual window a selection records has to be what the radius gives.
    let mut individual = two_units(&catalog);
    with_query(&mut individual, |q| {
        q["evidence"]["selections"][0]["source"]["context"]["end_line"] = json!(4);
    });
    assert_eq!(violation(prose(&individual)), Violation::RadiusDiffers { ficha: "E1".into() });
}

#[test]
fn claims_trim_last_empty_lines_and_keep_the_window_and_the_ficha_inside_it() {
    let catalog = edition(&PROJECT);
    let case = Case::of(&catalog, vec![target("src/b.rs", "outra", 1)], 2);
    let composed = claims(&case).unwrap();
    let unit = &composed.units[0];
    assert_eq!((unit.window_start_line, unit.window_end_line, unit.start_line, unit.end_line), (1, 3, 1, 1));
    assert_eq!(unit.excerpt_sha256.as_ref().unwrap().as_str(), sha(b"fn outra() {}"));
    assert_eq!(unit.text_sha256.as_str(), sha(b"fn outra() {}"));
    assert_eq!(composed.representation, Representation::Claims);
    // A ficha whose last line is past the trim is still inside the window, which is why it exists.
    let case = Case::of(&catalog, vec![target("src/a.rs", "tres", 8)], 2);
    let composed = claims(&case).unwrap();
    let unit = &composed.units[0];
    assert_eq!((unit.window_start_line, unit.window_end_line, unit.start_line, unit.end_line), (6, 12, 6, 10));
    assert!(unit.end_line < unit.window_end_line);
}

#[test]
fn prose_keeps_the_lines_of_the_bundle_with_no_trim_and_no_excerpt() {
    let catalog = edition(&PROJECT);
    let composed = prose(&Case::of(&catalog, vec![target("src/b.rs", "outra", 1)], 2)).unwrap();
    let unit = &composed.units[0];
    assert_eq!((unit.window_start_line, unit.window_end_line, unit.start_line, unit.end_line), (1, 3, 1, 3));
    assert_eq!(unit.excerpt_sha256, None);
    assert_eq!(unit.text_sha256.as_str(), sha(b"fn outra() {}\n\n"));
    assert_eq!(composed.representation, Representation::Prose);
    // The two forms of one window keep their hashes apart: a CR that stays is cut again in claims.
    let case = Case::of(&catalog, vec![target("src/r.rs", "com_cr", 1)], 0);
    let prosa = prose(&case).unwrap();
    let afirmacoes = claims(&case).unwrap();
    assert_eq!(prosa.units[0].text_sha256.as_str(), sha(b"fn com_cr() {\r\n    1;\n}"));
    assert_eq!(afirmacoes.units[0].excerpt_sha256.as_ref().unwrap().as_str(), sha(b"fn com_cr() {\r\n    1;\n}"));
    assert_eq!(afirmacoes.units[0].text_sha256.as_str(), sha(b"fn com_cr() {\n    1;\n}"));
}

/// A query the preparation cannot make today, since a declaration has text: only empty lines inside
/// a window. The source has them, and what is under test is what a construction does with them.
fn empty_window_query(numbered: bool) -> (String, FichaSeed, FichaSeed, Vec<u8>) {
    let bytes = b"a\n\n\nb\n".to_vec();
    let sha256 = sha(&bytes);
    let line = |n: usize, text: &str| if numbered { json!({ "number": n, "text": text }) } else { json!(text) };
    let selection = |id: &str, start: usize, end: usize, ctx: &str| json!({ "fact_id": id, "source": { "id": "S1", "path": "src/v.rs", "sha256": sha256, "line": start, "end_line": end, "context": { "requested_radius": 0, "start_line": start, "end_line": end, "context_id": ctx } } });
    let query = json!({ "evidence": {
        "selections": [selection("E1", 1, 1, "CTX_1"), selection("E2", 2, 3, "CTX_2")],
        "contexts": [
            { "id": "CTX_1", "source_id": "S1", "start_line": 1, "end_line": 1, "lines": [line(1, "a")] },
            { "id": "CTX_2", "source_id": "S1", "start_line": 2, "end_line": 3, "lines": [line(2, ""), line(3, "")] },
        ],
    } });
    let seed = |id: &str, start: usize, end: usize| FichaSeed {
        reference: FichaRef(id.into()), kind: FichaKind::Symbol, symbol_id: None, chunk_id: None, source_id: "src:v".into(), source_sha256: sha256.clone(), source_kind: "texto".into(), git_commit: None,
        path: "src/v.rs".into(), start_line: start, end_line: end, statement: "s".into(), authorship: "a".into(), reasons: vec![], reason_status: ReasonStatus::NotRecorded, declared_by: DeclaredBy::Server,
    };
    (query.to_string(), seed("E1", 1, 1), seed("E2", 2, 3), bytes)
}

#[test]
fn a_claims_window_empty_after_the_trim_fails_the_whole_construction() {
    let (query, one, two, bytes) = empty_window_query(true);
    let sha256 = sha(&bytes);
    let snapshots = [SnapshotInput { source_id: "src:v", path: "src/v.rs", sha256: &sha256, bytes: &bytes }];
    let seeds = [one, two];
    // The claims context leaves that window out; here it fails, whatever the other windows are.
    let items = [ClaimsEvidence { id: "CTX_1", path: "src/v.rs", start_line: 1, end_line: 1, excerpt: "a" }];
    let input = CompositionInput { query_json: &query, seeds: &seeds, radius: 0, form: Form::Claims(&items), snapshots: &snapshots };
    assert_eq!(compose(&input), Err(CompositionError::Normalization { unit: "CTX_2".into(), path: "src/v.rs".into(), error: NormalizationError::EmptyAfterTrim { start_line: 2, end_line: 3 } }));
    // Prose has no trim: the same window is a unit there, with no change in what it is.
    let input = CompositionInput { form: Form::Prose, ..input };
    let composed = compose(&input).unwrap();
    assert_eq!(windows(&composed), [("CTX_1", "src/v.rs", 1, 1), ("CTX_2", "src/v.rs", 2, 3)]);
    assert_eq!(composed.units[1].text_sha256.as_str(), sha(b"\n"));
}

#[test]
fn lines_without_numbers_give_the_same_units() {
    let (numbered, one, two, bytes) = empty_window_query(true);
    let (plain, ..) = empty_window_query(false);
    let sha256 = sha(&bytes);
    let snapshots = [SnapshotInput { source_id: "src:v", path: "src/v.rs", sha256: &sha256, bytes: &bytes }];
    let seeds = [one, two];
    let of = |query_json| compose(&CompositionInput { query_json, seeds: &seeds, radius: 0, form: Form::Prose, snapshots: &snapshots }).unwrap();
    assert_eq!(of(&numbered), of(&plain));
    // And a plain real query, with the same lines as a numbered one.
    let catalog = edition(&PROJECT);
    let request = |numbered: bool| json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "radius": 1, "numbered_lines": numbered, "gathered": true, "targets": [target("src/a.rs", "um", 1), target("src/a.rs", "tres", 8)] });
    assert_eq!(prose(&Case::prepare(&catalog, request(false))).unwrap(), prose(&Case::prepare(&catalog, request(true))).unwrap());
}

#[test]
fn a_snapshot_whose_bytes_are_not_the_recorded_ones_is_refused_even_when_the_window_reads_the_same() {
    let mut case = Case::of(&edition(&PROJECT), vec![target("src/a.rs", "um", 1)], 0);
    let bytes = &mut case.snapshots[0].3;
    assert_eq!(bytes[bytes.len() - 2], b'\n');
    bytes.insert(bytes.len() - 1, b' ');
    assert_eq!(violation(prose(&case)), Violation::SourceHashDiffers { unit: "E1".into() });
    // The edition's record alone being different is enough, and so is the query's.
    let mut case = Case::of(&edition(&PROJECT), vec![target("src/a.rs", "um", 1)], 0);
    case.snapshots[0].2 = text('e');
    assert_eq!(violation(prose(&case)), Violation::SourceHashDiffers { unit: "E1".into() });
    let mut case = Case::of(&edition(&PROJECT), vec![target("src/a.rs", "um", 1)], 0);
    with_query(&mut case, |q| q["evidence"]["source"]["sha256"] = json!(text('e')));
    assert_eq!(violation(prose(&case)), Violation::SourceHashDiffers { unit: "E1".into() });
    // A snapshot that is not there is told apart.
    let mut case = Case::of(&edition(&PROJECT), vec![target("src/a.rs", "um", 1)], 0);
    case.snapshots.clear();
    assert_eq!(prose(&case), Err(CompositionError::MissingSnapshot { path: "src/a.rs".into() }));
    // Bytes that are not text are refused, and say so.
    let mut case = Case::of(&edition(&PROJECT), vec![target("src/a.rs", "um", 1)], 0);
    case.snapshots[0].3 = vec![0xff, b'\n'];
    assert_eq!(prose(&case), Err(CompositionError::Normalization { unit: "E1".into(), path: "src/a.rs".into(), error: NormalizationError::InvalidUtf8 { valid_up_to: 0 } }));
}

#[test]
fn text_that_cannot_be_rebuilt_from_the_snapshot_is_refused() {
    let catalog = edition(&PROJECT);
    let case = Case::of(&catalog, vec![target("src/b.rs", "outra", 1)], 2);
    let composed = prose(&case).unwrap();
    let mut changed = composed.clone();
    changed.units[0].text_sha256 = hash('e');
    assert_eq!(violation_of(check_prose(&case, &changed)), Violation::TextDiffers { unit: "E1".into() });
    // The query's own line differs from the snapshot, with the same number of lines.
    let mut query_changed = Case::of(&catalog, vec![target("src/a.rs", "um", 1)], 0);
    with_query(&mut query_changed, |q| q["evidence"]["source"]["context"]["lines"][1]["text"] = json!("    9;"));
    assert_eq!(violation(prose(&query_changed)), Violation::UnitDiffersFromForm { unit: "E1".into() });
    // The same text under another number is not the same line.
    let mut renumbered = Case::of(&catalog, vec![target("src/a.rs", "um", 1)], 0);
    with_query(&mut renumbered, |q| q["evidence"]["source"]["context"]["lines"][1]["number"] = json!(9));
    assert_eq!(violation(prose(&renumbered)), Violation::UnitDiffersFromForm { unit: "E1".into() });
    // Claims: the excerpt in the context is the one that has to agree, and so does the unit's hash.
    let case = Case::of(&catalog, vec![target("src/b.rs", "outra", 1)], 2);
    let snapshots = case.inputs();
    let mut items = case.claims();
    let composed = claims(&case).unwrap();
    let input = |view: &[ClaimsEvidence<'_>], composed: &ComposedEvidence| check(composed, &CompositionInput { query_json: &case.query, seeds: &case.composition.fichas, radius: 2, form: Form::Claims(view), snapshots: &snapshots });
    let good = claims_view(&items);
    assert_eq!(input(&good, &composed), Ok(()));
    let mut excerpt = composed.clone();
    excerpt.units[0].excerpt_sha256 = Some(hash('e'));
    assert_eq!(violation_of(input(&good, &excerpt)), Violation::UnitDiffersFromForm { unit: "E1".into() });
    // The context and the unit agree with each other on a shorter range, and the snapshot does not.
    let shorter = [ClaimsEvidence { end_line: 2, ..good[0] }];
    let mut short = composed.clone();
    short.units[0].end_line = 2;
    assert_eq!(violation_of(input(&shorter, &short)), Violation::TextDiffers { unit: "E1".into() });
    // A context item that begins elsewhere, with the same text and the same end.
    let later = [ClaimsEvidence { start_line: 2, ..good[0] }];
    assert_eq!(violation_of(input(&later, &composed)), Violation::UnitDiffersFromForm { unit: "E1".into() });
    // The context and the unit agree on another excerpt, and the snapshot does not.
    let other = [ClaimsEvidence { excerpt: "fn outra() { }", ..good[0] }];
    let mut agreed = composed.clone();
    agreed.units[0].excerpt_sha256 = Some(Sha256Hex::try_from(sha(b"fn outra() { }")).unwrap());
    assert_eq!(violation_of(input(&other, &agreed)), Violation::TextDiffers { unit: "E1".into() });
    // A window past the end of the source is told by its numbers, before any text is read.
    let mut past = composed.clone();
    past.units[0].window_end_line = 99;
    assert_eq!(violation_of(input(&good, &past)), Violation::LineNumbers { unit: "E1".into() });
    items[0].4 = "fn outra() { }".into();
    let bad = claims_view(&items);
    assert_eq!(violation_of(input(&bad, &composed)), Violation::UnitDiffersFromForm { unit: "E1".into() });
}

#[test]
fn units_and_form_correspond_in_both_directions() {
    let catalog = edition(&PROJECT);
    let case = two_units(&catalog);
    let composed = prose(&case).unwrap();
    let mut missing = composed.clone();
    missing.units.pop();
    assert_eq!(violation_of(check_prose(&case, &missing)), Violation::FormItemWithoutUnit { item: "CTX_2".into() });
    let mut extra = composed.clone();
    let mut unit = extra.units[0].clone();
    unit.evidence_id = UnitId("CTX_9".into());
    extra.units.push(unit);
    assert_eq!(violation_of(check_prose(&case, &extra)), Violation::UnitNotInForm { unit: "CTX_9".into() });
    let mut twice = composed.clone();
    let unit = twice.units[0].clone();
    twice.units.push(unit);
    assert_eq!(violation_of(check_prose(&case, &twice)), Violation::DuplicateUnit { unit: "CTX_1".into() });
    let mut moved = composed.clone();
    moved.units[0].start_line = 2;
    moved.units[0].window_start_line = 2;
    assert_eq!(violation_of(check_prose(&case, &moved)), Violation::UnitDiffersFromForm { unit: "CTX_1".into() });
    // Each number of the range on its own, with the lines the query has still in place.
    let mut start_only = composed.clone();
    start_only.units[0].start_line = 2;
    assert_eq!(violation_of(check_prose(&case, &start_only)), Violation::UnitDiffersFromForm { unit: "CTX_1".into() });
    let mut end_only = composed.clone();
    end_only.units[0].end_line = 2;
    assert_eq!(violation_of(check_prose(&case, &end_only)), Violation::UnitDiffersFromForm { unit: "CTX_1".into() });
    // Another path holding the very same bytes: the text alone cannot tell them apart.
    let mut twin = two_units(&catalog);
    let (id, _, sha256, bytes) = twin.snapshots[0].clone();
    twin.snapshots.push((id, "src/gemeo.rs".into(), sha256, bytes));
    let mut other_path = composed.clone();
    other_path.units[0].path = "src/gemeo.rs".into();
    assert_eq!(violation_of(check_prose(&twin, &other_path)), Violation::UnitDiffersFromForm { unit: "CTX_1".into() });

    let snapshots = case.inputs();
    let items = case.claims();
    let view = claims_view(&items);
    let composed = claims(&case).unwrap();
    let with = |form: Form<'_>, composed: &ComposedEvidence| check(composed, &CompositionInput { query_json: &case.query, seeds: &case.composition.fichas, radius: 0, form, snapshots: &snapshots });
    assert_eq!(with(Form::Claims(&view), &composed), Ok(()));
    assert_eq!(violation_of(with(Form::Claims(&view[..1]), &composed)), Violation::UnitNotInForm { unit: "CTX_2".into() });
    let more = [view[0], view[1], ClaimsEvidence { id: "CTX_9", ..view[0] }];
    assert_eq!(violation_of(with(Form::Claims(&more), &composed)), Violation::FormItemWithoutUnit { item: "CTX_9".into() });
    let shifted = [view[0], ClaimsEvidence { end_line: view[1].end_line - 1, ..view[1] }];
    assert_eq!(violation_of(with(Form::Claims(&shifted), &composed)), Violation::UnitDiffersFromForm { unit: "CTX_2".into() });
    let other_path = [view[0], ClaimsEvidence { path: "src/b.rs", ..view[1] }];
    assert_eq!(violation_of(with(Form::Claims(&other_path), &composed)), Violation::UnitDiffersFromForm { unit: "CTX_2".into() });
}

#[test]
fn hashes_and_line_numbers_have_to_be_possible_in_their_form() {
    let catalog = edition(&PROJECT);
    let case = Case::of(&catalog, vec![target("src/b.rs", "outra", 1)], 2);
    let composed = prose(&case).unwrap();
    let mut with_excerpt = composed.clone();
    with_excerpt.units[0].excerpt_sha256 = Some(hash('e'));
    assert_eq!(violation_of(check_prose(&case, &with_excerpt)), Violation::ExcerptHashForm { unit: "E1".into() });
    let mut other_hash = composed.clone();
    other_hash.units[0].source_sha256 = hash('e');
    assert_eq!(violation_of(check_prose(&case, &other_hash)), Violation::SourceHashDiffers { unit: "E1".into() });
    let mut other_id = composed.clone();
    other_id.units[0].source_id = "src:outra".into();
    assert_eq!(violation_of(check_prose(&case, &other_id)), Violation::SourceHashDiffers { unit: "E1".into() });
    // In claims the trim can only shorten the end. The context agrees here, so only the numbers say it.
    let snapshots = case.inputs();
    let composed = claims(&case).unwrap();
    let items = case.claims();
    let view = claims_view(&items);
    let past = [ClaimsEvidence { end_line: 4, ..view[0] }];
    let mut numbers = composed.clone();
    numbers.units[0].end_line = 4;
    let result = check(&numbers, &CompositionInput { query_json: &case.query, seeds: &case.composition.fichas, radius: 2, form: Form::Claims(&past), snapshots: &snapshots });
    assert_eq!(violation_of(result), Violation::LineNumbers { unit: "E1".into() });
}

#[test]
fn what_the_preparation_knows_has_to_be_what_the_query_says() {
    let catalog = edition(&PROJECT);
    let good = two_units(&catalog);
    for change in [
        (|s: &mut FichaSeed| s.start_line += 1) as fn(&mut FichaSeed),
        |s| s.end_line += 1,
        |s| s.path = "src/b.rs".into(),
        |s| s.reference = FichaRef("E9".into()),
    ] {
        let mut case = two_units(&catalog);
        change(&mut case.composition.fichas[1]);
        let expected = case.composition.fichas[1].reference.0.clone();
        assert_eq!(violation(prose(&case)), Violation::SeedDiffersFromQuery { ficha: expected });
    }
    let mut fewer = two_units(&catalog);
    fewer.composition.fichas.pop();
    assert!(matches!(prose(&fewer), Err(CompositionError::MalformedQuery(_))));
    let mut broken = two_units(&catalog);
    broken.query = "{".into();
    assert!(matches!(prose(&broken), Err(CompositionError::MalformedQuery(_))));
    // Two fichas with one reference, in a result: the query has them apart.
    let mut twice = prose(&good).unwrap();
    twice.fichas[1].reference = FichaRef("E1".into());
    assert_eq!(violation_of(check_prose(&good, &twice)), Violation::DuplicateFicha { ficha: "E1".into() });
    // A ficha alone in its unit, gone from the result.
    let mut orphan = prose(&good).unwrap();
    orphan.fichas.pop();
    assert_eq!(violation_of(check_prose(&good, &orphan)), Violation::UnitWithoutFicha { unit: "CTX_2".into() });
}

#[test]
fn ids_and_order_are_the_querys_whatever_order_the_fichas_came_in() {
    let catalog = edition(&PROJECT);
    let composed = prose(&Case::of(&catalog, vec![target("src/a.rs", "tres", 8), target("src/b.rs", "outra", 1), target("src/a.rs", "um", 1)], 0)).unwrap();
    assert_eq!(windows(&composed), [("CTX_1", "src/a.rs", 1, 3), ("CTX_2", "src/a.rs", 8, 10), ("CTX_3", "src/b.rs", 1, 1)]);
    assert_eq!(links(&composed), [("E1", "CTX_2"), ("E2", "CTX_3"), ("E3", "CTX_1")]);
    assert_eq!(composed.fichas.iter().map(|f| f.start_line).collect::<Vec<_>>(), [8, 1, 1]);
}

#[test]
fn the_same_inputs_give_the_same_bytes() {
    let request = json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "radius": 1, "numbered_lines": true, "gathered": true, "owner_context": true, "callees": true, "targets": [target("src/d.rs", "ler", 5), target("src/a.rs", "um", 1)], "headers": ["src/h.rs"] });
    let make = || {
        let catalog = edition(&PROJECT);
        let case = Case::prepare(&catalog, request.clone());
        let (a, b) = (prose(&case).unwrap(), claims(&case).unwrap());
        let bytes = |c: &ComposedEvidence| (serde_json::to_vec(&c.fichas).unwrap(), serde_json::to_vec(&c.units).unwrap());
        (case.query.clone(), bytes(&a), bytes(&b), bytes(&prose(&case).unwrap()))
    };
    let (query, one, two, again) = make();
    assert_eq!(one, again);
    assert_ne!(one.1, two.1);
    assert_eq!(make(), (query, one, two, again));
}

#[test]
fn a_searchs_reasons_have_no_code_in_the_contract_and_stay_not_recorded() {
    let catalog = edition(&PROJECT);
    let request: librarian_ingest::prepare::Request = serde_json::from_value(json!({ "question": "gravar ponto", "top": 3, "depth": 0, "hits": 1, "numbered_lines": true })).unwrap();
    let prepared = librarian_ingest::prepare::prepare(&catalog, &request).unwrap();
    let search: Value = serde_json::from_slice(&prepared.search).unwrap();
    assert!(!search["hits"][0]["reasons"].as_array().unwrap().is_empty(), "the search did record reasons");
    let seed = &prepared.composition.fichas[0];
    assert_eq!(seed.reasons, vec![]);
    assert_eq!(seed.reason_status, ReasonStatus::NotRecorded);
    assert_eq!(seed.declared_by, DeclaredBy::Server);
    assert_eq!(seed.symbol_id.as_deref(), search["hits"][0]["symbol_id"].as_str());
    assert_eq!(seed.chunk_id.as_deref(), search["hits"][0]["chunk_id"].as_str());
}

#[test]
fn what_the_owner_context_recorded_comes_through_and_a_gathered_target_is_the_clients() {
    let catalog = edition(&PROJECT);
    let case = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "gathered": true, "owner_context": true, "targets": [target("src/d.rs", "ler", 5)] }));
    let search: Vec<(FichaKind, DeclaredBy, ReasonStatus, Vec<ReasonCode>)> = case.composition.fichas.iter().map(|s| (s.kind, s.declared_by, s.reason_status, s.reasons.iter().map(|r| r.code).collect())).collect();
    assert_eq!(search[0], (FichaKind::Symbol, DeclaredBy::Client, ReasonStatus::NotRecorded, vec![]));
    let owner = case.composition.fichas.iter().find(|s| s.reasons.iter().any(|r| r.code == ReasonCode::OwnerType)).unwrap();
    assert_eq!((owner.declared_by, owner.reason_status), (DeclaredBy::Server, ReasonStatus::ServerRecorded));
    assert_eq!(owner.reasons[0].fields, ["x"]);
    assert!(owner.reasons[0].for_target.ends_with("ler"), "{}", owner.reasons[0].for_target);
    let writer = case.composition.fichas.iter().find(|s| s.reasons.iter().any(|r| r.code == ReasonCode::FieldWriter)).unwrap();
    assert_eq!(writer.reasons[0].lines, [9]);
    // What the seeds say is what the fichas say.
    let composed = prose(&case).unwrap();
    for (ficha, seed) in composed.fichas.iter().zip(&case.composition.fichas) {
        assert_eq!((&ficha.reasons, ficha.reason_status, ficha.declared_by), (&seed.reasons, seed.reason_status, seed.declared_by));
    }
    assert!(composed.fichas.iter().any(|f| f.reason_status == ReasonStatus::ServerRecorded && !f.reasons.is_empty()));
    // A declaration the target calls, with the line of the call.
    let called = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "gathered": true, "owner_context": true, "callees": true, "targets": [target("src/k.rs", "chamadora", 2)] }));
    let seed = &called.composition.fichas[1];
    assert_eq!(seed.symbol_id.as_deref().map(|id| catalog.symbols()[id].name.as_str()), Some("chamada_unica"));
    assert_eq!((seed.reasons.len(), seed.reasons[0].code, seed.reasons[0].lines.clone()), (1, ReasonCode::Called, vec![3]));
    // A step named by checked answers was not picked by the client.
    let chain = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "targets": [target("src/d.rs", "ler", 5)] }));
    assert_eq!(chain.composition.fichas[0].declared_by, DeclaredBy::Server);
}

#[test]
fn a_file_head_is_a_ficha_of_its_kind_with_no_symbol() {
    let catalog = edition(&PROJECT);
    let case = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "gathered": true, "targets": [target("src/h.rs", "depois", 3)], "headers": ["src/h.rs"] }));
    let composed = prose(&case).unwrap();
    assert_eq!(windows(&composed), [("CTX_1", "src/h.rs", 1, 1), ("CTX_2", "src/h.rs", 3, 3)]);
    assert_eq!(links(&composed), [("E1", "CTX_2"), ("E2", "CTX_1")]);
    let head = &composed.fichas[1];
    assert_eq!((head.kind, head.symbol_id.clone(), head.chunk_id.clone(), head.declared_by, head.reason_status), (FichaKind::FileHead, None, None, DeclaredBy::Client, ReasonStatus::NotRecorded));
}

fn tags(case: &Case) -> Vec<(LimitProducer, LimitInput)> {
    let query = case.query_value();
    let evidence = &query["evidence"];
    let written = evidence["evidence_unknowns"].as_array().or(evidence["selections"][0]["evidence_unknowns"].as_array()).unwrap();
    let flat: Vec<&str> = case.composition.limits.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(flat, written.iter().map(|l| l.as_str().unwrap()).collect::<Vec<_>>(), "the limits, flattened, are the query's");
    case.composition.limits.iter().map(|l| (l.producer, l.input)).collect()
}

#[test]
fn limits_keep_their_order_and_text_and_say_what_wrote_them() {
    use LimitInput as I;
    use LimitProducer as P;
    let catalog = edition(&PROJECT);
    let search = Case::prepare(&catalog, json!({ "question": "gravar ponto", "top": 3, "depth": 0, "hits": 1 }));
    let found = tags(&search);
    let (last, profile) = found.split_last().unwrap();
    assert!(profile.len() >= 2);
    assert!(profile.iter().all(|t| *t == (P::ProfileSearch, I::EditionProfile)), "{profile:?}");
    assert_eq!(*last, (P::FichaNature, I::None));

    // The line of exclude_tests has a producer of its own.
    let without_tests = Case::prepare(&catalog, json!({ "question": "gravar ponto", "top": 3, "depth": 0, "hits": 1, "exclude_tests": true }));
    let found = tags(&without_tests);
    assert_eq!(found.len(), profile.len() + 2);
    assert_eq!(found[profile.len()], (P::ExcludeTests, I::Request));
    assert!(without_tests.composition.limits[profile.len()].text.starts_with("Test symbols"));
    assert_eq!(found[profile.len() + 1], (P::FichaNature, I::None));

    let chain = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "targets": [target("src/a.rs", "um", 1)] }));
    let found = tags(&chain);
    assert_eq!(found[0], (P::StepKind, I::Route));
    assert!(chain.composition.limits[0].text.starts_with("Chain step"));
    assert!(found[1..found.len() - 1].iter().all(|t| *t == (P::ProfileSearch, I::EditionProfile)), "{found:?}");
    assert_eq!(found[found.len() - 1], (P::FichaNature, I::None));

    let reviewed = json!({ "id": "F1", "statement": "s", "authorship": "a", "path": "src/d.rs", "line": 6, "end_line": 6, "citations": [] });
    let full = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "gathered": true, "owner_context": true, "callees": true, "targets": [target("src/d.rs", "ler", 5), target("src/h.rs", "depois", 3)], "headers": ["src/h.rs"], "reviewed_facts": [reviewed], "notes": ["uma nota", "outra nota"] }));
    let found = tags(&full);
    assert_eq!(found[..2], [(P::StepKind, I::Route), (P::OwnerStep, I::Request)]);
    assert!(full.composition.limits[0].text.starts_with("Gathered step"));
    assert!(full.composition.limits[1].text.starts_with("Owner context"));
    let callees = found.iter().position(|t| *t == (P::Callees, I::Request)).unwrap();
    assert!(full.composition.limits[callees].text.starts_with("Called declarations"));
    assert!(found[2..callees].iter().all(|t| *t == (P::ProfileSearch, I::EditionProfile)), "{found:?}");
    assert_eq!(found[callees + 1], (P::FichaNature, I::None));
    assert_eq!(found[callees + 2], (P::OwnerContext, I::SearchReport));
    let end = found.len();
    assert_eq!(found[end - 4..], [(P::ReviewedNote, I::Request), (P::HeaderNote, I::Request), (P::RequestNotes, I::Client), (P::RequestNotes, I::Client)]);
    assert_eq!(full.composition.limits[end - 1].text, "outra nota");
    assert_eq!(full.composition.limits[end - 2].text, "uma nota");
    assert!(found[callees + 2..end - 4].iter().all(|t| *t == (P::OwnerContext, I::SearchReport)), "{found:?}");
}

#[test]
fn an_edition_with_diagnostics_says_so_in_a_limit_of_its_own() {
    let catalog = edition(&[("src/a.rs", A_RS), ("src/q.rs", b"fn quebrado( {\n")]);
    assert!(!catalog.diagnostics().is_empty(), "the source above has to give a diagnostic");
    let case = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "accept_diagnostics": true, "targets": [target("src/a.rs", "um", 1)] }));
    let found = tags(&case);
    assert_eq!(*found.last().unwrap(), (LimitProducer::Diagnostics, LimitInput::Edition));
    assert_eq!(found[found.len() - 2], (LimitProducer::FichaNature, LimitInput::None));
}

// Provenance and U6, over what the preparation really gives. The claims queries are fixtures written
// by the real code of the other crate (prepare::write, then its claims::write), from the sources and
// the request below; this crate cannot call that code, so a new field there is seen here only when
// the fixtures are written again, and the permanent comparison belongs to the step that joins them.

const FACT_RECORD: &str = r#"{"authorship":"revisão humana no Librarian Studio","citations":[{"end_line":6,"path":"src/d.rs","quote":"        self.x","start_line":6}],"end_line":6,"event":"promoted","format_version":1,"id":"F1","kind":"FACT","line":6,"note":"lido","path":"src/d.rs","statement":"ler devolve o campo x."}"#;

struct Fixture {
    case: Case,
    query: String,
    context: Value,
}

fn fixture(catalog: &librarian_ingest::Catalog, name: &str) -> Fixture {
    let (request, query, context) = match name {
        "fatos" => (include_str!("evidence-fixtures/fatos.request.json"), include_str!("evidence-fixtures/fatos.query.json"), include_str!("evidence-fixtures/fatos.context.json")),
        "simples" => (include_str!("evidence-fixtures/simples.request.json"), include_str!("evidence-fixtures/simples.query.json"), include_str!("evidence-fixtures/simples.context.json")),
        "compacta" => (include_str!("evidence-fixtures/compacta.request.json"), include_str!("evidence-fixtures/compacta.query.json"), include_str!("evidence-fixtures/compacta.context.json")),
        other => panic!("{other}"),
    };
    Fixture { case: Case::prepare(catalog, serde_json::from_str(request).unwrap()), query: query.to_owned(), context: serde_json::from_str(context).unwrap() }
}

impl Fixture {
    fn items(&self) -> Vec<(String, String, usize, usize, String)> {
        self.context["evidence"].as_array().unwrap().iter().map(|e| (e["id"].as_str().unwrap().to_owned(), e["path"].as_str().unwrap().to_owned(), e["start_line"].as_u64().unwrap() as usize, e["end_line"].as_u64().unwrap() as usize, e["excerpt"].as_str().unwrap().to_owned())).collect()
    }
    fn composed(&self) -> ComposedEvidence {
        let snapshots = self.case.inputs();
        let items = self.items();
        let view = claims_view(&items);
        compose(&CompositionInput { query_json: &self.case.query, seeds: &self.case.composition.fichas, radius: self.case.composition.radius, form: Form::Claims(&view), snapshots: &snapshots }).unwrap()
    }
    fn with_query(&mut self, change: impl FnOnce(&mut Value)) {
        let mut query: Value = serde_json::from_str(&self.query).unwrap();
        change(&mut query);
        self.query = serde_json::to_string_pretty(&query).unwrap();
    }
    fn provenance(&self, records: &[FactRecord], declared: Option<&[String]>) -> Result<QueryProvenance, CompositionError> {
        let snapshots = self.case.inputs();
        let composed = self.composed();
        let question: Value = serde_json::from_str(&self.query).unwrap();
        let question = question["question"].as_str().unwrap().to_owned();
        build_provenance(&ProvenanceInput { query_json: &self.query, question: QuestionInput { original: &question, suffixes: &[] }, search: &self.case.search, composition: &self.case.composition, composed: &composed, snapshots: &snapshots, fact_records: records, project_instructions: declared })
    }
}

fn prose_provenance_with(case: &Case, records: &[FactRecord], question: QuestionInput) -> Result<QueryProvenance, CompositionError> {
    let snapshots = case.inputs();
    let composed = prose(case).unwrap();
    build_provenance(&ProvenanceInput { query_json: &case.query, question, search: &case.search, composition: &case.composition, composed: &composed, snapshots: &snapshots, fact_records: records, project_instructions: None })
}

fn prose_provenance(case: &Case, records: &[FactRecord]) -> Result<QueryProvenance, CompositionError> {
    let question = case.query_value()["question"].as_str().unwrap().to_owned();
    prose_provenance_with(case, records, QuestionInput { original: &question, suffixes: &[] })
}

fn provenance_violation(result: Result<QueryProvenance, CompositionError>) -> Violation {
    match result {
        Err(CompositionError::Violation(violation)) => violation,
        other => panic!("expected a violation: {other:?}"),
    }
}

fn reviewed_fact() -> Value {
    json!({ "id": "F1", "statement": "ler devolve o campo x.", "authorship": "revisão humana no Librarian Studio", "path": "src/d.rs", "line": 6, "end_line": 6, "citations": [{ "path": "src/d.rs", "start_line": 6, "end_line": 6, "quote": "        self.x" }] })
}

fn with_fact(catalog: &librarian_ingest::Catalog) -> Case {
    Case::prepare(catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "gathered": true, "targets": [target("src/d.rs", "ler", 5)], "reviewed_facts": [reviewed_fact()] }))
}

/// The lines of the owner context for one method of a project made for it.
fn owner_lines(files: &[(&str, &[u8])], method: Value) -> Vec<TaggedLimit> {
    let catalog = edition(files);
    let case = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "gathered": true, "owner_context": true, "targets": [method] }));
    prose_provenance(&case, &[]).unwrap();
    case.composition.limits.into_iter().filter(|l| l.producer == LimitProducer::OwnerContext).collect()
}

fn entries(case: &Case) -> Vec<LimitEntry> {
    prose_provenance(case, &[]).unwrap().limits
}

#[test]
fn every_producer_of_limits_has_its_input_rule_class_and_provenance() {
    use LimitInput as I;
    use LimitProducer as P;
    use LimitProvenance as V;
    let catalog = edition(&PROJECT);
    let profile = catalog.manifest().extractor.clone();
    let row = |e: &LimitEntry| (e.producer, e.input, e.rule_version.clone(), e.class, e.provenance, e.basis.clone());
    let system = LimitClass::SystemText;
    let search = entries(&Case::prepare(&catalog, json!({ "question": "gravar ponto", "top": 3, "depth": 0, "hits": 1, "numbered_lines": true })));
    let of_profile = search.len() - 1;
    assert!(of_profile >= 2);
    for e in &search[..of_profile] {
        assert_eq!(row(e), (P::ProfileSearch, I::EditionProfile, Some(profile.clone()), system, V::SystemFixed, None));
    }
    assert_eq!(row(&search[of_profile]), (P::FichaNature, I::None, None, system, V::SystemFixed, None));
    assert_eq!(search.iter().map(|e| e.index).collect::<Vec<_>>(), (0..search.len()).collect::<Vec<_>>());

    let chain = entries(&Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "targets": [target("src/a.rs", "um", 1)] })));
    assert_eq!(row(&chain[0]), (P::StepKind, I::Route, Some("chain-step-v1".to_owned()), system, V::SystemFixed, None));

    let full = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "gathered": true, "owner_context": true, "callees": true, "targets": [target("src/d.rs", "ler", 5), target("src/h.rs", "depois", 3)], "headers": ["src/h.rs"], "reviewed_facts": [reviewed_fact()], "notes": ["uma nota"] }));
    let found = entries(&full);
    let of = |producer: P| found.iter().filter(move |e| e.producer == producer).map(row);
    assert_eq!(of(P::OwnerStep).next().unwrap(), (P::OwnerStep, I::Request, Some("chain-step-v1".to_owned()), system, V::SystemGenerated, None));
    assert_eq!(of(P::Callees).next().unwrap(), (P::Callees, I::Request, None, system, V::SystemFixed, None));
    assert_eq!(of(P::OwnerContext).next().unwrap(), (P::OwnerContext, I::SearchReport, None, system, V::SystemFixed, None));
    assert_eq!(of(P::ReviewedNote).next().unwrap(), (P::ReviewedNote, I::Request, None, system, V::SystemFixed, None));
    assert_eq!(of(P::HeaderNote).next().unwrap(), (P::HeaderNote, I::Request, None, system, V::SystemFixed, None));
    assert_eq!(of(P::RequestNotes).next().unwrap(), (P::RequestNotes, I::Client, None, LimitClass::ClientText, V::UnverifiedClientText, None));

    let broken = edition(&[("src/a.rs", A_RS), ("src/q.rs", b"fn quebrado( {\n")]);
    let diagnosed = entries(&Case::prepare(&broken, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "accept_diagnostics": true, "targets": [target("src/a.rs", "um", 1)] })));
    assert_eq!(row(diagnosed.last().unwrap()), (P::Diagnostics, I::Edition, None, system, V::SystemGenerated, None));

    // Lines of the owner context whose text carries a value are generated, not fixed.
    let said = |lines: &[TaggedLimit], part: &str| lines.iter().find(|l| l.text.contains(part)).map(|l| (l.provenance, l.input));
    let mut writers = String::new();
    for n in 0..10 {
        writers.push_str(&format!("    fn g{n}(&mut self) {{\n        self.x = {n};\n    }}\n"));
    }
    let many = format!("struct Ponto {{\n    x: u8,\n}}\nimpl Ponto {{\n    fn ler(&self) -> u8 {{\n        self.x\n    }}\n{writers}}}\n");
    let lines = owner_lines(&[("src/m.rs", many.as_bytes())], target("src/m.rs", "ler", 5));
    assert_eq!(said(&lines, "Contexto do dono: o tipo"), Some((V::SystemFixed, I::SearchReport)), "{lines:?}");
    assert_eq!(said(&lines, "truncado"), Some((V::SystemGenerated, I::SearchReport)), "{lines:?}");
    let other: &[u8] = b"struct Ponto {\n    x: u8,\n}\nimpl Ponto {\n    fn ler(&self) -> u8 {\n        self.x\n    }\n}\nfn fora() {\n    let mut c = novo();\n    c.x = 2;\n}\n";
    let lines = owner_lines(&[("src/o.rs", other)], target("src/o.rs", "ler", 5));
    assert_eq!(said(&lines, "não foram seguidas"), Some((V::SystemGenerated, I::SearchReport)), "{lines:?}");
    let lines = owner_lines(&[("src/u.rs", b"impl Dup {\n    fn ve(&self) -> u8 {\n        self.y\n    }\n}\n"), ("src/v.rs", b"struct Dup {\n    y: u8,\n}\n"), ("src/w.rs", b"struct Dup {\n    y: u8,\n}\n")], target("src/u.rs", "ve", 2));
    assert_eq!(said(&lines, "declarações com esse nome"), Some((V::SystemGenerated, I::SearchReport)), "{lines:?}");
}

#[test]
fn exclude_tests_has_a_producer_of_its_own_and_the_line_keeps_its_text_and_place() {
    let catalog = edition(&PROJECT);
    pinned(LimitProducer::ExcludeTests, json!("exclude_tests"));
    let without = Case::prepare(&catalog, json!({ "question": "gravar ponto", "top": 3, "depth": 0, "hits": 1, "numbered_lines": true, "exclude_tests": true }));
    let with = Case::prepare(&catalog, json!({ "question": "gravar ponto", "top": 3, "depth": 0, "hits": 1, "numbered_lines": true }));
    let found = entries(&without);
    let line: Vec<&LimitEntry> = found.iter().filter(|e| e.producer == LimitProducer::ExcludeTests).collect();
    assert_eq!(line.len(), 1);
    let line = line[0];
    assert_eq!(line.input, LimitInput::Request);
    assert_eq!(line.rule_version.as_deref(), Some("lexical-search-v1"));
    assert_eq!((line.class, line.provenance, line.basis.clone()), (LimitClass::SystemText, LimitProvenance::SystemFixed, None));
    let text = "Test symbols marked by the extractor, and declarations inside them, were left out; helpers outside test functions remain.";
    assert_eq!(line.text_sha256.as_str(), sha(text.as_bytes()));
    // After the profile's lines, before the rest, and only when the search left tests out.
    assert_eq!(line.index, found.len() - 2);
    assert_eq!(without.composition.limits[line.index].text, text);
    assert!(!entries(&with).iter().any(|e| e.producer == LimitProducer::ExcludeTests));
}

#[test]
fn a_limit_changed_with_the_hash_of_the_query_redone_has_no_origin() {
    let catalog = edition(&PROJECT);
    let mut case = two_units(&catalog);
    let good = prose_provenance(&case, &[]).unwrap();
    with_query(&mut case, |q| {
        for selection in q["evidence"]["selections"].as_array_mut().unwrap() {
            selection["evidence_unknowns"][0] = json!("Nada aqui tem limite.");
        }
    });
    // The new query's own hash is good: this is about where the line came from.
    assert_ne!(sha(case.query.as_bytes()), good.sha256.as_str());
    assert_eq!(provenance_violation(prose_provenance(&case, &[])), Violation::LimitTextDiffers { index: 0 });
    // The two lines swapped: each text exists, in the wrong place.
    let mut swapped = two_units(&catalog);
    with_query(&mut swapped, |q| {
        for selection in q["evidence"]["selections"].as_array_mut().unwrap() {
            selection["evidence_unknowns"].as_array_mut().unwrap().swap(0, 1);
        }
    });
    assert_eq!(provenance_violation(prose_provenance(&swapped, &[])), Violation::LimitTextDiffers { index: 0 });
    // One line gone from the end.
    let mut fewer = two_units(&catalog);
    with_query(&mut fewer, |q| {
        for selection in q["evidence"]["selections"].as_array_mut().unwrap() {
            selection["evidence_unknowns"].as_array_mut().unwrap().pop();
        }
    });
    let lines = fewer.composition.limits.len();
    assert_eq!(provenance_violation(prose_provenance(&fewer, &[])), Violation::LimitTextDiffers { index: lines - 1 });
    // Two selections saying different things have no one list of limits.
    let mut apart = two_units(&catalog);
    with_query(&mut apart, |q| q["evidence"]["selections"][1]["evidence_unknowns"][0] = json!("Outro."));
    assert_eq!(provenance_violation(prose_provenance(&apart, &[])), Violation::FieldDiffers { item: "E2".into(), field: "evidence_unknowns".into() });
}

#[test]
fn a_limit_without_a_producer_or_with_an_unknown_one_is_refused() {
    let catalog = edition(&PROJECT);
    let mut extra = two_units(&catalog);
    with_query(&mut extra, |q| {
        for selection in q["evidence"]["selections"].as_array_mut().unwrap() {
            selection["evidence_unknowns"].as_array_mut().unwrap().push(json!("Linha sem produtor."));
        }
    });
    let lines = extra.composition.limits.len();
    assert_eq!(provenance_violation(prose_provenance(&extra, &[])), Violation::LimitWithoutOrigin { index: lines });
    let good = json!({ "index": 0, "text_sha256": text('b'), "producer": "exclude_tests", "input": "request", "rule_version": "lexical-search-v1", "basis": null, "class": "system_text", "provenance": "system_fixed" });
    assert!(serde_json::from_value::<LimitEntry>(good.clone()).is_ok());
    let mut none = good.clone();
    none.as_object_mut().unwrap().remove("producer");
    assert!(serde_json::from_value::<LimitEntry>(none).is_err());
    let mut unknown = good.clone();
    unknown["producer"] = json!("alguem");
    assert!(serde_json::from_value::<LimitEntry>(unknown).is_err());
}

#[test]
fn a_recorded_provenance_differing_in_any_part_of_a_limit_is_refused() {
    let catalog = edition(&PROJECT);
    let case = Case::prepare(&catalog, json!({ "question": "gravar ponto", "top": 3, "depth": 0, "hits": 1, "numbered_lines": true, "exclude_tests": true }));
    let snapshots = case.inputs();
    let composed = prose(&case).unwrap();
    let question = case.query_value()["question"].as_str().unwrap().to_owned();
    let input = ProvenanceInput { query_json: &case.query, question: QuestionInput { original: &question, suffixes: &[] }, search: &case.search, composition: &case.composition, composed: &composed, snapshots: &snapshots, fact_records: &[], project_instructions: None };
    let good = build_provenance(&input).unwrap();
    assert_eq!(check_provenance(&good, &input), Ok(()));
    let refused = |change: &dyn Fn(&mut QueryProvenance), part: &str| {
        let mut changed = good.clone();
        change(&mut changed);
        assert_eq!(check_provenance(&changed, &input), Err(CompositionError::Violation(Violation::ProvenanceDiffers { part: part.into() })));
    };
    refused(&|p| p.limits[0].producer = LimitProducer::Diagnostics, "limits");
    refused(&|p| p.limits[0].input = LimitInput::Client, "limits");
    refused(&|p| p.limits[0].rule_version = Some("outra".into()), "limits");
    refused(&|p| p.limits[0].text_sha256 = hash('e'), "limits");
    refused(&|p| p.limits[0].class = LimitClass::ClientText, "limits");
    refused(&|p| p.limits[0].provenance = LimitProvenance::NotAvailable, "limits");
    refused(&|p| p.limits.swap(0, 1), "limits");
    refused(&|p| p.fields[0].class = QueryFieldClass::Code, "fields");
    refused(&|p| p.fields[0].origin = FieldOrigin::Client, "fields");
    refused(&|p| p.fields[0].provenance = FieldProvenance::NotAvailable, "fields");
    refused(&|p| p.fields.pop().map(|_| ()).unwrap(), "fields");
    refused(&|p| p.sha256 = hash('e'), "sha256");
    refused(&|p| p.fixed_contract_sha256 = hash('e'), "fixed_contract_sha256");
    refused(&|p| p.project_instructions_sha256 = Some(hash('e')), "project_instructions_sha256");
    refused(&|p| p.prepared_question_sha256 = hash('e'), "prepared_question_sha256");
    refused(&|p| p.person_text.sha256 = hash('e'), "person_text");
    refused(&|p| p.code.units.clear(), "code");
}

#[test]
fn a_reviewed_fact_with_its_record_is_tied_to_it_and_to_its_ficha_unit_and_quotes() {
    let catalog = edition(&PROJECT);
    let case = with_fact(&catalog);
    let records = [FactRecord { fact_id: "F1", bytes: FACT_RECORD.as_bytes() }];
    let found = prose_provenance(&case, &records).unwrap();
    assert_eq!(found.reviewed_facts, vec![ReviewedFactRef {
        fact_id: "F1".into(),
        record_sha256: Some(Sha256Hex::try_from(sha(FACT_RECORD.as_bytes())).unwrap()),
        statement_sha256: Sha256Hex::try_from(sha("ler devolve o campo x.".as_bytes())).unwrap(),
        ficha: FichaRef("F1".into()),
        unit: UnitId("CTX_1".into()),
        citations: vec![CitationCheck { path: "src/d.rs".into(), start_line: 6, end_line: 6, quote_sha256: Sha256Hex::try_from(sha(b"        self.x")).unwrap(), checked_against: QuoteCheckedAgainst::Edition }],
        provenance: ReviewedFactProvenance::AcervoQuotesChecked,
    }]);
    let of_fact: Vec<ClassifiedField> = found.classified().into_iter().filter(|f| f.ficha.as_ref().is_some_and(|r| r.0 == "F1") && f.path.ends_with(".statement")).collect();
    assert_eq!(of_fact.len(), 1);
    assert_eq!((of_fact[0].class, of_fact[0].origin, of_fact[0].provenance), (QueryFieldClass::ReviewedFact, FieldOrigin::Acervo, FieldProvenance::AcervoQuotesChecked));
    assert_eq!(of_fact[0].unit, Some(UnitId("CTX_1".into())));
    // A symbol's statement beside it stays the server's text.
    let of_symbol = found.classified().into_iter().find(|f| f.ficha.as_ref().is_some_and(|r| r.0 == "E1") && f.path.ends_with(".statement")).unwrap();
    assert_eq!((of_symbol.class, of_symbol.origin, of_symbol.provenance), (QueryFieldClass::FichaDescription, FieldOrigin::Server, FieldProvenance::SystemGenerated));
}

#[test]
fn a_reviewed_fact_with_no_record_among_the_inputs_says_so_and_no_hash_is_made_up() {
    let catalog = edition(&PROJECT);
    let case = with_fact(&catalog);
    let found = prose_provenance(&case, &[]).unwrap();
    let fact = &found.reviewed_facts[0];
    assert_eq!((fact.record_sha256.clone(), fact.provenance), (None, ReviewedFactProvenance::RecordNotAvailable));
    // The quotes can still be checked: they are in the request and in the snapshot.
    assert_eq!(fact.citations.len(), 1);
    let statement = found.classified().into_iter().find(|f| f.ficha.as_ref().is_some_and(|r| r.0 == "F1") && f.path.ends_with(".statement")).unwrap();
    assert_eq!(statement.provenance, FieldProvenance::RecordNotAvailable);
    // Another fact's record does not stand for this one.
    let other = [FactRecord { fact_id: "F2", bytes: FACT_RECORD.as_bytes() }];
    assert_eq!(prose_provenance(&case, &other).unwrap().reviewed_facts[0].record_sha256, None);
}

#[test]
fn a_quote_changed_or_its_hash_changed_is_refused() {
    let catalog = edition(&PROJECT);
    let mut case = with_fact(&catalog);
    let snapshots = case.inputs();
    let composed = prose(&case).unwrap();
    let question = case.query_value()["question"].as_str().unwrap().to_owned();
    let input = ProvenanceInput { query_json: &case.query, question: QuestionInput { original: &question, suffixes: &[] }, search: &case.search, composition: &case.composition, composed: &composed, snapshots: &snapshots, fact_records: &[], project_instructions: None };
    let mut changed_hash = build_provenance(&input).unwrap();
    changed_hash.reviewed_facts[0].citations[0].quote_sha256 = hash('e');
    assert_eq!(check_provenance(&changed_hash, &input), Err(CompositionError::Violation(Violation::ProvenanceDiffers { part: "reviewed_facts".into() })));
    drop(snapshots);
    case.composition.reviewed[0].citations[0].quote = "        self.y".into();
    assert_eq!(provenance_violation(prose_provenance(&case, &[])), Violation::QuoteDiffers { fact: "F1".into(), path: "src/d.rs".into() });
    // A shifted line is a different quote, and a range past the end has no lines to compare.
    let mut shifted = with_fact(&catalog);
    shifted.composition.reviewed[0].citations[0].start_line = 7;
    shifted.composition.reviewed[0].citations[0].end_line = 7;
    assert_eq!(provenance_violation(prose_provenance(&shifted, &[])), Violation::QuoteDiffers { fact: "F1".into(), path: "src/d.rs".into() });
    let mut past = with_fact(&catalog);
    past.composition.reviewed[0].citations[0].end_line = 999;
    assert_eq!(provenance_violation(prose_provenance(&past, &[])), Violation::QuoteDiffers { fact: "F1".into(), path: "src/d.rs".into() });
}

#[test]
fn a_statement_or_anything_else_changed_in_the_fact_is_refused_by_its_record_and_by_the_query() {
    let catalog = edition(&PROJECT);
    let case = with_fact(&catalog);
    for (field, value) in [("event", json!("revoked")), ("id", json!("F9")), ("statement", json!("ler devolve outra coisa.")), ("authorship", json!("outra")), ("path", json!("src/a.rs")), ("line", json!(7)), ("end_line", json!(7)), ("citations", json!([]))] {
        let mut record: Value = serde_json::from_str(FACT_RECORD).unwrap();
        record[field] = value;
        let bytes = record.to_string();
        let records = [FactRecord { fact_id: "F1", bytes: bytes.as_bytes() }];
        assert_eq!(provenance_violation(prose_provenance(&case, &records)), Violation::FactRecordDiffers { fact: "F1".into(), field: field.into() });
    }
    let records = [FactRecord { fact_id: "F1", bytes: b"nao e json" }];
    assert_eq!(provenance_violation(prose_provenance(&case, &records)), Violation::FactRecordDiffers { fact: "F1".into(), field: "record".into() });
    // The query says another statement from the one the preparation carried.
    let mut query_changed = with_fact(&catalog);
    with_query(&mut query_changed, |q| q["evidence"]["selections"][1]["statement"] = json!("Outra afirmação."));
    assert_eq!(provenance_violation(prose_provenance(&query_changed, &[])), Violation::FieldDiffers { item: "F1".into(), field: "statement".into() });
}

/// A field as a row of a table: where it is written, for what, and what is said of it.
fn table(found: &QueryProvenance, ficha: &str) -> Vec<(String, QueryFieldClass, FieldOrigin, FieldProducer, FieldProvenance)> {
    found.classified().into_iter().filter(|f| f.ficha.as_ref().is_some_and(|r| r.0 == ficha)).map(|f| (f.path.clone(), f.class, f.origin, f.producer, f.provenance)).collect()
}

#[test]
fn each_field_of_the_prose_form_has_its_class_and_origin_by_ficha() {
    use FieldOrigin as O;
    use FieldProducer as B;
    use FieldProvenance as V;
    use QueryFieldClass as C;
    let catalog = edition(&PROJECT);
    let case = with_fact(&catalog);
    let found = prose_provenance(&case, &[]).unwrap();
    let p = "$.evidence.selections[]";
    let row = |name: &str, c, o, b, v| (format!("{p}{name}"), c, o, b, v);
    let mut expected_e1 = vec![
        row(".evidence_id", C::QueryIdentity, O::Server, B::Preparation, V::SystemGenerated),
        row(".evidence_unknowns[]", C::Limit, O::Server, B::LimitEntries, V::SystemGenerated),
        row(".unknowns_scope", C::FixedContract, O::Contract, B::GraphEngine, V::SystemFixed),
        row(".fact_id", C::Structure, O::Server, B::GraphEngine, V::SystemGenerated),
        row(".statement", C::FichaDescription, O::Server, B::Preparation, V::SystemGenerated),
        row(".authorship", C::FichaDescription, O::Server, B::Preparation, V::SystemGenerated),
        row(".metadata_scope", C::FixedContract, O::Contract, B::GraphEngine, V::SystemFixed),
        row(".scope", C::FixedContract, O::Contract, B::GraphEngine, V::SystemFixed),
        row(".source.id", C::Structure, O::Server, B::GraphEngine, V::SystemGenerated),
        row(".source.kind", C::SourceMetadata, O::Edition, B::Preparation, V::EditionVerified),
        // A project with no commit recorded: nothing can be told.
        row(".source.git_commit", C::SourceMetadata, O::Edition, B::Preparation, V::NotAvailable),
        row(".source.path", C::SourceMetadata, O::Edition, B::Preparation, V::EditionVerified),
        row(".source.sha256", C::SourceMetadata, O::Edition, B::Preparation, V::EditionVerified),
        row(".source.line", C::SourceMetadata, O::Edition, B::Preparation, V::EditionVerified),
        row(".source.end_line", C::SourceMetadata, O::Edition, B::Preparation, V::EditionVerified),
        row(".source.excerpt", C::Code, O::Edition, B::GraphEngine, V::EditionVerified),
        row(".source.content", C::FixedContract, O::Contract, B::Preparation, V::SystemFixed),
        row(".source.execution_scope", C::FixedContract, O::Contract, B::GraphEngine, V::SystemFixed),
        row(".source.context.requested_radius", C::RequestSetting, O::Request, B::GraphEngine, V::RequestRecorded),
        row(".source.context.start_line", C::Code, O::Edition, B::GraphEngine, V::EditionVerified),
        row(".source.context.end_line", C::Code, O::Edition, B::GraphEngine, V::EditionVerified),
        row(".source.context.context_id", C::Structure, O::Server, B::GraphEngine, V::SystemGenerated),
    ];
    let sort = |rows: &mut Vec<(String, C, O, B, V)>| rows.sort_by(|a, b| a.0.cmp(&b.0));
    sort(&mut expected_e1);
    let mut got = table(&found, "E1");
    sort(&mut got);
    assert_eq!(got, expected_e1);
    // A reviewed fact: same fields, and what is the acervo's says so.
    let fact_row = |name: &str, c, o, b, v| (format!("{p}{name}"), c, o, b, v);
    let mut got = table(&found, "F1");
    sort(&mut got);
    let in_fact = |name: &str| got.iter().find(|r| r.0 == format!("{p}{name}")).cloned().unwrap();
    assert_eq!(in_fact(".statement"), fact_row(".statement", C::ReviewedFact, O::Acervo, B::Preparation, V::RecordNotAvailable));
    assert_eq!(in_fact(".authorship"), fact_row(".authorship", C::ReviewedFact, O::Acervo, B::Preparation, V::RecordNotAvailable));
    assert_eq!(in_fact(".source.line"), fact_row(".source.line", C::ReviewedFact, O::Acervo, B::Preparation, V::RecordNotAvailable));
    assert_eq!(in_fact(".source.end_line"), fact_row(".source.end_line", C::ReviewedFact, O::Acervo, B::Preparation, V::RecordNotAvailable));
    assert_eq!(got.len(), expected_e1.len());
    // What belongs to a block is tied to its unit, and every field of a ficha to that ficha's unit.
    let block: Vec<ClassifiedField> = found.classified().into_iter().filter(|f| f.path == "$.evidence.contexts[].lines[].text").collect();
    assert_eq!(block.len(), 1);
    assert_eq!((block[0].unit.as_ref().unwrap().0.as_str(), block[0].class, block[0].rule_version.as_deref(), block[0].provenance), ("CTX_1", C::Code, Some("lines-v1"), V::EditionVerified));
    assert!(found.classified().into_iter().filter(|f| f.ficha.is_some()).all(|f| f.unit.as_ref().is_some_and(|u| u.0 == "CTX_1")));
    // The single form: the same fields under the evidence, with the lines inline.
    let single = prose_provenance(&Case::of(&catalog, vec![target("src/a.rs", "um", 1)], 0), &[]).unwrap();
    let names = |c: QueryFieldClass| single.classified().into_iter().filter(move |f| f.class == c).map(|f| f.path).collect::<Vec<_>>();
    assert!(names(C::Code).contains(&"$.evidence.source.context.lines[].text".to_owned()));
    assert!(names(C::FixedContract).contains(&"$.evidence.source.context.numbering".to_owned()));
    assert!(names(C::FichaDescription).contains(&"$.evidence.statement".to_owned()));
    assert!(single.classified().into_iter().filter(|f| f.path.starts_with("$.evidence.")).all(|f| f.ficha.as_ref().is_some_and(|r| r.0 == "E1") && f.unit.as_ref().is_some_and(|u| u.0 == "E1")));
    // A file head's description is the server's too.
    let head = prose_provenance(&Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "gathered": true, "targets": [target("src/h.rs", "depois", 3)], "headers": ["src/h.rs"] })), &[]).unwrap();
    let statement = head.classified().into_iter().find(|f| f.ficha.as_ref().is_some_and(|r| r.0 == "E2") && f.path.ends_with(".statement")).unwrap();
    assert_eq!((statement.class, statement.origin), (C::FichaDescription, O::Server));
}

#[test]
fn a_field_of_the_prose_form_that_is_not_what_its_origin_says_is_refused() {
    let catalog = edition(&PROJECT);
    let cases: [(&str, Value, &str); 12] = [
        ("fact_id", json!("E9"), "fact_id"),
        ("evidence_id", json!("chain:outra"), "evidence_id"),
        ("statement", json!("Outra."), "statement"),
        ("authorship", json!("Outra."), "authorship"),
        ("source/kind", json!("outra"), "source.kind"),
        ("source/git_commit", json!("abc"), "source.git_commit"),
        ("source/path", json!("src/b.rs"), "source.path"),
        ("source/sha256", json!(text('e')), "source.sha256"),
        ("source/line", json!(2), "source.line"),
        ("source/end_line", json!(2), "source.end_line"),
        ("source/excerpt", json!("outra linha"), "source.excerpt"),
        ("source/context/requested_radius", json!(9), "source.context.requested_radius"),
    ];
    for (place, value, field) in cases {
        let query = Query::Prose(two_units(&catalog));
        let mut changed: Value = serde_json::from_str(&query.text()).unwrap();
        let steps: Vec<String> = ["evidence", "selections", "0"].iter().map(|s| s.to_string()).chain(place.split('/').map(str::to_owned)).collect();
        *at(&mut changed, &steps) = value;
        assert_eq!(provenance_violation(classify_text(&query, changed.to_string())), Violation::FieldDiffers { item: "E1".into(), field: field.into() }, "{place}");
    }
    // The same in the form with one selection.
    let query = Query::Prose(Case::of(&catalog, vec![target("src/a.rs", "um", 1)], 0));
    let mut changed: Value = serde_json::from_str(&query.text()).unwrap();
    changed["evidence"]["statement"] = json!("Outra.");
    assert_eq!(provenance_violation(classify_text(&query, changed.to_string())), Violation::FieldDiffers { item: "E1".into(), field: "statement".into() });
    // A hash of the search that is some other search's.
    let mut other = two_units(&catalog);
    other.search = b"outra busca".to_vec();
    assert_eq!(provenance_violation(prose_provenance(&other, &[])), Violation::FieldDiffers { item: "E1".into(), field: "evidence_id".into() });
}

/// Every form this crate can read: prose in both shapes, with and without its options, and the three
/// claims queries written by the other crate.
enum Query {
    Prose(Case),
    Claims(Fixture),
}

impl Query {
    fn text(&self) -> String {
        match self {
            Query::Prose(case) => case.query.clone(),
            Query::Claims(fixture) => fixture.query.clone(),
        }
    }
}

fn all_queries(catalog: &librarian_ingest::Catalog) -> Vec<Query> {
    vec![
        Query::Prose(Case::of(catalog, vec![target("src/a.rs", "um", 1)], 0)),
        Query::Prose(two_units(catalog)),
        Query::Prose(with_fact(catalog)),
        Query::Prose(Case::prepare(catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "instructions": ["Do projeto."], "targets": [target("src/a.rs", "um", 1)] }))),
        Query::Prose(Case::prepare(catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "targets": [target("src/a.rs", "um", 1), target("src/a.rs", "tres", 8)] }))),
        Query::Claims(fixture(catalog, "fatos")),
        Query::Claims(fixture(catalog, "simples")),
        Query::Claims(fixture(catalog, "compacta")),
    ]
}

/// The classification of a query's text, which is the first thing the construction does with it.
fn classify_text(query: &Query, text: String) -> Result<QueryProvenance, CompositionError> {
    match query {
        Query::Prose(case) => {
            let snapshots = case.inputs();
            let composed = prose(case).unwrap();
            let question = case.query_value()["question"].as_str().unwrap().to_owned();
            build_provenance(&ProvenanceInput { query_json: &text, question: QuestionInput { original: &question, suffixes: &[] }, search: &case.search, composition: &case.composition, composed: &composed, snapshots: &snapshots, fact_records: &[], project_instructions: None })
        }
        Query::Claims(fixture) => {
            let snapshots = fixture.case.inputs();
            let composed = fixture.composed();
            let question: Value = serde_json::from_str(&fixture.query).unwrap();
            let question = question["question"].as_str().unwrap().to_owned();
            build_provenance(&ProvenanceInput { query_json: &text, question: QuestionInput { original: &question, suffixes: &[] }, search: &fixture.case.search, composition: &fixture.case.composition, composed: &composed, snapshots: &snapshots, fact_records: &[], project_instructions: None })
        }
    }
}

#[test]
fn every_field_of_every_real_query_is_classified_and_nothing_else_is() {
    let catalog = edition(&PROJECT);
    for query in all_queries(&catalog) {
        let text = query.text();
        let found = classify_text(&query, text.clone()).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        // The leaves of the real query, by path, against the ones in the classification.
        fn leaves(value: &Value, path: &str, out: &mut std::collections::BTreeSet<String>) {
            match value {
                Value::Object(map) => map.iter().for_each(|(k, v)| leaves(v, &format!("{path}.{k}"), out)),
                Value::Array(list) if !list.is_empty() => list.iter().for_each(|v| leaves(v, &format!("{path}[]"), out)),
                _ => { out.insert(path.to_owned()); }
            }
        }
        let mut real = std::collections::BTreeSet::new();
        leaves(&value, "$", &mut real);
        let classified: std::collections::BTreeSet<String> = found.classified().into_iter().map(|f| f.path.clone()).collect();
        assert_eq!(classified, real);
        // A value is hashed as written, except in the contract, which has one hash for all of it.
        assert!(found.classified().into_iter().all(|f| f.value_sha256.is_some() != (f.class == QueryFieldClass::FixedContract)));
    }
}

#[test]
fn a_new_field_anywhere_in_a_real_query_is_refused_where_it_is() {
    let catalog = edition(&PROJECT);
    for query in all_queries(&catalog) {
        let value: Value = serde_json::from_str(&query.text()).unwrap();
        let mut objects = vec![];
        object_paths(&value, &mut vec![], &mut objects);
        assert!(!objects.is_empty());
        for path in objects {
            let mut changed = value.clone();
            at(&mut changed, &path).as_object_mut().unwrap().insert("campo_novo".into(), json!(1));
            match classify_text(&query, changed.to_string()) {
                Err(CompositionError::Violation(Violation::UnknownField { path: found })) => assert!(found.ends_with(".campo_novo"), "{found}"),
                other => panic!("{path:?}: {other:?}"),
            }
        }
    }
}

/// What the contract lets a query not have, written out: everything else is required.
const MAY_BE_ABSENT: [&str; 12] = ["numbering", "excerpt", "ends_file", "symbols", "limits", "reviewed_facts", "allow_edits", "allow_patch", "precise_location", "strict_text", "project_instructions", "edits"];

#[test]
fn a_field_removed_or_renamed_anywhere_in_a_real_query_is_refused() {
    let catalog = edition(&PROJECT);
    let mut checked = 0;
    for query in all_queries(&catalog) {
        let value: Value = serde_json::from_str(&query.text()).unwrap();
        let mut objects = vec![];
        object_paths(&value, &mut vec![], &mut objects);
        for path in objects {
            let names: Vec<String> = { let mut copy = value.clone(); at(&mut copy, &path).as_object().unwrap().keys().cloned().collect() };
            for name in names {
                let mut renamed = value.clone();
                let object = at(&mut renamed, &path).as_object_mut().unwrap();
                let taken = object.remove(&name).unwrap();
                object.insert(format!("{name}_novo"), taken);
                let refused = matches!(classify_text(&query, renamed.to_string()), Err(CompositionError::Violation(Violation::UnknownField { .. } | Violation::MissingField { .. })));
                assert!(refused, "{path:?}: {name} renamed");
                let mut removed = value.clone();
                at(&mut removed, &path).as_object_mut().unwrap().remove(&name);
                let result = classify_text(&query, removed.to_string());
                // In the second alternative of `location` the schema has only a type; those names are listed too.
                let optional = MAY_BE_ABSENT.contains(&name.as_str()) || (path.iter().any(|s| s == "anyOf") && ["additionalProperties", "required", "properties"].contains(&name.as_str()));
                if !optional {
                    match result {
                        // Without its selections a query reads as the other shape, which then lacks its own fields.
                        Err(CompositionError::Violation(Violation::MissingField { path: found })) => assert!(found.ends_with(&format!(".{name}")) || name == "selections", "{found} for {name}"),
                        other => panic!("{path:?}: {name} removed: {other:?}"),
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 500, "{checked}");
}

#[test]
fn a_query_of_neither_shape_is_refused() {
    let catalog = edition(&PROJECT);
    let query = Query::Prose(two_units(&catalog));
    let mut no_blocks: Value = serde_json::from_str(&query.text()).unwrap();
    no_blocks["evidence"].as_object_mut().unwrap().remove("contexts");
    assert_eq!(provenance_violation(classify_text(&query, no_blocks.to_string())), Violation::MissingField { path: "$.evidence.contexts".into() });
    let query = Query::Prose(Case::of(&catalog, vec![target("src/a.rs", "um", 1)], 0));
    let mut blocks_too: Value = serde_json::from_str(&query.text()).unwrap();
    blocks_too["evidence"]["contexts"] = json!([]);
    assert_eq!(provenance_violation(classify_text(&query, blocks_too.to_string())), Violation::UnknownField { path: "$.evidence.contexts".into() });
}

#[test]
fn the_clients_text_and_what_the_server_added_are_kept_apart() {
    let catalog = edition(&PROJECT);
    let suffix = " Uma resposta anterior foi recusada na conferência mecânica: quote_mismatch.";
    let prepared = format!("Qual a pergunta?{suffix}");
    let case = Case::prepare(&catalog, json!({ "question": prepared, "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "targets": [target("src/a.rs", "um", 1)] }));
    let parts = [SuffixInput { text: suffix, rule: RetryRule::MechanicalRefused, attempt: 1 }];
    let found = prose_provenance_with(&case, &[], QuestionInput { original: "Qual a pergunta?", suffixes: &parts }).unwrap();
    assert_eq!(found.person_text, PersonText { sha256: Sha256Hex::try_from(sha("Qual a pergunta?".as_bytes())).unwrap(), declared_by: ClientDeclared::Client, provenance: ClientTextProvenance::UnverifiedClientText });
    assert_eq!(found.server_suffixes, vec![ServerSuffix { order: 1, sha256: Sha256Hex::try_from(sha(suffix.as_bytes())).unwrap(), declared_by: ServerRetryDeclared::ServerRetry, rule: RetryRule::MechanicalRefused, attempt: 1 }]);
    assert_eq!(found.prepared_question_sha256.as_str(), sha(prepared.as_bytes()));
    assert_ne!(found.person_text.sha256, found.prepared_question_sha256);
    let field = found.classified().into_iter().find(|f| f.path == "$.question").unwrap();
    assert_eq!((field.class, field.origin, field.provenance), (QueryFieldClass::ClientTextWithServerSuffixes, FieldOrigin::Client, FieldProvenance::UnverifiedClientText));
    // The whole text said to be the client's: a question with no suffix, and only that.
    let whole = prose_provenance_with(&case, &[], QuestionInput { original: &prepared, suffixes: &[] }).unwrap();
    assert_eq!(whole.classified().into_iter().find(|f| f.path == "$.question").unwrap().class, QueryFieldClass::ClientText);
    assert_eq!(whole.prepared_question_sha256, found.prepared_question_sha256);
    assert_ne!(whole.person_text, found.person_text);
    // Client's text and server's swapped does not make the prepared question.
    let swapped = [SuffixInput { text: "Qual a pergunta?", rule: RetryRule::MechanicalRefused, attempt: 1 }];
    assert_eq!(provenance_violation(prose_provenance_with(&case, &[], QuestionInput { original: suffix, suffixes: &swapped })), Violation::QuestionDiffers);
    // A suffix that is not in the question, or one counted twice.
    let twice = [parts[0], parts[0]];
    assert_eq!(provenance_violation(prose_provenance_with(&case, &[], QuestionInput { original: "Qual a pergunta?", suffixes: &twice })), Violation::QuestionDiffers);
    assert_eq!(provenance_violation(prose_provenance_with(&case, &[], QuestionInput { original: "Qual a pergunta?", suffixes: &[] })), Violation::QuestionDiffers);
}

#[test]
fn several_suffixes_keep_their_order_rule_and_attempt() {
    let catalog = edition(&PROJECT);
    let (one, two) = (" Primeira recusa.", " Segunda recusa.");
    let prepared = format!("Pergunta.{one}{two}");
    let case = Case::prepare(&catalog, json!({ "question": prepared, "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "targets": [target("src/a.rs", "um", 1)] }));
    let parts = [SuffixInput { text: one, rule: RetryRule::GateRefused, attempt: 1 }, SuffixInput { text: two, rule: RetryRule::MechanicalRefused, attempt: 2 }];
    let found = prose_provenance_with(&case, &[], QuestionInput { original: "Pergunta.", suffixes: &parts }).unwrap();
    assert_eq!(found.server_suffixes.iter().map(|s| (s.order, s.sha256.as_str().to_owned(), s.rule, s.attempt)).collect::<Vec<_>>(), [(1, sha(one.as_bytes()), RetryRule::GateRefused, 1), (2, sha(two.as_bytes()), RetryRule::MechanicalRefused, 2)]);
    let swapped = [parts[1], parts[0]];
    assert_eq!(provenance_violation(prose_provenance_with(&case, &[], QuestionInput { original: "Pergunta.", suffixes: &swapped })), Violation::QuestionDiffers);
    // A record that has them in another order, or with another rule, is not this question's.
    let snapshots = case.inputs();
    let composed = prose(&case).unwrap();
    let input = ProvenanceInput { query_json: &case.query, question: QuestionInput { original: "Pergunta.", suffixes: &parts }, search: &case.search, composition: &case.composition, composed: &composed, snapshots: &snapshots, fact_records: &[], project_instructions: None };
    assert_eq!(check_provenance(&found, &input), Ok(()));
    let mut changed = found.clone();
    changed.server_suffixes.swap(0, 1);
    assert_eq!(check_provenance(&changed, &input), Err(CompositionError::Violation(Violation::ProvenanceDiffers { part: "server_suffixes".into() })));
}

#[test]
fn the_prepared_question_has_to_be_the_one_in_the_query() {
    let catalog = edition(&PROJECT);
    let mut case = two_units(&catalog);
    let good = prose_provenance(&case, &[]).unwrap();
    assert_eq!(good.prepared_question_sha256.as_str(), sha(b"q"));
    with_query(&mut case, |q| q["question"] = json!("outra"));
    assert_eq!(provenance_violation(prose_provenance_with(&case, &[], QuestionInput { original: "q", suffixes: &[] })), Violation::QuestionDiffers);
}

#[test]
fn the_fixed_contract_and_the_projects_instructions_are_two_things() {
    let catalog = edition(&PROJECT);
    let of = |instructions: Value| {
        let mut request = json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "numbered_lines": true, "targets": [target("src/a.rs", "um", 1)] });
        if !instructions.is_null() {
            request["instructions"] = instructions;
        }
        Case::prepare(&catalog, request)
    };
    let (a, b) = (json!(["Do projeto, primeira."]), json!(["Do projeto, outra."]));
    let (with_a, with_b, none) = (prose_provenance(&of(a.clone()), &[]).unwrap(), prose_provenance(&of(b), &[]).unwrap(), prose_provenance(&of(Value::Null), &[]).unwrap());
    assert_eq!(with_a.project_instructions_sha256.as_ref().unwrap().as_str(), sha(a.to_string().as_bytes()));
    assert_ne!(with_a.project_instructions_sha256, with_b.project_instructions_sha256);
    assert_eq!(with_a.fixed_contract_sha256, with_b.fixed_contract_sha256);
    let instruction = |p: &QueryProvenance| { let f = p.classified().into_iter().find(|f| f.path == "$.instructions[]").unwrap(); (f.class, f.origin, f.provenance) };
    assert_eq!(instruction(&with_a), (QueryFieldClass::ProjectInstructions, FieldOrigin::Project, FieldProvenance::ProjectDeclared));
    // Without them the query carries the contract's own, and the hash of the contract says so.
    assert_eq!(none.project_instructions_sha256, None);
    assert_eq!(instruction(&none), (QueryFieldClass::FixedContract, FieldOrigin::Contract, FieldProvenance::SystemFixed));
    assert_ne!(none.fixed_contract_sha256, with_a.fixed_contract_sha256);

    // Claims: the project's instructions in their own field, next to the contract's.
    let fatos = fixture(&catalog, "fatos");
    let declared = vec!["Instrução do projeto.".to_owned()];
    let found = fatos.provenance(&[], Some(&declared)).unwrap();
    assert_eq!(found.project_instructions_sha256.as_ref().unwrap().as_str(), sha(json!(declared).to_string().as_bytes()));
    let field = |path: &str| { let f = found.classified().into_iter().find(|f| f.path == path).unwrap(); (f.class, f.provenance) };
    assert_eq!(field("$.project_instructions[]"), (QueryFieldClass::ProjectInstructions, FieldProvenance::ProjectDeclared));
    assert_eq!(field("$.instructions[]"), (QueryFieldClass::FixedContract, FieldProvenance::SystemFixed));
    assert_eq!(field("$.policy.allow_hypothesis"), (QueryFieldClass::ProjectSettings, FieldProvenance::ProjectDeclared));
    let other = vec!["Outra.".to_owned()];
    assert_eq!(provenance_violation(fatos.provenance(&[], Some(&other))), Violation::ProjectInstructionsDiffer);
    // Declared, and not in the query; and in the query, with nothing to compare it to: told as declared.
    let simples = fixture(&catalog, "simples");
    assert_eq!(simples.provenance(&[], None).unwrap().project_instructions_sha256, None);
    assert_eq!(provenance_violation(simples.provenance(&[], Some(&other))), Violation::ProjectInstructionsDiffer);
    assert_eq!(fatos.provenance(&[], None).unwrap().project_instructions_sha256, found.project_instructions_sha256);
    // Changing the project's text in the query moves one hash and not the other.
    let mut changed = fixture(&catalog, "fatos");
    changed.with_query(|q| q["project_instructions"][0] = json!("Alterada."));
    let moved = changed.provenance(&[], None).unwrap();
    assert_ne!(moved.project_instructions_sha256, found.project_instructions_sha256);
    assert_eq!(moved.fixed_contract_sha256, found.fixed_contract_sha256);
    // And the contract's text moves the other.
    let mut changed = fixture(&catalog, "fatos");
    changed.with_query(|q| q["instructions"][0] = json!("Alterada."));
    let moved = changed.provenance(&[], None).unwrap();
    assert_ne!(moved.fixed_contract_sha256, found.fixed_contract_sha256);
    assert_eq!(moved.project_instructions_sha256, found.project_instructions_sha256);
}

#[test]
fn what_varies_by_ficha_is_outside_the_hash_of_the_contract() {
    let catalog = edition(&PROJECT);
    let of = |targets: Vec<Value>| prose_provenance(&Case::of(&catalog, targets, 0), &[]).unwrap();
    let one = of(vec![target("src/a.rs", "um", 1), target("src/a.rs", "tres", 8)]);
    let other = of(vec![target("src/b.rs", "outra", 1), target("src/d.rs", "ler", 5), target("src/h.rs", "depois", 3)]);
    assert_eq!(one.fixed_contract_sha256, other.fixed_contract_sha256);
    assert_ne!(one.fields, other.fields);
    for name in [".statement", ".authorship", ".source.excerpt", ".source.git_commit", ".source.path", ".source.sha256", ".source.line", ".fact_id", ".evidence_id", ".source.context.requested_radius"] {
        assert!(one.classified().into_iter().filter(|f| f.path.ends_with(name)).all(|f| f.class != QueryFieldClass::FixedContract), "{name}");
        assert!(one.classified().into_iter().any(|f| f.path.ends_with(name)), "{name}");
    }
    // A radius or a statement is not in it; a text of the contract is.
    let radius = prose_provenance(&Case::of(&catalog, vec![target("src/a.rs", "um", 1), target("src/a.rs", "tres", 8)], 1), &[]).unwrap();
    assert_eq!(radius.fixed_contract_sha256, one.fixed_contract_sha256);
    let mut case = two_units(&catalog);
    with_query(&mut case, |q| {
        for selection in q["evidence"]["selections"].as_array_mut().unwrap() {
            selection["scope"] = json!("Outro escopo.");
        }
    });
    assert_ne!(prose_provenance(&case, &[]).unwrap().fixed_contract_sha256, one.fixed_contract_sha256);
    // A "fixed" text that differs between fichas was never fixed.
    for field in ["scope", "metadata_scope", "unknowns_scope"] {
        let mut varying = two_units(&catalog);
        with_query(&mut varying, |q| q["evidence"]["selections"][1][field] = json!("Só nesta ficha."));
        assert_eq!(provenance_violation(prose_provenance(&varying, &[])), Violation::FixedFieldVaries { path: format!("$.evidence.selections[].{field}") });
    }
    let mut varying = two_units(&catalog);
    with_query(&mut varying, |q| q["evidence"]["selections"][1]["source"]["execution_scope"] = json!("Só nesta ficha."));
    assert_eq!(provenance_violation(prose_provenance(&varying, &[])), Violation::FixedFieldVaries { path: "$.evidence.selections[].source.execution_scope".into() });
    let mut varying = two_units(&catalog);
    with_query(&mut varying, |q| q["evidence"]["contexts"][1]["numbering"] = json!("Só neste bloco."));
    assert_eq!(provenance_violation(prose_provenance(&varying, &[])), Violation::FixedFieldVaries { path: "$.evidence.contexts[].numbering".into() });
}

#[test]
fn the_claims_form_is_classified_and_what_it_carries_of_each_unit_is_the_unit() {
    use FieldProvenance as V;
    use QueryFieldClass as C;
    let catalog = edition(&PROJECT);
    let fatos = fixture(&catalog, "fatos");
    let records = [FactRecord { fact_id: "F1", bytes: FACT_RECORD.as_bytes() }];
    let found = fatos.provenance(&records, None).unwrap();
    let field = |path: &str, unit: Option<&str>| found.classified().into_iter().find(|f| f.path == path && (unit.is_none() || f.unit.as_ref().map(|u| u.0.as_str()) == unit)).map(|f| (f.class, f.origin, f.provenance, f.rule_version.clone())).unwrap();
    let lines = Some("lines-v1".to_owned());
    assert_eq!(field("$.evidence.items[].excerpt", Some("CTX_2")), (C::Code, FieldOrigin::Edition, V::EditionVerified, lines.clone()));
    assert_eq!(field("$.evidence.line_map[].lines[].text", Some("CTX_1")), (C::Code, FieldOrigin::Edition, V::EditionVerified, lines));
    assert_eq!(field("$.evidence.items[].symbols[].name", Some("CTX_2")), (C::DerivedFromEdition, FieldOrigin::Edition, V::SystemGenerated, None));
    assert_eq!(field("$.evidence.reviewed_facts[].statement", None), (C::ReviewedFact, FieldOrigin::Acervo, V::AcervoQuotesChecked, None));
    assert_eq!(field("$.response_schema.type", None), (C::FixedContract, FieldOrigin::Contract, V::SystemFixed, None));
    // The notes came twice, and the form keeps one; so does its record, with the client's class.
    let last = found.limits.last().unwrap();
    assert_eq!((last.producer, last.class, last.provenance, last.text_sha256.as_str()), (LimitProducer::RequestNotes, LimitClass::ClientText, LimitProvenance::UnverifiedClientText, sha("Uma nota.".as_bytes()).as_str()));
    assert_eq!(found.limits.iter().filter(|l| l.producer == LimitProducer::RequestNotes).count(), 1);
    assert_eq!(field("$.evidence.limits[]", None), (C::Limit, FieldOrigin::Client, V::UnverifiedClientText, None));
    assert_eq!(fatos.case.composition.limits.iter().filter(|l| l.producer == LimitProducer::RequestNotes).count(), 2);
    assert_eq!(found.reviewed_facts[0].unit, UnitId("CTX_2".into()));
    assert_eq!(found.sha256.as_str(), sha(fatos.query.as_bytes()));
    assert_eq!(found.code.units, [UnitId("CTX_1".into()), UnitId("CTX_2".into())]);
    // Each field of a fact belongs to that fact, and says how far it could be told.
    let statement = found.classified().into_iter().find(|f| f.path == "$.evidence.reviewed_facts[].statement").unwrap();
    assert_eq!(statement.ficha, Some(FichaRef("F1".into())));
    let without = fatos.provenance(&[], None).unwrap();
    let statement = without.classified().into_iter().find(|f| f.path == "$.evidence.reviewed_facts[].statement").unwrap();
    assert_eq!((statement.provenance, without.reviewed_facts[0].provenance), (V::RecordNotAvailable, ReviewedFactProvenance::RecordNotAvailable));
    // Compact: no excerpt in the query, and the rest still holds. English: the same fields.
    let compact = fixture(&catalog, "compacta");
    let classified = compact.provenance(&[], None).unwrap();
    assert!(!classified.classified().into_iter().any(|f| f.path == "$.evidence.items[].excerpt"));
    // Each thing the form says of a unit, changed in the query.
    type Change = fn(&mut Value);
    let refused: [(&str, Change, &str, &str); 8] = [
        ("id", |q| q["evidence"]["items"][0]["id"] = json!("CTX_9"), "CTX_1", "id"),
        ("path", |q| q["evidence"]["items"][0]["path"] = json!("src/b.rs"), "CTX_1", "path"),
        ("start", |q| q["evidence"]["items"][0]["start_line"] = json!(2), "CTX_1", "start_line"),
        ("end", |q| q["evidence"]["items"][0]["end_line"] = json!(2), "CTX_1", "end_line"),
        ("excerpt", |q| q["evidence"]["items"][0]["excerpt"] = json!("outra"), "CTX_1", "excerpt"),
        ("map id", |q| q["evidence"]["line_map"][0]["evidence_id"] = json!("CTX_9"), "CTX_1", "evidence_id"),
        ("map text", |q| q["evidence"]["line_map"][0]["lines"][0]["text"] = json!("outra"), "CTX_1", "line_map"),
        ("map number", |q| q["evidence"]["line_map"][0]["lines"][0]["number"] = json!(9), "CTX_1", "line_map"),
    ];
    for (name, change, item, field) in refused {
        let mut changed = fixture(&catalog, "fatos");
        changed.with_query(change);
        assert_eq!(provenance_violation(changed.provenance(&[], None)), Violation::FieldDiffers { item: item.into(), field: field.into() }, "{name}");
    }
    for (field, value) in [("id", json!("F9")), ("statement", json!("Outra.")), ("authorship", json!("Outra.")), ("path", json!("src/a.rs")), ("start_line", json!(9)), ("end_line", json!(9))] {
        let mut changed = fixture(&catalog, "fatos");
        changed.with_query(|q| q["evidence"]["reviewed_facts"][0][field] = value);
        assert_eq!(provenance_violation(changed.provenance(&[], None)), Violation::FieldDiffers { item: "F1".into(), field: field.into() });
    }
    let mut no_fact = fixture(&catalog, "fatos");
    no_fact.with_query(|q| { q["evidence"].as_object_mut().unwrap().remove("reviewed_facts"); });
    assert_eq!(provenance_violation(no_fact.provenance(&[], None)), Violation::FieldDiffers { item: "reviewed_facts".into(), field: "length".into() });
    let mut limit = fixture(&catalog, "fatos");
    limit.with_query(|q| q["evidence"]["limits"][0] = json!("Outro."));
    assert_eq!(provenance_violation(limit.provenance(&[], None)), Violation::LimitTextDiffers { index: 0 });
    let mut extra = fixture(&catalog, "fatos");
    extra.with_query(|q| q["evidence"]["limits"].as_array_mut().unwrap().push(json!("Sem origem.")));
    let lines_there = fatos.provenance(&[], None).unwrap().limits.len();
    assert_eq!(provenance_violation(extra.provenance(&[], None)), Violation::LimitWithoutOrigin { index: lines_there });
}

#[test]
fn the_same_inputs_give_the_same_provenance() {
    let make = || {
        let catalog = edition(&PROJECT);
        let case = with_fact(&catalog);
        let records = [FactRecord { fact_id: "F1", bytes: FACT_RECORD.as_bytes() }];
        let found = prose_provenance(&case, &records).unwrap();
        let again = prose_provenance(&case, &records).unwrap();
        let fatos = fixture(&catalog, "fatos").provenance(&records, None).unwrap();
        (serde_json::to_vec(&found).unwrap(), serde_json::to_vec(&again).unwrap(), serde_json::to_vec(&fatos).unwrap())
    };
    let (one, two, claims) = make();
    assert_eq!(one, two);
    assert_eq!(make(), (one, two, claims));
}

#[test]
fn a_real_provenance_has_no_word_of_truth_sufficiency_quality_or_human_action() {
    let catalog = edition(&PROJECT);
    let records = [FactRecord { fact_id: "F1", bytes: FACT_RECORD.as_bytes() }];
    let fatos = fixture(&catalog, "fatos").provenance(&records, None).unwrap();
    let prosa = prose_provenance(&with_fact(&catalog), &records).unwrap();
    for found in [fatos, prosa] {
        let value = serde_json::to_value(&found).unwrap();
        let mut names = vec![];
        keys(&value, &mut names);
        // The names of the fields and every value of a closed list a field can take.
        for field in &found.classified() {
            for name in [json!(field.class), json!(field.origin), json!(field.producer), json!(field.provenance)] {
                names.push(name.as_str().unwrap().to_owned());
            }
        }
        for reference in &found.reviewed_facts {
            names.push(json!(reference.provenance).as_str().unwrap().to_owned());
        }
        for word in ["true", "truth", "verdad", "verdade", "suffic", "suficien", "quality", "qualidade", "score", "relevan", "confidence", "approved", "aprovad", "human", "humano", "reviewed_by", "seen_by", "clicked", "weight"] {
            assert!(!names.iter().any(|n| n.contains(word)), "{word} in the names");
        }
    }
}

#[test]
fn the_closed_lists_of_the_query_have_exactly_the_written_values() {
    for (value, name) in [
        (QueryFieldClass::Structure, "structure"), (QueryFieldClass::Code, "code"), (QueryFieldClass::SourceMetadata, "source_metadata"), (QueryFieldClass::FichaDescription, "ficha_description"),
        (QueryFieldClass::ReviewedFact, "reviewed_fact"), (QueryFieldClass::Limit, "limit"), (QueryFieldClass::ClientText, "client_text"), (QueryFieldClass::ClientTextWithServerSuffixes, "client_text_with_server_suffixes"),
        (QueryFieldClass::FixedContract, "fixed_contract"), (QueryFieldClass::ProjectInstructions, "project_instructions"), (QueryFieldClass::ProjectSettings, "project_settings"), (QueryFieldClass::RequestSetting, "request_setting"),
        (QueryFieldClass::DerivedFromEdition, "derived_from_edition"), (QueryFieldClass::QueryIdentity, "query_identity"),
    ] {
        pinned(value, json!(name));
    }
    for (value, name) in [(FieldOrigin::Edition, "edition"), (FieldOrigin::Acervo, "acervo"), (FieldOrigin::Server, "server"), (FieldOrigin::Project, "project"), (FieldOrigin::Request, "request"), (FieldOrigin::Client, "client"), (FieldOrigin::Contract, "contract")] {
        pinned(value, json!(name));
    }
    for (value, name) in [(FieldProducer::GraphEngine, "graph_engine"), (FieldProducer::Preparation, "preparation"), (FieldProducer::ClaimsContext, "claims_context"), (FieldProducer::ClaimsEngine, "claims_engine"), (FieldProducer::LimitEntries, "limit_entries"), (FieldProducer::Client, "client")] {
        pinned(value, json!(name));
    }
    for (value, name) in [
        (FieldProvenance::EditionVerified, "edition_verified"), (FieldProvenance::AcervoQuotesChecked, "acervo_quotes_checked"), (FieldProvenance::RecordNotAvailable, "record_not_available"), (FieldProvenance::SystemFixed, "system_fixed"),
        (FieldProvenance::SystemGenerated, "system_generated"), (FieldProvenance::RequestRecorded, "request_recorded"), (FieldProvenance::ProjectDeclared, "project_declared"), (FieldProvenance::UnverifiedClientText, "unverified_client_text"), (FieldProvenance::NotAvailable, "not_available"),
    ] {
        pinned(value, json!(name));
    }
    pinned(ReviewedFactProvenance::RecordNotAvailable, json!("record_not_available"));
}

// The record of a suggestion, in its own terms. Its golden is apart from every other one, and the
// selection in it is a real one: a change in what `envelope::select` decides for the same input shows
// here, and is a new `envelope::RULES_VERSION`.

const SUGGESTION_GOLDEN: &str = include_str!("evidence-fixtures/suggestion-record.golden.json");
const SUGGESTION_GOLDEN_SHA256: &str = "7eb721517ba77e27fec96e4264810c2d75da2e28d7f589e8acf128c002fc41a5";

/// A record of a real selection, through the same steps the route takes up to its answer.
fn suggestion_record() -> SuggestionRecord {
    use librarian_ingest::{envelope, inventory};
    let catalog = edition(&PROJECT);
    let nodes: Vec<String> = catalog.symbols().values().filter(|s| s.name == "ler").map(|s| s.id.clone()).collect();
    let candidates = envelope::candidates(&catalog, &nodes, None).unwrap();
    let selection = envelope::select(&catalog, &candidates, None, envelope::BUDGET_BYTES, 16);
    let mut answer = serde_json::to_value(&selection).unwrap();
    answer["coverage"] = Value::Null;
    let rules = inventory::builtin().rules.iter().map(|rule| InventoryRuleEntry { origin: InventoryRuleOrigin::Builtin, rule: serde_json::to_value(rule).unwrap() }).collect();
    SuggestionRecord::new(
        format!("edition:{}", text('1')),
        envelope::RULES_VERSION,
        SuggestionInput { nodes, budget_bytes: envelope::BUDGET_BYTES, max_items: 16 },
        SuggestionInventoryRules::new(inventory::VERSION, rules).unwrap(),
        SuggestionMeasure::NoMeasure {},
        &answer,
    )
    .unwrap()
}

#[test]
fn the_golden_of_a_suggestion_record_is_reproduced_to_the_byte() {
    let bytes = suggestion_record().to_bytes().unwrap();
    // A checkout may write this file with other line ends; the record has none but LF.
    assert_eq!(String::from_utf8(bytes.clone()).unwrap(), SUGGESTION_GOLDEN.replace("\r\n", "\n"));
    assert_eq!(librarian_ingest::hash(&bytes), SUGGESTION_GOLDEN_SHA256);
    assert_eq!(librarian_ingest::envelope::RULES_VERSION, "envelope-1");
    // Twice, from a new edition each time, the same bytes.
    assert_eq!(suggestion_record().to_bytes().unwrap(), bytes);
    assert!(!String::from_utf8_lossy(&bytes).contains(SUGGESTION_GOLDEN_SHA256));
}

#[test]
fn a_suggestion_record_has_exactly_the_written_names() {
    let record = serde_json::json!({
        "kind": "suggestion_record",
        "format_version": 1,
        "edition_origin": "edition:x",
        "rules_version": "envelope-1",
        "input": { "nodes": ["a"], "budget_bytes": 6144, "max_items": 16 },
        "inventory_rules": { "version": "v", "sha256": text('a'), "rules": [{ "origin": "builtin", "rule": { "id": "r" } }, { "origin": "project", "rule": {} }] },
        "measure": { "read_status": "ok", "execution": "01-x", "execution_sha256": text('b') },
        "output": { "items": [{ "id": "a" }], "left_out": [{ "id": "b" }], "gaps": [{ "code": "c" }], "behavior": [{ "kind": "k" }], "measured": true, "used_bytes": 7, "coverage": { "total": 1 }, "limits": ["l"] },
    });
    pinned(serde_json::from_value::<SuggestionRecord>(record.clone()).unwrap(), record);
    pinned(SuggestionMeasure::NoMeasure {}, json!({ "read_status": "none" }));
    pinned(SuggestionMeasure::Unreadable {}, json!({ "read_status": "unreadable" }));
    pinned(SuggestionMeasure::Ok { execution: "01-x".into(), execution_sha256: hash('a') }, json!({ "read_status": "ok", "execution": "01-x", "execution_sha256": text('a') }));
    pinned(SuggestionRecordKind::SuggestionRecord, json!("suggestion_record"));
    pinned(SuggestionRecordVersion, json!(1));
    pinned(InventoryRuleOrigin::Builtin, json!("builtin"));
    pinned(InventoryRuleOrigin::Project, json!("project"));
}

#[test]
fn a_suggestion_record_refuses_what_it_does_not_know_and_requires_all_it_does() {
    let value = serde_json::to_value(suggestion_record()).unwrap();
    // The record's own objects: what a list of the output holds keeps the form the selection gives it.
    let own: [&[&str]; 4] = [&[], &["input"], &["inventory_rules"], &["output"]];
    let read = |v: Value| serde_json::from_value::<SuggestionRecord>(v);
    for place in own {
        let path: Vec<String> = place.iter().map(|s| s.to_string()).collect();
        let mut more = value.clone();
        at(&mut more, &path).as_object_mut().unwrap().insert("campo_novo".into(), json!(1));
        assert!(read(more).is_err(), "{place:?} took an unknown field");
        let names: Vec<String> = at(&mut value.clone(), &path).as_object().unwrap().keys().cloned().collect();
        assert!(!names.is_empty());
        for name in names {
            let mut less = value.clone();
            at(&mut less, &path).as_object_mut().unwrap().remove(&name);
            assert!(read(less).is_err(), "{place:?} went without {name}");
        }
    }
    let mut wrong_kind = value.clone();
    wrong_kind["kind"] = json!("evidence_package");
    assert!(read(wrong_kind).is_err());
    let mut wrong_version = value.clone();
    wrong_version["format_version"] = json!(2);
    assert!(read(wrong_version).is_err());
    // The states of a measure do not mix: no execution where there is none, and none missing.
    for (bad, name) in [(json!({ "read_status": "none", "execution": "01-x" }), "none"), (json!({ "read_status": "unreadable", "execution": "01-x", "execution_sha256": text('a') }), "unreadable"), (json!({ "read_status": "ok" }), "ok"), (json!({ "read_status": "ok", "execution": "01-x" }), "ok"), (json!({ "read_status": "outro" }), "outro"), (json!({}), "vazio")] {
        assert!(serde_json::from_value::<SuggestionMeasure>(bad).is_err(), "{name}");
    }
    assert_ne!(serde_json::to_value(SuggestionMeasure::NoMeasure {}).unwrap(), serde_json::to_value(SuggestionMeasure::Unreadable {}).unwrap());
}

/// Every part of a record, changed. Each change is another record, with another address.
#[test]
fn any_change_in_a_record_is_another_address() {
    let good = suggestion_record();
    let of = |record: &SuggestionRecord| librarian_ingest::hash(&record.to_bytes().unwrap());
    assert_ne!(good.output.items.len(), 0);
    assert!(good.output.items.len() >= 2, "order needs two items");
    type Change = Box<dyn Fn(&mut SuggestionRecord)>;
    let changes: Vec<(&str, Change)> = vec![
        ("edition", Box::new(|r| r.edition_origin = "edition:outra".into())),
        ("rules_version", Box::new(|r| r.rules_version = RulesVersion("envelope-2".into()))),
        ("nodes", Box::new(|r| r.input.nodes.push("outro".into()))),
        ("budget", Box::new(|r| r.input.budget_bytes += 1)),
        ("max_items", Box::new(|r| r.input.max_items += 1)),
        ("rules hash", Box::new(|r| r.inventory_rules.sha256 = hash('e'))),
        ("rules", Box::new(|r| { r.inventory_rules.rules.pop(); })),
        ("rule order", Box::new(|r| r.inventory_rules.rules.swap(0, 1))),
        ("rules version", Box::new(|r| r.inventory_rules.version = "outra".into())),
        ("measure none", Box::new(|r| r.measure = SuggestionMeasure::Unreadable {})),
        ("measure ok", Box::new(|r| r.measure = SuggestionMeasure::Ok { execution: "01-x".into(), execution_sha256: hash('a') })),
        ("items", Box::new(|r| { r.output.items.pop(); })),
        ("item order", Box::new(|r| r.output.items.swap(0, 1))),
        ("item content", Box::new(|r| r.output.items[0]["role"] = json!("outro"))),
        ("left_out", Box::new(|r| r.output.left_out.push(json!({ "id": "x" })))),
        ("gaps", Box::new(|r| r.output.gaps.push(json!({ "code": "x" })))),
        ("behavior", Box::new(|r| r.output.behavior.push(json!({ "kind": "x" })))),
        ("measured", Box::new(|r| r.output.measured = !r.output.measured)),
        ("used_bytes", Box::new(|r| r.output.used_bytes += 1)),
        ("coverage", Box::new(|r| r.output.coverage = json!({ "total": 1 }))),
        ("limits", Box::new(|r| r.output.limits.push("x".into()))),
    ];
    let mut seen = std::collections::BTreeSet::new();
    seen.insert(of(&good));
    for (name, change) in &changes {
        let mut record = good.clone();
        change(&mut record);
        assert!(seen.insert(of(&record)), "{name} kept an address");
        // Written over the good file, it is not read as the record that name says.
        let bytes = record.to_bytes().unwrap();
        assert!(matches!(SuggestionRecord::from_bytes(&bytes, &of(&good)), Err(SuggestionRecordError::HashDiffers { .. })), "{name}");
    }
    // Another execution's hash, in a measure that already had one.
    let mut one = good.clone();
    one.measure = SuggestionMeasure::Ok { execution: "01-x".into(), execution_sha256: hash('a') };
    let mut two = one.clone();
    two.measure = SuggestionMeasure::Ok { execution: "01-x".into(), execution_sha256: hash('b') };
    assert_ne!(of(&one), of(&two));
}

#[test]
fn bytes_are_taken_for_a_record_only_if_they_are_the_ones_its_name_says() {
    let good = suggestion_record();
    let bytes = good.to_bytes().unwrap();
    let sha = librarian_ingest::hash(&bytes);
    assert_eq!(SuggestionRecord::from_bytes(&bytes, &sha), Ok(good.clone()));
    // Other bytes under this name.
    let mut changed = bytes.clone();
    changed[10] ^= 1;
    assert!(matches!(SuggestionRecord::from_bytes(&changed, &sha), Err(SuggestionRecordError::HashDiffers { .. })));
    // The same record in other whitespace, under its own good hash, is not how a record is written.
    let compact = serde_json::to_vec(&good).unwrap();
    assert_eq!(SuggestionRecord::from_bytes(&compact, &librarian_ingest::hash(&compact)), Err(SuggestionRecordError::NotInItsOwnBytes));
    // A file that was never a record, under its own good hash.
    assert!(matches!(SuggestionRecord::from_bytes(b"{}\n", &librarian_ingest::hash(b"{}\n")), Err(SuggestionRecordError::Malformed(_))));
    // Rules changed and the hash of the file made good again: the rules' own hash says it.
    let mut rules = good.clone();
    rules.inventory_rules.rules.pop();
    let rewritten = rules.to_bytes().unwrap();
    assert_eq!(SuggestionRecord::from_bytes(&rewritten, &librarian_ingest::hash(&rewritten)), Err(SuggestionRecordError::RulesHashDiffers));
}

#[test]
fn a_record_is_only_made_from_an_answer_whose_fields_it_knows() {
    let rules = || SuggestionInventoryRules::new("v", vec![]).unwrap();
    let input = || SuggestionInput { nodes: vec![], budget_bytes: 10, max_items: 1 };
    let answer = json!({ "items": [], "left_out": [], "gaps": [], "behavior": [], "notes": [{ "text": "para o modelo" }], "budget_bytes": 10, "used_bytes": 0, "measured": false, "limits": [], "coverage": null });
    let make = |answer: &Value| SuggestionRecord::new("edition:x".into(), "r", input(), rules(), SuggestionMeasure::NoMeasure {}, answer);
    let made = make(&answer).unwrap();
    assert!(!String::from_utf8(made.to_bytes().unwrap()).unwrap().contains("para o modelo"));
    // A budget that is not the input's is not this suggestion.
    let mut other_budget = answer.clone();
    other_budget["budget_bytes"] = json!(11);
    assert!(make(&other_budget).is_err());
    // Each field it requires, and one it does not know.
    for name in ["items", "left_out", "gaps", "behavior", "used_bytes", "measured", "limits", "coverage", "budget_bytes"] {
        let mut less = answer.clone();
        less.as_object_mut().unwrap().remove(name);
        assert!(make(&less).is_err(), "{name}");
    }
    let mut more = answer.clone();
    more["outro"] = json!(1);
    assert!(make(&more).is_err());
}

#[test]
fn grouping_the_classification_keeps_every_field_and_place() {
    let catalog = edition(&PROJECT);
    for query in all_queries(&catalog) {
        let found = classify_text(&query, query.text()).unwrap();
        let expanded = found.classified();
        // Grouping again gives the same groups, and no two groups share a classification.
        assert_eq!(group_fields(expanded.clone()), found.fields);
        for (n, a) in found.fields.iter().enumerate() {
            for b in &found.fields[..n] {
                assert!(!(a.path == b.path && a.class == b.class && a.origin == b.origin && a.producer == b.producer && a.rule_version == b.rule_version && a.provenance == b.provenance));
            }
        }
        assert_eq!(expanded.len(), found.fields.iter().map(|g| g.places.len()).sum::<usize>());
    }
    // Interleaved fields come back, each with its own place and value.
    let field = |path: &str, ficha: &str, value: char| ClassifiedField {
        path: path.into(), class: QueryFieldClass::Code, origin: FieldOrigin::Edition, producer: FieldProducer::GraphEngine, rule_version: None,
        provenance: FieldProvenance::EditionVerified, ficha: Some(FichaRef(ficha.into())), unit: None, value_sha256: Some(hash(value)),
    };
    let list = vec![field("$.a", "E1", '1'), field("$.b", "E1", '2'), field("$.a", "E2", '3'), field("$.b", "E2", '4')];
    let groups = group_fields(list.clone());
    assert_eq!(groups.len(), 2);
    let mut back = expand_fields(&groups);
    let mut sorted = list;
    let order = |f: &ClassifiedField| (f.path.clone(), f.ficha.clone());
    back.sort_by_key(order);
    sorted.sort_by_key(order);
    assert_eq!(back, sorted);
    // A field that differs in anything of its classification is its own group.
    let mut other = field("$.a", "E3", '5');
    other.provenance = FieldProvenance::NotAvailable;
    assert_eq!(group_fields(vec![field("$.a", "E1", '1'), other]).len(), 2);
}

#[test]
fn in_prose_the_requests_own_instructions_have_to_be_the_ones_the_query_carries() {
    let catalog = edition(&PROJECT);
    let mut case = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "instructions": ["Do projeto."], "targets": [target("src/a.rs", "um", 1)] }));
    assert!(prose_provenance(&case, &[]).is_ok());
    // The request said other instructions than the query has: whoever calls, it is refused.
    case.composition.instructions = Some(vec!["Outra.".into()]);
    assert_eq!(provenance_violation(prose_provenance(&case, &[])), Violation::ProjectInstructionsDiffer);
    let mut query_changed = Case::prepare(&catalog, json!({ "question": "q", "top": 1, "depth": 0, "hits": 1, "instructions": ["Do projeto."], "targets": [target("src/a.rs", "um", 1)] }));
    with_query(&mut query_changed, |q| q["instructions"][0] = json!("Mudada."));
    assert_eq!(provenance_violation(prose_provenance(&query_changed, &[])), Violation::ProjectInstructionsDiffer);
}

// The selection, format 2. The scenario below is what the page sends after these actions, written
// out by hand from them and not by the code under test: `a` and `m` gathered by hand, `m` unmarked as
// a target; a suggestion from `a` (S1); its items joined; `m` taken out again; `b` marked as a target;
// a suggestion from `a` and `b` (S2); its new item joined.

fn reason(code: &str, for_target: &str, lines: &[usize]) -> Value {
    let mut r = json!({ "code": code, "for_target": for_target });
    if !lines.is_empty() {
        r["lines"] = json!(lines);
    }
    r
}

/// An item as the route serves it (`envelope::Entry`, with `facts` and nothing for what is empty).
#[allow(clippy::too_many_arguments)]
fn served(id: &str, name: &str, path: &str, kind: &str, lines: (usize, usize), role: &str, bytes: usize, reasons: Vec<Value>, calls: Option<f64>, left_out: Option<&str>, facts: &[&str]) -> Value {
    let mut item = json!({ "id": id, "name": name, "path": path, "kind": kind, "start_line": lines.0, "end_line": lines.1, "role": role, "bytes": bytes, "reasons": reasons, "facts": facts });
    if let Some(calls) = calls {
        item["calls"] = json!(calls);
    }
    if let Some(why) = left_out {
        item["left_out"] = json!(why);
        // What the route adds for the coverage, which the page does not read.
        item["coverage_state"] = json!("no_coverage");
    }
    item
}

fn gap_one() -> Value {
    json!({ "code": "homonym_call", "for_target": "A::run", "text": "x", "path": "src/a.rs", "line": 12, "candidates": ["X1", "X2"], "count": 2 })
}

fn gap_two() -> Value {
    json!({ "code": "field_without_writer", "for_target": "B", "text": "f" })
}

fn suggestion(nodes: &[&str], measure: Value, items: Vec<Value>, left_out: Vec<Value>, gaps: Vec<Value>, measured: bool, used: usize) -> SuggestionRecord {
    serde_json::from_value(json!({
        "kind": "suggestion_record", "format_version": 1, "edition_origin": format!("edition:{}", text('1')), "rules_version": "envelope-1",
        "input": { "nodes": nodes, "budget_bytes": 6144, "max_items": 16 },
        "inventory_rules": { "version": "inventory-1", "sha256": text('2'), "rules": [] },
        "measure": measure,
        "output": { "items": items, "left_out": left_out, "gaps": gaps, "behavior": [], "measured": measured, "used_bytes": used, "coverage": null, "limits": [] },
    }))
    .unwrap()
}

fn s1() -> SuggestionRecord {
    suggestion(&["a"], json!({ "read_status": "none" }), vec![
        served("a", "A::run", "src/a.rs", "method", (10, 20), "target", 100, vec![reason("chosen", "A::run", &[])], None, None, &[]),
        served("b", "B", "src/b.rs", "struct", (1, 5), "origin", 50, vec![json!({ "code": "owner_type", "for_target": "A::run", "fields": ["x"] })], None, None, &[]),
        served("header:src/a.rs", "src/a.rs", "src/a.rs", "header", (1, 3), "support", 40, vec![reason("header", "A::run", &[])], None, None, &[]),
    ], vec![
        served("c", "c_fn", "src/c.rs", "function", (30, 40), "support", 70, vec![reason("called", "A::run", &[12])], None, Some("budget"), &[]),
        served("d", "d_test", "tests/d.rs", "function", (1, 9), "constraint", 30, vec![reason("test_caller", "A::run", &[3])], None, Some("test"), &[]),
        served("e", "e_fn", "src/e.rs", "function", (1, 3), "support", 20, vec![reason("caller", "A::run", &[5])], None, Some("max_items"), &[]),
    ], vec![gap_one()], false, 190)
}

fn s2() -> SuggestionRecord {
    suggestion(&["a", "b"], json!({ "read_status": "ok", "execution": "01-medir", "execution_sha256": text('e') }), vec![
        served("a", "A::run", "src/a.rs", "method", (10, 20), "target", 100, vec![reason("chosen", "A::run", &[])], Some(5.0), None, &[]),
        served("b", "B", "src/b.rs", "struct", (1, 5), "target", 50, vec![reason("chosen", "B", &[])], Some(2.0), None, &["F1"]),
        served("c", "c_fn", "src/c.rs", "function", (30, 40), "cost", 70, vec![reason("called_measured", "A::run", &[14])], Some(9.0), None, &[]),
    ], vec![
        served("d", "d_test", "tests/d.rs", "function", (1, 9), "constraint", 30, vec![reason("test_caller", "A::run", &[3])], None, Some("test"), &[]),
    ], vec![gap_one(), gap_two()], true, 220)
}

fn selection_edition() -> SelectionEdition {
    let mut edition = SelectionEdition::default();
    for (id, name, path, kind, start, end, bytes) in [("a", "A::run", "src/a.rs", "method", 10, 20, 100), ("b", "B", "src/b.rs", "struct", 1, 5, 50),
        ("c", "c_fn", "src/c.rs", "function", 30, 40, 70), ("d", "d_test", "tests/d.rs", "function", 1, 9, 30), ("e", "e_fn", "src/e.rs", "function", 1, 3, 20),
        ("header:src/a.rs", "src/a.rs", "src/a.rs", "header", 1, 3, 40), ("m", "M", "src/m.rs", "function", 1, 2, 10)] {
        edition.sources.insert(id.into(), EditionSource { name: name.into(), path: path.into(), kind: kind.into(), start_line: start, end_line: end, bytes });
    }
    for (id, label, kind) in [("a", "run", "method"), ("b", "B", "struct"), ("m", "m_label", "function")] {
        edition.labels.insert(id.into(), (label.into(), kind.into()));
    }
    edition
}

#[allow(clippy::too_many_arguments)]
fn entry(id: &str, name: &str, path: Option<&str>, kind: &str, lines: Option<(usize, usize)>, role: Option<&str>, bytes: Option<usize>, calls: Option<f64>,
    reasons: Vec<Value>, facts: &[&str], left_out_why: Option<&str>, provenance: &str, decision: Option<(&str, &str)>) -> Value {
    json!({ "id": id, "name": name, "path": path, "kind": kind, "start_line": lines.map(|l| l.0), "end_line": lines.map(|l| l.1), "role": role, "bytes": bytes,
        "calls": calls, "reasons": reasons, "facts": facts, "left_out_why": left_out_why, "provenance": provenance,
        "decision": decision.map(|d| d.0), "decision_scope": decision.map(|d| d.1) })
}

fn s1_sha() -> String {
    text('3')
}

fn s2_sha() -> String {
    text('4')
}

/// The selection the page sends after the actions above, field by field.
fn selection_sent() -> Value {
    let mut d = entry("d", "d_test", Some("tests/d.rs"), "function", Some((1, 9)), Some("constraint"), Some(30), None, vec![reason("test_caller", "A::run", &[3])], &[], Some("test"), "left_out", None);
    d["why"] = json!("test");
    let mut e = entry("e", "e_fn", Some("src/e.rs"), "function", Some((1, 3)), Some("support"), Some(20), None, vec![reason("caller", "A::run", &[5])], &[], Some("max_items"), "left_out", None);
    e["why"] = json!("max_items");
    json!({
        "kind": "selection_record", "format_version": 2, "suggestions": [s1_sha(), s2_sha()], "seeds": ["a", "b"], "suggested_from": ["a", "b"],
        "measured": true, "budget_bytes": 6144, "suggested_bytes": 220, "included_bytes": 260,
        "included": [
            entry("a", "A::run", Some("src/a.rs"), "method", Some((10, 20)), Some("target"), Some(100), Some(5.0), vec![reason("chosen", "A::run", &[])], &[], None, "manual", Some(("included", "single"))),
            entry("b", "B", Some("src/b.rs"), "struct", Some((1, 5)), Some("target"), Some(50), Some(2.0),
                vec![json!({ "code": "owner_type", "for_target": "A::run", "fields": ["x"] }), reason("chosen", "B", &[])], &["F1"], None, "suggested", Some(("included", "bulk"))),
            entry("header:src/a.rs", "src/a.rs", Some("src/a.rs"), "header", Some((1, 3)), Some("support"), Some(40), None, vec![reason("header", "A::run", &[])], &[], None, "suggested", Some(("included", "bulk"))),
            entry("c", "c_fn", Some("src/c.rs"), "function", Some((30, 40)), Some("cost"), Some(70), Some(9.0),
                vec![reason("called", "A::run", &[12]), reason("called_measured", "A::run", &[14])], &[], Some("budget"), "left_out", Some(("included", "bulk"))),
        ],
        "removed": [json!({ "id": "m", "name": "m_label", "path": null, "kind": "function", "start_line": null, "end_line": null, "role": null, "bytes": null, "calls": null,
            "reasons": [], "facts": [], "left_out_why": null, "provenance": "manual", "decision": "removed", "decision_scope": "single" })],
        "left_out": [d, e],
        "not_joined": [],
        "gaps": [gap_one(), gap_two()],
    })
}

const SENT_NODES: [&str; 4] = ["a", "b", "header:src/a.rs", "c"];

fn check_sent(value: &Value, nodes: &[&str], records: &[SuggestionRecord]) -> Result<SelectionInput, SelectionRefusal> {
    let input = SelectionInput::from_value(value)?;
    let nodes: Vec<String> = nodes.iter().map(|n| n.to_string()).collect();
    let cited: Vec<&SuggestionRecord> = records.iter().collect();
    check_selection(&input, &nodes, &cited, &selection_edition())?;
    Ok(input)
}

fn refusal(value: &Value, nodes: &[&str]) -> String {
    match check_sent(value, nodes, &[s1(), s2()]) {
        Ok(_) => String::new(),
        Err(why) => why.to_string(),
    }
}

/// The selection changed by `edit`, refused with a reason that starts with `why` (accepted, if empty).
fn refused_as(edit: impl FnOnce(&mut Value), why: &str) {
    let mut value = selection_sent();
    edit(&mut value);
    let found = refusal(&value, &SENT_NODES);
    assert!(if why.is_empty() { found.is_empty() } else { found.starts_with(why) }, "expected «{why}», found «{found}»");
}

#[test]
fn the_pages_selection_after_two_suggestions_is_accepted() {
    assert_eq!(refusal(&selection_sent(), &SENT_NODES), "");
    // The JSON number the page writes for a count is the same number.
    let mut whole = selection_sent();
    whole["included"][0]["calls"] = json!(5);
    assert_eq!(refusal(&whole, &SENT_NODES), "");
}

#[test]
fn a_selection_has_exactly_the_written_names_and_codes() {
    let input = SelectionInput::from_value(&selection_sent()).unwrap();
    let value = serde_json::to_value(&input).unwrap();
    let names = |v: &Value| v.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    assert_eq!(names(&value), ["budget_bytes", "format_version", "gaps", "included", "included_bytes", "kind", "left_out", "measured", "not_joined", "removed", "seeds",
        "suggested_bytes", "suggested_from", "suggestions"]);
    assert_eq!(names(&value["included"][0]), ["bytes", "calls", "decision", "decision_scope", "end_line", "facts", "id", "kind", "left_out_why", "name", "path",
        "provenance", "reasons", "role", "start_line"]);
    assert_eq!(names(&value["left_out"][0]).len(), 16, "and `why`, there only");
    assert!(value["left_out"][0].get("why").is_some() && value["included"][0].get("why").is_none());
    assert_eq!((&value["kind"], &value["format_version"]), (&json!("selection_record"), &json!(2)));
    fn each<T: Serialize + DeserializeOwned + PartialEq + Debug>(values: Vec<(T, &str)>) {
        for (value, name) in values {
            pinned(value, json!(name));
        }
    }
    each(vec![(SelectionRecordKind::SelectionRecord, "selection_record")]);
    each(vec![(LeftOutWhy::DidNotRun, "did_not_run"), (LeftOutWhy::Test, "test"), (LeftOutWhy::Budget, "budget"), (LeftOutWhy::MaxItems, "max_items")]);
    each(vec![(SourceProvenance::Manual, "manual"), (SourceProvenance::Suggested, "suggested"), (SourceProvenance::LeftOut, "left_out")]);
    each(vec![(Decision::Included, "included"), (Decision::Removed, "removed")]);
    each(vec![(DecisionScope::Single, "single"), (DecisionScope::Bulk, "bulk")]);
    each(vec![(ServerDeclared::Server, "server")]);
    each(vec![(SelectionCheck::Shape, "shape"), (SelectionCheck::IncludedIsNodes, "included_is_nodes"), (SelectionCheck::Partition, "partition"),
        (SelectionCheck::SeedsIncluded, "seeds_included"), (SelectionCheck::Edition, "edition"), (SelectionCheck::Suggestions, "suggestions"),
        (SelectionCheck::Absorption, "absorption"), (SelectionCheck::IncludedBytes, "included_bytes")]);
    // A reason's code is one `envelope.rs` gives, `called_measured` among them; no other.
    pinned(ReasonCode::CalledMeasured, json!("called_measured"));
    refused_as(|v| v["included"][0]["reasons"][0]["code"] = json!("measured_call"), "forma");
    refused_as(|v| v["left_out"][0]["why"] = json!("cold"), "forma");
    refused_as(|v| v["included"][0]["provenance"] = json!("human"), "forma");
    refused_as(|v| v["included"][0]["decision"] = json!("kept"), "forma");
    refused_as(|v| v["included"][0]["role"] = json!("owner"), "forma");
}

#[test]
fn a_selection_out_of_the_contract_is_refused() {
    // The version and the kind: a format 1, or none, is not read as this one.
    refused_as(|v| v["format_version"] = json!(1), "forma: format_version não é 2");
    refused_as(|v| { v.as_object_mut().unwrap().remove("format_version"); }, "forma: format_version não é 2");
    refused_as(|v| v["kind"] = json!("selection"), "forma: kind não é selection_record");
    refused_as(|v| { v.as_object_mut().unwrap().remove("kind"); }, "forma: kind não é selection_record");
    // Unknown fields, `notes` among them, at the top and in a source.
    refused_as(|v| v["extra"] = json!(1), "forma: unknown field `extra`");
    refused_as(|v| v["notes"] = json!([]), "forma: unknown field `notes`");
    refused_as(|v| v["included"][0]["notes"] = json!(""), "forma: unknown field `notes`");
    refused_as(|v| v["gaps"][0]["notes"] = json!(""), "forma: unknown field `notes`");
    refused_as(|v| v["included"][1]["reasons"][0]["notes"] = json!(""), "forma: unknown field `notes`");
    // A field missing, at the top and in a source; a null is not a missing field, and the reverse.
    for field in ["suggestions", "seeds", "suggested_from", "measured", "budget_bytes", "suggested_bytes", "included_bytes", "included", "removed", "left_out", "not_joined", "gaps"] {
        refused_as(|v| { v.as_object_mut().unwrap().remove(field); }, "forma: missing field");
    }
    for field in ["id", "name", "path", "kind", "start_line", "end_line", "role", "bytes", "calls", "reasons", "facts", "left_out_why", "provenance", "decision", "decision_scope"] {
        refused_as(|v| { v["included"][2].as_object_mut().unwrap().remove(field); }, "forma: missing field");
    }
    refused_as(|v| v["included"][0]["reasons"] = Value::Null, "forma");
    // `why`: only in left_out, and never null there.
    refused_as(|v| v["included"][0]["why"] = json!("budget"), "forma: why fora do lugar em included");
    refused_as(|v| v["removed"][0]["why"] = Value::Null, "forma: why fora do lugar em removed");
    refused_as(|v| v["left_out"][0]["why"] = Value::Null, "forma: why fora do lugar em left_out");
    refused_as(|v| { v["left_out"][1].as_object_mut().unwrap().remove("why"); }, "forma: why fora do lugar em left_out");
    // Types.
    refused_as(|v| v["budget_bytes"] = json!("6144"), "forma: invalid type");
    refused_as(|v| v["measured"] = json!(1), "forma: invalid type");
    refused_as(|v| v["included"][0]["start_line"] = json!("10"), "forma: invalid type");
    refused_as(|v| v["included"][0]["bytes"] = json!(-1), "forma: invalid value");
    refused_as(|v| v["suggestions"][0] = json!("abc"), "forma: not a lowercase SHA-256");
    refused_as(|v| v["seeds"] = json!("a"), "forma: invalid type");
    assert_eq!(SelectionInput::from_value(&json!([])).unwrap_err(), SelectionRefusal::Shape("a seleção não é um objeto".into()));
}

#[test]
fn the_included_sources_are_the_questions_nodes_in_their_order() {
    let sent = selection_sent();
    assert_eq!(refusal(&sent, &["b", "a", "header:src/a.rs", "c"]), "included não é a lista de nodes da pergunta, na mesma ordem");
    assert_eq!(refusal(&sent, &["a", "b", "header:src/a.rs"]), "included não é a lista de nodes da pergunta, na mesma ordem");
    assert_eq!(refusal(&sent, &["a", "b", "header:src/a.rs", "c", "d"]), "included não é a lista de nodes da pergunta, na mesma ordem");
    // The same sources in another order: not the order of the question's nodes.
    refused_as(|v| v["included"].as_array_mut().unwrap().swap(0, 1), "included não é a lista de nodes");
}

#[test]
fn each_source_is_in_exactly_one_list() {
    // Twice: in one list or in two.
    refused_as(|v| { let d = v["left_out"][0].clone(); v["left_out"].as_array_mut().unwrap().push(d); }, "d aparece mais de uma vez");
    refused_as(|v| { let mut d = v["left_out"][0].clone(); d.as_object_mut().unwrap().remove("why"); v["not_joined"].as_array_mut().unwrap().push(d); }, "d aparece mais de uma vez");
    // In a list it cannot be in.
    refused_as(|v| { let mut d = v["left_out"].as_array_mut().unwrap().remove(0); d.as_object_mut().unwrap().remove("why"); v["not_joined"].as_array_mut().unwrap().push(d); },
        "partição: d está em not_joined");
    refused_as(|v| { let mut e = v["left_out"].as_array_mut().unwrap().remove(1); e.as_object_mut().unwrap().remove("why"); v["removed"].as_array_mut().unwrap().push(e); },
        "partição: e está em removed sem decisão de remoção");
    refused_as(|v| { let mut e = v["left_out"].as_array_mut().unwrap().remove(1); e.as_object_mut().unwrap().remove("why"); e["decision"] = json!("removed"); e["decision_scope"] = json!("single");
        v["removed"].as_array_mut().unwrap().push(e); }, "");
    // A source a cited suggestion mentions, in no list.
    refused_as(|v| { v["left_out"].as_array_mut().unwrap().remove(1); }, "partição: e é mencionada por uma sugestão citada e não está em nenhuma lista");
    // The seeds: among the included, once.
    refused_as(|v| v["seeds"] = json!(["a", "m"]), "semente m fora de included ou repetida");
    refused_as(|v| v["seeds"] = json!(["a", "a"]), "semente a fora de included ou repetida");
    refused_as(|v| v["seeds"] = json!([]), "");
}

#[test]
fn each_source_is_one_the_edition_has_with_its_data() {
    refused_as(|v| v["removed"][0]["id"] = json!("zz"), "edition: zz: não existe nesta edition");
    let mut unknown = selection_sent();
    unknown["included"][3]["id"] = json!("zz");
    assert_eq!(refusal(&unknown, &["a", "b", "header:src/a.rs", "zz"]), "edition: zz: não existe nesta edition");
    refused_as(|v| v["included"][2]["path"] = json!("src/b.rs"), "edition: header:src/a.rs: caminho");
    refused_as(|v| v["included"][0]["start_line"] = json!(11), "edition: a: linhas");
    refused_as(|v| v["left_out"][0]["end_line"] = json!(8), "edition: d: linhas");
    refused_as(|v| v["included"][1]["kind"] = json!("enum"), "edition: b: kind");
    refused_as(|v| v["included"][1]["name"] = json!("C"), "edition: b: nome");
    refused_as(|v| v["included"][3]["bytes"] = json!(71), "edition: c: bytes");
    // The sum is the server's, from those bytes.
    refused_as(|v| v["included_bytes"] = json!(261), "included_bytes não é a soma dos bytes incluídos");
    refused_as(|v| v["included_bytes"] = json!(290), "included_bytes não é a soma dos bytes incluídos");
}

#[test]
fn what_each_source_says_is_what_the_cited_suggestions_give_in_the_order_given() {
    // The latest suggestion that mentions a source gives its role, count and facts.
    refused_as(|v| v["included"][1]["role"] = json!("origin"), "b: role difere");
    refused_as(|v| v["included"][3]["calls"] = json!(8.0), "c: calls difere");
    refused_as(|v| v["included"][0]["calls"] = Value::Null, "a: calls difere");
    refused_as(|v| v["included"][1]["facts"] = json!([]), "b: facts difere");
    // The reasons of every one, in the order learnt; why it was kept out first, and why now.
    refused_as(|v| { v["included"][3]["reasons"].as_array_mut().unwrap().pop(); }, "c: reasons difere");
    refused_as(|v| v["included"][1]["reasons"].as_array_mut().unwrap().swap(0, 1), "b: reasons difere");
    refused_as(|v| v["included"][3]["left_out_why"] = json!("test"), "c: left_out_why difere");
    refused_as(|v| v["included"][3]["left_out_why"] = Value::Null, "c: left_out_why difere");
    refused_as(|v| v["left_out"][1]["why"] = json!("budget"), "partição: e está em left_out");
    refused_as(|v| v["included"][1]["provenance"] = json!("left_out"), "b: provenance difere");
    refused_as(|v| v["included"][3]["provenance"] = json!("suggested"), "c: provenance difere");
    // The gaps of all, each once, in order.
    refused_as(|v| { v["gaps"].as_array_mut().unwrap().pop(); }, "gaps: a união das lacunas difere");
    refused_as(|v| v["gaps"].as_array_mut().unwrap().swap(0, 1), "gaps: a união das lacunas difere");
    refused_as(|v| { let g = v["gaps"][0].clone(); v["gaps"].as_array_mut().unwrap().push(g); }, "gaps: a união das lacunas difere");
    // The last suggestion's budget, bytes, measure and start.
    refused_as(|v| v["budget_bytes"] = json!(6143), "última sugestão: budget_bytes difere");
    refused_as(|v| v["suggested_bytes"] = json!(190), "última sugestão: suggested_bytes difere");
    refused_as(|v| v["measured"] = json!(false), "última sugestão: measured difere");
    refused_as(|v| v["suggested_from"] = json!(["a"]), "última sugestão: suggested_from difere");
    // The suggestions in another order give other sources: not what was sent.
    let mut swapped = selection_sent();
    swapped["suggestions"] = json!([s2_sha(), s1_sha()]);
    assert!(check_sent(&swapped, &SENT_NODES, &[s2(), s1()]).is_err());
    // Left out twice, for two reasons: why it was first kept out stays, why it is out now is the latest.
    let mut again = s2();
    again.output.left_out.push(served("e", "e_fn", "src/e.rs", "function", (1, 3), "support", 20, vec![reason("caller", "A::run", &[5])], None, Some("budget"), &[]));
    let mut twice_out = selection_sent();
    twice_out["left_out"][1]["why"] = json!("budget");
    assert!(check_sent(&twice_out, &SENT_NODES, &[s1(), again.clone()]).is_ok());
    twice_out["left_out"][1]["left_out_why"] = json!("budget");
    assert_eq!(check_sent(&twice_out, &SENT_NODES, &[s1(), again]).unwrap_err().to_string(), "e: left_out_why difere do que as sugestões dão");
    // A gap a suggestion gives twice is kept twice, as the page keeps it: it filters each suggestion's
    // gaps by those known before it, not by those of the same suggestion.
    let mut twice = s1();
    twice.output.gaps = vec![gap_one(), gap_one()];
    let mut sent = selection_sent();
    sent["gaps"] = json!([gap_one(), gap_one(), gap_two()]);
    assert!(check_sent(&sent, &SENT_NODES, &[twice.clone(), s2()]).is_ok());
    sent["gaps"] = json!([gap_one(), gap_two()]);
    assert_eq!(check_sent(&sent, &SENT_NODES, &[twice, s2()]).unwrap_err().to_string(), "gaps: a união das lacunas difere do que as sugestões dão");
    // A suggestion more or less than those cited.
    assert!(check_sent(&selection_sent(), &SENT_NODES, &[s2()]).unwrap_err().to_string().starts_with("forma: sugestões citadas e lidas não conferem"));
    let mut none = selection_sent();
    none["suggestions"] = json!([]);
    assert!(check_sent(&none, &SENT_NODES, &[]).unwrap_err().to_string().starts_with("forma: nenhuma sugestão citada"));
}

#[test]
fn what_only_the_client_can_say_is_taken_when_nothing_contradicts_it() {
    // A source added by hand and never suggested: what the graph shows of it, nothing else.
    refused_as(|v| v["removed"][0]["name"] = json!("M"), "m: dados de uma fonte manual nunca sugerida difere");
    refused_as(|v| v["removed"][0]["bytes"] = json!(10), "m: dados de uma fonte manual nunca sugerida difere");
    refused_as(|v| v["removed"][0]["provenance"] = json!("suggested"), "declaração: m: nenhuma sugestão citada menciona esta fonte");
    // A source by hand exists through an action: it has a decision.
    refused_as(|v| { v["included"][0]["decision"] = Value::Null; v["included"][0]["decision_scope"] = Value::Null; }, "declaração: a: fonte manual sem decisão");
    // Decisions: both or neither; never `removed` for what is in, always for what was taken out.
    refused_as(|v| v["included"][1]["decision_scope"] = Value::Null, "declaração: b: decision e decision_scope");
    refused_as(|v| v["included"][1]["decision_scope"] = json!("all"), "forma");
    refused_as(|v| v["included"][1]["decision"] = json!("removed"), "partição: b está em included com decisão de remoção");
    refused_as(|v| v["removed"][0]["decision"] = json!("included"), "partição: m está em removed sem decisão de remoção");
    refused_as(|v| { v["left_out"][0]["decision"] = json!("included"); v["left_out"][0]["decision_scope"] = json!("single"); }, "partição: d está em left_out");
    // An included source with no decision of its own is the client's to say.
    refused_as(|v| { v["included"][2]["decision"] = Value::Null; v["included"][2]["decision_scope"] = Value::Null; }, "");
    // The same sources, the person taking out also what S1 brought: `b` and the file head. The file
    // head said to be by hand came before S1 or not at all: the page learnt `b` from S1, and in the
    // order it keeps, what came by hand before S1 comes before it.
    let out = |order: [&str; 3], head: &str| {
        let mut v = selection_sent();
        let mut head_entry = v["included"][2].clone();
        head_entry["provenance"] = json!(head);
        head_entry["decision"] = json!("removed");
        head_entry["decision_scope"] = json!("single");
        let mut b = v["included"][1].clone();
        b["decision"] = json!("removed");
        b["decision_scope"] = json!("single");
        let m = v["removed"][0].clone();
        let by_id = |id: &str| match id { "m" => m.clone(), "b" => b.clone(), _ => head_entry.clone() };
        v["removed"] = json!(order.iter().map(|id| by_id(id)).collect::<Vec<_>>());
        let a = v["included"][0].clone();
        let c = v["included"][3].clone();
        v["included"] = json!([a, c]);
        v["seeds"] = json!(["a"]);
        v["included_bytes"] = json!(170);
        refusal(&v, &["a", "c"])
    };
    assert_eq!(out(["m", "b", "header:src/a.rs"], "suggested"), "");
    assert_eq!(out(["m", "header:src/a.rs", "b"], "manual"), "");
    assert_eq!(out(["m", "b", "header:src/a.rs"], "manual"), "declaração: header:src/a.rs: manual, mas uma sugestão anterior já a mencionava");
    assert_eq!(out(["m", "header:src/a.rs", "b"], "suggested"), "declaração: b: fora da ordem em que as sugestões a trouxeram");
    // In one suggestion, its items come before what it left out: `b` before `e`.
    let both = |order: [&str; 3]| {
        let mut v = selection_sent();
        let mut b = v["included"][1].clone();
        b["decision"] = json!("removed");
        b["decision_scope"] = json!("single");
        let mut e = v["left_out"].as_array_mut().unwrap().remove(1);
        e.as_object_mut().unwrap().remove("why");
        e["decision"] = json!("removed");
        e["decision_scope"] = json!("single");
        let m = v["removed"][0].clone();
        v["removed"] = json!(order.iter().map(|id| match *id { "m" => m.clone(), "b" => b.clone(), _ => e.clone() }).collect::<Vec<_>>());
        let (a, head, c) = (v["included"][0].clone(), v["included"][2].clone(), v["included"][3].clone());
        v["included"] = json!([a, head, c]);
        v["seeds"] = json!(["a"]);
        v["included_bytes"] = json!(210);
        refusal(&v, &["a", "header:src/a.rs", "c"])
    };
    assert_eq!(both(["m", "b", "e"]), "");
    assert_eq!(both(["m", "e", "b"]), "declaração: b: fora da ordem em que as sugestões a trouxeram");
    // The order of what stayed out is the order learnt.
    refused_as(|v| v["left_out"].as_array_mut().unwrap().swap(0, 1), "declaração: d: fora da ordem");
}

#[test]
fn the_record_keeps_what_was_sent_and_tells_what_was_checked_from_what_was_said() {
    let input = check_sent(&selection_sent(), &SENT_NODES, &[s1(), s2()]).unwrap();
    let (one, two) = (s1(), s2());
    let record = SelectionRecord::new(format!("edition:{}", text('1')), "envelope-1", input.clone(), &[&one, &two]);
    let bytes = record.to_bytes().unwrap();
    let sha = librarian_ingest::hash(&bytes);
    // The same input, the same bytes; read back only by its own hash and in its own form.
    assert_eq!(SelectionRecord::new(format!("edition:{}", text('1')), "envelope-1", input.clone(), &[&one, &two]).to_bytes().unwrap(), bytes);
    assert_eq!(SelectionRecord::from_bytes(&bytes, &sha).unwrap(), record);
    assert!(SelectionRecord::from_bytes(&bytes, &text('0')).unwrap_err().starts_with("o registro"));
    let mut spaced = bytes.clone();
    spaced.insert(1, b' ');
    assert_eq!(SelectionRecord::from_bytes(&spaced, &librarian_ingest::hash(&spaced)).unwrap_err(), "registro fora da forma em que é gravado");
    assert!(!String::from_utf8_lossy(&bytes).contains(&sha));
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    let names = |v: &Value| v.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    assert_eq!(names(&value), ["checked_by_server", "declared_by_client", "edition_origin", "format_version", "kind", "received", "rules_version"]);
    assert_eq!((&value["kind"], &value["format_version"]), (&json!("selection_record"), &json!(2)));
    assert_eq!(value["received"], serde_json::to_value(&input).unwrap());
    assert_eq!(value["checked_by_server"], json!({ "declared_by": "server",
        "checks": ["shape", "included_is_nodes", "partition", "seeds_included", "edition", "suggestions", "absorption", "included_bytes"],
        "suggestions": [{ "sha256": s1_sha(), "measure": { "read_status": "none" } }, { "sha256": s2_sha(), "measure": { "read_status": "ok", "execution": "01-medir", "execution_sha256": text('e') } }],
        "included_bytes": 260 }));
    assert_eq!(value["declared_by_client"], json!({ "declared_by": "client", "suggestion_order": [s1_sha(), s2_sha()], "seeds": ["a", "b"],
        "decisions": [{ "id": "a", "decision": "included", "decision_scope": "single" }, { "id": "b", "decision": "included", "decision_scope": "bulk" },
            { "id": "header:src/a.rs", "decision": "included", "decision_scope": "bulk" }, { "id": "c", "decision": "included", "decision_scope": "bulk" },
            { "id": "m", "decision": "removed", "decision_scope": "single" }],
        "manual": ["a", "m"] }));
    // The measure it rested on is its last suggestion's, by name.
    assert_eq!(record.selection_basis(), MeasureSelectionBasis::Ok { suggestion: hash('4'), execution: "01-medir".into(), execution_sha256: hash('e') });
    assert_eq!(record.suggestions(), vec![hash('3'), hash('4')]);
    // Nothing in it says what a person saw, clicked or meant.
    for word in ["clicked", "viewed", "human", "person", "user"] {
        assert!(!String::from_utf8_lossy(&bytes).contains(word), "{word}");
    }
}

#[test]
fn a_package_names_a_selection_with_the_measure_of_its_last_suggestion() {
    let whole = package();
    assert!(check_structure(&whole).is_ok());
    let basis = |p: &mut EvidencePackage, b: MeasureSelectionBasis| p.measure.selection_basis = b;
    let shape = Err(PackageCheck::Shape { what: "selection basis", id: String::new() });
    let mut other = whole.clone();
    basis(&mut other, MeasureSelectionBasis::NoMeasure { suggestion: hash('6') });
    assert_eq!(check_structure(&other), shape);
    basis(&mut other, MeasureSelectionBasis::NotApplicable {});
    assert_eq!(check_structure(&other), shape);
    other.selection = None;
    assert_eq!(check_structure(&other), Ok(()));
    basis(&mut other, MeasureSelectionBasis::NotRecorded {});
    assert_eq!(check_structure(&other), Ok(()), "a package made before selections were checked");
    basis(&mut other, MeasureSelectionBasis::Unreadable { suggestion: hash('5') });
    assert_eq!(check_structure(&other), shape);
    let mut empty = whole.clone();
    empty.selection.as_mut().unwrap().suggestions.clear();
    assert_eq!(check_structure(&empty), shape);
}

#[test]
fn what_a_lacuna_round_added_is_the_servers_once_and_has_its_ficha() {
    let whole = package();
    assert_eq!(check_structure(&whole), Ok(()));
    let refused = |edit: &dyn Fn(&mut EvidencePackage)| { let mut p = whole.clone(); edit(&mut p); check_structure(&p) };
    let shape = |what: &'static str, id: &str| Err(PackageCheck::Shape { what, id: id.into() });
    // Twice the same declaration.
    assert_eq!(refused(&|p| { let a = p.server_additions[0].clone(); p.server_additions.push(a); }), Err(PackageCheck::Duplicate { what: "server addition", id: "sym:b".into() }));
    // No round, no name, no send it came from.
    assert_eq!(refused(&|p| p.server_additions[0].round = 0), shape("server addition", "sym:b"));
    assert_eq!(refused(&|p| p.server_additions[0].for_name.clear()), shape("server addition", "sym:b"));
    assert_eq!(refused(&|p| p.server_additions[0].from_turn.clear()), shape("server addition", "sym:b"));
    assert_eq!(refused(&|p| p.server_additions[0].from_send.clear()), shape("server addition", "sym:b"));
    // An addition with no ficha in the query.
    assert_eq!(refused(&|p| p.server_additions[0].symbol_id = "sym:z".into()), Err(PackageCheck::Unlinked { what: "server addition", id: "sym:z".into() }));
    // The added declaration's ficha said to be the client's, or another's said to be the server's.
    assert_eq!(refused(&|p| p.fichas[1].declared_by = DeclaredBy::Client), shape("ficha declared_by", "E2"));
    assert_eq!(refused(&|p| p.fichas[1].declared_by = DeclaredBy::Server), shape("ficha declared_by", "E2"));
    assert_eq!(refused(&|p| p.fichas[0].declared_by = DeclaredBy::ServerLacuna), shape("ficha declared_by", "E1"));
    // Nothing added: no ficha is the server's lacuna's.
    assert_eq!(refused(&|p| { p.server_additions.clear(); p.fichas[1].declared_by = DeclaredBy::Client; }), Ok(()));
    assert_eq!(refused(&|p| p.server_additions.clear()), shape("ficha declared_by", "E2"));
    // The record of an addition has exactly the written names.
    assert_eq!(serde_json::to_value(&whole.server_additions[0]).unwrap(), json!({ "declared_by": "server_lacuna", "round": 1, "from_turn": "12", "from_send": "01-provedor",
        "symbol_id": "sym:b", "for_name": "b", "reason": "lacuna_name", "decided_by_rule": "ladder::resolve" }));
}

/// A4: the same functions of the contract, U1 to U6, on neutral fixtures, with nothing that depends on
/// a project: three kinds of declaration in one edition (a free function, a method of a class, a method
/// in a block of methods), each source in every line form `lines-v1` names, radius 0 and 2, the prose
/// form and the claims form.
#[test]
fn a4_every_fixture_passes_u1_to_u6_in_both_forms_and_radii() {
    let free = "fn alfa() {\n    1;\n}\n\nfn beta() {\n    2;\n}\n";
    let class = "class K:\n    def m(self):\n        return 1\n\n    def n(self):\n        return 2\n";
    let block = "struct P {\n    x: u8,\n}\n\nimpl P {\n    fn ler(&self) -> u8 {\n        self.x\n    }\n}\n";
    type LineForm = (&'static str, fn(&str) -> Vec<u8>);
    let forms: [LineForm; 7] = [
        ("lf", |t| t.as_bytes().to_vec()),
        ("crlf", |t| t.replace('\n', "\r\n").into_bytes()),
        ("cr_cr_lf", |t| t.replace('\n', "\r\r\n").into_bytes()),
        ("lone_cr", |t| t.replacen('\n', "\r", 1).into_bytes()),
        ("bom", |t| [b"\xEF\xBB\xBF".as_slice(), t.as_bytes()].concat()),
        ("no_final_break", |t| t.trim_end_matches('\n').as_bytes().to_vec()),
        ("trailing_empty_lines", |t| format!("{t}\n\n\n").into_bytes()),
    ];
    for (name, make) in forms {
        let catalog = edition(&[("src/x.rs", &make(free)), ("src/y.py", &make(class)), ("src/z.rs", &make(block))]);
        let line = |path: &str, symbol: &str| catalog.symbols().values().find(|s| s.name == symbol && catalog.sources()[&s.source_id].path == path)
            .map(|s| catalog.chunks()[&s.chunk_id].start_line).unwrap_or_else(|| panic!("{name}: {symbol} in {path}"));
        let targets = || vec![target("src/x.rs", "beta", line("src/x.rs", "beta")), target("src/y.py", "n", line("src/y.py", "n")), target("src/z.rs", "ler", line("src/z.rs", "ler"))];
        for radius in [0, 2] {
            let case = Case::of(&catalog, targets(), radius);
            let at = format!("{name}, radius {radius}");
            // U1 to U5, in prose and in claims, and U6 in prose (claims U6 is on the recorded fixtures).
            let composed = prose(&case).unwrap_or_else(|e| panic!("{at}: {e:?}"));
            check_prose(&case, &composed).unwrap_or_else(|e| panic!("{at}: {e:?}"));
            let items = case.claims();
            let view = claims_view(&items);
            let snapshots = case.inputs();
            let input = CompositionInput { query_json: &case.query, seeds: &case.composition.fichas, radius: case.composition.radius, form: Form::Claims(&view), snapshots: &snapshots };
            let in_claims = compose(&input).unwrap_or_else(|e| panic!("{at}: {e:?}"));
            check(&in_claims, &input).unwrap_or_else(|e| panic!("{at}: {e:?}"));
            prose_provenance(&case, &[]).unwrap_or_else(|e| panic!("{at}: {e:?}"));
            // Every declaration asked for has its ficha, of every kind, each in a unit of its source.
            let fichas: Vec<(&str, &str)> = composed.fichas.iter().map(|f| (f.path.as_str(), f.unit.0.as_str())).collect();
            for path in ["src/x.rs", "src/y.py", "src/z.rs"] {
                assert!(fichas.iter().any(|(p, _)| *p == path), "{at}: {path} {fichas:?}");
            }
            for (path, unit) in &fichas {
                assert!(composed.units.iter().any(|u| u.evidence_id.0 == *unit && u.path == *path), "{at}: {path} {unit}");
            }
        }
    }
}

/// B5: a question frozen in the repository. The current code must prepare its query again byte for
/// byte, on every system; a change in what the preparation writes (a fixed instruction among it)
/// fails here before it can be committed. That such a change makes H3 refuse a stored question while
/// H1 and H2 pass is in the Studio's tests (`h3_the_current_code_makes_the_same_package_and_query_again`).
#[test]
fn b5_a_frozen_question_is_prepared_again_byte_for_byte() {
    let frozen = include_str!("evidence-fixtures/congelada.query.json").replace("\r\n", "\n");
    let case = Case::of(&edition(&PROJECT), vec![target("src/d.rs", "ler", 5)], 1);
    assert_eq!(case.query, frozen);
    // And it is a query the contract takes as it is: its units come again from the snapshot.
    let composed = prose(&case).unwrap();
    check_prose(&case, &composed).unwrap();
    prose_provenance(&case, &[]).unwrap();
}

/// B11: `envelope.rs` is inside the fence too, but for its named exceptions, which are where a
/// language's syntax is still known: `trait_impls` (blocks of methods of a trait for a type) and the
/// limit that says calls inside macros are not seen. A new one fails here.
#[test]
fn b11_the_envelope_is_fenced_but_for_its_named_exceptions() {
    let text = include_str!("../src/envelope.rs").replace("\r\n", "\n");
    let start = text.find(concat!("pub fn ", "trait_impls(")).unwrap();
    let end = start + text[start..].find("\n}\n").unwrap() + 3;
    let limit = concat!("calls inside ", "macros are not seen");
    assert!(text[start..end].contains(concat!("\"im", "pl\"")), "the exception is still what it is named for");
    assert!(text.contains(limit));
    let rest = format!("{}{}", &text[..start], &text[end..]).replace(limit, "").to_lowercase();
    for word in FORBIDDEN {
        assert!(!rest.contains(word), "envelope.rs mentions {word:?} outside its named exceptions");
    }
}

// P1: the two measures compared (format 2), every pair of readings.

fn at_ok(execution: &str, sha: char) -> MeasureAtQuestion {
    MeasureAtQuestion::Ok { execution: execution.into(), execution_sha256: hash(sha) }
}

fn basis_ok(suggestion: char, execution: &str, sha: char) -> MeasureSelectionBasis {
    MeasureSelectionBasis::Ok { suggestion: hash(suggestion), execution: execution.into(), execution_sha256: hash(sha) }
}

fn cannot(reasons: &[(MeasureSide, Incomparable)]) -> MeasureDivergence {
    MeasureDivergence::NotComparable { reasons: reasons.iter().map(|&(side, read_status)| NotComparable { side, read_status }).collect() }
}

#[test]
fn the_two_measures_compared_give_one_of_four_answers_and_name_what_cannot_be_compared() {
    use Incomparable::{NotRecorded, Unreadable};
    use MeasureDivergence::{Diverges, NotApplicable, Same};
    use MeasureSide::{AtQuestion, SelectionBasis};
    let none = MeasureAtQuestion::NoMeasure {};
    let at_unreadable = MeasureAtQuestion::Unreadable {};
    let basis_none = MeasureSelectionBasis::NoMeasure { suggestion: hash('5') };
    let basis_unreadable = MeasureSelectionBasis::Unreadable { suggestion: hash('5') };
    let not_recorded = MeasureSelectionBasis::NotRecorded {};
    let no_selection = MeasureSelectionBasis::NotApplicable {};
    let cases: Vec<(&str, MeasureAtQuestion, MeasureSelectionBasis, MeasureDivergence)> = vec![
        // ok and ok: the execution and its bytes; the suggestion that named it is not part of a measure.
        ("ok, mesma execução e bytes", at_ok("01-x", 'a'), basis_ok('5', "01-x", 'a'), Same {}),
        ("ok, outra sugestão, mesma medição", at_ok("01-x", 'a'), basis_ok('9', "01-x", 'a'), Same {}),
        ("ok, outra execução", at_ok("02-x", 'a'), basis_ok('5', "01-x", 'a'), Diverges {}),
        ("ok, mesma execução, outros bytes", at_ok("01-x", 'b'), basis_ok('5', "01-x", 'a'), Diverges {}),
        ("ok, outra execução e outros bytes", at_ok("02-x", 'b'), basis_ok('5', "01-x", 'a'), Diverges {}),
        // none: both none is the same; one none and one ok is not, either way.
        ("none e none", none.clone(), basis_none.clone(), Same {}),
        ("none na pergunta, ok na seleção", none.clone(), basis_ok('5', "01-x", 'a'), Diverges {}),
        ("ok na pergunta, none na seleção", at_ok("01-x", 'a'), basis_none.clone(), Diverges {}),
        // No selection: nothing to compare, whatever the question read.
        ("sem seleção, ok", at_ok("01-x", 'a'), no_selection.clone(), NotApplicable {}),
        ("sem seleção, none", none.clone(), no_selection.clone(), NotApplicable {}),
        ("sem seleção, ilegível", at_unreadable.clone(), no_selection.clone(), NotApplicable {}),
        // A side that cannot be compared is named, with why; the question's side first.
        ("pergunta ilegível, seleção ok", at_unreadable.clone(), basis_ok('5', "01-x", 'a'), cannot(&[(AtQuestion, Unreadable)])),
        ("pergunta ilegível, seleção none", at_unreadable.clone(), basis_none.clone(), cannot(&[(AtQuestion, Unreadable)])),
        ("seleção ilegível, pergunta ok", at_ok("01-x", 'a'), basis_unreadable.clone(), cannot(&[(SelectionBasis, Unreadable)])),
        ("seleção ilegível, pergunta none", none.clone(), basis_unreadable.clone(), cannot(&[(SelectionBasis, Unreadable)])),
        ("seleção sem registro, pergunta ok", at_ok("01-x", 'a'), not_recorded.clone(), cannot(&[(SelectionBasis, NotRecorded)])),
        ("seleção sem registro, pergunta none", none.clone(), not_recorded.clone(), cannot(&[(SelectionBasis, NotRecorded)])),
        ("os dois ilegíveis", at_unreadable.clone(), basis_unreadable.clone(), cannot(&[(AtQuestion, Unreadable), (SelectionBasis, Unreadable)])),
        ("pergunta ilegível, seleção sem registro", at_unreadable.clone(), not_recorded.clone(), cannot(&[(AtQuestion, Unreadable), (SelectionBasis, NotRecorded)])),
    ];
    for (name, at, basis, expected) in &cases {
        assert_eq!(&measure_divergence(at, basis), expected, "{name}");
        // The same readings, the same answer: nothing but the two is read.
        assert_eq!(measure_divergence(at, basis), measure_divergence(&at.clone(), &basis.clone()), "{name}");
    }
    // Every reading of each side meets every reading of the other.
    let ats = [at_ok("01-x", 'a'), none, at_unreadable];
    let bases = [basis_ok('5', "01-x", 'a'), basis_none, basis_unreadable, not_recorded, no_selection];
    for at in &ats {
        for basis in &bases {
            assert!(cases.iter().any(|(_, a, b, _)| a == at && b == basis), "{at:?} {basis:?} sem caso");
        }
    }
}

#[test]
fn the_comparison_is_written_with_its_state_and_reasons_and_nothing_else_reads_as_it() {
    use Incomparable::{NotRecorded, Unreadable};
    use MeasureSide::{AtQuestion, SelectionBasis};
    pinned(MeasureDivergence::Same {}, json!({ "status": "same" }));
    pinned(MeasureDivergence::Diverges {}, json!({ "status": "diverges" }));
    pinned(MeasureDivergence::NotApplicable {}, json!({ "status": "not_applicable" }));
    pinned(cannot(&[(AtQuestion, Unreadable), (SelectionBasis, NotRecorded)]),
        json!({ "status": "not_comparable", "reasons": [{ "side": "at_question", "read_status": "unreadable" }, { "side": "selection_basis", "read_status": "not_recorded" }] }));
    let read = |value: Value| serde_json::from_value::<MeasureDivergence>(value);
    assert!(read(json!({ "status": "talvez" })).is_err());
    assert!(read(json!({ "status": "same", "reasons": [] })).is_err());
    assert!(read(json!({ "status": "not_comparable" })).is_err());
    assert!(read(json!({ "status": "not_comparable", "reasons": [{ "side": "outro", "read_status": "unreadable" }] })).is_err());
    assert!(read(json!({ "status": "not_comparable", "reasons": [{ "side": "at_question", "read_status": "none" }] })).is_err());
    assert!(read(json!(true)).is_err());
    // `measure`: exactly one of the two fields, never null, never both, never neither.
    let measure = |extra: Value| {
        let mut value = json!({ "at_question": { "read_status": "none" }, "selection_basis": { "read_status": "not_applicable" } });
        for (k, v) in extra.as_object().unwrap() {
            value[k] = v.clone();
        }
        serde_json::from_value::<MeasureReferences>(value)
    };
    assert_eq!(measure(json!({ "measure_diverges": false })).unwrap().measure_diverges, Some(false));
    assert_eq!(measure(json!({ "measure_divergence": { "status": "not_applicable" } })).unwrap().measure_divergence, Some(MeasureDivergence::NotApplicable {}));
    assert!(measure(json!({})).is_err());
    assert!(measure(json!({ "measure_diverges": false, "measure_divergence": { "status": "not_applicable" } })).is_err());
    assert!(measure(json!({ "measure_diverges": null })).is_err());
    assert!(measure(json!({ "measure_divergence": null })).is_err());
    assert!(measure(json!({ "measure_diverges": false, "outro": 1 })).is_err());
    // Written back as read: format 1 has only its field, format 2 only its own.
    let v1 = measure(json!({ "measure_diverges": false })).unwrap();
    assert_eq!(serde_json::to_value(&v1).unwrap(), json!({ "at_question": { "read_status": "none" }, "selection_basis": { "read_status": "not_applicable" }, "measure_diverges": false }));
    let v2 = measure(json!({ "measure_divergence": { "status": "not_applicable" } })).unwrap();
    assert_eq!(serde_json::to_value(&v2).unwrap(), json!({ "at_question": { "read_status": "none" }, "selection_basis": { "read_status": "not_applicable" }, "measure_divergence": { "status": "not_applicable" } }));
}

/// The test package in format 2, its comparison worked out.
fn package_v2() -> EvidencePackage {
    let mut package = package();
    package.format_version = PackageFormat::V2;
    package.measure.measure_diverges = None;
    package.measure.measure_divergence = Some(measure_divergence(&package.measure.at_question, &package.measure.selection_basis));
    package
}

#[test]
fn h1_refuses_a_comparison_that_is_not_the_two_measures_compared_or_not_its_formats() {
    use Incomparable::{NotRecorded, Unreadable};
    use MeasureSide::{AtQuestion, SelectionBasis};
    let shape = Err(PackageCheck::Shape { what: "measure divergence", id: String::new() });
    let whole = package_v2();
    assert_eq!(whole.measure.measure_divergence, Some(MeasureDivergence::Diverges {}));
    assert_eq!(check_structure(&whole), Ok(()));
    // Every other state, and every other set of reasons.
    for wrong in [MeasureDivergence::Same {}, MeasureDivergence::NotApplicable {}, cannot(&[]), cannot(&[(AtQuestion, Unreadable)]),
        cannot(&[(SelectionBasis, NotRecorded)]), cannot(&[(SelectionBasis, Unreadable), (AtQuestion, Unreadable)])] {
        let mut changed = whole.clone();
        changed.measure.measure_divergence = Some(wrong.clone());
        assert_eq!(check_structure(&changed), shape, "{wrong:?}");
    }
    // A reason changed in a package that cannot be compared: the side, the cause, the order, one more.
    let mut unreadable = whole.clone();
    unreadable.measure.at_question = MeasureAtQuestion::Unreadable {};
    unreadable.measure.selection_basis = MeasureSelectionBasis::Unreadable { suggestion: hash('5') };
    unreadable.measure.measure_divergence = Some(cannot(&[(AtQuestion, Unreadable), (SelectionBasis, Unreadable)]));
    assert_eq!(check_structure(&unreadable), Ok(()));
    for wrong in [cannot(&[(SelectionBasis, Unreadable), (AtQuestion, Unreadable)]), cannot(&[(AtQuestion, Unreadable)]),
        cannot(&[(AtQuestion, NotRecorded), (SelectionBasis, Unreadable)]), cannot(&[(AtQuestion, Unreadable), (SelectionBasis, NotRecorded)]),
        cannot(&[(AtQuestion, Unreadable), (SelectionBasis, Unreadable), (SelectionBasis, Unreadable)])] {
        let mut changed = unreadable.clone();
        changed.measure.measure_divergence = Some(wrong.clone());
        assert_eq!(check_structure(&changed), shape, "{wrong:?}");
    }
    // A reading changed without the state: the state no longer agrees.
    let mut changed = whole.clone();
    changed.measure.at_question = at_ok("01-x", '6');
    assert_eq!(check_structure(&changed), shape);
    // Each format with its own field only: a package made in memory can carry both, and is refused.
    let mut changed = whole.clone();
    changed.measure.measure_diverges = Some(false);
    assert_eq!(check_structure(&changed), shape);
    let mut changed = whole.clone();
    changed.measure.measure_divergence = None;
    changed.measure.measure_diverges = Some(true);
    assert_eq!(check_structure(&changed), shape);
    let mut changed = package();
    changed.measure.measure_divergence = whole.measure.measure_divergence.clone();
    changed.measure.measure_diverges = None;
    assert_eq!(check_structure(&changed), shape);
    // Format 1 as it was: its value is not worked out here (making it again, H3, gives what that
    // format wrote); the structure is checked as before.
    for value in [false, true] {
        let mut v1 = package();
        v1.measure.measure_diverges = Some(value);
        assert_eq!(check_structure(&v1), Ok(()), "{value}");
    }
    // The rest of H1 is the same in format 2: the selection's basis still has to be its last suggestion's.
    let mut changed = whole.clone();
    changed.measure.selection_basis = basis_ok('6', "01-x", '6');
    changed.measure.measure_divergence = Some(measure_divergence(&changed.measure.at_question, &changed.measure.selection_basis));
    assert_eq!(check_structure(&changed), Err(PackageCheck::Shape { what: "selection basis", id: String::new() }));
}

#[test]
fn a_package_of_format_2_is_written_with_its_number_and_its_field_and_read_back_as_it_was() {
    let v2 = package_v2();
    let value = serde_json::to_value(&v2).unwrap();
    assert_eq!(value["format_version"], json!(2));
    assert_eq!(value["measure"]["measure_divergence"], json!({ "status": "diverges" }));
    assert!(value["measure"].get("measure_diverges").is_none());
    assert_eq!(parse(value).unwrap(), v2);
    let v1 = serde_json::to_value(package()).unwrap();
    assert_eq!((&v1["format_version"], &v1["measure"]["measure_diverges"]), (&json!(1), &json!(true)));
    assert!(v1["measure"].get("measure_divergence").is_none());
}
