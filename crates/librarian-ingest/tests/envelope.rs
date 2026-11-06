//! P5: the suggested envelope. From chosen declarations, by written names only: the owner type
//! and the writers of the fields read, what is called and the type a called method belongs to,
//! who calls without homonyms, the types written, and the head of the file; a gathered step can
//! carry file heads as evidence.
use librarian_ingest::{
    envelope::{self, Candidates},
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
fn reasons(suggestion: &Candidates) -> Vec<String> {
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
    let suggestion = envelope::candidates(&catalog, &chosen, None).unwrap();
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
    assert!(envelope::candidates(&catalog, &[], None).is_err());
    assert!(envelope::candidates(&catalog, &["nope".into()], None).is_err());
}

#[test]
fn a_type_in_the_signature_and_the_file_head_reach_a_gathered_step() {
    let project = Project::new();
    let catalog = project.edition();
    let suggestion = envelope::candidates(&catalog, &[id(&catalog, "list_for_day")], None).unwrap();
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

/// A hot function, the cold loader that also calls it, and the constructor where a computed value
/// can be kept: the shape of the rounds that made this selection necessary.
const SHAPES: &str = "pub struct Mat4 { pub v: f64 }

impl Mat4 {
    pub fn invert(&self) -> f64 {
        1.0 / self.v
    }
}

pub struct Shape {
    pub transform: Mat4,
}

impl Shape {
    pub fn new(transform: Mat4) -> Self {
        Shape { transform }
    }
    pub fn trace(&self, x: f64) -> f64 {
        self.transform.invert() * x
    }
}

pub fn load(path: &str, m: &Mat4) -> f64 {
    // A long loader, read once, that the frame never reaches.
    let padding = \"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\";
    m.invert() + path.len() as f64 + padding.len() as f64
}

pub fn frame(shapes: &[Shape]) -> f64 {
    shapes.iter().map(|s| s.trace(1.0)).sum()
}

#[test]
fn invert_checks() {
    let m = Mat4 { v: 2.0 };
    assert_eq!(m.invert(), 0.5);
}
";

fn shapes() -> (Project, Catalog) {
    let project = Project::new();
    fs::write(project.0.join("src/shapes.rs"), SHAPES).unwrap();
    let catalog = project.edition();
    (project, catalog)
}

fn measure(catalog: &Catalog, counted: &[(&str, f64)]) -> envelope::Measure {
    let mut measure = envelope::Measure::default();
    for (name, calls) in counted {
        let symbol = id(catalog, name);
        measure.counted.insert(symbol.clone());
        if *calls > 0. {
            measure.calls.insert(symbol, *calls);
        }
    }
    measure
}

fn selected(selection: &envelope::Selection) -> Vec<(&str, &str)> {
    selection.items.iter().map(|e| (e.item.name.as_str(), e.role.as_str())).collect()
}

fn out(selection: &envelope::Selection, name: &str) -> Option<String> {
    selection.left_out.iter().find(|e| e.item.name == name).and_then(|e| e.left_out.clone())
}

#[test]
fn what_did_not_run_stays_out_but_a_cold_constructor_stays_in() {
    let (_project, catalog) = shapes();
    let chosen = [id(&catalog, "impl Shape::trace")];
    let candidates = envelope::candidates(&catalog, &chosen, None).unwrap();
    let counted = measure(&catalog, &[("impl Shape::trace", 90.), ("impl Mat4::invert", 90.), ("impl Shape::new", 0.), ("frame", 1.), ("load", 0.)]);
    let selection = envelope::select(&catalog, &candidates, Some(&counted), envelope::BUDGET_BYTES, 16);
    let found = selected(&selection);
    // The constructor did not run in the measure: it stays, because the value can be kept there.
    for expected in [("impl Shape::trace", "target"), ("Shape", "origin"), ("impl Shape::new", "origin"), ("impl Mat4::invert", "cost"), ("frame", "path")] {
        assert!(found.contains(&expected), "{expected:?} not in {found:#?}");
    }
    assert!(selection.measured);
    assert_eq!(selection.used_bytes, selection.items.iter().map(|e| e.bytes).sum::<usize>());

    let invert = [id(&catalog, "impl Mat4::invert")];
    let candidates = envelope::candidates(&catalog, &invert, Some(&counted)).unwrap();
    let selection = envelope::select(&catalog, &candidates, Some(&counted), envelope::BUDGET_BYTES, 16);
    // The loader calls it and did not run; the test is left to the gate even though it builds a Mat4.
    assert_eq!(out(&selection, "load").as_deref(), Some("did_not_run"));
    assert_eq!(out(&selection, "invert_checks").as_deref(), Some("test"));
    assert!(selected(&selection).contains(&("impl Shape::trace", "path")));
}

#[test]
fn without_a_measure_the_structure_decides_and_says_so() {
    let (_project, catalog) = shapes();
    let chosen = [id(&catalog, "impl Mat4::invert")];
    let candidates = envelope::candidates(&catalog, &chosen, None).unwrap();
    let selection = envelope::select(&catalog, &candidates, None, envelope::BUDGET_BYTES, 16);
    assert!(!selection.measured);
    assert!(selected(&selection).contains(&("load", "path")));
    assert!(selection.limits.iter().any(|l| l.starts_with("No measure")));
}

#[test]
fn the_budget_cuts_by_role_and_never_the_chosen_ones() {
    let (_project, catalog) = shapes();
    let chosen = [id(&catalog, "impl Shape::trace")];
    let candidates = envelope::candidates(&catalog, &chosen, None).unwrap();
    // Too small for anything: the chosen declaration goes anyway, and each other says why it stayed out.
    let selection = envelope::select(&catalog, &candidates, None, 1, 16);
    assert_eq!(selected(&selection), [("impl Shape::trace", "target")]);
    assert!(selection.left_out.iter().all(|e| e.left_out.as_deref() == Some("budget")), "{:#?}", selection.left_out);
    // The count still caps a selection, and what is big does not block what fits after it.
    let capped = envelope::select(&catalog, &candidates, None, envelope::BUDGET_BYTES, 2);
    assert_eq!(capped.items.len(), 2);
    assert!(capped.left_out.iter().any(|e| e.left_out.as_deref() == Some("max_items")));
    assert_eq!(capped.items.len() + capped.left_out.len(), candidates.items.len());
}

/// Two shapes with the same method name, a field written through a receiver nobody can type, and
/// a loop, a field write and a test around the method that calls them.
const SCENE: &str = "pub struct Disk { pub r: f64 }
impl Disk {
    pub fn area_of(&self) -> f64 { self.r }
}
pub struct Square { pub r: f64 }
impl Square {
    pub fn area_of(&self) -> f64 { self.r }
}
pub struct Scene { pub disk: Disk, pub total: f64 }
impl Scene {
    pub fn sum(&mut self, n: usize) -> f64 {
        for _ in 0..n {
            self.total = self.total + self.disk.area_of();
        }
        self.total
    }
}
pub fn reset(c: Scene2) {
    c.total = 0.0;
}
pub struct Scene2 { pub total: f64 }
#[test]
fn sum_checks() {
    Scene { disk: Disk { r: 1.0 }, total: 0.0 }.sum(1);
}
";

fn scene() -> (Project, Catalog) {
    let project = Project::new();
    fs::write(project.0.join("src/scene.rs"), SCENE).unwrap();
    let catalog = project.edition();
    (project, catalog)
}

fn gaps<'a>(candidates: &'a envelope::Candidates, code: &str) -> Vec<&'a envelope::Gap> {
    candidates.gaps.iter().filter(|g| g.code == code).collect()
}

