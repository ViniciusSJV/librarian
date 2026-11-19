// Counts Python method calls whose receiver's first name is bound to an import in an outer scope
// and locally (parameter, assignment, for/with/except, comprehension) in an inner one: the local
// binding wins (step 2.2, Q2). Reads an edition; writes nothing.
use librarian_ingest::{graph::v2, resolve, Catalog};
use std::collections::BTreeMap;

fn main() {
    let edition = std::env::args().nth(1).expect("edition");
    let catalog = Catalog::load(std::path::Path::new(&edition)).unwrap();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    let facts = syntax.facts.clone().unwrap();
    let mut evidence = BTreeMap::new();
    let mut candidates: BTreeMap<&str, usize> = BTreeMap::new();
    for e in &graph.edges {
        if e.kind == "observes" {
            evidence.insert(e.to.as_str(), &e.evidence);
        }
        if e.kind == "name_candidate" {
            *candidates.entry(e.from.as_str()).or_default() += 1;
        }
    }
    let mut cache: BTreeMap<String, Option<resolve::CallFacts>> = BTreeMap::new();
    let (mut bound, mut shadowed, mut with_candidates) = (0, 0, 0);
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for (id, node) in &graph.nodes {
        if node.kind != "method_call_observed" || node.language != "python" {
            continue;
        }
        let Some(ev) = evidence.get(id.as_str()) else { continue };
        let Some(receiver) = resolve::written_receiver(&node.label) else { continue };
        let head = receiver.split(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or_default();
        let Some(f) = cache.entry(ev.path.clone()).or_insert_with(|| (facts.0)(&ev.path)).clone() else { continue };
        let at = ev.start_byte;
        let mut scopes: Vec<&resolve::NameScope> = f.scopes.iter().filter(|s| s.start <= at && at < s.end).collect();
        scopes.sort_by_key(|s| s.end - s.start);
        let chain: Vec<&&resolve::NameScope> = scopes.iter().enumerate().filter(|(i, s)| *i == 0 || !s.class).map(|(_, s)| s).collect();
        let decides = chain.iter().position(|s| s.locals.contains(head) || s.imports.contains_key(head));
        let Some(i) = decides else { continue };
        if chain[i].imports.contains_key(head) && !chain[i].locals.contains(head) {
            bound += 1;
            continue;
        }
        // Decided by a local binding: is there an import of the same name further out?
        let outer = chain[i..].iter().find_map(|s| s.imports.get(head));
        if let Some(import) = outer {
            shadowed += 1;
            let n = candidates.get(id.as_str()).copied().unwrap_or(0);
            if n > 0 {
                with_candidates += 1;
            }
            let same = if chain[i].imports.contains_key(head) { "mesmo escopo" } else { "escopo interno" };
            *kinds.entry(same.to_owned()).or_default() += 1;
            println!("{}:{}\t{}\t{:?}\tcandidatos={n}\t{same}", ev.path, ev.start_line, node.label.lines().next().unwrap_or_default(), import);
        }
    }
    println!("receptores ligados a import: {bound}; sombreados por ligacao local: {shadowed} (com candidatos: {with_candidates}); {kinds:?}");
}
