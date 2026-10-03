//! Bind an existing persisted derived/challengeable proof graph to INV.
//!
//! The executable creates no graph nodes or edges. It only records an
//! immutable, evidence-backed projection binding after both the INV obligation
//! and the legal-follow projection already exist.

use serde::Deserialize;
use sensiblaw_pg_source_store::{
    build_investigation_graph_binding, load_database_config,
    load_investigation_graph_binding, persist_investigation_graph_binding,
};
use std::{env, fs, process};

#[derive(Debug, Deserialize)]
struct Request {
    schema: String,
    binding_ref: String,
    obligation_ref: String,
    projection_ref: String,
    matter_ref: String,
    binding_evidence_refs: Vec<String>,
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: itir_inv1_bind_graph <graph-binding-request.json>".to_owned())?;
    let request: Request = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if request.schema != "itir.inv1.graph-binding-request.v1" {
        return Err("unsupported graph-binding request schema".into());
    }

    let binding = build_investigation_graph_binding(
        &request.binding_ref,
        &request.obligation_ref,
        &request.projection_ref,
        &request.matter_ref,
        request.binding_evidence_refs,
    )
    .map_err(|e| e.to_string())?;

    let config = load_database_config(None).map_err(|e| e.to_string())?;
    let persisted = persist_investigation_graph_binding(&config, &binding)
        .map_err(|e| e.to_string())?;
    let reopened = load_investigation_graph_binding(&config, &persisted.binding_ref)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "persisted graph binding did not reopen".to_owned())?;
    if reopened != persisted {
        return Err("persisted graph binding changed on reopen".into());
    }

    println!("{}", serde_json::to_string_pretty(&persisted).map_err(|e| e.to_string())?);
    println!("graph_created_by_this_program=false");
    println!("graph_edges_created_by_this_program=false");
    println!("semantic_authority_created=false");
    println!("access_authority_created=false");
    println!("acquisition_state_created=false");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
