#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_uses_requested_order_and_context() {
        let selection = Selection::new(vec!["F-2", "F-1"], 3);
        assert_eq!(selection.fact_ids, vec!["F-2", "F-1"]);
        assert_eq!(selection.context_lines, 3);
    }

    #[test]
    fn evidence_bundle_keeps_source_context_and_selection_order() {
        let source = SourceRef {
            id: "source-1".into(),
            path: "src/renderer.rs".into(),
            lines: vec!["a".into(), "b".into(), "c".into()],
        };
        let facts = vec![
            LibrarianFact::new("F-2", "second fact", "source-1", 2),
            LibrarianFact::new("F-1", "first fact", "source-1", 1),
        ];
        let selection = Selection::new(vec!["F-2", "F-1"], 1);
        let bundle = EvidenceBundle::new(source, facts.clone(), selection.clone());
        assert_eq!(bundle.selection.fact_ids, vec!["F-2", "F-1"]);
        assert_eq!(bundle.source.id, "source-1");
        assert_eq!(bundle.facts[0].id, "F-2");
        assert_eq!(bundle.facts.len(), 2);
        let exported = bundle.to_query("Explain this fact");
        assert_eq!(exported.question, "Explain this fact");
        assert_eq!(exported.evidence.facts[1].id, "F-1");
    }

    #[test]
    fn validation_rejects_empty_fact_and_empty_selection() {
        let valid = LibrarianFact::new("F-1", "statement", "source-1", 1);
        assert!(valid.validate().ok);

        let invalid = LibrarianFact::new("", "statement", "source-1", 1);
        assert!(!invalid.validate().ok);

        let empty_selection = Selection::new(vec![], 0);
        assert!(!empty_selection.validate().ok);

        let valid_bundle = EvidenceBundle::new(
            SourceRef {
                id: "source-1".into(),
                path: "src/example.rs".into(),
                lines: vec!["line 1".into()],
            },
            vec![valid.clone()],
            Selection::new(vec!["F-1"], 1),
        );
        assert!(valid_bundle.validate().ok);
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct SourceRef {
    pub id: String,
    pub path: String,
    pub lines: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct LibrarianFact {
    pub authorship: Option<String>,
    pub id: String,
    pub statement: String,
    pub source_id: String,
    pub line: usize,
}

impl LibrarianFact {
    pub fn new(id: &str, statement: &str, source_id: &str, line: usize) -> Self {
        Self {
            authorship: None,
            id: id.to_owned(),
            statement: statement.to_owned(),
            source_id: source_id.to_owned(),
            line,
        }
    }

    pub fn validate(&self) -> ValidationOutcome {
        if self.id.trim().is_empty() {
            return ValidationOutcome::fail("fact.id must not be empty");
        }
        if self.statement.trim().is_empty() {
            return ValidationOutcome::fail("fact.statement must not be empty");
        }
        if self.source_id.trim().is_empty() {
            return ValidationOutcome::fail("fact.source_id must not be empty");
        }
        if self.line == 0 {
            return ValidationOutcome::fail("fact.line must be greater than zero");
        }
        ValidationOutcome::ok()
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Selection {
    pub fact_ids: Vec<String>,
    pub context_lines: usize,
}

impl Selection {
    pub fn new(fact_ids: Vec<&str>, context_lines: usize) -> Self {
        Self {
            fact_ids: fact_ids.into_iter().map(str::to_owned).collect(),
            context_lines,
        }
    }

    pub fn validate(&self) -> ValidationOutcome {
        if self.fact_ids.is_empty() {
            return ValidationOutcome::fail("selection.fact_ids must not be empty");
        }
        if self.context_lines == 0 {
            return ValidationOutcome::fail("selection.context_lines must be greater than zero");
        }
        for fact_id in &self.fact_ids {
            if fact_id.trim().is_empty() {
                return ValidationOutcome::fail("selection.fact_ids must not contain empty ids");
            }
        }
        ValidationOutcome::ok()
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct ValidationOutcome {
    pub ok: bool,
    pub reason: Option<String>,
}

impl ValidationOutcome {
    pub fn ok() -> Self {
        Self {
            ok: true,
            reason: None,
        }
    }

    pub fn fail(reason: impl Into<String>) -> Self {
        Self {
            ok: false,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct EvidenceBundle {
    pub source: SourceRef,
    pub facts: Vec<LibrarianFact>,
    pub selection: Selection,
}

impl EvidenceBundle {
    pub fn new(source: SourceRef, facts: Vec<LibrarianFact>, selection: Selection) -> Self {
        Self {
            source,
            facts,
            selection,
        }
    }

    pub fn validate(&self) -> ValidationOutcome {
        if self.source.id.trim().is_empty() {
            return ValidationOutcome::fail("bundle.source.id must not be empty");
        }
        if self.source.path.trim().is_empty() {
            return ValidationOutcome::fail("bundle.source.path must not be empty");
        }
        if self.facts.is_empty() {
            return ValidationOutcome::fail("bundle.facts must not be empty");
        }
        let mut ids = std::collections::HashSet::new();
        for fact in &self.facts {
            if !ids.insert(fact.id.as_str()) {
                return ValidationOutcome::fail("bundle contains duplicate fact ids");
            }
            if fact.source_id != self.source.id
                || fact.line == 0
                || fact.line > self.source.lines.len()
            {
                return ValidationOutcome::fail(
                    "bundle fact must reference an existing source line",
                );
            }
            let outcome = fact.validate();
            if !outcome.ok {
                return ValidationOutcome::fail(format!(
                    "bundle fact invalid: {}",
                    outcome.reason.unwrap_or_default()
                ));
            }
        }
        if !self.selection.validate().ok {
            return self.selection.validate();
        }
        let mut selected = std::collections::HashSet::new();
        for id in &self.selection.fact_ids {
            if !ids.contains(id.as_str()) || !selected.insert(id.as_str()) {
                return ValidationOutcome::fail(
                    "bundle selection contains an unknown or duplicate fact",
                );
            }
        }
        ValidationOutcome::ok()
    }

    pub fn to_query(&self, question: &str) -> QueryExport {
        QueryExport {
            question: question.to_owned(),
            evidence: self.clone(),
            instructions: vec!["Cite a origem das evidências".to_owned()],
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct QueryExport {
    pub question: String,
    pub evidence: EvidenceBundle,
    pub instructions: Vec<String>,
}

/// Versioned textual source. Execution and capture metadata are supplied by the consumer.
/// Paths identify an edition only together with its content hash; IDs are dossier-local.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Source<Execution = (), Capture = ()> {
    pub capture: Option<Capture>,
    pub execution: Option<Execution>,
    pub executed: Option<bool>,
    pub git_commit: Option<String>,
    pub kind: Option<String>,
    pub path: Option<String>,
    pub sha256: Option<String>,
    pub id: String,
    pub lines: Vec<String>,
}

/// Shared multi-source dossier; engines may specialize source metadata.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Evidence<S = Source> {
    pub unknowns: Option<Vec<String>>,
    pub id: Option<String>,
    pub sources: Vec<S>,
    pub facts: Vec<LibrarianFact>,
}

/// Compatibility name for consumers predating the Librarian rename.
pub type BibliotecarioFact = LibrarianFact;
