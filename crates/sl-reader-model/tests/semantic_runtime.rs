use sensiblaw_reader_model::{
    bounded_neighbourhood, mabo_five_stage_registry, select_adaptive_cone, AdaptiveConePolicy,
    ConeCandidate, ContextAuthority, ContextBundle, ContextKind, ContextLink, ReadingRole,
    ReaderPropositionSpec, SemanticNodeKind, SourceCoordinate, WorldEdge, WorldEdgeKind, WorldNode,
};

#[test]
fn adaptive_cone_keeps_near_and_far_high_value_but_drops_far_low_value() {
    let candidates = vec![
        ConeCandidate::new("near", 1, 1, SemanticNodeKind::Support),
        ConeCandidate::new("far-high", 8, 100, SemanticNodeKind::Temporal),
        ConeCandidate::new("far-low", 8, 1, SemanticNodeKind::Context),
    ];
    let cone = select_adaptive_cone(
        "focus",
        &candidates,
        AdaptiveConePolicy {
            base_depth: 2,
            min_elucidatory_score: 50,
            max_nodes: 8,
        },
    );
    let refs: Vec<&str> = cone.nodes.iter().map(|node| node.node_ref.as_str()).collect();
    assert!(refs.contains(&"near"));
    assert!(refs.contains(&"far-high"));
    assert!(!refs.contains(&"far-low"));
}

#[test]
fn context_navigation_keeps_identity_background_and_authority_distinct() {
    let bundle = ContextBundle::new(
        "mabo:proposition:radical-title-native-title",
        vec![
            ContextLink::exact_source("source:mabo:1992:hca:23", "High Court source"),
            ContextLink::wikidata("Q1501525", "Mabo identity"),
            ContextLink::wikipedia(
                "wiki:en:Mabo_v_Queensland_(No_2)",
                "Background context",
            ),
            ContextLink::historical("context:mabo:history", "Historical context"),
        ],
    );
    assert!(bundle.links().iter().any(|link| {
        link.kind() == ContextKind::ExactSource
            && link.authority() == ContextAuthority::PrimaryAuthority
    }));
    assert!(bundle.links().iter().filter(|link| {
        matches!(link.kind(), ContextKind::Wikipedia | ContextKind::Wikidata)
    }).all(|link| {
        link.authority() != ContextAuthority::PrimaryAuthority
            && !link.creates_evidence_payment()
            && !link.creates_applicability()
    }));
}

#[test]
fn latent_world_projection_is_bounded_at_one_hundred_hops() {
    let nodes: Vec<WorldNode> = (0..=120)
        .map(|index| WorldNode::new(format!("n:{index}")))
        .collect();
    let edges: Vec<WorldEdge> = (0..120)
        .map(|index| WorldEdge::new(
            format!("n:{index}"),
            format!("n:{}", index + 1),
            WorldEdgeKind::Related,
        ))
        .collect();
    let projection = bounded_neighbourhood("n:0", &nodes, &edges, 100, 1000);
    assert!(projection.nodes.iter().any(|node| node.node_ref == "n:100"));
    assert!(!projection.nodes.iter().any(|node| node.node_ref == "n:101"));
    assert!(projection.max_hops_reached <= 100);
    assert!(!projection.creates_semantic_authority);
}

#[test]
fn mabo_registry_has_five_reopenable_roles_but_only_paid_source_is_executable() {
    let registry = mabo_five_stage_registry();
    assert_eq!(registry.len(), 5);
    assert_eq!(registry[0].role, ReadingRole::ChallengedPremise);
    assert_eq!(registry[1].role, ReadingRole::HistoricalInput);
    assert_eq!(registry[2].role, ReadingRole::AuthorityProposition);
    assert_eq!(registry[3].role, ReadingRole::ImmediateImplication);
    assert_eq!(registry[4].role, ReadingRole::DownstreamApplication);
    assert_eq!(registry.iter().filter(|spec| spec.source.is_paid()).count(), 1);
}

#[test]
fn same_specimen_types_support_non_mabo_material() {
    let spec = ReaderPropositionSpec {
        proposition_ref: "research:paper:claim:1".into(),
        label: "A research-paper claim".into(),
        role: ReadingRole::AuthorityProposition,
        source: SourceCoordinate::Residual {
            residual_ref: "reader-residual:research-source-span".into(),
        },
        context_refs: vec!["context:paper:abstract".into()],
    };
    assert_eq!(spec.proposition_ref, "research:paper:claim:1");
    assert!(!spec.source.is_paid());
}
