//! Canonical Sprint-2 manifestation envelope.
//!
//! This is deliberately source/provenance structure only. A manifestation may
//! be reviewed or projected elsewhere, but this carrier never creates semantic
//! authority, legal applicability or claim truth.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceManifestationFamily {
    ZelphHyperfabric,
    Wikidata,
    Wikipedia,
    Oalc,
    LegalAuthority,
    PdfDocument,
    Transcript,
    UserEvidence,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceManifestation {
    pub manifestation_ref: String,
    pub family: EvidenceManifestationFamily,
    pub source_ref: String,
    pub source_revision_ref: String,
    pub content_digest_ref: String,
    pub acquisition_receipt_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceManifestationError {
    EmptyCoordinate(&'static str),
    MustRemainCandidateOnly,
    PromotionNotAllowed,
}

impl EvidenceManifestation {
    pub fn validate(&self) -> Result<(), EvidenceManifestationError> {
        for (name, value) in [
            ("manifestation_ref", self.manifestation_ref.as_str()),
            ("source_ref", self.source_ref.as_str()),
            ("source_revision_ref", self.source_revision_ref.as_str()),
            ("content_digest_ref", self.content_digest_ref.as_str()),
            ("acquisition_receipt_ref", self.acquisition_receipt_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(EvidenceManifestationError::EmptyCoordinate(name));
            }
        }
        if !self.candidate_only {
            return Err(EvidenceManifestationError::MustRemainCandidateOnly);
        }
        if self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(EvidenceManifestationError::PromotionNotAllowed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(family: EvidenceManifestationFamily) -> EvidenceManifestation {
        EvidenceManifestation {
            manifestation_ref: "manifestation:fixture".into(),
            family,
            source_ref: "source:fixture".into(),
            source_revision_ref: "revision:fixture".into(),
            content_digest_ref: "sha256:fixture".into(),
            acquisition_receipt_ref: "receipt:fixture".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn one_envelope_accepts_all_sprint2_source_families() {
        for family in [
            EvidenceManifestationFamily::ZelphHyperfabric,
            EvidenceManifestationFamily::Wikidata,
            EvidenceManifestationFamily::Wikipedia,
            EvidenceManifestationFamily::Oalc,
            EvidenceManifestationFamily::LegalAuthority,
            EvidenceManifestationFamily::PdfDocument,
            EvidenceManifestationFamily::Transcript,
            EvidenceManifestationFamily::UserEvidence,
        ] {
            assert!(fixture(family).validate().is_ok());
        }
    }

    #[test]
    fn manifestation_never_promotes_semantics() {
        let mut value = fixture(EvidenceManifestationFamily::Wikidata);
        value.claim_truth_promoted = true;
        assert_eq!(
            value.validate(),
            Err(EvidenceManifestationError::PromotionNotAllowed)
        );
    }

    #[test]
    fn manifestation_requires_exact_revision_and_receipt_coordinates() {
        let mut value = fixture(EvidenceManifestationFamily::Oalc);
        value.source_revision_ref.clear();
        assert_eq!(
            value.validate(),
            Err(EvidenceManifestationError::EmptyCoordinate("source_revision_ref"))
        );

        let mut value = fixture(EvidenceManifestationFamily::Oalc);
        value.acquisition_receipt_ref.clear();
        assert_eq!(
            value.validate(),
            Err(EvidenceManifestationError::EmptyCoordinate(
                "acquisition_receipt_ref"
            ))
        );
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSourceRevision {
    pub source_revision_ref: String,
    pub manifestation_ref: String,
    pub content_digest_ref: String,
    pub revision_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceSpanKind {
    TextRange { start_char: u64, end_char: u64 },
    StructuredCoordinate { coordinate_ref: String },
    WholeRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSpan {
    pub source_revision_ref: String,
    pub span_ref: String,
    pub kind: EvidenceSpanKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceObservation {
    pub observation_ref: String,
    pub source_revision_ref: String,
    pub span: EvidenceSpan,
    pub predicate_ref: String,
    pub value_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceSubstrateError {
    EmptyCoordinate(&'static str),
    InvertedTextRange,
    RevisionMismatch,
    MustRemainCandidateOnly,
    PromotionNotAllowed,
}

impl EvidenceSourceRevision {
    pub fn from_manifestation(
        manifestation: &EvidenceManifestation,
        revision_receipt_ref: impl Into<String>,
    ) -> Result<Self, EvidenceSubstrateError> {
        let revision = Self {
            source_revision_ref: manifestation.source_revision_ref.clone(),
            manifestation_ref: manifestation.manifestation_ref.clone(),
            content_digest_ref: manifestation.content_digest_ref.clone(),
            revision_receipt_ref: revision_receipt_ref.into(),
        };
        revision.validate()?;
        Ok(revision)
    }

    pub fn validate(&self) -> Result<(), EvidenceSubstrateError> {
        for (name, value) in [
            ("source_revision_ref", self.source_revision_ref.as_str()),
            ("manifestation_ref", self.manifestation_ref.as_str()),
            ("content_digest_ref", self.content_digest_ref.as_str()),
            ("revision_receipt_ref", self.revision_receipt_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(EvidenceSubstrateError::EmptyCoordinate(name));
            }
        }
        Ok(())
    }
}

impl EvidenceSpan {
    pub fn text(
        source_revision_ref: impl Into<String>,
        span_ref: impl Into<String>,
        start_char: u64,
        end_char: u64,
    ) -> Result<Self, EvidenceSubstrateError> {
        if end_char < start_char {
            return Err(EvidenceSubstrateError::InvertedTextRange);
        }
        let span = Self {
            source_revision_ref: source_revision_ref.into(),
            span_ref: span_ref.into(),
            kind: EvidenceSpanKind::TextRange {
                start_char,
                end_char,
            },
        };
        span.validate()?;
        Ok(span)
    }

    pub fn structured(
        source_revision_ref: impl Into<String>,
        span_ref: impl Into<String>,
        coordinate_ref: impl Into<String>,
    ) -> Result<Self, EvidenceSubstrateError> {
        let span = Self {
            source_revision_ref: source_revision_ref.into(),
            span_ref: span_ref.into(),
            kind: EvidenceSpanKind::StructuredCoordinate {
                coordinate_ref: coordinate_ref.into(),
            },
        };
        span.validate()?;
        Ok(span)
    }

    pub fn whole_revision(
        source_revision_ref: impl Into<String>,
        span_ref: impl Into<String>,
    ) -> Result<Self, EvidenceSubstrateError> {
        let span = Self {
            source_revision_ref: source_revision_ref.into(),
            span_ref: span_ref.into(),
            kind: EvidenceSpanKind::WholeRevision,
        };
        span.validate()?;
        Ok(span)
    }

    pub fn validate(&self) -> Result<(), EvidenceSubstrateError> {
        if self.source_revision_ref.trim().is_empty() {
            return Err(EvidenceSubstrateError::EmptyCoordinate("source_revision_ref"));
        }
        if self.span_ref.trim().is_empty() {
            return Err(EvidenceSubstrateError::EmptyCoordinate("span_ref"));
        }
        match &self.kind {
            EvidenceSpanKind::TextRange {
                start_char,
                end_char,
            } if end_char < start_char => Err(EvidenceSubstrateError::InvertedTextRange),
            EvidenceSpanKind::StructuredCoordinate { coordinate_ref }
                if coordinate_ref.trim().is_empty() =>
            {
                Err(EvidenceSubstrateError::EmptyCoordinate("coordinate_ref"))
            }
            _ => Ok(()),
        }
    }
}

impl EvidenceObservation {
    pub fn validate(&self) -> Result<(), EvidenceSubstrateError> {
        for (name, value) in [
            ("observation_ref", self.observation_ref.as_str()),
            ("source_revision_ref", self.source_revision_ref.as_str()),
            ("predicate_ref", self.predicate_ref.as_str()),
            ("value_ref", self.value_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(EvidenceSubstrateError::EmptyCoordinate(name));
            }
        }
        self.span.validate()?;
        if self.source_revision_ref != self.span.source_revision_ref {
            return Err(EvidenceSubstrateError::RevisionMismatch);
        }
        if !self.candidate_only {
            return Err(EvidenceSubstrateError::MustRemainCandidateOnly);
        }
        if self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(EvidenceSubstrateError::PromotionNotAllowed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod substrate_tests {
    use super::*;

    #[test]
    fn structured_graph_coordinate_is_not_fabricated_as_text_span() {
        let span = EvidenceSpan::structured(
            "wikidata:Q1:oldid:1",
            "span:wikidata:Q1:P31",
            "wikidata:Q1:P31:Q5",
        )
        .unwrap();
        assert!(matches!(
            span.kind,
            EvidenceSpanKind::StructuredCoordinate { .. }
        ));
    }

    #[test]
    fn text_and_structured_evidence_share_one_span_abi() {
        let text = EvidenceSpan::text("revision:text:1", "span:text:0-10", 0, 10).unwrap();
        let graph = EvidenceSpan::structured(
            "revision:graph:1",
            "span:graph:p31",
            "graph:q1:p31:q5",
        )
        .unwrap();
        assert!(text.validate().is_ok());
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn observation_must_use_span_from_same_revision() {
        let observation = EvidenceObservation {
            observation_ref: "observation:fixture".into(),
            source_revision_ref: "revision:a".into(),
            span: EvidenceSpan::text("revision:b", "span:b", 0, 1).unwrap(),
            predicate_ref: "predicate:fixture".into(),
            value_ref: "value:fixture".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        assert_eq!(
            observation.validate(),
            Err(EvidenceSubstrateError::RevisionMismatch)
        );
    }

    #[test]
    fn source_revision_is_derived_from_manifestation_identity() {
        let manifestation = fixture(EvidenceManifestationFamily::Transcript);
        let revision =
            EvidenceSourceRevision::from_manifestation(&manifestation, "receipt:revision:fixture")
                .unwrap();
        assert_eq!(
            revision.source_revision_ref,
            manifestation.source_revision_ref
        );
        assert_eq!(revision.manifestation_ref, manifestation.manifestation_ref);
        assert_eq!(revision.content_digest_ref, manifestation.content_digest_ref);
    }
}
