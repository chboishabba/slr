//! S19 shared-world / cross-consumer dependency runtime.
//!
//! A shared coordinate is reusable only when a consumer dependency slice says
//! the coordinate is required and a reviewed join witness admits that exact
//! coordinate for that consumer.  Graph adjacency, citation co-occurrence,
//! ontology proximity, or same-QID observations may propose joins, but cannot
//! establish them.
//!
//! Reviewed world deltas are indexed back to the consumers whose dependency
//! slices contain the admitted coordinate.  Already-paid coordinates quotient
//! out before LegalFollow acquisition so cross-matter reuse does not regenerate
//! research work.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SharedCoordinateKind {
    Evidence,
    Proposition,
    Authority,
    LegalAtom,
    Fact,
    Event,
    Claim,
    Context,
    Treatment,
    SourceRevision,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SharedWorldCoordinate {
    pub coordinate_ref: String,
    pub kind: SharedCoordinateKind,
    pub semantic_ref: String,
    pub source_revision_refs: BTreeSet<String>,
    pub provenance_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl SharedWorldCoordinate {
    pub fn validate(&self) -> Result<(), String> {
        if self.coordinate_ref.trim().is_empty() || self.semantic_ref.trim().is_empty() {
            return Err("shared coordinate requires coordinate_ref and semantic_ref".into());
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("shared coordinate crossed non-promotion boundary".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerDependencySlice {
    pub consumer_ref: String,
    pub required_coordinate_refs: BTreeSet<String>,
    pub optional_coordinate_refs: BTreeSet<String>,
    pub dependency_slice_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ConsumerDependencySlice {
    pub fn validate(&self) -> Result<(), String> {
        if self.consumer_ref.trim().is_empty() || self.dependency_slice_ref.trim().is_empty() {
            return Err("consumer dependency slice requires refs".into());
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("consumer dependency slice crossed non-promotion boundary".into());
        }
        Ok(())
    }

    #[must_use]
    pub fn requires(&self, coordinate_ref: &str) -> bool {
        self.required_coordinate_refs.contains(coordinate_ref)
    }

    #[must_use]
    pub fn mentions(&self, coordinate_ref: &str) -> bool {
        self.required_coordinate_refs.contains(coordinate_ref)
            || self.optional_coordinate_refs.contains(coordinate_ref)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JoinProposalBasis {
    Citation,
    Treatment,
    SameSource,
    SameSemanticRef,
    SameQid,
    OntologyAdjacency,
    ConceptAdjacency,
    ExplicitDependency,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedJoinProposal {
    pub proposal_ref: String,
    pub source_consumer_ref: String,
    pub target_consumer_ref: String,
    pub coordinate_ref: String,
    pub basis: JoinProposalBasis,
    pub evidence_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewedSharedJoinWitness {
    pub witness_ref: String,
    pub source_consumer_ref: String,
    pub target_consumer_ref: String,
    pub coordinate_ref: String,
    pub dependency_slice_ref: String,
    pub review_ref: String,
    pub evidence_refs: BTreeSet<String>,
    pub dependency_membership_reviewed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn review_shared_join(
    proposal: &SharedJoinProposal,
    target_slice: &ConsumerDependencySlice,
    review_ref: impl Into<String>,
) -> Result<ReviewedSharedJoinWitness, String> {
    target_slice.validate()?;
    if proposal.proposal_ref.trim().is_empty()
        || proposal.coordinate_ref.trim().is_empty()
        || proposal.target_consumer_ref != target_slice.consumer_ref
    {
        return Err("shared join proposal does not match target consumer slice".into());
    }
    if !proposal.candidate_only
        || proposal.creates_semantic_authority
        || proposal.creates_claim_truth
    {
        return Err("shared join proposal crossed non-promotion boundary".into());
    }
    // A proposal is not enough.  The exact coordinate must already occur in
    // the target consumer's reviewed dependency slice.
    if !target_slice.mentions(&proposal.coordinate_ref) {
        return Err(format!(
            "coordinate {} is not in target consumer dependency slice {}",
            proposal.coordinate_ref, target_slice.dependency_slice_ref
        ));
    }
    let review_ref = review_ref.into();
    if review_ref.trim().is_empty() {
        return Err("shared join review_ref must be non-empty".into());
    }

    Ok(ReviewedSharedJoinWitness {
        witness_ref: format!(
            "shared-join:{}:{}:{}",
            proposal.source_consumer_ref, proposal.target_consumer_ref, proposal.coordinate_ref
        ),
        source_consumer_ref: proposal.source_consumer_ref.clone(),
        target_consumer_ref: proposal.target_consumer_ref.clone(),
        coordinate_ref: proposal.coordinate_ref.clone(),
        dependency_slice_ref: target_slice.dependency_slice_ref.clone(),
        review_ref,
        evidence_refs: proposal.evidence_refs.clone(),
        dependency_membership_reviewed: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedWorld {
    pub coordinates: BTreeMap<String, SharedWorldCoordinate>,
    pub dependency_slices: BTreeMap<String, ConsumerDependencySlice>,
    pub reviewed_joins: BTreeMap<(String, String), ReviewedSharedJoinWitness>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl SharedWorld {
    pub fn new() -> Self {
        Self {
            coordinates: BTreeMap::new(),
            dependency_slices: BTreeMap::new(),
            reviewed_joins: BTreeMap::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    pub fn admit_coordinate(&mut self, coordinate: SharedWorldCoordinate) -> Result<bool, String> {
        coordinate.validate()?;
        if let Some(existing) = self.coordinates.get(&coordinate.coordinate_ref) {
            if existing != &coordinate {
                return Err(format!(
                    "shared coordinate {} conflicts with existing admitted coordinate",
                    coordinate.coordinate_ref
                ));
            }
            return Ok(false);
        }
        self.coordinates
            .insert(coordinate.coordinate_ref.clone(), coordinate);
        Ok(true)
    }

    pub fn set_dependency_slice(&mut self, slice: ConsumerDependencySlice) -> Result<(), String> {
        slice.validate()?;
        self.dependency_slices.insert(slice.consumer_ref.clone(), slice);
        Ok(())
    }

    pub fn admit_reviewed_join(
        &mut self,
        witness: ReviewedSharedJoinWitness,
    ) -> Result<bool, String> {
        if !witness.dependency_membership_reviewed
            || !witness.candidate_only
            || witness.creates_semantic_authority
            || witness.creates_claim_truth
        {
            return Err("reviewed join witness crossed admission boundary".into());
        }
        let slice = self
            .dependency_slices
            .get(&witness.target_consumer_ref)
            .ok_or_else(|| "reviewed join target has no dependency slice".to_string())?;
        if slice.dependency_slice_ref != witness.dependency_slice_ref
            || !slice.mentions(&witness.coordinate_ref)
        {
            return Err("reviewed join no longer matches target dependency slice".into());
        }
        if !self.coordinates.contains_key(&witness.coordinate_ref) {
            return Err("reviewed join coordinate is not admitted to shared world".into());
        }
        let key = (
            witness.target_consumer_ref.clone(),
            witness.coordinate_ref.clone(),
        );
        if let Some(existing) = self.reviewed_joins.get(&key) {
            if existing != &witness {
                return Err("reviewed join conflicts with existing witness".into());
            }
            return Ok(false);
        }
        self.reviewed_joins.insert(key, witness);
        Ok(true)
    }

    #[must_use]
    pub fn coordinate_paid_for_consumer(
        &self,
        consumer_ref: &str,
        coordinate_ref: &str,
    ) -> bool {
        self.coordinates.contains_key(coordinate_ref)
            && self
                .reviewed_joins
                .contains_key(&(consumer_ref.to_owned(), coordinate_ref.to_owned()))
    }

    #[must_use]
    pub fn affected_consumers(&self, coordinate_ref: &str) -> BTreeSet<String> {
        self.dependency_slices
            .iter()
            .filter_map(|(consumer_ref, slice)| {
                slice
                    .mentions(coordinate_ref)
                    .then_some(consumer_ref.clone())
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedWorldQuotientReceipt {
    pub consumer_ref: String,
    pub reused_coordinate_refs: BTreeSet<String>,
    pub remaining_residual_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn quotient_frontier_against_shared_world(
    world: &SharedWorld,
    frontier: &ProofFrontier,
) -> Result<SharedWorldQuotientReceipt, String> {
    let slice = world
        .dependency_slices
        .get(&frontier.consumer_ref)
        .ok_or_else(|| format!("consumer {} has no dependency slice", frontier.consumer_ref))?;

    let mut reused = BTreeSet::new();
    let mut remaining = BTreeSet::new();

    for residual in frontier.open_residuals() {
        let paid = !residual.dependency_refs.is_empty()
            && residual.dependency_refs.iter().all(|coordinate_ref| {
                slice.requires(coordinate_ref)
                    && world.coordinate_paid_for_consumer(
                        &frontier.consumer_ref,
                        coordinate_ref,
                    )
            });

        if paid {
            reused.extend(residual.dependency_refs.iter().cloned());
        } else {
            remaining.insert(residual.residual_ref.clone());
        }
    }

    Ok(SharedWorldQuotientReceipt {
        consumer_ref: frontier.consumer_ref.clone(),
        reused_coordinate_refs: reused,
        remaining_residual_refs: remaining,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

pub fn frontier_after_shared_world_quotient(
    frontier: &ProofFrontier,
    receipt: &SharedWorldQuotientReceipt,
) -> Result<ProofFrontier, String> {
    if receipt.consumer_ref != frontier.consumer_ref
        || !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.creates_claim_truth
    {
        return Err("shared-world quotient receipt does not match frontier".into());
    }

    let residuals = frontier
        .residuals
        .iter()
        .cloned()
        .map(|mut residual| {
            if residual.status == ResidualStatus::Open
                && !receipt.remaining_residual_refs.contains(&residual.residual_ref)
            {
                residual.status = ResidualStatus::SatisfiedCandidate;
            }
            residual
        })
        .collect::<Vec<ProofResidual>>();

    Ok(ProofFrontier {
        consumer_ref: frontier.consumer_ref.clone(),
        frontier_ref: format!("{}:shared-world-quotient", frontier.frontier_ref),
        residuals,
        satisfied_payment_refs: frontier
            .satisfied_payment_refs
            .iter()
            .cloned()
            .chain(
                receipt
                    .reused_coordinate_refs
                    .iter()
                    .map(|coordinate_ref| format!("shared-world-payment:{coordinate_ref}")),
            )
            .collect(),
        contested_coordinate_refs: frontier.contested_coordinate_refs.clone(),
        authority_blocked_refs: frontier.authority_blocked_refs.clone(),
        authority: frontier.authority,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedConsumerRecomputePlan {
    pub changed_coordinate_refs: BTreeSet<String>,
    pub consumer_refs: BTreeSet<String>,
    pub cause_refs_by_consumer: BTreeMap<String, BTreeSet<String>>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn affected_consumer_recompute_plan(
    world: &SharedWorld,
    changed_coordinate_refs: impl IntoIterator<Item = String>,
) -> Result<AffectedConsumerRecomputePlan, String> {
    let changed_coordinate_refs = changed_coordinate_refs.into_iter().collect::<BTreeSet<_>>();
    let mut cause_refs_by_consumer: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for coordinate_ref in &changed_coordinate_refs {
        if !world.coordinates.contains_key(coordinate_ref) {
            return Err(format!(
                "cannot propagate unknown shared-world coordinate {coordinate_ref}"
            ));
        }
        for consumer_ref in world.affected_consumers(coordinate_ref) {
            cause_refs_by_consumer
                .entry(consumer_ref)
                .or_default()
                .insert(coordinate_ref.clone());
        }
    }

    Ok(AffectedConsumerRecomputePlan {
        changed_coordinate_refs,
        consumer_refs: cause_refs_by_consumer.keys().cloned().collect(),
        cause_refs_by_consumer,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coordinate(reference: &str) -> SharedWorldCoordinate {
        SharedWorldCoordinate {
            coordinate_ref: reference.into(),
            kind: SharedCoordinateKind::Authority,
            semantic_ref: format!("semantic:{reference}"),
            source_revision_refs: BTreeSet::from(["revision:1".into()]),
            provenance_refs: BTreeSet::from(["review:1".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn slice(consumer: &str, required: &[&str]) -> ConsumerDependencySlice {
        ConsumerDependencySlice {
            consumer_ref: consumer.into(),
            required_coordinate_refs: required.iter().map(|value| (*value).into()).collect(),
            optional_coordinate_refs: BTreeSet::new(),
            dependency_slice_ref: format!("slice:{consumer}"),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn frontier(consumer: &str, coordinate: &str) -> ProofFrontier {
        ProofFrontier {
            consumer_ref: consumer.into(),
            frontier_ref: format!("frontier:{consumer}"),
            residuals: vec![ProofResidual {
                residual_ref: format!("residual:{consumer}:authority"),
                proposition_ref: "proposition:native-title-authority".into(),
                producer_class_ref: "producer:authority".into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: Some("authority:HCA".into()),
                salience: 100,
                dependency_refs: vec![coordinate.into()],
                status: ResidualStatus::Open,
            }],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "candidate-only",
        }
    }

    #[test]
    fn adjacency_proposal_does_not_establish_join_outside_dependency_slice() {
        let proposal = SharedJoinProposal {
            proposal_ref: "proposal:munkara-mabo".into(),
            source_consumer_ref: "consumer:mabo".into(),
            target_consumer_ref: "consumer:munkara".into(),
            coordinate_ref: "coordinate:mabo-native-title".into(),
            basis: JoinProposalBasis::ConceptAdjacency,
            evidence_refs: BTreeSet::from(["citation-candidate:1".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let target = slice("consumer:munkara", &["coordinate:offshore-consultation"]);
        assert!(review_shared_join(&proposal, &target, "review:operator").is_err());
    }

    #[test]
    fn reviewed_dependency_join_reuses_paid_coordinate_without_collapsing_consumers() {
        let coordinate_ref = "coordinate:mabo-native-title";
        let mut world = SharedWorld::new();
        world.admit_coordinate(coordinate(coordinate_ref)).unwrap();
        world
            .set_dependency_slice(slice("consumer:mabo", &[coordinate_ref]))
            .unwrap();
        world
            .set_dependency_slice(slice("consumer:yindjibarndi", &[coordinate_ref]))
            .unwrap();

        let proposal = SharedJoinProposal {
            proposal_ref: "proposal:yindjibarndi-mabo".into(),
            source_consumer_ref: "consumer:mabo".into(),
            target_consumer_ref: "consumer:yindjibarndi".into(),
            coordinate_ref: coordinate_ref.into(),
            basis: JoinProposalBasis::Citation,
            evidence_refs: BTreeSet::from(["source:yindjibarndi:yunupingu-citation".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let target = world
            .dependency_slices
            .get("consumer:yindjibarndi")
            .unwrap()
            .clone();
        let witness = review_shared_join(&proposal, &target, "review:join").unwrap();
        world.admit_reviewed_join(witness).unwrap();

        let quotient =
            quotient_frontier_against_shared_world(&world, &frontier("consumer:yindjibarndi", coordinate_ref))
                .unwrap();
        assert!(quotient.remaining_residual_refs.is_empty());
        assert_eq!(
            quotient.reused_coordinate_refs,
            BTreeSet::from([coordinate_ref.into()])
        );
        assert_ne!("consumer:mabo", "consumer:yindjibarndi");
        assert!(!quotient.creates_claim_truth);
    }

    #[test]
    fn optional_dependency_does_not_discharge_residual() {
        let coordinate_ref = "coordinate:optional-context";
        let mut world = SharedWorld::new();
        world.admit_coordinate(coordinate(coordinate_ref)).unwrap();
        world
            .set_dependency_slice(ConsumerDependencySlice {
                consumer_ref: "consumer:test".into(),
                required_coordinate_refs: BTreeSet::new(),
                optional_coordinate_refs: BTreeSet::from([coordinate_ref.into()]),
                dependency_slice_ref: "slice:test".into(),
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            })
            .unwrap();

        let proposal = SharedJoinProposal {
            proposal_ref: "proposal:test:optional".into(),
            source_consumer_ref: "consumer:source".into(),
            target_consumer_ref: "consumer:test".into(),
            coordinate_ref: coordinate_ref.into(),
            basis: JoinProposalBasis::ConceptAdjacency,
            evidence_refs: BTreeSet::from(["candidate:context".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let target = world.dependency_slices["consumer:test"].clone();
        let witness = review_shared_join(&proposal, &target, "review:optional").unwrap();
        world.admit_reviewed_join(witness).unwrap();

        let quotient =
            quotient_frontier_against_shared_world(
                &world,
                &frontier("consumer:test", coordinate_ref),
            )
            .unwrap();
        assert_eq!(
            quotient.remaining_residual_refs,
            BTreeSet::from(["residual:consumer:test:authority".into()])
        );
        assert!(quotient.reused_coordinate_refs.is_empty());
    }

    #[test]
    fn reviewed_delta_recomputes_every_dependent_consumer_only() {
        let coordinate_ref = "coordinate:yunupingu-compensation";
        let mut world = SharedWorld::new();
        world.admit_coordinate(coordinate(coordinate_ref)).unwrap();
        world
            .set_dependency_slice(slice("consumer:yindjibarndi", &[coordinate_ref]))
            .unwrap();
        world
            .set_dependency_slice(slice("consumer:colonisation", &[coordinate_ref]))
            .unwrap();
        world
            .set_dependency_slice(slice("consumer:pabai", &["coordinate:climate-duty"]))
            .unwrap();

        let plan = affected_consumer_recompute_plan(
            &world,
            [coordinate_ref.to_owned()],
        )
        .unwrap();
        assert_eq!(
            plan.consumer_refs,
            BTreeSet::from([
                "consumer:yindjibarndi".into(),
                "consumer:colonisation".into(),
            ])
        );
        assert!(!plan.consumer_refs.contains("consumer:pabai"));
    }
}
