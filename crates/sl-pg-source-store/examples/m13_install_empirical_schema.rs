use sensiblaw_pg_source_store::{
    install_m13_empirical_schema, load_database_config,
};

fn main() -> Result<(), String> {
    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let receipt = install_m13_empirical_schema(&config)?;

    println!("statement_trace={}", receipt.statement_trace);
    println!("chat_source={}", receipt.chat_source);
    println!(
        "chronology_contestation={}",
        receipt.chronology_contestation
    );
    println!("event_discovery={}", receipt.event_discovery);
    println!("review_workstation={}", receipt.review_workstation);
    println!("operational_state={}", receipt.operational_state);
    println!(
        "creates_semantic_authority={}",
        receipt.creates_semantic_authority
    );
    println!("claim_truth_promoted={}", receipt.claim_truth_promoted);
    println!(
        "canonical_world_mutated={}",
        receipt.canonical_world_mutated
    );

    if receipt.creates_semantic_authority
        || receipt.claim_truth_promoted
        || receipt.canonical_world_mutated
    {
        return Err("M13 empirical schema installer crossed semantic boundary".into());
    }

    Ok(())
}
