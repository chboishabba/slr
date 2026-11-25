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
    plan_original_native_numeric_parser,
    persist_original_numeric_parser_plan,compose_numeric_sentence_native,
    publish_original_numeric_sentence,
    DatabaseConfig,NativeManifest,NativeNumericParserPlan,
    NumericToken,NumericOperatorLexicon,NumericSentenceLease,
    NumericMdlWeights,NativeSentenceClosure,
};
use crate::numeric_pnf_parser_store::{intern_all,NumericParserStoreError,checked_native_parser_sentence_refs};
use crate::numeric_pnf_operator::NumericOperatorError;
use crate::numeric_pnf_sql::NumericSqlError;
use crate::numeric_pnf_parser_plan::NumericParserPlanError;
use crate::numeric_pnf_source_bridge::{NativeNumericSourceError,load_exact_native_numeric_evidence_scope};
use crate::numeric_pnf_parser_plan::{original_numeric_sentence_identity,digest_numeric_sentence_refs};
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
    Database(String),
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

impl From<postgres::Error> for NumericPipelineError {
    fn from(error: postgres::Error) -> Self {
        let detail = error.as_db_error().map(|db| {
            let mut parts = vec![db.message().to_owned()];
            if let Some(detail) = db.detail() { parts.push(format!("detail={detail}")); }
            if let Some(hint) = db.hint() { parts.push(format!("hint={hint}")); }
            if let Some(constraint) = db.constraint() { parts.push(format!("constraint={constraint}")); }
            parts.push(format!("code={}", db.code().code()));
            parts.join("; ")
        }).unwrap_or_else(|| format!("{error:?}"));
        Self::Database(detail)
    }
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
    pub consumer_prefix_start_bound:Option<i64>,
    pub original_observations_loaded:usize,
    pub required_sentence_refs:Vec<String>,
    pub lean_workset_checked:bool,
    pub missing_required_sentence_refs:Vec<String>,
    pub already_published_required_refs:Vec<String>,
    pub newly_registered_sentence_refs:Vec<String>,
    pub closed_sentence_region_starts:Vec<i64>,
    pub sentence_jobs_remaining:usize,
    pub entity_occurrence_objects_inserted:i64,
    pub entity_occurrence_token_supports_inserted:i64,
    pub frontier_jobs_completed:usize,
    pub frontier_jobs_remaining:Option<i64>,
    pub exact_parser_sentence_receipts:bool,
    pub already_verified_required_refs:Vec<String>,
    pub revalidated_existing_sentence_refs:Vec<String>,
    pub original_observation_scan_performed:bool,
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

fn lease_one(database_url:&str,run_ref:&str,document_ref:&str,focus:Option<i64>)
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
           AND ($3::bigint IS NULL OR region.start_char<=$3)
           AND (work.state_id=1 OR
                 (work.state_id=2 AND work.lease_expires_at<CURRENT_TIMESTAMP))
         ORDER BY work.priority,work.work_id
         LIMIT 1 FOR UPDATE OF work SKIP LOCKED",
        &[&run_ref,&document_ref,&focus])?;
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


