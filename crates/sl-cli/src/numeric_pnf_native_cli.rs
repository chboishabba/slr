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
    run_native_numeric_exact_source_slice, OriginalNativeNumericEvidence,
    inspect_native_antecedent_review, native_antecedent_frontier,
    compile_native_source_local_answer,
    audit_original_numeric_identity_for_source_answer,
    retract_original_numeric_identity_for_source_answer,
    transport_original_numeric_candidates_for_source_answer,
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
            if (subcommand=="audit" && args.len()==3)
                || (subcommand=="retract" && args.len()==5)
                || (subcommand=="bind" && args.len()==5) => {
            let transport_budget=if subcommand=="bind"{
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
            if let Some(budget)=transport_budget {
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
            "numeric-pnf-native usage: inspect SOURCE_REV PARSER_RUN | plan SOURCE_REV PARSER_RUN | audit S29_REVIEW_ITEM_REF PARSER_RUN | bind S29_REVIEW_ITEM_REF PARSER_RUN MAX_CANDIDATES --commit | retract S29_REVIEW_ITEM_REF PARSER_RUN WITNESS_ID --rollback-probe|--commit | compile SOURCE_REV PARSER_RUN MAX_SENTENCES --commit [--allow-partial-source] (numeric DB URL required for audit/compile, durable source root for compile)".into()
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
        assert!(run(&["bind".into(),"a".into(),"b".into(),"1".into(),"--dry-run".into()]).is_err());
        assert!(run(&["retract".into(),"a".into(),"b".into(),"1".into(),"--dry-run".into()]).is_err());
        assert!(run(&["retract".into(),"a".into(),"b".into(),"0".into(),"--commit".into()]).is_err());
        assert!(run(&["compile".into(),"a".into(),"b".into(),
            "65".into(),"--commit".into()]).is_err());
        assert!(run(&["compile".into(),"a".into(),"b".into(),
            "1".into(),"--dry-run".into()]).is_err());
    }
}
