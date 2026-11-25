//! Explicit, bounded native numeric-PNF operator CLI.
//!
//! Inspect and plan are READ ONLY against the ORIGINAL SLR source DB.
//! Compile is opt-in through an exact --commit flag and operates only on
//! original numeric Postgres with bounded sentence lease work.
//! Never performs external QID admission or treats parser/coreference
//! evidence as an admitted document identity.

use sensiblaw_pg_source_store::{
    load_database_config, load_exact_native_numeric_evidence,
    plan_original_native_numeric_parser, deterministic_numeric_run_ref,
    run_native_numeric_exact_source_slice, run_native_numeric_prefix_slice, OriginalNativeNumericEvidence,
    inspect_native_antecedent_review, native_antecedent_frontier,
    compile_native_source_local_answer,
    audit_original_numeric_identity_for_source_answer,
    reuse_original_binding_and_identity_owners,
    retract_original_numeric_identity_for_source_answer,
    transport_original_numeric_candidates_for_source_answer,
    prepare_native_prefix_recovery, complete_parser_run,
    load_source_partition_changes, transport_original_numeric_candidates_for_source_answer_with_partition,
};
use serde::Serialize;
use std::{env,path::PathBuf};

#[derive(Serialize)]
struct NativeNumericPlanSummary {
    schema: &'static str,
    source_revision_ref: String,
    original_parser_run_ref: String,
    numeric_run_ref: String,
    document_ref: String,
    canonical_source_sha256: String,
    canonical_source_byte_length: usize,
    source_sentence_fibres: usize,
    source_token_count: usize,
    source_entity_count: usize,
    numeric_sentence_rows: usize,
    numeric_token_rows: usize,
    numeric_entity_rows: usize,
    full_document_coverage_paid: bool,
    semantic_identity_paid: bool,
    level_three_paid: bool,
    external_qid_paid: bool,
    residuals: Vec<String>,
}

fn exact_plan(
    source_revision: &str,
    parser_run: &str,
) -> Result<(OriginalNativeNumericEvidence,NativeNumericPlanSummary),String> {
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    let evidence=load_exact_native_numeric_evidence(
        &config,source_revision,parser_run,"slr-strict-numeric-pnf-v1"
    ).map_err(|e|e.to_string())?;
    let numeric_run=deterministic_numeric_run_ref(&evidence.manifest);
    let plan=plan_original_native_numeric_parser(&evidence,&numeric_run)
        .map_err(|e|e.to_string())?;
    let summary=NativeNumericPlanSummary{
        schema:"slr-native-numeric-pnf-source-plan-v1",
        source_revision_ref:evidence.manifest.source_revision_ref.clone(),
        original_parser_run_ref:evidence.manifest.parser_run_ref.clone(),
        numeric_run_ref:numeric_run,
        document_ref:evidence.manifest.document_ref.clone(),
        canonical_source_sha256:evidence.manifest.canonical_content_sha256.clone(),
        canonical_source_byte_length:evidence.manifest.source_byte_length,
        source_sentence_fibres:evidence.original_region_count,
        source_token_count:evidence.original_token_count,
        source_entity_count:evidence.parser_regions.iter().map(|x|x.entities.len()).sum(),
        numeric_sentence_rows:plan.sentences.len(),
        numeric_token_rows:plan.sentences.iter().map(|s|s.tokens.len()).sum(),
        numeric_entity_rows:plan.sentences.iter().map(|s|s.entities.len()).sum(),
        full_document_coverage_paid:evidence.full_document_coverage_paid,
        semantic_identity_paid:false,level_three_paid:false,external_qid_paid:false,
        residuals:plan.residuals,
    };
    Ok((evidence,summary))
}

