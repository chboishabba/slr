use std::collections::BTreeSet;

use sensiblaw_pg_source_store::{
    load_claims_for_event, load_contestation_relations_for_claims,
    load_database_config, load_gwb_chronology_capstone_manifest,
    load_observation_event_links_for_event, load_review_queue,
    load_source_statement, load_statement_observation_links_for_observation,
    load_temporal_assertions_for_event, materialize_gwb_chronology_capstone,
};

fn main() -> Result<(), String> {
    let manifest_path = std::env::args()
        .nth(1)
        .ok_or_else(|| {
            "usage: gwb_chronology_capstone <reviewed-manifest.json>".to_owned()
        })?;

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let manifest = load_gwb_chronology_capstone_manifest(&manifest_path)
        .map_err(|error| error.to_string())?;
    let receipt = materialize_gwb_chronology_capstone(&config, &manifest)
        .map_err(|error| error.to_string())?;

    println!("schema={}", manifest.schema);
    println!("matter_ref={}", receipt.matter_ref);
    println!("handoff_ref={}", receipt.handoff_ref);
    println!("statement_count={}", receipt.statement_count);
    println!("source_family_count={}", receipt.source_family_count);
    println!("event_count={}", receipt.event_count);
    println!(
        "temporal_assertion_count={}",
        receipt.temporal_assertion_count
    );
    println!("proposition_root_count={}", receipt.proposition_root_count);
    println!("claim_leaf_count={}", receipt.claim_leaf_count);
    println!(
        "contestation_relation_count={}",
        receipt.contestation_relation_count
    );
    println!("review_item_count={}", receipt.review_item_count);

    if !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
    {
        return Err("capstone receipt crossed non-promotion boundary".into());
    }

    let mut total_source_traces = 0usize;
    for event_ref in &receipt.event_refs {
        let event_links = load_observation_event_links_for_event(&config, event_ref)
            .map_err(|error| error.to_string())?;
        if event_links.is_empty() {
            return Err(format!("event {event_ref} has no observation links"));
        }

        let mut statement_refs = BTreeSet::new();
        for event_link in &event_links {
            let statement_links =
                load_statement_observation_links_for_observation(
                    &config,
                    &event_link.observation_ref,
                )
                .map_err(|error| error.to_string())?;
            if statement_links.is_empty() {
                return Err(format!(
                    "observation {} has no statement ancestry",
                    event_link.observation_ref
                ));
            }

            for statement_link in statement_links {
                let statement =
                    load_source_statement(&config, &statement_link.statement_ref)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| {
                            format!(
                                "missing persisted statement {}",
                                statement_link.statement_ref
                            )
                        })?;
                if statement.literal_text.is_empty() {
                    return Err(format!(
                        "statement {} lost literal text",
                        statement.statement_ref
                    ));
                }
                statement_refs.insert(statement.statement_ref);
                total_source_traces += 1;
            }
        }

        let temporals = load_temporal_assertions_for_event(&config, event_ref)
            .map_err(|error| error.to_string())?;
        let claims =
            load_claims_for_event(&config, event_ref).map_err(|error| error.to_string())?;
        let claim_refs = claims
            .iter()
            .map(|claim| claim.claim_ref.clone())
            .collect::<Vec<_>>();
        let relations =
            load_contestation_relations_for_claims(&config, &claim_refs)
                .map_err(|error| error.to_string())?;

        println!(
            "event={} observations={} statements={} temporals={} claims={} relations={}",
            event_ref,
            event_links.len(),
            statement_refs.len(),
            temporals.len(),
            claims.len(),
            relations.len(),
        );
    }

    if total_source_traces == 0 {
        return Err("capstone produced no event→observation→statement trace".into());
    }

    let review_items =
        load_review_queue(&config).map_err(|error| error.to_string())?;
    let gwb_review_items = review_items
        .iter()
        .filter(|item| {
            item.affected_consumer_refs
                .iter()
                .any(|value| value.contains(&receipt.matter_ref))
                || item.semantic_ref.starts_with("event:gwb:")
                || item.semantic_ref.starts_with("claim:gwb:")
        })
        .count();

    println!("m12_source_trace_count={total_source_traces}");
    println!("s29_gwb_review_queue_count={gwb_review_items}");
    println!("candidate_only=true");
    println!("creates_semantic_authority=false");
    println!("applicability_promoted=false");
    println!("claim_truth_promoted=false");

    Ok(())
}
