//! Small reusable command interface; exit code 2 reports diagnostics or current divergence.
use crate::{generate, Catalog, Config, Result};
use std::{fs, path::Path};

pub fn run(args: &[String]) -> Result<i32> {
    match args.first().map(String::as_str) {
        Some("search") if (3..=6).contains(&args.len()) => {
            let catalog = Catalog::load(Path::new(&args[1]))?;
            let lexicon = match args.get(3) {
                Some(path) if path != "-" => serde_json::from_slice(&fs::read(path)?)?,
                _ => crate::search::Lexicon::default(),
            };
            let top = args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(5);
            let depth = args.get(5).map(|s| s.parse()).transpose()?.unwrap_or(1);
            let report = crate::search::SearchIndex::new(&catalog, lexicon)?.search(&args[2], top, depth)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(if report.hits.is_empty() || !catalog.diagnostics().is_empty() { 2 } else { 0 })
        }
        Some("graph") if args.len() == 3 || args.len() == 4 => {
            let catalog = Catalog::load(Path::new(&args[1]))?;
            let (bytes, nodes, edges) = match args.get(3).map(String::as_str) {
                None | Some("v1") => {
                    let graph = crate::graph::Graph::build(&catalog);
                    (serde_json::to_vec_pretty(&graph)?, graph.nodes.len(), graph.edges.len())
                }
                Some("v2") => {
                    let graph = crate::graph::v2::Graph::build(&catalog);
                    (serde_json::to_vec_pretty(&graph)?, graph.nodes.len(), graph.edges.len())
                }
                Some(other) => return Err(format!("Unknown graph format: {other}").into()),
            };
            crate::write_new(Path::new(&args[2]), &bytes)?;
            println!("Graph: {nodes} nodes, {edges} edges -> {}", args[2]);
            Ok(if catalog.diagnostics().is_empty() { 0 } else { 2 })
        }
        Some("verify-graph") if args.len() == 3 => {
            let catalog = Catalog::load(Path::new(&args[1]))?;
            let bytes = fs::read(&args[2])?;
            // The file's own version selects the contract; unknown versions are refused.
            let version: serde_json::Value = serde_json::from_slice(&bytes)?;
            if version["version"] == crate::graph::v2::VERSION {
                serde_json::from_slice::<crate::graph::v2::Graph>(&bytes)?.verify(&catalog)?;
            } else {
                serde_json::from_slice::<crate::graph::Graph>(&bytes)?.verify(&catalog)?;
            }
            println!("Graph matches verified snapshots");
            Ok(if catalog.diagnostics().is_empty() { 0 } else { 2 })
        }
        Some("prepare") if args.len() == 4 => {
            let catalog = Catalog::load(Path::new(&args[1]))?;
            let prepared = crate::prepare::write(&catalog, &fs::read(&args[2])?, Path::new(&args[3]))?;
            // Reopen from disk; success describes a checked bundle, not just writes.
            crate::prepare::verify(&catalog, Path::new(&args[3]))?;
            println!("Prepared and verified: facts {:?} -> {}", prepared.query.selected_fact_ids(), args[3]);
            Ok(if catalog.diagnostics().is_empty() { 0 } else { 2 })
        }
        Some("verify-prepared") if args.len() == 3 => {
            let catalog = Catalog::load(Path::new(&args[1]))?;
            crate::prepare::verify(&catalog, Path::new(&args[2]))?;
            println!("Prepared bundle matches the verified edition");
            Ok(if catalog.diagnostics().is_empty() { 0 } else { 2 })
        }
        Some("generate") if args.len() == 4 => {
            let config: Config = serde_json::from_slice(&fs::read(&args[2])?)?;
            let manifest = generate(Path::new(&args[1]), &config, Path::new(&args[3]))?;
            // Reopen from disk; success describes a checked edition, not just writes.
            Catalog::load(Path::new(&args[3]))?;
            println!("Edition verified: {} sources, {} symbols, {} diagnostics -> {}",
                manifest.source_count, manifest.symbol_count, manifest.diagnostic_count, args[3]);
            Ok(if manifest.diagnostic_count == 0 { 0 } else { 2 })
        }
        Some("verify") if args.len() == 2 || args.len() == 3 => {
            let catalog = Catalog::load(Path::new(&args[1]))?;
            println!("Snapshots and records verified: {} sources, {} symbols, {} diagnostics",
                catalog.sources().len(), catalog.symbols().len(), catalog.diagnostics().len());
            let mut code = if catalog.diagnostics().is_empty() { 0 } else { 2 };
            if let Some(root) = args.get(2) {
                let changes = catalog.compare_current(Path::new(root))?;
                println!("{}", serde_json::to_string_pretty(&changes)?);
                if !changes.is_empty() { code = 2; }
            }
            Ok(code)
        }
        Some("investigate") if args.get(1).map(String::as_str) == Some("inventory") && (3..=5).contains(&args.len()) => {
            let catalog = Catalog::load(Path::new(&args[2]))?;
            let rules = match args.get(3) {
                Some(path) if path != "-" => serde_json::from_slice(&fs::read(path)?)?,
                _ => crate::inventory::RuleSet::default(),
            };
            let inventory = crate::inventory::inventory(&catalog, &rules)?;
            if let Some(file) = args.get(4) {
                crate::write_new(Path::new(file), &serde_json::to_vec_pretty(&inventory)?)?;
            }
            print!("{}", crate::inventory::summary(&inventory));
            Ok(if catalog.diagnostics().is_empty() { 0 } else { 2 })
        }
        Some("show") if args.len() == 3 => {
            let catalog = Catalog::load(Path::new(&args[1]))?;
            let symbols = catalog.symbols_named(&args[2]);
            if symbols.is_empty() { return Err(format!("Symbol not found: {}", args[2]).into()); }
            for symbol in symbols {
                let source = &catalog.sources()[&symbol.source_id];
                let chunk = catalog.chunk(&symbol.chunk_id).ok_or("Missing chunk")?;
                println!("{}:{}-{} [{}] {}\n{}\n", source.path, chunk.start_line, chunk.end_line,
                    symbol.kind, symbol.qualified_name,
                    std::str::from_utf8(catalog.excerpt(&symbol.chunk_id).ok_or("Missing excerpt")?)?);
            }
            Ok(0)
        }
        _ => Err("Usage: generate <project-root> <config.json> <new-edition> | verify <edition> [current-project-root] | show <edition> <symbol-name> | search <edition> <question> [lexicon.json|-] [top] [depth] | graph <edition> <new-file.json> [v1|v2] | verify-graph <edition> <graph.json> | prepare <edition> <request.json> <new-dir> | verify-prepared <edition> <dir> | investigate inventory <edition> [rules.json|-] [new-file.json]".into()),
    }
}
