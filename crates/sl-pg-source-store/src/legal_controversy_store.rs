//! PostgreSQL persistence for the typed legal-controversy layer.
//!
//! The underlying claim/proposition/source/review objects remain owned by their
//! existing stores. This module persists only legal controversy coordinates and
//! reopens them exactly; it does not adjudicate them.

#[path = "../../sl-core/src/legal_controversy.rs"]
mod domain;
pub use domain::*;

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::DatabaseConfig;

pub const LEGAL_CONTROVERSY_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS legal_controversy;

CREATE TABLE IF NOT EXISTS legal_controversy.proposition_fibre (
  fibre_ref TEXT PRIMARY KEY,
  claim_ref TEXT NOT NULL REFERENCES semantic.claim_leaf(claim_ref),
  party_ref TEXT NOT NULL,
  legal_role_ref TEXT NOT NULL,
  epistemic_status_ref TEXT NOT NULL,
  evidence_kind_ref TEXT NOT NULL,
  source_reference TEXT NOT NULL,
  reviewed_evidence_ref TEXT NULL REFERENCES semantic.reviewed_evidence_coordinate(reviewed_evidence_ref),
  normative_order_ref TEXT NOT NULL,
  temporal_reference TEXT NOT NULL,
  relation_reference TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS legal_controversy.matter (
  controversy_ref TEXT PRIMARY KEY,
  matter_ref TEXT NOT NULL,
  root_fibre_ref TEXT NOT NULL REFERENCES legal_controversy.proposition_fibre(fibre_ref),
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS legal_controversy.matter_fibre (
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref) ON DELETE CASCADE,
  fibre_ref TEXT NOT NULL REFERENCES legal_controversy.proposition_fibre(fibre_ref),
  PRIMARY KEY (controversy_ref, fibre_ref)
);

CREATE TABLE IF NOT EXISTS legal_controversy.response_edge (
  response_ref TEXT PRIMARY KEY,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref) ON DELETE CASCADE,
  target_fibre_ref TEXT NOT NULL REFERENCES legal_controversy.proposition_fibre(fibre_ref),
  response_fibre_ref TEXT NOT NULL REFERENCES legal_controversy.proposition_fibre(fibre_ref),
  mode_ref TEXT NOT NULL,
  response_reference TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS legal_controversy.residual (
  residual_ref TEXT PRIMARY KEY,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref) ON DELETE CASCADE,
  kind_ref TEXT NOT NULL,
  applicant_fibre_ref TEXT NOT NULL REFERENCES legal_controversy.proposition_fibre(fibre_ref),
  respondent_fibre_ref TEXT NOT NULL REFERENCES legal_controversy.proposition_fibre(fibre_ref),
  residual_level_ref TEXT NOT NULL,
  unresolved_question TEXT NOT NULL,
  requested_discriminator TEXT NULL,
  target_evidence_query TEXT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS legal_controversy.proof_obligation (
  obligation_ref TEXT PRIMARY KEY,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref) ON DELETE CASCADE,
  proposition_fibre_ref TEXT NOT NULL REFERENCES legal_controversy.proposition_fibre(fibre_ref),
  kind_ref TEXT NOT NULL,
  required_by TEXT NOT NULL,
  discharge_reference TEXT NULL,
  is_open BOOLEAN NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS legal_controversy.reverse_search (
  reverse_ref TEXT PRIMARY KEY,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref) ON DELETE CASCADE,
  goal_ref TEXT NOT NULL,
  open_obligation_refs TEXT[] NOT NULL,
  candidate_residual_refs TEXT[] NOT NULL,
  requested_discriminator TEXT NOT NULL,
  target_evidence_query TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);
"#;

#[derive(Debug, Error)]
pub enum LegalControversyStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("invalid legal controversy object")]
    InvalidDomainObject,
    #[error("upstream persistence prerequisite failed: {0}")]
    Upstream(String),
    #[error("persisted legal controversy row conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("legal controversy row not found: {0}")]
    NotFound(String),
    #[error("unknown persisted legal controversy enum {kind}: {value}")]
    UnknownEnum { kind: &'static str, value: String },
}

