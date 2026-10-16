//! Durable REAL-MATTER controversy investigation closure loop.
//!
//! Persistence is deliberately separate from the REL-owned acquisition queue:
//! a legal proof obligation is not relabelled as a relational-comparison
//! residual. Existing acquisition route/Pareto/reopening types are reused while
//! ownership remains with the legal controversy.

use postgres::{Client, NoTls};
use serde_json;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    build_controversy_evidence_stage, canonicalize_controversy_acquisition_routes,
    controversy_acquisition_pareto_frontier, controversy_obligation_from_parts,
    load_legal_controversy_matter, load_legal_controversy_residual,
    load_legal_proof_obligation, load_legal_proposition_fibre,
    load_reviewed_evidence_coordinate, selective_reopening,
    validate_controversy_acquisition_update, AcquisitionPriorityReceipt,
    AcquisitionRouteCandidate, AcquisitionUpdate, ControversyAcquisitionObligation,
    ControversyEvidenceStage, ControversyInvestigationError,
    ControversyReviewedEvidenceDischarge, DatabaseConfig, LegalControversyStoreError,
    ReviewedEvidenceStoreError, SelectiveReopeningReceipt,
};

pub const CONTROVERSY_INVESTIGATION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS legal_controversy;
CREATE TABLE IF NOT EXISTS legal_controversy.investigation_obligation (
  obligation_ref TEXT PRIMARY KEY,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref) ON DELETE CASCADE,
  legal_obligation_ref TEXT NOT NULL REFERENCES legal_controversy.proof_obligation(obligation_ref),
  residual_ref TEXT NOT NULL REFERENCES legal_controversy.residual(residual_ref),
  packet_sha256 TEXT NOT NULL,
  packet_json TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  UNIQUE(controversy_ref, legal_obligation_ref)
);
CREATE TABLE IF NOT EXISTS legal_controversy.investigation_route (
  route_ref TEXT PRIMARY KEY,
  obligation_ref TEXT NOT NULL REFERENCES legal_controversy.investigation_obligation(obligation_ref) ON DELETE CASCADE,
  packet_sha256 TEXT NOT NULL,
  packet_json TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_acquisition_authority BOOLEAN NOT NULL CHECK (NOT creates_acquisition_authority)
);
CREATE TABLE IF NOT EXISTS legal_controversy.investigation_priority (
  obligation_ref TEXT PRIMARY KEY REFERENCES legal_controversy.investigation_obligation(obligation_ref) ON DELETE CASCADE,
  packet_sha256 TEXT NOT NULL,
  packet_json TEXT NOT NULL,
  scalar_score_used BOOLEAN NOT NULL CHECK (NOT scalar_score_used),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_decision BOOLEAN NOT NULL CHECK (NOT creates_decision)
);
CREATE TABLE IF NOT EXISTS legal_controversy.investigation_update (
  obligation_ref TEXT NOT NULL REFERENCES legal_controversy.investigation_obligation(obligation_ref) ON DELETE CASCADE,
  acquisition_receipt_ref TEXT NOT NULL,
  update_sha256 TEXT NOT NULL,
  update_json TEXT NOT NULL,
  reopening_sha256 TEXT NULL,
  reopening_json TEXT NULL,
  PRIMARY KEY(obligation_ref, acquisition_receipt_ref)
);
CREATE TABLE IF NOT EXISTS legal_controversy.evidence_stage (
  stage_ref TEXT PRIMARY KEY,
  matter_ref TEXT NOT NULL,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref) ON DELETE CASCADE,
  legal_obligation_ref TEXT NOT NULL REFERENCES legal_controversy.proof_obligation(obligation_ref),
  residual_ref TEXT NOT NULL REFERENCES legal_controversy.residual(residual_ref),
  acquisition_obligation_ref TEXT NOT NULL REFERENCES legal_controversy.investigation_obligation(obligation_ref),
  reviewed_evidence_ref TEXT NOT NULL REFERENCES semantic.reviewed_evidence_coordinate(reviewed_evidence_ref),
  packet_sha256 TEXT NOT NULL,
  packet_json TEXT NOT NULL,
  evidence_supplied BOOLEAN NOT NULL CHECK (evidence_supplied),
  controversy_reopened BOOLEAN NOT NULL CHECK (controversy_reopened),
  requires_residual_reassessment BOOLEAN NOT NULL CHECK (requires_residual_reassessment),
  residual_resolved BOOLEAN NOT NULL CHECK (NOT residual_resolved),
  determines_credibility BOOLEAN NOT NULL CHECK (NOT determines_credibility),
  determines_ultimate_fact BOOLEAN NOT NULL CHECK (NOT determines_ultimate_fact),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);
