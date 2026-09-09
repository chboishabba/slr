//! Governed point-in-time NSW legislation acquisition carrier.
//!
//! This module is acquisition-only. It does not infer that a current
//! consolidation was in force historically, and it grants no legal/semantic
//! authority. A historical demand can be satisfied only by an immutable
//! retained artifact carrying explicit evidence that the source revision was
//! in force on the demanded date.

use crate::transport::{HttpRequest, HttpResponse};
use crate::KnownAuthorityDemand;

pub const NSW_LEGISLATION_PROVIDER_ID: &str = "provider:official-nsw-legislation";
pub const NSW_LEGISLATION_RECEIPT_AUTHORITY: &str = "experimental_candidate_only";
pub const NSW_LEGISLATION_ORIGIN: &str = "https://legislation.nsw.gov.au";
pub const NSW_LEGISLATION_UA: &str = "SensibLaw/0.1 governed-nsw-legislation-provider";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NSWLegislationProvider {
    OfficialNSWLegislation,
    RetainedArchivedPrimaryCopy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoricalVersionEvidenceKind {
    OfficialPointInTimeVersion,
    OfficialVersionHistoryWeld,
    RetainedArchivedPrimaryCopy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalLegislationDemand {
    pub demand_ref: String,
    pub jurisdiction_ref: String,
    pub act_identity_ref: String,
    /// Stable NSW legislation document id, e.g. `act-2002-022`.
    pub official_document_id: String,
    pub requested_locator: String,
    pub in_force_on: String,
    pub proposition_ref: String,
    pub source_identity_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalVersionEvidence {
    pub kind: HistoricalVersionEvidenceKind,
    /// Exact official/archival reference establishing the point-in-time state.
    pub evidence_reference: String,
    /// Human/machine-reviewable statement of what the reference establishes.
    pub evidence_scope: String,
    pub evidence_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficialPointInTimeFetchCandidate {
    pub demand_ref: String,
    pub official_document_id: String,
    pub requested_date: String,
    pub request_reference: String,
    pub final_reference: String,
    pub content_type: Option<String>,
    pub bytes: Vec<u8>,
    pub network_requests: u64,
    pub retained: bool,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImmutableLegislationArtifact {
    pub provider: NSWLegislationProvider,
    pub act_identity_ref: String,
    pub source_identity_ref: String,
    pub source_revision_ref: String,
    pub jurisdiction_ref: String,
    pub official_document_id: String,
    pub locator: String,
    pub source_date_ref: String,
    pub canonical_text_digest: String,
    pub local_artifact_ref: String,
    pub immutable: bool,
    pub point_in_time_evidence: HistoricalVersionEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalLegislationReceipt {
    pub demand_ref: String,
    pub act_identity_ref: String,
    pub source_identity_ref: String,
    pub source_revision_ref: String,
    pub jurisdiction_ref: String,
    pub official_document_id: String,
    pub locator: String,
    pub in_force_on: String,
    pub canonical_text_digest: String,
    pub local_artifact_ref: String,
    pub point_in_time_evidence: HistoricalVersionEvidence,
    pub compile_eligible: bool,
    pub parser_eligible: bool,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoricalLegislationError {
    WrongJurisdiction,
    DemandSourceIdentityMismatch,
    DemandPropositionMissing,
    DemandPropositionMismatch,
    ActIdentityMismatch,
    SourceIdentityMismatch,
    DocumentIdMismatch,
    LocatorMismatch,
    HistoricalDateMismatch,
    InvalidHistoricalDate,
    InvalidDocumentId,
    InvalidOfficialReference,
    UnexpectedHttpStatus,
    UnexpectedContentType,
    EmptyOfficialResponse,
    MutableArtifact,
    MissingRevision,
    MissingDigest,
    MissingLocalArtifact,
    MissingPointInTimeEvidence,
    CurrentConsolidationWithoutHistoricalEvidence,
}

fn looks_like_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn looks_like_nsw_document_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && (value.starts_with("act-") || value.starts_with("epi-") || value.starts_with("sl-"))
}

impl HistoricalLegislationDemand {
    pub fn validate(&self) -> Result<(), HistoricalLegislationError> {
        if self.jurisdiction_ref != "AU-NSW" {
            return Err(HistoricalLegislationError::WrongJurisdiction);
        }
        if !looks_like_iso_date(&self.in_force_on) {
            return Err(HistoricalLegislationError::InvalidHistoricalDate);
        }
        if !looks_like_nsw_document_id(&self.official_document_id) {
            return Err(HistoricalLegislationError::InvalidDocumentId);
        }
        Ok(())
    }
}

/// NSW Parliamentary Counsel documents this point-in-time XML shape for all
/// historical versions in the In Force/Repealed collections:
/// `/view/html/status/YYYY-MM-DD/id/xml`.
pub fn official_point_in_time_xml_reference(
    demand: &HistoricalLegislationDemand,
) -> Result<String, HistoricalLegislationError> {
    demand.validate()?;
    Ok(format!(
        "{NSW_LEGISLATION_ORIGIN}/view/html/inforce/{}/{}/xml",
        demand.in_force_on, demand.official_document_id
    ))
}

/// Construct the exact request for the existing governed HTTP executor. This
/// function does not itself perform I/O or bypass provider governance.
pub fn official_point_in_time_xml_request(
    demand: &HistoricalLegislationDemand,
) -> Result<HttpRequest, HistoricalLegislationError> {
    Ok(HttpRequest {
        url: official_point_in_time_xml_reference(demand)?,
        user_agent: NSW_LEGISLATION_UA.into(),
        referer: Some(NSW_LEGISLATION_ORIGIN.into()),
        timeout_seconds: 45,
    })
}

pub fn is_official_nsw_legislation_reference(reference: &str) -> bool {
    reference == NSW_LEGISLATION_ORIGIN
        || reference.starts_with("https://legislation.nsw.gov.au/")
}

/// Validate bytes returned by governed transport. This creates a fetch
/// candidate only. The bytes must still be retained, digested and admitted as
/// an immutable historical artifact before parser/compile eligibility.
pub fn validate_official_point_in_time_response(
    demand: &HistoricalLegislationDemand,
    request: &HttpRequest,
    response: HttpResponse,
) -> Result<OfficialPointInTimeFetchCandidate, HistoricalLegislationError> {
    let expected = official_point_in_time_xml_reference(demand)?;
    if request.url != expected || !is_official_nsw_legislation_reference(&response.final_url) {
        return Err(HistoricalLegislationError::InvalidOfficialReference);
    }
    if response.status_code != 200 {
        return Err(HistoricalLegislationError::UnexpectedHttpStatus);
    }
    if response.body.is_empty() {
        return Err(HistoricalLegislationError::EmptyOfficialResponse);
    }
    if response
        .content_type
        .as_deref()
        .is_some_and(|content_type| {
            let lower = content_type.to_ascii_lowercase();
            !lower.contains("xml") && !lower.contains("text/plain")
        })
    {
        return Err(HistoricalLegislationError::UnexpectedContentType);
    }
    Ok(OfficialPointInTimeFetchCandidate {
        demand_ref: demand.demand_ref.clone(),
        official_document_id: demand.official_document_id.clone(),
        requested_date: demand.in_force_on.clone(),
        request_reference: request.url.clone(),
        final_reference: response.final_url,
        content_type: response.content_type,
        bytes: response.body,
        network_requests: 1,
        retained: false,
        receipt_authority: NSW_LEGISLATION_RECEIPT_AUTHORITY,
    })
}

/// Bind the provider-neutral authority demand to the legislation-specific
/// historical requirement. This is a shape/identity weld only; it performs no
/// acquisition and does not close the proof residual.
pub fn bind_known_authority_to_historical_legislation(
    known: &KnownAuthorityDemand,
    historical: HistoricalLegislationDemand,
) -> Result<HistoricalLegislationDemand, HistoricalLegislationError> {
    historical.validate()?;
    if known.jurisdiction_ref != historical.jurisdiction_ref {
        return Err(HistoricalLegislationError::WrongJurisdiction);
    }
    if known.source_identity_ref != historical.source_identity_ref {
        return Err(HistoricalLegislationError::DemandSourceIdentityMismatch);
    }
    let Some(proposition) = known.proposition_ref.as_deref() else {
        return Err(HistoricalLegislationError::DemandPropositionMissing);
    };
    if proposition != historical.proposition_ref {
        return Err(HistoricalLegislationError::DemandPropositionMismatch);
    }
    Ok(historical)
}

/// Admit an already-retained immutable source revision as payment for the
/// historical acquisition demand. A present-day consolidation is insufficient
/// unless the point-in-time evidence independently establishes that this exact
/// revision/text state was in force on the demanded date.
pub fn admit_historical_legislation_artifact(
    demand: &HistoricalLegislationDemand,
    artifact: &ImmutableLegislationArtifact,
) -> Result<HistoricalLegislationReceipt, HistoricalLegislationError> {
    demand.validate()?;
    if artifact.act_identity_ref != demand.act_identity_ref {
        return Err(HistoricalLegislationError::ActIdentityMismatch);
    }
    if artifact.source_identity_ref != demand.source_identity_ref {
        return Err(HistoricalLegislationError::SourceIdentityMismatch);
    }
    if artifact.jurisdiction_ref != demand.jurisdiction_ref {
        return Err(HistoricalLegislationError::WrongJurisdiction);
    }
    if artifact.official_document_id != demand.official_document_id {
        return Err(HistoricalLegislationError::DocumentIdMismatch);
    }
    if artifact.locator != demand.requested_locator {
        return Err(HistoricalLegislationError::LocatorMismatch);
    }
    if artifact.source_date_ref != demand.in_force_on {
        return Err(HistoricalLegislationError::HistoricalDateMismatch);
    }
    if !artifact.immutable {
        return Err(HistoricalLegislationError::MutableArtifact);
    }
    if artifact.source_revision_ref.trim().is_empty() {
        return Err(HistoricalLegislationError::MissingRevision);
    }
    if artifact.canonical_text_digest.trim().is_empty() {
        return Err(HistoricalLegislationError::MissingDigest);
    }
    if artifact.local_artifact_ref.trim().is_empty() {
        return Err(HistoricalLegislationError::MissingLocalArtifact);
    }
    if artifact.point_in_time_evidence.evidence_reference.trim().is_empty()
        || artifact.point_in_time_evidence.evidence_scope.trim().is_empty()
    {
        return Err(HistoricalLegislationError::MissingPointInTimeEvidence);
    }

    Ok(HistoricalLegislationReceipt {
        demand_ref: demand.demand_ref.clone(),
        act_identity_ref: demand.act_identity_ref.clone(),
        source_identity_ref: artifact.source_identity_ref.clone(),
        source_revision_ref: artifact.source_revision_ref.clone(),
        jurisdiction_ref: artifact.jurisdiction_ref.clone(),
        official_document_id: artifact.official_document_id.clone(),
        locator: artifact.locator.clone(),
        in_force_on: demand.in_force_on.clone(),
        canonical_text_digest: artifact.canonical_text_digest.clone(),
        local_artifact_ref: artifact.local_artifact_ref.clone(),
        point_in_time_evidence: artifact.point_in_time_evidence.clone(),
        compile_eligible: true,
        parser_eligible: true,
        receipt_authority: NSW_LEGISLATION_RECEIPT_AUTHORITY,
    })
}

pub const fn acquisition_is_semantic_payment() -> bool {
    false
}

pub const fn acquisition_creates_legal_authority() -> bool {
    false
}

pub const fn current_consolidation_is_historical_proof() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::HttpResponse;
    use crate::{CitationTreatmentIntent, PropositionUseIntent};

    fn known() -> KnownAuthorityDemand {
        KnownAuthorityDemand {
            demand_ref: "demand:cullen:cla:s5b:2017".into(),
            jurisdiction_ref: "AU-NSW".into(),
            source_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
            medium_neutral_citation: None,
            explicit_austlii_ref: None,
            proposition_ref: Some("prop:NSW:CLA:s5B:definition".into()),
            use_intent: PropositionUseIntent::SourceProposition,
            treatment_intent: CitationTreatmentIntent::None,
        }
    }

    fn demand() -> HistoricalLegislationDemand {
        HistoricalLegislationDemand {
            demand_ref: "demand:cullen:cla:s5b:2017".into(),
            jurisdiction_ref: "AU-NSW".into(),
            act_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
            official_document_id: "act-2002-022".into(),
            requested_locator: "s 5B".into(),
            in_force_on: "2017-01-26".into(),
            proposition_ref: "prop:NSW:CLA:s5B:definition".into(),
            source_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
        }
    }

    fn artifact() -> ImmutableLegislationArtifact {
        ImmutableLegislationArtifact {
            provider: NSWLegislationProvider::OfficialNSWLegislation,
            act_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
            source_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
            source_revision_ref: "nsw-legislation:CLA-2002:2017-01-26:pit".into(),
            jurisdiction_ref: "AU-NSW".into(),
            official_document_id: "act-2002-022".into(),
            locator: "s 5B".into(),
            source_date_ref: "2017-01-26".into(),
            canonical_text_digest: "sha256:fixture".into(),
            local_artifact_ref: "artifacts/nsw/cla-2002/2017-01-26/s5B.txt".into(),
            immutable: true,
            point_in_time_evidence: HistoricalVersionEvidence {
                kind: HistoricalVersionEvidenceKind::OfficialPointInTimeVersion,
                evidence_reference: "https://legislation.nsw.gov.au/view/html/inforce/2017-01-26/act-2002-022/xml".into(),
                evidence_scope: "official point-in-time version requested for 2017-01-26".into(),
                evidence_digest: Some("sha256:evidence-fixture".into()),
            },
        }
    }

    #[test]
    fn official_pit_request_uses_documented_nsw_historical_xml_shape() {
        let request = official_point_in_time_xml_request(&demand()).unwrap();
        assert_eq!(
            request.url,
            "https://legislation.nsw.gov.au/view/html/inforce/2017-01-26/act-2002-022/xml"
        );
        assert_eq!(request.referer.as_deref(), Some(NSW_LEGISLATION_ORIGIN));
    }

    #[test]
    fn official_pit_response_is_fetch_candidate_not_retained_artifact() {
        let request = official_point_in_time_xml_request(&demand()).unwrap();
        let candidate = validate_official_point_in_time_response(
            &demand(),
            &request,
            HttpResponse {
                final_url: request.url.clone(),
                status_code: 200,
                content_type: Some("application/xml".into()),
                body: b"<legislation/>".to_vec(),
            },
        )
        .unwrap();
        assert!(!candidate.retained);
        assert_eq!(candidate.network_requests, 1);
        assert!(!acquisition_is_semantic_payment());
    }

    #[test]
    fn exact_historical_artifact_is_parser_eligible_but_non_semantic() {
        let bound = bind_known_authority_to_historical_legislation(&known(), demand()).unwrap();
        let receipt = admit_historical_legislation_artifact(&bound, &artifact()).unwrap();
        assert!(receipt.compile_eligible);
        assert!(receipt.parser_eligible);
        assert_eq!(receipt.in_force_on, "2017-01-26");
        assert!(!acquisition_is_semantic_payment());
        assert!(!acquisition_creates_legal_authority());
    }

    #[test]
    fn current_or_wrong_date_artifact_cannot_pay_historical_demand() {
        let mut current = artifact();
        current.source_date_ref = "2026-09-09".into();
        assert_eq!(
            admit_historical_legislation_artifact(&demand(), &current),
            Err(HistoricalLegislationError::HistoricalDateMismatch)
        );
        assert!(!current_consolidation_is_historical_proof());
    }

    #[test]
    fn mutable_or_unproven_artifact_is_rejected() {
        let mut candidate = artifact();
        candidate.immutable = false;
        assert_eq!(
            admit_historical_legislation_artifact(&demand(), &candidate),
            Err(HistoricalLegislationError::MutableArtifact)
        );
        candidate = artifact();
        candidate.point_in_time_evidence.evidence_reference.clear();
        assert_eq!(
            admit_historical_legislation_artifact(&demand(), &candidate),
            Err(HistoricalLegislationError::MissingPointInTimeEvidence)
        );
    }
}
