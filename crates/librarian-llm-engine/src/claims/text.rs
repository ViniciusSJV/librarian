//! Opt-in rules over a claim's `text` (`strict_text`): line numbers and backticked code it states
//! must be covered by the claim's own citations, and a sentence naming code and a line must name
//! code on that line. Mechanical and partial: a paraphrase without numbers or backticks passes
//! unseen, and meaning is still not checked.
use super::{Claim, Context};
use std::collections::HashSet;

pub(super) const INSTRUCTION: &str = "Com strict_text=true, o text só pode afirmar o que as citações do próprio claim mostram: todo número de linha escrito no text (\"linha 88\", \"linhas 84–86\", \"arquivo.rs:88\") precisa estar dentro de uma citação do claim, uma frase que nomeia código e uma linha precisa nomear código que está naquela linha, e todo trecho entre crases precisa aparecer literalmente nas linhas citadas (\"...\" pode abreviar o meio de um trecho). Para afirmar algo sobre outra linha, cite-a ou crie outro claim.";

/// Ranges longer than this are not expanded; they count as uncited.
const MAX_RANGE: usize = 10_000;

pub(super) fn check(context: &Context, claim: &Claim) -> Vec<&'static str> {
    let mut issues = vec![];
    let covered = |n: usize| claim.citations.iter().any(|c| c.start_line <= n && n <= c.end_line);
    let uncited_line = mentioned_lines(&claim.text)
        .into_iter()
        .any(|(start, end)| end < start || end - start > MAX_RANGE || !(start..=end).all(covered));
    if uncited_line {
        issues.push("text_cites_uncited_line");
    }
    // The source lines themselves, so a mismatched quote cannot vouch for the text.
    let cited: Vec<String> = claim
        .citations
        .iter()
        .filter_map(|c| {
            let e = context.evidence.iter().find(|e| e.id == c.evidence_id)?;
            let lines: Vec<&str> = e.excerpt.lines().collect();
            let range = c.start_line.checked_sub(e.start_line)?..(c.end_line + 1).checked_sub(e.start_line)?;
            Some(lines.get(range)?.join("\n"))
        })
        .collect();
    let cited = collapse(&cited.join("\n"));
    let names = |span: &str| {
        claim.citations.iter().any(|c| c.path == span || c.evidence_id == span)
    };
    let uncited_code = backticked(&claim.text).into_iter().any(|span| {
        let span = span.trim();
        !span.is_empty() && !names(span) && !appears(&cited, span)
    });
    if uncited_code {
        issues.push("text_quotes_uncited_code");
    }
    if !uncited_line && misplaced_name(context, claim) {
        issues.push("text_line_lacks_named_code");
    }
    issues
}

/// A sentence names code (identifiers that occur in the cited lines) and lines, and none of
/// those identifiers is on any of those lines: "zone_of is called (daily_report.py:46)" when line 46 is
/// `rows.append(`. Sentences naming no cited identifier are not judged.
fn misplaced_name(context: &Context, claim: &Claim) -> bool {
    let source = |n: usize| {
        claim.citations.iter().filter(|c| c.start_line <= n && n <= c.end_line).find_map(|c| {
            let e = context.evidence.iter().find(|e| e.id == c.evidence_id)?;
            e.excerpt.lines().nth(n.checked_sub(e.start_line)?)
        })
    };
    // The declarations the claim talks about are its subject: "save talks to the database on
    // lines 7, 9 and 10" names `save`, which is on none of them. A subject alone does not make a
    // sentence judged, but it still matches its own declaration line.
    let subjects: HashSet<&str> = claim
        .citations
        .iter()
        .filter_map(|c| context.evidence.iter().find(|e| e.id == c.evidence_id))
        .flat_map(|e| e.symbols.iter().map(|s| s.name.as_str()))
        .chain(claim.location.as_ref().map(|l| l.symbol_name.as_str()))
        .collect();
    let cited: HashSet<&str> = claim
        .citations
        .iter()
        .flat_map(|c| (c.start_line..=c.end_line).take(MAX_RANGE).filter_map(source))
        .flat_map(identifiers)
        .collect();
    // File names are places, not code: `daily_report.py:46` does not name `daily_report`. The
    // stand-in still reads as a file name, so `:46` stays a line mention.
    let mut text = claim.text.clone();
    for c in &claim.citations {
        text = text.replace(&c.path, "f.f");
        if let Some(base) = c.path.rsplit('/').next() {
            text = text.replace(base, "f.f");
        }
    }
    // Any one of the sentence's lines may carry the names: "rows is created on line 43, before the
    // loop on line 44" names line 44 only as a landmark.
    sentences(&text).into_iter().any(|sentence| {
        let named: HashSet<&str> = code_names(sentence).into_iter().filter(|n| cited.contains(n)).collect();
        let lines = mentioned_lines(sentence);
        named.iter().any(|n| !subjects.contains(n))
            && !lines.is_empty()
            && !lines.into_iter().any(|(start, end)| {
                (start..=end).take(MAX_RANGE).filter_map(source).any(|line| identifiers(line).iter().any(|n| named.contains(n)))
            })
    })
}

