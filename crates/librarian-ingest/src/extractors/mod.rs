//! Language extractor contract and versioned registries. Parser types never leave an adapter.
use crate::{Chunk, Diagnostic, Source, Symbol};
use std::path::Path;

mod python;
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
    /// A block (a loop, a `with`) spans its body, but its label is only the head, up to here.
    /// None labels the whole range.
    pub label_end: Option<usize>,
}

pub(crate) trait Extractor: Sync {
    fn extract(&self, source: &Source, bytes: &[u8]) -> Extraction;
    /// Empty when the bytes do not parse; `extract` already reports that diagnostic.
    fn observe(&self, bytes: &[u8]) -> Vec<Occurrence>;
    /// Symbol kinds a written name may point to as a candidate.
    fn candidate_kinds(&self) -> &'static [&'static str];
    /// Candidate kinds for one occurrence kind; profiles before G6 use `candidate_kinds` for all.
    fn candidate_kinds_for(&self, _occurrence: &str) -> &'static [&'static str] {
        self.candidate_kinds()
    }
    /// Container kinds used for navigation; search does not rank them as answers.
    fn navigation_kinds(&self) -> &'static [&'static str];
    /// How a method's owner type and the writes of its fields are written (G6d). None when the
    /// adapter does not record them.
    fn owner_syntax(&self) -> Option<&'static OwnerSyntax> {
        None
    }
    /// Occurrence kinds that are calls, followed by "what does it call?" (P1b).
    fn call_kinds(&self) -> &'static [&'static str] {
        &[]
    }
    /// Occurrence kinds recorded only when their written name matches a declaration (v5): a
    /// plain name is far more often a local than a use, and nothing is kept for those.
    fn matched_only(&self) -> &'static [&'static str] {
        &[]
    }
    /// Occurrence kinds whose plain name means the top-level declaration of the same file when
    /// there is one, as a module scope works (v5); otherwise every homonym is a candidate.
    fn file_scoped(&self) -> &'static [&'static str] {
        &[]
    }
}

/// G6d: the written forms an owner context follows in one language. Only names and occurrence
/// kinds; the graph-side rule (`owner.rs`) never names a language.
pub struct OwnerSyntax {
    /// Symbol kinds whose owner is looked for.
    pub method_kinds: &'static [&'static str],
    /// A container of these kinds is the owner itself.
    pub owner_containers: &'static [&'static str],
    /// A container of this kind names its owner with an occurrence of this kind.
    pub naming_container: Option<(&'static str, &'static str)>,
    /// Symbol kinds a naming container may name.
    pub owner_kinds: &'static [&'static str],
    /// The receiver of the method's own fields (`self.<field>`).
    pub receiver: &'static str,
    /// Occurrence kind of a field read through a receiver.
    pub read: &'static str,
    /// Occurrence kind of a field write through a receiver (`x.<field> = ...`).
    pub write: Option<&'static str>,
    /// Literal of a type and the fields it fills, and the written name that means the
    /// container's own type (`Self`).
    pub literal: Option<(&'static str, &'static str, &'static str)>,
}

impl Registry {
    /// Owner syntax of a language in this profile; None when the profile records no owner context.
    pub fn owner_syntax(&self, language: &str) -> Option<&'static OwnerSyntax> {
        if !self.owner_context {
            return None;
        }
        self.extractor(language)?.owner_syntax()
    }
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
    /// Directory names never traversed during discovery (environments, caches, VCS).
    /// Explicit inputs are still selected.
    pub skipped_dirs: &'static [&'static str],
    /// Above this many symbols with the written name, no candidate edge is drawn and the
    /// occurrence records the count instead. None keeps every candidate (profiles before G6).
    pub max_candidates: Option<usize>,
    /// Whether editions record what a chain step needs to bring a method's owner context
    /// (impl self types and field writes; G6d).
    pub owner_context: bool,
}

/// Frozen v1 profile. Changing any of it requires a new identity, not an edit.
static V1: Registry = Registry {
    identity: crate::EXTRACTOR,
    languages: &[("rs", "rust")],
    extractors: &[("rust", &rust::RUST)],
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
    skipped_dirs: &[],
    max_candidates: None,
    owner_context: false,
};

