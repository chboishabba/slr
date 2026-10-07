use sensiblaw_pg_source_store::{
    load_database_config, load_matter_controversy, project_matter_personas,
    project_matter_reverse_proof_search, MatterProceduralGoal,
};

#[test]
#[ignore = "requires genuine provider-backed OALC Mabo rows and explicit human legal review"]
fn live_oalc_mabo_forms_reviewed_adversarial_controversy_without_merits_promotion() {
    let controversy_ref = std::env::var("ITIR_REAL_MABO_CONTROVERSY_REF")
        .expect("ITIR_REAL_MABO_CONTROVERSY_REF must identify the persisted real Mabo controversy");
    let config = load_database_config(None).expect("DATABASE_URL must identify the live PG store");
    let persisted = load_matter_controversy(&config, &controversy_ref)
        .expect("real Mabo controversy must reopen exactly");
    let matter = &persisted.controversy;

    assert!(matter.propositions.len() >= 2, "miniature controversy needs competing propositions");
    assert!(!matter.responses.is_empty(), "miniature controversy needs a typed response");
    assert!(!matter.residuals.is_empty(), "miniature controversy needs a genuine unresolved residual");
    assert!(matter
        .propositions
        .iter()
        .any(|p| p.reviewed_evidence_ref.is_some()), "at least one proposition must cross the human review gate");
    assert!(matter
        .propositions
        .iter()
        .all(|p| !p.source_ref.to_ascii_lowercase().contains("wikisource")),
        "the OALC specimen must not relabel the historical Wikisource source revision");
    assert!(matter
        .residuals
        .iter()
        .any(|r| !r.target_evidence_query.trim().is_empty()));

    let reverse = project_matter_reverse_proof_search(
        matter,
        MatterProceduralGoal::DecideEvidenceNeeded,
    )
    .expect("real Mabo controversy must support backward proof search");
    assert!(!reverse.target_evidence_queries.is_empty());
    assert!(!reverse.creates_actual_reopening);
    assert!(!reverse.creates_access_authority);

    let personas = project_matter_personas(matter, &reverse)
        .expect("same persisted controversy must project for all operators");
    assert_eq!(personas.client.controversy_ref, controversy_ref);
    assert_eq!(personas.solicitor.controversy_ref, controversy_ref);
    assert_eq!(personas.court.controversy_ref, controversy_ref);
    assert!(!personas.court.determines_credibility);
    assert!(!personas.court.determines_ultimate_fact);
    assert!(!personas.court.assigns_normative_weight);
    assert!(!personas.court.enters_final_judgment);
    assert!(!personas.creates_semantic_authority);
    assert!(!personas.claim_truth_promoted);
    assert!(!personas.canonical_world_mutated);
}
