use sensiblaw_formal_proof_bridge::{
    persist_authenticated_formal_evidence, reopen_authenticated_formal_evidence,
    FormalEvidenceReceiptInput, FormalProofBridgeError, RefereeReceiptLocator,
};

fn env_ref(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} must identify the already-produced Mabo/DA51/ITIR coordinate"))
}

fn optional_env_ref(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty() && value != "-")
}

fn dependency_refs() -> Vec<String> {
    std::env::var("ITIR_MABO_FORMAL_DEPENDENCY_REFS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[test]
#[ignore = "requires the reviewed Mabo SLR demand plus independently kernel-certified DA51/ITIR evidence"]
fn live_mabo_da51_itir_evidence_pays_only_the_exact_formal_premise() {
    let database_url = env_ref("DATABASE_URL");
    let evidence = FormalEvidenceReceiptInput {
        evidence_ref: env_ref("ITIR_MABO_FORMAL_EVIDENCE_REF"),
        demand_ref: env_ref("ITIR_MABO_FORMAL_DEMAND_REF"),
        obligation_ref: env_ref("ITIR_MABO_FORMAL_OBLIGATION_REF"),
        da51_address: optional_env_ref("ITIR_MABO_DA51_ADDRESS"),
        producer: env_ref("ITIR_MABO_FORMAL_PRODUCER"),
        artifact_ref: env_ref("ITIR_MABO_FORMAL_ARTIFACT_REF"),
        artifact_digest: env_ref("ITIR_MABO_FORMAL_ARTIFACT_DIGEST"),
        statement_digest: env_ref("ITIR_MABO_FORMAL_STATEMENT_DIGEST"),
        checker: env_ref("ITIR_MABO_FORMAL_CHECKER"),
        checker_revision: env_ref("ITIR_MABO_FORMAL_CHECKER_REVISION"),
        source_revision: env_ref("ITIR_MABO_FORMAL_SOURCE_REVISION"),
        result_kind: env_ref("ITIR_MABO_FORMAL_RESULT_KIND"),
        certification_state: env_ref("ITIR_MABO_FORMAL_CERTIFICATION_STATE"),
        dependencies: dependency_refs(),
        establishes_formal_premise: true,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };

    let locator = RefereeReceiptLocator {
        repo: env_ref("ITIR_MABO_REFEREE_REPO"),
        sha: env_ref("ITIR_MABO_REFEREE_SHA"),
        path: env_ref("ITIR_MABO_REFEREE_PATH"),
        canonical_thread_id: env_ref("ITIR_MABO_REFEREE_THREAD"),
    };
    let authenticated = persist_authenticated_formal_evidence(&database_url, &evidence, &locator)
        .expect("exact independently issued SLR premise binding must replay/pay");
    let reopened = reopen_authenticated_formal_evidence(&database_url, &evidence.evidence_ref, &locator)
        .expect("persisted payment must independently reopen its referee receipt");
    assert_eq!(authenticated, reopened);
    let payment = authenticated.payment();

    let mut changed = evidence.clone();
    changed.statement_digest.push_str(":foreign");
    assert_eq!(persist_authenticated_formal_evidence(&database_url, &changed, &locator),
        Err(FormalProofBridgeError::StatementDigestMismatch));
    let mut bad = locator.clone();
    bad.path = "../foreign.lean".into();
    assert_eq!(reopen_authenticated_formal_evidence(&database_url, &evidence.evidence_ref, &bad),
        Err(FormalProofBridgeError::UnauthenticatedRefereeReceipt));

    assert_eq!(payment.demand_ref, evidence.demand_ref);
    assert_eq!(payment.obligation_ref, evidence.obligation_ref);
    assert_eq!(payment.evidence_ref, evidence.evidence_ref);
    assert_eq!(payment.statement_digest, evidence.statement_digest);
    assert!(payment.formal_premise_paid);
    assert!(!payment.creates_legal_authority);
    assert!(!payment.applicability_promoted);
    assert!(!payment.claim_truth_promoted);
}
