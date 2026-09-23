//! M11 / S26 comparative and counterfactual world runtime.
//!
//! A comparative receipt separates:
//! - world-input deltas that may causally affect a consumer/query;
//! - proof/residual outcome deltas that report what changed after rerun.
//!
//! A world difference is not automatically an answer difference. Query
//! relevance is paid only by an explicit consumer/query slice. Minimal
//! answer-changing distinctions are searched over relevant *input* deltas,
//! never over the observed answer delta itself.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ComparativeDeltaKind {
    FactAdded,
    FactRemoved,
    FactChanged,
    ReviewStateChanged,
    ScopeChanged,
    AuthorityChanged,
    ApplicabilityChanged,
    DefeaterAdded,
    DefeaterRemoved,
    CounterDefeaterAdded,
    CounterDefeaterRemoved,
    JurisdictionChanged,
    AsAtChanged,
    ConsumerDependencyChanged,
    ResidualOpened,
    ResidualClosed,
    RouteStatusChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ComparativeDeltaRole {
    WorldInput,
    ProofOutcome,
    ResidualOutcome,
    Context,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeDelta {
    pub delta_ref: String,
    pub kind: ComparativeDeltaKind,
    pub role: ComparativeDeltaRole,
    pub coordinate_ref: Option<String>,
    pub route_ref: Option<String>,
    pub residual_ref: Option<String>,
    pub before_ref: Option<String>,
    pub after_ref: Option<String>,
    pub cause_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ComparativeDelta {
    pub fn validate(&self) -> Result<(), String> {
        if self.delta_ref.trim().is_empty() {
            return Err("comparative delta requires delta_ref".into());
        }
        if self.coordinate_ref.is_none()
            && self.route_ref.is_none()
            && self.residual_ref.is_none()
        {
            return Err("comparative delta requires coordinate/route/residual identity".into());
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("comparative delta crossed non-promotion boundary".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeCoordinateState {
    pub coordinate_ref: String,
    pub semantic_ref: String,
    pub review_state_ref: String,
    pub scope_refs: BTreeSet<String>,
    pub authority_ref: Option<String>,
    pub applicability_ref: Option<String>,
    pub jurisdiction_ref: Option<String>,
    pub as_at_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeRouteState {
    pub route_ref: String,
    pub status_ref: String,
    pub active_defeater_refs: BTreeSet<String>,
    pub counter_defeater_refs: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeWorldState {
    pub world_ref: String,
    pub coordinates: BTreeMap<String, ComparativeCoordinateState>,
    pub routes: BTreeMap<String, ComparativeRouteState>,
    pub residual_refs: BTreeSet<String>,
    pub stop_state_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ComparativeWorldState {
    pub fn validate(&self) -> Result<(), String> {
        if self.world_ref.trim().is_empty() {
            return Err("comparative world requires world_ref".into());
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("comparative world crossed non-promotion boundary".into());
        }
        for (coordinate_ref, coordinate) in &self.coordinates {
            if coordinate_ref != &coordinate.coordinate_ref
                || !coordinate.candidate_only
                || coordinate.creates_semantic_authority
                || coordinate.creates_claim_truth
            {
                return Err(format!(
                    "comparative coordinate {coordinate_ref} crossed identity/non-promotion boundary"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeQuerySlice {
    pub query_ref: String,
    pub consumer_ref: String,
    pub coordinate_refs: BTreeSet<String>,
    pub route_refs: BTreeSet<String>,
    pub residual_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ComparativeQuerySlice {
    pub fn validate(&self) -> Result<(), String> {
        if self.query_ref.trim().is_empty() || self.consumer_ref.trim().is_empty() {
            return Err("comparative query slice requires query/consumer refs".into());
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("comparative query slice crossed non-promotion boundary".into());
        }
        Ok(())
    }

    #[must_use]
    pub fn mentions_delta(&self, delta: &ComparativeDelta) -> bool {
        delta
            .coordinate_ref
            .as_ref()
            .is_some_and(|reference| self.coordinate_refs.contains(reference))
            || delta
                .route_ref
                .as_ref()
                .is_some_and(|reference| self.route_refs.contains(reference))
            || delta
                .residual_ref
                .as_ref()
                .is_some_and(|reference| self.residual_refs.contains(reference))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeWorldIr {
    pub left_world_ref: String,
    pub right_world_ref: String,
    pub shared_coordinate_refs: BTreeSet<String>,
    pub changed_coordinate_refs: BTreeSet<String>,
    pub shared_route_refs: BTreeSet<String>,
    pub changed_route_refs: BTreeSet<String>,
    pub changed_residual_refs: BTreeSet<String>,
    pub changed_stop_state: bool,
    pub deltas: Vec<ComparativeDelta>,
    pub query_relevant_delta_refs: BTreeSet<String>,
    pub query_irrelevant_delta_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn coordinate_field_deltas(
    left: &ComparativeCoordinateState,
    right: &ComparativeCoordinateState,
) -> Vec<ComparativeDelta> {
    let mut out = Vec::new();
    let base = format!("{}:{}", left.coordinate_ref, right.coordinate_ref);
    let mut push = |kind, suffix: &str, before: Option<String>, after: Option<String>| {
        out.push(ComparativeDelta {
            delta_ref: format!("delta:{base}:{suffix}"),
            kind,
            role: ComparativeDeltaRole::WorldInput,
            coordinate_ref: Some(left.coordinate_ref.clone()),
            route_ref: None,
            residual_ref: None,
            before_ref: before,
            after_ref: after,
            cause_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        });
    };

    if left.semantic_ref != right.semantic_ref {
        push(
            ComparativeDeltaKind::FactChanged,
            "semantic",
            Some(left.semantic_ref.clone()),
            Some(right.semantic_ref.clone()),
        );
    }
    if left.review_state_ref != right.review_state_ref {
        push(
            ComparativeDeltaKind::ReviewStateChanged,
            "review",
            Some(left.review_state_ref.clone()),
            Some(right.review_state_ref.clone()),
        );
    }
    if left.scope_refs != right.scope_refs {
        push(
            ComparativeDeltaKind::ScopeChanged,
            "scope",
            Some(format!("{:?}", left.scope_refs)),
            Some(format!("{:?}", right.scope_refs)),
        );
    }
    if left.authority_ref != right.authority_ref {
        push(
            ComparativeDeltaKind::AuthorityChanged,
            "authority",
            left.authority_ref.clone(),
            right.authority_ref.clone(),
        );
    }
    if left.applicability_ref != right.applicability_ref {
        push(
            ComparativeDeltaKind::ApplicabilityChanged,
            "applicability",
            left.applicability_ref.clone(),
            right.applicability_ref.clone(),
        );
    }
    if left.jurisdiction_ref != right.jurisdiction_ref {
        push(
            ComparativeDeltaKind::JurisdictionChanged,
            "jurisdiction",
            left.jurisdiction_ref.clone(),
            right.jurisdiction_ref.clone(),
        );
    }
    if left.as_at_ref != right.as_at_ref {
        push(
            ComparativeDeltaKind::AsAtChanged,
            "as-at",
            left.as_at_ref.clone(),
            right.as_at_ref.clone(),
        );
    }
    out
}

pub fn compare_worlds(
    left: &ComparativeWorldState,
    right: &ComparativeWorldState,
    query: &ComparativeQuerySlice,
    additional_input_deltas: impl IntoIterator<Item = ComparativeDelta>,
) -> Result<ComparativeWorldIr, String> {
    left.validate()?;
    right.validate()?;
    query.validate()?;

    let explicit_input_deltas = additional_input_deltas
        .into_iter()
        .map(|delta| {
            delta.validate()?;
            if delta.role != ComparativeDeltaRole::WorldInput
                && delta.role != ComparativeDeltaRole::Context
            {
                return Err("additional deltas must be world-input/context deltas".into());
            }
            Ok(delta)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let explicitly_typed_coordinate_refs = explicit_input_deltas
        .iter()
        .filter_map(|delta| delta.coordinate_ref.clone())
        .collect::<BTreeSet<_>>();

    let left_coords = left.coordinates.keys().cloned().collect::<BTreeSet<_>>();
    let right_coords = right.coordinates.keys().cloned().collect::<BTreeSet<_>>();
    let shared_coordinate_refs = left_coords
        .intersection(&right_coords)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changed_coordinate_refs = BTreeSet::new();
    let mut deltas = Vec::new();

    for coordinate_ref in left_coords.difference(&right_coords) {
        changed_coordinate_refs.insert(coordinate_ref.clone());
        if explicitly_typed_coordinate_refs.contains(coordinate_ref) {
            continue;
        }
        deltas.push(ComparativeDelta {
            delta_ref: format!("delta:coordinate-removed:{coordinate_ref}"),
            kind: ComparativeDeltaKind::FactRemoved,
            role: ComparativeDeltaRole::WorldInput,
            coordinate_ref: Some(coordinate_ref.clone()),
            route_ref: None,
            residual_ref: None,
            before_ref: Some(
                left.coordinates
                    .get(coordinate_ref)
                    .expect("left coordinate exists")
                    .semantic_ref
                    .clone(),
            ),
            after_ref: None,
            cause_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        });
    }
    for coordinate_ref in right_coords.difference(&left_coords) {
        changed_coordinate_refs.insert(coordinate_ref.clone());
        if explicitly_typed_coordinate_refs.contains(coordinate_ref) {
            continue;
        }
        deltas.push(ComparativeDelta {
            delta_ref: format!("delta:coordinate-added:{coordinate_ref}"),
            kind: ComparativeDeltaKind::FactAdded,
            role: ComparativeDeltaRole::WorldInput,
            coordinate_ref: Some(coordinate_ref.clone()),
            route_ref: None,
            residual_ref: None,
            before_ref: None,
            after_ref: Some(
                right
                    .coordinates
                    .get(coordinate_ref)
                    .expect("right coordinate exists")
                    .semantic_ref
                    .clone(),
            ),
            cause_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        });
    }
    for coordinate_ref in &shared_coordinate_refs {
        let left_coordinate = &left.coordinates[coordinate_ref];
        let right_coordinate = &right.coordinates[coordinate_ref];
        let field_deltas = coordinate_field_deltas(left_coordinate, right_coordinate);
        if !field_deltas.is_empty() {
            changed_coordinate_refs.insert(coordinate_ref.clone());
            if !explicitly_typed_coordinate_refs.contains(coordinate_ref) {
                deltas.extend(field_deltas);
            }
        }
    }

    for delta in explicit_input_deltas {
        if let Some(coordinate_ref) = &delta.coordinate_ref {
            changed_coordinate_refs.insert(coordinate_ref.clone());
        }
        deltas.push(delta);
    }

    let left_routes = left.routes.keys().cloned().collect::<BTreeSet<_>>();
    let right_routes = right.routes.keys().cloned().collect::<BTreeSet<_>>();
    let shared_route_refs = left_routes
        .intersection(&right_routes)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changed_route_refs = BTreeSet::new();

    for route_ref in left_routes.union(&right_routes) {
        match (left.routes.get(route_ref), right.routes.get(route_ref)) {
            (Some(before), Some(after)) if before != after => {
                changed_route_refs.insert(route_ref.clone());
                deltas.push(ComparativeDelta {
                    delta_ref: format!("delta:route-status:{route_ref}"),
                    kind: ComparativeDeltaKind::RouteStatusChanged,
                    role: ComparativeDeltaRole::ProofOutcome,
                    coordinate_ref: None,
                    route_ref: Some(route_ref.clone()),
                    residual_ref: None,
                    before_ref: Some(before.status_ref.clone()),
                    after_ref: Some(after.status_ref.clone()),
                    cause_refs: BTreeSet::new(),
                    candidate_only: true,
                    creates_semantic_authority: false,
                    creates_claim_truth: false,
                });
            }
            (Some(before), None) => {
                changed_route_refs.insert(route_ref.clone());
                deltas.push(ComparativeDelta {
                    delta_ref: format!("delta:route-removed:{route_ref}"),
                    kind: ComparativeDeltaKind::RouteStatusChanged,
                    role: ComparativeDeltaRole::ProofOutcome,
                    coordinate_ref: None,
                    route_ref: Some(route_ref.clone()),
                    residual_ref: None,
                    before_ref: Some(before.status_ref.clone()),
                    after_ref: None,
                    cause_refs: BTreeSet::new(),
                    candidate_only: true,
                    creates_semantic_authority: false,
                    creates_claim_truth: false,
                });
            }
            (None, Some(after)) => {
                changed_route_refs.insert(route_ref.clone());
                deltas.push(ComparativeDelta {
                    delta_ref: format!("delta:route-added:{route_ref}"),
                    kind: ComparativeDeltaKind::RouteStatusChanged,
                    role: ComparativeDeltaRole::ProofOutcome,
                    coordinate_ref: None,
                    route_ref: Some(route_ref.clone()),
                    residual_ref: None,
                    before_ref: None,
                    after_ref: Some(after.status_ref.clone()),
                    cause_refs: BTreeSet::new(),
                    candidate_only: true,
                    creates_semantic_authority: false,
                    creates_claim_truth: false,
                });
            }
            _ => {}
        }
    }

    let changed_residual_refs = left
        .residual_refs
        .symmetric_difference(&right.residual_refs)
        .cloned()
        .collect::<BTreeSet<_>>();
    for residual_ref in left.residual_refs.difference(&right.residual_refs) {
        deltas.push(ComparativeDelta {
            delta_ref: format!("delta:residual-closed:{residual_ref}"),
            kind: ComparativeDeltaKind::ResidualClosed,
            role: ComparativeDeltaRole::ResidualOutcome,
            coordinate_ref: None,
            route_ref: None,
            residual_ref: Some(residual_ref.clone()),
            before_ref: Some("open".into()),
            after_ref: Some("closed".into()),
            cause_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        });
    }
    for residual_ref in right.residual_refs.difference(&left.residual_refs) {
        deltas.push(ComparativeDelta {
            delta_ref: format!("delta:residual-opened:{residual_ref}"),
            kind: ComparativeDeltaKind::ResidualOpened,
            role: ComparativeDeltaRole::ResidualOutcome,
            coordinate_ref: None,
            route_ref: None,
            residual_ref: Some(residual_ref.clone()),
            before_ref: Some("closed".into()),
            after_ref: Some("open".into()),
            cause_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        });
    }

    let changed_stop_state = left.stop_state_ref != right.stop_state_ref;

    let query_relevant_delta_refs = deltas
        .iter()
        .filter(|delta| query.mentions_delta(delta))
        .map(|delta| delta.delta_ref.clone())
        .collect::<BTreeSet<_>>();
    let query_irrelevant_delta_refs = deltas
        .iter()
        .filter(|delta| !query.mentions_delta(delta))
        .map(|delta| delta.delta_ref.clone())
        .collect::<BTreeSet<_>>();

    Ok(ComparativeWorldIr {
        left_world_ref: left.world_ref.clone(),
        right_world_ref: right.world_ref.clone(),
        shared_coordinate_refs,
        changed_coordinate_refs,
        shared_route_refs,
        changed_route_refs,
        changed_residual_refs,
        changed_stop_state,
        deltas,
        query_relevant_delta_refs,
        query_irrelevant_delta_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerChangingDistinction {
    pub query_ref: String,
    pub baseline_answer_ref: String,
    pub target_answer_ref: String,
    pub delta_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn enumerate_subsets<T: Clone>(values: &[T]) -> Vec<Vec<T>> {
    let mut subsets = vec![Vec::new()];
    for value in values {
        let additions = subsets
            .iter()
            .cloned()
            .map(|mut subset| {
                subset.push(value.clone());
                subset
            })
            .collect::<Vec<_>>();
        subsets.extend(additions);
    }
    subsets.sort_by_key(Vec::len);
    subsets
}

pub fn minimal_answer_changing_distinction<F>(
    comparison: &ComparativeWorldIr,
    query: &ComparativeQuerySlice,
    baseline_answer_ref: &str,
    target_answer_ref: &str,
    evaluate: F,
) -> Result<Option<AnswerChangingDistinction>, String>
where
    F: Fn(&[ComparativeDelta]) -> String,
{
    if baseline_answer_ref.trim().is_empty() || target_answer_ref.trim().is_empty() {
        return Err("answer-changing search requires baseline and target answer refs".into());
    }
    if baseline_answer_ref == target_answer_ref {
        return Ok(None);
    }

    let candidates = comparison
        .deltas
        .iter()
        .filter(|delta| {
            delta.role == ComparativeDeltaRole::WorldInput
                && comparison
                    .query_relevant_delta_refs
                    .contains(&delta.delta_ref)
        })
        .cloned()
        .collect::<Vec<_>>();

    if candidates.len() > 20 {
        return Err("answer-changing distinction brute-force search refuses >20 input deltas".into());
    }

    for subset in enumerate_subsets(&candidates) {
        if subset.is_empty() || evaluate(&subset) != target_answer_ref {
            continue;
        }
        let inclusion_minimal = (0..subset.len()).all(|index| {
            let reduced = subset
                .iter()
                .enumerate()
                .filter_map(|(candidate_index, delta)| {
                    (candidate_index != index).then_some(delta.clone())
                })
                .collect::<Vec<_>>();
            evaluate(&reduced) != target_answer_ref
        });
        if inclusion_minimal {
            return Ok(Some(AnswerChangingDistinction {
                query_ref: query.query_ref.clone(),
                baseline_answer_ref: baseline_answer_ref.into(),
                target_answer_ref: target_answer_ref.into(),
                delta_refs: subset
                    .iter()
                    .map(|delta| delta.delta_ref.clone())
                    .collect(),
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            }));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coordinate(reference: &str, semantic: &str) -> ComparativeCoordinateState {
        ComparativeCoordinateState {
            coordinate_ref: reference.into(),
            semantic_ref: semantic.into(),
            review_state_ref: "reviewed".into(),
            scope_refs: BTreeSet::new(),
            authority_ref: None,
            applicability_ref: None,
            jurisdiction_ref: Some("AU".into()),
            as_at_ref: None,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn world(reference: &str, coordinates: &[(&str, &str)]) -> ComparativeWorldState {
        ComparativeWorldState {
            world_ref: reference.into(),
            coordinates: coordinates
                .iter()
                .map(|(reference, semantic)| {
                    (
                        (*reference).into(),
                        coordinate(reference, semantic),
                    )
                })
                .collect(),
            routes: BTreeMap::new(),
            residual_refs: BTreeSet::new(),
            stop_state_ref: None,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    #[test]
    fn world_difference_outside_query_slice_is_not_query_relevant() {
        let left = world("world:left", &[("coordinate:q", "q"), ("coordinate:noise", "old")]);
        let right = world("world:right", &[("coordinate:q", "q"), ("coordinate:noise", "new")]);
        let query = ComparativeQuerySlice {
            query_ref: "query:q".into(),
            consumer_ref: "consumer:q".into(),
            coordinate_refs: BTreeSet::from(["coordinate:q".into()]),
            route_refs: BTreeSet::new(),
            residual_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };

        let comparison = compare_worlds(&left, &right, &query, []).unwrap();
        assert!(comparison.query_relevant_delta_refs.is_empty());
        assert!(!comparison.query_irrelevant_delta_refs.is_empty());
    }

    #[test]
    fn domain_typed_shared_coordinate_delta_replaces_generic_field_delta() {
        let left = world("world:same", &[("coordinate:theory", "newton")]);
        let right = world("world:same", &[("coordinate:theory", "relativity")]);
        let query = ComparativeQuerySlice {
            query_ref: "query:theory".into(),
            consumer_ref: "consumer:physics".into(),
            coordinate_refs: BTreeSet::from(["coordinate:theory".into()]),
            route_refs: BTreeSet::new(),
            residual_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let typed = ComparativeDelta {
            delta_ref: "delta:theory:newton-gr".into(),
            kind: ComparativeDeltaKind::FactChanged,
            role: ComparativeDeltaRole::WorldInput,
            coordinate_ref: Some("coordinate:theory".into()),
            route_ref: None,
            residual_ref: None,
            before_ref: Some("newton".into()),
            after_ref: Some("relativity".into()),
            cause_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };

        let comparison = compare_worlds(&left, &right, &query, [typed]).unwrap();
        assert_eq!(
            comparison
                .deltas
                .iter()
                .filter(|delta| delta.coordinate_ref.as_deref() == Some("coordinate:theory"))
                .count(),
            1
        );
        assert!(comparison
            .deltas
            .iter()
            .any(|delta| delta.delta_ref == "delta:theory:newton-gr"));
    }

    #[test]
    fn domain_typed_coordinate_delta_replaces_generic_addition_delta() {
        let left = world("world:left", &[]);
        let right = world("world:right", &[("coordinate:d", "defeater")]);
        let query = ComparativeQuerySlice {
            query_ref: "query:d".into(),
            consumer_ref: "consumer:d".into(),
            coordinate_refs: BTreeSet::from(["coordinate:d".into()]),
            route_refs: BTreeSet::new(),
            residual_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let typed = ComparativeDelta {
            delta_ref: "delta:d:defeater".into(),
            kind: ComparativeDeltaKind::DefeaterAdded,
            role: ComparativeDeltaRole::WorldInput,
            coordinate_ref: Some("coordinate:d".into()),
            route_ref: None,
            residual_ref: None,
            before_ref: None,
            after_ref: Some("defeater".into()),
            cause_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };

        let comparison = compare_worlds(&left, &right, &query, [typed]).unwrap();
        assert_eq!(
            comparison
                .deltas
                .iter()
                .filter(|delta| delta.coordinate_ref.as_deref() == Some("coordinate:d"))
                .count(),
            1
        );
        assert_eq!(
            comparison
                .deltas
                .iter()
                .find(|delta| delta.coordinate_ref.as_deref() == Some("coordinate:d"))
                .unwrap()
                .kind,
            ComparativeDeltaKind::DefeaterAdded
        );
    }

    #[test]
    fn answer_changing_search_ignores_proof_outcome_delta_as_cause() {
        let left = world("world:left", &[("coordinate:d", "absent")]);
        let right = world("world:right", &[("coordinate:d", "present")]);
        let query = ComparativeQuerySlice {
            query_ref: "query:d".into(),
            consumer_ref: "consumer:d".into(),
            coordinate_refs: BTreeSet::from(["coordinate:d".into()]),
            route_refs: BTreeSet::from(["route:d".into()]),
            residual_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let comparison = compare_worlds(&left, &right, &query, []).unwrap();
        let distinction = minimal_answer_changing_distinction(
            &comparison,
            &query,
            "reachable",
            "defeated",
            |subset| {
                if subset.iter().any(|delta| delta.coordinate_ref.as_deref() == Some("coordinate:d")) {
                    "defeated".into()
                } else {
                    "reachable".into()
                }
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(distinction.delta_refs.len(), 1);
        assert!(!distinction.creates_claim_truth);
    }
}
