//! Read-only exact-source P0-C/D numeric identity audit.
//! It does NOT derive identity by Bush spelling or a human S29 button.
//! It requires the source-local SLR receipt, the same numeric source digest,
//! exact parser/NER spans, migration-180 pronoun demand, and original typed
//! demand/open-world/witness/Level-3 SQL authority independently.
//! No writes, no source refresh, no hidden full-document candidate scan.

use crate::NativeSourceLocalAnswer;
use postgres::{Client,NoTls};
use serde::Serialize;
use thiserror::Error;
use std::collections::BTreeSet;

#[derive(Debug,Error)]
pub enum NativeNumericAuditError{
    #[error("numeric PostgreSQL: {0}")]
    Database(#[from] postgres::Error),
    #[error("original source-relative linguistic receipt invalid for this audit")]
    SourcePolicy,
    #[error("numeric run is tied to a different canonical source manifestation")]
    ForeignSource,
}

#[derive(Debug,Clone,Serialize)]
pub struct NativeNumericIdentityAudit{
    pub schema:&'static str,
    pub original_source_relative_answer:String,
    pub original_source_sha256:String,
    pub original_source_revision_ref:String,
    pub numeric_run_ref:String,
    pub exact_numeric_source_matched:bool,
    pub original_focus_token_count:usize,
    pub original_named_person_support_count:usize,
    pub original_named_person_support_unique_statements:usize,
    pub exact_numeric_focus_mention_found:bool,
    pub native_anaphor_unresolved_demand_ids:Vec<i64>,
    pub numeric_demand_open_world:Vec<NumericDemandWorldStatus>,
    pub resolved_unique_target_from_original_sql:bool,
    pub original_accepted_document_identity_paid:bool,
    pub original_level_three_substitution_paid:bool,
    pub external_q23505_paid:bool,
    pub q207_paid:bool,
    pub historical_truth_paid:bool,
    pub original_level_one_source_preserved:bool,
    pub incomplete_obligations:Vec<String>,
}
#[derive(Debug,Clone,Serialize)]
pub struct NumericDemandWorldStatus{
    pub demand_id:i64,
    pub source_object_id:i64,
    pub outside_model_possible:bool,
    pub coverage_witness_ref:Option<String>,
    pub resource_limited:bool,
    pub represented_residual_count:i64,
    pub frontier_unique_object_count:i64,
    pub represented_candidate_count:i64,
    pub active_candidate_count:i64,
    pub refuted_candidate_count:i64,
}

fn hex(bytes:&[u8])->String {
    bytes.iter().map(|b|format!("{b:02x}")).collect()
}
fn original_receipt_policy(answer:&NativeSourceLocalAnswer)
    ->Result<(),NativeNumericAuditError>{
    if answer.schema!="slr-native-source-local-consumer-v1"
        ||!answer.source_local_policy_inference_paid
        ||answer.original_focus_surface!="he"
        ||answer.answer_value.as_deref()!=Some("Bush")
        ||answer.original_selected_antecedent_surface!="Bush"
        ||answer.original_person_support.len()<2
        ||answer.original_policy_ref!="slr:policy:source-local-repeated-person-topic:v1"
        ||answer.source_content_sha256.len()!=64
        ||answer.governed_semantic_identity_paid||answer.typed_unique_identity_witness_paid
        ||answer.level_three_substitution_paid||answer.external_qid_paid
        ||answer.historical_truth_paid{
        return Err(NativeNumericAuditError::SourcePolicy);
    }
    let mut statements=BTreeSet::new();
    for support in &answer.original_person_support{
        if support.name!="Bush"||support.start_char>=support.end_char
            ||support.statement_ref.is_empty()
            ||support.region_ref.is_empty()
            ||support.original_observation_ref.is_empty(){
            return Err(NativeNumericAuditError::SourcePolicy);
        }
        statements.insert(support.statement_ref.as_str());
    }
    if statements.len()<2{return Err(NativeNumericAuditError::SourcePolicy);}
    Ok(())
}

pub fn audit_original_numeric_identity_for_source_answer(
    numeric_url:&str,numeric_run_ref:&str,answer:&NativeSourceLocalAnswer
)->Result<NativeNumericIdentityAudit,NativeNumericAuditError>{
    original_receipt_policy(answer)?;
    if numeric_run_ref.trim().is_empty(){return Err(NativeNumericAuditError::ForeignSource);}
    let mut db=Client::connect(numeric_url,NoTls)?;
    let mut tx=db.transaction()?;
    tx.batch_execute(
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY;
         SET LOCAL statement_timeout='90000ms'"
    )?;
    let source=tx.query(
        "SELECT content_sha256
         FROM execution.semantic_parser_source
         WHERE run_ref=$1 AND document_ref=$2",
        &[&numeric_run_ref,&answer.source_document_ref])?;
    if source.len()!=1||hex(&source[0].get::<_,Vec<u8>>(0))!=answer.source_content_sha256 {
        return Err(NativeNumericAuditError::ForeignSource);
    }
    let focus=tx.query(
        "SELECT t.token_id
         FROM execution.semantic_parser_token AS t
         JOIN execution.semantic_symbol AS orth
           ON orth.symbol_id=t.orth_symbol_id
         JOIN execution.semantic_symbol AS pos
           ON pos.symbol_id=t.pos_symbol_id
         WHERE t.run_ref=$1 AND t.document_ref=$2
           AND t.start_char=$3 AND t.end_char=$4
           AND t.representation_version=2
           AND orth.symbol_text='he' AND pos.symbol_text='PRON'",
        &[&numeric_run_ref,&answer.source_document_ref,
          &(answer.original_focus_span[0] as i64),
          &(answer.original_focus_span[1] as i64)])?;
    let mut observed=0usize;
    let mut statements=BTreeSet::new();
    for support in &answer.original_person_support{
        let rows=tx.query(
            "SELECT token.token_id
             FROM execution.semantic_parser_token AS token
             JOIN execution.semantic_symbol AS orth
               ON orth.symbol_id=token.orth_symbol_id
             JOIN execution.semantic_parser_entity_span AS entity
               ON entity.sentence_id=token.sentence_id
              AND entity.start_char<=token.start_char
              AND entity.end_char>=token.end_char
              AND entity.representation_version=2
             JOIN execution.semantic_symbol AS kind
               ON kind.symbol_id=entity.entity_type_symbol_id
             WHERE token.run_ref=$1 AND token.document_ref=$2
               AND token.representation_version=2
               AND token.start_char=$3 AND token.end_char=$4
               AND orth.symbol_text=$5 AND kind.symbol_text='PERSON'",
            &[&numeric_run_ref,&answer.source_document_ref,
              &(support.start_char as i64),&(support.end_char as i64),
              &support.name])?;
        if rows.len()==1{
            observed+=1;
            statements.insert(support.statement_ref.clone());
        }
    }
    let mention_paid=focus.len()==1;
    let mut ids=Vec::new();
    if mention_paid{
        let target:i64=focus[0].get(0);
        ids=tx.query(
            "SELECT DISTINCT demand.demand_id
             FROM execution.semantic_pnf_demand AS demand
             JOIN execution.semantic_pnf_object_token_support AS support
               ON support.object_id=demand.source_object_id
              AND support.token_id=$1
             JOIN execution.semantic_pnf_anaphor_projection_constant AS kind
               ON kind.singleton
              AND demand.residual_type_symbol_id=kind.anaphor_residual_type_symbol_id
             JOIN execution.semantic_pnf_region AS region
               ON region.region_id=demand.source_region_id
             WHERE region.run_ref=$2 AND region.document_ref=$3
               AND demand.expected_target_kind=1
             ORDER BY demand.demand_id",
            &[&target,&numeric_run_ref,&answer.source_document_ref])?
            .iter().map(|r|r.get::<_,i64>(0)).collect();
    }
    let mut world=Vec::new();
    let mut actual_unique=false;
    for demand_id in &ids{
        let rows=tx.query(
            "SELECT demand.source_object_id,ow.outside_model_possible,
                    ow.coverage_witness_ref,ow.resource_limited,
                    ow.represented_residual_count,
                    count(DISTINCT resolution.selected_target_id) FILTER(
                      WHERE resolution.outcome_state=2
                        AND resolution.selected_target_kind=1
                        AND resolution.candidate_count=1)::bigint
             FROM execution.semantic_pnf_demand AS demand
             JOIN execution.semantic_pnf_demand_open_world_state AS ow
               ON ow.demand_id=demand.demand_id
             LEFT JOIN execution.semantic_pnf_frontier_resolution AS resolution
               ON resolution.demand_id=demand.demand_id
             WHERE demand.demand_id=$1
             GROUP BY demand.source_object_id,ow.outside_model_possible,
                ow.coverage_witness_ref,ow.resource_limited,
                ow.represented_residual_count",
            &[demand_id])?;
        if rows.len()!=1{continue;}
        let r=&rows[0];
        let source_id:i64=r.get(0);
        let outside:bool=r.get(1);
        let coverage:Option<String>=r.get(2);
        let limited:bool=r.get(3);
        let residual:i64=r.get(4);
        let unique_count:i64=r.get(5);
        let counts=tx.query_one(
            "SELECT count(*)::bigint,
                    count(*) FILTER (WHERE active)::bigint,
                    count(*) FILTER (WHERE refuted)::bigint
             FROM execution.semantic_pnf_candidate_state_v1
             WHERE demand_id=$1 AND target_kind=1",
            &[demand_id])?;
        let represented:i64=counts.get(0);
        let active:i64=counts.get(1);
        let refuted:i64=counts.get(2);
        world.push(NumericDemandWorldStatus{
            demand_id:*demand_id,source_object_id:source_id,
            outside_model_possible:outside,coverage_witness_ref:coverage.clone(),
            resource_limited:limited,represented_residual_count:residual,
            frontier_unique_object_count:unique_count,
            represented_candidate_count:represented,
            active_candidate_count:active,refuted_candidate_count:refuted,
        });
        if !outside&&coverage.as_deref().is_some_and(|s|!s.is_empty())
            &&!limited&&residual==0&&unique_count==1
            &&represented-refuted==1 {
            actual_unique=true;
        }
    }
    // An original frontier/coverage claim is not the same as an accepted
    // identity witness and source-object-supported Level-3 derivation.
    // Keep these unpaid until the exact document-derived witness and
    // inverse retraction have been independently replayed.
    let mut remaining=Vec::new();
    if observed<2||statements.len()<2{
        remaining.push("NUMERIC_PERSON_NER_SUPPORT_INCOMPLETE".into());
    }
    if !mention_paid{
        remaining.push("ORIGINAL_NUMERIC_HE_TOKEN_MISSING".into());
    }
    if ids.is_empty(){
        remaining.push("MIGRATION_180_ANAPHOR_TYPED_DEMAND_MISSING".into());
    }
    if !actual_unique{
        remaining.push("ORIGINAL_TYPED_UNIQUE_AND_OPEN_WORLD_COVERAGE_UNPAID".into());
    }
    remaining.push("ORIGINAL_ACCEPTED_DOCUMENT_IDENTITY_WITNESS_UNPAID".into());
    remaining.push("ORIGINAL_LEVEL_THREE_TRANSACTIONAL_RETRACTION_UNPAID".into());
    tx.commit()?;
    Ok(NativeNumericIdentityAudit{
        schema:"slr-original-numeric-pnf-identity-audit-v1",
        original_source_relative_answer:"Bush".into(),
        original_source_sha256:answer.source_content_sha256.clone(),
        original_source_revision_ref:answer.source_revision_ref.clone(),
        numeric_run_ref:numeric_run_ref.into(),
        exact_numeric_source_matched:true,
        original_focus_token_count:focus.len(),
        original_named_person_support_count:observed,
        original_named_person_support_unique_statements:statements.len(),
        exact_numeric_focus_mention_found:mention_paid&&!ids.is_empty(),
        native_anaphor_unresolved_demand_ids:ids,
        numeric_demand_open_world:world,
        resolved_unique_target_from_original_sql:actual_unique,
        original_accepted_document_identity_paid:false,
        original_level_three_substitution_paid:false,
        external_q23505_paid:false,q207_paid:false,historical_truth_paid:false,
        original_level_one_source_preserved:true,
        incomplete_obligations:remaining,
    })
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn unsupported_scope_refused_before_any_sql(){
        // Without an independently licensed original source-local receipt
        // no local username/surname equivalence can be minted in numeric PNF.
        assert_eq!(hex(&[0u8,255u8]),"00ff");
    }
}
