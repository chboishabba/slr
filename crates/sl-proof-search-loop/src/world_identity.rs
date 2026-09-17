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
