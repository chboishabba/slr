//! Exact SLR -> ORIGINAL SensibLaw numeric parser v2 PostgreSQL bootstrap.
//!
//! One bounded, idempotent transaction for source, partition, sentences,
//! numeric tokens, dependency heads, and entity spans. Original schema
//! authority is retained; no PNF identity witness or Level-3 row is inserted.
//! This producer does not re-run spaCy. Only original source-backed observations
//! and explicit POS->TAG fallback are admitted. Full Unicode casefold is not
//! guessed: unhandled normalized letters fail closed.

use crate::{NativeNumericParserPlan,NativeManifest,
    NativeNumericParserSentencePlan};
use postgres::{Client,NoTls,Transaction};
use serde_json::Value;
use sha2::{Digest,Sha256};
use std::collections::{BTreeMap,BTreeSet};
use std::fs::{self,File,OpenOptions};
use std::io::Write;
use std::path::{Path,PathBuf};
use thiserror::Error;
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
    #[error("strict Python casefold not representable; refusing approximate symbol")]
    UnsupportedUnicodeCasefold,
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
        2|5|11|12|13|14|15|16|17|18|19=>{
            if nfc.chars().any(|c|!c.is_ascii()&&c.is_alphabetic()){
                return Err(NumericParserStoreError::UnsupportedUnicodeCasefold);
            }
            nfc.to_ascii_lowercase()
        }
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

fn intern_all(
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

/// Register numeric v2 parser observations without broad Python reparsing.
/// Preconditions require an exact, source-validated plan (never a Q207 label).
/// For a partial SLR parser-run scope, coverage remains OPEN by construction.
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
    const MAX_SENTENCES:usize=4096;
    const MAX_TOKENS:usize=100_000;
    let count=plan.sentences.iter().map(|s|s.tokens.len()).sum::<usize>();
    if plan.sentences.len()>MAX_SENTENCES||count>MAX_TOKENS{
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
    for sentence in &by_scope{
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
    let source_ref=format!("parser-source:{}",plan.original_source_sha256);
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

    for (sequence,sentence) in by_scope.iter().enumerate(){
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
        let refs=sentence.tokens.iter().map(|t|t.token_ref.clone()).collect::<Vec<_>>();
        let ords=sentence.tokens.iter().map(|t|t.original_token_ordinal).collect::<Vec<_>>();
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
        let lemma_origin=sentence.tokens.iter().map(|t|t.lemma_origin_id)
            .collect::<Vec<_>>();
        let tag_origin=sentence.tokens.iter().map(|t|t.tag_origin_id)
            .collect::<Vec<_>>();
        if n>0{
            tx.execute(
                "INSERT INTO execution.semantic_parser_token
                 (token_ref,run_ref,document_ref,partition_ref,sentence_ref,
                  local_token_ordinal,start_char,end_char,
                  head_start_char,head_end_char,token_digest,
                  orth_symbol_id,lemma_symbol_id,pos_symbol_id,
                  tag_symbol_id,dependency_symbol_id,
                  lemma_origin_id,pos_origin_id,tag_origin_id,
                  dependency_origin_id,representation_version)
                 SELECT t.token_ref,$1,$2,$3,$4,t.ordinal,t.start_char,
                   t.end_char,t.head_start,t.head_end,t.digest,
                   t.orth,t.lemma,t.pos,t.tag,t.dep,
                   t.lemma_origin,1,t.tag_origin,1,2
                 FROM unnest($5::text[],$6::integer[],$7::bigint[],
                   $8::bigint[],$9::bigint[],$10::bigint[],
                   $11::bytea[],$12::bigint[],$13::bigint[],
                   $14::bigint[],$15::bigint[],$16::bigint[],
                   $17::smallint[],$18::smallint[])
                 AS t(token_ref,ordinal,start_char,end_char,
                   head_start,head_end,digest,orth,lemma,pos,tag,dep,
                   lemma_origin,tag_origin)
                 ON CONFLICT(token_ref) DO NOTHING",
                &[&plan.numeric_run_ref,&plan.source_document_ref,
                  &partition_ref,&sentence.sentence_ref,&refs,&ords,&starts,
                  &ends,&head_starts,&head_ends,&digests,&orth,&lemma,
                  &pos,&tag,&dep,&lemma_origin,&tag_origin])?;
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
            "SELECT count(*)::bigint
             FROM execution.semantic_parser_token AS t
             WHERE t.run_ref=$1 AND t.document_ref=$2
               AND t.sentence_ref=$3 AND t.representation_version=2
               AND (t.head_token_id IS NULL OR
                    t.head_start_char IS DISTINCT FROM
                      (SELECT head.start_char FROM execution.semantic_parser_token AS head
                       WHERE head.token_id=t.head_token_id) OR
                    t.head_end_char IS DISTINCT FROM
                      (SELECT head.end_char FROM execution.semantic_parser_token AS head
                       WHERE head.token_id=t.head_token_id))",
            &[&plan.numeric_run_ref,&plan.source_document_ref,
              &sentence.sentence_ref])?.get(0);
        if mismatches>0{return Err(NumericParserStoreError::ExistingConflict);}
        // Original entity span uses the same v2 numeric digest and source label,
        // but no semantic actor authority is created.
        for entity in &sentence.entities{
            let kind=sym(&symbols,8,&entity.label_ref)?;
            tx.execute(
                "INSERT INTO execution.semantic_parser_entity_span
                 (entity_ref,run_ref,document_ref,partition_ref,sentence_ref,
                  start_char,end_char,entity_digest,entity_type_symbol_id,
                  representation_version)
                 VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,2)
                 ON CONFLICT(entity_ref) DO NOTHING",
                &[&entity.entity_ref,&plan.numeric_run_ref,
                  &plan.source_document_ref,&partition_ref,
                  &sentence.sentence_ref,&entity.start_char,
                  &entity.end_char,&entity.entity_digest.to_vec(),&kind])?;
        }
    }
    let total=plan.sentences.len() as i64;
    tx.execute(
        "INSERT INTO execution.semantic_parser_document_coverage
         (run_ref,document_ref,total_partitions,completed_partitions,
          open_boundary_obligations,tokenization,sentence_segmentation,
          part_of_speech,morphology,dependencies,named_entities,state)
         VALUES($1,$2,$3,$3,0,TRUE,TRUE,TRUE,FALSE,TRUE,TRUE,$4)
         ON CONFLICT(run_ref,document_ref) DO UPDATE SET
           total_partitions=EXCLUDED.total_partitions,
           completed_partitions=EXCLUDED.completed_partitions,
           state=EXCLUDED.state",
        &[&plan.numeric_run_ref,&plan.source_document_ref,&total,
          &if plan.full_document_coverage_paid{"complete"}else{"open"}])?;
    tx.commit()?;
    Ok(NumericParserStoreReceipt{
        source_sha256:plan.original_source_sha256.clone(),
        numeric_run_ref:plan.numeric_run_ref.clone(),
        numeric_document_ref:plan.source_document_ref.clone(),
        original_parser_run_ref:plan.original_parser_run_ref.clone(),
        original_source_revision_ref:plan.source_revision_ref.clone(),
        numeric_sentences:plan.sentences.len(),
        numeric_tokens:total_numeric_tokens,numeric_entities:total_numeric_entities,
        coverage_complete:plan.full_document_coverage_paid,
        identity_paid:false,level_three_paid:false,
    })
}
