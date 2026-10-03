use postgres::{Client, NoTls};
use serde_json;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{load_acquisition_queue, DatabaseConfig, InvestigationStoreError};
use sensiblaw_reader_model::PersistedWorkbenchProjection;
use super::investigation_graph_binding::{
    InvestigationGraphBinding, InvestigationGraphBindingError, INV_GRAPH_BINDING_SCHEMA,
};

pub const INV_GRAPH_BINDING_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.investigation_graph_binding (
    binding_ref TEXT PRIMARY KEY,
    obligation_ref TEXT NOT NULL
      REFERENCES semantic.investigation_acquisition_obligation(obligation_ref),
    projection_ref TEXT NOT NULL,
    matter_ref TEXT NOT NULL,
    packet_sha256 TEXT NOT NULL,
    packet_json TEXT NOT NULL,
    derived_only BOOLEAN NOT NULL CHECK(derived_only),
    challengeable BOOLEAN NOT NULL CHECK(challengeable),
    creates_graph_edges BOOLEAN NOT NULL CHECK(NOT creates_graph_edges),
    creates_semantic_authority BOOLEAN NOT NULL CHECK(NOT creates_semantic_authority),
    creates_access_authority BOOLEAN NOT NULL CHECK(NOT creates_access_authority),
    creates_acquisition_state BOOLEAN NOT NULL CHECK(NOT creates_acquisition_state)
);
CREATE UNIQUE INDEX IF NOT EXISTS investigation_graph_binding_obligation_projection_idx
  ON semantic.investigation_graph_binding(obligation_ref, projection_ref);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundInvestigationGraphProjection {
    pub binding: InvestigationGraphBinding,
    pub projection: PersistedWorkbenchProjection,
}

