use std::env;
use std::path::{Path, PathBuf};

use sensiblaw_world_expansion_runtime::digital_esd_world::materialize_digital_esd_world;
use sensiblaw_world_store::load_database_config;

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|x| x == key).and_then(|i| args.get(i + 1)).cloned()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let artifact_root = env::var("DIGITAL_ESD_ARTIFACT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("artifacts/digital-esd/real-eric"));

    let processing_ledger = arg_value(&args, "--processing-ledger")
        .map(PathBuf::from)
        .unwrap_or_else(|| artifact_root.join("slr-parse/study-processing-ledger.jsonl"));

    let corpus_ref = arg_value(&args, "--corpus-ref")
        .unwrap_or_else(|| "digital-esd:eric:43996".into());

    let compiler_ref = arg_value(&args, "--compiler-ref")
        .unwrap_or_else(|| "sensiblaw:db-native-world:v0_1".into());

    let limit = arg_value(&args, "--limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100);

    let env_file = arg_value(&args, "--env-file").map(PathBuf::from);
    let config = load_database_config(env_file.as_deref().map(Path::new))?;

    let receipt = materialize_digital_esd_world(
        &config,
        &processing_ledger,
        &corpus_ref,
        &compiler_ref,
        limit,
    )?;

    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}
