//! Consumer-directed research adequacy.
//!
//! This is a deliberately small runtime carrier for the already-formalised
//! DASHI FactorsThrough / NonFactorability idea.  Rust does not claim a proof
//! of factorisation.  It records which query-required coordinates are paid,
//! emits typed research demands for missing coordinates, and can explicitly
//! terminate unresolved when a missing coordinate is known closed.

use std::collections::BTreeSet;

use crate::ProjectionGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsumerAxis {
    SemanticIdentity,
    SourceRevision,
    SourceSpan,
    Provenance,
    Treatment,
    Temporal,
    Jurisdiction,
    FactualPredicate,
    BurdenOrException,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerQueryDemand {
    pub query_ref: String,
    pub required_axes: BTreeSet<ConsumerAxis>,
    pub required_semantic_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConsumerCoverage {
    /// Coordinates independently paid by the caller/world.
    pub paid_axes: BTreeSet<ConsumerAxis>,
    /// Missing coordinates which have been explicitly closed as unavailable,
    /// contested, or otherwise non-resolvable within the declared world.
    pub closed_unresolved_axes: BTreeSet<ConsumerAxis>,
    pub unresolved_refs: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsumerResearchDemandKind {
    AcquireSource,
    ReviewTreatment,
    ResolveTemporalCoordinate,
    ResolveJurisdiction,
    ReviewFact,
    ReviewBurdenOrException,
    RecoverProvenance,
    ResolveSemanticIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerResearchDemand {
    pub axis: ConsumerAxis,
    pub kind: ConsumerResearchDemandKind,
    pub target_ref: String,
    pub reason_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumerAdequacyDisposition {
    Adequate,
    NeedsResearch,
    ExplicitlyUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerAdequacyReceipt {
    pub query_ref: String,
    pub disposition: ConsumerAdequacyDisposition,
    pub paid_axes: BTreeSet<ConsumerAxis>,
    pub missing_axes: BTreeSet<ConsumerAxis>,
    pub missing_semantic_refs: BTreeSet<String>,
    pub research_demands: Vec<ConsumerResearchDemand>,
    pub unresolved_refs: BTreeSet<String>,
    /// Runtime accounting is not itself a formal FactorsThrough proof.
    pub factors_through_formally_proved: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn research_kind(axis: ConsumerAxis) -> ConsumerResearchDemandKind {
    match axis {
        ConsumerAxis::SemanticIdentity => ConsumerResearchDemandKind::ResolveSemanticIdentity,
        ConsumerAxis::SourceRevision | ConsumerAxis::SourceSpan => {
            ConsumerResearchDemandKind::AcquireSource
        }
        ConsumerAxis::Provenance => ConsumerResearchDemandKind::RecoverProvenance,
        ConsumerAxis::Treatment => ConsumerResearchDemandKind::ReviewTreatment,
        ConsumerAxis::Temporal => ConsumerResearchDemandKind::ResolveTemporalCoordinate,
        ConsumerAxis::Jurisdiction => ConsumerResearchDemandKind::ResolveJurisdiction,
        ConsumerAxis::FactualPredicate => ConsumerResearchDemandKind::ReviewFact,
        ConsumerAxis::BurdenOrException => {
            ConsumerResearchDemandKind::ReviewBurdenOrException
        }
    }
}

fn graph_paid_axes(
    graph: &ProjectionGraph,
    required_semantic_refs: &BTreeSet<String>,
) -> BTreeSet<ConsumerAxis> {
    let required_nodes = required_semantic_refs
        .iter()
        .filter_map(|reference| {
            graph
                .nodes
                .iter()
                .find(|node| node.semantic_ref == *reference)
        })
        .collect::<Vec<_>>();

    let mut axes = BTreeSet::new();
    if required_nodes.len() == required_semantic_refs.len() {
        axes.insert(ConsumerAxis::SemanticIdentity);
    }
    if !required_nodes.is_empty()
        && required_nodes
            .iter()
            .all(|node| !node.source_revision_refs.is_empty())
    {
        axes.insert(ConsumerAxis::SourceRevision);
    }
    if !required_nodes.is_empty()
        && required_nodes.iter().all(|node| !node.span_refs.is_empty())
    {
        axes.insert(ConsumerAxis::SourceSpan);
    }
    if axes.contains(&ConsumerAxis::SourceRevision)
        && axes.contains(&ConsumerAxis::SourceSpan)
    {
        axes.insert(ConsumerAxis::Provenance);
    }
    axes
}

pub fn assess_consumer_adequacy(
    demand: &ConsumerQueryDemand,
    graph: &ProjectionGraph,
    coverage: &ConsumerCoverage,
) -> Result<ConsumerAdequacyReceipt, String> {
    if demand.query_ref.trim().is_empty() {
        return Err("consumer query_ref must be non-empty".into());
    }
    if !demand.candidate_only || demand.creates_semantic_authority {
        return Err("consumer demand crossed candidate-only boundary".into());
    }
    if !graph.projection_only || graph.creates_semantic_authority {
        return Err("consumer adequacy requires a non-authoritative projection".into());
    }

    let visible = graph
        .nodes
        .iter()
        .map(|node| node.semantic_ref.clone())
        .collect::<BTreeSet<_>>();
    let missing_semantic_refs = demand
        .required_semantic_refs
        .difference(&visible)
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut paid_axes = coverage.paid_axes.clone();
    paid_axes.extend(graph_paid_axes(graph, &demand.required_semantic_refs));

    let mut missing_axes = demand
        .required_axes
        .difference(&paid_axes)
        .copied()
        .collect::<BTreeSet<_>>();
    if !missing_semantic_refs.is_empty() {
        missing_axes.insert(ConsumerAxis::SemanticIdentity);
    }

    let disposition = if missing_axes.is_empty() && missing_semantic_refs.is_empty() {
        ConsumerAdequacyDisposition::Adequate
    } else if missing_axes
        .iter()
        .all(|axis| coverage.closed_unresolved_axes.contains(axis))
    {
        ConsumerAdequacyDisposition::ExplicitlyUnresolved
    } else {
        ConsumerAdequacyDisposition::NeedsResearch
    };

    let target = demand
        .required_semantic_refs
        .iter()
        .next()
        .cloned()
        .unwrap_or_else(|| demand.query_ref.clone());
    let research_demands = if disposition == ConsumerAdequacyDisposition::NeedsResearch {
        missing_axes
            .iter()
            .filter(|axis| !coverage.closed_unresolved_axes.contains(axis))
            .map(|axis| ConsumerResearchDemand {
                axis: *axis,
                kind: research_kind(*axis),
                target_ref: target.clone(),
                reason_ref: format!("consumer-nonadequacy:{}:{axis:?}", demand.query_ref),
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(ConsumerAdequacyReceipt {
        query_ref: demand.query_ref.clone(),
        disposition,
        paid_axes,
        missing_axes,
        missing_semantic_refs,
        research_demands,
        unresolved_refs: coverage.unresolved_refs.clone(),
        factors_through_formally_proved: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProjectionEdge, ProjectionKind, ProjectionNode};

    fn graph() -> ProjectionGraph {
        ProjectionGraph {
            kind: ProjectionKind::IssueProof,
            nodes: vec![ProjectionNode {
                semantic_ref: "case:fixture".into(),
                semantic_kind: "CaseAuthority".into(),
                manifestation_refs: vec!["manifestation:fixture".into()],
                source_revision_refs: vec!["revision:fixture".into()],
                span_refs: vec!["span:fixture".into()],
                projection_role: "IssueProof".into(),
            }],
            edges: vec![ProjectionEdge {
                from_ref: "case:fixture".into(),
                to_ref: "issue:fixture".into(),
                relation: "supports".into(),
            }],
            deterministic_digest: "sha256:fixture".into(),
            projection_only: true,
            creates_semantic_authority: false,
        }
    }

    #[test]
    fn source_provenance_can_be_paid_by_projection_while_treatment_stays_live() {
        let demand = ConsumerQueryDemand {
            query_ref: "query:fixture".into(),
            required_axes: BTreeSet::from([
                ConsumerAxis::SemanticIdentity,
                ConsumerAxis::SourceRevision,
                ConsumerAxis::SourceSpan,
                ConsumerAxis::Provenance,
                ConsumerAxis::Treatment,
            ]),
            required_semantic_refs: BTreeSet::from(["case:fixture".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        };
        let receipt =
            assess_consumer_adequacy(&demand, &graph(), &ConsumerCoverage::default()).unwrap();

        assert_eq!(receipt.disposition, ConsumerAdequacyDisposition::NeedsResearch);
        assert!(receipt.paid_axes.contains(&ConsumerAxis::Provenance));
        assert_eq!(receipt.missing_axes, BTreeSet::from([ConsumerAxis::Treatment]));
        assert_eq!(receipt.research_demands.len(), 1);
        assert_eq!(
            receipt.research_demands[0].kind,
            ConsumerResearchDemandKind::ReviewTreatment
        );
        assert!(!receipt.factors_through_formally_proved);
    }

    #[test]
    fn all_required_axes_paid_is_runtime_adequate_without_truth_promotion() {
        let demand = ConsumerQueryDemand {
            query_ref: "query:fixture".into(),
            required_axes: BTreeSet::from([
                ConsumerAxis::SemanticIdentity,
                ConsumerAxis::SourceRevision,
                ConsumerAxis::SourceSpan,
                ConsumerAxis::Provenance,
                ConsumerAxis::Treatment,
                ConsumerAxis::Temporal,
                ConsumerAxis::Jurisdiction,
            ]),
            required_semantic_refs: BTreeSet::from(["case:fixture".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        };
        let coverage = ConsumerCoverage {
            paid_axes: BTreeSet::from([
                ConsumerAxis::Treatment,
                ConsumerAxis::Temporal,
                ConsumerAxis::Jurisdiction,
            ]),
            ..ConsumerCoverage::default()
        };
        let receipt = assess_consumer_adequacy(&demand, &graph(), &coverage).unwrap();

        assert_eq!(receipt.disposition, ConsumerAdequacyDisposition::Adequate);
        assert!(receipt.missing_axes.is_empty());
        assert!(receipt.research_demands.is_empty());
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.creates_claim_truth);
        assert!(!receipt.factors_through_formally_proved);
    }

    #[test]
    fn closed_missing_axis_terminates_explicitly_unresolved() {
        let demand = ConsumerQueryDemand {
            query_ref: "query:fixture".into(),
            required_axes: BTreeSet::from([ConsumerAxis::Treatment]),
            required_semantic_refs: BTreeSet::from(["case:fixture".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        };
        let coverage = ConsumerCoverage {
            closed_unresolved_axes: BTreeSet::from([ConsumerAxis::Treatment]),
            unresolved_refs: BTreeSet::from(["residual:treatment:unavailable".into()]),
            ..ConsumerCoverage::default()
        };
        let receipt = assess_consumer_adequacy(&demand, &graph(), &coverage).unwrap();

        assert_eq!(
            receipt.disposition,
            ConsumerAdequacyDisposition::ExplicitlyUnresolved
        );
        assert!(receipt.research_demands.is_empty());
        assert!(receipt
            .unresolved_refs
            .contains("residual:treatment:unavailable"));
    }
}
