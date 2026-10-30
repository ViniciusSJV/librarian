//! G5: Python enters only through its extractor and the v2 registry profile.
//! A mixed Rust + Python project goes through edition, graph, search and prepare unchanged.
use librarian_ingest::{
    generate_with_profile,
    graph::{v2, Graph},
    prepare::{self, Request},
    search::{Lexicon, SearchIndex},
    Catalog, Config, EXTRACTOR_V2,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "librarian-python-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for sub in ["", "src", "pkg", "pkg/sub", "pkg/.venv/lib", "pkg/__pycache__"] {
            fs::create_dir_all(dir.join(sub)).unwrap();
        }
        fs::write(dir.join("src/lib.rs"), "pub struct Camera;\nimpl Camera {\n    pub fn ray(&self) -> u8 { 1 }\n}\n").unwrap();
        fs::write(
            dir.join("pkg/camera.py"),
            "\u{feff}import math\n\n\
             class Camera(Base):\n    \
                 \"\"\"A camera.\"\"\"\n\n    \
                 @property\n    \
                 def size(self) -> int:\n        \
                     return self.lens.size()\n\n    \
                 @staticmethod\n    \
                 @cache\n    \
                 def ray_from_pixel(x: float, y: \"Pixel\") -> \"Ray\":\n        \
                     direction = normalize(x - y)\n        \
                     return Ray(origin=make(x), direction=direction)\n\n\
             async def fetch(url: str, client: Optional[Client] = None) -> Camera:\n    \
                 def helper():\n        \
                     return url\n    \
                 with open(url) as handle:\n        \
                     pass\n    \
                 match url:\n        \
                     case \"a\":\n            \
                         helper()\n    \
                 return [load(u) for u in [url]]\n",
        )
        .unwrap();
        fs::write(dir.join("pkg/sub/test_camera.py"), "class TestCamera:\n    def test_ray(self):\n        assert True\n\ndef test_direction():\n    pass\n").unwrap();
        fs::write(dir.join("pkg/broken.py"), "def broken(:\n    pass\n").unwrap();
        fs::write(dir.join("pkg/.venv/lib/vendored.py"), "def vendored():\n    pass\n").unwrap();
        fs::write(dir.join("pkg/__pycache__/cached.py"), "def cached():\n    pass\n").unwrap();
        Self(dir)
    }
    fn edition(&self) -> Catalog {
        let config = Config { project: "mixed".into(), inputs: vec!["src".into(), "pkg".into()] };
        let edition = self.0.join("edition");
        let _ = fs::remove_dir_all(&edition);
        // G5 is the v2 profile; the current profile (v3, G6) is covered by tests/g6.rs.
        generate_with_profile(EXTRACTOR_V2, &self.0, &config, &edition).unwrap();
        Catalog::load(&edition).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn symbol<'a>(catalog: &'a Catalog, qualified: &str) -> &'a librarian_ingest::Symbol {
    catalog
        .symbols()
        .values()
        .find(|s| s.qualified_name == qualified)
        .unwrap_or_else(|| panic!("missing {qualified}"))
}

#[test]
fn python_declarations_are_extracted_with_decorators_and_scopes() {
    let project = Project::new();
    let catalog = project.edition();
    assert_eq!(catalog.manifest().extractor, EXTRACTOR_V2);
    let paths: Vec<_> = catalog.sources().values().map(|s| (s.path.as_str(), s.language.as_str())).collect();
    // Environments and caches are not traversed; Rust and Python are both classified.
    assert!(paths.contains(&("src/lib.rs", "rust")));
    assert!(paths.contains(&("pkg/camera.py", "python")));
    assert!(!paths.iter().any(|(p, _)| p.contains(".venv") || p.contains("__pycache__")));

    let kinds = |q: &str| symbol(&catalog, q).kind.clone();
    assert_eq!(kinds("Camera"), "class");
    assert_eq!(kinds("Camera.size"), "method");
    assert_eq!(kinds("Camera.ray_from_pixel"), "method");
    assert_eq!(kinds("fetch"), "function");
    assert_eq!(kinds("fetch.helper"), "function");
    assert_eq!(kinds("impl Camera::ray"), "method");

    // The chunk starts at the first decorator and keeps original bytes (after the BOM).
    let ray = symbol(&catalog, "Camera.ray_from_pixel");
    let excerpt = std::str::from_utf8(catalog.excerpt(&ray.chunk_id).unwrap()).unwrap();
    assert!(excerpt.starts_with("@staticmethod\n    @cache\n    def ray_from_pixel("), "{excerpt}");
    assert!(excerpt.ends_with("direction=direction)"));
    let chunk = catalog.chunk(&ray.chunk_id).unwrap();
    assert_eq!((chunk.start_line, chunk.end_line), (10, 14));

    // Test markers are naming observations.
    assert!(symbol(&catalog, "TestCamera.test_ray").is_test);
    assert!(symbol(&catalog, "test_direction").is_test);
    assert!(!symbol(&catalog, "fetch").is_test);

    // A file that does not parse is a diagnostic; its bytes are still preserved.
    let codes: Vec<_> = catalog.diagnostics().iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["python_parse_error"]);
    assert!(catalog.source_by_path("pkg/broken.py").is_some());
}

