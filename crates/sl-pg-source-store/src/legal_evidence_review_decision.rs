//! Durable human legal-evidence review decision.
//!
//! A generic review receipt records reviewer + accept/qualify action, but does
//! not by itself bind the legal evidence role or normative order chosen by that
//! reviewer. REAL-MATTER-1 requires those choices to be durable before reviewed
//! evidence can be materialised. This module adds that narrow binding without
//! promoting applicability, legal authority, or claim truth.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

pub const LEGAL_EVIDENCE_REVIEW_DECISION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.legal_evidence_review_decision (
  decision_ref TEXT PRIMARY KEY,
  review_receipt_ref TEXT NOT NULL REFERENCES semantic.review_receipt(command_ref),
  review_item_ref TEXT NOT NULL REFERENCES semantic.review_item(review_item_ref),
  reviewer_ref TEXT NOT NULL,
  observation_ref TEXT NOT NULL,
  consumer_ref TEXT NOT NULL,
  requirement_ref TEXT NOT NULL,
  evidence_role_ref TEXT NOT NULL CHECK (length(btrim(evidence_role_ref)) > 0),
  normative_order_ref TEXT NOT NULL CHECK (length(btrim(normative_order_ref)) > 0),
  proposition_ref TEXT NOT NULL,
  source_manifestation_ref TEXT NOT NULL REFERENCES source_provenance.legal_source_manifestation(manifestation_ref),
  candidate_pnf_batch_ref TEXT NOT NULL REFERENCES pnf.statement_candidate_batch(batch_ref),
  candidate_factor_ref TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  decision_sha256 BYTEA NOT NULL,
  UNIQUE (
    review_receipt_ref, consumer_ref, requirement_ref, evidence_role_ref,
    normative_order_ref, proposition_ref, source_manifestation_ref,
    candidate_pnf_batch_ref, candidate_factor_ref
  )
);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalEvidenceReviewDecisionDraft {
    pub review_receipt_ref: String,
    pub observation_ref: String,
    pub consumer_ref: String,
    pub requirement_ref: String,
    pub evidence_role_ref: String,
    pub normative_order_ref: String,
    pub proposition_ref: String,
    pub source_manifestation_ref: String,
    pub candidate_pnf_batch_ref: String,
    pub candidate_factor_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLegalEvidenceReviewDecision {
    pub decision_ref: String,
    pub review_receipt_ref: String,
    pub review_item_ref: String,
    pub reviewer_ref: String,
    pub observation_ref: String,
    pub consumer_ref: String,
    pub requirement_ref: String,
    pub evidence_role_ref: String,
    pub normative_order_ref: String,
    pub proposition_ref: String,
    pub source_manifestation_ref: String,
    pub candidate_pnf_batch_ref: String,
    pub candidate_factor_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum LegalEvidenceReviewDecisionError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("upstream persistence prerequisite failed: {0}")]
    Upstream(String),
    #[error("required legal evidence review coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("review receipt is not an accepted/qualified review of the declared observation")]
    ReviewReceiptMismatch,
    #[error("review item does not bind the declared source, candidate, observation and consumer")]
    ReviewItemMismatch,
    #[error("candidate batch/factor does not reopen as an unpaid candidate")]
    CandidateMismatch,
    #[error("source manifestation and candidate source do not identify the same source revision/document")]
    SourceMismatch,
    #[error("persisted legal evidence review decision conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("legal evidence review decision not found: {0}")]
    NotFound(String),
}

