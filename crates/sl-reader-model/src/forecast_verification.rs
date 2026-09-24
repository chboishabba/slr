//! Production forecast-verification reader carriers.
//!
//! Mirrors the golden semantics in dashi_agda PR #1031 without importing
//! application-specific WorldMonitor assumptions. The normal production path
//! is normalized Postgres -> these typed carriers -> runtime computation.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.max(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rational {
    pub numerator: u64,
    pub denominator: u64,
}

impl Rational {
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, String> {
        if denominator == 0 {
            return Err("rational denominator must be non-zero".into());
        }
        let d = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / d,
            denominator: denominator / d,
        })
    }

    pub fn zero() -> Self {
        Self { numerator: 0, denominator: 1 }
    }

    pub fn one() -> Self {
        Self { numerator: 1, denominator: 1 }
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, String> {
        let left = (self.numerator as u128)
            .checked_mul(rhs.denominator as u128)
            .ok_or_else(|| "rational addition overflow".to_string())?;
        let right = (rhs.numerator as u128)
            .checked_mul(self.denominator as u128)
            .ok_or_else(|| "rational addition overflow".to_string())?;
        let numerator = left
            .checked_add(right)
            .ok_or_else(|| "rational addition overflow".to_string())?;
        let denominator = (self.denominator as u128)
            .checked_mul(rhs.denominator as u128)
            .ok_or_else(|| "rational addition overflow".to_string())?;
        if numerator > u64::MAX as u128 || denominator > u64::MAX as u128 {
            return Err("rational addition overflow".into());
        }
        Self::new(numerator as u64, denominator as u64)
    }

    pub fn checked_mul(self, rhs: Self) -> Result<Self, String> {
        let numerator = (self.numerator as u128)
            .checked_mul(rhs.numerator as u128)
            .ok_or_else(|| "rational multiplication overflow".to_string())?;
        let denominator = (self.denominator as u128)
            .checked_mul(rhs.denominator as u128)
            .ok_or_else(|| "rational multiplication overflow".to_string())?;
        if numerator > u64::MAX as u128 || denominator > u64::MAX as u128 {
            return Err("rational multiplication overflow".into());
        }
        Self::new(numerator as u64, denominator as u64)
    }

    pub fn checked_div_u64(self, rhs: u64) -> Result<Self, String> {
        if rhs == 0 {
            return Err("cannot divide rational by zero".into());
        }
        let denominator = (self.denominator as u128)
            .checked_mul(rhs as u128)
            .ok_or_else(|| "rational division overflow".to_string())?;
        if denominator > u64::MAX as u128 {
            return Err("rational division overflow".into());
        }
        Self::new(self.numerator, denominator as u64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Probability(pub Rational);

impl Probability {
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, String> {
        let value = Rational::new(numerator, denominator)?;
        if value.numerator > value.denominator {
            return Err("probability must lie in [0,1]".into());
        }
        Ok(Self(value))
    }

    pub fn rational(self) -> Rational {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BinaryOutcome {
    No,
    Yes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ForecastDomain {
    Conflict,
    Cyber,
    Energy,
    Geopolitical,
    Infrastructure,
    Macro,
    Market,
    Military,
    Political,
    SupplyChain,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ForecastOrigin {
    BetEngine,
    LegacyDetector,
    StateDerived,
    Unknown,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PublishedBinaryForecast {
    pub forecast_ref: String,
    pub proposition_ref: String,
    pub issued_at_epoch_ms: i64,
    pub horizon_ref: String,
    pub probability: Probability,
    pub domain: ForecastDomain,
    pub origin: ForecastOrigin,
    pub model_ref: String,
    pub source_state_ref: String,
    pub resolution_policy_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl PublishedBinaryForecast {
    pub fn validate(&self) -> Result<(), String> {
        for (label, value) in [
            ("forecast_ref", self.forecast_ref.as_str()),
            ("proposition_ref", self.proposition_ref.as_str()),
            ("horizon_ref", self.horizon_ref.as_str()),
            ("model_ref", self.model_ref.as_str()),
            ("source_state_ref", self.source_state_ref.as_str()),
            ("resolution_policy_ref", self.resolution_policy_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("{label} must be non-empty"));
            }
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("published forecast crossed non-promotion boundary".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResolutionEvidence {
    pub evidence_ref: String,
    pub resolver_ref: String,
    pub judged_at_epoch_ms: i64,
    pub policy_version_ref: String,
    pub evidence_set_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ForecastResolutionState {
    Open,
    AwaitingJudge,
    Scored {
        outcome: BinaryOutcome,
        evidence: ResolutionEvidence,
    },
    Voided {
        reason_ref: String,
        evidence: ResolutionEvidence,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ForecastResolutionRevision {
    pub resolution_revision_ref: String,
    pub forecast_ref: String,
    pub previous_revision_ref: Option<String>,
    pub state: ForecastResolutionState,
    pub revision_evidence_ref: String,
    pub revision_policy_ref: String,
    pub created_at_epoch_ms: i64,
    pub creates_world_event: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ForecastResolutionRevision {
    pub fn validate(&self) -> Result<(), String> {
        if self.resolution_revision_ref.trim().is_empty()
            || self.forecast_ref.trim().is_empty()
            || self.revision_evidence_ref.trim().is_empty()
            || self.revision_policy_ref.trim().is_empty()
        {
            return Err("resolution revision requires identity/provenance refs".into());
        }
        if self.creates_world_event
            || !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("resolution revision crossed observer/non-promotion boundary".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ScorableForecast {
    pub forecast: PublishedBinaryForecast,
    pub resolution_revision_ref: String,
    pub outcome: BinaryOutcome,
    pub resolution_evidence_ref: String,
    pub scoring_eligibility_ref: String,
}

impl ScorableForecast {
    pub fn validate(&self) -> Result<(), String> {
        self.forecast.validate()?;
        if self.resolution_revision_ref.trim().is_empty()
            || self.resolution_evidence_ref.trim().is_empty()
            || self.scoring_eligibility_ref.trim().is_empty()
        {
            return Err("scorable forecast requires resolution and eligibility refs".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ForecastCohort {
    pub cohort_ref: String,
    pub target_population_ref: String,
    pub predicate_ref: String,
}

impl ForecastCohort {
    pub fn validate(&self) -> Result<(), String> {
        if self.cohort_ref.trim().is_empty()
            || self.target_population_ref.trim().is_empty()
            || self.predicate_ref.trim().is_empty()
        {
            return Err("forecast cohort requires explicit identity and predicate".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ForecastEvidenceAvailability {
    pub evidence_ref: String,
    pub event_time_epoch_ms: Option<i64>,
    pub created_at_epoch_ms: Option<i64>,
    pub published_at_epoch_ms: Option<i64>,
    pub discoverable_at_epoch_ms: Option<i64>,
    pub acquired_at_epoch_ms: i64,
    pub available_at_epoch_ms: Option<i64>,
    pub provenance_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ForecastTimeAdmissibility {
    AdmissibleAtForecast,
    HindsightOnly,
    AvailabilityUnresolved,
}

pub fn classify_forecast_time_admissibility(
    forecast: &PublishedBinaryForecast,
    evidence: &ForecastEvidenceAvailability,
) -> ForecastTimeAdmissibility {
    match evidence.available_at_epoch_ms {
        Some(available_at) if available_at <= forecast.issued_at_epoch_ms => {
            ForecastTimeAdmissibility::AdmissibleAtForecast
        }
        Some(_) => ForecastTimeAdmissibility::HindsightOnly,
        None => ForecastTimeAdmissibility::AvailabilityUnresolved,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ForecastScoreProjection {
    pub score_ref: String,
    pub cohort_ref: String,
    pub resolution_ledger_ref: String,
    pub scored_forecast_refs: Vec<String>,
    pub brier_mean: Rational,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
    pub claims_causal_attribution: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ForecastResidualKind {
    SourceProvenance,
    ForecastTimeAvailability,
    Mechanism,
    Regime,
    Calibration,
    ScorabilitySelection,
    ResolutionObserver,
    Comparator,
    ObjectDecomposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ForecastResidualProjection {
    pub residual_ref: String,
    pub forecast_or_score_ref: String,
    pub consumer_ref: String,
    pub kind: ForecastResidualKind,
    pub required_coordinate_ref: String,
    pub dependency_refs: Vec<String>,
    pub open: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ForecastResearchProbeKind {
    Support,
    Defeater,
    Comparator,
    Contradiction,
    Counterexample,
    VocabularyExploration,
    AuthorityFamilyExploration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ForecastResearchDemand {
    pub demand_ref: String,
    pub residual_ref: String,
    pub consumer_ref: String,
    pub probe_kind: ForecastResearchProbeKind,
    pub expected_proposition_shape_ref: String,
    pub temporal_cut_ref: String,
    pub source_policy_ref: String,
    pub budget_ref: String,
    pub candidate_only: bool,
    pub pays_residual: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}


#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ForecastCalibrationBucketProjection {
    pub bucket_ref: String,
    pub forecast_count: u64,
    pub average_predicted: Rational,
    pub observed_frequency: Rational,
    pub bucket_brier: Rational,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ForecastCalibrationBucketObservation {
    Empty { bucket_ref: String, reason_ref: String },
    Populated(ForecastCalibrationBucketProjection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ForecastAggregateScorecard {
    pub scorecard_ref: String,
    pub source_snapshot_ref: String,
    pub headline_count: u64,
    pub ledger_count: u64,
    pub resolved_count: u64,
    pub scored_count: u64,
    pub voided_count: u64,
    pub awaiting_judge_count: u64,
    pub still_open_count: u64,
    pub excluded_scored_count: u64,
    pub headline_brier: Rational,
    pub all_scored_brier: Rational,
    pub market_overlap_count: Option<u64>,
    pub market_overlap_forecast_brier: Option<Rational>,
    pub market_overlap_reference_brier: Option<Rational>,
    pub calibration: Vec<ForecastCalibrationBucketObservation>,
    pub individual_forecast_lineage_available: bool,
    pub resolution_evidence_available: bool,
    pub judge_input_lineage_available: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ForecastAggregateScorecard {
    pub fn validate(&self) -> Result<(), String> {
        if self.scorecard_ref.trim().is_empty() || self.source_snapshot_ref.trim().is_empty() {
            return Err("aggregate scorecard requires identity and source refs".into());
        }
        if self.resolved_count + self.awaiting_judge_count + self.still_open_count
            != self.ledger_count
        {
            return Err("aggregate scorecard ledger partition does not reconcile".into());
        }
        if self.scored_count + self.voided_count != self.resolved_count {
            return Err("aggregate scorecard resolved partition does not reconcile".into());
        }
        if self.headline_count + self.excluded_scored_count != self.scored_count {
            return Err("aggregate scorecard headline selection does not reconcile".into());
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("aggregate scorecard crossed non-promotion boundary".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forecast() -> PublishedBinaryForecast {
        PublishedBinaryForecast {
            forecast_ref: "forecast:1".into(),
            proposition_ref: "prop:1".into(),
            issued_at_epoch_ms: 1000,
            horizon_ref: "horizon:30d".into(),
            probability: Probability::new(3, 5).unwrap(),
            domain: ForecastDomain::Geopolitical,
            origin: ForecastOrigin::LegacyDetector,
            model_ref: "model:1".into(),
            source_state_ref: "source-state:1".into(),
            resolution_policy_ref: "policy:1".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    #[test]
    fn probability_is_exact_and_bounded() {
        assert_eq!(Probability::new(2, 4).unwrap().rational(), Rational::new(1, 2).unwrap());
        assert!(Probability::new(6, 5).is_err());
        assert!(Probability::new(1, 0).is_err());
    }

    #[test]
    fn forecast_time_cut_separates_hindsight() {
        let f = forecast();
        let early = ForecastEvidenceAvailability {
            evidence_ref: "e:early".into(),
            event_time_epoch_ms: None,
            created_at_epoch_ms: Some(800),
            published_at_epoch_ms: Some(900),
            discoverable_at_epoch_ms: Some(900),
            acquired_at_epoch_ms: 1500,
            available_at_epoch_ms: Some(900),
            provenance_ref: "prov:early".into(),
        };
        let late = ForecastEvidenceAvailability {
            available_at_epoch_ms: Some(1200),
            evidence_ref: "e:late".into(),
            provenance_ref: "prov:late".into(),
            ..early.clone()
        };
        let unresolved = ForecastEvidenceAvailability {
            available_at_epoch_ms: None,
            evidence_ref: "e:unknown".into(),
            provenance_ref: "prov:unknown".into(),
            ..early
        };

        assert_eq!(
            classify_forecast_time_admissibility(&f, &early),
            ForecastTimeAdmissibility::AdmissibleAtForecast
        );
        assert_eq!(
            classify_forecast_time_admissibility(&f, &late),
            ForecastTimeAdmissibility::HindsightOnly
        );
        assert_eq!(
            classify_forecast_time_admissibility(&f, &unresolved),
            ForecastTimeAdmissibility::AvailabilityUnresolved
        );
    }

    #[test]
    fn resolution_revision_cannot_create_world_event() {
        let bad = ForecastResolutionRevision {
            resolution_revision_ref: "rr:1".into(),
            forecast_ref: "forecast:1".into(),
            previous_revision_ref: None,
            state: ForecastResolutionState::Open,
            revision_evidence_ref: "evidence:1".into(),
            revision_policy_ref: "policy:1".into(),
            created_at_epoch_ms: 1,
            creates_world_event: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        assert!(bad.validate().is_err());
    }
}
