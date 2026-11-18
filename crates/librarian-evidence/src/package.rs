//! The making of a package: `build` takes everything a question's package says, as its keeper read
//! it, and gives the package's bytes or the stable code of what failed. Nothing half made comes out,
//! and nothing is read here: a file the keeper could not read is handed over as unreadable, and the
//! code it gives is the one the package would have given had it read the file itself.
use librarian_ingest::{
    envelope,
    evidence::{
        self, BoundArtifacts, ClaimsEvidence, Composition, CompositionInput, DeclaredBy, EditionIdentity, EvidencePackage, FactRecord, Form,
        MeasureAtQuestion, MeasureReferences, MeasureSelectionBasis, Normalization, OriginKind, PackageFormat, PackageKind, ProvenanceInput,
        QuestionInput, Representation, RetryRule, Route, RulesVersion, SelectionRecord, SelectionRecordFormat, SelectionReference, ServerAddition,
        ServerDeclared, Sha256Hex, SnapshotInput, SuffixInput, Versions,
    },
    hash, Catalog,
};
use serde_json::Value;

/// The files prepared for a question, as their keeper read them; None is a file that could not be
/// read. They are bound to the package by their hashes.
#[derive(Debug, Clone, Copy)]
pub struct Prepared<'a> {
    /// The preparation's request, as `prepare` wrote it.
    pub request: Option<&'a [u8]>,
    /// The search report: names the catalog.
    pub search: Option<&'a [u8]>,
    pub dossier: Option<&'a [u8]>,
    /// The query of the bundle, the prose one.
    pub bundle_query: Option<&'a [u8]>,
}

/// The files of a claims question, as their keeper read them.
#[derive(Debug, Clone, Copy)]
pub struct ClaimsFiles<'a> {
    pub context: Option<&'a [u8]>,
    pub query: Option<&'a [u8]>,
}

/// The prior criteria of a question: none, their bytes, or there and not readable.
#[derive(Debug, Clone, Copy)]
pub enum Criteria<'a> {
    Absent,
    Present(&'a [u8]),
    Unreadable,
}

impl<'a> Criteria<'a> {
    /// The bytes, when there are some to read.
    pub fn bytes(&self) -> Option<&'a [u8]> {
        match self {
            Criteria::Present(bytes) => Some(bytes),
            Criteria::Absent | Criteria::Unreadable => None,
        }
    }
}

/// Where the evidence of a question came from, as its keeper recorded it.
#[derive(Debug, Clone)]
pub struct Origin {
    pub route: Route,
    /// The finding a gathered question came from, when it names one this contract knows.
    pub kind: Option<OriginKind>,
    /// A gathered question whose origin sent a selection. With no record accepted for it, the
    /// package says only that none was tied to it.
    pub selection_sent: bool,
    /// What a lacuna round added, as the question records it; Err when the record cannot be read.
    pub additions: Result<Vec<ServerAddition>, ()>,
}

impl Origin {
    /// A question searched, with nothing gathered.
    pub fn search() -> Self {
        Self { route: Route::Search, kind: None, selection_sent: false, additions: Ok(vec![]) }
    }
}

/// What a server retry added to a client's question, with the rule that added it.
#[derive(Debug, Clone)]
pub struct Suffix {
    pub text: String,
    pub rule: RetryRule,
    pub attempt: usize,
}

/// The question as asked: the client's text and what the server added to it, in order.
#[derive(Debug, Clone, Copy)]
pub struct Asked<'a> {
    pub original: &'a str,
    pub suffixes: &'a [SuffixInput<'a>],
}

/// The measure there was when the question was made, as its keeper found it.
#[derive(Debug, Clone)]
pub enum MeasureRead {
    /// An execution read: its id, if the record names one, and the hash of its bytes.
    Read { execution: Option<String>, sha256: String },
    None,
    Unreadable,
}

/// What a package records of the measure there was when the question was made.
pub fn measure_at_question(read: MeasureRead) -> Result<MeasureAtQuestion, &'static str> {
    Ok(match read {
        MeasureRead::Read { execution, sha256 } => MeasureAtQuestion::Ok {
            execution: execution.ok_or("measure_unidentified")?,
            execution_sha256: Sha256Hex::try_from(sha256).map_err(|_| "measure_unidentified")?,
        },
        MeasureRead::None => MeasureAtQuestion::NoMeasure {},
        MeasureRead::Unreadable => MeasureAtQuestion::Unreadable {},
    })
}