pub fn install_legal_controversy_schema(
    config: &DatabaseConfig,
) -> Result<(), LegalControversyStoreError> {
    crate::install_chronology_contestation_schema(config)
        .map_err(|error| LegalControversyStoreError::Upstream(error.to_string()))?;
    crate::install_reviewed_evidence_schema(config)
        .map_err(|error| LegalControversyStoreError::Upstream(error.to_string()))?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(LEGAL_CONTROVERSY_SCHEMA_SQL)?;
    Ok(())
}

fn party_db(value: PartyRole) -> &'static str {
    match value {
        PartyRole::Applicant => "applicant",
        PartyRole::Respondent => "respondent",
        PartyRole::Court => "court",
        PartyRole::ExternalWitness => "external_witness",
    }
}
fn party_from_db(value: &str) -> Result<PartyRole, LegalControversyStoreError> {
    match value {
        "applicant" => Ok(PartyRole::Applicant),
        "respondent" => Ok(PartyRole::Respondent),
        "court" => Ok(PartyRole::Court),
        "external_witness" => Ok(PartyRole::ExternalWitness),
        _ => Err(LegalControversyStoreError::UnknownEnum { kind: "party", value: value.into() }),
    }
}
fn legal_role_db(value: LegalRole) -> &'static str {
    match value {
        LegalRole::PleadedFact => "pleaded_fact",
        LegalRole::EvidentiaryFact => "evidentiary_fact",
        LegalRole::LegalProposition => "legal_proposition",
        LegalRole::CausalLink => "causal_link",
        LegalRole::RequestedOrder => "requested_order",
    }
}
fn legal_role_from_db(value: &str) -> Result<LegalRole, LegalControversyStoreError> {
    match value {
        "pleaded_fact" => Ok(LegalRole::PleadedFact),
        "evidentiary_fact" => Ok(LegalRole::EvidentiaryFact),
        "legal_proposition" => Ok(LegalRole::LegalProposition),
        "causal_link" => Ok(LegalRole::CausalLink),
        "requested_order" => Ok(LegalRole::RequestedOrder),
        _ => Err(LegalControversyStoreError::UnknownEnum { kind: "legal_role", value: value.into() }),
    }
}
fn epistemic_db(value: EpistemicStatus) -> &'static str {
    match value {
        EpistemicStatus::Alleged => "alleged",
        EpistemicStatus::Admitted => "admitted",
        EpistemicStatus::Disputed => "disputed",
        EpistemicStatus::Supported => "supported",
        EpistemicStatus::Proved => "proved",
        EpistemicStatus::Rejected => "rejected",
        EpistemicStatus::Unresolved => "unresolved",
    }
}
fn epistemic_from_db(value: &str) -> Result<EpistemicStatus, LegalControversyStoreError> {
    match value {
        "alleged" => Ok(EpistemicStatus::Alleged),
        "admitted" => Ok(EpistemicStatus::Admitted),
        "disputed" => Ok(EpistemicStatus::Disputed),
        "supported" => Ok(EpistemicStatus::Supported),
        "proved" => Ok(EpistemicStatus::Proved),
        "rejected" => Ok(EpistemicStatus::Rejected),
        "unresolved" => Ok(EpistemicStatus::Unresolved),
        _ => Err(LegalControversyStoreError::UnknownEnum { kind: "epistemic_status", value: value.into() }),
    }
}
fn response_mode_db(value: ResponseMode) -> &'static str {
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
fn response_mode_from_db(value: &str) -> Result<ResponseMode, LegalControversyStoreError> {
    match value {
        "deny_occurrence" => Ok(ResponseMode::DenyOccurrence),
        "admit_occurrence_dispute_characterisation" => Ok(ResponseMode::AdmitOccurrenceDisputeCharacterisation),
        "admit_conduct_add_context" => Ok(ResponseMode::AdmitConductAddContext),
        "dispute_causation" => Ok(ResponseMode::DisputeCausation),
        "challenge_evidence_reliability" => Ok(ResponseMode::ChallengeEvidenceReliability),
        "offer_alternative_event" => Ok(ResponseMode::OfferAlternativeEvent),
        "admit_proposition" => Ok(ResponseMode::AdmitProposition),
        _ => Err(LegalControversyStoreError::UnknownEnum { kind: "response_mode", value: value.into() }),
    }
}
fn disagreement_db(value: DisagreementKind) -> &'static str {
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
fn disagreement_from_db(value: &str) -> Result<DisagreementKind, LegalControversyStoreError> {
    match value {
        "node" => Ok(DisagreementKind::Node),
        "relation" => Ok(DisagreementKind::Relation),
        "evidence" => Ok(DisagreementKind::Evidence),
        "characterisation" => Ok(DisagreementKind::Characterisation),
        "causal" => Ok(DisagreementKind::Causal),
        "legal_consequence" => Ok(DisagreementKind::LegalConsequence),
        "normative_order_mismatch" => Ok(DisagreementKind::NormativeOrderMismatch),
        _ => Err(LegalControversyStoreError::UnknownEnum { kind: "disagreement", value: value.into() }),
    }
}
fn obligation_db(value: ObligationKind) -> &'static str {
    match value {
        ObligationKind::Premise => "premise",
        ObligationKind::Evidence => "evidence",
        ObligationKind::Response => "response",
        ObligationKind::Discriminator => "discriminator",
        ObligationKind::Adjudicative => "adjudicative",
    }
}
fn obligation_from_db(value: &str) -> Result<ObligationKind, LegalControversyStoreError> {
    match value {
        "premise" => Ok(ObligationKind::Premise),
        "evidence" => Ok(ObligationKind::Evidence),
        "response" => Ok(ObligationKind::Response),
        "discriminator" => Ok(ObligationKind::Discriminator),
        "adjudicative" => Ok(ObligationKind::Adjudicative),
        _ => Err(LegalControversyStoreError::UnknownEnum { kind: "obligation", value: value.into() }),
    }
}
fn goal_db(value: ProceduralGoal) -> &'static str {
    match value {
        ProceduralGoal::IdentifyCommonGround => "identify_common_ground",
        ProceduralGoal::IsolateResidualControversy => "isolate_residual_controversy",
        ProceduralGoal::DecideEvidenceNeeded => "decide_evidence_needed",
        ProceduralGoal::PrepareForAdjudication => "prepare_for_adjudication",
    }
}
fn goal_from_db(value: &str) -> Result<ProceduralGoal, LegalControversyStoreError> {
    match value {
        "identify_common_ground" => Ok(ProceduralGoal::IdentifyCommonGround),
        "isolate_residual_controversy" => Ok(ProceduralGoal::IsolateResidualControversy),
        "decide_evidence_needed" => Ok(ProceduralGoal::DecideEvidenceNeeded),
        "prepare_for_adjudication" => Ok(ProceduralGoal::PrepareForAdjudication),
        _ => Err(LegalControversyStoreError::UnknownEnum { kind: "procedural_goal", value: value.into() }),
    }
}

