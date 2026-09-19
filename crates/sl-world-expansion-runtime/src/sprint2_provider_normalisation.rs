//! Sprint-2 M2.4 provider normalisation.
//!
//! Existing provider-specific acquisition artifacts lower into one canonical
//! evidence carrier.  This module performs no acquisition and no semantic
//! promotion.  Review remains an explicit input and all reviewed evidence then
//! enters the already-paid SharedEvidenceReducer.

use sensiblaw_core::canonical_evidence::{
    EvidenceManifestation, EvidenceManifestationError, EvidenceObservation,
    EvidenceSourceRevision, EvidenceSpan, EvidenceSubstrateError,
};
use sensiblaw_governed_legal_provider::OalcLookupReceipt;
use sensiblaw_proof_search_loop::world_observation::GetterBackend;
use sensiblaw_proof_search_loop::world_observation_adapters::{
    oalc_evidence_manifestation, oalc_source_observation,
    wikidata_evidence_manifestation, wikidata_property_observation,
    wikipedia_article_observation, wikipedia_evidence_manifestation,
};
use sensiblaw_proof_search_loop::world_expansion_adapters::AcquiredWikidataEntity;
use sensiblaw_reviewed_evidence_payment::{
    reduce_reviewed_canonical_evidence, CanonicalEvidenceProjection,
    ReviewedCanonicalEvidence, ReviewedEvidenceCoordinate,
    SharedEvidenceReductionReceipt, SharedEvidenceReducerError,
};
use sensiblaw_route_executor::AcquiredSource;
use sensiblaw_route_selector::RouteCandidate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCanonicalEvidence {
    pub provider_ref: String,
    pub manifestation: EvidenceManifestation,
    pub revision: EvidenceSourceRevision,
    pub observation: EvidenceObservation,
}

#[derive(Debug)]
pub enum ProviderNormalisationError {
    Manifestation(EvidenceManifestationError),
    Substrate(EvidenceSubstrateError),
    Adapter(String),
    IdentityMismatch(&'static str),
    Reducer(SharedEvidenceReducerError),
}

impl std::fmt::Display for ProviderNormalisationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manifestation(error) => write!(f, "manifestation error: {error:?}"),
            Self::Substrate(error) => write!(f, "canonical evidence error: {error:?}"),
            Self::Adapter(error) => write!(f, "provider adapter error: {error}"),
            Self::IdentityMismatch(name) => write!(f, "provider identity mismatch: {name}"),
            Self::Reducer(error) => write!(f, "shared reducer error: {error:?}"),
        }
    }
}

impl std::error::Error for ProviderNormalisationError {}

impl From<EvidenceManifestationError> for ProviderNormalisationError {
    fn from(value: EvidenceManifestationError) -> Self {
        Self::Manifestation(value)
    }
}

impl From<EvidenceSubstrateError> for ProviderNormalisationError {
    fn from(value: EvidenceSubstrateError) -> Self {
        Self::Substrate(value)
    }
}

impl From<SharedEvidenceReducerError> for ProviderNormalisationError {
    fn from(value: SharedEvidenceReducerError) -> Self {
        Self::Reducer(value)
    }
}

impl ProviderCanonicalEvidence {
    pub fn validate(&self) -> Result<(), ProviderNormalisationError> {
        if self.provider_ref.trim().is_empty() {
            return Err(ProviderNormalisationError::IdentityMismatch("provider_ref"));
        }
        self.manifestation.validate()?;
        self.revision.validate()?;
        self.observation.validate()?;

        if self.revision.source_revision_ref != self.manifestation.source_revision_ref {
            return Err(ProviderNormalisationError::IdentityMismatch(
                "revision/source_revision_ref",
            ));
        }
        if self.revision.manifestation_ref != self.manifestation.manifestation_ref {
            return Err(ProviderNormalisationError::IdentityMismatch(
                "revision/manifestation_ref",
            ));
        }
        if self.revision.content_digest_ref != self.manifestation.content_digest_ref {
            return Err(ProviderNormalisationError::IdentityMismatch(
                "revision/content_digest_ref",
            ));
        }
        if self.observation.source_revision_ref != self.revision.source_revision_ref {
            return Err(ProviderNormalisationError::IdentityMismatch(
                "observation/source_revision_ref",
            ));
        }
        Ok(())
    }
}

