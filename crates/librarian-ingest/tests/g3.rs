//! G3 (Piloto 1b): "what does it call?" must not leave out, without saying so, the call
//! occurrences it knows but does not present as links. Absence from the answer is not absence
//! from the project. Each omission is reported in `unresolved` with why:
//! - `above_candidate_limit`: more same-named declarations than the profile links; the graph keeps
//!   only the count, and no candidate is chosen;
//! - `no_candidate_in_project`: no declaration of the project has the written name; this says
//!   nothing about the call being external;
//! - `node_limit`: a link the walk knows, left out by `max_nodes`; a presentation limit, not an
//!   ambiguity.
//!
//! The tests read `Calls` as the HTTP route serializes it, so they also fix the API shape.
use librarian_ingest::{generate, graph::v2, Catalog, Config};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Nine `notify` methods: one more than the profile's cap of 8 candidates.
fn channels() -> String {
    (0..9).map(|i| format!("class Channel{i}:\n    def notify(self, flip):\n        return {i}\n\n\n")).collect()
}

const FLOW: &str = "def helper(x):
    return x


def other(x):
    return x


def handler(chans, flip, handlers):
    for ch in chans:
        ch.notify(flip)
    missing_helper(flip)
    helper(flip)
    other(flip)
    handlers[0](flip)
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-g3-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("pkg/channels.py"), channels()).unwrap();
        fs::write(dir.join("pkg/flow.py"), FLOW).unwrap();
        Self(dir)
    }

    fn catalog(&self) -> Catalog {
        let path = self.0.join("edition");
        let config = Config { project: "g3".into(), inputs: vec!["pkg".into()] };
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

/// The call occurrence written as `label` inside `owner`.
fn occurrence(graph: &v2::Graph, owner: &str, label: &str) -> String {
    let found: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.kind == "observes" && e.from == owner)
        .map(|e| &graph.nodes[&e.to])
        .filter(|n| n.label == label && n.kind.ends_with("call_observed"))
        .collect();
    assert_eq!(found.len(), 1, "{label}: {found:?}");
    found[0].id.clone()
}

/// What the HTTP route sends for these calls.
fn sent(calls: &v2::Calls) -> Value {
    serde_json::to_value(calls).unwrap()
}

/// The entry of `unresolved` for this occurrence, or a failure saying what the answer has.
fn unresolved_entry(answer: &Value, occurrence: &str) -> Value {
    let Some(list) = answer.get("unresolved").and_then(Value::as_array) else {
        panic!("no `unresolved` in the answer; it has: {:?}", answer.as_object().unwrap().keys().collect::<Vec<_>>());
    };
    list.iter()
        .find(|u| u["occurrence"] == occurrence)
        .cloned()
        .unwrap_or_else(|| panic!("{occurrence} is not in `unresolved`: {list:?}"))
}

fn setup() -> (Project, v2::Graph, v2::CallSyntax, String) {
    let project = Project::new();
    let catalog = project.catalog();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    let handler = symbol(&graph, "handler");
    (project, graph, syntax, handler)
}

/// The links as (text, resolution, candidates), the shape existing consumers read.
fn links(graph: &v2::Graph, calls: &v2::Calls) -> Vec<(String, String, String, usize)> {
    calls.links.iter().map(|l| (l.text.clone(), graph.nodes[&l.to].label.clone(), l.resolution.clone(), l.candidates)).collect()
}

#[test]
fn g3_a_call_above_the_candidate_limit_is_reported_with_its_count_and_no_candidate() {
    let (_project, graph, syntax, handler) = setup();
    let notify = occurrence(&graph, &handler, "ch.notify(flip)");
    // The graph knows the ambiguity: a count and no candidate edge.
    assert_eq!(graph.nodes[&notify].ambiguous_candidates, Some(9));
    assert!(!graph.edges.iter().any(|e| e.from == notify && e.kind == "name_candidate"));

    let calls = graph.calls_from(&handler, 1, 50, &syntax, &BTreeMap::new()).unwrap();
    // No link is drawn to any of the nine, and none is chosen.
    assert!(calls.links.iter().all(|l| l.occurrence != notify));
    // Resolved calls stay as they were.
    let resolved = links(&graph, &calls);
    assert_eq!(resolved, [
        ("helper".to_owned(), "helper".to_owned(), "unique".to_owned(), 1),
        ("other".to_owned(), "other".to_owned(), "unique".to_owned(), 1),
    ]);
    let entry = unresolved_entry(&sent(&calls), &notify);
    assert_eq!(entry["reason"], "above_candidate_limit");
    assert_eq!(entry["candidates"], 9);
    assert_eq!(entry["text"], "ch.notify(flip)");
    assert_eq!(entry["from"], Value::String(handler.clone()));
    assert!(entry.get("to").is_none(), "no candidate is chosen: {entry}");
}

