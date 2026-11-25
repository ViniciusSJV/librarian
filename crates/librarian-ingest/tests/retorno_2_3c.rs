//! Step 2.3c (DEC-41): the type of a receiver bound to a call, by the call's written return, in the
//! same file and across files; the field of a class of another file (Python); and `let x =
//! T::f(..)` by the return of `f`, not by `T` (Rust). One level only. One test per rule.
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

const MODELOS: &str = "from datetime import datetime


class Pedido:
    def fechar(self):
        return 1

    def nota(self):
        return Nota()

    def proximo(self):
        return self


class Nota:
    def fechar(self):
        return 2

    def nota(self):
        return self

    def proximo(self):
        return self


class Fabrica:
    def pedido(self) -> Pedido:
        return Pedido()

    def outra(self) -> \"Self\":
        return self


class Avulsa:
    def solta(self):
        return 3


def novo_pedido() -> Pedido:
    return Pedido()


def opcional() -> Pedido | None:
    return None


def agora() -> datetime:
    return datetime.now()


def pedidos() -> list[Pedido]:
    return []


def sem_anotacao():
    return Pedido()


def duplo() -> Pedido:
    return Pedido()


def igual() -> Pedido:
    return Pedido()


def nota_nova() -> Nota:
    return Nota()
";

const APOIO: &str = "from pkg.modelos import Pedido, Nota


class Requisicao:
    pedido: Pedido


def duplo() -> Nota:
    return Nota()


def igual() -> Pedido:
    return Pedido()


def de_fora() -> Pedido:
    return Pedido()
";

const CASOS: &str = "from pkg.modelos import Fabrica, novo_pedido, opcional, agora, pedidos, sem_anotacao, nota_nova
from pkg.apoio import Requisicao, de_fora
from pkg import modelos
from pkg.modelos import *
from pkg.apoio import *


def mesmo():
    p = modelos.novo_pedido()
    p.fechar()


def entre_arquivos():
    p = de_fora()
    p.fechar()


def metodo(fab: Fabrica):
    p = fab.pedido()
    p.fechar()
    fab.outra().proximo()


def direto():
    novo_pedido().fechar()


def talvez():
    p = opcional()
    p.fechar()


def campo(r: Requisicao):
    r.pedido.fechar()


def fora():
    d = agora()
    d.fechar()


def generico():
    ps = pedidos()
    ps.fechar()


def sem_tipo():
    p = sem_anotacao()
    p.fechar()


def discordam():
    d = duplo()
    d.fechar()


def concordam():
    d = igual()
    d.fechar()


def cadeia():
    p = novo_pedido()
    n = p.nota()
    n.fechar()


def laco():
    x = x.proximo()
    x.fechar()


def derruba():
    n = nota_nova()
    n.solta()


class Servico:
    def __init__(self):
        self.pedido = novo_pedido()

    def usa(self):
        self.pedido.fechar()
";

const RUST: &str = "pub mod apoio;

pub struct Pedido;
pub struct Nota;

impl Pedido {
    pub fn abrir() -> Self {
        Pedido
    }
    pub fn nota_nova() -> Nota {
        Nota
    }
    pub fn talvez() -> Option<Self> {
        None
    }
    pub fn caixa() -> Box<Self> {
        Box::new(Pedido)
    }
    pub fn tentar() -> Result<Self, String> {
        Ok(Pedido)
    }
    pub fn texto() -> String {
        String::new()
    }
    pub fn fechar(&self) -> usize {
        1
    }
    pub fn take(&self) -> usize {
        1
    }
    pub fn tamanho(&self) -> usize {
        1
    }
}

impl Nota {
    pub fn fechar(&self) -> usize {
        2
    }
    pub fn take(&self) -> usize {
        2
    }
}

pub fn feito() -> Pedido {
    Pedido
}

pub fn misto() -> Pedido {
    Pedido
}

pub fn r_self() -> usize {
    let p = Pedido::abrir();
    p.fechar()
}

pub fn r_outro() -> usize {
    let n = Pedido::nota_nova();
    n.fechar()
}

pub fn r_option() -> usize {
    let mut p = Pedido::talvez();
    p.take()
}

pub fn r_box() -> usize {
    let b = Pedido::caixa();
    b.fechar()
}

pub fn r_q() -> Result<usize, String> {
    let p = Pedido::tentar()?;
    Ok(p.fechar())
}

pub fn r_ref() -> usize {
    let p = Pedido;
    let r = apoio::referencia(&p);
    r.fechar()
}

pub fn r_fora() -> usize {
    let s = Pedido::texto();
    s.tamanho()
}

pub fn r_vec() -> usize {
    let v = Vec::new();
    v.tamanho()
}

pub fn r_concordam() -> usize {
    let p = feito();
    p.fechar()
}

pub fn r_discordam() -> usize {
    let p = misto();
    p.fechar()
}

pub fn r_entre_arquivos() -> usize {
    let p = apoio::longe();
    p.fechar()
}
";

