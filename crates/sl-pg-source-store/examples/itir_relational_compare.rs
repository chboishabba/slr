//! ITIR structural interlingua specimen runner.
//! Accepts *already-produced* PNF candidate observations from arbitrary
//! source families; no parser, ontology heuristic or truth promotion.
use std::{env,fs};
use serde::Deserialize;
use sensiblaw_pg_source_store::{
    RelationalObservation,RelationalConsumer,
    compare_relational_observations,
};
#[derive(Deserialize)]
struct ComparisonRequest {
    left:RelationalObservation,
    right:RelationalObservation,
    consumer:RelationalConsumer,
}
fn main()->Result<(),String>{
    let path=env::args().nth(1)
        .ok_or_else(||"usage: itir_relational_compare <typed-pnf-pair.json>".to_owned())?;
    let raw=fs::read(&path).map_err(|e|e.to_string())?;
    let req:ComparisonRequest=serde_json::from_slice(&raw)
        .map_err(|e|e.to_string())?;
    let output=compare_relational_observations(
        &req.left,&req.right,&req.consumer,
    ).map_err(|e|e.to_string())?;
    println!("{}",serde_json::to_string_pretty(&output)
        .map_err(|e|e.to_string())?);
    Ok(())
}
