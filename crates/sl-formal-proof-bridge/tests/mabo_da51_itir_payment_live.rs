use sensiblaw_formal_proof_bridge::{
    persist_and_validate_formal_evidence, FormalEvidenceReceiptInput,
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

    let payment = persist_and_validate_formal_evidence(&database_url, &evidence)
        .expect("exact reviewed demand and independently certified evidence must replay/pay");

    assert_eq!(payment.demand_ref, evidence.demand_ref);
    assert_eq!(payment.obligation_ref, evidence.obligation_ref);
    assert_eq!(payment.evidence_ref, evidence.evidence_ref);
    assert_eq!(payment.statement_digest, evidence.statement_digest);
    assert!(payment.formal_premise_paid);
    assert!(!payment.creates_legal_authority);
    assert!(!payment.applicability_promoted);
    assert!(!payment.claim_truth_promoted);
}