/// Identifiers of at least three characters: shorter ones (`a`, `de`, `x`) match prose too often.
fn identifiers(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| w.chars().count() >= 3 && !w.starts_with(|c: char| c.is_ascii_digit()))
        .collect()
}

/// Identifiers a sentence writes as code: inside backticks, joined by `.` or `::`, called with
/// `(`, or shaped like code (`zone_of`, `colorAt`). Plain words that happen to be names in the
/// source ("the canvas", "each pixel") are prose.
fn code_names(sentence: &str) -> Vec<&str> {
    sentence
        .split_whitespace()
        .flat_map(|chunk| {
            let core = chunk.trim_matches(|c: char| !(c.is_alphanumeric() || c == '_'));
            let joined = chunk.contains('`') || core.contains('.') || core.contains("::") || chunk.trim_start_matches('(').contains('(');
            identifiers(core).into_iter().filter(move |id| {
                joined || id.contains('_') || id.chars().skip(1).any(char::is_uppercase)
            })
        })
        .collect()
}

/// Split after `.`, `!`, `?` or `;` followed by a space or the end, and at line breaks.
fn sentences(text: &str) -> Vec<&str> {
    let mut parts = vec![];
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for (k, &(at, c)) in chars.iter().enumerate() {
        let next = chars.get(k + 1).map(|&(_, n)| n);
        if c == '\n' || (".!?;".contains(c) && next.is_none_or(char::is_whitespace)) {
            parts.push(&text[start..at]);
            start = at + c.len_utf8();
        }
    }
    parts.push(&text[start..]);
    parts
}

/// Whitespace runs collapsed to one space, so line breaks and indentation do not matter.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `span` occurs in `cited`; "..." or "…" in the span skips any text, in order.
fn appears(cited: &str, span: &str) -> bool {
    let mut rest = cited;
    for piece in span.split("...").flat_map(|p| p.split('…')).map(collapse).filter(|p| !p.is_empty()) {
        match rest.find(&piece) {
            Some(at) => rest = &rest[at + piece.len()..],
            None => return false,
        }
    }
    true
}

/// Contents of `single` backtick spans; an unmatched trailing backtick is ignored.
fn backticked(text: &str) -> Vec<&str> {
    let parts: Vec<&str> = text.split('`').collect();
    let complete = if parts.len().is_multiple_of(2) { parts.len() - 1 } else { parts.len() };
    parts[..complete].iter().skip(1).step_by(2).copied().collect()
}

/// Lines a text names: "linha 86", "linhas 84–86 e 88", "lines 3 and 4", "camera.rs:86".
fn mentioned_lines(text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = vec![];
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphabetic() && (i == 0 || !chars[i - 1].is_alphanumeric()) {
            let len = chars[i..].iter().take_while(|c| c.is_alphabetic()).count();
            let word: String = chars[i..i + len].iter().collect::<String>().to_lowercase();
            i += len;
            let whole = !chars.get(i).is_some_and(|c| c.is_alphanumeric());
            if whole && matches!(word.as_str(), "linha" | "linhas" | "line" | "lines") {
                i = list(&chars, i, &mut found);
            }
            continue;
        }
        if chars[i] == ':' && after_file_name(&chars, i) && chars.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
            i = range(&chars, i + 1, &mut found).unwrap_or(i + 1);
            continue;
        }
        i += 1;
    }
    found
}

/// The token before a colon looks like a file name (`camera.rs`, `src/lib.rs`), not a time.
fn after_file_name(chars: &[char], colon: usize) -> bool {
    let start = chars[..colon]
        .iter()
        .rposition(|c| c.is_whitespace() || "(`'\"".contains(*c))
        .map_or(0, |p| p + 1);
    let token = &chars[start..colon];
    token.contains(&'.') && token.last().is_some_and(|c| c.is_alphabetic())
}