#[test]
fn g3_a_call_with_no_declaration_of_its_name_in_the_project_is_reported_as_such() {
    let (_project, graph, syntax, handler) = setup();
    let missing = occurrence(&graph, &handler, "missing_helper");
    // The occurrence exists in the graph, with no candidate edge and no count.
    assert_eq!(graph.nodes[&missing].ambiguous_candidates, None);
    assert!(!graph.edges.iter().any(|e| e.from == missing && e.kind == "name_candidate"));

    let calls = graph.calls_from(&handler, 1, 50, &syntax, &BTreeMap::new()).unwrap();
    let entry = unresolved_entry(&sent(&calls), &missing);
    assert_eq!(entry["reason"], "no_candidate_in_project");
    assert_eq!(entry["text"], "missing_helper");
    // Nothing beyond what the data shows: not "external", not "absent".
    let said = entry.to_string().to_lowercase();
    assert!(!said.contains("extern") && !said.contains("absent"), "{entry}");
}

#[test]
fn g3_a_link_left_out_by_the_node_limit_is_reported_apart_from_ambiguity() {
    let (_project, graph, syntax, handler) = setup();
    let all = graph.calls_from(&handler, 1, 50, &syntax, &BTreeMap::new()).unwrap();
    // The seed and one callee: the other unique callee no longer fits.
    let capped = graph.calls_from(&handler, 1, 2, &syntax, &BTreeMap::new()).unwrap();
    assert!(capped.truncated && capped.nodes.len() == 2);
    let kept: Vec<&str> = capped.links.iter().map(|l| l.occurrence.as_str()).collect();
    let left_out: Vec<&v2::CallLink> = all.links.iter().filter(|l| !kept.contains(&l.occurrence.as_str())).collect();
    assert_eq!(left_out.len(), 1, "one known link does not fit: {left_out:?}");
    let answer = sent(&capped);
    let entry = unresolved_entry(&answer, &left_out[0].occurrence);
    assert_eq!(entry["reason"], "node_limit");
    assert_eq!(entry["text"], Value::String(left_out[0].text.clone()));
    // The limit is not an ambiguity: the call's own resolution is kept.
    assert_eq!(entry["resolution"], "unique");
    assert_eq!(entry["candidates"], 1);
    // The ambiguous and the unmatched calls are still there with their own reasons.
    let reasons: Vec<&str> = answer["unresolved"].as_array().unwrap().iter().filter_map(|u| u["reason"].as_str()).collect();
    assert!(reasons.contains(&"above_candidate_limit") && reasons.contains(&"no_candidate_in_project"), "{reasons:?}");
}

/// A call with no written name (`handlers[0](flip)`) has nothing to compare with the project's
/// names. Decided in the Piloto 1b: it is `no_candidate_in_project` too (no declaration of the
/// project matches the occurrence), and its written text is what tells it apart from
/// `missing_helper(flip)`. The graph contract is not changed to say more.
#[test]
fn g3_a_call_with_no_written_name_is_not_left_out_silently() {
    let (_project, graph, syntax, handler) = setup();
    let indexed = occurrence(&graph, &handler, "handlers[0]");
    let calls = graph.calls_from(&handler, 1, 50, &syntax, &BTreeMap::new()).unwrap();
    let entry = unresolved_entry(&sent(&calls), &indexed);
    assert_eq!(entry["reason"], "no_candidate_in_project", "{entry}");
    assert_eq!(entry["text"], "handlers[0]");
    assert_eq!(entry["candidates"], 0);
}

/// Compatibility: the fields consumers read today keep their names and meaning.
#[test]
fn g3_the_existing_fields_of_calls_stay_as_they_are() {
    let (_project, graph, syntax, handler) = setup();
    let answer = sent(&graph.calls_from(&handler, 1, 50, &syntax, &BTreeMap::new()).unwrap());
    for field in ["seed", "depth", "max_nodes", "truncated", "nodes", "links"] {
        assert!(answer.get(field).is_some(), "{field}");
    }
    for field in ["from", "to", "occurrence", "occurrence_kind", "text", "candidates", "resolution", "candidate_edge", "evidence"] {
        assert!(answer["links"][0].get(field).is_some(), "links[].{field}");
    }
}
