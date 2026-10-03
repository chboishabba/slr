//! ITIR-INV-1 — proof-directed investigation/acquisition over REL residuals.
//!
//! Residual -> acquisition obligation -> mixed-orientation Pareto frontier ->
//! lawful acquisition -> new source -> selective reopening. Priority never
//! becomes source truth, access authority, semantic authority, or recommendation.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::{ComparisonResidual, RelationalComparison};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordAvailability { Present, NotLocated, KnownAbsent }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessDisposition {
    Public,
    Authorized,
    RequiresAuthorization,
    ProhibitedOrUnavailable,
    Unknown,
}
impl AccessDisposition {
    pub fn executable(self) -> bool {
        matches!(self, Self::Public | Self::Authorized)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceIndependence { KnownIndependent, KnownDependent, Unknown }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateRelation {
    MetadataDuplicate,
    PublicationDuplicate,
    ReportFamilyDuplicate,
    SameEmpiricalStudy,
    DerivativeOfCommonSource,
    NoKnownDuplicateRelation,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionObligation {
    pub obligation_ref: String,
    pub comparison_ref: String,
    pub residual_obligation_ref: String,
    pub source_revision_refs: Vec<String>,
    pub target_description: String,
    pub current_availability: RecordAvailability,
    pub authority_or_access_constraint_ref: String,
    pub dependency_target_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionRouteCandidate {
    pub route_ref: String,
    pub obligation_ref: String,
    pub route_description: String,
    pub source_locator_ref: String,
    pub access_disposition: AccessDisposition,
    pub authority_receipt_ref: Option<String>,
    pub provenance_genealogy_ref: String,
    pub independence: EvidenceIndependence,
    pub independence_receipt_ref: Option<String>,
    pub duplicate_relation: DuplicateRelation,
    /// Larger is better.
    pub information_gain: u32,
    /// Larger is better.
    pub dependency_closure_impact: u32,
    /// Larger is better.
    pub residual_coverage: u32,
    /// Larger is better; does not itself prove independence.
    pub provenance_novelty: u32,
    /// Smaller is better.
    pub acquisition_cost: u32,
    pub axis_estimation_receipt_ref: String,
    pub candidate_only: bool,
    pub creates_acquisition_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionPriorityReceipt {
    pub obligation_ref: String,
    pub frontier_route_refs: Vec<String>,
    pub executable_frontier_route_refs: Vec<String>,
    pub blocked_frontier_route_refs: Vec<String>,
    pub scalar_score_used: bool,
    pub creates_semantic_authority: bool,
    pub creates_screening_or_acquisition_decision: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionUpdate {
    pub obligation_ref: String,
    pub before: RecordAvailability,
    pub after: RecordAvailability,
    pub acquired_source_revision_ref: Option<String>,
    pub acquisition_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectiveReopeningReceipt {
    pub changed_source_revision_ref: String,
    pub directly_affected_refs: Vec<String>,
    pub transitively_affected_refs: Vec<String>,
    pub unrelated_refs_not_reopened: Vec<String>,
    pub dependency_graph_ref: String,
    pub creates_semantic_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisRelation { Better, Equal, Worse }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParetoAxisRelation {
    pub axis_ref: String,
    pub relation: AxisRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParetoDominanceWitness {
    pub dominator_ref: String,
    pub dominated_ref: String,
    pub weak_axis_relations: Vec<ParetoAxisRelation>,
    pub strict_axis_refs: Vec<String>,
    pub scalar_score_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteFrontierDisposition {
    FrontierExecutable,
    FrontierBlocked,
    Dominated { witness: ParetoDominanceWitness },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDispositionReceipt {
    pub route_ref: String,
    pub disposition: RouteFrontierDisposition,
    pub render_order_ref: String,
    pub preferred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PotentialReopeningCone {
    pub seed_ref: String,
    pub direct_refs: Vec<String>,
    pub transitive_refs: Vec<String>,
    pub creates_actual_reopening: bool,
    pub creates_semantic_authority: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InvestigationAcquisitionError {
    #[error("missing or promotion-bearing acquisition identity")]
    InvalidIdentity,
    #[error("only not-located evidence may create an open acquisition obligation")]
    WrongAvailability,
    #[error("route is not scoped to the requested obligation")]
    WrongObligation,
    #[error("authorized access requires a non-empty authority receipt")]
    MissingAuthorityReceipt,
    #[error("known independence/dependence requires a genealogy receipt")]
    MissingIndependenceReceipt,
    #[error("acquisition update is not a permitted availability transition")]
    InvalidUpdate,
}

fn valid(s: &str) -> bool { !s.trim().is_empty() }

pub fn obligation_from_residual(
    comparison: &RelationalComparison,
    residual: &ComparisonResidual,
    mut source_revision_refs: Vec<String>,
    target_description: &str,
    authority_or_access_constraint_ref: &str,
    dependency_target_refs: Vec<String>,
) -> Result<AcquisitionObligation, InvestigationAcquisitionError> {
    if !valid(&comparison.comparison_ref)
        || !valid(&residual.obligation_ref)
        || !valid(target_description)
        || !valid(authority_or_access_constraint_ref)
        || source_revision_refs.len() < 2
        || source_revision_refs.iter().any(|s| !valid(s))
    {
        return Err(InvestigationAcquisitionError::InvalidIdentity);
    }
    source_revision_refs.sort();
    source_revision_refs.dedup();
    if source_revision_refs.len() < 2 {
        return Err(InvestigationAcquisitionError::InvalidIdentity);
    }
    Ok(AcquisitionObligation {
        obligation_ref: format!("acquisition:{}:{}", comparison.comparison_ref, residual.obligation_ref),
        comparison_ref: comparison.comparison_ref.clone(),
        residual_obligation_ref: residual.obligation_ref.clone(),
        source_revision_refs,
        target_description: target_description.into(),
        current_availability: RecordAvailability::NotLocated,
        authority_or_access_constraint_ref: authority_or_access_constraint_ref.into(),
        dependency_target_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}

fn validate_route(
    obligation: &AcquisitionObligation,
    route: &AcquisitionRouteCandidate,
) -> Result<(), InvestigationAcquisitionError> {
    if !obligation.candidate_only
        || obligation.creates_semantic_authority
        || obligation.claim_truth_promoted
        || !route.candidate_only
        || route.creates_acquisition_authority
        || !valid(&route.route_ref)
        || !valid(&route.route_description)
        || !valid(&route.source_locator_ref)
        || !valid(&route.provenance_genealogy_ref)
        || !valid(&route.axis_estimation_receipt_ref)
    {
        return Err(InvestigationAcquisitionError::InvalidIdentity);
    }
    if obligation.current_availability != RecordAvailability::NotLocated {
        return Err(InvestigationAcquisitionError::WrongAvailability);
    }
    if route.obligation_ref != obligation.obligation_ref {
        return Err(InvestigationAcquisitionError::WrongObligation);
    }
    if route.access_disposition == AccessDisposition::Authorized
        && route.authority_receipt_ref.as_deref().is_none_or(|s| !valid(s))
    {
        return Err(InvestigationAcquisitionError::MissingAuthorityReceipt);
    }
    if route.independence != EvidenceIndependence::Unknown
        && route.independence_receipt_ref.as_deref().is_none_or(|s| !valid(s))
    {
        return Err(InvestigationAcquisitionError::MissingIndependenceReceipt);
    }
    Ok(())
}

/// Direct mixed-orientation dominance. No transformed loss scale is used.
pub fn weakly_dominates(left: &AcquisitionRouteCandidate, right: &AcquisitionRouteCandidate) -> bool {
    left.information_gain >= right.information_gain
        && left.dependency_closure_impact >= right.dependency_closure_impact
        && left.residual_coverage >= right.residual_coverage
        && left.provenance_novelty >= right.provenance_novelty
        && left.acquisition_cost <= right.acquisition_cost
}

fn strictly_better_some_axis(left: &AcquisitionRouteCandidate, right: &AcquisitionRouteCandidate) -> bool {
    left.information_gain > right.information_gain
        || left.dependency_closure_impact > right.dependency_closure_impact
        || left.residual_coverage > right.residual_coverage
        || left.provenance_novelty > right.provenance_novelty
        || left.acquisition_cost < right.acquisition_cost
}

fn axis_relation_max(left: u32, right: u32) -> AxisRelation {
    if left > right { AxisRelation::Better } else if left == right { AxisRelation::Equal } else { AxisRelation::Worse }
}
fn axis_relation_min(left: u32, right: u32) -> AxisRelation {
    if left < right { AxisRelation::Better } else if left == right { AxisRelation::Equal } else { AxisRelation::Worse }
}

pub fn dominance_witness(
    dominator: &AcquisitionRouteCandidate,
    dominated: &AcquisitionRouteCandidate,
) -> Option<ParetoDominanceWitness> {
    if !weakly_dominates(dominator, dominated) || !strictly_better_some_axis(dominator, dominated) {
        return None;
    }
    let relations = vec![
        ParetoAxisRelation { axis_ref: "information_gain".into(), relation: axis_relation_max(dominator.information_gain, dominated.information_gain) },
        ParetoAxisRelation { axis_ref: "dependency_closure_impact".into(), relation: axis_relation_max(dominator.dependency_closure_impact, dominated.dependency_closure_impact) },
        ParetoAxisRelation { axis_ref: "residual_coverage".into(), relation: axis_relation_max(dominator.residual_coverage, dominated.residual_coverage) },
        ParetoAxisRelation { axis_ref: "provenance_novelty".into(), relation: axis_relation_max(dominator.provenance_novelty, dominated.provenance_novelty) },
        ParetoAxisRelation { axis_ref: "acquisition_cost".into(), relation: axis_relation_min(dominator.acquisition_cost, dominated.acquisition_cost) },
    ];
    let strict_axis_refs = relations.iter()
        .filter(|row| row.relation == AxisRelation::Better)
        .map(|row| row.axis_ref.clone())
        .collect();
    Some(ParetoDominanceWitness {
        dominator_ref: dominator.route_ref.clone(),
        dominated_ref: dominated.route_ref.clone(),
        weak_axis_relations: relations,
        strict_axis_refs,
        scalar_score_used: false,
    })
}

pub fn acquisition_pareto_frontier(
    obligation: &AcquisitionObligation,
    routes: &[AcquisitionRouteCandidate],
) -> Result<AcquisitionPriorityReceipt, InvestigationAcquisitionError> {
    for route in routes { validate_route(obligation, route)?; }
    let mut frontier = Vec::new();
    for (i, candidate) in routes.iter().enumerate() {
        let dominated = routes.iter().enumerate().any(|(j, other)| {
            i != j && dominance_witness(other, candidate).is_some()
        });
        if !dominated { frontier.push(candidate); }
    }
    frontier.sort_by(|a, b| a.route_ref.cmp(&b.route_ref));
    let frontier_refs = frontier.iter().map(|r| r.route_ref.clone()).collect();
    let executable = frontier.iter().filter(|r| r.access_disposition.executable()).map(|r| r.route_ref.clone()).collect();
    let blocked = frontier.iter().filter(|r| !r.access_disposition.executable()).map(|r| r.route_ref.clone()).collect();
    Ok(AcquisitionPriorityReceipt {
        obligation_ref: obligation.obligation_ref.clone(),
        frontier_route_refs: frontier_refs,
        executable_frontier_route_refs: executable,
        blocked_frontier_route_refs: blocked,
        scalar_score_used: false,
        creates_semantic_authority: false,
        creates_screening_or_acquisition_decision: false,
    })
}

pub fn route_frontier_dispositions(
    obligation: &AcquisitionObligation,
    routes: &[AcquisitionRouteCandidate],
) -> Result<Vec<RouteDispositionReceipt>, InvestigationAcquisitionError> {
    for route in routes { validate_route(obligation, route)?; }
    let mut ordered = routes.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| a.route_ref.cmp(&b.route_ref));
    let mut out = Vec::with_capacity(ordered.len());
    for route in &ordered {
        let mut dominators = ordered.iter()
            .filter_map(|other| dominance_witness(other, route))
            .collect::<Vec<_>>();
        dominators.sort_by(|a, b| a.dominator_ref.cmp(&b.dominator_ref));
        let disposition = if let Some(witness) = dominators.into_iter().next() {
            RouteFrontierDisposition::Dominated { witness }
        } else if route.access_disposition.executable() {
            RouteFrontierDisposition::FrontierExecutable
        } else {
            RouteFrontierDisposition::FrontierBlocked
        };
        out.push(RouteDispositionReceipt {
            route_ref: route.route_ref.clone(),
            disposition,
            render_order_ref: format!("route-id:{}", route.route_ref),
            preferred: false,
        });
    }
    Ok(out)
}

pub fn potential_reopening_cone(
    seed_ref: &str,
    edges: &[(String, String)],
) -> Result<PotentialReopeningCone, InvestigationAcquisitionError> {
    if !valid(seed_ref) { return Err(InvestigationAcquisitionError::InvalidIdentity); }
    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (from, to) in edges {
        if !valid(from) || !valid(to) { return Err(InvestigationAcquisitionError::InvalidIdentity); }
        adjacency.entry(from.clone()).or_default().push(to.clone());
    }
    let mut direct = adjacency.get(seed_ref).cloned().unwrap_or_default();
    direct.sort();
    direct.dedup();
    let direct_set = direct.iter().cloned().collect::<BTreeSet<_>>();
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from(direct.clone());
    while let Some(node) = queue.pop_front() {
        if !visited.insert(node.clone()) { continue; }
        if let Some(next) = adjacency.get(&node) { queue.extend(next.iter().cloned()); }
    }
    let transitive_refs = visited.iter().filter(|r| !direct_set.contains(*r)).cloned().collect();
    Ok(PotentialReopeningCone {
        seed_ref: seed_ref.into(),
        direct_refs: direct,
        transitive_refs,
        creates_actual_reopening: false,
        creates_semantic_authority: false,
    })
}

pub fn apply_acquisition_update(
    obligation: &AcquisitionObligation,
    update: &AcquisitionUpdate,
) -> Result<(), InvestigationAcquisitionError> {
    if update.obligation_ref != obligation.obligation_ref || !valid(&update.acquisition_receipt_ref) {
        return Err(InvestigationAcquisitionError::WrongObligation);
    }
    let valid_transition = matches!(
        (update.before, update.after),
        (RecordAvailability::NotLocated, RecordAvailability::Present)
            | (RecordAvailability::NotLocated, RecordAvailability::KnownAbsent)
    );
    if !valid_transition || update.before != obligation.current_availability {
        return Err(InvestigationAcquisitionError::InvalidUpdate);
    }
    if update.after == RecordAvailability::Present
        && update.acquired_source_revision_ref.as_deref().is_none_or(|s| !valid(s))
    {
        return Err(InvestigationAcquisitionError::InvalidUpdate);
    }
    if update.after == RecordAvailability::KnownAbsent && update.acquired_source_revision_ref.is_some() {
        return Err(InvestigationAcquisitionError::InvalidUpdate);
    }
    Ok(())
}

pub fn selective_reopening(
    changed_source_revision_ref: &str,
    dependency_graph_ref: &str,
    edges: &[(String, String)],
    universe_refs: &[String],
) -> Result<SelectiveReopeningReceipt, InvestigationAcquisitionError> {
    if !valid(changed_source_revision_ref) || !valid(dependency_graph_ref) {
        return Err(InvestigationAcquisitionError::InvalidIdentity);
    }
    let cone = potential_reopening_cone(changed_source_revision_ref, edges)?;
    let visited = cone.direct_refs.iter().chain(cone.transitive_refs.iter()).cloned().collect::<BTreeSet<_>>();
    let unrelated = universe_refs.iter()
        .filter(|r| *r != changed_source_revision_ref && !visited.contains(*r))
        .cloned()
        .collect();
    Ok(SelectiveReopeningReceipt {
        changed_source_revision_ref: changed_source_revision_ref.into(),
        directly_affected_refs: cone.direct_refs,
        transitively_affected_refs: cone.transitive_refs,
        unrelated_refs_not_reopened: unrelated,
        dependency_graph_ref: dependency_graph_ref.into(),
        creates_semantic_authority: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComparisonFinding, ResidualKind};

    fn comparison() -> RelationalComparison {
        RelationalComparison {
            schema: "itir.relational-comparison.v1".into(),
            consumer_ref: "matter:investigation".into(),
            left_observation_ref: "source:a".into(),
            right_observation_ref: "source:b".into(),
            comparison_ref: "comparison:1".into(),
            finding: ComparisonFinding::PartialResidual,
            residuals: vec![],
            used_alignment_witness_refs: vec![], role_type_evidence_refs: vec![],
            positive_support_refs: vec![], counter_support_refs: vec![], explicit_unknown_refs: vec![],
            creates_semantic_authority: false, merges_sources: false,
            proves_independence: false, claim_truth_promoted: false,
        }
    }

    fn obligation() -> AcquisitionObligation {
        obligation_from_residual(
            &comparison(),
            &ComparisonResidual {
                kind: ResidualKind::MissingProvenance,
                left_ref: None,
                right_ref: None,
                obligation_ref: "missing:original-record".into(),
            },
            vec!["revision:a".into(), "revision:b".into()],
            "obtain original record",
            "lawful-access:public-records",
            vec!["assessment:case".into()],
        ).unwrap()
    }

    fn route(id: &str, gain: u32, cost: u32) -> AcquisitionRouteCandidate {
        AcquisitionRouteCandidate {
            route_ref: id.into(), obligation_ref: obligation().obligation_ref,
            route_description: "lawful route".into(), source_locator_ref: format!("locator:{id}"),
            access_disposition: AccessDisposition::Public, authority_receipt_ref: None,
            provenance_genealogy_ref: format!("genealogy:{id}"), independence: EvidenceIndependence::Unknown,
            independence_receipt_ref: None, duplicate_relation: DuplicateRelation::Unknown,
            information_gain: gain, dependency_closure_impact: 3, residual_coverage: 2,
            provenance_novelty: 2, acquisition_cost: cost,
            axis_estimation_receipt_ref: "estimate:reviewed".into(),
            candidate_only: true, creates_acquisition_authority: false,
        }
    }

    #[test]
    fn pareto_is_not_scalar_and_removes_strictly_dominated_route() {
        let o = obligation();
        let r = acquisition_pareto_frontier(&o, &[route("better", 5, 2), route("worse", 4, 3), route("cheap", 2, 1)]).unwrap();
        assert!(!r.scalar_score_used);
        assert_eq!(r.frontier_route_refs, vec!["better", "cheap"]);
    }

    #[test]
    fn mixed_orientation_dominance_preserves_values_above_1000() {
        let better = route("better", 5000, 10);
        let worse = route("worse", 1200, 10);
        let witness = dominance_witness(&better, &worse).unwrap();
        assert_eq!(witness.strict_axis_refs, vec!["information_gain"]);
        assert!(!witness.scalar_score_used);
    }

    #[test]
    fn blocked_frontier_and_executable_dominated_are_structurally_distinct() {
        let o = obligation();
        let mut blocked = route("blocked", 10, 10);
        blocked.access_disposition = AccessDisposition::RequiresAuthorization;
        let dominated = route("dominated", 9, 11);
        let rows = route_frontier_dispositions(&o, &[dominated, blocked]).unwrap();
        assert_eq!(rows[0].route_ref, "blocked");
        assert!(matches!(rows[0].disposition, RouteFrontierDisposition::FrontierBlocked));
        assert_eq!(rows[1].route_ref, "dominated");
        assert!(matches!(rows[1].disposition, RouteFrontierDisposition::Dominated { .. }));
        assert!(rows.iter().all(|row| !row.preferred));
    }

    #[test]
    fn two_tradeoff_routes_remain_jointly_nondominated_in_stable_id_order() {
        let o = obligation();
        let a = route("a-high-gain", 20, 20);
        let b = route("b-low-cost", 10, 5);
        let rows = route_frontier_dispositions(&o, &[b, a]).unwrap();
        assert_eq!(rows.iter().map(|r| r.route_ref.as_str()).collect::<Vec<_>>(), vec!["a-high-gain", "b-low-cost"]);
        assert!(rows.iter().all(|r| matches!(r.disposition, RouteFrontierDisposition::FrontierExecutable)));
    }

    #[test]
    fn potential_reopening_cone_is_not_actual_reopening() {
        let cone = potential_reopening_cone("route:a", &[
            ("route:a".into(), "assessment:one".into()),
            ("assessment:one".into(), "assessment:two".into()),
        ]).unwrap();
        assert_eq!(cone.direct_refs, vec!["assessment:one"]);
        assert_eq!(cone.transitive_refs, vec!["assessment:two"]);
        assert!(!cone.creates_actual_reopening);
        assert!(!cone.creates_semantic_authority);
    }

    #[test]
    fn unknown_access_remains_blocked_not_deleted() {
        let o = obligation();
        let mut r = route("unknown", 5, 1);
        r.access_disposition = AccessDisposition::Unknown;
        let p = acquisition_pareto_frontier(&o, &[r]).unwrap();
        assert_eq!(p.frontier_route_refs, vec!["unknown"]);
        assert!(p.executable_frontier_route_refs.is_empty());
        assert_eq!(p.blocked_frontier_route_refs, vec!["unknown"]);
    }

    #[test]
    fn known_absence_closes_only_exact_acquisition_branch() {
        let o = obligation();
        let update = AcquisitionUpdate {
            obligation_ref: o.obligation_ref.clone(), before: RecordAvailability::NotLocated,
            after: RecordAvailability::KnownAbsent, acquired_source_revision_ref: None,
            acquisition_receipt_ref: "receipt:known-absent".into(),
        };
        apply_acquisition_update(&o, &update).unwrap();
    }

    #[test]
    fn selective_reopening_uses_only_dependency_paths() {
        let r = selective_reopening(
            "source:new", "graph:1",
            &[("source:new".into(), "assessment:a".into()), ("assessment:a".into(), "assessment:case".into())],
            &["assessment:a".into(), "assessment:case".into(), "assessment:unrelated".into()],
        ).unwrap();
        assert_eq!(r.directly_affected_refs, vec!["assessment:a"]);
        assert_eq!(r.transitively_affected_refs, vec!["assessment:case"]);
        assert_eq!(r.unrelated_refs_not_reopened, vec!["assessment:unrelated"]);
    }
}
