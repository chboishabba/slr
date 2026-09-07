use crate::frontier::ProofFrontier;
use crate::transition::{FrontierTransitionReceipt, ResearchTermination};
use crate::world::ResearchWorldSnapshot;
use sensiblaw_proof_search_scheduler::{CandidateMove, CandidateMoveReceipt, ExecutionCostVector};

pub const ITERATION_SCHEMA: &str = "sl.proof_search_iteration.v0_1";
pub const FRONTIER_ITERATION_SCHEMA_V02: &str = "sl.proof_search_frontier_iteration.v0_2";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFrontierIterationReceiptV02 {
    pub schema_version: &'static str,
    pub base_iteration: CanonicalIterationReceipt,
    pub prior_frontier_ref: String,
    pub next_frontier_ref: String,
    pub changed_residual_refs: Vec<String>,
    pub termination: ResearchTermination,
    pub world_snapshot_ref: String,
    pub learned_query_terms: Vec<String>,
    pub learned_authority_refs: Vec<String>,
    pub reasoning_delta_refs: Vec<String>,
    pub authority_boundary: &'static str,
}

fn termination_name(termination: ResearchTermination) -> &'static str {
    match termination {
        ResearchTermination::Continue => "Continue",
        ResearchTermination::ClosedCandidate => "ClosedCandidate",
        ResearchTermination::Contested => "Contested",
        ResearchTermination::AuthorityBlocked => "AuthorityBlocked",
        ResearchTermination::Underidentified => "Underidentified",
        ResearchTermination::SaturatedCandidate => "SaturatedCandidate",
        ResearchTermination::BudgetExhausted => "BudgetExhausted",
    }
}

impl CanonicalFrontierIterationReceiptV02 {
    pub fn to_canonical_json(&self) -> String {
        format!(
            "{{\"schema_version\":{},\"base_iteration\":{},\"prior_frontier_ref\":{},\"next_frontier_ref\":{},\"changed_residual_refs\":{},\"termination\":{},\"world_snapshot_ref\":{},\"learned_query_terms\":{},\"learned_authority_refs\":{},\"reasoning_delta_refs\":{},\"authority_boundary\":{}}}",
            q(self.schema_version),
            self.base_iteration.to_canonical_json(),
            q(&self.prior_frontier_ref),
            q(&self.next_frontier_ref),
            arr(&self.changed_residual_refs),
            q(termination_name(self.termination)),
            q(&self.world_snapshot_ref),
            arr(&self.learned_query_terms),
            arr(&self.learned_authority_refs),
            arr(&self.reasoning_delta_refs),
            q(self.authority_boundary),
        )
    }
}

pub fn build_frontier_iteration_receipt_v02(
    base_iteration: CanonicalIterationReceipt,
    transition: &FrontierTransitionReceipt,
    world: &ResearchWorldSnapshot,
    reasoning_delta_refs: Vec<String>,
) -> CanonicalFrontierIterationReceiptV02 {
    CanonicalFrontierIterationReceiptV02 {
        schema_version: FRONTIER_ITERATION_SCHEMA_V02,
        base_iteration,
        prior_frontier_ref: transition.prior_frontier_ref.clone(),
        next_frontier_ref: transition.next_frontier_ref.clone(),
        changed_residual_refs: transition.changed_residual_refs.clone(),
        termination: transition.termination,
        world_snapshot_ref: world.snapshot_ref.clone(),
        learned_query_terms: world.query_vocabulary.iter().cloned().collect(),
        learned_authority_refs: world.authority_neighbourhood.iter().cloned().collect(),
        reasoning_delta_refs,
        authority_boundary: "experimental_candidate_only",
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

    #[test]
    fn frontier_receipt_v02_keeps_candidate_termination_and_learned_world() {
        let base = CanonicalIterationReceipt {
            schema_version: ITERATION_SCHEMA,
            runtime_head: "head".into(),
            consumer_ref: "c".into(),
            frontier_ref: "f0".into(),
            proof_gap_refs: vec!["r1".into()],
            candidate_move_refs: vec!["m1".into()],
            pareto_frontier_refs: vec!["m1".into()],
            selected_move_ref: "m1".into(),
            selected_source_revision_ref: None,
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
        let transition = FrontierTransitionReceipt {
            prior_frontier_ref: "f0".into(),
            next_frontier_ref: "f1".into(),
            changed_residual_refs: vec!["r1".into()],
            termination: ResearchTermination::ClosedCandidate,
            transition_authority: "experimental_candidate_only",
        };
        let mut world = ResearchWorldSnapshot::default();
        world.snapshot_ref = "world:1".into();
        world.query_vocabulary.insert("positive operational act".into());
        let receipt = build_frontier_iteration_receipt_v02(
            base,
            &transition,
            &world,
            vec!["reasoning:1".into()],
        );
        let json = receipt.to_canonical_json();
        assert!(json.contains(FRONTIER_ITERATION_SCHEMA_V02));
        assert!(json.contains("ClosedCandidate"));
        assert!(json.contains("positive operational act"));
    }
}
