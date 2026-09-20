use crate::waltons::{self, WaltonsPaths};
use sensiblaw_governed_legal_provider::{run_live_oalc_case_follow, OalcCaseFollowRequest};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

type CliResult<T = ()> = Result<T, String>;

const STATE_SCHEMA: &str = "sl.waltons.live_pipeline_state.v0_1";

fn require_live_network_feature() -> CliResult {
    if cfg!(feature = "live-network") {
        Ok(())
    } else {
        Err(
            "Waltons live OALC acquisition requires sensiblaw-cli built with --features live-network"
                .into(),
        )
    }
}

fn state_path(paths: &WaltonsPaths) -> PathBuf {
    paths.base.join("waltons-live-pipeline-state.json")
}

fn write_state(
    paths: &WaltonsPaths,
    stage: &str,
    gate: &str,
    next_command: &str,
    details: Value,
) -> CliResult {
    fs::create_dir_all(&paths.base)
        .map_err(|error| format!("create {}: {error}", paths.base.display()))?;
    let value = json!({
        "schema_version": STATE_SCHEMA,
        "stage": stage,
        "operator_gate": gate,
        "next_command": next_command,
        "base": paths.base,
        "candidate_only": true,
        "creates_legal_authority": false,
        "creates_current_law_conclusion": false,
        "details": details,
    });
    fs::write(
        state_path(paths),
        serde_json::to_vec_pretty(&value)
            .map_err(|error| format!("encode live pipeline state: {error}"))?,
    )
    .map_err(|error| format!("write {}: {error}", state_path(paths).display()))?;
    println!(
        "waltons_live_stage={} gate={} state={}",
        stage,
        gate,
        state_path(paths).display()
    );
    println!("waltons_live_next={next_command}");
    Ok(())
}

fn require(path: &Path, label: &str) -> CliResult {
    if path.exists() {
        Ok(())
    } else {
        Err(format!(
            "{label} is missing: {}. Run the preceding live pipeline stage first.",
            path.display()
        ))
    }
}

fn read_json(path: &Path) -> CliResult<Value> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode {}: {error}", path.display()))
}

pub fn status(paths: &WaltonsPaths) -> CliResult {
    waltons::status(paths);
    let state = state_path(paths);
    if state.exists() {
        let value = read_json(&state)?;
        println!(
            "waltons_live_state stage={} gate={} next={}",
            value["stage"], value["operator_gate"], value["next_command"]
        );
    } else {
        println!(
            "waltons_live_state=not_started next=sensiblaw legal-follow waltons --base {} live prepare",
            paths.base.display()
        );
    }
    Ok(())
}

/// Live OALC entrypoint.  This performs every deterministic source-side step
/// through generation of the first human paragraph-review worksheet.
pub fn prepare(paths: &WaltonsPaths) -> CliResult {
    require_live_network_feature()?;
    waltons::acquire(paths)?;
    waltons::materialise(paths)?;
    waltons::review_prepare(paths)?;
    waltons::cited_by_plan(paths)?;

    require(&paths.receipt, "Waltons OALC source receipt")?;
    require(&paths.text, "Waltons retained OALC text")?;
    require(&paths.queue, "Waltons materialised review queue")?;
    require(&paths.worksheet, "Waltons paragraph review worksheet")?;
    require(&paths.citedby_manifest, "Waltons cited-by manifest")?;

    write_state(
        paths,
        "paragraph_review_required",
        "human_paragraph_review",
        &format!(
            "edit {} then run: sensiblaw legal-follow waltons --base {} live paragraph-reviewed",
            paths.worksheet.display(),
            paths.base.display()
        ),
        json!({
            "oalc_source_receipt": paths.receipt,
            "retained_text": paths.text,
            "review_queue": paths.queue,
            "paragraph_review_worksheet": paths.worksheet,
            "cited_by_manifest": paths.citedby_manifest,
            "source_acquisition_complete": true,
            "paragraph_review_automatic": false,
        }),
    )
}

