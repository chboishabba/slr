//! Revision-pinned native Rust acquisition for the Open Australian Legal Corpus.
//!
//! This is the correctness-first fallback lane modelled by the Agda OALC
//! contracts: Rust owns acquisition; Python is not a provider dependency; the
//! full corpus is never retained locally.  The client streams `corpus.jsonl`
//! from a full Hugging Face commit SHA, selects an exact governed row, and
//! returns only that row's canonical text to the existing cache/materialisation
//! layer.
//!
//! Native projected Parquet scanning can replace this implementation as an
//! optimisation.  It must preserve the same immutable revision/exact-row
//! receipt contract and may not turn partial-shard failure into source absence.

use std::io::{BufRead, BufReader};
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;
use thiserror::Error;

use crate::{
    AcquiredSourceBundle, CacheFirstAcquirer, CacheLookupDemand,
    ExactResolutionReceiptOwned, ResolvedExternalDocumentOwned, ResolutionPath,
    TemporalCoverage,
};

pub const OALC_HF_REPO_ID: &str = "isaacus/open-australian-legal-corpus";
pub const OALC_HF_PROVIDER_REF: &str = "huggingface:oalc";
pub const OALC_HF_CORPUS_FILE: &str = "corpus.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcHfJsonlProfile {
    /// Full immutable Hugging Face commit SHA. Branch/tag names are rejected.
    pub dataset_revision_ref: String,
    pub expected_source_ref: String,
    pub expected_document_type_ref: String,
    /// Governed demand coordinates this provider profile is allowed to pay.
    pub expected_source_role_ref: String,
    pub expected_authority_level_ref: String,
    /// Optional exact OALC version_id when already known. If absent, the entire
    /// pinned stream is scanned to prove there is exactly one matching row.
    pub expected_version_id: Option<String>,
    /// Optional bearer token for private/rate-limited Hub access.
    pub bearer_token: Option<String>,
    pub connect_timeout_seconds: u64,
    pub read_timeout_seconds: u64,
}

impl OalcHfJsonlProfile {
    pub fn validate(&self) -> Result<(), OalcHfJsonlAcquireError> {
        if !is_full_git_sha(&self.dataset_revision_ref) {
            return Err(OalcHfJsonlAcquireError::MutableOrInvalidRevision(
                self.dataset_revision_ref.clone(),
            ));
        }
        for (name, value) in [
            ("expected_source_ref", self.expected_source_ref.as_str()),
            (
                "expected_document_type_ref",
                self.expected_document_type_ref.as_str(),
            ),
            ("expected_source_role_ref", self.expected_source_role_ref.as_str()),
            (
                "expected_authority_level_ref",
                self.expected_authority_level_ref.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(OalcHfJsonlAcquireError::EmptyProfileCoordinate(name));
            }
        }
        if let Some(version) = self.expected_version_id.as_deref() {
            if version.trim().is_empty() {
                return Err(OalcHfJsonlAcquireError::EmptyProfileCoordinate(
                    "expected_version_id",
                ));
            }
        }
        Ok(())
    }

    pub fn pinned_corpus_url(&self) -> Result<String, OalcHfJsonlAcquireError> {
        self.validate()?;
        Ok(format!(
            "https://huggingface.co/datasets/{OALC_HF_REPO_ID}/resolve/{}/{OALC_HF_CORPUS_FILE}",
            self.dataset_revision_ref
        ))
    }
}

