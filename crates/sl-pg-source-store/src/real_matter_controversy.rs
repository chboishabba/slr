//! Strict REAL-MATTER adversarial controversy materialiser.
//!
//! The caller selects existing reviewed evidence and claim coordinates. Source,
//! provider, normative-order and review ancestry are reopened from PostgreSQL.
//! The output is a challengeable controversy structure plus reverse-search
//! receipt; no merits/adjudicative result is produced.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    load_legal_controversy_matter, load_legal_controversy_residual,
    load_legal_proof_obligation, load_reviewed_evidence_coordinate,
    persist_legal_controversy_matter, persist_legal_controversy_residual,
    persist_legal_proof_obligation, persist_legal_proposition_fibre,
    persist_reverse_legal_proof_search, persist_typed_response_edge, DatabaseConfig,
    DisagreementKind, EpistemicStatus, LegalControversyMatter, LegalControversyResidual,
    LegalControversyStoreError, LegalProofObligation, LegalPropositionFibre, LegalRole,
    ObligationKind, PartyRole, ProceduralGoal, ResponseMode, ReverseLegalProofSearch,
    ReviewedEvidenceStoreError, TypedResponseEdge,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealMatterControversyDraft {
    pub matter_ref: String,
    pub applicant_claim_ref: String,
    pub applicant_reviewed_evidence_ref: String,
    pub applicant_normative_order_ref: String,
    pub respondent_claim_ref: String,
    pub respondent_reviewed_evidence_ref: String,
    pub respondent_normative_order_ref: String,
    pub response_mode: ResponseMode,
    pub disagreement_kind: DisagreementKind,
    pub unresolved_question: String,
    pub requested_discriminator: String,
    pub target_evidence_query: String,
    pub procedural_goal: ProceduralGoal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealMatterControversyReceipt {
    pub matter_ref: String,
    pub controversy_ref: String,
    pub applicant_fibre_ref: String,
    pub respondent_fibre_ref: String,
    pub response_ref: String,
    pub residual_ref: String,
    pub obligation_ref: String,
    pub reverse_ref: String,
    pub applicant_normative_order_ref: String,
    pub respondent_normative_order_ref: String,
    pub response_mode: ResponseMode,
    pub disagreement_kind: DisagreementKind,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum RealMatterControversyError {
    #[error("required real-matter controversy coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error(transparent)]
    ReviewedEvidence(#[from] ReviewedEvidenceStoreError),
    #[error(transparent)]
    Controversy(#[from] LegalControversyStoreError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("reviewed evidence does not match the declared normative order")]
    NormativeOrderMismatch,
    #[error("reviewed evidence has no matching provider-backed legal source manifestation")]
    MissingProviderManifestation,
    #[error("claim coordinate is not persisted")]
    MissingClaim,
    #[error("controversy has no open residual with a discriminator/query target")]
    NoOpenResidual,
}

pub fn materialize_real_matter_controversy(
    config: &DatabaseConfig,
    draft: &RealMatterControversyDraft,
) -> Result<RealMatterControversyReceipt, RealMatterControversyError> {
    validate_draft(draft)?;
    let applicant = load_reviewed_evidence_coordinate(config, &draft.applicant_reviewed_evidence_ref)?;
    let respondent = load_reviewed_evidence_coordinate(config, &draft.respondent_reviewed_evidence_ref)?;
    if applicant.normative_order_ref != draft.applicant_normative_order_ref
        || respondent.normative_order_ref != draft.respondent_normative_order_ref
    {
        return Err(RealMatterControversyError::NormativeOrderMismatch);
    }
    require_provider_manifestation(config, &applicant.source_revision_ref, &applicant.document_ref)?;
    require_provider_manifestation(config, &respondent.source_revision_ref, &respondent.document_ref)?;
    require_claim(config, &draft.applicant_claim_ref)?;
    require_claim(config, &draft.respondent_claim_ref)?;

    let controversy_ref = stable_ref("legal-controversy", &[
        &draft.matter_ref,
        &applicant.reviewed_evidence_ref,
        &respondent.reviewed_evidence_ref,
        response_mode_ref(draft.response_mode),
        disagreement_ref(draft.disagreement_kind),
    ]);
    let applicant_fibre_ref = stable_ref("legal-fibre-applicant", &[
        &controversy_ref,
        &draft.applicant_claim_ref,
        &applicant.reviewed_evidence_ref,
    ]);
    let respondent_fibre_ref = stable_ref("legal-fibre-respondent", &[
        &controversy_ref,
        &draft.respondent_claim_ref,
        &respondent.reviewed_evidence_ref,
    ]);

    let applicant_fibre = LegalPropositionFibre {
        fibre_ref: applicant_fibre_ref.clone(),
        claim_ref: draft.applicant_claim_ref.clone(),
        party: PartyRole::Applicant,
        legal_role: LegalRole::LegalProposition,
        epistemic_status: EpistemicStatus::Supported,
        evidence_kind_ref: "source_text".into(),
        source_reference: applicant.exact_span_ref.clone(),
        reviewed_evidence_ref: Some(applicant.reviewed_evidence_ref.clone()),
        normative_order_ref: applicant.normative_order_ref.clone(),
        temporal_reference: applicant.temporal_refs.first().cloned().unwrap_or_else(|| "temporal:unknown".into()),
        relation_reference: applicant.proposition_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let respondent_fibre = LegalPropositionFibre {
        fibre_ref: respondent_fibre_ref.clone(),
        claim_ref: draft.respondent_claim_ref.clone(),
        party: PartyRole::Respondent,
        legal_role: LegalRole::LegalProposition,
        epistemic_status: EpistemicStatus::Supported,
        evidence_kind_ref: "source_text".into(),
        source_reference: respondent.exact_span_ref.clone(),
        reviewed_evidence_ref: Some(respondent.reviewed_evidence_ref.clone()),
        normative_order_ref: respondent.normative_order_ref.clone(),
        temporal_reference: respondent.temporal_refs.first().cloned().unwrap_or_else(|| "temporal:unknown".into()),
        relation_reference: respondent.proposition_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    persist_legal_proposition_fibre(config, &applicant_fibre)?;
    persist_legal_proposition_fibre(config, &respondent_fibre)?;

    let matter = LegalControversyMatter {
        controversy_ref: controversy_ref.clone(),
        matter_ref: draft.matter_ref.clone(),
        root_fibre_ref: applicant_fibre_ref.clone(),
        proposition_fibre_refs: vec![applicant_fibre_ref.clone(), respondent_fibre_ref.clone()],
        response_refs: vec![],
        residual_refs: vec![],
        obligation_refs: vec![],
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    persist_legal_controversy_matter(config, &matter)?;

    let response_ref = stable_ref("legal-response", &[
        &controversy_ref,
        &applicant_fibre_ref,
        &respondent_fibre_ref,
        response_mode_ref(draft.response_mode),
    ]);
    persist_typed_response_edge(config, &TypedResponseEdge {
        response_ref: response_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        target_fibre_ref: applicant_fibre_ref.clone(),
        response_fibre_ref: respondent_fibre_ref.clone(),
        mode: draft.response_mode,
        response_reference: respondent.reviewed_evidence_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })?;

    let residual_ref = stable_ref("legal-residual", &[
        &controversy_ref,
        disagreement_ref(draft.disagreement_kind),
        &draft.unresolved_question,
    ]);
    persist_legal_controversy_residual(config, &LegalControversyResidual {
        residual_ref: residual_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        kind: draft.disagreement_kind,
        applicant_fibre_ref: applicant_fibre_ref.clone(),
        respondent_fibre_ref: respondent_fibre_ref.clone(),
        residual_level_ref: "residual:open".into(),
        unresolved_question: draft.unresolved_question.clone(),
        requested_discriminator: Some(draft.requested_discriminator.clone()),
        target_evidence_query: Some(draft.target_evidence_query.clone()),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })?;

    let obligation_ref = stable_ref("legal-obligation", &[
        &controversy_ref,
        &applicant_fibre_ref,
        &residual_ref,
    ]);
    persist_legal_proof_obligation(config, &LegalProofObligation {
        obligation_ref: obligation_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        proposition_fibre_ref: applicant_fibre_ref.clone(),
        kind: ObligationKind::Discriminator,
        required_by: residual_ref.clone(),
        discharge_reference: None,
        is_open: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })?;

    let reverse = reverse_search_real_matter_controversy(config, &controversy_ref, draft.procedural_goal)?;
    Ok(RealMatterControversyReceipt {
        matter_ref: draft.matter_ref.clone(), controversy_ref, applicant_fibre_ref,
        respondent_fibre_ref, response_ref, residual_ref, obligation_ref,
        reverse_ref: reverse.reverse_ref,
        applicant_normative_order_ref: applicant.normative_order_ref,
        respondent_normative_order_ref: respondent.normative_order_ref,
        response_mode: draft.response_mode, disagreement_kind: draft.disagreement_kind,
        candidate_only: true, creates_semantic_authority: false, creates_legal_authority: false,
        applicability_promoted: false, claim_truth_promoted: false,
    })
}

pub fn reverse_search_real_matter_controversy(
    config: &DatabaseConfig,
    controversy_ref: &str,
    goal: ProceduralGoal,
) -> Result<ReverseLegalProofSearch, RealMatterControversyError> {
    let matter = load_legal_controversy_matter(config, controversy_ref)?;
    let mut open_obligation_refs = Vec::new();
    for obligation_ref in matter.obligation_refs {
        let obligation = load_legal_proof_obligation(config, &obligation_ref)?;
        if obligation.is_open { open_obligation_refs.push(obligation_ref); }
    }
    let mut candidate_residual_refs = Vec::new();
    let mut requested_discriminator = None;
    let mut target_evidence_query = None;
    for residual_ref in matter.residual_refs {
        let residual = load_legal_controversy_residual(config, &residual_ref)?;
        if requested_discriminator.is_none() {
            requested_discriminator = residual.requested_discriminator.clone();
            target_evidence_query = residual.target_evidence_query.clone();
        }
        candidate_residual_refs.push(residual_ref);
    }
    let Some(requested_discriminator) = requested_discriminator else {
        return Err(RealMatterControversyError::NoOpenResidual);
    };
    let Some(target_evidence_query) = target_evidence_query else {
        return Err(RealMatterControversyError::NoOpenResidual);
    };
    let reverse_ref = stable_ref("legal-reverse-search", &[
        controversy_ref,
        goal_ref(goal),
        &requested_discriminator,
        &target_evidence_query,
    ]);
    let reverse = ReverseLegalProofSearch {
        reverse_ref,
        controversy_ref: controversy_ref.to_owned(),
        goal,
        open_obligation_refs,
        candidate_residual_refs,
        requested_discriminator,
        target_evidence_query,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    Ok(persist_reverse_legal_proof_search(config, &reverse)?)
}

fn require_provider_manifestation(
    config: &DatabaseConfig,
    source_revision_ref: &str,
    document_ref: &str,
) -> Result<(), RealMatterControversyError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let exists: bool = client.query_one(
        r#"SELECT EXISTS(
             SELECT 1 FROM source_provenance.legal_source_manifestation
             WHERE source_revision_ref=$1 AND document_ref=$2
               AND candidate_only AND NOT creates_semantic_authority
               AND NOT creates_legal_authority AND NOT applicability_promoted
               AND NOT claim_truth_promoted)"#,
        &[&source_revision_ref, &document_ref],
    )?.get(0);
    if exists { Ok(()) } else { Err(RealMatterControversyError::MissingProviderManifestation) }
}

fn require_claim(config: &DatabaseConfig, claim_ref: &str) -> Result<(), RealMatterControversyError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let exists: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM semantic.claim_leaf WHERE claim_ref=$1)",
        &[&claim_ref],
    )?.get(0);
    if exists { Ok(()) } else { Err(RealMatterControversyError::MissingClaim) }
}

fn validate_draft(draft: &RealMatterControversyDraft) -> Result<(), RealMatterControversyError> {
    for (name, value) in [
        ("matter_ref", draft.matter_ref.as_str()),
        ("applicant_claim_ref", draft.applicant_claim_ref.as_str()),
        ("applicant_reviewed_evidence_ref", draft.applicant_reviewed_evidence_ref.as_str()),
        ("applicant_normative_order_ref", draft.applicant_normative_order_ref.as_str()),
        ("respondent_claim_ref", draft.respondent_claim_ref.as_str()),
        ("respondent_reviewed_evidence_ref", draft.respondent_reviewed_evidence_ref.as_str()),
        ("respondent_normative_order_ref", draft.respondent_normative_order_ref.as_str()),
        ("unresolved_question", draft.unresolved_question.as_str()),
        ("requested_discriminator", draft.requested_discriminator.as_str()),
        ("target_evidence_query", draft.target_evidence_query.as_str()),
    ] {
        if value.trim().is_empty() { return Err(RealMatterControversyError::EmptyCoordinate(name)); }
    }
    Ok(())
}

fn stable_ref(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts { hasher.update((part.len() as u64).to_be_bytes()); hasher.update(part.as_bytes()); }
    format!("{prefix}:sha256:{:x}", hasher.finalize())
}
fn response_mode_ref(value: ResponseMode) -> &'static str {
    match value {
        ResponseMode::DenyOccurrence => "deny_occurrence",
        ResponseMode::AdmitOccurrenceDisputeCharacterisation => "admit_occurrence_dispute_characterisation",
        ResponseMode::AdmitConductAddContext => "admit_conduct_add_context",
        ResponseMode::DisputeCausation => "dispute_causation",
        ResponseMode::ChallengeEvidenceReliability => "challenge_evidence_reliability",
        ResponseMode::OfferAlternativeEvent => "offer_alternative_event",
        ResponseMode::AdmitProposition => "admit_proposition",
    }
}
fn disagreement_ref(value: DisagreementKind) -> &'static str {
    match value {
        DisagreementKind::Node => "node",
        DisagreementKind::Relation => "relation",
        DisagreementKind::Evidence => "evidence",
        DisagreementKind::Characterisation => "characterisation",
        DisagreementKind::Causal => "causal",
        DisagreementKind::LegalConsequence => "legal_consequence",
        DisagreementKind::NormativeOrderMismatch => "normative_order_mismatch",
    }
}
fn goal_ref(value: ProceduralGoal) -> &'static str {
    match value {
        ProceduralGoal::IdentifyCommonGround => "identify_common_ground",
        ProceduralGoal::IsolateResidualControversy => "isolate_residual_controversy",
        ProceduralGoal::DecideEvidenceNeeded => "decide_evidence_needed",
        ProceduralGoal::PrepareForAdjudication => "prepare_for_adjudication",
    }
}
