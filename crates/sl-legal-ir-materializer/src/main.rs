use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use sensiblaw_legal_ir_materializer::{load_database_config, LegalIrMaterializer};

fn value_after(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

fn env_file(args: &[String]) -> Option<PathBuf> {
    value_after(args, "--env-file").map(PathBuf::from)
}

fn usage() -> ! {
    eprintln!("usage:\n  sensiblaw-legal-ir-materializer ingest [--input file.slri] [--env-file .env]\n  sensiblaw-legal-ir-materializer weld --subject-ref REF --source-revision-ref REF --span-ref REF [--env-file .env]");
    std::process::exit(2)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or_else(|| usage());
    let env_path = env_file(&args);
    let config = load_database_config(env_path.as_deref())?;
    let mut materializer = LegalIrMaterializer::connect(&config)?;

    match command {
        "ingest" => {
            let receipt = if let Some(path) = value_after(&args, "--input") {
                let file = File::open(Path::new(&path))?;
                let mut reader = BufReader::new(file);
                materializer.ingest(&mut reader)?
            } else {
                let stdin = io::stdin();
                let mut reader = stdin.lock();
                materializer.ingest(&mut reader)?
            };
            println!(
                "SLR_LEGAL_IR_INGEST_RECEIPT records_seen={} rows_inserted={} binary_wire={} append_only={} semantic_promotion={}",
                receipt.records_seen,
                receipt.rows_inserted,
                receipt.binary_wire,
                receipt.append_only,
                receipt.semantic_promotion
            );
        }
        "weld" => {
            let subject = value_after(&args, "--subject-ref").unwrap_or_else(|| usage());
            let source_revision = value_after(&args, "--source-revision-ref").unwrap_or_else(|| usage());
            let span = value_after(&args, "--span-ref").unwrap_or_else(|| usage());
            let state = materializer.exact_source_weld(&subject, &source_revision, &span)?;
            println!(
                "SLR_LEGAL_IR_SOURCE_WELD_RECEIPT exact_source_paid={} proposition_truth_paid={} applicability_paid={}",
                state.exact_source_paid,
                state.proposition_truth_paid,
                state.applicability_paid
            );
            if !state.exact_source_paid { std::process::exit(3); }
        }
        _ => usage(),
    }
    Ok(())
}