/// Rust (unchanged adapter) and Python. Rust records match v1; the identity differs.
static V2: Registry = Registry {
    identity: crate::EXTRACTOR_V2,
    languages: &[("rs", "rust"), ("py", "python")],
    extractors: &[("rust", &rust::RUST), ("python", &python::PYTHON)],
    limits: &[
        "Structural declarations only; no behavioral claims or executed-test evidence.",
        "Rust: no macro expansion, cfg evaluation, cross-file name resolution or call graph.",
        "Python: no import resolution, dynamic dispatch, decorator evaluation or runtime behavior; files that do not parse are diagnostics.",
        "Test markers are observations (Rust #[test]; Python test*/Test* names), not evidence that tests ran.",
        "File-local scopes; ordinary comments are preserved in snapshots, not attached to declarations.",
        "Files read individually; no atomic repository snapshot or Git/author authentication.",
        "Parse failures preserve bytes and diagnostics; completeness requires inspecting diagnostics.",
        "Hashes establish consistency, not authenticity; manifest is not signed.",
    ],
    graph_version: "rust-python-syntax-graph-v1",
    search_limits: &[
        "Lexical candidates, not a semantic answer. Minimum coverage: 60% of query term groups; scores are not probabilities.",
        "Name candidates are not resolved types/calls/imports. No macros, cfg or decorator evaluation, compiler or runtime proof.",
        "Graph expands only the first hit. Source nodes are leaves. Depth bounded to 0..3, 48 nodes, 96 edges.",
        "Excerpts limited to 4000 UTF-8 bytes each and 16000 total. Truncation is explicit; hashes/lines refer to full chunks.",
    ],
    skipped_dirs: &[
        ".git", ".hg", ".svn", ".venv", "venv", ".tox", ".nox", "__pycache__", ".mypy_cache",
        ".pytest_cache", ".ruff_cache", "site-packages", "node_modules", "target", ".studio",
    ],
    max_candidates: None,
    owner_context: false,
};

/// G6: the v2 languages plus fields, attribute accesses, and calls and bases as name candidates.
/// Still syntax: a candidate is a written name that matches, never a resolved target.
static V3: Registry = Registry {
    identity: crate::EXTRACTOR_V3,
    languages: &[("rs", "rust"), ("py", "python")],
    extractors: &[("rust", &rust::RUST_V3), ("python", &python::PYTHON_V3)],
    limits: &[
        "Structural declarations only; no behavioral claims or executed-test evidence.",
        "Rust: no macro expansion, cfg evaluation, cross-file name resolution or call graph.",
        "Python: no import resolution, dynamic dispatch, decorator evaluation or runtime behavior; files that do not parse are diagnostics.",
        "Fields: Rust named struct fields; Python class-body assignments and self.<name> assignments in __init__.",
        "Calls, attribute accesses and base classes link to same-named declarations as candidates; receivers and types are not resolved.",
        "A name with more than 8 same-language declarations of a candidate kind gets no candidate edges; the occurrence records the count.",
        "Test markers are observations (Rust #[test]; Python test*/Test* names), not evidence that tests ran.",
        "File-local scopes; ordinary comments are preserved in snapshots, not attached to declarations.",
        "Files read individually; no atomic repository snapshot or Git/author authentication.",
        "Parse failures preserve bytes and diagnostics; completeness requires inspecting diagnostics.",
        "Hashes establish consistency, not authenticity; manifest is not signed.",
    ],
    graph_version: "rust-python-syntax-graph-v2",
    search_limits: &[
        "Lexical candidates, not a semantic answer. Minimum coverage: 60% of query term groups; scores are not probabilities.",
        "Name candidates are not resolved types/calls/fields/imports. No macros, cfg or decorator evaluation, compiler or runtime proof.",
        "Graph expands only the first hit. Source nodes are leaves. Depth bounded to 0..3, 48 nodes, 96 edges.",
        "Excerpts limited to 4000 UTF-8 bytes each and 16000 total. Truncation is explicit; hashes/lines refer to full chunks.",
    ],
    skipped_dirs: &[
        ".git", ".hg", ".svn", ".venv", "venv", ".tox", ".nox", "__pycache__", ".mypy_cache",
        ".pytest_cache", ".ruff_cache", "site-packages", "node_modules", "target", ".studio",
    ],
    max_candidates: Some(8),
    owner_context: false,
};

