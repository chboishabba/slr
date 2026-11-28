//! Native family/workbench operations over existing PG owners.
use sensiblaw_pg_source_store::{
    load_database_config,run_persisted_relational_family,inspect_persisted_relational_family,
    prepare_relational_family_revision,publish_relational_family_revision,inspect_relational_family_revision,
    apply_persisted_review_command,ReviewAction,ReviewCommand,load_rel_acceptance_corpus,
};
fn print_json<T:serde::Serialize>(value:&T)->Result<(),String>{
    println!("{}",serde_json::to_string_pretty(value).map_err(|e|e.to_string())?);Ok(())
}

/// Project the exact native family residuals into the existing proof-search
/// frontier. Acquisition completion is not semantic payment: it defers the
/// old residual for reviewed re-entry. No similarity score is a proof input.
fn family_research_frontier(family:&sensiblaw_pg_source_store::PersistedRelationalFamily)
    ->Result<serde_json::Value,String>{
    use sensiblaw_proof_search_loop::{frontier::{ProofFrontier,ProofResidual,ResidualStatus},
        hypothesis::families_for_frontier};
    use std::collections::BTreeSet;
    use sha2::{Digest,Sha256};
    use serde_json::json;
    let p=&family.projection;
    if !p.candidate_only || p.formal_consumer_equivalence_paid
        || p.all_live_reading_coverage_paid || p.creates_edit_authority
        || family.creates_access_authority || family.applies_external_edit {
        return Err("promotion-bearing family".into());
    }
    let mut frontier=ProofFrontier{consumer_ref:p.consumer_ref.clone(),
        frontier_ref:format!("frontier:relational:{}",family.family_ref),residuals:vec![],
        satisfied_payment_refs:vec![],contested_coordinate_refs:vec![],
        authority_blocked_refs:vec![],authority:"experimental_candidate_only"};
    let mut inverse=Vec::new();
    let mut seen=BTreeSet::new();
    for pair in &p.directed_comparisons {
        let c=&pair.comparison;
        if c.consumer_ref!=p.consumer_ref || c.creates_semantic_authority
            || c.claim_truth_promoted || c.merges_sources || c.proves_independence {
            return Err("wrong consumer or comparison promotion".into());
        }
        for (ordinal,r) in c.residuals.iter().enumerate(){
            let mut demands=p.acquisition_demands.iter().filter(|d|
                d.obligation.comparison_ref==c.comparison_ref &&
                d.obligation.residual_obligation_ref==r.obligation_ref &&
                d.exact_residuals.contains(r));
            let d=demands.next().ok_or("missing exact acquisition demand")?;
            if demands.next().is_some() || !d.obligation.candidate_only
                || d.obligation.creates_semantic_authority || d.obligation.claim_truth_promoted {
                return Err("ambiguous or promoted acquisition demand".into());
            }
            let mut lifecycles=family.acquisition_lifecycles.iter()
                .filter(|l|l.obligation_ref==d.obligation.obligation_ref);
            let l=lifecycles.next().ok_or("missing durable acquisition lifecycle")?;
            if lifecycles.next().is_some(){return Err("ambiguous lifecycle".into());}
            let encoded=serde_json::to_vec(&(&c.comparison_ref,ordinal,r))
                .map_err(|e|e.to_string())?;
            let residual_ref=format!("residual:relational:sha256:{:x}",Sha256::digest(encoded));
            if !seen.insert(residual_ref.clone()){return Err("duplicate exact residual".into());}
            let mut dependencies:BTreeSet<String>=d.obligation.source_revision_refs.iter().cloned().collect();
            dependencies.extend(d.obligation.dependency_target_refs.iter().cloned());
            dependencies.extend([c.comparison_ref.clone(),d.obligation.obligation_ref.clone(),
                d.obligation.authority_or_access_constraint_ref.clone()]);
            dependencies.extend(r.left_ref.iter().chain(r.right_ref.iter()).cloned());
            let deferred=family.superseded_by.is_some() || l.terminal_update.is_some();
            let status=if deferred{ResidualStatus::Underidentified}else{ResidualStatus::Open};
            frontier.residuals.push(ProofResidual{residual_ref:residual_ref.clone(),
                proposition_ref:format!("consumer-obligation:{}",r.obligation_ref),
                producer_class_ref:"producer:relational-discriminator".into(),
                jurisdiction_ref:None,authority_requirement_ref:None,salience:1,
                dependency_refs:dependencies.into_iter().collect(),status});
            inverse.push(json!({"residual_ref":residual_ref,"comparison_ref":c.comparison_ref,
                "ordinal":ordinal,"exact_residual":r,"obligation":d.obligation,
                "lifecycle":l,"requires_reviewed_reentry":deferred}));
        }
    }
    let residuals:Vec<_>=frontier.residuals.iter().map(|r|json!({
        "residual_ref":r.residual_ref,"proposition_ref":r.proposition_ref,
        "producer_class_ref":r.producer_class_ref,"dependency_refs":r.dependency_refs,
        "status":if r.status==ResidualStatus::Open{"open"}else{"underidentified"}
    })).collect();
    let hypotheses:Vec<_>=families_for_frontier(&frontier).iter().map(|h|json!({
        "hypothesis_ref":h.hypothesis_ref,"residual_ref":h.residual_ref,
        "target_proposition_ref":h.target_proposition_ref,"kind":format!("{:?}",h.kind),
        "candidate_only":true
    })).collect();
    Ok(json!({"schema":"slr.relational-research-frontier.v1","family_ref":family.family_ref,
        "consumer_ref":frontier.consumer_ref,"frontier_ref":frontier.frontier_ref,
        "superseded_by":family.superseded_by,"residuals":residuals,"inverse":inverse,
        "hypotheses":hypotheses,"satisfied_payment_refs":frontier.satisfied_payment_refs,
        "candidate_only":true,"formal_consumer_equivalence_paid":false,
        "world_truth_paid":false,"creates_access_authority":false,"writes_database":false,
        "planning_inputs_required":["consumer-owned lexical contexts","declared calibrated values","local index","research world snapshot"]}))
}

