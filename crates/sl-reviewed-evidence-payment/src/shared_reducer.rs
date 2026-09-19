use sensiblaw_core::canonical_evidence::{EvidenceObservation, EvidenceSubstrateError};

use crate::ReviewedEvidenceCoordinate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedCanonicalEvidence {
    pub reviewed_evidence_ref: String,
    pub observation: EvidenceObservation,
    pub review_ref: String,
    pub payment_ref: String,
    pub consumer_id: String,
    pub requirement_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectionFamily {
    World,
    Matter,
    Legal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionDisposition {
    Produced { delta_ref: String },
    Abstained { reason_ref: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalProjectionReceipt {
    pub family: ProjectionFamily,
    pub disposition: ProjectionDisposition,
    pub reviewed_evidence_ref: String,
    pub observation_ref: String,
    pub source_revision_ref: String,
    pub span_ref: String,
    pub review_ref: String,
    pub payment_ref: String,
    pub candidate_only: bool,
    pub semantic_promotion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedEvidenceReductionReceipt {
    pub reviewed_evidence_ref: String,
    pub observation_ref: String,
    pub source_revision_ref: String,
    pub span_ref: String,
    pub review_ref: String,
    pub payment_ref: String,
    pub projections: Vec<CanonicalProjectionReceipt>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedEvidenceReducerError {
    EmptyCoordinate(&'static str),
    EvidenceReferenceMismatch,
    ReviewedEvidenceIdentityMismatch,
    ReviewPromotionNotAllowed,
    CanonicalEvidence(EvidenceSubstrateError),
    ProjectionFamilyMismatch {
        expected: ProjectionFamily,
        actual: ProjectionFamily,
    },
    ProjectionFailed {
        family: ProjectionFamily,
        message: String,
    },
}

impl From<EvidenceSubstrateError> for SharedEvidenceReducerError {
    fn from(value: EvidenceSubstrateError) -> Self {
        Self::CanonicalEvidence(value)
    }
}

fn reviewed_evidence_ref(review_ref: &str, observation_ref: &str) -> String {
    format!("reviewed-canonical-evidence:{review_ref}:{observation_ref}")
}

impl ReviewedCanonicalEvidence {
    pub fn from_reviewed_coordinate(
        review: &ReviewedEvidenceCoordinate,
        observation: EvidenceObservation,
        payment_ref: impl Into<String>,
    ) -> Result<Self, SharedEvidenceReducerError> {
        observation.validate()?;
        if !review.candidate_only
            || review.creates_semantic_authority
            || review.applicability_promoted
            || review.claim_truth_promoted
        {
            return Err(SharedEvidenceReducerError::ReviewPromotionNotAllowed);
        }
        if review.evidence_ref != observation.observation_ref {
            return Err(SharedEvidenceReducerError::EvidenceReferenceMismatch);
        }

        let payment_ref = payment_ref.into();
        for (name, value) in [
            ("review_ref", review.review_ref.as_str()),
            ("consumer_id", review.consumer_id.as_str()),
            ("requirement_id", review.requirement_id.as_str()),
            ("payment_ref", payment_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(SharedEvidenceReducerError::EmptyCoordinate(name));
            }
        }

        Ok(Self {
            reviewed_evidence_ref: reviewed_evidence_ref(
                &review.review_ref,
                &observation.observation_ref,
            ),
            observation,
            review_ref: review.review_ref.clone(),
            payment_ref,
            consumer_id: review.consumer_id.clone(),
            requirement_id: review.requirement_id.clone(),
        })
    }

    pub fn validate(&self) -> Result<(), SharedEvidenceReducerError> {
        self.observation.validate()?;
        let expected_reviewed_evidence_ref =
            reviewed_evidence_ref(&self.review_ref, &self.observation.observation_ref);
        if self.reviewed_evidence_ref != expected_reviewed_evidence_ref {
            return Err(SharedEvidenceReducerError::ReviewedEvidenceIdentityMismatch);
        }

        for (name, value) in [
            ("reviewed_evidence_ref", self.reviewed_evidence_ref.as_str()),
            ("review_ref", self.review_ref.as_str()),
            ("payment_ref", self.payment_ref.as_str()),
            ("consumer_id", self.consumer_id.as_str()),
            ("requirement_id", self.requirement_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(SharedEvidenceReducerError::EmptyCoordinate(name));
            }
        }
        Ok(())
    }
}

pub trait CanonicalEvidenceProjection {
    fn family(&self) -> ProjectionFamily;

    fn project(
        &self,
        evidence: &ReviewedCanonicalEvidence,
    ) -> Result<ProjectionDisposition, String>;
}

fn project_one(
    expected: ProjectionFamily,
    projector: &dyn CanonicalEvidenceProjection,
    evidence: &ReviewedCanonicalEvidence,
) -> Result<CanonicalProjectionReceipt, SharedEvidenceReducerError> {
    let actual = projector.family();
    if actual != expected {
        return Err(SharedEvidenceReducerError::ProjectionFamilyMismatch { expected, actual });
    }
    let disposition = projector.project(evidence).map_err(|message| {
        SharedEvidenceReducerError::ProjectionFailed {
            family: expected,
            message,
        }
    })?;

    match &disposition {
        ProjectionDisposition::Produced { delta_ref } if delta_ref.trim().is_empty() => {
            return Err(SharedEvidenceReducerError::EmptyCoordinate("delta_ref"));
        }
        ProjectionDisposition::Abstained { reason_ref } if reason_ref.trim().is_empty() => {
            return Err(SharedEvidenceReducerError::EmptyCoordinate("reason_ref"));
        }
        _ => {}
    }

    Ok(CanonicalProjectionReceipt {
        family: expected,
        disposition,
        reviewed_evidence_ref: evidence.reviewed_evidence_ref.clone(),
        observation_ref: evidence.observation.observation_ref.clone(),
        source_revision_ref: evidence.observation.source_revision_ref.clone(),
        span_ref: evidence.observation.span.span_ref.clone(),
        review_ref: evidence.review_ref.clone(),
        payment_ref: evidence.payment_ref.clone(),
        candidate_only: true,
        semantic_promotion: false,
    })
}

pub fn reduce_reviewed_canonical_evidence(
    evidence: &ReviewedCanonicalEvidence,
    world: Option<&dyn CanonicalEvidenceProjection>,
    matter: Option<&dyn CanonicalEvidenceProjection>,
    legal: Option<&dyn CanonicalEvidenceProjection>,
) -> Result<SharedEvidenceReductionReceipt, SharedEvidenceReducerError> {
    evidence.validate()?;

    let mut projections = Vec::new();
    for (family, projector) in [
        (ProjectionFamily::World, world),
        (ProjectionFamily::Matter, matter),
        (ProjectionFamily::Legal, legal),
    ] {
        if let Some(projector) = projector {
            projections.push(project_one(family, projector, evidence)?);
        }
    }

    Ok(SharedEvidenceReductionReceipt {
        reviewed_evidence_ref: evidence.reviewed_evidence_ref.clone(),
        observation_ref: evidence.observation.observation_ref.clone(),
        source_revision_ref: evidence.observation.source_revision_ref.clone(),
        span_ref: evidence.observation.span.span_ref.clone(),
        review_ref: evidence.review_ref.clone(),
        payment_ref: evidence.payment_ref.clone(),
        projections,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_consumer_residual::EvidenceCoordinateKind;
    use sensiblaw_core::canonical_evidence::EvidenceSpan;

    fn observation() -> EvidenceObservation {
        EvidenceObservation {
            observation_ref: "observation:fixture".into(),
            source_revision_ref: "revision:fixture".into(),
            span: EvidenceSpan::text(
                "revision:fixture",
                "span:fixture:0-12",
                0,
                12,
            )
            .unwrap(),
            predicate_ref: "predicate:fixture".into(),
            value_ref: "value:fixture".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn review() -> ReviewedEvidenceCoordinate {
        ReviewedEvidenceCoordinate {
            review_ref: "review:fixture".into(),
            consumer_id: "consumer:fixture".into(),
            requirement_id: "requirement:fixture".into(),
            coordinate: EvidenceCoordinateKind::Mechanism,
            source_ref: Some("manifestation:fixture".into()),
            evidence_ref: "observation:fixture".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    struct FixedProjection {
        family: ProjectionFamily,
        disposition: ProjectionDisposition,
    }

    impl CanonicalEvidenceProjection for FixedProjection {
        fn family(&self) -> ProjectionFamily {
            self.family
        }

        fn project(
            &self,
            _evidence: &ReviewedCanonicalEvidence,
        ) -> Result<ProjectionDisposition, String> {
            Ok(self.disposition.clone())
        }
    }

    #[test]
    fn all_projection_receipts_retain_one_reviewed_canonical_identity() {
        let evidence = ReviewedCanonicalEvidence::from_reviewed_coordinate(
            &review(),
            observation(),
            "payment:fixture",
        )
        .unwrap();
        let world = FixedProjection {
            family: ProjectionFamily::World,
            disposition: ProjectionDisposition::Produced {
                delta_ref: "world-delta:fixture".into(),
            },
        };
        let matter = FixedProjection {
            family: ProjectionFamily::Matter,
            disposition: ProjectionDisposition::Produced {
                delta_ref: "matter-delta:fixture".into(),
            },
        };
        let legal = FixedProjection {
            family: ProjectionFamily::Legal,
            disposition: ProjectionDisposition::Produced {
                delta_ref: "legal-delta:fixture".into(),
            },
        };

        let receipt = reduce_reviewed_canonical_evidence(
            &evidence,
            Some(&world),
            Some(&matter),
            Some(&legal),
        )
        .unwrap();

        assert_eq!(receipt.projections.len(), 3);
        for projection in &receipt.projections {
            assert_eq!(projection.reviewed_evidence_ref, evidence.reviewed_evidence_ref);
            assert_eq!(projection.observation_ref, evidence.observation.observation_ref);
            assert_eq!(projection.source_revision_ref, evidence.observation.source_revision_ref);
            assert_eq!(projection.span_ref, evidence.observation.span.span_ref);
            assert_eq!(projection.review_ref, evidence.review_ref);
            assert_eq!(projection.payment_ref, evidence.payment_ref);
            assert!(projection.candidate_only);
            assert!(!projection.semantic_promotion);
        }
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.applicability_promoted);
        assert!(!receipt.claim_truth_promoted);
    }

    #[test]
    fn projection_may_abstain_without_fabricating_a_delta() {
        let evidence = ReviewedCanonicalEvidence::from_reviewed_coordinate(
            &review(),
            observation(),
            "payment:fixture",
        )
        .unwrap();
        let legal = FixedProjection {
            family: ProjectionFamily::Legal,
            disposition: ProjectionDisposition::Abstained {
                reason_ref: "not-applicable:legal".into(),
            },
        };

        let receipt =
            reduce_reviewed_canonical_evidence(&evidence, None, None, Some(&legal)).unwrap();
        assert!(matches!(
            receipt.projections[0].disposition,
            ProjectionDisposition::Abstained { .. }
        ));
    }

    #[test]
    fn review_must_name_the_exact_canonical_observation() {
        let mut review = review();
        review.evidence_ref = "observation:other".into();
        assert_eq!(
            ReviewedCanonicalEvidence::from_reviewed_coordinate(
                &review,
                observation(),
                "payment:fixture",
            ),
            Err(SharedEvidenceReducerError::EvidenceReferenceMismatch)
        );
    }

    #[test]
    fn reviewed_evidence_identity_cannot_be_rewritten_after_construction() {
        let mut evidence = ReviewedCanonicalEvidence::from_reviewed_coordinate(
            &review(),
            observation(),
            "payment:fixture",
        )
        .unwrap();
        evidence.reviewed_evidence_ref = "reviewed-canonical-evidence:rewritten".into();

        assert_eq!(
            evidence.validate(),
            Err(SharedEvidenceReducerError::ReviewedEvidenceIdentityMismatch)
        );
    }

    #[test]
    fn projection_family_cannot_bypass_its_shared_slot() {
        let evidence = ReviewedCanonicalEvidence::from_reviewed_coordinate(
            &review(),
            observation(),
            "payment:fixture",
        )
        .unwrap();
        let wrong = FixedProjection {
            family: ProjectionFamily::Legal,
            disposition: ProjectionDisposition::Produced {
                delta_ref: "legal-delta:fixture".into(),
            },
        };

        assert!(matches!(
            reduce_reviewed_canonical_evidence(&evidence, Some(&wrong), None, None),
            Err(SharedEvidenceReducerError::ProjectionFamilyMismatch {
                expected: ProjectionFamily::World,
                actual: ProjectionFamily::Legal
            })
        ));
    }
}
