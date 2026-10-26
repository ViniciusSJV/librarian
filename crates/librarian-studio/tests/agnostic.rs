//! Fence: running a project's commands knows argument lists, exit codes, time and
//! `librarian-measure` lines, never a language, a build tool or a benchmark format.

const AGNOSTIC: [(&str, &str); 1] = [("execution.rs", include_str!("../src/execution.rs"))];

const FORBIDDEN: [&str; 11] = [
    "rust", "cargo", "python", "pytest", "npm", "node", "java", "maven", "gradle", "criterion", "bench",
];

#[test]
fn execution_does_not_name_tools() {
    for (file, text) in AGNOSTIC {
        let lower = text.to_lowercase();
        for word in FORBIDDEN {
            assert!(!lower.contains(word), "{file} mentions {word:?}");
        }
    }
}
