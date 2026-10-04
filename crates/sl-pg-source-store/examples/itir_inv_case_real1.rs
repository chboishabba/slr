//! INV-CASE-REAL-1 bottom-up empirical acceptance runner.
//!
//! Normal execution accepts exactly one persisted acceptance-case ref.  JSON
//! is not a control-plane input.  The case reopens its native-product selectors,
//! consumer fibre, reviewed-evidence coordinate, acquisition routes, governance
//! controls and graph binding from PostgreSQL.

use serde::Serialize;
use sensiblaw_pg_source_store::{
    build_inv_governance_packet, build_investigation_graph_binding,
    load_bound_investigation_graph_projection, load_database_config,
    load_inv_acceptance_case, load_reviewed_evidence_coordinate,
    materialize_legal_ir_from_reviewed_evidence, obligation_from_residual,
    persist_acquisition_queue, persist_investigation_graph_binding,
    persist_inv_governance_packet, persist_relational_comparison,
    persist_reviewed_legal_follow_projection,
    relational_observation_from_persisted_pnf,
    AcceptanceObservationKind, AcceptanceObservationSelector,
    PersistedRelationalObservationRequest, RelationalObservation,
    ServiceEvidenceState,
};
use std::{collections::BTreeSet, env, process};

const RECEIPT_SCHEMA: &str = "itir.inv-case-real1.receipt.v2-persisted-control";

#[derive(Debug, Serialize)]
struct Receipt {
    schema: &'static str,
    case_ref: String,
    matter_ref: String,
    reviewed_evidence_ref: String,
    normative_order_ref: String,
    left_source_revision_ref: String,
    right_source_revision_ref: String,
    left_observation_ref: String,
    right_observation_ref: String,
    comparison_ref: String,
    residual_obligation_ref: String,
    legal_ir_projection_ref: String,
    legal_follow_projection_ref: String,
    inv_obligation_ref: String,
    gov_packet_ref: String,
    graph_binding_ref: String,
    graph_node_count: usize,
    graph_edge_count: usize,
    required_matter_visibility_refs: Vec<String>,
    persisted_acceptance_control: bool,
    persisted_reviewed_evidence: bool,
    persisted_pnf_grounding: bool,
    rel_reopened_exactly: bool,
    legal_follow_derived_only: bool,
    inv_derived_from_rel_residual: bool,
    graph_binding_reopened_exactly: bool,
    creates_semantic_authority: bool,
    creates_claim_truth: bool,
    creates_access_authority: bool,
    acquisition_executed: bool,
}

fn observation(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
    selector: &AcceptanceObservationSelector,
) -> Result<RelationalObservation, String> {
    if selector.kind != AcceptanceObservationKind::CandidatePnf {
        return Err(
            "INV-CASE-REAL-1 currently requires persisted candidate-PNF selectors; native Wikidata remains a REL-corpus source lane"
                .into(),
        );
    }
    relational_observation_from_persisted_pnf(
        config,
        &PersistedRelationalObservationRequest {
            batch_ref: selector
                .batch_ref
                .clone()
                .ok_or_else(|| "persisted INV selector lacks batch_ref".to_owned())?,
            selected_predicate_candidate_ref: selector
                .selected_predicate_candidate_ref
                .clone()
                .ok_or_else(|| {
                    "persisted INV selector lacks selected predicate candidate".to_owned()
                })?,
            observation_ref: selector.observation_ref.clone(),
            source_family: selector
                .source_family
                .ok_or_else(|| "persisted INV selector lacks source family".to_owned())?,
            context: selector.context.clone(),
            provenance_refs: selector.provenance_refs.clone(),
        },
    )
    .map_err(|error| error.to_string())
}

