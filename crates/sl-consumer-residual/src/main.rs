use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use sensiblaw_consumer_residual::{compile_consumer_residual_stream, decode_consumer_spec};

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|value| value == key).and_then(|index| args.get(index + 1)).cloned()
}

fn required_path(args: &[String], key: &str) -> PathBuf {
    PathBuf::from(arg_value(args, key).unwrap_or_else(|| panic!("missing {key}")))
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
    match command {
        "compile" => {
            let world_path = required_path(&args, "--world");
            let consumer_path = required_path(&args, "--consumer");
            let output_path = required_path(&args, "--output");
            let iteration = required_i64(&args, "--iteration");

            let mut consumer_reader = BufReader::new(File::open(consumer_path)?);
            let spec = decode_consumer_spec(&mut consumer_reader)?;
            let mut world_reader = BufReader::new(File::open(world_path)?);
            let mut output = BufWriter::new(File::create(output_path)?);
            let receipt = compile_consumer_residual_stream(
                &mut world_reader,
                &spec,
                &mut output,
                iteration,
            )?;
            output.flush()?;
            eprintln!(
                "SLR_CONSUMER_RESIDUAL_RECEIPT requirements_total={} requirements_paid={} requirements_unpaid={} gaps_emitted={} obligations_emitted={} candidate_only={} semantic_promotion={}",
                receipt.requirements_total,
                receipt.requirements_paid,
                receipt.requirements_unpaid,
                receipt.gaps_emitted,
                receipt.obligations_emitted,
                receipt.candidate_only,
                receipt.semantic_promotion,
            );
        }
        _ => {
            eprintln!("usage: sensiblaw-consumer-residual compile --world world.slrw --consumer consumer.slrc --output residual-world.slrw --iteration N");
            std::process::exit(2);
        }
    }
    Ok(())
}
