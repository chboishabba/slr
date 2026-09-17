use std::fs;
use std::io::Cursor;

use sensiblaw_world_expansion_runtime::reviewed_campaign::{
    self, prepare_reviewed_mabo_identity_cycle, reviewed_acquisition_request,
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
use sensiblaw_route_selector::{decode_route_candidate, RouteCandidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, fetch_entity_rdf_revision_receipt,
};
use sensiblaw_world_expansion_runtime::{
    apply_known_identity_payment_transaction, diagnose_mabo_context_world_identity,
    durable_mabo_campaign_total, identity_coherence_baseline, mabo_consumer_diagnosis_frontier,
    mabo_remaining_world_expansion_policy, parse_mabo_identity_review_tsv,
    plan_mabo_identity_reviews, PgDiscoveryLineageSink, ReviewedCycleQueueSource,
    ReviewedPreparedCycle, WorldStoreReviewedPaymentSink, MABO_NOVEL_IDENTITY_TARGET,
};
use sensiblaw_world_store::{load_database_config as load_world_database_config, WorldStore};

const MABO_QID: &str = "Q1501525";
const MAX_HOPS: u32 = 100;
const MAX_NODES: usize = 10_000;
const MAX_EDGES: usize = 50_000;

fn matching_route(
    request: &reviewed_campaign::ReviewedAcquisitionRequest,
    acquired: &sensiblaw_wikimedia_candidate_provider::AcquiredEntityRdf,
) -> Result<RouteCandidate, Box<dyn std::error::Error>> {
    let mut encoded = Vec::new();
    emit_candidates_from_rdf(
        &acquired.qid,
        Cursor::new(acquired.rdf_bytes.as_slice()),
        &mut encoded,
    )?;
    let mut cursor = Cursor::new(encoded);
    let mut matches = Vec::new();
    while let Some(candidate) = decode_route_candidate(&mut cursor)? {
        if candidate.route_family == RouteFamily::WikidataProperty
            && candidate.source_ref == request.source_qid
            && candidate.target_ref == request.target_ref
            && candidate.property_ref == request.property_ref
        {
            matches.push(candidate);
        }
    }
    matches.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    match matches.as_slice() {
        [route] => Ok(route.clone()),
        [] => Err(std::io::Error::other(format!(
            "no pinned reviewed Wikidata route for {} --{}--> {} at {}",
            request.source_qid,
            request.property_ref,
            request.target_ref,
            request.source_revision_ref
        ))
        .into()),
        _ => Err(std::io::Error::other(format!(
            "multiple pinned reviewed Wikidata routes for {} --{}--> {} at {}",
            request.source_qid,
            request.property_ref,
            request.target_ref,
            request.source_revision_ref
        ))
        .into()),
    }
}

fn print_pending(plan: &sensiblaw_world_expansion_runtime::MaboIdentityReviewPlan) {
    for row in &plan.pending_rows {
        println!(
            "pending_identity_review={}\trequired_scope={:?}\trelation_types={:?}",
            row.representation_ref, row.source_revision_refs, row.relation_type_refs
        );
        println!(
            "review_manifest_template={}\tworld-object:<reviewed-id>\treview:<operator-ref>",
            row.representation_ref
        );
    }
    for assignment in &plan.unmatched_assignments {
        println!(
            "unmatched_review_assignment={}\t{}\t{}",
            assignment.representation_ref, assignment.identity_class_ref, assignment.review_ref
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pg_config = load_database_config(None)?;
    let baseline = load_discovery_identity_baseline(&pg_config)?;
    let world = load_latent_world_rows_with_budget(
        &pg_config,
        MABO_QID,
        LatentWorldBudget {
            max_hops: MAX_HOPS,
            max_nodes: MAX_NODES,
            max_edges: MAX_EDGES,
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
    let frontier = mabo_consumer_diagnosis_frontier(
        &diagnosis,
        "frontier:mabo-context-world-identity:campaign:0",
    );
    let remaining_policy = mabo_remaining_world_expansion_policy(&baseline);
    let mut session = WorldExpansionSession::new(frontier, remaining_policy);

    let review_path = std::env::args().nth(1);
    let review_text = match review_path.as_deref() {
        Some(path) => fs::read_to_string(path)?,
        None => String::new(),
    };
    let reviews = parse_mabo_identity_review_tsv(&review_text)?;
    let plan = plan_mabo_identity_reviews(&diagnosis, &reviews)?;

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
    println!("diagnosed_identity_requirements={}", diagnosis.rows.len());
    println!("reviewed_assignments_matched={}", plan.matched.len());
    println!("reviewed_assignments_pending={}", plan.pending_rows.len());
    println!(
        "reviewed_assignments_unmatched={}",
        plan.unmatched_assignments.len()
    );

    if review_path.is_none() {
        println!("stop_reason=IdentityReviewRequired");
        print_pending(&plan);
        println!("candidate_only=true");
        println!("creates_semantic_authority=false");
        println!("applicability_promoted=false");
        println!("claim_truth_promoted=false");
        return Ok(());
    }

    let world_config = load_world_database_config(None)?;
    let world_store = WorldStore::connect(&world_config)?;
    let mut payment_sink = WorldStoreReviewedPaymentSink::new(world_store);
    let mut novel_cycles: Vec<ReviewedPreparedCycle> = Vec::new();
    let mut known_identity_payments = 0usize;

    for (index, planned) in plan.matched.iter().enumerate() {
        let request = reviewed_acquisition_request(planned)?;
        let acquired = fetch_entity_rdf_revision_receipt(&request.source_qid, request.revision_id)?;
        if acquired.source_revision_ref != request.source_revision_ref {
            return Err(std::io::Error::other(format!(
                "provider returned wrong pinned source manifestation: expected {}, got {}",
                request.source_revision_ref, acquired.source_revision_ref
            ))
            .into());
        }
        let route = matching_route(&request, &acquired)?;
        let prepared = prepare_reviewed_mabo_identity_cycle(
            &diagnosis,
            planned,
            &acquired,
            &route,
            (index as i64).saturating_mul(2),
        )?;

        if baseline
            .identity_class_refs
            .contains(&planned.assignment.identity_class_ref)
        {
            apply_known_identity_payment_transaction(
                &mut session,
                format!(
                    "frontier:mabo:world-identity:known:{}",
                    planned.assignment.representation_ref
                ),
                &planned.assignment.identity_class_ref,
                &prepared.prepared.observation,
                &prepared.reviewed_payment_wire,
                &mut payment_sink,
            )
            .map_err(|error| {
                std::io::Error::other(format!("known identity payment failed: {error:?}"))
            })?;
            known_identity_payments += 1;
        } else {
            novel_cycles.push(prepared);
        }
    }

    let queued_novel_cycles = novel_cycles.len();
    let queue = ReviewedCycleQueueSource::new(novel_cycles, payment_sink);
    let mut source = IdentityCoherentCycleSource::with_baseline(
        queue,
        identity_coherence_baseline(&baseline),
    );
    let mut lineage_sink = PgDiscoveryLineageSink::new(pg_config);
    let receipt = run_recurrent_world_expansion(
        &mut session,
        &mut source,
        &mut lineage_sink,
        WorldExpansionRunnerConfig {
            max_cycles: queued_novel_cycles.max(1),
        },
    )
    .map_err(|error| std::io::Error::other(format!("recurrent runner failed: {error:?}")))?;

    let durable_total =
        durable_mabo_campaign_total(&baseline, receipt.final_novel_identity_classes);
    println!("known_identity_payments={known_identity_payments}");
    println!("queued_novel_cycles={queued_novel_cycles}");
    println!("cycles_committed={}", receipt.cycles_committed);
    println!(
        "new_novel_identity_classes={}",
        receipt.final_novel_identity_classes
    );
    println!("durable_total_identity_classes={durable_total}");
    println!("target_identity_classes={MABO_NOVEL_IDENTITY_TARGET}");
    println!("remaining_open_residuals={:?}", receipt.remaining_open_residual_refs);
    println!("stop_reason={:?}", receipt.stop_reason);
    print_pending(&plan);
    println!("candidate_only=true");
    println!("creates_semantic_authority=false");
    println!("applicability_promoted=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
