use crate::frontier::{ProofResidual, ResidualStatus};
use sensiblaw_core::canonical_evidence::{EvidenceObservation, EvidenceSpan, EvidenceSubstrateError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GetterBackend { SlrNative, LeanInterop, Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalStatus { Retrieved, Missing, Failed }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessStatus { Current, Stale, Unknown }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceClass {
    RevisionPinnedExternalSource,
    RevisionPinnedFixture,
    DigestPinnedExternalSource,
    OtherCandidateSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldObservation {
    pub request_ref: String,
    pub object_ref: String,
    pub relation_ref: String,
    pub source_ref: String,
    pub source_revision_ref: String,
    pub content_digest_ref: String,
    pub value_ref: String,
    pub retrieval_status: RetrievalStatus,
    pub freshness_status: FreshnessStatus,
    pub provenance_class: ProvenanceClass,
    pub backend: GetterBackend,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedWorldObservation {
    pub request_ref: String,
    pub object_ref: String,
    pub relation_ref: String,
    pub source_ref: String,
    pub source_revision_ref: String,
    pub content_digest_ref: String,
    pub value_ref: String,
    pub retrieval_status: RetrievalStatus,
    pub freshness_status: FreshnessStatus,
    pub provenance_class: ProvenanceClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldObservationError { EmptyCoordinate, MustRemainCandidateOnly, ObservationMayNotPromote }

impl WorldObservation {
    pub fn validate(&self) -> Result<(), WorldObservationError> {
        if [self.request_ref.as_str(), self.object_ref.as_str(), self.relation_ref.as_str(), self.source_ref.as_str(), self.source_revision_ref.as_str(), self.content_digest_ref.as_str(), self.value_ref.as_str()]
            .iter().any(|value| value.trim().is_empty()) {
            return Err(WorldObservationError::EmptyCoordinate);
        }
        if !self.candidate_only { return Err(WorldObservationError::MustRemainCandidateOnly); }
        if self.creates_semantic_authority || self.claim_truth_promoted { return Err(WorldObservationError::ObservationMayNotPromote); }
        Ok(())
    }

    pub fn canonical_evidence_observation(
        &self,
    ) -> Result<EvidenceObservation, EvidenceSubstrateError> {
        let span_ref = format!(
            "span:structured:{}:{}:{}",
            self.source_revision_ref, self.object_ref, self.relation_ref
        );
        let coordinate_ref = format!(
            "structured-coordinate:{}:{}:{}",
            self.object_ref, self.relation_ref, self.value_ref
        );
        let span = EvidenceSpan::structured(
            self.source_revision_ref.clone(),
            span_ref,
            coordinate_ref,
        )?;
        let observation = EvidenceObservation {
            observation_ref: format!(
                "observation:{}:{}:{}",
                self.request_ref, self.relation_ref, self.value_ref
            ),
            source_revision_ref: self.source_revision_ref.clone(),
            span,
            predicate_ref: self.relation_ref.clone(),
            value_ref: self.value_ref.clone(),
            candidate_only: self.candidate_only,
            creates_semantic_authority: self.creates_semantic_authority,
            applicability_promoted: false,
            claim_truth_promoted: self.claim_truth_promoted,
        };
        observation.validate()?;
        Ok(observation)
    }

    #[must_use]
    pub fn normalized(&self) -> NormalizedWorldObservation {
        NormalizedWorldObservation {
            request_ref: self.request_ref.clone(), object_ref: self.object_ref.clone(), relation_ref: self.relation_ref.clone(), source_ref: self.source_ref.clone(),
            source_revision_ref: self.source_revision_ref.clone(), content_digest_ref: self.content_digest_ref.clone(), value_ref: self.value_ref.clone(),
            retrieval_status: self.retrieval_status, freshness_status: self.freshness_status, provenance_class: self.provenance_class,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchKind {
    RequestMismatch, ObjectMismatch, RelationMismatch, SourceMismatch, RevisionMismatch,
    DigestMismatch, ValueMismatch, RetrievalMismatch, FreshnessMismatch, ProvenanceMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetterParityResidual {
    pub residual_ref: String,
    pub left_backend: GetterBackend,
    pub right_backend: GetterBackend,
    pub kind: MismatchKind,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationParity { Agreement, Residual(GetterParityResidual) }

fn mismatch_kind(left: &NormalizedWorldObservation, right: &NormalizedWorldObservation) -> Option<MismatchKind> {
    if left.request_ref != right.request_ref { return Some(MismatchKind::RequestMismatch); }
    if left.object_ref != right.object_ref { return Some(MismatchKind::ObjectMismatch); }
    if left.relation_ref != right.relation_ref { return Some(MismatchKind::RelationMismatch); }
    if left.source_ref != right.source_ref { return Some(MismatchKind::SourceMismatch); }
    if left.source_revision_ref != right.source_revision_ref { return Some(MismatchKind::RevisionMismatch); }
    if left.content_digest_ref != right.content_digest_ref { return Some(MismatchKind::DigestMismatch); }
    if left.value_ref != right.value_ref { return Some(MismatchKind::ValueMismatch); }
    if left.retrieval_status != right.retrieval_status { return Some(MismatchKind::RetrievalMismatch); }
    if left.freshness_status != right.freshness_status { return Some(MismatchKind::FreshnessMismatch); }
    if left.provenance_class != right.provenance_class { return Some(MismatchKind::ProvenanceMismatch); }
    None
}

#[must_use]
pub fn compare_observations(left: &WorldObservation, right: &WorldObservation) -> ObservationParity {
    let Some(kind) = mismatch_kind(&left.normalized(), &right.normalized()) else { return ObservationParity::Agreement; };
    ObservationParity::Residual(GetterParityResidual {
        residual_ref: format!("getter-parity:{}:{}:{:?}", left.request_ref, left.relation_ref, kind),
        left_backend: left.backend, right_backend: right.backend, kind,
        candidate_only: true, creates_semantic_authority: false, claim_truth_promoted: false,
    })
}

/// Convert an observation disagreement into the same open `ProofResidual`
/// surface consumed by the P7 frontier. Agreement creates no work. This is a
/// candidate diagnostic residual, never a truth judgment about either backend.
#[must_use]
pub fn getter_parity_to_proof_residual(
    parity: &ObservationParity,
    proposition_ref: impl Into<String>,
    jurisdiction_ref: Option<&str>,
    salience: u64,
) -> Option<ProofResidual> {
    let ObservationParity::Residual(residual) = parity else { return None; };
    Some(ProofResidual {
        residual_ref: residual.residual_ref.clone(),
        proposition_ref: proposition_ref.into(),
        producer_class_ref: "producer:getter-parity".into(),
        jurisdiction_ref: jurisdiction_ref.map(ToOwned::to_owned),
        authority_requirement_ref: None,
        salience,
        dependency_refs: vec![format!("backend:{:?}", residual.left_backend), format!("backend:{:?}", residual.right_backend)],
        status: ResidualStatus::Open,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mabo_observation(backend: GetterBackend, value_ref: &str) -> WorldObservation {
        WorldObservation {
            request_ref: "query:mabo:P710".into(), object_ref: "Q1501525".into(), relation_ref: "P710".into(), source_ref: "wikidata".into(),
            source_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(), content_digest_ref: "sha256:mabo-fixture".into(), value_ref: value_ref.into(),
            retrieval_status: RetrievalStatus::Retrieved, freshness_status: FreshnessStatus::Current,
            provenance_class: ProvenanceClass::RevisionPinnedExternalSource, backend,
            candidate_only: true, creates_semantic_authority: false, claim_truth_promoted: false,
        }
    }

    #[test]
    fn same_mabo_property_from_two_backends_agrees_after_normalization() {
        assert_eq!(compare_observations(&mabo_observation(GetterBackend::SlrNative, "Q975866"), &mabo_observation(GetterBackend::LeanInterop, "Q975866")), ObservationParity::Agreement);
    }

    #[test]
    fn backend_identity_does_not_change_observation_semantics() {
        let slr = mabo_observation(GetterBackend::SlrNative, "Q975866");
        let lean = mabo_observation(GetterBackend::LeanInterop, "Q975866");
        assert_ne!(slr.backend, lean.backend);
        assert_eq!(slr.normalized(), lean.normalized());
    }

    #[test]
    fn mismatched_value_becomes_parity_residual_not_truth_judgment() {
        let parity = compare_observations(&mabo_observation(GetterBackend::SlrNative, "Q975866"), &mabo_observation(GetterBackend::LeanInterop, "Q36074"));
        assert!(matches!(parity, ObservationParity::Residual(GetterParityResidual { kind: MismatchKind::ValueMismatch, .. })));
    }

    #[test]
    fn nat_climate_fixture_can_use_same_observation_abi() {
        let observation = WorldObservation {
            request_ref: "query:nat-climate:p5991-p14143".into(), object_ref: "Q10884".into(), relation_ref: "migration:P5991->P14143".into(),
            source_ref: "sensiblaw:nat-climate-fixture".into(), source_revision_ref: "provided_snapshot_2026-04-01".into(),
            content_digest_ref: "fixture-content-hash-not-pinned-in-this-owner".into(), value_ref: "review-required".into(),
            retrieval_status: RetrievalStatus::Retrieved, freshness_status: FreshnessStatus::Unknown,
            provenance_class: ProvenanceClass::RevisionPinnedFixture, backend: GetterBackend::SlrNative,
            candidate_only: true, creates_semantic_authority: false, claim_truth_promoted: false,
        };
        assert!(observation.validate().is_ok());
        assert!(!observation.creates_semantic_authority);
        assert!(!observation.claim_truth_promoted);
    }
    #[test]
    fn world_observation_lowers_to_structured_canonical_observation() {
        use sensiblaw_core::canonical_evidence::EvidenceSpanKind;
        let observation = mabo_observation(GetterBackend::SlrNative, "Q975866");
        let canonical = observation.canonical_evidence_observation().unwrap();
        assert_eq!(canonical.source_revision_ref, observation.source_revision_ref);
        assert_eq!(canonical.predicate_ref, "P710");
        assert_eq!(canonical.value_ref, "Q975866");
        assert!(matches!(
            canonical.span.kind,
            EvidenceSpanKind::StructuredCoordinate { .. }
        ));
        assert!(canonical.validate().is_ok());
    }

}