const RUST_APOIO: &str = "use crate::{Nota, Pedido};

pub fn referencia(p: &Pedido) -> &Pedido {
    p
}

pub fn feito() -> Pedido {
    Pedido
}

pub fn misto() -> Nota {
    Nota
}

pub fn longe() -> Pedido {
    Pedido
}
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-retorno23c-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("src/lib.rs"), RUST).unwrap();
        fs::write(dir.join("src/apoio.rs"), RUST_APOIO).unwrap();
        fs::write(dir.join("pkg/__init__.py"), "").unwrap();
        fs::write(dir.join("pkg/modelos.py"), MODELOS).unwrap();
        fs::write(dir.join("pkg/apoio.py"), APOIO).unwrap();
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
    let config = Config { project: "retorno23c".into(), inputs: vec!["src".into(), "pkg".into()] };
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
        Resolution::Homonym(targets) | Resolution::Overrides(targets) => ("homonym".to_owned(), labels(targets)),
        Resolution::AboveLimit(n) => (format!("above_candidate_limit {n}"), vec![]),
        Resolution::NoCandidate | Resolution::OutOfChain => ("no_candidate_in_project".to_owned(), vec![]),
    }
}

fn r(how: &str, targets: &[&str]) -> (String, Vec<String>) {
    (how.to_owned(), targets.iter().map(|t| t.to_string()).collect())
}

fn homonym(a: &str, b: &str) -> (String, Vec<String>) {
    r("homonym", &[a, b])
}

/// The `type_rule` that `/api/calls` gives the link of a call.
fn type_rule(s: &Setup, caller: &str, text: &str) -> Option<String> {
    let calls = s.graph.calls_from(&symbol(&s.graph, caller), 1, 50, &s.syntax, &BTreeMap::new()).unwrap();
    calls.links.iter().find(|l| l.text == text).unwrap().type_rule.clone()
}

#[test]
fn python_a_return_types_the_receiver_in_the_same_file_and_across_files() {
    let s = setup();
    assert_eq!(resolved(&s, "mesmo", "p.fechar()"), r("type", &["Pedido.fechar"]));
    assert_eq!(resolved(&s, "entre_arquivos", "p.fechar()"), r("type", &["Pedido.fechar"]));
    assert_eq!(type_rule(&s, "mesmo", "p.fechar()").as_deref(), Some("retorno"));
    // `f(..).m()` and `self.x = f(..)` (Q4).
    assert_eq!(resolved(&s, "direto", "novo_pedido().fechar()"), r("type", &["Pedido.fechar"]));
    assert_eq!(resolved(&s, "Servico.usa", "self.pedido.fechar()"), r("type", &["Pedido.fechar"]));
}

#[test]
fn python_a_methods_return_an_optional_and_self() {
    let s = setup();
    // `fab.pedido() -> Pedido`: a method, settled by the written type of `fab`.
    assert_eq!(resolved(&s, "metodo", "p.fechar()"), r("type", &["Pedido.fechar"]));
    // `Pedido | None` is `Pedido` (DEC-39).
    assert_eq!(resolved(&s, "talvez", "p.fechar()"), r("type", &["Pedido.fechar"]));
    // `-> "Self"` is the method's class: `Fabrica` has no `proximo` and no base, and rules out
    // `Pedido.proximo` and `Nota.proximo`.
    assert_eq!(resolved(&s, "metodo", "fab.outra().proximo()"), r("no_candidate_in_project", &[]));
}

#[test]
fn python_a_field_of_a_class_of_another_file() {
    let s = setup();
    assert_eq!(resolved(&s, "campo", "r.pedido.fechar()"), r("type", &["Pedido.fechar"]));
    assert_eq!(type_rule(&s, "campo", "r.pedido.fechar()").as_deref(), Some("campo"));
}

#[test]
fn python_no_type_from_outside_a_generic_or_no_annotation() {
    let s = setup();
    assert_eq!(resolved(&s, "fora", "d.fechar()"), homonym("Nota.fechar", "Pedido.fechar"));
    assert_eq!(resolved(&s, "generico", "ps.fechar()"), homonym("Nota.fechar", "Pedido.fechar"));
    assert_eq!(resolved(&s, "sem_tipo", "p.fechar()"), homonym("Nota.fechar", "Pedido.fechar"));
}

#[test]
fn python_candidates_that_agree_give_the_type_and_that_disagree_give_none() {
    let s = setup();
    // `igual` and `duplo` come by `import *` from both modules: the scope does not decide (step
    // 2.4a, DEC-42), and the binding call is read as in the 2.3c.
    // (b) `igual()` is declared twice, and both return `Pedido`.
    assert_eq!(resolved(&s, "concordam", "d.fechar()"), r("type", &["Pedido.fechar"]));
    // (d) `duplo()` returns `Pedido` in one file and `Nota` in the other.
    assert_eq!(resolved(&s, "discordam", "d.fechar()"), homonym("Nota.fechar", "Pedido.fechar"));
}

