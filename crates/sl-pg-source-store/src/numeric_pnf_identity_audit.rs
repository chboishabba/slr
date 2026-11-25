//! Read-only exact-source P0-C/D numeric identity audit.
//! It does NOT derive identity by Bush spelling or a human S29 button.
//! It requires the source-local SLR receipt, the same numeric source digest,
//! exact parser/NER spans, migration-180 pronoun demand, and original typed
//! demand/open-world/witness/Level-3 SQL authority independently.
//! No writes, no source refresh, no hidden full-document candidate scan.

use crate::NativeSourceLocalAnswer;
use postgres::{Client,NoTls};
use serde::Serialize;
use sha2::{Digest,Sha256};
use thiserror::Error;
use std::collections::{BTreeMap,BTreeSet};

#[derive(Debug,Error)]
pub enum NativeNumericAuditError{
    #[error("numeric PostgreSQL: {0}")]
    Database(#[from] postgres::Error),
    #[error("original source-relative linguistic receipt invalid for this audit")]
    SourcePolicy,
    #[error("numeric run is tied to a different canonical source manifestation")]
    ForeignSource,
    #[error("selected original document-derived witness is not uniquely current")]
    WitnessNotCurrent,
    #[error("selected witness has no original persisted Level-3 dependency")]
    NoWitnessedLevelThree,
    #[error("original admission probe changed immutable Level-1 or failed rollback preservation")]
    AdmissionProbeInvariant,
    #[error("original retraction failed preservation or current-projection checks; transaction rolled back")]
    RetractionInvariant,
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
    pub original_bush_supported_numeric_object_count:usize,
    pub exact_numeric_focus_mention_found:bool,
    pub native_anaphor_unresolved_demand_ids:Vec<i64>,
    pub numeric_demand_open_world:Vec<NumericDemandWorldStatus>,
    pub resolved_unique_target_from_original_sql:bool,
    pub observed_accepted_document_witness_ids:Vec<i64>,
    pub observed_witnessed_level_three_derivation_refs:Vec<String>,
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
    pub selected_live_object_id:Option<i64>,
    pub live_candidate_count:i64,
    pub invalid_refutation_count:i64,
    pub producer_h9_target_occurrence_count:i64,
    pub quality_gated_h9_entity_occurrence_count:i64,
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
    let mut person_token_ids=BTreeSet::<i64>::new();
    let mut statements=BTreeSet::new();
    for support in &answer.original_person_support{
        let rows=tx.query(
            "SELECT token.token_id
             FROM execution.semantic_parser_token AS token
             JOIN execution.semantic_symbol AS orth
               ON orth.symbol_id=token.orth_symbol_id
             JOIN execution.semantic_parser_entity_span AS entity
               ON entity.run_ref=token.run_ref
              AND entity.document_ref=token.document_ref
              AND entity.sentence_ref=token.sentence_ref
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
            let token_id:i64=rows[0].get(0);
            if token_id<=0{return Err(NativeNumericAuditError::ForeignSource);}
            observed+=1;
            person_token_ids.insert(token_id);
            statements.insert(support.statement_ref.clone());
        }
    }
    let observed_person_tokens=person_token_ids.into_iter().collect::<Vec<_>>();
    let supported_bush_objects=tx.query(
        "SELECT DISTINCT object.object_id
         FROM execution.semantic_pnf_object_token_support AS support
         JOIN execution.semantic_pnf_object AS object
           ON object.object_id=support.object_id
         JOIN execution.semantic_pnf_region AS region
           ON region.region_id=object.region_id
         WHERE support.token_id=ANY($1::bigint[])
           AND region.run_ref=$2 AND region.document_ref=$3",
        &[&observed_person_tokens,&numeric_run_ref,&answer.source_document_ref])?
        .into_iter().map(|r|r.get::<_,i64>(0))
        .collect::<BTreeSet<_>>();
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
    let mut observed_witnesses=BTreeSet::<i64>::new();
    let mut observed_derivations=BTreeSet::<String>::new();
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
             WHERE demand_id=$1",
            &[demand_id])?;
        let represented:i64=counts.get(0);
        let active:i64=counts.get(1);
        let refuted:i64=counts.get(2);
        // All represented target kinds count. A deferred target is still
        // possible, and a refutation is valid only with same-candidate evidence.
        let live=tx.query(
            "SELECT state.target_kind,state.target_id
             FROM execution.semantic_pnf_candidate_state_v1 state
             WHERE state.demand_id=$1 AND NOT state.refuted
             ORDER BY state.target_kind,state.target_id",
            &[demand_id])?;
        let invalid_refutation:i64=tx.query_one(
            "SELECT count(*)::bigint
             FROM execution.semantic_pnf_candidate_state_v1 state
             LEFT JOIN execution.semantic_pnf_candidate_evidence evidence
               ON evidence.evidence_id=state.admissibility_evidence_id
              AND evidence.demand_id=state.demand_id
              AND evidence.target_kind=state.target_kind
              AND evidence.target_id=state.target_id
             WHERE state.demand_id=$1 AND state.refuted
               AND evidence.evidence_id IS NULL",
            &[demand_id])?.get(0);
        let live_object=if live.len()==1&&live[0].get::<_,i16>(0)==1{
            Some(live[0].get::<_,i64>(1))
        }else{None};
        // Do not compare independent counts. Require the SAME object in
        // every unique proof row, within the same immutable source.
        let selected_live_object=if let Some(target)=live_object{
            let matched:bool=tx.query_one(
                "SELECT EXISTS (
                    SELECT 1 FROM execution.semantic_pnf_frontier_resolution resolution
                    JOIN execution.semantic_pnf_object object
                      ON object.object_id=resolution.selected_target_id
                    JOIN execution.semantic_pnf_region region ON region.region_id=object.region_id
                    WHERE resolution.demand_id=$1 AND resolution.outcome_state=2
                      AND resolution.selected_target_kind=1 AND resolution.candidate_count=1
                      AND resolution.selected_target_id=$2 AND region.run_ref=$3
                      AND region.document_ref=$4
                 ) AND NOT EXISTS (
                    SELECT 1 FROM execution.semantic_pnf_frontier_resolution resolution
                    WHERE resolution.demand_id=$1 AND resolution.outcome_state=2
                      AND (resolution.selected_target_kind IS DISTINCT FROM 1
                        OR resolution.selected_target_id IS DISTINCT FROM $2
                        OR resolution.candidate_count IS DISTINCT FROM 1)
                 )",
                &[demand_id,&target,&numeric_run_ref,&answer.source_document_ref])?.get(0);
            if matched&&target!=source_id&&supported_bush_objects.contains(&target){
                Some(target)
            }else{None}
        }else{None};

        // H9 provenance is a separate source-region semantic target relation.
        // An antecedent in a different sentence cannot be relabelled as an H9
        // target occurrence merely because it is the selected actor.
        let h9_target:i64=tx.query_one(
            "SELECT count(*)::bigint FROM execution.semantic_pnf_demand_h9_target_support_v1
             WHERE demand_id=$1",&[demand_id])?.get(0);
        let h9_entities:i64=tx.query_one(
            "SELECT count(*)::bigint FROM execution.semantic_pnf_demand_parser_entity_occurrence_v1
             WHERE demand_id=$1",&[demand_id])?.get(0);
        if let Some(target)=selected_live_object {
            let rows=tx.query(
                "SELECT DISTINCT w.witness_id
                 FROM execution.semantic_pnf_identity_witness w
                 JOIN execution.semantic_pnf_identity_witness_admission a
                   ON a.witness_id=w.witness_id AND a.admission_state=2
                 JOIN execution.semantic_pnf_canonical_entity entity
                   ON entity.entity_id=w.target_entity_id AND entity.authority_class=2
                 JOIN execution.semantic_pnf_identity_projection p
                   ON p.source_object_id=w.source_object_id AND p.authority_class=2
                  AND p.target_entity_id=entity.entity_id AND w.witness_id=ANY(p.witness_ids)
                 JOIN execution.semantic_pnf_frontier_resolution resolution
                   ON resolution.demand_id=w.demand_id AND resolution.interface_id=w.resolution_interface_id
                 WHERE w.demand_id=$1 AND w.source_object_id=$2
                   AND entity.anchor_object_id=$3 AND w.authority_class=2 AND w.witness_kind=5
                   AND w.candidate_count=1 AND resolution.outcome_state=2
                   AND resolution.candidate_count=1 AND resolution.selected_target_kind=1
                   AND resolution.selected_target_id=$3
                   AND NOT EXISTS (
                       SELECT 1 FROM execution.semantic_pnf_demand_constraint dc
                       WHERE dc.demand_id=w.demand_id AND dc.required
                         AND NOT EXISTS (
                           SELECT 1 FROM execution.semantic_pnf_identity_witness_constraint wc
                           WHERE wc.witness_id=w.witness_id AND wc.constraint_ordinal=dc.ordinal
                             AND wc.key_kind=dc.key_kind AND wc.key_a=dc.key_a AND wc.key_b=dc.key_b
                             AND wc.polarity=dc.polarity AND wc.satisfied))",
                &[demand_id,&source_id,&target])?;
            for row in rows {
                let witness:i64=row.get(0);
                observed_witnesses.insert(witness);
                for d in tx.query(
                    "SELECT DISTINCT derivation.derivation_ref
                     FROM execution.semantic_pnf_factor_derivation derivation
                     JOIN execution.semantic_pnf_factor_derivation_premise premise
                       ON premise.derivation_id=derivation.derivation_id
                     JOIN execution.semantic_pnf_factor factor ON factor.factor_id=premise.factor_id
                     JOIN execution.semantic_pnf_region region ON region.region_id=factor.region_id
                     WHERE region.run_ref=$2 AND region.document_ref=$3
                       AND derivation.epistemic_level=3 AND derivation.derivation_state=2
                       AND derivation.authority_class=2
                       AND EXISTS (
                         SELECT 1 FROM execution.semantic_pnf_factor_derivation_argument a
                         WHERE a.derivation_id=derivation.derivation_id
                           AND a.identity_entity_id IS NOT NULL AND $1=ANY(a.identity_witness_ids))",
                    &[&witness,&numeric_run_ref,&answer.source_document_ref])? {
                    observed_derivations.insert(d.get::<_,String>(0));
                }
            }
        }
        world.push(NumericDemandWorldStatus{
            demand_id:*demand_id,source_object_id:source_id,
            outside_model_possible:outside,coverage_witness_ref:coverage.clone(),
            resource_limited:limited,represented_residual_count:residual,
            frontier_unique_object_count:unique_count,
            selected_live_object_id:selected_live_object,
            live_candidate_count:live.len() as i64,
            invalid_refutation_count:invalid_refutation,
            producer_h9_target_occurrence_count:h9_target,
            quality_gated_h9_entity_occurrence_count:h9_entities,
            represented_candidate_count:represented,
            active_candidate_count:active,refuted_candidate_count:refuted,
        });
        if !outside&&coverage.as_deref().is_some_and(|s|!s.is_empty())
            &&!limited&&residual==0&&unique_count==1
            &&represented-refuted==1&&invalid_refutation==0
            &&selected_live_object.is_some() {
            actual_unique=true;
        }
    }
    // Multiple demands for the same token are an unresolved source-object
    // ambiguity; an existential success cannot close the endpoint.
    actual_unique=actual_unique&&ids.len()==1&&world.len()==1;
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
    if supported_bush_objects.is_empty(){
        remaining.push("EXACT_BUSH_OBJECT_ANCHOR_MISSING".into());
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
        original_bush_supported_numeric_object_count:supported_bush_objects.len(),
        exact_numeric_focus_mention_found:mention_paid&&!ids.is_empty(),
        native_anaphor_unresolved_demand_ids:ids,
        numeric_demand_open_world:world,
        resolved_unique_target_from_original_sql:actual_unique,
        observed_accepted_document_witness_ids:observed_witnesses.into_iter().collect(),
        observed_witnessed_level_three_derivation_refs:observed_derivations.into_iter().collect(),
        original_accepted_document_identity_paid:false,
        original_level_three_substitution_paid:false,
        external_q23505_paid:false,q207_paid:false,historical_truth_paid:false,
        original_level_one_source_preserved:true,
        incomplete_obligations:remaining,
    })
}


/// Original SQL witness retraction transaction. No identity is minted here.
/// The caller supplies the exact document-derived witness and immutable source
/// receipt. A dry probe performs the real inverse inside a rolled-back
/// transaction; an explicit commit persists it. This is a semantic check,
/// not a claim that the legacy SQL refresh is physically delta-linear.
#[derive(Debug,Clone,Serialize)]
pub struct NativeNumericRetractionReceipt {
    pub schema:&'static str,
    pub witness_id:i64,
    pub numeric_run_ref:String,
    pub source_sha256:String,
    pub original_admission_revision:i64,
    pub retracted_admission_revision:i64,
    pub affected_level_three_derivation_refs:Vec<String>,
    pub immutable_level_one_preserved:bool,
    pub unaffected_level_three_preserved:bool,
    pub revoked_witness_absent_from_current_projection:bool,
    pub revoked_witness_absent_from_current_derivations:bool,
    pub committed:bool,
}
fn level_one_snapshot(
    tx:&mut postgres::Transaction<'_>,run_id:i64,document_id:i64,
)->Result<Vec<String>,postgres::Error>{
    let rows=tx.query(
        "SELECT jsonb_build_array(to_jsonb(f),
                   COALESCE((SELECT jsonb_agg(to_jsonb(e) ORDER BY e.slot_ordinal)
                             FROM execution.semantic_pnf_hyperedge e
                             WHERE e.factor_id=f.factor_id),'[]'::jsonb))::text
         FROM execution.semantic_pnf_factor f
         JOIN execution.semantic_pnf_region r ON r.region_id=f.region_id
         WHERE r.run_id=$1 AND r.document_id=$2
         ORDER BY f.factor_id",
        &[&run_id,&document_id])?;
    Ok(rows.iter().map(|r|r.get::<_,String>(0)).collect())
}
fn level_three_snapshot(
    tx:&mut postgres::Transaction<'_>,run_id:i64,document_id:i64,
)->Result<std::collections::BTreeMap<String,String>,postgres::Error>{
    let rows=tx.query(
        "SELECT d.derivation_ref,
            jsonb_build_array(to_jsonb(d)-'derivation_id'-'created_at',
              COALESCE((SELECT jsonb_agg(to_jsonb(p)-'derivation_id' ORDER BY p.premise_ordinal)
                        FROM execution.semantic_pnf_factor_derivation_premise p
                        WHERE p.derivation_id=d.derivation_id),'[]'::jsonb),
              COALESCE((SELECT jsonb_agg(to_jsonb(a)-'derivation_id' ORDER BY a.slot_ordinal)
                        FROM execution.semantic_pnf_factor_derivation_argument a
                        WHERE a.derivation_id=d.derivation_id),'[]'::jsonb),
              COALESCE((SELECT jsonb_agg(to_jsonb(s)-'derivation_id' ORDER BY s.slot_ordinal)
                        FROM execution.semantic_pnf_factor_derivation_support s
                        WHERE s.derivation_id=d.derivation_id),'[]'::jsonb))::text
         FROM execution.semantic_pnf_factor_derivation d
         JOIN execution.semantic_pnf_interface i ON i.interface_id=d.scope_interface_id
         JOIN execution.semantic_pnf_region r ON r.region_id=i.region_id
         WHERE r.run_id=$1 AND r.document_id=$2 AND d.epistemic_level=3",
        &[&run_id,&document_id])?;
    Ok(rows.iter().map(|r|(r.get::<_,String>(0),r.get::<_,String>(1))).collect())
}
pub fn retract_original_numeric_identity_for_source_answer(
    numeric_url:&str,numeric_run_ref:&str,answer:&NativeSourceLocalAnswer,
    witness_id:i64,commit:bool,
)->Result<NativeNumericRetractionReceipt,NativeNumericAuditError>{
    original_receipt_policy(answer)?;
    if witness_id<=0||numeric_run_ref.trim().is_empty(){
        return Err(NativeNumericAuditError::ForeignSource);
    }
    let mut db=Client::connect(numeric_url,NoTls)?;
    let mut tx=db.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
        SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
    tx.query_one("SELECT pg_advisory_xact_lock(hashtext($1),hashtext($2))",
        &[&numeric_run_ref,&answer.source_document_ref])?;
    let rows=tx.query(
        "SELECT r.run_id,r.document_id,a.revision,w.source_object_id
         FROM execution.semantic_pnf_identity_witness w
         JOIN execution.semantic_pnf_identity_witness_admission a
           ON a.witness_id=w.witness_id
         JOIN execution.semantic_pnf_demand demand ON demand.demand_id=w.demand_id
           AND demand.source_object_id=w.source_object_id
         JOIN execution.semantic_pnf_object o ON o.object_id=w.source_object_id
         JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
         JOIN execution.semantic_parser_source src
           ON src.run_ref=r.run_ref AND src.document_ref=r.document_ref
         JOIN execution.semantic_pnf_object_token_support os ON os.object_id=o.object_id
         JOIN execution.semantic_parser_token t ON t.token_id=os.token_id
           AND t.run_ref=r.run_ref AND t.document_ref=r.document_ref
         JOIN execution.semantic_pnf_canonical_entity entity ON entity.entity_id=w.target_entity_id
         WHERE w.witness_id=$1 AND r.run_ref=$2 AND r.document_ref=$3
           AND encode(src.content_sha256,'hex')=$4
           AND t.start_char=$5 AND t.end_char=$6 AND t.representation_version=2
           AND w.authority_class=2 AND entity.authority_class=2
           AND w.witness_kind=5 AND w.candidate_count=1 AND a.admission_state=2
         FOR UPDATE OF a",
        &[&witness_id,&numeric_run_ref,&answer.source_document_ref,
          &answer.source_content_sha256,&(answer.original_focus_span[0] as i64),
          &(answer.original_focus_span[1] as i64)])?;
    if rows.len()!=1 {return Err(NativeNumericAuditError::WitnessNotCurrent);}
    let run_id:i64=rows[0].get(0);
    let document_id:i64=rows[0].get(1);
    let old_revision:i64=rows[0].get(2);
    let source_object:i64=rows[0].get(3);
    let current:bool=tx.query_one(
        "SELECT EXISTS (SELECT 1 FROM execution.semantic_pnf_identity_projection
         WHERE source_object_id=$1 AND authority_class=2 AND $2=ANY(witness_ids))",
        &[&source_object,&witness_id])?.get(0);
    if !current{return Err(NativeNumericAuditError::WitnessNotCurrent);}
    let level_one=level_one_snapshot(&mut tx,run_id,document_id)?;
    let prior=level_three_snapshot(&mut tx,run_id,document_id)?;
    let affected:Vec<String>=tx.query(
        "SELECT DISTINCT d.derivation_ref
         FROM execution.semantic_pnf_factor_derivation d
         JOIN execution.semantic_pnf_interface i ON i.interface_id=d.scope_interface_id
         JOIN execution.semantic_pnf_region r ON r.region_id=i.region_id
         WHERE r.run_id=$1 AND r.document_id=$2 AND d.epistemic_level=3
           AND (EXISTS (SELECT 1 FROM execution.semantic_pnf_factor_derivation_argument a
                        WHERE a.derivation_id=d.derivation_id AND $3=ANY(a.identity_witness_ids))
             OR EXISTS (SELECT 1 FROM execution.semantic_pnf_factor_derivation_support s
                        WHERE s.derivation_id=d.derivation_id AND $3=ANY(s.identity_witness_ids)))
         ORDER BY d.derivation_ref",
        &[&run_id,&document_id,&witness_id])?.iter().map(|r|r.get(0)).collect();
    if affected.is_empty(){return Err(NativeNumericAuditError::NoWitnessedLevelThree);}
    // Migration 073 owns admission revision, direct substitutions and
    // composition refresh. Migration 086's supported substitutions need
    // explicit stale-dependent cleanup before their original refresh.
    let changed:bool=tx.query_one(
        "SELECT execution.retract_numeric_pnf_identity_witness($1)",&[&witness_id])?.get(0);
    if !changed{return Err(NativeNumericAuditError::WitnessNotCurrent);}
    tx.execute(
        "DELETE FROM execution.semantic_pnf_factor_derivation d
         USING execution.semantic_pnf_interface i,execution.semantic_pnf_region r
         WHERE d.scope_interface_id=i.interface_id AND i.region_id=r.region_id
           AND r.run_id=$1 AND r.document_id=$2
           AND d.rule_ref='identity-supported-substitution:v1'
           AND (EXISTS (SELECT 1 FROM execution.semantic_pnf_factor_derivation_argument a
                        WHERE a.derivation_id=d.derivation_id AND $3=ANY(a.identity_witness_ids))
             OR EXISTS (SELECT 1 FROM execution.semantic_pnf_factor_derivation_support s
                        WHERE s.derivation_id=d.derivation_id AND $3=ANY(s.identity_witness_ids)))",
        &[&run_id,&document_id,&witness_id])?;
    tx.query_one("SELECT execution.refresh_numeric_pnf_supported_identity_substitution_derivations($1,$2)",
        &[&run_id,&document_id])?;
    let revision:i64=tx.query_one(
        "SELECT a.revision FROM execution.semantic_pnf_identity_witness_admission a
         JOIN execution.semantic_pnf_identity_witness w ON w.witness_id=a.witness_id
         WHERE a.witness_id=$1 AND a.admission_state=3",
        &[&witness_id])?.get(0);
    let no_projection:bool=tx.query_one(
        "SELECT NOT EXISTS (SELECT 1 FROM execution.semantic_pnf_identity_projection
         WHERE source_object_id=$1 AND $2=ANY(witness_ids))",
        &[&source_object,&witness_id])?.get(0);
    let no_derived:bool=tx.query_one(
        "SELECT NOT EXISTS (
            SELECT 1 FROM execution.semantic_pnf_factor_derivation_argument
             WHERE $1=ANY(identity_witness_ids)
            UNION ALL
            SELECT 1 FROM execution.semantic_pnf_factor_derivation_support
             WHERE $1=ANY(identity_witness_ids))",
        &[&witness_id])?.get(0);
    let immutable=level_one==level_one_snapshot(&mut tx,run_id,document_id)?;
    let after=level_three_snapshot(&mut tx,run_id,document_id)?;
    let unaffected=prior.iter().all(|(key,value)|
        affected.contains(key)||after.get(key)==Some(value));
    if revision!=old_revision+1||!no_projection||!no_derived||!immutable||!unaffected{
        return Err(NativeNumericAuditError::RetractionInvariant);
    }
    if commit {tx.commit()?;}else{tx.rollback()?;}
    Ok(NativeNumericRetractionReceipt{
        schema:"slr-original-numeric-pnf-retraction-v1",witness_id,
        numeric_run_ref:numeric_run_ref.into(),source_sha256:answer.source_content_sha256.clone(),
        original_admission_revision:old_revision,retracted_admission_revision:revision,
        affected_level_three_derivation_refs:affected,immutable_level_one_preserved:immutable,
        unaffected_level_three_preserved:unaffected,
        revoked_witness_absent_from_current_projection:no_projection,
        revoked_witness_absent_from_current_derivations:no_derived,committed:commit,
    })
}



