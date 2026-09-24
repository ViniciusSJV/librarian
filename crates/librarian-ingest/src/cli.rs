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
        Some("graph") if args.len() == 3 => {
            let catalog = Catalog::load(Path::new(&args[1]))?;
            let graph = crate::graph::Graph::build(&catalog);
            crate::write_new(Path::new(&args[2]), &serde_json::to_vec_pretty(&graph)?)?;
            println!("Graph: {} nodes, {} edges -> {}", graph.nodes.len(), graph.edges.len(), args[2]);
            Ok(if catalog.diagnostics().is_empty() { 0 } else { 2 })
        }
        Some("verify-graph") if args.len() == 3 => {
            let catalog = Catalog::load(Path::new(&args[1]))?;
            let graph: crate::graph::Graph = serde_json::from_slice(&fs::read(&args[2])?)?;
            graph.verify(&catalog)?;
            println!("Graph matches verified snapshots");
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
        _ => Err("Usage: generate <project-root> <config.json> <new-edition> | verify <edition> [current-project-root] | show <edition> <symbol-name> | search <edition> <question> [lexicon.json|-] [top] [depth] | graph <edition> <new-file.json> | verify-graph <edition> <graph.json>".into()),
    }
}
