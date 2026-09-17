//! Preparation helpers for the explicit-review Mabo recurrent campaign.
//!
//! This module deliberately does no review inference. It takes one diagnosis
//! row plus one explicit operator-reviewed identity assignment, checks that the
//! pinned Wikidata manifestation and route are exactly those diagnosed, emits
//! the reviewed SameObject payment, and prepares the existing recurrent-cycle
//! ABI. Provider/network I/O remains with the executable.

use std::io::Cursor;

use sensiblaw_consumer_residual::EvidenceCoordinateKind;
use sensiblaw_pg_source_store::mabo_wikidata_property_ref;
use sensiblaw_proof_search_loop::frontier::ResidualStatus;
use sensiblaw_proof_search_loop::transition::ResidualAssessmentKind;
use sensiblaw_proof_search_loop::world_expansion::{
    DisambiguationOutcome, ResidualClass, ReviewDecision,
};
use sensiblaw_proof_search_loop::world_expansion_adapters::{
    from_wikidata_route, AcquiredWikidataEntity, ExpansionScoring,
};
use sensiblaw_proof_search_loop::world_expansion_reentry::PostAcquisitionWorldObservation;
use sensiblaw_proof_search_loop::world_expansion_runner::PreparedWorldExpansionCycle;
use sensiblaw_proof_search_loop::world_expansion_step::ResidualRouting;
use sensiblaw_proof_search_loop::world_identity::{
    WorldIdentityResolutionKind, WorldIdentityResolutionReceipt, WorldObjectIdentity,
};
use sensiblaw_proof_search_loop::world_observation::GetterBackend;
use sensiblaw_proof_search_loop::world_observation_adapters::wikidata_property_observation;
use sensiblaw_reviewed_evidence_payment::{
    compile_consumer_residual_stream_review_aware, compile_reviewed_evidence_payment,
    ReviewedEvidenceCoordinate,
};
use sensiblaw_route_selector::{RouteCandidate, RouteFamily};
use thiserror::Error;

use crate::{
    parse_mabo_wikidata_source_revision_ref, MaboConsumerDiagnosis, MaboPlannedIdentityReview,
    ReviewedPreparedCycle,
};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MaboReviewedCyclePreparationError {
    #[error("invalid exact Wikidata revision reference: {0}")]
    InvalidRevisionRef(String),
    #[error("reviewed diagnosis has multiple source revisions; acquisition must be exact")]
    AmbiguousSourceRevision,
    #[error("reviewed diagnosis does not contain a replayable bounded Mabo Wikidata relation")]
    MissingDiagnosedRelation,
    #[error("review assignment representation does not match diagnosis row")]
    ReviewRepresentationMismatch,
    #[error("diagnosis row is promoting or not candidate-only")]
    PromotingDiagnosisRow,
    #[error("diagnosis residual is missing or not open")]
    ResidualNotOpen,
    #[error("diagnosed source revision does not match acquired source")]
    SourceRevisionMismatch,
    #[error("route is not the diagnosed reviewed Wikidata relation")]
    RouteMismatch,
    #[error("world observation adapter failed: {0}")]
    ObservationAdapter(String),
    #[error("reviewed evidence payment failed: {0}")]
    ReviewedPayment(String),
    #[error("review-aware residual compilation did not pay exactly one diagnosed requirement")]
    RequirementNotPaid,
    #[error("world-expansion adapter failed: {0}")]
    ExpansionAdapter(String),
}

/// Exact provider/route request derived from one explicit reviewed diagnosis.
///
/// This is intentionally a pure projection. It does not fetch Wikidata and it
/// does not create review. The executable may use these exact coordinates for
/// provider acquisition and then must still pass the resulting artifact and
/// decoded route through `prepare_reviewed_mabo_identity_cycle`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedAcquisitionRequest {
    pub source_qid: String,
    pub revision_id: u64,
    pub source_revision_ref: String,
    pub property_ref: String,
    pub target_ref: String,
}

