use sensiblaw_governed_legal_provider::{
    run_live_oalc_case_follow, OalcCaseFollowRequest,
};
use std::path::PathBuf;

fn value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

pub fn run(args: Vec<String>) -> Result<(), String> {
    if args.first().map(String::as_str) != Some("acquire") {
        return Err(
            "usage: sensiblaw legal-follow case acquire --citation '[YYYY] COURT N' [--court-ref REF] [--oalc-jurisdiction J] [--legal-jurisdiction J] [--as-at YYYY-MM-DD] [--output-dir PATH]"
                .into(),
        );
    }
    let citation = value(&args, "--citation")
        .ok_or_else(|| "--citation '[YYYY] COURT N' is required".to_string())?;
    let safe = citation
        .replace('[', "")
        .replace(']', "")
        .replace(' ', "-")
        .to_ascii_lowercase();
    let output_dir = value(&args, "--output-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("artifacts/oalc/cases/{safe}")));

    let mut request = OalcCaseFollowRequest::for_citation(citation, output_dir);
    if let Some(court_ref) = value(&args, "--court-ref") {
        request.court_ref = court_ref;
    }
    if let Some(jurisdiction) = value(&args, "--oalc-jurisdiction") {
        request.oalc_jurisdiction = jurisdiction;
    }
    if let Some(jurisdiction) = value(&args, "--legal-jurisdiction") {
        request.legal_jurisdiction = jurisdiction;
    }
    if let Some(as_at) = value(&args, "--as-at") {
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
