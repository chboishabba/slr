//! Governed NSW point-in-time legislation adapter for SensibLaw.
//!
//! This is a source-specific leaf over `sensiblaw-governed-legal-provider`.
//! The canonical provider crate still owns governance, HTTP transport, and the
//! provider-neutral `KnownAuthorityDemand`. This crate adds only the missing
//! historical-legislation coordinates and immutable-retention gate.
//!
//! Chain:
//! KnownAuthorityDemand -> HistoricalLegislationDemand -> governed one-request
//! PIT fetch -> immutable retained revision -> parser handoff.
//!
//! None of those steps is semantic/legal payment.

use std::fs;
use std::path::{Path, PathBuf};

use sensiblaw_governed_legal_provider::{
    GovernanceError, GovernedExecutionContext, HttpRequest, HttpResponse, HttpTransport,
    KnownAuthorityDemand,
};
use sha2::{Digest, Sha256};

pub const PROVIDER_ID: &str = "provider:official-nsw-legislation";
pub const RECEIPT_AUTHORITY: &str = "experimental_candidate_only";
pub const PARSER_AUTHORITY: &str = "source_observation_only";
pub const NSW_ORIGIN: &str = "https://legislation.nsw.gov.au";
pub const NSW_UA: &str = "SensibLaw/0.1 governed-nsw-legislation-provider";

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
    /// Stable NSW legislation identifier, e.g. `act-2002-022`.
    pub official_document_id: String,
    pub requested_locator: String,
    /// Date on which the legal consumer requires the source to have been in force.
    pub in_force_on: String,
    /// Optional independently sourced effective-from date of the selected version.
    pub expected_version_effective_from: Option<String>,
    pub proposition_ref: String,
    pub source_identity_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalVersionEvidence {
    pub kind: HistoricalVersionEvidenceKind,
    pub evidence_reference: String,
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
    pub act_identity_ref: String,
    pub source_identity_ref: String,
    pub source_revision_ref: String,
    pub jurisdiction_ref: String,
    pub official_document_id: String,
    pub locator: String,
    pub in_force_on: String,
    pub version_effective_from: Option<String>,
    pub canonical_bytes_digest: String,
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
    pub version_effective_from: Option<String>,
    pub canonical_bytes_digest: String,
    pub local_artifact_ref: String,
    pub point_in_time_evidence: HistoricalVersionEvidence,
    pub compile_eligible: bool,
    pub parser_eligible: bool,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalLegislationParserHandoff {
    pub source_identity_ref: String,
    pub source_revision_ref: String,
    pub jurisdiction_ref: String,
    pub official_document_id: String,
    pub locator: String,
    pub in_force_on: String,
    pub version_effective_from: Option<String>,
    pub canonical_bytes_digest: String,
    pub local_artifact_ref: String,
    pub parser_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoricalLegislationError {
    WrongJurisdiction,
    DemandSourceIdentityMismatch,
    DemandPropositionMissing,
    DemandPropositionMismatch,
    InvalidHistoricalDate,
    InvalidEffectiveDate,
    InvalidDocumentId,
    InvalidOfficialReference,
    UnexpectedHttpStatus,
    UnexpectedContentType,
    EmptyOfficialResponse,
    Transport(String),
    Governance(String),
    Retention(String),
    ActIdentityMismatch,
    SourceIdentityMismatch,
    DocumentIdMismatch,
    LocatorMismatch,
    HistoricalDateMismatch,
    EffectiveDateMismatch,
    MutableArtifact,
    MissingRevision,
    MissingDigest,
    MissingLocalArtifact,
    MissingPointInTimeEvidence,
}

fn iso_date(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
}

fn document_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && (value.starts_with("act-") || value.starts_with("epi-") || value.starts_with("sl-"))
}

impl HistoricalLegislationDemand {
    pub fn validate(&self) -> Result<(), HistoricalLegislationError> {
        if self.jurisdiction_ref != "AU-NSW" {
            return Err(HistoricalLegislationError::WrongJurisdiction);
        }
        if !iso_date(&self.in_force_on) {
            return Err(HistoricalLegislationError::InvalidHistoricalDate);
        }
        if self
            .expected_version_effective_from
            .as_deref()
            .is_some_and(|date| !iso_date(date))
        {
            return Err(HistoricalLegislationError::InvalidEffectiveDate);
        }
        if !document_id(&self.official_document_id) {
            return Err(HistoricalLegislationError::InvalidDocumentId);
        }
        Ok(())
    }
}

/// Exact official NSW point-in-time XML reference documented for historical
/// versions. The requested date and the selected version's effective-from date
/// remain separate coordinates.
pub fn official_pit_xml_reference(
    demand: &HistoricalLegislationDemand,
) -> Result<String, HistoricalLegislationError> {
    demand.validate()?;
    Ok(format!(
        "{NSW_ORIGIN}/view/html/inforce/{}/{}/xml",
        demand.in_force_on, demand.official_document_id
    ))
}

pub fn official_pit_xml_request(
    demand: &HistoricalLegislationDemand,
) -> Result<HttpRequest, HistoricalLegislationError> {
    Ok(HttpRequest {
        url: official_pit_xml_reference(demand)?,
        user_agent: NSW_UA.into(),
        referer: Some(NSW_ORIGIN.into()),
        timeout_seconds: 45,
    })
}

fn official_reference(reference: &str) -> bool {
    reference == NSW_ORIGIN || reference.starts_with("https://legislation.nsw.gov.au/")
}

pub fn validate_official_pit_response(
    demand: &HistoricalLegislationDemand,
    request: &HttpRequest,
    response: HttpResponse,
) -> Result<OfficialPointInTimeFetchCandidate, HistoricalLegislationError> {
    if request.url != official_pit_xml_reference(demand)? || !official_reference(&response.final_url)
    {
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
        receipt_authority: RECEIPT_AUTHORITY,
    })
}