/// G6d: v3 plus, in Rust, struct literals (`struct_literal`, `field_init`), field writes
/// (`field_assign`) and the type written in an impl (`impl_self_type`). Enough for a chain step
/// to bring a method's owner type and the declarations that write the fields it reads.
static V4: Registry = Registry {
    identity: crate::EXTRACTOR_V4,
    languages: &[("rs", "rust"), ("py", "python")],
    extractors: &[("rust", &rust::RUST_V4), ("python", &python::PYTHON_V3)],
    limits: &[
        "Structural declarations only; no behavioral claims or executed-test evidence.",
        "Rust: no macro expansion, cfg evaluation, cross-file name resolution or call graph.",
        "Python: no import resolution, dynamic dispatch, decorator evaluation or runtime behavior; files that do not parse are diagnostics.",
        "Fields: Rust named struct fields; Python class-body assignments and self.<name> assignments in __init__.",
        "Calls, attribute accesses and base classes link to same-named declarations as candidates; receivers and types are not resolved.",
        "Rust: struct literals, the fields they fill, field writes (x.f = ..., x.f += ...) and the type written in an impl are candidates by name; patterns are not observed.",
        "A name with more than 8 same-language declarations of a candidate kind gets no candidate edges; the occurrence records the count.",
        "Test markers are observations (Rust #[test]; Python test*/Test* names), not evidence that tests ran.",
        "File-local scopes; ordinary comments are preserved in snapshots, not attached to declarations.",
        "Files read individually; no atomic repository snapshot or Git/author authentication.",
        "Parse failures preserve bytes and diagnostics; completeness requires inspecting diagnostics.",
        "Hashes establish consistency, not authenticity; manifest is not signed.",
    ],
    graph_version: "rust-python-syntax-graph-v3",
    search_limits: &[
        "Lexical candidates, not a semantic answer. Minimum coverage: 60% of query term groups; scores are not probabilities.",
        "Name candidates are not resolved types/calls/fields/imports. No macros, cfg or decorator evaluation, compiler or runtime proof.",
        "Graph expands only the first hit. Source nodes are leaves. Depth bounded to 0..3, 48 nodes, 96 edges.",
        "Excerpts limited to 4000 UTF-8 bytes each and 16000 total. Truncation is explicit; hashes/lines refer to full chunks.",
    ],
    skipped_dirs: &[
        ".git", ".hg", ".svn", ".venv", "venv", ".tox", ".nox", "__pycache__", ".mypy_cache",
        ".pytest_cache", ".ruff_cache", "site-packages", "node_modules", "target", ".studio",
    ],
    max_candidates: Some(8),
    owner_context: true,
};

/// Step 1 of the road to investigation: v4 plus what the graph did not see being used. Python:
/// module variables, names used as values or written (`Depends(get_db)`, `with _lock:`,
/// `_cache[k] = v`), keyword arguments, and annotations that name a module variable. Rust: enum
/// variants, paths used as values, struct patterns. Both: loops and, in Python, `with` blocks,
/// spanning their bodies. Plain names are kept only when they match a declaration.
static V5: Registry = Registry {
    identity: crate::EXTRACTOR_V5,
    languages: &[("rs", "rust"), ("py", "python")],
    extractors: &[("rust", &rust::RUST_V5), ("python", &python::PYTHON_V5)],
    limits: &[
        "Structural declarations only; no behavioral claims or executed-test evidence.",
        "Rust: no macro expansion, cfg evaluation, cross-file name resolution or call graph.",
        "Python: no import resolution, dynamic dispatch, decorator evaluation or runtime behavior; files that do not parse are diagnostics.",
        "Fields: Rust named struct fields; Python class-body assignments and self.<name> assignments in __init__. Python module variables: names assigned at module level, declared by their first assignment.",
        "Calls, attribute accesses, base classes, names used as values, keyword arguments and patterns link to same-named declarations as candidates; receivers and types are not resolved.",
        "Rust: struct literals, the fields they fill, field writes (x.f = ..., x.f += ...) and the type written in an impl are candidates by name.",
        "A plain name is a use only when it is not bound in its function (parameters, assignments, patterns) and matches a declaration; names in macros, getattr or strings are not seen.",
        "Python: a plain name used as a value or written links only to the top-level declaration of its own file when there is one (module scope); otherwise to every same-named declaration. Imports are not followed.",
        "Python module writes: assignment to a name declared global, and to an item of a module name (x[k] = v); mutating calls (x.update(...)) are reads of the name.",
        "Loops and with blocks are structure: being inside one says nothing about how many times or for how long the code runs.",
        "A name with more than 8 same-language declarations of a candidate kind gets no candidate edges; the occurrence records the count.",
        "Test markers are observations (Rust #[test]; Python test*/Test* names), not evidence that tests ran.",
        "File-local scopes; ordinary comments are preserved in snapshots, not attached to declarations.",
        "Files read individually; no atomic repository snapshot or Git/author authentication.",
        "Parse failures preserve bytes and diagnostics; completeness requires inspecting diagnostics.",
        "Hashes establish consistency, not authenticity; manifest is not signed.",
    ],
    graph_version: "rust-python-syntax-graph-v4",
    search_limits: &[
        "Lexical candidates, not a semantic answer. Minimum coverage: 60% of query term groups; scores are not probabilities.",
        "Name candidates are not resolved types/calls/fields/imports. No macros, cfg or decorator evaluation, compiler or runtime proof.",
        "Graph expands only the first hit. Source nodes are leaves. Depth bounded to 0..3, 48 nodes, 96 edges.",
        "Excerpts limited to 4000 UTF-8 bytes each and 16000 total. Truncation is explicit; hashes/lines refer to full chunks.",
    ],
    skipped_dirs: &[
        ".git", ".hg", ".svn", ".venv", "venv", ".tox", ".nox", "__pycache__", ".mypy_cache",
        ".pytest_cache", ".ruff_cache", "site-packages", "node_modules", "target", ".studio",
    ],
    max_candidates: Some(8),
    owner_context: true,
};

