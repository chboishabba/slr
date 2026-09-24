//! M12.2 UI-independent semantic trace path.
//!
//! This is the reader contract for traversing from an already-owned semantic
//! object back to literal source material and forward again to downstream
//! chronology/claim uses.  It is a projection only: it cannot create evidence
//! payment, semantic admission, applicability, or claim truth.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceReviewState {
    Unreviewed,
    ParseReviewed,
    SemanticallyAdmitted,
    Rejected,
    Abstained,
    Qualified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatementTraceCoordinate {
    pub statement_ref: String,
    pub document_ref: String,
    pub source_revision_ref: String,
    pub span_ref: String,
    pub literal_text: String,
}

impl StatementTraceCoordinate {
    pub fn validate(&self) -> Result<(), SemanticTraceError> {
        for (name, value) in [
            ("statement_ref", self.statement_ref.as_str()),
            ("document_ref", self.document_ref.as_str()),
            ("source_revision_ref", self.source_revision_ref.as_str()),
            ("span_ref", self.span_ref.as_str()),
            ("literal_text", self.literal_text.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(SemanticTraceError::EmptyCoordinate(name));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseTraceCoordinate {
    pub candidate_pnf_ref: String,
    pub parser_receipt_ref: Option<String>,
    pub review_ref: Option<String>,
    pub admission_receipt_ref: Option<String>,
    pub review_state: TraceReviewState,
}

impl ParseTraceCoordinate {
    pub fn validate(&self) -> Result<(), SemanticTraceError> {
        if self.candidate_pnf_ref.trim().is_empty() {
            return Err(SemanticTraceError::EmptyCoordinate("candidate_pnf_ref"));
        }
        if self.review_state == TraceReviewState::SemanticallyAdmitted
            && self
                .admission_receipt_ref
                .as_deref()
                .map_or(true, |value| value.trim().is_empty())
        {
            return Err(SemanticTraceError::MissingAdmissionReceipt);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticTracePath {
    pub focus_ref: String,
    pub statement: StatementTraceCoordinate,
    pub parse: Option<ParseTraceCoordinate>,
    pub observation_ref: String,
    pub event_ref: Option<String>,
    pub claim_refs: Vec<String>,
    pub downstream_use_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticTraceError {
    EmptyCoordinate(&'static str),
    MissingAdmissionReceipt,
    PromotionNotAllowed,
}

impl SemanticTracePath {
    pub fn validate(&self) -> Result<(), SemanticTraceError> {
        if self.focus_ref.trim().is_empty() {
            return Err(SemanticTraceError::EmptyCoordinate("focus_ref"));
        }
        if self.observation_ref.trim().is_empty() {
            return Err(SemanticTraceError::EmptyCoordinate("observation_ref"));
        }
        self.statement.validate()?;
        if let Some(parse) = &self.parse {
            parse.validate()?;
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(SemanticTraceError::PromotionNotAllowed);
        }
        Ok(())
    }

    #[must_use]
    pub fn reverse_source_chain(&self) -> Vec<&str> {
        let mut chain = vec![
            self.observation_ref.as_str(),
            self.statement.statement_ref.as_str(),
            self.statement.span_ref.as_str(),
            self.statement.source_revision_ref.as_str(),
        ];
        if let Some(parse) = &self.parse {
            chain.insert(1, parse.candidate_pnf_ref.as_str());
        }
        chain
    }

    #[must_use]
    pub fn forward_downstream_chain(&self) -> Vec<&str> {
        let mut chain = vec![
            self.statement.source_revision_ref.as_str(),
            self.statement.span_ref.as_str(),
            self.statement.statement_ref.as_str(),
        ];
        if let Some(parse) = &self.parse {
            chain.push(parse.candidate_pnf_ref.as_str());
        }
        chain.push(self.observation_ref.as_str());
        if let Some(event_ref) = &self.event_ref {
            chain.push(event_ref.as_str());
        }
        chain.extend(self.claim_refs.iter().map(String::as_str));
        chain.extend(self.downstream_use_refs.iter().map(String::as_str));
        chain
    }

    #[must_use]
    pub const fn creates_evidence_payment(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace() -> SemanticTracePath {
        SemanticTracePath {
            focus_ref: "event:1".into(),
            statement: StatementTraceCoordinate {
                statement_ref: "statement:1".into(),
                document_ref: "document:1".into(),
                source_revision_ref: "revision:1".into(),
                span_ref: "span:1".into(),
                literal_text: "Alice called Bob.".into(),
            },
            parse: Some(ParseTraceCoordinate {
                candidate_pnf_ref: "candidate:1".into(),
                parser_receipt_ref: Some("parser:1".into()),
                review_ref: Some("review:1".into()),
                admission_receipt_ref: Some("admission:1".into()),
                review_state: TraceReviewState::SemanticallyAdmitted,
            }),
            observation_ref: "observation:1".into(),
            event_ref: Some("event:1".into()),
            claim_refs: vec!["claim:1".into()],
            downstream_use_refs: vec!["issue:1".into()],
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn trace_walks_event_to_literal_source_and_back_without_promotion() {
        let trace = trace();
        trace.validate().unwrap();
        assert_eq!(
            trace.reverse_source_chain(),
            vec!["observation:1", "candidate:1", "statement:1", "span:1", "revision:1"]
        );
        assert_eq!(
            trace.forward_downstream_chain(),
            vec![
                "revision:1",
                "span:1",
                "statement:1",
                "candidate:1",
                "observation:1",
                "event:1",
                "claim:1",
                "issue:1",
            ]
        );
        assert!(!trace.creates_evidence_payment());
        assert!(!trace.creates_semantic_authority);
        assert!(!trace.applicability_promoted);
        assert!(!trace.claim_truth_promoted);
    }

    #[test]
    fn admitted_trace_requires_admission_receipt() {
        let mut trace = trace();
        trace.parse.as_mut().unwrap().admission_receipt_ref = None;
        assert_eq!(
            trace.validate(),
            Err(SemanticTraceError::MissingAdmissionReceipt)
        );
    }
}
