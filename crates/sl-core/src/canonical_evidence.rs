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
