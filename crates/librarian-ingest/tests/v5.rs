//! Profile v5 (step 1 of the road to investigation): uses the graph did not see. Python module
//! variables, names used as values or written, keyword arguments and annotations naming a
//! module variable; Rust variants, paths used as values and struct patterns; loops and `with`
//! blocks. All by written names: a plain name counts only when it is not local and matches.
use librarian_ingest::{
    generate, generate_with_profile,
    graph::{v2, Graph},
    Catalog, Config, EXTRACTOR_V4, EXTRACTOR_V5,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

// The dependency of a FastAPI project, a lock and a cache at module level.
const DEPS: &str = "from typing import Annotated
from threading import Lock

from fastapi import Depends

_lock = Lock()
_cache = {}
counter = 0


def get_db():
    yield 1


SessionDep = Annotated[Session, Depends(get_db)]


def route(session: SessionDep):
    with _lock:
        for d in session.items():
            _cache[d] = zone_of(d)
    return [x for x in _cache]


def bump():
    global counter
    counter += 1


def shadow(get_db):
    return get_db


def build():
    return Delivery(day=3, missing=1)


def zone_of(d):
    return d
";

const MODELS: &str = "class Delivery:
    day: int = 0


_lock = None


def other():
    return _lock
";

const SHAPES: &str = "const MAX: u32 = 3;

pub enum Shape {
    Sphere(f64),
    Plane,
}

pub struct Camera {
    pub transform: f64,
}

fn helper(x: u32) -> u32 {
    x
}

pub fn run(c: Camera, s: Shape, items: Vec<u32>) -> u32 {
    let Camera { transform } = c;
    let mut total = MAX;
    for i in items.iter() {
        total += helper(*i);
    }
    while total > 10 {
        total -= 1;
    }
    loop {
        break;
    }
    let f = helper;
    let k = match s {
        Shape::Sphere(r) => r as u32,
        Shape::Plane => 0,
    };
    let _ = Shape::Sphere(1.0);
    total + k + f(1) + transform as u32
}
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-v5-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("app")).unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("app/deps.py"), DEPS).unwrap();
        fs::write(dir.join("app/models.py"), MODELS).unwrap();
        fs::write(dir.join("src/shapes.rs"), SHAPES).unwrap();
        Self(dir)
    }

    fn edition(&self, name: &str, profile: Option<&str>) -> Catalog {
        let path = self.0.join(name);
        let config = Config { project: "v5".into(), inputs: vec!["app".into(), "src".into()] };
        match profile {
            Some(p) => generate_with_profile(p, &self.0, &config, &path).unwrap(),
            None => generate(&self.0, &config, &path).unwrap(),
        };
        Catalog::load(&path).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn id(catalog: &Catalog, qualified: &str, kind: &str) -> String {
    let found: Vec<_> = catalog.symbols().values().filter(|s| s.qualified_name == qualified && s.kind == kind).collect();
    assert_eq!(found.len(), 1, "{qualified} ({kind})");
    found[0].id.clone()
}

fn labels(graph: &Graph, kind: &str) -> Vec<String> {
    let mut found: Vec<String> = graph.nodes.values().filter(|n| n.kind == kind).map(|n| n.label.clone()).collect();
    found.sort();
    found
}

/// Who uses a symbol, as `label@layer via kind`, sorted.
fn users(graph: &v2::Graph, seed: &str, depth: usize) -> Vec<String> {
    let used = graph.used_by(seed, depth, 100).unwrap();
    let layer = |id: &str| used.nodes.iter().find(|n| n.id == id).map(|n| (n.label.clone(), n.layer)).unwrap();
    let mut found: Vec<String> = used
        .links
        .iter()
        .map(|l| {
            let (label, layer) = layer(&l.from);
            format!("{label}@{layer} via {}", l.occurrence_kind)
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

#[test]
fn python_module_names_are_declared_and_their_uses_seen() {
    let project = Project::new();
    let catalog = project.edition("v5", Some(EXTRACTOR_V5));
    assert_eq!(catalog.manifest().extractor, EXTRACTOR_V5);
    let mut variables: Vec<&str> = catalog.symbols().values().filter(|s| s.kind == "variable").map(|s| s.name.as_str()).collect();
    variables.sort();
    assert_eq!(variables, ["SessionDep", "_cache", "_lock", "_lock", "counter"]);
    let graph = v2::Graph::build(&catalog);
    let lock = |path: &str| catalog.symbols().values().find(|s| s.name == "_lock" && catalog.sources()[&s.source_id].path == path).unwrap().id.clone();

    // Depends(get_db) is a use of get_db, and the routes reach it through the annotation.
    // `shadow` has a parameter named get_db: its `get_db` is local, not a use.
    assert_eq!(users(&graph, &id(&catalog, "get_db", "function"), 2), [
        "SessionDep@1 via name_reference",
        "route@2 via type_reference",
    ]);
    // Each module's `_lock` is its own: a plain name means the declaration of its file.
    assert_eq!(users(&graph, &lock("app/deps.py"), 1), ["route@1 via name_reference"]);
    assert_eq!(users(&graph, &lock("app/models.py"), 1), ["other@1 via name_reference"]);
    // The cache is written by item and read by the comprehension.
    assert_eq!(users(&graph, &id(&catalog, "_cache", "variable"), 1), ["route@1 via name_reference", "route@1 via name_write"]);
    // `global counter` makes `counter += 1` a write of the module name.
    assert_eq!(users(&graph, &id(&catalog, "counter", "variable"), 1), ["bump@1 via name_write"]);
    // A keyword argument fills the field it names; a name that matches no field leaves nothing.
    assert_eq!(users(&graph, &id(&catalog, "Delivery.day", "field"), 1), ["build@1 via keyword_argument"]);
    // A called name is a call, not also a name used as a value.
    assert_eq!(users(&graph, &id(&catalog, "zone_of", "function"), 1), ["route@1 via call_observed"]);
}

#[test]
fn loops_and_with_blocks_span_their_bodies_and_are_labelled_by_their_heads() {
    let project = Project::new();
    let catalog = project.edition("v5", Some(EXTRACTOR_V5));
    let graph = Graph::build(&catalog);
    assert_eq!(graph.version, "rust-python-syntax-graph-v4");
    assert_eq!(labels(&graph, "loop"), [
        "[x for x in _cache]",
        "for d in session.items()",
        "for i in items.iter()",
        "loop",
        "while total > 10",
    ]);
    assert_eq!(labels(&graph, "with_block"), ["with _lock"]);
    // The block's evidence covers its body: the write to the cache is inside the loop and the with.
    let span = |kind: &str, label: &str| graph.edges.iter().find(|e| e.kind == kind && graph.nodes[&e.to].label == label).unwrap().evidence.clone();
    let (with, for_loop, write) = (span("with_block", "with _lock"), span("loop", "for d in session.items()"), span("name_write", "_cache"));
    assert!(with.start_line < for_loop.start_line && for_loop.end_line <= with.end_line);
    assert!(for_loop.start_byte <= write.start_byte && write.end_byte <= for_loop.end_byte);
}

#[test]
fn rust_variants_values_and_patterns_are_uses() {
    let project = Project::new();
    let catalog = project.edition("v5", Some(EXTRACTOR_V5));
    let graph = v2::Graph::build(&catalog);
    assert_eq!(users(&graph, &id(&catalog, "MAX", "const"), 1), ["run@1 via name_reference"]);
    // A tuple variant is matched in a pattern and built by a call; a unit variant by its path.
    assert_eq!(users(&graph, &id(&catalog, "Shape::Sphere", "variant"), 1), ["run@1 via call_observed", "run@1 via struct_pattern"]);
    assert_eq!(users(&graph, &id(&catalog, "Shape::Plane", "variant"), 1), ["run@1 via name_reference"]);
    // `let f = helper;` uses the function as a value; `helper(*i)` is a call.
    assert_eq!(users(&graph, &id(&catalog, "helper", "function"), 1), ["run@1 via call_observed", "run@1 via name_reference"]);
    // Taking a struct apart reads its fields.
    assert_eq!(users(&graph, &id(&catalog, "Camera::transform", "field"), 1), ["run@1 via field_pattern"]);
    assert_eq!(users(&graph, &id(&catalog, "Camera", "struct"), 1), ["run@1 via struct_pattern", "run@1 via type_reference"]);
    // Locals (`total`, `f`, `r`) are never name references.
    let base = Graph::build(&catalog);
    let references = labels(&base, "name_reference");
    assert!(!references.iter().any(|l| ["total", "f", "r", "k", "transform"].contains(&l.as_str())), "{references:?}");
}

#[test]
fn the_v4_profile_is_still_reproduced_without_the_new_records() {
    let project = Project::new();
    let catalog = project.edition("v4", Some(EXTRACTOR_V4));
    assert_eq!(catalog.manifest().extractor, EXTRACTOR_V4);
    assert!(!catalog.symbols().values().any(|s| s.kind == "variable" || s.kind == "variant"));
    let graph = Graph::build(&catalog);
    assert_eq!(graph.version, "rust-python-syntax-graph-v3");
    for kind in ["name_reference", "name_write", "keyword_argument", "loop", "with_block", "struct_pattern", "field_pattern"] {
        assert!(labels(&graph, kind).is_empty(), "{kind}");
    }
    graph.verify(&catalog).unwrap();
    let current = project.edition("v5", Some(EXTRACTOR_V5));
    v2::Graph::build(&current).verify(&current).unwrap();
}
