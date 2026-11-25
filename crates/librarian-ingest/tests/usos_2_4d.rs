//! Step 2.4d (DEC-46): "where is it used?" is the inverse of "what does it call?". The invariant is checked
//! in both directions over the gate's synthetic projects and a small one with every kind of use.
use librarian_ingest::{
    generate,
    graph::{
        uses::{UseIndex, Uses},
        v2,
    },
    Catalog, Config,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

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
    index: UseIndex,
}

fn built(root: &Path, input: &str, project: Project) -> Setup {
    let path = project.0.join("edition");
    generate(root, &Config { project: "usos24d".into(), inputs: vec![input.into()] }, &path).unwrap();
    let catalog = Catalog::load(&path).unwrap();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    let index = UseIndex::build(&graph, &syntax);
    Setup { _project: project, graph, syntax, index }
}

fn temp() -> Project {
    let root = std::env::temp_dir().join(format!("usos24d-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    fs::create_dir_all(&root).unwrap();
    Project(root)
}

fn setup(files: &[(&str, &str)]) -> Setup {
    let project = temp();
    for (name, text) in files {
        let path = project.0.join("pkg").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let root = project.0.clone();
    built(&root, "pkg", project)
}

fn synthetic(language: &str, input: &str) -> Setup {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/precisao/sinteticos").join(language).canonicalize().unwrap();
    built(&root, input, temp())
}

fn id(s: &Setup, label: &str) -> String {
    let found: Vec<_> = s.graph.nodes.values().filter(|n| n.category == "symbol" && n.label == label).collect();
    assert_eq!(found.len(), 1, "{label}");
    found[0].id.clone()
}

fn used(s: &Setup, label: &str, depth: usize) -> Uses {
    s.index.used_by(&s.graph, &id(s, label), depth, 120).unwrap()
}

/// The direct uses of one kind, as `who: text`.
fn direct(s: &Setup, label: &str, kind: &str, basis: Option<&str>) -> Vec<String> {
    let seed = id(s, label);
    let mut out: Vec<String> = used(s, label, 1)
        .links
        .iter()
        .filter(|l| l.to == seed && l.kind == kind && (basis.is_none() || l.basis == basis))
        .map(|l| format!("{}: {}", s.graph.nodes[&l.from].label, l.text.lines().next().unwrap()))
        .collect();
    out.sort();
    out
}

type Key = (String, String, String, String, String, Option<String>);

/// Both directions, over every symbol: every link of `calls_from` is a use of its target, of the kind it
/// says, and every linked or possible use is a link of `calls_from`. Returns how many of each resolution.
fn invariant(s: &Setup) -> BTreeMap<String, usize> {
    let mut forward: BTreeSet<Key> = BTreeSet::new();
    let mut backward: BTreeSet<Key> = BTreeSet::new();
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    for node in s.graph.nodes.values().filter(|n| n.category == "symbol") {
        let calls = s.graph.calls_from(&node.id, 1, 300, &s.syntax, &BTreeMap::new()).unwrap();
        assert!(calls.unresolved.iter().all(|u| u.reason != "node_limit"));
        for l in &calls.links {
            let kind = if l.resolution == "homonym" { format!("possible {}", l.basis) } else { format!("linked {}", l.relation) };
            *by.entry(format!("{} {}", l.resolution, l.relation)).or_default() += 1;
            let key = |to: &str, instance: Option<String>| (l.from.clone(), to.to_owned(), l.occurrence.clone(), kind.clone(), l.resolution.clone(), instance);
            // The field answers for the call too; the use names the method that declares it, with `via`.
            let field = node.kind == "field" && l.via.is_none()
                && s.graph.edges.iter().any(|e| e.kind == "contains" && e.to == node.id && matches!(s.graph.nodes[&e.from].kind.as_str(), "method" | "function"));
            if field {
                let uses = s.index.used_by(&s.graph, &l.to, 1, 300).unwrap();
                assert!(uses.links.iter().any(|u| u.occurrence == l.occurrence && u.via.as_deref() == Some(node.id.as_str())));
                continue;
            }
            forward.insert(key(&l.to, None));
            if let Some(init) = &l.runs {
                forward.insert(key(init, Some(l.to.clone())));
            }
        }
        let uses = s.index.used_by(&s.graph, &node.id, 1, 1).unwrap();
        let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
        for u in &uses.links {
            // One kind per occurrence, whatever the cap on the drawing.
            assert!(seen.insert((&u.occurrence, &u.to)) , "{u:?}");
            if u.kind == "name" {
                assert!(u.resolution.is_none() && u.basis.is_none());
                continue;
            }
            let kind = if u.kind == "possible" { format!("possible {}", u.basis.unwrap()) } else { format!("linked {}", u.relation.unwrap()) };
            backward.insert((u.from.clone(), u.to.clone(), u.occurrence.clone(), kind, u.resolution.unwrap().to_owned(), u.instance_of.clone()));
        }
        // What is not a use is not also one.
        for aside in uses.above_limit.iter().chain(&uses.abstract_calls).chain(&uses.ruled_out) {
            assert!(!seen.contains(&(aside.occurrence.as_str(), node.id.as_str())), "{aside:?}");
        }
        let c = &uses.counts;
        assert_eq!(c.linked + c.possible_type + c.possible_homonym + c.name, uses.links.len());
        assert_eq!((c.above_limit, c.abstract_calls, c.ruled_out), (uses.above_limit.len(), uses.abstract_calls.len(), uses.ruled_out.len()));
    }
    let only: Vec<_> = forward.symmetric_difference(&backward).collect();
    assert!(only.is_empty(), "{} {only:#?}", only.len());
    by
}

#[test]
fn every_link_is_a_use_and_every_use_a_link_in_the_python_synthetic() {
    let by = invariant(&synthetic("py", "pkg"));
    for how in ["unique calls", "type calls", "receiver calls", "scope calls", "import calls", "homonym calls", "unique instantiates"] {
        assert!(by.contains_key(how), "{how}: {by:?}");
    }
}

#[test]
fn every_link_is_a_use_and_every_use_a_link_in_the_rust_synthetic() {
    let by = invariant(&synthetic("rs", "src"));
    for how in ["unique calls", "type calls", "trait_default calls", "scope calls", "homonym calls"] {
        assert!(by.contains_key(how), "{how}: {by:?}");
    }
}

const CORPOS: &str = "class Base:
    def rodar(self):
        raise NotImplementedError()

    def copiar(self):
        return 1

    def usar(self):
        self.rodar()
        self.copiar()


class Um(Base):
    def rodar(self):
        return 1


class Dois(Base):
    def rodar(self):
        return 2

    def copiar(self):
        return 2


class Motor:
    def __init__(self):
        self.x = Um()

    def ligar(self):
        return self.x


class Carro:
    def __init__(self):
        self.motor = Motor()


def marcar(nome):
    return nome


@marcar('a')
def montar():
    c = Carro()
    return c.motor


def passar():
    montar()
    return marcar


def copiar():
    return 3


def solto(x):
    x.copiar()
";

fn corpos() -> Setup {
    setup(&[("__init__.py", ""), ("corpos.py", CORPOS)])
}

#[test]
fn the_kinds_are_apart() {
    let s = corpos();
    invariant(&s);
    // The bodies that may run; the abstract one, aside, is in none of the kinds.
    assert_eq!(direct(&s, "Um.rodar", "possible", Some("type")), ["Base.usar: self.rodar()"]);
    assert_eq!(direct(&s, "Dois.rodar", "possible", Some("type")), ["Base.usar: self.rodar()"]);
    let abstract_ = used(&s, "Base.rodar", 1);
    assert_eq!((abstract_.links.len(), abstract_.counts.abstract_calls, abstract_.counts.ruled_out), (0, 1, 0));
    // A homonym is possible too, in another group.
    assert_eq!(direct(&s, "Dois.copiar", "possible", Some("type")), ["Base.usar: self.copiar()"]);
    assert_eq!(direct(&s, "Dois.copiar", "possible", Some("graph_edge")), ["solto: x.copiar()"]);
    // The plain function is ruled out of both calls, by their form: counted, never a use.
    let plain = used(&s, "copiar", 1);
    assert_eq!((plain.links.len(), plain.counts.ruled_out), (0, 2));
    // A name that is not a call; and a decorator, linked with its relation.
    assert_eq!(direct(&s, "marcar", "name", None), ["passar: marcar"]);
    let decorated = used(&s, "marcar", 1);
    assert_eq!(decorated.links.iter().filter(|l| l.kind == "linked").map(|l| l.relation.unwrap()).collect::<Vec<_>>(), ["decorates"]);
}

#[test]
fn an_instantiation_uses_the_class_and_the_init_it_runs() {
    let s = corpos();
    assert_eq!(direct(&s, "Carro", "linked", None), ["montar: Carro"]);
    let init = used(&s, "Carro.__init__", 1);
    assert_eq!((init.links.len(), init.links[0].relation, init.links[0].instance_of.as_deref()), (1, Some("instantiates"), Some(id(&s, "Carro").as_str())));
    // Written in a field's value: the method is the user, with `via`.
    let motor = used(&s, "Motor.__init__", 1);
    assert_eq!((s.graph.nodes[&motor.links[0].from].label.as_str(), motor.links[0].via.as_deref()), ("Carro.__init__", Some(id(&s, "Carro.motor").as_str())));
}

#[test]
fn the_walk_goes_up_by_the_linked_uses_only_and_never_by_a_decorator() {
    let s = corpos();
    let layers = |label: &str| -> Vec<(String, usize)> {
        let mut out: Vec<_> = used(&s, label, 3).nodes.iter().map(|n| (n.label.clone(), n.layer)).collect();
        out.sort();
        out
    };
    let at = |l: &str, n: usize| (l.to_owned(), n);
    assert_eq!(layers("Motor.__init__"), [at("Carro.__init__", 1), at("Motor.__init__", 0), at("montar", 2), at("passar", 3)]);
    // `montar` is decorated by it; `passar` writes its name. Neither is gone up from.
    assert_eq!(layers("marcar"), [at("marcar", 0), at("montar", 1), at("passar", 1)]);
    // A body that may run is shown and stops; a homonym is in the list of the direct uses, never drawn.
    assert_eq!(layers("Dois.copiar"), [at("Base.usar", 1), at("Dois.copiar", 0)]);
    assert_eq!(direct(&s, "Dois.copiar", "possible", Some("graph_edge")), ["solto: x.copiar()"]);
    assert!(!used(&s, "Dois.copiar", 1).truncated);
}

#[test]
fn the_cap_cuts_the_drawing_and_the_linked_uses_last() {
    let s = corpos();
    let uses = s.index.used_by(&s.graph, &id(&s, "marcar"), 1, 2).unwrap();
    assert!(uses.truncated);
    assert_eq!(uses.nodes.iter().map(|n| n.label.as_str()).collect::<Vec<_>>(), ["marcar", "montar"]);
    assert_eq!((uses.links.len(), uses.counts.linked, uses.counts.name), (2, 1, 1));
    assert!(s.index.used_by(&s.graph, "nada", 1, 10).is_err());
    assert!(s.index.used_by(&s.graph, &id(&s, "marcar"), 0, 10).is_err());
    assert!(s.index.used_by(&s.graph, &id(&s, "marcar"), 1, 301).is_err());
}

#[test]
fn calls_above_the_limit_are_a_gap_filtered_by_the_form_and_the_type() {
    let many: String = (0..10).map(|i| format!("class C{i}:\n    def abrir(self):\n        return {i}\n\n\n")).collect();
    let s = setup(&[("__init__.py", ""), ("muitos.py", &format!("{many}def abrir():\n    return 0\n\n\ndef usa(x):\n    x.abrir()\n\n\ndef conhecido(x: C1):\n    x.abrir()\n"))]);
    invariant(&s);
    let gap = used(&s, "C0.abrir", 1);
    assert_eq!((gap.links.len(), gap.counts.above_limit, gap.above_limit[0].candidates), (0, 1, 11));
    // Settled by the type: a use, and not in anyone's gap.
    let known = used(&s, "C1.abrir", 1);
    assert_eq!((known.counts.linked, known.counts.above_limit), (1, 1));
    // `x.abrir()` does not reach a plain function.
    assert_eq!(used(&s, "abrir", 1).counts.above_limit, 0);

    let impls: String = (0..9).map(|i| format!("pub struct A{i};\nimpl From<A{i}> for Alvo {{\n    fn from(_: A{i}) -> Self {{ Alvo }}\n}}\n")).collect();
    let s = setup(&[("lib.rs", &format!("pub struct Alvo;\npub struct Outro;\n{impls}impl Outro {{\n    pub fn from(_: u8) -> Self {{ Outro }}\n}}\npub fn usa() {{\n    Alvo::from(A0);\n    f64::from(1u8);\n}}\n"))]);
    invariant(&s);
    // The type rules out `Alvo::from(..)` for `Outro`; it does not rule out by a primitive (known limit).
    let texts = |label: &str| used(&s, label, 1).above_limit.iter().map(|a| a.text.clone()).collect::<Vec<_>>();
    assert_eq!(texts("impl From<A3> for Alvo::from"), ["Alvo::from", "f64::from"]);
    assert_eq!(texts("impl Outro::from"), ["f64::from"]);
}