pub fn persist_legal_proposition_fibre(
    config: &DatabaseConfig,
    value: &LegalPropositionFibre,
) -> Result<LegalPropositionFibre, LegalControversyStoreError> {
    value.validate().map_err(|_| LegalControversyStoreError::InvalidDomainObject)?;
    install_legal_controversy_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO legal_controversy.proposition_fibre
           (fibre_ref,claim_ref,party_ref,legal_role_ref,epistemic_status_ref,evidence_kind_ref,
            source_reference,reviewed_evidence_ref,normative_order_ref,temporal_reference,
            relation_reference,candidate_only,creates_semantic_authority,creates_legal_authority,
            applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,TRUE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(fibre_ref) DO NOTHING"#,
        &[&value.fibre_ref,&value.claim_ref,&party_db(value.party),&legal_role_db(value.legal_role),
          &epistemic_db(value.epistemic_status),&value.evidence_kind_ref,&value.source_reference,
          &value.reviewed_evidence_ref,&value.normative_order_ref,&value.temporal_reference,
          &value.relation_reference],
    )?;
    let loaded = load_legal_proposition_fibre(config, &value.fibre_ref)?;
    if loaded != *value { return Err(LegalControversyStoreError::ExistingRowConflict); }
    Ok(loaded)
}

pub fn load_legal_proposition_fibre(
    config: &DatabaseConfig,
    fibre_ref: &str,
) -> Result<LegalPropositionFibre, LegalControversyStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT fibre_ref,claim_ref,party_ref,legal_role_ref,epistemic_status_ref,evidence_kind_ref,
                  source_reference,reviewed_evidence_ref,normative_order_ref,temporal_reference,
                  relation_reference,candidate_only,creates_semantic_authority,creates_legal_authority,
                  applicability_promoted,claim_truth_promoted
           FROM legal_controversy.proposition_fibre WHERE fibre_ref=$1"#,
        &[&fibre_ref],
    )?.ok_or_else(|| LegalControversyStoreError::NotFound(fibre_ref.to_owned()))?;
    Ok(LegalPropositionFibre {
        fibre_ref: row.get(0), claim_ref: row.get(1), party: party_from_db(&row.get::<_,String>(2))?,
        legal_role: legal_role_from_db(&row.get::<_,String>(3))?,
        epistemic_status: epistemic_from_db(&row.get::<_,String>(4))?, evidence_kind_ref: row.get(5),
        source_reference: row.get(6), reviewed_evidence_ref: row.get(7), normative_order_ref: row.get(8),
        temporal_reference: row.get(9), relation_reference: row.get(10), candidate_only: row.get(11),
        creates_semantic_authority: row.get(12), creates_legal_authority: row.get(13),
        applicability_promoted: row.get(14), claim_truth_promoted: row.get(15),
    })
}

