//! Revision-separated candidate join of Zelph graph observations and native
//! Wikidata statement bundles. This is NOT a graph-derived P14143/P854 fact,
//! a source payment, an S29 approval, or a public Wikidata edit.
//!
//! DAG provenance: physical object -> decoded graph observation -> read-only
//! candidate join -> independently reviewed native statement/source.
use crate::bundle::StatementBundleReview;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphCoverage {
    Uninspected,
    Partial,
    DeclaredQueryFamily, // still not automatically full source-graph coverage
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalZelphObservation {
    pub artifact_id: String,
    pub graph_revision: String,
    pub subject_qid: String,
    pub internal_node_id: u64,
    pub name_shard_ids: Vec<String>,
    pub graph_query_family: String,
    pub route_receipt_ref: String,
    pub coverage: GraphCoverage,
}

/// An asserted correlation of two *different* revision systems.
/// The caller must separately check the supplied provenance receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionCorrespondence {
    pub graph_revision: String,
    pub native_revision: String,
    pub source_mapping_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNativeEvidencePair {
    pub subject_qid: String,
    pub native_bundle_id: String,
    pub native_source_unit_id: String,
    pub native_property_id: String,
    pub native_revision: String,
    pub graph_artifact_id: String,
    pub graph_revision: String,
    pub internal_node_id: u64,
    pub physical_name_shards: Vec<String>,
    pub graph_query_family: String,
    pub graph_coverage: GraphCoverage,
    pub native_qualifiers: Vec<String>,
    pub native_references: Vec<String>,
    pub source_mapping_receipt_ref: String,
    pub route_receipt_ref: String,
    pub candidate_only: bool,
    pub creates_truth: bool,
    pub creates_source_support_payment: bool,
    pub creates_edit_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphNativeJoinError {
    EmptyRequiredEvidence,
    SubjectQidMismatch,
    MixedGraphRevision,
    MissingNativeRevision,
    NoNameShardWitness,
}

/// Explicit *candidate* join.  A graph revision is not substituted for a
/// native Wikidata statement revision, and no native GUID is invented from
/// an internal Zelph node ID or from a bundle identifier.
pub fn candidate_graph_native_join(
    bundle: &StatementBundleReview,
    native_revision: &str,
    graph: &PhysicalZelphObservation,
    correspondence: &RevisionCorrespondence,
) -> Result<GraphNativeEvidencePair, GraphNativeJoinError> {
    if [
        &graph.artifact_id,
        &graph.graph_revision,
        &graph.subject_qid,
        &graph.graph_query_family,
        &graph.route_receipt_ref,
        &correspondence.source_mapping_receipt_ref,
    ]
    .iter()
    .any(|value| value.is_empty())
    {
        return Err(GraphNativeJoinError::EmptyRequiredEvidence);
    }
    if graph.name_shard_ids.is_empty()
        || graph.name_shard_ids.iter().any(String::is_empty)
    {
        return Err(GraphNativeJoinError::NoNameShardWitness);
    }
    if graph.subject_qid != bundle.subject_qid {
        return Err(GraphNativeJoinError::SubjectQidMismatch);
    }
    if graph.graph_revision != correspondence.graph_revision {
        return Err(GraphNativeJoinError::MixedGraphRevision);
    }
    if native_revision.is_empty()
        || correspondence.native_revision.is_empty()
        || native_revision != correspondence.native_revision
    {
        return Err(GraphNativeJoinError::MissingNativeRevision);
    }
    Ok(GraphNativeEvidencePair {
        subject_qid: bundle.subject_qid.clone(),
        native_bundle_id: bundle.bundle_id.clone(),
        native_source_unit_id: bundle.source_unit_id.clone(),
        native_property_id: bundle.property_id.clone(),
        native_revision: native_revision.into(),
        graph_artifact_id: graph.artifact_id.clone(),
        graph_revision: graph.graph_revision.clone(),
        internal_node_id: graph.internal_node_id,
        physical_name_shards: graph.name_shard_ids.clone(),
        graph_query_family: graph.graph_query_family.clone(),
        graph_coverage: graph.coverage,
        native_qualifiers: bundle.qualifier_refs.clone(),
        native_references: bundle.reference_refs.clone(),
        source_mapping_receipt_ref: correspondence.source_mapping_receipt_ref.clone(),
        route_receipt_ref: graph.route_receipt_ref.clone(),
        candidate_only: true,
        creates_truth: false,
        creates_source_support_payment: false,
        creates_edit_authority: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::WikidataRank;
    use crate::MigrationDisposition;

    fn bundle() -> StatementBundleReview {
        StatementBundleReview::new(
            "bundle:source:statement".into(),
            "source-unit:nat".into(),
            "Q10403939".into(),
            "P5991".into(),
            "mainsnak:nat".into(),
            vec!["P580:2018".into(), "P3831:scope".into()],
            vec!["P854:source-document".into()],
            WikidataRank::Normal,
            vec!["provenance:nat".into()],
            MigrationDisposition::SplitRequired,
            vec!["source still needs review".into()],
        ).unwrap()
    }
    fn graph() -> PhysicalZelphObservation {
        PhysicalZelphObservation {
            artifact_id: "wikidata-20260309-all".into(),
            graph_revision: "hf:immutable-dataset-sha".into(),
            subject_qid: "Q10403939".into(),
            internal_node_id: 24721153,
            name_shard_ids: vec!["chunk-000001-wikidata".into()],
            graph_query_family: "nodeOfName".into(),
            route_receipt_ref: "sha256:raw-physical-route".into(),
            coverage: GraphCoverage::Partial,
        }
    }
    fn correspondence() -> RevisionCorrespondence {
        RevisionCorrespondence {
            graph_revision: "hf:immutable-dataset-sha".into(),
            native_revision: "wikidata:revision-123".into(),
            source_mapping_receipt_ref: "receipt:source-correspondence".into(),
        }
    }

    #[test]
    fn candidate_join_preserves_both_evidence_planes_and_authority_firewall() {
        let joined = candidate_graph_native_join(
            &bundle(), "wikidata:revision-123", &graph(), &correspondence()
        ).unwrap();
        assert_eq!(joined.internal_node_id, 24721153);
        assert_eq!(joined.native_property_id, "P5991");
        assert_eq!(joined.graph_coverage, GraphCoverage::Partial);
        assert_eq!(joined.native_references, vec!["P854:source-document"]);
        assert!(joined.candidate_only);
        assert!(!joined.creates_truth);
        assert!(!joined.creates_source_support_payment);
        assert!(!joined.creates_edit_authority);
    }

    #[test]
    fn graph_qid_mismatch_cannot_join_native_statement() {
        let mut changed = graph();
        changed.subject_qid = "Q10422059".into();
        assert_eq!(
            candidate_graph_native_join(
                &bundle(), "wikidata:revision-123", &changed, &correspondence()
            ).unwrap_err(),
            GraphNativeJoinError::SubjectQidMismatch
        );
    }

    #[test]
    fn changed_graph_and_native_revisions_require_explicit_recheck() {
        let mut changed = graph();
        changed.graph_revision = "hf:new".into();
        assert_eq!(
            candidate_graph_native_join(
                &bundle(), "wikidata:revision-123", &changed, &correspondence()
            ).unwrap_err(),
            GraphNativeJoinError::MixedGraphRevision
        );
        assert_eq!(
            candidate_graph_native_join(
                &bundle(), "wikidata:revision-124", &graph(), &correspondence()
            ).unwrap_err(),
            GraphNativeJoinError::MissingNativeRevision
        );
    }

    #[test]
    fn missing_physical_witness_refuses_candidate_pair() {
        let mut changed = graph();
        changed.name_shard_ids.clear();
        assert_eq!(
            candidate_graph_native_join(
                &bundle(), "wikidata:revision-123", &changed, &correspondence()
            ).unwrap_err(),
            GraphNativeJoinError::NoNameShardWitness
        );
    }
}
