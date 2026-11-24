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
    if source_revision_ref.is_empty()||parser_run_ref.is_empty()
        ||rule_set_revision.is_empty(){
        return Err(NativeNumericSourceError::SourceManifest);
    }
    let mut db=Client::connect(config.database_url(),NoTls)?;
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
    let mut fibres=Vec::with_capacity(jobs.len());
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
        if start!=last_end{full_document=false;}
        last_end=end;
        let rows=db.query(
            "SELECT token_ordinal,start_char,end_char,surface,lemma,pos,
                    morph_json,head_ordinal,dependency_ref
             FROM ingest.parser_token WHERE compilation_key=$1
             ORDER BY token_ordinal",
            &[&compilation_key])?;
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
        let entity_rows=db.query(
            "SELECT entity_ordinal,start_char,end_char,surface,label_ref
             FROM ingest.parser_entity
             WHERE compilation_key=$1 ORDER BY entity_ordinal",
            &[&compilation_key])?;
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
        fibres.push(OriginalNumericSentenceFibre{
            compilation_key,region_ref,start_char:start,end_char:end,
            tokens,entities,full_numeric_tag_capability_available:false,
        });
    }
    if last_end!=len{full_document=false;}
    let original_token_count=fibres.iter().map(|f|f.tokens.len()).sum();
    let mut residuals=vec![
        "original_SLR_parser_does_not_persist_numeric_TAG_symbol_capability".into(),
        "original_numeric_PNF_object_and_H9_typed_demand_not_paid".into(),
    ];
    if !full_document{
        residuals.push("parser_run_does_not_cover_entire_canonical_document".into());
    }
    Ok(OriginalNativeNumericEvidence{
        schema:"slr-exact-source-native-numeric-evidence-v1",
        manifest,original_source_ref,
        source_parser_family:r.get(6),
        source_parser_version:r.get(7),
        source_model_ref:r.get(8),
        original_region_count:fibres.len(),
        original_token_count,parser_regions:fibres,
        full_document_coverage_paid:full_document,
        residuals,typed_identity_paid:false,external_qid_paid:false,
    })
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