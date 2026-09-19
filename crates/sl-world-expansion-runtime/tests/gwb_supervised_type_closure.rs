use sensiblaw_world_expansion_runtime::gwb_supervised_type_closure::{
    acquire_supervised_type_closure_with, evaluate_observed_type_closure,
    TieredTypeClosureProvider, TypeClosureDisposition, TypeClosureError,
    TypeClosureNodeProvider, TypeClosureQuestion, TypeClosureRequest, TypeNodeAcquisition,
    TypeNodeReceipt, TypeObservation, TypeObservationProperty, TypeProviderStats,
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


#[test]
fn q7725634_historical_lean_cache_shape_is_direct_superclass_observation() {
    // Historical regression only:
    // dashi_lean4@349f9b7d...
    // DASHI/output-final_aristotle/.wikidata-cache/Q7725634.json
    // lastrevid=2519725566 contained direct P279 values Q838948 and Q47461344.
    // This does NOT assert current Wikidata state.
    let closure = evaluate_observed_type_closure(
        &TypeClosureRequest {
            root_qid: "Q7725634".into(),
            question: TypeClosureQuestion::Superclass,
            max_depth: 4,
            max_nodes: 32,
        },
        &[
            obs(
                "Q7725634",
                TypeObservationProperty::SubclassOf,
                "Q838948",
                "wikidata:Q7725634:oldid:2519725566",
            ),
            obs(
                "Q7725634",
                TypeObservationProperty::SubclassOf,
                "Q47461344",
                "wikidata:Q7725634:oldid:2519725566",
            ),
        ],
        false,
    )
    .unwrap();

    assert_eq!(
        closure.disposition,
        TypeClosureDisposition::DirectSuperclassObserved
    );
    assert_eq!(
        closure.direct_superclasses,
        vec!["Q47461344".to_string(), "Q838948".to_string()]
    );
    assert!(!closure.superclass_residual_paid);
    assert!(!closure.global_ontology_complete);
    assert!(!closure.snapshot_simultaneous);
}


#[derive(Clone)]
struct FixtureProvider {
    rows: std::collections::BTreeMap<String, TypeNodeAcquisition>,
    simultaneous: bool,
    calls: usize,
}

impl FixtureProvider {
    fn new(rows: Vec<TypeNodeAcquisition>, simultaneous: bool) -> Self {
        Self {
            rows: rows.into_iter().map(|row| (row.qid.clone(), row)).collect(),
            simultaneous,
            calls: 0,
        }
    }
}

impl TypeClosureNodeProvider for FixtureProvider {
    fn acquire_type_node(
        &mut self,
        qid: &str,
    ) -> Result<Option<TypeNodeAcquisition>, TypeClosureError> {
        self.calls += 1;
        Ok(self.rows.get(qid).cloned())
    }

    fn snapshot_simultaneous(&self) -> bool {
        self.simultaneous
    }

    fn stats(&self) -> TypeProviderStats {
        TypeProviderStats {
            provider_calls: self.calls,
            ..TypeProviderStats::default()
        }
    }
}

fn node(qid: &str, rows: Vec<TypeObservation>, source: &str) -> TypeNodeAcquisition {
    TypeNodeAcquisition {
        qid: qid.into(),
        observations: rows,
        node_receipt: TypeNodeReceipt {
            qid: qid.into(),
            source_revision_ref: source.into(),
            evidence_digest_ref:
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        },
    }
}

#[test]
fn snapshot_first_provider_avoids_live_when_snapshot_covers_closure() {
    let snapshot = FixtureProvider::new(
        vec![
            node(
                "Q1",
                vec![obs(
                    "Q1",
                    TypeObservationProperty::SubclassOf,
                    "Q2",
                    "zelph-hf:snapshot-1:Q1",
                )],
                "zelph-hf:snapshot-1:Q1",
            ),
            node("Q2", vec![], "zelph-hf:snapshot-1:Q2"),
        ],
        true,
    );
    let live = FixtureProvider::new(vec![], false);
    let mut tiered = TieredTypeClosureProvider::new(snapshot, live);
    let closure = acquire_supervised_type_closure_with(
        &TypeClosureRequest {
            root_qid: "Q1".into(),
            question: TypeClosureQuestion::Superclass,
            max_depth: 4,
            max_nodes: 16,
        },
        &mut tiered,
    )
    .unwrap();

    assert!(closure.snapshot_simultaneous);
    assert_eq!(tiered.stats().snapshot_hits, 2);
    assert_eq!(tiered.stats().live_fallbacks, 0);
    assert_eq!(tiered.live().stats().provider_calls, 0);
}

#[test]
fn live_fallback_is_explicit_and_revokes_snapshot_simultaneity() {
    let snapshot = FixtureProvider::new(
        vec![node(
            "Q1",
            vec![obs(
                "Q1",
                TypeObservationProperty::SubclassOf,
                "Q2",
                "zelph-hf:snapshot-1:Q1",
            )],
            "zelph-hf:snapshot-1:Q1",
        )],
        true,
    );
    let live = FixtureProvider::new(
        vec![node("Q2", vec![], "wikidata:Q2:oldid:42")],
        false,
    );
    let mut tiered = TieredTypeClosureProvider::new(snapshot, live);
    let closure = acquire_supervised_type_closure_with(
        &TypeClosureRequest {
            root_qid: "Q1".into(),
            question: TypeClosureQuestion::Superclass,
            max_depth: 4,
            max_nodes: 16,
        },
        &mut tiered,
    )
    .unwrap();

    assert!(!closure.snapshot_simultaneous);
    assert_eq!(tiered.stats().snapshot_hits, 1);
    assert_eq!(tiered.stats().live_fallbacks, 1);
    assert_eq!(tiered.live().stats().provider_calls, 1);
}

#[test]
fn provider_gap_abstains_by_marking_closure_truncated() {
    let mut provider = FixtureProvider::new(vec![], true);
    let closure = acquire_supervised_type_closure_with(
        &TypeClosureRequest {
            root_qid: "Q1".into(),
            question: TypeClosureQuestion::Superclass,
            max_depth: 4,
            max_nodes: 16,
        },
        &mut provider,
    )
    .unwrap();

    assert!(closure.truncated);
    assert_eq!(closure.disposition, TypeClosureDisposition::Truncated);
    assert!(!closure.superclass_residual_paid);
}