#[test]
fn python_the_returned_class_rules_a_candidate_out() {
    let s = setup();
    // `Nota` has no base and no `solta`: the one candidate, `Avulsa.solta`, is ruled out.
    assert_eq!(resolved(&s, "derruba", "n.solta()"), r("no_candidate_in_project", &[]));
}

#[test]
fn one_level_a_chain_is_not_followed_and_a_loop_ends() {
    let s = setup();
    // `p.nota()` takes `p`'s type by the return; `n = p.nota()` would need a second level.
    assert_eq!(resolved(&s, "cadeia", "p.nota()"), r("type", &["Pedido.nota"]));
    assert_eq!(resolved(&s, "cadeia", "n.fechar()"), homonym("Nota.fechar", "Pedido.fechar"));
    // `x = x.proximo()`: the binding call is resolved without the return, and the query ends.
    assert_eq!(resolved(&s, "laco", "x.proximo()"), homonym("Nota.proximo", "Pedido.proximo"));
    assert_eq!(resolved(&s, "laco", "x.fechar()"), homonym("Nota.fechar", "Pedido.fechar"));
}

#[test]
fn a_persons_choice_does_not_reach_the_binding_call() {
    let s = setup();
    let caller = symbol(&s.graph, "discordam");
    let binding = call(&s.graph, &caller, "duplo");
    // The person picks `duplo` of `modelos.py` (returns `Pedido`) for the binding call.
    let pedido = s
        .graph
        .edges
        .iter()
        .find(|e| e.kind == "declares" && e.evidence.path == "pkg/modelos.py" && s.graph.nodes[&e.to].label == "duplo")
        .map(|e| e.to.clone())
        .unwrap();
    let chosen = BTreeMap::from([(binding.clone(), pedido.clone())]);
    let calls = s.graph.calls_from(&caller, 1, 50, &s.syntax, &chosen).unwrap();
    assert!(calls.links.iter().any(|l| l.occurrence == binding && l.to == pedido && l.resolution == "chosen"));
    let fechar: Vec<_> = calls.links.iter().filter(|l| l.text == "d.fechar()").collect();
    assert!(fechar.iter().all(|l| l.resolution == "homonym"), "{fechar:?}");
}

#[test]
fn rust_let_x_t_f_takes_the_return_of_f_not_t() {
    let s = setup();
    assert_eq!(resolved(&s, "r_self", "p.fechar()"), r("type", &["impl Pedido::fechar"]));
    assert_eq!(type_rule(&s, "r_self", "p.fechar()").as_deref(), Some("retorno"));
    // `Pedido::nota_nova() -> Nota`: until 2.3c the path gave `Pedido`.
    assert_eq!(resolved(&s, "r_outro", "n.fechar()"), r("type", &["impl Nota::fechar"]));
    // `let x = f(..)` with `f` in another file, returning a reference.
    assert_eq!(resolved(&s, "r_entre_arquivos", "p.fechar()"), r("type", &["impl Pedido::fechar"]));
    assert_eq!(resolved(&s, "r_ref", "r.fechar()"), r("type", &["impl Pedido::fechar"]));
}

#[test]
fn rust_wrappers_decide_nothing() {
    let s = setup();
    // `Option<Self>`: `Option::take` runs, not `Pedido::take`.
    assert_eq!(resolved(&s, "r_option", "p.take()"), homonym("impl Nota::take", "impl Pedido::take"));
    // `Box<Self>` (Q2): no type, though `Pedido::fechar` would run by auto-deref.
    assert_eq!(resolved(&s, "r_box", "b.fechar()"), homonym("impl Nota::fechar", "impl Pedido::fechar"));
    // `Result<Self, E>` with `?` (Q2, limit): no binding is read, no type.
    assert_eq!(resolved(&s, "r_q", "p.fechar()"), homonym("impl Nota::fechar", "impl Pedido::fechar"));
}

#[test]
fn rust_a_return_from_outside_follows_the_rule_of_a_type_from_outside() {
    let s = setup();
    // `Pedido::texto() -> String`: an inherent method of `Pedido` is ruled out.
    assert_eq!(resolved(&s, "r_fora", "s.tamanho()"), r("no_candidate_in_project", &[]));
    // (c) `Vec::new()` has no candidate in the project: the path's type from outside, as before.
    assert_eq!(resolved(&s, "r_vec", "v.tamanho()"), r("no_candidate_in_project", &[]));
}

#[test]
fn rust_candidates_that_agree_give_the_type_and_that_disagree_give_none() {
    let s = setup();
    assert_eq!(resolved(&s, "r_concordam", "p.fechar()"), r("type", &["impl Pedido::fechar"]));
    // Step 2.4a (DEC-42): `misto()` is the one of the caller's own module (`-> Pedido`), by the
    // case (a); until then the two `misto` disagreed (d) and gave no type. The (d) in Rust is in
    // `tests/escopo_2_4a.rs` (two globs).
    assert_eq!(resolved(&s, "r_discordam", "p.fechar()"), r("type", &["impl Pedido::fechar"]));
}
