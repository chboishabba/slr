//! M11 cross-domain acceptance adapter for WorldMonitor forecast/evidence runs.
//!
//! Mirrors the bounded shape of worldmonitor/scripts/diff-forecast-runs.mjs:
//! run id, forecast depth/status, state labels, top forecast titles, interaction
//! counts, domain counts, and mapped-signal counts. Changes are typed without
//! promoting derived forecast/risk changes into world truth.

use serde::{Deserialize, Serialize};

pub const WORLDMONITOR_COMPARATIVE_SOURCE_COMMIT: &str =
    "7142fb8d806fbfe238b2918f4e22ba8ceeac06af";
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    compile_typed_change_set, typed_locus, ChangeLayer, ComparativeDelta,
    ComparativeDeltaKind, ComparativeDeltaRole, TypedChangeSet,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorldMonitorForecastRun {
    pub run_ref: String,
    pub forecast_depth: String,
    pub deep_forecast_status: String,
    pub state_labels: BTreeSet<String>,
    pub top_forecast_titles: BTreeSet<String>,
    pub traced_forecast_count: i64,
    pub impact_expansion_candidate_count: i64,
    pub impact_expansion_mapped_signal_count: i64,
    pub simulation_interaction_count: i64,
    pub reportable_interaction_count: i64,
    pub published_domain_counts: BTreeMap<String, i64>,
    pub source_measurement_refs: BTreeSet<String>,
    pub forecast_model_ref: Option<String>,
    pub dashboard_projection_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMonitorComparativeReceipt {
    pub change_set: TypedChangeSet,
    pub state_label_delta: bool,
    pub forecast_title_delta: bool,
    pub source_measurement_delta: bool,
    pub model_delta: bool,
    pub dashboard_projection_delta: bool,
    pub world_changed_inferred: bool,
    pub claim_truth_inferred: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn delta(
    delta_ref: impl Into<String>,
    kind: ComparativeDeltaKind,
    coordinate_ref: impl Into<String>,
    before_ref: Option<String>,
    after_ref: Option<String>,
) -> ComparativeDelta {
    ComparativeDelta {
        delta_ref: delta_ref.into(),
        kind,
        role: ComparativeDeltaRole::WorldInput,
        coordinate_ref: Some(coordinate_ref.into()),
        route_ref: None,
        residual_ref: None,
        before_ref,
        after_ref,
        cause_refs: BTreeSet::new(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

pub fn compare_worldmonitor_forecast_runs(
    comparison_ref: &str,
    baseline: &WorldMonitorForecastRun,
    candidate: &WorldMonitorForecastRun,
) -> Result<WorldMonitorComparativeReceipt, String> {
    if comparison_ref.trim().is_empty()
        || baseline.run_ref.trim().is_empty()
        || candidate.run_ref.trim().is_empty()
    {
        return Err("WorldMonitor comparison requires identity refs".into());
    }

    let mut loci = Vec::new();

    let source_measurement_delta =
        baseline.source_measurement_refs != candidate.source_measurement_refs;
    if source_measurement_delta {
        loci.push(typed_locus(
            "locus:worldmonitor:source-measurements",
            delta(
                "delta:worldmonitor:source-measurements",
                ComparativeDeltaKind::FactChanged,
                "coordinate:worldmonitor:source-measurements",
                Some(format!("{:?}", baseline.source_measurement_refs)),
                Some(format!("{:?}", candidate.source_measurement_refs)),
            ),
            ChangeLayer::WorldEvidence,
            Some("source-measurement-set".into()),
            BTreeSet::from(["layer:world".into()]),
            BTreeSet::from(["worldmonitor:cross-source-signal-evidence".into()]),
        )?);
    }

    let state_label_delta = baseline.state_labels != candidate.state_labels;
    let forecast_title_delta =
        baseline.top_forecast_titles != candidate.top_forecast_titles;
    let representation_delta = state_label_delta
        || forecast_title_delta
        || baseline.forecast_depth != candidate.forecast_depth
        || baseline.deep_forecast_status != candidate.deep_forecast_status
        || baseline.traced_forecast_count != candidate.traced_forecast_count
        || baseline.impact_expansion_candidate_count
            != candidate.impact_expansion_candidate_count
        || baseline.impact_expansion_mapped_signal_count
            != candidate.impact_expansion_mapped_signal_count
        || baseline.simulation_interaction_count
            != candidate.simulation_interaction_count
        || baseline.reportable_interaction_count
            != candidate.reportable_interaction_count
        || baseline.published_domain_counts != candidate.published_domain_counts;
    if representation_delta {
        loci.push(typed_locus(
            "locus:worldmonitor:forecast-representation",
            delta(
                "delta:worldmonitor:forecast-representation",
                ComparativeDeltaKind::FactChanged,
                "coordinate:worldmonitor:forecast-representation",
                Some(baseline.run_ref.clone()),
                Some(candidate.run_ref.clone()),
            ),
            ChangeLayer::Representation,
            Some("forecast-run".into()),
            BTreeSet::from(["layer:world".into()]),
            BTreeSet::from(["worldmonitor:diff-forecast-runs".into()]),
        )?);
    }

    let model_delta = baseline.forecast_model_ref != candidate.forecast_model_ref;
    if model_delta {
        loci.push(typed_locus(
            "locus:worldmonitor:forecast-model",
            delta(
                "delta:worldmonitor:forecast-model",
                ComparativeDeltaKind::FactChanged,
                "coordinate:worldmonitor:forecast-model",
                baseline.forecast_model_ref.clone(),
                candidate.forecast_model_ref.clone(),
            ),
            ChangeLayer::Theory,
            Some("forecast-model-policy".into()),
            BTreeSet::from(["layer:world".into()]),
            BTreeSet::from(["worldmonitor:explicit-model-ref".into()]),
        )?);
    }

    let dashboard_projection_delta =
        baseline.dashboard_projection_ref != candidate.dashboard_projection_ref;
    if dashboard_projection_delta {
        loci.push(typed_locus(
            "locus:worldmonitor:dashboard-projection",
            delta(
                "delta:worldmonitor:dashboard-projection",
                ComparativeDeltaKind::ConsumerDependencyChanged,
                "coordinate:worldmonitor:dashboard-projection",
                baseline.dashboard_projection_ref.clone(),
                candidate.dashboard_projection_ref.clone(),
            ),
            ChangeLayer::ConsumerProjection,
            Some("dashboard-risk-projection".into()),
            BTreeSet::from(["layer:world".into()]),
            BTreeSet::from(["worldmonitor:dashboard-projection".into()]),
        )?);
    }

    let change_set = compile_typed_change_set(
        comparison_ref,
        loci,
        [ChangeLayer::World],
    )?;

    Ok(WorldMonitorComparativeReceipt {
        change_set,
        state_label_delta,
        forecast_title_delta,
        source_measurement_delta,
        model_delta,
        dashboard_projection_delta,
        world_changed_inferred: false,
        claim_truth_inferred: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(reference: &str) -> WorldMonitorForecastRun {
        WorldMonitorForecastRun {
            run_ref: reference.into(),
            forecast_depth: "standard".into(),
            deep_forecast_status: "complete".into(),
            state_labels: BTreeSet::from(["stable".into()]),
            top_forecast_titles: BTreeSet::from(["baseline".into()]),
            traced_forecast_count: 2,
            impact_expansion_candidate_count: 1,
            impact_expansion_mapped_signal_count: 1,
            simulation_interaction_count: 3,
            reportable_interaction_count: 2,
            published_domain_counts: BTreeMap::from([("geopolitics".into(), 1)]),
            source_measurement_refs: BTreeSet::from(["signal:1".into()]),
            forecast_model_ref: Some("model:v1".into()),
            dashboard_projection_ref: Some("risk:v1".into()),
        }
    }

    #[test]
    fn forecast_change_is_representation_not_world_change() {
        let baseline = run("run:0");
        let mut candidate = run("run:1");
        candidate.state_labels.insert("escalating".into());
        candidate.top_forecast_titles.insert("candidate".into());

        let receipt =
            compare_worldmonitor_forecast_runs("comparison:wm", &baseline, &candidate)
                .unwrap();

        assert!(receipt.state_label_delta);
        assert!(receipt.forecast_title_delta);
        assert!(receipt
            .change_set
            .changed_layers
            .contains(&ChangeLayer::Representation));
        assert!(receipt
            .change_set
            .invariant_layers
            .contains(&ChangeLayer::World));
        assert!(!receipt.world_changed_inferred);
        assert!(!receipt.claim_truth_inferred);
    }

    #[test]
    fn source_measurement_model_and_dashboard_changes_stay_separate() {
        let baseline = run("run:0");
        let mut candidate = run("run:1");
        candidate.source_measurement_refs.insert("signal:2".into());
        candidate.forecast_model_ref = Some("model:v2".into());
        candidate.dashboard_projection_ref = Some("risk:v2".into());

        let receipt =
            compare_worldmonitor_forecast_runs("comparison:wm-axes", &baseline, &candidate)
                .unwrap();

        assert!(receipt
            .change_set
            .changed_layers
            .contains(&ChangeLayer::WorldEvidence));
        assert!(receipt
            .change_set
            .changed_layers
            .contains(&ChangeLayer::Theory));
        assert!(receipt
            .change_set
            .changed_layers
            .contains(&ChangeLayer::ConsumerProjection));
        assert!(!receipt.world_changed_inferred);
    }
}