/// Everything a package is made from.
pub struct BuildInput<'a> {
    /// The edition the question was prepared over; its snapshots are the sources of the units.
    pub catalog: &'a Catalog,
    /// What the preparation composed: fichas, reviewed facts and radius.
    pub composition: &'a Composition,
    pub asked: Asked<'a>,
    pub origin: &'a Origin,
    pub prepared: Prepared<'a>,
    /// Some for a claims question, whose query is the one sent.
    pub claims: Option<ClaimsFiles<'a>>,
    pub criteria: Criteria<'a>,
    /// The instructions of the project the query was written with.
    pub project_instructions: Option<&'a [String]>,
    /// Each reviewed fact's line in the acervo, in its exact bytes; Err when the acervo cannot be read.
    pub fact_lines: Result<&'a [(String, Vec<u8>)], ()>,
    /// The measure when the question was made: a new question's as read now, one made again as recorded.
    pub at_question: Result<MeasureAtQuestion, &'static str>,
    /// The selection accepted at the entry, by the hash of its record.
    pub selection: Option<(&'a Sha256Hex, &'a SelectionRecord)>,
    /// A new question's is the one written now; a package made again keeps the one it has.
    pub format: PackageFormat,
}

/// An edition's identity, as a prepared bundle's `origin.json` names it.
pub fn edition_origin(catalog: &Catalog) -> librarian_ingest::Result<String> {
    Ok(format!("edition:{}", hash(&serde_json::to_vec(catalog.manifest())?)))
}

