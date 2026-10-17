use sensiblaw_pg_source_store::{
    classify_manifestation_pnf_overlap, ComparisonFinding,
    ManifestationPnfOverlapDisposition,
};

#[test]
fn exact_or_licensed_candidate_shape_closes_only_the_transformation_residual() {
    for finding in [
        ComparisonFinding::ExactCandidateShape,
        ComparisonFinding::LicensedCompatible,
    ] {
        let result = classify_manifestation_pnf_overlap(finding);
        assert_eq!(result.disposition, ManifestationPnfOverlapDisposition::Exact);
        assert!(!result.requires_human_pnf_review);
        assert!(result.closes_source_transformation_residual);
        assert!(!result.creates_legal_controversy);
        assert!(!result.creates_semantic_authority);
        assert!(!result.creates_legal_authority);
        assert!(!result.applicability_promoted);
        assert!(!result.claim_truth_promoted);
        assert_eq!(result.independent_source_count_increment, 0);
    }
}

#[test]
fn partial_candidate_overlap_stays_reviewable_without_becoming_a_legal_dispute() {
    for finding in [
        ComparisonFinding::CompatibleQualification,
        ComparisonFinding::PartialResidual,
        ComparisonFinding::PolarityConflictCandidate,
    ] {
        let result = classify_manifestation_pnf_overlap(finding);
        assert_eq!(result.disposition, ManifestationPnfOverlapDisposition::Partial);
        assert!(result.requires_human_pnf_review);
        assert!(!result.closes_source_transformation_residual);
        assert!(!result.creates_legal_controversy);
        assert_eq!(result.independent_source_count_increment, 0);
    }
}

#[test]
fn missing_typed_meet_or_undetermined_remains_fail_closed() {
    for finding in [
        ComparisonFinding::MissingTypedMeet,
        ComparisonFinding::Undetermined,
    ] {
        let result = classify_manifestation_pnf_overlap(finding);
        assert_eq!(result.disposition, ManifestationPnfOverlapDisposition::NoTypedMeet);
        assert!(result.requires_human_pnf_review);
        assert!(!result.closes_source_transformation_residual);
        assert!(!result.creates_semantic_authority);
        assert!(!result.claim_truth_promoted);
    }
}
