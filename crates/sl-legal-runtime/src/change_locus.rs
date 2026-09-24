//! M11.1 / S26.9 typed change-locus IR.
//!
//! This wraps the already-green ComparativeDelta ABI rather than changing it.
//! The same delta kind can occur at different epistemic/operational layers;
//! callers must therefore state where the change occurred before asking whether
//! it is query relevant or answer changing.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{ComparativeDelta, ComparativeDeltaKind, ComparativeDeltaRole};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ChangeLayer {
    World,
    WorldEvidence,
    Observation,
    Representation,
    Theory,
    Belief,
    ConsumerProjection,
    Review,
    Scope,
    Applicability,
    ProofOutcome,
    ResidualOutcome,
}

impl ChangeLayer {
    #[must_use]
    pub fn is_outcome(self) -> bool {
        matches!(self, Self::ProofOutcome | Self::ResidualOutcome)
    }

    #[must_use]
    pub fn is_world(self) -> bool {
        matches!(self, Self::World | Self::WorldEvidence)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeLocus {
    pub locus_ref: String,
    pub delta: ComparativeDelta,
    pub layer: ChangeLayer,
    /// Optional more specific sub-layer, e.g. "law-description",
    /// "instrument-resolution", "consumer-share-scope".
    pub sublayer_ref: Option<String>,
    pub invariant_layer_refs: BTreeSet<String>,
    pub justification_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ChangeLocus {
    pub fn validate(&self) -> Result<(), String> {
        self.delta.validate()?;
        if self.locus_ref.trim().is_empty() {
            return Err("change locus requires locus_ref".into());
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("change locus crossed non-promotion boundary".into());
        }
        match self.delta.role {
            ComparativeDeltaRole::ProofOutcome
                if self.layer != ChangeLayer::ProofOutcome =>
            {
                return Err("proof-outcome delta must be located at ProofOutcome".into());
            }
            ComparativeDeltaRole::ResidualOutcome
                if self.layer != ChangeLayer::ResidualOutcome =>
            {
                return Err("residual-outcome delta must be located at ResidualOutcome".into());
            }
            ComparativeDeltaRole::WorldInput if self.layer.is_outcome() => {
                return Err("world-input delta cannot be located at outcome layer".into());
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedChangeSet {
    pub comparison_ref: String,
    pub loci: Vec<ChangeLocus>,
    pub delta_refs_by_layer: BTreeMap<ChangeLayer, BTreeSet<String>>,
    pub changed_layers: BTreeSet<ChangeLayer>,
    pub invariant_layers: BTreeSet<ChangeLayer>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn compile_typed_change_set(
    comparison_ref: &str,
    loci: impl IntoIterator<Item = ChangeLocus>,
    declared_invariant_layers: impl IntoIterator<Item = ChangeLayer>,
) -> Result<TypedChangeSet, String> {
    if comparison_ref.trim().is_empty() {
        return Err("typed change set requires comparison_ref".into());
    }
    let loci = loci.into_iter().collect::<Vec<_>>();
    let mut delta_refs_by_layer = BTreeMap::<ChangeLayer, BTreeSet<String>>::new();
    for locus in &loci {
        locus.validate()?;
        delta_refs_by_layer
            .entry(locus.layer)
            .or_default()
            .insert(locus.delta.delta_ref.clone());
    }
    let changed_layers = delta_refs_by_layer.keys().copied().collect::<BTreeSet<_>>();
    let invariant_layers = declared_invariant_layers.into_iter().collect::<BTreeSet<_>>();
    if !changed_layers.is_disjoint(&invariant_layers) {
        return Err("same change layer declared both changed and invariant".into());
    }

    Ok(TypedChangeSet {
        comparison_ref: comparison_ref.into(),
        loci,
        delta_refs_by_layer,
        changed_layers,
        invariant_layers,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[must_use]
pub fn layer_compatible_with_kind(
    kind: ComparativeDeltaKind,
    layer: ChangeLayer,
) -> bool {
    match kind {
        ComparativeDeltaKind::ReviewStateChanged => layer == ChangeLayer::Review,
        ComparativeDeltaKind::ScopeChanged => {
            matches!(layer, ChangeLayer::Scope | ChangeLayer::ConsumerProjection)
        }
        ComparativeDeltaKind::ApplicabilityChanged
        | ComparativeDeltaKind::DefeaterAdded
        | ComparativeDeltaKind::DefeaterRemoved
        | ComparativeDeltaKind::CounterDefeaterAdded
        | ComparativeDeltaKind::CounterDefeaterRemoved => {
            layer == ChangeLayer::Applicability
        }
        ComparativeDeltaKind::ResidualOpened
        | ComparativeDeltaKind::ResidualClosed => layer == ChangeLayer::ResidualOutcome,
        ComparativeDeltaKind::RouteStatusChanged => layer == ChangeLayer::ProofOutcome,
        ComparativeDeltaKind::ConsumerDependencyChanged => {
            layer == ChangeLayer::ConsumerProjection
        }
        ComparativeDeltaKind::AsAtChanged
        | ComparativeDeltaKind::JurisdictionChanged => {
            matches!(layer, ChangeLayer::World | ChangeLayer::WorldEvidence)
        }
        ComparativeDeltaKind::AuthorityChanged => {
            matches!(layer, ChangeLayer::WorldEvidence | ChangeLayer::Review)
        }
        ComparativeDeltaKind::FactAdded
        | ComparativeDeltaKind::FactRemoved
        | ComparativeDeltaKind::FactChanged => {
            matches!(
                layer,
                ChangeLayer::World
                    | ChangeLayer::WorldEvidence
                    | ChangeLayer::Observation
                    | ChangeLayer::Representation
                    | ChangeLayer::Theory
                    | ChangeLayer::Belief
            )
        }
    }
}

pub fn typed_locus(
    locus_ref: &str,
    delta: ComparativeDelta,
    layer: ChangeLayer,
    sublayer_ref: Option<String>,
    invariant_layer_refs: BTreeSet<String>,
    justification_refs: BTreeSet<String>,
) -> Result<ChangeLocus, String> {
    if !layer_compatible_with_kind(delta.kind, layer) {
        return Err(format!(
            "delta kind {:?} is incompatible with layer {:?}",
            delta.kind, layer
        ));
    }
    let locus = ChangeLocus {
        locus_ref: locus_ref.into(),
        delta,
        layer,
        sublayer_ref,
        invariant_layer_refs,
        justification_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    locus.validate()?;
    Ok(locus)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact_delta(reference: &str) -> ComparativeDelta {
        ComparativeDelta {
            delta_ref: reference.into(),
            kind: ComparativeDeltaKind::FactChanged,
            role: ComparativeDeltaRole::WorldInput,
            coordinate_ref: Some("coordinate:x".into()),
            route_ref: None,
            residual_ref: None,
            before_ref: Some("before".into()),
            after_ref: Some("after".into()),
            cause_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    #[test]
    fn theory_change_can_explicitly_hold_world_invariant() {
        let locus = typed_locus(
            "locus:theory",
            fact_delta("delta:theory"),
            ChangeLayer::Theory,
            None,
            BTreeSet::from(["layer:world".into()]),
            BTreeSet::from(["DASHI.Core.WorldRepresentationSeparationExact".into()]),
        )
        .unwrap();
        let set = compile_typed_change_set(
            "comparison:theory",
            [locus],
            [ChangeLayer::World],
        )
        .unwrap();
        assert!(set.changed_layers.contains(&ChangeLayer::Theory));
        assert!(set.invariant_layers.contains(&ChangeLayer::World));
        assert!(!set.changed_layers.contains(&ChangeLayer::World));
    }

    #[test]
    fn proof_outcome_cannot_be_mislabeled_world_change() {
        let delta = ComparativeDelta {
            delta_ref: "delta:route".into(),
            kind: ComparativeDeltaKind::RouteStatusChanged,
            role: ComparativeDeltaRole::ProofOutcome,
            coordinate_ref: None,
            route_ref: Some("route:x".into()),
            residual_ref: None,
            before_ref: Some("open".into()),
            after_ref: Some("closed".into()),
            cause_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        assert!(typed_locus(
            "locus:bad",
            delta,
            ChangeLayer::World,
            None,
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .is_err());
    }

    #[test]
    fn applicability_delta_is_not_a_theory_delta() {
        let delta = ComparativeDelta {
            delta_ref: "delta:defeater".into(),
            kind: ComparativeDeltaKind::DefeaterAdded,
            role: ComparativeDeltaRole::WorldInput,
            coordinate_ref: Some("coordinate:defeater".into()),
            route_ref: None,
            residual_ref: None,
            before_ref: None,
            after_ref: Some("present".into()),
            cause_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        assert!(typed_locus(
            "locus:bad",
            delta,
            ChangeLayer::Theory,
            None,
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .is_err());
    }
}
