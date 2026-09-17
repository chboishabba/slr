use sensiblaw_consumer_residual::{
    compile_consumer_residual_stream_review_aware, compile_reviewed_evidence_payment,
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, RequirementNeed, RequirementScope,
    ReviewedEvidenceCoordinate,
};
use sensiblaw_world_store::{decode_record, WorldRecordKind};

fn spec() -> ConsumerSpec {
    ConsumerSpec {
        consumer_id: "consumer:mabo-100-identity-classes".into(),
        surface_id: "surface:mabo".into(),
        requirements: vec![ConsumerRequirement {
            requirement_id: "participant-identity".into(),
            need: RequirementNeed::EvidenceCoordinate(EvidenceCoordinateKind::SameObject),
            scope: RequirementScope::SourceManifestation("wikidata:Q1501525:oldid:2333409615".into()),
        }],
    }
}

#[test]
fn reviewed_evidence_coordinate_emits_review_and_exact_gap_obligation_payments() {
    let review = ReviewedEvidenceCoordinate {
        review_ref: "review:mabo:P710:Q975866".into(),
        consumer_id: "consumer:mabo-100-identity-classes".into(),
        requirement_id: "participant-identity".into(),
        coordinate: EvidenceCoordinateKind::SameObject,
        source_ref: Some("wikidata:Q1501525:oldid:2333409615".into()),
        evidence_ref: "world-observation:mabo:P710:Q975866".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let mut bytes = Vec::new();
    let receipt = compile_reviewed_evidence_payment(&spec(), &review, &mut bytes, 7).unwrap();
    assert_eq!(receipt.payments_emitted, 2);
    assert!(receipt.review_emitted);
    assert!(receipt.candidate_only);
    assert!(!receipt.semantic_promotion);

    let mut cursor = std::io::Cursor::new(bytes);
    let first = decode_record(&mut cursor).unwrap().unwrap();
    let second = decode_record(&mut cursor).unwrap().unwrap();
    let third = decode_record(&mut cursor).unwrap().unwrap();
    assert_eq!(first.kind, WorldRecordKind::Review);
    assert_eq!(second.kind, WorldRecordKind::Payment);
    assert_eq!(third.kind, WorldRecordKind::Payment);
    let mut targets = vec![second.aux1.unwrap(), third.aux1.unwrap()];
    targets.sort();
    assert_eq!(targets, vec![
        "gap:consumer:mabo-100-identity-classes:participant-identity",
        "obligation:consumer:mabo-100-identity-classes:participant-identity",
    ]);
}

#[test]
fn reviewed_evidence_payment_makes_evidence_requirement_paid_on_next_frontier() {
    let review = ReviewedEvidenceCoordinate {
        review_ref: "review:mabo:P710:Q975866".into(),
        consumer_id: "consumer:mabo-100-identity-classes".into(),
        requirement_id: "participant-identity".into(),
        coordinate: EvidenceCoordinateKind::SameObject,
        source_ref: Some("wikidata:Q1501525:oldid:2333409615".into()),
        evidence_ref: "world-observation:mabo:P710:Q975866".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let mut reviewed_world = Vec::new();
    compile_reviewed_evidence_payment(&spec(), &review, &mut reviewed_world, 7).unwrap();

    let mut next = Vec::new();
    let receipt = compile_consumer_residual_stream_review_aware(
        &mut std::io::Cursor::new(reviewed_world),
        &spec(),
        &mut next,
        8,
    )
    .unwrap();
    assert_eq!(receipt.requirements_paid, 1);
    assert_eq!(receipt.requirements_unpaid, 0);
    assert_eq!(receipt.gaps_emitted, 0);
    assert_eq!(receipt.obligations_emitted, 0);
}

#[test]
fn evidence_payment_fails_closed_on_coordinate_scope_or_promotion_mismatch() {
    let mut review = ReviewedEvidenceCoordinate {
        review_ref: "review:mabo:P710:Q975866".into(),
        consumer_id: "consumer:mabo-100-identity-classes".into(),
        requirement_id: "participant-identity".into(),
        coordinate: EvidenceCoordinateKind::Authority,
        source_ref: Some("wikidata:Q1501525:oldid:2333409615".into()),
        evidence_ref: "world-observation:mabo:P710:Q975866".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    assert!(compile_reviewed_evidence_payment(&spec(), &review, &mut Vec::new(), 7).is_err());

    review.coordinate = EvidenceCoordinateKind::SameObject;
    review.source_ref = Some("wikidata:Q1:oldid:1".into());
    assert!(compile_reviewed_evidence_payment(&spec(), &review, &mut Vec::new(), 7).is_err());

    review.source_ref = Some("wikidata:Q1501525:oldid:2333409615".into());
    review.claim_truth_promoted = true;
    assert!(compile_reviewed_evidence_payment(&spec(), &review, &mut Vec::new(), 7).is_err());
}
