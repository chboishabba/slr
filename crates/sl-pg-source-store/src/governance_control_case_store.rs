use postgres::{Client, NoTls};
use serde_json;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{load_acquisition_queue, DatabaseConfig, InvestigationStoreError};
use super::governance_control_case::{
    AcquisitionGovernancePacket, GovernanceControlCaseError, INV_GOVERNANCE_SCHEMA,
};

pub const INV_GOVERNANCE_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.inv_governance_packet (
    packet_ref TEXT PRIMARY KEY,
    obligation_ref TEXT NOT NULL
      REFERENCES semantic.investigation_acquisition_obligation(obligation_ref),
    comparison_ref TEXT NOT NULL,
    packet_sha256 TEXT NOT NULL,
    packet_json TEXT NOT NULL,
    creates_semantic_authority BOOLEAN NOT NULL CHECK(NOT creates_semantic_authority),
    creates_access_authority BOOLEAN NOT NULL CHECK(NOT creates_access_authority),
    creates_priority BOOLEAN NOT NULL CHECK(NOT creates_priority),
    certification_claim BOOLEAN NOT NULL CHECK(NOT certification_claim)
);
"#;

#[derive(Debug, Error)]
pub enum GovernanceStoreError {
    #[error(transparent)]
    Pg(#[from] postgres::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Investigation(#[from] InvestigationStoreError),
    #[error(transparent)]
    Packet(#[from] GovernanceControlCaseError),
    #[error("governance packet does not bind exactly to the persisted INV queue")]
    WrongOwner,
    #[error("immutable governance replay changed")]
    ChangedReplay,
}

fn digest(body: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(body.as_bytes()))
}

pub fn persist_inv_governance_packet(
    config: &DatabaseConfig,
    packet: &AcquisitionGovernancePacket,
) -> Result<AcquisitionGovernancePacket, GovernanceStoreError> {
    if packet.schema != INV_GOVERNANCE_SCHEMA
        || packet.creates_semantic_authority
        || packet.creates_access_authority
        || packet.creates_priority
        || packet.certification_claim
    {
        return Err(GovernanceStoreError::WrongOwner);
    }
    let queue = load_acquisition_queue(config, &packet.obligation_ref)?
        .ok_or(GovernanceStoreError::WrongOwner)?;
    if packet.comparison_ref != queue.obligation.comparison_ref
        || packet.source_revision_refs != queue.obligation.source_revision_refs
    {
        return Err(GovernanceStoreError::WrongOwner);
    }

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(INV_GOVERNANCE_SQL)?;
    let body = serde_json::to_string(packet)?;
    let hash = digest(&body);
    client.execute(
        "INSERT INTO semantic.inv_governance_packet
         (packet_ref, obligation_ref, comparison_ref, packet_sha256, packet_json,
          creates_semantic_authority, creates_access_authority, creates_priority,
          certification_claim)
         VALUES($1,$2,$3,$4,$5,FALSE,FALSE,FALSE,FALSE)
         ON CONFLICT(packet_ref) DO NOTHING",
        &[&packet.packet_ref, &packet.obligation_ref, &packet.comparison_ref, &hash, &body],
    )?;
    let reopened = load_inv_governance_packet(config, &packet.packet_ref)?
        .ok_or(GovernanceStoreError::ChangedReplay)?;
    if reopened != *packet {
        return Err(GovernanceStoreError::ChangedReplay);
    }
    Ok(reopened)
}

pub fn load_inv_governance_packet(
    config: &DatabaseConfig,
    packet_ref: &str,
) -> Result<Option<AcquisitionGovernancePacket>, GovernanceStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(INV_GOVERNANCE_SQL)?;
    let Some(row) = client.query_opt(
        "SELECT packet_sha256, packet_json, creates_semantic_authority,
                creates_access_authority, creates_priority, certification_claim
         FROM semantic.inv_governance_packet WHERE packet_ref=$1",
        &[&packet_ref],
    )? else { return Ok(None); };
    if row.get::<_, bool>(2)
        || row.get::<_, bool>(3)
        || row.get::<_, bool>(4)
        || row.get::<_, bool>(5)
    {
        return Err(GovernanceStoreError::ChangedReplay);
    }
    let body: String = row.get(1);
    if digest(&body) != row.get::<_, String>(0) {
        return Err(GovernanceStoreError::ChangedReplay);
    }
    let packet: AcquisitionGovernancePacket = serde_json::from_str(&body)?;
    let queue = load_acquisition_queue(config, &packet.obligation_ref)?
        .ok_or(GovernanceStoreError::WrongOwner)?;
    if packet.comparison_ref != queue.obligation.comparison_ref
        || packet.source_revision_refs != queue.obligation.source_revision_refs
    {
        return Err(GovernanceStoreError::WrongOwner);
    }
    Ok(Some(packet))
}
