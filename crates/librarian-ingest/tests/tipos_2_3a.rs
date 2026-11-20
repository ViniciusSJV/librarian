//! Step 2.3a (DEC-39): the written type of a Python receiver, the rule that gave each type, and
//! instantiation. One test per rule, with Python and Rust where it applies.
use librarian_ingest::{
    generate, generate_with_profile,
    graph::v2,
    resolve::{Resolution, Resolver},
    Catalog, Config, EXTRACTOR_V8,
};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const MODELS: &str = "from typing import Protocol


class Cliente:
    def __init__(self, nome):
        self.nome = nome

    def salvar(self):
        return 1


class Repo:
    def salvar(self):
        return 2


class Base:
    def herdado(self):
        return 3


class Filho(Base):
    def proprio(self):
        return 4


class Dinamico:
    def __getattr__(self, nome):
        return nome


class Sink(Protocol):
    def write(self, texto: str) -> None:
        ...


class Arquivo:
    def write(self, texto):
        return texto


class Apprise:
    def notify(self):
        return 5
";

const FORMS: &str = "class Form:
    def valor(self):
        return 6


class Outro:
    def valor(self):
        return 7
";

const CASOS: &str = "import fora
from apprise_ext import Apprise
from pkg import forms
from pkg.models import Base, Cliente, Dinamico, Filho, Repo, Sink
from pkg.models import Cliente as C
from pkg.m3 import Muitos


def n_param(c: Cliente | None = None):
    if c is None:
        c = fora.busca()
    c.salvar()


def n_anot():
    x: Repo = fora.busca()
    x.salvar()


def n_ctor():
    a = Cliente(1)
    a.salvar()


def n_sem_init():
    r = Repo()
    r.salvar()


def n_modulo():
    f = forms.Form()
    f.valor()


def n_direto():
    Cliente(1).salvar()


def n_alias():
    c = C(1)
    c.salvar()


def n_conflito(flag):
    a = Cliente(1)
    if flag:
        a = Repo()
    a.salvar()


def n_externo():
    a = Apprise()
    a.notify()


def n_derruba():
    r = Repo()
    r.herdado()


def n_base(f: Filho):
    f.herdado()


def n_getattr(d: Dinamico):
    d.herdado()


