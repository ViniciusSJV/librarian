use bibliotecario_core::{BibliotecarioFact, EvidenceBundle, Selection, SourceRef};

fn fixture() -> EvidenceBundle {
    EvidenceBundle::new(
        SourceRef {
            id: "S".into(),
            path: "manual.txt".into(),
            lines: vec!["instruction".into()],
        },
        vec![BibliotecarioFact::new("F", "instruction", "S", 1)],
        Selection::new(vec!["F"], 1),
    )
}

#[test]
fn rejects_dangling_source_line_and_selection() {
    assert!(fixture().validate().ok);
    let mut bundle = fixture();
    bundle.facts[0].source_id = "OTHER".into();
    assert!(!bundle.validate().ok);
    let mut bundle = fixture();
    bundle.facts[0].line = 2;
    assert!(!bundle.validate().ok);
    let mut bundle = fixture();
    bundle.selection.fact_ids = vec!["UNKNOWN".into()];
    assert!(!bundle.validate().ok);
}

#[test]
fn rejects_duplicate_facts_and_selected_ids() {
    let mut bundle = fixture();
    bundle.facts.push(bundle.facts[0].clone());
    assert!(!bundle.validate().ok);
    let mut bundle = fixture();
    bundle.selection.fact_ids.push("F".into());
    assert!(!bundle.validate().ok);
}
