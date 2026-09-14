use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use sensiblaw_residual_planner::plan_active_frontier_stream;

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|value| value == key)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn required_path(args: &[String], key: &str) -> PathBuf {
    PathBuf::from(arg_value(args, key).unwrap_or_else(|| panic!("missing {key}")))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("");
    match command {
        "plan" => {
            let input_path = required_path(&args, "--frontier");
            let output_path = required_path(&args, "--output");
            let mut input = BufReader::new(File::open(input_path)?);
            let mut output = BufWriter::new(File::create(output_path)?);
            let receipt = plan_active_frontier_stream(&mut input, &mut output)?;
            output.flush()?;
            eprintln!(
                "SLR_RESIDUAL_PLANNER_RECEIPT obligations_seen={} route_intents_emitted={} candidate_only={} semantic_promotion={} route_intent_is_claim_truth={}",
                receipt.obligations_seen,
                receipt.route_intents_emitted,
                receipt.candidate_only,
                receipt.semantic_promotion,
                receipt.route_intent_is_claim_truth,
            );
        }
        _ => {
            eprintln!("usage: sensiblaw-residual-planner plan --frontier active-frontier.slrw --output route-intents.slrw");
            std::process::exit(2);
        }
    }
    Ok(())
}
