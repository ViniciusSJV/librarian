//! G6d (profile v4): in Rust, struct literals, field writes and the type written in an impl are
//! occurrences; a chain step with owner context brings a method's owner type and the
//! declarations that write the fields it reads. All by written names, never resolved.
use librarian_ingest::{
    generate, generate_with_profile,
    graph::{v2, Graph},
    prepare::{self, Request},
    Catalog, Config, EXTRACTOR_V3, EXTRACTOR_V4,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const CAMERA: &str = "pub struct Matrix {
    pub v: f64,
}

impl Matrix {
    pub fn identity() -> Self {
        Matrix { v: 1.0 }
    }
    pub fn inverse(&self) -> Matrix {
        Matrix { v: 1.0 / self.v }
    }
}

pub struct Camera {
    pub transform: Matrix,
    pixel_size: f64,
    depth: u8,
}

impl Camera {
    pub fn new(pixel_size: f64) -> Self {
        Self { transform: Matrix::identity(), pixel_size, depth: 4 }
    }
    pub fn with_transform(mut self, transform: Matrix) -> Self {
        self.transform = transform;
        self
    }
    pub fn with_depth(mut self, depth: u8) -> Self {
        self.depth = depth;
        self
    }
    pub fn ray(&self, x: f64) -> f64 {
        let inverse = self.transform.inverse();
        inverse.v * x * self.pixel_size
    }
}
";

const SCENE: &str = "pub struct Sphere {
    pub transform: f64,
}

impl Sphere {
    pub fn set(&mut self, t: f64) {
        self.transform = t;
    }
}

pub fn scene(mut c: Camera, mut s: Sphere) -> Camera {
    c.transform = Matrix::identity();
    s.transform += 1.0;
    Camera { transform: Matrix::identity(), pixel_size: 1.0, depth: 1 }
}
";

