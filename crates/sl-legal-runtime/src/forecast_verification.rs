//! Production forecast-verification computation.
//!
//! Executable mirror of the golden forecast kernel in dashi_agda PR #1031.
//! Keeps scoring, forecast-time admissibility, resolution identity and research
//! residuals distinct. All probability/loss arithmetic is exact rational
//! arithmetic over the reader-model carrier.

use sensiblaw_reader_model::{
    BinaryOutcome, ForecastCohort, ForecastResearchDemand, ForecastResearchProbeKind,
    ForecastResidualKind, ForecastResidualProjection, ForecastScoreProjection,
    Probability, Rational, ScorableForecast,
};

pub const DASHI_FORECAST_GOLDEN_PR: &str = "chboishabba/dashi_agda#1031";
pub const DASHI_FORECAST_GOLDEN_HEAD: &str =
    "1167e6199cc9929d876ebdf92ae786bc11a786a0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastRowScore {
    pub forecast_ref: String,
    pub resolution_revision_ref: String,
    pub brier_loss: Rational,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastCohortScoreReceipt {
    pub projection: ForecastScoreProjection,
    pub row_scores: Vec<ForecastRowScore>,
    pub cohort_predicate_ref: String,
    pub target_population_ref: String,
    pub all_rows_scorable: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
    pub claims_causal_attribution: bool,
}

pub fn brier_loss(
    probability: Probability,
    outcome: BinaryOutcome,
) -> Result<Rational, String> {
    let p = probability.rational();
    let error_numerator = match outcome {
        BinaryOutcome::No => p.numerator,
        BinaryOutcome::Yes => p
            .denominator
            .checked_sub(p.numerator)
            .ok_or_else(|| "probability exceeded one".to_string())?,
    };
    let numerator = (error_numerator as u128)
        .checked_mul(error_numerator as u128)
        .ok_or_else(|| "brier numerator overflow".to_string())?;
    let denominator = (p.denominator as u128)
        .checked_mul(p.denominator as u128)
        .ok_or_else(|| "brier denominator overflow".to_string())?;
    if numerator > u64::MAX as u128 || denominator > u64::MAX as u128 {
        return Err("brier rational overflow".into());
    }
    Rational::new(numerator as u64, denominator as u64)
}

pub fn score_forecast_row(row: &ScorableForecast) -> Result<ForecastRowScore, String> {
    row.validate()?;
    Ok(ForecastRowScore {
        forecast_ref: row.forecast.forecast_ref.clone(),
        resolution_revision_ref: row.resolution_revision_ref.clone(),
        brier_loss: brier_loss(row.forecast.probability, row.outcome)?,
    })
}