/// Parse only the exact manifestation form used by the reviewed-context
/// persistence lane: `wikidata:<QID>:oldid:<positive revision>`.
pub fn parse_wikidata_revision_ref(
    source_revision_ref: &str,
) -> Result<(String, u64), MaboReviewedCyclePreparationError> {
    parse_mabo_wikidata_source_revision_ref(source_revision_ref)
        .map(|rev| (rev.qid, rev.oldid))
        .map_err(|_| {
            MaboReviewedCyclePreparationError::InvalidRevisionRef(source_revision_ref.to_owned())
        })
}

fn diagnosed_property_refs(relation_type_refs: &[String]) -> Vec<&'static str> {
    let mut properties = relation_type_refs
        .iter()
        .filter_map(|relation_type_ref| mabo_wikidata_property_ref(relation_type_ref))
        .collect::<Vec<_>>();
    properties.sort_unstable();
    properties.dedup();
    properties
}

/// Derive one exact pinned provider request from a matched review row.
///
/// The persisted Mabo context layer stores bounded semantic relation types
/// (`participant`, `judge`, etc.), not raw property ids. We recover a provider
/// coordinate only through the exact inverse exported by that producer. When a
/// target has several reviewed relation witnesses at the same pinned source
/// revision, the lexicographically first bounded property is selected solely as
/// a deterministic reacquisition witness; the SameObject review remains about
/// `target_ref -> reviewed identity class`, not about preferring that relation.
pub fn reviewed_acquisition_request(
    planned: &MaboPlannedIdentityReview,
) -> Result<ReviewedAcquisitionRequest, MaboReviewedCyclePreparationError> {
    if planned.row.representation_ref != planned.assignment.representation_ref {
        return Err(MaboReviewedCyclePreparationError::ReviewRepresentationMismatch);
    }
    if !planned.row.candidate_only
        || planned.row.creates_semantic_authority
        || planned.row.applicability_promoted
        || planned.row.claim_truth_promoted
    {
        return Err(MaboReviewedCyclePreparationError::PromotingDiagnosisRow);
    }

    let [source_revision_ref] = planned.row.source_revision_refs.as_slice() else {
        return Err(MaboReviewedCyclePreparationError::AmbiguousSourceRevision);
    };
    let (source_qid, revision_id) = parse_wikidata_revision_ref(source_revision_ref)?;
    let property_ref = diagnosed_property_refs(&planned.row.relation_type_refs)
        .into_iter()
        .next()
        .ok_or(MaboReviewedCyclePreparationError::MissingDiagnosedRelation)?
        .to_owned();

    Ok(ReviewedAcquisitionRequest {
        source_qid,
        revision_id,
        source_revision_ref: source_revision_ref.clone(),
        property_ref,
        target_ref: planned.row.representation_ref.clone(),
    })
}