pub fn persist_legal_controversy_matter(
    config: &DatabaseConfig,
    value: &LegalControversyMatter,
) -> Result<LegalControversyMatter, LegalControversyStoreError> {
    value.validate().map_err(|_| LegalControversyStoreError::InvalidDomainObject)?;
    install_legal_controversy_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.execute(
        r#"INSERT INTO legal_controversy.matter
           (controversy_ref,matter_ref,root_fibre_ref,candidate_only,creates_semantic_authority,
            creates_legal_authority,applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,TRUE,FALSE,FALSE,FALSE,FALSE) ON CONFLICT(controversy_ref) DO NOTHING"#,
        &[&value.controversy_ref,&value.matter_ref,&value.root_fibre_ref],
    )?;
    for fibre_ref in &value.proposition_fibre_refs {
        tx.execute(
            "INSERT INTO legal_controversy.matter_fibre(controversy_ref,fibre_ref) VALUES($1,$2) ON CONFLICT DO NOTHING",
            &[&value.controversy_ref, fibre_ref],
        )?;
    }
    tx.commit()?;
    let loaded = load_legal_controversy_matter(config, &value.controversy_ref)?;
    if loaded.matter_ref != value.matter_ref
        || loaded.root_fibre_ref != value.root_fibre_ref
        || loaded.proposition_fibre_refs != canonical(value.proposition_fibre_refs.clone())
    { return Err(LegalControversyStoreError::ExistingRowConflict); }
    Ok(loaded)
}

pub fn load_legal_controversy_matter(
    config: &DatabaseConfig,
    controversy_ref: &str,
) -> Result<LegalControversyMatter, LegalControversyStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        "SELECT matter_ref,root_fibre_ref,candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted FROM legal_controversy.matter WHERE controversy_ref=$1",
        &[&controversy_ref],
    )?.ok_or_else(|| LegalControversyStoreError::NotFound(controversy_ref.to_owned()))?;
    let proposition_fibre_refs = refs(&mut client,
        "SELECT fibre_ref FROM legal_controversy.matter_fibre WHERE controversy_ref=$1 ORDER BY fibre_ref",
        controversy_ref)?;
    let response_refs = refs(&mut client,
        "SELECT response_ref FROM legal_controversy.response_edge WHERE controversy_ref=$1 ORDER BY response_ref",
        controversy_ref)?;
    let residual_refs = refs(&mut client,
        "SELECT residual_ref FROM legal_controversy.residual WHERE controversy_ref=$1 ORDER BY residual_ref",
        controversy_ref)?;
    let obligation_refs = refs(&mut client,
        "SELECT obligation_ref FROM legal_controversy.proof_obligation WHERE controversy_ref=$1 ORDER BY obligation_ref",
        controversy_ref)?;
    Ok(LegalControversyMatter {
        controversy_ref: controversy_ref.to_owned(), matter_ref: row.get(0), root_fibre_ref: row.get(1),
        proposition_fibre_refs, response_refs, residual_refs, obligation_refs, candidate_only: row.get(2),
        creates_semantic_authority: row.get(3), creates_legal_authority: row.get(4),
        applicability_promoted: row.get(5), claim_truth_promoted: row.get(6),
    })
}