#[derive(Debug, Error)]
pub enum OalcHfJsonlAcquireError {
    #[error("OALC Hugging Face revision must be a full 40-hex immutable commit SHA: {0}")]
    MutableOrInvalidRevision(String),
    #[error("OALC provider profile coordinate is empty: {0}")]
    EmptyProfileCoordinate(&'static str),
    #[error("governed source demand does not match this OALC provider profile: {0}")]
    DemandProfileMismatch(&'static str),
    #[error("Hugging Face request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("failed while streaming OALC corpus: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid OALC JSONL row at line {line}: {source}")]
    InvalidJson {
        line: u64,
        #[source]
        source: serde_json::Error,
    },
    #[error("pinned OALC corpus contains no exact row for the governed demand")]
    ExactRowNotFound,
    #[error("pinned OALC corpus contains more than one exact row for the governed demand")]
    AmbiguousExactRow,
    #[error("resolved OALC row has an empty canonical text payload")]
    EmptyCanonicalText,
}

#[derive(Debug, Deserialize)]
struct OalcJsonlRow {
    version_id: String,
    #[serde(rename = "type")]
    document_type: String,
    jurisdiction: String,
    source: String,
    mime: String,
    date: Option<String>,
    citation: String,
    url: Option<String>,
    text: String,
}

#[derive(Debug)]
pub struct OalcHfJsonlAcquirer {
    profile: OalcHfJsonlProfile,
    client: Client,
}

impl OalcHfJsonlAcquirer {
    pub fn new(profile: OalcHfJsonlProfile) -> Result<Self, OalcHfJsonlAcquireError> {
        profile.validate()?;
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(profile.connect_timeout_seconds.max(1)))
            .read_timeout(Duration::from_secs(profile.read_timeout_seconds.max(1)))
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent("SensibLaw-SLR/source-materialisation-1")
            .build()?;
        Ok(Self { profile, client })
    }

    pub fn profile(&self) -> &OalcHfJsonlProfile {
        &self.profile
    }

    fn exact_row(
        &self,
        demand: &CacheLookupDemand<'_>,
    ) -> Result<OalcJsonlRow, OalcHfJsonlAcquireError> {
        validate_demand_against_profile(demand, &self.profile)?;
        let url = self.profile.pinned_corpus_url()?;
        let mut request = self.client.get(url);
        if let Some(token) = self.profile.bearer_token.as_deref() {
            request = request.bearer_auth(token);
        }
        let response = request.send()?.error_for_status()?;
        let mut matched: Option<OalcJsonlRow> = None;
        let reader = BufReader::new(response);
        for (index, line) in reader.lines().enumerate() {
            let line_no = u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1);
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let row: OalcJsonlRow = serde_json::from_str(&line).map_err(|source| {
                OalcHfJsonlAcquireError::InvalidJson {
                    line: line_no,
                    source,
                }
            })?;
            if !row_matches(demand, &self.profile, &row) {
                continue;
            }
            if matched.is_some() {
                return Err(OalcHfJsonlAcquireError::AmbiguousExactRow);
            }
            matched = Some(row);

            // A configured version_id is itself an exact row identity, so no
            // second row can legitimately satisfy the same full identity.
            if self.profile.expected_version_id.is_some() {
                break;
            }
        }
        matched.ok_or(OalcHfJsonlAcquireError::ExactRowNotFound)
    }
}

impl CacheFirstAcquirer for OalcHfJsonlAcquirer {
    type Error = OalcHfJsonlAcquireError;

    fn acquire(
        &mut self,
        demand: &CacheLookupDemand<'_>,
    ) -> Result<AcquiredSourceBundle, Self::Error> {
        let row = self.exact_row(demand)?;
        if row.text.trim().is_empty() {
            return Err(OalcHfJsonlAcquireError::EmptyCanonicalText);
        }
        let source_url = row.url.clone();
        let temporal_coverage = match demand.temporal_ref {
            Some(requested) if row.date.as_deref() == Some(requested) => {
                TemporalCoverage::HistoricallyVerified
            }
            _ => TemporalCoverage::LatestKnownOnly,
        };
        let resolution_evidence_ref = format!(
            "hf:oalc:{}:{}",
            self.profile.dataset_revision_ref, row.version_id
        );
        Ok(AcquiredSourceBundle {
            document: ResolvedExternalDocumentOwned {
                provider_ref: OALC_HF_PROVIDER_REF.into(),
                dataset_ref: OALC_HF_REPO_ID.into(),
                dataset_revision_ref: self.profile.dataset_revision_ref.clone(),
                external_version_ref: row.version_id,
                citation: row.citation,
                source_ref: row.source,
                jurisdiction_ref: row.jurisdiction,
                document_type_ref: row.document_type,
                temporal_coverage,
                resolution_path: ResolutionPath::RevisionPinnedStreamingLegacy,
                source_url,
                canonical_text: row.text,
            },
            receipt: ExactResolutionReceiptOwned {
                demand_ref: demand.demand_ref.to_owned(),
                consumer_ref: None,
                requested_citation: demand.citation.to_owned(),
                requested_jurisdiction_ref: demand.jurisdiction_ref.to_owned(),
                requested_source_role_ref: demand.source_role_ref.to_owned(),
                requested_authority_level_ref: demand.authority_level_ref.to_owned(),
                requested_temporal_ref: demand.temporal_ref.map(str::to_owned),
                exact_demand_match: true,
                acquisition_authority_ref: format!(
                    "hf:oalc:pinned-revision:{}",
                    self.profile.dataset_revision_ref
                ),
                receipt_authority_ref: "source-observation-only".into(),
                network_request_count: 1,
                resolver_ref: "slr-rust:oalc-hf-jsonl-stream:v1".into(),
                resolution_evidence_ref,
            },
        })
    }
}

