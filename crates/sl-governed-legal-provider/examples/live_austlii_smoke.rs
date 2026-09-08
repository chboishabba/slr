#[cfg(feature = "live-network")]
fn main() {
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
    let citation = std::env::var("SENSIBLAW_HCA_SMOKE_CITATION")
        .unwrap_or_else(|_| "[2026] HCA 19".to_string());
    let source_identity = std::env::var("SENSIBLAW_HCA_SMOKE_SOURCE_ID")
        .unwrap_or_else(|_| "case:[2026]-HCA-19".to_string());
    let proposition = std::env::var("SENSIBLAW_HCA_SMOKE_PROPOSITION")
        .unwrap_or_else(|_| "prop:cullen-positive-operational-duty".to_string());
    let official = official_hca_known_reference(&citation)
        .expect("smoke citation must have an exact official HCA reference calibration");

    let output_dir = PathBuf::from(
        std::env::var("SENSIBLAW_LIVE_RECEIPT_DIR")
            .unwrap_or_else(|_| "/tmp/sensiblaw-live-legal".to_string()),
    );
    fs::create_dir_all(&output_dir).expect("create live receipt directory");

    let demand = KnownAuthorityDemand {
        demand_ref: "live-smoke:official-hca".into(),
        jurisdiction_ref: "AU".into(),
        source_identity_ref: source_identity.clone(),
        medium_neutral_citation: Some(citation.clone()),
        explicit_austlii_ref: None,
        proposition_ref: Some(proposition.clone()),
        use_intent: PropositionUseIntent::SourceProposition,
        treatment_intent: CitationTreatmentIntent::CitedBy,
    };

    let first_resolution = resolve_known_authority(&demand, &ResolutionContext::default());
    assert!(matches!(
        first_resolution.stage,
        ResolutionStage::OfficialHighCourt { .. }
    ));

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
    let fetched = executor.fetch_hca(&official).expect("bounded official HCA document fetch");
    assert_eq!(executor.network_requests(), 1);
    assert_eq!(fetched.network_requests, 1);
    assert_eq!(fetched.provider, LegalProvider::HighCourtAustralia);
    assert!(!fetched.locally_ingested);

    let digest = format!("sha256:{:x}", Sha256::digest(&fetched.bytes));
    let artifact_path = output_dir.join("authority.html");
    fs::write(&artifact_path, &fetched.bytes).expect("persist fetched authority bytes locally");
    let source_revision = format!("source-revision:{}", digest);
    let ingested = mark_locally_ingested(
        &fetched,
        source_identity.clone(),
        source_revision.clone(),
        digest.clone(),
    );
    assert!(ingested.locally_ingested);

    let replay_context = ResolutionContext {
        persisted: vec![PersistedAuthorityReceipt {
            source_identity_ref: source_identity.clone(),
            source_revision_ref: source_revision.clone(),
            jurisdiction_ref: "AU".into(),
            compile_eligible: true,
        }],
        ..ResolutionContext::default()
    };
    let replay = resolve_known_authority(&demand, &replay_context);
    assert!(matches!(replay.stage, ResolutionStage::Persisted { .. }));

    let receipt = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"sl.governed_legal_acquisition.v0_1\",\n",
            "  \"runtime_head\": \"{}\",\n",
            "  \"authority\": \"experimental_candidate_only\",\n",
            "  \"provider\": \"HighCourtAustralia\",\n",
            "  \"provider_access_status\": \"Available\",\n",
            "  \"source_identity_ref\": \"{}\",\n",
            "  \"proposition_ref\": \"{}\",\n",
            "  \"medium_neutral_citation\": \"{}\",\n",
            "  \"explicit_reference\": \"{}\",\n",
            "  \"first_run\": {{\"network_requests\": 1, \"locally_ingested\": true, \"bytes_digest\": \"{}\", \"source_revision_ref\": \"{}\"}},\n",
            "  \"replay_run\": {{\"network_requests\": 0, \"resolution\": \"Persisted\"}},\n",
            "  \"search_claimed_semantic_payment\": false,\n",
            "  \"acquisition_claimed_authority_receipt\": false\n",
            "}}\n"
        ),
        json_escape(&runtime_head),
        json_escape(&source_identity),
        json_escape(&proposition),
        json_escape(&citation),
        json_escape(&official),
        json_escape(&digest),
        json_escape(&source_revision),
    );
    let receipt_path = output_dir.join("governed-legal-acquisition-v01.json");
    fs::write(&receipt_path, receipt).expect("write deterministic live acquisition receipt");

    println!(
        "provider=HighCourtAustralia receipt={} first_network_requests=1 replay_network_requests=0 authority={}",
        receipt_path.display(), RECEIPT_AUTHORITY
    );
}

#[cfg(not(feature = "live-network"))]
fn main() {
    eprintln!("live smoke requires --features live-network");
}
