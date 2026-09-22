//! Native governed OALC case-law acquisition.
//!
//! This module owns the reusable live acquisition path previously embedded in
//! an example binary.  It preserves the same authority firewall: successful
//! acquisition yields candidate-only source material and never legal truth.

use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OalcCitationMatch {
    Exact,
    Contains,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OalcCaseAcquisitionMode {
    IndexedOnly,
    IndexedThenPinnedStream,
    IndexedThenPinnedRangeIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedOalcStreamReceipt {
    pub row: OalcCorpusRow,
    pub rows_examined: u64,
    pub bytes_read: u64,
    pub terminated_after_match: bool,
    pub uniqueness_exhaustively_verified: bool,
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
pub struct OalcExactSourceRequest {
    pub citation: String,
    pub citation_match: OalcCitationMatch,
    pub document_type: String,
    pub source: Option<String>,
    pub jurisdiction: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcExactSourceRunReceipt {
    pub corpus_revision_sha: String,
    pub row: OalcCorpusRow,
    pub resolution_path: String,
    pub network_requests: u64,
}

pub fn oalc_corpus_row_matches(request: &PinnedOalcStreamRequest, row: &OalcCorpusRow) -> bool {
    let citation_matches = match request.citation_match {
        OalcCitationMatch::Exact => row.citation == request.citation,
        // OALC case citations include a party-name prefix, while the medium-neutral
        // citation itself is the terminal coordinate.  A raw substring test makes
        // `[1988] HCA 7` spuriously match `[1988] HCA 72` and turns a unique
        // authority into a false ambiguity.
        OalcCitationMatch::Contains => row.citation.ends_with(&request.citation),
    };
    citation_matches
        && row.document_type == request.document_type
        && request
            .source
            .as_deref()
            .map_or(true, |source| row.source == source)
        && request
            .jurisdiction
            .as_deref()
            .map_or(true, |jurisdiction| row.jurisdiction == jurisdiction)
}

pub fn scan_pinned_oalc_jsonl<R: BufRead>(
    mut reader: R,
    request: &PinnedOalcStreamRequest,
) -> Result<PinnedOalcStreamReceipt, OalcCaseFollowError> {
    let mut line = String::new();
    let mut rows_examined = 0u64;
    let mut bytes_read = 0u64;

    loop {
        line.clear();
        let read = reader.read_line(&mut line).map_err(|error| {
            OalcCaseFollowError::StreamingFallback(format!(
                "pinned corpus stream interrupted after {rows_examined} rows and {bytes_read} bytes: {error}"
            ))
        })?;
        if read == 0 {
            return Err(OalcCaseFollowError::SourceResidual(format!(
                "revision-pinned corpus stream completed with no exact row for {} after {rows_examined} rows and {bytes_read} bytes",
                request.citation
            )));
        }

        rows_examined += 1;
        bytes_read = bytes_read.saturating_add(read as u64);
        let row: OalcCorpusRow = serde_json::from_str(line.trim_end()).map_err(|error| {
            OalcCaseFollowError::Json(format!("decode pinned corpus row {rows_examined}: {error}"))
        })?;
        if !oalc_corpus_row_matches(request, &row) {
            continue;
        }

        return Ok(PinnedOalcStreamReceipt {
            row,
            rows_examined,
            bytes_read,
            terminated_after_match: true,
            uniqueness_exhaustively_verified: false,
        });
    }
}

pub fn oalc_exact_source_filter_predicate(
    request: &OalcExactSourceRequest,
) -> Result<String, OalcCaseFollowError> {
    if request.citation.trim().is_empty() || request.document_type.trim().is_empty() {
        return Err(OalcCaseFollowError::InvalidRequest(
            "exact source requires citation and document_type".into(),
        ));
    }
    let citation = request.citation.replace('\'', "''");
    let document_type = request.document_type.replace('\'', "''");
    let citation_clause =
        match request.citation_match {
            OalcCitationMatch::Exact => format!("\"citation\"='{citation}'"),
            OalcCitationMatch::Contains => return Err(OalcCaseFollowError::InvalidRequest(
                "contains citation matching must use bounded datasets-server /search, not /filter"
                    .into(),
            )),
        };
    let mut clauses = vec![citation_clause, format!("\"type\"='{document_type}'")];
    if let Some(source) = request.source.as_deref() {
        clauses.push(format!("\"source\"='{}'", source.replace('\'', "''")));
    }
    if let Some(jurisdiction) = request.jurisdiction.as_deref() {
        clauses.push(format!(
            "\"jurisdiction\"='{}'",
            jurisdiction.replace('\'', "''")
        ));
    }
    Ok(clauses.join(" AND "))
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
            "FCA" => "court:FCA",
            "FCAFC" => "court:FCAFC",
            "NSWCA" => "court:NSWCA",
            "NSWSC" => "court:NSWSC",
            "VSCA" => "court:VSCA",
            "VSC" => "court:VSC",
            "QCA" => "court:QCA",
            "QSC" => "court:QSC",
            "WASCA" => "court:WASCA",
            "WASC" => "court:WASC",
            "SASCFC" => "court:SASCFC",
            "SASC" => "court:SASC",
            "TASFC" => "court:TASFC",
            "TASSC" => "court:TASSC",
            "ACTCA" => "court:ACTCA",
            "ACTSC" => "court:ACTSC",
            "NTCA" => "court:NTCA",
            "NTSC" => "court:NTSC",
            _ => "court:unknown",
        }
        .to_string();
        let oalc_jurisdiction = match court_token.as_str() {
            "HCA" | "FCA" | "FCAFC" => "commonwealth",
            "NSWCA" | "NSWSC" => "new_south_wales",
            "VSCA" | "VSC" => "victoria",
            "QCA" | "QSC" => "queensland",
            "WASCA" | "WASC" => "western_australia",
            "SASCFC" | "SASC" => "south_australia",
            "TASFC" | "TASSC" => "tasmania",
            "ACTCA" | "ACTSC" => "australian_capital_territory",
            "NTCA" | "NTSC" => "northern_territory",
            _ => "",
        }
        .to_string();
        let legal_jurisdiction = match court_token.as_str() {
            "HCA" | "FCA" | "FCAFC" => "AU",
            "NSWCA" | "NSWSC" => "AU-NSW",
            "VSCA" | "VSC" => "AU-VIC",
            "QCA" | "QSC" => "AU-QLD",
            "WASCA" | "WASC" => "AU-WA",
            "SASCFC" | "SASC" => "AU-SA",
            "TASFC" | "TASSC" => "AU-TAS",
            "ACTCA" | "ACTSC" => "AU-ACT",
            "NTCA" | "NTSC" => "AU-NT",
            _ => "AU",
        }
        .to_string();
        Self {
            citation,
            court_ref,
            oalc_jurisdiction,
            legal_jurisdiction,
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
    pub stream_rows_examined: Option<u64>,
    pub stream_bytes_read: Option<u64>,
    pub stream_terminated_after_match: Option<bool>,
    pub stream_uniqueness_exhaustively_verified: Option<bool>,
    pub range_index_hit: Option<bool>,
    pub range_index_requests: Option<u64>,
    pub range_index_rows_indexed_this_run: Option<u64>,
    pub range_index_bytes_indexed_this_run: Option<u64>,
    pub range_index_byte_start: Option<u64>,
    pub range_index_byte_len: Option<u64>,
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
    use super::{
        OalcCaseAcquisitionMode, OalcCaseFollowError, OalcCaseFollowRequest,
        OalcCaseFollowRunReceipt, OalcCitationMatch, OalcCorpusRow, OalcExactSourceRequest,
        OalcExactSourceRunReceipt, PinnedOalcStreamReceipt, PinnedOalcStreamRequest,
    };
    use crate::{
        classify_exact_filter, classify_http_status, GovernedExecutionContext, HttpRequest,
        HttpResponse, HttpTransport, LiveGovernanceBounds, OalcDocumentKind,
        lookup_or_build_oalc_range_index, OalcExactLookupDisposition, OalcFilterIndexState,
        OalcRangeLookupReceipt, OalcResolvedSourceReceipt, OalcSourceDemand,
        OalcTemporalCoverage, PinnedOalcDatasetSelection, ProviderAccessStatus, UreqTransport,
        OALC_CONFIG, OALC_DATASET_ID, OALC_RECEIPT_AUTHORITY, OALC_SPLIT, SENSIBLAW_UA,
    };
    use sensiblaw_legal_follow_plan::{
        exact_oalc_case_law_demand, plan_legal_sources, AuthorityLevel, LegalSourceDemand,
        SourceRole, OALC_PROVIDER_PROFILE,
    };
    use serde::Deserialize;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::io::BufReader;
    use std::path::Path;
    use std::thread;
    use std::time::{Duration, Instant};

    const HF_DATASET_API: &str =
        "https://huggingface.co/api/datasets/isaacus/open-australian-legal-corpus";
    const HF_FILTER_API: &str = "https://datasets-server.huggingface.co/filter";
    const HF_SEARCH_API: &str = "https://datasets-server.huggingface.co/search";
    // The pinned corpus is roughly 9.4 GB.  This is a whole-response deadline,
    // not an in-memory buffer: parsing remains line-by-line through BufReader.
    const PINNED_STREAM_TIMEOUT_SECONDS: u64 = 900;

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
        row: OalcCorpusRow,
    }

    struct GovernedOalc<T> {
        transport: T,
        context: GovernedExecutionContext,
        last_request: Option<Instant>,
        requests: u64,
    }

    fn bounded_response_detail(response: &HttpResponse) -> String {
        const MAX: usize = 512;
        let body = String::from_utf8_lossy(&response.body);
        let mut detail = body.chars().take(MAX).collect::<String>();
        if body.chars().count() > MAX {
            detail.push_str("…");
        }
        detail.replace(['\n', '\r'], " ")
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
                    "OALC provider unavailable: {status:?} (HTTP {} content-type={} body={:?})",
                    response.status_code,
                    response.content_type.as_deref().unwrap_or("unknown"),
                    bounded_response_detail(&response),
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

    fn stream_pinned_corpus(
        request: &PinnedOalcStreamRequest,
    ) -> Result<PinnedOalcStreamReceipt, OalcCaseFollowError> {
        if request.revision.trim().is_empty()
            || request.citation.trim().is_empty()
            || request.document_type.trim().is_empty()
        {
            return Err(OalcCaseFollowError::InvalidRequest(
                "pinned stream requires revision, citation and document_type".into(),
            ));
        }
        let url = format!(
            "https://huggingface.co/datasets/{OALC_DATASET_ID}/resolve/{}/corpus.jsonl?download=true",
            request.revision
        );
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(PINNED_STREAM_TIMEOUT_SECONDS))
            .build();
        let response = match agent
            .get(&url)
            .set("User-Agent", SENSIBLAW_UA)
            .set("Referer", "https://huggingface.co/")
            .call()
        {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(error) => return Err(OalcCaseFollowError::StreamingFallback(error.to_string())),
        };
        let status = classify_http_status(response.status());
        if status != ProviderAccessStatus::Available {
            return Err(OalcCaseFollowError::StreamingFallback(format!(
                "pinned corpus stream unavailable: {status:?}"
            )));
        }

        let reader = BufReader::new(response.into_reader());
        super::scan_pinned_oalc_jsonl(reader, request)
    }

    pub fn run_pinned(
        request: &PinnedOalcStreamRequest,
    ) -> Result<PinnedOalcStreamReceipt, OalcCaseFollowError> {
        stream_pinned_corpus(request)
    }

    fn resolve_exact_source_indexed(
        request: &OalcExactSourceRequest,
    ) -> Result<OalcExactSourceRunReceipt, OalcCaseFollowError> {
        if request.citation.trim().is_empty() || request.document_type.trim().is_empty() {
            return Err(OalcCaseFollowError::InvalidRequest(
                "exact source requires citation and document_type".into(),
            ));
        }
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

        let (url, bounded_resolution_label) = match request.citation_match {
            OalcCitationMatch::Exact => {
                let where_clause = super::oalc_exact_source_filter_predicate(request)?;
                (
                    format!(
                        "{HF_FILTER_API}?dataset={}&config={}&split={}&where={}&offset=0&length=2",
                        encode(OALC_DATASET_ID),
                        encode(OALC_CONFIG),
                        encode(OALC_SPLIT),
                        encode(&where_clause)
                    ),
                    "filter_exact",
                )
            }
            OalcCitationMatch::Contains => (
                format!(
                    "{HF_SEARCH_API}?dataset={}&config={}&split={}&query={}&offset=0&length=100",
                    encode(OALC_DATASET_ID),
                    encode(OALC_CONFIG),
                    encode(OALC_SPLIT),
                    encode(&request.citation)
                ),
                "search_exact_mnc",
            ),
        };
        let response: FilterResponse = serde_json::from_slice(&provider.get(&url)?.body)
            .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?;
        let index_state = if response.partial {
            OalcFilterIndexState::Partial
        } else {
            OalcFilterIndexState::Complete
        };
        let bounded_request = PinnedOalcStreamRequest {
            revision: info.sha.clone(),
            citation: request.citation.clone(),
            citation_match: request.citation_match,
            document_type: request.document_type.clone(),
            source: request.source.clone(),
            jurisdiction: request.jurisdiction.clone(),
        };
        let rows = response
            .rows
            .into_iter()
            .map(|row| row.row)
            .filter(|row| super::oalc_corpus_row_matches(&bounded_request, row))
            .collect();
        let (row, resolution_path) = match classify_exact_filter(rows, index_state) {
            OalcExactLookupDisposition::Found(row) => (row, bounded_resolution_label),
            OalcExactLookupDisposition::RequireRevisionPinnedStreaming => {
                let stream = stream_pinned_corpus(&bounded_request)?;
                (stream.row, "revision_pinned_streaming")
            }
            OalcExactLookupDisposition::CompleteIndexAbsent => {
                return Err(OalcCaseFollowError::SourceResidual(format!(
                    "complete OALC index found no bounded source for {}",
                    request.citation
                )))
            }
            OalcExactLookupDisposition::Ambiguous(count) => {
                return Err(OalcCaseFollowError::SourceResidual(format!(
                    "OALC exact source query returned {count} candidates for {}",
                    request.citation
                )))
            }
        };
        if !super::oalc_corpus_row_matches(&bounded_request, &row) {
            return Err(OalcCaseFollowError::Validation(
                "OALC exact source result failed post-retrieval bounds".into(),
            ));
        }
        Ok(OalcExactSourceRunReceipt {
            corpus_revision_sha: info.sha,
            row,
            resolution_path: resolution_path.into(),
            network_requests: provider.requests,
        })
    }

    pub fn resolve_dataset_revision() -> Result<String, OalcCaseFollowError> {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(60))
            .build();
        let response = agent
            .get(HF_DATASET_API)
            .set("User-Agent", SENSIBLAW_UA)
            .set("Referer", "https://huggingface.co/")
            .call()
            .map_err(|error| OalcCaseFollowError::Provider(error.to_string()))?;
        let info: DatasetInfo = serde_json::from_reader(response.into_reader())
            .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?;
        if info.sha.trim().is_empty() {
            return Err(OalcCaseFollowError::Dataset(
                "OALC metadata returned empty revision".into(),
            ));
        }
        Ok(info.sha)
    }

    fn stream_fallback(
        provider: &mut GovernedOalc<UreqTransport>,
        request: &OalcCaseFollowRequest,
        revision: &str,
    ) -> Result<PinnedOalcStreamReceipt, OalcCaseFollowError> {
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
        let stream_request = PinnedOalcStreamRequest {
            revision: revision.to_string(),
            citation: request.citation.clone(),
            citation_match: OalcCitationMatch::Contains,
            document_type: "decision".into(),
            source: None,
            jurisdiction: (!request.oalc_jurisdiction.is_empty())
                .then(|| request.oalc_jurisdiction.clone()),
        };
        let result = stream_pinned_corpus(&stream_request);
        provider.requests += 1;
        provider.last_request = Some(Instant::now());
        result
    }

    fn range_index_fallback(
        request: &OalcCaseFollowRequest,
        revision: &str,
    ) -> Result<OalcRangeLookupReceipt, OalcCaseFollowError> {
        let range_request = PinnedOalcStreamRequest {
            revision: revision.to_string(),
            citation: request.citation.clone(),
            citation_match: OalcCitationMatch::Contains,
            document_type: "decision".into(),
            source: None,
            jurisdiction: (!request.oalc_jurisdiction.is_empty())
                .then(|| request.oalc_jurisdiction.clone()),
        };
        lookup_or_build_oalc_range_index(&range_request)
    }

    fn run_with_mode(
        request: &OalcCaseFollowRequest,
        mode: OalcCaseAcquisitionMode,
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
        let url = format!(
            "{HF_SEARCH_API}?dataset={}&config={}&split={}&query={}&offset=0&length=100",
            encode(OALC_DATASET_ID),
            encode(OALC_CONFIG),
            encode(OALC_SPLIT),
            encode(&request.citation)
        );
        let bounded_request = PinnedOalcStreamRequest {
            revision: info.sha.clone(),
            citation: request.citation.clone(),
            citation_match: OalcCitationMatch::Contains,
            document_type: "decision".into(),
            source: None,
            jurisdiction: (!request.oalc_jurisdiction.is_empty())
                .then(|| request.oalc_jurisdiction.clone()),
        };
        let search = provider.get(&url);
        let (record, resolution_path, stream_receipt, range_receipt) = match search {
            Ok(response) => {
                let response: FilterResponse = serde_json::from_slice(&response.body)
                    .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?;
                let index_state = if response.partial {
                    OalcFilterIndexState::Partial
                } else {
                    OalcFilterIndexState::Complete
                };
                let rows = response
                    .rows
                    .into_iter()
                    .map(|row| row.row)
                    .filter(|row| super::oalc_corpus_row_matches(&bounded_request, row))
                    .collect();
                match classify_exact_filter(rows, index_state) {
                    OalcExactLookupDisposition::Found(row) => {
                        (row, "search_exact_mnc", None, None)
                    }
                    OalcExactLookupDisposition::RequireRevisionPinnedStreaming
                        if mode == OalcCaseAcquisitionMode::IndexedThenPinnedStream =>
                    {
                        let stream = stream_fallback(&mut provider, request, &info.sha)?;
                        (
                            stream.row.clone(),
                            "revision_pinned_streaming_after_incomplete_index",
                            Some(stream),
                            None,
                        )
                    }
                    OalcExactLookupDisposition::RequireRevisionPinnedStreaming
                        if mode == OalcCaseAcquisitionMode::IndexedThenPinnedRangeIndex =>
                    {
                        let range = range_index_fallback(request, &info.sha)?;
                        (
                            range.row.clone(),
                            if range.index_hit {
                                "revision_pinned_range_index_hit"
                            } else {
                                "revision_pinned_range_index_build_after_incomplete_index"
                            },
                            None,
                            Some(range),
                        )
                    }
                    OalcExactLookupDisposition::CompleteIndexAbsent
                        if mode == OalcCaseAcquisitionMode::IndexedThenPinnedStream =>
                    {
                        let stream = stream_fallback(&mut provider, request, &info.sha)?;
                        (
                            stream.row.clone(),
                            "revision_pinned_streaming_after_index_absence",
                            Some(stream),
                            None,
                        )
                    }
                    OalcExactLookupDisposition::CompleteIndexAbsent
                        if mode == OalcCaseAcquisitionMode::IndexedThenPinnedRangeIndex =>
                    {
                        let range = range_index_fallback(request, &info.sha)?;
                        (
                            range.row.clone(),
                            if range.index_hit {
                                "revision_pinned_range_index_hit"
                            } else {
                                "revision_pinned_range_index_build_after_index_absence"
                            },
                            None,
                            Some(range),
                        )
                    }
                    OalcExactLookupDisposition::RequireRevisionPinnedStreaming => {
                        return Err(OalcCaseFollowError::SourceResidual(format!(
                            "bounded OALC search was incomplete for {}; acquisition mode is IndexedOnly",
                            request.citation
                        )))
                    }
                    OalcExactLookupDisposition::CompleteIndexAbsent => {
                        return Err(OalcCaseFollowError::SourceResidual(format!(
                            "complete bounded OALC search found no exact terminal-MNC source for {}",
                            request.citation
                        )))
                    }
                    OalcExactLookupDisposition::Ambiguous(count) => {
                        return Err(OalcCaseFollowError::SourceResidual(format!(
                            "OALC returned {count} candidates for {}",
                            request.citation
                        )))
                    }
                }
            }
            Err(index_error)
                if mode == OalcCaseAcquisitionMode::IndexedThenPinnedStream =>
            {
                let stream =
                    stream_fallback(&mut provider, request, &info.sha).map_err(|stream_error| {
                        OalcCaseFollowError::StreamingFallback(format!(
                            "OALC index path failed ({index_error:?}); pinned stream fallback also failed ({stream_error:?})"
                        ))
                    })?;
                (
                    stream.row.clone(),
                    "revision_pinned_streaming_after_index_provider_failure",
                    Some(stream),
                    None,
                )
            }
            Err(index_error)
                if mode == OalcCaseAcquisitionMode::IndexedThenPinnedRangeIndex =>
            {
                let range =
                    range_index_fallback(request, &info.sha).map_err(|range_error| {
                        OalcCaseFollowError::StreamingFallback(format!(
                            "OALC index path failed ({index_error:?}); pinned range-index fallback also failed ({range_error:?})"
                        ))
                    })?;
                (
                    range.row.clone(),
                    if range.index_hit {
                        "revision_pinned_range_index_hit_after_index_provider_failure"
                    } else {
                        "revision_pinned_range_index_build_after_index_provider_failure"
                    },
                    None,
                    Some(range),
                )
            }
            Err(error) => return Err(error),
        };

        if !super::oalc_corpus_row_matches(&bounded_request, &record)
            || record.text.trim().is_empty()
        {
            return Err(OalcCaseFollowError::Validation(
                "OALC result failed decision/citation/text validation".into(),
            ));
        }
        let text_path = request.output_dir.join("judgment.txt");
        readonly_write(&text_path, &record.text)?;
        let range_requests = range_receipt
            .as_ref()
            .map(|value| value.range_requests)
            .unwrap_or(0);
        let total_network_requests = provider.requests.saturating_add(range_requests);
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
            network_requests: total_network_requests,
            stream_rows_examined: stream_receipt.as_ref().map(|value| value.rows_examined),
            stream_bytes_read: stream_receipt.as_ref().map(|value| value.bytes_read),
            stream_terminated_after_match: stream_receipt
                .as_ref()
                .map(|value| value.terminated_after_match),
            stream_uniqueness_exhaustively_verified: stream_receipt
                .as_ref()
                .map(|value| value.uniqueness_exhaustively_verified),
            range_index_hit: range_receipt.as_ref().map(|value| value.index_hit),
            range_index_requests: range_receipt.as_ref().map(|value| value.range_requests),
            range_index_rows_indexed_this_run: range_receipt
                .as_ref()
                .map(|value| value.rows_indexed_this_run),
            range_index_bytes_indexed_this_run: range_receipt
                .as_ref()
                .map(|value| value.bytes_indexed_this_run),
            range_index_byte_start: range_receipt
                .as_ref()
                .map(|value| value.entry.byte_start),
            range_index_byte_len: range_receipt
                .as_ref()
                .map(|value| value.entry.byte_len),
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
            stream_rows_examined: stream_receipt.as_ref().map(|value| value.rows_examined),
            stream_bytes_read: stream_receipt.as_ref().map(|value| value.bytes_read),
            stream_terminated_after_match: stream_receipt
                .as_ref()
                .map(|value| value.terminated_after_match),
            stream_uniqueness_exhaustively_verified: stream_receipt
                .as_ref()
                .map(|value| value.uniqueness_exhaustively_verified),
            range_index_hit: range_receipt.as_ref().map(|value| value.index_hit),
            range_index_requests: range_receipt.as_ref().map(|value| value.range_requests),
            range_index_rows_indexed_this_run: range_receipt
                .as_ref()
                .map(|value| value.rows_indexed_this_run),
            range_index_bytes_indexed_this_run: range_receipt
                .as_ref()
                .map(|value| value.bytes_indexed_this_run),
            range_index_byte_start: range_receipt
                .as_ref()
                .map(|value| value.entry.byte_start),
            range_index_byte_len: range_receipt
                .as_ref()
                .map(|value| value.entry.byte_len),
            candidate_only: true,
            creates_legal_authority: false,
            creates_claim_truth: false,
        })
    }

    pub fn resolve_exact_source(
        request: &OalcExactSourceRequest,
    ) -> Result<OalcExactSourceRunReceipt, OalcCaseFollowError> {
        resolve_exact_source_indexed(request)
    }

    pub fn run_with_acquisition_mode(
        request: &OalcCaseFollowRequest,
        mode: OalcCaseAcquisitionMode,
    ) -> Result<OalcCaseFollowRunReceipt, OalcCaseFollowError> {
        run_with_mode(request, mode)
    }

    /// Resolve a single authority with the durable HF-only fallback: the
    /// Dataset Server index is an accelerator, not the source boundary.
    pub fn run(
        request: &OalcCaseFollowRequest,
    ) -> Result<OalcCaseFollowRunReceipt, OalcCaseFollowError> {
        run_with_mode(request, OalcCaseAcquisitionMode::IndexedThenPinnedStream)
    }

    /// Strict indexed-only mode retained for explicitly bounded callers.
    pub fn run_filter_only(
        request: &OalcCaseFollowRequest,
    ) -> Result<OalcCaseFollowRunReceipt, OalcCaseFollowError> {
        run_with_mode(request, OalcCaseAcquisitionMode::IndexedOnly)
    }
}

