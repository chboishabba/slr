//! Persisted typed adversarial controversy over one Matter.
//!
//! This is a coordination carrier over existing reviewed-evidence and REL
//! owners. It does not re-store source bytes, evidence payloads, comparison
//! packets, authority or adjudicative conclusions. The packet records how
//! already-owned proposition/support coordinates participate in a controversy.

use std::collections::BTreeSet;

use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    load_relational_comparison, load_reviewed_evidence_coordinate, DatabaseConfig,
};

pub const MATTER_CONTROVERSY_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.matter_controversy (
  controversy_ref TEXT PRIMARY KEY,
  matter_ref TEXT NOT NULL,
  packet_sha256 TEXT NOT NULL,
  packet_json TEXT NOT NULL,
  derived_only BOOLEAN NOT NULL CHECK (derived_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);
CREATE INDEX IF NOT EXISTS matter_controversy_matter_idx
ON semantic.matter_controversy (matter_ref);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatterPartyRole {
    Applicant,
    Respondent,
    Court,
    ExternalWitness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatterLegalRole {
    PleadedFact,
    EvidentiaryFact,
    LegalProposition,
    CausalLink,
    RequestedOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatterEpistemicStatus {
    Alleged,
    Admitted,
    Disputed,
    Supported,
    Proved,
    Rejected,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatterEvidenceKind {
    SourceText,
    AccountRecord,
    Message,
    Report,
    Testimony,
    EventRecord,
    OtherEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatterResponseMode {
    DenyOccurrence,
    AdmitOccurrenceDisputeCharacterisation,
    AdmitConductAddContext,
    DisputeCausation,
    ChallengeEvidenceReliability,
    OfferAlternativeEvent,
    AdmitProposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatterDisagreementKind {
    Node,
    Relation,
    Evidence,
    Characterisation,
    Causal,
    LegalConsequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatterObligationKind {
    Premise,
    Evidence,
    Response,
    Discriminator,
    Adjudicative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatterPropositionDraft {
    pub proposition_ref: String,
    pub matter_ref: String,
    pub party: MatterPartyRole,
    pub legal_role: MatterLegalRole,
    pub epistemic_status: MatterEpistemicStatus,
    /// Existing source revision or other persisted source coordinate.
    pub source_ref: String,
    /// When present this must reopen as reviewed evidence owned by this Matter.
    pub reviewed_evidence_ref: Option<String>,
    pub evidence_kind: MatterEvidenceKind,
    pub temporal_ref: String,
    pub relation_ref: String,
    pub normative_order_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatterResponseDraft {
    pub response_ref: String,
    pub target_proposition_ref: String,
    pub response_proposition_ref: String,
    pub mode: MatterResponseMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatterControversyResidualDraft {
    pub residual_ref: String,
    pub kind: MatterDisagreementKind,
    pub applicant_proposition_ref: String,
    pub respondent_proposition_ref: String,
    pub unresolved_question: String,
    /// Optional link to an already persisted REL comparison/residual.
    pub relational_comparison_ref: Option<String>,
    pub relational_obligation_ref: Option<String>,
    pub requested_discriminator_ref: String,
    pub target_evidence_query: String,
    /// Candidate dependency targets only. This does not create actual reopening.
    pub potential_reopening_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatterObligationDraft {
    pub obligation_ref: String,
    pub proposition_ref: String,
    pub kind: MatterObligationKind,
    pub required_by_ref: String,
    pub discharge_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatterControversyDraft {
    pub controversy_ref: String,
    pub matter_ref: String,
    pub root_proposition_ref: String,
    pub propositions: Vec<MatterPropositionDraft>,
    pub responses: Vec<MatterResponseDraft>,
    pub residuals: Vec<MatterControversyResidualDraft>,
    pub obligations: Vec<MatterObligationDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedMatterControversy {
    pub controversy: MatterControversyDraft,
    pub derived_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum MatterControversyError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("required controversy coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("controversy references a proposition from another Matter")]
    WrongMatterOwner,
    #[error("controversy contains a duplicate or unknown proposition coordinate")]
    PropositionMismatch,
    #[error("response edge is invalid or self-targeting")]
    ResponseMismatch,
    #[error("controversy residual is malformed")]
    ResidualMismatch,
    #[error("proof obligation is malformed")]
    ObligationMismatch,
    #[error("reviewed-evidence owner does not reopen exactly: {0}")]
    ReviewedEvidenceMismatch(String),
    #[error("REL residual owner does not reopen exactly: {0}")]
    RelationalResidualMismatch(String),
    #[error("persisted controversy changed or crossed a non-promotion boundary")]
    ChangedReplay,
    #[error("matter controversy not found: {0}")]
    NotFound(String),
}

pub fn validate_matter_controversy_draft(
    draft: &MatterControversyDraft,
) -> Result<(), MatterControversyError> {
    for (name, value) in [
        ("controversy_ref", draft.controversy_ref.as_str()),
        ("matter_ref", draft.matter_ref.as_str()),
        ("root_proposition_ref", draft.root_proposition_ref.as_str()),
    ] {
        required(name, value)?;
    }
    if draft.propositions.is_empty() {
        return Err(MatterControversyError::PropositionMismatch);
    }

    let mut proposition_refs = BTreeSet::new();
    for proposition in &draft.propositions {
        for (name, value) in [
            ("proposition_ref", proposition.proposition_ref.as_str()),
            ("proposition_matter_ref", proposition.matter_ref.as_str()),
            ("source_ref", proposition.source_ref.as_str()),
            ("temporal_ref", proposition.temporal_ref.as_str()),
            ("relation_ref", proposition.relation_ref.as_str()),
            ("normative_order_ref", proposition.normative_order_ref.as_str()),
        ] {
            required(name, value)?;
        }
        if proposition.matter_ref != draft.matter_ref {
            return Err(MatterControversyError::WrongMatterOwner);
        }
        if !proposition_refs.insert(proposition.proposition_ref.clone()) {
            return Err(MatterControversyError::PropositionMismatch);
        }
        if proposition
            .reviewed_evidence_ref
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(MatterControversyError::EmptyCoordinate(
                "reviewed_evidence_ref",
            ));
        }
    }
    if !proposition_refs.contains(&draft.root_proposition_ref) {
        return Err(MatterControversyError::PropositionMismatch);
    }

    let mut response_refs = BTreeSet::new();
    for response in &draft.responses {
        required("response_ref", &response.response_ref)?;
        if !response_refs.insert(response.response_ref.clone())
            || response.target_proposition_ref == response.response_proposition_ref
            || !proposition_refs.contains(&response.target_proposition_ref)
            || !proposition_refs.contains(&response.response_proposition_ref)
        {
            return Err(MatterControversyError::ResponseMismatch);
        }
    }

    let mut residual_refs = BTreeSet::new();
    for residual in &draft.residuals {
        for (name, value) in [
            ("residual_ref", residual.residual_ref.as_str()),
            ("unresolved_question", residual.unresolved_question.as_str()),
            (
                "requested_discriminator_ref",
                residual.requested_discriminator_ref.as_str(),
            ),
            ("target_evidence_query", residual.target_evidence_query.as_str()),
        ] {
            required(name, value)?;
        }
        if !residual_refs.insert(residual.residual_ref.clone())
            || residual.applicant_proposition_ref == residual.respondent_proposition_ref
            || !proposition_refs.contains(&residual.applicant_proposition_ref)
            || !proposition_refs.contains(&residual.respondent_proposition_ref)
            || residual
                .potential_reopening_refs
                .iter()
                .any(|reference| !proposition_refs.contains(reference))
            || residual.relational_comparison_ref.is_some()
                != residual.relational_obligation_ref.is_some()
        {
            return Err(MatterControversyError::ResidualMismatch);
        }
    }

    let mut obligation_refs = BTreeSet::new();
    for obligation in &draft.obligations {
        for (name, value) in [
            ("obligation_ref", obligation.obligation_ref.as_str()),
            ("required_by_ref", obligation.required_by_ref.as_str()),
        ] {
            required(name, value)?;
        }
        if !obligation_refs.insert(obligation.obligation_ref.clone())
            || !proposition_refs.contains(&obligation.proposition_ref)
            || obligation
                .discharge_ref
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(MatterControversyError::ObligationMismatch);
        }
    }
    Ok(())
}

pub fn install_matter_controversy_schema(
    config: &DatabaseConfig,
) -> Result<(), MatterControversyError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(MATTER_CONTROVERSY_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_matter_controversy(
    config: &DatabaseConfig,
    draft: &MatterControversyDraft,
) -> Result<PersistedMatterControversy, MatterControversyError> {
    validate_matter_controversy_draft(draft)?;
    validate_external_owners(config, draft)?;
    install_matter_controversy_schema(config)?;

    let packet = PersistedMatterControversy {
        controversy: draft.clone(),
        derived_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let body = serde_json::to_string(&packet)?;
    let digest = sha256_text(&body);
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO semantic.matter_controversy
           (controversy_ref,matter_ref,packet_sha256,packet_json,derived_only,
            creates_semantic_authority,creates_legal_authority,
            applicability_promoted,claim_truth_promoted)
           VALUES ($1,$2,$3,$4,TRUE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT (controversy_ref) DO NOTHING"#,
        &[&draft.controversy_ref, &draft.matter_ref, &digest, &body],
    )?;
    let loaded = load_matter_controversy(config, &draft.controversy_ref)?;
    if loaded != packet {
        return Err(MatterControversyError::ChangedReplay);
    }
    Ok(loaded)
}

pub fn load_matter_controversy(
    config: &DatabaseConfig,
    controversy_ref: &str,
) -> Result<PersistedMatterControversy, MatterControversyError> {
    required("controversy_ref", controversy_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT matter_ref,packet_sha256,packet_json,derived_only,
                      creates_semantic_authority,creates_legal_authority,
                      applicability_promoted,claim_truth_promoted
               FROM semantic.matter_controversy WHERE controversy_ref=$1"#,
            &[&controversy_ref],
        )?
        .ok_or_else(|| MatterControversyError::NotFound(controversy_ref.to_owned()))?;
    if !row.get::<_, bool>(3)
        || row.get::<_, bool>(4)
        || row.get::<_, bool>(5)
        || row.get::<_, bool>(6)
        || row.get::<_, bool>(7)
    {
        return Err(MatterControversyError::ChangedReplay);
    }
    let body: String = row.get(2);
    if sha256_text(&body) != row.get::<_, String>(1) {
        return Err(MatterControversyError::ChangedReplay);
    }
    let packet: PersistedMatterControversy = serde_json::from_str(&body)?;
    if packet.controversy.controversy_ref != controversy_ref
        || packet.controversy.matter_ref != row.get::<_, String>(0)
        || !packet.derived_only
        || packet.creates_semantic_authority
        || packet.creates_legal_authority
        || packet.applicability_promoted
        || packet.claim_truth_promoted
    {
        return Err(MatterControversyError::ChangedReplay);
    }
    validate_matter_controversy_draft(&packet.controversy)?;
    validate_external_owners(config, &packet.controversy)?;
    Ok(packet)
}

fn validate_external_owners(
    config: &DatabaseConfig,
    draft: &MatterControversyDraft,
) -> Result<(), MatterControversyError> {
    for proposition in &draft.propositions {
        let Some(reviewed_ref) = proposition.reviewed_evidence_ref.as_deref() else {
            continue;
        };
        let reviewed = load_reviewed_evidence_coordinate(config, reviewed_ref)
            .map_err(|error| MatterControversyError::ReviewedEvidenceMismatch(error.to_string()))?;
        if reviewed.consumer_ref != draft.matter_ref
            || reviewed.source_revision_ref != proposition.source_ref
            || reviewed.normative_order_ref != proposition.normative_order_ref
            || !reviewed.candidate_only
            || reviewed.creates_semantic_authority
            || reviewed.applicability_promoted
            || reviewed.claim_truth_promoted
        {
            return Err(MatterControversyError::ReviewedEvidenceMismatch(
                reviewed_ref.to_owned(),
            ));
        }
    }

    for residual in &draft.residuals {
        let (Some(comparison_ref), Some(obligation_ref)) = (
            residual.relational_comparison_ref.as_deref(),
            residual.relational_obligation_ref.as_deref(),
        ) else {
            continue;
        };
        let comparison = load_relational_comparison(config, comparison_ref)
            .map_err(|error| MatterControversyError::RelationalResidualMismatch(error.to_string()))?
            .ok_or_else(|| {
                MatterControversyError::RelationalResidualMismatch(comparison_ref.to_owned())
            })?;
        if comparison.comparison.comparison_ref != comparison_ref
            || comparison.comparison.creates_semantic_authority
            || comparison.comparison.claim_truth_promoted
            || !comparison
                .comparison
                .residuals
                .iter()
                .any(|candidate| candidate.obligation_ref == obligation_ref)
        {
            return Err(MatterControversyError::RelationalResidualMismatch(
                obligation_ref.to_owned(),
            ));
        }
    }
    Ok(())
}

fn required(name: &'static str, value: &str) -> Result<(), MatterControversyError> {
    if value.trim().is_empty() {
        Err(MatterControversyError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn sha256_text(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}
