#[cfg(not(feature = "live-network"))]
fn main() {
    eprintln!("enable --features live-network to run governed OALC contract follow");
}

#[cfg(feature = "live-network")]
mod live {
    use sensiblaw_governed_legal_provider::{
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
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, Instant};

    const HF_DATASET_API: &str =
        "https://huggingface.co/api/datasets/isaacus/open-australian-legal-corpus";
    const HF_FILTER_API: &str = "https://datasets-server.huggingface.co/filter";
    const WALTONS_MNC: &str = "[1988] HCA 7";

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

    fn filter_url(demand: &OalcSourceDemand) -> Result<String, String> {
        let predicate =
            oalc_filter_predicate(demand).map_err(|error| format!("filter predicate: {error:?}"))?;
        Ok(format!(
            "{HF_FILTER_API}?dataset={}&config={}&split={}&where={}&offset=0&length=2",
            encode(OALC_DATASET_ID),
            encode(OALC_CONFIG),
            encode(OALC_SPLIT),
            encode(&predicate)
        ))
    }

    fn sha256(bytes: &[u8]) -> String {
        format!("sha256:{:x}", Sha256::digest(bytes))
    }

    fn readonly_write(path: &Path, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, text.as_bytes())?;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_readonly(true);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }

    fn waltons_demand() -> Result<OalcSourceDemand, String> {
        let demand = LegalSourceDemand {
            demand_ref: "contract-follow:waltons:hca7".into(),
            origin_ref: "doctrine:au:contract:estoppel".into(),
            jurisdiction_ref: Some("AU".into()),
            source_roles: vec![SourceRole::PrimaryCaseLaw],
            authority_levels: vec![AuthorityLevel::Official],
            provider_profile_refs: vec![OALC_PROVIDER_PROFILE.into()],
            requested_facets: vec![
                "case.full_text".into(),
                "case.citation_graph".into(),
                "case.treatment".into(),
            ],
            temporal_refs: vec!["as_at:2026-09-20".into()],
            provenance_refs: vec!["australian-contracts-follow:v1".into()],
            priority: 100,
        };
        let plan = plan_legal_sources(&demand, &[]);
        let exact = exact_oalc_case_law_demand(
            &plan,
            &demand.origin_ref,
            WALTONS_MNC,
            "court:HCA",
        )
        .map_err(|error| format!("LegalFollow case demand: {error:?}"))?;
        OalcSourceDemand::from_case_law(&exact)
            .map_err(|error| format!("OALC case demand: {error:?}"))
    }

    fn streaming_fallback(
        revision_sha: &str,
    ) -> Result<OalcRow, Box<dyn std::error::Error>> {
        let helper = env::var("SENSIBLAW_OALC_STREAMING_HELPER")
            .unwrap_or_else(|_| "script/stream_oalc_exact_record.py".into());
        let output = std::process::Command::new("python3")
            .arg(helper)
            .arg("--dataset-id")
            .arg(OALC_DATASET_ID)
            .arg("--config")
            .arg(OALC_CONFIG)
            .arg("--split")
            .arg(OALC_SPLIT)
            .arg("--revision")
            .arg(revision_sha)
            .arg("--citation")
            .arg(WALTONS_MNC)
            .env("HF_HUB_DISABLE_TELEMETRY", "1")
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "OALC streaming fallback residual for {WALTONS_MNC}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into());
        }
        Ok(serde_json::from_slice(&output.stdout)?)
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
        fn get(&mut self, url: &str) -> Result<HttpResponse, String> {
            if self.requests >= self.context.bounds.max_network_requests {
                return Err("governed OALC request budget exceeded".into());
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
                .map_err(|error| error.to_string())?;
            self.requests += 1;
            self.last_request = Some(Instant::now());
            let status = classify_http_status(response.status_code);
            if status != ProviderAccessStatus::Available {
                return Err(format!("OALC provider unavailable: {status:?}"));
            }
            Ok(response)
        }
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        if !env::args().any(|arg| arg == "--operator-opt-in") {
            return Err("explicit --operator-opt-in required".into());
        }
        let output_dir = PathBuf::from(
            env::var("SENSIBLAW_OALC_OUTPUT")
                .unwrap_or_else(|_| "artifacts/oalc/contracts/waltons".into()),
        );
        fs::create_dir_all(&output_dir)?;

        let context = GovernedExecutionContext {
            operator_opt_in: true,
            cache_checked_first: true,
            persisted_receipts_checked_first: true,
            bounds: LiveGovernanceBounds {
                minimum_pacing_seconds: 4,
                burst: 1,
                max_depth: 1,
                max_new_documents: 1,
                max_network_requests: 2,
            },
        };
        context
            .validate()
            .map_err(|error| format!("governance: {error:?}"))?;
        let mut provider = GovernedOalc {
            transport: UreqTransport,
            context,
            last_request: None,
            requests: 0,
        };

        let info: DatasetInfo = serde_json::from_slice(&provider.get(HF_DATASET_API)?.body)?;
        if info.sha.trim().is_empty() {
            return Err("OALC metadata returned empty revision".into());
        }
        let dataset = PinnedOalcDatasetSelection {
            dataset_id: OALC_DATASET_ID.into(),
            config: OALC_CONFIG.into(),
            split: OALC_SPLIT.into(),
            corpus_revision_ref: format!("{OALC_DATASET_ID}@{}", info.sha),
        };
        dataset
            .validate()
            .map_err(|error| format!("dataset: {error:?}"))?;

        let demand = waltons_demand()?;
        assert_eq!(demand.document_kind, OalcDocumentKind::CaseLaw);
        let response: FilterResponse =
            serde_json::from_slice(&provider.get(&filter_url(&demand)?)?.body)?;
        let state = if response.partial {
            OalcFilterIndexState::Partial
        } else {
            OalcFilterIndexState::Complete
        };
        let rows = response.rows.into_iter().map(|row| row.row).collect();
        let (record, resolution_path) = match classify_exact_filter(rows, state) {
            OalcExactLookupDisposition::Found(row) => (row, "filter_exact"),
            OalcExactLookupDisposition::RequireRevisionPinnedStreaming => {
                (streaming_fallback(&info.sha)?, "revision_pinned_streaming")
            }
            OalcExactLookupDisposition::CompleteIndexAbsent => {
                return Err("OALC complete index found no Waltons record; source residual remains".into())
            }
            OalcExactLookupDisposition::Ambiguous(count) => {
                return Err(format!("OALC returned {count} Waltons candidates; identity review required").into())
            }
        };

        if !record.citation.contains(WALTONS_MNC)
            || record.document_type != "decision"
            || record.text.trim().is_empty()
        {
            return Err("OALC result failed exact Waltons decision validation".into());
        }

        let artifact = output_dir.join("waltons-stores-v-maher.txt");
        readonly_write(&artifact, &record.text)?;
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
            local_artifact_ref: artifact,
            temporal_coverage: OalcTemporalCoverage::DecisionDateAnchored,
            resolution_path: resolution_path.into(),
            network_requests: provider.requests,
            receipt_authority: OALC_RECEIPT_AUTHORITY,
            candidate_only: true,
            creates_legal_authority: false,
            creates_claim_truth: false,
        };
        receipt
            .validate_against(&dataset, &demand)
            .map_err(|error| format!("receipt: {error:?}"))?;

        println!(
            "LegalFollow -> governed OALC Waltons source candidate; revision={}; version={}; path={}; network_requests={}; authority={}",
            receipt.corpus_revision_ref,
            receipt.version_id,
            receipt.resolution_path,
            receipt.network_requests,
            receipt.receipt_authority
        );
        Ok(())
    }
}

#[cfg(feature = "live-network")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    live::run()
}
