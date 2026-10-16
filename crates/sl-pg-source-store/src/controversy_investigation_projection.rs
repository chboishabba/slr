//! Read-only product projection for the controversy closure loop.
//!
//! UI consumers start from the persisted Matter controversy/legal obligation;
//! they do not need to know or guess the derived acquisition-obligation ref.

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::{
    load_controversy_evidence_stage, load_controversy_investigation_queue,
    ControversyEvidenceStage, ControversyInvestigationStoreError,
    DatabaseConfig, DurableControversyInvestigationQueue,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControversyInvestigationStatus {
    pub matter_ref: String,
    pub controversy_ref: String,
    pub legal_obligation_ref: String,
    pub queue: DurableControversyInvestigationQueue,
    pub evidence_stages: Vec<ControversyEvidenceStage>,
    pub evidence_supplied: bool,
    pub requires_residual_reassessment: bool,
    pub residual_resolved_by_system: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Error)]
pub enum ControversyInvestigationProjectionError {
    #[error(transparent)]
    Store(#[from] ControversyInvestigationStoreError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("controversy investigation projection has inconsistent ownership")]
    WrongOwner,
}

pub fn load_controversy_investigation_status(
    config: &DatabaseConfig,
    controversy_ref: &str,
    legal_obligation_ref: &str,
) -> Result<Option<ControversyInvestigationStatus>, ControversyInvestigationProjectionError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let Some(row) = client.query_opt(
        r#"SELECT obligation_ref FROM legal_controversy.investigation_obligation
           WHERE controversy_ref=$1 AND legal_obligation_ref=$2"#,
        &[&controversy_ref, &legal_obligation_ref],
    )? else {
        return Ok(None);
    };
    let obligation_ref: String = row.get(0);
    let queue = load_controversy_investigation_queue(config, &obligation_ref)?
        .ok_or(ControversyInvestigationProjectionError::WrongOwner)?;
    if queue.obligation.controversy_ref != controversy_ref
        || queue.obligation.legal_obligation_ref != legal_obligation_ref
    {
        return Err(ControversyInvestigationProjectionError::WrongOwner);
    }

    let mut evidence_stages = Vec::new();
    for row in client.query(
        r#"SELECT stage_ref FROM legal_controversy.evidence_stage
           WHERE controversy_ref=$1 AND legal_obligation_ref=$2
           ORDER BY stage_sequence, stage_ref"#,
        &[&controversy_ref, &legal_obligation_ref],
    )? {
        let stage = load_controversy_evidence_stage(config, &row.get::<_, String>(0))?;
        if stage.matter_ref != queue.obligation.matter_ref
            || stage.controversy_ref != queue.obligation.controversy_ref
            || stage.legal_obligation_ref != queue.obligation.legal_obligation_ref
        {
            return Err(ControversyInvestigationProjectionError::WrongOwner);
        }
        evidence_stages.push(stage);
    }
    let evidence_supplied = evidence_stages.iter().any(|stage| stage.evidence_supplied);
    let requires_residual_reassessment = evidence_stages
        .iter()
        .any(|stage| stage.requires_residual_reassessment && !stage.residual_resolved);

    Ok(Some(ControversyInvestigationStatus {
        matter_ref: queue.obligation.matter_ref.clone(),
        controversy_ref: queue.obligation.controversy_ref.clone(),
        legal_obligation_ref: queue.obligation.legal_obligation_ref.clone(),
        queue,
        evidence_stages,
        evidence_supplied,
        requires_residual_reassessment,
        residual_resolved_by_system: false,
        creates_semantic_authority: false,
        creates_legal_authority: false,
    }))
}
