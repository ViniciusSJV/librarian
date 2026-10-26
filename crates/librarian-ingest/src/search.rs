//! Deterministic lexical retrieval. Vocabulary belongs to the consumer, not the renderer here.
use crate::{
    graph::{Graph, Neighborhood},
    Catalog, Result,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lexicon {
    #[serde(default)]
    pub groups: Vec<Vec<String>>,
    #[serde(default)]
    pub stop_words: Vec<String>,
}

/// Words every question drops before searching, in addition to the lexicon's `stop_words`.
pub const BUILTIN_STOP_WORDS: &str =
    "a o as os de da do das dos e em no na nos nas um uma para por que qual quais onde como the of in to from with is does how where";

/// Accent folding for Portuguese, identifier splitting (including acronym boundaries).
pub fn tokens(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut split = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase()
            && i > 0
            && (chars[i - 1].is_lowercase()
                || chars[i - 1].is_numeric()
                || (chars[i - 1].is_uppercase()
                    && chars.get(i + 1).is_some_and(|n| n.is_lowercase())))
        {
            split.push(' ');
        }
        let c = match c.to_lowercase().next().unwrap_or(c) {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        };
        if ('\u{0300}'..='\u{036f}').contains(&c) {
            continue;
        }
        split.push(if c.is_alphanumeric() { c } else { ' ' });
    }
    split.split_whitespace().map(str::to_owned).collect()
}

