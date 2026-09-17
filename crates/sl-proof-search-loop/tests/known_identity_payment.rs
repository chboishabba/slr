use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::transition::ResidualAssessmentKind;
use sensiblaw_proof_search_loop::world_expansion::{mabo_world_expansion_policy, WorldExpansionLedger};
use sensiblaw_proof_search_loop::world_expansion_reentry::PostAcquisitionWorldObservation;
use sensiblaw_proof_search_loop::world_expansion_session::WorldExpansionSession;
use sensiblaw_proof_search_loop::world_known_identity_payment::{
    reenter_known_identity_payment, KnownIdentityPaymentError,
};

fn residual(status: ResidualStatus) -> ProofResidual {
    ProofResidual {
        residual_ref: "residual:mabo:participant-identity".into(),
        proposition_ref: "mabo:participant:eddie-mabo".into(),
        producer_class_ref: "producer:world-expansion".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: None,
        salience: 100,
        dependency_refs: vec![],
        status,
    }
}

fn frontier(status: ResidualStatus) -> ProofFrontier {
    ProofFrontier {
        consumer_ref: "consumer:mabo-100-identity-classes".into(),
        frontier_ref: "frontier:mabo:0".into(),
        residuals: vec![residual(status)],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    }
}

fn observation() -> PostAcquisitionWorldObservation {
    PostAcquisitionWorldObservation {
        observation_ref: "query:mabo:P710:Q975866".into(),
        source_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(),
        triggering_residual_ref: "residual:mabo:participant-identity".into(),
        assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
        observed_residual_contraction: 1,
        newly_exposed_residuals: vec![],
        pnf_world_disambiguation_ref: "review:mabo:P710:Q975866".into(),
        observation_authority: "experimental_candidate_only",
    }
}

#[test]
fn known_identity_payment_closes_open_residual_without_novelty_or_discovery_lineage() {
    let session = WorldExpansionSession {
        frontier: frontier(ResidualStatus::Open),
        ledger: WorldExpansionLedger::default(),
        policy: mabo_world_expansion_policy(),
        lineages: vec![],
    };

    let receipt = reenter_known_identity_payment(
        &session,
        "frontier:mabo:1",
        "world-object:eddie-mabo",
        &observation(),
    )
    .unwrap();

    assert_eq!(receipt.identity_class_ref, "world-object:eddie-mabo");
    assert_eq!(receipt.observed_residual_contraction, 1);
    assert_eq!(receipt.next_frontier.open_residuals().count(), 0);
    assert_eq!(session.ledger.total_new_world_objects, 0);
    assert!(session.lineages.is_empty());
    assert!(receipt.non_novel_payment);
    assert!(!receipt.creates_semantic_authority);
    assert!(!receipt.claim_truth_promoted);
}

#[test]
fn known_identity_payment_requires_open_matching_residual() {
    let session = WorldExpansionSession {
        frontier: frontier(ResidualStatus::SatisfiedCandidate),
        ledger: WorldExpansionLedger::default(),
        policy: mabo_world_expansion_policy(),
        lineages: vec![],
    };
    assert_eq!(
        reenter_known_identity_payment(
            &session,
            "frontier:mabo:1",
            "world-object:eddie-mabo",
            &observation(),
        ),
        Err(KnownIdentityPaymentError::ResidualNotOpen)
    );

    let mut wrong = observation();
    wrong.triggering_residual_ref = "residual:other".into();
    let open_session = WorldExpansionSession {
        frontier: frontier(ResidualStatus::Open),
        ledger: WorldExpansionLedger::default(),
        policy: mabo_world_expansion_policy(),
        lineages: vec![],
    };
    assert_eq!(
        reenter_known_identity_payment(
            &open_session,
            "frontier:mabo:1",
            "world-object:eddie-mabo",
            &wrong,
        ),
        Err(KnownIdentityPaymentError::ResidualNotFound)
    );
}
