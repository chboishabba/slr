use std::fs;
use std::io::Cursor;

use sensiblaw_pg_source_store::{
    load_database_config, load_discovery_identity_baseline,
    load_latent_world_rows_with_budget, load_reviewed_context_expansion_sources,
    materialize_non_novel_identity_aliases, materialize_reviewed_context_expansion,
    LatentWorldBudget, NonNovelIdentityAliasInput,
};
use sensiblaw_proof_search_loop::world_expansion_runner::{
    run_recurrent_world_expansion, RecurrentRunStopReason, WorldExpansionRunnerConfig,
};
use sensiblaw_proof_search_loop::world_expansion_session::WorldExpansionSession;
use sensiblaw_proof_search_loop::world_identity_guard::IdentityCoherentCycleSource;
use sensiblaw_route_selector::{decode_route_candidate, RouteCandidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, fetch_entity_rdf_revision_receipt,
    fetch_latest_entity_rdf_revision_receipt,
};
use sensiblaw_world_expansion_runtime::adaptive_campaign::{
    parse_bounded_target_context, select_next_mabo_adaptive_decision, MaboAdaptiveDecision,
};
use sensiblaw_world_expansion_runtime::adaptive_context_review::{
    bounded_context_candidate_set_sha256, matching_context_review,
    parse_mabo_context_review_tsv, prepare_reviewed_context_expansion,
};
use sensiblaw_world_expansion_runtime::reviewed_campaign::{
    self, prepare_reviewed_mabo_identity_cycle, reviewed_acquisition_request,
};
use sensiblaw_world_expansion_runtime::{
    apply_known_identity_payment_transaction, diagnose_mabo_context_world_identity,
    identity_coherence_baseline, mabo_consumer_diagnosis_frontier,
    mabo_remaining_world_expansion_policy, parse_mabo_identity_review_tsv,
    PgDiscoveryLineageSink, ReviewedCycleQueueSource, WorldStoreReviewedPaymentSink,
    MaboPlannedIdentityReview, MABO_NOVEL_IDENTITY_TARGET,
};
use sensiblaw_world_store::{load_database_config as load_world_database_config, WorldStore};

const MABO_QID: &str = "Q1501525";
const CAMPAIGN_MAX_CYCLES: usize = 100;
const WORLD_VIEW_MAX_HOPS: u32 = 100;
const WORLD_VIEW_MAX_NODES: usize = 10_000;
const WORLD_VIEW_MAX_EDGES: usize = 50_000;

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

