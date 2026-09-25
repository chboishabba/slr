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
use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewItem, ReviewItemKind, ReviewStatus,
};
use thiserror::Error;

use crate::{
    install_review_workstation_schema, persist_review_item, DatabaseConfig,
    ReviewWorkstationStoreError,
};

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
    pub cluster_review_items: usize,
    pub contestation_review_items: usize,
    pub review_item_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_event_identity: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
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
) -> Result<Vec<String>, postgres::Error> {
    let sql = match kind {
        "proposition" => {
            "SELECT DISTINCT statement_ref
             FROM semantic.proposition_candidate_occurrence
             WHERE proposition_fingerprint_ref=$1
             ORDER BY statement_ref"
        }
        "event" => {
            "SELECT DISTINCT statement_ref
             FROM semantic.event_candidate_occurrence
             WHERE event_fingerprint_ref=$1
             ORDER BY statement_ref"
        }
        _ => unreachable!("fixed reconciliation semantic kind"),
    };
    Ok(client
        .query(sql, &[&fingerprint_ref])?
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

    let mut review_item_refs = BTreeSet::new();
    let mut cluster_review_items = 0usize;

    for row in pressure_rows {
        let pressure_ref: String = row.get(0);
        let kind: String = row.get(1);
        let fingerprint_ref: String = row.get(2);
        let occurrence_count: i64 = row.get(3);
        let source_revision_count: i64 = row.get(4);
        let reason_ref: String = row.get(5);
        let candidate_only: bool = row.get(6);
        let requires_review: bool = row.get(7);
        let creates_authority: bool = row.get(8);
        let truth: bool = row.get(9);

        if !candidate_only || !requires_review || creates_authority || truth {
            return Err(ReconciliationReviewError::PromotionBoundary);
        }

        let source_refs =
            load_occurrence_statement_refs(&mut client, &kind, &fingerprint_ref)?;
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
        let review_item_ref = format!("review-item:reconciliation:{fingerprint_ref}");
        let item = ReviewItem {
            review_item_ref: review_item_ref.clone(),
            semantic_ref: fingerprint_ref,
            item_kind,
            reason: format!(
                "automatic {kind} candidate cluster has {occurrence_count} occurrences across {source_revision_count} source revisions ({reason_ref}); review does not create semantic identity"
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
            )? {
                source_refs.insert(statement_ref);
            }
        }
        if source_refs.is_empty() {
            continue;
        }

        let review_item_ref = format!("review-item:{relation_ref}");
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

    Ok(ReconciliationReviewReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        cluster_review_items,
        contestation_review_items,
        review_item_refs: review_item_refs.into_iter().collect(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_event_identity: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
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