#[cfg(feature = "live-network")]
pub use live::{
    resolve_dataset_revision as resolve_oalc_dataset_revision,
    resolve_exact_source as resolve_live_oalc_exact_source, run as run_live_oalc_case_follow,
    run_filter_only as run_live_oalc_case_follow_filter_only, run_pinned as run_pinned_oalc_stream,
    run_with_acquisition_mode as run_live_oalc_case_follow_with_mode,
};

#[cfg(not(feature = "live-network"))]
pub fn run_live_oalc_case_follow(
    _request: &OalcCaseFollowRequest,
) -> Result<OalcCaseFollowRunReceipt, OalcCaseFollowError> {
    Err(OalcCaseFollowError::LiveNetworkFeatureDisabled)
}

#[cfg(not(feature = "live-network"))]
pub fn run_live_oalc_case_follow_filter_only(
    _request: &OalcCaseFollowRequest,
) -> Result<OalcCaseFollowRunReceipt, OalcCaseFollowError> {
    Err(OalcCaseFollowError::LiveNetworkFeatureDisabled)
}

#[cfg(not(feature = "live-network"))]
pub fn run_live_oalc_case_follow_with_mode(
    _request: &OalcCaseFollowRequest,
    _mode: OalcCaseAcquisitionMode,
) -> Result<OalcCaseFollowRunReceipt, OalcCaseFollowError> {
    Err(OalcCaseFollowError::LiveNetworkFeatureDisabled)
}

