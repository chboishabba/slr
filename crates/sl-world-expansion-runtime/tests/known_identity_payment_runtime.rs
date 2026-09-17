use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::transition::ResidualAssessmentKind;
use sensiblaw_proof_search_loop::world_expansion::mabo_world_expansion_policy;
use sensiblaw_proof_search_loop::world_expansion_reentry::PostAcquisitionWorldObservation;
use sensiblaw_proof_search_loop::world_expansion_runner::{
    RecurrentRunBlocker, RecurrentRunBlockerKind,
};
use sensiblaw_proof_search_loop::world_expansion_session::WorldExpansionSession;
use sensiblaw_world_expansion_runtime::{
    apply_known_identity_payment_transaction, KnownIdentityPaymentSink,
    KnownIdentityPaymentRuntimeError,
};

fn session() -> WorldExpansionSession {
    WorldExpansionSession::new(
        ProofFrontier {
            consumer_ref: "consumer:mabo-100-identity-classes".into(),
            frontier_ref: "frontier:mabo:0".into(),
            residuals: vec![ProofResidual {
                residual_ref: "residual:mabo:participant-identity".into(),
                proposition_ref: "mabo:participant:eddie-mabo".into(),
                producer_class_ref: "producer:world-expansion".into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: None,
                salience: 100,
                dependency_refs: vec![],
                status: ResidualStatus::Open,
            }],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        },
        mabo_world_expansion_policy(),
    )
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

#[derive(Default)]
struct RecordingSink {
    persisted: usize,
    fail: bool,
}

impl KnownIdentityPaymentSink for RecordingSink {
    fn persist_reviewed_payment(
        &mut self,
        _wire: &[u8],
    ) -> Result<(), RecurrentRunBlocker> {
        if self.fail {
            return Err(RecurrentRunBlocker::new(
                RecurrentRunBlockerKind::PersistenceBlocked,
                "world-store:blocked",
            ));
        }
        self.persisted += 1;
        Ok(())
    }
}

#[test]
fn successful_payment_persists_before_frontier_commit_and_never_advances_novelty() {
    let mut session = session();
    let mut sink = RecordingSink::default();
    let receipt = apply_known_identity_payment_transaction(
        &mut session,
        "frontier:mabo:1",
        "world-object:eddie-mabo",
        &observation(),
        b"review-payment-wire",
        &mut sink,
    )
    .unwrap();

    assert_eq!(sink.persisted, 1);
    assert_eq!(receipt.identity_class_ref, "world-object:eddie-mabo");
    assert_eq!(session.frontier.frontier_ref, "frontier:mabo:1");
    assert_eq!(session.frontier.open_residuals().count(), 0);
    assert_eq!(session.ledger.total_new_world_objects, 0);
    assert!(session.lineages.is_empty());
}

#[test]
fn persistence_failure_rolls_back_frontier_and_novelty() {
    let mut session = session();
    let initial = session.clone();
    let mut sink = RecordingSink {
        persisted: 0,
        fail: true,
    };
    let error = apply_known_identity_payment_transaction(
        &mut session,
        "frontier:mabo:1",
        "world-object:eddie-mabo",
        &observation(),
        b"review-payment-wire",
        &mut sink,
    )
    .unwrap_err();

    assert_eq!(
        error,
        KnownIdentityPaymentRuntimeError::Persistence(RecurrentRunBlocker::new(
            RecurrentRunBlockerKind::PersistenceBlocked,
            "world-store:blocked",
        ))
    );
    assert_eq!(session, initial);
}
