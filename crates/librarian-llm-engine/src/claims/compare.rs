//! Compares answers to the same question by content, not wording. Content is the claim's kind,
//! its location (symbol and operation lines) or, without one, its cited lines. Agreement between
//! answers is a signal, not proof: models can share the same mistake.
use super::{Claim, Kind};
use serde::Serialize;

/// What a claim asserts, stripped of its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Content {
    pub kind: &'static str,
    /// Present for located claims: (evidence_id, symbol_name, declaration_line).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<(String, String, usize)>,
    /// Operation lines when located; otherwise every cited range, sorted.
    pub lines: Vec<(String, usize, usize)>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Match {
    /// The answer has a claim with exactly this content.
    Equal,
    /// Same symbol with overlapping lines, or the same lines with another kind.
    Partial,
    Absent,
}

#[derive(Debug, Serialize)]
pub struct Cell {
    #[serde(rename = "match")]
    pub matched: Match,
    /// Claims of that answer behind the match, by id.
    pub claim_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Row {
    pub content: Content,
    /// One cell per answer, in the order given.
    pub cells: Vec<Cell>,
}

fn kind(k: &Kind) -> &'static str {
    match k {
        Kind::Fact => "FACT",
        Kind::Inference => "INFERENCE",
        Kind::Hypothesis => "HYPOTHESIS",
        Kind::Lacuna => "LACUNA",
    }
}

pub fn content(claim: &Claim) -> Content {
    match &claim.location {
        Some(l) => Content {
            kind: kind(&claim.kind),
            symbol: Some((l.evidence_id.clone(), l.symbol_name.clone(), l.declaration_line)),
            lines: vec![(l.evidence_id.clone(), l.operation_start_line, l.operation_end_line)],
        },
        None => {
            let mut lines: Vec<_> = claim.citations.iter().map(|c| (c.evidence_id.clone(), c.start_line, c.end_line)).collect();
            lines.sort();
            lines.dedup();
            Content { kind: kind(&claim.kind), symbol: None, lines }
        }
    }
}

fn overlaps(a: &[(String, usize, usize)], b: &[(String, usize, usize)]) -> bool {
    a.iter().any(|(ea, sa, fa)| b.iter().any(|(eb, sb, fb)| ea == eb && sa <= fb && sb <= fa))
}

fn partial(a: &Content, b: &Content) -> bool {
    let same_place = a.symbol == b.symbol && overlaps(&a.lines, &b.lines);
    let same_lines_other_kind = a.lines == b.lines && a.symbol == b.symbol && a.kind != b.kind;
    a != b && (same_place || same_lines_other_kind)
}

/// One row per distinct content found in any answer; `None` is an answer without readable claims.
/// Rows shared by more answers come first.
pub fn compare(answers: &[Option<&[Claim]>]) -> Vec<Row> {
    let contents: Vec<Vec<(String, Content)>> = answers
        .iter()
        .map(|a| a.unwrap_or_default().iter().map(|c| (c.id.clone(), content(c))).collect())
        .collect();
    let mut distinct: Vec<Content> = vec![];
    for (_, c) in contents.iter().flatten() {
        if !distinct.contains(c) {
            distinct.push(c.clone());
        }
    }
    let mut rows: Vec<Row> = distinct
        .into_iter()
        .map(|content| {
            let cells = contents
                .iter()
                .map(|claims| {
                    let ids = |pick: &dyn Fn(&Content) -> bool| -> Vec<String> {
                        claims.iter().filter(|(_, c)| pick(c)).map(|(id, _)| id.clone()).collect()
                    };
                    let equal = ids(&|c| *c == content);
                    if !equal.is_empty() {
                        return Cell { matched: Match::Equal, claim_ids: equal };
                    }
                    let near = ids(&|c| partial(c, &content));
                    let matched = if near.is_empty() { Match::Absent } else { Match::Partial };
                    Cell { matched, claim_ids: near }
                })
                .collect();
            Row { content, cells }
        })
        .collect();
    let equal = |r: &Row| r.cells.iter().filter(|c| c.matched == Match::Equal).count();
    rows.sort_by(|a, b| equal(b).cmp(&equal(a)).then_with(|| a.content.lines.cmp(&b.content.lines)));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claims::Response;

    fn claims(json: &str) -> Vec<Claim> {
        serde_json::from_str::<Response>(json).unwrap().claims
    }

    fn located(id: &str, kind: &str, start: usize, end: usize) -> String {
        format!(r#"{{"id":"{id}","kind":"{kind}","text":"t","premises":[],"verification_plan":null,"citations":[],
            "location":{{"evidence_id":"E1","path":"p.rs","symbol_name":"f","declaration_line":75,"operation_start_line":{start},"operation_end_line":{end}}}}}"#)
    }

    #[test]
    fn equal_partial_and_absent_by_content_not_text() {
        // qwen and Haiku point at line 86; Haiku also at 84–85; a third answer at 85–86 as INFERENCE.
        let qwen = claims(&format!(r#"{{"schema_version":1,"claims":[{}]}}"#, located("C1", "FACT", 86, 86)));
        let haiku = claims(&format!(r#"{{"schema_version":1,"claims":[{},{}]}}"#, located("C1", "FACT", 86, 86), located("C2", "FACT", 84, 85)));
        let other = claims(&format!(r#"{{"schema_version":1,"claims":[{}]}}"#, located("X", "INFERENCE", 85, 86)));
        let rows = compare(&[Some(&qwen), Some(&haiku), Some(&other), None]);
        assert_eq!(rows.len(), 3);
        // Line 86 is shared by two answers, partial in the third, absent in the unreadable one.
        let first: Vec<_> = rows[0].cells.iter().map(|c| &c.matched).collect();
        assert_eq!(first, [&Match::Equal, &Match::Equal, &Match::Partial, &Match::Absent]);
        assert_eq!(rows[0].content.lines, [("E1".to_string(), 86, 86)]);
        assert_eq!(rows[0].cells[2].claim_ids, ["X"]);
        // 84–85 only in Haiku; qwen's 86 does not overlap it.
        let lines_84: &Row = rows.iter().find(|r| r.content.lines[0].1 == 84).unwrap();
        let cells: Vec<_> = lines_84.cells.iter().map(|c| &c.matched).collect();
        assert_eq!(cells, [&Match::Absent, &Match::Equal, &Match::Partial, &Match::Absent]);
    }

    #[test]
    fn unlocated_claims_compare_by_cited_lines() {
        let a = claims(r#"{"schema_version":1,"claims":[{"id":"A","kind":"LACUNA","text":"x","premises":[],"verification_plan":null,
            "citations":[{"evidence_id":"E1","path":"p","start_line":5,"end_line":6,"quote":"q"},{"evidence_id":"E1","path":"p","start_line":1,"end_line":1,"quote":"q"}]}]}"#);
        let b = claims(r#"{"schema_version":1,"claims":[{"id":"B","kind":"LACUNA","text":"other words","premises":[],"verification_plan":null,
            "citations":[{"evidence_id":"E1","path":"p","start_line":1,"end_line":1,"quote":"q"},{"evidence_id":"E1","path":"p","start_line":5,"end_line":6,"quote":"q"}]}]}"#);
        let rows = compare(&[Some(&a), Some(&b)]);
        assert_eq!(rows.len(), 1, "citation order and text do not matter");
        assert!(rows[0].cells.iter().all(|c| c.matched == Match::Equal));
    }
}