fn print_json<T:Serialize>(value:&T)->Result<(),String>{
    println!("{}",serde_json::to_string_pretty(value)
        .map_err(|e|e.to_string())?);
    Ok(())
}
pub fn run(args:&[String])->Result<(),String>{
    match args {
        [subcommand,review,parser_run,flag] if subcommand=="prefix-prepare"&&(flag=="--commit"||flag=="--inspect")=>{
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let receipt=prepare_native_prefix_recovery(&config,review,parser_run,flag=="--commit")?;
            print_json(&receipt)
        }
        [subcommand,review,parser_run,flag] if subcommand=="endpoint"&&flag=="--commit"=>{
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            // Existing parser-only worker may already have completed queued
            // jobs. Finalize just this original run, never source compilation.
            let scope=prepare_native_prefix_recovery(&config,review,parser_run,false)?;
            if !scope.parser_scope_ready{
                return print_json(&serde_json::json!({
                    "schema":"slr-native-antecedent-endpoint-v1","prefix_scope":scope,
                    "outcome":"unresolved_original_prefix_parser_scope",
                    "governed_identity_paid":false,"level_three_paid":false
                }));
            }
            complete_parser_run(&config,parser_run).map_err(|e|e.to_string())?;
            let url=env::var("SENSIBLAW_DATABASE_URL").map_err(|_|"SENSIBLAW_DATABASE_URL required")?;
            let root=env::var("SLR_NATIVE_NUMERIC_SOURCE_ROOT").map(PathBuf::from)
                .map_err(|_|"SLR_NATIVE_NUMERIC_SOURCE_ROOT required")?;
            let mut publication=Vec::new();
            for _ in 0..8 {
                let step=run_native_numeric_prefix_slice(&config,&url,&scope.source_revision_ref,parser_run,&root,64,
                    i64::try_from(scope.focus_start_char).map_err(|e|e.to_string())?)
                    .map_err(|e|e.to_string())?;
                let done=step.sentence_jobs_remaining==0&&
                    !step.residuals.iter().any(|r|r.starts_with("original_numeric_parser_sentence_bootstrap_pending:"));
                publication.push(step);
                if done{break;}
            }
            let inspection=inspect_native_antecedent_review(&config,review).map_err(|e|e.to_string())?;
            let frontier=native_antecedent_frontier(&config,review,parser_run).map_err(|e|e.to_string())?;
            let answer=compile_native_source_local_answer(&inspection,&frontier).map_err(|e|e.to_string())?;
            if !answer.source_local_policy_inference_paid{
                return print_json(&serde_json::json!({"schema":"slr-native-antecedent-endpoint-v1",
                    "prefix_scope":scope,"numeric_publication":publication,"source_local_answer":answer,
                    "outcome":"source_local_policy_unresolved","governed_identity_paid":false,"level_three_paid":false}));
            }
            // The original publication receipt already binds the immutable
            // source/parser manifest. Do not reopen the full original tape.
            let numeric_run=publication.last().ok_or("no scoped publication receipt")?.numeric_run_ref.clone();
            let source_partition_changes=load_source_partition_changes(&config,&scope.source_revision_ref,
                &format!("sha256:{}",scope.source_sha256)).map_err(|e|e.to_string())?;
            let mut passes=Vec::new();
            for _ in 0..8 {
                let step=transport_original_numeric_candidates_for_source_answer_with_partition(
                    &url,&numeric_run,&answer,64,&source_partition_changes)
                    .map_err(|e|e.to_string())?;
                let done=step.governed_identity_rollback_probe.is_some();
                let progress=!step.inserted_candidate_ids.is_empty()||step.nominal_head_objects_inserted>0
                    ||step.original_mention_binding_campaign.newly_materialized_span_objects>0
                    ||step.morphology_observations_inserted>0;
                passes.push(step);
                if done||!progress{break;}
            }
            let readback=reuse_original_binding_and_identity_owners(&url,&numeric_run,&answer).map_err(|e|e.to_string())?;
            let last=passes.last().ok_or("no native binding pass")?;
            let prefix_complete=last.original_mention_binding_campaign.prefix_uncovered_nonwhitespace==0;
            let cross_owner=last.native_binding_bridge.indexed_entity_spans_missing_from_projected.is_empty()
                &&last.native_binding_bridge.projected_entity_spans_missing_from_index.is_empty();
            let entity_sets=readback.original_binding_sets.iter().filter(|s|s.same_manifestation
                &&s.referential_type_ref=="entity_reference"&&s.reference_factor_revision_ref.starts_with("factor-revision:native-binding-v2:"))
                .collect::<Vec<_>>();
            let inverse=!entity_sets.is_empty()&&entity_sets.iter().all(|s|s.exact_entity_inverse_complete
                &&s.native_projection_verified&&s.original_index_matches_persisted_members
                &&s.original_index_prefix_matches_full_query);
            let projected=last.governed_identity_rollback_probe.as_ref().map(|p|&p.candidate_class_projection);
            let mut rivals=Vec::new();
            let mut typed_objects=std::collections::BTreeSet::new();
            for set in &entity_sets{
                for member in &set.original_members{
                    let objects=&member.exact_same_manifestation_numeric_object_ids;
                    typed_objects.extend(objects.iter().copied());
                    let original_person_support=answer.original_person_support.iter().filter(|support|
                        member.original_span_coordinates.iter().any(|(a,b)|
                            i64::from(*a)<=support.start_char as i64&&i64::from(*b)>=support.end_char as i64))
                        .collect::<Vec<_>>();
                    let prospective_classes=projected.map(|p|p.admitted_classes.iter()
                        .filter(|(_,ids)|ids.iter().any(|id|objects.contains(id)))
                        .map(|(id,_)|*id).collect::<Vec<_>>()).unwrap_or_default();
                    let persistent_classes=last.original_document_identity_observations.iter()
                        .filter(|o|objects.contains(&o.object_id)).collect::<Vec<_>>();
                    let signed_horizons=last.original_horizon_observations.iter()
                        .filter(|o|o.target_kind==1&&objects.contains(&o.target_id)).collect::<Vec<_>>();
                    rivals.push(serde_json::json!({"binding_set_ref":set.candidate_set_ref,
                        "factor_revision_ref":member.candidate_factor_revision_ref,
                        "spans":member.original_span_coordinates,"object_ids":objects,
                        "exact_token_ids":member.exact_projection_token_ids,
                        "original_person_support_contained_not_identity":original_person_support,
                        "actual_document_identity_observations":persistent_classes,
                        "rollback_only_entity_class_ids":prospective_classes,"signed_horizons":signed_horizons,
                        "accessibility_path":member.accessibility_path_ref,
                        "distance_tokens":member.distance_tokens}));
                }
            }
            let transported_not_indexed=last.original_document_identity_observations.iter()
                .filter(|o|!typed_objects.contains(&o.object_id)).map(|o|o.object_id).collect::<Vec<_>>();
            let mut blockers=Vec::new();
            if !prefix_complete{blockers.push("exact_numeric_prefix_token_coverage_unpaid");}
            if !cross_owner{blockers.push("original_projected_typed_membership_disagreement");}
            if !inverse{blockers.push("typed_entity_inverse_unpaid");}
            if !last.complete_typed_coverage_paid{blockers.push("original_typed_coverage_witness_unpaid");}
            if last.outside_model_possible{blockers.push("outside_model_possible");}
            if !last.native_binding_bridge.structural_relation_parity_paid{blockers.push("full_original_relation_parity_unpaid");}
            if last.morphology_head_unobservable_count>0{blockers.push("scalar_morphology_head_residual");}
            if !transported_not_indexed.is_empty(){blockers.push("transported_objects_outside_typed_index_not_refuted");}
            let typed_dossier=serde_json::json!({
                "full_index_entity_rivals":rivals,"typed_entity_object_count":typed_objects.len(),
                "transported_objects_outside_index":transported_not_indexed,
                "focus_reference_alternatives":last.native_binding_bridge.focus_reference_alternatives,
                "prospective_identity_classes_not_admitted":projected,
                "blockers":blockers,"automatic_semantic_refutation_from_index_absence":false
            });
            let identity=readback.original_numeric_audits.iter().any(|a|a.numeric_run_ref==numeric_run&&a.original_accepted_document_identity_paid);
            let l3=readback.original_numeric_audits.iter().any(|a|a.numeric_run_ref==numeric_run&&a.original_level_three_substitution_paid);
            let endpoint=if identity&&l3&&prefix_complete&&cross_owner&&inverse{
                "governed_source_answer"
            }else{"source_local_answer_with_governed_identity_unresolved"};
            print_json(&serde_json::json!({
                "schema":"slr-native-antecedent-endpoint-v1","prefix_scope":scope,
                "numeric_publication":publication,"binding_passes":passes,"original_owner_reuse":readback,
                "source_partition_changes":source_partition_changes,
                "source_local_answer":answer,"typed_consumer_dossier":typed_dossier,
                "observed_prefix_token_coverage":prefix_complete,
                "projected_vs_original_entity_membership_parity":cross_owner,"exact_entity_inverse_complete":inverse,
                "outcome":endpoint,"governed_identity_paid":identity,"level_three_paid":l3,
                "external_qid_paid":false,"historical_truth_paid":false
            }))
        }
        [subcommand,source_revision,parser_run] if subcommand=="inspect" => {
            let (evidence,_)=exact_plan(source_revision,parser_run)?;
            let total_entities=evidence.parser_regions.iter()
                .map(|r|r.entities.len()).sum::<usize>();
            print_json(&serde_json::json!({
                "schema":"slr-native-numeric-source-inspection-v1",
                "original_source_revision_ref":source_revision,
                "original_parser_run_ref":parser_run,
                "canonical_document_ref":evidence.manifest.document_ref,
                "source_sha256":evidence.manifest.canonical_content_sha256,
                "source_byte_length":evidence.manifest.source_byte_length,
                "source_codepoint_length":evidence.manifest.source_codepoint_length,
                "sentence_regions":evidence.original_region_count,
                "token_count":evidence.original_token_count,
                "entity_count":total_entities,
                "full_original_parser_coverage":evidence.full_document_coverage_paid,
                "residuals":evidence.residuals,
                "semantic_identity_paid":false,
                "external_qid_paid":false,
                "writes_database":false
            }))
        }
        [subcommand,source_revision,parser_run] if subcommand=="plan" => {
            let (_,summary)=exact_plan(source_revision,parser_run)?;
            print_json(&summary)
        }
        [subcommand,review_item_ref,original_parser_run]
        | [subcommand,review_item_ref,original_parser_run,_,_]
            if (matches!(subcommand.as_str(),"audit"|"reuse-original") && args.len()==3)
                || (subcommand=="retract" && args.len()==5)
                || (matches!(subcommand.as_str(),"bind"|"bind-campaign") && args.len()==5) => {
            let transport_budget=if matches!(subcommand.as_str(),"bind"|"bind-campaign"){
                if args[4]!="--commit"{return Err("candidate transport requires --commit".into());}
                let n=args[3].parse::<usize>().map_err(|_|"invalid candidate budget")?;
                if !(1..=64).contains(&n){return Err("candidate budget must be 1..=64".into());}
                Some(n)
            }else{None};
            let retraction=if subcommand=="retract" {
                let id=args[3].parse::<i64>().map_err(|_|"invalid witness ID")?;
                let commit=match args[4].as_str(){
                    "--commit"=>true,"--rollback-probe"=>false,
                    _=>return Err("retraction requires --commit or --rollback-probe".into()),
                };
                if id<=0{return Err("invalid witness ID".into());}
                Some((id,commit))
            }else{None};
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let inspection=inspect_native_antecedent_review(
                &config,review_item_ref
            ).map_err(|e|e.to_string())?;
            let frontier=native_antecedent_frontier(
                &config,review_item_ref,original_parser_run
            ).map_err(|e|e.to_string())?;
            let answer=compile_native_source_local_answer(
                &inspection,&frontier
            ).map_err(|e|e.to_string())?;
            let evidence=load_exact_native_numeric_evidence(
                &config,&answer.source_revision_ref,original_parser_run,
                "slr-strict-numeric-pnf-v1"
            ).map_err(|e|e.to_string())?;
            if evidence.manifest.canonical_content_sha256!=
                answer.source_content_sha256{
                return Err("source consumer and numeric compiler use different canonical bytes".into());
            }
            let numeric_url=env::var("SENSIBLAW_DATABASE_URL")
                .map_err(|_|"numeric audit requires SENSIBLAW_DATABASE_URL")?;
            let numeric_run=deterministic_numeric_run_ref(&evidence.manifest);
            if subcommand=="reuse-original" {
                let receipt=reuse_original_binding_and_identity_owners(&numeric_url,&numeric_run,&answer)
                    .map_err(|e|e.to_string())?;
                return print_json(&receipt);
            }
            if let Some(budget)=transport_budget {
                if subcommand=="bind-campaign" {
                    let mut passes=Vec::new();
                    let mut stop_reason="bounded_pass_limit";
                    for _ in 0..8 {
                        let receipt=transport_original_numeric_candidates_for_source_answer(
                            &numeric_url,&numeric_run,&answer,budget
                        ).map_err(|e|e.to_string())?;
                        let done=receipt.governed_identity_rollback_probe.is_some();
                        let progress=!receipt.inserted_candidate_ids.is_empty()
                            ||receipt.nominal_head_objects_inserted>0
                            ||receipt.original_mention_binding_campaign.newly_materialized_span_objects>0
                            ||receipt.morphology_observations_inserted>0;
                        passes.push(receipt);
                        if done{stop_reason="bounded_execution_drained_governed_probe_completed";break;}
                        if !progress{stop_reason="unresolved_without_execution_progress";break;}
                    }
                    let owner_reuse=reuse_original_binding_and_identity_owners(&numeric_url,&numeric_run,&answer)
                        .map_err(|e|e.to_string())?;
                    return print_json(&serde_json::json!({
                        "schema":"slr-native-binding-campaign-v2",
                        "stop_reason":stop_reason,"committed_passes":passes.len(),
                        "passes":passes,"original_owner_reuse":owner_reuse,"focus_identity_paid":false,"typed_coverage_paid":false
                    }));
                }
                let receipt=transport_original_numeric_candidates_for_source_answer(
                    &numeric_url,&numeric_run,&answer,budget
                ).map_err(|e|e.to_string())?;
                return print_json(&receipt);
            }
            if let Some((witness_id,commit))=retraction {
                let receipt=retract_original_numeric_identity_for_source_answer(
                    &numeric_url,&numeric_run,&answer,witness_id,commit
                ).map_err(|e|e.to_string())?;
                return print_json(&receipt);
            }
            let audit=audit_original_numeric_identity_for_source_answer(
                &numeric_url,&numeric_run,&answer
            ).map_err(|e|e.to_string())?;
            print_json(&audit)
        }
        [subcommand,source_revision,parser_run,budget,flag]
        | [subcommand,source_revision,parser_run,budget,flag,_]
            if subcommand=="compile" && flag=="--commit" => {
            if matches!(args,[_,_,_,_,_,arg] if arg!="--allow-partial-source") {
                return Err("unexpected numeric source compile option".into());
            }
            let max_sentences=budget.parse::<usize>()
                .map_err(|_|"sentence budget must be a positive integer".to_owned())?;
            if !(1..=64).contains(&max_sentences){
                return Err("bounded native compiler supports 1..=64 sentences per invocation".into());
            }
            let numeric_url=env::var("SENSIBLAW_DATABASE_URL")
                .map_err(|_|"SENSIBLAW_DATABASE_URL required for explicit original numeric writes")?;
            if numeric_url.trim().is_empty(){
                return Err("empty original numeric database URL".into());
            }
            let root=env::var("SLR_NATIVE_NUMERIC_SOURCE_ROOT")
                .map(PathBuf::from)
                .map_err(|_|"SLR_NATIVE_NUMERIC_SOURCE_ROOT required for durable source artifacts")?;
            let (evidence,summary)=exact_plan(source_revision,parser_run)?;
            // A source-native parser run may cover an exact **bounded set**
            // of sentence regions without the whole document. Allow only an
            // explicit scoped compile; preserve open-world residuals and
            // prohibit any identity admission from that partial coverage.
            let partial_requested=matches!(args,
                [_,_,_,_,_,arg] if arg=="--allow-partial-source");
            if !evidence.full_document_coverage_paid && !partial_requested {
                return Err("original parser only covers a source slice; explicit --allow-partial-source required and global identity closure remains unpaid".into());
            }
            if summary.numeric_run_ref.is_empty()||summary.source_token_count==0{
                return Err("original source numeric plan is incomplete".into());
            }
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let receipt=run_native_numeric_exact_source_slice(
                &config,&numeric_url,source_revision,parser_run,&root,max_sentences
            ).map_err(|e|e.to_string())?;
            if receipt.numeric_run_ref!=summary.numeric_run_ref
                ||receipt.original_source_sha256!=summary.canonical_source_sha256 {
                return Err("native original numeric commit receipt disagrees with preflight plan".into());
            }
            print_json(&receipt)
        }
        _=>Err(
            "numeric-pnf-native usage: prefix-prepare REVIEW PARSER_RUN --inspect|--commit | endpoint REVIEW PARSER_RUN --commit | inspect SOURCE_REV PARSER_RUN | plan SOURCE_REV PARSER_RUN | audit S29_REVIEW_ITEM_REF PARSER_RUN | reuse-original S29_REVIEW_ITEM_REF PARSER_RUN | bind S29_REVIEW_ITEM_REF PARSER_RUN MAX_CANDIDATES --commit | bind-campaign S29_REVIEW_ITEM_REF PARSER_RUN MAX_CANDIDATES --commit | retract S29_REVIEW_ITEM_REF PARSER_RUN WITNESS_ID --rollback-probe|--commit | compile SOURCE_REV PARSER_RUN MAX_SENTENCES --commit [--allow-partial-source] (numeric DB URL required for audit/compile, durable source root for compile)".into()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_or_wrong_commit_command_never_runs() {
        assert!(run(&["compile".into()]).is_err());
        assert!(run(&["bind".into(),"a".into(),"b".into(),"65".into(),"--commit".into()]).is_err());
        assert!(run(&["bind-campaign".into(),"a".into(),"b".into(),"65".into(),"--commit".into()]).is_err());
        assert!(run(&["bind-campaign".into(),"a".into(),"b".into(),"1".into(),"--rollback-probe".into()]).is_err());
        assert!(run(&["bind".into(),"a".into(),"b".into(),"1".into(),"--dry-run".into()]).is_err());
        assert!(run(&["retract".into(),"a".into(),"b".into(),"1".into(),"--dry-run".into()]).is_err());
        assert!(run(&["retract".into(),"a".into(),"b".into(),"0".into(),"--commit".into()]).is_err());
        assert!(run(&["compile".into(),"a".into(),"b".into(),
            "65".into(),"--commit".into()]).is_err());
        assert!(run(&["compile".into(),"a".into(),"b".into(),
            "1".into(),"--dry-run".into()]).is_err());
    }
}
