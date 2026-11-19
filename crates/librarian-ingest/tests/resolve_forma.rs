//! Step 2.2: the form of a call admits only some candidates (`resolve.rs`). One test per rule,
//! with Python and Rust cases: a method call never reaches a free function; a Python module
//! receiver reaches that module's functions (`import`); a receiver bound to an import from outside
//! the project reaches nothing; a local binding wins over an import; a plain call never reaches a
//! method (save a function of the class, in its body); the receiver's written type rules a unique
//! candidate out, with its exceptions; `self.m()` above the limit is `receiver`.
use librarian_ingest::{
    generate,
    graph::v2,
    resolve::{Resolution, Resolver},
    Catalog, Config,
};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const EMAILS: &str = "def send(x):
    return x


def login(x):
    return x
";

const VIEWS: &str = "def login(x):
    return x
";

const ADMIN: &str = "class Admin:
    def login(self, x):
        return x

    def time(self):
        return 0

    def gravar(self):
        return 1

    def sign(self, x):
        return x

    def salvar_tudo(self):
        return 1
";

const CASOS: &str = "import time
from django.contrib import auth
from django.core.signing import TimestampSigner
from pkg import emails
import pkg.views
from . import views as v


def salvar_tudo():
    return 1


def modulo():
    emails.send(1)


def modulo_homonimo():
    emails.login(1)


def modulo_ponto():
    pkg.views.login(1)


def modulo_relativo():
    v.login(1)


def externo():
    time.time()


def externo_from():
    auth.login(1)


def externo_chamada():
    TimestampSigner().sign(1)


def metodo(c):
    c.send(1)


def sombra_parametro(emails):
    emails.send(1)


def sombra_atribuicao():
    emails = Outro()
    emails.send(1)


def sombra_for(xs):
    for emails in xs:
        emails.send(1)


def sombra_with(x):
    with x as emails:
        emails.send(1)


def sombra_except():
    try:
        pass
    except Exception as emails:
        emails.send(1)


def sombra_comprehension(xs):
    return [emails.send(1) for emails in xs]


def simples():
    salvar_tudo()


def so_metodo():
    gravar()


def aninhada():
    def interna():
        return 1

    return interna()


class ComCorpo:
    def _ajuda():
        return 1

    valor = _ajuda()

    def usa(self):
        return 1
";

const TIPOS: &str = "use crate::muitos::*;

pub struct Groups {
    pub children: Vec<u8>,
}

impl Groups {
    pub fn len(&self) -> usize {
        self.children.len()
    }
}

pub struct Outro;

pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {}
}

pub trait Quase {
    fn quase(&self) -> bool;
}

impl Quase for f64 {
    fn quase(&self) -> bool {
        true
    }
}

pub fn lines() -> u8 {
    1
}

pub fn n_forma(model: &str) -> u8 {
    model.lines()
}

pub fn n_vec(v: Vec<u8>) -> usize {
    v.len()
}

pub fn n_ref(v: &Vec<u8>) -> usize {
    v.len()
}

pub fn n_box(v: Box<Vec<u8>>) -> usize {
    v.len()
}

pub fn n_rc(v: std::rc::Rc<Vec<u8>>) -> usize {
    v.len()
}

pub fn n_arc(v: std::sync::Arc<Vec<u8>>) -> usize {
    v.len()
}

pub fn n_trait_externo(x: f64) -> usize {
    x.len()
}

pub fn n_outro(o: Outro) -> usize {
    o.len()
}

pub fn n_proprio(g: Groups) -> usize {
    g.len()
}

pub fn n_espelho() {
    drop(Guard);
}

pub fn n_acima(s: &S1) -> u8 {
    s.chama()
}
";

/// Nine types with `go` and `chama`: above the Rust profile's limit of 8 candidates. `chama`
/// calls `self.go()`.
fn muitos() -> String {
    (1..=9)
        .map(|i| format!("pub struct S{i};\n\nimpl S{i} {{\n    pub fn go(&self) -> u8 {{\n        {i}\n    }}\n    pub fn chama(&self) -> u8 {{\n        self.go()\n    }}\n}}\n\n"))
        .collect()
}

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-forma-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("src/lib.rs"), "pub mod muitos;\npub mod tipos;\n").unwrap();
        fs::write(dir.join("src/tipos.rs"), TIPOS).unwrap();
        fs::write(dir.join("src/muitos.rs"), muitos()).unwrap();
        fs::write(dir.join("pkg/__init__.py"), "").unwrap();
        fs::write(dir.join("pkg/emails.py"), EMAILS).unwrap();
        fs::write(dir.join("pkg/views.py"), VIEWS).unwrap();
        fs::write(dir.join("pkg/admin.py"), ADMIN).unwrap();
        fs::write(dir.join("pkg/casos.py"), CASOS).unwrap();
        Self(dir)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Setup {
    _project: Project,
    graph: v2::Graph,
    syntax: v2::CallSyntax,
}