/// Steps 3b and 3c of the road to investigation: v5 plus, in Python, what a loop needs to tell
/// whether it walks a whole collection: parameters (`parameter`, `keyword_parameter`), `x in
/// name` comparisons (`membership`) and names bound to a list in a function (`list_binding`).
/// The Rust adapter is the v5 one.
static V6: Registry = Registry {
    identity: crate::EXTRACTOR_V6,
    languages: &[("rs", "rust"), ("py", "python")],
    extractors: &[("rust", &rust::RUST_V5), ("python", &python::PYTHON_V6)],
    limits: &[
        "Structural declarations only; no behavioral claims or executed-test evidence.",
        "Rust: no macro expansion, cfg evaluation, cross-file name resolution or call graph.",
        "Python: no import resolution, dynamic dispatch, decorator evaluation or runtime behavior; files that do not parse are diagnostics.",
        "Fields: Rust named struct fields; Python class-body assignments and self.<name> assignments in __init__. Python module variables: names assigned at module level, declared by their first assignment.",
        "Calls, attribute accesses, base classes, names used as values, keyword arguments and patterns link to same-named declarations as candidates; receivers and types are not resolved.",
        "Rust: struct literals, the fields they fill, field writes (x.f = ..., x.f += ...) and the type written in an impl are candidates by name.",
        "A plain name is a use only when it is not bound in its function (parameters, assignments, patterns) and matches a declaration; names in macros, getattr or strings are not seen.",
        "Python: a plain name used as a value or written links only to the top-level declaration of its own file when there is one (module scope); otherwise to every same-named declaration. Imports are not followed.",
        "Python module writes: assignment to a name declared global, and to an item of a module name (x[k] = v); mutating calls (x.update(...)) are reads of the name.",
        "Loops and with blocks are structure: being inside one says nothing about how many times or for how long the code runs.",
        "Python: parameters, `x in name` comparisons and names bound to a list literal, list comprehension, list(...) or sorted(...) in a function are recorded as written; a list built otherwise (append, a call's result) is not seen as a list.",
        "A name with more than 8 same-language declarations of a candidate kind gets no candidate edges; the occurrence records the count.",
        "Test markers are observations (Rust #[test]; Python test*/Test* names), not evidence that tests ran.",
        "File-local scopes; ordinary comments are preserved in snapshots, not attached to declarations.",
        "Files read individually; no atomic repository snapshot or Git/author authentication.",
        "Parse failures preserve bytes and diagnostics; completeness requires inspecting diagnostics.",
        "Hashes establish consistency, not authenticity; manifest is not signed.",
    ],
    graph_version: "rust-python-syntax-graph-v5",
    search_limits: &[
        "Lexical candidates, not a semantic answer. Minimum coverage: 60% of query term groups; scores are not probabilities.",
        "Name candidates are not resolved types/calls/fields/imports. No macros, cfg or decorator evaluation, compiler or runtime proof.",
        "Graph expands only the first hit. Source nodes are leaves. Depth bounded to 0..3, 48 nodes, 96 edges.",
        "Excerpts limited to 4000 UTF-8 bytes each and 16000 total. Truncation is explicit; hashes/lines refer to full chunks.",
    ],
    skipped_dirs: &[
        ".git", ".hg", ".svn", ".venv", "venv", ".tox", ".nox", "__pycache__", ".mypy_cache",
        ".pytest_cache", ".ruff_cache", "site-packages", "node_modules", "target", ".studio",
    ],
    max_candidates: Some(8),
    owner_context: true,
};