#[test]
fn a_call_with_homonyms_is_a_gap_until_the_measure_settles_it() {
    let (_project, catalog) = scene();
    let chosen = [id(&catalog, "impl Scene::sum")];
    let open = envelope::candidates(&catalog, &chosen, None).unwrap();
    let found = gaps(&open, "homonym_call");
    let call = found.iter().find(|g| g.text.contains("area_of")).unwrap_or_else(|| panic!("{:#?}", open.gaps));
    assert_eq!(call.candidates.len(), 2);
    assert_eq!(call.line, Some(13));
    assert!(!open.items.iter().any(|i| i.name.ends_with("area_of")));

    // Only Disk's ran: it comes in as a cost, and the gap closes.
    let counted = measure(&catalog, &[("impl Disk::area_of", 10.), ("impl Square::area_of", 0.), ("impl Scene::sum", 1.)]);
    let settled = envelope::candidates(&catalog, &chosen, Some(&counted)).unwrap();
    assert!(!gaps(&settled, "homonym_call").iter().any(|g| g.text.contains("area_of")), "{:#?}", settled.gaps);
    let selection = envelope::select(&catalog, &settled, Some(&counted), envelope::BUDGET_BYTES, 16);
    assert!(selected(&selection).contains(&("impl Disk::area_of", "cost")), "{:#?}", selected(&selection));

    // Both ran: it stays a gap, naming the two that ran.
    let both = measure(&catalog, &[("impl Disk::area_of", 10.), ("impl Square::area_of", 3.), ("impl Scene::sum", 1.)]);
    let open = envelope::candidates(&catalog, &chosen, Some(&both)).unwrap();
    assert_eq!(gaps(&open, "homonym_call").iter().find(|g| g.text.contains("area_of")).unwrap().candidates.len(), 2);

    // One ran and the other was never counted: the selection settles it all the same, though the
    // measure says nothing about the other one. Known and kept as it is; the page's flowchart asks
    // for more before settling. This holds the behavior in place, it does not endorse it.
    let uncounted = measure(&catalog, &[("impl Disk::area_of", 10.), ("impl Scene::sum", 1.)]);
    let settled = envelope::candidates(&catalog, &chosen, Some(&uncounted)).unwrap();
    assert!(!gaps(&settled, "homonym_call").iter().any(|g| g.text.contains("area_of")), "{:#?}", settled.gaps);
    assert!(selected(&envelope::select(&catalog, &settled, Some(&uncounted), envelope::BUDGET_BYTES, 16)).contains(&("impl Disk::area_of", "cost")));
}