fn setup() -> Setup {
    let project = Project::new();
    let path = project.0.join("edition");
    let config = Config { project: "forma".into(), inputs: vec!["src".into(), "pkg".into()] };
    generate(&project.0, &config, &path).unwrap();
    let catalog = Catalog::load(&path).unwrap();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    Setup { _project: project, graph, syntax }
}

fn symbol(graph: &v2::Graph, label: &str) -> String {
    let found: Vec<_> = graph.nodes.values().filter(|n| n.category == "symbol" && n.label == label).collect();
    assert_eq!(found.len(), 1, "{label}");
    found[0].id.clone()
}

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

/// What the rule says for `text` in `caller`, with the graph's candidate count.
fn resolved(s: &Setup, caller: &str, text: &str) -> (String, Vec<String>, usize) {
    let resolver = Resolver::new(&s.graph, &s.syntax);
    let caller = symbol(&s.graph, caller);
    let occurrence = call(&s.graph, &caller, text);
    let labels = |ids: Vec<&str>| {
        let mut l: Vec<String> = ids.into_iter().map(|id| s.graph.nodes[id].label.clone()).collect();
        l.sort();
        l
    };
    let graph = resolver.candidates(&occurrence).len();
    match resolver.resolve(&caller, &occurrence, None) {
        Resolution::Linked(how, targets) => (how.as_str().to_owned(), labels(targets), graph),
        Resolution::Homonym(targets) => ("homonym".to_owned(), labels(targets), graph),
        Resolution::AboveLimit(n) => (format!("above_candidate_limit {n}"), vec![], graph),
        Resolution::NoCandidate => ("no_candidate_in_project".to_owned(), vec![], graph),
    }
}

fn r(how: &str, targets: &[&str], graph: usize) -> (String, Vec<String>, usize) {
    (how.to_owned(), targets.iter().map(|t| t.to_string()).collect(), graph)
}

#[test]
fn a_method_call_never_reaches_a_free_function() {
    let s = setup();
    // Python: `c` is a parameter, not a module; `send` is only a free function.
    assert_eq!(resolved(&s, "metodo", "c.send(1)"), r("no_candidate_in_project", &[], 1));
    // Rust: `model.lines()` with the free function `lines` (RT-15/16).
    assert_eq!(resolved(&s, "n_forma", "model.lines()"), r("no_candidate_in_project", &[], 1));
}

#[test]
fn a_module_receiver_reaches_that_modules_functions() {
    let s = setup();
    // The only candidate: the label stays `unique`.
    assert_eq!(resolved(&s, "modulo", "emails.send(1)"), r("unique", &["send"], 1));
    // Among homonyms (two free functions and a method), the module decides: `import`.
    assert_eq!(resolved(&s, "modulo_homonimo", "emails.login(1)"), r("import", &["login"], 3));
    // `import pkg.views` and `pkg.views.login()`; `from . import views as v`.
    let at = |caller: &str, text: &str| {
        let resolver = Resolver::new(&s.graph, &s.syntax);
        let caller = symbol(&s.graph, caller);
        let occurrence = call(&s.graph, &caller, text);
        match resolver.resolve(&caller, &occurrence, None) {
            Resolution::Linked(how, t) => (how.as_str().to_owned(), t.iter().map(|id| s.graph.nodes.values().find(|n| n.id == *id).map(|n| n.id.clone()).unwrap()).collect::<Vec<_>>()),
            other => (format!("{other:?}"), vec![]),
        }
    };
    let views_login = s.graph.nodes.values().find(|n| n.label == "login" && n.id.contains("function") && s.graph.edges.iter().any(|e| e.kind == "declares" && e.to == n.id && e.evidence.path == "pkg/views.py")).unwrap().id.clone();
    assert_eq!(at("modulo_ponto", "pkg.views.login(1)"), ("import".to_owned(), vec![views_login.clone()]));
    assert_eq!(at("modulo_relativo", "v.login(1)"), ("import".to_owned(), vec![views_login]));
}

#[test]
fn a_receiver_bound_to_an_import_from_outside_the_project_reaches_nothing() {
    let s = setup();
    // `import time`: the project's method `Admin.time` is not it.
    assert_eq!(resolved(&s, "externo", "time.time()"), r("no_candidate_in_project", &[], 1));
    // `from django.contrib import auth`: neither the functions nor the method `login`.
    assert_eq!(resolved(&s, "externo_from", "auth.login(1)"), r("no_candidate_in_project", &[], 3));
    // The receiver is the result of a class from outside the project: its methods are not the project's.
    assert_eq!(resolved(&s, "externo_chamada", "TimestampSigner().sign(1)"), r("no_candidate_in_project", &[], 1));
}

#[test]
fn a_local_binding_wins_over_an_import() {
    let s = setup();
    // Parameter, assignment, `for`, `with`, `except` and comprehension targets named `emails`:
    // a variable, not the module, so the free function is not reached.
    for (caller, text) in [
        ("sombra_parametro", "emails.send(1)"),
        ("sombra_atribuicao", "emails.send(1)"),
        ("sombra_for", "emails.send(1)"),
        ("sombra_with", "emails.send(1)"),
        ("sombra_except", "emails.send(1)"),
        ("sombra_comprehension", "emails.send(1)"),
    ] {
        assert_eq!(resolved(&s, caller, text), r("no_candidate_in_project", &[], 1), "{caller}");
    }
}

