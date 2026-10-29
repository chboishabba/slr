//! Mabo/public-law adapter for the domain-generic LegalFollow campaign kernel.
//!
//! This is intentionally an adapter, not a second campaign controller.  The
//! existing Mabo world-expansion runtime still owns diagnosis, review,
//! acquisition and persistence.  The generic LegalFollow kernel owns only the
//! recurrence: recompute residuals -> select demand -> accept reviewed delta.

use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_legal_runtime::{
    GenericCampaignBudget, GenericCampaignStop, GenericLegalFollowCampaign,
    LegalFollowCampaignDomain,
};

use crate::{
    diagnose_mabo_context_world_identity, MaboConsumerDiagnosis,
    MaboIdentityDiagnosisRow, MaboIdentityReviewAssignment,
};
use sensiblaw_pg_source_store::{DiscoveryIdentityBaseline, LatentWorldRows};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboLegalFollowWorld {
    pub diagnosis: MaboConsumerDiagnosis,
    pub reviewed_identity_classes: BTreeMap<String, String>,
    pub review_refs: BTreeMap<String, String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl MaboLegalFollowWorld {
    pub fn from_diagnosis(diagnosis: MaboConsumerDiagnosis) -> Result<Self, String> {
        if diagnosis.creates_semantic_authority
            || diagnosis.applicability_promoted
            || diagnosis.claim_truth_promoted
        {
            return Err("Mabo diagnosis crossed non-promotion boundary".into());
        }
        Ok(Self {
            diagnosis,
            reviewed_identity_classes: BTreeMap::new(),
            review_refs: BTreeMap::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        })
    }

    #[must_use]
    pub fn is_paid(&self, representation_ref: &str) -> bool {
        self.reviewed_identity_classes.contains_key(representation_ref)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboLegalFollowResidual {
    pub row: MaboIdentityDiagnosisRow,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboLegalFollowDemand {
    pub representation_ref: String,
    pub residual_ref: String,
    pub source_revision_refs: Vec<String>,
    pub relation_type_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

pub fn pending_mabo_identity_review_bundle(
    demand: &MaboLegalFollowDemand,
) -> Result<String, String> {
    if demand.representation_ref.trim().is_empty()
        || demand.residual_ref.trim().is_empty()
        || !demand.candidate_only
        || demand.creates_semantic_authority
        || demand.applicability_promoted
        || demand.claim_truth_promoted
    {
        return Err("cannot render pending bundle for invalid/promoting Mabo demand".into());
    }

    let mut source_revisions = demand.source_revision_refs.clone();
    source_revisions.sort();
    source_revisions.dedup();
    let mut relation_types = demand.relation_type_refs.clone();
    relation_types.sort();
    relation_types.dedup();

    let mut out = String::new();
    out.push_str("# status=IdentityReviewRequired\n");
    out.push_str(&format!(
        "# representation_ref={}\n",
        demand.representation_ref
    ));
    out.push_str(&format!("# residual_ref={}\n", demand.residual_ref));
    for revision in source_revisions {
        out.push_str(&format!("# source_revision_ref={revision}\n"));
    }
    for relation in relation_types {
        out.push_str(&format!("# relation_type_ref={relation}\n"));
    }
    out.push_str(&format!(
        "# review_manifest_template\t{}\tworld-object:<reviewed-id>\treview:<operator-ref>\n",
        demand.representation_ref
    ));
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboReviewedIdentityDelta {
    pub assignment: MaboIdentityReviewAssignment,
    pub triggering_residual_ref: String,
    pub source_revision_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl MaboReviewedIdentityDelta {
    pub fn from_reviewed_assignment(
        residual: &MaboLegalFollowResidual,
        assignment: MaboIdentityReviewAssignment,
    ) -> Result<Self, String> {
        if assignment.representation_ref != residual.row.representation_ref {
            return Err(format!(
                "Mabo reviewed assignment {} does not pay residual representation {}",
                assignment.representation_ref, residual.row.representation_ref
            ));
        }
        if assignment.identity_class_ref.trim().is_empty() || assignment.review_ref.trim().is_empty()
        {
            return Err("Mabo reviewed identity delta requires identity class and review refs".into());
        }
        Ok(Self {
            assignment,
            triggering_residual_ref: residual.row.residual_ref.clone(),
            source_revision_refs: residual.row.source_revision_refs.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaboLegalFollowDomain;

impl LegalFollowCampaignDomain for MaboLegalFollowDomain {
    type World = MaboLegalFollowWorld;
    type Residual = MaboLegalFollowResidual;
    type Demand = MaboLegalFollowDemand;
    type Delta = MaboReviewedIdentityDelta;

    fn recompute_residuals(&self, world: &Self::World) -> Vec<Self::Residual> {
        let mut rows = world
            .diagnosis
            .rows
            .iter()
            .filter(|row| !world.is_paid(&row.representation_ref))
            .cloned()
            .map(|row| MaboLegalFollowResidual {
                row,
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            left.row
                .representation_ref
                .cmp(&right.row.representation_ref)
        });
        rows
    }

    fn select_fresh(
        &self,
        _world: &Self::World,
        residuals: &[Self::Residual],
    ) -> Option<Self::Demand> {
        residuals.first().map(|residual| MaboLegalFollowDemand {
            representation_ref: residual.row.representation_ref.clone(),
            residual_ref: residual.row.residual_ref.clone(),
            source_revision_refs: residual.row.source_revision_refs.clone(),
            relation_type_refs: residual.row.relation_type_refs.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        })
    }

    fn apply_reviewed_delta(
        &self,
        world: &Self::World,
        delta: &Self::Delta,
    ) -> Result<Self::World, String> {
        if !delta.candidate_only
            || delta.creates_semantic_authority
            || delta.applicability_promoted
            || delta.claim_truth_promoted
        {
            return Err("Mabo reviewed delta crossed non-promotion boundary".into());
        }

        let row = world
            .diagnosis
            .rows
            .iter()
            .find(|row| row.representation_ref == delta.assignment.representation_ref)
            .ok_or_else(|| {
                format!(
                    "Mabo reviewed delta has no current diagnosed residual for {}",
                    delta.assignment.representation_ref
                )
            })?;
        if row.residual_ref != delta.triggering_residual_ref {
            return Err(format!(
                "Mabo reviewed delta residual mismatch: expected {}, got {}",
                row.residual_ref, delta.triggering_residual_ref
            ));
        }
        if !delta.source_revision_refs.iter().all(|revision| {
            row.source_revision_refs.iter().any(|expected| expected == revision)
        }) {
            return Err("Mabo reviewed delta contains source revision outside diagnosed row".into());
        }
        if let Some(existing) = world
            .reviewed_identity_classes
            .get(&delta.assignment.representation_ref)
        {
            if existing != &delta.assignment.identity_class_ref {
                return Err(format!(
                    "Mabo representation {} already reviewed as different identity class",
                    delta.assignment.representation_ref
                ));
            }
        }

        let mut next = world.clone();
        next.reviewed_identity_classes.insert(
            delta.assignment.representation_ref.clone(),
            delta.assignment.identity_class_ref.clone(),
        );
        next.review_refs.insert(
            delta.assignment.representation_ref.clone(),
            delta.assignment.review_ref.clone(),
        );
        next.candidate_only = true;
        next.creates_semantic_authority = false;
        next.applicability_promoted = false;
        next.claim_truth_promoted = false;
        Ok(next)
    }
}

pub type MaboGenericLegalFollowCampaign =
    GenericLegalFollowCampaign<MaboLegalFollowDomain>;

pub fn mabo_generic_campaign_from_diagnosis(
    diagnosis: MaboConsumerDiagnosis,
    max_reviewed_deltas: usize,
) -> Result<MaboGenericLegalFollowCampaign, String> {
    let world = MaboLegalFollowWorld::from_diagnosis(diagnosis)?;
    GenericLegalFollowCampaign::new(
        MaboLegalFollowDomain,
        world,
        GenericCampaignBudget {
            max_reviewed_deltas,
        },
    )
}

pub fn mabo_generic_campaign_from_world(
    world: &LatentWorldRows,
    baseline: &DiscoveryIdentityBaseline,
    max_reviewed_deltas: usize,
) -> Result<MaboGenericLegalFollowCampaign, String> {
    mabo_generic_campaign_from_diagnosis(
        diagnose_mabo_context_world_identity(world, baseline),
        max_reviewed_deltas,
    )
}

/// Replay an already-reviewed Mabo identity sequence through the *generic*
/// campaign kernel.  Missing reviews stop explicitly at the next demand; this
/// helper does not fabricate review decisions.
pub fn apply_reviewed_mabo_sequence(
    campaign: &mut MaboGenericLegalFollowCampaign,
    assignments: &[MaboIdentityReviewAssignment],
) -> Result<usize, String> {
    let by_representation = assignments
        .iter()
        .cloned()
        .map(|review| (review.representation_ref.clone(), review))
        .collect::<BTreeMap<_, _>>();

    let mut applied = 0usize;
    loop {
        let demand = match campaign.next_demand() {
            Ok(demand) => demand,
            Err(GenericCampaignStop::NoFreshDemand)
            | Err(GenericCampaignStop::BudgetExhausted) => break,
        };
        let Some(assignment) = by_representation.get(&demand.representation_ref).cloned() else {
            break;
        };
        let residual = campaign
            .state()
            .residuals
            .iter()
            .find(|residual| residual.row.residual_ref == demand.residual_ref)
            .ok_or_else(|| "generic Mabo demand lost its residual".to_string())?
            .clone();
        let delta = MaboReviewedIdentityDelta::from_reviewed_assignment(
            &residual,
            assignment,
        )?;
        campaign.accept_reviewed_delta(&delta)?;
        applied += 1;
    }
    Ok(applied)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboGenericCampaignReceipt {
    pub residuals_remaining: usize,
    pub reviewed_identity_count: usize,
    pub review_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[must_use]
pub fn mabo_generic_campaign_receipt(
    campaign: &MaboGenericLegalFollowCampaign,
) -> MaboGenericCampaignReceipt {
    MaboGenericCampaignReceipt {
        residuals_remaining: campaign.state().residuals.len(),
        reviewed_identity_count: campaign
            .state()
            .world
            .reviewed_identity_classes
            .len(),
        review_refs: campaign
            .state()
            .world
            .review_refs
            .values()
            .cloned()
            .collect(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MABO_CONTEXT_IDENTITY_CONSUMER;
    use sensiblaw_consumer_residual::ConsumerSpec;
    use sensiblaw_proof_search_loop::frontier::{ProofResidual, ResidualStatus};
    use sensiblaw_proof_search_loop::world_expansion::ResidualClass;

    fn row(representation: &str, residual: &str) -> MaboIdentityDiagnosisRow {
        MaboIdentityDiagnosisRow {
            representation_ref: representation.into(),
            relation_type_refs: vec!["context:wikidata:P31".into()],
            source_revision_refs: vec![format!("wikidata:{representation}:oldid:123")],
            requirement_id: format!("world-identity:{representation}"),
            residual_ref: residual.into(),
            residual_class: ResidualClass::Identity,
            discovery_route_ref: "residual-observation",
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn diagnosis() -> MaboConsumerDiagnosis {
        let rows = vec![
            row("Q100", "residual:mabo:world-identity:Q100"),
            row("Q200", "residual:mabo:world-identity:Q200"),
        ];
        MaboConsumerDiagnosis {
            consumer_spec: ConsumerSpec {
                consumer_id: MABO_CONTEXT_IDENTITY_CONSUMER.into(),
                surface_id: "surface:mabo:reviewed-context-world".into(),
                requirements: vec![],
            },
            residuals: rows
                .iter()
                .map(|row| ProofResidual {
                    residual_ref: row.residual_ref.clone(),
                    proposition_ref: format!("mabo:world-identity:{}", row.representation_ref),
                    producer_class_ref: "producer:world-expansion".into(),
                    jurisdiction_ref: Some("AU".into()),
                    authority_requirement_ref: None,
                    salience: 100,
                    dependency_refs: vec![],
                    status: ResidualStatus::Open,
                })
                .collect(),
            rows,
            reviewed_context_edges_considered: 2,
            known_identity_representations: 0,
            duplicate_target_edges: 0,
            out_of_scope_or_wrong_type_edges: 0,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn real_mabo_domain_runs_through_same_generic_kernel() {
        let mut campaign = mabo_generic_campaign_from_diagnosis(diagnosis(), 4).unwrap();
        assert_eq!(campaign.next_demand().unwrap().representation_ref, "Q100");

        let reviews = vec![
            MaboIdentityReviewAssignment {
                representation_ref: "Q100".into(),
                identity_class_ref: "world-object:mabo:q100".into(),
                review_ref: "review:mabo:q100".into(),
            },
            MaboIdentityReviewAssignment {
                representation_ref: "Q200".into(),
                identity_class_ref: "world-object:mabo:q200".into(),
                review_ref: "review:mabo:q200".into(),
            },
        ];

        assert_eq!(apply_reviewed_mabo_sequence(&mut campaign, &reviews).unwrap(), 2);
        assert_eq!(
            campaign.next_demand(),
            Err(GenericCampaignStop::NoFreshDemand)
        );
        let receipt = mabo_generic_campaign_receipt(&campaign);
        assert_eq!(receipt.residuals_remaining, 0);
        assert_eq!(receipt.reviewed_identity_count, 2);
        assert_eq!(receipt.review_refs.len(), 2);
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.applicability_promoted);
        assert!(!receipt.claim_truth_promoted);
    }

    #[test]
    fn missing_review_stops_without_fabricating_delta() {
        let mut campaign = mabo_generic_campaign_from_diagnosis(diagnosis(), 4).unwrap();
        let reviews = vec![MaboIdentityReviewAssignment {
            representation_ref: "Q100".into(),
            identity_class_ref: "world-object:mabo:q100".into(),
            review_ref: "review:mabo:q100".into(),
        }];
        assert_eq!(apply_reviewed_mabo_sequence(&mut campaign, &reviews).unwrap(), 1);
        assert_eq!(campaign.next_demand().unwrap().representation_ref, "Q200");
        assert_eq!(campaign.state().accepted_reviewed_deltas, 1);
    }

    #[test]
    fn pending_identity_bundle_is_comment_only_and_cannot_authorize_itself() {
        let demand = MaboLegalFollowDemand {
            representation_ref: "Q200".into(),
            residual_ref: "residual:mabo:world-identity:Q200".into(),
            source_revision_refs: vec!["wikidata:Q1:oldid:2".into()],
            relation_type_refs: vec!["context:wikidata:participant".into()],
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        let bundle = pending_mabo_identity_review_bundle(&demand).unwrap();
        assert!(bundle.lines().all(|line| line.starts_with('#')));
        assert!(bundle.contains("representation_ref=Q200"));
        assert!(bundle.contains("review_manifest_template"));
        assert!(crate::parse_mabo_identity_review_tsv(&bundle)
            .unwrap()
            .is_empty());
    }

}