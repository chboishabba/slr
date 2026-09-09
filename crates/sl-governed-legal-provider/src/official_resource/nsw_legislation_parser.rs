//! Parser handoff for already-admitted point-in-time NSW legislation.
//!
//! Only a `HistoricalLegislationReceipt` may create this handoff. Raw live
//! fetch bytes, current consolidations, or unverified local files cannot be
//! passed to spaCy/PNF through this carrier.

use crate::official_resource::nsw_legislation::HistoricalLegislationReceipt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalLegislationParserHandoff {
    pub source_identity_ref: String,
    pub source_revision_ref: String,
    pub jurisdiction_ref: String,
    pub official_document_id: String,
    pub locator: String,
    pub in_force_on: String,
    pub canonical_text_digest: String,
    pub local_artifact_ref: String,
    pub parser_authority: &'static str,
}

pub const HISTORICAL_LEGISLATION_PARSER_AUTHORITY: &str = "source_observation_only";

pub fn parser_handoff_from_historical_receipt(
    receipt: &HistoricalLegislationReceipt,
) -> Option<HistoricalLegislationParserHandoff> {
    if !receipt.parser_eligible || !receipt.compile_eligible {
        return None;
    }
    Some(HistoricalLegislationParserHandoff {
        source_identity_ref: receipt.source_identity_ref.clone(),
        source_revision_ref: receipt.source_revision_ref.clone(),
        jurisdiction_ref: receipt.jurisdiction_ref.clone(),
        official_document_id: receipt.official_document_id.clone(),
        locator: receipt.locator.clone(),
        in_force_on: receipt.in_force_on.clone(),
        canonical_text_digest: receipt.canonical_text_digest.clone(),
        local_artifact_ref: receipt.local_artifact_ref.clone(),
        parser_authority: HISTORICAL_LEGISLATION_PARSER_AUTHORITY,
    })
}

pub const fn parser_handoff_creates_legal_authority() -> bool {
    false
}

pub const fn parser_handoff_creates_atomic_gate() -> bool {
    false
}
