//! Native exact-source numeric PNF pipeline over the ORIGINAL SQL owner.
//!
//! One original-SLR source -> one numeric run, then bounded leased sentence
//! composition and atomic original SQL publication. Reuses SQL migration 040
//! sentence/work trigger and 180 relation-valued anaphor demand producer.
//! No Python document-sized compatibility replay, no nearest-name identity.
//!
//! This is source-written; completion additionally needs kernel compilation,
//! strict reference parity, persistent checkpoint/repair tests and H9 witness.

use crate::{
    load_exact_native_numeric_evidence,plan_original_native_numeric_parser,
    persist_original_numeric_parser_plan,compose_numeric_sentence_native,
    publish_original_numeric_sentence,
    DatabaseConfig,NativeManifest,NativeNumericParserPlan,
    NumericToken,NumericOperatorLexicon,NumericSentenceLease,
    NumericMdlWeights,NativeSentenceClosure,
};
use crate::numeric_pnf_parser_store::{intern_all,NumericParserStoreError};
use crate::numeric_pnf_operator::NumericOperatorError;
use crate::numeric_pnf_sql::NumericSqlError;
use crate::numeric_pnf_parser_plan::NumericParserPlanError;
use crate::numeric_pnf_source_bridge::NativeNumericSourceError;
use postgres::{Client,NoTls};
use sha2::{Digest,Sha256};
use serde::Serialize;
use std::collections::{BTreeMap,BTreeSet};
use std::path::Path;
use std::time::{SystemTime,UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug,Error)]
pub enum NumericPipelineError {
    #[error("source validation: {0}")]
    Source(#[from] NativeNumericSourceError),
    #[error("original parser plan: {0}")]
    Plan(#[from] NumericParserPlanError),
    #[error("numeric PostgreSQL parser: {0}")]
    Parser(#[from] NumericParserStoreError),
    #[error("numeric SQL query: {0}")]
    Database(#[from] postgres::Error),
    #[error("numeric operator: {0}")]
    Operator(#[from] NumericOperatorError),
    #[error("numeric sentence persistence: {0}")]
    Publisher(#[from] NumericSqlError),
    #[error("original numeric source has conflicting parser/region state")]
    ForeignRun,
    #[error("numeric sentence has corrupt head/token/factor scope")]
    InvalidSentence,
    #[error("numeric pipeline job cannot be replayed within bounded limits")]
    Budget,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct NumericPipelineReceipt {
    pub schema:&'static str,
    pub original_source_sha256:String,
    pub original_source_revision_ref:String,
    pub original_parser_run_ref:String,
    pub numeric_run_ref:String,
    pub numeric_source_token_count:usize,
    pub numeric_source_sentence_count:usize,
    pub numeric_source_entity_count:usize,
    pub source_full_document_coverage_paid:bool,
    pub sentence_jobs_completed:usize,
    pub sentence_jobs_remaining:usize,
    pub bounded_replay_supported:bool,
    pub all_original_typed_demands_covered:bool,
    pub unique_document_identity_paid:bool,
    pub level_three_identity_substitution_paid:bool,
    pub external_qid_paid:bool,
    pub historical_truth_paid:bool,
    pub residuals:Vec<String>,
}

fn hash_parts(parts:&[&str])->String{
    let mut h=Sha256::new();
    for s in std::iter::once("slr-native-strict-numeric-pnf-run:v1")
        .chain(parts.iter().copied()){
        h.update((s.len() as u64).to_be_bytes());h.update(s.as_bytes());
    }
    format!("slr-native-numeric-run:sha256:{:x}",h.finalize())
}
pub fn deterministic_numeric_run_ref(manifest:&NativeManifest)->String{
    hash_parts(&[&manifest.document_ref,&manifest.source_revision_ref,
        &manifest.parser_run_ref,&manifest.canonical_content_sha256,
        &manifest.rule_set_revision])
}
fn original_bytes(config:&DatabaseConfig,manifest:&NativeManifest)
    ->Result<Vec<u8>,NumericPipelineError>{
    let mut db=Client::connect(config.database_url(),NoTls)?;
    let rows=db.query(
        "SELECT c.payload,g.content_digest_ref
         FROM ingest.generic_source_revision AS g
         JOIN corpus.document AS d ON d.document_ref=g.document_ref
         JOIN corpus.canonical_content AS c ON c.canonical_ref=d.canonical_ref
         WHERE g.source_revision_ref=$1 AND g.document_ref=$2",
        &[&manifest.source_revision_ref,&manifest.document_ref])?;
    if rows.len()!=1{return Err(NumericPipelineError::ForeignRun);}
    let bytes:Vec<u8>=rows[0].get(0);
    let declared:String=rows[0].get(1);
    if declared!=format!("sha256:{}",manifest.canonical_content_sha256)
        ||manifest.verify_source_bytes(&bytes).is_err(){
        return Err(NumericPipelineError::ForeignRun);
    }
    Ok(bytes)
}

fn operator_symbols()->BTreeSet<(i16,String)>{
    let mut names=BTreeSet::new();
    for name in ["must","shall","may","not","never","if","when",
       "provided","providing","unless","except","excluding",
       "commence","begin","repeal","amend","cease"]{
        names.insert((2,name.into()));
    }
    for name in ["aux","auxpass","nsubj","nsubjpass","csubj",
       "obj","dobj","pobj","attr","oprd","mark","prep","advmod"]{
        names.insert((5,name.into()));
    }
    for name in ["VERB","AUX"]{names.insert((3,name.into()));}
    for name in ["semantic.normative_relation","semantic.legal_condition",
       "semantic.legal_exception","semantic.legal_transition"]{
        names.insert((10,name.into()));
    }
    for name in ["normative.obligation","normative.permission_candidate",
       "normative.prohibition","legal.activation_condition_candidate",
       "legal.exception_candidate","legal.commencement",
       "legal.commencement_candidate","legal.repeal",
       "legal.amendment","legal.cessation"]{names.insert((11,name.into()));}
    for name in ["conduct","bearer","object","condition","exception",
       "host","transition","legal_object"]{names.insert((12,name.into()));}
    for name in ["jurisdiction_unresolved","legal_time_unresolved",
       "normative_scope_unresolved","modal_sense_unresolved",
       "norm_bearer_unresolved","exception_attachment_unresolved",
       "exception_burden_unresolved","condition_attachment_unresolved",
       "legal_object_identity_unresolved","effective_time_unresolved"]{
        names.insert((13,name.into()));
    }
    names.insert((14,"parser.role_participant".into()));
    names
}
fn lexicon(database_url:&str)->Result<NumericOperatorLexicon,NumericPipelineError>{
    let mut db=Client::connect(database_url,NoTls)?;
    let mut tx=db.transaction()?;
    tx.batch_execute("SET LOCAL statement_timeout='90000ms'")?;
    let symbols=intern_all(&mut tx,&operator_symbols())?;
    tx.commit()?;
    let mut lex=NumericOperatorLexicon{
        lemma_ids:BTreeMap::new(),dependency_ids:BTreeMap::new(),
        pos_ids:BTreeMap::new(),factor_type_ids:BTreeMap::new(),
        predicate_ids:BTreeMap::new(),role_ids:BTreeMap::new(),
        residual_ids:BTreeMap::new(),object_kind_ids:BTreeMap::new(),
    };
    for ((kind,text),id) in symbols{
        let map=match kind{
            2=>&mut lex.lemma_ids,
            3=>&mut lex.pos_ids,
            5=>&mut lex.dependency_ids,
            10=>&mut lex.factor_type_ids,
            11=>&mut lex.predicate_ids,
            12=>&mut lex.role_ids,
            13=>&mut lex.residual_ids,
            14=>&mut lex.object_kind_ids,
            _=>return Err(NumericPipelineError::ForeignRun),
        };
        map.insert(text,id);
    }
    lex.validate()?;
    Ok(lex)
}

fn lease_one(database_url:&str,run_ref:&str,document_ref:&str)
    ->Result<Option<(NumericSentenceLease,i64)>,NumericPipelineError>{
    let mut db=Client::connect(database_url,NoTls)?;
    let mut tx=db.transaction()?;
    tx.batch_execute("SET LOCAL statement_timeout='90000ms'")?;
    let job=tx.query_opt(
        "SELECT work.work_id,work.region_id,work.lease_epoch,
                link.sentence_id
         FROM execution.semantic_pnf_work_item AS work
         JOIN execution.semantic_pnf_region AS region
           ON region.region_id=work.region_id
         JOIN execution.semantic_pnf_sentence_region AS link
           ON link.region_id=work.region_id
         WHERE work.run_ref=$1 AND work.document_ref=$2
           AND work.operation_id=1 AND region.region_kind=1
           AND (work.state_id=1 OR
                 (work.state_id=2 AND work.lease_expires_at<CURRENT_TIMESTAMP))
         ORDER BY work.priority,work.work_id
         LIMIT 1 FOR UPDATE OF work SKIP LOCKED",
        &[&run_ref,&document_ref])?;
    let Some(job)=job else{return Ok(None)};
    let work_id:i64=job.get(0);
    let region_id:i64=job.get(1);
    let prior:i64=job.get(2);
    let sentence_id:i64=job.get(3);
    let now=SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|_|NumericPipelineError::Budget)?.as_nanos().to_string();
    let token=hash_parts(&[run_ref,&work_id.to_string(),
        &prior.to_string(),&std::process::id().to_string(),&now]);
    let updated=tx.execute(
        "UPDATE execution.semantic_pnf_work_item
         SET state_id=2,lease_owner='slr-native-numeric-v1',
           lease_token=$1,lease_epoch=lease_epoch+1,
           lease_expires_at=CURRENT_TIMESTAMP + INTERVAL '180 seconds',
           attempt_count=attempt_count+1
         WHERE work_id=$2 AND lease_epoch=$3
           AND (state_id=1 OR (state_id=2
              AND lease_expires_at<CURRENT_TIMESTAMP))",
        &[&token,&work_id,&prior])?;
    if updated!=1{return Err(NumericPipelineError::Budget);}
    tx.commit()?;
    Ok(Some((NumericSentenceLease{
        work_id,region_id,lease_token:token,lease_epoch:prior+1,
    },sentence_id)))
}
fn numeric_sentence_tokens(
    database_url:&str,run_ref:&str,document_ref:&str,sentence_id:i64,
)->Result<Vec<NumericToken>,NumericPipelineError>{
    let mut db=Client::connect(database_url,NoTls)?;
    let rows=db.query(
        "SELECT token_id,orth_symbol_id,lemma_symbol_id,pos_symbol_id,
                tag_symbol_id,dependency_symbol_id,head_token_id,morph_set_id,
                start_char,end_char
         FROM execution.semantic_parser_token
         WHERE run_ref=$1 AND document_ref=$2 AND sentence_id=$3
           AND representation_version=2
         ORDER BY local_token_ordinal",
        &[&run_ref,&document_ref,&sentence_id])?;
    let mut tokens=Vec::with_capacity(rows.len());
    let mut ids=BTreeSet::new();
    for row in rows{
        let t=NumericToken{
            token_id:row.get(0),
            orth_id:row.get::<_,Option<i64>>(1)
                .ok_or(NumericPipelineError::InvalidSentence)?,
            lemma_id:row.get::<_,Option<i64>>(2)
                .ok_or(NumericPipelineError::InvalidSentence)?,
            pos_id:row.get::<_,Option<i64>>(3)
                .ok_or(NumericPipelineError::InvalidSentence)?,
            tag_id:row.get::<_,Option<i64>>(4)
                .ok_or(NumericPipelineError::InvalidSentence)?,
            dependency_id:row.get::<_,Option<i64>>(5)
                .ok_or(NumericPipelineError::InvalidSentence)?,
            head_token_id:row.get::<_,Option<i64>>(6)
                .ok_or(NumericPipelineError::InvalidSentence)?,
            morph_set_id:row.get(7),
            start_char:row.get(8),end_char:row.get(9),
        };
        if !ids.insert(t.token_id){return Err(NumericPipelineError::InvalidSentence);}
        tokens.push(t);
    }
    for token in &tokens{
        if !ids.contains(&token.head_token_id){
            return Err(NumericPipelineError::InvalidSentence);
        }
    }
    Ok(tokens)
}

pub fn run_native_numeric_exact_source_slice(
    slr:&DatabaseConfig,
    numeric_database_url:&str,
    source_revision_ref:&str,
    original_parser_run_ref:&str,
    source_root:&Path,
    max_sentences:usize,
)->Result<NumericPipelineReceipt,NumericPipelineError>{
    if max_sentences==0||max_sentences>64{
        return Err(NumericPipelineError::Budget);
    }
    let original=load_exact_native_numeric_evidence(
        slr,source_revision_ref,original_parser_run_ref,
        "slr-strict-numeric-pnf-v1"
    )?;
    let canonical=original_bytes(slr,&original.manifest)?;
    let run_ref=deterministic_numeric_run_ref(&original.manifest);
    let plan:NativeNumericParserPlan=
        plan_original_native_numeric_parser(&original,&run_ref)?;
    let persisted=persist_original_numeric_parser_plan(
        numeric_database_url,source_root,&canonical,&plan,&original.manifest
    )?;
    let lex=lexicon(numeric_database_url)?;
    let mut completed=0usize;
    let mdl=NumericMdlWeights::default();
    while completed<max_sentences{
        let Some((lease,sentence_id))=lease_one(
            numeric_database_url,&run_ref,&original.manifest.document_ref
        )? else {break};
        let tokens=numeric_sentence_tokens(
            numeric_database_url,&run_ref,
            &original.manifest.document_ref,sentence_id
        )?;
        let closure:NativeSentenceClosure=
            compose_numeric_sentence_native(lease.region_id,&tokens,&lex)?;
        publish_original_numeric_sentence(
            numeric_database_url,&original.manifest,&run_ref,
            &canonical,&lease,&closure,&mdl
        )?;
        completed+=1;
    }
    let mut db=Client::connect(numeric_database_url,NoTls)?;
    let remaining:i64=db.query_one(
        "SELECT count(*)::bigint
         FROM execution.semantic_pnf_work_item
         WHERE run_ref=$1 AND document_ref=$2 AND operation_id=1
           AND state_id<>3",
        &[&run_ref,&original.manifest.document_ref])?.get(0);
    let mut residuals=plan.residuals.clone();
    if remaining>0 {
        residuals.push("numeric_sentence_work_remains_bounded_and_reopenable".into());
    }
    if !plan.full_document_coverage_paid{
        residuals.push("original_parser_scope_not_globally_complete".into());
    }
    residuals.push(
        "no_original_resolved_unique_actor_witness_or_Level3_admission_issued".into()
    );
    Ok(NumericPipelineReceipt{
        schema:"slr-native-strict-numeric-pnf-slice-v1",
        original_source_sha256:plan.original_source_sha256,
        original_source_revision_ref:source_revision_ref.into(),
        original_parser_run_ref:original_parser_run_ref.into(),
        numeric_run_ref:run_ref,numeric_source_token_count:persisted.numeric_tokens,
        numeric_source_sentence_count:persisted.numeric_sentences,
        numeric_source_entity_count:persisted.numeric_entities,
        source_full_document_coverage_paid:plan.full_document_coverage_paid,
        sentence_jobs_completed:completed,sentence_jobs_remaining:remaining as usize,
        bounded_replay_supported:true,all_original_typed_demands_covered:false,
        unique_document_identity_paid:false,level_three_identity_substitution_paid:false,
        external_qid_paid:false,historical_truth_paid:false,residuals,
    })
}
