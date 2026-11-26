//! Durable ITIR-INV-1 acquisition queue.
//!
//! Stores acquisition *obligations, candidate routes and Pareto receipts* as
//! lower-authority sidecars over an existing persisted REL comparison.
//! It never creates a source, performs network access, or admits evidence.

use postgres::{Client,GenericClient,NoTls};
use std::collections::BTreeSet;
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use thiserror::Error;
use crate::relational_interlingua_store::{RelationalSourceDigests,load_relational_comparison_on_client};
use crate::{
    acquisition_pareto_frontier, apply_acquisition_update,
    selective_reopening,
    AcquisitionObligation,AcquisitionPriorityReceipt,AcquisitionRouteCandidate,
    AcquisitionUpdate,DatabaseConfig,DurableRelationalError,
    InvestigationAcquisitionError,RecordAvailability,SelectiveReopeningReceipt,
    DurableRelationalComparison,
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
CREATE TABLE IF NOT EXISTS semantic.investigation_acquisition_update (
    obligation_ref TEXT NOT NULL
      REFERENCES semantic.investigation_acquisition_obligation(obligation_ref),
    acquisition_receipt_ref TEXT NOT NULL,
    packet_sha256 TEXT NOT NULL,
    packet_json TEXT NOT NULL,
    PRIMARY KEY(obligation_ref,acquisition_receipt_ref)
);
CREATE TABLE IF NOT EXISTS semantic.investigation_selective_reopening (
    obligation_ref TEXT NOT NULL
      REFERENCES semantic.investigation_acquisition_obligation(obligation_ref),
    acquisition_receipt_ref TEXT NOT NULL,
    packet_sha256 TEXT NOT NULL,
    packet_json TEXT NOT NULL,
    creates_semantic_authority BOOLEAN NOT NULL CHECK(NOT creates_semantic_authority),
    PRIMARY KEY(obligation_ref,acquisition_receipt_ref)
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

fn validate_owner(obligation:&AcquisitionObligation,comparison:&DurableRelationalComparison)
    ->Result<(),InvestigationStoreError>{
    if comparison.comparison.comparison_ref!=obligation.comparison_ref
        ||!comparison.comparison.residuals.iter().any(|r|r.obligation_ref==obligation.residual_obligation_ref)
        ||obligation.source_revision_refs!={
            let mut refs=vec![comparison.left.source_revision_ref.clone(),comparison.right.source_revision_ref.clone()];
            refs.sort();refs.dedup();refs
        }
        ||!obligation.candidate_only||obligation.creates_semantic_authority||obligation.claim_truth_promoted
        ||obligation.obligation_ref.trim().is_empty()
        ||obligation.target_description.trim().is_empty()
        ||obligation.authority_or_access_constraint_ref.trim().is_empty()
        ||obligation.current_availability!=RecordAvailability::NotLocated
    {return Err(InvestigationStoreError::WrongOwner);}
    Ok(())
}

/// Lock the canonical obligation row and verify exact replay before any
/// route or priority mutation. The caller owns the transaction.
fn insert_and_lock_obligation(client:&mut impl GenericClient,obligation:&AcquisitionObligation)
    ->Result<bool,InvestigationStoreError>{
    let body=serde_json::to_string(obligation)?;
    let inserted=client.execute(
        "INSERT INTO semantic.investigation_acquisition_obligation
         (obligation_ref,comparison_ref,packet_sha256,packet_json,
          candidate_only,creates_semantic_authority,claim_truth_promoted)
         VALUES($1,$2,$3,$4,TRUE,FALSE,FALSE)
         ON CONFLICT(obligation_ref) DO NOTHING",
        &[&obligation.obligation_ref,&obligation.comparison_ref,&digest(&body),&body],
    )?==1;
    let row=client.query_one(
        "SELECT comparison_ref,packet_sha256,packet_json,candidate_only,
                creates_semantic_authority,claim_truth_promoted
         FROM semantic.investigation_acquisition_obligation WHERE obligation_ref=$1 FOR UPDATE",
        &[&obligation.obligation_ref])?;
    if row.get::<_,String>(0)!=obligation.comparison_ref
        ||row.get::<_,String>(1)!=digest(&body)||row.get::<_,String>(2)!=body
        ||!row.get::<_,bool>(3)||row.get::<_,bool>(4)||row.get::<_,bool>(5){
        return Err(InvestigationStoreError::ChangedReplay);
    }
    Ok(inserted)
}

fn write_routes_and_priority(client:&mut impl GenericClient,obligation:&AcquisitionObligation,
    routes:&[AcquisitionRouteCandidate])->Result<DurableAcquisitionQueue,InvestigationStoreError>{
    if routes.iter().map(|r|&r.route_ref).collect::<BTreeSet<_>>().len()!=routes.len(){
        return Err(InvestigationStoreError::ChangedReplay);
    }
    let mut routes=routes.to_vec();routes.sort_by(|a,b|a.route_ref.cmp(&b.route_ref));
    let priority=acquisition_pareto_frontier(obligation,&routes)?;
    for route in &routes{
        let body=serde_json::to_string(route)?;
        client.execute(
            "INSERT INTO semantic.investigation_acquisition_route
             (route_ref,obligation_ref,packet_sha256,packet_json,candidate_only,creates_acquisition_authority)
             VALUES($1,$2,$3,$4,TRUE,FALSE) ON CONFLICT(route_ref) DO NOTHING",
            &[&route.route_ref,&route.obligation_ref,&digest(&body),&body],
        )?;
    }
    let priority_json=serde_json::to_string(&priority)?;
    client.execute(
        "INSERT INTO semantic.investigation_acquisition_priority
         (obligation_ref,packet_sha256,packet_json,scalar_score_used,creates_semantic_authority,creates_decision)
         VALUES($1,$2,$3,FALSE,FALSE,FALSE)
         ON CONFLICT(obligation_ref) DO UPDATE SET
           packet_sha256=EXCLUDED.packet_sha256,packet_json=EXCLUDED.packet_json
         WHERE semantic.investigation_acquisition_priority.packet_sha256<>EXCLUDED.packet_sha256",
        &[&obligation.obligation_ref,&digest(&priority_json),&priority_json],
    )?;
    let reopened=load_acquisition_queue_rows(client,&obligation.obligation_ref)?
        .ok_or(InvestigationStoreError::ChangedReplay)?;
    let expected=DurableAcquisitionQueue{obligation:obligation.clone(),routes,priority};
    if reopened!=expected{return Err(InvestigationStoreError::ChangedReplay);}
    Ok(reopened)
}

/// Family replay must never replace an existing acquisition frontier with
/// an empty one. Only a brand-new obligation receives an empty route set.
/// Parent comparison has just been validated in the same transaction.
pub(crate) fn ensure_acquisition_obligation_on_client(
    client:&mut impl GenericClient,obligation:&AcquisitionObligation,
    parent:&DurableRelationalComparison,
)->Result<DurableAcquisitionQueue,InvestigationStoreError>{
    validate_owner(obligation,parent)?;
    if insert_and_lock_obligation(client,obligation)?{
        write_routes_and_priority(client,obligation,&[])
    }else{
        let existing=load_acquisition_queue_rows(client,&obligation.obligation_ref)?
            .ok_or(InvestigationStoreError::ChangedReplay)?;
        if existing.obligation!=*obligation{return Err(InvestigationStoreError::ChangedReplay);}
        Ok(existing)
    }
}

pub fn persist_acquisition_queue(
    config:&DatabaseConfig,obligation:&AcquisitionObligation,routes:&[AcquisitionRouteCandidate],
)->Result<DurableAcquisitionQueue,InvestigationStoreError>{
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let mut tx=client.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ; SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
    let mut digests=RelationalSourceDigests::new(&mut tx)?;
    let parent=load_relational_comparison_on_client(&mut tx,&mut digests,&obligation.comparison_ref)?
        .ok_or(InvestigationStoreError::WrongOwner)?;
    validate_owner(obligation,&parent)?;
    tx.batch_execute(INVESTIGATION_ACQUISITION_SQL)?;
    insert_and_lock_obligation(&mut tx,obligation)?;
    let result=write_routes_and_priority(&mut tx,obligation,routes)?;
    tx.commit()?;
    Ok(result)
}

fn source_revision_exists(
    client:&mut impl GenericClient,revision:&str,
)->Result<bool,postgres::Error>{
    if client.query_opt(
        "SELECT 1 FROM ingest.generic_source_revision
         WHERE source_revision_ref=$1 AND candidate_only=TRUE
           AND creates_semantic_authority=FALSE
           AND applicability_promoted=FALSE AND claim_truth_promoted=FALSE",
        &[&revision])?.is_some(){return Ok(true);}
    Ok(client.query_opt(
        "SELECT 1 FROM corpus.chat_archive_message
         WHERE source_revision_ref=$1 AND candidate_only=TRUE
           AND creates_semantic_authority=FALSE
           AND applicability_promoted=FALSE AND claim_truth_promoted=FALSE
         LIMIT 1",
        &[&revision])?.is_some())
}

/// Persist an acquisition result only after the source is already ingested by
/// its native adapter. Known-absent closes the exact branch and produces no
/// selective-reopening receipt.
pub fn persist_acquisition_update(
    config:&DatabaseConfig,
    update:&AcquisitionUpdate,
    dependency_graph_ref:&str,
    dependency_edges:&[(String,String)],
    dependency_universe_refs:&[String],
)->Result<Option<SelectiveReopeningReceipt>,InvestigationStoreError>{
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut client=connection.transaction()?;
    client.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ; SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
    client.batch_execute(INVESTIGATION_ACQUISITION_SQL)?;
    let queue=load_acquisition_queue_rows(&mut client,&update.obligation_ref)?
        .ok_or(InvestigationStoreError::WrongOwner)?;
    insert_and_lock_obligation(&mut client,&queue.obligation)?;
    let mut digests=RelationalSourceDigests::new(&mut client)?;
    let parent=load_relational_comparison_on_client(&mut client,&mut digests,&queue.obligation.comparison_ref)?
        .ok_or(InvestigationStoreError::WrongOwner)?;
    validate_owner(&queue.obligation,&parent)?;
    apply_acquisition_update(&queue.obligation,update)?;

    if update.after==RecordAvailability::Present{
        let revision=update.acquired_source_revision_ref.as_ref()
            .ok_or(InvestigationStoreError::ChangedReplay)?;
        if !source_revision_exists(&mut client,revision)?{
            return Err(InvestigationStoreError::WrongOwner);
        }
    }

    let body=serde_json::to_string(update)?;
    client.execute(
        "INSERT INTO semantic.investigation_acquisition_update
         (obligation_ref,acquisition_receipt_ref,packet_sha256,packet_json)
         VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING",
        &[&update.obligation_ref,&update.acquisition_receipt_ref,
          &digest(&body),&body],
    )?;
    let row=client.query_one(
        "SELECT packet_sha256,packet_json
         FROM semantic.investigation_acquisition_update
         WHERE obligation_ref=$1 AND acquisition_receipt_ref=$2",
        &[&update.obligation_ref,&update.acquisition_receipt_ref])?;
    let reopened_json:String=row.get(1);
    if digest(&reopened_json)!=row.get::<_,String>(0)
        ||serde_json::from_str::<AcquisitionUpdate>(&reopened_json)?!=*update{
        return Err(InvestigationStoreError::ChangedReplay);
    }

    if update.after==RecordAvailability::KnownAbsent{
        client.commit()?;
        return Ok(None);
    }
    let revision=update.acquired_source_revision_ref.as_ref().unwrap();
    let reopening=selective_reopening(
        revision,dependency_graph_ref,dependency_edges,dependency_universe_refs)?;
    let reopening_json=serde_json::to_string(&reopening)?;
    client.execute(
        "INSERT INTO semantic.investigation_selective_reopening
         (obligation_ref,acquisition_receipt_ref,packet_sha256,packet_json,
          creates_semantic_authority)
         VALUES($1,$2,$3,$4,FALSE) ON CONFLICT DO NOTHING",
        &[&update.obligation_ref,&update.acquisition_receipt_ref,
          &digest(&reopening_json),&reopening_json],
    )?;
    let row=client.query_one(
        "SELECT packet_sha256,packet_json,creates_semantic_authority
         FROM semantic.investigation_selective_reopening
         WHERE obligation_ref=$1 AND acquisition_receipt_ref=$2",
        &[&update.obligation_ref,&update.acquisition_receipt_ref])?;
    if row.get::<_,String>(0)!=digest(&reopening_json)||row.get::<_,String>(1)!=reopening_json
        ||row.get::<_,bool>(2){return Err(InvestigationStoreError::ChangedReplay);}
    client.commit()?;
    Ok(Some(reopening))
}

fn load_acquisition_queue_rows(
    client:&mut impl GenericClient,obligation_ref:&str,
)->Result<Option<DurableAcquisitionQueue>,InvestigationStoreError>{
    let Some(obligation_row)=client.query_opt(
        "SELECT packet_sha256,packet_json,candidate_only,
                creates_semantic_authority,claim_truth_promoted,comparison_ref
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
    if obligation.obligation_ref!=obligation_ref||obligation.comparison_ref!=obligation_row.get::<_,String>(5){
        return Err(InvestigationStoreError::ChangedReplay);
    }

    let mut routes=Vec::new();
    for row in client.query(
        "SELECT packet_sha256,packet_json,candidate_only,
                creates_acquisition_authority,route_ref
         FROM semantic.investigation_acquisition_route
         WHERE obligation_ref=$1 ORDER BY route_ref",&[&obligation_ref])?{
        if !row.get::<_,bool>(2)||row.get::<_,bool>(3){
            return Err(InvestigationStoreError::ChangedReplay);
        }
        let body:String=row.get(1);
        if digest(&body)!=row.get::<_,String>(0){
            return Err(InvestigationStoreError::ChangedReplay);
        }
        let route:AcquisitionRouteCandidate=serde_json::from_str(&body)?;
        if route.route_ref!=row.get::<_,String>(4)||route.obligation_ref!=obligation_ref{
            return Err(InvestigationStoreError::ChangedReplay);
        }
        routes.push(route);
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

    Ok(Some(DurableAcquisitionQueue{obligation,routes,priority}))
}
pub fn load_acquisition_queue(
    config:&DatabaseConfig,obligation_ref:&str,
)->Result<Option<DurableAcquisitionQueue>,InvestigationStoreError>{
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let mut tx=client.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let queue=load_acquisition_queue_rows(&mut tx,obligation_ref)?;
    if let Some(ref q)=queue{
        let mut digests=RelationalSourceDigests::new(&mut tx)?;
        let parent=load_relational_comparison_on_client(&mut tx,&mut digests,&q.obligation.comparison_ref)?
            .ok_or(InvestigationStoreError::WrongOwner)?;
        validate_owner(&q.obligation,&parent)?;
    }
    tx.commit()?;
    Ok(queue)
}
