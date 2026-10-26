//! Opt-in `allow_edits`: a HYPOTHESIS may carry the change it proposes as line edits over the
//! evidence (replace lines X–Y, rewrite a whole declaration, or insert before or after line Z)
//! instead of a unified diff. There are no hunk headers, counts or context lines to get wrong;
//! the diff is derived here, from the evidence. A replace cites every line it removes; a
//! rewrite does not need to, since its derived diff shows them. Nothing is applied or compiled.
use super::{Claim, Context, Kind};
use serde::{Deserialize, Serialize};

pub(super) const INSTRUCTION: &str = "Com allow_edits=true, a pergunta pede mudanças no código: proponha cada mudança como HYPOTHESIS com edits. Isso vale como o pedido explícito de hipótese e prevalece sobre project_instructions que proíbem propor alterações. edits é uma lista de {path, op, start_line, end_line, new_lines}: op=\"replace\" apaga as linhas start_line..end_line de evidence.line_map e põe new_lines no lugar (use só quando essas linhas devem sumir ou mudar; lista vazia apaga); op=\"rewrite\" reescreve uma declaração inteira: start_line e end_line são a primeira e a última linha dela em evidence.line_map e new_lines é a declaração inteira já mudada, copiando sem mudança as linhas que ficam (inclusive atributos e a chave final); o diff mostra o que mudou. op=\"insert_before\" insere new_lines antes da linha start_line e op=\"insert_after\" insere depois dela, sem apagar nada, com end_line igual a start_line. Para acrescentar algo antes de uma linha que fecha um bloco, use insert_before nessa linha. new_lines são linhas completas, com a indentação, sem número de linha. Cada linha trocada por replace precisa estar citada no próprio claim, com quote igual ao line_map. Cada edição mantém o equilíbrio de chaves, parênteses e colchetes das linhas que troca. As edições não se sobrepõem e ficam dentro das evidências; uma mudança em várias declarações usa uma edição para cada uma. Não escreva diff. O ganho é sempre hipótese: verification_plan diz como medir antes e depois e como conferir que o resultado não muda. Uma HYPOTHESIS usa location=null e premises=[]. Sem mudança proposta, use edits=null.";

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
            "op": {"type": "string", "enum": ["replace", "rewrite", "insert_before", "insert_after"]},
            "start_line": {"type": "integer", "minimum": 1},
            "end_line": {"type": "integer", "minimum": 1},
            "new_lines": {"type": "array", "items": {"type": "string"}}
        }
    }})
}

/// The evidence lines an edit covers, or the reason it cannot be placed.
fn place<'c>(context: &'c Context, edit: &Edit) -> Result<Vec<&'c str>, &'static str> {
    let shape = matches!(edit.op.as_str(), "replace" | "rewrite" | "insert_before" | "insert_after")
        && edit.start_line >= 1
        && edit.end_line >= edit.start_line
        && (matches!(edit.op.as_str(), "replace" | "rewrite") || edit.end_line == edit.start_line)
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

/// A rewrite restates a whole declaration, so it ends on the last line of one the caller's
/// symbols mark, and starts on its declaration line or above it (its attributes). Rewriting
/// only the first lines would leave the old body after the new one. Evidence without symbol
/// records gives nothing to compare, and the rule does not apply.
fn whole_declaration(context: &Context, edit: &Edit) -> bool {
    let symbols: Vec<_> = context.evidence.iter().filter(|e| e.path == edit.path).flat_map(|e| &e.symbols).collect();
    symbols.is_empty() || symbols.iter().any(|s| edit.start_line <= s.declaration_line && edit.end_line == s.end_line)
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
        let cited = |n: usize| claim.citations.iter().any(|c| c.path == edit.path && c.start_line <= n && n <= c.end_line);
        if matches!(edit.op.as_str(), "replace" | "rewrite") {
            // A replace cites each line it removes. A rewrite restates a whole declaration, and
            // its derived diff already shows every line it really removes.
            let uncited = edit.op == "replace" && !(edit.start_line..=edit.end_line).all(cited);
            if uncited {
                push("edits_change_uncited_lines");
            }
            if old == edit.new_lines.iter().map(String::as_str).collect::<Vec<_>>() {
                push("edits_no_change");
            }
            if edit.op == "rewrite" && !whole_declaration(context, edit) {
                push("edits_rewrite_not_whole_declaration");
            }
        } else if edit.new_lines.is_empty() {
            push("edits_no_change");
        }
        // A replacement keeps the delimiters it removes; an insertion brings balanced ones.
        let removed = if matches!(edit.op.as_str(), "replace" | "rewrite") { balance(old.iter().copied()) } else { [0; 3] };
        if removed != balance(edit.new_lines.iter().map(String::as_str)) {
            push("edits_unbalanced");
        }
    }
    let mut ranges: Vec<_> = edits.iter().map(|e| (&e.path, e.start_line, e.end_line)).collect();
    ranges.sort();
    if ranges.windows(2).any(|w| w[0].0 == w[1].0 && w[1].1 <= w[0].2) {
        push("edits_overlap");
    }
    issues
}

/// Context lines kept around each change, as `git diff` does; `git apply` needs them on both
/// sides of a hunk that is not at the start or end of the file.
const CONTEXT: usize = 3;

