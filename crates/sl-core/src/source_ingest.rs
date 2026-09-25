//! INGEST-1 generic source compilation boundary.
//!
//! Provider-specific source structure is preserved here, but all content-bearing
//! text converges into the existing canonical evidence ABI before M12 semantic
//! compilation:
//!
//! provider structure -> EvidenceManifestation -> EvidenceSourceRevision
//!                    -> exact EvidenceSpan -> statement/PNF
//!
//! Operational/observer metadata is explicitly not promoted into free-text
//! semantic input merely because it can be ingested.

use std::collections::BTreeSet;

use crate::canonical_evidence::{
    manifestation_ref_for_revision, EvidenceManifestation,
    EvidenceManifestationFamily, EvidenceSourceRevision, EvidenceSpan,
    EvidenceSubstrateError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceFamily {
    Document,
    Mail,
    Chat,
    SocialMessage,
    Transcript,
    Audio,
    ImageOcr,
    Web,
    Wiki,
    LegalAuthority,
    NoteResearch,
    FieldCapture,
    Calendar,
    FinancialRecord,
    StructuredDataset,
    MachineArtifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestRoleClass {
    /// Content may yield exact source spans/records for ordinary M12 semantic
    /// candidate production.
    ContentSource,
    /// Observation about another producer/source. Metadata alone is not a
    /// substitute for the observed content.
    ObserverSource,
    /// Work/activity/process state. It belongs in operational projections,
    /// not silently in proposition semantics.
    OperationalSource,
    /// Externally governed authority source such as a fetched judgment.
    ExternalAuthoritySource,
    /// Context projected alongside a Matter without becoming source truth.
    ContextOverlay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceIngestEnvelope {
    pub source_ref: String,
    pub source_revision_ref: String,
    pub provider_ref: String,
    pub family: SourceFamily,
    pub role_class: IngestRoleClass,
    pub content_digest_ref: String,
    pub acquisition_receipt_ref: String,
    pub media_type_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceIngestError {
    EmptyCoordinate(&'static str),
    PromotionNotAllowed,
    ContentSpanRequired,
    MetadataOnlyMayNotBecomeTextStatement,
    RevisionMismatch,
    InvalidRange,
    DuplicateRegion(String),
    MissingBodyRevision,
    InvalidMailParticipant,
    InvalidDocumentRegion,
    CanonicalWeldMismatch(&'static str),
    CanonicalEvidence(EvidenceSubstrateError),
}

impl From<EvidenceSubstrateError> for SourceIngestError {
    fn from(value: EvidenceSubstrateError) -> Self {
        Self::CanonicalEvidence(value)
    }
}

fn require(name: &'static str, value: &str) -> Result<(), SourceIngestError> {
    if value.trim().is_empty() {
        Err(SourceIngestError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

impl SourceIngestEnvelope {
    pub fn validate(&self) -> Result<(), SourceIngestError> {
        for (name, value) in [
            ("source_ref", self.source_ref.as_str()),
            ("source_revision_ref", self.source_revision_ref.as_str()),
            ("provider_ref", self.provider_ref.as_str()),
            ("content_digest_ref", self.content_digest_ref.as_str()),
            (
                "acquisition_receipt_ref",
                self.acquisition_receipt_ref.as_str(),
            ),
            ("media_type_ref", self.media_type_ref.as_str()),
        ] {
            require(name, value)?;
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(SourceIngestError::PromotionNotAllowed);
        }
        Ok(())
    }

    #[must_use]
    pub const fn semantic_text_allowed(&self) -> bool {
        matches!(
            self.role_class,
            IngestRoleClass::ContentSource | IngestRoleClass::ExternalAuthoritySource
        )
    }

    pub fn to_manifestation(&self) -> Result<EvidenceManifestation, SourceIngestError> {
        self.validate()?;
        let family = match self.family {
            SourceFamily::LegalAuthority => EvidenceManifestationFamily::LegalAuthority,
            SourceFamily::Transcript | SourceFamily::Audio => {
                EvidenceManifestationFamily::Transcript
            }
            SourceFamily::Document => {
                if self.media_type_ref.eq_ignore_ascii_case("application/pdf") {
                    EvidenceManifestationFamily::PdfDocument
                } else {
                    EvidenceManifestationFamily::Other
                }
            },
            SourceFamily::Wiki => EvidenceManifestationFamily::Wikipedia,
            _ => EvidenceManifestationFamily::Other,
        };
        let manifestation = EvidenceManifestation {
            manifestation_ref: manifestation_ref_for_revision(&self.source_revision_ref),
            family,
            source_ref: self.source_ref.clone(),
            source_revision_ref: self.source_revision_ref.clone(),
            content_digest_ref: self.content_digest_ref.clone(),
            acquisition_receipt_ref: self.acquisition_receipt_ref.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        manifestation
            .validate()
            .map_err(|_| SourceIngestError::PromotionNotAllowed)?;
        Ok(manifestation)
    }

    pub fn to_revision(&self) -> Result<EvidenceSourceRevision, SourceIngestError> {
        let manifestation = self.to_manifestation()?;
        Ok(EvidenceSourceRevision::from_manifestation(
            &manifestation,
            self.acquisition_receipt_ref.clone(),
        )?)
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalCompiledSource {
    pub ingest: SourceIngestEnvelope,
    pub manifestation: EvidenceManifestation,
    pub revision: EvidenceSourceRevision,
    pub provider_specific_review_shortcut: bool,
    pub provider_specific_projection_shortcut: bool,
}

impl CanonicalCompiledSource {
    pub fn from_ingest(ingest: SourceIngestEnvelope) -> Result<Self, SourceIngestError> {
        ingest.validate()?;
        let manifestation = ingest.to_manifestation()?;
        let revision = EvidenceSourceRevision::from_manifestation(
            &manifestation,
            ingest.acquisition_receipt_ref.clone(),
        )?;
        let compiled = Self {
            ingest,
            manifestation,
            revision,
            provider_specific_review_shortcut: false,
            provider_specific_projection_shortcut: false,
        };
        compiled.validate()?;
        Ok(compiled)
    }

    pub fn validate(&self) -> Result<(), SourceIngestError> {
        self.ingest.validate()?;
        self.manifestation
            .validate()
            .map_err(|_| SourceIngestError::PromotionNotAllowed)?;
        self.revision.validate()?;
        if self.manifestation.source_ref != self.ingest.source_ref {
            return Err(SourceIngestError::CanonicalWeldMismatch("source_ref"));
        }
        if self.manifestation.source_revision_ref != self.ingest.source_revision_ref {
            return Err(SourceIngestError::CanonicalWeldMismatch(
                "manifestation_source_revision_ref",
            ));
        }
        if self.revision.source_revision_ref != self.manifestation.source_revision_ref {
            return Err(SourceIngestError::CanonicalWeldMismatch(
                "revision_source_revision_ref",
            ));
        }
        if self.revision.manifestation_ref != self.manifestation.manifestation_ref {
            return Err(SourceIngestError::CanonicalWeldMismatch(
                "revision_manifestation_ref",
            ));
        }
        if self.revision.content_digest_ref != self.manifestation.content_digest_ref {
            return Err(SourceIngestError::CanonicalWeldMismatch(
                "revision_content_digest_ref",
            ));
        }
        if self.provider_specific_review_shortcut
            || self.provider_specific_projection_shortcut
        {
            return Err(SourceIngestError::PromotionNotAllowed);
        }
        Ok(())
    }

    pub fn weld_span(&self, span: EvidenceSpan) -> Result<CanonicalSourceRegion, SourceIngestError> {
        self.validate()?;
        span.validate()?;
        if span.source_revision_ref != self.revision.source_revision_ref {
            return Err(SourceIngestError::RevisionMismatch);
        }
        Ok(CanonicalSourceRegion {
            source_revision_ref: self.revision.source_revision_ref.clone(),
            span,
            eligibility: SemanticRegionEligibility::Unknown,
            structure_creates_semantic_observation: false,
            structure_creates_claim_truth: false,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticRegionEligibility {
    SemanticCandidate,
    TransportOnly,
    StructuralOnly,
    ObserverOnly,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSourceRegion {
    pub source_revision_ref: String,
    pub span: EvidenceSpan,
    pub eligibility: SemanticRegionEligibility,
    pub structure_creates_semantic_observation: bool,
    pub structure_creates_claim_truth: bool,
}

impl CanonicalSourceRegion {
    pub fn validate(&self) -> Result<(), SourceIngestError> {
        self.span.validate()?;
        if self.source_revision_ref != self.span.source_revision_ref {
            return Err(SourceIngestError::RevisionMismatch);
        }
        if self.structure_creates_semantic_observation || self.structure_creates_claim_truth {
            return Err(SourceIngestError::PromotionNotAllowed);
        }
        Ok(())
    }

    #[must_use]
    pub fn with_eligibility(mut self, eligibility: SemanticRegionEligibility) -> Self {
        self.eligibility = eligibility;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentRegionKind {
    Chapter,
    Section,
    Page,
    Paragraph,
    Sentence,
    Footnote,
    TableCell,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRegion {
    pub region_ref: String,
    pub source_revision_ref: String,
    pub parent_region_ref: Option<String>,
    pub kind: DocumentRegionKind,
    pub start_char: u64,
    pub end_char: u64,
}

impl DocumentRegion {
    pub fn validate(&self) -> Result<(), SourceIngestError> {
        require("region_ref", &self.region_ref)?;
        require("source_revision_ref", &self.source_revision_ref)?;
        if self
            .parent_region_ref
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(SourceIngestError::EmptyCoordinate("parent_region_ref"));
        }
        if self.start_char >= self.end_char {
            return Err(SourceIngestError::InvalidDocumentRegion);
        }
        Ok(())
    }

    pub fn as_span(&self) -> Result<EvidenceSpan, SourceIngestError> {
        self.validate()?;
        Ok(EvidenceSpan::text(
            self.source_revision_ref.clone(),
            self.region_ref.clone(),
            self.start_char,
            self.end_char,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongDocumentSource {
    pub ingest: SourceIngestEnvelope,
    pub title: Option<String>,
    pub edition_ref: Option<String>,
    pub regions: Vec<DocumentRegion>,
}

impl LongDocumentSource {
    pub fn validate(&self) -> Result<(), SourceIngestError> {
        self.ingest.validate()?;
        if self.ingest.family != SourceFamily::Document {
            return Err(SourceIngestError::InvalidDocumentRegion);
        }
        if !self.ingest.semantic_text_allowed() {
            return Err(SourceIngestError::MetadataOnlyMayNotBecomeTextStatement);
        }
        let mut seen = BTreeSet::new();
        for region in &self.regions {
            region.validate()?;
            if region.source_revision_ref != self.ingest.source_revision_ref {
                return Err(SourceIngestError::RevisionMismatch);
            }
            if !seen.insert(region.region_ref.clone()) {
                return Err(SourceIngestError::DuplicateRegion(region.region_ref.clone()));
            }
        }
        Ok(())
    }

    pub fn exact_spans(&self) -> Result<Vec<EvidenceSpan>, SourceIngestError> {
        self.validate()?;
        self.regions.iter().map(DocumentRegion::as_span).collect()
    }

    pub fn canonical_compiled_source(&self) -> Result<CanonicalCompiledSource, SourceIngestError> {
        self.validate()?;
        CanonicalCompiledSource::from_ingest(self.ingest.clone())
    }

    pub fn canonical_regions(&self) -> Result<Vec<CanonicalSourceRegion>, SourceIngestError> {
        let source = self.canonical_compiled_source()?;
        self.regions
            .iter()
            .map(|region| {
                let eligibility = match region.kind {
                    DocumentRegionKind::Sentence => SemanticRegionEligibility::SemanticCandidate,
                    _ => SemanticRegionEligibility::StructuralOnly,
                };
                source
                    .weld_span(region.as_span()?)
                    .map(|region| region.with_eligibility(eligibility))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailParticipant {
    pub display_name: Option<String>,
    pub address: String,
}

impl MailParticipant {
    pub fn validate(&self) -> Result<(), SourceIngestError> {
        if self.address.trim().is_empty() {
            return Err(SourceIngestError::InvalidMailParticipant);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailBodySegmentKind {
    AuthoredHere,
    QuotedPriorMessage,
    ForwardedMessage,
    Signature,
    HeaderReplica,
    Disclaimer,
    AttachmentText,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailBodySegment {
    pub segment_ref: String,
    pub body_revision_ref: String,
    pub kind: MailBodySegmentKind,
    pub start_char: u64,
    pub end_char: u64,
    /// If this segment is a transport/reproduction of earlier material, retain
    /// the earlier message identity without pretending the quote is a new
    /// independent witness.
    pub derived_from_message_ref: Option<String>,
    pub canonical_text_lineage_ref: Option<String>,
}

impl MailBodySegment {
    pub fn validate(&self) -> Result<(), SourceIngestError> {
        require("segment_ref", &self.segment_ref)?;
        require("body_revision_ref", &self.body_revision_ref)?;
        if self.start_char >= self.end_char {
            return Err(SourceIngestError::InvalidRange);
        }
        if self
            .derived_from_message_ref
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(SourceIngestError::EmptyCoordinate(
                "derived_from_message_ref",
            ));
        }
        if self
            .canonical_text_lineage_ref
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(SourceIngestError::EmptyCoordinate(
                "canonical_text_lineage_ref",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub const fn is_independent_authorship_candidate(&self) -> bool {
        matches!(
            self.kind,
            MailBodySegmentKind::AuthoredHere | MailBodySegmentKind::AttachmentText
        )
    }

    pub fn as_span(&self) -> Result<EvidenceSpan, SourceIngestError> {
        self.validate()?;
        Ok(EvidenceSpan::text(
            self.body_revision_ref.clone(),
            self.segment_ref.clone(),
            self.start_char,
            self.end_char,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailMessageSource {
    pub ingest: SourceIngestEnvelope,
    pub message_ref: String,
    pub account_or_collection_ref: String,
    pub provider_message_id: Option<String>,
    pub internet_message_id: Option<String>,
    pub thread_ref: Option<String>,
    pub reply_to_ref: Option<String>,
    pub reference_message_refs: Vec<String>,
    pub from: Vec<MailParticipant>,
    pub to: Vec<MailParticipant>,
    pub cc: Vec<MailParticipant>,
    pub bcc: Vec<MailParticipant>,
    pub sent_time_ref: Option<String>,
    pub received_time_ref: Option<String>,
    pub subject_revision_ref: Option<String>,
    pub body_revision_ref: String,
    pub attachment_refs: Vec<String>,
    pub raw_source_ref: String,
    pub segments: Vec<MailBodySegment>,
}

impl MailMessageSource {
    pub fn validate(&self) -> Result<(), SourceIngestError> {
        self.ingest.validate()?;
        if self.ingest.family != SourceFamily::Mail {
            return Err(SourceIngestError::MissingBodyRevision);
        }
        if !self.ingest.semantic_text_allowed() {
            return Err(SourceIngestError::MetadataOnlyMayNotBecomeTextStatement);
        }
        if self.body_revision_ref != self.ingest.source_revision_ref {
            return Err(SourceIngestError::RevisionMismatch);
        }
        for (name, value) in [
            ("message_ref", self.message_ref.as_str()),
            (
                "account_or_collection_ref",
                self.account_or_collection_ref.as_str(),
            ),
            ("body_revision_ref", self.body_revision_ref.as_str()),
            ("raw_source_ref", self.raw_source_ref.as_str()),
        ] {
            require(name, value)?;
        }
        for participant in self
            .from
            .iter()
            .chain(self.to.iter())
            .chain(self.cc.iter())
            .chain(self.bcc.iter())
        {
            participant.validate()?;
        }
        let mut seen = BTreeSet::new();
        for segment in &self.segments {
            segment.validate()?;
            if segment.body_revision_ref != self.body_revision_ref {
                return Err(SourceIngestError::RevisionMismatch);
            }
            if !seen.insert(segment.segment_ref.clone()) {
                return Err(SourceIngestError::DuplicateRegion(
                    segment.segment_ref.clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn exact_body_spans(&self) -> Result<Vec<EvidenceSpan>, SourceIngestError> {
        self.validate()?;
        self.segments.iter().map(MailBodySegment::as_span).collect()
    }

    pub fn authored_semantic_spans(&self) -> Result<Vec<EvidenceSpan>, SourceIngestError> {
        self.validate()?;
        self.segments
            .iter()
            .filter(|segment| segment.is_independent_authorship_candidate())
            .map(MailBodySegment::as_span)
            .collect()
    }

    #[must_use]
    pub fn quoted_transport_occurrence_count(&self) -> usize {
        self.segments
            .iter()
            .filter(|segment| {
                matches!(
                    segment.kind,
                    MailBodySegmentKind::QuotedPriorMessage
                        | MailBodySegmentKind::ForwardedMessage
                )
            })
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCompilationReceipt {
    pub source_ref: String,
    pub source_revision_ref: String,
    pub family: SourceFamily,
    pub role_class: IngestRoleClass,
    pub exact_region_count: usize,
    pub semantic_candidate_region_count: usize,
    pub transport_or_nonsemantic_region_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub source_omitted_when_parse_fails: bool,
    pub candidate_interpretation_is_truth: bool,
}

pub fn document_compilation_receipt(
    document: &LongDocumentSource,
) -> Result<SourceCompilationReceipt, SourceIngestError> {
    document.validate()?;
    let semantic_candidate_region_count = document
        .regions
        .iter()
        .filter(|region| region.kind == DocumentRegionKind::Sentence)
        .count();
    Ok(SourceCompilationReceipt {
        source_ref: document.ingest.source_ref.clone(),
        source_revision_ref: document.ingest.source_revision_ref.clone(),
        family: SourceFamily::Document,
        role_class: document.ingest.role_class,
        exact_region_count: document.regions.len(),
        semantic_candidate_region_count,
        transport_or_nonsemantic_region_count: document
            .regions
            .len()
            .saturating_sub(semantic_candidate_region_count),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        source_omitted_when_parse_fails: false,
        candidate_interpretation_is_truth: false,
    })
}

pub fn mail_compilation_receipt(
    message: &MailMessageSource,
) -> Result<SourceCompilationReceipt, SourceIngestError> {
    message.validate()?;
    let semantic_candidate_region_count = message
        .segments
        .iter()
        .filter(|segment| segment.is_independent_authorship_candidate())
        .count();
    Ok(SourceCompilationReceipt {
        source_ref: message.ingest.source_ref.clone(),
        source_revision_ref: message.body_revision_ref.clone(),
        family: SourceFamily::Mail,
        role_class: message.ingest.role_class,
        exact_region_count: message.segments.len(),
        semantic_candidate_region_count,
        transport_or_nonsemantic_region_count: message
            .segments
            .len()
            .saturating_sub(semantic_candidate_region_count),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        source_omitted_when_parse_fails: false,
        candidate_interpretation_is_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingest(family: SourceFamily, role_class: IngestRoleClass) -> SourceIngestEnvelope {
        SourceIngestEnvelope {
            source_ref: "source:1".into(),
            source_revision_ref: "revision:1".into(),
            provider_ref: "provider:test".into(),
            family,
            role_class,
            content_digest_ref: "sha256:test".into(),
            acquisition_receipt_ref: "receipt:1".into(),
            media_type_ref: "text/plain".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }


    #[test]
    fn canonical_compiled_source_is_a_real_manifestation_revision_weld() {
        let envelope = ingest(SourceFamily::Document, IngestRoleClass::ContentSource);
        let compiled = CanonicalCompiledSource::from_ingest(envelope).unwrap();

        assert_eq!(
            compiled.revision.source_revision_ref,
            compiled.manifestation.source_revision_ref
        );
        assert_eq!(
            compiled.revision.manifestation_ref,
            compiled.manifestation.manifestation_ref
        );
        assert_eq!(
            compiled.revision.content_digest_ref,
            compiled.manifestation.content_digest_ref
        );
        assert!(!compiled.provider_specific_review_shortcut);
        assert!(!compiled.provider_specific_projection_shortcut);

        let region = compiled
            .weld_span(EvidenceSpan::text(
                compiled.revision.source_revision_ref.clone(),
                "span:fixture",
                0,
                10,
            )
            .unwrap())
            .unwrap()
            .with_eligibility(SemanticRegionEligibility::SemanticCandidate);
        assert!(region.validate().is_ok());
        assert_eq!(
            region.span.source_revision_ref,
            compiled.revision.source_revision_ref
        );
        assert!(!region.structure_creates_semantic_observation);
        assert!(!region.structure_creates_claim_truth);
    }

    #[test]
    fn all_content_families_share_existing_canonical_evidence_abi() {
        for family in [
            SourceFamily::Document,
            SourceFamily::Mail,
            SourceFamily::Chat,
            SourceFamily::SocialMessage,
            SourceFamily::Transcript,
            SourceFamily::ImageOcr,
            SourceFamily::Web,
            SourceFamily::Wiki,
            SourceFamily::LegalAuthority,
            SourceFamily::NoteResearch,
            SourceFamily::FieldCapture,
        ] {
            let envelope = ingest(family, IngestRoleClass::ContentSource);
            assert!(envelope.to_manifestation().unwrap().validate().is_ok());
            assert!(envelope.to_revision().unwrap().validate().is_ok());
        }
    }

    #[test]
    fn observer_and_operational_metadata_cannot_silently_become_text_semantics() {
        for role_class in [
            IngestRoleClass::ObserverSource,
            IngestRoleClass::OperationalSource,
            IngestRoleClass::ContextOverlay,
        ] {
            let mut document = LongDocumentSource {
                ingest: ingest(SourceFamily::Document, role_class),
                title: None,
                edition_ref: None,
                regions: vec![DocumentRegion {
                    region_ref: "span:1".into(),
                    source_revision_ref: "revision:1".into(),
                    parent_region_ref: None,
                    kind: DocumentRegionKind::Sentence,
                    start_char: 0,
                    end_char: 10,
                }],
            };
            assert_eq!(
                document.validate(),
                Err(SourceIngestError::MetadataOnlyMayNotBecomeTextStatement)
            );
            document.ingest.role_class = IngestRoleClass::ContentSource;
            assert!(document.validate().is_ok());
        }
    }


    #[test]
    fn long_document_regions_are_welded_to_canonical_spans_before_semantics() {
        let document = LongDocumentSource {
            ingest: ingest(SourceFamily::Document, IngestRoleClass::ContentSource),
            title: Some("Book".into()),
            edition_ref: None,
            regions: vec![
                DocumentRegion {
                    region_ref: "chapter:1".into(),
                    source_revision_ref: "revision:1".into(),
                    parent_region_ref: None,
                    kind: DocumentRegionKind::Chapter,
                    start_char: 0,
                    end_char: 100,
                },
                DocumentRegion {
                    region_ref: "sentence:1".into(),
                    source_revision_ref: "revision:1".into(),
                    parent_region_ref: Some("chapter:1".into()),
                    kind: DocumentRegionKind::Sentence,
                    start_char: 0,
                    end_char: 20,
                },
            ],
        };

        let canonical = document.canonical_regions().unwrap();
        assert_eq!(canonical.len(), 2);
        assert_eq!(
            canonical[0].eligibility,
            SemanticRegionEligibility::StructuralOnly
        );
        assert_eq!(
            canonical[1].eligibility,
            SemanticRegionEligibility::SemanticCandidate
        );
        assert!(canonical.iter().all(|region| {
            region.span.source_revision_ref == "revision:1"
                && !region.structure_creates_semantic_observation
                && !region.structure_creates_claim_truth
        }));
    }

    #[test]
    fn whole_long_document_keeps_hierarchical_exact_regions() {
        let document = LongDocumentSource {
            ingest: ingest(SourceFamily::Document, IngestRoleClass::ContentSource),
            title: Some("Book".into()),
            edition_ref: Some("edition:1".into()),
            regions: vec![
                DocumentRegion {
                    region_ref: "chapter:1".into(),
                    source_revision_ref: "revision:1".into(),
                    parent_region_ref: None,
                    kind: DocumentRegionKind::Chapter,
                    start_char: 0,
                    end_char: 100,
                },
                DocumentRegion {
                    region_ref: "paragraph:1".into(),
                    source_revision_ref: "revision:1".into(),
                    parent_region_ref: Some("chapter:1".into()),
                    kind: DocumentRegionKind::Paragraph,
                    start_char: 0,
                    end_char: 50,
                },
                DocumentRegion {
                    region_ref: "sentence:1".into(),
                    source_revision_ref: "revision:1".into(),
                    parent_region_ref: Some("paragraph:1".into()),
                    kind: DocumentRegionKind::Sentence,
                    start_char: 0,
                    end_char: 20,
                },
            ],
        };
        let spans = document.exact_spans().unwrap();
        let receipt = document_compilation_receipt(&document).unwrap();
        assert_eq!(spans.len(), 3);
        assert_eq!(receipt.exact_region_count, 3);
        assert_eq!(receipt.semantic_candidate_region_count, 1);
        assert_eq!(receipt.transport_or_nonsemantic_region_count, 2);
        assert!(!receipt.source_omitted_when_parse_fails);
        assert!(!receipt.candidate_interpretation_is_truth);
    }

    #[test]
    fn quoted_mail_is_transport_not_independent_authorship() {
        let message = MailMessageSource {
            ingest: ingest(SourceFamily::Mail, IngestRoleClass::ContentSource),
            message_ref: "mail:1".into(),
            account_or_collection_ref: "mailbox:test".into(),
            provider_message_id: Some("provider-id:1".into()),
            internet_message_id: Some("<1@example.test>".into()),
            thread_ref: Some("thread:1".into()),
            reply_to_ref: None,
            reference_message_refs: vec![],
            from: vec![MailParticipant {
                display_name: Some("Alice".into()),
                address: "alice@example.test".into(),
            }],
            to: vec![MailParticipant {
                display_name: Some("Bob".into()),
                address: "bob@example.test".into(),
            }],
            cc: vec![],
            bcc: vec![],
            sent_time_ref: Some("time:sent:1".into()),
            received_time_ref: None,
            subject_revision_ref: None,
            body_revision_ref: "revision:1".into(),
            attachment_refs: vec![],
            raw_source_ref: "raw:mail:1".into(),
            segments: vec![
                MailBodySegment {
                    segment_ref: "span:authored".into(),
                    body_revision_ref: "revision:1".into(),
                    kind: MailBodySegmentKind::AuthoredHere,
                    start_char: 0,
                    end_char: 12,
                    derived_from_message_ref: None,
                    canonical_text_lineage_ref: Some("lineage:text:new".into()),
                },
                MailBodySegment {
                    segment_ref: "span:quoted".into(),
                    body_revision_ref: "revision:1".into(),
                    kind: MailBodySegmentKind::QuotedPriorMessage,
                    start_char: 13,
                    end_char: 40,
                    derived_from_message_ref: Some("mail:prior".into()),
                    canonical_text_lineage_ref: Some("lineage:text:prior".into()),
                },
                MailBodySegment {
                    segment_ref: "span:signature".into(),
                    body_revision_ref: "revision:1".into(),
                    kind: MailBodySegmentKind::Signature,
                    start_char: 41,
                    end_char: 50,
                    derived_from_message_ref: None,
                    canonical_text_lineage_ref: None,
                },
            ],
        };

        assert_eq!(message.exact_body_spans().unwrap().len(), 3);
        assert_eq!(message.authored_semantic_spans().unwrap().len(), 1);
        assert_eq!(message.quoted_transport_occurrence_count(), 1);

        let receipt = mail_compilation_receipt(&message).unwrap();
        assert_eq!(receipt.exact_region_count, 3);
        assert_eq!(receipt.semantic_candidate_region_count, 1);
        assert_eq!(receipt.transport_or_nonsemantic_region_count, 2);
        assert!(!receipt.claim_truth_promoted);
    }

    #[test]
    fn provider_identity_never_becomes_universal_message_identity() {
        let message = MailMessageSource {
            ingest: ingest(SourceFamily::Mail, IngestRoleClass::ContentSource),
            message_ref: "mail:canonical-source-coordinate".into(),
            account_or_collection_ref: "collection:jmail".into(),
            provider_message_id: Some("jmail:42".into()),
            internet_message_id: Some("<external@example.test>".into()),
            thread_ref: None,
            reply_to_ref: None,
            reference_message_refs: vec![],
            from: vec![],
            to: vec![],
            cc: vec![],
            bcc: vec![],
            sent_time_ref: None,
            received_time_ref: None,
            subject_revision_ref: None,
            body_revision_ref: "revision:1".into(),
            attachment_refs: vec![],
            raw_source_ref: "raw:1".into(),
            segments: vec![],
        };
        message.validate().unwrap();
        assert_ne!(
            message.message_ref,
            message.provider_message_id.as_deref().unwrap()
        );
        assert_ne!(
            message.message_ref,
            message.internet_message_id.as_deref().unwrap()
        );
    }
}
