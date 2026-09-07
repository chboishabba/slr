use sensiblaw_governed_legal_provider::{
    CitationTreatmentIntent, KnownAuthorityDemand, PropositionUseIntent,
};
use sensiblaw_proof_search_loop::bound_acquisition::bind_authority_demand;
use sensiblaw_proof_search_loop::frontier::{ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::hypothesis::{family_for_residual, SearchHypothesisKind};

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

    println!(
        "residual={} proposition={} producer={} hypothesis={} source={} authority={} semantic_payment=false consumer_closed=false",
        bound.residual_ref,
        bound.proposition_ref,
        bound.scheduled_producer_ref,
        bound.hypothesis_ref,
        bound.demand.source_identity_ref,
        bound.binding_authority,
    );
}
