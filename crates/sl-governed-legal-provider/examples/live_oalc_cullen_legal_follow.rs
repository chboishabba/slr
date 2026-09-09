#[path = "../src/oalc_legislation_contract.rs"]
mod oalc_legislation_contract;

#[cfg(not(feature = "live-network"))]
fn main() {
    eprintln!("enable --features live-network to run governed OALC resolution");
}

#[cfg(feature = "live-network")]
mod live {
    use oalc_legislation_contract::{
        OalcLegislationDemand, OalcResolvedDocumentReceipt, OalcTemporalCoverage,
        PinnedOalcDatasetSelection, CULLEN_CLA_CITATION, CULLEN_VICARIOUS_CITATION,
        OALC_CONFIG, OALC_DATASET_ID, OALC_RECEIPT_AUTHORITY, OALC_SPLIT,
    };
    use sensiblaw_governed_legal_provider::{
        classify_http_status, GovernedExecutionContext, HttpRequest, HttpResponse, HttpTransport,
        LiveGovernanceBounds, ProviderAccessStatus, SENSIBLAW_UA, UreqTransport,
    };
    use sensiblaw_legal_follow_plan::{
        exact_oalc_legislation_demand, plan_legal_sources, AuthorityLevel, LegalSourceDemand,
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
    const TARGETS: [&str; 2] = [CULLEN_CLA_CITATION, CULLEN_VICARIOUS_CITATION];

    #[derive(Debug, Deserialize)]
    struct DatasetInfo {
        sha: String,
    }

    #[derive(Debug, Deserialize)]
    struct FilterResponse {
        rows: Vec<FilterRow>,
    }

    #[derive(Debug, Deserialize)]
    struct FilterRow {
        row: OalcRow,
    }

    #[derive(Debug, Deserialize)]
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

    fn sha256(bytes: &[u8]) -> String {
        format!("sha256:{:x}", Sha256::digest(bytes))
    }

    fn slug(citation: &str) -> String {
        citation
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-")
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

    fn filter_url(citation: &str) -> String {
        let escaped = citation.replace('\'', "''");
        let where_clause = format!(
            "\"citation\"='{escaped}' AND \"source\"='nsw_legislation' AND \"jurisdiction\"='new_south_wales' AND \"type\"='primary_legislation'"
        );
        format!(
            "{HF_FILTER_API}?dataset={}&config={}&split={}&where={}&offset=0&length=2",
            encode(OALC_DATASET_ID),
            encode(OALC_CONFIG),
            encode(OALC_SPLIT),
            encode(&where_clause)
        )
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

    fn tsv(value: &str) -> String {
        value.replace(['\t', '\r', '\n'], " ")
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
        fn new(transport: T, context: GovernedExecutionContext) -> Result<Self, String> {
            context.validate().map_err(|err| format!("governance: {err:?}"))?;
            Ok(Self { transport, context, last_request: None, requests: 0 })
        }

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
                .map_err(|err| err.to_string())?;
            self.requests += 1;
            self.last_request = Some(Instant::now());
            let status = classify_http_status(response.status_code);
            if status != ProviderAccessStatus::Available {
                return Err(format!("OALC provider unavailable: {status:?}"));
            }
            Ok(response)
        }
    }

    fn legal_follow_demand(citation: &str) -> Result<sensiblaw_legal_follow_plan::ExactLegislationSourceDemand, String> {
        let demand = LegalSourceDemand {
            demand_ref: format!("cullen:oalc:{citation}"),
            origin_ref: "consumer:Cullen:governing-legislation-pnf".into(),
            jurisdiction_ref: Some("AU-NSW".into()),
            source_roles: vec![SourceRole::PrimaryLegislation],
            authority_levels: vec![AuthorityLevel::Official],
            provider_profile_refs: vec![OALC_PROVIDER_PROFILE.into()],
            requested_facets: vec!["legislation.text".into()],
            temporal_refs: vec!["latest_known_only".into()],
            provenance_refs: vec!["legal-follow:oalc:cullen".into()],
            priority: 100,
        };
        let plan = plan_legal_sources(&demand, &[]);
        exact_oalc_legislation_demand(&plan, &demand.origin_ref, citation)
            .map_err(|err| format!("LegalFollow exact demand failed: {err:?}"))
    }

    fn provider_demand(
        exact: &sensiblaw_legal_follow_plan::ExactLegislationSourceDemand,
    ) -> OalcLegislationDemand {
        OalcLegislationDemand {
            demand_ref: exact.demand_ref.clone(),
            citation: exact.citation.clone(),
            jurisdiction: "new_south_wales".into(),
            source: "nsw_legislation".into(),
            document_type: "primary_legislation".into(),
            temporal_coverage_required: OalcTemporalCoverage::LatestKnownOnly,
        }
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let operator_opt_in = env::args().any(|arg| arg == "--operator-opt-in");
        if !operator_opt_in {
            return Err("explicit --operator-opt-in required".into());
        }
        let output_dir = PathBuf::from(
            env::var("SENSIBLAW_OALC_OUTPUT")
                .unwrap_or_else(|_| "artifacts/oalc/cullen-governing-law/materialised".into()),
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
                max_new_documents: 2,
                max_network_requests: 3,
            },
        };
        let mut provider = GovernedOalc::new(UreqTransport, context)
            .map_err(|err| format!("governed OALC init failed: {err}"))?;

        let info_response = provider
            .get(HF_DATASET_API)
            .map_err(|err| format!("OALC dataset metadata fetch failed: {err}"))?;
        let info: DatasetInfo = serde_json::from_slice(&info_response.body)?;
        if info.sha.trim().is_empty() {
            return Err("Hugging Face dataset metadata returned empty SHA".into());
        }
        let corpus_revision = format!("{OALC_DATASET_ID}@{}", info.sha);
        let dataset = PinnedOalcDatasetSelection {
            dataset_id: OALC_DATASET_ID.into(),
            config: OALC_CONFIG.into(),
            split: OALC_SPLIT.into(),
            corpus_revision_ref: corpus_revision.clone(),
        };
        dataset
            .validate()
            .map_err(|err| format!("resolved OALC dataset selection invalid: {err:?}"))?;

        let receipt_path = output_dir.join("oalc_legislation_receipts.tsv");
        let mut receipt = String::from(
            "citation\tversion_id\tcorpus_revision\tsource\tjurisdiction\ttype\tdate\turl\twhen_scraped\tcanonical_text_digest\tlocal_artifact_ref\ttemporal_status\tnetwork_requests\treceipt_authority\n",
        );

        for citation in TARGETS {
            let exact = legal_follow_demand(citation)?;
            let demand = provider_demand(&exact);
            let response = provider
                .get(&filter_url(citation))
                .map_err(|err| format!("OALC exact row fetch failed for {citation}: {err}"))?;
            let filtered: FilterResponse = serde_json::from_slice(&response.body)?;
            if filtered.rows.len() != 1 {
                return Err(format!(
                    "OALC exact citation must resolve to one row, got {} for {citation}",
                    filtered.rows.len()
                )
                .into());
            }
            let record = filtered.rows.into_iter().next().expect("length checked").row;
            if record.citation != citation || record.text.trim().is_empty() {
                return Err(format!("OALC returned wrong or empty record for {citation}").into());
            }
            let artifact = output_dir.join(format!("{}.txt", slug(citation)));
            readonly_write(&artifact, &record.text)?;
            let digest = sha256(record.text.as_bytes());
            let resolved = OalcResolvedDocumentReceipt {
                demand_ref: demand.demand_ref.clone(),
                citation: record.citation.clone(),
                version_id: record.version_id.clone(),
                corpus_revision_ref: corpus_revision.clone(),
                source: record.source.clone(),
                jurisdiction: record.jurisdiction.clone(),
                document_type: record.document_type.clone(),
                canonical_text_digest: digest.clone(),
                local_artifact_ref: artifact.clone(),
                temporal_coverage: OalcTemporalCoverage::LatestKnownOnly,
                network_requests: 1,
                receipt_authority: OALC_RECEIPT_AUTHORITY,
            };
            resolved
                .validate_against(&dataset, &demand)
                .map_err(|err| format!("resolved OALC receipt invalid for {citation}: {err:?}"))?;

            let row = [
                record.citation,
                record.version_id,
                corpus_revision.clone(),
                record.source,
                record.jurisdiction,
                record.document_type,
                record.date.unwrap_or_default(),
                record.url.unwrap_or_default(),
                record.when_scraped.unwrap_or_default(),
                digest,
                artifact.to_string_lossy().into_owned(),
                "latest_known_only".to_string(),
                "1".to_string(),
                OALC_RECEIPT_AUTHORITY.to_string(),
            ]
            .iter()
            .map(|value| tsv(value))
            .collect::<Vec<_>>()
            .join("\t");
            receipt.push_str(&row);
            receipt.push('\n');
        }

        fs::write(&receipt_path, receipt)?;
        println!(
            "LegalFollow -> governed OALC resolved 2 legislation documents; corpus_revision={corpus_revision}; network_requests={}; receipts={}",
            provider.requests,
            receipt_path.display()
        );
        Ok(())
    }
}

#[cfg(feature = "live-network")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    live::run()
}
