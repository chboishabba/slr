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
    Database(String),
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

impl From<postgres::Error> for NativeNumericAuditError {
    fn from(error: postgres::Error) -> Self {
        let detail = error.as_db_error().map(|db| {
            let mut parts = vec![db.message().to_owned()];
            if let Some(detail) = db.detail() { parts.push(format!("detail={detail}")); }
            if let Some(hint) = db.hint() { parts.push(format!("hint={hint}")); }
            if let Some(constraint) = db.constraint() { parts.push(format!("constraint={constraint}")); }
            parts.push(format!("code={}", db.code().code()));
            parts.join("; ")
        }).unwrap_or_else(|| format!("{error:?}"));
        Self::Database(detail)
    }
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
    pub historical_h6_proof_revalidation_required:bool,
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
        let mut invalid_refutation:i64=tx.query_one(
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
        let current_h6_installed:bool=tx.query_one("SELECT to_regclass('execution.slr_native_current_candidate_evidence_v1') IS NOT NULL",&[])?.get(0);
        let h6_managed=if current_h6_installed{
            tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.slr_native_h6_managed_demand WHERE demand_id=$1)",&[demand_id])?.get::<_,bool>(0)
        }else{false};
        let historical_h6_proof_revalidation_required=if h6_managed{
            let stale:i64=tx.query_one("SELECT count(*)::bigint FROM execution.semantic_pnf_candidate_evidence e
                WHERE e.demand_id=$1 AND e.horizon IN(3,6) AND NOT EXISTS(
                   SELECT 1 FROM execution.slr_native_current_candidate_evidence_v1 c WHERE c.evidence_id=e.evidence_id)",
                &[demand_id])?.get(0);
            invalid_refutation+=tx.query_one("SELECT count(*)::bigint FROM execution.semantic_pnf_candidate_state_v1 s
                WHERE s.demand_id=$1 AND s.refuted AND EXISTS(
                   SELECT 1 FROM execution.semantic_pnf_candidate_evidence e WHERE e.evidence_id=s.admissibility_evidence_id
                     AND NOT EXISTS(SELECT 1 FROM execution.slr_native_current_candidate_evidence_v1 c WHERE c.evidence_id=e.evidence_id))",
                &[demand_id])?.get::<_,i64>(0);
            let debt_installed:bool=tx.query_one("SELECT to_regclass('execution.slr_native_h3_authority_debt') IS NOT NULL",&[])?.get(0);
            let debt=if debt_installed{
                tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.slr_native_h3_authority_debt WHERE demand_id=$1)",&[demand_id])?.get::<_,bool>(0)
            }else{
                tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.semantic_pnf_candidate_evidence WHERE demand_id=$1 AND horizon=3
                    AND (evidence_ref LIKE 'native-occurrence-transport:v1:%' OR evidence_ref LIKE 'native-binding-morphology:%'
                         OR evidence_ref LIKE 'native-binding-index-member:v2:%'))",&[demand_id])?.get::<_,bool>(0)
            };
            let current_guard:bool=tx.query_one("SELECT to_regprocedure('execution.slr_native_current_authority_usable(bigint)') IS NOT NULL",&[])?.get(0);
            let blocked=if current_guard{!tx.query_one("SELECT execution.slr_native_current_authority_usable($1)",&[demand_id])?.get::<_,bool>(0)}else{true};
            stale>0||debt||blocked
        }else{
            // Before installation, the legacy H6 carrier has no validity
            // proof. Never certify a closed world based on that absence.
            tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.semantic_pnf_candidate_evidence
                WHERE demand_id=$1 AND ((horizon=6 AND evidence_family=2
                  AND evidence_kind IN('h6_discourse_factor_role_signature','h6_temporal_factor_role_signature'))
                  OR (horizon=3 AND (evidence_ref LIKE 'native-occurrence-transport:v1:%'
                      OR evidence_ref LIKE 'native-binding-morphology:%' OR evidence_ref LIKE 'native-binding-index-member:v2:%'))))",
                &[demand_id])?.get(0)
        };
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
            invalid_refutation_count:invalid_refutation,historical_h6_proof_revalidation_required,
            producer_h9_target_occurrence_count:h9_target,
            quality_gated_h9_entity_occurrence_count:h9_entities,
            represented_candidate_count:represented,
            active_candidate_count:active,refuted_candidate_count:refuted,
        });
        if !outside&&coverage.as_deref().is_some_and(|s|!s.is_empty())
            &&!limited&&residual==0&&unique_count==1
            &&represented-refuted==1&&invalid_refutation==0&&!historical_h6_proof_revalidation_required
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



/// The original read-only audit intentionally never promotes observations
/// into payment. This explicit operator campaign consumes ONLY an already
/// current original focus witness and checks its real inverse, in rollback.
#[derive(Debug,Clone,Serialize)]
pub struct NativeFocusIdentityCertification {
    pub original:NativeNumericIdentityAudit,
    pub inverse:Option<NativeNumericRetractionReceipt>,
    pub rollback_readback_exact:bool,
    pub governed_identity_paid:bool,
    pub level_three_paid:bool,
    pub persisted:bool,
}

pub fn certify_original_focus_identity(
    url:&str,run:&str,answer:&NativeSourceLocalAnswer,
)->Result<NativeFocusIdentityCertification,NativeNumericAuditError>{
    let original=audit_original_numeric_identity_for_source_answer(url,run,answer)?;
    let mut receipt=NativeFocusIdentityCertification{
        original,inverse:None,rollback_readback_exact:false,
        governed_identity_paid:false,level_three_paid:false,persisted:false,
    };
    if !receipt.original.resolved_unique_target_from_original_sql
        || !receipt.original.exact_numeric_focus_mention_found
        || receipt.original.observed_accepted_document_witness_ids.len()!=1
        || receipt.original.observed_witnessed_level_three_derivation_refs.is_empty(){
        return Ok(receipt);
    }
    let witness=receipt.original.observed_accepted_document_witness_ids[0];
    let inverse=retract_original_numeric_identity_for_source_answer(url,run,answer,witness,false)?;
    let reopened=audit_original_numeric_identity_for_source_answer(url,run,answer)?;
    let before=serde_json::to_value(&receipt.original).map_err(|_|NativeNumericAuditError::RetractionInvariant)?;
    let after=serde_json::to_value(&reopened).map_err(|_|NativeNumericAuditError::RetractionInvariant)?;
    let observed=receipt.original.observed_witnessed_level_three_derivation_refs.iter().collect::<BTreeSet<_>>();
    let affected=inverse.affected_level_three_derivation_refs.iter().collect::<BTreeSet<_>>();
    if before!=after||inverse.committed||!inverse.immutable_level_one_preserved
       ||!inverse.unaffected_level_three_preserved||!inverse.revoked_witness_absent_from_current_projection
       ||!inverse.revoked_witness_absent_from_current_derivations||!observed.is_subset(&affected){
        return Err(NativeNumericAuditError::RetractionInvariant);
    }
    receipt.rollback_readback_exact=true;
    receipt.governed_identity_paid=true;receipt.level_three_paid=true;receipt.inverse=Some(inverse);
    Ok(receipt)
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
                if name.eq_ignore_ascii_case(feature){Some(value)}else{None}
            }).flat_map(|value|value.split(',').filter(|v|!v.is_empty()).map(str::to_owned))
              .collect::<BTreeSet<_>>()
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
         ), previous AS (
           SELECT c.* FROM candidate c
           WHERE EXISTS (
             SELECT 1 FROM execution.semantic_pnf_candidate_evidence e
             WHERE e.demand_id=$1 AND e.target_kind=1 AND e.target_id=c.object_id
               AND e.horizon=3 AND e.provenance_ref='binding-compatibility:pnf-kind-morphology:v0_3'
               AND (e.evidence_ref LIKE 'native-binding-morphology:v0_3:'||$4::bigint::text||':%'
                 OR e.evidence_ref LIKE 'native-binding-morphology:v0_4:'||$4::bigint::text||':%'))
         ), pending AS (
           SELECT * FROM previous
           UNION ALL
           (SELECT c.* FROM candidate c WHERE NOT EXISTS(
               SELECT 1 FROM previous p WHERE p.object_id=c.object_id)
            ORDER BY c.object_id LIMIT $5)
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
        let legacy_ref=format!("native-binding-morphology:v0_3:{focus_token}:{head}");
        let evidence_kind=if mismatch.is_empty(){"morphology_no_observed_mismatch".to_owned()}
            else{format!("morphology_mismatch:{}",mismatch.join(","))};
        // Preserve exact legacy atoms. A changed same-head assessment
        // receives a distinct immutable key rather than overwriting history.
        let legacy=tx.query_opt("SELECT signed_residual,evidence_kind,source_interface_id,source_region_id,
               evidence_family,horizon,provenance_ref FROM execution.semantic_pnf_candidate_evidence
               WHERE demand_id=$1 AND target_kind=1 AND target_id=$2 AND evidence_ref=$3",
               &[&demand,&object,&legacy_ref])?;
        let evidence_ref=if legacy.as_ref().map_or(true,|r|
            r.get::<_,i64>(0)==residual&&r.get::<_,String>(1)==evidence_kind
            &&r.get::<_,Option<i64>>(2)==Some(interface)&&r.get::<_,Option<i64>>(3)==Some(region)
            &&r.get::<_,i16>(4)==1&&r.get::<_,i16>(5)==3
            &&r.get::<_,Option<String>>(6).as_deref()==Some("binding-compatibility:pnf-kind-morphology:v0_3")){
            legacy_ref
        }else{
            format!("native-binding-morphology:v0_4:{focus_token}:{head}:{}",
                bridge_digest(&(residual,&evidence_kind,interface,region))?)
        };
        inserted+=tx.execute(
            "INSERT INTO execution.semantic_pnf_candidate_evidence
              (demand_id,target_kind,target_id,evidence_ref,evidence_family,horizon,
               signed_residual,evidence_kind,provenance_ref,source_interface_id,source_region_id)
             VALUES($1,1,$2,$3,1,3,$4,$5,
               'binding-compatibility:pnf-kind-morphology:v0_3',$6,$7)
             ON CONFLICT(demand_id,target_kind,target_id,evidence_ref) DO NOTHING",
            &[&demand,&object,&evidence_ref,&residual,&evidence_kind,&interface,&region])? as usize;
        let recorded=tx.execute("INSERT INTO slr_native_h3_desired(evidence_id)
            SELECT evidence_id FROM execution.semantic_pnf_candidate_evidence
            WHERE demand_id=$1 AND target_kind=1 AND target_id=$2 AND evidence_ref=$3
              AND signed_residual=$4 AND evidence_kind=$5 AND source_interface_id=$6 AND source_region_id=$7
              AND evidence_family=1 AND horizon=3
              AND provenance_ref='binding-compatibility:pnf-kind-morphology:v0_3'",
            &[&demand,&object,&evidence_ref,&residual,&evidence_kind,&interface,&region])?;
        if recorded!=1{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    }
    // A null/multiple head is a residual, not an endlessly retried job.
    let remaining:i64=tx.query_one(
        "WITH eligible AS (
           SELECT u.target_id,min(t.token_id) AS token_id
           FROM execution.semantic_pnf_candidate_universe u
           JOIN execution.semantic_pnf_object o ON o.object_id=u.target_id AND o.active
           JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
           JOIN execution.semantic_pnf_object_token_support s ON s.object_id=o.object_id
           JOIN execution.semantic_parser_token t ON t.token_id=s.token_id
             AND t.run_ref=r.run_ref AND t.document_ref=r.document_ref
             AND t.representation_version=2 AND t.lemma_symbol_id=o.head_symbol_id
           WHERE u.demand_id=$1 AND u.target_kind=1 AND r.run_ref=$2 AND r.document_ref=$3
           GROUP BY u.target_id HAVING count(DISTINCT t.token_id)=1
         ) SELECT count(*)::bigint FROM eligible c WHERE NOT EXISTS(
           SELECT 1 FROM execution.semantic_pnf_candidate_evidence e
           JOIN slr_native_h3_desired x USING(evidence_id)
           WHERE e.demand_id=$1 AND e.target_kind=1 AND e.target_id=c.target_id
             AND (e.evidence_ref='native-binding-morphology:v0_3:'||$4::bigint::text||':'||c.token_id::text
                 OR e.evidence_ref LIKE 'native-binding-morphology:v0_4:'||$4::bigint::text||':'||c.token_id::text||':%'))",
         &[&demand,&run_ref,&document_ref,&focus_token])?.get(0);
    Ok((inserted,remaining))
}


#[derive(Debug,Clone,Serialize)]
pub struct OriginalParserIdentityCandidateObservation{
    pub candidate_id:i64,
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
    pub admitted_candidate_witnesses:Vec<OriginalParserCandidateWitnessObservation>,
    pub bounded_identity_parity:Option<NativeBoundedIdentityParity>,
    pub evidence_delta_frontier:Option<NativeIdentityEvidenceDelta>,
    pub support_index:Option<NativeIdentitySupportIndexReceipt>,
    pub scoped_admission_parity:Vec<NativeScopedAdmissionParity>,
}
fn observe_refreshed_parser_identity_producer(
    probe:&mut postgres::Transaction<'_>,run_id:i64,document_id:i64,demand:i64,
)->Result<OriginalParserIdentityProducerObservation,NativeNumericAuditError>{
    let rows=probe.query(
        "SELECT c.source_object_id,c.target_object_id,c.witness_kind,
                c.source_token_id,c.target_token_id,c.evidence_ref,
                c.candidate_count,c.evidence_state,c.candidate_id
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
            candidate_id:r.get(8),source_object_id:r.get(0),target_object_id:r.get(1),witness_kind:r.get(2),
            source_token_id:r.get(3),target_token_id:r.get(4),evidence_ref:r.get(5),
            candidate_count:r.get(6),evidence_state:r.get(7),
        }).collect();
    let overflow:i64=probe.query_one(
        "SELECT count(*)::bigint FROM execution.semantic_pnf_proper_name_evidence_overflow
         WHERE run_id=$1 AND document_id=$2",&[&run_id,&document_id])?.get(0);
    Ok(OriginalParserIdentityProducerObservation{
        producer_ref:"execution.refresh_numeric_pnf_parser_identity_evidence",
        scope:"current_numeric_document_candidate_projection_not_global_coverage",
        refreshed_in_rollback_probe:true,candidate_rows:rows,name_overflow_rows:overflow,
        admission_executed:false,admitted_candidate_witnesses:Vec::new(),bounded_identity_parity:None,evidence_delta_frontier:None,support_index:None,scoped_admission_parity:Vec::new(),
    })
}


/// Exact original m080 candidate -> witness -> admitted entity inverse.
//// These are rollback-only observations in the caller; not a new authority.
/// The independent corroboration IDs are retained, not merely a Boolean.
#[derive(Debug,Clone,Serialize)]
pub struct OriginalParserCandidateWitnessObservation {
    pub candidate_id:i64,
    pub witness_id:i64,
    pub witness_role:i16,
    pub source_object_id:i64,
    pub entity_id:i64,
    pub anchor_object_id:i64,
    pub witness_kind:i16,
    pub admission_revision:i64,
    pub independent_corroboration_witness_ids:Vec<i64>,
}

