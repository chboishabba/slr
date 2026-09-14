use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use sensiblaw_consumer_residual::{
    compile_consumer_residual_stream, decode_consumer_spec, encode_consumer_spec,
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, FragmentKind, RequirementNeed,
    RequirementScope, CONSUMER_VERSION,
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

fn evidence_kind(value: &str) -> EvidenceCoordinateKind {
    match value {
        "source-identity" => EvidenceCoordinateKind::SourceIdentity,
        "same-object" => EvidenceCoordinateKind::SameObject,
        "authority" => EvidenceCoordinateKind::Authority,
        "mechanism" => EvidenceCoordinateKind::Mechanism,
        "quantification" => EvidenceCoordinateKind::Quantification,
        "probability" => EvidenceCoordinateKind::Probability,
        "counterfactual" => EvidenceCoordinateKind::Counterfactual,
        "instrument-comparison" => EvidenceCoordinateKind::InstrumentComparison,
        "incidence" => EvidenceCoordinateKind::Incidence,
        "classification" => EvidenceCoordinateKind::Classification,
        _ => panic!("unknown evidence coordinate kind: {value}"),
    }
}

fn scope(value: &str) -> RequirementScope {
    if value == "any" {
        RequirementScope::AnySource
    } else if let Some(source) = value.strip_prefix("source=") {
        if source.is_empty() {
            panic!("source scope requires a source manifestation ID");
        }
        RequirementScope::SourceManifestation(source.to_owned())
    } else {
        panic!("scope must be any or source=<source-manifestation-id>");
    }
}

fn requirements(args: &[String]) -> Vec<ConsumerRequirement> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--requirement" => {
                let requirement_id = args.get(index + 1).unwrap_or_else(|| panic!("--requirement needs ID, fragment, and scope"));
                let fragment = args.get(index + 2).unwrap_or_else(|| panic!("--requirement needs ID, fragment, and scope"));
                let scope_value = args.get(index + 3).unwrap_or_else(|| panic!("--requirement needs ID, fragment, and scope"));
                result.push(ConsumerRequirement {
                    requirement_id: requirement_id.to_owned(),
                    need: RequirementNeed::PnfFragment(fragment_kind(fragment)),
                    scope: scope(scope_value),
                });
                index += 4;
            }
            "--evidence-requirement" => {
                let requirement_id = args.get(index + 1).unwrap_or_else(|| panic!("--evidence-requirement needs ID, coordinate, and scope"));
                let kind = args.get(index + 2).unwrap_or_else(|| panic!("--evidence-requirement needs ID, coordinate, and scope"));
                let scope_value = args.get(index + 3).unwrap_or_else(|| panic!("--evidence-requirement needs ID, coordinate, and scope"));
                result.push(ConsumerRequirement {
                    requirement_id: requirement_id.to_owned(),
                    need: RequirementNeed::EvidenceCoordinate(evidence_kind(kind)),
                    scope: scope(scope_value),
                });
                index += 4;
            }
            _ => index += 1,
        }
    }
    if result.is_empty() {
        panic!("at least one --requirement or --evidence-requirement is required");
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
                "SLR_CONSUMER_SPEC_BINARY_RECEIPT consumer_wire_version={} requirements={} binary_wire=true json_transport=false regex_parser=false",
                CONSUMER_VERSION,
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
            let receipt = compile_consumer_residual_stream(&mut world_reader, &spec, &mut output, iteration)?;
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
            eprintln!("usage: sensiblaw-consumer-residual encode --consumer-id ID --surface-id ID [--requirement ID fragment any|source=MANIFEST] [--evidence-requirement ID coordinate any|source=MANIFEST] --output consumer.slrc");
            eprintln!("   or: sensiblaw-consumer-residual compile --world world.slrw --consumer consumer.slrc --output residual-world.slrw --iteration N");
            std::process::exit(2);
        }
    }
    Ok(())
}
