//! SCALE-1 reviewed reconciliation materialization.
//!
//! A parser/reconciliation fingerprint is never itself a proposition identity.
//! After an explicit S29 Accept command for a source-scoped proposition cluster,
//! this bridge may materialize the existing chronology carriers:
//!
//!   reviewed grouping -> candidate PropositionRoot
//!                     -> per-source ClaimLeafs (still Unreviewed)
//!
//! The grouping review is not claim review and never promotes truth.

use postgres::{Client, NoTls};
use sensiblaw_core::chronology_contestation::{
    ClaimLeaf, ClaimLeafKind, ClaimReviewState, PropositionRoot,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    install_chronology_contestation_schema, install_review_workstation_schema,
    persist_claim_leaf, persist_proposition_root, ChronologyContestationStoreError,
    DatabaseConfig, ReviewWorkstationStoreError,
};

pub const RECONCILIATION_MATERIALIZATION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;

CREATE TABLE IF NOT EXISTS semantic.reconciliation_proposition_materialization (
    materialization_ref TEXT PRIMARY KEY,
    proposition_ref TEXT NOT NULL REFERENCES semantic.proposition_root(proposition_ref),
    proposition_fingerprint_ref TEXT NOT NULL,
    base_signature_ref TEXT NOT NULL,
    review_item_ref TEXT NOT NULL REFERENCES semantic.review_item(review_item_ref),
    accepted_review_command_ref TEXT NOT NULL REFERENCES semantic.review_receipt(command_ref),
    source_revision_ref TEXT NOT NULL,
    reviewed_grouping_identity BOOLEAN NOT NULL CHECK (reviewed_grouping_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    UNIQUE (proposition_fingerprint_ref, review_item_ref)
);

CREATE TABLE IF NOT EXISTS semantic.reconciliation_claim_materialization (
    claim_ref TEXT PRIMARY KEY REFERENCES semantic.claim_leaf(claim_ref),
    materialization_ref TEXT NOT NULL
      REFERENCES semantic.reconciliation_proposition_materialization(materialization_ref)
      ON DELETE CASCADE,
    statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref),
    proposition_fingerprint_ref TEXT NOT NULL,
    polarity_ref TEXT NOT NULL CHECK (polarity_ref IN ('positive','negative')),
    claim_review_state_ref TEXT NOT NULL CHECK (claim_review_state_ref='unreviewed'),
    grouping_review_is_claim_review BOOLEAN NOT NULL CHECK (NOT grouping_review_is_claim_review),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);
"#;

