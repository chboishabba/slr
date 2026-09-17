use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::world_expansion::ResidualClass;
use sensiblaw_world_expansion_runtime::{
    select_next_reviewed_mabo_gap, MaboIdentityDiagnosisRow, MaboIdentityReviewAssignment,
    MaboIdentityReviewPlan, MaboPlannedIdentityReview,
};

fn row(representation: &str, residual: &str, relation_types: &[&str]) -> MaboIdentityDiagnosisRow {
    MaboIdentityDiagnosisRow {
        representation_ref: representation.into(),
        relation_type_refs: relation_types.iter().map(|value| (*value).into()).collect(),
        source_revision_refs: vec!["wikidata:Q1501525:oldid:2333409615".into()],
        requirement_id: format!("world-identity:{representation}"),
        residual_ref: residual.into(),
        residual_class: ResidualClass::Identity,
        discovery_route_ref: "residual-observation",
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn planned(representation: &str, residual: &str, relation_types: &[&str]) -> MaboPlannedIdentityReview {
    MaboPlannedIdentityReview {
        row: row(representation, residual, relation_types),
        assignment: MaboIdentityReviewAssignment {
            representation_ref: representation.into(),
            identity_class_ref: format!("world-object:{}", representation.to_lowercase()),
            review_ref: format!("review:{representation}"),
        },
    }
}

fn residual(reference: &str) -> ProofResidual {
    ProofResidual {
        residual_ref: reference.into(),
        proposition_ref: format!("proposition:{reference}"),
        producer_class_ref: "producer:world-expansion".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: None,
        salience: 100,
        dependency_refs: vec![],
        status: ResidualStatus::Open,
    }
}

fn frontier(residual_refs: &[&str]) -> ProofFrontier {
    ProofFrontier {
        consumer_ref: "consumer:mabo-context-world-identity".into(),
        frontier_ref: "frontier:test".into(),
        residuals: residual_refs.iter().map(|reference| residual(reference)).collect(),
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    }
}

#[test]
fn selects_exactly_one_current_open_reviewed_residual_via_frontier_pareto() {
    let plan = MaboIdentityReviewPlan {
        matched: vec![
            planned("Q2", "r:q2", &["context:wikidata:judge"]),
            planned(
                "Q1",
                "r:q1",
                &["context:wikidata:participant", "context:wikidata:jurisdiction"],
            ),
        ],
        pending_rows: vec![],
        unmatched_assignments: vec![],
    };

    let selected = select_next_reviewed_mabo_gap(&frontier(&["r:q1", "r:q2"]), &plan)
        .expect("one reviewed frontier move should be selected");

    assert_eq!(selected.residual_ref, "r:q1");
    assert_eq!(selected.representation_ref, "Q1");
    assert_eq!(selected.shared_dependency_gain, 2);
}

#[test]
fn rebuilt_frontier_changes_the_next_selection_instead_of_consuming_a_prefilled_queue() {
    let plan = MaboIdentityReviewPlan {
        matched: vec![
            planned(
                "Q1",
                "r:q1",
                &["context:wikidata:participant", "context:wikidata:jurisdiction"],
            ),
            planned("Q2", "r:q2", &["context:wikidata:judge"]),
        ],
        pending_rows: vec![],
        unmatched_assignments: vec![],
    };

    let first = select_next_reviewed_mabo_gap(&frontier(&["r:q1", "r:q2"]), &plan).unwrap();
    assert_eq!(first.representation_ref, "Q1");

    let second = select_next_reviewed_mabo_gap(&frontier(&["r:q2"]), &plan).unwrap();
    assert_eq!(second.representation_ref, "Q2");
    assert_eq!(second.residual_ref, "r:q2");
}

#[test]
fn reviewed_rows_not_present_as_open_current_residuals_are_not_selectable() {
    let plan = MaboIdentityReviewPlan {
        matched: vec![planned("Q1", "r:q1", &["context:wikidata:participant"])],
        pending_rows: vec![],
        unmatched_assignments: vec![],
    };

    assert!(select_next_reviewed_mabo_gap(&frontier(&["r:other"]), &plan).is_none());
}
