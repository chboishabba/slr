use crate::acquisition::AcquiredAuthorityHandoff;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldAcquisitionProviderKind {
    DocumentSource,
    SensorObservation,
    MarketData,
    WikimediaReference,
    CommunitySource,
    LocalArchive,
    RuntimeGenerated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldAcquisitionCarrierKind {
    AttributedDocument,
    CalibratedSensor,
    MarketEvent,
    WikimediaReference,
    CommunityKnowledge,
    ArchiveContent,
    RuntimeReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldAcquisitionExecutionStatus {
    ExecutedWithOutput,
    ExecutedNoMatch,
    BlockedBeforeExecution,
    ProviderUnavailable,
    ExecutionFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeNeutralWorldAcquisitionEnvelope {
    pub consumer_ref: String,
    pub exact_prerequisite_ref: String,
    pub requested_coordinate_ref: String,
    pub provider_kind: WorldAcquisitionProviderKind,
    pub carrier_kind: WorldAcquisitionCarrierKind,
    pub execution_status: WorldAcquisitionExecutionStatus,
    pub request_identity_ref: String,
    pub result_identity_ref: String,
    pub source_or_instrument_ref: String,
    pub source_revision_or_observation_time_ref: String,
    pub exact_locator_or_coordinate_ref: String,
    pub content_identity_or_hash_ref: String,
    pub coverage_ref: String,
    pub uncertainty_ref: String,
    pub provenance_ref: String,
    pub acquisition_authority_ref: String,
    pub raw_evidence_ref: String,
    pub candidate_only: bool,
    pub creates_truth: bool,
    pub creates_applicability: bool,
    pub creates_intervention_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldAcquisitionEnvelopeError {
    NonCandidateInput,
}

#[allow(clippy::too_many_arguments)]
pub fn from_acquired_legal_authority(
    handoff: &AcquiredAuthorityHandoff,
    consumer_ref: impl Into<String>,
    exact_prerequisite_ref: impl Into<String>,
    requested_coordinate_ref: impl Into<String>,
    request_identity_ref: impl Into<String>,
    exact_locator_ref: impl Into<String>,
    coverage_ref: impl Into<String>,
    uncertainty_ref: impl Into<String>,
    provenance_ref: impl Into<String>,
) -> Result<RuntimeNeutralWorldAcquisitionEnvelope, WorldAcquisitionEnvelopeError> {
    if !handoff.candidate_only {
        return Err(WorldAcquisitionEnvelopeError::NonCandidateInput);
    }

    Ok(RuntimeNeutralWorldAcquisitionEnvelope {
        consumer_ref: consumer_ref.into(),
        exact_prerequisite_ref: exact_prerequisite_ref.into(),
        requested_coordinate_ref: requested_coordinate_ref.into(),
        provider_kind: WorldAcquisitionProviderKind::DocumentSource,
        carrier_kind: WorldAcquisitionCarrierKind::AttributedDocument,
        execution_status: WorldAcquisitionExecutionStatus::ExecutedWithOutput,
        request_identity_ref: request_identity_ref.into(),
        result_identity_ref: handoff.provider_receipt_ref.clone(),
        source_or_instrument_ref: handoff.source_identity_ref.clone(),
        source_revision_or_observation_time_ref: handoff.source_revision_ref.clone(),
        exact_locator_or_coordinate_ref: exact_locator_ref.into(),
        content_identity_or_hash_ref: handoff.provider_receipt_ref.clone(),
        coverage_ref: coverage_ref.into(),
        uncertainty_ref: uncertainty_ref.into(),
        provenance_ref: provenance_ref.into(),
        acquisition_authority_ref: "experimental_candidate_only".into(),
        raw_evidence_ref: handoff.source_revision_ref.clone(),
        candidate_only: true,
        creates_truth: false,
        creates_applicability: false,
        creates_intervention_authority: false,
    })
}

pub fn execution_allows_semantic_assessment(
    envelope: &RuntimeNeutralWorldAcquisitionEnvelope,
) -> bool {
    envelope.execution_status == WorldAcquisitionExecutionStatus::ExecutedWithOutput
}

pub fn world_acquisition_envelope_is_semantic_payment(
    _envelope: &RuntimeNeutralWorldAcquisitionEnvelope,
) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_handoff_projects_to_candidate_only_world_envelope() {
        let handoff = AcquiredAuthorityHandoff {
            source_revision_ref: "source:hca:2026:19:rev:fixture".into(),
            source_identity_ref: "case:[2026]-HCA-19".into(),
            provider_receipt_ref: "provider:hca:sha256:bytes".into(),
            network_requests_used_to_acquire: 1,
            candidate_only: true,
        };

        let envelope = from_acquired_legal_authority(
            &handoff,
            "consumer:legal-accountability",
            "missing:exact-primary-source-support",
            "coordinate:source-span",
            "request:hca-19",
            "paragraphs:exact-locator-still-reviewed-downstream",
            "coverage:bounded-case-document",
            "uncertainty:semantic-correspondence-open",
            "provenance:official-source-to-local-revision",
        )
        .unwrap();

        assert_eq!(
            envelope.provider_kind,
            WorldAcquisitionProviderKind::DocumentSource
        );
        assert_eq!(
            envelope.carrier_kind,
            WorldAcquisitionCarrierKind::AttributedDocument
        );
        assert!(execution_allows_semantic_assessment(&envelope));
        assert!(envelope.candidate_only);
        assert!(!envelope.creates_truth);
        assert!(!envelope.creates_applicability);
        assert!(!envelope.creates_intervention_authority);
        assert!(!world_acquisition_envelope_is_semantic_payment(&envelope));
    }

    #[test]
    fn no_match_does_not_enter_semantic_assessment() {
        let envelope = RuntimeNeutralWorldAcquisitionEnvelope {
            consumer_ref: "consumer:market".into(),
            exact_prerequisite_ref: "missing:identity".into(),
            requested_coordinate_ref: "coordinate:public-wallet-linkage".into(),
            provider_kind: WorldAcquisitionProviderKind::MarketData,
            carrier_kind: WorldAcquisitionCarrierKind::MarketEvent,
            execution_status: WorldAcquisitionExecutionStatus::ExecutedNoMatch,
            request_identity_ref: "request:market:no-match".into(),
            result_identity_ref: "result:market:no-match".into(),
            source_or_instrument_ref: "provider:public-market".into(),
            source_revision_or_observation_time_ref: "time:fixture".into(),
            exact_locator_or_coordinate_ref: "query:fixture".into(),
            content_identity_or_hash_ref: "none".into(),
            coverage_ref: "coverage:bounded-query".into(),
            uncertainty_ref: "identity remains unresolved".into(),
            provenance_ref: "provider:no-match-receipt".into(),
            acquisition_authority_ref: "experimental_candidate_only".into(),
            raw_evidence_ref: "none".into(),
            candidate_only: true,
            creates_truth: false,
            creates_applicability: false,
            creates_intervention_authority: false,
        };

        assert!(!execution_allows_semantic_assessment(&envelope));
        assert!(!world_acquisition_envelope_is_semantic_payment(&envelope));
    }
}