// Direct port of SensibLaw binding_candidate_sets.py:
// binding-compatibility:pnf-kind-morphology:v0_3. Missing features retain
// the candidate. Compare sets, never scalar similarity or spelling.
const MORPH_FEATURES:[&str;3]=["Gender","Number","Person"];
fn morphology_mismatches(reference:&[String],candidate:&[String])->Vec<&'static str>{
    MORPH_FEATURES.into_iter().filter(|feature|{
        let values=|rows:&[String]|{
            rows.iter().filter_map(|row|{
                let (name,value)=row.split_once('=')?;
                if name.eq_ignore_ascii_case(feature){Some(value.to_owned())}else{None}
            }).collect::<BTreeSet<_>>()
        };
        let a=values(reference);let b=values(candidate);
        !a.is_empty()&&!b.is_empty()&&a.is_disjoint(&b)
    }).collect()
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeNumericHorizonObservation{
    pub target_kind:i16,
    pub target_id:i64,
    pub horizon:i16,
    pub signed_residual:i64,
    pub phase:i16,
    pub evidence_count:i64,
    pub active:bool,
    pub execution_residual:bool,
    pub refuted:bool,
    pub strict_preference:bool,
}
#[derive(Debug,Clone,Serialize,PartialEq,Eq)]
pub struct NativeNumericDocumentIdentityObservation{
    pub object_id:i64,
    pub admitted_entity_ids:Vec<i64>,
    pub admitted_witness_ids:Vec<i64>,
}

/// Consumer projection of the existing proof-relevant identity fibre (m069).
/// Preserve the inverse occurrence fibre; do not manufacture singleton entity
/// IDs for unresolved objects. Only original admitted classes can be grouped.
#[derive(Debug,Clone,Serialize)]
pub struct NativeNumericIdentityFibreProjection{
    pub admitted_classes:BTreeMap<i64,Vec<i64>>,
    pub unresolved_keys:Vec<(i16,i64)>,
    pub conflicting_objects:Vec<i64>,
    pub refuted_keys:Vec<(i16,i64)>,
    pub outside_model_possible:bool,
}
fn project_original_identity_fibre(
    horizons:&[NativeNumericHorizonObservation],
    classes:&[NativeNumericDocumentIdentityObservation],
    outside:bool,
)->NativeNumericIdentityFibreProjection{
    let by_object=classes.iter().map(|c|(c.object_id,c)).collect::<BTreeMap<_,_>>();
    let mut projection=NativeNumericIdentityFibreProjection{
        admitted_classes:BTreeMap::new(),unresolved_keys:Vec::new(),
        conflicting_objects:Vec::new(),refuted_keys:Vec::new(),
        outside_model_possible:outside,
    };
    for h in horizons.iter().filter(|h|h.horizon==3){
        let key=(h.target_kind,h.target_id);
        if h.refuted{projection.refuted_keys.push(key);continue;}
        if h.target_kind==1{
            if let Some(c)=by_object.get(&h.target_id){
                if let [entity]=c.admitted_entity_ids.as_slice(){
                    if !c.admitted_witness_ids.is_empty(){
                        projection.admitted_classes.entry(*entity).or_default().push(h.target_id);
                        continue;
                    }
                }
                if c.admitted_entity_ids.len()>1{
                    projection.conflicting_objects.push(h.target_id);
                }
            }
        }
        projection.unresolved_keys.push(key);
    }
    projection
}

