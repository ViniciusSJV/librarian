//! Step 2.4b (DEC-43): a Python method found along the chain of bases (C3) of the receiver's static
//! class: `self`, `cls`, `super()`, a written type, a `@property`; what the chain leaves undecided; an
//! abstract method's overrides; a candidate outside the chain. The sources are the gate's synthetic
//! project, whose answers were checked by running them in a real Python (docs/reports/fase-2/dados/2.4b).
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

const APOIO: &str = include_str!("../../../tools/precisao/sinteticos/py/pkg/heranca_apoio.py");
const HERANCA: &str = include_str!("../../../tools/precisao/sinteticos/py/pkg/heranca.py");
const REGRAS: &str = include_str!("../../../tools/precisao/sinteticos/py/pkg/heranca_regras.py");

struct Project(PathBuf);

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

fn setup(files: &[(&str, &str)]) -> Setup {
    let root = std::env::temp_dir().join(format!("heranca24b-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    fs::create_dir_all(root.join("pkg")).unwrap();
    for (name, text) in files {
        fs::write(root.join("pkg").join(name), text).unwrap();
    }
    let path = root.join("edition");
    generate(&root, &Config { project: "heranca24b".into(), inputs: vec!["pkg".into()] }, &path).unwrap();
    let catalog = Catalog::load(&path).unwrap();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    Setup { _project: Project(root), graph, syntax }
}

fn real() -> Setup {
    setup(&[("__init__.py", ""), ("heranca_apoio.py", APOIO), ("heranca.py", HERANCA), ("heranca_regras.py", REGRAS)])
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

fn check(s: &Setup, caller: &str, text: &str, how: &str, targets: &[&str]) {
    let resolver = Resolver::new(&s.graph, &s.syntax);
    let owner = symbol(&s.graph, caller);
    let occurrence = call(&s.graph, &owner, text);
    let (got, ids) = match resolver.resolve(&owner, &occurrence, None) {
        Resolution::Linked(h, t) => (h.as_str(), t),
        Resolution::Homonym(t) => ("homonym", t),
        Resolution::Overrides(t) => ("overrides", t),
        Resolution::AboveLimit(_) => ("above", vec![]),
        Resolution::NoCandidate | Resolution::OutOfChain => ("none", vec![]),
    };
    let mut labels: Vec<&str> = ids.into_iter().map(|t| s.graph.nodes[t].label.as_str()).collect();
    labels.sort();
    assert_eq!((got, labels.as_slice()), (how, targets), "{caller} {text}");
}

#[test]
fn an_inherited_method_is_found_along_the_c3_order() {
    let s = real();
    for (caller, text, target) in [
        ("c_cadeia", "x.topo()", "Raiz.topo"),
        ("c_generica", "x.topo()", "Raiz.topo"),
        ("c_apelido", "x.topo()", "Raiz.topo"),
        ("c_construtor", "n.topo()", "Raiz.topo"),
        ("c_fora_depois", "x.topo()", "Raiz.topo"),
        // Depth first would give `DA.mm`.
        ("c_diamante", "x.mm()", "DC.mm"),
        // The static type decides, whatever the object is.
        ("c_sobrescrita", "x.passo()", "Raiz.passo"),
        ("HFilho.h_proprio", "self.topo()", "Raiz.topo"),
        ("HFilho.fabrica_cls", "cls.criar()", "Raiz.criar"),
        ("HFilho.passo", "super().passo()", "Raiz.passo"),
        ("HFilho.passo_velho", "super(HFilho, self).passo()", "Raiz.passo"),
        ("c_prop", "x.transporte.enviar()", "Transporte.enviar"),
        ("c_prop_herdada", "x.transporte.enviar()", "Transporte.enviar"),
    ] {
        check(&s, caller, text, "type", &[target]);
    }
}

#[test]
fn what_the_chain_does_not_tell_stays_undecided() {
    let s = real();
    let both = ["Alheia.topo", "Raiz.topo"];
    // A base bound by branches that disagree; `__getattr__`.
    check(&s, "c_condicional", "x.topo()", "homonym", &both);
    check(&s, "c_getattr", "x.topo()", "homonym", &both);
    // A name bound to something that is not a method, by the class or by the instance.
    for caller in ["c_atributo", "c_instancia"] {
        check(&s, caller, "x.escondido()", "homonym", &["Alheia.escondido", "BaseEsc.escondido"]);
    }
    check(&s, "c_protocolo", "x.mm()", "above", &[]);
    check(&s, "c_prop_sem_tipo", "x.sem_tipo.enviar()", "homonym", &["Alheia.enviar", "Rapido.enviar", "Transporte.enviar"]);
    // A base from outside comes first: the project's later one is the only candidate left, not a link.
    check(&s, "c_fora_antes", "x.topo()", "homonym", &["Raiz.topo"]);
    // `super()` in a class whose subclass goes on elsewhere, and in a mixin without a base.
    check(&s, "CoopA.mm", "super().mm()", "above", &[]);
    check(&s, "MistSuper.mm", "super().mm()", "above", &[]);
}

#[test]
fn a_class_outside_the_chain_is_never_the_target() {
    let s = real();
    check(&s, "c_so_fora", "x.topo()", "none", &[]);
    check(&s, "c_derruba", "x.inexistente()", "none", &[]);
    check(&s, "r_fora", "x.mm()", "none", &[]);
    check(&s, "Fio.topo_de_fora", "self.enviar()", "none", &[]);
    // A subclass of the project brings it (a mixin): it stays, and `Alheia` goes.
    check(&s, "Mistura.ir", "self.vizinho()", "homonym", &["Vizinha.vizinho"]);
    // A base that is an expression, a `metaclass`, a class decorator: they may bring the method.
    for caller in ["r_base_dinamica", "r_meta", "r_decorador"] {
        check(&s, caller, "x.mm()", "above", &[]);
    }
    // `super()` does not use `__getattr__`.
    check(&s, "SuperDelegado.mm", "super().mm()", "none", &[]);
}

#[test]
fn super_never_reaches_its_own_class() {
    let s = setup(&[("m.py", "from fora import Base\n\n\nclass A(Base):\n    def so(self):\n        return super().so()\n")]);
    check(&s, "A.so", "super().so()", "none", &[]);
}

#[test]
fn an_abstract_method_gives_its_overrides() {
    let s = real();
    check(&s, "r_abstrato", "x.mandar()", "overrides", &["EnvioA.mandar", "EnvioC.mandar"]);
    check(&s, "Envio.usar", "self.mandar()", "overrides", &["EnvioA.mandar", "EnvioC.mandar"]);
    check(&s, "r_vazio", "x.vazio()", "overrides", &["EnvioA.vazio", "EnvioSuper.vazio"]);
    check(&s, "r_abc", "x.mandar()", "overrides", &["ComAbcFilha.mandar"]);
    // A concrete class in the middle; a constructor; `super()`; nobody overrides it.
    check(&s, "r_abstrato_meio", "x.mandar()", "type", &["EnvioA.mandar"]);
    check(&s, "r_construtor", "e.mandar()", "type", &["Envio.mandar"]);
    check(&s, "EnvioSuper.vazio", "super().vazio()", "type", &["Envio.vazio"]);
    check(&s, "r_sem_sobrescrita", "x.ninguem()", "unique", &["SemSobrescrita.ninguem"]);
}

#[test]
fn an_instantiation_runs_the_inherited_init() {
    let s = real();
    let resolver = Resolver::new(&s.graph, &s.syntax);
    let runs = |class: &str| resolver.instance_entry(&symbol(&s.graph, class)).map(|m| s.graph.nodes[m].label.clone());
    assert_eq!(runs("HNeto").as_deref(), Some("Raiz.__init__"));
    assert_eq!(runs("ForaDepois").as_deref(), Some("Raiz.__init__"));
    // A base from outside first; a class decorator, a `metaclass`, a `__new__`.
    for class in ["ForaAntes", "Dados", "ComMeta", "ComNew", "DA"] {
        assert_eq!(runs(class), None, "{class}");
    }
}

#[test]
fn the_overrides_above_the_limit_are_reported_and_may_be_chosen() {
    let mut text = String::from("class Base:\n    def m(self):\n        raise NotImplementedError\n\n\n");
    for i in 0..12 {
        text += &format!("class K{i}(Base):\n    def m(self):\n        return {i}\n\n\n");
    }
    text += "def usa(b: Base):\n    return b.m()\n";
    let s = setup(&[("m.py", &text)]);
    let seed = symbol(&s.graph, "usa");
    let calls = s.graph.calls_from(&seed, 2, 100, &s.syntax, &BTreeMap::new()).unwrap();
    let links: Vec<_> = calls.links.iter().filter(|l| l.from == seed).collect();
    assert_eq!(links.len(), 12);
    assert!(links.iter().all(|l| l.resolution == "homonym" && l.candidates == 12 && l.candidate_edge.is_none() && l.basis == "type" && l.type_rule.as_deref() == Some("parametro")));
    assert!(calls.unresolved.is_empty());
    let (occurrence, picked) = (links[3].occurrence.clone(), links[3].to.clone());
    let calls = s.graph.calls_from(&seed, 2, 100, &s.syntax, &BTreeMap::from([(occurrence.clone(), picked.clone())])).unwrap();
    let links: Vec<_> = calls.links.iter().filter(|l| l.from == seed).collect();
    assert_eq!((links.len(), links[0].to.as_str(), links[0].resolution.as_str()), (1, picked.as_str(), "chosen"));
    // The abstract method itself, or any other declaration, is not a choice.
    let calls = s.graph.calls_from(&seed, 2, 100, &s.syntax, &BTreeMap::from([(occurrence, symbol(&s.graph, "Base.m"))])).unwrap();
    assert_eq!(calls.links.iter().filter(|l| l.from == seed).count(), 12);
}
