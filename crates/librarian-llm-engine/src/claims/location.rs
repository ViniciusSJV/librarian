//! Caller-supplied symbol boundaries, not a language parser or semantic proof.
use super::{Claim, Context, Evidence, Kind};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Symbol {
    pub name: String,
    pub declaration_line: usize,
    /// One-based Unicode scalar column in the declaration line, not a byte offset.
    pub name_column: usize,
    pub end_line: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub evidence_id: String,
    pub path: String,
    pub symbol_name: String,
    pub declaration_line: usize,
    pub operation_start_line: usize,
    pub operation_end_line: usize,
}

pub(super) fn validate_symbols(e: &Evidence) -> Result<(), String> {
    let mut identities = HashSet::new();
    for s in &e.symbols {
        if s.name.trim().is_empty()
            || s.name.contains(['\n', '\r'])
            || s.name_column == 0
            || s.declaration_line < e.start_line
            || s.end_line > e.end_line
            || s.end_line < s.declaration_line
            || !identities.insert((&s.name, s.declaration_line))
        {
            return Err("Invalid or duplicate symbol identity/interval".into());
        }
        let declaration = e
            .excerpt
            .lines()
            .nth(s.declaration_line - e.start_line)
            .unwrap();
        let selected: String = declaration
            .chars()
            .skip(s.name_column - 1)
            .take(s.name.chars().count())
            .collect();
        if selected != s.name {
            return Err("Symbol name does not match declaration at name_column".into());
        }
    }
    Ok(())
}

pub(super) fn schema(context: &Context) -> serde_json::Value {
    use serde_json::json;
    let mut schema = super::response_schema(&context.policy);
    if context.require_location {
        let claim = &mut schema["properties"]["claims"]["items"];
        claim["required"]
            .as_array_mut()
            .unwrap()
            .push(json!("location"));
        claim["properties"]["location"] = json!({
            "anyOf":[{"type":"null"},{
                "type":"object","additionalProperties":false,
                "required":["evidence_id","path","symbol_name","declaration_line","operation_start_line","operation_end_line"],
                "properties":{
                    "evidence_id":{"type":"string","minLength":1},
                    "path":{"type":"string","minLength":1},
                    "symbol_name":{"type":"string","minLength":1},
                    "declaration_line":{"type":"integer","minimum":1},
                    "operation_start_line":{"type":"integer","minimum":1},
                    "operation_end_line":{"type":"integer","minimum":1}
                }
            }]
        });
    }
    schema
}

pub(super) fn check(context: &Context, claim: &Claim) -> Vec<&'static str> {
    let Some(l) = &claim.location else {
        return if context.require_location && matches!(claim.kind, Kind::Fact) {
            vec!["missing_location"]
        } else {
            vec![]
        };
    };
    if !matches!(claim.kind, Kind::Fact) {
        return vec!["location_requires_fact"];
    }
    let Some(e) = context.evidence.iter().find(|e| e.id == l.evidence_id) else {
        return vec!["location_unknown_evidence"];
    };
    let mut issues = vec![];
    if l.path != e.path {
        issues.push("location_wrong_path");
    }
    let Some(s) = e
        .symbols
        .iter()
        .find(|s| s.name == l.symbol_name && s.declaration_line == l.declaration_line)
    else {
        issues.push("location_unknown_symbol");
        return issues;
    };
    if l.operation_start_line < s.declaration_line
        || l.operation_end_line > s.end_line
        || l.operation_end_line < l.operation_start_line
    {
        issues.push("location_operation_out_of_range");
    }
    // Exact quote validation is performed by the main checker; coverage alone cannot pass it.
    let covered = |start, end| {
        claim.citations.iter().any(|c| {
            c.evidence_id == e.id && c.path == e.path && c.start_line <= start && c.end_line >= end
        })
    };
    if !covered(s.declaration_line, s.declaration_line) {
        issues.push("location_declaration_not_cited");
    }
    if !covered(l.operation_start_line, l.operation_end_line) {
        issues.push("location_operation_not_cited");
    }
    issues
}
