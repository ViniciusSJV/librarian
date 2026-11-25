//! Step 2.4a (DEC-42), after the review of 24/11/2026: `global` and `nonlocal` bind in another scope,
//! and only a definition visible from where the call is written is picked (its own scope, or one
//! around it). One test per rule.
use librarian_ingest::{
    generate,
    graph::v2,
    resolve::{Resolution, Resolver},
    Catalog, Config,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const OUTRO: &str = "def trocada():
    return 1


def so_global():
    return 1


def fixa():
    return 1


def intocada():
    return 1


def por_nonlocal():
    return 1


def escondida():
    return 1


def no_padrao():
    return 1


def no_iter():
    return 1


def da_classe():
    return 1
";

const CASOS: &str = "def trocada():
    return 0


def fixa():
    return 0


def intocada():
    return 0


def no_padrao():
    return 0


def no_iter():
    return []


def troca(nova):
    global trocada
    trocada = nova


def cria(nova):
    global so_global
    so_global = nova


def usa_trocada():
    trocada()


def usa_so_global():
    so_global()


def usa_fixa():
    fixa()


def em_volta():
    def intocada():
        return 2

    def dentro():
        global intocada
        intocada()

    return dentro


def com_nonlocal(nova):
    def por_nonlocal():
        return 2

    def muda():
        nonlocal por_nonlocal
        por_nonlocal = nova

    def usa():
        por_nonlocal()

    return muda, usa


def esconde():
    def escondida():
        return 2

    return escondida


def usa_escondida():
    escondida()


def padrao(x=no_padrao()):
    def no_padrao():
        return 2

    return x


def iterador():
    return [no_iter for no_iter in no_iter()]


class Caixa:
    def da_classe(self):
        return 2

    def usa(self):
        da_classe()
";

const RUST: &str = "pub mod outro;

pub fn irma() -> usize {
    1
}

pub fn esconde() -> usize {
    fn escondida() -> usize {
        2
    }
    escondida()
}

pub fn usa_escondida() -> usize {
    escondida()
}

pub fn bloco_interno() -> usize {
    {
        fn irma() -> usize {
            2
        }
    }
    irma()
}

pub fn no_bloco() -> usize {
    {
        fn irma() -> usize {
            2
        }
        irma()
    }
}

pub fn com_use() -> usize {
    use crate::outro::importada;
    importada()
}

pub fn sem_use() -> usize {
    importada()
}

pub struct Tipo;

pub fn com_impl() {
    fn irma() -> usize {
        3
    }
    impl Tipo {
        pub fn chama(&self) -> usize {
            irma()
        }
    }
}
";

const RUST_OUTRO: &str = "pub fn irma() -> usize {
    4
}

pub fn escondida() -> usize {
    4
}

pub fn importada() -> usize {
    4
}

pub mod mais {
    pub fn importada() -> usize {
        5
    }
}
";

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

fn setup() -> Setup {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("librarian-visivel24a-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for d in ["src", "pkg"] {
        fs::create_dir_all(dir.join(d)).unwrap();
    }
    for (f, text) in [("src/lib.rs", RUST), ("src/outro.rs", RUST_OUTRO), ("pkg/__init__.py", ""), ("pkg/outro.py", OUTRO), ("pkg/casos.py", CASOS)] {
        fs::write(dir.join(f), text).unwrap();
    }
    let project = Project(dir);
    let path = project.0.join("edition");
    let config = Config { project: "visivel24a".into(), inputs: vec!["src".into(), "pkg".into()] };
    generate(&project.0, &config, &path).unwrap();
    let catalog = Catalog::load(&path).unwrap();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    Setup { _project: project, graph, syntax }
}

/// The resolution of the one call `text` the symbol `caller` writes in `file`, and the targets as
/// `label (path:line)`.
fn resolved(s: &Setup, file: &str, caller: &str, text: &str) -> (String, Vec<String>) {
    let g = &s.graph;
    let declared = |id: &str| g.edges.iter().find(|e| e.kind == "declares" && e.to == id).map(|e| (e.evidence.path.clone(), e.evidence.start_line));
    let found: Vec<_> = g
        .edges
        .iter()
        .filter(|e| e.kind == "observes" && e.evidence.path == file && g.nodes[&e.from].label == caller)
        .filter(|e| g.nodes[&e.to].kind == "call_observed" && g.nodes[&e.to].label == text)
        .collect();
    assert_eq!(found.len(), 1, "{caller} {text}");
    let labels = |ids: Vec<&str>| {
        let mut l: Vec<String> = ids
            .into_iter()
            .map(|id| {
                let (path, line) = declared(id).unwrap();
                format!("{} ({path}:{line})", g.nodes[id].label)
            })
            .collect();
        l.sort();
        l
    };
    match Resolver::new(g, &s.syntax).resolve(&found[0].from, &found[0].to, None) {
        Resolution::Linked(how, targets) => (how.as_str().to_owned(), labels(targets)),
        Resolution::Homonym(targets) => ("homonym".to_owned(), labels(targets)),
        Resolution::AboveLimit(n) => (format!("above_candidate_limit {n}"), vec![]),
        Resolution::NoCandidate => ("no_candidate_in_project".to_owned(), vec![]),
    }
}

