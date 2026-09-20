//! Explicit Waltons proposition-evidence review gate.
//!
//! This module takes source-located OALC paragraph candidates and an explicit
//! reviewer decision, then emits the existing reviewed-evidence payment
//! substrate. Paying the evidence-coordinate obligation does not promote claim
//! truth, applicability, or semantic/legal authority.

use serde::{Deserialize, Serialize};
use sensiblaw_consumer_residual::{
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, RequirementNeed, RequirementScope,
};
use sensiblaw_core::canonical_evidence::{EvidenceObservation, EvidenceSpan};
use sensiblaw_reviewed_evidence_payment::{
    compile_reviewed_evidence_payment, ReviewedCanonicalEvidence, ReviewedEvidenceCoordinate,
    ReviewedEvidencePaymentError, ReviewedEvidencePaymentReceipt,
};

use crate::oalc_judgment_materialization::{
    JudgmentParagraphCandidate, OalcJudgmentMaterialisation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EstoppelRequirementRole {
    AssumptionOrExpectation,
    Reliance,
    Detriment,
    Unconscionability,
}

impl EstoppelRequirementRole {
    pub const fn requirement_ref(self) -> &'static str {
        match self {
            Self::AssumptionOrExpectation => "requirement:estoppel:assumption",
            Self::Reliance => "requirement:estoppel:reliance",
            Self::Detriment => "requirement:estoppel:detriment",
            Self::Unconscionability => "requirement:estoppel:unconscionability",
        }
    }

    pub const fn proposition_ref(self) -> &'static str {
        match self {
            Self::AssumptionOrExpectation => "prop:estoppel:assumption-or-expectation",
            Self::Reliance => "prop:estoppel:reliance",
            Self::Detriment => "prop:estoppel:detriment",
            Self::Unconscionability => "prop:estoppel:unconscionability",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PropositionEvidenceDisposition {
    Supports,
    Contests,
    ContextOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewedWaltonsParagraphDecision {
    pub paragraph_locator_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_sha256: String,
    pub role: EstoppelRequirementRole,
    pub disposition: PropositionEvidenceDisposition,
    pub reviewer_ref: String,
    pub review_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedWaltonsPropositionEvidenceReceipt {
    pub role: EstoppelRequirementRole,
    pub proposition_ref: String,
    pub paragraph_locator_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_sha256: String,
    pub disposition: PropositionEvidenceDisposition,
    pub observation_ref: String,
    pub review_ref: String,
    pub manifestation_ref: String,
    pub reviewed_evidence: Option<ReviewedCanonicalEvidence>,
    pub payment_receipt: Option<ReviewedEvidencePaymentReceipt>,
    pub payment_bytes: Vec<u8>,
    pub reviewer_ref: String,
    pub review_evidence_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaltonsParagraphReviewError {
    ParagraphNotFound,
    SourceRevisionMismatch,
    DigestMismatch,
    RequirementNotMatched,
    MissingReviewer,
    MissingReviewEvidence,
    Evidence(String),
    Payment(String),
}

impl From<ReviewedEvidencePaymentError> for WaltonsParagraphReviewError {
    fn from(value: ReviewedEvidencePaymentError) -> Self {
        Self::Payment(format!("{value:?}"))
    }
}

fn paragraph_for_decision<'a>(
    materialization: &'a OalcJudgmentMaterialisation,
    decision: &ReviewedWaltonsParagraphDecision,
) -> Result<&'a JudgmentParagraphCandidate, WaltonsParagraphReviewError> {
    let paragraph = materialization
        .paragraph_candidates
        .iter()
        .find(|paragraph| paragraph.paragraph_locator_ref == decision.paragraph_locator_ref)
        .ok_or(WaltonsParagraphReviewError::ParagraphNotFound)?;

    if paragraph.source_revision_ref != decision.source_revision_ref
        || materialization.source_revision_ref != decision.source_revision_ref
    {
        return Err(WaltonsParagraphReviewError::SourceRevisionMismatch);
    }
    if paragraph.canonical_text_sha256 != decision.canonical_text_sha256
        || materialization.canonical_text_sha256 != decision.canonical_text_sha256
    {
        return Err(WaltonsParagraphReviewError::DigestMismatch);
    }
    if !paragraph
        .matched_research_criterion_refs
        .iter()
        .any(|value| value == decision.role.requirement_ref())
    {
        return Err(WaltonsParagraphReviewError::RequirementNotMatched);
    }
    Ok(paragraph)
}

pub fn waltons_estoppel_research_criteria(
) -> Vec<crate::oalc_judgment_materialization::ParagraphResearchCriterion> {
    use crate::oalc_judgment_materialization::ParagraphResearchCriterion;
    vec![
        ParagraphResearchCriterion {
            criterion_ref: "requirement:estoppel:assumption".into(),
            needles: vec![
                "assumption".into(),
                "expectation".into(),
                "representation".into(),
            ],
        },
        ParagraphResearchCriterion {
            criterion_ref: "requirement:estoppel:reliance".into(),
            needles: vec!["reliance".into(), "relied".into(), "acted".into()],
        },
        ParagraphResearchCriterion {
            criterion_ref: "requirement:estoppel:detriment".into(),
            needles: vec!["detriment".into(), "detrimental".into()],
        },
        ParagraphResearchCriterion {
            criterion_ref: "requirement:estoppel:unconscionability".into(),
            needles: vec!["unconscionable".into(), "unconscionability".into()],
        },
    ]
}

pub fn waltons_estoppel_consumer_spec(
    materialization: &OalcJudgmentMaterialisation,
) -> ConsumerSpec {
    let manifestation_ref = format!("manifestation:{}", materialization.source_revision_ref);
    ConsumerSpec {
        consumer_id: "consumer:waltons-estoppel-materialisation".into(),
        surface_id: "surface:contract:estoppel".into(),
        requirements: [
            EstoppelRequirementRole::AssumptionOrExpectation,
            EstoppelRequirementRole::Reliance,
            EstoppelRequirementRole::Detriment,
            EstoppelRequirementRole::Unconscionability,
        ]
        .into_iter()
        .map(|role| ConsumerRequirement {
            requirement_id: role.requirement_ref().into(),
            need: RequirementNeed::EvidenceCoordinate(EvidenceCoordinateKind::Authority),
            scope: RequirementScope::SourceManifestation(manifestation_ref.clone()),
        })
        .collect(),
    }
}

pub fn compile_reviewed_waltons_paragraph(
    materialization: &OalcJudgmentMaterialisation,
    decision: &ReviewedWaltonsParagraphDecision,
    iteration_index: i64,
) -> Result<ReviewedWaltonsPropositionEvidenceReceipt, WaltonsParagraphReviewError> {
    if decision.reviewer_ref.trim().is_empty() {
        return Err(WaltonsParagraphReviewError::MissingReviewer);
    }
    if decision.review_evidence_refs.is_empty()
        || decision.review_evidence_refs.iter().any(|value| value.trim().is_empty())
    {
        return Err(WaltonsParagraphReviewError::MissingReviewEvidence);
    }
    let paragraph = paragraph_for_decision(materialization, decision)?;
    let span_ref = paragraph.paragraph_locator_ref.clone();
    let observation_ref = format!(
        "observation:waltons:{}:{}",
        decision.role.requirement_ref(),
        paragraph.paragraph_ordinal
    );
    let observation = EvidenceObservation {
        observation_ref: observation_ref.clone(),
        source_revision_ref: materialization.source_revision_ref.clone(),
        span: EvidenceSpan::structured(
            &materialization.source_revision_ref,
            &span_ref,
            paragraph
                .reported_paragraph_label
                .clone()
                .unwrap_or_else(|| paragraph.paragraph_locator_ref.clone()),
        )
        .map_err(|error| WaltonsParagraphReviewError::Evidence(format!("{error:?}")))?,
        predicate_ref: format!("predicate:reviewed-source-support:{}", decision.role.requirement_ref()),
        value_ref: decision.role.proposition_ref().into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    observation
        .validate()
        .map_err(|error| WaltonsParagraphReviewError::Evidence(format!("{error:?}")))?;

    let manifestation_ref = format!("manifestation:{}", materialization.source_revision_ref);
    let review = ReviewedEvidenceCoordinate {
        review_ref: format!(
            "review:waltons:{}:{}",
            decision.role.requirement_ref(),
            paragraph.paragraph_ordinal
        ),
        consumer_id: "consumer:waltons-estoppel-materialisation".into(),
        requirement_id: decision.role.requirement_ref().into(),
        coordinate: EvidenceCoordinateKind::Authority,
        source_ref: Some(manifestation_ref.clone()),
        evidence_ref: observation_ref,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };

    let spec = waltons_estoppel_consumer_spec(materialization);
    let mut payment_bytes = Vec::new();
    let (reviewed_evidence, payment_receipt) =
        if decision.disposition == PropositionEvidenceDisposition::Supports {
            let reviewed = ReviewedCanonicalEvidence::from_reviewed_coordinate(
                &review,
                observation,
                format!(
                    "payment:waltons:{}:{}",
                    decision.role.requirement_ref(),
                    paragraph.paragraph_ordinal
                ),
            )
            .map_err(|error| WaltonsParagraphReviewError::Evidence(format!("{error:?}")))?;
            let payment = compile_reviewed_evidence_payment(
                &spec,
                &review,
                &mut payment_bytes,
                iteration_index,
            )?;
            (Some(reviewed), Some(payment))
        } else {
            (None, None)
        };

    Ok(ReviewedWaltonsPropositionEvidenceReceipt {
        role: decision.role,
        proposition_ref: decision.role.proposition_ref().into(),
        paragraph_locator_ref: paragraph.paragraph_locator_ref.clone(),
        source_revision_ref: paragraph.source_revision_ref.clone(),
        canonical_text_sha256: paragraph.canonical_text_sha256.clone(),
        disposition: decision.disposition,
        observation_ref: review.evidence_ref.clone(),
        review_ref: review.review_ref.clone(),
        manifestation_ref,
        reviewed_evidence,
        payment_receipt,
        payment_bytes,
        reviewer_ref: decision.reviewer_ref.clone(),
        review_evidence_refs: decision.review_evidence_refs.clone(),
        candidate_only: true,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

pub const fn reviewed_waltons_evidence_is_proposition_truth(
    _receipt: &ReviewedWaltonsPropositionEvidenceReceipt,
) -> bool {
    false
}

pub const fn reviewed_waltons_evidence_is_current_binding_authority(
    _receipt: &ReviewedWaltonsPropositionEvidenceReceipt,
) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oalc_judgment_materialization::{
        materialize_oalc_judgment, ParagraphResearchCriterion,
    };
    use sensiblaw_governed_legal_provider::{
        OalcResolvedSourceReceipt, OalcTemporalCoverage, OALC_RECEIPT_AUTHORITY,
    };
    use sha2::{Digest, Sha256};
    use std::path::PathBuf;

    fn sha256(text: &str) -> String {
        format!("sha256:{:x}", Sha256::digest(text.as_bytes()))
    }

    fn fixture(text: &str) -> OalcJudgmentMaterialisation {
        let receipt = OalcResolvedSourceReceipt {
            demand_ref: "contract-follow:waltons:hca7".into(),
            origin_ref: "doctrine:au:contract:estoppel".into(),
            citation: "[1988] HCA 7".into(),
            version_id: "high_court_of_australia:1988-hca-7".into(),
            corpus_revision_ref: "isaacus/open-australian-legal-corpus@fixture".into(),
            source: "high_court_of_australia".into(),
            jurisdiction: "commonwealth".into(),
            document_type: "decision".into(),
            court: Some("court:HCA".into()),
            date: Some("1988-02-19".into()),
            canonical_url: None,
            when_scraped: None,
            canonical_text_digest: sha256(text),
            local_artifact_ref: PathBuf::from("waltons.txt"),
            temporal_coverage: OalcTemporalCoverage::DecisionDateAnchored,
            resolution_path: "fixture".into(),
            network_requests: 0,
            stream_rows_examined: None,
            stream_bytes_read: None,
            stream_terminated_after_match: None,
            stream_uniqueness_exhaustively_verified: None,
            range_index_hit: None,
            range_index_requests: None,
            range_index_rows_indexed_this_run: None,
            range_index_bytes_indexed_this_run: None,
            range_index_byte_start: None,
            range_index_byte_len: None,
            receipt_authority: OALC_RECEIPT_AUTHORITY.into(),
            candidate_only: true,
            creates_legal_authority: false,
            creates_claim_truth: false,
        };
        materialize_oalc_judgment(
            &receipt,
            text,
            &[ParagraphResearchCriterion {
                criterion_ref: "requirement:estoppel:reliance".into(),
                needles: vec!["reliance".into()],
            }],
        )
        .unwrap()
    }

    #[test]
    fn explicit_review_can_pay_authority_coordinate_without_promoting_truth() {
        let materialization = fixture("[1] Reliance was central to the estoppel argument.\n");
        let paragraph = &materialization.paragraph_candidates[0];
        let decision = ReviewedWaltonsParagraphDecision {
            paragraph_locator_ref: paragraph.paragraph_locator_ref.clone(),
            source_revision_ref: paragraph.source_revision_ref.clone(),
            canonical_text_sha256: paragraph.canonical_text_sha256.clone(),
            role: EstoppelRequirementRole::Reliance,
            disposition: PropositionEvidenceDisposition::Supports,
            reviewer_ref: "reviewer:fixture".into(),
            review_evidence_refs: vec!["review-note:fixture".into()],
        };
        let receipt = compile_reviewed_waltons_paragraph(&materialization, &decision, 1).unwrap();
        assert_eq!(receipt.payment_receipt.as_ref().unwrap().payments_emitted, 2);
        assert!(receipt.payment_receipt.as_ref().unwrap().review_emitted);
        assert!(receipt.reviewed_evidence.is_some());
        assert!(receipt.candidate_only);
        assert!(!receipt.creates_legal_authority);
        assert!(!receipt.claim_truth_promoted);
        assert!(!reviewed_waltons_evidence_is_proposition_truth(&receipt));
    }

    #[test]
    fn contested_or_context_evidence_does_not_contract_the_evidence_frontier() {
        let materialization = fixture("[1] Reliance was discussed.\n");
        let paragraph = &materialization.paragraph_candidates[0];
        for disposition in [
            PropositionEvidenceDisposition::Contests,
            PropositionEvidenceDisposition::ContextOnly,
        ] {
            let decision = ReviewedWaltonsParagraphDecision {
                paragraph_locator_ref: paragraph.paragraph_locator_ref.clone(),
                source_revision_ref: paragraph.source_revision_ref.clone(),
                canonical_text_sha256: paragraph.canonical_text_sha256.clone(),
                role: EstoppelRequirementRole::Reliance,
                disposition,
                reviewer_ref: "reviewer:fixture".into(),
                review_evidence_refs: vec!["review-note:fixture".into()],
            };
            let receipt =
                compile_reviewed_waltons_paragraph(&materialization, &decision, 1).unwrap();
            assert!(receipt.payment_receipt.is_none());
            assert!(receipt.reviewed_evidence.is_none());
            assert!(receipt.payment_bytes.is_empty());
            assert!(!receipt.claim_truth_promoted);
        }
    }

    #[test]
    fn reviewer_cannot_pay_a_role_not_matched_by_the_paragraph() {
        let materialization = fixture("[1] Reliance was discussed.\n");
        let paragraph = &materialization.paragraph_candidates[0];
        let decision = ReviewedWaltonsParagraphDecision {
            paragraph_locator_ref: paragraph.paragraph_locator_ref.clone(),
            source_revision_ref: paragraph.source_revision_ref.clone(),
            canonical_text_sha256: paragraph.canonical_text_sha256.clone(),
            role: EstoppelRequirementRole::Detriment,
            disposition: PropositionEvidenceDisposition::Supports,
            reviewer_ref: "reviewer:fixture".into(),
            review_evidence_refs: vec!["review-note:fixture".into()],
        };
        assert_eq!(
            compile_reviewed_waltons_paragraph(&materialization, &decision, 1),
            Err(WaltonsParagraphReviewError::RequirementNotMatched)
        );
    }
}