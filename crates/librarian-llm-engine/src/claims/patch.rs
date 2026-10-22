//! Opt-in `allow_patch`: a HYPOTHESIS may carry a unified diff of the change it proposes.
//! The checker reads the diff against the evidence only: every old line (context and removed)
//! must be the evidence text at that line, and every removed line must be cited by the claim.
//! It does not apply, compile or run anything; a diff that applies is not a faster program.
use super::{Claim, Context, Kind};

pub(super) const INSTRUCTION: &str = "Com allow_patch=true, uma HYPOTHESIS que propõe uma mudança pode trazer patch: um diff unificado (linhas --- a/<path> e +++ b/<path>, blocos @@ -início,quantidade +início,quantidade @@, linhas com espaço, - ou + no começo) sobre os paths de evidence.items. As linhas de contexto e as removidas precisam ser idênticas às de evidence.line_map nos mesmos números, e cada linha removida precisa estar citada no próprio claim. O ganho é sempre hipótese: verification_plan diz como medir antes e depois. Sem mudança proposta, use patch=null.";

struct Hunk {
    path: String,
    old_start: usize,
    old_count: usize,
    new_count: usize,
    /// (marker, text) in order: ' ' context, '-' removed, '+' added.
    lines: Vec<(char, String)>,
}

pub(super) fn check(context: &Context, claim: &Claim) -> Vec<&'static str> {
    let Some(patch) = &claim.patch else { return vec![] };
    if !context.allow_patch {
        return vec!["patch_not_allowed"];
    }
    if !matches!(claim.kind, Kind::Hypothesis) {
        return vec!["patch_requires_hypothesis"];
    }
    let Some(hunks) = parse(patch) else { return vec!["patch_malformed"] };
    let mut issues = vec![];
    for hunk in &hunks {
        let mut push = |code| {
            if !issues.contains(&code) {
                issues.push(code);
            }
        };
        let olds = hunk.lines.iter().filter(|(m, _)| *m != '+').count();
        let news = hunk.lines.iter().filter(|(m, _)| *m != '-').count();
        if olds != hunk.old_count || news != hunk.new_count || !hunk.lines.iter().any(|(m, _)| *m != ' ') {
            push("patch_malformed");
            continue;
        }
        let Some(evidence) = context.evidence.iter().find(|e| {
            e.path == hunk.path && e.start_line <= hunk.old_start && hunk.old_start + hunk.old_count <= e.end_line + 1
        }) else {
            if context.evidence.iter().any(|e| e.path == hunk.path) {
                push("patch_outside_evidence");
            } else {
                push("patch_unknown_path");
            }
            continue;
        };
        let source: Vec<&str> = evidence.excerpt.lines().collect();
        let mut number = hunk.old_start;
        let mut removed = vec![];
        for (marker, text) in &hunk.lines {
            if *marker == '+' {
                continue;
            }
            if source.get(number - evidence.start_line) != Some(&text.as_str()) {
                push("patch_does_not_apply");
            }
            if *marker == '-' {
                removed.push(number);
            }
            number += 1;
        }
        // A pure insertion is anchored by its context lines instead.
        let anchors = if removed.is_empty() { (hunk.old_start..hunk.old_start + hunk.old_count).collect() } else { removed.clone() };
        let cited = |n: &usize| claim.citations.iter().any(|c| c.path == hunk.path && c.start_line <= *n && *n <= c.end_line);
        if (removed.is_empty() && !anchors.iter().any(cited)) || !removed.iter().all(cited) {
            push("patch_changes_uncited_lines");
        }
    }
    issues
}

/// Unified diff hunks, or None when the text is not one. `a/` and `b/` prefixes are dropped;
/// both headers must name the same file. An empty line inside a hunk is an empty context line.
fn parse(patch: &str) -> Option<Vec<Hunk>> {
    let mut hunks: Vec<Hunk> = vec![];
    let mut path: Option<String> = None;
    let mut lines = patch.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(old) = line.strip_prefix("--- ") {
            let new = lines.next()?.strip_prefix("+++ ")?;
            let clean = |p: &str| p.split('\t').next().unwrap_or(p).trim().trim_start_matches("a/").trim_start_matches("b/").to_owned();
            if clean(old) != clean(new) {
                return None;
            }
            path = Some(clean(new));
        } else if let Some(header) = line.strip_prefix("@@ ") {
            let ranges = header.split(" @@").next()?;
            let mut parts = ranges.split_whitespace();
            let range = |part: Option<&str>, sign: char| -> Option<(usize, usize)> {
                let body = part?.strip_prefix(sign)?;
                let (start, count) = body.split_once(',').unwrap_or((body, "1"));
                Some((start.parse().ok()?, count.parse().ok()?))
            };
            let (old_start, old_count) = range(parts.next(), '-')?;
            let (_, new_count) = range(parts.next(), '+')?;
            if old_start == 0 {
                return None;
            }
            let mut hunk = Hunk { path: path.clone()?, old_start, old_count, new_count, lines: vec![] };
            while let Some(next) = lines.peek() {
                if next.starts_with("@@ ") || next.starts_with("--- ") {
                    break;
                }
                let next = lines.next()?;
                match next.chars().next() {
                    None => hunk.lines.push((' ', String::new())),
                    Some('\\') => {}
                    Some(marker @ (' ' | '-' | '+')) => hunk.lines.push((marker, next[1..].to_owned())),
                    Some(_) => return None,
                }
            }
            hunks.push(hunk);
        } else if !line.trim().is_empty() && path.is_none() && hunks.is_empty() {
            // Leading `diff --git` or index lines are allowed before the first header.
            continue;
        } else if !line.trim().is_empty() {
            return None;
        }
    }
    (!hunks.is_empty()).then_some(hunks)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn reads_headers_counts_and_markers() {
        let hunks = parse("diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -3,2 +3,2 @@ fn x()\n-let a = 1;\n+let a = 2;\n \n").unwrap();
        assert_eq!((hunks[0].path.as_str(), hunks[0].old_start, hunks[0].old_count, hunks[0].new_count), ("src/a.rs", 3, 2, 2));
        assert_eq!(hunks[0].lines, vec![('-', "let a = 1;".into()), ('+', "let a = 2;".into()), (' ', String::new())]);
        assert_eq!(parse("@@ -1 +1 @@\n-x\n+y\n").map(|h| h.len()), None, "a hunk needs a file header");
        assert_eq!(parse("not a diff").map(|h| h.len()), None);
        assert_eq!(parse("--- a/x\n+++ b/y\n@@ -1 +1 @@\n-x\n+y\n").map(|h| h.len()), None);
        assert_eq!(parse("--- a/x\n+++ b/x\n@@ -1 +1 @@\n-x\n+y\n").unwrap()[0].old_count, 1);
    }
}
