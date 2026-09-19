use crate::world_expansion_adapters::{AcquiredWikidataEntity, ExpansionAdapterError};
use crate::world_observation::{
    FreshnessStatus, GetterBackend, ProvenanceClass, RetrievalStatus, WorldObservation,
};
use sensiblaw_core::canonical_evidence::{
    EvidenceManifestation, EvidenceManifestationError, EvidenceManifestationFamily,
};
use sensiblaw_governed_legal_provider::OalcLookupReceipt;
use sensiblaw_route_executor::AcquiredSource;
use sensiblaw_route_selector::{ProducerFamily, RouteCandidate, RouteFamily};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldObservationAdapterError {
    ExpansionAdapter(ExpansionAdapterError),
    OalcReceiptNotCandidateOnly,
    WikipediaSourceNotCandidateOnly,
    WikipediaSourcePromoted,
    WikidataSourceMismatch,
    WrongWikidataRoute,
    WrongWikidataProducer,
    WikidataSourceNotCandidateOnly,
    WikidataSourcePromoted,
}

impl std::fmt::Display for WorldObservationAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExpansionAdapter(err) => write!(f, "expansion adapter: {err}"),
            other => write!(f, "{other:?}"),
        }
    }
}

impl std::error::Error for WorldObservationAdapterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ExpansionAdapter(err) => Some(err),
            _ => None,
        }
    }
}

const fn supported_wikidata_property_producer(producer: ProducerFamily) -> bool {
    matches!(
        producer,
        ProducerFamily::IdentitySource
            | ProducerFamily::AuthoritySource
            | ProducerFamily::ClassificationEvidence
    )
}

