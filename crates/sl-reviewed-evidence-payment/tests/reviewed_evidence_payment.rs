use sensiblaw_consumer_residual::{
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, RequirementNeed, RequirementScope,
};
use sensiblaw_reviewed_evidence_payment::{
    compile_consumer_residual_stream_review_aware, compile_reviewed_evidence_payment,
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
            scope: RequirementScope::SourceManifestation(
                "wikidata:Q1501525:oldid:2333409615".into(),
            ),
        }],
    }
}

fn review() -> ReviewedEvidenceCoordinate {
    ReviewedEvidenceCoordinate {
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
    }
}

#[test]
fn reviewed_evidence_emits_review_and_distinct_exact_payments() {
    let mut bytes = Vec::new();
    let receipt = compile_reviewed_evidence_payment(&spec(), &review(), &mut bytes, 7).unwrap();
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
    assert_ne!(second.id, third.id);
    let mut targets = vec![second.aux1.unwrap(), third.aux1.unwrap()];
    targets.sort();
    assert_eq!(targets, vec![
        "gap:consumer:mabo-100-identity-classes:participant-identity",
        "obligation:consumer:mabo-100-identity-classes:participant-identity",
    ]);
}

#[test]
fn explicit_reviewed_payment_stays_paid_on_next_residual_compile() {
    let mut reviewed_world = Vec::new();
    compile_reviewed_evidence_payment(&spec(), &review(), &mut reviewed_world, 7).unwrap();

    let mut next = Vec::new();
    let receipt = compile_consumer_residual_stream_review_aware(
        &mut std::io::Cursor::new(reviewed_world),
        &spec(),
        &mut next,
        8,
    )
    .unwrap();
    assert_eq!(receipt.requirements_total, 1);
    assert_eq!(receipt.requirements_paid, 1);
    assert_eq!(receipt.requirements_unpaid, 0);
    assert_eq!(receipt.gaps_emitted, 0);
    assert_eq!(receipt.obligations_emitted, 0);
}

#[test]
fn evidence_payment_fails_closed_on_coordinate_scope_or_promotion_mismatch() {
    let mut value = review();
    value.coordinate = EvidenceCoordinateKind::Authority;
    assert!(compile_reviewed_evidence_payment(&spec(), &value, &mut Vec::new(), 7).is_err());

    value = review();
    value.source_ref = Some("wikidata:Q1:oldid:1".into());
    assert!(compile_reviewed_evidence_payment(&spec(), &value, &mut Vec::new(), 7).is_err());

    value = review();
    value.claim_truth_promoted = true;
    assert!(compile_reviewed_evidence_payment(&spec(), &value, &mut Vec::new(), 7).is_err());
}
