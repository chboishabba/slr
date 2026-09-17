use std::io::Cursor;

use sensiblaw_consumer_residual::compile_consumer_residual_stream;
use sensiblaw_pg_source_store::{
    load_database_config, load_discovery_identity_baseline, load_latent_world_rows_with_budget,
    LatentWorldBudget,
};
use sensiblaw_world_expansion_runtime::diagnose_mabo_context_world_identity;

const MABO_QID: &str = "Q1501525";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_database_config(None)?;
    let baseline = load_discovery_identity_baseline(&config)?;
    let world = load_latent_world_rows_with_budget(
        &config,
        MABO_QID,
        LatentWorldBudget {
            max_hops: 100,
            max_nodes: 10_000,
            max_edges: 50_000,
        },
    )?;

    if world.creates_semantic_authority
        || world.applicability_promoted
        || world.claim_truth_promoted
    {
        return Err(std::io::Error::other(
            "latent-world traversal unexpectedly promoted semantic state",
        )
        .into());
    }

    let diagnosis = diagnose_mabo_context_world_identity(&world, &baseline);
    if diagnosis.creates_semantic_authority
        || diagnosis.applicability_promoted
        || diagnosis.claim_truth_promoted
    {
        return Err(std::io::Error::other(
            "consumer diagnosis unexpectedly promoted semantic state",
        )
        .into());
    }

    // Compile against an empty payment stream. The point of this executable is
    // to materialise the *open* consumer frontier from the observed 100-hop
    // world, not to pretend review/payment has already happened.
    let mut empty_world = Cursor::new(Vec::<u8>::new());
    let mut residual_wire = Vec::new();
    let residual_receipt = compile_consumer_residual_stream(
        &mut empty_world,
        &diagnosis.consumer_spec,
        &mut residual_wire,
        0,
    )?;

    println!("seed_ref={}", world.seed_ref);
    println!("requested_max_hops={}", world.requested_max_hops);
    println!("deepest_observed_hop={}", world.deepest_observed_hop);
    println!("visited_refs={}", world.visited_refs.len());
    println!("edges={}", world.edges.len());
    println!("frontier_exhausted={}", world.frontier_exhausted);
    println!("traversal_residuals={:?}", world.residual_refs);
    println!(
        "durable_baseline_identity_classes={}",
        baseline.identity_class_refs.len()
    );
    println!(
        "reviewed_context_edges_considered={}",
        diagnosis.reviewed_context_edges_considered
    );
    println!(
        "known_identity_representations={}",
        diagnosis.known_identity_representations
    );
    println!("duplicate_target_edges={}", diagnosis.duplicate_target_edges);
    println!(
        "out_of_scope_or_wrong_type_edges={}",
        diagnosis.out_of_scope_or_wrong_type_edges
    );
    println!(
        "open_identity_requirements={}",
        diagnosis.consumer_spec.requirements.len()
    );
    println!("requirements_total={}", residual_receipt.requirements_total);
    println!("requirements_paid={}", residual_receipt.requirements_paid);
    println!("requirements_unpaid={}", residual_receipt.requirements_unpaid);
    println!("gaps_emitted={}", residual_receipt.gaps_emitted);
    println!("obligations_emitted={}", residual_receipt.obligations_emitted);
    println!("payments_emitted={}", residual_receipt.payments_emitted);
    println!("residual_wire_bytes={}", residual_wire.len());

    for residual in diagnosis.residuals.iter().take(25) {
        println!("open_residual={}", residual.residual_ref);
    }
    for row in diagnosis.rows.iter().take(25) {
        println!(
            "identity_review_demand={} source_revisions={:?} relation_types={:?} discovery_route={}",
            row.representation_ref,
            row.source_revision_refs,
            row.relation_type_refs,
            row.discovery_route_ref
        );
    }

    println!("candidate_only=true");
    println!("creates_semantic_authority=false");
    println!("applicability_promoted=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
