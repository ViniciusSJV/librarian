//! Step 2.4a (DEC-42): a plain call (`f()`) and a module path (`m::f()`) settled by the caller's
//! scope, as the language binds names (Python and Rust), and an alias (`as`, a type alias, a local
//! alias) naming what it stands for. One test per rule.
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

const INIT: &str = "from .base import reexportada
";

const BASE: &str = "class Cliente:
    def salvar(self):
        return 1


class Fornecedor:
    def salvar(self):
        return 2


def g():
    return 1


def reexportada():
    return 2


def join(a, b):
    return a


def so_tipo():
    return 0
";

const OUTRO: &str = "def g():
    return 3


def reexportada():
    return 4


def modulo_local():
    return 5


def sobre(x):
    return x


def condicional():
    return 6


def interna():
    return 7


def len(x):
    return 0


def so_tipo():
    return 1


def tarde():
    return 2


def fabrica():
    return g
";

const CASOS: &str = "from os.path import join
from typing import TYPE_CHECKING, TypeAlias, overload
from pkg.base import Cliente as C, Fornecedor
from pkg import reexportada
from pkg import base

if TYPE_CHECKING:
    from pkg.base import so_tipo

ANTES = tarde()


def modulo_local():
    return 0


@overload
def sobre(x: int) -> int: ...


@overload
def sobre(x: str) -> str: ...


def sobre(x):
    return x


if base:
    def condicional():
        return 1
else:
    def condicional():
        return 2


def tarde():
    return 3


Tipo: TypeAlias = Fornecedor
Mod = base.Cliente


def usa_modulo():
    modulo_local()


