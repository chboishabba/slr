//! Non-persistent historical revision perturbation for the mature Mabo world.
//!
//! This deliberately does *not* wait for Wikidata to change after the campaign
//! was reviewed.  It searches a bounded real revision history for an earlier
//! exact manifestation whose bounded context differs from the reviewed-current
//! manifestation, reconstructs that historical world in memory, and compares
//! consumer diagnosis against the current reviewed world.
//!
//! No review receipt or semantic row is written by this executable.
//!
//! Usage:
//!   cargo run -p sensiblaw-world-expansion-runtime --example mabo_revision_shadow -- \
//!     <seed_ref> [max_history_steps] [receipt.json]

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};

use sensiblaw_pg_source_store::{
    load_database_config, load_discovery_identity_baseline,
    load_latent_world_rows_with_context_revision_slice,
    load_reviewed_context_source_revision_coordinates, ContextRevisionWorldSlice,
    LatentWorldBudget,
};
use sensiblaw_wikimedia_candidate_provider::{
    fetch_entity_rdf_revision_receipt, fetch_previous_revision_id,
};
use sensiblaw_world_expansion_runtime::{
    adaptive_campaign::{parse_bounded_target_context, ParsedBoundedContextCandidate},
    adaptive_context_review::bounded_context_candidate_set_sha256,
    mabo_generic_legal_follow::mabo_generic_campaign_from_world,
    mabo_revision_campaign::{
        highest_reviewed_mabo_revisions, shadow_context_revision_world,
        MaboReviewedRevisionCoordinate,
    },
};
use serde_json::{json, Value};

fn parse_or<T: std::str::FromStr>(arg: Option<&String>, default: T) -> T {
    arg.and_then(|value| value.parse::<T>().ok()).unwrap_or(default)
}

fn emit_receipt(
    output: Option<&Path>,
    receipt: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("terminal_receipt_json={}", serde_json::to_string(receipt)?);
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
        println!("terminal_receipt_path={}", path.display());
    }
    Ok(())
}

fn candidate_keyset(candidates: &[ParsedBoundedContextCandidate]) -> BTreeSet<String> {
    candidates
        .iter()
        .map(|candidate| {
            format!(
                "{}|{}|{}",
                candidate.source_qid, candidate.property_ref, candidate.target_qid
            )
        })
        .collect()
}

fn residual_representation_refs(
    campaign: &sensiblaw_world_expansion_runtime::mabo_generic_legal_follow::MaboGenericLegalFollowCampaign,
) -> BTreeSet<String> {
    campaign
        .state()
        .residuals
        .iter()
        .map(|residual| residual.row.representation_ref.clone())
        .collect()
}

struct HistoricalPerturbation {
    coordinate: MaboReviewedRevisionCoordinate,
    historical_revision_id: u64,
    historical_revision_ref: String,
    current_candidates: Vec<ParsedBoundedContextCandidate>,
    historical_candidates: Vec<ParsedBoundedContextCandidate>,
    history_steps: usize,
}