pub fn install_legal_evidence_review_decision_schema(
    config: &DatabaseConfig,
) -> Result<(), LegalEvidenceReviewDecisionError> {
    crate::install_review_workstation_schema(config)
        .map_err(|error| LegalEvidenceReviewDecisionError::Upstream(error.to_string()))?;
    crate::install_candidate_pnf_schema(config)
        .map_err(|error| LegalEvidenceReviewDecisionError::Upstream(error.to_string()))?;
    crate::install_legal_source_manifestation_schema(config)
        .map_err(|error| LegalEvidenceReviewDecisionError::Upstream(error.to_string()))?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(LEGAL_EVIDENCE_REVIEW_DECISION_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_legal_evidence_review_decision(
    config: &DatabaseConfig,
    draft: &LegalEvidenceReviewDecisionDraft,
) -> Result<PersistedLegalEvidenceReviewDecision, LegalEvidenceReviewDecisionError> {
    validate_draft(draft)?;
    install_legal_evidence_review_decision_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;

    let review = client.query_opt(
        r#"SELECT review_item_ref,reviewer_ref,semantic_ref,action_ref,candidate_only,
                  creates_semantic_authority,applicability_promoted,claim_truth_promoted
           FROM semantic.review_receipt WHERE command_ref=$1"#,
        &[&draft.review_receipt_ref],
    )?.ok_or(LegalEvidenceReviewDecisionError::ReviewReceiptMismatch)?;
    let review_item_ref: String = review.get(0);
    let reviewer_ref: String = review.get(1);
    let action_ref: String = review.get(3);
    if review.get::<_, String>(2) != draft.observation_ref
        || !matches!(action_ref.as_str(), "accept" | "qualify")
        || !review.get::<_, bool>(4)
        || review.get::<_, bool>(5)
        || review.get::<_, bool>(6)
        || review.get::<_, bool>(7)
    {
        return Err(LegalEvidenceReviewDecisionError::ReviewReceiptMismatch);
    }

    // The human receipt must belong to the exact review item prepared from this
    // source/candidate pair. Same-observation receipt replay is not sufficient.
    let review_item = client.query_opt(
        r#"SELECT i.semantic_ref,i.item_kind_ref,i.current_status_ref,
                  i.candidate_only,i.creates_semantic_authority,
                  i.applicability_promoted,i.claim_truth_promoted,
                  EXISTS(SELECT 1 FROM semantic.review_item_provenance p
                         WHERE p.review_item_ref=i.review_item_ref AND p.provenance_ref=$2),
                  EXISTS(SELECT 1 FROM semantic.review_item_provenance p
                         WHERE p.review_item_ref=i.review_item_ref AND p.provenance_ref=$3),
                  EXISTS(SELECT 1 FROM semantic.review_item_provenance p
                         WHERE p.review_item_ref=i.review_item_ref AND p.provenance_ref=$4),
                  EXISTS(SELECT 1 FROM semantic.review_item_consumer c
                         WHERE c.review_item_ref=i.review_item_ref AND c.consumer_ref=$5)
           FROM semantic.review_item i WHERE i.review_item_ref=$1"#,
        &[&review_item_ref, &draft.source_manifestation_ref, &draft.candidate_pnf_batch_ref,
          &draft.candidate_factor_ref, &draft.consumer_ref],
    )?.ok_or(LegalEvidenceReviewDecisionError::ReviewItemMismatch)?;
    let review_status: String = review_item.get(2);
    if review_item.get::<_, String>(0) != draft.observation_ref
        || review_item.get::<_, String>(1) != "observation"
        || !matches!(review_status.as_str(), "accepted" | "qualified")
        || !review_item.get::<_, bool>(3)
        || review_item.get::<_, bool>(4)
        || review_item.get::<_, bool>(5)
        || review_item.get::<_, bool>(6)
        || !review_item.get::<_, bool>(7)
        || !review_item.get::<_, bool>(8)
        || !review_item.get::<_, bool>(9)
        || !review_item.get::<_, bool>(10)
    {
        return Err(LegalEvidenceReviewDecisionError::ReviewItemMismatch);
    }

    let candidate = client.query_opt(
        r#"SELECT s.source_revision_ref,s.document_ref,b.candidate_only,
                  b.semantic_admission_paid,b.proposition_support_paid,b.applicability_paid,
                  b.claim_truth_paid,f.candidate_only
           FROM pnf.statement_candidate_batch b
           JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
           JOIN pnf.statement_candidate_factor f
             ON f.batch_ref=b.batch_ref AND f.statement_ref=b.statement_ref
           WHERE b.batch_ref=$1 AND f.candidate_ref=$2"#,
        &[&draft.candidate_pnf_batch_ref, &draft.candidate_factor_ref],
    )?.ok_or(LegalEvidenceReviewDecisionError::CandidateMismatch)?;
    if !candidate.get::<_, bool>(2)
        || candidate.get::<_, bool>(3)
        || candidate.get::<_, bool>(4)
        || candidate.get::<_, bool>(5)
        || candidate.get::<_, bool>(6)
        || !candidate.get::<_, bool>(7)
    {
        return Err(LegalEvidenceReviewDecisionError::CandidateMismatch);
    }
    let source_revision_ref: String = candidate.get(0);
    let document_ref: String = candidate.get(1);

    let source = client.query_opt(
        r#"SELECT source_revision_ref,document_ref,candidate_only,creates_semantic_authority,
                  creates_legal_authority,applicability_promoted,claim_truth_promoted
           FROM source_provenance.legal_source_manifestation WHERE manifestation_ref=$1"#,
        &[&draft.source_manifestation_ref],
    )?.ok_or(LegalEvidenceReviewDecisionError::SourceMismatch)?;
    if source.get::<_, String>(0) != source_revision_ref
        || source.get::<_, String>(1) != document_ref
        || !source.get::<_, bool>(2)
        || source.get::<_, bool>(3)
        || source.get::<_, bool>(4)
        || source.get::<_, bool>(5)
        || source.get::<_, bool>(6)
    {
        return Err(LegalEvidenceReviewDecisionError::SourceMismatch);
    }

    let decision_ref = stable_ref(&[
        "legal-evidence-review-decision:v1",
        &draft.review_receipt_ref,
        &draft.observation_ref,
        &draft.consumer_ref,
        &draft.requirement_ref,
        &draft.evidence_role_ref,
        &draft.normative_order_ref,
        &draft.proposition_ref,
        &draft.source_manifestation_ref,
        &draft.candidate_pnf_batch_ref,
        &draft.candidate_factor_ref,
    ]);
    let digest = digest_parts(&[
        &decision_ref,
        &review_item_ref,
        &reviewer_ref,
        &source_revision_ref,
        &document_ref,
    ]);
    client.execute(
        r#"INSERT INTO semantic.legal_evidence_review_decision
           (decision_ref,review_receipt_ref,review_item_ref,reviewer_ref,observation_ref,
            consumer_ref,requirement_ref,evidence_role_ref,normative_order_ref,proposition_ref,
            source_manifestation_ref,candidate_pnf_batch_ref,candidate_factor_ref,candidate_only,
            creates_semantic_authority,creates_legal_authority,applicability_promoted,
            claim_truth_promoted,decision_sha256)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,TRUE,FALSE,FALSE,FALSE,FALSE,$14)
           ON CONFLICT (decision_ref) DO NOTHING"#,
        &[&decision_ref,&draft.review_receipt_ref,&review_item_ref,&reviewer_ref,
          &draft.observation_ref,&draft.consumer_ref,&draft.requirement_ref,&draft.evidence_role_ref,
          &draft.normative_order_ref,&draft.proposition_ref,&draft.source_manifestation_ref,
          &draft.candidate_pnf_batch_ref,&draft.candidate_factor_ref,&&digest[..]],
    )?;

    let loaded = load_legal_evidence_review_decision(config, &decision_ref)?;
    if loaded.review_receipt_ref != draft.review_receipt_ref
        || loaded.review_item_ref != review_item_ref
        || loaded.reviewer_ref != reviewer_ref
        || loaded.observation_ref != draft.observation_ref
        || loaded.consumer_ref != draft.consumer_ref
        || loaded.requirement_ref != draft.requirement_ref
        || loaded.evidence_role_ref != draft.evidence_role_ref
        || loaded.normative_order_ref != draft.normative_order_ref
        || loaded.proposition_ref != draft.proposition_ref
        || loaded.source_manifestation_ref != draft.source_manifestation_ref
        || loaded.candidate_pnf_batch_ref != draft.candidate_pnf_batch_ref
        || loaded.candidate_factor_ref != draft.candidate_factor_ref
        || !loaded.candidate_only || loaded.creates_semantic_authority
        || loaded.creates_legal_authority || loaded.applicability_promoted || loaded.claim_truth_promoted
    {
        return Err(LegalEvidenceReviewDecisionError::ExistingRowConflict);
    }
    Ok(loaded)
}

pub fn load_legal_evidence_review_decision(
    config: &DatabaseConfig,
    decision_ref: &str,
) -> Result<PersistedLegalEvidenceReviewDecision, LegalEvidenceReviewDecisionError> {
    required("decision_ref", decision_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT decision_ref,review_receipt_ref,review_item_ref,reviewer_ref,observation_ref,
                  consumer_ref,requirement_ref,evidence_role_ref,normative_order_ref,proposition_ref,
                  source_manifestation_ref,candidate_pnf_batch_ref,candidate_factor_ref,candidate_only,
                  creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted
           FROM semantic.legal_evidence_review_decision WHERE decision_ref=$1"#,
        &[&decision_ref],
    )?.ok_or_else(|| LegalEvidenceReviewDecisionError::NotFound(decision_ref.to_owned()))?;
    Ok(PersistedLegalEvidenceReviewDecision {
        decision_ref: row.get(0), review_receipt_ref: row.get(1), review_item_ref: row.get(2),
        reviewer_ref: row.get(3), observation_ref: row.get(4), consumer_ref: row.get(5),
        requirement_ref: row.get(6), evidence_role_ref: row.get(7), normative_order_ref: row.get(8),
        proposition_ref: row.get(9), source_manifestation_ref: row.get(10),
        candidate_pnf_batch_ref: row.get(11), candidate_factor_ref: row.get(12),
        candidate_only: row.get(13), creates_semantic_authority: row.get(14),
        creates_legal_authority: row.get(15), applicability_promoted: row.get(16),
        claim_truth_promoted: row.get(17),
    })
}

fn validate_draft(draft: &LegalEvidenceReviewDecisionDraft) -> Result<(), LegalEvidenceReviewDecisionError> {
    for (name, value) in [
        ("review_receipt_ref", draft.review_receipt_ref.as_str()),
        ("observation_ref", draft.observation_ref.as_str()),
        ("consumer_ref", draft.consumer_ref.as_str()),
        ("requirement_ref", draft.requirement_ref.as_str()),
        ("evidence_role_ref", draft.evidence_role_ref.as_str()),
        ("normative_order_ref", draft.normative_order_ref.as_str()),
        ("proposition_ref", draft.proposition_ref.as_str()),
        ("source_manifestation_ref", draft.source_manifestation_ref.as_str()),
        ("candidate_pnf_batch_ref", draft.candidate_pnf_batch_ref.as_str()),
        ("candidate_factor_ref", draft.candidate_factor_ref.as_str()),
    ] { required(name, value)?; }
    Ok(())
}

fn required(name: &'static str, value: &str) -> Result<(), LegalEvidenceReviewDecisionError> {
    if value.trim().is_empty() { Err(LegalEvidenceReviewDecisionError::EmptyCoordinate(name)) } else { Ok(()) }
}

fn stable_ref(parts: &[&str]) -> String {
    format!("legal-evidence-review-decision:sha256:{}", hex(&digest_parts(parts)))
}
fn digest_parts(parts: &[&str]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts { hasher.update((part.len() as u64).to_be_bytes()); hasher.update(part.as_bytes()); }
    hasher.finalize().into()
}
fn hex(bytes: &[u8]) -> String {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes { out.push(D[(byte >> 4) as usize] as char); out.push(D[(byte & 15) as usize] as char); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_identity_changes_when_normative_order_changes() {
        let a = stable_ref(&["receipt","obs","consumer","req","role","order:a","prop","source","batch","factor"]);
        let b = stable_ref(&["receipt","obs","consumer","req","role","order:b","prop","source","batch","factor"]);
        assert_ne!(a, b);
    }
}
