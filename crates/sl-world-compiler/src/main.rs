use std::env;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::PathBuf;

use sensiblaw_world_compiler::compile_observation_stream;

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|x| x == key).and_then(|i| args.get(i + 1)).cloned()
}

fn required_i64(args: &[String], key: &str) -> i64 {
    arg_value(args, key)
        .unwrap_or_else(|| panic!("missing {key}"))
        .parse::<i64>()
        .unwrap_or_else(|_| panic!("invalid integer for {key}"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("");
    if command != "compile" {
        eprintln!("usage: sensiblaw-world-compiler compile --iteration N [--input observations.slro] [--output world.slrw]");
        std::process::exit(2);
    }

    let iteration = required_i64(&args, "--iteration");
    let input_path = arg_value(&args, "--input").map(PathBuf::from);
    let output_path = arg_value(&args, "--output").map(PathBuf::from);

    let mut input: Box<dyn io::Read> = if let Some(path) = input_path {
        Box::new(BufReader::new(File::open(path)?))
    } else {
        Box::new(BufReader::new(io::stdin().lock()))
    };
    let mut output: Box<dyn Write> = if let Some(path) = output_path {
        Box::new(BufWriter::new(File::create(path)?))
    } else {
        Box::new(BufWriter::new(io::stdout().lock()))
    };

    let receipt = compile_observation_stream(&mut input, &mut output, iteration)?;
    output.flush()?;
    eprintln!(
        "SLR_WORLD_COMPILER_BINARY_RECEIPT manifestations={} pnf_candidates={} world_atoms={} unresolved_dependencies={} candidate_only={} semantic_promotion={} json_transport=false regex_parser=false",
        receipt.manifestations,
        receipt.pnf_candidates,
        receipt.world_atoms,
        receipt.unresolved_dependencies,
        receipt.candidate_only,
        receipt.semantic_promotion,
    );
    Ok(())
}