#[test]
fn a_cut_origin_is_a_gap_and_behavior_says_what_is_written() {
    let (_project, catalog) = scene();
    let chosen = [id(&catalog, "impl Scene::sum")];
    let candidates = envelope::candidates(&catalog, &chosen, None).unwrap();
    // No budget: the owner type stays out, and that is said.
    let cut = envelope::select(&catalog, &candidates, None, 1, 16);
    assert!(cut.gaps.iter().any(|g| g.code == "origin_left_out" && g.text == "Scene"), "{:#?}", cut.gaps);

    // A receiver nobody can type: one gap for the field, with the write it saw.
    let unfollowed = gaps(&candidates, "write_not_followed");
    assert!(unfollowed.iter().any(|g| g.for_target == "total" && g.text.contains("c.total") && g.count == Some(1)), "{:#?}", candidates.gaps);
    let selection = envelope::select(&catalog, &candidates, None, envelope::BUDGET_BYTES, 16);
    assert!(!selection.gaps.iter().any(|g| g.code == "origin_left_out"));
    let found = librarian_ingest::inventory::inventory(&catalog, &Default::default()).unwrap();
    let written = envelope::behavior(&catalog, &found, &selection);
    let kinds = |kind: &str| written.iter().filter(|b| b.kind == kind).map(|b| b.line).collect::<Vec<_>>();
    assert_eq!(kinds("loop"), [12], "{written:#?}");
    assert_eq!(kinds("state_write"), [13], "{written:#?}");
}

#[test]
fn the_notes_say_why_each_source_is_there_and_what_is_open() {
    let (_project, catalog) = scene();
    let chosen = [id(&catalog, "impl Scene::sum")];
    let candidates = envelope::candidates(&catalog, &chosen, None).unwrap();
    let mut selection = envelope::select(&catalog, &candidates, None, envelope::BUDGET_BYTES, 16);
    let found = librarian_ingest::inventory::inventory(&catalog, &Default::default()).unwrap();
    selection.behavior = envelope::behavior(&catalog, &found, &selection);
    let notes = envelope::notes(&selection);
    let has = |piece: &str| notes.iter().any(|n| n.text.contains(piece));
    assert!(notes.iter().any(|n| n.id == chosen[0] && n.text.contains("alvo da mudança")), "{notes:#?}");
    assert!(has("Não resolvido: em impl Scene::sum, a chamada"), "{notes:#?}");
    assert!(has("receptores de tipo desconhecido"), "{notes:#?}");
    assert!(has("laço"), "{notes:#?}");
    assert!(has("escreve"), "{notes:#?}");
    // Rules say why a source is there; the first line says they do not describe behavior.
    assert!(notes[0].text.contains("não como ele se comporta"));
}