#[derive(Debug, Error)]
pub enum ReconciliationMaterializationError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("review workstation error: {0}")]
    Review(#[from] ReviewWorkstationStoreError),
    #[error("chronology/contestation persistence error: {0}")]
    Chronology(#[from] ChronologyContestationStoreError),
    #[error("review item is not an accepted proposition-cluster review")]
    ReviewNotAccepted,
    #[error("accepted review receipt is missing or mismatched")]
    AcceptedReviewReceiptMissing,
    #[error("review item does not point at a proposition fingerprint candidate")]
    NotPropositionFingerprint,
    #[error("review item lacks matching reconciliation-pressure provenance")]
    MissingReconciliationPressureProvenance,
    #[error("review source ancestry does not match proposition occurrences")]
    SourceAncestryMismatch,
    #[error("reconciliation candidate crossed a non-promotion boundary")]
    PromotionBoundary,
    #[error("existing materialization conflicts with deterministic identity")]
    ExistingMaterializationConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedReconciliationPropositionReceipt {
    pub materialization_ref: String,
    pub proposition_ref: String,
    pub proposition_fingerprint_ref: String,
    pub source_revision_ref: String,
    pub review_item_ref: String,
    pub accepted_review_command_ref: String,
    pub claim_refs: Vec<String>,
    pub reviewed_grouping_identity: bool,
    pub claim_review_state_unreviewed: bool,
    pub grouping_review_is_claim_review: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

fn digest_ref(domain: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in std::iter::once(domain).chain(parts.iter().copied()) {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

pub fn install_reconciliation_materialization_schema(
    config: &DatabaseConfig,
) -> Result<(), ReconciliationMaterializationError> {
    install_review_workstation_schema(config)?;
    install_chronology_contestation_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(RECONCILIATION_MATERIALIZATION_SCHEMA_SQL)?;
    Ok(())
}

pub fn materialize_accepted_reconciliation_proposition(
    config: &DatabaseConfig,
    review_item_ref: &str,
    accepted_review_command_ref: &str,
) -> Result<ReviewedReconciliationPropositionReceipt, ReconciliationMaterializationError> {
    install_reconciliation_materialization_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;

    let review = client.query_opt(
        r#"
        SELECT semantic_ref, item_kind_ref, current_status_ref,
               candidate_only, creates_semantic_authority,
               applicability_promoted, claim_truth_promoted
        FROM semantic.review_item
        WHERE review_item_ref=$1
        "#,
        &[&review_item_ref],
    )?;
    let Some(review) = review else {
        return Err(ReconciliationMaterializationError::ReviewNotAccepted);
    };
    let fingerprint_ref: String = review.get(0);
    let item_kind_ref: String = review.get(1);
    let current_status_ref: String = review.get(2);
    if item_kind_ref != "pnf_parse"
        || current_status_ref != "accepted"
        || !review.get::<_, bool>(3)
        || review.get::<_, bool>(4)
        || review.get::<_, bool>(5)
        || review.get::<_, bool>(6)
    {
        return Err(ReconciliationMaterializationError::ReviewNotAccepted);
    }

    let matching_pressure_provenance: bool = client
        .query_one(
            r#"
            SELECT EXISTS (
              SELECT 1
              FROM semantic.review_item_provenance rp
              JOIN semantic.reconciliation_pressure_candidate p
                ON p.pressure_ref=rp.provenance_ref
              WHERE rp.review_item_ref=$1
                AND p.semantic_kind_ref='proposition'
                AND p.semantic_fingerprint_ref=$2
                AND p.candidate_only
                AND p.requires_review
                AND NOT p.creates_semantic_authority
                AND NOT p.claim_truth_promoted
            )
            "#,
            &[&review_item_ref, &fingerprint_ref],
        )?
        .get(0);
    if !matching_pressure_provenance {
        return Err(
            ReconciliationMaterializationError::MissingReconciliationPressureProvenance,
        );
    }

    let accepted_receipt: bool = client
        .query_one(
            r#"
            SELECT EXISTS (
              SELECT 1
              FROM semantic.review_receipt
              WHERE command_ref=$1
                AND review_item_ref=$2
                AND semantic_ref=$3
                AND action_ref='accept'
                AND effect_ref='status_changed'
                AND effect_value_ref='accepted'
                AND candidate_only
                AND NOT creates_semantic_authority
                AND NOT applicability_promoted
                AND NOT claim_truth_promoted
            )
            "#,
            &[
                &accepted_review_command_ref,
                &review_item_ref,
                &fingerprint_ref,
            ],
        )?
        .get(0);
    if !accepted_receipt {
        return Err(ReconciliationMaterializationError::AcceptedReviewReceiptMissing);
    }

    let fingerprint = client.query_opt(
        r#"
        SELECT base_signature_ref, polarity_ref, candidate_only,
               creates_proposition_identity, creates_semantic_authority,
               applicability_promoted, claim_truth_promoted
        FROM semantic.proposition_fingerprint_candidate
        WHERE proposition_fingerprint_ref=$1
        "#,
        &[&fingerprint_ref],
    )?;
    let Some(fingerprint) = fingerprint else {
        return Err(ReconciliationMaterializationError::NotPropositionFingerprint);
    };
    let base_signature_ref: String = fingerprint.get(0);
    let polarity_ref: String = fingerprint.get(1);
    if !fingerprint.get::<_, bool>(2)
        || fingerprint.get::<_, bool>(3)
        || fingerprint.get::<_, bool>(4)
        || fingerprint.get::<_, bool>(5)
        || fingerprint.get::<_, bool>(6)
    {
        return Err(ReconciliationMaterializationError::PromotionBoundary);
    }

    let source_refs = client
        .query(
            "SELECT source_ref FROM semantic.review_item_source
             WHERE review_item_ref=$1 ORDER BY source_ref",
            &[&review_item_ref],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    if source_refs.is_empty() {
        return Err(ReconciliationMaterializationError::SourceAncestryMismatch);
    }

    let mut source_revision_ref: Option<String> = None;
    for statement_ref in &source_refs {
        let occurrence = client.query_opt(
            r#"
            SELECT o.source_revision_ref
            FROM semantic.proposition_candidate_occurrence o
            WHERE o.proposition_fingerprint_ref=$1
              AND o.statement_ref=$2
              AND o.candidate_only
              AND NOT o.creates_semantic_authority
              AND NOT o.claim_truth_promoted
            "#,
            &[&fingerprint_ref, statement_ref],
        )?;
        let Some(occurrence) = occurrence else {
            return Err(ReconciliationMaterializationError::SourceAncestryMismatch);
        };
        let revision: String = occurrence.get(0);
        if source_revision_ref
            .as_ref()
            .is_some_and(|existing| existing != &revision)
        {
            return Err(ReconciliationMaterializationError::SourceAncestryMismatch);
        }
        source_revision_ref = Some(revision);
    }
    let source_revision_ref =
        source_revision_ref.ok_or(ReconciliationMaterializationError::SourceAncestryMismatch)?;

    let proposition_ref = format!(
        "proposition:reconciliation:{}",
        digest_ref("reconciliation-proposition:v1", &[&base_signature_ref])
    );
    let proposition = PropositionRoot {
        proposition_ref: proposition_ref.clone(),
        label: base_signature_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    persist_proposition_root(config, &proposition)?;

    let kind = match polarity_ref.as_str() {
        "positive" => ClaimLeafKind::Affirmation,
        "negative" => ClaimLeafKind::Denial,
        _ => return Err(ReconciliationMaterializationError::PromotionBoundary),
    };

    let materialization_ref = format!(
        "reconciliation-proposition-materialization:{}",
        digest_ref(
            "reconciliation-proposition-materialization:v1",
            &[
                &fingerprint_ref,
                &source_revision_ref,
                &review_item_ref,
                accepted_review_command_ref,
            ],
        )
    );

    let mut claim_pairs = Vec::with_capacity(source_refs.len());
    for statement_ref in &source_refs {
        let claim_ref = format!(
            "claim:reconciliation:{}",
            digest_ref(
                "reconciliation-claim:v1",
                &[&proposition_ref, &fingerprint_ref, statement_ref],
            )
        );
        let claim = ClaimLeaf {
            claim_ref: claim_ref.clone(),
            proposition_ref: proposition_ref.clone(),
            kind,
            speaker_ref: None,
            statement_refs: vec![statement_ref.clone()],
            observation_refs: vec![],
            temporal_refs: vec![],
            scope_refs: vec![],
            review_state: ClaimReviewState::Unreviewed,
            review_ref: None,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        persist_claim_leaf(config, &claim)?;
        claim_pairs.push((statement_ref.clone(), claim_ref));
    }

    let mut tx = client.transaction()?;
    tx.execute(
        r#"
        INSERT INTO semantic.reconciliation_proposition_materialization
          (materialization_ref, proposition_ref, proposition_fingerprint_ref,
           base_signature_ref, review_item_ref, accepted_review_command_ref,
           source_revision_ref, reviewed_grouping_identity,
           creates_semantic_authority, applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,TRUE,FALSE,FALSE,FALSE)
        ON CONFLICT (materialization_ref) DO NOTHING
        "#,
        &[
            &materialization_ref,
            &proposition_ref,
            &fingerprint_ref,
            &base_signature_ref,
            &review_item_ref,
            &accepted_review_command_ref,
            &source_revision_ref,
        ],
    )?;

    for (statement_ref, claim_ref) in &claim_pairs {
        tx.execute(
            r#"
            INSERT INTO semantic.reconciliation_claim_materialization
              (claim_ref, materialization_ref, statement_ref,
               proposition_fingerprint_ref, polarity_ref,
               claim_review_state_ref, grouping_review_is_claim_review,
               creates_semantic_authority, claim_truth_promoted)
            VALUES ($1,$2,$3,$4,$5,'unreviewed',FALSE,FALSE,FALSE)
            ON CONFLICT (claim_ref) DO NOTHING
            "#,
            &[
                claim_ref,
                &materialization_ref,
                statement_ref,
                &fingerprint_ref,
                &polarity_ref,
            ],
        )?;
    }
    tx.commit()?;

    let stored = client.query_one(
        r#"
        SELECT proposition_ref, proposition_fingerprint_ref, source_revision_ref,
               review_item_ref, accepted_review_command_ref,
               reviewed_grouping_identity, creates_semantic_authority,
               applicability_promoted, claim_truth_promoted
        FROM semantic.reconciliation_proposition_materialization
        WHERE materialization_ref=$1
        "#,
        &[&materialization_ref],
    )?;
    if stored.get::<_, String>(0) != proposition_ref
        || stored.get::<_, String>(1) != fingerprint_ref
        || stored.get::<_, String>(2) != source_revision_ref
        || stored.get::<_, String>(3) != review_item_ref
        || stored.get::<_, String>(4) != accepted_review_command_ref
        || !stored.get::<_, bool>(5)
        || stored.get::<_, bool>(6)
        || stored.get::<_, bool>(7)
        || stored.get::<_, bool>(8)
    {
        return Err(ReconciliationMaterializationError::ExistingMaterializationConflict);
    }

    let mut claim_refs = claim_pairs
        .into_iter()
        .map(|(_, claim_ref)| claim_ref)
        .collect::<Vec<_>>();
    claim_refs.sort();
    claim_refs.dedup();

    Ok(ReviewedReconciliationPropositionReceipt {
        materialization_ref,
        proposition_ref,
        proposition_fingerprint_ref: fingerprint_ref,
        source_revision_ref,
        review_item_ref: review_item_ref.to_owned(),
        accepted_review_command_ref: accepted_review_command_ref.to_owned(),
        claim_refs,
        reviewed_grouping_identity: true,
        claim_review_state_unreviewed: true,
        grouping_review_is_claim_review: false,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposition_identity_ignores_polarity_but_claim_identity_does_not() {
        let base = "a=[alice]|p=[call]|o=[bob]";
        let proposition_a = digest_ref("reconciliation-proposition:v1", &[base]);
        let proposition_b = digest_ref("reconciliation-proposition:v1", &[base]);
        assert_eq!(proposition_a, proposition_b);

        let positive = digest_ref("reconciliation-claim:v1", &[base, "positive", "statement:1"]);
        let negative = digest_ref("reconciliation-claim:v1", &[base, "negative", "statement:1"]);
        assert_ne!(positive, negative);
    }
}
