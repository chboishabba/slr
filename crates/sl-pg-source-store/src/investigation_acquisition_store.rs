//! Durable ITIR-INV-1 acquisition queue.
//!
//! Stores acquisition *obligations, candidate routes and Pareto receipts* as
//! lower-authority sidecars over an existing persisted REL comparison.
//! It never creates a source, performs network access, or admits evidence.

use postgres::{Client,NoTls};
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use thiserror::Error;
use crate::{
    acquisition_pareto_frontier, load_relational_comparison,
    AcquisitionObligation,AcquisitionPriorityReceipt,AcquisitionRouteCandidate,
    DatabaseConfig,DurableRelationalError,InvestigationAcquisitionError,
};

pub const INVESTIGATION_ACQUISITION_SQL:&str=r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.investigation_acquisition_obligation (
    obligation_ref TEXT PRIMARY KEY,
    comparison_ref TEXT NOT NULL
      REFERENCES semantic.itir_relational_comparison(comparison_ref),
    packet_sha256 TEXT NOT NULL,
    packet_json TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK(candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK(NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK(NOT claim_truth_promoted)
);
CREATE TABLE IF NOT EXISTS semantic.investigation_acquisition_route (
    route_ref TEXT PRIMARY KEY,
    obligation_ref TEXT NOT NULL
      REFERENCES semantic.investigation_acquisition_obligation(obligation_ref),
    packet_sha256 TEXT NOT NULL,
    packet_json TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK(candidate_only),
    creates_acquisition_authority BOOLEAN NOT NULL
      CHECK(NOT creates_acquisition_authority)
);
CREATE TABLE IF NOT EXISTS semantic.investigation_acquisition_priority (
    obligation_ref TEXT PRIMARY KEY
      REFERENCES semantic.investigation_acquisition_obligation(obligation_ref),
    packet_sha256 TEXT NOT NULL,
    packet_json TEXT NOT NULL,
    scalar_score_used BOOLEAN NOT NULL CHECK(NOT scalar_score_used),
    creates_semantic_authority BOOLEAN NOT NULL CHECK(NOT creates_semantic_authority),
    creates_decision BOOLEAN NOT NULL CHECK(NOT creates_decision)
);
"#;

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct DurableAcquisitionQueue {
    pub obligation:AcquisitionObligation,
    pub routes:Vec<AcquisitionRouteCandidate>,
    pub priority:AcquisitionPriorityReceipt,
}
#[derive(Debug,Error)]
pub enum InvestigationStoreError {
    #[error(transparent)]
    Pg(#[from]postgres::Error),
    #[error(transparent)]
    Json(#[from]serde_json::Error),
    #[error(transparent)]
    Comparison(#[from]DurableRelationalError),
    #[error(transparent)]
    Acquisition(#[from]InvestigationAcquisitionError),
    #[error("comparison or residual owner is missing/inconsistent")]
    WrongOwner,
    #[error("changed replay under immutable acquisition identity")]
    ChangedReplay,
}
fn digest(s:&str)->String{
    format!("sha256:{:x}",Sha256::digest(s.as_bytes()))
}

pub fn persist_acquisition_queue(
    config:&DatabaseConfig,
    obligation:&AcquisitionObligation,
    routes:&[AcquisitionRouteCandidate],
)->Result<DurableAcquisitionQueue,InvestigationStoreError>{
    let comparison=load_relational_comparison(config,&obligation.comparison_ref)?
        .ok_or(InvestigationStoreError::WrongOwner)?;
    if !comparison.comparison.residuals.iter()
        .any(|r|r.obligation_ref==obligation.residual_obligation_ref)
        || obligation.source_revision_refs!={
            let mut refs=vec![
                comparison.comparison.left_observation_ref.clone(),
                comparison.comparison.right_observation_ref.clone(),
            ];
            refs.sort();refs.dedup();refs
        }
    {return Err(InvestigationStoreError::WrongOwner);}

    let priority=acquisition_pareto_frontier(obligation,routes)?;
    let mut client=Client::connect(config.database_url(),NoTls)?;
    client.batch_execute(INVESTIGATION_ACQUISITION_SQL)?;

    let obligation_json=serde_json::to_string(obligation)?;
    client.execute(
        "INSERT INTO semantic.investigation_acquisition_obligation
         (obligation_ref,comparison_ref,packet_sha256,packet_json,
          candidate_only,creates_semantic_authority,claim_truth_promoted)
         VALUES($1,$2,$3,$4,TRUE,FALSE,FALSE)
         ON CONFLICT(obligation_ref) DO NOTHING",
        &[&obligation.obligation_ref,&obligation.comparison_ref,
          &digest(&obligation_json),&obligation_json],
    )?;

    for route in routes{
        let body=serde_json::to_string(route)?;
        client.execute(
            "INSERT INTO semantic.investigation_acquisition_route
             (route_ref,obligation_ref,packet_sha256,packet_json,
              candidate_only,creates_acquisition_authority)
             VALUES($1,$2,$3,$4,TRUE,FALSE)
             ON CONFLICT(route_ref) DO NOTHING",
            &[&route.route_ref,&route.obligation_ref,&digest(&body),&body],
        )?;
    }

    let priority_json=serde_json::to_string(&priority)?;
    client.execute(
        "INSERT INTO semantic.investigation_acquisition_priority
         (obligation_ref,packet_sha256,packet_json,scalar_score_used,
          creates_semantic_authority,creates_decision)
         VALUES($1,$2,$3,FALSE,FALSE,FALSE)
         ON CONFLICT(obligation_ref) DO UPDATE SET
           packet_sha256=EXCLUDED.packet_sha256,
           packet_json=EXCLUDED.packet_json",
        &[&obligation.obligation_ref,&digest(&priority_json),&priority_json],
    )?;

    let reopened=load_acquisition_queue(config,&obligation.obligation_ref)?
        .ok_or(InvestigationStoreError::ChangedReplay)?;
    let expected=DurableAcquisitionQueue{
        obligation:obligation.clone(),routes:routes.to_vec(),priority,
    };
    if reopened!=expected{return Err(InvestigationStoreError::ChangedReplay);}
    Ok(reopened)
}

pub fn load_acquisition_queue(
    config:&DatabaseConfig,obligation_ref:&str,
)->Result<Option<DurableAcquisitionQueue>,InvestigationStoreError>{
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let Some(obligation_row)=client.query_opt(
        "SELECT packet_sha256,packet_json,candidate_only,
                creates_semantic_authority,claim_truth_promoted
         FROM semantic.investigation_acquisition_obligation
         WHERE obligation_ref=$1",&[&obligation_ref])?
    else{return Ok(None)};
    if !obligation_row.get::<_,bool>(2)
        ||obligation_row.get::<_,bool>(3)||obligation_row.get::<_,bool>(4){
        return Err(InvestigationStoreError::ChangedReplay);
    }
    let obligation_json:String=obligation_row.get(1);
    if digest(&obligation_json)!=obligation_row.get::<_,String>(0){
        return Err(InvestigationStoreError::ChangedReplay);
    }
    let obligation:AcquisitionObligation=serde_json::from_str(&obligation_json)?;

    let mut routes=Vec::new();
    for row in client.query(
        "SELECT packet_sha256,packet_json,candidate_only,
                creates_acquisition_authority
         FROM semantic.investigation_acquisition_route
         WHERE obligation_ref=$1 ORDER BY route_ref",&[&obligation_ref])?{
        if !row.get::<_,bool>(2)||row.get::<_,bool>(3){
            return Err(InvestigationStoreError::ChangedReplay);
        }
        let body:String=row.get(1);
        if digest(&body)!=row.get::<_,String>(0){
            return Err(InvestigationStoreError::ChangedReplay);
        }
        routes.push(serde_json::from_str(&body)?);
    }
    let priority_row=client.query_one(
        "SELECT packet_sha256,packet_json,scalar_score_used,
                creates_semantic_authority,creates_decision
         FROM semantic.investigation_acquisition_priority
         WHERE obligation_ref=$1",&[&obligation_ref])?;
    if priority_row.get::<_,bool>(2)
        ||priority_row.get::<_,bool>(3)||priority_row.get::<_,bool>(4){
        return Err(InvestigationStoreError::ChangedReplay);
    }
    let priority_json:String=priority_row.get(1);
    if digest(&priority_json)!=priority_row.get::<_,String>(0){
        return Err(InvestigationStoreError::ChangedReplay);
    }
    let priority:AcquisitionPriorityReceipt=serde_json::from_str(&priority_json)?;
    if priority!=acquisition_pareto_frontier(&obligation,&routes)?{
        return Err(InvestigationStoreError::ChangedReplay);
    }

    // Reopen the parent comparison to ensure this lower-authority queue cannot
    // outlive or silently rebind to another semantic candidate.
    let comparison=load_relational_comparison(config,&obligation.comparison_ref)?
        .ok_or(InvestigationStoreError::WrongOwner)?;
    if !comparison.comparison.residuals.iter()
        .any(|r|r.obligation_ref==obligation.residual_obligation_ref){
        return Err(InvestigationStoreError::WrongOwner);
    }
    Ok(Some(DurableAcquisitionQueue{obligation,routes,priority}))
}
