//! The checks of a stored package, before anything is sent with it. H1: the mark, the bytes, the
//! package with itself and with the artifacts bound to it, the question it was made for. H2: its
//! units again from the edition's snapshots. H3: the package made again by this code from what was
//! prepared and recorded, byte for byte but for the code that built it. Nothing is rewritten; what
//! does not agree is refused as it is, with a message that starts with the level that refused it.
//!
//! What a check reads is what its keeper hands over: the artifacts as `TurnFiles`, the question as
//! `QuestionRecord`. A file that could not be read is handed over as such, and refused as the check
//! that needs it would have refused it.
use crate::{
    keeper::Keeper,
    lacuna,
    package::{self, Asked, BuildInput, ClaimsFiles, Criteria, Origin, Prepared, Suffix},
    selection,
};
use librarian_ingest::{
    envelope,
    evidence::{self, EvidencePackage, PackageFormat, Representation, RetryRule, SelectionInput, SelectionRecord, ServerAddition, Sha256Hex, SnapshotInput, SuffixInput},
    hash,
    prepare::{self, Request},
    Catalog, Result,
};
use serde_json::Value;

/// The record of the selection a question was asked with, as its keeper found it.
#[derive(Debug, Clone, Copy)]
pub enum SelectionFile<'a> {
    Absent,
    /// There, and not readable.
    Unreadable,
    Bytes(&'a [u8]),
}

/// The artifacts of one question, as its keeper read them; None is a file that could not be read.
#[derive(Debug, Clone, Copy)]
pub struct TurnFiles<'a> {
    /// The package's bytes, as kept.
    pub package: Option<&'a [u8]>,
    pub prepared: Prepared<'a>,
    /// The bundle's `origin.json`: names the edition the bundle was prepared over.
    pub bundle_origin: std::result::Result<&'a [u8], &'a str>,
    /// Read whatever the question's format: a prose question has none.
    pub claims: ClaimsFiles<'a>,
    pub criteria: Criteria<'a>,
    pub selection: SelectionFile<'a>,
}

/// What the keeper recorded of a question when it was made.
#[derive(Debug, Clone)]
pub struct QuestionRecord<'a> {
    /// The mark that names the question's package: `{ format_version, sha256 }`, or `{ error }` when
    /// none could be made.
    pub mark: &'a Value,
    /// The question prepared: the client's text and what the server added, joined.
    pub prepared: &'a str,
    /// The client's text and what the server added, apart; Err with why they could not be read.
    pub asked: std::result::Result<(String, Vec<Suffix>), String>,
    /// The hash of the criteria fixed with the question, if any.
    pub criteria_sha256: Option<&'a str>,
    /// Whether the question was asked in claims.
    pub claims: bool,
    /// The selection the client sent, when it reads as one.
    pub sent_selection: Option<SelectionInput>,
    /// What lacuna rounds added, as recorded; Err when the record cannot be read.
    pub additions: std::result::Result<Vec<ServerAddition>, ()>,
    /// The declarations gathered, the client's then the server's.
    pub nodes: Vec<String>,
}

fn refuse(level: &str, why: String) -> Box<dyn std::error::Error> {
    format!("{level}: {why}; nada foi enviado").into()
}

