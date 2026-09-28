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

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewItem, ReviewItemKind, ReviewStatus,
};
use thiserror::Error;

use crate::corpus_reconciliation_store::CORPUS_RECONCILIATION_DETECTOR_REF;
use crate::review_workstation_store::persist_review_projection_item_with_client;
use crate::{
    install_review_workstation_schema, DatabaseConfig, ReviewWorkstationStoreError,
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

CREATE TABLE IF NOT EXISTS semantic.reconciliation_review_projection_stage_receipt_v2 (
    source_revision_ref TEXT NOT NULL,
    parser_run_ref TEXT NOT NULL,
    reconciliation_detector_ref TEXT NOT NULL,
    algorithm_ref TEXT NOT NULL,
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
      source_revision_ref, parser_run_ref, reconciliation_detector_ref,
      algorithm_ref, consumer_scope_ref
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
    pub pressure_rows_scanned: usize,
    pub contestation_rows_scanned: usize,
    pub occurrence_rows_scanned: usize,
    pub occurrence_lookup_count: usize,
    pub review_items_persist_attempted: usize,
    pub persisted_refs_verified: usize,
    pub input_identity_ns: u128,
    pub materialize_ns: u128,
    pub stage_receipt_ns: u128,
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
    pressure_components: &[String],
    contestation_components: &[String],
    occurrence_components: &[String],
) -> String {
    let mut parts = Vec::with_capacity(
        pressure_components.len()
            + contestation_components.len()
            + occurrence_components.len()
            + 3,
    );
    parts.push(format!("pressure-count:{}", pressure_components.len()));
    parts.extend(pressure_components.iter().map(|value| format!("pressure:{value}")));
    parts.push(format!(
        "contestation-count:{}",
        contestation_components.len()
    ));
    parts.extend(
        contestation_components
            .iter()
            .map(|value| format!("contestation:{value}")),
    );
    parts.push(format!("occurrence-count:{}", occurrence_components.len()));
    parts.extend(
        occurrence_components
            .iter()
            .map(|value| format!("occurrence:{value}")),
    );
    let borrowed = parts.iter().map(String::as_str).collect::<Vec<_>>();
    digest_ref("reconciliation-review-input:v2", &borrowed)
}

fn upstream_review_projection_ref(
    source_revision_ref: &str,
    parser_run_ref: &str,
) -> String {
    digest_ref(
        "reconciliation-review-upstream:v1",
        &[
            source_revision_ref,
            parser_run_ref,
            CORPUS_RECONCILIATION_DETECTOR_REF,
            REVIEW_PROJECTION_ALGORITHM_REF,
        ],
    )
}

fn load_fast_reusable_projection_receipt(
    client: &mut Client,
    source_revision_ref: &str,
    parser_run_ref: &str,
    consumer_scope_ref: &str,
) -> Result<Option<ReconciliationReviewReceipt>, ReconciliationReviewError> {
    let upstream = client.query_opt(
        r#"
        SELECT candidate_only, creates_semantic_authority,
               creates_entity_identity, creates_proposition_identity,
               creates_event_identity, claim_truth_promoted
        FROM semantic.corpus_reconciliation_stage_receipt
        WHERE source_revision_ref=$1
          AND parser_run_ref=$2
          AND detector_ref=$3
        "#,
        &[
            &source_revision_ref,
            &parser_run_ref,
            &CORPUS_RECONCILIATION_DETECTOR_REF,
        ],
    )?;
    let Some(upstream) = upstream else {
        return Ok(None);
    };
    if !upstream.get::<_, bool>(0)
        || upstream.get::<_, bool>(1)
        || upstream.get::<_, bool>(2)
        || upstream.get::<_, bool>(3)
        || upstream.get::<_, bool>(4)
        || upstream.get::<_, bool>(5)
    {
        return Err(ReconciliationReviewError::PromotionBoundary);
    }

    let Some(row) = client.query_opt(
        r#"
        SELECT cluster_review_items, contestation_review_items, review_item_refs,
               candidate_only, creates_semantic_authority, creates_event_identity,
               applicability_promoted, claim_truth_promoted
        FROM semantic.reconciliation_review_projection_stage_receipt_v2
        WHERE source_revision_ref=$1
          AND parser_run_ref=$2
          AND reconciliation_detector_ref=$3
          AND algorithm_ref=$4
          AND consumer_scope_ref=$5
        "#,
        &[
            &source_revision_ref,
            &parser_run_ref,
            &CORPUS_RECONCILIATION_DETECTOR_REF,
            &REVIEW_PROJECTION_ALGORITHM_REF,
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
    if boundaries.get::<_, i64>(0) != review_item_refs.len() as i64
        || !boundaries.get::<_, bool>(1)
    {
        return Ok(None);
    }

    let receipt = ReconciliationReviewReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        stage_reused: true,
        input_fingerprint_ref: upstream_review_projection_ref(
            source_revision_ref,
            parser_run_ref,
        ),
        pressure_rows_scanned: 0,
        contestation_rows_scanned: 0,
        occurrence_rows_scanned: 0,
        occurrence_lookup_count: 0,
        review_items_persist_attempted: 0,
        persisted_refs_verified: review_item_refs.len(),
        input_identity_ns: 0,
        materialize_ns: 0,
        stage_receipt_ns: 0,
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

fn load_reusable_projection_receipt(
    client: &mut Client,
    source_revision_ref: &str,
    input_fingerprint_ref: &str,
    consumer_scope_ref: &str,
    pressure_rows_scanned: usize,
    contestation_rows_scanned: usize,
    occurrence_rows_scanned: usize,
    occurrence_lookup_count: usize,
    input_identity_ns: u128,
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
        pressure_rows_scanned,
        contestation_rows_scanned,
        occurrence_rows_scanned,
        occurrence_lookup_count,
        review_items_persist_attempted: 0,
        persisted_refs_verified: review_item_refs.len(),
        input_identity_ns,
        materialize_ns: 0,
        stage_receipt_ns: 0,
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

pub fn enqueue_reconciliation_review_items_for_parser_run(
    config: &DatabaseConfig,
    source_revision_ref: &str,
    parser_run_ref: &str,
    affected_consumer_refs: Vec<String>,
) -> Result<ReconciliationReviewReceipt, ReconciliationReviewError> {
    install_review_workstation_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let input_identity_started = Instant::now();
    client.batch_execute(REVIEW_PROJECTION_STAGE_SQL)?;
    let consumer_scope_ref = canonical_scope_ref(&affected_consumer_refs);

    if let Some(mut receipt) = load_fast_reusable_projection_receipt(
        &mut client,
        source_revision_ref,
        parser_run_ref,
        &consumer_scope_ref,
    )? {
        receipt.input_identity_ns = input_identity_started.elapsed().as_nanos();
        return Ok(receipt);
    }

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


    let occurrence_rows = client.query(
        r#"
        SELECT 'proposition'::TEXT AS semantic_kind_ref,
               proposition_fingerprint_ref AS semantic_fingerprint_ref,
               statement_ref
        FROM semantic.proposition_candidate_occurrence
        WHERE source_revision_ref=$1
        UNION ALL
        SELECT 'event'::TEXT,
               event_fingerprint_ref,
               statement_ref
        FROM semantic.event_candidate_occurrence
        WHERE source_revision_ref=$1
        ORDER BY 1,2,3
        "#,
        &[&source_revision_ref],
    )?;
    let occurrence_lookup_count = 1usize;
    let occurrence_rows_scanned = occurrence_rows.len();
    let mut occurrence_refs =
        BTreeMap::<(String, String), BTreeSet<String>>::new();
    let mut occurrence_components = Vec::with_capacity(occurrence_rows.len());
    for row in occurrence_rows {
        let kind: String = row.get(0);
        let fingerprint: String = row.get(1);
        let statement_ref: String = row.get(2);
        occurrence_components.push(format!(
            "{kind}\u{1f}{fingerprint}\u{1f}{statement_ref}"
        ));
        occurrence_refs
            .entry((kind, fingerprint))
            .or_default()
            .insert(statement_ref);
    }

    let pressure_components = pressure_rows
        .iter()
        .map(|row| {
            format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}",
                row.get::<_, String>(0),
                row.get::<_, String>(1),
                row.get::<_, String>(2),
                row.get::<_, String>(5),
            )
        })
        .collect::<Vec<_>>();
    let contestation_components = conflict_rows
        .iter()
        .map(|row| {
            format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                row.get::<_, String>(0),
                row.get::<_, String>(1),
                row.get::<_, String>(2),
                row.get::<_, String>(3),
                row.get::<_, String>(4),
            )
        })
        .collect::<Vec<_>>();
    let input_fingerprint_ref = review_input_fingerprint(
        &pressure_components,
        &contestation_components,
        &occurrence_components,
    );
    let pressure_rows_scanned = pressure_rows.len();
    let contestation_rows_scanned = conflict_rows.len();
    let input_identity_ns = input_identity_started.elapsed().as_nanos();

    if let Some(receipt) = load_reusable_projection_receipt(
        &mut client,
        source_revision_ref,
        &input_fingerprint_ref,
        &consumer_scope_ref,
        pressure_rows_scanned,
        contestation_rows_scanned,
        occurrence_rows_scanned,
        occurrence_lookup_count,
        input_identity_ns,
    )? {
        return Ok(receipt);
    }

    let materialize_started = Instant::now();
    let mut review_item_refs = BTreeSet::new();
    let mut cluster_review_items = 0usize;
       let mut review_items_persist_attempted = 0usize;

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

        let source_refs = occurrence_refs
            .get(&(kind.clone(), fingerprint_ref.clone()))
            .map(|refs| refs.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
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
        review_items_persist_attempted += 1;
        persist_review_projection_item_with_client(&mut client, &item)?;
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
            if let Some(refs) =
                occurrence_refs.get(&("proposition".to_owned(), fingerprint.clone()))
            {
                source_refs.extend(refs.iter().cloned());
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
        review_items_persist_attempted += 1;
        persist_review_projection_item_with_client(&mut client, &item)?;
        review_item_refs.insert(review_item_ref);
        contestation_review_items += 1;
    }

    let materialize_ns = materialize_started.elapsed().as_nanos();
    let stage_receipt_started = Instant::now();
    let mut receipt = ReconciliationReviewReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        stage_reused: false,
        input_fingerprint_ref: input_fingerprint_ref.clone(),
        pressure_rows_scanned,
        contestation_rows_scanned,
        occurrence_rows_scanned,
        occurrence_lookup_count,
        review_items_persist_attempted,
        persisted_refs_verified: review_item_refs.len(),
        input_identity_ns,
        materialize_ns,
        stage_receipt_ns: 0,
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
    client.execute(
        r#"
        INSERT INTO semantic.reconciliation_review_projection_stage_receipt_v2
        (source_revision_ref, parser_run_ref, reconciliation_detector_ref,
         algorithm_ref, consumer_scope_ref, cluster_review_items,
         contestation_review_items, review_item_refs, candidate_only,
         creates_semantic_authority, creates_event_identity,
         applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,TRUE,FALSE,FALSE,FALSE,FALSE)
        ON CONFLICT (
          source_revision_ref, parser_run_ref, reconciliation_detector_ref,
          algorithm_ref, consumer_scope_ref
        ) DO NOTHING
        "#,
        &[
            &source_revision_ref,
            &parser_run_ref,
            &CORPUS_RECONCILIATION_DETECTOR_REF,
            &REVIEW_PROJECTION_ALGORITHM_REF,
            &consumer_scope_ref,
            &(receipt.cluster_review_items as i64),
            &(receipt.contestation_review_items as i64),
            &receipt.review_item_refs,
        ],
    )?;
    receipt.stage_receipt_ns = stage_receipt_started.elapsed().as_nanos();

    Ok(receipt)
}

pub fn enqueue_reconciliation_review_items(
    config: &DatabaseConfig,
    source_revision_ref: &str,
    affected_consumer_refs: Vec<String>,
) -> Result<ReconciliationReviewReceipt, ReconciliationReviewError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(REVIEW_PROJECTION_STAGE_SQL)?;
    let row = client.query_opt(
        r#"
        SELECT s.parser_run_ref
        FROM semantic.corpus_reconciliation_stage_receipt s
        JOIN ingest.parser_run r ON r.parser_run_ref=s.parser_run_ref
        WHERE s.source_revision_ref=$1
          AND s.detector_ref=$2
        ORDER BY r.completed_at DESC NULLS LAST, r.created_at DESC, s.parser_run_ref
        LIMIT 1
        "#,
        &[&source_revision_ref, &CORPUS_RECONCILIATION_DETECTOR_REF],
    )?;
    let Some(row) = row else {
        return Err(ReconciliationReviewError::PromotionBoundary);
    };
    let parser_run_ref: String = row.get(0);
    enqueue_reconciliation_review_items_for_parser_run(
        config,
        source_revision_ref,
        &parser_run_ref,
        affected_consumer_refs,
    )
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
