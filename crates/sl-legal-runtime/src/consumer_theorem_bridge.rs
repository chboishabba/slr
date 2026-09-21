//! Theorem-bearing bridge for consumer adequacy.
//!
//! Runtime coordinate coverage is useful for routing but is not a proof of
//! query-indexed factorisation.  This module is the only legal-runtime surface
//! allowed to turn a coordinate-complete adequacy candidate into a
//! theorem-backed ConsumerAdequate receipt.  The promotion requires an explicit
//! external formal witness reference bound to the exact query and projection
//! digest.
//!
//! Conversely, exact non-factorability witnesses compile constructively into
//! typed research demands with provenance for the lost coordinate.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ConsumerAdequacyDisposition, ConsumerAdequacyReceipt, ConsumerAxis,
    ConsumerResearchDemand, ConsumerResearchDemandKind, ProjectionGraph,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactorsThroughWitnessReceipt {
    pub query_ref: String,
    pub projection_digest: String,
    pub theorem_module_ref: String,
    pub theorem_ref: String,
    pub theorem_artifact_digest: String,
    pub exact_query_indexed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

/// External typechecker receipt.  This is intentionally not itself accepted by
/// \`certify_consumer_adequacy\`; it must first pass
/// \`kernel_checked_factors_through_witness\`, which returns an opaque witness.
///
/// The receipt says that the named Agda owner containing the exact query-indexed
/// FactorsThrough theorem was type-checked successfully against the source
/// artifact digest recorded here.  The theorem name remains attribution;
/// successful kernel/typechecker verification is the promotion gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgdaFactorsThroughTypecheckReceipt {
    pub schema_version: String,
    pub verifier: String,
    pub command_ref: String,
    pub exit_code: i32,
    pub query_ref: String,
    pub projection_digest: String,
    pub theorem_module_ref: String,
    pub theorem_ref: String,
    pub theorem_artifact_digest: String,
    pub exact_query_indexed: bool,
    pub factors_through_claim: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

/// Opaque runtime capability created only by validating a successful formal
/// typecheck receipt.  Callers cannot construct the proof-bearing value through
/// a public struct literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelCheckedFactorsThroughWitness {
    metadata: FactorsThroughWitnessReceipt,
    verifier: String,
    command_ref: String,
    verification_receipt_digest: String,
}

impl KernelCheckedFactorsThroughWitness {
    #[must_use]
    pub fn metadata(&self) -> &FactorsThroughWitnessReceipt {
        &self.metadata
    }

    #[must_use]
    pub fn verification_receipt_digest(&self) -> &str {
        &self.verification_receipt_digest
    }

    #[must_use]
    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    #[must_use]
    pub fn command_ref(&self) -> &str {
        &self.command_ref
    }
}