fn skip_spaces(chars: &[char], mut i: usize) -> usize {
    while chars.get(i).is_some_and(|c| c.is_whitespace()) {
        i += 1;
    }
    i
}

/// A word at `i` from `words`, returning the position after it.
fn word(chars: &[char], i: usize, words: &[&str]) -> Option<usize> {
    let len = chars.get(i..)?.iter().take_while(|c| c.is_alphabetic()).count();
    let text: String = chars[i..i + len].iter().collect::<String>().to_lowercase();
    (len > 0 && words.contains(&text.as_str())).then_some(i + len)
}

fn number(chars: &[char], i: usize) -> Option<(usize, usize)> {
    let len = chars.get(i..)?.iter().take_while(|c| c.is_ascii_digit()).count();
    if len == 0 || len > 9 || chars.get(i + len).is_some_and(|c| c.is_alphabetic()) {
        return None;
    }
    Some((chars[i..i + len].iter().collect::<String>().parse().ok()?, i + len))
}

/// `86`, `84-86`, `84–86`, `84 a 86`, `84 até 86`, `84 to 86`.
fn range(chars: &[char], i: usize, found: &mut Vec<(usize, usize)>) -> Option<usize> {
    let (start, after) = number(chars, i)?;
    let next = skip_spaces(chars, after);
    let joined = match chars.get(next) {
        Some('-' | '–' | '—') => Some(next + 1),
        _ => word(chars, next, &["a", "até", "to"]),
    };
    if let Some((end, done)) = joined.and_then(|j| number(chars, skip_spaces(chars, j))) {
        found.push((start, end));
        return Some(done);
    }
    found.push((start, start));
    Some(after)
}

/// Ranges separated by commas, "e" or "and", after "linha(s)" or "line(s)".
fn list(chars: &[char], mut i: usize, found: &mut Vec<(usize, usize)>) -> usize {
    while let Some(after) = range(chars, skip_spaces(chars, i), found) {
        i = after;
        let next = skip_spaces(chars, i);
        let separator = match chars.get(next) {
            Some(',') => Some(next + 1),
            _ => word(chars, next, &["e", "and"]),
        };
        // A separator counts only when another number follows it.
        match separator {
            Some(s) if number(chars, skip_spaces(chars, s)).is_some() => i = s,
            _ => break,
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::{appears, backticked, collapse, mentioned_lines};

    #[test]
    fn finds_line_mentions_in_portuguese_english_and_paths() {
        assert_eq!(mentioned_lines("na linha 86 do arquivo"), [(86, 86)]);
        assert_eq!(mentioned_lines("Linhas 84–86 e 88, depois a linha 90."), [(84, 86), (88, 88), (90, 90)]);
        assert_eq!(mentioned_lines("lines 3 and 4, line 7 to 9"), [(3, 3), (4, 4), (7, 9)]);
        assert_eq!(mentioned_lines("src/camera.rs:86 e camera.rs:84-86"), [(86, 86), (84, 86)]);
        assert_eq!(mentioned_lines("linhas 84 a 86"), [(84, 86)]);
        // Not line mentions: other words, times, versions, a list that stops at a word.
        assert!(mentioned_lines("alinhado 3 vezes, às 10:30 h? não: 3:x, versão 1.2:3").is_empty());
        assert!(mentioned_lines("guilinha 5, 3 linhas de código").is_empty());
        assert_eq!(mentioned_lines("linha 86 e a variável 3"), [(86, 86)]);
    }

    #[test]
    fn backticked_code_must_appear_in_order_with_ellipsis() {
        assert_eq!(backticked("a `x` b `y z` c `open"), ["x", "y z"]);
        let cited = collapse("        let pixel = inverse * Tuple::point(world_x, world_y, -1.0);\n        let direction = (pixel - origin).normalize();");
        assert!(appears(&cited, "let direction = (pixel - origin).normalize();"));
        assert!(appears(&cited, "inverse * Tuple::point(...)"));
        assert!(appears(&cited, "inverse  *  Tuple::point(world_x, …, -1.0)"));
        assert!(!appears(&cited, "Ray::new(origin, direction)"));
        assert!(!appears(&cited, "normalize() ... Tuple::point"));
    }
}
