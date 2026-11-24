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
#[derive(Debug,Clone,Serialize)]
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
    let original_h6_evidence_inserted=run_original_h6_producer(&mut tx,demand,numeric_run_ref,&answer.source_document_ref)?;
    let (original_horizon_observations,original_document_identity_observations)=
        observe_original_resolution_fibre(&mut tx,demand,numeric_run_ref,&answer.source_document_ref)?;
    let document_identity_fibre=project_original_identity_fibre(&original_horizon_observations,&original_document_identity_observations,open);
    let original_cross_resource_needs=observe_original_cross_resource_needs(&mut tx,demand,numeric_run_ref,&answer.source_document_ref)?;
    let original_parser_identity_producer=original_parser_identity_producer_probe(
        &mut tx,numeric_run_ref,&answer.source_document_ref,demand)?;
    tx.commit()?;
    Ok(NativeNumericCandidateTransportReceipt{
        schema:"slr-native-candidate-only-transport-v1",
        numeric_run_ref:numeric_run_ref.into(),demand_id:demand,source_object_id:source,
        inserted_candidate_ids:ids,remaining_untransported_count:remaining,
        represented_candidate_count:count,outside_model_possible:open,
        complete_typed_coverage_paid:false,
        nominal_head_objects_inserted,nominal_head_token_supports_inserted,nominal_heads_remaining,original_h6_evidence_inserted,
        morphology_observations_inserted,candidates_without_morphology_observation,
        original_horizon_observations,original_document_identity_observations,
        original_parser_identity_producer,document_identity_fibre,original_cross_resource_needs,
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
    fn unsupported_scope_refused_before_any_sql(){
        // Without an independently licensed original source-local receipt
        // no local username/surname equivalence can be minted in numeric PNF.
        assert_eq!(hex(&[0u8,255u8]),"00ff");
    }
}
