//! Durable carrier for the consumer-indexed relational comparator.
//! Distinct source revisions must already exist in the native generic/chat
//! stores. The database stores *candidates and residuals*; it does not store
//! admitted identity or replace the existing S29 correspondence review.
use postgres::{Client,GenericClient,NoTls};
use std::collections::BTreeMap;
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use thiserror::Error;
use crate::{
    DatabaseConfig,RelationalObservation,RelationalConsumer,
    RelationalComparison,RelationalComparisonError,
    compare_relational_observations, compare_relational_observations_strict,
    RELATIONAL_INTERLINGUA_SCHEMA, STRICT_RELATIONAL_INTERLINGUA_SCHEMA,
};

pub const RELATIONAL_COMPARISON_SQL:&str=r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.itir_relational_comparison (
    comparison_ref TEXT PRIMARY KEY,
    left_source_revision_ref TEXT NOT NULL,
    right_source_revision_ref TEXT NOT NULL,
    consumer_ref TEXT NOT NULL,
    packet_sha256 TEXT NOT NULL,
    packet_json TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK(candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK(NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK(NOT claim_truth_promoted),
    merges_sources BOOLEAN NOT NULL CHECK(NOT merges_sources),
    proves_independence BOOLEAN NOT NULL CHECK(NOT proves_independence),
    CHECK (left_source_revision_ref<>right_source_revision_ref)
);
CREATE INDEX IF NOT EXISTS itir_relational_comparison_scope_idx
 ON semantic.itir_relational_comparison (consumer_ref,left_source_revision_ref,right_source_revision_ref);
"#;
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub struct DurableRelationalComparison {
    pub left:RelationalObservation,
    pub right:RelationalObservation,
    pub left_source_content_sha256:String,
    pub right_source_content_sha256:String,
    pub consumer:RelationalConsumer,
    pub comparison:RelationalComparison,
}
#[derive(Debug,Error)]
pub enum DurableRelationalError {
    #[error(transparent)]
    Pg(#[from]postgres::Error),
    #[error(transparent)]
    Json(#[from]serde_json::Error),
    #[error(transparent)]
    Comparison(#[from]RelationalComparisonError),
    #[error("canonical native source revision is missing")]
    MissingSource,
    #[error("comparison replay is inconsistent with persisted witness and scope")]
    ChangedReplay,
}
fn exists(client:&mut impl GenericClient,name:&str)->Result<bool,postgres::Error>{
    Ok(client.query_one("SELECT to_regclass($1)::text",&[&name])?
        .get::<_,Option<String>>(0).is_some())
}
/// Reopen and digest the actual native source bytes rather than merely
/// confirming a claimed revision ID exists. Existing generic content
/// integrity is already checked by its owner; chat uses its canonical
/// payload through the document/canonical-content spine.
fn source_digest(client:&mut impl GenericClient,revision:&str,
    generic:bool,chat:bool)->Result<Option<String>,postgres::Error>{
    let generic_row=if generic{
        client.query_opt(
            "SELECT c.payload,r.content_digest_ref FROM ingest.generic_source_revision r
             JOIN corpus.document d ON d.document_ref=r.document_ref
             JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
             WHERE r.source_revision_ref=$1 AND r.candidate_only=TRUE
             AND r.creates_semantic_authority=FALSE
             AND r.applicability_promoted=FALSE
             AND r.claim_truth_promoted=FALSE",
            &[&revision])?
    }else{None};
    if let Some(row)=generic_row {
        let bytes:Vec<u8>=row.get(0);
        let expected:String=row.get(1);
        let computed=format!("sha256:{:x}",Sha256::digest(&bytes));
        // Source revision and canonical bytes must agree *before* they can
        // enter the relational acceptance artefact.
        return Ok((expected==computed).then_some(computed));
    }
    let row=if chat{
        client.query_opt(
            "SELECT c.payload FROM corpus.chat_archive_message m
             JOIN corpus.document d ON d.document_ref=m.document_ref
             JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
             WHERE m.source_revision_ref=$1 AND m.candidate_only=TRUE
             AND m.creates_semantic_authority=FALSE
             AND m.applicability_promoted=FALSE
             AND m.claim_truth_promoted=FALSE LIMIT 1",
            &[&revision])?
    }else{None};
    Ok(row.map(|r|{
        let bytes:Vec<u8>=r.get(0);
        format!("sha256:{:x}",Sha256::digest(&bytes))
    }))
}
fn sha(s:&str)->String {
    format!("sha256:{:x}",Sha256::digest(s.as_bytes()))
}
/// Reuse the exact digest owner for every pair in one transaction. The cache
/// is bound to this caller-owned snapshot and never survives its transaction.
pub(crate) struct RelationalSourceDigests {
    generic:bool,chat:bool,values:BTreeMap<String,String>,
}
impl RelationalSourceDigests {
    pub(crate) fn new(client:&mut impl GenericClient)->Result<Self,DurableRelationalError>{
        Ok(Self{generic:exists(client,"ingest.generic_source_revision")?,
            chat:exists(client,"corpus.chat_archive_message")?,values:BTreeMap::new()})
    }
    fn get(&mut self,client:&mut impl GenericClient,revision:&str)->Result<String,DurableRelationalError>{
        if let Some(value)=self.values.get(revision){return Ok(value.clone());}
        let value=source_digest(client,revision,self.generic,self.chat)?
            .ok_or(DurableRelationalError::MissingSource)?;
        self.values.insert(revision.into(),value.clone());
        Ok(value)
    }
}
fn recompute(schema:&str,left:&RelationalObservation,right:&RelationalObservation,
    consumer:&RelationalConsumer)->Result<RelationalComparison,DurableRelationalError>{
    Ok(match schema{
        RELATIONAL_INTERLINGUA_SCHEMA=>compare_relational_observations(left,right,consumer)?,
        STRICT_RELATIONAL_INTERLINGUA_SCHEMA=>compare_relational_observations_strict(left,right,consumer)?,
        _=>return Err(DurableRelationalError::ChangedReplay),
    })
}

/// Existing pair persistence owner, now with atomic insert/readback. Legacy
/// V1 IDs remain immutable; new strict families use V2 in this same table.
pub(crate) fn persist_relational_comparison_on_client(
    client:&mut impl GenericClient,digests:&mut RelationalSourceDigests,
    left:&RelationalObservation,right:&RelationalObservation,consumer:&RelationalConsumer,
    schema:&str,
)->Result<DurableRelationalComparison,DurableRelationalError>{
    let comparison=recompute(schema,left,right,consumer)?;
    if left.source_revision_ref==right.source_revision_ref{return Err(DurableRelationalError::ChangedReplay);}
    let result=DurableRelationalComparison{
        left:left.clone(),right:right.clone(),
        left_source_content_sha256:digests.get(client,&left.source_revision_ref)?,
        right_source_content_sha256:digests.get(client,&right.source_revision_ref)?,
        consumer:consumer.clone(),comparison,
    };
    let body=serde_json::to_string(&result)?;
    client.execute(
        "INSERT INTO semantic.itir_relational_comparison
         (comparison_ref,left_source_revision_ref,right_source_revision_ref,
         consumer_ref,packet_sha256,packet_json,candidate_only,
         creates_semantic_authority,claim_truth_promoted,merges_sources,proves_independence)
         VALUES($1,$2,$3,$4,$5,$6,TRUE,FALSE,FALSE,FALSE,FALSE)
         ON CONFLICT (comparison_ref) DO NOTHING",
        &[&result.comparison.comparison_ref,&left.source_revision_ref,
          &right.source_revision_ref,&consumer.consumer_ref,&sha(&body),&body],
    )?;
    let reopened=load_relational_comparison_on_client(client,digests,&result.comparison.comparison_ref)?
        .ok_or(DurableRelationalError::ChangedReplay)?;
    if result!=reopened{return Err(DurableRelationalError::ChangedReplay);}
    Ok(reopened)
}
pub fn persist_relational_comparison(
    config:&DatabaseConfig,left:&RelationalObservation,right:&RelationalObservation,
    consumer:&RelationalConsumer,
)->Result<DurableRelationalComparison,DurableRelationalError>{
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let mut tx=client.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ; SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
    tx.batch_execute(RELATIONAL_COMPARISON_SQL)?;
    let mut digests=RelationalSourceDigests::new(&mut tx)?;
    let result=persist_relational_comparison_on_client(&mut tx,&mut digests,
        left,right,consumer,RELATIONAL_INTERLINGUA_SCHEMA)?;
    tx.commit()?;
    Ok(result)
}

pub(crate) fn load_relational_comparison_on_client(
    client:&mut impl GenericClient,digests:&mut RelationalSourceDigests,comparison_ref:&str,
)->Result<Option<DurableRelationalComparison>,DurableRelationalError>{
    let Some(row)=client.query_opt(
        "SELECT left_source_revision_ref,right_source_revision_ref,
          consumer_ref,packet_sha256,packet_json,candidate_only,
          creates_semantic_authority,claim_truth_promoted,
          merges_sources,proves_independence
         FROM semantic.itir_relational_comparison WHERE comparison_ref=$1",
         &[&comparison_ref])? else {return Ok(None)};
    if !row.get::<_,bool>(5) || [6,7,8,9].iter().any(|i|row.get::<_,bool>(*i)) {
        return Err(DurableRelationalError::ChangedReplay);
    }
    let content:String=row.get(4);
    if sha(&content)!=row.get::<_,String>(3) {return Err(DurableRelationalError::ChangedReplay);}
    let data:DurableRelationalComparison=serde_json::from_str(&content)?;
    let computed=recompute(&data.comparison.schema,&data.left,&data.right,&data.consumer)?;
    if computed!=data.comparison || comparison_ref!=computed.comparison_ref
        || data.left.source_revision_ref!=row.get::<_,String>(0)
        || data.right.source_revision_ref!=row.get::<_,String>(1)
        || data.consumer.consumer_ref!=row.get::<_,String>(2)
        || data.left.source_revision_ref==data.right.source_revision_ref {
        return Err(DurableRelationalError::ChangedReplay);
    }
    for (rev,expected) in [
        (&data.left.source_revision_ref,&data.left_source_content_sha256),
        (&data.right.source_revision_ref,&data.right_source_content_sha256),
    ]{
        if digests.get(client,rev)?!=*expected{return Err(DurableRelationalError::ChangedReplay);}
    }
    Ok(Some(data))
}
pub fn load_relational_comparison(
    config:&DatabaseConfig,comparison_ref:&str,
)->Result<Option<DurableRelationalComparison>,DurableRelationalError>{
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let mut tx=client.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let mut digests=RelationalSourceDigests::new(&mut tx)?;
    let result=load_relational_comparison_on_client(&mut tx,&mut digests,comparison_ref)?;
    tx.commit()?;
    Ok(result)
}
