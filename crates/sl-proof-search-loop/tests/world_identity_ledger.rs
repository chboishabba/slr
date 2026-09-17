use sensiblaw_proof_search_loop::world_expansion::{
    review_admission_with_identity, DisambiguationOutcome, ExpansionCandidate,
    KnowledgeObjectKind, ProducerLane, ResidualClass, ReviewDecision, WorldExpansionLedger,
};
use sensiblaw_proof_search_loop::world_identity::{
    WorldIdentityResolutionKind, WorldIdentityResolutionReceipt, WorldObjectIdentity,
};

fn candidate(candidate_ref: &str, object_ref: &str) -> ExpansionCandidate {
    ExpansionCandidate {
        candidate_ref: candidate_ref.into(),
        object_ref: object_ref.into(),
        object_kind: KnowledgeObjectKind::Qid,
        discovery_parent_ref: "Q1501525".into(),
        triggering_residual_ref: "residual:mabo:participant-identity".into(),
        residual_class: ResidualClass::Identity,
        producer_lane: ProducerLane::WikidataIdentity,
        source_revision_ref: Some("wikidata:Q1501525:oldid:2333409615".into()),
        expected_residual_contraction: 1,
        provenance_quality: 5,
        same_object_confidence: 5,
        expected_new_world_value: 5,
        acquisition_cost: 1,
        admissible: true,
    }
}

fn resolution(
    receipt_ref: &str,
    identity_class_ref: &str,
    primary: &str,
    aliases: &[&str],
) -> WorldIdentityResolutionReceipt {
    let mut identity = WorldObjectIdentity::new(identity_class_ref, primary);
    for alias in aliases {
        identity = identity.with_alias(*alias);
    }
    WorldIdentityResolutionReceipt {
        receipt_ref: receipt_ref.into(),
        identity,
        resolution_kind: WorldIdentityResolutionKind::SameObjectDifferentRepresentation,
        evidence_ref: format!("evidence:{receipt_ref}"),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn same_representation_cannot_be_counted_under_two_identity_classes() {
    let mut ledger = WorldExpansionLedger::default();
    let first_candidate = candidate("candidate:first", "Q975866");
    let first = resolution(
        "identity:first",
        "world-object:eddie-mabo",
        "Q975866",
        &["https://en.wikipedia.org/wiki/Eddie_Mabo"],
    );
    let first_receipt = review_admission_with_identity(
        &mut ledger,
        &first_candidate,
        ReviewDecision::Reviewed,
        DisambiguationOutcome::NewRelatedObject,
        &first,
    );
    assert!(first_receipt.admitted);
    assert_eq!(ledger.total_new_world_objects, 1);

    let conflicting_candidate = candidate("candidate:conflict", "Q975866");
    let conflicting = resolution(
        "identity:conflict",
        "world-object:incorrect-second-class",
        "Q975866",
        &[],
    );
    let conflict_receipt = review_admission_with_identity(
        &mut ledger,
        &conflicting_candidate,
        ReviewDecision::Reviewed,
        DisambiguationOutcome::NewRelatedObject,
        &conflicting,
    );

    assert!(!conflict_receipt.admitted);
    assert_eq!(ledger.total_new_world_objects, 1);
    assert_eq!(ledger.identity_ambiguous, 1);
    assert_eq!(
        ledger.identity_class_for_representation("Q975866"),
        Some("world-object:eddie-mabo")
    );
}

#[test]
fn same_identity_class_can_learn_a_new_alias_without_advancing_novelty() {
    let mut ledger = WorldExpansionLedger::default();
    let qid_candidate = candidate("candidate:qid", "Q975866");
    let qid_only = resolution(
        "identity:qid",
        "world-object:eddie-mabo",
        "Q975866",
        &[],
    );
    assert!(review_admission_with_identity(
        &mut ledger,
        &qid_candidate,
        ReviewDecision::Reviewed,
        DisambiguationOutcome::NewRelatedObject,
        &qid_only,
    )
    .admitted);

    let article_candidate = candidate(
        "candidate:article-alias",
        "https://en.wikipedia.org/wiki/Eddie_Mabo",
    );
    let alias_resolution = resolution(
        "identity:article-alias",
        "world-object:eddie-mabo",
        "Q975866",
        &["https://en.wikipedia.org/wiki/Eddie_Mabo"],
    );
    let alias_receipt = review_admission_with_identity(
        &mut ledger,
        &article_candidate,
        ReviewDecision::Reviewed,
        DisambiguationOutcome::SameObject,
        &alias_resolution,
    );

    assert!(!alias_receipt.admitted);
    assert_eq!(ledger.total_new_world_objects, 1);
    assert_eq!(ledger.duplicates_seen, 1);
    assert_eq!(
        ledger.identity_class_for_representation("https://en.wikipedia.org/wiki/Eddie_Mabo"),
        Some("world-object:eddie-mabo")
    );
}