/// Resume after the operator has edited the paragraph-review worksheet.
/// Finalization/payment/frontier compilation are deterministic.  The next
/// irreducible gate is a sanctioned cited-by provider result.
pub fn paragraph_reviewed(paths: &WaltonsPaths) -> CliResult {
    require(&paths.worksheet, "Waltons paragraph review worksheet")?;
    waltons::review_finalize(paths)?;
    waltons::review_compile(paths)?;
    waltons::frontier(paths)?;
    waltons::s14_sync(paths)?;

    write_state(
        paths,
        "cited_by_provider_required",
        "external_cited_by_provider",
        &format!(
            "obtain provider results, then run: sensiblaw legal-follow waltons --base {} live cited-by PROVIDER_RESULTS.json",
            paths.base.display()
        ),
        json!({
            "paragraph_review_decisions": paths.decisions,
            "reviewed_proposition_receipts": paths.reviewed,
            "reviewed_evidence_payments": paths.payments,
            "wrongtype_frontier": paths.frontier,
            "proposition_contract_hops": paths.proposition_hops,
            "s14_trajectory": paths.s14_trajectory,
            "cited_by_manifest": paths.citedby_manifest,
            "provider_result_is_treatment": false,
            "provider_failure_is_negative_legal_evidence": false,
        }),
    )
}

/// Consume a sanctioned cited-by provider result, re-acquire every candidate
/// through live OALC, and materialise the reviewed authority-identity gate.
fn safe_citation_dir(citation: &str) -> String {
    citation
        .replace('[', "")
        .replace(']', "")
        .replace(' ', "-")
        .to_ascii_lowercase()
}

fn reacquire_cited_by_candidates_resilient(paths: &WaltonsPaths) -> CliResult<Value> {
    let normalized = read_json(&paths.citedby_candidates)?;
    let candidates = normalized["candidates"]
        .as_array()
        .ok_or_else(|| "normalized cited-by candidates missing candidates array".to_string())?;

    fs::create_dir_all(&paths.later_dir)
        .map_err(|error| format!("create {}: {error}", paths.later_dir.display()))?;

    let mut resolved = Vec::new();
    let mut residuals = Vec::new();
    for candidate in candidates {
        let Some(citation) = candidate["medium_neutral_citation"].as_str() else {
            residuals.push(json!({
                "citation": null,
                "state": "source_residual",
                "reason": "candidate missing medium_neutral_citation",
                "missing_source_is_negative_legal_evidence": false,
            }));
            continue;
        };
        let output_dir = paths.later_dir.join(safe_citation_dir(citation));
        let mut request = OalcCaseFollowRequest::for_citation(citation, output_dir);
        request.as_at = waltons::DEFAULT_AS_AT.into();
        match run_live_oalc_case_follow(&request) {
            Ok(run) => resolved.push(json!({
                "citation": citation,
                "state": "source_resolved",
                "source_receipt": run.source_receipt_path,
                "canonical_text": run.canonical_text_path,
                "candidate_only": true,
                "creates_legal_authority": false,
            })),
            Err(error) => residuals.push(json!({
                "citation": citation,
                "state": "source_residual",
                "reason": format!("{error:?}"),
                "missing_source_is_negative_legal_evidence": false,
                "candidate_only": true,
                "creates_legal_authority": false,
            })),
        }
    }

    let report = json!({
        "schema_version": "sl.waltons.cited_by_oalc_acquisition.v0_1",
        "candidate_count": candidates.len(),
        "resolved_count": resolved.len(),
        "residual_count": residuals.len(),
        "missing_source_is_negative_legal_evidence": false,
        "candidate_only": true,
        "creates_legal_authority": false,
        "resolved": resolved,
        "residuals": residuals,
    });
    let report_path = paths.later_dir.join("oalc-acquisition-report.json");
    fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report)
            .map_err(|error| format!("encode cited-by OALC acquisition report: {error}"))?,
    )
    .map_err(|error| format!("write {}: {error}", report_path.display()))?;

    if report["resolved_count"].as_u64().unwrap_or_default() == 0 {
        return Err(format!(
            "no cited-by candidates resolved through OALC; inspect {}. Source misses are residuals, not negative legal evidence.",
            report_path.display()
        ));
    }
    Ok(report)
}

