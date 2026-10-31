//! Profile v7 (R1 of the renderer phase, step 7 for Rust): a closure run once per element of a
//! written iterator chain is a loop, and a method call's receiver gets its written type. The
//! inventory follows a call with homonyms by that type, to the default of a trait the type
//! implements, or to every implementation of one trait's method, and keeps one chain per call
//! that enters a declaration.
use librarian_ingest::{
    generate_with_profile,
    graph::v2,
    inventory::{self, Inventory, LoopFinding, RuleSet},
    Catalog, Config, EXTRACTOR_V6, EXTRACTOR_V7,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

// The shapes of the renderer, small: a trait default that inverts the transform, an enum that
// dispatches by match, a group of children in a Vec and the pixel loop as a closure.
const SCENE: &str = "pub struct Matrix {
    pub m: [f64; 16],
}

impl Matrix {
    pub fn inverse(&self) -> Matrix {
        let mut out = Matrix { m: [0.0; 16] };
        for i in 0..16 {
            out.m[i] = self.m[i].abs();
        }
        out
    }
}

pub trait Shape {
    fn local_hit(&self, t: f64) -> f64;
    fn transform(&self) -> Matrix;
    fn hit(&self, t: f64) -> f64 {
        let inverse = self.transform().inverse();
        self.local_hit(t + inverse.m[0])
    }
}

pub struct Ball {
    pub radius: f64,
}

pub struct Tri {
    pub a: f64,
}

impl Shape for Ball {
    fn local_hit(&self, t: f64) -> f64 {
        (t * self.radius).sqrt()
    }
    fn transform(&self) -> Matrix {
        Matrix { m: [1.0; 16] }
    }
}

impl Shape for Tri {
    fn local_hit(&self, t: f64) -> f64 {
        t.abs() + self.a
    }
    fn transform(&self) -> Matrix {
        Matrix { m: [1.0; 16] }
    }
}

pub enum Thing {
    Ball(Ball),
    Tri(Tri),
}

impl Shape for Thing {
    fn local_hit(&self, t: f64) -> f64 {
        match self {
            Thing::Ball(ref b) => b.local_hit(t),
            Thing::Tri(ref x) => x.local_hit(t),
        }
    }
    fn transform(&self) -> Matrix {
        Matrix { m: [1.0; 16] }
    }
}

pub struct Group {
    pub children: Vec<Thing>,
}

impl Group {
    pub fn hit(&self, t: f64) -> f64 {
        let mut best = 0.0;
        for child in &self.children {
            best += child.hit(t);
        }
        best
    }
}

pub fn render(group: Group, size: usize) -> f64 {
    let values = vec![1.0];
    let first = values.first().map(|v| v * 2.0);
    (0..size).for_each(|x| {
        group.hit(x as f64);
    });
    first.unwrap_or(0.0)
}
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-v7-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/scene.rs"), SCENE).unwrap();
        Self(dir)
    }

    fn edition(&self, name: &str, profile: &str) -> Catalog {
        let path = self.0.join(name);
        let config = Config { project: "v7".into(), inputs: vec!["src".into()] };
        generate_with_profile(profile, &self.0, &config, &path).unwrap();
        Catalog::load(&path).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Occurrences of a kind as `label -> candidate names`, sorted.
fn occurrences(graph: &v2::Graph, kind: &str) -> Vec<String> {
    let mut found: Vec<String> = graph
        .nodes
        .values()
        .filter(|n| n.kind == kind)
        .map(|n| {
            let mut names: Vec<&str> = graph.edges.iter().filter(|e| e.kind == "name_candidate" && e.from == n.id).map(|e| graph.nodes[&e.to].label.as_str()).collect();
            names.sort();
            format!("{} -> {}", n.label, names.join(","))
        })
        .collect();
    found.sort();
    found
}

fn find<'a>(inventory: &'a Inventory, head: &str) -> &'a LoopFinding {
    inventory.loops.iter().find(|l| l.head == head).unwrap_or_else(|| panic!("no loop {head}"))
}

