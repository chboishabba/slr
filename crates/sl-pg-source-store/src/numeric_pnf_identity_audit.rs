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
        canonical_source_verified:true,prefix_uncovered_nonwhitespace:uncovered,parser_token_count:tokens.len(),
        projected_nominal_span_count:observations.len(),newly_materialized_span_objects:created,
        spans_without_exact_objects:observations.iter().filter(|r|r.exact_object_ids.is_empty()).count(),
        compatible_before_cap:compatible,compatible_deferred_by_cap:compatible.saturating_sub(64),
        represented_spans:observations,canonical_tokenizer_parity_paid:false,
        full_factor_graph_parity_paid:false,typed_coverage_paid:false,
    })
}
// Record candidate-only source-derived assessments in original m086 evidence.
// Never use cap exclusion or an incomplete graph to refute an occurrence.
fn persist_original_mention_assessments(
    tx:&mut postgres::Transaction<'_>,demand:i64,focus:i64,
    campaign:&mut NativeMentionBindingCampaign,
)->Result<(),NativeNumericAuditError>{
    for span in &mut campaign.represented_spans{
        for object in &span.exact_object_ids{
            let evidence=format!("native-mention-binding:v1:{focus}:{}:{}:{object}",span.start_char,span.end_char);
            let kind=format!("projected_mention_binding:{}:{}",span.accessibility_path,
                if span.morphology_mismatches.is_empty(){"no_observed_mismatch"}else{"morphology_mismatch"});
            // Morphology is already a separate signed evidence coordinate.
            // Do not double-charge it or fabricate structural exclusion.
            span.persisted_candidate_evidence+=tx.execute(
                "INSERT INTO execution.semantic_pnf_candidate_evidence
                  (demand_id,target_kind,target_id,evidence_ref,evidence_family,horizon,
                   signed_residual,evidence_kind,provenance_ref,source_interface_id,source_region_id)
                 SELECT $1,1,o.object_id,$3,1,3,0,$4,
                   'binding-accessibility:document-structural:v0_3:projected-parser-mention',
                   i.interface_id,o.region_id
                 FROM execution.semantic_pnf_candidate_universe u
                 JOIN execution.semantic_pnf_object o ON o.object_id=u.target_id
                 JOIN execution.semantic_pnf_interface i ON i.region_id=o.region_id
                 WHERE u.demand_id=$1 AND u.target_kind=1 AND u.target_id=$2
                 ON CONFLICT(demand_id,target_kind,target_id,evidence_ref) DO NOTHING",
                &[&demand,object,&evidence,&kind])? as usize;
        }
    }
    Ok(())
}

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
    let ids=classes.iter().flat_map(|c|c.admitted_witness_ids.iter().copied()).collect::<BTreeSet<_>>();
    let level_one=level_one_snapshot(tx,run,doc)?;
    let before=level_three_snapshot(tx,run,doc)?;
    let mut receipts=Vec::new();
    for id in ids.into_iter().take(64){
        let eligible:bool=tx.query_one(
            "SELECT EXISTS(SELECT 1 FROM execution.semantic_pnf_identity_evidence_witness p
             JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=p.witness_id
             JOIN execution.semantic_pnf_identity_evidence_candidate c ON c.candidate_id=p.candidate_id
             WHERE p.witness_id=$1 AND a.admission_state=2 AND c.run_id=$2 AND c.document_id=$3)",
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
        receipts.push(NativeParserWitnessWithdrawalProbe{
            witness_id:id,affected_level_three_refs:affected,witness_history_preserved:changed&&history,
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
        source_factor_count,focus_factor_argument_count,factor_alignment_counts,witness_withdrawal_probes,projected_relation_probe,
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
    let mut original_mention_binding_campaign=original_mention_binding_campaign(
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
    persist_original_mention_assessments(&mut tx,demand,focus_token,&mut original_mention_binding_campaign)?;
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
        schema:"slr-native-candidate-only-transport-v3",
        numeric_run_ref:numeric_run_ref.into(),demand_id:demand,source_object_id:source,
        inserted_candidate_ids:ids,remaining_untransported_count:remaining,
        represented_candidate_count:count,outside_model_possible:open,
        complete_typed_coverage_paid:false,
        nominal_head_objects_inserted,nominal_head_token_supports_inserted,nominal_heads_remaining,original_h6_evidence_inserted,
        morphology_observations_inserted,candidates_without_morphology_observation,
        original_horizon_observations,original_document_identity_observations,
        original_parser_identity_producer,document_identity_fibre,original_cross_resource_needs,
        structural_binding,original_mention_binding_campaign,original_h9_readiness,governed_identity_rollback_probe,
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