pub fn cited_by(paths: &WaltonsPaths, provider_results: &Path) -> CliResult {
    require_live_network_feature()?;
    require(&paths.citedby_manifest, "Waltons cited-by manifest")?;
    waltons::cited_by_import(paths, provider_results)?;
    waltons::cited_by_worklist(paths)?;
    let acquisition = reacquire_cited_by_candidates_resilient(paths)?;
    waltons::identity_prepare(paths)?;

    require(&paths.citedby_candidates, "normalized cited-by candidates")?;
    require(&paths.identity_worksheet, "authority identity review worksheet")?;

    write_state(
        paths,
        "authority_identity_review_required",
        "human_authority_identity_review",
        &format!(
            "edit {} then run: sensiblaw legal-follow waltons --base {} live identity-reviewed",
            paths.identity_worksheet.display(),
            paths.base.display()
        ),
        json!({
            "provider_results": provider_results,
            "normalized_candidates": paths.citedby_candidates,
            "later_authorities_dir": paths.later_dir,
            "identity_review_worksheet": paths.identity_worksheet,
            "later_authorities_reacquired_via_oalc": true,
            "oalc_acquisition": acquisition,
            "missing_source_is_negative_legal_evidence": false,
            "provider_candidates_create_treatment": false,
            "raw_oalc_identity_creates_trace_alias": false,
        }),
    )
}

/// Resume after reviewed document:oalc:* -> canonical contract authority
/// identity decisions.  Identity hops are compiled, treatment candidates are
/// extracted from the reacquired judgments, and the human treatment worksheet
/// is prepared.
pub fn identity_reviewed(paths: &WaltonsPaths) -> CliResult {
    require(&paths.identity_worksheet, "authority identity review worksheet")?;
    waltons::identity_finalize(paths)?;
    waltons::identity_compile(paths)?;
    waltons::treatment_queue(paths)?;
    waltons::treatment_merge(paths)?;
    waltons::treatment_prepare(paths)?;
    waltons::s14_sync(paths)?;

    require(&paths.identity_hops, "reviewed authority identity hops")?;
    require(&paths.merged_treatment_queue, "merged treatment queue")?;
    require(&paths.treatment_worksheet, "treatment review worksheet")?;

    write_state(
        paths,
        "treatment_review_required",
        "human_treatment_review",
        &format!(
            "edit {} then run: sensiblaw legal-follow waltons --base {} live treatment-reviewed",
            paths.treatment_worksheet.display(),
            paths.base.display()
        ),
        json!({
            "identity_decisions": paths.identity_decisions,
            "identity_contract_hops": paths.identity_hops,
            "treatment_queue": paths.merged_treatment_queue,
            "treatment_review_worksheet": paths.treatment_worksheet,
            "s14_trajectory": paths.s14_trajectory,
            "citation_occurrence_is_not_treatment": true,
        }),
    )
}

