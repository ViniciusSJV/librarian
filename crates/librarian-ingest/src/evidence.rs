//! The evidence package: the server's record of a prepared query of a new question. It says where
//! each part of that query came from, which fichas compose it and which hashes tie it to the
//! edition. These are the contract's types, the `lines-v1` rule, the construction of fichas and
//! units, the provenance of the query, the record of a suggestion, and the checks of a stored
//! package that do not need the code that made it.
//!
//! What a package is not: the send envelope (SYSTEM, skills and the other parts of a call are not
//! here), what a model read, the truth of any statement, the quality or sufficiency of a selection,
//! or proof of a human action. Hashes check consistency, not authenticity, and a hash of the query
//! says nothing about where its parts came from.
//!
//! Every field is required: an optional value is written as null, so absent and empty cannot be
//! mistaken for each other. The one exception is the field each package format has of its own in
//! `measure`, which the other format does not have at all. Nothing here varies between two builds of the same input by the clock.
use serde::{Deserialize, Serialize};

/// The package's own version, the one new packages are written in; it says nothing about the version
/// of a selection record. Packages of format 1 are still read, checked and made again as they were.
pub const PACKAGE_FORMAT_VERSION: u32 = 2;
/// The version of a selection record a new question may refer to. Records of earlier formats stay
/// as they were written and are never read as this one.
pub const SELECTION_RECORD_FORMAT: u32 = 2;

/// `kind`: names the artifact, so that its `format_version` is not taken for another artifact's.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageKind {
    EvidencePackage,
}

/// `format_version` of a package: 1, as packages were first written, with `measure_diverges`; or 2,
/// with `measure_divergence` in its place. No other number reads as one. The format a package was
/// written in is the one it is checked and made again in.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "u32", into = "u32")]
pub enum PackageFormat {
    V1,
    V2,
}

impl PackageFormat {
    /// The format of every package written now.
    pub const WRITTEN: PackageFormat = PackageFormat::V2;
}

impl TryFrom<u32> for PackageFormat {
    type Error = String;
    fn try_from(version: u32) -> Result<Self, String> {
        match version {
            1 => Ok(Self::V1),
            2 => Ok(Self::V2),
            _ => Err(format!("evidence package format {version} is not known")),
        }
    }
}

impl From<PackageFormat> for u32 {
    fn from(format: PackageFormat) -> u32 {
        match format {
            PackageFormat::V1 => 1,
            PackageFormat::V2 => 2,
        }
    }
}

/// `record_format` of a selection record a package refers to: 2, and no other number reads as one.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "u32", into = "u32")]
pub struct SelectionRecordFormat;

impl TryFrom<u32> for SelectionRecordFormat {
    type Error = String;
    fn try_from(version: u32) -> Result<Self, String> {
        if version == SELECTION_RECORD_FORMAT { Ok(Self) } else { Err(format!("selection record format {version} is not accepted by a package")) }
    }
}

impl From<SelectionRecordFormat> for u32 {
    fn from(_: SelectionRecordFormat) -> u32 {
        SELECTION_RECORD_FORMAT
    }
}

/// A SHA-256 as the crate's `hash` writes it: 64 lowercase hexadecimal digits. Well formed says
/// only that: which bytes it is the hash of is the job of whoever checks it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Hex(String);

impl Sha256Hex {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Sha256Hex {
    type Error = String;
    fn try_from(text: String) -> Result<Self, String> {
        if text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            Ok(Self(text))
        } else {
            Err(format!("not a lowercase SHA-256: {text:?}"))
        }
    }
}

impl From<Sha256Hex> for String {
    fn from(hash: Sha256Hex) -> String {
        hash.0
    }
}

/// A unit's id in its own query (`E1`, `CTX_1`). It is local: the same text in another query may
/// have another id, and it identifies nothing outside this package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct UnitId(pub String);

/// A ficha's reference in its own query (`E2`, `F1`). Local, like a unit's id.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct FichaRef(pub String);

/// How a unit's text comes from a snapshot's bytes. The version names the whole rule.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Normalization {
    #[serde(rename = "lines-v1")]
    LinesV1,
}

/// The version of the selection rules. On its own it does not bring back the rules it names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct RulesVersion(pub String);

/// The Librarian code that prepared the question, not the project it read. `Unknown` says that it
/// could not be told, and promises no way of finding that code again.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum BuilderIdentity {
    Recorded { git_commit: String, git_dirty: bool },
    Unknown {},
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Versions {
    pub normalization: Normalization,
    pub rules: RulesVersion,
    pub builder: BuilderIdentity,
}

/// The edition the question was prepared from. `git_head` and `git_dirty` are the analyzed
/// project's, as the edition recorded them, and may be unknown; they say nothing about the code in
/// `Versions::builder`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditionIdentity {
    pub origin: String,
    pub catalog_sha256: Sha256Hex,
    pub extractor: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub git_head: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub git_dirty: Option<bool>,
}

/// How the question reached the preparation: `gathered` also covers a question of the ladder.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Search,
    Gathered,
    Chain,
    Path,
}

/// What the client said a gathered question came from. A statement, not something observed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OriginKind {
    Selection,
    Ladder,
    Hot,
    Loop,
    Operation,
}

/// The form of the prepared query the units are read from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Representation {
    Claims,
    Prose,
}

/// Who says a piece of information is so. `Client` is a statement the server received, not an
/// action it saw; the other values are what the server itself recorded or added.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeclaredBy {
    Client,
    Server,
    ServerLacuna,
}

/// `declared_by` where only the client can be the author.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClientDeclared {
    Client,
}

/// `declared_by` of what a server retry added to a question.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerRetryDeclared {
    ServerRetry,
}

/// `declared_by` of what a lacuna round added to a question.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerLacunaDeclared {
    ServerLacuna,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FichaKind {
    Symbol,
    FileHead,
    ReviewedFact,
}

/// The selection's role for a ficha, as `envelope::Entry::role` names them. It says why the ficha
/// is there by a fixed rule, never how much it matters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FichaRole {
    Target,
    Origin,
    Cost,
    Path,
    Support,
    Constraint,
}

/// Why a ficha is in view, as `envelope::Reason::code` names it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    Chosen,
    OwnerType,
    FieldWriter,
    Called,
    CalledMeasured,
    ReturnType,
    CalledOwner,
    Caller,
    TestCaller,
    TypeReference,
    TraitImpl,
    Header,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FichaReason {
    pub code: ReasonCode,
    pub for_target: String,
    pub fields: Vec<String>,
    pub lines: Vec<usize>,
}

/// Where a ficha's reasons stand. With none recorded, it says so, and no reason is made up.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasonStatus {
    VerifiedAgainstSuggestion,
    ServerRecorded,
    NotRecorded,
}

/// A selected item. It points at exactly one unit; a unit may hold several fichas. It holds no
/// text and no hash of text: the query's content is what `Unit` describes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Ficha {
    #[serde(rename = "ref")]
    pub reference: FichaRef,
    pub kind: FichaKind,
    pub unit: UnitId,
    #[serde(deserialize_with = "Option::deserialize")]
    pub symbol_id: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub chunk_id: Option<String>,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(deserialize_with = "Option::deserialize")]
    pub role: Option<FichaRole>,
    pub reasons: Vec<FichaReason>,
    pub reason_status: ReasonStatus,
    pub declared_by: DeclaredBy,
}

/// Content the prepared query represents: one source, a range of lines and what it reads there.
/// `window_*` is the range before the claims form trims its last empty lines. No byte range: the
/// text sent is not a slice of the source's bytes, so one would promise an equality that is false.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub evidence_id: UnitId,
    pub source_id: String,
    /// The bytes of the source in the snapshot.
    pub source_sha256: Sha256Hex,
    pub path: String,
    pub window_start_line: usize,
    pub window_end_line: usize,
    pub start_line: usize,
    pub end_line: usize,
    /// The text the query always carries for this unit, line by line.
    pub text_sha256: Sha256Hex,
    /// The claims form's excerpt; the prose form has none.
    #[serde(deserialize_with = "Option::deserialize")]
    pub excerpt_sha256: Option<Sha256Hex>,
}

/// `provenance` of code in the query.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodeProvenance {
    EditionVerified,
}

/// The code class of the query: the units that carry it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodeClass {
    pub units: Vec<UnitId>,
    pub provenance: CodeProvenance,
}

/// `provenance` of text a client sent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClientTextProvenance {
    UnverifiedClientText,
}

/// The question as the client sent it. It is not evidence, and the server cannot say a person wrote it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PersonText {
    pub sha256: Sha256Hex,
    pub declared_by: ClientDeclared,
    pub provenance: ClientTextProvenance,
}

/// The rule that made a server retry add text to a question.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetryRule {
    GateRefused,
    MechanicalRefused,
}

/// Text a server retry added to the client's question, in the order it was added.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServerSuffix {
    pub order: usize,
    pub sha256: Sha256Hex,
    pub declared_by: ServerRetryDeclared,
    pub rule: RetryRule,
    pub attempt: usize,
}

/// What wrote a line of `evidence.limits`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LimitProducer {
    StepKind,
    OwnerStep,
    ProfileSearch,
    FichaNature,
    OwnerContext,
    Callees,
    ReviewedNote,
    HeaderNote,
    Diagnostics,
    RequestNotes,
    /// The line a search adds when it left out what the extractor marked as tests.
    ExcludeTests,
}

/// What a limit's producer read to write it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LimitInput {
    None,
    Route,
    Request,
    EditionProfile,
    Edition,
    SearchReport,
    Client,
}

/// What a value in a limit rests on, when it rests on something other than its input. No producer
/// writes a value of this kind today, and the field stays null until one does.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LimitBasis {
    Execution { execution: String, execution_sha256: Sha256Hex },
    Source { source_id: String, source_sha256: Sha256Hex },
}

/// Text in a limit is not evidence: it is the system's own text or a client's.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LimitClass {
    SystemText,
    ClientText,
}

/// Where a limit's text can be traced to. A limit is never vouched for by being in a query with a
/// good hash: what cannot be traced says `UnverifiedClientText` or `NotAvailable`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LimitProvenance {
    SystemFixed,
    SystemGenerated,
    UnverifiedClientText,
    NotAvailable,
}

/// One line of `evidence.limits`, by its place in the query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LimitEntry {
    pub index: usize,
    pub text_sha256: Sha256Hex,
    pub producer: LimitProducer,
    pub input: LimitInput,
    /// The version of the producer's rule, when it has its own; null when it is the builder's code.
    #[serde(deserialize_with = "Option::deserialize")]
    pub rule_version: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub basis: Option<LimitBasis>,
    pub class: LimitClass,
    pub provenance: LimitProvenance,
}

/// `checked_against` of a reviewed fact's quote: the edition the question was prepared from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuoteCheckedAgainst {
    Edition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CitationCheck {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub quote_sha256: Sha256Hex,
    pub checked_against: QuoteCheckedAgainst,
}

/// `provenance` of a reviewed fact: its quotes match the edition, and its record either came and
/// says the same or was not among the inputs. None of it says whether the statement is true.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewedFactProvenance {
    AcervoQuotesChecked,
    RecordNotAvailable,
}

/// A reviewed fact in `evidence.reviewed_facts`, tied to its record and to the quotes checked.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewedFactRef {
    pub fact_id: String,
    /// Of the fact's line in the acervo. Null when that line was not among the inputs.
    #[serde(deserialize_with = "Option::deserialize")]
    pub record_sha256: Option<Sha256Hex>,
    pub statement_sha256: Sha256Hex,
    pub ficha: FichaRef,
    pub unit: UnitId,
    pub citations: Vec<CitationCheck>,
    pub provenance: ReviewedFactProvenance,
}

/// The closed list of what a field of a prepared query can be.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryFieldClass {
    /// How the query is arranged: ids local to it, containers. No content of its own.
    Structure,
    /// Lines of a source, as a unit describes them.
    Code,
    /// What the edition recorded of a source or of where a declaration is.
    SourceMetadata,
    /// The server's description of a ficha. System text, not evidence of code or of a fact.
    FichaDescription,
    /// What the acervo says of a reviewed fact.
    ReviewedFact,
    /// A line of `evidence.limits`, with its origin in `QueryProvenance::limits`.
    Limit,
    /// The question as the client sent it.
    ClientText,
    /// The question as prepared: the client's text with what a retry added.
    ClientTextWithServerSuffixes,
    /// The same text in every query of one set of rules. Covered by `fixed_contract_sha256`.
    FixedContract,
    /// What the project tells the model. Covered by `project_instructions_sha256`.
    ProjectInstructions,
    /// A choice of the project: what the answer may do, not what it says.
    ProjectSettings,
    /// A value of the request that shaped the query.
    RequestSetting,
    /// Worked out from the edition by the form's own rule, which is not checked here.
    DerivedFromEdition,
    /// The name the query goes by.
    QueryIdentity,
}

/// Where a field's content comes from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldOrigin {
    Edition,
    Acervo,
    Server,
    Project,
    Request,
    Client,
    Contract,
}

/// What wrote a field. For a limit's line, the entry in `QueryProvenance::limits` says it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldProducer {
    GraphEngine,
    Preparation,
    ClaimsContext,
    ClaimsEngine,
    LimitEntries,
    Client,
}

/// How far a field's origin can be told at this layer. None of these says a field is true.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldProvenance {
    /// Compared with the edition's record or rebuilt from its snapshot.
    EditionVerified,
    /// The fact's record came and agrees, and its quotes are the edition's lines.
    AcervoQuotesChecked,
    /// The fact's record was not among the inputs.
    RecordNotAvailable,
    SystemFixed,
    SystemGenerated,
    /// Written in the request. Whoever wrote the request is not told by this.
    RequestRecorded,
    /// What the project declares. Its text is not vouched for.
    ProjectDeclared,
    UnverifiedClientText,
    NotAvailable,
}

/// One field of a query at one place: a field repeated by ficha or by unit has one entry for each.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClassifiedField {
    /// `$` and the names, with `[]` for the elements of a list.
    pub path: String,
    pub class: QueryFieldClass,
    pub origin: FieldOrigin,
    pub producer: FieldProducer,
    /// The field's rule, when it has its own version.
    #[serde(deserialize_with = "Option::deserialize")]
    pub rule_version: Option<String>,
    pub provenance: FieldProvenance,
    #[serde(deserialize_with = "Option::deserialize")]
    pub ficha: Option<FichaRef>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub unit: Option<UnitId>,
    /// Of the JSON list of the field's values at this place. Null in the fixed contract, whose hash
    /// covers it.
    #[serde(deserialize_with = "Option::deserialize")]
    pub value_sha256: Option<Sha256Hex>,
}

/// A place a field is at: its ficha, its unit and the hash of its values there, each null when it
/// has none. Written as a list of three, in that order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldPlace(pub Option<FichaRef>, pub Option<UnitId>, pub Option<Sha256Hex>);

/// Every place of one field that has one classification, which is written once for all of them.
/// The groups follow the order a field is first met in; the places, the order they are met in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FieldGroup {
    pub path: String,
    pub class: QueryFieldClass,
    pub origin: FieldOrigin,
    pub producer: FieldProducer,
    #[serde(deserialize_with = "Option::deserialize")]
    pub rule_version: Option<String>,
    pub provenance: FieldProvenance,
    pub places: Vec<FieldPlace>,
}

/// The groups of a classification: nothing of it is left out, only written once.
pub fn group_fields(fields: Vec<ClassifiedField>) -> Vec<FieldGroup> {
    let mut groups: Vec<FieldGroup> = vec![];
    for field in fields {
        let place = FieldPlace(field.ficha, field.unit, field.value_sha256);
        let same = |g: &&mut FieldGroup| g.path == field.path && g.class == field.class && g.origin == field.origin && g.producer == field.producer && g.rule_version == field.rule_version && g.provenance == field.provenance;
        match groups.iter_mut().find(|g| same(g)) {
            Some(group) => group.places.push(place),
            None => groups.push(FieldGroup { path: field.path, class: field.class, origin: field.origin, producer: field.producer, rule_version: field.rule_version, provenance: field.provenance, places: vec![place] }),
        }
    }
    groups
}

/// The classification the groups were made from, one field per place, group by group.
pub fn expand_fields(groups: &[FieldGroup]) -> Vec<ClassifiedField> {
    groups
        .iter()
        .flat_map(|g| g.places.iter().map(move |FieldPlace(ficha, unit, value_sha256)| ClassifiedField {
            path: g.path.clone(),
            class: g.class,
            origin: g.origin,
            producer: g.producer,
            rule_version: g.rule_version.clone(),
            provenance: g.provenance,
            ficha: ficha.clone(),
            unit: unit.clone(),
            value_sha256: value_sha256.clone(),
        }))
        .collect()
}