fn line(text: &str, needle: &str, nth: usize) -> usize {
    text.lines().enumerate().filter(|(_, l)| l.trim_start().starts_with(needle)).nth(nth).unwrap().0 + 1
}

fn py(s: &Setup, caller: &str, text: &str) -> (String, Vec<String>) {
    resolved(s, "pkg/casos.py", caller, text)
}

fn rs(s: &Setup, caller: &str, text: &str) -> (String, Vec<String>) {
    resolved(s, "src/lib.rs", caller, text)
}

fn scope(target: &str, file: &str, line: usize) -> (String, Vec<String>) {
    ("scope".to_owned(), vec![format!("{target} ({file}:{line})")])
}

#[test]
fn python_a_global_binding_belongs_to_the_module_and_may_not_have_run() {
    let s = setup();
    // `def trocada` and `global trocada; trocada = nova` disagree: not decided, as before.
    assert_eq!(py(&s, "usa_trocada", "trocada").0, "homonym");
    // Bound only through `global`, to a variable: never outro.py's `so_global`.
    assert_eq!(py(&s, "usa_so_global", "so_global").0, "no_candidate_in_project");
    assert_eq!(py(&s, "usa_fixa", "fixa"), scope("fixa", "pkg/casos.py", line(CASOS, "def fixa", 0)));
}

#[test]
fn python_a_name_declared_global_is_the_modules_not_the_enclosing_functions() {
    let s = setup();
    assert_eq!(py(&s, "em_volta.dentro", "intocada"), scope("intocada", "pkg/casos.py", line(CASOS, "def intocada", 0)));
}

#[test]
fn python_a_nonlocal_binding_belongs_to_the_enclosing_function() {
    let s = setup();
    assert_eq!(py(&s, "com_nonlocal.usa", "por_nonlocal").0, "homonym");
}

#[test]
fn python_a_definition_that_is_not_visible_is_not_picked() {
    let s = setup();
    // Nested in another function of the file.
    assert_eq!(py(&s, "usa_escondida", "escondida").0, "no_candidate_in_project");
    // A default is evaluated outside the function: not its nested `no_padrao`.
    assert_eq!(py(&s, "padrao", "no_padrao"), scope("no_padrao", "pkg/casos.py", line(CASOS, "def no_padrao", 0)));
    // The first iterable of a comprehension is evaluated outside it.
    assert_eq!(py(&s, "iterador", "no_iter"), scope("no_iter", "pkg/casos.py", line(CASOS, "def no_iter", 0)));
    // A class body is not seen from its methods.
    assert_eq!(py(&s, "Caixa.usa", "da_classe").0, "no_candidate_in_project");
}

#[test]
fn rust_a_definition_that_is_not_visible_is_not_picked() {
    let s = setup();
    // Nested in another function of the file.
    assert_eq!(rs(&s, "usa_escondida", "escondida").0, "no_candidate_in_project");
    assert_eq!(rs(&s, "esconde", "escondida").0, "scope");
    // In an inner block that ended: the module's. In that block: the block's.
    assert_eq!(rs(&s, "bloco_interno", "irma"), scope("irma", "src/lib.rs", line(RUST, "pub fn irma", 0)));
    assert_eq!(rs(&s, "no_bloco", "irma"), scope("no_bloco::irma", "src/lib.rs", line(RUST, "fn irma", 1)));
    // An item of the block around an `impl` is seen from its methods.
    assert_eq!(rs(&s, "com_impl::impl Tipo::chama", "irma"), scope("com_impl::irma", "src/lib.rs", line(RUST, "fn irma", 2)));
}

#[test]
fn rust_a_use_in_a_function_counts_only_there() {
    let s = setup();
    assert_eq!(rs(&s, "com_use", "importada"), ("import".to_owned(), vec!["importada (src/outro.rs:9)".to_owned()]));
    assert_eq!(rs(&s, "sem_use", "importada").0, "no_candidate_in_project");
}