/// One governed request. `GovernedExecutionContext` remains the canonical policy
/// owner; this leaf cannot bypass operator opt-in/cache-first/persisted-first.
pub fn fetch_official_pit_xml<T: HttpTransport>(
    transport: &mut T,
    context: &GovernedExecutionContext,
    demand: &HistoricalLegislationDemand,
) -> Result<OfficialPointInTimeFetchCandidate, HistoricalLegislationError>
where
    T::Error: ToString,
{
    context
        .validate()
        .map_err(|err: GovernanceError| HistoricalLegislationError::Governance(format!("{err:?}")))?;
    let request = official_pit_xml_request(demand)?;
    let response = transport
        .get(&request)
        .map_err(|err| HistoricalLegislationError::Transport(err.to_string()))?;
    validate_official_pit_response(demand, &request, response)
}

pub fn bind_known_authority_demand(
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

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Retain exact official bytes and make the file read-only. This still does not
/// establish legal meaning; it only creates an immutable source-revision object.
pub fn retain_official_fetch_candidate(
    demand: &HistoricalLegislationDemand,
    candidate: &OfficialPointInTimeFetchCandidate,
    destination: impl AsRef<Path>,
) -> Result<ImmutableLegislationArtifact, HistoricalLegislationError> {
    if candidate.demand_ref != demand.demand_ref
        || candidate.official_document_id != demand.official_document_id
        || candidate.requested_date != demand.in_force_on
        || candidate.request_reference != official_pit_xml_reference(demand)?
    {
        return Err(HistoricalLegislationError::InvalidOfficialReference);
    }
    let destination: PathBuf = destination.as_ref().to_owned();
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| HistoricalLegislationError::Retention(err.to_string()))?;
    }
    fs::write(&destination, &candidate.bytes)
        .map_err(|err| HistoricalLegislationError::Retention(err.to_string()))?;
    let mut permissions = fs::metadata(&destination)
        .map_err(|err| HistoricalLegislationError::Retention(err.to_string()))?
        .permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&destination, permissions)
        .map_err(|err| HistoricalLegislationError::Retention(err.to_string()))?;

    let digest = sha256(&candidate.bytes);
    Ok(ImmutableLegislationArtifact {
        act_identity_ref: demand.act_identity_ref.clone(),
        source_identity_ref: demand.source_identity_ref.clone(),
        source_revision_ref: format!(
            "nsw-legislation:{}:{}:{}",
            demand.official_document_id, demand.in_force_on, digest
        ),
        jurisdiction_ref: demand.jurisdiction_ref.clone(),
        official_document_id: demand.official_document_id.clone(),
        locator: demand.requested_locator.clone(),
        in_force_on: demand.in_force_on.clone(),
        version_effective_from: demand.expected_version_effective_from.clone(),
        canonical_bytes_digest: digest,
        local_artifact_ref: destination.to_string_lossy().into_owned(),
        immutable: true,
        point_in_time_evidence: HistoricalVersionEvidence {
            kind: HistoricalVersionEvidenceKind::OfficialPointInTimeVersion,
            evidence_reference: candidate.final_reference.clone(),
            evidence_scope: format!(
                "official NSW point-in-time XML requested for {}",
                demand.in_force_on
            ),
            evidence_digest: None,
        },
    })
}

