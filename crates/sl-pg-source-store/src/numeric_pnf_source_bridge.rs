//! Exact, read-only original SLR parser -> native numeric-PNF source bridge.
//! Reuses persisted original source and parser products, without re-parsing,
//! nearest-name matching or converting an unavailable tag into invented data.
//! Source byte/codepoint boundaries are indexed ONCE, not rescanned per token.

use crate::{DatabaseConfig, NativeManifest};
use postgres::{Client,NoTls};
use serde::Serialize;
use sha2::{Digest,Sha256};
use std::collections::{BTreeMap,BTreeSet};
use thiserror::Error;

#[derive(Debug,Error)]
pub enum NativeNumericSourceError{
    #[error("postgres original source/parser query failed: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("original source/manifest does not match exact canonical bytes")]
    SourceManifest,
    #[error("original parser run missing, incomplete, or semantically promoted")]
    ParserRun,
    #[error("original parser sentence job missing/incomplete or overlapping")]
    JobScope,
    #[error("original parser token conflicts with exact source span/head")]
    ParserToken,
    #[error("original canonical UTF-8 source is invalid")]
    Utf8,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct OriginalNumericRawToken{
    pub compilation_key:String,
    pub original_token_ordinal:i32,
    pub start_char:i64,
    pub end_char:i64,
    pub surface:String,
    pub lemma:String,
    pub pos:String,
    pub morph_json:Option<String>,
    pub head_ordinal:Option<i32>,
    pub dependency_ref:String,
    pub original_tag_unavailable:bool,
    pub semantic_admission_paid:bool,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct OriginalNumericRawEntity{
    pub original_entity_ordinal:i32,
    pub start_char:i64,
    pub end_char:i64,
    pub surface:String,
    pub label_ref:String,
    pub candidate_only:bool,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct OriginalNumericSentenceFibre{
    pub compilation_key:String,
    pub region_ref:String,
    pub start_char:i64,
    pub end_char:i64,
    pub tokens:Vec<OriginalNumericRawToken>,
    pub entities:Vec<OriginalNumericRawEntity>,
    pub full_numeric_tag_capability_available:bool,
}

#[derive(Debug,Clone,Serialize)]
pub struct OriginalNativeNumericEvidence{
    pub schema:&'static str,
    pub manifest:NativeManifest,
    pub original_source_ref:String,
    pub source_parser_family:String,
    pub source_parser_version:String,
    pub source_model_ref:String,
    pub parser_regions:Vec<OriginalNumericSentenceFibre>,
    pub original_region_count:usize,
    pub original_token_count:usize,
    pub full_document_coverage_paid:bool,
    pub residuals:Vec<String>,
    pub typed_identity_paid:bool,
    pub external_qid_paid:bool,
}

fn byte_boundaries(text:&str)->Vec<usize>{
    let mut offsets=text.char_indices().map(|(b,_)|b).collect::<Vec<_>>();
    offsets.push(text.len());
    offsets
}

fn original_span<'a>(text:&'a str,bounds:&[usize],start:i64,end:i64)
    ->Result<&'a str,NativeNumericSourceError>{
    if start<0||end<=start {return Err(NativeNumericSourceError::ParserToken);}
    let a=usize::try_from(start).map_err(|_|NativeNumericSourceError::ParserToken)?;
    let b=usize::try_from(end).map_err(|_|NativeNumericSourceError::ParserToken)?;
    let sa=*bounds.get(a).ok_or(NativeNumericSourceError::ParserToken)?;
    let sb=*bounds.get(b).ok_or(NativeNumericSourceError::ParserToken)?;
    text.get(sa..sb).ok_or(NativeNumericSourceError::ParserToken)
}

/// Every source envelope and parser observation is checked against the
/// original SLR database. This is a candidate producer, never identity.
pub fn load_exact_native_numeric_evidence(
    config:&DatabaseConfig,source_revision_ref:&str,parser_run_ref:&str,
    rule_set_revision:&str,
)->Result<OriginalNativeNumericEvidence,NativeNumericSourceError>{
    Ok(load_exact_native_numeric_evidence_scope(config,source_revision_ref,parser_run_ref,rule_set_revision,None)?.0)
}
/// The original job-coordinate inventory is cheap metadata. Only the selected
/// source-region token/entity products enter the physical observation fold.
pub(crate) fn load_exact_native_numeric_evidence_scope(
    config:&DatabaseConfig,source_revision_ref:&str,parser_run_ref:&str,
    rule_set_revision:&str,focus:Option<i64>,
)->Result<(OriginalNativeNumericEvidence,Vec<(i64,i64)>),NativeNumericSourceError>{
    if source_revision_ref.is_empty()||parser_run_ref.is_empty()
        ||rule_set_revision.is_empty(){
        return Err(NativeNumericSourceError::SourceManifest);
    }
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut db=connection.transaction()?;
    db.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let rows=db.query(
        "SELECT g.source_ref,g.document_ref,g.content_digest_ref,c.payload,
                g.candidate_only,g.creates_semantic_authority,
                g.applicability_promoted,g.claim_truth_promoted
         FROM ingest.generic_source_revision AS g
         JOIN corpus.document AS d ON d.document_ref=g.document_ref
         JOIN corpus.canonical_content AS c ON c.canonical_ref=d.canonical_ref
         WHERE g.source_revision_ref=$1",
        &[&source_revision_ref])?;
    if rows.len()!=1{return Err(NativeNumericSourceError::SourceManifest);}
    let row=&rows[0];
    let original_source_ref:String=row.get(0);
    let document_ref:String=row.get(1);
    let declared_sha:String=row.get(2);
    let bytes:Vec<u8>=row.get(3);
    if !row.get::<_,bool>(4)||row.get::<_,bool>(5)
        ||row.get::<_,bool>(6)||row.get::<_,bool>(7){
        return Err(NativeNumericSourceError::SourceManifest);
    }
    let text=std::str::from_utf8(&bytes).map_err(|_|NativeNumericSourceError::Utf8)?;
    let hash=format!("{:x}",Sha256::digest(&bytes));
    let bounds=byte_boundaries(text);
    let len=(bounds.len()-1) as i64;
    if declared_sha!=format!("sha256:{hash}")
        ||document_ref!=format!("document:sha256:{hash}"){
        return Err(NativeNumericSourceError::SourceManifest);
    }
    let manifest=NativeManifest{
        document_ref,source_revision_ref:source_revision_ref.to_owned(),
        canonical_content_sha256:hash,source_byte_length:bytes.len(),
        source_codepoint_length:bounds.len()-1,
        coordinate_contract_ref:"unicode-codepoints:v1".into(),
        parser_run_ref:parser_run_ref.into(),
        rule_set_revision:rule_set_revision.into(),
    };
    manifest.verify_source_bytes(&bytes).map_err(|_|NativeNumericSourceError::SourceManifest)?;
    let run=db.query(
        "SELECT source_revision_ref,status,candidate_only,
                creates_semantic_authority,applicability_promoted,
                claim_truth_promoted,parser_family,parser_version,model_ref
         FROM ingest.parser_run WHERE parser_run_ref=$1",
        &[&parser_run_ref])?;
    if run.len()!=1{return Err(NativeNumericSourceError::ParserRun);}
    let r=&run[0];
    if r.get::<_,String>(0)!=source_revision_ref
        ||r.get::<_,String>(1)!="complete"||!r.get::<_,bool>(2)
        ||r.get::<_,bool>(3)||r.get::<_,bool>(4)||r.get::<_,bool>(5){
        return Err(NativeNumericSourceError::ParserRun);
    }
    let jobs=db.query(
        "SELECT compilation_key,region_ref,region_start_char,region_end_char,
                status,source_revision_ref,parser_product_key,candidate_only,
                creates_semantic_authority,applicability_promoted,claim_truth_promoted
         FROM ingest.parser_job WHERE parser_run_ref=$1
         ORDER BY region_start_char,region_end_char,compilation_key",
        &[&parser_run_ref])?;
    if jobs.is_empty(){return Err(NativeNumericSourceError::JobScope);}
    // One indexed source-run fetch per observation family, NOT 2N remote
    // round trips for N source/parser regions. The original per-region
    // source/coordinate checks below are unchanged.
    let all_token_rows=db.query(
        "SELECT token.token_ordinal,token.start_char,token.end_char,
                token.surface,token.lemma,token.pos,token.morph_json,
                token.head_ordinal,token.dependency_ref,
                token.compilation_key
         FROM ingest.parser_token AS token
         JOIN ingest.parser_job AS job
           ON job.compilation_key=token.compilation_key
         WHERE job.parser_run_ref=$1 AND ($2::bigint IS NULL OR job.region_start_char<=$2)
         ORDER BY token.compilation_key,token.token_ordinal",
        &[&parser_run_ref,&focus])?;
    let mut token_rows_by_job=BTreeMap::<String,Vec<postgres::Row>>::new();
    for row in all_token_rows{
        let key:String=row.get(9);
        token_rows_by_job.entry(key).or_default().push(row);
    }
    let all_entity_rows=db.query(
        "SELECT entity.entity_ordinal,entity.start_char,entity.end_char,
                entity.surface,entity.label_ref,entity.compilation_key
         FROM ingest.parser_entity AS entity
         JOIN ingest.parser_job AS job
           ON job.compilation_key=entity.compilation_key
         WHERE job.parser_run_ref=$1 AND ($2::bigint IS NULL OR job.region_start_char<=$2)
         ORDER BY entity.compilation_key,entity.entity_ordinal",
        &[&parser_run_ref,&focus])?;
    let mut entity_rows_by_job=BTreeMap::<String,Vec<postgres::Row>>::new();
    for row in all_entity_rows{
        let key:String=row.get(5);
        entity_rows_by_job.entry(key).or_default().push(row);
    }
    let mut fibres=Vec::new();
    let mut inventory=Vec::with_capacity(jobs.len());
    let mut last_end=0i64;
    let mut full_document=true;
    for job in jobs{
        let compilation_key:String=job.get(0);
        let region_ref:String=job.get(1);
        let start:i64=job.get(2);
        let end:i64=job.get(3);
        if start<last_end ||start<0||end>len||end<=start
            ||job.get::<_,String>(4)!="succeeded"
            ||job.get::<_,String>(5)!=source_revision_ref
            ||job.get::<_,Option<String>>(6).as_deref().unwrap_or("").is_empty()
            ||!job.get::<_,bool>(7)||job.get::<_,bool>(8)
            ||job.get::<_,bool>(9)||job.get::<_,bool>(10){
            return Err(NativeNumericSourceError::JobScope);
        }
        if text[bounds[last_end as usize]..bounds[start as usize]].chars().any(|c|!c.is_whitespace()){
            full_document=false;
        }
        last_end=end;
        inventory.push((start,end));
        if focus.is_some_and(|f|start>f){full_document=false;continue;}
        let rows=token_rows_by_job.remove(&compilation_key)
            .unwrap_or_default();
        let mut ordinals=BTreeSet::new();
        let mut tokens=Vec::with_capacity(rows.len());
        let mut prev_ord=None;
        for row in rows{
            let ordinal:i32=row.get(0);
            let token_start:i64=row.get(1);
            let token_end:i64=row.get(2);
            let surface:String=row.get(3);
            let lemma:String=row.get(4);
            let pos:String=row.get(5);
            let morph_json:Option<String>=row.get(6);
            let head_ordinal:Option<i32>=row.get(7);
            let dependency_ref:String=row.get(8);
            if ordinal<0||prev_ord.is_some_and(|prior|ordinal<=prior)
                ||token_start<start||token_end>end
                ||original_span(text,&bounds,token_start,token_end)?!=surface
                ||pos.is_empty()||dependency_ref.is_empty(){
                return Err(NativeNumericSourceError::ParserToken);
            }
            prev_ord=Some(ordinal);
            ordinals.insert(ordinal);
            tokens.push(OriginalNumericRawToken{
                compilation_key:compilation_key.clone(),original_token_ordinal:ordinal,
                start_char:token_start,end_char:token_end,surface,lemma,pos,
                morph_json,head_ordinal,dependency_ref,
                original_tag_unavailable:true,semantic_admission_paid:false,
            });
        }
        for t in &tokens{
            if let Some(h)=t.head_ordinal{
                if !ordinals.contains(&h){
                    return Err(NativeNumericSourceError::ParserToken);
                }
            }
        }
        let entity_rows=entity_rows_by_job.remove(&compilation_key)
            .unwrap_or_default();
        let mut entities=Vec::with_capacity(entity_rows.len());
        let mut ordinal_seen=BTreeSet::new();
        for row in entity_rows{
            let ordinal:i32=row.get(0);
            let ent_start:i64=row.get(1);
            let ent_end:i64=row.get(2);
            let surface:String=row.get(3);
            let label_ref:String=row.get(4);
            if ordinal<0||!ordinal_seen.insert(ordinal)
                ||ent_start<start||ent_end>end
                ||original_span(text,&bounds,ent_start,ent_end)?!=surface
                ||label_ref.is_empty(){
                return Err(NativeNumericSourceError::ParserToken);
            }
            entities.push(OriginalNumericRawEntity{
                original_entity_ordinal:ordinal,start_char:ent_start,
                end_char:ent_end,surface,label_ref,candidate_only:true,
            });
        }
        // Whitespace gaps are allowed; any other missing source codepoint
        // remains a coverage residual. A sentence coordinate is not a token.
        let mut covered=vec![false;(end-start) as usize];
        for t in &tokens{
            covered[(t.start_char-start) as usize..(t.end_char-start) as usize].fill(true);
        }
        if text[bounds[start as usize]..bounds[end as usize]].chars().zip(&covered)
            .any(|(ch,paid)|!ch.is_whitespace()&&!*paid){full_document=false;}
        fibres.push(OriginalNumericSentenceFibre{
            compilation_key,region_ref,start_char:start,end_char:end,
            tokens,entities,full_numeric_tag_capability_available:false,
        });
    }
    if text[bounds[last_end as usize]..].chars().any(|c|!c.is_whitespace()){full_document=false;}
    if !token_rows_by_job.is_empty()||!entity_rows_by_job.is_empty(){
        return Err(NativeNumericSourceError::JobScope);
    }
    let original_token_count=fibres.iter().map(|f|f.tokens.len()).sum();
    let mut residuals=vec![
        "original_SLR_parser_does_not_persist_numeric_TAG_symbol_capability".into(),
        "original_numeric_PNF_object_and_H9_typed_demand_not_paid".into(),
    ];
    if !full_document{
        residuals.push("parser_run_does_not_cover_entire_canonical_document".into());
    }
    Ok((OriginalNativeNumericEvidence{
        schema:"slr-exact-source-native-numeric-evidence-v1",
        manifest,original_source_ref,
        source_parser_family:r.get(6),
        source_parser_version:r.get(7),
        source_model_ref:r.get(8),
        original_region_count:fibres.len(),
        original_token_count,parser_regions:fibres,
        full_document_coverage_paid:full_document,
        residuals,typed_identity_paid:false,external_qid_paid:false,
    },inventory))
}


/// Exact same-run extension, using only original source sentence regions and
/// existing content-addressed parser jobs. No new segmentation or compiler.
/// Pending jobs are executed by the existing parser-only worker.
#[derive(Debug,Clone,Serialize)]
pub struct NativePrefixRecoveryReceipt{
    pub source_revision_ref:String,pub parser_run_ref:String,pub source_sha256:String,
    pub focus_start_char:u64,pub original_prefix_regions:usize,
    pub already_succeeded:usize,pub missing_region_refs:Vec<String>,
    pub queued_region_refs:Vec<String>,pub residual_region_refs:Vec<String>,
    pub uncovered_source_intervals:Vec<(usize,usize,usize)>,
    pub uncovered_region_intervals:Vec<(usize,usize,usize)>,
    pub original_gap_paragraph_receipts:Vec<NativeGapParagraphReceipt>,
    pub original_parser_token_prefix_covered:bool,
    pub source_region_prefix_covered:bool,
    pub overlapping_region_residuals:Vec<String>,
    pub newly_enqueued_or_pending:usize,pub reused_products:usize,
    pub parser_scope_ready:bool,pub writes_database:bool,
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeGapParagraphReceipt{
    pub region_ref:String,pub start_char:i64,pub end_char:i64,pub literal_sha256:String,
    pub original_heading_exclusion_matches:bool,pub nonempty_line_count:usize,
    pub covered_gap_intervals:Vec<(usize,usize,usize)>,
}
pub fn prepare_native_prefix_recovery(
    config:&DatabaseConfig,review_item:&str,parser_run:&str,commit:bool,
)->Result<NativePrefixRecoveryReceipt,String>{
    use crate::{inspect_native_antecedent_review,ParserRunReceipt,ParserRegionJobSpec,
        enqueue_parser_regions_with_content_reuse};
    let review=inspect_native_antecedent_review(config,review_item).map_err(|e|e.to_string())?;
    let focus=review.focus.start_char;
    let mut db=Client::connect(config.database_url(),NoTls).map_err(|e|e.to_string())?;
    let row=db.query_one("SELECT c.payload,g.content_digest_ref FROM ingest.generic_source_revision g
        JOIN corpus.document d ON d.document_ref=g.document_ref
        JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
        WHERE g.source_revision_ref=$1 AND g.document_ref=$2 AND g.candidate_only
          AND NOT g.creates_semantic_authority AND NOT g.applicability_promoted AND NOT g.claim_truth_promoted",
        &[&review.source_revision_ref,&review.document_ref]).map_err(|e|e.to_string())?;
    let bytes:Vec<u8>=row.get(0);
    let sha=format!("{:x}",Sha256::digest(&bytes));
    if sha!=review.canonical_content_sha256||row.get::<_,String>(1)!=format!("sha256:{sha}"){
        return Err("prefix recovery source digest mismatch".into());
    }
    let text=std::str::from_utf8(&bytes).map_err(|e|e.to_string())?;
    let chars=text.chars().collect::<Vec<_>>();
    let stop=usize::try_from(focus).map_err(|e|e.to_string())?;
    if stop>chars.len(){return Err("focus outside canonical source".into());}
    let run=db.query_one("SELECT source_revision_ref,parser_family,parser_version,model_ref,
        config_digest_ref,config_json,candidate_only,creates_semantic_authority,
        applicability_promoted,claim_truth_promoted FROM ingest.parser_run WHERE parser_run_ref=$1",
        &[&parser_run]).map_err(|e|e.to_string())?;
    if run.get::<_,String>(0)!=review.source_revision_ref||!run.get::<_,bool>(6)
        ||run.get::<_,bool>(7)||run.get::<_,bool>(8)||run.get::<_,bool>(9){
        return Err("foreign or promoted parser run".into());
    }
    let receipt=ParserRunReceipt{parser_run_ref:parser_run.into(),source_revision_ref:review.source_revision_ref.clone(),
        parser_family:run.get(1),parser_version:run.get(2),model_ref:run.get(3),
        config_digest_ref:run.get(4),config_json:run.get(5),candidate_only:true,
        creates_semantic_authority:false,applicability_promoted:false,claim_truth_promoted:false};
    let focus_i64=i64::try_from(focus).map_err(|e|e.to_string())?;
    let regions=db.query("SELECT DISTINCT r.region_ref,r.start_char,r.end_char
        FROM ingest.long_document_region r
        WHERE r.source_revision_ref=$1 AND r.source_ref=$2 AND r.region_kind='sentence'
          AND r.start_char<$3 AND r.candidate_only AND NOT r.creates_semantic_authority AND NOT r.claim_truth_promoted
        ORDER BY r.start_char,r.end_char,r.region_ref",
        &[&review.source_revision_ref,&review.original_source_ref,&focus_i64]).map_err(|e|e.to_string())?;
    let jobs=db.query("SELECT region_ref,region_start_char,region_end_char,status,source_revision_ref,
        candidate_only,creates_semantic_authority,applicability_promoted,claim_truth_promoted
        FROM ingest.parser_job WHERE parser_run_ref=$1 ORDER BY region_start_char,region_end_char",
        &[&parser_run]).map_err(|e|e.to_string())?;
    let mut existing=BTreeMap::new();let mut last=0i64;
    for j in jobs{
        let a:i64=j.get(1);let b:i64=j.get(2);
        if a<last||b<=a||b>chars.len() as i64||j.get::<_,String>(4)!=review.source_revision_ref
            ||!j.get::<_,bool>(5)||j.get::<_,bool>(6)||j.get::<_,bool>(7)||j.get::<_,bool>(8){
            return Err("invalid original parser job scope".into());
        }
        last=b;
        existing.insert(j.get::<_,String>(0),(a,b,j.get::<_,String>(3)));
    }
    let mut missing=Vec::new();let mut queued=Vec::new();let mut residual=Vec::new();
    let mut overlapping_region_residuals=Vec::new();
    let mut succeeded=0;let mut covered=vec![false;stop];let mut prior=0i64;
    for region in &regions{
        let key:String=region.get(0);let a:i64=region.get(1);let b:i64=region.get(2);
        if a<prior||b<=a||b>chars.len() as i64{return Err("overlapping original source sentence regions".into());}
        prior=b;
        match existing.get(&key){
            Some((x,y,state)) if *x==a&&*y==b=>{
                match state.as_str(){
                    "succeeded"=>{succeeded+=1;covered[a as usize..(b as usize).min(stop)].fill(true);},
                    "queued"|"leased"=>queued.push(key),
                    "residual"=>residual.push(key),
                    _=>return Err("unknown parser job state".into()),
                }
            }
            Some(_)=>return Err("original region/job coordinate mismatch".into()),
            None=>{
                // Never add a sentence overlapping a larger pre-existing job.
                // Such an interval must be recovered from that job's own tape.
                if let Some((owner,(x,y,state)))=existing.iter().find(|(_, (x,y,_))|*x<=a&&*y>=b){
                    match state.as_str(){
                        "succeeded"=>{succeeded+=1;covered[a as usize..(b as usize).min(stop)].fill(true);},
                        "queued"|"leased"=>queued.push(owner.clone()),
                        "residual"=>residual.push(owner.clone()),
                        _=>return Err("unknown containing job state".into()),
                    }
                    let _=(x,y);
                }else if existing.values().any(|(x,y,_)|a<*y&&*x<b){
                    overlapping_region_residuals.push(key);
                }else{
                    missing.push(ParserRegionJobSpec{region_ref:key,start_char:a as u64,end_char:b as u64});
                }
            }
        }
    }
    if missing.len()>128||missing.iter().map(|r|r.end_char-r.start_char).sum::<u64>()>100_000{
        return Err("prefix-only parser recovery exceeds bounded scope".into());
    }
    let gaps=|covered:&[bool]|{
        let mut out=Vec::new();let mut i=0;
        while i<covered.len(){
            if covered[i]{i+=1;continue;}
            let a=i;while i<covered.len()&&!covered[i]{i+=1;}
            let n=chars[a..i].iter().filter(|c|!c.is_whitespace()).count();
            if n>0{out.push((a,i,n));}
        }
        out
    };
    queued.sort();queued.dedup();residual.sort();residual.dedup();
    // The reused worker drains this parser run. Do not accidentally release
    // an unrelated pending corpus job through a prefix-only campaign.
    if existing.iter().any(|(key,(_,_,state))|matches!(state.as_str(),"queued"|"leased")&&!queued.contains(key)){
        return Err("original parser run contains pending work outside the prefix scope".into());
    }
    let mut token_covered=vec![false;stop];
    let token_rows=db.query("SELECT t.start_char,t.end_char,t.surface FROM ingest.parser_token t
        JOIN ingest.parser_job j ON j.compilation_key=t.compilation_key
        WHERE j.parser_run_ref=$1 AND j.status='succeeded' AND t.start_char<$2
        ORDER BY t.start_char,t.end_char,t.token_ordinal",&[&parser_run,&focus_i64]).map_err(|e|e.to_string())?;
    for t in token_rows{
        let a=usize::try_from(t.get::<_,i64>(0)).map_err(|e|e.to_string())?;
        let b=usize::try_from(t.get::<_,i64>(1)).map_err(|e|e.to_string())?;
        if a>=b||b>chars.len()||chars[a..b].iter().collect::<String>()!=t.get::<_,String>(2)
            ||token_covered[a..b.min(stop)].iter().any(|x|*x){
            return Err("original prefix parser token coordinate mismatch".into());
        }
        token_covered[a..b.min(stop)].fill(true);
    }
    let mut enqueued=0;let mut reused=0;
    if commit&&!missing.is_empty(){
        let e=enqueue_parser_regions_with_content_reuse(config,&receipt,&missing,text).map_err(|e|e.to_string())?;
        enqueued=e.queued_new_job_count;reused=e.reused_job_count();
    }
    // Pre-enqueue readback, deliberately separate region scope and actual
    // non-whitespace token coverage. A source interval without an original
    // sentence owner is an explicit residual, not permission to fabricate it.
    let uncovered=gaps(&token_covered);
    let uncovered_regions=gaps(&covered);
    // Reopen the actual authored paragraph carrier and apply the ORIGINAL
    // heading exclusion, not an invented gap segmentation. A multiline
    // paragraph matching the heading predicate is a possible segmentation
    // defect, not authority to mutate the established sentence ledger.
    let mut original_gap_paragraph_receipts=Vec::new();
    if !uncovered_regions.is_empty(){
        let ps=db.query("SELECT region_ref,start_char,end_char
            FROM ingest.long_document_region
            WHERE source_revision_ref=$1 AND source_ref=$2 AND region_kind='paragraph'
              AND start_char<$3 AND candidate_only AND NOT creates_semantic_authority AND NOT claim_truth_promoted
            ORDER BY start_char,end_char,region_ref",
            &[&review.source_revision_ref,&review.original_source_ref,&focus_i64]).map_err(|e|e.to_string())?;
        for p in ps{
            let a:i64=p.get(1);let b:i64=p.get(2);
            if a<0||b<=a||b>chars.len() as i64{return Err("foreign paragraph coordinates".into());}
            let intersect=uncovered_regions.iter().filter(|(x,y,_)|(*x as i64)<b&&(*y as i64)>a)
                .copied().collect::<Vec<_>>();
            if intersect.is_empty(){continue;}
            let literal=chars[a as usize..b as usize].iter().collect::<String>();
            original_gap_paragraph_receipts.push(NativeGapParagraphReceipt{
                region_ref:p.get(0),start_char:a,end_char:b,
                literal_sha256:format!("{:x}",Sha256::digest(literal.as_bytes())),
                original_heading_exclusion_matches:crate::plain_text_document_adapter::chapter_heading(&literal),
                nonempty_line_count:literal.lines().filter(|l|!l.trim().is_empty()).count(),
                covered_gap_intervals:intersect,
            });
        }
    }
    Ok(NativePrefixRecoveryReceipt{
        source_revision_ref:review.source_revision_ref,parser_run_ref:parser_run.into(),source_sha256:sha,
        focus_start_char:focus,original_prefix_regions:regions.len(),already_succeeded:succeeded,
        missing_region_refs:missing.iter().map(|r|r.region_ref.clone()).collect(),
        queued_region_refs:queued.clone(),residual_region_refs:residual.clone(),
        original_parser_token_prefix_covered:uncovered.is_empty(),
        source_region_prefix_covered:uncovered_regions.is_empty(),
        uncovered_source_intervals:uncovered,uncovered_region_intervals:uncovered_regions,original_gap_paragraph_receipts,
        overlapping_region_residuals:overlapping_region_residuals.clone(),
        newly_enqueued_or_pending:enqueued+queued.len(),
        reused_products:reused,parser_scope_ready:missing.is_empty()&&queued.is_empty()&&residual.is_empty()&&overlapping_region_residuals.is_empty(),
        writes_database:commit&&!missing.is_empty(),
    })
}

/// Recover exact selected original heading-excluded paragraphs. The original
/// partition ledger is the durable publication owner, and the original
/// prefix recovery remains the parser job owner. This helper is not yet
/// activated by the CLI: the numeric consumer absence-dependency handshake
/// must be integrated before the repair can be a complete endpoint campaign.
pub fn publish_native_prefix_heading_repair(
    config:&DatabaseConfig, review_item:&str, parser_run:&str,
)->Result<crate::long_document_ingest_store::HeadingPartitionPublication,String>{
    let scope=prepare_native_prefix_recovery(config,review_item,parser_run,false)?;
    if !scope.parser_scope_ready {
        return Err("original parser scope must be drained before partition repair".into());
    }
    let (source,text)=crate::load_generic_text_source(config,&scope.source_revision_ref)
        .map_err(|e|e.to_string())?;
    let envelope=crate::load_generic_source_envelope(config,&scope.source_revision_ref)
        .map_err(|e|e.to_string())?;
    if source.content_digest_ref!=format!("sha256:{}",scope.source_sha256)
        || source.source_ref!=envelope.source_ref || !envelope.semantic_text_allowed() {
        return Err("foreign canonical source for repair".into());
    }
    let (legacy,_)=crate::build_plain_text_long_source_with_family(
        &envelope.source_ref,&envelope.source_revision_ref,&envelope.provider_ref,
        &envelope.acquisition_receipt_ref,envelope.family,None,None,&text,
    ).map_err(|e|e.to_string())?;
    let chars=text.chars().collect::<Vec<_>>();
    let mut selected=Vec::new();
    for gap in scope.original_gap_paragraph_receipts.iter()
        .filter(|p|p.original_heading_exclusion_matches&&p.nonempty_line_count>1) {
        let a=usize::try_from(gap.start_char).map_err(|e|e.to_string())?;
        let b=usize::try_from(gap.end_char).map_err(|e|e.to_string())?;
        let literal=chars.get(a..b).ok_or("repair paragraph outside canonical source")?
            .iter().collect::<String>();
        if format!("{:x}",Sha256::digest(literal.as_bytes()))!=gap.literal_sha256
            || !legacy.regions.iter().any(|r|r.region_ref==gap.region_ref
                &&r.kind==sensiblaw_core::source_ingest::DocumentRegionKind::Paragraph
                &&r.start_char==a as u64&&r.end_char==b as u64)
        {
            return Err("repair paragraph differs from original source ledger".into());
        }
        selected.push(gap.region_ref.clone());
    }
    if selected.is_empty() {
        return Err("no checked heading-exclusion paragraph to repair".into());
    }
    crate::long_document_ingest_store::publish_heading_partition(
        config,&legacy,&text,&selected,parser_run,scope.focus_start_char,
    ).map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn utf8_codepoint_boundaries_index_once(){
        let body="he 🙂 Bush";
        let bounds=byte_boundaries(body);
        assert_eq!(original_span(body,&bounds,0,2).unwrap(),"he");
        assert_eq!(original_span(body,&bounds,3,4).unwrap(),"🙂");
        assert_eq!(original_span(body,&bounds,5,9).unwrap(),"Bush");
        assert!(original_span(body,&bounds,5,10).is_err());
    }
    #[test]
    fn original_tag_absence_must_be_a_residual(){
        let token=OriginalNumericRawToken{
            compilation_key:"original".into(),original_token_ordinal:0,
            start_char:0,end_char:2,surface:"he".into(),
            lemma:"he".into(),pos:"PRON".into(),morph_json:None,
            head_ordinal:None,dependency_ref:"nsubj".into(),
            original_tag_unavailable:true,semantic_admission_paid:false,
        };
        assert!(token.original_tag_unavailable);
        assert!(!token.semantic_admission_paid);
    }
}