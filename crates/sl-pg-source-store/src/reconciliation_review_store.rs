//! SCALE-1 L2 -> S29 review projection.
//!
//! Automatic corpus reconciliation can populate the human review queue without
//! creating semantic identity or bypassing the existing M13 admission paths.
//!
//! Important:
//! - repeated event fingerprints become generic Observation review work;
//! - they do NOT become EventAssembly proposals;
//! - polarity conflict candidates become ClaimContestation review work;
//! - accepting these review items changes workflow state only.

use std::collections::BTreeSet;

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewItem, ReviewItemKind, ReviewStatus,
};
use thiserror::Error;

use crate::{
    install_review_workstation_schema, persist_review_item, DatabaseConfig,
    ReviewWorkstationStoreError,
};

const REVIEW_PROJECTION_ALGORITHM_REF: &str = "scale1:reconciliation-review-projection:v1";

const REVIEW_PROJECTION_STAGE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS semantic.reconciliation_review_projection_stage_receipt (
    source_revision_ref TEXT NOT NULL,
    algorithm_ref TEXT NOT NULL,
    input_fingerprint_ref TEXT NOT NULL,
    consumer_scope_ref TEXT NOT NULL,
    cluster_review_items BIGINT NOT NULL,
    contestation_review_items BIGINT NOT NULL,
    review_item_refs TEXT[] NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    creates_event_identity BOOLEAN NOT NULL CHECK (NOT creates_event_identity),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    PRIMARY KEY (
      source_revision_ref, algorithm_ref, input_fingerprint_ref, consumer_scope_ref
    )
);
"#;

