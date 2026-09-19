use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use sensiblaw_pg_source_store::{
    load_database_config, load_gwb_hops, load_gwb_reviewed_move_refs, load_latest_gwb_hop,
    load_open_gwb_ambiguity_residuals, materialize_gwb_ambiguity_residuals,
    materialize_gwb_campaign_commit, GwbCampaignCommitInput, GwbHopLedgerInput,
};
use sensiblaw_route_executor::acquire_wikipedia_article;
use sensiblaw_route_selector::{decode_route_candidate, RouteCandidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, emit_multilingual_wikipedia_candidates_from_rdf,
    fetch_latest_entity_rdf_revision_receipt,
};
use sensiblaw_world_expansion_runtime::gwb_ambiguity_campaign::{
    ambiguity_residuals_from_rows, compile_gwb_question_frontier,
    gwb_ambiguity_world_sha256, project_open_gwb_state_after_review,
    residuals_opened_by_reviewed_routes, seed_gwb_qid_ambiguities,
    select_gwb_investigation, GwbAmbiguityKind, GwbAmbiguityResidual,
    GwbInvestigationCandidate, GwbInvestigationKind, GWB_ADAPTIVE_CAMPAIGN_REF,
    GWB_ADAPTIVE_HOP_TARGET,
};
use sensiblaw_world_expansion_runtime::gwb_analysis::{
    analyze_gwb_hops, render_gwb_analysis_receipt,
};
use sensiblaw_world_expansion_runtime::gwb_review::{
    gwb_frontier_sha256, parse_gwb_review_tsv, pending_gwb_review_bundle,
    prepare_reviewed_gwb_hop, GwbResidualEffect, GwbReviewAssignment,
};
use sensiblaw_world_expansion_runtime::gwb_supervised_type_closure::{
    acquire_supervised_type_closure, render_type_closure_review_evidence,
    type_closure_route_candidates, TypeClosureQuestion, TypeClosureRequest,
};

const GWB_REVIEWED_ROOT_QIDS: &[&str] = &[
    "Q207",      // George W. Bush
    "Q2743830",  // Bush family
    "Q23505",    // George H. W. Bush
    "Q942966",   // Decision Points
    "Q16156115", // Family of Secrets, runtime-resolved work identity
];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_candidates(bytes: Vec<u8>) -> Result<Vec<RouteCandidate>, Box<dyn std::error::Error>> {
    let mut cursor = Cursor::new(bytes);
    let mut rows = Vec::new();
    while let Some(row) = decode_route_candidate(&mut cursor)? {
        rows.push(row);
    }
    rows.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    rows.dedup_by(|left, right| left.candidate_id == right.candidate_id);
    Ok(rows)
}

fn gwb_peer_wikipedia_surface(url: &str) -> bool {
    ["en", "es", "fr", "de", "simple"].iter().any(|language| {
        url.starts_with(&format!("https://{language}.wikipedia.org/"))
    })
}

fn selected_route_observations(
    selected: &GwbInvestigationCandidate,
    rdf_bytes: &[u8],
    root_qid: &str,
) -> Result<Vec<RouteCandidate>, Box<dyn std::error::Error>> {
    let mut encoded = Vec::new();
    emit_candidates_from_rdf(root_qid, Cursor::new(rdf_bytes), &mut encoded)?;
    emit_multilingual_wikipedia_candidates_from_rdf(
        root_qid,
        Cursor::new(rdf_bytes),
        &mut encoded,
    )?;
    let all = decode_candidates(encoded)?;
    let rows = all
        .into_iter()
        .filter(|route| match selected.investigation_kind {
            GwbInvestigationKind::TypeClass | GwbInvestigationKind::Superclass => {
                route.route_family == RouteFamily::WikidataProperty
                    && matches!(route.property_ref.as_str(), "P31" | "P279")
            }
            GwbInvestigationKind::Property => {
                route.route_family == RouteFamily::WikidataProperty
                    && !matches!(route.property_ref.as_str(), "P31" | "P279")
            }
            GwbInvestigationKind::Identity | GwbInvestigationKind::SourceWorkIdentity => {
                route.route_family == RouteFamily::WikidataProperty
                    && route.property_ref == "P31"
            }
            GwbInvestigationKind::CrossLanguageSurface => {
                selected.target_ref.is_none()
                    && route.route_family == RouteFamily::WikipediaArticle
                    && gwb_peer_wikipedia_surface(&route.target_ref)
            }
            _ => false,
        })
        .collect();
    Ok(rows)
}