/// The bytes of a question's package, or the stable code of what failed.
pub fn build(input: &BuildInput) -> Result<Vec<u8>, &'static str> {
    let catalog = input.catalog;
    // What a lacuna round added: those fichas are the server's.
    let additions = input.origin.additions.clone().map_err(|_| "server_additions_unreadable")?;
    let mut composition = input.composition.clone();
    for seed in &mut composition.fichas {
        if seed.symbol_id.as_ref().is_some_and(|id| additions.iter().any(|a| a.symbol_id == *id)) {
            seed.declared_by = DeclaredBy::ServerLacuna;
        }
    }
    let composition = &composition;
    fn read(bytes: Option<&[u8]>) -> Result<&[u8], &'static str> {
        bytes.ok_or("artifact_unreadable")
    }
    let request_bytes = read(input.prepared.request)?;
    let search = read(input.prepared.search)?;
    let dossier = read(input.prepared.dossier)?;
    let bundle_query = std::str::from_utf8(read(input.prepared.bundle_query)?).map_err(|_| "artifact_unreadable")?;
    let (context_bytes, claims_query) = match input.claims {
        Some(files) => (Some(read(files.context)?), Some(std::str::from_utf8(read(files.query)?).map_err(|_| "artifact_unreadable")?)),
        None => (None, None),
    };
    let criteria = match input.criteria {
        Criteria::Absent => None,
        Criteria::Present(bytes) => Some(bytes),
        Criteria::Unreadable => return Err("artifact_unreadable"),
    };
    // The sources the fichas and the quotes are in, as the edition has them.
    let mut paths: Vec<&str> = composition.fichas.iter().map(|f| f.path.as_str()).collect();
    paths.extend(composition.reviewed.iter().flat_map(|f| f.citations.iter().map(|c| c.path.as_str())));
    let mut snapshots: Vec<SnapshotInput> = vec![];
    for path in paths {
        if snapshots.iter().any(|s| s.path == path) {
            continue;
        }
        let source = catalog.source_by_path(path).ok_or("snapshot_unavailable")?;
        let bytes = catalog.source_bytes(path).ok_or("snapshot_unavailable")?;
        snapshots.push(SnapshotInput { source_id: &source.id, path: &source.path, sha256: &source.sha256, bytes });
    }
    // Claims: the context's evidence is what the form is checked against.
    let context: Option<Value> = context_bytes.map(serde_json::from_slice).transpose().map_err(|_| "claims_context_unreadable")?;
    let items: Vec<ClaimsEvidence> = match &context {
        Some(context) => context["evidence"].as_array().ok_or("claims_context_unreadable")?.iter().map(|e| Some(ClaimsEvidence {
            id: e["id"].as_str()?, path: e["path"].as_str()?, start_line: e["start_line"].as_u64()? as usize, end_line: e["end_line"].as_u64()? as usize, excerpt: e["excerpt"].as_str()?,
        })).collect::<Option<Vec<_>>>().ok_or("claims_context_unreadable")?,
        None => vec![],
    };
    let claims = input.claims.is_some();
    let form = if claims { Form::Claims(&items) } else { Form::Prose };
    let composition_input = CompositionInput { query_json: bundle_query, seeds: &composition.fichas, radius: composition.radius, form, snapshots: &snapshots };
    let composed = evidence::compose(&composition_input).map_err(|_| "composition_failed")?;
    // Each reviewed fact's line in the acervo, in its exact bytes, when it is there.
    let lines = input.fact_lines.map_err(|_| "acervo_unreadable")?;
    let records: Vec<FactRecord> = composition.reviewed.iter()
        .filter_map(|fact| lines.iter().find(|(id, _)| *id == fact.id).map(|(_, line)| FactRecord { fact_id: &fact.id, bytes: line }))
        .collect();
    let query_json = claims_query.unwrap_or(bundle_query);
    let query = evidence::build_provenance(&ProvenanceInput {
        query_json,
        question: QuestionInput { original: input.asked.original, suffixes: input.asked.suffixes },
        search,
        composition,
        composed: &composed,
        snapshots: &snapshots,
        fact_records: &records,
        project_instructions: input.project_instructions,
    }).map_err(|_| "provenance_failed")?;
    let at_question = input.at_question.clone()?;
    // A selection checked at the entry is named by its record, and rests on its last suggestion's
    // measure. A package made before selections were checked said only that none was tied to it.
    let (selection, selection_basis) = match input.selection {
        Some((sha256, record)) => (Some(SelectionReference { record_format: SelectionRecordFormat, record_sha256: sha256.clone(),
            suggestions: record.suggestions(), declared_by: ServerDeclared::Server }), record.selection_basis()),
        None => (None, match input.origin.selection_sent {
            true => MeasureSelectionBasis::NotRecorded {},
            false => MeasureSelectionBasis::NotApplicable {},
        }),
    };
    let manifest = catalog.manifest();
    let report: Value = serde_json::from_slice(search).map_err(|_| "artifact_unreadable")?;
    let sha = |bytes: &[u8]| Sha256Hex::try_from(hash(bytes)).map_err(|_| "artifact_unreadable");
    let package = EvidencePackage {
        kind: PackageKind::EvidencePackage,
        format_version: input.format,
        versions: Versions { normalization: Normalization::LinesV1, rules: RulesVersion(envelope::RULES_VERSION.to_owned()), builder: crate::identity::identity() },
        edition: EditionIdentity {
            origin: edition_origin(catalog).map_err(|_| "edition_unreadable")?,
            catalog_sha256: Sha256Hex::try_from(report["catalog_sha256"].as_str().unwrap_or_default().to_owned()).map_err(|_| "edition_unreadable")?,
            extractor: manifest.extractor.clone(),
            git_head: manifest.git.head.clone(),
            git_dirty: manifest.git.dirty,
        },
        route: input.origin.route,
        origin_kind: input.origin.kind,
        representation: if claims { Representation::Claims } else { Representation::Prose },
        fichas: composed.fichas,
        units: composed.units,
        query,
        // Format 1 wrote false and compared nothing; it is made again as it was. Format 2 compares.
        measure: match input.format {
            PackageFormat::V1 => MeasureReferences { at_question, selection_basis, measure_diverges: Some(false), measure_divergence: None },
            PackageFormat::V2 => {
                let divergence = evidence::measure_divergence(&at_question, &selection_basis);
                MeasureReferences { at_question, selection_basis, measure_diverges: None, measure_divergence: Some(divergence) }
            }
        },
        selection,
        server_additions: additions,
        bound: BoundArtifacts {
            request_sha256: sha(request_bytes)?,
            search_sha256: sha(search)?,
            dossier_sha256: sha(dossier)?,
            bundle_query_sha256: sha(bundle_query.as_bytes())?,
            claims_context_sha256: context_bytes.map(sha).transpose()?,
            criteria_sha256: criteria.map(sha).transpose()?,
        },
    };
    let mut bytes = serde_json::to_vec_pretty(&package).map_err(|_| "serialization_failed")?;
    bytes.push(b'\n');
    Ok(bytes)
}
