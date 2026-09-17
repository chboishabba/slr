use crate::world_expansion_reentry::DiscoveryLineageReceipt;
use crate::world_identity::WorldIdentityResolutionReceipt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityAwareDiscoveryLineageReceipt {
    pub identity_class_ref: String,
    pub representation_ref: String,
    pub identity_resolution_ref: String,
    pub lineage: DiscoveryLineageReceipt,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityLineageError {
    RepresentationNotInIdentityClass,
    IdentityResolutionNotCandidateOnly,
    IdentityResolutionMayNotPromote,
}

pub fn bind_lineage_to_identity(
    lineage: DiscoveryLineageReceipt,
    identity: &WorldIdentityResolutionReceipt,
) -> Result<IdentityAwareDiscoveryLineageReceipt, IdentityLineageError> {
    if !identity.candidate_only {
        return Err(IdentityLineageError::IdentityResolutionNotCandidateOnly);
    }
    if identity.creates_semantic_authority
        || identity.applicability_promoted
        || identity.claim_truth_promoted
    {
        return Err(IdentityLineageError::IdentityResolutionMayNotPromote);
    }
    if !identity.identity.contains_representation(&lineage.object_ref) {
        return Err(IdentityLineageError::RepresentationNotInIdentityClass);
    }
    Ok(IdentityAwareDiscoveryLineageReceipt {
        identity_class_ref: identity.identity.identity_class_ref.clone(),
        representation_ref: lineage.object_ref.clone(),
        identity_resolution_ref: identity.receipt_ref.clone(),
        lineage,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_expansion::ProducerLane;
    use crate::world_identity::{WorldObjectIdentity, WorldIdentityResolutionReceipt};

    fn lineage() -> DiscoveryLineageReceipt {
        DiscoveryLineageReceipt {
            object_ref: "https://en.wikipedia.org/wiki/Eddie_Mabo".into(),
            discovery_parent_ref: "Q1501525".into(),
            triggering_residual_ref: "residual:mabo:participant-identity".into(),
            selected_candidate_ref: "wikipedia:eddie-mabo".into(),
            producer_lane: ProducerLane::WikipediaContext,
            source_revision_ref: "etag:eddie".into(),
            pnf_world_disambiguation_ref: "pnf-world:mabo:eddie".into(),
            expected_residual_contraction: 2,
            observed_residual_contraction: 1,
            new_residual_refs: vec![],
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            receipt_authority: "candidate_world_expansion_only",
        }
    }

    #[test]
    fn lineage_retains_representation_and_world_identity_class_separately() {
        let identity = WorldIdentityResolutionReceipt::same_object(
            "identity-resolution:mabo:eddie",
            WorldObjectIdentity::new("world-object:eddie-mabo", "Q975866")
                .with_alias("https://en.wikipedia.org/wiki/Eddie_Mabo"),
            "reviewed-qid-sitelink",
        );
        let bound = bind_lineage_to_identity(lineage(), &identity).unwrap();
        assert_eq!(bound.identity_class_ref, "world-object:eddie-mabo");
        assert_eq!(bound.representation_ref, "https://en.wikipedia.org/wiki/Eddie_Mabo");
        assert_ne!(bound.identity_class_ref, bound.representation_ref);
        assert!(!bound.creates_semantic_authority);
        assert!(!bound.claim_truth_promoted);
    }
}