/// R1 of the renderer phase (step 7 for Rust): v6 plus, in Rust, a closure run once per element
/// of a written iterator chain as a loop, and the type of a method call receiver when it is
/// written (`self` in an impl, an annotated parameter or `let`, `T::f(..)`, `T { .. }`, a field
/// or the element of a field declared in the same file, a tuple variant pattern).
static V7: Registry = Registry {
    identity: crate::EXTRACTOR_V7,
    languages: &[("rs", "rust"), ("py", "python")],
    extractors: &[("rust", &rust::RUST_V6), ("python", &python::PYTHON_V6)],
    limits: &[
        "Structural declarations only; no behavioral claims or executed-test evidence.",
        "Rust: no macro expansion, cfg evaluation, cross-file name resolution or call graph.",
        "Python: no import resolution, dynamic dispatch, decorator evaluation or runtime behavior; files that do not parse are diagnostics.",
        "Fields: Rust named struct fields; Python class-body assignments and self.<name> assignments in __init__. Python module variables: names assigned at module level, declared by their first assignment.",
        "Calls, attribute accesses, base classes, names used as values, keyword arguments and patterns link to same-named declarations as candidates; receivers and types are not resolved.",
        "Rust: struct literals, the fields they fill, field writes (x.f = ..., x.f += ...) and the type written in an impl are candidates by name.",
        "A plain name is a use only when it is not bound in its function (parameters, assignments, patterns) and matches a declaration; names in macros, getattr or strings are not seen.",
        "Python: a plain name used as a value or written links only to the top-level declaration of its own file when there is one (module scope); otherwise to every same-named declaration. Imports are not followed.",
        "Python module writes: assignment to a name declared global, and to an item of a module name (x[k] = v); mutating calls (x.update(...)) are reads of the name.",
        "Rust: a closure passed to for_each, map, filter, fold and the like on a written iterator chain (iter(), par_bridge(), a range, ...) is a loop; one passed to a method on anything else (Option::map) is not.",
        "Rust: a method call receiver gets a receiver_type when its type is written: self in an impl, an annotated parameter or let, T::f(..), T { .. }, a field or the element of a collection field declared in the same file, a tuple variant pattern of an enum declared in the same file. A later binding of a name replaces the earlier in the whole function; blocks are not scopes.",
        "Loops and with blocks are structure: being inside one says nothing about how many times or for how long the code runs.",
        "Python: parameters, `x in name` comparisons and names bound to a list literal, list comprehension, list(...) or sorted(...) in a function are recorded as written; a list built otherwise (append, a call's result) is not seen as a list.",
        "A name with more than 8 same-language declarations of a candidate kind gets no candidate edges; the occurrence records the count.",
        "Test markers are observations (Rust #[test]; Python test*/Test* names), not evidence that tests ran.",
        "File-local scopes; ordinary comments are preserved in snapshots, not attached to declarations.",
        "Files read individually; no atomic repository snapshot or Git/author authentication.",
        "Parse failures preserve bytes and diagnostics; completeness requires inspecting diagnostics.",
        "Hashes establish consistency, not authenticity; manifest is not signed.",
    ],
    graph_version: "rust-python-syntax-graph-v5",
    search_limits: &[
        "Lexical candidates, not a semantic answer. Minimum coverage: 60% of query term groups; scores are not probabilities.",
        "Name candidates are not resolved types/calls/fields/imports. No macros, cfg or decorator evaluation, compiler or runtime proof.",
        "Graph expands only the first hit. Source nodes are leaves. Depth bounded to 0..3, 48 nodes, 96 edges.",
        "Excerpts limited to 4000 UTF-8 bytes each and 16000 total. Truncation is explicit; hashes/lines refer to full chunks.",
    ],
    skipped_dirs: &[
        ".git", ".hg", ".svn", ".venv", "venv", ".tox", ".nox", "__pycache__", ".mypy_cache",
        ".pytest_cache", ".ruff_cache", "site-packages", "node_modules", "target", ".studio",
    ],
    max_candidates: Some(8),
    owner_context: true,
};

