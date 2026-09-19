use std::env;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::PathBuf;

use sensiblaw_world_store::{load_database_config, WorldStore};

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|x| x == key).and_then(|i| args.get(i + 1)).cloned()
}

fn env_file(args: &[String]) -> Option<PathBuf> {
    arg_value(args, "--env-file").map(PathBuf::from)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("");
    let config = load_database_config(env_file(&args).as_deref())?;
    let mut store = WorldStore::connect(&config)?;
    match command {
        "ingest-wire" => {
            let receipt = if let Some(path) = arg_value(&args, "--input") {
                store.ingest_wire(BufReader::new(File::open(path)?))?
            } else {
                store.ingest_wire(BufReader::new(io::stdin().lock()))?
            };
            println!(
                "SLR_WORLD_BINARY_INGEST_RECEIPT records={} copy_streams={} binary_wire={} postgres_persistence_is_semantic_authority={} semantic_promotion={}",
                receipt.records,
                receipt.copy_streams,
                receipt.binary_wire,
                receipt.postgres_persistence_is_semantic_authority,
                receipt.semantic_promotion,
            );
        }
        "frontier" => {
            let stdout = io::stdout();
            let mut out = BufWriter::new(stdout.lock());
            let rows = store.write_latest_frontier(&mut out)?;
            out.flush()?;
            eprintln!(
                "SLR_WORLD_FRONTIER_BINARY_STREAM_RECEIPT rows={} buffered_full_frontier=false binary_wire=true candidate_only=true semantic_promotion=false",
                rows
            );
        }
        _ => {
            eprintln!("usage: sensiblaw-world-store <ingest-wire|frontier> [--env-file .env] [--input stream.slrw]");
            std::process::exit(2);
        }
    }
    Ok(())
}
