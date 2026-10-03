//! INV-REOPEN-1 empirical selective-reopening runner.
//!
//! The newly acquired source must already be persisted through its native
//! source path and compiled to candidate PNF.  This runner records the
//! acquisition update, verifies that the old REL comparison is actually in the
//! selective reopening cone, then constructs R1 by replacing exactly one side
//! with the newly persisted observation and re-running the same consumer.

use serde::{Deserialize, Serialize};
use sensiblaw_pg_source_store::{
    load_database_config, load_relational_comparison, persist_acquisition_update,
    persist_relational_comparison, relational_observation_from_persisted_pnf,
    AcquisitionUpdate, ObservationContext, PersistedRelationalObservationRequest,
    RelationalSourceFamily,
};
use std::{collections::BTreeSet, env, fs, process};

const REQUEST_SCHEMA: &str = "itir.inv-reopen-real1.request.v1";
const RECEIPT_SCHEMA: &str = "itir.inv-reopen-real1.receipt.v1";

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReplacementSide {
    Left,
    Right,
}

#[derive(Debug, Deserialize)]
struct PnfObservationSpec {
    batch_ref: String,
    selected_predicate_candidate_ref: String,
    observation_ref: String,
    source_family: RelationalSourceFamily,
    #[serde(default)]
    context: ObservationContext,
    #[serde(default)]
    provenance_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Request {
    schema: String,
    r0_comparison_ref: String,
    update: AcquisitionUpdate,
    dependency_graph_ref: String,
    dependency_edges: Vec<(String, String)>,
    dependency_universe_refs: Vec<String>,
    replacement_side: ReplacementSide,
    acquired_observation: PnfObservationSpec,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema: &'static str,
    r0_comparison_ref: String,
    r1_comparison_ref: String,
    acquired_source_revision_ref: String,
    reopened_directly: bool,
    reopened_transitively: bool,
    unrelated_refs_not_reopened: Vec<String>,
    r0_finding: String,
    r1_finding: String,
    r0_residual_refs: Vec<String>,
    r1_residual_refs: Vec<String>,
    replacement_side: &'static str,
    acquired_source_preexisted: bool,
    acquired_pnf_preexisted: bool,
    creates_semantic_authority: bool,
    claim_truth_promoted: bool,
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: itir_inv_reopen_real1 <reopening-request.json>".to_owned())?;
    let request: Request = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if request.schema != REQUEST_SCHEMA || request.dependency_graph_ref.trim().is_empty() {
        return Err("invalid INV-REOPEN-1 request".into());
    }

    let config = load_database_config(None).map_err(|e| e.to_string())?;
    let r0 = load_relational_comparison(&config, &request.r0_comparison_ref)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "R0 persisted REL comparison does not exist".to_owned())?;

    let acquired = relational_observation_from_persisted_pnf(
        &config,
        &PersistedRelationalObservationRequest {
            batch_ref: request.acquired_observation.batch_ref,
            selected_predicate_candidate_ref: request
                .acquired_observation
                .selected_predicate_candidate_ref,
            observation_ref: request.acquired_observation.observation_ref,
            source_family: request.acquired_observation.source_family,
            context: request.acquired_observation.context,
            provenance_refs: request.acquired_observation.provenance_refs,
        },
    )
    .map_err(|e| e.to_string())?;
    if request.update.acquired_source_revision_ref.as_deref()
        != Some(acquired.source_revision_ref.as_str())
    {
        return Err("acquisition update does not name the persisted source that owns the acquired PNF".into());
    }

    let reopening = persist_acquisition_update(
        &config,
        &request.update,
        &request.dependency_graph_ref,
        &request.dependency_edges,
        &request.dependency_universe_refs,
    )
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "present-source acquisition must create a selective reopening receipt".to_owned())?;

    let direct = reopening
        .directly_affected_refs
        .iter()
        .any(|reference| reference == &request.r0_comparison_ref);
    let transitive = reopening
        .transitively_affected_refs
        .iter()
        .any(|reference| reference == &request.r0_comparison_ref);
    if !direct && !transitive {
        return Err("R0 comparison is not in the selective reopening cone; refusing unrelated recomputation".into());
    }

    let (left, right, side) = match request.replacement_side {
        ReplacementSide::Left => (acquired.clone(), r0.right.clone(), "left"),
        ReplacementSide::Right => (r0.left.clone(), acquired.clone(), "right"),
    };
    if left.source_revision_ref == right.source_revision_ref {
        return Err("reopened comparison would collapse two source revisions into one".into());
    }
    let r1 = persist_relational_comparison(&config, &left, &right, &r0.consumer)
        .map_err(|e| e.to_string())?;

    let unrelated = reopening
        .unrelated_refs_not_reopened
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if unrelated.contains(&request.r0_comparison_ref) {
        return Err("R0 was simultaneously marked affected and unrelated".into());
    }

    let receipt = Receipt {
        schema: RECEIPT_SCHEMA,
        r0_comparison_ref: request.r0_comparison_ref,
        r1_comparison_ref: r1.comparison.comparison_ref.clone(),
        acquired_source_revision_ref: acquired.source_revision_ref,
        reopened_directly: direct,
        reopened_transitively: transitive,
        unrelated_refs_not_reopened: reopening.unrelated_refs_not_reopened,
        r0_finding: format!("{:?}", r0.comparison.finding),
        r1_finding: format!("{:?}", r1.comparison.finding),
        r0_residual_refs: r0
            .comparison
            .residuals
            .iter()
            .map(|residual| residual.obligation_ref.clone())
            .collect(),
        r1_residual_refs: r1
            .comparison
            .residuals
            .iter()
            .map(|residual| residual.obligation_ref.clone())
            .collect(),
        replacement_side: side,
        acquired_source_preexisted: true,
        acquired_pnf_preexisted: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    };
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
