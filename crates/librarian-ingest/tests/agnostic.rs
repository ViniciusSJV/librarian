//! Fence: the graph builder and search know records and occurrences, never a language.
//! Language knowledge belongs to `src/extractors/` and its versioned registry profiles.

const AGNOSTIC: [(&str, &str); 5] = [
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
