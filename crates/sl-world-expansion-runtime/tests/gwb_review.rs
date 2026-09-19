use sensiblaw_world_expansion_runtime::gwb_ambiguity_campaign::{
    compile_gwb_ambiguity_frontier, GwbAmbiguityKind, GwbAmbiguityResidual,
    GwbInvestigationCandidate, GwbInvestigationKind,
};
use sensiblaw_world_expansion_runtime::gwb_review::{
    parse_gwb_review_tsv, pending_gwb_review_bundle, prepare_reviewed_gwb_hop,
    GwbResidualEffect, GwbReviewOutcome, GwbReviewPreparationError,
};

fn compiled() -> sensiblaw_world_expansion_runtime::gwb_ambiguity_campaign::GwbCompiledAmbiguityFrontier {
    let residual = GwbAmbiguityResidual {
        residual_ref: "residual:gwb:Q207:type".into(),
        subject_ref: "Q207".into(),
        proposition_ref: "gwb:ambiguity:Q207:type".into(),
        kind: GwbAmbiguityKind::TypeClass,
        root_qid: Some("Q207".into()),
        salience: 10,
        dependency_refs: vec!["dep:gwb:Q207".into()],
    };
    let candidate = GwbInvestigationCandidate::governed_query(
        residual.residual_ref.clone(),
        "move:gwb:test",
        GwbInvestigationKind::TypeClass,
        "producer:wikidata-classification",
        Some("Q207".into()),
        Some("Q5".into()),
        "wikidata:P31",
        1,
        5,
        1,
        0,
        1,
    );
    compile_gwb_ambiguity_frontier(&[residual], &[candidate], "frontier:gwb:0")
}

#[test]
fn pending_bundle_is_reviewable_but_cannot_self_authorize() {
    let compiled = compiled();
    let selected = compiled.investigations.get("move:gwb:test").unwrap();
    let bundle = pending_gwb_review_bundle(
        0,
        &compiled,
        selected,
        "wikidata:Q207:oldid:123",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .unwrap();

    assert!(bundle.contains("# status=GwbReviewRequired"));
    assert!(bundle.contains("# selected_move=move:gwb:test"));
    assert!(bundle.contains("# review_manifest_template\t"));
    assert!(parse_gwb_review_tsv(&bundle).unwrap().is_empty());
}

#[test]
fn exact_review_assignment_binds_frontier_move_and_source_evidence() {
    let compiled = compiled();
    let selected = compiled.investigations.get("move:gwb:test").unwrap();
    let bundle = pending_gwb_review_bundle(
        3,
        &compiled,
        selected,
        "wikidata:Q207:oldid:123",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .unwrap();
    let template = bundle
        .lines()
        .find(|line| line.starts_with("# review_manifest_template\t"))
        .unwrap()
        .trim_start_matches("# review_manifest_template\t")
        .replace("<outcome>", "new-conceptual-parent")
        .replace("<residual-effect>", "keep-open")
        .replace("<review-ref>", "review:gwb:3");

    let reviews = parse_gwb_review_tsv(&template).unwrap();
    assert_eq!(reviews.len(), 1);
    let prepared = prepare_reviewed_gwb_hop(
        3,
        &compiled,
        selected,
        "wikidata:Q207:oldid:123",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        &reviews[0],
    )
    .unwrap();

    assert_eq!(prepared.outcome, GwbReviewOutcome::NewConceptualParent);
    assert_eq!(prepared.residual_effect, GwbResidualEffect::KeepOpen);
    assert!(prepared.candidate_only);
    assert!(!prepared.creates_semantic_authority);
    assert!(!prepared.claim_truth_promoted);
}

#[test]
fn negative_outcome_cannot_close_residual() {
    let compiled = compiled();
    let selected = compiled.investigations.get("move:gwb:test").unwrap();
    let bundle = pending_gwb_review_bundle(
        4,
        &compiled,
        selected,
        "wikidata:Q207:oldid:123",
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    )
    .unwrap();
    let line = bundle
        .lines()
        .find(|line| line.starts_with("# review_manifest_template\t"))
        .unwrap()
        .trim_start_matches("# review_manifest_template\t")
        .replace("<outcome>", "wrong-type")
        .replace("<residual-effect>", "close-reviewed")
        .replace("<review-ref>", "review:gwb:4");
    let assignment = parse_gwb_review_tsv(&line).unwrap().pop().unwrap();

    assert!(matches!(
        prepare_reviewed_gwb_hop(
            4,
            &compiled,
            selected,
            "wikidata:Q207:oldid:123",
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            &assignment,
        ),
        Err(GwbReviewPreparationError::NegativeOutcomeMayNotCloseResidual)
    ));
}
