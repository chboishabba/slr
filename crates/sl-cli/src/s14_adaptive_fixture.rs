//! Three-hop adaptive S14 acceptance fixture.
//!
//! This is an executable calibration for the controller semantics, not a claim
//! that three live human-reviewed authorities have been completed.  Each hop is
//! a typed candidate delta, each accepted hop forces a fresh frontier
//! recomputation, and the emitted receipt records the observed frontier
//! transition rather than consuming a fixed work queue.

use crate::contracts::{run_native_expansion_trajectory, NativeExpansionBatch};
use serde_json::{json, Value};
use sensiblaw_legal_follow_plan::{
    australian_contract_landscape_seed, AuthorityLevel, ContractDoctrine,
    ContractLandscapeExpansionDelta, ContractTraceEdge, ContractTraceNode, SourceRole,
    TraceNodeKind, TreatmentKind,
};
use std::fs;
use std::path::PathBuf;

type CliResult<T = ()> = Result<T, String>;

const AS_AT: &str = "2026-09-20";

fn candidate_case(
    semantic_ref: &str,
    label: &str,
    doctrine: ContractDoctrine,
    citation: &str,
) -> ContractTraceNode {
    ContractTraceNode {
        semantic_ref: semantic_ref.into(),
        label: label.into(),
        kind: TraceNodeKind::CaseAuthority,
        doctrine: Some(doctrine),
        jurisdiction_ref: "AU".into(),
        court_ref: Some("court:fixture".into()),
        decision_or_effective_date: Some("2000-01-01".into()),
        valid_from: None,
        valid_to: None,
        source_role: SourceRole::PrimaryCaseLaw,
        authority_level: AuthorityLevel::Official,
        source_citation: citation.into(),
        candidate_only: true,
        creates_legal_authority: false,
    }
}

fn reviewed_candidate_delta(
    semantic_ref: &str,
    label: &str,
    doctrine: ContractDoctrine,
    citation: &str,
    provenance_ref: &str,
) -> ContractLandscapeExpansionDelta {
    ContractLandscapeExpansionDelta {
        discovered_nodes: vec![candidate_case(semantic_ref, label, doctrine, citation)],
        discovered_edges: vec![ContractTraceEdge {
            from_ref: "landscape:au:contract-law".into(),
            to_ref: semantic_ref.into(),
            treatment: TreatmentKind::Seeds,
            candidate_only: true,
            creates_legal_authority: false,
        }],
        provenance_ref: provenance_ref.into(),
        candidate_only: true,
        creates_legal_authority: false,
    }
}

