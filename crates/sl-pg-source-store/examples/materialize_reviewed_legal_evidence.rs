//! Persist reviewed legal evidence and materialise legal IR from durable refs.
//!
//! This is an operator/review payment command, not an inference command.
//! Source/document/span/parser coordinates are reopened from the persisted
//! candidate batch. JSON is not accepted.
//!
//! Usage:
//!   materialize_reviewed_legal_evidence \
//!     <source-manifestation-ref> <review-receipt-ref> <observation-ref> \
//!     <consumer-ref> <requirement-ref> <evidence-role-ref> <normative-order-ref> \
//!     <proposition-ref> <candidate-batch-ref> <candidate-factor-ref> \
//!     <pnf-build-ref> <refined-pnf-graph-ref> <pnf-factor-ref> <pnf-revision-ref> \
//!     <structural-signature-ref> <predicate-ref> <legal-system-ref> \
//!     <jurisdiction-ref> <temporal-ref-or-dash> <author-ref> <institution-ref-or-dash>

use sensiblaw_pg_source_store::{
    load_database_config, materialize_reviewed_legal_evidence,
    resolve_reviewed_legal_evidence_request, ReviewedLegalEvidenceSelection,
};
use std::{env, process};

fn optional(value:&str)->Option<String>{if value=="-"{None}else{Some(value.to_owned())}}
fn optional_vec(value:&str)->Vec<String>{optional(value).into_iter().collect()}

fn run()->Result<(),String>{
    let a=env::args().skip(1).collect::<Vec<_>>();
    if a.len()!=21 {
        return Err("usage: materialize_reviewed_legal_evidence <source-manifestation-ref> <review-receipt-ref> <observation-ref> <consumer-ref> <requirement-ref> <evidence-role-ref> <normative-order-ref> <proposition-ref> <candidate-batch-ref> <candidate-factor-ref> <pnf-build-ref> <refined-pnf-graph-ref> <pnf-factor-ref> <pnf-revision-ref> <structural-signature-ref> <predicate-ref> <legal-system-ref> <jurisdiction-ref> <temporal-ref-or-dash> <author-ref> <institution-ref-or-dash>".into());
    }
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    let request=resolve_reviewed_legal_evidence_request(&config,&ReviewedLegalEvidenceSelection{
        source_manifestation_ref:a[0].clone(),review_receipt_ref:a[1].clone(),observation_ref:a[2].clone(),
        consumer_ref:a[3].clone(),requirement_ref:a[4].clone(),evidence_role_ref:a[5].clone(),
        normative_order_ref:a[6].clone(),proposition_ref:a[7].clone(),candidate_pnf_batch_ref:a[8].clone(),
        candidate_factor_ref:a[9].clone(),pnf_build_ref:a[10].clone(),refined_pnf_graph_ref:a[11].clone(),
        pnf_factor_ref:a[12].clone(),pnf_revision_ref:a[13].clone(),structural_signature_ref:a[14].clone(),
        predicate_ref:a[15].clone(),legal_system_refs:vec![a[16].clone()],jurisdiction_refs:vec![a[17].clone()],
        temporal_refs:optional_vec(&a[18]),author_ref:a[19].clone(),institution_ref:optional(&a[20]),
    }).map_err(|e|e.to_string())?;
    let result=materialize_reviewed_legal_evidence(&config,&request).map_err(|e|e.to_string())?;
    println!("source_manifestation_ref={}",result.source_manifestation.manifestation_ref);
    println!("reviewed_evidence_ref={}",result.reviewed_evidence.reviewed_evidence_ref);
    println!("normative_order_ref={}",result.reviewed_evidence.normative_order_ref);
    println!("legal_ir_projection_ref={}",result.legal_ir.projection_ref);
    println!("legal_ir_observation_ref={}",result.legal_ir.observation_ref);
    println!("exact_native_source_reopened={}",result.exact_native_source_reopened);
    println!("review_role_explicit={}",result.review_role_explicit);
    println!("normative_order_explicit={}",result.normative_order_explicit);
    println!("creates_semantic_authority=false");
    println!("creates_legal_authority=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");process::exit(1);}}