#[test]
fn python_occurrences_and_candidates_stay_within_the_language() {
    let project = Project::new();
    let catalog = project.edition();
    let graph = Graph::build(&catalog);
    let kind_of = |e: &librarian_ingest::graph::Edge| graph.nodes[&e.to].kind.clone();
    let observed: std::collections::BTreeSet<_> = graph
        .edges
        .iter()
        .filter(|e| e.evidence.path == "pkg/camera.py" && e.resolution == "syntax_only")
        .map(kind_of)
        .collect();
    for kind in ["call_observed", "method_call_observed", "type_reference", "declared_return", "local_binding"] {
        assert!(observed.contains(kind), "{kind} not observed; got {observed:?}");
    }
    // `-> Camera` in Python may point to the Python class, never to the Rust struct.
    let python_camera = symbol(&catalog, "Camera").id.clone();
    let rust_camera = catalog.symbols().values().find(|s| s.qualified_name == "Camera" && s.kind == "struct").unwrap().id.clone();
    let candidates: Vec<_> = graph.edges.iter().filter(|e| e.resolution == "name_candidate").collect();
    assert!(candidates.iter().any(|e| e.to == python_camera && e.evidence.path == "pkg/camera.py"));
    assert!(!candidates.iter().any(|e| e.to == rust_camera && e.evidence.path.ends_with(".py")));
    assert!(!candidates.iter().any(|e| e.to == python_camera && e.evidence.path.ends_with(".rs")));

    graph.verify(&catalog).unwrap();
    let projected = v2::Graph::build(&catalog);
    assert_eq!(projected.derived_from, "rust-python-syntax-graph-v1");
    projected.verify(&catalog).unwrap();
    assert!(projected.nodes.values().any(|n| n.language == "python"));
}

#[test]
fn python_methods_are_searched_and_prepared_like_rust() {
    let project = Project::new();
    let catalog = project.edition();
    let lexicon = Lexicon {
        groups: vec![vec!["raios".into(), "ray".into()], vec!["direcao".into(), "direction".into()]],
        stop_words: vec![],
    };
    let report = SearchIndex::new(&catalog, lexicon.clone())
        .unwrap()
        .search("direção dos raios", 3, 1)
        .unwrap();
    // The class is navigation (it spans its methods); the method is the answer.
    assert_eq!(report.hits[0].name, "Camera.ray_from_pixel");
    assert!(!report.hits.iter().any(|h| h.name == "Camera"));

    let request = Request {
        question: "direção dos raios".into(),
        lexicon,
        top: 3,
        depth: 1,
        hits: 1,
        radius: 0,
        numbered_lines: true,
        instructions: None,
        accept_diagnostics: true,
        targets: None,
        reviewed_facts: vec![],
        owner_context: false,
        exclude_tests: false,
        picked: false,
        callees: false,
        gathered: false,
        headers: vec![],
    };
    let prepared = prepare::prepare(&catalog, &request).unwrap();
    let dossier: serde_json::Value = serde_json::from_str(&prepared.dossier).unwrap();
    assert_eq!(dossier["sources"][0]["kind"], "python");
    assert_eq!(dossier["sources"][0]["content"], "code");
    assert_eq!(
        dossier["facts"][0]["statement"],
        "method `Camera.ray_from_pixel` declarado em pkg/camera.py, linhas 10–14."
    );
    let query: serde_json::Value = serde_json::from_str(prepared.query.query_json().unwrap()).unwrap();
    let lines = &query["evidence"]["source"]["context"]["lines"];
    assert_eq!(lines[0]["number"], 10);
    assert_eq!(lines[0]["text"], "    @staticmethod");
}

#[test]
fn vocabulary_lists_name_tokens_by_frequency() {
    let project = Project::new();
    let catalog = project.edition();
    let vocabulary = librarian_ingest::search::vocabulary(&catalog);
    let count = |t: &str| vocabulary.iter().find(|(w, _)| w == t).map(|(_, n)| *n);
    // "camera" names the Python class, the Rust struct, impl and test class tokens.
    assert!(count("camera").unwrap() >= 3);
    assert_eq!(count("pixel"), Some(1));
    // Short and numeric tokens are left out; the order is by frequency, then alphabetical.
    assert!(vocabulary.iter().all(|(w, _)| w.chars().count() >= 3));
    assert!(vocabulary.windows(2).all(|p| p[0].1 > p[1].1 || (p[0].1 == p[1].1 && p[0].0 < p[1].0)));
}
