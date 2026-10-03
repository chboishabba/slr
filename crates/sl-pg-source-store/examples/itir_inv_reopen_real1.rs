//! INV-REOPEN-1 empirical selective-reopening runner.
//!
//! Normal execution accepts one persisted reopening-check ref.  Operational
//! dependency edges, expected-unrelated refs and any acquired native-product
//! selector are reopened from PostgreSQL; JSON is receipt export only.

use serde::Serialize;
use sensiblaw_pg_source_store::{
    load_database_config, load_inv_reopen_check, load_inv_reopen_dependency_control,
    load_relational_comparison, persist_acquisition_update, persist_relational_comparison,
    relational_observation_from_persisted_pnf, AcceptanceObservationKind,
    AcceptanceObservationSelector, AcquisitionUpdate, PersistedRelationalObservationRequest,
    RecordAvailability, RelationalObservation,
};
use std::{collections::BTreeSet, env, process};

const RECEIPT_SCHEMA: &str = "itir.inv-reopen-real1.receipt.v2-persisted-control";

#[derive(Debug, Serialize)]
struct Receipt {
    schema: &'static str,
    reopen_check_ref: String,
    case_ref: String,
    r0_comparison_ref: String,
    r1_comparison_ref: Option<String>,
    acquired_source_revision_ref: Option<String>,
    reopened_directly: bool,
    reopened_transitively: bool,
    unrelated_refs_not_reopened: Vec<String>,
    known_absence_closed_branch_only: bool,
    r0_finding: String,
    r1_finding: Option<String>,
    r0_residual_refs: Vec<String>,
    r1_residual_refs: Vec<String>,
    replacement_side: Option<String>,
    acquired_source_preexisted: bool,
    acquired_pnf_preexisted: bool,
    persisted_acceptance_control: bool,
    creates_semantic_authority: bool,
    claim_truth_promoted: bool,
}

fn availability(value: &str) -> Result<RecordAvailability, String> {
    match value {
        "present" => Ok(RecordAvailability::Present),
        "not_located" => Ok(RecordAvailability::NotLocated),
        "known_absent" => Ok(RecordAvailability::KnownAbsent),
        other => Err(format!("unknown persisted availability coordinate: {other}")),
    }
}

fn observation(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
    selector: &AcceptanceObservationSelector,
) -> Result<RelationalObservation, String> {
    if selector.kind != AcceptanceObservationKind::CandidatePnf {
        return Err("INV reopening currently requires a persisted candidate-PNF selector".into());
    }
    relational_observation_from_persisted_pnf(
        config,
        &PersistedRelationalObservationRequest {
            batch_ref: selector.batch_ref.clone().ok_or_else(|| "missing persisted batch_ref".to_owned())?,
            selected_predicate_candidate_ref: selector.selected_predicate_candidate_ref.clone()
                .ok_or_else(|| "missing persisted predicate candidate".to_owned())?,
            observation_ref: selector.observation_ref.clone(),
            source_family: selector.source_family.ok_or_else(|| "missing persisted source family".to_owned())?,
            context: selector.context.clone(),
            provenance_refs: selector.provenance_refs.clone(),
        },
    ).map_err(|error| error.to_string())
}

