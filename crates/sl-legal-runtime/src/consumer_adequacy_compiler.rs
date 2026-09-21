//! S15.6 consumer adequacy witness compiler.
//!
//! This is the executable bridge between operational closure and theorem-bearing
//! consumer adequacy.  It never treats CurrentFrontierClosed as proof.  A closed
//! operational frontier may be reopened when an exact query-indexed
//! NonFactorability witness exposes a missing axis.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    assess_consumer_adequacy, certify_consumer_adequacy,
    compile_nonfactorability_residual, ConsumerAdequacyDisposition,
    ConsumerAdequacyReceipt, ConsumerAxis, ConsumerCoverage, ConsumerQueryDemand,
    ExactConsumerResidual, KernelCheckedFactorsThroughWitness,
    NonFactorabilityWitnessReceipt, ProjectionGraph,
    TheoremBackedConsumerAdequacyReceipt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationalResearchState {
    Open,
    CurrentFrontierClosed,
    BudgetExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedsResearchCompilation {
    pub runtime_receipt: ConsumerAdequacyReceipt,
    pub exact_residuals: Vec<ExactConsumerResidual>,
    /// Runtime demands that do not yet have an exact formal defect witness.
    pub unproved_demand_reason_refs: Vec<String>,
    pub operational_frontier_was_closed: bool,
    pub reopened_by_nonfactorability: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplicitlyUnresolvedCompilation {
    pub runtime_receipt: ConsumerAdequacyReceipt,
    pub operational_frontier_closed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentFrontierClosedWithoutAdequacy {
    pub runtime_receipt: ConsumerAdequacyReceipt,
    pub adequacy_proof_required: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetExhaustedWithoutAdequacy {
    pub runtime_receipt: ConsumerAdequacyReceipt,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsumerAdequacyCompilation {
    ConsumerAdequate(TheoremBackedConsumerAdequacyReceipt),
    NeedsResearch(NeedsResearchCompilation),
    ExplicitlyUnresolved(ExplicitlyUnresolvedCompilation),
    CurrentFrontierClosedWithoutAdequacy(CurrentFrontierClosedWithoutAdequacy),
    BudgetExhaustedWithoutAdequacy(BudgetExhaustedWithoutAdequacy),
    AdequacyProofRequired {
        runtime_receipt: ConsumerAdequacyReceipt,
        candidate_only: bool,
        creates_semantic_authority: bool,
        creates_claim_truth: bool,
    },
}

fn exact_witnesses_by_axis<'a>(
    witnesses: &'a [NonFactorabilityWitnessReceipt],
    query_ref: &str,
    projection_digest: &str,
) -> Result<BTreeMap<ConsumerAxis, &'a NonFactorabilityWitnessReceipt>, String> {
    let mut by_axis = BTreeMap::new();
    for witness in witnesses {
        if witness.query_ref != query_ref || witness.projection_digest != projection_digest {
            return Err(
                "nonfactorability witness does not match compiler query/projection coordinates"
                    .into(),
            );
        }
        if let Some(existing) = by_axis.insert(witness.lost_axis, witness) {
            if existing != witness {
                return Err(format!(
                    "multiple nonfactorability witnesses disagree for axis {:?}",
                    witness.lost_axis
                ));
            }
        }
    }
    Ok(by_axis)
}

pub fn compile_consumer_adequacy(
    demand: &ConsumerQueryDemand,
    graph: &ProjectionGraph,
    coverage: &ConsumerCoverage,
    operational_state: OperationalResearchState,
    formal_adequacy: Option<&KernelCheckedFactorsThroughWitness>,
    nonfactorability_witnesses: &[NonFactorabilityWitnessReceipt],
) -> Result<ConsumerAdequacyCompilation, String> {
    let runtime = assess_consumer_adequacy(demand, graph, coverage)?;

    if let Some(witness) = formal_adequacy {
        let certified = certify_consumer_adequacy(&runtime, graph, witness)?;
        return Ok(ConsumerAdequacyCompilation::ConsumerAdequate(certified));
    }

    match runtime.disposition {
        ConsumerAdequacyDisposition::NeedsResearch => {
            let by_axis = exact_witnesses_by_axis(
                nonfactorability_witnesses,
                &runtime.query_ref,
                &graph.deterministic_digest,
            )?;
            let target_ref = demand
                .required_semantic_refs
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| demand.query_ref.clone());

            let mut exact_residuals = Vec::new();
            let mut unproved_demand_reason_refs = Vec::new();
            for research_demand in &runtime.research_demands {
                if let Some(witness) = by_axis.get(&research_demand.axis) {
                    exact_residuals.push(compile_nonfactorability_residual(
                        witness,
                        target_ref.clone(),
                    )?);
                } else {
                    unproved_demand_reason_refs.push(research_demand.reason_ref.clone());
                }
            }
            exact_residuals.sort_by(|left, right| left.residual_ref.cmp(&right.residual_ref));
            unproved_demand_reason_refs.sort();
            unproved_demand_reason_refs.dedup();

            let operational_frontier_was_closed =
                operational_state == OperationalResearchState::CurrentFrontierClosed;
            Ok(ConsumerAdequacyCompilation::NeedsResearch(
                NeedsResearchCompilation {
                    reopened_by_nonfactorability: operational_frontier_was_closed
                        && !exact_residuals.is_empty(),
                    runtime_receipt: runtime,
                    exact_residuals,
                    unproved_demand_reason_refs,
                    operational_frontier_was_closed,
                    candidate_only: true,
                    creates_semantic_authority: false,
                    creates_claim_truth: false,
                },
            ))
        }
        ConsumerAdequacyDisposition::ExplicitlyUnresolved => {
            Ok(ConsumerAdequacyCompilation::ExplicitlyUnresolved(
                ExplicitlyUnresolvedCompilation {
                    runtime_receipt: runtime,
                    operational_frontier_closed:
                        operational_state == OperationalResearchState::CurrentFrontierClosed,
                    candidate_only: true,
                    creates_semantic_authority: false,
                    creates_claim_truth: false,
                },
            ))
        }
        ConsumerAdequacyDisposition::Adequate => match operational_state {
            OperationalResearchState::CurrentFrontierClosed => Ok(
                ConsumerAdequacyCompilation::CurrentFrontierClosedWithoutAdequacy(
                    CurrentFrontierClosedWithoutAdequacy {
                        runtime_receipt: runtime,
                        adequacy_proof_required: true,
                        candidate_only: true,
                        creates_semantic_authority: false,
                        creates_claim_truth: false,
                    },
                ),
            ),
            OperationalResearchState::BudgetExhausted => Ok(
                ConsumerAdequacyCompilation::BudgetExhaustedWithoutAdequacy(
                    BudgetExhaustedWithoutAdequacy {
                        runtime_receipt: runtime,
                        candidate_only: true,
                        creates_semantic_authority: false,
                        creates_claim_truth: false,
                    },
                ),
            ),
            OperationalResearchState::Open => Ok(
                ConsumerAdequacyCompilation::AdequacyProofRequired {
                    runtime_receipt: runtime,
                    candidate_only: true,
                    creates_semantic_authority: false,
                    creates_claim_truth: false,
                },
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        kernel_checked_factors_through_witness, AgdaFactorsThroughTypecheckReceipt,
        ConsumerResearchDemandKind, ProjectionKind, ProjectionNode,
    };
    use std::collections::BTreeSet;

    fn graph() -> ProjectionGraph {
        ProjectionGraph {
            kind: ProjectionKind::IssueProof,
            nodes: vec![ProjectionNode {
                semantic_ref: "case:mature".into(),
                semantic_kind: "CaseAuthority".into(),
                manifestation_refs: vec!["manifestation:mature".into()],
                source_revision_refs: vec!["revision:mature".into()],
                span_refs: vec!["span:mature".into()],
                projection_role: "IssueProof".into(),
            }],
            edges: vec![],
            deterministic_digest: "sha256:mature-projection".into(),
            projection_only: true,
            creates_semantic_authority: false,
        }
    }

    fn query(required_axes: BTreeSet<ConsumerAxis>) -> ConsumerQueryDemand {
        ConsumerQueryDemand {
            query_ref: "query:mature".into(),
            required_axes,
            required_semantic_refs: BTreeSet::from(["case:mature".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        }
    }

    fn checked_witness() -> KernelCheckedFactorsThroughWitness {
        kernel_checked_factors_through_witness(&AgdaFactorsThroughTypecheckReceipt {
            schema_version: "sl.formal.agda_factors_through_typecheck.v0_1".into(),
            verifier: "agda".into(),
            command_ref:
                "agda -i . DASHI/Law/ConsumerAdequacyRuntimeTheoremBridgeExact.agda".into(),
            exit_code: 0,
            query_ref: "query:mature".into(),
            projection_digest: "sha256:mature-projection".into(),
            theorem_module_ref: "DASHI.Law.ConsumerAdequacyRuntimeTheoremBridgeExact".into(),
            theorem_ref: "demoFormalAdequacyRecovered".into(),
            theorem_artifact_digest:
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .into(),
            exact_query_indexed: true,
            factors_through_claim: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        })
        .unwrap()
    }

    #[test]
    fn mature_world_no_fresh_demand_is_not_consumer_adequate() {
        let result = compile_consumer_adequacy(
            &query(BTreeSet::from([
                ConsumerAxis::SemanticIdentity,
                ConsumerAxis::SourceRevision,
                ConsumerAxis::SourceSpan,
                ConsumerAxis::Provenance,
            ])),
            &graph(),
            &ConsumerCoverage::default(),
            OperationalResearchState::CurrentFrontierClosed,
            None,
            &[],
        )
        .unwrap();

        let ConsumerAdequacyCompilation::CurrentFrontierClosedWithoutAdequacy(receipt) =
            result
        else {
            panic!("closed mature world must remain operational closure without proof");
        };
        assert!(receipt.adequacy_proof_required);
        assert!(!receipt.runtime_receipt.factors_through_formally_proved);
    }

    #[test]
    fn mature_world_with_kernel_checked_witness_is_consumer_adequate() {
        let result = compile_consumer_adequacy(
            &query(BTreeSet::from([
                ConsumerAxis::SemanticIdentity,
                ConsumerAxis::SourceRevision,
                ConsumerAxis::SourceSpan,
                ConsumerAxis::Provenance,
            ])),
            &graph(),
            &ConsumerCoverage::default(),
            OperationalResearchState::CurrentFrontierClosed,
            Some(&checked_witness()),
            &[],
        )
        .unwrap();

        let ConsumerAdequacyCompilation::ConsumerAdequate(receipt) = result else {
            panic!("kernel-checked FactorsThrough witness must certify adequacy");
        };
        assert!(receipt.consumer_adequate);
        assert!(receipt.factors_through_formally_proved);
        assert_eq!(receipt.formal_verifier, "agda");
    }

    #[test]
    fn closed_frontier_losing_temporal_axis_reopens_exact_research() {
        let demand = query(BTreeSet::from([
            ConsumerAxis::SemanticIdentity,
            ConsumerAxis::SourceRevision,
            ConsumerAxis::SourceSpan,
            ConsumerAxis::Provenance,
            ConsumerAxis::Temporal,
        ]));
        let defect = NonFactorabilityWitnessReceipt {
            query_ref: demand.query_ref.clone(),
            projection_digest: graph().deterministic_digest,
            lost_axis: ConsumerAxis::Temporal,
            left_world_ref: "world:2025".into(),
            right_world_ref: "world:2026".into(),
            shared_projection_ref: "projection:time-erased".into(),
            left_answer_ref: "answer:old-law".into(),
            right_answer_ref: "answer:new-law".into(),
            theorem_module_ref: "DASHI.Law.LegalWorldRevisionReconstructionExact".into(),
            theorem_ref: "temporalProjectionDefect".into(),
            theorem_artifact_digest:
                "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    .into(),
            exact_fibre_collision: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let result = compile_consumer_adequacy(
            &demand,
            &graph(),
            &ConsumerCoverage::default(),
            OperationalResearchState::CurrentFrontierClosed,
            None,
            &[defect],
        )
        .unwrap();

        let ConsumerAdequacyCompilation::NeedsResearch(compiled) = result else {
            panic!("lost temporal axis must reopen research");
        };
        assert!(compiled.operational_frontier_was_closed);
        assert!(compiled.reopened_by_nonfactorability);
        assert_eq!(compiled.exact_residuals.len(), 1);
        assert_eq!(compiled.exact_residuals[0].lost_axis, ConsumerAxis::Temporal);
        assert_eq!(
            compiled.exact_residuals[0].demand.kind,
            ConsumerResearchDemandKind::ResolveTemporalCoordinate
        );
        assert!(!compiled.creates_claim_truth);
    }
}
