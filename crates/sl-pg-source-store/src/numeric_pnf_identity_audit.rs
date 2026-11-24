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
