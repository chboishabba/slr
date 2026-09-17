#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qid_and_wikipedia_representation_can_share_one_world_identity() {
        let qid = RepresentationIdentity::new("Q975866", RepresentationKind::Qid);
        let article = RepresentationIdentity::new(
            "https://en.wikipedia.org/wiki/Eddie_Mabo",
            RepresentationKind::WikipediaArticle,
        );
        let receipt = SameObjectReceipt::reviewed(
            "world:mabo:person:eddie-mabo",
            qid.clone(),
            article.clone(),
            "review:same-object:eddie-mabo",
        );

        let mut registry = WorldIdentityRegistry::default();
        assert_eq!(
            registry.resolve(&qid, None).unwrap(),
            WorldIdentityResolution::NewIdentityClass("world:mabo:person:eddie-mabo".into())
        );
        assert_eq!(
            registry.resolve(&article, Some(&receipt)).unwrap(),
            WorldIdentityResolution::ExistingIdentityClass("world:mabo:person:eddie-mabo".into())
        );
        assert_eq!(registry.identity_class_count(), 1);
    }

    #[test]
    fn alias_without_reviewed_same_object_receipt_fails_closed() {
        let qid = RepresentationIdentity::new("Q975866", RepresentationKind::Qid);
        let article = RepresentationIdentity::new(
            "https://en.wikipedia.org/wiki/Eddie_Mabo",
            RepresentationKind::WikipediaArticle,
        );
        let mut registry = WorldIdentityRegistry::default();
        registry
            .register_canonical("world:mabo:person:eddie-mabo", &qid)
            .unwrap();

        assert_eq!(
            registry.resolve(&article, None),
            Err(WorldIdentityError::UnresolvedRepresentation(
                "https://en.wikipedia.org/wiki/Eddie_Mabo".into()
            ))
        );
    }

    #[test]
    fn same_object_receipt_must_name_registered_and_candidate_representations() {
        let qid = RepresentationIdentity::new("Q975866", RepresentationKind::Qid);
        let article = RepresentationIdentity::new(
            "https://en.wikipedia.org/wiki/Eddie_Mabo",
            RepresentationKind::WikipediaArticle,
        );
        let wrong = RepresentationIdentity::new("Q1", RepresentationKind::Qid);
        let receipt = SameObjectReceipt::reviewed(
            "world:mabo:person:eddie-mabo",
            wrong,
            article.clone(),
            "review:wrong",
        );
        let mut registry = WorldIdentityRegistry::default();
        registry
            .register_canonical("world:mabo:person:eddie-mabo", &qid)
            .unwrap();

        assert_eq!(
            registry.resolve(&article, Some(&receipt)),
            Err(WorldIdentityError::SameObjectReceiptMismatch)
        );
    }
}
