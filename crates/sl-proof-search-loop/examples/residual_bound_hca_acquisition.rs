use sensiblaw_governed_legal_provider::{
    CitationTreatmentIntent, KnownAuthorityDemand, PropositionUseIntent,
};
use sensiblaw_proof_search_loop::bound_acquisition::bind_authority_demand;
use sensiblaw_proof_search_loop::frontier::{ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::hypothesis::{family_for_residual, SearchHypothesisKind};
use std::fs;
use std::path::PathBuf;

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn main() {
    let residual = ProofResidual {
        residual_ref: "residual:cullen-positive-operational-act".into(),
        proposition_ref: "prop:cullen-positive-operational-duty".into(),
        producer_class_ref: "producer:exact-primary-authority".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: Some("official-primary-case".into()),
        salience: 10,
        dependency_refs: vec!["source:[2026]-HCA-19".into()],
        status: ResidualStatus::Open,
    };
    let hypothesis = family_for_residual(&residual)
        .into_iter()
        .find(|hypothesis| hypothesis.kind == SearchHypothesisKind::Support)
        .expect("open residual gets support hypothesis");
    let demand = KnownAuthorityDemand {
        demand_ref: "demand:cullen-official-source".into(),
        jurisdiction_ref: "AU".into(),
        source_identity_ref: "case:[2026]-HCA-19".into(),
        medium_neutral_citation: Some("[2026] HCA 19".into()),
        explicit_austlii_ref: None,
        proposition_ref: Some("prop:cullen-positive-operational-duty".into()),
        use_intent: PropositionUseIntent::SourceProposition,
        treatment_intent: CitationTreatmentIntent::None,
    };

    let bound = bind_authority_demand(&residual, &hypothesis, demand)
        .expect("live source demand must match exact open residual");

    assert!(bound.source_route_pays_scheduled_gap());
    assert!(bound.source_route_uses_scheduled_producer());
    assert!(!bound.acquisition_is_semantic_payment());
    assert!(!bound.acquisition_closes_consumer());

    let output_path = PathBuf::from(
        std::env::var("SENSIBLAW_BOUND_ACQUISITION_PLAN")
            .unwrap_or_else(|_| "/tmp/sensiblaw-live-legal/residual-bound-acquisition-v01.json".into()),
    );
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("create bound acquisition permit directory");
    }
    let citation = bound
        .demand
        .medium_neutral_citation
        .as_deref()
        .expect("bounded HCA fixture carries exact MNC");
    let permit = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"sl.residual_bound_authority_demand.v0_1\",\n",
            "  \"authority\": \"experimental_candidate_only\",\n",
            "  \"residual_ref\": \"{}\",\n",
            "  \"proposition_ref\": \"{}\",\n",
            "  \"scheduled_producer_ref\": \"{}\",\n",
            "  \"hypothesis_ref\": \"{}\",\n",
            "  \"jurisdiction_ref\": \"AU\",\n",
            "  \"source_identity_ref\": \"{}\",\n",
            "  \"medium_neutral_citation\": \"{}\",\n",
            "  \"source_route_pays_scheduled_gap\": true,\n",
            "  \"source_route_uses_scheduled_producer\": true,\n",
            "  \"acquisition_claimed_semantic_payment\": false,\n",
            "  \"acquisition_claimed_consumer_closure\": false\n",
            "}}\n"
        ),
        json_escape(&bound.residual_ref),
        json_escape(&bound.proposition_ref),
        json_escape(&bound.scheduled_producer_ref),
        json_escape(&bound.hypothesis_ref),
        json_escape(&bound.demand.source_identity_ref),
        json_escape(citation),
    );
    fs::write(&output_path, permit).expect("write deterministic residual-bound acquisition permit");

    println!(
        "permit={} residual={} proposition={} producer={} hypothesis={} source={} authority={} semantic_payment=false consumer_closed=false",
        output_path.display(),
        bound.residual_ref,
        bound.proposition_ref,
        bound.scheduled_producer_ref,
        bound.hypothesis_ref,
        bound.demand.source_identity_ref,
        bound.binding_authority,
    );
}
