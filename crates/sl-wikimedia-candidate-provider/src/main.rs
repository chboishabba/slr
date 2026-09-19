use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, fetch_and_emit, fetch_and_emit_revision,
};
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|value| value == key)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn required(args: &[String], key: &str) -> String {
    arg_value(args, key).unwrap_or_else(|| panic!("missing {key}"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("");
    match command {
        "fetch" => {
            let qid = required(&args, "--qid");
            let output = PathBuf::from(required(&args, "--output"));
            let mut writer = BufWriter::new(File::create(output)?);
            let receipt = match arg_value(&args, "--revision") {
                Some(value) => fetch_and_emit_revision(&qid, value.parse()?, &mut writer)?,
                None => fetch_and_emit(&qid, &mut writer)?,
            };
            writer.flush()?;
            eprintln!("SLR_WIKIMEDIA_RDF_PROVIDER_RECEIPT qid={} direct_property_candidates={} wikipedia_article_candidates={} producer_search_candidates={} parser_repair_candidates={} rdf_xml={} json_transport={} regex_parser={} route_candidate_is_claim_truth={} semantic_promotion={}",
                qid,
                receipt.direct_property_candidates,
                receipt.wikipedia_article_candidates,
                receipt.producer_search_candidates,
                receipt.parser_repair_candidates,
                receipt.rdf_xml,
                receipt.json_transport,
                receipt.regex_parser,
                receipt.route_candidate_is_claim_truth,
                receipt.semantic_promotion,
            );
        }
        "from-rdf" => {
            let qid = required(&args, "--qid");
            let input = PathBuf::from(required(&args, "--input"));
            let output = PathBuf::from(required(&args, "--output"));
            let mut reader = BufReader::new(File::open(input)?);
            let mut writer = BufWriter::new(File::create(output)?);
            let receipt = emit_candidates_from_rdf(&qid, &mut reader, &mut writer)?;
            writer.flush()?;
            eprintln!("SLR_WIKIMEDIA_RDF_PROVIDER_RECEIPT qid={} direct_property_candidates={} wikipedia_article_candidates={} producer_search_candidates={} parser_repair_candidates={} rdf_xml={} json_transport={} regex_parser={} route_candidate_is_claim_truth={} semantic_promotion={}",
                qid,
                receipt.direct_property_candidates,
                receipt.wikipedia_article_candidates,
                receipt.producer_search_candidates,
                receipt.parser_repair_candidates,
                receipt.rdf_xml,
                receipt.json_transport,
                receipt.regex_parser,
                receipt.route_candidate_is_claim_truth,
                receipt.semantic_promotion,
            );
        }
        _ => {
            eprintln!("usage: sensiblaw-wikimedia-candidate-provider fetch --qid QID [--revision ID] --output candidates.slrg");
            eprintln!("   or: sensiblaw-wikimedia-candidate-provider from-rdf --qid QID --input entity.rdf --output candidates.slrg");
            std::process::exit(2);
        }
    }
    Ok(())
}
