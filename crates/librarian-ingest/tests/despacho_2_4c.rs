//! Step 2.4c (DEC-44): the body that runs is the object's. With the receiver's static type, a method that
//! a subclass of the project overrides is ambiguous: the type's own body and the overrides. The source
//! is the gate's synthetic project, checked in a real Python (docs/reports/fase-2/dados/2.4c).
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

const DESPACHO: &str = include_str!("../../../tools/precisao/sinteticos/py/pkg/despacho.py");

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
    let root = std::env::temp_dir().join(format!("despacho24c-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    fs::create_dir_all(root.join("pkg")).unwrap();
    for (name, text) in files {
        fs::write(root.join("pkg").join(name), text).unwrap();
    }
    let path = root.join("edition");
    generate(&root, &Config { project: "despacho24c".into(), inputs: vec!["pkg".into()] }, &path).unwrap();
    let catalog = Catalog::load(&path).unwrap();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    Setup { _project: Project(root), graph, syntax }
}

fn real() -> Setup {
    setup(&[("__init__.py", ""), ("despacho.py", DESPACHO)])
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

const ANDAR: [&str; 4] = ["Carro.andar", "Irma.andar", "Moto.andar", "Veiculo.andar"];

#[test]
fn an_overridden_method_gives_every_body_that_may_run() {
    let s = real();
    for (caller, text) in [
        ("d_parametro", "x.andar()"),
        ("d_construtor_anotado", "x.andar()"),
        ("d_retorno", "x.andar()"),
        ("Veiculo.rodar", "self.andar()"),
        ("Garagem.do_campo", "self.veiculo.andar()"),
        ("Garagem.da_propriedade", "self.atual.andar()"),
    ] {
        check(&s, caller, text, "overrides", &ANDAR);
    }
    check(&s, "Veiculo.montar", "cls.nome()", "overrides", &["Carro.nome", "Veiculo.nome"]);
    check(&s, "d_abstrato_meio", "x.area()", "overrides", &["Quadrado.area", "Solida.area"]);
    // The abstract ones go (DEC-43).
    check(&s, "d_abstrato", "x.area()", "overrides", &["Quadrado.area", "Solida.area"]);
    // What each subclass reaches, not only what it declares: a mixin before the base.
    check(&s, "d_mistura", "x.abrir()", "overrides", &["Ficha.abrir", "MisturaFicha.abrir"]);
    check(&s, "Ficha.usar", "self.abrir()", "overrides", &["Ficha.abrir", "MisturaFicha.abrir"]);
}

#[test]
fn without_an_override_or_with_a_known_body_the_link_stays() {
    let s = real();
    // No subclass overrides it; only a sibling declares the name.
    check(&s, "d_sem_sobrescrita", "x.parar()", "unique", &["Veiculo.parar"]);
    check(&s, "Veiculo.frear", "self.parar()", "unique", &["Veiculo.parar"]);
    check(&s, "d_meio", "x.andar()", "type", &["Carro.andar"]);
    check(&s, "d_irma", "x.andar()", "type", &["Moto.andar"]);
    // A constructor, `super()` and the class's name.
    check(&s, "d_construtor", "x.andar()", "type", &["Veiculo.andar"]);
    check(&s, "d_construtor_direto", "Veiculo().andar()", "type", &["Veiculo.andar"]);
    check(&s, "Carro.pelo_super", "super().andar()", "type", &["Veiculo.andar"]);
    // Changed in step 2.4d-0 (DEC-45): until then the class's name gave the receiver no type, and the call was
    // `homonym` with the four `andar`. Now the class is used: the body is the one `Veiculo` declares, never a
    // subclass's, because the call names the class and not an object (a real Python runs `Veiculo.andar`).
    check(&s, "Veiculo.pelo_nome", "Veiculo.andar(self)", "type", &["Veiculo.andar"]);
    // A base from outside before the class, in a subclass, does not count (a limit).
    check(&s, "d_fora_antes", "x.girar()", "unique", &["Motor.girar"]);
}

#[test]
fn a_new_that_builds_another_class_is_not_an_exact_constructor() {
    let s = real();
    check(&s, "d_novo", "x.agir()", "overrides", &["ComNovo.agir", "ComNovoFilha.agir"]);
    // A class decorator and a `metaclass`, also of a base.
    for (head, class) in [("@deco\nclass A:", "A"), ("class A(metaclass=M):", "A"), ("class Z(metaclass=M):\n    pass\n\n\nclass A(Z):", "A")] {
        let text = format!("{head}\n    def m(self):\n        return 1\n\n\nclass B(A):\n    def m(self):\n        return 2\n\n\ndef usa():\n    x = {class}()\n    return x.m()\n");
        let s = setup(&[("m.py", &text)]);
        check(&s, "usa", "x.m()", "overrides", &["A.m", "B.m"]);
    }
}

#[test]
fn a_private_name_is_not_overridden() {
    let s = real();
    check(&s, "Veiculo.ligar", "self.__motor()", "receiver", &["Veiculo.__motor"]);
}

#[test]
fn a_subclass_that_binds_the_name_leaves_no_single_link() {
    let s = real();
    check(&s, "d_atributo", "x.trancar()", "overrides", &["Porta.trancar"]);
    // By the instance, out of `__init__`; with another method of that name too.
    let s = setup(&[("m.py", "class A:\n    def m(self):\n        return 1\n\n    def usa(self):\n        return self.m()\n\n\nclass B(A):\n    def poe(self, f):\n        self.m = f\n\n\nclass C(A):\n    def m(self):\n        return 3\n")]);
    check(&s, "A.usa", "self.m()", "overrides", &["A.m", "C.m"]);
}

#[test]
fn an_empty_override_of_a_concrete_method_counts() {
    let s = setup(&[("m.py", "class A:\n    def m(self):\n        return 1\n\n\nclass Mudo(A):\n    def m(self):\n        pass\n\n\ndef usa(x: A):\n    return x.m()\n")]);
    check(&s, "usa", "x.m()", "overrides", &["A.m", "Mudo.m"]);
}

#[test]
fn an_empty_override_between_a_concrete_method_and_another_override_runs() {
    let text = "class A:\n    def m(self):\n        return 1\n\n\nclass Meio(A):\n    def m(self):\n        pass\n\n\nclass Fim(Meio):\n    def m(self):\n        return 3\n\n\ndef do_alto(x: A):\n    return x.m()\n\n\ndef do_meio(x: Meio):\n    return x.m()\n\n\ndef do_fim(x: Fim):\n    return x.m()\n";
    let s = setup(&[("m.py", text)]);
    check(&s, "do_alto", "x.m()", "overrides", &["A.m", "Fim.m", "Meio.m"]);
    // The written type's own empty body runs too: there is a concrete one above it.
    check(&s, "do_meio", "x.m()", "overrides", &["Fim.m", "Meio.m"]);
    check(&s, "do_fim", "x.m()", "type", &["Fim.m"]);
    // The same body with nothing concrete above it is abstract (DEC-43), also when inherited by the
    // written type; an empty body under a concrete override runs.
    let text = "class A:\n    def m(self):\n        pass\n\n\nclass Meio(A):\n    def m(self):\n        ...\n\n\nclass Fim(Meio):\n    def m(self):\n        return 3\n\n\nclass Mudo(Fim):\n    def m(self):\n        pass\n\n\nclass Herda(A):\n    pass\n\n\nclass HerdaFim(Herda):\n    def m(self):\n        return 4\n\n\ndef do_alto(x: A):\n    return x.m()\n\n\ndef do_meio(x: Meio):\n    return x.m()\n\n\ndef do_herda(x: Herda):\n    return x.m()\n";
    let s = setup(&[("m.py", text)]);
    check(&s, "do_alto", "x.m()", "overrides", &["Fim.m", "HerdaFim.m", "Mudo.m"]);
    check(&s, "do_meio", "x.m()", "overrides", &["Fim.m", "Mudo.m"]);
    check(&s, "do_herda", "x.m()", "overrides", &["HerdaFim.m"]);
    // `@abstractmethod` is abstract whatever is above it: its class builds no object.
    let text = "from abc import ABC, abstractmethod\n\n\nclass A:\n    def m(self):\n        return 1\n\n\nclass Meio(A, ABC):\n    @abstractmethod\n    def m(self):\n        return 2\n\n\nclass Fim(Meio):\n    def m(self):\n        return 3\n\n\ndef do_alto(x: A):\n    return x.m()\n\n\ndef do_meio(x: Meio):\n    return x.m()\n";
    let s = setup(&[("m.py", text)]);
    check(&s, "do_alto", "x.m()", "overrides", &["A.m", "Fim.m"]);
    check(&s, "do_meio", "x.m()", "overrides", &["Fim.m"]);
}

#[test]
fn the_link_names_the_abstract_method_that_left_the_list() {
    let text = "class A:\n    def m(self):\n        raise NotImplementedError\n\n    def n(self):\n        return 1\n\n\nclass B(A):\n    def m(self):\n        return 2\n\n    def n(self):\n        return 3\n\n\ndef usa(x: A):\n    x.m()\n    x.n()\n";
    let s = setup(&[("m.py", text)]);
    let seed = symbol(&s.graph, "usa");
    let calls = s.graph.calls_from(&seed, 1, 100, &s.syntax, &BTreeMap::new()).unwrap();
    let of = |text: &str| -> Vec<Option<String>> { calls.links.iter().filter(|l| l.from == seed && l.text == text).map(|l| l.abstract_method.clone()).collect() };
    assert_eq!(of("x.m()"), vec![Some(symbol(&s.graph, "A.m"))]);
    assert_eq!(of("x.n()"), vec![None, None]);
}

#[test]
fn the_bodies_are_reported_by_the_type_and_any_of_them_may_be_chosen() {
    let mut text = String::from("class Base:\n    def m(self):\n        return 0\n\n\n");
    for i in 0..12 {
        text += &format!("class K{i}(Base):\n    def m(self):\n        return {i}\n\n\n");
    }
    text += "def usa(b: Base):\n    return b.m()\n";
    let s = setup(&[("m.py", &text)]);
    let seed = symbol(&s.graph, "usa");
    let calls = s.graph.calls_from(&seed, 2, 100, &s.syntax, &BTreeMap::new()).unwrap();
    let links: Vec<_> = calls.links.iter().filter(|l| l.from == seed).collect();
    assert_eq!(links.len(), 13);
    assert!(links.iter().all(|l| l.resolution == "homonym" && l.candidates == 13 && l.candidate_edge.is_none() && l.basis == "type" && l.type_rule.as_deref() == Some("parametro")));
    assert!(calls.unresolved.is_empty());
    let (occurrence, base) = (links[0].occurrence.clone(), symbol(&s.graph, "Base.m"));
    let calls = s.graph.calls_from(&seed, 2, 100, &s.syntax, &BTreeMap::from([(occurrence, base.clone())])).unwrap();
    let links: Vec<_> = calls.links.iter().filter(|l| l.from == seed).collect();
    assert_eq!((links.len(), links[0].to.as_str(), links[0].resolution.as_str()), (1, base.as_str(), "chosen"));
}
