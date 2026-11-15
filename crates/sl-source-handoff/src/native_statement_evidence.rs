//! B5 source-preserving *candidate* native Wikibase statement comparison.
//!
//! This is an in-process ABI guard between existing SLR StatementBundleReview
//! and a separately captured native Wikibase Action API entity/statement.
//! It cannot authenticate the supplied entity bytes or P854 document on its
//! own; the native fetch/parser and independent primary-source reviewer must
//! produce those receipts. No invented GUID, statement migration or edit.

use crate::bundle::{StatementBundleReview, WikidataRank};
use crate::zelph_graph_native_join::GraphNativeEvidencePair;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeWikibaseStatementObservation {
    pub subject_qid: String,
    /// Wikibase statement ID: never synthesized from Zelph node ID.
    pub statement_guid: String,
    pub property_id: String,
    pub native_revision: String,
    /// Digest claim of independently acquired native raw JSON; not rehashed here.
    pub entity_raw_sha256: String,
    pub mainsnak_ref: String,
    pub rank: WikidataRank,
    /// Exact ordered native values. Do not silently collapse 1-to-N statements.
    pub qualifier_refs: Vec<String>,
    pub reference_refs: Vec<String>,
    /// Exact P854 URL values as decoded from the native statement references.
    pub p854_urls: Vec<String>,
    pub native_entity_snapshot_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNativeStatementCandidate {
    pub subject_qid: String,
    pub statement_guid: String,
    pub property_id: String,
    pub graph_revision: String,
    pub native_revision: String,
    pub route_receipt_ref: String,
    pub source_mapping_receipt_ref: String,
    pub entity_raw_sha256_claim: String,
    pub native_snapshot_ref: String,
    pub p854_url_candidates: Vec<String>,
    pub verified_source_document_receipt: Option<String>,
    pub source_support_paid: bool,
    pub owns_semantic_promotion: bool,
    pub owns_edit_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStatementJoinError {
    CandidateIsAuthoritative,
    MissingNativeStatementIdentity,
    InvalidNativeHashClaim,
    QidMismatch,
    PropertyMismatch,
    NativeRevisionMismatch,
    MainSnakMismatch,
    RankMismatch,
    QualifierDrift,
    ReferenceDrift,
    P854ReferenceMismatch,
}

fn lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn exact_p854_refs(refs: &[String]) -> Vec<String> {
    refs.iter()
        .filter_map(|reference| reference.strip_prefix("P854:"))
        .map(str::to_owned)
        .collect()
}

/// Compare an independently supplied native observation with BOTH existing
/// SLR and graph-only candidate coordinates. No source/truth/evidence payment.
pub fn join_native_statement_read_only(
    graph: &GraphNativeEvidencePair,
    bundle: &StatementBundleReview,
    native: &NativeWikibaseStatementObservation,
) -> Result<GraphNativeStatementCandidate, NativeStatementJoinError> {
    if !graph.candidate_only || graph.creates_truth
        || graph.creates_source_support_payment || graph.creates_edit_authority
        || bundle.creates_world_truth || bundle.owns_source_authority
        || bundle.owns_semantic_promotion
    {
        return Err(NativeStatementJoinError::CandidateIsAuthoritative);
    }
    if native.statement_guid.is_empty() || native.native_revision.is_empty()
        || native.native_entity_snapshot_ref.is_empty()
        || native.mainsnak_ref.is_empty()
    {
        return Err(NativeStatementJoinError::MissingNativeStatementIdentity);
    }
    if !lower_hex_sha256(&native.entity_raw_sha256) {
        return Err(NativeStatementJoinError::InvalidNativeHashClaim);
    }
    if native.subject_qid != bundle.subject_qid || native.subject_qid != graph.subject_qid {
        return Err(NativeStatementJoinError::QidMismatch);
    }
    if native.property_id != bundle.property_id || native.property_id != graph.native_property_id {
        return Err(NativeStatementJoinError::PropertyMismatch);
    }
    if native.native_revision != graph.native_revision {
        return Err(NativeStatementJoinError::NativeRevisionMismatch);
    }
    if native.mainsnak_ref != bundle.mainsnak_ref {
        return Err(NativeStatementJoinError::MainSnakMismatch);
    }
    if native.rank != bundle.rank {
        return Err(NativeStatementJoinError::RankMismatch);
    }
    if native.qualifier_refs != bundle.qualifier_refs
        || native.qualifier_refs != graph.native_qualifiers
    {
        return Err(NativeStatementJoinError::QualifierDrift);
    }
    if native.reference_refs != bundle.reference_refs
        || native.reference_refs != graph.native_references
    {
        return Err(NativeStatementJoinError::ReferenceDrift);
    }
    let p854 = exact_p854_refs(&native.reference_refs);
    if p854 != native.p854_urls || p854.iter().any(|uri| {
        !uri.starts_with("https://") && !uri.starts_with("http://")
    }) {
        return Err(NativeStatementJoinError::P854ReferenceMismatch);
    }
    Ok(GraphNativeStatementCandidate {
        subject_qid: native.subject_qid.clone(),
        statement_guid: native.statement_guid.clone(),
        property_id: native.property_id.clone(),
        graph_revision: graph.graph_revision.clone(),
        native_revision: native.native_revision.clone(),
        route_receipt_ref: graph.route_receipt_ref.clone(),
        source_mapping_receipt_ref: graph.source_mapping_receipt_ref.clone(),
        entity_raw_sha256_claim: native.entity_raw_sha256.clone(),
        native_snapshot_ref: native.native_entity_snapshot_ref.clone(),
        p854_url_candidates: native.p854_urls.clone(),
        verified_source_document_receipt: None,
        source_support_paid: false,
        owns_semantic_promotion: false,
        owns_edit_authority: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::WikidataRank;
    use crate::zelph_graph_native_join::{
        candidate_graph_native_join, GraphCoverage, PhysicalZelphObservation,
        RevisionCorrespondence,
    };
    use crate::MigrationDisposition;

    fn input() -> (StatementBundleReview, GraphNativeEvidencePair, NativeWikibaseStatementObservation) {
        let bundle = StatementBundleReview::new(
            "bundle:source:statement".into(), "source-unit:nat".into(),
            "Q10403939".into(), "P5991".into(), "mainsnak:nat".into(),
            vec!["P580:2018".into(), "P3831:scope".into()],
            vec!["P854:https://example.test/report.pdf".into()],
            WikidataRank::Normal, vec!["provenance:nat".into()],
            MigrationDisposition::SplitRequired,
            vec!["source still needs review".into()],
        ).unwrap();
        let graph = PhysicalZelphObservation {
            artifact_id: "wikidata-20260309-all".into(),
            graph_revision: "hf:immutable-SHA".into(),
            subject_qid: "Q10403939".into(),
            internal_node_id: 24721153,
            name_shard_ids: vec!["nodeOfName-chunk-1".into()],
            graph_query_family: "nodeOfName".into(),
            route_receipt_ref: "sha256:route".into(),
            coverage: GraphCoverage::Partial,
        };
        let corr = RevisionCorrespondence {
            graph_revision: graph.graph_revision.clone(),
            native_revision: "wikidata:rev123".into(),
            source_mapping_receipt_ref: "receipt:cross-revision".into(),
        };
        let pair = candidate_graph_native_join(&bundle, "wikidata:rev123", &graph, &corr).unwrap();
        let native = NativeWikibaseStatementObservation {
            subject_qid: "Q10403939".into(),
            statement_guid: "Q10403939$from-original-wikibase".into(),
            property_id: "P5991".into(),
            native_revision: "wikidata:rev123".into(),
            entity_raw_sha256: "a".repeat(64),
            mainsnak_ref: "mainsnak:nat".into(),
            rank: WikidataRank::Normal,
            qualifier_refs: bundle.qualifier_refs.clone(),
            reference_refs: bundle.reference_refs.clone(),
            p854_urls: vec!["https://example.test/report.pdf".into()],
            native_entity_snapshot_ref: "raw:entity-123.json".into(),
        };
        (bundle, pair, native)
    }

    #[test]
    fn full_native_coordinates_preserved_but_no_source_payment() {
        let (bundle, pair, native) = input();
        let output = join_native_statement_read_only(&pair, &bundle, &native).unwrap();
        assert_eq!(output.statement_guid, native.statement_guid);
        assert_eq!(output.p854_url_candidates, native.p854_urls);
        assert_eq!(output.entity_raw_sha256_claim, native.entity_raw_sha256);
        assert_eq!(output.verified_source_document_receipt, None);
        assert!(!output.source_support_paid);
        assert!(!output.owns_semantic_promotion);
        assert!(!output.owns_edit_authority);
    }

    #[test]
    fn rejects_changed_qid_property_revision_and_guid() {
        let (bundle, pair, native) = input();
        let mut changed = native.clone();
        changed.subject_qid = "Q10422059".into();
        assert_eq!(join_native_statement_read_only(&pair, &bundle, &changed).unwrap_err(),
            NativeStatementJoinError::QidMismatch);
        changed = native.clone();
        changed.property_id = "P14143".into();
        assert_eq!(join_native_statement_read_only(&pair, &bundle, &changed).unwrap_err(),
            NativeStatementJoinError::PropertyMismatch);
        changed = native.clone();
        changed.native_revision = "wikidata:rev124".into();
        assert_eq!(join_native_statement_read_only(&pair, &bundle, &changed).unwrap_err(),
            NativeStatementJoinError::NativeRevisionMismatch);
        changed = native.clone();
        changed.statement_guid.clear();
        assert_eq!(join_native_statement_read_only(&pair, &bundle, &changed).unwrap_err(),
            NativeStatementJoinError::MissingNativeStatementIdentity);
    }

    #[test]
    fn refuses_qualifier_reference_rank_and_mainsnak_drift() {
        let (bundle, pair, native) = input();
        let mut changed = native.clone();
        changed.qualifier_refs.clear();
        assert_eq!(join_native_statement_read_only(&pair, &bundle, &changed).unwrap_err(),
            NativeStatementJoinError::QualifierDrift);
        changed = native.clone();
        changed.reference_refs.clear();
        assert_eq!(join_native_statement_read_only(&pair, &bundle, &changed).unwrap_err(),
            NativeStatementJoinError::ReferenceDrift);
        changed = native.clone();
        changed.rank = WikidataRank::Preferred;
        assert_eq!(join_native_statement_read_only(&pair, &bundle, &changed).unwrap_err(),
            NativeStatementJoinError::RankMismatch);
        changed = native.clone();
        changed.mainsnak_ref = "other".into();
        assert_eq!(join_native_statement_read_only(&pair, &bundle, &changed).unwrap_err(),
            NativeStatementJoinError::MainSnakMismatch);
    }

    #[test]
    fn p854_url_and_raw_hash_claim_cannot_be_forged_silently() {
        let (bundle, pair, native) = input();
        let mut changed = native.clone();
        changed.p854_urls = vec!["https://another.test/report.pdf".into()];
        assert_eq!(join_native_statement_read_only(&pair, &bundle, &changed).unwrap_err(),
            NativeStatementJoinError::P854ReferenceMismatch);
        changed = native.clone();
        changed.entity_raw_sha256 = "not-a-hash".into();
        assert_eq!(join_native_statement_read_only(&pair, &bundle, &changed).unwrap_err(),
            NativeStatementJoinError::InvalidNativeHashClaim);
    }
}
