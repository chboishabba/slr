//! Normalized PostgreSQL persistence for production forecast verification.
//!
//! No semantic JSON authority surface is used here. Forecast identity,
//! resolution revisions, temporal evidence availability, cohort membership,
//! score snapshots and residual/research dependencies are stored as typed,
//! normalized rows.

use postgres::Client;
use sensiblaw_reader_model::{
    BinaryOutcome, ForecastCohort, ForecastDomain, ForecastEvidenceAvailability,
    ForecastOrigin, ForecastResearchDemand, ForecastResearchProbeKind,
    ForecastResidualKind, ForecastResidualProjection, ForecastResolutionRevision,
    ForecastResolutionState, ForecastScoreProjection, Probability,
    PublishedBinaryForecast, Rational, ResolutionEvidence, ScorableForecast,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ForecastStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("invalid forecast persistence state: {0}")]
    Invalid(String),
    #[error("integer conversion overflow for {0}")]
    Overflow(&'static str),
}

fn as_i64(value: u64, label: &'static str) -> Result<i64, ForecastStoreError> {
    i64::try_from(value).map_err(|_| ForecastStoreError::Overflow(label))
}

fn as_u64(value: i64, label: &'static str) -> Result<u64, ForecastStoreError> {
    u64::try_from(value).map_err(|_| ForecastStoreError::Invalid(format!("{label} is negative")))
}

fn domain_db(value: ForecastDomain) -> &'static str {
    match value {
        ForecastDomain::Conflict => "conflict",
        ForecastDomain::Cyber => "cyber",
        ForecastDomain::Energy => "energy",
        ForecastDomain::Geopolitical => "geopolitical",
        ForecastDomain::Infrastructure => "infrastructure",
        ForecastDomain::Macro => "macro",
        ForecastDomain::Market => "market",
        ForecastDomain::Military => "military",
        ForecastDomain::Political => "political",
        ForecastDomain::SupplyChain => "supply_chain",
        ForecastDomain::Other => "other",
    }
}

fn parse_domain(value: &str) -> Result<ForecastDomain, ForecastStoreError> {
    Ok(match value {
        "conflict" => ForecastDomain::Conflict,
        "cyber" => ForecastDomain::Cyber,
        "energy" => ForecastDomain::Energy,
        "geopolitical" => ForecastDomain::Geopolitical,
        "infrastructure" => ForecastDomain::Infrastructure,
        "macro" => ForecastDomain::Macro,
        "market" => ForecastDomain::Market,
        "military" => ForecastDomain::Military,
        "political" => ForecastDomain::Political,
        "supply_chain" => ForecastDomain::SupplyChain,
        "other" => ForecastDomain::Other,
        other => return Err(ForecastStoreError::Invalid(format!("unknown forecast domain {other}"))),
    })
}

fn origin_db(value: ForecastOrigin) -> &'static str {
    match value {
        ForecastOrigin::BetEngine => "bet_engine",
        ForecastOrigin::LegacyDetector => "legacy_detector",
        ForecastOrigin::StateDerived => "state_derived",
        ForecastOrigin::Unknown => "unknown",
        ForecastOrigin::Other => "other",
    }
}

fn parse_origin(value: &str) -> Result<ForecastOrigin, ForecastStoreError> {
    Ok(match value {
        "bet_engine" => ForecastOrigin::BetEngine,
        "legacy_detector" => ForecastOrigin::LegacyDetector,
        "state_derived" => ForecastOrigin::StateDerived,
        "unknown" => ForecastOrigin::Unknown,
        "other" => ForecastOrigin::Other,
        other => return Err(ForecastStoreError::Invalid(format!("unknown forecast origin {other}"))),
    })
}

