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
ALTER TABLE semantic.investigation_graph_binding
  ADD COLUMN IF NOT EXISTS legal_obligation_ref TEXT;
ALTER TABLE semantic.investigation_graph_binding
  ADD COLUMN IF NOT EXISTS acquisition_obligation_ref TEXT;
UPDATE semantic.investigation_graph_binding
   SET acquisition_obligation_ref=obligation_ref
 WHERE legal_obligation_ref IS NULL AND acquisition_obligation_ref IS NULL;
"#;

/// Install a two-owner foreign-key shape without relabelling a legal proof
/// obligation as a relational acquisition obligation. Legacy acquisition rows
/// retain their real acquisition FK through acquisition_obligation_ref.
pub const LEGAL_PROOF_GRAPH_BINDING_MIGRATION_SQL: &str = r#"
ALTER TABLE semantic.investigation_graph_binding
 DROP CONSTRAINT IF EXISTS investigation_graph_binding_obligation_ref_fkey;
DO $$
BEGIN
 IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='investigation_graph_binding_acquisition_owner_fk') THEN
  ALTER TABLE semantic.investigation_graph_binding ADD CONSTRAINT investigation_graph_binding_acquisition_owner_fk
    FOREIGN KEY (acquisition_obligation_ref) REFERENCES semantic.investigation_acquisition_obligation(obligation_ref);
 END IF;
 IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='investigation_graph_binding_legal_owner_fk') THEN
  ALTER TABLE semantic.investigation_graph_binding ADD CONSTRAINT investigation_graph_binding_legal_owner_fk
    FOREIGN KEY (legal_obligation_ref) REFERENCES legal_controversy.proof_obligation(obligation_ref);
 END IF;
 IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='investigation_graph_binding_exact_owner_ck') THEN
  ALTER TABLE semantic.investigation_graph_binding ADD CONSTRAINT investigation_graph_binding_exact_owner_ck CHECK (
   (acquisition_obligation_ref=obligation_ref AND legal_obligation_ref IS NULL)
   OR (legal_obligation_ref=obligation_ref AND acquisition_obligation_ref IS NULL));
 END IF;
END $$;
"#;

pub fn persist_legal_proof_investigation_graph_binding(
    config: &DatabaseConfig,
    binding: &InvestigationGraphBinding,
) -> Result<InvestigationGraphBinding, InvestigationGraphBindingStoreError> {
    validate_binding_shape(binding)?;
    crate::install_legal_controversy_schema(config)
        .map_err(|_| InvestigationGraphBindingStoreError::WrongOwner)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(INV_GRAPH_BINDING_SQL)?;
    client.batch_execute(LEGAL_PROOF_GRAPH_BINDING_MIGRATION_SQL)?;
    let owner = client.query_opt(
        "SELECT m.matter_ref FROM legal_controversy.proof_obligation o
         JOIN legal_controversy.matter m ON m.controversy_ref=o.controversy_ref
         WHERE o.obligation_ref=$1 AND o.is_open AND o.candidate_only
           AND NOT o.creates_semantic_authority AND NOT o.creates_legal_authority
           AND NOT o.applicability_promoted AND NOT o.claim_truth_promoted
           AND m.candidate_only AND NOT m.creates_semantic_authority
           AND NOT m.creates_legal_authority AND NOT m.applicability_promoted
           AND NOT m.claim_truth_promoted",
        &[&binding.obligation_ref],
    )?.ok_or(InvestigationGraphBindingStoreError::WrongOwner)?;
    if owner.get::<_, String>(0) != binding.matter_ref ||
       !persisted_projection_is_derived_challengeable(&mut client, &binding.projection_ref)? {
        return Err(InvestigationGraphBindingStoreError::WrongOwner);
    }
    let body = serde_json::to_string(binding)?;
    let hash = digest(&body);
    client.execute(
        "INSERT INTO semantic.investigation_graph_binding
          (binding_ref,obligation_ref,projection_ref,matter_ref,packet_sha256,packet_json,
           derived_only,challengeable,creates_graph_edges,creates_semantic_authority,
           creates_access_authority,creates_acquisition_state,legal_obligation_ref)
         VALUES($1,$2,$3,$4,$5,$6,TRUE,TRUE,FALSE,FALSE,FALSE,FALSE,$2)
         ON CONFLICT(binding_ref) DO NOTHING",
        &[&binding.binding_ref,&binding.obligation_ref,&binding.projection_ref,
          &binding.matter_ref,&hash,&body],
    )?;
    let reopened = load_investigation_graph_binding(config, &binding.binding_ref)?
        .ok_or(InvestigationGraphBindingStoreError::ChangedReplay)?;
    if reopened != *binding {
        return Err(InvestigationGraphBindingStoreError::ChangedReplay);
    }
    Ok(reopened)
}

