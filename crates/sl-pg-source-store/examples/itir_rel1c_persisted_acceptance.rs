//! REL-1C heterogeneous corpus acceptance over persisted native products.
//!
//! Normal execution accepts one durable corpus ref.  Corpus membership,
//! native-product selectors, consumer fibres and required/forbidden residual
//! contracts are PostgreSQL-owned acceptance objects, not JSON control input.

use serde::Serialize;
use sensiblaw_pg_source_store::{
    load_database_config, load_rel_acceptance_corpus, persist_relational_comparison,
    relational_observation_from_persisted_pnf,
    relational_observation_from_persisted_wikidata,
    AcceptanceObservationKind, AcceptanceObservationSelector,
    PersistedRelationalObservationRequest, PersistedWikidataObservationRequest,
    RelationalObservation, RelationalSourceFamily, ResidualKind,
};
use std::{collections::BTreeSet, env, process};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Purpose { WikidataText, BiomedicalPair, MultilingualWikipedia, NegativeControl }

fn purpose(value:&str)->Result<Purpose,String>{match value{
    "wikidata_text"=>Ok(Purpose::WikidataText),
    "biomedical_pair"=>Ok(Purpose::BiomedicalPair),
    "multilingual_wikipedia"=>Ok(Purpose::MultilingualWikipedia),
    "negative_control"=>Ok(Purpose::NegativeControl),
    other=>Err(format!("unknown persisted REL-1C purpose: {other}")),
}}

#[derive(Debug,Serialize)]
struct CaseReceipt {
    case_ref:String,
    purpose:Purpose,
    comparison_ref:String,
    left_source_revision_ref:String,
    right_source_revision_ref:String,
    left_family:RelationalSourceFamily,
    right_family:RelationalSourceFamily,
    left_language_ref:Option<String>,
    right_language_ref:Option<String>,
    finding:String,
    residual_kinds:Vec<String>,
    persisted_native_products_only:bool,
    persisted_acceptance_control:bool,
    exact_pg_reopen:bool,
    creates_semantic_authority:bool,
    claim_truth_promoted:bool,
}

fn observation(
    config:&sensiblaw_pg_source_store::DatabaseConfig,
    selector:&AcceptanceObservationSelector,
)->Result<RelationalObservation,String>{
    match selector.kind {
        AcceptanceObservationKind::CandidatePnf => {
            relational_observation_from_persisted_pnf(config,
                &PersistedRelationalObservationRequest{
                    batch_ref:selector.batch_ref.clone().ok_or_else(||"candidate selector missing batch_ref".to_owned())?,
                    selected_predicate_candidate_ref:selector.selected_predicate_candidate_ref.clone()
                        .ok_or_else(||"candidate selector missing selected predicate".to_owned())?,
                    observation_ref:selector.observation_ref.clone(),
                    source_family:selector.source_family.ok_or_else(||"candidate selector missing source family".to_owned())?,
                    context:selector.context.clone(),
                    provenance_refs:selector.provenance_refs.clone(),
                }).map_err(|e|e.to_string())
        }
        AcceptanceObservationKind::WikidataNative => {
            relational_observation_from_persisted_wikidata(config,
                &PersistedWikidataObservationRequest{
                    diagnostic_ref:selector.diagnostic_ref.clone().ok_or_else(||"Wikidata selector missing diagnostic_ref".to_owned())?,
                    native_statement_ref:selector.native_statement_ref.clone().ok_or_else(||"Wikidata selector missing native_statement_ref".to_owned())?,
                    observation_ref:selector.observation_ref.clone(),
                    context:selector.context.clone(),
                    provenance_refs:selector.provenance_refs.clone(),
                }).map_err(|e|e.to_string())
        }
    }
}

fn residual_name(kind:&ResidualKind)->String{format!("{kind:?}")}

