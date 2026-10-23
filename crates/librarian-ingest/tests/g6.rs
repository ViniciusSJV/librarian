//! G6 (profile v3): fields are symbols; attribute accesses, calls and base classes link to
//! same-named declarations as candidates, never resolved. Names with too many homonyms get none.
use librarian_ingest::{
    generate_with_profile,
    graph::{v2, Graph},
    Catalog, Config, EXTRACTOR_V2, EXTRACTOR_V3,
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
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-g6-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(
            dir.join("src/lib.rs"),
            "pub struct Vehicle {\n    pub placa: String,\n    ano: u16,\n}\n\npub struct Pair(u8, u8);\n\nimpl Vehicle {\n    pub fn check(&self) -> bool {\n        self.placa.is_empty()\n    }\n}\n\npub fn run(v: &Vehicle) -> bool {\n    v.check()\n}\n",
        )
        .unwrap();
        fs::write(
            dir.join("pkg/models.py"),
            "class Base:\n    pass\n\n\nclass VehicleBase(Base):\n    placa: str = \"\"\n    ano = 2020\n\n\nclass VehicleUpdate:\n    placa: str | None = None\n\n\nclass Owner:\n    def __init__(self, name):\n        self.name = name\n        self.name = name.strip()\n\n    def rename(self, new):\n        self.name = new\n\n\nclass Counter:\n    hits = misses = 0\n",
        )
        .unwrap();
        fs::write(
            dir.join("pkg/rules.py"),
            "def validate(vehicle):\n    return vehicle.placa.isdigit()\n\n\nclass Defaults:\n    def default(self, vehicle):\n        vehicle.placa = \"0000\"\n",
        )
        .unwrap();
        fs::write(
            dir.join("pkg/service.py"),
            "from pkg import rules\n\n\ndef save(vehicle):\n    rules.validate(vehicle)\n    return vehicle\n\n\ndef patch(vehicle):\n    validate(vehicle)\n",
        )
        .unwrap();
        // Nine methods named `get`: one more than the profile links.
        let many: String = (0..9).map(|i| format!("class C{i}:\n    def get(self):\n        return {i}\n\n\n")).collect();
        fs::write(dir.join("pkg/many.py"), format!("{many}def use(x):\n    return x.get()\n")).unwrap();
        Self(dir)
    }

    fn config(&self) -> Config {
        Config { project: "g6".into(), inputs: vec!["src".into(), "pkg".into()] }
    }

    fn edition(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        let _ = fs::remove_dir_all(&path);
        path
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn symbol_id(catalog: &Catalog, qualified: &str, kind: &str) -> String {
    let found: Vec<_> = catalog.symbols().values().filter(|s| s.qualified_name == qualified && s.kind == kind).collect();
    assert_eq!(found.len(), 1, "{qualified} ({kind})");
    found[0].id.clone()
}

/// Candidate targets reached from occurrences of `kind` whose text is `label`.
fn candidates(graph: &Graph, kind: &str, label: &str) -> Vec<String> {
    let occurrences: Vec<&String> = graph.nodes.values().filter(|n| n.kind == kind && n.label == label).map(|n| &n.id).collect();
    assert!(!occurrences.is_empty(), "no {kind} occurrence `{label}`");
    let mut targets: Vec<String> = graph
        .edges
        .iter()
        .filter(|e| e.resolution == "name_candidate" && occurrences.contains(&&e.from))
        .map(|e| e.to.clone())
        .collect();
    targets.sort();
    targets.dedup();
    targets
}

#[test]
fn fields_are_symbols_in_both_languages() {
    let project = Project::new();
    let edition = project.edition("v3");
    generate_with_profile(EXTRACTOR_V3, &project.0, &project.config(), &edition).unwrap();
    let catalog = Catalog::load(&edition).unwrap();
    assert_eq!(catalog.manifest().extractor, EXTRACTOR_V3);
    let fields: Vec<_> = {
        let mut f: Vec<_> = catalog.symbols().values().filter(|s| s.kind == "field").map(|s| s.qualified_name.clone()).collect();
        f.sort();
        f
    };
    // Class-body assignments and self.<name> in __init__, declared once; tuple fields have no name.
    assert_eq!(fields, ["Counter.hits", "Counter.misses", "Owner.name", "Vehicle::ano", "Vehicle::placa", "VehicleBase.ano", "VehicleBase.placa", "VehicleUpdate.placa"]);
    let chunk = |q: &str, k: &str| {
        let s = &catalog.symbols()[&symbol_id(&catalog, q, k)];
        let c = catalog.chunk(&s.chunk_id).unwrap();
        (c.start_line, c.end_line)
    };
    assert_eq!(chunk("Vehicle::placa", "field"), (2, 2));
    assert_eq!(chunk("VehicleBase.placa", "field"), (6, 6));
    // The first assignment in __init__ is the declaration.
    assert_eq!(chunk("Owner.name", "field"), (16, 16));
    // `hits = misses = 0` declares two fields, each spanning its own name.
    assert_eq!(chunk("Counter.hits", "field"), (24, 24));
    assert_ne!(symbol_id(&catalog, "Counter.hits", "field"), symbol_id(&catalog, "Counter.misses", "field"));

    // The v2 profile still reads the same project without fields.
    let old = project.edition("v2");
    generate_with_profile(EXTRACTOR_V2, &project.0, &project.config(), &old).unwrap();
    let old = Catalog::load(&old).unwrap();
    assert!(old.symbols().values().all(|s| s.kind != "field"));
}

#[test]
fn accesses_calls_and_bases_are_candidates_never_resolutions() {
    let project = Project::new();
    let edition = project.edition("v3");
    generate_with_profile(EXTRACTOR_V3, &project.0, &project.config(), &edition).unwrap();
    let catalog = Catalog::load(&edition).unwrap();
    let graph = Graph::build(&catalog);
    assert_eq!(graph.version, "rust-python-syntax-graph-v2");
    let id = |q: &str, k: &str| symbol_id(&catalog, q, k);

    // A Python access reaches every Python field named `placa`, and no Rust one.
    let mut python_placa = vec![id("VehicleBase.placa", "field"), id("VehicleUpdate.placa", "field")];
    python_placa.sort();
    assert_eq!(candidates(&graph, "attribute_access", "vehicle.placa"), python_placa);
    // In Rust, `self.placa` reaches the Rust field only.
    assert_eq!(candidates(&graph, "attribute_access", "self.placa"), [id("Vehicle::placa", "field")]);
    // Writes outside __init__ are accesses of the field too.
    assert_eq!(candidates(&graph, "attribute_access", "self.name"), [id("Owner.name", "field")]);

    // Calls: through a module receiver or by name, the written callee is the candidate.
    assert_eq!(candidates(&graph, "method_call_observed", "rules.validate(vehicle)"), [id("validate", "function")]);
    assert_eq!(candidates(&graph, "call_observed", "validate"), [id("validate", "function")]);
    assert_eq!(candidates(&graph, "method_call_observed", "v.check()"), [id("impl Vehicle::check", "method")]);
    // A called attribute is a call, not also an attribute access.
    assert!(!graph.nodes.values().any(|n| n.kind == "attribute_access" && n.label == "rules.validate"));

    // Inheritance is recorded as written: `VehicleBase(Base)` points at class Base.
    assert_eq!(candidates(&graph, "base_class", "Base"), [id("Base", "class")]);

    // Each candidate edge carries the occurrence's own bytes as evidence and stays unresolved.
    for edge in graph.edges.iter().filter(|e| e.kind == "name_candidate") {
        assert_eq!(edge.resolution, "name_candidate");
        let source = fs::read(project.0.join(&edge.evidence.path)).unwrap();
        let bytes = &source[edge.evidence.start_byte..edge.evidence.end_byte];
        assert_eq!(String::from_utf8_lossy(bytes), graph.nodes[&edge.from].label);
    }
}

#[test]
fn too_many_homonyms_record_a_count_instead_of_edges() {
    let project = Project::new();
    let edition = project.edition("v3");
    generate_with_profile(EXTRACTOR_V3, &project.0, &project.config(), &edition).unwrap();
    let catalog = Catalog::load(&edition).unwrap();
    let graph = Graph::build(&catalog);
    let call = graph.nodes.values().find(|n| n.kind == "method_call_observed" && n.label == "x.get()").unwrap();
    assert_eq!(call.ambiguous_candidates, Some(9));
    assert!(!graph.edges.iter().any(|e| e.from == call.id && e.kind == "name_candidate"));
    // Linked occurrences carry no count.
    assert!(graph.nodes.values().filter(|n| n.label == "vehicle.placa").all(|n| n.ambiguous_candidates.is_none()));

    // The neutral contract carries the count and verifies against the catalog.
    let projected = v2::Graph::build(&catalog);
    assert_eq!(projected.derived_from, "rust-python-syntax-graph-v2");
    assert_eq!(projected.nodes[&call.id].ambiguous_candidates, Some(9));
    let bytes = serde_json::to_vec(&projected).unwrap();
    serde_json::from_slice::<v2::Graph>(&bytes).unwrap().verify(&catalog).unwrap();
    // A forged count is refused.
    let mut forged = projected.clone();
    forged.nodes.get_mut(&call.id).unwrap().ambiguous_candidates = Some(3);
    assert!(forged.verify(&catalog).is_err());
}


#[test]
fn used_by_follows_written_names_back_in_layers() {
    let project = Project::new();
    let edition = project.edition("v3");
    generate_with_profile(EXTRACTOR_V3, &project.0, &project.config(), &edition).unwrap();
    let catalog = Catalog::load(&edition).unwrap();
    let graph = v2::Graph::build(&catalog);
    let placa = symbol_id(&catalog, "VehicleBase.placa", "field");
    let used = graph.used_by(&placa, 3, 50).unwrap();
    let at = |layer: usize| {
        let mut labels: Vec<_> = used.nodes.iter().filter(|n| n.layer == layer).map(|n| n.label.clone()).collect();
        labels.sort();
        labels
    };
    // placa <- validate (reads it) and Defaults.default (writes it) <- save and patch (call validate).
    assert_eq!(at(0), ["VehicleBase.placa"]);
    assert_eq!(at(1), ["Defaults.default", "validate"]);
    assert_eq!(at(2), ["patch", "save"]);
    assert!(!used.truncated);
    let read = used.links.iter().find(|l| l.text == "vehicle.placa" && used.nodes.iter().any(|n| n.id == l.from && n.label == "validate")).unwrap();
    // `vehicle.placa` also matches VehicleUpdate.placa: the link says it is one of two homonyms.
    assert_eq!((read.occurrence_kind.as_str(), read.candidates, read.evidence.path.as_str(), read.evidence.start_line), ("attribute_access", 2, "pkg/rules.py", 2));
    let default = used.nodes.iter().find(|n| n.label == "Defaults.default").unwrap();
    assert_eq!((default.container.as_deref(), default.path.as_str(), default.line), (Some("Defaults"), "pkg/rules.py", 6));
    // Every link is a candidate edge of the graph, with its own evidence.
    for link in &used.links {
        let edge = graph.edges.iter().find(|e| e.id == link.candidate_edge).unwrap();
        assert_eq!((edge.kind.as_str(), &edge.from, &edge.to, &edge.evidence), ("name_candidate", &link.occurrence, &link.to, &link.evidence));
    }

    // Limits are explicit: a node cap truncates; occurrences and bad limits are refused.
    let capped = graph.used_by(&placa, 3, 2).unwrap();
    assert_eq!(capped.nodes.len(), 2);
    assert!(capped.truncated);
    assert!(capped.links.iter().all(|l| capped.nodes.iter().any(|n| n.id == l.from)));
    let occurrence = graph.nodes.values().find(|n| n.category == "occurrence").unwrap().id.clone();
    assert!(graph.used_by(&occurrence, 2, 10).is_err());
    assert!(graph.used_by(&placa, 0, 10).is_err());
    assert!(graph.used_by(&placa, 7, 10).is_err());
    // Inheritance shows up the same way: class Base <- VehicleBase (written as its base).
    let base = graph.used_by(&symbol_id(&catalog, "Base", "class"), 1, 10).unwrap();
    assert_eq!(base.links.iter().map(|l| (l.occurrence_kind.as_str(), l.text.as_str())).collect::<Vec<_>>(), [("base_class", "Base")]);
}
