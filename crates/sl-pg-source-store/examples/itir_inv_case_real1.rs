//! INV-CASE-REAL-1 bottom-up empirical acceptance runner.
//!
//! This program cannot create source text, parser output, PNF factors or a
//! hand-authored REL observation.  It starts from two persisted candidate-PNF
//! batches, constructs the generic observations from those durable products,
//! persists a genuine REL comparison/residual, materializes reviewed legal IR
//! from already-existing legal source/PNF coordinates, projects that reviewed
//! support relation into the derived-only legal-follow graph, and only then
//! creates INV/GOV and the graph binding.

use serde::{Deserialize, Serialize};
use sensiblaw_pg_source_store::{
    build_inv_governance_packet, build_investigation_graph_binding,
    load_bound_investigation_graph_projection, load_database_config,
    materialize_reviewed_proposition_support, obligation_from_residual,
    persist_acquisition_queue, persist_investigation_graph_binding,
    persist_inv_governance_packet, persist_relational_comparison,
    persist_reviewed_legal_follow_projection,
    relational_observation_from_persisted_pnf, AccessDisposition,
    AcquisitionRouteCandidate, AiUseState, DuplicateRelation,
    EvidenceIndependence, GovernanceAccessState, ObservationContext,
    PersistedRelationalObservationRequest, PrivacyExposureState,
    RelationalConsumer, RelationalSourceFamily, ReviewedPropositionSupport,
    ServiceEvidenceState,
};
use std::{collections::BTreeSet, env, fs, process};

const REQUEST_SCHEMA: &str = "itir.inv-case-real1.request.v1";
const RECEIPT_SCHEMA: &str = "itir.inv-case-real1.receipt.v1";

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
struct LegalSupportSpec {
    proposition_ref: String,
    source_revision_ref: String,
    document_ref: String,
    exact_span_ref: String,
    parser_build_ref: String,
    pnf_build_ref: String,
    refined_pnf_graph_ref: String,
    pnf_factor_ref: String,
    pnf_revision_ref: String,
    structural_signature_ref: String,
    predicate_ref: String,
    observation_provenance_refs: Vec<String>,
    #[serde(default)]
    observation_residual_refs: Vec<String>,
    #[serde(default)]
    legal_system_refs: Vec<String>,
    #[serde(default)]
    jurisdiction_refs: Vec<String>,
    #[serde(default)]
    temporal_refs: Vec<String>,
    author_ref: String,
    #[serde(default)]
    institution_ref: Option<String>,
}

