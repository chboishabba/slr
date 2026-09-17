#[path = "../src/reviewed_campaign.rs"]
mod reviewed_campaign;

use std::env;
use std::fs;
use std::io::Cursor;

use reviewed_campaign::{
    prepare_reviewed_mabo_identity_cycle, reviewed_acquisition_request,
};
use sensiblaw_pg_source_store::{
    load_database_config, load_discovery_identity_baseline, load_latent_world_rows_with_budget,
    LatentWorldBudget,
};
use sensiblaw_proof_search_loop::world_expansion_runner::{
    run_recurrent_world_expansion, WorldExpansionRunnerConfig,
};
use sensiblaw_proof_search_loop::world_expansion_session::WorldExpansionSession;
use sensiblaw_proof_search_loop::world_identity_guard::IdentityCoherentCycleSource;
use sensiblaw_route_selector::{decode_route_candidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, fetch_entity_rdf_revision_receipt,
};
use sensiblaw_world_expansion_runtime::{
    diagnose_mabo_context_world_identity, durable_mabo_campaign_total,
    identity_coherence_baseline, mabo_consumer_diagnosis_frontier,
    mabo_remaining_world_expansion_policy, parse_mabo_identity_review_tsv,
    plan_mabo_identity_reviews, PgDiscoveryLineageSink, ReviewedCycleQueueSource,
    WorldStoreReviewedPaymentSink, MABO_NOVEL_IDENTITY_TARGET,
};
use sensiblaw_world_store::{load_database_config as load_world_database_config, WorldStore};

const MABO_QID: &str = "Q1501525";

fn required_arg(args: &[String], index: usize, name: &str) -> Result<String, std::io::Error> {
    args.get(index)
        .cloned()
        .ok_or_else(|| std::io::Error::other(format!("missing required argument: {name}")))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    let review_manifest_path = required_arg(&args, 1, "review-manifest.tsv")?;
    let max_cycles = args
        .get(2)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(1000);

    let pg_config = load_database_config(None)?;
    let baseline = load_discovery_identity_baseline(&pg_config)?;
    let world = load_latent_world_rows_with_budget(
        &pg_config,
        MABO_QID,
        LatentWorldBudget {
            max_hops: 100,
            max_nodes: 10_000,
            max_edges: 50_000,
        },
    )?;
    let diagnosis = diagnose_mabo_context_world_identity(&world, &baseline);
    let frontier = mabo_consumer_diagnosis_frontier(
        &diagnosis,
        "frontier:mabo-context-world-identity:reviewed-campaign",
    );

    let review_manifest = fs::read_to_string(&review_manifest_path)?;
    let reviews = parse_mabo_identity_review_tsv(&review_manifest)?;
    let plan = plan_mabo_identity_reviews(&diagnosis, &reviews)?;

    if plan.matched.is_empty() {
        println!("mode=blocked");
        println!("blocker=identity-review-required");
        println!("diagnosed_open_identity_requirements={}", diagnosis.rows.len());
        println!("pending_review_rows={}", plan.pending_rows.len());
        println!("unmatched_review_assignments={}", plan.unmatched_assignments.len());
        println!("durable_baseline_identity_classes={}", baseline.identity_class_refs.len());
        println!("candidate_only=true");
        println!("creates_semantic_authority=false");
        println!("applicability_promoted=false");
        println!("claim_truth_promoted=false");
        return Ok(());
    }

    let mut prepared_cycles = Vec::with_capacity(plan.matched.len());
    for (offset, planned) in plan.matched.iter().enumerate() {
        let request = reviewed_acquisition_request(planned)?;
        let acquired = fetch_entity_rdf_revision_receipt(&request.source_qid, request.revision_id)?;

        let mut encoded = Vec::new();
        emit_candidates_from_rdf(
            &request.source_qid,
            Cursor::new(acquired.rdf_bytes.as_slice()),
            &mut encoded,
        )?;
        let mut cursor = Cursor::new(encoded);
        let mut route = None;
        while let Some(candidate) = decode_route_candidate(&mut cursor)? {
            if candidate.route_family == RouteFamily::WikidataProperty
                && candidate.source_ref == request.source_qid
                && candidate.property_ref == request.property_ref
                && candidate.target_ref == request.target_ref
            {
                route = Some(candidate);
                break;
            }
        }
        let route = route.ok_or_else(|| {
            std::io::Error::other(format!(
                "diagnosed pinned Wikidata route absent: {} --{}--> {} @ {}",
                request.source_qid,
                request.property_ref,
                request.target_ref,
                request.source_revision_ref
            ))
        })?;

        prepared_cycles.push(prepare_reviewed_mabo_identity_cycle(
            &diagnosis,
            planned,
            &acquired,
            &route,
            i64::try_from(offset).unwrap_or(i64::MAX),
        )?);
    }

    let world_config = load_world_database_config(None)?;
    let world_store = WorldStore::connect(&world_config)?;
    let payment_sink = WorldStoreReviewedPaymentSink::new(world_store);
    let reviewed_source = ReviewedCycleQueueSource::new(prepared_cycles, payment_sink);
    let mut source = IdentityCoherentCycleSource::with_baseline(
        reviewed_source,
        identity_coherence_baseline(&baseline),
    );
    let mut sink = PgDiscoveryLineageSink::new(pg_config);
    let mut session = WorldExpansionSession::new(frontier, mabo_remaining_world_expansion_policy(&baseline));

    let receipt = run_recurrent_world_expansion(
        &mut session,
        &mut source,
        &mut sink,
        WorldExpansionRunnerConfig { max_cycles },
    )?;
    let durable_total = durable_mabo_campaign_total(&baseline, receipt.final_novel_identity_classes);

    println!("mode=reviewed-recurrent-campaign");
    println!("review_manifest={review_manifest_path}");
    println!("requested_max_hops={}", world.requested_max_hops);
    println!("deepest_observed_hop={}", world.deepest_observed_hop);
    println!("frontier_exhausted={}", world.frontier_exhausted);
    println!("diagnosed_open_identity_requirements={}", diagnosis.rows.len());
    println!("reviewed_cycles_prepared={}", plan.matched.len());
    println!("pending_review_rows={}", plan.pending_rows.len());
    println!("unmatched_review_assignments={}", plan.unmatched_assignments.len());
    println!("cycles_committed={}", receipt.cycles_committed);
    println!("durable_baseline_identity_classes={}", baseline.identity_class_refs.len());
    println!("new_novel_identity_classes={}", receipt.final_novel_identity_classes);
    println!("durable_total_identity_classes={durable_total}");
    println!("target_identity_classes={MABO_NOVEL_IDENTITY_TARGET}");
    println!("remaining_open_residuals={}", receipt.remaining_open_residual_refs.len());
    println!("stop_reason={:?}", receipt.stop_reason);
    println!("candidate_only=true");
    println!("creates_semantic_authority=false");
    println!("applicability_promoted=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