/// The classes of the prepared query, each with its own origin. `sha256` is the hash of the
/// prepared query: it shows these bytes were not changed, not where any part of them came from.
/// The send envelope is not described here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QueryProvenance {
    pub sha256: Sha256Hex,
    pub code: CodeClass,
    pub person_text: PersonText,
    pub server_suffixes: Vec<ServerSuffix>,
    /// The question as prepared: the client's text with any suffix a retry added.
    pub prepared_question_sha256: Sha256Hex,
    pub limits: Vec<LimitEntry>,
    pub reviewed_facts: Vec<ReviewedFactRef>,
    pub fixed_contract_sha256: Sha256Hex,
    #[serde(deserialize_with = "Option::deserialize")]
    pub project_instructions_sha256: Option<Sha256Hex>,
    /// Every field of the query, each in its class, grouped by classification. U6: no field is
    /// outside these groups.
    pub fields: Vec<FieldGroup>,
}

impl QueryProvenance {
    /// One entry per field and place, as the classification gives them.
    pub fn classified(&self) -> Vec<ClassifiedField> {
        expand_fields(&self.fields)
    }
}

/// The measure the server found when the question was recorded. It says nothing of what a person
/// was looking at, and "not applicable" has no place here: every question has this reading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "read_status", rename_all = "snake_case", deny_unknown_fields)]
pub enum MeasureAtQuestion {
    Ok { execution: String, execution_sha256: Sha256Hex },
    #[serde(rename = "none")]
    NoMeasure {},
    Unreadable {},
}

/// The measure the suggestion behind a selection rested on. Only a question with a selection has
/// one, so the others say `not_applicable`, which is not the same as `none`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "read_status", rename_all = "snake_case", deny_unknown_fields)]
pub enum MeasureSelectionBasis {
    Ok { suggestion: Sha256Hex, execution: String, execution_sha256: Sha256Hex },
    #[serde(rename = "none")]
    NoMeasure { suggestion: Sha256Hex },
    Unreadable { suggestion: Sha256Hex },
    NotApplicable {},
    /// The question came from a selection, and no reference to its suggestion has been tied to it
    /// yet: the measure it rested on cannot be named.
    NotRecorded {},
}

/// Two different readings, never one. The latest measure of the edition is not recorded: it
/// changes, and is worked out when someone asks. A divergence is told, and blocks nothing.
///
/// What the package says of the two depends on its format, and each format has only its own field:
/// format 1 has `measure_diverges`, which that format always wrote false without comparing anything,
/// so it proves nothing; format 2 has `measure_divergence`, worked out by `measure_divergence`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, try_from = "MeasureReferencesRead")]
pub struct MeasureReferences {
    pub at_question: MeasureAtQuestion,
    pub selection_basis: MeasureSelectionBasis,
    /// Format 1 only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measure_diverges: Option<bool>,
    /// Format 2 only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measure_divergence: Option<MeasureDivergence>,
}

/// `measure` as read: exactly one of the two fields, so that having neither is refused as any absent
/// field is. Which one the package's format calls for is checked with the package (H1).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeasureReferencesRead {
    at_question: MeasureAtQuestion,
    selection_basis: MeasureSelectionBasis,
    #[serde(default, deserialize_with = "present")]
    measure_diverges: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    measure_divergence: Option<MeasureDivergence>,
}

/// A field that, when written, has a value: null is not one of its values.
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(deserializer: D) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

impl TryFrom<MeasureReferencesRead> for MeasureReferences {
    type Error = &'static str;
    fn try_from(read: MeasureReferencesRead) -> Result<Self, &'static str> {
        if read.measure_diverges.is_some() == read.measure_divergence.is_some() {
            return Err("measure has measure_diverges (format 1) or measure_divergence (format 2), exactly one");
        }
        Ok(Self { at_question: read.at_question, selection_basis: read.selection_basis, measure_diverges: read.measure_diverges, measure_divergence: read.measure_divergence })
    }
}

/// Format 2: the measure the question recorded against the one its selection rested on. Equal,
/// different, impossible to compare and why, or nothing to compare because there is no selection:
/// four answers, never folded into one yes or no.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum MeasureDivergence {
    Same {},
    Diverges {},
    NotComparable { reasons: Vec<NotComparable> },
    NotApplicable {},
}

/// Which reading could not be compared, and why.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NotComparable {
    pub side: MeasureSide,
    pub read_status: Incomparable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeasureSide {
    AtQuestion,
    SelectionBasis,
}

/// A reading that names no measure to compare: the execution could not be read, or the selection's
/// record of it was never tied to the question.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Incomparable {
    Unreadable,
    NotRecorded,
}

/// The two readings compared, from what is recorded only. A measure is its `read_status`, its
/// execution and the hash of that execution's bytes; which suggestion named it is not part of it.
/// Without a selection there is nothing to compare. A reading that could not be read, or was not
/// recorded, cannot be compared, and each such side is named, the question's first. Otherwise two
/// measures are the same when both name the same execution with the same bytes, or both say there
/// was none; any other pair diverges.
pub fn measure_divergence(at_question: &MeasureAtQuestion, selection_basis: &MeasureSelectionBasis) -> MeasureDivergence {
    use MeasureAtQuestion as At;
    use MeasureSelectionBasis as Basis;
    if matches!(selection_basis, Basis::NotApplicable {}) {
        return MeasureDivergence::NotApplicable {};
    }
    let mut reasons = vec![];
    if matches!(at_question, At::Unreadable {}) {
        reasons.push(NotComparable { side: MeasureSide::AtQuestion, read_status: Incomparable::Unreadable });
    }
    match selection_basis {
        Basis::Unreadable { .. } => reasons.push(NotComparable { side: MeasureSide::SelectionBasis, read_status: Incomparable::Unreadable }),
        Basis::NotRecorded {} => reasons.push(NotComparable { side: MeasureSide::SelectionBasis, read_status: Incomparable::NotRecorded }),
        _ => {}
    }
    if !reasons.is_empty() {
        return MeasureDivergence::NotComparable { reasons };
    }
    match (at_question, selection_basis) {
        (At::Ok { execution: a, execution_sha256: s }, Basis::Ok { execution: b, execution_sha256: t, .. }) if a == b && s == t => MeasureDivergence::Same {},
        (At::NoMeasure {}, Basis::NoMeasure { .. }) => MeasureDivergence::Same {},
        _ => MeasureDivergence::Diverges {},
    }
}

/// `declared_by` of what only the server itself checked and wrote.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerDeclared {
    Server,
}

/// The selection record a question came with, and the suggestions it refers to, in the order the
/// client gave. The link is the server's: it checked the record before writing it. What the record
/// says a person did stays the client's statement inside it (`declared_by_client`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectionReference {
    pub record_format: SelectionRecordFormat,
    pub record_sha256: Sha256Hex,
    pub suggestions: Vec<Sha256Hex>,
    pub declared_by: ServerDeclared,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdditionReason {
    LacunaName,
}

/// The rule that decided a server addition.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdditionRule {
    #[serde(rename = "ladder::resolve")]
    LadderResolve,
}

/// A declaration a lacuna round added to what the client sent, kept apart from it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServerAddition {
    pub declared_by: ServerLacunaDeclared,
    pub round: usize,
    pub from_turn: String,
    pub from_send: String,
    pub symbol_id: String,
    pub for_name: String,
    pub reason: AdditionReason,
    pub decided_by_rule: AdditionRule,
}

/// The prepared files, by the hash of their bytes. `claims_context_sha256` is null in the prose
/// form, `criteria_sha256` when the question has no criteria.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoundArtifacts {
    pub request_sha256: Sha256Hex,
    pub search_sha256: Sha256Hex,
    pub dossier_sha256: Sha256Hex,
    pub bundle_query_sha256: Sha256Hex,
    #[serde(deserialize_with = "Option::deserialize")]
    pub claims_context_sha256: Option<Sha256Hex>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub criteria_sha256: Option<Sha256Hex>,
}

/// One package per question that reached the preparation. A question refused before that has none.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidencePackage {
    pub kind: PackageKind,
    pub format_version: PackageFormat,
    pub versions: Versions,
    pub edition: EditionIdentity,
    pub route: Route,
    #[serde(deserialize_with = "Option::deserialize")]
    pub origin_kind: Option<OriginKind>,
    pub representation: Representation,
    pub fichas: Vec<Ficha>,
    pub units: Vec<Unit>,
    pub query: QueryProvenance,
    pub measure: MeasureReferences,
    #[serde(deserialize_with = "Option::deserialize")]
    pub selection: Option<SelectionReference>,
    pub server_additions: Vec<ServerAddition>,
    pub bound: BoundArtifacts,
}

// `lines-v1`: how a unit's text comes from a snapshot's bytes. Pure: the bytes and the window go in,
// and nothing else is read. The prepared query is the reference for what these produce, and nothing
// here changes it. In claims the steps are all six; in prose, only N1 to N3, because the prose query
// carries the window's lines as they are.

/// Why a window has no normalized text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizationError {
    /// N1: the snapshot is not UTF-8. It is refused, never replaced: a replaced byte would be text
    /// the source does not have.
    InvalidUtf8 { valid_up_to: usize },
    /// N3: the window is not inside the snapshot's lines. It is never clamped to fit.
    WindowOutOfRange { start_line: usize, end_line: usize, lines: usize },
    /// N4: in claims, every line of the window is empty, so no line is left. What that means for a
    /// question is not decided here.
    EmptyAfterTrim { start_line: usize, end_line: usize },
}

impl std::fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUtf8 { valid_up_to } => write!(f, "snapshot is not UTF-8 after byte {valid_up_to}"),
            Self::WindowOutOfRange { start_line, end_line, lines } => write!(f, "window {start_line}-{end_line} is outside the {lines} lines of the snapshot"),
            Self::EmptyAfterTrim { start_line, end_line } => write!(f, "window {start_line}-{end_line} has only empty lines"),
        }
    }
}

impl std::error::Error for NormalizationError {}

/// A line of the text the query carries, with its number in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberedLine {
    pub number: usize,
    pub text: String,
}

/// A window after `lines-v1`. `window_*` is the range asked for; `start_line` and `end_line`, the
/// range left after N4. `excerpt` exists only in claims. `line_map` is what the query carries line
/// by line, and `text_sha256` is its hash; the two hashes differ when a line ended in two CRs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedWindow {
    pub window_start_line: usize,
    pub window_end_line: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub excerpt: Option<String>,
    pub excerpt_sha256: Option<Sha256Hex>,
    pub line_map: Vec<NumberedLine>,
    pub text_sha256: Sha256Hex,
}

fn sha256_of(bytes: &[u8]) -> Sha256Hex {
    Sha256Hex(crate::hash(bytes))
}

/// The hash of a source's bytes in the snapshot, as they are: not of any text made from them.
pub fn source_sha256(snapshot: &[u8]) -> Sha256Hex {
    sha256_of(snapshot)
}

/// N2: cut at each LF, taking away the one CR right before it, if there is one. Other CRs stay, and
/// so does a last line with no LF, whatever it ends in. After a last LF there is no empty line.
pub fn split_lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n')
        .map(|line| match line.strip_suffix('\n') {
            Some(body) => body.strip_suffix('\r').unwrap_or(body),
            None => line,
        })
        .collect()
}

/// N1 and N2: the snapshot's lines. A BOM stays as the first character of the first line, and
/// U+FFFD written in the source is a character like any other.
pub fn snapshot_lines(snapshot: &[u8]) -> Result<Vec<&str>, NormalizationError> {
    let text = std::str::from_utf8(snapshot).map_err(|error| NormalizationError::InvalidUtf8 { valid_up_to: error.valid_up_to() })?;
    Ok(split_lines(text))
}

/// N1 to N6 for the lines `start_line` to `end_line` of a snapshot, from 1 and inclusive.
pub fn normalize_window(snapshot: &[u8], start_line: usize, end_line: usize, representation: Representation) -> Result<NormalizedWindow, NormalizationError> {
    let lines = snapshot_lines(snapshot)?;
    if start_line == 0 || end_line < start_line || end_line > lines.len() {
        return Err(NormalizationError::WindowOutOfRange { start_line, end_line, lines: lines.len() });
    }
    let mut kept = &lines[start_line - 1..end_line];
    let (excerpt, text_lines): (Option<String>, Vec<String>) = match representation {
        Representation::Prose => (None, kept.iter().map(|line| (*line).to_owned()).collect()),
        Representation::Claims => {
            // N4: only lines with nothing at all in them, and only at the end.
            while kept.last().is_some_and(|line| line.is_empty()) {
                kept = &kept[..kept.len() - 1];
            }
            if kept.is_empty() {
                return Err(NormalizationError::EmptyAfterTrim { start_line, end_line });
            }
            // N5, then N6: the lines of the excerpt, cut again. That second cut is part of the rule.
            let excerpt = kept.join("\n");
            let text_lines = split_lines(&excerpt).into_iter().map(str::to_owned).collect();
            (Some(excerpt), text_lines)
        }
    };
    let text_sha256 = sha256_of(text_lines.join("\n").as_bytes());
    let excerpt_sha256 = excerpt.as_deref().map(|text| sha256_of(text.as_bytes()));
    let line_map = text_lines.into_iter().enumerate().map(|(offset, text)| NumberedLine { number: start_line + offset, text }).collect();
    Ok(NormalizedWindow {
        window_start_line: start_line,
        window_end_line: end_line,
        start_line,
        end_line: start_line + kept.len() - 1,
        excerpt,
        excerpt_sha256,
        line_map,
        text_sha256,
    })
}

// Fichas and units of one prepared query. Pure, like `lines-v1`: everything comes in as an argument
// and nothing is read from anywhere else. The query already says how it was composed (which window
// each ficha went into, and with what radius), so that is read from it, never worked out from text
// or from how close two ranges are. This is not a package: there is no version, no edition identity
// and no measure here, and what these functions return does not pretend to have them.

/// A line of `evidence.limits` with what wrote it and what that read. The texts, flattened in this
/// order, are the query's `evidence_unknowns`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedLimit {
    pub text: String,
    pub producer: LimitProducer,
    pub input: LimitInput,
    /// Null when the rule is the builder's own code.
    pub rule_version: Option<String>,
    pub provenance: LimitProvenance,
}

/// What the preparation knows of a selected item, before it is tied to a unit of the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FichaSeed {
    pub reference: FichaRef,
    pub kind: FichaKind,
    pub symbol_id: Option<String>,
    pub chunk_id: Option<String>,
    /// The edition's id of the source and the hash it recorded for it.
    pub source_id: String,
    pub source_sha256: String,
    pub source_kind: String,
    /// The commit the edition recorded for the project it read, if it did.
    pub git_commit: Option<String>,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub statement: String,
    pub authorship: String,
    pub reasons: Vec<FichaReason>,
    pub reason_status: ReasonStatus,
    pub declared_by: DeclaredBy,
}

/// What the preparation knows of how a query was composed, in the order the query has it.
#[derive(Debug, Clone)]
pub struct Composition {
    /// The request's radius, which is written to `request.json` as it came.
    pub radius: usize,
    pub fichas: Vec<FichaSeed>,
    pub limits: Vec<TaggedLimit>,
    /// The reviewed facts the request carried, with their quotes, which no query has.
    pub reviewed: Vec<crate::prepare::ReviewedFact>,
    /// The request's instructions: in the prose form, the project's text; absent, the contract's.
    pub instructions: Option<Vec<String>>,
}

/// A source's bytes in the edition's snapshot, with what the edition recorded about them.
#[derive(Debug, Clone, Copy)]
pub struct SnapshotInput<'a> {
    pub source_id: &'a str,
    pub path: &'a str,
    pub sha256: &'a str,
    pub bytes: &'a [u8],
}

/// A block of the claims form, as its context carries it. Its type belongs to another crate, so the
/// caller hands over these values.
#[derive(Debug, Clone, Copy)]
pub struct ClaimsEvidence<'a> {
    pub id: &'a str,
    pub path: &'a str,
    pub start_line: usize,
    pub end_line: usize,
    pub excerpt: &'a str,
}

/// The form the units are read from, with what U1 compares them to in that form.
#[derive(Debug, Clone, Copy)]
pub enum Form<'a> {
    /// The blocks of the prepared query.
    Prose,
    /// The evidence of the claims context.
    Claims(&'a [ClaimsEvidence<'a>]),
}

impl Form<'_> {
    pub fn representation(&self) -> Representation {
        match self {
            Self::Prose => Representation::Prose,
            Self::Claims(_) => Representation::Claims,
        }
    }
}

/// Everything a composition depends on.
#[derive(Debug, Clone, Copy)]
pub struct CompositionInput<'a> {
    /// The prepared query, in the bytes the bundle has.
    pub query_json: &'a str,
    pub seeds: &'a [FichaSeed],
    pub radius: usize,
    pub form: Form<'a>,
    pub snapshots: &'a [SnapshotInput<'a>],
}

/// The fichas and units of a query, tied to each other and checked. Not a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedEvidence {
    pub representation: Representation,
    pub fichas: Vec<Ficha>,
    pub units: Vec<Unit>,
}

