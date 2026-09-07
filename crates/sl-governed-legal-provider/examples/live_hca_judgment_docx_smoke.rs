#[cfg(feature = "live-network")]
#[path = "../src/docx_text.rs"]
mod docx_text;
#[cfg(feature = "live-network")]
#[path = "../src/official_resource.rs"]
mod official_resource;

#[cfg(feature = "live-network")]
fn main() {
    use docx_text::extract_docx_canonical_text;
    use official_resource::{
        discover_hca_judgment_resources, preferred_hca_judgment_resource, JudgmentResourceKind,
    };
    use sensiblaw_governed_legal_provider::*;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::PathBuf;

    fn json_escape(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    if std::env::var("SENSIBLAW_LIVE_LEGAL_OPT_IN").ok().as_deref() != Some("1") {
        eprintln!("refusing live legal acquisition: set SENSIBLAW_LIVE_LEGAL_OPT_IN=1 explicitly");
        std::process::exit(2);
    }

    let runtime_head = std::env::var("SENSIBLAW_RUNTIME_HEAD")
        .expect("SENSIBLAW_RUNTIME_HEAD must pin the exact git head for a live receipt");
    let landing_path = PathBuf::from(
        std::env::var("SENSIBLAW_HCA_LANDING_HTML")
            .unwrap_or_else(|_| "/tmp/sensiblaw-live-legal/authority.html".into()),
    );
    let output_dir = PathBuf::from(
        std::env::var("SENSIBLAW_LIVE_RECEIPT_DIR")
            .unwrap_or_else(|_| "/tmp/sensiblaw-live-legal".into()),
    );
    fs::create_dir_all(&output_dir).expect("create live receipt directory");

    let landing_bytes = fs::read(&landing_path).expect("read persisted HCA landing page");
    let resources = discover_hca_judgment_resources(&landing_bytes);
    let preferred = preferred_hca_judgment_resource(&resources)
        .expect("HCA landing page must expose a judgment document resource");
    assert_eq!(preferred.kind, JudgmentResourceKind::Docx);

    let context = GovernedExecutionContext {
        operator_opt_in: true,
        cache_checked_first: true,
        persisted_receipts_checked_first: true,
        bounds: LiveGovernanceBounds {
            max_network_requests: 1,
            max_new_documents: 1,
            ..LiveGovernanceBounds::HISTORICAL_DEFAULT
        },
    };
    let mut executor = GovernedExecutor::new(UreqTransport, context).expect("governance preflight");
    let fetched = executor
        .fetch_hca(&preferred.reference)
        .expect("bounded official HCA judgment DOCX fetch");
    assert_eq!(executor.network_requests(), 1);
    assert_eq!(fetched.provider, LegalProvider::HighCourtAustralia);
    assert_eq!(fetched.network_requests, 1);
    assert!(!fetched.locally_ingested);

    let digest = format!("sha256:{:x}", Sha256::digest(&fetched.bytes));
    let artifact_path = output_dir.join("judgment.docx");
    fs::write(&artifact_path, &fetched.bytes).expect("persist HCA judgment DOCX locally");
    let source_identity = "document:hca:[2026]-HCA-19:docx".to_string();
    let source_revision = format!("source-revision:{digest}");
    let ingested = mark_locally_ingested(
        &fetched,
        source_identity.clone(),
        source_revision.clone(),
        digest.clone(),
    );
    assert!(ingested.locally_ingested);

    let materialized = extract_docx_canonical_text(&fetched.bytes)
        .expect("materialize official HCA DOCX into canonical local text");
    let canonical_text_digest = format!("sha256:{:x}", Sha256::digest(materialized.text.as_bytes()));
    let canonical_text_path = output_dir.join("judgment.txt");
    fs::write(&canonical_text_path, materialized.text.as_bytes())
        .expect("persist canonical HCA judgment text locally");

    let replay_context = ResolutionContext {
        persisted: vec![PersistedAuthorityReceipt {
            source_identity_ref: source_identity.clone(),
            source_revision_ref: source_revision.clone(),
            jurisdiction_ref: "AU".into(),
            compile_eligible: true,
        }],
        ..ResolutionContext::default()
    };
    let replay_demand = KnownAuthorityDemand {
        demand_ref: "live-smoke:official-hca-judgment-docx".into(),
        jurisdiction_ref: "AU".into(),
        source_identity_ref: source_identity.clone(),
        medium_neutral_citation: Some("[2026] HCA 19".into()),
        explicit_austlii_ref: None,
        proposition_ref: Some("prop:cullen-positive-operational-duty".into()),
        use_intent: PropositionUseIntent::SourceProposition,
        treatment_intent: CitationTreatmentIntent::None,
    };
    let replay = resolve_known_authority(&replay_demand, &replay_context);
    assert!(matches!(replay.stage, ResolutionStage::Persisted { .. }));

    let receipt = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"sl.governed_official_judgment_acquisition.v0_1\",\n",
            "  \"runtime_head\": \"{}\",\n",
            "  \"authority\": \"experimental_candidate_only\",\n",
            "  \"provider\": \"HighCourtAustralia\",\n",
            "  \"source_identity_ref\": \"{}\",\n",
            "  \"medium_neutral_citation\": \"[2026] HCA 19\",\n",
            "  \"landing_page_network_requests\": 0,\n",
            "  \"resource_discovery_network_requests\": 0,\n",
            "  \"document_kind\": \"Docx\",\n",
            "  \"document_reference\": \"{}\",\n",
            "  \"document_fetch\": {{\"network_requests\": 1, \"locally_ingested\": true, \"bytes_digest\": \"{}\", \"source_revision_ref\": \"{}\"}},\n",
            "  \"canonical_text\": {{\"locally_materialized\": true, \"sha256\": \"{}\", \"paragraph_count\": {}}},\n",
            "  \"replay_run\": {{\"network_requests\": 0, \"resolution\": \"Persisted\"}},\n",
            "  \"document_fetch_claimed_semantic_payment\": false,\n",
            "  \"document_fetch_claimed_legal_authority\": false,\n",
            "  \"canonical_text_claimed_semantic_payment\": false\n",
            "}}\n"
        ),
        json_escape(&runtime_head),
        json_escape(&source_identity),
        json_escape(&preferred.reference),
        json_escape(&digest),
        json_escape(&source_revision),
        json_escape(&canonical_text_digest),
        materialized.paragraph_count,
    );
    let receipt_path = output_dir.join("governed-official-judgment-acquisition-v01.json");
    fs::write(&receipt_path, receipt).expect("write official judgment acquisition receipt");

    println!(
        "provider=HighCourtAustralia document=Docx landing_network=0 document_network=1 replay_network=0 paragraphs={} text_digest={} receipt={} authority={}",
        materialized.paragraph_count,
        canonical_text_digest,
        receipt_path.display(),
        RECEIPT_AUTHORITY
    );
}

#[cfg(not(feature = "live-network"))]
fn main() {
    eprintln!("live HCA judgment DOCX smoke requires --features live-network");
}