fn validate_demand_against_profile(
    demand: &CacheLookupDemand<'_>,
    profile: &OalcHfJsonlProfile,
) -> Result<(), OalcHfJsonlAcquireError> {
    if demand.source_role_ref != profile.expected_source_role_ref {
        return Err(OalcHfJsonlAcquireError::DemandProfileMismatch(
            "source_role_ref",
        ));
    }
    if demand.authority_level_ref != profile.expected_authority_level_ref {
        return Err(OalcHfJsonlAcquireError::DemandProfileMismatch(
            "authority_level_ref",
        ));
    }
    if demand.citation.trim().is_empty() {
        return Err(OalcHfJsonlAcquireError::DemandProfileMismatch("citation"));
    }
    if demand.jurisdiction_ref.trim().is_empty() {
        return Err(OalcHfJsonlAcquireError::DemandProfileMismatch(
            "jurisdiction_ref",
        ));
    }
    Ok(())
}

fn row_matches(
    demand: &CacheLookupDemand<'_>,
    profile: &OalcHfJsonlProfile,
    row: &OalcJsonlRow,
) -> bool {
    row.citation == demand.citation
        && row.jurisdiction == demand.jurisdiction_ref
        && row.source == profile.expected_source_ref
        && row.document_type == profile.expected_document_type_ref
        && profile
            .expected_version_id
            .as_deref()
            .map(|expected| row.version_id == expected)
            .unwrap_or(true)
}

fn is_full_git_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> OalcHfJsonlProfile {
        OalcHfJsonlProfile {
            dataset_revision_ref: "0123456789abcdef0123456789abcdef01234567".into(),
            expected_source_ref: "austlii".into(),
            expected_document_type_ref: "case_law".into(),
            expected_source_role_ref: "primary_case_law".into(),
            expected_authority_level_ref: "high_court".into(),
            expected_version_id: Some("austlii:1992/HCA/23".into()),
            bearer_token: None,
            connect_timeout_seconds: 30,
            read_timeout_seconds: 300,
        }
    }

    #[test]
    fn hf_revision_is_full_commit_sha_only() {
        for invalid in [
            "main",
            "v5.0.0",
            "refs/convert/parquet",
            "0123456",
            "",
        ] {
            let mut value = profile();
            value.dataset_revision_ref = invalid.into();
            assert!(matches!(
                value.validate(),
                Err(OalcHfJsonlAcquireError::MutableOrInvalidRevision(_))
            ));
        }
        assert!(profile().validate().is_ok());
    }

    #[test]
    fn pinned_url_contains_full_revision_not_latest() {
        let value = profile();
        let url = value.pinned_corpus_url().unwrap();
        assert!(url.contains(&value.dataset_revision_ref));
        assert!(!url.contains("/resolve/main/"));
    }

    #[test]
    fn exact_row_match_includes_provider_source_and_document_type() {
        let demand = CacheLookupDemand {
            demand_ref: "demand:mabo",
            citation: "Mabo v Queensland (No 2) [1992] HCA 23",
            jurisdiction_ref: "commonwealth",
            source_role_ref: "primary_case_law",
            authority_level_ref: "high_court",
            temporal_ref: None,
        };
        let row = OalcJsonlRow {
            version_id: "austlii:1992/HCA/23".into(),
            document_type: "case_law".into(),
            jurisdiction: "commonwealth".into(),
            source: "austlii".into(),
            mime: "text/html".into(),
            date: Some("1992-06-03".into()),
            citation: demand.citation.into(),
            url: None,
            text: "exact".into(),
        };
        assert!(row_matches(&demand, &profile(), &row));
    }
}