ALTER TABLE legal_controversy.evidence_stage
  ADD COLUMN IF NOT EXISTS stage_sequence BIGSERIAL;
CREATE UNIQUE INDEX IF NOT EXISTS controversy_evidence_stage_sequence_idx
  ON legal_controversy.evidence_stage(stage_sequence);
CREATE INDEX IF NOT EXISTS controversy_evidence_stage_lookup_idx
  ON legal_controversy.evidence_stage(controversy_ref, legal_obligation_ref);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableControversyInvestigationQueue {
    pub obligation: ControversyAcquisitionObligation,
    pub routes: Vec<AcquisitionRouteCandidate>,
    pub priority: AcquisitionPriorityReceipt,
}

#[derive(Debug, Error)]
pub enum ControversyInvestigationStoreError {
    #[error(transparent)]
    Domain(#[from] ControversyInvestigationError),
    #[error(transparent)]
    Controversy(#[from] LegalControversyStoreError),
    #[error(transparent)]
    ReviewedEvidence(#[from] ReviewedEvidenceStoreError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("controversy investigation owner is missing or inconsistent")]
    WrongOwner,
    #[error("persisted controversy investigation changed under immutable identity")]
    ChangedReplay,
    #[error("acquired source revision is not present in a native/provider-backed source store")]
    AcquiredSourceNotIngested,
}

fn digest(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

pub fn install_controversy_investigation_schema(
    config: &DatabaseConfig,
) -> Result<(), ControversyInvestigationStoreError> {
    crate::install_legal_controversy_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CONTROVERSY_INVESTIGATION_SCHEMA_SQL)?;
    Ok(())
}

pub fn derive_controversy_acquisition_obligation(
    config: &DatabaseConfig,
    controversy_ref: &str,
    legal_obligation_ref: &str,
    authority_or_access_constraint_ref: &str,
    dependency_target_refs: Vec<String>,
) -> Result<ControversyAcquisitionObligation, ControversyInvestigationStoreError> {
    let matter = load_legal_controversy_matter(config, controversy_ref)?;
    let legal_obligation = load_legal_proof_obligation(config, legal_obligation_ref)?;
    if legal_obligation.controversy_ref != matter.controversy_ref {
        return Err(ControversyInvestigationStoreError::WrongOwner);
    }
    let residual = load_legal_controversy_residual(config, &legal_obligation.required_by)?;
    let applicant = load_legal_proposition_fibre(config, &residual.applicant_fibre_ref)?;
    let respondent = load_legal_proposition_fibre(config, &residual.respondent_fibre_ref)?;
    let applicant_reviewed_ref = applicant
        .reviewed_evidence_ref
        .as_deref()
        .ok_or(ControversyInvestigationStoreError::WrongOwner)?;
    let respondent_reviewed_ref = respondent
        .reviewed_evidence_ref
        .as_deref()
        .ok_or(ControversyInvestigationStoreError::WrongOwner)?;
    let applicant_reviewed = load_reviewed_evidence_coordinate(config, applicant_reviewed_ref)?;
    let respondent_reviewed = load_reviewed_evidence_coordinate(config, respondent_reviewed_ref)?;

    Ok(controversy_obligation_from_parts(
        &matter,
        &residual,
        &legal_obligation,
        &[applicant, respondent],
        vec![
            applicant_reviewed.source_revision_ref,
            respondent_reviewed.source_revision_ref,
        ],
        authority_or_access_constraint_ref,
        dependency_target_refs,
    )?)
}

pub fn persist_controversy_investigation_queue(
    config: &DatabaseConfig,
    obligation: &ControversyAcquisitionObligation,
    routes: &[AcquisitionRouteCandidate],
) -> Result<DurableControversyInvestigationQueue, ControversyInvestigationStoreError> {
    install_controversy_investigation_schema(config)?;
    let rederived = derive_controversy_acquisition_obligation(
        config,
        &obligation.controversy_ref,
        &obligation.legal_obligation_ref,
        &obligation.authority_or_access_constraint_ref,
        obligation.dependency_target_refs.clone(),
    )?;
    if rederived != *obligation {
        return Err(ControversyInvestigationStoreError::WrongOwner);
    }

    let canonical_routes = canonicalize_controversy_acquisition_routes(routes)?;
    let priority = controversy_acquisition_pareto_frontier(obligation, &canonical_routes)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let obligation_json = serde_json::to_string(obligation)?;
    client.execute(
        r#"INSERT INTO legal_controversy.investigation_obligation
           (obligation_ref,controversy_ref,legal_obligation_ref,residual_ref,packet_sha256,packet_json,
            candidate_only,creates_semantic_authority,creates_legal_authority,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,TRUE,FALSE,FALSE,FALSE)
           ON CONFLICT(obligation_ref) DO NOTHING"#,
        &[
            &obligation.obligation_ref,
            &obligation.controversy_ref,
            &obligation.legal_obligation_ref,
            &obligation.residual_ref,
            &digest(&obligation_json),
            &obligation_json,
        ],
    )?;
    for route in &canonical_routes {
        let body = serde_json::to_string(route)?;
        client.execute(
            r#"INSERT INTO legal_controversy.investigation_route
               (route_ref,obligation_ref,packet_sha256,packet_json,candidate_only,creates_acquisition_authority)
               VALUES($1,$2,$3,$4,TRUE,FALSE) ON CONFLICT(route_ref) DO NOTHING"#,
            &[&route.route_ref, &route.obligation_ref, &digest(&body), &body],
        )?;
    }
    let priority_json = serde_json::to_string(&priority)?;
    client.execute(
        r#"INSERT INTO legal_controversy.investigation_priority
           (obligation_ref,packet_sha256,packet_json,scalar_score_used,creates_semantic_authority,creates_decision)
           VALUES($1,$2,$3,FALSE,FALSE,FALSE)
           ON CONFLICT(obligation_ref) DO NOTHING"#,
        &[&obligation.obligation_ref, &digest(&priority_json), &priority_json],
    )?;
    let reopened = load_controversy_investigation_queue(config, &obligation.obligation_ref)?
        .ok_or(ControversyInvestigationStoreError::ChangedReplay)?;
    let expected = DurableControversyInvestigationQueue {
        obligation: obligation.clone(),
        routes: canonical_routes,
        priority,
    };
    if reopened != expected {
        return Err(ControversyInvestigationStoreError::ChangedReplay);
    }
    Ok(reopened)
}

pub fn load_controversy_investigation_queue(
    config: &DatabaseConfig,
    obligation_ref: &str,
) -> Result<Option<DurableControversyInvestigationQueue>, ControversyInvestigationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let Some(row) = client.query_opt(
        r#"SELECT packet_sha256,packet_json,candidate_only,creates_semantic_authority,creates_legal_authority,claim_truth_promoted
           FROM legal_controversy.investigation_obligation WHERE obligation_ref=$1"#,
        &[&obligation_ref],
    )? else {
        return Ok(None);
    };
    let obligation_json: String = row.get(1);
    if digest(&obligation_json) != row.get::<_, String>(0)
        || !row.get::<_, bool>(2)
        || row.get::<_, bool>(3)
        || row.get::<_, bool>(4)
        || row.get::<_, bool>(5)
    {
        return Err(ControversyInvestigationStoreError::ChangedReplay);
    }
    let obligation: ControversyAcquisitionObligation = serde_json::from_str(&obligation_json)?;
    let mut routes = Vec::new();
    for route_row in client.query(
        r#"SELECT packet_sha256,packet_json,candidate_only,creates_acquisition_authority
           FROM legal_controversy.investigation_route WHERE obligation_ref=$1 ORDER BY route_ref"#,
        &[&obligation_ref],
    )? {
        let body: String = route_row.get(1);
        if digest(&body) != route_row.get::<_, String>(0)
            || !route_row.get::<_, bool>(2)
            || route_row.get::<_, bool>(3)
        {
            return Err(ControversyInvestigationStoreError::ChangedReplay);
        }
        routes.push(serde_json::from_str(&body)?);
    }
    if routes != canonicalize_controversy_acquisition_routes(&routes)? {
        return Err(ControversyInvestigationStoreError::ChangedReplay);
    }
    let priority_row = client.query_one(
        r#"SELECT packet_sha256,packet_json,scalar_score_used,creates_semantic_authority,creates_decision
           FROM legal_controversy.investigation_priority WHERE obligation_ref=$1"#,
        &[&obligation_ref],
    )?;
    let priority_json: String = priority_row.get(1);
    if digest(&priority_json) != priority_row.get::<_, String>(0)
        || priority_row.get::<_, bool>(2)
        || priority_row.get::<_, bool>(3)
        || priority_row.get::<_, bool>(4)
    {
        return Err(ControversyInvestigationStoreError::ChangedReplay);
    }
    let priority: AcquisitionPriorityReceipt = serde_json::from_str(&priority_json)?;
    if priority != controversy_acquisition_pareto_frontier(&obligation, &routes)? {
        return Err(ControversyInvestigationStoreError::ChangedReplay);
    }
    Ok(Some(DurableControversyInvestigationQueue {
        obligation,
        routes,
        priority,
    }))
}

fn acquired_source_exists(
    client: &mut Client,
    source_revision_ref: &str,
) -> Result<bool, postgres::Error> {
    if client
        .query_opt(
            r#"SELECT 1 FROM ingest.generic_source_revision
               WHERE source_revision_ref=$1 AND candidate_only=TRUE
                 AND creates_semantic_authority=FALSE AND applicability_promoted=FALSE
                 AND claim_truth_promoted=FALSE"#,
            &[&source_revision_ref],
        )?
        .is_some()
    {
        return Ok(true);
    }
    if client
        .query_opt(
            r#"SELECT 1 FROM corpus.chat_archive_message
               WHERE source_revision_ref=$1 AND candidate_only=TRUE
                 AND creates_semantic_authority=FALSE AND applicability_promoted=FALSE
                 AND claim_truth_promoted=FALSE LIMIT 1"#,
            &[&source_revision_ref],
        )?
        .is_some()
    {
        return Ok(true);
    }
    Ok(client
        .query_opt(
            r#"SELECT 1 FROM source_provenance.legal_source_manifestation
               WHERE source_revision_ref=$1 AND candidate_only=TRUE
                 AND creates_semantic_authority=FALSE AND creates_legal_authority=FALSE
                 AND applicability_promoted=FALSE AND claim_truth_promoted=FALSE LIMIT 1"#,
            &[&source_revision_ref],
        )?
        .is_some())
}

pub fn persist_controversy_acquisition_update(
    config: &DatabaseConfig,
    update: &AcquisitionUpdate,
    dependency_graph_ref: &str,
    dependency_edges: &[(String, String)],
    dependency_universe_refs: &[String],
) -> Result<SelectiveReopeningReceipt, ControversyInvestigationStoreError> {
    let queue = load_controversy_investigation_queue(config, &update.obligation_ref)?
        .ok_or(ControversyInvestigationStoreError::WrongOwner)?;
    validate_controversy_acquisition_update(&queue.obligation, update)?;
    let source_revision_ref = update
        .acquired_source_revision_ref
        .as_deref()
        .ok_or(ControversyInvestigationStoreError::WrongOwner)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    if !acquired_source_exists(&mut client, source_revision_ref)? {
        return Err(ControversyInvestigationStoreError::AcquiredSourceNotIngested);
    }
    let reopening = selective_reopening(
        source_revision_ref,
        dependency_graph_ref,
        dependency_edges,
        dependency_universe_refs,
    )?;
    let update_json = serde_json::to_string(update)?;
    let reopening_json = serde_json::to_string(&reopening)?;
    client.execute(
        r#"INSERT INTO legal_controversy.investigation_update
           (obligation_ref,acquisition_receipt_ref,update_sha256,update_json,reopening_sha256,reopening_json)
           VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING"#,
        &[
            &update.obligation_ref,
            &update.acquisition_receipt_ref,
            &digest(&update_json),
            &update_json,
            &digest(&reopening_json),
            &reopening_json,
        ],
    )?;

    // Reopen the immutable row even after an insertion conflict. A repeated
    // receipt is valid only when it is an exact replay of both the acquisition
    // update and its selective-reopening receipt.
    let persisted = client
        .query_opt(
            r#"SELECT update_sha256,update_json,reopening_sha256,reopening_json
               FROM legal_controversy.investigation_update
               WHERE obligation_ref=$1 AND acquisition_receipt_ref=$2"#,
            &[&update.obligation_ref, &update.acquisition_receipt_ref],
        )?
        .ok_or(ControversyInvestigationStoreError::ChangedReplay)?;
    let persisted_update_json: String = persisted.get(1);
    let persisted_reopening_json: String = persisted
        .get::<_, Option<String>>(3)
        .ok_or(ControversyInvestigationStoreError::ChangedReplay)?;
    if persisted_update_json != update_json
        || persisted_reopening_json != reopening_json
        || persisted.get::<_, String>(0) != digest(&persisted_update_json)
        || persisted
            .get::<_, Option<String>>(2)
            .as_deref()
            .is_none_or(|expected| expected != digest(&persisted_reopening_json))
    {
        return Err(ControversyInvestigationStoreError::ChangedReplay);
    }
    let persisted_reopening: SelectiveReopeningReceipt =
        serde_json::from_str(&persisted_reopening_json)?;
    if persisted_reopening != reopening {
        return Err(ControversyInvestigationStoreError::ChangedReplay);
    }
    Ok(persisted_reopening)
}

pub fn persist_controversy_reviewed_evidence_stage(
    config: &DatabaseConfig,
    obligation_ref: &str,
    acquisition_receipt_ref: &str,
    reviewed_evidence_ref: &str,
) -> Result<ControversyEvidenceStage, ControversyInvestigationStoreError> {
    let queue = load_controversy_investigation_queue(config, obligation_ref)?
        .ok_or(ControversyInvestigationStoreError::WrongOwner)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT update_sha256,update_json,reopening_sha256,reopening_json
               FROM legal_controversy.investigation_update
               WHERE obligation_ref=$1 AND acquisition_receipt_ref=$2"#,
            &[&obligation_ref, &acquisition_receipt_ref],
        )?
        .ok_or(ControversyInvestigationStoreError::WrongOwner)?;
    let update_json: String = row.get(1);
    let reopening_json: String = row
        .get::<_, Option<String>>(3)
        .ok_or(ControversyInvestigationStoreError::WrongOwner)?;
    if digest(&update_json) != row.get::<_, String>(0)
        || row
            .get::<_, Option<String>>(2)
            .as_deref()
            .is_none_or(|expected| expected != digest(&reopening_json))
    {
        return Err(ControversyInvestigationStoreError::ChangedReplay);
    }
    let update: AcquisitionUpdate = serde_json::from_str(&update_json)?;
    let reopening: SelectiveReopeningReceipt = serde_json::from_str(&reopening_json)?;
    let reviewed = load_reviewed_evidence_coordinate(config, reviewed_evidence_ref)?;
    if !reviewed.candidate_only
        || reviewed.creates_semantic_authority
        || reviewed.applicability_promoted
        || reviewed.claim_truth_promoted
    {
        return Err(ControversyInvestigationStoreError::WrongOwner);
    }
    let discharge = ControversyReviewedEvidenceDischarge {
        reviewed_evidence_ref: reviewed.reviewed_evidence_ref,
        requirement_ref: reviewed.requirement_ref,
        source_revision_ref: reviewed.source_revision_ref,
    };
    let stage = build_controversy_evidence_stage(&queue.obligation, &update, &discharge, &reopening)?;
    let body = serde_json::to_string(&stage)?;
    client.execute(
        r#"INSERT INTO legal_controversy.evidence_stage
           (stage_ref,matter_ref,controversy_ref,legal_obligation_ref,residual_ref,
            acquisition_obligation_ref,reviewed_evidence_ref,packet_sha256,packet_json,
            evidence_supplied,controversy_reopened,requires_residual_reassessment,residual_resolved,
            determines_credibility,determines_ultimate_fact,creates_semantic_authority,
            creates_legal_authority,applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,TRUE,TRUE,TRUE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(stage_ref) DO NOTHING"#,
        &[
            &stage.stage_ref,
            &stage.matter_ref,
            &stage.controversy_ref,
            &stage.legal_obligation_ref,
            &stage.residual_ref,
            &stage.acquisition_obligation_ref,
            &stage.reviewed_evidence_ref,
            &digest(&body),
            &body,
        ],
    )?;
    let reopened = load_controversy_evidence_stage(config, &stage.stage_ref)?;
    if reopened != stage {
        return Err(ControversyInvestigationStoreError::ChangedReplay);
    }
    Ok(reopened)
}

pub fn load_controversy_evidence_stage(
    config: &DatabaseConfig,
    stage_ref: &str,
) -> Result<ControversyEvidenceStage, ControversyInvestigationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT packet_sha256,packet_json,evidence_supplied,controversy_reopened,
                      requires_residual_reassessment,residual_resolved,determines_credibility,
                      determines_ultimate_fact,creates_semantic_authority,creates_legal_authority,
                      applicability_promoted,claim_truth_promoted
               FROM legal_controversy.evidence_stage WHERE stage_ref=$1"#,
            &[&stage_ref],
        )?
        .ok_or(ControversyInvestigationStoreError::WrongOwner)?;
    let body: String = row.get(1);
    if digest(&body) != row.get::<_, String>(0)
        || !row.get::<_, bool>(2)
        || !row.get::<_, bool>(3)
        || !row.get::<_, bool>(4)
        || row.get::<_, bool>(5)
        || row.get::<_, bool>(6)
        || row.get::<_, bool>(7)
        || row.get::<_, bool>(8)
        || row.get::<_, bool>(9)
        || row.get::<_, bool>(10)
        || row.get::<_, bool>(11)
    {
        return Err(ControversyInvestigationStoreError::ChangedReplay);
    }
    Ok(serde_json::from_str(&body)?)
}

pub fn load_latest_controversy_evidence_stage(
    config: &DatabaseConfig,
    controversy_ref: &str,
    legal_obligation_ref: &str,
) -> Result<Option<ControversyEvidenceStage>, ControversyInvestigationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT stage_ref FROM legal_controversy.evidence_stage
           WHERE controversy_ref=$1 AND legal_obligation_ref=$2
           ORDER BY stage_sequence DESC, stage_ref DESC LIMIT 1"#,
        &[&controversy_ref, &legal_obligation_ref],
    )?;
    match row {
        Some(row) => Ok(Some(load_controversy_evidence_stage(config, &row.get::<_, String>(0))?)),
        None => Ok(None),
    }
}