pub fn score_forecast_cohort(
    score_ref: &str,
    resolution_ledger_ref: &str,
    cohort: &ForecastCohort,
    rows: &[ScorableForecast],
) -> Result<ForecastCohortScoreReceipt, String> {
    if score_ref.trim().is_empty() || resolution_ledger_ref.trim().is_empty() {
        return Err("forecast score requires score and ledger refs".into());
    }
    cohort.validate()?;
    if rows.is_empty() {
        return Err("cannot score empty forecast cohort".into());
    }

    let row_scores = rows
        .iter()
        .map(score_forecast_row)
        .collect::<Result<Vec<_>, _>>()?;

    let total = row_scores
        .iter()
        .try_fold(Rational::zero(), |acc, row| acc.checked_add(row.brier_loss))?;
    let mean = total.checked_div_u64(row_scores.len() as u64)?;

    let projection = ForecastScoreProjection {
        score_ref: score_ref.into(),
        cohort_ref: cohort.cohort_ref.clone(),
        resolution_ledger_ref: resolution_ledger_ref.into(),
        scored_forecast_refs: row_scores
            .iter()
            .map(|row| row.forecast_ref.clone())
            .collect(),
        brier_mean: mean,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
        claims_causal_attribution: false,
    };

    Ok(ForecastCohortScoreReceipt {
        projection,
        row_scores,
        cohort_predicate_ref: cohort.predicate_ref.clone(),
        target_population_ref: cohort.target_population_ref.clone(),
        all_rows_scorable: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
        claims_causal_attribution: false,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForecastScoreControl {
    Raw,
    SameCohortRule,
    SameResolvedIntersection,
    SameOriginMixture,
    SameDomainMixture,
    SameResolutionObserver,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledForecastScoreReceipt {
    pub control: ForecastScoreControl,
    pub baseline_score_ref: String,
    pub comparison_score_ref: String,
    pub baseline_score: Rational,
    pub comparison_score: Rational,
    pub comparison_lower: bool,
    pub equal_score: bool,
    pub claims_causal_attribution: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn rational_cmp(left: Rational, right: Rational) -> Result<std::cmp::Ordering, String> {
    let l = (left.numerator as u128)
        .checked_mul(right.denominator as u128)
        .ok_or_else(|| "score comparison overflow".to_string())?;
    let r = (right.numerator as u128)
        .checked_mul(left.denominator as u128)
        .ok_or_else(|| "score comparison overflow".to_string())?;
    Ok(l.cmp(&r))
}

pub fn compare_controlled_forecast_scores(
    control: ForecastScoreControl,
    baseline: &ForecastScoreProjection,
    comparison: &ForecastScoreProjection,
) -> Result<ControlledForecastScoreReceipt, String> {
    let ordering = rational_cmp(comparison.brier_mean, baseline.brier_mean)?;
    Ok(ControlledForecastScoreReceipt {
        control,
        baseline_score_ref: baseline.score_ref.clone(),
        comparison_score_ref: comparison.score_ref.clone(),
        baseline_score: baseline.brier_mean,
        comparison_score: comparison.brier_mean,
        comparison_lower: ordering == std::cmp::Ordering::Less,
        equal_score: ordering == std::cmp::Ordering::Equal,
        claims_causal_attribution: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

pub fn compile_forecast_research_demand(
    residual: &ForecastResidualProjection,
    demand_ref: &str,
    temporal_cut_ref: &str,
    source_policy_ref: &str,
    budget_ref: &str,
) -> Result<ForecastResearchDemand, String> {
    if !residual.open {
        return Err("closed forecast residual does not generate research demand".into());
    }
    if residual.residual_ref.trim().is_empty()
        || residual.consumer_ref.trim().is_empty()
        || residual.required_coordinate_ref.trim().is_empty()
        || demand_ref.trim().is_empty()
        || temporal_cut_ref.trim().is_empty()
        || source_policy_ref.trim().is_empty()
        || budget_ref.trim().is_empty()
    {
        return Err("forecast research demand requires typed identity refs".into());
    }

    let probe_kind = match residual.kind {
        ForecastResidualKind::SourceProvenance => {
            ForecastResearchProbeKind::AuthorityFamilyExploration
        }
        ForecastResidualKind::ForecastTimeAvailability => {
            ForecastResearchProbeKind::Comparator
        }
        ForecastResidualKind::Mechanism => ForecastResearchProbeKind::Defeater,
        ForecastResidualKind::Regime => ForecastResearchProbeKind::Comparator,
        ForecastResidualKind::Calibration => ForecastResearchProbeKind::Comparator,
        ForecastResidualKind::ScorabilitySelection => {
            ForecastResearchProbeKind::Contradiction
        }
        ForecastResidualKind::ResolutionObserver => {
            ForecastResearchProbeKind::Contradiction
        }
        ForecastResidualKind::Comparator => ForecastResearchProbeKind::Comparator,
        ForecastResidualKind::ObjectDecomposition => {
            ForecastResearchProbeKind::VocabularyExploration
        }
    };

    Ok(ForecastResearchDemand {
        demand_ref: demand_ref.into(),
        residual_ref: residual.residual_ref.clone(),
        consumer_ref: residual.consumer_ref.clone(),
        probe_kind,
        expected_proposition_shape_ref: residual.required_coordinate_ref.clone(),
        temporal_cut_ref: temporal_cut_ref.into(),
        source_policy_ref: source_policy_ref.into(),
        budget_ref: budget_ref.into(),
        candidate_only: true,
        pays_residual: false,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_reader_model::{
        ForecastDomain, ForecastOrigin, PublishedBinaryForecast,
    };

    fn forecast(reference: &str, probability: Probability) -> PublishedBinaryForecast {
        PublishedBinaryForecast {
            forecast_ref: reference.into(),
            proposition_ref: format!("prop:{reference}"),
            issued_at_epoch_ms: 10,
            horizon_ref: "30d".into(),
            probability,
            domain: ForecastDomain::Geopolitical,
            origin: ForecastOrigin::LegacyDetector,
            model_ref: "model:1".into(),
            source_state_ref: "source-state:1".into(),
            resolution_policy_ref: "resolution-policy:1".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn row(reference: &str, p: (u64, u64), outcome: BinaryOutcome) -> ScorableForecast {
        ScorableForecast {
            forecast: forecast(reference, Probability::new(p.0, p.1).unwrap()),
            resolution_revision_ref: format!("resolution:{reference}:r1"),
            outcome,
            resolution_evidence_ref: format!("evidence:{reference}"),
            scoring_eligibility_ref: format!("eligibility:{reference}"),
        }
    }

    #[test]
    fn exact_brier_matches_golden_half_baseline() {
        assert_eq!(
            brier_loss(Probability::new(1, 2).unwrap(), BinaryOutcome::No).unwrap(),
            Rational::new(1, 4).unwrap()
        );
        assert_eq!(
            brier_loss(Probability::new(1, 2).unwrap(), BinaryOutcome::Yes).unwrap(),
            Rational::new(1, 4).unwrap()
        );
    }

    #[test]
    fn exact_brier_hits_zero_and_one_boundaries() {
        assert_eq!(
            brier_loss(Probability::new(0, 1).unwrap(), BinaryOutcome::No).unwrap(),
            Rational::zero()
        );
        assert_eq!(
            brier_loss(Probability::new(0, 1).unwrap(), BinaryOutcome::Yes).unwrap(),
            Rational::one()
        );
    }

    #[test]
    fn finite_cohort_mean_is_derived_from_rows() {
        let cohort = ForecastCohort {
            cohort_ref: "cohort:headline".into(),
            target_population_ref: "population:issued".into(),
            predicate_ref: "origin:not-bet-engine-or-state-derived".into(),
        };
        let rows = vec![
            row("a", (1, 2), BinaryOutcome::Yes),
            row("b", (1, 2), BinaryOutcome::No),
        ];
        let receipt =
            score_forecast_cohort("score:1", "ledger:r1", &cohort, &rows).unwrap();

        assert_eq!(receipt.projection.brier_mean, Rational::new(1, 4).unwrap());
        assert_eq!(receipt.row_scores.len(), 2);
        assert!(!receipt.claims_causal_attribution);
        assert!(!receipt.creates_claim_truth);
    }

    #[test]
    fn residual_compiles_to_typed_nonpaying_research_demand() {
        let residual = ForecastResidualProjection {
            residual_ref: "residual:mechanism".into(),
            forecast_or_score_ref: "score:1".into(),
            consumer_ref: "consumer:forecast-explanation".into(),
            kind: ForecastResidualKind::Mechanism,
            required_coordinate_ref: "coordinate:failure-mechanism".into(),
            dependency_refs: vec!["forecast:1".into()],
            open: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let demand = compile_forecast_research_demand(
            &residual,
            "demand:mechanism",
            "cut:forecast-time",
            "source-policy:primary-first",
            "budget:bounded",
        )
        .unwrap();

        assert_eq!(demand.probe_kind, ForecastResearchProbeKind::Defeater);
        assert!(!demand.pays_residual);
        assert!(!demand.creates_semantic_authority);
        assert!(!demand.creates_claim_truth);
    }
}
