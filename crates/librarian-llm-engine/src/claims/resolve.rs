//! Citations the Librarian completes from the evidence before the strict check: the model
//! points at lines, the evidence supplies their text. Every change is recorded; no rule of
//! `check` is relaxed, and a passed check still says nothing about meaning.
use super::{Citation, Context, Evidence, Kind, Response};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Resolution {
    pub claim_id: String,
    /// `citation_range_from_quote`, `operation_range_from_quote`,
    /// `declaration_citation_added` or `operation_citation_added`.
    pub code: &'static str,
    pub evidence_id: String,
    pub start_line: usize,
    pub end_line: usize,
    /// The lines the model gave, when the change moved them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<[usize; 2]>,
}

/// The response with its citations completed, and what was changed. A response that is not
/// claims JSON, or needs nothing, comes back unchanged byte for byte.
///
/// - A quote that is not the text of its lines, but is the exact text of exactly one other run
///   of lines of the same evidence, moves to those lines; so does the location's operation when
///   it named the same lines. The quote is the model's proof of what it read; the numbers are not.
/// - A located FACT gets the citation of its declaration line and of its operation lines when
///   no citation covers them, quoted from the evidence. Both are positions the model already
///   wrote in `location`; the declaration itself is the caller's structural record.
pub fn resolve(context: &Context, response: &str) -> (String, Vec<Resolution>) {
    let Ok(mut parsed) = serde_json::from_str::<Response>(response) else {
        return (response.to_owned(), vec![]);
    };
    let mut resolutions = vec![];
    for claim in &mut parsed.claims {
        let mut record = |code, e: &Evidence, start, end, original| {
            resolutions.push(Resolution { claim_id: claim.id.clone(), code, evidence_id: e.id.clone(), start_line: start, end_line: end, original });
        };
        for c in &mut claim.citations {
            let Some(e) = context.evidence.iter().find(|e| e.id == c.evidence_id && e.path == c.path) else { continue };
            if lines(e, c.start_line, c.end_line).as_deref() == Some(c.quote.as_str()) {
                continue;
            }
            let Some(start) = unique_run(e, &c.quote) else { continue };
            let original = [c.start_line, c.end_line];
            c.start_line = start;
            c.end_line = start + c.quote.split('\n').count() - 1;
            record("citation_range_from_quote", e, c.start_line, c.end_line, Some(original));
            if let Some(l) = claim.location.as_mut().filter(|l| l.evidence_id == e.id && [l.operation_start_line, l.operation_end_line] == original) {
                l.operation_start_line = c.start_line;
                l.operation_end_line = c.end_line;
                record("operation_range_from_quote", e, c.start_line, c.end_line, Some(original));
            }
        }
        if !matches!(claim.kind, Kind::Fact) {
            continue;
        }
        let Some(l) = &claim.location else { continue };
        let Some(e) = context.evidence.iter().find(|e| e.id == l.evidence_id && e.path == l.path) else { continue };
        let Some(s) = e.symbols.iter().find(|s| s.name == l.symbol_name && s.declaration_line == l.declaration_line) else { continue };
        let mut wanted = vec![("declaration_citation_added", s.declaration_line, s.declaration_line)];
        if s.declaration_line <= l.operation_start_line && l.operation_start_line <= l.operation_end_line && l.operation_end_line <= s.end_line {
            wanted.push(("operation_citation_added", l.operation_start_line, l.operation_end_line));
        }
        for (code, start, end) in wanted {
            let covered = claim.citations.iter().any(|c| c.evidence_id == e.id && c.path == e.path && c.start_line <= start && c.end_line >= end);
            let Some(quote) = lines(e, start, end).filter(|_| !covered) else { continue };
            claim.citations.push(Citation { evidence_id: e.id.clone(), path: e.path.clone(), start_line: start, end_line: end, quote });
            record(code, e, start, end, None);
        }
    }
    if resolutions.is_empty() {
        return (response.to_owned(), resolutions);
    }
    (serde_json::to_string(&parsed).expect("claims serialize"), resolutions)
}

/// The evidence text of lines `start..=end`, joined with LF, when they are inside it.
fn lines(e: &Evidence, start: usize, end: usize) -> Option<String> {
    if start < e.start_line || end > e.end_line || end < start {
        return None;
    }
    Some(e.excerpt.lines().skip(start - e.start_line).take(end - start + 1).collect::<Vec<_>>().join("\n"))
}

/// First line of the only run of evidence lines equal to `quote`. A quote with no letter or
/// digit (a lone brace) says nothing about where the model looked.
fn unique_run(e: &Evidence, quote: &str) -> Option<usize> {
    if !quote.chars().any(char::is_alphanumeric) {
        return None;
    }
    let all: Vec<&str> = e.excerpt.lines().collect();
    let wanted: Vec<&str> = quote.split('\n').collect();
    let mut found = (0..(all.len() + 1).saturating_sub(wanted.len())).filter(|&i| all[i..i + wanted.len()] == wanted[..]);
    let first = found.next()?;
    found.next().is_none().then_some(e.start_line + first)
}
