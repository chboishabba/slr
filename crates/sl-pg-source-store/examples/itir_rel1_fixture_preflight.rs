//! ITIR-REL-1B fixture acquisition/preflight.
//!
//! This utility does NOT create fixtures. It checks an independently prepared
//! acquisition manifest against the live authority database and reports the
//! exact missing source revisions / canonical hashes / provenance coordinates
//! needed before the integrated acceptance runner can honestly execute.

use postgres::{Client,NoTls};
use serde::{Deserialize,Serialize};
use sensiblaw_pg_source_store::load_database_config;
use sha2::{Digest,Sha256};
use std::{env,fs,process};

#[derive(Debug,Deserialize)]
struct Manifest {
    schema:String,
    fixture_set_ref:String,
    cases:Vec<FixtureCase>,
}
#[derive(Debug,Deserialize)]
struct FixtureCase {
    case_ref:String,
    purpose:FixturePurpose,
    left:SourceRequirement,
    right:SourceRequirement,
    consumer_ref:String,
    alignment_licence_refs:Vec<String>,
    producer_receipt_refs:Vec<String>,
    negative_control:bool,
}
#[derive(Debug,Clone,Copy,Deserialize,Serialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
enum FixturePurpose {
    WikidataText,
    BiomedicalCrossSpecies,
    MultilingualWikipedia,
}
#[derive(Debug,Deserialize)]
struct SourceRequirement {
    source_revision_ref:String,
    expected_sha256:String,
    expected_native_family:String,
    acquisition_receipt_ref:String,
    parser_or_adapter_receipt_ref:String,
    revision_locator_ref:String,
}
#[derive(Debug,Serialize)]
struct SourceCheck {
    source_revision_ref:String,
    expected_sha256:String,
    observed_sha256:Option<String>,
    expected_native_family:String,
    observed_native_family:Option<String>,
    acquisition_receipt_matches:Option<bool>,
    found:bool,
    digest_matches:bool,
    family_matches:bool,
    native_store:String,
    missing_coordinates:Vec<String>,
}
#[derive(Debug,Serialize)]
struct CaseCheck {
    case_ref:String,
    purpose:FixturePurpose,
    left:SourceCheck,
    right:SourceCheck,
    consumer_ref:String,
    alignment_licence_refs_present:bool,
    producer_receipt_refs_present:bool,
    negative_control:bool,
    ready:bool,
}
#[derive(Debug,Serialize)]
struct PreflightReceipt {
    schema:&'static str,
    fixture_set_ref:String,
    cases:Vec<CaseCheck>,
    ready_case_count:usize,
    missing_case_count:usize,
    has_wikidata_text:bool,
    has_biomedical_cross_species:bool,
    has_multilingual_wikipedia:bool,
    has_negative_control:bool,
    acceptance_ready:bool,
    creates_semantic_authority:bool,
    synthesizes_missing_evidence:bool,
}