/// Code vocabulary: lexical tokens of declared names and how many symbols carry each,
/// most frequent first. Deterministic; it proposes lexicon terms and never ranks hits.
pub fn vocabulary(catalog: &Catalog) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for symbol in catalog.symbols.values() {
        let unique: BTreeSet<String> = tokens(&symbol.name).into_iter().collect();
        for token in unique {
            if token.chars().count() >= 3 && !token.chars().all(|c| c.is_numeric()) {
                *counts.entry(token).or_default() += 1;
            }
        }
    }
    let mut ranked: Vec<_> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchReason {
    pub term: String,
    pub expanded: Vec<String>,
    pub fields: Vec<String>,
    pub weight: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    pub symbol_id: String,
    pub name: String,
    pub path: String,
    pub source_sha256: String,
    pub start_line: usize,
    pub end_line: usize,
    pub chunk_id: String,
    pub chunk_sha256: String,
    pub score: usize,
    pub reasons: Vec<MatchReason>,
    pub excerpt: String,
    pub excerpt_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchReport {
    pub version: String,
    pub catalog_sha256: String,
    pub lexicon_sha256: String,
    pub question: String,
    pub terms: Vec<String>,
    pub status: String,
    pub hits: Vec<Hit>,
    pub graph: Option<Neighborhood>,
    pub diagnostics: usize,
    pub limits: Vec<String>,
    /// Chain step with owner context (G6d): the declarations added after the targets and why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_context: Option<crate::owner::OwnerContext>,
}

struct Document {
    id: String,
    fields: Vec<(&'static str, usize, BTreeSet<String>)>,
}
pub struct SearchIndex<'a> {
    catalog: &'a Catalog,
    lexicon: Lexicon,
    aliases: BTreeMap<String, BTreeSet<String>>,
    stops: BTreeSet<String>,
    documents: Vec<Document>,
    /// Symbols marked as tests, and declarations inside them, were left out (`without_tests`).
    tests_excluded: bool,
}

impl<'a> SearchIndex<'a> {
    pub fn new(catalog: &'a Catalog, lexicon: Lexicon) -> Result<Self> {
        let mut aliases = BTreeMap::new();
        for group in &lexicon.groups {
            let mut set = BTreeSet::new();
            for term in group {
                let normalized = tokens(term);
                if normalized.len() != 1 {
                    return Err("Each alias must normalize to one lexical token".into());
                }
                set.insert(normalized[0].clone());
            }
            for term in &set {
                if aliases.insert(term.clone(), set.clone()).is_some() {
                    return Err(format!("Overlapping alias group: {term}").into());
                }
            }
        }
        let mut stops: BTreeSet<String> = tokens(BUILTIN_STOP_WORDS).into_iter().collect();
        for word in &lexicon.stop_words {
            stops.extend(tokens(word));
        }
        let documents = catalog
            .symbols
            .values()
            // Each extractor declares which container kinds are navigation, not answers.
            .filter(|s| {
                let language = &catalog.sources[&s.source_id].language;
                !catalog
                    .registry
                    .extractor(language)
                    .is_some_and(|e| e.navigation_kinds().contains(&s.kind.as_str()))
            })
            .map(|s| {
                let source = &catalog.sources[&s.source_id];
                let text =
                    String::from_utf8_lossy(catalog.excerpt(&s.chunk_id).unwrap_or_default());
                Document {
                    id: s.id.clone(),
                    fields: vec![
                        ("name", 8, tokens(&s.name).into_iter().collect()),
                        ("scope", 3, tokens(&s.qualified_name).into_iter().collect()),
                        ("path", 1, tokens(&source.path).into_iter().collect()),
                        ("code", 1, tokens(&text).into_iter().collect()),
                    ],
                }
            })
            .collect();
        Ok(Self {
            catalog,
            lexicon,
            aliases,
            stops,
            documents,
            tests_excluded: false,
        })
    }

    /// Leaves out symbols the extractor marked as tests and every declaration inside one (a
    /// helper written within a test function). Helpers outside test functions stay.
    pub fn without_tests(mut self) -> Self {
        fn span<'c>(catalog: &'c Catalog, s: &'c crate::Symbol) -> (&'c str, usize, usize) {
            let c = &catalog.chunks[&s.chunk_id];
            (s.source_id.as_str(), c.start_byte, c.end_byte)
        }
        let catalog = self.catalog;
        let tests: Vec<_> = catalog.symbols.values().filter(|s| s.is_test).map(|s| span(catalog, s)).collect();
        self.documents.retain(|d| {
            let (source, start, end) = span(catalog, &catalog.symbols[&d.id]);
            !tests.iter().any(|(s, a, b)| *s == source && *a <= start && end <= *b)
        });
        self.tests_excluded = true;
        self
    }

    pub fn search(&self, question: &str, top: usize, depth: usize) -> Result<SearchReport> {
        if question.trim().is_empty()
            || question.len() > 8192
            || !(1..=10).contains(&top)
            || depth > 3
        {
            return Err("Question required (<=8192 bytes), top 1..10, depth 0..3".into());
        }
        // Synonyms in the same question count once, preventing plural repetition from inflating coverage.
        let mut groups: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();
        for term in tokens(question)
            .into_iter()
            .filter(|t| !self.stops.contains(t))
        {
            let expanded = self
                .aliases
                .get(&term)
                .cloned()
                .unwrap_or_else(|| BTreeSet::from([term.clone()]));
            let key = expanded.iter().next().unwrap().clone();
            groups.entry(key).or_insert((term, expanded));
        }
        let mut ranked = Vec::new();
        for doc in &self.documents {
            let mut reasons = vec![];
            for (term, expanded) in groups.values() {
                let mut fields = vec![];
                let mut weight = 0;
                for (name, w, words) in &doc.fields {
                    if !words.is_disjoint(expanded) {
                        fields.push((*name).into());
                        weight = weight.max(*w);
                    }
                }
                if weight > 0 {
                    reasons.push(MatchReason {
                        term: term.clone(),
                        expanded: expanded.iter().cloned().collect(),
                        fields,
                        weight,
                    });
                }
            }
            if groups.is_empty() || reasons.len() * 5 < groups.len() * 3 {
                continue;
            }
            let symbol = &self.catalog.symbols[&doc.id];
            // An observed test marker ranks lower; it is not evidence that a test ran.
            let score = reasons.len() * 100 + reasons.iter().map(|r| r.weight).sum::<usize>()
                - if symbol.is_test { 20 } else { 0 };
            ranked.push((score, symbol.is_test, doc.id.clone(), reasons));
        }
        ranked.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then(a.1.cmp(&b.1))
                .then_with(|| {
                    let ac = &self.catalog.chunks[&self.catalog.symbols[&a.2].chunk_id];
                    let bc = &self.catalog.chunks[&self.catalog.symbols[&b.2].chunk_id];
                    (ac.end_byte - ac.start_byte).cmp(&(bc.end_byte - bc.start_byte))
                })
                .then(a.2.cmp(&b.2))
        });
        let mut hits: Vec<Hit> = vec![];
        let mut budget = 16000usize;
        for (score, _, id, reasons) in ranked {
            if hits.len() == top || budget == 0 {
                break;
            }
            let symbol = &self.catalog.symbols[&id];
            let chunk = &self.catalog.chunks[&symbol.chunk_id];
            // Skip overlapping symbol spans from the same file after a better-scoring hit.
            if hits.iter().any(|h| {
                let c = &self.catalog.chunks[&h.chunk_id];
                c.source_id == chunk.source_id
                    && c.start_byte < chunk.end_byte
                    && chunk.start_byte < c.end_byte
            }) {
                continue;
            }
            hits.push(hit(self.catalog, id, score, reasons, &mut budget)?);
        }
        let graph = Graph::build(self.catalog);
        let neighborhood = hits
            .first()
            .map(|h| graph.neighborhood(&h.symbol_id, depth, 48, 96))
            .transpose()?;
        Ok(SearchReport { version: "lexical-search-v1".into(), catalog_sha256: graph.catalog_sha256,
            lexicon_sha256: crate::hash(&serde_json::to_vec(&self.lexicon)?), question: question.into(),
            terms: groups.values().map(|(term,_)| term.clone()).collect(),
            status: if hits.is_empty() { "no_lexical_evidence" } else { "candidates" }.into(), hits,
            graph: neighborhood, diagnostics: self.catalog.diagnostics.len(),
            limits: self.catalog.registry.search_limits.iter().map(|l| l.to_string())
                .chain(self.tests_excluded.then(|| "Test symbols marked by the extractor, and declarations inside them, were left out; helpers outside test functions remain.".to_owned()))
                .collect(),
            owner_context: None })
    }
}

