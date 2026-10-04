//! Durable dependency/reopening controls for INV empirical acceptance.
//!
//! Kept separate from the semantic comparison: dependency edges and declared
//! unrelated consumers are operational acceptance coordinates, not source or
//! claim truth.

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::{install_inv_acceptance_schema, DatabaseConfig};

pub const ACCEPTANCE_REOPEN_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS acceptance;
CREATE TABLE IF NOT EXISTS acceptance.inv_reopen_dependency (
  reopen_check_ref TEXT PRIMARY KEY REFERENCES acceptance.inv_reopen_check(reopen_check_ref) ON DELETE CASCADE,
  dependency_graph_ref TEXT NOT NULL,
  dependency_from_refs TEXT[] NOT NULL,
  dependency_to_refs TEXT[] NOT NULL,
  dependency_universe_refs TEXT[] NOT NULL,
  expected_unrelated_refs TEXT[] NOT NULL,
  acceptance_only BOOLEAN NOT NULL CHECK (acceptance_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  CHECK (cardinality(dependency_from_refs) = cardinality(dependency_to_refs))
);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvReopenDependencyControl {
    pub reopen_check_ref: String,
    pub dependency_graph_ref: String,
    pub dependency_edges: Vec<(String, String)>,
    pub dependency_universe_refs: Vec<String>,
    pub expected_unrelated_refs: Vec<String>,
}

#[derive(Debug, Error)]
pub enum AcceptanceReopenStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("INV acceptance prerequisite failed: {0}")]
    AcceptanceControl(#[from] crate::AcceptanceControlStoreError),
    #[error("invalid reopening dependency control")]
    InvalidControl,
    #[error("reopening dependency control not found: {0}")]
    NotFound(String),
    #[error("persisted reopening dependency control crossed the acceptance firewall")]
    Promotion,
}

pub fn persist_inv_reopen_dependency_control(
    config: &DatabaseConfig,
    control: &InvReopenDependencyControl,
) -> Result<(), AcceptanceReopenStoreError> {
    if control.reopen_check_ref.trim().is_empty()
        || control.dependency_graph_ref.trim().is_empty()
        || control
            .dependency_edges
            .iter()
            .any(|(a, b)| a.trim().is_empty() || b.trim().is_empty())
        || control
            .dependency_universe_refs
            .iter()
            .any(|v| v.trim().is_empty())
        || control
            .expected_unrelated_refs
            .iter()
            .any(|v| v.trim().is_empty())
    {
        return Err(AcceptanceReopenStoreError::InvalidControl);
    }
    let from = control
        .dependency_edges
        .iter()
        .map(|(a, _)| a.clone())
        .collect::<Vec<_>>();
    let to = control
        .dependency_edges
        .iter()
        .map(|(_, b)| b.clone())
        .collect::<Vec<_>>();
    install_inv_acceptance_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(ACCEPTANCE_REOPEN_SCHEMA_SQL)?;
    client.execute(
        r#"INSERT INTO acceptance.inv_reopen_dependency
        (reopen_check_ref,dependency_graph_ref,dependency_from_refs,dependency_to_refs,
         dependency_universe_refs,expected_unrelated_refs,acceptance_only,
         creates_semantic_authority,claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,true,false,false)
        ON CONFLICT (reopen_check_ref) DO NOTHING"#,
        &[
            &control.reopen_check_ref,
            &control.dependency_graph_ref,
            &from,
            &to,
            &control.dependency_universe_refs,
            &control.expected_unrelated_refs,
        ],
    )?;
    Ok(())
}

pub fn load_inv_reopen_dependency_control(
    config: &DatabaseConfig,
    reopen_check_ref: &str,
) -> Result<InvReopenDependencyControl, AcceptanceReopenStoreError> {
    if reopen_check_ref.trim().is_empty() {
        return Err(AcceptanceReopenStoreError::InvalidControl);
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT dependency_graph_ref,dependency_from_refs,dependency_to_refs,
           dependency_universe_refs,expected_unrelated_refs,acceptance_only,
           creates_semantic_authority,claim_truth_promoted
           FROM acceptance.inv_reopen_dependency WHERE reopen_check_ref=$1"#,
            &[&reopen_check_ref],
        )?
        .ok_or_else(|| AcceptanceReopenStoreError::NotFound(reopen_check_ref.to_owned()))?;
    if !row.get::<_, bool>(5) || row.get::<_, bool>(6) || row.get::<_, bool>(7) {
        return Err(AcceptanceReopenStoreError::Promotion);
    }
    let from: Vec<String> = row.get(1);
    let to: Vec<String> = row.get(2);
    if from.len() != to.len() {
        return Err(AcceptanceReopenStoreError::InvalidControl);
    }
    Ok(InvReopenDependencyControl {
        reopen_check_ref: reopen_check_ref.to_owned(),
        dependency_graph_ref: row.get(0),
        dependency_edges: from.into_iter().zip(to).collect(),
        dependency_universe_refs: row.get(3),
        expected_unrelated_refs: row.get(4),
    })
}
