//! S28 chronology/contestation reader projection.
//!
//! This module places already-owned temporal assertions into operator buckets.
//! It does not infer missing dates, merge competing accounts, or create
//! semantic/evidential authority.

use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_core::chronology_contestation::{
    ClaimLeaf, ContestationRelation, PropositionRoot, TemporalAssertion, TemporalForm,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChronologyPlacementKind {
    Exact,
    Approximate,
    RelativeOnly,
    Undated,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventChronologyInput {
    pub event_ref: String,
    pub temporal_assertions: Vec<TemporalAssertion>,
    pub observation_refs: Vec<String>,
    pub statement_refs: Vec<String>,
    pub proposition_refs: Vec<String>,
    pub claim_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChronologyEntry {
    pub event_ref: String,
    pub temporal_ref: Option<String>,
    pub placement: ChronologyPlacementKind,
    pub display_coordinate: String,
    pub relative_event_ref: Option<String>,
    pub observation_refs: Vec<String>,
    pub statement_refs: Vec<String>,
    pub proposition_refs: Vec<String>,
    pub claim_refs: Vec<String>,
    pub contestation_relation_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

impl ChronologyEntry {
    #[must_use]
    pub fn has_contestation(&self) -> bool {
        !self.contestation_relation_refs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionContestationView {
    pub root: PropositionRoot,
    pub leaves: Vec<ClaimLeaf>,
    pub relations: Vec<ContestationRelation>,
    pub orphan_relation_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChronologyProjection {
    pub entries: Vec<ChronologyEntry>,
    pub proposition_views: Vec<PropositionContestationView>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChronologyProjectionError {
    EmptyEventRef,
    EventPromotionBoundary,
    InvalidTemporalAssertion(String),
    InvalidProposition(String),
    InvalidClaim(String),
    InvalidRelation(String),
    ClaimReferencesUnknownProposition(String),
}

fn placement_for(form: &TemporalForm) -> (ChronologyPlacementKind, String, Option<String>) {
    match form {
        TemporalForm::ExactInstant { instant_ref } => {
            (ChronologyPlacementKind::Exact, instant_ref.clone(), None)
        }
        TemporalForm::ExactDate { date_ref } => {
            (ChronologyPlacementKind::Exact, date_ref.clone(), None)
        }
        TemporalForm::Interval { start_ref, end_ref } => (
            ChronologyPlacementKind::Exact,
            format!("{start_ref}..{end_ref}"),
            None,
        ),
        TemporalForm::Approximate { label } => {
            (ChronologyPlacementKind::Approximate, label.clone(), None)
        }
        TemporalForm::RelativeBefore { event_ref } => (
            ChronologyPlacementKind::RelativeOnly,
            format!("before:{event_ref}"),
            Some(event_ref.clone()),
        ),
        TemporalForm::RelativeAfter { event_ref } => (
            ChronologyPlacementKind::RelativeOnly,
            format!("after:{event_ref}"),
            Some(event_ref.clone()),
        ),
        TemporalForm::Contemporaneous { event_ref } => (
            ChronologyPlacementKind::RelativeOnly,
            format!("contemporaneous:{event_ref}"),
            Some(event_ref.clone()),
        ),
        TemporalForm::Undated => (
            ChronologyPlacementKind::Undated,
            "undated".into(),
            None,
        ),
        TemporalForm::Unknown => (
            ChronologyPlacementKind::Unknown,
            "unknown".into(),
            None,
        ),
    }
}

fn sorted_unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let set: BTreeSet<String> = values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect();
    set.into_iter().collect()
}

pub fn project_proposition_contestation(
    roots: &[PropositionRoot],
    claims: &[ClaimLeaf],
    relations: &[ContestationRelation],
) -> Result<Vec<PropositionContestationView>, ChronologyProjectionError> {
    let mut roots_by_ref = BTreeMap::new();
    for root in roots {
        root.validate()
            .map_err(|_| ChronologyProjectionError::InvalidProposition(root.proposition_ref.clone()))?;
        roots_by_ref.insert(root.proposition_ref.clone(), root.clone());
    }

    let mut claims_by_ref = BTreeMap::new();
    let mut claims_by_root: BTreeMap<String, Vec<ClaimLeaf>> = BTreeMap::new();
    for claim in claims {
        claim.validate()
            .map_err(|_| ChronologyProjectionError::InvalidClaim(claim.claim_ref.clone()))?;
        if !roots_by_ref.contains_key(&claim.proposition_ref) {
            return Err(ChronologyProjectionError::ClaimReferencesUnknownProposition(
                claim.claim_ref.clone(),
            ));
        }
        claims_by_ref.insert(claim.claim_ref.clone(), claim.clone());
        claims_by_root
            .entry(claim.proposition_ref.clone())
            .or_default()
            .push(claim.clone());
    }

    let mut relation_refs_by_root: BTreeMap<String, Vec<ContestationRelation>> = BTreeMap::new();
    let mut orphan_relation_refs_by_root: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for relation in relations {
        relation
            .validate()
            .map_err(|_| ChronologyProjectionError::InvalidRelation(relation.relation_ref.clone()))?;

        let left_root = claims_by_ref
            .get(&relation.from_claim_ref)
            .map(|claim| claim.proposition_ref.clone());
        let right_root = claims_by_ref
            .get(&relation.to_claim_ref)
            .map(|claim| claim.proposition_ref.clone());

        match (left_root, right_root) {
            (Some(left), Some(right)) if left == right => {
                relation_refs_by_root
                    .entry(left)
                    .or_default()
                    .push(relation.clone());
            }
            (Some(left), Some(right)) => {
                // Cross-root relations are retained against both roots rather
                // than silently merging the proposition identities.
                relation_refs_by_root
                    .entry(left.clone())
                    .or_default()
                    .push(relation.clone());
                relation_refs_by_root
                    .entry(right)
                    .or_default()
                    .push(relation.clone());
            }
            (Some(root), None) | (None, Some(root)) => {
                orphan_relation_refs_by_root
                    .entry(root)
                    .or_default()
                    .push(relation.relation_ref.clone());
            }
            (None, None) => {}
        }
    }

    let mut views = Vec::new();
    for (proposition_ref, root) in roots_by_ref {
        let mut leaves = claims_by_root.remove(&proposition_ref).unwrap_or_default();
        leaves.sort_by(|left, right| left.claim_ref.cmp(&right.claim_ref));

        let mut attached_relations = relation_refs_by_root
            .remove(&proposition_ref)
            .unwrap_or_default();
        attached_relations.sort_by(|left, right| left.relation_ref.cmp(&right.relation_ref));
        attached_relations.dedup_by(|left, right| left.relation_ref == right.relation_ref);

        let mut orphan_relation_refs = orphan_relation_refs_by_root
            .remove(&proposition_ref)
            .unwrap_or_default();
        orphan_relation_refs.sort();
        orphan_relation_refs.dedup();

        views.push(PropositionContestationView {
            root,
            leaves,
            relations: attached_relations,
            orphan_relation_refs,
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        });
    }
    views.sort_by(|left, right| left.root.proposition_ref.cmp(&right.root.proposition_ref));
    Ok(views)
}

pub fn project_chronology(
    events: &[EventChronologyInput],
    roots: &[PropositionRoot],
    claims: &[ClaimLeaf],
    relations: &[ContestationRelation],
) -> Result<ChronologyProjection, ChronologyProjectionError> {
    let proposition_views = project_proposition_contestation(roots, claims, relations)?;

    let relation_refs_by_claim: BTreeMap<String, Vec<String>> = {
        let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for relation in relations {
            relation
                .validate()
                .map_err(|_| ChronologyProjectionError::InvalidRelation(relation.relation_ref.clone()))?;
            out.entry(relation.from_claim_ref.clone())
                .or_default()
                .push(relation.relation_ref.clone());
            out.entry(relation.to_claim_ref.clone())
                .or_default()
                .push(relation.relation_ref.clone());
        }
        out
    };

    let mut entries = Vec::new();
    for event in events {
        if event.event_ref.trim().is_empty() {
            return Err(ChronologyProjectionError::EmptyEventRef);
        }
        if !event.candidate_only
            || event.creates_semantic_authority
            || event.claim_truth_promoted
        {
            return Err(ChronologyProjectionError::EventPromotionBoundary);
        }

        let relation_refs = sorted_unique(event.claim_refs.iter().flat_map(|claim_ref| {
            relation_refs_by_claim
                .get(claim_ref)
                .cloned()
                .unwrap_or_default()
        }));

        let observations = sorted_unique(event.observation_refs.clone());
        let statements = sorted_unique(event.statement_refs.clone());
        let propositions = sorted_unique(event.proposition_refs.clone());
        let claims = sorted_unique(event.claim_refs.clone());

        if event.temporal_assertions.is_empty() {
            entries.push(ChronologyEntry {
                event_ref: event.event_ref.clone(),
                temporal_ref: None,
                placement: ChronologyPlacementKind::Unknown,
                display_coordinate: "unknown".into(),
                relative_event_ref: None,
                observation_refs: observations,
                statement_refs: statements,
                proposition_refs: propositions,
                claim_refs: claims,
                contestation_relation_refs: relation_refs,
                candidate_only: true,
                creates_semantic_authority: false,
                claim_truth_promoted: false,
            });
            continue;
        }

        for temporal in &event.temporal_assertions {
            temporal.validate().map_err(|_| {
                ChronologyProjectionError::InvalidTemporalAssertion(temporal.temporal_ref.clone())
            })?;
            let (placement, display_coordinate, relative_event_ref) =
                placement_for(&temporal.form);
            entries.push(ChronologyEntry {
                event_ref: event.event_ref.clone(),
                temporal_ref: Some(temporal.temporal_ref.clone()),
                placement,
                display_coordinate,
                relative_event_ref,
                observation_refs: sorted_unique(
                    observations
                        .iter()
                        .cloned()
                        .chain(temporal.observation_refs.iter().cloned()),
                ),
                statement_refs: sorted_unique(
                    statements
                        .iter()
                        .cloned()
                        .chain(temporal.statement_refs.iter().cloned()),
                ),
                proposition_refs: propositions.clone(),
                claim_refs: claims.clone(),
                contestation_relation_refs: relation_refs.clone(),
                candidate_only: true,
                creates_semantic_authority: false,
                claim_truth_promoted: false,
            });
        }
    }

    entries.sort_by(|left, right| {
        left.placement
            .cmp(&right.placement)
            .then_with(|| left.display_coordinate.cmp(&right.display_coordinate))
            .then_with(|| left.event_ref.cmp(&right.event_ref))
            .then_with(|| left.temporal_ref.cmp(&right.temporal_ref))
    });

    Ok(ChronologyProjection {
        entries,
        proposition_views,
        candidate_only: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::chronology_contestation::{
        ClaimLeafKind, ClaimReviewState, ContestationRelationKind,
    };

    fn temporal(reference: &str, form: TemporalForm) -> TemporalAssertion {
        TemporalAssertion {
            temporal_ref: reference.into(),
            form,
            statement_refs: vec![format!("statement:{reference}")],
            observation_refs: vec![format!("observation:{reference}")],
            review_ref: Some(format!("review:{reference}")),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn root() -> PropositionRoot {
        PropositionRoot {
            proposition_ref: "proposition:call".into(),
            label: "The call occurred".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn claims() -> Vec<ClaimLeaf> {
        vec![
            ClaimLeaf {
                claim_ref: "claim:a".into(),
                proposition_ref: "proposition:call".into(),
                kind: ClaimLeafKind::Affirmation,
                speaker_ref: Some("actor:a".into()),
                statement_refs: vec!["statement:a".into()],
                observation_refs: vec!["observation:a".into()],
                temporal_refs: vec![],
                scope_refs: vec![],
                review_state: ClaimReviewState::Accepted,
                review_ref: Some("review:a".into()),
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
            ClaimLeaf {
                claim_ref: "claim:b".into(),
                proposition_ref: "proposition:call".into(),
                kind: ClaimLeafKind::Denial,
                speaker_ref: Some("actor:b".into()),
                statement_refs: vec!["statement:b".into()],
                observation_refs: vec!["observation:b".into()],
                temporal_refs: vec![],
                scope_refs: vec![],
                review_state: ClaimReviewState::Unreviewed,
                review_ref: None,
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
        ]
    }

    fn contradiction() -> ContestationRelation {
        ContestationRelation {
            relation_ref: "relation:a:b".into(),
            from_claim_ref: "claim:a".into(),
            to_claim_ref: "claim:b".into(),
            kind: ContestationRelationKind::Contradicts,
            statement_refs: vec!["statement:a".into(), "statement:b".into()],
            observation_refs: vec![],
            review_ref: Some("review:relation".into()),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn chronology_keeps_exact_approximate_relative_undated_unknown_distinct() {
        let event = EventChronologyInput {
            event_ref: "event:1".into(),
            temporal_assertions: vec![
                temporal(
                    "exact",
                    TemporalForm::ExactDate {
                        date_ref: "2026-09-24".into(),
                    },
                ),
                temporal(
                    "approx",
                    TemporalForm::Approximate {
                        label: "~mid September".into(),
                    },
                ),
                temporal(
                    "relative",
                    TemporalForm::RelativeAfter {
                        event_ref: "event:0".into(),
                    },
                ),
                temporal("undated", TemporalForm::Undated),
                temporal("unknown", TemporalForm::Unknown),
            ],
            observation_refs: vec!["observation:event".into()],
            statement_refs: vec!["statement:event".into()],
            proposition_refs: vec![],
            claim_refs: vec![],
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        };

        let projection = project_chronology(&[event], &[], &[], &[]).unwrap();
        let kinds: BTreeSet<_> = projection
            .entries
            .iter()
            .map(|entry| entry.placement)
            .collect();
        assert_eq!(
            kinds,
            BTreeSet::from([
                ChronologyPlacementKind::Exact,
                ChronologyPlacementKind::Approximate,
                ChronologyPlacementKind::RelativeOnly,
                ChronologyPlacementKind::Undated,
                ChronologyPlacementKind::Unknown,
            ])
        );
    }

    #[test]
    fn contestation_is_derived_from_typed_relations_not_boolean_state() {
        let event = EventChronologyInput {
            event_ref: "event:call".into(),
            temporal_assertions: vec![temporal(
                "call-date",
                TemporalForm::ExactDate {
                    date_ref: "2026-09-24".into(),
                },
            )],
            observation_refs: vec!["observation:a".into(), "observation:b".into()],
            statement_refs: vec!["statement:a".into(), "statement:b".into()],
            proposition_refs: vec!["proposition:call".into()],
            claim_refs: vec!["claim:a".into(), "claim:b".into()],
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        };
        let projection =
            project_chronology(&[event], &[root()], &claims(), &[contradiction()]).unwrap();
        assert!(projection.entries[0].has_contestation());
        assert_eq!(
            projection.entries[0].contestation_relation_refs,
            vec!["relation:a:b"]
        );
        assert_eq!(projection.proposition_views[0].leaves.len(), 2);
        assert_eq!(projection.proposition_views[0].relations.len(), 1);
    }

    #[test]
    fn event_without_temporal_assertion_is_unknown_not_absent() {
        let event = EventChronologyInput {
            event_ref: "event:undetermined-date".into(),
            temporal_assertions: vec![],
            observation_refs: vec!["observation:1".into()],
            statement_refs: vec!["statement:1".into()],
            proposition_refs: vec![],
            claim_refs: vec![],
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        };
        let projection = project_chronology(&[event], &[], &[], &[]).unwrap();
        assert_eq!(projection.entries.len(), 1);
        assert_eq!(
            projection.entries[0].placement,
            ChronologyPlacementKind::Unknown
        );
    }

    #[test]
    fn same_root_does_not_merge_competing_claim_leaves() {
        let views = project_proposition_contestation(&[root()], &claims(), &[contradiction()])
            .unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].leaves.len(), 2);
        assert_ne!(views[0].leaves[0].claim_ref, views[0].leaves[1].claim_ref);
        assert_ne!(views[0].leaves[0].kind, views[0].leaves[1].kind);
    }
}
