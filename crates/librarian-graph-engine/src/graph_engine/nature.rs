//! Declared nature of a source, independent of language. The only place that knows the
//! v0.1 `kind` aliases; everything else decides by `Nature`.
use super::Source;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nature {
    /// Describes a procedure; never evidence that it ran.
    Code,
    /// Transcript of a run; may carry an `execution` record checked against its header.
    ExecutionRecord,
    Document,
    /// Neither `content` nor a legacy alias declares it.
    Undeclared,
}

/// v0.1 dossiers declared nature through these `kind` values, before `content` existed.
const LEGACY_KINDS: [(&str, Nature); 2] = [
    ("rust_source", Nature::Code),
    ("test_run", Nature::ExecutionRecord),
];

const LEGACY_METADATA_SCOPE: &str = "Os metadados exportados são declarações do dossiê, não verificações de origem ou execução. O campo legado executed é omitido para rust_source.";
const METADATA_SCOPE: &str = "Os metadados exportados são declarações do dossiê, não verificações de origem ou execução. O campo legado executed é omitido para fontes de código.";

/// Unknown `content` values and contradictions with a legacy `kind` are refused.
pub fn of(source: &Source) -> Result<Nature, String> {
    let legacy = LEGACY_KINDS
        .iter()
        .find(|(kind, _)| source.kind.as_deref() == Some(kind))
        .map(|(_, nature)| *nature);
    let Some(content) = source.content.as_deref() else {
        return Ok(legacy.unwrap_or(Nature::Undeclared));
    };
    let declared = match content {
        "code" => Nature::Code,
        "execution_record" => Nature::ExecutionRecord,
        "document" => Nature::Document,
        other => return Err(format!("{}: content desconhecido \"{other}\".", source.id)),
    };
    if legacy.is_some_and(|nature| nature != declared) {
        return Err(format!(
            "{}: content \"{content}\" contradiz o kind declarado.",
            source.id
        ));
    }
    Ok(declared)
}

/// v0.1 exports keep their exact text; sources declaring `content` use neutral wording.
pub fn metadata_scope(source: &Source) -> &'static str {
    if source.content.is_some() {
        METADATA_SCOPE
    } else {
        LEGACY_METADATA_SCOPE
    }
}
