//! Native governed OALC case-law acquisition.
//!
//! This module owns the reusable live acquisition path previously embedded in
//! an example binary.  It preserves the same authority firewall: successful
//! acquisition yields candidate-only source material and never legal truth.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OalcCitationMatch {
    Exact,
    Contains,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedOalcStreamRequest {
    pub revision: String,
    pub citation: String,
    pub citation_match: OalcCitationMatch,
    pub document_type: String,
    pub source: Option<String>,
    pub jurisdiction: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OalcCorpusRow {
    pub version_id: String,
    #[serde(rename = "type")]
    pub document_type: String,
    pub jurisdiction: String,
    pub source: String,
    pub citation: String,
    #[serde(default)]
    pub mime: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub when_scraped: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcCaseFollowRequest {
    pub citation: String,
    pub court_ref: String,
    pub oalc_jurisdiction: String,
    pub legal_jurisdiction: String,
    pub output_dir: PathBuf,
    pub as_at: String,
}

impl OalcCaseFollowRequest {
    pub fn for_citation(citation: impl Into<String>, output_dir: impl Into<PathBuf>) -> Self {
        let citation = citation.into();
        let court_token = citation
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_ascii_uppercase();
        let court_ref = match court_token.as_str() {
            "HCA" => "court:HCA",
            "FCA" | "FCAFC" => "court:FCA",
            "NSWCA" => "court:NSWCA",
            "VSCA" => "court:VSCA",
            "QCA" => "court:QCA",
            "WASCA" => "court:WASCA",
            _ => "court:unknown",
        }
        .to_string();
        let oalc_jurisdiction = match court_token.as_str() {
            "HCA" | "FCA" | "FCAFC" => "commonwealth",
            "NSWCA" => "new_south_wales",
            "VSCA" => "victoria",
            "QCA" => "queensland",
            "WASCA" => "western_australia",
            _ => "",
        }
        .to_string();
        Self {
            citation,
            court_ref,
            oalc_jurisdiction,
            legal_jurisdiction: "AU".into(),
            output_dir: output_dir.into(),
            as_at: "2026-09-20".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcCaseFollowRunReceipt {
    pub source_receipt_path: PathBuf,
    pub canonical_text_path: PathBuf,
    pub citation: String,
    pub corpus_revision_ref: String,
    pub version_id: String,
    pub resolution_path: String,
    pub network_requests: u64,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OalcCaseFollowError {
    LiveNetworkFeatureDisabled,
    InvalidRequest(String),
    Governance(String),
    Provider(String),
    Dataset(String),
    SourceResidual(String),
    Validation(String),
    Io(String),
    Json(String),
    StreamingFallback(String),
}

#[cfg(feature = "live-network")]
mod live {
    use super::{OalcCaseFollowError, OalcCaseFollowRequest, OalcCaseFollowRunReceipt};
    use crate::{
        classify_exact_filter, classify_http_status, oalc_filter_predicate,
        GovernedExecutionContext, HttpRequest, HttpResponse, HttpTransport, LiveGovernanceBounds,
        OalcDocumentKind, OalcExactLookupDisposition, OalcFilterIndexState,
        OalcResolvedSourceReceipt, OalcSourceDemand, OalcTemporalCoverage,
        PinnedOalcDatasetSelection, ProviderAccessStatus, UreqTransport, OALC_CONFIG,
        OALC_DATASET_ID, OALC_RECEIPT_AUTHORITY, OALC_SPLIT, SENSIBLAW_UA,
    };
    use sensiblaw_legal_follow_plan::{
        exact_oalc_case_law_demand, plan_legal_sources, AuthorityLevel, LegalSourceDemand,
        SourceRole, OALC_PROVIDER_PROFILE,
    };
    use serde::Deserialize;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::io::{BufRead, BufReader};
    use std::path::Path;
    use std::thread;
    use std::time::{Duration, Instant};

    const HF_DATASET_API: &str =
        "https://huggingface.co/api/datasets/isaacus/open-australian-legal-corpus";
    const HF_FILTER_API: &str = "https://datasets-server.huggingface.co/filter";

    #[derive(Debug, Deserialize)]
    struct DatasetInfo {
        sha: String,
    }

    #[derive(Debug, Deserialize)]
    struct FilterResponse {
        rows: Vec<FilterRow>,
        #[serde(default)]
        partial: bool,
    }

    #[derive(Debug, Deserialize)]
    struct FilterRow {
        row: OalcRow,
    }

    #[derive(Debug, Clone, Deserialize)]
    struct OalcRow {
        version_id: String,
        #[serde(rename = "type")]
        document_type: String,
        jurisdiction: String,
        source: String,
        citation: String,
        #[serde(default)]
        date: Option<String>,
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        when_scraped: Option<String>,
        text: String,
    }

    struct GovernedOalc<T> {
        transport: T,
        context: GovernedExecutionContext,
        last_request: Option<Instant>,
        requests: u64,
    }

    impl<T> GovernedOalc<T>
    where
        T: HttpTransport,
        T::Error: ToString,
    {
        fn get(&mut self, url: &str) -> Result<HttpResponse, OalcCaseFollowError> {
            if self.requests >= self.context.bounds.max_network_requests {
                return Err(OalcCaseFollowError::Governance(
                    "OALC request budget exceeded".into(),
                ));
            }
            if let Some(last) = self.last_request {
                let minimum = Duration::from_secs(self.context.bounds.minimum_pacing_seconds);
                let elapsed = last.elapsed();
                if elapsed < minimum {
                    thread::sleep(minimum - elapsed);
                }
            }
            let response = self
                .transport
                .get(&HttpRequest {
                    url: url.into(),
                    user_agent: SENSIBLAW_UA.into(),
                    referer: Some("https://huggingface.co/".into()),
                    timeout_seconds: 60,
                })
                .map_err(|error| OalcCaseFollowError::Provider(error.to_string()))?;
            self.requests += 1;
            self.last_request = Some(Instant::now());
            let status = classify_http_status(response.status_code);
            if status != ProviderAccessStatus::Available {
                return Err(OalcCaseFollowError::Provider(format!(
                    "OALC provider unavailable: {status:?}"
                )));
            }
            Ok(response)
        }
    }

    fn encode(value: &str) -> String {
        let mut out = String::new();
        for byte in value.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(byte as char)
                }
                other => {
                    const HEX: &[u8; 16] = b"0123456789ABCDEF";
                    out.push('%');
                    out.push(HEX[(other >> 4) as usize] as char);
                    out.push(HEX[(other & 0x0f) as usize] as char);
                }
            }
        }
        out
    }

    fn sha256(bytes: &[u8]) -> String {
        format!("sha256:{:x}", Sha256::digest(bytes))
    }

    fn readonly_write(path: &Path, text: &str) -> Result<(), OalcCaseFollowError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
        }
        fs::write(path, text.as_bytes())
            .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
        let mut permissions = fs::metadata(path)
            .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(path, permissions)
            .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
        Ok(())
    }

    fn validate_request(request: &OalcCaseFollowRequest) -> Result<(), OalcCaseFollowError> {
        for (label, value) in [
            ("citation", request.citation.as_str()),
            ("court_ref", request.court_ref.as_str()),
            ("legal_jurisdiction", request.legal_jurisdiction.as_str()),
            ("as_at", request.as_at.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(OalcCaseFollowError::InvalidRequest(format!(
                    "{label} must not be empty"
                )));
            }
        }
        if request.output_dir.as_os_str().is_empty() {
            return Err(OalcCaseFollowError::InvalidRequest(
                "output_dir must not be empty".into(),
            ));
        }
        Ok(())
    }

    fn case_demand(
        request: &OalcCaseFollowRequest,
    ) -> Result<OalcSourceDemand, OalcCaseFollowError> {
        let demand = LegalSourceDemand {
            demand_ref: format!("legal-follow:case:{}", request.citation),
            origin_ref: "legal-follow:cited-by-treatment".into(),
            jurisdiction_ref: Some(request.legal_jurisdiction.clone()),
            source_roles: vec![SourceRole::PrimaryCaseLaw],
            authority_levels: vec![AuthorityLevel::Official],
            provider_profile_refs: vec![OALC_PROVIDER_PROFILE.into()],
            requested_facets: vec![
                "case.full_text".into(),
                "case.citation_graph".into(),
                "case.treatment".into(),
            ],
            temporal_refs: vec![format!("as_at:{}", request.as_at)],
            provenance_refs: vec!["native-oalc-case-follow".into()],
            priority: 100,
        };
        let plan = plan_legal_sources(&demand, &[]);
        let exact = exact_oalc_case_law_demand(
            &plan,
            &demand.origin_ref,
            &request.citation,
            &request.court_ref,
        )
        .map_err(|error| OalcCaseFollowError::Validation(format!("{error:?}")))?;
        OalcSourceDemand::from_case_law(&exact)
            .map_err(|error| OalcCaseFollowError::Validation(format!("{error:?}")))
    }

    fn stream_fallback(
        provider: &mut GovernedOalc<UreqTransport>,
        request: &OalcCaseFollowRequest,
        revision: &str,
    ) -> Result<OalcRow, OalcCaseFollowError> {
        if provider.requests >= provider.context.bounds.max_network_requests {
            return Err(OalcCaseFollowError::Governance(
                "OALC request budget exceeded before pinned stream".into(),
            ));
        }
        if let Some(last) = provider.last_request {
            let minimum = Duration::from_secs(provider.context.bounds.minimum_pacing_seconds);
            let elapsed = last.elapsed();
            if elapsed < minimum {
                thread::sleep(minimum - elapsed);
            }
        }

        let url = format!(
            "https://huggingface.co/datasets/{OALC_DATASET_ID}/resolve/{revision}/corpus.jsonl?download=true"
        );
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(120))
            .build();
        let response = match agent
            .get(&url)
            .set("User-Agent", SENSIBLAW_UA)
            .set("Referer", "https://huggingface.co/")
            .call()
        {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(error) => {
                return Err(OalcCaseFollowError::StreamingFallback(error.to_string()))
            }
        };
        provider.requests += 1;
        provider.last_request = Some(Instant::now());

        let status = classify_http_status(response.status());
        if status != ProviderAccessStatus::Available {
            return Err(OalcCaseFollowError::StreamingFallback(format!(
                "pinned corpus stream unavailable: {status:?}"
            )));
        }

        let mut match_row: Option<OalcRow> = None;
        let reader = BufReader::new(response.into_reader());
        for line in reader.lines() {
            let line = line
                .map_err(|error| OalcCaseFollowError::StreamingFallback(error.to_string()))?;
            let row: OalcRow = serde_json::from_str(&line)
                .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?;
            if row.document_type != "decision"
                || !row.citation.contains(&request.citation)
                || (!request.oalc_jurisdiction.is_empty()
                    && row.jurisdiction != request.oalc_jurisdiction)
            {
                continue;
            }
            if match_row.is_some() {
                return Err(OalcCaseFollowError::SourceResidual(format!(
                    "revision-pinned corpus stream returned multiple candidates for {}",
                    request.citation
                )));
            }
            match_row = Some(row);
        }

        match_row.ok_or_else(|| {
            OalcCaseFollowError::SourceResidual(format!(
                "revision-pinned corpus stream completed with no {} decision",
                request.citation
            ))
        })
    }

    pub fn run(
        request: &OalcCaseFollowRequest,
    ) -> Result<OalcCaseFollowRunReceipt, OalcCaseFollowError> {
        validate_request(request)?;
        fs::create_dir_all(&request.output_dir)
            .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;

        let context = GovernedExecutionContext {
            operator_opt_in: true,
            cache_checked_first: true,
            persisted_receipts_checked_first: true,
            bounds: LiveGovernanceBounds {
                minimum_pacing_seconds: 4,
                burst: 1,
                max_depth: 1,
                max_new_documents: 1,
                max_network_requests: 3,
            },
        };
        context
            .validate()
            .map_err(|error| OalcCaseFollowError::Governance(format!("{error:?}")))?;
        let mut provider = GovernedOalc {
            transport: UreqTransport,
            context,
            last_request: None,
            requests: 0,
        };

        let info: DatasetInfo = serde_json::from_slice(&provider.get(HF_DATASET_API)?.body)
            .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?;
        if info.sha.trim().is_empty() {
            return Err(OalcCaseFollowError::Dataset(
                "OALC metadata returned empty revision".into(),
            ));
        }
        let dataset = PinnedOalcDatasetSelection {
            dataset_id: OALC_DATASET_ID.into(),
            config: OALC_CONFIG.into(),
            split: OALC_SPLIT.into(),
            corpus_revision_ref: format!("{OALC_DATASET_ID}@{}", info.sha),
        };
        dataset
            .validate()
            .map_err(|error| OalcCaseFollowError::Dataset(format!("{error:?}")))?;

        let demand = case_demand(request)?;
        if demand.document_kind != OalcDocumentKind::CaseLaw {
            return Err(OalcCaseFollowError::Validation(
                "LegalFollow demand did not lower to case law".into(),
            ));
        }
        let predicate = oalc_filter_predicate(&demand)
            .map_err(|error| OalcCaseFollowError::Validation(format!("{error:?}")))?;
        let url = format!(
            "{HF_FILTER_API}?dataset={}&config={}&split={}&where={}&offset=0&length=2",
            encode(OALC_DATASET_ID),
            encode(OALC_CONFIG),
            encode(OALC_SPLIT),
            encode(&predicate)
        );
        let response: FilterResponse = serde_json::from_slice(&provider.get(&url)?.body)
            .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?;
        let index_state = if response.partial {
            OalcFilterIndexState::Partial
        } else {
            OalcFilterIndexState::Complete
        };
        let rows = response.rows.into_iter().map(|row| row.row).collect();
        let (record, resolution_path) = match classify_exact_filter(rows, index_state) {
            OalcExactLookupDisposition::Found(row) => (row, "filter_exact"),
            OalcExactLookupDisposition::RequireRevisionPinnedStreaming => {
                (
                    stream_fallback(&mut provider, request, &info.sha)?,
                    "revision_pinned_streaming",
                )
            }
            OalcExactLookupDisposition::CompleteIndexAbsent => {
                return Err(OalcCaseFollowError::SourceResidual(format!(
                    "complete OALC index found no {}",
                    request.citation
                )))
            }
            OalcExactLookupDisposition::Ambiguous(count) => {
                return Err(OalcCaseFollowError::SourceResidual(format!(
                    "OALC returned {count} candidates for {}",
                    request.citation
                )))
            }
        };

        if !record.citation.contains(&request.citation)
            || record.document_type != "decision"
            || record.text.trim().is_empty()
        {
            return Err(OalcCaseFollowError::Validation(
                "OALC result failed decision/citation/text validation".into(),
            ));
        }
        if !request.oalc_jurisdiction.is_empty()
            && record.jurisdiction != request.oalc_jurisdiction
        {
            return Err(OalcCaseFollowError::Validation(format!(
                "OALC jurisdiction mismatch: expected {} got {}",
                request.oalc_jurisdiction, record.jurisdiction
            )));
        }

        let text_path = request.output_dir.join("judgment.txt");
        readonly_write(&text_path, &record.text)?;
        let receipt = OalcResolvedSourceReceipt {
            demand_ref: demand.demand_ref.clone(),
            origin_ref: demand.origin_ref.clone(),
            citation: demand.citation.clone(),
            version_id: record.version_id,
            corpus_revision_ref: dataset.corpus_revision_ref.clone(),
            source: record.source,
            jurisdiction: record.jurisdiction,
            document_type: record.document_type,
            court: demand.court_ref.clone(),
            date: record.date,
            canonical_url: record.url,
            when_scraped: record.when_scraped,
            canonical_text_digest: sha256(record.text.as_bytes()),
            local_artifact_ref: text_path.clone(),
            temporal_coverage: OalcTemporalCoverage::DecisionDateAnchored,
            resolution_path: resolution_path.into(),
            network_requests: provider.requests,
            receipt_authority: OALC_RECEIPT_AUTHORITY.into(),
            candidate_only: true,
            creates_legal_authority: false,
            creates_claim_truth: false,
        };
        receipt
            .validate_against(&dataset, &demand)
            .map_err(|error| OalcCaseFollowError::Validation(format!("{error:?}")))?;

        let receipt_path = request.output_dir.join("oalc-source-receipt.json");
        fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&receipt)
                .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?,
        )
        .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;

        Ok(OalcCaseFollowRunReceipt {
            source_receipt_path: receipt_path,
            canonical_text_path: text_path,
            citation: request.citation.clone(),
            corpus_revision_ref: receipt.corpus_revision_ref,
            version_id: receipt.version_id,
            resolution_path: receipt.resolution_path,
            network_requests: receipt.network_requests,
            candidate_only: true,
            creates_legal_authority: false,
            creates_claim_truth: false,
        })
    }
}

#[cfg(feature = "live-network")]
pub use live::run as run_live_oalc_case_follow;

#[cfg(not(feature = "live-network"))]
pub fn run_live_oalc_case_follow(
    _request: &OalcCaseFollowRequest,
) -> Result<OalcCaseFollowRunReceipt, OalcCaseFollowError> {
    Err(OalcCaseFollowError::LiveNetworkFeatureDisabled)
}
