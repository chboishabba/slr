//! M12 unified source -> statement -> PNF candidate spine.
//!
//! This is the production weld between exact source identity and the existing
//! CandidatePnfProducer surface.  Initial intake and research re-entry use the
//! same compiler.  Parser output remains candidate-only: parse review does not
//! pay proposition support, legal applicability, semantic admission, or claim
//! truth.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::{CandidatePnfBatch, CandidatePnfError, CandidatePnfProducer, ExactSourceSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementOrigin {
    InitialIntake,
    ResearchReentry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceStatementEnvelope {
    pub statement_ref: String,
    pub document_ref: String,
    pub source_revision_ref: String,
    pub span: ExactSourceSpan,
    pub literal_text: String,
    pub origin: StatementOrigin,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl SourceStatementEnvelope {
    pub fn validate(&self) -> Result<(), StatementPnfSpineError> {
        for (name, value) in [
            ("statement_ref", self.statement_ref.as_str()),
            ("document_ref", self.document_ref.as_str()),
            ("source_revision_ref", self.source_revision_ref.as_str()),
            ("span_ref", self.span.span_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(StatementPnfSpineError::EmptyCoordinate(name));
            }
        }
        if self.span.start_char >= self.span.end_char {
            return Err(StatementPnfSpineError::InvalidSpan);
        }
        if self.literal_text.is_empty() {
            return Err(StatementPnfSpineError::EmptyLiteralText);
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(StatementPnfSpineError::PromotionNotAllowed);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementCandidatePnf {
    pub statement: SourceStatementEnvelope,
    pub pnf: CandidatePnfBatch,
    pub parser_receipt_ref: String,
    pub candidate_only: bool,
    pub semantic_admission_paid: bool,
    pub proposition_support_paid: bool,
    pub applicability_paid: bool,
    pub claim_truth_paid: bool,
}

impl StatementCandidatePnf {
    pub fn validate(&self) -> Result<(), StatementPnfSpineError> {
        self.statement.validate()?;
        if self.parser_receipt_ref.trim().is_empty() {
            return Err(StatementPnfSpineError::EmptyCoordinate("parser_receipt_ref"));
        }
        if self.pnf.exact_span_ref != self.statement.span.span_ref {
            return Err(StatementPnfSpineError::SpanIdentityMismatch);
        }
        if !self.candidate_only
            || self.semantic_admission_paid
            || self.proposition_support_paid
            || self.applicability_paid
            || self.claim_truth_paid
            || self.pnf.proposition_support_paid
            || self.pnf.applicability_paid
            || self.pnf.claim_truth_paid
        {
            return Err(StatementPnfSpineError::PromotionNotAllowed);
        }
        for candidate in &self.pnf.candidates {
            if !candidate.candidate_only {
                return Err(StatementPnfSpineError::ParserCandidateWasPromoted);
            }
            if candidate.source_start_char < self.statement.span.start_char
                || candidate.source_end_char > self.statement.span.end_char
                || candidate.source_start_char >= candidate.source_end_char
            {
                return Err(StatementPnfSpineError::CandidateEscapesStatementSpan);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseReviewDisposition {
    AcceptedForAdmissionReview,
    Rejected,
    Abstained,
    Qualified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementParseReview {
    pub review_ref: String,
    pub candidate_ref: String,
    pub disposition: ParseReviewDisposition,
    pub qualification_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedStatementPnf {
    pub candidate: StatementCandidatePnf,
    pub reviews: Vec<StatementParseReview>,
    pub reviewed_candidate_refs: BTreeSet<String>,
    pub retained_unreviewed_candidate_refs: BTreeSet<String>,
    pub semantic_admission_paid: bool,
    pub proposition_support_paid: bool,
    pub applicability_paid: bool,
    pub claim_truth_paid: bool,
}

impl ReviewedStatementPnf {
    pub fn validate(&self) -> Result<(), StatementPnfSpineError> {
        self.candidate.validate()?;
        if self.semantic_admission_paid
            || self.proposition_support_paid
            || self.applicability_paid
            || self.claim_truth_paid
        {
            return Err(StatementPnfSpineError::PromotionNotAllowed);
        }
        let available: BTreeSet<&str> = self
            .candidate
            .pnf
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_ref.as_str())
            .collect();
        for review in &self.reviews {
            if review.review_ref.trim().is_empty() || review.candidate_ref.trim().is_empty() {
                return Err(StatementPnfSpineError::EmptyReviewCoordinate);
            }
            if !available.contains(review.candidate_ref.as_str()) {
                return Err(StatementPnfSpineError::UnknownCandidateReview);
            }
            if matches!(review.disposition, ParseReviewDisposition::Qualified)
                && review
                    .qualification_ref
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Err(StatementPnfSpineError::MissingQualification);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum StatementPnfSpineError {
    #[error("required statement coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("statement literal text is empty")]
    EmptyLiteralText,
    #[error("statement span must have start < end")]
    InvalidSpan,
    #[error("statement/PNF spine crossed a non-promotion boundary")]
    PromotionNotAllowed,
    #[error("candidate PNF exact span differs from statement exact span")]
    SpanIdentityMismatch,
    #[error("parser emitted a non-candidate factor")]
    ParserCandidateWasPromoted,
    #[error("parser candidate escapes the exact statement span")]
    CandidateEscapesStatementSpan,
    #[error("parse review contains an empty reference")]
    EmptyReviewCoordinate,
    #[error("parse review references a candidate absent from this statement")]
    UnknownCandidateReview,
    #[error("qualified parse review requires a qualification reference")]
    MissingQualification,
    #[error("statement origin does not match the requested compiler entry point")]
    OriginMismatch,
    #[error(transparent)]
    CandidatePnf(#[from] CandidatePnfError),
}

pub fn compile_statement_pnf<P: CandidatePnfProducer>(
    producer: &P,
    statement: SourceStatementEnvelope,
    parser_receipt_ref: impl Into<String>,
) -> Result<StatementCandidatePnf, StatementPnfSpineError> {
    statement.validate()?;
    let pnf = producer.produce(&statement.span)?;
    let compiled = StatementCandidatePnf {
        statement,
        pnf,
        parser_receipt_ref: parser_receipt_ref.into(),
        candidate_only: true,
        semantic_admission_paid: false,
        proposition_support_paid: false,
        applicability_paid: false,
        claim_truth_paid: false,
    };
    compiled.validate()?;
    Ok(compiled)
}

pub fn compile_initial_intake_statement<P: CandidatePnfProducer>(
    producer: &P,
    statement: SourceStatementEnvelope,
    parser_receipt_ref: impl Into<String>,
) -> Result<StatementCandidatePnf, StatementPnfSpineError> {
    if statement.origin != StatementOrigin::InitialIntake {
        return Err(StatementPnfSpineError::OriginMismatch);
    }
    compile_statement_pnf(producer, statement, parser_receipt_ref)
}

pub fn compile_research_reentry_statement<P: CandidatePnfProducer>(
    producer: &P,
    statement: SourceStatementEnvelope,
    parser_receipt_ref: impl Into<String>,
) -> Result<StatementCandidatePnf, StatementPnfSpineError> {
    if statement.origin != StatementOrigin::ResearchReentry {
        return Err(StatementPnfSpineError::OriginMismatch);
    }
    compile_statement_pnf(producer, statement, parser_receipt_ref)
}

pub fn review_statement_parse(
    candidate: StatementCandidatePnf,
    reviews: Vec<StatementParseReview>,
) -> Result<ReviewedStatementPnf, StatementPnfSpineError> {
    candidate.validate()?;

    let reviewed_candidate_refs: BTreeSet<String> =
        reviews.iter().map(|review| review.candidate_ref.clone()).collect();
    let retained_unreviewed_candidate_refs = candidate
        .pnf
        .candidates
        .iter()
        .map(|factor| factor.candidate_ref.clone())
        .filter(|candidate_ref| !reviewed_candidate_refs.contains(candidate_ref))
        .collect();

    let reviewed = ReviewedStatementPnf {
        candidate,
        reviews,
        reviewed_candidate_refs,
        retained_unreviewed_candidate_refs,
        semantic_admission_paid: false,
        proposition_support_paid: false,
        applicability_paid: false,
        claim_truth_paid: false,
    };
    reviewed.validate()?;
    Ok(reviewed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SpacyTsvAdapter;

    fn statement(origin: StatementOrigin) -> SourceStatementEnvelope {
        SourceStatementEnvelope {
            statement_ref: "statement:doc:rev1:0-20".into(),
            document_ref: "doc:1".into(),
            source_revision_ref: "revision:1".into(),
            span: ExactSourceSpan {
                span_ref: "span:doc:rev1:0-20".into(),
                start_char: 0,
                end_char: 20,
            },
            literal_text: "Alice called Bob.".into(),
            origin,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn parser() -> SpacyTsvAdapter<'static> {
        SpacyTsvAdapter::new(
            "T\t1\t0\t5\t0\tAlice\talice\tPROPN\t2\tnsubj\n\
             T\t2\t6\t12\t0\tcalled\tcall\tVERB\t2\tROOT\n\
             T\t3\t13\t16\t0\tBob\tbob\tPROPN\t2\tdobj",
        )
    }

    #[test]
    fn initial_and_reentry_use_the_same_candidate_compiler() {
        let intake =
            compile_initial_intake_statement(&parser(), statement(StatementOrigin::InitialIntake), "parse:1")
                .unwrap();
        let reentry = compile_research_reentry_statement(
            &parser(),
            statement(StatementOrigin::ResearchReentry),
            "parse:2",
        )
        .unwrap();

        assert_eq!(intake.pnf, reentry.pnf);
        assert!(!intake.semantic_admission_paid);
        assert!(!reentry.semantic_admission_paid);
    }

    #[test]
    fn parse_review_is_not_semantic_admission_or_truth() {
        let candidate =
            compile_initial_intake_statement(&parser(), statement(StatementOrigin::InitialIntake), "parse:1")
                .unwrap();
        let candidate_ref = candidate.pnf.candidates[0].candidate_ref.clone();
        let reviewed = review_statement_parse(
            candidate,
            vec![StatementParseReview {
                review_ref: "review:parse:1".into(),
                candidate_ref: candidate_ref.clone(),
                disposition: ParseReviewDisposition::AcceptedForAdmissionReview,
                qualification_ref: None,
            }],
        )
        .unwrap();

        assert!(reviewed.reviewed_candidate_refs.contains(&candidate_ref));
        assert!(!reviewed.semantic_admission_paid);
        assert!(!reviewed.proposition_support_paid);
        assert!(!reviewed.applicability_paid);
        assert!(!reviewed.claim_truth_paid);
    }

    #[test]
    fn wrong_origin_fails_closed() {
        assert!(matches!(
            compile_initial_intake_statement(
                &parser(),
                statement(StatementOrigin::ResearchReentry),
                "parse:wrong"
            ),
            Err(StatementPnfSpineError::OriginMismatch)
        ));
    }
}
