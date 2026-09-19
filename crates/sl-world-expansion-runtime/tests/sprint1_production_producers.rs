use std::collections::BTreeMap;

use sensiblaw_governed_legal_provider::{OalcRecord, OalcSnapshot};
use sensiblaw_route_selector::ProducerFamily;
use sensiblaw_wikimedia_candidate_provider::entity_revision_receipt_from_rdf;
use sensiblaw_world_expansion_runtime::gwb_supervised_type_closure::{
    TypeClosureError, TypeClosureNodeProvider, TypeNodeAcquisition, TypeNodeReceipt,
    TypeObservation, TypeObservationProperty, TypeProviderStats,
};
use sensiblaw_world_expansion_runtime::sprint1_acquisition_machine::{
    ProducerExecutionPlan, Sprint1ProducerController,
};
use sensiblaw_world_expansion_runtime::sprint1_producers::{
    register_sprint1_production_families, ClassificationProducerExecutor,
    OalcAuthorityProducerExecutor, WikidataIdentityProducerExecutor,
};

#[derive(Clone)]
struct FixtureClassificationProvider {
    rows: BTreeMap<String, TypeNodeAcquisition>,
    calls: usize,
}

impl TypeClosureNodeProvider for FixtureClassificationProvider {
    fn acquire_type_node(
        &mut self,
        qid: &str,
    ) -> Result<Option<TypeNodeAcquisition>, TypeClosureError> {
        self.calls += 1;
        Ok(self.rows.get(qid).cloned())
    }

    fn snapshot_simultaneous(&self) -> bool {
        true
    }

    fn stats(&self) -> TypeProviderStats {
        TypeProviderStats {
            provider_calls: self.calls,
            ..TypeProviderStats::default()
        }
    }
}

fn node(qid: &str, observations: Vec<TypeObservation>) -> TypeNodeAcquisition {
    TypeNodeAcquisition {
        qid: qid.into(),
        observations,
        node_receipt: TypeNodeReceipt {
            qid: qid.into(),
            source_revision_ref: format!("zelph-hf:snapshot:{qid}"),
            evidence_digest_ref:
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .into(),
        },
    }
}

fn plan(family: ProducerFamily, target: &str) -> ProducerExecutionPlan {
    ProducerExecutionPlan {
        residual_ref: format!("residual:{family:?}:{target}"),
        move_ref: format!("move:{family:?}:{target}"),
        producer: family,
        target_ref: target.into(),
    }
}

#[test]
fn real_production_adapters_share_one_controller_and_remain_candidate_only() {
    let classification = FixtureClassificationProvider {
        rows: BTreeMap::from([
            (
                "Q1".into(),
                node(
                    "Q1",
                    vec![TypeObservation {
                        subject_qid: "Q1".into(),
                        property: TypeObservationProperty::InstanceOf,
                        target_qid: "Q10".into(),
                        source_revision_ref: "zelph-hf:snapshot:Q1".into(),
                        evidence_digest_ref:
                            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                                .into(),
                    }],
                ),
            ),
            ("Q10".into(), node("Q10", vec![])),
        ]),
        calls: 0,
    };

    let identity = WikidataIdentityProducerExecutor::with_fetcher(|qid| {
        entity_revision_receipt_from_rdf(qid, 42, b"<rdf:RDF/>".to_vec())
            .map_err(|error| {
                sensiblaw_world_expansion_runtime::sprint1_acquisition_machine::Sprint1AcquisitionError::Provider(
                    format!("fixture-identity:{error}"),
                )
            })
    });

    let authority = OalcAuthorityProducerExecutor::new(OalcSnapshot {
        corpus_revision_ref: "oalc:fixture:v1".into(),
        records: vec![OalcRecord {
            citation: "[2026] HCA 19".into(),
            source_identity_ref: "case:[2026]-HCA-19".into(),
            source_revision_ref: "oalc:[2026]-HCA-19:rev:fixture".into(),
            canonical_text_digest:
                "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    .into(),
            local_artifact_ref: "fixture://cullen".into(),
        }],
    });

    let mut controller = Sprint1ProducerController::new();
    register_sprint1_production_families(
        &mut controller,
        ClassificationProducerExecutor::new(classification, 4, 16),
        identity,
        authority,
    );

    let classification = controller
        .execute(&plan(ProducerFamily::ClassificationEvidence, "Q1"))
        .unwrap();
    assert_eq!(classification.producer, ProducerFamily::ClassificationEvidence);
    assert!(classification.complete);

    let identity = controller
        .execute(&plan(ProducerFamily::IdentitySource, "Q2"))
        .unwrap();
    assert_eq!(identity.producer, ProducerFamily::IdentitySource);
    assert_eq!(identity.source_revision_ref, "wikidata:Q2:oldid:42");

    let authority = controller
        .execute(&plan(ProducerFamily::AuthoritySource, "[2026] HCA 19"))
        .unwrap();
    assert_eq!(authority.producer, ProducerFamily::AuthoritySource);
    assert_eq!(
        authority.source_revision_ref,
        "oalc:[2026]-HCA-19:rev:fixture"
    );

    for evidence in [classification, identity, authority] {
        assert!(evidence.candidate_only);
        assert!(!evidence.creates_semantic_authority);
        assert!(!evidence.applicability_promoted);
        assert!(!evidence.claim_truth_promoted);
    }
}
