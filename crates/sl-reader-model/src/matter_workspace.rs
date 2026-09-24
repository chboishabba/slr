//! S30.B generic Matter composition.
//!
//! This module composes existing reader projections under one MatterContext.
//! It does not create a new persistence model and it does not infer missing
//! knowledge-time or semantic identities.

use std::collections::BTreeSet;

use sensiblaw_core::matter_context::{
    project_matter_context, ContextProjectionCoordinate, KnowledgeCutMembership,
    MatterContext, MatterContextError, MatterContextProjection,
};

use crate::{
    ChronologyProjection, EventDiscoveryProjection, OperationalTimelineProjection,
    ReviewQueueProjection, SemanticTracePath,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeTimelineEntry {
    pub semantic_ref: String,
    pub source_revision_ref: String,
    pub knowledge_time_ref: String,
    pub knowledge_membership: KnowledgeCutMembership,
    pub source_role_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterWorkspaceInput {
    pub matter_ref: String,
    pub context: MatterContext,
    pub context_coordinates: Vec<ContextProjectionCoordinate>,
    pub source_traces: Vec<SemanticTracePath>,
    pub event_timeline: ChronologyProjection,
    pub knowledge_timeline: Vec<KnowledgeTimelineEntry>,
    pub operational_timeline: OperationalTimelineProjection,
    pub join_proposals: EventDiscoveryProjection,
    pub review_queue: ReviewQueueProjection,
    pub legal_proof_refs: Vec<String>,
    pub research_refs: Vec<String>,
    pub work_product_refs: Vec<String>,
    pub handoff_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterWorkspaceProjection {
    pub matter_ref: String,
    pub context_projection: MatterContextProjection,
    pub source_traces: Vec<SemanticTracePath>,
    pub event_timeline: ChronologyProjection,
    pub knowledge_timeline: Vec<KnowledgeTimelineEntry>,
    pub operational_timeline: OperationalTimelineProjection,
    pub join_proposals: EventDiscoveryProjection,
    pub review_queue: ReviewQueueProjection,
    pub legal_proof_refs: Vec<String>,
    pub research_refs: Vec<String>,
    pub work_product_refs: Vec<String>,
    pub handoff_refs: Vec<String>,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
    pub canonical_world_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatterWorkspaceError {
    EmptyMatterRef,
    ContextMatterMismatch,
    ContextProjection(MatterContextError),
    InvalidSourceTrace(String),
    PromotionBoundary,
    EmptyKnowledgeCoordinate(&'static str),
    DuplicateKnowledgeSemanticRef(String),
}

impl From<MatterContextError> for MatterWorkspaceError {
    fn from(value: MatterContextError) -> Self {
        Self::ContextProjection(value)
    }
}

impl KnowledgeTimelineEntry {
    pub fn validate(&self) -> Result<(), MatterWorkspaceError> {
        for (name, value) in [
            ("semantic_ref", self.semantic_ref.as_str()),
            ("source_revision_ref", self.source_revision_ref.as_str()),
            ("knowledge_time_ref", self.knowledge_time_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(MatterWorkspaceError::EmptyKnowledgeCoordinate(name));
            }
        }
        if self
            .source_role_ref
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(MatterWorkspaceError::EmptyKnowledgeCoordinate(
                "source_role_ref",
            ));
        }
        Ok(())
    }
}

pub fn project_matter_workspace(
    input: &MatterWorkspaceInput,
) -> Result<MatterWorkspaceProjection, MatterWorkspaceError> {
    if input.matter_ref.trim().is_empty() {
        return Err(MatterWorkspaceError::EmptyMatterRef);
    }
    if input.context.matter_ref != input.matter_ref {
        return Err(MatterWorkspaceError::ContextMatterMismatch);
    }

    let context_projection =
        project_matter_context(&input.context, &input.context_coordinates)?;
    let visible = context_projection
        .included_refs
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    if input.event_timeline.creates_semantic_authority
        || input.event_timeline.claim_truth_promoted
        || input.operational_timeline.creates_semantic_authority
        || input.operational_timeline.pays_evidence
        || input.operational_timeline.claim_truth_promoted
        || input.join_proposals.creates_event_identity
        || input.join_proposals.creates_semantic_authority
        || input.join_proposals.claim_truth_promoted
        || input.review_queue.creates_semantic_authority
        || input.review_queue.applicability_promoted
        || input.review_queue.claim_truth_promoted
    {
        return Err(MatterWorkspaceError::PromotionBoundary);
    }

    let source_traces = input
        .source_traces
        .iter()
        .filter_map(|trace| {
            if trace.validate().is_err() {
                return None;
            }
            let visible_trace = visible.contains(trace.focus_ref.as_str())
                || visible.contains(trace.statement.statement_ref.as_str())
                || visible.contains(trace.statement.source_revision_ref.as_str());
            visible_trace.then(|| trace.clone())
        })
        .collect::<Vec<_>>();

    for trace in &input.source_traces {
        trace
            .validate()
            .map_err(|_| MatterWorkspaceError::InvalidSourceTrace(trace.focus_ref.clone()))?;
    }

    let mut event_timeline = input.event_timeline.clone();
    event_timeline
        .entries
        .retain(|entry| visible.contains(entry.event_ref.as_str()));
    event_timeline.proposition_views.retain(|view| {
        visible.contains(view.root.proposition_ref.as_str())
            || view
                .leaves
                .iter()
                .any(|claim| visible.contains(claim.claim_ref.as_str()))
    });

    let mut knowledge_seen = BTreeSet::new();
    let mut knowledge_timeline = Vec::new();
    for entry in &input.knowledge_timeline {
        entry.validate()?;
        if !knowledge_seen.insert(entry.semantic_ref.clone()) {
            return Err(MatterWorkspaceError::DuplicateKnowledgeSemanticRef(
                entry.semantic_ref.clone(),
            ));
        }
        if visible.contains(entry.semantic_ref.as_str())
            || visible.contains(entry.source_revision_ref.as_str())
        {
            knowledge_timeline.push(entry.clone());
        }
    }
    knowledge_timeline.sort_by(|left, right| {
        left.knowledge_time_ref
            .cmp(&right.knowledge_time_ref)
            .then_with(|| left.semantic_ref.cmp(&right.semantic_ref))
    });

    let mut operational_timeline = input.operational_timeline.clone();
    operational_timeline.entries.retain(|entry| {
        visible.contains(entry.event.operational_event_ref.as_str())
            || entry
                .target_refs
                .iter()
                .any(|target| visible.contains(target.as_str()))
    });
    operational_timeline.linked_target_count = operational_timeline
        .entries
        .iter()
        .flat_map(|entry| entry.target_refs.iter().cloned())
        .collect::<BTreeSet<_>>()
        .len();

    let mut join_proposals = input.join_proposals.clone();
    join_proposals.proposals.retain(|view| {
        visible.contains(view.proposal.proposal_ref.as_str())
            || view
                .proposal
                .observation_refs
                .iter()
                .any(|reference| visible.contains(reference.as_str()))
            || view
                .proposal
                .statement_refs
                .iter()
                .any(|reference| visible.contains(reference.as_str()))
    });

    let mut review_queue = input.review_queue.clone();
    review_queue.items.retain(|item| {
        visible.contains(item.review_item_ref.as_str())
            || visible.contains(item.semantic_ref.as_str())
            || item
                .source_refs
                .iter()
                .any(|reference| visible.contains(reference.as_str()))
    });
    // Keep count maps honest after context filtering.
    review_queue.count_by_kind.clear();
    review_queue.count_by_status.clear();
    for item in &review_queue.items {
        *review_queue.count_by_kind.entry(item.item_kind).or_insert(0) += 1;
        *review_queue
            .count_by_status
            .entry(crate::ReviewStatusKey::from(item.current_status))
            .or_insert(0) += 1;
    }

    Ok(MatterWorkspaceProjection {
        matter_ref: input.matter_ref.clone(),
        context_projection,
        source_traces,
        event_timeline,
        knowledge_timeline,
        operational_timeline,
        join_proposals,
        review_queue,
        legal_proof_refs: filter_refs(&input.legal_proof_refs, &visible),
        research_refs: filter_refs(&input.research_refs, &visible),
        work_product_refs: filter_refs(&input.work_product_refs, &visible),
        handoff_refs: filter_refs(&input.handoff_refs, &visible),
        creates_semantic_authority: false,
        claim_truth_promoted: false,
        canonical_world_mutated: false,
    })
}

fn filter_refs(values: &[String], visible: &BTreeSet<String>) -> Vec<String> {
    let mut out = values
        .iter()
        .filter(|value| visible.contains(value.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::matter_context::{
        DisclosureBoundary, MatterConsumerRole, MatterPurpose,
    };

    fn context() -> MatterContext {
        MatterContext {
            matter_ref: "matter:1".into(),
            purpose_ref: MatterPurpose::LegalAdvocacy,
            active_consumer_role: MatterConsumerRole::Lawyer,
            disclosure_boundary: DisclosureBoundary::RoleScoped,
            knowledge_time_cut_ref: Some("cut:1".into()),
            sealed_refs: vec!["event:sealed".into()],
            minimum_necessary: true,
            purpose_limited: true,
            access_logged: true,
            revocable: true,
            mutates_canonical_world: false,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        }
    }

    fn coordinate(reference: &str) -> ContextProjectionCoordinate {
        ContextProjectionCoordinate {
            semantic_ref: reference.into(),
            matter_ref: "matter:1".into(),
            allowed_roles: vec![MatterConsumerRole::Lawyer],
            allowed_purposes: vec![MatterPurpose::LegalAdvocacy],
            knowledge_membership: KnowledgeCutMembership::KnownAtCut,
            explicitly_selected: true,
            sealed: false,
        }
    }

    #[test]
    fn context_filters_all_matter_dimensions_without_mutating_world() {
        let input = MatterWorkspaceInput {
            matter_ref: "matter:1".into(),
            context: context(),
            context_coordinates: vec![
                coordinate("event:visible"),
                coordinate("source-revision:visible"),
                coordinate("operational:visible"),
                coordinate("proposal:visible"),
                coordinate("review:visible"),
                coordinate("proof:visible"),
            ],
            source_traces: vec![],
            event_timeline: ChronologyProjection::default(),
            knowledge_timeline: vec![KnowledgeTimelineEntry {
                semantic_ref: "source-revision:visible".into(),
                source_revision_ref: "source-revision:visible".into(),
                knowledge_time_ref: "knowledge:2026-09-01".into(),
                knowledge_membership: KnowledgeCutMembership::KnownAtCut,
                source_role_ref: Some("memoir".into()),
            }],
            operational_timeline: OperationalTimelineProjection::default(),
            join_proposals: EventDiscoveryProjection::default(),
            review_queue: ReviewQueueProjection {
                items: vec![],
                count_by_kind: Default::default(),
                count_by_status: Default::default(),
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
            legal_proof_refs: vec!["proof:visible".into(), "proof:hidden".into()],
            research_refs: vec![],
            work_product_refs: vec![],
            handoff_refs: vec![],
        };

        let projection = project_matter_workspace(&input).unwrap();
        assert_eq!(projection.knowledge_timeline.len(), 1);
        assert_eq!(projection.legal_proof_refs, vec!["proof:visible"]);
        assert!(!projection.canonical_world_mutated);
        assert!(!projection.creates_semantic_authority);
        assert!(!projection.claim_truth_promoted);
    }

    #[test]
    fn sealed_coordinate_is_excluded_from_workspace_projection() {
        let input = MatterWorkspaceInput {
            matter_ref: "matter:1".into(),
            context: context(),
            context_coordinates: vec![coordinate("event:sealed")],
            source_traces: vec![],
            event_timeline: ChronologyProjection::default(),
            knowledge_timeline: vec![],
            operational_timeline: OperationalTimelineProjection::default(),
            join_proposals: EventDiscoveryProjection::default(),
            review_queue: ReviewQueueProjection {
                items: vec![],
                count_by_kind: Default::default(),
                count_by_status: Default::default(),
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
            legal_proof_refs: vec![],
            research_refs: vec![],
            work_product_refs: vec![],
            handoff_refs: vec![],
        };
        let projection = project_matter_workspace(&input).unwrap();
        assert!(projection.context_projection.included_refs.is_empty());
        assert_eq!(projection.context_projection.exclusions.len(), 1);
    }
}
