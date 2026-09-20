//! Generic governed OALC source contract.
//!
//! This module generalises the earlier Cullen legislation-only contract so the
//! same revision-pinned OALC acquisition seam can carry both legislation and
//! case law.  OALC remains an acquisition substrate: receipts are candidate
//! source observations and never create legal authority, applicability, claim
//! truth, or citation-use classification.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use sensiblaw_legal_follow_plan::{
    ExactCaseLawSourceDemand, ExactLegislationSourceDemand, SourceRole,
    OALC_DATASET_ID as LEGAL_FOLLOW_OALC_DATASET_ID,
};

pub const OALC_DATASET_ID: &str = "isaacus/open-australian-legal-corpus";
pub const OALC_CONFIG: &str = "corpus";
pub const OALC_SPLIT: &str = "corpus";
pub const OALC_RECEIPT_AUTHORITY: &str = "experimental_candidate_only";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OalcDocumentKind {
    CaseLaw,
    Legislation,
}

impl OalcDocumentKind {
    pub const fn expected_oalc_type(self) -> &'static str {
        match self {
            Self::CaseLaw => "decision",
            Self::Legislation => "primary_legislation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OalcTemporalCoverage {
    LatestKnownOnly,
    DecisionDateAnchored,
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
pub struct OalcSourceDemand {
    pub demand_ref: String,
    pub origin_ref: String,
    pub citation: String,
    pub jurisdiction_ref: String,
    pub court_ref: Option<String>,
    pub document_kind: OalcDocumentKind,
    pub expected_source_role: SourceRole,
    pub requested_temporal_ref: Option<String>,
    pub dataset_ref: String,
    pub provider_profile_ref: String,
    pub authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OalcSourceContractError {
    MissingRevision,
    MutableRevisionAlias,
    WrongDataset,
    WrongConfig,
    WrongSplit,
    EmptyDemandRef,
    EmptyOriginRef,
    EmptyCitation,
    EmptyJurisdiction,
    EmptyCourt,
    WrongSourceRole,
    WrongDocumentType,
    WrongCitation,
    WrongJurisdiction,
    WrongCourt,
    EmptyVersionId,
    EmptyDigest,
    MissingArtifact,
    RevisionMismatch,
    AuthorityPromotion,
}

fn looks_mutable_revision_alias(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "main" | "master" | "latest" | "head" | "current" | "oalc:revision-unset"
    )
}

impl PinnedOalcDatasetSelection {
    pub fn validate(&self) -> Result<(), OalcSourceContractError> {
        if self.dataset_id != OALC_DATASET_ID
            || self.dataset_id != LEGAL_FOLLOW_OALC_DATASET_ID
        {
            return Err(OalcSourceContractError::WrongDataset);
        }
        if self.config != OALC_CONFIG {
            return Err(OalcSourceContractError::WrongConfig);
        }
        if self.split != OALC_SPLIT {
            return Err(OalcSourceContractError::WrongSplit);
        }
        if self.corpus_revision_ref.trim().is_empty() {
            return Err(OalcSourceContractError::MissingRevision);
        }
        if looks_mutable_revision_alias(&self.corpus_revision_ref) {
            return Err(OalcSourceContractError::MutableRevisionAlias);
        }
        Ok(())
    }
}

impl OalcSourceDemand {
    pub fn from_case_law(exact: &ExactCaseLawSourceDemand) -> Result<Self, OalcSourceContractError> {
        if exact.source_role != SourceRole::PrimaryCaseLaw {
            return Err(OalcSourceContractError::WrongSourceRole);
        }
        if exact.citation.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyCitation);
        }
        if exact.jurisdiction_ref.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyJurisdiction);
        }
        if exact.court_ref.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyCourt);
        }
        Ok(Self {
            demand_ref: exact.demand_ref.clone(),
            origin_ref: exact.origin_ref.clone(),
            citation: exact.citation.clone(),
            jurisdiction_ref: exact.jurisdiction_ref.clone(),
            court_ref: Some(exact.court_ref.clone()),
            document_kind: OalcDocumentKind::CaseLaw,
            expected_source_role: SourceRole::PrimaryCaseLaw,
            requested_temporal_ref: exact.requested_temporal_ref.clone(),
            dataset_ref: exact.dataset_ref.clone(),
            provider_profile_ref: exact.provider_profile_ref.clone(),
            authority: exact.authority,
        })
    }

    pub fn from_legislation(
        exact: &ExactLegislationSourceDemand,
    ) -> Result<Self, OalcSourceContractError> {
        if exact.source_role != SourceRole::PrimaryLegislation {
            return Err(OalcSourceContractError::WrongSourceRole);
        }
        if exact.citation.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyCitation);
        }
        if exact.jurisdiction_ref.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyJurisdiction);
        }
        Ok(Self {
            demand_ref: exact.demand_ref.clone(),
            origin_ref: exact.origin_ref.clone(),
            citation: exact.citation.clone(),
            jurisdiction_ref: exact.jurisdiction_ref.clone(),
            court_ref: None,
            document_kind: OalcDocumentKind::Legislation,
            expected_source_role: SourceRole::PrimaryLegislation,
            requested_temporal_ref: exact.requested_temporal_ref.clone(),
            dataset_ref: exact.dataset_ref.clone(),
            provider_profile_ref: exact.provider_profile_ref.clone(),
            authority: exact.authority,
        })
    }

    pub fn validate(&self) -> Result<(), OalcSourceContractError> {
        if self.demand_ref.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyDemandRef);
        }
        if self.origin_ref.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyOriginRef);
        }
        if self.citation.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyCitation);
        }
        if self.jurisdiction_ref.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyJurisdiction);
        }
        match (self.document_kind, self.expected_source_role) {
            (OalcDocumentKind::CaseLaw, SourceRole::PrimaryCaseLaw)
            | (OalcDocumentKind::Legislation, SourceRole::PrimaryLegislation) => {}
            _ => return Err(OalcSourceContractError::WrongSourceRole),
        }
        if self.document_kind == OalcDocumentKind::CaseLaw {
            match self.court_ref.as_deref() {
                Some(court) if !court.trim().is_empty() => {}
                _ => return Err(OalcSourceContractError::EmptyCourt),
            }
        }
        if self.dataset_ref != OALC_DATASET_ID {
            return Err(OalcSourceContractError::WrongDataset);
        }
        if self.authority != "acquisition_plan_only" {
            return Err(OalcSourceContractError::AuthorityPromotion);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OalcResolvedSourceReceipt {
    pub demand_ref: String,
    pub origin_ref: String,
    pub citation: String,
    pub version_id: String,
    pub corpus_revision_ref: String,
    pub source: String,
    pub jurisdiction: String,
    pub document_type: String,
    pub court: Option<String>,
    pub date: Option<String>,
    pub canonical_url: Option<String>,
    pub when_scraped: Option<String>,
    pub canonical_text_digest: String,
    pub local_artifact_ref: PathBuf,
    pub temporal_coverage: OalcTemporalCoverage,
    pub resolution_path: String,
    pub network_requests: u64,
    pub receipt_authority: String,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_claim_truth: bool,
}

impl OalcResolvedSourceReceipt {
    pub fn validate_against(
        &self,
        dataset: &PinnedOalcDatasetSelection,
        demand: &OalcSourceDemand,
    ) -> Result<(), OalcSourceContractError> {
        dataset.validate()?;
        demand.validate()?;
        if self.demand_ref != demand.demand_ref || self.origin_ref != demand.origin_ref {
            return Err(OalcSourceContractError::WrongCitation);
        }
        if self.citation != demand.citation {
            return Err(OalcSourceContractError::WrongCitation);
        }
        if self.corpus_revision_ref != dataset.corpus_revision_ref {
            return Err(OalcSourceContractError::RevisionMismatch);
        }
        if self.jurisdiction.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyJurisdiction);
        }
        if self.document_type != demand.document_kind.expected_oalc_type() {
            return Err(OalcSourceContractError::WrongDocumentType);
        }
        if self.version_id.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyVersionId);
        }
        if self.canonical_text_digest.trim().is_empty() {
            return Err(OalcSourceContractError::EmptyDigest);
        }
        if self.local_artifact_ref.as_os_str().is_empty() {
            return Err(OalcSourceContractError::MissingArtifact);
        }
        if self.receipt_authority != OALC_RECEIPT_AUTHORITY
            || !self.candidate_only
            || self.creates_legal_authority
            || self.creates_claim_truth
        {
            return Err(OalcSourceContractError::AuthorityPromotion);
        }
        Ok(())
    }

    pub const fn is_semantic_payment(&self) -> bool {
        false
    }

    pub const fn is_authority_receipt(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OalcFilterIndexState {
    Complete,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OalcExactLookupDisposition<T> {
    Found(T),
    CompleteIndexAbsent,
    RequireRevisionPinnedStreaming,
    Ambiguous(usize),
}

pub fn classify_exact_filter<T>(
    rows: Vec<T>,
    index_state: OalcFilterIndexState,
) -> OalcExactLookupDisposition<T> {
    match rows.len() {
        1 => OalcExactLookupDisposition::Found(rows.into_iter().next().expect("length checked")),
        0 if index_state == OalcFilterIndexState::Partial => {
            OalcExactLookupDisposition::RequireRevisionPinnedStreaming
        }
        0 => OalcExactLookupDisposition::CompleteIndexAbsent,
        count => OalcExactLookupDisposition::Ambiguous(count),
    }
}

pub fn oalc_filter_predicate(demand: &OalcSourceDemand) -> Result<String, OalcSourceContractError> {
    demand.validate()?;
    let citation = demand.citation.replace('\'', "''");
    let kind = demand.document_kind.expected_oalc_type().replace('\'', "''");
    match demand.document_kind {
        OalcDocumentKind::CaseLaw => Ok(format!(
            "\"citation\" LIKE '%{citation}%' AND \"type\"='{kind}'"
        )),
        OalcDocumentKind::Legislation => Ok(format!(
            "\"citation\"='{citation}' AND \"type\"='{kind}'"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_legal_follow_plan::{
        exact_oalc_case_law_demand, exact_oalc_legislation_demand, plan_legal_sources,
        AuthorityLevel, LegalSourceDemand, SourceRole, OALC_PROVIDER_PROFILE,
    };

    fn case_exact() -> ExactCaseLawSourceDemand {
        let demand = LegalSourceDemand {
            demand_ref: "demand:waltons".into(),
            origin_ref: "doctrine:au:contract:estoppel".into(),
            jurisdiction_ref: Some("AU".into()),
            source_roles: vec![SourceRole::PrimaryCaseLaw],
            authority_levels: vec![AuthorityLevel::Official],
            provider_profile_refs: vec![OALC_PROVIDER_PROFILE.into()],
            requested_facets: vec!["case.full_text".into()],
            temporal_refs: vec!["as_at:2026-09-20".into()],
            provenance_refs: vec!["fixture".into()],
            priority: 100,
        };
        let plan = plan_legal_sources(&demand, &[]);
        exact_oalc_case_law_demand(
            &plan,
            &demand.origin_ref,
            "[1988] HCA 7",
            "court:HCA",
        )
        .unwrap()
    }

    fn legislation_exact() -> ExactLegislationSourceDemand {
        let demand = LegalSourceDemand {
            demand_ref: "demand:qld:s68".into(),
            origin_ref: "doctrine:au:contract:privity".into(),
            jurisdiction_ref: Some("AU-QLD".into()),
            source_roles: vec![SourceRole::PrimaryLegislation],
            authority_levels: vec![AuthorityLevel::Official],
            provider_profile_refs: vec![OALC_PROVIDER_PROFILE.into()],
            requested_facets: vec!["legislation.text".into()],
            temporal_refs: vec!["as_at:2026-09-20".into()],
            provenance_refs: vec!["fixture".into()],
            priority: 100,
        };
        let plan = plan_legal_sources(&demand, &[]);
        exact_oalc_legislation_demand(
            &plan,
            &demand.origin_ref,
            "Property Law Act 2023 (Qld)",
        )
        .unwrap()
    }

    #[test]
    fn one_provider_contract_accepts_case_law_and_legislation() {
        let case = OalcSourceDemand::from_case_law(&case_exact()).unwrap();
        let statute = OalcSourceDemand::from_legislation(&legislation_exact()).unwrap();
        assert_eq!(case.document_kind, OalcDocumentKind::CaseLaw);
        assert_eq!(statute.document_kind, OalcDocumentKind::Legislation);
        assert_eq!(case.expected_source_role, SourceRole::PrimaryCaseLaw);
        assert_eq!(statute.expected_source_role, SourceRole::PrimaryLegislation);
    }

    #[test]
    fn partial_zero_result_requires_revision_pinned_streaming() {
        let result: OalcExactLookupDisposition<()> =
            classify_exact_filter(Vec::new(), OalcFilterIndexState::Partial);
        assert_eq!(
            result,
            OalcExactLookupDisposition::RequireRevisionPinnedStreaming
        );
    }

    #[test]
    fn complete_zero_result_is_source_absence_not_negative_legal_evidence() {
        let result: OalcExactLookupDisposition<()> =
            classify_exact_filter(Vec::new(), OalcFilterIndexState::Complete);
        assert_eq!(result, OalcExactLookupDisposition::CompleteIndexAbsent);
    }

    #[test]
    fn case_filter_is_document_type_generic() {
        let demand = OalcSourceDemand::from_case_law(&case_exact()).unwrap();
        let predicate = oalc_filter_predicate(&demand).unwrap();
        assert!(predicate.contains("[1988] HCA 7"));
        assert!(predicate.contains("\"type\"='decision'"));
        assert!(predicate.contains("LIKE"));
    }

    #[test]
    fn legislation_filter_is_document_type_generic() {
        let demand = OalcSourceDemand::from_legislation(&legislation_exact()).unwrap();
        let predicate = oalc_filter_predicate(&demand).unwrap();
        assert!(predicate.contains("Property Law Act 2023 (Qld)"));
        assert!(predicate.contains("\"type\"='primary_legislation'"));
    }
}