fn valid(s:&str)->bool{!s.trim().is_empty()}
fn digest(bytes:&[u8])->String{
    format!("sha256:{:x}",Sha256::digest(bytes))
}
fn source_bytes(client:&mut Client,revision:&str)
    ->Result<Option<(Vec<u8>,String,Option<String>,Option<String>)>,postgres::Error>{
    if let Some(row)=client.query_opt(
        "SELECT c.payload,r.source_family_ref,r.acquisition_receipt_ref
         FROM ingest.generic_source_revision r
         JOIN corpus.document d ON d.document_ref=r.document_ref
         JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
         WHERE r.source_revision_ref=$1
           AND r.candidate_only=TRUE
           AND r.creates_semantic_authority=FALSE
           AND r.applicability_promoted=FALSE
           AND r.claim_truth_promoted=FALSE",
        &[&revision],
    )?{
        return Ok(Some((
            row.get::<_,Vec<u8>>(0),"generic_source".into(),
            Some(row.get::<_,String>(1)),Some(row.get::<_,String>(2)),
        )));
    }
    if let Some(row)=client.query_opt(
        "SELECT c.payload
         FROM corpus.chat_archive_message m
         JOIN corpus.document d ON d.document_ref=m.document_ref
         JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
         WHERE m.source_revision_ref=$1
           AND m.candidate_only=TRUE
           AND m.creates_semantic_authority=FALSE
           AND m.applicability_promoted=FALSE
           AND m.claim_truth_promoted=FALSE
         LIMIT 1",
        &[&revision],
    )?{
        return Ok(Some((
            row.get::<_,Vec<u8>>(0),"chat_archive".into(),
            Some("chat".into()),None,
        )));
    }
    Ok(None)
}
fn check_source(client:&mut Client,r:&SourceRequirement)
    ->Result<SourceCheck,String>{
    let mut missing=Vec::new();
    for (name,value) in [
        ("source_revision_ref",&r.source_revision_ref),
        ("expected_sha256",&r.expected_sha256),
        ("expected_native_family",&r.expected_native_family),
        ("acquisition_receipt_ref",&r.acquisition_receipt_ref),
        ("parser_or_adapter_receipt_ref",&r.parser_or_adapter_receipt_ref),
        ("revision_locator_ref",&r.revision_locator_ref),
    ]{
        if !valid(value){missing.push(name.into());}
    }
    let found=source_bytes(client,&r.source_revision_ref).map_err(|e|e.to_string())?;
    let (observed,native_store,observed_family,stored_acquisition)=match found{
        Some((bytes,store,family,acquisition))=>
            (Some(digest(&bytes)),store,family,acquisition),
        None=>(None,"missing".into(),None,None),
    };
    let digest_matches=observed.as_deref()==Some(r.expected_sha256.as_str());
    let family_matches=observed_family.as_deref()==Some(r.expected_native_family.as_str());
    let acquisition_receipt_matches=stored_acquisition.as_ref()
        .map(|stored|stored==&r.acquisition_receipt_ref);
    Ok(SourceCheck{
        source_revision_ref:r.source_revision_ref.clone(),
        expected_sha256:r.expected_sha256.clone(),
        observed_sha256:observed,
        expected_native_family:r.expected_native_family.clone(),
        observed_native_family:observed_family,
        acquisition_receipt_matches,
        found:native_store!="missing",digest_matches,family_matches,native_store,
        missing_coordinates:missing,
    })
}
fn run()->Result<(),String>{
    let path=env::args().nth(1).ok_or_else(||
        "usage: itir_rel1_fixture_preflight <acquisition-manifest.json>".to_owned())?;
    let manifest:Manifest=serde_json::from_slice(&fs::read(path)
        .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    if manifest.schema!="itir.rel1.fixture-acquisition.v1"
        ||!valid(&manifest.fixture_set_ref)||manifest.cases.is_empty(){
        return Err("invalid REL-1 fixture acquisition manifest".into());
    }
    let cfg=load_database_config(None).map_err(|e|e.to_string())?;
    let mut client=Client::connect(cfg.database_url(),NoTls)
        .map_err(|e|e.to_string())?;
    let mut cases=Vec::new();
    for case in manifest.cases{
        let left=check_source(&mut client,&case.left)?;
        let right=check_source(&mut client,&case.right)?;
        let licences=!case.alignment_licence_refs.is_empty()
            &&case.alignment_licence_refs.iter().all(|x|valid(x));
        let producers=!case.producer_receipt_refs.is_empty()
            &&case.producer_receipt_refs.iter().all(|x|valid(x));
        let ready=left.found&&right.found&&left.digest_matches&&right.digest_matches
            &&left.family_matches&&right.family_matches
            &&left.acquisition_receipt_matches.unwrap_or(true)
            &&right.acquisition_receipt_matches.unwrap_or(true)
            &&left.missing_coordinates.is_empty()&&right.missing_coordinates.is_empty()
            &&valid(&case.consumer_ref)&&licences&&producers;
        cases.push(CaseCheck{
            case_ref:case.case_ref,purpose:case.purpose,left,right,
            consumer_ref:case.consumer_ref,
            alignment_licence_refs_present:licences,
            producer_receipt_refs_present:producers,
            negative_control:case.negative_control,ready,
        });
    }
    let ready=cases.iter().filter(|c|c.ready).count();
    let receipt=PreflightReceipt{
        schema:"itir.rel1.fixture-preflight.v1",
        fixture_set_ref:manifest.fixture_set_ref.clone(),
        ready_case_count:ready,missing_case_count:cases.len()-ready,
        has_wikidata_text:cases.iter().any(|c|c.purpose==FixturePurpose::WikidataText),
        has_biomedical_cross_species:cases.iter().any(|c|c.purpose==FixturePurpose::BiomedicalCrossSpecies),
        has_multilingual_wikipedia:cases.iter().any(|c|c.purpose==FixturePurpose::MultilingualWikipedia),
        has_negative_control:cases.iter().any(|c|c.negative_control),
        acceptance_ready:ready==cases.len()
            &&cases.iter().any(|c|c.purpose==FixturePurpose::WikidataText)
            &&cases.iter().any(|c|c.purpose==FixturePurpose::BiomedicalCrossSpecies)
            &&cases.iter().any(|c|c.purpose==FixturePurpose::MultilingualWikipedia)
            &&cases.iter().any(|c|c.negative_control),
        creates_semantic_authority:false,
        synthesizes_missing_evidence:false,
        cases,
    };
    println!("{}",serde_json::to_string_pretty(&receipt)
        .map_err(|e|e.to_string())?);
    if !receipt.acceptance_ready{
        return Err(format!(
            "fixture corpus incomplete: {} cases missing/unready",
            receipt.missing_case_count));
    }
    Ok(())
}
fn main(){
    if let Err(e)=run(){eprintln!("{e}");process::exit(2);}
}