/// H1 and H2 of a question's package. The package, its hash and its bytes when both pass.
pub fn check_package(keeper: &dyn Keeper, catalog: std::result::Result<&Catalog, String>, files: &TurnFiles, question: &QuestionRecord)
    -> Result<(String, EvidencePackage, Vec<u8>)> {
    let mark = question.mark;
    if let Some(code) = mark.get("error") {
        return Err(format!("Esta pergunta não tem pacote de evidências ({}); nada foi enviado", code.as_str().unwrap_or("?")).into());
    }
    let fields = mark.as_object().ok_or_else(|| refuse("H1", "marca do pacote ilegível".into()))?;
    // Format 1 or 2: the mark names the format, and the package has to be in that same format.
    let format = mark["format_version"].as_u64().and_then(|v| u32::try_from(v).ok()).and_then(|v| PackageFormat::try_from(v).ok());
    let Some(format) = format.filter(|_| fields.len() == 2) else {
        return Err(refuse("H1", "marca do pacote em formato desconhecido".into()));
    };
    let sha256 = Sha256Hex::try_from(mark["sha256"].as_str().unwrap_or_default().to_owned()).map_err(|_| refuse("H1", "hash da marca inválido".into()))?;
    let bytes = files.package.ok_or_else(|| refuse("H1", "pacote ausente".into()))?.to_vec();
    if hash(&bytes) != sha256.as_str() {
        return Err(refuse("H1", "o pacote não é o que a marca nomeia".into()));
    }
    let package: EvidencePackage = serde_json::from_slice(&bytes).map_err(|e| refuse("H1", format!("pacote ilegível ({e})")))?;
    let mut again = serde_json::to_vec_pretty(&package)?;
    again.push(b'\n');
    if again != bytes {
        return Err(refuse("H1", "pacote fora da forma em que é gravado".into()));
    }
    if package.format_version != format {
        return Err(refuse("H1", "o formato da marca não é o do pacote".into()));
    }
    evidence::check_structure(&package).map_err(|e| refuse("H1", e.to_string()))?;
    // The versions it names are ones this code knows; the builder is the one recorded, whatever it is.
    if package.versions.rules.0 != envelope::RULES_VERSION {
        return Err(refuse("H1", format!("regras {} desconhecidas", package.versions.rules.0)));
    }
    // The artifacts of this question are the ones bound, and only those.
    let of = |bytes: Option<&[u8]>| bytes.map(hash);
    let claims = package.representation == Representation::Claims;
    let bound = &package.bound;
    let claims_context = of(files.claims.context);
    let criteria = of(files.criteria.bytes());
    let checks = [
        ("request", Some(bound.request_sha256.as_str().to_owned()) == of(files.prepared.request)),
        ("search", Some(bound.search_sha256.as_str().to_owned()) == of(files.prepared.search)),
        ("dossier", Some(bound.dossier_sha256.as_str().to_owned()) == of(files.prepared.dossier)),
        ("bundle_query", Some(bound.bundle_query_sha256.as_str().to_owned()) == of(files.prepared.bundle_query)),
        // A claims question has its context, a prose one has none: a null is no stand-in for it.
        ("claims_context", claims == bound.claims_context_sha256.is_some()
            && bound.claims_context_sha256.as_ref().map(|h| h.as_str().to_owned()) == if claims { claims_context } else { None }),
        ("criteria", bound.criteria_sha256.as_ref().map(|h| h.as_str().to_owned()) == criteria && question.criteria_sha256.map(str::to_owned) == criteria),
        ("query", Some(package.query.sha256.as_str().to_owned()) == of(if claims { files.claims.query } else { files.prepared.bundle_query })),
        ("representation", claims == question.claims),
    ];
    if let Some((name, _)) = checks.iter().find(|(_, same)| !same) {
        return Err(refuse("H1", format!("{name} difere do vinculado no pacote")));
    }
    // The question as asked, and the edition the package names.
    let asked = question.asked.clone()?;
    let suffixes: Vec<(String, RetryRule, usize)> = package.query.server_suffixes.iter().map(|s| (s.sha256.as_str().to_owned(), s.rule, s.attempt)).collect();
    let asked_suffixes: Vec<(String, RetryRule, usize)> = asked.1.iter().map(|s| (hash(s.text.as_bytes()), s.rule, s.attempt)).collect();
    let joined: String = std::iter::once(asked.0.as_str()).chain(asked.1.iter().map(|s| s.text.as_str())).collect();
    if package.query.prepared_question_sha256.as_str() != hash(question.prepared.as_bytes()) || joined != question.prepared
        || package.query.person_text.sha256.as_str() != hash(asked.0.as_bytes()) || suffixes != asked_suffixes {
        return Err(refuse("H1", "a pergunta registrada não é a do pacote".into()));
    }
    let origin: Value = serde_json::from_slice(files.bundle_origin?)?;
    let search: Value = serde_json::from_slice(files.prepared.search.ok_or("search.json ilegível")?)?;
    if origin["dossier"]["source_path"] != package.edition.origin.as_str() || search["catalog_sha256"] != package.edition.catalog_sha256.as_str() {
        return Err(refuse("H1", "a edition do pacote não é a do bundle".into()));
    }
    // The selection's record is the one the package names, and says what the question says it sent.
    let included: Option<Vec<String>> = match &package.selection {
        Some(reference) => {
            let SelectionFile::Bytes(bytes) = files.selection else { return Err(refuse("H1", "registro de seleção ausente".into())) };
            let record = SelectionRecord::from_bytes(bytes, reference.record_sha256.as_str()).map_err(|e| refuse("H1", format!("registro de seleção: {e}")))?;
            if record.suggestions() != reference.suggestions || record.selection_basis() != package.measure.selection_basis {
                return Err(refuse("H1", "o registro de seleção não é o que o pacote diz".into()));
            }
            if question.sent_selection.as_ref() != Some(&record.received) || record.edition_origin != package.edition.origin {
                return Err(refuse("H1", "a seleção da pergunta não é a do registro".into()));
            }
            Some(record.received.included.iter().map(|e| e.id.clone()).collect())
        }
        None if !matches!(files.selection, SelectionFile::Absent) => return Err(refuse("H1", "registro de seleção que o pacote não nomeia".into())),
        None => None,
    };
    // What the server added is what the question records, in its order.
    let recorded = question.additions.clone().map_err(|_| refuse("H1", "complementações ilegíveis".into()))?;
    if recorded != package.server_additions {
        return Err(refuse("H1", "as complementações não são as da pergunta".into()));
    }
    // H2: from the edition the question was prepared over, never from the analyzed project.
    let catalog = catalog.map_err(|e| refuse("H2", format!("edition indisponível ({e})")))?;
    // Each one asked for by its LACUNA and found for it in the edition, after the client's.
    lacuna::check_additions(keeper, catalog, &question.nodes, included.as_deref(), &package.server_additions).map_err(|e| refuse("H1", format!("complementação: {e}")))?;
    let mut snapshots: Vec<SnapshotInput> = vec![];
    for unit in &package.units {
        if snapshots.iter().any(|s| s.path == unit.path) {
            continue;
        }
        let (Some(source), Some(bytes)) = (catalog.source_by_path(&unit.path), catalog.source_bytes(&unit.path)) else {
            return Err(refuse("H2", format!("snapshot de {} indisponível", unit.path)));
        };
        snapshots.push(SnapshotInput { source_id: &source.id, path: &source.path, sha256: &source.sha256, bytes });
    }
    evidence::check_sources(&package, &snapshots).map_err(|e| refuse("H2", e.to_string()))?;
    Ok((sha256.as_str().to_owned(), package, bytes))
}