pub fn ensure_forecast_verification_schema(
    client: &mut Client,
) -> Result<(), ForecastStoreError> {
    client.batch_execute(
        r#"
        CREATE SCHEMA IF NOT EXISTS forecast;

        CREATE TABLE IF NOT EXISTS forecast.published_forecast (
            forecast_ref TEXT PRIMARY KEY,
            proposition_ref TEXT NOT NULL,
            issued_at_epoch_ms BIGINT NOT NULL,
            horizon_ref TEXT NOT NULL,
            probability_numerator BIGINT NOT NULL CHECK (probability_numerator >= 0),
            probability_denominator BIGINT NOT NULL CHECK (probability_denominator > 0),
            domain_ref TEXT NOT NULL,
            origin_ref TEXT NOT NULL,
            model_ref TEXT NOT NULL,
            source_state_ref TEXT NOT NULL,
            resolution_policy_ref TEXT NOT NULL,
            candidate_only BOOLEAN NOT NULL DEFAULT TRUE CHECK (candidate_only),
            creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
            creates_claim_truth BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_claim_truth),
            CHECK (probability_numerator <= probability_denominator)
        );

        CREATE TABLE IF NOT EXISTS forecast.resolution_revision (
            resolution_revision_ref TEXT PRIMARY KEY,
            forecast_ref TEXT NOT NULL REFERENCES forecast.published_forecast(forecast_ref),
            previous_revision_ref TEXT NULL REFERENCES forecast.resolution_revision(resolution_revision_ref),
            state_kind TEXT NOT NULL CHECK (state_kind IN ('open','awaiting_judge','scored','voided')),
            outcome_ref TEXT NULL CHECK (outcome_ref IS NULL OR outcome_ref IN ('no','yes')),
            void_reason_ref TEXT NULL,
            evidence_ref TEXT NULL,
            resolver_ref TEXT NULL,
            judged_at_epoch_ms BIGINT NULL,
            policy_version_ref TEXT NULL,
            evidence_set_ref TEXT NULL,
            revision_evidence_ref TEXT NOT NULL,
            revision_policy_ref TEXT NOT NULL,
            created_at_epoch_ms BIGINT NOT NULL,
            creates_world_event BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_world_event),
            candidate_only BOOLEAN NOT NULL DEFAULT TRUE CHECK (candidate_only),
            creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
            creates_claim_truth BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_claim_truth)
        );

        CREATE INDEX IF NOT EXISTS resolution_revision_forecast_created_idx
            ON forecast.resolution_revision(forecast_ref, created_at_epoch_ms, resolution_revision_ref);

        CREATE TABLE IF NOT EXISTS forecast.evidence_availability (
            evidence_ref TEXT PRIMARY KEY,
            event_time_epoch_ms BIGINT NULL,
            created_at_epoch_ms BIGINT NULL,
            published_at_epoch_ms BIGINT NULL,
            discoverable_at_epoch_ms BIGINT NULL,
            acquired_at_epoch_ms BIGINT NOT NULL,
            available_at_epoch_ms BIGINT NULL,
            provenance_ref TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS forecast.cohort (
            cohort_ref TEXT PRIMARY KEY,
            target_population_ref TEXT NOT NULL,
            predicate_ref TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS forecast.cohort_member (
            cohort_ref TEXT NOT NULL REFERENCES forecast.cohort(cohort_ref),
            forecast_ref TEXT NOT NULL REFERENCES forecast.published_forecast(forecast_ref),
            included BOOLEAN NOT NULL,
            selection_reason_ref TEXT NOT NULL,
            PRIMARY KEY (cohort_ref, forecast_ref)
        );

        CREATE TABLE IF NOT EXISTS forecast.score_snapshot (
            score_ref TEXT PRIMARY KEY,
            cohort_ref TEXT NOT NULL REFERENCES forecast.cohort(cohort_ref),
            resolution_ledger_ref TEXT NOT NULL,
            brier_numerator BIGINT NOT NULL CHECK (brier_numerator >= 0),
            brier_denominator BIGINT NOT NULL CHECK (brier_denominator > 0),
            candidate_only BOOLEAN NOT NULL DEFAULT TRUE CHECK (candidate_only),
            creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
            creates_claim_truth BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_claim_truth),
            claims_causal_attribution BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claims_causal_attribution)
        );

        CREATE TABLE IF NOT EXISTS forecast.score_member (
            score_ref TEXT NOT NULL REFERENCES forecast.score_snapshot(score_ref),
            forecast_ref TEXT NOT NULL REFERENCES forecast.published_forecast(forecast_ref),
            PRIMARY KEY (score_ref, forecast_ref)
        );

        CREATE TABLE IF NOT EXISTS forecast.residual (
            residual_ref TEXT PRIMARY KEY,
            forecast_or_score_ref TEXT NOT NULL,
            consumer_ref TEXT NOT NULL,
            residual_kind_ref TEXT NOT NULL,
            required_coordinate_ref TEXT NOT NULL,
            open BOOLEAN NOT NULL,
            candidate_only BOOLEAN NOT NULL DEFAULT TRUE CHECK (candidate_only),
            creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
            creates_claim_truth BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_claim_truth)
        );

        CREATE TABLE IF NOT EXISTS forecast.residual_dependency (
            residual_ref TEXT NOT NULL REFERENCES forecast.residual(residual_ref),
            dependency_ref TEXT NOT NULL,
            PRIMARY KEY (residual_ref, dependency_ref)
        );

        CREATE TABLE IF NOT EXISTS forecast.research_demand (
            demand_ref TEXT PRIMARY KEY,
            residual_ref TEXT NOT NULL REFERENCES forecast.residual(residual_ref),
            consumer_ref TEXT NOT NULL,
            probe_kind_ref TEXT NOT NULL,
            expected_proposition_shape_ref TEXT NOT NULL,
            temporal_cut_ref TEXT NOT NULL,
            source_policy_ref TEXT NOT NULL,
            budget_ref TEXT NOT NULL,
            candidate_only BOOLEAN NOT NULL DEFAULT TRUE CHECK (candidate_only),
            pays_residual BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT pays_residual),
            creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
            creates_claim_truth BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_claim_truth)
        );
        "#,
    )?;
    Ok(())
}

