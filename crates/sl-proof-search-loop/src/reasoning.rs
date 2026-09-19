use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CitationUse {
    Mentioned,
    Quoted,
    ReliedOn,
    Adopted,
    Applied,
    Followed,
    Distinguished,
    Criticised,
    Rejected,
    Overruled,
    PartySubmission,
    HistoricalBackground,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReasoningRole {
    Rule,
    Premise,
    Exception,
    Analogy,
    Distinction,
    Policy,
    FactualFinding,
    Burden,
    Remedy,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConditionKind {
    Factual,
    Legal,
    Jurisdictional,
    Temporal,
    Procedural,
    Evidential,
    Exception,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConditionCoordinate {
    pub kind: ConditionKind,
    pub condition_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionReasoningEdge {
    pub citing_document_ref: String,
    pub citing_proposition_ref: String,
    pub cited_document_ref: String,
    pub cited_proposition_ref: String,
    pub citation_use: CitationUse,
    pub reasoning_role: ReasoningRole,
    pub condition_coordinates: Vec<ConditionCoordinate>,
    pub pinpoint_ref: Option<String>,
    pub judge_or_speaker_ref: Option<String>,
    pub court_ref: Option<String>,
    pub jurisdiction_ref: Option<String>,
    pub temporal_ref: Option<String>,
    pub outcome_ref: Option<String>,
    pub remedy_ref: Option<String>,
    pub burden_refs: Vec<String>,
    pub exception_refs: Vec<String>,
    pub lexical_realisation: String,
    pub reviewed: bool,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReasoningGraphDelta {
    pub edges: Vec<PropositionReasoningEdge>,
    pub discovered_citation_refs: Vec<String>,
    pub lexical_terms: Vec<String>,
    pub condition_refs: Vec<String>,
    pub delta_authority: &'static str,
}

impl ReasoningGraphDelta {
    pub fn from_edges(edges: Vec<PropositionReasoningEdge>) -> Self {
        let mut citations = BTreeSet::new();
        let mut lexical = BTreeSet::new();
        let mut conditions = BTreeSet::new();
        for e in &edges {
            citations.insert(e.cited_document_ref.clone());
            if !e.lexical_realisation.is_empty() {
                lexical.insert(e.lexical_realisation.clone());
            }
            for c in &e.condition_coordinates {
                conditions.insert(c.condition_ref.clone());
            }
        }
        Self {
            edges,
            discovered_citation_refs: citations.into_iter().collect(),
            lexical_terms: lexical.into_iter().collect(),
            condition_refs: conditions.into_iter().collect(),
            delta_authority: "experimental_candidate_only",
        }
    }
}

pub fn edge_may_feed_search(edge: &PropositionReasoningEdge) -> bool {
    edge.reviewed && edge.candidate_only
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsed_case_delta_retains_reasoning_conditions_and_citation_use() {
        let edge = PropositionReasoningEdge {
            citing_document_ref: "case:a".into(),
            citing_proposition_ref: "prop:a".into(),
            cited_document_ref: "case:b".into(),
            cited_proposition_ref: "prop:b".into(),
            citation_use: CitationUse::Distinguished,
            reasoning_role: ReasoningRole::Distinction,
            condition_coordinates: vec![ConditionCoordinate { kind: ConditionKind::Factual, condition_ref: "cond:positive-act".into() }],
            pinpoint_ref: Some("[42]".into()),
            judge_or_speaker_ref: Some("judge:x".into()),
            court_ref: Some("HCA".into()),
            jurisdiction_ref: Some("AU".into()),
            temporal_ref: Some("2015".into()),
            outcome_ref: Some("claim-dismissed".into()),
            remedy_ref: None,
            burden_refs: vec![],
            exception_refs: vec![],
            lexical_realisation: "positive operational act".into(),
            reviewed: true,
            candidate_only: true,
        };
        let d = ReasoningGraphDelta::from_edges(vec![edge]);
        assert_eq!(d.discovered_citation_refs, vec!["case:b"]);
        assert_eq!(d.condition_refs, vec!["cond:positive-act"]);
    }
}
