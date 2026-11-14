use crate::{wikidata_reference_role, ReferenceRole, SourceUnit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalReferenceExecutionStatus {
    ExecutedWithOutput,
    ExecutedNoMatch,
    BlockedBeforeExecution,
    ProviderUnavailable,
    ExecutionFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundP854InspectionDemand {
    pub consumer_ref: String,
    pub exact_prerequisite_ref: String,
    pub requested_coordinate_ref: String,
    pub request_identity_ref: String,
    pub statement_ref: String,
    pub reference_property: String,
    pub reference_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundP854InspectionDemandError {
    EmptyRequiredField,
    NotP854,
    NotSourceCandidate,
}

impl BoundP854InspectionDemand {
    pub fn new(
        consumer_ref: impl Into<String>,
        exact_prerequisite_ref: impl Into<String>,
        requested_coordinate_ref: impl Into<String>,
        request_identity_ref: impl Into<String>,
        statement_ref: impl Into<String>,
        reference_property: impl Into<String>,
        reference_url: impl Into<String>,
    ) -> Result<Self, BoundP854InspectionDemandError> {
        let demand = Self {
            consumer_ref: consumer_ref.into(),
            exact_prerequisite_ref: exact_prerequisite_ref.into(),
            requested_coordinate_ref: requested_coordinate_ref.into(),
            request_identity_ref: request_identity_ref.into(),
            statement_ref: statement_ref.into(),
            reference_property: reference_property.into(),
            reference_url: reference_url.into(),
        };
        if [
            &demand.consumer_ref,
            &demand.exact_prerequisite_ref,
            &demand.requested_coordinate_ref,
            &demand.request_identity_ref,
            &demand.statement_ref,
            &demand.reference_property,
            &demand.reference_url,
        ]
        .iter()
        .any(|value| value.is_empty())
        {
            return Err(BoundP854InspectionDemandError::EmptyRequiredField);
        }
        if demand.reference_property != "P854" {
            return Err(BoundP854InspectionDemandError::NotP854);
        }
        if wikidata_reference_role(&demand.reference_property) != ReferenceRole::SourceCandidate {
            return Err(BoundP854InspectionDemandError::NotSourceCandidate);
        }
        Ok(demand)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalReferenceInspectionReceipt {
    pub demand: BoundP854InspectionDemand,
    pub execution_status: ExternalReferenceExecutionStatus,
    pub result_identity_ref: String,
    pub source_revision_ref: String,
    pub exact_locator_ref: String,
    pub content_identity_or_hash_ref: String,
    pub coverage_ref: String,
    pub uncertainty_ref: String,
    pub provenance_ref: String,
    pub raw_evidence_ref: String,
    pub referenced_content_inspected: bool,
    pub same_url_as_p854: bool,
    pub candidate_only: bool,
    pub creates_truth: bool,
    pub creates_source_support_payment: bool,
    pub creates_applicability: bool,
    pub creates_intervention_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalReferenceInspectionError {
    EmptyRequiredField,
    SourceUrlMissing,
    SourceUrlMismatch,
}

#[allow(clippy::too_many_arguments)]
pub fn inspected_p854_source_unit(
    demand: BoundP854InspectionDemand,
    source: &SourceUnit,
    result_identity_ref: impl Into<String>,
    exact_locator_ref: impl Into<String>,
    content_identity_or_hash_ref: impl Into<String>,
    coverage_ref: impl Into<String>,
    uncertainty_ref: impl Into<String>,
    provenance_ref: impl Into<String>,
) -> Result<ExternalReferenceInspectionReceipt, ExternalReferenceInspectionError> {
    let source_url = source
        .origin
        .source_url
        .as_ref()
        .ok_or(ExternalReferenceInspectionError::SourceUrlMissing)?;
    if source_url != &demand.reference_url {
        return Err(ExternalReferenceInspectionError::SourceUrlMismatch);
    }

    let result_identity_ref = result_identity_ref.into();
    let exact_locator_ref = exact_locator_ref.into();
    let content_identity_or_hash_ref = content_identity_or_hash_ref.into();
    let coverage_ref = coverage_ref.into();
    let uncertainty_ref = uncertainty_ref.into();
    let provenance_ref = provenance_ref.into();
    if [
        &result_identity_ref,
        &exact_locator_ref,
        &content_identity_or_hash_ref,
        &coverage_ref,
        &uncertainty_ref,
        &provenance_ref,
    ]
    .iter()
    .any(|value| value.is_empty())
    {
        return Err(ExternalReferenceInspectionError::EmptyRequiredField);
    }

    Ok(ExternalReferenceInspectionReceipt {
        demand,
        execution_status: ExternalReferenceExecutionStatus::ExecutedWithOutput,
        result_identity_ref,
        source_revision_ref: source.source_unit_id.clone(),
        exact_locator_ref,
        content_identity_or_hash_ref,
        coverage_ref,
        uncertainty_ref,
        provenance_ref,
        raw_evidence_ref: source.source_unit_id.clone(),
        referenced_content_inspected: true,
        same_url_as_p854: true,
        candidate_only: true,
        creates_truth: false,
        creates_source_support_payment: false,
        creates_applicability: false,
        creates_intervention_authority: false,
    })
}

pub fn no_match_p854_receipt(
    demand: BoundP854InspectionDemand,
    result_identity_ref: impl Into<String>,
    coverage_ref: impl Into<String>,
    provenance_ref: impl Into<String>,
) -> ExternalReferenceInspectionReceipt {
    ExternalReferenceInspectionReceipt {
        demand,
        execution_status: ExternalReferenceExecutionStatus::ExecutedNoMatch,
        result_identity_ref: result_identity_ref.into(),
        source_revision_ref: "none".into(),
        exact_locator_ref: "none".into(),
        content_identity_or_hash_ref: "none".into(),
        coverage_ref: coverage_ref.into(),
        uncertainty_ref: "bounded inspection returned no matching content; proposition remains unresolved".into(),
        provenance_ref: provenance_ref.into(),
        raw_evidence_ref: "none".into(),
        referenced_content_inspected: false,
        same_url_as_p854: true,
        candidate_only: true,
        creates_truth: false,
        creates_source_support_payment: false,
        creates_applicability: false,
        creates_intervention_authority: false,
    }
}

pub fn inspection_allows_semantic_assessment(receipt: &ExternalReferenceInspectionReceipt) -> bool {
    receipt.execution_status == ExternalReferenceExecutionStatus::ExecutedWithOutput
        && receipt.referenced_content_inspected
        && receipt.same_url_as_p854
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldAcquisitionProviderKind {
    WikimediaReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldAcquisitionCarrierKind {
    WikimediaReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldAcquisitionExecutionStatus {
    ExecutedWithOutput,
    ExecutedNoMatch,
    BlockedBeforeExecution,
    ProviderUnavailable,
    ExecutionFailed,
}

impl From<ExternalReferenceExecutionStatus> for WorldAcquisitionExecutionStatus {
    fn from(value: ExternalReferenceExecutionStatus) -> Self {
        match value {
            ExternalReferenceExecutionStatus::ExecutedWithOutput => Self::ExecutedWithOutput,
            ExternalReferenceExecutionStatus::ExecutedNoMatch => Self::ExecutedNoMatch,
            ExternalReferenceExecutionStatus::BlockedBeforeExecution => Self::BlockedBeforeExecution,
            ExternalReferenceExecutionStatus::ProviderUnavailable => Self::ProviderUnavailable,
            ExternalReferenceExecutionStatus::ExecutionFailed => Self::ExecutionFailed,
        }
    }
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

pub fn p854_inspection_to_world_envelope(
    receipt: &ExternalReferenceInspectionReceipt,
) -> RuntimeNeutralWorldAcquisitionEnvelope {
    RuntimeNeutralWorldAcquisitionEnvelope {
        consumer_ref: receipt.demand.consumer_ref.clone(),
        exact_prerequisite_ref: receipt.demand.exact_prerequisite_ref.clone(),
        requested_coordinate_ref: receipt.demand.requested_coordinate_ref.clone(),
        provider_kind: WorldAcquisitionProviderKind::WikimediaReference,
        carrier_kind: WorldAcquisitionCarrierKind::WikimediaReference,
        execution_status: receipt.execution_status.into(),
        request_identity_ref: receipt.demand.request_identity_ref.clone(),
        result_identity_ref: receipt.result_identity_ref.clone(),
        source_or_instrument_ref: receipt.demand.reference_url.clone(),
        source_revision_or_observation_time_ref: receipt.source_revision_ref.clone(),
        exact_locator_or_coordinate_ref: receipt.exact_locator_ref.clone(),
        content_identity_or_hash_ref: receipt.content_identity_or_hash_ref.clone(),
        coverage_ref: receipt.coverage_ref.clone(),
        uncertainty_ref: receipt.uncertainty_ref.clone(),
        provenance_ref: receipt.provenance_ref.clone(),
        acquisition_authority_ref: "experimental_candidate_only".into(),
        raw_evidence_ref: receipt.raw_evidence_ref.clone(),
        candidate_only: true,
        creates_truth: false,
        creates_applicability: false,
        creates_intervention_authority: false,
    }
}

pub fn world_envelope_is_source_support_payment(
    _envelope: &RuntimeNeutralWorldAcquisitionEnvelope,
) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentFormat, RetrievalMethod, RevisionId, SourceOrigin, SourceRevision, SourceType};

    fn external_source(url: &str) -> SourceUnit {
        SourceUnit::new(
            "external-reference:fixture".into(),
            "Q10884".into(),
            "unit:external-reference:fixture:rev1".into(),
            SourceRevision {
                revision_id: RevisionId::Text("rev1".into()),
                revision_timestamp: "2026-09-08T00:00:00Z".into(),
                retrieval_method: RetrievalMethod::HtmlSnapshot,
            },
            SourceOrigin {
                source_type: SourceType::Html,
                source_url: Some(url.into()),
                title: Some("External reference fixture".into()),
            },
            ContentFormat::Text,
            "bounded referenced content supporting later exact semantic review".into(),
            vec![],
            "metadata:external-reference-fixture".into(),
            "receipt:external-reference-fixture".into(),
        )
        .unwrap()
    }

    fn demand(url: &str) -> BoundP854InspectionDemand {
        BoundP854InspectionDemand::new(
            "consumer:nat-source-support",
            "missing:source-support",
            "coordinate:P854-external-content",
            "request:nat:p854:fixture",
            "statement:Q10884:P5991:fixture",
            "P854",
            url,
        )
        .unwrap()
    }

    #[test]
    fn p854_is_bound_to_exact_live_source_support_coordinate() {
        let d = demand("https://example.org/source");
        assert_eq!(d.reference_property, "P854");
        assert_eq!(d.exact_prerequisite_ref, "missing:source-support");
        assert_eq!(d.requested_coordinate_ref, "coordinate:P854-external-content");
    }

    #[test]
    fn inspected_same_url_can_enter_assessment_but_does_not_pay() {
        let url = "https://example.org/source";
        let source = external_source(url);
        let receipt = inspected_p854_source_unit(
            demand(url),
            &source,
            "result:p854:fixture",
            "anchor:exact-claim-span",
            "sha256:fixture-content",
            "coverage:bounded-source-document",
            "uncertainty:semantic-correspondence-still-required",
            "provenance:p854-to-source-unit",
        )
        .unwrap();
        assert!(inspection_allows_semantic_assessment(&receipt));
        assert!(!receipt.creates_source_support_payment);
        let envelope = p854_inspection_to_world_envelope(&receipt);
        assert_eq!(envelope.provider_kind, WorldAcquisitionProviderKind::WikimediaReference);
        assert_eq!(envelope.execution_status, WorldAcquisitionExecutionStatus::ExecutedWithOutput);
        assert!(envelope.candidate_only);
        assert!(!envelope.creates_truth);
        assert!(!world_envelope_is_source_support_payment(&envelope));
    }

    #[test]
    fn p854_url_mismatch_fails_closed() {
        let source = external_source("https://example.org/other");
        let result = inspected_p854_source_unit(
            demand("https://example.org/source"),
            &source,
            "result:p854:mismatch",
            "anchor:x",
            "sha256:x",
            "coverage:x",
            "uncertainty:x",
            "provenance:x",
        );
        assert_eq!(result, Err(ExternalReferenceInspectionError::SourceUrlMismatch));
    }

    #[test]
    fn no_match_is_not_negative_fact_or_semantic_assessment() {
        let receipt = no_match_p854_receipt(
            demand("https://example.org/source"),
            "result:p854:no-match",
            "coverage:bounded-fetch",
            "provenance:bounded-fetch",
        );
        assert!(!inspection_allows_semantic_assessment(&receipt));
        assert!(!receipt.creates_truth);
        assert!(!receipt.creates_source_support_payment);
        let envelope = p854_inspection_to_world_envelope(&receipt);
        assert_eq!(envelope.execution_status, WorldAcquisitionExecutionStatus::ExecutedNoMatch);
        assert!(!world_envelope_is_source_support_payment(&envelope));
    }
}