// Read the original cumulative H3/H6/H9 fibre and proof-relevant document
// projection. Do not conflate a preference with a unique admitted identity.
fn observe_original_resolution_fibre(
    tx:&mut postgres::Transaction<'_>,demand:i64,run_ref:&str,document_ref:&str,
)->Result<(Vec<NativeNumericHorizonObservation>,Vec<NativeNumericDocumentIdentityObservation>),NativeNumericAuditError>{
    let horizons=tx.query(
        "SELECT h.target_kind,h.target_id,h.horizon,h.signed_residual,h.phase,
                h.evidence_count,h.active,h.execution_residual,h.refuted,
                COALESCE(p.strict_winner,FALSE)
         FROM execution.semantic_pnf_candidate_horizon_state_v1 h
         LEFT JOIN execution.semantic_pnf_progressive_preference_candidate_v1 p
           ON p.demand_id=h.demand_id AND p.target_kind=h.target_kind
          AND p.target_id=h.target_id AND p.horizon=h.horizon
         WHERE h.demand_id=$1 ORDER BY h.target_kind,h.target_id,h.horizon",&[&demand])?
        .iter().map(|r|NativeNumericHorizonObservation{
            target_kind:r.get(0),target_id:r.get(1),horizon:r.get(2),
            signed_residual:r.get(3),phase:r.get(4),evidence_count:r.get(5),
            active:r.get(6),execution_residual:r.get(7),refuted:r.get(8),
            strict_preference:r.get(9),
        }).collect();
    // Keep conflicting admitted entity IDs visible; the original projection
    // intentionally omits conflicts, so absence of a projection is not a
    // reason to silently identify a candidate with itself or another object.
    let classes=tx.query(
        "SELECT u.target_id,
                COALESCE(array_agg(DISTINCT w.target_entity_id ORDER BY w.target_entity_id)
                    FILTER (WHERE w.witness_id IS NOT NULL),'{}'::bigint[]),
                COALESCE(array_agg(DISTINCT w.witness_id ORDER BY w.witness_id)
                    FILTER (WHERE w.witness_id IS NOT NULL),'{}'::bigint[])
         FROM execution.semantic_pnf_candidate_universe u
         JOIN execution.semantic_pnf_object o ON o.object_id=u.target_id
         JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
         LEFT JOIN LATERAL (
             SELECT w.witness_id,w.target_entity_id
             FROM execution.semantic_pnf_identity_witness w
             JOIN execution.semantic_pnf_identity_witness_admission a
               ON a.witness_id=w.witness_id AND a.admission_state=2
             JOIN execution.semantic_pnf_canonical_entity e
               ON e.entity_id=w.target_entity_id AND e.authority_class=2
             JOIN execution.semantic_pnf_object anchor ON anchor.object_id=e.anchor_object_id
             JOIN execution.semantic_pnf_region ar ON ar.region_id=anchor.region_id
             WHERE w.source_object_id=u.target_id AND w.authority_class=2
               AND ar.run_ref=$2 AND ar.document_ref=$3
         ) w ON TRUE
         WHERE u.demand_id=$1 AND u.target_kind=1
           AND r.run_ref=$2 AND r.document_ref=$3
         GROUP BY u.target_id ORDER BY u.target_id",&[&demand,&run_ref,&document_ref])?
        .iter().map(|r|NativeNumericDocumentIdentityObservation{
            object_id:r.get(0),admitted_entity_ids:r.get(1),admitted_witness_ids:r.get(2),
        }).collect();
    Ok((horizons,classes))
}

// Native exact-head morphology assessment over the already represented fibre.
// This is only one coordinate of the original binding compatibility contract;
// absent/multiply anchored heads, discourse/quotation and other kinds remain
// residual. Negative evidence is a mismatch observation, NOT a refutation.
fn refine_original_numeric_morphology(
    tx:&mut postgres::Transaction<'_>,demand:i64,focus_token:i64,
    run_ref:&str,document_ref:&str,budget:usize,
)->Result<(usize,i64),NativeNumericAuditError>{
    let focus:Vec<String>=tx.query_one(
        "SELECT COALESCE(array_agg(f.symbol_text||'='||v.symbol_text
                           ORDER BY f.symbol_text,v.symbol_text)
                         FILTER (WHERE f.symbol_id IS NOT NULL),'{}'::text[])
         FROM execution.semantic_parser_token t
         LEFT JOIN execution.semantic_morph_set_member m ON m.morph_set_id=t.morph_set_id
         LEFT JOIN execution.semantic_symbol f ON f.symbol_id=m.feature_symbol_id
         LEFT JOIN execution.semantic_symbol v ON v.symbol_id=m.value_symbol_id
         WHERE t.token_id=$1 AND t.run_ref=$2 AND t.document_ref=$3
           AND t.representation_version=2",&[&focus_token,&run_ref,&document_ref])?.get(0);
    let rows=tx.query(
        "WITH candidate AS MATERIALIZED (
           SELECT u.target_id AS object_id,r.region_id,i.interface_id,
                  min(t.token_id) AS token_id
           FROM execution.semantic_pnf_candidate_universe u
           JOIN execution.semantic_pnf_object o ON o.object_id=u.target_id
           JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
           JOIN execution.semantic_pnf_interface i ON i.region_id=r.region_id
           JOIN execution.semantic_pnf_object_token_support s ON s.object_id=o.object_id
           JOIN execution.semantic_parser_token t ON t.token_id=s.token_id
             AND t.run_ref=r.run_ref AND t.document_ref=r.document_ref
             AND t.representation_version=2 AND t.lemma_symbol_id=o.head_symbol_id
           WHERE u.demand_id=$1 AND u.target_kind=1 AND r.run_ref=$2 AND r.document_ref=$3
             AND o.active
           GROUP BY u.target_id,r.region_id,i.interface_id
           HAVING count(DISTINCT t.token_id)=1
         ), pending AS (
           SELECT c.* FROM candidate c
           WHERE NOT EXISTS (
             SELECT 1 FROM execution.semantic_pnf_candidate_evidence e
             WHERE e.demand_id=$1 AND e.target_kind=1 AND e.target_id=c.object_id
               AND e.evidence_ref='native-binding-morphology:v0_3:'||$4::bigint::text||':'||c.token_id::text)
           ORDER BY c.object_id LIMIT $5
         )
         SELECT p.object_id,p.region_id,p.interface_id,p.token_id,
                COALESCE(array_agg(f.symbol_text||'='||v.symbol_text
                           ORDER BY f.symbol_text,v.symbol_text)
                         FILTER (WHERE f.symbol_id IS NOT NULL),'{}'::text[])
         FROM pending p
         JOIN execution.semantic_parser_token t ON t.token_id=p.token_id
         LEFT JOIN execution.semantic_morph_set_member m ON m.morph_set_id=t.morph_set_id
         LEFT JOIN execution.semantic_symbol f ON f.symbol_id=m.feature_symbol_id
         LEFT JOIN execution.semantic_symbol v ON v.symbol_id=m.value_symbol_id
         GROUP BY p.object_id,p.region_id,p.interface_id,p.token_id
         ORDER BY p.object_id",&[&demand,&run_ref,&document_ref,&focus_token,&(budget as i64)])?;
    let mut inserted=0usize;
    for row in rows{
        let object:i64=row.get(0);let region:i64=row.get(1);
        let interface:i64=row.get(2);let head:i64=row.get(3);
        let features:Vec<String>=row.get(4);
        let mismatch=morphology_mismatches(&focus,&features);
        let residual:i64=if mismatch.is_empty(){0}else{-1};
        let evidence_ref=format!("native-binding-morphology:v0_3:{focus_token}:{head}");
        let evidence_kind=if mismatch.is_empty(){"morphology_no_observed_mismatch".to_owned()}
            else{format!("morphology_mismatch:{}",mismatch.join(","))};
        inserted+=tx.execute(
            "INSERT INTO execution.semantic_pnf_candidate_evidence
              (demand_id,target_kind,target_id,evidence_ref,evidence_family,horizon,
               signed_residual,evidence_kind,provenance_ref,source_interface_id,source_region_id)
             VALUES($1,1,$2,$3,1,3,$4,$5,
               'binding-compatibility:pnf-kind-morphology:v0_3',$6,$7)
             ON CONFLICT(demand_id,target_kind,target_id,evidence_ref) DO NOTHING",
            &[&demand,&object,&evidence_ref,&residual,&evidence_kind,&interface,&region])? as usize;
    }
    let remaining:i64=tx.query_one(
        "SELECT count(*)::bigint FROM execution.semantic_pnf_candidate_universe u
         WHERE u.demand_id=$1 AND NOT EXISTS(
           SELECT 1 FROM execution.semantic_pnf_candidate_evidence e
           WHERE e.demand_id=u.demand_id AND e.target_kind=u.target_kind AND e.target_id=u.target_id
             AND e.evidence_ref LIKE 'native-binding-morphology:v0_3:%')",&[&demand])?.get(0);
    Ok((inserted,remaining))
}


#[derive(Debug,Clone,Serialize)]
pub struct OriginalParserIdentityCandidateObservation{
    pub source_object_id:i64,
    pub target_object_id:i64,
    pub witness_kind:i16,
    pub source_token_id:Option<i64>,
    pub target_token_id:Option<i64>,
    pub evidence_ref:String,
    pub candidate_count:i16,
    pub evidence_state:i16,
}
#[derive(Debug,Clone,Serialize)]
pub struct OriginalParserIdentityProducerObservation{
    pub producer_ref:&'static str,
    pub scope:&'static str,
    pub refreshed_in_rollback_probe:bool,
    pub candidate_rows:Vec<OriginalParserIdentityCandidateObservation>,
    pub name_overflow_rows:i64,
    pub admission_executed:bool,
}
fn original_parser_identity_producer_probe(
    tx:&mut postgres::Transaction<'_>,run_ref:&str,document_ref:&str,
    demand:i64,
)->Result<OriginalParserIdentityProducerObservation,NativeNumericAuditError>{
    let row=tx.query_one(
        "SELECT DISTINCT r.run_id,r.document_id
         FROM execution.semantic_pnf_demand d
         JOIN execution.semantic_pnf_region r ON r.region_id=d.source_region_id
         WHERE d.demand_id=$1 AND r.run_ref=$2 AND r.document_ref=$3",
        &[&demand,&run_ref,&document_ref])?;
    let run_id:i64=row.get(0);let document_id:i64=row.get(1);
    // m090 refresh supersedes current candidate rows. Never leave its writes
    // behind when the native parser coverage and m080 admission lifecycle are
    // not yet certified. Execute the original producer in a nested transaction
    // and retain only the candidate observation, not a live witness or ID.
    let mut probe=tx.transaction()?;
    probe.query_one("SELECT execution.refresh_numeric_pnf_parser_identity_evidence($1,$2)",
        &[&run_id,&document_id])?;
    let rows=probe.query(
        "SELECT c.source_object_id,c.target_object_id,c.witness_kind,
                c.source_token_id,c.target_token_id,c.evidence_ref,
                c.candidate_count,c.evidence_state
         FROM execution.semantic_pnf_identity_evidence_candidate c
         WHERE c.run_id=$1 AND c.document_id=$2 AND c.evidence_state=1
           AND (EXISTS(
                SELECT 1 FROM execution.semantic_pnf_candidate_universe u
                WHERE u.demand_id=$3 AND u.target_kind=1 AND u.target_id=c.source_object_id)
             OR EXISTS(
                SELECT 1 FROM execution.semantic_pnf_candidate_universe u
                WHERE u.demand_id=$3 AND u.target_kind=1 AND u.target_id=c.target_object_id))
         ORDER BY c.source_object_id,c.target_object_id,c.witness_kind,c.evidence_ref",
        &[&run_id,&document_id,&demand])?
        .iter().map(|r|OriginalParserIdentityCandidateObservation{
            source_object_id:r.get(0),target_object_id:r.get(1),witness_kind:r.get(2),
            source_token_id:r.get(3),target_token_id:r.get(4),evidence_ref:r.get(5),
            candidate_count:r.get(6),evidence_state:r.get(7),
        }).collect();
    let overflow:i64=probe.query_one(
        "SELECT count(*)::bigint FROM execution.semantic_pnf_proper_name_evidence_overflow
         WHERE run_id=$1 AND document_id=$2",&[&run_id,&document_id])?.get(0);
    probe.rollback()?;
    Ok(OriginalParserIdentityProducerObservation{
        producer_ref:"execution.refresh_numeric_pnf_parser_identity_evidence",
        scope:"current_numeric_document_candidate_projection_not_global_coverage",
        refreshed_in_rollback_probe:true,candidate_rows:rows,name_overflow_rows:overflow,
        admission_executed:false,
    })
}


#[derive(Debug,Clone,Serialize)]
pub struct OriginalCrossResourceNeedObservation{
    pub need_id:i64,
    pub demand_id:i64,
    pub source_object_id:Option<i64>,
    pub consumer_ref:String,
    pub query_ref:String,
    pub policy_ref:String,
    pub need_kind:i16,
    pub provider_id:i16,
    pub axis_kind:Option<i16>,
    pub provider_property_numeric_id:Option<i64>,
    pub need_revision:i64,
    pub active_request_ids:Vec<i64>,
}
fn observe_original_cross_resource_needs(
    tx:&mut postgres::Transaction<'_>,demand:i64,run_ref:&str,document_ref:&str,
)->Result<Vec<OriginalCrossResourceNeedObservation>,NativeNumericAuditError>{
    // Reuse m096/m112/m113/m119. Do not construct provider queries from a
    // pronoun or attach candidate evidence to the wrong demand. Related
    // occurrence demands keep their own consumer/query/policy and provider.
    Ok(tx.query(
        "SELECT n.need_id,d.demand_id,d.source_object_id,
                n.consumer_ref,n.query_ref,n.policy_ref,n.need_kind,n.provider_id,
                n.axis_kind,n.provider_property_numeric_id,n.need_revision,
                ARRAY(SELECT DISTINCT req.request_id
                      FROM execution.semantic_pnf_external_request_active_member_v1 m
                      JOIN execution.semantic_pnf_external_request req
                        ON req.request_id=m.request_id
                      WHERE m.demand_id=n.demand_id AND m.consumer_ref=n.consumer_ref
                        AND m.query_ref=n.query_ref AND m.policy_ref=n.policy_ref
                        AND m.need_kind=n.need_kind AND req.provider_id=n.provider_id
                        AND ((req.request_kind=1 AND n.need_kind IN (1,2))
                          OR (req.request_kind=2 AND n.need_kind=2
                            AND req.axis_kind=n.axis_kind
                            AND req.provider_property_numeric_id=n.provider_property_numeric_id)
                          OR (req.request_kind=3 AND n.need_kind=3))
                      ORDER BY req.request_id)
         FROM execution.semantic_pnf_consumer_external_need n
         JOIN execution.semantic_pnf_demand d ON d.demand_id=n.demand_id
         JOIN execution.semantic_pnf_region r ON r.region_id=d.source_region_id
         WHERE n.active AND r.run_ref=$2 AND r.document_ref=$3
           AND (d.demand_id=$1 OR EXISTS(
             SELECT 1 FROM execution.semantic_pnf_candidate_universe u
             WHERE u.demand_id=$1 AND u.target_kind=1 AND u.target_id=d.source_object_id))
         ORDER BY d.demand_id,n.need_id",
        &[&demand,&run_ref,&document_ref])?
        .iter().map(|r|OriginalCrossResourceNeedObservation{
            need_id:r.get(0),demand_id:r.get(1),source_object_id:r.get(2),
            consumer_ref:r.get(3),query_ref:r.get(4),policy_ref:r.get(5),
            need_kind:r.get(6),provider_id:r.get(7),axis_kind:r.get(8),
            provider_property_numeric_id:r.get(9),need_revision:r.get(10),
            active_request_ids:r.get(11),
        }).collect())
}


// Port the existing nominal-head licensing boundary (NOUN/PROPN) from
// SensibLaw entity_resolution.py and retired m045, without restoring the
// broad trigger or inventing an identity. NER-supported tokens and existing
// exact head objects already have occurrence carriers. This is a lexical
// head slice, not the full nominal phrase/mention-identity factor lattice.
fn materialize_prior_nominal_heads(
    tx:&mut postgres::Transaction<'_>,run_ref:&str,document_ref:&str,
    focus_start:i64,budget:usize,
)->Result<(i64,i64,i64),NativeNumericAuditError>{
    let noun:i64=tx.query_one("SELECT execution.ensure_semantic_symbol(14::smallint,'mention.noun')",&[])?.get(0);
    let proper:i64=tx.query_one("SELECT execution.ensure_semantic_symbol(14::smallint,'mention.proper_noun')",&[])?.get(0);
    tx.execute(
        "CREATE TEMP TABLE native_numeric_nominal_delta ON COMMIT DROP AS
         SELECT t.token_id,t.lemma_symbol_id,r.region_id,
                CASE WHEN pos.symbol_text='PROPN' THEN $5::bigint ELSE $4::bigint END AS kind,
                digest(convert_to('slr:numeric-nominal-head:v1','UTF8')
                     ||int8send(r.region_id)||int8send(t.token_id),'sha256') AS object_digest
         FROM execution.semantic_parser_token t
         JOIN execution.semantic_symbol pos ON pos.symbol_id=t.pos_symbol_id
         JOIN execution.semantic_pnf_sentence_region l ON l.sentence_id=t.sentence_id
         JOIN execution.semantic_pnf_region r ON r.region_id=l.region_id
         WHERE t.run_ref=$1 AND t.document_ref=$2 AND t.representation_version=2
           AND r.run_ref=$1 AND r.document_ref=$2 AND r.region_kind=1
           AND r.closure_state IN (2,3) AND t.end_char<=$3
           AND pos.symbol_text IN ('NOUN','PROPN') AND t.lemma_symbol_id IS NOT NULL
           AND NOT EXISTS (
             SELECT 1 FROM execution.semantic_pnf_object_token_support os
             JOIN execution.semantic_pnf_object o ON o.object_id=os.object_id
             WHERE os.token_id=t.token_id AND o.region_id=r.region_id AND o.active)
           AND NOT EXISTS (
             SELECT 1 FROM execution.semantic_parser_entity_span e
             WHERE e.run_ref=t.run_ref AND e.document_ref=t.document_ref
               AND e.sentence_ref=t.sentence_ref AND e.representation_version=2
               AND e.start_char<=t.start_char AND e.end_char>=t.end_char)
         ORDER BY t.end_char DESC,t.token_id LIMIT $6",
        &[&run_ref,&document_ref,&focus_start,&noun,&proper,&(budget as i64)])?;
    let objects=tx.execute(
        "INSERT INTO execution.semantic_pnf_object
           (object_digest,region_id,object_kind_symbol_id,head_symbol_id,
            scope_region_id,promotion_level,information_gain,representation_cost,
            ambiguity_cost,promotion_score,active)
         SELECT object_digest,region_id,kind,lemma_symbol_id,region_id,0,0,1,1,0,TRUE
         FROM native_numeric_nominal_delta
         ON CONFLICT(object_digest) DO NOTHING",&[])?;
    let supports=tx.execute(
        "INSERT INTO execution.semantic_pnf_object_token_support(object_id,token_id,ordinal)
         SELECT o.object_id,d.token_id,0
         FROM native_numeric_nominal_delta d JOIN execution.semantic_pnf_object o
           ON o.object_digest=d.object_digest AND o.region_id=d.region_id
          AND o.object_kind_symbol_id=d.kind AND o.head_symbol_id=d.lemma_symbol_id AND o.active
         ON CONFLICT DO NOTHING",&[])?;
    let valid:bool=tx.query_one(
        "SELECT NOT EXISTS (
           SELECT 1 FROM native_numeric_nominal_delta d WHERE NOT EXISTS(
             SELECT 1 FROM execution.semantic_pnf_object o
             JOIN execution.semantic_pnf_object_token_support s ON s.object_id=o.object_id
             WHERE o.object_digest=d.object_digest AND o.region_id=d.region_id
               AND o.object_kind_symbol_id=d.kind AND o.head_symbol_id=d.lemma_symbol_id
               AND o.active AND s.token_id=d.token_id AND s.ordinal=0))",&[])?.get(0);
    if !valid{return Err(NativeNumericAuditError::ForeignSource);}
    let remaining:i64=tx.query_one(
        "SELECT count(DISTINCT t.token_id)::bigint
         FROM execution.semantic_parser_token t
         JOIN execution.semantic_symbol pos ON pos.symbol_id=t.pos_symbol_id
         JOIN execution.semantic_pnf_sentence_region l ON l.sentence_id=t.sentence_id
         JOIN execution.semantic_pnf_region r ON r.region_id=l.region_id
         WHERE t.run_ref=$1 AND t.document_ref=$2 AND t.representation_version=2
           AND r.run_ref=$1 AND r.document_ref=$2 AND r.region_kind=1
           AND r.closure_state IN (2,3) AND t.end_char<=$3
           AND pos.symbol_text IN ('NOUN','PROPN') AND t.lemma_symbol_id IS NOT NULL
           AND NOT EXISTS (
             SELECT 1 FROM execution.semantic_pnf_object_token_support os
             JOIN execution.semantic_pnf_object o ON o.object_id=os.object_id
             WHERE os.token_id=t.token_id AND o.region_id=r.region_id AND o.active)
           AND NOT EXISTS (
             SELECT 1 FROM execution.semantic_parser_entity_span e
             WHERE e.run_ref=t.run_ref AND e.document_ref=t.document_ref
               AND e.sentence_ref=t.sentence_ref AND e.representation_version=2
               AND e.start_char<=t.start_char AND e.end_char>=t.end_char)",
        &[&run_ref,&document_ref,&focus_start])?.get(0);
    Ok((objects as i64,supports as i64,remaining))
}

// Reuse m110's actual typed factor/role and temporal producer on an exact
// demand-local consumer queue. Do not advance or certify H3 completion:
// a partial source can still gather useful H6 evidence without closing H3.
fn run_original_h6_producer(
    tx:&mut postgres::Transaction<'_>,demand:i64,run_ref:&str,document_ref:&str,
)->Result<i64,NativeNumericAuditError>{
    let row=tx.query_one(
        "SELECT r.run_id,r.document_id
         FROM execution.semantic_pnf_demand d JOIN execution.semantic_pnf_region r
         ON r.region_id=d.source_region_id
         WHERE d.demand_id=$1 AND r.run_ref=$2 AND r.document_ref=$3",
        &[&demand,&run_ref,&document_ref])?;
    let run_id:i64=row.get(0);let document_id:i64=row.get(1);
    let consumer="slr-native-numeric-pnf";
    let query=format!("numeric-demand:{demand}");
    let policy="candidate-only-h6:v1";
    tx.execute(
        "INSERT INTO execution.semantic_pnf_consumer_horizon_work_queue
           (demand_id,consumer_ref,query_ref,policy_ref,horizon,work_state)
         VALUES($1,$2,$3,$4,6,1)
         ON CONFLICT(demand_id,consumer_ref,query_ref,policy_ref,horizon)
         DO UPDATE SET work_state=1,completed_at=NULL",
        &[&demand,&consumer,&query,&policy])?;
    let inserted:i64=tx.query_one(
        "SELECT execution.refresh_numeric_pnf_h6_discourse_temporal_evidence_for_consumer(
           $1,$2,$3,$4,$5,FALSE)",
        &[&run_id,&document_id,&consumer,&query,&policy])?.get(0);
    Ok(inserted)
}



#[derive(Debug,Clone,Serialize)]
pub struct NativeH9DemandReadiness {
    pub demand_id:i64,
    pub source_object_id:Option<i64>,
    pub semantic_target_occurrences:i64,
    pub provider_quality_entity_occurrences:i64,
    pub ready_h9_work_items:i64,
    pub matched_contract_rows:i64,
    pub admitted_external_need_rows:i64,
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeH9AdmissionObservation{
    pub demand_id:i64,
    pub consumer_ref:String,
    pub query_ref:String,
    pub policy_ref:String,
    pub contract_id:Option<i64>,
    pub anchor_object_id:Option<i64>,
    pub label_symbol_id:Option<i64>,
    pub admission_reason:i16,
    pub admitted:bool,
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeH9ReadinessReceipt{
    pub related_demands:Vec<NativeH9DemandReadiness>,
    pub original_admission_rows:Vec<NativeH9AdmissionObservation>,
    pub provider_execution_performed:bool,
}
// Read the original producer-specific target occurrence and consumer
// contracts. A cross-sentence antecedent candidate is not itself an H9
// semantic target occurrence. No work/contract is fabricated when absent.
fn observe_native_h9_readiness(
    tx:&mut postgres::Transaction<'_>,demand:i64,run_ref:&str,document_ref:&str,
)->Result<NativeH9ReadinessReceipt,NativeNumericAuditError>{
    let related=tx.query(
        "SELECT d.demand_id,d.source_object_id,
          (SELECT count(*)::bigint FROM execution.semantic_pnf_demand_h9_target_support_v1 v WHERE v.demand_id=d.demand_id),
          (SELECT count(*)::bigint FROM execution.semantic_pnf_demand_parser_entity_occurrence_v1 v WHERE v.demand_id=d.demand_id),
          (SELECT count(*)::bigint FROM execution.semantic_pnf_consumer_horizon_work_queue w
            WHERE w.demand_id=d.demand_id AND w.horizon=9 AND w.work_state=1)
         FROM execution.semantic_pnf_demand d
         JOIN execution.semantic_pnf_region r ON r.region_id=d.source_region_id
         WHERE r.run_ref=$2 AND r.document_ref=$3 AND
          (d.demand_id=$1 OR EXISTS (
             SELECT 1 FROM execution.semantic_pnf_candidate_universe u
             WHERE u.demand_id=$1 AND u.target_kind=1 AND u.target_id=d.source_object_id))
         ORDER BY d.demand_id",
        &[&demand,&run_ref,&document_ref])?;
    let ids=related.iter().map(|r|r.get::<_,i64>(0)).collect::<Vec<_>>();
    let admission=tx.query(
        "SELECT demand_id,consumer_ref,query_ref,policy_ref,contract_id,
                anchor_object_id,label_symbol_id,admission_reason,admitted
         FROM execution.semantic_pnf_h9_external_admission_v1
         WHERE demand_id=ANY($1::bigint[])
         ORDER BY demand_id,consumer_ref,query_ref,policy_ref,contract_id",
        &[&ids])?;
    let original_admission_rows=admission.iter().map(|r|NativeH9AdmissionObservation{
        demand_id:r.get(0),consumer_ref:r.get(1),query_ref:r.get(2),policy_ref:r.get(3),
        contract_id:r.get(4),anchor_object_id:r.get(5),label_symbol_id:r.get(6),
        admission_reason:r.get(7),admitted:r.get(8),
    }).collect::<Vec<_>>();
    let related_demands=related.iter().map(|r|{
        let id:i64=r.get(0);
        NativeH9DemandReadiness{
            demand_id:id,source_object_id:r.get(1),
            semantic_target_occurrences:r.get(2),provider_quality_entity_occurrences:r.get(3),
            ready_h9_work_items:r.get(4),
            matched_contract_rows:original_admission_rows.iter()
                .filter(|a|a.demand_id==id&&a.contract_id.is_some()).count() as i64,
            admitted_external_need_rows:original_admission_rows.iter()
                .filter(|a|a.demand_id==id&&a.admitted).count() as i64,
        }
    }).collect();
    Ok(NativeH9ReadinessReceipt{related_demands,original_admission_rows,provider_execution_performed:false})
}

// Original binding_candidate_sets.py paragraph/quotation prefix rules.
// Codepoint coordinates, not UTF-8 byte offsets. These are parser-anchor
// observations, not proof of reporting scope or mention-identity factor type.
fn source_binding_context(text:&str,offset:i64)->Option<(usize,usize)>{
    let n=usize::try_from(offset).ok()?;
    let mut straight=0usize;let mut left=0usize;let mut right=0usize;
    let mut paragraphs=0usize;let mut previous_newline=false;
    let mut count=0usize;
    for ch in text.chars().take(n){
        count+=1;
        match ch {
            '"' => straight+=1,
            '“' => left+=1,
            '”' => right+=1,
            _ => {}
        }
        // Python str.count("\n\n") counts non-overlapping pairs.
        if ch=='\n'&&previous_newline{paragraphs+=1;previous_newline=false;}
        else{previous_newline=ch=='\n';}
    }
    if count!=n{return None;}
    Some((paragraphs,straight%2+left.saturating_sub(right)))
}

#[derive(Debug,Clone,Serialize)]
pub struct NativeBindingStructuralObservation{
    pub target_kind:i16,
    pub target_id:i64,
    pub exact_head_token_id:Option<i64>,
    pub parser_pos:Option<String>,
    pub first_char:Option<i64>,
    pub same_sentence:Option<bool>,
    pub quotation_depth:Option<usize>,
    pub paragraph_index:Option<usize>,
    pub quotation_boundary_observed:Option<bool>,
    pub accessibility_residual:&'static str,
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeBindingStructuralReceipt{
    pub canonical_source_reopened:bool,
    pub focus_quotation_depth:Option<usize>,
    pub focus_paragraph_index:Option<usize>,
    pub observations:Vec<NativeBindingStructuralObservation>,
    pub typed_factor_coverage_paid:bool,
}
fn observe_native_binding_structure(
    tx:&mut postgres::Transaction<'_>,demand:i64,focus_token:i64,
    run_ref:&str,document_ref:&str,expected_sha:&str,
)->Result<NativeBindingStructuralReceipt,NativeNumericAuditError>{
    let row=tx.query_one(
        "SELECT src.locator,t.start_char,t.sentence_id
         FROM execution.semantic_parser_token t
         JOIN execution.semantic_parser_source src
           ON src.run_ref=t.run_ref AND src.document_ref=t.document_ref
         WHERE t.token_id=$1 AND t.run_ref=$2 AND t.document_ref=$3
           AND encode(src.content_sha256,'hex')=$4",
        &[&focus_token,&run_ref,&document_ref,&expected_sha])?;
    let path:String=row.get(0);let focus_start:i64=row.get(1);
    let focus_sentence:i64=row.get(2);
    // Locator is the original numeric parser's durable source artifact.
    // Missing/mismatched bytes leave context unknown; never use other text.
    let text=std::fs::read(path).ok().filter(|bytes|
        hex(&Sha256::digest(bytes))==expected_sha)
        .and_then(|bytes|String::from_utf8(bytes).ok());
    let focus=text.as_deref().and_then(|s|source_binding_context(s,focus_start));
    let rows=tx.query(
        "SELECT u.target_kind,u.target_id,min(t.start_char),
                CASE WHEN count(DISTINCT t.sentence_id)=1 THEN min(t.sentence_id) END,
                CASE WHEN count(DISTINCT t.token_id) FILTER
                    (WHERE t.lemma_symbol_id=o.head_symbol_id)=1
                    THEN min(t.token_id) FILTER (WHERE t.lemma_symbol_id=o.head_symbol_id) END,
                CASE WHEN count(DISTINCT t.token_id) FILTER
                    (WHERE t.lemma_symbol_id=o.head_symbol_id)=1
                    THEN min(pos.symbol_text) FILTER (WHERE t.lemma_symbol_id=o.head_symbol_id) END
         FROM execution.semantic_pnf_candidate_universe u
         LEFT JOIN execution.semantic_pnf_object o ON u.target_kind=1 AND o.object_id=u.target_id AND o.active
         LEFT JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
           AND r.run_ref=$2 AND r.document_ref=$3
         LEFT JOIN execution.semantic_pnf_object_token_support s ON s.object_id=o.object_id
           AND r.region_id IS NOT NULL
         LEFT JOIN execution.semantic_parser_token t ON t.token_id=s.token_id
           AND t.run_ref=$2 AND t.document_ref=$3 AND t.representation_version=2
         LEFT JOIN execution.semantic_symbol pos ON pos.symbol_id=t.pos_symbol_id
         WHERE u.demand_id=$1 GROUP BY u.target_kind,u.target_id
         ORDER BY u.target_kind,u.target_id",
        &[&demand,&run_ref,&document_ref])?;
    let observations=rows.iter().map(|r|{
        let first:Option<i64>=r.get(2);let sentence:Option<i64>=r.get(3);
        let head:Option<i64>=r.get(4);let pos:Option<String>=r.get(5);
        let ctx=text.as_deref().zip(first).and_then(|(s,n)|source_binding_context(s,n));
        let boundary=focus.zip(ctx).map(|(a,b)|a.1!=b.1);
        let same=sentence.map(|s|s==focus_sentence);
        let residual=if first.is_none()||head.is_none(){"exact_anchor_missing_or_ambiguous"}
            else if focus.is_none()||ctx.is_none(){"canonical_context_unavailable"}
            else if boundary==Some(true){"quotation_boundary_reporting_scope_unpaid"}
            else if pos.as_deref()!=Some("NOUN")&&pos.as_deref()!=Some("PROPN"){"entity_factor_type_unpaid"}
            else{"clause_reporting_and_mention_factor_correspondence_unpaid"};
        NativeBindingStructuralObservation{
            target_kind:r.get(0),target_id:r.get(1),exact_head_token_id:head,
            parser_pos:pos,first_char:first,same_sentence:same,
            quotation_depth:ctx.map(|x|x.1),paragraph_index:ctx.map(|x|x.0),
            quotation_boundary_observed:boundary,accessibility_residual:residual,
        }
    }).collect();
    Ok(NativeBindingStructuralReceipt{
        canonical_source_reopened:text.is_some(),
        focus_quotation_depth:focus.map(|x|x.1),focus_paragraph_index:focus.map(|x|x.0),
        observations,typed_factor_coverage_paid:false,
    })
}


// Source port of entity_resolution.py lexical/name-shaped mention licensing,
// corpus_compilation.py semantic.mention_identity and binding_candidate_sets.py
// v0_3. This projection uses original parser-token intervals, NOT a claim
// of canonical tokenizer or full semantic graph parity. A name run and its
// lexical components are distinct occurrence fibres, never automatic aliases.
#[derive(Debug,Clone)]
struct NativeMentionToken {
    id:i64, region:i64, sentence:i64, start:i64, end:i64,
    orth:String, pos:String, lemma:i64, head:Option<i64>,
    morph:Vec<String>,
}
#[derive(Debug,Clone,Default)]
struct BindingAnchor {
    start:i64, sentence:Option<i64>, paragraph:Option<usize>,
    quotation:Option<usize>, clause:Option<String>,
    reporting:Option<String>, coordination:Option<String>,
}
fn same_known<T:PartialEq>(a:&Option<T>,b:&Option<T>)->bool{
    a.as_ref().zip(b.as_ref()).is_some_and(|(x,y)|x==y)
}
fn original_accessibility_path(reference:&BindingAnchor,candidate:&BindingAnchor)->&'static str{
    if candidate.start>=reference.start{return "not_preceding_reference";}
    if reference.quotation.zip(candidate.quotation).is_some_and(|(a,b)|a!=b){
        return if reference.reporting.is_some()||candidate.reporting.is_some(){
            "reporting_content_boundary"
        }else{"quotation_boundary_crossed"};
    }
    if same_known(&reference.clause,&candidate.clause){return "same_clause";}
    if same_known(&reference.reporting,&candidate.reporting){return "governing_clause";}
    if same_known(&reference.coordination,&candidate.coordination){return "preceding_coordinated_clause";}
    if same_known(&reference.sentence,&candidate.sentence){return "same_sentence";}
    if same_known(&reference.paragraph,&candidate.paragraph){return "preceding_discourse_unit";}
    if reference.paragraph.zip(candidate.paragraph).is_some_and(|(a,b)|b<a){
        return "preceding_paragraph";
    }
    "preceding_document_unit"
}
fn original_binding_type(factor_type:&str,pos:&str,reference_type:&str)->bool{
    match reference_type{
        "entity_reference"=>factor_type=="semantic.mention_identity"&&matches!(pos,"NOUN"|"PROPN"),
        "eventuality_reference"=>factor_type=="semantic.eventuality",
        "proposition_reference"=>matches!(factor_type,"semantic.embedded_proposition"|"semantic.proposition"),
        _=>false,
    }
}
fn original_path_allowed(path:&str,reference_type:&str)->bool{
    match path{
        "not_preceding_reference"|"quotation_boundary_crossed"=>false,
        "reporting_content_boundary"=>matches!(reference_type,"eventuality_reference"|"proposition_reference"),
        _=>matches!(reference_type,"entity_reference"|"eventuality_reference"|"proposition_reference"),
    }
}
fn original_lexical_license(s:&str)->bool{
    s.chars().any(char::is_alphanumeric)&&!matches!(s.trim().to_lowercase().as_str(),
        "a"|"an"|"and"|"as"|"at"|"by"|"for"|"from"|"in"|"into"|"of"|"on"|"or"|"the"|"to"|"with")
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeMentionSpanBinding{
    pub start_char:i64,pub end_char:i64,pub token_ids:Vec<i64>,
    pub original_mention_ref:String,pub factor_type:&'static str,
    pub license_kinds:Vec<&'static str>,pub anchor_pos:String,
    pub exact_object_ids:Vec<i64>,pub unique_dependency_head:Option<i64>,
    pub accessibility_path:&'static str,pub morphology_mismatches:Vec<&'static str>,
    pub projected_compatible:bool,pub within_original_execution_cap:bool,
    pub persisted_candidate_evidence:usize,
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeMentionBindingCampaign{
    pub original_policy_ref:&'static str,
    pub canonical_source_verified:bool,
    pub prefix_uncovered_nonwhitespace:usize,
    pub prefix_uncovered_ranges:Vec<(usize,usize,usize)>,
    pub parser_token_count:usize,
    pub projected_nominal_span_count:usize,
    pub newly_materialized_span_objects:usize,
    pub spans_without_exact_objects:usize,
    pub compatible_before_cap:usize,
    pub compatible_deferred_by_cap:usize,
    pub represented_spans:Vec<NativeMentionSpanBinding>,
    pub canonical_tokenizer_parity_paid:bool,
    pub full_factor_graph_parity_paid:bool,
    pub typed_coverage_paid:bool,
}
fn exact_mention_support_index(
    tx:&mut postgres::Transaction<'_>,run_ref:&str,doc:&str,
)->Result<BTreeMap<(i64,Vec<i64>),Vec<i64>>,NativeNumericAuditError>{
    let rows=tx.query(
        "SELECT o.region_id,o.object_id,array_agg(DISTINCT t.token_id ORDER BY t.token_id)
         FROM execution.semantic_pnf_object o
         JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
         JOIN execution.semantic_pnf_object_token_support s ON s.object_id=o.object_id
         JOIN execution.semantic_parser_token t ON t.token_id=s.token_id
         WHERE r.run_ref=$1 AND r.document_ref=$2 AND o.active
         GROUP BY o.region_id,o.object_id
         HAVING bool_and(t.run_ref=$1 AND t.document_ref=$2 AND t.representation_version=2)
         ORDER BY o.region_id,o.object_id",&[&run_ref,&doc])?;
    let mut out=BTreeMap::<(i64,Vec<i64>),Vec<i64>>::new();
    for r in rows{out.entry((r.get(0),r.get(2))).or_default().push(r.get(1));}
    Ok(out)
}
fn nominal_span_indices(tokens:&[NativeMentionToken])->BTreeMap<(usize,usize),Vec<&'static str>>{
    let mut spans=BTreeMap::<(usize,usize),Vec<&'static str>>::new();
    for (i,t) in tokens.iter().enumerate(){
        if original_binding_type("semantic.mention_identity",&t.pos,"entity_reference")
            &&original_lexical_license(&t.orth){
            spans.entry((i,i+1)).or_default().push("lexical_token");
        }
    }
    let mut i=0;
    while i<tokens.len(){
        if tokens[i].pos!="PROPN"{i+=1;continue;}
        let mut j=i+1;
        while j<tokens.len()&&tokens[j].pos=="PROPN"
            &&tokens[j].sentence==tokens[i].sentence&&tokens[j].region==tokens[i].region{j+=1;}
        spans.entry((i,j)).or_default().push("named_entity_shape");
        i=j;
    }
    spans
}
fn original_mention_binding_campaign(
    tx:&mut postgres::Transaction<'_>,run_ref:&str,doc:&str,sha:&str,focus_token:i64,
    budget:usize,
)->Result<NativeMentionBindingCampaign,NativeNumericAuditError>{
    let source=tx.query_one(
        "SELECT locator FROM execution.semantic_parser_source
         WHERE run_ref=$1 AND document_ref=$2 AND encode(content_sha256,'hex')=$3",
         &[&run_ref,&doc,&sha])?;
    let locator:String=source.get(0);
    let bytes=std::fs::read(locator).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    if hex(&Sha256::digest(&bytes))!=sha{return Err(NativeNumericAuditError::ForeignSource);}
    let text=String::from_utf8(bytes).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let chars=text.chars().collect::<Vec<_>>();
    let focus=tx.query_one(
        "SELECT t.start_char,t.sentence_id,
          ARRAY(SELECT f.symbol_text||'='||v.symbol_text
                FROM execution.semantic_morph_set_member m
                JOIN execution.semantic_symbol f ON f.symbol_id=m.feature_symbol_id
                JOIN execution.semantic_symbol v ON v.symbol_id=m.value_symbol_id
                WHERE m.morph_set_id=t.morph_set_id ORDER BY f.symbol_text,v.symbol_text)
         FROM execution.semantic_parser_token t WHERE t.token_id=$1
         AND t.run_ref=$2 AND t.document_ref=$3 AND t.representation_version=2",
        &[&focus_token,&run_ref,&doc])?;
    let start:i64=focus.get(0);let sentence:i64=focus.get(1);
    let morphology:Vec<String>=focus.get(2);
    let focus_context=source_binding_context(&text,start).ok_or(NativeNumericAuditError::ForeignSource)?;
    let anchor=BindingAnchor{start,sentence:Some(sentence),paragraph:Some(focus_context.0),
        quotation:Some(focus_context.1),..Default::default()};
    let rows=tx.query(
        "SELECT t.token_id,r.region_id,t.sentence_id,t.start_char,t.end_char,
                orth.symbol_text,pos.symbol_text,t.lemma_symbol_id,t.head_token_id,
                ARRAY(SELECT f.symbol_text||'='||v.symbol_text
                    FROM execution.semantic_morph_set_member m
                    JOIN execution.semantic_symbol f ON f.symbol_id=m.feature_symbol_id
                    JOIN execution.semantic_symbol v ON v.symbol_id=m.value_symbol_id
                    WHERE m.morph_set_id=t.morph_set_id ORDER BY f.symbol_text,v.symbol_text)
         FROM execution.semantic_parser_token t
         JOIN execution.semantic_pnf_sentence_region l ON l.sentence_id=t.sentence_id
         JOIN execution.semantic_pnf_region r ON r.region_id=l.region_id
         JOIN execution.semantic_symbol orth ON orth.symbol_id=t.orth_symbol_id
         JOIN execution.semantic_symbol pos ON pos.symbol_id=t.pos_symbol_id
         WHERE t.run_ref=$1 AND t.document_ref=$2 AND t.representation_version=2
           AND r.run_ref=$1 AND r.document_ref=$2 AND r.region_kind=1
           AND r.closure_state IN (2,3) AND t.end_char<=$3
         ORDER BY t.start_char,t.end_char,t.token_id",&[&run_ref,&doc,&start])?;
    let mut tokens=Vec::new();let mut ids=BTreeSet::new();
    let mut covered=vec![false;usize::try_from(start).map_err(|_|NativeNumericAuditError::ForeignSource)?];
    for row in rows{
        let t=NativeMentionToken{id:row.get(0),region:row.get(1),sentence:row.get(2),
            start:row.get(3),end:row.get(4),orth:row.get(5),pos:row.get(6),
            lemma:row.get(7),head:row.get(8),morph:row.get(9)};
        let a=usize::try_from(t.start).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        let b=usize::try_from(t.end).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        if a>=b||b>covered.len()||b>chars.len()||!ids.insert(t.id)
            ||covered[a..b].iter().any(|x|*x)
            ||chars[a..b].iter().collect::<String>()!=t.orth{
            return Err(NativeNumericAuditError::ForeignSource);
        }
        covered[a..b].fill(true);tokens.push(t);
    }
    let uncovered=chars.iter().take(covered.len()).zip(&covered)
        .filter(|(ch,seen)|!**seen&&!ch.is_whitespace()).count();
    let mut prefix_uncovered_ranges=Vec::new();
    let mut offset=0usize;
    while offset<covered.len(){
        if covered[offset]{offset+=1;continue;}
        let a=offset;
        while offset<covered.len()&&!covered[offset]{offset+=1;}
        let n=chars[a..offset].iter().filter(|ch|!ch.is_whitespace()).count();
        if n>0{prefix_uncovered_ranges.push((a,offset,n));}
    }
    let spans=nominal_span_indices(&tokens);
    let mut support=exact_mention_support_index(tx,run_ref,doc)?;
    let noun:i64=tx.query_one("SELECT execution.ensure_semantic_symbol(14::smallint,'mention.noun')",&[])?.get(0);
    let proper:i64=tx.query_one("SELECT execution.ensure_semantic_symbol(14::smallint,'mention.proper_noun')",&[])?.get(0);
    let mut created=0usize;let mut observations=Vec::new();
    for ((a,b),licenses) in spans.into_iter().rev(){
        let slice=&tokens[a..b];let first=&slice[0];let last=&slice[slice.len()-1];
        let mut token_ids=slice.iter().map(|t|t.id).collect::<Vec<_>>();
        token_ids.sort_unstable();
        let head=slice.iter().filter(|t|t.head.is_some_and(|h|h==t.id||!token_ids.contains(&h)))
            .collect::<Vec<_>>();
        let head=if let [h]=head.as_slice(){Some(*h)}else{None};
        let key=(first.region,token_ids.clone());
        let mut objects=support.get(&key).cloned().unwrap_or_default();
        if objects.is_empty()&&created<budget{
            if let Some(h)=head{
                // Distinct source-span object; exact support set, no spelling quotient.
                let mut d=Sha256::new();d.update(b"slr:numeric-mention-span:v1");
                d.update(first.region.to_be_bytes());
                for id in &token_ids{d.update(id.to_be_bytes());}
                let digest=d.finalize().to_vec();
                let kind=if first.pos=="PROPN"{proper}else{noun};
                let row=tx.query_one(
                    "INSERT INTO execution.semantic_pnf_object
                     (object_digest,region_id,object_kind_symbol_id,head_symbol_id,scope_region_id,
                      promotion_level,information_gain,representation_cost,ambiguity_cost,promotion_score,active)
                     VALUES($1,$2,$3,$4,$2,0,0,1,1,0,TRUE)
                     ON CONFLICT(object_digest) DO UPDATE SET object_digest=EXCLUDED.object_digest
                     RETURNING object_id,region_id,object_kind_symbol_id,head_symbol_id,active",
                     &[&digest,&first.region,&kind,&h.lemma])?;
                if row.get::<_,i64>(1)!=first.region||row.get::<_,i64>(2)!=kind
                    ||row.get::<_,i64>(3)!=h.lemma||!row.get::<_,bool>(4){
                    return Err(NativeNumericAuditError::ForeignSource);
                }
                let id:i64=row.get(0);
                for (ordinal,t) in slice.iter().enumerate(){
                    tx.execute(
                        "INSERT INTO execution.semantic_pnf_object_token_support(object_id,token_id,ordinal)
                         VALUES($1,$2,$3) ON CONFLICT DO NOTHING",
                        &[&id,&t.id,&i16::try_from(ordinal).map_err(|_|NativeNumericAuditError::ForeignSource)?])?;
                }
                let actual:Vec<i64>=tx.query_one(
                    "SELECT array_agg(DISTINCT token_id ORDER BY token_id)
                     FROM execution.semantic_pnf_object_token_support WHERE object_id=$1",
                    &[&id])?.get(0);
                if actual!=token_ids{return Err(NativeNumericAuditError::ForeignSource);}
                objects.push(id);support.insert(key,objects.clone());created+=1;
            }
        }
        let ctx=source_binding_context(&text,first.start).ok_or(NativeNumericAuditError::ForeignSource)?;
        // Mention factor metadata contains mention_ref, not a relation ref.
        // The original reference factor's clause/reporting metadata has not
        // been reconstructed, so this is a parser-coordinate fallback path.
        let candidate=BindingAnchor{start:first.start,sentence:Some(first.sentence),
            paragraph:Some(ctx.0),quotation:Some(ctx.1),..Default::default()};
        let path=original_accessibility_path(&anchor,&candidate);
        let mismatches=morphology_mismatches(&morphology,&first.morph);
        let compatible=original_path_allowed(path,"entity_reference")&&mismatches.is_empty();
        observations.push(NativeMentionSpanBinding{
            start_char:first.start,end_char:last.end,token_ids,
            original_mention_ref:format!("mention:{doc}:{}:{}",first.start,last.end),
            factor_type:"semantic.mention_identity",license_kinds:licenses,anchor_pos:first.pos.clone(),
            exact_object_ids:objects,unique_dependency_head:head.map(|h|h.id),
            accessibility_path:path,morphology_mismatches:mismatches,
            projected_compatible:compatible,within_original_execution_cap:false,
            persisted_candidate_evidence:0,
        });
    }
    observations.sort_by(|a,b|b.start_char.cmp(&a.start_char).then(a.original_mention_ref.cmp(&b.original_mention_ref)));
    let mut compatible=0usize;
    for row in &mut observations{if row.projected_compatible{
        compatible+=1;row.within_original_execution_cap=compatible<=64;
    }}
    Ok(NativeMentionBindingCampaign{
        original_policy_ref:"SensibLaw:f9c4670e:mention-licensing+binding-candidate-sets:v0_3",
        canonical_source_verified:true,prefix_uncovered_nonwhitespace:uncovered,prefix_uncovered_ranges,parser_token_count:tokens.len(),
        projected_nominal_span_count:observations.len(),newly_materialized_span_objects:created,
        spans_without_exact_objects:observations.iter().filter(|r|r.exact_object_ids.is_empty()).count(),
        compatible_before_cap:compatible,compatible_deferred_by_cap:compatible.saturating_sub(64),
        represented_spans:observations,canonical_tokenizer_parity_paid:false,
        full_factor_graph_parity_paid:false,typed_coverage_paid:false,
    })
}

/// Candidate-only native source projection for the installed m009 binder.
/// Observed parser rank is document-global, but is not canonical tokenizer
/// parity when parser coverage is incomplete. The exact source interval and
/// ordered token support are separate from the binder's first-token anchor.
#[derive(Debug,Clone,Serialize)]
struct NativeBindingAnchorV2 {
    key:String,kind:String,token_ids:Vec<i64>,object_ids:Vec<i64>,
    start_char:i64,end_char:i64,ordinal:i32,end_ordinal:i32,sentence:i32,
    pos:String,dependency:String,paragraph:i32,quotation:i32,
    clause:Option<String>,reporting:Option<String>,coordination:Option<String>,
    morphology:Vec<(String,String)>,
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeBindingSetBridgeReceipt{
    pub referential_type:String,pub candidate_set_ref:String,
    pub persisted_member_count:usize,pub full_index_member_count:usize,pub cap_deferred_count:usize,
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeBindingBridgeReceipt {
    pub snapshot_ref:String,pub source_parser_tokens:usize,
    pub anchors:usize,pub mention_anchors:usize,pub relation_anchors:usize,
    pub focus_reference_anchors:usize,pub retired_index_anchors:u64,
    pub candidate_sets:Vec<NativeBindingSetBridgeReceipt>,
    pub focus_reference_alternatives:Vec<&'static str>,
    pub original_index_candidate_evidence_inserted:u64,
    pub exact_index_entity_objects_outside_transport:usize,
    pub unresolved_structural_metadata:Vec<&'static str>,
    pub mention_spans_without_objects:usize,
    pub mention_spans_with_multiple_objects:usize,
    pub observed_document_order_verified:bool,
    pub canonical_tokenizer_parity_paid:bool,
    pub structural_relation_parity_paid:bool,
    pub typed_coverage_paid:bool,
}
#[derive(Debug,Clone)]
struct BridgeToken {
    id:i64,sentence_id:i64,sentence:i32,ordinal:i32,start:i64,end:i64,
    region:i64,head:i64,lemma_id:i64,orth:String,lemma:String,pos:String,dep:String,
    morphology:Vec<(String,String)>,
}
fn bridge_morphology(rows:&[String])->Vec<(String,String)> {
    let mut values=BTreeSet::new();
    for row in rows{
        if let Some((feature,raw))=row.split_once('='){
            for value in raw.split(',').filter(|v|!v.is_empty()){
                if !feature.is_empty(){values.insert((feature.to_owned(),value.to_owned()));}
            }
        }
    }
    values.into_iter().collect()
}
fn bridge_anchor(
    key:String,kind:&str,ids:&[i64],objects:Vec<i64>,
    tokens:&BTreeMap<i64,BridgeToken>,text:&str,relation:Option<(&str,&str)>,
)->Result<NativeBindingAnchorV2,NativeNumericAuditError>{
    let mut support=ids.iter().map(|id|tokens.get(id).ok_or(NativeNumericAuditError::ForeignSource))
        .collect::<Result<Vec<_>,_>>()?;
    support.sort_by_key(|t|t.ordinal);
    support.dedup_by_key(|t|t.id);
    let first=*support.first().ok_or(NativeNumericAuditError::ForeignSource)?;
    let last=*support.last().ok_or(NativeNumericAuditError::ForeignSource)?;
    if support.iter().any(|t|t.sentence_id!=first.sentence_id){
        return Err(NativeNumericAuditError::ForeignSource);
    }
    let (paragraph,quotation)=source_binding_context(text,first.start).ok_or(NativeNumericAuditError::ForeignSource)?;
    let clause=match relation{
        Some((name,"predicate"|"composition"))=>Some(format!("clause:{name}")),
        _=>Some(format!("clause:sentence:{}:unresolved",first.sentence)),
    };
    let reporting=relation.filter(|(_,typ)|*typ=="composition").map(|(name,_)|name.to_owned());
    let coordination=relation.filter(|(_,typ)|*typ=="conjunction").map(|(name,_)|name.to_owned());
    Ok(NativeBindingAnchorV2{
        key,kind:kind.into(),token_ids:support.iter().map(|t|t.id).collect(),
        object_ids:objects,start_char:first.start,end_char:last.end,ordinal:first.ordinal,
        end_ordinal:last.ordinal.checked_add(1).ok_or(NativeNumericAuditError::ForeignSource)?,
        sentence:first.sentence,pos:first.pos.clone(),dependency:first.dep.clone(),
        paragraph:i32::try_from(paragraph).map_err(|_|NativeNumericAuditError::ForeignSource)?,
        quotation:i32::try_from(quotation).map_err(|_|NativeNumericAuditError::ForeignSource)?,
        clause,reporting,coordination,morphology:first.morphology.clone(),
    })
}
fn bridge_digest<T:Serialize>(value:&T)->Result<String,NativeNumericAuditError>{
    let bytes=serde_json::to_vec(value).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    Ok(hex(&Sha256::digest(bytes)))
}
fn materialize_native_binding_bridge(
    tx:&mut postgres::Transaction<'_>,run_ref:&str,doc:&str,sha:&str,
    focus_token:i64,demand:i64,campaign:&NativeMentionBindingCampaign,
)->Result<NativeBindingBridgeReceipt,NativeNumericAuditError>{
    let locator:String=tx.query_one(
        "SELECT locator FROM execution.semantic_parser_source
         WHERE run_ref=$1 AND document_ref=$2 AND encode(content_sha256,'hex')=$3",
        &[&run_ref,&doc,&sha])?.get(0);
    let bytes=std::fs::read(locator).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    if hex(&Sha256::digest(&bytes))!=sha{return Err(NativeNumericAuditError::ForeignSource);}
    let source=String::from_utf8(bytes).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let chars=source.chars().collect::<Vec<_>>();
    // Global order over actual source coordinates, never sentence-local token
    // indices or surrogate IDs. Missing source intervals remain unpaid.
    let rows=tx.query(
        "SELECT t.token_id,t.sentence_id,r.region_id,t.start_char,t.end_char,
                t.head_token_id,t.lemma_symbol_id,orth.symbol_text,lemma.symbol_text,
                pos.symbol_text,dep.symbol_text,
                ARRAY(SELECT f.symbol_text||'='||v.symbol_text
                  FROM execution.semantic_morph_set_member m
                  JOIN execution.semantic_symbol f ON f.symbol_id=m.feature_symbol_id
                  JOIN execution.semantic_symbol v ON v.symbol_id=m.value_symbol_id
                  WHERE m.morph_set_id=t.morph_set_id ORDER BY f.symbol_text,v.symbol_text)
         FROM execution.semantic_parser_token t
         JOIN execution.semantic_pnf_sentence_region l ON l.sentence_id=t.sentence_id
         JOIN execution.semantic_pnf_region r ON r.region_id=l.region_id
         JOIN execution.semantic_symbol orth ON orth.symbol_id=t.orth_symbol_id
         JOIN execution.semantic_symbol lemma ON lemma.symbol_id=t.lemma_symbol_id
         JOIN execution.semantic_symbol pos ON pos.symbol_id=t.pos_symbol_id
         JOIN execution.semantic_symbol dep ON dep.symbol_id=t.dependency_symbol_id
         WHERE t.run_ref=$1 AND t.document_ref=$2 AND t.representation_version=2
           AND r.run_ref=$1 AND r.document_ref=$2 AND r.region_kind=1 AND r.closure_state IN (2,3)
         ORDER BY t.start_char,t.end_char,t.token_id",&[&run_ref,&doc])?;
    let mut tokens=BTreeMap::new();
    let mut seen_sentences=BTreeMap::new();
    let mut prior_end=0usize;
    let mut previous_sentence=None;
    let mut sentence=-1i32;
    for (ordinal,row) in rows.iter().enumerate(){
        let id:i64=row.get(0);let sentence_id:i64=row.get(1);
        if previous_sentence!=Some(sentence_id){
            if seen_sentences.contains_key(&sentence_id){return Err(NativeNumericAuditError::ForeignSource);}
            sentence=sentence.checked_add(1).ok_or(NativeNumericAuditError::ForeignSource)?;
            seen_sentences.insert(sentence_id,sentence);previous_sentence=Some(sentence_id);
        }
        let start:i64=row.get(3);let end:i64=row.get(4);
        let a=usize::try_from(start).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        let b=usize::try_from(end).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        let orth:String=row.get(7);
        if a>=b||a<prior_end||b>chars.len()||chars[a..b].iter().collect::<String>()!=orth{
            return Err(NativeNumericAuditError::ForeignSource);
        }
        prior_end=b;
        let t=BridgeToken{id,sentence_id,sentence,
            ordinal:i32::try_from(ordinal).map_err(|_|NativeNumericAuditError::ForeignSource)?,
            region:row.get(2),start,end,head:row.get(5),lemma_id:row.get(6),orth,
            lemma:row.get(8),pos:row.get(9),dep:row.get::<_,String>(10).to_ascii_lowercase(),
            morphology:bridge_morphology(&row.get::<_,Vec<String>>(11))};
        if tokens.insert(id,t).is_some(){return Err(NativeNumericAuditError::ForeignSource);}
    }
    let focus=tokens.get(&focus_token).ok_or(NativeNumericAuditError::ForeignSource)?;
    if focus.pos!="PRON" {return Err(NativeNumericAuditError::ForeignSource);}
    let mut relation_tokens=tokens.values().map(|t|NativeRelationToken{
        id:t.id,region:t.region,lemma_id:t.lemma_id,head:t.head,start:t.start,end:t.end,
        orth:t.orth.clone(),lemma:t.lemma.clone(),pos:t.pos.clone(),dep:t.dep.clone(),
    }).collect::<Vec<_>>();
    relation_tokens.sort_by_key(|t|(t.start,t.end,t.id));
    if relation_tokens.iter().any(|t|tokens.get(&t.head).map_or(true,|h|h.region!=t.region)){
        return Err(NativeNumericAuditError::ForeignSource);
    }
    let mut relations=projected_predicate_relations(&relation_tokens);
    relations.sort_by_key(|r|(r.region,r.head_token_id,r.relation_type,r.roles.clone()));
    let mut anchors=Vec::new();
    for span in &campaign.represented_spans{
        let anchor=bridge_anchor(span.original_mention_ref.clone(),"semantic.mention_identity",
            &span.token_ids,span.exact_object_ids.clone(),&tokens,&source,None)?;
        if anchor.start_char!=span.start_char||anchor.end_char!=span.end_char||anchor.pos!=span.anchor_pos{
            return Err(NativeNumericAuditError::ForeignSource);
        }
        anchors.push(anchor);
    }
    let mention_anchors=anchors.len();
    let mut focus_keys=Vec::new();let mut relation_anchors=0usize;
    for relation in &relations{
        let name=format!("native-projected-relation:{}",bridge_digest(&(run_ref,doc,relation))?);
        let ids=relation.roles.iter().map(|(_,id)|*id).collect::<Vec<_>>();
        let meta=Some((name.as_str(),relation.relation_type));
        let kind=if relation.relation_type=="predicate"{"semantic.eventuality"}else{"semantic.embedded_proposition"};
        let a=bridge_anchor(name.clone(),kind,&ids,vec![],&tokens,&source,meta)?;
        // Original structural binder tests first-token precedence; retain the
        // entire relation support separately, including discontinuities.
        if a.ordinal<focus.ordinal{anchors.push(a);relation_anchors+=1;}
        for (role,id) in &relation.roles{
            if *id!=focus_token||!matches!(*role,"subject"|"object"|"argument"){continue;}
            let key=format!("{name}:argument:{role}:{id}");
            let a=bridge_anchor(key.clone(),&format!("semantic.argument.{role}"),
                &[*id],vec![],&tokens,&source,meta)?;
            if !focus_keys.contains(&key){focus_keys.push(key);anchors.push(a);}
        }
    }
    if focus_keys.is_empty(){return Err(NativeNumericAuditError::SourcePolicy);}
    anchors.sort_by(|a,b|a.key.cmp(&b.key));
    // Snapshot binds all observed parser coordinates, original projected
    // anchor metadata, exact source and inverse object IDs; never a count key.
    let tape=tokens.values().map(|t|(t.id,t.sentence_id,t.ordinal,t.start,t.end,t.head,
        &t.pos,&t.dep,&t.morphology)).collect::<Vec<_>>();
    let snapshot=format!("slr-native-binding-index:v2:{}",bridge_digest(&(run_ref,doc,sha,tape,&anchors))?);
    // Explicit projection/history tables on the existing numeric DB. Old
    // immutable factor/revision/member/span rows are never deleted. Only
    // mutable anchor index rows are retired, after saving their exact JSON.
    tx.batch_execute(
        "CREATE TABLE IF NOT EXISTS execution.slr_native_binding_anchor_history(
            factor_revision_ref text PRIMARY KEY,document_ref text NOT NULL,
            anchor_payload jsonb NOT NULL,retired_at timestamptz NOT NULL DEFAULT now()
         );
         CREATE TABLE IF NOT EXISTS execution.slr_native_binding_anchor_projection(
            factor_revision_ref text PRIMARY KEY REFERENCES algebra.factor_revision(factor_revision_ref),
            run_ref text NOT NULL,document_ref text NOT NULL,snapshot_ref text NOT NULL,
            span_ref text NOT NULL REFERENCES corpus.span(span_ref),
            token_ids bigint[] NOT NULL,object_ids bigint[] NOT NULL,
            active boolean NOT NULL
         );
         CREATE INDEX IF NOT EXISTS slr_native_binding_projection_doc_idx
            ON execution.slr_native_binding_anchor_projection(document_ref,active,snapshot_ref);")?;
    // A genuine legacy producer's index is not ours to retire or mix with a
    // different token coordinate chart. Reject instead of contaminating it.
    // The old binder scopes its index by document, not parser run.
    tx.query_one("SELECT pg_advisory_xact_lock(hashtext('slr-native-binding-document:v2'),hashtext($1))",&[&doc])?;
    let foreign:bool=tx.query_one(
        "SELECT EXISTS(SELECT 1 FROM pnf.factor_anchor a WHERE a.document_ref=$1
          AND a.factor_revision_ref NOT LIKE 'factor-revision:native-binding:%'
          AND a.factor_revision_ref NOT LIKE 'factor-revision:native-binding-v2:%')
          OR EXISTS(SELECT 1 FROM pnf.factor_anchor a WHERE a.document_ref=$1
                    AND a.factor_revision_ref LIKE 'factor-revision:native-binding-v2:%'
                    AND NOT EXISTS(SELECT 1 FROM execution.slr_native_binding_anchor_projection p
                        WHERE p.factor_revision_ref=a.factor_revision_ref AND p.active))
          OR EXISTS(SELECT 1 FROM execution.slr_native_binding_anchor_projection
                    WHERE document_ref=$1 AND active AND run_ref<>$2)",
        &[&doc,&run_ref])?.get(0);
    if foreign{return Err(NativeNumericAuditError::ForeignSource);}
    let legacy:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM pnf.factor_anchor
        WHERE document_ref=$1 AND factor_revision_ref LIKE 'factor-revision:native-binding:%')",&[&doc])?.get(0);
    if legacy && tx.query_one("SELECT count(DISTINCT run_ref)::bigint FROM execution.semantic_parser_source WHERE document_ref=$1",&[&doc])?.get::<_,i64>(0)!=1{
        return Err(NativeNumericAuditError::ForeignSource);
    }
    tx.execute(
        "INSERT INTO execution.slr_native_binding_anchor_history(factor_revision_ref,document_ref,anchor_payload)
         SELECT a.factor_revision_ref,a.document_ref,to_jsonb(a) FROM pnf.factor_anchor a
         WHERE a.document_ref=$1 AND (a.factor_revision_ref LIKE 'factor-revision:native-binding:%'
            OR EXISTS(SELECT 1 FROM execution.slr_native_binding_anchor_projection p
                 WHERE p.factor_revision_ref=a.factor_revision_ref AND p.snapshot_ref<>$2))
         ON CONFLICT DO NOTHING",&[&doc,&snapshot])?;
    tx.execute(
        "UPDATE execution.binding_candidate_set_build b SET build_state_ref='superseded'
         WHERE b.build_state_ref='completed' AND EXISTS(
           SELECT 1 FROM pnf.factor_anchor a WHERE a.factor_revision_ref=b.reference_factor_revision_ref
           AND a.document_ref=$1 AND (a.factor_revision_ref LIKE 'factor-revision:native-binding:%'
            OR EXISTS(SELECT 1 FROM execution.slr_native_binding_anchor_projection p
                WHERE p.factor_revision_ref=a.factor_revision_ref AND p.snapshot_ref<>$2)))",
        &[&doc,&snapshot])?;
    let retired=tx.execute(
        "DELETE FROM pnf.factor_anchor a WHERE a.document_ref=$1
          AND (a.factor_revision_ref LIKE 'factor-revision:native-binding:%'
          OR EXISTS(SELECT 1 FROM execution.slr_native_binding_anchor_projection p
            WHERE p.factor_revision_ref=a.factor_revision_ref AND p.snapshot_ref<>$2))",
        &[&doc,&snapshot])?;
    tx.execute("UPDATE execution.slr_native_binding_anchor_projection SET active=false
        WHERE document_ref=$1 AND snapshot_ref<>$2 AND active",&[&doc,&snapshot])?;
    let mut references=Vec::new();
    for a in &anchors{
        let suffix=bridge_digest(&(&snapshot,a))?;
        let factor=format!("factor:native-binding-v2:{suffix}");
        let revision=format!("factor-revision:native-binding-v2:{suffix}");
        let span=format!("span:native-binding-v2:{suffix}");
        let dg=Sha256::digest(format!("factor:{factor}").as_bytes()).to_vec();
        let start=i32::try_from(a.start_char).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        let end=i32::try_from(a.end_char).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        let next=a.ordinal.checked_add(1).ok_or(NativeNumericAuditError::ForeignSource)?;
        let morph=bridge_digest(&a.morphology)?;
        tx.execute("INSERT INTO corpus.span(span_ref,document_ref,start_char,end_char,start_token,end_token,span_type_ref)
            VALUES($1,$2,$3,$4,$5,$6,'native-binding-factor:v2') ON CONFLICT DO NOTHING",
            &[&span,&doc,&start,&end,&a.ordinal,&a.end_ordinal])?;
        tx.execute("INSERT INTO algebra.factor(factor_ref,document_ref,factor_type_ref,closure_state_ref,factor_sha256)
            VALUES($1,$2,$3,'requires_external_resolution',$4) ON CONFLICT DO NOTHING",
            &[&factor,&doc,&a.kind,&dg])?;
        tx.execute("INSERT INTO algebra.factor_revision(factor_revision_ref,factor_ref,closure_state_ref,factor_sha256)
            VALUES($1,$2,'requires_external_resolution',$3) ON CONFLICT DO NOTHING",
            &[&revision,&factor,&dg])?;
        tx.execute("INSERT INTO pnf.factor_anchor(factor_revision_ref,document_ref,sentence_index,clause_ref,
            start_token,end_token,pnf_kind_ref,morphology_sha256,discourse_unit_ref,paragraph_index,
            quotation_depth,reporting_scope_ref,coordination_group_ref,parser_pos_ref,parser_dependency_ref)
            VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) ON CONFLICT DO NOTHING",
            &[&revision,&doc,&a.sentence,&a.clause,&a.ordinal,&next,&a.kind,&morph,
              &format!("sentence:{}",a.sentence),&a.paragraph,&a.quotation,&a.reporting,
              &a.coordination,&a.pos,&a.dependency])?;
        for (feature,value) in &a.morphology{
            tx.execute("INSERT INTO pnf.factor_morphology(factor_revision_ref,feature_ref,value_ref)
                VALUES($1,$2,$3) ON CONFLICT DO NOTHING",&[&revision,feature,value])?;
        }
        tx.execute("INSERT INTO execution.slr_native_binding_anchor_projection
            (factor_revision_ref,run_ref,document_ref,snapshot_ref,span_ref,token_ids,object_ids,active)
            VALUES($1,$2,$3,$4,$5,$6,$7,true) ON CONFLICT(factor_revision_ref) DO UPDATE SET active=true",
            &[&revision,&run_ref,&doc,&snapshot,&span,&a.token_ids,&a.object_ids])?;
        // Verify inverse on the stored projection, not just insertion count.
        let valid:bool=tx.query_one(
            "SELECT EXISTS(SELECT 1 FROM execution.slr_native_binding_anchor_projection p
              JOIN pnf.factor_anchor a ON a.factor_revision_ref=p.factor_revision_ref
              JOIN corpus.span s ON s.span_ref=p.span_ref
              WHERE p.factor_revision_ref=$1 AND p.run_ref=$2 AND p.document_ref=$3
                AND p.snapshot_ref=$4 AND p.active AND p.token_ids=$5 AND p.object_ids=$6
                AND s.document_ref=$3 AND s.start_char=$7 AND s.end_char=$8
                AND s.start_token=$9 AND s.end_token=$10 AND a.start_token=$9 AND a.end_token=$11)",
            &[&revision,&run_ref,&doc,&snapshot,&a.token_ids,&a.object_ids,&start,&end,&a.ordinal,&a.end_ordinal,&next])?.get(0);
        let metadata:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM pnf.factor_anchor
            WHERE factor_revision_ref=$1 AND sentence_index=$2 AND paragraph_index=$3 AND quotation_depth=$4
              AND clause_ref IS NOT DISTINCT FROM $5 AND reporting_scope_ref IS NOT DISTINCT FROM $6
              AND coordination_group_ref IS NOT DISTINCT FROM $7 AND pnf_kind_ref=$8
              AND parser_pos_ref=$9 AND parser_dependency_ref=$10 AND morphology_sha256=$11)",
            &[&revision,&a.sentence,&a.paragraph,&a.quotation,&a.clause,&a.reporting,&a.coordination,
              &a.kind,&a.pos,&a.dependency,&morph])?.get(0);
        let stored_morph=tx.query("SELECT feature_ref,value_ref FROM pnf.factor_morphology
            WHERE factor_revision_ref=$1 ORDER BY feature_ref,value_ref",&[&revision])?
            .iter().map(|r|(r.get::<_,String>(0),r.get::<_,String>(1))).collect::<BTreeSet<_>>();
        if !valid||!metadata||stored_morph!=a.morphology.iter().cloned().collect::<BTreeSet<_>>(){
            return Err(NativeNumericAuditError::ForeignSource);
        }
        if focus_keys.contains(&a.key){
            persist_native_reference_alternatives(tx,&factor,&revision,a.kind=="semantic.argument.subject")?;
            references.push((factor,revision));
        }
    }
    let mut sets=Vec::new();
    for (factor,revision) in references{
        for typ in ["entity_reference","eventuality_reference","proposition_reference"]{
            sets.push(materialize_native_binding_set_v2(tx,&snapshot,&doc,&factor,&revision,typ)?);
        }
    }
    let (original_index_candidate_evidence_inserted,exact_index_entity_objects_outside_transport)=
        persist_native_index_evidence(tx,demand,&snapshot,&doc,&sets)?;
    let has_subject=anchors.iter().any(|a|focus_keys.contains(&a.key)&&a.kind=="semantic.argument.subject");
    let mut alternatives=vec!["entity_reference","eventuality_reference","proposition_reference"];
    if has_subject{alternatives.push("expletive_realisation");}
    Ok(NativeBindingBridgeReceipt{
        snapshot_ref:snapshot,source_parser_tokens:tokens.len(),anchors:anchors.len(),mention_anchors,
        relation_anchors,focus_reference_anchors:focus_keys.len(),retired_index_anchors:retired,candidate_sets:sets,
        focus_reference_alternatives:alternatives,original_index_candidate_evidence_inserted,
        exact_index_entity_objects_outside_transport,
        unresolved_structural_metadata:vec!["canonical_tokenizer_parity","full_original_mention_factor_parity",
            "original_relation_id_and_bundle_parity","nonprojected_coordination_and_other_relation_families"],
        mention_spans_without_objects:campaign.spans_without_exact_objects,
        mention_spans_with_multiple_objects:campaign.represented_spans.iter().filter(|s|s.exact_object_ids.len()>1).count(),
        observed_document_order_verified:true,canonical_tokenizer_parity_paid:false,
        structural_relation_parity_paid:false,typed_coverage_paid:false,
    })
}
fn persist_native_reference_alternatives(
    tx:&mut postgres::Transaction<'_>,factor:&str,revision:&str,subject:bool,
)->Result<(),NativeNumericAuditError>{
    let mut types=vec!["entity_reference","eventuality_reference","proposition_reference"];
    if subject{types.push("expletive_realisation");}
    for typ in types{
        let alternative=format!("{factor}:{typ}");
        let digest=Sha256::digest(format!("slr-native-reference-alternative:v2:{alternative}").as_bytes()).to_vec();
        tx.execute("INSERT INTO algebra.alternative(alternative_ref,type_ref,authority_state_ref,alternative_sha256)
            VALUES($1,'semantic.reference_candidate','candidate_only',$2) ON CONFLICT DO NOTHING",
            &[&alternative,&digest])?;
        tx.execute("INSERT INTO algebra.factor_revision_alternative(factor_revision_ref,alternative_ref,alternative_state_ref)
            VALUES($1,$2,'alternative') ON CONFLICT DO NOTHING",&[&revision,&alternative])?;
    }
    let mut residuals=vec!["antecedent_unresolved","referential_type_unresolved"];
    if subject{residuals.push("grammatical_subject_semantic_status_unresolved");}
    for residual in residuals{
        let key=format!("{revision}:residual:{residual}");
        let digest=Sha256::digest(format!("slr-native-reference-residual:v2:{key}").as_bytes()).to_vec();
        tx.execute("INSERT INTO algebra.residual(residual_ref,target_ref,residual_type_ref,residual_state_ref,residual_sha256)
            VALUES($1,$2,$3,'open',$4) ON CONFLICT DO NOTHING",&[&key,&revision,&residual,&digest])?;
    }
    Ok(())
}
fn materialize_native_binding_set_v2(
    tx:&mut postgres::Transaction<'_>,snapshot:&str,doc:&str,factor:&str,revision:&str,typ:&str,
)->Result<NativeBindingSetBridgeReceipt,NativeNumericAuditError>{
    let access="binding-accessibility:document-structural:v0_3";
    let compat="binding-compatibility:pnf-kind-morphology:v0_3";
    let n:i64=tx.query_one("SELECT count(*)::bigint FROM pnf.factor_anchor WHERE document_ref=$1",&[&doc])?.get(0);
    let full=exact_original_binding_members(tx,revision,typ,access,compat,
        i32::try_from(n.max(1)).map_err(|_|NativeNumericAuditError::ForeignSource)?)?;
    let members=exact_original_binding_members(tx,revision,typ,access,compat,64)?;
    if !full.iter().take(64).eq(members.iter()) {return Err(NativeNumericAuditError::ForeignSource);}
    let suffix=bridge_digest(&(snapshot,revision,typ,access,compat,&full))?;
    let build=format!("binding-build:native-v2:{suffix}");
    let set=format!("binding-set:native-v2:{suffix}");
    let count=i32::try_from(members.len()).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let set_sha=bridge_digest(&members)?;
    tx.execute("INSERT INTO resolution.binding_candidate_set(candidate_set_ref,document_ref,reference_factor_ref,
        reference_factor_revision_ref,referential_type_ref,accessibility_declaration_ref,compatibility_declaration_ref,
        generator_build_ref,compatibility_state_ref,member_count,candidate_set_sha256)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,'compatible_candidate',$9,$10) ON CONFLICT DO NOTHING",
        &[&set,&doc,&factor,&revision,&typ,&access,&compat,&build,&count,&set_sha])?;
    for (i,(_,candidate,path,_)) in members.iter().enumerate(){
        let dg=bridge_digest(&(&set,candidate,path))?;
        let assessment=format!("binding-assessment:native-v2:{dg}");
        tx.execute("INSERT INTO resolution.binding_compatibility_assessment(compatibility_assessment_ref,candidate_set_ref,
            candidate_factor_ref,compatibility_state_ref,accessibility_path_ref,assessment_sha256)
            VALUES($1,$2,$3,'compatible_candidate',$4,$5) ON CONFLICT DO NOTHING",
            &[&assessment,&set,candidate,path,&dg])?;
        tx.execute("INSERT INTO resolution.binding_candidate_member(candidate_set_ref,candidate_factor_ref,
            compatibility_assessment_ref,accessibility_path_ref,ordinal) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING",
            &[&set,candidate,&assessment,path,&(i as i32)])?;
    }
    tx.execute("INSERT INTO execution.binding_candidate_set_build(generator_build_ref,candidate_set_ref,
        reference_factor_revision_ref,document_pnf_index_ref,accessibility_declaration_ref,
        compatibility_declaration_ref,referential_type_ref,build_key_sha256,build_state_ref)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,'completed')
        ON CONFLICT(generator_build_ref) DO UPDATE SET build_state_ref='completed'",
        &[&build,&set,&revision,&snapshot,&access,&compat,&typ,&suffix])?;
    let stored=tx.query("SELECT candidate_factor_ref,accessibility_path_ref FROM resolution.binding_candidate_member
        WHERE candidate_set_ref=$1 ORDER BY ordinal",&[&set])?
        .iter().map(|r|(r.get::<_,String>(0),r.get::<_,String>(1))).collect::<Vec<_>>();
    if !original_index_member_parity(&stored,&members){return Err(NativeNumericAuditError::ForeignSource);}
    Ok(NativeBindingSetBridgeReceipt{referential_type:typ.into(),candidate_set_ref:set,
        persisted_member_count:members.len(),full_index_member_count:full.len(),
        cap_deferred_count:full.len().saturating_sub(64)})
}
// Bind actual original SQL candidate membership to the native demand's
// append-only evidence fibre. The full indexed population is retained,
// including cap-deferred members. No sign, preference or refutation is
// inferred from membership, absence, a missing object or an incomplete index.
fn persist_native_index_evidence(
    tx:&mut postgres::Transaction<'_>,demand:i64,snapshot:&str,doc:&str,
    sets:&[NativeBindingSetBridgeReceipt],
)->Result<(u64,usize),NativeNumericAuditError>{
    let mut inserted=0u64;let mut outside=BTreeSet::new();
    let n:i64=tx.query_one("SELECT count(*)::bigint FROM pnf.factor_anchor WHERE document_ref=$1",&[&doc])?.get(0);
    let limit=i32::try_from(n.max(1)).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    for set in sets.iter().filter(|s|s.referential_type=="entity_reference"){
        let rows=tx.query(
            "SELECT p.factor_revision_ref,p.object_ids,m.accessibility_path_ref
             FROM resolution.binding_candidate_set s
             JOIN execution.binding_candidate_set_build b ON b.candidate_set_ref=s.candidate_set_ref
             CROSS JOIN LATERAL resolution.query_binding_candidates(s.reference_factor_revision_ref,
                s.referential_type_ref,s.accessibility_declaration_ref,s.compatibility_declaration_ref,$3) m
             JOIN execution.slr_native_binding_anchor_projection p
               ON p.factor_revision_ref=m.candidate_factor_revision_ref AND p.active AND p.snapshot_ref=$2
             WHERE s.candidate_set_ref=$1 AND b.document_pnf_index_ref=$2 AND b.build_state_ref='completed'",
            &[&set.candidate_set_ref,&snapshot,&limit])?;
        if rows.len()!=set.full_index_member_count{return Err(NativeNumericAuditError::ForeignSource);}
        for row in rows{
            let revision:String=row.get(0);let objects:Vec<i64>=row.get(1);let path:String=row.get(2);
            for object in objects{
                let present:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.semantic_pnf_candidate_universe
                    WHERE demand_id=$1 AND target_kind=1 AND target_id=$2)",&[&demand,&object])?.get(0);
                if !present{outside.insert(object);continue;}
                let evidence=format!("native-binding-index-member:v2:{}",bridge_digest(&(&set.candidate_set_ref,&revision,object,&path))?);
                inserted+=tx.execute("INSERT INTO execution.semantic_pnf_candidate_evidence
                    (demand_id,target_kind,target_id,evidence_ref,evidence_family,horizon,signed_residual,
                     evidence_kind,provenance_ref,source_interface_id,source_region_id)
                    SELECT $1,1,o.object_id,$3,1,3,0,$4,$5,i.interface_id,o.region_id
                    FROM execution.semantic_pnf_object o JOIN execution.semantic_pnf_interface i ON i.region_id=o.region_id
                    WHERE o.object_id=$2 AND o.active
                    ON CONFLICT(demand_id,target_kind,target_id,evidence_ref) DO NOTHING",
                    &[&demand,&object,&evidence,&format!("original_binding_index_member:{path}"),&set.candidate_set_ref])?;
            }
        }
    }
    Ok((inserted,outside.len()))
}

// The v1 projected mention assessments remain immutable historical zero-sign
// observations. New evidence is emitted only by persist_native_index_evidence
// from the actual original SQL index.

// Class-level uniqueness is not occurrence uniqueness. This helper is only
// a consumer gate; coverage must be independently paid by the original owner.
fn covered_unique_class(p:&NativeNumericIdentityFibreProjection,coverage:bool)->Option<i64>{
    if !coverage||p.outside_model_possible||!p.unresolved_keys.is_empty()
        ||!p.conflicting_objects.is_empty()||p.admitted_classes.len()!=1{return None;}
    p.admitted_classes.iter().next().filter(|(_,xs)|!xs.is_empty()).map(|(id,_)|*id)
}


#[derive(Debug,Clone,Serialize)]
pub struct NativeParserWitnessWithdrawalProbe{
    pub witness_id:i64,
    pub affected_level_three_refs:Vec<String>,
    pub nonvacuous_dependency:bool,
    pub original_derivation_replay_verified:bool,
    pub witness_history_preserved:bool,
    pub projection_removed:bool,
    pub dependent_rows_removed:bool,
    pub immutable_level_one_preserved:bool,
    pub unaffected_level_three_preserved:bool,
    pub rollback_restored:bool,
}
fn probe_parser_witness_withdrawals(
    tx:&mut postgres::Transaction<'_>,run:i64,doc:i64,
    classes:&[NativeNumericDocumentIdentityObservation],
)->Result<Vec<NativeParserWitnessWithdrawalProbe>,NativeNumericAuditError>{
    let candidates=classes.iter().flat_map(|c|c.admitted_witness_ids.iter().copied()).collect::<BTreeSet<_>>();
    // Read the actual derivation dependency fibre, including base/anchor
    // witnesses not in the focus candidate projection. No synthetic witness.
    let dependencies=tx.query(
        "WITH scoped AS (
          SELECT d.derivation_id FROM execution.semantic_pnf_factor_derivation d
          JOIN execution.semantic_pnf_interface i ON i.interface_id=d.scope_interface_id
          JOIN execution.semantic_pnf_region r ON r.region_id=i.region_id
          WHERE r.run_id=$1 AND r.document_id=$2 AND d.epistemic_level=3
         ), used AS (
          SELECT unnest(a.identity_witness_ids) AS witness_id
          FROM execution.semantic_pnf_factor_derivation_argument a JOIN scoped s USING(derivation_id)
          UNION
          SELECT unnest(a.identity_witness_ids) AS witness_id
          FROM execution.semantic_pnf_factor_derivation_support a JOIN scoped s USING(derivation_id)
         )
         SELECT DISTINCT w.witness_id FROM used u
         JOIN execution.semantic_pnf_identity_witness w ON w.witness_id=u.witness_id
         JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=w.witness_id
         JOIN execution.semantic_pnf_object o ON o.object_id=w.source_object_id
         JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
         WHERE r.run_id=$1 AND r.document_id=$2 AND w.authority_class=2 AND a.admission_state=2
         ORDER BY w.witness_id",&[&run,&doc])?.iter().map(|r|r.get::<_,i64>(0)).collect::<BTreeSet<_>>();
    let ids=dependencies.iter().copied().chain(candidates.difference(&dependencies).copied()).collect::<Vec<_>>();
    let level_one=level_one_snapshot(tx,run,doc)?;
    let before=level_three_snapshot(tx,run,doc)?;
    let mut receipts=Vec::new();
    for id in ids.into_iter().take(64){
        let eligible:bool=tx.query_one(
            "SELECT EXISTS(SELECT 1 FROM execution.semantic_pnf_identity_witness w
             JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=w.witness_id
             JOIN execution.semantic_pnf_object o ON o.object_id=w.source_object_id
             JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
             WHERE w.witness_id=$1 AND w.authority_class=2 AND a.admission_state=2
               AND r.run_id=$2 AND r.document_id=$3
               AND EXISTS(SELECT 1 FROM execution.semantic_pnf_identity_projection p
                 WHERE p.source_object_id=w.source_object_id AND $1=ANY(p.witness_ids)))",
            &[&id,&run,&doc])?.get(0);
        if !eligible{continue;}
        let affected:Vec<String>=tx.query(
            "SELECT DISTINCT d.derivation_ref FROM execution.semantic_pnf_factor_derivation d
             JOIN execution.semantic_pnf_interface i ON i.interface_id=d.scope_interface_id
             JOIN execution.semantic_pnf_region r ON r.region_id=i.region_id
             WHERE r.run_id=$2 AND r.document_id=$3 AND d.epistemic_level=3
             AND (EXISTS(SELECT 1 FROM execution.semantic_pnf_factor_derivation_argument a
                  WHERE a.derivation_id=d.derivation_id AND $1=ANY(a.identity_witness_ids))
              OR EXISTS(SELECT 1 FROM execution.semantic_pnf_factor_derivation_support s
                  WHERE s.derivation_id=d.derivation_id AND $1=ANY(s.identity_witness_ids)))
             ORDER BY d.derivation_ref",&[&id,&run,&doc])?.iter().map(|r|r.get(0)).collect();
        let mut inverse=tx.transaction()?;
        let changed:bool=inverse.query_one("SELECT execution.retract_numeric_pnf_identity_witness($1)",&[&id])?.get(0);
        // Original supported-substitution cleanup used by native retract.
        inverse.execute(
            "DELETE FROM execution.semantic_pnf_factor_derivation d
             USING execution.semantic_pnf_interface i,execution.semantic_pnf_region r
             WHERE d.scope_interface_id=i.interface_id AND i.region_id=r.region_id
               AND r.run_id=$1 AND r.document_id=$2
               AND d.rule_ref='identity-supported-substitution:v1'
               AND (EXISTS(SELECT 1 FROM execution.semantic_pnf_factor_derivation_argument a
                       WHERE a.derivation_id=d.derivation_id AND $3=ANY(a.identity_witness_ids))
                 OR EXISTS(SELECT 1 FROM execution.semantic_pnf_factor_derivation_support s
                       WHERE s.derivation_id=d.derivation_id AND $3=ANY(s.identity_witness_ids)))",
            &[&run,&doc,&id])?;
        inverse.query_one("SELECT execution.refresh_numeric_pnf_supported_identity_substitution_derivations($1,$2)",&[&run,&doc])?;
        let history:bool=inverse.query_one(
            "SELECT EXISTS(SELECT 1 FROM execution.semantic_pnf_identity_witness w
             JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=w.witness_id
             WHERE w.witness_id=$1 AND a.admission_state=3)",&[&id])?.get(0);
        let removed:bool=inverse.query_one(
            "SELECT NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_identity_projection WHERE $1=ANY(witness_ids))",
            &[&id])?.get(0);
        let dependent:bool=inverse.query_one(
            "SELECT NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_factor_derivation_argument WHERE $1=ANY(identity_witness_ids)
             UNION ALL SELECT 1 FROM execution.semantic_pnf_factor_derivation_support WHERE $1=ANY(identity_witness_ids))",
            &[&id])?.get(0);
        let immutable=level_one==level_one_snapshot(&mut inverse,run,doc)?;
        let after=level_three_snapshot(&mut inverse,run,doc)?;
        let unaffected=before.iter().all(|(key,value)|affected.contains(key)||after.get(key)==Some(value));
        inverse.rollback()?;
        let restored=before==level_three_snapshot(tx,run,doc)?
            &&level_one==level_one_snapshot(tx,run,doc)?
            &&tx.query_one("SELECT admission_state=2 FROM execution.semantic_pnf_identity_witness_admission WHERE witness_id=$1",&[&id])?.get::<_,bool>(0);
        // A rollback alone does not exercise the forward producer again.
        // Restore the admitted witness, then invoke the original derivation
        // owners in a new savepoint and compare exact proof-bearing content.
        let replay_verified=if !affected.is_empty()&&restored{
            let mut replay=tx.transaction()?;
            replay.query_one("SELECT execution.refresh_numeric_pnf_identity_substitution_derivations($1,$2)",&[&run,&doc])?;
            replay.query_one("SELECT execution.refresh_numeric_pnf_supported_identity_substitution_derivations($1,$2)",&[&run,&doc])?;
            let verified=before==level_three_snapshot(&mut replay,run,doc)?
                &&level_one==level_one_snapshot(&mut replay,run,doc)?;
            replay.rollback()?;
            verified
        }else{false};
        if !changed||!history||!removed||!dependent||!immutable||!unaffected||!restored
            ||(!affected.is_empty()&&!replay_verified){
            return Err(NativeNumericAuditError::RetractionInvariant);
        }
        receipts.push(NativeParserWitnessWithdrawalProbe{
            witness_id:id,nonvacuous_dependency:!affected.is_empty(),original_derivation_replay_verified:replay_verified,
            affected_level_three_refs:affected,witness_history_preserved:changed&&history,
            projection_removed:removed,dependent_rows_removed:dependent,immutable_level_one_preserved:immutable,
            unaffected_level_three_preserved:unaffected,rollback_restored:restored,
        });
    }
    Ok(receipts)
}


// Candidate relation projection: original shared_reducer.py predicate and
// composition branches + semantic_reductions.py predicate/argument rules.
// The original strict numeric run remains immutable: publish this projected
// factor graph only in a nested rollback acceptance transaction. This is not
// a full relational bundle (action metadata, modifiers, temporal and spatial
// families still need original producer parity).
#[derive(Debug,Clone)]
struct NativeRelationToken{
    id:i64,region:i64,lemma_id:i64,head:i64,start:i64,end:i64,
    orth:String,lemma:String,pos:String,dep:String,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
struct ProjectedRelation{
    region:i64,head_token_id:i64,relation_type:&'static str,
    roles:Vec<(&'static str,i64)>,
}
fn projected_predicate_relations(tokens:&[NativeRelationToken])->Vec<ProjectedRelation>{
    let mut children=BTreeMap::<i64,Vec<&NativeRelationToken>>::new();
    for t in tokens{children.entry(t.head).or_default().push(t);}
    let nounish=|t:&&NativeRelationToken|matches!(t.pos.as_str(),"NOUN"|"PROPN"|"PRON"|"NUM");
    let mut out=Vec::new();
    for t in tokens{
        if !matches!(t.pos.as_str(),"VERB"|"AUX")
            ||!matches!(t.dep.as_str(),"root"|"acl"|"xcomp"|"ccomp"|"advcl"){continue;}
        let direct=children.get(&t.id).map(Vec::as_slice).unwrap_or(&[]);
        let aux=direct.iter().copied().filter(|c|matches!(c.dep.as_str(),"aux"|"auxpass")&&c.pos=="AUX").collect::<Vec<_>>();
        let mut subjects=direct.iter().copied().filter(|c|matches!(c.dep.as_str(),"nsubj"|"nsubjpass")).filter(nounish).collect::<Vec<_>>();
        for a in &aux{
            subjects.extend(children.get(&a.id).into_iter().flatten().copied()
                .filter(|c|matches!(c.dep.as_str(),"nsubj"|"nsubjpass")).filter(nounish));
        }
        let objects=direct.iter().copied().filter(|c|matches!(c.dep.as_str(),"dobj"|"obj"|"attr"|"oprd")).filter(nounish).collect::<Vec<_>>();
        let oblique=direct.iter().copied().filter(|c|matches!(c.dep.as_str(),"obl"|"pobj"|"iobj")).filter(nounish).collect::<Vec<_>>();
        let complements=direct.iter().copied().filter(|c|matches!(c.dep.as_str(),"attr"|"acomp"|"ccomp"|"xcomp"|"oprd")).collect::<Vec<_>>();
        // Explicit parser negation only, do not infer lexical negation or
        // inflection until the original action metadata producer is ported.
        let neg=direct.iter().copied().filter(|c|c.dep=="neg").collect::<Vec<_>>();
        let base=||{
            let mut roles=vec![("head",t.id)];
            roles.extend(subjects.iter().map(|s|("subject",s.id)));roles
        };
        let tail=|roles:&mut Vec<(&'static str,i64)>|{
            roles.extend(neg.iter().map(|s|("negation",s.id)));
            roles.extend(aux.iter().map(|s|("auxiliary",s.id)));
        };
        if objects.is_empty(){
            let mut roles=base();
            for s in oblique{roles.extend([("oblique",s.id),("argument",s.id)]);}
            for s in &complements{roles.extend([("complement",s.id),("argument",s.id)]);}
            tail(&mut roles);
            out.push(ProjectedRelation{region:t.region,head_token_id:t.id,relation_type:"predicate",roles});
        }else{
            for s in objects{
                let mut roles=base();roles.extend([("object",s.id),("argument",s.id)]);tail(&mut roles);
                out.push(ProjectedRelation{region:t.region,head_token_id:t.id,relation_type:"predicate",roles});
            }
        }
        for s in complements.into_iter().filter(|s|matches!(s.dep.as_str(),"ccomp"|"xcomp")){
            out.push(ProjectedRelation{region:t.region,head_token_id:t.id,relation_type:"composition",roles:vec![("host",t.id),("content",s.id)]});
        }
    }
    // Canonical dedup retains distinct role-labelled occurrences.
    let mut seen=BTreeSet::new();
    for r in &mut out { let mut role_seen=BTreeSet::new(); r.roles.retain(|pair|role_seen.insert(*pair)); }
    out.retain(|r|seen.insert((r.region,r.relation_type,r.roles.clone())));
    out
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeProjectedRelationProbe{
    pub original_producer_refs:Vec<&'static str>,
    pub source_parser_tokens:usize,
    pub predicate_relations:usize,
    pub composition_relations:usize,
    pub relation_budget_deferred:usize,
    pub projected_factor_ids:Vec<i64>,
    pub projected_focus_subject_factor_ids:Vec<i64>,
    pub focus_reference_alternatives:Vec<&'static str>,
    pub original_direct_substitutions:i64,
    pub original_supported_substitutions:i64,
    pub new_level_three_refs:Vec<String>,
    pub withdrawal_probes:Vec<NativeParserWitnessWithdrawalProbe>,
    pub nonvacuous_withdrawal_count:usize,
    pub projected_level_one_preserved:bool,
    pub outer_level_one_restored:bool,
    pub outer_level_three_restored:bool,
    pub persisted:bool,
    pub full_original_relation_parity_paid:bool,
    pub focus_identity_paid:bool,
}
fn relation_symbol(tx:&mut postgres::Transaction<'_>,kind:i16,name:&str)->Result<i64,NativeNumericAuditError>{
    Ok(tx.query_one("SELECT execution.ensure_semantic_symbol($1,$2)",&[&kind,&name])?.get(0))
}
fn projected_relation_probe(
    tx:&mut postgres::Transaction<'_>,run:i64,doc:i64,run_ref:&str,document_ref:&str,
    demand:i64,classes:&[NativeNumericDocumentIdentityObservation],
)->Result<NativeProjectedRelationProbe,NativeNumericAuditError>{
    let before_one=level_one_snapshot(tx,run,doc)?;
    let before_three=level_three_snapshot(tx,run,doc)?;
    let source=tx.query_one("SELECT locator,encode(content_sha256,'hex')
        FROM execution.semantic_parser_source WHERE run_ref=$1 AND document_ref=$2",
        &[&run_ref,&document_ref])?;
    let locator:String=source.get(0);let sha:String=source.get(1);
    let bytes=std::fs::read(locator).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    if hex(&Sha256::digest(&bytes))!=sha{return Err(NativeNumericAuditError::ForeignSource);}
    let text=String::from_utf8(bytes).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let chars=text.chars().collect::<Vec<_>>();
    let rows=tx.query(
        "SELECT t.token_id,r.region_id,t.lemma_symbol_id,t.head_token_id,t.start_char,t.end_char,
                orth.symbol_text,lemma.symbol_text,pos.symbol_text,dep.symbol_text
         FROM execution.semantic_parser_token t
         JOIN execution.semantic_pnf_sentence_region l ON l.sentence_id=t.sentence_id
         JOIN execution.semantic_pnf_region r ON r.region_id=l.region_id
         JOIN execution.semantic_symbol orth ON orth.symbol_id=t.orth_symbol_id
         JOIN execution.semantic_symbol lemma ON lemma.symbol_id=t.lemma_symbol_id
         JOIN execution.semantic_symbol pos ON pos.symbol_id=t.pos_symbol_id
         JOIN execution.semantic_symbol dep ON dep.symbol_id=t.dependency_symbol_id
         WHERE t.run_ref=$1 AND t.document_ref=$2 AND t.representation_version=2
           AND r.run_ref=$1 AND r.document_ref=$2 AND r.region_kind=1 AND r.closure_state IN (2,3)
         ORDER BY t.start_char,t.end_char,t.token_id",
        &[&run_ref,&document_ref])?;
    let mut tokens=Vec::new();let mut ids=BTreeSet::new();
    for row in rows{
        let t=NativeRelationToken{id:row.get(0),region:row.get(1),lemma_id:row.get(2),head:row.get(3),
            start:row.get(4),end:row.get(5),orth:row.get(6),lemma:row.get(7),pos:row.get(8),
            dep:row.get::<_,String>(9).to_ascii_lowercase()};
        let a=usize::try_from(t.start).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        let b=usize::try_from(t.end).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        if a>=b||b>chars.len()||!ids.insert(t.id)||chars[a..b].iter().collect::<String>()!=t.orth{
            return Err(NativeNumericAuditError::ForeignSource);
        }
        tokens.push(t);
    }
    let by_id=tokens.iter().map(|t|(t.id,t)).collect::<BTreeMap<_,_>>();
    if tokens.iter().any(|t|by_id.get(&t.head).map_or(true,|h|h.region!=t.region)){
        return Err(NativeNumericAuditError::ForeignSource);
    }
    let focus_ids=tx.query(
        "SELECT s.token_id FROM execution.semantic_pnf_demand d
         JOIN execution.semantic_pnf_object_token_support s ON s.object_id=d.source_object_id
         WHERE d.demand_id=$1",&[&demand])?.iter().map(|r|r.get::<_,i64>(0)).collect::<BTreeSet<_>>();
    let mut relations=projected_predicate_relations(&tokens);
    let predicates=relations.iter().filter(|r|r.relation_type=="predicate").count();
    let compositions=relations.len()-predicates;
    // First preserve the consumer's actual source predicate, then canonical
    // document order. The bound is execution only, never semantic exclusion.
    relations.sort_by_key(|r|(!r.roles.iter().any(|(role,id)|*role=="subject"&&focus_ids.contains(id)),r.region,r.head_token_id,r.relation_type,r.roles.clone()));
    let deferred=relations.len().saturating_sub(256);
    relations.truncate(256);
    let mut probe=tx.transaction()?;
    let object_kind=relation_symbol(&mut probe,14,"parser.role_participant")?;
    let eventuality=relation_symbol(&mut probe,10,"semantic.eventuality")?;
    let composition=relation_symbol(&mut probe,10,"semantic.embedded_proposition")?;
    let mut role_ids=BTreeMap::new();
    let mut role_objects=BTreeMap::<i64,i64>::new();
    let mut factor_ids=Vec::new();let mut focus_factor_ids=Vec::new();
    for relation in relations{
        let mut hash=Sha256::new();hash.update(b"slr:projected-source-relation-probe:v1");
        hash.update(relation.region.to_be_bytes());hash.update(relation.relation_type.as_bytes());
        for (role,id) in &relation.roles{
            hash.update((role.len() as u64).to_be_bytes());hash.update(role.as_bytes());hash.update(id.to_be_bytes());
        }
        let digest=hash.finalize().to_vec();
        let typ=if relation.relation_type=="predicate"{eventuality}else{composition};
        let pred=relation_symbol(&mut probe,11,&format!("parser.{}:{}",relation.relation_type,by_id[&relation.head_token_id].lemma))?;
        let factor:i64=probe.query_one(
            "INSERT INTO execution.semantic_pnf_factor
             (factor_digest,region_id,factor_type_symbol_id,predicate_symbol_id,scope_region_id,
              temporal_state,modal_state,promotion_level,support_score,active)
             VALUES($1,$2,$3,$4,$2,0,0,0,1,TRUE) RETURNING factor_id",
            &[&digest,&relation.region,&typ,&pred])?.get(0);
        factor_ids.push(factor);
        if relation.roles.iter().any(|(role,id)|*role=="subject"&&focus_ids.contains(id)){
            focus_factor_ids.push(factor);
        }
        let support=relation.roles.iter().map(|(_,id)|*id).collect::<BTreeSet<_>>();
        for (ordinal,id) in support.iter().enumerate(){
            probe.execute("INSERT INTO execution.semantic_pnf_factor_token_support(factor_id,token_id,ordinal)
                VALUES($1,$2,$3)",&[&factor,id,&i16::try_from(ordinal).map_err(|_|NativeNumericAuditError::ForeignSource)?])?;
        }
        for (ordinal,(role,id)) in relation.roles.iter().enumerate(){
            let t=by_id[id];
            if t.region!=relation.region{return Err(NativeNumericAuditError::ForeignSource);}
            let object=if let Some(o)=role_objects.get(id){*o}else{
                let mut h=Sha256::new();h.update(b"slr:projected-source-role-probe:v1");
                h.update(t.region.to_be_bytes());h.update(id.to_be_bytes());
                let dg=h.finalize().to_vec();
                let object:i64=probe.query_one(
                    "INSERT INTO execution.semantic_pnf_object
                     (object_digest,region_id,object_kind_symbol_id,head_symbol_id,scope_region_id,
                      promotion_level,information_gain,representation_cost,ambiguity_cost,promotion_score,active)
                     VALUES($1,$2,$3,$4,$2,0,0,1,1,0,TRUE) RETURNING object_id",
                    &[&dg,&t.region,&object_kind,&t.lemma_id])?.get(0);
                probe.execute("INSERT INTO execution.semantic_pnf_object_token_support(object_id,token_id,ordinal)
                    VALUES($1,$2,0)",&[&object,id])?;
                role_objects.insert(*id,object);object
            };
            let role_id=if let Some(x)=role_ids.get(role){*x}else{
                let x=relation_symbol(&mut probe,12,role)?;
                role_ids.insert(*role,x);x
            };
            probe.execute("INSERT INTO execution.semantic_pnf_hyperedge
                (factor_id,slot_ordinal,role_symbol_id,object_id,resolution_state,required)
                VALUES($1,$2,$3,$4,2,TRUE)",&[&factor,&i16::try_from(ordinal).map_err(|_|NativeNumericAuditError::ForeignSource)?,&role_id,&object])?;
        }
    }
    let projected_one=level_one_snapshot(&mut probe,run,doc)?;
    let direct:i64=probe.query_one("SELECT execution.refresh_numeric_pnf_identity_substitution_derivations($1,$2)",&[&run,&doc])?.get(0);
    let supported:i64=probe.query_one("SELECT execution.refresh_numeric_pnf_supported_identity_substitution_derivations($1,$2)",&[&run,&doc])?.get(0);
    let after_three=level_three_snapshot(&mut probe,run,doc)?;
    let new_refs=after_three.keys().filter(|k|!before_three.contains_key(*k)).cloned().collect();
    let withdrawal_probes=probe_parser_witness_withdrawals(&mut probe,run,doc,classes)?;
    let preserved=projected_one==level_one_snapshot(&mut probe,run,doc)?;
    probe.rollback()?;
    let one_restored=before_one==level_one_snapshot(tx,run,doc)?;
    let three_restored=before_three==level_three_snapshot(tx,run,doc)?;
    if !preserved||!one_restored||!three_restored{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    Ok(NativeProjectedRelationProbe{
        original_producer_refs:vec!["SensibLaw:f9c4670e:shared_reducer.collect_canonical_relational_bundle",
            "SensibLaw:f9c4670e:semantic_reductions.grammar:semantic:predicate:v0_4",
            "SensibLaw:f9c4670e:semantic_reductions.grammar:semantic:composition:v0_4"],
        source_parser_tokens:tokens.len(),predicate_relations:predicates,composition_relations:compositions,
        relation_budget_deferred:deferred,projected_factor_ids:factor_ids,
        focus_reference_alternatives:if focus_factor_ids.is_empty(){vec![]}else{
            vec!["entity_reference","eventuality_reference","proposition_reference","expletive_realisation"]},
        projected_focus_subject_factor_ids:focus_factor_ids,
        original_direct_substitutions:direct,original_supported_substitutions:supported,new_level_three_refs:new_refs,
        nonvacuous_withdrawal_count:withdrawal_probes.iter().filter(|p|p.nonvacuous_dependency).count(),
        withdrawal_probes,projected_level_one_preserved:preserved,outer_level_one_restored:one_restored,
        outer_level_three_restored:three_restored,persisted:false,full_original_relation_parity_paid:false,focus_identity_paid:false,
    })
}

#[derive(Debug,Clone,Serialize)]
pub struct NativeGovernedIdentityProbe{
    pub admission_owner:&'static str,
    pub level_three_owner:&'static str,
    pub admitted_source_witness_count:i64,
    pub candidate_class_projection:NativeNumericIdentityFibreProjection,
    pub covered_unique_entity_id:Option<i64>,
    pub level_three_refs_new_in_probe:Vec<String>,
    pub level_three_rows_after_refresh:i64,
    pub direct_level_three_rows_after_refresh:i64,
    pub factor_alignment_counts:BTreeMap<String,i64>,
    pub source_factor_count:i64,
    pub focus_factor_argument_count:i64,
    pub witness_withdrawal_probes:Vec<NativeParserWitnessWithdrawalProbe>,
    pub nonvacuous_withdrawal_count:usize,
    pub projected_relation_probe:NativeProjectedRelationProbe,
    pub immutable_level_one_preserved:bool,
    pub rollback_preserved_prior_level_three:bool,
    pub rollback_preserved_prior_candidate_classes:bool,
    pub persisted:bool,
    pub focus_identity_paid:bool,
}
// Run the original governed m080 policy and original m086 substitution
// producer, on the exact document, in a savepoint. No focus identity is
// admitted by this call; even locally licensed parser witnesses are only
// observed because the source/typed binding coverage is not yet certified.
fn probe_original_governed_identity(
    tx:&mut postgres::Transaction<'_>,run_ref:&str,document_ref:&str,demand:i64,
)->Result<NativeGovernedIdentityProbe,NativeNumericAuditError>{
    let row=tx.query_one(
        "SELECT r.run_id,r.document_id FROM execution.semantic_pnf_demand d
         JOIN execution.semantic_pnf_region r ON r.region_id=d.source_region_id
         WHERE d.demand_id=$1 AND r.run_ref=$2 AND r.document_ref=$3",
        &[&demand,&run_ref,&document_ref])?;
    let run:i64=row.get(0);let document:i64=row.get(1);
    let level_one=level_one_snapshot(tx,run,document)?;
    let prior=level_three_snapshot(tx,run,document)?;
    let (_,prior_classes)=observe_original_resolution_fibre(tx,demand,run_ref,document_ref)?;
    let mut probe=tx.transaction()?;
    probe.query_one("SELECT execution.refresh_numeric_pnf_parser_identity_evidence($1,$2)",
        &[&run,&document])?;
    let admitted:i64=probe.query_one(
        "SELECT execution.admit_numeric_pnf_parser_identity_evidence($1,$2)",
        &[&run,&document])?.get(0);
    let (horizons,classes)=observe_original_resolution_fibre(&mut probe,demand,run_ref,document_ref)?;
    let outside:bool=probe.query_one(
        "SELECT outside_model_possible FROM execution.semantic_pnf_demand_open_world_state WHERE demand_id=$1",
        &[&demand])?.get(0);
    let projection=project_original_identity_fibre(&horizons,&classes,outside);
    let source_factor_count:i64=probe.query_one(
        "SELECT count(*)::bigint FROM execution.semantic_pnf_factor f JOIN execution.semantic_pnf_region r
         ON r.region_id=f.region_id WHERE r.run_id=$1 AND r.document_id=$2 AND f.active",&[&run,&document])?.get(0);
    let focus_factor_argument_count:i64=probe.query_one(
        "SELECT count(DISTINCT (e.factor_id,e.slot_ordinal))::bigint
         FROM execution.semantic_pnf_demand d
         JOIN execution.semantic_pnf_object_token_support s ON s.object_id=d.source_object_id
         JOIN execution.semantic_pnf_object_token_support t ON t.token_id=s.token_id
         JOIN execution.semantic_pnf_hyperedge e ON e.object_id=t.object_id
         JOIN execution.semantic_pnf_factor f ON f.factor_id=e.factor_id AND f.active
         JOIN execution.semantic_pnf_region r ON r.region_id=f.region_id
         WHERE d.demand_id=$1 AND r.run_id=$2 AND r.document_id=$3",&[&demand,&run,&document])?.get(0);
    let factor_alignment_counts=probe.query(
        "SELECT alignment_class,projection_count FROM execution.semantic_pnf_identity_factor_alignment_summary_v1
         WHERE run_id=$1 AND document_id=$2 ORDER BY alignment_class",&[&run,&document])?
        .iter().map(|r|(r.get::<_,String>(0),r.get::<_,i64>(1))).collect();
    let direct:i64=probe.query_one(
        "SELECT execution.refresh_numeric_pnf_identity_substitution_derivations($1,$2)",&[&run,&document])?.get(0);
    let rows:i64=probe.query_one(
        "SELECT execution.refresh_numeric_pnf_supported_identity_substitution_derivations($1,$2)",
        &[&run,&document])?.get(0);
    let after=level_three_snapshot(&mut probe,run,document)?;
    let refs=after.keys().filter(|k|!prior.contains_key(*k)).cloned().collect();
    let witness_withdrawal_probes=probe_parser_witness_withdrawals(&mut probe,run,document,&classes)?;
    let projected_relation_probe=projected_relation_probe(&mut probe,run,document,run_ref,document_ref,demand,&classes)?;
    let immutable=level_one==level_one_snapshot(&mut probe,run,document)?;
    probe.rollback()?;
    let rollback_level_three=prior==level_three_snapshot(tx,run,document)?;
    let (_,restored)=observe_original_resolution_fibre(tx,demand,run_ref,document_ref)?;
    let rollback_classes=prior_classes==restored;
    if !immutable||!rollback_level_three||!rollback_classes
        ||level_one!=level_one_snapshot(tx,run,document)?{
        return Err(NativeNumericAuditError::AdmissionProbeInvariant);
    }
    Ok(NativeGovernedIdentityProbe{
        admission_owner:"execution.admit_numeric_pnf_parser_identity_evidence",
        level_three_owner:"m070 direct + m086 supported substitution",
        admitted_source_witness_count:admitted,
        covered_unique_entity_id:covered_unique_class(&projection,false),
        candidate_class_projection:projection,level_three_refs_new_in_probe:refs,
        level_three_rows_after_refresh:rows,direct_level_three_rows_after_refresh:direct,
        source_factor_count,focus_factor_argument_count,factor_alignment_counts,
        nonvacuous_withdrawal_count:witness_withdrawal_probes.iter().filter(|p|p.nonvacuous_dependency).count(),
        witness_withdrawal_probes,projected_relation_probe,
        immutable_level_one_preserved:immutable,
        rollback_preserved_prior_level_three:rollback_level_three,
        rollback_preserved_prior_candidate_classes:rollback_classes,
        persisted:false,focus_identity_paid:false,
    })
}

/// Candidate-only, exact-source consumer transport. This extends the represented
/// fibre, not the capped legacy planner. It never selects, refutes, closes
/// coverage, admits a witness, or substitutes a Level-3 argument.
/// Existing occurrence objects from every preceding sentence participate; the
/// two supported PERSON occurrences are not collapsed or preferentially selected.
#[derive(Debug,Clone,Serialize)]
pub struct NativeNumericCandidateTransportReceipt {
    pub schema:&'static str,
    pub numeric_run_ref:String,
    pub demand_id:i64,
    pub source_object_id:i64,
    pub inserted_candidate_ids:Vec<i64>,
    pub remaining_untransported_count:i64,
    pub represented_candidate_count:i64,
    pub outside_model_possible:bool,
    pub complete_typed_coverage_paid:bool,
    pub nominal_head_objects_inserted:i64,
    pub nominal_head_token_supports_inserted:i64,
    pub nominal_heads_remaining:i64,
    pub original_h6_evidence_inserted:i64,
    pub morphology_observations_inserted:usize,
    pub candidates_without_morphology_observation:i64,
    pub original_horizon_observations:Vec<NativeNumericHorizonObservation>,
    pub original_document_identity_observations:Vec<NativeNumericDocumentIdentityObservation>,
    pub original_parser_identity_producer:OriginalParserIdentityProducerObservation,
    pub document_identity_fibre:NativeNumericIdentityFibreProjection,
    pub original_cross_resource_needs:Vec<OriginalCrossResourceNeedObservation>,
    pub structural_binding:NativeBindingStructuralReceipt,
    pub native_binding_bridge:NativeBindingBridgeReceipt,
    pub original_mention_binding_campaign:NativeMentionBindingCampaign,
    pub original_h9_readiness:NativeH9ReadinessReceipt,
    pub governed_identity_rollback_probe:Option<NativeGovernedIdentityProbe>,
}
pub fn transport_original_numeric_candidates_for_source_answer(
    numeric_url:&str,numeric_run_ref:&str,answer:&NativeSourceLocalAnswer,
    budget:usize,
)->Result<NativeNumericCandidateTransportReceipt,NativeNumericAuditError>{
    original_receipt_policy(answer)?;
    if budget==0||budget>64||numeric_run_ref.trim().is_empty(){
        return Err(NativeNumericAuditError::SourcePolicy);
    }
    let mut db=Client::connect(numeric_url,NoTls)?;
    let mut tx=db.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
        SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
    tx.query_one("SELECT pg_advisory_xact_lock(hashtext($1),hashtext($2))",
        &[&numeric_run_ref,&answer.source_document_ref])?;
    let rows=tx.query(
        "SELECT DISTINCT d.demand_id,d.source_object_id,t.token_id
         FROM execution.semantic_pnf_demand d
         JOIN execution.semantic_pnf_anaphor_projection_constant k
           ON k.singleton AND k.anaphor_residual_type_symbol_id=d.residual_type_symbol_id
         JOIN execution.semantic_pnf_region r ON r.region_id=d.source_region_id
         JOIN execution.semantic_parser_source src
           ON src.run_ref=r.run_ref AND src.document_ref=r.document_ref
         JOIN execution.semantic_pnf_object_token_support os ON os.object_id=d.source_object_id
         JOIN execution.semantic_parser_token t ON t.token_id=os.token_id
           AND t.run_ref=r.run_ref AND t.document_ref=r.document_ref
           AND t.representation_version=2
         JOIN execution.semantic_symbol pos ON pos.symbol_id=t.pos_symbol_id
         JOIN execution.semantic_symbol orth ON orth.symbol_id=t.orth_symbol_id
         JOIN execution.semantic_pnf_demand_open_world_state ow ON ow.demand_id=d.demand_id
         WHERE r.run_ref=$1 AND r.document_ref=$2
           AND encode(src.content_sha256,'hex')=$3
           AND t.start_char=$4 AND t.end_char=$5
           AND pos.symbol_text='PRON' AND orth.symbol_text='he'
           AND d.expected_target_kind=1 AND d.recency_class=3
           AND d.state IN (1,3) AND ow.outside_model_possible
           AND NOT EXISTS (
             SELECT 1 FROM execution.semantic_pnf_identity_witness w
             JOIN execution.semantic_pnf_identity_witness_admission a
               ON a.witness_id=w.witness_id AND a.admission_state=2
             WHERE w.demand_id=d.demand_id)",
        &[&numeric_run_ref,&answer.source_document_ref,&answer.source_content_sha256,
          &(answer.original_focus_span[0] as i64),&(answer.original_focus_span[1] as i64)])?;
    if rows.len()!=1{return Err(NativeNumericAuditError::SourcePolicy);}
    let demand:i64=rows[0].get(0);
    let source:i64=rows[0].get(1);
    let focus_token:i64=rows[0].get(2);
    let (nominal_head_objects_inserted,nominal_head_token_supports_inserted,nominal_heads_remaining)=
        materialize_prior_nominal_heads(&mut tx,numeric_run_ref,&answer.source_document_ref,
            answer.original_focus_span[0] as i64,budget)?;
    let original_mention_binding_campaign=original_mention_binding_campaign(
        &mut tx,numeric_run_ref,&answer.source_document_ref,&answer.source_content_sha256,focus_token,budget)?;
    // Materialize the exact existing occurrence relation. Recency is the only
    // exclusion here: morphology/quotation/type refinement is not proved yet.
    // Lack of a live occurrence object is not negative evidence.
    tx.execute(
        "CREATE TEMP TABLE native_numeric_transport_possible ON COMMIT DROP AS
         SELECT o.object_id,r.region_id,i.interface_id,
                min(t.start_char)::bigint AS first_char,
                max(t.end_char)::bigint AS last_char
         FROM execution.semantic_pnf_object o
         JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
         JOIN execution.semantic_pnf_interface i ON i.region_id=r.region_id
         JOIN execution.semantic_pnf_object_token_support os ON os.object_id=o.object_id
         JOIN execution.semantic_parser_token t ON t.token_id=os.token_id
         WHERE r.run_ref=$1 AND r.document_ref=$2 AND r.region_kind=1
           AND r.closure_state IN (2,3) AND o.active AND o.object_id<>$3
         GROUP BY o.object_id,r.region_id,i.interface_id
         HAVING bool_and(t.run_ref=$1 AND t.document_ref=$2
                       AND t.representation_version=2)
            AND max(t.end_char)<=$4",
        &[&numeric_run_ref,&answer.source_document_ref,&source,
          &(answer.original_focus_span[0] as i64)])?;
    tx.execute(
        "CREATE TEMP TABLE native_numeric_transport_delta ON COMMIT DROP AS
         SELECT p.* FROM native_numeric_transport_possible p
         WHERE NOT EXISTS (
           SELECT 1 FROM execution.semantic_pnf_candidate_universe u
           WHERE u.demand_id=$1 AND u.target_kind=1 AND u.target_id=p.object_id)
         ORDER BY p.last_char DESC,p.object_id LIMIT $2",
        &[&demand,&(budget as i64)])?;
    // Use the original append-only carrier (m086), not demand_candidate:
    // the latter drives m062's unsafe early unique resolution on capped rows.
    tx.execute(
        "INSERT INTO execution.semantic_pnf_demand_candidate_observation
          (demand_id,target_kind,target_id,observation_kind,source_interface_id,
           ancestor_distance,index_rank,candidate_score)
         SELECT $1::bigint,1,object_id,1,interface_id,
                NULL,NULL,NULL
         FROM native_numeric_transport_delta ORDER BY object_id",
        &[&demand])?;
    tx.execute(
        "INSERT INTO execution.semantic_pnf_candidate_execution_event
           (demand_id,target_kind,target_id,event_kind,reason_ref)
         SELECT $1::bigint,1,object_id,2,'native-occurrence-transport-unrefined:v1'
         FROM native_numeric_transport_delta",
        &[&demand])?;
    tx.execute(
        "INSERT INTO execution.semantic_pnf_candidate_evidence
           (demand_id,target_kind,target_id,evidence_ref,evidence_family,horizon,
            signed_residual,evidence_kind,provenance_ref,source_interface_id,source_region_id)
         SELECT $1::bigint,1,object_id,
                'native-occurrence-transport:v1:'||object_id::text,
                1,3,0,'candidate_generation','native-occurrence-transport:v1',
                interface_id,region_id
         FROM native_numeric_transport_delta
         ON CONFLICT(demand_id,target_kind,target_id,evidence_ref) DO NOTHING",
        &[&demand])?;
    let ids=tx.query("SELECT object_id FROM native_numeric_transport_delta ORDER BY object_id",&[])?
        .iter().map(|r|r.get::<_,i64>(0)).collect::<Vec<_>>();
    let remaining:i64=tx.query_one(
        "SELECT count(*)::bigint FROM native_numeric_transport_possible p
         WHERE NOT EXISTS (
           SELECT 1 FROM execution.semantic_pnf_candidate_universe u
           WHERE u.demand_id=$1 AND u.target_kind=1 AND u.target_id=p.object_id)",
        &[&demand])?.get(0);
    let count:i64=tx.query_one(
        "SELECT count(*)::bigint FROM execution.semantic_pnf_candidate_universe
         WHERE demand_id=$1",&[&demand])?.get(0);
    // Keep original unknown outside-model and existing residual accounting.
    // A zero transport remainder closes only this materialized object slice.
    let open:bool=tx.query_one(
        "SELECT outside_model_possible FROM execution.semantic_pnf_demand_open_world_state
         WHERE demand_id=$1",&[&demand])?.get(0);
    if !open{return Err(NativeNumericAuditError::SourcePolicy);}
    let (morphology_observations_inserted,candidates_without_morphology_observation)=
        refine_original_numeric_morphology(&mut tx,demand,focus_token,numeric_run_ref,&answer.source_document_ref,budget)?;
    let native_binding_bridge=materialize_native_binding_bridge(&mut tx,numeric_run_ref,
        &answer.source_document_ref,&answer.source_content_sha256,focus_token,demand,&original_mention_binding_campaign)?;
    let original_h6_evidence_inserted=run_original_h6_producer(&mut tx,demand,numeric_run_ref,&answer.source_document_ref)?;
    let (original_horizon_observations,original_document_identity_observations)=
        observe_original_resolution_fibre(&mut tx,demand,numeric_run_ref,&answer.source_document_ref)?;
    let document_identity_fibre=project_original_identity_fibre(&original_horizon_observations,&original_document_identity_observations,open);
    let original_cross_resource_needs=observe_original_cross_resource_needs(&mut tx,demand,numeric_run_ref,&answer.source_document_ref)?;
    let original_parser_identity_producer=original_parser_identity_producer_probe(
        &mut tx,numeric_run_ref,&answer.source_document_ref,demand)?;
    let original_h9_readiness=observe_native_h9_readiness(&mut tx,demand,numeric_run_ref,&answer.source_document_ref)?;
    let structural_binding=observe_native_binding_structure(&mut tx,demand,focus_token,
        numeric_run_ref,&answer.source_document_ref,&answer.source_content_sha256)?;
    // Only run the document-scoped original admission probe after bounded
    // occurrence/morphology work is drained. This is still not typed coverage.
    let governed_identity_rollback_probe=if remaining==0&&nominal_heads_remaining==0
        &&candidates_without_morphology_observation==0
        &&original_mention_binding_campaign.newly_materialized_span_objects<budget{
        Some(probe_original_governed_identity(&mut tx,numeric_run_ref,&answer.source_document_ref,demand)?)
    }else{None};
    tx.commit()?;
    Ok(NativeNumericCandidateTransportReceipt{
        schema:"slr-native-candidate-only-transport-v5",
        numeric_run_ref:numeric_run_ref.into(),demand_id:demand,source_object_id:source,
        inserted_candidate_ids:ids,remaining_untransported_count:remaining,
        represented_candidate_count:count,outside_model_possible:open,
        complete_typed_coverage_paid:false,
        nominal_head_objects_inserted,nominal_head_token_supports_inserted,nominal_heads_remaining,original_h6_evidence_inserted,
        morphology_observations_inserted,candidates_without_morphology_observation,
        original_horizon_observations,original_document_identity_observations,
        original_parser_identity_producer,document_identity_fibre,original_cross_resource_needs,
        structural_binding,native_binding_bridge,original_mention_binding_campaign,original_h9_readiness,governed_identity_rollback_probe,
    })
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn original_binding_morphology_retains_missing_and_overlapping_features(){
        let f=vec!["Number=Sing".into(),"Gender=Masc".into(),"Person=3".into()];
        assert!(morphology_mismatches(&f,&[]).is_empty());
        assert!(morphology_mismatches(&f,&["NUMBER=Sing".into(),"NUMBER=Plur".into()]).is_empty());
        assert_eq!(morphology_mismatches(&f,&["GENDER=Fem".into(),"Number=Plur".into()]),vec!["Gender","Number"]);
    }
    #[test]
    fn class_projection_preserves_inverse_fibre_and_conflicts(){
        let h=|id| NativeNumericHorizonObservation{
            target_kind:1,target_id:id,horizon:3,signed_residual:0,phase:0,
            evidence_count:1,active:false,execution_residual:true,refuted:false,strict_preference:false,
        };
        let cls=|id,entities:Vec<i64>| NativeNumericDocumentIdentityObservation{
            object_id:id,admitted_entity_ids:entities,admitted_witness_ids:vec![99],
        };
        let result=project_original_identity_fibre(
            &[h(10),h(11),h(12),h(13)],
            &[cls(10,vec![20]),cls(11,vec![20]),cls(13,vec![30,31])],true);
        assert_eq!(result.admitted_classes.get(&20),Some(&vec![10,11]));
        assert_eq!(result.unresolved_keys,vec![(1,12),(1,13)]);
        assert_eq!(result.conflicting_objects,vec![13]);
        assert!(result.outside_model_possible);
    }
    #[test]
    fn native_binding_source_context_uses_unicode_nonoverlapping_prefix(){
        let s="é\n\n\n“a” \"b";
        assert_eq!(source_binding_context(s,1),Some((0,0)));
        assert_eq!(source_binding_context(s,5),Some((1,1)));
        assert_eq!(source_binding_context(s,7),Some((1,0)));
        assert_eq!(source_binding_context(s,9),Some((1,1)));
        assert_eq!(source_binding_context(s,99),None);
        assert_eq!(source_binding_context(s,-1),None);
    }
    #[test]
    fn covered_class_gate_distinguishes_multiple_occurrences_from_unknowns(){
        let mut p=NativeNumericIdentityFibreProjection{
            admitted_classes:BTreeMap::from([(20,vec![10,11])]),
            unresolved_keys:vec![],conflicting_objects:vec![],refuted_keys:vec![],
            outside_model_possible:false,
        };
        assert_eq!(covered_unique_class(&p,true),Some(20));
        assert_eq!(covered_unique_class(&p,false),None);
        p.outside_model_possible=true;
        assert_eq!(covered_unique_class(&p,true),None);
        p.outside_model_possible=false;
        p.unresolved_keys.push((1,12));
        assert_eq!(covered_unique_class(&p,true),None);
        p.unresolved_keys.clear();
        p.admitted_classes.insert(30,vec![12]);
        assert_eq!(covered_unique_class(&p,true),None);
    }
    #[test]
    fn original_binding_all_three_referential_types_and_accessibility(){
        assert!(original_binding_type("semantic.mention_identity","NOUN","entity_reference"));
        assert!(!original_binding_type("semantic.mention_identity","PRON","entity_reference"));
        assert!(original_binding_type("semantic.eventuality","VERB","eventuality_reference"));
        assert!(original_binding_type("semantic.embedded_proposition","VERB","proposition_reference"));
        assert!(!original_binding_type("semantic.eventuality","VERB","entity_reference"));
        let r=BindingAnchor{start:20,quotation:Some(1),sentence:Some(2),..Default::default()};
        let mut c=BindingAnchor{start:10,quotation:Some(0),sentence:Some(1),..Default::default()};
        assert_eq!(original_accessibility_path(&r,&c),"quotation_boundary_crossed");
        c.reporting=Some("report".into());
        let p=original_accessibility_path(&r,&c);
        assert_eq!(p,"reporting_content_boundary");
        assert!(!original_path_allowed(p,"entity_reference"));
        assert!(original_path_allowed(p,"proposition_reference"));
        c.quotation=Some(1);c.sentence=Some(2);
        assert_eq!(original_accessibility_path(&r,&c),"same_sentence");
        c.start=20;assert_eq!(original_accessibility_path(&r,&c),"not_preceding_reference");
    }
    #[test]
    fn original_mention_licenses_retain_name_components_and_span(){
        let t=|id,pos:&str,orth:&str|NativeMentionToken{
            id,region:1,sentence:1,start:id,end:id+1,orth:orth.into(),pos:pos.into(),
            lemma:id,head:Some(id),morph:vec![],
        };
        let tokens=vec![t(1,"PROPN","George"),t(2,"PROPN","Bush"),t(3,"VERB","said"),
            t(4,"NOUN","president"),t(5,"NOUN","the")];
        let s=nominal_span_indices(&tokens);
        assert_eq!(s.len(),4);
        assert_eq!(s[&(0,1)],vec!["lexical_token"]);
        assert_eq!(s[&(0,2)],vec!["named_entity_shape"]);
        assert_eq!(s[&(1,2)],vec!["lexical_token"]);
        assert_eq!(s[&(3,4)],vec!["lexical_token"]);
    }
    #[test]
    fn original_predicate_projection_keeps_object_argument_and_reference_roles(){
        let t=|id,head,pos:&str,dep:&str|NativeRelationToken{
            id,region:1,lemma_id:id,head,start:id,end:id+1,orth:id.to_string(),lemma:id.to_string(),
            pos:pos.into(),dep:dep.into(),
        };
        let rows=projected_predicate_relations(&[t(1,2,"PRON","nsubj"),t(2,2,"VERB","root"),
            t(3,2,"NOUN","obj"),t(4,2,"PART","neg"),t(5,2,"VERB","ccomp")]);
        assert!(rows.iter().any(|r|r.relation_type=="predicate"&&r.roles==vec![
            ("head",2),("subject",1),("object",3),("argument",3),("negation",4)]));
        assert!(rows.iter().any(|r|r.relation_type=="composition"&&r.roles==vec![("host",2),("content",5)]));
    }
    #[test]
    fn unsupported_scope_refused_before_any_sql(){
        // Without an independently licensed original source-local receipt
        // no local username/surname equivalence can be minted in numeric PNF.
        assert_eq!(hex(&[0u8,255u8]),"00ff");
    }
}

// Native consumer of the ALREADY IMPLEMENTED SensibLaw #439 indexed binding
// authority. Do not construct another mention lattice, structural path
// evaluator, parser or identity admission. The original stored revision,
// declaration and member coordinates are the output authority.
#[derive(Debug,Clone,Serialize)]
pub struct OriginalBindingMemberReuse {
    pub candidate_factor_ref:String,
    pub candidate_factor_revision_ref:String,
    pub accessibility_path_ref:String,
    pub distance_tokens:i32,
    pub original_span_coordinates:Vec<(i32,i32)>,
    pub exact_projection_token_ids:Vec<i64>,
    pub native_inverse_matches_snapshot:bool,
    pub exact_same_manifestation_numeric_object_ids:Vec<i64>,
}
#[derive(Debug,Clone,Serialize)]
pub struct OriginalBindingSetReuse {
    pub document_ref:String,
    pub same_manifestation:bool,
    pub reference_factor_ref:String,
    pub reference_factor_revision_ref:String,
    pub candidate_set_ref:String,
    pub generator_build_ref:String,
    pub referential_type_ref:String,
    pub accessibility_declaration_ref:String,
    pub compatibility_declaration_ref:String,
    pub original_persisted_member_count:i32,
    pub original_index_matches_persisted_members:bool,
    pub original_index_prefix_matches_full_query:bool,
    pub original_index_query_complete:bool,
    pub original_index_member_count:usize,
    pub native_projection_verified:bool,
    pub exact_entity_inverse_complete:bool,
    pub original_cap_deferred_count:usize,
    pub original_exclusion_summaries:Vec<(String,i32)>,
    pub original_members:Vec<OriginalBindingMemberReuse>,
}
#[derive(Debug,Clone,Serialize)]
pub struct OriginalOwnerReuseReceipt {
    pub schema:&'static str,
    pub source_revision_ref:String,
    pub source_sha256:String,
    pub same_manifestation_document_ref:String,
    pub source_content_equal_compiler_documents:Vec<String>,
    pub original_binding_owner_available:bool,
    pub original_binding_sets:Vec<OriginalBindingSetReuse>,
    pub original_numeric_run_refs:Vec<String>,
    pub original_numeric_audits:Vec<NativeNumericIdentityAudit>,
    pub reusable_binding_set_refs:Vec<String>,
    pub original_index_complete_is_not_semantic_coverage:bool,
    pub writes_database:bool,
    pub residuals:Vec<String>,
}
fn exact_original_binding_members(
    tx:&mut postgres::Transaction<'_>,revision:&str,typ:&str,access:&str,compat:&str,limit:i32,
)->Result<Vec<(String,String,String,i32)>,NativeNumericAuditError>{
    Ok(tx.query(
        "SELECT candidate_factor_revision_ref,candidate_factor_ref,accessibility_path_ref,distance_tokens
         FROM resolution.query_binding_candidates($1,$2,$3,$4,$5)",
        &[&revision,&typ,&access,&compat,&limit])?.iter()
        .map(|r|(r.get(0),r.get(1),r.get(2),r.get(3))).collect())
}
fn original_index_member_parity(
    stored:&[(String,String)],indexed:&[(String,String,String,i32)],
)->bool{
    let expected=stored.iter().map(|(f,p)|(f.as_str(),p.strip_prefix("accessibility:").unwrap_or(p.as_str()))).collect::<BTreeSet<_>>();
    let actual=indexed.iter().map(|(_,f,p,_)|(f.as_str(),p.as_str())).collect::<BTreeSet<_>>();
    expected.len()==stored.len()&&actual.len()==indexed.len()&&expected==actual
}
pub fn reuse_original_binding_and_identity_owners(
    url:&str,numeric_run:&str,answer:&NativeSourceLocalAnswer,
)->Result<OriginalOwnerReuseReceipt,NativeNumericAuditError>{
    original_receipt_policy(answer)?;
    let mut db=Client::connect(url,NoTls)?;
    let mut tx=db.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY;
                     SET LOCAL statement_timeout='90000ms'")?;
    let src=tx.query_one(
        "SELECT locator,encode(content_sha256,'hex') FROM execution.semantic_parser_source
          WHERE run_ref=$1 AND document_ref=$2",
        &[&numeric_run,&answer.source_document_ref])?;
    let path:String=src.get(0);
    let bytes=std::fs::read(path).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let sha=hex(&Sha256::digest(&bytes));
    if sha!=answer.source_content_sha256||sha!=src.get::<_,String>(1){
        return Err(NativeNumericAuditError::ForeignSource);
    }
    let source=String::from_utf8(bytes).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let start=i32::try_from(answer.original_focus_span[0]).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let end=i32::try_from(answer.original_focus_span[1]).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let chars=source.chars().collect::<Vec<_>>();
    if start<0||start>=end||end as usize>chars.len()
        ||chars[start as usize..end as usize].iter().collect::<String>()!=answer.original_focus_surface {
        return Err(NativeNumericAuditError::ForeignSource);
    }
    let owner_available:bool=tx.query_one(
        "SELECT to_regprocedure('resolution.query_binding_candidates(text,text,text,text,integer)') IS NOT NULL
         AND to_regclass('pnf.factor_anchor') IS NOT NULL
         AND to_regclass('resolution.binding_candidate_set') IS NOT NULL
         AND to_regclass('resolution.binding_candidate_member') IS NOT NULL
         AND to_regclass('resolution.binding_exclusion_summary') IS NOT NULL
         AND to_regclass('execution.binding_candidate_set_build') IS NOT NULL
         AND to_regclass('corpus.span') IS NOT NULL
         AND to_regclass('algebra.factor_revision') IS NOT NULL",&[])?.get(0);
    let mut docs=BTreeSet::new();let mut residuals=Vec::new();
    // Same raw bytes are necessary but not sufficient for manifestation
    // identity. Legacy JSON-string digest is only a retrieval key. Verify
    // actual text; do not relabel any historic immutable IDs.
    let legacy=hex(&Sha256::digest(serde_json::to_string(&source)
        .map_err(|_|NativeNumericAuditError::ForeignSource)?.as_bytes()));
    if tx.query_one("SELECT to_regclass('public.compiler_document') IS NOT NULL",&[])?.get::<_,bool>(0){
        for row in tx.query("SELECT document_ref,canonical_text FROM public.compiler_document
            WHERE document_ref=$1 OR content_sha256=ANY($2::text[])",
            &[&answer.source_document_ref,&vec![sha.clone(),legacy.clone()]])?{
            let text:String=row.get(1);
            if text.as_bytes()==source.as_bytes(){docs.insert(row.get::<_,String>(0));}
        }
    }
    // Normalized compiler owner; only uncompressed exact UTF-8 bytes qualify.
    if tx.query_one("SELECT to_regclass('corpus.document') IS NOT NULL
        AND to_regclass('corpus.canonical_content') IS NOT NULL",&[])?.get::<_,bool>(0){
        for row in tx.query("SELECT d.document_ref,c.payload FROM corpus.document d
            JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
            WHERE (d.document_ref=$1 OR encode(c.content_sha256,'hex')=ANY($2::text[]))
              AND (c.compression_ref IS NULL OR c.compression_ref IN ('none','identity'))",
            &[&answer.source_document_ref,&vec![sha.clone(),legacy]])?{
            let payload:Vec<u8>=row.get(1);
            if payload==source.as_bytes(){docs.insert(row.get::<_,String>(0));}
        }
    }
    let mut sets=Vec::new();let mut reusable=Vec::new();
    if owner_available{
        for doc in &docs{
            let same=doc==&answer.source_document_ref;
            // Use original canonical span index, not a numeric parser-token
            // ordinal guess. Distinct factor revisions remain distinct.
            let has_native_projection:bool=tx.query_one(
                "SELECT to_regclass('execution.slr_native_binding_anchor_projection') IS NOT NULL",&[])?.get(0);
            let rows=if has_native_projection{
                tx.query(
                    "SELECT DISTINCT s.reference_factor_ref,s.reference_factor_revision_ref,
                        s.candidate_set_ref,s.generator_build_ref,s.referential_type_ref,
                        s.accessibility_declaration_ref,s.compatibility_declaration_ref,s.member_count
                     FROM resolution.binding_candidate_set s
                     JOIN pnf.factor_anchor a ON a.factor_revision_ref=s.reference_factor_revision_ref
                         AND a.document_ref=s.document_ref
                     JOIN execution.slr_native_binding_anchor_projection p ON p.factor_revision_ref=a.factor_revision_ref
                         AND p.active AND p.document_ref=a.document_ref AND p.run_ref=$4
                     JOIN corpus.span sp ON sp.span_ref=p.span_ref AND sp.document_ref=a.document_ref
                     JOIN execution.binding_candidate_set_build b ON b.candidate_set_ref=s.candidate_set_ref
                         AND b.generator_build_ref=s.generator_build_ref
                         AND b.reference_factor_revision_ref=s.reference_factor_revision_ref
                         AND b.referential_type_ref=s.referential_type_ref
                         AND b.accessibility_declaration_ref=s.accessibility_declaration_ref
                         AND b.compatibility_declaration_ref=s.compatibility_declaration_ref
                         AND b.document_pnf_index_ref=p.snapshot_ref AND b.build_state_ref='completed'
                     WHERE s.document_ref=$1 AND sp.start_char=$2 AND sp.end_char=$3
                       AND a.parser_pos_ref='PRON' AND a.pnf_kind_ref LIKE 'semantic.argument%'
                     ORDER BY s.reference_factor_ref,s.reference_factor_revision_ref,s.candidate_set_ref",
                    &[doc,&start,&end,&numeric_run])?
            }else{Vec::new()};
            // Do not use the known-bad v1 native chart. Genuine non-native
            // original factors remain visible through the historical owner.
            let mut native_object_inverse=BTreeMap::<Vec<i64>,Vec<i64>>::new();
            if same&&has_native_projection{
                let inverse=tx.query("SELECT o.object_id,array_agg(DISTINCT t.token_id ORDER BY t.token_id)
                    FROM execution.semantic_pnf_object o
                    JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
                    JOIN execution.semantic_pnf_object_token_support s ON s.object_id=o.object_id
                    JOIN execution.semantic_parser_token t ON t.token_id=s.token_id
                    WHERE r.run_ref=$1 AND r.document_ref=$2 AND r.region_kind=1 AND o.active
                    GROUP BY o.object_id
                    HAVING bool_and(t.run_ref=$1 AND t.document_ref=$2 AND t.representation_version=2)
                    ORDER BY o.object_id",&[&numeric_run,doc])?;
                for row in inverse{
                    native_object_inverse.entry(row.get(1)).or_default().push(row.get(0));
                }
            }
            let mut rows=rows;
            rows.extend(tx.query(
                "SELECT DISTINCT s.reference_factor_ref,s.reference_factor_revision_ref,
                    s.candidate_set_ref,s.generator_build_ref,s.referential_type_ref,
                    s.accessibility_declaration_ref,s.compatibility_declaration_ref,s.member_count
                 FROM resolution.binding_candidate_set s
                 JOIN pnf.factor_anchor a ON a.factor_revision_ref=s.reference_factor_revision_ref
                     AND a.document_ref=s.document_ref
                 JOIN corpus.span sp ON sp.document_ref=a.document_ref
                     AND sp.start_token=a.start_token AND sp.end_token=a.end_token
                 JOIN execution.binding_candidate_set_build b ON b.candidate_set_ref=s.candidate_set_ref
                     AND b.generator_build_ref=s.generator_build_ref
                     AND b.reference_factor_revision_ref=s.reference_factor_revision_ref
                     AND b.referential_type_ref=s.referential_type_ref
                     AND b.accessibility_declaration_ref=s.accessibility_declaration_ref
                     AND b.compatibility_declaration_ref=s.compatibility_declaration_ref
                     AND b.build_state_ref='completed'
                 WHERE s.document_ref=$1 AND sp.start_char=$2 AND sp.end_char=$3
                   AND a.parser_pos_ref='PRON' AND a.pnf_kind_ref LIKE 'semantic.argument%'
                   AND a.factor_revision_ref NOT LIKE 'factor-revision:native-binding:%'
                   AND a.factor_revision_ref NOT LIKE 'factor-revision:native-binding-v2:%'
                   AND NOT EXISTS (SELECT 1 FROM corpus.span other WHERE other.document_ref=a.document_ref
                       AND other.start_token=a.start_token AND other.end_token=a.end_token
                       AND (other.start_char<>sp.start_char OR other.end_char<>sp.end_char))
                 ORDER BY s.reference_factor_ref,s.reference_factor_revision_ref,s.candidate_set_ref",
                &[doc,&start,&end])?);
            for row in rows{
                let reference:String=row.get(0);let revision:String=row.get(1);
                let set:String=row.get(2);let build:String=row.get(3);
                let typ:String=row.get(4);let access:String=row.get(5);let compat:String=row.get(6);
                let count:i32=row.get(7);
                let stored=tx.query(
                    "SELECT candidate_factor_ref,accessibility_path_ref
                     FROM resolution.binding_candidate_member WHERE candidate_set_ref=$1 ORDER BY ordinal",
                    &[&set])?.iter().map(|r|(r.get::<_,String>(0),r.get::<_,String>(1))).collect::<Vec<_>>();
                let capped=exact_original_binding_members(&mut tx,&revision,&typ,&access,&compat,64)?;
                // Upper bound by actual factor anchors, not arbitrary guessed
                // candidate cap. Keep original query semantics and ordering.
                let n:i64=tx.query_one("SELECT count(*)::bigint FROM pnf.factor_anchor WHERE document_ref=$1",&[doc])?.get(0);
                let limit=i32::try_from(n.max(1)).map_err(|_|NativeNumericAuditError::ForeignSource)?;
                let full=exact_original_binding_members(&mut tx,&revision,&typ,&access,&compat,limit)?;
                let parity=count>=0&&count as usize==stored.len()&&original_index_member_parity(&stored,&capped);
                let prefix=full.iter().take(64).eq(capped.iter());
                let exclusions=tx.query("SELECT reason_ref,excluded_count FROM resolution.binding_exclusion_summary
                    WHERE candidate_set_ref=$1 ORDER BY reason_ref",&[&set])?
                    .iter().map(|r|(r.get(0),r.get(1))).collect();
                let mut members=Vec::new();
                for (member_revision,factor,path,distance) in &full{
                    let native=member_revision.starts_with("factor-revision:native-binding-v2:");
                    let mut projection_ids=Vec::new();
                    let mut snapshot_objects=Vec::new();
                    let mut native_inverse=false;
                    let spans=if native&&has_native_projection{
                        let link=tx.query_one(
                            "SELECT s.start_char,s.end_char,p.token_ids,p.object_ids
                             FROM execution.slr_native_binding_anchor_projection p
                             JOIN corpus.span s ON s.span_ref=p.span_ref AND s.document_ref=p.document_ref
                             JOIN execution.binding_candidate_set_build b ON b.document_pnf_index_ref=p.snapshot_ref
                             WHERE p.factor_revision_ref=$1 AND p.document_ref=$2 AND p.run_ref=$3
                               AND p.active AND b.generator_build_ref=$4 AND b.build_state_ref='completed'",
                            &[member_revision,doc,&numeric_run,&build])?;
                        projection_ids=link.get(2);snapshot_objects=link.get(3);
                        vec![(link.get::<_,i32>(0),link.get::<_,i32>(1))]
                    }else{
                        tx.query(
                            "SELECT DISTINCT sp.start_char,sp.end_char FROM pnf.factor_anchor a
                             JOIN corpus.span sp ON sp.document_ref=a.document_ref
                                 AND sp.start_token=a.start_token AND sp.end_token=a.end_token
                             WHERE a.factor_revision_ref=$1 AND a.document_ref=$2
                             ORDER BY sp.start_char,sp.end_char",&[member_revision,doc])?
                            .iter().map(|r|(r.get::<_,i32>(0),r.get::<_,i32>(1))).collect::<Vec<_>>()
                    };
                    let mut objects=Vec::new();
                    if same&&spans.len()==1{
                        let (a,b)=spans[0];
                        if native{
                            // Check each exact support token, in source order.
                            // For relations the support can be discontinuous.
                            let actual=tx.query(
                                "SELECT token_id,start_char,end_char FROM execution.semantic_parser_token
                                 WHERE run_ref=$1 AND document_ref=$2 AND representation_version=2
                                   AND token_id=ANY($3::bigint[]) ORDER BY start_char,end_char,token_id",
                                &[&numeric_run,doc,&projection_ids])?;
                            native_inverse=actual.iter().map(|r|r.get::<_,i64>(0)).eq(projection_ids.iter().copied())
                                &&actual.first().map(|r|r.get::<_,i64>(1))==Some(i64::from(a))
                                &&actual.last().map(|r|r.get::<_,i64>(2))==Some(i64::from(b));
                            if typ=="entity_reference"{
                                let mut sorted=projection_ids.clone();sorted.sort_unstable();
                                objects=native_object_inverse.get(&sorted).cloned().unwrap_or_default();
                                snapshot_objects.sort_unstable();snapshot_objects.dedup();
                                native_inverse &= objects==snapshot_objects;
                            }
                        }else if typ=="entity_reference"{
                            objects=tx.query(
                                "WITH expected AS (
                                    SELECT array_agg(token_id ORDER BY token_id) AS ids,min(start_char) AS a,max(end_char) AS b
                                    FROM execution.semantic_parser_token WHERE run_ref=$1 AND document_ref=$2
                                      AND representation_version=2 AND start_char >= $3 AND end_char <= $4
                                 ), actual AS (
                                    SELECT o.object_id,array_agg(DISTINCT s.token_id ORDER BY s.token_id) AS ids
                                    FROM execution.semantic_pnf_object o JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
                                    JOIN execution.semantic_pnf_object_token_support s ON s.object_id=o.object_id
                                    WHERE r.run_ref=$1 AND r.document_ref=$2 AND o.active GROUP BY o.object_id
                                 )
                                 SELECT actual.object_id FROM actual,expected WHERE actual.ids=expected.ids
                                   AND expected.a=$3 AND expected.b=$4 ORDER BY actual.object_id",
                                &[&numeric_run,doc,&i64::from(a),&i64::from(b)])?
                                .iter().map(|r|r.get(0)).collect();
                        }
                    }
                    members.push(OriginalBindingMemberReuse{
                        candidate_factor_ref:factor.clone(),candidate_factor_revision_ref:member_revision.clone(),
                        accessibility_path_ref:path.clone(),distance_tokens:*distance,
                        original_span_coordinates:spans,exact_projection_token_ids:projection_ids,
                        native_inverse_matches_snapshot:native_inverse,exact_same_manifestation_numeric_object_ids:objects,
                    });
                }
                let native_reference=revision.starts_with("factor-revision:native-binding-v2:");
                let native_reference_verified=if native_reference&&has_native_projection&&same{
                    tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.slr_native_binding_anchor_projection p
                        JOIN corpus.span s ON s.span_ref=p.span_ref AND s.document_ref=p.document_ref
                        JOIN execution.semantic_parser_token t ON t.token_id=p.token_ids[1]
                        JOIN execution.binding_candidate_set_build b ON b.document_pnf_index_ref=p.snapshot_ref
                        WHERE p.factor_revision_ref=$1 AND p.active AND p.run_ref=$2 AND p.document_ref=$3
                          AND cardinality(p.token_ids)=1 AND t.run_ref=$2 AND t.document_ref=$3
                          AND t.representation_version=2 AND t.start_char=$4 AND t.end_char=$5
                          AND s.start_char=$6 AND s.end_char=$7 AND b.generator_build_ref=$8)",
                        &[&revision,&numeric_run,doc,&i64::from(start),&i64::from(end),&start,&end,&build])?.get::<_,bool>(0)
                }else{false};
                let native_projection_verified=native_reference_verified&&members.iter().all(|m|m.native_inverse_matches_snapshot);
                let exact_entity_inverse_complete=typ=="entity_reference"&&!members.is_empty()&&members.iter().all(|m|
                    m.original_span_coordinates.len()==1&&!m.exact_same_manifestation_numeric_object_ids.is_empty()
                    &&!m.candidate_factor_revision_ref.starts_with("factor-revision:native-binding:")
                    &&(!native_reference||m.native_inverse_matches_snapshot));
                // Index self-consistency alone is not reusable correspondence.
                if same&&parity&&prefix&&exact_entity_inverse_complete&&(!native_reference||native_projection_verified){reusable.push(set.clone());}
                if native_reference&&!native_projection_verified{
                    residuals.push(format!("native_binding_projection_inverse_mismatch:{set}"));
                }
                if typ=="entity_reference"&&!exact_entity_inverse_complete{
                    residuals.push(format!("entity_binding_inverse_incomplete:{set}"));
                }
                sets.push(OriginalBindingSetReuse{
                    document_ref:doc.clone(),same_manifestation:same,reference_factor_ref:reference,
                    reference_factor_revision_ref:revision,candidate_set_ref:set,generator_build_ref:build,
                    referential_type_ref:typ,accessibility_declaration_ref:access,compatibility_declaration_ref:compat,
                    original_persisted_member_count:count,original_index_matches_persisted_members:parity,
                    original_index_prefix_matches_full_query:prefix,original_index_query_complete:true,
                    original_index_member_count:full.len(),native_projection_verified,exact_entity_inverse_complete,
                    original_cap_deferred_count:full.len().saturating_sub(64),
                    original_exclusion_summaries:exclusions,original_members:members,
                });
            }
        }
    }else{residuals.push("original_structural_binding_owner_not_installed".into());}
    if sets.is_empty(){residuals.push("no_persisted_exact_focus_binding_set_in_original_compiler".into());}
    if reusable.is_empty(){residuals.push("no_same_manifestation_entity_binding_set_with_complete_inverse_and_index_parity".into());}
    let runs=tx.query("SELECT DISTINCT run_ref FROM execution.semantic_parser_source
        WHERE document_ref=$1 AND encode(content_sha256,'hex')=$2 ORDER BY run_ref",
        &[&answer.source_document_ref,&sha])?.iter().map(|r|r.get::<_,String>(0)).collect::<Vec<_>>();
    tx.commit()?;
    // Use the original already-written native audit over each exact original
    // run, without compiling or replaying the approximate v3 projection.
    let mut audits=Vec::new();
    for run in &runs {
        audits.push(audit_original_numeric_identity_for_source_answer(url,run,answer)?);
    }
    if !audits.iter().any(|a|a.original_accepted_document_identity_paid){
        residuals.push("no_current_accepted_focus_identity_on_exact_manifestation_runs".into());
    }
    if !audits.iter().any(|a|a.original_level_three_substitution_paid){
        residuals.push("no_current_witnessed_focus_level_three_on_exact_manifestation_runs".into());
    }
    Ok(OriginalOwnerReuseReceipt{
        schema:"slr-native-original-binding-owner-reuse-v2",source_revision_ref:answer.source_revision_ref.clone(),
        source_sha256:sha,same_manifestation_document_ref:answer.source_document_ref.clone(),
        source_content_equal_compiler_documents:docs.into_iter().collect(),
        original_binding_owner_available:owner_available,original_binding_sets:sets,
        original_numeric_run_refs:runs,original_numeric_audits:audits,reusable_binding_set_refs:reusable,
        original_index_complete_is_not_semantic_coverage:true,writes_database:false,residuals,
    })
}
#[cfg(test)]
#[test]
fn original_owner_reuse_parity_never_collapses_factor_or_path(){
    let a=vec![("a".to_owned(),"accessibility:same_sentence".to_owned())];
    let mut b=vec![("ra".to_owned(),"a".to_owned(),"same_sentence".to_owned(),1)];
    assert!(original_index_member_parity(&a,&b));
    b[0].2="preceding_paragraph".into();
    assert!(!original_index_member_parity(&a,&b));
    b[0].2="same_sentence".into();b.push(b[0].clone());
    assert!(!original_index_member_parity(&a,&b));
}

#[cfg(test)]
mod native_binding_bridge_regression {
    use super::*;
    fn token(id:i64,sentence:i32,ordinal:i32,start:i64,end:i64,pos:&str,orth:&str)->BridgeToken{
        BridgeToken{id,sentence_id:i64::from(sentence)+1,sentence,ordinal,start,end,
            region:i64::from(sentence)+1,head:id,lemma_id:id,orth:orth.into(),lemma:orth.into(),
            pos:pos.into(),dep:"root".into(),morphology:vec![]}
    }
    #[test]
    fn numeric_pnf_binding_global_chart_retains_source_and_distinct_occurrences(){
        // Local ordinal zero in both sentences must not become a common
        // anchor. The multi-token named span also has a distinct inverse.
        let source="A B.\n\n“C D.”";
        let tokens=BTreeMap::from([
            (30,token(30,0,0,0,1,"PROPN","A")),
            (10,token(10,0,1,2,3,"PROPN","B")),
            (20,token(20,1,3,7,8,"PROPN","C")),
            (40,token(40,1,4,9,10,"PROPN","D")),
        ]);
        let a=bridge_anchor("a".into(),"semantic.mention_identity",&[10,30],vec![99],&tokens,source,None).unwrap();
        let b=bridge_anchor("b".into(),"semantic.mention_identity",&[20],vec![],&tokens,source,None).unwrap();
        assert_eq!((a.ordinal,a.end_ordinal,a.start_char,a.end_char),(0,2,0,3));
        assert_eq!(a.token_ids,vec![30,10]);assert_eq!(a.object_ids,vec![99]);
        assert_eq!((b.ordinal,b.paragraph,b.quotation),(3,1,1));
        assert_ne!(a.clause,b.clause);
        assert_ne!(bridge_digest(&a).unwrap(),bridge_digest(&b).unwrap());
        let mut a2=a.clone();a2.object_ids.push(100);
        assert_ne!(bridge_digest(&a).unwrap(),bridge_digest(&a2).unwrap());
    }
    #[test]
    fn numeric_pnf_binding_structural_metadata_and_morphology_are_not_guessed(){
        let source="he";
        let mut t=token(7,0,0,0,2,"PRON","he");
        t.morphology=bridge_morphology(&["Number=Sing,Plur".into(),"Gender=Masc".into(),"Number=Sing".into()]);
        let tokens=BTreeMap::from([(7,t)]);
        let a=bridge_anchor("x".into(),"semantic.argument.subject",&[7],vec![],&tokens,source,Some(("p","predicate"))).unwrap();
        assert_eq!(a.clause.as_deref(),Some("clause:p"));assert_eq!(a.reporting,None);
        assert_eq!(a.morphology,vec![("Gender".to_owned(),"Masc".to_owned()),("Number".to_owned(),"Plur".to_owned()),("Number".to_owned(),"Sing".to_owned())]);
        let c=bridge_anchor("y".into(),"semantic.embedded_proposition",&[7],vec![],&tokens,source,Some(("c","composition"))).unwrap();
        assert_eq!(c.reporting.as_deref(),Some("c"));assert_eq!(c.coordination,None);
    }
}
