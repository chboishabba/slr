//! M13 minimal recipient-scoped handoff.
//!
//! This is a bounded projection/export contract over MatterContext. It is not a
//! report generator and it never mutates or deletes canonical matter state.

use std::collections::BTreeSet;

use crate::matter_context::{
    ContextProjectionExclusion, DisclosureBoundary, MatterContextProjection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandoffRecipientProfile {
    Lawyer,
    Clinician,
    Advocate,
    Regulator,
    PublicOfficial,
    Researcher,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimalHandoffSelection {
    pub handoff_ref: String,
    pub matter_ref: String,
    pub recipient_ref: String,
    pub recipient_profile: HandoffRecipientProfile,
    pub disclosure_boundary: DisclosureBoundary,
    pub selected_refs: Vec<String>,
    pub redaction_refs: Vec<String>,
    pub retention_policy_ref: String,
    pub redaction_policy_ref: String,
    pub text_export_policy_ref: String,
    pub local_only: bool,
    pub do_not_sync: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimalHandoffPreview {
    pub handoff_ref: String,
    pub matter_ref: String,
    pub recipient_ref: String,
    pub recipient_profile: HandoffRecipientProfile,
    pub disclosure_boundary: DisclosureBoundary,
    pub exported_refs: Vec<String>,
    pub redacted_refs: Vec<String>,
    pub visible_exclusions: Vec<ContextProjectionExclusion>,
    pub retention_policy_ref: String,
    pub redaction_policy_ref: String,
    pub text_export_policy_ref: String,
    pub local_only: bool,
    pub do_not_sync: bool,
    pub canonical_world_mutated: bool,
    pub redaction_deletes_canonical_source: bool,
    pub unshared_means_absent_from_world: bool,
    pub export_creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinimalHandoffError {
    EmptyCoordinate(&'static str),
    MatterMismatch,
    SelectedRefNotVisible(String),
    RedactionRefNotSelected(String),
    DuplicateSelectedRef(String),
    DuplicateRedactionRef(String),
    ProtectedDisclosureRequiresNoSync,
}

impl MinimalHandoffSelection {
    pub fn validate(&self) -> Result<(), MinimalHandoffError> {
        for (name, value) in [
            ("handoff_ref", self.handoff_ref.as_str()),
            ("matter_ref", self.matter_ref.as_str()),
            ("recipient_ref", self.recipient_ref.as_str()),
            ("retention_policy_ref", self.retention_policy_ref.as_str()),
            ("redaction_policy_ref", self.redaction_policy_ref.as_str()),
            ("text_export_policy_ref", self.text_export_policy_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(MinimalHandoffError::EmptyCoordinate(name));
            }
        }
        if self
            .selected_refs
            .iter()
            .chain(self.redaction_refs.iter())
            .any(|value| value.trim().is_empty())
        {
            return Err(MinimalHandoffError::EmptyCoordinate("handoff_refs"));
        }
        if self.disclosure_boundary == DisclosureBoundary::ProtectedDisclosure
            && !self.do_not_sync
        {
            return Err(MinimalHandoffError::ProtectedDisclosureRequiresNoSync);
        }
        Ok(())
    }
}

pub fn preview_minimal_handoff(
    selection: &MinimalHandoffSelection,
    context: &MatterContextProjection,
) -> Result<MinimalHandoffPreview, MinimalHandoffError> {
    selection.validate()?;
    if selection.matter_ref != context.matter_ref {
        return Err(MinimalHandoffError::MatterMismatch);
    }

    let visible = context
        .included_refs
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    let mut selected_seen = BTreeSet::new();
    for reference in &selection.selected_refs {
        if !selected_seen.insert(reference.as_str()) {
            return Err(MinimalHandoffError::DuplicateSelectedRef(
                reference.clone(),
            ));
        }
        if !visible.contains(reference.as_str()) {
            return Err(MinimalHandoffError::SelectedRefNotVisible(
                reference.clone(),
            ));
        }
    }

    let selected = selection
        .selected_refs
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut redaction_seen = BTreeSet::new();
    for reference in &selection.redaction_refs {
        if !redaction_seen.insert(reference.as_str()) {
            return Err(MinimalHandoffError::DuplicateRedactionRef(
                reference.clone(),
            ));
        }
        if !selected.contains(reference.as_str()) {
            return Err(MinimalHandoffError::RedactionRefNotSelected(
                reference.clone(),
            ));
        }
    }

    let redacted = selection
        .redaction_refs
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut exported_refs = selection
        .selected_refs
        .iter()
        .filter(|reference| !redacted.contains(reference.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    exported_refs.sort();

    let mut redacted_refs = selection.redaction_refs.clone();
    redacted_refs.sort();

    Ok(MinimalHandoffPreview {
        handoff_ref: selection.handoff_ref.clone(),
        matter_ref: selection.matter_ref.clone(),
        recipient_ref: selection.recipient_ref.clone(),
        recipient_profile: selection.recipient_profile,
        disclosure_boundary: selection.disclosure_boundary,
        exported_refs,
        redacted_refs,
        visible_exclusions: context.exclusions.clone(),
        retention_policy_ref: selection.retention_policy_ref.clone(),
        redaction_policy_ref: selection.redaction_policy_ref.clone(),
        text_export_policy_ref: selection.text_export_policy_ref.clone(),
        local_only: selection.local_only,
        do_not_sync: selection.do_not_sync,
        canonical_world_mutated: false,
        redaction_deletes_canonical_source: false,
        unshared_means_absent_from_world: false,
        export_creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matter_context::MatterContextProjection;

    fn context() -> MatterContextProjection {
        MatterContextProjection {
            matter_ref: "matter:1".into(),
            included_refs: vec!["claim:a".into(), "source:a".into()],
            exclusions: vec![],
            canonical_world_mutated: false,
            invisibility_means_false: false,
            unshared_means_absent: false,
            role_visibility_creates_truth: false,
            later_knowledge_rewrites_cut: false,
        }
    }

    fn selection() -> MinimalHandoffSelection {
        MinimalHandoffSelection {
            handoff_ref: "handoff:1".into(),
            matter_ref: "matter:1".into(),
            recipient_ref: "recipient:lawyer".into(),
            recipient_profile: HandoffRecipientProfile::Lawyer,
            disclosure_boundary: DisclosureBoundary::RecipientScoped,
            selected_refs: vec!["claim:a".into(), "source:a".into()],
            redaction_refs: vec!["source:a".into()],
            retention_policy_ref: "retention:bounded".into(),
            redaction_policy_ref: "redaction:explicit".into(),
            text_export_policy_ref: "text-export:reviewed-only".into(),
            local_only: true,
            do_not_sync: true,
        }
    }

    #[test]
    fn preview_shows_exact_export_and_redaction_without_world_mutation() {
        let preview = preview_minimal_handoff(&selection(), &context()).unwrap();
        assert_eq!(preview.exported_refs, vec!["claim:a"]);
        assert_eq!(preview.redacted_refs, vec!["source:a"]);
        assert!(!preview.canonical_world_mutated);
        assert!(!preview.redaction_deletes_canonical_source);
        assert!(!preview.unshared_means_absent_from_world);
        assert!(!preview.export_creates_semantic_authority);
        assert!(!preview.claim_truth_promoted);
    }

    #[test]
    fn handoff_cannot_export_ref_hidden_by_context() {
        let mut value = selection();
        value.selected_refs.push("claim:hidden".into());
        assert_eq!(
            preview_minimal_handoff(&value, &context()),
            Err(MinimalHandoffError::SelectedRefNotVisible(
                "claim:hidden".into()
            ))
        );
    }

    #[test]
    fn redaction_must_be_selected_before_it_can_be_hidden() {
        let mut value = selection();
        value.redaction_refs = vec!["claim:other".into()];
        assert_eq!(
            preview_minimal_handoff(&value, &context()),
            Err(MinimalHandoffError::RedactionRefNotSelected(
                "claim:other".into()
            ))
        );
    }

    #[test]
    fn protected_disclosure_defaults_to_no_sync_boundary() {
        let mut value = selection();
        value.disclosure_boundary = DisclosureBoundary::ProtectedDisclosure;
        value.do_not_sync = false;
        assert_eq!(
            value.validate(),
            Err(MinimalHandoffError::ProtectedDisclosureRequiresNoSync)
        );
    }
}
