use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepresentationKind {
    Qid,
    WikipediaArticle,
    LegalSource,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepresentationIdentity {
    pub representation_ref: String,
    pub kind: RepresentationKind,
}

impl RepresentationIdentity {
    #[must_use]
    pub fn new(reference: impl Into<String>, kind: RepresentationKind) -> Self {
        Self {
            representation_ref: reference.into(),
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SameObjectReceipt {
    pub canonical_world_object_ref: String,
    pub established_representation: RepresentationIdentity,
    pub candidate_representation: RepresentationIdentity,
    pub review_ref: String,
    pub reviewed: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl SameObjectReceipt {
    #[must_use]
    pub fn reviewed(
        canonical_world_object_ref: impl Into<String>,
        established_representation: RepresentationIdentity,
        candidate_representation: RepresentationIdentity,
        review_ref: impl Into<String>,
    ) -> Self {
        Self {
            canonical_world_object_ref: canonical_world_object_ref.into(),
            established_representation,
            candidate_representation,
            review_ref: review_ref.into(),
            reviewed: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldIdentityResolution {
    NewIdentityClass(String),
    ExistingIdentityClass(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldIdentityError {
    EmptyCoordinate(&'static str),
    CanonicalClassUnknown(String),
    RepresentationAlreadyBound(String),
    SameObjectReceiptRequired,
    SameObjectReceiptMismatch,
    SameObjectReceiptMayNotPromote,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldIdentityRegistry {
    representation_to_class: BTreeMap<RepresentationIdentity, String>,
    identity_classes: BTreeSet<String>,
}

impl WorldIdentityRegistry {
    pub fn register_canonical(
        &mut self,
        canonical_world_object_ref: impl Into<String>,
        representation: &RepresentationIdentity,
    ) -> Result<(), WorldIdentityError> {
        let canonical_world_object_ref = canonical_world_object_ref.into();
        if canonical_world_object_ref.trim().is_empty() {
            return Err(WorldIdentityError::EmptyCoordinate("canonical_world_object_ref"));
        }
        if representation.representation_ref.trim().is_empty() {
            return Err(WorldIdentityError::EmptyCoordinate("representation_ref"));
        }
        if let Some(existing) = self.representation_to_class.get(representation) {
            if existing != &canonical_world_object_ref {
                return Err(WorldIdentityError::RepresentationAlreadyBound(
                    representation.representation_ref.clone(),
                ));
            }
            return Ok(());
        }
        self.identity_classes.insert(canonical_world_object_ref.clone());
        self.representation_to_class
            .insert(representation.clone(), canonical_world_object_ref);
        Ok(())
    }

    pub fn attach_to_class(
        &mut self,
        canonical_world_object_ref: &str,
        representation: &RepresentationIdentity,
        receipt: Option<&SameObjectReceipt>,
    ) -> Result<WorldIdentityResolution, WorldIdentityError> {
        if !self.identity_classes.contains(canonical_world_object_ref) {
            return Err(WorldIdentityError::CanonicalClassUnknown(
                canonical_world_object_ref.to_owned(),
            ));
        }
        if let Some(existing) = self.representation_to_class.get(representation) {
            return Ok(WorldIdentityResolution::ExistingIdentityClass(existing.clone()));
        }
        let receipt = receipt.ok_or(WorldIdentityError::SameObjectReceiptRequired)?;
        if !receipt.reviewed || receipt.creates_semantic_authority || receipt.creates_claim_truth {
            return Err(WorldIdentityError::SameObjectReceiptMayNotPromote);
        }
        let established_matches = self
            .representation_to_class
            .get(&receipt.established_representation)
            .is_some_and(|class_ref| class_ref == canonical_world_object_ref);
        let candidate_matches = receipt.candidate_representation == *representation;
        if receipt.canonical_world_object_ref != canonical_world_object_ref
            || !established_matches
            || !candidate_matches
        {
            return Err(WorldIdentityError::SameObjectReceiptMismatch);
        }
        self.representation_to_class.insert(
            representation.clone(),
            canonical_world_object_ref.to_owned(),
        );
        Ok(WorldIdentityResolution::ExistingIdentityClass(
            canonical_world_object_ref.to_owned(),
        ))
    }

    pub fn resolve(
        &mut self,
        representation: &RepresentationIdentity,
        receipt: Option<&SameObjectReceipt>,
    ) -> Result<WorldIdentityResolution, WorldIdentityError> {
        if let Some(existing) = self.representation_to_class.get(representation) {
            return Ok(WorldIdentityResolution::ExistingIdentityClass(existing.clone()));
        }
        if let Some(receipt) = receipt {
            return self.attach_to_class(
                &receipt.canonical_world_object_ref,
                representation,
                Some(receipt),
            );
        }
        let canonical = representation.representation_ref.clone();
        self.register_canonical(canonical.clone(), representation)?;
        Ok(WorldIdentityResolution::NewIdentityClass(canonical))
    }

    #[must_use]
    pub fn identity_class_count(&self) -> usize {
        self.identity_classes.len()
    }
}

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
        registry
            .register_canonical("world:mabo:person:eddie-mabo", &qid)
            .unwrap();
        assert_eq!(
            registry.resolve(&article, Some(&receipt)).unwrap(),
            WorldIdentityResolution::ExistingIdentityClass("world:mabo:person:eddie-mabo".into())
        );
        assert_eq!(registry.identity_class_count(), 1);
    }

    #[test]
    fn unseen_representation_may_open_new_identity_class_without_guessing_aliases() {
        let qid = RepresentationIdentity::new("Q1358798", RepresentationKind::Qid);
        let mut registry = WorldIdentityRegistry::default();
        assert_eq!(
            registry.resolve(&qid, None).unwrap(),
            WorldIdentityResolution::NewIdentityClass("Q1358798".into())
        );
        assert_eq!(registry.identity_class_count(), 1);
    }

    #[test]
    fn alias_without_reviewed_same_object_receipt_fails_closed_when_canonical_class_is_claimed() {
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
            registry.attach_to_class("world:mabo:person:eddie-mabo", &article, None),
            Err(WorldIdentityError::SameObjectReceiptRequired)
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
            registry.attach_to_class(
                "world:mabo:person:eddie-mabo",
                &article,
                Some(&receipt),
            ),
            Err(WorldIdentityError::SameObjectReceiptMismatch)
        );
    }
}