/// R5 of the renderer phase: v7 plus, in Rust, the written type of the base of a field access or
/// write (`group.transform = ..` when `group` has a written type), so that a write through
/// another receiver can be told apart from writes to same-named fields of other types.
static V8: Registry = Registry {
    identity: crate::EXTRACTOR_V8,
    languages: &[("rs", "rust"), ("py", "python")],
    extractors: &[("rust", &rust::RUST_V7), ("python", &python::PYTHON_V6)],
    limits: &[
        "Structural declarations only; no behavioral claims or executed-test evidence.",
        "Rust: no macro expansion, cfg evaluation, cross-file name resolution or call graph.",
        "Python: no import resolution, dynamic dispatch, decorator evaluation or runtime behavior; files that do not parse are diagnostics.",
        "Fields: Rust named struct fields; Python class-body assignments and self.<name> assignments in __init__. Python module variables: names assigned at module level, declared by their first assignment.",
        "Calls, attribute accesses, base classes, names used as values, keyword arguments and patterns link to same-named declarations as candidates; receivers and types are not resolved.",
        "Rust: struct literals, the fields they fill, field writes (x.f = ..., x.f += ...) and the type written in an impl are candidates by name.",
        "A plain name is a use only when it is not bound in its function (parameters, assignments, patterns) and matches a declaration; names in macros, getattr or strings are not seen.",
        "Python: a plain name used as a value or written links only to the top-level declaration of its own file when there is one (module scope); otherwise to every same-named declaration. Imports are not followed.",
        "Python module writes: assignment to a name declared global, and to an item of a module name (x[k] = v); mutating calls (x.update(...)) are reads of the name.",
        "Rust: a closure passed to for_each, map, filter, fold and the like on a written iterator chain (iter(), par_bridge(), a range, ...) is a loop; one passed to a method on anything else (Option::map) is not.",
        "Rust: a method call receiver gets a receiver_type when its type is written: self in an impl, an annotated parameter or let, T::f(..), T { .. }, a field or the element of a collection field declared in the same file, a tuple variant pattern of an enum declared in the same file; the base of a field access or write gets one the same way. A later binding of a name replaces the earlier in the whole function; blocks are not scopes.",
        "Loops and with blocks are structure: being inside one says nothing about how many times or for how long the code runs.",
        "Python: parameters, `x in name` comparisons and names bound to a list literal, list comprehension, list(...) or sorted(...) in a function are recorded as written; a list built otherwise (append, a call's result) is not seen as a list.",
        "A name with more than 8 same-language declarations of a candidate kind gets no candidate edges; the occurrence records the count.",
        "Test markers are observations (Rust #[test]; Python test*/Test* names), not evidence that tests ran.",
        "File-local scopes; ordinary comments are preserved in snapshots, not attached to declarations.",
        "Files read individually; no atomic repository snapshot or Git/author authentication.",
        "Parse failures preserve bytes and diagnostics; completeness requires inspecting diagnostics.",
        "Hashes establish consistency, not authenticity; manifest is not signed.",
    ],
    graph_version: "rust-python-syntax-graph-v5",
    search_limits: &[
        "Lexical candidates, not a semantic answer. Minimum coverage: 60% of query term groups; scores are not probabilities.",
        "Name candidates are not resolved types/calls/fields/imports. No macros, cfg or decorator evaluation, compiler or runtime proof.",
        "Graph expands only the first hit. Source nodes are leaves. Depth bounded to 0..3, 48 nodes, 96 edges.",
        "Excerpts limited to 4000 UTF-8 bytes each and 16000 total. Truncation is explicit; hashes/lines refer to full chunks.",
    ],
    skipped_dirs: &[
        ".git", ".hg", ".svn", ".venv", "venv", ".tox", ".nox", "__pycache__", ".mypy_cache",
        ".pytest_cache", ".ruff_cache", "site-packages", "node_modules", "target", ".studio",
    ],
    max_candidates: Some(8),
    owner_context: true,
};

impl Registry {
    /// Profile used for new editions.
    pub fn current() -> &'static Self {
        &V8
    }

    /// Profile that produced an edition; unknown identities are refused, never upgraded.
    pub fn for_identity(identity: &str) -> Option<&'static Self> {
        [&V1, &V2, &V3, &V4, &V5, &V6, &V7, &V8].into_iter().find(|r| r.identity == identity)
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
