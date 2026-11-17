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
        let mut reviewed = BTreeSet::new();
        for review in &self.reviews {
            if review.review_ref.trim().is_empty() || review.candidate_ref.trim().is_empty() {
                return Err(StatementPnfSpineError::EmptyReviewCoordinate);
            }
            if !available.contains(review.candidate_ref.as_str()) {
                return Err(StatementPnfSpineError::UnknownCandidateReview);
            }
            // A candidate cannot simultaneously have incompatible parse-review
            // dispositions. A later review belongs in a new reviewed revision.
            if !reviewed.insert(review.candidate_ref.clone()) {
                return Err(StatementPnfSpineError::DuplicateCandidateReview);
            }
            if matches!(review.disposition, ParseReviewDisposition::Qualified)
                && review
                    .qualification_ref
                    .as_deref()
                    .map_or(true, |value| value.trim().is_empty())
            {
                return Err(StatementPnfSpineError::MissingQualification);
            }
        }
        let unreviewed: BTreeSet<String> = available
            .iter()
            .filter(|candidate_ref| !reviewed.contains(*candidate_ref))
            .map(|candidate_ref| (*candidate_ref).to_owned())
            .collect();
        if self.reviewed_candidate_refs != reviewed
            || self.retained_unreviewed_candidate_refs != unreviewed
        {
            return Err(StatementPnfSpineError::ReviewIndexMismatch);
        }
        Ok(())
    }

    /// Only an explicit accepted parse review may enter an admission-review
    /// worklist. This is not semantic admission, a world-identity resolution,
    /// a legal proposition, or a formal-proof demand.
    pub fn accepted_for_admission_review_refs(
        &self,
    ) -> Result<BTreeSet<String>, StatementPnfSpineError> {
        self.validate()?;
        Ok(self
            .reviews
            .iter()
            .filter(|review| matches!(review.disposition, ParseReviewDisposition::AcceptedForAdmissionReview))
            .map(|review| review.candidate_ref.clone())
            .collect())
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
    #[error("a candidate has more than one parse review in the same revision")]
    DuplicateCandidateReview,
    #[error("stored reviewed/unreviewed candidate indices disagree with source reviews")]
    ReviewIndexMismatch,
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

/// Checked M12 handoff for the database-native parser. Reuses the ordinary
/// source->candidate PNF compiler, then reopens the complete parser region.
/// No additional semantic authority or legal proposition is generated.
pub fn compile_statement_pnf_with_parser_evidence(
    snapshot: &crate::DbNativeParserSnapshot,
    statement: SourceStatementEnvelope,
    evidence: &crate::ParserEvidenceRegion,
    parser_receipt_ref: impl Into<String>,
) -> Result<StatementCandidatePnf, StatementPnfSpineError> {
    if evidence.parser_run_ref() != snapshot.parser_run_ref()
        || evidence.source_revision_ref() != snapshot.source_revision_ref()
        || evidence.source_revision_ref() != statement.source_revision_ref.as_str()
    {
        return Err(StatementPnfSpineError::CandidatePnf(
            crate::CandidatePnfError::PersistedParserResidual {
                span_ref: statement.span.span_ref.clone(),
                error_ref: "parser-evidence-revision-or-run-mismatch".into(),
            },
        ));
    }
    if !evidence.matches_source_literal(&statement.literal_text) {
        return Err(StatementPnfSpineError::CandidatePnf(
            CandidatePnfError::PersistedParserResidual {
                span_ref: statement.span.span_ref.clone(),
                error_ref: "parser-evidence-source-literal-mismatch-or-missing-product-key".into(),
            },
        ));
    }
    if !evidence.matches_observation_coverage(&statement.literal_text) {
        return Err(StatementPnfSpineError::CandidatePnf(
            CandidatePnfError::PersistedParserResidual {
                span_ref: statement.span.span_ref.clone(),
                error_ref: "parser-evidence-token-head-entity-coverage-mismatch".into(),
            },
        ));
    }
    let parser_receipt_ref = parser_receipt_ref.into();
    let expected_receipt = format!(
        "db-parser:{}:{}", snapshot.parser_run_ref(), statement.span.span_ref
    );
    if parser_receipt_ref != expected_receipt {
        return Err(StatementPnfSpineError::CandidatePnf(
            CandidatePnfError::PersistedParserResidual {
                span_ref: statement.span.span_ref.clone(),
                error_ref: "parser-evidence-receipt-mismatch".into(),
            },
        ));
    }
    let compiled = compile_statement_pnf(snapshot, statement, parser_receipt_ref)?;
    crate::audit_candidate_pnf_evidence(
        &compiled.statement.span,
        &compiled.statement.source_revision_ref,
        &compiled.pnf,
        evidence,
    )?;
    Ok(compiled)
}

/// Source-bound PG entry point: load original parser observations,
/// run the existing M12 candidate compiler and enforce exact parity.
#[derive(Debug, thiserror::Error)]
pub enum DbStatementPnfCheckedError {
    #[error("parser evidence load failed: {0}")]
    Parser(#[from] crate::DbNativeParserError),
    #[error("M12 candidate/parity check failed: {0}")]
    Statement(#[from] StatementPnfSpineError),
}

pub fn compile_db_statement_pnf_checked(
    config: &crate::DatabaseConfig,
    snapshot: &crate::DbNativeParserSnapshot,
    statement: SourceStatementEnvelope,
) -> Result<(StatementCandidatePnf, crate::ParserEvidenceRegion), DbStatementPnfCheckedError> {
    let evidence = crate::load_parser_evidence_region(
        config, snapshot.parser_run_ref(), &statement.span.span_ref,
    )?;
    let receipt = format!(
        "db-parser:{}:{}", snapshot.parser_run_ref(), statement.span.span_ref
    );
    let compiled = compile_statement_pnf_with_parser_evidence(
        snapshot, statement, &evidence, receipt,
    )?;
    Ok((compiled, evidence))
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
    fn forged_review_indices_cannot_enter_admission_worklist() {
        let candidate = compile_initial_intake_statement(
            &parser(), statement(StatementOrigin::InitialIntake), "parse:1"
        ).unwrap();
        let mut reviewed = review_statement_parse(candidate, vec![]).unwrap();
        let forged = reviewed.candidate.pnf.candidates[0].candidate_ref.clone();
        reviewed.reviewed_candidate_refs.insert(forged.clone());
        assert!(matches!(
            reviewed.accepted_for_admission_review_refs(),
            Err(StatementPnfSpineError::ReviewIndexMismatch)
        ));

        reviewed.reviewed_candidate_refs.clear();
        reviewed.retained_unreviewed_candidate_refs.remove(&forged);
        assert!(matches!(
            reviewed.validate(),
            Err(StatementPnfSpineError::ReviewIndexMismatch)
        ));
    }

    #[test]
    fn conflicting_reviews_cannot_bypass_selection() {
        let candidate = compile_initial_intake_statement(
            &parser(), statement(StatementOrigin::InitialIntake), "parse:1"
        ).unwrap();
        let candidate_ref = candidate.pnf.candidates[0].candidate_ref.clone();
        let reviews = vec![
            StatementParseReview {
                review_ref: "review:a".into(),
                candidate_ref: candidate_ref.clone(),
                disposition: ParseReviewDisposition::AcceptedForAdmissionReview,
                qualification_ref: None,
            },
            StatementParseReview {
                review_ref: "review:b".into(),
                candidate_ref,
                disposition: ParseReviewDisposition::Rejected,
                qualification_ref: None,
            },
        ];
        assert!(matches!(
            review_statement_parse(candidate, reviews),
            Err(StatementPnfSpineError::DuplicateCandidateReview)
        ));
    }

    #[test]
    fn reviewed_does_not_mean_accepted_for_admission_review() {
        let candidate = compile_initial_intake_statement(
            &parser(), statement(StatementOrigin::InitialIntake), "parse:1"
        ).unwrap();
        let accepted_ref = candidate.pnf.candidates[0].candidate_ref.clone();
        let rejected_ref = candidate.pnf.candidates[1].candidate_ref.clone();
        let reviewed = review_statement_parse(candidate, vec![
            StatementParseReview {
                review_ref: "review:accepted".into(),
                candidate_ref: accepted_ref.clone(),
                disposition: ParseReviewDisposition::AcceptedForAdmissionReview,
                qualification_ref: None,
            },
            StatementParseReview {
                review_ref: "review:rejected".into(),
                candidate_ref: rejected_ref.clone(),
                disposition: ParseReviewDisposition::Rejected,
                qualification_ref: None,
            },
        ]).unwrap();
        let accepted = reviewed.accepted_for_admission_review_refs().unwrap();
        assert!(accepted.contains(&accepted_ref));
        assert!(!accepted.contains(&rejected_ref));
        assert!(reviewed.reviewed_candidate_refs.contains(&rejected_ref));
        assert!(!reviewed.semantic_admission_paid);
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
