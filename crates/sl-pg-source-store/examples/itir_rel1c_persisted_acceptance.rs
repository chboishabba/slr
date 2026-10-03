//! REL-1C heterogeneous corpus acceptance over persisted native products.
//!
//! Unlike REL-1B, cases do not carry hand-authored RelationalObservation
//! values. Each side is reopened either from durable candidate PNF or from a
//! persisted native Wikidata diagnostic statement, then compared and persisted.

use serde::{Deserialize, Serialize};
use sensiblaw_pg_source_store::{
    load_database_config, persist_relational_comparison,
    relational_observation_from_persisted_pnf,
    relational_observation_from_persisted_wikidata,
    ComparisonFinding, ObservationContext, PersistedRelationalObservationRequest,
    PersistedWikidataObservationRequest, RelationalConsumer, RelationalObservation,
    RelationalSourceFamily, ResidualKind,
};
use std::{collections::BTreeSet, env, fs, process};

const REQUEST_SCHEMA:&str="itir.rel1c.persisted-corpus.v1";

#[derive(Debug,Clone,Copy,Deserialize,Serialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
enum Purpose { WikidataText, BiomedicalPair, MultilingualWikipedia, NegativeControl }

#[derive(Debug,Deserialize)]
#[serde(tag="kind",rename_all="snake_case")]
enum ObservationSpec {
    CandidatePnf {
        batch_ref:String,
        selected_predicate_candidate_ref:String,
        observation_ref:String,
        source_family:RelationalSourceFamily,
        #[serde(default)] context:ObservationContext,
        #[serde(default)] provenance_refs:Vec<String>,
    },
    WikidataNative {
        diagnostic_ref:String,
        native_statement_ref:String,
        observation_ref:String,
        #[serde(default)] context:ObservationContext,
        #[serde(default)] provenance_refs:Vec<String>,
    },
}

#[derive(Debug,Deserialize)]
struct Case {
    case_ref:String,
    purpose:Purpose,
    left:ObservationSpec,
    right:ObservationSpec,
    consumer:RelationalConsumer,
    expected_finding:ComparisonFinding,
    #[serde(default)] required_residual_kinds:Vec<ResidualKind>,
    #[serde(default)] forbidden_residual_kinds:Vec<ResidualKind>,
}
#[derive(Debug,Deserialize)]
struct Suite { schema:String, corpus_ref:String, cases:Vec<Case> }

#[derive(Debug,Serialize)]
struct CaseReceipt {
    case_ref:String,
    purpose:Purpose,
    comparison_ref:String,
    left_source_revision_ref:String,
    right_source_revision_ref:String,
    left_family:RelationalSourceFamily,
    right_family:RelationalSourceFamily,
    finding:ComparisonFinding,
    residual_kinds:Vec<ResidualKind>,
    persisted_native_products_only:bool,
    exact_pg_reopen:bool,
    creates_semantic_authority:bool,
    claim_truth_promoted:bool,
}

fn observation(
    config:&sensiblaw_pg_source_store::DatabaseConfig,
    spec:ObservationSpec,
)->Result<RelationalObservation,String>{
    match spec {
        ObservationSpec::CandidatePnf{batch_ref,selected_predicate_candidate_ref,
            observation_ref,source_family,context,provenance_refs}=>
            relational_observation_from_persisted_pnf(config,
                &PersistedRelationalObservationRequest{
                    batch_ref,selected_predicate_candidate_ref,observation_ref,
                    source_family,context,provenance_refs,
                }).map_err(|e|e.to_string()),
        ObservationSpec::WikidataNative{diagnostic_ref,native_statement_ref,
            observation_ref,context,provenance_refs}=>
            relational_observation_from_persisted_wikidata(config,
                &PersistedWikidataObservationRequest{
                    diagnostic_ref,native_statement_ref,observation_ref,context,provenance_refs,
                }).map_err(|e|e.to_string()),
    }
}

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
        &&r.left_source_revision_ref!=r.right_source_revision_ref) {
        return Err("REL-1C lacks two-revision multilingual Wikipedia case".into());
    }
    if !receipts.iter().any(|r|r.purpose==Purpose::NegativeControl) {
        return Err("REL-1C lacks a negative control".into());
    }
    Ok(())
}

fn run()->Result<(),String>{
    let path=env::args().nth(1)
        .ok_or_else(||"usage: itir_rel1c_persisted_acceptance <corpus.json>".to_owned())?;
    let suite:Suite=serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())?;
    if suite.schema!=REQUEST_SCHEMA||suite.corpus_ref.trim().is_empty()||suite.cases.len()<4 {
        return Err("REL-1C requires a named corpus with at least four cases".into());
    }
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    let mut receipts=Vec::new();
    for case in suite.cases {
        let left=observation(&config,case.left)?;
        let right=observation(&config,case.right)?;
        if left.source_revision_ref==right.source_revision_ref {
            return Err(format!("{} reuses one source revision on both sides",case.case_ref));
        }
        let persisted=persist_relational_comparison(&config,&left,&right,&case.consumer)
            .map_err(|e|e.to_string())?;
        if persisted.comparison.finding!=case.expected_finding {
            return Err(format!("{} finding mismatch",case.case_ref));
        }
        let found=persisted.comparison.residuals.iter()
            .map(|r|r.kind.clone()).collect::<Vec<_>>();
        let found_set=found.iter().map(|k|format!("{k:?}")).collect::<BTreeSet<_>>();
        let required=case.required_residual_kinds.iter().map(|k|format!("{k:?}"))
            .collect::<BTreeSet<_>>();
        let forbidden=case.forbidden_residual_kinds.iter().map(|k|format!("{k:?}"))
            .collect::<BTreeSet<_>>();
        if !required.is_subset(&found_set)||!forbidden.is_disjoint(&found_set) {
            return Err(format!("{} residual contract mismatch",case.case_ref));
        }
        receipts.push(CaseReceipt{
            case_ref:case.case_ref,purpose:case.purpose,
            comparison_ref:persisted.comparison.comparison_ref,
            left_source_revision_ref:left.source_revision_ref,
            right_source_revision_ref:right.source_revision_ref,
            left_family:left.source_family,right_family:right.source_family,
            finding:persisted.comparison.finding,residual_kinds:found,
            persisted_native_products_only:true,exact_pg_reopen:true,
            creates_semantic_authority:false,claim_truth_promoted:false,
        });
    }
    validate_shape(&receipts)?;
    println!("{}",serde_json::to_string_pretty(&receipts).map_err(|e|e.to_string())?);
    println!("corpus_ref={}",suite.corpus_ref);
    println!("rel1c_persisted_corpus_acceptance=true");
    println!("hand_authored_relational_observations=false");
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");process::exit(1);}}
