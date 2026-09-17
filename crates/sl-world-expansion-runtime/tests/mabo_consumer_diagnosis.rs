use std::collections::BTreeMap;

use sensiblaw_consumer_residual::{EvidenceCoordinateKind, RequirementNeed, RequirementScope};
use sensiblaw_pg_source_store::{DiscoveryIdentityBaseline, LatentWorldEdgeRow, LatentWorldRows};
use sensiblaw_proof_search_loop::frontier::ResidualStatus;
use sensiblaw_proof_search_loop::world_expansion::ResidualClass;
use sensiblaw_world_expansion_runtime::diagnose_mabo_context_world_identity;

fn world(edges: Vec<LatentWorldEdgeRow>) -> LatentWorldRows {
    LatentWorldRows {
        seed_ref: "Q1501525".into(),
        max_hops: 100,
        requested_max_hops: 100,
        visited_refs: vec!["Q1501525".into(), "Q975866".into(), "Q123".into()],
        deepest_observed_hop: 2,
        frontier_exhausted: true,
        frontier_refs: vec![],
        residual_refs: vec![],
        edges,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn reviewed_edge(relation_type_ref: &str, target: &str) -> LatentWorldEdgeRow {
    LatentWorldEdgeRow {
        from_ref: "Q1501525".into(),
        to_ref: target.into(),
        // `latent_world` stores `algebra.relation.relation_type_ref` in this
        // compatibility field, despite the historical `relation_ref` name.
        relation_ref: relation_type_ref.into(),
        provenance_refs: vec![
            "context:wikidata:wikidata:Q1501525:oldid:2333409615".into(),
        ],
    }
}

#[test]
fn reviewed_context_targets_become_explicit_same_object_requirements_only_once() {
    let rows = world(vec![
        reviewed_edge("context:wikidata:participant", "Q975866"),
        reviewed_edge("context:wikidata:judge", "Q975866"),
        LatentWorldEdgeRow {
            from_ref: "Q1501525".into(),
            to_ref: "internal:pnf:1".into(),
            relation_ref: "legal_ir:pnf_factor".into(),
            provenance_refs: vec!["legal_ir:build:1".into()],
        },
    ]);

    let diagnosis = diagnose_mabo_context_world_identity(&rows, &DiscoveryIdentityBaseline::default());

    assert_eq!(diagnosis.reviewed_context_edges_considered, 2);
    assert_eq!(diagnosis.duplicate_target_edges, 1);
    assert_eq!(diagnosis.out_of_scope_or_wrong_type_edges, 1);
    assert_eq!(diagnosis.consumer_spec.requirements.len(), 1);
    assert_eq!(diagnosis.residuals.len(), 1);

    let requirement = &diagnosis.consumer_spec.requirements[0];
    assert_eq!(requirement.requirement_id, "world-identity:Q975866");
    assert_eq!(
        requirement.need,
        RequirementNeed::EvidenceCoordinate(EvidenceCoordinateKind::SameObject)
    );
    assert_eq!(
        requirement.scope,
        RequirementScope::SourceManifestation(
            "wikidata:Q1501525:oldid:2333409615".into()
        )
    );

    let residual = &diagnosis.residuals[0];
    assert_eq!(residual.residual_ref, "residual:mabo:world-identity:Q975866");
    assert_eq!(residual.status, ResidualStatus::Open);
    assert_eq!(diagnosis.rows[0].residual_class, ResidualClass::Identity);
    assert_eq!(diagnosis.rows[0].discovery_route_ref, "residual-observation");
    assert!(!diagnosis.creates_semantic_authority);
    assert!(!diagnosis.applicability_promoted);
    assert!(!diagnosis.claim_truth_promoted);
}

#[test]
fn durable_identity_baseline_quotients_known_representations_before_residual_emission() {
    let rows = world(vec![reviewed_edge(
        "context:wikidata:participant",
        "Q975866",
    )]);
    let mut baseline = DiscoveryIdentityBaseline::default();
    baseline.representation_identity_class_refs = BTreeMap::from([(
        "Q975866".into(),
        "world-object:eddie-mabo".into(),
    )]);
    baseline.identity_class_refs.insert("world-object:eddie-mabo".into());

    let diagnosis = diagnose_mabo_context_world_identity(&rows, &baseline);

    assert_eq!(diagnosis.known_identity_representations, 1);
    assert!(diagnosis.consumer_spec.requirements.is_empty());
    assert!(diagnosis.residuals.is_empty());
    assert!(diagnosis.rows.is_empty());
}