fn hex_digest(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(71);
    out.push_str("sha256:");
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

pub fn wikidata_property_observation(
    request_ref: impl Into<String>,
    source: &AcquiredWikidataEntity,
    route: &RouteCandidate,
    backend: GetterBackend,
) -> Result<WorldObservation, WorldObservationAdapterError> {
    if route.route_family != RouteFamily::WikidataProperty {
        return Err(WorldObservationAdapterError::WrongWikidataRoute);
    }
    if !supported_wikidata_property_producer(route.producer) {
        return Err(WorldObservationAdapterError::WrongWikidataProducer);
    }
    if !source.candidate_only {
        return Err(WorldObservationAdapterError::WikidataSourceNotCandidateOnly);
    }
    if source.semantic_promotion {
        return Err(WorldObservationAdapterError::WikidataSourcePromoted);
    }
    if source.qid != route.source_ref {
        return Err(WorldObservationAdapterError::WikidataSourceMismatch);
    }
    Ok(WorldObservation {
        request_ref: request_ref.into(),
        object_ref: source.qid.clone(),
        relation_ref: route.property_ref.clone(),
        source_ref: "wikidata".into(),
        source_revision_ref: source.source_revision_ref.clone(),
        content_digest_ref: source.content_digest_ref.clone(),
        value_ref: route.target_ref.clone(),
        retrieval_status: RetrievalStatus::Retrieved,
        freshness_status: FreshnessStatus::Current,
        provenance_class: ProvenanceClass::RevisionPinnedExternalSource,
        backend,
        candidate_only: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}

pub fn wikipedia_article_observation(
    request_ref: impl Into<String>,
    source: &AcquiredSource,
    backend: GetterBackend,
) -> Result<WorldObservation, WorldObservationAdapterError> {
    if !source.candidate_only {
        return Err(WorldObservationAdapterError::WikipediaSourceNotCandidateOnly);
    }
    if source.semantic_promotion {
        return Err(WorldObservationAdapterError::WikipediaSourcePromoted);
    }
    Ok(WorldObservation {
        request_ref: request_ref.into(),
        object_ref: source.canonical_url.clone(),
        relation_ref: "wikipedia:article-content".into(),
        source_ref: source.source_ref.clone(),
        source_revision_ref: source.revision_ref.clone(),
        content_digest_ref: hex_digest(&source.source_sha256),
        value_ref: source.document_ref.clone(),
        retrieval_status: RetrievalStatus::Retrieved,
        freshness_status: FreshnessStatus::Unknown,
        provenance_class: ProvenanceClass::DigestPinnedExternalSource,
        backend,
        candidate_only: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}

pub fn oalc_source_observation(
    request_ref: impl Into<String>,
    receipt: &OalcLookupReceipt,
    backend: GetterBackend,
) -> Result<WorldObservation, WorldObservationAdapterError> {
    if receipt.receipt_authority != "experimental_candidate_only" {
        return Err(WorldObservationAdapterError::OalcReceiptNotCandidateOnly);
    }
    Ok(WorldObservation {
        request_ref: request_ref.into(),
        object_ref: receipt.source_identity_ref.clone(),
        relation_ref: "legal:source-manifestation".into(),
        source_ref: "oalc".into(),
        source_revision_ref: receipt.source_revision_ref.clone(),
        content_digest_ref: receipt.canonical_text_digest.clone(),
        value_ref: receipt.citation.clone(),
        retrieval_status: RetrievalStatus::Retrieved,
        freshness_status: FreshnessStatus::Unknown,
        provenance_class: ProvenanceClass::DigestPinnedExternalSource,
        backend,
        candidate_only: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}


fn validated_manifestation(
    manifestation: EvidenceManifestation,
) -> Result<EvidenceManifestation, EvidenceManifestationError> {
    manifestation.validate()?;
    Ok(manifestation)
}

pub fn wikidata_evidence_manifestation(
    source: &AcquiredWikidataEntity,
    acquisition_receipt_ref: impl Into<String>,
) -> Result<EvidenceManifestation, EvidenceManifestationError> {
    validated_manifestation(EvidenceManifestation {
        manifestation_ref: format!("manifestation:wikidata:{}", source.source_revision_ref),
        family: EvidenceManifestationFamily::Wikidata,
        source_ref: format!("wikidata:{}", source.qid),
        source_revision_ref: source.source_revision_ref.clone(),
        content_digest_ref: source.content_digest_ref.clone(),
        acquisition_receipt_ref: acquisition_receipt_ref.into(),
        candidate_only: source.candidate_only,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: source.semantic_promotion,
    })
}

pub fn wikipedia_evidence_manifestation(
    source: &AcquiredSource,
    acquisition_receipt_ref: impl Into<String>,
) -> Result<EvidenceManifestation, EvidenceManifestationError> {
    validated_manifestation(EvidenceManifestation {
        manifestation_ref: format!(
            "manifestation:wikipedia:{}:{}",
            source.revision_ref, source.document_ref
        ),
        family: EvidenceManifestationFamily::Wikipedia,
        source_ref: source.source_ref.clone(),
        source_revision_ref: source.revision_ref.clone(),
        content_digest_ref: hex_digest(&source.source_sha256),
        acquisition_receipt_ref: acquisition_receipt_ref.into(),
        candidate_only: source.candidate_only,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: source.semantic_promotion,
    })
}

pub fn oalc_evidence_manifestation(
    receipt: &OalcLookupReceipt,
    acquisition_receipt_ref: impl Into<String>,
) -> Result<EvidenceManifestation, EvidenceManifestationError> {
    validated_manifestation(EvidenceManifestation {
        manifestation_ref: format!("manifestation:oalc:{}", receipt.source_revision_ref),
        family: EvidenceManifestationFamily::Oalc,
        source_ref: receipt.source_identity_ref.clone(),
        source_revision_ref: receipt.source_revision_ref.clone(),
        content_digest_ref: receipt.canonical_text_digest.clone(),
        acquisition_receipt_ref: acquisition_receipt_ref.into(),
        candidate_only: receipt.receipt_authority == "experimental_candidate_only",
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
