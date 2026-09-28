//! Prepare a constrained query or check a preserved response, without network access.
use librarian_llm_engine::{claims, ollama::client::hash};
use std::{fs, io::Write};

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 || !matches!(args[1].as_str(), "prepare" | "check") {
        return Err("Usage: claims prepare CONTEXT.json QUESTION.txt NEW_QUERY.json | claims check CONTEXT.json RESPONSE.txt NEW_REPORT.json".into());
    }
    let context_bytes = fs::read(&args[2])?;
    let context: claims::Context = serde_json::from_slice(&context_bytes)?;
    let input = fs::read_to_string(&args[3])?;
    let (output, passed) = if args[1] == "prepare" {
        (claims::query(&context, &input)?, true)
    } else {
        let report = claims::check(&context, &input)?;
        let passed = report.mechanical_status == "passed";
        let output = serde_json::json!({
            "context_sha256": hash(&context_bytes),
            "response_sha256": hash(input.as_bytes()),
            "report": report,
            "limits": "Mechanical checks only. Evidence is supplied by the caller. Semantic review remains pending; accepted is always false."
        });
        (serde_json::to_string_pretty(&output)?, passed)
    };
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[4])?;
    file.write_all(output.as_bytes())?;
    file.sync_all()?;
    Ok(passed)
}

fn main() {
    std::process::exit(match run() {
        Ok(true) => 0,
        Ok(false) => 2,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    });
}
