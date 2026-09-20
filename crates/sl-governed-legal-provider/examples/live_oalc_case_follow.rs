#[cfg(not(feature = "live-network"))]
fn main() {
    eprintln!("enable --features live-network to run governed OALC case follow");
}

#[cfg(feature = "live-network")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use sensiblaw_governed_legal_provider::{
        run_live_oalc_case_follow, OalcCaseFollowRequest,
    };
    use std::env;
    use std::path::PathBuf;

    let values = env::args().skip(1).collect::<Vec<_>>();
    if !values.iter().any(|arg| arg == "--operator-opt-in") {
        return Err("explicit --operator-opt-in required".into());
    }
    fn value(values: &[String], flag: &str) -> Option<String> {
        values
            .iter()
            .position(|arg| arg == flag)
            .and_then(|index| values.get(index + 1))
            .cloned()
    }

    let citation = value(&values, "--citation")
        .ok_or("--citation '[YYYY] COURT N' required")?;
    let output_dir = value(&values, "--output-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let safe = citation
                .replace('[', "")
                .replace(']', "")
                .replace(' ', "-")
                .to_ascii_lowercase();
            PathBuf::from(format!("artifacts/oalc/cases/{safe}"))
        });

    let mut request = OalcCaseFollowRequest::for_citation(citation, output_dir);
    if let Some(court) = value(&values, "--court-ref") {
        request.court_ref = court;
    }
    if let Some(jurisdiction) = value(&values, "--oalc-jurisdiction") {
        request.oalc_jurisdiction = jurisdiction;
    }
    if let Some(jurisdiction) = value(&values, "--legal-jurisdiction") {
        request.legal_jurisdiction = jurisdiction;
    }
    if let Some(as_at) = value(&values, "--as-at") {
        request.as_at = as_at;
    }

    let receipt = run_live_oalc_case_follow(&request)
        .map_err(|error| format!("governed OALC case acquisition failed: {error:?}"))?;

    println!(
        "oalc_case_receipt={} citation={} revision={} version={} path={} network={} authority=experimental_candidate_only",
        receipt.source_receipt_path.display(),
        receipt.citation,
        receipt.corpus_revision_ref,
        receipt.version_id,
        receipt.resolution_path,
        receipt.network_requests,
    );
    Ok(())
}
