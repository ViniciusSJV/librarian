//! Opt-in `allow_edits`: a HYPOTHESIS may carry the change it proposes as line edits over the
//! evidence (replace lines X–Y, or insert after line Z) instead of a unified diff. There are no
//! hunk headers, counts or context lines to get wrong; the diff is derived here, from the
//! evidence. Every replaced line must be cited by the claim. Nothing is applied or compiled.
use super::{Claim, Context, Kind};
use serde::{Deserialize, Serialize};

pub(super) const INSTRUCTION: &str = "Com allow_edits=true, a pergunta pede mudanças no código: proponha cada mudança como HYPOTHESIS com edits. Isso vale como o pedido explícito de hipótese e prevalece sobre project_instructions que proíbem propor alterações. edits é uma lista de {path, op, start_line, end_line, new_lines}: op=\"replace\" troca as linhas start_line..end_line de evidence.line_map por new_lines (lista vazia apaga as linhas); op=\"insert_after\" insere new_lines depois da linha start_line, com end_line igual a start_line. new_lines são linhas completas, com a indentação, sem número de linha. Cada linha trocada por replace precisa estar citada no próprio claim, com quote igual ao line_map. As edições não se sobrepõem e ficam dentro das evidências; uma mudança em várias declarações usa uma edição para cada uma. Não escreva diff. O ganho é sempre hipótese: verification_plan diz como medir antes e depois e como conferir que o resultado não muda. Uma HYPOTHESIS usa location=null e premises=[]. Sem mudança proposta, use edits=null.";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edit {
    pub path: String,
    /// `replace` or `insert_after`.
    pub op: String,
    pub start_line: usize,
    pub end_line: usize,
    pub new_lines: Vec<String>,
}

pub(super) fn schema() -> serde_json::Value {
    serde_json::json!({"type": ["array", "null"], "items": {
        "type": "object", "additionalProperties": false,
        "required": ["path", "op", "start_line", "end_line", "new_lines"],
        "properties": {
            "path": {"type": "string", "minLength": 1},
            "op": {"type": "string", "enum": ["replace", "insert_after"]},
            "start_line": {"type": "integer", "minimum": 1},
            "end_line": {"type": "integer", "minimum": 1},
            "new_lines": {"type": "array", "items": {"type": "string"}}
        }
    }})
}

/// The evidence lines an edit covers, or the reason it cannot be placed.
fn place<'c>(context: &'c Context, edit: &Edit) -> Result<Vec<&'c str>, &'static str> {
    let shape = matches!(edit.op.as_str(), "replace" | "insert_after")
        && edit.start_line >= 1
        && edit.end_line >= edit.start_line
        && (edit.op == "replace" || edit.end_line == edit.start_line)
        && edit.new_lines.iter().all(|l| !l.contains('\n') && !l.contains('\r'));
    if !shape {
        return Err("edits_malformed");
    }
    let Some(evidence) = context
        .evidence
        .iter()
        .find(|e| e.path == edit.path && e.start_line <= edit.start_line && edit.end_line <= e.end_line)
    else {
        return Err(if context.evidence.iter().any(|e| e.path == edit.path) { "edits_outside_evidence" } else { "edits_unknown_path" });
    };
    Ok(evidence
        .excerpt
        .lines()
        .skip(edit.start_line - evidence.start_line)
        .take(edit.end_line - edit.start_line + 1)
        .collect())
}

pub(super) fn check(context: &Context, claim: &Claim) -> Vec<&'static str> {
    let Some(edits) = &claim.edits else { return vec![] };
    if !context.allow_edits {
        return vec!["edits_not_allowed"];
    }
    if !matches!(claim.kind, Kind::Hypothesis) {
        return vec!["edits_require_hypothesis"];
    }
    if edits.is_empty() {
        return vec!["edits_malformed"];
    }
    let mut issues = vec![];
    let mut push = |code| {
        if !issues.contains(&code) {
            issues.push(code);
        }
    };
    for edit in edits {
        let old = match place(context, edit) {
            Ok(old) => old,
            Err(code) => {
                push(code);
                continue;
            }
        };
        if edit.op == "replace" {
            let cited = |n: usize| claim.citations.iter().any(|c| c.path == edit.path && c.start_line <= n && n <= c.end_line);
            if !(edit.start_line..=edit.end_line).all(cited) {
                push("edits_change_uncited_lines");
            }
            if old == edit.new_lines.iter().map(String::as_str).collect::<Vec<_>>() {
                push("edits_no_change");
            }
        } else if edit.new_lines.is_empty() {
            push("edits_no_change");
        }
    }
    let mut ranges: Vec<_> = edits.iter().map(|e| (&e.path, e.start_line, e.end_line)).collect();
    ranges.sort();
    if ranges.windows(2).any(|w| w[0].0 == w[1].0 && w[1].1 <= w[0].2) {
        push("edits_overlap");
    }
    issues
}

/// The unified diff a claim's edits describe, when they pass `check`; for display and export.
/// Hunks follow file order, with new line numbers shifted by the edits above them.
pub fn diff(context: &Context, claim: &Claim) -> Option<String> {
    let edits = claim.edits.as_ref()?;
    if !check(context, claim).is_empty() {
        return None;
    }
    let mut sorted: Vec<&Edit> = edits.iter().collect();
    sorted.sort_by(|a, b| (&a.path, a.start_line).cmp(&(&b.path, b.start_line)));
    let mut out = String::new();
    let mut current: Option<&str> = None;
    let mut shift: isize = 0;
    for edit in sorted {
        if current != Some(edit.path.as_str()) {
            out.push_str(&format!("--- a/{0}\n+++ b/{0}\n", edit.path));
            current = Some(&edit.path);
            shift = 0;
        }
        let old = place(context, edit).ok()?;
        let new_start = (edit.start_line as isize + shift) as usize;
        if edit.op == "replace" {
            out.push_str(&format!("@@ -{},{} +{},{} @@\n", edit.start_line, old.len(), new_start, edit.new_lines.len()));
            old.iter().for_each(|l| out.push_str(&format!("-{l}\n")));
            shift += edit.new_lines.len() as isize - old.len() as isize;
        } else {
            out.push_str(&format!("@@ -{},1 +{},{} @@\n {}\n", edit.start_line, new_start, edit.new_lines.len() + 1, old[0]));
            shift += edit.new_lines.len() as isize;
        }
        edit.new_lines.iter().for_each(|l| out.push_str(&format!("+{l}\n")));
    }
    Some(out)
}
