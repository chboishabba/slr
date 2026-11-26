//! Native family/workbench operations over existing PG owners.
use sensiblaw_pg_source_store::{
    load_database_config,run_persisted_relational_family,inspect_persisted_relational_family,
    prepare_relational_family_revision,publish_relational_family_revision,inspect_relational_family_revision,
    apply_persisted_review_command,ReviewAction,ReviewCommand,
};
fn print_json<T:serde::Serialize>(value:&T)->Result<(),String>{
    println!("{}",serde_json::to_string_pretty(value).map_err(|e|e.to_string())?);Ok(())
}
pub fn run(args:&[String])->Result<(),String>{
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    match args {
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
        _=>Err("usage: relational-family CONSUMER ACCESS SELECTOR SELECTOR ... | inspect FAMILY | prepare-revision FAMILY OLD_SELECTOR NEW_SELECTOR | inspect-revision TRANSITION | review-revision TRANSITION COMMAND REVIEWER accept|reject|abstain | publish-revision TRANSITION".into())
    }
}