fn type_closure_question_for_selection(
    selected: &GwbInvestigationCandidate,
    residuals: &[GwbAmbiguityResidual],
) -> Option<TypeClosureQuestion> {
    for residual_ref in &selected.target_residual_refs {
        let residual = residuals
            .iter()
            .find(|residual| &residual.residual_ref == residual_ref)?;
        match residual.kind {
            GwbAmbiguityKind::Superclass | GwbAmbiguityKind::Subclass => {
                return Some(TypeClosureQuestion::Superclass);
            }
            GwbAmbiguityKind::TypeClass
            | GwbAmbiguityKind::PropertySupport
            | GwbAmbiguityKind::CompetingAlternatives
            | GwbAmbiguityKind::ConsumerSemanticGap => {
                return Some(TypeClosureQuestion::TypeClass);
            }
            _ => {}
        }
    }
    None
}

fn append_observations(mut bundle: String, rows: &[RouteCandidate]) -> String {
    for row in rows {
        bundle.push_str(&format!(
            "# observed_route\t{}\t{:?}\t{:?}\t{}\t{}\t{}\n",
            row.candidate_id,
            row.producer,
            row.route_family,
            row.source_ref,
            row.property_ref,
            row.target_ref
        ));
    }
    bundle
}

fn pending_path(hop_index: usize) -> PathBuf {
    PathBuf::from(format!(
        "artifacts/gwb/reviews/pending/hop-{hop_index:03}.pending.tsv"
    ))
}

fn matching_review<'a>(
    hop_index: usize,
    selected_move_ref: &str,
    reviews: &'a [GwbReviewAssignment],
) -> Option<&'a GwbReviewAssignment> {
    reviews.iter().find(|review| {
        review.hop_index == hop_index && review.selected_move_ref == selected_move_ref
    })
}

