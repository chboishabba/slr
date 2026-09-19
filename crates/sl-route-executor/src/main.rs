use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use sensiblaw_route_executor::execute_selected_routes;

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
        "execute" => {
            let routes = required_path(&args, "--routes");
            let output = required_path(&args, "--output");
            let mut input = BufReader::new(File::open(routes)?);
            let mut out = BufWriter::new(File::create(output)?);
            let receipt = execute_selected_routes(&mut input, &mut out)?;
            out.flush()?;
            eprintln!(
                "SLR_ROUTE_EXECUTION_RECEIPT selected_routes_seen={} executable_routes_seen={} sources_emitted={} deferred_routes={} acquired_source_wire=SLRX acquisition_creates_claim_truth={} candidate_only={} semantic_promotion={} json_transport=false regex_parser=false",
                receipt.selected_routes_seen,
                receipt.executable_routes_seen,
                receipt.sources_emitted,
                receipt.deferred_routes,
                receipt.acquisition_creates_claim_truth,
                receipt.candidate_only,
                receipt.semantic_promotion,
            );
        }
        _ => {
            eprintln!("usage: sensiblaw-route-executor execute --routes selected-routes.slrw --output acquired-sources.slrx");
            std::process::exit(2);
        }
    }
    Ok(())
}