pub fn admit_historical_artifact(
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
    if artifact.official_document_id != demand.official_document_id {
        return Err(HistoricalLegislationError::DocumentIdMismatch);
    }
    if artifact.locator != demand.requested_locator {
        return Err(HistoricalLegislationError::LocatorMismatch);
    }
    if artifact.in_force_on != demand.in_force_on {
        return Err(HistoricalLegislationError::HistoricalDateMismatch);
    }
    if artifact.version_effective_from != demand.expected_version_effective_from {
        return Err(HistoricalLegislationError::EffectiveDateMismatch);
    }
    if !artifact.immutable {
        return Err(HistoricalLegislationError::MutableArtifact);
    }
    if artifact.source_revision_ref.trim().is_empty() {
        return Err(HistoricalLegislationError::MissingRevision);
    }
    if artifact.canonical_bytes_digest.trim().is_empty() {
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
        act_identity_ref: artifact.act_identity_ref.clone(),
        source_identity_ref: artifact.source_identity_ref.clone(),
        source_revision_ref: artifact.source_revision_ref.clone(),
        jurisdiction_ref: artifact.jurisdiction_ref.clone(),
        official_document_id: artifact.official_document_id.clone(),
        locator: artifact.locator.clone(),
        in_force_on: artifact.in_force_on.clone(),
        version_effective_from: artifact.version_effective_from.clone(),
        canonical_bytes_digest: artifact.canonical_bytes_digest.clone(),
        local_artifact_ref: artifact.local_artifact_ref.clone(),
        point_in_time_evidence: artifact.point_in_time_evidence.clone(),
        compile_eligible: true,
        parser_eligible: true,
        receipt_authority: RECEIPT_AUTHORITY,
    })
}

pub fn parser_handoff(
    receipt: &HistoricalLegislationReceipt,
) -> Option<HistoricalLegislationParserHandoff> {
    (receipt.compile_eligible && receipt.parser_eligible).then(|| HistoricalLegislationParserHandoff {
        source_identity_ref: receipt.source_identity_ref.clone(),
        source_revision_ref: receipt.source_revision_ref.clone(),
        jurisdiction_ref: receipt.jurisdiction_ref.clone(),
        official_document_id: receipt.official_document_id.clone(),
        locator: receipt.locator.clone(),
        in_force_on: receipt.in_force_on.clone(),
        version_effective_from: receipt.version_effective_from.clone(),
        canonical_bytes_digest: receipt.canonical_bytes_digest.clone(),
        local_artifact_ref: receipt.local_artifact_ref.clone(),
        parser_authority: PARSER_AUTHORITY,
    })
}

pub const fn current_consolidation_is_historical_proof() -> bool { false }
pub const fn acquisition_is_semantic_payment() -> bool { false }
pub const fn acquisition_creates_legal_authority() -> bool { false }
pub const fn parser_handoff_creates_atomic_gate() -> bool { false }

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_governed_legal_provider::{
        CitationTreatmentIntent, LiveGovernanceBounds, PropositionUseIntent,
    };

    fn demand() -> HistoricalLegislationDemand {
        HistoricalLegislationDemand {
            demand_ref: "demand:cullen:cla:s5b:2017".into(),
            jurisdiction_ref: "AU-NSW".into(),
            act_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
            official_document_id: "act-2002-022".into(),
            requested_locator: "s 5B".into(),
            in_force_on: "2017-01-26".into(),
            expected_version_effective_from: Some("2015-07-01".into()),
            proposition_ref: "prop:NSW:CLA:s5B:definition".into(),
            source_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
        }
    }

    #[test]
    fn exact_pit_reference_is_revision_pinned() {
        assert_eq!(
            official_pit_xml_reference(&demand()).unwrap(),
            "https://legislation.nsw.gov.au/view/html/inforce/2017-01-26/act-2002-022/xml"
        );
    }

    #[test]
    fn provider_neutral_demand_must_weld_to_same_proposition_and_source() {
        let known = KnownAuthorityDemand {
            demand_ref: "demand:cullen:cla:s5b:2017".into(),
            jurisdiction_ref: "AU-NSW".into(),
            source_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
            medium_neutral_citation: None,
            explicit_austlii_ref: None,
            proposition_ref: Some("prop:NSW:CLA:s5B:definition".into()),
            use_intent: PropositionUseIntent::SourceProposition,
            treatment_intent: CitationTreatmentIntent::None,
        };
        assert!(bind_known_authority_demand(&known, demand()).is_ok());
    }

    #[test]
    fn governance_must_be_paid_before_transport() {
        let context = GovernedExecutionContext {
            operator_opt_in: false,
            cache_checked_first: true,
            persisted_receipts_checked_first: true,
            bounds: LiveGovernanceBounds::HISTORICAL_DEFAULT,
        };
        assert!(context.validate().is_err());
    }

    #[test]
    fn current_consolidation_never_counts_as_historical_proof_by_default() {
        assert!(!current_consolidation_is_historical_proof());
        assert!(!acquisition_is_semantic_payment());
        assert!(!acquisition_creates_legal_authority());
        assert!(!parser_handoff_creates_atomic_gate());
    }
}