fn ensure_initial_state(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let open = load_open_gwb_ambiguity_residuals(config, GWB_ADAPTIVE_CAMPAIGN_REF)?;
    let latest = load_latest_gwb_hop(config, GWB_ADAPTIVE_CAMPAIGN_REF)?;
    if !open.is_empty() || latest.is_some() {
        return Ok(());
    }
    let seeds = GWB_REVIEWED_ROOT_QIDS
        .iter()
        .flat_map(|qid| seed_gwb_qid_ambiguities(qid))
        .collect::<Vec<_>>();
    let receipt = materialize_gwb_ambiguity_residuals(config, &seeds)?;
    println!("gwb_seed_residuals_inserted={}", receipt.inserted_count);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let review_text = match args.get(1) {
        Some(path) => fs::read_to_string(path)?,
        None => String::new(),
    };
    // Reviews are parsed up front for syntax only. They are not consulted until
    // after the current-world frontier has selected exactly one investigation.
    let reviews = parse_gwb_review_tsv(&review_text)?;

    let pg_config = load_database_config(None)?;
    ensure_initial_state(&pg_config)?;

    loop {
        let latest = load_latest_gwb_hop(&pg_config, GWB_ADAPTIVE_CAMPAIGN_REF)?;
        let hop_index = latest
            .as_ref()
            .map(|row| row.hop_index.saturating_add(1))
            .unwrap_or(0);
        let prior_receipt_sha256 = latest.as_ref().map(|row| row.receipt_sha256.clone());

        if hop_index >= GWB_ADAPTIVE_HOP_TARGET {
            println!("stop_reason=TargetComplete");
            println!("gwb_committed_reviewed_hops={hop_index}");
            break;
        }

        let open_rows =
            load_open_gwb_ambiguity_residuals(&pg_config, GWB_ADAPTIVE_CAMPAIGN_REF)?;
        if open_rows.is_empty() {
            println!("stop_reason=FrontierExhausted");
            println!("gwb_committed_reviewed_hops={hop_index}");
            break;
        }
        let residuals = ambiguity_residuals_from_rows(&open_rows)?;
        let reviewed_moves =
            load_gwb_reviewed_move_refs(&pg_config, GWB_ADAPTIVE_CAMPAIGN_REF)?;
        let compiled = compile_gwb_question_frontier(
            &residuals,
            &reviewed_moves,
            format!("frontier:gwb:ambiguity:{hop_index}"),
        );
        let Some(selected) = select_gwb_investigation(&compiled).cloned() else {
            println!("stop_reason=FrontierExhausted");
            println!("gwb_committed_reviewed_hops={hop_index}");
            break;
        };

        println!("gwb_hop_index={hop_index}");
        println!("gwb_open_residuals={}", open_rows.len());
        println!("gwb_reviewed_moves={}", reviewed_moves.len());
        println!("selected_move={}", selected.move_ref);
        println!(
            "selected_investigation_kind={}",
            selected.investigation_kind.as_str()
        );
        println!(
            "selected_target_residuals={:?}",
            selected.target_residual_refs
        );

        let mut observed_routes = Vec::new();
        let mut supervised_type_closure_evidence = String::new();
        let (source_revision_ref, evidence_digest_ref) =
            if selected.investigation_kind == GwbInvestigationKind::CrossLanguageSurface
                && selected.target_ref.is_some()
            {
                let root_qid = selected
                    .source_ref
                    .as_deref()
                    .ok_or_else(|| std::io::Error::other("Wikipedia investigation missing root QID"))?;
                let target = selected
                    .target_ref
                    .as_deref()
                    .ok_or_else(|| std::io::Error::other("Wikipedia investigation missing target URL"))?;
                let acquired = acquire_wikipedia_article(root_qid, target)?;
                if !acquired.candidate_only || acquired.semantic_promotion {
                    return Err(std::io::Error::other(
                        "Wikipedia acquisition violated candidate-only boundary",
                    )
                    .into());
                }
                (
                    format!("wikipedia:{}:{}", acquired.language, acquired.revision_ref),
                    format!("sha256:{}", hex(&acquired.source_sha256)),
                )
            } else if matches!(
                selected.investigation_kind,
                GwbInvestigationKind::TypeClass
                    | GwbInvestigationKind::Superclass
                    | GwbInvestigationKind::Property
                    | GwbInvestigationKind::Identity
                    | GwbInvestigationKind::SourceWorkIdentity
                    | GwbInvestigationKind::CrossLanguageSurface
            ) {
                let qid = selected
                    .source_ref
                    .as_deref()
                    .ok_or_else(|| std::io::Error::other("Wikidata investigation missing QID"))?;
                let acquired = fetch_latest_entity_rdf_revision_receipt(qid)?;
                if !acquired.candidate_only || acquired.semantic_promotion {
                    return Err(std::io::Error::other(
                        "Wikidata acquisition violated candidate-only boundary",
                    )
                    .into());
                }
                observed_routes =
                    selected_route_observations(&selected, &acquired.rdf_bytes, qid)?;
                (acquired.source_revision_ref, acquired.content_digest_ref)
            } else if selected.investigation_kind
                == GwbInvestigationKind::ExternalOntologyFallback
            {
                let qid = selected
                    .source_ref
                    .as_deref()
                    .ok_or_else(|| std::io::Error::other(
                        "supervised classification fallback missing root QID"
                    ))?;
                let question = type_closure_question_for_selection(&selected, &residuals)
                    .ok_or_else(|| std::io::Error::other(
                        "external fallback is not a supervised type/class residual"
                    ))?;
                let closure = acquire_supervised_type_closure(&TypeClosureRequest {
                    root_qid: qid.to_owned(),
                    question,
                    max_depth: 4,
                    max_nodes: 32,
                })?;
                if !closure.candidate_only
                    || closure.creates_semantic_authority
                    || closure.applicability_promoted
                    || closure.claim_truth_promoted
                    || closure.superclass_residual_paid
                {
                    return Err(std::io::Error::other(
                        "supervised type closure violated non-promotion boundary"
                    )
                    .into());
                }
                println!(
                    "supervised_type_closure_disposition={}",
                    closure.disposition.as_str()
                );
                println!(
                    "supervised_type_closure_nodes={}",
                    closure.node_receipts.len()
                );
                println!(
                    "supervised_type_closure_truncated={}",
                    closure.truncated
                );
                observed_routes = type_closure_route_candidates(&closure);
                supervised_type_closure_evidence =
                    render_type_closure_review_evidence(&closure);
                (closure.source_manifest_ref, closure.evidence_digest_ref)
            } else {
                println!("stop_reason=DriverBlocked");
                println!("blocked_move={}", selected.move_ref);
                println!(
                    "blocked_requirement=governed-executor:{}",
                    selected.investigation_kind.as_str()
                );
                break;
            };

        let Some(assignment) = matching_review(hop_index, &selected.move_ref, &reviews) else {
            let bundle = pending_gwb_review_bundle(
                hop_index,
                &compiled,
                &selected,
                &source_revision_ref,
                &evidence_digest_ref,
            )?;
            let mut bundle = append_observations(bundle, &observed_routes);
            bundle.push_str(&supervised_type_closure_evidence);
            let path = pending_path(hop_index);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, bundle)?;
            println!("stop_reason=GwbReviewRequired");
            println!("gwb_review_bundle_path={}", path.display());
            println!("frontier_sha256={}", gwb_frontier_sha256(&compiled));
            println!("source_revision_ref={source_revision_ref}");
            println!("evidence_digest_ref={evidence_digest_ref}");
            println!("observed_route_count={}", observed_routes.len());
            break;
        };

        let prepared = prepare_reviewed_gwb_hop(
            hop_index,
            &compiled,
            &selected,
            &source_revision_ref,
            &evidence_digest_ref,
            assignment,
        )?;

        let close_residual_refs = if prepared.residual_effect == GwbResidualEffect::CloseReviewed {
            prepared.selected.target_residual_refs.clone()
        } else {
            vec![]
        };
        let opened_residuals = residuals_opened_by_reviewed_routes(
            hop_index,
            &prepared.selected,
            prepared.outcome.as_str(),
            &observed_routes,
        );
        let projected_after = project_open_gwb_state_after_review(
            &open_rows,
            &close_residual_refs,
            &opened_residuals,
        )?;

        let world_before_sha256 = gwb_ambiguity_world_sha256(&open_rows);
        let world_after_sha256 = gwb_ambiguity_world_sha256(&projected_after);
        let frontier_sha256 = gwb_frontier_sha256(&compiled);
        let opened_refs = opened_residuals
            .iter()
            .map(|row| row.residual_ref.clone())
            .collect::<Vec<_>>();

        let commit = materialize_gwb_campaign_commit(
            &pg_config,
            &GwbCampaignCommitInput {
                hop: GwbHopLedgerInput {
                    campaign_ref: GWB_ADAPTIVE_CAMPAIGN_REF.into(),
                    hop_index,
                    prior_receipt_sha256,
                    world_before_sha256,
                    frontier_sha256,
                    selected_move_ref: prepared.selected.move_ref.clone(),
                    investigation_kind_ref: prepared.selected.investigation_kind.as_str().into(),
                    producer_ref: prepared.selected.producer_ref.clone(),
                    source_revision_ref: prepared.source_revision_ref.clone(),
                    evidence_digest_ref: prepared.evidence_digest_ref.clone(),
                    review_ref: prepared.review_ref.clone(),
                    outcome_ref: prepared.outcome.as_str().into(),
                    residual_effect_ref: prepared.residual_effect.as_str().into(),
                    world_after_sha256,
                    closed_residual_refs: close_residual_refs.clone(),
                    opened_residual_refs: opened_refs,
                    candidate_only: true,
                    creates_semantic_authority: false,
                    applicability_promoted: false,
                    claim_truth_promoted: false,
                },
                close_residual_refs,
                open_residuals: opened_residuals,
            },
        )?;

        println!("hop_commit=reviewed-investigation");
        println!("hop_receipt_sha256={}", commit.hop_receipt_sha256);
        println!("residuals_closed={}", commit.residuals_closed);
        println!("residuals_opened={}", commit.residuals_opened);
        println!("candidate_only={}", commit.candidate_only);
        println!(
            "creates_semantic_authority={}",
            commit.creates_semantic_authority
        );
        println!("applicability_promoted={}", commit.applicability_promoted);
        println!("claim_truth_promoted={}", commit.claim_truth_promoted);
        if let Ok(rows) = load_gwb_hops(&pg_config, GWB_ADAPTIVE_CAMPAIGN_REF) {
            let analysis = analyze_gwb_hops(&rows);
            println!("{}", render_gwb_analysis_receipt(&analysis));
        }
        // Loop: reconstruct state, recompile the frontier, and select afresh.
    }

    Ok(())
}