pub fn persist_typed_response_edge(
    config: &DatabaseConfig,
    value: &TypedResponseEdge,
) -> Result<TypedResponseEdge, LegalControversyStoreError> {
    value.validate().map_err(|_| LegalControversyStoreError::InvalidDomainObject)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO legal_controversy.response_edge
           (response_ref,controversy_ref,target_fibre_ref,response_fibre_ref,mode_ref,response_reference,
            candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,TRUE,FALSE,FALSE,FALSE,FALSE) ON CONFLICT(response_ref) DO NOTHING"#,
        &[&value.response_ref,&value.controversy_ref,&value.target_fibre_ref,&value.response_fibre_ref,
          &response_mode_db(value.mode),&value.response_reference],
    )?;
    load_typed_response_edge(config, &value.response_ref)
}

pub fn load_typed_response_edge(
    config: &DatabaseConfig,
    response_ref: &str,
) -> Result<TypedResponseEdge, LegalControversyStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        "SELECT controversy_ref,target_fibre_ref,response_fibre_ref,mode_ref,response_reference,candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted FROM legal_controversy.response_edge WHERE response_ref=$1",
        &[&response_ref],
    )?.ok_or_else(|| LegalControversyStoreError::NotFound(response_ref.to_owned()))?;
    Ok(TypedResponseEdge {
        response_ref: response_ref.to_owned(), controversy_ref: row.get(0), target_fibre_ref: row.get(1),
        response_fibre_ref: row.get(2), mode: response_mode_from_db(&row.get::<_,String>(3))?,
        response_reference: row.get(4), candidate_only: row.get(5), creates_semantic_authority: row.get(6),
        creates_legal_authority: row.get(7), applicability_promoted: row.get(8), claim_truth_promoted: row.get(9),
    })
}

pub fn persist_legal_controversy_residual(
    config: &DatabaseConfig,
    value: &LegalControversyResidual,
) -> Result<LegalControversyResidual, LegalControversyStoreError> {
    value.validate().map_err(|_| LegalControversyStoreError::InvalidDomainObject)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO legal_controversy.residual
           (residual_ref,controversy_ref,kind_ref,applicant_fibre_ref,respondent_fibre_ref,
            residual_level_ref,unresolved_question,requested_discriminator,target_evidence_query,
            candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,TRUE,FALSE,FALSE,FALSE,FALSE) ON CONFLICT(residual_ref) DO NOTHING"#,
        &[&value.residual_ref,&value.controversy_ref,&disagreement_db(value.kind),&value.applicant_fibre_ref,
          &value.respondent_fibre_ref,&value.residual_level_ref,&value.unresolved_question,
          &value.requested_discriminator,&value.target_evidence_query],
    )?;
    load_legal_controversy_residual(config, &value.residual_ref)
}

pub fn load_legal_controversy_residual(
    config: &DatabaseConfig,
    residual_ref: &str,
) -> Result<LegalControversyResidual, LegalControversyStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        "SELECT controversy_ref,kind_ref,applicant_fibre_ref,respondent_fibre_ref,residual_level_ref,unresolved_question,requested_discriminator,target_evidence_query,candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted FROM legal_controversy.residual WHERE residual_ref=$1",
        &[&residual_ref],
    )?.ok_or_else(|| LegalControversyStoreError::NotFound(residual_ref.to_owned()))?;
    Ok(LegalControversyResidual {
        residual_ref: residual_ref.to_owned(), controversy_ref: row.get(0),
        kind: disagreement_from_db(&row.get::<_,String>(1))?, applicant_fibre_ref: row.get(2),
        respondent_fibre_ref: row.get(3), residual_level_ref: row.get(4), unresolved_question: row.get(5),
        requested_discriminator: row.get(6), target_evidence_query: row.get(7), candidate_only: row.get(8),
        creates_semantic_authority: row.get(9), creates_legal_authority: row.get(10),
        applicability_promoted: row.get(11), claim_truth_promoted: row.get(12),
    })
}