// Use the original migration-145 owner, not a parallel Rust interpretation of
// its typed frontier. Each step is atomic: deleting the dirty marker and
// rebuilding its interface either both commit or both roll back. The source
// parser and all sentence work must be complete before this stage is entered.
fn reduce_original_frontier_slice(
    db:&mut Client, run_ref:&str, document_ref:&str, budget:usize,
)->Result<(usize,i64),NumericPipelineError>{
    let started=std::time::Instant::now();
    let mut completed=0usize;
    // Seed missing/stale receipts using original m200 child-closure and
    // child-reduction freshness rules, not a second frontier reducer.
    {
        let mut tx=db.transaction()?;
        tx.batch_execute("SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
        tx.execute(
            "INSERT INTO execution.semantic_pnf_frontier_dirty
                (interface_id,reason_interface_id,dirty_reason)
             SELECT i.interface_id,NULL,
                    CASE WHEN receipt.interface_id IS NULL THEN 'missing_reduction_receipt'
                         ELSE 'stale_graph_revision' END
             FROM execution.semantic_pnf_interface i
             JOIN execution.semantic_pnf_region r ON r.region_id=i.region_id
             LEFT JOIN execution.semantic_pnf_frontier_reduction_receipt receipt
               ON receipt.interface_id=i.interface_id
             WHERE r.run_ref=$1 AND r.document_ref=$2
               AND r.region_kind NOT IN (1,2,4,9) AND i.closure_state IN (2,3)
               AND (receipt.interface_id IS NULL OR
                    receipt.graph_revision IS DISTINCT FROM i.graph_revision
                    OR EXISTS (
                        SELECT 1 FROM execution.semantic_pnf_interface child
                        JOIN execution.semantic_pnf_region child_region ON child_region.region_id=child.region_id
                        WHERE child.parent_interface_id=i.interface_id
                          AND child_region.region_kind NOT IN (2,4,9)
                          AND child_region.closed_at>receipt.reduced_at
                    )
                    OR EXISTS (
                        SELECT 1 FROM execution.semantic_pnf_interface child
                        JOIN execution.semantic_pnf_region child_region ON child_region.region_id=child.region_id
                        JOIN execution.semantic_pnf_frontier_reduction_receipt child_receipt
                          ON child_receipt.interface_id=child.interface_id
                        WHERE child.parent_interface_id=i.interface_id
                          AND child_region.region_kind NOT IN (2,4,9)
                          AND child_receipt.reduced_at>receipt.reduced_at
                    ))
             ON CONFLICT (interface_id) DO NOTHING",
            &[&run_ref,&document_ref])?;
        tx.commit()?;
    }
    while completed<budget {
        let remaining=90_000u128.saturating_sub(started.elapsed().as_millis());
        if remaining<100 {break;}
        let mut tx=db.transaction()?;
        tx.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE; SET LOCAL lock_timeout='5000ms'")?;
        tx.query_one("SELECT set_config('statement_timeout',$1,true)",
            &[&format!("{}ms",remaining)])?;
        // Serialize native reductions for this exact run/document. Do not
        // SKIP LOCKED a child and incorrectly reduce a parent ahead of it.
        tx.query_one(
            "SELECT pg_advisory_xact_lock(hashtext($1),hashtext($2))",
            &[&run_ref,&document_ref])?;
        let row=tx.query_opt(
            "SELECT dirty.interface_id
             FROM execution.semantic_pnf_frontier_dirty dirty
             JOIN execution.semantic_pnf_interface i ON i.interface_id=dirty.interface_id
             JOIN execution.semantic_pnf_region r ON r.region_id=i.region_id
             WHERE r.run_ref=$1 AND r.document_ref=$2
               AND r.region_kind NOT IN (1,2,4,9) AND i.closure_state IN (2,3)
             ORDER BY r.region_kind,(r.end_char-r.start_char),r.sequence_no,dirty.interface_id
             LIMIT 1 FOR UPDATE OF dirty",
            &[&run_ref,&document_ref])?;
        let Some(row)=row else {tx.commit()?;break;};
        let id:i64=row.get(0);
        if tx.execute("DELETE FROM execution.semantic_pnf_frontier_dirty WHERE interface_id=$1",
            &[&id])?!=1 {return Err(NumericPipelineError::ForeignRun);}
        // The existing m209 owner already performs desired/current
        // reconciliation. Correct its exact historical receipt accounting
        // only, without duplicating the reducer. Unknown versions are not
        // silently overwritten; legacy databases without m209 are unchanged.
        tx.batch_execute(include_str!("numeric_pnf_m209_receipt_correction.sql"))?;
        // m211 publication barrier: topology construction can defer this
        // stable owner. A bounded explicit publication must force it, or
        // a read-only return could consume the dirty marker.
        tx.query_one("SELECT set_config('sensiblaw.force_frontier_rebuild','on',true)",&[])?;
        tx.query_one("SELECT * FROM execution.rebuild_numeric_pnf_parent_frontier($1)",&[&id])?;
        // Reopen the original receipt, not just the function return shape.
        // Failure rolls back the marker removal in the same transaction.
        let receipt_current:bool=tx.query_one(
            "SELECT EXISTS(SELECT 1 FROM execution.semantic_pnf_interface i
              JOIN execution.semantic_pnf_frontier_reduction_receipt receipt
                ON receipt.interface_id=i.interface_id
              WHERE i.interface_id=$1 AND receipt.graph_revision=i.graph_revision)",
            &[&id])?.get(0);
        if !receipt_current{return Err(NumericPipelineError::ForeignRun);}
        tx.commit()?;
        completed+=1;
    }
    let pending:i64=db.query_one(
        "SELECT count(*)::bigint FROM execution.semantic_pnf_frontier_dirty dirty
         JOIN execution.semantic_pnf_interface i ON i.interface_id=dirty.interface_id
         JOIN execution.semantic_pnf_region r ON r.region_id=i.region_id
         WHERE r.run_ref=$1 AND r.document_ref=$2
           AND r.region_kind NOT IN (1,2,4,9)",
        &[&run_ref,&document_ref])?.get(0);
    Ok((completed,pending))
}