#[cfg(not(feature = "live-network"))]
pub fn run_pinned_oalc_stream(
    _request: &PinnedOalcStreamRequest,
) -> Result<PinnedOalcStreamReceipt, OalcCaseFollowError> {
    Err(OalcCaseFollowError::LiveNetworkFeatureDisabled)
}

#[cfg(not(feature = "live-network"))]
pub fn resolve_oalc_dataset_revision() -> Result<String, OalcCaseFollowError> {
    Err(OalcCaseFollowError::LiveNetworkFeatureDisabled)
}

#[cfg(not(feature = "live-network"))]
pub fn resolve_live_oalc_exact_source(
    _request: &OalcExactSourceRequest,
) -> Result<OalcExactSourceRunReceipt, OalcCaseFollowError> {
    Err(OalcCaseFollowError::LiveNetworkFeatureDisabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(citation: &str, document_type: &str, source: &str, jurisdiction: &str) -> OalcCorpusRow {
        OalcCorpusRow {
            version_id: "version:fixture".into(),
            document_type: document_type.into(),
            jurisdiction: jurisdiction.into(),
            source: source.into(),
            citation: citation.into(),
            mime: None,
            date: None,
            url: None,
            when_scraped: None,
            text: "fixture text".into(),
        }
    }

    #[test]
    fn exact_legislation_filter_is_bounded_offline() {
        let request = OalcExactSourceRequest {
            citation: "Civil Liability Act 2002 (NSW)".into(),
            citation_match: OalcCitationMatch::Exact,
            document_type: "primary_legislation".into(),
            source: Some("nsw_legislation".into()),
            jurisdiction: Some("new_south_wales".into()),
        };
        let predicate = oalc_exact_source_filter_predicate(&request).unwrap();
        assert!(predicate.contains("\"citation\"='Civil Liability Act 2002 (NSW)'"));
        assert!(predicate.contains("\"type\"='primary_legislation'"));
        assert!(predicate.contains("\"source\"='nsw_legislation'"));
        assert!(predicate.contains("\"jurisdiction\"='new_south_wales'"));
        assert!(oalc_corpus_row_matches(
            &PinnedOalcStreamRequest {
                revision: "deadbeef".into(),
                citation: request.citation.clone(),
                citation_match: request.citation_match,
                document_type: request.document_type.clone(),
                source: request.source.clone(),
                jurisdiction: request.jurisdiction.clone(),
            },
            &row(
                "Civil Liability Act 2002 (NSW)",
                "primary_legislation",
                "nsw_legislation",
                "new_south_wales",
            ),
        ));
    }

    #[test]
    fn case_request_maps_supported_state_and_territory_courts() {
        let cases = [
            ("[2020] NSWSC 1", "court:NSWSC", "new_south_wales"),
            ("[2020] VSC 1", "court:VSC", "victoria"),
            ("[2020] QSC 1", "court:QSC", "queensland"),
            ("[2020] WASC 1", "court:WASC", "western_australia"),
            ("[2020] SASC 1", "court:SASC", "south_australia"),
            ("[2020] TASSC 1", "court:TASSC", "tasmania"),
            (
                "[2020] ACTSC 1",
                "court:ACTSC",
                "australian_capital_territory",
            ),
            ("[2020] NTSC 1", "court:NTSC", "northern_territory"),
        ];
        for (citation, court, jurisdiction) in cases {
            let request = OalcCaseFollowRequest::for_citation(citation, "/tmp/oalc-map-test");
            assert_eq!(request.court_ref, court);
            assert_eq!(request.oalc_jurisdiction, jurisdiction);
            assert!(request.legal_jurisdiction.starts_with("AU"));
        }
    }

    #[test]
    fn case_containment_matches_full_oalc_citation_but_not_wrong_jurisdiction() {
        let request = PinnedOalcStreamRequest {
            revision: "deadbeef".into(),
            citation: "[1988] HCA 7".into(),
            citation_match: OalcCitationMatch::Contains,
            document_type: "decision".into(),
            source: None,
            jurisdiction: Some("commonwealth".into()),
        };
        assert!(oalc_corpus_row_matches(
            &request,
            &row(
                "Waltons Stores (Interstate) Ltd v Maher [1988] HCA 7",
                "decision",
                "high_court_of_australia",
                "commonwealth",
            ),
        ));
        assert!(!oalc_corpus_row_matches(
            &request,
            &row(
                "Re Griffin; Ex parte Professional Radio and Electronics Institute (Aust.) [1988] HCA 72",
                "decision",
                "high_court_of_australia",
                "commonwealth",
            ),
        ));
        assert!(!oalc_corpus_row_matches(
            &request,
            &row(
                "Waltons Stores (Interstate) Ltd v Maher [1988] HCA 7",
                "decision",
                "fixture",
                "new_south_wales",
            ),
        ));
    }

    #[test]
    fn filter_predicate_escapes_single_quotes_on_supported_equality_path() {
        let request = OalcExactSourceRequest {
            citation: "O'Brien v Example [2020] HCA 1".into(),
            citation_match: OalcCitationMatch::Exact,
            document_type: "decision".into(),
            source: None,
            jurisdiction: None,
        };
        let predicate = oalc_exact_source_filter_predicate(&request).unwrap();
        assert!(predicate.contains("O''Brien"));
        assert!(!predicate.contains("LIKE"));
    }

    #[test]
    fn contains_filter_predicate_is_rejected_in_favour_of_search() {
        let request = OalcExactSourceRequest {
            citation: "[1999] HCA 10".into(),
            citation_match: OalcCitationMatch::Contains,
            document_type: "decision".into(),
            source: None,
            jurisdiction: Some("commonwealth".into()),
        };
        assert!(matches!(
            oalc_exact_source_filter_predicate(&request),
            Err(OalcCaseFollowError::InvalidRequest(_))
        ));
    }

    #[test]
    fn exact_filter_predicate_uses_documented_equality() {
        let request = OalcExactSourceRequest {
            citation: "Example v Example [1999] HCA 10".into(),
            citation_match: OalcCitationMatch::Exact,
            document_type: "decision".into(),
            source: None,
            jurisdiction: Some("commonwealth".into()),
        };
        let predicate = oalc_exact_source_filter_predicate(&request).unwrap();
        assert!(predicate.contains("\"citation\"='Example v Example [1999] HCA 10'"));
        assert!(!predicate.contains("LIKE"));
    }

    #[test]
    fn recursive_acquisition_mode_allows_index_then_pinned_stream() {
        assert_ne!(
            OalcCaseAcquisitionMode::IndexedOnly,
            OalcCaseAcquisitionMode::IndexedThenPinnedStream
        );
    }

    #[test]
    fn pinned_stream_receipt_does_not_overclaim_uniqueness() {
        let receipt = PinnedOalcStreamReceipt {
            row: row(
                "Giumelli v Giumelli [1999] HCA 10",
                "decision",
                "high_court_of_australia",
                "commonwealth",
            ),
            rows_examined: 42,
            bytes_read: 4096,
            terminated_after_match: true,
            uniqueness_exhaustively_verified: false,
        };
        assert!(receipt.terminated_after_match);
        assert!(!receipt.uniqueness_exhaustively_verified);
        assert_eq!(receipt.rows_examined, 42);
    }

    #[test]
    fn pinned_jsonl_scanner_stops_at_first_exact_mnc_and_records_cost() {
        use std::io::Cursor;

        let request = PinnedOalcStreamRequest {
            revision: "deadbeef".into(),
            citation: "[1999] HCA 10".into(),
            citation_match: OalcCitationMatch::Contains,
            document_type: "decision".into(),
            source: None,
            jurisdiction: Some("commonwealth".into()),
        };
        let before = serde_json::to_string(&row(
            "Example v Example [1998] HCA 1",
            "decision",
            "high_court_of_australia",
            "commonwealth",
        ))
        .unwrap();
        let target = serde_json::to_string(&row(
            "Giumelli v Giumelli [1999] HCA 10",
            "decision",
            "high_court_of_australia",
            "commonwealth",
        ))
        .unwrap();

        // The malformed third line must never be parsed: exact-MNC acquisition
        // terminates immediately after the target row.
        let jsonl = format!("{before}\n{target}\n{{malformed\n");
        let receipt = scan_pinned_oalc_jsonl(Cursor::new(jsonl.as_bytes()), &request).unwrap();

        assert_eq!(receipt.row.citation, "Giumelli v Giumelli [1999] HCA 10");
        assert_eq!(receipt.rows_examined, 2);
        assert_eq!(
            receipt.bytes_read,
            (before.len() + 1 + target.len() + 1) as u64
        );
        assert!(receipt.terminated_after_match);
        assert!(!receipt.uniqueness_exhaustively_verified);
    }

    #[test]
    fn completed_pinned_scan_without_match_is_source_residual() {
        use std::io::Cursor;

        let request = PinnedOalcStreamRequest {
            revision: "deadbeef".into(),
            citation: "[1999] HCA 10".into(),
            citation_match: OalcCitationMatch::Contains,
            document_type: "decision".into(),
            source: None,
            jurisdiction: Some("commonwealth".into()),
        };
        let other = serde_json::to_string(&row(
            "Example v Example [1998] HCA 1",
            "decision",
            "high_court_of_australia",
            "commonwealth",
        ))
        .unwrap();
        let error =
            scan_pinned_oalc_jsonl(Cursor::new(format!("{other}\n").into_bytes()), &request)
                .unwrap_err();
        assert!(matches!(error, OalcCaseFollowError::SourceResidual(_)));
    }
    #[test]
    fn range_index_mode_is_distinct_from_one_shot_streaming() {
        assert_ne!(
            OalcCaseAcquisitionMode::IndexedThenPinnedRangeIndex,
            OalcCaseAcquisitionMode::IndexedThenPinnedStream
        );
    }

}