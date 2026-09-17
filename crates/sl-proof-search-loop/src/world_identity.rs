use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldObjectIdentity {
    pub identity_class_ref: String,
    pub primary_representation_ref: String,
    pub representation_refs: BTreeSet<String>,
}

impl WorldObjectIdentity {
    #[must_use]
    pub fn new(identity_class_ref: impl Into<String>, primary_representation_ref: impl Into<String>) -> Self {
        let primary_representation_ref = primary_representation_ref.into();
        let mut representation_refs = BTreeSet::new();
        representation_refs.insert(primary_representation_ref.clone());
        Self {
            identity_class_ref: identity_class_ref.into(),
            primary_representation_ref,
            representation_refs,
        }
    }

    #[must_use]
    pub fn with_alias(mut self, representation_ref: impl Into<String>) -> Self {
        self.representation_refs.insert(representation_ref.into());
        self
    }

    #[must_use]
    pub fn contains_representation(&self, representation_ref: &str) -> bool {
        self.representation_refs.contains(representation_ref)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldIdentityResolutionKind {
    ExactSameRepresentation,
    SameObjectDifferentRepresentation,
    RelatedObject,
    Ambiguous,
    WrongType,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldIdentityResolutionReceipt {
    pub receipt_ref: String,
    pub identity: WorldObjectIdentity,
    pub resolution_kind: WorldIdentityResolutionKind,
    pub evidence_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl WorldIdentityResolutionReceipt {
    #[must_use]
    pub fn same_object(
        receipt_ref: impl Into<String>,
        identity: WorldObjectIdentity,
        evidence_ref: impl Into<String>,
    ) -> Self {
        Self {
            receipt_ref: receipt_ref.into(),
            identity,
            resolution_kind: WorldIdentityResolutionKind::SameObjectDifferentRepresentation,
            evidence_ref: evidence_ref.into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[must_use]
    pub fn identity_class_ref(&self) -> &str {
        &self.identity.identity_class_ref
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qid_and_article_aliases_resolve_to_one_world_identity_class() {
        let identity = WorldObjectIdentity::new("world-object:eddie-mabo", "Q975866")
            .with_alias("https://en.wikipedia.org/wiki/Eddie_Mabo");
        assert!(identity.contains_representation("Q975866"));
        assert!(identity.contains_representation("https://en.wikipedia.org/wiki/Eddie_Mabo"));
        assert_eq!(identity.identity_class_ref, "world-object:eddie-mabo");
    }

    #[test]
    fn representation_strings_do_not_define_world_identity() {
        let first = WorldObjectIdentity::new("world-object:eddie-mabo", "Q975866")
            .with_alias("https://en.wikipedia.org/wiki/Eddie_Mabo");
        let second = WorldObjectIdentity::new(
            "world-object:eddie-mabo",
            "https://en.wikipedia.org/wiki/Eddie_Mabo",
        );
        assert_eq!(first.identity_class_ref, second.identity_class_ref);
        assert_ne!(first.primary_representation_ref, second.primary_representation_ref);
    }

    #[test]
    fn identity_receipt_is_candidate_only_and_non_promoting() {
        let receipt = WorldIdentityResolutionReceipt::same_object(
            "identity-resolution:mabo:eddie",
            WorldObjectIdentity::new("world-object:eddie-mabo", "Q975866")
                .with_alias("https://en.wikipedia.org/wiki/Eddie_Mabo"),
            "qid-sitelink-review:mabo:eddie",
        );
        assert!(receipt.candidate_only);
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.applicability_promoted);
        assert!(!receipt.claim_truth_promoted);
    }
}