fn assemble(
    provider_ref: impl Into<String>,
    manifestation: EvidenceManifestation,
    revision_receipt_ref: impl Into<String>,
    span: EvidenceSpan,
    observation_ref: impl Into<String>,
    predicate_ref: impl Into<String>,
    value_ref: impl Into<String>,
) -> Result<ProviderCanonicalEvidence, ProviderNormalisationError> {
    manifestation.validate()?;
    let revision =
        EvidenceSourceRevision::from_manifestation(&manifestation, revision_receipt_ref)?;
    if span.source_revision_ref != revision.source_revision_ref {
        return Err(ProviderNormalisationError::IdentityMismatch(
            "span/source_revision_ref",
        ));
    }

    let observation = EvidenceObservation {
        observation_ref: observation_ref.into(),
        source_revision_ref: revision.source_revision_ref.clone(),
        span,
        predicate_ref: predicate_ref.into(),
        value_ref: value_ref.into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    observation.validate()?;

    let evidence = ProviderCanonicalEvidence {
        provider_ref: provider_ref.into(),
        manifestation,
        revision,
        observation,
    };
    evidence.validate()?;
    Ok(evidence)
}

pub fn normalize_wikidata_provider(
    request_ref: impl Into<String>,
    source: &AcquiredWikidataEntity,
    route: &RouteCandidate,
    acquisition_receipt_ref: impl Into<String>,
    revision_receipt_ref: impl Into<String>,
) -> Result<ProviderCanonicalEvidence, ProviderNormalisationError> {
    let request_ref = request_ref.into();
    let world = wikidata_property_observation(
        request_ref.clone(),
        source,
        route,
        GetterBackend::SlrNative,
    )
    .map_err(|error| ProviderNormalisationError::Adapter(error.to_string()))?;
    let manifestation = wikidata_evidence_manifestation(source, acquisition_receipt_ref)?;
    let span = EvidenceSpan::structured(
        world.source_revision_ref.clone(),
        format!("span:{}:{}", world.object_ref, world.relation_ref),
        format!(
            "wikidata:{}:{}:{}",
            world.object_ref, world.relation_ref, world.value_ref
        ),
    )?;
    assemble(
        "provider:wikidata",
        manifestation,
        revision_receipt_ref,
        span,
        format!("observation:{request_ref}"),
        world.relation_ref,
        world.value_ref,
    )
}

pub fn normalize_wikipedia_provider(
    request_ref: impl Into<String>,
    source: &AcquiredSource,
    acquisition_receipt_ref: impl Into<String>,
    revision_receipt_ref: impl Into<String>,
) -> Result<ProviderCanonicalEvidence, ProviderNormalisationError> {
    let request_ref = request_ref.into();
    let world = wikipedia_article_observation(
        request_ref.clone(),
        source,
        GetterBackend::SlrNative,
    )
    .map_err(|error| ProviderNormalisationError::Adapter(error.to_string()))?;
    let manifestation = wikipedia_evidence_manifestation(source, acquisition_receipt_ref)?;
    let span = EvidenceSpan::whole_revision(
        world.source_revision_ref.clone(),
        format!("span:whole:{}", world.source_revision_ref),
    )?;
    assemble(
        "provider:wikipedia",
        manifestation,
        revision_receipt_ref,
        span,
        format!("observation:{request_ref}"),
        world.relation_ref,
        world.value_ref,
    )
}

pub fn normalize_oalc_provider(
    request_ref: impl Into<String>,
    receipt: &OalcLookupReceipt,
    acquisition_receipt_ref: impl Into<String>,
    revision_receipt_ref: impl Into<String>,
) -> Result<ProviderCanonicalEvidence, ProviderNormalisationError> {
    let request_ref = request_ref.into();
    let world = oalc_source_observation(
        request_ref.clone(),
        receipt,
        GetterBackend::SlrNative,
    )
    .map_err(|error| ProviderNormalisationError::Adapter(error.to_string()))?;
    let manifestation = oalc_evidence_manifestation(receipt, acquisition_receipt_ref)?;
    let span = EvidenceSpan::whole_revision(
        world.source_revision_ref.clone(),
        format!("span:whole:{}", world.source_revision_ref),
    )?;
    assemble(
        "provider:oalc",
        manifestation,
        revision_receipt_ref,
        span,
        format!("observation:{request_ref}"),
        world.relation_ref,
        world.value_ref,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn reduce_reviewed_provider_evidence(
    evidence: &ProviderCanonicalEvidence,
    review: &ReviewedEvidenceCoordinate,
    payment_ref: impl Into<String>,
    world: Option<&dyn CanonicalEvidenceProjection>,
    matter: Option<&dyn CanonicalEvidenceProjection>,
    legal: Option<&dyn CanonicalEvidenceProjection>,
) -> Result<SharedEvidenceReductionReceipt, ProviderNormalisationError> {
    evidence.validate()?;
    let reviewed = ReviewedCanonicalEvidence::from_reviewed_coordinate(
        review,
        evidence.observation.clone(),
        payment_ref,
    )?;
    Ok(reduce_reviewed_canonical_evidence(
        &reviewed, world, matter, legal,
    )?)
}