pub fn persist_published_forecast(
    client: &mut Client,
    forecast: &PublishedBinaryForecast,
) -> Result<(), ForecastStoreError> {
    forecast.validate().map_err(ForecastStoreError::Invalid)?;
    let p = forecast.probability.rational();
    client.execute(
        r#"
        INSERT INTO forecast.published_forecast (
            forecast_ref, proposition_ref, issued_at_epoch_ms, horizon_ref,
            probability_numerator, probability_denominator, domain_ref, origin_ref,
            model_ref, source_state_ref, resolution_policy_ref,
            candidate_only, creates_semantic_authority, creates_claim_truth
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
        ON CONFLICT (forecast_ref) DO UPDATE SET
            proposition_ref = EXCLUDED.proposition_ref,
            issued_at_epoch_ms = EXCLUDED.issued_at_epoch_ms,
            horizon_ref = EXCLUDED.horizon_ref,
            probability_numerator = EXCLUDED.probability_numerator,
            probability_denominator = EXCLUDED.probability_denominator,
            domain_ref = EXCLUDED.domain_ref,
            origin_ref = EXCLUDED.origin_ref,
            model_ref = EXCLUDED.model_ref,
            source_state_ref = EXCLUDED.source_state_ref,
            resolution_policy_ref = EXCLUDED.resolution_policy_ref
        "#,
        &[
            &forecast.forecast_ref,
            &forecast.proposition_ref,
            &forecast.issued_at_epoch_ms,
            &forecast.horizon_ref,
            &as_i64(p.numerator, "probability numerator")?,
            &as_i64(p.denominator, "probability denominator")?,
            &domain_db(forecast.domain),
            &origin_db(forecast.origin),
            &forecast.model_ref,
            &forecast.source_state_ref,
            &forecast.resolution_policy_ref,
            &forecast.candidate_only,
            &forecast.creates_semantic_authority,
            &forecast.creates_claim_truth,
        ],
    )?;
    Ok(())
}

fn resolution_fields(
    state: &ForecastResolutionState,
) -> (
    &'static str,
    Option<&'static str>,
    Option<&str>,
    Option<&ResolutionEvidence>,
) {
    match state {
        ForecastResolutionState::Open => ("open", None, None, None),
        ForecastResolutionState::AwaitingJudge => ("awaiting_judge", None, None, None),
        ForecastResolutionState::Scored { outcome, evidence } => (
            "scored",
            Some(match outcome {
                BinaryOutcome::No => "no",
                BinaryOutcome::Yes => "yes",
            }),
            None,
            Some(evidence),
        ),
        ForecastResolutionState::Voided { reason_ref, evidence } => {
            ("voided", None, Some(reason_ref.as_str()), Some(evidence))
        }
    }
}

pub fn persist_resolution_revision(
    client: &mut Client,
    revision: &ForecastResolutionRevision,
) -> Result<(), ForecastStoreError> {
    revision.validate().map_err(ForecastStoreError::Invalid)?;
    let (state_kind, outcome, void_reason, evidence) = resolution_fields(&revision.state);
    let evidence_ref = evidence.map(|v| v.evidence_ref.as_str());
    let resolver_ref = evidence.map(|v| v.resolver_ref.as_str());
    let judged_at = evidence.map(|v| v.judged_at_epoch_ms);
    let policy_version = evidence.map(|v| v.policy_version_ref.as_str());
    let evidence_set = evidence.map(|v| v.evidence_set_ref.as_str());

    client.execute(
        r#"
        INSERT INTO forecast.resolution_revision (
            resolution_revision_ref, forecast_ref, previous_revision_ref,
            state_kind, outcome_ref, void_reason_ref,
            evidence_ref, resolver_ref, judged_at_epoch_ms,
            policy_version_ref, evidence_set_ref,
            revision_evidence_ref, revision_policy_ref, created_at_epoch_ms,
            creates_world_event, candidate_only, creates_semantic_authority,
            creates_claim_truth
        ) VALUES (
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18
        )
        ON CONFLICT (resolution_revision_ref) DO NOTHING
        "#,
        &[
            &revision.resolution_revision_ref,
            &revision.forecast_ref,
            &revision.previous_revision_ref,
            &state_kind,
            &outcome,
            &void_reason,
            &evidence_ref,
            &resolver_ref,
            &judged_at,
            &policy_version,
            &evidence_set,
            &revision.revision_evidence_ref,
            &revision.revision_policy_ref,
            &revision.created_at_epoch_ms,
            &revision.creates_world_event,
            &revision.candidate_only,
            &revision.creates_semantic_authority,
            &revision.creates_claim_truth,
        ],
    )?;
    Ok(())
}

pub fn persist_evidence_availability(
    client: &mut Client,
    evidence: &ForecastEvidenceAvailability,
) -> Result<(), ForecastStoreError> {
    if evidence.evidence_ref.trim().is_empty() || evidence.provenance_ref.trim().is_empty() {
        return Err(ForecastStoreError::Invalid(
            "evidence availability requires evidence and provenance refs".into(),
        ));
    }
    client.execute(
        r#"
        INSERT INTO forecast.evidence_availability (
            evidence_ref, event_time_epoch_ms, created_at_epoch_ms,
            published_at_epoch_ms, discoverable_at_epoch_ms, acquired_at_epoch_ms,
            available_at_epoch_ms, provenance_ref
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
        ON CONFLICT (evidence_ref) DO UPDATE SET
            event_time_epoch_ms = EXCLUDED.event_time_epoch_ms,
            created_at_epoch_ms = EXCLUDED.created_at_epoch_ms,
            published_at_epoch_ms = EXCLUDED.published_at_epoch_ms,
            discoverable_at_epoch_ms = EXCLUDED.discoverable_at_epoch_ms,
            acquired_at_epoch_ms = EXCLUDED.acquired_at_epoch_ms,
            available_at_epoch_ms = EXCLUDED.available_at_epoch_ms,
            provenance_ref = EXCLUDED.provenance_ref
        "#,
        &[
            &evidence.evidence_ref,
            &evidence.event_time_epoch_ms,
            &evidence.created_at_epoch_ms,
            &evidence.published_at_epoch_ms,
            &evidence.discoverable_at_epoch_ms,
            &evidence.acquired_at_epoch_ms,
            &evidence.available_at_epoch_ms,
            &evidence.provenance_ref,
        ],
    )?;
    Ok(())
}

pub fn persist_forecast_cohort(
    client: &mut Client,
    cohort: &ForecastCohort,
) -> Result<(), ForecastStoreError> {
    cohort.validate().map_err(ForecastStoreError::Invalid)?;
    client.execute(
        r#"
        INSERT INTO forecast.cohort (
            cohort_ref, target_population_ref, predicate_ref
        ) VALUES ($1,$2,$3)
        ON CONFLICT (cohort_ref) DO UPDATE SET
            target_population_ref = EXCLUDED.target_population_ref,
            predicate_ref = EXCLUDED.predicate_ref
        "#,
        &[&cohort.cohort_ref, &cohort.target_population_ref, &cohort.predicate_ref],
    )?;
    Ok(())
}

pub fn persist_forecast_cohort_member(
    client: &mut Client,
    cohort_ref: &str,
    forecast_ref: &str,
    included: bool,
    selection_reason_ref: &str,
) -> Result<(), ForecastStoreError> {
    if cohort_ref.trim().is_empty()
        || forecast_ref.trim().is_empty()
        || selection_reason_ref.trim().is_empty()
    {
        return Err(ForecastStoreError::Invalid(
            "cohort membership requires identity and reason refs".into(),
        ));
    }
    client.execute(
        r#"
        INSERT INTO forecast.cohort_member (
            cohort_ref, forecast_ref, included, selection_reason_ref
        ) VALUES ($1,$2,$3,$4)
        ON CONFLICT (cohort_ref, forecast_ref) DO UPDATE SET
            included = EXCLUDED.included,
            selection_reason_ref = EXCLUDED.selection_reason_ref
        "#,
        &[&cohort_ref, &forecast_ref, &included, &selection_reason_ref],
    )?;
    Ok(())
}

pub fn persist_forecast_score_projection(
    client: &mut Client,
    score: &ForecastScoreProjection,
) -> Result<(), ForecastStoreError> {
    if !score.candidate_only
        || score.creates_semantic_authority
        || score.creates_claim_truth
        || score.claims_causal_attribution
    {
        return Err(ForecastStoreError::Invalid(
            "forecast score crossed non-promotion/causal boundary".into(),
        ));
    }
    let numerator = as_i64(score.brier_mean.numerator, "Brier numerator")?;
    let denominator = as_i64(score.brier_mean.denominator, "Brier denominator")?;
    let mut tx = client.transaction()?;
    tx.execute(
        r#"
        INSERT INTO forecast.score_snapshot (
            score_ref, cohort_ref, resolution_ledger_ref,
            brier_numerator, brier_denominator, candidate_only,
            creates_semantic_authority, creates_claim_truth,
            claims_causal_attribution
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
        ON CONFLICT (score_ref) DO UPDATE SET
            cohort_ref = EXCLUDED.cohort_ref,
            resolution_ledger_ref = EXCLUDED.resolution_ledger_ref,
            brier_numerator = EXCLUDED.brier_numerator,
            brier_denominator = EXCLUDED.brier_denominator
        "#,
        &[
            &score.score_ref,
            &score.cohort_ref,
            &score.resolution_ledger_ref,
            &numerator,
            &denominator,
            &score.candidate_only,
            &score.creates_semantic_authority,
            &score.creates_claim_truth,
            &score.claims_causal_attribution,
        ],
    )?;
    tx.execute("DELETE FROM forecast.score_member WHERE score_ref = $1", &[&score.score_ref])?;
    for forecast_ref in &score.scored_forecast_refs {
        tx.execute(
            "INSERT INTO forecast.score_member (score_ref, forecast_ref) VALUES ($1,$2)",
            &[&score.score_ref, forecast_ref],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn load_published_forecast(
    client: &mut Client,
    forecast_ref: &str,
) -> Result<PublishedBinaryForecast, ForecastStoreError> {
    let row = client.query_one(
        r#"
        SELECT proposition_ref, issued_at_epoch_ms, horizon_ref,
               probability_numerator, probability_denominator,
               domain_ref, origin_ref, model_ref, source_state_ref,
               resolution_policy_ref, candidate_only,
               creates_semantic_authority, creates_claim_truth
        FROM forecast.published_forecast
        WHERE forecast_ref = $1
        "#,
        &[&forecast_ref],
    )?;
    let numerator: i64 = row.get(3);
    let denominator: i64 = row.get(4);
    let probability = Probability::new(
        as_u64(numerator, "probability numerator")?,
        as_u64(denominator, "probability denominator")?,
    )
    .map_err(ForecastStoreError::Invalid)?;
    let domain_ref: String = row.get(5);
    let origin_ref: String = row.get(6);

    Ok(PublishedBinaryForecast {
        forecast_ref: forecast_ref.into(),
        proposition_ref: row.get(0),
        issued_at_epoch_ms: row.get(1),
        horizon_ref: row.get(2),
        probability,
        domain: parse_domain(&domain_ref)?,
        origin: parse_origin(&origin_ref)?,
        model_ref: row.get(7),
        source_state_ref: row.get(8),
        resolution_policy_ref: row.get(9),
        candidate_only: row.get(10),
        creates_semantic_authority: row.get(11),
        creates_claim_truth: row.get(12),
    })
}

pub fn load_latest_scorable_forecast(
    client: &mut Client,
    forecast_ref: &str,
) -> Result<Option<ScorableForecast>, ForecastStoreError> {
    let rows = client.query(
        r#"
        SELECT resolution_revision_ref, outcome_ref, evidence_ref
        FROM forecast.resolution_revision
        WHERE forecast_ref = $1 AND state_kind = 'scored'
        ORDER BY created_at_epoch_ms DESC, resolution_revision_ref DESC
        LIMIT 1
        "#,
        &[&forecast_ref],
    )?;
    let Some(row) = rows.first() else {
        return Ok(None);
    };
    let outcome_ref: String = row.get(1);
    let outcome = match outcome_ref.as_str() {
        "no" => BinaryOutcome::No,
        "yes" => BinaryOutcome::Yes,
        other => {
            return Err(ForecastStoreError::Invalid(format!(
                "unknown binary outcome {other}"
            )))
        }
    };
    let evidence_ref: Option<String> = row.get(2);
    let evidence_ref = evidence_ref.ok_or_else(|| {
        ForecastStoreError::Invalid("scored resolution lacks evidence ref".into())
    })?;

    Ok(Some(ScorableForecast {
        forecast: load_published_forecast(client, forecast_ref)?,
        resolution_revision_ref: row.get(0),
        outcome,
        resolution_evidence_ref: evidence_ref,
        scoring_eligibility_ref: format!("eligibility:resolved-scored:{forecast_ref}"),
    }))
}

pub fn load_scorable_forecasts_for_cohort(
    client: &mut Client,
    cohort_ref: &str,
) -> Result<Vec<ScorableForecast>, ForecastStoreError> {
    let rows = client.query(
        r#"
        SELECT forecast_ref
        FROM forecast.cohort_member
        WHERE cohort_ref = $1 AND included = TRUE
        ORDER BY forecast_ref
        "#,
        &[&cohort_ref],
    )?;
    let mut out = Vec::new();
    for row in rows {
        let forecast_ref: String = row.get(0);
        if let Some(scorable) = load_latest_scorable_forecast(client, &forecast_ref)? {
            out.push(scorable);
        }
    }
    Ok(out)
}

pub fn load_forecast_score_projection(
    client: &mut Client,
    score_ref: &str,
) -> Result<ForecastScoreProjection, ForecastStoreError> {
    let row = client.query_one(
        r#"
        SELECT cohort_ref, resolution_ledger_ref, brier_numerator,
               brier_denominator, candidate_only,
               creates_semantic_authority, creates_claim_truth,
               claims_causal_attribution
        FROM forecast.score_snapshot
        WHERE score_ref = $1
        "#,
        &[&score_ref],
    )?;
    let member_rows = client.query(
        "SELECT forecast_ref FROM forecast.score_member WHERE score_ref = $1 ORDER BY forecast_ref",
        &[&score_ref],
    )?;
    let numerator: i64 = row.get(2);
    let denominator: i64 = row.get(3);
    Ok(ForecastScoreProjection {
        score_ref: score_ref.into(),
        cohort_ref: row.get(0),
        resolution_ledger_ref: row.get(1),
        scored_forecast_refs: member_rows.into_iter().map(|r| r.get(0)).collect(),
        brier_mean: Rational::new(
            as_u64(numerator, "Brier numerator")?,
            as_u64(denominator, "Brier denominator")?,
        )
        .map_err(ForecastStoreError::Invalid)?,
        candidate_only: row.get(4),
        creates_semantic_authority: row.get(5),
        creates_claim_truth: row.get(6),
        claims_causal_attribution: row.get(7),
    })
}


fn residual_kind_db(value: ForecastResidualKind) -> &'static str {
    match value {
        ForecastResidualKind::SourceProvenance => "source_provenance",
        ForecastResidualKind::ForecastTimeAvailability => "forecast_time_availability",
        ForecastResidualKind::Mechanism => "mechanism",
        ForecastResidualKind::Regime => "regime",
        ForecastResidualKind::Calibration => "calibration",
        ForecastResidualKind::ScorabilitySelection => "scorability_selection",
        ForecastResidualKind::ResolutionObserver => "resolution_observer",
        ForecastResidualKind::Comparator => "comparator",
        ForecastResidualKind::ObjectDecomposition => "object_decomposition",
    }
}

fn probe_kind_db(value: ForecastResearchProbeKind) -> &'static str {
    match value {
        ForecastResearchProbeKind::Support => "support",
        ForecastResearchProbeKind::Defeater => "defeater",
        ForecastResearchProbeKind::Comparator => "comparator",
        ForecastResearchProbeKind::Contradiction => "contradiction",
        ForecastResearchProbeKind::Counterexample => "counterexample",
        ForecastResearchProbeKind::VocabularyExploration => "vocabulary_exploration",
        ForecastResearchProbeKind::AuthorityFamilyExploration => "authority_family_exploration",
    }
}

pub fn persist_forecast_residual(
    client: &mut Client,
    residual: &ForecastResidualProjection,
) -> Result<(), ForecastStoreError> {
    if residual.residual_ref.trim().is_empty()
        || residual.forecast_or_score_ref.trim().is_empty()
        || residual.consumer_ref.trim().is_empty()
        || residual.required_coordinate_ref.trim().is_empty()
    {
        return Err(ForecastStoreError::Invalid(
            "forecast residual requires identity and coordinate refs".into(),
        ));
    }
    if !residual.candidate_only
        || residual.creates_semantic_authority
        || residual.creates_claim_truth
    {
        return Err(ForecastStoreError::Invalid(
            "forecast residual crossed non-promotion boundary".into(),
        ));
    }

    let mut tx = client.transaction()?;
    tx.execute(
        r#"
        INSERT INTO forecast.residual (
            residual_ref, forecast_or_score_ref, consumer_ref,
            residual_kind_ref, required_coordinate_ref, open,
            candidate_only, creates_semantic_authority, creates_claim_truth
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
        ON CONFLICT (residual_ref) DO UPDATE SET
            forecast_or_score_ref = EXCLUDED.forecast_or_score_ref,
            consumer_ref = EXCLUDED.consumer_ref,
            residual_kind_ref = EXCLUDED.residual_kind_ref,
            required_coordinate_ref = EXCLUDED.required_coordinate_ref,
            open = EXCLUDED.open
        "#,
        &[
            &residual.residual_ref,
            &residual.forecast_or_score_ref,
            &residual.consumer_ref,
            &residual_kind_db(residual.kind),
            &residual.required_coordinate_ref,
            &residual.open,
            &residual.candidate_only,
            &residual.creates_semantic_authority,
            &residual.creates_claim_truth,
        ],
    )?;
    tx.execute(
        "DELETE FROM forecast.residual_dependency WHERE residual_ref = $1",
        &[&residual.residual_ref],
    )?;
    for dependency_ref in &residual.dependency_refs {
        tx.execute(
            "INSERT INTO forecast.residual_dependency (residual_ref, dependency_ref) VALUES ($1,$2)",
            &[&residual.residual_ref, dependency_ref],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn persist_forecast_research_demand(
    client: &mut Client,
    demand: &ForecastResearchDemand,
) -> Result<(), ForecastStoreError> {
    if demand.demand_ref.trim().is_empty()
        || demand.residual_ref.trim().is_empty()
        || demand.consumer_ref.trim().is_empty()
        || demand.expected_proposition_shape_ref.trim().is_empty()
        || demand.temporal_cut_ref.trim().is_empty()
        || demand.source_policy_ref.trim().is_empty()
        || demand.budget_ref.trim().is_empty()
    {
        return Err(ForecastStoreError::Invalid(
            "forecast research demand requires identity refs".into(),
        ));
    }
    if !demand.candidate_only
        || demand.pays_residual
        || demand.creates_semantic_authority
        || demand.creates_claim_truth
    {
        return Err(ForecastStoreError::Invalid(
            "research demand cannot pay residual or promote truth/authority".into(),
        ));
    }

    client.execute(
        r#"
        INSERT INTO forecast.research_demand (
            demand_ref, residual_ref, consumer_ref, probe_kind_ref,
            expected_proposition_shape_ref, temporal_cut_ref,
            source_policy_ref, budget_ref, candidate_only, pays_residual,
            creates_semantic_authority, creates_claim_truth
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
        ON CONFLICT (demand_ref) DO UPDATE SET
            residual_ref = EXCLUDED.residual_ref,
            consumer_ref = EXCLUDED.consumer_ref,
            probe_kind_ref = EXCLUDED.probe_kind_ref,
            expected_proposition_shape_ref = EXCLUDED.expected_proposition_shape_ref,
            temporal_cut_ref = EXCLUDED.temporal_cut_ref,
            source_policy_ref = EXCLUDED.source_policy_ref,
            budget_ref = EXCLUDED.budget_ref
        "#,
        &[
            &demand.demand_ref,
            &demand.residual_ref,
            &demand.consumer_ref,
            &probe_kind_db(demand.probe_kind),
            &demand.expected_proposition_shape_ref,
            &demand.temporal_cut_ref,
            &demand.source_policy_ref,
            &demand.budget_ref,
            &demand.candidate_only,
            &demand.pays_residual,
            &demand.creates_semantic_authority,
            &demand.creates_claim_truth,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_storage_roundtrips() {
        for domain in [
            ForecastDomain::Conflict,
            ForecastDomain::Cyber,
            ForecastDomain::Energy,
            ForecastDomain::Geopolitical,
            ForecastDomain::Infrastructure,
            ForecastDomain::Macro,
            ForecastDomain::Market,
            ForecastDomain::Military,
            ForecastDomain::Political,
            ForecastDomain::SupplyChain,
            ForecastDomain::Other,
        ] {
            assert_eq!(parse_domain(domain_db(domain)).unwrap(), domain);
        }
        for origin in [
            ForecastOrigin::BetEngine,
            ForecastOrigin::LegacyDetector,
            ForecastOrigin::StateDerived,
            ForecastOrigin::Unknown,
            ForecastOrigin::Other,
        ] {
            assert_eq!(parse_origin(origin_db(origin)).unwrap(), origin);
        }
    }

    #[test]
    fn integer_conversion_rejects_negative_or_too_large() {
        assert!(as_u64(-1, "x").is_err());
        assert!(as_i64(u64::MAX, "x").is_err());
    }
}
