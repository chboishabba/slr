#[cfg(not(feature = "live-network"))]
fn main() {
    eprintln!("enable --features live-network to run governed OALC case follow");
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

    #[derive(Debug)]
    struct Args {
        citation: String,
        court_ref: String,
        oalc_jurisdiction: String,
        legal_jurisdiction: String,
        output_dir: PathBuf,
    }

    fn parse_args() -> Result<Args, String> {
        let values = env::args().skip(1).collect::<Vec<_>>();
        if !values.iter().any(|arg| arg == "--operator-opt-in") {
            return Err("explicit --operator-opt-in required".into());
        }
        fn value(values: &[String], flag: &str) -> Option<String> {
            values.iter().position(|arg| arg == flag)
                .and_then(|index| values.get(index + 1))
                .cloned()
        }
        let citation = value(&values, "--citation")
            .ok_or("--citation '[YYYY] COURT N' required")?;
        let court_token = citation
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_ascii_uppercase();
        let default_court = match court_token.as_str() {
            "HCA" => "court:HCA",
            "FCA" | "FCAFC" => "court:FCA",
            "NSWCA" => "court:NSWCA",
            "VSCA" => "court:VSCA",
            "QCA" => "court:QCA",
            "WASCA" => "court:WASCA",
            _ => "court:unknown",
        };
        let default_oalc_jurisdiction = match court_token.as_str() {
            "HCA" | "FCA" | "FCAFC" => "commonwealth",
            "NSWCA" => "new_south_wales",
            "VSCA" => "victoria",
            "QCA" => "queensland",
            "WASCA" => "western_australia",
            _ => "",
        };
        let safe = citation
            .replace('[', "")
            .replace(']', "")
            .replace(' ', "-")
            .to_ascii_lowercase();
        Ok(Args {
            citation,
            court_ref: value(&values, "--court-ref").unwrap_or_else(|| default_court.into()),
            oalc_jurisdiction: value(&values, "--oalc-jurisdiction")
                .unwrap_or_else(|| default_oalc_jurisdiction.into()),
            legal_jurisdiction: value(&values, "--legal-jurisdiction")
                .unwrap_or_else(|| "AU".into()),
            output_dir: value(&values, "--output-dir")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(format!("artifacts/oalc/cases/{safe}"))),
        })
    }

    #[derive(Debug, Deserialize)]
    struct DatasetInfo { sha: String }

    #[derive(Debug, Deserialize)]
    struct FilterResponse {
        rows: Vec<FilterRow>,
        #[serde(default)]
        partial: bool,
    }

    #[derive(Debug, Deserialize)]
    struct FilterRow { row: OalcRow }

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
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
                other => {
                    const HEX: &[u8;16]=b"0123456789ABCDEF";
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

    fn readonly_write(path: &Path, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent)=path.parent(){ fs::create_dir_all(parent)?; }
        fs::write(path,text.as_bytes())?;
        let mut permissions=fs::metadata(path)?.permissions();
        permissions.set_readonly(true);
        fs::set_permissions(path,permissions)?;
        Ok(())
    }

    fn case_demand(args: &Args) -> Result<OalcSourceDemand,String> {
        let demand=LegalSourceDemand{
            demand_ref:format!("legal-follow:case:{}",args.citation),
            origin_ref:"legal-follow:cited-by-treatment".into(),
            jurisdiction_ref:Some(args.legal_jurisdiction.clone()),
            source_roles:vec![SourceRole::PrimaryCaseLaw],
            authority_levels:vec![AuthorityLevel::Official],
            provider_profile_refs:vec![OALC_PROVIDER_PROFILE.into()],
            requested_facets:vec!["case.full_text".into(),"case.citation_graph".into(),"case.treatment".into()],
            temporal_refs:vec!["as_at:2026-09-20".into()],
            provenance_refs:vec!["cited-by-primary-source-reacquisition".into()],
            priority:100,
        };
        let plan=plan_legal_sources(&demand,&[]);
        let exact=exact_oalc_case_law_demand(&plan,&demand.origin_ref,&args.citation,&args.court_ref)
            .map_err(|e|format!("LegalFollow case demand: {e:?}"))?;
        OalcSourceDemand::from_case_law(&exact).map_err(|e|format!("OALC case demand: {e:?}"))
    }

    fn streaming_fallback(args:&Args,revision:&str)->Result<OalcRow,Box<dyn std::error::Error>>{
        let helper=env::var("SENSIBLAW_OALC_STREAMING_HELPER")
            .unwrap_or_else(|_|"script/stream_oalc_exact_record.py".into());
        let output=std::process::Command::new("python3")
            .arg(helper)
            .arg("--dataset-id").arg(OALC_DATASET_ID)
            .arg("--config").arg(OALC_CONFIG)
            .arg("--split").arg(OALC_SPLIT)
            .arg("--revision").arg(revision)
            .arg("--citation").arg(&args.citation)
            .arg("--citation-match").arg("contains")
            .arg("--document-type").arg("decision")
            .arg("--source").arg("")
            .arg("--jurisdiction").arg(&args.oalc_jurisdiction)
            .env("HF_HUB_DISABLE_TELEMETRY","1")
            .output()?;
        if !output.status.success(){
            return Err(format!("OALC streaming residual for {}: {}",args.citation,String::from_utf8_lossy(&output.stderr).trim()).into());
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }

    struct GovernedOalc<T>{
        transport:T,
        context:GovernedExecutionContext,
        last_request:Option<Instant>,
        requests:u64,
    }
    impl<T> GovernedOalc<T>
    where T:HttpTransport,T::Error:ToString {
        fn get(&mut self,url:&str)->Result<HttpResponse,String>{
            if self.requests>=self.context.bounds.max_network_requests{return Err("OALC request budget exceeded".into());}
            if let Some(last)=self.last_request{
                let minimum=Duration::from_secs(self.context.bounds.minimum_pacing_seconds);
                let elapsed=last.elapsed();
                if elapsed<minimum{thread::sleep(minimum-elapsed);}
            }
            let response=self.transport.get(&HttpRequest{
                url:url.into(),user_agent:SENSIBLAW_UA.into(),referer:Some("https://huggingface.co/".into()),timeout_seconds:60
            }).map_err(|e|e.to_string())?;
            self.requests+=1; self.last_request=Some(Instant::now());
            let status=classify_http_status(response.status_code);
            if status!=ProviderAccessStatus::Available{return Err(format!("OALC provider unavailable: {status:?}"));}
            Ok(response)
        }
    }

    pub fn run()->Result<(),Box<dyn std::error::Error>>{
        let args=parse_args().map_err(|e|format!("arguments: {e}"))?;
        fs::create_dir_all(&args.output_dir)?;
        let context=GovernedExecutionContext{
            operator_opt_in:true,cache_checked_first:true,persisted_receipts_checked_first:true,
            bounds:LiveGovernanceBounds{minimum_pacing_seconds:4,burst:1,max_depth:1,max_new_documents:1,max_network_requests:2},
        };
        context.validate().map_err(|e|format!("governance: {e:?}"))?;
        let mut provider=GovernedOalc{transport:UreqTransport,context,last_request:None,requests:0};
        let info:DatasetInfo=serde_json::from_slice(&provider.get(HF_DATASET_API)?.body)?;
        let dataset=PinnedOalcDatasetSelection{
            dataset_id:OALC_DATASET_ID.into(),config:OALC_CONFIG.into(),split:OALC_SPLIT.into(),
            corpus_revision_ref:format!("{OALC_DATASET_ID}@{}",info.sha),
        };
        dataset.validate().map_err(|e|format!("dataset: {e:?}"))?;
        let demand=case_demand(&args)?;
        assert_eq!(demand.document_kind,OalcDocumentKind::CaseLaw);
        let predicate=oalc_filter_predicate(&demand).map_err(|e|format!("predicate: {e:?}"))?;
        let url=format!("{HF_FILTER_API}?dataset={}&config={}&split={}&where={}&offset=0&length=2",
            encode(OALC_DATASET_ID),encode(OALC_CONFIG),encode(OALC_SPLIT),encode(&predicate));
        let response:FilterResponse=serde_json::from_slice(&provider.get(&url)?.body)?;
        let state=if response.partial{OalcFilterIndexState::Partial}else{OalcFilterIndexState::Complete};
        let rows=response.rows.into_iter().map(|r|r.row).collect();
        let (record,resolution_path)=match classify_exact_filter(rows,state){
            OalcExactLookupDisposition::Found(row)=>(row,"filter_exact"),
            OalcExactLookupDisposition::RequireRevisionPinnedStreaming=>(streaming_fallback(&args,&info.sha)?,"revision_pinned_streaming"),
            OalcExactLookupDisposition::CompleteIndexAbsent=>return Err(format!("OALC complete index found no {}",args.citation).into()),
            OalcExactLookupDisposition::Ambiguous(n)=>return Err(format!("OALC returned {n} candidates for {}",args.citation).into()),
        };
        if !record.citation.contains(&args.citation)||record.document_type!="decision"||record.text.trim().is_empty(){
            return Err("OALC decision failed exact post-retrieval validation".into());
        }
        if !args.oalc_jurisdiction.is_empty() && record.jurisdiction!=args.oalc_jurisdiction{
            return Err(format!("OALC jurisdiction mismatch: expected {} got {}",args.oalc_jurisdiction,record.jurisdiction).into());
        }
        let text_path=args.output_dir.join("judgment.txt");
        readonly_write(&text_path,&record.text)?;
        let receipt=OalcResolvedSourceReceipt{
            demand_ref:demand.demand_ref.clone(),origin_ref:demand.origin_ref.clone(),citation:demand.citation.clone(),
            version_id:record.version_id,corpus_revision_ref:dataset.corpus_revision_ref.clone(),source:record.source,
            jurisdiction:record.jurisdiction,document_type:record.document_type,court:demand.court_ref.clone(),date:record.date,
            canonical_url:record.url,when_scraped:record.when_scraped,canonical_text_digest:sha256(record.text.as_bytes()),
            local_artifact_ref:text_path,temporal_coverage:OalcTemporalCoverage::DecisionDateAnchored,
            resolution_path:resolution_path.into(),network_requests:provider.requests,
            receipt_authority:OALC_RECEIPT_AUTHORITY.into(),candidate_only:true,creates_legal_authority:false,creates_claim_truth:false,
        };
        receipt.validate_against(&dataset,&demand).map_err(|e|format!("receipt: {e:?}"))?;
        let receipt_path=args.output_dir.join("oalc-source-receipt.json");
        fs::write(&receipt_path,serde_json::to_vec_pretty(&receipt)?)?;
        println!("oalc_case_receipt={} citation={} revision={} version={} authority={}",
            receipt_path.display(),args.citation,receipt.corpus_revision_ref,receipt.version_id,receipt.receipt_authority);
        Ok(())
    }
}

#[cfg(feature = "live-network")]
fn main()->Result<(),Box<dyn std::error::Error>>{live::run()}