fn find_historical_context_perturbation(
    reviewed: &[MaboReviewedRevisionCoordinate],
    max_history_steps: usize,
) -> Result<Option<HistoricalPerturbation>, Box<dyn std::error::Error>> {
    for coordinate in reviewed {
        let current = fetch_entity_rdf_revision_receipt(
            &coordinate.source_ref,
            coordinate.reviewed_revision_id,
        )?;
        let current_candidates = parse_bounded_target_context(&current)?;
        let current_keys = candidate_keyset(&current_candidates);

        let mut cursor = coordinate.reviewed_revision_id;
        for history_steps in 1..=max_history_steps {
            let previous = fetch_previous_revision_id(&coordinate.source_ref, cursor)?;
            let acquired =
                fetch_entity_rdf_revision_receipt(&coordinate.source_ref, previous)?;
            let historical_candidates = parse_bounded_target_context(&acquired)?;
            if candidate_keyset(&historical_candidates) != current_keys {
                return Ok(Some(HistoricalPerturbation {
                    coordinate: coordinate.clone(),
                    historical_revision_id: previous,
                    historical_revision_ref: acquired.source_revision_ref,
                    current_candidates,
                    historical_candidates,
                    history_steps,
                }));
            }
            cursor = previous;
        }
    }
    Ok(None)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return Err(
            "usage: mabo_revision_shadow <seed_ref> [max_history_steps] [receipt.json]"
                .into(),
        );
    }
    let seed_ref = &args[0];
    let max_history_steps = parse_or(args.get(1), 32_usize);
    if max_history_steps == 0 {
        return Err("max_history_steps must be positive".into());
    }
    let receipt_path = args.get(2).map(PathBuf::from);

    let config = load_database_config(None)?;
    let coordinates = load_reviewed_context_source_revision_coordinates(&config)?;
    let reviewed = highest_reviewed_mabo_revisions(&coordinates)?;
    let context_slice = ContextRevisionWorldSlice {
        wikidata_source_revisions: reviewed
            .iter()
            .map(|coordinate| {
                (
                    coordinate.source_ref.clone(),
                    coordinate.reviewed_revision_ref.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>(),
    };
    let current_world = load_latent_world_rows_with_context_revision_slice(
        &config,
        seed_ref,
        LatentWorldBudget {
            max_hops: 100,
            max_nodes: 10_000,
            max_edges: 50_000,
        },
        &context_slice,
    )?;
    let baseline = load_discovery_identity_baseline(&config)?;
    let current_campaign =
        mabo_generic_campaign_from_world(&current_world, &baseline, 1_000)?;
    let current_residuals = residual_representation_refs(&current_campaign);

    let Some(perturbation) =
        find_historical_context_perturbation(&reviewed, max_history_steps)?
    else {
        emit_receipt(
            receipt_path.as_deref(),
            &json!({
                "schema_version": "sl.mabo_revision_shadow.v0_1",
                "seed_ref": seed_ref,
                "stop": "NoHistoricalContextPerturbationWithinBound",
                "max_history_steps": max_history_steps,
                "reviewed_source_count": reviewed.len(),
                "current_identity_residual_refs": current_residuals,
                "fixture_only": true,
                "persisted": false,
                "consumer_adequate_formally_proved": false,
                "candidate_only": true,
                "creates_semantic_authority": false,
                "applicability_promoted": false,
                "claim_truth_promoted": false
            }),
        )?;
        return Ok(());
    };

    let current_digest = bounded_context_candidate_set_sha256(
        &perturbation.coordinate.reviewed_revision_ref,
        &perturbation.current_candidates,
    )?;
    let historical_digest = bounded_context_candidate_set_sha256(
        &perturbation.historical_revision_ref,
        &perturbation.historical_candidates,
    )?;

    let (historical_world, shadow_receipt) = shadow_context_revision_world(
        &current_world,
        &perturbation.coordinate.source_ref,
        &perturbation.historical_revision_ref,
        &perturbation.historical_candidates,
    )?;
    let historical_campaign =
        mabo_generic_campaign_from_world(&historical_world, &baseline, 1_000)?;
    let historical_residuals = residual_representation_refs(&historical_campaign);

    let introduced_in_current = current_residuals
        .difference(&historical_residuals)
        .cloned()
        .collect::<Vec<_>>();
    let removed_in_current = historical_residuals
        .difference(&current_residuals)
        .cloned()
        .collect::<Vec<_>>();
    let context_added_in_current = candidate_keyset(&perturbation.current_candidates)
        .difference(&candidate_keyset(&perturbation.historical_candidates))
        .cloned()
        .collect::<Vec<_>>();
    let context_removed_in_current = candidate_keyset(&perturbation.historical_candidates)
        .difference(&candidate_keyset(&perturbation.current_candidates))
        .cloned()
        .collect::<Vec<_>>();

    let consumer_frontier_changed =
        !introduced_in_current.is_empty() || !removed_in_current.is_empty();

    emit_receipt(
        receipt_path.as_deref(),
        &json!({
            "schema_version": "sl.mabo_revision_shadow.v0_1",
            "seed_ref": seed_ref,
            "stop": if consumer_frontier_changed {
                "HistoricalRevisionChangedConsumerFrontier"
            } else {
                "HistoricalRevisionChangedContextOnly"
            },
            "source_ref": perturbation.coordinate.source_ref,
            "historical_revision_id": perturbation.historical_revision_id,
            "historical_revision_ref": perturbation.historical_revision_ref,
            "reviewed_current_revision_id": perturbation.coordinate.reviewed_revision_id,
            "reviewed_current_revision_ref": perturbation.coordinate.reviewed_revision_ref,
            "history_steps": perturbation.history_steps,
            "historical_candidate_set_sha256": historical_digest,
            "current_candidate_set_sha256": current_digest,
            "context_added_in_current": context_added_in_current,
            "context_removed_in_current": context_removed_in_current,
            "historical_identity_residual_refs": historical_residuals,
            "current_identity_residual_refs": current_residuals,
            "identity_residuals_introduced_in_current": introduced_in_current,
            "identity_residuals_removed_in_current": removed_in_current,
            "consumer_frontier_changed": consumer_frontier_changed,
            "shadow_removed_current_context_edges": shadow_receipt.removed_current_context_edges,
            "shadow_added_historical_context_edges": shadow_receipt.added_shadow_context_edges,
            "historical_world_visited_count": shadow_receipt.shadow_visited_count,
            "current_world_visited_count": shadow_receipt.base_visited_count,
            "fixture_only": true,
            "persisted": false,
            "consumer_adequate_formally_proved": false,
            "candidate_only": true,
            "creates_semantic_authority": false,
            "applicability_promoted": false,
            "claim_truth_promoted": false
        }),
    )?;

    Ok(())
}