/// An invariant that does not hold, and where. Each value names the only thing it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// U1: a unit with nothing of its id in the query or in the claims context.
    UnitNotInForm { unit: String },
    /// U1: something in the query or in the claims context with no unit.
    FormItemWithoutUnit { item: String },
    /// U1: a unit whose range or text is not what the query or the claims context carries.
    UnitDiffersFromForm { unit: String },
    /// U1: two units with one id.
    DuplicateUnit { unit: String },
    /// U2: the snapshot's bytes are not the ones the edition and the query recorded.
    SourceHashDiffers { unit: String },
    /// U2: line numbers that cannot be, for this unit in this form.
    LineNumbers { unit: String },
    /// U2: an excerpt hash where the form has none, or none where it has one.
    ExcerptHashForm { unit: String },
    /// U3: the text rebuilt from the snapshot is not the unit's.
    TextDiffers { unit: String },
    /// U4: the query ties this ficha to no unit.
    FichaWithoutUnit { ficha: String },
    /// U4: the unit this ficha names does not exist.
    FichaUnitUnknown { ficha: String, unit: String },
    /// U4: a unit no ficha points to.
    UnitWithoutFicha { unit: String },
    /// U4: two fichas with one reference.
    DuplicateFicha { ficha: String },
    /// U4: what the preparation knows of a ficha is not what the query says of it.
    SeedDiffersFromQuery { ficha: String },
    /// U4: a ficha and its unit in different sources, or in sources with different hashes.
    FichaOtherSource { ficha: String, unit: String },
    /// U4: a ficha's lines go past its unit's window.
    FichaOutsideWindow { ficha: String, unit: String },
    /// U5: the query's radius for this ficha, or the window it gives, is not the input's.
    RadiusDiffers { ficha: String },
    /// U5: the union of the fichas, widened by the radius, is not the unit's window.
    CoverageDiffers { unit: String, expected: (usize, usize), found: Vec<(usize, usize)> },
    /// U6: a field of the query with no row in the contract.
    UnknownField { path: String },
    /// U6: a field the contract requires is not in the query.
    MissingField { path: String },
    /// U6: a field of the fixed contract with two values in one query.
    FixedFieldVaries { path: String },
    /// A field of the query is not what its origin says.
    FieldDiffers { item: String, field: String },
    /// A line of the query's limits with no line of the preparation behind it.
    LimitWithoutOrigin { index: usize },
    /// A line of the query's limits with another text, or one missing.
    LimitTextDiffers { index: usize },
    /// The prepared question is not the client's text with what the retries added.
    QuestionDiffers,
    /// A fact's record does not say what the query carries.
    FactRecordDiffers { fact: String, field: String },
    /// A quote of a reviewed fact is not the edition's lines.
    QuoteDiffers { fact: String, path: String },
    /// The query's project instructions are not the ones the project declares.
    ProjectInstructionsDiffer,
    /// A recorded provenance is not the one the inputs give.
    ProvenanceDiffers { part: String },
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Why a query has no fichas and units. No partial result exists: a window that cannot be a unit
/// fails the whole construction, and nothing is left out in silence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionError {
    /// The query does not have the shape the preparation writes.
    MalformedQuery(String),
    /// A source of the query with no snapshot among the inputs.
    MissingSnapshot { path: String },
    /// A window of the query has no normalized text.
    Normalization { unit: String, path: String, error: NormalizationError },
    Violation(Violation),
}

impl std::fmt::Display for CompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedQuery(detail) => write!(f, "malformed query: {detail}"),
            Self::MissingSnapshot { path } => write!(f, "{path}: no snapshot"),
            Self::Normalization { unit, path, error } => write!(f, "{unit} ({path}): {error}"),
            Self::Violation(violation) => write!(f, "{violation}"),
        }
    }
}

impl std::error::Error for CompositionError {}

impl From<Violation> for CompositionError {
    fn from(violation: Violation) -> Self {
        Self::Violation(violation)
    }
}

/// A selection of the query, as it says it was composed.
struct QueryLink {
    reference: String,
    path: String,
    line: usize,
    end_line: usize,
    unit: Option<String>,
    requested_radius: Option<usize>,
    window_start: usize,
    window_end: usize,
}

/// A window of the query, with the lines it carries.
struct QueryBlock {
    id: String,
    path: String,
    sha256: String,
    start_line: usize,
    end_line: usize,
    lines: Vec<NumberedLine>,
}

struct QueryView {
    links: Vec<QueryLink>,
    blocks: Vec<QueryBlock>,
}

fn malformed(detail: impl Into<String>) -> CompositionError {
    CompositionError::MalformedQuery(detail.into())
}

fn text_at<'a>(value: &'a serde_json::Value, name: &str) -> Result<&'a str, CompositionError> {
    value[name].as_str().ok_or_else(|| malformed(format!("no text at {name}")))
}

fn number_at(value: &serde_json::Value, name: &str) -> Result<usize, CompositionError> {
    value[name].as_u64().map(|n| n as usize).ok_or_else(|| malformed(format!("no number at {name}")))
}

fn read_link(selection: &serde_json::Value, unit: Option<String>) -> Result<QueryLink, CompositionError> {
    let source = &selection["source"];
    let context = &source["context"];
    Ok(QueryLink {
        reference: text_at(selection, "fact_id")?.to_owned(),
        path: text_at(source, "path")?.to_owned(),
        line: number_at(source, "line")?,
        end_line: number_at(source, "end_line")?,
        unit,
        requested_radius: context["requested_radius"].as_u64().map(|n| n as usize),
        window_start: number_at(context, "start_line")?,
        window_end: number_at(context, "end_line")?,
    })
}

fn read_block(id: String, source: &serde_json::Value, block: &serde_json::Value) -> Result<QueryBlock, CompositionError> {
    let start_line = number_at(block, "start_line")?;
    let lines = block["lines"].as_array().ok_or_else(|| malformed(format!("{id}: no lines")))?;
    let lines = lines
        .iter()
        .enumerate()
        .map(|(offset, line)| {
            let text = line.get("text").unwrap_or(line).as_str().ok_or_else(|| malformed(format!("{id}: a line is not text")))?;
            let number = line.get("number").and_then(serde_json::Value::as_u64).map_or(start_line + offset, |n| n as usize);
            Ok(NumberedLine { number, text: text.to_owned() })
        })
        .collect::<Result<Vec<_>, CompositionError>>()?;
    Ok(QueryBlock {
        path: text_at(source, "path")?.to_owned(),
        sha256: text_at(source, "sha256")?.to_owned(),
        start_line,
        end_line: number_at(block, "end_line")?,
        lines,
        id,
    })
}

/// One selection keeps its window inline, named after its ficha; several share numbered blocks, and
/// each selection names the block it went into.
fn read_query(query_json: &str) -> Result<QueryView, CompositionError> {
    let query: serde_json::Value = serde_json::from_str(query_json).map_err(|error| malformed(error.to_string()))?;
    let evidence = &query["evidence"];
    let Some(contexts) = evidence.get("contexts") else {
        let id = text_at(evidence, "fact_id")?.to_owned();
        let source = &evidence["source"];
        return Ok(QueryView { links: vec![read_link(evidence, Some(id.clone()))?], blocks: vec![read_block(id, source, &source["context"])?] });
    };
    let selections = evidence["selections"].as_array().ok_or_else(|| malformed("no selections"))?;
    let mut links = vec![];
    for selection in selections {
        let unit = selection["source"]["context"]["context_id"].as_str().map(str::to_owned);
        links.push(read_link(selection, unit)?);
    }
    let mut blocks = vec![];
    for block in contexts.as_array().ok_or_else(|| malformed("contexts is not a list"))? {
        let id = text_at(block, "id")?.to_owned();
        let source = selections
            .iter()
            .map(|selection| &selection["source"])
            .find(|source| source["id"] == block["source_id"])
            .ok_or_else(|| malformed(format!("{id}: no selection has its source")))?;
        blocks.push(read_block(id, source, block)?);
    }
    Ok(QueryView { links, blocks })
}

fn snapshot_of<'a>(input: &CompositionInput<'a>, path: &str) -> Result<SnapshotInput<'a>, CompositionError> {
    input.snapshots.iter().find(|snapshot| snapshot.path == path).copied().ok_or_else(|| CompositionError::MissingSnapshot { path: path.to_owned() })
}

fn normalized(input: &CompositionInput, unit: &str, path: &str, start_line: usize, end_line: usize) -> Result<NormalizedWindow, CompositionError> {
    normalize_window(snapshot_of(input, path)?.bytes, start_line, end_line, input.form.representation())
        .map_err(|error| CompositionError::Normalization { unit: unit.to_owned(), path: path.to_owned(), error })
}

/// The fichas and units of a query: one unit per window, in the query's order, and each ficha tied
/// to the unit the query says it went into. Whatever is returned has passed U1 to U5.
pub fn compose(input: &CompositionInput) -> Result<ComposedEvidence, CompositionError> {
    let view = read_query(input.query_json)?;
    let mut units = vec![];
    for block in &view.blocks {
        let snapshot = snapshot_of(input, &block.path)?;
        let window = normalized(input, &block.id, &block.path, block.start_line, block.end_line)?;
        units.push(Unit {
            evidence_id: UnitId(block.id.clone()),
            source_id: snapshot.source_id.to_owned(),
            source_sha256: source_sha256(snapshot.bytes),
            path: block.path.clone(),
            window_start_line: window.window_start_line,
            window_end_line: window.window_end_line,
            start_line: window.start_line,
            end_line: window.end_line,
            text_sha256: window.text_sha256,
            excerpt_sha256: window.excerpt_sha256,
        });
    }
    if input.seeds.len() != view.links.len() {
        return Err(malformed(format!("{} fichas prepared, {} selections in the query", input.seeds.len(), view.links.len())));
    }
    let mut fichas = vec![];
    for (seed, link) in input.seeds.iter().zip(&view.links) {
        let ficha = seed.reference.0.clone();
        let agrees = seed.reference.0 == link.reference && seed.path == link.path && seed.start_line == link.line && seed.end_line == link.end_line;
        if !agrees {
            return Err(Violation::SeedDiffersFromQuery { ficha }.into());
        }
        let Some(unit) = link.unit.clone() else {
            return Err(Violation::FichaWithoutUnit { ficha }.into());
        };
        fichas.push(Ficha {
            reference: seed.reference.clone(),
            kind: seed.kind,
            unit: UnitId(unit),
            symbol_id: seed.symbol_id.clone(),
            chunk_id: seed.chunk_id.clone(),
            path: seed.path.clone(),
            start_line: seed.start_line,
            end_line: seed.end_line,
            // A role is a suggestion's, and no suggestion is read here.
            role: None,
            reasons: seed.reasons.clone(),
            reason_status: seed.reason_status,
            declared_by: seed.declared_by,
        });
    }
    let composed = ComposedEvidence { representation: input.form.representation(), fichas, units };
    check_view(&composed, input, &view)?;
    Ok(composed)
}

/// U1 to U5 of a composition against its inputs. U6 is not here.
pub fn check(composed: &ComposedEvidence, input: &CompositionInput) -> Result<(), CompositionError> {
    check_view(composed, input, &read_query(input.query_json)?)
}

fn check_view(composed: &ComposedEvidence, input: &CompositionInput, view: &QueryView) -> Result<(), CompositionError> {
    check_form(composed, input, view)?;
    for unit in &composed.units {
        check_hashes_and_numbers(unit, input, view)?;
        check_text(unit, input)?;
    }
    check_links(composed, input)?;
    check_coverage(composed, input, view)
}

/// U1: every unit is in the form and everything in the form is a unit, with the same id, range and
/// text. An id found twice is not a correspondence.
fn check_form(composed: &ComposedEvidence, input: &CompositionInput, view: &QueryView) -> Result<(), CompositionError> {
    for (n, unit) in composed.units.iter().enumerate() {
        if composed.units[..n].iter().any(|other| other.evidence_id == unit.evidence_id) {
            return Err(Violation::DuplicateUnit { unit: unit.evidence_id.0.clone() }.into());
        }
    }
    let has_unit = |id: &str| composed.units.iter().any(|unit| unit.evidence_id.0 == id);
    match input.form {
        Form::Prose => {
            for block in &view.blocks {
                if !has_unit(&block.id) {
                    return Err(Violation::FormItemWithoutUnit { item: block.id.clone() }.into());
                }
            }
            for unit in &composed.units {
                let id = &unit.evidence_id.0;
                let block = view.blocks.iter().find(|block| &block.id == id).ok_or_else(|| Violation::UnitNotInForm { unit: id.clone() })?;
                let window = normalized(input, id, &unit.path, unit.window_start_line, unit.window_end_line)?;
                let same = block.path == unit.path && block.start_line == unit.start_line && block.end_line == unit.end_line && block.lines == window.line_map;
                if !same {
                    return Err(Violation::UnitDiffersFromForm { unit: id.clone() }.into());
                }
            }
        }
        Form::Claims(items) => {
            for item in items {
                if !has_unit(item.id) {
                    return Err(Violation::FormItemWithoutUnit { item: item.id.to_owned() }.into());
                }
            }
            for unit in &composed.units {
                let id = &unit.evidence_id.0;
                let item = items.iter().find(|item| item.id == id).ok_or_else(|| Violation::UnitNotInForm { unit: id.clone() })?;
                let same = item.path == unit.path
                    && item.start_line == unit.start_line
                    && item.end_line == unit.end_line
                    && unit.excerpt_sha256.as_ref().is_some_and(|hash| hash.as_str() == crate::hash(item.excerpt.as_bytes()));
                if !same {
                    return Err(Violation::UnitDiffersFromForm { unit: id.clone() }.into());
                }
            }
        }
    }
    Ok(())
}

/// U2: the same source hash from the bytes, the edition and the query, and numbers that can be.
fn check_hashes_and_numbers(unit: &Unit, input: &CompositionInput, view: &QueryView) -> Result<(), CompositionError> {
    let id = &unit.evidence_id.0;
    let snapshot = snapshot_of(input, &unit.path)?;
    let in_query = view.blocks.iter().find(|block| &block.id == id).map(|block| block.sha256.as_str());
    let bytes = source_sha256(snapshot.bytes);
    if unit.source_sha256 != bytes || snapshot.sha256 != bytes.as_str() || in_query != Some(bytes.as_str()) || unit.source_id != snapshot.source_id {
        return Err(Violation::SourceHashDiffers { unit: id.clone() }.into());
    }
    let lines = snapshot_lines(snapshot.bytes).map_err(|error| CompositionError::Normalization { unit: id.clone(), path: unit.path.clone(), error })?.len();
    let inside = 1 <= unit.window_start_line && unit.start_line <= unit.end_line && unit.end_line <= unit.window_end_line && unit.window_end_line <= lines;
    if !inside || unit.start_line != unit.window_start_line {
        return Err(Violation::LineNumbers { unit: id.clone() }.into());
    }
    if unit.excerpt_sha256.is_some() != (input.form.representation() == Representation::Claims) {
        return Err(Violation::ExcerptHashForm { unit: id.clone() }.into());
    }
    Ok(())
}

/// U3: the text comes again from the snapshot, by the rule of `lines-v1` and no other.
fn check_text(unit: &Unit, input: &CompositionInput) -> Result<(), CompositionError> {
    let id = &unit.evidence_id.0;
    let window = normalized(input, id, &unit.path, unit.window_start_line, unit.window_end_line)?;
    let same = window.text_sha256 == unit.text_sha256 && window.excerpt_sha256 == unit.excerpt_sha256 && window.end_line == unit.end_line;
    if !same {
        return Err(Violation::TextDiffers { unit: id.clone() }.into());
    }
    Ok(())
}

/// U4: each ficha points to one existing unit, of its source and its edition, with its lines inside
/// the window; each unit has a ficha. The edition shows in the hash it recorded for the source.
fn check_links(composed: &ComposedEvidence, input: &CompositionInput) -> Result<(), CompositionError> {
    for (n, ficha) in composed.fichas.iter().enumerate() {
        let reference = &ficha.reference.0;
        if composed.fichas[..n].iter().any(|other| other.reference == ficha.reference) {
            return Err(Violation::DuplicateFicha { ficha: reference.clone() }.into());
        }
        let unit = composed.units.iter().find(|unit| unit.evidence_id == ficha.unit).ok_or_else(|| Violation::FichaUnitUnknown { ficha: reference.clone(), unit: ficha.unit.0.clone() })?;
        let seed = input.seeds.iter().find(|seed| seed.reference == ficha.reference).ok_or_else(|| Violation::SeedDiffersFromQuery { ficha: reference.clone() })?;
        if ficha.path != unit.path || seed.path != unit.path || seed.source_id != unit.source_id || seed.source_sha256 != unit.source_sha256.as_str() {
            return Err(Violation::FichaOtherSource { ficha: reference.clone(), unit: ficha.unit.0.clone() }.into());
        }
        if ficha.start_line < unit.window_start_line || ficha.end_line < ficha.start_line || unit.window_end_line < ficha.end_line {
            return Err(Violation::FichaOutsideWindow { ficha: reference.clone(), unit: ficha.unit.0.clone() }.into());
        }
    }
    for unit in &composed.units {
        if !composed.fichas.iter().any(|ficha| ficha.unit == unit.evidence_id) {
            return Err(Violation::UnitWithoutFicha { unit: unit.evidence_id.0.clone() }.into());
        }
    }
    Ok(())
}

/// A ficha's lines widened by the radius, stopping at the ends of the source.
fn widened(start_line: usize, end_line: usize, radius: usize, lines: usize) -> (usize, usize) {
    (start_line.saturating_sub(radius).max(1), end_line.saturating_add(radius).min(lines))
}