fn validate_binding_shape(binding: &InvestigationGraphBinding)
    -> Result<(), InvestigationGraphBindingStoreError>
{
    if binding.schema != INV_GRAPH_BINDING_SCHEMA
        || binding.binding_ref.trim().is_empty()
        || binding.obligation_ref.trim().is_empty()
        || binding.projection_ref.trim().is_empty()
        || binding.matter_ref.trim().is_empty()
        || binding.binding_evidence_refs.is_empty()
        || binding.binding_evidence_refs.iter().any(|r| r.trim().is_empty())
        || !binding.derived_only || !binding.challengeable
        || binding.creates_graph_edges || binding.creates_semantic_authority
        || binding.creates_access_authority || binding.creates_acquisition_state {
        return Err(InvestigationGraphBindingStoreError::WrongOwner);
    }
    Ok(())
}

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
          creates_access_authority, creates_acquisition_state, acquisition_obligation_ref)
         VALUES($1,$2,$3,$4,$5,$6,TRUE,TRUE,FALSE,FALSE,FALSE,FALSE,$2)
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
    validate_binding_shape(&binding)?;
    let owner_row = client.query_one(
        "SELECT legal_obligation_ref, acquisition_obligation_ref, matter_ref, projection_ref
         FROM semantic.investigation_graph_binding WHERE binding_ref=$1",
        &[&binding_ref],
    )?;
    let legal_ref: Option<String> = owner_row.get(0);
    let acquisition_ref: Option<String> = owner_row.get(1);
    if owner_row.get::<_,String>(2)!=binding.matter_ref
      || owner_row.get::<_,String>(3)!=binding.projection_ref { return Err(InvestigationGraphBindingStoreError::WrongOwner); }
    match (legal_ref, acquisition_ref) {
        (Some(legal), None) if legal == binding.obligation_ref => {
            let owner = client.query_opt(
                "SELECT m.matter_ref FROM legal_controversy.proof_obligation o
                 JOIN legal_controversy.matter m ON m.controversy_ref=o.controversy_ref
                 WHERE o.obligation_ref=$1 AND o.is_open AND o.candidate_only
                   AND NOT o.creates_semantic_authority AND NOT o.creates_legal_authority
                   AND NOT o.applicability_promoted AND NOT o.claim_truth_promoted",
                &[&legal],
            )?.ok_or(InvestigationGraphBindingStoreError::WrongOwner)?;
            if owner.get::<_,String>(0)!=binding.matter_ref {
                return Err(InvestigationGraphBindingStoreError::WrongOwner);
            }
        }
        (None, Some(acquisition)) if acquisition == binding.obligation_ref => {
            load_acquisition_queue(config, &acquisition)?
                .ok_or(InvestigationGraphBindingStoreError::WrongOwner)?;
        }
        _ => return Err(InvestigationGraphBindingStoreError::WrongOwner),
    }
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
    let rows = client.query(
        "SELECT binding_ref FROM semantic.investigation_graph_binding
         WHERE obligation_ref=$1 ORDER BY binding_ref LIMIT 2",
        &[&obligation_ref],
    )?;
    if rows.len() > 1 {
        // Never pick an arbitrary stale graph after a review/revision change.
        return Err(InvestigationGraphBindingStoreError::ChangedReplay);
    }
    let Some(row) = rows.first() else { return Ok(None); };
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
