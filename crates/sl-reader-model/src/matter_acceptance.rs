//! M13 historical Mary/SensibLaw acceptance diagnostics.
//!
//! This is an operator/product acceptance projection over an already-built
//! MatterWorkspaceProjection. It does not add a new semantic ontology.
//! Party-assertion / procedural-outcome / later-annotation roles are explicit
//! reviewed acceptance coordinates, never inferred from wording or chronology.

use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_core::chronology_contestation::ContestationRelationKind;

use crate::{
    ChronologyPlacementKind, MatterWorkspaceProjection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatterAcceptanceSemanticRole {
    PartyAssertion,
    ProceduralOutcome,
    LaterAnnotation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterAcceptanceRoleCoordinate {
    pub semantic_ref: String,
    pub role: MatterAcceptanceSemanticRole,
    pub review_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterAcceptanceInput {
    pub workspace: MatterWorkspaceProjection,
    /// Explicitly reviewed material that is meaningful to the Matter but has
    /// not been assembled into a semantic event. This cannot be inferred from
    /// absence in one timeline query.
    pub no_event_refs: Vec<String>,
    pub role_coordinates: Vec<MatterAcceptanceRoleCoordinate>,
    /// Review/work coordinates identifying unresolved procedural significance.
    pub procedural_significance_review_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterAcceptanceReceipt {
    pub matter_ref: String,
    pub exact_event_refs: Vec<String>,
    pub approximate_event_refs: Vec<String>,
    pub relative_event_refs: Vec<String>,
    pub undated_event_refs: Vec<String>,
    pub unknown_date_event_refs: Vec<String>,
    pub missing_date_event_refs: Vec<String>,
    pub missing_actor_claim_refs: Vec<String>,
    pub contradictory_relation_refs: Vec<String>,
    pub event_without_source_trace_refs: Vec<String>,
    pub claim_without_source_trace_refs: Vec<String>,
    pub source_trace_without_downstream_refs: Vec<String>,
    pub no_event_refs: Vec<String>,
    pub party_assertion_refs: Vec<String>,
    pub procedural_outcome_refs: Vec<String>,
    pub later_annotation_refs: Vec<String>,
    pub procedural_significance_review_refs: Vec<String>,
    pub operational_carryover_refs: Vec<String>,
    pub source_reopenable_ref_count: usize,
    pub event_entry_count: usize,
    pub knowledge_entry_count: usize,
    pub work_event_count: usize,
    pub suggested_join_count: usize,
    pub review_item_count: usize,
    pub context_exclusion_count: usize,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
    pub canonical_world_mutated: bool,
    pub no_event_means_false: bool,
    pub missing_actor_means_unknown_person: bool,
    pub missing_date_means_event_did_not_happen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatterAcceptanceError {
    EmptyCoordinate(&'static str),
    RoleCoordinateNotVisible(String),
    RoleCoordinateWithoutReview(String),
    DuplicateRoleCoordinate(String),
    RoleCoordinatePromotion(String),
    NoEventRefNotVisible(String),
    ProceduralReviewRefNotVisible(String),
}

impl MatterAcceptanceRoleCoordinate {
    pub fn validate(&self) -> Result<(), MatterAcceptanceError> {
        if self.semantic_ref.trim().is_empty() {
            return Err(MatterAcceptanceError::EmptyCoordinate("semantic_ref"));
        }
        if self.review_ref.trim().is_empty() {
            return Err(MatterAcceptanceError::RoleCoordinateWithoutReview(
                self.semantic_ref.clone(),
            ));
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.claim_truth_promoted
        {
            return Err(MatterAcceptanceError::RoleCoordinatePromotion(
                self.semantic_ref.clone(),
            ));
        }
        Ok(())
    }
}

pub fn project_matter_acceptance(
    input: &MatterAcceptanceInput,
) -> Result<MatterAcceptanceReceipt, MatterAcceptanceError> {
    let visible = input
        .workspace
        .context_projection
        .included_refs
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut role_by_ref = BTreeMap::new();
    for coordinate in &input.role_coordinates {
        coordinate.validate()?;
        if !visible.contains(&coordinate.semantic_ref) {
            return Err(MatterAcceptanceError::RoleCoordinateNotVisible(
                coordinate.semantic_ref.clone(),
            ));
        }
        if role_by_ref
            .insert(coordinate.semantic_ref.clone(), coordinate.role)
            .is_some()
        {
            return Err(MatterAcceptanceError::DuplicateRoleCoordinate(
                coordinate.semantic_ref.clone(),
            ));
        }
    }

    let mut no_event_refs = sorted_unique(&input.no_event_refs)?;
    for reference in &no_event_refs {
        if !visible.contains(reference) {
            return Err(MatterAcceptanceError::NoEventRefNotVisible(
                reference.clone(),
            ));
        }
    }

    let mut procedural_significance_review_refs =
        sorted_unique(&input.procedural_significance_review_refs)?;
    for reference in &procedural_significance_review_refs {
        let visible_review = visible.contains(reference)
            || input
                .workspace
                .review_queue
                .items
                .iter()
                .any(|item| item.review_item_ref == *reference);
        if !visible_review {
            return Err(MatterAcceptanceError::ProceduralReviewRefNotVisible(
                reference.clone(),
            ));
        }
    }

    let mut exact_event_refs = BTreeSet::new();
    let mut approximate_event_refs = BTreeSet::new();
    let mut relative_event_refs = BTreeSet::new();
    let mut undated_event_refs = BTreeSet::new();
    let mut unknown_date_event_refs = BTreeSet::new();
    let mut missing_date_event_refs = BTreeSet::new();
    for entry in &input.workspace.event_timeline.entries {
        match entry.placement {
            ChronologyPlacementKind::Exact => {
                exact_event_refs.insert(entry.event_ref.clone());
            }
            ChronologyPlacementKind::Approximate => {
                approximate_event_refs.insert(entry.event_ref.clone());
            }
            ChronologyPlacementKind::RelativeOnly => {
                relative_event_refs.insert(entry.event_ref.clone());
            }
            ChronologyPlacementKind::Undated => {
                undated_event_refs.insert(entry.event_ref.clone());
                missing_date_event_refs.insert(entry.event_ref.clone());
            }
            ChronologyPlacementKind::Unknown => {
                unknown_date_event_refs.insert(entry.event_ref.clone());
                missing_date_event_refs.insert(entry.event_ref.clone());
            }
        }
    }

    let trace_event_refs = input
        .workspace
        .source_traces
        .iter()
        .filter_map(|trace| trace.event_ref.clone())
        .collect::<BTreeSet<_>>();
    let all_event_refs = input
        .workspace
        .event_timeline
        .entries
        .iter()
        .map(|entry| entry.event_ref.clone())
        .collect::<BTreeSet<_>>();
    let event_without_source_trace_refs = all_event_refs
        .difference(&trace_event_refs)
        .cloned()
        .collect::<Vec<_>>();

    let trace_statement_refs = input
        .workspace
        .source_traces
        .iter()
        .map(|trace| trace.statement.statement_ref.clone())
        .collect::<BTreeSet<_>>();
    let mut claim_without_source_trace_refs = BTreeSet::new();
    let mut missing_actor_claim_refs = BTreeSet::new();
    let mut contradictory_relation_refs = BTreeSet::new();
    for view in &input.workspace.event_timeline.proposition_views {
        for leaf in &view.leaves {
            if leaf.speaker_ref.is_none() {
                missing_actor_claim_refs.insert(leaf.claim_ref.clone());
            }
            if !leaf
                .statement_refs
                .iter()
                .any(|reference| trace_statement_refs.contains(reference))
            {
                claim_without_source_trace_refs.insert(leaf.claim_ref.clone());
            }
        }
        for relation in &view.relations {
            if relation.kind == ContestationRelationKind::Contradicts {
                contradictory_relation_refs.insert(relation.relation_ref.clone());
            }
        }
    }

    let mut source_trace_without_downstream_refs = input
        .workspace
        .source_traces
        .iter()
        .filter(|trace| {
            trace.event_ref.is_none()
                && trace.claim_refs.is_empty()
                && trace.downstream_use_refs.is_empty()
        })
        .map(|trace| trace.statement.statement_ref.clone())
        .collect::<Vec<_>>();
    source_trace_without_downstream_refs.sort();
    source_trace_without_downstream_refs.dedup();

    let party_assertion_refs = role_refs(
        &role_by_ref,
        MatterAcceptanceSemanticRole::PartyAssertion,
    );
    let procedural_outcome_refs = role_refs(
        &role_by_ref,
        MatterAcceptanceSemanticRole::ProceduralOutcome,
    );
    let later_annotation_refs = role_refs(
        &role_by_ref,
        MatterAcceptanceSemanticRole::LaterAnnotation,
    );

    let mut operational_carryover_refs = input
        .workspace
        .operational_outstanding
        .states
        .iter()
        .map(|state| state.operational_state_ref.clone())
        .collect::<Vec<_>>();
    operational_carryover_refs.sort();
    operational_carryover_refs.dedup();

    // Keep deterministic ordering for stable acceptance receipts.
    no_event_refs.sort();
    procedural_significance_review_refs.sort();

    Ok(MatterAcceptanceReceipt {
        matter_ref: input.workspace.matter_ref.clone(),
        exact_event_refs: exact_event_refs.into_iter().collect(),
        approximate_event_refs: approximate_event_refs.into_iter().collect(),
        relative_event_refs: relative_event_refs.into_iter().collect(),
        undated_event_refs: undated_event_refs.into_iter().collect(),
        unknown_date_event_refs: unknown_date_event_refs.into_iter().collect(),
        missing_date_event_refs: missing_date_event_refs.into_iter().collect(),
        missing_actor_claim_refs: missing_actor_claim_refs.into_iter().collect(),
        contradictory_relation_refs: contradictory_relation_refs.into_iter().collect(),
        event_without_source_trace_refs,
        claim_without_source_trace_refs:
            claim_without_source_trace_refs.into_iter().collect(),
        source_trace_without_downstream_refs,
        no_event_refs,
        party_assertion_refs,
        procedural_outcome_refs,
        later_annotation_refs,
        procedural_significance_review_refs,
        operational_carryover_refs,
        source_reopenable_ref_count: input.workspace.source_traces.len(),
        event_entry_count: input.workspace.event_timeline.entries.len(),
        knowledge_entry_count: input.workspace.knowledge_timeline.len(),
        work_event_count: input.workspace.operational_timeline.entries.len(),
        suggested_join_count: input.workspace.join_proposals.proposals.len(),
        review_item_count: input.workspace.review_queue.items.len(),
        context_exclusion_count: input.workspace.context_projection.exclusions.len(),
        creates_semantic_authority: false,
        claim_truth_promoted: false,
        canonical_world_mutated: false,
        no_event_means_false: false,
        missing_actor_means_unknown_person: false,
        missing_date_means_event_did_not_happen: false,
    })
}

fn sorted_unique(values: &[String]) -> Result<Vec<String>, MatterAcceptanceError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(MatterAcceptanceError::EmptyCoordinate("acceptance_refs"));
    }
    Ok(values
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

fn role_refs(
    roles: &BTreeMap<String, MatterAcceptanceSemanticRole>,
    wanted: MatterAcceptanceSemanticRole,
) -> Vec<String> {
    roles
        .iter()
        .filter_map(|(reference, role)| (*role == wanted).then(|| reference.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChronologyProjection, EventDiscoveryProjection,
        OperationalOutstandingProjection, OperationalTimelineProjection,
        ReviewQueueProjection,
    };
    use sensiblaw_core::matter_context::MatterContextProjection;

    fn empty_workspace() -> MatterWorkspaceProjection {
        MatterWorkspaceProjection {
            matter_ref: "matter:mary".into(),
            context_projection: MatterContextProjection {
                matter_ref: "matter:mary".into(),
                included_refs: vec![
                    "statement:no-event".into(),
                    "claim:party".into(),
                    "event:outcome".into(),
                    "annotation:later".into(),
                    "review:procedure".into(),
                ],
                exclusions: vec![],
                canonical_world_mutated: false,
                invisibility_means_false: false,
                unshared_means_absent: false,
                role_visibility_creates_truth: false,
                later_knowledge_rewrites_cut: false,
            },
            source_traces: vec![],
            event_timeline: ChronologyProjection::default(),
            knowledge_timeline: vec![],
            operational_timeline: OperationalTimelineProjection::default(),
            operational_outstanding: OperationalOutstandingProjection::default(),
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
            creates_semantic_authority: false,
            claim_truth_promoted: false,
            canonical_world_mutated: false,
        }
    }

    #[test]
    fn explicit_mary_roles_remain_distinct_and_non_promoting() {
        let receipt = project_matter_acceptance(&MatterAcceptanceInput {
            workspace: empty_workspace(),
            no_event_refs: vec!["statement:no-event".into()],
            role_coordinates: vec![
                MatterAcceptanceRoleCoordinate {
                    semantic_ref: "claim:party".into(),
                    role: MatterAcceptanceSemanticRole::PartyAssertion,
                    review_ref: "review:party".into(),
                    candidate_only: true,
                    creates_semantic_authority: false,
                    claim_truth_promoted: false,
                },
                MatterAcceptanceRoleCoordinate {
                    semantic_ref: "event:outcome".into(),
                    role: MatterAcceptanceSemanticRole::ProceduralOutcome,
                    review_ref: "review:outcome".into(),
                    candidate_only: true,
                    creates_semantic_authority: false,
                    claim_truth_promoted: false,
                },
                MatterAcceptanceRoleCoordinate {
                    semantic_ref: "annotation:later".into(),
                    role: MatterAcceptanceSemanticRole::LaterAnnotation,
                    review_ref: "review:annotation".into(),
                    candidate_only: true,
                    creates_semantic_authority: false,
                    claim_truth_promoted: false,
                },
            ],
            procedural_significance_review_refs: vec!["review:procedure".into()],
        })
        .unwrap();

        assert_eq!(receipt.party_assertion_refs, vec!["claim:party"]);
        assert_eq!(receipt.procedural_outcome_refs, vec!["event:outcome"]);
        assert_eq!(receipt.later_annotation_refs, vec!["annotation:later"]);
        assert_eq!(receipt.no_event_refs, vec!["statement:no-event"]);
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.claim_truth_promoted);
        assert!(!receipt.no_event_means_false);
    }

    #[test]
    fn navigation_gaps_are_explicit_acceptance_debt() {
        let mut workspace = empty_workspace();
        workspace.event_timeline.entries.push(crate::ChronologyEntry {
            event_ref: "event:no-source".into(),
            temporal_ref: None,
            placement: ChronologyPlacementKind::Unknown,
            display_coordinate: "unknown".into(),
            relative_event_ref: None,
            observation_refs: vec![],
            statement_refs: vec![],
            proposition_refs: vec![],
            claim_refs: vec![],
            contestation_relation_refs: vec![],
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        });

        let receipt = project_matter_acceptance(&MatterAcceptanceInput {
            workspace,
            no_event_refs: vec![],
            role_coordinates: vec![],
            procedural_significance_review_refs: vec![],
        })
        .unwrap();

        assert_eq!(
            receipt.event_without_source_trace_refs,
            vec!["event:no-source"]
        );
        assert_eq!(receipt.unknown_date_event_refs, vec!["event:no-source"]);
        assert_eq!(receipt.missing_date_event_refs, vec!["event:no-source"]);
        assert!(!receipt.missing_date_means_event_did_not_happen);
    }

    #[test]
    fn one_semantic_ref_cannot_silently_fill_two_acceptance_roles() {
        let shared = MatterAcceptanceRoleCoordinate {
            semantic_ref: "claim:party".into(),
            role: MatterAcceptanceSemanticRole::PartyAssertion,
            review_ref: "review:a".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        };
        let mut second = shared.clone();
        second.role = MatterAcceptanceSemanticRole::ProceduralOutcome;
        second.review_ref = "review:b".into();

        assert_eq!(
            project_matter_acceptance(&MatterAcceptanceInput {
                workspace: empty_workspace(),
                no_event_refs: vec![],
                role_coordinates: vec![shared, second],
                procedural_significance_review_refs: vec![],
            }),
            Err(MatterAcceptanceError::DuplicateRoleCoordinate(
                "claim:party".into()
            ))
        );
    }
}
