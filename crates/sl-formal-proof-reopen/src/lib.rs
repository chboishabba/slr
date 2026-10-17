//! Adapter from exact formal-evidence payment into the existing SLR sparse-reopen engine.
//!
//! This crate owns no new dependency graph.  It projects an accepted formal
//! evidence receipt to the already-existing `Evidence` source-coordinate kind
//! and delegates wake selection to `sensiblaw_evidential_reopen::sparse_wake`.

use sensiblaw_evidential_reopen::{
    sparse_wake, ReverseDependency, SourceCoordinate, SourceCoordinateKind, WakeRequest,
};
use sensiblaw_formal_proof_bridge::FormalEvidencePayment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalEvidenceReopenPlan {
    pub changed: SourceCoordinate,
    pub wakes: Vec<WakeRequest>,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

/// Reuse the canonical reverse-dependency engine for a newly accepted formal
/// evidence payment.  Missing dependency means zero work; unrelated fibres stay
/// closed.  The adapter does not reinterpret the proof receipt or widen payment.
pub fn plan_formal_evidence_reopen(
    payment: &FormalEvidencePayment,
    dependencies: &[ReverseDependency],
) -> FormalEvidenceReopenPlan {
    let changed = SourceCoordinate {
        kind: SourceCoordinateKind::Evidence,
        stable_ref: payment.evidence_ref.clone(),
    };
    let wakes = sparse_wake(std::slice::from_ref(&changed), dependencies);
    FormalEvidenceReopenPlan {
        changed,
        wakes,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_evidential_reopen::{ConsumerFibreKey, ReverseDependency};

    fn payment() -> FormalEvidencePayment {
        FormalEvidencePayment {
            demand_ref: "formal-demand:1".into(),
            obligation_ref: "proof-obligation:1".into(),
            evidence_ref: "formal-evidence:1".into(),
            statement_digest: "sha256:statement-a".into(),
            formal_premise_paid: true,
            reopen_refs: vec!["residual:declared".into()],
            creates_legal_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn fibre(demand: &str, consumer: &str) -> ConsumerFibreKey {
        ConsumerFibreKey {
            demand_ref: demand.into(),
            consumer_ref: consumer.into(),
            query_ref: "query:1".into(),
            policy_ref: "policy:1".into(),
        }
    }

    #[test]
    fn missing_dependency_means_zero_work() {
        let plan = plan_formal_evidence_reopen(&payment(), &[]);
        assert!(plan.wakes.is_empty());
        assert_eq!(plan.changed.kind, SourceCoordinateKind::Evidence);
        assert_eq!(plan.changed.stable_ref, "formal-evidence:1");
    }

    #[test]
    fn only_explicit_formal_evidence_dependency_reopens() {
        let interested = fibre("formal-demand:1", "consumer:formal-premise");
        let unrelated = fibre("other-demand", "consumer:remedy");
        let deps = vec![
            ReverseDependency {
                source: SourceCoordinate {
                    kind: SourceCoordinateKind::Evidence,
                    stable_ref: "formal-evidence:1".into(),
                },
                consumer: interested.clone(),
                minimum_horizon: 6,
            },
            ReverseDependency {
                source: SourceCoordinate {
                    kind: SourceCoordinateKind::Evidence,
                    stable_ref: "other-evidence".into(),
                },
                consumer: unrelated,
                minimum_horizon: 6,
            },
        ];
        let plan = plan_formal_evidence_reopen(&payment(), &deps);
        assert_eq!(plan.wakes.len(), 1);
        assert_eq!(plan.wakes[0].consumer, interested);
        assert!(!plan.creates_semantic_authority);
        assert!(!plan.creates_legal_authority);
        assert!(!plan.applicability_promoted);
        assert!(!plan.claim_truth_promoted);
    }
}