/// Prepare one already-reviewed Mabo identity cycle against an exactly pinned
/// acquired Wikidata entity and one exact route emitted from those bytes.
///
/// The SameObject coordinate pays only representation -> reviewed identity
/// class. The discovery relation to the Mabo parent remains `NewRelatedObject`.
pub fn prepare_reviewed_mabo_identity_cycle(
    diagnosis: &MaboConsumerDiagnosis,
    planned: &MaboPlannedIdentityReview,
    acquired: &AcquiredWikidataEntity,
    route: &RouteCandidate,
    payment_iteration_index: i64,
) -> Result<ReviewedPreparedCycle, MaboReviewedCyclePreparationError> {
    let row = &planned.row;
    let assignment = &planned.assignment;

    if row.representation_ref != assignment.representation_ref {
        return Err(MaboReviewedCyclePreparationError::ReviewRepresentationMismatch);
    }
    if !row.candidate_only
        || row.creates_semantic_authority
        || row.applicability_promoted
        || row.claim_truth_promoted
    {
        return Err(MaboReviewedCyclePreparationError::PromotingDiagnosisRow);
    }

    let residual = diagnosis
        .residuals
        .iter()
        .find(|residual| residual.residual_ref == row.residual_ref)
        .filter(|residual| residual.status == ResidualStatus::Open)
        .ok_or(MaboReviewedCyclePreparationError::ResidualNotOpen)?;

    if !row
        .source_revision_refs
        .iter()
        .any(|source_revision_ref| source_revision_ref == &acquired.source_revision_ref)
    {
        return Err(MaboReviewedCyclePreparationError::SourceRevisionMismatch);
    }

    let route_matches_reviewed_relation = row
        .relation_type_refs
        .iter()
        .filter_map(|relation_type_ref| mabo_wikidata_property_ref(relation_type_ref))
        .any(|property_ref| property_ref == route.property_ref);
    if route.route_family != RouteFamily::WikidataProperty
        || route.target_ref != row.representation_ref
        || !route_matches_reviewed_relation
    {
        return Err(MaboReviewedCyclePreparationError::RouteMismatch);
    }

    let observation = wikidata_property_observation(
        format!("query:mabo:{}:{}", route.property_ref, route.target_ref),
        acquired,
        route,
        GetterBackend::SlrNative,
    )
    .map_err(|error| MaboReviewedCyclePreparationError::ObservationAdapter(format!("{error:?}")))?;

    let reviewed = ReviewedEvidenceCoordinate {
        review_ref: assignment.review_ref.clone(),
        consumer_id: diagnosis.consumer_spec.consumer_id.clone(),
        requirement_id: row.requirement_id.clone(),
        coordinate: EvidenceCoordinateKind::SameObject,
        source_ref: Some(observation.source_revision_ref.clone()),
        evidence_ref: observation.request_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let mut payment_wire = Vec::new();
    compile_reviewed_evidence_payment(
        &diagnosis.consumer_spec,
        &reviewed,
        &mut payment_wire,
        payment_iteration_index,
    )
    .map_err(|error| MaboReviewedCyclePreparationError::ReviewedPayment(error.to_string()))?;

    let mut residual_output = Vec::new();
    let residual_receipt = compile_consumer_residual_stream_review_aware(
        &mut Cursor::new(payment_wire.as_slice()),
        &diagnosis.consumer_spec,
        &mut residual_output,
        payment_iteration_index.saturating_add(1),
    )
    .map_err(|error| MaboReviewedCyclePreparationError::ReviewedPayment(error.to_string()))?;
    if residual_receipt.requirements_paid != 1 {
        return Err(MaboReviewedCyclePreparationError::RequirementNotPaid);
    }

    let expansion_candidate = from_wikidata_route(
        residual,
        ResidualClass::Identity,
        acquired,
        route,
        ExpansionScoring {
            expected_residual_contraction: 1,
            provenance_quality: 5,
            same_object_confidence: 5,
            expected_new_world_value: 5,
            acquisition_cost: 1,
        },
    )
    .map_err(|error| MaboReviewedCyclePreparationError::ExpansionAdapter(format!("{error:?}")))?;

    let identity_resolution = WorldIdentityResolutionReceipt {
        receipt_ref: format!("identity-resolution:{}", assignment.review_ref),
        identity: WorldObjectIdentity::new(
            assignment.identity_class_ref.clone(),
            assignment.representation_ref.clone(),
        ),
        resolution_kind: WorldIdentityResolutionKind::ExactSameRepresentation,
        evidence_ref: assignment.review_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };

    let post_observation = PostAcquisitionWorldObservation {
        observation_ref: observation.request_ref.clone(),
        source_revision_ref: observation.source_revision_ref.clone(),
        triggering_residual_ref: residual.residual_ref.clone(),
        assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
        observed_residual_contraction: 1,
        newly_exposed_residuals: vec![],
        pnf_world_disambiguation_ref: assignment.review_ref.clone(),
        observation_authority: "experimental_candidate_only",
    };

    Ok(ReviewedPreparedCycle {
        prepared: PreparedWorldExpansionCycle {
            next_frontier_ref: format!(
                "frontier:mabo:world-identity:reviewed:{}",
                assignment.representation_ref
            ),
            routing: ResidualRouting {
                residual_ref: residual.residual_ref.clone(),
                residual_class: row.residual_class,
                routing_reason_ref: format!(
                    "{}:explicit-identity-review",
                    diagnosis.consumer_spec.consumer_id
                ),
            },
            candidates: vec![expansion_candidate],
            review_decision: ReviewDecision::Reviewed,
            disambiguation_outcome: DisambiguationOutcome::NewRelatedObject,
            identity_resolution,
            observation: post_observation,
        },
        reviewed_payment_wire: payment_wire,
    })
}
