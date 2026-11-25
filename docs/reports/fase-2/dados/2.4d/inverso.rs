// Passo 2.4d, Parada 1 (simulador; nao e codigo de producao). Resolve uma vez TODA chamada de um run, com um Resolver
// so, e cronometra: carregar a edition, ler o grafo, montar o Resolver, resolver tudo, inverter.
//   inverso <run> [saida.json]
use librarian_ingest::{graph::v2, resolve::{Resolution, Resolver, INSTANTIATED}, Catalog};
use std::{collections::BTreeMap, time::Instant};
use serde_json::{json, Value};

fn main() {
    let run = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let ms = |t: Instant| t.elapsed().as_secs_f64() * 1000.;
    let t = Instant::now();
    let catalog = Catalog::load(&run.join("edition")).unwrap();
    let load = ms(t);
    let t = Instant::now();
    let graph: v2::Graph = serde_json::from_slice(&std::fs::read(run.join("graph-v2.json")).unwrap()).unwrap();
    let parse = ms(t);
    let t = Instant::now();
    let syntax = catalog.call_syntax();
    let resolver = Resolver::new(&graph, &syntax);
    let new = ms(t);
    let t = Instant::now();
    let mut out: Vec<Value> = vec![];
    let mut inverse: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut entries: BTreeMap<&str, Option<&str>> = BTreeMap::new();
    let mut n = 0usize;
    for e in graph.edges.iter().filter(|e| e.kind == "observes") {
        let Some((kinds, _)) = syntax.languages.get(&graph.nodes[&e.from].language) else { continue };
        let written = &graph.nodes[&e.to];
        if !kinds.contains(&written.kind.as_str()) { continue; }
        let (caller, occ) = (e.from.as_str(), e.to.as_str());
        let edges = resolver.admitted(caller, occ);
        let decorates = resolver.decorates(occ);
        let resolved = resolver.resolve(caller, occ, None);
        let chain = matches!(resolved, Resolution::OutOfChain);
        let (targets, how, basis): (Vec<&str>, &str, &str) = match resolved {
            Resolution::Linked(how, t) if t.len() == 1 => (t, how.as_str(), how.basis()),
            Resolution::NoCandidate | Resolution::OutOfChain => (vec![], "homonym", "graph_edge"),
            Resolution::Overrides(t) => (t, "homonym", "type"),
            _ => (edges.iter().map(|e| e.to.as_str()).collect(), "homonym", "graph_edge"),
        };
        let how = if targets.is_empty() { if written.ambiguous_candidates.is_some() && !chain { "above_candidate_limit" } else { "no_candidate_in_project" } } else { how };
        let one = targets.len() == 1 && how != "homonym";
        let inst = one && !decorates && INSTANTIATED.contains(&graph.nodes[targets[0]].kind.as_str());
        let runs = if inst { *entries.entry(targets[0]).or_insert_with(|| resolver.instance_entry(targets[0])) } else { None };
        for t in &targets { inverse.entry(t).or_default().push(n); }
        if let Some(r) = runs { inverse.entry(r).or_default().push(n); }
        let rule = if basis == "type" && !targets.is_empty() { resolver.type_rule(caller, occ) } else { None };
        out.push(json!({"occ": occ, "from": caller, "to": targets, "res": how, "basis": basis, "rel": if decorates {"decorates"} else if inst {"instantiates"} else {"calls"}, "runs": runs, "rule": rule,
            "adm": edges.iter().map(|e| e.to.as_str()).collect::<Vec<_>>()}));
        n += 1;
    }
    let all = ms(t);
    eprintln!("{}: chamadas {n}; carregar edition {load:.0} ms; ler grafo {parse:.0}; Resolver::new {new:.0}; resolver e inverter tudo {all:.0}; alvos {}", run.display(), inverse.len());
    if let Some(p) = std::env::args().nth(2) { std::fs::write(p, serde_json::to_vec(&out).unwrap()).unwrap(); }
}