def n_protocol(s: Sink):
    s.write(\"x\")


def n_teto():
    Muitos().roda()


class Servico:
    def __init__(self, repo: Repo):
        self.repo = repo
        self.cliente = Cliente(1)

    def gravar(self):
        self.repo.salvar()
        self.cliente.salvar()

    def aninhado(self):
        def dentro():
            return self.cliente.salvar()

        return dentro()
";

const RUST: &str = "pub struct Par(pub u8, pub u8);
pub struct A(pub u8);

pub enum E {
    A(u8),
    B(u8),
}

pub struct Ponto {
    pub x: u8,
}

pub struct Caixa {
    pub ponto: Ponto,
    pub pontos: Vec<Ponto>,
}

impl Ponto {
    pub fn novo() -> Self {
        Ponto { x: 0 }
    }
    pub fn mede(&self) -> u8 {
        self.x
    }
}

impl Caixa {
    pub fn tudo(&self) -> u8 {
        let mut n = self.ponto.mede();
        for p in self.pontos.iter() {
            n += p.mede();
        }
        n + self.mede_um()
    }
    pub fn mede_um(&self) -> u8 {
        1
    }
}

pub fn fabrica() -> Ponto {
    Ponto { x: 1 }
}

pub fn usa(q: &Ponto, e: E) -> u8 {
    let a: Ponto = Ponto::novo();
    let b = Ponto::novo();
    let c = Ponto { x: 2 };
    let d = fabrica();
    let p = Par(1, 2);
    let v = E::A(3);
    let w = match e {
        E::B(n) => n,
        _ => 0,
    };
    let _ = (p, v, w);
    q.mede() + a.mede() + b.mede() + c.mede() + d.mede()
}

pub enum Forma {
    Ponto(Ponto),
}

pub fn forma(f: Forma) -> u8 {
    match f {
        Forma::Ponto(ponto) => ponto.mede(),
    }
}
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-tipos23a-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("src/lib.rs"), RUST).unwrap();
        fs::write(dir.join("pkg/__init__.py"), "").unwrap();
        fs::write(dir.join("pkg/models.py"), MODELS).unwrap();
        fs::write(dir.join("pkg/forms.py"), FORMS).unwrap();
        fs::write(dir.join("pkg/casos.py"), CASOS).unwrap();
        // Nine classes `Muitos`, one per module: above the limit of 8 candidates.
        for i in 1..=9 {
            fs::write(dir.join(format!("pkg/m{i}.py")), format!("class Muitos:
    def roda(self):
        return {i}
")).unwrap();
        }
        Self(dir)
    }

    fn catalog(&self, name: &str, profile: Option<&str>) -> Catalog {
        let path = self.0.join(name);
        let config = Config { project: "tipos23a".into(), inputs: vec!["src".into(), "pkg".into()] };
        match profile {
            Some(p) => drop(generate_with_profile(p, &self.0, &config, &path).unwrap()),
            None => drop(generate(&self.0, &config, &path).unwrap()),
        }
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
    let catalog = project.catalog("edition", None);
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    Setup { _project: project, catalog, graph, syntax }
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

fn resolved(s: &Setup, caller: &str, text: &str) -> (String, Vec<String>) {
    let resolver = Resolver::new(&s.graph, &s.syntax);
    let caller = symbol(&s.graph, caller);
    let occurrence = call(&s.graph, &caller, text);
    let labels = |ids: Vec<&str>| {
        let mut l: Vec<String> = ids.into_iter().map(|id| s.graph.nodes[id].label.clone()).collect();
        l.sort();
        l
    };
    match resolver.resolve(&caller, &occurrence, None) {
        Resolution::Linked(how, targets) => (how.as_str().to_owned(), labels(targets)),
        Resolution::Homonym(targets) => ("homonym".to_owned(), labels(targets)),
        Resolution::AboveLimit(n) => (format!("above_candidate_limit {n}"), vec![]),
        Resolution::NoCandidate => ("no_candidate_in_project".to_owned(), vec![]),
    }
}

fn r(how: &str, targets: &[&str]) -> (String, Vec<String>) {
    (how.to_owned(), targets.iter().map(|t| t.to_string()).collect())
}

/// The type_rule of the receiver_type a caller records for a receiver written as `receiver`.
fn rules(s: &Setup, caller: &str) -> Vec<(String, String)> {
    let caller = symbol(&s.graph, caller);
    let mut out: Vec<(String, String)> = s
        .graph
        .edges
        .iter()
        .filter(|e| e.kind == "observes" && e.from == caller && s.graph.nodes[&e.to].kind == "receiver_type")
        .map(|e| (s.graph.nodes[&e.to].label.clone(), s.graph.nodes[&e.to].type_rule.clone().unwrap_or_default()))
        .collect();
    out.sort();
    out.dedup();
    out
}

fn pair(a: &str, b: &str) -> (String, String) {
    (a.to_owned(), b.to_owned())
}

#[test]
fn an_annotation_types_the_receiver_for_the_whole_function() {
    let s = setup();
    // `c: Cliente | None`, reassigned by a call from outside the project (HC-F6).
    assert_eq!(resolved(&s, "n_param", "c.salvar()"), r("type", &["Cliente.salvar"]));
    assert_eq!(rules(&s, "n_param"), [pair("c", "parametro")]);
    // `x: Repo = ..`, a variable's annotation.
    assert_eq!(resolved(&s, "n_anot", "x.salvar()"), r("type", &["Repo.salvar"]));
    assert_eq!(rules(&s, "n_anot"), [pair("x", "anotacao")]);
}

#[test]
fn a_constructor_types_the_receiver() {
    let s = setup();
    assert_eq!(resolved(&s, "n_ctor", "a.salvar()"), r("type", &["Cliente.salvar"]));
    assert_eq!(resolved(&s, "n_modulo", "f.valor()"), r("type", &["Form.valor"]));
    assert_eq!(resolved(&s, "n_direto", "Cliente(1).salvar()"), r("type", &["Cliente.salvar"]));
    assert_eq!(rules(&s, "n_ctor"), [pair("a", "construtor")]);
    // Assignments that disagree give no type.
    assert_eq!(resolved(&s, "n_conflito", "a.salvar()").0, "homonym");
}

#[test]
fn a_field_of_init_and_self_in_a_closure_type_the_receiver() {
    let s = setup();
    // `self.repo = repo` with `repo: Repo`, and `self.cliente = Cliente(1)`, in `__init__`.
    assert_eq!(resolved(&s, "Servico.gravar", "self.repo.salvar()"), r("type", &["Repo.salvar"]));
    assert_eq!(resolved(&s, "Servico.gravar", "self.cliente.salvar()"), r("type", &["Cliente.salvar"]));
    assert_eq!(rules(&s, "Servico.gravar"), [pair("self.cliente", "campo"), pair("self.repo", "campo")]);
    assert_eq!(resolved(&s, "Servico.aninhado.dentro", "self.cliente.salvar()"), r("type", &["Cliente.salvar"]));
}

#[test]
fn the_class_is_found_by_the_files_imports() {
    let s = setup();
    // `C` is `Cliente as C`: an alias is the 2.4's (N8); no type.
    assert_eq!(resolved(&s, "n_alias", "c.salvar()").0, "homonym");
    // `Apprise` comes from outside the project: the project's homonymous class is not its type,
    // and a type from outside rules nothing out in Python.
    assert_eq!(resolved(&s, "n_externo", "a.notify()"), r("unique", &["Apprise.notify"]));
}

#[test]
fn a_class_without_base_rules_a_candidate_out_unless_something_may_bring_the_method() {
    let s = setup();
    // `Repo` has no base and no `herdado`: the one candidate, `Base.herdado`, is ruled out.
    assert_eq!(resolved(&s, "n_derruba", "r.herdado()"), r("no_candidate_in_project", &[]));
    // A base may bring it (inheritance is the 2.4's): the unique stays.
    assert_eq!(resolved(&s, "n_base", "f.herdado()"), r("unique", &["Base.herdado"]));
    // `__getattr__` may answer any name (DEC-39): the unique stays.
    assert_eq!(resolved(&s, "n_getattr", "d.herdado()"), r("unique", &["Base.herdado"]));
}

#[test]
fn a_class_name_above_the_limit_is_found_by_the_import() {
    let s = setup();
    // Nine `Muitos` and nine `roda`: no candidate edge for the type nor for the call; the import
    // names `pkg/m3.py`, and its class settles the call.
    let (how, targets) = resolved(&s, "n_teto", "Muitos().roda()");
    let m3 = s.graph.nodes.values().find(|n| n.label == "Muitos.roda" && n.id.contains(&s.catalog.source_by_path("pkg/m3.py").unwrap().id)).unwrap();
    assert_eq!((how.as_str(), targets.as_slice()), ("type", ["Muitos.roda".to_owned()].as_slice()));
    let resolver = Resolver::new(&s.graph, &s.syntax);
    let caller = symbol(&s.graph, "n_teto");
    let Resolution::Linked(_, ids) = resolver.resolve(&caller, &call(&s.graph, &caller, "Muitos().roda()"), None) else { panic!() };
    assert_eq!(ids, [m3.id.as_str()]);
}

#[test]
fn a_protocol_gives_no_link() {
    let s = setup();
    // The protocol's method is a signature; the body that runs is the passed object's (Q6).
    assert_eq!(resolved(&s, "n_protocol", "s.write(\"x\")"), r("homonym", &["Arquivo.write", "Sink.write"]));
}

#[test]
fn an_instantiation_is_its_own_relation_and_runs_the_classs_init() {
    let s = setup();
    let walk = |caller: &str, depth: usize| s.graph.calls_from(&symbol(&s.graph, caller), depth, 50, &s.syntax, &BTreeMap::new()).unwrap();
    let link = |calls: &v2::Calls, text: &str| calls.links.iter().find(|l| l.text == text).unwrap().clone();
    let label = |id: &str| s.graph.nodes[id].label.clone();
    // `Cliente(1)`: the class, `instantiates`, and the walk goes on in its `__init__`.
    let calls = walk("n_ctor", 2);
    let cliente = link(&calls, "Cliente");
    assert_eq!((label(&cliente.to).as_str(), cliente.relation.as_str(), cliente.resolution.as_str()), ("Cliente", "instantiates", "unique"));
    assert_eq!(cliente.runs.as_deref().map(label).as_deref(), Some("Cliente.__init__"));
    assert!(calls.nodes.iter().any(|n| n.label == "Cliente.__init__"));
    assert_eq!(link(&calls, "a.salvar()").relation, "calls");
    // `Repo()`: no `__init__` of its own; the walk stops at the class.
    let calls = walk("n_sem_init", 2);
    let repo = link(&calls, "Repo");
    assert_eq!((repo.relation.as_str(), repo.runs.as_deref()), ("instantiates", None));
    // `forms.Form()`: a class of the project module, now a candidate of the method call.
    let form = link(&walk("n_modulo", 1), "forms.Form()");
    assert_eq!((label(&form.to).as_str(), form.relation.as_str(), form.resolution.as_str()), ("Form", "instantiates", "unique"));
}

#[test]
fn rust_tuple_structs_and_variants_are_instantiated() {
    let s = setup();
    // `E::A(3)`: the struct `A` and the variant `E::A` match the name; the path picks the variant.
    assert_eq!(resolved(&s, "usa", "E::A"), r("type", &["E::A"]));
    let calls = s.graph.calls_from(&symbol(&s.graph, "usa"), 1, 50, &s.syntax, &BTreeMap::new()).unwrap();
    let relation = |text: &str| calls.links.iter().find(|l| l.text == text).map(|l| l.relation.clone());
    assert_eq!(relation("E::A").as_deref(), Some("instantiates"));
    assert_eq!(relation("Par").as_deref(), Some("instantiates"));
    assert_eq!(relation("Ponto::novo").as_deref(), Some("calls"));
}

#[test]
fn rust_receiver_types_record_their_rule() {
    let s = setup();
    assert_eq!(
        rules(&s, "usa"),
        [pair("a", "anotacao"), pair("b", "caminho"), pair("c", "construtor"), pair("d", "retorno"), pair("q", "parametro")]
    );
    assert_eq!(rules(&s, "impl Caixa::tudo"), [pair("p", "item"), pair("self", "self"), pair("self.ponto", "campo"), pair("self.pontos", "campo")]);
    assert_eq!(rules(&s, "forma"), [pair("ponto", "variante")]);
}

#[test]
fn earlier_profiles_record_neither_rules_nor_python_types() {
    let project = Project::new();
    let old = project.catalog("v8", Some(EXTRACTOR_V8));
    let graph = v2::Graph::build(&old);
    assert!(graph.nodes.values().all(|n| n.type_rule.is_none()));
    assert!(!graph.nodes.values().any(|n| n.kind == "receiver_type" && n.language == "python"));
    graph.verify(&old).unwrap();
    // The current profile records both, and its graph verifies too.
    let s = setup();
    assert!(s.graph.nodes.values().any(|n| n.kind == "receiver_type" && n.language == "python" && n.type_rule.is_some()));
    s.graph.verify(&s.catalog).unwrap();
}
