//! Explicit no-observation endpoint replay. This is a conservative
//! execution fence, not a new semantic cache authority. Source and numeric
//! databases may be separate; two-sided stamp collection gives a common
//! observation instant without comparing clocks.
use crate::DatabaseConfig;
use postgres::{Client,NoTls};
use serde_json::Value;
use sha2::{Digest,Sha256};
use std::path::{Path,PathBuf};
use thiserror::Error;

#[derive(Debug,Error)]
pub enum NativeEndpointReplayError {
    #[error(transparent)]
    Database(#[from]postgres::Error),
    #[error(transparent)]
    File(#[from]std::io::Error),
    #[error(transparent)]
    Json(#[from]serde_json::Error),
    #[error("native endpoint replay contract or cached payload mismatch")]
    Contract,
}
type Result<T>=std::result::Result<T,NativeEndpointReplayError>;

fn digest(v:&[u8])->String{format!("{:x}",Sha256::digest(v))}
fn file_digest(p:&Path)->Result<String>{Ok(digest(&std::fs::read(p)?))}

fn implementation_fingerprint()->Result<Option<Value>>{
    let Some(script)=std::env::var_os("SLR_NATIVE_LEAN_WORKSET_EXECUTABLE") else{return Ok(None)};
    let script=PathBuf::from(script).canonicalize()?;
    let Some(root)=script.parent().and_then(Path::parent) else{return Ok(None)};
    let files=[
        "lean-toolchain",
        "scripts/produce_native_pnf_workset.sh",
        "tools/NativePnfWorkset.lean",
        "DASHI/output-final_aristotle/RequestProject/Slr/NumericPnfRustPortContract.lean",
        ".lake/build/lib/lean/RequestProject/Slr/NumericPnfRustPortContract.olean",
        "DASHI/output-final_aristotle/RequestProject/Slr/GwbCoreferenceFormalRepair.lean",
        ".lake/build/lib/lean/RequestProject/Slr/GwbCoreferenceFormalRepair.olean",
    ];
    let mut pinned=std::collections::BTreeMap::new();
    for name in files{
        let p=root.join(name);
        if !p.is_file(){return Ok(None);}
        pinned.insert(name,file_digest(&p)?);
    }
    let source_root=std::env::var_os("SLR_NATIVE_NUMERIC_SOURCE_ROOT")
        .map(PathBuf::from).map(|p|p.canonicalize()).transpose()?;
    Ok(Some(serde_json::json!({
        "schema":"slr-native-endpoint-replay-implementation:v1",
        "native_binary":file_digest(&std::env::current_exe()?)?,
        "golden_files":pinned,"golden_script":file_digest(&script)?,
        "source_root":source_root,
        "candidate_only":true
    })))
}

fn install(url:&str)->Result<bool>{
    let mut db=Client::connect(url,NoTls)?;
    let mut tx=db.transaction()?;
    tx.batch_execute("SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
    // The event-trigger continuity fence is deliberately opt-in and needs
    // the local DDL owner. Do not attempt privileged installation as a
    // normal application role or silently weaken the proof.
    let ddl_owner:bool=tx.query_one("SELECT rolsuper FROM pg_roles WHERE rolname=current_user",&[])?.get(0);
    if !ddl_owner{return Ok(false);}
    tx.query_one("SELECT pg_advisory_xact_lock(hashtext('slr-native-endpoint-replay:v1'))",&[])?;
    let installed:bool=tx.query_one("SELECT to_regprocedure('slr_native_replay.current_stamp()') IS NOT NULL",&[])?.get(0);
    if !installed{tx.batch_execute(include_str!("numeric_pnf_endpoint_replay.sql"))?;}
    let ready:bool=tx.query_one("SELECT slr_native_replay.fence_intact()",&[])?.get(0);
    let ready=if ready{true}else{tx.query_one("SELECT slr_native_replay.install_fence()",&[])?.get(0)};
    if ready && stamp_on(&mut tx)?.is_none(){
        // A catalog/storage rewrite can be visible without DDL event
        // notification. Rebaseline only under an intact event trigger
        // and increment generation, never reusing earlier cached receipts.
        tx.execute("UPDATE slr_native_replay.fence f SET generation=f.generation+1,
              last_catalog_snapshot=slr_native_replay.catalog_snapshot()
            WHERE f.singleton AND f.last_catalog_snapshot IS DISTINCT FROM slr_native_replay.catalog_snapshot()
              AND EXISTS(SELECT 1 FROM pg_event_trigger e WHERE e.evtname='slr_endpoint_replay_catalog_change'
                AND e.evtenabled='A' AND e.evtevent='ddl_command_end'
                AND e.evtfoid='slr_native_replay.capture_catalog_change()'::regprocedure)",&[])?;
    }
    let intact=ready && stamp_on(&mut tx)?.is_some();
    tx.commit()?;
    Ok(intact)
}
fn stamp_on(db:&mut impl postgres::GenericClient)->Result<Option<String>>{
    let installed:bool=db.query_one("SELECT to_regprocedure('slr_native_replay.current_stamp()') IS NOT NULL",&[])?.get(0);
    if !installed{return Ok(None);}
    let row=db.query_one("SELECT slr_native_replay.current_stamp(),current_setting('search_path'),
        current_setting('TimeZone'),current_setting('DateStyle'),current_setting('extra_float_digits'),
        current_setting('session_replication_role')",&[])?;
    let raw:Option<String>=row.get(0);
    Ok(raw.map(|s|digest(&serde_json::to_vec(&(s,row.get::<_,String>(1),row.get::<_,String>(2),
        row.get::<_,String>(3),row.get::<_,String>(4),row.get::<_,String>(5))).expect("string tuple serializes"))))
}
fn stamp(url:&str)->Result<Option<String>>{
    let mut db=Client::connect(url,NoTls)?;
    let mut tx=db.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let value=stamp_on(&mut tx)?;
    tx.commit()?;
    Ok(value)
}

#[derive(Debug,Clone,PartialEq,Eq)]
struct DualStamp { source:String,numeric:String }

fn same_instant(source:&str,numeric:&str)->Result<Option<DualStamp>>{
    let a=stamp(source)?;
    let b=stamp(numeric)?;
    let c=stamp(source)?;
    match (a,b,c){
        (Some(source),Some(numeric),Some(after)) if source==after=>Ok(Some(DualStamp{source,numeric})),
        _=>Ok(None),
    }
}

/// Session contains DB coordinates only in memory. No credential or
/// connection string enters a durable receipt or the output.
pub struct NativeEndpointReplaySession {
    source_url:String,
    numeric_url:String,
    key:String,
    implementation:Value,
    before:Option<DualStamp>,
}
pub fn prepare_native_endpoint_replay(config:&DatabaseConfig,numeric_url:&str,review:&str,parser_run:&str)
    ->Result<Option<NativeEndpointReplaySession>>{
    if std::env::var("SLR_NATIVE_ENDPOINT_REPLAY_MODE").as_deref()!=Ok("1")
        ||std::env::var("SLR_NATIVE_IDENTITY_REPLAY_MODE").as_deref()!=Ok("1")
        ||std::env::var("SLR_NATIVE_BOUNDED_IDENTITY_PROBE").as_deref()!=Ok("1")
        ||std::env::var("SLR_NATIVE_DOCUMENT_IDENTITY_PUBLISH").as_deref()==Ok("1"){
        return Ok(None);
    }
    let Some(implementation)=implementation_fingerprint()? else{return Ok(None)};
    let source_url=config.database_url().to_owned();
    if !install(&source_url)?||!install(numeric_url)?{return Ok(None);}
    let before=same_instant(&source_url,numeric_url)?;
    let key=digest(&serde_json::to_vec(&serde_json::json!({
        "review":review,"parser_run":parser_run,"implementation":implementation,
        "contract":"slr-native-endpoint-replay:v1"
    }))?);
    Ok(Some(NativeEndpointReplaySession{source_url,numeric_url:numeric_url.into(),key,implementation,before}))
}

fn replayable_candidate(v:&Value)->bool{
    v.get("schema").and_then(Value::as_str)==Some("slr-native-antecedent-endpoint-v1")
        &&v.get("outcome").and_then(Value::as_str)==Some("source_local_answer_with_governed_identity_unresolved")
        &&v["prefix_scope"]["parser_scope_ready"]==false_or_true(true)
        &&v["prefix_scope"]["source_region_prefix_covered"]==false_or_true(true)
        &&v["prefix_scope"]["original_parser_token_prefix_covered"]==false_or_true(true)
        &&v["source_local_answer"]["source_local_policy_inference_paid"]==false_or_true(true)
        &&v["governed_identity_paid"]==false_or_true(false)&&v["level_three_paid"]==false_or_true(false)
        &&v["external_qid_paid"]==false_or_true(false)&&v["historical_truth_paid"]==false_or_true(false)
        &&v["observed_prefix_token_coverage"]==false_or_true(true)
        &&v["projected_vs_original_entity_membership_parity"]==false_or_true(true)
        &&v["exact_entity_inverse_complete"]==false_or_true(true)
        &&v["numeric_publication"].as_array().and_then(|p|p.last()).is_some_and(|p|
            p["exact_parser_sentence_receipts"]==false_or_true(true)
                &&p["lean_workset_checked"]==false_or_true(true)
                &&p["sentence_jobs_remaining"].as_u64()==Some(0))
        &&v["binding_passes"].as_array().and_then(|p|p.last()).is_some_and(|p|
            p["identity_execution_replay"]["reference_baseline_present"]==false_or_true(true)
                &&p["identity_execution_replay"]["input_events_pending"].as_u64()==Some(0)
                &&p["consumer_horizon_work"]["h3_ready_after"]==false_or_true(false)
                &&p["consumer_horizon_work"]["h6_ready_after"]==false_or_true(false)
                &&p["consumer_horizon_work"]["nonmonotone_input_residual"]==false_or_true(false)
                &&p["consumer_horizon_work"]["h6_validity"]["durable_nonmonotone_authority_pending"]==false_or_true(false)
                &&p["consumer_horizon_work"]["h6_validity"]["original_consumer_authority_reconciled"]==false_or_true(true)
                &&p["outside_model_possible"]==false_or_true(true)
                &&p["complete_typed_coverage_paid"]==false_or_true(false))
}
fn false_or_true(b:bool)->Value{Value::Bool(b)}

// The PostgreSQL fence cannot observe writes to the external canonical
// source file. Reopen its exact indexed locator and verify bytes against
// the receipt in the SAME numeric snapshot. This is one source-file read,
// not a parser, binding, or observation scan.
fn canonical_source_matches(db:&mut impl postgres::GenericClient,payload:&Value,implementation:&Value)->Result<bool>{
    let Some(run)=payload["numeric_publication"].as_array().and_then(|p|p.last())
        .and_then(|p|p["numeric_run_ref"].as_str()) else{return Ok(false)};
    let Some(doc)=payload["source_local_answer"]["source_document_ref"].as_str() else{return Ok(false)};
    let Some(expected)=payload["source_local_answer"]["source_content_sha256"].as_str() else{return Ok(false)};
    let Some(root)=implementation["source_root"].as_str() else{return Ok(false)};
    let rows=db.query("SELECT locator,encode(content_sha256,'hex'),byte_count
        FROM execution.semantic_parser_source WHERE run_ref=$1 AND document_ref=$2",&[&run,&doc])?;
    if rows.len()!=1||rows[0].get::<_,String>(1)!=expected{return Ok(false);}
    let locator:String=rows[0].get(0);
    let Ok(path)=PathBuf::from(locator).canonicalize() else{return Ok(false)};
    if !path.starts_with(Path::new(root)){return Ok(false);}
    let Ok(bytes)=std::fs::read(path) else{return Ok(false)};
    Ok(i64::try_from(bytes.len()).ok()==Some(rows[0].get::<_,i64>(2))&&digest(&bytes)==expected)
}

impl NativeEndpointReplaySession {
    /// Cache row and numeric stamp share the same read-only snapshot.
    /// The source is collected on both sides of it. An intervening
    /// committed source mutation cannot make a stale cache hit.
    pub fn reopen(&self)->Result<Option<Value>>{
        let Some(before)=&self.before else{return Ok(None)};
        let a=stamp(&self.source_url)?;
        let mut db=Client::connect(&self.numeric_url,NoTls)?;
        let mut tx=db.transaction()?;
        tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
        let n=stamp_on(&mut tx)?;
        let row=tx.query_opt("SELECT source_stamp,numeric_stamp,payload_sha256,payload
          FROM slr_native_replay.endpoint_receipt WHERE key_ref=$1",&[&self.key])?;
        let source_bytes_verified=if let Some(row)=&row{
            let payload:Value=row.get(3);
            canonical_source_matches(&mut tx,&payload,&self.implementation)?
        }else{false};
        tx.commit()?;
        let c=stamp(&self.source_url)?;
        if !source_bytes_verified||a.as_deref()!=Some(before.source.as_str())||c!=a||n.as_deref()!=Some(before.numeric.as_str()){
            return Ok(None);
        }
        let Some(row)=row else{return Ok(None)};
        if row.get::<_,String>(0)!=before.source||row.get::<_,String>(1)!=before.numeric{return Ok(None);}
        let payload:Value=row.get(3);
        if digest(&serde_json::to_vec(&payload)?)!=row.get::<_,String>(2)||!replayable_candidate(&payload){
            return Err(NativeEndpointReplayError::Contract);
        }
        if implementation_fingerprint()?.as_ref()!=Some(&self.implementation){return Ok(None);}
        // Historical diagnostic receipts stay unchanged inside this
        // payload. No producer, kernel or database proof ran in this hit.
        let mut value=payload;
        value["endpoint_execution_replay"]=serde_json::json!({
            "schema":"slr-native-endpoint-replay-hit:v1","hit":true,
            "source_stamp":before.source,"numeric_stamp":before.numeric,
            "source_observation_scan_performed":false,"numeric_binding_scan_performed":false,
            "external_canonical_source_bytes_verified":true,
            "parser_identity_refresh_invocations":0,"semantic_recomputation_performed":false,
            "writes_semantic_state":false,"new_golden_kernel_execution":false,
            "current_authority_reused":false,"conservative_database_wide_invalidation":true
        });
        Ok(Some(value))
    }
    /// The cold result can seed a cache only after an entire quiescent
    /// reference/readback pass. Same counts or an unchanged source SHA
    /// are not enough. Writes during the pass force another bounded pass.
    pub fn record_if_quiescent(&self,value:&Value)->Result<bool>{
        let Some(before)=&self.before else{return Ok(false)};
        if !replayable_candidate(value)||implementation_fingerprint()?.as_ref()!=Some(&self.implementation){
            return Ok(false);
        }
        if same_instant(&self.source_url,&self.numeric_url)?.as_ref()!=Some(before){return Ok(false);}
        let payload_sha=digest(&serde_json::to_vec(value)?);
        let mut db=Client::connect(&self.numeric_url,NoTls)?;
        let mut tx=db.transaction()?;
        tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ; SET LOCAL statement_timeout='90000ms'")?;
        if stamp_on(&mut tx)?.as_deref()!=Some(before.numeric.as_str())
            ||!canonical_source_matches(&mut tx,value,&self.implementation)?{return Ok(false);}
        tx.execute("INSERT INTO slr_native_replay.endpoint_receipt(key_ref,source_stamp,numeric_stamp,payload_sha256,payload)
          VALUES($1,$2,$3,$4,$5) ON CONFLICT(key_ref) DO UPDATE SET
          source_stamp=EXCLUDED.source_stamp,numeric_stamp=EXCLUDED.numeric_stamp,
          payload_sha256=EXCLUDED.payload_sha256,payload=EXCLUDED.payload",
          &[&self.key,&before.source,&before.numeric,&payload_sha,value])?;
        tx.commit()?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn no_candidate_replay_from_incomplete_or_authority_packet(){
        assert!(!replayable_candidate(&serde_json::json!({"schema":"slr-native-antecedent-endpoint-v1"})));
        assert!(!replayable_candidate(&serde_json::json!({"outcome":"governed_source_answer","governed_identity_paid":true})));
    }
}
