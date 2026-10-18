use sensiblaw_pg_source_store::{
    load_current_legal_controversy_residuals, load_database_config,
    persist_reviewed_residual_supersession, ResidualSupersessionDraft,
};

fn required_arg(args: &[String], index: usize, name: &str) -> Result<String, String> {
    args.get(index)
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing required {name}"))
}

fn main() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 5 {
        return Err(
            "usage: reconcile_legal_controversy_residual_head \
<controversy_ref> <successor_residual_ref> <review_command_ref> \
<predecessor_residual_ref> [predecessor_residual_ref ...]"
                .into(),
        );
    }

    let controversy_ref = required_arg(&args, 1, "controversy_ref")?;
    let successor_residual_ref = required_arg(&args, 2, "successor_residual_ref")?;
    let review_command_ref = required_arg(&args, 3, "review_command_ref")?;
    let predecessor_residual_refs = args[4..]
        .iter()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if predecessor_residual_refs.is_empty() {
        return Err("at least one historical predecessor residual is required".into());
    }

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let receipt = persist_reviewed_residual_supersession(
        &config,
        &ResidualSupersessionDraft {
            controversy_ref: controversy_ref.clone(),
            successor_residual_ref,
            predecessor_residual_refs,
            review_command_ref,
        },
    )
    .map_err(|error| error.to_string())?;

    let current = load_current_legal_controversy_residuals(&config, &controversy_ref)
        .map_err(|error| error.to_string())?;

    println!("supersession_ref={}", receipt.supersession_ref);
    println!("controversy_ref={}", receipt.controversy_ref);
    println!("lane_ref={}", receipt.lane_ref);
    println!("successor_residual_ref={}", receipt.successor_residual_ref);
    println!(
        "predecessor_residual_refs={}",
        receipt.predecessor_residual_refs.join(",")
    );
    println!("review_command_ref={}", receipt.review_command_ref);
    println!(
        "current_residual_refs={}",
        current
            .iter()
            .map(|residual| residual.residual_ref.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("creates_semantic_authority=false");
    println!("creates_legal_authority=false");
    println!("applicability_promoted=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
