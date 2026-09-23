//! File-backed dossier consumer. Run from the directory used by source paths.
use librarian_graph_engine::{filesystem::SourceChecks, prepare_query};
use std::{fs, path::Path};

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        return Err("Usage: export DOSSIER QUESTION NEW_DIRECTORY CONTEXT FACT_ID...".into());
    }
    let dossier = fs::read_to_string(&args[1]).map_err(|e| e.to_string())?;
    let question = fs::read_to_string(&args[2]).map_err(|e| e.to_string())?;
    let radius = args[4].parse::<usize>().map_err(|e| e.to_string())?;
    let ids: Vec<&str> = args[5..].iter().map(String::as_str).collect();
    let mut checks = SourceChecks::default();
    let prepared = prepare_query(&dossier, Some(&question), &ids, radius, |source| {
        // This consumer requires file-backed editions; the generic API also allows inline sources.
        if source.path.is_none() || source.sha256.is_none() {
            return Err(format!("{}: file path and hash required", source.id));
        }
        checks.validate(source)
    })?;
    prepared.write_bundle(Path::new(&args[3]), &args[1], &args[2])?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
