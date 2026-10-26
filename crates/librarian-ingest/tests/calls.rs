//! P1b: "what does it call?" Forward from a symbol through written calls. A call is followed
//! only when its name points to one declaration (unique, the receiver's own call within the
//! same container, or a candidate the person chose); homonyms are reported, never followed.
use librarian_ingest::{generate, graph::v2, Catalog, Config};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const WORLD: &str = "pub struct World {
    pub n: f64,
}

impl World {
    pub fn color_at(&self, x: f64) -> f64 {
        let h = self.hit(x);
        self.shade(h)
    }
    pub fn hit(&self, x: f64) -> f64 {
        x * self.n
    }
    pub fn shade(&self, h: f64) -> f64 {
        lighting(h)
    }
}

pub struct Stripe;

impl Stripe {
    pub fn color_at(&self, x: f64) -> f64 {
        x
    }
    pub fn hit(&self, x: f64) -> f64 {
        x
    }
}

pub fn lighting(h: f64) -> f64 {
    h * 0.5
}

pub fn render(world: &World, x: f64) -> f64 {
    world.color_at(x)
}

#[test]
fn render_world_check() {
    fn render_world_helper() -> f64 {
        1.0
    }
    assert_eq!(render_world_helper(), 1.0);
}
";

const SHAPES: &str = "class World:
    def color_at(self, x):
        return self.hit(x)

    def hit(self, x):
        return x


class Stripe:
    def hit(self, x):
        return x
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-calls-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("src/world.rs"), WORLD).unwrap();
        fs::write(dir.join("pkg/shapes.py"), SHAPES).unwrap();
        Self(dir)
    }

    fn catalog(&self) -> Catalog {
        let path = self.0.join("edition");
        let config = Config { project: "calls".into(), inputs: vec!["src".into(), "pkg".into()] };
        generate(&self.0, &config, &path).unwrap();
        Catalog::load(&path).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn symbol(graph: &v2::Graph, label: &str) -> String {
    let found: Vec<_> = graph.nodes.values().filter(|n| n.category == "symbol" && n.label == label).collect();
    assert_eq!(found.len(), 1, "{label}");
    found[0].id.clone()
}

fn label(graph: &v2::Graph, id: &str) -> String {
    graph.nodes[id].label.clone()
}

#[test]
fn calls_are_followed_only_where_the_written_name_points_to_one_declaration() {
    let project = Project::new();
    let catalog = project.catalog();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    let render = symbol(&graph, "render");

    // world.color_at matches World::color_at and Stripe::color_at: both reported, neither followed.
    let calls = graph.calls_from(&render, 3, 50, &syntax, &BTreeMap::new()).unwrap();
    let mut first: Vec<_> = calls.links.iter().map(|l| (label(&graph, &l.to), l.resolution.as_str(), l.candidates)).collect();
    first.sort();
    assert_eq!(first, [("impl Stripe::color_at".to_owned(), "homonym", 2), ("impl World::color_at".to_owned(), "homonym", 2)]);
    assert_eq!(calls.nodes.len(), 3);

    // The person chooses World::color_at; from there self.hit is the receiver's own call: of its
    // two homonyms, only World::hit shares the caller's impl. self.shade and lighting are unique.
    let occurrence = calls.links[0].occurrence.clone();
    let chosen = BTreeMap::from([(occurrence, symbol(&graph, "impl World::color_at"))]);
    let calls = graph.calls_from(&render, 3, 50, &syntax, &chosen).unwrap();
    let path: Vec<_> = calls.links.iter().map(|l| (label(&graph, &l.from), label(&graph, &l.to), l.resolution.as_str())).collect();
    let step = |a: &str, b: &str, r: &'static str| (a.to_owned(), b.to_owned(), r);
    assert_eq!(path, [
        step("render", "impl World::color_at", "chosen"),
        step("impl World::color_at", "impl World::hit", "receiver"),
        step("impl World::color_at", "impl World::shade", "unique"),
        step("impl World::shade", "lighting", "unique"),
    ]);
    let layers: Vec<_> = calls.nodes.iter().map(|n| (n.label.clone(), n.layer)).collect();
    assert_eq!(layers.last().unwrap(), &("lighting".to_owned(), 3));
    // Every link is a candidate edge of the graph, with its own evidence.
    for link in &calls.links {
        let edge = graph.edges.iter().find(|e| e.id == link.candidate_edge).unwrap();
        assert_eq!((edge.kind.as_str(), &edge.from, &edge.to), ("name_candidate", &link.occurrence, &link.to));
    }
    // A depth limit stops the walk; the node cap truncates.
    assert_eq!(graph.calls_from(&render, 1, 50, &syntax, &chosen).unwrap().links.len(), 1);
    let capped = graph.calls_from(&render, 3, 2, &syntax, &chosen).unwrap();
    assert!(capped.truncated && capped.nodes.len() == 2);
    assert!(graph.calls_from(&render, 0, 50, &syntax, &chosen).is_err());

    // The same rule in Python, where the container is the class.
    let python = graph.calls_from(&symbol(&graph, "World.color_at"), 1, 50, &syntax, &BTreeMap::new()).unwrap();
    let found: Vec<_> = python.links.iter().map(|l| (label(&graph, &l.to), l.resolution.as_str(), l.candidates)).collect();
    assert_eq!(found, [("World.hit".to_owned(), "receiver", 2)]);
    // Without a receiver in the syntax (profiles before owner syntax), the call stays a homonym.
    let bare = v2::CallSyntax { languages: syntax.languages.iter().map(|(l, (k, _))| (l.clone(), (k.clone(), None))).collect() };
    let homonyms = graph.calls_from(&symbol(&graph, "World.color_at"), 1, 50, &bare, &BTreeMap::new()).unwrap();
    assert!(homonyms.links.iter().all(|l| l.resolution == "homonym") && homonyms.links.len() == 2);
}

#[test]
fn a_search_without_tests_leaves_out_test_functions_and_what_is_written_inside_them() {
    use librarian_ingest::search::{Lexicon, SearchIndex};
    let project = Project::new();
    let catalog = project.catalog();
    let names = |index: SearchIndex| -> Vec<String> {
        let mut found: Vec<_> = index.search("render world", 10, 0).unwrap().hits.into_iter().map(|h| h.name).collect();
        found.sort();
        found
    };
    let all = names(SearchIndex::new(&catalog, Lexicon::default()).unwrap());
    // Without the filter, a helper written inside the test function is a hit like any other.
    assert_eq!(all, ["render", "render_world_check::render_world_helper"]);
    let index = SearchIndex::new(&catalog, Lexicon::default()).unwrap().without_tests();
    let report = index.search("render world", 10, 0).unwrap();
    assert!(report.hits.iter().all(|h| !h.name.starts_with("render_world_check")));
    assert!(report.hits.iter().any(|h| h.name == "render"));
    assert!(report.limits.last().unwrap().starts_with("Test symbols"));
}
