//! Sprint 7 projection fabric.
//! Projection is selection and presentation over canonical explanation identities.

use std::collections::{BTreeMap, BTreeSet};
use sha2::{Digest, Sha256};
use crate::{ExplanationIndex, MatterIssueWorkbench};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectionKind {
    SourceView, Timeline, IssueProof, EntityRelationship, CitationAuthority, Flow, Comparative,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectionContext {
    pub temporal_refs: BTreeMap<String, String>,
    pub jurisdiction_refs: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionQuery {
    pub kind: ProjectionKind,
    pub semantic_selection: BTreeSet<String>,
    pub as_at: Option<String>,
    pub jurisdiction_slice: BTreeSet<String>,
}

impl ProjectionQuery {
    pub fn new(kind: ProjectionKind) -> Self {
        Self { kind, semantic_selection: BTreeSet::new(), as_at: None, jurisdiction_slice: BTreeSet::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionNode {
    pub semantic_ref: String,
    pub semantic_kind: String,
    pub manifestation_refs: Vec<String>,
    pub source_revision_refs: Vec<String>,
    pub span_refs: Vec<String>,
    pub projection_role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionEdge {
    pub from_ref: String,
    pub to_ref: String,
    pub relation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionGraph {
    pub kind: ProjectionKind,
    pub nodes: Vec<ProjectionNode>,
    pub edges: Vec<ProjectionEdge>,
    pub deterministic_digest: String,
    pub projection_only: bool,
    pub creates_semantic_authority: bool,
}

fn digest(parts: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut h = Sha256::new();
    for part in parts {
        let b = part.as_ref().as_bytes();
        h.update((b.len() as u64).to_le_bytes());
        h.update(b);
    }
    format!("sha256:{:x}", h.finalize())
}

fn closure(index: &ExplanationIndex, selected: &BTreeSet<String>) -> BTreeSet<String> {
    if selected.is_empty() { return index.records.keys().cloned().collect(); }
    let mut keep = selected.clone();
    let mut frontier = selected.iter().cloned().collect::<Vec<_>>();
    while let Some(current) = frontier.pop() {
        if let Some(record) = index.records.get(&current) {
            for next in record.dependencies.iter().chain(record.reverse_dependencies.iter()) {
                if keep.insert(next.clone()) { frontier.push(next.clone()); }
            }
        }
    }
    keep
}

fn visible(kind: ProjectionKind, semantic_kind: crate::ExplainableKind) -> bool {
    use crate::ExplainableKind::*;
    use ProjectionKind::*;
    match kind {
        SourceView => matches!(semantic_kind, SourceRevision | Span | ReviewedEvidence | Observation | Document | LegalElement | LegalIssue),
        Timeline => matches!(semantic_kind, TimelineEntry | Event | Observation | SourceRevision | Span),
        IssueProof => matches!(semantic_kind, LegalIssue | LegalElement | ReviewedEvidence | Observation | Residual | Action | SourceRevision | Span),
        EntityRelationship => matches!(semantic_kind, Entity | Event | Observation | Document | SourceRevision | Span),
        CitationAuthority => matches!(semantic_kind, LegalIssue | LegalElement | ReviewedEvidence | Observation | SourceRevision | Span | Document),
        Flow | Comparative => true,
    }
}

pub fn compile_projection(
    workbench: &MatterIssueWorkbench,
    index: &ExplanationIndex,
    query: &ProjectionQuery,
    context: &ProjectionContext,
) -> Result<ProjectionGraph, String> {
    workbench.validate_projection_boundary()?;
    index.validate()?;

    let reachable = closure(index, &query.semantic_selection);
    let mut node_map = BTreeMap::new();

    for semantic_ref in reachable {
        let Some(record) = index.records.get(&semantic_ref) else { continue; };
        if !visible(query.kind, record.kind) && !query.semantic_selection.contains(&semantic_ref) { continue; }

        if let Some(as_at) = &query.as_at {
            if let Some(time_ref) = context.temporal_refs.get(&semantic_ref) {
                if time_ref > as_at { continue; }
            }
        }
        if !query.jurisdiction_slice.is_empty() {
            if let Some(jurisdictions) = context.jurisdiction_refs.get(&semantic_ref) {
                if jurisdictions.is_disjoint(&query.jurisdiction_slice) { continue; }
            }
        }

        let manifestation_refs = record.provenance.iter()
            .filter_map(|p| p.manifestation_ref.clone()).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
        let source_revision_refs = record.provenance.iter()
            .map(|p| p.source_revision_ref.clone()).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
        let span_refs = record.provenance.iter()
            .filter_map(|p| p.span_ref.clone()).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();

        node_map.insert(semantic_ref.clone(), ProjectionNode {
            semantic_ref,
            semantic_kind: format!("{:?}", record.kind),
            manifestation_refs,
            source_revision_refs,
            span_refs,
            projection_role: format!("{:?}", query.kind),
        });
    }

    let mut edges = Vec::new();
    for (semantic_ref, record) in &index.records {
        if !node_map.contains_key(semantic_ref) { continue; }
        for dependency in &record.dependencies {
            if node_map.contains_key(dependency) {
                edges.push(ProjectionEdge {
                    from_ref: dependency.clone(), to_ref: semantic_ref.clone(), relation: "depends-on".into(),
                });
            }
        }
    }
    edges.sort_by(|a,b| (&a.from_ref,&a.to_ref,&a.relation).cmp(&(&b.from_ref,&b.to_ref,&b.relation)));

    let mut nodes = node_map.into_values().collect::<Vec<_>>();
    nodes.sort_by(|a,b| a.semantic_ref.cmp(&b.semantic_ref));

    let parts = std::iter::once(format!("{:?}", query.kind))
        .chain(nodes.iter().flat_map(|n| [
            n.semantic_ref.clone(), n.semantic_kind.clone(), n.manifestation_refs.join(","),
            n.source_revision_refs.join(","), n.span_refs.join(",")
        ]))
        .chain(edges.iter().flat_map(|e| [e.from_ref.clone(), e.to_ref.clone(), e.relation.clone()]))
        .collect::<Vec<_>>();

    let graph = ProjectionGraph {
        kind: query.kind,
        deterministic_digest: digest(parts.iter().map(String::as_str)),
        nodes,
        edges,
        projection_only: true,
        creates_semantic_authority: false,
    };
    graph.validate(index)?;
    Ok(graph)
}

impl ProjectionGraph {
    pub fn contains(&self, semantic_ref: &str) -> bool {
        self.nodes.iter().any(|node| node.semantic_ref == semantic_ref)
    }

    pub fn validate(&self, index: &ExplanationIndex) -> Result<(), String> {
        if !self.projection_only || self.creates_semantic_authority {
            return Err("projection graph crossed semantic authority boundary".into());
        }
        for node in &self.nodes {
            let canonical = index.records.get(&node.semantic_ref)
                .ok_or_else(|| format!("projection invented semantic id {}", node.semantic_ref))?;
            let canonical_manifestations = canonical.provenance.iter()
                .filter_map(|p| p.manifestation_ref.as_deref()).collect::<BTreeSet<_>>();
            let canonical_revisions = canonical.provenance.iter()
                .map(|p| p.source_revision_ref.as_str()).collect::<BTreeSet<_>>();
            if node.manifestation_refs.iter().any(|r| !canonical_manifestations.contains(r.as_str())) {
                return Err(format!("projection rewrote manifestation for {}", node.semantic_ref));
            }
            if node.source_revision_refs.iter().any(|r| !canonical_revisions.contains(r.as_str())) {
                return Err(format!("projection rewrote provenance for {}", node.semantic_ref));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build_australian_calibration_capstone, compile_explanation_index, project_matter_issue_workbench,
        AustralianCalibrationKind, MatterEventProjection, MatterWorkbenchSeed,
    };

    #[test]
    fn same_canonical_identity_survives_all_projection_families() {
        let capstone = build_australian_calibration_capstone(AustralianCalibrationKind::CullenNswCla).unwrap();
        let last = capstone.campaign.hops.last().unwrap();
        let anchor = capstone.issue.elements[0].evidence[0].observation_ref.clone();
        let time = "2026-09-20T00:00:00+10:00".to_string();
        let workbench = project_matter_issue_workbench(
            "matter:cullen", &capstone.issue, last,
            MatterWorkbenchSeed {
                events: vec![MatterEventProjection {
                    event_ref: "event:cullen:fixture".into(), label: "fixture".into(), time_ref: time.clone(),
                    observation_refs: vec![anchor.clone()], entity_refs: Vec::new(), candidate_only: true,
                }],
                observation_time_refs: BTreeMap::from([(anchor.clone(), time.clone())]),
                ..MatterWorkbenchSeed::default()
            },
        ).unwrap();
        let index = compile_explanation_index(&workbench, &capstone.issue, last).unwrap();
        let context = ProjectionContext { temporal_refs: BTreeMap::from([(anchor.clone(), time)]), ..ProjectionContext::default() };

        for kind in [
            ProjectionKind::SourceView, ProjectionKind::Timeline, ProjectionKind::IssueProof,
            ProjectionKind::EntityRelationship, ProjectionKind::CitationAuthority,
            ProjectionKind::Flow, ProjectionKind::Comparative,
        ] {
            let mut query = ProjectionQuery::new(kind);
            query.semantic_selection.insert(anchor.clone());
            let graph = compile_projection(&workbench, &index, &query, &context).unwrap();
            assert!(graph.contains(&anchor), "{kind:?} lost canonical anchor");
            let projected = graph.nodes.iter().find(|node| node.semantic_ref == anchor).unwrap();
            let canonical = index.get(&anchor).unwrap();
            assert_eq!(projected.source_revision_refs[0], canonical.provenance[0].source_revision_ref);
            assert!(!graph.creates_semantic_authority);
        }
    }

    #[test]
    fn as_at_filter_does_not_mutate_canonical_state() {
        let capstone = build_australian_calibration_capstone(AustralianCalibrationKind::Mabo).unwrap();
        let last = capstone.campaign.hops.last().unwrap();
        let anchor = capstone.issue.elements[0].evidence[0].observation_ref.clone();
        let workbench = project_matter_issue_workbench("matter:mabo", &capstone.issue, last, MatterWorkbenchSeed::default()).unwrap();
        let index = compile_explanation_index(&workbench, &capstone.issue, last).unwrap();
        let context = ProjectionContext {
            temporal_refs: BTreeMap::from([(anchor.clone(), "2026-10-01".into())]),
            ..ProjectionContext::default()
        };
        let mut query = ProjectionQuery::new(ProjectionKind::Timeline);
        query.semantic_selection.insert(anchor.clone());
        query.as_at = Some("2026-09-20".into());
        let graph = compile_projection(&workbench, &index, &query, &context).unwrap();
        assert!(!graph.contains(&anchor));
        assert!(index.get(&anchor).is_some());
    }
}
