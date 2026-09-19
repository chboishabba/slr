//! Compile the already-persisted flagship Mabo proposition state into
//! heterogeneous adaptive research residuals.
//!
//! This layer does not reinterpret graph adjacency as legal semantics. It uses
//! the existing proposition-payment ABI and exact persisted proposition/span
//! weld. Explicit qualifier/defeater/comparator residuals remain valid bounded
//! Why coverage while also remaining researchable debts for the adaptive loop.

use sensiblaw_evidence_payment::{
    evaluate_proposition_chain_payment, PropositionEvidenceObservation,
    PropositionProofRole, PropositionRoleResidual,
};
use sensiblaw_pg_source_store::{PropositionObservationRow, PropositionRows};
use sensiblaw_proof_search_loop::frontier::{ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::world_expansion::{ProducerLane, ResidualClass};
use thiserror::Error;

use crate::adaptive_campaign::TypedAdaptiveResidualMove;

pub const MABO_RADICAL_TITLE_PROPOSITION: &str =
    "mabo:proposition:radical-title-native-title";
pub const MABO_RADICAL_TITLE_SPAN: &str =
    "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";
pub const MABO_RADICAL_TITLE_OALC_SOURCE: &str = "case:[1992]-HCA-23";

fn support_observation(row: &PropositionObservationRow) -> PropositionEvidenceObservation {
    PropositionEvidenceObservation {
        observation_ref: row.observation_ref.clone(),
        pnf_factor_ref: row.pnf_factor_ref.clone(),
        pnf_revision_ref: row.pnf_revision_ref.clone(),
        role: PropositionProofRole::Support,
        observation_provenance_refs: row.observation_provenance_refs.clone(),
        graph_source_span_refs: row.graph_source_span_refs.clone(),
        residual_refs: row.residual_refs.clone(),
    }
}

fn role_residual(role: PropositionProofRole, reference: &str) -> PropositionRoleResidual {
    PropositionRoleResidual {
        role,
        residual_ref: reference.into(),
    }
}

fn legal_move(
    residual_ref: impl Into<String>,
    proposition_ref: &str,
    authority_requirement: Option<&str>,
    move_ref: impl Into<String>,
    operation: impl Into<String>,
    reduction: u64,
    diagnosis_reference: impl Into<String>,
) -> TypedAdaptiveResidualMove {
    let residual_ref = residual_ref.into();
    TypedAdaptiveResidualMove {
        residual: ProofResidual {
            residual_ref,
            proposition_ref: proposition_ref.into(),
            producer_class_ref: "producer:governed-legal".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: authority_requirement.map(str::to_owned),
            salience: reduction,
            dependency_refs: vec![MABO_RADICAL_TITLE_SPAN.into()],
            status: ResidualStatus::Open,
        },
        residual_class: ResidualClass::Legal,
        producer_lane: ProducerLane::GovernedLegal,
        move_ref: move_ref.into(),
        source_ref: Some(MABO_RADICAL_TITLE_OALC_SOURCE.into()),
        provider_operation_ref: operation.into(),
        expected_whole_frontier_reduction: reduction,
        shared_dependency_gain: 1,
        network_requests: 1,
        operator_review_cost: 1,
        admissible: true,
        diagnosis_reference: diagnosis_reference.into(),
    }
}

fn provenance_move(
    residual_ref: impl Into<String>,
    proposition_ref: &str,
    observation_ref: &str,
) -> TypedAdaptiveResidualMove {
    let residual_ref = residual_ref.into();
    TypedAdaptiveResidualMove {
        residual: ProofResidual {
            residual_ref: residual_ref.clone(),
            proposition_ref: proposition_ref.into(),
            producer_class_ref: "producer:source-specific-provenance".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: None,
            salience: 1,
            dependency_refs: vec![observation_ref.into(), MABO_RADICAL_TITLE_SPAN.into()],
            status: ResidualStatus::Open,
        },
        residual_class: ResidualClass::Provenance,
        producer_lane: ProducerLane::SourceSpecificProvenance,
        move_ref: format!("move:mabo:provenance:{residual_ref}"),
        source_ref: Some(observation_ref.into()),
        provider_operation_ref: "legal-ir:source-specific-provenance-follow".into(),
        expected_whole_frontier_reduction: 1,
        shared_dependency_gain: 1,
        network_requests: 0,
        operator_review_cost: 1,
        admissible: true,
        diagnosis_reference: format!("diagnosis:mabo:pnf-residual:{observation_ref}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboPropositionResearchDiagnosis {
    pub moves: Vec<TypedAdaptiveResidualMove>,
    pub proposition_chain_paid: bool,
    pub why_executable: bool,
    pub applicability_paid: bool,
    pub claim_truth_paid: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MaboPropositionResearchDiagnosisError {
    #[error("unexpected Mabo proposition coordinate: {0}")]
    PropositionMismatch(String),
    #[error("unexpected Mabo exact span coordinate: {0}")]
    SpanMismatch(String),
}

pub fn diagnose_mabo_proposition_research(
    rows: &PropositionRows,
) -> Result<MaboPropositionResearchDiagnosis, MaboPropositionResearchDiagnosisError> {
    if rows.proposition_ref != MABO_RADICAL_TITLE_PROPOSITION {
        return Err(MaboPropositionResearchDiagnosisError::PropositionMismatch(
            rows.proposition_ref.clone(),
        ));
    }
    if rows.required_span_ref != MABO_RADICAL_TITLE_SPAN {
        return Err(MaboPropositionResearchDiagnosisError::SpanMismatch(
            rows.required_span_ref.clone(),
        ));
    }

    let observations = rows
        .observations
        .iter()
        .map(support_observation)
        .collect::<Vec<_>>();
    let explicit_role_residuals = vec![
        role_residual(
            PropositionProofRole::Qualifier,
            "mabo:residual:qualifier-unpaid",
        ),
        role_residual(
            PropositionProofRole::Defeater,
            "mabo:residual:defeater-unpaid",
        ),
        role_residual(
            PropositionProofRole::Comparator,
            "mabo:residual:comparator-unpaid",
        ),
    ];
    let chain = evaluate_proposition_chain_payment(
        &rows.proposition_ref,
        &rows.required_span_ref,
        rows.exact_source_paid,
        &observations,
        &explicit_role_residuals,
    );

    let mut moves = Vec::new();

    for residual_ref in &chain.residual_refs {
        match residual_ref.as_str() {
            "reader-residual:exact-authority-span" => moves.push(legal_move(
                residual_ref.clone(),
                &rows.proposition_ref,
                Some("primary-legal-source"),
                "move:mabo:oalc:exact-authority-span",
                "oalc:exact-mnc:[1992]-HCA-23",
                4,
                "diagnosis:mabo:missing-exact-authority-span",
            )),
            "reader-residual:proposition-support" => moves.push(legal_move(
                residual_ref.clone(),
                &rows.proposition_ref,
                Some("reviewed-proposition-support"),
                "move:mabo:proposition-support",
                "legal-ir:reviewed-support-acquisition",
                4,
                "diagnosis:mabo:missing-proposition-support",
            )),
            "reader-residual:source-provenance-weld" => moves.push(
                provenance_move(
                    residual_ref.clone(),
                    &rows.proposition_ref,
                    rows.observations
                        .first()
                        .map(|row| row.observation_ref.as_str())
                        .unwrap_or("mabo:observation:missing-support-weld"),
                ),
            ),
            _ => moves.push(provenance_move(
                residual_ref.clone(),
                &rows.proposition_ref,
                "mabo:reader-chain",
            )),
        }
    }

    // These retained roles already count as coverage for bounded Why, but are
    // deliberately still researchable. Research may replace an explicit
    // residual with a reviewed observation without changing the narrower
    // reader contract or promoting applicability/truth.
    for (role, residual_ref, suffix) in [
        (
            PropositionProofRole::Qualifier,
            "mabo:residual:qualifier-unpaid",
            "qualifier",
        ),
        (
            PropositionProofRole::Defeater,
            "mabo:residual:defeater-unpaid",
            "defeater",
        ),
        (
            PropositionProofRole::Comparator,
            "mabo:residual:comparator-unpaid",
            "comparator",
        ),
    ] {
        let _ = role;
        moves.push(legal_move(
            residual_ref,
            &rows.proposition_ref,
            Some("primary-legal-source"),
            format!("move:mabo:oalc:{suffix}"),
            format!("governed-legal:proposition-role:{suffix}"),
            1,
            format!("diagnosis:mabo:explicit-{suffix}-residual"),
        ));
    }

    // Reviewed PNF observations may carry additional explicit residual
    // coordinates. Preserve them as provenance-class research demands rather
    // than silently interpreting their names as payments or truth.
    for observation in &rows.observations {
        for residual_ref in &observation.residual_refs {
            moves.push(provenance_move(
                residual_ref.clone(),
                &rows.proposition_ref,
                &observation.observation_ref,
            ));
        }
    }

    moves.sort_by(|left, right| {
        left.residual
            .residual_ref
            .cmp(&right.residual.residual_ref)
            .then_with(|| left.move_ref.cmp(&right.move_ref))
    });
    moves.dedup_by(|left, right| {
        left.residual.residual_ref == right.residual.residual_ref
            && left.move_ref == right.move_ref
    });

    Ok(MaboPropositionResearchDiagnosis {
        moves,
        proposition_chain_paid: chain.proposition_chain_paid,
        why_executable: chain.why_executable,
        applicability_paid: chain.applicability_paid,
        claim_truth_paid: chain.claim_truth_paid,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