// Explicit source-occurrence carrier, NOT the retired migration-045 broad
// procedural promotion trigger. Source-backed NER spans are distinct local
// observations even when their spelling repeats. Do not export them into the
// legacy uniqueness planner until the typed coverage/admission guard exists.
// Existing numeric factor objects and original pronoun demands remain intact.
// This extension has its own digest domain and is not strict-legacy parity.
fn materialize_numeric_entity_occurrences(
    db:&mut Client, run_ref:&str, document_ref:&str, budget:usize,focus:Option<i64>,
)->Result<(i64,i64),NumericPipelineError>{
    let mut tx=db.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
        SET LOCAL statement_timeout='600000ms'; SET LOCAL lock_timeout='5000ms'")?;
    tx.query_one("SELECT pg_advisory_xact_lock(hashtext($1),hashtext($2))",
        &[&run_ref,&document_ref])?;
    let kind:i64=tx.query_one(
        "SELECT execution.ensure_semantic_symbol(14::smallint,'mention.entity')",
        &[])?.get(0);
    // A unique dependency head of the exact span is required. All contained
    // source tokens remain supported; crossing/ambiguous spans remain unpaid.
    tx.execute(
        "CREATE TEMP TABLE native_numeric_entity_occurrence
         ON COMMIT DROP AS
         WITH span AS MATERIALIZED (
           SELECT e.entity_id,ps.sentence_id,e.start_char,e.end_char,r.region_id,
                  digest(convert_to('slr:numeric-ner-occurrence:v1','UTF8')
                         || int8send(r.region_id) || int8send(e.entity_id),'sha256') AS object_digest
           FROM execution.semantic_parser_entity_span e
           JOIN execution.semantic_parser_sentence ps
             ON ps.sentence_ref=e.sentence_ref AND ps.run_ref=e.run_ref
            AND ps.document_ref=e.document_ref AND ps.representation_version=2
           JOIN execution.semantic_pnf_sentence_region l ON l.sentence_id=ps.sentence_id
           JOIN execution.semantic_pnf_region r ON r.region_id=l.region_id
           WHERE e.run_ref=$1 AND e.document_ref=$2 AND e.representation_version=2
             AND r.run_ref=$1 AND r.document_ref=$2 AND r.region_kind=1
             AND r.closure_state IN (2,3)
             AND ($4::bigint IS NULL OR r.start_char<=$4)
         ), supported AS MATERIALIZED (
           SELECT s.*,t.token_id,t.lemma_symbol_id,t.head_token_id,
                  t.start_char AS token_start,t.end_char AS token_end
           FROM span s JOIN execution.semantic_parser_token t
             ON t.sentence_id=s.sentence_id AND t.run_ref=$1 AND t.document_ref=$2
             AND t.representation_version=2 AND t.start_char>=s.start_char
             AND t.end_char<=s.end_char
         ), head AS (
           SELECT s.entity_id,s.region_id,min(s.lemma_symbol_id) AS head_symbol_id
           FROM supported s
           WHERE s.head_token_id IS NOT NULL AND s.lemma_symbol_id IS NOT NULL
             AND (s.head_token_id=s.token_id OR NOT EXISTS (
                 SELECT 1 FROM supported h WHERE h.entity_id=s.entity_id
                    AND h.region_id=s.region_id AND h.token_id=s.head_token_id))
           GROUP BY s.entity_id,s.region_id HAVING count(*)=1
         ), eligible AS (
           SELECT s.entity_id,s.region_id,s.object_digest,h.head_symbol_id
           FROM span s JOIN head h USING(entity_id,region_id)
           JOIN supported t USING(entity_id,region_id)
           GROUP BY s.entity_id,s.region_id,s.object_digest,h.head_symbol_id,s.start_char,s.end_char
           HAVING min(t.token_start)=s.start_char AND max(t.token_end)=s.end_char
              AND count(*)<=32768 AND bool_and(t.head_token_id IS NOT NULL)
         ), missing AS (
           SELECT e.* FROM eligible e
           WHERE NOT EXISTS (SELECT 1 FROM execution.semantic_pnf_object o
                             WHERE o.object_digest=e.object_digest)
              OR EXISTS (
                 SELECT 1 FROM supported t
                 WHERE t.entity_id=e.entity_id AND t.region_id=e.region_id
                   AND NOT EXISTS (
                     SELECT 1 FROM execution.semantic_pnf_object o
                     JOIN execution.semantic_pnf_object_token_support os ON os.object_id=o.object_id
                     WHERE o.object_digest=e.object_digest AND os.token_id=t.token_id))
           ORDER BY e.region_id,e.entity_id LIMIT $3
         )
         SELECT m.*,t.token_id,
                (row_number() OVER(PARTITION BY m.region_id,m.entity_id
                    ORDER BY t.token_start,t.token_id)-1)::smallint AS ordinal
         FROM missing m JOIN supported t USING(entity_id,region_id)",
        &[&run_ref,&document_ref,&(budget as i64),&focus])?;
    // No identity witness, demand closure, external alignment or promotion.
    let objects=tx.execute(
        "INSERT INTO execution.semantic_pnf_object
          (object_digest,region_id,object_kind_symbol_id,head_symbol_id,
           scope_region_id,promotion_level,information_gain,representation_cost,
           ambiguity_cost,promotion_score,active)
         SELECT DISTINCT object_digest,region_id,$1::bigint,head_symbol_id,
                region_id,0,0,1,1,0,TRUE FROM native_numeric_entity_occurrence
         ON CONFLICT(object_digest) DO NOTHING",&[&kind])?;
    let support=tx.execute(
        "INSERT INTO execution.semantic_pnf_object_token_support(object_id,token_id,ordinal)
         SELECT o.object_id,s.token_id,s.ordinal
         FROM native_numeric_entity_occurrence s
         JOIN execution.semantic_pnf_object o ON o.object_digest=s.object_digest
          AND o.region_id=s.region_id AND o.object_kind_symbol_id=$1::bigint
          AND o.head_symbol_id=s.head_symbol_id
         ON CONFLICT DO NOTHING",&[&kind])?;
    let valid:bool=tx.query_one(
        "SELECT NOT EXISTS (
           SELECT 1 FROM native_numeric_entity_occurrence s
           WHERE NOT EXISTS (
             SELECT 1 FROM execution.semantic_pnf_object o
             JOIN execution.semantic_pnf_object_token_support os ON os.object_id=o.object_id
             WHERE o.object_digest=s.object_digest AND o.region_id=s.region_id
               AND o.object_kind_symbol_id=$1::bigint AND o.head_symbol_id=s.head_symbol_id
               AND os.token_id=s.token_id AND os.ordinal=s.ordinal))",
        &[&kind])?.get(0);
    if !valid{return Err(NumericPipelineError::ForeignRun);}
    tx.commit()?;
    Ok((objects as i64,support as i64))
}

pub fn run_native_numeric_exact_source_slice(
    slr:&DatabaseConfig,numeric_database_url:&str,source_revision_ref:&str,
    original_parser_run_ref:&str,source_root:&Path,max_sentences:usize,
)->Result<NumericPipelineReceipt,NumericPipelineError>{
    run_native_numeric_scoped_slice(slr,numeric_database_url,source_revision_ref,
        original_parser_run_ref,source_root,max_sentences,None)
}
pub fn run_native_numeric_prefix_slice(
    slr:&DatabaseConfig,numeric_database_url:&str,source_revision_ref:&str,
    original_parser_run_ref:&str,source_root:&Path,max_sentences:usize,focus_start:i64,
)->Result<NumericPipelineReceipt,NumericPipelineError>{
    if focus_start<0{return Err(NumericPipelineError::ForeignRun);}
    run_native_numeric_scoped_slice(slr,numeric_database_url,source_revision_ref,
        original_parser_run_ref,source_root,max_sentences,Some(focus_start))
}
fn run_native_numeric_scoped_slice(
    slr:&DatabaseConfig,
    numeric_database_url:&str,
    source_revision_ref:&str,
    original_parser_run_ref:&str,
    source_root:&Path,
    max_sentences:usize,focus:Option<i64>,
)->Result<NumericPipelineReceipt,NumericPipelineError>{
    if max_sentences==0||max_sentences>64{
        return Err(NumericPipelineError::Budget);
    }
    let (original,inventory)=load_exact_native_numeric_evidence_scope(
        slr,source_revision_ref,original_parser_run_ref,
        "slr-strict-numeric-pnf-v1",focus
    )?;
    let canonical=original_bytes(slr,&original.manifest)?;
    let run_ref=deterministic_numeric_run_ref(&original.manifest);
    let mut plan:NativeNumericParserPlan=
        plan_original_native_numeric_parser(&original,&run_ref)?;
    // One indexed read of existing immutable sentence refs; source bootstrap
    // and numeric work use independent <=64-sentence bounded cursors.
    let mut bootstrap_db=Client::connect(numeric_database_url,NoTls)?;
    let known=bootstrap_db.query(
        "SELECT sentence_ref
         FROM execution.semantic_parser_sentence
         WHERE run_ref=$1 AND document_ref=$2
           AND representation_version=2",
        &[&run_ref,&original.manifest.document_ref])?
        .into_iter().map(|r|r.get::<_,String>(0))
        .collect::<BTreeSet<_>>();
    let expected=inventory.iter().map(|(start,end)|
        original_numeric_sentence_identity(&run_ref,&original.manifest.document_ref,*start,*end).0)
        .collect::<BTreeSet<_>>();
    if expected.len()!=inventory.len(){return Err(NumericPipelineError::ForeignRun);}
    // Keep the original registration's global metadata inventory; do not
    // assert globally complete token/entity evidence from a prefix-only tape.
    plan.expected_total_sentence_regions=expected.len();
    plan.expected_sentence_refs_sha256=digest_numeric_sentence_refs(&expected.iter().cloned().collect::<Vec<_>>());
    if !known.is_subset(&expected) {
        return Err(NumericPipelineError::ForeignRun);
    }
    // Preserve the immutable partition allocation coordinates on extension.
    // Original m149 sentence region sequence is start_char, independently of
    // this append-only parser partition allocation ordinal.
    let allocated=bootstrap_db.query("SELECT s.sentence_ref,p.sequence_no
        FROM execution.semantic_parser_sentence s JOIN execution.semantic_parser_partition p
          ON p.partition_ref=s.partition_ref
        WHERE s.run_ref=$1 AND s.document_ref=$2 AND s.representation_version=2",
        &[&run_ref,&original.manifest.document_ref])?;
    let mut ordinals=std::collections::BTreeMap::new();
    let mut seen=BTreeSet::new();
    for row in allocated{
        let id:String=row.get(0);
        let seq=usize::try_from(row.get::<_,i64>(1)).map_err(|_|NumericPipelineError::ForeignRun)?;
        if !seen.insert(seq)||ordinals.insert(id,seq).is_some(){return Err(NumericPipelineError::ForeignRun);}
    }
    if ordinals.len()!=known.len()||seen.iter().copied().ne(0..known.len()){
        return Err(NumericPipelineError::ForeignRun);
    }
    let mut next=known.len();
    for sentence in &mut plan.sentences{
        sentence.source_sentence_index=if let Some(seq)=ordinals.get(&sentence.sentence_ref){*seq}else{
            let seq=next;next+=1;seq
        };
    }
    let required_sentence_refs=plan.sentences.iter().filter(|s|required_prefix_region(s.start_char,focus))
        .map(|s|s.sentence_ref.clone()).collect::<Vec<_>>();
    let verified=checked_native_parser_sentence_refs(&mut bootstrap_db,&plan)?;
    if !verified.is_subset(&known){return Err(NumericPipelineError::ForeignRun);}
    let already_published_required=required_sentence_refs.iter().filter(|s|verified.contains(*s)).cloned().collect::<Vec<_>>();
    let already_verified_required_refs=already_published_required.clone();
    let (missing_required_sentence_refs,lean_workset_checked)=consumer_prefix_workset(
        &plan.original_source_sha256,&run_ref,&required_sentence_refs,&already_published_required,focus.is_some())?;
    let missing_required=missing_required_sentence_refs.iter().cloned().collect::<BTreeSet<_>>();
    let mut selected=Vec::new();
    let mut token_budget=0usize;
    for sentence in &plan.sentences{
        if !missing_required.contains(&sentence.sentence_ref){continue;}
        if sentence.tokens.len()>12_000 {
            return Err(NumericPipelineError::Budget);
        }
        if selected.len()>=max_sentences
            ||token_budget+sentence.tokens.len()>12_000 {
            break;
        }
        token_budget+=sentence.tokens.len();
        selected.push(sentence.clone());
    }
    let selected_sentence_refs=selected.iter().map(|s|s.sentence_ref.clone()).collect::<Vec<_>>();
    let revalidated_existing_sentence_refs=selected_sentence_refs.iter().filter(|s|known.contains(*s)).cloned().collect::<Vec<_>>();
    let newly_registered_sentence_refs=selected_sentence_refs.iter().filter(|s|!known.contains(*s)).cloned().collect::<Vec<_>>();
    let just_inserted=if selected.is_empty(){
        0
    }else{
        let mut slice=plan.clone();
        slice.sentences=selected;
        persist_original_numeric_parser_plan(
            numeric_database_url,source_root,&canonical,&slice,&original.manifest
        )?.numeric_sentences
    };
    // The source is already registered if there were no missing sentences;
    // it cannot be accepted solely because the run string matches.
    let source=bootstrap_db.query(
        "SELECT content_sha256,byte_count,char_count
         FROM execution.semantic_parser_source
         WHERE run_ref=$1 AND document_ref=$2",
        &[&run_ref,&original.manifest.document_ref])?;
    if source.len()!=1
        ||source[0].get::<_,Vec<u8>>(0)!=Sha256::digest(&canonical).to_vec()
        ||source[0].get::<_,i64>(1)!=canonical.len() as i64
        ||source[0].get::<_,i64>(2)!=original.manifest.source_codepoint_length as i64{
        return Err(NumericPipelineError::ForeignRun);
    }
    let current=bootstrap_db.query(
        "SELECT sentence_ref
         FROM execution.semantic_parser_sentence
         WHERE run_ref=$1 AND document_ref=$2 AND representation_version=2",
        &[&run_ref,&original.manifest.document_ref])?
        .into_iter().map(|r|r.get::<_,String>(0))
        .collect::<BTreeSet<_>>();
    if !current.is_subset(&expected) ||current.len()>expected.len(){
        return Err(NumericPipelineError::ForeignRun);
    }
    let verified_after=checked_native_parser_sentence_refs(&mut bootstrap_db,&plan)?;
    if !verified_after.is_subset(&current){return Err(NumericPipelineError::ForeignRun);}
    let pending_parser_sentences=plan.sentences.iter().filter(|s|required_prefix_region(s.start_char,focus)&&!verified_after.contains(&s.sentence_ref)).count();
    let physical_missing_required=required_sentence_refs.iter().filter(|s|!current.contains(*s)).count();
    let unrelated_pending_parser_sentences=expected.len()-current.len()-physical_missing_required;
    let exact_parser_sentence_receipts=pending_parser_sentences==0;
    let lex=lexicon(numeric_database_url)?;
    // The number staged in this invocation is a receipt fact, not a
    // claim about global source coverage or completed numeric work.
    let _newly_registered_sentences=just_inserted;
    let mut completed=0usize;
    let mut closed_sentence_region_starts=Vec::new();
    let mdl=NumericMdlWeights::default();
    while completed<max_sentences{
        let Some((lease,sentence_id))=lease_one(
            numeric_database_url,&run_ref,&original.manifest.document_ref,focus
        )? else {break};
        let tokens=numeric_sentence_tokens(
            numeric_database_url,&run_ref,
            &original.manifest.document_ref,sentence_id
        )?;
        let closure:NativeSentenceClosure=
            compose_numeric_sentence_native(lease.region_id,&tokens,&lex)?;
        let region_start:i64=bootstrap_db.query_one(
            "SELECT start_char FROM execution.semantic_pnf_region WHERE region_id=$1",
            &[&lease.region_id])?.get(0);
        if !required_prefix_region(region_start,focus){return Err(NumericPipelineError::ForeignRun);}
        publish_original_numeric_sentence(
            numeric_database_url,&original.manifest,&run_ref,
            &canonical,&lease,&closure,&mdl
        )?;
        closed_sentence_region_starts.push(region_start);
        completed+=1;
    }
    let mut db=Client::connect(numeric_database_url,NoTls)?;
    let remaining:i64=db.query_one(
        "SELECT count(*)::bigint
         FROM execution.semantic_pnf_work_item w
         JOIN execution.semantic_pnf_region r ON r.region_id=w.region_id
         WHERE w.run_ref=$1 AND w.document_ref=$2 AND w.operation_id=1
           AND w.state_id<>3 AND ($3::bigint IS NULL OR r.start_char<=$3)",
        &[&run_ref,&original.manifest.document_ref,&focus])?.get(0);
    let (entity_objects,entity_supports)=materialize_numeric_entity_occurrences(
        &mut db,&run_ref,&original.manifest.document_ref,max_sentences,focus)?;
    let (frontier_completed,frontier_remaining)=if focus.is_none()&&pending_parser_sentences==0
        &&remaining==0&&plan.full_document_coverage_paid {
        let (done,pending)=reduce_original_frontier_slice(
            &mut db,&run_ref,&original.manifest.document_ref,max_sentences)?;
        (done,Some(pending))
    }else{(0,None)};
    let mut residuals=plan.residuals.clone();
    residuals.push("native_entity_occurrences_are_unexported_candidate_anchors_not_complete_typed_binding".into());
    if frontier_remaining!=Some(0){
        residuals.push("original_parent_frontier_reduction_pending".into());
    }
    if unrelated_pending_parser_sentences>0 {residuals.push(format!("unrelated_parser_sentences_deferred:{}",unrelated_pending_parser_sentences));}
    if pending_parser_sentences>0 {
        residuals.push(format!(
            "original_numeric_parser_sentence_bootstrap_pending:{}",
            pending_parser_sentences
        ));
    }
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
        numeric_run_ref:run_ref,
        numeric_source_token_count:bootstrap_db.query_one(
            "SELECT count(*)::bigint FROM execution.semantic_parser_token
             WHERE run_ref=$1 AND document_ref=$2 AND representation_version=2",
            &[&plan.numeric_run_ref,&plan.source_document_ref]
        )?.get::<_,i64>(0) as usize,
        numeric_source_sentence_count:current.len(),
        numeric_source_entity_count:bootstrap_db.query_one(
            "SELECT count(*)::bigint FROM execution.semantic_parser_entity_span
             WHERE run_ref=$1 AND document_ref=$2 AND representation_version=2",
            &[&plan.numeric_run_ref,&plan.source_document_ref]
        )?.get::<_,i64>(0) as usize,
        source_full_document_coverage_paid:plan.full_document_coverage_paid
            &&pending_parser_sentences==0&&unrelated_pending_parser_sentences==0,
        consumer_prefix_start_bound:focus,
        original_observations_loaded:original.original_token_count,
        required_sentence_refs,lean_workset_checked,missing_required_sentence_refs,
        already_published_required_refs:already_published_required,
        newly_registered_sentence_refs,closed_sentence_region_starts,
        sentence_jobs_completed:completed,sentence_jobs_remaining:remaining as usize,
        entity_occurrence_objects_inserted:entity_objects,
        entity_occurrence_token_supports_inserted:entity_supports,
        frontier_jobs_completed:frontier_completed,frontier_jobs_remaining:frontier_remaining,
        exact_parser_sentence_receipts,already_verified_required_refs,revalidated_existing_sentence_refs,
        original_observation_scan_performed:true,bounded_replay_supported:false,all_original_typed_demands_covered:false,
        unique_document_identity_paid:false,level_three_identity_substitution_paid:false,
        external_qid_paid:false,historical_truth_paid:false,residuals,
    })
}