def usa_fora():
    join(\"a\", \"b\")


def usa_local(fabrica):
    g = fabrica()
    g()


def usa_overload():
    sobre(1)


def usa_condicional():
    condicional()


def usa_type_checking():
    so_tipo()


def usa_reexport():
    reexportada()


def usa_builtin():
    len([])


def externa():
    def interna():
        return 1
    interna()


def usa_alias_import():
    C(\"x\")


def tipo_alias_import():
    c = C(\"x\")
    c.salvar()


def tipo_alias_local():
    D = C
    d = D()
    d.salvar()


def tipo_alias_modulo(a: Tipo):
    a.salvar()


def tipo_alias_pontuado(a: Mod):
    a.salvar()
";

const ESTRELA: &str = "from pkg.outro import *


def usa_estrela():
    g()
";

const RUST: &str = "pub mod modelos;
pub mod outro;
use crate::modelos::importada;
use crate::modelos::Cliente as C;
type C2 = modelos::Cliente;

pub fn livre() -> usize {
    1
}

pub fn r_modulo() -> usize {
    livre()
}

pub fn r_use() -> usize {
    importada()
}

pub fn r_bloco() -> usize {
    fn bloco() -> usize {
        9
    }
    bloco()
}

pub fn r_variavel() -> usize {
    let livre = || 5;
    livre()
}

pub fn r_alias_new() -> usize {
    let c = C::new();
    c.salvar()
}

pub fn r_alias_tipo() -> usize {
    let c = C2::new();
    c.salvar()
}

pub fn r_caminho() -> usize {
    outro::bloco()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn livre() -> usize {
        3
    }

    #[test]
    fn t_proprio() {
        livre();
    }

    #[test]
    fn t_glob() {
        importada();
    }
}

pub mod globos {
    use crate::modelos::*;
    use crate::outro::*;

    pub fn r_dois_globs() -> usize {
        let d = dupla();
        d.salvar()
    }
}

pub mod fora {
    use std::collections::*;

    pub fn r_glob_fora() -> usize {
        livre()
    }
}

pub mod sem_glob {
    pub fn r_invisivel() -> usize {
        livre()
    }
}

pub mod externo {
    use std::fs::write;

    pub fn r_externo() {
        write(\"a\", \"b\").ok();
    }
}
";

const RUST_MODELOS: &str = "pub struct Cliente;
pub struct Fornecedor;

impl Cliente {
    pub fn new() -> Self {
        Cliente
    }
    pub fn salvar(&self) -> usize {
        1
    }
}

impl Fornecedor {
    pub fn new() -> Self {
        Fornecedor
    }
    pub fn salvar(&self) -> usize {
        2
    }
}

pub fn importada() -> usize {
    1
}

pub fn dupla() -> Cliente {
    Cliente
}
";

const RUST_OUTRO: &str = "use crate::modelos::Fornecedor;

pub fn livre() -> usize {
    2
}

pub fn importada() -> usize {
    2
}

pub fn dupla() -> Fornecedor {
    Fornecedor
}

pub fn bloco() -> usize {
    3
}

pub fn write() -> usize {
    0
}
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-escopo24a-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("src/lib.rs"), RUST).unwrap();
        fs::write(dir.join("src/modelos.rs"), RUST_MODELOS).unwrap();
        fs::write(dir.join("src/outro.rs"), RUST_OUTRO).unwrap();
        fs::write(dir.join("pkg/__init__.py"), INIT).unwrap();
        fs::write(dir.join("pkg/base.py"), BASE).unwrap();
        fs::write(dir.join("pkg/outro.py"), OUTRO).unwrap();
        fs::write(dir.join("pkg/casos.py"), CASOS).unwrap();
        fs::write(dir.join("pkg/estrela.py"), ESTRELA).unwrap();
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
    let config = Config { project: "escopo24a".into(), inputs: vec!["src".into(), "pkg".into()] };
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

/// The resolution and the targets, each as `label (path)`.
fn resolved(s: &Setup, caller: &str, text: &str) -> (String, Vec<String>) {
    let resolver = Resolver::new(&s.graph, &s.syntax);
    let caller = symbol(&s.graph, caller);
    let occurrence = call(&s.graph, &caller, text);
    let path = |id: &str| s.graph.edges.iter().find(|e| e.kind == "declares" && e.to == id).map(|e| e.evidence.path.clone()).unwrap_or_default();
    let labels = |ids: Vec<&str>| {
        let mut l: Vec<String> = ids.into_iter().map(|id| format!("{} ({})", s.graph.nodes[id].label, path(id))).collect();
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

fn how(s: &Setup, caller: &str, text: &str) -> String {
    resolved(s, caller, text).0
}

#[test]
fn python_a_def_of_the_module_settles_homonyms() {
    let s = setup();
    assert_eq!(resolved(&s, "usa_modulo", "modulo_local"), r("scope", &["modulo_local (pkg/casos.py)"]));
    // A nested def comes before the module.
    assert_eq!(resolved(&s, "externa", "interna"), r("scope", &["externa.interna (pkg/casos.py)"]));
}

#[test]
fn python_the_last_def_after_overloads() {
    let s = setup();
    let (how, targets) = resolved(&s, "usa_overload", "sobre");
    assert_eq!(how, "scope");
    assert_eq!(targets.len(), 1);
    // The third `sobre` of casos.py, the implementation (the two others are `@overload`).
    let resolver = Resolver::new(&s.graph, &s.syntax);
    let caller = symbol(&s.graph, "usa_overload");
    let Resolution::Linked(_, ids) = resolver.resolve(&caller, &call(&s.graph, &caller, "sobre"), None) else { panic!() };
    let end = s.graph.edges.iter().find(|e| e.kind == "declares" && e.to == ids[0]).unwrap().evidence.end_byte;
    let last = CASOS.find("def sobre(x):\n").unwrap();
    assert!(end > last, "{end} {last}");
}

#[test]
fn python_an_import_settles_and_follows_a_re_export() {
    let s = setup();
    assert_eq!(resolved(&s, "usa_reexport", "reexportada"), r("import", &["reexportada (pkg/base.py)"]));
}

#[test]
fn python_an_import_from_outside_a_variable_or_a_builtin_rule_the_candidates_out() {
    let s = setup();
    // `from os.path import join`: the project's `join` is not the one called.
    assert_eq!(how(&s, "usa_fora", "join"), "no_candidate_in_project");
    // `g = fabrica()`: a variable, not `g` of base.py or outro.py.
    assert_eq!(how(&s, "usa_local", "g"), "no_candidate_in_project");
    // `len` is the builtin, not outro.py's.
    assert_eq!(how(&s, "usa_builtin", "len"), "no_candidate_in_project");
    // At the module's level the code runs in order: `tarde` is not bound yet.
    assert_eq!(how(&s, "ANTES", "tarde"), "no_candidate_in_project");
}

#[test]
fn python_what_the_scope_does_not_tell_stays_as_before() {
    let s = setup();
    // Two `def condicional` in an `if`/`else`, with outro.py's: the bindings disagree.
    assert_eq!(how(&s, "usa_condicional", "condicional"), "homonym");
    // Bound only under `TYPE_CHECKING`: not when the code runs.
    assert_eq!(how(&s, "usa_type_checking", "so_tipo"), "homonym");
    // `from pkg.outro import *`: `g` may come from there.
    assert_eq!(how(&s, "usa_estrela", "g"), "homonym");
}

#[test]
fn python_an_alias_of_an_import_instantiates_and_types() {
    let s = setup();
    assert_eq!(resolved(&s, "usa_alias_import", "C"), r("import", &["Cliente (pkg/base.py)"]));
    let calls = s.graph.calls_from(&symbol(&s.graph, "usa_alias_import"), 1, 50, &s.syntax, &BTreeMap::new()).unwrap();
    let link = calls.links.iter().find(|l| l.text == "C").unwrap();
    assert_eq!((link.relation.as_str(), link.basis.as_str()), ("instantiates", "import"));
    assert_eq!(resolved(&s, "tipo_alias_import", "c.salvar()"), r("type", &["Cliente.salvar (pkg/base.py)"]));
}

#[test]
fn python_a_local_alias_and_a_type_alias_of_the_module() {
    let s = setup();
    assert_eq!(resolved(&s, "tipo_alias_local", "d.salvar()"), r("type", &["Cliente.salvar (pkg/base.py)"]));
    assert_eq!(resolved(&s, "tipo_alias_modulo", "a.salvar()"), r("type", &["Fornecedor.salvar (pkg/base.py)"]));
    assert_eq!(resolved(&s, "tipo_alias_pontuado", "a.salvar()"), r("type", &["Cliente.salvar (pkg/base.py)"]));
}

#[test]
fn the_api_says_scope_with_its_own_basis() {
    let s = setup();
    let calls = s.graph.calls_from(&symbol(&s.graph, "usa_modulo"), 1, 50, &s.syntax, &BTreeMap::new()).unwrap();
    let link = calls.links.iter().find(|l| l.text == "modulo_local").unwrap();
    assert_eq!((link.resolution.as_str(), link.basis.as_str()), ("scope", "scope"));
}

#[test]
fn rust_an_item_of_the_module_or_of_the_block() {
    let s = setup();
    assert_eq!(resolved(&s, "r_modulo", "livre"), r("scope", &["livre (src/lib.rs)"]));
    assert_eq!(resolved(&s, "r_bloco", "bloco"), r("scope", &["r_bloco::bloco (src/lib.rs)"]));
    assert_eq!(resolved(&s, "tests::t_proprio", "livre"), r("scope", &["tests::livre (src/lib.rs)"]));
}

#[test]
fn rust_a_use_a_glob_and_a_module_path() {
    let s = setup();
    assert_eq!(resolved(&s, "r_use", "importada"), r("import", &["importada (src/modelos.rs)"]));
    // `use super::*` brings the parent's `use crate::modelos::importada`.
    assert_eq!(resolved(&s, "tests::t_glob", "importada"), r("import", &["importada (src/modelos.rs)"]));
    assert_eq!(resolved(&s, "r_caminho", "outro::bloco"), r("import", &["bloco (src/outro.rs)"]));
}

#[test]
fn rust_what_the_scope_rules_out_and_what_it_does_not_tell() {
    let s = setup();
    // `use std::fs::write`: not outro.rs's `write`.
    assert_eq!(how(&s, "externo::r_externo", "write"), "no_candidate_in_project");
    // Not visible from `sem_glob`, and no glob from outside the crate.
    assert_eq!(how(&s, "sem_glob::r_invisivel", "livre"), "no_candidate_in_project");
    // A glob from outside the crate may bring it.
    assert_eq!(how(&s, "fora::r_glob_fora", "livre"), "homonym");
    // A closure in a variable: a value, not an item.
    assert_eq!(how(&s, "r_variavel", "livre"), "homonym");
    // Two globs bring `dupla`: not decided, and the two returns disagree (d of the 2.3c).
    assert_eq!(how(&s, "globos::r_dois_globs", "dupla"), "homonym");
    assert_eq!(how(&s, "globos::r_dois_globs", "d.salvar()"), "homonym");
}

#[test]
fn rust_an_alias_of_a_type_by_use_as_and_by_type() {
    let s = setup();
    assert_eq!(resolved(&s, "r_alias_new", "C::new"), r("type", &["impl Cliente::new (src/modelos.rs)"]));
    assert_eq!(resolved(&s, "r_alias_new", "c.salvar()"), r("type", &["impl Cliente::salvar (src/modelos.rs)"]));
    assert_eq!(resolved(&s, "r_alias_tipo", "C2::new"), r("type", &["impl Cliente::new (src/modelos.rs)"]));
    assert_eq!(resolved(&s, "r_alias_tipo", "c.salvar()"), r("type", &["impl Cliente::salvar (src/modelos.rs)"]));
}
