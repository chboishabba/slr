//! C4.11 CLI: prepare SLR S29 governed review from an exact C4.10 receipt.
//! Does not adjudicate a coreference or attach Wikidata identities.
use sensiblaw_pg_source_store::{
    load_database_config, prepare_native_antecedent_review,
    NativeAntecedentSourceCandidate,
};
use std::fs;

fn source_local_answer(
    review_item_ref: &str,
    parser_run_ref: Option<&str>,
) -> Result<sensiblaw_pg_source_store::NativeSourceLocalAnswer, String> {
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    let run=match parser_run_ref {
        Some(run) if !run.trim().is_empty() => run.to_owned(),
        Some(_) => return Err("empty original parser run".into()),
        None => {
            let runs=sensiblaw_pg_source_store::native_antecedent_parser_runs(
                &config,review_item_ref,
            ).map_err(|e|e.to_string())?;
            let full=runs.iter().filter(|x|x.structurally_eligible_for_frontier)
                .collect::<Vec<_>>();
            if full.len()!=1 {
                return Err(format!(
                    "source-local consumer requires exactly one complete original parser run; found {}",
                    full.len()
                ));
            }
            full[0].parser_run_ref.clone()
        }
    };
    let review=sensiblaw_pg_source_store::inspect_native_antecedent_review(
        &config,review_item_ref,
    ).map_err(|e|e.to_string())?;
    let frontier=sensiblaw_pg_source_store::native_antecedent_frontier(
        &config,review_item_ref,&run,
    ).map_err(|e|e.to_string())?;
    sensiblaw_pg_source_store::compile_native_source_local_answer(
        &review,&frontier,
    ).map_err(|e|e.to_string())
}