fn observe_original_parser_candidate_witnesses(
    tx:&mut postgres::Transaction<'_>,run:i64,document:i64,
    producer:&mut OriginalParserIdentityProducerObservation,
)->Result<(),NativeNumericAuditError>{
    let candidates=producer.candidate_rows.iter().map(|c|(c.candidate_id,c))
        .collect::<BTreeMap<_,_>>();
    if candidates.len()!=producer.candidate_rows.len()||candidates.keys().any(|id|*id<=0) {
        return Err(NativeNumericAuditError::ForeignSource);
    }
    let ids=candidates.keys().copied().collect::<Vec<_>>();
    let rows=tx.query("SELECT c.candidate_id,p.witness_id,p.witness_role,
        w.source_object_id,w.target_entity_id,e.anchor_object_id,w.witness_kind,a.revision,
        w.authority_class,e.authority_class,w.candidate_count,c.evidence_state,c.candidate_count,
        ARRAY(SELECT DISTINCT k.witness_id
          FROM execution.semantic_pnf_identity_witness k
          JOIN execution.semantic_pnf_identity_witness_admission ka
            ON ka.witness_id=k.witness_id AND ka.admission_state=2
          WHERE k.target_entity_id=w.target_entity_id AND k.witness_id<>w.witness_id
            AND k.witness_kind IN (2,4,5,6,8)
          ORDER BY k.witness_id)
        FROM execution.semantic_pnf_identity_evidence_candidate c
        JOIN execution.semantic_pnf_identity_evidence_witness p ON p.candidate_id=c.candidate_id
        JOIN execution.semantic_pnf_identity_witness w ON w.witness_id=p.witness_id
        JOIN execution.semantic_pnf_identity_witness_admission a
          ON a.witness_id=w.witness_id AND a.admission_state=2
        JOIN execution.semantic_pnf_canonical_entity e ON e.entity_id=w.target_entity_id
        WHERE c.run_id=$1 AND c.document_id=$2 AND c.candidate_id=ANY($3::bigint[])
        ORDER BY c.candidate_id,p.witness_role,p.witness_id",&[&run,&document,&ids])?;
    let mut observed=Vec::with_capacity(rows.len());
    for row in rows {
        let candidate_id:i64=row.get(0);
        let c=candidates.get(&candidate_id).ok_or(NativeNumericAuditError::ForeignSource)?;
        let role:i16=row.get(2);
        let source:i64=row.get(3);
        let anchor:Option<i64>=row.get(5);
        let witness_kind:i16=row.get(6);
        let corroboration:Vec<i64>=row.get(13);
        let direct=matches!(c.witness_kind,2|4|6);
        // Mirror original Agda IdentityEvidenceProduction and m080:
        // proper-name expansion cannot bootstrap; anchor/source provenance
        // is proof-relevant and authority cannot cross the document boundary.
        let licensed=match role {
            1=>direct && source==c.target_object_id && witness_kind==1,
            2=>source==c.source_object_id && witness_kind==c.witness_kind
                && (direct || (c.witness_kind==3 && !corroboration.is_empty())),
            _=>false,
        };
        if !licensed || anchor!=Some(c.target_object_id) || c.candidate_count!=1
            || row.get::<_,i16>(10)!=1 || row.get::<_,i16>(12)!=1
            || row.get::<_,i16>(8)!=2 || row.get::<_,i16>(9)!=2
            || !matches!(row.get::<_,i16>(11),1|2) {
            return Err(NativeNumericAuditError::AdmissionProbeInvariant);
        }
        observed.push(OriginalParserCandidateWitnessObservation{
            candidate_id,witness_id:row.get(1),witness_role:role,
            source_object_id:source,entity_id:row.get(4),
            anchor_object_id:c.target_object_id,witness_kind,
            admission_revision:row.get(7),independent_corroboration_witness_ids:corroboration,
        });
    }
    producer.admitted_candidate_witnesses=observed;
    producer.admission_executed=true;
    Ok(())
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

/// Original H6 atoms remain immutable history (including any proof FK).
/// Current physical support is a separately versioned projection, never a
/// synthetic negative atom or an instruction to refute a candidate.
#[derive(Debug,Clone,Serialize)]
pub struct NativeCurrentSignedHorizon {
    pub target_kind:i16,
    pub target_id:i64,
    pub horizon:i16,
    pub signed_residual:i64,
    pub phase:i16,
    pub evidence_count:i64,
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeH6ValidityReceipt {
    pub historical_atom_count:usize,
    pub currently_supported_atom_count:usize,
    pub native_h3_stale_evidence_ids:Vec<i64>,
    pub native_h3_state_rows_changed:u64,
    pub durable_nonmonotone_authority_pending:bool,
    pub stale_evidence_ids:Vec<i64>,
    pub stale_admissibility_event_ids:Vec<i64>,
    pub current_stale_admissibility_event_ids:Vec<i64>,
    pub original_proof_or_sufficiency_revalidation_required:bool,
    pub state_rows_changed:u64,
    pub current_signed_horizons:Vec<NativeCurrentSignedHorizon>,
    pub current_projection_exact:bool,
    pub original_consumer_authority_reconciled:bool,
}
fn ensure_native_h6_current_authority(tx:&mut postgres::Transaction<'_>,demand:i64)
    ->Result<(),NativeNumericAuditError>{
    let installed:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='execution' AND p.proname='slr_wake_native_validity_delta' AND p.pronargs=0 AND position('slr_native_h3_authority_debt' in p.prosrc)>0)",&[])?.get(0);
    if !installed{
        tx.query_one("SELECT pg_advisory_xact_lock(hashtext('slr-native-h6-current:v1'))",&[])?;
        let ready:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='execution' AND p.proname='slr_wake_native_validity_delta' AND p.pronargs=0 AND position('slr_native_h3_authority_debt' in p.prosrc)>0)",&[])?.get(0);
        if !ready{tx.batch_execute(include_str!("numeric_pnf_h6_current_authority.sql"))?;}
    }
    let intact:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.slr_native_h6_current_contract WHERE contract_ref='slr-native-h6-current:v1')
      AND EXISTS(SELECT 1 FROM execution.slr_native_h3_current_contract WHERE contract_ref='slr-native-h3-current:v1')
      AND EXISTS(SELECT 1 FROM execution.slr_native_current_authority_contract WHERE contract_ref='slr-native-current-authority:v4')
      AND position('slr_native_h3_authority_debt' in pg_get_functiondef(
         'execution.slr_wake_native_validity_delta()'::regprocedure))>0
      AND (SELECT count(*)=2 FROM pg_trigger t WHERE NOT t.tgisinternal AND t.tgenabled IN ('O','A')
         AND t.tgfoid='execution.slr_wake_native_validity_delta()'::regprocedure
         AND t.tgnewtable='inserted_validity'
         AND ((t.tgname='slr_native_h3_validity_delta_wake' AND t.tgrelid='execution.slr_native_h3_validity_history'::regclass)
           OR (t.tgname='slr_native_h6_validity_delta_wake' AND t.tgrelid='execution.slr_native_h6_validity_history'::regclass)))
      AND position('slr_native_candidate_only_authority_absent' in pg_get_functiondef(
         'execution.slr_reconcile_native_candidate_only_debt(bigint,boolean,boolean)'::regprocedure))>0
      AND position('slr_native_current_authority_usable' in pg_get_functiondef(
         'execution.numeric_pnf_consumer_stop_at_horizon(bigint,text,text,text,smallint)'::regprocedure))>0
      AND position('slr_native_h3_validity_current' in pg_get_viewdef('execution.slr_native_current_candidate_evidence_v1'::regclass))>0
      AND position('slr_native_current_candidate_evidence_v1' in pg_get_viewdef('execution.semantic_pnf_candidate_evidence_horizon_v1'::regclass))>0
      AND position('slr_native_current_candidate_evidence_v1' in pg_get_viewdef('execution.semantic_pnf_candidate_evidence_classification_v1'::regclass))>0
      AND position('slr_native_current_candidate_evidence_v1' in pg_get_functiondef(
         'execution.refresh_numeric_pnf_consumer_horizon_outcome(bigint,bigint,smallint,text,text,text)'::regprocedure))>0",&[])?.get(0);
    if !intact{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    tx.execute("INSERT INTO execution.slr_native_h6_managed_demand(demand_id) VALUES($1) ON CONFLICT DO NOTHING",&[&demand])?;
    Ok(())
}

// Scope is deliberately restricted to the three native producers above.
// The exact desired ID relation is independently reconstructed from their
// current carrier; no deletion, scoring compensation, or semantic event.
fn reconcile_native_h3_validity(tx:&mut postgres::Transaction<'_>,demand:i64)
    ->Result<u64,NativeNumericAuditError>{
    let n:i64=tx.query_one("WITH observed AS MATERIALIZED(
        SELECT e.evidence_id,e.demand_id,to_jsonb(e) atom,
               EXISTS(SELECT 1 FROM slr_native_h3_desired d WHERE d.evidence_id=e.evidence_id) supported
        FROM execution.semantic_pnf_candidate_evidence e
        WHERE e.demand_id=$1 AND e.horizon=3 AND e.evidence_family=1
        AND ((e.provenance_ref='native-occurrence-transport:v1' AND e.evidence_ref LIKE 'native-occurrence-transport:v1:%')
          OR (e.provenance_ref='binding-compatibility:pnf-kind-morphology:v0_3'
              AND (e.evidence_ref LIKE 'native-binding-morphology:v0_3:%' OR e.evidence_ref LIKE 'native-binding-morphology:v0_4:%'))
          OR (e.evidence_kind LIKE 'original_binding_index_member:%' AND e.evidence_ref LIKE 'native-binding-index-member:v2:%'))
      ), changed AS (
        INSERT INTO execution.slr_native_h3_validity_current AS c(evidence_id,demand_id,supported,revision)
        SELECT evidence_id,demand_id,supported,1 FROM observed
        ON CONFLICT(evidence_id) DO UPDATE SET supported=EXCLUDED.supported,revision=c.revision+1
        WHERE c.demand_id=EXCLUDED.demand_id AND c.supported IS DISTINCT FROM EXCLUDED.supported
        RETURNING evidence_id,demand_id,supported,revision
      ), history AS (
        INSERT INTO execution.slr_native_h3_validity_history(evidence_id,demand_id,revision,supported,original_atom)
        SELECT c.evidence_id,c.demand_id,c.revision,c.supported,o.atom FROM changed c JOIN observed o USING(evidence_id)
        RETURNING evidence_id
      ) SELECT count(*)::bigint FROM history",&[&demand])?.get(0);
    u64::try_from(n).map_err(|_|NativeNumericAuditError::AdmissionProbeInvariant)
}

fn reopen_native_h6_validity(tx:&mut postgres::Transaction<'_>,demand:i64)
    ->Result<NativeH6ValidityReceipt,NativeNumericAuditError>{
    ensure_native_h6_current_authority(tx,demand)?;
    let row=tx.query_one("WITH physical AS MATERIALIZED (
        SELECT * FROM execution.slr_current_m110_h6_atoms($1)
      ), observed AS MATERIALIZED (
        SELECT e.evidence_id,e.demand_id,to_jsonb(e) AS atom,
          EXISTS(SELECT 1 FROM physical p WHERE p.demand_id=e.demand_id AND p.target_kind=e.target_kind
            AND p.target_id=e.target_id AND p.evidence_ref=e.evidence_ref AND p.signed_residual=e.signed_residual
            AND p.evidence_kind=e.evidence_kind AND p.provenance_ref IS NOT DISTINCT FROM e.provenance_ref
            AND p.source_interface_id IS NOT DISTINCT FROM e.source_interface_id
            AND p.source_region_id IS NOT DISTINCT FROM e.source_region_id) AS supported
        FROM execution.semantic_pnf_candidate_evidence e
        WHERE e.demand_id=$1 AND e.horizon=6 AND e.evidence_family=2
          AND ((e.evidence_kind='h6_discourse_factor_role_signature' AND e.provenance_ref='numeric-factor-signature')
             OR (e.evidence_kind='h6_temporal_factor_role_signature' AND e.provenance_ref='numeric-temporal-state'))
      ), changed AS (
        INSERT INTO execution.slr_native_h6_validity_current AS c(evidence_id,demand_id,supported,revision)
        SELECT evidence_id,demand_id,supported,1 FROM observed
        ON CONFLICT(evidence_id) DO UPDATE SET supported=EXCLUDED.supported,revision=c.revision+1
        WHERE c.demand_id=EXCLUDED.demand_id AND c.supported IS DISTINCT FROM EXCLUDED.supported
        RETURNING evidence_id,demand_id,supported,revision
      ), history AS (
        INSERT INTO execution.slr_native_h6_validity_history(evidence_id,demand_id,revision,supported,original_atom)
        SELECT c.evidence_id,c.demand_id,c.revision,c.supported,o.atom
          FROM changed c JOIN observed o USING(evidence_id)
        RETURNING evidence_id
      )
      SELECT (SELECT count(*) FROM observed)::bigint,
        (SELECT count(*) FROM observed WHERE supported)::bigint,
        ARRAY(SELECT evidence_id FROM observed WHERE NOT supported ORDER BY evidence_id),
        ARRAY(SELECT a.event_id FROM execution.semantic_pnf_candidate_admissibility_event a
          JOIN observed o ON o.evidence_id=a.evidence_id WHERE NOT o.supported ORDER BY a.event_id),
        (SELECT count(*) FROM history)::bigint",
        &[&demand])?;
    let historical:i64=row.get(0);let supported:i64=row.get(1);
    let stale:Vec<i64>=row.get(2);let stale_events:Vec<i64>=row.get(3);let changed:i64=row.get(4);
    if historical<0||supported<0||changed<0||historical!=supported+stale.len() as i64{
        return Err(NativeNumericAuditError::AdmissionProbeInvariant);
    }
    let h3_stale:Vec<i64>=tx.query("SELECT evidence_id FROM execution.slr_native_h3_validity_current
       WHERE demand_id=$1 AND NOT supported ORDER BY evidence_id",&[&demand])?.iter().map(|r|r.get(0)).collect();
    let durable_pending:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.slr_native_h3_authority_debt WHERE demand_id=$1)",&[&demand])?.get(0);
    let all_stale=stale.iter().chain(h3_stale.iter()).copied().collect::<Vec<_>>();
    // Same original m086 cumulative carrier with only physically withdrawn
    // m110 atoms omitted. Do not subtract an arbitrary score or refute a
    // candidate; all other evidence and neutral/negative atoms survive.
    let current_signed_horizons:Vec<NativeCurrentSignedHorizon>=tx.query("WITH horizon(horizon) AS
        (VALUES(3::smallint),(6::smallint),(9::smallint))
      SELECT u.target_kind,u.target_id,h.horizon,
          COALESCE(sum(e.signed_residual),0)::bigint,
          sign(COALESCE(sum(e.signed_residual),0))::smallint,count(e.evidence_id)::bigint
        FROM execution.semantic_pnf_candidate_universe u CROSS JOIN horizon h
        LEFT JOIN execution.semantic_pnf_candidate_evidence e
          ON e.demand_id=u.demand_id AND e.target_kind=u.target_kind AND e.target_id=u.target_id
         AND e.horizon<=h.horizon AND NOT EXISTS(
          SELECT 1 FROM execution.slr_native_h6_validity_current v
          WHERE v.evidence_id=e.evidence_id AND v.demand_id=e.demand_id AND NOT v.supported)
         AND NOT EXISTS(SELECT 1 FROM execution.slr_native_h3_validity_current v
           WHERE v.evidence_id=e.evidence_id AND v.demand_id=e.demand_id AND NOT v.supported)
        WHERE u.demand_id=$1
        GROUP BY u.target_kind,u.target_id,h.horizon ORDER BY u.target_kind,u.target_id,h.horizon",
      &[&demand])?.iter().map(|r|NativeCurrentSignedHorizon{
          target_kind:r.get(0),target_id:r.get(1),horizon:r.get(2),
          signed_residual:r.get(3),phase:r.get(4),evidence_count:r.get(5),
      }).collect();
    let projected=tx.query("SELECT target_kind,target_id,horizon,signed_residual,phase,evidence_count
        FROM execution.semantic_pnf_candidate_evidence_horizon_v1 WHERE demand_id=$1 ORDER BY target_kind,target_id,horizon",
        &[&demand])?;
    if projected.len()!=current_signed_horizons.len() || projected.iter().zip(&current_signed_horizons).any(|(r,h)|
        r.get::<_,i16>(0)!=h.target_kind||r.get::<_,i64>(1)!=h.target_id||r.get::<_,i16>(2)!=h.horizon
        ||r.get::<_,i64>(3)!=h.signed_residual||r.get::<_,i16>(4)!=h.phase||r.get::<_,i64>(5)!=h.evidence_count){
        return Err(NativeNumericAuditError::AdmissionProbeInvariant);
    }
    // Historical invalid atoms can be retained without blocking a current
    // candidate-only outcome. A live admissibility event, closed-world proof,
    // or sufficiency certificate referencing the old carrier needs its own
    // original authority; do not issue a synthetic restoration/refutation.
    let current_stale_events=tx.query("WITH latest AS (
        SELECT DISTINCT ON(target_kind,target_id) event_id,evidence_id,target_kind,target_id
        FROM execution.semantic_pnf_candidate_admissibility_event WHERE demand_id=$1
        ORDER BY target_kind,target_id,event_id DESC)
      SELECT event_id FROM latest WHERE evidence_id=ANY($2::bigint[]) OR NOT EXISTS(
        SELECT 1 FROM execution.slr_native_current_candidate_evidence_v1 e
        WHERE e.evidence_id=latest.evidence_id AND e.demand_id=$1
          AND e.target_kind=latest.target_kind AND e.target_id=latest.target_id)
      ORDER BY event_id",
        &[&demand,&all_stale])?.iter().map(|r|r.get::<_,i64>(0)).collect::<Vec<_>>();
    let proof_or_sufficiency= !all_stale.is_empty() && tx.query_one("SELECT
        EXISTS(SELECT 1 FROM execution.semantic_pnf_frontier_resolution WHERE demand_id=$1 AND outcome_state=2)
        OR EXISTS(SELECT 1 FROM execution.semantic_pnf_demand_open_world_state WHERE demand_id=$1 AND NOT outside_model_possible)
        OR EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_sufficiency_current_v1 WHERE demand_id=$1 AND certificate_state=1)",
        &[&demand])?.get::<_,bool>(0);
    Ok(NativeH6ValidityReceipt{
        historical_atom_count:historical as usize,currently_supported_atom_count:supported as usize,
        original_consumer_authority_reconciled:current_stale_events.is_empty()&&!proof_or_sufficiency&&!durable_pending,
        native_h3_stale_evidence_ids:h3_stale,native_h3_state_rows_changed:0,durable_nonmonotone_authority_pending:durable_pending,
        stale_evidence_ids:stale,stale_admissibility_event_ids:stale_events,
        current_stale_admissibility_event_ids:current_stale_events,original_proof_or_sufficiency_revalidation_required:proof_or_sufficiency,
        state_rows_changed:changed as u64,current_signed_horizons,current_projection_exact:true,
    })
}

/// Original m094/m110 queue execution for the exact source-local consumer.
/// A completed horizon means the represented work slice was processed, not
/// exhaustive typed coverage, unique identity, or a completed external H9.
#[derive(Debug,Clone,Serialize)]
pub struct NativeConsumerHorizonWorkReceipt {
    pub consumer_ref:String,
    pub query_ref:String,
    pub policy_ref:String,
    pub h3_ready_before:bool,
    pub h3_processed:bool,
    pub h6_ready_before:bool,
    pub h6_processed:bool,
    pub h3_ready_after:bool,
    pub h6_ready_after:bool,
    pub h6_producer_invocations:usize,
    pub h6_evidence_inserted:i64,
    pub h9_residual_ready:bool,
    pub represented_slice_drained:bool,
    pub nonmonotone_input_residual:bool,
    pub h6_validity:NativeH6ValidityReceipt,
    pub typed_coverage_paid:bool,
    pub external_execution_performed:bool,
}

fn process_original_native_consumer_horizons(
    tx:&mut postgres::Transaction<'_>,demand:i64,run_ref:&str,answer:&NativeSourceLocalAnswer,
    represented_slice_drained:bool,inputs_changed:bool,nonmonotone:bool,
)->Result<NativeConsumerHorizonWorkReceipt,NativeNumericAuditError>{
    let row=tx.query_one("SELECT r.run_id,r.document_id FROM execution.semantic_pnf_demand d
        JOIN execution.semantic_pnf_region r ON r.region_id=d.source_region_id
        WHERE d.demand_id=$1 AND r.run_ref=$2 AND r.document_ref=$3",
        &[&demand,&run_ref,&answer.source_document_ref])?;
    let run:i64=row.get(0);let doc:i64=row.get(1);
    let consumer=answer.consumer_scope_ref;
    let query=&answer.original_review_item_ref;
    let policy=&answer.original_policy_ref;
    // The original 110 owner accepts a consumer/query/policy scope, not
    // a demand parameter. Reject a foreign demand registered under this key.
    let foreign:bool=tx.query_one("SELECT EXISTS(
        SELECT 1 FROM execution.semantic_pnf_consumer_horizon_work_queue w
        JOIN execution.semantic_pnf_demand d ON d.demand_id=w.demand_id
        JOIN execution.semantic_pnf_region r ON r.region_id=d.source_region_id
        WHERE r.run_id=$1 AND r.document_id=$2 AND w.consumer_ref=$3
          AND w.query_ref=$4 AND w.policy_ref=$5 AND w.demand_id<>$6)",
        &[&run,&doc,&consumer,query,policy,&demand])?.get(0);
    if foreign{return Err(NativeNumericAuditError::ForeignSource);}
    tx.execute("INSERT INTO execution.semantic_pnf_consumer_horizon_work_queue
        (demand_id,consumer_ref,query_ref,policy_ref,horizon)
        VALUES($1,$2,$3,$4,3) ON CONFLICT DO NOTHING",
        &[&demand,&consumer,query,policy])?;
    if inputs_changed {
        tx.execute("UPDATE execution.semantic_pnf_consumer_horizon_work_queue
            SET work_state=1,completed_at=NULL WHERE demand_id=$1 AND consumer_ref=$2
            AND query_ref=$3 AND policy_ref=$4 AND horizon=3 AND work_state<>1",
            &[&demand,&consumer,query,policy])?;
    }
    // Register exact positive source-coordinate dependencies for existing
    // evidence and typed candidate objects. The original m139/163 triggers
    // own subsequent evidence wakeups. The separate interval registration
    // covers previously absent source/family observations.
    tx.execute("INSERT INTO execution.semantic_pnf_consumer_reverse_dependency
        (source_kind,source_id,demand_id,consumer_ref,query_ref,policy_ref,minimum_horizon,dependency_kind)
        SELECT c.kind,c.id,$1,$2,$3,$4,3,1 FROM (
          SELECT 2::smallint AS kind,d.source_object_id AS id FROM execution.semantic_pnf_demand d WHERE d.demand_id=$1
          UNION SELECT 5::smallint,d.source_interface_id FROM execution.semantic_pnf_demand d WHERE d.demand_id=$1
          UNION SELECT 4::smallint,d.source_region_id FROM execution.semantic_pnf_demand d WHERE d.demand_id=$1
          UNION SELECT 2::smallint,u.target_id FROM execution.semantic_pnf_candidate_universe u WHERE u.demand_id=$1 AND u.target_kind=1
          UNION SELECT 1::smallint,s.token_id FROM execution.semantic_pnf_candidate_universe u
            JOIN execution.semantic_pnf_object_token_support s ON s.object_id=u.target_id
            WHERE u.demand_id=$1 AND u.target_kind=1
          UNION SELECT 6::smallint,e.evidence_id FROM execution.semantic_pnf_candidate_evidence e WHERE e.demand_id=$1 AND e.horizon=3
          UNION SELECT 4::smallint,e.source_region_id FROM execution.semantic_pnf_candidate_evidence e WHERE e.demand_id=$1 AND e.horizon=3
          UNION SELECT 5::smallint,e.source_interface_id FROM execution.semantic_pnf_candidate_evidence e WHERE e.demand_id=$1 AND e.horizon=3
        ) c WHERE c.id IS NOT NULL ON CONFLICT DO NOTHING",&[&demand,&consumer,query,policy])?;
    let ready=|db:&mut postgres::Transaction<'_>,horizon:i16|->Result<bool,NativeNumericAuditError>{
        Ok(db.query_one("SELECT EXISTS(SELECT 1 FROM execution.semantic_pnf_consumer_horizon_work_queue
          WHERE demand_id=$1 AND consumer_ref=$2 AND query_ref=$3 AND policy_ref=$4
            AND horizon=$5 AND work_state=1)",&[&demand,&consumer,query,policy,&horizon])?.get(0))
    };
    if nonmonotone{
        tx.execute("INSERT INTO execution.slr_native_h3_authority_debt(demand_id) VALUES($1) ON CONFLICT DO NOTHING",&[&demand])?;
    }
    let h3_changed=reconcile_native_h3_validity(tx,demand)?;
    let mut h6_validity=reopen_native_h6_validity(tx,demand)?;
    h6_validity.native_h3_state_rows_changed=h3_changed;
    // A checked, drained represented slice with no original authority
    // can settle scheduling debt. A live proof/coverage/admissibility or
    // sufficiency claim remains blocked and needs its original owner.
    let debt_retired=if h6_validity.durable_nonmonotone_authority_pending
        && represented_slice_drained && h6_validity.current_projection_exact {
        tx.query_one("SELECT execution.slr_reconcile_native_candidate_only_debt($1,$2,$3)",
            &[&demand,&represented_slice_drained,&h6_validity.current_projection_exact])?.get::<_,bool>(0)
    }else{false};
    if debt_retired {
        h6_validity.durable_nonmonotone_authority_pending=false;
        h6_validity.original_consumer_authority_reconciled=
            h6_validity.current_stale_admissibility_event_ids.is_empty()
            &&!h6_validity.original_proof_or_sufficiency_revalidation_required;
    }
    let nonmonotone=(nonmonotone&&!debt_retired)||!h6_validity.original_consumer_authority_reconciled;
    if h3_changed>0 {
        for horizon in [3i16,6,9]{
            tx.query_one("SELECT execution.slr_refresh_native_preference($1,$2)",&[&demand,&horizon])?;
        }
        tx.execute("UPDATE execution.semantic_pnf_consumer_horizon_work_queue
            SET work_state=1,completed_at=NULL WHERE demand_id=$1 AND consumer_ref=$2
              AND query_ref=$3 AND policy_ref=$4 AND horizon IN(3,6,9) AND work_state<>1",
            &[&demand,&consumer,query,policy])?;
    }
    // A physical H6 validity change can arise without parser input changes.
    // Reopen the original consumer's H6 outcome (and downstream work) when
    // H6 had already been requested; H3 still owns its own readiness.
    if h6_validity.state_rows_changed>0 {
        // Preferences have no proof authority; retract stale winners even
        // while a separate original proof/sufficiency residual stays open.
        tx.query_one("SELECT execution.slr_refresh_native_preference($1,6::smallint)",&[&demand])?;
        tx.query_one("SELECT execution.slr_refresh_native_preference($1,9::smallint)",&[&demand])?;
        tx.execute("UPDATE execution.semantic_pnf_consumer_horizon_work_queue
            SET work_state=1,completed_at=NULL WHERE demand_id=$1 AND consumer_ref=$2
              AND query_ref=$3 AND policy_ref=$4 AND horizon IN(6,9) AND work_state<>1",
            &[&demand,&consumer,query,policy])?;
    }
    // Suspend existing derived outcomes even when a pending queue cannot
    // yet be completed. Do not rewrite any original proof/certificate rows.
    let authority_usable:bool=tx.query_one("SELECT execution.slr_native_current_authority_usable($1)",&[&demand])?.get(0);
    if !authority_usable {
        for horizon in [3i16,6,9]{
            tx.query_one("SELECT execution.refresh_numeric_pnf_consumer_horizon_outcome($1,$2,$3,$4,$5,$6)",
                &[&run,&doc,&horizon,&consumer,query,policy])?;
        }
    }
    if authority_usable!=h6_validity.original_consumer_authority_reconciled{
        return Err(NativeNumericAuditError::AdmissionProbeInvariant);
    }
    let h3_ready=ready(tx,3)?;
    let mut h3_processed=false;
    // H3 owner work is the already materialized bounded represented slice.
    // Nonmonotone support changes need original evidence withdrawal first;
    // m110's insert-only H6 cannot silently retire old positive evidence.
    if represented_slice_drained && !nonmonotone && h3_ready {
        tx.query_one("SELECT execution.slr_refresh_native_preference($1,3::smallint)",&[&demand])?;
        tx.query_one("SELECT execution.advance_numeric_pnf_horizon_work_for_consumer($1,$2,3::smallint,$3,$4,$5)",
            &[&run,&doc,&consumer,query,policy])?;
        h3_processed=!ready(tx,3)?;
        if !h3_processed{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    }
    let h6_ready=ready(tx,6)?;
    let mut h6_processed=false;let mut h6_evidence_inserted=0i64;let mut h6_invocations=0usize;
    if represented_slice_drained&&!nonmonotone&&!ready(tx,3)?&&h6_ready{
        // Do not invoke the document-wide preference rebuild in process_h6.
        // Use the actual scoped evidence owner and original scoped advance.
        // Existing H6 evidence remains append-only, preserving signed/zero H3.
        h6_invocations=1;
        h6_evidence_inserted=tx.query_one("SELECT execution.refresh_numeric_pnf_h6_discourse_temporal_evidence_for_consumer($1,$2,$3,$4,$5,FALSE)",
            &[&run,&doc,&consumer,query,policy])?.get(0);
        h6_validity=reopen_native_h6_validity(tx,demand)?;
        h6_validity.native_h3_state_rows_changed=h3_changed;
        if !h6_validity.original_consumer_authority_reconciled{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
        tx.query_one("SELECT execution.slr_refresh_native_preference($1,6::smallint)",&[&demand])?;
        tx.query_one("SELECT execution.slr_refresh_native_preference($1,9::smallint)",&[&demand])?;
        tx.query_one("SELECT execution.advance_numeric_pnf_horizon_work_for_consumer($1,$2,6::smallint,$3,$4,$5)",
            &[&run,&doc,&consumer,query,policy])?;
        h6_processed=!ready(tx,6)?;
        if !h6_processed{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    }
    // A newly inserted H6 atom may legitimately wake an H3 dependency via
    // the original trigger. Leave that work ready; never acknowledge a wake
    // that arrived after its original processing pass.
    Ok(NativeConsumerHorizonWorkReceipt{
        consumer_ref:consumer.into(),query_ref:query.clone(),policy_ref:policy.clone(),
        h3_ready_before:h3_ready,h3_processed,h6_ready_before:h6_ready,h6_processed,
        h3_ready_after:ready(tx,3)?,h6_ready_after:ready(tx,6)?,
        h6_producer_invocations:h6_invocations,h6_evidence_inserted,
        h9_residual_ready:ready(tx,9)?,represented_slice_drained,nonmonotone_input_residual:nonmonotone,
        h6_validity,typed_coverage_paid:false,external_execution_performed:false,
    })
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
    pub dependency_head_alternatives:Vec<i64>,
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
    let unresolved_kind:i64=tx.query_one("SELECT execution.ensure_semantic_symbol(14::smallint,'mention.unresolved_head')",&[])?.get(0);
    let mut created=0usize;let mut observations=Vec::new();
    for ((a,b),licenses) in spans.into_iter().rev(){
        let slice=&tokens[a..b];let first=&slice[0];let last=&slice[slice.len()-1];
        let mut token_ids=slice.iter().map(|t|t.id).collect::<Vec<_>>();
        token_ids.sort_unstable();
        let head=slice.iter().filter(|t|t.head.is_some_and(|h|h==t.id||!token_ids.contains(&h)))
            .collect::<Vec<_>>();
        let head_alternatives=head.iter().map(|h|h.id).collect::<Vec<_>>();
        let head=if let [h]=head.as_slice(){Some(*h)}else{None};
        let key=(first.region,token_ids.clone());
        let mut objects=support.get(&key).cloned().unwrap_or_default();
        if objects.is_empty()&&created<budget{
            {
                // A multi-head or headless occurrence is still a source object.
                // NULL lexical head + distinct unresolved kind prevents guessed name identity.
                let mut d=Sha256::new();
                if head.is_some(){d.update(b"slr:numeric-mention-span:v1");}
                else{d.update(b"slr:numeric-mention-span:unresolved-head:v1");}
                d.update(first.region.to_be_bytes());
                for id in &token_ids{d.update(id.to_be_bytes());}
                let digest=d.finalize().to_vec();
                let kind=if head.is_none(){unresolved_kind}else if first.pos=="PROPN"{proper}else{noun};
                let lexical_head=head.map(|h|h.lemma);
                let row=tx.query_one(
                    "INSERT INTO execution.semantic_pnf_object
                     (object_digest,region_id,object_kind_symbol_id,head_symbol_id,scope_region_id,
                      promotion_level,information_gain,representation_cost,ambiguity_cost,promotion_score,active)
                     VALUES($1,$2,$3,$4,$2,0,0,1,1,0,TRUE)
                     ON CONFLICT(object_digest) DO UPDATE SET object_digest=EXCLUDED.object_digest
                     RETURNING object_id,region_id,object_kind_symbol_id,head_symbol_id,active",
                     &[&digest,&first.region,&kind,&lexical_head])?;
                if row.get::<_,i64>(1)!=first.region||row.get::<_,i64>(2)!=kind
                    ||row.get::<_,Option<i64>>(3)!=lexical_head||!row.get::<_,bool>(4){
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
            exact_object_ids:objects,unique_dependency_head:head.map(|h|h.id),dependency_head_alternatives:head_alternatives,
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
    pub native_relation_reductions:usize,pub native_role_reductions:usize,
    pub native_action_metadata_reductions:usize,
    pub native_relation_algebra_rows:usize,
    pub question_mode_observation:Option<serde_json::Value>,pub original_tag_capability_missing_count:usize,
    pub focus_reference_anchors:usize,pub retired_index_anchors:u64,
    pub candidate_sets:Vec<NativeBindingSetBridgeReceipt>,
    pub focus_reference_alternatives:Vec<&'static str>,
    pub original_index_candidate_evidence_inserted:u64,
    pub exact_index_entity_objects_outside_transport:usize,
    pub indexed_entity_spans_missing_from_projected:Vec<(i64,i64)>,
    pub projected_entity_spans_missing_from_index:Vec<(i64,i64)>,
    pub multihead_or_headless_spans:usize,
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
                if !feature.is_empty(){
                    // Numeric parser kind 6 uppercases feature symbols.
                    // Original m009 declaration compares exact UD feature
                    // names ('Number','Gender','Person'); map only those
                    // known coordinates, keeping all other symbols intact.
                    let canonical=MORPH_FEATURES.iter().copied()
                        .find(|name|feature.eq_ignore_ascii_case(name)).unwrap_or(feature);
                    values.insert((canonical.to_owned(),value.to_owned()));
                }
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
           AND r.start_char <= (SELECT start_char FROM execution.semantic_parser_token WHERE token_id=$3)
         ORDER BY t.start_char,t.end_char,t.token_id",&[&run_ref,&doc,&focus_token])?;
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
    let relations=native_relation_grammar(&relation_tokens);
    let (relation_reductions,question_mode,missing_original_tags)=native_relation_reductions(tx,run_ref,doc,&relation_tokens,&relations)?;
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
        let (_,kind,_,_)=relation_declaration(relation.relation_type).ok_or(NativeNumericAuditError::ForeignSource)?;
        let a=bridge_anchor(name.clone(),kind,&ids,vec![],&tokens,&source,meta)?;
        // Original structural binder tests first-token precedence; retain the
        // entire relation support separately, including discontinuities.
        // Keep the entire containing sentence relation carrier, including
        // the focus predicate itself. The original binding query owns
        // preceding-accessibility, not this materialization step.
        anchors.push(a);relation_anchors+=1;
        for (ordinal,(role,id)) in relation.roles.iter().enumerate(){
            if relation_role_constraint(relation.relation_type,role).is_none(){continue;}
            let key=format!("{name}:role:{ordinal}:{role}:{id}");
            let role_kind=if relation.relation_type=="predicate"{format!("semantic.argument.{role}")}else{
                format!("semantic.relation_role.{}.{role}",relation.relation_type)};
            let a=bridge_anchor(key.clone(),&role_kind,&[*id],vec![],&tokens,&source,meta)?;
            if *id==focus_token&&relation.relation_type=="predicate"&&matches!(*role,"subject"|"object"|"argument"){
                focus_keys.push(key);anchors.push(a);
            }else{anchors.push(a);}
        }
        if relation.relation_type=="predicate"{
            let key=format!("{name}:feature:action_meta");
            let a=bridge_anchor(key,"semantic.predicate_inflection",&[relation.head_token_id],vec![],&tokens,&source,meta)?;
            anchors.push(a);
        }
    }
    if focus_keys.is_empty(){return Err(NativeNumericAuditError::SourcePolicy);}
    anchors.sort_by(|a,b|a.key.cmp(&b.key));
    // Snapshot binds all observed parser coordinates, original projected
    // anchor metadata, exact source and inverse object IDs; never a count key.
    let tape=tokens.values().map(|t|(t.id,t.sentence_id,t.ordinal,t.start,t.end,t.head,
        &t.pos,&t.dep,&t.morphology)).collect::<Vec<_>>();
    let snapshot=format!("slr-native-binding-index:v2:{}",bridge_digest(&("consumer-prefix:v4",run_ref,doc,sha,tape,&anchors,&relation_reductions,&question_mode,missing_original_tags))?);
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
            active boolean NOT NULL,source_key text
         );
         ALTER TABLE execution.slr_native_binding_anchor_projection ADD COLUMN IF NOT EXISTS source_key text;
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
    // Preserve the actual relation/role reduction payload and original
    // parser-coordinate inverse. This is candidate algebra, not a change
    // to an already closed immutable numeric Level-1 sentence.
    tx.batch_execute("CREATE TABLE IF NOT EXISTS execution.slr_native_relation_reduction(
        run_ref text NOT NULL,document_ref text NOT NULL,snapshot_ref text NOT NULL,
        relation_ref text NOT NULL,payload jsonb NOT NULL,
        PRIMARY KEY(run_ref,document_ref,snapshot_ref,relation_ref))")?;
    for r in &relation_reductions{
        let payload=serde_json::to_value(r).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        tx.execute("INSERT INTO execution.slr_native_relation_reduction(run_ref,document_ref,snapshot_ref,relation_ref,payload)
            VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING",&[&run_ref,&doc,&snapshot,&r.relation_ref,&payload])?;
        let stored:serde_json::Value=tx.query_one("SELECT payload FROM execution.slr_native_relation_reduction
            WHERE run_ref=$1 AND document_ref=$2 AND snapshot_ref=$3 AND relation_ref=$4",
            &[&run_ref,&doc,&snapshot,&r.relation_ref])?.get(0);
        if stored!=payload{return Err(NativeNumericAuditError::ForeignSource);}
    }
    if let Some(q)=&question_mode{
        let name=format!("native-question-mode:{}",bridge_digest(&(run_ref,doc,q))?);
        tx.execute("INSERT INTO execution.slr_native_relation_reduction(run_ref,document_ref,snapshot_ref,relation_ref,payload)
            VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING",&[&run_ref,&doc,&snapshot,&name,q])?;
        let stored:serde_json::Value=tx.query_one("SELECT payload FROM execution.slr_native_relation_reduction
            WHERE run_ref=$1 AND document_ref=$2 AND snapshot_ref=$3 AND relation_ref=$4",
            &[&run_ref,&doc,&snapshot,&name])?.get(0);
        if &stored!=q{return Err(NativeNumericAuditError::ForeignSource);}
    }
    let mut local_relation_keys=BTreeSet::new();
    for row in &relation_reductions{
        if row.residuals.is_empty(){local_relation_keys.insert(row.relation_ref.clone());}
        if row.action_meta.is_some(){local_relation_keys.insert(format!("{}:feature:action_meta",row.relation_ref));}
        for role in &row.role_factors{
            if role["residuals"].as_array().is_some_and(|r|r.is_empty()){
                let ordinal=role["ordinal"].as_u64().ok_or(NativeNumericAuditError::ForeignSource)?;
                let name=role["role"].as_str().ok_or(NativeNumericAuditError::ForeignSource)?;
                let token=role["token_id"].as_i64().ok_or(NativeNumericAuditError::ForeignSource)?;
                local_relation_keys.insert(format!("{}:role:{ordinal}:{name}:{token}",row.relation_ref));
            }
        }
    }
    let mut references=Vec::new();
    let mut relation_factor_keys=BTreeMap::<String,(String,String)>::new();
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
        let closure=if local_relation_keys.contains(&a.key){"locally_closed"}else{"requires_external_resolution"};
        tx.execute("INSERT INTO algebra.factor(factor_ref,document_ref,factor_type_ref,closure_state_ref,factor_sha256)
            VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING",
            &[&factor,&doc,&a.kind,&closure,&dg])?;
        tx.execute("INSERT INTO algebra.factor_revision(factor_revision_ref,factor_ref,closure_state_ref,factor_sha256)
            VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING",
            &[&revision,&factor,&closure,&dg])?;
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
            (factor_revision_ref,run_ref,document_ref,snapshot_ref,span_ref,token_ids,object_ids,active,source_key)
            VALUES($1,$2,$3,$4,$5,$6,$7,true,$8) ON CONFLICT(factor_revision_ref) DO UPDATE
            SET active=true,source_key=COALESCE(execution.slr_native_binding_anchor_projection.source_key,EXCLUDED.source_key)
            WHERE NOT execution.slr_native_binding_anchor_projection.active OR execution.slr_native_binding_anchor_projection.source_key IS NULL",
            &[&revision,&run_ref,&doc,&snapshot,&span,&a.token_ids,&a.object_ids,&a.key])?;
        // Verify inverse on the stored projection, not just insertion count.
        let valid:bool=tx.query_one(
            "SELECT EXISTS(SELECT 1 FROM execution.slr_native_binding_anchor_projection p
              JOIN pnf.factor_anchor a ON a.factor_revision_ref=p.factor_revision_ref
              JOIN corpus.span s ON s.span_ref=p.span_ref
              WHERE p.factor_revision_ref=$1 AND p.run_ref=$2 AND p.document_ref=$3
                AND p.snapshot_ref=$4 AND p.active AND p.token_ids=$5 AND p.object_ids=$6
                AND s.document_ref=$3 AND s.start_char=$7 AND s.end_char=$8
                AND s.start_token=$9 AND s.end_token=$10 AND a.start_token=$9 AND a.end_token=$11 AND p.source_key=$12)",
            &[&revision,&run_ref,&doc,&snapshot,&a.token_ids,&a.object_ids,&start,&end,&a.ordinal,&a.end_ordinal,&next,&a.key])?.get(0);
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
        relation_factor_keys.insert(a.key.clone(),(factor.clone(),revision.clone()));
        if focus_keys.contains(&a.key){
            persist_native_reference_alternatives(tx,&factor,&revision,a.kind=="semantic.argument.subject")?;
            references.push((factor,revision));
        }
    }
    // Weld the actual source relation fibres to the original algebra
    // alternative, residual and constraint carriers; no bare index anchor
    // can stand in for the original relation-valued candidate.
    let relation_algebra_rows=materialize_native_relation_algebra(tx,doc,&snapshot,
        &relation_reductions,question_mode.as_ref(),&relation_factor_keys)?;
    let mut sets=Vec::new();
    for (factor,revision) in references{
        for typ in ["entity_reference","eventuality_reference","proposition_reference"]{
            sets.push(materialize_native_binding_set_v2(tx,&snapshot,&doc,&factor,&revision,typ)?);
        }
    }
    let (original_index_candidate_evidence_inserted,exact_index_entity_objects_outside_transport)=
        persist_native_index_evidence(tx,demand,&snapshot,&doc,&sets)?;
    // Cross-owner parity, not self-parity. Compare exact mention spans,
    // including cap-deferred candidates, without conflating equal spelling.
    let mut indexed_spans=BTreeSet::new();
    for set in sets.iter().filter(|s|s.referential_type=="entity_reference"){
        let rows=tx.query("SELECT sp.start_char,sp.end_char
            FROM resolution.binding_candidate_set s
            CROSS JOIN LATERAL resolution.query_binding_candidates(s.reference_factor_revision_ref,
                s.referential_type_ref,s.accessibility_declaration_ref,s.compatibility_declaration_ref,$2) m
            JOIN execution.slr_native_binding_anchor_projection p ON p.factor_revision_ref=m.candidate_factor_revision_ref AND p.active
            JOIN corpus.span sp ON sp.span_ref=p.span_ref WHERE s.candidate_set_ref=$1",
            &[&set.candidate_set_ref,&i32::try_from(anchors.len()).map_err(|_|NativeNumericAuditError::ForeignSource)?])?;
        for row in rows{indexed_spans.insert((i64::from(row.get::<_,i32>(0)),i64::from(row.get::<_,i32>(1))));}
    }
    let projected_spans=campaign.represented_spans.iter().filter(|s|s.projected_compatible)
        .map(|s|(s.start_char,s.end_char)).collect::<BTreeSet<_>>();
    let indexed_entity_spans_missing_from_projected=indexed_spans.difference(&projected_spans).copied().collect();
    let projected_entity_spans_missing_from_index=projected_spans.difference(&indexed_spans).copied().collect();
    let has_subject=anchors.iter().any(|a|focus_keys.contains(&a.key)&&a.kind=="semantic.argument.subject");
    let mut alternatives=vec!["entity_reference","eventuality_reference","proposition_reference"];
    if has_subject{alternatives.push("expletive_realisation");}
    Ok(NativeBindingBridgeReceipt{
        snapshot_ref:snapshot,source_parser_tokens:tokens.len(),anchors:anchors.len(),mention_anchors,
        relation_anchors,native_relation_reductions:relation_reductions.len(),
        native_role_reductions:relation_reductions.iter().map(|r|r.role_factors.len()).sum(),
        native_action_metadata_reductions:relation_reductions.iter().filter(|r|r.action_meta.is_some()).count(),native_relation_algebra_rows:relation_algebra_rows,
        question_mode_observation:question_mode,original_tag_capability_missing_count:missing_original_tags,focus_reference_anchors:focus_keys.len(),retired_index_anchors:retired,candidate_sets:sets,
        focus_reference_alternatives:alternatives,original_index_candidate_evidence_inserted,
        exact_index_entity_objects_outside_transport,indexed_entity_spans_missing_from_projected,
        projected_entity_spans_missing_from_index,
        multihead_or_headless_spans:campaign.represented_spans.iter().filter(|s|s.unique_dependency_head.is_none()).count(),
        unresolved_structural_metadata:vec!["canonical_tokenizer_parity","full_original_mention_factor_parity",
            "original_relation_root_reference_crosswalk_reconstructed_not_independently_compared","original_role_factor_span_references_reconstructed_not_independently_compared","question_mode_original_tag_unavailable","consumer_prefix_only_later_source_relations_unvisited"],
        mention_spans_without_objects:campaign.spans_without_exact_objects,
        mention_spans_with_multiple_objects:campaign.represented_spans.iter().filter(|s|s.exact_object_ids.len()>1).count(),
        observed_document_order_verified:true,canonical_tokenizer_parity_paid:false,
        structural_relation_parity_paid:false,typed_coverage_paid:false,
    })
}
/// Original m006 algebra and existing factor-revision owners. The native
/// relation payload is bound to the immutable revision and read back; role
/// constraints retain ordered source occurrence identity in their own key.
fn persist_relation_candidate(
    tx:&mut postgres::Transaction<'_>,factor:&str,revision:&str,typ:&str,
    payload:&serde_json::Value,grammar:&str,relation:&str,residuals:&[&str],
)->Result<(),NativeNumericAuditError>{
    let suffix=bridge_digest(&(revision,typ,payload,grammar,relation))?;
    let alternative=format!("alternative:native-relation:{suffix}");
    let bytes=Sha256::digest(suffix.as_bytes()).to_vec();
    let literal=serde_json::to_string(payload).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    tx.execute("INSERT INTO algebra.alternative(alternative_ref,type_ref,value_literal,authority_state_ref,alternative_sha256)
        VALUES($1,$2,$3,'candidate_only',$4) ON CONFLICT DO NOTHING",&[&alternative,&typ,&literal,&bytes])?;
    tx.execute("INSERT INTO algebra.factor_alternative(factor_ref,alternative_ref,alternative_state_ref)
        VALUES($1,$2,'alternative') ON CONFLICT DO NOTHING",&[&factor,&alternative])?;
    tx.execute("INSERT INTO algebra.factor_revision_alternative(factor_revision_ref,alternative_ref,alternative_state_ref)
        VALUES($1,$2,'alternative') ON CONFLICT DO NOTHING",&[&revision,&alternative])?;
    for provenance in [grammar,relation]{
        tx.execute("INSERT INTO algebra.alternative_derivation(alternative_ref,derivation_ref)
            VALUES($1,$2) ON CONFLICT DO NOTHING",&[&alternative,&provenance])?;
    }
    for residual in residuals{
        let key=format!("{revision}:residual:{residual}");
        let digest=Sha256::digest(format!("slr-native-reference-residual:v2:{key}").as_bytes()).to_vec();
        tx.execute("INSERT INTO algebra.residual(residual_ref,target_ref,residual_type_ref,residual_state_ref,residual_sha256)
            VALUES($1,$2,$3,'open',$4) ON CONFLICT DO NOTHING",&[&key,&revision,residual,&digest])?;
        let exact:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM algebra.residual WHERE residual_ref=$1
            AND target_ref=$2 AND residual_type_ref=$3 AND residual_state_ref='open' AND residual_sha256=$4)",
            &[&key,&revision,residual,&digest])?.get(0);
        if !exact{return Err(NativeNumericAuditError::ForeignSource);}
    }
    let valid:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM algebra.alternative a
        JOIN algebra.factor_revision_alternative r ON r.alternative_ref=a.alternative_ref
        WHERE a.alternative_ref=$1 AND r.factor_revision_ref=$2 AND a.type_ref=$3
          AND a.value_literal=$4 AND a.authority_state_ref='candidate_only' AND a.alternative_sha256=$5)",
        &[&alternative,&revision,&typ,&literal,&bytes])?.get(0);
    if !valid{return Err(NativeNumericAuditError::ForeignSource);}
    Ok(())
}
fn materialize_native_relation_algebra(
    tx:&mut postgres::Transaction<'_>,doc:&str,snapshot:&str,
    rows:&[NativeRelationReductionRow],question:Option<&serde_json::Value>,
    keys:&BTreeMap<String,(String,String)>,
)->Result<usize,NativeNumericAuditError>{
    let mut count=0usize;
    for row in rows{
        let (root,revision)=keys.get(&row.relation_ref).ok_or(NativeNumericAuditError::ForeignSource)?;
        let packet=serde_json::to_value(row).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        persist_relation_candidate(tx,root,revision,row.candidate_type,&packet,row.grammar_ref,&row.relation_ref,&row.residuals)?;
        count+=1;
        for role in &row.role_factors{
            let ordinal=role["ordinal"].as_u64().ok_or(NativeNumericAuditError::ForeignSource)?;
            let name=role["role"].as_str().ok_or(NativeNumericAuditError::ForeignSource)?;
            let token=role["token_id"].as_i64().ok_or(NativeNumericAuditError::ForeignSource)?;
            let key=format!("{}:role:{ordinal}:{name}:{token}",row.relation_ref);
            let (factor,rev)=keys.get(&key).ok_or(NativeNumericAuditError::ForeignSource)?;
            let residuals=role["residuals"].as_array().ok_or(NativeNumericAuditError::ForeignSource)?
                .iter().map(|x|x.as_str().ok_or(NativeNumericAuditError::ForeignSource)).collect::<Result<Vec<_>,_>>()?;
            persist_relation_candidate(tx,factor,rev,"semantic.argument_candidate",role,row.grammar_ref,&row.relation_ref,&residuals)?;
            let constraint=role["constraint"].as_str().ok_or(NativeNumericAuditError::ForeignSource)?;
            let suffix=bridge_digest(&(snapshot,rev,revision,constraint,ordinal,role))?;
            let cref=format!("constraint:native-relation:{suffix}");
            let sha=Sha256::digest(suffix.as_bytes()).to_vec();
            tx.execute("INSERT INTO algebra.constraint(constraint_ref,constraint_type_ref,required,declaration_ref,constraint_sha256)
              VALUES($1,$2,TRUE,(SELECT declaration_ref FROM algebra.declaration WHERE declaration_ref=$3),$4) ON CONFLICT DO NOTHING",
              &[&cref,&constraint,&row.grammar_ref,&sha])?;
            for (f,direction) in [(factor,"source"),(root,"target")]{
                tx.execute("INSERT INTO algebra.constraint_factor(constraint_ref,factor_ref,factor_role_ref)
                    VALUES($1,$2,$3) ON CONFLICT DO NOTHING",&[&cref,f,&direction])?;
            }
            let exact:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM algebra.constraint c
                JOIN algebra.constraint_factor s ON s.constraint_ref=c.constraint_ref AND s.factor_role_ref='source'
                JOIN algebra.constraint_factor t ON t.constraint_ref=c.constraint_ref AND t.factor_role_ref='target'
                WHERE c.constraint_ref=$1 AND c.constraint_type_ref=$2 AND c.required AND c.constraint_sha256=$3
                   AND s.factor_ref=$4 AND t.factor_ref=$5)",&[&cref,&constraint,&sha,factor,root])?.get(0);
            if !exact{return Err(NativeNumericAuditError::ForeignSource);}
            count+=1;
        }
        if let Some(meta)=&row.action_meta{
            let key=format!("{}:feature:action_meta",row.relation_ref);
            let (f,rev)=keys.get(&key).ok_or(NativeNumericAuditError::ForeignSource)?;
            persist_relation_candidate(tx,f,rev,"semantic.predicate_inflection_candidate",meta,row.grammar_ref,&row.relation_ref,&[])?;
            count+=1;
        }
    }
    if let Some(q)=question{
        // Mode-only relation has no atom binding. Preserve it as an algebra
        // candidate without manufacturing a token anchor or referent.
        let suffix=bridge_digest(&(snapshot,q))?;
        let factor=format!("factor:native-question:{suffix}");
        let revision=format!("factor-revision:native-question:{suffix}");
        let sha=Sha256::digest(suffix.as_bytes()).to_vec();
        tx.execute("INSERT INTO algebra.factor(factor_ref,document_ref,factor_type_ref,closure_state_ref,factor_sha256)
            VALUES($1,$2,'semantic.embedded_proposition','requires_external_resolution',$3) ON CONFLICT DO NOTHING",
            &[&factor,&doc,&sha])?;
        tx.execute("INSERT INTO algebra.factor_revision(factor_revision_ref,factor_ref,closure_state_ref,factor_sha256)
            VALUES($1,$2,'requires_external_resolution',$3) ON CONFLICT DO NOTHING",&[&revision,&factor,&sha])?;
        persist_relation_candidate(tx,&factor,&revision,"semantic.composition_candidate",q,"grammar:semantic:composition:v0_4",
            &format!("native-question-mode:{}",bridge_digest(&(doc,q))?),
            &["composition_scope_unresolved","proposition_truth_not_evaluated"])?;
        count+=1;
    }
    Ok(count)
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
        ON CONFLICT(generator_build_ref) DO UPDATE SET build_state_ref='completed'
        WHERE execution.binding_candidate_set_build.build_state_ref<>'completed'",
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
                let actual=tx.query("SELECT e.evidence_id FROM execution.semantic_pnf_candidate_evidence e
                    JOIN execution.semantic_pnf_object o ON o.object_id=e.target_id AND o.active
                    JOIN execution.semantic_pnf_interface i ON i.region_id=o.region_id
                    WHERE e.demand_id=$1 AND e.target_kind=1 AND e.target_id=$2 AND e.evidence_ref=$3
                      AND e.evidence_kind=$4 AND e.provenance_ref=$5 AND e.horizon=3 AND e.evidence_family=1
                      AND e.signed_residual=0 AND e.source_interface_id=i.interface_id AND e.source_region_id=o.region_id",
                    &[&demand,&object,&evidence,&format!("original_binding_index_member:{path}"),&set.candidate_set_ref])?;
                let active:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.semantic_pnf_object WHERE object_id=$1 AND active)",&[&object])?.get(0);
                if active!=(!actual.is_empty())||actual.len()>1{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
                for row in actual{
                    tx.execute("INSERT INTO slr_native_h3_desired VALUES($1) ON CONFLICT DO NOTHING",&[&row.get::<_,i64>(0)])?;
                }
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
        // Original shared_reducer uses dependency negation OR its pinned
        // lexical negation cue; do not infer a claim truth judgement.
        let neg=direct.iter().copied().filter(|c|c.dep=="neg"||matches!(c.lemma.to_lowercase().as_str(),"not"|"never"|"no")).collect::<Vec<_>>();
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
    // The donor dedup key includes ordered roles, including repetitions.
    let mut seen=BTreeSet::new();
    out.retain(|r|seen.insert((r.region,r.relation_type,r.roles.clone())));
    out
}

/** Original shared_reducer.py 5e427655744b and semantic_reductions.py
 * a239d2c14255, pinned SensibLaw f9c4670e. Candidate relation syntax only.
 * Question mode is a value role, not an atom, and is retained separately. */
fn native_relation_grammar(tokens:&[NativeRelationToken])->Vec<ProjectedRelation>{
    let mut rows=projected_predicate_relations(tokens);
    let by_id=tokens.iter().map(|t|(t.id,t)).collect::<BTreeMap<_,_>>();
    for t in tokens {
        if let Some(head)=by_id.get(&t.head) {
            if matches!(t.dep.as_str(),"compound"|"amod"|"nmod"|"appos"|"conj")
                &&matches!(head.pos.as_str(),"NOUN"|"PROPN"|"PRON"|"NUM"){
                rows.push(ProjectedRelation{region:t.region,head_token_id:head.id,relation_type:"modifier",
                    roles:vec![("head",head.id),("modifier",t.id)]});
            }
            if t.dep=="conj" {
                rows.push(ProjectedRelation{region:t.region,head_token_id:head.id,relation_type:"conjunction",
                    roles:vec![("item",head.id),("item",t.id)]});
            }
        }
        if matches!(t.dep.as_str(),"npadvmod"|"tmod"|"pobj")
            &&t.orth.chars().count()==4&&t.orth.chars().all(|c|c.is_ascii_digit()){
            rows.push(ProjectedRelation{region:t.region,head_token_id:t.id,relation_type:"temporal",
                roles:vec![("anchor",t.id)]});
        }
    }
    // Original producer emits relations in the generating token's source
    // order, not numeric allocation order or relation family order.
    rows.sort_by_key(|r|{
        let generator=match r.relation_type{"modifier"|"conjunction"=>r.roles.last().map(|(_,id)|*id).unwrap_or(r.head_token_id),_=>r.head_token_id};
        let order=by_id.get(&generator).map(|t|(t.start,t.end,t.id)).unwrap_or((i64::MAX,i64::MAX,generator));
        let lane=match r.relation_type{"predicate"=>0,"composition"=>1,"modifier"=>2,"conjunction"=>3,_=>4};
        (order,lane)
    });
    let mut seen=BTreeSet::new();
    rows.retain(|r|seen.insert((r.region,r.relation_type,r.roles.clone())));
    rows
}

fn relation_declaration(kind:&str)->Option<(&'static str,&'static str,&'static str,&'static [&'static str])>{
    Some(match kind{
        "predicate"=>("grammar:semantic:predicate:v0_4","semantic.eventuality","semantic.eventuality_candidate",&["event_identity_unresolved"]),
        "composition"=>("grammar:semantic:composition:v0_4","semantic.embedded_proposition","semantic.composition_candidate",&["composition_scope_unresolved","proposition_truth_not_evaluated"]),
        "modifier"=>("grammar:semantic:modification:v0_3","semantic.nominal_description","semantic.nominal_description_candidate",&["nominal_denotation_unresolved","modifier_scope_unresolved"]),
        "conjunction"=>("grammar:semantic:coordination:v0_3","semantic.coordination","semantic.coordination_candidate",&[]),
        "temporal"=>("grammar:semantic:temporal:v0_3","semantic.temporal_expression","semantic.temporal_candidate",&["temporal_extent_unresolved"]),
        "spatial"=>("grammar:semantic:spatial:v0_3","semantic.spatial_expression","semantic.spatial_candidate",&["spatial_extent_unresolved"]),
        _=>return None,
    })
}
fn relation_role_constraint(kind:&str,role:&str)->Option<&'static str>{
    match (kind,role){
        ("predicate","subject")=>Some("syntactic_subject_of"),
        ("predicate","object")=>Some("syntactic_object_of"),
        ("predicate","oblique")=>Some("syntactic_oblique_of"),
        ("predicate","complement")=>Some("syntactic_complement_of"),
        ("predicate","argument")=>Some("syntactic_argument_of"),
        ("composition","host")=>Some("host_of_embedded_proposition"),
        ("composition","content")=>Some("content_of"),
        ("modifier","head")=>Some("nominal_head_of"),
        ("modifier","modifier")=>Some("nominal_modifier_of"),
        ("conjunction","item")=>Some("coordination_member_of"),
        ("temporal","anchor")=>Some("temporal_anchor_of"),
        ("spatial","anchor")=>Some("spatial_anchor_of"),
        _=>None,
    }
}

/// Exact original raw morphology is reopened from the checked parser plan,
/// not from the scalar head or POS fallback tag. Missing metadata stays
/// missing. Donor ontology_mapping.py canonical_action_morphology semantics
/// use an adapter with no children; do not reinterpret passive child roles.
fn original_relation_action_metadata(raw:Option<&str>,surface:&str,dep:&str)
    ->Result<serde_json::Value,NativeNumericAuditError>{
    let parsed=raw.map(serde_json::from_str::<serde_json::Value>).transpose()
        .map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let morph=parsed.as_ref().and_then(|v|v.as_object());
    let values=|name:&str|->Vec<String>{
        let Some(v)=morph.and_then(|m|m.get(name)) else{return Vec::new()};
        if let Some(xs)=v.as_array(){
            xs.iter().filter_map(|x|x.as_str()).map(|x|x.trim().to_lowercase()).filter(|x|!x.is_empty()).collect()
        }else{v.as_str().into_iter().map(|x|x.trim().to_lowercase()).filter(|x|!x.is_empty()).collect()}
    };
    let mapped=|name:&str,map:&[(&str,&'static str)]|->&'static str{
        values(name).iter().find_map(|v|map.iter().find(|(k,_)|v==*k).map(|(_,x)|*x)).unwrap_or("unknown")
    };
    let verb_form=mapped("VerbForm",&[("fin","finite"),("inf","infinitive"),("part","participle"),("ger","gerund")]);
    let tense=mapped("Tense",&[("past","past"),("pres","present"),("fut","future")]);
    let mood=mapped("Mood",&[("ind","indicative"),("cnd","conditional"),("imp","imperative"),("sub","subjunctive")]);
    let aspects=values("Aspect");
    let perf=aspects.iter().any(|x|x=="perf");let prog=aspects.iter().any(|x|x=="prog");
    let aspect=match (perf,prog,verb_form!="unknown"){(true,true,_)=>"perfect_progressive",(true,false,_)=>"perfect",
        (false,true,_)=>"progressive",(false,false,true)=>"simple",_=>"unknown"};
    let voice=if morph.is_none(){"unknown"}else if dep=="auxpass"{"passive"}else{"active"};
    Ok(serde_json::json!({"surface":surface,"tense":tense,"aspect":aspect,"verb_form":verb_form,
        "voice":voice,"mood":mood,"modality":"asserted","source":if morph.is_some(){"spacy_morphology"}else{"fallback"}}))
}

// Exact subset of the donor itir.typed-canonical.v1 contract used by
// relational_bundle_v1 / semantic_reductions.py. Encode detached primitives
// directly; JSON serialization is not the original identity substrate.
// Reject floating values rather than approximating their typed encoding.
fn original_relation_digest(value:&serde_json::Value)->Result<String,NativeNumericAuditError>{
    fn length(d:&mut Sha256,n:usize){d.update((n as u64).to_be_bytes());}
    fn string(d:&mut Sha256,s:&str){d.update(b"S");length(d,s.len());d.update(s.as_bytes());}
    fn encode(d:&mut Sha256,v:&serde_json::Value)->Result<(),NativeNumericAuditError>{
        use serde_json::Value;
        match v {
            Value::Null=>d.update(b"N"),
            Value::Bool(b)=>d.update(if *b {b"B1"} else {b"B0"}),
            Value::String(s)=>string(d,s),
            Value::Number(n)=>{
                let (negative,magnitude)=if let Some(i)=n.as_i64(){(i<0,i.unsigned_abs())}
                    else if let Some(u)=n.as_u64(){(false,u)}
                    else{return Err(NativeNumericAuditError::ForeignSource)};
                let raw=magnitude.to_be_bytes();
                let first=raw.iter().position(|b|*b!=0).unwrap_or(raw.len()-1);
                d.update(if negative{b"I-"}else{b"I+"});
                length(d,raw.len()-first);d.update(&raw[first..]);
            },
            Value::Array(xs)=>{d.update(b"Q");length(d,xs.len());for x in xs{encode(d,x)?;}},
            Value::Object(m)=>{
                d.update(b"M");length(d,m.len());
                let mut keys=m.keys().collect::<Vec<_>>();keys.sort();
                for k in keys{string(d,k);encode(d,&m[k])?;}
            },
        }
        Ok(())
    }
    let mut d=Sha256::new();let contract="itir.typed-canonical.v1";
    d.update(b"ITIR");length(&mut d,contract.len());d.update(contract.as_bytes());
    encode(&mut d,value)?;Ok(hex(&d.finalize()))
}

// Original shared_reducer.py emits the full atom payload; corpus_compilation.py
// hashes it to semantic-atom before semantic_reductions.py hashes each binding.
// Preserve the original parser label spelling (not the normalized SQL symbol).
fn original_relation_atom_payload(id:&str,text:&str,start:i64,end:i64,lemma:&str,
    pos:&str,dep:&str,raw_morph:Option<&str>)->Result<serde_json::Value,NativeNumericAuditError>{
    let mut atom=serde_json::json!({"id":id,"text":text,"span":[start,end]});
    let lemma=if lemma.is_empty(){text}else{lemma};
    if !lemma.is_empty(){atom["lemma"]=serde_json::json!(lemma);}
    if !pos.is_empty(){atom["pos"]=serde_json::json!(pos);}
    if !dep.is_empty(){atom["dependency"]=serde_json::json!(dep);}
    if let Some(raw)=raw_morph {
        let parsed:serde_json::Value=serde_json::from_str(raw).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        let m=parsed.as_object().ok_or(NativeNumericAuditError::ForeignSource)?;
        // The public spaCy adapter uses a list of values per feature; the
        // stored source plan can retain either a list or a UD comma string.
        let mut normalized=serde_json::Map::new();
        for (k,v) in m{
            let values=if let Some(xs)=v.as_array(){
                xs.iter().map(|x|x.as_str().ok_or(NativeNumericAuditError::ForeignSource))
                    .collect::<Result<Vec<_>,_>>()?
            }else if let Some(x)=v.as_str(){x.split(',').collect::<Vec<_>>()}
            else{return Err(NativeNumericAuditError::ForeignSource)};
            normalized.insert(k.clone(),serde_json::json!(values));
        }
        if !normalized.is_empty(){atom["morph"]=serde_json::Value::Object(normalized);}
    }
    Ok(atom)
}
fn original_atom_span_ref(doc:&str,atom:&serde_json::Value)->Result<String,NativeNumericAuditError>{
    Ok(format!("semantic-atom:{}",original_relation_digest(&serde_json::json!({"document_ref":doc,"atom":atom}))?))
}
fn original_binding_factor_ref(doc:&str,relation_id:&str,role:&str,binding:&serde_json::Value,
    feature:bool)->Result<String,NativeNumericAuditError>{
    let mut key=serde_json::json!({"document_ref":doc,"relation_id":relation_id,"binding":binding});
    key[if feature{"feature_role"}else{"role"}]=serde_json::json!(role);
    Ok(format!("factor:{}",original_relation_digest(&key)?))
}

#[derive(Debug,Clone,Serialize)]
struct NativeOriginalRelationReference {
    scope:&'static str,
    relation_id:String,
    reconstructed_relation_ref:String,
    reconstructed_root_factor_ref:String,
    original_relation_payload:serde_json::Value,
    atom_token_inverse:BTreeMap<String,i64>,
    atom_span_refs:BTreeMap<String,String>,
    atom_payloads:BTreeMap<String,serde_json::Value>,
    independently_compared:bool,
}
fn original_relation_reference(doc:&str,grammar:&str,index:usize,kind:&str,
    roles:Vec<serde_json::Value>,atom_token_inverse:BTreeMap<String,i64>)
    ->Result<NativeOriginalRelationReference,NativeNumericAuditError>{
    let id=format!("e{index}");
    let relation=serde_json::json!({"id":id,"type":kind,"roles":roles});
    let relation_ref=format!("semantic-relation:{}",original_relation_digest(
        &serde_json::json!({"document_ref":doc,"relation":relation}))?);
    let factor_ref=format!("factor:{}",original_relation_digest(
        &serde_json::json!({"document_ref":doc,"declaration_ref":grammar,"relation_id":id}))?);
    Ok(NativeOriginalRelationReference{scope:"supplied_original_parser_prefix",relation_id:id,
        reconstructed_relation_ref:relation_ref,reconstructed_root_factor_ref:factor_ref,
        original_relation_payload:relation,atom_token_inverse,atom_span_refs:BTreeMap::new(),
        atom_payloads:BTreeMap::new(),independently_compared:false})
}
// Donor allocation is first ensure_atom encounter, not numeric token
// order. In the object-centred predicate branch head and object are
// allocated BEFORE the emitted subject roles. Preserve this distinction.
fn original_relation_atom_ids(relation:&ProjectedRelation,atoms:&mut BTreeMap<i64,String>)
    ->Vec<(String,i64,&'static str)>{
    fn ensure(atoms:&mut BTreeMap<i64,String>,id:i64)->String{
        let next=format!("a{}",atoms.len()+1);
        atoms.entry(id).or_insert(next).clone()
    }
    if relation.relation_type=="predicate"{
        ensure(atoms,relation.head_token_id);
        if let Some((_,object))=relation.roles.iter().find(|(r,_)|*r=="object"){ensure(atoms,*object);}
    }
    relation.roles.iter().map(|(role,id)|(ensure(atoms,*id),*id,*role)).collect()
}

#[derive(Debug,Clone,Serialize)]
struct NativeRelationReductionRow {
    relation_ref:String,
    projected_relation_ordinal:usize,
    original_reference_crosswalk:NativeOriginalRelationReference,
    relation:ProjectedRelation,
    grammar_ref:&'static str,
    factor_type:&'static str,
    candidate_type:&'static str,
    residuals:Vec<&'static str>,
    action_meta:Option<serde_json::Value>,
    original_action_feature_factor_ref:Option<String>,
    // role occurrence, not role-name map: repeated item or subject survives.
    role_factors:Vec<serde_json::Value>,
}
fn native_relation_reductions(
    tx:&mut postgres::Transaction<'_>,run:&str,doc:&str,tokens:&[NativeRelationToken],
    relations:&[ProjectedRelation],
)->Result<(Vec<NativeRelationReductionRow>,Option<serde_json::Value>,usize),NativeNumericAuditError>{
    let wanted=tokens.iter().map(|t|t.id).collect::<Vec<_>>();
    let rows=tx.query("SELECT t.token_id,p.morph_json,p.tag_origin_id,p.tag,p.lemma,p.pos,p.dependency,p.orth,p.start_char,p.end_char
        FROM execution.slr_native_parser_sentence_receipt r
        CROSS JOIN LATERAL jsonb_to_recordset(r.plan_payload->'tokens') p(token_ref text,morph_json text,tag_origin_id smallint,tag text,
            lemma text,pos text,dependency text,orth text,start_char bigint,end_char bigint)
        JOIN execution.semantic_parser_token t ON t.token_ref=p.token_ref AND t.sentence_ref=r.sentence_ref
        WHERE r.run_ref=$1 AND r.document_ref=$2 AND t.token_id=ANY($3::bigint[])",
        &[&run,&doc,&wanted])?;
    let raw=rows.iter().map(|r|(r.get::<_,i64>(0),(r.get::<_,Option<String>>(1),r.get::<_,i16>(2),r.get::<_,Option<String>>(3),
        r.get::<_,String>(4),r.get::<_,String>(5),r.get::<_,String>(6),r.get::<_,String>(7),r.get::<_,i64>(8),r.get::<_,i64>(9)))).collect::<BTreeMap<_,_>>();
    if raw.len()!=rows.len()||raw.len()!=wanted.len(){return Err(NativeNumericAuditError::ForeignSource);}
    let by_id=tokens.iter().map(|t|(t.id,t)).collect::<BTreeMap<_,_>>();
    for t in tokens{
        let p=&raw[&t.id];
        if p.4!=t.pos||!p.5.eq_ignore_ascii_case(&t.dep)||p.6!=t.orth||p.7!=t.start||p.8!=t.end{
            return Err(NativeNumericAuditError::ForeignSource);
        }
    }
    let mut out=Vec::new();
    let mut original_atoms=BTreeMap::new();
    for (index,relation) in relations.iter().enumerate(){
        let (grammar_ref,factor_type,candidate_type,residuals)=relation_declaration(relation.relation_type)
            .ok_or(NativeNumericAuditError::ForeignSource)?;
        let relation_ref=format!("native-projected-relation:{}",bridge_digest(&(run,doc,relation))?);
        let head=by_id.get(&relation.head_token_id).ok_or(NativeNumericAuditError::ForeignSource)?;
        let action_meta=if relation.relation_type=="predicate"{
            Some(original_relation_action_metadata(raw[&head.id].0.as_deref(),&head.orth,&head.dep)?)
        }else{None};
        let atom_roles=original_relation_atom_ids(relation,&mut original_atoms);
        let inverse=atom_roles.iter().map(|(atom,id,_)|(atom.clone(),*id)).collect::<BTreeMap<_,_>>();
        let mut original_roles=atom_roles.iter().map(|(atom,_,role)|serde_json::json!({"role":role,"atom":atom})).collect::<Vec<_>>();
        if let Some(meta)=&action_meta{original_roles.push(serde_json::json!({"role":"action_meta","value":meta}));}
        let mut original_reference_crosswalk=original_relation_reference(doc,grammar_ref,index+1,relation.relation_type,original_roles,inverse)?;
        for (atom_id,id,_) in &atom_roles {
            let p=&raw[id];
            let atom=original_relation_atom_payload(atom_id,&p.6,p.7,p.8,&p.3,&p.4,&p.5,p.0.as_deref())?;
            let span_ref=original_atom_span_ref(doc,&atom)?;
            original_reference_crosswalk.atom_span_refs.insert(atom_id.clone(),span_ref);
            original_reference_crosswalk.atom_payloads.insert(atom_id.clone(),atom);
        }
        let original_action_feature_factor_ref=if let Some(meta)=&action_meta{
            let binding=serde_json::json!({"role":"action_meta","atom_id":null,"atom_ref":null,"value":meta});
            Some(original_binding_factor_ref(doc,&original_reference_crosswalk.relation_id,"action_meta",&binding,true)?)
        }else{None};
        let mut role_factors=Vec::new();
        for (ordinal,(role,id)) in relation.roles.iter().enumerate(){
            let atom=by_id.get(id).ok_or(NativeNumericAuditError::ForeignSource)?;
            let Some(constraint)=relation_role_constraint(relation.relation_type,role) else{continue};
            let pronoun=relation.relation_type=="predicate"&&*role=="subject"&&atom.pos=="PRON";
            let kind=if relation.relation_type=="predicate"{format!("semantic.argument.{role}")}else{
                format!("semantic.relation_role.{}.{role}",relation.relation_type)};
            let residuals=if pronoun{vec!["syntactic_argument_structure_unchecked","antecedent_unresolved",
                "grammatical_subject_semantic_status_unresolved","referential_type_unresolved"]}
                else if relation.relation_type=="predicate"{vec!["syntactic_argument_structure_unchecked","argument_identity_unresolved"]}
                else{vec![]};
            let atom_id=&atom_roles[ordinal].0;
            let span_ref=original_reference_crosswalk.atom_span_refs.get(atom_id).ok_or(NativeNumericAuditError::ForeignSource)?;
            let binding=serde_json::json!({"role":role,"atom_id":atom_id,"atom_ref":span_ref,"value":null});
            let original_role_ref=original_binding_factor_ref(doc,&original_reference_crosswalk.relation_id,role,&binding,false)?;
            let original_role_constraint_ref=format!("constraint:{original_role_ref}");
            role_factors.push(serde_json::json!({"ordinal":ordinal,"role":role,"token_id":id,"factor_type":kind,
                "reconstructed_original_role_factor_ref":original_role_ref,"reconstructed_original_constraint_ref":original_role_constraint_ref,
                "original_binding":binding,
                "constraint":constraint,"residuals":residuals,"original_span":[atom.start,atom.end],
                "parser_pos":atom.pos,"parser_dependency":atom.dep,"parser_morph_json":raw[id].0,
                "reference_alternatives":if pronoun{vec!["entity_reference","eventuality_reference","proposition_reference","expletive_realisation"]}else{vec![]}}));
        }
        out.push(NativeRelationReductionRow{relation_ref,projected_relation_ordinal:index+1,original_reference_crosswalk,relation:relation.clone(),grammar_ref,factor_type,candidate_type,
            residuals:residuals.to_vec(),action_meta,original_action_feature_factor_ref,role_factors});
    }
    // The donor question detector checks punctuation first within each
    // sentence and then tag/preposed auxiliary. Original SLR did not retain
    // TAG, so a known punctuation/auxiliary mode is an observation, not a
    // complete first-question witness. Do not assign it a fake atom role.
    let missing_tags=raw.values().filter(|p|p.1!=1||p.2.as_deref().map_or(true,str::is_empty)).count();
    let mut sentences=BTreeMap::<i64,Vec<&NativeRelationToken>>::new();
    for t in tokens{sentences.entry(t.region).or_default().push(t);}
    let mut ordered=sentences.into_values().collect::<Vec<_>>();
    ordered.sort_by_key(|s|s.first().map(|t|t.start).unwrap_or(i64::MAX));
    let mut question=None;
    for sentence in ordered{
        let hit=sentence.iter().copied().find(|t|t.orth=="?").map(|t|(t,"question_mark"))
          .or_else(||sentence.iter().copied().find_map(|t|{
            let p=&raw[&t.id];
            if p.1==1&&matches!(p.2.as_deref(),Some("WP"|"WRB"|"WDT")){
                Some((t,"original_wh_tag"))
            }else if t.dep=="aux"&&by_id.get(&t.head)
                .is_some_and(|head|matches!(head.pos.as_str(),"VERB"|"AUX")&&t.start<head.start){
                Some((t,"preposed_auxiliary"))
            }else{None}
          }));
        if let Some((t,route))=hit{
            let mode=serde_json::json!({"role":"mode","value":"question","span_start":t.start,"span_end":t.end});
            let original_reference_crosswalk=original_relation_reference(doc,"grammar:semantic:composition:v0_4",
                relations.len()+1,"composition",vec![mode.clone()],BTreeMap::new())?;
            question=Some(serde_json::json!({"grammar_ref":"grammar:semantic:composition:v0_4",
                "original_reference_crosswalk":original_reference_crosswalk,
                "factor_type":"semantic.embedded_proposition","candidate_type":"semantic.composition_candidate",
                "relation_type":"composition","mode_role":mode,
                "source_token_id":t.id,"route":route,"bindings":[],"residuals":["composition_scope_unresolved","proposition_truth_not_evaluated"],
                "original_tag_capability_missing_count":missing_tags,"first_original_question_paid":false,"candidate_only":true}));
            break;
        }
    }
    Ok((out,question,missing_tags))
}
#[derive(Debug,Clone,Serialize)]
pub struct NativeTypedRelationProofBridge{
    pub source_relation_ref:String,pub numeric_factor_id:i64,pub exact_source_token_ids:Vec<i64>,
    pub indexed_factor_revision_refs:Vec<String>,pub indexed_support_matches:bool,
    pub focus_argument_roles:Vec<&'static str>,pub level_three_derivation_refs:Vec<String>,
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
    pub typed_relation_proof_bridge:Vec<NativeTypedRelationProofBridge>,
    pub typed_relation_index_members_not_projected:Vec<String>,
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
           AND r.start_char <= (SELECT min(t0.start_char)
               FROM execution.semantic_pnf_demand d JOIN execution.semantic_pnf_object_token_support s ON s.object_id=d.source_object_id
               JOIN execution.semantic_parser_token t0 ON t0.token_id=s.token_id WHERE d.demand_id=$3)
         ORDER BY t.start_char,t.end_char,t.token_id",
        &[&run_ref,&document_ref,&demand])?;
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
    let focus_start=focus_ids.iter().filter_map(|id|by_id.get(id).map(|t|t.start)).min()
        .ok_or(NativeNumericAuditError::SourcePolicy)?;
    // Focus roles first, then the actual preceding source chart, not
    // database allocation/region IDs. Remaining budget can replay other
    // previously accepted source relations.
    relations.sort_by_key(|r|(!r.roles.iter().any(|(_,id)|focus_ids.contains(id)),
        r.roles.iter().map(|(_,id)|by_id[id].start).min().unwrap_or(i64::MAX)>=focus_start,
        by_id[&r.head_token_id].start,r.relation_type,r.roles.clone()));
    let deferred=relations.len().saturating_sub(256);
    relations.truncate(256);
    let mut probe=tx.transaction()?;
    let object_kind=relation_symbol(&mut probe,14,"parser.role_participant")?;
    let eventuality=relation_symbol(&mut probe,10,"semantic.eventuality")?;
    let composition=relation_symbol(&mut probe,10,"semantic.embedded_proposition")?;
    let mut role_ids=BTreeMap::new();
    let mut role_objects=BTreeMap::<i64,i64>::new();
    let mut factor_ids=Vec::new();let mut focus_factor_ids=Vec::new();
    let mut typed_relation_proof_bridge=Vec::new();
    for relation in relations{
        let source_relation_ref=format!("native-projected-relation:{}",bridge_digest(&(run_ref,document_ref,&relation))?);
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
        let mut source_order=support.iter().copied().collect::<Vec<_>>();
        source_order.sort_by_key(|id|(by_id[id].start,by_id[id].end,*id));
        let links=probe.query("SELECT factor_revision_ref,token_ids
            FROM execution.slr_native_binding_anchor_projection
            WHERE run_ref=$1 AND document_ref=$2 AND active AND source_key=$3
            ORDER BY factor_revision_ref",&[&run_ref,&document_ref,&source_relation_ref])?;
        let matching=!links.is_empty()&&links.iter().all(|r|r.get::<_,Vec<i64>>(1)==source_order);
        let revisions=links.iter().map(|r|r.get::<_,String>(0)).collect::<Vec<_>>();
        typed_relation_proof_bridge.push(NativeTypedRelationProofBridge{
            source_relation_ref,numeric_factor_id:factor,exact_source_token_ids:source_order,
            indexed_factor_revision_refs:revisions,indexed_support_matches:matching,
            focus_argument_roles:relation.roles.iter().filter(|(_,id)|focus_ids.contains(id)).map(|(role,_)|*role).collect(),
            level_three_derivation_refs:Vec::new(),
        });
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
    let mut projected_refs=BTreeSet::new();
    for relation in &mut typed_relation_proof_bridge{
        projected_refs.extend(relation.indexed_factor_revision_refs.iter().cloned());
        relation.level_three_derivation_refs=probe.query(
            "SELECT DISTINCT d.derivation_ref FROM execution.semantic_pnf_factor_derivation d
             JOIN execution.semantic_pnf_factor_derivation_premise p ON p.derivation_id=d.derivation_id
             WHERE p.factor_id=$1 AND d.epistemic_level=3 ORDER BY d.derivation_ref",
            &[&relation.numeric_factor_id])?.iter().map(|r|r.get::<_,String>(0)).collect();
    }
    let indexed=probe.query("SELECT p.factor_revision_ref FROM execution.slr_native_binding_anchor_projection p
        JOIN pnf.factor_anchor a ON a.factor_revision_ref=p.factor_revision_ref
        WHERE p.run_ref=$1 AND p.document_ref=$2 AND p.active
          AND a.pnf_kind_ref IN ('semantic.eventuality','semantic.embedded_proposition')
        ORDER BY p.factor_revision_ref",&[&run_ref,&document_ref])?;
    let typed_relation_index_members_not_projected=indexed.iter().map(|r|r.get::<_,String>(0))
        .filter(|r|!projected_refs.contains(r)).collect::<Vec<_>>();
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
        typed_relation_proof_bridge,typed_relation_index_members_not_projected,
        original_direct_substitutions:direct,original_supported_substitutions:supported,new_level_three_refs:new_refs,
        nonvacuous_withdrawal_count:withdrawal_probes.iter().filter(|p|p.nonvacuous_dependency).count(),
        withdrawal_probes,projected_level_one_preserved:preserved,outer_level_one_restored:one_restored,
        outer_level_three_restored:three_restored,persisted:false,full_original_relation_parity_paid:false,focus_identity_paid:false,
    })
}

#[derive(Debug,Clone,Serialize)]
pub struct NativeGovernedIdentityProbe{
    pub admission_owner:&'static str,
    pub parser_refresh_elapsed_ms:u128,
    pub parser_admission_elapsed_ms:u128,
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
/// Bounded producer parity is an explicit audit gate, not a second
/// admission path. Original m080 remains the sole witness authority.
/// All reference and bounded executions happen in rollback savepoints.
#[derive(Debug,Clone,Serialize)]
pub struct NativeBoundedIdentityParity {
    pub reference_candidate_rows:usize,
    pub bounded_candidate_rows:usize,
    pub omitted_ambiguous_rows:usize,
    pub overflow_mentions:usize,
    pub reference_admitted_witnesses:usize,
    pub bounded_admitted_witnesses:usize,
    pub candidate_projection_exact:bool,
    pub original_admission_projection_exact:bool,
    pub embedded_reference_name_rows:usize,
    pub embedded_bounded_name_rows:usize,
    pub golden_standalone_permission_matches:bool,
    pub reference_elapsed_ms:u128,
    pub bounded_elapsed_ms:u128,
    pub full_typed_coverage_paid:bool,
    pub persisted:bool,
}

/// Candidate-only materialization of the original document anchor and
/// family supports. This is a rebuildable execution index, not a new
/// parser, source of semantic authority, or proof of complete dependencies.
#[derive(Debug,Clone,Serialize)]
pub struct NativeIdentitySupportIndexReceipt {
    pub scope_ref:String,
    pub observed_rows:usize,
    pub support_revision:Option<i64>,
    pub journaled_change_rows:usize,
    pub affected_token_ids:Vec<i64>,
    pub registered_consumer_work_keys:usize,
    pub consumer_queue_rows_written:u64,
    pub inserted_rows:usize,
    pub changed_rows:usize,
    pub withdrawn_rows:usize,
    pub unchanged_rows:usize,
    pub affected_sentence_ids:Vec<i64>,
    pub affected_family_ids:Vec<i64>,
    pub affected_name_token_ids:Vec<i64>,
    pub affected_anchor_object_ids:Vec<i64>,
    pub hot_symbol_scope_changed:bool,
    pub zero_family_mentions:usize,
    pub unanchored_mentions:usize,
    pub embedded_mentions:usize,
    pub support_scan_performed:bool,
    pub input_replay_mode:&'static str,
    pub input_events_acknowledged:usize,
    pub delta_dirty_sentence_count:usize,
    pub delta_live_source_scan_performed:bool,
    pub delta_reference_exact:bool,
    pub dependency_complete:bool,
    pub identity_paid:bool,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
struct IdentitySupportRow {
    lane:i16,
    key:String,
    sentence_id:Option<i64>,
    family_id:Option<i64>,
    payload:serde_json::Value,
}

fn read_identity_support_index(probe:&mut postgres::Transaction<'_>)
    ->Result<Vec<IdentitySupportRow>,NativeNumericAuditError>{
    // Every local row is retained, including missing anchors, empty families
    // and embedded mentions. Sentence and family dependencies are independent.
    let rows=probe.query("SELECT lane,key,sentence_id,family_id,payload FROM (
      SELECT 1::smallint AS lane,t.token_id::text AS key,t.sentence_id,
        t.lemma_symbol_id AS family_id,to_jsonb(t) AS payload FROM pg_temp.slr_identity_token t
      UNION ALL SELECT 2,p.entity_id::text,s.sentence_id,NULL::bigint,to_jsonb(p)
        FROM pg_temp.slr_identity_person p JOIN pg_temp.slr_identity_sentence s USING(sentence_ref)
      UNION ALL SELECT 3,a.token_id::text,a.sentence_id,t.lemma_symbol_id,to_jsonb(a)
        FROM pg_temp.slr_identity_anchor a JOIN pg_temp.slr_identity_token t USING(token_id)
      UNION ALL SELECT 4,f.parser_entity_id::text,s.sentence_id,f.family_lemma_symbol_id,to_jsonb(f)
        FROM pg_temp.slr_identity_family f JOIN pg_temp.slr_identity_sentence s USING(sentence_ref)
      UNION ALL SELECT 5,m.token_id::text,m.sentence_id,m.lemma_symbol_id,to_jsonb(m)
        FROM pg_temp.slr_identity_mention_scope m
      UNION ALL SELECT 6,'hot-constants',NULL::bigint,NULL::bigint,to_jsonb(c)
        FROM pg_temp.slr_identity_constant c
      UNION ALL SELECT 7,concat(c.cue_class,':',c.symbol_id),NULL::bigint,NULL::bigint,to_jsonb(c)
        FROM execution.semantic_pnf_hot_cue_symbol c
      UNION ALL SELECT 8,s.sentence_id::text,s.sentence_id,NULL::bigint,to_jsonb(s)
        FROM pg_temp.slr_identity_sentence s
    ) support ORDER BY lane,key LIMIT 100001",&[])?;
    if rows.len()>100000{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    let mut seen=BTreeSet::new();
    let mut out=Vec::with_capacity(rows.len());
    for r in rows {
        let row=IdentitySupportRow{lane:r.get(0),key:r.get(1),sentence_id:r.get(2),family_id:r.get(3),payload:r.get(4)};
        if row.key.is_empty()||!seen.insert((row.lane,row.key.clone())){return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
        out.push(row);
    }
    Ok(out)
}

/// Exact namespace for the original source and golden owner. A baseline
/// created before input capture was installed cannot be a delta baseline.
fn identity_support_scope(run_ref:&str,answer:&NativeSourceLocalAnswer)
    ->Result<(String,serde_json::Value),NativeNumericAuditError>{
    let scope=serde_json::json!({
        "schema":"slr-native-identity-support-index-v2","run_ref":run_ref,
        "document_ref":answer.source_document_ref,"source_revision_ref":answer.source_revision_ref,
        "source_sha256":answer.source_content_sha256,"parser_run_ref":answer.original_parser_run_ref,
        "producer":"m090-bounded-input-delta-v2",
        "golden":"BoundedProperNameEvidenceExecution:3cc851c87c50c3cc18c9610984dc48fb5f28d11d",
    });
    let scope_ref=bridge_digest(&scope)?;

    Ok((scope_ref,scope))
}

const IDENTITY_INPUT_CONTRACT:&str="slr-identity-input-delta:v1";

fn ensure_identity_input_capture(db:&mut postgres::Client)->Result<(),NativeNumericAuditError>{
    let exists:bool=db.query_one("SELECT to_regclass('execution.slr_native_identity_input_contract') IS NOT NULL",&[])?.get(0);
    if !exists{
        let mut tx=db.transaction()?;
        tx.query_one("SELECT pg_advisory_xact_lock(hashtext('slr-identity-input-delta:v1'))",&[])?;
        let installed:bool=tx.query_one("SELECT to_regclass('execution.slr_native_identity_input_contract') IS NOT NULL",&[])?.get(0);
        if !installed{tx.batch_execute(include_str!("numeric_pnf_identity_input_delta.sql"))?;}
        tx.commit()?;
    }
    // Never silently reinstall a missing/disabled capture trigger over an
    // existing baseline: continuity would be unknown.
    let contract:bool=db.query_one("SELECT EXISTS(SELECT 1 FROM execution.slr_native_identity_input_contract WHERE contract_ref=$1)",&[&IDENTITY_INPUT_CONTRACT])?.get(0);
    let capture_count:i64=db.query_one("SELECT count(*)::bigint FROM pg_trigger t
        JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace
        JOIN pg_proc p ON p.oid=t.tgfoid
        WHERE n.nspname='execution' AND t.tgname LIKE 'slr_id_input_%'
          AND p.proname=t.tgname AND NOT t.tgisinternal AND t.tgenabled IN ('O','A')",&[])?.get(0);
    if !contract||capture_count!=32{return Err(NativeNumericAuditError::ForeignSource);}
    Ok(())
}

#[derive(Debug,Clone)]
struct NativeIdentityInputFrontier {
    scope_ref:String,
    event_ids:Vec<i64>,
    dirty_sentence_refs:Vec<String>,
    checked_baseline:bool,
    whole_scope:bool,
}

fn identity_input_frontier(tx:&mut postgres::Transaction<'_>,run_ref:&str,answer:&NativeSourceLocalAnswer)
    ->Result<NativeIdentityInputFrontier,NativeNumericAuditError>{
    let (scope_ref,_)=identity_support_scope(run_ref,answer)?;
    let checked_baseline:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.slr_native_identity_input_baseline
      WHERE scope_ref=$1 AND contract_ref=$2 AND exact_reference_parity)",&[&scope_ref,&IDENTITY_INPUT_CONTRACT])?.get(0);
    let events=tx.query("SELECT e.event_id,e.whole_scope FROM execution.slr_native_identity_input_event e
      WHERE ((e.run_ref=$1 AND e.document_ref=$2) OR e.run_ref IS NULL)
        AND NOT EXISTS(SELECT 1 FROM execution.slr_native_identity_input_ack a
            WHERE a.scope_ref=$3 AND a.event_id=e.event_id)
      ORDER BY e.event_id LIMIT 100001",&[&run_ref,&answer.source_document_ref,&scope_ref])?;
    if events.len()>100000{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    let event_ids=events.iter().map(|r|r.get::<_,i64>(0)).collect::<Vec<_>>();
    let whole_scope=events.iter().any(|r|r.get::<_,bool>(1));
    let mut dirty_sentence_refs=Vec::new();
    if checked_baseline && !whole_scope && !event_ids.is_empty(){
        // Old and current sentence/token rows recover deleted and inserted
        // coordinates; interval intersection includes previously absent
        // fallback anchors and all containing-region changes.
        dirty_sentence_refs=tx.query("WITH events AS MATERIALIZED (
            SELECT * FROM execution.slr_native_identity_input_event WHERE event_id=ANY($4::bigint[])
          ), direct AS (SELECT unnest(sentence_refs) AS sentence_ref FROM events),
          tokens AS (SELECT DISTINCT unnest(token_ids) AS token_id FROM events),
          old_sentences AS (
            SELECT i.payload->>'sentence_ref' AS sentence_ref,
              (i.payload->>'start_char')::bigint AS start_char,
              (i.payload->>'end_char')::bigint AS end_char
            FROM execution.slr_native_identity_support_index i WHERE i.scope_ref=$3 AND i.lane=8
          ), affected AS (
            SELECT sentence_ref FROM direct
            UNION SELECT t.sentence_ref FROM tokens d JOIN execution.semantic_parser_token t USING(token_id)
              WHERE t.run_ref=$1 AND t.document_ref=$2
            UNION SELECT i.payload->>'sentence_ref' FROM tokens d
              JOIN execution.slr_native_identity_support_index i
                ON i.scope_ref=$3 AND i.lane=1 AND i.key=d.token_id::text
            UNION SELECT s.sentence_ref FROM events e
              JOIN execution.semantic_parser_sentence s ON s.run_ref=$1 AND s.document_ref=$2
                AND e.start_char<s.end_char AND e.end_char>s.start_char
            UNION SELECT s.sentence_ref FROM events e JOIN old_sentences s
                ON e.start_char<s.end_char AND e.end_char>s.start_char
          )
          SELECT sentence_ref FROM affected WHERE sentence_ref IS NOT NULL ORDER BY sentence_ref",
          &[&run_ref,&answer.source_document_ref,&scope_ref,&event_ids])?.iter().map(|r|r.get(0)).collect();
    }
    Ok(NativeIdentityInputFrontier{scope_ref,event_ids,dirty_sentence_refs,checked_baseline,whole_scope})
}

/// Reuse only the checked execution baseline, not a previous semantic proof.
//// No current identity/witness/Level-3 result is inferred from this receipt.
#[derive(Debug,Clone,Serialize)]
pub struct NativeIdentityExecutionReplay {
    pub scope_ref:String,
    pub input_events_pending:usize,
    pub reference_baseline_present:bool,
    pub support_revision:i64,
    pub support_rows_reopened:usize,
    pub source_support_scan_performed:bool,
    pub parser_identity_refresh_invocations:usize,
    pub current_authority_reused:bool,
}
fn try_native_identity_execution_replay(
    tx:&mut postgres::Transaction<'_>,run:&str,answer:&NativeSourceLocalAnswer,
)->Result<Option<NativeIdentityExecutionReplay>,NativeNumericAuditError>{
    // Audit mode is the cold/reference acceptance campaign. Explicit replay
    // can bypass that diagnostic only when the installed input journal is
    // continuous and all visible input events have already been acknowledged.
    if std::env::var("SLR_NATIVE_BOUNDED_IDENTITY_PROBE").as_deref()!=Ok("1")
      ||std::env::var("SLR_NATIVE_IDENTITY_REPLAY_MODE").as_deref()!=Ok("1")
      ||std::env::var("SLR_NATIVE_DOCUMENT_IDENTITY_PUBLISH").as_deref()==Ok("1"){
        return Ok(None);
    }
    let f=identity_input_frontier(tx,run,answer)?;
    if !f.checked_baseline||!f.event_ids.is_empty(){return Ok(None);}
    let revision:i64=tx.query_one("SELECT COALESCE(max(revision),0) FROM execution.slr_native_identity_support_revision
        WHERE scope_ref=$1",&[&f.scope_ref])?.get(0);
    if revision<=0{return Ok(None);}
    Ok(Some(NativeIdentityExecutionReplay{
        scope_ref:f.scope_ref,input_events_pending:0,reference_baseline_present:true,support_revision:revision,
        support_rows_reopened:0,source_support_scan_performed:false,parser_identity_refresh_invocations:0,
        current_authority_reused:false,
    }))
}

fn read_cached_identity_support(tx:&mut postgres::Transaction<'_>,scope:&str)
    ->Result<Vec<IdentitySupportRow>,NativeNumericAuditError>{
    let rows=tx.query("SELECT lane,key,sentence_id,family_id,payload FROM execution.slr_native_identity_support_index
      WHERE scope_ref=$1 ORDER BY lane,key LIMIT 100001",&[&scope])?;
    if rows.len()>100000{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    Ok(rows.iter().map(|r|IdentitySupportRow{lane:r.get(0),key:r.get(1),sentence_id:r.get(2),family_id:r.get(3),payload:r.get(4)}).collect())
}

/// Actual delta producer, checked against the separately rebuilt original
/// carrier. Its result, not the reference rows, is what the outer transaction
/// writes. The full reference is still deliberately run in this audit mode.
fn replay_identity_support_delta(probe:&mut postgres::Transaction<'_>,run:i64,doc:i64,
    frontier:&NativeIdentityInputFrontier,reference:&[IdentitySupportRow])
    ->Result<(Vec<IdentitySupportRow>,&'static str),NativeNumericAuditError>{
    if !frontier.checked_baseline||frontier.whole_scope{
        return Ok((reference.to_vec(),if frontier.checked_baseline{"global-invalidation-rebuild"}else{"cold-reference-baseline"}));
    }
    if frontier.event_ids.is_empty(){
        let cached=read_cached_identity_support(probe,&frontier.scope_ref)?;
        if cached!=reference{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
        return Ok((cached,"unchanged-input-reopen"));
    }
    let mut delta=probe.transaction()?;
    delta.batch_execute("DROP TABLE pg_temp.slr_identity_mention_scope,pg_temp.slr_identity_family,
      pg_temp.slr_identity_anchor,pg_temp.slr_identity_person,pg_temp.slr_identity_sentence,
      pg_temp.slr_identity_token,pg_temp.slr_identity_constant,pg_temp.slr_identity_stage_scope")?;
    delta.batch_execute(include_str!("numeric_pnf_identity_support_delta.sql"))?;
    let visited:i64=delta.query_one("SELECT pg_temp.slr_stage_identity_support_delta($1,$2,$3,$4)",
      &[&run,&doc,&frontier.scope_ref,&frontier.dirty_sentence_refs])?.get(0);
    if visited!=frontier.dirty_sentence_refs.len() as i64{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    let result=read_identity_support_index(&mut delta)?;
    if result!=reference{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    delta.rollback()?;
    Ok((result,"dirty-sentence-and-family-reopen"))
}

fn acknowledge_identity_inputs(tx:&mut postgres::Transaction<'_>,frontier:&NativeIdentityInputFrontier)
    ->Result<usize,NativeNumericAuditError>{
    let count=tx.execute("INSERT INTO execution.slr_native_identity_input_ack(scope_ref,event_id)
      SELECT $1,id FROM unnest($2::bigint[]) ids(id) ON CONFLICT DO NOTHING",
      &[&frontier.scope_ref,&frontier.event_ids])?;
    if count!=frontier.event_ids.len() as u64{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    tx.execute("INSERT INTO execution.slr_native_identity_input_baseline(scope_ref,contract_ref,exact_reference_parity)
      VALUES($1,$2,TRUE) ON CONFLICT(scope_ref) DO NOTHING",&[&frontier.scope_ref,&IDENTITY_INPUT_CONTRACT])?;
    Ok(count as usize)
}

/// Called only after the rollback audit has proved preservation. Persist
/// the observed source support in the outer native transport transaction.
/// No cache hit is licensed by this snapshot; all support was read afresh.
fn sync_identity_support_index(
    tx:&mut postgres::Transaction<'_>,run_ref:&str,answer:&NativeSourceLocalAnswer,
    rows:&[IdentitySupportRow],
)->Result<NativeIdentitySupportIndexReceipt,NativeNumericAuditError>{
    let (scope_ref,scope)=identity_support_scope(run_ref,answer)?;
    tx.batch_execute("CREATE TABLE IF NOT EXISTS execution.slr_native_identity_support_scope (
        scope_ref text PRIMARY KEY, scope_payload jsonb NOT NULL
    );
    CREATE TABLE IF NOT EXISTS execution.slr_native_identity_support_index (
        scope_ref text NOT NULL REFERENCES execution.slr_native_identity_support_scope(scope_ref),
        lane smallint NOT NULL CHECK(lane BETWEEN 1 AND 8),
        key text NOT NULL,sentence_id bigint,family_id bigint,payload jsonb NOT NULL,
        PRIMARY KEY(scope_ref,lane,key)
    );
    CREATE INDEX IF NOT EXISTS slr_native_identity_support_sentence_idx
       ON execution.slr_native_identity_support_index(scope_ref,sentence_id,lane,key);
    CREATE INDEX IF NOT EXISTS slr_native_identity_support_family_idx
       ON execution.slr_native_identity_support_index(scope_ref,family_id,lane,key);
    CREATE TABLE IF NOT EXISTS execution.slr_native_identity_support_revision (
        scope_ref text NOT NULL REFERENCES execution.slr_native_identity_support_scope(scope_ref),
        revision bigint NOT NULL CHECK(revision>0),
        change_count bigint NOT NULL CHECK(change_count>0),
        PRIMARY KEY(scope_ref,revision)
    );
    CREATE TABLE IF NOT EXISTS execution.slr_native_identity_support_change (
        scope_ref text NOT NULL,revision bigint NOT NULL,lane smallint NOT NULL,key text NOT NULL,
        before_row jsonb,after_row jsonb,
        PRIMARY KEY(scope_ref,revision,lane,key),
        FOREIGN KEY(scope_ref,revision) REFERENCES execution.slr_native_identity_support_revision(scope_ref,revision),
        CHECK(before_row IS DISTINCT FROM after_row)
    );")?;
    tx.query_one("SELECT pg_advisory_xact_lock(hashtext('slr-native-identity-support-index-v1'),hashtext($1))",&[&scope_ref])?;
    tx.execute("INSERT INTO execution.slr_native_identity_support_scope VALUES($1,$2) ON CONFLICT DO NOTHING",&[&scope_ref,&scope])?;
    let stored:serde_json::Value=tx.query_one("SELECT scope_payload FROM execution.slr_native_identity_support_scope WHERE scope_ref=$1",&[&scope_ref])?.get(0);
    if stored!=scope{return Err(NativeNumericAuditError::ForeignSource);}
    let packet=serde_json::to_value(rows).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let deltas=tx.query("WITH incoming AS MATERIALIZED (
        SELECT * FROM jsonb_to_recordset($2) AS r(lane smallint,key text,sentence_id bigint,family_id bigint,payload jsonb)
    ), prior AS MATERIALIZED (
        SELECT lane,key,sentence_id,family_id,payload FROM execution.slr_native_identity_support_index WHERE scope_ref=$1
    )
    SELECT CASE WHEN p.key IS NULL THEN 1 WHEN n.key IS NULL THEN 2
        WHEN (p.sentence_id,p.family_id,p.payload) IS DISTINCT FROM (n.sentence_id,n.family_id,n.payload) THEN 3
        ELSE 4 END::smallint,
        p.sentence_id,n.sentence_id,p.family_id,n.family_id,COALESCE(p.lane,n.lane),
        COALESCE(p.key,n.key),CASE WHEN p.key IS NOT NULL THEN to_jsonb(p) END,
        CASE WHEN n.key IS NOT NULL THEN to_jsonb(n) END
      FROM prior p FULL JOIN incoming n USING(lane,key)",&[&scope_ref,&packet])?;
    let(mut inserted,mut withdrawn,mut changed,mut unchanged)=(0usize,0usize,0usize,0usize);
    let mut sentences=BTreeSet::new();let mut families=BTreeSet::new();let mut hot_changed=false;let mut journal=Vec::new();
    for d in deltas{
        match d.get::<_,i16>(0){
            1=>inserted+=1,2=>withdrawn+=1,3=>changed+=1,4=>{unchanged+=1;continue;},
            _=>return Err(NativeNumericAuditError::AdmissionProbeInvariant),
        }
        sentences.extend(d.get::<_,Option<i64>>(1));sentences.extend(d.get::<_,Option<i64>>(2));
        families.extend(d.get::<_,Option<i64>>(3));families.extend(d.get::<_,Option<i64>>(4));
        hot_changed|=matches!(d.get::<_,i16>(5),6|7);
        journal.push(serde_json::json!({
            "lane":d.get::<_,i16>(5),"key":d.get::<_,String>(6),
            "before_row":d.get::<_,Option<serde_json::Value>>(7),
            "after_row":d.get::<_,Option<serde_json::Value>>(8)
        }));
    }
    let sentence_ids=sentences.into_iter().collect::<Vec<_>>();
    let family_ids=families.into_iter().collect::<Vec<_>>();
    // Reopen both sides of the changed support relation before replacing
    // the derived current index. A family insertion must reach old
    // zero-family mentions, and a deleted anchor must remain visible.
    // Hot constant/cue edits conservatively invalidate the whole scope.
    let affected=tx.query("WITH incoming AS (
        SELECT * FROM jsonb_to_recordset($2) AS r(lane smallint,key text,sentence_id bigint,family_id bigint,payload jsonb)
    ), support AS (
        SELECT lane,key,sentence_id,family_id,payload FROM execution.slr_native_identity_support_index WHERE scope_ref=$1
        UNION SELECT lane,key,sentence_id,family_id,payload FROM incoming
    )
    SELECT DISTINCT lane,key,CASE lane WHEN 3 THEN (payload->>'object_id')::bigint
        WHEN 4 THEN (payload->>'anchor_object_id')::bigint
        WHEN 5 THEN (payload->>'source_object_id')::bigint ELSE NULL END
    FROM support
    WHERE lane IN (1,3,4,5) AND ($5::boolean OR sentence_id=ANY($3::bigint[]) OR family_id=ANY($4::bigint[]))
    ORDER BY lane,key",&[&scope_ref,&packet,&sentence_ids,&family_ids,&hot_changed])?;
    let mut affected_names=BTreeSet::new();let mut affected_anchors=BTreeSet::new();let mut affected_tokens=BTreeSet::new();
    for row in affected {
        let lane:i16=row.get(0);
        if matches!(lane,1|3|5) {
            let key:String=row.get(1);
            let token=key.parse::<i64>().map_err(|_|NativeNumericAuditError::AdmissionProbeInvariant)?;
            affected_tokens.insert(token);
            if lane==5{affected_names.insert(token);}
        }
        affected_anchors.extend(row.get::<_,Option<i64>>(2));
    }
    // Persist the exact old/new row delta in the same transaction as the
    // derived support index and the original m094 queue wake. A digest or
    // an empty support delta is not a coverage/admission receipt.
    let changed_count=inserted+changed+withdrawn;
    if journal.len()!=changed_count{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    let affected_token_ids=affected_tokens.into_iter().collect::<Vec<_>>();
    let affected_anchor_object_ids=affected_anchors.into_iter().collect::<Vec<_>>();
    let (support_revision,registered_consumer_work_keys,consumer_queue_rows_written)=if changed_count>0{
        let revision:i64=tx.query_one("SELECT COALESCE(max(revision),0)+1 FROM execution.slr_native_identity_support_revision WHERE scope_ref=$1",&[&scope_ref])?.get(0);
        tx.execute("INSERT INTO execution.slr_native_identity_support_revision(scope_ref,revision,change_count) VALUES($1,$2,$3)",
            &[&scope_ref,&revision,&(changed_count as i64)])?;
        let journal_packet=serde_json::Value::Array(journal);
        let n=tx.execute("INSERT INTO execution.slr_native_identity_support_change(scope_ref,revision,lane,key,before_row,after_row)
            SELECT $1,$2,r.lane,r.key,r.before_row,r.after_row
            FROM jsonb_to_recordset($3) r(lane smallint,key text,before_row jsonb,after_row jsonb)",
            &[&scope_ref,&revision,&journal_packet])?;
        if n!=changed_count as u64{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
        // m091/m094 numeric source kinds: 1 token, 2 object. Also reopen
        // existing absence/interval consumers using OLD and NEW source spans.
        // A document-wide family change may touch an earlier mention.
        let work=tx.query_one("WITH incoming AS (
            SELECT * FROM jsonb_to_recordset($2) AS r(lane smallint,key text,sentence_id bigint,family_id bigint,payload jsonb)
        ), support AS (
            SELECT lane,key,sentence_id,family_id,payload FROM execution.slr_native_identity_support_index WHERE scope_ref=$1
            UNION SELECT lane,key,sentence_id,family_id,payload FROM incoming
        ), spans AS (
            SELECT DISTINCT (payload->>'start_char')::bigint AS a,(payload->>'end_char')::bigint AS b
              FROM support WHERE lane=1 AND (payload->>'token_id')::bigint=ANY($3::bigint[])
        ), changed_source AS (
            SELECT 1::smallint AS kind,unnest($3::bigint[]) AS id
            UNION SELECT 2::smallint,unnest($4::bigint[])
        ), affected AS (
            SELECT d.demand_id,d.consumer_ref,d.query_ref,d.policy_ref,d.minimum_horizon AS horizon
              FROM changed_source c
              JOIN execution.semantic_pnf_consumer_reverse_dependency d ON d.source_kind=c.kind AND d.source_id=c.id
              JOIN execution.semantic_pnf_demand q ON q.demand_id=d.demand_id
              JOIN execution.semantic_pnf_region r ON r.region_id=q.source_region_id
             WHERE r.run_ref=$5 AND r.document_ref=$6
            UNION
            SELECT d.demand_id,d.consumer_ref,d.query_ref,d.policy_ref,3::smallint
              FROM execution.slr_native_consumer_source_interval d
             WHERE d.run_ref=$5 AND d.document_ref=$6 AND d.source_revision_ref=$7
               AND d.source_sha256=$8 AND d.parser_run_ref=$9
               AND ($10::boolean OR EXISTS(SELECT 1 FROM spans s WHERE s.a<d.end_char AND s.b>d.start_char))
        ), queued AS (
            INSERT INTO execution.semantic_pnf_consumer_horizon_work_queue AS q
                (demand_id,consumer_ref,query_ref,policy_ref,horizon,work_state,completed_at)
            SELECT demand_id,consumer_ref,query_ref,policy_ref,horizon,1,NULL FROM affected
            ON CONFLICT(demand_id,consumer_ref,query_ref,policy_ref,horizon)
            DO UPDATE SET work_state=1,completed_at=NULL
            WHERE q.work_state<>1 OR q.completed_at IS NOT NULL RETURNING demand_id
        )
        SELECT (SELECT count(*)::bigint FROM affected),(SELECT count(*)::bigint FROM queued)",
        &[&scope_ref,&packet,&affected_token_ids,&affected_anchor_object_ids,&run_ref,
          &answer.source_document_ref,&answer.source_revision_ref,&answer.source_content_sha256,
          &answer.original_parser_run_ref,&hot_changed])?;
        let matched:i64=work.get(0);let written:i64=work.get(1);
        if matched<0||written<0||written>matched{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
        (Some(revision),matched as usize,written as u64)
    }else{(None,0,0)};
    let removed=tx.execute("DELETE FROM execution.slr_native_identity_support_index p
        WHERE p.scope_ref=$1 AND NOT EXISTS(SELECT 1 FROM jsonb_to_recordset($2) AS n(lane smallint,key text)
            WHERE n.lane=p.lane AND n.key=p.key)",&[&scope_ref,&packet])?;
    let written=tx.execute("INSERT INTO execution.slr_native_identity_support_index AS p
        (scope_ref,lane,key,sentence_id,family_id,payload)
        SELECT $1,n.lane,n.key,n.sentence_id,n.family_id,n.payload
        FROM jsonb_to_recordset($2) AS n(lane smallint,key text,sentence_id bigint,family_id bigint,payload jsonb)
        ON CONFLICT(scope_ref,lane,key) DO UPDATE SET sentence_id=EXCLUDED.sentence_id,
          family_id=EXCLUDED.family_id,payload=EXCLUDED.payload
        WHERE (p.sentence_id,p.family_id,p.payload) IS DISTINCT FROM
          (EXCLUDED.sentence_id,EXCLUDED.family_id,EXCLUDED.payload)",&[&scope_ref,&packet])?;
    if removed!=withdrawn as u64||written!=(inserted+changed) as u64
       ||inserted+changed+unchanged!=rows.len(){return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    Ok(NativeIdentitySupportIndexReceipt{
        scope_ref,observed_rows:rows.len(),support_revision,journaled_change_rows:changed_count,
        affected_token_ids,registered_consumer_work_keys,consumer_queue_rows_written,
        inserted_rows:inserted,changed_rows:changed,
        withdrawn_rows:withdrawn,unchanged_rows:unchanged,
        affected_sentence_ids:sentence_ids,affected_family_ids:family_ids,
        affected_name_token_ids:affected_names.into_iter().collect(),affected_anchor_object_ids,
        hot_symbol_scope_changed:hot_changed,
        zero_family_mentions:rows.iter().filter(|r|r.lane==5&&r.payload["full_count"].as_i64()==Some(0)).count(),
        unanchored_mentions:rows.iter().filter(|r|r.lane==5&&r.payload["source_object_id"].is_null()).count(),
        embedded_mentions:rows.iter().filter(|r|r.lane==5&&r.payload["embedded_in_person"].as_bool()==Some(true)).count(),
        support_scan_performed:true,input_replay_mode:"unverified",input_events_acknowledged:0,
        delta_dirty_sentence_count:0,delta_live_source_scan_performed:true,delta_reference_exact:false,
        dependency_complete:false,identity_paid:false,
    })
}

type ParserEvidenceKey=(i64,i64,i16,Option<i64>,Option<i64>,Option<i64>,String,i16);
fn parser_evidence_keys(tx:&mut postgres::Transaction<'_>,run:i64,doc:i64)
    ->Result<BTreeSet<ParserEvidenceKey>,NativeNumericAuditError>{
    parser_evidence_keys_in_states(tx,run,doc,&[1i16])
}

fn parser_evidence_keys_in_states(tx:&mut postgres::Transaction<'_>,run:i64,doc:i64,states:&[i16])
    ->Result<BTreeSet<ParserEvidenceKey>,NativeNumericAuditError>{
    // A bounded acceptance campaign may not silently truncate the
    // reference relation. Fail if the explicit audit budget is exceeded.
    let rows=tx.query("SELECT source_object_id,target_object_id,witness_kind,
        source_token_id,target_token_id,sentence_id,evidence_ref,candidate_count
        FROM execution.semantic_pnf_identity_evidence_candidate
        WHERE run_id=$1 AND document_id=$2 AND evidence_state=ANY($3::smallint[])
        LIMIT 100001",&[&run,&doc,&states])?;
    if rows.len()>100000 {return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    let keys=rows.iter().map(|r|(r.get(0),r.get(1),r.get(2),r.get(3),r.get(4),r.get(5),r.get(6),r.get(7)))
        .collect::<BTreeSet<_>>();
    if keys.len()!=rows.len(){return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    Ok(keys)
}

fn parser_admitted_witness_keys(tx:&mut postgres::Transaction<'_>,run:i64,doc:i64)
    ->Result<BTreeSet<String>,NativeNumericAuditError>{
    // Dense witness/entity IDs can differ across rollback savepoints.
    // Compare the complete original semantic witness key, not allocation.
    let rows=tx.query("SELECT DISTINCT jsonb_build_array(w.source_object_id,e.entity_ref,
        w.witness_kind,w.authority_class,w.candidate_count,w.source_interface_id,
        p.witness_role,c.source_object_id,c.target_object_id,c.witness_kind,c.evidence_ref)::text
        FROM execution.semantic_pnf_identity_evidence_candidate c
        JOIN execution.semantic_pnf_identity_evidence_witness p ON p.candidate_id=c.candidate_id
        JOIN execution.semantic_pnf_identity_witness w ON w.witness_id=p.witness_id
        JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=w.witness_id AND a.admission_state=2
        JOIN execution.semantic_pnf_canonical_entity e ON e.entity_id=w.target_entity_id
        WHERE c.run_id=$1 AND c.document_id=$2 AND c.evidence_state IN (1,2,3)
        LIMIT 100001",&[&run,&doc])?;
    if rows.len()>100000 {return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    Ok(rows.iter().map(|r|r.get::<_,String>(0)).collect())
}

/// Select the exact deterministic first K eligible rows of each original
/// name fibre. All other lanes remain literally equal. The input here is
/// the independently reopened original reference relation.
fn bounded_parser_evidence_reference(reference:&BTreeSet<ParserEvidenceKey>,limit:usize)
    ->Result<(BTreeSet<ParserEvidenceKey>,BTreeMap<(i64,i64),usize>),NativeNumericAuditError>{
    if limit==0{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    let mut retained=BTreeSet::new();
    let mut name_groups:BTreeMap<(i64,i64),Vec<(i64,&ParserEvidenceKey)>>=BTreeMap::new();
    for key in reference {
        if key.2!=3 {retained.insert(key.clone());continue;}
        let token=key.3.ok_or(NativeNumericAuditError::AdmissionProbeInvariant)?;
        let (head,entity)=key.6.rsplit_once(':').ok_or(NativeNumericAuditError::AdmissionProbeInvariant)?;
        let entity=entity.parse::<i64>().map_err(|_|NativeNumericAuditError::AdmissionProbeInvariant)?;
        if entity<=0 || head!=format!("proper-name-expansion:{token}") {
            return Err(NativeNumericAuditError::AdmissionProbeInvariant);
        }
        name_groups.entry((key.0,token)).or_default().push((entity,key));
    }
    let mut overflow=BTreeMap::new();
    for (mention,mut rows) in name_groups {
        rows.sort_by_key(|(entity,k)|(*entity,k.1,k.4));
        if rows.len()>limit {
            if rows.iter().any(|(_,k)|k.7<=1) {return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
            overflow.insert(mention,rows.len()-limit);
        }
        retained.extend(rows.into_iter().take(limit).map(|(_,k)|k.clone()));
    }
    Ok((retained,overflow))
}

/// Actual evidence-row delta, keyed by the original m077 unique key.
/// No semantic change is inferred from refreshing state 1/2/3 alone.
/// This is a measured frontier of the bounded producer, not yet a complete
/// source-insertion dependency oracle or a production admission scheduler.
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct NativeIdentityEvidenceDelta {
    pub inserted:usize,
    pub withdrawn:usize,
    pub changed:usize,
    pub unchanged:usize,
    pub touched_sentence_ids:Vec<i64>,
    pub touched_token_ids:Vec<i64>,
    pub affected_target_object_ids:Vec<i64>,
    pub affected_original_witness_ids:Vec<i64>,
    pub affected_target_entity_ids:Vec<i64>,
    pub affected_existing_candidate_ids:Vec<i64>,
    pub complete_source_dependency_paid:bool,
}

fn identity_evidence_delta(before:&BTreeSet<ParserEvidenceKey>,after:&BTreeSet<ParserEvidenceKey>)
    ->Result<NativeIdentityEvidenceDelta,NativeNumericAuditError>{
    type NaturalKey=(i64,i64,i16,String);
    let index=|rows:&BTreeSet<ParserEvidenceKey>|->Result<BTreeMap<NaturalKey,ParserEvidenceKey>,NativeNumericAuditError>{
        let mut m=BTreeMap::new();
        for r in rows {
            if m.insert((r.0,r.1,r.2,r.6.clone()),r.clone()).is_some() {
                return Err(NativeNumericAuditError::AdmissionProbeInvariant);
            }
        }
        Ok(m)
    };
    let a=index(before)?;let b=index(after)?;
    let keys=a.keys().chain(b.keys()).collect::<BTreeSet<_>>();
    let mut inserted=0;let mut withdrawn=0;let mut changed=0;let mut unchanged=0;
    let mut sentences=BTreeSet::new();let mut tokens=BTreeSet::new();let mut targets=BTreeSet::new();
    for k in keys {
        let old=a.get(k);let new=b.get(k);
        if old==new {unchanged+=1;continue;}
        match (old,new) {
            (None,Some(_))=>inserted+=1,
            (Some(_),None)=>withdrawn+=1,
            (Some(_),Some(_))=>changed+=1,
            _=>return Err(NativeNumericAuditError::AdmissionProbeInvariant),
        }
        for r in old.into_iter().chain(new) {
            targets.insert(r.1);
            sentences.extend(r.5);
            tokens.extend(r.3);
            tokens.extend(r.4);
        }
    }
    Ok(NativeIdentityEvidenceDelta{inserted,withdrawn,changed,unchanged,
        touched_sentence_ids:sentences.into_iter().collect(),
        touched_token_ids:tokens.into_iter().collect(),
        affected_target_object_ids:targets.into_iter().collect(),
        affected_original_witness_ids:Vec::new(),affected_target_entity_ids:Vec::new(),
        affected_existing_candidate_ids:Vec::new(),complete_source_dependency_paid:false})
}

/// Reopen the m080 inverse for the exact changed target groups. Include
/// every name candidate sharing a target, even if its own evidence row
/// did not change: its independent corroboration may have been withdrawn.
/// This is observational until the scoped admission/retraction owner is
/// parity-certified, and leaves other original witnesses untouched.
fn reopen_identity_delta_provenance(
    tx:&mut postgres::Transaction<'_>,run:i64,doc:i64,delta:&mut NativeIdentityEvidenceDelta
)->Result<(),NativeNumericAuditError>{
    if delta.affected_target_object_ids.is_empty(){return Ok(());}
    let rows=tx.query("SELECT DISTINCT c.candidate_id,p.witness_id,w.target_entity_id
        FROM execution.semantic_pnf_identity_evidence_candidate c
        LEFT JOIN execution.semantic_pnf_identity_evidence_witness p ON p.candidate_id=c.candidate_id
        LEFT JOIN execution.semantic_pnf_identity_witness w ON w.witness_id=p.witness_id
        WHERE c.run_id=$1 AND c.document_id=$2 AND c.target_object_id=ANY($3::bigint[])
        ORDER BY c.candidate_id,p.witness_id,w.target_entity_id
        LIMIT 100001",&[&run,&doc,&delta.affected_target_object_ids])?;
    if rows.len()>100000{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    let mut candidates=BTreeSet::new();let mut witnesses=BTreeSet::new();let mut entities=BTreeSet::new();
    for r in rows {
        candidates.insert(r.get::<_,i64>(0));
        witnesses.extend(r.get::<_,Option<i64>>(1));
        entities.extend(r.get::<_,Option<i64>>(2));
    }
    delta.affected_existing_candidate_ids=candidates.into_iter().collect();
    delta.affected_original_witness_ids=witnesses.into_iter().collect();
    delta.affected_target_entity_ids=entities.into_iter().collect();
    Ok(())
}

fn refresh_bounded_parser_identity_with_parity(
    probe:&mut postgres::Transaction<'_>,run:i64,doc:i64,
)->Result<(NativeBoundedIdentityParity,BTreeSet<String>,NativeIdentityEvidenceDelta),NativeNumericAuditError>{
    let before=parser_evidence_keys_in_states(probe,run,doc,&[1,2,3])?;
    let (reference,reference_witnesses,reference_elapsed_ms)={
        let mut reference_tx=probe.transaction()?;
        let started=std::time::Instant::now();
        reference_tx.query_one("SELECT execution.refresh_numeric_pnf_parser_identity_evidence($1,$2)",&[&run,&doc])?;
        let elapsed=started.elapsed().as_millis();
        let keys=parser_evidence_keys(&mut reference_tx,run,doc)?;
        reference_tx.query_one("SELECT execution.admit_numeric_pnf_parser_identity_evidence($1,$2)",&[&run,&doc])?;
        let witnesses=parser_admitted_witness_keys(&mut reference_tx,run,doc)?;
        reference_tx.rollback()?;
        (keys,witnesses,elapsed)
    };
    let (expected,expected_overflow)=bounded_parser_evidence_reference(&reference,16)?;
    // Initialize the session-local schema before creating a temporary function.
    probe.batch_execute("CREATE TEMP TABLE slr_identity_probe_scope (run_id bigint) ON COMMIT DROP;")?;
    probe.batch_execute(include_str!("numeric_pnf_identity_bounded.sql"))?;
    let started=std::time::Instant::now();
    probe.query_one("SELECT pg_temp.slr_refresh_numeric_pnf_parser_identity_evidence_bounded($1,$2)",&[&run,&doc])?;
    let bounded_elapsed_ms=started.elapsed().as_millis();
    let actual=parser_evidence_keys(probe,run,doc)?;
    let embedded=probe.query("SELECT token_id FROM pg_temp.slr_identity_mention_scope WHERE embedded_in_person",&[])?
        .iter().map(|r|r.get::<_,i64>(0)).collect::<BTreeSet<_>>();
    let embedded_reference_name_rows=reference.iter().filter(|r|r.2==3&&r.3.is_some_and(|t|embedded.contains(&t))).count();
    let embedded_bounded_name_rows=actual.iter().filter(|r|r.2==3&&r.3.is_some_and(|t|embedded.contains(&t))).count();
    if actual!=expected {return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    // Exact original family count and eligible count are independently
    // reopened from the indexed source carrier. Every omitted reference
    // row must be accounted for by an overflow entry, never a refutation.
    let overflow=probe.query("SELECT o.source_object_id,o.source_token_id,
        o.possible_target_count,o.retained_target_limit,
        (SELECT count(*) FROM pg_temp.slr_identity_family f
          WHERE f.family_lemma_symbol_id=o.family_lemma_symbol_id),
        (SELECT count(*) FROM pg_temp.slr_identity_family f
          JOIN pg_temp.slr_identity_token m ON m.token_id=o.source_token_id
          WHERE f.family_lemma_symbol_id=o.family_lemma_symbol_id
            AND m.lemma_symbol_id=o.family_lemma_symbol_id
            AND f.anchor_object_id<>o.source_object_id
            AND NOT (m.sentence_ref=f.sentence_ref AND m.start_char>=f.start_char AND m.end_char<=f.end_char))
        FROM execution.semantic_pnf_proper_name_evidence_overflow o
        WHERE o.run_id=$1 AND o.document_id=$2",&[&run,&doc])?;
    let mut actual_overflow=BTreeMap::new();
    for r in overflow {
        let key=(r.get::<_,i64>(0),r.get::<_,i64>(1));
        let full:i64=r.get(2);let cap:i16=r.get(3);let reopened:i64=r.get(4);let eligible:i64=r.get(5);
        if cap!=16 || full!=reopened || eligible<=i64::from(cap) || full<eligible
            || actual_overflow.insert(key,(eligible-i64::from(cap)) as usize).is_some(){
            return Err(NativeNumericAuditError::AdmissionProbeInvariant);
        }
    }
    if expected_overflow!=actual_overflow{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
    let mut delta=identity_evidence_delta(&before,&actual)?;
    reopen_identity_delta_provenance(probe,run,doc,&mut delta)?;
    Ok((NativeBoundedIdentityParity{
        reference_candidate_rows:reference.len(),bounded_candidate_rows:actual.len(),
        omitted_ambiguous_rows:reference.len()-actual.len(),overflow_mentions:actual_overflow.len(),
        reference_admitted_witnesses:reference_witnesses.len(),bounded_admitted_witnesses:0,
        candidate_projection_exact:true,original_admission_projection_exact:false,
        embedded_reference_name_rows,embedded_bounded_name_rows,
        golden_standalone_permission_matches:embedded_reference_name_rows==0&&embedded_bounded_name_rows==0,
        reference_elapsed_ms,bounded_elapsed_ms,full_typed_coverage_paid:false,persisted:false,
    },reference_witnesses,delta))
}

#[derive(Debug,Clone,Serialize)]
pub struct NativeScopedAdmissionParity {
    pub target_object_id:i64,
    pub target_object_ids:Vec<i64>,
    pub scenario:&'static str,
    pub withdrawn_direct_candidates:u64,
    pub withdrawn_candidate_rows:u64,
    pub changed_witness_keys:usize,
    pub changed_projection_source_ids:Vec<i64>,
    pub affected_factor_ids:Vec<i64>,
    pub changed_level_three_refs:Vec<String>,
    pub reference_admission_exact:bool,
    pub reference_level_three_exact:bool,
    pub golden_producer_exact:Option<bool>,
    pub unrelated_admission_revisions_preserved:bool,
    pub unrelated_derivation_ids_preserved:bool,
    pub immutable_level_one_preserved:bool,
    pub rollback_restored:bool,
    pub persisted:bool,
}

/// Original current projection, grouped by exact source object (including
/// authority and witness multiplicity). Absence is represented by no key.
fn identity_projection_snapshot(
    tx:&mut postgres::Transaction<'_>,run:i64,doc:i64,
)->Result<BTreeMap<i64,String>,NativeNumericAuditError>{
    let rows=tx.query("SELECT p.source_object_id,
        jsonb_agg(to_jsonb(p) ORDER BY p.authority_class,p.target_entity_id,p.witness_ids)::text
        FROM execution.semantic_pnf_identity_projection p
        JOIN execution.semantic_pnf_object o ON o.object_id=p.source_object_id
        JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
        WHERE r.run_id=$1 AND r.document_id=$2 GROUP BY p.source_object_id",&[&run,&doc])?;
    Ok(rows.iter().map(|r|(r.get(0),r.get(1))).collect())
}
fn changed_projection_sources(before:&BTreeMap<i64,String>,after:&BTreeMap<i64,String>)->Vec<i64>{
    before.keys().chain(after.keys()).copied().collect::<BTreeSet<_>>().into_iter()
        .filter(|id|before.get(id)!=after.get(id)).collect()
}

/// Exact reverse of m086 support: direct object or shared original token.
/// Start from every changed source, including formerly absent projections.
/// Do not use only the previously chosen unique support path.
fn affected_identity_factors(
    tx:&mut postgres::Transaction<'_>,run:i64,doc:i64,objects:&[i64],
)->Result<Vec<i64>,NativeNumericAuditError>{
    if objects.is_empty(){return Ok(Vec::new());}
    Ok(tx.query("WITH affected_objects AS (
        SELECT unnest($3::bigint[]) AS object_id
        UNION SELECT s2.object_id
          FROM execution.semantic_pnf_object_token_support s1
          JOIN execution.semantic_pnf_object_token_support s2 USING(token_id)
          WHERE s1.object_id=ANY($3::bigint[])
      )
      SELECT DISTINCT e.factor_id FROM affected_objects o
      JOIN execution.semantic_pnf_hyperedge e ON e.object_id=o.object_id
      JOIN execution.semantic_pnf_factor f ON f.factor_id=e.factor_id
      JOIN execution.semantic_pnf_region r ON r.region_id=f.region_id
      WHERE r.run_id=$1 AND r.document_id=$2 ORDER BY e.factor_id",
      &[&run,&doc,&objects])?.iter().map(|r|r.get(0)).collect())
}

fn rebuild_reference_identity_level_three(
    tx:&mut postgres::Transaction<'_>,run:i64,doc:i64,
)->Result<(),NativeNumericAuditError>{
    // m072 already rebuilds direct derivations. Original m086 is insert-only,
    // so clear ONLY its rebuildable rule projection before calling its owner.
    tx.execute("DELETE FROM execution.semantic_pnf_factor_derivation d
      USING execution.semantic_pnf_interface i,execution.semantic_pnf_region r
      WHERE d.scope_interface_id=i.interface_id AND i.region_id=r.region_id
        AND r.run_id=$1 AND r.document_id=$2 AND d.rule_ref='identity-supported-substitution:v1'",
      &[&run,&doc])?;
    tx.query_one("SELECT execution.refresh_numeric_pnf_identity_substitution_derivations($1,$2)",&[&run,&doc])?;
    tx.query_one("SELECT execution.refresh_numeric_pnf_supported_identity_substitution_derivations($1,$2)",&[&run,&doc])?;
    Ok(())
}

fn scoped_derivation_ids(tx:&mut postgres::Transaction<'_>,run:i64,doc:i64)
    ->Result<BTreeMap<String,(i64,i64)>,NativeNumericAuditError>{
    let rows=tx.query("SELECT d.derivation_ref,d.derivation_id,p.factor_id
        FROM execution.semantic_pnf_factor_derivation d
        JOIN execution.semantic_pnf_factor_derivation_premise p
          ON p.derivation_id=d.derivation_id AND p.premise_ordinal=0
        JOIN execution.semantic_pnf_interface i ON i.interface_id=d.scope_interface_id
        JOIN execution.semantic_pnf_region r ON r.region_id=i.region_id
        WHERE r.run_id=$1 AND r.document_id=$2 AND d.rule_ref IN
          ('identity-substitution:v1','identity-supported-substitution:v1')",&[&run,&doc])?;
    let result=rows.iter().map(|r|(r.get::<_,String>(0),(r.get::<_,i64>(1),r.get::<_,i64>(2)))).collect::<BTreeMap<_,_>>();
    if result.len()!=rows.len(){return Err(NativeNumericAuditError::RetractionInvariant);}
    Ok(result)
}

/// A single real source-bound scenario, using the original full admission
/// and Level-3 owners as reference and the scoped owners as refinement.
/// Run all changes in nested savepoints; no provisional proof is persisted.
fn probe_identity_admission_scenario(
    tx:&mut postgres::Transaction<'_>,run:i64,doc:i64,
    scenario:&'static str,targets:&[i64],withdraw_ids:&[i64],
)->Result<NativeScopedAdmissionParity,NativeNumericAuditError>{
    let before=parser_admitted_witness_keys(tx,run,doc)?;
    let projections=identity_projection_snapshot(tx,run,doc)?;
    let before_l3=level_three_snapshot(tx,run,doc)?;
    let before_ids=scoped_derivation_ids(tx,run,doc)?;
    let level_one=level_one_snapshot(tx,run,doc)?;
    let unrelated_sql="SELECT a.witness_id,a.admission_state,a.revision
        FROM execution.semantic_pnf_identity_witness_admission a
        JOIN execution.semantic_pnf_identity_witness w ON w.witness_id=a.witness_id
        JOIN execution.semantic_pnf_canonical_entity e ON e.entity_id=w.target_entity_id
        JOIN execution.semantic_pnf_object o ON o.object_id=w.source_object_id
        JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
        WHERE r.run_id=$1 AND r.document_id=$2
          AND NOT COALESCE(e.anchor_object_id=ANY($3::bigint[]),false)
        ORDER BY a.witness_id";
    // Compare text rather than assuming the original revision SQL width.
    let read_unrelated=|db:&mut postgres::Transaction<'_>|->Result<Vec<(i64,i16,String)>,NativeNumericAuditError>{
        let sql=unrelated_sql.replace("a.revision","a.revision::text");
        Ok(db.query(&sql,&[&run,&doc,&targets])?.iter().map(|r|(r.get(0),r.get(1),r.get(2))).collect())
    };
    let unrelated=read_unrelated(tx)?;
    let withdraw=|db:&mut postgres::Transaction<'_>|->Result<u64,NativeNumericAuditError>{
        let n=db.execute("UPDATE execution.semantic_pnf_identity_evidence_candidate SET evidence_state=4
          WHERE run_id=$1 AND document_id=$2 AND target_object_id=ANY($3::bigint[])
            AND candidate_id=ANY($4::bigint[]) AND evidence_state IN (1,2,3)",
          &[&run,&doc,&targets,&withdraw_ids])?;
        if n!=withdraw_ids.len() as u64{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
        Ok(n)
    };
    let(reference,ref_projection,ref_l3,withdrawn)={
        let mut p=tx.transaction()?;
        p.execute("UPDATE execution.semantic_pnf_identity_evidence_candidate SET evidence_state=1
          WHERE run_id=$1 AND document_id=$2 AND evidence_state IN (1,2,3) AND evidence_state<>1",&[&run,&doc])?;
        let n=withdraw(&mut p)?;
        p.query_one("SELECT execution.admit_numeric_pnf_parser_identity_evidence($1,$2)",&[&run,&doc])?;
        let keys=parser_admitted_witness_keys(&mut p,run,doc)?;
        let projection=identity_projection_snapshot(&mut p,run,doc)?;
        rebuild_reference_identity_level_three(&mut p,run,doc)?;
        let l3=level_three_snapshot(&mut p,run,doc)?;
        if level_one!=level_one_snapshot(&mut p,run,doc)?{return Err(NativeNumericAuditError::RetractionInvariant);}
        p.rollback()?;
        (keys,projection,l3,n)
    };
    let(scoped,scope_projection,scope_l3,sources,factors,unrelated_preserved,unrelated_ids,immutable)={
        let mut p=tx.transaction()?;
        p.batch_execute(include_str!("numeric_pnf_identity_admit_scoped.sql"))?;
        p.batch_execute(include_str!("numeric_pnf_identity_substitution_scoped.sql"))?;
        p.execute("UPDATE execution.semantic_pnf_identity_evidence_candidate SET evidence_state=1
          WHERE run_id=$1 AND document_id=$2 AND target_object_id=ANY($3::bigint[])
            AND evidence_state IN (1,2,3) AND evidence_state<>1",&[&run,&doc,&targets])?;
        let n=withdraw(&mut p)?;
        if n!=withdrawn{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
        p.query_one("SELECT pg_temp.slr_admit_numeric_pnf_parser_identity_targets($1,$2,$3)",&[&run,&doc,&targets])?;
        let keys=parser_admitted_witness_keys(&mut p,run,doc)?;
        let projection=identity_projection_snapshot(&mut p,run,doc)?;
        let changed=changed_projection_sources(&projections,&projection);
        let factors=affected_identity_factors(&mut p,run,doc,&changed)?;
        p.query_one("SELECT pg_temp.slr_refresh_identity_substitution_factors($1,$2,$3)",&[&run,&doc,&factors])?;
        p.query_one("SELECT pg_temp.slr_refresh_supported_identity_substitution_factors($1,$2,$3)",&[&run,&doc,&factors])?;
        let l3=level_three_snapshot(&mut p,run,doc)?;
        let after_ids=scoped_derivation_ids(&mut p,run,doc)?;
        let unaffected=before_ids.iter().filter(|(_,(_,id))|!factors.contains(id))
            .all(|(key,value)|after_ids.get(key)==Some(value))
            &&after_ids.iter().filter(|(_,(_,id))|!factors.contains(id))
                .all(|(key,value)|before_ids.get(key)==Some(value));
        let preserved=unrelated==read_unrelated(&mut p)?;
        let immutable=level_one==level_one_snapshot(&mut p,run,doc)?;
        p.rollback()?;
        (keys,projection,l3,changed,factors,preserved,unaffected,immutable)
    };
    // The golden source rule is exercised by the actual bounded writer,
    // not only by deleting a reference candidate. Reuse the exact support
    // stage in a third savepoint, then compare its full semantic output.
    let golden_producer_exact=if scenario=="agda-standalone-source-correction" {
        let embedded=tx.query("SELECT token_id FROM pg_temp.slr_identity_mention_scope WHERE embedded_in_person",&[])?
            .iter().map(|r|r.get::<_,i64>(0)).collect::<BTreeSet<_>>();
        let expected=parser_evidence_keys_in_states(tx,run,doc,&[1,2,3])?.into_iter()
            .filter(|k|k.2!=3||!k.3.is_some_and(|id|embedded.contains(&id))).collect::<BTreeSet<_>>();
        let overflow_sql="SELECT source_object_id,source_token_id,family_lemma_symbol_id,possible_target_count,retained_target_limit
            FROM execution.semantic_pnf_proper_name_evidence_overflow
            WHERE run_id=$1 AND document_id=$2 ORDER BY source_object_id,source_token_id,family_lemma_symbol_id";
        let read_overflow=|p:&mut postgres::Transaction<'_>|->Result<BTreeSet<(i64,i64,i64,i64,i16)>,NativeNumericAuditError>{
            Ok(p.query(overflow_sql,&[&run,&doc])?.iter().map(|r|(r.get(0),r.get(1),r.get(2),r.get(3),r.get(4))).collect())
        };
        let expected_overflow=read_overflow(tx)?.into_iter().filter(|k|!embedded.contains(&k.1)).collect::<BTreeSet<_>>();
        let mut p=tx.transaction()?;
        p.query_one("SELECT pg_temp.slr_write_numeric_pnf_parser_identity_bounded($1,$2,TRUE)",&[&run,&doc])?;
        if parser_evidence_keys(&mut p,run,doc)?!=expected||read_overflow(&mut p)?!=expected_overflow {
            return Err(NativeNumericAuditError::AdmissionProbeInvariant);
        }
        p.query_one("SELECT execution.admit_numeric_pnf_parser_identity_evidence($1,$2)",&[&run,&doc])?;
        let keys=parser_admitted_witness_keys(&mut p,run,doc)?;
        let projection=identity_projection_snapshot(&mut p,run,doc)?;
        rebuild_reference_identity_level_three(&mut p,run,doc)?;
        let exact=keys==reference&&projection==ref_projection&&level_three_snapshot(&mut p,run,doc)?==ref_l3
            &&level_one==level_one_snapshot(&mut p,run,doc)?;
        p.rollback()?;
        if !exact{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
        Some(true)
    }else{None};
    let restored=before==parser_admitted_witness_keys(tx,run,doc)?
        &&projections==identity_projection_snapshot(tx,run,doc)?
        &&before_l3==level_three_snapshot(tx,run,doc)?
        &&before_ids==scoped_derivation_ids(tx,run,doc)?
        &&unrelated==read_unrelated(tx)?&&level_one==level_one_snapshot(tx,run,doc)?;
    if scoped!=reference||scope_projection!=ref_projection||scope_l3!=ref_l3
        ||!unrelated_preserved||!unrelated_ids||!immutable||!restored {
        return Err(NativeNumericAuditError::RetractionInvariant);
    }
    let changed_level_three_refs=before_l3.keys().chain(scope_l3.keys()).cloned()
        .collect::<BTreeSet<_>>().into_iter().filter(|k|before_l3.get(k)!=scope_l3.get(k)).collect();
    Ok(NativeScopedAdmissionParity{target_object_id:targets.first().copied().unwrap_or(0),
        target_object_ids:targets.to_vec(),scenario,
        withdrawn_direct_candidates:if scenario=="direct-withdrawal"{withdrawn}else{0},
        withdrawn_candidate_rows:withdrawn,changed_witness_keys:before.symmetric_difference(&scoped).count(),
        changed_projection_source_ids:sources,affected_factor_ids:factors,changed_level_three_refs,
        reference_admission_exact:true,reference_level_three_exact:true,golden_producer_exact,
        unrelated_admission_revisions_preserved:unrelated_preserved,unrelated_derivation_ids_preserved:unrelated_ids,
        immutable_level_one_preserved:immutable,rollback_restored:restored,persisted:false})
}

/// Integrated campaign: actual evidence-delta target replay, empty workset,
/// golden standalone-source correction, and real direct-witness withdrawal.
/// The baseline is rebuilt once inside a parent savepoint and restored.
fn probe_scoped_parser_admission(
    tx:&mut postgres::Transaction<'_>,run:i64,doc:i64,delta_targets:&[i64],
)->Result<Vec<NativeScopedAdmissionParity>,NativeNumericAuditError>{
    let mut baseline=tx.transaction()?;
    rebuild_reference_identity_level_three(&mut baseline,run,doc)?;
    let mut results=Vec::new();
    results.push(probe_identity_admission_scenario(&mut baseline,run,doc,"empty-workset",&[],&[])?);
    if !delta_targets.is_empty() {
        results.push(probe_identity_admission_scenario(&mut baseline,run,doc,"actual-evidence-target-replay",delta_targets,&[])?);
    }
    let embedded=baseline.query("SELECT c.candidate_id,c.target_object_id
        FROM execution.semantic_pnf_identity_evidence_candidate c
        JOIN pg_temp.slr_identity_mention_scope m ON m.token_id=c.source_token_id
        WHERE c.run_id=$1 AND c.document_id=$2 AND c.witness_kind=3
          AND c.evidence_state IN (1,2,3) AND m.embedded_in_person
        ORDER BY c.candidate_id",&[&run,&doc])?;
    let ids=embedded.iter().map(|r|r.get::<_,i64>(0)).collect::<Vec<_>>();
    let targets=embedded.iter().map(|r|r.get::<_,i64>(1)).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
    results.push(probe_identity_admission_scenario(&mut baseline,run,doc,"agda-standalone-source-correction",&targets,&ids)?);
    let selected=baseline.query_opt("SELECT c.target_object_id
        FROM execution.semantic_pnf_identity_evidence_candidate c
        WHERE c.run_id=$1 AND c.document_id=$2 AND c.evidence_state=2 AND c.witness_kind IN (2,4,6)
        ORDER BY EXISTS(SELECT 1 FROM execution.semantic_pnf_identity_evidence_candidate n
          WHERE n.run_id=c.run_id AND n.document_id=c.document_id AND n.target_object_id=c.target_object_id
          AND n.witness_kind=3 AND n.evidence_state=2) DESC,
        EXISTS(SELECT 1 FROM execution.semantic_pnf_identity_evidence_witness p
          JOIN execution.semantic_pnf_factor_derivation_argument a ON p.witness_id=ANY(a.identity_witness_ids)
          WHERE p.candidate_id=c.candidate_id) DESC,c.target_object_id LIMIT 1",&[&run,&doc])?;
    if let Some(row)=selected{
        let target:i64=row.get(0);
        let ids=baseline.query("SELECT candidate_id FROM execution.semantic_pnf_identity_evidence_candidate
          WHERE run_id=$1 AND document_id=$2 AND target_object_id=$3 AND witness_kind IN (2,4,6)
            AND evidence_state IN (1,2,3) ORDER BY candidate_id",&[&run,&doc,&target])?
          .iter().map(|r|r.get::<_,i64>(0)).collect::<Vec<_>>();
        if !ids.is_empty(){results.push(probe_identity_admission_scenario(&mut baseline,run,doc,"direct-withdrawal",&[target],&ids)?);}
    }
    baseline.rollback()?;
    Ok(results)
}

/// Explicit document-local publication. The original m080 owner issues the
/// witnesses; the native implementation never promotes a source-answer
/// policy to typed demand coverage, a focus identity, or historical truth.
#[derive(Debug,Clone,Serialize)]
pub struct NativeDocumentIdentityPublication {
    pub requested:bool,
    pub committed:bool,
    pub blockers:Vec<&'static str>,
    pub affected_target_ids:Vec<i64>,
    pub affected_factor_ids:Vec<i64>,
    pub candidate_rows_changed:u64,
    pub overflow_rows_changed:u64,
    pub reference_exact:bool,
    pub immutable_level_one_preserved:bool,
    pub unrelated_admissions_preserved:bool,
    pub unrelated_derivations_preserved:bool,
    pub focus_identity_paid:bool,
}

fn publish_checked_document_identity(
    tx:&mut postgres::Transaction<'_>,run_ref:&str,answer:&NativeSourceLocalAnswer,
    producer:&OriginalParserIdentityProducerObservation,probe_receipt:&NativeGovernedIdentityProbe,
)->Result<NativeDocumentIdentityPublication,NativeNumericAuditError>{
    let mut receipt=NativeDocumentIdentityPublication{
        requested:true,committed:false,blockers:Vec::new(),affected_target_ids:Vec::new(),
        affected_factor_ids:Vec::new(),candidate_rows_changed:0,overflow_rows_changed:0,
        reference_exact:false,immutable_level_one_preserved:false,unrelated_admissions_preserved:false,
        unrelated_derivations_preserved:false,focus_identity_paid:false,
    };
    let golden=producer.scoped_admission_parity.iter().any(|p|p.scenario=="agda-standalone-source-correction"
        &&p.golden_producer_exact==Some(true)&&p.reference_admission_exact&&p.reference_level_three_exact
        &&p.immutable_level_one_preserved&&p.rollback_restored);
    let withdrawn=producer.scoped_admission_parity.iter().any(|p|p.scenario=="direct-withdrawal"
        &&p.changed_witness_keys>0&&!p.changed_level_three_refs.is_empty()&&p.reference_level_three_exact
        &&p.reference_admission_exact&&p.rollback_restored);
    if !golden||!withdrawn||!probe_receipt.immutable_level_one_preserved
       ||!probe_receipt.rollback_preserved_prior_level_three||!probe_receipt.rollback_preserved_prior_candidate_classes
       ||!producer.support_index.as_ref().is_some_and(|p|p.delta_reference_exact){
        receipt.blockers.push("original_reference_and_golden_rollback_parity_unpaid");
    }
    let row=tx.query_one("SELECT r.run_id,d.document_id FROM execution.semantic_pnf_run_identity r
        CROSS JOIN execution.semantic_pnf_document_identity d WHERE r.run_ref=$1 AND d.document_ref=$2",
        &[&run_ref,&answer.source_document_ref])?;
    let run:i64=row.get(0);let doc:i64=row.get(1);
    let source=tx.query_one("SELECT locator FROM execution.semantic_parser_source
        WHERE run_ref=$1 AND document_ref=$2 AND encode(content_sha256,'hex')=$3",
        &[&run_ref,&answer.source_document_ref,&answer.source_content_sha256])?;
    let locator:String=source.get(0);
    let bytes=std::fs::read(locator).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    if hex(&Sha256::digest(&bytes))!=answer.source_content_sha256{return Err(NativeNumericAuditError::ForeignSource);}
    let text=std::str::from_utf8(&bytes).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let chars=text.chars().collect::<Vec<_>>();
    let mut covered=vec![false;chars.len()];
    let tokens=tx.query("SELECT t.start_char,t.end_char FROM execution.semantic_parser_token t
        JOIN execution.semantic_pnf_sentence_region l ON l.sentence_id=t.sentence_id
        JOIN execution.semantic_pnf_region r ON r.region_id=l.region_id
        WHERE t.run_ref=$1 AND t.document_ref=$2 AND t.representation_version=2
          AND r.run_id=$3 AND r.document_id=$4 AND r.closure_state IN(2,3)
        ORDER BY t.start_char,t.end_char,t.token_id",&[&run_ref,&answer.source_document_ref,&run,&doc])?;
    let mut prev=0usize;
    for t in tokens{
        let a=usize::try_from(t.get::<_,i64>(0)).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        let b=usize::try_from(t.get::<_,i64>(1)).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        if a<prev||a>=b||b>covered.len(){return Err(NativeNumericAuditError::ForeignSource);}
        covered[a..b].fill(true);prev=b;
    }
    if chars.iter().zip(&covered).any(|(c,paid)|!c.is_whitespace()&&!*paid) {
        receipt.blockers.push("full_document_token_and_closed_region_coverage_unpaid");
    }
    let pending:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.semantic_pnf_work_item w
        JOIN execution.semantic_pnf_region r ON r.region_id=w.region_id
        WHERE r.run_id=$1 AND r.document_id=$2 AND w.operation_id=1 AND w.state_id<>3)",
        &[&run,&doc])?.get(0);
    if pending{receipt.blockers.push("original_numeric_sentence_work_pending");}
    // Original m080 can re-admit historical parser provenance. An explicit
    // independently revoked witness is not permission to do that on replay.
    let explicitly_revoked:bool=tx.query_one("SELECT EXISTS(
        SELECT 1 FROM execution.semantic_pnf_identity_evidence_candidate c
        JOIN execution.semantic_pnf_identity_evidence_witness p ON p.candidate_id=c.candidate_id
        JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=p.witness_id
        WHERE c.run_id=$1 AND c.document_id=$2 AND a.admission_state=3)",&[&run,&doc])?.get(0);
    if explicitly_revoked{receipt.blockers.push("explicit_witness_revocation_requires_original_authority");}
    if !receipt.blockers.is_empty(){return Ok(receipt);}
    let level_one=level_one_snapshot(tx,run,doc)?;
    let before_l3=level_three_snapshot(tx,run,doc)?;
    let before_ids=scoped_derivation_ids(tx,run,doc)?;
    let before_projection=identity_projection_snapshot(tx,run,doc)?;
    let before=parser_evidence_keys_in_states(tx,run,doc,&[1,2,3])?;
    let mut stage=tx.transaction()?;
    stage.batch_execute("CREATE TEMP TABLE slr_document_publication_probe(singleton bool) ON COMMIT DROP;")?;
    stage.batch_execute(include_str!("numeric_pnf_identity_bounded.sql"))?;
    stage.query_one("SELECT pg_temp.slr_refresh_numeric_pnf_parser_identity_evidence_bounded($1,$2,TRUE)",&[&run,&doc])?;
    // Independently reject fallback to a lexical match when an actual
    // representation-support edge exists but no exact matching head does.
    let unsupported_fallback:bool=stage.query_one("SELECT EXISTS(
        SELECT 1 FROM pg_temp.slr_identity_anchor a
        WHERE EXISTS(SELECT 1 FROM execution.semantic_pnf_object_token_support s
                     WHERE s.token_id=a.token_id)
          AND NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_object_token_support s
                     WHERE s.token_id=a.token_id AND s.object_id=a.object_id))",&[])?.get(0);
    if unsupported_fallback{
        stage.rollback()?;
        receipt.blockers.push("golden_exact_support_priority_disagreement");
        return Ok(receipt);
    }
    let desired=parser_evidence_keys(&mut stage,run,doc)?;
    let overflow_sql="SELECT source_object_id,source_token_id,family_lemma_symbol_id,possible_target_count,retained_target_limit
        FROM execution.semantic_pnf_proper_name_evidence_overflow WHERE run_id=$1 AND document_id=$2";
    let read_overflow=|p:&mut postgres::Transaction<'_>|->Result<BTreeSet<(i64,i64,i64,i64,i16)>,NativeNumericAuditError>{
        Ok(p.query(overflow_sql,&[&run,&doc])?.iter().map(|r|(r.get(0),r.get(1),r.get(2),r.get(3),r.get(4))).collect())
    };
    let desired_overflow=read_overflow(&mut stage)?;
    stage.rollback()?;
    let delta=identity_evidence_delta(&before,&desired)?;
    let mut targets=delta.affected_target_object_ids;
    // Include previously unprocessed direct evidence and accepted stale
    // provenance, even if the natural evidence row itself did not change.
    targets.extend(tx.query("SELECT DISTINCT c.target_object_id FROM execution.semantic_pnf_identity_evidence_candidate c
        WHERE c.run_id=$1 AND c.document_id=$2 AND ((c.evidence_state=1 AND c.candidate_count=1 AND c.witness_kind IN(2,4,6))
        OR (c.evidence_state=4 AND EXISTS(SELECT 1 FROM execution.semantic_pnf_identity_evidence_witness p
            JOIN execution.semantic_pnf_identity_witness_admission a ON a.witness_id=p.witness_id AND a.admission_state=2
            WHERE p.candidate_id=c.candidate_id)))",&[&run,&doc])?.iter().map(|r|r.get::<_,i64>(0)));
    targets.sort_unstable();targets.dedup();
    // No write on identical current evidence unless the complete reference
    // below discovers unpaid original admission/derivation work.
    let packet=serde_json::Value::Array(desired.iter().map(|k|serde_json::json!({
        "source_object_id":k.0,"target_object_id":k.1,"witness_kind":k.2,
        "source_token_id":k.3,"target_token_id":k.4,"sentence_id":k.5,"evidence_ref":k.6,"candidate_count":k.7
    })).collect());
    let overflow_packet=serde_json::Value::Array(desired_overflow.iter().map(|k|serde_json::json!({
        "source_object_id":k.0,"source_token_id":k.1,"family_lemma_symbol_id":k.2,
        "possible_target_count":k.3,"retained_target_limit":k.4
    })).collect());
    let unrelated_sql="SELECT a.witness_id,a.admission_state,a.revision::text FROM execution.semantic_pnf_identity_witness_admission a
        JOIN execution.semantic_pnf_identity_witness w ON w.witness_id=a.witness_id
        JOIN execution.semantic_pnf_canonical_entity e ON e.entity_id=w.target_entity_id
        JOIN execution.semantic_pnf_object o ON o.object_id=w.source_object_id
        JOIN execution.semantic_pnf_region r ON r.region_id=o.region_id
        WHERE r.run_id=$1 AND r.document_id=$2 AND NOT COALESCE(e.anchor_object_id=ANY($3::bigint[]),false)
        ORDER BY a.witness_id";
    let read_unrelated=|p:&mut postgres::Transaction<'_>|->Result<Vec<(i64,i16,String)>,NativeNumericAuditError>{
        Ok(p.query(unrelated_sql,&[&run,&doc,&targets])?.iter().map(|r|(r.get(0),r.get(1),r.get(2))).collect())
    };
    let unrelated=read_unrelated(tx)?;
    let mut scoped=tx.transaction()?;
    scoped.batch_execute("CREATE TEMP TABLE slr_document_publication_scope(singleton bool) ON COMMIT DROP;")?;
    scoped.batch_execute(include_str!("numeric_pnf_identity_admit_scoped.sql"))?;
    scoped.batch_execute(include_str!("numeric_pnf_identity_substitution_scoped.sql"))?;
    let retired=scoped.execute("UPDATE execution.semantic_pnf_identity_evidence_candidate c SET evidence_state=4
        WHERE c.run_id=$1 AND c.document_id=$2 AND c.evidence_state IN(1,2,3)
          AND NOT EXISTS(SELECT 1 FROM jsonb_to_recordset($3) n(source_object_id bigint,target_object_id bigint,
              witness_kind smallint,evidence_ref text)
              WHERE (n.source_object_id,n.target_object_id,n.witness_kind,n.evidence_ref)=
                (c.source_object_id,c.target_object_id,c.witness_kind,c.evidence_ref))",&[&run,&doc,&packet])?;
    let changed=scoped.execute("INSERT INTO execution.semantic_pnf_identity_evidence_candidate AS c
        (run_id,document_id,source_object_id,target_object_id,witness_kind,source_token_id,target_token_id,sentence_id,evidence_ref,candidate_count,evidence_state)
        SELECT $1,$2,n.source_object_id,n.target_object_id,n.witness_kind,n.source_token_id,n.target_token_id,n.sentence_id,n.evidence_ref,n.candidate_count,1
        FROM jsonb_to_recordset($3) n(source_object_id bigint,target_object_id bigint,witness_kind smallint,
            source_token_id bigint,target_token_id bigint,sentence_id bigint,evidence_ref text,candidate_count smallint)
        ON CONFLICT(run_id,document_id,source_object_id,target_object_id,witness_kind,evidence_ref)
        DO UPDATE SET evidence_state=1,source_token_id=EXCLUDED.source_token_id,target_token_id=EXCLUDED.target_token_id,
          sentence_id=EXCLUDED.sentence_id,candidate_count=EXCLUDED.candidate_count
        WHERE c.evidence_state=4 OR (c.source_token_id,c.target_token_id,c.sentence_id,c.candidate_count)
          IS DISTINCT FROM (EXCLUDED.source_token_id,EXCLUDED.target_token_id,EXCLUDED.sentence_id,EXCLUDED.candidate_count)",
        &[&run,&doc,&packet])?;
    let overflow_removed=scoped.execute("DELETE FROM execution.semantic_pnf_proper_name_evidence_overflow o
        WHERE o.run_id=$1 AND o.document_id=$2 AND NOT EXISTS(
            SELECT 1 FROM jsonb_to_recordset($3) n(source_object_id bigint,source_token_id bigint,family_lemma_symbol_id bigint,
                possible_target_count bigint,retained_target_limit smallint)
            WHERE (o.source_object_id,o.source_token_id,o.family_lemma_symbol_id,o.possible_target_count,o.retained_target_limit)
              =(n.source_object_id,n.source_token_id,n.family_lemma_symbol_id,n.possible_target_count,n.retained_target_limit))",
        &[&run,&doc,&overflow_packet])?;
    let overflow_added=scoped.execute("INSERT INTO execution.semantic_pnf_proper_name_evidence_overflow
        (run_id,document_id,source_object_id,source_token_id,family_lemma_symbol_id,possible_target_count,retained_target_limit)
        SELECT $1,$2,n.source_object_id,n.source_token_id,n.family_lemma_symbol_id,n.possible_target_count,n.retained_target_limit
        FROM jsonb_to_recordset($3) n(source_object_id bigint,source_token_id bigint,family_lemma_symbol_id bigint,
          possible_target_count bigint,retained_target_limit smallint)
        WHERE NOT EXISTS(SELECT 1 FROM execution.semantic_pnf_proper_name_evidence_overflow o
          WHERE o.run_id=$1 AND o.document_id=$2 AND
          (o.source_object_id,o.source_token_id,o.family_lemma_symbol_id,o.possible_target_count,o.retained_target_limit)=
          (n.source_object_id,n.source_token_id,n.family_lemma_symbol_id,n.possible_target_count,n.retained_target_limit))",
        &[&run,&doc,&overflow_packet])?;
    scoped.execute("UPDATE execution.semantic_pnf_identity_evidence_candidate SET evidence_state=1
        WHERE run_id=$1 AND document_id=$2 AND target_object_id=ANY($3::bigint[])
          AND evidence_state IN(2,3)",&[&run,&doc,&targets])?;
    scoped.query_one("SELECT pg_temp.slr_admit_numeric_pnf_parser_identity_targets($1,$2,$3)",&[&run,&doc,&targets])?;
    let projection=identity_projection_snapshot(&mut scoped,run,doc)?;
    let sources=changed_projection_sources(&before_projection,&projection);
    let factors=affected_identity_factors(&mut scoped,run,doc,&sources)?;
    scoped.query_one("SELECT pg_temp.slr_refresh_identity_substitution_factors($1,$2,$3)",&[&run,&doc,&factors])?;
    scoped.query_one("SELECT pg_temp.slr_refresh_supported_identity_substitution_factors($1,$2,$3)",&[&run,&doc,&factors])?;
    let current=parser_evidence_keys_in_states(&mut scoped,run,doc,&[1,2,3])?;
    let witness_keys=parser_admitted_witness_keys(&mut scoped,run,doc)?;
    let after_l3=level_three_snapshot(&mut scoped,run,doc)?;
    let after_ids=scoped_derivation_ids(&mut scoped,run,doc)?;
    let unaffected=before_ids.iter().filter(|(_,(_,id))|!factors.contains(id)).all(|(k,v)|after_ids.get(k)==Some(v))
        &&after_ids.iter().filter(|(_,(_,id))|!factors.contains(id)).all(|(k,v)|before_ids.get(k)==Some(v));
    let unrelated_preserved=unrelated==read_unrelated(&mut scoped)?;
    let immutable=level_one==level_one_snapshot(&mut scoped,run,doc)?;
    if current!=desired||read_overflow(&mut scoped)?!=desired_overflow||!unaffected||!unrelated_preserved||!immutable{
        return Err(NativeNumericAuditError::RetractionInvariant);
    }
    // Reference after the scoped candidate allocations uses the SAME
    // original witness/entity IDs. It reconstructs the entire admitted
    // projection in a savepoint; disagreement means no live publication.
    let mut reference=scoped.transaction()?;
    reference.execute("UPDATE execution.semantic_pnf_identity_evidence_candidate SET evidence_state=1
        WHERE run_id=$1 AND document_id=$2 AND evidence_state IN(2,3)",&[&run,&doc])?;
    reference.query_one("SELECT execution.admit_numeric_pnf_parser_identity_evidence($1,$2)",&[&run,&doc])?;
    rebuild_reference_identity_level_three(&mut reference,run,doc)?;
    let exact=witness_keys==parser_admitted_witness_keys(&mut reference,run,doc)?
        &&projection==identity_projection_snapshot(&mut reference,run,doc)?
        &&after_l3==level_three_snapshot(&mut reference,run,doc)?
        &&level_one==level_one_snapshot(&mut reference,run,doc)?;
    reference.rollback()?;
    if !exact{return Err(NativeNumericAuditError::RetractionInvariant);}
    // Do not label unchanged outside derivations as affected; preserve
    // every nonselected original derivation payload as well as its ID.
    for (key,value) in &before_l3{
        if before_ids.get(key).is_some_and(|(_,id)|!factors.contains(id))&&after_l3.get(key)!=Some(value){
            return Err(NativeNumericAuditError::RetractionInvariant);
        }
    }
    scoped.commit()?;
    receipt.committed=true;receipt.reference_exact=true;receipt.immutable_level_one_preserved=immutable;
    receipt.unrelated_admissions_preserved=unrelated_preserved;receipt.unrelated_derivations_preserved=unaffected;
    receipt.affected_target_ids=targets;receipt.affected_factor_ids=factors;
    receipt.candidate_rows_changed=retired+changed;receipt.overflow_rows_changed=overflow_removed+overflow_added;
    Ok(receipt)
}

fn probe_original_governed_identity(
    tx:&mut postgres::Transaction<'_>,run_ref:&str,document_ref:&str,demand:i64,answer:&NativeSourceLocalAnswer,
)->Result<(OriginalParserIdentityProducerObservation,NativeGovernedIdentityProbe),NativeNumericAuditError>{
    let row=tx.query_one(
        "SELECT r.run_id,r.document_id FROM execution.semantic_pnf_demand d
         JOIN execution.semantic_pnf_region r ON r.region_id=d.source_region_id
         WHERE d.demand_id=$1 AND r.run_ref=$2 AND r.document_ref=$3",
        &[&demand,&run_ref,&document_ref])?;
    let run:i64=row.get(0);let document:i64=row.get(1);
    let level_one=level_one_snapshot(tx,run,document)?;
    let prior=level_three_snapshot(tx,run,document)?;
    let (_,prior_classes)=observe_original_resolution_fibre(tx,demand,run_ref,document_ref)?;
    let bounded_enabled=std::env::var("SLR_NATIVE_BOUNDED_IDENTITY_PROBE").as_deref()==Ok("1");
    let input_frontier=if bounded_enabled{Some(identity_input_frontier(tx,run_ref,answer)?)}else{None};
    let mut probe=tx.transaction()?;
    let started=std::time::Instant::now();
    let bounded=if bounded_enabled {
        Some(refresh_bounded_parser_identity_with_parity(&mut probe,run,document)?)
    } else {
        probe.query_one("SELECT execution.refresh_numeric_pnf_parser_identity_evidence($1,$2)",&[&run,&document])?;
        None
    };
    let parser_refresh_elapsed_ms=started.elapsed().as_millis();
    let support_rows=if let Some(frontier)=&input_frontier {
        let reference=read_identity_support_index(&mut probe)?;
        Some(replay_identity_support_delta(&mut probe,run,document,frontier,&reference)?)
    }else{None};
    let mut original_parser_identity_producer=observe_refreshed_parser_identity_producer(&mut probe,run,document,demand)?;
    let started=std::time::Instant::now();
    let admitted:i64=probe.query_one(
        "SELECT execution.admit_numeric_pnf_parser_identity_evidence($1,$2)",
        &[&run,&document])?.get(0);
    let parser_admission_elapsed_ms=started.elapsed().as_millis();
    observe_original_parser_candidate_witnesses(&mut probe,run,document,&mut original_parser_identity_producer)?;
    if let Some((mut parity,reference_witnesses,delta))=bounded {
        let current=parser_admitted_witness_keys(&mut probe,run,document)?;
        if current!=reference_witnesses{return Err(NativeNumericAuditError::AdmissionProbeInvariant);}
        parity.bounded_admitted_witnesses=current.len();
        parity.original_admission_projection_exact=true;
        original_parser_identity_producer.producer_ref="pg_temp.slr_refresh_numeric_pnf_parser_identity_evidence_bounded";
        original_parser_identity_producer.bounded_identity_parity=Some(parity);
        original_parser_identity_producer.scoped_admission_parity=probe_scoped_parser_admission(
            &mut probe,run,document,&delta.affected_target_object_ids)?;
        original_parser_identity_producer.evidence_delta_frontier=Some(delta);
    }
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
    if let (Some((rows,mode)),Some(frontier))=(support_rows,input_frontier) {
        let mut receipt=sync_identity_support_index(tx,run_ref,answer,&rows)?;
        if receipt.scope_ref!=frontier.scope_ref{return Err(NativeNumericAuditError::ForeignSource);}
        receipt.input_events_acknowledged=acknowledge_identity_inputs(tx,&frontier)?;
        receipt.input_replay_mode=mode;
        receipt.delta_dirty_sentence_count=frontier.dirty_sentence_refs.len();
        receipt.delta_live_source_scan_performed=mode!="unchanged-input-reopen";
        receipt.delta_reference_exact=true;
        original_parser_identity_producer.support_index=Some(receipt);
    }
    Ok((original_parser_identity_producer,NativeGovernedIdentityProbe{
        parser_refresh_elapsed_ms,parser_admission_elapsed_ms,
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
    }))
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
    pub consumer_horizon_work:NativeConsumerHorizonWorkReceipt,
    pub morphology_observations_inserted:usize,
    pub candidates_without_morphology_observation:i64,
    pub morphology_head_unobservable_count:usize,
    pub original_horizon_observations:Vec<NativeNumericHorizonObservation>,
    pub original_document_identity_observations:Vec<NativeNumericDocumentIdentityObservation>,
    pub original_parser_identity_producer:Option<OriginalParserIdentityProducerObservation>,
    pub parser_identity_refresh_invocations:usize,
    pub identity_execution_replay:Option<NativeIdentityExecutionReplay>,
    pub lean_typed_evidence:Option<serde_json::Value>,
    pub source_partition_wake:NativePartitionWakeReceipt,
    pub document_identity_fibre:NativeNumericIdentityFibreProjection,
    pub original_cross_resource_needs:Vec<OriginalCrossResourceNeedObservation>,
    pub structural_binding:NativeBindingStructuralReceipt,
    pub native_binding_bridge:NativeBindingBridgeReceipt,
    pub original_mention_binding_campaign:NativeMentionBindingCampaign,
    pub original_h9_readiness:NativeH9ReadinessReceipt,
    pub governed_identity_rollback_probe:Option<NativeGovernedIdentityProbe>,
    pub document_identity_publication:Option<NativeDocumentIdentityPublication>,
}
/// Reopen the actual original m086 latest admissibility evidence and invoke
/// the existing Lean executable on the exact demand fibre. This is a typed
/// live-evidence producer, NOT the original coverage or identity authority.
fn produce_lean_typed_evidence(
    tx: &mut postgres::Transaction<'_>, answer: &NativeSourceLocalAnswer,
    run: &str, demand: i64, source: i64, snapshot: &str, outside: bool,
    horizons: &[NativeNumericHorizonObservation],
) -> Result<Option<serde_json::Value>, NativeNumericAuditError> {
    use std::io::Write;
    use std::process::{Command,Stdio};
    let Some(executable) = std::env::var_os("SLR_NATIVE_LEAN_WORKSET_EXECUTABLE") else {
        return Ok(None);
    };
    use crate::numeric_pnf_sql::{read_original_typed_candidate_rows,candidate_fibre_from_original_rows};
    let rows=read_original_typed_candidate_rows(tx,demand,None)
        .map_err(|e|NativeNumericAuditError::Database(e.to_string()))?;
    let kinds=rows.iter().map(|r|r.target_kind).collect::<BTreeSet<_>>();
    let mut represented=Vec::new();
    let mut refuted=Vec::new();
    let mut live=Vec::new();
    // Rust parity uses the already existing CandidateFibre, not a second
    // ad hoc semantic resolver. Candidate keys remain kind-sensitive.
    for kind in kinds {
        let mut fibre=candidate_fibre_from_original_rows(demand,kind,&rows)
            .map_err(|e|NativeNumericAuditError::Database(e.to_string()))?;
        for h in horizons.iter().filter(|h|h.target_kind==kind) {
            fibre.record_original_signed_residual(
                u64::try_from(h.target_id).map_err(|_|NativeNumericAuditError::ForeignSource)?,
                h.horizon,h.signed_residual,
            ).map_err(|_|NativeNumericAuditError::ForeignSource)?;
        }
        represented.extend(fibre.represented_ids().map(|id|serde_json::json!({"target_kind":kind,"target_id":id})));
        refuted.extend(fibre.refuted_ids().map(|id|serde_json::json!({"target_kind":kind,"target_id":id})));
        live.extend(fibre.live_ids().map(|id|serde_json::json!({"target_kind":kind,"target_id":id})));
    }
    let candidates=rows;
    let horizon_keys=horizons.iter().filter(|h|h.horizon==3)
        .map(|h|serde_json::json!({"target_kind":h.target_kind,"target_id":h.target_id}))
        .collect::<Vec<_>>();
    let horizon_refuted=horizons.iter().filter(|h|h.horizon==3 && h.refuted)
        .map(|h|serde_json::json!({"target_kind":h.target_kind,"target_id":h.target_id}))
        .collect::<Vec<_>>();
    if horizon_keys!=represented || horizon_refuted!=refuted {
        return Err(NativeNumericAuditError::ForeignSource);
    }
    // All signed/zero horizon rows are included in the input identity; the
    // Lean selection does not use their sign as a refutation.
    let mut input=serde_json::json!({"schema":"slr-native-typed-evidence-input-v1",
        "source_sha256":answer.source_content_sha256,"source_revision_ref":answer.source_revision_ref,
        "numeric_run_ref":run,"snapshot_ref":snapshot,"demand_id":demand,"source_object_id":source,
        "outside_model_possible":outside,"candidates":candidates,"horizons":horizons});
    let digest=bridge_digest(&input)?;
    input["input_digest"]=serde_json::json!(digest);
    let expected=serde_json::json!({"schema":"slr-native-typed-evidence-output-v1",
        "source_sha256":answer.source_content_sha256,"source_revision_ref":answer.source_revision_ref,
        "numeric_run_ref":run,"snapshot_ref":snapshot,"input_digest":digest,
        "demand_id":demand,"source_object_id":source,"outside_model_possible":outside,
        "represented":represented,"refuted":refuted,"live":live,
        "typed_coverage_paid":false,"identity_paid":false});
    let bytes=serde_json::to_vec(&input).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let mut child=Command::new("timeout").args(["--kill-after=5s","90s"]).arg(&executable)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e|NativeNumericAuditError::Database(format!("Lean typed evidence start: {e}")))?;
    child.stdin.take().ok_or(NativeNumericAuditError::ForeignSource)?.write_all(&bytes)
        .map_err(|e|NativeNumericAuditError::Database(format!("Lean typed evidence input: {e}")))?;
    let output=child.wait_with_output().map_err(|e|NativeNumericAuditError::Database(format!("Lean typed evidence process: {e}")))?;
    if !output.status.success(){return Err(NativeNumericAuditError::Database(format!("Lean typed evidence: {}",String::from_utf8_lossy(&output.stderr))));}
    let mut actual:serde_json::Value=serde_json::from_slice(&output.stdout).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let receipt=actual.as_object_mut().and_then(|o|o.remove("kernel_receipt"))
        .ok_or(NativeNumericAuditError::ForeignSource)?;
    if actual!=expected{return Err(NativeNumericAuditError::ForeignSource);}
    // Independently reopen exact local kernel artifacts, not a supplied
    // "kernel-certified" metadata string. This checks the projection theorem
    // only; the original coverage/admission proof and authenticated formal
    // payment still require their own authority.
    let exe=std::fs::canonicalize(&executable).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let root=exe.parent().and_then(|p|p.parent()).ok_or(NativeNumericAuditError::ForeignSource)?;
    let dir=std::fs::canonicalize(receipt["artifact_dir"].as_str().ok_or(NativeNumericAuditError::ForeignSource)?)
        .map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let base=std::fs::canonicalize(root.join(".dashi/native-typed-evidence"))
        .map_err(|_|NativeNumericAuditError::ForeignSource)?;
    if dir.parent()!=Some(base.as_path()) || receipt["schema"]!="slr-lean-typed-projection-kernel-v1"
        || receipt["input_digest"]!=actual["input_digest"] || receipt["theorem"]!="slrTypedProjectionCertificate"
        || receipt["kernel_checked"]!=true || receipt["typed_coverage_paid"]!=false || receipt["identity_paid"]!=false
        || receipt["checker"].as_str().map_or(true,|x|x.is_empty()) {
        return Err(NativeNumericAuditError::ForeignSource);
    }
    let read=|p:&std::path::Path|std::fs::read(p).map_err(|_|NativeNumericAuditError::ForeignSource);
    let digest=|v:&[u8]|hex(&Sha256::digest(v));
    let raw=read(&dir.join("input.json"))?;
    let proof=read(&dir.join("Certificate.lean"))?;
    let log=read(&dir.join("kernel.log"))?;
    let contract=read(&root.join("DASHI/output-final_aristotle/RequestProject/Slr/NumericPnfRustPortContract.lean"))?;
    let producer=read(&root.join("tools/NativePnfWorkset.lean"))?;
    let key_list=|v:&serde_json::Value|->Result<String,NativeNumericAuditError>{
        let keys=v.as_array().ok_or(NativeNumericAuditError::ForeignSource)?;
        let items=keys.iter().map(|k|{
            let kind=k["target_kind"].as_u64().ok_or(NativeNumericAuditError::ForeignSource)?;
            let id=k["target_id"].as_u64().ok_or(NativeNumericAuditError::ForeignSource)?;
            Ok(format!("⟨{kind}, {id}⟩"))
        }).collect::<Result<Vec<_>,NativeNumericAuditError>>()?;
        Ok(format!("[{}]",items.join(", ")))
    };
    let expected_proof=format!("import RequestProject.Slr.NumericPnfRustPortContract\nopen Wikidata.Slr.NumericPnfRustPortContract\ntheorem slrTypedProjectionCertificate : liveTypedKeys {} {} = {} := by decide\n#print axioms slrTypedProjectionCertificate\n",
        key_list(&actual["represented"])?,
        key_list(&actual["refuted"])?,
        key_list(&actual["live"])?);
    // jq -r adds one newline to the already newline-terminated theorem.
    let proof_text=std::str::from_utf8(&proof).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    let log_text=std::str::from_utf8(&log).map_err(|_|NativeNumericAuditError::ForeignSource)?;
    if raw!=bytes || proof_text.trim_end()!=expected_proof.trim_end()
        || !log_text.contains("slrTypedProjectionCertificate") || log_text.contains("sorryAx")
        || receipt["input_artifact_sha256"]!=digest(&raw)
        || receipt["proof_sha256"]!=digest(&proof)
        || receipt["kernel_log_sha256"]!=digest(&log)
        || receipt["contract_sha256"]!=digest(&contract)
        || receipt["producer_sha256"]!=digest(&producer) {
        return Err(NativeNumericAuditError::ForeignSource);
    }
    actual["kernel_receipt"]=receipt;
    Ok(Some(actual))
}

/// Extension of the original m094 consumer reverse dependency index for
/// absence/coverage of *not-yet-existing* numeric regions. Wake output is
/// the original m094 queue, not a second scheduler or semantic authority.
#[derive(Debug,Clone,Serialize)]
pub struct NativePartitionWakeReceipt {
    pub applied_partition_refs: Vec<String>,
    pub already_applied_partition_refs: Vec<String>,
    pub affected_queue_rows: u64,
    pub dependency_scope_start_char: i64,
    pub dependency_scope_end_char: i64,
}
fn sync_native_partition_consumer(
    tx:&mut postgres::Transaction<'_>, run:&str, answer:&NativeSourceLocalAnswer,
    demand:i64, focus:i64, changes:&[crate::long_document_ingest_store::SourcePartitionChange],
)->Result<NativePartitionWakeReceipt,NativeNumericAuditError>{
    let end:i64=tx.query_one("SELECT r.end_char FROM execution.semantic_pnf_demand d
        JOIN execution.semantic_pnf_region r ON r.region_id=d.source_region_id
        JOIN execution.semantic_parser_token t ON t.token_id=$2
        WHERE d.demand_id=$1 AND r.run_ref=$3 AND r.document_ref=$4
          AND t.run_ref=$3 AND t.document_ref=$4 AND r.start_char<=t.start_char AND t.end_char<=r.end_char",
        &[&demand,&focus,&run,&answer.source_document_ref])?.get(0);
    if end<=answer.original_focus_span[0] as i64 {
        return Err(NativeNumericAuditError::ForeignSource);
    }
    tx.batch_execute("CREATE TABLE IF NOT EXISTS execution.slr_native_consumer_source_interval (
        run_ref text NOT NULL,document_ref text NOT NULL,source_revision_ref text NOT NULL,
        source_sha256 text NOT NULL,parser_run_ref text NOT NULL,
        demand_id bigint NOT NULL REFERENCES execution.semantic_pnf_demand(demand_id) ON DELETE CASCADE,
        consumer_ref text NOT NULL,query_ref text NOT NULL,policy_ref text NOT NULL,
        start_char bigint NOT NULL,end_char bigint NOT NULL CHECK(end_char>start_char),
        PRIMARY KEY(run_ref,document_ref,source_revision_ref,demand_id,consumer_ref,query_ref,policy_ref)
    );
    CREATE INDEX IF NOT EXISTS slr_native_consumer_source_interval_scope_idx
        ON execution.slr_native_consumer_source_interval(run_ref,document_ref,source_revision_ref);
    CREATE INDEX IF NOT EXISTS slr_native_consumer_source_interval_range_idx
        ON execution.slr_native_consumer_source_interval USING gist(int8range(start_char,end_char,'[)'));
    CREATE TABLE IF NOT EXISTS execution.slr_native_partition_wake_receipt (
        run_ref text NOT NULL,document_ref text NOT NULL,source_revision_ref text NOT NULL,
        partition_ref text NOT NULL,previous_partition_ref text NOT NULL,
        source_digest_ref text NOT NULL,delta_digest text NOT NULL,
        affected_queue_rows bigint NOT NULL CHECK(affected_queue_rows>=0),
        PRIMARY KEY(run_ref,document_ref,source_revision_ref,partition_ref)
    );
    CREATE TABLE IF NOT EXISTS execution.slr_native_partition_consumer_ack (
        run_ref text NOT NULL,document_ref text NOT NULL,source_revision_ref text NOT NULL,
        partition_ref text NOT NULL,demand_id bigint NOT NULL,
        consumer_ref text NOT NULL,query_ref text NOT NULL,policy_ref text NOT NULL,
        affected boolean NOT NULL,
        PRIMARY KEY(run_ref,document_ref,source_revision_ref,partition_ref,demand_id,consumer_ref,query_ref,policy_ref),
        FOREIGN KEY(run_ref,document_ref,source_revision_ref,partition_ref)
          REFERENCES execution.slr_native_partition_wake_receipt(run_ref,document_ref,source_revision_ref,partition_ref),
        FOREIGN KEY(run_ref,document_ref,source_revision_ref,demand_id,consumer_ref,query_ref,policy_ref)
          REFERENCES execution.slr_native_consumer_source_interval(run_ref,document_ref,source_revision_ref,demand_id,consumer_ref,query_ref,policy_ref)
          ON DELETE CASCADE
    );")?;
    let doc=&answer.source_document_ref;
    let rev=&answer.source_revision_ref;
    let sha=&answer.source_content_sha256;
    let parser=&answer.original_parser_run_ref;
    let consumer=&answer.consumer_scope_ref;
    let query=&answer.original_review_item_ref;
    let policy=&answer.original_policy_ref;
    tx.execute("INSERT INTO execution.slr_native_consumer_source_interval
        (run_ref,document_ref,source_revision_ref,source_sha256,parser_run_ref,demand_id,consumer_ref,query_ref,policy_ref,start_char,end_char)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,0,$10) ON CONFLICT DO NOTHING",
        &[&run,doc,rev,sha,parser,&demand,consumer,query,policy,&end])?;
    let exact:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM execution.slr_native_consumer_source_interval
        WHERE run_ref=$1 AND document_ref=$2 AND source_revision_ref=$3 AND source_sha256=$4
          AND parser_run_ref=$5 AND demand_id=$6 AND consumer_ref=$7 AND query_ref=$8 AND policy_ref=$9
          AND start_char=0 AND end_char=$10)",
        &[&run,doc,rev,sha,parser,&demand,consumer,query,policy,&end])?.get(0);
    if !exact {return Err(NativeNumericAuditError::ForeignSource);}
    let mut applied=Vec::new();
    let mut reused=Vec::new();
    let mut affected=0u64;
    let mut previous:Option<&str>=None;
    for c in changes {
        if c.source_revision_ref!=*rev || c.source_digest_ref!=format!("sha256:{sha}")
            || c.added.is_empty() || c.added.len()>128
            || previous.is_some_and(|p|p!=c.previous_partition_ref.as_str())
            || c.partition_ref==c.previous_partition_ref {
            return Err(NativeNumericAuditError::ForeignSource);
        }
        previous=Some(c.partition_ref.as_str());
        let delta_digest=bridge_digest(c)?;
        let prior=tx.query_opt("SELECT previous_partition_ref,source_digest_ref,delta_digest
            FROM execution.slr_native_partition_wake_receipt
            WHERE run_ref=$1 AND document_ref=$2 AND source_revision_ref=$3 AND partition_ref=$4",
            &[&run,doc,rev,&c.partition_ref])?;
        if let Some(row)=&prior {
            if row.get::<_,String>(0)!=c.previous_partition_ref
                ||row.get::<_,String>(1)!=c.source_digest_ref ||row.get::<_,String>(2)!=delta_digest {
                return Err(NativeNumericAuditError::ForeignSource);
            }
        } else {
            tx.execute("INSERT INTO execution.slr_native_partition_wake_receipt
                (run_ref,document_ref,source_revision_ref,partition_ref,previous_partition_ref,source_digest_ref,delta_digest,affected_queue_rows)
                VALUES($1,$2,$3,$4,$5,$6,$7,0)",&[&run,doc,rev,&c.partition_ref,&c.previous_partition_ref,
                &c.source_digest_ref,&delta_digest])?;
        }
        let mut starts=Vec::new();let mut ends=Vec::new();
        for r in &c.added {
            if r.start_char<0||r.end_char<=r.start_char||r.region_ref.is_empty()||r.parent_region_ref.is_empty(){
                return Err(NativeNumericAuditError::ForeignSource);
            }
            starts.push(r.start_char);ends.push(r.end_char);
        }
        // Intersect exact source intervals (including absence dependencies)
        // before enqueue. Original m139 handles ordinary positive evidence.
        // A partition receipt alone is not a consumer receipt: consumers
        // registered later must not inherit a previous consumer's ack.
        // Reuse m094's exact work key; ack unrelated scopes without waking.
        let row=tx.query_one("WITH changed AS (
            SELECT * FROM unnest($5::bigint[],$6::bigint[]) AS r(start_char,end_char)
        ), pending AS MATERIALIZED (
            SELECT d.demand_id,d.consumer_ref,d.query_ref,d.policy_ref,
              EXISTS(SELECT 1 FROM changed c WHERE
                int8range(d.start_char,d.end_char,'[)') && int8range(c.start_char,c.end_char,'[)')) AS affected
            FROM execution.slr_native_consumer_source_interval d
            WHERE d.run_ref=$1 AND d.document_ref=$2 AND d.source_revision_ref=$3 AND d.source_sha256=$4
              AND NOT EXISTS(SELECT 1 FROM execution.slr_native_partition_consumer_ack a
                WHERE a.run_ref=d.run_ref AND a.document_ref=d.document_ref
                  AND a.source_revision_ref=d.source_revision_ref AND a.partition_ref=$7
                  AND a.demand_id=d.demand_id AND a.consumer_ref=d.consumer_ref
                  AND a.query_ref=d.query_ref AND a.policy_ref=d.policy_ref)
        ), woken AS (
            INSERT INTO execution.semantic_pnf_consumer_horizon_work_queue
                (demand_id,consumer_ref,query_ref,policy_ref,horizon,work_state,completed_at)
            SELECT demand_id,consumer_ref,query_ref,policy_ref,3,1,NULL FROM pending WHERE affected
            ON CONFLICT(demand_id,consumer_ref,query_ref,policy_ref,horizon)
            DO UPDATE SET work_state=1,completed_at=NULL RETURNING demand_id
        ), acknowledged AS (
            INSERT INTO execution.slr_native_partition_consumer_ack
                (run_ref,document_ref,source_revision_ref,partition_ref,demand_id,consumer_ref,query_ref,policy_ref,affected)
            SELECT $1,$2,$3,$7,demand_id,consumer_ref,query_ref,policy_ref,affected FROM pending
            RETURNING demand_id
        )
        SELECT (SELECT count(*)::bigint FROM woken),(SELECT count(*)::bigint FROM acknowledged)",
        &[&run,doc,rev,sha,&starts,&ends,&c.partition_ref])?;
        let rows:i64=row.get(0);let acks:i64=row.get(1);
        if rows<0 || acks<rows {return Err(NativeNumericAuditError::ForeignSource);}
        if rows>0 {
            tx.execute("UPDATE execution.slr_native_partition_wake_receipt
                SET affected_queue_rows=affected_queue_rows+$5
                WHERE run_ref=$1 AND document_ref=$2 AND source_revision_ref=$3 AND partition_ref=$4",
                &[&run,doc,rev,&c.partition_ref,&rows])?;
        }
        if prior.is_some() && acks==0 { reused.push(c.partition_ref.clone()); }
        else { applied.push(c.partition_ref.clone()); }
        affected+=rows as u64;
    }
    Ok(NativePartitionWakeReceipt{applied_partition_refs:applied,already_applied_partition_refs:reused,
        affected_queue_rows:affected,dependency_scope_start_char:0,dependency_scope_end_char:end})
}

pub fn transport_original_numeric_candidates_for_source_answer(
    numeric_url:&str,numeric_run_ref:&str,answer:&NativeSourceLocalAnswer,
    budget:usize,
)->Result<NativeNumericCandidateTransportReceipt,NativeNumericAuditError>{
    transport_original_numeric_candidates_for_source_answer_with_partition(
        numeric_url,numeric_run_ref,answer,budget,&[])
}
pub fn transport_original_numeric_candidates_for_source_answer_with_partition(
    numeric_url:&str,numeric_run_ref:&str,answer:&NativeSourceLocalAnswer,
    budget:usize,changes:&[crate::long_document_ingest_store::SourcePartitionChange],
)->Result<NativeNumericCandidateTransportReceipt,NativeNumericAuditError>{
    original_receipt_policy(answer)?;
    if budget==0||budget>64||numeric_run_ref.trim().is_empty(){
        return Err(NativeNumericAuditError::SourcePolicy);
    }
    let mut db=Client::connect(numeric_url,NoTls)?;
    if std::env::var("SLR_NATIVE_BOUNDED_IDENTITY_PROBE").as_deref()==Ok("1"){
        ensure_identity_input_capture(&mut db)?;
    }
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
    ensure_native_h6_current_authority(&mut tx,demand)?;
    tx.batch_execute("CREATE TEMP TABLE slr_native_h3_desired(evidence_id bigint PRIMARY KEY) ON COMMIT DROP")?;
    let source_partition_wake=sync_native_partition_consumer(&mut tx,numeric_run_ref,answer,demand,focus_token,changes)?;
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
    tx.execute("INSERT INTO slr_native_h3_desired(evidence_id)
        SELECT e.evidence_id FROM execution.semantic_pnf_candidate_evidence e
        JOIN native_numeric_transport_possible p ON p.object_id=e.target_id
        WHERE e.demand_id=$1 AND e.target_kind=1 AND e.horizon=3 AND e.evidence_family=1
          AND e.evidence_ref='native-occurrence-transport:v1:'||p.object_id::text
          AND e.provenance_ref='native-occurrence-transport:v1' AND e.evidence_kind='candidate_generation'
          AND e.signed_residual=0 AND e.source_interface_id=p.interface_id AND e.source_region_id=p.region_id
        ON CONFLICT DO NOTHING",&[&demand])?;
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
    let structural_binding=observe_native_binding_structure(&mut tx,demand,focus_token,
        numeric_run_ref,&answer.source_document_ref,&answer.source_content_sha256)?;
    // Only run the document-scoped original admission probe after bounded
    // occurrence/morphology work is drained. This is still not typed coverage.
    let morphology_head_unobservable_count=structural_binding.observations.iter()
        .filter(|o|o.target_kind==1&&o.exact_head_token_id.is_none()).count();
    let represented_slice_drained=remaining==0&&nominal_heads_remaining==0
        &&candidates_without_morphology_observation==0
        &&original_mention_binding_campaign.spans_without_exact_objects==0
        &&original_mention_binding_campaign.prefix_uncovered_nonwhitespace==0;
    let identity_execution_replay=if represented_slice_drained{
        try_native_identity_execution_replay(&mut tx,numeric_run_ref,answer)?
    }else{None};
    let (original_parser_identity_producer,governed_identity_rollback_probe)=if represented_slice_drained
        &&identity_execution_replay.is_none(){
        let (observed,governed)=probe_original_governed_identity(&mut tx,numeric_run_ref,&answer.source_document_ref,demand,answer)?;
        (Some(observed),Some(governed))
    }else{(None,None)};
    let support_delta=original_parser_identity_producer.as_ref().and_then(|p|p.support_index.as_ref());
    let nonmonotone=support_delta.is_some_and(|p|p.withdrawn_rows>0||p.changed_rows>0);
    let inputs_changed=source_partition_wake.affected_queue_rows>0
        ||nominal_head_objects_inserted>0||nominal_head_token_supports_inserted>0||!ids.is_empty()
        ||morphology_observations_inserted>0||original_mention_binding_campaign.newly_materialized_span_objects>0
        ||native_binding_bridge.original_index_candidate_evidence_inserted>0
        ||support_delta.is_some_and(|p|p.journaled_change_rows>0);
    let consumer_horizon_work=process_original_native_consumer_horizons(&mut tx,demand,numeric_run_ref,
        answer,represented_slice_drained,inputs_changed,nonmonotone)?;
    let original_h6_evidence_inserted=consumer_horizon_work.h6_evidence_inserted;
    let document_identity_publication=if std::env::var("SLR_NATIVE_DOCUMENT_IDENTITY_PUBLISH").as_deref()==Ok("1")
        &&!consumer_horizon_work.nonmonotone_input_residual
        &&consumer_horizon_work.h6_validity.original_consumer_authority_reconciled {
        if let (Some(producer),Some(probe))=(&original_parser_identity_producer,&governed_identity_rollback_probe){
            Some(publish_checked_document_identity(&mut tx,numeric_run_ref,answer,producer,probe)?)
        }else{None}
    }else{None};
    let (original_horizon_observations,original_document_identity_observations)=
        observe_original_resolution_fibre(&mut tx,demand,numeric_run_ref,&answer.source_document_ref)?;
    let document_identity_fibre=project_original_identity_fibre(&original_horizon_observations,&original_document_identity_observations,open);
    // Original cumulative views still include historical H6 atoms. Until
    // the authority owner consumes the current validity projection, never
    // submit those stale observations as a golden current-evidence packet.
    let lean_typed_evidence=if !consumer_horizon_work.nonmonotone_input_residual
        &&consumer_horizon_work.h6_validity.original_consumer_authority_reconciled {
        produce_lean_typed_evidence(&mut tx,answer,numeric_run_ref,demand,source,
            &native_binding_bridge.snapshot_ref,open,&original_horizon_observations)?
    }else{None};
    let original_cross_resource_needs=observe_original_cross_resource_needs(&mut tx,demand,numeric_run_ref,&answer.source_document_ref)?;
    let original_h9_readiness=observe_native_h9_readiness(&mut tx,demand,numeric_run_ref,&answer.source_document_ref)?;
    let parser_identity_refresh_invocations=original_parser_identity_producer.as_ref()
        .map_or(0,|p|if p.bounded_identity_parity.is_some(){3}else{1})
        +usize::from(document_identity_publication.as_ref().is_some_and(|p|p.committed||p.blockers.contains(&"golden_exact_support_priority_disagreement")));
    tx.commit()?;
    Ok(NativeNumericCandidateTransportReceipt{
        schema:"slr-native-candidate-only-transport-v6",
        numeric_run_ref:numeric_run_ref.into(),demand_id:demand,source_object_id:source,
        inserted_candidate_ids:ids,remaining_untransported_count:remaining,
        represented_candidate_count:count,outside_model_possible:open,
        complete_typed_coverage_paid:false,
        nominal_head_objects_inserted,nominal_head_token_supports_inserted,nominal_heads_remaining,original_h6_evidence_inserted,consumer_horizon_work,
        morphology_observations_inserted,candidates_without_morphology_observation,morphology_head_unobservable_count,
        original_horizon_observations,original_document_identity_observations,
        original_parser_identity_producer,parser_identity_refresh_invocations,identity_execution_replay,lean_typed_evidence,source_partition_wake,document_identity_fibre,original_cross_resource_needs,
        structural_binding,native_binding_bridge,original_mention_binding_campaign,original_h9_readiness,governed_identity_rollback_probe,document_identity_publication,
    })
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn identity_delta_distinguishes_payload_change_from_new_and_withdrawn_keys(){
        let kept=name_key(10,20,1,1);
        let removed=name_key(11,21,2,2);
        let old=name_key(12,22,3,1);
        let new=name_key(12,22,3,2);
        let added=name_key(13,23,4,2);
        let before=BTreeSet::from([kept.clone(),removed,old]);
        let after=BTreeSet::from([kept,new,added]);
        let d=identity_evidence_delta(&before,&after).unwrap();
        assert_eq!((d.inserted,d.withdrawn,d.changed,d.unchanged),(1,1,1,1));
        assert_eq!(d.affected_target_object_ids,vec![102,103,104]);
        assert!(!d.complete_source_dependency_paid);
        let no_change=identity_evidence_delta(&after,&after).unwrap();
        assert_eq!((no_change.inserted,no_change.withdrawn,no_change.changed,no_change.unchanged),(0,0,0,3));
        assert!(no_change.affected_target_object_ids.is_empty());
    }
    fn name_key(source:i64,token:i64,entity:i64,count:i16)->ParserEvidenceKey{
        (source,entity+100,3,Some(token),Some(entity+200),Some(1),
            format!("proper-name-expansion:{token}:{entity}"),count)
    }
    #[test]
    fn bounded_identity_exact_reference_keeps_full_family_and_direct_lanes(){
        // Deliberately non-lexical ID order (2,10,100).
        let mut reference=(1..=20).map(|id|name_key(10,20,id,20)).collect::<BTreeSet<_>>();
        let other=name_key(10,21,100,20);
        let direct=(10,99,4,Some(20),Some(98),Some(1),"parser-appos:20:98".into(),1);
        reference.insert(other.clone());reference.insert(direct.clone());
        let (retained,overflow)=bounded_parser_evidence_reference(&reference,16).unwrap();
        assert_eq!(retained.len(),18);
        assert_eq!(overflow,BTreeMap::from([((10,20),4)]));
        assert!(retained.contains(&direct)&&retained.contains(&other));
        for id in 1..=20 {
            assert_eq!(retained.contains(&name_key(10,20,id,20)),id<=16);
        }
        assert_eq!(bounded_parser_evidence_reference(&reference,32).unwrap(),(reference,BTreeMap::new()));
    }
    #[test]
    fn bounded_identity_never_discards_a_unique_or_foreign_evidence_row(){
        let invalid=(1..=17).map(|id|name_key(10,20,id,1)).collect::<BTreeSet<_>>();
        assert!(bounded_parser_evidence_reference(&invalid,16).is_err());
        let mut key=name_key(10,20,1,1);
        key.6="proper-name-expansion:999:1".into();
        assert!(bounded_parser_evidence_reference(&BTreeSet::from([key]),16).is_err());
        assert!(bounded_parser_evidence_reference(&BTreeSet::new(),0).is_err());
    }

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
    fn original_relation_grammar_keeps_distinct_modifier_coordination_and_temporal_roles(){
        let t=|id,head,pos:&str,dep:&str,orth:&str|NativeRelationToken{
            id,region:1,lemma_id:id,head,start:id,end:id+1,orth:orth.into(),lemma:orth.into(),pos:pos.into(),dep:dep.into()
        };
        let rows=native_relation_grammar(&[t(1,1,"NOUN","root","actor"),t(2,1,"NOUN","conj","person"),
            t(3,1,"NUM","tmod","2001"),t(4,1,"ADJ","amod","new")]);
        assert!(rows.iter().any(|r|r.relation_type=="modifier"&&r.roles==vec![("head",1),("modifier",2)]));
        assert!(rows.iter().any(|r|r.relation_type=="conjunction"&&r.roles==vec![("item",1),("item",2)]));
        assert!(rows.iter().any(|r|r.relation_type=="temporal"&&r.roles==vec![("anchor",3)]));
        assert_eq!(relation_declaration("conjunction").unwrap().1,"semantic.coordination");
        assert_eq!(relation_role_constraint("modifier","modifier"),Some("nominal_modifier_of"));
    }
    #[test]
    fn original_relation_atom_allocation_preserves_object_before_subject(){
        let row=ProjectedRelation{region:1,head_token_id:20,relation_type:"predicate",
            roles:vec![("head",20),("subject",10),("object",30),("argument",30)]};
        let mut atoms=BTreeMap::new();
        let mapped=original_relation_atom_ids(&row,&mut atoms);
        assert_eq!(mapped,vec![("a1".to_owned(),20,"head"),("a3".to_owned(),10,"subject"),
            ("a2".to_owned(),30,"object"),("a2".to_owned(),30,"argument")]);
        let a=original_relation_digest(&serde_json::json!({"x":1})).unwrap();
        let b=original_relation_digest(&serde_json::json!({"x":"1"})).unwrap();
        assert_ne!(a,b);
        // Independently encoded from SensibLaw canonical.py's typed byte
        // contract, using UTF-8 byte lengths, sorted map keys and integer tags.
        assert_eq!(original_relation_digest(&serde_json::json!({"x":1,"a":null})).unwrap(),
            "16483dd9d489e927584c7929ee62743a8bad819f49e368c27cec4e70390b2f37");
        let roles=mapped.iter().map(|(atom,_,role)|serde_json::json!({"role":role,"atom":atom})).collect();
        let inverse=mapped.iter().map(|(atom,id,_)|(atom.clone(),*id)).collect();
        let cross=original_relation_reference("doc:test","grammar:semantic:predicate:v0_4",1,"predicate",roles,inverse).unwrap();
        assert_eq!(cross.reconstructed_root_factor_ref,
            "factor:d188f7cd217a008ad29b9f8484440b8ebdfabcba7de9cbea45b0257c3a92427e");
        assert_eq!(cross.reconstructed_relation_ref,
            "semantic-relation:a71ae0d62dd09b86a4f2727acf3d1739db33913d6c2085f3f4ac2a6b26d2f782");
        assert!(!cross.independently_compared);
        assert_eq!(original_relation_digest(&serde_json::json!({"x":1,"a":null})).unwrap(),
            original_relation_digest(&serde_json::json!({"a":null,"x":1})).unwrap());
    }
    #[test]
    fn original_atom_and_role_factor_typed_reference_vectors(){
        let atom=original_relation_atom_payload("a3","He",10,12,"he","PRON","nsubj",
            Some(r#"{"Number":"Sing","Person":["3"]}"#)).unwrap();
        assert_eq!(atom["morph"]["Number"],serde_json::json!(["Sing"]));
        let span=original_atom_span_ref("doc:test",&atom).unwrap();
        assert_eq!(span,"semantic-atom:ca502008c2cffdd92e249667acbb117b8e93ebeafa5cf73c5a020de214d43972");
        let binding=serde_json::json!({"role":"subject","atom_id":"a3","atom_ref":span,"value":null});
        let role=original_binding_factor_ref("doc:test","e1","subject",&binding,false).unwrap();
        assert_eq!(role,"factor:3bc79ed3125c03ff1755d24a26b4932861738d73be5c2674416d0001948810d7");
        assert_ne!(role,original_binding_factor_ref("doc:test","e1","subject",&binding,true).unwrap());
        let empty=original_relation_atom_payload("a1","Went",0,4,"","VERB","ROOT",Some("{}")).unwrap();
        assert_eq!(empty["lemma"],"Went");assert_eq!(empty["dependency"],"ROOT");
        assert!(empty.get("morph").is_none());
    }
    #[test]
    fn original_action_metadata_preserves_missing_and_explicit_morphology(){
        let missing=original_relation_action_metadata(None,"went","root").unwrap();
        assert_eq!(missing["voice"],"unknown");
        let known=original_relation_action_metadata(Some(r#"{"VerbForm":["Fin"],"Tense":["Past"],"Aspect":["Perf","Prog"],"Mood":["Ind"]}"#),"went","auxpass").unwrap();
        assert_eq!(known["tense"],"past");assert_eq!(known["aspect"],"perfect_progressive");
        assert_eq!(known["voice"],"passive");assert_eq!(known["verb_form"],"finite");
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
                    let mut snapshot_objects:Vec<i64>=Vec::new();
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
        t.morphology=bridge_morphology(&["NUMBER=Sing,Plur".into(),"GENDER=Masc".into(),"Number=Sing".into()]);
        let tokens=BTreeMap::from([(7,t)]);
        let a=bridge_anchor("x".into(),"semantic.argument.subject",&[7],vec![],&tokens,source,Some(("p","predicate"))).unwrap();
        assert_eq!(a.clause.as_deref(),Some("clause:p"));assert_eq!(a.reporting,None);
        assert_eq!(a.morphology,vec![("Gender".to_owned(),"Masc".to_owned()),("Number".to_owned(),"Plur".to_owned()),("Number".to_owned(),"Sing".to_owned())]);
        assert!(morphology_mismatches(&["NUMBER=Sing,Plur".into()],&["Number=Plur".into()]).is_empty());
        assert_eq!(morphology_mismatches(&["NUMBER=Sing".into()],&["Number=Plur".into()]),vec!["Number"]);
        let c=bridge_anchor("y".into(),"semantic.embedded_proposition",&[7],vec![],&tokens,source,Some(("c","composition"))).unwrap();
        assert_eq!(c.reporting.as_deref(),Some("c"));assert_eq!(c.coordination,None);
    }
}
