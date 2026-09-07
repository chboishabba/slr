use crate::reasoning::ReasoningGraphDelta;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRevisionRecord {
    pub source_revision_ref: String,
    pub canonical_bytes_digest: String,
    pub canonical_text_digest: String,
    pub provider_receipt_ref: String,
    pub jurisdiction_ref: Option<String>,
    pub source_role_ref: String,
    pub authority_candidate_ref: Option<String>,
    pub parsed_pnf_ref: String,
    pub citation_topology_ref: String,
    pub assessment_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResearchWorldSnapshot {
    pub snapshot_ref: String,
    pub source_revisions: BTreeMap<String, SourceRevisionRecord>,
    pub reasoning_deltas: Vec<ReasoningGraphDelta>,
    pub query_vocabulary: BTreeSet<String>,
    pub authority_neighbourhood: BTreeSet<String>,
    pub iteration_receipt_refs: Vec<String>,
    pub current_conclusion_refs: BTreeSet<String>,
    pub authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldExtensionError {
    SourceRevisionRewriteAttempt,
}

impl ResearchWorldSnapshot {
    pub fn append_source(&mut self, source: SourceRevisionRecord) -> Result<(), WorldExtensionError> {
        if let Some(existing) = self.source_revisions.get(&source.source_revision_ref) {
            if existing != &source {
                return Err(WorldExtensionError::SourceRevisionRewriteAttempt);
            }
            return Ok(());
        }
        self.source_revisions.insert(source.source_revision_ref.clone(), source);
        Ok(())
    }

    pub fn apply_reasoning_delta(&mut self, delta: ReasoningGraphDelta) {
        self.query_vocabulary.extend(delta.lexical_terms.iter().cloned());
        self.authority_neighbourhood.extend(delta.discovered_citation_refs.iter().cloned());
        self.reasoning_deltas.push(delta);
    }

    pub fn replace_current_conclusions(&mut self, conclusions: impl IntoIterator<Item = String>) {
        self.current_conclusion_refs = conclusions.into_iter().collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(digest: &str) -> SourceRevisionRecord {
        SourceRevisionRecord {
            source_revision_ref: "source:1".into(),
            canonical_bytes_digest: digest.into(),
            canonical_text_digest: digest.into(),
            provider_receipt_ref: "provider:local".into(),
            jurisdiction_ref: Some("AU".into()),
            source_role_ref: "primary-case".into(),
            authority_candidate_ref: Some("authority:case".into()),
            parsed_pnf_ref: "pnf:1".into(),
            citation_topology_ref: "citations:1".into(),
            assessment_receipt_refs: vec![],
        }
    }

    #[test]
    fn source_revisions_are_append_only_but_conclusions_are_not() {
        let mut world = ResearchWorldSnapshot::default();
        world.append_source(source("sha:a")).unwrap();
        assert_eq!(world.append_source(source("sha:b")), Err(WorldExtensionError::SourceRevisionRewriteAttempt));
        world.replace_current_conclusions(["conclusion:a".into()]);
        world.replace_current_conclusions(["conclusion:b".into()]);
        assert!(world.current_conclusion_refs.contains("conclusion:b"));
        assert!(!world.current_conclusion_refs.contains("conclusion:a"));
    }
}
