//! The answer key of the selection: real changes that passed the gate, with the declarations each
//! one edited. Replays the selection over each change's edition, with no model, and compares it to
//! what was sent when the change was made. The data is local, in `.studio/gabarito` (editions and
//! measures of the runs), so this runs on request:
//! `cargo test -p librarian-ingest --test gabarito -- --ignored --nocapture`
use librarian_ingest::{
    envelope::{self, Measure},
    inventory::{self, RuleSet},
    Catalog,
};
use serde_json::Value;
use std::{fs, path::PathBuf};

fn ids(value: &Value) -> Vec<String> {
    serde_json::from_value(value.clone()).unwrap()
}

fn bytes(catalog: &Catalog, id: &str) -> usize {
    match catalog.symbols().get(id) {
        Some(symbol) => catalog.excerpt(&symbol.chunk_id).map_or(0, <[u8]>::len),
        None => 0,
    }
}

#[test]
#[ignore = "needs the local data in .studio/gabarito"]
fn the_selection_keeps_what_each_change_edited() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.studio/gabarito");
    let cases: Vec<Value> = serde_json::from_slice(&fs::read(root.join("casos.json")).unwrap()).unwrap();
    let mut missing = vec![];
    for case in &cases {
        let dir = root.join(case["id"].as_str().unwrap());
        let catalog = Catalog::load(&dir.join("edition")).unwrap();
        let measure: Option<Measure> = fs::read(dir.join("measure.json")).ok().map(|b| serde_json::from_slice(&b).unwrap());
        let seeds = ids(&case["seed_ids"]);
        let mut candidates = envelope::candidates(&catalog, &seeds, measure.as_ref()).unwrap();
        let found = inventory::inventory(&catalog, &RuleSet::default()).unwrap();
        envelope::trait_impls(&catalog, &found, &seeds, measure.as_ref(), &mut candidates);
        let mut selection = envelope::select(&catalog, &candidates, measure.as_ref(), envelope::BUDGET_BYTES, 16);
        selection.behavior = envelope::behavior(&catalog, &found, &selection);

        let selected: Vec<&str> = selection.items.iter().map(|e| e.item.id.as_str()).collect();
        let must = ids(&case["must_ids"]);
        let names = ids(&case["must"]);
        let lost: Vec<&String> = must.iter().zip(&names).filter(|(id, _)| !selected.contains(&id.as_str())).map(|(_, n)| n).collect();
        let recorded = ids(&case["recorded_ids"]);
        let recorded_bytes: usize = recorded.iter().map(|id| bytes(&catalog, id)).sum();
        println!("\n{}  sementes: {}", case["id"].as_str().unwrap(), ids(&case["seeds"]).join(", "));
        println!("  candidatas {}; selecionadas {} ({} bytes), gravado {} ({} bytes sem topos)", candidates.items.len(), selection.items.len(), selection.used_bytes, recorded.len(), recorded_bytes);
        println!("  editadas presentes {}/{}{}", must.len() - lost.len(), must.len(), if lost.is_empty() { String::new() } else { format!("; FALTAM: {lost:?}") });
        for entry in &selection.items {
            let edited = if must.contains(&entry.item.id) { "*" } else { " " };
            println!("  {edited} {:<10} {:>5}b {:>11} {}", entry.role, entry.bytes, entry.calls.map_or(String::new(), |c| c.to_string()), entry.item.name);
        }
        let mut out: Vec<String> = selection.left_out.iter().filter(|e| e.left_out.as_deref() != Some("test")).map(|e| format!("{} [{}]", e.item.name, e.left_out.as_deref().unwrap_or(""))).collect();
        out.sort();
        println!("  fora: {}; testes fora: {}", out.join(", "), selection.left_out.len() - out.len());
        let lost_from_recording: Vec<&String> = recorded.iter().filter(|id| !selected.contains(&id.as_str())).filter_map(|id| catalog.symbols().get(id).map(|s| &s.qualified_name)).collect();
        println!("  do gravado, saiu: {lost_from_recording:?}");
        for gap in &selection.gaps {
            println!("  lacuna {}: {} para {} {:?} {:?}", gap.code, gap.text.split_whitespace().collect::<Vec<_>>().join(" "), gap.for_target, gap.line, gap.candidates);
        }
        for b in &selection.behavior {
            println!("  comportamento {}: {} [{}] {}:{} {}", b.kind, b.name, b.detail, b.path, b.line, b.text.split_whitespace().collect::<Vec<_>>().join(" "));
        }
        missing.extend(lost.into_iter().map(|n| format!("{}: {n}", case["id"].as_str().unwrap())));
    }
    assert!(missing.is_empty(), "declarações editadas fora da seleção: {missing:#?}");
}
