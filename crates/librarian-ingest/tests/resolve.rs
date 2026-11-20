//! Step 2.1: the one rule of resolution (`resolve.rs`), shared by "what does it call?", the
//! inventory, the envelope and the owner context. One test per resolution, then what
//! `calls_from` reports for each, and what the owner context takes as settled.
use librarian_ingest::{
    generate,
    graph::v2,
    owner,
    resolve::{How, Resolution, Resolver, SETTLED},
    Catalog, Config,
};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const MODELS: &str = "pub trait Notifica {
    fn notify(&self) -> u8;
    fn rotulo(&self) -> u8;
    fn padrao(&self) -> u8 {
        1
    }
}

pub struct Email;
pub struct Sms;
pub struct Fila;

impl Notifica for Email {
    fn notify(&self) -> u8 {
        2
    }
    fn rotulo(&self) -> u8 {
        2
    }
}

impl Notifica for Sms {
    fn notify(&self) -> u8 {
        3
    }
    fn rotulo(&self) -> u8 {
        3
    }
}

impl Fila {
    pub fn notify(&self) -> u8 {
        4
    }
    pub fn padrao(&self) -> u8 {
        5
    }
}

pub struct Camera;
pub struct Cena {
    pub camera: Camera,
}
pub struct Outra;

impl Camera {
    pub fn validate(&self) -> bool {
        true
    }
}

impl Cena {
    pub fn validate(&self) -> bool {
        self.camera.validate() && self.check()
    }
    pub fn check(&self) -> bool {
        true
    }
}

impl Outra {
    pub fn check(&self) -> bool {
        false
    }
}

pub struct M<const N: usize>;
pub struct Z;

impl M<2> {
    pub fn det(&self) -> u8 {
        2
    }
}

impl M<3> {
    pub fn det(&self) -> u8 {
        3
    }
}

impl Z {
    pub fn det(&self) -> u8 {
        0
    }
}
";

const CASOS: &str = "use crate::models::*;
use crate::muitos::*;

pub fn unica() -> u8 {
    1
}

pub fn n_unique() -> u8 {
    unica()
}

pub fn n_type(e: &Email) -> u8 {
    e.notify()
}

pub fn n_trait_default(e: &Email) -> u8 {
    e.padrao()
}

pub fn n_trait(xs: &[Box<dyn Notifica>]) -> u8 {
    let mut total = 0;
    for ch in xs {
        total += ch.rotulo();
    }
    total
}

pub fn n_homonym(xs: &[Fila]) -> u8 {
    let mut total = 0;
    for x in xs {
        total += x.notify();
    }
    total
}

pub fn n_generic(m: &M<3>) -> u8 {
    m.det()
}

pub fn n_above(s: &S3) -> u8 {
    s.go() + S4::new().go()
}

pub fn n_above_untyped(xs: &[S1]) -> u8 {
    let mut total = 0;
    for s in xs {
        total += s.go();
    }
    total
}
";

const SHAPES: &str = "class Repo:
    def salvar(self):
        return 1


class Cliente:
    def salvar(self):
        return 2


class Servico:
    def __init__(self):
        self.repo = Repo()

    def salvar(self):
        return self.repo.salvar()

    def gravar(self):
        return self.salvar()


def usa(c: Cliente):
    return c.salvar()
";