pub fn persist_legal_proof_obligation(
    config: &DatabaseConfig,
    value: &LegalProofObligation,
) -> Result<LegalProofObligation, LegalControversyStoreError> {
    value.validate().map_err(|_| LegalControversyStoreError::InvalidDomainObject)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO legal_controversy.proof_obligation
           (obligation_ref,controversy_ref,proposition_fibre_ref,kind_ref,required_by,discharge_reference,is_open,
            candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,$7,TRUE,FALSE,FALSE,FALSE,FALSE) ON CONFLICT(obligation_ref) DO NOTHING"#,
        &[&value.obligation_ref,&value.controversy_ref,&value.proposition_fibre_ref,&obligation_db(value.kind),
          &value.required_by,&value.discharge_reference,&value.is_open],
    )?;
    load_legal_proof_obligation(config, &value.obligation_ref)
}

pub fn load_legal_proof_obligation(
    config: &DatabaseConfig,
    obligation_ref: &str,
) -> Result<LegalProofObligation, LegalControversyStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        "SELECT controversy_ref,proposition_fibre_ref,kind_ref,required_by,discharge_reference,is_open,candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted FROM legal_controversy.proof_obligation WHERE obligation_ref=$1",
        &[&obligation_ref],
    )?.ok_or_else(|| LegalControversyStoreError::NotFound(obligation_ref.to_owned()))?;
    Ok(LegalProofObligation {
        obligation_ref: obligation_ref.to_owned(), controversy_ref: row.get(0), proposition_fibre_ref: row.get(1),
        kind: obligation_from_db(&row.get::<_,String>(2))?, required_by: row.get(3), discharge_reference: row.get(4),
        is_open: row.get(5), candidate_only: row.get(6), creates_semantic_authority: row.get(7),
        creates_legal_authority: row.get(8), applicability_promoted: row.get(9), claim_truth_promoted: row.get(10),
    })
}

pub fn persist_reverse_legal_proof_search(
    config: &DatabaseConfig,
    value: &ReverseLegalProofSearch,
) -> Result<ReverseLegalProofSearch, LegalControversyStoreError> {
    value.validate().map_err(|_| LegalControversyStoreError::InvalidDomainObject)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO legal_controversy.reverse_search
           (reverse_ref,controversy_ref,goal_ref,open_obligation_refs,candidate_residual_refs,
            requested_discriminator,target_evidence_query,candidate_only,creates_semantic_authority,
            creates_legal_authority,applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,$7,TRUE,FALSE,FALSE,FALSE,FALSE) ON CONFLICT(reverse_ref) DO NOTHING"#,
        &[&value.reverse_ref,&value.controversy_ref,&goal_db(value.goal),&value.open_obligation_refs,
          &value.candidate_residual_refs,&value.requested_discriminator,&value.target_evidence_query],
    )?;
    load_reverse_legal_proof_search(config, &value.reverse_ref)
}

pub fn load_reverse_legal_proof_search(
    config: &DatabaseConfig,
    reverse_ref: &str,
) -> Result<ReverseLegalProofSearch, LegalControversyStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        "SELECT controversy_ref,goal_ref,open_obligation_refs,candidate_residual_refs,requested_discriminator,target_evidence_query,candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted FROM legal_controversy.reverse_search WHERE reverse_ref=$1",
        &[&reverse_ref],
    )?.ok_or_else(|| LegalControversyStoreError::NotFound(reverse_ref.to_owned()))?;
    Ok(ReverseLegalProofSearch {
        reverse_ref: reverse_ref.to_owned(), controversy_ref: row.get(0), goal: goal_from_db(&row.get::<_,String>(1))?,
        open_obligation_refs: row.get(2), candidate_residual_refs: row.get(3), requested_discriminator: row.get(4),
        target_evidence_query: row.get(5), candidate_only: row.get(6), creates_semantic_authority: row.get(7),
        creates_legal_authority: row.get(8), applicability_promoted: row.get(9), claim_truth_promoted: row.get(10),
    })
}

fn refs(client: &mut Client, query: &str, owner_ref: &str) -> Result<Vec<String>, postgres::Error> {
    client.query(query, &[&owner_ref]).map(|rows| rows.into_iter().map(|row| row.get(0)).collect())
}
fn canonical(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}
