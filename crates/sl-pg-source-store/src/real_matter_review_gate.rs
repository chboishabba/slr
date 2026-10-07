//! REAL-MATTER-1 durable human-review gate.
//!
//! This is orchestration only. It reopens one exact legal source manifestation
//! plus one candidate PNF factor and persists a canonical pending review item.
//! It cannot accept the item, choose evidence role/normative order, or create
//! reviewed evidence. Those remain explicit later human/domain payments.

use postgres::{Client, NoTls};
use sensiblaw_core::review_workstation::{ReviewAction, ReviewItem, ReviewItemKind, ReviewStatus};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{persist_review_item, DatabaseConfig, ReviewWorkstationStoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealMatterReviewGateDraft {
    pub observation_ref: String,
    pub consumer_ref: String,
    pub reason: String,
    pub source_manifestation_ref: String,
    pub candidate_pnf_batch_ref: String,
    pub candidate_factor_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealMatterReviewGateReceipt {
    pub review_item_ref: String,
    pub observation_ref: String,
    pub consumer_ref: String,
    pub source_manifestation_ref: String,
    pub source_revision_ref: String,
    pub document_ref: String,
    pub statement_ref: String,
    pub exact_span_ref: String,
    pub candidate_pnf_batch_ref: String,
    pub candidate_factor_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum RealMatterReviewGateError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    ReviewStore(#[from] ReviewWorkstationStoreError),
    #[error("required real-matter review coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("candidate batch/factor does not reopen as an unpaid candidate")]
    CandidateMismatch,
    #[error("source manifestation and candidate source do not identify the same source revision/document")]
    SourceMismatch,
}

pub fn prepare_real_matter_review_gate(
    config: &DatabaseConfig,
    draft: &RealMatterReviewGateDraft,
) -> Result<RealMatterReviewGateReceipt, RealMatterReviewGateError> {
    validate_draft(draft)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let candidate = client.query_opt(
        r#"SELECT s.source_revision_ref,s.document_ref,s.statement_ref,b.exact_span_ref,
                  b.candidate_only,b.semantic_admission_paid,b.proposition_support_paid,
                  b.applicability_paid,b.claim_truth_paid,f.candidate_only
           FROM pnf.statement_candidate_batch b
           JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
           JOIN pnf.statement_candidate_factor f
             ON f.batch_ref=b.batch_ref AND f.statement_ref=b.statement_ref
           WHERE b.batch_ref=$1 AND f.candidate_ref=$2"#,
        &[&draft.candidate_pnf_batch_ref, &draft.candidate_factor_ref],
    )?.ok_or(RealMatterReviewGateError::CandidateMismatch)?;
    if !candidate.get::<_, bool>(4)
        || candidate.get::<_, bool>(5)
        || candidate.get::<_, bool>(6)
        || candidate.get::<_, bool>(7)
        || candidate.get::<_, bool>(8)
        || !candidate.get::<_, bool>(9)
    {
        return Err(RealMatterReviewGateError::CandidateMismatch);
    }
    let source_revision_ref: String = candidate.get(0);
    let document_ref: String = candidate.get(1);
    let statement_ref: String = candidate.get(2);
    let exact_span_ref: String = candidate.get(3);

    let source = client.query_opt(
        r#"SELECT source_revision_ref,document_ref,candidate_only,creates_semantic_authority,
                  creates_legal_authority,applicability_promoted,claim_truth_promoted
           FROM source_provenance.legal_source_manifestation WHERE manifestation_ref=$1"#,
        &[&draft.source_manifestation_ref],
    )?.ok_or(RealMatterReviewGateError::SourceMismatch)?;
    if source.get::<_, String>(0) != source_revision_ref
        || source.get::<_, String>(1) != document_ref
        || !source.get::<_, bool>(2)
        || source.get::<_, bool>(3)
        || source.get::<_, bool>(4)
        || source.get::<_, bool>(5)
        || source.get::<_, bool>(6)
    {
        return Err(RealMatterReviewGateError::SourceMismatch);
    }

    let review_item_ref = stable_ref(&[
        "real-matter-review-item:v1",
        &draft.observation_ref,
        &draft.consumer_ref,
        &draft.source_manifestation_ref,
        &draft.candidate_pnf_batch_ref,
        &draft.candidate_factor_ref,
    ]);
    let item = ReviewItem {
        review_item_ref: review_item_ref.clone(),
        semantic_ref: draft.observation_ref.clone(),
        item_kind: ReviewItemKind::Observation,
        reason: draft.reason.clone(),
        provenance_refs: vec![
            draft.source_manifestation_ref.clone(),
            draft.candidate_pnf_batch_ref.clone(),
            draft.candidate_factor_ref.clone(),
            exact_span_ref.clone(),
        ],
        source_refs: vec![statement_ref.clone(), document_ref.clone()],
        current_status: ReviewStatus::Pending,
        available_actions: vec![
            ReviewAction::Accept,
            ReviewAction::Reject,
            ReviewAction::Abstain,
            ReviewAction::Qualify,
            ReviewAction::RequestEvidence,
            ReviewAction::OpenSource,
            ReviewAction::FollowAuthority,
        ],
        affected_consumer_refs: vec![draft.consumer_ref.clone()],
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let persisted = persist_review_item(config, &item)?;

    Ok(RealMatterReviewGateReceipt {
        review_item_ref: persisted.review_item_ref,
        observation_ref: draft.observation_ref.clone(),
        consumer_ref: draft.consumer_ref.clone(),
        source_manifestation_ref: draft.source_manifestation_ref.clone(),
        source_revision_ref,
        document_ref,
        statement_ref,
        exact_span_ref,
        candidate_pnf_batch_ref: draft.candidate_pnf_batch_ref.clone(),
        candidate_factor_ref: draft.candidate_factor_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

fn validate_draft(draft: &RealMatterReviewGateDraft) -> Result<(), RealMatterReviewGateError> {
    for (name, value) in [
        ("observation_ref", draft.observation_ref.as_str()),
        ("consumer_ref", draft.consumer_ref.as_str()),
        ("reason", draft.reason.as_str()),
        ("source_manifestation_ref", draft.source_manifestation_ref.as_str()),
        ("candidate_pnf_batch_ref", draft.candidate_pnf_batch_ref.as_str()),
        ("candidate_factor_ref", draft.candidate_factor_ref.as_str()),
    ] {
        if value.trim().is_empty() { return Err(RealMatterReviewGateError::EmptyCoordinate(name)); }
    }
    Ok(())
}

fn stable_ref(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts { hasher.update((part.len() as u64).to_be_bytes()); hasher.update(part.as_bytes()); }
    let digest: [u8; 32] = hasher.finalize().into();
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(64);
    for byte in digest { hex.push(D[(byte >> 4) as usize] as char); hex.push(D[(byte & 15) as usize] as char); }
    format!("review-item:real-matter:sha256:{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_item_identity_is_source_and_candidate_specific() {
        let a = stable_ref(&["v1","obs","matter","source:a","batch","factor"]);
        let b = stable_ref(&["v1","obs","matter","source:b","batch","factor"]);
        assert_ne!(a, b);
    }
}