/// U5: the radius is the input's in every selection, and the union of the fichas of a unit, widened
/// by it, is that unit's window: one range, and no line more or less.
fn check_coverage(composed: &ComposedEvidence, input: &CompositionInput, view: &QueryView) -> Result<(), CompositionError> {
    for unit in &composed.units {
        let snapshot = snapshot_of(input, &unit.path)?;
        let id = &unit.evidence_id.0;
        let lines = snapshot_lines(snapshot.bytes).map_err(|error| CompositionError::Normalization { unit: id.clone(), path: unit.path.clone(), error })?.len();
        let mut ranges = vec![];
        for ficha in composed.fichas.iter().filter(|ficha| ficha.unit == unit.evidence_id) {
            let range = widened(ficha.start_line, ficha.end_line, input.radius, lines);
            let link = view.links.iter().find(|link| link.reference == ficha.reference.0);
            if !link.is_some_and(|link| link.requested_radius == Some(input.radius) && (link.window_start, link.window_end) == range) {
                return Err(Violation::RadiusDiffers { ficha: ficha.reference.0.clone() }.into());
            }
            ranges.push(range);
        }
        ranges.sort_unstable();
        let mut found: Vec<(usize, usize)> = vec![];
        for (start, end) in ranges {
            match found.last_mut() {
                Some(last) if start <= last.1 + 1 => last.1 = last.1.max(end),
                _ => found.push((start, end)),
            }
        }
        let expected = (unit.window_start_line, unit.window_end_line);
        if found != [expected] {
            return Err(Violation::CoverageDiffers { unit: id.clone(), expected, found }.into());
        }
    }
    Ok(())
}

// Provenance of the query, and U6. Every field of a prepared query has a row below; a field with no
// row, or a row with no field where one is required, is refused. The hash of a query says its bytes
// were not changed; what says where each part came from is this, and nothing here says any part is
// true, enough or good. Pure, like the rest: what cannot be seen in the inputs is told as such.

/// What a row says of a field: its class, where it comes from, what wrote it, how far that can be
/// told, and the rule when it has its own. `Reading` marks what depends on the value or the ficha.
#[derive(Clone, Copy)]
struct Kind(QueryFieldClass, FieldOrigin, FieldProducer, FieldProvenance, Option<&'static str>, Reading);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reading {
    AsWritten,
    /// Null when the edition recorded no commit.
    GitCommit,
    /// A statement or an authorship: the acervo's in a reviewed fact, the server's otherwise.
    FichaText,
    /// Where a ficha is: the acervo's in a reviewed fact, the edition's otherwise.
    FichaLine,
    /// The client's text, with or without what a retry added.
    Question,
    /// System text, unless a line of it came from a client.
    Limit,
    /// As far as the fact's record was among the inputs.
    ReviewedFact,
    /// The prose form's instructions: the project's when the request has them, the contract's if not.
    ProseInstructions,
}

use FieldOrigin as Of;
use FieldProducer as By;
use FieldProvenance as Is;
use QueryFieldClass as Class;

const LINES_V1: Option<&str> = Some("lines-v1");
const SHAPE: Kind = Kind(Class::Structure, Of::Server, By::GraphEngine, Is::SystemGenerated, None, Reading::AsWritten);
const CODE: Kind = Kind(Class::Code, Of::Edition, By::GraphEngine, Is::EditionVerified, LINES_V1, Reading::AsWritten);
const SOURCE: Kind = Kind(Class::SourceMetadata, Of::Edition, By::Preparation, Is::EditionVerified, None, Reading::AsWritten);
const COMMIT: Kind = Kind(Class::SourceMetadata, Of::Edition, By::Preparation, Is::EditionVerified, None, Reading::GitCommit);
const FICHA_TEXT: Kind = Kind(Class::FichaDescription, Of::Server, By::Preparation, Is::SystemGenerated, None, Reading::FichaText);
const FICHA_LINE: Kind = Kind(Class::SourceMetadata, Of::Edition, By::Preparation, Is::EditionVerified, None, Reading::FichaLine);
const IDENTITY: Kind = Kind(Class::QueryIdentity, Of::Server, By::Preparation, Is::SystemGenerated, None, Reading::AsWritten);
const LIMIT: Kind = Kind(Class::Limit, Of::Server, By::LimitEntries, Is::SystemGenerated, None, Reading::Limit);
const SCOPE: Kind = Kind(Class::FixedContract, Of::Contract, By::GraphEngine, Is::SystemFixed, None, Reading::AsWritten);
const LITERAL: Kind = Kind(Class::FixedContract, Of::Contract, By::Preparation, Is::SystemFixed, None, Reading::AsWritten);
const RADIUS: Kind = Kind(Class::RequestSetting, Of::Request, By::GraphEngine, Is::RequestRecorded, None, Reading::AsWritten);
const QUESTION: Kind = Kind(Class::ClientText, Of::Client, By::Client, Is::UnverifiedClientText, None, Reading::Question);
const PROSE_RULES: Kind = Kind(Class::FixedContract, Of::Contract, By::GraphEngine, Is::SystemFixed, None, Reading::ProseInstructions);
const CLAIMS_SHAPE: Kind = Kind(Class::Structure, Of::Server, By::ClaimsEngine, Is::SystemGenerated, None, Reading::AsWritten);
const CLAIMS_CODE: Kind = Kind(Class::Code, Of::Edition, By::ClaimsContext, Is::EditionVerified, LINES_V1, Reading::AsWritten);
const CLAIMS_RULES: Kind = Kind(Class::FixedContract, Of::Contract, By::ClaimsEngine, Is::SystemFixed, None, Reading::AsWritten);
const DERIVED: Kind = Kind(Class::DerivedFromEdition, Of::Edition, By::ClaimsContext, Is::SystemGenerated, None, Reading::AsWritten);
const REVIEWED: Kind = Kind(Class::ReviewedFact, Of::Acervo, By::ClaimsContext, Is::AcervoQuotesChecked, None, Reading::ReviewedFact);
const PROJECT_TEXT: Kind = Kind(Class::ProjectInstructions, Of::Project, By::ClaimsEngine, Is::ProjectDeclared, None, Reading::AsWritten);
const SETTING: Kind = Kind(Class::ProjectSettings, Of::Project, By::ClaimsEngine, Is::ProjectDeclared, None, Reading::AsWritten);

/// A selection of the prose form, from where it starts; a leading `?` says it may be absent.
const PROSE_SELECTION: &[(&str, Kind)] = &[
    ("", SHAPE),
    (".evidence_id", IDENTITY),
    (".evidence_unknowns", LIMIT),
    (".evidence_unknowns[]", LIMIT),
    (".unknowns_scope", SCOPE),
    (".fact_id", SHAPE),
    (".statement", FICHA_TEXT),
    (".authorship", FICHA_TEXT),
    (".metadata_scope", SCOPE),
    (".scope", SCOPE),
    (".source", SHAPE),
    (".source.id", SHAPE),
    (".source.kind", SOURCE),
    (".source.git_commit", COMMIT),
    (".source.path", SOURCE),
    (".source.sha256", SOURCE),
    (".source.line", FICHA_LINE),
    (".source.end_line", FICHA_LINE),
    (".source.excerpt", CODE),
    (".source.content", LITERAL),
    (".source.execution_scope", SCOPE),
    (".source.context", SHAPE),
    (".source.context.requested_radius", RADIUS),
    (".source.context.start_line", CODE),
    (".source.context.end_line", CODE),
];
/// A single selection keeps its lines inline; several name a block.
const PROSE_INLINE: &[(&str, Kind)] = &[
    ("?.source.context.numbering", SCOPE),
    (".source.context.lines", CODE),
    (".source.context.lines[]", CODE),
    (".source.context.lines[].number", CODE),
    (".source.context.lines[].text", CODE),
];
const PROSE_LINKED: &[(&str, Kind)] = &[(".source.context.context_id", SHAPE)];
const PROSE_SHARED: &[(&str, Kind)] = &[
    ("$.evidence.selections", SHAPE),
    ("$.evidence.scope", SCOPE),
    ("$.evidence.contexts", SHAPE),
    ("$.evidence.contexts[]", SHAPE),
    ("$.evidence.contexts[].id", SHAPE),
    ("$.evidence.contexts[].source_id", SHAPE),
    ("$.evidence.contexts[].start_line", CODE),
    ("$.evidence.contexts[].end_line", CODE),
    ("?$.evidence.contexts[].numbering", SCOPE),
    ("$.evidence.contexts[].lines", CODE),
    ("$.evidence.contexts[].lines[]", CODE),
    ("$.evidence.contexts[].lines[].number", CODE),
    ("$.evidence.contexts[].lines[].text", CODE),
];
const PROSE_QUERY: &[(&str, Kind)] = &[("$", SHAPE), ("$.question", QUESTION), ("$.evidence", SHAPE), ("$.instructions", PROSE_RULES), ("$.instructions[]", PROSE_RULES)];

const CLAIMS_QUERY: &[(&str, Kind)] = &[
    ("$", CLAIMS_SHAPE),
    ("$.question", QUESTION),
    ("$.evidence", CLAIMS_SHAPE),
    ("$.evidence.items", CLAIMS_SHAPE),
    ("$.evidence.items[]", CLAIMS_SHAPE),
    ("$.evidence.items[].id", CLAIMS_SHAPE),
    ("$.evidence.items[].path", CLAIMS_CODE),
    ("$.evidence.items[].start_line", CLAIMS_CODE),
    ("$.evidence.items[].end_line", CLAIMS_CODE),
    ("?$.evidence.items[].excerpt", CLAIMS_CODE),
    ("?$.evidence.items[].ends_file", DERIVED),
    ("?$.evidence.items[].symbols", DERIVED),
    ("$.evidence.items[].symbols[]", DERIVED),
    ("$.evidence.items[].symbols[].name", DERIVED),
    ("$.evidence.items[].symbols[].declaration_line", DERIVED),
    ("$.evidence.items[].symbols[].end_line", DERIVED),
    ("$.evidence.items[].symbols[].name_column", DERIVED),
    ("$.evidence.line_map", CLAIMS_CODE),
    ("$.evidence.line_map[]", CLAIMS_CODE),
    ("$.evidence.line_map[].evidence_id", CLAIMS_SHAPE),
    ("$.evidence.line_map[].lines", CLAIMS_CODE),
    ("$.evidence.line_map[].lines[]", CLAIMS_CODE),
    ("$.evidence.line_map[].lines[].number", CLAIMS_CODE),
    ("$.evidence.line_map[].lines[].text", CLAIMS_CODE),
    ("?$.evidence.limits", LIMIT),
    ("$.evidence.limits[]", LIMIT),
    ("?$.evidence.reviewed_facts", REVIEWED),
    ("$.evidence.reviewed_facts[]", REVIEWED),
    ("$.evidence.reviewed_facts[].id", REVIEWED),
    ("$.evidence.reviewed_facts[].statement", REVIEWED),
    ("$.evidence.reviewed_facts[].authorship", REVIEWED),
    ("$.evidence.reviewed_facts[].path", REVIEWED),
    ("$.evidence.reviewed_facts[].start_line", REVIEWED),
    ("$.evidence.reviewed_facts[].end_line", REVIEWED),
    ("$.policy", SETTING),
    ("$.policy.allow_inference", SETTING),
    ("$.policy.allow_hypothesis", SETTING),
    ("?$.allow_edits", SETTING),
    ("?$.allow_patch", SETTING),
    ("$.require_location", DERIVED),
    ("?$.precise_location", CLAIMS_RULES),
    ("?$.strict_text", CLAIMS_RULES),
    ("$.instructions", CLAIMS_RULES),
    ("$.instructions[]", CLAIMS_RULES),
    ("?$.project_instructions", PROJECT_TEXT),
    ("$.project_instructions[]", PROJECT_TEXT),
];

/// The answer's form, whole. It says nothing of any ficha: no evidence id, no path is written in it.
const RESPONSE_SCHEMA: &[&str] = &[
    "", ".additionalProperties", ".properties", ".required", ".required[]", ".type", ".properties.schema_version", ".properties.schema_version.enum", ".properties.schema_version.enum[]", ".properties.schema_version.type",
    ".properties.claims", ".properties.claims.minItems", ".properties.claims.type", ".properties.claims.items", ".properties.claims.items.additionalProperties", ".properties.claims.items.required", ".properties.claims.items.required[]", ".properties.claims.items.type", ".properties.claims.items.properties",
    ".properties.claims.items.properties.id", ".properties.claims.items.properties.id.minLength", ".properties.claims.items.properties.id.type",
    ".properties.claims.items.properties.kind", ".properties.claims.items.properties.kind.enum", ".properties.claims.items.properties.kind.enum[]", ".properties.claims.items.properties.kind.type",
    ".properties.claims.items.properties.text", ".properties.claims.items.properties.text.minLength", ".properties.claims.items.properties.text.type",
    ".properties.claims.items.properties.premises", ".properties.claims.items.properties.premises.type", ".properties.claims.items.properties.premises.items", ".properties.claims.items.properties.premises.items.minLength", ".properties.claims.items.properties.premises.items.type",
    ".properties.claims.items.properties.verification_plan", ".properties.claims.items.properties.verification_plan.type", ".properties.claims.items.properties.verification_plan.type[]",
    ".properties.claims.items.properties.citations", ".properties.claims.items.properties.citations.type", ".properties.claims.items.properties.citations.items", ".properties.claims.items.properties.citations.items.additionalProperties", ".properties.claims.items.properties.citations.items.properties", ".properties.claims.items.properties.citations.items.required", ".properties.claims.items.properties.citations.items.required[]", ".properties.claims.items.properties.citations.items.type",
    ".properties.claims.items.properties.citations.items.properties.evidence_id", ".properties.claims.items.properties.citations.items.properties.evidence_id.minLength", ".properties.claims.items.properties.citations.items.properties.evidence_id.type",
    ".properties.claims.items.properties.citations.items.properties.path", ".properties.claims.items.properties.citations.items.properties.path.minLength", ".properties.claims.items.properties.citations.items.properties.path.type",
    ".properties.claims.items.properties.citations.items.properties.start_line", ".properties.claims.items.properties.citations.items.properties.start_line.minimum", ".properties.claims.items.properties.citations.items.properties.start_line.type",
    ".properties.claims.items.properties.citations.items.properties.end_line", ".properties.claims.items.properties.citations.items.properties.end_line.minimum", ".properties.claims.items.properties.citations.items.properties.end_line.type",
    ".properties.claims.items.properties.citations.items.properties.quote", ".properties.claims.items.properties.citations.items.properties.quote.type",
    ".properties.claims.items.properties.location", ".properties.claims.items.properties.location.anyOf", ".properties.claims.items.properties.location.anyOf[]", ".properties.claims.items.properties.location.anyOf[].type",
    "?.properties.claims.items.properties.location.anyOf[].additionalProperties", "?.properties.claims.items.properties.location.anyOf[].required", ".properties.claims.items.properties.location.anyOf[].required[]", "?.properties.claims.items.properties.location.anyOf[].properties",
    ".properties.claims.items.properties.location.anyOf[].properties.evidence_id", ".properties.claims.items.properties.location.anyOf[].properties.evidence_id.minLength", ".properties.claims.items.properties.location.anyOf[].properties.evidence_id.type",
    ".properties.claims.items.properties.location.anyOf[].properties.path", ".properties.claims.items.properties.location.anyOf[].properties.path.minLength", ".properties.claims.items.properties.location.anyOf[].properties.path.type",
    ".properties.claims.items.properties.location.anyOf[].properties.symbol_name", ".properties.claims.items.properties.location.anyOf[].properties.symbol_name.minLength", ".properties.claims.items.properties.location.anyOf[].properties.symbol_name.type",
    ".properties.claims.items.properties.location.anyOf[].properties.declaration_line", ".properties.claims.items.properties.location.anyOf[].properties.declaration_line.minimum", ".properties.claims.items.properties.location.anyOf[].properties.declaration_line.type",
    ".properties.claims.items.properties.location.anyOf[].properties.operation_start_line", ".properties.claims.items.properties.location.anyOf[].properties.operation_start_line.minimum", ".properties.claims.items.properties.location.anyOf[].properties.operation_start_line.type",
    ".properties.claims.items.properties.location.anyOf[].properties.operation_end_line", ".properties.claims.items.properties.location.anyOf[].properties.operation_end_line.minimum", ".properties.claims.items.properties.location.anyOf[].properties.operation_end_line.type",
    "?.properties.claims.items.properties.patch", ".properties.claims.items.properties.patch.type", ".properties.claims.items.properties.patch.type[]",
    "?.properties.claims.items.properties.edits", ".properties.claims.items.properties.edits.type", ".properties.claims.items.properties.edits.type[]", ".properties.claims.items.properties.edits.items", ".properties.claims.items.properties.edits.items.additionalProperties", ".properties.claims.items.properties.edits.items.required", ".properties.claims.items.properties.edits.items.required[]", ".properties.claims.items.properties.edits.items.type", ".properties.claims.items.properties.edits.items.properties",
    ".properties.claims.items.properties.edits.items.properties.path", ".properties.claims.items.properties.edits.items.properties.path.minLength", ".properties.claims.items.properties.edits.items.properties.path.type",
    ".properties.claims.items.properties.edits.items.properties.op", ".properties.claims.items.properties.edits.items.properties.op.enum", ".properties.claims.items.properties.edits.items.properties.op.enum[]", ".properties.claims.items.properties.edits.items.properties.op.type",
    ".properties.claims.items.properties.edits.items.properties.start_line", ".properties.claims.items.properties.edits.items.properties.start_line.minimum", ".properties.claims.items.properties.edits.items.properties.start_line.type",
    ".properties.claims.items.properties.edits.items.properties.end_line", ".properties.claims.items.properties.edits.items.properties.end_line.minimum", ".properties.claims.items.properties.edits.items.properties.end_line.type",
    ".properties.claims.items.properties.edits.items.properties.new_lines", ".properties.claims.items.properties.edits.items.properties.new_lines.type", ".properties.claims.items.properties.edits.items.properties.new_lines.items", ".properties.claims.items.properties.edits.items.properties.new_lines.items.type",
];

