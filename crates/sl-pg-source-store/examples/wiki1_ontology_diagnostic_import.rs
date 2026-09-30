//! Import JMD attributed, separately executed Lean diagnostic packets
//! into the existing SLR PG + S29 review store. Source revision MUST exist.
//! No public Wikidata edit/repair action is performed.
use std::{fs,io::{self,BufRead}};
use sensiblaw_pg_source_store::{
    load_database_config, persist_ontology_diagnostic,
    OntologyDiagnosticPacket,
};

fn main()->Result<(),String> {
    let mut args=std::env::args().skip(1);
    let path=args.next().ok_or_else(||
        "usage: wiki1_ontology_diagnostic_import <packet.jsonl> <consumer-scope-ref>".to_owned())?;
    let scope=args.next().ok_or_else(||
        "missing consumer-scope-ref".to_owned())?;
    if args.next().is_some() || scope.trim().is_empty() {
        return Err("unexpected arguments or missing scope".into());
    }
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    let content=fs::read(&path).map_err(|e|e.to_string())?;
    let cursor=io::Cursor::new(content);
    let mut count=0usize;
    for (line_number,line) in cursor.lines().enumerate() {
        let line=line.map_err(|e|e.to_string())?;
        if line.trim().is_empty() {continue;}
        let packet:OntologyDiagnosticPacket=serde_json::from_str(&line)
            .map_err(|e|format!("line {}: {e}",line_number+1))?;
        let receipt=persist_ontology_diagnostic(&config,&packet,&scope)
            .map_err(|e|format!("line {}: {e}",line_number+1))?;
        println!(
            "diagnostic_ref={} review_item_ref={} graph_view={:?}              review_status={:?} edit_authority=false",
            receipt.diagnostic_ref,receipt.review_item_ref,
            receipt.packet.graph_view,receipt.s29_status
        );
        count+=1;
    }
    println!("source_pinned_diagnostics_persisted={count}");
    println!("semantic_authority=false");
    println!("wikidata_edit_authority=false");
    Ok(())
}
