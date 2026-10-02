//! ITIR-INV-1 acquisition queue builder over a persisted REL comparison.
//!
//! Input is a reviewed candidate-acquisition request, not a search instruction.
//! The program does not perform network access or acquire evidence. It binds
//! one persisted REL residual to lawful route candidates, computes the
//! non-scalar Pareto frontier, persists it, and reopens the exact queue.

use serde::Deserialize;
use sensiblaw_pg_source_store::{
    load_database_config,load_relational_comparison,
    obligation_from_residual,persist_acquisition_queue,
    AcquisitionRouteCandidate,
};
use std::{env,fs,process};

#[derive(Debug,Deserialize)]
struct Request{
    schema:String,
    comparison_ref:String,
    residual_obligation_ref:String,
    target_description:String,
    authority_or_access_constraint_ref:String,
    dependency_target_refs:Vec<String>,
    routes:Vec<AcquisitionRouteCandidate>,
}
fn run()->Result<(),String>{
    let path=env::args().nth(1)
        .ok_or_else(||"usage: itir_inv1_queue <acquisition-request.json>".to_owned())?;
    let request:Request=serde_json::from_slice(&fs::read(path)
        .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    if request.schema!="itir.inv1.acquisition-request.v1"{
        return Err("unsupported acquisition request schema".into());
    }
    let cfg=load_database_config(None).map_err(|e|e.to_string())?;
    let comparison=load_relational_comparison(&cfg,&request.comparison_ref)
        .map_err(|e|e.to_string())?
        .ok_or_else(||"unknown persisted REL comparison".to_owned())?;
    let residual=comparison.comparison.residuals.iter()
        .find(|r|r.obligation_ref==request.residual_obligation_ref)
        .ok_or_else(||"requested acquisition obligation is not a residual of the selected comparison".to_owned())?;
    let obligation=obligation_from_residual(
        &comparison.comparison,residual,
        vec![
            comparison.left.source_revision_ref.clone(),
            comparison.right.source_revision_ref.clone(),
        ],
        &request.target_description,
        &request.authority_or_access_constraint_ref,
        request.dependency_target_refs,
    ).map_err(|e|e.to_string())?;
    let queue=persist_acquisition_queue(&cfg,&obligation,&request.routes)
        .map_err(|e|e.to_string())?;
    println!("{}",serde_json::to_string_pretty(&queue)
        .map_err(|e|e.to_string())?);
    println!("acquisition_executed=false");
    println!("source_truth_promoted=false");
    println!("scalar_score_used=false");
    Ok(())
}
fn main(){
    if let Err(e)=run(){eprintln!("{e}");process::exit(1);}
}
