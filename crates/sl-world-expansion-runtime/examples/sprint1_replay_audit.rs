use std::path::Path;

use sensiblaw_world_expansion_runtime::gwb_ambiguity_campaign::GWB_ADAPTIVE_CAMPAIGN_REF;
use sensiblaw_world_expansion_runtime::sprint1_acquisition_machine::load_and_replay_campaign_head;
use sensiblaw_world_store::load_database_config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_path = Path::new(".env");
    let config = load_database_config(env_path.exists().then_some(env_path))?;
    let campaign_ref =
        std::env::var("SLR_GWB_CAMPAIGN_REF").unwrap_or_else(|_| GWB_ADAPTIVE_CAMPAIGN_REF.into());

    let head = load_and_replay_campaign_head(&config, &campaign_ref)?;

    println!("campaign_ref={}", head.campaign_ref);
    println!("completed_hops={}", head.completed_hops);
    println!(
        "receipt_head={}",
        head.prior_receipt_sha256.as_deref().unwrap_or("none")
    );
    println!(
        "world_head={}",
        head.world_sha256.as_deref().unwrap_or("none")
    );
    println!(
        "frontier_head={}",
        head.last_frontier_sha256.as_deref().unwrap_or("none")
    );
    println!("producer_families={}", head.producer_refs.len());
    for producer in &head.producer_refs {
        println!("producer={producer}");
    }
    println!("rejected_or_blocked_hops={}", head.rejected_or_blocked_hops);
    println!("candidate_only={}", head.candidate_only);
    println!(
        "creates_semantic_authority={}",
        head.creates_semantic_authority
    );
    println!("applicability_promoted={}", head.applicability_promoted);
    println!("claim_truth_promoted={}", head.claim_truth_promoted);

    Ok(())
}
