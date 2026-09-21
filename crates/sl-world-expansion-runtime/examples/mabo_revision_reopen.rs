//! Combined S15/S16/S18 live experiment:
//! closed Mabo world -> bounded source revision perturbations -> exact context
//! re-review -> recompute identity frontier -> generic LegalFollow reviewed
//! deltas.
//!
//! Usage:
//!   cargo run -p sensiblaw-world-expansion-runtime --example mabo_revision_reopen -- \
//!     <seed_ref> <context_review.tsv> <identity_review.tsv> //!     [max_sources] [pending_dir] [receipt.json]
//!
//! Both review manifests may be empty files. Missing review is an explicit stop
//! and a pending review bundle is written; no decision is fabricated. The
//! review manifest never determines scheduling order.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use sensiblaw_pg_source_store::{
    load_database_config, load_discovery_identity_baseline,
    load_latent_world_rows_with_context_revision_slice,
    load_reviewed_context_source_revision_coordinates,
    materialize_reviewed_context_expansion, ContextRevisionWorldSlice,
    LatentWorldBudget,
};
use sensiblaw_wikimedia_candidate_provider::fetch_entity_rdf_revision_receipt;
use sensiblaw_world_expansion_runtime::{
    adaptive_campaign::parse_bounded_target_context,
    adaptive_context_review::{
        bounded_context_candidate_set_sha256, matching_context_review,
        parse_mabo_context_review_tsv, pending_context_review_bundle,
        prepare_reviewed_context_expansion,
    },
    mabo_generic_legal_follow::{
        apply_reviewed_mabo_sequence, mabo_generic_campaign_from_world,
        mabo_generic_campaign_receipt, pending_mabo_identity_review_bundle,
    },
    mabo_revision_campaign::{
        highest_reviewed_mabo_revisions, probe_latest_mabo_revision_changes,
    },
    parse_mabo_identity_review_tsv,
};
use serde_json::{json, Value};

fn parse_or<T: std::str::FromStr>(arg: Option<&String>, default: T) -> T {
    arg.and_then(|value| value.parse::<T>().ok()).unwrap_or(default)
}