/// One symbol as a hit, its excerpt cut to 4000 bytes and to what is left of `budget`.
fn hit(catalog: &Catalog, id: String, score: usize, reasons: Vec<MatchReason>, budget: &mut usize) -> Result<Hit> {
    let symbol = &catalog.symbols[&id];
    let chunk = &catalog.chunks[&symbol.chunk_id];
    let source = &catalog.sources[&symbol.source_id];
    let text = std::str::from_utf8(catalog.excerpt(&symbol.chunk_id).ok_or("Chunk without excerpt")?)?;
    let mut end = text.len().min(4000).min(*budget);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    *budget -= end;
    Ok(Hit {
        name: symbol.qualified_name.clone(),
        path: source.path.clone(),
        source_sha256: source.sha256.clone(),
        start_line: chunk.start_line,
        end_line: chunk.end_line,
        chunk_id: chunk.id.clone(),
        chunk_sha256: chunk.sha256.clone(),
        symbol_id: id,
        score,
        reasons,
        excerpt: text[..end].into(),
        excerpt_truncated: end < text.len(),
    })
}

/// Report of a chain step: the given declarations, in order, as hits without score or reasons.
/// Nothing is searched; the neighborhood is the first hit's, as in a search, for the page.
/// With an owner context, its added declarations follow the targets in `symbol_ids`.
pub fn chain_report(catalog: &Catalog, graph: &Graph, question: &str, symbol_ids: &[String], depth: usize,
    owner_context: Option<crate::owner::OwnerContext>) -> Result<SearchReport> {
    let mut budget = 16000usize;
    let hits = symbol_ids.iter().map(|id| hit(catalog, id.clone(), 0, vec![], &mut budget)).collect::<Result<Vec<_>>>()?;
    let neighborhood = hits.first().map(|h| graph.neighborhood(&h.symbol_id, depth, 48, 96)).transpose()?;
    let mut limits = vec!["Chain step: declarations named by mechanically checked answers to an earlier question; nothing was searched.".to_owned()];
    if owner_context.is_some() {
        limits.push(format!("Owner context: a method target brings the type its container is or names and the declarations that write the fields it reads, by written names; writes through other receivers are counted, not added; at most {} additions.", crate::owner::MAX_ADDED));
    }
    // The lexical-coverage limit does not apply: no term was matched.
    limits.extend(catalog.registry.search_limits.iter().filter(|l| !l.starts_with("Lexical")).map(|l| l.to_string()));
    Ok(SearchReport { version: "chain-step-v1".into(), catalog_sha256: graph.catalog_sha256.clone(),
        lexicon_sha256: String::new(), question: question.into(), terms: vec![], status: "chain".into(), hits,
        graph: neighborhood, diagnostics: catalog.diagnostics.len(), limits, owner_context })
}
