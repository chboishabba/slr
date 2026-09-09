#![allow(dead_code)]

//! OALC legislation input contract for the SensibLaw parser/PNF path.
//!
//! This carrier is intentionally narrower than general OALC indexing. It says
//! what must be supplied before a local OALC legislation document may be used
//! as parser input: a local corpus path, an immutable/pinned corpus revision,
//! an exact legislation record, retained text digest, and an explicit temporal
//! coverage status.
//!
//! It does NOT claim historical equivalence, legal authority, semantic truth,
//! applicability, or an Atomic gate.

use std::path::{Path, PathBuf};

pub const OALC_PARSER_AUTHORITY: &str = "source_observation_only";
pub const OALC_RECEIPT_AUTHORITY: &str = "experimental_candidate_only";

pub const CULLEN_CLA_CITATION: &str = "Civil Liability Act 2002 (NSW)";
pub const CULLEN_VICARIOUS_CITATION: &str =
    "Law Reform (Vicarious Liability) Act 1983 (NSW)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OalcTemporalCoverage {
    /// OALC NSW legislation is usable as the latest-known/current parser text,
    /// but no historical equivalence is claimed for a past legal date.
    LatestKnownOnly,
    /// A separate source receipt has independently paid historical equivalence.
    HistoricallyVerified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedOalcCorpusInput {
    pub corpus_jsonl_path: PathBuf,
    pub corpus_revision_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OalcInputError {
    MissingCorpusPath,
    MissingRevision,
    MutableRevisionAlias,
    WrongSource,
    WrongJurisdiction,
    WrongDocumentType,
    EmptyVersionId,
    EmptyText,
    EmptyDigest,
    MissingArtifact,
}

fn looks_mutable_revision_alias(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "main" | "master" | "latest" | "head" | "current" | "oalc:revision-unset"
    )
}

impl PinnedOalcCorpusInput {
    pub fn validate(&self) -> Result<(), OalcInputError> {
        if self.corpus_jsonl_path.as_os_str().is_empty() {
            return Err(OalcInputError::MissingCorpusPath);
        }
        if self.corpus_revision_ref.trim().is_empty() {
            return Err(OalcInputError::MissingRevision);
        }
        if looks_mutable_revision_alias(&self.corpus_revision_ref) {
            return Err(OalcInputError::MutableRevisionAlias);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcLegislationDocumentReceipt {
    pub citation: String,
    pub version_id: String,
    pub corpus_revision_ref: String,
    pub source: String,
    pub jurisdiction: String,
    pub document_type: String,
    pub canonical_text_digest: String,
    pub local_artifact_ref: PathBuf,
    pub temporal_coverage: OalcTemporalCoverage,
    pub receipt_authority: &'static str,
}

impl OalcLegislationDocumentReceipt {
    pub fn validate(&self) -> Result<(), OalcInputError> {
        if self.source != "nsw_legislation" {
            return Err(OalcInputError::WrongSource);
        }
        if self.jurisdiction != "new_south_wales" {
            return Err(OalcInputError::WrongJurisdiction);
        }
        if self.document_type != "primary_legislation" {
            return Err(OalcInputError::WrongDocumentType);
        }
        if self.version_id.trim().is_empty() {
            return Err(OalcInputError::EmptyVersionId);
        }
        if self.canonical_text_digest.trim().is_empty() {
            return Err(OalcInputError::EmptyDigest);
        }
        if self.local_artifact_ref.as_os_str().is_empty() {
            return Err(OalcInputError::MissingArtifact);
        }
        Ok(())
    }

    pub const fn creates_historical_equivalence(&self) -> bool {
        false
    }

    pub const fn creates_legal_authority(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcSectionSliceReceipt {
    pub citation: String,
    pub section: String,
    pub parent_version_id: String,
    pub corpus_revision_ref: String,
    pub parent_digest: String,
    pub slice_start: usize,
    pub slice_end: usize,
    pub slice_digest: String,
    pub slice_artifact_ref: PathBuf,
    pub temporal_coverage: OalcTemporalCoverage,
    pub parser_authority: &'static str,
}

impl OalcSectionSliceReceipt {
    pub fn parser_eligible(&self) -> bool {
        self.slice_start < self.slice_end
            && !self.slice_digest.trim().is_empty()
            && !self.parent_digest.trim().is_empty()
            && !self.parent_version_id.trim().is_empty()
            && !self.corpus_revision_ref.trim().is_empty()
            && !self.slice_artifact_ref.as_os_str().is_empty()
    }

    pub const fn pays_in_force_on(&self, _date: &str) -> bool {
        matches!(self.temporal_coverage, OalcTemporalCoverage::HistoricallyVerified)
    }

    pub const fn creates_atomic_gate(&self) -> bool {
        false
    }
}

pub fn validate_cullen_target_citation(citation: &str) -> bool {
    matches!(citation, CULLEN_CLA_CITATION | CULLEN_VICARIOUS_CITATION)
}

pub fn corpus_path_exists(input: &PinnedOalcCorpusInput) -> bool {
    Path::new(&input.corpus_jsonl_path).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutable_revision_alias_is_rejected() {
        let input = PinnedOalcCorpusInput {
            corpus_jsonl_path: PathBuf::from("/data/oalc/corpus.jsonl"),
            corpus_revision_ref: "latest".into(),
        };
        assert_eq!(input.validate(), Err(OalcInputError::MutableRevisionAlias));
    }

    #[test]
    fn latest_known_slice_is_parser_eligible_but_not_historically_paid() {
        let slice = OalcSectionSliceReceipt {
            citation: CULLEN_CLA_CITATION.into(),
            section: "5B".into(),
            parent_version_id: "version:fixture".into(),
            corpus_revision_ref: "isaacus/open-australian-legal-corpus@deadbeef".into(),
            parent_digest: "sha256:parent".into(),
            slice_start: 10,
            slice_end: 20,
            slice_digest: "sha256:slice".into(),
            slice_artifact_ref: PathBuf::from("artifact.txt"),
            temporal_coverage: OalcTemporalCoverage::LatestKnownOnly,
            parser_authority: OALC_PARSER_AUTHORITY,
        };
        assert!(slice.parser_eligible());
        assert!(!slice.pays_in_force_on("2017-01-26"));
        assert!(!slice.creates_atomic_gate());
    }
}