/// A fact's line in the acervo, in its exact bytes. Whoever has the acervo hands it over: nothing
/// here reads a file, and a fact with no line among the inputs says so.
#[derive(Debug, Clone, Copy)]
pub struct FactRecord<'a> {
    pub fact_id: &'a str,
    pub bytes: &'a [u8],
}

/// What a server retry added, with the rule that made it add that.
#[derive(Debug, Clone, Copy)]
pub struct SuffixInput<'a> {
    pub text: &'a str,
    pub rule: RetryRule,
    pub attempt: usize,
}

/// The question as the client sent it and what a retry added to it, in order.
#[derive(Debug, Clone, Copy)]
pub struct QuestionInput<'a> {
    pub original: &'a str,
    pub suffixes: &'a [SuffixInput<'a>],
}

/// Everything the provenance of a query depends on. `composed` is what `compose` returned for it.
#[derive(Debug, Clone, Copy)]
pub struct ProvenanceInput<'a> {
    /// The query of `composed.representation`, in the bytes that file has.
    pub query_json: &'a str,
    pub question: QuestionInput<'a>,
    /// The bytes of `search.json`, whose hash names the dossier.
    pub search: &'a [u8],
    pub composition: &'a Composition,
    pub composed: &'a ComposedEvidence,
    pub snapshots: &'a [SnapshotInput<'a>],
    pub fact_records: &'a [FactRecord<'a>],
    /// The instructions the project declares, when they can be told at this layer.
    pub project_instructions: Option<&'a [String]>,
}

struct Rows {
    kinds: std::collections::BTreeMap<String, Kind>,
    /// Each object's fields a query has to have.
    required: std::collections::BTreeMap<String, Vec<String>>,
}

impl Rows {
    fn new() -> Self {
        Self { kinds: Default::default(), required: Default::default() }
    }
    fn add(&mut self, row: &str, prefix: &str, kind: Kind) {
        let (optional, row) = match row.strip_prefix('?') {
            Some(rest) => (true, rest),
            None => (false, row),
        };
        let path = format!("{prefix}{row}");
        if !optional {
            // The elements of a list are not a field of anything: the list is.
            if let Some((parent, name)) = path.rsplit_once('.').filter(|(_, name)| !name.ends_with("[]")) {
                self.required.entry(parent.to_owned()).or_default().push(name.to_owned());
            }
        }
        self.kinds.insert(path, kind);
    }
    fn prose(single: bool) -> Self {
        let mut rows = Self::new();
        for (row, kind) in PROSE_QUERY {
            rows.add(row, "", *kind);
        }
        let selection = if single { "$.evidence" } else { "$.evidence.selections[]" };
        // The evidence itself was added above: its own row is not a selection's in the single form.
        for (row, kind) in PROSE_SELECTION.iter().filter(|(row, _)| !(single && row.is_empty())) {
            rows.add(row, selection, *kind);
        }
        if single {
            for (row, kind) in PROSE_INLINE {
                rows.add(row, selection, *kind);
            }
        } else {
            for (row, kind) in PROSE_LINKED {
                rows.add(row, selection, *kind);
            }
            for (row, kind) in PROSE_SHARED {
                rows.add(row, "", *kind);
            }
        }
        rows
    }
    fn claims() -> Self {
        let mut rows = Self::new();
        for (row, kind) in CLAIMS_QUERY {
            rows.add(row, "", *kind);
        }
        for row in RESPONSE_SCHEMA {
            let (optional, name) = match row.strip_prefix('?') {
                Some(rest) => ("?", rest),
                None => ("", *row),
            };
            rows.add(&format!("{optional}$.response_schema{name}"), "", CLAIMS_RULES);
        }
        rows
    }
}

struct Classifier<'a> {
    rows: Rows,
    input: &'a ProvenanceInput<'a>,
    fields: Vec<ClassifiedField>,
    /// Each field's values for its place, in order: one value when it appears once.
    values: Vec<Vec<&'a serde_json::Value>>,
    places: std::collections::BTreeMap<(String, Option<String>, Option<String>), usize>,
    /// What the fixed contract is made of: only fields that are the same wherever they appear.
    fixed: Vec<(String, &'a serde_json::Value)>,
}

fn is_reviewed(input: &ProvenanceInput, ficha: Option<&str>) -> bool {
    ficha.is_some_and(|ficha| input.composition.reviewed.iter().any(|fact| fact.id == ficha))
}

fn has_record(input: &ProvenanceInput, fact: &str) -> bool {
    input.fact_records.iter().any(|record| record.fact_id == fact)
}

impl<'a> Classifier<'a> {
    /// What a row says once the value and the ficha are known.
    fn read(&self, kind: Kind, value: &serde_json::Value, ficha: Option<&str>) -> (QueryFieldClass, FieldOrigin, FieldProducer, FieldProvenance, Option<&'static str>) {
        let Kind(class, origin, producer, provenance, rule, reading) = kind;
        let input = self.input;
        let of_fact = |fact: &str| if has_record(input, fact) { Is::AcervoQuotesChecked } else { Is::RecordNotAvailable };
        match reading {
            Reading::AsWritten => (class, origin, producer, provenance, rule),
            Reading::GitCommit if value.is_null() => (class, origin, producer, Is::NotAvailable, rule),
            Reading::GitCommit => (class, origin, producer, provenance, rule),
            Reading::FichaText | Reading::FichaLine => match ficha.filter(|_| is_reviewed(input, ficha)) {
                Some(fact) => (Class::ReviewedFact, Of::Acervo, producer, of_fact(fact), rule),
                None => (class, origin, producer, provenance, rule),
            },
            Reading::Question if input.question.suffixes.is_empty() => (class, origin, producer, provenance, rule),
            Reading::Question => (Class::ClientTextWithServerSuffixes, origin, producer, provenance, rule),
            Reading::Limit if input.composition.limits.iter().any(|limit| limit.provenance == LimitProvenance::UnverifiedClientText) => (class, Of::Client, producer, Is::UnverifiedClientText, rule),
            Reading::Limit => (class, origin, producer, provenance, rule),
            Reading::ReviewedFact => {
                let every_record = input.composition.reviewed.iter().all(|fact| has_record(input, &fact.id));
                let is_there = ficha.map_or(every_record, |fact| has_record(input, fact));
                (class, origin, producer, if is_there { Is::AcervoQuotesChecked } else { Is::RecordNotAvailable }, rule)
            }
            Reading::ProseInstructions if input.composition.instructions.is_some() => (Class::ProjectInstructions, Of::Project, By::Preparation, Is::ProjectDeclared, rule),
            Reading::ProseInstructions => (class, origin, producer, provenance, rule),
        }
    }

    fn walk(&mut self, path: &str, value: &'a serde_json::Value, ficha: Option<&str>, unit: Option<&str>, inside_fixed: bool) -> Result<(), Violation> {
        let Some(kind) = self.rows.kinds.get(path).copied() else {
            return Err(Violation::UnknownField { path: path.to_owned() });
        };
        // What a place in the query belongs to: the ficha or the unit it is written for.
        let name = |field: &str| value[field].as_str().map(str::to_owned);
        let (own_ficha, own_unit) = match path {
            "$.evidence.selections[]" => (name("fact_id"), value["source"]["context"]["context_id"].as_str().map(str::to_owned)),
            "$.evidence" if value.get("fact_id").is_some() => (name("fact_id"), name("fact_id")),
            "$.evidence.contexts[]" | "$.evidence.items[]" => (None, name("id")),
            "$.evidence.line_map[]" => (None, name("evidence_id")),
            "$.evidence.reviewed_facts[]" => (name("id"), None),
            _ => (None, None),
        };
        let ficha = own_ficha.as_deref().or(ficha);
        let unit = own_unit.as_deref().or(unit);
        let (class, origin, producer, provenance, rule) = self.read(kind, value, ficha);
        let fixed = class == Class::FixedContract;
        if fixed && !inside_fixed {
            // A fixed field has one value, wherever it is written.
            if self.fixed.iter().any(|(other, seen)| other == path && *seen != value) {
                return Err(Violation::FixedFieldVaries { path: path.to_owned() });
            }
            if !self.fixed.iter().any(|(other, _)| other == path) {
                self.fixed.push((path.to_owned(), value));
            }
        }
        match value {
            serde_json::Value::Object(object) => {
                for required in self.rows.required.get(path).into_iter().flatten() {
                    if !object.contains_key(required) {
                        return Err(Violation::MissingField { path: format!("{path}.{required}") });
                    }
                }
                for (field, child) in object {
                    self.walk(&format!("{path}.{field}"), child, ficha, unit, fixed)?;
                }
            }
            serde_json::Value::Array(items) if !items.is_empty() => {
                let element = format!("{path}[]");
                for item in items {
                    self.walk(&element, item, ficha, unit, fixed)?;
                }
            }
            _ => {
                let place = (path.to_owned(), ficha.map(str::to_owned), unit.map(str::to_owned));
                let index = match self.places.get(&place) {
                    Some(index) => *index,
                    None => {
                        self.places.insert(place, self.fields.len());
                        self.fields.push(ClassifiedField {
                            path: path.to_owned(),
                            class,
                            origin,
                            producer,
                            rule_version: rule.map(str::to_owned),
                            provenance,
                            ficha: ficha.map(|f| FichaRef(f.to_owned())),
                            unit: unit.map(|u| UnitId(u.to_owned())),
                            value_sha256: None,
                        });
                        self.values.push(vec![]);
                        self.fields.len() - 1
                    }
                };
                self.values[index].push(value);
            }
        }
        Ok(())
    }
}

/// The hash of a value's compact JSON text.
fn json_hash<T: Serialize>(value: &T) -> Result<Sha256Hex, CompositionError> {
    Ok(sha256_of(serde_json::to_string(value).map_err(|error| malformed(error.to_string()))?.as_bytes()))
}

fn differs(item: &str, field: &str) -> CompositionError {
    Violation::FieldDiffers { item: item.to_owned(), field: field.to_owned() }.into()
}

fn lines_of<'a>(input: &ProvenanceInput<'a>, path: &str) -> Result<Vec<&'a str>, CompositionError> {
    let snapshot = input.snapshots.iter().find(|snapshot| snapshot.path == path).ok_or_else(|| CompositionError::MissingSnapshot { path: path.to_owned() })?;
    snapshot_lines(snapshot.bytes).map_err(|error| CompositionError::Normalization { unit: path.to_owned(), path: path.to_owned(), error })
}

/// The prepared question is the client's text with what the retries added, and nothing else.
fn question_parts(input: &ProvenanceInput, query: &serde_json::Value) -> Result<(PersonText, Vec<ServerSuffix>, Sha256Hex), CompositionError> {
    let prepared = query["question"].as_str().ok_or_else(|| malformed("no question"))?;
    let QuestionInput { original, suffixes } = input.question;
    let composed: String = std::iter::once(original).chain(suffixes.iter().map(|suffix| suffix.text)).collect();
    if composed != prepared {
        return Err(Violation::QuestionDiffers.into());
    }
    let person = PersonText { sha256: sha256_of(original.as_bytes()), declared_by: ClientDeclared::Client, provenance: ClientTextProvenance::UnverifiedClientText };
    let suffixes = suffixes
        .iter()
        .enumerate()
        .map(|(n, suffix)| ServerSuffix { order: n + 1, sha256: sha256_of(suffix.text.as_bytes()), declared_by: ServerRetryDeclared::ServerRetry, rule: suffix.rule, attempt: suffix.attempt })
        .collect();
    Ok((person, suffixes, sha256_of(prepared.as_bytes())))
}

/// One entry per line of the query's limits, each from the line the preparation tagged. A line with
/// no tagged line behind it, however good the hash of the query, has no origin to record.
fn limit_entries(input: &ProvenanceInput, written: &[&str]) -> Result<Vec<LimitEntry>, CompositionError> {
    let tagged: Vec<&TaggedLimit> = match input.composed.representation {
        Representation::Prose => input.composition.limits.iter().collect(),
        // The claims form keeps the first of two lines with the same text.
        Representation::Claims => {
            let mut once: Vec<&TaggedLimit> = vec![];
            for limit in &input.composition.limits {
                if !once.iter().any(|other| other.text == limit.text) {
                    once.push(limit);
                }
            }
            once
        }
    };
    let mut entries = vec![];
    for (index, text) in written.iter().enumerate() {
        let limit = tagged.get(index).ok_or(Violation::LimitWithoutOrigin { index })?;
        if limit.text != *text {
            return Err(Violation::LimitTextDiffers { index }.into());
        }
        let class = if limit.provenance == LimitProvenance::UnverifiedClientText { LimitClass::ClientText } else { LimitClass::SystemText };
        entries.push(LimitEntry { index, text_sha256: sha256_of(text.as_bytes()), producer: limit.producer, input: limit.input, rule_version: limit.rule_version.clone(), basis: None, class, provenance: limit.provenance });
    }
    if written.len() != tagged.len() {
        return Err(Violation::LimitTextDiffers { index: written.len() }.into());
    }
    Ok(entries)
}

fn text_list(value: &serde_json::Value) -> Result<Vec<&str>, CompositionError> {
    value.as_array().ok_or_else(|| malformed("no list of limits"))?.iter().map(|line| line.as_str().ok_or_else(|| malformed("a limit is not text"))).collect()
}

/// The record says what the query carries: same fact, same statement, same lines, same citations.
fn check_record(fact: &crate::prepare::ReviewedFact, bytes: &[u8]) -> Result<(), CompositionError> {
    let wrong = |field: &str| CompositionError::from(Violation::FactRecordDiffers { fact: fact.id.clone(), field: field.to_owned() });
    let record: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| wrong("record"))?;
    let citations: Vec<serde_json::Value> = fact.citations.iter().map(|c| serde_json::json!({ "path": c.path, "start_line": c.start_line, "end_line": c.end_line, "quote": c.quote })).collect();
    let checks = [
        ("event", record["event"] == "promoted"),
        ("id", record["id"] == fact.id.as_str()),
        ("statement", record["statement"] == fact.statement.as_str()),
        ("authorship", record["authorship"] == fact.authorship.as_str()),
        ("path", record["path"] == fact.path.as_str()),
        ("line", record["line"] == fact.line),
        ("end_line", record["end_line"] == fact.end_line),
        ("citations", record["citations"] == serde_json::Value::Array(citations)),
    ];
    match checks.iter().find(|(_, same)| !same) {
        Some((field, _)) => Err(wrong(field)),
        None => Ok(()),
    }
}

/// What can be said of each reviewed fact: its record, if it came, and its quotes against the
/// snapshots. That the quotes are the edition's lines says nothing of whether the statement is true,
/// and the authorship is what the record declares, not a person's action that anyone saw.
fn reviewed_refs(input: &ProvenanceInput) -> Result<Vec<ReviewedFactRef>, CompositionError> {
    let mut refs = vec![];
    for fact in &input.composition.reviewed {
        let ficha = input.composed.fichas.iter().find(|ficha| ficha.reference.0 == fact.id).ok_or_else(|| Violation::FichaWithoutUnit { ficha: fact.id.clone() })?;
        let record = input.fact_records.iter().find(|record| record.fact_id == fact.id);
        if let Some(record) = record {
            check_record(fact, record.bytes)?;
        }
        let mut citations = vec![];
        for quote in &fact.citations {
            let lines = lines_of(input, &quote.path)?;
            let found = (quote.start_line >= 1 && quote.end_line >= quote.start_line && quote.end_line <= lines.len()).then(|| lines[quote.start_line - 1..quote.end_line].join("\n"));
            if found.as_deref() != Some(quote.quote.as_str()) {
                return Err(Violation::QuoteDiffers { fact: fact.id.clone(), path: quote.path.clone() }.into());
            }
            citations.push(CitationCheck { path: quote.path.clone(), start_line: quote.start_line, end_line: quote.end_line, quote_sha256: sha256_of(quote.quote.as_bytes()), checked_against: QuoteCheckedAgainst::Edition });
        }
        refs.push(ReviewedFactRef {
            fact_id: fact.id.clone(),
            record_sha256: record.map(|record| sha256_of(record.bytes)),
            statement_sha256: sha256_of(fact.statement.as_bytes()),
            ficha: ficha.reference.clone(),
            unit: ficha.unit.clone(),
            citations,
            provenance: if record.is_some() { ReviewedFactProvenance::AcervoQuotesChecked } else { ReviewedFactProvenance::RecordNotAvailable },
        });
    }
    Ok(refs)
}

