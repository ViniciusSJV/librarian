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
        let mut stops: BTreeSet<String> = tokens("a o as os de da do das dos e em no na nos nas um uma para por que qual quais onde como the of in to from with is does how where").into_iter().collect();
        for word in &lexicon.stop_words {
            stops.extend(tokens(word));
        }
        let documents = catalog
            .symbols
            .values()
            .filter(|s| !matches!(s.kind.as_str(), "module" | "impl"))
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
        })
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
            let source = &self.catalog.sources[&symbol.source_id];
            let bytes = self.catalog.excerpt(&symbol.chunk_id).unwrap();
            let text = std::str::from_utf8(bytes)?;
            let mut end = text.len().min(4000).min(budget);
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            budget -= end;
            hits.push(Hit {
                symbol_id: id,
                name: symbol.qualified_name.clone(),
                path: source.path.clone(),
                source_sha256: source.sha256.clone(),
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                chunk_id: chunk.id.clone(),
                chunk_sha256: chunk.sha256.clone(),
                score,
                reasons,
                excerpt: text[..end].into(),
                excerpt_truncated: end < text.len(),
            });
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
            limits: vec!["Lexical candidates, not a semantic answer. Minimum coverage: 60% of query term groups; scores are not probabilities.".into(),
                "Name candidates are not resolved types/calls. No macros, cfg evaluation, compiler or runtime proof.".into(),
                "Graph expands only the first hit. Source nodes are leaves. Depth bounded to 0..3, 48 nodes, 96 edges.".into(),
                "Excerpts limited to 4000 UTF-8 bytes each and 16000 total. Truncation is explicit; hashes/lines refer to full chunks.".into()] })
    }
}
