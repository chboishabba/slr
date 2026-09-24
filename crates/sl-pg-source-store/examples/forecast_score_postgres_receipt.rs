use postgres::{Client, NoTls};
use sensiblaw_legal_runtime::score_forecast_cohort;
use sensiblaw_pg_source_store::{
    ensure_forecast_verification_schema, load_database_config, load_forecast_cohort,
    load_scorable_forecasts_for_cohort, persist_forecast_score_projection,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cohort_ref = std::env::args()
        .nth(1)
        .ok_or("usage: forecast_score_postgres_receipt <cohort_ref> [score_ref] [ledger_ref]")?;
    let score_ref = std::env::args()
        .nth(2)
        .unwrap_or_else(|| format!("score:{cohort_ref}:recomputed"));
    let ledger_ref = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "resolution-ledger:current".to_string());

    let config = load_database_config(None)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    ensure_forecast_verification_schema(&mut client)?;

    let cohort = load_forecast_cohort(&mut client, &cohort_ref)?;
    let rows = load_scorable_forecasts_for_cohort(&mut client, &cohort_ref)?;
    if rows.is_empty() {
        return Err(format!("cohort {cohort_ref} has no included scored forecasts").into());
    }

    let receipt = score_forecast_cohort(&score_ref, &ledger_ref, &cohort, &rows)?;
    persist_forecast_score_projection(&mut client, &receipt.projection)?;

    println!("score_ref={}", receipt.projection.score_ref);
    println!("cohort_ref={}", receipt.projection.cohort_ref);
    println!("resolution_ledger_ref={}", receipt.projection.resolution_ledger_ref);
    println!("scored_forecast_count={}", receipt.row_scores.len());
    println!(
        "brier={}/{}",
        receipt.projection.brier_mean.numerator,
        receipt.projection.brier_mean.denominator
    );
    println!("all_rows_scorable={}", receipt.all_rows_scorable);
    println!("claims_causal_attribution={}", receipt.claims_causal_attribution);
    println!("creates_semantic_authority={}", receipt.creates_semantic_authority);
    println!("creates_claim_truth={}", receipt.creates_claim_truth);

    Ok(())
}
