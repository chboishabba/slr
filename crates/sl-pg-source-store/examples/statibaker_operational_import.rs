use sensiblaw_pg_source_store::{
    load_database_config, load_statibaker_activity_ledger,
    materialize_statibaker_activity_ledger,
};

fn main() -> Result<(), String> {
    let ledger_path = std::env::args()
        .nth(1)
        .ok_or_else(|| "usage: statibaker_operational_import <activity_ledger.json> <YYYY-MM-DD>".to_owned())?;
    let state_date = std::env::args()
        .nth(2)
        .ok_or_else(|| "usage: statibaker_operational_import <activity_ledger.json> <YYYY-MM-DD>".to_owned())?;

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let ledger =
        load_statibaker_activity_ledger(&ledger_path).map_err(|error| error.to_string())?;
    let receipt =
        materialize_statibaker_activity_ledger(&config, &state_date, &ledger)
            .map_err(|error| error.to_string())?;

    println!("state_date={}", receipt.state_date);
    println!("producer_algorithm={}", receipt.producer_algorithm);
    println!("producer_input_hash={}", receipt.producer_input_hash);
    println!("event_count={}", receipt.event_count);
    println!(
        "creates_semantic_authority={}",
        receipt.creates_semantic_authority
    );
    println!("pays_evidence={}", receipt.pays_evidence);
    println!("claim_truth_promoted={}", receipt.claim_truth_promoted);

    if receipt.creates_semantic_authority
        || receipt.pays_evidence
        || receipt.claim_truth_promoted
    {
        return Err("StatiBaker import crossed semantic boundary".into());
    }

    for event_ref in receipt.operational_event_refs {
        println!("operational_event={event_ref}");
    }

    Ok(())
}
