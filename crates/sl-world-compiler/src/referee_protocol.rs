//! Commit-bound referee evidence. No actor-generated receipt self-certifies.
use sha2::{Digest, Sha256};

pub const REFEREE_RECEIPT_VERSION: &str = "slr.referee-verification.v1";
pub const CRITIQUE_VERSION: &str = "slr.referee-critique.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome { Passed, Failed, TimedOut, NotRun }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefereeVerificationReceipt {
    pub subject_commit: String,
    pub check_id: String,
    pub check_version: String,
    pub environment_digest: [u8; 32],
    pub dependency_digest: [u8; 32],
    pub diagnostic_digest: [u8; 32],
    pub verifier_principal: String,
    pub actor_principal: String,
    pub outcome: CheckOutcome,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionRequest<'a> {
    pub current_commit: &'a str,
    pub check_id: &'a str,
    pub check_version: &'a str,
    pub environment_digest: [u8; 32],
    pub dependency_digest: [u8; 32],
    pub trusted_verifier: &'a str,
    pub authenticated_receipt: bool,
}

/// This is a single-check eligibility gate, not a merge authorization.
/// Authentication must be established by the local referee, not this struct.
pub fn check_eligible(receipt: &RefereeVerificationReceipt, request: &PromotionRequest<'_>) -> bool {
    valid_git_sha(&receipt.subject_commit)
        && receipt.subject_commit == request.current_commit
        && !receipt.check_id.is_empty()
        && receipt.check_id == request.check_id
        && receipt.check_version == request.check_version
        && receipt.environment_digest == request.environment_digest
        && receipt.dependency_digest == request.dependency_digest
        && request.authenticated_receipt
        && !receipt.verifier_principal.is_empty()
        && receipt.verifier_principal == request.trusted_verifier
        && receipt.verifier_principal != receipt.actor_principal
        && receipt.outcome == CheckOutcome::Passed
        && receipt.candidate_only
        && !receipt.creates_semantic_authority
        && !receipt.creates_claim_truth
}
pub fn valid_git_sha(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefereeCritiqueResidual {
    pub protocol: &'static str,
    pub subject_commit: String,
    pub check_id: String,
    pub failure_class: String,
    pub path: String,
    pub symbol: String,
    pub expected: String,
    pub observed: String,
    pub diagnostic_digest: [u8; 32],
    pub required_action: &'static str,
}
impl RefereeCritiqueResidual {
    /// Fixed-field, length-prefixed encoding avoids JSON key-order ambiguity.
    pub fn deterministic_digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        for field in [
            self.protocol, &self.subject_commit, &self.check_id,
            &self.failure_class, &self.path, &self.symbol,
            &self.expected, &self.observed, self.required_action,
        ] {
            hash.update((field.len() as u64).to_le_bytes());
            hash.update(field.as_bytes());
        }
        hash.update(self.diagnostic_digest);
        hash.finalize().into()
    }
}
pub fn critique_for_failure(
    receipt: &RefereeVerificationReceipt,
    failure_class: &str,
    path: &str,
    symbol: &str,
    expected: &str,
    observed: &str,
) -> Option<RefereeCritiqueResidual> {
    if receipt.outcome == CheckOutcome::Passed { return None; }
    Some(RefereeCritiqueResidual {
        protocol: CRITIQUE_VERSION, subject_commit: receipt.subject_commit.clone(),
        check_id: receipt.check_id.clone(), failure_class: failure_class.into(),
        path: path.into(), symbol: symbol.into(), expected: expected.into(),
        observed: observed.into(), diagnostic_digest: receipt.diagnostic_digest,
        required_action: "repair-and-resubmit",
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (RefereeVerificationReceipt, PromotionRequest<'static>) {
        let commit = "a".repeat(40);
        let receipt = RefereeVerificationReceipt {
            subject_commit: commit, check_id: "cargo-test".into(), check_version: "1".into(),
            environment_digest: [1;32], dependency_digest: [2;32],
            diagnostic_digest: [3;32], verifier_principal: "local-referee".into(),
            actor_principal: "remote-actor".into(), outcome: CheckOutcome::Passed,
            candidate_only: true, creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let request = PromotionRequest {
            current_commit: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            check_id: "cargo-test", check_version: "1",
            environment_digest: [1;32], dependency_digest: [2;32],
            trusted_verifier: "local-referee", authenticated_receipt: true,
        };
        (receipt, request)
    }
    #[test]
    fn valid_check_and_stale_commit_rejection() {
        let (receipt, mut request) = fixture();
        assert!(check_eligible(&receipt, &request));
        request.current_commit = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        assert!(!check_eligible(&receipt, &request));
    }
    #[test]
    fn digest_and_identity_fail_closed() {
        let (mut receipt, mut request) = fixture();
        request.dependency_digest = [4;32];
        assert!(!check_eligible(&receipt, &request));
        request.dependency_digest = [2;32];
        request.authenticated_receipt = false;
        assert!(!check_eligible(&receipt, &request));
        request.authenticated_receipt = true;
        receipt.verifier_principal = receipt.actor_principal.clone();
        assert!(!check_eligible(&receipt, &request));
    }
    #[test]
    fn failed_check_never_promotes_and_emits_deterministic_critique() {
        let (mut receipt, request) = fixture();
        receipt.outcome = CheckOutcome::Failed;
        assert!(!check_eligible(&receipt, &request));
        let a = critique_for_failure(&receipt, "binary-layout-mismatch",
            "crates/sl-world-compiler/src/protected_fleet.rs", "decode_pnf_slot", "32", "36").unwrap();
        let b = critique_for_failure(&receipt, "binary-layout-mismatch",
            "crates/sl-world-compiler/src/protected_fleet.rs", "decode_pnf_slot", "32", "36").unwrap();
        assert_eq!(a.deterministic_digest(), b.deterministic_digest());
        assert_eq!(a.protocol, "slr.referee-critique.v1");
        assert_ne!(a.deterministic_digest(), critique_for_failure(&receipt,
            "binary-layout-mismatch", "crates/sl-world-compiler/src/protected_fleet.rs",
            "decode_pnf_slot", "32", "37").unwrap().deterministic_digest());
    }
    #[test]
    fn candidate_or_authority_promotion_is_rejected() {
        let (mut receipt, request) = fixture();
        receipt.candidate_only = false;
        assert!(!check_eligible(&receipt, &request));
        receipt.candidate_only = true;
        receipt.creates_claim_truth = true;
        assert!(!check_eligible(&receipt, &request));
    }
}