const SHAPES: &str = "class Shape:
    def __init__(self, t):
        self.transform = t
        self.cache = None

    def move(self, d):
        self.transform = self.transform + d

    def point(self, x):
        return self.transform * x
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-g6d-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("src/camera.rs"), CAMERA).unwrap();
        fs::write(dir.join("src/scene.rs"), SCENE).unwrap();
        fs::write(dir.join("pkg/shapes.py"), SHAPES).unwrap();
        Self(dir)
    }

    fn edition(&self, name: &str, profile: Option<&str>) -> Catalog {
        let path = self.0.join(name);
        let _ = fs::remove_dir_all(&path);
        let config = Config { project: "g6d".into(), inputs: vec!["src".into(), "pkg".into()] };
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

fn labels(graph: &Graph, kind: &str) -> Vec<String> {
    let mut found: Vec<String> = graph.nodes.values().filter(|n| n.kind == kind).map(|n| n.label.clone()).collect();
    found.sort();
    found
}

fn symbol_id(catalog: &Catalog, qualified: &str, kind: &str) -> String {
    let found: Vec<_> = catalog.symbols().values().filter(|s| s.qualified_name == qualified && s.kind == kind).collect();
    assert_eq!(found.len(), 1, "{qualified} ({kind})");
    found[0].id.clone()
}

#[test]
fn literals_writes_and_impl_types_are_occurrences() {
    let project = Project::new();
    let catalog = project.edition("v4", None);
    assert_eq!(catalog.manifest().extractor, EXTRACTOR_V4);
    let graph = Graph::build(&catalog);
    assert_eq!(graph.version, "rust-python-syntax-graph-v3");

    assert_eq!(labels(&graph, "impl_self_type"), ["Camera", "Matrix", "Sphere"]);
    // The impl's own type is not also a type reference of the impl.
    let impl_camera = symbol_id(&catalog, "impl Camera", "impl");
    assert!(!graph.edges.iter().any(|e| e.from == impl_camera && graph.nodes[&e.to].kind == "type_reference"));
    // Writes are `field_assign`, not also accesses; reads stay accesses. Python keeps the v3
    // adapter, where `self.transform` (three times in shapes.py) is an access.
    assert_eq!(labels(&graph, "field_assign"), ["c.transform", "s.transform", "self.depth", "self.transform", "self.transform"]);
    assert_eq!(labels(&graph, "attribute_access"), ["inverse.v", "self.pixel_size", "self.transform", "self.transform", "self.transform", "self.transform", "self.v"]);
    // Literals and the fields they fill, shorthand included.
    assert_eq!(labels(&graph, "field_init"), ["depth", "depth", "pixel_size", "pixel_size", "transform", "transform", "v", "v"]);
    let literals = labels(&graph, "struct_literal");
    assert!(literals.contains(&"Self { transform: Matrix::identity(), pixel_size, depth: 4 }".to_owned()));
    // Candidates by name: a Camera literal reaches the struct; `Self` reaches nothing.
    let camera = symbol_id(&catalog, "Camera", "struct");
    let reaches = |label: &str| -> Vec<String> {
        let ids: Vec<&String> = graph.nodes.values().filter(|n| n.kind == "struct_literal" && n.label.starts_with(label)).map(|n| &n.id).collect();
        graph.edges.iter().filter(|e| e.kind == "name_candidate" && ids.contains(&&e.from)).map(|e| e.to.clone()).collect()
    };
    assert_eq!(reaches("Camera {"), [camera.clone()]);
    assert!(reaches("Self {").is_empty());
    // `s.transform += 1.0` matches both fields named transform; nothing is chosen.
    let compound = graph.nodes.values().find(|n| n.kind == "field_assign" && n.label == "s.transform").unwrap();
    let mut targets: Vec<_> = graph.edges.iter().filter(|e| e.from == compound.id).map(|e| e.to.clone()).collect();
    targets.sort();
    let mut fields = vec![symbol_id(&catalog, "Camera::transform", "field"), symbol_id(&catalog, "Sphere::transform", "field")];
    fields.sort();
    assert_eq!(targets, fields);

    // "Onde é usado?" now sees who builds a Camera and who writes its fields.
    let projected = v2::Graph::build(&catalog);
    let used = projected.used_by(&camera, 1, 50).unwrap();
    let mut kinds: Vec<_> = used.links.iter().map(|l| (l.occurrence_kind.as_str(), l.text.as_str())).collect();
    kinds.sort();
    assert!(kinds.contains(&("impl_self_type", "Camera")));
    assert!(kinds.iter().any(|(k, t)| *k == "struct_literal" && t.starts_with("Camera {")));

    // The v3 profile reads the same project without the new kinds.
    let old = Graph::build(&project.edition("v3", Some(EXTRACTOR_V3)));
    for kind in ["impl_self_type", "field_assign", "field_init", "struct_literal"] {
        assert!(labels(&old, kind).is_empty(), "{kind} in v3");
    }
    assert!(labels(&old, "attribute_access").contains(&"self.depth".to_owned()));
}

fn chain(target: &str, line: usize, path: &str, owner_context: bool) -> Value {
    json!({
        "question": "Como reduzir o custo de ray?",
        "top": 5, "depth": 1, "hits": 1, "radius": 0, "numbered_lines": true,
        "targets": [{"path": path, "symbol_name": target, "declaration_line": line}],
        "owner_context": owner_context
    })
}

fn prepare_value(catalog: &Catalog, request: Value) -> Result<prepare::Prepared, String> {
    let request: Request = serde_json::from_value(request).unwrap();
    prepare::prepare(catalog, &request).map_err(|e| e.to_string())
}

#[test]
fn owner_context_brings_the_type_and_the_writers_of_read_fields() {
    let project = Project::new();
    let catalog = project.edition("v4", None);
    let prepared = prepare_value(&catalog, chain("ray", 32, "src/camera.rs", true)).unwrap();
    let search: Value = serde_json::from_slice(&prepared.search).unwrap();
    let names: Vec<&str> = search["hits"].as_array().unwrap().iter().map(|h| h["name"].as_str().unwrap()).collect();
    // The target, its owner, and the writers of the fields it reads, in source order. `with_depth`
    // writes a field `ray` does not read, and `Sphere::set` writes a homonym in another type.
    assert_eq!(names, ["impl Camera::ray", "Camera", "impl Camera::new", "impl Camera::with_transform", "scene"]);
    let context = &search["owner_context"];
    let added: Vec<(&str, &str, Value)> = context["added"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| (a["reason"].as_str().unwrap(), a["resolution"].as_str().unwrap(), a["fields"].clone()))
        .collect();
    assert_eq!(added, [
        ("owner_type", "container_name", json!(["pixel_size", "transform"])),
        ("field_writer", "owner_literal", json!(["pixel_size", "transform"])),
        ("field_writer", "receiver_in_owner", json!(["transform"])),
        ("field_writer", "owner_literal", json!(["pixel_size", "transform"])),
    ]);
    // Each link carries where it is written: `impl Camera` (line 20), `self.transform =` (25).
    assert_eq!(context["added"][0]["evidence"][0]["start_line"], 20);
    assert_eq!(context["added"][2]["evidence"][0]["start_line"], 25);
    // Writes through other receivers are counted and listed, never added.
    assert_eq!(context["not_followed_total"], 2);
    let listed: Vec<&str> = context["not_followed"].as_array().unwrap().iter().map(|n| n["text"].as_str().unwrap()).collect();
    assert_eq!(listed, ["c.transform", "s.transform"]);
    assert_eq!(context["truncated"], false);

    // The dossier: one structural fact per declaration, saying why it is there.
    let dossier: Value = serde_json::from_str(&prepared.dossier).unwrap();
    let statements: Vec<&str> = dossier["facts"].as_array().unwrap().iter().map(|f| f["statement"].as_str().unwrap()).collect();
    assert_eq!(statements.len(), 5);
    assert!(statements[0].starts_with("method `impl Camera::ray`") && !statements[0].contains("Contexto do dono"));
    assert!(statements[1].contains("struct `Camera`") && statements[1].contains("campos que ele lê: `pixel_size`, `transform`"));
    assert!(statements[3].contains("escreve `transform` pelo próprio receptor (linha 25)"));
    let unknowns = dossier["unknowns"].to_string();
    assert!(unknowns.contains("Contexto do dono") && unknowns.contains("2 escritas"));
    assert_eq!(prepared.query.selected_fact_ids(), ["E1", "E2", "E3", "E4", "E5"]);

    // Written and verified like any chain step; without the option the step is unchanged.
    let dir = project.0.join("prepared");
    prepare::write(&catalog, &serde_json::to_vec_pretty(&chain("ray", 32, "src/camera.rs", true)).unwrap(), &dir).unwrap();
    prepare::verify(&catalog, &dir).unwrap();
    let plain = prepare_value(&catalog, chain("ray", 32, "src/camera.rs", false)).unwrap();
    let search: Value = serde_json::from_slice(&plain.search).unwrap();
    assert_eq!(search["hits"].as_array().unwrap().len(), 1);
    assert!(search.get("owner_context").is_none());
    let request = serde_json::to_string(&serde_json::from_value::<Request>(chain("ray", 32, "src/camera.rs", false)).unwrap()).unwrap();
    assert!(!request.contains("owner_context"));
}

#[test]
fn owner_context_in_python_is_the_enclosing_class() {
    let project = Project::new();
    let catalog = project.edition("v4", None);
    let prepared = prepare_value(&catalog, chain("point", 9, "pkg/shapes.py", true)).unwrap();
    let search: Value = serde_json::from_slice(&prepared.search).unwrap();
    let names: Vec<&str> = search["hits"].as_array().unwrap().iter().map(|h| h["name"].as_str().unwrap()).collect();
    // The class spans its methods, so the writers are already inside it.
    assert_eq!(names, ["Shape.point", "Shape"]);
    let added = &search["owner_context"]["added"][0];
    assert_eq!((added["resolution"].as_str(), added["fields"].clone()), (Some("container"), json!(["transform"])));
}

#[test]
fn owner_context_is_refused_where_it_cannot_be_told() {
    let project = Project::new();
    // Editions of earlier profiles do not record owner syntax.
    let old = project.edition("v3", Some(EXTRACTOR_V3));
    assert!(prepare_value(&old, chain("ray", 32, "src/camera.rs", true)).err().unwrap().contains("v4"));
    // Only a chain step carries it.
    let catalog = project.edition("v4", None);
    let mut lexical = chain("ray", 32, "src/camera.rs", true);
    lexical.as_object_mut().unwrap().remove("targets");
    assert!(prepare_value(&catalog, lexical).err().unwrap().contains("only by a chain"));
    // A free function has no owner: nothing is added.
    let prepared = prepare_value(&catalog, chain("scene", 11, "src/scene.rs", true)).unwrap();
    let search: Value = serde_json::from_slice(&prepared.search).unwrap();
    assert_eq!(search["hits"].as_array().unwrap().len(), 1);
    assert_eq!(search["owner_context"]["added"], json!([]));
}

#[test]
fn callees_bring_what_the_target_calls_without_ambiguity() {
    let project = Project::new();
    let catalog = project.edition("v4", None);
    let mut request = chain("ray", 32, "src/camera.rs", true);
    request["callees"] = json!(true);
    let prepared = prepare_value(&catalog, request.clone()).unwrap();
    let search: Value = serde_json::from_slice(&prepared.search).unwrap();
    // `ray` calls `self.transform.inverse()`: one declaration has that name, so it comes too,
    // after the owner context, with the call's line as its evidence.
    let added = search["owner_context"]["added"].as_array().unwrap();
    let called: Vec<(&str, &str)> = added.iter().filter(|a| a["reason"] == "called")
        .map(|a| (a["name"].as_str().unwrap(), a["resolution"].as_str().unwrap())).collect();
    assert_eq!(called, [("inverse", "unique")]);
    assert_eq!(added.last().unwrap()["evidence"][0]["start_line"], 33);
    assert!(search["limits"].to_string().contains("Called declarations"));
    let dossier: Value = serde_json::from_str(&prepared.dossier).unwrap();
    assert!(dossier["facts"].to_string().contains("Chamada por `impl Camera::ray`"));
    // Written and verified like any chain step; the option needs the owner context.
    let dir = project.0.join("prepared-callees");
    prepare::write(&catalog, &serde_json::to_vec_pretty(&request).unwrap(), &dir).unwrap();
    prepare::verify(&catalog, &dir).unwrap();
    let mut alone = chain("ray", 32, "src/camera.rs", false);
    alone["callees"] = json!(true);
    assert!(prepare_value(&catalog, alone).err().unwrap().contains("owner_context"));
}
