//! B5 independent raw Wikibase observation receipt ↔ typed SLR candidate join.
//!
//! This coordinates the offline ITIR full-entity JSON byte verifier with the
//! existing SLR native candidate review. It DOES NOT parse/re-hash supplied
//! raw bytes, authenticate upstream HTTP provenance, verify a P854 document,
//! pay a native source proposition or persist an S29 reviewer decision.

use crate::native_statement_evidence::GraphNativeStatementCandidate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEntityRawReceipt {
    pub source_kind: String, // exactly full-wikibase-json-entity
    pub subject_qid: String,
    pub statement_guid: String,
    pub property_id: String,
    pub native_entity_revision_id: u64,
    pub raw_entity_sha256_checked: String,
    pub snapshot_bytes_match_pinned_digest: bool,
    pub same_source_native_statement_found: bool,
    pub p854_url_candidates: Vec<String>,
    pub quantitative_amount: Option<String>,
    pub quantitative_unit: Option<String>,
    pub upstream_http_provenance_independently_verified: bool,
    pub p854_documents_independently_verified: bool,
    pub reviewed_source_support_paid: bool,
    pub persisted_s29_review_verified: bool,
    pub edit_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRawReceiptError {
    UnsupportedOriginalSchema,
    UncheckedRawDigest,
    SubjectMismatch,
    StatementGuidMismatch,
    PropertyMismatch,
    NativeRevisionMismatch,
    NativeRawDigestMismatch,
    P854CandidateMismatch,
    NumericP14143QuantityMissing,
    UnsafeAuthorityClaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawNativeCandidateForReview {
    pub statement_guid: String,
    pub native_revision: String,
    pub raw_entity_sha256_claim: String,
    pub p854_url_candidates: Vec<String>,
    pub native_quantity_amount: Option<String>,
    pub native_quantity_unit: Option<String>,
    pub raw_receipt_coordinates_match: bool,
    pub original_http_response_authenticated_in_slr: bool,
    pub primary_p854_documents_verified: bool,
    pub s29_persistent_review_verified: bool,
    pub source_support_paid: bool,
    pub creates_edit_authority: bool,
}

/// Explicit same-entity/rank-independent crosswalk; byte cryptographic trust
/// still belongs to the upstream actual-raw-file verifier and source origin.
/// This deliberately does not treat a caller-provided true Boolean as evidence
/// that SLR itself rehashed an original Wikibase response or checked the URL.
pub fn join_raw_native_receipt_read_only(
    joined: &GraphNativeStatementCandidate,
    receipt: &NativeEntityRawReceipt,
) -> Result<RawNativeCandidateForReview, NativeRawReceiptError> {
    if receipt.source_kind != "full-wikibase-json-entity" {
        return Err(NativeRawReceiptError::UnsupportedOriginalSchema);
    }
    if !receipt.snapshot_bytes_match_pinned_digest || !receipt.same_source_native_statement_found {
        return Err(NativeRawReceiptError::UncheckedRawDigest);
    }
    if !joined.entity_raw_sha256_claim.bytes().all(|b| b.is_ascii_hexdigit())
        || joined.entity_raw_sha256_claim.len() != 64
        || !receipt.raw_entity_sha256_checked.bytes().all(|b| b.is_ascii_hexdigit())
        || receipt.raw_entity_sha256_checked.len() != 64
    {
        return Err(NativeRawReceiptError::UncheckedRawDigest);
    }
    if joined.subject_qid != receipt.subject_qid {
        return Err(NativeRawReceiptError::SubjectMismatch);
    }
    if joined.statement_guid != receipt.statement_guid {
        return Err(NativeRawReceiptError::StatementGuidMismatch);
    }
    if joined.property_id != receipt.property_id {
        return Err(NativeRawReceiptError::PropertyMismatch);
    }
    if receipt.native_entity_revision_id == 0
        || joined.native_revision != format!("wikidata:rev{}", receipt.native_entity_revision_id)
    {
        return Err(NativeRawReceiptError::NativeRevisionMismatch);
    }
    if joined.entity_raw_sha256_claim != receipt.raw_entity_sha256_checked {
        return Err(NativeRawReceiptError::NativeRawDigestMismatch);
    }
    if joined.p854_url_candidates != receipt.p854_url_candidates {
        return Err(NativeRawReceiptError::P854CandidateMismatch);
    }
    if joined.property_id == "P14143"
        && (receipt.quantitative_amount.as_ref().map(|x| x.is_empty()).unwrap_or(true)
            || receipt.quantitative_unit.as_ref().map(|x| x.is_empty()).unwrap_or(true))
    {
        return Err(NativeRawReceiptError::NumericP14143QuantityMissing);
    }
    if joined.source_support_paid || joined.owns_semantic_promotion || joined.owns_edit_authority
        || receipt.reviewed_source_support_paid
        || receipt.persisted_s29_review_verified || receipt.edit_authority
    {
        return Err(NativeRawReceiptError::UnsafeAuthorityClaim);
    }
    Ok(RawNativeCandidateForReview {
        statement_guid: joined.statement_guid.clone(),
        native_revision: joined.native_revision.clone(),
        raw_entity_sha256_claim: receipt.raw_entity_sha256_checked.clone(),
        p854_url_candidates: receipt.p854_url_candidates.clone(),
        native_quantity_amount: receipt.quantitative_amount.clone(),
        native_quantity_unit: receipt.quantitative_unit.clone(),
        raw_receipt_coordinates_match: true,
        original_http_response_authenticated_in_slr: false,
        primary_p854_documents_verified: false,
        s29_persistent_review_verified: false,
        source_support_paid: false,
        creates_edit_authority: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (GraphNativeStatementCandidate, NativeEntityRawReceipt) {
        let qid = "Q10403939".to_owned();
        let guid = "Q10403939$00000000-0000-4000-8000-111111111111".to_owned();
        let sha = "a".repeat(64);
        let urls = vec!["https://example.test/primary-report.pdf".to_owned()];
        let joined = GraphNativeStatementCandidate {
            subject_qid: qid.clone(),
            statement_guid: guid.clone(),
            property_id: "P14143".into(),
            graph_revision: "hf:immutable-sha".into(),
            native_revision: "wikidata:rev123".into(),
            route_receipt_ref: "sha256:source-route".into(),
            source_mapping_receipt_ref: "receipt:correspondence".into(),
            entity_raw_sha256_claim: sha.clone(),
            native_snapshot_ref: "raw:entity-123.json".into(),
            p854_url_candidates: urls.clone(),
            verified_source_document_receipt: None,
            source_support_paid: false,
            owns_semantic_promotion: false,
            owns_edit_authority: false,
        };
        let receipt = NativeEntityRawReceipt {
            source_kind: "full-wikibase-json-entity".into(),
            subject_qid: qid,
            statement_guid: guid,
            property_id: "P14143".into(),
            native_entity_revision_id: 123,
            raw_entity_sha256_checked: sha,
            snapshot_bytes_match_pinned_digest: true,
            same_source_native_statement_found: true,
            p854_url_candidates: urls,
            quantitative_amount: Some("+20.6".into()),
            quantitative_unit: Some("http://www.wikidata.org/entity/Q8799".into()),
            upstream_http_provenance_independently_verified: false,
            p854_documents_independently_verified: false,
            reviewed_source_support_paid: false,
            persisted_s29_review_verified: false,
            edit_authority: false,
        };
        (joined, receipt)
    }

    #[test]
    fn exact_same_raw_entity_coordinates_produce_only_candidate() {
        let (joined, receipt) = fixture();
        let output = join_raw_native_receipt_read_only(&joined, &receipt).unwrap();
        assert_eq!(output.native_quantity_amount.as_deref(), Some("+20.6"));
        assert!(output.raw_receipt_coordinates_match);
        assert!(!output.original_http_response_authenticated_in_slr);
        assert!(!output.primary_p854_documents_verified);
        assert!(!output.s29_persistent_review_verified);
        assert!(!output.source_support_paid);
        assert!(!output.creates_edit_authority);
    }

    #[test]
    fn forged_guid_revision_digest_or_url_refused() {
        let (joined, orig) = fixture();
        let mut receipt = orig.clone();
        receipt.statement_guid.push_str("different");
        assert_eq!(join_raw_native_receipt_read_only(&joined, &receipt).unwrap_err(),
                   NativeRawReceiptError::StatementGuidMismatch);
        receipt = orig.clone();
        receipt.native_entity_revision_id = 124;
        assert_eq!(join_raw_native_receipt_read_only(&joined, &receipt).unwrap_err(),
                   NativeRawReceiptError::NativeRevisionMismatch);
        receipt = orig.clone();
        receipt.raw_entity_sha256_checked = "b".repeat(64);
        assert_eq!(join_raw_native_receipt_read_only(&joined, &receipt).unwrap_err(),
                   NativeRawReceiptError::NativeRawDigestMismatch);
        receipt = orig.clone();
        receipt.p854_url_candidates.clear();
        assert_eq!(join_raw_native_receipt_read_only(&joined, &receipt).unwrap_err(),
                   NativeRawReceiptError::P854CandidateMismatch);
    }

    #[test]
    fn p14143_without_quantity_and_invalid_entity_shape_refused() {
        let (joined, orig) = fixture();
        let mut receipt = orig.clone();
        receipt.quantitative_amount = None;
        assert_eq!(join_raw_native_receipt_read_only(&joined, &receipt).unwrap_err(),
                   NativeRawReceiptError::NumericP14143QuantityMissing);
        receipt = orig.clone();
        receipt.source_kind = "wikibase-rest-statement".into();
        assert_eq!(join_raw_native_receipt_read_only(&joined, &receipt).unwrap_err(),
                   NativeRawReceiptError::UnsupportedOriginalSchema);
        receipt = orig.clone();
        receipt.snapshot_bytes_match_pinned_digest = false;
        assert_eq!(join_raw_native_receipt_read_only(&joined, &receipt).unwrap_err(),
                   NativeRawReceiptError::UncheckedRawDigest);
    }

    #[test]
    fn no_flag_promotes_a_caller_claim_to_native_authority() {
        let (joined, orig) = fixture();
        let mut receipt = orig.clone();
        receipt.reviewed_source_support_paid = true;
        assert_eq!(join_raw_native_receipt_read_only(&joined, &receipt).unwrap_err(),
                   NativeRawReceiptError::UnsafeAuthorityClaim);
        receipt = orig.clone();
        // An external boolean asserting upstream source validity remains
        // untrusted metadata here; it must not become a verified SLR payment.
        receipt.p854_documents_independently_verified = true;
        receipt.upstream_http_provenance_independently_verified = true;
        let result = join_raw_native_receipt_read_only(&joined, &receipt).unwrap();
        assert!(!result.primary_p854_documents_verified);
        assert!(!result.original_http_response_authenticated_in_slr);
        assert!(!result.source_support_paid);
    }
}
