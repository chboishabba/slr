//! Empirical-source bridges into the relational interlingua.
//!
//! Acceptance observations are reconstructed from durable parser/native
//! products. Callers may choose an already-persisted predicate/statement and
//! supply consumer context, but may not author role/filler structure in the
//! acceptance packet.

use thiserror::Error;

use crate::{
    load_candidate_pnf_batch, load_ontology_diagnostic, load_source_statement,
    observation_from_candidate_pnf, observation_from_native_wikidata,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedWikidataObservationRequest {
    pub diagnostic_ref: String,
    pub native_statement_ref: String,
    pub observation_ref: String,
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
    OntologyStore(#[from] crate::OntologyReviewError),
    #[error(transparent)]
    Relational(#[from] RelationalComparisonError),
    #[error("persisted candidate batch does not exist")]
    MissingBatch,
    #[error("persisted candidate batch owner statement does not exist")]
    MissingStatement,
    #[error("persisted candidate batch and source statement disagree on exact span")]
    WrongSpanOwner,
    #[error("persisted candidate/native source crossed candidate/non-promotion boundary")]
    PromotionBoundary,
    #[error("selected predicate is not a persisted predicate candidate in this batch")]
    MissingPredicate,
    #[error("selected native Wikidata statement is absent from the persisted diagnostic packet")]
    MissingNativeStatement,
}

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
        proposition_support_paid: false,
        applicability_paid: false,
        claim_truth_paid: false,
    };
    let mut provenance = request.provenance_refs.clone();
    provenance.extend([request.batch_ref.clone(), batch.parser_receipt_ref.clone()]);
    provenance.sort();
    provenance.dedup();
    Ok(observation_from_candidate_pnf(
        &statement.source_revision_ref,
        request.source_family,
        &request.observation_ref,
        &batch.parser_receipt_ref,
        &request.selected_predicate_candidate_ref,
        &reconstructed,
        request.context.clone(),
        provenance,
    )?)
}

pub fn relational_observation_from_persisted_wikidata(
    config: &DatabaseConfig,
    request: &PersistedWikidataObservationRequest,
) -> Result<RelationalObservation, PersistedRelationalObservationError> {
    let read = load_ontology_diagnostic(config, &request.diagnostic_ref)?;
    if read.creates_semantic_authority
        || read.grants_wikidata_edit_authority
        || !read.packet.candidate_only
        || read.packet.creates_semantic_authority
        || read.packet.claim_truth_promoted
        || read.packet.grants_wikidata_edit_authority
    {
        return Err(PersistedRelationalObservationError::PromotionBoundary);
    }
    let statement = read
        .packet
        .witnesses
        .iter()
        .flat_map(|w| w.native_statements.iter())
        .find(|s| s.statement_ref == request.native_statement_ref)
        .ok_or(PersistedRelationalObservationError::MissingNativeStatement)?;
    let mut provenance = request.provenance_refs.clone();
    provenance.extend([
        request.diagnostic_ref.clone(),
        read.packet.producer_receipt_ref.clone(),
        read.packet.source_snapshot_digest_ref.clone(),
        statement.statement_revision_ref.clone(),
    ]);
    provenance.sort();
    provenance.dedup();
    Ok(observation_from_native_wikidata(
        &read.packet.source_revision_ref,
        statement,
        &read.packet.producer_receipt_ref,
        provenance,
        request.context.clone(),
    )?)
}