fn run() -> Result<(), String> {
    let case_ref = env::args()
        .nth(1)
        .ok_or_else(|| "usage: itir_inv_case_real1 <persisted-case-ref>".to_owned())?;
    if env::args().nth(2).is_some() {
        return Err("INV-CASE-REAL-1 accepts one persisted case ref, not a JSON/request packet".into());
    }

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let request = load_inv_acceptance_case(&config, &case_ref).map_err(|error| error.to_string())?;
    if request.routes.len() < 2 || request.graph_binding_evidence_refs.is_empty() {
        return Err(
            "persisted INV case requires at least two acquisition routes and graph-binding evidence"
                .into(),
        );
    }

    let left = observation(&config, &request.left)?;
    let right = observation(&config, &request.right)?;
    if left.source_revision_ref == right.source_revision_ref {
        return Err("real case requires two distinct persisted source revisions".into());
    }

    let rel = persist_relational_comparison(&config, &left, &right, &request.consumer)
        .map_err(|error| error.to_string())?;
    let residual = rel
        .comparison
        .residuals
        .get(request.residual_ordinal)
        .ok_or_else(|| {
            "selected persisted residual ordinal does not exist in the REL comparison".to_owned()
        })?
        .clone();

    let reviewed = load_reviewed_evidence_coordinate(&config, &request.reviewed_evidence_ref)
        .map_err(|error| error.to_string())?;
    if reviewed.source_revision_ref != left.source_revision_ref
        && reviewed.source_revision_ref != right.source_revision_ref
    {
        return Err(
            "persisted reviewed evidence must belong to one of the two REL source revisions"
                .into(),
        );
    }
    if reviewed.consumer_ref != request.consumer.consumer_ref {
        return Err(
            "reviewed-evidence consumer differs from the persisted case consumer fibre".into(),
        );
    }
    let legal_ir = materialize_legal_ir_from_reviewed_evidence(
        &config,
        &request.reviewed_evidence_ref,
    )
    .map_err(|error| error.to_string())?;
    let legal_support = reviewed.as_legal_ir_support();
    let legal_follow = persist_reviewed_legal_follow_projection(
        &config,
        &legal_support,
        &legal_ir,
    )
    .map_err(|error| error.to_string())?;

    let mut source_revision_refs = vec![
        left.source_revision_ref.clone(),
        right.source_revision_ref.clone(),
    ];
    source_revision_refs.sort();
    source_revision_refs.dedup();
    let obligation = obligation_from_residual(
        &rel.comparison,
        &residual,
        source_revision_refs.clone(),
        &request.target_description,
        &request.authority_or_access_constraint_ref,
        request.dependency_target_refs.clone(),
    )
    .map_err(|error| error.to_string())?;

    let mut routes = request.routes.clone();
    for route in &mut routes {
        route.obligation_ref = obligation.obligation_ref.clone();
    }
    let queue = persist_acquisition_queue(&config, &obligation, &routes)
        .map_err(|error| error.to_string())?;

    let governance = &request.governance;
    let gov = build_inv_governance_packet(
        &governance.packet_ref,
        &obligation,
        &governance.purpose_ref,
        governance.access_state,
        governance.privacy_state,
        governance.ai_use_state,
        ServiceEvidenceState {
            service_change_state: governance.service_change_state.clone(),
            evidence_state: governance.evidence_state.clone(),
        },
        governance.control_refs.clone(),
        governance.evidence_refs.clone(),
        governance.security_refs.clone(),
    )
    .map_err(|error| error.to_string())?;
    let gov = persist_inv_governance_packet(&config, &gov).map_err(|error| error.to_string())?;

    let binding = build_investigation_graph_binding(
        &request.graph_binding_ref,
        &obligation.obligation_ref,
        &legal_follow.projection_ref,
        &request.matter_ref,
        request.graph_binding_evidence_refs.clone(),
    )
    .map_err(|error| error.to_string())?;
    let binding = persist_investigation_graph_binding(&config, &binding)
        .map_err(|error| error.to_string())?;
    let bound = load_bound_investigation_graph_projection(
        &config,
        &obligation.obligation_ref,
        &request.matter_ref,
    )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "persisted investigation graph did not reopen".to_owned())?;
    if bound.binding != binding
        || bound.projection.legal_follow_graph.projection_ref != legal_follow.projection_ref
    {
        return Err("graph binding/projection changed on reopen".into());
    }

    let mut visibility = BTreeSet::new();
    visibility.extend(source_revision_refs.iter().cloned());
    visibility.extend(bound.projection.source_refs.iter().cloned());

    let receipt = Receipt {
        schema: RECEIPT_SCHEMA,
        case_ref: request.case_ref,
        matter_ref: request.matter_ref,
        reviewed_evidence_ref: reviewed.reviewed_evidence_ref,
        normative_order_ref: reviewed.normative_order_ref,
        left_source_revision_ref: left.source_revision_ref.clone(),
        right_source_revision_ref: right.source_revision_ref.clone(),
        left_observation_ref: left.observation_ref.clone(),
        right_observation_ref: right.observation_ref.clone(),
        comparison_ref: rel.comparison.comparison_ref.clone(),
        residual_obligation_ref: residual.obligation_ref.clone(),
        legal_ir_projection_ref: legal_ir.projection_ref,
        legal_follow_projection_ref: legal_follow.projection_ref,
        inv_obligation_ref: queue.obligation.obligation_ref.clone(),
        gov_packet_ref: gov.packet_ref,
        graph_binding_ref: binding.binding_ref,
        graph_node_count: bound.projection.legal_follow_graph.nodes.len(),
        graph_edge_count: bound.projection.legal_follow_graph.edges.len(),
        required_matter_visibility_refs: visibility.into_iter().collect(),
        persisted_acceptance_control: true,
        persisted_reviewed_evidence: true,
        persisted_pnf_grounding: true,
        rel_reopened_exactly: true,
        legal_follow_derived_only: bound.projection.legal_follow_graph.derived_only,
        inv_derived_from_rel_residual: queue.obligation.residual_obligation_ref
            == residual.obligation_ref,
        graph_binding_reopened_exactly: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
        creates_access_authority: false,
        acquisition_executed: false,
    };

    // JSON here is receipt export only. It is not consumed by the execution path.
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
