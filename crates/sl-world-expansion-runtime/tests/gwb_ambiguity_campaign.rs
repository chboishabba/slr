use sensiblaw_route_selector::{ProducerFamily, RouteCandidate, RouteFamily};
use sensiblaw_world_expansion_runtime::gwb_ambiguity_campaign::{
    compile_gwb_ambiguity_frontier, compile_gwb_question_frontier,
    gwb_question_investigations, route_candidate_to_investigation,
    select_gwb_investigation, GwbAmbiguityKind, GwbAmbiguityResidual,
    GwbInvestigationKind,
};

fn residual(kind: GwbAmbiguityKind) -> GwbAmbiguityResidual {
    GwbAmbiguityResidual {
        residual_ref: "residual:gwb:Q207:type".into(),
        subject_ref: "Q207".into(),
        proposition_ref: "gwb:ambiguity:Q207:type".into(),
        kind,
        root_qid: Some("Q207".into()),
        salience: 10,
        dependency_refs: vec!["dep:gwb:Q207".into()],
    }
}

fn route(property: &str, target: &str, specificity: u32) -> RouteCandidate {
    RouteCandidate {
        candidate_id: format!("wikidata:Q207:{property}:{target}"),
        producer: ProducerFamily::ClassificationEvidence,
        route_family: RouteFamily::WikidataProperty,
        source_ref: "Q207".into(),
        target_ref: target.into(),
        property_ref: property.into(),
        cross_language_gap_coverage: 0,
        source_surface_support: 0,
        root_qid_support: 1,
        typed_property_support: 1,
        route_specificity: specificity,
        yield_history_observed: 0,
        prior_contracted_old_gaps: 0,
        prior_retired_obligations: 0,
        prior_new_gap_atoms: 0,
        prior_network_requests: 0,
    }
}

#[test]
fn p31_and_p279_are_discriminators_not_mandatory_traversal_steps() {
    let r = residual(GwbAmbiguityKind::TypeClass);
    let p31 = route_candidate_to_investigation(
        &r,
        &route("P31", "Q5", 5),
        "wikidata:Q207:oldid:123",
    )
    .unwrap();
    let p279 = route_candidate_to_investigation(
        &r,
        &route("P279", "Q215627", 5),
        "wikidata:Q207:oldid:123",
    )
    .unwrap();

    assert_eq!(p31.investigation_kind, GwbInvestigationKind::TypeClass);
    assert_eq!(p279.investigation_kind, GwbInvestigationKind::Superclass);
    assert!(p31.candidate_only);
    assert!(!p31.creates_semantic_authority);
    assert!(!p31.claim_truth_promoted);
}

#[test]
fn specificity_can_dominate_generic_property_without_scalar_ambiguity_score() {
    let r = residual(GwbAmbiguityKind::TypeClass);
    let p31 = route_candidate_to_investigation(
        &r,
        &route("P31", "Q5", 5),
        "wikidata:Q207:oldid:123",
    )
    .unwrap();
    let generic = route_candidate_to_investigation(
        &r,
        &route("P361", "Q30", 4),
        "wikidata:Q207:oldid:123",
    )
    .unwrap();

    let compiled = compile_gwb_ambiguity_frontier(
        &[r],
        &[generic, p31],
        "frontier:gwb:test",
    );
    let selected = select_gwb_investigation(&compiled).unwrap();
    assert_eq!(selected.property_ref.as_deref(), Some("P31"));
    assert!(!compiled.pareto_dimensions_scalarized);
    assert!(!compiled.frontier_rank_is_truth_rank);
}

#[test]
fn multilingual_surface_and_type_questions_can_coexist_on_one_pareto_frontier() {
    let type_residual = residual(GwbAmbiguityKind::TypeClass);
    let surface_residual = GwbAmbiguityResidual {
        residual_ref: "residual:gwb:Q207:surface:es".into(),
        subject_ref: "Q207".into(),
        proposition_ref: "gwb:surface-gap:Q207:es".into(),
        kind: GwbAmbiguityKind::CrossLanguageGap,
        root_qid: Some("Q207".into()),
        salience: 8,
        dependency_refs: vec!["dep:gwb:Q207:surface".into()],
    };
    let type_move = route_candidate_to_investigation(
        &type_residual,
        &route("P31", "Q5", 5),
        "wikidata:Q207:oldid:123",
    )
    .unwrap();
    let article = RouteCandidate {
        candidate_id: "wikipedia-article:Q207:https://es.wikipedia.org/wiki/George_W._Bush".into(),
        producer: ProducerFamily::ArticleSemantic,
        route_family: RouteFamily::WikipediaArticle,
        source_ref: "Q207".into(),
        target_ref: "https://es.wikipedia.org/wiki/George_W._Bush".into(),
        property_ref: String::new(),
        cross_language_gap_coverage: 1,
        source_surface_support: 1,
        root_qid_support: 1,
        typed_property_support: 0,
        route_specificity: 4,
        yield_history_observed: 0,
        prior_contracted_old_gaps: 0,
        prior_retired_obligations: 0,
        prior_new_gap_atoms: 0,
        prior_network_requests: 0,
    };
    let surface_move = route_candidate_to_investigation(
        &surface_residual,
        &article,
        "wikidata:Q207:oldid:123",
    )
    .unwrap();

    let compiled = compile_gwb_ambiguity_frontier(
        &[type_residual, surface_residual],
        &[type_move, surface_move],
        "frontier:gwb:mixed",
    );

    assert_eq!(compiled.frontier.residuals.len(), 2);
    assert_eq!(compiled.candidates.len(), 2);
    assert!(compiled
        .investigations
        .values()
        .any(|move_| move_.investigation_kind == GwbInvestigationKind::CrossLanguageSurface));
    assert!(compiled
        .investigations
        .values()
        .any(|move_| move_.investigation_kind == GwbInvestigationKind::TypeClass));
}


#[test]
fn current_world_questions_are_selected_before_any_provider_observation_exists() {
    let r = residual(GwbAmbiguityKind::TypeClass);
    let compiled = compile_gwb_question_frontier(
        &[r],
        &std::collections::BTreeSet::new(),
        "frontier:gwb:question-first",
    );
    let selected = select_gwb_investigation(&compiled).unwrap();

    assert_eq!(selected.move_ref, "move:gwb:inspect:Q207:classification");
    assert!(selected.source_revision_ref.is_none());
    assert_eq!(selected.source_ref.as_deref(), Some("Q207"));
    assert!(selected.candidate_only);
    assert!(!selected.creates_semantic_authority);
}

#[test]
#[allow(clippy::cloned_ref_to_slice_refs)]
fn reviewed_narrow_question_falls_through_to_external_ontology_then_snowball() {
    let r = residual(GwbAmbiguityKind::TypeClass);
    let classification = "move:gwb:inspect:Q207:classification".to_string();
    let reviewed = std::collections::BTreeSet::from([classification.clone()]);
    let after_classification = gwb_question_investigations(&[r.clone()], &reviewed);
    assert_eq!(after_classification.len(), 1);
    assert_eq!(
        after_classification[0].investigation_kind,
        GwbInvestigationKind::ExternalOntologyFallback
    );

    let external = after_classification[0].move_ref.clone();
    let reviewed = std::collections::BTreeSet::from([classification, external]);
    let after_external = gwb_question_investigations(&[r], &reviewed);
    assert_eq!(after_external.len(), 1);
    assert_eq!(
        after_external[0].investigation_kind,
        GwbInvestigationKind::Snowball
    );
}