/// What a selection of the prose form says of its ficha is what the preparation and the snapshot say.
fn check_prose_fields(input: &ProvenanceInput, evidence: &serde_json::Value) -> Result<(), CompositionError> {
    let selections: Vec<&serde_json::Value> = match evidence.get("selections") {
        Some(list) => list.as_array().ok_or_else(|| malformed("selections is not a list"))?.iter().collect(),
        None => vec![evidence],
    };
    let seeds = &input.composition.fichas;
    if selections.len() != seeds.len() {
        return Err(malformed("selections and fichas differ in number"));
    }
    for (selection, seed) in selections.iter().zip(seeds) {
        let ficha = seed.reference.0.as_str();
        let source = &selection["source"];
        let context = &source["context"];
        let lines = lines_of(input, &seed.path)?;
        let expected_excerpt = seed.start_line.checked_sub(1).and_then(|n| lines.get(n)).copied();
        let name = selection["evidence_id"].as_str().and_then(|id| id.rsplit_once(':')).map(|(_, hash)| hash);
        let checks = [
            ("fact_id", selection["fact_id"] == ficha),
            ("evidence_id", name == Some(crate::hash(input.search).as_str())),
            ("statement", selection["statement"] == seed.statement.as_str()),
            ("authorship", selection["authorship"] == seed.authorship.as_str()),
            ("source.kind", source["kind"] == seed.source_kind.as_str()),
            ("source.git_commit", source["git_commit"] == serde_json::json!(seed.git_commit)),
            ("source.path", source["path"] == seed.path.as_str()),
            ("source.sha256", source["sha256"] == seed.source_sha256.as_str()),
            ("source.line", source["line"] == seed.start_line),
            ("source.end_line", source["end_line"] == seed.end_line),
            ("source.excerpt", source["excerpt"].as_str() == expected_excerpt),
            ("source.context.requested_radius", context["requested_radius"] == input.composition.radius),
        ];
        if let Some((field, _)) = checks.iter().find(|(_, same)| !same) {
            return Err(differs(ficha, field));
        }
    }
    let first = text_list(&selections[0]["evidence_unknowns"])?;
    for selection in &selections[1..] {
        if text_list(&selection["evidence_unknowns"])? != first {
            return Err(differs(selection["fact_id"].as_str().unwrap_or("?"), "evidence_unknowns"));
        }
    }
    Ok(())
}

/// What the claims form carries of each unit is the unit, and its facts are the preparation's.
fn check_claims_fields(input: &ProvenanceInput, evidence: &serde_json::Value) -> Result<(), CompositionError> {
    let list = |name: &str| evidence[name].as_array().ok_or_else(|| malformed(format!("no {name}")));
    let items = list("items")?;
    let map = list("line_map")?;
    let units = &input.composed.units;
    if items.len() != units.len() || map.len() != units.len() {
        return Err(malformed("items, line_map and units differ in number"));
    }
    for ((item, lines), unit) in items.iter().zip(map).zip(units) {
        let id = unit.evidence_id.0.as_str();
        let checks = [
            ("id", item["id"] == id),
            ("path", item["path"] == unit.path.as_str()),
            ("start_line", item["start_line"] == unit.start_line),
            ("end_line", item["end_line"] == unit.end_line),
            ("evidence_id", lines["evidence_id"] == id),
        ];
        if let Some((field, _)) = checks.iter().find(|(_, same)| !same) {
            return Err(differs(id, field));
        }
        // Absent in the compact form: the context, compared by U1, still has it.
        if item.get("excerpt").is_some() && Some(crate::hash(item["excerpt"].as_str().unwrap_or_default().as_bytes()).as_str()) != unit.excerpt_sha256.as_ref().map(Sha256Hex::as_str) {
            return Err(differs(id, "excerpt"));
        }
        let lines = lines["lines"].as_array().ok_or_else(|| malformed(format!("{id}: no lines")))?;
        let texts: Vec<&str> = lines.iter().map(|line| line["text"].as_str().unwrap_or_default()).collect();
        let numbered = lines.iter().enumerate().all(|(offset, line)| line["number"] == unit.start_line + offset);
        if !numbered || crate::hash(texts.join("\n").as_bytes()) != unit.text_sha256.as_str() {
            return Err(differs(id, "line_map"));
        }
    }
    let facts: Vec<&serde_json::Value> = evidence.get("reviewed_facts").and_then(serde_json::Value::as_array).map_or(vec![], |list| list.iter().collect());
    if facts.len() != input.composition.reviewed.len() {
        return Err(differs("reviewed_facts", "length"));
    }
    for (written, fact) in facts.iter().zip(&input.composition.reviewed) {
        let checks = [
            ("id", written["id"] == fact.id.as_str()),
            ("statement", written["statement"] == fact.statement.as_str()),
            ("authorship", written["authorship"] == fact.authorship.as_str()),
            ("path", written["path"] == fact.path.as_str()),
            ("start_line", written["start_line"] == fact.line),
            ("end_line", written["end_line"] == fact.end_line),
        ];
        if let Some((field, _)) = checks.iter().find(|(_, same)| !same) {
            return Err(differs(&fact.id, field));
        }
    }
    Ok(())
}

/// The class and the origin of every field of a query, and U6 with them.
pub fn build_provenance(input: &ProvenanceInput) -> Result<QueryProvenance, CompositionError> {
    let query: serde_json::Value = serde_json::from_str(input.query_json).map_err(|error| malformed(error.to_string()))?;
    let representation = input.composed.representation;
    let evidence = &query["evidence"];
    let (rows, limits, project_written) = match representation {
        Representation::Prose => {
            // Several selections need their blocks and one has none: the rows of each shape say so.
            let single = evidence.get("selections").is_none();
            let limits = if single { &evidence["evidence_unknowns"] } else { &evidence["selections"][0]["evidence_unknowns"] };
            let project = input.composition.instructions.is_some().then(|| &query["instructions"]);
            (Rows::prose(single), limits, project)
        }
        Representation::Claims => (Rows::claims(), &evidence["limits"], query.get("project_instructions")),
    };
    let mut classifier = Classifier { rows, input, fields: vec![], values: vec![], places: Default::default(), fixed: vec![] };
    classifier.walk("$", &query, None, None, false)?;
    let Classifier { mut fields, values, fixed, .. } = classifier;
    for (field, values) in fields.iter_mut().zip(&values) {
        // A fixed field's value is in the hash of the contract, and only there.
        if field.class != Class::FixedContract {
            field.value_sha256 = Some(json_hash(values)?);
        }
    }
    match representation {
        Representation::Prose => check_prose_fields(input, evidence)?,
        Representation::Claims => check_claims_fields(input, evidence)?,
    }
    let (person_text, server_suffixes, prepared_question_sha256) = question_parts(input, &query)?;
    let limits = if limits.is_null() { vec![] } else { limit_entries(input, &text_list(limits)?)? };
    let reviewed_facts = reviewed_refs(input)?;
    // What the project declares, if the caller can tell: the text itself is not vouched for.
    if let (Some(declared), Some(written)) = (input.project_instructions, project_written) {
        if written != &serde_json::json!(declared) {
            return Err(Violation::ProjectInstructionsDiffer.into());
        }
    }
    if input.project_instructions.is_some_and(|declared| !declared.is_empty()) && project_written.is_none() {
        return Err(Violation::ProjectInstructionsDiffer.into());
    }
    // In prose the request's own instructions are what the query has to carry, whoever calls this.
    if let (Representation::Prose, Some(own)) = (representation, &input.composition.instructions) {
        if query["instructions"] != serde_json::json!(own) {
            return Err(Violation::ProjectInstructionsDiffer.into());
        }
    }
    Ok(QueryProvenance {
        sha256: sha256_of(input.query_json.as_bytes()),
        code: CodeClass { units: input.composed.units.iter().map(|unit| unit.evidence_id.clone()).collect(), provenance: CodeProvenance::EditionVerified },
        person_text,
        server_suffixes,
        prepared_question_sha256,
        limits,
        reviewed_facts,
        fixed_contract_sha256: json_hash(&fixed)?,
        project_instructions_sha256: project_written.map(json_hash).transpose()?,
        fields: group_fields(fields),
    })
}

/// A provenance as it was recorded is the one these inputs give, part by part.
pub fn check_provenance(recorded: &QueryProvenance, input: &ProvenanceInput) -> Result<(), CompositionError> {
    let now = build_provenance(input)?;
    let part = if recorded.sha256 != now.sha256 {
        "sha256"
    } else if recorded.code != now.code {
        "code"
    } else if recorded.person_text != now.person_text {
        "person_text"
    } else if recorded.server_suffixes != now.server_suffixes {
        "server_suffixes"
    } else if recorded.prepared_question_sha256 != now.prepared_question_sha256 {
        "prepared_question_sha256"
    } else if recorded.limits != now.limits {
        "limits"
    } else if recorded.reviewed_facts != now.reviewed_facts {
        "reviewed_facts"
    } else if recorded.fixed_contract_sha256 != now.fixed_contract_sha256 {
        "fixed_contract_sha256"
    } else if recorded.project_instructions_sha256 != now.project_instructions_sha256 {
        "project_instructions_sha256"
    } else if recorded.fields != now.fields {
        "fields"
    } else {
        return Ok(());
    };
    Err(Violation::ProvenanceDiffers { part: part.to_owned() }.into())
}

// The record of a suggestion: what the server used and what it answered, kept so that a selection can
// later be checked against it and not against what a browser says. It is addressed by the hash of
// its own bytes, so it holds no hash of itself and nothing that changes between two equal answers.
// It says nothing of whether the suggestion was good, and it knows no question.

/// The record's own version; it says nothing of a package's or a selection record's.
pub const SUGGESTION_RECORD_VERSION: u32 = 1;

/// `kind`: names the artifact, so that its `format_version` is not taken for another artifact's.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionRecordKind {
    SuggestionRecord,
}

/// `format_version` of a suggestion record: 1, and no other number reads as one.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "u32", into = "u32")]
pub struct SuggestionRecordVersion;

impl TryFrom<u32> for SuggestionRecordVersion {
    type Error = String;
    fn try_from(version: u32) -> Result<Self, String> {
        if version == SUGGESTION_RECORD_VERSION { Ok(Self) } else { Err(format!("suggestion record format {version} is not known")) }
    }
}

impl From<SuggestionRecordVersion> for u32 {
    fn from(_: SuggestionRecordVersion) -> u32 {
        SUGGESTION_RECORD_VERSION
    }
}

/// What the selection was asked for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SuggestionInput {
    pub nodes: Vec<String>,
    pub budget_bytes: usize,
    pub max_items: usize,
}

/// Where a rule of the inventory came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InventoryRuleOrigin {
    Builtin,
    Project,
}

/// A rule as the inventory applied it. Its form is the inventory's, which reads it back.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InventoryRuleEntry {
    pub origin: InventoryRuleOrigin,
    pub rule: serde_json::Value,
}

/// The rules themselves, not only their hash: a hash alone would not bring them back.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SuggestionInventoryRules {
    pub version: String,
    /// Of the compact JSON of `rules`.
    pub sha256: Sha256Hex,
    pub rules: Vec<InventoryRuleEntry>,
}

impl SuggestionInventoryRules {
    pub fn new(version: &str, rules: Vec<InventoryRuleEntry>) -> Result<Self, String> {
        let text = serde_json::to_string(&rules).map_err(|error| error.to_string())?;
        Ok(Self { version: version.to_owned(), sha256: sha256_of(text.as_bytes()), rules })
    }
}

/// The measure the selection used. `ok` names the execution it was read from; the other two have no
/// execution to name, and neither says anything of how recent a measure is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "read_status", rename_all = "snake_case", deny_unknown_fields)]
pub enum SuggestionMeasure {
    Ok { execution: String, execution_sha256: Sha256Hex },
    /// There was no measure to read.
    #[serde(rename = "none")]
    NoMeasure {},
    /// There was something to read, and it could not be used.
    Unreadable {},
}

/// What the suggestion answered, as it was served. Its lists keep the form the selection gives them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SuggestionOutput {
    pub items: Vec<serde_json::Value>,
    pub left_out: Vec<serde_json::Value>,
    pub gaps: Vec<serde_json::Value>,
    pub behavior: Vec<serde_json::Value>,
    pub measured: bool,
    pub used_bytes: usize,
    /// Null with no measure.
    pub coverage: serde_json::Value,
    pub limits: Vec<String>,
}

/// One suggestion, as the server made it. Every field is required, and a field not named here is
/// refused.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SuggestionRecord {
    pub kind: SuggestionRecordKind,
    pub format_version: SuggestionRecordVersion,
    pub edition_origin: String,
    pub rules_version: RulesVersion,
    pub input: SuggestionInput,
    pub inventory_rules: SuggestionInventoryRules,
    pub measure: SuggestionMeasure,
    pub output: SuggestionOutput,
}

/// Why a stored record cannot be taken for the one its name says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestionRecordError {
    /// The bytes are not the ones the hash names.
    HashDiffers { expected: String, found: String },
    /// The bytes are not a record of this format.
    Malformed(String),
    /// A record of this format, not written the way a record is written.
    NotInItsOwnBytes,
    /// The rules are not the ones their hash names.
    RulesHashDiffers,
}

impl std::fmt::Display for SuggestionRecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HashDiffers { expected, found } => write!(f, "suggestion record {expected} holds bytes of {found}"),
            Self::Malformed(detail) => write!(f, "malformed suggestion record: {detail}"),
            Self::NotInItsOwnBytes => write!(f, "suggestion record not in the bytes a record is written in"),
            Self::RulesHashDiffers => write!(f, "suggestion record whose rules are not the ones their hash names"),
        }
    }
}

impl std::error::Error for SuggestionRecordError {}

impl SuggestionRecord {
    /// A record from what the route answers. `notes` goes to the model and to no record, and the
    /// budget is the input's; anything else the answer had and this record does not name is refused,
    /// so that nothing served is left out unnoticed.
    pub fn new(edition_origin: String, rules_version: &str, input: SuggestionInput, inventory_rules: SuggestionInventoryRules, measure: SuggestionMeasure, answer: &serde_json::Value) -> Result<Self, String> {
        let mut output = answer.clone();
        let fields = output.as_object_mut().ok_or("the answer is not an object")?;
        fields.remove("notes");
        let budget = fields.remove("budget_bytes");
        if budget.as_ref().and_then(serde_json::Value::as_u64) != Some(input.budget_bytes as u64) {
            return Err(format!("the answer's budget {budget:?} is not the input's {}", input.budget_bytes));
        }
        let output = serde_json::from_value(output).map_err(|error| error.to_string())?;
        Ok(Self {
            kind: SuggestionRecordKind::SuggestionRecord,
            format_version: SuggestionRecordVersion,
            edition_origin,
            rules_version: RulesVersion(rules_version.to_owned()),
            input,
            inventory_rules,
            measure,
            output,
        })
    }

    /// The bytes of the record: the same serialization the project's other artifacts use. Fields go
    /// in the order written here and an object's keys in sorted order, whatever order they came in.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let mut bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// The record these bytes are, if they are the ones `expected` names and hold what they say.
    pub fn from_bytes(bytes: &[u8], expected: &str) -> Result<Self, SuggestionRecordError> {
        let found = crate::hash(bytes);
        if found != expected {
            return Err(SuggestionRecordError::HashDiffers { expected: expected.to_owned(), found });
        }
        let record: Self = serde_json::from_slice(bytes).map_err(|error| SuggestionRecordError::Malformed(error.to_string()))?;
        if record.to_bytes().map_err(SuggestionRecordError::Malformed)? != bytes {
            return Err(SuggestionRecordError::NotInItsOwnBytes);
        }
        let rules = SuggestionInventoryRules::new(&record.inventory_rules.version, record.inventory_rules.rules.clone()).map_err(SuggestionRecordError::Malformed)?;
        if rules.sha256 != record.inventory_rules.sha256 {
            return Err(SuggestionRecordError::RulesHashDiffers);
        }
        Ok(record)
    }
}

// The selection, format 2: what a client says it gathered for a question, out of which suggestions,
// and what a person did with each source. The server takes it as a statement and checks it against
// the suggestion records it wrote and against the edition: every source in exactly one list, each
// one's data the data the cited suggestions gave, absorbed in the order the client gave them, the
// way the page absorbs them. What it cannot check (that a person clicked, which decision came first,
// when a source was added by hand) stays the client's, and the record says so. A check proves
// consistency between those statements, the records and the edition, never a human action.

/// `kind` of a selection, as the client sends it and as the record is kept.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectionRecordKind {
    SelectionRecord,
}

/// Why the selection kept a source out, as `envelope::Entry::left_out` names it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LeftOutWhy {
    DidNotRun,
    Test,
    Budget,
    MaxItems,
}

/// How a source first came into the selection, as the page keeps it. `manual` is the client's word.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceProvenance {
    Manual,
    Suggested,
    LeftOut,
}

