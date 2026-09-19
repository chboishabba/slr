use sensiblaw_world_expansion_runtime::gwb_supervised_type_closure::{
    evaluate_observed_type_closure, TypeClosureDisposition, TypeClosureQuestion,
    TypeObservation, TypeObservationProperty, TypeClosureRequest,
};

fn obs(
    subject: &str,
    property: TypeObservationProperty,
    target: &str,
    revision: &str,
) -> TypeObservation {
    TypeObservation {
        subject_qid: subject.into(),
        property,
        target_qid: target.into(),
        source_revision_ref: revision.into(),
        evidence_digest_ref:
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    }
}

fn request(question: TypeClosureQuestion) -> TypeClosureRequest {
    TypeClosureRequest {
        root_qid: "Q1".into(),
        question,
        max_depth: 4,
        max_nodes: 16,
    }
}

#[test]
fn direct_p279_is_observed_superclass_evidence_not_truth_promotion() {
    let closure = evaluate_observed_type_closure(
        &request(TypeClosureQuestion::Superclass),
        &[obs(
            "Q1",
            TypeObservationProperty::SubclassOf,
            "Q2",
            "wikidata:Q1:oldid:1",
        )],
        false,
    )
    .unwrap();

    assert_eq!(
        closure.disposition,
        TypeClosureDisposition::DirectSuperclassObserved
    );
    assert_eq!(closure.direct_superclasses, vec!["Q2"]);
    assert!(closure.observed_superclass_closure.contains(&"Q2".into()));
    assert!(closure.candidate_only);
    assert!(!closure.creates_semantic_authority);
    assert!(!closure.applicability_promoted);
    assert!(!closure.claim_truth_promoted);
}

#[test]
fn p31_then_p279_chain_yields_bounded_instance_type_closure() {
    let closure = evaluate_observed_type_closure(
        &request(TypeClosureQuestion::TypeClass),
        &[
            obs(
                "Q1",
                TypeObservationProperty::InstanceOf,
                "Q10",
                "wikidata:Q1:oldid:1",
            ),
            obs(
                "Q10",
                TypeObservationProperty::SubclassOf,
                "Q20",
                "wikidata:Q10:oldid:2",
            ),
            obs(
                "Q20",
                TypeObservationProperty::SubclassOf,
                "Q30",
                "wikidata:Q20:oldid:3",
            ),
        ],
        false,
    )
    .unwrap();

    assert_eq!(
        closure.disposition,
        TypeClosureDisposition::InstanceTypeClosureObserved
    );
    assert_eq!(closure.direct_instance_types, vec!["Q10"]);
    assert_eq!(
        closure.observed_instance_type_closure,
        vec!["Q10", "Q20", "Q30"]
    );
    assert!(!closure.global_ontology_complete);
    assert!(!closure.snapshot_simultaneous);
}

#[test]
fn superclass_question_with_only_p31_yields_review_pressure_not_wrong_type_fact() {
    let closure = evaluate_observed_type_closure(
        &request(TypeClosureQuestion::Superclass),
        &[
            obs(
                "Q1",
                TypeObservationProperty::InstanceOf,
                "Q10",
                "wikidata:Q1:oldid:1",
            ),
            obs(
                "Q10",
                TypeObservationProperty::SubclassOf,
                "Q20",
                "wikidata:Q10:oldid:2",
            ),
        ],
        false,
    )
    .unwrap();

    assert_eq!(
        closure.disposition,
        TypeClosureDisposition::InstanceShapedForSuperclassQuestion
    );
    assert_eq!(closure.direct_superclasses.len(), 0);
    assert_eq!(closure.observed_instance_type_closure, vec!["Q10", "Q20"]);
    assert!(!closure.superclass_residual_paid);
}

#[test]
fn bounded_absence_is_not_global_absence() {
    let closure = evaluate_observed_type_closure(
        &request(TypeClosureQuestion::Superclass),
        &[],
        false,
    )
    .unwrap();

    assert_eq!(
        closure.disposition,
        TypeClosureDisposition::ObservedNoTypedEdge
    );
    assert!(closure.observed_direct_surface_complete);
    assert!(!closure.global_ontology_complete);
    assert!(!closure.superclass_residual_paid);
}

#[test]
fn truncation_forces_abstaining_disposition() {
    let closure = evaluate_observed_type_closure(
        &request(TypeClosureQuestion::Superclass),
        &[obs(
            "Q1",
            TypeObservationProperty::SubclassOf,
            "Q2",
            "wikidata:Q1:oldid:1",
        )],
        true,
    )
    .unwrap();

    assert_eq!(closure.disposition, TypeClosureDisposition::Truncated);
    assert!(!closure.superclass_residual_paid);
}
