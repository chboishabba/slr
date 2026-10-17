//! Immutable residual refinement/supersession for one persisted legal controversy.
//!
//! Residual rows remain historical facts about what the system represented at a
//! particular reviewed stage.  A later residual never overwrites an earlier row.
//! Instead, an explicit S29 review receipt may license a supersession edge.  The
//! operator-facing Matter projection is then derived as the unique unsuperseded
//! residual head in each typed residual lane.
//!
//! This module deliberately does not infer "latest" from insertion order, hashes,
//! lexical order, or timestamps.  Multiple unsuperseded heads in one lane are an
//! ambiguity and fail closed until a reviewed reconciliation is persisted.

use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    install_legal_controversy_schema, install_review_workstation_schema,
    load_legal_controversy_matter, load_legal_controversy_residual,
    load_legal_proof_obligation, DatabaseConfig, DisagreementKind,
    LegalControversyMatter, LegalControversyResidual,
};

pub const LEGAL_CONTROVERSY_RESIDUAL_HISTORY_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS legal_controversy;

CREATE TABLE IF NOT EXISTS legal_controversy.residual_supersession (
  supersession_ref TEXT PRIMARY KEY,
  controversy_ref TEXT NOT NULL
    REFERENCES legal_controversy.matter(controversy_ref) ON DELETE CASCADE,
  lane_ref TEXT NOT NULL,
  successor_residual_ref TEXT NOT NULL
    REFERENCES legal_controversy.residual(residual_ref),
  review_command_ref TEXT NOT NULL
    REFERENCES semantic.review_receipt(command_ref),
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS legal_controversy.residual_supersession_predecessor (
  supersession_ref TEXT NOT NULL
    REFERENCES legal_controversy.residual_supersession(supersession_ref) ON DELETE CASCADE,
  predecessor_residual_ref TEXT NOT NULL
    REFERENCES legal_controversy.residual(residual_ref),
  PRIMARY KEY (supersession_ref, predecessor_residual_ref)
);

CREATE INDEX IF NOT EXISTS residual_supersession_controversy_lane_idx
ON legal_controversy.residual_supersession(controversy_ref, lane_ref);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualSupersessionDraft {
    pub controversy_ref: String,
    pub successor_residual_ref: String,
    pub predecessor_residual_refs: Vec<String>,
    /// Existing S29 `semantic.review_receipt.command_ref` whose semantic_ref is
    /// the successor residual and whose action explicitly accepts/qualifies/
    /// supersedes the reviewed stage.
    pub review_command_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualSupersessionReceipt {
    pub supersession_ref: String,
    pub controversy_ref: String,
    pub lane_ref: String,
    pub successor_residual_ref: String,
    pub predecessor_residual_refs: Vec<String>,
    pub review_command_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LegalControversyResidualHistoryError {
    #[error("required residual-history coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("residual supersession crosses controversy or typed residual lane")]
    ResidualLaneMismatch,
    #[error("residual supersession is self-referential or contains duplicate predecessors")]
    InvalidSupersession,
    #[error("more than one unsuperseded residual remains in one typed lane")]
    AmbiguousResidualHead,
    #[error("review receipt does not explicitly review the successor residual")]
    ReviewReceiptMismatch,
    #[error("persisted residual supersession conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("residual history row not found: {0}")]
    NotFound(String),
    #[error("store error: {0}")]
    Store(String),
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for LegalControversyResidualHistoryError {
    fn from(value: postgres::Error) -> Self {
        Self::Postgres(value.to_string())
    }
}

fn nonempty(name: &'static str, value: &str) -> Result<(), LegalControversyResidualHistoryError> {
    if value.trim().is_empty() {
        Err(LegalControversyResidualHistoryError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn disagreement_tag(value: DisagreementKind) -> &'static str {
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

fn hash_ref(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{prefix}:sha256:{:x}", hasher.finalize())
}

/// Stable identity for the semantic lane in which a residual may evolve.
/// Narrative wording/discriminator/query are intentionally excluded: changing
/// those is exactly what a reviewed refinement is allowed to supersede.
#[must_use]
pub fn residual_lane_ref(residual: &LegalControversyResidual) -> String {
    hash_ref(
        "legal-residual-lane",
        &[
            &residual.controversy_ref,
            disagreement_tag(residual.kind),
            &residual.applicant_fibre_ref,
            &residual.respondent_fibre_ref,
        ],
    )
}

fn canonical_refs(values: &[String]) -> Result<Vec<String>, LegalControversyResidualHistoryError> {
    if values.is_empty() {
        return Err(LegalControversyResidualHistoryError::InvalidSupersession);
    }
    let mut result = values.to_vec();
    if result.iter().any(|value| value.trim().is_empty()) {
        return Err(LegalControversyResidualHistoryError::EmptyCoordinate(
            "predecessor_residual_ref",
        ));
    }
    result.sort();
    if result.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(LegalControversyResidualHistoryError::InvalidSupersession);
    }
    Ok(result)
}

fn validate_receipt_shape(
    receipt: &ResidualSupersessionReceipt,
) -> Result<(), LegalControversyResidualHistoryError> {
    for (name, value) in [
        ("supersession_ref", receipt.supersession_ref.as_str()),
        ("controversy_ref", receipt.controversy_ref.as_str()),
        ("lane_ref", receipt.lane_ref.as_str()),
        ("successor_residual_ref", receipt.successor_residual_ref.as_str()),
        ("review_command_ref", receipt.review_command_ref.as_str()),
    ] {
        nonempty(name, value)?;
    }
    let canonical = canonical_refs(&receipt.predecessor_residual_refs)?;
    if canonical != receipt.predecessor_residual_refs
        || canonical
            .iter()
            .any(|value| value == &receipt.successor_residual_ref)
        || !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.creates_legal_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
    {
        return Err(LegalControversyResidualHistoryError::InvalidSupersession);
    }
    Ok(())
}

/// Pure current-head derivation used by both runtime and regression tests.
/// Every typed lane must have exactly one unsuperseded head.
pub fn derive_current_residual_refs(
    residuals: &[LegalControversyResidual],
    supersessions: &[ResidualSupersessionReceipt],
) -> Result<Vec<String>, LegalControversyResidualHistoryError> {
    let by_ref = residuals
        .iter()
        .map(|residual| (residual.residual_ref.as_str(), residual))
        .collect::<BTreeMap<_, _>>();
    let mut by_lane = BTreeMap::<String, Vec<&LegalControversyResidual>>::new();
    for residual in residuals {
        if residual.residual_ref.trim().is_empty() || residual.controversy_ref.trim().is_empty() {
            return Err(LegalControversyResidualHistoryError::EmptyCoordinate("residual_ref"));
        }
        by_lane.entry(residual_lane_ref(residual)).or_default().push(residual);
    }

    let mut superseded = BTreeSet::<String>::new();
    let mut predecessor_owner = BTreeMap::<String, String>::new();
    for receipt in supersessions {
        validate_receipt_shape(receipt)?;
        let successor = by_ref
            .get(receipt.successor_residual_ref.as_str())
            .ok_or(LegalControversyResidualHistoryError::ResidualLaneMismatch)?;
        if successor.controversy_ref != receipt.controversy_ref
            || residual_lane_ref(successor) != receipt.lane_ref
        {
            return Err(LegalControversyResidualHistoryError::ResidualLaneMismatch);
        }
        for predecessor_ref in &receipt.predecessor_residual_refs {
            let predecessor = by_ref
                .get(predecessor_ref.as_str())
                .ok_or(LegalControversyResidualHistoryError::ResidualLaneMismatch)?;
            if predecessor.controversy_ref != receipt.controversy_ref
                || residual_lane_ref(predecessor) != receipt.lane_ref
            {
                return Err(LegalControversyResidualHistoryError::ResidualLaneMismatch);
            }
            if predecessor_owner
                .insert(predecessor_ref.clone(), receipt.successor_residual_ref.clone())
                .is_some()
            {
                return Err(LegalControversyResidualHistoryError::InvalidSupersession);
            }
            superseded.insert(predecessor_ref.clone());
        }
    }

    let mut current = Vec::new();
    for (_lane, lane_residuals) in by_lane {
        let mut heads = lane_residuals
            .into_iter()
            .filter(|residual| !superseded.contains(&residual.residual_ref))
            .map(|residual| residual.residual_ref.clone())
            .collect::<Vec<_>>();
        heads.sort();
        if heads.len() != 1 {
            return Err(LegalControversyResidualHistoryError::AmbiguousResidualHead);
        }
        current.push(heads.remove(0));
    }
    current.sort();
    Ok(current)
}

pub fn install_legal_controversy_residual_history_schema(
    config: &DatabaseConfig,
) -> Result<(), LegalControversyResidualHistoryError> {
    install_legal_controversy_schema(config)
        .map_err(|error| LegalControversyResidualHistoryError::Store(error.to_string()))?;
    install_review_workstation_schema(config)
        .map_err(|error| LegalControversyResidualHistoryError::Store(error.to_string()))?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(LEGAL_CONTROVERSY_RESIDUAL_HISTORY_SQL)?;
    Ok(())
}

fn require_review_receipt(
    config: &DatabaseConfig,
    command_ref: &str,
    successor_residual_ref: &str,
) -> Result<(), LegalControversyResidualHistoryError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT semantic_ref,action_ref,candidate_only,creates_semantic_authority,
                  applicability_promoted,claim_truth_promoted
           FROM semantic.review_receipt WHERE command_ref=$1"#,
        &[&command_ref],
    )?;
    let Some(row) = row else {
        return Err(LegalControversyResidualHistoryError::ReviewReceiptMismatch);
    };
    let action: String = row.get(1);
    if row.get::<_, String>(0) != successor_residual_ref
        || !matches!(action.as_str(), "accept" | "qualify" | "supersede")
        || !row.get::<_, bool>(2)
        || row.get::<_, bool>(3)
        || row.get::<_, bool>(4)
        || row.get::<_, bool>(5)
    {
        return Err(LegalControversyResidualHistoryError::ReviewReceiptMismatch);
    }
    Ok(())
}

pub fn persist_reviewed_residual_supersession(
    config: &DatabaseConfig,
    draft: &ResidualSupersessionDraft,
) -> Result<ResidualSupersessionReceipt, LegalControversyResidualHistoryError> {
    for (name, value) in [
        ("controversy_ref", draft.controversy_ref.as_str()),
        ("successor_residual_ref", draft.successor_residual_ref.as_str()),
        ("review_command_ref", draft.review_command_ref.as_str()),
    ] {
        nonempty(name, value)?;
    }
    install_legal_controversy_residual_history_schema(config)?;
    let successor = load_legal_controversy_residual(config, &draft.successor_residual_ref)
        .map_err(|error| LegalControversyResidualHistoryError::Store(error.to_string()))?;
    let lane_ref = residual_lane_ref(&successor);
    if successor.controversy_ref != draft.controversy_ref {
        return Err(LegalControversyResidualHistoryError::ResidualLaneMismatch);
    }
    let predecessors = canonical_refs(&draft.predecessor_residual_refs)?;
    if predecessors
        .iter()
        .any(|value| value == &draft.successor_residual_ref)
    {
        return Err(LegalControversyResidualHistoryError::InvalidSupersession);
    }
    for predecessor_ref in &predecessors {
        let predecessor = load_legal_controversy_residual(config, predecessor_ref)
            .map_err(|error| LegalControversyResidualHistoryError::Store(error.to_string()))?;
        if predecessor.controversy_ref != draft.controversy_ref
            || residual_lane_ref(&predecessor) != lane_ref
        {
            return Err(LegalControversyResidualHistoryError::ResidualLaneMismatch);
        }
    }
    require_review_receipt(config, &draft.review_command_ref, &draft.successor_residual_ref)?;

    let predecessor_key = predecessors.join("\u{1f}");
    let supersession_ref = hash_ref(
        "legal-residual-supersession",
        &[
            &draft.controversy_ref,
            &lane_ref,
            &draft.successor_residual_ref,
            &predecessor_key,
            &draft.review_command_ref,
        ],
    );
    let expected = ResidualSupersessionReceipt {
        supersession_ref: supersession_ref.clone(),
        controversy_ref: draft.controversy_ref.clone(),
        lane_ref,
        successor_residual_ref: draft.successor_residual_ref.clone(),
        predecessor_residual_refs: predecessors,
        review_command_ref: draft.review_command_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    validate_receipt_shape(&expected)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.execute(
        r#"INSERT INTO legal_controversy.residual_supersession
           (supersession_ref,controversy_ref,lane_ref,successor_residual_ref,review_command_ref,
            candidate_only,creates_semantic_authority,creates_legal_authority,
            applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,TRUE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(supersession_ref) DO NOTHING"#,
        &[
            &expected.supersession_ref,
            &expected.controversy_ref,
            &expected.lane_ref,
            &expected.successor_residual_ref,
            &expected.review_command_ref,
        ],
    )?;
    for predecessor_ref in &expected.predecessor_residual_refs {
        tx.execute(
            r#"INSERT INTO legal_controversy.residual_supersession_predecessor
               (supersession_ref,predecessor_residual_ref)
               VALUES($1,$2) ON CONFLICT DO NOTHING"#,
            &[&expected.supersession_ref, predecessor_ref],
        )?;
    }
    tx.commit()?;
    let reopened = load_residual_supersession(config, &supersession_ref)?;
    if reopened != expected {
        return Err(LegalControversyResidualHistoryError::ExistingRowConflict);
    }

    // The new receipt must actually make this lane unambiguous; otherwise the
    // operator has reviewed only part of an already-branching history.
    let _ = load_current_legal_controversy_residuals(config, &draft.controversy_ref)?;
    Ok(reopened)
}

pub fn load_residual_supersession(
    config: &DatabaseConfig,
    supersession_ref: &str,
) -> Result<ResidualSupersessionReceipt, LegalControversyResidualHistoryError> {
    nonempty("supersession_ref", supersession_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT controversy_ref,lane_ref,successor_residual_ref,review_command_ref,
                  candidate_only,creates_semantic_authority,creates_legal_authority,
                  applicability_promoted,claim_truth_promoted
           FROM legal_controversy.residual_supersession WHERE supersession_ref=$1"#,
        &[&supersession_ref],
    )?;
    let Some(row) = row else {
        return Err(LegalControversyResidualHistoryError::NotFound(
            supersession_ref.to_owned(),
        ));
    };
    let predecessors = client
        .query(
            r#"SELECT predecessor_residual_ref
               FROM legal_controversy.residual_supersession_predecessor
               WHERE supersession_ref=$1 ORDER BY predecessor_residual_ref"#,
            &[&supersession_ref],
        )?
        .into_iter()
        .map(|row| row.get(0))
        .collect::<Vec<String>>();
    let receipt = ResidualSupersessionReceipt {
        supersession_ref: supersession_ref.to_owned(),
        controversy_ref: row.get(0),
        lane_ref: row.get(1),
        successor_residual_ref: row.get(2),
        predecessor_residual_refs: predecessors,
        review_command_ref: row.get(3),
        candidate_only: row.get(4),
        creates_semantic_authority: row.get(5),
        creates_legal_authority: row.get(6),
        applicability_promoted: row.get(7),
        claim_truth_promoted: row.get(8),
    };
    validate_receipt_shape(&receipt)?;
    Ok(receipt)
}

pub fn load_residual_supersessions_for_controversy(
    config: &DatabaseConfig,
    controversy_ref: &str,
) -> Result<Vec<ResidualSupersessionReceipt>, LegalControversyResidualHistoryError> {
    nonempty("controversy_ref", controversy_ref)?;
    install_legal_controversy_residual_history_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let refs = client.query(
        r#"SELECT supersession_ref FROM legal_controversy.residual_supersession
           WHERE controversy_ref=$1 ORDER BY supersession_ref"#,
        &[&controversy_ref],
    )?;
    refs.into_iter()
        .map(|row| {
            let reference: String = row.get(0);
            load_residual_supersession(config, &reference)
        })
        .collect()
}

pub fn load_current_legal_controversy_residuals(
    config: &DatabaseConfig,
    controversy_ref: &str,
) -> Result<Vec<LegalControversyResidual>, LegalControversyResidualHistoryError> {
    let matter = load_legal_controversy_matter(config, controversy_ref)
        .map_err(|error| LegalControversyResidualHistoryError::Store(error.to_string()))?;
    let residuals = matter
        .residual_refs
        .iter()
        .map(|reference| {
            load_legal_controversy_residual(config, reference)
                .map_err(|error| LegalControversyResidualHistoryError::Store(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let supersessions = load_residual_supersessions_for_controversy(config, controversy_ref)?;
    let current_refs = derive_current_residual_refs(&residuals, &supersessions)?;
    let by_ref = residuals
        .into_iter()
        .map(|residual| (residual.residual_ref.clone(), residual))
        .collect::<BTreeMap<_, _>>();
    current_refs
        .into_iter()
        .map(|reference| {
            by_ref
                .get(&reference)
                .cloned()
                .ok_or(LegalControversyResidualHistoryError::ResidualLaneMismatch)
        })
        .collect()
}

/// Operator/reverse-search projection of a legal controversy.  The ordinary
/// `load_legal_controversy_matter` remains the historical/audit loader and still
/// exposes every immutable residual/obligation row.
pub fn load_current_legal_controversy_matter(
    config: &DatabaseConfig,
    controversy_ref: &str,
) -> Result<LegalControversyMatter, LegalControversyResidualHistoryError> {
    let mut matter = load_legal_controversy_matter(config, controversy_ref)
        .map_err(|error| LegalControversyResidualHistoryError::Store(error.to_string()))?;
    let current_residuals = load_current_legal_controversy_residuals(config, controversy_ref)?;
    let current_refs = current_residuals
        .iter()
        .map(|residual| residual.residual_ref.clone())
        .collect::<BTreeSet<_>>();
    let all_residual_refs = matter.residual_refs.iter().cloned().collect::<BTreeSet<_>>();
    let historical_refs = all_residual_refs
        .difference(&current_refs)
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut current_obligations = Vec::new();
    for obligation_ref in &matter.obligation_refs {
        let obligation = load_legal_proof_obligation(config, obligation_ref)
            .map_err(|error| LegalControversyResidualHistoryError::Store(error.to_string()))?;
        if historical_refs.contains(&obligation.required_by) {
            continue;
        }
        current_obligations.push(obligation_ref.clone());
    }
    current_obligations.sort();
    current_obligations.dedup();

    matter.residual_refs = current_refs.into_iter().collect();
    matter.obligation_refs = current_obligations;
    Ok(matter)
}