/// The last action the client says a person took on a source.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Included,
    Removed,
}

/// Whether that action was over the one source or over many at once.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionScope {
    Single,
    Bulk,
}

/// A reason as a suggestion gives it; empty `fields` and `lines` are written as absent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectionReason {
    pub code: ReasonCode,
    pub for_target: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<usize>,
}

/// A gap as a suggestion gives it (`envelope::Gap`): what is empty is written as absent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectionGap {
    pub code: String,
    pub for_target: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

/// One source of the selection. Every field is required, and a null is written for what a source
/// added by hand and never suggested does not have. `why` is only in `left_out`, and only there.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SelectionEntry {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub path: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub kind: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub start_line: Option<usize>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub end_line: Option<usize>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub role: Option<FichaRole>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub bytes: Option<usize>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub calls: Option<f64>,
    pub reasons: Vec<SelectionReason>,
    pub facts: Vec<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub left_out_why: Option<LeftOutWhy>,
    pub provenance: SourceProvenance,
    #[serde(deserialize_with = "Option::deserialize")]
    pub decision: Option<Decision>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub decision_scope: Option<DecisionScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<LeftOutWhy>,
}

/// A selection as the client sends it in `origin.selection`. Every field is required; a field not
/// named here is refused, `notes` among them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SelectionInput {
    pub kind: SelectionRecordKind,
    pub format_version: SelectionRecordFormat,
    pub suggestions: Vec<Sha256Hex>,
    pub seeds: Vec<String>,
    pub suggested_from: Vec<String>,
    pub measured: bool,
    pub budget_bytes: usize,
    pub suggested_bytes: usize,
    pub included_bytes: usize,
    pub included: Vec<SelectionEntry>,
    pub removed: Vec<SelectionEntry>,
    pub left_out: Vec<SelectionEntry>,
    pub not_joined: Vec<SelectionEntry>,
    pub gaps: Vec<SelectionGap>,
}

/// Why a selection is not accepted. Each names what it is about; none says a person did anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionRefusal {
    /// Not the contract: kind, version, a field missing, unknown or of the wrong type, a code not known.
    Shape(String),
    /// `included` is not the question's nodes, in the same order.
    NotNodes,
    /// A source twice, in one list or in two.
    Duplicate { id: String },
    /// A source in a list it cannot be in, or a suggested source in none.
    Partition { id: String, why: &'static str },
    /// A seed that is not among the included sources, or named twice.
    Seed { id: String },
    /// The edition has no such source, or not with these data.
    Edition { id: String, what: &'static str },
    /// A cited suggestion that cannot be used.
    Suggestion { sha256: String, why: String },
    /// What the cited suggestions give, absorbed in the order given, is not what was sent.
    Differs { id: String, what: &'static str },
    /// A statement of the client that its own data or the suggestions contradict.
    Declaration { id: String, what: &'static str },
    /// `included_bytes` is not the sum of the included sources' bytes.
    Bytes,
}

impl std::fmt::Display for SelectionRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shape(detail) => write!(f, "forma: {detail}"),
            Self::NotNodes => write!(f, "included não é a lista de nodes da pergunta, na mesma ordem"),
            Self::Duplicate { id } => write!(f, "{id} aparece mais de uma vez"),
            Self::Partition { id, why } => write!(f, "partição: {id} {why}"),
            Self::Seed { id } => write!(f, "semente {id} fora de included ou repetida"),
            Self::Edition { id, what } => write!(f, "edition: {id}: {what}"),
            Self::Suggestion { sha256, why } => write!(f, "sugestão {sha256}: {why}"),
            Self::Differs { id, what } => write!(f, "{id}: {what} difere do que as sugestões dão"),
            Self::Declaration { id, what } => write!(f, "declaração: {id}: {what}"),
            Self::Bytes => write!(f, "included_bytes não é a soma dos bytes incluídos"),
        }
    }
}

impl std::error::Error for SelectionRefusal {}

impl SelectionInput {
    /// The selection a client sent, if it has the contract's form. `why` is only read in `left_out`:
    /// anywhere else it is refused, and there it may not be null.
    pub fn from_value(value: &serde_json::Value) -> Result<Self, SelectionRefusal> {
        let fields = value.as_object().ok_or_else(|| SelectionRefusal::Shape("a seleção não é um objeto".into()))?;
        if fields.get("kind") != Some(&serde_json::json!("selection_record")) {
            return Err(SelectionRefusal::Shape("kind não é selection_record".into()));
        }
        if fields.get("format_version") != Some(&serde_json::json!(SELECTION_RECORD_FORMAT)) {
            return Err(SelectionRefusal::Shape(format!("format_version não é {SELECTION_RECORD_FORMAT}")));
        }
        for list in ["included", "removed", "not_joined", "left_out"] {
            for entry in fields.get(list).and_then(serde_json::Value::as_array).into_iter().flatten() {
                let why = entry.get("why");
                let fits = if list == "left_out" { why.is_some_and(|w| !w.is_null()) } else { why.is_none() };
                if !fits {
                    return Err(SelectionRefusal::Shape(format!("why fora do lugar em {list}")));
                }
            }
        }
        serde_json::from_value(value.clone()).map_err(|error| SelectionRefusal::Shape(error.to_string()))
    }

    fn lists(&self) -> [(&'static str, &[SelectionEntry]); 4] {
        [("included", &self.included), ("removed", &self.removed), ("left_out", &self.left_out), ("not_joined", &self.not_joined)]
    }
}

/// A source as the edition has it: what a suggestion of this edition would say of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditionSource {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub bytes: usize,
}

/// What the edition says of the sources a selection names: `sources` from its catalog (declarations
/// and file heads), `labels` from its graph (label and kind of a node), which is what the page shows
/// of a source added by hand. A source the edition does not have is in neither.
#[derive(Debug, Clone, Default)]
pub struct SelectionEdition {
    pub sources: std::collections::BTreeMap<String, EditionSource>,
    pub labels: std::collections::BTreeMap<String, (String, String)>,
}

/// An item of a suggestion as it was served: the fields the page reads of it, the rest left aside.
#[derive(Debug, Clone, Deserialize)]
struct Served {
    id: String,
    path: String,
    name: String,
    kind: String,
    start_line: usize,
    end_line: usize,
    reasons: Vec<SelectionReason>,
    role: FichaRole,
    bytes: usize,
    #[serde(default)]
    calls: Option<f64>,
    #[serde(default)]
    left_out: Option<LeftOutWhy>,
    #[serde(default)]
    facts: Vec<String>,
}

/// What the page knows of a source after absorbing the suggestions, in its own words: the fields of
/// the latest mention, the reasons of every one, the first `left_out` given and the latest.
struct Absorbed {
    entry: SelectionEntry,
    kept_out: Option<LeftOutWhy>,
    first: SourceProvenance,
    /// The suggestion of the first mention, and the place in it: the order the page learnt it in.
    at: (usize, usize),
}

/// Each source the suggestions mentioned, in the order first mentioned, with what the page knows of it.
type Known = Vec<(String, Absorbed)>;

/// The suggestions a selection cites, absorbed in the order given, as the page absorbs them: the
/// items of each, then what it left out; the gaps of each that are not there yet.
fn absorb(suggestions: &[&SuggestionRecord]) -> Result<(Known, Vec<SelectionGap>), SelectionRefusal> {
    let mut known: Vec<(String, Absorbed)> = vec![];
    let mut gaps: Vec<SelectionGap> = vec![];
    for (n, record) in suggestions.iter().enumerate() {
        let parse = |value: &serde_json::Value| serde_json::from_value::<Served>(value.clone()).map_err(|e| SelectionRefusal::Shape(format!("sugestão citada ilegível: {e}")));
        let mentions = record.output.items.iter().map(|v| (v, SourceProvenance::Suggested)).chain(record.output.left_out.iter().map(|v| (v, SourceProvenance::LeftOut)));
        for (pos, (value, provenance)) in mentions.enumerate() {
            let served = parse(value)?;
            let before = known.iter().position(|(id, _)| *id == served.id);
            let earlier = before.map(|i| &known[i].1);
            let mut reasons = earlier.map(|a| a.entry.reasons.clone()).unwrap_or_default();
            let new: Vec<SelectionReason> = served.reasons.iter().filter(|r| !reasons.contains(r)).cloned().collect();
            reasons.extend(new);
            let entry = SelectionEntry {
                id: served.id.clone(), name: served.name, path: Some(served.path), kind: Some(served.kind),
                start_line: Some(served.start_line), end_line: Some(served.end_line), role: Some(served.role), bytes: Some(served.bytes),
                calls: served.calls, reasons, facts: served.facts,
                left_out_why: earlier.and_then(|a| a.entry.left_out_why).or(served.left_out),
                provenance: earlier.map_or(provenance, |a| a.first), decision: None, decision_scope: None, why: None,
            };
            let absorbed = Absorbed { entry, kept_out: served.left_out, first: earlier.map_or(provenance, |a| a.first), at: earlier.map_or((n, pos), |a| a.at) };
            match before {
                Some(i) => known[i].1 = absorbed,
                None => known.push((served.id, absorbed)),
            }
        }
        // As the page does: what is not among the gaps known before this suggestion, all of it.
        let served: Vec<SelectionGap> = record.output.gaps.iter().map(|value| serde_json::from_value(value.clone()))
            .collect::<Result<_, _>>().map_err(|e| SelectionRefusal::Shape(format!("lacuna citada ilegível: {e}")))?;
        let new: Vec<SelectionGap> = served.into_iter().filter(|g| !gaps.contains(g)).collect();
        gaps.extend(new);
    }
    Ok((known, gaps))
}

/// Checks a selection a client sent with a question over `nodes`, against the suggestion records
/// it cites (already read and checked, in the order cited) and the edition. Nothing is written.
pub fn check_selection(input: &SelectionInput, nodes: &[String], suggestions: &[&SuggestionRecord], edition: &SelectionEdition) -> Result<(), SelectionRefusal> {
    use SelectionRefusal as R;
    if input.included.iter().map(|e| &e.id).ne(nodes.iter()) {
        return Err(R::NotNodes);
    }
    let mut seen: Vec<&str> = vec![];
    for (_, list) in input.lists() {
        for entry in list {
            if seen.contains(&entry.id.as_str()) {
                return Err(R::Duplicate { id: entry.id.clone() });
            }
            seen.push(&entry.id);
        }
    }
    for (n, seed) in input.seeds.iter().enumerate() {
        if input.seeds[..n].contains(seed) || !nodes.contains(seed) {
            return Err(R::Seed { id: seed.clone() });
        }
    }
    // Each source is one the edition has; a suggested one, with the edition's data.
    for (_, list) in input.lists() {
        for e in list {
            let source = edition.sources.get(&e.id).ok_or_else(|| R::Edition { id: e.id.clone(), what: "não existe nesta edition" })?;
            if e.path.is_none() {
                continue;
            }
            let checks: [(&'static str, bool); 5] = [
                ("caminho", e.path.as_deref() == Some(source.path.as_str())),
                ("linhas", (e.start_line, e.end_line) == (Some(source.start_line), Some(source.end_line))),
                ("kind", e.kind.as_deref() == Some(source.kind.as_str())),
                ("nome", e.name == source.name),
                ("bytes", e.bytes == Some(source.bytes)),
            ];
            if let Some((what, _)) = checks.iter().find(|(_, same)| !same) {
                return Err(R::Edition { id: e.id.clone(), what });
            }
        }
    }
    let last = suggestions.last().ok_or_else(|| R::Shape("nenhuma sugestão citada".into()))?;
    if suggestions.len() != input.suggestions.len() {
        return Err(R::Shape("sugestões citadas e lidas não conferem".into()));
    }
    let (known, gaps) = absorb(suggestions)?;
    let absorbed = |id: &str| known.iter().find(|(k, _)| k == id).map(|(_, a)| a);
    for (list, entries) in input.lists() {
        for e in entries {
            let id = || e.id.clone();
            if e.decision.is_some() != e.decision_scope.is_some() {
                return Err(R::Declaration { id: id(), what: "decision e decision_scope precisam ser ambos nulos ou ambos preenchidos" });
            }
            // A source added by hand exists only through an action: it always has one.
            if e.provenance == SourceProvenance::Manual && e.decision.is_none() {
                return Err(R::Declaration { id: id(), what: "fonte manual sem decisão" });
            }
            let kept_out = match absorbed(&e.id) {
                Some(a) => {
                    let x = &a.entry;
                    let fields: [(&'static str, bool); 11] = [
                        ("name", e.name == x.name), ("path", e.path == x.path), ("kind", e.kind == x.kind),
                        ("start_line", e.start_line == x.start_line), ("end_line", e.end_line == x.end_line),
                        ("role", e.role == x.role), ("bytes", e.bytes == x.bytes), ("calls", e.calls == x.calls),
                        ("reasons", e.reasons == x.reasons), ("facts", e.facts == x.facts), ("left_out_why", e.left_out_why == x.left_out_why),
                    ];
                    if let Some((what, _)) = fields.iter().find(|(_, same)| !same) {
                        return Err(R::Differs { id: id(), what });
                    }
                    if e.provenance != SourceProvenance::Manual && e.provenance != a.first {
                        return Err(R::Differs { id: id(), what: "provenance" });
                    }
                    a.kept_out
                }
                None => {
                    // Never suggested: added by hand, with what the page shows of the edition's graph.
                    if e.provenance != SourceProvenance::Manual {
                        return Err(R::Declaration { id: id(), what: "nenhuma sugestão citada menciona esta fonte" });
                    }
                    let (name, kind) = edition.labels.get(&e.id).map_or((e.id.clone(), None), |(l, k)| (l.clone(), Some(k.clone())));
                    let bare = e.name == name && e.kind == kind && e.path.is_none() && e.start_line.is_none() && e.end_line.is_none() && e.role.is_none()
                        && e.bytes.is_none() && e.calls.is_none() && e.reasons.is_empty() && e.facts.is_empty() && e.left_out_why.is_none();
                    if !bare {
                        return Err(R::Differs { id: id(), what: "dados de uma fonte manual nunca sugerida" });
                    }
                    None
                }
            };
            let fits = match list {
                "included" => e.decision != Some(Decision::Removed),
                "removed" => e.decision == Some(Decision::Removed),
                "left_out" => e.decision.is_none() && kept_out.is_some() && e.why == kept_out,
                _ => e.decision.is_none() && kept_out.is_none(),
            };
            if !fits {
                return Err(R::Partition { id: id(), why: match list {
                    "included" => "está em included com decisão de remoção",
                    "removed" => "está em removed sem decisão de remoção",
                    "left_out" => "está em left_out sem ser o que a última sugestão deixou de fora, ou com decisão",
                    _ => "está em not_joined sendo deixada de fora pela última sugestão, ou com decisão",
                } });
            }
        }
        // The other lists keep the order the page learnt each source in: by the first suggestion
        // that mentioned it. One added by hand came before any suggestion that mentions it.
        if list != "included" {
            for (n, e) in entries.iter().enumerate() {
                let Some(a) = absorbed(&e.id) else { continue };
                for before in &entries[..n] {
                    let Some(b) = absorbed(&before.id) else { continue };
                    if before.provenance == SourceProvenance::Manual {
                        continue;
                    }
                    let contradicts = match e.provenance {
                        SourceProvenance::Manual => b.at.0 >= a.at.0,
                        _ => b.at > a.at,
                    };
                    if contradicts {
                        return Err(R::Declaration { id: e.id.clone(), what: match e.provenance {
                            SourceProvenance::Manual => "manual, mas uma sugestão anterior já a mencionava",
                            _ => "fora da ordem em que as sugestões a trouxeram",
                        } });
                    }
                }
            }
        }
    }
    for (id, _) in &known {
        if !seen.contains(&id.as_str()) {
            return Err(R::Partition { id: id.clone(), why: "é mencionada por uma sugestão citada e não está em nenhuma lista" });
        }
    }
    if input.gaps != gaps {
        return Err(R::Differs { id: "gaps".into(), what: "a união das lacunas" });
    }
    let summary: [(&'static str, bool); 4] = [
        ("suggested_from", input.suggested_from == last.input.nodes),
        ("measured", input.measured == last.output.measured),
        ("budget_bytes", input.budget_bytes == last.input.budget_bytes),
        ("suggested_bytes", input.suggested_bytes == last.output.used_bytes),
    ];
    if let Some((what, _)) = summary.iter().find(|(_, same)| !same) {
        return Err(R::Differs { id: "última sugestão".into(), what });
    }
    if input.included_bytes != input.included.iter().map(|e| e.bytes.unwrap_or(0)).sum::<usize>() {
        return Err(R::Bytes);
    }
    Ok(())
}

/// What the server checked of a selection, each by name, in the order it checks them.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectionCheck {
    Shape,
    IncludedIsNodes,
    Partition,
    SeedsIncluded,
    Edition,
    Suggestions,
    Absorption,
    IncludedBytes,
}

pub const SELECTION_CHECKS: [SelectionCheck; 8] = [
    SelectionCheck::Shape, SelectionCheck::IncludedIsNodes, SelectionCheck::Partition, SelectionCheck::SeedsIncluded,
    SelectionCheck::Edition, SelectionCheck::Suggestions, SelectionCheck::Absorption, SelectionCheck::IncludedBytes,
];

/// A cited suggestion, as the server read and checked it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CheckedSuggestion {
    pub sha256: Sha256Hex,
    pub measure: SuggestionMeasure,
}

/// What the server checked, and what it computed itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectionChecked {
    pub declared_by: ServerDeclared,
    pub checks: Vec<SelectionCheck>,
    /// In the order the client cited them.
    pub suggestions: Vec<CheckedSuggestion>,
    /// The sum of the included sources' bytes, each the edition's.
    pub included_bytes: usize,
}

