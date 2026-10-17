use sensiblaw_pg_source_store::{
    validate_formal_evidence_payment, FormalEvidenceReceiptInput, FormalProofBridgeError,
    FormalProofDemand, FormalProofRouteReceiptInput,
};

fn demand() -> FormalProofDemand {
    FormalProofDemand {
        demand_ref: "formal-demand:smith:1".into(),
        obligation_ref: "proof-obligation:smith:1".into(),
        consumer_ref: "consumer:smith:construction".into(),
        pnf_revision_ref: "pnf-revision:smith:1".into(),
        residual_ref: "residual:smith:construction".into(),
        proposition_ref: Some("proposition:smith:construction".into()),
        statement_digest: "sha256:statement-a".into(),
        statement_language: "lean4".into(),
        target_prover: "lean".into(),
        da51_address: Some("0xDA51831100800000".into()),
        source_revision_refs: vec!["source-revision:smith:hca37".into()],
        requested_result_kind: "formal-proof".into(),
        dependent_refs: vec!["residual:smith:dependent".into()],
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn evidence() -> FormalEvidenceReceiptInput {
    FormalEvidenceReceiptInput {
        evidence_ref: "formal-evidence:smith:1".into(),
        demand_ref: "formal-demand:smith:1".into(),
        obligation_ref: "proof-obligation:smith:1".into(),
        da51_address: Some("0xDA51831100800000".into()),
        producer: "lean-kernel".into(),
        artifact_ref: "artifact:smith-proof:1".into(),
        artifact_digest: "sha256:artifact-a".into(),
        statement_digest: "sha256:statement-a".into(),
        checker: "lean".into(),
        checker_revision: "lean:exact-head".into(),
        source_revision: "source-revision:smith:hca37".into(),
        result_kind: "formal-proof".into(),
        certification_state: "kernel-certified".into(),
        dependencies: vec![],
        establishes_formal_premise: true,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn formal_demand_requires_exact_identity_coordinates() {
    let mut d = demand();
    d.obligation_ref.clear();
    assert!(matches!(
        d.validate(),
        Err(FormalProofBridgeError::EmptyCoordinate("obligation_ref"))
    ));
}

#[test]
fn formal_evidence_wrong_obligation_fails_closed() {
    let d = demand();
    let mut e = evidence();
    e.obligation_ref = "proof-obligation:other".into();
    assert!(matches!(
        validate_formal_evidence_payment(&d, &e),
        Err(FormalProofBridgeError::ObligationMismatch)
    ));
}

#[test]
fn formal_evidence_wrong_statement_digest_fails_closed() {
    let d = demand();
    let mut e = evidence();
    e.statement_digest = "sha256:statement-b".into();
    assert!(matches!(
        validate_formal_evidence_payment(&d, &e),
        Err(FormalProofBridgeError::StatementDigestMismatch)
    ));
}

#[test]
fn route_receipt_cannot_pay_formal_obligation() {
    let d = demand();
    let route = FormalProofRouteReceiptInput {
        route_ref: "route:smith:1".into(),
        demand_ref: d.demand_ref,
        obligation_ref: d.obligation_ref,
        statement_digest: d.statement_digest,
        establishes_proof: false,
    };
    assert!(!route.can_pay_formal_obligation());
}

#[test]
fn formal_evidence_payment_reopens_only_declared_dependents() {
    let d = demand();
    let e = evidence();
    let payment = validate_formal_evidence_payment(&d, &e).unwrap();
    assert_eq!(payment.reopen_refs, vec!["residual:smith:dependent"]);
    assert!(payment.formal_premise_paid);
    assert!(!payment.creates_legal_authority);
    assert!(!payment.applicability_promoted);
    assert!(!payment.claim_truth_promoted);
}