fn emit_terminal_receipt(
    output: Option<&Path>,
    receipt: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let compact = serde_json::to_string(receipt)?;
    println!("terminal_receipt_json={compact}");
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        return Err(
            "usage: mabo_revision_reopen <seed_ref> <context_review.tsv> <identity_review.tsv> [max_sources] [pending_dir] [receipt.json]"
                .into(),
        );
    }

    let seed_ref = &args[0];
    let context_review_path = PathBuf::from(&args[1]);
    let identity_review_path = PathBuf::from(&args[2]);
    let max_sources = parse_or(args.get(3), 32_usize);
    let pending_dir = args
        .get(4)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("artifacts/mabo/revision-reviews/pending"));
    let receipt_path = args.get(5).map(PathBuf::from);

    let config = load_database_config(None)?;
    let reviewed_rows = load_reviewed_context_source_revision_coordinates(&config)?;
    println!("seed_ref={seed_ref}");
    println!("reviewed_context_revision_coordinates={}", reviewed_rows.len());
    println!("revision_probe_max_sources={max_sources}");

    let probe = probe_latest_mabo_revision_changes(&reviewed_rows, max_sources)?;
    println!("reviewed_source_count={}", probe.reviewed_source_count);
    println!("probed_source_count={}", probe.probed_source_count);
    println!("revision_reopen_count={}", probe.reopen_residuals.len());
    println!("unchanged_source_count={}", probe.unchanged_source_refs.len());
    println!("unprobed_source_count={}", probe.unprobed_source_refs.len());
    println!("probe_truncated={}", probe.probe_truncated);
    println!("probe_complete={}", probe.probe_complete);
    println!("revision_probe_blockers={}", probe.blockers.len());
    for blocker in &probe.blockers {
        println!(
            "revision_probe_blocker={} retryable={} http_status={:?} detail={}",
            blocker.source_ref,
            blocker.retryable,
            blocker.http_status_code,
            blocker.detail
        );
    }
    println!("candidate_only={}", probe.candidate_only);
    println!("creates_semantic_authority={}", probe.creates_semantic_authority);
    println!("applicability_promoted={}", probe.applicability_promoted);
    println!("claim_truth_promoted={}", probe.claim_truth_promoted);

    let blocker_values = probe
        .blockers
        .iter()
        .map(|blocker| {
            json!({
                "source_ref": blocker.source_ref,
                "detail": blocker.detail,
                "retryable": blocker.retryable,
                "http_status_code": blocker.http_status_code,
                "candidate_only": blocker.candidate_only,
                "creates_semantic_authority": blocker.creates_semantic_authority,
                "claim_truth_promoted": blocker.claim_truth_promoted
            })
        })
        .collect::<Vec<_>>();

    if probe.reopen_residuals.is_empty() {
        let (stop, closure_reason, current_frontier_closed) =
            if !probe.blockers.is_empty() {
                (
                    "RevisionProbeBlocked",
                    "RevisionLookupBlockersRemain",
                    false,
                )
            } else if probe.probe_truncated {
                (
                    "RevisionProbeBudgetExhausted",
                    "UnprobedReviewedSourcesRemain",
                    false,
                )
            } else {
                (
                    "CurrentFrontierClosedWithoutAdequacy",
                    "NoRevisionPerturbation",
                    true,
                )
            };
        println!("stop={stop}");
        println!("closure_reason={closure_reason}");
        println!("consumer_adequate_inferred=false");
        emit_terminal_receipt(
            receipt_path.as_deref(),
            &json!({
                "schema_version": "sl.mabo_revision_reopen.v0_2",
                "seed_ref": seed_ref,
                "stop": stop,
                "closure_reason": closure_reason,
                "current_frontier_closed": current_frontier_closed,
                "consumer_adequate_formally_proved": false,
                "reviewed_source_count": probe.reviewed_source_count,
                "probed_source_count": probe.probed_source_count,
                "probe_truncated": probe.probe_truncated,
                "probe_complete": probe.probe_complete,
                "unprobed_source_refs": probe.unprobed_source_refs,
                "revision_probe_blockers": blocker_values,
                "revision_reopen_count": 0,
                "reviewed_context_revisions_paid": 0,
                "reviewed_identity_deltas_applied": 0,
                "candidate_only": true,
                "creates_semantic_authority": false,
                "applicability_promoted": false,
                "claim_truth_promoted": false
            }),
        )?;
        return Ok(());
    }

    let context_review_text = fs::read_to_string(&context_review_path)?;
    let context_reviews = parse_mabo_context_review_tsv(&context_review_text)?;
    let mut reviewed_context_revisions_paid = 0usize;
    let mut processed_revision_deltas = Vec::<Value>::new();

    // Deterministic source order comes from the revision probe. Review-manifest
    // presence is never used to reorder or select work.
    for reopened in &probe.reopen_residuals {
        println!("selected_revision_residual={}", reopened.residual_ref);
        println!("selected_source_ref={}", reopened.source_ref);
        println!("reviewed_revision_ref={}", reopened.reviewed_revision_ref);
        println!("latest_revision_ref={}", reopened.latest_revision_ref);

        // Reacquire the exact revision discovered by the bounded coordinate
        // probe. We deliberately do not call a second "latest content" operation.
        let acquired =
            fetch_entity_rdf_revision_receipt(&reopened.source_ref, reopened.latest_revision_id)?;
        if acquired.source_revision_ref != reopened.latest_revision_ref
            || acquired.qid != reopened.source_ref
            || !acquired.candidate_only
            || acquired.semantic_promotion
        {
            return Err(std::io::Error::other(
                "revision re-acquisition did not return exact candidate-only source coordinate",
            )
            .into());
        }

        let candidates = parse_bounded_target_context(&acquired)?;
        let digest =
            bounded_context_candidate_set_sha256(&acquired.source_revision_ref, &candidates)?;
        println!("latest_bounded_candidate_count={}", candidates.len());
        println!("latest_bounded_candidate_set_sha256={digest}");

        let Some(context_review) =
            matching_context_review(&acquired.source_revision_ref, &digest, &context_reviews)
        else {
            fs::create_dir_all(&pending_dir)?;
            let pending = pending_context_review_bundle(
                &acquired.qid,
                &acquired.source_revision_ref,
                &candidates,
            )?;
            let pending_path = pending_dir.join(format!(
                "{}__{}.pending.tsv",
                acquired.qid, acquired.revision_id
            ));
            fs::write(&pending_path, pending)?;
            println!("stop=ContextReviewRequired");
            println!("pending_context_review={}", pending_path.display());
            println!("consumer_adequate_inferred=false");
            emit_terminal_receipt(
                receipt_path.as_deref(),
                &json!({
                    "schema_version": "sl.mabo_revision_reopen.v0_2",
                    "seed_ref": seed_ref,
                    "stop": "ContextReviewRequired",
                    "current_frontier_closed": false,
                    "consumer_adequate_formally_proved": false,
                    "selected_revision_residual": reopened.residual_ref,
                    "source_ref": reopened.source_ref,
                    "reviewed_revision_ref": reopened.reviewed_revision_ref,
                    "latest_revision_ref": reopened.latest_revision_ref,
                    "candidate_set_sha256": digest,
                    "pending_context_review": pending_path,
                    "revision_reopen_count": probe.reopen_residuals.len(),
                    "reviewed_context_revisions_paid": reviewed_context_revisions_paid,
                    "processed_revision_deltas": processed_revision_deltas,
                    "reviewed_identity_deltas_applied": 0,
                    "probe_truncated": probe.probe_truncated,
                    "probe_complete": probe.probe_complete,
                    "unprobed_source_refs": probe.unprobed_source_refs,
                    "revision_probe_blockers": blocker_values,
                    "candidate_only": true,
                    "creates_semantic_authority": false,
                    "applicability_promoted": false,
                    "claim_truth_promoted": false
                }),
            )?;
            return Ok(());
        };

        let prepared = prepare_reviewed_context_expansion(&candidates, context_review)?;
        let materialized =
            materialize_reviewed_context_expansion(&config, &prepared.edges, &prepared.expansion)?;
        println!("context_review_ref={}", context_review.review_ref);
        println!(
            "reviewed_context_materialized_count={}",
            materialized.materialized_count
        );
        println!("context_revision_reopened_and_paid=true");

        reviewed_context_revisions_paid += 1;
        processed_revision_deltas.push(json!({
            "residual_ref": reopened.residual_ref,
            "source_ref": reopened.source_ref,
            "reviewed_revision_ref": reopened.reviewed_revision_ref,
            "latest_revision_ref": reopened.latest_revision_ref,
            "candidate_set_sha256": digest,
            "review_ref": context_review.review_ref,
            "bounded_candidate_count": candidates.len(),
            "materialized_count": materialized.materialized_count
        }));
    }

    // Recompute once from durable state after every payable reviewed revision
    // delta has been committed.  The world slice selects the highest reviewed
    // Wikidata manifestation per source; historical context remains durable
    // provenance but cannot contaminate the R1 traversal.
    let current_reviewed_coordinates =
        load_reviewed_context_source_revision_coordinates(&config)?;
    let highest_reviewed =
        highest_reviewed_mabo_revisions(&current_reviewed_coordinates)?;
    let context_slice = ContextRevisionWorldSlice {
        wikidata_source_revisions: highest_reviewed
            .iter()
            .map(|coordinate| {
                (
                    coordinate.source_ref.clone(),
                    coordinate.reviewed_revision_ref.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>(),
    };
    println!(
        "world_context_revision_slice_sources={}",
        context_slice.wikidata_source_revisions.len()
    );
    let world = load_latent_world_rows_with_context_revision_slice(
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
    let mut campaign = mabo_generic_campaign_from_world(&world, &baseline, 1_000)?;
    println!(
        "post_revision_identity_residuals={}",
        campaign.state().residuals.len()
    );

    let identity_review_text = fs::read_to_string(&identity_review_path)?;
    let identity_reviews = parse_mabo_identity_review_tsv(&identity_review_text)?;
    let reviewed_deltas_applied =
        apply_reviewed_mabo_sequence(&mut campaign, &identity_reviews)?;
    let receipt = mabo_generic_campaign_receipt(&campaign);
    println!("reviewed_identity_deltas_applied={reviewed_deltas_applied}");
    println!("residuals_remaining={}", receipt.residuals_remaining);
    println!("reviewed_identity_count={}", receipt.reviewed_identity_count);
    println!("authority_promoted={}", receipt.creates_semantic_authority);
    println!("applicability_promoted={}", receipt.applicability_promoted);
    println!("claim_truth_promoted={}", receipt.claim_truth_promoted);

    let (
        stop,
        current_frontier_closed,
        next_representation_ref,
        next_residual_ref,
        pending_identity_review,
    ) = match campaign.next_demand() {
            Ok(next) => {
                println!("stop=IdentityReviewRequired");
                println!("next_representation_ref={}", next.representation_ref);
                println!("next_residual_ref={}", next.residual_ref);
                println!(
                    "identity_review_manifest_template={}\tworld-object:<reviewed-id>\treview:<operator-ref>",
                    next.representation_ref
                );

                fs::create_dir_all(&pending_dir)?;
                let bundle = pending_mabo_identity_review_bundle(&next)?;
                let safe_ref = next.representation_ref.replace(':', "__");
                let pending_path =
                    pending_dir.join(format!("identity__{safe_ref}.pending.tsv"));
                fs::write(&pending_path, bundle)?;
                println!("pending_identity_review={}", pending_path.display());

                (
                    "IdentityReviewRequired",
                    false,
                    Some(next.representation_ref),
                    Some(next.residual_ref),
                    Some(pending_path.display().to_string()),
                )
            }
            Err(sensiblaw_legal_runtime::GenericCampaignStop::NoFreshDemand) => {
                if !probe.blockers.is_empty() {
                    println!("stop=RevisionProbeBlocked");
                    ("RevisionProbeBlocked", false, None, None, None)
                } else if probe.probe_truncated {
                    println!("stop=RevisionProbeBudgetExhausted");
                    ("RevisionProbeBudgetExhausted", false, None, None, None)
                } else {
                    println!("stop=CurrentFrontierClosedWithoutAdequacy");
                    (
                        "CurrentFrontierClosedWithoutAdequacy",
                        true,
                        None,
                        None,
                        None,
                    )
                }
            }
            Err(sensiblaw_legal_runtime::GenericCampaignStop::BudgetExhausted) => {
                println!("stop=BudgetExhaustedWithoutAdequacy");
                ("BudgetExhaustedWithoutAdequacy", false, None, None, None)
            }
        };
    println!("consumer_adequate_inferred=false");
    emit_terminal_receipt(
        receipt_path.as_deref(),
        &json!({
            "schema_version": "sl.mabo_revision_reopen.v0_2",
            "seed_ref": seed_ref,
            "stop": stop,
            "current_frontier_closed": current_frontier_closed,
            "consumer_adequate_formally_proved": false,
            "revision_reopen_count": probe.reopen_residuals.len(),
            "reviewed_context_revisions_paid": reviewed_context_revisions_paid,
            "processed_revision_deltas": processed_revision_deltas,
            "world_context_revision_slice": context_slice.wikidata_source_revisions,
            "reviewed_identity_deltas_applied": reviewed_deltas_applied,
            "residuals_remaining": receipt.residuals_remaining,
            "next_representation_ref": next_representation_ref,
            "next_residual_ref": next_residual_ref,
            "pending_identity_review": pending_identity_review,
            "probe_truncated": probe.probe_truncated,
            "probe_complete": probe.probe_complete,
            "unprobed_source_refs": probe.unprobed_source_refs,
            "revision_probe_blockers": blocker_values,
            "candidate_only": true,
            "creates_semantic_authority": false,
            "applicability_promoted": false,
            "claim_truth_promoted": false
        }),
    )?;

    Ok(())
}