//! SCALE-1 L2 -> S28.AUTO bridge.
//!
//! Durable Statement/PNF + reconciliation outputs are projected into generic
//! candidate observations. Postgres indexes their detector signals so corpus
//! scale event discovery need not compare every observation pair.
//!
//! SQL only preselects cross-family pairs sharing a temporal signal plus at
//! least one other signal kind. This avoids counting entity + an event
//! fingerprint derived from the same PNF factors as independent evidence.
//! The established S28.AUTO detector is then run on the bounded observations
//! and remains the owner of CandidateEventJoinProposal construction.
//!
//! No operation here creates event identity, semantic authority, applicability,
//! or claim truth.

use std::collections::BTreeSet;

use postgres::{Client, NoTls};
use sensiblaw_core::event_discovery::{
    discover_event_join_proposals, EventDiscoveryPolicy, EventJoinObservation,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    canonical_statement_observation_link_ref, install_event_discovery_schema,
    install_statement_trace_schema, persist_event_join_proposal_with_review,
    persist_statement_observation_links_with_client, DatabaseConfig, EventDiscoveryStoreError,
    StatementObservationDisposition, StatementObservationLink,
    StatementTraceStoreError,
};

pub const SCALE1_AUTO_EVENT_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;

CREATE TABLE IF NOT EXISTS semantic.scale1_auto_observation_candidate (
    observation_ref TEXT PRIMARY KEY,
    statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref) ON DELETE CASCADE,
    batch_ref TEXT NOT NULL REFERENCES pnf.statement_candidate_batch(batch_ref) ON DELETE CASCADE,
    parser_run_ref TEXT NOT NULL,
    source_revision_ref TEXT NOT NULL,
    source_family_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_observation_identity BOOLEAN NOT NULL CHECK (NOT creates_observation_identity),
    creates_event_identity BOOLEAN NOT NULL CHECK (NOT creates_event_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    UNIQUE (statement_ref, batch_ref, parser_run_ref)
);

CREATE INDEX IF NOT EXISTS scale1_auto_observation_source_idx
ON semantic.scale1_auto_observation_candidate(source_revision_ref, source_family_ref);