/// The unified diff a claim's edits describe, when they pass `check`; for display and export.
/// Each evidence block is rewritten with its edits; changes closer than twice the context
/// share a hunk, and context never leaves the block. New line numbers follow the edits above.
pub fn diff(context: &Context, claim: &Claim) -> Option<String> {
    let edits = claim.edits.as_ref()?;
    if !check(context, claim).is_empty() {
        return None;
    }
    let mut paths: Vec<&str> = edits.iter().map(|e| e.path.as_str()).collect();
    paths.sort();
    paths.dedup();
    let mut out = String::new();
    for path in paths {
        out.push_str(&format!("--- a/{path}\n+++ b/{path}\n"));
        let mut blocks: Vec<_> = context.evidence.iter().filter(|e| e.path == path).collect();
        blocks.sort_by_key(|e| e.start_line);
        let mut shift: isize = 0;
        for block in blocks {
            let inside: Vec<&Edit> = edits
                .iter()
                .filter(|e| e.path == path && block.start_line <= e.start_line && e.end_line <= block.end_line)
                .collect();
            if inside.is_empty() {
                continue;
            }
            // The block as (marker, text, old line number) in its new order.
            let mut entries: Vec<(char, &str, usize)> = vec![];
            let lines: Vec<&str> = block.excerpt.lines().collect();
            let mut rewritten_until = 0;
            for (i, line) in lines.iter().copied().enumerate() {
                let n = block.start_line + i;
                if n <= rewritten_until {
                    continue;
                }
                if let Some(rewrite) = inside.iter().find(|e| e.op == "rewrite" && e.start_line == n) {
                    let old = &lines[i..=i + rewrite.end_line - rewrite.start_line];
                    entries.extend(line_diff(old, &rewrite.new_lines).into_iter().map(|(m, text, k)| (m, text, k.map_or(0, |k| n + k))));
                    rewritten_until = rewrite.end_line;
                    continue;
                }
                let added = |op: &'static str| inside.iter().filter(move |e| e.op == op && e.start_line == n).flat_map(|e| &e.new_lines);
                entries.extend(added("insert_before").map(|l| ('+', l.as_str(), 0)));
                match inside.iter().find(|e| e.op == "replace" && e.start_line <= n && n <= e.end_line) {
                    Some(replace) => {
                        entries.push(('-', line, n));
                        if n == replace.end_line {
                            entries.extend(replace.new_lines.iter().map(|l| ('+', l.as_str(), 0)));
                        }
                    }
                    None => entries.push((' ', line, n)),
                }
                entries.extend(added("insert_after").map(|l| ('+', l.as_str(), 0)));
            }
            let changed: Vec<usize> = (0..entries.len()).filter(|i| entries[*i].0 != ' ').collect();
            let mut groups: Vec<(usize, usize)> = vec![];
            for i in changed {
                match groups.last_mut() {
                    Some((_, last)) if i - *last <= 2 * CONTEXT + 1 => *last = i,
                    _ => groups.push((i, i)),
                }
            }
            for (first, last) in groups {
                let lo = first.saturating_sub(CONTEXT);
                let hi = (last + CONTEXT).min(entries.len() - 1);
                let hunk = &entries[lo..=hi];
                let old_start = hunk.iter().find(|e| e.0 != '+')?.2;
                let old_count = hunk.iter().filter(|e| e.0 != '+').count();
                let new_count = hunk.iter().filter(|e| e.0 != '-').count();
                let new_start = old_start as isize + shift;
                out.push_str(&format!("@@ -{old_start},{old_count} +{new_start},{new_count} @@\n"));
                hunk.iter().for_each(|(marker, text, _)| out.push_str(&format!("{marker}{text}\n")));
                shift += new_count as isize - old_count as isize;
            }
        }
    }
    Some(out)
}

/// Net `{}`, `()` and `[]` of some lines. Delimiters inside strings count too: a change that
/// moves one into or out of a literal is refused, which errs on the side of review.
fn balance<'a>(lines: impl Iterator<Item = &'a str>) -> [isize; 3] {
    let mut net = [0isize; 3];
    for c in lines.flat_map(str::chars) {
        match c {
            '{' => net[0] += 1,
            '}' => net[0] -= 1,
            '(' => net[1] += 1,
            ')' => net[1] -= 1,
            '[' => net[2] += 1,
            ']' => net[2] -= 1,
            _ => {}
        }
    }
    net
}

/// Minimal line diff of a rewrite by longest common subsequence: kept lines stay context, so a
/// rewritten declaration shows only what really changed. Each entry carries the offset of its
/// old line (None for added lines). Large rewrites fall back to removing all, then adding all.
fn line_diff<'a>(old: &[&'a str], new: &'a [String]) -> Vec<(char, &'a str, Option<usize>)> {
    let (n, m) = (old.len(), new.len());
    if n * m > 4_000_000 {
        let removed = old.iter().enumerate().map(|(k, l)| ('-', *l, Some(k)));
        return removed.chain(new.iter().map(|l| ('+', l.as_str(), None))).collect();
    }
    // lcs[i][j]: length of the common subsequence of old[i..] and new[j..].
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if old[i] == new[j] { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, vec![]);
    while i < n || j < m {
        if i < n && j < m && old[i] == new[j] {
            out.push((' ', old[i], Some(i)));
            (i, j) = (i + 1, j + 1);
        } else if i < n && (j == m || lcs[i + 1][j] >= lcs[i][j + 1]) {
            out.push(('-', old[i], Some(i)));
            i += 1;
        } else {
            out.push(('+', new[j].as_str(), None));
            j += 1;
        }
    }
    out
}
