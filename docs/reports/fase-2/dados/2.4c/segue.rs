// Passo 2.4c, Parada 2: o `follow` do inventario (src/inventory.rs), copiado, sobre TODA chamada de um run.
// Vai em crates/librarian-ingest/examples/ de uma worktree descartavel.   segue <edition> > saida.tsv
use librarian_ingest::{graph::v2, resolve::{Resolution, Resolver, INSTANTIATED}, Catalog};

fn main() {
    let catalog = Catalog::load(std::path::Path::new(&std::env::args().nth(1).unwrap())).unwrap();
    let graph = v2::Graph::build(&catalog);
    let syntax = catalog.call_syntax();
    let resolver = Resolver::new(&graph, &syntax);
    for e in graph.edges.iter().filter(|e| e.kind == "observes") {
        let language = &graph.nodes[&e.from].language;
        let Some((kinds, _)) = syntax.languages.get(language) else { continue };
        if !kinds.contains(&graph.nodes[&e.to].kind.as_str()) {
            continue;
        }
        let answer = if resolver.decorates(&e.to) {
            "decorador, nao segue".to_owned()
        } else {
            match resolver.resolve(&e.from, &e.to, None) {
                Resolution::Linked(how, targets) => {
                    let mut to: Vec<String> = targets
                        .into_iter()
                        .filter_map(|t| if INSTANTIATED.contains(&graph.nodes[t].kind.as_str()) { resolver.instance_entry(t) } else { Some(t) })
                        .map(|t| graph.nodes[t].label.clone())
                        .collect();
                    to.sort();
                    format!("segue {} {}", how.as_str(), to.join(","))
                }
                Resolution::Homonym(c) | Resolution::Overrides(c) => format!("nao segue, {}", c.len()),
                Resolution::AboveLimit(_) | Resolution::NoCandidate | Resolution::OutOfChain => "nao segue, 0".to_owned(),
            }
        };
        let text = graph.nodes[&e.to].label.lines().next().unwrap_or_default().to_owned();
        println!("{}:{}\t{}-{}\t{}\t{}", e.evidence.path, e.evidence.start_line, e.evidence.start_byte, e.evidence.end_byte, text, answer);
    }
}
