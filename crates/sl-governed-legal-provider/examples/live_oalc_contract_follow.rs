#[cfg(not(feature = "live-network"))]
fn main() {
    eprintln!("enable --features live-network to run governed OALC contract follow");
}

#[cfg(feature = "live-network")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use sensiblaw_governed_legal_provider::{
        run_live_oalc_case_follow, OalcCaseFollowRequest,
    };
    use std::env;
    use std::path::PathBuf;

    if !env::args().any(|arg| arg == "--operator-opt-in") {
        return Err("explicit --operator-opt-in required".into());
    }

    let output_dir = PathBuf::from(
        env::var("SENSIBLAW_OALC_OUTPUT")
            .unwrap_or_else(|_| "artifacts/oalc/contracts/waltons".into()),
    );
    let mut request = OalcCaseFollowRequest::for_citation("[1988] HCA 7", output_dir.clone());
    request.court_ref = "court:HCA".into();
    request.oalc_jurisdiction = "commonwealth".into();
    request.legal_jurisdiction = "AU".into();
    request.as_at = "2026-09-20".into();

    let run = run_live_oalc_case_follow(&request)
        .map_err(|error| format!("Waltons OALC acquisition failed: {error:?}"))?;

    let generic_receipt = run.source_receipt_path;
    let generic_text = run.canonical_text_path;
    let waltons_receipt = output_dir.join("waltons-oalc-source-receipt.json");
    let waltons_text = output_dir.join("waltons-stores-v-maher.txt");

    if generic_receipt != waltons_receipt {
        std::fs::rename(&generic_receipt, &waltons_receipt)?;
    }
    if generic_text != waltons_text {
        std::fs::rename(&generic_text, &waltons_text)?;
    }

    let mut receipt: sensiblaw_governed_legal_provider::OalcResolvedSourceReceipt =
        serde_json::from_slice(&std::fs::read(&waltons_receipt)?)?;
    receipt.local_artifact_ref = waltons_text;
    std::fs::write(&waltons_receipt, serde_json::to_vec_pretty(&receipt)?)?;

    println!(
        "LegalFollow -> governed OALC Waltons source candidate; revision={}; version={}; path={}; network_requests={}; authority={}",
        receipt.corpus_revision_ref,
        receipt.version_id,
        receipt.resolution_path,
        receipt.network_requests,
        receipt.receipt_authority
    );
    println!("receipt_artifact={}", waltons_receipt.display());
    Ok(())
}
