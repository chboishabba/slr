//! Thin OALC provider adapter for the generic legal-source manifestation store.
//!
//! Usage:
//!   materialize_oalc_legal_source_manifestation \
//!     <source-revision-ref> <document-ref> <native-source-ref> \
//!     <native-revision-ref> <canonical-sha256-hex> <acquisition-receipt-ref>
//!
//! The command does not ingest semantic assertions or review proposition
//! support. It only proves that the declared OALC native identity/digest maps
//! to the exact canonical bytes of an existing persisted legal source revision.

use sensiblaw_pg_source_store::{
    load_database_config, persist_legal_source_manifestation,
    LegalSourceManifestationDraft,
};
use std::{env, process};

fn run() -> Result<(), String> {
    let args=env::args().skip(1).collect::<Vec<_>>();
    if args.len()!=6 {
        return Err("usage: materialize_oalc_legal_source_manifestation <source-revision-ref> <document-ref> <native-source-ref> <native-revision-ref> <canonical-sha256-hex> <acquisition-receipt-ref>".into());
    }
    let config=load_database_config(None).map_err(|error|error.to_string())?;
    let persisted=persist_legal_source_manifestation(&config,&LegalSourceManifestationDraft{
        source_revision_ref:args[0].clone(),document_ref:args[1].clone(),
        source_family_ref:"legal".into(),provider_ref:"oalc".into(),
        native_source_ref:args[2].clone(),native_revision_ref:args[3].clone(),
        expected_canonical_sha256_hex:args[4].clone(),acquisition_receipt_ref:args[5].clone(),
    }).map_err(|error|error.to_string())?;
    println!("manifestation_ref={}",persisted.manifestation_ref);
    println!("source_revision_ref={}",persisted.source_revision_ref);
    println!("document_ref={}",persisted.document_ref);
    println!("canonical_ref={}",persisted.canonical_ref);
    println!("canonical_sha256={}",persisted.canonical_sha256_hex);
    println!("provider_ref={}",persisted.provider_ref);
    println!("exact_native_source_reopened=true");
    println!("creates_semantic_authority=false");
    println!("creates_legal_authority=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
fn main(){if let Err(error)=run(){eprintln!("{error}");process::exit(1);}}
