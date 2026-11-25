//! Step 2.4d-0 (DEC-45): `Class.m(..)` with a class of the project resolves along the class's own chain of
//! bases, upwards, and never goes down to a subclass. The sources are the gate's synthetic project, checked
//! in a real Python (docs/reports/fase-2/dados/2.4d0/roda2.out).
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

const NOME: &str = include_str!("../../../tools/precisao/sinteticos/py/pkg/classe_nome.py");
const FORA: &str = include_str!("../../../tools/precisao/sinteticos/py/pkg/classe_nome_fora.py");
const USOS: &str = include_str!("../../../tools/precisao/sinteticos/py/pkg/classe_nome_usos.py");

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
    let root = std::env::temp_dir().join(format!("classenome24d0-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    fs::create_dir_all(root.join("pkg")).unwrap();
    for (name, text) in files {
        fs::write(root.join("pkg").join(name), text).unwrap();
    }
    let path = root.join("edition");
    generate(&root, &Config { project: "classenome24d0".into(), inputs: vec!["pkg".into()] }, &path).unwrap();
    let catalog = Catalog::load(&path).unwrap();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    Setup { _project: Project(root), graph, syntax }
}

fn real() -> Setup {
    setup(&[("__init__.py", ""), ("classe_nome.py", NOME), ("classe_nome_fora.py", FORA), ("classe_nome_usos.py", USOS)])
}

fn symbol(graph: &v2::Graph, label: &str) -> String {
    let found: Vec<_> = graph.nodes.values().filter(|n| n.category == "symbol" && n.label == label).collect();
    assert_eq!(found.len(), 1, "{label}");
    found[0].id.clone()
}

/// The answer to the caller's one method call whose text starts so, with the targets by their labels.
fn answer(s: &Setup, caller: &str, text: &str) -> (String, Vec<String>) {
    let resolver = Resolver::new(&s.graph, &s.syntax);
    let owner = symbol(&s.graph, caller);
    let found: Vec<_> = s
        .graph
        .edges
        .iter()
        .filter(|e| e.kind == "observes" && e.from == owner && s.graph.nodes[&e.to].kind == "method_call_observed" && s.graph.nodes[&e.to].label.starts_with(text))
        .collect();
    assert_eq!(found.len(), 1, "{caller} {text}");
    let (how, ids) = match resolver.resolve(&owner, &found[0].to, None) {
        Resolution::Linked(h, t) => (h.as_str(), t),
        Resolution::Homonym(t) => ("homonym", t),
        Resolution::Overrides(t) => ("overrides", t),
        Resolution::AboveLimit(_) => ("above", vec![]),
        Resolution::NoCandidate | Resolution::OutOfChain => ("none", vec![]),
    };
    let mut labels: Vec<String> = ids.into_iter().map(|t| s.graph.nodes[t].label.clone()).collect();
    labels.sort();
    (how.to_owned(), labels)
}

fn check(s: &Setup, caller: &str, text: &str, how: &str, targets: &[&str]) {
    assert_eq!(answer(s, caller, text), (how.to_owned(), targets.iter().map(|t| t.to_string()).collect()), "{caller} {text}");
}

/// The rule does not settle it: the candidates stay, as many as before.
fn undecided(s: &Setup, caller: &str, text: &str, count: usize) {
    let (how, targets) = answer(s, caller, text);
    assert_eq!((how.as_str(), targets.len()), ("homonym", count), "{caller} {text}");
}

#[test]
fn the_name_reaches_the_class_as_the_scope_binds_it() {
    let s = real();
    for (caller, text, target) in [
        ("cn_c_importada", "CnBase.", "CnBase.somar_cn"),
        ("cn_c_mesmo_modulo", "CnLocal.", "CnLocal.somar_cn"),
        ("cn_c_alias_import", "Alias.", "CnBase.somar_cn"),
        ("cn_c_alias_modulo", "Apelido.", "CnFilha.somar_cn"),
        ("cn_c_alias_local", "k.", "CnFilha.somar_cn"),
        ("cn_c_reexportada", "Reexp.", "CnFilha.somar_cn"),
        ("cn_c_modulo", "classe_nome.", "CnBase.somar_cn"),
        ("cn_c_modulo_as", "modulo.", "CnFilha.somar_cn"),
        ("cn_c_modulo_pontos", "pkg.", "CnFilha.somar_cn"),
        ("cn_c_aninhada", "CnOutra.", "CnOutra.CnDentro.somar_cn"),
        ("cn_c_aninhada_modulo", "modulo.", "CnOutra.CnDentro.somar_cn"),
        ("CnLocal.CnConf.de_dentro_cn", "CnLocal.", "CnLocal.CnConf.somar_cn"),
        ("cn_c_import_local", "N.", "CnNeta.rotulo_cn"),
    ] {
        check(&s, caller, text, "type", &[target]);
    }
}

#[test]
fn it_goes_up_the_chain_and_never_down() {
    let s = real();
    check(&s, "cn_r_classmethod", "CnBase.", "type", &["CnBase.criar_cn"]);
    check(&s, "cn_r_staticmethod", "CnFilha.", "type", &["CnFilha.somar_cn"]);
    // With the object passed: subclasses that override it do not count.
    check(&s, "cn_r_instancia", "CnBase.", "type", &["CnBase.rotulo_cn"]);
    check(&s, "cn_r_instancia_filha", "CnFilha.", "type", &["CnFilha.rotulo_cn"]);
    check(&s, "CnFilha.rotulo_cn", "CnBase.", "type", &["CnBase.rotulo_cn"]);
    check(&s, "CnFilha.__init__", "CnBase.__init__", "type", &["CnBase.__init__"]);
    check(&s, "cn_r_init_herdado", "CnNeta.__init__", "type", &["CnFilha.__init__"]);
    // Inherited.
    check(&s, "cn_r_herdado", "CnNeta.", "type", &["CnBase.herdado_cn"]);
    check(&s, "cn_r_herdado_meio", "CnNeta.", "type", &["CnFilha.somar_cn"]);
    // A free function of the name is out by the form (DEC-38); one candidate is left, and the class settles it.
    check(&s, "cn_r_solta", "CnBalde.", "type", &["CnBalde.autorizar_cn"]);
    // A typed object still dispatches (DEC-44).
    check(&s, "cn_g_tipo", "x.", "overrides", &["CnBase.rotulo_cn", "CnFilha.rotulo_cn", "CnNeta.rotulo_cn"]);
    check(&s, "cn_g_objeto", "CnBase()", "type", &["CnBase.rotulo_cn"]);
}

#[test]
fn a_variable_or_two_possible_classes_are_not_the_class() {
    let s = real();
    // Its constructor, as before.
    check(&s, "cn_r_sombra_variavel", "CnBase.", "type", &["CnOutra.rotulo_cn"]);
    for caller in ["cn_r_sombra_parametro", "cn_r_sombra_for", "cn_r_sombra_compreensao"] {
        undecided(&s, caller, "CnBase.", 2);
    }
    undecided(&s, "cn_r_sombra_condicional", "Base2.", 2);
    undecided(&s, "cn_r_condicional", "Z.", 6);
    undecided(&s, "cn_g_type_self", "type(x).", 6);
}

#[test]
fn a_name_that_is_not_only_a_method_is_not_decided() {
    let s = real();
    undecided(&s, "cn_r_atributo", "CnComAtributo.", 2);
    undecided(&s, "cn_r_atributo_apelido", "CnComAtributo.", 2);
    undecided(&s, "cn_r_religada", "CnReligada.", 2);
    undecided(&s, "cn_r_metaclasse", "CnComMeta.", 2);
    undecided(&s, "cn_r_protocolo", "CnContrato.", 6);
    // A nested class the body binds again, or an inherited one.
    undecided(&s, "cn_c_aninhada_trocada", "CnTrocada.", 6);
    undecided(&s, "cn_c_aninhada_herdada", "CnHerdeiraDeOutra.", 6);
    // A private name is the class's own only inside it.
    check(&s, "CnPrivada.abrir_cn", "CnPrivada.", "type", &["CnPrivada.__cofre_cn"]);
    undecided(&s, "CnCofre2.abrir_cn", "CnPrivada.", 2);
}

#[test]
fn a_base_from_outside_stops_the_search_and_a_method_out_of_the_chain_is_never_the_target() {
    let s = real();
    check(&s, "cn_f_propria", "CnTarefaPropria.", "type", &["CnTarefaPropria.rodar_fora_cn"]);
    check(&s, "cn_f_depois", "CnMistoDepois.", "type", &["CnBase.rotulo_cn"]);
    // Before the project's: a limit, as in the DEC-43.
    check(&s, "cn_f_antes", "CnMisto.", "homonym", &["CnBase.rotulo_cn"]);
    for (caller, text) in [("cn_f_so_de_fora", "CnTarefa."), ("cn_f_classmethod_de_fora", "CnDic."), ("cn_r_ninguem", "CnSem."), ("cn_r_init_ninguem", "CnSem.__init__")] {
        check(&s, caller, text, "none", &[]);
    }
    // Only upwards: a subclass's method is not the class's.
    let s = setup(&[("m.py", "class A:\n    pass\n\n\nclass B(A):\n    @staticmethod\n    def m():\n        return 1\n\n\ndef usa():\n    return A.m()\n")]);
    check(&s, "usa", "A.m()", "none", &[]);
}

#[test]
fn the_return_of_the_method_types_what_it_binds() {
    let s = real();
    check(&s, "cn_r_retorno_herdado", "CnNeta.criar_cn", "type", &["CnBase.criar_cn"]);
    check(&s, "cn_r_retorno_herdado", "x.", "type", &["CnBase.liberar_cn"]);
}

#[test]
fn the_link_says_the_rule_also_above_the_limit() {
    let mut text = String::new();
    for i in 0..12 {
        text += &format!("class K{i}:\n    @staticmethod\n    def m():\n        return {i}\n\n\n");
    }
    text += "def usa():\n    return K3.m()\n";
    let s = setup(&[("m.py", &text)]);
    let seed = symbol(&s.graph, "usa");
    let calls = s.graph.calls_from(&seed, 1, 100, &s.syntax, &BTreeMap::new()).unwrap();
    let links: Vec<_> = calls.links.iter().filter(|l| l.from == seed).collect();
    assert_eq!(links.len(), 1);
    let l = links[0];
    assert_eq!(
        (l.to.as_str(), l.resolution.as_str(), l.basis.as_str(), l.relation.as_str(), l.type_rule.as_deref(), l.candidate_edge.as_deref()),
        (symbol(&s.graph, "K3.m").as_str(), "type", "type", "calls", Some("classe"), None)
    );
    assert!(calls.unresolved.is_empty());
}
