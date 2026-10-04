//! Persisted-source bridge into the generic relational interlingua.
//!
//! A REL observation used for empirical acceptance must be reconstructible
//! from a persisted candidate-PNF batch and its owning persisted source
//! statement.  This module deliberately does not accept role/filler data from
//! an acceptance JSON packet.

use thiserror::Error;

use crate::{
    load_candidate_pnf_batch, load_source_statement, observation_from_candidate_pnf,
    CandidatePnfBatch, CandidatePnfRole, DatabaseConfig, ObservationContext,
    RelationalComparisonError, RelationalObservation, RelationalSourceFamily,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedRelationalObservationRequest {
    pub batch_ref: String,
    pub selected_predicate_candidate_ref: String,
    pub observation_ref: String,
    pub source_family: RelationalSourceFamily,
    pub context: ObservationContext,
    pub provenance_refs: Vec<String>,
}

#[derive(Debug, Error)]
pub enum PersistedRelationalObservationError {
    #[error(transparent)]
    CandidateStore(#[from] crate::CandidatePnfStoreError),
    #[error(transparent)]
    StatementStore(#[from] crate::StatementTraceStoreError),
    #[error(transparent)]
    Relational(#[from] RelationalComparisonError),
    #[error("persisted candidate batch does not exist")]
    MissingBatch,
    #[error("persisted candidate batch owner statement does not exist")]
    MissingStatement,
    #[error("persisted candidate batch and source statement disagree on exact span")]
    WrongSpanOwner,
    #[error("persisted candidate batch crossed candidate/non-promotion boundary")]
    PromotionBoundary,
    #[error("selected predicate is not a persisted predicate candidate in this batch")]
    MissingPredicate,
}

/// Reconstruct one relational observation from durable candidate-PNF state.
///
/// The observation's source revision, exact span, parser receipt, factors and
/// predicate identity all originate in persisted state.  The caller may only
/// supply consumer-context metadata and the choice of one already-persisted
/// predicate candidate.
pub fn relational_observation_from_persisted_pnf(
    config: &DatabaseConfig,
    request: &PersistedRelationalObservationRequest,
) -> Result<RelationalObservation, PersistedRelationalObservationError> {
    let batch = load_candidate_pnf_batch(config, &request.batch_ref)?
        .ok_or(PersistedRelationalObservationError::MissingBatch)?;
    let statement = load_source_statement(config, &batch.statement_ref)?
        .ok_or(PersistedRelationalObservationError::MissingStatement)?;

    if statement.exact_span_ref != batch.exact_span_ref {
        return Err(PersistedRelationalObservationError::WrongSpanOwner);
    }
    if !statement.candidate_only
        || statement.creates_semantic_authority
        || statement.applicability_promoted
        || statement.claim_truth_promoted
        || !batch.candidate_only
        || batch.semantic_admission_paid
        || batch.proposition_support_paid
        || batch.applicability_paid
        || batch.claim_truth_paid
    {
        return Err(PersistedRelationalObservationError::PromotionBoundary);
    }
    if !batch.factors.iter().any(|factor| {
        factor.candidate_ref == request.selected_predicate_candidate_ref
            && factor.role == CandidatePnfRole::Predicate
            && factor.candidate_only
    }) {
        return Err(PersistedRelationalObservationError::MissingPredicate);
    }

    let reconstructed = CandidatePnfBatch {
        exact_span_ref: batch.exact_span_ref.clone(),
        candidates: batch.factors.clone(),
        proposition_support_paid: batch.proposition_support_paid,
        applicability_paid: batch.applicability_paid,
        claim_truth_paid: batch.claim_truth_paid,
    };
    let mut provenance_refs = request.provenance_refs.clone();
    provenance_refs.push(request.batch_ref.clone());
    provenance_refs.push(batch.parser_receipt_ref.clone());
    provenance_refs.sort();
    provenance_refs.dedup();

    observation_from_candidate_pnf(
        &statement.source_revision_ref,
        request.source_family,
        &request.observation_ref,
        &batch.parser_receipt_ref,
        &request.selected_predicate_candidate_ref,
        &reconstructed,
        request.context.clone(),
        provenance_refs,
    )
    .map_err(Into::into)
}