#[test]
fn a_plain_call_never_reaches_a_method() {
    let s = setup();
    // Python: the free function among a function and a method.
    assert_eq!(resolved(&s, "simples", "salvar_tudo"), r("unique", &["salvar_tudo"], 2));
    // Only a method has the name.
    assert_eq!(resolved(&s, "so_metodo", "gravar"), r("no_candidate_in_project", &[], 1));
    // A nested function is a function, not a method.
    assert_eq!(resolved(&s, "aninhada", "interna"), r("unique", &["aninhada.interna"], 1));
    // In a class body, a plain name is the class's own function.
    assert_eq!(resolved(&s, "ComCorpo.valor", "_ajuda"), r("unique", &["ComCorpo._ajuda"], 1));
    // Rust: `drop(x)` is not `impl Drop for Guard::drop`.
    assert_eq!(resolved(&s, "n_espelho", "drop"), r("no_candidate_in_project", &[], 1));
}

#[test]
fn the_receivers_written_type_rules_a_unique_candidate_out() {
    let s = setup();
    // A type from outside the project, and an inherent method of a project type.
    assert_eq!(resolved(&s, "n_vec", "v.len()"), r("no_candidate_in_project", &[], 1));
    assert_eq!(resolved(&s, "impl Groups::len", "self.children.len()"), r("no_candidate_in_project", &[], 1));
    // A project type that has no such method.
    assert_eq!(resolved(&s, "n_outro", "o.len()"), r("no_candidate_in_project", &[], 1));
    // The type agrees: the label stays.
    assert_eq!(resolved(&s, "n_proprio", "g.len()"), r("unique", &["impl Groups::len"], 1));
}

#[test]
fn the_type_does_not_rule_out_a_reference() {
    let s = setup();
    assert_eq!(resolved(&s, "n_ref", "v.len()"), r("unique", &["impl Groups::len"], 1));
}

#[test]
fn the_type_does_not_rule_out_a_box() {
    let s = setup();
    assert_eq!(resolved(&s, "n_box", "v.len()"), r("unique", &["impl Groups::len"], 1));
}

#[test]
fn the_type_does_not_rule_out_an_rc() {
    let s = setup();
    assert_eq!(resolved(&s, "n_rc", "v.len()"), r("unique", &["impl Groups::len"], 1));
}

#[test]
fn the_type_does_not_rule_out_an_arc() {
    let s = setup();
    assert_eq!(resolved(&s, "n_arc", "v.len()"), r("unique", &["impl Groups::len"], 1));
}

#[test]
fn the_type_does_not_rule_out_a_type_the_project_implements_a_trait_for() {
    let s = setup();
    // `impl Quase for f64`: the project adds methods to f64.
    assert_eq!(resolved(&s, "n_trait_externo", "x.len()"), r("unique", &["impl Groups::len"], 1));
}

#[test]
fn self_dot_m_above_the_limit_is_receiver() {
    let s = setup();
    // RT-55: nine `go`, no candidate edge; the caller's impl has one.
    assert_eq!(resolved(&s, "impl S4::chama", "self.go()"), r("receiver", &["impl S4::go"], 0));
}

#[test]
fn calls_reports_the_admitted_candidates() {
    let s = setup();
    let report = |caller: &str, text: &str| {
        let caller = symbol(&s.graph, caller);
        let occurrence = call(&s.graph, &caller, text);
        let calls = s.graph.calls_from(&caller, 1, 50, &s.syntax, &BTreeMap::new()).unwrap();
        let mut out: Vec<(String, usize, String, bool)> = calls.links.iter().filter(|l| l.occurrence == occurrence).map(|l| (l.resolution.clone(), l.candidates, l.basis.clone(), l.candidate_edge.is_some())).collect();
        out.extend(calls.unresolved.iter().filter(|u| u.occurrence == occurrence).map(|u| (u.reason.clone(), u.candidates, String::new(), false)));
        out
    };
    let x = |a: &str, n: usize, b: &str, e: bool| (a.to_owned(), n, b.to_owned(), e);
    // `import`: one admitted candidate, its edge, basis `import`.
    assert_eq!(report("modulo_homonimo", "emails.login(1)"), [x("import", 1, "import", true)]);
    // Ruled out by the form: no candidate, never a link.
    assert_eq!(report("externo_from", "auth.login(1)"), [x("no_candidate_in_project", 0, "", false)]);
    // A homonym lists only what the form reaches.
    assert_eq!(report("n_vec", "v.len()"), [x("no_candidate_in_project", 0, "", false)]);
    // `receiver` above the limit: no candidate edge, the recorded count.
    assert_eq!(report("impl S4::chama", "self.go()"), [x("receiver", 9, "graph_edge", false)]);
}