/// A chain as `name (resolution) > ...`.
fn chain(steps: &[inventory::Step]) -> String {
    steps.iter().map(|s| format!("{} ({})", s.to_name, s.resolution)).collect::<Vec<_>>().join(" > ")
}

#[test]
fn closures_of_iterator_chains_are_loops_and_written_receiver_types_are_recorded() {
    let project = Project::new();
    let catalog = project.edition("v7", EXTRACTOR_V7);
    let graph = v2::Graph::build(&catalog);
    graph.verify(&catalog).unwrap();

    let loops: Vec<String> = occurrences(&graph, "loop").into_iter().map(|l| l.trim_end_matches(" -> ").to_owned()).collect();
    // `values.first().map(..)` maps an Option: not a loop.
    assert_eq!(loops, ["for child in &self.children", "for i in 0..16", "for_each(|x|"]);

    assert_eq!(
        occurrences(&graph, "receiver_type"),
        [
            // `b` bound by `Thing::Ball(ref b)`, `x` by `Thing::Tri(ref x)`.
            "b -> Ball",
            // The element of `children: Vec<Thing>`.
            "child -> Thing",
            "group -> Group",
            // `t: f64` is written, but f64 is declared nowhere in the project. `self.m[i]` is an
            // index and `self` in the trait's default method has no written type: no record.
            "t -> ",
            "x -> Tri",
        ]
    );
}

#[test]
fn the_inventory_follows_homonyms_by_written_type_trait_default_and_trait_implementations() {
    let project = Project::new();
    let catalog = project.edition("v7", EXTRACTOR_V7);
    let found = inventory::inventory(&catalog, &RuleSet::default()).unwrap();

    let pixels = find(&found, "for_each(|x|");
    assert!(pixels.unfollowed.is_empty(), "{:?}", pixels.unfollowed);
    // The pixel loop reaches the inversion through the group's children and the trait default.
    let to_inverse: Vec<String> = pixels.loops_through_calls.iter().filter(|c| c.head == "for i in 0..16").map(|c| chain(&c.steps)).collect();
    assert_eq!(to_inverse, ["impl Group::hit (type) > Shape::hit (trait_default) > impl Matrix::inverse (unique)"]);

    // The sqrt of Ball, by two calls that enter it: the trait's `self.local_hit` (every
    // implementation) and the match arm's `b.local_hit` (the variant's type).
    let sqrt = found.operations.iter().find(|o| o.text.contains("sqrt")).unwrap();
    let mut to_sqrt: Vec<String> = pixels.through_calls.iter().filter(|c| c.operation == sqrt.id).map(|c| chain(&c.steps)).collect();
    to_sqrt.sort();
    assert_eq!(
        to_sqrt,
        [
            "impl Group::hit (type) > Shape::hit (trait_default) > impl Shape for Ball::local_hit (trait)",
            "impl Group::hit (type) > Shape::hit (trait_default) > impl Shape for Thing::local_hit (trait) > impl Shape for Ball::local_hit (type)",
        ]
    );

    let children = find(&found, "for child in &self.children");
    assert_eq!(children.loops_through_calls.iter().map(|c| chain(&c.steps)).collect::<Vec<_>>(), ["Shape::hit (trait_default) > impl Matrix::inverse (unique)"]);
}

#[test]
fn the_v6_profile_has_neither_closure_loops_nor_receiver_types() {
    let project = Project::new();
    let catalog = project.edition("v6", EXTRACTOR_V6);
    assert_eq!(catalog.manifest().extractor, EXTRACTOR_V6);
    let graph = v2::Graph::build(&catalog);
    graph.verify(&catalog).unwrap();
    assert!(occurrences(&graph, "receiver_type").is_empty());
    assert!(!occurrences(&graph, "loop").iter().any(|l| l.starts_with("for_each")));
    // Without types, `child.hit` has two homonyms (the group's and the trait's): not followed,
    // so the children's loop holds nothing and is only counted.
    let found = inventory::inventory(&catalog, &RuleSet::default()).unwrap();
    assert!(!found.loops.iter().any(|l| l.head.starts_with("for_each") || l.head == "for child in &self.children"));
    assert_eq!(found.loops_without_operations, 1);
}
