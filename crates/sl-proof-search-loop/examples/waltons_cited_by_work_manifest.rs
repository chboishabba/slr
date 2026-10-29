use sensiblaw_governed_legal_provider::{
    citation_traversal_plan, OalcResolvedSourceReceipt,
};
use sensiblaw_proof_search_loop::oalc_judgment_materialization::later_treatment_cited_by_demand;
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = PathBuf::from(
        env::var("SENSIBLAW_OALC_OUTPUT")
            .unwrap_or_else(|_| "artifacts/oalc/contracts/waltons".into()),
    );
    let receipt_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("waltons-oalc-source-receipt.json"));
    let output_path = env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("waltons-cited-by-work-manifest.json"));

    let receipt: OalcResolvedSourceReceipt =
        serde_json::from_slice(&fs::read(&receipt_path)?)?;
    let demand = later_treatment_cited_by_demand(
        &receipt,
        Some("prop:estoppel:waltons-treatment".into()),
    );
    let traversal = citation_traversal_plan(&demand)
        .ok_or("Waltons CitedBy demand unexpectedly produced no traversal plan")?;

    let output = json!({
        "schema_version": "sl.cited_by_work_manifest.v0_1",
        "root_authority": {
            "source_identity_ref": demand.source_identity_ref,
            "medium_neutral_citation": demand.medium_neutral_citation,
            "source_revision_ref": format!("{}:{}", receipt.corpus_revision_ref, receipt.version_id),
            "canonical_text_digest": receipt.canonical_text_digest,
        },
        "demand": {
            "demand_ref": demand.demand_ref,
            "jurisdiction_ref": demand.jurisdiction_ref,
            "proposition_ref": demand.proposition_ref,
            "use_intent": format!("{:?}", demand.use_intent),
            "treatment_intent": format!("{:?}", demand.treatment_intent),
        },
        "traversal": {
            "provider": format!("{:?}", traversal.provider),
            "operation": format!("{:?}", traversal.operation),
            "max_depth": traversal.bounds.max_depth,
            "max_new_documents": traversal.bounds.max_new_documents,
            "max_network_requests": traversal.bounds.max_network_requests,
            "minimum_pacing_seconds": traversal.bounds.minimum_pacing_seconds,
            "acquisition_authority": format!("{:?}", traversal.acquisition_authority),
        },
        "execution": {
            "state": "provider_adapter_required",
            "text_search_is_not_cited_by_traversal": true,
            "provider_failure_is_negative_legal_evidence": false,
            "candidate_only": true,
            "creates_legal_authority": false,
        }
    });

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&output)?)?;
    println!(
        "waltons_cited_by_manifest={} provider={:?} operation={:?} execution=provider_adapter_required",
        output_path.display(),
        traversal.provider,
        traversal.operation,
    );
    Ok(())
}
