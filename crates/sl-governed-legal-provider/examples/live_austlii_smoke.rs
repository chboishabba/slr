#[cfg(feature = "live-network")]
fn main() {
    use std::collections::BTreeMap;
    use sensiblaw_governed_legal_provider::*;

    if std::env::var("SENSIBLAW_LIVE_LEGAL_OPT_IN").ok().as_deref() != Some("1") {
        eprintln!("refusing live legal acquisition: set SENSIBLAW_LIVE_LEGAL_OPT_IN=1 explicitly");
        std::process::exit(2);
    }

    let query_text = std::env::var("SENSIBLAW_AUSTLII_SMOKE_QUERY")
        .unwrap_or_else(|_| "[2026] HCA 19".to_string());
    let query = SinoQuery {
        meta: "/au".into(),
        query: query_text,
        method: SinoMethod::Phrase,
        results: 1,
        offset: 0,
        rank: None,
        callback: None,
        mask_path: Vec::new(),
        mask_by_phc: BTreeMap::new(),
    };
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
    let receipt = executor.austlii_search(&query).expect("bounded AustLII smoke search");
    println!(
        "provider=AustLII status={} network_requests={} authority={} request_url={}",
        receipt.status_code,
        executor.network_requests(),
        receipt.receipt_authority,
        receipt.request_url
    );
}

#[cfg(not(feature = "live-network"))]
fn main() {
    eprintln!("live smoke requires --features live-network");
}