fn is_qid(value: &str) -> bool {
    value
        .strip_prefix('Q')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

fn print_identity_review_required(row: &sensiblaw_world_expansion_runtime::MaboIdentityDiagnosisRow) {
    println!("stop_reason=IdentityReviewRequired");
    println!("selected_residual={}", row.residual_ref);
    println!("selected_representation={}", row.representation_ref);
    println!("selected_relation_types={:?}", row.relation_type_refs);
    println!("selected_source_revisions={:?}", row.source_revision_refs);
    println!(
        "identity_review_manifest_template={}\tworld-object:<reviewed-id>\treview:<operator-ref>",
        row.representation_ref
    );
}

fn print_context_review_required(
    source_qid: &str,
    source_revision_ref: &str,
    digest: &str,
    candidates: &[sensiblaw_world_expansion_runtime::adaptive_campaign::ParsedBoundedContextCandidate],
) {
    println!("stop_reason=ContextReviewRequired");
    println!("context_source_qid={source_qid}");
    println!("context_source_revision={source_revision_ref}");
    println!("bounded_context_candidate_count={}", candidates.len());
    println!("bounded_context_candidate_set_sha256={digest}");
    for candidate in candidates {
        println!(
            "bounded_context_candidate={}\t{}\t{}\t{}",
            candidate.candidate_id,
            candidate.source_qid,
            candidate.property_ref,
            candidate.target_qid
        );
    }
    println!(
        "context_review_manifest_template={source_revision_ref}\t{digest}\treview:<operator-ref>"
    );
}

fn complete_context_expansion(
    pg_config: &sensiblaw_pg_source_store::DatabaseConfig,
    source_qid: &str,
    context_reviews: &[sensiblaw_world_expansion_runtime::adaptive_context_review::ContextExpansionReviewAssignment],
) -> Result<bool, Box<dyn std::error::Error>> {
    let acquired = fetch_latest_entity_rdf_revision_receipt(source_qid)?;
    if acquired.qid != source_qid || !acquired.candidate_only || acquired.semantic_promotion {
        return Err(std::io::Error::other("target source acquisition violated candidate-only exact-QID contract").into());
    }
    let candidates = parse_bounded_target_context(&acquired)?;
    let digest = bounded_context_candidate_set_sha256(&acquired.source_revision_ref, &candidates)?;
    println!("context_source_qid={source_qid}");
    println!("context_source_revision={}", acquired.source_revision_ref);
    println!("bounded_context_candidate_count={}", candidates.len());
    println!("bounded_context_candidate_set_sha256={digest}");

    let Some(review) = matching_context_review(
        &acquired.source_revision_ref,
        &digest,
        context_reviews,
    ) else {
        print_context_review_required(
            source_qid,
            &acquired.source_revision_ref,
            &digest,
            &candidates,
        );
        return Ok(false);
    };

    let prepared = prepare_reviewed_context_expansion(&candidates, review)?;
    let receipt = materialize_reviewed_context_expansion(
        pg_config,
        &prepared.edges,
        &prepared.expansion,
    )?;
    println!("context_review_ref={}", review.review_ref);
    println!("context_edges_reviewed={}", prepared.edges.len());
    println!(
        "context_expansion_receipt_materialized={}",
        receipt.materialized_count
    );
    Ok(true)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let identity_review_text = args
        .get(1)
        .map(fs::read_to_string)
        .transpose()?
        .unwrap_or_default();
    let context_review_text = args
        .get(2)
        .map(fs::read_to_string)
        .transpose()?
        .unwrap_or_default();
    let identity_reviews = parse_mabo_identity_review_tsv(&identity_review_text)?;
    let context_reviews = parse_mabo_context_review_tsv(&context_review_text)?;

    let pg_config = load_database_config(None)?;
    let world_config = load_world_database_config(None)?;
    let mut adaptive_cycles_completed = 0usize;
    let mut identity_admissions_committed = 0usize;
    let mut known_identity_alias_payments = 0usize;

    loop {
        let baseline = load_discovery_identity_baseline(&pg_config)?;
        if baseline.identity_class_refs.len() >= MABO_NOVEL_IDENTITY_TARGET {
            println!("stop_reason=TargetComplete");
            println!("adaptive_cycles_completed={adaptive_cycles_completed}");
            println!("durable_total_identity_classes={}", baseline.identity_class_refs.len());
            break;
        }
        if adaptive_cycles_completed >= CAMPAIGN_MAX_CYCLES {
            println!("stop_reason=CycleBudgetExhausted");
            println!("adaptive_cycles_completed={adaptive_cycles_completed}");
            println!("campaign_cycle_budget={CAMPAIGN_MAX_CYCLES}");
            break;
        }

        let world = load_latent_world_rows_with_budget(
            &pg_config,
            MABO_QID,
            LatentWorldBudget {
                max_hops: WORLD_VIEW_MAX_HOPS,
                max_nodes: WORLD_VIEW_MAX_NODES,
                max_edges: WORLD_VIEW_MAX_EDGES,
            },
        )?;
        if world.creates_semantic_authority
            || world.applicability_promoted
            || world.claim_truth_promoted
        {
            return Err(std::io::Error::other(
                "latent-world inspection unexpectedly promoted semantic state",
            )
            .into());
        }
        let expanded_sources = load_reviewed_context_expansion_sources(&pg_config)?;
        let diagnosis = diagnose_mabo_context_world_identity(&world, &baseline);

        println!("campaign_cycle_index={adaptive_cycles_completed}");
        println!("world_view_requested_max_hops={}", world.requested_max_hops);
        println!("world_view_deepest_observed_hop={}", world.deepest_observed_hop);
        println!("world_view_visited_refs={}", world.visited_refs.len());
        println!("world_view_edges={}", world.edges.len());
        println!("durable_identity_classes={}", baseline.identity_class_refs.len());
        println!("diagnosed_identity_requirements={}", diagnosis.rows.len());
        println!("reviewed_expanded_sources={}", expanded_sources.len());

        let Some(decision) = select_next_mabo_adaptive_decision(
            &diagnosis,
            &baseline,
            &world,
            &expanded_sources,
            format!("frontier:mabo:adaptive:{adaptive_cycles_completed}"),
        ) else {
            println!("stop_reason=FrontierExhausted");
            println!("adaptive_cycles_completed={adaptive_cycles_completed}");
            break;
        };

        match decision {
            MaboAdaptiveDecision::ContextExpansion(selection) => {
                println!("selected_kind=context-expansion");
                println!("selected_residual={}", selection.residual_ref);
                println!("selected_move={}", selection.move_ref);
                println!("selected_source_qid={}", selection.source_qid);
                println!("selected_shared_dependency_gain={}", selection.shared_dependency_gain);
                if !complete_context_expansion(
                    &pg_config,
                    &selection.source_qid,
                    &context_reviews,
                )? {
                    break;
                }
                adaptive_cycles_completed += 1;
                println!("cycle_commit=context-expansion");
            }
            MaboAdaptiveDecision::Identity(selection) => {
                println!("selected_kind=identity");
                println!("selected_residual={}", selection.residual_ref);
                println!("selected_move={}", selection.move_ref);
                println!("selected_representation={}", selection.representation_ref);
                println!("selected_shared_dependency_gain={}", selection.shared_dependency_gain);

                let row = diagnosis
                    .rows
                    .iter()
                    .find(|row| row.residual_ref == selection.residual_ref)
                    .ok_or_else(|| std::io::Error::other("selected identity residual missing from current diagnosis"))?;
                let Some(assignment) = identity_reviews
                    .iter()
                    .find(|assignment| assignment.representation_ref == row.representation_ref)
                    .cloned()
                else {
                    print_identity_review_required(row);
                    break;
                };
                let planned = MaboPlannedIdentityReview {
                    row: row.clone(),
                    assignment: assignment.clone(),
                };
                let request = reviewed_acquisition_request(&planned)?;
                let acquired = fetch_entity_rdf_revision_receipt(
                    &request.source_qid,
                    request.revision_id,
                )?;
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
                    &planned,
                    &acquired,
                    &route,
                    i64::try_from(adaptive_cycles_completed).unwrap_or(i64::MAX).saturating_mul(2),
                )?;
                let identity_frontier = mabo_consumer_diagnosis_frontier(
                    &diagnosis,
                    format!("frontier:mabo:identity:{adaptive_cycles_completed}"),
                );
                let mut session = WorldExpansionSession::new(
                    identity_frontier,
                    mabo_remaining_world_expansion_policy(&baseline),
                );
                let world_store = WorldStore::connect(&world_config)?;
                let payment_sink = WorldStoreReviewedPaymentSink::new(world_store);

                if baseline.identity_class_refs.contains(&assignment.identity_class_ref) {
                    let mut payment_sink = payment_sink;
                    apply_known_identity_payment_transaction(
                        &mut session,
                        format!("frontier:mabo:identity-alias:{}", assignment.representation_ref),
                        &assignment.identity_class_ref,
                        &prepared.prepared.observation,
                        &prepared.reviewed_payment_wire,
                        &mut payment_sink,
                    )
                    .map_err(|error| {
                        std::io::Error::other(format!("known identity payment failed: {error:?}"))
                    })?;
                    materialize_non_novel_identity_aliases(
                        &pg_config,
                        &[NonNovelIdentityAliasInput {
                            representation_ref: assignment.representation_ref.clone(),
                            identity_class_ref: assignment.identity_class_ref.clone(),
                            review_ref: assignment.review_ref.clone(),
                            source_revision_ref: acquired.source_revision_ref.clone(),
                            candidate_only: true,
                            creates_semantic_authority: false,
                            applicability_promoted: false,
                            claim_truth_promoted: false,
                        }],
                    )?;
                    known_identity_alias_payments += 1;
                    println!("identity_commit=known-non-novel-alias");
                } else {
                    let queue = ReviewedCycleQueueSource::new(vec![prepared], payment_sink);
                    let mut source = IdentityCoherentCycleSource::with_baseline(
                        queue,
                        identity_coherence_baseline(&baseline),
                    );
                    let mut lineage_sink = PgDiscoveryLineageSink::new(pg_config.clone());
                    let receipt = run_recurrent_world_expansion(
                        &mut session,
                        &mut source,
                        &mut lineage_sink,
                        WorldExpansionRunnerConfig { max_cycles: 1 },
                    )
                    .map_err(|error| {
                        std::io::Error::other(format!("single adaptive identity cycle failed: {error:?}"))
                    })?;
                    if receipt.cycles_committed != 1 {
                        return Err(std::io::Error::other(format!(
                            "adaptive identity runner did not commit exactly one cycle: {:?}",
                            receipt.stop_reason
                        ))
                        .into());
                    }
                    if !matches!(
                        receipt.stop_reason,
                        RecurrentRunStopReason::CycleBudgetExhausted
                            | RecurrentRunStopReason::TargetComplete
                            | RecurrentRunStopReason::FrontierExhausted
                    ) {
                        return Err(std::io::Error::other(format!(
                            "adaptive identity runner stopped unexpectedly: {:?}",
                            receipt.stop_reason
                        ))
                        .into());
                    }
                    identity_admissions_committed += 1;
                    println!("identity_commit=novel-reviewed-lineage");
                }

                // A QID identity transition is not considered a complete
                // adaptive acquisition/re-entry cycle until its own exact
                // source manifestation has been parsed and context-reviewed.
                if is_qid(&selection.representation_ref) {
                    if !complete_context_expansion(
                        &pg_config,
                        &selection.representation_ref,
                        &context_reviews,
                    )? {
                        println!("partial_transition=identity-committed-context-pending");
                        break;
                    }
                }
                adaptive_cycles_completed += 1;
                println!("cycle_commit=identity-plus-reentry");
            }
        }
    }

    let final_baseline = load_discovery_identity_baseline(&pg_config)?;
    println!("adaptive_cycles_completed={adaptive_cycles_completed}");
    println!("identity_admissions_committed={identity_admissions_committed}");
    println!("known_identity_alias_payments={known_identity_alias_payments}");
    println!("campaign_cycle_budget={CAMPAIGN_MAX_CYCLES}");
    println!(
        "durable_total_identity_classes={}",
        final_baseline.identity_class_refs.len()
    );
    println!("target_identity_classes={MABO_NOVEL_IDENTITY_TARGET}");
    println!("candidate_only=true");
    println!("creates_semantic_authority=false");
    println!("applicability_promoted=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
