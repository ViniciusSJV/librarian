//! Fence: the dossier engine decides by declared source nature, never by language.
//! Legacy v0.1 `kind` aliases live only in `src/graph_engine/nature.rs`.

const AGNOSTIC: [(&str, &str); 6] = [
    ("graph_engine.rs", include_str!("../src/graph_engine.rs")),
    ("bundle.rs", include_str!("../src/graph_engine/bundle.rs")),
    ("capture.rs", include_str!("../src/graph_engine/capture.rs")),
    ("filesystem.rs", include_str!("../src/graph_engine/filesystem.rs")),
    ("metrics.rs", include_str!("../src/graph_engine/metrics.rs")),
    ("lib.rs", include_str!("../src/lib.rs")),
];

const FORBIDDEN: [&str; 6] = [
    "rust", "python", "java", "typescript", "\"test_run\"", "kind.as_deref",
];

#[test]
fn engine_does_not_decide_by_language_or_legacy_kind() {
    for (file, text) in AGNOSTIC {
        let lower = text.to_lowercase();
        for word in FORBIDDEN {
            assert!(!lower.contains(word), "{file} mentions {word:?}");
        }
    }
}
