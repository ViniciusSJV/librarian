//! Step 2.3b (DEC-40): who runs a call written in a decorator or in a field's value. One test
//! per rule, one showing that each call is counted once by everything that counts calls by owner,
//! and one with the text a chain step gives the model.
use librarian_ingest::{
    envelope, generate,
    graph::v2,
    inventory,
    owner,
    prepare::{self, Request},
    Catalog, Config,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const DECO: &str = "def guarda(nome):
    open(nome)
    def decorador(f):
        return interno(f)
    return decorador


def interno(f):
    return f


class Erro(Exception):
    pass
";

const MODELS: &str = "from pkg.deco import guarda, Erro


class Repo:
    def salvar(self, x):
        return x


class Sink:
    def __init__(self):
        self.n = 0


class Servico:
    def __init__(self):
        self.repo = Repo()
        self.sink = Sink()

    def gravar(self):
        self.repo.salvar(1)


@guarda(\"X\")
def view():
    return Repo()
    open(\"v\")


class Teste:
    @guarda(nome=Erro(\"falhou\"))
    def test_um(self):
        return 1
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-dono23b-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("pkg/__init__.py"), "").unwrap();
        fs::write(dir.join("pkg/deco.py"), DECO).unwrap();
        fs::write(dir.join("pkg/models.py"), MODELS).unwrap();
        Self(dir)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Setup {
    project: Project,
    catalog: Catalog,
    graph: v2::Graph,
    syntax: v2::CallSyntax,
}

fn setup() -> Setup {
    let project = Project::new();
    let path = project.0.join("edition");
    let config = Config { project: "dono23b".into(), inputs: vec!["pkg".into()] };
    drop(generate(&project.0, &config, &path).unwrap());
    let catalog = Catalog::load(&path).unwrap();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    Setup { project, catalog, graph, syntax }
}

impl Setup {
    fn id(&self, label: &str) -> String {
        let found: Vec<_> = self.graph.nodes.values().filter(|n| n.category == "symbol" && n.label == label).collect();
        assert_eq!(found.len(), 1, "{label}");
        found[0].id.clone()
    }

    fn label(&self, id: &str) -> String {
        self.graph.nodes[id].label.clone()
    }

    fn calls(&self, label: &str, depth: usize) -> v2::Calls {
        self.graph.calls_from(&self.id(label), depth, 50, &self.syntax, &BTreeMap::new()).unwrap()
    }
}

fn link<'c>(calls: &'c v2::Calls, text: &str) -> &'c v2::CallLink {
    calls.links.iter().find(|l| l.text == text).unwrap_or_else(|| panic!("{text}"))
}

#[test]
fn a_decorators_call_decorates_and_is_not_followed() {
    let s = setup();
    let calls = s.calls("view", 3);
    // `@guarda("X")`: the one `guarda`, but it runs when the `def` runs; the walk stops there.
    let guarda = link(&calls, "guarda");
    assert_eq!((s.label(&guarda.to).as_str(), guarda.resolution.as_str(), guarda.relation.as_str()), ("guarda", "unique", "decorates"));
    assert_eq!(guarda.from, s.id("view"));
    assert!(!calls.nodes.iter().any(|n| n.label == "guarda.decorador" || n.label == "interno"));
    // The body's call is the view's own.
    assert_eq!(link(&calls, "Repo").relation, "instantiates");
    // A class built inside a decorator's keyword argument also runs with the `def`: `decorates`,
    // and its `__init__` is not followed.
    let teste = s.calls("Teste.test_um", 2);
    let erro = link(&teste, "Erro");
    assert_eq!((erro.relation.as_str(), erro.runs.as_deref()), ("decorates", None));
    // The inventory does not follow a decorator either: `view` reaches `Repo`, never `guarda`.
    let found = inventory::inventory(&s.catalog, &inventory::RuleSet::default()).unwrap();
    let row = |label: &str| found.symbols.iter().find(|r| r.symbol == s.id(label)).unwrap_or_else(|| panic!("{label}"));
    assert!(!row("view").calls.contains(&s.id("guarda")));
    assert!(row("guarda").called_by.is_empty());
    let _ = &s.project;
}

#[test]
fn a_method_answers_for_the_fields_it_declares() {
    let s = setup();
    let init = s.id("Servico.__init__");
    let calls = s.calls("Servico.__init__", 2);
    // `self.repo = Repo()` and `self.sink = Sink()`: written in the fields, run by `__init__`.
    let repo = link(&calls, "Repo");
    assert_eq!((repo.from.as_str(), repo.via.as_deref()), (init.as_str(), Some(s.id("Servico.repo").as_str())));
    assert_eq!((repo.relation.as_str(), repo.resolution.as_str()), ("instantiates", "unique"));
    let sink = link(&calls, "Sink");
    assert_eq!(sink.runs.as_deref().map(|r| s.label(r)).as_deref(), Some("Sink.__init__"));
    assert!(calls.nodes.iter().any(|n| n.label == "Sink.__init__"));
    // The field still answers for its own value, with the same answer and no `via`.
    let field = s.calls("Servico.repo", 1);
    let own = link(&field, "Repo");
    assert_eq!((own.from.as_str(), own.via.as_deref(), own.resolution.as_str(), own.relation.as_str()),
        (s.id("Servico.repo").as_str(), None, "unique", "instantiates"));
    // A method that declares no field answers as before.
    assert!(s.calls("Servico.gravar", 1).links.iter().all(|l| l.via.is_none()));
}

#[test]
fn each_call_is_counted_once() {
    let s = setup();
    let symbols: Vec<String> = s.graph.nodes.values().filter(|n| n.category == "symbol").map(|n| n.id.clone()).collect();
    let calls: BTreeSet<&str> = s.graph.edges.iter()
        .filter(|e| e.kind == "observes" && matches!(s.graph.nodes[&e.to].kind.as_str(), "call_observed" | "method_call_observed"))
        .map(|e| e.to.as_str()).collect();
    // `/api/calls`: in each answer an occurrence appears once. Counted by the symbol the graph
    // says observes it (links without `via`), each call is counted exactly once over all symbols.
    let mut owned: BTreeMap<String, usize> = BTreeMap::new();
    for id in &symbols {
        let answer = s.graph.calls_from(id, 1, 300, &s.syntax, &BTreeMap::new()).unwrap();
        // A call is a link (several only for a homonym) or an unresolved entry, never both.
        let linked: BTreeMap<&str, Option<&String>> = answer.links.iter().map(|l| (l.occurrence.as_str(), l.via.as_ref())).collect();
        for u in &answer.unresolved {
            assert!(!linked.contains_key(u.occurrence.as_str()), "{} twice in the answer of {id}", u.occurrence);
        }
        let written = linked.into_iter().chain(answer.unresolved.iter().map(|u| (u.occurrence.as_str(), u.via.as_ref())));
        for (occurrence, via) in written {
            if via.is_none() {
                *owned.entry(occurrence.to_owned()).or_default() += 1;
            }
        }
    }
    assert_eq!(owned.keys().map(String::as_str).collect::<BTreeSet<_>>(), calls);
    assert!(owned.values().all(|n| *n == 1), "{owned:?}");

    // Owner context (`called`, what the chain step and `prepare` add): asked for the method and
    // its two fields, `Repo` and `Sink` come once each, with one line each.
    let targets = [s.id("Servico.__init__"), s.id("Servico.repo"), s.id("Servico.sink")];
    let (added, _) = owner::called(&s.catalog, &targets, &targets, 10).unwrap();
    let names: Vec<(String, usize)> = added.iter().map(|a| (a.name.clone(), a.evidence.len())).collect();
    assert_eq!(names, [("Repo".to_owned(), 1), ("Sink".to_owned(), 1)]);

    // Envelope: the same three chosen, one `called` reason for each call.
    let suggestion = envelope::candidates(&s.catalog, &targets, None).unwrap();
    let called: Vec<(String, usize)> = suggestion.items.iter()
        .map(|i| (i.name.clone(), i.reasons.iter().filter(|r| r.code == "called").count()))
        .filter(|(_, n)| *n > 0).collect();
    assert_eq!(called, [("Repo".to_owned(), 1), ("Sink".to_owned(), 1)]);

    // Inventory: by the symbol the graph says observes each call, so the field's call is never
    // the method's too. Its operations are one per occurrence and rule.
    let found = inventory::inventory(&s.catalog, &inventory::RuleSet::default()).unwrap();
    let ops: BTreeSet<(&str, &str)> = found.operations.iter().map(|o| (o.occurrence.as_str(), o.rule.as_str())).collect();
    assert_eq!(ops.len(), found.operations.len());
}

#[test]
fn a_chain_step_says_what_the_link_is() {
    let s = setup();
    let request = |name: &str, line: usize| -> Request {
        serde_json::from_value(json!({
            "question": "O que roda?", "top": 5, "depth": 1, "hits": 1, "radius": 0, "numbered_lines": true,
            "targets": [{"path": "pkg/models.py", "symbol_name": name, "declaration_line": line}],
            "owner_context": true, "callees": true
        })).unwrap()
    };
    let facts = |name: &str, line: usize| -> String {
        let prepared = prepare::prepare(&s.catalog, &request(name, line)).unwrap();
        let dossier: Value = serde_json::from_str(&prepared.dossier).unwrap();
        dossier["facts"].to_string()
    };
    // `view` (line 23, its decorator): `guarda` is its decorator, `Repo` is built by it.
    let view = facts("view", 23);
    assert!(view.contains("Decorador de `view` (linha 23): é a única declaração com esse nome na edition. A chamada roda quando a declaração de `view` é executada"), "{view}");
    assert!(view.contains("Instanciada por `view` (linha 25): é a única declaração com esse nome na edition. A chamada constrói um valor dela"), "{view}");
    assert!(!view.contains("Chamada por `view`"), "{view}");
    // `__init__` brings what its fields build, once each.
    let init = facts("__init__", 15);
    assert!(init.contains("Instanciada por `Servico.__init__` (linha 16)"), "{init}");
    assert!(init.contains("Instanciada por `Servico.__init__` (linha 17)"), "{init}");
}
