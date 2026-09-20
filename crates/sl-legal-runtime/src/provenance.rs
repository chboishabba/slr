//! Sprint 6 universal provenance and explanation.
//! This is an index over existing canonical identities, never a second authority store.

use std::collections::{BTreeMap, BTreeSet};

use crate::{InformationActionKind, LegalCampaignState, LegalProjectionState, MatterIssueWorkbench, WrongTypeIssueState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExplainableKind {
    SourceRevision, Span, ReviewedEvidence, Observation, Entity, Event, Document,
    TimelineEntry, LegalElement, LegalIssue, Requirement, Residual, Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExplanationClass { SourceBacked, ProjectionMetadata, SystemMetadata }

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProvenanceAddress {
    pub source_revision_ref: String,
    pub span_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionLineage {
    pub current_revision_ref: String,
    pub previous_revision_ref: Option<String>,
    pub affected_semantic_refs: Vec<String>,
    pub stale_projection_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedRevisionTransition {
    pub manifestation_ref: Option<String>,
    pub previous_revision_ref: String,
    pub current_revision_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualExplanation {
    pub residual_ref: String,
    pub missing_requirement_ref: String,
    pub evidence_type_sought: String,
    pub route: Option<InformationActionKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainableRef {
    pub semantic_ref: String,
    pub kind: ExplainableKind,
    pub class: ExplanationClass,
    pub provenance: Vec<ProvenanceAddress>,
    pub dependencies: Vec<String>,
    pub reverse_dependencies: Vec<String>,
    pub evidence_uses: Vec<String>,
    pub legal_uses: Vec<String>,
    pub residuals: Vec<String>,
    pub revision_lineage: Vec<RevisionLineage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplanationIndex {
    pub records: BTreeMap<String, ExplainableRef>,
    pub residual_explanations: BTreeMap<String, ResidualExplanation>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
}

fn push_unique(values: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if !values.contains(&value) { values.push(value); }
}

fn address(revision: &str, span: Option<&str>) -> ProvenanceAddress {
    ProvenanceAddress { source_revision_ref: revision.to_owned(), span_ref: span.map(ToOwned::to_owned) }
}

fn bare(reference: impl Into<String>, kind: ExplainableKind, class: ExplanationClass) -> ExplainableRef {
    ExplainableRef {
        semantic_ref: reference.into(), kind, class, provenance: Vec::new(),
        dependencies: Vec::new(), reverse_dependencies: Vec::new(),
        evidence_uses: Vec::new(), legal_uses: Vec::new(), residuals: Vec::new(),
        revision_lineage: Vec::new(),
    }
}

fn insert(records: &mut BTreeMap<String, ExplainableRef>, record: ExplainableRef) {
    records.entry(record.semantic_ref.clone()).and_modify(|existing| {
        for value in &record.provenance { if !existing.provenance.contains(value) { existing.provenance.push(value.clone()); } }
        for value in &record.dependencies { push_unique(&mut existing.dependencies, value.clone()); }
        for value in &record.evidence_uses { push_unique(&mut existing.evidence_uses, value.clone()); }
        for value in &record.legal_uses { push_unique(&mut existing.legal_uses, value.clone()); }
        for value in &record.residuals { push_unique(&mut existing.residuals, value.clone()); }
        if existing.class != ExplanationClass::SourceBacked && record.class == ExplanationClass::SourceBacked {
            existing.class = ExplanationClass::SourceBacked;
        }
    }).or_insert(record);
}

fn evidence_type(kind: crate::LegalResidualKind) -> &'static str {
    use crate::LegalResidualKind::*;
    match kind {
        Source => "source", MatterEvidence => "matter-evidence", Element => "element-evidence",
        ExceptionOrDefeater => "exception-or-defeater evidence", FormalRuleDerivation => "formal derivation",
        Burden => "burden evidence", JurisdictionOrTime => "jurisdiction-or-temporal authority",
        Remedy => "remedy evidence", ClosedForConsumer => "none",
    }
}

pub fn compile_explanation_index_from_state(
    workbench: &MatterIssueWorkbench,
    issue: &WrongTypeIssueState,
    state: &LegalProjectionState,
) -> Result<ExplanationIndex, String> {
    workbench.validate_projection_boundary()?;
    let mut records = BTreeMap::new();

    for observation in &workbench.observations {
        let mut r = bare(&observation.observation_ref, ExplainableKind::Observation, ExplanationClass::SourceBacked);
        r.provenance.push(address(&observation.source_revision_ref, Some(&observation.span_ref)));
        r.dependencies.push(observation.span_ref.clone());
        r.legal_uses.extend(observation.element_refs.clone());
        insert(&mut records, r);

        let mut revision = bare(&observation.source_revision_ref, ExplainableKind::SourceRevision, ExplanationClass::SourceBacked);
        revision.provenance.push(address(&observation.source_revision_ref, None));
        insert(&mut records, revision);

        let mut span = bare(&observation.span_ref, ExplainableKind::Span, ExplanationClass::SourceBacked);
        span.provenance.push(address(&observation.source_revision_ref, Some(&observation.span_ref)));
        span.dependencies.push(observation.source_revision_ref.clone());
        insert(&mut records, span);
    }

    for element in &issue.elements {
        let mut er = bare(&element.element.element_ref, ExplainableKind::LegalElement, ExplanationClass::SourceBacked);
        for evidence in &element.evidence {
            er.provenance.push(address(&evidence.source_revision_ref, Some(&evidence.span_ref)));
            push_unique(&mut er.dependencies, evidence.reviewed_evidence_ref.clone());
            push_unique(&mut er.evidence_uses, evidence.observation_ref.clone());

            let mut reviewed = bare(&evidence.reviewed_evidence_ref, ExplainableKind::ReviewedEvidence, ExplanationClass::SourceBacked);
            reviewed.provenance.push(address(&evidence.source_revision_ref, Some(&evidence.span_ref)));
            reviewed.dependencies.push(evidence.observation_ref.clone());
            reviewed.legal_uses.push(element.element.element_ref.clone());
            insert(&mut records, reviewed);
        }
        insert(&mut records, er);
    }

    for node in &workbench.issue.nodes {
        let kind = if node.semantic_kind == "legal-issue" { ExplainableKind::LegalIssue } else { ExplainableKind::LegalElement };
        let class = if node.source_revision_refs.is_empty() || node.span_refs.is_empty() {
            ExplanationClass::ProjectionMetadata
        } else { ExplanationClass::SourceBacked };
        let mut r = bare(&node.node_ref, kind, class);
        for revision in &node.source_revision_refs {
            if node.span_refs.is_empty() { r.provenance.push(address(revision, None)); }
            else { for span in &node.span_refs { r.provenance.push(address(revision, Some(span))); } }
        }
        r.dependencies.extend(node.dependency_refs.clone());
        r.legal_uses.extend(node.downstream_refs.clone());
        r.residuals.extend(node.residual_refs.clone());
        insert(&mut records, r);
    }

    for document in &workbench.documents {
        let mut r = bare(&document.document_ref, ExplainableKind::Document, ExplanationClass::SourceBacked);
        for span in &document.span_refs { r.provenance.push(address(&document.source_revision_ref, Some(span))); }
        r.dependencies.extend(document.observation_refs.clone());
        insert(&mut records, r);
    }

    for entity in &workbench.entities {
        let mut r = bare(&entity.entity_ref, ExplainableKind::Entity, ExplanationClass::ProjectionMetadata);
        for revision in &entity.source_revision_refs { r.provenance.push(address(revision, None)); }
        insert(&mut records, r);
    }

    for event in &workbench.events {
        let mut r = bare(&event.event_ref, ExplainableKind::Event, ExplanationClass::ProjectionMetadata);
        r.dependencies.extend(event.observation_refs.clone());
        r.dependencies.extend(event.entity_refs.clone());
        for observation_ref in &event.observation_refs {
            if let Some(observation) = records.get(observation_ref) { r.provenance.extend(observation.provenance.clone()); }
        }
        insert(&mut records, r);
    }

    for entry in &workbench.timeline {
        let mut r = bare(format!("timeline:{}:{}", entry.time_ref, entry.item_ref), ExplainableKind::TimelineEntry, ExplanationClass::ProjectionMetadata);
        r.dependencies.push(entry.item_ref.clone());
        for revision in &entry.source_revision_refs { r.provenance.push(address(revision, None)); }
        insert(&mut records, r);
    }

    let mut residual_explanations = BTreeMap::new();
    for residual in &state.residuals {
        if !records.contains_key(&residual.target_ref) {
            insert(&mut records, bare(
                &residual.target_ref,
                ExplainableKind::Requirement,
                ExplanationClass::SystemMetadata,
            ));
        }
        let route = state.selected_action.as_ref()
            .filter(|a| a.residual_ref == residual.residual_ref).map(|a| a.kind);
        residual_explanations.insert(residual.residual_ref.clone(), ResidualExplanation {
            residual_ref: residual.residual_ref.clone(),
            missing_requirement_ref: residual.target_ref.clone(),
            evidence_type_sought: evidence_type(residual.kind).into(),
            route,
        });
        let mut r = bare(&residual.residual_ref, ExplainableKind::Residual, ExplanationClass::SystemMetadata);
        r.dependencies.push(residual.target_ref.clone());
        for source in &residual.source_refs { r.provenance.push(address(source, None)); }
        insert(&mut records, r);
    }

    if let Some(action) = &state.selected_action {
        let mut r = bare(&action.action_ref, ExplainableKind::Action, ExplanationClass::SystemMetadata);
        r.dependencies.push(action.residual_ref.clone());
        insert(&mut records, r);
    }

    let edges = records.iter().flat_map(|(child, record)| {
        record.dependencies.iter().map(move |parent| (parent.clone(), child.clone()))
    }).collect::<Vec<_>>();
    for (parent, child) in edges {
        if let Some(record) = records.get_mut(&parent) { push_unique(&mut record.reverse_dependencies, child); }
    }

    let revisions = records.values().flat_map(|r| r.provenance.iter().map(|p| p.source_revision_ref.clone())).collect::<BTreeSet<_>>();
    for revision in revisions {
        let affected = records.values()
            .filter(|r| r.provenance.iter().any(|p| p.source_revision_ref == revision))
            .map(|r| r.semantic_ref.clone()).collect::<Vec<_>>();
        let stale_projection_refs = affected.iter()
            .filter(|semantic_ref| {
                records.get(*semantic_ref)
                    .map(|record| record.class != ExplanationClass::SourceBacked)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        if let Some(record) = records.get_mut(&revision) {
            record.revision_lineage.push(RevisionLineage {
                current_revision_ref: revision.clone(),
                previous_revision_ref: None,
                affected_semantic_refs: affected,
                stale_projection_refs,
            });
        }
    }

    let index = ExplanationIndex { records, residual_explanations, candidate_only: true, creates_semantic_authority: false };
    index.validate()?;
    Ok(index)
}

pub fn compile_explanation_index(
    workbench: &MatterIssueWorkbench,
    issue: &WrongTypeIssueState,
    campaign: &LegalCampaignState,
) -> Result<ExplanationIndex, String> {
    let state = LegalProjectionState::from(campaign);
    compile_explanation_index_from_state(workbench, issue, &state)
}

impl ExplanationIndex {
    pub fn apply_verified_revision_transition(
        &mut self,
        transition: &VerifiedRevisionTransition,
    ) -> Result<RevisionLineage, String> {
        if transition.previous_revision_ref.trim().is_empty()
            || transition.current_revision_ref.trim().is_empty()
            || transition.previous_revision_ref == transition.current_revision_ref
        {
            return Err("revision transition must name distinct non-empty revisions".into());
        }

        let current = self.records.get(&transition.current_revision_ref).ok_or_else(|| {
            format!(
                "current revision {} is not present in the explanation index",
                transition.current_revision_ref
            )
        })?;
        if current.kind != ExplainableKind::SourceRevision {
            return Err(format!(
                "{} is not a source revision",
                transition.current_revision_ref
            ));
        }
        if let Some(expected_manifestation) = transition.manifestation_ref.as_deref() {
            let matches_manifestation = current.provenance.iter().any(|provenance| {
                provenance.manifestation_ref.as_deref() == Some(expected_manifestation)
            });
            if !matches_manifestation {
                return Err(format!(
                    "revision {} does not belong to manifestation {}",
                    transition.current_revision_ref, expected_manifestation
                ));
            }
        }

        let affected_semantic_refs = self.records.values()
            .filter(|record| record.provenance.iter().any(|provenance| {
                provenance.source_revision_ref == transition.current_revision_ref
                    || provenance.source_revision_ref == transition.previous_revision_ref
            }))
            .map(|record| record.semantic_ref.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        let stale_projection_refs = affected_semantic_refs.iter()
            .filter(|semantic_ref| {
                self.records.get(*semantic_ref)
                    .map(|record| record.class != ExplanationClass::SourceBacked)
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<Vec<_>>();

        let lineage = RevisionLineage {
            current_revision_ref: transition.current_revision_ref.clone(),
            previous_revision_ref: Some(transition.previous_revision_ref.clone()),
            affected_semantic_refs,
            stale_projection_refs,
        };
        self.records
            .get_mut(&transition.current_revision_ref)
            .expect("current revision checked above")
            .revision_lineage
            .push(lineage.clone());
        Ok(lineage)
    }

    pub fn get(&self, semantic_ref: &str) -> Option<&ExplainableRef> { self.records.get(semantic_ref) }

    pub fn material_consequences(&self, semantic_ref: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut frontier = vec![semantic_ref.to_owned()];
        while let Some(current) = frontier.pop() {
            if let Some(record) = self.records.get(&current) {
                for child in &record.reverse_dependencies {
                    if seen.insert(child.clone()) { frontier.push(child.clone()); }
                }
            }
        }
        seen
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.candidate_only || self.creates_semantic_authority {
            return Err("explanation index crossed semantic authority boundary".into());
        }
        for record in self.records.values() {
            if record.semantic_ref.trim().is_empty() { return Err("anonymous semantic object".into()); }
            if record.class == ExplanationClass::SourceBacked && record.provenance.is_empty() {
                return Err(format!("source-backed object {} has no provenance path", record.semantic_ref));
            }
            for dependency in &record.dependencies {
                if !self.records.contains_key(dependency) {
                    return Err(format!("{} depends on unexplained object {}", record.semantic_ref, dependency));
                }
            }
        }
        for residual in self.residual_explanations.values() {
            if residual.missing_requirement_ref.trim().is_empty() || residual.evidence_type_sought.trim().is_empty() {
                return Err(format!("residual {} is not explainable", residual.residual_ref));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_australian_calibration_capstone, project_matter_issue_workbench, AustralianCalibrationKind, MatterEventProjection, MatterWorkbenchSeed};

    #[test]
    fn visible_semantics_have_source_or_explicit_projection_class_and_reverse_impact() {
        let capstone = build_australian_calibration_capstone(AustralianCalibrationKind::Mabo).unwrap();
        let last = capstone.campaign.hops.last().unwrap();
        let observation_ref = capstone.issue.elements[0].evidence[0].observation_ref.clone();
        let workbench = project_matter_issue_workbench(
            "matter:mabo", &capstone.issue, last,
            MatterWorkbenchSeed {
                events: vec![MatterEventProjection {
                    event_ref: "event:mabo:fixture".into(), label: "fixture".into(),
                    time_ref: "2026-09-20T00:00:00+10:00".into(),
                    observation_refs: vec![observation_ref.clone()], entity_refs: Vec::new(), candidate_only: true,
                }],
                observation_time_refs: BTreeMap::from([(observation_ref.clone(), "2026-09-20T00:00:00+10:00".into())]),
                ..MatterWorkbenchSeed::default()
            },
        ).unwrap();
        let index = compile_explanation_index(&workbench, &capstone.issue, last).unwrap();
        let observation = index.get(&observation_ref).unwrap();
        assert_eq!(observation.class, ExplanationClass::SourceBacked);
        assert!(!observation.provenance.is_empty());
        assert!(index.material_consequences(&observation_ref).contains("event:mabo:fixture"));
        assert!(!index.creates_semantic_authority);
    }

    #[test]
    fn verified_revision_transition_reports_affected_and_stale_projections() {
        let capstone = build_australian_calibration_capstone(AustralianCalibrationKind::Mabo).unwrap();
        let last = capstone.campaign.hops.last().unwrap();
        let evidence = &capstone.issue.elements[0].evidence[0];
        let observation_ref = evidence.observation_ref.clone();
        let workbench = project_matter_issue_workbench(
            "matter:mabo",
            &capstone.issue,
            last,
            MatterWorkbenchSeed {
                events: vec![MatterEventProjection {
                    event_ref: "event:mabo:revision-sensitive".into(),
                    label: "revision-sensitive projection".into(),
                    time_ref: "2026-09-20T00:00:00+10:00".into(),
                    observation_refs: vec![observation_ref],
                    entity_refs: Vec::new(),
                    candidate_only: true,
                }],
                ..MatterWorkbenchSeed::default()
            },
        ).unwrap();
        let mut index = compile_explanation_index(&workbench, &capstone.issue, last).unwrap();
        let transition = VerifiedRevisionTransition {
            manifestation_ref: evidence.manifestation_ref.clone(),
            previous_revision_ref: "revision:verified-previous".into(),
            current_revision_ref: evidence.source_revision_ref.clone(),
        };
        let lineage = index.apply_verified_revision_transition(&transition).unwrap();
        assert_eq!(lineage.previous_revision_ref.as_deref(), Some("revision:verified-previous"));
        assert!(lineage.affected_semantic_refs.contains(&evidence.observation_ref));
        assert!(lineage.stale_projection_refs.contains(&"event:mabo:revision-sensitive".to_string()));
    }
}
