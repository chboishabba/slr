use sensiblaw_pg_source_store::{
    load_database_config, materialize_discovery_lineage, DiscoveryLineageInput,
};

fn usage() -> ! {
    eprintln!(
        "usage: materialize_mabo_discovery_lineage \
         <object_ref> <discovery_parent_ref> <triggering_residual_ref> \
         <selected_candidate_ref> <producer_lane> <source_revision_ref> \
         <pnf_world_disambiguation_ref> <expected_contraction> \
         <observed_contraction> [new_residual_ref ...]\n\n\
         producer_lane: governed-legal | wikidata-identity | wikipedia-context | source-specific-provenance | other\n\
         database connection is read via the existing SLR DATABASE_URL/.env loader"
    );
    std::process::exit(2);
}

fn lane(value: &str) -> String {
    match value {
        "governed-legal"
        | "wikidata-identity"
        | "wikipedia-context"
        | "source-specific-provenance"
        | "other" => value.to_owned(),
        _ => usage(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 9 {
        usage();
    }
    let expected_residual_contraction = args[7].parse::<u64>()?;
    let observed_residual_contraction = args[8].parse::<u64>()?;
    let lineage = DiscoveryLineageInput {
        object_ref: args[0].clone(),
        discovery_parent_ref: args[1].clone(),
        triggering_residual_ref: args[2].clone(),
        selected_candidate_ref: args[3].clone(),
        producer_lane_ref: lane(&args[4]),
        source_revision_ref: args[5].clone(),
        pnf_world_disambiguation_ref: args[6].clone(),
        expected_residual_contraction,
        observed_residual_contraction,
        new_residual_refs: args[9..].to_vec(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_authority: "candidate_world_expansion_only".into(),
    };

    let config = load_database_config(None)?;
    let receipt = materialize_discovery_lineage(&config, &[lineage])?;
    println!("attempted_count={}", receipt.attempted_count);
    println!("materialized_count={}", receipt.materialized_count);
    println!("candidate_only={}", receipt.candidate_only);
    println!(
        "creates_semantic_authority={}",
        receipt.creates_semantic_authority
    );
    println!("applicability_promoted={}", receipt.applicability_promoted);
    println!("claim_truth_promoted={}", receipt.claim_truth_promoted);
    Ok(())
}
