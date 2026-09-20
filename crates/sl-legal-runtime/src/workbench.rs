//! Source-addressable Sprint-4 M4.A matter + issue workbench.
//!
//! This module is deliberately a read-only projection over canonical reviewed
//! evidence and the legal runtime.  It does not create facts, legal authority,
//! applicability, violation, liability, or remedy.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    InformationAction, LegalCampaignState, MatterWorkspaceProjection, WrongTypeIssueState,
    project_matter_issue_workspace,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatterEntityKind {
    Person,
    Organisation,
    Account,
    Place,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterEntityProjection {
    pub entity_ref: String,
    pub label: String,
    pub kind: MatterEntityKind,
    pub source_revision_refs: Vec<String>,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterObservationProjection {
    pub observation_ref: String,
    pub source_revision_ref: String,
    pub span_ref: String,
    pub element_refs: Vec<String>,
    pub disposition_refs: Vec<String>,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterDocumentProjection {
    pub document_ref: String,
    pub source_revision_ref: String,
    pub span_refs: Vec<String>,
    pub observation_refs: Vec<String>,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterEventProjection {
    pub event_ref: String,
    pub label: String,
    pub time_ref: String,
    pub observation_refs: Vec<String>,
    pub entity_refs: Vec<String>,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatterTimelineKind {
    Observation,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterTimelineEntry {
    pub time_ref: String,
    pub item_ref: String,
    pub kind: MatterTimelineKind,
    pub source_revision_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MatterWorkbenchSeed {
    pub entities: Vec<MatterEntityProjection>,
    pub events: Vec<MatterEventProjection>,
    /// Optional temporal coordinates for canonical observations.
    pub observation_time_refs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterIssueWorkbench {
    pub matter_ref: String,
    pub entities: Vec<MatterEntityProjection>,
    pub observations: Vec<MatterObservationProjection>,
    pub events: Vec<MatterEventProjection>,
    pub documents: Vec<MatterDocumentProjection>,
    pub timeline: Vec<MatterTimelineEntry>,
    pub issue: MatterWorkspaceProjection,
    pub next_action: Option<InformationAction>,
    pub candidate_only: bool,
    pub projection_only: bool,
    pub creates_semantic_authority: bool,
}

impl MatterIssueWorkbench {
    pub fn validate_projection_boundary(&self) -> Result<(), String> {
        if !self.candidate_only || !self.projection_only || self.creates_semantic_authority {
            return Err("M4.A workbench must remain candidate-only, projection-only, and non-authoritative".into());
        }
        if !self.issue.projection_only || self.issue.creates_semantic_authority {
            return Err("embedded issue workspace crossed the projection boundary".into());
        }

        let observation_refs = self
            .observations
            .iter()
            .map(|observation| observation.observation_ref.as_str())
            .collect::<BTreeSet<_>>();
        let entity_refs = self
            .entities
            .iter()
            .map(|entity| entity.entity_ref.as_str())
            .collect::<BTreeSet<_>>();

        for entity in &self.entities {
            if !entity.candidate_only {
                return Err(format!("entity {} is not candidate-only", entity.entity_ref));
            }
        }
        for observation in &self.observations {
            if !observation.candidate_only {
                return Err(format!(
                    "observation {} is not candidate-only",
                    observation.observation_ref
                ));
            }
            if observation.source_revision_ref.trim().is_empty() || observation.span_ref.trim().is_empty() {
                return Err(format!(
                    "observation {} lost exact source identity",
                    observation.observation_ref
                ));
            }
        }

        for event in &self.events {
            if !event.candidate_only {
                return Err(format!("event {} is not candidate-only", event.event_ref));
            }
            if event
                .observation_refs
                .iter()
                .any(|reference| !observation_refs.contains(reference.as_str()))
            {
                return Err(format!("event {} references an unknown observation", event.event_ref));
            }
            if event
                .entity_refs
                .iter()
                .any(|reference| !entity_refs.contains(reference.as_str()))
            {
                return Err(format!("event {} references an unknown entity", event.event_ref));
            }
        }

        for document in &self.documents {
            if !document.candidate_only {
                return Err(format!("document {} is not candidate-only", document.document_ref));
            }
            if document
                .observation_refs
                .iter()
                .any(|reference| !observation_refs.contains(reference.as_str()))
            {
                return Err(format!("document {} references an unknown observation", document.document_ref));
            }
        }
        Ok(())
    }

    pub fn issue_element_count(&self) -> usize {
        self.issue
            .nodes
            .iter()
            .filter(|node| node.semantic_kind.starts_with("legal-element:"))
            .count()
    }
}

/// Compile the complete Priority-5/M4.A read-only workbench.
///
/// Canonical observations and documents are derived from the exact reviewed
/// evidence links already present in the WrongType issue state. Entities,
/// event grouping, and temporal labels are projection metadata supplied by the
/// caller; they cannot pay evidence or legal coordinates.
pub fn project_matter_issue_workbench(
    matter_ref: impl Into<String>,
    issue_state: &WrongTypeIssueState,
    campaign: &LegalCampaignState,
    seed: MatterWorkbenchSeed,
) -> Result<MatterIssueWorkbench, String> {
    let matter_ref = matter_ref.into();
    if matter_ref.trim().is_empty() {
        return Err("matter_ref must not be empty".into());
    }

    let mut by_observation: BTreeMap<String, MatterObservationProjection> = BTreeMap::new();
    for element in &issue_state.elements {
        for evidence in &element.evidence {
            let entry = by_observation
                .entry(evidence.observation_ref.clone())
                .or_insert_with(|| MatterObservationProjection {
                    observation_ref: evidence.observation_ref.clone(),
                    source_revision_ref: evidence.source_revision_ref.clone(),
                    span_ref: evidence.span_ref.clone(),
                    element_refs: Vec::new(),
                    disposition_refs: Vec::new(),
                    candidate_only: true,
                });
            if entry.source_revision_ref != evidence.source_revision_ref
                || entry.span_ref != evidence.span_ref
            {
                return Err(format!(
                    "observation {} was projected with conflicting source identity",
                    evidence.observation_ref
                ));
            }
            if !entry.element_refs.contains(&evidence.element_ref) {
                entry.element_refs.push(evidence.element_ref.clone());
            }
            let disposition = format!("{:?}", evidence.disposition);
            if !entry.disposition_refs.contains(&disposition) {
                entry.disposition_refs.push(disposition);
            }
        }
    }
    let observations = by_observation.into_values().collect::<Vec<_>>();

    let mut documents_by_revision: BTreeMap<String, MatterDocumentProjection> = BTreeMap::new();
    for observation in &observations {
        let document = documents_by_revision
            .entry(observation.source_revision_ref.clone())
            .or_insert_with(|| MatterDocumentProjection {
                document_ref: format!("document:{}", observation.source_revision_ref),
                source_revision_ref: observation.source_revision_ref.clone(),
                span_refs: Vec::new(),
                observation_refs: Vec::new(),
                candidate_only: true,
            });
        if !document.span_refs.contains(&observation.span_ref) {
            document.span_refs.push(observation.span_ref.clone());
        }
        if !document.observation_refs.contains(&observation.observation_ref) {
            document.observation_refs.push(observation.observation_ref.clone());
        }
    }
    let documents = documents_by_revision.into_values().collect::<Vec<_>>();

    let observation_lookup = observations
        .iter()
        .map(|observation| {
            (
                observation.observation_ref.as_str(),
                observation.source_revision_ref.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut timeline = Vec::new();
    for observation in &observations {
        if let Some(time_ref) = seed.observation_time_refs.get(&observation.observation_ref) {
            timeline.push(MatterTimelineEntry {
                time_ref: time_ref.clone(),
                item_ref: observation.observation_ref.clone(),
                kind: MatterTimelineKind::Observation,
                source_revision_refs: vec![observation.source_revision_ref.clone()],
            });
        }
    }
    for event in &seed.events {
        let source_revision_refs = event
            .observation_refs
            .iter()
            .filter_map(|reference| observation_lookup.get(reference.as_str()).copied())
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        timeline.push(MatterTimelineEntry {
            time_ref: event.time_ref.clone(),
            item_ref: event.event_ref.clone(),
            kind: MatterTimelineKind::Event,
            source_revision_refs,
        });
    }
    timeline.sort_by(|left, right| {
        left.time_ref
            .cmp(&right.time_ref)
            .then(left.item_ref.cmp(&right.item_ref))
    });

    let issue = project_matter_issue_workspace(&matter_ref, issue_state, campaign);
    let workbench = MatterIssueWorkbench {
        matter_ref,
        entities: seed.entities,
        observations,
        events: seed.events,
        documents,
        timeline,
        next_action: campaign.selected_action.clone(),
        issue,
        candidate_only: true,
        projection_only: true,
        creates_semantic_authority: false,
    };
    workbench.validate_projection_boundary()?;
    Ok(workbench)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AustralianCalibrationKind, build_australian_calibration_capstone};

    #[test]
    fn priority5_workbench_contains_all_m4_a_surfaces_and_exact_sources() {
        let capstone =
            build_australian_calibration_capstone(AustralianCalibrationKind::CullenNswCla)
                .unwrap();
        let last = capstone.campaign.hops.last().unwrap();

        let observation_ref = capstone.issue.elements[0].evidence[0].observation_ref.clone();
        let seed = MatterWorkbenchSeed {
            entities: vec![MatterEntityProjection {
                entity_ref: "entity:cullen:party-a".into(),
                label: "Party A".into(),
                kind: MatterEntityKind::Person,
                source_revision_refs: vec![
                    capstone.issue.elements[0].evidence[0].source_revision_ref.clone(),
                ],
                candidate_only: true,
            }],
            events: vec![MatterEventProjection {
                event_ref: "event:cullen:reviewed".into(),
                label: "Reviewed matter event".into(),
                time_ref: "2026-09-20T00:00:00+10:00".into(),
                observation_refs: vec![observation_ref.clone()],
                entity_refs: vec!["entity:cullen:party-a".into()],
                candidate_only: true,
            }],
            observation_time_refs: BTreeMap::from([(
                observation_ref,
                "2026-09-20T00:00:00+10:00".into(),
            )]),
        };

        let workbench =
            project_matter_issue_workbench("matter:cullen", &capstone.issue, last, seed).unwrap();

        assert!(!workbench.entities.is_empty());
        assert!(!workbench.observations.is_empty());
        assert!(!workbench.events.is_empty());
        assert!(!workbench.documents.is_empty());
        assert!(!workbench.timeline.is_empty());
        assert!(workbench.issue_element_count() > 0);
        assert!(workbench.projection_only);
        assert!(!workbench.creates_semantic_authority);

        for observation in &workbench.observations {
            assert!(!observation.source_revision_ref.is_empty());
            assert!(!observation.span_ref.is_empty());
        }
    }

    #[test]
    fn workbench_rejects_event_that_invents_an_observation() {
        let capstone =
            build_australian_calibration_capstone(AustralianCalibrationKind::Mabo).unwrap();
        let last = capstone.campaign.hops.last().unwrap();
        let seed = MatterWorkbenchSeed {
            events: vec![MatterEventProjection {
                event_ref: "event:bad".into(),
                label: "bad".into(),
                time_ref: "2026-09-20".into(),
                observation_refs: vec!["observation:not-present".into()],
                entity_refs: Vec::new(),
                candidate_only: true,
            }],
            ..MatterWorkbenchSeed::default()
        };
        assert!(
            project_matter_issue_workbench("matter:mabo", &capstone.issue, last, seed).is_err()
        );
    }
}
