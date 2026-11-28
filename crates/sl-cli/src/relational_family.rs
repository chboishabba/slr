//! Native family/workbench operations over existing PG owners.
use sensiblaw_pg_source_store::{
    load_database_config,run_persisted_relational_family,inspect_persisted_relational_family,
    prepare_relational_family_revision,publish_relational_family_revision,inspect_relational_family_revision,
    apply_persisted_review_command,ReviewAction,ReviewCommand,load_rel_acceptance_corpus,
};
fn print_json<T:serde::Serialize>(value:&T)->Result<(),String>{
    println!("{}",serde_json::to_string_pretty(value).map_err(|e|e.to_string())?);Ok(())
}
pub fn run(args:&[String])->Result<(),String>{
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    match args {
        [mode,revision,batches_path,run,doc,snapshot,digest] if mode=="coordinate-meet"=>{
            let raw=std::fs::read_to_string(batches_path).map_err(|e|e.to_string())?;
            let batches:Vec<String>=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
            print_json(&sensiblaw_pg_source_store::read_article_numeric_coordinate_meet(
                &config,revision,&batches,run,doc,snapshot,digest).map_err(|e|e.to_string())?)
        },
        [mode,run,doc,snapshot,digest,mentions_path] if mode=="numeric-action-roles"=>{
            let raw=std::fs::read_to_string(mentions_path).map_err(|e|e.to_string())?;
            let mentions:Vec<i64>=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
            print_json(&sensiblaw_pg_source_store::query_native_recorded_action_roles(
                config.database_url(),run,doc,snapshot,digest,&mentions).map_err(|e|e.to_string())?)
        },
        [mode,revision,batches_path,mentions_path] if mode=="article-actions"=>{
            let read = |path:&str| -> Result<Vec<String>,String> {
                let raw=std::fs::read_to_string(path).map_err(|e|e.to_string())?;
                serde_json::from_str(&raw).map_err(|e|e.to_string())
            };
            let batches=read(batches_path)?;
            let mentions=read(mentions_path)?;
            print_json(&sensiblaw_pg_source_store::read_recorded_article_actions(
                &config,revision,&batches,&mentions).map_err(|e|e.to_string())?)
        },
        [mode,corpus,access] if mode=="acceptance-corpus"=>{
            let suite=load_rel_acceptance_corpus(&config,corpus).map_err(|e|e.to_string())?;
            let mut receipts=Vec::new();
            for case in suite.cases{
                let left=case.left.selector_ref;
                let right=case.right.selector_ref;
                let family=run_persisted_relational_family(&config,&case.consumer.consumer_ref,access,&[left.clone(),right.clone()])
                    .map_err(|e|format!("{}: {e}",case.case_ref))?;
                let reopened=inspect_persisted_relational_family(&config,&family.family_ref).map_err(|e|e.to_string())?;
                if reopened!=family{return Err(format!("{}: exact family reopen mismatch",case.case_ref));}
                let actual=family.projection.directed_comparisons.iter().find(|p|p.left_selector_ref==left&&p.right_selector_ref==right)
                    .ok_or("original directed pair missing")?;
                // Old REL-1C expected labels are historical controls; strict
                // native projection may expose additional real residuals.
                let required=case.required_residual_kinds.iter().all(|k|actual.comparison.residuals.iter().any(|r|&r.kind==k));
                let forbidden=case.forbidden_residual_kinds.iter().any(|k|actual.comparison.residuals.iter().any(|r|&r.kind==k));
                receipts.push(serde_json::json!({"case_ref":case.case_ref,"purpose":case.purpose_ref,
                    "legacy_expected_finding":case.expected_finding,
                    "strict_actual_finding":actual.comparison.finding,
                    "legacy_required_residuals_present":required,"legacy_forbidden_residual_present":forbidden,
                    "family":family,"exact_native_family_reopen":true}));
            }
            if receipts.is_empty(){return Err("empty persisted acceptance corpus".into());}
            print_json(&serde_json::json!({"schema":"itir.native-relational-corpus.v1","corpus_ref":corpus,
                "cases":receipts,"semantic_coverage_paid":false}))
        },
        [mode,family,left,right,query,evidence,repo,sha,path,thread] if mode=="licensed-reciprocal"=>{
            let locator=sensiblaw_pg_source_store::RefereeReceiptLocator{
                repo:repo.clone(),sha:sha.clone(),path:path.clone(),canonical_thread_id:thread.clone(),
            };
            print_json(&sensiblaw_pg_source_store::read_authenticated_reciprocal_query(
                &config,family,left,right,query,evidence,&locator).map_err(|e|e.to_string())?)
        },
        [mode,family,left,right] if mode=="reciprocal"=>{
            let reopened=inspect_persisted_relational_family(&config,family).map_err(|e|e.to_string())?;
            print_json(&sensiblaw_pg_source_store::recorded_reciprocal_query(&reopened,left,right).map_err(|e|e.to_string())?)
        },
        [mode,family,filler] if mode=="participant"=>{
            if filler.trim().is_empty(){return Err("empty exact participant factor".into());}
            let reopened=inspect_persisted_relational_family(&config,family).map_err(|e|e.to_string())?;
            print_json(&sensiblaw_pg_source_store::recorded_participant_query(&reopened,filler))
        },
        [mode,family] if mode=="inspect"=>print_json(&inspect_persisted_relational_family(&config,family).map_err(|e|e.to_string())?),
        [mode,family,old,new] if mode=="prepare-revision"=>print_json(&prepare_relational_family_revision(&config,family,old,new).map_err(|e|e.to_string())?),
        [mode,transition] if mode=="inspect-revision"=>print_json(&inspect_relational_family_revision(&config,transition).map_err(|e|e.to_string())?),
        [mode,transition] if mode=="publish-revision"=>print_json(&publish_relational_family_revision(&config,transition).map_err(|e|e.to_string())?),
        [mode,transition,command,reviewer,action] if mode=="review-revision"=>{
            let t=inspect_relational_family_revision(&config,transition).map_err(|e|e.to_string())?;
            let action=match action.as_str(){"accept"=>ReviewAction::Accept,"reject"=>ReviewAction::Reject,
                "abstain"=>ReviewAction::Abstain,_=>return Err("invalid candidate review action".into())};
            let (receipt,item)=apply_persisted_review_command(&config,&ReviewCommand{
                command_ref:command.clone(),review_item_ref:t.review_item_ref,
                action,reviewer_ref:reviewer.clone(),qualification_ref:None,evidence_request_ref:None,
            }).map_err(|e|e.to_string())?;
            print_json(&serde_json::json!({"command_ref":receipt.command_ref,"review_item_ref":item.review_item_ref,
                "candidate_review_status":format!("{:?}",item.current_status),"creates_semantic_authority":false}))
        },
        _ if (4..=12).contains(&args.len())=>print_json(&run_persisted_relational_family(&config,&args[0],&args[1],&args[2..]).map_err(|e|e.to_string())?),
        _=>Err("usage: relational-family CONSUMER ACCESS SELECTOR SELECTOR ... | coordinate-meet REVISION BATCH_REFS_JSON RUN DOC SNAPSHOT DIGEST | numeric-action-roles RUN DOC SNAPSHOT DIGEST MENTION_TOKEN_IDS_JSON | article-actions REVISION BATCH_REFS_JSON MENTION_REFS_JSON | licensed-reciprocal FAMILY LEFT RIGHT QUERY EVIDENCE REPO SHA PATH THREAD | reciprocal FAMILY LEFT RIGHT | participant FAMILY FILLER_REF | inspect FAMILY | prepare-revision FAMILY OLD_SELECTOR NEW_SELECTOR | inspect-revision TRANSITION | review-revision TRANSITION COMMAND REVIEWER accept|reject|abstain | publish-revision TRANSITION".into())
    }
}