CREATE TABLE IF NOT EXISTS semantic.scale1_auto_observation_signal (
    observation_ref TEXT NOT NULL
      REFERENCES semantic.scale1_auto_observation_candidate(observation_ref)
      ON DELETE CASCADE,
    signal_kind_ref TEXT NOT NULL CHECK (
      signal_kind_ref IN ('entity','temporal','fingerprint')
    ),
    signal_ref TEXT NOT NULL,
    detector_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_event_identity BOOLEAN NOT NULL CHECK (NOT creates_event_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    PRIMARY KEY (observation_ref, signal_kind_ref, signal_ref, detector_ref)
);

CREATE INDEX IF NOT EXISTS scale1_auto_signal_lookup_idx
ON semantic.scale1_auto_observation_signal(signal_kind_ref, signal_ref, observation_ref);

CREATE TABLE IF NOT EXISTS semantic.scale1_auto_event_stage_receipt (
    source_revision_ref TEXT NOT NULL,
    parser_run_ref TEXT NOT NULL,
    detector_ref TEXT NOT NULL,
    policy_ref TEXT NOT NULL,
    consumer_scope_ref TEXT NOT NULL,
    observations_materialized BIGINT NOT NULL,
    bounded_pair_count BIGINT NOT NULL,
    proposal_count BIGINT NOT NULL,
    review_item_count BIGINT NOT NULL,
    proposal_refs TEXT[] NOT NULL,
    review_item_refs TEXT[] NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_observation_identity BOOLEAN NOT NULL CHECK (NOT creates_observation_identity),
    creates_event_identity BOOLEAN NOT NULL CHECK (NOT creates_event_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    PRIMARY KEY (
      source_revision_ref, parser_run_ref, detector_ref, policy_ref, consumer_scope_ref
    )
);

"#;

const DETECTOR_REF: &str = "scale1:auto-event-signal:v1";
const POLICY_REF: &str = "scale1:auto-event-default-policy:v1";

#[derive(Debug, Error)]
pub enum Scale1AutoEventError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("statement trace persistence error: {0}")]
    StatementTrace(#[from] StatementTraceStoreError),
    #[error("event discovery persistence error: {0}")]
    EventStore(#[from] EventDiscoveryStoreError),
    #[error("S28.AUTO detector rejected generated observation/proposal")]
    Detector,
    #[error("persisted AUTO candidate crossed a non-promotion boundary")]
    PromotionBoundary,
    #[error("AUTO observation has no source family")]
    MissingSourceFamily,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scale1AutoEventReceipt {
    pub source_revision_ref: String,
    pub stage_reused: bool,
    pub parser_run_ref: String,
    pub observations_materialized: usize,
    pub bounded_pair_count: usize,
    pub proposal_count: usize,
    pub review_item_count: usize,
    pub proposal_refs: Vec<String>,
    pub review_item_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_observation_identity: bool,
    pub creates_event_identity: bool,
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

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn month_number(value: &str) -> Option<u8> {
    Some(match value {
        "january" | "jan" => 1,
        "february" | "feb" => 2,
        "march" | "mar" => 3,
        "april" | "apr" => 4,
        "may" => 5,
        "june" | "jun" => 6,
        "july" | "jul" => 7,
        "august" | "aug" => 8,
        "september" | "sep" | "sept" => 9,
        "october" | "oct" => 10,
        "november" | "nov" => 11,
        "december" | "dec" => 12,
        _ => return None,
    })
}

fn parse_day(value: &str) -> Option<u8> {
    let trimmed = value
        .trim_end_matches(|c: char| c.is_ascii_punctuation())
        .trim_end_matches("st")
        .trim_end_matches("nd")
        .trim_end_matches("rd")
        .trim_end_matches("th");
    let day = trimmed.parse::<u8>().ok()?;
    (1..=31).contains(&day).then_some(day)
}

fn parse_year(value: &str) -> Option<u16> {
    let year = value
        .trim_matches(|c: char| c.is_ascii_punctuation())
        .parse::<u16>()
        .ok()?;
    (1000..=2999).contains(&year).then_some(year)
}

fn canonical_date_bucket(surface: &str) -> Option<String> {
    let lowered = surface.to_lowercase().replace(',', " ").replace('.', " ");
    let parts = lowered
        .split_whitespace()
        .map(|value| value.to_owned())
        .collect::<Vec<_>>();

    match parts.as_slice() {
        [year] => parse_year(year).map(|year| format!("{year:04}")),
        [month, year] => {
            let month = month_number(month)?;
            let year = parse_year(year)?;
            Some(format!("{year:04}-{month:02}"))
        }
        [first, second, year] => {
            let year = parse_year(year)?;
            if let (Some(month), Some(day)) = (month_number(first), parse_day(second)) {
                Some(format!("{year:04}-{month:02}-{day:02}"))
            } else if let (Some(day), Some(month)) = (parse_day(first), month_number(second)) {
                Some(format!("{year:04}-{month:02}-{day:02}"))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn temporal_bucket_ref(label_ref: &str, surface: &str) -> String {
    if label_ref.eq_ignore_ascii_case("DATE") {
        if let Some(canonical) = canonical_date_bucket(surface) {
            return format!("temporal-bucket:date:{canonical}");
        }
    }
    let normalized = normalize(surface);
    format!(
        "temporal-bucket:{}",
        digest_ref("scale1-temporal-bucket:v1", &[label_ref, &normalized])
    )
}

fn persist_signal(
    client: &mut impl postgres::GenericClient,
    observation_ref: &str,
    kind: &str,
    signal_ref: &str,
) -> Result<(), postgres::Error> {
    client.execute(
        r#"INSERT INTO semantic.scale1_auto_observation_signal
           (observation_ref, signal_kind_ref, signal_ref, detector_ref,
            candidate_only, creates_event_identity,
            creates_semantic_authority, claim_truth_promoted)
           VALUES ($1,$2,$3,$4,TRUE,FALSE,FALSE,FALSE)
           ON CONFLICT DO NOTHING"#,
        &[&observation_ref, &kind, &signal_ref, &DETECTOR_REF],
    )?;
    Ok(())
}

fn materialize_source_observations(
    config: &DatabaseConfig,
    source_revision_ref: &str,
    parser_run_ref: &str,
) -> Result<usize, Scale1AutoEventError> {
    install_statement_trace_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(SCALE1_AUTO_EVENT_SCHEMA_SQL)?;

    let source = client.query_opt(
        "SELECT source_family_ref
         FROM ingest.generic_source_revision
         WHERE source_revision_ref=$1
           AND candidate_only
           AND NOT creates_semantic_authority
           AND NOT applicability_promoted
           AND NOT claim_truth_promoted",
        &[&source_revision_ref],
    )?;
    let Some(source) = source else {
        return Err(Scale1AutoEventError::MissingSourceFamily);
    };
    let source_family_ref: String = source.get(0);

    let rows = client.query(
        r#"
        SELECT b.batch_ref, b.statement_ref, b.parser_receipt_ref
        FROM pnf.statement_candidate_batch b
        JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
        WHERE s.source_revision_ref=$1
          AND b.candidate_only
          AND NOT b.semantic_admission_paid
          AND NOT b.proposition_support_paid
          AND NOT b.applicability_paid
          AND NOT b.claim_truth_paid
        ORDER BY b.statement_ref, b.batch_ref
        "#,
        &[&source_revision_ref],
    )?;

    let mut observation_refs = Vec::with_capacity(rows.len());
    let mut batch_refs = Vec::with_capacity(rows.len());
    let mut statement_refs = Vec::with_capacity(rows.len());
    let mut parser_receipt_refs = Vec::with_capacity(rows.len());
    for row in rows {
        let batch_ref: String = row.get(0);
        let statement_ref: String = row.get(1);
        let parser_receipt_ref: String = row.get(2);
        let observation_ref = format!(
            "observation:scale1:{}",
            digest_ref(
                "scale1-auto-observation:v1",
                &[&statement_ref, &batch_ref, parser_run_ref],
            )
        );
        observation_refs.push(observation_ref);
        batch_refs.push(batch_ref);
        statement_refs.push(statement_ref);
        parser_receipt_refs.push(parser_receipt_ref);
    }

    if !observation_refs.is_empty() {
        let parser_runs = vec![parser_run_ref.to_owned(); observation_refs.len()];
        let source_revisions = vec![source_revision_ref.to_owned(); observation_refs.len()];
        let source_families = vec![source_family_ref.clone(); observation_refs.len()];
        client.execute(
            r#"
            INSERT INTO semantic.scale1_auto_observation_candidate
              (observation_ref, statement_ref, batch_ref, parser_run_ref,
               source_revision_ref, source_family_ref, candidate_only,
               creates_observation_identity, creates_event_identity,
               creates_semantic_authority, applicability_promoted,
               claim_truth_promoted)
            SELECT observation_ref, statement_ref, batch_ref, parser_run_ref,
                   source_revision_ref, source_family_ref,
                   TRUE,FALSE,FALSE,FALSE,FALSE,FALSE
            FROM UNNEST(
              $1::TEXT[], $2::TEXT[], $3::TEXT[], $4::TEXT[],
              $5::TEXT[], $6::TEXT[]
            ) AS u(
              observation_ref, statement_ref, batch_ref, parser_run_ref,
              source_revision_ref, source_family_ref
            )
            ON CONFLICT (observation_ref) DO NOTHING
            "#,
            &[
                &observation_refs,
                &statement_refs,
                &batch_refs,
                &parser_runs,
                &source_revisions,
                &source_families,
            ],
        )?;
    }

    // Entity signals are already deterministic fingerprints. Project all of
    // them for this source/parser run in two set-wise statements.
    client.execute(
        r#"
        INSERT INTO semantic.scale1_auto_observation_signal
          (observation_ref, signal_kind_ref, signal_ref, detector_ref,
           candidate_only, creates_event_identity,
           creates_semantic_authority, claim_truth_promoted)
        SELECT DISTINCT o.observation_ref, 'entity',
               n.entity_fingerprint_ref, $3, TRUE,FALSE,FALSE,FALSE
        FROM semantic.scale1_auto_observation_candidate o
        JOIN semantic.named_entity_candidate n
          ON n.statement_ref=o.statement_ref
         AND n.parser_run_ref=o.parser_run_ref
        WHERE o.source_revision_ref=$1 AND o.parser_run_ref=$2
        ON CONFLICT DO NOTHING
        "#,
        &[&source_revision_ref, &parser_run_ref, &DETECTOR_REF],
    )?;
    client.execute(
        r#"
        INSERT INTO semantic.scale1_auto_observation_signal
          (observation_ref, signal_kind_ref, signal_ref, detector_ref,
           candidate_only, creates_event_identity,
           creates_semantic_authority, claim_truth_promoted)
        SELECT DISTINCT o.observation_ref, 'entity',
               e.entity_fingerprint_ref, $3, TRUE,FALSE,FALSE,FALSE
        FROM semantic.scale1_auto_observation_candidate o
        JOIN semantic.entity_mention_candidate e
          ON e.statement_ref=o.statement_ref
        WHERE o.source_revision_ref=$1 AND o.parser_run_ref=$2
        ON CONFLICT DO NOTHING
        "#,
        &[&source_revision_ref, &parser_run_ref, &DETECTOR_REF],
    )?;

    // Event fingerprints are likewise already normalized; project them set-wise.
    client.execute(
        r#"
        INSERT INTO semantic.scale1_auto_observation_signal
          (observation_ref, signal_kind_ref, signal_ref, detector_ref,
           candidate_only, creates_event_identity,
           creates_semantic_authority, claim_truth_promoted)
        SELECT DISTINCT o.observation_ref, 'fingerprint',
               e.event_fingerprint_ref, $3, TRUE,FALSE,FALSE,FALSE
        FROM semantic.scale1_auto_observation_candidate o
        JOIN semantic.event_candidate_occurrence e
          ON e.statement_ref=o.statement_ref
         AND e.source_revision_ref=o.source_revision_ref
        WHERE o.source_revision_ref=$1 AND o.parser_run_ref=$2
        ON CONFLICT DO NOTHING
        "#,
        &[&source_revision_ref, &parser_run_ref, &DETECTOR_REF],
    )?;

    // Date/time bucketing deliberately remains Rust-owned; load all mentions in
    // one query and bulk persist the normalized detector signals.
    let temporal_rows = client.query(
        r#"
        SELECT o.observation_ref, t.label_ref, t.surface
        FROM semantic.scale1_auto_observation_candidate o
        JOIN semantic.temporal_mention_candidate t
          ON t.statement_ref=o.statement_ref
         AND t.parser_run_ref=o.parser_run_ref
        WHERE o.source_revision_ref=$1 AND o.parser_run_ref=$2
        ORDER BY o.observation_ref, t.start_char, t.end_char, t.temporal_mention_ref
        "#,
        &[&source_revision_ref, &parser_run_ref],
    )?;
    if !temporal_rows.is_empty() {
        let mut temporal_observations = Vec::with_capacity(temporal_rows.len());
        let mut temporal_refs = Vec::with_capacity(temporal_rows.len());
        for row in temporal_rows {
            let observation_ref: String = row.get(0);
            let label_ref: String = row.get(1);
            let surface: String = row.get(2);
            temporal_observations.push(observation_ref);
            temporal_refs.push(temporal_bucket_ref(&label_ref, &surface));
        }
        client.execute(
            r#"
            INSERT INTO semantic.scale1_auto_observation_signal
              (observation_ref, signal_kind_ref, signal_ref, detector_ref,
               candidate_only, creates_event_identity,
               creates_semantic_authority, claim_truth_promoted)
            SELECT observation_ref, 'temporal', signal_ref, $3,
                   TRUE,FALSE,FALSE,FALSE
            FROM UNNEST($1::TEXT[], $2::TEXT[]) AS u(observation_ref, signal_ref)
            ON CONFLICT DO NOTHING
            "#,
            &[&temporal_observations, &temporal_refs, &DETECTOR_REF],
        )?;
    }

    // Trace links retain the same canonical digest and full persisted reopen
    // comparison, but are written and reopened set-wise.
    let links = observation_refs
        .iter()
        .zip(batch_refs.iter())
        .zip(statement_refs.iter())
        .zip(parser_receipt_refs.iter())
        .map(|(((observation_ref, batch_ref), statement_ref), parser_receipt_ref)| {
            StatementObservationLink {
                link_ref: canonical_statement_observation_link_ref(
                    statement_ref,
                    batch_ref,
                    observation_ref,
                ),
                statement_ref: statement_ref.clone(),
                candidate_pnf_ref: batch_ref.clone(),
                observation_ref: observation_ref.clone(),
                parser_receipt_ref: Some(parser_receipt_ref.clone()),
                parse_review_ref: None,
                admission_receipt_ref: None,
                disposition: StatementObservationDisposition::Candidate,
                qualification_ref: None,
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            }
        })
        .collect::<Vec<_>>();
    let persisted_links =
        persist_statement_observation_links_with_client(&mut client, &links)?;
    if persisted_links.len() != links.len() {
        return Err(Scale1AutoEventError::PromotionBoundary);
    }

    Ok(observation_refs.len())
}

fn bounded_pair_refs(
    client: &mut Client,
    source_revision_ref: &str,
) -> Result<Vec<(String, String)>, postgres::Error> {
    let rows = client.query(
        r#"
        WITH touched AS (
          SELECT observation_ref
          FROM semantic.scale1_auto_observation_candidate
          WHERE source_revision_ref=$1
        ),
        pair_signal AS (
          SELECT LEAST(a.observation_ref, b.observation_ref) AS left_ref,
                 GREATEST(a.observation_ref, b.observation_ref) AS right_ref,
                 a.signal_kind_ref
          FROM semantic.scale1_auto_observation_signal a
          JOIN semantic.scale1_auto_observation_signal b
            ON a.signal_kind_ref=b.signal_kind_ref
           AND a.signal_ref=b.signal_ref
           AND a.observation_ref < b.observation_ref
          JOIN semantic.scale1_auto_observation_candidate oa
            ON oa.observation_ref=a.observation_ref
          JOIN semantic.scale1_auto_observation_candidate ob
            ON ob.observation_ref=b.observation_ref
          WHERE oa.source_family_ref <> ob.source_family_ref
            AND (
              a.observation_ref IN (SELECT observation_ref FROM touched)
              OR b.observation_ref IN (SELECT observation_ref FROM touched)
            )
            AND oa.candidate_only AND ob.candidate_only
            AND NOT oa.creates_event_identity AND NOT ob.creates_event_identity
            AND NOT oa.creates_semantic_authority
            AND NOT ob.creates_semantic_authority
        )
        SELECT left_ref, right_ref
        FROM pair_signal
        GROUP BY left_ref, right_ref
        HAVING COUNT(DISTINCT signal_kind_ref) >= 2
           AND BOOL_OR(signal_kind_ref = 'temporal')
        ORDER BY left_ref, right_ref
        "#,
        &[&source_revision_ref],
    )?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get::<_, String>(0), row.get::<_, String>(1)))
        .collect())
}

fn load_observation(
    client: &mut Client,
    observation_ref: &str,
) -> Result<EventJoinObservation, Scale1AutoEventError> {
    let row = client.query_one(
        r#"
        SELECT statement_ref, source_family_ref, candidate_only,
               creates_observation_identity, creates_event_identity,
               creates_semantic_authority, applicability_promoted,
               claim_truth_promoted
        FROM semantic.scale1_auto_observation_candidate
        WHERE observation_ref=$1
        "#,
        &[&observation_ref],
    )?;
    if !row.get::<_, bool>(2)
        || row.get::<_, bool>(3)
        || row.get::<_, bool>(4)
        || row.get::<_, bool>(5)
        || row.get::<_, bool>(6)
        || row.get::<_, bool>(7)
    {
        return Err(Scale1AutoEventError::PromotionBoundary);
    }
    let statement_ref: String = row.get(0);
    let source_family_ref: String = row.get(1);

    let signal_rows = client.query(
        r#"
        SELECT signal_kind_ref, signal_ref
        FROM semantic.scale1_auto_observation_signal
        WHERE observation_ref=$1
          AND candidate_only
          AND NOT creates_event_identity
          AND NOT creates_semantic_authority
          AND NOT claim_truth_promoted
        ORDER BY signal_kind_ref, signal_ref
        "#,
        &[&observation_ref],
    )?;

    let mut entity_refs = Vec::new();
    let mut temporal_bucket_refs = Vec::new();
    let mut fingerprint_refs = Vec::new();
    for signal in signal_rows {
        let kind: String = signal.get(0);
        let reference: String = signal.get(1);
        match kind.as_str() {
            "entity" => entity_refs.push(reference),
            "temporal" => temporal_bucket_refs.push(reference),
            "fingerprint" => fingerprint_refs.push(reference),
            _ => return Err(Scale1AutoEventError::PromotionBoundary),
        }
    }

    Ok(EventJoinObservation {
        observation_ref: observation_ref.to_owned(),
        statement_refs: vec![statement_ref],
        source_family_ref,
        qid_refs: vec![],
        entity_refs,
        temporal_bucket_refs,
        fingerprint_refs,
        explicit_cross_reference_refs: vec![],
        user_declared_same_incident_refs: vec![],
    })
}

fn consumer_scope_ref(affected_consumer_refs: &[String]) -> String {
    let mut refs = affected_consumer_refs.to_vec();
    refs.sort();
    refs.dedup();
    let borrowed = refs.iter().map(String::as_str).collect::<Vec<_>>();
    digest_ref("scale1-auto-consumer-scope:v1", &borrowed)
}

fn load_auto_stage_receipt(
    client: &mut Client,
    source_revision_ref: &str,
    parser_run_ref: &str,
    consumer_scope_ref: &str,
) -> Result<Option<Scale1AutoEventReceipt>, Scale1AutoEventError> {
    let Some(row) = client.query_opt(
        r#"
        SELECT observations_materialized, bounded_pair_count, proposal_count,
               review_item_count, proposal_refs, review_item_refs,
               candidate_only, creates_observation_identity,
               creates_event_identity, creates_semantic_authority,
               applicability_promoted, claim_truth_promoted
        FROM semantic.scale1_auto_event_stage_receipt
        WHERE source_revision_ref=$1
          AND parser_run_ref=$2
          AND detector_ref=$3
          AND policy_ref=$4
          AND consumer_scope_ref=$5
        "#,
        &[&source_revision_ref, &parser_run_ref, &DETECTOR_REF, &POLICY_REF, &consumer_scope_ref],
    )? else {
        return Ok(None);
    };
    let count = |idx: usize| row.get::<_, i64>(idx).max(0) as usize;
    let receipt = Scale1AutoEventReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        stage_reused: true,
        parser_run_ref: parser_run_ref.to_owned(),
        observations_materialized: count(0),
        bounded_pair_count: count(1),
        proposal_count: count(2),
        review_item_count: count(3),
        proposal_refs: row.get(4),
        review_item_refs: row.get(5),
        candidate_only: row.get(6),
        creates_observation_identity: row.get(7),
        creates_event_identity: row.get(8),
        creates_semantic_authority: row.get(9),
        applicability_promoted: row.get(10),
        claim_truth_promoted: row.get(11),
    };
    if !receipt.candidate_only
        || receipt.creates_observation_identity
        || receipt.creates_event_identity
        || receipt.creates_semantic_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
    {
        return Err(Scale1AutoEventError::PromotionBoundary);
    }
    Ok(Some(receipt))
}

pub fn discover_scale1_auto_event_proposals(
    config: &DatabaseConfig,
    source_revision_ref: &str,
    parser_run_ref: &str,
    affected_consumer_refs: Vec<String>,
) -> Result<Scale1AutoEventReceipt, Scale1AutoEventError> {
    install_event_discovery_schema(config)?;
    let consumer_scope_ref = consumer_scope_ref(&affected_consumer_refs);
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(SCALE1_AUTO_EVENT_SCHEMA_SQL)?;
    if let Some(receipt) = load_auto_stage_receipt(
        &mut client,
        source_revision_ref,
        parser_run_ref,
        &consumer_scope_ref,
    )? {
        return Ok(receipt);
    }

    let observations_materialized =
        materialize_source_observations(config, source_revision_ref, parser_run_ref)?;
    let pairs = bounded_pair_refs(&mut client, source_revision_ref)?;

    let policy = EventDiscoveryPolicy::default();
    let mut proposal_refs = BTreeSet::new();
    let mut review_item_refs = BTreeSet::new();

    for (left_ref, right_ref) in &pairs {
        let observations = vec![
            load_observation(&mut client, left_ref)?,
            load_observation(&mut client, right_ref)?,
        ];
        let proposals = discover_event_join_proposals(&observations, &policy)
            .map_err(|_| Scale1AutoEventError::Detector)?;
        for proposal in proposals {
            let (persisted, review) =
                persist_event_join_proposal_with_review(
                    config,
                    &proposal,
                    affected_consumer_refs.clone(),
                )?;
            proposal_refs.insert(persisted.proposal_ref);
            review_item_refs.insert(review.review_item_ref);
        }
    }

    let receipt = Scale1AutoEventReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        stage_reused: false,
        parser_run_ref: parser_run_ref.to_owned(),
        observations_materialized,
        bounded_pair_count: pairs.len(),
        proposal_count: proposal_refs.len(),
        review_item_count: review_item_refs.len(),
        proposal_refs: proposal_refs.into_iter().collect(),
        review_item_refs: review_item_refs.into_iter().collect(),
        candidate_only: true,
        creates_observation_identity: false,
        creates_event_identity: false,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };

    client.execute(
        r#"
        INSERT INTO semantic.scale1_auto_event_stage_receipt
        (source_revision_ref, parser_run_ref, detector_ref, policy_ref,
         consumer_scope_ref, observations_materialized, bounded_pair_count,
         proposal_count, review_item_count, proposal_refs, review_item_refs,
         candidate_only, creates_observation_identity, creates_event_identity,
         creates_semantic_authority, applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,
                TRUE,FALSE,FALSE,FALSE,FALSE,FALSE)
        ON CONFLICT (
          source_revision_ref, parser_run_ref, detector_ref, policy_ref, consumer_scope_ref
        ) DO NOTHING
        "#,
        &[
            &source_revision_ref,
            &parser_run_ref,
            &DETECTOR_REF,
            &POLICY_REF,
            &consumer_scope_ref,
            &(receipt.observations_materialized as i64),
            &(receipt.bounded_pair_count as i64),
            &(receipt.proposal_count as i64),
            &(receipt.review_item_count as i64),
            &receipt.proposal_refs,
            &receipt.review_item_refs,
        ],
    )?;

    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporal_bucket_is_detector_identity_not_temporal_assertion() {
        assert_eq!(
            temporal_bucket_ref("DATE", " January  1, 1997 "),
            temporal_bucket_ref("DATE", "1 Jan 1997")
        );
        assert_eq!(
            temporal_bucket_ref("DATE", "January 1st, 1997"),
            "temporal-bucket:date:1997-01-01"
        );
        assert_eq!(
            temporal_bucket_ref("DATE", "Jan. 1997"),
            "temporal-bucket:date:1997-01"
        );
        assert_eq!(
            temporal_bucket_ref("DATE", "1997"),
            "temporal-bucket:date:1997"
        );
    }

    #[test]
    fn consumer_scope_is_order_and_duplicate_invariant() {
        let a = vec!["consumer:b".to_owned(), "consumer:a".to_owned()];
        let b = vec![
            "consumer:a".to_owned(),
            "consumer:b".to_owned(),
            "consumer:a".to_owned(),
        ];
        assert_eq!(consumer_scope_ref(&a), consumer_scope_ref(&b));
        assert_ne!(
            consumer_scope_ref(&a),
            consumer_scope_ref(&["consumer:c".to_owned()])
        );
    }

    #[test]
    fn detector_domains_do_not_collapse() {
        assert_ne!(
            digest_ref("scale1-temporal-bucket:v1", &["DATE", "1997"]),
            digest_ref("scale1-auto-observation:v1", &["DATE", "1997"])
        );
    }
}
