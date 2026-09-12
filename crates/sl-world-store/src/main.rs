use std::env;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::PathBuf;

use sensiblaw_world_store::{load_database_config, records_from_round_files, WorldStore};

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|x| x == key).and_then(|i| args.get(i + 1)).cloned()
}

fn env_file(args: &[String]) -> Option<PathBuf> {
    arg_value(args, "--env-file").map(PathBuf::from)
}

fn required_path(args: &[String], key: &str) -> PathBuf {
    PathBuf::from(arg_value(args, key).unwrap_or_else(|| panic!("missing {key}")))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("");
    let config = load_database_config(env_file(&args).as_deref())?;
    let mut store = WorldStore::connect(&config)?;
    match command {
        "ingest-round" => {
            let records = records_from_round_files(
                &required_path(&args, "--article-pnf"),
                &required_path(&args, "--closure"),
                &required_path(&args, "--route-plan"),
                &required_path(&args, "--iteration"),
            )?;
            let receipt = store.ingest_records(records)?;
            println!("{}", serde_json::to_string(&receipt)?);
        }
        "ingest-ndjson" => {
            let receipt = if let Some(path) = arg_value(&args, "--input") {
                store.ingest_ndjson(BufReader::new(File::open(path)?))?
            } else {
                store.ingest_ndjson(BufReader::new(io::stdin().lock()))?
            };
            println!("{}", serde_json::to_string(&receipt)?);
        }
        "frontier" => {
            let stdout = io::stdout();
            let mut out = BufWriter::new(stdout.lock());
            let rows = store.write_latest_frontier(&mut out)?;
            out.flush()?;
            eprintln!(
                "SLR_WORLD_FRONTIER_STREAM_RECEIPT rows={} buffered_full_frontier=false candidate_only=true semantic_promotion=false",
                rows
            );
        }
        _ => {
            eprintln!("usage: sensiblaw-world-store <ingest-round|ingest-ndjson|frontier> [--env-file .env] ...");
            std::process::exit(2);
        }
    }
    Ok(())
}
