use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SemanticNodeKind {
    Source,
    Support,
    Qualifier,
    Defeater,
    Comparator,
    Temporal,
    Authority,
    Context,
    Residual,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConeCandidate {
    pub node_ref: String,
    pub distance: u32,
    pub elucidatory_score: u32,
    pub kind: SemanticNodeKind,
    pub mandatory: bool,
    pub provenance_refs: Vec<String>,
    pub source_span_refs: Vec<String>,
    pub residual_ref: Option<String>,
}

impl ConeCandidate {
    #[must_use]
    pub fn new(
        node_ref: impl Into<String>,
        distance: u32,
        elucidatory_score: u32,
        kind: SemanticNodeKind,
    ) -> Self {
        Self {
            node_ref: node_ref.into(),
            distance,
            elucidatory_score,
            kind,
            mandatory: false,
            provenance_refs: Vec::new(),
            source_span_refs: Vec::new(),
            residual_ref: None,
        }
    }

    #[must_use]
    pub fn mandatory(mut self) -> Self {
        self.mandatory = true;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdaptiveConePolicy {
    pub base_depth: u32,
    pub min_elucidatory_score: u32,
    pub max_nodes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveExplanationCone {
    pub focus_ref: String,
    pub nodes: Vec<ConeCandidate>,
    pub omitted_count: usize,
}

/// Keep a candidate when it is locally near, unusually explanatory, or an
/// explicitly mandatory payment/provenance coordinate. The final projection is
/// bounded and deterministic; mandatory coordinates are never displaced by an
/// optional high-scoring candidate.
#[must_use]
pub fn select_adaptive_cone(
    focus_ref: impl Into<String>,
    candidates: &[ConeCandidate],
    policy: AdaptiveConePolicy,
) -> AdaptiveExplanationCone {
    let mut eligible: Vec<ConeCandidate> = candidates
        .iter()
        .filter(|candidate| {
            candidate.mandatory
                || candidate.distance <= policy.base_depth
                || candidate.elucidatory_score >= policy.min_elucidatory_score
        })
        .cloned()
        .collect();
    eligible.sort_by(|left, right| {
        right
            .mandatory
            .cmp(&left.mandatory)
            .then_with(|| left.distance.cmp(&right.distance))
            .then_with(|| right.elucidatory_score.cmp(&left.elucidatory_score))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.node_ref.cmp(&right.node_ref))
    });
    let omitted_count = eligible.len().saturating_sub(policy.max_nodes);
    eligible.truncate(policy.max_nodes);
    AdaptiveExplanationCone {
        focus_ref: focus_ref.into(),
        nodes: eligible,
        omitted_count,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextKind {
    ExactSource,
    Wikipedia,
    Wikidata,
    Historical,
    SemanticFocus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextAuthority {
    PrimaryAuthority,
    BackgroundContext,
    IdentityOnly,
    HistoricalContext,
    SemanticProjection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextLink {
    target_ref: String,
    label: String,
    kind: ContextKind,
    authority: ContextAuthority,
}

impl ContextLink {
    #[must_use]
    pub fn exact_source(target_ref: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(target_ref, label, ContextKind::ExactSource, ContextAuthority::PrimaryAuthority)
    }

    #[must_use]
    pub fn wikipedia(target_ref: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(target_ref, label, ContextKind::Wikipedia, ContextAuthority::BackgroundContext)
    }

    #[must_use]
    pub fn wikidata(target_ref: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(target_ref, label, ContextKind::Wikidata, ContextAuthority::IdentityOnly)
    }

    #[must_use]
    pub fn historical(target_ref: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(target_ref, label, ContextKind::Historical, ContextAuthority::HistoricalContext)
    }

    #[must_use]
    pub fn semantic_focus(target_ref: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(target_ref, label, ContextKind::SemanticFocus, ContextAuthority::SemanticProjection)
    }

    fn new(
        target_ref: impl Into<String>,
        label: impl Into<String>,
        kind: ContextKind,
        authority: ContextAuthority,
    ) -> Self {
        Self { target_ref: target_ref.into(), label: label.into(), kind, authority }
    }

    #[must_use]
    pub const fn kind(&self) -> ContextKind { self.kind }
    #[must_use]
    pub const fn authority(&self) -> ContextAuthority { self.authority }
    #[must_use]
    pub const fn creates_evidence_payment(&self) -> bool { false }
    #[must_use]
    pub const fn creates_applicability(&self) -> bool { false }
    #[must_use]
    pub fn target_ref(&self) -> &str { &self.target_ref }
    #[must_use]
    pub fn label(&self) -> &str { &self.label }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextBundle {
    pub focus_ref: String,
    links: Vec<ContextLink>,
}

impl ContextBundle {
    #[must_use]
    pub fn new(focus_ref: impl Into<String>, links: Vec<ContextLink>) -> Self {
        Self { focus_ref: focus_ref.into(), links }
    }
    #[must_use]
    pub fn links(&self) -> &[ContextLink] { &self.links }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldEdgeKind {
    Supports,
    Qualifies,
    Defeats,
    Compares,
    Temporal,
    Context,
    Cites,
    Related,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldNode {
    pub node_ref: String,
}

impl WorldNode {
    #[must_use]
    pub fn new(node_ref: impl Into<String>) -> Self { Self { node_ref: node_ref.into() } }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldEdge {
    pub from_ref: String,
    pub to_ref: String,
    pub kind: WorldEdgeKind,
}

impl WorldEdge {
    #[must_use]
    pub fn new(from_ref: impl Into<String>, to_ref: impl Into<String>, kind: WorldEdgeKind) -> Self {
        Self { from_ref: from_ref.into(), to_ref: to_ref.into(), kind }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderWorldProjection {
    pub seed_ref: String,
    pub nodes: Vec<WorldNode>,
    pub edges: Vec<WorldEdge>,
    pub max_hops_reached: u32,
    pub omitted_by_budget: usize,
    pub creates_semantic_authority: bool,
}

/// Deterministic read-only breadth-first projection over an already-owned world
/// graph. It creates no persistence, proof payment, or authority.
#[must_use]
pub fn bounded_neighbourhood(
    seed_ref: &str,
    nodes: &[WorldNode],
    edges: &[WorldEdge],
    max_hops: u32,
    max_nodes: usize,
) -> ReaderWorldProjection {
    let node_map: BTreeMap<&str, &WorldNode> = nodes.iter().map(|node| (node.node_ref.as_str(), node)).collect();
    let mut adjacency: BTreeMap<&str, Vec<&WorldEdge>> = BTreeMap::new();
    for edge in edges {
        adjacency.entry(edge.from_ref.as_str()).or_default().push(edge);
        adjacency.entry(edge.to_ref.as_str()).or_default().push(edge);
    }
    for list in adjacency.values_mut() {
        list.sort_by(|left, right| {
            left.from_ref.cmp(&right.from_ref)
                .then_with(|| left.to_ref.cmp(&right.to_ref))
                .then_with(|| (left.kind as u8).cmp(&(right.kind as u8)))
        });
    }

    let mut queue = VecDeque::new();
    let mut visited: BTreeMap<String, u32> = BTreeMap::new();
    if node_map.contains_key(seed_ref) && max_nodes > 0 {
        visited.insert(seed_ref.to_owned(), 0);
        queue.push_back(seed_ref.to_owned());
    }
    let mut max_hops_reached = 0;
    let mut omitted_by_budget = 0;
    while let Some(current) = queue.pop_front() {
        let distance = visited[&current];
        max_hops_reached = max_hops_reached.max(distance);
        if distance >= max_hops { continue; }
        if let Some(outgoing) = adjacency.get(current.as_str()) {
            for edge in outgoing {
                let next = if edge.from_ref == current { &edge.to_ref } else { &edge.from_ref };
                if visited.contains_key(next) || !node_map.contains_key(next.as_str()) { continue; }
                if visited.len() >= max_nodes {
                    omitted_by_budget += 1;
                    continue;
                }
                visited.insert(next.clone(), distance + 1);
                queue.push_back(next.clone());
            }
        }
    }

    let selected_refs: BTreeSet<&str> = visited.keys().map(String::as_str).collect();
    let mut selected_nodes: Vec<WorldNode> = visited
        .keys()
        .filter_map(|node_ref| node_map.get(node_ref.as_str()).map(|node| (*node).clone()))
        .collect();
    selected_nodes.sort_by(|left, right| {
        visited[&left.node_ref]
            .cmp(&visited[&right.node_ref])
            .then_with(|| left.node_ref.cmp(&right.node_ref))
    });
    let mut selected_edges: Vec<WorldEdge> = edges
        .iter()
        .filter(|edge| selected_refs.contains(edge.from_ref.as_str()) && selected_refs.contains(edge.to_ref.as_str()))
        .cloned()
        .collect();
    selected_edges.sort_by(|left, right| left.from_ref.cmp(&right.from_ref).then_with(|| left.to_ref.cmp(&right.to_ref)));

    ReaderWorldProjection {
        seed_ref: seed_ref.to_owned(),
        nodes: selected_nodes,
        edges: selected_edges,
        max_hops_reached,
        omitted_by_budget,
        creates_semantic_authority: false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadingRole {
    ChallengedPremise,
    HistoricalInput,
    AuthorityProposition,
    ImmediateImplication,
    DownstreamApplication,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceCoordinate {
    Paid { source_revision_ref: String, span_ref: String },
    Residual { residual_ref: String },
}

impl SourceCoordinate {
    #[must_use]
    pub const fn is_paid(&self) -> bool { matches!(self, Self::Paid { .. }) }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderPropositionSpec {
    pub proposition_ref: String,
    pub label: String,
    pub role: ReadingRole,
    pub source: SourceCoordinate,
    pub context_refs: Vec<String>,
}

/// Bounded Mabo reading profile matching the five roles formalised in Agda.
/// Only the already-paid radical-title coordinate is source-executable here;
/// the remaining source/proof coordinates stay explicit residuals until their
/// own exact spans and reviewed proposition welds are persisted.
#[must_use]
pub fn mabo_five_stage_registry() -> Vec<ReaderPropositionSpec> {
    vec![
        ReaderPropositionSpec {
            proposition_ref: "mabo:proposition:challenged-premise".into(),
            label: "Challenged premise".into(),
            role: ReadingRole::ChallengedPremise,
            source: SourceCoordinate::Residual { residual_ref: "reader-residual:mabo:challenged-premise-source".into() },
            context_refs: vec!["Q1501525".into()],
        },
        ReaderPropositionSpec {
            proposition_ref: "mabo:proposition:historical-input".into(),
            label: "Historical input".into(),
            role: ReadingRole::HistoricalInput,
            source: SourceCoordinate::Residual { residual_ref: "reader-residual:mabo:historical-input-source".into() },
            context_refs: vec!["context:mabo:history".into()],
        },
        ReaderPropositionSpec {
            proposition_ref: "mabo:proposition:radical-title-native-title".into(),
            label: "Radical title did not automatically confer absolute beneficial ownership".into(),
            role: ReadingRole::AuthorityProposition,
            source: SourceCoordinate::Paid {
                source_revision_ref: "source-revision:mabo:1992:hca:23:wikisource:page-39:rev-16058297:2026-06-29".into(),
                span_ref: "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership".into(),
            },
            context_refs: vec!["source:mabo:1992:hca:23".into(), "Q1501525".into()],
        },
        ReaderPropositionSpec {
            proposition_ref: "mabo:proposition:immediate-implication".into(),
            label: "Immediate implication".into(),
            role: ReadingRole::ImmediateImplication,
            source: SourceCoordinate::Residual { residual_ref: "reader-residual:mabo:immediate-implication-source".into() },
            context_refs: vec![],
        },
        ReaderPropositionSpec {
            proposition_ref: "mabo:proposition:downstream-application".into(),
            label: "Downstream application".into(),
            role: ReadingRole::DownstreamApplication,
            source: SourceCoordinate::Residual { residual_ref: "reader-residual:mabo:downstream-application-source".into() },
            context_refs: vec![],
        },
    ]
}
