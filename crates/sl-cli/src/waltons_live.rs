use crate::waltons::{self, WaltonsPaths};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

type CliResult<T = ()> = Result<T, String>;

const STATE_SCHEMA: &str = "sl.waltons.live_pipeline_state.v0_1";

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
pub fn cited_by(paths: &WaltonsPaths, provider_results: &Path) -> CliResult {
    require(&paths.citedby_manifest, "Waltons cited-by manifest")?;
    waltons::cited_by_import(paths, provider_results)?;
    waltons::cited_by_worklist(paths)?;
    waltons::cited_by_acquire(paths)?;
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
    let hop_count = trajectory["hop_count"].as_u64().unwrap_or_default();
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
            "accepted_hop_count": hop_count,
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
