#![allow(dead_code)]

//! OALC legislation source contract for the SensibLaw LegalFollow -> provider
//! -> parser/PNF path.
//!
//! `corpus.jsonl` is NOT part of the public semantic/runtime contract. A local
//! JSONL file may be used by an offline provider implementation, but ordinary
//! LegalFollow operation asks for an exact OALC source and receives a retained
//! document receipt.
//!
//! Public chain:
//!   LegalFollow source demand
//!     -> pinned OALC dataset selection
//!     -> exact legislation resolution receipt
//!     -> retained document receipt
//!     -> section slice receipt
//!     -> spaCy/PNF handoff.
//!
//! None of these receipts claims historical equivalence, legal authority,
//! semantic truth, applicability, or an Atomic gate.

use std::path::PathBuf;

pub const OALC_DATASET_ID: &str = "isaacus/open-australian-legal-corpus";
pub const OALC_CONFIG: &str = "corpus";
pub const OALC_SPLIT: &str = "corpus";
pub const OALC_PARSER_AUTHORITY: &str = "source_observation_only";
pub const OALC_RECEIPT_AUTHORITY: &str = "experimental_candidate_only";

pub const CULLEN_CLA_CITATION: &str = "Civil Liability Act 2002 (NSW)";
pub const CULLEN_VICARIOUS_CITATION: &str =
    "Law Reform (Vicarious Liability) Act 1983 (NSW)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OalcTemporalCoverage {
    /// OALC NSW legislation is usable as latest-known parser text, but no
    /// historical equivalence is claimed for a past legal date.
    LatestKnownOnly,
    /// A separate source receipt independently pays historical equivalence.
    HistoricallyVerified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedOalcDatasetSelection {
    pub dataset_id: String,
    pub config: String,
    pub split: String,
    pub corpus_revision_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcLegislationDemand {
    pub demand_ref: String,
    pub citation: String,
    pub jurisdiction: String,
    pub source: String,
    pub document_type: String,
    pub temporal_coverage_required: OalcTemporalCoverage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OalcInputError {
    MissingRevision,
    MutableRevisionAlias,
    WrongDataset,
    WrongConfig,
    WrongSplit,
    WrongCitation,
    WrongSource,
    WrongJurisdiction,
    WrongDocumentType,
    EmptyVersionId,
    EmptyText,
    EmptyDigest,
    MissingArtifact,
    RevisionMismatch,
}

fn looks_mutable_revision_alias(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "main" | "master" | "latest" | "head" | "current" | "oalc:revision-unset"
    )
}

impl PinnedOalcDatasetSelection {
    pub fn validate(&self) -> Result<(), OalcInputError> {
        if self.dataset_id != OALC_DATASET_ID {
            return Err(OalcInputError::WrongDataset);
        }
        if self.config != OALC_CONFIG {
            return Err(OalcInputError::WrongConfig);
        }
        if self.split != OALC_SPLIT {
            return Err(OalcInputError::WrongSplit);
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

impl OalcLegislationDemand {
    pub fn validate(&self) -> Result<(), OalcInputError> {
        if !validate_cullen_target_citation(&self.citation) {
            return Err(OalcInputError::WrongCitation);
        }
        if self.source != "nsw_legislation" {
            return Err(OalcInputError::WrongSource);
        }
        if self.jurisdiction != "new_south_wales" {
            return Err(OalcInputError::WrongJurisdiction);
        }
        if self.document_type != "primary_legislation" {
            return Err(OalcInputError::WrongDocumentType);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcResolvedDocumentReceipt {
    pub demand_ref: String,
    pub citation: String,
    pub version_id: String,
    pub corpus_revision_ref: String,
    pub source: String,
    pub jurisdiction: String,
    pub document_type: String,
    pub canonical_text_digest: String,
    pub local_artifact_ref: PathBuf,
    pub temporal_coverage: OalcTemporalCoverage,
    pub network_requests: u64,
    pub receipt_authority: &'static str,
}

impl OalcResolvedDocumentReceipt {
    pub fn validate_against(
        &self,
        dataset: &PinnedOalcDatasetSelection,
        demand: &OalcLegislationDemand,
    ) -> Result<(), OalcInputError> {
        dataset.validate()?;
        demand.validate()?;
        if self.demand_ref != demand.demand_ref || self.citation != demand.citation {
            return Err(OalcInputError::WrongCitation);
        }
        if self.corpus_revision_ref != dataset.corpus_revision_ref {
            return Err(OalcInputError::RevisionMismatch);
        }
        if self.source != demand.source {
            return Err(OalcInputError::WrongSource);
        }
        if self.jurisdiction != demand.jurisdiction {
            return Err(OalcInputError::WrongJurisdiction);
        }
        if self.document_type != demand.document_type {
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

/// Optional offline implementation input. This is deliberately NOT required by
/// LegalFollow or by the parser contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineOalcJsonlBackend {
    pub corpus_jsonl_path: PathBuf,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pinned() -> PinnedOalcDatasetSelection {
        PinnedOalcDatasetSelection {
            dataset_id: OALC_DATASET_ID.into(),
            config: OALC_CONFIG.into(),
            split: OALC_SPLIT.into(),
            corpus_revision_ref: "isaacus/open-australian-legal-corpus@deadbeef".into(),
        }
    }

    #[test]
    fn local_corpus_path_is_not_part_of_public_input_contract() {
        let input = pinned();
        assert!(input.validate().is_ok());
    }

    #[test]
    fn mutable_revision_alias_is_rejected() {
        let mut input = pinned();
        input.corpus_revision_ref = "latest".into();
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