fn valid_sha256_ref(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn digest_parts(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    format!(
        "sha256:{}",
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub fn kernel_checked_factors_through_witness(
    receipt: &AgdaFactorsThroughTypecheckReceipt,
) -> Result<KernelCheckedFactorsThroughWitness, String> {
    if receipt.schema_version != "sl.formal.agda_factors_through_typecheck.v0_1" {
        return Err("unsupported FactorsThrough typecheck receipt schema".into());
    }
    if receipt.verifier != "agda" {
        return Err("FactorsThrough formal receipt must be verified by Agda".into());
    }
    if receipt.exit_code != 0 {
        return Err("FactorsThrough Agda typecheck did not exit successfully".into());
    }
    if receipt.command_ref.trim().is_empty()
        || receipt.query_ref.trim().is_empty()
        || receipt.projection_digest.trim().is_empty()
        || receipt.theorem_module_ref.trim().is_empty()
        || receipt.theorem_ref.trim().is_empty()
        || !valid_sha256_ref(&receipt.theorem_artifact_digest)
    {
        return Err("FactorsThrough formal receipt has incomplete coordinates".into());
    }
    if !receipt.exact_query_indexed || !receipt.factors_through_claim {
        return Err("FactorsThrough formal receipt is not an exact query-indexed claim".into());
    }
    if !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.creates_claim_truth
    {
        return Err("FactorsThrough formal receipt crossed non-promotion boundary".into());
    }

    let exit_code = receipt.exit_code.to_string();
    let verification_receipt_digest = digest_parts(&[
        receipt.schema_version.as_str(),
        receipt.verifier.as_str(),
        receipt.command_ref.as_str(),
        exit_code.as_str(),
        receipt.query_ref.as_str(),
        receipt.projection_digest.as_str(),
        receipt.theorem_module_ref.as_str(),
        receipt.theorem_ref.as_str(),
        receipt.theorem_artifact_digest.as_str(),
        "exact-query-indexed=true",
        "factors-through=true",
    ]);

    Ok(KernelCheckedFactorsThroughWitness {
        metadata: FactorsThroughWitnessReceipt {
            query_ref: receipt.query_ref.clone(),
            projection_digest: receipt.projection_digest.clone(),
            theorem_module_ref: receipt.theorem_module_ref.clone(),
            theorem_ref: receipt.theorem_ref.clone(),
            theorem_artifact_digest: receipt.theorem_artifact_digest.clone(),
            exact_query_indexed: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        verifier: receipt.verifier.clone(),
        command_ref: receipt.command_ref.clone(),
        verification_receipt_digest,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TheoremBackedConsumerAdequacyReceipt {
    pub query_ref: String,
    pub projection_digest: String,
    pub theorem_module_ref: String,
    pub theorem_ref: String,
    pub theorem_artifact_digest: String,
    pub formal_verifier: String,
    pub formal_command_ref: String,
    pub verification_receipt_digest: String,
    pub runtime_receipt: ConsumerAdequacyReceipt,
    pub factors_through_formally_proved: bool,
    pub consumer_adequate: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn certify_consumer_adequacy(
    runtime: &ConsumerAdequacyReceipt,
    graph: &ProjectionGraph,
    checked: &KernelCheckedFactorsThroughWitness,
) -> Result<TheoremBackedConsumerAdequacyReceipt, String> {
    let witness = checked.metadata();
    if runtime.disposition != ConsumerAdequacyDisposition::Adequate
        || !runtime.missing_axes.is_empty()
        || !runtime.missing_semantic_refs.is_empty()
        || !runtime.research_demands.is_empty()
    {
        return Err(
            "formal consumer adequacy requires a coordinate-complete runtime candidate".into(),
        );
    }
    if runtime.query_ref != witness.query_ref {
        return Err("formal adequacy witness query_ref does not match runtime query".into());
    }
    if graph.deterministic_digest != witness.projection_digest {
        return Err("formal adequacy witness projection digest does not match runtime projection".into());
    }
    if witness.theorem_module_ref.trim().is_empty()
        || witness.theorem_ref.trim().is_empty()
        || !valid_sha256_ref(&witness.theorem_artifact_digest)
    {
        return Err("formal adequacy witness must name theorem module, theorem and artifact digest".into());
    }
    if !witness.exact_query_indexed {
        return Err("formal adequacy witness must be exact and query-indexed".into());
    }
    if !witness.candidate_only
        || witness.creates_semantic_authority
        || witness.creates_claim_truth
        || !runtime.candidate_only
        || runtime.creates_semantic_authority
        || runtime.creates_claim_truth
        || !graph.projection_only
        || graph.creates_semantic_authority
    {
        return Err("formal adequacy witness crossed non-promotion boundary".into());
    }

    let mut runtime_receipt = runtime.clone();
    runtime_receipt.factors_through_formally_proved = true;

    Ok(TheoremBackedConsumerAdequacyReceipt {
        query_ref: runtime.query_ref.clone(),
        projection_digest: graph.deterministic_digest.clone(),
        theorem_module_ref: witness.theorem_module_ref.clone(),
        theorem_ref: witness.theorem_ref.clone(),
        theorem_artifact_digest: witness.theorem_artifact_digest.clone(),
        formal_verifier: checked.verifier().to_owned(),
        formal_command_ref: checked.command_ref().to_owned(),
        verification_receipt_digest: checked.verification_receipt_digest().to_owned(),
        runtime_receipt,
        factors_through_formally_proved: true,
        consumer_adequate: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonFactorabilityWitnessReceipt {
    pub query_ref: String,
    pub projection_digest: String,
    pub lost_axis: ConsumerAxis,
    pub left_world_ref: String,
    pub right_world_ref: String,
    pub shared_projection_ref: String,
    pub left_answer_ref: String,
    pub right_answer_ref: String,
    pub theorem_module_ref: String,
    pub theorem_ref: String,
    pub theorem_artifact_digest: String,
    pub exact_fibre_collision: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactConsumerResidual {
    pub residual_ref: String,
    pub query_ref: String,
    pub projection_digest: String,
    pub lost_axis: ConsumerAxis,
    pub demand: ConsumerResearchDemand,
    pub nonfactorability_theorem_ref: String,
    pub witness_refs: Vec<String>,
    pub reason_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn demand_kind(axis: ConsumerAxis) -> ConsumerResearchDemandKind {
    match axis {
        ConsumerAxis::SemanticIdentity => ConsumerResearchDemandKind::ResolveSemanticIdentity,
        ConsumerAxis::SourceRevision | ConsumerAxis::SourceSpan => {
            ConsumerResearchDemandKind::AcquireSource
        }
        ConsumerAxis::Provenance => ConsumerResearchDemandKind::RecoverProvenance,
        ConsumerAxis::Treatment => ConsumerResearchDemandKind::ReviewTreatment,
        ConsumerAxis::Temporal => ConsumerResearchDemandKind::ResolveTemporalCoordinate,
        ConsumerAxis::Jurisdiction => ConsumerResearchDemandKind::ResolveJurisdiction,
        ConsumerAxis::FactualPredicate => ConsumerResearchDemandKind::ReviewFact,
        ConsumerAxis::BurdenOrException => {
            ConsumerResearchDemandKind::ReviewBurdenOrException
        }
    }
}

fn axis_slug(axis: ConsumerAxis) -> &'static str {
    match axis {
        ConsumerAxis::SemanticIdentity => "semantic-identity",
        ConsumerAxis::SourceRevision => "source-revision",
        ConsumerAxis::SourceSpan => "source-span",
        ConsumerAxis::Provenance => "provenance",
        ConsumerAxis::Treatment => "treatment",
        ConsumerAxis::Temporal => "temporal",
        ConsumerAxis::Jurisdiction => "jurisdiction",
        ConsumerAxis::FactualPredicate => "factual-predicate",
        ConsumerAxis::BurdenOrException => "burden-or-exception",
    }
}

pub fn compile_nonfactorability_residual(
    witness: &NonFactorabilityWitnessReceipt,
    target_ref: impl Into<String>,
) -> Result<ExactConsumerResidual, String> {
    if witness.query_ref.trim().is_empty()
        || witness.projection_digest.trim().is_empty()
        || witness.left_world_ref.trim().is_empty()
        || witness.right_world_ref.trim().is_empty()
        || witness.shared_projection_ref.trim().is_empty()
        || witness.left_answer_ref.trim().is_empty()
        || witness.right_answer_ref.trim().is_empty()
        || witness.theorem_ref.trim().is_empty()
        || witness.theorem_module_ref.trim().is_empty()
        || witness.theorem_artifact_digest.trim().is_empty()
    {
        return Err("nonfactorability witness is incomplete".into());
    }
    if witness.left_world_ref == witness.right_world_ref {
        return Err("nonfactorability witness must compare distinct worlds".into());
    }
    if witness.left_answer_ref == witness.right_answer_ref {
        return Err("nonfactorability witness must expose distinct consumer answers".into());
    }
    if !witness.exact_fibre_collision {
        return Err("nonfactorability residual requires an exact fibre collision".into());
    }
    if !witness.candidate_only
        || witness.creates_semantic_authority
        || witness.creates_claim_truth
    {
        return Err("nonfactorability witness crossed non-promotion boundary".into());
    }

    let target_ref = target_ref.into();
    if target_ref.trim().is_empty() {
        return Err("nonfactorability residual target_ref must be non-empty".into());
    }
    let residual_ref = format!(
        "consumer-residual:{}:{}:{}",
        witness.query_ref,
        axis_slug(witness.lost_axis),
        witness.theorem_artifact_digest
    );
    let reason_ref = format!(
        "nonfactorability:{}:{}",
        witness.theorem_ref,
        axis_slug(witness.lost_axis)
    );

    Ok(ExactConsumerResidual {
        residual_ref,
        query_ref: witness.query_ref.clone(),
        projection_digest: witness.projection_digest.clone(),
        lost_axis: witness.lost_axis,
        demand: ConsumerResearchDemand {
            axis: witness.lost_axis,
            kind: demand_kind(witness.lost_axis),
            target_ref,
            reason_ref: reason_ref.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        nonfactorability_theorem_ref: witness.theorem_ref.clone(),
        witness_refs: vec![
            witness.left_world_ref.clone(),
            witness.right_world_ref.clone(),
            witness.shared_projection_ref.clone(),
            witness.left_answer_ref.clone(),
            witness.right_answer_ref.clone(),
        ],
        reason_ref,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use crate::{ProjectionKind, ProjectionNode};

    fn graph() -> ProjectionGraph {
        ProjectionGraph {
            kind: ProjectionKind::IssueProof,
            nodes: vec![ProjectionNode {
                semantic_ref: "case:fixture".into(),
                semantic_kind: "CaseAuthority".into(),
                manifestation_refs: vec![],
                source_revision_refs: vec!["revision:fixture".into()],
                span_refs: vec!["span:fixture".into()],
                projection_role: "IssueProof".into(),
            }],
            edges: vec![],
            deterministic_digest: "sha256:projection".into(),
            projection_only: true,
            creates_semantic_authority: false,
        }
    }

    fn runtime() -> ConsumerAdequacyReceipt {
        ConsumerAdequacyReceipt {
            query_ref: "query:fixture".into(),
            disposition: ConsumerAdequacyDisposition::Adequate,
            paid_axes: BTreeSet::from([
                ConsumerAxis::SemanticIdentity,
                ConsumerAxis::SourceRevision,
                ConsumerAxis::SourceSpan,
            ]),
            missing_axes: BTreeSet::new(),
            missing_semantic_refs: BTreeSet::new(),
            research_demands: vec![],
            unresolved_refs: BTreeSet::new(),
            factors_through_formally_proved: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn formal_receipt(exact: bool, exit_code: i32) -> AgdaFactorsThroughTypecheckReceipt {
        AgdaFactorsThroughTypecheckReceipt {
            schema_version: "sl.formal.agda_factors_through_typecheck.v0_1".into(),
            verifier: "agda".into(),
            command_ref: "agda -i . DASHI/Law/Fixture.agda".into(),
            exit_code,
            query_ref: "query:fixture".into(),
            projection_digest: "sha256:projection".into(),
            theorem_module_ref: "DASHI.Law.ConsumerDirectedLegalFollowAdequacyExact".into(),
            theorem_ref: "fixtureFactorsThrough".into(),
            theorem_artifact_digest:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            exact_query_indexed: exact,
            factors_through_claim: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    #[test]
    fn theorem_name_alone_cannot_promote_runtime_adequacy() {
        let unchecked = FactorsThroughWitnessReceipt {
            query_ref: "query:fixture".into(),
            projection_digest: "sha256:projection".into(),
            theorem_module_ref: "DASHI.Law.ConsumerDirectedLegalFollowAdequacyExact".into(),
            theorem_ref: "fixtureFactorsThrough".into(),
            theorem_artifact_digest:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            exact_query_indexed: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        // No public certification function accepts this metadata-only value.
        assert_eq!(unchecked.theorem_ref, "fixtureFactorsThrough");

        assert!(kernel_checked_factors_through_witness(&formal_receipt(false, 0)).is_err());
        assert!(kernel_checked_factors_through_witness(&formal_receipt(true, 1)).is_err());

        let checked =
            kernel_checked_factors_through_witness(&formal_receipt(true, 0)).unwrap();
        let certified = certify_consumer_adequacy(&runtime(), &graph(), &checked).unwrap();
        assert!(certified.consumer_adequate);
        assert!(certified.factors_through_formally_proved);
        assert!(certified.runtime_receipt.factors_through_formally_proved);
        assert_eq!(certified.formal_verifier, "agda");
        assert!(valid_sha256_ref(&certified.verification_receipt_digest));
        assert!(!certified.creates_semantic_authority);
        assert!(!certified.creates_claim_truth);
    }

    #[test]
    fn exact_collision_compiles_to_exact_treatment_residual() {
        let witness = NonFactorabilityWitnessReceipt {
            query_ref: "query:treatment".into(),
            projection_digest: "sha256:projection".into(),
            lost_axis: ConsumerAxis::Treatment,
            left_world_ref: "world:treatment-missing".into(),
            right_world_ref: "world:treatment-paid".into(),
            shared_projection_ref: "projection:same-surface".into(),
            left_answer_ref: "answer:unresolved".into(),
            right_answer_ref: "answer:resolved".into(),
            theorem_module_ref: "DASHI.Law.ConsumerDirectedLegalFollowAdequacyExact".into(),
            theorem_ref: "coarseTreatmentDefect".into(),
            theorem_artifact_digest: "sha256:collision".into(),
            exact_fibre_collision: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let residual =
            compile_nonfactorability_residual(&witness, "case:target").unwrap();
        assert_eq!(residual.lost_axis, ConsumerAxis::Treatment);
        assert_eq!(
            residual.demand.kind,
            ConsumerResearchDemandKind::ReviewTreatment
        );
        assert!(residual
            .witness_refs
            .contains(&"world:treatment-missing".into()));
        assert!(!residual.creates_semantic_authority);
    }
}