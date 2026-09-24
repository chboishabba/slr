//! Typed Mabo query/world snapshot for S15/S16/S18 automation.
//!
//! This is an adapter over the existing Mabo consumer and the generic
//! query-world controller.  It does not define new legal semantics.
//!
//! The snapshot deliberately distinguishes:
//! - reviewed context observations (source/revision coordinates),
//! - durable reviewed identity classes,
//! - currently unresolved identity representations,
//! - operational campaign closure, and
//! - theorem-bearing consumer adequacy.
//!
//! Seeing a QID in reviewed context does not itself pay SameObject.  Only a
//! representation already present in the durable identity baseline is emitted
//! as a visible semantic node.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_legal_runtime::{
    ConsumerAxis, ConsumerCoverage, ConsumerQueryDemand, LegalWorldCoordinate,
    OperationalResearchState, ProjectionGraph, ProjectionKind, ProjectionNode,
    QueryDependencySlice, QueryWorldRunDecision, RevisionDependencyIndex,
    KernelCheckedFactorsThroughWitness, KernelCheckedNonFactorabilityWitness,
    decide_query_world_run,
};
use sensiblaw_pg_source_store::{
    ContextRevisionWorldSlice, DiscoveryIdentityBaseline, LatentWorldRows,
};

use crate::{
    diagnose_mabo_context_world_identity,
    mabo_revision_campaign::MaboRevisionProbeReceipt,
    parse_mabo_wikidata_source_revision_ref, MaboConsumerDiagnosis,
    MABO_CONTEXT_IDENTITY_CONSUMER,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaboRevisionProbeSnapshot {
    pub reviewed_source_count: usize,
    pub probed_source_count: usize,
    pub revision_reopen_residual_refs: Vec<String>,
    pub unchanged_source_refs: Vec<String>,
    pub unprobed_source_refs: Vec<String>,
    pub blocker_source_refs: Vec<String>,
    pub probe_truncated: bool,
    pub probe_complete: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl MaboRevisionProbeSnapshot {
    #[must_use]
    pub fn from_receipt(receipt: &MaboRevisionProbeReceipt) -> Self {
        let mut revision_reopen_residual_refs = receipt
            .reopen_residuals
            .iter()
            .map(|residual| residual.residual_ref.clone())
            .collect::<Vec<_>>();
        revision_reopen_residual_refs.sort();
        revision_reopen_residual_refs.dedup();

        let mut unchanged_source_refs = receipt.unchanged_source_refs.clone();
        unchanged_source_refs.sort();
        unchanged_source_refs.dedup();

        let mut unprobed_source_refs = receipt.unprobed_source_refs.clone();
        unprobed_source_refs.sort();
        unprobed_source_refs.dedup();

        let mut blocker_source_refs = receipt
            .blockers
            .iter()
            .map(|blocker| blocker.source_ref.clone())
            .collect::<Vec<_>>();
        blocker_source_refs.sort();
        blocker_source_refs.dedup();

        Self {
            reviewed_source_count: receipt.reviewed_source_count,
            probed_source_count: receipt.probed_source_count,
            revision_reopen_residual_refs,
            unchanged_source_refs,
            unprobed_source_refs,
            blocker_source_refs,
            probe_truncated: receipt.probe_truncated,
            probe_complete: receipt.probe_complete,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaboQueryWorldSnapshot {
    pub schema_version: String,
    pub consumer_ref: String,
    pub seed_ref: String,
    pub world: LegalWorldCoordinate,
    pub context_revision_slice: BTreeMap<String, String>,
    pub query_demand: ConsumerQueryDemand,
    pub coverage: ConsumerCoverage,
    pub operational_state: OperationalResearchState,
    pub dependencies: RevisionDependencyIndex,
    pub query_dependency_slice: QueryDependencySlice,
    pub projection: ProjectionGraph,
    pub revision_probe: MaboRevisionProbeSnapshot,
    pub diagnosed_identity_residual_refs: Vec<String>,
    pub known_identity_representations: usize,
    pub reviewed_context_edges_considered: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn digest_parts(parts: impl IntoIterator<Item = String>) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        let bytes = part.as_bytes();
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn reviewed_context_revision(provenance_refs: &[String]) -> Option<String> {
    provenance_refs.iter().find_map(|provenance| {
        provenance
            .strip_prefix("context:wikidata:")
            .map(ToOwned::to_owned)
    })
}

#[derive(Debug, Clone, Default)]
struct TargetObservation {
    source_refs: BTreeSet<String>,
    source_revision_refs: BTreeSet<String>,
}

fn observed_targets(
    world: &LatentWorldRows,
) -> Result<BTreeMap<String, TargetObservation>, String> {
    let mut targets: BTreeMap<String, TargetObservation> = BTreeMap::new();
    for edge in &world.edges {
        if !edge.relation_ref.starts_with("context:wikidata:") {
            continue;
        }
        let Some(revision_ref) = reviewed_context_revision(&edge.provenance_refs) else {
            continue;
        };
        let parsed = parse_mabo_wikidata_source_revision_ref(&revision_ref)
            .map_err(|error| format!("parse Mabo reviewed source revision: {error}"))?;
        if parsed.qid != edge.from_ref {
            return Err(format!(
                "reviewed context edge source {} does not match pinned revision {}",
                edge.from_ref, revision_ref
            ));
        }
        let observation = targets.entry(edge.to_ref.clone()).or_default();
        observation.source_refs.insert(edge.from_ref.clone());
        observation.source_revision_refs.insert(revision_ref);
    }
    Ok(targets)
}

fn projection_digest(
    query_ref: &str,
    nodes: &[ProjectionNode],
    required_refs: &BTreeSet<String>,
    source_refs: &BTreeSet<String>,
    source_revision_refs: &BTreeSet<String>,
) -> String {
    let mut parts = vec![
        "mabo-query-world-snapshot:v1".to_owned(),
        query_ref.to_owned(),
    ];
    parts.extend(required_refs.iter().map(|reference| format!("required:{reference}")));
    parts.extend(source_refs.iter().map(|reference| format!("source:{reference}")));
    parts.extend(
        source_revision_refs
            .iter()
            .map(|reference| format!("revision:{reference}")),
    );
    parts.extend(nodes.iter().flat_map(|node| {
        [
            format!("node:{}", node.semantic_ref),
            format!("identity-class:{}", node.manifestation_refs.join(",")),
            format!("source-revisions:{}", node.source_revision_refs.join(",")),
        ]
    }));
    digest_parts(parts)
}

fn world_ref(
    seed_ref: &str,
    as_at: &str,
    context_slice: &ContextRevisionWorldSlice,
) -> String {
    let mut parts = vec![
        "mabo-legal-world:v1".to_owned(),
        seed_ref.to_owned(),
        as_at.to_owned(),
    ];
    parts.extend(
        context_slice
            .wikidata_source_revisions
            .iter()
            .map(|(source, revision)| format!("{source}={revision}")),
    );
    let digest = digest_parts(parts);
    format!(
        "world:mabo:{}:{}",
        seed_ref,
        digest.trim_start_matches("sha256:")
    )
}

fn operational_state_for_snapshot(
    diagnosis: &MaboConsumerDiagnosis,
    probe: &MaboRevisionProbeReceipt,
) -> OperationalResearchState {
    if probe.probe_truncated {
        OperationalResearchState::BudgetExhausted
    } else if !probe.probe_complete
        || !probe.blockers.is_empty()
        || !probe.reopen_residuals.is_empty()
        || !diagnosis.rows.is_empty()
    {
        OperationalResearchState::Open
    } else {
        OperationalResearchState::CurrentFrontierClosed
    }
}

#[allow(clippy::too_many_arguments)]
pub fn compile_mabo_query_world_snapshot(
    seed_ref: &str,
    as_at: &str,
    world: &LatentWorldRows,
    baseline: &DiscoveryIdentityBaseline,
    context_slice: &ContextRevisionWorldSlice,
    probe: &MaboRevisionProbeReceipt,
) -> Result<MaboQueryWorldSnapshot, String> {
    if seed_ref.trim().is_empty() || as_at.trim().is_empty() {
        return Err("Mabo query/world snapshot requires seed_ref and explicit as_at".into());
    }
    context_slice
        .validate()
        .map_err(|error| format!("validate Mabo context revision slice: {error}"))?;

    if world.creates_semantic_authority
        || world.applicability_promoted
        || world.claim_truth_promoted
        || !probe.candidate_only
        || probe.creates_semantic_authority
        || probe.applicability_promoted
        || probe.claim_truth_promoted
    {
        return Err("Mabo query/world snapshot crossed non-promotion boundary".into());
    }

    let diagnosis = diagnose_mabo_context_world_identity(world, baseline);
    if diagnosis.creates_semantic_authority
        || diagnosis.applicability_promoted
        || diagnosis.claim_truth_promoted
    {
        return Err("Mabo diagnosis crossed non-promotion boundary".into());
    }

    let targets = observed_targets(world)?;
    let required_semantic_refs = targets.keys().cloned().collect::<BTreeSet<_>>();
    let source_refs = targets
        .values()
        .flat_map(|observation| observation.source_refs.iter().cloned())
        .collect::<BTreeSet<_>>();
    let source_revision_refs = targets
        .values()
        .flat_map(|observation| observation.source_revision_refs.iter().cloned())
        .collect::<BTreeSet<_>>();

    let required_axes = BTreeSet::from([ConsumerAxis::SemanticIdentity]);
    let query_ref = MABO_CONTEXT_IDENTITY_CONSUMER.to_owned();

    let mut nodes = required_semantic_refs
        .iter()
        .filter_map(|representation_ref| {
            let identity_class_ref = baseline
                .representation_identity_class_refs
                .get(representation_ref)?;
            let observation = targets.get(representation_ref)?;
            Some(ProjectionNode {
                semantic_ref: representation_ref.clone(),
                semantic_kind: "ReviewedWorldIdentity".into(),
                manifestation_refs: vec![identity_class_ref.clone()],
                source_revision_refs: observation
                    .source_revision_refs
                    .iter()
                    .cloned()
                    .collect(),
                // SameObject identity review is a durable identity coordinate;
                // context RDF does not provide a legal-text span coordinate.
                span_refs: Vec::new(),
                projection_role: "MaboContextIdentity".into(),
            })
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.semantic_ref.cmp(&right.semantic_ref));

    let projection = ProjectionGraph {
        kind: ProjectionKind::IssueProof,
        edges: Vec::new(),
        deterministic_digest: projection_digest(
            &query_ref,
            &nodes,
            &required_semantic_refs,
            &source_refs,
            &source_revision_refs,
        ),
        nodes,
        projection_only: true,
        creates_semantic_authority: false,
    };

    let mut source_to_propositions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (target_ref, observation) in &targets {
        for source_ref in &observation.source_refs {
            source_to_propositions
                .entry(source_ref.clone())
                .or_default()
                .insert(target_ref.clone());
        }
    }
    let dependencies = RevisionDependencyIndex {
        source_to_propositions,
        proposition_dependents: BTreeMap::new(),
    };

    let query_demand = ConsumerQueryDemand {
        query_ref: query_ref.clone(),
        required_axes: required_axes.clone(),
        required_semantic_refs: required_semantic_refs.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
    };
    let coverage = ConsumerCoverage::default();
    let query_dependency_slice = QueryDependencySlice {
        query_ref: query_ref.clone(),
        required_axes,
        semantic_refs: required_semantic_refs,
        proof_refs: BTreeSet::new(),
        source_refs,
        source_revision_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    query_dependency_slice.validate()?;

    let legal_world = LegalWorldCoordinate {
        world_ref: world_ref(seed_ref, as_at, context_slice),
        matter_ref: format!("matter:mabo:{seed_ref}"),
        jurisdiction_ref: "AU".into(),
        as_at: as_at.to_owned(),
        source_revisions: context_slice.wikidata_source_revisions.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    legal_world.validate()?;

    let mut diagnosed_identity_residual_refs = diagnosis
        .rows
        .iter()
        .map(|row| row.residual_ref.clone())
        .collect::<Vec<_>>();
    diagnosed_identity_residual_refs.sort();
    diagnosed_identity_residual_refs.dedup();

    let snapshot = MaboQueryWorldSnapshot {
        schema_version: "sl.mabo_query_world_snapshot.v0_1".into(),
        consumer_ref: query_ref,
        seed_ref: seed_ref.to_owned(),
        world: legal_world,
        context_revision_slice: context_slice.wikidata_source_revisions.clone(),
        query_demand,
        coverage,
        operational_state: operational_state_for_snapshot(&diagnosis, probe),
        dependencies,
        query_dependency_slice,
        projection,
        revision_probe: MaboRevisionProbeSnapshot::from_receipt(probe),
        diagnosed_identity_residual_refs,
        known_identity_representations: diagnosis.known_identity_representations,
        reviewed_context_edges_considered: diagnosis.reviewed_context_edges_considered,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    snapshot.validate()?;
    Ok(snapshot)
}

impl MaboQueryWorldSnapshot {
    pub fn decide_current_world(
        &self,
        formal_adequacy: Option<&KernelCheckedFactorsThroughWitness>,
        nonfactorability_witnesses: &[KernelCheckedNonFactorabilityWitness],
    ) -> Result<QueryWorldRunDecision, String> {
        self.validate()?;
        decide_query_world_run(
            &self.world,
            &self.world,
            &self.dependencies,
            &self.query_dependency_slice,
            &self.query_demand,
            &self.projection,
            &self.coverage,
            self.operational_state,
            formal_adequacy,
            nonfactorability_witnesses,
        )
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != "sl.mabo_query_world_snapshot.v0_1"
            || self.consumer_ref != MABO_CONTEXT_IDENTITY_CONSUMER
            || self.query_demand.query_ref != self.consumer_ref
            || self.query_dependency_slice.query_ref != self.consumer_ref
            || self.seed_ref.trim().is_empty()
            || !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
            || !self.projection.projection_only
            || self.projection.creates_semantic_authority
        {
            return Err("invalid Mabo query/world snapshot envelope".into());
        }
        self.world.validate()?;
        self.query_dependency_slice.validate()?;

        if self.world.source_revisions != self.context_revision_slice {
            return Err("Mabo world/source revision slice drifted from snapshot".into());
        }
        if self.query_demand.required_axes
            != self.query_dependency_slice.required_axes
            || self.query_demand.required_semantic_refs
                != self.query_dependency_slice.semantic_refs
        {
            return Err("Mabo query demand drifted from dependency slice".into());
        }

        let visible = self
            .projection
            .nodes
            .iter()
            .map(|node| node.semantic_ref.clone())
            .collect::<BTreeSet<_>>();
        let unresolved = self
            .query_demand
            .required_semantic_refs
            .difference(&visible)
            .cloned()
            .collect::<BTreeSet<_>>();
        let diagnosed = self
            .diagnosed_identity_residual_refs
            .iter()
            .filter_map(|residual| {
                residual
                    .strip_prefix("residual:mabo:world-identity:")
                    .map(ToOwned::to_owned)
            })
            .collect::<BTreeSet<_>>();
        if unresolved != diagnosed {
            return Err(
                "Mabo query/world snapshot unresolved identity set diverged from diagnosis"
                    .into(),
            );
        }

        if self.operational_state == OperationalResearchState::CurrentFrontierClosed {
            if !self.revision_probe.probe_complete
                || self.revision_probe.probe_truncated
                || !self.revision_probe.blocker_source_refs.is_empty()
                || !self.revision_probe.revision_reopen_residual_refs.is_empty()
                || !self.diagnosed_identity_residual_refs.is_empty()
            {
                return Err(
                    "Mabo snapshot claimed closed while revision/identity work remained"
                        .into(),
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mabo_revision_campaign::MaboRevisionProbeReceipt;
    use sensiblaw_pg_source_store::{LatentWorldEdgeRow, LatentWorldRows};

    fn mature_world() -> LatentWorldRows {
        LatentWorldRows {
            seed_ref: "Q1".into(),
            max_hops: 100,
            requested_max_hops: 100,
            visited_refs: vec!["Q1".into(), "Q2".into()],
            deepest_observed_hop: 1,
            frontier_exhausted: true,
            frontier_refs: vec![],
            residual_refs: vec![],
            edges: vec![LatentWorldEdgeRow {
                from_ref: "Q1".into(),
                to_ref: "Q2".into(),
                relation_ref: "context:wikidata:participant".into(),
                provenance_refs: vec![
                    "context:wikidata:wikidata:Q1:oldid:100".into(),
                ],
            }],
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn probe() -> MaboRevisionProbeReceipt {
        MaboRevisionProbeReceipt {
            reviewed_source_count: 1,
            probed_source_count: 1,
            reopen_residuals: vec![],
            unchanged_source_refs: vec!["Q1".into()],
            unprobed_source_refs: vec![],
            blockers: vec![],
            probe_truncated: false,
            probe_complete: true,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn mature_closed_world_snapshot_is_runtime_complete_but_not_formal_adequacy() {
        let baseline = DiscoveryIdentityBaseline {
            identity_class_refs: BTreeSet::from(["world-object:known".into()]),
            representation_identity_class_refs: BTreeMap::from([(
                "Q2".into(),
                "world-object:known".into(),
            )]),
        };
        let context_slice = ContextRevisionWorldSlice {
            wikidata_source_revisions: BTreeMap::from([(
                "Q1".into(),
                "wikidata:Q1:oldid:100".into(),
            )]),
        };
        let snapshot = compile_mabo_query_world_snapshot(
            "Q1",
            "2026-09-22",
            &mature_world(),
            &baseline,
            &context_slice,
            &probe(),
        )
        .unwrap();

        assert_eq!(
            snapshot.operational_state,
            OperationalResearchState::CurrentFrontierClosed
        );
        assert_eq!(
            snapshot.query_demand.required_semantic_refs,
            BTreeSet::from(["Q2".into()])
        );
        assert_eq!(snapshot.projection.nodes.len(), 1);
        assert!(snapshot.diagnosed_identity_residual_refs.is_empty());
        assert!(!snapshot.creates_semantic_authority);
        assert!(!snapshot.creates_claim_truth);
    }

    #[test]
    fn unseen_context_target_remains_absent_and_reopens_identity_research() {
        let baseline = DiscoveryIdentityBaseline::default();
        let context_slice = ContextRevisionWorldSlice {
            wikidata_source_revisions: BTreeMap::from([(
                "Q1".into(),
                "wikidata:Q1:oldid:100".into(),
            )]),
        };
        let snapshot = compile_mabo_query_world_snapshot(
            "Q1",
            "2026-09-22",
            &mature_world(),
            &baseline,
            &context_slice,
            &probe(),
        )
        .unwrap();

        assert_eq!(snapshot.projection.nodes.len(), 0);
        assert_eq!(
            snapshot.diagnosed_identity_residual_refs,
            vec!["residual:mabo:world-identity:Q2"]
        );
        assert_eq!(snapshot.operational_state, OperationalResearchState::Open);
    }

    #[test]
    fn mature_closed_snapshot_requires_fresh_adequacy_witness() {
        let baseline = DiscoveryIdentityBaseline {
            identity_class_refs: BTreeSet::from(["world-object:known".into()]),
            representation_identity_class_refs: BTreeMap::from([(
                "Q2".into(),
                "world-object:known".into(),
            )]),
        };
        let context_slice = ContextRevisionWorldSlice {
            wikidata_source_revisions: BTreeMap::from([(
                "Q1".into(),
                "wikidata:Q1:oldid:100".into(),
            )]),
        };
        let snapshot = compile_mabo_query_world_snapshot(
            "Q1",
            "2026-09-22",
            &mature_world(),
            &baseline,
            &context_slice,
            &probe(),
        )
        .unwrap();

        let decision = snapshot.decide_current_world(None, &[]).unwrap();
        assert_eq!(
            decision.kind,
            sensiblaw_legal_runtime::QueryWorldRunDecisionKind::RequireFreshAdequacyWitness
        );
        assert!(decision.operational_frontier_closed);
        assert!(decision.run_may_stop);
        assert!(!decision.consumer_adequate_formally_proved);
    }

    #[test]
    fn unresolved_identity_snapshot_reopens_research_without_fabricated_proof() {
        let context_slice = ContextRevisionWorldSlice {
            wikidata_source_revisions: BTreeMap::from([(
                "Q1".into(),
                "wikidata:Q1:oldid:100".into(),
            )]),
        };
        let snapshot = compile_mabo_query_world_snapshot(
            "Q1",
            "2026-09-22",
            &mature_world(),
            &DiscoveryIdentityBaseline::default(),
            &context_slice,
            &probe(),
        )
        .unwrap();

        let decision = snapshot.decide_current_world(None, &[]).unwrap();
        assert_eq!(
            decision.kind,
            sensiblaw_legal_runtime::QueryWorldRunDecisionKind::ReopenExactResearch
        );
        assert!(!decision.operational_frontier_closed);
        assert!(!decision.run_may_stop);
        assert!(!decision.consumer_adequate_formally_proved);
        assert!(!decision.unproved_demand_reason_refs.is_empty());
    }


    #[test]
    fn snapshot_json_roundtrip_preserves_controller_decision() {
        let baseline = DiscoveryIdentityBaseline {
            identity_class_refs: BTreeSet::from(["world-object:known".into()]),
            representation_identity_class_refs: BTreeMap::from([(
                "Q2".into(),
                "world-object:known".into(),
            )]),
        };
        let context_slice = ContextRevisionWorldSlice {
            wikidata_source_revisions: BTreeMap::from([(
                "Q1".into(),
                "wikidata:Q1:oldid:100".into(),
            )]),
        };
        let snapshot = compile_mabo_query_world_snapshot(
            "Q1",
            "2026-09-22",
            &mature_world(),
            &baseline,
            &context_slice,
            &probe(),
        )
        .unwrap();

        let before = snapshot.decide_current_world(None, &[]).unwrap();
        let json = serde_json::to_string(&snapshot).unwrap();
        let decoded: MaboQueryWorldSnapshot = serde_json::from_str(&json).unwrap();
        decoded.validate().unwrap();
        let after = decoded.decide_current_world(None, &[]).unwrap();

        assert_eq!(before.kind, after.kind);
        assert_eq!(before.query_ref, after.query_ref);
        assert_eq!(
            before.impact.new_projection.graph.deterministic_digest,
            after.impact.new_projection.graph.deterministic_digest
        );
        assert_eq!(
            after.kind,
            sensiblaw_legal_runtime::QueryWorldRunDecisionKind::RequireFreshAdequacyWitness
        );
        assert!(!after.consumer_adequate_formally_proved);
    }

}
