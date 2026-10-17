use sensiblaw_core::canonical_evidence::{
    ConsumerAdequacyReceipt, ConsumerInformativenessReceipt, MatterConsumerProjectionError,
    MatterPersona, MatterRepresentationProjection, SameMatterInformativenessReceipt,
};

fn adequacy(representation_ref: &str, consumer_ref: &str) -> ConsumerAdequacyReceipt {
    ConsumerAdequacyReceipt::new(
        representation_ref,
        consumer_ref,
        "query-family:understand-live-controversy",
        format!("admission:{representation_ref}"),
        [format!("factor:{representation_ref}")],
        true,
        true,
        None,
    )
    .unwrap()
}

fn projection(
    matter_ref: &str,
    controversy_ref: &str,
    persona: MatterPersona,
    representation_ref: &str,
) -> MatterRepresentationProjection {
    MatterRepresentationProjection::new(
        matter_ref,
        controversy_ref,
        persona,
        representation_ref,
        adequacy(representation_ref, "consumer:affected-person"),
        format!("projection:{representation_ref}"),
    )
    .unwrap()
}

#[test]
fn same_matter_simple_and_full_surfaces_can_carry_query_relative_informativeness() {
    let simple = projection(
        "matter:real",
        "controversy:real",
        MatterPersona::AffectedPerson,
        "repr:simple",
    );
    let full = projection(
        "matter:real",
        "controversy:real",
        MatterPersona::SolicitorCounsel,
        "repr:full",
    );

    let informativeness = ConsumerInformativenessReceipt::new(
        simple.adequacy.clone(),
        full.adequacy.clone(),
        ["distinction:typed-response", "distinction:open-obligation"],
        ["distinction:typed-response", "distinction:open-obligation"],
        "review:factorisation:same-matter",
    )
    .unwrap();

    let receipt = SameMatterInformativenessReceipt::new(
        simple,
        full,
        informativeness,
        "receipt:same-matter:informativeness",
    )
    .unwrap();

    assert_eq!(receipt.matter_ref(), "matter:real");
    assert_eq!(receipt.controversy_ref(), "controversy:real");
    assert!(receipt.informativeness.left_at_least_as_informative());
    assert!(!receipt.creates_new_matter);
    assert!(!receipt.creates_semantic_authority);
    assert!(!receipt.determines_credibility);
    assert!(!receipt.enters_final_judgment);
}

#[test]
fn informativeness_binding_rejects_cross_matter_comparison() {
    let left = projection(
        "matter:a",
        "controversy:shared-looking",
        MatterPersona::AffectedPerson,
        "repr:simple",
    );
    let right = projection(
        "matter:b",
        "controversy:shared-looking",
        MatterPersona::SolicitorCounsel,
        "repr:full",
    );
    let informativeness = ConsumerInformativenessReceipt::new(
        left.adequacy.clone(),
        right.adequacy.clone(),
        ["distinction:a"],
        ["distinction:a"],
        "review:factorisation:cross-matter",
    )
    .unwrap();

    assert_eq!(
        SameMatterInformativenessReceipt::new(
            left,
            right,
            informativeness,
            "receipt:cross-matter",
        )
        .unwrap_err(),
        MatterConsumerProjectionError::MatterMismatch
    );
}

#[test]
fn projection_requires_adequacy_representation_identity() {
    let mismatched = adequacy("repr:other", "consumer:affected-person");

    let err = MatterRepresentationProjection::new(
        "matter:real",
        "controversy:real",
        MatterPersona::AffectedPerson,
        "repr:actual",
        mismatched,
        "projection:repr:actual",
    )
    .unwrap_err();

    assert_eq!(
        err,
        MatterConsumerProjectionError::RepresentationProjectionMismatch
    );
}

#[test]
fn informativeness_binding_requires_exact_adequacy_ancestry() {
    let left = projection(
        "matter:real",
        "controversy:real",
        MatterPersona::AffectedPerson,
        "repr:simple",
    );
    let right = projection(
        "matter:real",
        "controversy:real",
        MatterPersona::SolicitorCounsel,
        "repr:full",
    );
    let mut informativeness = ConsumerInformativenessReceipt::new(
        left.adequacy.clone(),
        right.adequacy.clone(),
        ["distinction:a"],
        ["distinction:a"],
        "review:factorisation:same-matter",
    )
    .unwrap();
    informativeness.left.admissibility_receipt_ref = "admission:mutated".into();

    assert_eq!(
        SameMatterInformativenessReceipt::new(
            left,
            right,
            informativeness,
            "receipt:mutated-ancestry",
        )
        .unwrap_err(),
        MatterConsumerProjectionError::InformativenessProjectionMismatch
    );
}

#[test]
fn hand_built_cross_consumer_informativeness_is_rejected() {
    let left = MatterRepresentationProjection::new(
        "matter:real",
        "controversy:real",
        MatterPersona::AffectedPerson,
        "repr:simple",
        adequacy("repr:simple", "consumer:affected-person"),
        "projection:repr:simple",
    )
    .unwrap();
    let right = MatterRepresentationProjection::new(
        "matter:real",
        "controversy:real",
        MatterPersona::Expert,
        "repr:expert",
        adequacy("repr:expert", "consumer:expert"),
        "projection:repr:expert",
    )
    .unwrap();

    // Deliberately bypass ConsumerInformativenessReceipt::new to simulate a
    // malformed decoded/constructed receipt with otherwise valid adequacy rows.
    let informativeness = ConsumerInformativenessReceipt {
        left: left.adequacy.clone(),
        right: right.adequacy.clone(),
        right_preserved_distinction_refs: ["distinction:a".to_owned()].into_iter().collect(),
        left_preserved_distinction_refs: ["distinction:a".to_owned()].into_iter().collect(),
        reviewed_factorisation_receipt_ref: "review:factorisation:malformed".into(),
        creates_universal_superiority: false,
        promotes_claim_truth: false,
    };

    assert_eq!(
        SameMatterInformativenessReceipt::new(
            left,
            right,
            informativeness,
            "receipt:cross-consumer",
        )
        .unwrap_err(),
        MatterConsumerProjectionError::InformativenessProjectionMismatch
    );
}
