use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::transition::ResidualAssessmentKind;
use sensiblaw_proof_search_loop::world_expansion::{
    DisambiguationOutcome, ExpansionCandidate, KnowledgeObjectKind, ProducerLane, ResidualClass,
    ReviewDecision, mabo_world_expansion_policy,
};
use sensiblaw_proof_search_loop::world_expansion_identity_session::apply_reviewed_cycle_with_identity;
use sensiblaw_proof_search_loop::world_expansion_reentry::PostAcquisitionWorldObservation;
use sensiblaw_proof_search_loop::world_expansion_session::WorldExpansionSession;
use sensiblaw_proof_search_loop::world_expansion_step::ResidualRouting;
use sensiblaw_proof_search_loop::world_identity::{WorldIdentityResolutionReceipt, WorldObjectIdentity};

fn frontier() -> ProofFrontier {
    ProofFrontier {
        consumer_ref: "consumer:mabo-100-object-world".into(),
        frontier_ref: "frontier:mabo:0".into(),
        residuals: vec![ProofResidual {
            residual_ref: "residual:mabo:eddie-identity".into(),
            proposition_ref: "mabo:proposition:participant-identity".into(),
            producer_class_ref: "producer:world-expansion".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: None,
            salience: 100,
            dependency_refs: vec![],
            status: ResidualStatus::Open,
        }],
        satisfied_payment_refs: vec![], contested_coordinate_refs: vec![], authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    }
}

fn candidate(object_ref: &str, kind: KnowledgeObjectKind, lane: ProducerLane) -> ExpansionCandidate {
    ExpansionCandidate {
        candidate_ref: format!("candidate:{object_ref}"), object_ref: object_ref.into(), object_kind: kind,
        discovery_parent_ref: "Q1501525".into(), triggering_residual_ref: "residual:mabo:eddie-identity".into(),
        residual_class: ResidualClass::Identity, producer_lane: lane,
        source_revision_ref: Some("source-revision:test".into()), expected_residual_contraction: 2,
        provenance_quality: 5, same_object_confidence: 5, expected_new_world_value: 5,
        acquisition_cost: 1, admissible: true,
    }
}

fn observation() -> PostAcquisitionWorldObservation {
    PostAcquisitionWorldObservation {
        observation_ref: "world-observation:mabo:eddie".into(), source_revision_ref: "source-revision:test".into(),
        triggering_residual_ref: "residual:mabo:eddie-identity".into(),
        assessment_kind: ResidualAssessmentKind::SatisfiedCandidate, observed_residual_contraction: 2,
        newly_exposed_residuals: vec![], pnf_world_disambiguation_ref: "pnf-world:mabo:eddie".into(),
        observation_authority: "experimental_candidate_only",
    }
}

#[test]
fn qid_then_article_alias_does_not_advance_novel_object_count_twice() {
    let identity = WorldIdentityResolutionReceipt::same_object(
        "identity-resolution:mabo:eddie",
        WorldObjectIdentity::new("world-object:eddie-mabo", "Q975866")
            .with_alias("https://en.wikipedia.org/wiki/Eddie_Mabo"),
        "reviewed-qid-sitelink",
    );
    let routing = ResidualRouting {
        residual_ref: "residual:mabo:eddie-identity".into(), residual_class: ResidualClass::Identity,
        routing_reason_ref: "pnf:participant-identity".into(),
    };
    let mut session = WorldExpansionSession::new(frontier(), mabo_world_expansion_policy());
    apply_reviewed_cycle_with_identity(
        &mut session, "frontier:mabo:1", &routing,
        &[candidate("Q975866", KnowledgeObjectKind::Qid, ProducerLane::WikidataIdentity)],
        ReviewDecision::Reviewed, DisambiguationOutcome::NewRelatedObject, &identity, &observation(),
    ).unwrap();
    assert_eq!(session.ledger.total_new_world_objects, 1);
    assert!(session.ledger.contains_identity_class("world-object:eddie-mabo"));
}
