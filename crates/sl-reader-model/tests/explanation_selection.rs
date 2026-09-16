use sensiblaw_reader_model::{
    select_from_explanation_cone, AdaptiveConePolicy, ConeCandidate, CoordinateCoverage,
    PropositionPayment, ReaderDisposition, ReaderIntent, ResidualRef, SemanticNodeKind,
    SemanticRef, SourcePayment, SpanRef,
};

#[test]
fn paid_explanation_coordinates_are_mandatory_in_adaptive_selection() {
    let source = SourcePayment::paid(
        SemanticRef::new("prop:test"),
        "source-revision:test:1",
        SpanRef::new("span:test:1"),
    );
    let payment = PropositionPayment::bounded(
        source,
        vec!["observation:support".into()],
        CoordinateCoverage::Residualised(ResidualRef::new("residual:qualifier")),
        CoordinateCoverage::Residualised(ResidualRef::new("residual:defeater")),
        CoordinateCoverage::Residualised(ResidualRef::new("residual:comparator")),
    );
    let ReaderDisposition::ExecuteBoundedWhy(cone) = payment.resolve(ReaderIntent::WhyClaim) else {
        panic!("bounded payment must execute Why");
    };
    let extras = vec![
        ConeCandidate::new("temporal:far-but-useful", 9, 100, SemanticNodeKind::Temporal),
        ConeCandidate::new("context:far-low", 9, 1, SemanticNodeKind::Context),
    ];
    let selected = select_from_explanation_cone(
        &cone,
        &extras,
        AdaptiveConePolicy {
            base_depth: 2,
            min_elucidatory_score: 50,
            max_nodes: 16,
        },
    );
    let refs: Vec<&str> = selected.nodes.iter().map(|node| node.node_ref.as_str()).collect();
    assert!(refs.contains(&"span:test:1"));
    assert!(refs.contains(&"observation:support"));
    assert!(refs.contains(&"residual:qualifier"));
    assert!(refs.contains(&"residual:defeater"));
    assert!(refs.contains(&"residual:comparator"));
    assert!(refs.contains(&"temporal:far-but-useful"));
    assert!(!refs.contains(&"context:far-low"));
}