fn run() -> Result<(), String> {
    let reopen_check_ref = env::args().nth(1)
        .ok_or_else(|| "usage: itir_inv_reopen_real1 <persisted-reopen-check-ref>".to_owned())?;
    if env::args().nth(2).is_some() {
        return Err("INV-REOPEN-1 accepts one persisted reopening ref, not a JSON/request packet".into());
    }

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let request = load_inv_reopen_check(&config, &reopen_check_ref).map_err(|error| error.to_string())?;
    let dependency = load_inv_reopen_dependency_control(&config, &reopen_check_ref)
        .map_err(|error| error.to_string())?;
    if dependency.reopen_check_ref != request.reopen_check_ref {
        return Err("reopening dependency control does not belong to the requested check".into());
    }

    let r0 = load_relational_comparison(&config, &request.old_comparison_ref)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "R0 persisted REL comparison does not exist".to_owned())?;
    let r0_residual_refs = r0.comparison.residuals.iter()
        .map(|residual| residual.obligation_ref.clone()).collect::<Vec<_>>();

    let before = availability(&request.before_availability_ref)?;
    let after = availability(&request.after_availability_ref)?;
    let update = AcquisitionUpdate {
        obligation_ref: request.acquisition_obligation_ref.clone(),
        before,
        after,
        acquired_source_revision_ref: request.acquired_source_revision_ref.clone(),
        acquisition_receipt_ref: request.acquisition_receipt_ref.clone(),
    };

    if after == RecordAvailability::KnownAbsent {
        if request.new_selector.is_some() || request.acquired_source_revision_ref.is_some() {
            return Err("known-absent update may not carry an acquired source/PNF replacement".into());
        }
        let reopening = persist_acquisition_update(
            &config, &update, &dependency.dependency_graph_ref,
            &dependency.dependency_edges, &dependency.dependency_universe_refs,
        ).map_err(|error| error.to_string())?;
        if reopening.is_some() {
            return Err("known-absent update illegally created a selective reopening receipt".into());
        }
        let receipt = Receipt {
            schema: RECEIPT_SCHEMA,
            reopen_check_ref: request.reopen_check_ref,
            case_ref: request.case_ref,
            r0_comparison_ref: request.old_comparison_ref,
            r1_comparison_ref: None,
            acquired_source_revision_ref: None,
            reopened_directly: false,
            reopened_transitively: false,
            unrelated_refs_not_reopened: dependency.expected_unrelated_refs,
            known_absence_closed_branch_only: true,
            r0_finding: format!("{:?}", r0.comparison.finding),
            r1_finding: None,
            r0_residual_refs,
            r1_residual_refs: vec![],
            replacement_side: None,
            acquired_source_preexisted: false,
            acquired_pnf_preexisted: false,
            persisted_acceptance_control: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        };
        println!("{}", serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?);
        return Ok(());
    }

    if after != RecordAvailability::Present {
        return Err("INV-REOPEN-1 accepts only present or known_absent terminal updates".into());
    }
    let acquired_selector = request.new_selector.as_ref()
        .ok_or_else(|| "present-source update requires a persisted acquired selector".to_owned())?;
    let acquired = observation(&config, acquired_selector)?;
    if request.acquired_source_revision_ref.as_deref() != Some(acquired.source_revision_ref.as_str()) {
        return Err("acquisition update does not name the persisted source that owns the acquired PNF".into());
    }

    let reopening = persist_acquisition_update(
        &config, &update, &dependency.dependency_graph_ref,
        &dependency.dependency_edges, &dependency.dependency_universe_refs,
    ).map_err(|error| error.to_string())?
     .ok_or_else(|| "present-source acquisition must create a selective reopening receipt".to_owned())?;

    let direct = reopening.directly_affected_refs.iter().any(|reference| reference == &request.old_comparison_ref);
    let transitive = reopening.transitively_affected_refs.iter().any(|reference| reference == &request.old_comparison_ref);
    if !direct && !transitive {
        return Err("R0 comparison is not in the selective reopening cone; refusing unrelated recomputation".into());
    }

    let unrelated = reopening.unrelated_refs_not_reopened.iter().cloned().collect::<BTreeSet<_>>();
    if unrelated.contains(&request.old_comparison_ref) {
        return Err("R0 was simultaneously marked affected and unrelated".into());
    }
    let expected_unrelated = dependency.expected_unrelated_refs.iter().cloned().collect::<BTreeSet<_>>();
    if !expected_unrelated.is_subset(&unrelated) {
        return Err("one or more persisted unrelated consumers were reopened or omitted from the non-reopen receipt".into());
    }

    let (left, right) = match request.replace_side_ref.as_str() {
        "left" => (acquired.clone(), r0.right.clone()),
        "right" => (r0.left.clone(), acquired.clone()),
        _ => return Err("persisted replacement side must be left or right".into()),
    };
    if left.source_revision_ref == right.source_revision_ref {
        return Err("reopened comparison would collapse two source revisions into one".into());
    }
    let r1 = persist_relational_comparison(&config, &left, &right, &r0.consumer)
        .map_err(|error| error.to_string())?;

    let receipt = Receipt {
        schema: RECEIPT_SCHEMA,
        reopen_check_ref: request.reopen_check_ref,
        case_ref: request.case_ref,
        r0_comparison_ref: request.old_comparison_ref,
        r1_comparison_ref: Some(r1.comparison.comparison_ref.clone()),
        acquired_source_revision_ref: Some(acquired.source_revision_ref),
        reopened_directly: direct,
        reopened_transitively: transitive,
        unrelated_refs_not_reopened: reopening.unrelated_refs_not_reopened,
        known_absence_closed_branch_only: false,
        r0_finding: format!("{:?}", r0.comparison.finding),
        r1_finding: Some(format!("{:?}", r1.comparison.finding)),
        r0_residual_refs,
        r1_residual_refs: r1.comparison.residuals.iter().map(|residual| residual.obligation_ref.clone()).collect(),
        replacement_side: Some(request.replace_side_ref),
        acquired_source_preexisted: true,
        acquired_pnf_preexisted: true,
        persisted_acceptance_control: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    };
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
