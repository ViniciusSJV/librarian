//! P5: the suggested envelope. From chosen declarations, by written names only: the owner type
//! and the writers of the fields read, what is called and the type a called method belongs to,
//! who calls without homonyms, the types written, and the head of the file; a gathered step can
//! carry file heads as evidence.
use librarian_ingest::{
    envelope::{self, Suggestion},
    generate,
    prepare::{self, Request},
    Catalog, Config,
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

// The World of the renderer, small: the change that caches each object's inverse needed
// World::default (it fills the fields) and Matrix (what inverse returns).
const WORLD: &str = "use crate::object::Object;

pub struct World {
    pub objects: Vec<Object>,
}

impl World {
    pub fn new(objects: Vec<Object>) -> Self {
        World { objects }
    }
    pub fn intersect_world(&self, ray: Ray) -> f64 {
        let mut t = 0.0;
        for o in self.objects.iter() {
            t += o.hit(ray);
        }
        t
    }
}

impl Default for World {
    fn default() -> Self {
        World { objects: vec![] }
    }
}

pub fn render(w: &World, ray: Ray) -> f64 {
    w.intersect_world(ray)
}
";

const OBJECT: &str = "pub struct Ray {
    pub x: f64,
}

pub struct Matrix {
    pub v: f64,
}

impl Matrix {
    pub fn inverse(&self) -> Self {
        Matrix { v: 1.0 / self.v }
    }
}

pub struct Object {
    pub transform: Matrix,
}

impl Object {
    pub fn hit(&self, ray: Ray) -> f64 {
        let inverse = self.transform.inverse();
        inverse.v * ray.x
    }
}
";

const REPO: &str = "import uuid
from collections.abc import Sequence

from pkg.models import Delivery


def list_for_day(day: int) -> Sequence[Delivery]:
    return [Delivery(day)]
";

const MODELS: &str = "class Delivery:
    def __init__(self, day):
        self.day = day
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-envelope-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("src/world.rs"), WORLD).unwrap();
        fs::write(dir.join("src/object.rs"), OBJECT).unwrap();
        fs::write(dir.join("pkg/repo.py"), REPO).unwrap();
        fs::write(dir.join("pkg/models.py"), MODELS).unwrap();
        Self(dir)
    }

    fn edition(&self) -> Catalog {
        let path = self.0.join("edition");
        let config = Config { project: "envelope".into(), inputs: vec!["src".into(), "pkg".into()] };
        generate(&self.0, &config, &path).unwrap();
        Catalog::load(&path).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn id(catalog: &Catalog, qualified: &str) -> String {
    let found: Vec<_> = catalog.symbols().values().filter(|s| s.qualified_name == qualified).collect();
    assert_eq!(found.len(), 1, "{qualified}");
    found[0].id.clone()
}

/// Each suggested item as `name: code, code…`.
fn reasons(suggestion: &Suggestion) -> Vec<String> {
    suggestion
        .items
        .iter()
        .map(|i| format!("{}: {}", i.name, i.reasons.iter().map(|r| r.code.as_str()).collect::<Vec<_>>().join(", ")))
        .collect()
}

#[test]
fn the_world_change_gets_default_and_matrix_suggested() {
    let project = Project::new();
    let catalog = project.edition();
    let chosen = [id(&catalog, "impl World::intersect_world"), id(&catalog, "impl Object::hit")];
    let suggestion = envelope::suggest(&catalog, &chosen).unwrap();
    let found = reasons(&suggestion);
    for expected in [
        "impl World::intersect_world: chosen",
        "impl Object::hit: chosen",
        "World: owner_type",
        "impl World::new: field_writer",
        "impl Default for World::default: field_writer",
        "Object: owner_type, called_owner",
        "render: caller",
        "Ray: type_reference, type_reference",
        "impl Matrix::inverse: called",
        "Matrix: called_owner",
        "src/world.rs: header",
    ] {
        assert!(found.contains(&expected.to_owned()), "{expected} not in {found:#?}");
    }
    // object.rs starts with a declaration: it has no head to suggest.
    assert!(!suggestion.items.iter().any(|i| i.id == "header:src/object.rs"));
    // Every reason names the chosen declaration it is for, and the line where the link is.
    let default = suggestion.items.iter().find(|i| i.name == "impl Default for World::default").unwrap();
    assert_eq!(default.reasons[0].for_target, "impl World::intersect_world");
    assert_eq!(default.reasons[0].fields, ["objects"]);
    assert_eq!(default.reasons[0].lines, [22]);
    let head = suggestion.items.iter().find(|i| i.id == "header:src/world.rs").unwrap();
    assert_eq!((head.start_line, head.end_line), (1, 1));
    assert!(envelope::suggest(&catalog, &[]).is_err());
    assert!(envelope::suggest(&catalog, &["nope".into()]).is_err());
}

#[test]
fn a_type_in_the_signature_and_the_file_head_reach_a_gathered_step() {
    let project = Project::new();
    let catalog = project.edition();
    let suggestion = envelope::suggest(&catalog, &[id(&catalog, "list_for_day")]).unwrap();
    let found = reasons(&suggestion);
    // Written in the return type, and called to build one.
    assert!(found.contains(&"Delivery: called, type_reference".to_owned()), "{found:#?}");
    // The head stops at the last non-blank line before the first declaration.
    assert_eq!(envelope::header(&catalog, "pkg/repo.py"), Some((1, 4)));
    assert_eq!(envelope::header(&catalog, "pkg/models.py"), None);

    let request = |gathered: bool| -> Request {
        serde_json::from_value(json!({
            "question": "Como trazer junto?", "top": 5, "depth": 1, "hits": 1, "radius": 0, "numbered_lines": true,
            "targets": [{"path": "pkg/repo.py", "symbol_name": "list_for_day", "declaration_line": 7}],
            "gathered": gathered, "headers": ["pkg/repo.py"]
        }))
        .unwrap()
    };
    let prepared = prepare::prepare(&catalog, &request(true)).unwrap();
    let dossier: serde_json::Value = serde_json::from_str(&prepared.dossier).unwrap();
    let head = &dossier["facts"][1];
    assert_eq!((head["id"].as_str(), head["line"].as_u64(), head["end_line"].as_u64()), (Some("E2"), Some(1), Some(4)));
    assert_eq!(prepared.query.selected_fact_ids(), ["E1", "E2"]);
    // Heads ride only on a gathered step, once each, and only where lines precede a declaration.
    assert!(prepare::prepare(&catalog, &request(false)).is_err());
    let mut twice = request(true);
    twice.headers.push("pkg/repo.py".into());
    assert!(prepare::prepare(&catalog, &twice).is_err());
    let mut none = request(true);
    none.headers = vec!["pkg/models.py".into()];
    assert!(prepare::prepare(&catalog, &none).is_err());
}
