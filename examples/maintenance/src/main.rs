//! Synthetic maintenance catalogue: no renderer, LLM or database dependency.
use bibliotecario_core::BibliotecarioFact;
use bibliotecario_graph_engine::{filesystem::SourceChecks, prepare_query, Evidence, Source};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn run(source_path: &Path, destination: &Path) -> Result<(), String> {
    let bytes = fs::read(source_path).map_err(|e| e.to_string())?;
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let source = Source {
        id: "MANUAL_V1".into(),
        kind: Some("maintenance_manual".into()),
        path: Some(source_path.to_string_lossy().into_owned()),
        sha256: Some(format!("{:x}", Sha256::digest(&bytes))),
        lines: text.lines().map(str::to_owned).collect(),
        capture: None,
        execution: None,
        executed: None,
        git_commit: None,
    };
    // Statements are supplied by this adapter, not automatically extracted or proved.
    let evidence = Evidence {
        id: Some("MAINTENANCE_DEMO_V1".into()),
        unknowns: Some(vec![
            "Exemplo fictício; não comprova manutenção executada.".into()
        ]),
        sources: vec![source],
        facts: vec![
            BibliotecarioFact::new(
                "F_INSPECTION",
                "O manual registra inspeção mensal.",
                "MANUAL_V1",
                2,
            ),
            BibliotecarioFact::new(
                "F_RECORD",
                "O manual pede registro da inspeção.",
                "MANUAL_V1",
                3,
            ),
        ],
    };
    let dossier = serde_json::to_string_pretty(&evidence).map_err(|e| e.to_string())?;
    let mut checks = SourceChecks::default();
    let prepared = prepare_query(
        &dossier,
        Some("O que o manual solicita? Cite os IDs e não afirme que a inspeção ocorreu."),
        &["F_RECORD", "F_INSPECTION"],
        1,
        |source| checks.validate(source),
    )?;
    prepared.write_bundle(
        destination,
        "memory:maintenance-adapter",
        "memory:maintenance-question",
    )?;
    println!("Bundle validado: {}", destination.display());
    Ok(())
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        eprintln!("Uso: maintenance-example MANUAL.txt DIRETORIO_NOVO");
        std::process::exit(2);
    }
    if let Err(error) = run(Path::new(&args[1]), Path::new(&args[2])) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
