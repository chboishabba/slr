use sensiblaw_formal_proof_bridge::{
    demand_replay_matches, evidence_replay_matches, payment_replay_matches, route_replay_matches,
    FormalEvidencePayment, FormalEvidenceReceiptInput, FormalProofDemand, FormalProofRouteReceiptInput,
};

fn demand() -> FormalProofDemand {
    FormalProofDemand {
        demand_ref: "demand:1".into(),
        obligation_ref: "obligation:1".into(),
        consumer_ref: "consumer:1".into(),
        pnf_revision_ref: "pnf:1".into(),
        residual_ref: "residual:1".into(),
        proposition_ref: Some("proposition:1".into()),
        statement_digest: "sha256:statement".into(),
        statement_language: "lean4".into(),
        target_prover: "lean".into(),
        da51_address: Some("0xDA51831100800000".into()),
        source_revision_refs: vec!["source:1".into()],
        requested_result_kind: "formal-proof".into(),
        dependent_refs: vec!["dependent:1".into()],
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn route() -> FormalProofRouteReceiptInput {
    FormalProofRouteReceiptInput {
        route_ref: "route:1".into(),
        demand_ref: "demand:1".into(),
        obligation_ref: "obligation:1".into(),
        statement_digest: "sha256:statement".into(),
        establishes_proof: false,
    }
}

fn evidence() -> FormalEvidenceReceiptInput {
    FormalEvidenceReceiptInput {
        evidence_ref: "evidence:1".into(),
        demand_ref: "demand:1".into(),
        obligation_ref: "obligation:1".into(),
        da51_address: Some("0xDA51831100800000".into()),
        producer: "producer:lean".into(),
        artifact_ref: "artifact:1".into(),
        artifact_digest: "sha256:artifact".into(),
        statement_digest: "sha256:statement".into(),
        checker: "lean-kernel".into(),
        checker_revision: "lean:exact".into(),
        source_revision: "source:1".into(),
        result_kind: "formal-proof".into(),
        certification_state: "kernel-certified".into(),
        dependencies: vec!["dep:a".into()],
        establishes_formal_premise: true,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn payment() -> FormalEvidencePayment {
    FormalEvidencePayment {
        demand_ref: "demand:1".into(),
        obligation_ref: "obligation:1".into(),
        evidence_ref: "evidence:1".into(),
        statement_digest: "sha256:statement".into(),
        formal_premise_paid: true,
        reopen_refs: vec!["dependent:1".into()],
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn demand_replay_checks_every_immutable_coordinate() {
    let original = demand();
    let mut changed = original.clone();
    changed.target_prover = "agda".into();
    assert!(!demand_replay_matches(&original, &changed));

    let mut changed = original.clone();
    changed.source_revision_refs = vec!["source:other".into()];
    assert!(!demand_replay_matches(&original, &changed));

    assert!(demand_replay_matches(&original, &original));
}

#[test]
fn route_replay_checks_every_immutable_coordinate() {
    let original = route();
    let mut changed = original.clone();
    changed.statement_digest = "sha256:other".into();
    assert!(!route_replay_matches(&original, &changed));
    assert!(route_replay_matches(&original, &original));
}

#[test]
fn evidence_replay_checks_checker_artifact_and_dependency_coordinates() {
    let original = evidence();
    let mut changed = original.clone();
    changed.checker_revision = "lean:other".into();
    assert!(!evidence_replay_matches(&original, &changed));

    let mut changed = original.clone();
    changed.dependencies = vec!["dep:b".into()];
    assert!(!evidence_replay_matches(&original, &changed));

    assert!(evidence_replay_matches(&original, &original));
}

#[test]
fn payment_replay_checks_exact_reopening_cone() {
    let original = payment();
    let mut changed = original.clone();
    changed.reopen_refs.push("dependent:extra".into());
    assert!(!payment_replay_matches(&original, &changed));
    assert!(payment_replay_matches(&original, &original));
}