pub fn build_three_hop_adaptive_fixture_receipt() -> CliResult<Value> {
    let trace = australian_contract_landscape_seed();
    let batches = vec![
        NativeExpansionBatch {
            source_ref: "reviewed-hop:fixture:construction".into(),
            deltas: vec![reviewed_candidate_delta(
                "case:fixture:construction",
                "fixture reviewed construction authority candidate",
                ContractDoctrine::Construction,
                "fixture:construction-primary-case",
                "reviewed-hop:fixture:construction",
            )],
            reviewed_residuals: Vec::new(),
        },
        NativeExpansionBatch {
            source_ref: "reviewed-hop:fixture:unconscionability".into(),
            deltas: vec![reviewed_candidate_delta(
                "case:fixture:unconscionability",
                "fixture reviewed unconscionability authority candidate",
                ContractDoctrine::Unconscionability,
                "fixture:unconscionability-primary-case",
                "reviewed-hop:fixture:unconscionability",
            )],
            reviewed_residuals: Vec::new(),
        },
        NativeExpansionBatch {
            source_ref: "reviewed-hop:fixture:penalties".into(),
            deltas: vec![reviewed_candidate_delta(
                "case:fixture:penalties",
                "fixture reviewed penalties authority candidate",
                ContractDoctrine::Penalties,
                "fixture:penalties-primary-case",
                "reviewed-hop:fixture:penalties",
            )],
            reviewed_residuals: Vec::new(),
        },
    ];

    let trajectory = run_native_expansion_trajectory(trace, batches, AS_AT, None)?;
    let hops = trajectory["trajectory"]
        .as_array()
        .ok_or_else(|| "adaptive fixture trajectory is not an array".to_string())?;
    if hops.len() != 3 {
        return Err(format!(
            "adaptive fixture expected three accepted hops, observed {}",
            hops.len()
        ));
    }

    let context_counts = hops
        .iter()
        .map(|hop| {
            hop["frontier_counts"]["context_expansion"]
                .as_u64()
                .ok_or_else(|| "adaptive fixture missing context frontier count".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !context_counts.windows(2).all(|pair| pair[1] < pair[0]) {
        return Err(format!(
            "adaptive fixture did not reopen/recompute a shrinking context frontier: {context_counts:?}"
        ));
    }

    let primary_counts = hops
        .iter()
        .map(|hop| {
            hop["frontier_counts"]["primary_source_acquisition"]
                .as_u64()
                .ok_or_else(|| "adaptive fixture missing primary-source frontier count".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !primary_counts.windows(2).all(|pair| pair[1] > pair[0]) {
        return Err(format!(
            "adaptive fixture did not surface newly added primary authorities: {primary_counts:?}"
        ));
    }

    for hop in hops {
        let receipt = &hop["expansion_receipt"];
        if receipt["recompute_frontier_required"] != true
            || receipt["old_source_history_preserved"] != true
            || receipt["old_conclusions_frozen"] != false
            || receipt["creates_legal_authority"] != false
            || receipt["creates_current_law_conclusion"] != false
        {
            return Err("adaptive fixture crossed an S14 expansion invariant".into());
        }
    }

    if trajectory["campaign_runtime"] != "ContractFollowCampaign"
        || trajectory.get("final_trace").is_none()
    {
        return Err("adaptive fixture did not run through resumable ContractFollowCampaign".into());
    }

    Ok(json!({
        "schema_version": "sl.australian_contracts.three_hop_adaptive_fixture.v0_2",
        "calibration_fixture_only": true,
        "campaign_runtime": "ContractFollowCampaign",
        "resumable_final_trace": true,
        "claims_live_human_reviewed_campaign": false,
        "hop_count": 3,
        "fresh_frontier_after_every_hop": true,
        "context_frontier_counts": context_counts,
        "primary_source_frontier_counts": primary_counts,
        "fixed_upfront_queue_consumed": false,
        "candidate_only": true,
        "creates_legal_authority": false,
        "creates_current_law_conclusion": false,
        "trajectory": trajectory,
    }))
}

pub fn run(args: Vec<String>) -> CliResult {
    let output = if let Some(index) = args.iter().position(|arg| arg == "--output") {
        let path = args
            .get(index + 1)
            .ok_or_else(|| "--output requires a path".to_string())?;
        let output = build_three_hop_adaptive_fixture_receipt()?;
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create {}: {error}", parent.display()))?;
        }
        fs::write(
            &path,
            serde_json::to_vec_pretty(&output)
                .map_err(|error| format!("encode adaptive fixture: {error}"))?,
        )
        .map_err(|error| format!("write {}: {error}", path.display()))?;
        println!("contracts_three_hop_adaptive_fixture={}", path.display());
        output
    } else {
        build_three_hop_adaptive_fixture_receipt()?
    };

    println!(
        "contracts_three_hop_hops={} fresh_frontier={} live_review_claim=false authority=false current_law_conclusion=false",
        output["hop_count"],
        output["fresh_frontier_after_every_hop"],
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_hop_fixture_recomputes_frontier_after_every_accepted_hop() {
        let receipt = build_three_hop_adaptive_fixture_receipt().unwrap();
        assert_eq!(receipt["hop_count"], 3);
        assert_eq!(receipt["fresh_frontier_after_every_hop"], true);
        assert_eq!(receipt["fixed_upfront_queue_consumed"], false);
        assert_eq!(receipt["campaign_runtime"], "ContractFollowCampaign");
        assert_eq!(receipt["resumable_final_trace"], true);
        assert_eq!(receipt["calibration_fixture_only"], true);
        assert_eq!(receipt["claims_live_human_reviewed_campaign"], false);
        assert_eq!(receipt["creates_legal_authority"], false);
        assert_eq!(receipt["creates_current_law_conclusion"], false);

        let context = receipt["context_frontier_counts"].as_array().unwrap();
        assert_eq!(context.len(), 3);
        assert!(context[1].as_u64().unwrap() < context[0].as_u64().unwrap());
        assert!(context[2].as_u64().unwrap() < context[1].as_u64().unwrap());

        let primary = receipt["primary_source_frontier_counts"].as_array().unwrap();
        assert_eq!(primary.len(), 3);
        assert!(primary[1].as_u64().unwrap() > primary[0].as_u64().unwrap());
        assert!(primary[2].as_u64().unwrap() > primary[1].as_u64().unwrap());
    }
}