/// Nine types with `go` and `new`: above the Rust profile's limit of 8 candidates.
fn muitos() -> String {
    (1..=9)
        .map(|i| format!("pub struct S{i};\n\nimpl S{i} {{\n    pub fn new() -> Self {{\n        S{i}\n    }}\n    pub fn go(&self) -> u8 {{\n        {i}\n    }}\n}}\n\n"))
        .collect()
}

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-resolve-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("src/lib.rs"), "pub mod casos;\npub mod models;\npub mod muitos;\n").unwrap();
        fs::write(dir.join("src/models.rs"), MODELS).unwrap();
        fs::write(dir.join("src/casos.rs"), CASOS).unwrap();
        fs::write(dir.join("src/muitos.rs"), muitos()).unwrap();
        fs::write(dir.join("pkg/shapes.py"), SHAPES).unwrap();
        Self(dir)
    }

    fn catalog(&self) -> Catalog {
        let path = self.0.join("edition");
        let config = Config { project: "resolve".into(), inputs: vec!["src".into(), "pkg".into()] };
        generate(&self.0, &config, &path).unwrap();
        Catalog::load(&path).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Setup {
    _project: Project,
    catalog: Catalog,
    graph: v2::Graph,
    syntax: v2::CallSyntax,
}

fn setup() -> Setup {
    let project = Project::new();
    let catalog = project.catalog();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    Setup { _project: project, catalog, graph, syntax }
}

fn symbol(graph: &v2::Graph, label: &str) -> String {
    let found: Vec<_> = graph.nodes.values().filter(|n| n.category == "symbol" && n.label == label).collect();
    assert_eq!(found.len(), 1, "{label}");
    found[0].id.clone()
}

/// The call occurrence `text` written in `caller`'s body.
fn call(graph: &v2::Graph, caller: &str, text: &str) -> String {
    let found: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.kind == "observes" && e.from == caller && graph.nodes[&e.to].label == text)
        .filter(|e| matches!(graph.nodes[&e.to].kind.as_str(), "call_observed" | "method_call_observed"))
        .map(|e| e.to.clone())
        .collect();
    assert_eq!(found.len(), 1, "{text}");
    found[0].clone()
}

/// What the rule says for `text` in `caller`: how, and the targets' labels.
fn resolved(s: &Setup, caller: &str, text: &str, chosen: Option<&str>) -> (String, Vec<String>) {
    let resolver = Resolver::new(&s.graph, &s.syntax);
    let caller = symbol(&s.graph, caller);
    let occurrence = call(&s.graph, &caller, text);
    let chosen = chosen.map(|c| symbol(&s.graph, c));
    let labels = |ids: Vec<&str>| {
        let mut l: Vec<String> = ids.into_iter().map(|id| s.graph.nodes[id].label.clone()).collect();
        l.sort();
        l
    };
    match resolver.resolve(&caller, &occurrence, chosen.as_deref()) {
        Resolution::Linked(how, targets) => (how.as_str().to_owned(), labels(targets)),
        Resolution::Homonym(targets) => ("homonym".to_owned(), labels(targets)),
        Resolution::AboveLimit(n) => (format!("above_candidate_limit {n}"), vec![]),
        Resolution::NoCandidate => ("no_candidate_in_project".to_owned(), vec![]),
    }
}

fn r(how: &str, targets: &[&str]) -> (String, Vec<String>) {
    (how.to_owned(), targets.iter().map(|t| t.to_string()).collect())
}

/// What `calls_from` reports for `text` in `caller`: (resolution, candidates, basis, has a
/// candidate edge, target) for each link, or the unresolved reason.
fn reported(s: &Setup, caller: &str, text: &str) -> Vec<(String, usize, String, bool, String)> {
    let caller = symbol(&s.graph, caller);
    let occurrence = call(&s.graph, &caller, text);
    let calls = s.graph.calls_from(&caller, 1, 50, &s.syntax, &BTreeMap::new()).unwrap();
    let mut out: Vec<_> = calls
        .links
        .iter()
        .filter(|l| l.occurrence == occurrence)
        .map(|l| (l.resolution.clone(), l.candidates, l.basis.clone(), l.candidate_edge.is_some(), s.graph.nodes[&l.to].label.clone()))
        .collect();
    out.extend(calls.unresolved.iter().filter(|u| u.occurrence == occurrence).map(|u| (u.reason.clone(), u.candidates, String::new(), false, String::new())));
    out.sort();
    out
}