/// An action the client says a person took on a source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeclaredDecision {
    pub id: String,
    pub decision: Decision,
    pub decision_scope: DecisionScope,
}

/// What only the client says: checked for coherence, not for truth.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectionDeclared {
    pub declared_by: ClientDeclared,
    /// The order the suggestions were absorbed in.
    pub suggestion_order: Vec<Sha256Hex>,
    /// What the question is about.
    pub seeds: Vec<String>,
    pub decisions: Vec<DeclaredDecision>,
    /// Sources said to have been added by hand.
    pub manual: Vec<String>,
}

/// The record of a selection a question came with: what the client sent, as accepted, and apart
/// from it what the server checked and what stays the client's word. Addressed by the hash of its
/// bytes: it holds no hash of itself, no clock and nothing that changes between two equal inputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SelectionRecord {
    pub kind: SelectionRecordKind,
    pub format_version: SelectionRecordFormat,
    pub edition_origin: String,
    pub rules_version: RulesVersion,
    pub checked_by_server: SelectionChecked,
    pub declared_by_client: SelectionDeclared,
    pub received: SelectionInput,
}

impl SelectionRecord {
    /// The record of a selection `check_selection` accepted, with the suggestions it cites.
    pub fn new(edition_origin: String, rules_version: &str, received: SelectionInput, suggestions: &[&SuggestionRecord]) -> Self {
        let checked = received.suggestions.iter().zip(suggestions).map(|(sha256, record)| CheckedSuggestion { sha256: sha256.clone(), measure: record.measure.clone() }).collect();
        let decisions = received.lists().into_iter().flat_map(|(_, list)| list.iter())
            .filter_map(|e| Some(DeclaredDecision { id: e.id.clone(), decision: e.decision?, decision_scope: e.decision_scope? })).collect();
        let manual = received.lists().into_iter().flat_map(|(_, list)| list.iter()).filter(|e| e.provenance == SourceProvenance::Manual).map(|e| e.id.clone()).collect();
        Self {
            kind: SelectionRecordKind::SelectionRecord,
            format_version: SelectionRecordFormat,
            edition_origin,
            rules_version: RulesVersion(rules_version.to_owned()),
            checked_by_server: SelectionChecked {
                declared_by: ServerDeclared::Server,
                checks: SELECTION_CHECKS.to_vec(),
                suggestions: checked,
                included_bytes: received.included.iter().map(|e| e.bytes.unwrap_or(0)).sum(),
            },
            declared_by_client: SelectionDeclared {
                declared_by: ClientDeclared::Client,
                suggestion_order: received.suggestions.clone(),
                seeds: received.seeds.clone(),
                decisions,
                manual,
            },
            received,
        }
    }

    /// The bytes of the record, written as a suggestion's record is.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let mut bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// The record these bytes are, if they are the ones `expected` names and written as a record is.
    pub fn from_bytes(bytes: &[u8], expected: &str) -> Result<Self, String> {
        let found = crate::hash(bytes);
        if found != expected {
            return Err(format!("o registro {expected} tem os bytes de {found}"));
        }
        let record: Self = serde_json::from_slice(bytes).map_err(|error| format!("registro ilegível ({error})"))?;
        if record.to_bytes()? != bytes {
            return Err("registro fora da forma em que é gravado".into());
        }
        Ok(record)
    }

    /// The suggestions cited, in the client's order.
    pub fn suggestions(&self) -> Vec<Sha256Hex> {
        self.checked_by_server.suggestions.iter().map(|s| s.sha256.clone()).collect()
    }

    /// The measure the selection rested on: the last cited suggestion's, which gave its budget.
    pub fn selection_basis(&self) -> MeasureSelectionBasis {
        match self.checked_by_server.suggestions.last() {
            Some(CheckedSuggestion { sha256, measure }) => match measure {
                SuggestionMeasure::Ok { execution, execution_sha256 } => MeasureSelectionBasis::Ok { suggestion: sha256.clone(), execution: execution.clone(), execution_sha256: execution_sha256.clone() },
                SuggestionMeasure::NoMeasure {} => MeasureSelectionBasis::NoMeasure { suggestion: sha256.clone() },
                SuggestionMeasure::Unreadable {} => MeasureSelectionBasis::Unreadable { suggestion: sha256.clone() },
            },
            None => MeasureSelectionBasis::NotRecorded {},
        }
    }
}

// H4, reproduction: each suggestion a question's selection cites, made again by the current code from
// what its record names (the edition, the input, the inventory's rules, the execution it read) and
// compared with the record. It reproduces the suggestion, never the client's decisions. It is told,
// not enforced: a question is sent on H1 to H3, whatever H4 says. It never runs older code: a record
// of other rules is told so, with the code the question was made with. The facts of each item come
// from the acervo of the moment and the record does not keep that acervo: they are taken from the
// record as given, left out of the comparison, and said not to be reproduced. H4 proves nothing of
// those facts' content now, nor that they are still the same as when the suggestion was made.

/// Why a cited suggestion cannot be made again as recorded.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotReplayable {
    /// No record has the bytes the hash names in this run.
    SuggestionMissing,
    /// The execution the record read is not there any more.
    ExecutionMissing,
    /// The execution is there, with other bytes than the record names.
    ExecutionDiffers,
    /// The record is of other selection or inventory rules than this code's.
    RulesVersionDiffers,
    /// Same rules, and the suggestion made again is not the recorded one.
    OutputDiffers,
}

/// One cited suggestion, made again or not, with why.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum SuggestionReplay {
    Reproduced { suggestion: Sha256Hex },
    NotReplayable { suggestion: Sha256Hex, reason: NotReplayable },
}

/// What a replay did with each item's facts: the recorded ids, reused as given, never made again.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactsInReplay {
    RecordedNotReproduced,
}

/// H4 of a question. `not_applicable` when no suggestion is behind it (no package, or no selection).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reproduction {
    NotApplicable {},
    /// Every cited suggestion made again, the same but for the facts.
    Reproduced { suggestions: Vec<SuggestionReplay>, facts: FactsInReplay },
    /// At least one was not. `builder_at_question` is the code the question was made with, which is
    /// not always the one the suggestion was made with; `code_recoverable` says whether it names a
    /// commit with nothing uncommitted.
    NotReplayable { suggestions: Vec<SuggestionReplay>, facts: FactsInReplay, builder_at_question: BuilderIdentity, code_recoverable: bool },
}

impl Reproduction {
    /// The reproduction of these suggestions, in the order cited.
    pub fn of(suggestions: Vec<SuggestionReplay>, builder: &BuilderIdentity) -> Self {
        let all = suggestions.iter().all(|s| matches!(s, SuggestionReplay::Reproduced { .. }));
        match all {
            true => Self::Reproduced { suggestions, facts: FactsInReplay::RecordedNotReproduced },
            false => Self::NotReplayable {
                suggestions, facts: FactsInReplay::RecordedNotReproduced, builder_at_question: builder.clone(),
                code_recoverable: matches!(builder, BuilderIdentity::Recorded { git_dirty: false, .. }),
            },
        }
    }
}

/// Two records of one suggestion are the same but for each item's `facts`: compared byte for byte
/// with that field, and only that field, taken out of every item and of what was left out.
pub fn same_but_facts(recorded: &SuggestionRecord, again: &SuggestionRecord) -> bool {
    let bare = |record: &SuggestionRecord| {
        let mut record = record.clone();
        for item in record.output.items.iter_mut().chain(record.output.left_out.iter_mut()) {
            if let Some(fields) = item.as_object_mut() {
                fields.remove("facts");
            }
        }
        record.to_bytes()
    };
    matches!((bare(recorded), bare(again)), (Ok(a), Ok(b)) if a == b)
}

// Checks of a stored package, before what it describes is used. Each one names the guarantee it is
// part of: H1, that what was recorded agrees with itself; H2, that its units come again from the
// snapshots by `lines-v1`. Neither reads the analyzed project, its commit or the selection rules,
// and neither writes anything. H3, the query made again by the current code, belongs to the caller,
// which has that code.

/// Why a stored package is not taken as what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageCheck {
    /// H1: two units with one id, two fichas with one reference.
    Duplicate { what: &'static str, id: String },
    /// H1: a reference to a ficha or a unit the package does not have.
    UnknownReference { at: &'static str, id: String },
    /// H1: a unit no ficha points to, or the code class not naming the units.
    Unlinked { what: &'static str, id: String },
    /// H1: line numbers or hashes that cannot be, for this representation.
    Shape { what: &'static str, id: String },
    /// H2: a unit's source has no snapshot among the inputs.
    MissingSnapshot { path: String },
    /// H2: the snapshot is not the bytes the unit names.
    SourceDiffers { unit: String },
    /// H2: the unit's text does not come again from the snapshot by `lines-v1`.
    TextDiffers { unit: String },
    /// H2: the window has no text by `lines-v1` (outside the file, not UTF-8, empty after N4).
    Normalization { unit: String, error: NormalizationError },
    /// H1: a ficha outside its unit's window or in another source.
    FichaOutside { ficha: String },
}

impl std::fmt::Display for PackageCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PackageCheck {}

/// H1: the package agrees with itself. Every reference names something it has, nothing is there
/// twice, every unit has a ficha, and the numbers and hashes fit the representation.
pub fn check_structure(package: &EvidencePackage) -> Result<(), PackageCheck> {
    let claims = package.representation == Representation::Claims;
    for (n, unit) in package.units.iter().enumerate() {
        let id = &unit.evidence_id.0;
        if package.units[..n].iter().any(|u| u.evidence_id == unit.evidence_id) {
            return Err(PackageCheck::Duplicate { what: "unit", id: id.clone() });
        }
        let fits = 1 <= unit.window_start_line
            && unit.window_start_line == unit.start_line
            && unit.start_line <= unit.end_line
            && unit.end_line <= unit.window_end_line
            && (claims || unit.end_line == unit.window_end_line);
        if !fits {
            return Err(PackageCheck::Shape { what: "unit lines", id: id.clone() });
        }
        if unit.excerpt_sha256.is_some() != claims {
            return Err(PackageCheck::Shape { what: "unit excerpt", id: id.clone() });
        }
        if !package.fichas.iter().any(|f| f.unit == unit.evidence_id) {
            return Err(PackageCheck::Unlinked { what: "unit", id: id.clone() });
        }
    }
    for (n, ficha) in package.fichas.iter().enumerate() {
        let id = &ficha.reference.0;
        if package.fichas[..n].iter().any(|f| f.reference == ficha.reference) {
            return Err(PackageCheck::Duplicate { what: "ficha", id: id.clone() });
        }
        if !package.units.iter().any(|u| u.evidence_id == ficha.unit) {
            return Err(PackageCheck::UnknownReference { at: "ficha unit", id: id.clone() });
        }
        if ficha.start_line == 0 || ficha.end_line < ficha.start_line {
            return Err(PackageCheck::Shape { what: "ficha lines", id: id.clone() });
        }
        // A ficha lies inside its unit's window, in the same source.
        let unit = package.units.iter().find(|u| u.evidence_id == ficha.unit).expect("checked above");
        if ficha.path != unit.path || ficha.start_line < unit.window_start_line || unit.window_end_line < ficha.end_line {
            return Err(PackageCheck::FichaOutside { ficha: id.clone() });
        }
    }
    let ids: Vec<&UnitId> = package.units.iter().map(|u| &u.evidence_id).collect();
    if package.query.code.units.iter().collect::<Vec<_>>() != ids {
        return Err(PackageCheck::Unlinked { what: "code class", id: String::new() });
    }
    for fact in &package.query.reviewed_facts {
        let ficha = package.fichas.iter().find(|f| f.reference == fact.ficha).ok_or_else(|| PackageCheck::UnknownReference { at: "reviewed fact ficha", id: fact.fact_id.clone() })?;
        if ficha.unit != fact.unit || ficha.kind != FichaKind::ReviewedFact || fact.ficha.0 != fact.fact_id {
            return Err(PackageCheck::UnknownReference { at: "reviewed fact unit", id: fact.fact_id.clone() });
        }
    }
    for group in &package.query.fields {
        for FieldPlace(ficha, unit, _) in &group.places {
            if let Some(ficha) = ficha {
                // A place may name a reviewed fact by its id; it is a ficha all the same.
                if !package.fichas.iter().any(|f| &f.reference == ficha) {
                    return Err(PackageCheck::UnknownReference { at: "field ficha", id: ficha.0.clone() });
                }
            }
            if let Some(unit) = unit {
                if !package.units.iter().any(|u| &u.evidence_id == unit) {
                    return Err(PackageCheck::UnknownReference { at: "field unit", id: unit.0.clone() });
                }
            }
        }
    }
    // What a lacuna round added: each declaration once, with what made it, and its ficha the server's.
    for (n, addition) in package.server_additions.iter().enumerate() {
        let id = &addition.symbol_id;
        if package.server_additions[..n].iter().any(|a| a.symbol_id == *id) {
            return Err(PackageCheck::Duplicate { what: "server addition", id: id.clone() });
        }
        if addition.round == 0 || addition.for_name.is_empty() || addition.from_turn.is_empty() || addition.from_send.is_empty() {
            return Err(PackageCheck::Shape { what: "server addition", id: id.clone() });
        }
        if !package.fichas.iter().any(|f| f.symbol_id.as_ref() == Some(id)) {
            return Err(PackageCheck::Unlinked { what: "server addition", id: id.clone() });
        }
    }
    for ficha in &package.fichas {
        let added = ficha.symbol_id.as_ref().is_some_and(|id| package.server_additions.iter().any(|a| a.symbol_id == *id));
        if added != (ficha.declared_by == DeclaredBy::ServerLacuna) {
            return Err(PackageCheck::Shape { what: "ficha declared_by", id: ficha.reference.0.clone() });
        }
    }
    // A selection's measure is its last suggestion's; with no selection there is none to name.
    let basis = &package.measure.selection_basis;
    let fits = match &package.selection {
        Some(selection) => {
            let named = match basis {
                MeasureSelectionBasis::Ok { suggestion, .. } | MeasureSelectionBasis::NoMeasure { suggestion } | MeasureSelectionBasis::Unreadable { suggestion } => Some(suggestion),
                MeasureSelectionBasis::NotApplicable {} | MeasureSelectionBasis::NotRecorded {} => None,
            };
            named.is_some() && named == selection.suggestions.last()
        }
        None => matches!(basis, MeasureSelectionBasis::NotApplicable {} | MeasureSelectionBasis::NotRecorded {}),
    };
    if !fits {
        return Err(PackageCheck::Shape { what: "selection basis", id: String::new() });
    }
    // Each format with its own field only. Format 1's value was never worked out, and is not checked
    // here: making it again (H3) gives what that format wrote. Format 2's is the two readings compared.
    let measure = &package.measure;
    let told = match package.format_version {
        PackageFormat::V1 => measure.measure_divergence.is_none() && measure.measure_diverges.is_some(),
        PackageFormat::V2 => measure.measure_diverges.is_none()
            && measure.measure_divergence.as_ref() == Some(&measure_divergence(&measure.at_question, &measure.selection_basis)),
    };
    if !told {
        return Err(PackageCheck::Shape { what: "measure divergence", id: String::new() });
    }
    Ok(())
}

/// H2: each unit comes again from its snapshot by `lines-v1`, in the package's representation.
/// Where its fichas lie was checked with the structure (H1).
pub fn check_sources(package: &EvidencePackage, snapshots: &[SnapshotInput]) -> Result<(), PackageCheck> {
    for unit in &package.units {
        let id = &unit.evidence_id.0;
        let snapshot = snapshots.iter().find(|s| s.path == unit.path).ok_or_else(|| PackageCheck::MissingSnapshot { path: unit.path.clone() })?;
        let bytes = source_sha256(snapshot.bytes);
        if unit.source_sha256 != bytes || snapshot.sha256 != bytes.as_str() || unit.source_id != snapshot.source_id {
            return Err(PackageCheck::SourceDiffers { unit: id.clone() });
        }
        let window = normalize_window(snapshot.bytes, unit.window_start_line, unit.window_end_line, package.representation)
            .map_err(|error| PackageCheck::Normalization { unit: id.clone(), error })?;
        let same = window.start_line == unit.start_line && window.end_line == unit.end_line && window.text_sha256 == unit.text_sha256 && window.excerpt_sha256 == unit.excerpt_sha256;
        if !same {
            return Err(PackageCheck::TextDiffers { unit: id.clone() });
        }
    }
    Ok(())
}