/// What H3 makes a package again from, besides the artifacts and the question.
pub struct Remake<'a> {
    pub catalog: &'a Catalog,
    pub files: &'a TurnFiles<'a>,
    pub question: &'a QuestionRecord<'a>,
    pub origin: &'a Origin,
    /// The instructions the query was written with, as the claims context records them.
    pub recorded_instructions: Option<&'a [String]>,
    /// Each reviewed fact's line in the acervo, in its exact bytes; Err when it cannot be read.
    pub fact_lines: std::result::Result<&'a [(String, Vec<u8>)], ()>,
}

/// H3: the package this code makes from what was prepared and recorded is the stored one, except
/// for the code that built it and the measure read when the question was made.
pub fn rederive(keeper: &dyn Keeper, input: &Remake, stored: &EvidencePackage, stored_bytes: &[u8]) -> Result<()> {
    let refuse = |why: String| refuse("H3", why);
    let request: Request = serde_json::from_slice(input.files.prepared.request.ok_or("prepared/request.json ilegível")?)?;
    let composition = prepare::prepare(input.catalog, &request).map_err(|e| refuse(format!("preparação não refeita ({e})")))?.composition;
    let (original, suffixes) = input.question.asked.clone()?;
    let suffixes: Vec<SuffixInput> = suffixes.iter().map(|s| SuffixInput { text: &s.text, rule: s.rule, attempt: s.attempt }).collect();
    let claims = stored.representation == Representation::Claims;
    // The selection's record made again by this code, from what the client sent, the suggestion
    // records of the run and the edition: checked as at the entry, against the client's nodes.
    let selection = match &stored.selection {
        Some(reference) => {
            let SelectionFile::Bytes(bytes) = input.files.selection else { return Err("registro de seleção ausente".into()) };
            let record = SelectionRecord::from_bytes(bytes, reference.record_sha256.as_str())?;
            let nodes: Vec<String> = record.received.included.iter().map(|e| e.id.clone()).collect();
            let again = selection::accept(keeper, input.catalog, &nodes, record.received.clone()).map_err(|e| refuse(format!("registro de seleção não refeito ({e})")))?;
            if again.bytes() != bytes {
                return Err(refuse("o código atual não refaz o registro de seleção".into()));
            }
            Some(again)
        }
        None => None,
    };
    let bytes = package::build(&BuildInput {
        catalog: input.catalog,
        composition: &composition,
        asked: Asked { original: &original, suffixes: &suffixes },
        origin: input.origin,
        prepared: input.files.prepared,
        claims: claims.then_some(input.files.claims),
        criteria: input.files.criteria,
        project_instructions: input.recorded_instructions,
        fact_lines: input.fact_lines,
        at_question: Ok(stored.measure.at_question.clone()),
        selection: selection.as_ref().map(|s| (s.sha256(), s.record())),
        format: stored.format_version,
    })
    .map_err(|code| refuse(format!("pacote não refeito ({code})")))?;
    // The builder is the one field allowed to differ: what it built is compared byte for byte.
    let mut again: EvidencePackage = serde_json::from_slice(&bytes)?;
    again.versions.builder = stored.versions.builder.clone();
    let mut again_bytes = serde_json::to_vec_pretty(&again)?;
    again_bytes.push(b'\n');
    if again_bytes == stored_bytes {
        return Ok(());
    }
    // Different bytes: the first part that differs names the refusal.
    let parts: [(&str, bool); 10] = [
        ("versions", again.versions == stored.versions),
        ("edition", again.edition == stored.edition),
        ("route", (again.route, again.origin_kind, again.representation) == (stored.route, stored.origin_kind, stored.representation)),
        ("fichas", again.fichas == stored.fichas),
        ("units", again.units == stored.units),
        ("query", again.query == stored.query),
        ("measure", again.measure == stored.measure),
        ("selection", again.selection == stored.selection),
        ("server_additions", again.server_additions == stored.server_additions),
        ("bound", again.bound == stored.bound),
    ];
    let part = parts.iter().find(|(_, same)| !same).map_or("a forma", |(part, _)| part);
    Err(refuse(format!("o código atual não refaz {part} do pacote")))
}
