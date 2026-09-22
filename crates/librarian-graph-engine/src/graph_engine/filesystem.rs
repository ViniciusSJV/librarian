//! File-backed evidence checks. Relative paths retain the CLI's current-directory semantics.
use super::{capture, metrics, CaptureLink, Source};
use sha2::{Digest, Sha256};
use std::fs;

// Resultado de uma conferência local. Não é uma garantia de imutabilidade.
pub struct CheckedCapture {
    record: serde_json::Value,
    report: String,
    output_path: std::path::PathBuf,
}

fn check_capture_document(link: &CaptureLink) -> Result<CheckedCapture, String> {
    let path = std::path::Path::new(&link.path);
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    metrics::add("record_link_read", bytes.len());
    if format!("{:x}", Sha256::digest(&bytes)) != link.sha256 {
        return Err("Hash do registro de captura diverge do dossiê".into());
    }
    let record: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if record["run_id"] != link.run_id || link.run_id.trim().is_empty() {
        return Err("ID de execução diverge da captura".into());
    }
    let output_path = path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("saida.bin");
    let report = capture::validate(path)?;
    Ok(CheckedCapture {
        record,
        report,
        output_path,
    })
}

fn check_source_capture_output(
    source_path: &str,
    source_hash: &str,
    checked: &CheckedCapture,
) -> Result<(), String> {
    if fs::canonicalize(source_path).map_err(|e| e.to_string())?
        != fs::canonicalize(&checked.output_path).map_err(|e| e.to_string())?
        || checked.record["output"]["sha256"] != source_hash
    {
        return Err("Fonte do dossiê não corresponde à saída da captura".into());
    }
    Ok(())
}

pub type CaptureCache =
    std::collections::HashMap<(std::path::PathBuf, String, String), CheckedCapture>;

fn capture_key(link: &CaptureLink) -> Result<(std::path::PathBuf, String, String), String> {
    // Não canonicalizar: a pasta declarada determina onde fica saida.bin.
    let path = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join(&link.path);
    Ok((path, link.sha256.clone(), link.run_id.clone()))
}

pub fn validate_capture_link(source: &Source) -> Result<Option<serde_json::Value>, String> {
    validate_capture_link_cached(source, &mut CaptureCache::new())
}

pub fn validate_capture_link_cached(
    source: &Source,
    documents: &mut CaptureCache,
) -> Result<Option<serde_json::Value>, String> {
    let Some(link) = &source.capture else {
        return Ok(None);
    };
    metrics::add("capture_link", 0);
    if source.kind.as_deref() != Some("test_run") || source.execution.is_some() {
        return Err(format!(
            "{}: capture exige test_run sem execution legado",
            source.id
        ));
    }
    validate_source_version(source)?;
    let source_path = source
        .path
        .as_ref()
        .ok_or("capture exige path e sha256 da saída")?;
    let source_hash = source
        .sha256
        .as_ref()
        .ok_or("capture exige sha256 da saída")?;
    let checked = match documents.entry(capture_key(link)?) {
        std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(check_capture_document(link)?)
        }
    };
    check_source_capture_output(source_path, source_hash, &checked)?;
    let record = &checked.record;
    let report = &checked.report;
    Ok(Some(serde_json::json!({
        "status": "capture_and_source_match",
        "record": link,
        "result": record["result"],
        "checked": ["record_sha256", "run_id", "source_output_path", "source_sha256_and_lines", "output_bytes_and_sha256", "source_comparisons_and_current_files"],
        "report": report,
        "scope": "Conferência local no momento da leitura, sem snapshot atômico. Não autentica execução, não comprova bytes compilados nem valida semanticamente a ficha. Datas e ambiente recebem apenas conferência básica. Fontes unavailable não têm arquivo atual conferido."
    })))
}

pub fn validate_source_content(source: &Source, bytes: &[u8]) -> Result<(), String> {
    let expected = match &source.sha256 {
        Some(hash) => hash,
        None => return Err(format!("{}: SHA-256 ausente.", source.id)),
    };
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != *expected {
        return Err(format!(
            "{}: o arquivo atual difere da edição registrada (SHA-256 diferente).",
            source.id
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("{}: conteúdo não é UTF-8: {}", source.id, error))?;
    if !text.lines().eq(source.lines.iter().map(String::as_str)) {
        return Err(format!(
            "{}: as linhas do dossiê diferem das linhas do arquivo.",
            source.id
        ));
    }
    Ok(())
}

// O formato atual separa o cabeçalho do restante por uma linha vazia.
// Não buscamos campos na saída do comando, que pode conter textos semelhantes.
pub fn validate_source_version(source: &Source) -> Result<(), String> {
    match (&source.path, &source.sha256) {
        (None, None) => Ok(()),
        (Some(path), Some(_)) => {
            let bytes = fs::read(path).map_err(|error| {
                format!("{}: não foi possível ler {}: {}", source.id, path, error)
            })?;
            metrics::add("source_content_read", bytes.len());
            validate_source_content(source, &bytes)
        }
        _ => Err(format!(
            "{}: path e sha256 devem ser informados juntos.",
            source.id
        )),
    }
}

/// Per-operation cache; never a persistent assertion that a source is unchanged.
#[derive(Default)]
pub struct SourceChecks {
    documents: CaptureCache,
}

impl SourceChecks {
    pub fn validate(&mut self, source: &Source) -> Result<Option<serde_json::Value>, String> {
        validate_source_version(source)?;
        validate_capture_link_cached(source, &mut self.documents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_capture_does_not_approve_an_unrelated_source() {
        let dir = std::env::temp_dir().join(format!("capture-output-link-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let output = dir.join("saida.bin");
        let other = dir.join("other.bin");
        fs::write(&output, b"same bytes").unwrap();
        fs::write(&other, b"same bytes").unwrap();
        let hash = format!("{:x}", Sha256::digest(b"same bytes"));
        let checked = CheckedCapture {
            record: serde_json::json!({"output": {"sha256": hash}}),
            report: String::new(),
            output_path: output.clone(),
        };
        check_source_capture_output(output.to_str().unwrap(), &hash, &checked).unwrap();
        assert!(check_source_capture_output(other.to_str().unwrap(), &hash, &checked).is_err());
        assert!(
            check_source_capture_output(output.to_str().unwrap(), &"0".repeat(64), &checked)
                .is_err()
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