/// The Lean executable is the reference producer of the actual requested
/// workset; Rust independently checks the exact ordered output before any
/// publication. No source/semantic authority is inferred from process exit.
/// Legacy full-document publication remains unchanged.
fn consumer_prefix_workset(
    source_sha:&str,run:&str,required:&[String],published:&[String],scoped:bool,
)->Result<(Vec<String>,bool),NumericPipelineError>{
    use std::io::Write;
    use std::process::{Command,Stdio};
    let existing=published.iter().collect::<BTreeSet<_>>();
    let reference=required.iter().filter(|s|!existing.contains(s)).cloned().collect::<Vec<_>>();
    if !scoped{return Ok((reference,false));}
    let executable=match std::env::var_os("SLR_NATIVE_LEAN_WORKSET_EXECUTABLE"){
        Some(path)=>path,
        None=>return Ok((reference,false)),
    };
    let input=serde_json::json!({
        "schema":"slr-native-prefix-workset-input-v1","source_sha256":source_sha,
        "numeric_run_ref":run,"required_sentence_refs":required,
        "already_published_required_refs":published});
    let expected=serde_json::json!({
        "schema":"slr-native-prefix-workset-output-v1","source_sha256":source_sha,
        "numeric_run_ref":run,"required_sentence_refs":required,
        "already_published_required_refs":published,
        "missing_required_sentence_refs":reference,
        "semantic_admission_paid":false,"identity_paid":false});
    let bytes=serde_json::to_vec(&input).map_err(|_|NumericPipelineError::ForeignRun)?;
    let mut child=Command::new(executable).stdin(Stdio::piped()).stdout(Stdio::piped())
        .stderr(Stdio::piped()).spawn().map_err(|e|NumericPipelineError::Database(format!("Lean workset start: {e}")))?;
    child.stdin.take().ok_or(NumericPipelineError::ForeignRun)?.write_all(&bytes)
        .map_err(|e|NumericPipelineError::Database(format!("Lean workset input: {e}")))?;
    let output=child.wait_with_output().map_err(|e|NumericPipelineError::Database(format!("Lean workset process: {e}")))?;
    if !output.status.success(){return Err(NumericPipelineError::Database(format!("Lean workset: {}",String::from_utf8_lossy(&output.stderr))));}
    let got:serde_json::Value=serde_json::from_slice(&output.stdout)
        .map_err(|_|NumericPipelineError::ForeignRun)?;
    if got!=expected{return Err(NumericPipelineError::ForeignRun);}
    let selected=serde_json::from_value::<Vec<String>>(got["missing_required_sentence_refs"].clone())
        .map_err(|_|NumericPipelineError::ForeignRun)?;
    Ok((selected,true))
}

// Golden: Lean NumericPnfRustPortContract.consumerRegionWork; physical
// prefix scope includes the full containing focus sentence. No source omission
// becomes a semantic exclusion.
fn required_prefix_region(start:i64,focus:Option<i64>)->bool{focus.map_or(true,|end|start<=end)}
#[cfg(test)]
mod numeric_pnf_consumer_workset_golden{
    use super::*;
    #[test] fn numeric_pnf_no_unrelated_publication(){
        let source=[0,230,2215,2660,3603,4530,4600,10000];
        let published=[0,230,2215,2660,3603,4530];
        let work=source.iter().filter(|s|required_prefix_region(**s,Some(4534))&&!published.contains(s)).count();
        assert_eq!(work,0);
        assert!(required_prefix_region(4530,Some(4534)));
        assert!(!required_prefix_region(4600,Some(4534)));
    }
}
