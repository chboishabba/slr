//! Durable carrier for the consumer-indexed relational comparator.
//! Distinct source revisions must already exist in the native generic/chat
//! stores. The database stores *candidates and residuals*; it does not store
//! admitted identity or replace the existing S29 correspondence review.
use postgres::{Client,NoTls};
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use thiserror::Error;
use crate::{
    DatabaseConfig,RelationalObservation,RelationalConsumer,
    RelationalComparison,RelationalComparisonError,
    compare_relational_observations,
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
fn exists(client:&mut Client,name:&str)->Result<bool,postgres::Error>{
    Ok(client.query_one("SELECT to_regclass($1)::text",&[&name])?
        .get::<_,Option<String>>(0).is_some())
}
/// Reopen and digest the actual native source bytes rather than merely
/// confirming a claimed revision ID exists. Existing generic content
/// integrity is already checked by its owner; chat uses its canonical
/// payload through the document/canonical-content spine.
fn source_digest(client:&mut Client,revision:&str,
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
/// Native SQL source identity check at *both* load and insert. Presence
/// alone does not authenticate parser provenance or consumer permissions.
pub fn persist_relational_comparison(
    config:&DatabaseConfig,left:&RelationalObservation,
    right:&RelationalObservation,consumer:&RelationalConsumer,
)->Result<DurableRelationalComparison,DurableRelationalError>{
    let comparison=compare_relational_observations(left,right,consumer)?;
    if left.source_revision_ref==right.source_revision_ref {
        return Err(DurableRelationalError::ChangedReplay);
    }
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let generic=exists(&mut client,"ingest.generic_source_revision")?;
    let chat=exists(&mut client,"corpus.chat_archive_message")?;
    let left_digest=source_digest(&mut client,&left.source_revision_ref,generic,chat)?
        .ok_or(DurableRelationalError::MissingSource)?;
    let right_digest=source_digest(&mut client,&right.source_revision_ref,generic,chat)?
        .ok_or(DurableRelationalError::MissingSource)?;
    let result=DurableRelationalComparison{
        left:left.clone(),right:right.clone(),
        left_source_content_sha256:left_digest,
        right_source_content_sha256:right_digest,
        consumer:consumer.clone(),comparison,
    };
    client.batch_execute(RELATIONAL_COMPARISON_SQL)?;
    let body=serde_json::to_string(&result)?;
    client.execute(
        "INSERT INTO semantic.itir_relational_comparison
         (comparison_ref,left_source_revision_ref,right_source_revision_ref,
         consumer_ref,packet_sha256,packet_json,candidate_only,
         creates_semantic_authority,claim_truth_promoted,merges_sources,
         proves_independence)
         VALUES($1,$2,$3,$4,$5,$6,TRUE,FALSE,FALSE,FALSE,FALSE)
         ON CONFLICT (comparison_ref) DO NOTHING",
        &[&result.comparison.comparison_ref,&left.source_revision_ref,
          &right.source_revision_ref,&consumer.consumer_ref,
          &sha(&body),&body],
    )?;
    let reopened=load_relational_comparison(config,&result.comparison.comparison_ref)?
        .ok_or(DurableRelationalError::ChangedReplay)?;
    if result!=reopened {return Err(DurableRelationalError::ChangedReplay);}
    Ok(reopened)
}
pub fn load_relational_comparison(
    config:&DatabaseConfig,comparison_ref:&str,
)->Result<Option<DurableRelationalComparison,DurableRelationalError>{
    let mut client=Client::connect(config.database_url(),NoTls)?;
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
    if sha(&content)!=row.get::<_,String>(3) {
        return Err(DurableRelationalError::ChangedReplay);
    }
    let data:DurableRelationalComparison=serde_json::from_str(&content)?;
    let computed=compare_relational_observations(
        &data.left,&data.right,&data.consumer)?;
    if computed!=data.comparison || comparison_ref!=computed.comparison_ref
        || data.left.source_revision_ref!=row.get::<_,String>(0)
        || data.right.source_revision_ref!=row.get::<_,String>(1)
        || data.consumer.consumer_ref!=row.get::<_,String>(2)
        || data.left.source_revision_ref==data.right.source_revision_ref {
        return Err(DurableRelationalError::ChangedReplay);
    }
    let generic=exists(&mut client,"ingest.generic_source_revision")?;
    let chat=exists(&mut client,"corpus.chat_archive_message")?;
    for (rev,expected) in [
        (&data.left.source_revision_ref,&data.left_source_content_sha256),
        (&data.right.source_revision_ref,&data.right_source_content_sha256),
    ]{
        let actual=source_digest(&mut client,rev,generic,chat)?
            .ok_or(DurableRelationalError::MissingSource)?;
        if actual!=*expected{
            return Err(DurableRelationalError::ChangedReplay);
        }
    }
    Ok(Some(data))
}