#[test]
fn unique_one_declaration_of_the_name() {
    let s = setup();
    assert_eq!(resolved(&s, "n_unique", "unica", None), r("unique", &["unica"]));
    assert_eq!(reported(&s, "n_unique", "unica"), [("unique".to_owned(), 1, "graph_edge".to_owned(), true, "unica".to_owned())]);
}

#[test]
fn receiver_is_only_self_dot_m() {
    let s = setup();
    // self.check(): two homonyms, one in the caller's impl.
    assert_eq!(resolved(&s, "impl Cena::validate", "self.check()", None), r("receiver", &["impl Cena::check"]));
    // self.camera.validate() is not the receiver's own call: it was linked to Cena::validate itself.
    assert_ne!(resolved(&s, "impl Cena::validate", "self.camera.validate()", None).0, "receiver");
    // Python: self.salvar() is; self.repo.salvar() is not: since step 2.3a it is settled by the
    // field's written type (`self.repo = Repo()` in `__init__`), never by the receiver rule.
    assert_eq!(resolved(&s, "Servico.gravar", "self.salvar()", None), r("receiver", &["Servico.salvar"]));
    assert_eq!(resolved(&s, "Servico.salvar", "self.repo.salvar()", None), r("type", &["Repo.salvar"]));
}

#[test]
fn type_by_the_receivers_written_type() {
    let s = setup();
    assert_eq!(resolved(&s, "n_type", "e.notify()", None), r("type", &["impl Notifica for Email::notify"]));
    // A field of the impl's type: the case of `self.camera.validate()` in the ray tracer.
    assert_eq!(resolved(&s, "impl Cena::validate", "self.camera.validate()", None), r("type", &["impl Camera::validate"]));
    // Generic arguments are not compared: M<3> names both impls of M.
    assert_eq!(resolved(&s, "n_generic", "m.det()", None), r("type", &["impl M<2>::det", "impl M<3>::det"]));
    // Python, since step 2.3a: the annotated parameter settles it.
    assert_eq!(resolved(&s, "usa", "c.salvar()", None), r("type", &["Cliente.salvar"]));
}

#[test]
fn type_above_the_candidate_limit() {
    let s = setup();
    assert_eq!(resolved(&s, "n_above", "s.go()", None), r("type", &["impl S3::go"]));
    assert_eq!(resolved(&s, "n_above", "S4::new", None), r("type", &["impl S4::new"]));
    // The receiver `S4::new()` gets its type from the path: `T::f(..)`.
    assert_eq!(resolved(&s, "n_above", "S4::new().go()", None), r("type", &["impl S4::go"]));
    // No written type: the graph's count stays.
    assert_eq!(resolved(&s, "n_above_untyped", "s.go()", None), r("above_candidate_limit 9", &[]));
}

#[test]
fn trait_default_when_the_type_does_not_define_it() {
    let s = setup();
    assert_eq!(resolved(&s, "n_trait_default", "e.padrao()", None), r("trait_default", &["Notifica::padrao"]));
}

#[test]
fn trait_every_implementation_when_all_candidates_are_of_one_trait() {
    let s = setup();
    assert_eq!(
        resolved(&s, "n_trait", "ch.rotulo()", None),
        r("trait", &["Notifica::rotulo", "impl Notifica for Email::rotulo", "impl Notifica for Sms::rotulo"])
    );
}

#[test]
fn homonym_when_nothing_settles_it() {
    let s = setup();
    assert_eq!(
        resolved(&s, "n_homonym", "x.notify()", None),
        r("homonym", &["Notifica::notify", "impl Fila::notify", "impl Notifica for Email::notify", "impl Notifica for Sms::notify"])
    );
}

#[test]
fn chosen_comes_before_everything() {
    let s = setup();
    assert_eq!(resolved(&s, "n_type", "e.notify()", Some("impl Fila::notify")), r("chosen", &["impl Fila::notify"]));
    // A choice that is not a candidate is not a choice.
    assert_eq!(resolved(&s, "n_type", "e.notify()", Some("impl Camera::validate")).0, "type");
    assert_eq!(How::Chosen.basis(), "graph_edge");
}

