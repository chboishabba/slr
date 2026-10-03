//! ITIR INV/GOV case receipt over already-persisted canonical objects.
//!
//! This executable deliberately cannot create source revisions or REL
//! comparisons. It starts from one existing persisted REL comparison and one
//! residual already owned by that comparison, persists the INV queue and GOV
//! sidecar, reopens both, and optionally records a separately completed
//! acquisition result after the acquired source has already been ingested by
//! its native adapter.

use serde::{Deserialize, Serialize};
use sensiblaw_pg_source_store::{
    build_inv_governance_packet, load_acquisition_queue, load_database_config,
    load_inv_governance_packet, load_relational_comparison, obligation_from_residual,
    persist_acquisition_queue, persist_acquisition_update, persist_inv_governance_packet,
    AcquisitionGovernancePacket, AcquisitionRouteCandidate, AcquisitionUpdate,
    AiUseState, DurableAcquisitionQueue, GovernanceAccessState, PrivacyExposureState,
    SelectiveReopeningReceipt, ServiceEvidenceState,
};
use std::{env, fs, process};

const REQUEST_SCHEMA: &str = "itir.inv1.governed-case-request.v1";
const RECEIPT_SCHEMA: &str = "itir.inv1.governed-case-receipt.v1";

#[derive(Debug, Deserialize)]
struct GovernanceRequest {
    packet_ref: String,
    purpose_ref: String,
    access_state: GovernanceAccessState,
    privacy_state: PrivacyExposureState,
    ai_use_state: AiUseState,
    service_change_state: String,
    evidence_state: String,
    control_refs: Vec<String>,
    evidence_refs: Vec<String>,
    security_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AcquisitionResultRequest {
    update: AcquisitionUpdate,
    dependency_graph_ref: String,
    dependency_edges: Vec<(String, String)>,
    dependency_universe_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Request {
    schema: String,
    comparison_ref: String,
    residual_obligation_ref: String,
    target_description: String,
    authority_or_access_constraint_ref: String,
    dependency_target_refs: Vec<String>,
    routes: Vec<AcquisitionRouteCandidate>,
    governance: GovernanceRequest,
    #[serde(default)]
    acquisition_result: Option<AcquisitionResultRequest>,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema: &'static str,
    comparison_ref: String,
    residual_obligation_ref: String,
    source_revision_refs: Vec<String>,
    queue: DurableAcquisitionQueue,
    governance: AcquisitionGovernancePacket,
    selective_reopening: Option<SelectiveReopeningReceipt>,
    queue_reopened_exactly: bool,
    governance_reopened_exactly: bool,
    acquisition_result_recorded: bool,
    source_created_by_this_program: bool,
    relational_comparison_created_by_this_program: bool,
    acquisition_executed_by_this_program: bool,
    scalar_score_used: bool,
    semantic_authority_created: bool,
    access_authority_created: bool,
    certification_claim_created: bool,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_request(request: &Request) -> Result<(), String> {
    if request.schema != REQUEST_SCHEMA {
        return Err("unsupported governed-case request schema".into());
    }
    if !nonempty(&request.comparison_ref)
        || !nonempty(&request.residual_obligation_ref)
        || !nonempty(&request.target_description)
        || !nonempty(&request.authority_or_access_constraint_ref)
        || request.routes.is_empty()
        || request.dependency_target_refs.iter().any(|value| !nonempty(value))
    {
        return Err("governed-case request is missing required INV coordinates".into());
    }
    let governance = &request.governance;
    if !nonempty(&governance.packet_ref)
        || !nonempty(&governance.purpose_ref)
        || !nonempty(&governance.service_change_state)
        || !nonempty(&governance.evidence_state)
    {
        return Err("governance request is missing required coordinates".into());
    }
    if let Some(result) = request.acquisition_result.as_ref() {
        if !nonempty(&result.dependency_graph_ref) {
            return Err("acquisition result requires an explicit dependency graph ref".into());
        }
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let path = env::args().nth(1).ok_or_else(|| {
        "usage: itir_inv1_governed_case <governed-case-request.json>".to_owned()
    })?;
    let request: Request = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    validate_request(&request)?;

    let config = load_database_config(None).map_err(|e| e.to_string())?;
    let durable_rel = load_relational_comparison(&config, &request.comparison_ref)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "unknown persisted REL comparison; this program will not manufacture one".to_owned()
        })?;
    let residual = durable_rel
        .comparison
        .residuals
        .iter()
        .find(|residual| residual.obligation_ref == request.residual_obligation_ref)
        .ok_or_else(|| {
            "requested residual is not owned by the selected persisted REL comparison".to_owned()
        })?;

    let mut source_revision_refs = vec![
        durable_rel.left.source_revision_ref.clone(),
        durable_rel.right.source_revision_ref.clone(),
    ];
    source_revision_refs.sort();
    source_revision_refs.dedup();

    let obligation = obligation_from_residual(
        &durable_rel.comparison,
        residual,
        source_revision_refs.clone(),
        &request.target_description,
        &request.authority_or_access_constraint_ref,
        request.dependency_target_refs.clone(),
    )
    .map_err(|e| e.to_string())?;

    let queue = persist_acquisition_queue(&config, &obligation, &request.routes)
        .map_err(|e| e.to_string())?;
    let reopened_queue = load_acquisition_queue(&config, &obligation.obligation_ref)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "persisted INV queue did not reopen".to_owned())?;
    if reopened_queue != queue {
        return Err("persisted INV queue changed on reopen".into());
    }

    let governance_request = request.governance;
    let governance = build_inv_governance_packet(
        &governance_request.packet_ref,
        &obligation,
        &governance_request.purpose_ref,
        governance_request.access_state,
        governance_request.privacy_state,
        governance_request.ai_use_state,
        ServiceEvidenceState {
            service_change_state: governance_request.service_change_state,
            evidence_state: governance_request.evidence_state,
        },
        governance_request.control_refs,
        governance_request.evidence_refs,
        governance_request.security_refs,
    )
    .map_err(|e| e.to_string())?;
    let governance = persist_inv_governance_packet(&config, &governance)
        .map_err(|e| e.to_string())?;
    let reopened_governance = load_inv_governance_packet(&config, &governance.packet_ref)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "persisted GOV packet did not reopen".to_owned())?;
    if reopened_governance != governance {
        return Err("persisted GOV packet changed on reopen".into());
    }

    let (selective_reopening, acquisition_result_recorded) =
        if let Some(result) = request.acquisition_result {
            if result.update.obligation_ref != obligation.obligation_ref {
                return Err("acquisition result is not bound to the generated obligation".into());
            }
            let reopening = persist_acquisition_update(
                &config,
                &result.update,
                &result.dependency_graph_ref,
                &result.dependency_edges,
                &result.dependency_universe_refs,
            )
            .map_err(|e| e.to_string())?;
            (reopening, true)
        } else {
            (None, false)
        };

    let receipt = Receipt {
        schema: RECEIPT_SCHEMA,
        comparison_ref: durable_rel.comparison.comparison_ref.clone(),
        residual_obligation_ref: residual.obligation_ref.clone(),
        source_revision_refs,
        scalar_score_used: queue.priority.scalar_score_used,
        queue,
        governance,
        selective_reopening,
        queue_reopened_exactly: true,
        governance_reopened_exactly: true,
        acquisition_result_recorded,
        source_created_by_this_program: false,
        relational_comparison_created_by_this_program: false,
        acquisition_executed_by_this_program: false,
        semantic_authority_created: false,
        access_authority_created: false,
        certification_claim_created: false,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
