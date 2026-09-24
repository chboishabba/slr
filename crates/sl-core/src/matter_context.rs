//! S30.C MatterContext projection contract.
//!
//! MatterContext is not another semantic world. It controls which coordinates
//! from the already-reviewed world may be rendered/used/disclosed for one
//! purpose, consumer role, and knowledge-time cut.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatterConsumerRole {
    ProtectedSubject,
    SupportOperator,
    Clinician,
    Advocate,
    Lawyer,
    Journalist,
    PublicOfficial,
    PublicAudience,
    Regulator,
    Researcher,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatterPurpose {
    PersonalReview,
    CarePlanning,
    LegalAdvocacy,
    AffidavitPreparation,
    LegalResearch,
    JournalisticVerification,
    PublicAdministrationReview,
    RegulatoryReview,
    ResearchPublication,
    HandoffPreparation,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisclosureBoundary {
    MatterInternal,
    RoleScoped,
    RecipientScoped,
    MetadataOnly,
    PublicProjection,
    ProtectedDisclosure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeCutMembership {
    KnownAtCut,
    KnownAfterCut,
    UnknownAtCut,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterContext {
    pub matter_ref: String,
    pub purpose_ref: MatterPurpose,
    pub active_consumer_role: MatterConsumerRole,
    pub disclosure_boundary: DisclosureBoundary,
    pub knowledge_time_cut_ref: Option<String>,
    pub sealed_refs: Vec<String>,
    pub minimum_necessary: bool,
    pub purpose_limited: bool,
    pub access_logged: bool,
    pub revocable: bool,
    pub mutates_canonical_world: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextProjectionCoordinate {
    pub semantic_ref: String,
    pub matter_ref: String,
    pub allowed_roles: Vec<MatterConsumerRole>,
    pub allowed_purposes: Vec<MatterPurpose>,
    pub knowledge_membership: KnowledgeCutMembership,
    pub explicitly_selected: bool,
    pub sealed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextExclusionReason {
    WrongMatter,
    Sealed,
    RoleNotAllowed,
    PurposeNotAllowed,
    KnownAfterCut,
    UnknownAtCut,
    NotSelectedMinimumNecessary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextProjectionExclusion {
    pub semantic_ref: String,
    pub reason: ContextExclusionReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterContextProjection {
    pub matter_ref: String,
    pub included_refs: Vec<String>,
    pub exclusions: Vec<ContextProjectionExclusion>,
    pub canonical_world_mutated: bool,
    pub invisibility_means_false: bool,
    pub unshared_means_absent: bool,
    pub role_visibility_creates_truth: bool,
    pub later_knowledge_rewrites_cut: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatterContextError {
    EmptyMatterRef,
    EmptyKnowledgeCutRef,
    EmptySemanticRef,
    EmptyCoordinateMatterRef,
    DuplicateSemanticRef(String),
    NonMinimumNecessaryContext,
    NonPurposeLimitedContext,
    UnloggedContext,
    NonRevocableContext,
    CanonicalMutationNotAllowed,
    SemanticPromotionNotAllowed,
}

impl MatterContext {
    pub fn validate(&self) -> Result<(), MatterContextError> {
        if self.matter_ref.trim().is_empty() {
            return Err(MatterContextError::EmptyMatterRef);
        }
        if self
            .knowledge_time_cut_ref
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(MatterContextError::EmptyKnowledgeCutRef);
        }
        if !self.minimum_necessary {
            return Err(MatterContextError::NonMinimumNecessaryContext);
        }
        if !self.purpose_limited {
            return Err(MatterContextError::NonPurposeLimitedContext);
        }
        if !self.access_logged {
            return Err(MatterContextError::UnloggedContext);
        }
        if !self.revocable {
            return Err(MatterContextError::NonRevocableContext);
        }
        if self.mutates_canonical_world {
            return Err(MatterContextError::CanonicalMutationNotAllowed);
        }
        if self.creates_semantic_authority || self.claim_truth_promoted {
            return Err(MatterContextError::SemanticPromotionNotAllowed);
        }
        Ok(())
    }
}

impl ContextProjectionCoordinate {
    pub fn validate(&self) -> Result<(), MatterContextError> {
        if self.semantic_ref.trim().is_empty() {
            return Err(MatterContextError::EmptySemanticRef);
        }
        if self.matter_ref.trim().is_empty() {
            return Err(MatterContextError::EmptyCoordinateMatterRef);
        }
        Ok(())
    }
}

pub fn project_matter_context(
    context: &MatterContext,
    coordinates: &[ContextProjectionCoordinate],
) -> Result<MatterContextProjection, MatterContextError> {
    context.validate()?;

    let sealed_refs = context
        .sealed_refs
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let mut seen = std::collections::BTreeSet::new();
    let mut included_refs = Vec::new();
    let mut exclusions = Vec::new();

    for coordinate in coordinates {
        coordinate.validate()?;
        if !seen.insert(coordinate.semantic_ref.clone()) {
            return Err(MatterContextError::DuplicateSemanticRef(
                coordinate.semantic_ref.clone(),
            ));
        }

        let reason = if coordinate.matter_ref != context.matter_ref {
            Some(ContextExclusionReason::WrongMatter)
        } else if coordinate.sealed || sealed_refs.contains(coordinate.semantic_ref.as_str()) {
            Some(ContextExclusionReason::Sealed)
        } else if !coordinate.allowed_roles.is_empty()
            && !coordinate
                .allowed_roles
                .contains(&context.active_consumer_role)
        {
            Some(ContextExclusionReason::RoleNotAllowed)
        } else if !coordinate.allowed_purposes.is_empty()
            && !coordinate.allowed_purposes.contains(&context.purpose_ref)
        {
            Some(ContextExclusionReason::PurposeNotAllowed)
        } else if context.knowledge_time_cut_ref.is_some()
            && coordinate.knowledge_membership == KnowledgeCutMembership::KnownAfterCut
        {
            Some(ContextExclusionReason::KnownAfterCut)
        } else if context.knowledge_time_cut_ref.is_some()
            && coordinate.knowledge_membership == KnowledgeCutMembership::UnknownAtCut
        {
            Some(ContextExclusionReason::UnknownAtCut)
        } else if context.minimum_necessary && !coordinate.explicitly_selected {
            Some(ContextExclusionReason::NotSelectedMinimumNecessary)
        } else {
            None
        };

        if let Some(reason) = reason {
            exclusions.push(ContextProjectionExclusion {
                semantic_ref: coordinate.semantic_ref.clone(),
                reason,
            });
        } else {
            included_refs.push(coordinate.semantic_ref.clone());
        }
    }

    included_refs.sort();
    exclusions.sort_by(|left, right| {
        left.semantic_ref
            .cmp(&right.semantic_ref)
            .then_with(|| left.reason.cmp(&right.reason))
    });

    Ok(MatterContextProjection {
        matter_ref: context.matter_ref.clone(),
        included_refs,
        exclusions,
        canonical_world_mutated: false,
        invisibility_means_false: false,
        unshared_means_absent: false,
        role_visibility_creates_truth: false,
        later_knowledge_rewrites_cut: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> MatterContext {
        MatterContext {
            matter_ref: "matter:1".into(),
            purpose_ref: MatterPurpose::LegalAdvocacy,
            active_consumer_role: MatterConsumerRole::Lawyer,
            disclosure_boundary: DisclosureBoundary::RecipientScoped,
            knowledge_time_cut_ref: Some("knowledge-cut:2026-09-01".into()),
            sealed_refs: vec!["claim:sealed".into()],
            minimum_necessary: true,
            purpose_limited: true,
            access_logged: true,
            revocable: true,
            mutates_canonical_world: false,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        }
    }

    fn coordinate(reference: &str) -> ContextProjectionCoordinate {
        ContextProjectionCoordinate {
            semantic_ref: reference.into(),
            matter_ref: "matter:1".into(),
            allowed_roles: vec![MatterConsumerRole::Lawyer],
            allowed_purposes: vec![MatterPurpose::LegalAdvocacy],
            knowledge_membership: KnowledgeCutMembership::KnownAtCut,
            explicitly_selected: true,
            sealed: false,
        }
    }

    #[test]
    fn later_knowledge_is_excluded_without_rewriting_world() {
        let mut later = coordinate("claim:later");
        later.knowledge_membership = KnowledgeCutMembership::KnownAfterCut;

        let projection = project_matter_context(&context(), &[later]).unwrap();
        assert!(projection.included_refs.is_empty());
        assert_eq!(
            projection.exclusions[0].reason,
            ContextExclusionReason::KnownAfterCut
        );
        assert!(!projection.canonical_world_mutated);
        assert!(!projection.later_knowledge_rewrites_cut);
    }

    #[test]
    fn sealed_ref_is_not_rendered_and_does_not_become_false() {
        let sealed = coordinate("claim:sealed");
        let projection = project_matter_context(&context(), &[sealed]).unwrap();
        assert!(projection.included_refs.is_empty());
        assert_eq!(
            projection.exclusions[0].reason,
            ContextExclusionReason::Sealed
        );
        assert!(!projection.invisibility_means_false);
    }

    #[test]
    fn minimum_necessary_requires_explicit_selection() {
        let mut item = coordinate("claim:not-selected");
        item.explicitly_selected = false;
        let projection = project_matter_context(&context(), &[item]).unwrap();
        assert_eq!(
            projection.exclusions[0].reason,
            ContextExclusionReason::NotSelectedMinimumNecessary
        );
        assert!(!projection.unshared_means_absent);
    }

    #[test]
    fn role_visibility_does_not_create_truth() {
        let projection =
            project_matter_context(&context(), &[coordinate("claim:visible")]).unwrap();
        assert_eq!(projection.included_refs, vec!["claim:visible"]);
        assert!(!projection.role_visibility_creates_truth);
    }
}