pub fn run(args:&[String])->Result<(),String>{
    // Local operator-only receipt inspection; no DB or authority access.
    if let [mode,receipt_path,consumer,required_json]=args{
        if mode=="inspect-indexed-inventory"{
            let raw=std::fs::read(receipt_path).map_err(|e|e.to_string())?;
            let receipt:serde_json::Value=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
            let required:Vec<String>=serde_json::from_str(required_json).map_err(|e|e.to_string())?;
            let out=sensiblaw_proof_search_loop::world_observation::inspect_indexed_inventory(
                &receipt,consumer,&required).map_err(|e|format!("{e:?}"))?;
            return print_json(&out);
        }
    }
    // Operator supplies an original JMD receipt; native chart is reopened
    // from persisted source owners, not supplied as arbitrary JSON.
    if let [mode,receipt_path,consumer,revision,run,doc,snapshot,digest,batches_json]=args{
        if mode=="inspect-indexed-native-meet"{
            let raw=std::fs::read(receipt_path).map_err(|e|e.to_string())?;
            let receipt:serde_json::Value=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
            if batches_json.len()>262144{return Err("batch reference request too large".into());}
            let batches:Vec<String>=serde_json::from_str(batches_json).map_err(|e|e.to_string())?;
            if batches.is_empty()||batches.len()>4096||batches.iter().any(|b|b.trim().is_empty()||b.len()>2048){
                return Err("invalid bounded batch references".into());
            }
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let native=sensiblaw_pg_source_store::read_article_numeric_predicate_transport(
                &config,revision,&batches,run,doc,snapshot,digest).map_err(|e|e.to_string())?;
            let out=sensiblaw_proof_search_loop::world_observation::inspect_indexed_native_predicate_meet(
                &receipt,&native,consumer).map_err(|e|format!("{e:?}"))?;
            return print_json(&out);
        }
    }
    // Local operator supplies indexed article receipt; reopen independent
    // native corpus predicate inventory through its existing immutable owner.
    if let [mode,receipt_path,consumer,run,doc,snapshot,digest]=args{
        if mode=="inspect-heterogeneous-overlap"{
            let raw=std::fs::read(receipt_path).map_err(|e|e.to_string())?;
            let receipt:serde_json::Value=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let native=sensiblaw_pg_source_store::query_native_original_relations(
                config.database_url(),run,doc,snapshot,digest,"predicate").map_err(|e|e.to_string())?;
            let out=sensiblaw_proof_search_loop::world_observation::inspect_heterogeneous_predicate_overlap(
                &receipt,&native,consumer).map_err(|e|format!("{e:?}"))?;
            return print_json(&out);
        }
    }
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    match args {
        [mode,revision,batches_path,run,doc,snapshot,digest,observation,consumer,query,evidence,repo,sha,path,thread] if mode=="licensed-semantic"=>{
            let raw=std::fs::read_to_string(batches_path).map_err(|e|e.to_string())?;
            let batches:Vec<String>=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
            let locator=sensiblaw_pg_source_store::RefereeReceiptLocator{
                repo:repo.clone(),sha:sha.clone(),path:path.clone(),canonical_thread_id:thread.clone(),
            };
            print_json(&sensiblaw_pg_source_store::read_authenticated_source_semantic_query(
                &config,revision,&batches,run,doc,snapshot,digest,observation,consumer,query,evidence,&locator)
                .map_err(|e|e.to_string())?)
        },
        [mode,revision,run,doc,snapshot,digest,batches_json] if mode=="inspect-source-fibre"=>{
            // Inline bounded list of persisted batch references, not a caller
            // supplied local filesystem path. Reuses the existing native owners.
            if batches_json.len()>262144{return Err("batch reference request too large".into());}
            let batches:Vec<String>=serde_json::from_str(batches_json).map_err(|e|e.to_string())?;
            if batches.is_empty()||batches.len()>4096||batches.iter().any(|b|b.trim().is_empty()||b.len()>2048){
                return Err("invalid bounded batch references".into());
            }
            print_json(&sensiblaw_pg_source_store::read_article_semantic_candidates(
                &config,revision,&batches,run,doc,snapshot,digest).map_err(|e|e.to_string())?)
        },
        [mode,revision,batches_path,run,doc,snapshot,digest] if mode=="predicate-transport"||mode=="semantic-candidates"=>{
            let raw=std::fs::read_to_string(batches_path).map_err(|e|e.to_string())?;
            let batches:Vec<String>=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
            let answer=if mode=="predicate-transport"{
                sensiblaw_pg_source_store::read_article_numeric_predicate_transport(
                    &config,revision,&batches,run,doc,snapshot,digest)
            }else{
                sensiblaw_pg_source_store::read_article_semantic_candidates(
                    &config,revision,&batches,run,doc,snapshot,digest)
            }.map_err(|e|e.to_string())?;
            print_json(&answer)
        },
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
        [mode,family] if mode=="inspect-research-frontier"=>{
            let reopened=inspect_persisted_relational_family(&config,family).map_err(|e|e.to_string())?;
            print_json(&family_research_frontier(&reopened)?)
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
        _=>Err("usage: relational-family inspect-heterogeneous-overlap RECEIPT CONSUMER RUN DOC SNAPSHOT DIGEST | inspect-indexed-native-meet RECEIPT CONSUMER REVISION RUN DOC SNAPSHOT DIGEST BATCH_REFS_JSON | inspect-indexed-inventory RECEIPT CONSUMER REQUIRED_LANES_JSON | CONSUMER ACCESS SELECTOR SELECTOR ... | licensed-semantic REVISION BATCH_REFS_JSON RUN DOC SNAPSHOT DIGEST OBSERVATION CONSUMER QUERY EVIDENCE REPO SHA PATH THREAD | predicate-transport REVISION BATCH_REFS_JSON RUN DOC SNAPSHOT DIGEST | semantic-candidates REVISION BATCH_REFS_JSON RUN DOC SNAPSHOT DIGEST | coordinate-meet REVISION BATCH_REFS_JSON RUN DOC SNAPSHOT DIGEST | numeric-action-roles RUN DOC SNAPSHOT DIGEST MENTION_TOKEN_IDS_JSON | article-actions REVISION BATCH_REFS_JSON MENTION_REFS_JSON | licensed-reciprocal FAMILY LEFT RIGHT QUERY EVIDENCE REPO SHA PATH THREAD | reciprocal FAMILY LEFT RIGHT | participant FAMILY FILLER_REF | inspect-research-frontier FAMILY | inspect FAMILY | prepare-revision FAMILY OLD_SELECTOR NEW_SELECTOR | inspect-revision TRANSITION | review-revision TRANSITION COMMAND REVIEWER accept|reject|abstain | publish-revision TRANSITION".into())
    }
}