impl From<LegalSupportSpec> for ReviewedPropositionSupport {
    fn from(value: LegalSupportSpec) -> Self {
        Self {
            proposition_ref: value.proposition_ref,
            source_revision_ref: value.source_revision_ref,
            document_ref: value.document_ref,
            exact_span_ref: value.exact_span_ref,
            parser_build_ref: value.parser_build_ref,
            pnf_build_ref: value.pnf_build_ref,
            refined_pnf_graph_ref: value.refined_pnf_graph_ref,
            pnf_factor_ref: value.pnf_factor_ref,
            pnf_revision_ref: value.pnf_revision_ref,
            structural_signature_ref: value.structural_signature_ref,
            predicate_ref: value.predicate_ref,
            observation_provenance_refs: value.observation_provenance_refs,
            observation_residual_refs: value.observation_residual_refs,
            legal_system_refs: value.legal_system_refs,
            jurisdiction_refs: value.jurisdiction_refs,
            temporal_refs: value.temporal_refs,
            author_ref: value.author_ref,
            institution_ref: value.institution_ref,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RouteSpec {
    route_ref: String,
    route_description: String,
    source_locator_ref: String,
    access_disposition: AccessDisposition,
    #[serde(default)]
    authority_receipt_ref: Option<String>,
    provenance_genealogy_ref: String,
    independence: EvidenceIndependence,
    #[serde(default)]
    independence_receipt_ref: Option<String>,
    duplicate_relation: DuplicateRelation,
    information_gain: u32,
    dependency_closure_impact: u32,
    residual_coverage: u32,
    provenance_novelty: u32,
    acquisition_cost: u32,
    axis_estimation_receipt_ref: String,
}

#[derive(Debug, Deserialize)]
struct GovernanceSpec {
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
struct Request {
    schema: String,
    matter_ref: String,
    left: PnfObservationSpec,
    right: PnfObservationSpec,
    consumer: RelationalConsumer,
    residual_ordinal: usize,
    target_description: String,
    authority_or_access_constraint_ref: String,
    dependency_target_refs: Vec<String>,
    routes: Vec<RouteSpec>,
    legal_support: LegalSupportSpec,
    governance: GovernanceSpec,
    graph_binding_ref: String,
    graph_binding_evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema: &'static str,
    matter_ref: String,
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

fn observation_request(spec: PnfObservationSpec) -> PersistedRelationalObservationRequest {
    PersistedRelationalObservationRequest {
        batch_ref: spec.batch_ref,
        selected_predicate_candidate_ref: spec.selected_predicate_candidate_ref,
        observation_ref: spec.observation_ref,
        source_family: spec.source_family,
        context: spec.context,
        provenance_refs: spec.provenance_refs,
    }
}

fn route(spec: RouteSpec, obligation_ref: &str) -> AcquisitionRouteCandidate {
    AcquisitionRouteCandidate {
        route_ref: spec.route_ref,
        obligation_ref: obligation_ref.into(),
        route_description: spec.route_description,
        source_locator_ref: spec.source_locator_ref,
        access_disposition: spec.access_disposition,
        authority_receipt_ref: spec.authority_receipt_ref,
        provenance_genealogy_ref: spec.provenance_genealogy_ref,
        independence: spec.independence,
        independence_receipt_ref: spec.independence_receipt_ref,
        duplicate_relation: spec.duplicate_relation,
        information_gain: spec.information_gain,
        dependency_closure_impact: spec.dependency_closure_impact,
        residual_coverage: spec.residual_coverage,
        provenance_novelty: spec.provenance_novelty,
        acquisition_cost: spec.acquisition_cost,
        axis_estimation_receipt_ref: spec.axis_estimation_receipt_ref,
        candidate_only: true,
        creates_acquisition_authority: false,
    }
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: itir_inv_case_real1 <real-case-request.json>".to_owned())?;
    let request: Request = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if request.schema != REQUEST_SCHEMA
        || request.matter_ref.trim().is_empty()
        || request.routes.len() < 2
        || request.graph_binding_evidence_refs.is_empty()
    {
        return Err("INV-CASE-REAL-1 requires a matter, two acquisition routes, and graph-binding evidence".into());
    }

    let config = load_database_config(None).map_err(|e| e.to_string())?;
    let left = relational_observation_from_persisted_pnf(&config, &observation_request(request.left))
        .map_err(|e| e.to_string())?;
    let right = relational_observation_from_persisted_pnf(&config, &observation_request(request.right))
        .map_err(|e| e.to_string())?;
    if left.source_revision_ref == right.source_revision_ref {
        return Err("real case requires two distinct persisted source revisions".into());
    }

    let rel = persist_relational_comparison(&config, &left, &right, &request.consumer)
        .map_err(|e| e.to_string())?;
    let residual = rel
        .comparison
        .residuals
        .get(request.residual_ordinal)
        .ok_or_else(|| "selected residual ordinal does not exist in the persisted comparison".to_owned())?
        .clone();

    let legal_support: ReviewedPropositionSupport = request.legal_support.into();
    if legal_support.source_revision_ref != left.source_revision_ref
        && legal_support.source_revision_ref != right.source_revision_ref
    {
        return Err("reviewed legal-follow support must belong to one of the two REL source revisions".into());
    }
    let legal_ir = materialize_reviewed_proposition_support(&config, &legal_support)
        .map_err(|e| e.to_string())?;
    let legal_follow = persist_reviewed_legal_follow_projection(&config, &legal_support, &legal_ir)
        .map_err(|e| e.to_string())?;

    let mut source_revision_refs = vec![left.source_revision_ref.clone(), right.source_revision_ref.clone()];
    source_revision_refs.sort();
    source_revision_refs.dedup();
    let obligation = obligation_from_residual(
        &rel.comparison,
        &residual,
        source_revision_refs.clone(),
        &request.target_description,
        &request.authority_or_access_constraint_ref,
        request.dependency_target_refs,
    )
    .map_err(|e| e.to_string())?;
    let routes = request
        .routes
        .into_iter()
        .map(|spec| route(spec, &obligation.obligation_ref))
        .collect::<Vec<_>>();
    let queue = persist_acquisition_queue(&config, &obligation, &routes)
        .map_err(|e| e.to_string())?;

    let governance = request.governance;
    let gov = build_inv_governance_packet(
        &governance.packet_ref,
        &obligation,
        &governance.purpose_ref,
        governance.access_state,
        governance.privacy_state,
        governance.ai_use_state,
        ServiceEvidenceState {
            service_change_state: governance.service_change_state,
            evidence_state: governance.evidence_state,
        },
        governance.control_refs,
        governance.evidence_refs,
        governance.security_refs,
    )
    .map_err(|e| e.to_string())?;
    let gov = persist_inv_governance_packet(&config, &gov).map_err(|e| e.to_string())?;

    let binding = build_investigation_graph_binding(
        &request.graph_binding_ref,
        &obligation.obligation_ref,
        &legal_follow.projection_ref,
        &request.matter_ref,
        request.graph_binding_evidence_refs,
    )
    .map_err(|e| e.to_string())?;
    let binding = persist_investigation_graph_binding(&config, &binding)
        .map_err(|e| e.to_string())?;
    let bound = load_bound_investigation_graph_projection(&config, &obligation.obligation_ref)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "persisted investigation graph did not reopen".to_owned())?;
    if bound.binding != binding || bound.projection.legal_follow_graph.projection_ref != legal_follow.projection_ref {
        return Err("graph binding/projection changed on reopen".into());
    }

    let mut visibility = BTreeSet::new();
    visibility.extend(source_revision_refs.iter().cloned());
    visibility.extend(bound.projection.source_refs.iter().cloned());

    let receipt = Receipt {
        schema: RECEIPT_SCHEMA,
        matter_ref: request.matter_ref,
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
        persisted_pnf_grounding: true,
        rel_reopened_exactly: true,
        legal_follow_derived_only: bound.projection.legal_follow_graph.derived_only,
        inv_derived_from_rel_residual: queue.obligation.residual_obligation_ref == residual.obligation_ref,
        graph_binding_reopened_exactly: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
        creates_access_authority: false,
        acquisition_executed: false,
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