pub fn run(args: &[String]) -> Result<(), String> {
    match args {
        [subcommand, candidate_path] if subcommand == "prepare" => {
            let raw=fs::read_to_string(candidate_path)
                .map_err(|e|format!("read exact original C4.10 receipt: {e}"))?;
            let candidate:NativeAntecedentSourceCandidate=serde_json::from_str(&raw)
                .map_err(|e|format!("parse original C4.10 source-bound receipt: {e}"))?;
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let proposal=prepare_native_antecedent_review(&config,&candidate)
                .map_err(|e|e.to_string())?;
            println!("{}",serde_json::json!({
                "schema":"slr-native-antecedent-s29-review-queue-receipt-v1",
                "proposal_ref":proposal.proposal_ref,
                "review_item_ref":proposal.review_item_ref,
                "source_revision_ref":proposal.source_revision_ref,
                "focus_statement_ref":proposal.focus_statement_ref,
                "candidate_statement_ref":proposal.candidate_statement_ref,
                "s29_review_status":format!("{:?}",proposal.review_status),
                "antecedent_resolution":proposal.antecedent_resolution,
                "identity_admission_paid":proposal.identity_admission_paid,
                "external_qid_paid":proposal.external_qid_paid,
                "review_required":true,
                "candidate_only":true
            }));
            Ok(())
        }
        [subcommand, review_item_ref] if subcommand == "runs" => {
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let runs=sensiblaw_pg_source_store::native_antecedent_parser_runs(
                &config,review_item_ref,
            ).map_err(|e|e.to_string())?;
            println!("{}",serde_json::to_string_pretty(&runs)
                .map_err(|e|e.to_string())?);
            Ok(())
        }
        [subcommand, review_item_ref] if subcommand == "frontier" => {
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let runs=sensiblaw_pg_source_store::native_antecedent_parser_runs(
                &config,review_item_ref,
            ).map_err(|e|e.to_string())?;
            let complete=runs.iter().filter(|r|
                r.structurally_eligible_for_frontier).collect::<Vec<_>>();
            if complete.len()!=1 {
                return Err(format!("original parser run ambiguous or incomplete: {} eligible; use 'runs' and pass an explicit PARSER_RUN_REF",complete.len()));
            }
            let frontier=sensiblaw_pg_source_store::native_antecedent_frontier(
                &config,review_item_ref,&complete[0].parser_run_ref,
            ).map_err(|e|e.to_string())?;
            println!("{}",serde_json::to_string_pretty(&frontier)
                .map_err(|e|e.to_string())?);
            Ok(())
        }
        [subcommand, review_item_ref, parser_run_ref] if subcommand == "frontier" => {
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let frontier=sensiblaw_pg_source_store::native_antecedent_frontier(
                &config,review_item_ref,parser_run_ref,
            ).map_err(|e|e.to_string())?;
            println!("{}",serde_json::to_string_pretty(&frontier)
                .map_err(|e|e.to_string())?);
            Ok(())
        }
        [subcommand, review_item_ref] if subcommand == "derive" => {
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let runs=sensiblaw_pg_source_store::native_antecedent_parser_runs(
                &config,review_item_ref,
            ).map_err(|e|e.to_string())?;
            let complete=runs.iter().filter(|r|
                r.structurally_eligible_for_frontier).collect::<Vec<_>>();
            if complete.len()!=1 {
                return Err(format!(
                    "native source requires exactly one complete original parser run, got {} (inspect 'runs', or provide explicit PARSER_RUN_REF)",
                    complete.len()
                ));
            }
            let review=sensiblaw_pg_source_store::inspect_native_antecedent_review(
                &config,review_item_ref,
            ).map_err(|e|e.to_string())?;
            let frontier=sensiblaw_pg_source_store::native_antecedent_frontier(
                &config,review_item_ref,&complete[0].parser_run_ref,
            ).map_err(|e|e.to_string())?;
            let answer=sensiblaw_pg_source_store::derive_native_named_topic(
                &review,&frontier,
            ).map_err(|e|e.to_string())?;
            println!("{}",serde_json::to_string_pretty(&answer)
                .map_err(|e|e.to_string())?);
            Ok(())
        }
        [subcommand, review_item_ref, parser_run_ref] if subcommand == "derive" => {
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let review=sensiblaw_pg_source_store::inspect_native_antecedent_review(
                &config,review_item_ref,
            ).map_err(|e|e.to_string())?;
            let frontier=sensiblaw_pg_source_store::native_antecedent_frontier(
                &config,review_item_ref,parser_run_ref,
            ).map_err(|e|e.to_string())?;
            let answer=sensiblaw_pg_source_store::derive_native_named_topic(
                &review,&frontier,
            ).map_err(|e|e.to_string())?;
            println!("{}",serde_json::to_string_pretty(&answer)
                .map_err(|e|e.to_string())?);
            Ok(())
        }
        [subcommand, review_item_ref] if subcommand == "answer" => {
            let answer=source_local_answer(review_item_ref,None)?;
            println!("{}",serde_json::to_string_pretty(&answer)
                .map_err(|e|e.to_string())?);
            Ok(())
        }
        [subcommand, review_item_ref, parser_run_ref] if subcommand == "answer" => {
            let answer=source_local_answer(review_item_ref,Some(parser_run_ref))?;
            println!("{}",serde_json::to_string_pretty(&answer)
                .map_err(|e|e.to_string())?);
            Ok(())
        }
        [subcommand, review_item_ref, prior_path] if subcommand == "replay" => {
            let raw=fs::read_to_string(prior_path).map_err(|e|e.to_string())?;
            let prior:serde_json::Value=serde_json::from_str(&raw)
                .map_err(|e|e.to_string())?;
            let answer=source_local_answer(review_item_ref,None)?;
            let comparison=sensiblaw_pg_source_store::evaluate_native_source_local_replay(
                &prior,answer
            );
            println!("{}",serde_json::to_string_pretty(&comparison)
                .map_err(|e|e.to_string())?);
            Ok(())
        }
        [subcommand, review_item_ref, prior_path, parser_run_ref]
            if subcommand == "replay" => {
            let raw=fs::read_to_string(prior_path).map_err(|e|e.to_string())?;
            let prior:serde_json::Value=serde_json::from_str(&raw)
                .map_err(|e|e.to_string())?;
            let answer=source_local_answer(review_item_ref,Some(parser_run_ref))?;
            let comparison=sensiblaw_pg_source_store::evaluate_native_source_local_replay(
                &prior,answer
            );
            println!("{}",serde_json::to_string_pretty(&comparison)
                .map_err(|e|e.to_string())?);
            Ok(())
        }
        [subcommand, review_item_ref] if subcommand == "inspect" => {
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let packet=sensiblaw_pg_source_store::inspect_native_antecedent_review(
                &config,review_item_ref,
            ).map_err(|e|e.to_string())?;
            let output=serde_json::to_string_pretty(&packet)
                .map_err(|e|e.to_string())?;
            println!("{output}");
            Ok(())
        }
        _ => Err(
            "usage: sensiblaw native-antecedent-review \
             <prepare ORIGINAL_C410_RECEIPT.json | inspect REVIEW_ITEM_REF | runs REVIEW_ITEM_REF | frontier REVIEW_ITEM_REF [PARSER_RUN_REF] | derive REVIEW_ITEM_REF [PARSER_RUN_REF] | answer REVIEW_ITEM_REF [PARSER_RUN_REF] | replay REVIEW_ITEM_REF ANSWER.json [PARSER_RUN_REF]>"
             .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    #[test]
    fn no_candidate_receipt_or_implicit_review() {
        assert!(run(&[]).is_err());
        assert!(run(&["review".into(),"unreviewed.json".into()]).is_err());
        assert!(run(&["inspect".into(),"".into(),"extra".into()]).is_err());
        assert!(run(&["frontier".into(),"only-one-argument".into()]).is_err());
        assert!(run(&["runs".into(),"bad".into(),"extra".into()]).is_err());
        assert!(run(&["derive".into(),"bad".into(),"extra".into(),"again".into()]).is_err());
        assert!(run(&["answer".into(),"bad".into(),"one".into(),"two".into()]).is_err());
        assert!(run(&["replay".into(),"bad".into(),"one".into(),"two".into(),"three".into()]).is_err());
    }
}