#[derive(Debug, Error)]
pub enum InvestigationGraphBindingStoreError {
    #[error(transparent)]
    Pg(#[from] postgres::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Investigation(#[from] InvestigationStoreError),
    #[error(transparent)]
    Binding(#[from] InvestigationGraphBindingError),
    #[error(transparent)]
    Projection(#[from] super::legacy::WorkbenchProjectionError),
    #[error("graph binding owner or persisted projection is missing/inconsistent")]
    WrongOwner,
    #[error("immutable graph binding changed on replay")]
    ChangedReplay,
}

fn digest(body: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(body.as_bytes()))
}

fn persisted_projection_is_derived_challengeable(
    client: &mut Client,
    projection_ref: &str,
) -> Result<bool, postgres::Error> {
    Ok(client
        .query_opt(
            "SELECT 1 FROM pnf_follow_projection
             WHERE projection_ref=$1
               AND projection_kind='legal_follow'
               AND authority_ceiling='derived_only_challengeable'
               AND promotion_allowed=FALSE
               AND execution_allowed=FALSE",
            &[&projection_ref],
        )?
        .is_some())
}

pub fn persist_investigation_graph_binding(
    config: &DatabaseConfig,
    binding: &InvestigationGraphBinding,
) -> Result<InvestigationGraphBinding, InvestigationGraphBindingStoreError> {
    if binding.schema != INV_GRAPH_BINDING_SCHEMA
        || !binding.derived_only
        || !binding.challengeable
        || binding.creates_graph_edges
        || binding.creates_semantic_authority
        || binding.creates_access_authority
        || binding.creates_acquisition_state
    {
        return Err(InvestigationGraphBindingStoreError::WrongOwner);
    }
    load_acquisition_queue(config, &binding.obligation_ref)?
        .ok_or(InvestigationGraphBindingStoreError::WrongOwner)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    if !persisted_projection_is_derived_challengeable(&mut client, &binding.projection_ref)? {
        return Err(InvestigationGraphBindingStoreError::WrongOwner);
    }
    client.batch_execute(INV_GRAPH_BINDING_SQL)?;
    let body = serde_json::to_string(binding)?;
    let hash = digest(&body);
    client.execute(
        "INSERT INTO semantic.investigation_graph_binding
         (binding_ref, obligation_ref, projection_ref, matter_ref,
          packet_sha256, packet_json, derived_only, challengeable,
          creates_graph_edges, creates_semantic_authority,
          creates_access_authority, creates_acquisition_state)
         VALUES($1,$2,$3,$4,$5,$6,TRUE,TRUE,FALSE,FALSE,FALSE,FALSE)
         ON CONFLICT(binding_ref) DO NOTHING",
        &[
            &binding.binding_ref,
            &binding.obligation_ref,
            &binding.projection_ref,
            &binding.matter_ref,
            &hash,
            &body,
        ],
    )?;
    let reopened = load_investigation_graph_binding(config, &binding.binding_ref)?
        .ok_or(InvestigationGraphBindingStoreError::ChangedReplay)?;
    if reopened != *binding {
        return Err(InvestigationGraphBindingStoreError::ChangedReplay);
    }
    Ok(reopened)
}

pub fn load_investigation_graph_binding(
    config: &DatabaseConfig,
    binding_ref: &str,
) -> Result<Option<InvestigationGraphBinding>, InvestigationGraphBindingStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(INV_GRAPH_BINDING_SQL)?;
    let Some(row) = client.query_opt(
        "SELECT packet_sha256, packet_json, derived_only, challengeable,
                creates_graph_edges, creates_semantic_authority,
                creates_access_authority, creates_acquisition_state
         FROM semantic.investigation_graph_binding WHERE binding_ref=$1",
        &[&binding_ref],
    )? else {
        return Ok(None);
    };
    if !row.get::<_, bool>(2)
        || !row.get::<_, bool>(3)
        || row.get::<_, bool>(4)
        || row.get::<_, bool>(5)
        || row.get::<_, bool>(6)
        || row.get::<_, bool>(7)
    {
        return Err(InvestigationGraphBindingStoreError::ChangedReplay);
    }
    let body: String = row.get(1);
    if digest(&body) != row.get::<_, String>(0) {
        return Err(InvestigationGraphBindingStoreError::ChangedReplay);
    }
    let binding: InvestigationGraphBinding = serde_json::from_str(&body)?;
    load_acquisition_queue(config, &binding.obligation_ref)?
        .ok_or(InvestigationGraphBindingStoreError::WrongOwner)?;
    if !persisted_projection_is_derived_challengeable(&mut client, &binding.projection_ref)? {
        return Err(InvestigationGraphBindingStoreError::WrongOwner);
    }
    Ok(Some(binding))
}

pub fn load_investigation_graph_binding_for_obligation(
    config: &DatabaseConfig,
    obligation_ref: &str,
) -> Result<Option<InvestigationGraphBinding>, InvestigationGraphBindingStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(INV_GRAPH_BINDING_SQL)?;
    let Some(row) = client.query_opt(
        "SELECT binding_ref FROM semantic.investigation_graph_binding
         WHERE obligation_ref=$1 ORDER BY binding_ref LIMIT 1",
        &[&obligation_ref],
    )? else {
        return Ok(None);
    };
    let binding_ref: String = row.get(0);
    load_investigation_graph_binding(config, &binding_ref)
}

/// Reopen a graph already bound to this INV obligation. Persistence and
/// projection lookup stay in SLR; UI consumers receive typed rows only.
pub fn load_bound_investigation_graph_projection(
    config: &DatabaseConfig,
    obligation_ref: &str,
    matter_ref: &str,
) -> Result<Option<BoundInvestigationGraphProjection>, InvestigationGraphBindingStoreError> {
    let Some(binding) = load_investigation_graph_binding_for_obligation(config, obligation_ref)?
    else {
        return Ok(None);
    };
    if binding.matter_ref != matter_ref {
        return Err(InvestigationGraphBindingStoreError::WrongOwner);
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let projection = super::legacy::load_persisted_workbench_projection(
        &mut client,
        &binding.projection_ref,
        matter_ref,
    )?;
    if projection.legal_follow_graph.projection_ref != binding.projection_ref
        || !projection.legal_follow_graph.derived_only
        || !projection.legal_follow_graph.challengeable
        || projection.creates_semantic_authority
        || projection.creates_claim_truth
        || projection.pays_residual
    {
        return Err(InvestigationGraphBindingStoreError::WrongOwner);
    }
    Ok(Some(BoundInvestigationGraphProjection { binding, projection }))
}
