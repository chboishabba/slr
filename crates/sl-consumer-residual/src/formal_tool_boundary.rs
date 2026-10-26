//! Governance firewall for formal MCP outputs.
//! Similarity, preflight, and interactive goals never constitute proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormalToolKind {
    Preflight,
    Similarity,
    ObligationRoute,
    InteractiveGoal,
    IsolatedKernelCheck,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalToolReceipt {
    pub kind: FormalToolKind,
    pub subject_commit: String,
    pub source_digest: String,
    pub artifact_digest: String,
    pub checker_digest: String,
    pub dependency_digest: String,
    pub evidence_digest: String,
    pub candidate_only: bool,
    pub establishes_proof: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertifiedHandoff<'a> {
    pub subject_commit: &'a str,
    pub source_digest: &'a str,
    pub artifact_digest: &'a str,
    pub checker_digest: &'a str,
    pub dependency_digest: &'a str,
    pub authenticated_local_referee: bool,
    pub independent_kernel_success: bool,
}
/// Classification of a handoff, not an authorization to merge or pay.
pub fn kernel_handoff_eligible(
    receipt: &FormalToolReceipt,
    expected: &CertifiedHandoff<'_>,
) -> bool {
    receipt.kind == FormalToolKind::IsolatedKernelCheck
        && receipt.subject_commit == expected.subject_commit
        && receipt.source_digest == expected.source_digest
        && receipt.artifact_digest == expected.artifact_digest
        && receipt.checker_digest == expected.checker_digest
        && receipt.dependency_digest == expected.dependency_digest
        && !receipt.subject_commit.is_empty()
        && !receipt.source_digest.is_empty()
        && !receipt.artifact_digest.is_empty()
        && !receipt.checker_digest.is_empty()
        && !receipt.dependency_digest.is_empty()
        && !receipt.evidence_digest.is_empty()
        && expected.authenticated_local_referee
        && expected.independent_kernel_success
        && !receipt.candidate_only
        && receipt.establishes_proof
}
#[cfg(test)]
mod tests {
    use super::*;
    fn receipt(kind: FormalToolKind) -> FormalToolReceipt {
        FormalToolReceipt {
            kind, subject_commit:"commit".into(), source_digest:"source".into(),
            artifact_digest:"artifact".into(), checker_digest:"checker".into(),
            dependency_digest:"deps".into(), evidence_digest:"evidence".into(),
            candidate_only:true, establishes_proof:false,
        }
    }
    fn expected() -> CertifiedHandoff<'static> {
        CertifiedHandoff {
            subject_commit:"commit", source_digest:"source",
            artifact_digest:"artifact", checker_digest:"checker",
            dependency_digest:"deps", authenticated_local_referee:true,
            independent_kernel_success:true,
        }
    }
    #[test]
    fn all_exploration_surfaces_fail_closed() {
        for kind in [FormalToolKind::Preflight, FormalToolKind::Similarity,
            FormalToolKind::ObligationRoute, FormalToolKind::InteractiveGoal] {
            let mut r = receipt(kind);
            r.candidate_only = false;
            r.establishes_proof = true; // even forged flags do not promote
            assert!(!kernel_handoff_eligible(&r, &expected()));
        }
    }
    #[test]
    fn isolated_kernel_requires_matching_bound_evidence() {
        let mut r = receipt(FormalToolKind::IsolatedKernelCheck);
        assert!(!kernel_handoff_eligible(&r, &expected()));
        r.candidate_only = false;
        r.establishes_proof = true;
        assert!(kernel_handoff_eligible(&r, &expected()));
        r.dependency_digest = "stale".into();
        assert!(!kernel_handoff_eligible(&r, &expected()));
    }
    #[test]
    fn local_referee_must_authenticate() {
        let mut r = receipt(FormalToolKind::IsolatedKernelCheck);
        r.candidate_only = false;
        r.establishes_proof = true;
        let mut e = expected();
        e.authenticated_local_referee = false;
        assert!(!kernel_handoff_eligible(&r, &e));
        e.authenticated_local_referee = true;
        e.independent_kernel_success = false;
        assert!(!kernel_handoff_eligible(&r, &e));
    }
}
