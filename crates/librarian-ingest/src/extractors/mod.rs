//! Language extractor contract and versioned registries. Parser types never leave an adapter.
use crate::{Chunk, Diagnostic, Source, Symbol};
use std::path::Path;

mod rust;

/// Records derived from one source's preserved bytes; nothing else is read.
pub(crate) struct Extraction {
    pub symbols: Vec<Symbol>,
    pub chunks: Vec<Chunk>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Syntactic occurrence in original byte offsets. The graph builder attaches owner and evidence.
pub(crate) struct Occurrence {
    pub kind: &'static str,
    pub start: usize,
    pub end: usize,
    /// Written name that may match declarations; a navigational candidate, never a resolution.
    pub candidate: Option<String>,
}

pub(crate) trait Extractor: Sync {
    fn extract(&self, source: &Source, bytes: &[u8]) -> Extraction;
    /// Empty when the bytes do not parse; `extract` already reports that diagnostic.
    fn observe(&self, bytes: &[u8]) -> Vec<Occurrence>;
    /// Symbol kinds a written name may point to as a candidate.
    fn candidate_kinds(&self) -> &'static [&'static str];
    /// Container kinds used for navigation; search does not rank them as answers.
    fn navigation_kinds(&self) -> &'static [&'static str];
}

/// Discovery, classification, extractors and limits behind one recorded identity.
/// Generation and verification use the same registry; editions are never reinterpreted.
pub(crate) struct Registry {
    pub identity: &'static str,
    /// Extensions discovered in directories and the language they are classified as.
    languages: &'static [(&'static str, &'static str)],
    extractors: &'static [(&'static str, &'static dyn Extractor)],
    limits: &'static [&'static str],
    /// Version recorded in graphs built from editions of this profile.
    pub graph_version: &'static str,
    /// Limits reported with each search; they describe what this profile cannot observe.
    pub search_limits: &'static [&'static str],
}

/// Frozen v1 profile. Changing any of it requires a new identity, not an edit.
static V1: Registry = Registry {
    identity: crate::EXTRACTOR,
    languages: &[("rs", "rust")],
    extractors: &[("rust", &rust::Rust)],
    limits: &[
        "Structural declarations only; no behavioral claims or executed-test evidence.",
        "No macro expansion, cfg evaluation, cross-file name resolution or call graph.",
        "File-local scopes; ordinary comments are preserved in snapshots, not attached to declarations.",
        "Files read individually; no atomic repository snapshot or Git/author authentication.",
        "Parse failures preserve bytes and diagnostics; completeness requires inspecting diagnostics.",
        "Hashes establish consistency, not authenticity; manifest is not signed.",
    ],
    graph_version: "rust-syntax-graph-v1",
    search_limits: &[
        "Lexical candidates, not a semantic answer. Minimum coverage: 60% of query term groups; scores are not probabilities.",
        "Name candidates are not resolved types/calls. No macros, cfg evaluation, compiler or runtime proof.",
        "Graph expands only the first hit. Source nodes are leaves. Depth bounded to 0..3, 48 nodes, 96 edges.",
        "Excerpts limited to 4000 UTF-8 bytes each and 16000 total. Truncation is explicit; hashes/lines refer to full chunks.",
    ],
};

impl Registry {
    /// Profile used for new editions.
    pub fn current() -> &'static Self {
        &V1
    }

    /// Profile that produced an edition; unknown identities are refused, never upgraded.
    pub fn for_identity(identity: &str) -> Option<&'static Self> {
        [&V1].into_iter().find(|r| r.identity == identity)
    }

    /// Unlisted extensions are auxiliary: preserved and hashed, never extracted.
    pub fn language(&self, path: &str) -> &'static str {
        let extension = Path::new(path).extension().and_then(|e| e.to_str());
        self.languages
            .iter()
            .find(|(e, _)| Some(*e) == extension)
            .map_or("auxiliary", |(_, language)| language)
    }

    /// Whether directory traversal selects this path. Explicit inputs are always selected.
    pub fn discovers(&self, path: &str) -> bool {
        self.language(path) != "auxiliary"
    }

    pub fn extractor(&self, language: &str) -> Option<&'static dyn Extractor> {
        self.extractors
            .iter()
            .find(|(l, _)| *l == language)
            .map(|(_, extractor)| *extractor)
    }

    pub fn limits(&self) -> Vec<String> {
        self.limits.iter().map(|l| l.to_string()).collect()
    }
}
