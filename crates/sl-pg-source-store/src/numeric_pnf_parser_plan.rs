//! Deterministic exact-source SLR -> original numeric parser row plan.
//! The original numeric parser v2 sentence/token identity is binary:
 //! sentence_digest = numeric_digest(run_ref UTF8, document_ref UTF8,
 //!                                  sentence_start, sentence_end, 2)
 //! token_digest = numeric_digest(sentence_digest, start, end, ordinal)
 //! This is a candidate producer, not a numeric identity witness.
 //!
 //! Missing original parser capabilities are explicit. An absent tag uses
 //! the ORIGINAL numeric parser POS fallback with origin_id=3. A missing
 //! dependency head is not silently assigned to ROOT.

use crate::{OriginalNativeNumericEvidence,NumericDigestValue,
    strict_numeric_digest};
use serde::{Deserialize,Serialize};
use std::collections::BTreeMap;
use sha2::{Digest,Sha256};
use thiserror::Error;

#[derive(Debug,Error)]
pub enum NumericParserPlanError {
    #[error("empty original numeric run or source revision")]
    EmptyScope,
    #[error("the original source has no scoped native parser sentences")]
    MissingSentences,
    #[error("original token has no valid explicit numeric dependency head")]
    MissingHead,
    #[error("original sentence token ordinals are not uniquely addressable")]
    DuplicateOrdinal,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct NativeNumericParserTokenPlan {
    pub original_compilation_key:String,
    pub original_token_ordinal:i32,
    pub token_ref:String,
    pub token_digest:[u8;32],
    pub start_char:i64,
    pub end_char:i64,
    pub orth:String,
    pub lemma:String,
    pub pos:String,
    pub tag:String,
    pub dependency:String,
    pub head_token_ref:String,
    pub head_start_char:i64,
    pub head_end_char:i64,
    pub lemma_origin_id:i16,
    pub pos_origin_id:i16,
    pub tag_origin_id:i16,
    pub dependency_origin_id:i16,
    pub morph_json:Option<String>,
    pub creates_semantic_authority:bool,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct NativeNumericParserEntityPlan {
    pub original_compilation_key:String,
    pub original_entity_ordinal:i32,
    pub entity_ref:String,
    pub entity_digest:[u8;32],
    pub start_char:i64,
    pub end_char:i64,
    pub surface:String,
    pub label_ref:String,
    pub source_candidate_only:bool,
    pub creates_semantic_authority:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct NativeNumericParserSentencePlan {
    pub source_sentence_index:usize,
    pub original_compilation_key:String,
    pub original_region_ref:String,
    pub sentence_ref:String,
    pub sentence_digest:[u8;32],
    pub start_char:i64,
    pub end_char:i64,
    pub tokens:Vec<NativeNumericParserTokenPlan>,
    pub entities:Vec<NativeNumericParserEntityPlan>,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct NativeNumericParserPlan {
    pub schema:String,
    pub numeric_run_ref:String,
    pub source_revision_ref:String,
    pub original_source_sha256:String,
    pub original_source_byte_count:usize,
    pub original_source_codepoint_count:usize,
    pub original_parser_run_ref:String,
    pub source_parser_family:String,
    pub source_parser_version:String,
    pub source_model_ref:String,
    pub source_document_ref:String,
    pub sentences:Vec<NativeNumericParserSentencePlan>,
    pub expected_total_sentence_regions:usize,
    pub expected_total_token_count:usize,
    pub expected_total_entity_count:usize,
    pub expected_sentence_refs_sha256:String,
    pub full_document_coverage_paid:bool,
    pub semantic_identity_paid:bool,
    pub level_three_paid:bool,
    pub residuals:Vec<String>,
}

pub(crate) fn digest_numeric_sentence_refs(refs:&[String])->String{
    let mut ordered=refs.to_vec();
    ordered.sort();
    let mut hash=Sha256::new();
    for item in &ordered{
        hash.update((item.len() as u64).to_be_bytes());
        hash.update(item.as_bytes());
    }
    format!("{:x}",hash.finalize())
}
fn int(n:i64)->NumericDigestValue{NumericDigestValue::Int(n as i128)}
fn bytes(s:&str)->NumericDigestValue{NumericDigestValue::Bytes(s.as_bytes().to_vec())}
fn prefixed_ref(prefix:&str,digest:&[u8;32])->String {
    let mut result=prefix.to_string();
    for b in digest {result.push_str(&format!("{b:02x}"));}
    result
}

pub fn plan_original_native_numeric_parser(
    source:&OriginalNativeNumericEvidence,
    numeric_run_ref:&str,
) -> Result<NativeNumericParserPlan,NumericParserPlanError>{
    if numeric_run_ref.is_empty()||source.manifest.source_revision_ref.is_empty(){
        return Err(NumericParserPlanError::EmptyScope);
    }
    if source.parser_regions.is_empty(){
        return Err(NumericParserPlanError::MissingSentences);
    }
    let mut sentences=Vec::with_capacity(source.parser_regions.len());
    for (source_sentence_index,region) in source.parser_regions.iter().enumerate() {
        let sentence_digest=strict_numeric_digest(vec![
            bytes(numeric_run_ref),bytes(&source.manifest.document_ref),
            int(region.start_char),int(region.end_char),int(2),
        ]);
        let sentence_ref=prefixed_ref("parser-sentence:",&sentence_digest);
        let mut temp=BTreeMap::new();
        for token in &region.tokens {
            let digest=strict_numeric_digest(vec![
                NumericDigestValue::Bytes(sentence_digest.to_vec()),
                int(token.start_char),int(token.end_char),
                int(i64::from(token.original_token_ordinal)),
            ]);
            let token_ref=prefixed_ref("parser-token:",&digest);
            if temp.insert(token.original_token_ordinal,
                (token_ref,digest,token)).is_some(){
                return Err(NumericParserPlanError::DuplicateOrdinal);
            }
        }
        let mut token_rows=Vec::new();
        for (_, (token_ref,digest,t)) in &temp {
            let target=match t.head_ordinal {
                Some(id)=>temp.get(&id).map(|(r,_,raw)|
                    (r.clone(),raw.start_char,raw.end_char)),
                None if t.dependency_ref.eq_ignore_ascii_case("ROOT") =>
                    Some((token_ref.clone(),t.start_char,t.end_char)),
                None=>None,
            }.ok_or(NumericParserPlanError::MissingHead)?;
            let has_lemma=!t.lemma.trim().is_empty();
            token_rows.push(NativeNumericParserTokenPlan{
                original_compilation_key:t.compilation_key.clone(),
                original_token_ordinal:t.original_token_ordinal,
                token_ref:token_ref.clone(),token_digest:*digest,
                start_char:t.start_char,end_char:t.end_char,
                orth:t.surface.clone(),
                lemma:if has_lemma {t.lemma.clone()} else {t.surface.clone()},
                pos:t.pos.clone(),
                // Exact original numeric fallback convention when a tag
                // is absent: tag=POS with tag_origin_id=3.
                tag:t.pos.clone(),dependency:t.dependency_ref.clone(),
                head_token_ref:target.0,
                head_start_char:target.1,head_end_char:target.2,
                lemma_origin_id:if has_lemma{1}else{2},
                pos_origin_id:1,tag_origin_id:3,dependency_origin_id:1,
                morph_json:t.morph_json.clone(),
                creates_semantic_authority:false,
            });
        }
        let mut entities=Vec::with_capacity(region.entities.len());
        for entity in &region.entities{
            let digest=strict_numeric_digest(vec![
                bytes(numeric_run_ref),bytes(&source.manifest.document_ref),
                int(entity.start_char),int(entity.end_char),int(8),
            ]);
            entities.push(NativeNumericParserEntityPlan{
                original_compilation_key:region.compilation_key.clone(),
                original_entity_ordinal:entity.original_entity_ordinal,
                entity_ref:prefixed_ref("parser-entity:",&digest),
                entity_digest:digest,start_char:entity.start_char,
                end_char:entity.end_char,surface:entity.surface.clone(),
                label_ref:entity.label_ref.clone(),
                source_candidate_only:true,creates_semantic_authority:false,
            });
        }
        entities.sort_by_key(|e|(e.start_char,e.end_char,e.original_entity_ordinal));
        sentences.push(NativeNumericParserSentencePlan{
            source_sentence_index,
            original_compilation_key:region.compilation_key.clone(),
            original_region_ref:region.region_ref.clone(),
            sentence_ref,sentence_digest,start_char:region.start_char,
            end_char:region.end_char,tokens:token_rows,entities,
        });
    }
    let mut residuals=source.residuals.clone();
    residuals.push(
        "numeric_parser_tag_is_explicit_POS_fallback_not_original_TAG".into());
    residuals.push(
        "numeric_anaphor_identity_remains_open_until_original_SQL_demand_and_witness".into());
    Ok(NativeNumericParserPlan{
        schema:"slr-exact-native-numeric-parser-plan-v1".into(),
        numeric_run_ref:numeric_run_ref.into(),
        source_revision_ref:source.manifest.source_revision_ref.clone(),
        original_source_sha256:source.manifest.canonical_content_sha256.clone(),
        original_source_byte_count:source.manifest.source_byte_length,
        original_source_codepoint_count:source.manifest.source_codepoint_length,
        original_parser_run_ref:source.manifest.parser_run_ref.clone(),
        source_parser_family:source.source_parser_family.clone(),
        source_parser_version:source.source_parser_version.clone(),
        source_model_ref:source.source_model_ref.clone(),
        source_document_ref:source.manifest.document_ref.clone(),
        expected_total_sentence_regions:sentences.len(),
        expected_total_token_count:sentences.iter().map(|s|s.tokens.len()).sum(),
        expected_total_entity_count:sentences.iter().map(|s|s.entities.len()).sum(),
        expected_sentence_refs_sha256:digest_numeric_sentence_refs(
            &sentences.iter().map(|s|s.sentence_ref.clone()).collect::<Vec<_>>()
        ),
        sentences,full_document_coverage_paid:source.full_document_coverage_paid,
        semantic_identity_paid:false,level_three_paid:false,residuals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NativeManifest,OriginalNumericSentenceFibre,
        OriginalNumericRawToken};
    #[test]
    fn explicit_head_self_is_not_missing(){
        let source=OriginalNativeNumericEvidence{
            schema:"test",manifest:NativeManifest{
                document_ref:"document:sha256:original".into(),
                source_revision_ref:"revision:original".into(),
                canonical_content_sha256:"a".repeat(64),
                source_byte_length:2,source_codepoint_length:2,
                coordinate_contract_ref:"unicode-codepoints:v1".into(),
                parser_run_ref:"parser:original".into(),rule_set_revision:"v1".into(),
            },original_source_ref:"original".into(),
            source_parser_family:"source-spacy".into(),
            source_parser_version:"v1".into(),
            source_model_ref:"en_core_web_sm".into(),
            parser_regions:vec![OriginalNumericSentenceFibre{
                compilation_key:"job".into(),region_ref:"region".into(),
                start_char:0,end_char:2,
                tokens:vec![OriginalNumericRawToken{
                    compilation_key:"job".into(),original_token_ordinal:0,
                    start_char:0,end_char:2,surface:"he".into(),
                    lemma:"he".into(),pos:"PRON".into(),morph_json:None,
                    head_ordinal:Some(0),dependency_ref:"ROOT".into(),
                    original_tag_unavailable:true,semantic_admission_paid:false,
                }],entities:vec![],full_numeric_tag_capability_available:false,
            }],original_region_count:1,original_token_count:1,
            full_document_coverage_paid:true,residuals:vec![],
            typed_identity_paid:false,external_qid_paid:false,
        };
        let plan=plan_original_native_numeric_parser(&source,"num:run").unwrap();
        assert_eq!(plan.sentences[0].tokens[0].head_token_ref,
            plan.sentences[0].tokens[0].token_ref);
        assert_eq!(plan.sentences[0].tokens[0].tag_origin_id,3);
        assert!(!plan.semantic_identity_paid);
    }
}