#[derive(Debug, Error)]
pub enum ReconciliationReviewError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("review workstation error: {0}")]
    Review(#[from] ReviewWorkstationStoreError),
    #[error("reconciliation candidate violated non-promotion boundary")]
    PromotionBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationReviewReceipt {
    pub source_revision_ref: String,
    pub stage_reused: bool,
    pub input_fingerprint_ref: String,
    pub cluster_review_items: usize,
    pub contestation_review_items: usize,
    pub review_item_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_event_identity: bool,
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

fn canonical_scope_ref(affected_consumer_refs: &[String]) -> String {
    let mut refs = affected_consumer_refs.to_vec();
    refs.sort();
    refs.dedup();
    let parts = refs.iter().map(String::as_str).collect::<Vec<_>>();
    digest_ref("reconciliation-review-consumer-scope:v1", &parts)
}

fn review_input_fingerprint(
    pressure_refs: &[String],
    contestation_refs: &[String],
) -> String {
    let mut parts = Vec::with_capacity(pressure_refs.len() + contestation_refs.len() + 2);
    parts.push(format!("pressure-count:{}", pressure_refs.len()));
    parts.extend(pressure_refs.iter().map(|value| format!("pressure:{value}")));
    parts.push(format!("contestation-count:{}", contestation_refs.len()));
    parts.extend(
        contestation_refs
            .iter()
            .map(|value| format!("contestation:{value}")),
    );
    let borrowed = parts.iter().map(String::as_str).collect::<Vec<_>>();
    digest_ref("reconciliation-review-input:v1", &borrowed)
}

fn load_reusable_projection_receipt(
    client: &mut Client,
    source_revision_ref: &str,
    input_fingerprint_ref: &str,
    consumer_scope_ref: &str,
) -> Result<Option<ReconciliationReviewReceipt>, ReconciliationReviewError> {
    let Some(row) = client.query_opt(
        r#"
        SELECT cluster_review_items, contestation_review_items, review_item_refs,
               candidate_only, creates_semantic_authority, creates_event_identity,
               applicability_promoted, claim_truth_promoted
        FROM semantic.reconciliation_review_projection_stage_receipt
        WHERE source_revision_ref=$1
          AND algorithm_ref=$2
          AND input_fingerprint_ref=$3
          AND consumer_scope_ref=$4
        "#,
        &[
            &source_revision_ref,
            &REVIEW_PROJECTION_ALGORITHM_REF,
            &input_fingerprint_ref,
            &consumer_scope_ref,
        ],
    )? else {
        return Ok(None);
    };

    let review_item_refs: Vec<String> = row.get(2);
    let boundaries = client.query_one(
        r#"
        SELECT COUNT(*)::BIGINT,
               COALESCE(BOOL_AND(
                 candidate_only
                 AND NOT creates_semantic_authority
                 AND NOT applicability_promoted
                 AND NOT claim_truth_promoted
               ), TRUE)
        FROM semantic.review_item
        WHERE review_item_ref = ANY($1)
        "#,
        &[&review_item_refs],
    )?;
    let expected = review_item_refs.len() as i64;
    if boundaries.get::<_, i64>(0) != expected || !boundaries.get::<_, bool>(1) {
        return Ok(None);
    }

    let receipt = ReconciliationReviewReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        stage_reused: true,
        input_fingerprint_ref: input_fingerprint_ref.to_owned(),
        cluster_review_items: row.get::<_, i64>(0).max(0) as usize,
        contestation_review_items: row.get::<_, i64>(1).max(0) as usize,
        review_item_refs,
        candidate_only: row.get(3),
        creates_semantic_authority: row.get(4),
        creates_event_identity: row.get(5),
        applicability_promoted: row.get(6),
        claim_truth_promoted: row.get(7),
    };
    if !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.creates_event_identity
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
    {
        return Err(ReconciliationReviewError::PromotionBoundary);
    }
    Ok(Some(receipt))
}

fn common_actions() -> Vec<ReviewAction> {
    vec![
        ReviewAction::Accept,
        ReviewAction::Reject,
        ReviewAction::Abstain,
        ReviewAction::Qualify,
        ReviewAction::RequestEvidence,
        ReviewAction::OpenSource,
    ]
}

fn load_occurrence_statement_refs(
    client: &mut Client,
    kind: &str,
    fingerprint_ref: &str,
    source_revision_ref: &str,
) -> Result<Vec<String>, postgres::Error> {
    let sql = match kind {
        "proposition" => {
            "SELECT DISTINCT statement_ref
             FROM semantic.proposition_candidate_occurrence
             WHERE proposition_fingerprint_ref=$1 AND source_revision_ref=$2
             ORDER BY statement_ref"
        }
        "event" => {
            "SELECT DISTINCT statement_ref
             FROM semantic.event_candidate_occurrence
             WHERE event_fingerprint_ref=$1 AND source_revision_ref=$2
             ORDER BY statement_ref"
        }
        _ => unreachable!("fixed reconciliation semantic kind"),
    };
    Ok(client
        .query(sql, &[&fingerprint_ref, &source_revision_ref])?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect())
}

pub fn enqueue_reconciliation_review_items(
    config: &DatabaseConfig,
    source_revision_ref: &str,
    affected_consumer_refs: Vec<String>,
) -> Result<ReconciliationReviewReceipt, ReconciliationReviewError> {
    install_review_workstation_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(REVIEW_PROJECTION_STAGE_SQL)?;

    let pressure_rows = client.query(
        r#"
        WITH touched AS (
          SELECT DISTINCT 'proposition'::TEXT AS semantic_kind_ref,
                          proposition_fingerprint_ref AS semantic_fingerprint_ref
          FROM semantic.proposition_candidate_occurrence
          WHERE source_revision_ref=$1
          UNION
          SELECT DISTINCT 'event'::TEXT,
                          event_fingerprint_ref
          FROM semantic.event_candidate_occurrence
          WHERE source_revision_ref=$1
        )
        SELECT p.pressure_ref, p.semantic_kind_ref, p.semantic_fingerprint_ref,
               p.occurrence_count, p.source_revision_count,
               p.reason_ref, p.candidate_only, p.requires_review,
               p.creates_semantic_authority, p.claim_truth_promoted
        FROM semantic.reconciliation_pressure_candidate p
        JOIN touched t
          ON t.semantic_kind_ref=p.semantic_kind_ref
         AND t.semantic_fingerprint_ref=p.semantic_fingerprint_ref
        ORDER BY p.semantic_kind_ref, p.semantic_fingerprint_ref
        "#,
        &[&source_revision_ref],
    )?;

    let conflict_rows = client.query(
        r#"
        WITH touched_proposition AS (
          SELECT DISTINCT proposition_fingerprint_ref
          FROM semantic.proposition_candidate_occurrence
          WHERE source_revision_ref=$1
        )
        SELECT c.relation_ref, c.base_signature_ref,
               c.positive_proposition_fingerprint_ref,
               c.negative_proposition_fingerprint_ref,
               c.detector_ref, c.candidate_only, c.requires_review,
               c.creates_contestation_identity,
               c.creates_semantic_authority, c.claim_truth_promoted
        FROM semantic.contestation_candidate c
        WHERE c.positive_proposition_fingerprint_ref IN
                (SELECT proposition_fingerprint_ref FROM touched_proposition)
           OR c.negative_proposition_fingerprint_ref IN
                (SELECT proposition_fingerprint_ref FROM touched_proposition)
        ORDER BY c.relation_ref
        "#,
        &[&source_revision_ref],
    )?;


    let pressure_refs = pressure_rows
        .iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    let contestation_refs = conflict_rows
        .iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    let input_fingerprint_ref =
        review_input_fingerprint(&pressure_refs, &contestation_refs);
    let consumer_scope_ref = canonical_scope_ref(&affected_consumer_refs);

    if let Some(receipt) = load_reusable_projection_receipt(
        &mut client,
        source_revision_ref,
        &input_fingerprint_ref,
        &consumer_scope_ref,
    )? {
        return Ok(receipt);
    }

    let mut review_item_refs = BTreeSet::new();
    let mut cluster_review_items = 0usize;

    for row in pressure_rows {
        let pressure_ref: String = row.get(0);
        let kind: String = row.get(1);
        let fingerprint_ref: String = row.get(2);
        let _occurrence_count: i64 = row.get(3);
        let _source_revision_count: i64 = row.get(4);
        let reason_ref: String = row.get(5);
        let candidate_only: bool = row.get(6);
        let requires_review: bool = row.get(7);
        let creates_authority: bool = row.get(8);
        let truth: bool = row.get(9);

        if !candidate_only || !requires_review || creates_authority || truth {
            return Err(ReconciliationReviewError::PromotionBoundary);
        }

        let source_refs = load_occurrence_statement_refs(
            &mut client,
            &kind,
            &fingerprint_ref,
            source_revision_ref,
        )?;
        if source_refs.is_empty() {
            continue;
        }

        // This is a review of the candidate cluster itself, not reviewed
        // proposition/event identity. EventAssembly remains owned by S28.AUTO.
        let item_kind = match kind.as_str() {
            "proposition" => ReviewItemKind::PnfParse,
            "event" => ReviewItemKind::Observation,
            _ => continue,
        };
        let review_item_ref = format!(
            "review-item:reconciliation:{source_revision_ref}:{fingerprint_ref}"
        );
        let item = ReviewItem {
            review_item_ref: review_item_ref.clone(),
            semantic_ref: fingerprint_ref,
            item_kind,
            reason: format!(
                "automatic {kind} candidate cluster surfaced by {reason_ref}; review does not create semantic identity"
            ),
            provenance_refs: vec![pressure_ref],
            source_refs,
            current_status: ReviewStatus::Pending,
            available_actions: common_actions(),
            affected_consumer_refs: affected_consumer_refs.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        persist_review_item(config, &item)?;
        review_item_refs.insert(review_item_ref);
        cluster_review_items += 1;
    }

    let mut contestation_review_items = 0usize;
    for row in conflict_rows {
        let relation_ref: String = row.get(0);
        let base_signature_ref: String = row.get(1);
        let positive_ref: String = row.get(2);
        let negative_ref: String = row.get(3);
        let detector_ref: String = row.get(4);
        let candidate_only: bool = row.get(5);
        let requires_review: bool = row.get(6);
        let creates_identity: bool = row.get(7);
        let creates_authority: bool = row.get(8);
        let truth: bool = row.get(9);
        if !candidate_only
            || !requires_review
            || creates_identity
            || creates_authority
            || truth
        {
            return Err(ReconciliationReviewError::PromotionBoundary);
        }

        let mut source_refs = BTreeSet::new();
        for fingerprint in [&positive_ref, &negative_ref] {
            for statement_ref in load_occurrence_statement_refs(
                &mut client,
                "proposition",
                fingerprint,
                source_revision_ref,
            )? {
                source_refs.insert(statement_ref);
            }
        }
        if source_refs.is_empty() {
            continue;
        }

        let review_item_ref = format!(
            "review-item:{source_revision_ref}:{relation_ref}"
        );
        let item = ReviewItem {
            review_item_ref: review_item_ref.clone(),
            semantic_ref: relation_ref.clone(),
            item_kind: ReviewItemKind::ClaimContestation,
            reason: format!(
                "automatic polarity-conflict candidate for shared predicate/participant signature {base_signature_ref}; detector {detector_ref}; review is required before any contestation identity"
            ),
            provenance_refs: vec![positive_ref, negative_ref, relation_ref],
            source_refs: source_refs.into_iter().collect(),
            current_status: ReviewStatus::Pending,
            available_actions: common_actions(),
            affected_consumer_refs: affected_consumer_refs.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        persist_review_item(config, &item)?;
        review_item_refs.insert(review_item_ref);
        contestation_review_items += 1;
    }

    let receipt = ReconciliationReviewReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        stage_reused: false,
        input_fingerprint_ref: input_fingerprint_ref.clone(),
        cluster_review_items,
        contestation_review_items,
        review_item_refs: review_item_refs.into_iter().collect(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_event_identity: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };

    client.execute(
        r#"
        INSERT INTO semantic.reconciliation_review_projection_stage_receipt
        (source_revision_ref, algorithm_ref, input_fingerprint_ref,
         consumer_scope_ref, cluster_review_items, contestation_review_items,
         review_item_refs, candidate_only, creates_semantic_authority,
         creates_event_identity, applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,TRUE,FALSE,FALSE,FALSE,FALSE)
        ON CONFLICT (
          source_revision_ref, algorithm_ref, input_fingerprint_ref, consumer_scope_ref
        ) DO NOTHING
        "#,
        &[
            &source_revision_ref,
            &REVIEW_PROJECTION_ALGORITHM_REF,
            &input_fingerprint_ref,
            &consumer_scope_ref,
            &(receipt.cluster_review_items as i64),
            &(receipt.contestation_review_items as i64),
            &receipt.review_item_refs,
        ],
    )?;

    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_event_cluster_review_is_not_event_assembly() {
        assert_eq!(ReviewItemKind::Observation, ReviewItemKind::Observation);
        assert_ne!(ReviewItemKind::Observation, ReviewItemKind::EventAssembly);
    }

    #[test]
    fn review_actions_do_not_include_follow_authority_for_candidate_clusters() {
        assert!(!common_actions().contains(&ReviewAction::FollowAuthority));
    }
}