fn validate_shape(receipts:&[CaseReceipt])->Result<(),String>{
    if !receipts.iter().any(|r|r.purpose==Purpose::WikidataText
        &&((r.left_family==RelationalSourceFamily::Wikidata
             &&r.right_family==RelationalSourceFamily::Wikipedia)
          ||(r.right_family==RelationalSourceFamily::Wikidata
             &&r.left_family==RelationalSourceFamily::Wikipedia))) {
        return Err("REL-1C lacks persisted Wikidata↔Wikipedia case".into());
    }
    if !receipts.iter().any(|r|r.purpose==Purpose::BiomedicalPair
        &&r.left_family==RelationalSourceFamily::Biomedical
        &&r.right_family==RelationalSourceFamily::Biomedical
        &&r.left_source_revision_ref!=r.right_source_revision_ref) {
        return Err("REL-1C lacks two-source biomedical case".into());
    }
    if !receipts.iter().any(|r|r.purpose==Purpose::MultilingualWikipedia
        &&r.left_family==RelationalSourceFamily::Wikipedia
        &&r.right_family==RelationalSourceFamily::Wikipedia
        &&r.left_source_revision_ref!=r.right_source_revision_ref
        &&r.left_language_ref.is_some()
        &&r.right_language_ref.is_some()
        &&r.left_language_ref!=r.right_language_ref) {
        return Err("REL-1C lacks two persisted Wikipedia views with distinct explicit language coordinates".into());
    }
    if !receipts.iter().any(|r|r.purpose==Purpose::NegativeControl) {
        return Err("REL-1C lacks a negative control".into());
    }
    Ok(())
}

fn run()->Result<(),String>{
    let corpus_ref=env::args().nth(1)
        .ok_or_else(||"usage: itir_rel1c_persisted_acceptance <persisted-corpus-ref>".to_owned())?;
    if env::args().nth(2).is_some(){
        return Err("REL-1C accepts one persisted corpus ref, not a JSON/request packet".into());
    }
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    let suite=load_rel_acceptance_corpus(&config,&corpus_ref).map_err(|e|e.to_string())?;
    if suite.cases.len()<4{return Err("REL-1C requires a persisted corpus with at least four cases".into());}

    let mut receipts=Vec::new();
    for case in suite.cases {
        let case_purpose=purpose(&case.purpose_ref)?;
        let left=observation(&config,&case.left)?;
        let right=observation(&config,&case.right)?;
        if left.source_revision_ref==right.source_revision_ref {
            return Err(format!("{} reuses one source revision on both sides",case.case_ref));
        }
        if case_purpose==Purpose::MultilingualWikipedia
            &&(left.source_family!=RelationalSourceFamily::Wikipedia
                ||right.source_family!=RelationalSourceFamily::Wikipedia
                ||left.context.language_ref.is_none()
                ||right.context.language_ref.is_none()
                ||left.context.language_ref==right.context.language_ref) {
            return Err(format!(
                "{} is labelled multilingual but lacks two distinct explicit persisted language coordinates",
                case.case_ref));
        }
        let persisted=persist_relational_comparison(&config,&left,&right,&case.consumer)
            .map_err(|e|e.to_string())?;
        if persisted.comparison.finding!=case.expected_finding {
            return Err(format!("{} finding mismatch against persisted acceptance contract",case.case_ref));
        }
        let found=persisted.comparison.residuals.iter().map(|r|r.kind.clone()).collect::<Vec<_>>();
        let found_set=found.iter().map(residual_name).collect::<BTreeSet<_>>();
        let required=case.required_residual_kinds.iter().map(residual_name).collect::<BTreeSet<_>>();
        let forbidden=case.forbidden_residual_kinds.iter().map(residual_name).collect::<BTreeSet<_>>();
        if !required.is_subset(&found_set)||!forbidden.is_disjoint(&found_set) {
            return Err(format!("{} residual contract mismatch",case.case_ref));
        }
        receipts.push(CaseReceipt{
            case_ref:case.case_ref,purpose:case_purpose,
            comparison_ref:persisted.comparison.comparison_ref,
            left_source_revision_ref:left.source_revision_ref,
            right_source_revision_ref:right.source_revision_ref,
            left_family:left.source_family,right_family:right.source_family,
            left_language_ref:left.context.language_ref,
            right_language_ref:right.context.language_ref,
            finding:format!("{:?}",persisted.comparison.finding),
            residual_kinds:found.iter().map(residual_name).collect(),
            persisted_native_products_only:true,persisted_acceptance_control:true,
            exact_pg_reopen:true,creates_semantic_authority:false,claim_truth_promoted:false,
        });
    }
    validate_shape(&receipts)?;
    // JSON is receipt export only; it never configures this run.
    println!("{}",serde_json::to_string_pretty(&receipts).map_err(|e|e.to_string())?);
    println!("corpus_ref={}",suite.corpus_ref);
    println!("rel1c_persisted_corpus_acceptance=true");
    println!("hand_authored_relational_observations=false");
    println!("json_control_plane=false");
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");process::exit(1);}}