fn validate_final_s14_trajectory(paths: &WaltonsPaths, trajectory: &Value) -> CliResult<Value> {
    if trajectory["transport"] != "typed_rust_in_process"
        || trajectory["json_is_semantic_command_transport"] != false
        || trajectory["creates_legal_authority"] != false
        || trajectory["creates_current_law_conclusion"] != false
    {
        return Err("final S14 trajectory crossed the native transport/authority boundary".into());
    }

    let hops = trajectory["trajectory"]
        .as_array()
        .ok_or_else(|| "final S14 trajectory missing trajectory array".to_string())?;
    let mut bootstrap_hops = 0u64;
    let mut identity_hops = 0u64;
    let mut proposition_hops = 0u64;
    let mut treatment_hops = 0u64;
    let identity_ref = paths.identity_hops.display().to_string();
    let proposition_ref = paths.proposition_hops.display().to_string();
    let treatment_ref = paths.treatment_hops.display().to_string();

    for hop in hops {
        let receipt = &hop["expansion_receipt"];
        if receipt["recompute_frontier_required"] != true
            || receipt["old_source_history_preserved"] != true
            || receipt["old_conclusions_frozen"] != false
            || receipt["creates_legal_authority"] != false
            || receipt["creates_current_law_conclusion"] != false
        {
            return Err("final S14 trajectory contains a hop that violates recomputation/history/authority invariants".into());
        }
        let source_ref = hop["source_ref"].as_str().unwrap_or_default();
        if source_ref.starts_with("bootstrap:") {
            bootstrap_hops += 1;
        } else if source_ref == identity_ref.as_str() {
            identity_hops += 1;
        } else if source_ref == proposition_ref.as_str() {
            proposition_hops += 1;
        } else if source_ref == treatment_ref.as_str() {
            treatment_hops += 1;
        }
    }

    let reviewed_hops = identity_hops + proposition_hops + treatment_hops;
    Ok(json!({
        "accepted_hop_count": hops.len(),
        "bootstrap_hop_count": bootstrap_hops,
        "reviewed_identity_hop_count": identity_hops,
        "reviewed_proposition_hop_count": proposition_hops,
        "reviewed_treatment_hop_count": treatment_hops,
        "accepted_reviewed_hop_count": reviewed_hops,
        "has_accepted_reviewed_hop": reviewed_hops > 0,
        "recompute_after_every_accepted_hop": true,
        "old_source_history_preserved": true,
        "old_conclusions_frozen": false,
        "typed_rust_transport": true,
    }))
}

/// Final deterministic stage after human treatment decisions.  This compiles
/// proposition-level citation-use receipts, temporal genealogy, candidate S14
/// treatment edges and the final typed adaptive trajectory.
pub fn treatment_reviewed(paths: &WaltonsPaths) -> CliResult {
    require(&paths.treatment_worksheet, "treatment review worksheet")?;
    waltons::treatment_finalize(paths)?;
    waltons::genealogy(paths)?;
    waltons::s14_sync(paths)?;

    require(&paths.genealogy, "reviewed temporal treatment genealogy")?;
    require(&paths.treatment_hops, "reviewed treatment contract hops")?;
    require(&paths.s14_trajectory, "S14 adaptive trajectory")?;

    let trajectory = read_json(&paths.s14_trajectory)?;
    let validated = validate_final_s14_trajectory(paths, &trajectory)?;
    let residual_count = trajectory["reviewed_residual_count"]
        .as_u64()
        .unwrap_or_default();

    write_state(
        paths,
        "complete",
        "none",
        "none",
        json!({
            "treatment_decisions": paths.treatment_decisions,
            "genealogy": paths.genealogy,
            "treatment_contract_hops": paths.treatment_hops,
            "s14_trajectory": paths.s14_trajectory,
            "trajectory_validation": validated,
            "reviewed_residual_count": residual_count,
            "live_oalc_source_chain_exercised": true,
            "candidate_only": true,
            "creates_legal_authority": false,
            "creates_current_law_conclusion": false,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_live_pipeline_has_a_deterministic_entrypoint() {
        let base = std::env::temp_dir().join(format!(
            "sensiblaw-waltons-live-state-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        let paths = WaltonsPaths::from_base(base.clone());
        write_state(
            &paths,
            "paragraph_review_required",
            "human_paragraph_review",
            "next",
            json!({"fixture": true}),
        )
        .unwrap();
        let state = read_json(&state_path(&paths)).unwrap();
        assert_eq!(state["schema_version"], STATE_SCHEMA);
        assert_eq!(state["operator_gate"], "human_paragraph_review");
        assert_eq!(state["creates_legal_authority"], false);
        assert_eq!(state["creates_current_law_conclusion"], false);
        let _ = fs::remove_dir_all(base);
    }
}
