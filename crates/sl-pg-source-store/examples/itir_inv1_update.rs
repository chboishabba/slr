//! ITIR-INV-1 acquisition-result recorder.
//!
//! This executable does not fetch evidence. It records the result of a
//! separately completed lawful acquisition *after* the acquired source has
//! already been ingested through its native adapter, then computes selective
//! reopening over an explicit dependency graph.

use serde::Deserialize;
use sensiblaw_pg_source_store::{
    load_database_config,persist_acquisition_update,AcquisitionUpdate,
};
use std::{env,fs,process};

#[derive(Debug,Deserialize)]
struct Request{
    schema:String,
    update:AcquisitionUpdate,
    dependency_graph_ref:String,
    dependency_edges:Vec<(String,String)>,
    dependency_universe_refs:Vec<String>,
}
fn run()->Result<(),String>{
    let path=env::args().nth(1)
        .ok_or_else(||"usage: itir_inv1_update <acquisition-result.json>".to_owned())?;
    let request:Request=serde_json::from_slice(&fs::read(path)
        .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    if request.schema!="itir.inv1.acquisition-result.v1"
        ||request.dependency_graph_ref.trim().is_empty(){
        return Err("unsupported/invalid acquisition result packet".into());
    }
    let cfg=load_database_config(None).map_err(|e|e.to_string())?;
    let reopening=persist_acquisition_update(
        &cfg,&request.update,&request.dependency_graph_ref,
        &request.dependency_edges,&request.dependency_universe_refs,
    ).map_err(|e|e.to_string())?;
    match reopening{
        Some(receipt)=>{
            println!("{}",serde_json::to_string_pretty(&receipt)
                .map_err(|e|e.to_string())?);
            println!("selective_reopening_created=true");
        },
        None=>{
            println!("selective_reopening_created=false");
            println!("known_absence_closes_exact_acquisition_branch_only=true");
        }
    }
    println!("source_acquired_by_this_program=false");
    println!("semantic_authority_created=false");
    Ok(())
}
fn main(){
    if let Err(e)=run(){eprintln!("{e}");process::exit(1);}
}
