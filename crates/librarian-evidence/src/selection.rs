//! A selection: the sources a client chose over the suggestions it was shown. It is checked once, at
//! the entry of its question, against the suggestion records it cites and the edition, and recorded;
//! the same input gives the same bytes, which is how H3 makes it again.
use crate::{keeper::Keeper, package::edition_origin, suggestion};
use librarian_ingest::{
    envelope,
    evidence::{self, EditionSource, SelectionEdition, SelectionInput, SelectionRecord, SelectionRefusal, Sha256Hex, SuggestionRecord},
    graph::v2,
    hash, Catalog, Result,
};

/// A selection checked at the entry of a question: its record and the bytes it is kept in. Only
/// `accept` makes one.
pub struct AcceptedSelection {
    record: SelectionRecord,
    bytes: Vec<u8>,
    sha256: Sha256Hex,
}

impl AcceptedSelection {
    pub fn record(&self) -> &SelectionRecord {
        &self.record
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn sha256(&self) -> &Sha256Hex {
        &self.sha256
    }
}

/// The record of a selection: its suggestions read back and checked, then the selection checked
/// against them and the edition. The same input gives the same bytes.
pub fn accept(keeper: &dyn Keeper, catalog: &Catalog, nodes: &[String], input: SelectionInput) -> Result<AcceptedSelection> {
    let mut records = vec![];
    for sha256 in &input.suggestions {
        let suggestion = |why: String| SelectionRefusal::Suggestion { sha256: sha256.as_str().to_owned(), why };
        let record = suggestion::verify(keeper, catalog, sha256.as_str()).map_err(|e| suggestion(e.to_string()))?;
        if record.rules_version.0 != envelope::RULES_VERSION {
            return Err(suggestion(format!("regras {} não são as desta versão", record.rules_version.0)).into());
        }
        records.push(record);
    }
    let edition = selection_edition(keeper, catalog, &input)?;
    let cited: Vec<&SuggestionRecord> = records.iter().collect();
    evidence::check_selection(&input, nodes, &cited, &edition)?;
    let record = SelectionRecord::new(edition_origin(catalog)?, envelope::RULES_VERSION, input, &cited);
    let bytes = record.to_bytes()?;
    let sha256 = Sha256Hex::try_from(hash(&bytes))?;
    Ok(AcceptedSelection { record, bytes, sha256 })
}

/// What the edition says of the sources a selection names: the catalog's data, as a suggestion
/// gives them (a file head by the envelope's rule), and the graph's label, as a page shows it.
fn selection_edition(keeper: &dyn Keeper, catalog: &Catalog, input: &SelectionInput) -> Result<SelectionEdition> {
    let graph: v2::Graph = serde_json::from_slice(&keeper.graph()?)?;
    let mut edition = SelectionEdition::default();
    let ids = [&input.included, &input.removed, &input.left_out, &input.not_joined].into_iter().flatten().map(|e| e.id.as_str());
    for id in ids {
        let source = match id.strip_prefix(envelope::HEADER_PREFIX) {
            Some(path) => envelope::header(catalog, path).and_then(|(start, end)| {
                let source = catalog.source_by_path(path)?;
                let bytes = String::from_utf8_lossy(catalog.source_bytes(&source.path)?).lines().take(end).map(|l| l.len() + 1).sum();
                Some(EditionSource { name: path.to_owned(), path: path.to_owned(), kind: "header".into(), start_line: start, end_line: end, bytes })
            }),
            None => catalog.symbols().get(id).map(|symbol| {
                let chunk = &catalog.chunks()[&symbol.chunk_id];
                EditionSource { name: symbol.qualified_name.clone(), path: catalog.sources()[&symbol.source_id].path.clone(), kind: symbol.kind.clone(),
                    start_line: chunk.start_line, end_line: chunk.end_line, bytes: chunk.end_byte - chunk.start_byte }
            }),
        };
        if let Some(source) = source {
            edition.sources.insert(id.to_owned(), source);
        }
        if let Some(node) = graph.nodes.get(id) {
            edition.labels.insert(id.to_owned(), (node.label.clone(), node.kind.clone()));
        }
    }
    Ok(edition)
}
