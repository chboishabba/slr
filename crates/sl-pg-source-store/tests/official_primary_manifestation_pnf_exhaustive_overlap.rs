use sensiblaw_pg_source_store::{
    measure_semantic_pnf_coverage, semantic_pnf_coverage_is_consistent,
    CandidatePnfFactor, CandidatePnfRole,
};

fn factor(reference: &str, role: CandidatePnfRole, lemma: &str, dependency: &str) -> CandidatePnfFactor {
    CandidatePnfFactor {
        candidate_ref: reference.into(),
        role,
        source_start_char: 0,
        source_end_char: 1,
        surface: lemma.into(),
        lemma: lemma.into(),
        dependency_ref: dependency.into(),
        candidate_only: true,
    }
}

#[test]
fn full_semantic_multiset_match_is_exhaustive_despite_parser_debris() {
    let pdf = vec![
        factor("p1", CandidatePnfRole::Predicate, "include", "ROOT"),
        factor("p2", CandidatePnfRole::Predicate, "assist", "relcl"),
        factor("a1", CandidatePnfRole::Actor, "technology", "nsubj"),
        factor("q1", CandidatePnfRole::Qualifier, "directly", "advmod"),
        factor("junk1", CandidatePnfRole::Other, " ", "dep"),
        factor("junk2", CandidatePnfRole::Other, "\n", "dep"),
    ];
    let docx = vec![
        factor("d1", CandidatePnfRole::Predicate, "assist", "relcl"),
        factor("d2", CandidatePnfRole::Qualifier, "directly", "advmod"),
        factor("d3", CandidatePnfRole::Actor, "technology", "nsubj"),
        factor("d4", CandidatePnfRole::Predicate, "include", "ROOT"),
    ];

    let coverage = measure_semantic_pnf_coverage(&pdf, &docx);
    assert_eq!(coverage.pdf_parser_factor_count, 6);
    assert_eq!(coverage.docx_parser_factor_count, 4);
    assert_eq!(coverage.pdf_semantic_factor_count, 4);
    assert_eq!(coverage.docx_semantic_factor_count, 4);
    assert_eq!(coverage.matched_semantic_factor_count, 4);
    assert_eq!(coverage.unmatched_pdf_semantic_count, 0);
    assert_eq!(coverage.unmatched_docx_semantic_count, 0);
    assert_eq!(coverage.ignored_pdf_nonsemantic_count, 2);
    assert_eq!(coverage.ignored_docx_nonsemantic_count, 0);
    assert!(coverage.semantic_factor_multiset_exact);
    assert!(coverage.closes_source_transformation_residual);
    assert!(semantic_pnf_coverage_is_consistent(&coverage));
}

#[test]
fn one_missing_semantic_factor_keeps_transformation_residual_open() {
    let pdf = vec![
        factor("p1", CandidatePnfRole::Predicate, "include", "ROOT"),
        factor("p2", CandidatePnfRole::Predicate, "assist", "relcl"),
    ];
    let docx = vec![factor("d1", CandidatePnfRole::Predicate, "include", "ROOT")];

    let coverage = measure_semantic_pnf_coverage(&pdf, &docx);
    assert_eq!(coverage.matched_semantic_factor_count, 1);
    assert_eq!(coverage.unmatched_pdf_semantic_count, 1);
    assert_eq!(coverage.unmatched_docx_semantic_count, 0);
    assert!(!coverage.semantic_factor_multiset_exact);
    assert!(!coverage.closes_source_transformation_residual);
    assert!(semantic_pnf_coverage_is_consistent(&coverage));
}

#[test]
fn multiplicity_is_semantic_not_set_membership() {
    let pdf = vec![
        factor("p1", CandidatePnfRole::Qualifier, "not", "neg"),
        factor("p2", CandidatePnfRole::Qualifier, "not", "neg"),
    ];
    let docx = vec![factor("d1", CandidatePnfRole::Qualifier, "not", "neg")];

    let coverage = measure_semantic_pnf_coverage(&pdf, &docx);
    assert_eq!(coverage.matched_semantic_factor_count, 1);
    assert_eq!(coverage.unmatched_pdf_semantic_count, 1);
    assert!(!coverage.semantic_factor_multiset_exact);
    assert!(semantic_pnf_coverage_is_consistent(&coverage));
}

#[test]
fn corrupted_exact_flag_with_unmatched_semantic_factor_fails_replay_validation() {
    let pdf = vec![
        factor("p1", CandidatePnfRole::Predicate, "include", "ROOT"),
        factor("p2", CandidatePnfRole::Predicate, "assist", "relcl"),
    ];
    let docx = vec![factor("d1", CandidatePnfRole::Predicate, "include", "ROOT")];
    let mut coverage = measure_semantic_pnf_coverage(&pdf, &docx);

    coverage.semantic_factor_multiset_exact = true;
    coverage.closes_source_transformation_residual = true;
    coverage.requires_human_pnf_review = false;

    assert!(!semantic_pnf_coverage_is_consistent(&coverage));
}
