//! Exact SLR -> ORIGINAL SensibLaw numeric parser v2 PostgreSQL bootstrap.
//!
//! One bounded, idempotent transaction for source, partition, sentences,
//! numeric tokens, dependency heads, and entity spans. Original schema
//! authority is retained; no PNF identity witness or Level-3 row is inserted.
//! This producer does not re-run spaCy. Only original source-backed observations
//! and explicit POS->TAG fallback are admitted. Unicode casefold uses the
//! full non-Turkic casefold table rather than an ASCII approximation.

use crate::{NativeNumericParserPlan,NativeManifest,
    NativeNumericParserSentencePlan};
use crate::numeric_pnf_parser_plan::digest_numeric_sentence_refs;
use postgres::{Client,NoTls,Transaction};
use serde_json::Value;
use sha2::{Digest,Sha256};
use std::collections::{BTreeMap,BTreeSet};
use std::fs::{self,File,OpenOptions};
use std::io::Write;
use std::path::{Path,PathBuf};
use thiserror::Error;
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug,Error)]
pub enum NumericParserStoreError {
    #[error("original numeric PostgreSQL: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("canonical source or numeric parser plan does not match")]
    ForeignSource,
    #[error("numeric source artifact could not be durably written: {0}")]
    Artifact(#[from] std::io::Error),
    #[error("source parser coordinates, heads or overlapping sentence scope invalid")]
    InvalidSentence,
    #[error("malformed original morphology evidence")]
    MalformedMorphology,
    #[error("original numeric source run conflicts with existing immutable rows")]
    ExistingConflict,
    #[error("numeric parser stage exceeded fixed bounded budget")]
    ResourceLimit,
}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct NumericParserStoreReceipt {
    pub source_sha256:String,
    pub numeric_run_ref:String,
    pub numeric_document_ref:String,
    pub original_parser_run_ref:String,
    pub original_source_revision_ref:String,
    pub numeric_sentences:usize,
    pub numeric_tokens:usize,
    pub numeric_entities:usize,
    pub coverage_complete:bool,
    pub identity_paid:bool,
    pub level_three_paid:bool,
}

/// All symbols originate from source annotations, not inferred world actors.
fn normalize(kind:i16,text:&str)->Result<String,NumericParserStoreError>{
    let nfc=text.nfc().collect::<String>();
    if nfc.is_empty(){return Err(NumericParserStoreError::ForeignSource);}
    let canonical=match kind{
        // Original Python normalize_symbol: these kinds use casefold.
        2|5|11|12|13|14|15|16|17|18|19=>
            nfc.chars().case_fold().collect::<String>(),
        // POS/TAG/NER/morph feature uses Python upper().
        3|4|8|6=>nfc.to_uppercase(),
        _=>nfc,
    };
    Ok(canonical)
}
fn sha_ref(domain:&str,parts:&[&str])->String{
    let mut hash=Sha256::new();
    for part in std::iter::once(domain).chain(parts.iter().copied()){
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}",hash.finalize())
}
fn byte_boundaries(text:&str)->Vec<usize>{
    let mut b=text.char_indices().map(|(p,_)|p).collect::<Vec<_>>();
    b.push(text.len());
    b
}
/// Port of original SensibLaw authored_paragraphs: one linear Unicode
/// codepoint scan over immutable source, preserving blank-line boundaries.
/// Paragraphs remain authoritative original source coordinates, not semantic
/// co-presence or identity evidence.
pub fn authored_numeric_paragraphs(text:&str)->Vec<(usize,usize)>{
    let chars=text.chars().collect::<Vec<_>>();
    if chars.is_empty(){return Vec::new();}
    let mut out=Vec::new();
    let mut current=0usize;
    let mut i=0usize;
    while i<chars.len(){
        if chars[i]=='\n'{
            let mut j=i+1;
            while j<chars.len()&&(chars[j]==' '||chars[j]=='\t'){j+=1;}
            if j<chars.len()&&chars[j]=='\n'{
                let mut left=current;
                while left<i&&chars[left].is_whitespace(){left+=1;}
                let mut right=i;
                while right>left&&chars[right-1].is_whitespace(){right-=1;}
                if left<right{out.push((left,right));}
                let mut k=j+1;
                while k<chars.len()&&chars[k]=='\n'{k+=1;}
                current=k;i=k;continue;
            }
        }
        i+=1;
    }
    let mut left=current;
    while left<chars.len()&&chars[left].is_whitespace(){left+=1;}
    let mut right=chars.len();
    while right>left&&chars[right-1].is_whitespace(){right-=1;}
    if left<right{out.push((left,right));}
    if out.is_empty(){out.push((0,chars.len()));}
    out
}

fn source_file(root:&Path,sha:&str,bytes:&[u8])->Result<PathBuf,NumericParserStoreError>{
    let dir=root.join("original-numeric-sources").join(&sha[..2]);
    fs::create_dir_all(&dir)?;
    let dest=dir.join(format!("{sha}.utf8"));
    if dest.exists(){
        if fs::read(&dest)?!=bytes{return Err(NumericParserStoreError::ExistingConflict);}
        return Ok(dest);
    }
    let temp=dir.join(format!(".{sha}.{}.tmp",std::process::id()));
    let mut fd=OpenOptions::new().write(true).create_new(true).open(&temp)?;
    fd.write_all(bytes)?;fd.sync_all()?;drop(fd);
    match fs::rename(&temp,&dest){
        Ok(())=>{},
        Err(e)=>{
            if dest.exists()&&fs::read(&dest)?==bytes{
                let _=fs::remove_file(&temp);
            }else{return Err(e.into());}
        }
    }
    File::open(&dir)?.sync_all()?;
    Ok(dest)
}
fn symbols(plan:&NativeNumericParserPlan)
    ->Result<BTreeSet<(i16,String)>,NumericParserStoreError>{
    let mut symbols=BTreeSet::new();
    for sentence in &plan.sentences{
        for t in &sentence.tokens{
            for (kind,text) in [
                (1,t.orth.as_str()),(2,t.lemma.as_str()),
                (3,t.pos.as_str()),(4,t.tag.as_str()),
                (5,t.dependency.as_str())
            ]{symbols.insert((kind,normalize(kind,text)?));}
            if let Some(raw)=&t.morph_json{
                let value:Value=serde_json::from_str(raw)
                    .map_err(|_|NumericParserStoreError::MalformedMorphology)?;
                let map=value.as_object().ok_or(NumericParserStoreError::MalformedMorphology)?;
                for (feature,values) in map{
                    symbols.insert((6,normalize(6,feature)?));
                    match values{
                        Value::String(v)=>{symbols.insert((7,normalize(7,v)?));},
                        Value::Array(xs)=>for v in xs{
                            symbols.insert((7,normalize(7,v.as_str()
                                .ok_or(NumericParserStoreError::MalformedMorphology)?)?));
                        },
                        _=>return Err(NumericParserStoreError::MalformedMorphology),
                    }
                }
            }
        }
        for entity in &sentence.entities{
            symbols.insert((8,normalize(8,&entity.label_ref)?));
        }
    }
    Ok(symbols)
}

pub(crate) fn intern_all(
    tx:&mut Transaction<'_>,needed:&BTreeSet<(i16,String)>,
)->Result<BTreeMap<(i16,String),i64>,NumericParserStoreError>{
    let kinds=needed.iter().map(|x|x.0).collect::<Vec<_>>();
    let texts=needed.iter().map(|x|x.1.clone()).collect::<Vec<_>>();
    tx.batch_execute(
        "CREATE TEMP TABLE IF NOT EXISTS tmp_slr_numeric_symbols(
           kind_id smallint NOT NULL,symbol_text text NOT NULL,
           PRIMARY KEY(kind_id,symbol_text)) ON COMMIT DROP;
         TRUNCATE tmp_slr_numeric_symbols")?;
    tx.execute(
        "INSERT INTO tmp_slr_numeric_symbols
         SELECT * FROM unnest($1::smallint[],$2::text[])
         ON CONFLICT DO NOTHING",
        &[&kinds,&texts])?;
    // Setwise lock in global stable order only for new corpus symbols.
    tx.query(
        "SELECT pg_advisory_xact_lock(
           hashtextextended(c.kind_id::text || ':' || c.symbol_text,0))
         FROM tmp_slr_numeric_symbols AS c
         WHERE NOT EXISTS(
           SELECT 1 FROM execution.semantic_symbol AS s
           WHERE s.kind_id=c.kind_id AND s.symbol_text=c.symbol_text)
         ORDER BY c.kind_id,c.symbol_text",&[])?;
    tx.execute(
        "INSERT INTO execution.semantic_symbol(kind_id,symbol_text,symbol_digest)
         SELECT kind_id,symbol_text,
           digest('symbol:v1'::bytea || int2send(kind_id) ||
             int8send(octet_length(convert_to(symbol_text,'UTF8'))::bigint) ||
             convert_to(symbol_text,'UTF8'),'sha256')
         FROM tmp_slr_numeric_symbols
         ORDER BY kind_id,symbol_text
         ON CONFLICT(kind_id,symbol_text) DO NOTHING",&[])?;
    let result=tx.query(
        "SELECT s.kind_id,s.symbol_text,s.symbol_id
         FROM execution.semantic_symbol AS s
         JOIN tmp_slr_numeric_symbols AS t USING(kind_id,symbol_text)",&[])?;
    let mut ids=BTreeMap::new();
    for row in result{
        ids.insert((row.get::<_,i16>(0),row.get::<_,String>(1)),
            row.get::<_,i64>(2));
    }
    if ids.len()!=needed.len(){return Err(NumericParserStoreError::ExistingConflict);}
    Ok(ids)
}
fn sym(
    symbols:&BTreeMap<(i16,String),i64>,kind:i16,text:&str,
)->Result<i64,NumericParserStoreError>{
    let key=(kind,normalize(kind,text)?);
    symbols.get(&key).copied().ok_or(NumericParserStoreError::ExistingConflict)
}

fn morph_pairs(
    raw:&Option<String>,
    symbols:&BTreeMap<(i16,String),i64>,
)->Result<Vec<(i64,i64)>,NumericParserStoreError>{
    let Some(raw)=raw else{return Ok(Vec::new());};
    let parsed:Value=serde_json::from_str(raw)
        .map_err(|_|NumericParserStoreError::MalformedMorphology)?;
    let map=parsed.as_object().ok_or(NumericParserStoreError::MalformedMorphology)?;
    let mut pairs=BTreeSet::new();
    for (feature,value) in map{
        let feature_id=sym(symbols,6,feature)?;
        match value{
            Value::String(x)=>{pairs.insert((feature_id,sym(symbols,7,x)?));},
            Value::Array(xs)=>for x in xs{
                let val=x.as_str().ok_or(NumericParserStoreError::MalformedMorphology)?;
                pairs.insert((feature_id,sym(symbols,7,val)?));
            },
            _=>return Err(NumericParserStoreError::MalformedMorphology),
        }
    }
    Ok(pairs.into_iter().collect())
}
fn morph_digest(pairs:&[(i64,i64)])->Vec<u8>{
    // Original numeric_digest(6, tuple(feature,value,feature,value,...))
    let mut values=Vec::new();
    for &(feature,value) in pairs{values.push(crate::NumericDigestValue::Int(feature as i128));
        values.push(crate::NumericDigestValue::Int(value as i128));}
    crate::strict_numeric_digest(vec![
        crate::NumericDigestValue::Int(6),
        crate::NumericDigestValue::Seq(values),
    ]).to_vec()
}
/// One set-wise morph-set family and one member family, no per-token INSERT.
fn intern_morphology(
    tx:&mut Transaction<'_>,
    plan:&NativeNumericParserPlan,
    symbols:&BTreeMap<(i16,String),i64>,
)->Result<BTreeMap<(String,i32),Option<i64>>,NumericParserStoreError>{
    let mut sets=BTreeMap::<Vec<(i64,i64)>,Vec<u8>>::new();
    let mut by_token=BTreeMap::<(String,i32),Vec<(i64,i64)>>::new();
    for sentence in &plan.sentences{
        for t in &sentence.tokens{
            let pairs=morph_pairs(&t.morph_json,symbols)?;
            if !pairs.is_empty(){
                sets.entry(pairs.clone()).or_insert_with(||morph_digest(&pairs));
            }
            by_token.insert((sentence.sentence_ref.clone(),
                t.original_token_ordinal),pairs);
        }
    }
    if sets.len()>i16::MAX as usize{return Err(NumericParserStoreError::ResourceLimit);}
    if sets.is_empty(){
        return Ok(by_token.into_keys().map(|k|(k,None)).collect());
    }
    let digests=sets.values().cloned().collect::<Vec<_>>();
    let counts=sets.keys().map(|k|k.len() as i16).collect::<Vec<_>>();
    tx.execute(
        "INSERT INTO execution.semantic_morph_set(morph_digest,member_count)
         SELECT * FROM unnest($1::bytea[],$2::smallint[])
         ON CONFLICT(morph_digest) DO NOTHING",&[&digests,&counts])?;
    let mut member_digest=Vec::<Vec<u8>>::new();
    let mut member_ord=Vec::<i16>::new();
    let mut member_feature=Vec::<i64>::new();
    let mut member_value=Vec::<i64>::new();
    for (pairs,digest) in &sets{
        for (ordinal,(feature,value)) in pairs.iter().enumerate(){
            member_digest.push(digest.clone());member_ord.push(ordinal as i16);
            member_feature.push(*feature);member_value.push(*value);
        }
    }
    tx.execute(
        "WITH input AS (
           SELECT * FROM unnest($1::bytea[],$2::smallint[],
                                  $3::bigint[],$4::bigint[])
             AS x(digest,ordinal,feature_id,value_id))
         INSERT INTO execution.semantic_morph_set_member(
           morph_set_id,ordinal,feature_symbol_id,value_symbol_id)
         SELECT m.morph_set_id,x.ordinal,x.feature_id,x.value_id
         FROM input AS x
         JOIN execution.semantic_morph_set AS m ON m.morph_digest=x.digest
         ON CONFLICT DO NOTHING",
        &[&member_digest,&member_ord,&member_feature,&member_value])?;
    let rows=tx.query(
        "SELECT morph_digest,morph_set_id
         FROM execution.semantic_morph_set
         WHERE morph_digest=ANY($1::bytea[])",
        &[&digests])?;
    let mut ids=BTreeMap::new();
    for row in rows{
        ids.insert(row.get::<_,Vec<u8>>(0),row.get::<_,i64>(1));
    }
    if ids.len()!=sets.len(){return Err(NumericParserStoreError::ExistingConflict);}
    let mut result=BTreeMap::new();
    for (key,pairs) in by_token{
        let value=if pairs.is_empty(){None}else{
            let digest=sets.get(&pairs).ok_or(NumericParserStoreError::ExistingConflict)?;
            Some(*ids.get(digest).ok_or(NumericParserStoreError::ExistingConflict)?)
        };
        result.insert(key,value);
    }
    Ok(result)
}

/// Register numeric v2 parser observations without broad Python reparsing.
/// Preconditions require an exact, source-validated plan (never a Q207 label).
/// For a partial SLR parser-run scope, coverage remains OPEN by construction.
// A parser sentence is not a verified publication merely because its
// coordinate-derived sentence_ref exists. Bind the exact original producer
// plan and the complete registered numeric carrier; never trust a count or
// a coordinate digest for lexical/morphological equivalence.
const NATIVE_PARSER_SENTENCE_RECEIPT_SQL:&str="CREATE TABLE IF NOT EXISTS execution.slr_native_parser_sentence_receipt (
    run_ref text NOT NULL,document_ref text NOT NULL,sentence_ref text NOT NULL,
    source_revision_ref text NOT NULL,parser_run_ref text NOT NULL,source_sha256 text NOT NULL,
    contract_ref text NOT NULL,plan_payload jsonb NOT NULL,registered_payload jsonb NOT NULL,
    PRIMARY KEY(run_ref,document_ref,sentence_ref)
);";

const NATIVE_PARSER_SENTENCE_PAYLOAD_SQL:&str="SELECT s.sentence_ref,
    jsonb_build_object('sentence',to_jsonb(s),'partition',to_jsonb(p),
        'tokens',COALESCE((SELECT jsonb_agg(to_jsonb(t) ORDER BY t.local_token_ordinal,t.token_id)
            FROM execution.semantic_parser_token t WHERE t.sentence_ref=s.sentence_ref
              AND t.run_ref=s.run_ref AND t.document_ref=s.document_ref),'[]'::jsonb),
        'entities',COALESCE((SELECT jsonb_agg(to_jsonb(e) ORDER BY e.entity_ref)
            FROM execution.semantic_parser_entity_span e WHERE e.sentence_ref=s.sentence_ref
              AND e.run_ref=s.run_ref AND e.document_ref=s.document_ref),'[]'::jsonb))
    FROM execution.semantic_parser_sentence s JOIN execution.semantic_parser_partition p ON p.partition_ref=s.partition_ref
    WHERE s.run_ref=$1 AND s.document_ref=$2 AND s.sentence_ref=ANY($3::text[])
      AND s.representation_version=2 AND p.run_ref=s.run_ref AND p.document_ref=s.document_ref";

fn parser_sentence_payloads(db:&mut impl postgres::GenericClient,plan:&NativeNumericParserPlan,refs:&[String])
    ->Result<BTreeMap<String,Value>,NumericParserStoreError>{
    let rows=db.query(NATIVE_PARSER_SENTENCE_PAYLOAD_SQL,&[&plan.numeric_run_ref,&plan.source_document_ref,&refs])?;
    let map=rows.iter().map(|r|(r.get::<_,String>(0),r.get::<_,Value>(1))).collect::<BTreeMap<_,_>>();
    if rows.len()!=map.len(){return Err(NumericParserStoreError::ExistingConflict);}
    Ok(map)
}

pub(crate) fn checked_native_parser_sentence_refs(db:&mut impl postgres::GenericClient,plan:&NativeNumericParserPlan)
    ->Result<BTreeSet<String>,NumericParserStoreError>{
    let exists:bool=db.query_one("SELECT to_regclass('execution.slr_native_parser_sentence_receipt') IS NOT NULL",&[])?.get(0);
    if !exists{return Ok(BTreeSet::new());}
    let refs=plan.sentences.iter().map(|s|s.sentence_ref.clone()).collect::<Vec<_>>();
    let receipts=db.query("SELECT sentence_ref,source_revision_ref,parser_run_ref,source_sha256,contract_ref,plan_payload,registered_payload
        FROM execution.slr_native_parser_sentence_receipt
        WHERE run_ref=$1 AND document_ref=$2 AND sentence_ref=ANY($3::text[])",
        &[&plan.numeric_run_ref,&plan.source_document_ref,&refs])?;
    let present=receipts.iter().map(|r|r.get::<_,String>(0)).collect::<Vec<_>>();
    let payloads=parser_sentence_payloads(db,plan,&present)?;
    let expected=plan.sentences.iter().map(|s|(s.sentence_ref.as_str(),s)).collect::<BTreeMap<_,_>>();
    let mut checked=BTreeSet::new();
    for r in receipts{
        let name:String=r.get(0);
        let original=expected.get(name.as_str()).ok_or(NumericParserStoreError::ExistingConflict)?;
        let original_payload=serde_json::to_value(original).map_err(|_|NumericParserStoreError::ForeignSource)?;
        if r.get::<_,String>(1)!=plan.source_revision_ref || r.get::<_,String>(2)!=plan.original_parser_run_ref
           || r.get::<_,String>(3)!=plan.original_source_sha256 || r.get::<_,String>(4)!="slr-native-parser-sentence-receipt:v1"
           || r.get::<_,Value>(5)!=original_payload || payloads.get(&name)!=Some(&r.get::<_,Value>(6))
           || !checked.insert(name) {
            return Err(NumericParserStoreError::ExistingConflict);
        }
    }
    Ok(checked)
}

pub fn persist_original_numeric_parser_plan(
    database_url:&str,
    source_root:&Path,
    canonical_bytes:&[u8],
    plan:&NativeNumericParserPlan,
    source_manifest:&NativeManifest,
)->Result<NumericParserStoreReceipt,NumericParserStoreError>{
    source_manifest.verify_source_bytes(canonical_bytes)
        .map_err(|_|NumericParserStoreError::ForeignSource)?;
    if plan.original_source_sha256!=source_manifest.canonical_content_sha256
        ||plan.original_source_byte_count!=canonical_bytes.len()
        ||plan.original_source_codepoint_count!=source_manifest.source_codepoint_length
        ||plan.original_parser_run_ref!=source_manifest.parser_run_ref
        ||plan.source_revision_ref!=source_manifest.source_revision_ref
        ||plan.source_document_ref!=source_manifest.document_ref
        ||plan.numeric_run_ref.is_empty()
        ||plan.sentences.is_empty()
        ||plan.semantic_identity_paid||plan.level_three_paid{
        return Err(NumericParserStoreError::ForeignSource);
    }
    // Every call is one restartable bounded source slice. The permanent
    // sentence/work trigger remains indexed by exact source+numeric run,
    // not by a program-loop cursor.
    const MAX_SENTENCES:usize=64;
    const MAX_TOKENS:usize=12_000;
    let count=plan.sentences.iter().map(|s|s.tokens.len()).sum::<usize>();
    if plan.sentences.len()>MAX_SENTENCES||count>MAX_TOKENS
        ||plan.expected_total_sentence_regions<plan.sentences.len()
        ||plan.expected_total_token_count<count
        ||plan.expected_total_entity_count<
            plan.sentences.iter().map(|s|s.entities.len()).sum::<usize>()
        ||plan.expected_sentence_refs_sha256.len()!=64{
        return Err(NumericParserStoreError::ResourceLimit);
    }
    let text=std::str::from_utf8(canonical_bytes)
        .map_err(|_|NumericParserStoreError::ForeignSource)?;
    let bounds=byte_boundaries(text);
    let total_numeric_tokens=plan.sentences.iter().map(|s|s.tokens.len()).sum();
    let total_numeric_entities=plan.sentences.iter().map(|s|s.entities.len()).sum();
    let mut by_scope=plan.sentences.iter().collect::<Vec<_>>();
    by_scope.sort_by_key(|s|(s.start_char,s.end_char));
    let mut last_end=0i64;
    let mut original_ordinals=BTreeSet::new();
    for sentence in &by_scope{
        if sentence.source_sentence_index>=plan.expected_total_sentence_regions
            ||!original_ordinals.insert(sentence.source_sentence_index){
            return Err(NumericParserStoreError::InvalidSentence);
        }
        let st=usize::try_from(sentence.start_char)
            .map_err(|_|NumericParserStoreError::InvalidSentence)?;
        let en=usize::try_from(sentence.end_char)
            .map_err(|_|NumericParserStoreError::InvalidSentence)?;
        if sentence.start_char<last_end||st>=en||en>=bounds.len(){
            return Err(NumericParserStoreError::InvalidSentence);
        }
        last_end=sentence.end_char;
        let mut seen=BTreeSet::new();
        for t in &sentence.tokens{
            let a=usize::try_from(t.start_char)
                .map_err(|_|NumericParserStoreError::InvalidSentence)?;
            let b=usize::try_from(t.end_char)
                .map_err(|_|NumericParserStoreError::InvalidSentence)?;
            if a<st||b>en||a>=b||!seen.insert(t.original_token_ordinal)
               ||text.get(bounds[a]..bounds[b])!=Some(t.orth.as_str()){
                return Err(NumericParserStoreError::InvalidSentence);
            }
        }
        for ent in &sentence.entities{
            let a=usize::try_from(ent.start_char)
                .map_err(|_|NumericParserStoreError::InvalidSentence)?;
            let b=usize::try_from(ent.end_char)
                .map_err(|_|NumericParserStoreError::InvalidSentence)?;
            if a<st||b>en||a>=b
                ||text.get(bounds[a]..bounds[b])!=Some(ent.surface.as_str()){
                return Err(NumericParserStoreError::InvalidSentence);
            }
        }
    }
    let normalized=symbols(plan)?;
    let source_path=source_file(source_root,
        &source_manifest.canonical_content_sha256,canonical_bytes)?;
    let mut db=Client::connect(database_url,NoTls)?;
    let mut tx=db.transaction()?;
    tx.batch_execute("SET LOCAL statement_timeout='90000ms'")?;
    tx.batch_execute(NATIVE_PARSER_SENTENCE_RECEIPT_SQL)?;
    tx.query_one("SELECT pg_advisory_xact_lock(hashtext($1),hashtext($2))", &[&plan.numeric_run_ref,&plan.source_document_ref])?;
    tx.query(
        "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
        &[&format!("slr-native-numeric:{}:{}",
            plan.numeric_run_ref,plan.source_document_ref)])?;
    tx.execute(
        "INSERT INTO execution.semantic_run
         (run_ref,document_ref,authority_backend,lifecycle,kernel_key,
          kernel_contract,worker_budget)
         VALUES($1,$2,'postgresql','running',
                'slr.native.numeric-parser-v2',
                'slr-native-source-parser-reuse:v1',1)
         ON CONFLICT(run_ref) DO NOTHING",
        &[&plan.numeric_run_ref,&plan.source_document_ref])?;
    let source_ref=format!(
        "parser-source:sha256:{}",
        sha_ref("slr-native-numeric-source-v1",
           &[&plan.numeric_run_ref,&plan.source_document_ref,
             &plan.original_source_sha256,&plan.original_parser_run_ref])
    );
    tx.execute(
        "INSERT INTO execution.semantic_parser_source
         (source_ref,run_ref,document_ref,content_sha256,byte_count,
          char_count,encoding_ref,locator)
         VALUES($1,$2,$3,$4,$5,$6,'utf8-canonical-source:v1',$7)
         ON CONFLICT(source_ref) DO NOTHING",
        &[&source_ref,&plan.numeric_run_ref,&plan.source_document_ref,
          &Sha256::digest(canonical_bytes).to_vec(),
          &(canonical_bytes.len() as i64),
          &(source_manifest.source_codepoint_length as i64),
          &source_path.to_string_lossy().to_string()])?;
    let run_check=tx.query_one(
        "SELECT document_ref,kernel_contract
         FROM execution.semantic_run WHERE run_ref=$1",
        &[&plan.numeric_run_ref])?;
    if run_check.get::<_,String>(0)!=plan.source_document_ref
        ||run_check.get::<_,String>(1)!="slr-native-source-parser-reuse:v1"{
        return Err(NumericParserStoreError::ExistingConflict);
    }
    let source_check=tx.query(
        "SELECT run_ref,document_ref,content_sha256,byte_count,char_count
         FROM execution.semantic_parser_source WHERE source_ref=$1",
        &[&source_ref])?;
    if source_check.len()!=1
        ||source_check[0].get::<_,String>(0)!=plan.numeric_run_ref
        ||source_check[0].get::<_,String>(1)!=plan.source_document_ref
        ||source_check[0].get::<_,Vec<u8>>(2)!=Sha256::digest(canonical_bytes).to_vec()
        ||source_check[0].get::<_,i64>(3)!=canonical_bytes.len() as i64
        ||source_check[0].get::<_,i64>(4)!=source_manifest.source_codepoint_length as i64{
        return Err(NumericParserStoreError::ExistingConflict);
    }
    let symbols=intern_all(&mut tx,&normalized)?;
    let morph_ids=intern_morphology(&mut tx,plan,&symbols)?;

    // Create exactly one original numeric document owner BEFORE sentence
    // INSERT. The original SQL trigger owns sentence regions, links and
    // leased work items; do not reproduce that trigger in Rust.
    let document_digest=crate::strict_numeric_digest(vec![
        crate::NumericDigestValue::Bytes(plan.numeric_run_ref.as_bytes().to_vec()),
        crate::NumericDigestValue::Bytes(plan.source_document_ref.as_bytes().to_vec()),
        crate::NumericDigestValue::Int(10),
        crate::NumericDigestValue::Int(0),
        crate::NumericDigestValue::Int(source_manifest.source_codepoint_length as i128),
    ]);
    let document_region=tx.query_one(
        "INSERT INTO execution.semantic_pnf_region
         (region_digest,run_ref,document_ref,region_kind,
          start_char,end_char,sequence_no,closure_state,authored_boundary)
         VALUES($1,$2,$3,10,0,$4,0,1,TRUE)
         ON CONFLICT(run_ref,document_ref,region_kind,start_char,end_char)
         DO UPDATE SET authored_boundary=TRUE
         RETURNING region_id",
        &[&document_digest.to_vec(),&plan.numeric_run_ref,
          &plan.source_document_ref,
          &(source_manifest.source_codepoint_length as i64)])?;
    let document_region_id:i64=document_region.get(0);
    if document_region_id<=0{return Err(NumericParserStoreError::ExistingConflict);}

    let paragraphs=authored_numeric_paragraphs(text);
    if paragraphs.is_empty(){return Err(NumericParserStoreError::ForeignSource);}
    // One original paragraph insertion statement: no paragraph-level query loop.
    let p_start=paragraphs.iter().map(|p|p.0 as i64).collect::<Vec<_>>();
    let p_end=paragraphs.iter().map(|p|p.1 as i64).collect::<Vec<_>>();
    let p_ord=(0..paragraphs.len()).map(|x|x as i64).collect::<Vec<_>>();
    let p_digest=paragraphs.iter().map(|&(start,end)|
        crate::strict_numeric_digest(vec![
            crate::NumericDigestValue::Bytes(plan.numeric_run_ref.as_bytes().to_vec()),
            crate::NumericDigestValue::Bytes(plan.source_document_ref.as_bytes().to_vec()),
            crate::NumericDigestValue::Int(3),
            crate::NumericDigestValue::Int(start as i128),
            crate::NumericDigestValue::Int(end as i128),
        ]).to_vec()).collect::<Vec<_>>();
    tx.execute(
        "INSERT INTO execution.semantic_pnf_region
         (region_digest,run_ref,document_ref,region_kind,
          start_char,end_char,sequence_no,parent_region_id,
          closure_state,authored_boundary)
         SELECT p.digest,$1,$2,3,p.start_char,p.end_char,
                p.ordinal,$3,1,TRUE
         FROM unnest($4::bytea[],$5::bigint[],$6::bigint[],
                     $7::bigint[])
           AS p(digest,start_char,end_char,ordinal)
         ON CONFLICT(run_ref,document_ref,region_kind,start_char,end_char)
         DO UPDATE SET parent_region_id=COALESCE(
             execution.semantic_pnf_region.parent_region_id,
             EXCLUDED.parent_region_id),
           authored_boundary=TRUE",
        &[&plan.numeric_run_ref,&plan.source_document_ref,
          &document_region_id,&p_digest,&p_start,&p_end,&p_ord])?;
    tx.execute(
        "INSERT INTO execution.semantic_pnf_region_edge
         (source_region_id,target_region_id,edge_kind,ordinal)
         SELECT region.region_id,$1,1,region.sequence_no
         FROM execution.semantic_pnf_region AS region
         WHERE region.run_ref=$2 AND region.document_ref=$3
           AND region.region_kind=3 AND region.parent_region_id=$1
         ON CONFLICT DO NOTHING",
        &[&document_region_id,&plan.numeric_run_ref,&plan.source_document_ref])?;
    tx.execute(
        "WITH p AS (
           SELECT region_id,sequence_no,
                  LEAD(region_id) OVER (ORDER BY sequence_no) AS following
           FROM execution.semantic_pnf_region
           WHERE run_ref=$1 AND document_ref=$2
             AND region_kind=3 AND parent_region_id=$3)
         INSERT INTO execution.semantic_pnf_region_edge
           (source_region_id,target_region_id,edge_kind,ordinal)
         SELECT p.region_id,p.following,2,p.sequence_no
         FROM p WHERE p.following IS NOT NULL
         ON CONFLICT DO NOTHING",
        &[&plan.numeric_run_ref,&plan.source_document_ref,&document_region_id])?;
    // In the original authoring semantics, a sentence spanning two
    // paragraphs requires a separately resolved boundary repair.
    for sentence in &by_scope {
        let overlaps=paragraphs.iter().any(|&(a,b)|{
            let start=sentence.start_char as usize;
            let end=sentence.end_char as usize;
            start>=a && start<b && end>b
        });
        if overlaps{return Err(NumericParserStoreError::InvalidSentence);}
    }


    for sentence in &by_scope{
        let sequence=sentence.source_sentence_index;
        let st=sentence.start_char as usize;
        let en=sentence.end_char as usize;
        let partition_ref=format!("slr-native-numeric-partition:sha256:{}",
            sha_ref("slr-native-numeric-partition-v1",
                &[&plan.numeric_run_ref,&plan.source_document_ref,
                  &sentence.original_region_ref,&sequence.to_string()]));
        tx.execute(
            "INSERT INTO execution.semantic_parser_partition
              (partition_ref,run_ref,document_ref,source_ref,parser_contract_ref,
               partition_kind,sequence_no,owner_start_char,owner_end_char,
               context_start_char,context_end_char,owner_start_byte,
               owner_end_byte,context_start_byte,context_end_byte,
               state,repair_depth)
             VALUES($1,$2,$3,$4,'slr-native-source-parser-reuse:v1',
               'structural',$5,$6,$7,$6,$7,$8,$9,$8,$9,'completed',0)
             ON CONFLICT(partition_ref) DO NOTHING",
            &[&partition_ref,&plan.numeric_run_ref,&plan.source_document_ref,
              &source_ref,&(sequence as i64),&sentence.start_char,
              &sentence.end_char,&(bounds[st] as i64),&(bounds[en] as i64)])?;
        tx.execute(
            "INSERT INTO execution.semantic_parser_sentence
              (sentence_ref,run_ref,document_ref,partition_ref,
               local_sentence_ordinal,start_char,end_char,
               segmentation_contract_ref,ownership_state,
               sentence_digest,representation_version)
             VALUES($1,$2,$3,$4,0,$5,$6,
                'slr-source-native-sentence:v1','owned',$7,2)
             ON CONFLICT(sentence_ref) DO NOTHING",
            &[&sentence.sentence_ref,&plan.numeric_run_ref,
              &plan.source_document_ref,&partition_ref,
              &sentence.start_char,&sentence.end_char,
              &sentence.sentence_digest.to_vec()])?;
        // One SQL INSERT statement for all sentence tokens, to preserve the
        // original migration-150 setwise dependency-head transition-table
        // trigger and prevent per-token UPDATE explosion.
        let n=sentence.tokens.len();
        // Original m175 producer-complete fast path: allocate numeric IDs
        // before INSERT and include exact head_token_id in the first
        // statement. Avoid migration-150's extra UPDATE of every token.
        // m152's deferred row checks can only be skipped after we perform
        // the independent full-sentence setwise postcondition below.
        let sentence_id:i64=tx.query_one(
            "SELECT sentence_id
             FROM execution.semantic_parser_sentence
             WHERE sentence_ref=$1 AND run_ref=$2 AND document_ref=$3
               AND representation_version=2",
            &[&sentence.sentence_ref,&plan.numeric_run_ref,
              &plan.source_document_ref])?.get(0);
        if sentence_id<=0{return Err(NumericParserStoreError::ExistingConflict);}
        // Resolve the durable natural keys before allocating missing IDs.
        // Sequence consumption and dependency heads must be stable on replay.
        let refs=sentence.tokens.iter().map(|t|t.token_ref.clone()).collect::<Vec<_>>();
        let token_ids=tx.query(
            "SELECT COALESCE(t.token_id,nextval(pg_get_serial_sequence(
                 'execution.semantic_parser_token','token_id')))::bigint
             FROM unnest($1::text[]) WITH ORDINALITY AS wanted(ref,ord)
             LEFT JOIN execution.semantic_parser_token t ON t.token_ref=wanted.ref
             ORDER BY wanted.ord", &[&refs])?
            .into_iter().map(|r|r.get::<_,i64>(0)).collect::<Vec<_>>();
        if token_ids.len()!=n{return Err(NumericParserStoreError::ExistingConflict);}
        let mut planned_ids=BTreeMap::<String,i64>::new();
        for (token,id) in sentence.tokens.iter().zip(&token_ids) {
            if *id<=0||planned_ids.insert(token.token_ref.clone(),*id).is_some(){
                return Err(NumericParserStoreError::ExistingConflict);
            }
        }
        let head_ids=sentence.tokens.iter().map(|t|
            planned_ids.get(&t.head_token_ref).copied()
                .ok_or(NumericParserStoreError::InvalidSentence))
            .collect::<Result<Vec<_>,_>>()?;
        let ords=sentence.tokens.iter().map(|t|t.original_token_ordinal as i64)
            .collect::<Vec<i64>>();
        let starts=sentence.tokens.iter().map(|t|t.start_char).collect::<Vec<_>>();
        let ends=sentence.tokens.iter().map(|t|t.end_char).collect::<Vec<_>>();
        let digests=sentence.tokens.iter().map(|t|t.token_digest.to_vec()).collect::<Vec<_>>();
        let orth=sentence.tokens.iter().map(|t|sym(&symbols,1,&t.orth))
            .collect::<Result<Vec<_>,_>>()?;
        let lemma=sentence.tokens.iter().map(|t|sym(&symbols,2,&t.lemma))
            .collect::<Result<Vec<_>,_>>()?;
        let pos=sentence.tokens.iter().map(|t|sym(&symbols,3,&t.pos))
            .collect::<Result<Vec<_>,_>>()?;
        let tag=sentence.tokens.iter().map(|t|sym(&symbols,4,&t.tag))
            .collect::<Result<Vec<_>,_>>()?;
        let dep=sentence.tokens.iter().map(|t|sym(&symbols,5,&t.dependency))
            .collect::<Result<Vec<_>,_>>()?;
        let head_starts=sentence.tokens.iter().map(|t|t.head_start_char)
            .collect::<Vec<_>>();
        let head_ends=sentence.tokens.iter().map(|t|t.head_end_char)
            .collect::<Vec<_>>();
        let morph=sentence.tokens.iter().map(|t|
            morph_ids.get(&(sentence.sentence_ref.clone(),t.original_token_ordinal))
                .cloned().flatten()).collect::<Vec<_>>();
        let lemma_origin=sentence.tokens.iter().map(|t|t.lemma_origin_id)
            .collect::<Vec<_>>();
        let tag_origin=sentence.tokens.iter().map(|t|t.tag_origin_id)
            .collect::<Vec<_>>();
        if n>0{
            // Both flags are transaction-local execution capabilities, not
            // semantic authority. Original m175/m152 are bypassed ONLY
            // because the Rust source-plan checks unique same-sentence
            // heads and the original SQL full-fibre check below is mandatory.
            tx.batch_execute(
                "SET LOCAL sensiblaw.producer_complete_numeric_heads='on';
                 SET LOCAL sensiblaw.setwise_numeric_head_integrity='on'")?;
            tx.execute(
                "INSERT INTO execution.semantic_parser_token
                 (token_id,token_ref,run_ref,document_ref,partition_ref,
                  sentence_ref,sentence_id,local_token_ordinal,
                  start_char,end_char,head_start_char,head_end_char,
                  head_token_id,token_digest,orth_symbol_id,
                  lemma_symbol_id,pos_symbol_id,tag_symbol_id,
                  dependency_symbol_id,morph_set_id,
                  lemma_origin_id,pos_origin_id,tag_origin_id,
                  dependency_origin_id,representation_version)
                 SELECT t.token_id,t.token_ref,$1,$2,$3,$4,$5,
                   t.ordinal,t.start_char,t.end_char,
                   t.head_start,t.head_end,t.head_token_id,t.digest,
                   t.orth,t.lemma,t.pos,t.tag,t.dep,t.morph_id,
                   t.lemma_origin,1,t.tag_origin,1,2
                 FROM unnest($6::bigint[],$7::text[],$8::bigint[],
                   $9::bigint[],$10::bigint[],$11::bigint[],
                   $12::bigint[],$13::bigint[],$14::bytea[],
                   $15::bigint[],$16::bigint[],$17::bigint[],
                   $18::bigint[],$19::bigint[],$20::bigint[],
                   $21::smallint[],$22::smallint[])
                 AS t(token_id,token_ref,ordinal,start_char,end_char,
                   head_start,head_end,head_token_id,digest,
                   orth,lemma,pos,tag,dep,morph_id,
                   lemma_origin,tag_origin)
                 ON CONFLICT(token_ref) DO NOTHING",
                &[&plan.numeric_run_ref,&plan.source_document_ref,
                  &partition_ref,&sentence.sentence_ref,&sentence_id,
                  &token_ids,&refs,&&ords[..],&starts,&ends,&head_starts,
                  &head_ends,&head_ids,&digests,&orth,&lemma,&pos,&tag,
                  &dep,&morph,&lemma_origin,&tag_origin])?;
        }
        let count:i64=tx.query_one(
            "SELECT count(*)::bigint FROM execution.semantic_parser_token
             WHERE run_ref=$1 AND document_ref=$2 AND sentence_ref=$3
             AND representation_version=2",
            &[&plan.numeric_run_ref,&plan.source_document_ref,
              &sentence.sentence_ref])?.get(0);
        if count!=n as i64{return Err(NumericParserStoreError::ExistingConflict);}
        // Check every original numeric token digest and exact head coordinate.
        let mismatches:i64=tx.query_one(
            "WITH expected AS (
               SELECT * FROM unnest($4::text[],$5::bytea[],
                  $6::bigint[],$7::bigint[],$8::bigint[],$9::bigint[],
                  $10::bigint[],$11::bigint[],$12::bigint[],$13::bigint[],
                  $14::bigint[],$15::bigint[],$16::smallint[],$17::smallint[])
               AS expected(token_ref,token_digest,head_token_id,
                           start_char,end_char,ordinal,orth,lemma,pos,tag,dep,morph,
                           lemma_origin,tag_origin))
             SELECT count(*)::bigint
             FROM expected
             LEFT JOIN execution.semantic_parser_token AS t
               ON t.token_ref=expected.token_ref
             LEFT JOIN execution.semantic_parser_token AS head
               ON head.token_id=t.head_token_id
             WHERE t.token_id IS NULL OR t.run_ref IS DISTINCT FROM $1
               OR t.document_ref IS DISTINCT FROM $2
               OR t.sentence_ref IS DISTINCT FROM $3
               OR t.representation_version IS DISTINCT FROM 2
               OR t.token_digest IS DISTINCT FROM expected.token_digest
               OR t.start_char IS DISTINCT FROM expected.start_char
               OR t.end_char IS DISTINCT FROM expected.end_char
               OR t.local_token_ordinal IS DISTINCT FROM expected.ordinal
               OR t.head_token_id IS DISTINCT FROM expected.head_token_id
               OR t.orth_symbol_id IS DISTINCT FROM expected.orth
               OR t.lemma_symbol_id IS DISTINCT FROM expected.lemma
               OR t.pos_symbol_id IS DISTINCT FROM expected.pos
               OR t.tag_symbol_id IS DISTINCT FROM expected.tag
               OR t.dependency_symbol_id IS DISTINCT FROM expected.dep
               OR t.morph_set_id IS DISTINCT FROM expected.morph
               OR t.lemma_origin_id IS DISTINCT FROM expected.lemma_origin
               OR t.tag_origin_id IS DISTINCT FROM expected.tag_origin
               OR t.pos_origin_id IS DISTINCT FROM 1
               OR t.dependency_origin_id IS DISTINCT FROM 1
               OR head.sentence_id IS DISTINCT FROM t.sentence_id
               OR head.start_char IS DISTINCT FROM t.head_start_char
               OR head.end_char IS DISTINCT FROM t.head_end_char",
            &[&plan.numeric_run_ref,&plan.source_document_ref,
              &sentence.sentence_ref,&refs,&digests,&head_ids,
              &starts,&ends,&ords,&orth,&lemma,&pos,&tag,&dep,&morph,&lemma_origin,&tag_origin])?.get(0);
        if mismatches>0{return Err(NumericParserStoreError::ExistingConflict);}
        // Entity observations are candidates and must remain NER source
        // spans; one setwise statement per bounded sentence, not N SQL calls.
        let erefs=sentence.entities.iter().map(|e|e.entity_ref.clone())
            .collect::<Vec<_>>();
        let estarts=sentence.entities.iter().map(|e|e.start_char)
            .collect::<Vec<_>>();
        let eends=sentence.entities.iter().map(|e|e.end_char)
            .collect::<Vec<_>>();
        let edigests=sentence.entities.iter().map(|e|e.entity_digest.as_slice())
            .collect::<Vec<&[u8]>>();
        let ekinds=sentence.entities.iter().map(|e|sym(&symbols,8,&e.label_ref))
            .collect::<Result<Vec<_>,_>>()?;
        if !erefs.is_empty(){
            tx.execute(
                "INSERT INTO execution.semantic_parser_entity_span
                 (entity_ref,run_ref,document_ref,partition_ref,sentence_ref,
                  start_char,end_char,entity_digest,entity_type_symbol_id,
                  representation_version)
                 SELECT e.entity_ref,$1,$2,$3,$4,e.start_char,e.end_char,
                        e.entity_digest,e.entity_type,2
                 FROM unnest($5::text[],$6::bigint[],$7::bigint[],
                             $8::bytea[],$9::bigint[])
                   AS e(entity_ref,start_char,end_char,entity_digest,entity_type)
                 ON CONFLICT(entity_ref) DO NOTHING",
                &[&plan.numeric_run_ref,&plan.source_document_ref,
                  &partition_ref,&sentence.sentence_ref,
                  &erefs,&estarts,&eends,&edigests,&ekinds])?;
        }
        // Validate exact entity payload and multiplicity even for existing
        // registrations. Equal source coordinates alone do not pay NER parity.
        let entity_mismatches:i64=tx.query_one("WITH expected AS (
            SELECT * FROM unnest($4::text[],$5::bigint[],$6::bigint[],$7::bytea[],$8::bigint[])
              e(entity_ref,start_char,end_char,entity_digest,entity_type_symbol_id)
          ), actual AS (
            SELECT entity_ref,start_char,end_char,entity_digest,entity_type_symbol_id,partition_ref,representation_version
            FROM execution.semantic_parser_entity_span WHERE run_ref=$1 AND document_ref=$2 AND sentence_ref=$3
          )
          SELECT count(*)::bigint FROM expected e FULL JOIN actual a USING(entity_ref)
          WHERE e.entity_ref IS NULL OR a.entity_ref IS NULL OR a.partition_ref IS DISTINCT FROM $9
            OR a.representation_version IS DISTINCT FROM 2 OR
            (a.start_char,a.end_char,a.entity_digest,a.entity_type_symbol_id) IS DISTINCT FROM
            (e.start_char,e.end_char,e.entity_digest,e.entity_type_symbol_id)",
            &[&plan.numeric_run_ref,&plan.source_document_ref,&sentence.sentence_ref,&erefs,&estarts,&eends,&edigests,&ekinds,&partition_ref])?.get(0);
        if entity_mismatches!=0{return Err(NumericParserStoreError::ExistingConflict);}
    }
    // Persist a per-sentence immutable receipt only after all original
    // parser row and head checks succeed in this transaction.
    let receipt_refs=plan.sentences.iter().map(|s|s.sentence_ref.clone()).collect::<Vec<_>>();
    let registered_payloads=parser_sentence_payloads(&mut tx,plan,&receipt_refs)?;
    for sentence in &plan.sentences{
        let original_payload=serde_json::to_value(sentence).map_err(|_|NumericParserStoreError::ForeignSource)?;
        let registered=registered_payloads.get(&sentence.sentence_ref).ok_or(NumericParserStoreError::ExistingConflict)?;
        tx.execute("INSERT INTO execution.slr_native_parser_sentence_receipt
          (run_ref,document_ref,sentence_ref,source_revision_ref,parser_run_ref,source_sha256,contract_ref,plan_payload,registered_payload)
          VALUES($1,$2,$3,$4,$5,$6,'slr-native-parser-sentence-receipt:v1',$7,$8)
          ON CONFLICT DO NOTHING",&[&plan.numeric_run_ref,&plan.source_document_ref,&sentence.sentence_ref,
            &plan.source_revision_ref,&plan.original_parser_run_ref,&plan.original_source_sha256,&original_payload,registered])?;
    }
    if checked_native_parser_sentence_refs(&mut tx,plan)?.len()!=receipt_refs.len(){
        return Err(NumericParserStoreError::ExistingConflict);
    }
    // One set-wise original numeric parser receipt family. Metadata comes
    // from the actual SLR parser run, not a newly claimed spaCy execution.
    tx.execute(
        "WITH partitions AS MATERIALIZED (
           SELECT partition_ref,run_ref,document_ref,parser_contract_ref
           FROM execution.semantic_parser_partition
           WHERE run_ref=$1 AND document_ref=$2
         ), sentences AS (
           SELECT sentence.partition_ref,count(*)::bigint AS sentence_count
           FROM execution.semantic_parser_sentence AS sentence
           JOIN partitions AS p USING(partition_ref)
           WHERE sentence.representation_version=2
           GROUP BY sentence.partition_ref
         ), tokens AS (
           SELECT token.partition_ref,count(*)::bigint AS token_count
           FROM execution.semantic_parser_token AS token
           JOIN partitions AS p USING(partition_ref)
           WHERE token.representation_version=2
           GROUP BY token.partition_ref
         ), entities AS (
           SELECT entity.partition_ref,count(*)::bigint AS entity_count
           FROM execution.semantic_parser_entity_span AS entity
           JOIN partitions AS p USING(partition_ref)
           WHERE entity.representation_version=2
           GROUP BY entity.partition_ref
         ), per_partition AS (
           SELECT p.*,COALESCE(sentences.sentence_count,0) AS sentence_count,
                  COALESCE(tokens.token_count,0) AS token_count,
                  COALESCE(entities.entity_count,0) AS entity_count
           FROM partitions AS p
           LEFT JOIN sentences USING(partition_ref)
           LEFT JOIN tokens USING(partition_ref)
           LEFT JOIN entities USING(partition_ref)
         )
         INSERT INTO execution.semantic_parser_partition_receipt
           (receipt_ref,partition_ref,run_ref,document_ref,
            parser_contract_ref,model_name,model_version,
            tokenization,sentence_segmentation,part_of_speech,
            morphology,dependencies,named_entities,
            sentence_count,token_count,entity_count,
            boundary_obligation_count,elapsed_ns,worker_pid,
            backend_pid,receipt_sha256)
         SELECT
           'slr-native-numeric-receipt:sha256:' ||
             encode(digest(convert_to(p.partition_ref || ':' || $3,'UTF8'),
                           'sha256'),'hex'),
           p.partition_ref,p.run_ref,p.document_ref,p.parser_contract_ref,
           $4,$5,TRUE,TRUE,TRUE,FALSE,TRUE,TRUE,
           p.sentence_count,p.token_count,p.entity_count,
           0,0,pg_backend_pid()::bigint,pg_backend_pid(),
           digest(convert_to(p.partition_ref || ':' || $3,'UTF8'),'sha256')
         FROM per_partition AS p
         ON CONFLICT(partition_ref) DO NOTHING",
        &[&plan.numeric_run_ref,&plan.source_document_ref,
          &plan.original_parser_run_ref,&plan.source_model_ref,
          &plan.source_parser_version])?;
    let missing_receipts:i64=tx.query_one(
        "SELECT count(*)::bigint
         FROM execution.semantic_parser_partition AS partition
         WHERE partition.run_ref=$1 AND partition.document_ref=$2
           AND NOT EXISTS (
             SELECT 1 FROM execution.semantic_parser_partition_receipt AS receipt
             WHERE receipt.partition_ref=partition.partition_ref)",
        &[&plan.numeric_run_ref,&plan.source_document_ref])?.get(0);
    if missing_receipts>0{return Err(NumericParserStoreError::ExistingConflict);}

    let current=tx.query(
        "SELECT sentence_ref
         FROM execution.semantic_parser_sentence
         WHERE run_ref=$1 AND document_ref=$2 AND representation_version=2
         ORDER BY sentence_ref",
        &[&plan.numeric_run_ref,&plan.source_document_ref])?;
    let refs=current.iter().map(|r|r.get::<_,String>(0)).collect::<Vec<_>>();
    let complete=plan.full_document_coverage_paid
        &&refs.len()==plan.expected_total_sentence_regions
        &&digest_numeric_sentence_refs(&refs)==plan.expected_sentence_refs_sha256;
    // A matching cardinality with a nonmatching source registration is
    // evidence of a foreign sentence: fail instead of claiming coverage.
    if refs.len()>plan.expected_total_sentence_regions
        ||(refs.len()==plan.expected_total_sentence_regions
            &&digest_numeric_sentence_refs(&refs)!=plan.expected_sentence_refs_sha256) {
        return Err(NumericParserStoreError::ExistingConflict);
    }
    let registered=refs.len() as i64;
    let expected=plan.expected_total_sentence_regions as i64;
    tx.execute(
        "INSERT INTO execution.semantic_parser_document_coverage
         (run_ref,document_ref,total_partitions,completed_partitions,
          open_boundary_obligations,tokenization,sentence_segmentation,
          part_of_speech,morphology,dependencies,named_entities,state)
         VALUES($1,$2,$3,$4,0,TRUE,TRUE,TRUE,FALSE,TRUE,TRUE,$5)
         ON CONFLICT(run_ref,document_ref) DO UPDATE SET
           total_partitions=EXCLUDED.total_partitions,
           completed_partitions=EXCLUDED.completed_partitions,
           state=EXCLUDED.state",
        &[&plan.numeric_run_ref,&plan.source_document_ref,&expected,
          &registered,&if complete{"complete"}else{"open"}])?;
    tx.commit()?;
    Ok(NumericParserStoreReceipt{
        source_sha256:plan.original_source_sha256.clone(),
        numeric_run_ref:plan.numeric_run_ref.clone(),
        numeric_document_ref:plan.source_document_ref.clone(),
        original_parser_run_ref:plan.original_parser_run_ref.clone(),
        original_source_revision_ref:plan.source_revision_ref.clone(),
        numeric_sentences:plan.sentences.len(),
        numeric_tokens:total_numeric_tokens,numeric_entities:total_numeric_entities,
        coverage_complete:complete,
        identity_paid:false,level_three_paid:false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_authored_paragraph_boundaries_are_not_flattened() {
        assert_eq!(
            authored_numeric_paragraphs(" a \n \n b\n\nc "),
            vec![(1,2),(7,8),(10,11)]
        );
        assert_eq!(authored_numeric_paragraphs("hello"),vec![(0,5)]);
        assert_eq!(authored_numeric_paragraphs(" \n\n "),vec![(0,4)]);
        assert!(authored_numeric_paragraphs("").is_empty());
    }

    #[test]
    fn lexical_normalization_is_explicit_and_never_fuzzy() {
        assert_eq!(normalize(2,"MUST").unwrap(),"must");
        assert_eq!(normalize(3,"verb").unwrap(),"VERB");
        assert_eq!(normalize(8,"person").unwrap(),"PERSON");
        assert_eq!(normalize(1,"Naïve").unwrap(),"Naïve");
        assert_eq!(normalize(2,"Straße").unwrap(),"strasse");
        assert_eq!(normalize(2,"ﬁ").unwrap(),"fi");
    }

    #[test]
    fn morphology_digest_matches_independent_strict_binary_reference() {
        let pairs=vec![(101,202),(103,204)];
        let digest=morph_digest(&pairs);
        assert_eq!(digest.iter().map(|b|format!("{b:02x}")).collect::<String>(),
            "160cb06ab2477f94272b61f0b3ae14419c74dc33d675a7dd19f727b04cbfb74b");
    }

    #[test]
    fn numeric_source_ref_is_run_bound_not_label_bound() {
        let x=sha_ref("slr-native-numeric-source-v1",
            &["numeric-run:1","document:sha256:book","digest","parser:1"]);
        let y=sha_ref("slr-native-numeric-source-v1",
            &["numeric-run:2","document:sha256:book","digest","parser:1"]);
        assert_ne!(x,y);
    }
}
