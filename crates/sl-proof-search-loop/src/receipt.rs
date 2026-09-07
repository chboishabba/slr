use crate::frontier::ProofFrontier;
use sensiblaw_proof_search_scheduler::{CandidateMove, CandidateMoveReceipt, ExecutionCostVector};

pub const ITERATION_SCHEMA: &str = "sl.proof_search_iteration.v0_1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalIterationReceipt {
    pub schema_version: &'static str,
    pub runtime_head: String,
    pub consumer_ref: String,
    pub frontier_ref: String,
    pub proof_gap_refs: Vec<String>,
    pub candidate_move_refs: Vec<String>,
    pub pareto_frontier_refs: Vec<String>,
    pub selected_move_ref: String,
    pub selected_source_revision_ref: Option<String>,
    pub execution_cost: ExecutionCostVector,
    pub artifact_digest_ref: Option<String>,
    pub pnf_receipt_ref: Option<String>,
    pub assessment_ref: Option<String>,
    pub frontier_delta_ref: Option<String>,
    pub wake_refs: Vec<String>,
    pub next_move_ref: Option<String>,
    pub authority_boundary: &'static str,
    pub input_digest_ref: String,
    pub output_digest_ref: String,
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn q(s: &str) -> String { format!("\"{}\"", esc(s)) }
fn arr(xs: &[String]) -> String { format!("[{}]", xs.iter().map(|x| q(x)).collect::<Vec<_>>().join(",")) }
fn opt(v: &Option<String>) -> String { v.as_ref().map(|x| q(x)).unwrap_or_else(|| "null".into()) }

impl CanonicalIterationReceipt {
    pub fn to_canonical_json(&self) -> String {
        format!(
            "{{\"schema_version\":{},\"runtime_head\":{},\"consumer_ref\":{},\"frontier_ref\":{},\"proof_gap_refs\":{},\"candidate_move_refs\":{},\"pareto_frontier_refs\":{},\"selected_move_ref\":{},\"selected_source_revision_ref\":{},\"execution_cost\":{{\"network_requests\":{},\"minimum_pacing_seconds\":{},\"citation_depth\":{},\"maximum_new_documents\":{},\"cache_misses\":{},\"local_bytes_read_cost\":{},\"parser_pnf_cost\":{},\"semantic_assessment_cost\":{},\"operator_review_cost\":{}}},\"artifact_digest_ref\":{},\"pnf_receipt_ref\":{},\"assessment_ref\":{},\"frontier_delta_ref\":{},\"wake_refs\":{},\"next_move_ref\":{},\"authority_boundary\":{},\"input_digest_ref\":{},\"output_digest_ref\":{}}}",
            q(self.schema_version), q(&self.runtime_head), q(&self.consumer_ref), q(&self.frontier_ref),
            arr(&self.proof_gap_refs), arr(&self.candidate_move_refs), arr(&self.pareto_frontier_refs),
            q(&self.selected_move_ref), opt(&self.selected_source_revision_ref),
            self.execution_cost.network_requests, self.execution_cost.minimum_pacing_seconds,
            self.execution_cost.citation_depth, self.execution_cost.maximum_new_documents,
            self.execution_cost.cache_misses, self.execution_cost.local_bytes_read_cost,
            self.execution_cost.parser_pnf_cost, self.execution_cost.semantic_assessment_cost,
            self.execution_cost.operator_review_cost, opt(&self.artifact_digest_ref),
            opt(&self.pnf_receipt_ref), opt(&self.assessment_ref), opt(&self.frontier_delta_ref),
            arr(&self.wake_refs), opt(&self.next_move_ref), q(self.authority_boundary),
            q(&self.input_digest_ref), q(&self.output_digest_ref),
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_iteration_receipt(
    runtime_head: &str,
    frontier: &ProofFrontier,
    candidates: &[CandidateMove],
    selected: &CandidateMoveReceipt,
    selected_move: &CandidateMove,
    artifact_digest_ref: Option<String>,
    pnf_receipt_ref: Option<String>,
    assessment_ref: Option<String>,
    frontier_delta_ref: Option<String>,
    wake_refs: Vec<String>,
    next_move_ref: Option<String>,
    input_digest_ref: String,
    output_digest_ref: String,
) -> CanonicalIterationReceipt {
    CanonicalIterationReceipt {
        schema_version: ITERATION_SCHEMA,
        runtime_head: runtime_head.into(),
        consumer_ref: frontier.consumer_ref.clone(),
        frontier_ref: frontier.frontier_ref.clone(),
        proof_gap_refs: frontier.open_residuals().map(|r| r.residual_ref.clone()).collect(),
        candidate_move_refs: candidates.iter().map(|m| m.move_ref.clone()).collect(),
        pareto_frontier_refs: selected.frontier_move_refs.clone(),
        selected_move_ref: selected.selected_move_ref.clone(),
        selected_source_revision_ref: selected_move.source_ref.clone(),
        execution_cost: selected_move.cost,
        artifact_digest_ref,
        pnf_receipt_ref,
        assessment_ref,
        frontier_delta_ref,
        wake_refs,
        next_move_ref,
        authority_boundary: "experimental_candidate_only",
        input_digest_ref,
        output_digest_ref,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_is_deterministic_for_same_receipt() {
        let receipt = CanonicalIterationReceipt {
            schema_version: ITERATION_SCHEMA,
            runtime_head: "head".into(),
            consumer_ref: "c".into(),
            frontier_ref: "f".into(),
            proof_gap_refs: vec!["r1".into(), "r2".into()],
            candidate_move_refs: vec!["m1".into()],
            pareto_frontier_refs: vec!["m1".into()],
            selected_move_ref: "m1".into(),
            selected_source_revision_ref: Some("s1".into()),
            execution_cost: ExecutionCostVector::default(),
            artifact_digest_ref: None,
            pnf_receipt_ref: None,
            assessment_ref: None,
            frontier_delta_ref: None,
            wake_refs: vec![],
            next_move_ref: None,
            authority_boundary: "experimental_candidate_only",
            input_digest_ref: "in".into(),
            output_digest_ref: "out".into(),
        };
        assert_eq!(receipt.to_canonical_json(), receipt.to_canonical_json());
        assert!(receipt.to_canonical_json().contains("sl.proof_search_iteration.v0_1"));
    }
}
