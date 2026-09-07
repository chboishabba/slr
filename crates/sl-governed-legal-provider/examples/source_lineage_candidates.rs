use sensiblaw_governed_legal_provider::{
    citation_traversal_plan, resolve_known_authority, CitationTreatmentIntent,
    KnownAuthorityDemand, PropositionUseIntent, ResolutionContext, ResolutionStage,
};

fn demand(
    demand_ref: &str,
    source_identity_ref: &str,
    mnc: &str,
    proposition_ref: &str,
    treatment_intent: CitationTreatmentIntent,
) -> KnownAuthorityDemand {
    KnownAuthorityDemand {
        demand_ref: demand_ref.into(),
        jurisdiction_ref: "AU".into(),
        source_identity_ref: source_identity_ref.into(),
        medium_neutral_citation: Some(mnc.into()),
        explicit_austlii_ref: None,
        proposition_ref: Some(proposition_ref.into()),
        use_intent: PropositionUseIntent::CitationTreatment,
        treatment_intent,
    }
}

fn main() {
    let context = ResolutionContext::default();

    let mallonland = demand(
        "duty:mallonland:treatment",
        "case:Mallonland-v-Advanta-Seeds-2024-HCA-25",
        "[2024] HCA 25",
        "prop:mallonland-salient-features-constraint",
        CitationTreatmentIntent::CitedBy,
    );
    let cullen = demand(
        "duty:cullen:treatment",
        "case:Cullen-v-State-of-Queensland-2026-HCA-19",
        "[2026] HCA 19",
        "prop:cullen-positive-operational-duty",
        CitationTreatmentIntent::CitedBy,
    );
    let pabai = demand(
        "duty:pabai:treatment",
        "case:Pabai-v-Commonwealth-2025-FCA-796",
        "[2025] FCA 796",
        "prop:pabai-current-climate-policy-obstruction",
        CitationTreatmentIntent::CitedBy,
    );

    for item in [&mallonland, &cullen, &pabai] {
        let resolution = resolve_known_authority(item, &context);
        assert!(matches!(resolution.stage, ResolutionStage::JadeExactMnc { .. }));
        assert!(!resolution.search_is_semantic_payment());
        assert!(!resolution.acquisition_is_authority_receipt());

        let traversal = citation_traversal_plan(item).expect("treatment intent should plan traversal");
        assert_eq!(traversal.proposition_ref, item.proposition_ref);
        assert_eq!(traversal.bounds.max_depth, 1);
        assert_eq!(traversal.bounds.max_new_documents, 5);
    }

    assert_ne!(mallonland.source_identity_ref, cullen.source_identity_ref);
    assert_ne!(cullen.source_identity_ref, pabai.source_identity_ref);
    assert_ne!(mallonland.proposition_ref, cullen.proposition_ref);
    assert_ne!(cullen.proposition_ref, pabai.proposition_ref);
}
