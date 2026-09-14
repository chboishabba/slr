use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use sensiblaw_consumer_residual::{
    compile_consumer_residual_stream, decode_consumer_spec, encode_consumer_spec,
    ConsumerRequirement, ConsumerSpec, FragmentKind, RequirementScope,
};

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|value| value == key)
        .and_then(|index| args.get(index + 1))
        .cloned()
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

fn required_value(args: &[String], key: &str) -> String {
    arg_value(args, key).unwrap_or_else(|| panic!("missing {key}"))
}

fn fragment_kind(value: &str) -> FragmentKind {
    match value {
        "actor" => FragmentKind::Actor,
        "patient" => FragmentKind::Patient,
        "property" => FragmentKind::Property,
        "relation" => FragmentKind::Relation,
        "conjunction" => FragmentKind::Conjunction,
        "negation" => FragmentKind::Negation,
        "modality" => FragmentKind::Modality,
        "quantifier" => FragmentKind::Quantifier,
        "temporal" => FragmentKind::Temporal,
        "content-clause" => FragmentKind::ContentClause,
        "clause-attachment" => FragmentKind::ClauseAttachment,
        "unresolved" => FragmentKind::Unresolved,
        _ => panic!("unknown fragment kind: {value}"),
    }
}

fn requirements(args: &[String]) -> Vec<ConsumerRequirement> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] != "--requirement" {
            index += 1;
            continue;
        }
        let requirement_id = args
            .get(index + 1)
            .unwrap_or_else(|| panic!("--requirement needs ID, fragment, and scope"));
        let fragment = args
            .get(index + 2)
            .unwrap_or_else(|| panic!("--requirement needs ID, fragment, and scope"));
        let scope = args
            .get(index + 3)
            .unwrap_or_else(|| panic!("--requirement needs ID, fragment, and scope"));
        let scope = if scope == "any" {
            RequirementScope::AnySource
        } else if let Some(source) = scope.strip_prefix("source=") {
            if source.is_empty() {
                panic!("source scope requires a source manifestation ID");
            }
            RequirementScope::SourceManifestation(source.to_owned())
        } else {
            panic!("scope must be any or source=<source-manifestation-id>");
        };
        result.push(ConsumerRequirement {
            requirement_id: requirement_id.to_owned(),
            fragment: fragment_kind(fragment),
            scope,
        });
        index += 4;
    }
    if result.is_empty() {
        panic!("at least one --requirement is required");
    }
    result
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("");
    match command {
        "encode" => {
            let output_path = required_path(&args, "--output");
            let spec = ConsumerSpec {
                consumer_id: required_value(&args, "--consumer-id"),
                surface_id: required_value(&args, "--surface-id"),
                requirements: requirements(&args),
            };
            let mut output = BufWriter::new(File::create(output_path)?);
            encode_consumer_spec(&mut output, &spec)?;
            output.flush()?;
            eprintln!(
                "SLR_CONSUMER_SPEC_BINARY_RECEIPT requirements={} binary_wire=true json_transport=false regex_parser=false",
                spec.requirements.len(),
            );
        }
        "compile" => {
            let world_path = required_path(&args, "--world");
            let consumer_path = required_path(&args, "--consumer");
            let output_path = required_path(&args, "--output");
            let iteration = required_i64(&args, "--iteration");

            let mut consumer_reader = BufReader::new(File::open(consumer_path)?);
            let spec = decode_consumer_spec(&mut consumer_reader)?;
            let mut world_reader = BufReader::new(File::open(world_path)?);
            let mut output = BufWriter::new(File::create(output_path)?);
            let receipt =
                compile_consumer_residual_stream(&mut world_reader, &spec, &mut output, iteration)?;
            output.flush()?;
            eprintln!(
                "SLR_CONSUMER_RESIDUAL_RECEIPT requirements_total={} requirements_paid={} requirements_unpaid={} gaps_emitted={} obligations_emitted={} payments_emitted={} candidate_only={} semantic_promotion={} append_only_contraction=true",
                receipt.requirements_total,
                receipt.requirements_paid,
                receipt.requirements_unpaid,
                receipt.gaps_emitted,
                receipt.obligations_emitted,
                receipt.payments_emitted,
                receipt.candidate_only,
                receipt.semantic_promotion,
            );
        }
        _ => {
            eprintln!("usage: sensiblaw-consumer-residual encode --consumer-id ID --surface-id ID --requirement ID fragment any|source=MANIFEST --output consumer.slrc");
            eprintln!("   or: sensiblaw-consumer-residual compile --world world.slrw --consumer consumer.slrc --output residual-world.slrw --iteration N");
            std::process::exit(2);
        }
    }
    Ok(())
}