#[test]
fn calls_reports_one_target_and_keeps_several_as_homonyms() {
    let s = setup();
    let link = |res: &str, n: usize, basis: &str, edge: bool, to: &str| (res.to_owned(), n, basis.to_owned(), edge, to.to_owned());
    assert_eq!(reported(&s, "n_type", "e.notify()"), [link("type", 4, "type", true, "impl Notifica for Email::notify")]);
    assert_eq!(reported(&s, "n_trait_default", "e.padrao()"), [link("trait_default", 2, "type", true, "Notifica::padrao")]);
    assert_eq!(reported(&s, "impl Cena::validate", "self.check()"), [link("receiver", 2, "graph_edge", true, "impl Cena::check")]);
    // Above the limit: no candidate edge (null), the recorded count, the type as basis.
    assert_eq!(reported(&s, "n_above", "s.go()"), [link("type", 9, "type", false, "impl S3::go")]);
    assert_eq!(reported(&s, "n_above_untyped", "s.go()"), [link("above_candidate_limit", 9, "", false, "")]);
    // Several targets (trait, a generic type): every candidate as homonym, none followed.
    let trait_links = reported(&s, "n_trait", "ch.rotulo()");
    assert!(trait_links.len() == 3 && trait_links.iter().all(|l| l.0 == "homonym" && l.2 == "graph_edge"), "{trait_links:?}");
    let generic = reported(&s, "n_generic", "m.det()");
    assert!(generic.len() == 3 && generic.iter().all(|l| l.0 == "homonym"), "{generic:?}");
    // The null candidate edge is written as null in the answer, beside the basis.
    let caller = symbol(&s.graph, "n_above");
    let answer = serde_json::to_value(s.graph.calls_from(&caller, 1, 50, &s.syntax, &BTreeMap::new()).unwrap()).unwrap();
    let above = answer["links"].as_array().unwrap().iter().find(|l| l["text"] == "s.go()").unwrap();
    assert!(above["candidate_edge"].is_null() && above["basis"] == "type");
}

#[test]
fn a_type_link_is_followed_and_a_homonym_is_not() {
    let s = setup();
    let caller = symbol(&s.graph, "impl Cena::validate");
    let calls = s.graph.calls_from(&caller, 2, 50, &s.syntax, &BTreeMap::new()).unwrap();
    let mut labels: Vec<_> = calls.nodes.iter().map(|n| n.label.as_str()).collect();
    labels.sort();
    assert_eq!(labels, ["impl Camera::validate", "impl Cena::check", "impl Cena::validate"]);
}

#[test]
fn the_owner_context_takes_settled_links_only() {
    let s = setup();
    assert_eq!(SETTLED, ["unique", "import", "scope", "receiver", "type", "trait_default"]);
    let added = |label: &str| {
        let (added, _) = owner::called(&s.catalog, &[symbol(&s.graph, label)], &[], 10).unwrap();
        let mut out: Vec<(String, String)> = added.iter().map(|a| (s.graph.nodes[&a.symbol_id].label.clone(), a.resolution.clone())).collect();
        out.sort();
        out
    };
    let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(added("impl Cena::validate"), [pair("impl Camera::validate", "type"), pair("impl Cena::check", "receiver")]);
    assert_eq!(added("n_trait_default"), [pair("Notifica::padrao", "trait_default")]);
    // Not settled: a trait's implementations, a homonym. Python's self.repo.salvar() used to bring
    // Servico.salvar itself, then nothing; since step 2.3a, the field's type settles it on Repo.salvar.
    assert!(added("n_trait").is_empty() && added("n_homonym").is_empty());
    assert_eq!(added("Servico.salvar"), [pair("Repo.salvar", "type")]);
}
