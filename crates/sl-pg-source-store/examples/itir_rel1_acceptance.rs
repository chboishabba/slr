//! ITIR-REL-1 bounded *real-source* regression runner.
//!
//! Supply actual previously ingested source revisions in the database,
//! independent expected findings, and source-content digests. No fabricated
//! "pass" is inferred from running a parser or from the GitHub source itself.
//! Independent original-source verification and domain licence review remain
//! distinct from this executable acceptance gate.
use serde::{Deserialize,Serialize};
use sensiblaw_pg_source_store::{
    load_database_config, persist_relational_comparison,
    compare_relational_observations, RelationalObservation,RelationalConsumer,
    ComparisonFinding,ResidualKind,SourceFamily,
};
use std::{collections::BTreeSet,env,fs,process};

#[derive(Debug,Deserialize)]
struct Suite{
    schema:String,
    source_fixture_set_ref:String,
    cases:Vec<Case>,
}
#[derive(Debug,Deserialize)]
struct Case{
    case_ref:String,
    family_purpose:String,
    independent_source_receipt_refs:Vec<String>,
    left_source_sha256:String,
    right_source_sha256:String,
    left:RelationalObservation,
    right:RelationalObservation,
    consumer:RelationalConsumer,
    expected_finding:ComparisonFinding,
    required_residual_kinds:Vec<ResidualKind>,
    forbidden_residual_kinds:Vec<ResidualKind>,
    negative_control:bool,
}
#[derive(Debug,Serialize)]
struct CaseReceipt<'a>{
    case_ref:&'a str,
    status:&'static str,
    comparison_ref:String,
    finding:ComparisonFinding,
    observed_residual_kinds:Vec<String>,
    source_digest_matched:bool,
    exact_pg_reopen:bool,
    witness_independence_established:bool,
    creates_semantic_authority:bool,
    claim_truth_promoted:bool,
}
fn run()->Result<(),String>{
    let path=env::args().nth(1).ok_or_else(||
        "usage: itir_rel1_acceptance <original-source-suite.json>".to_owned())?;
    let suite:Suite=serde_json::from_slice(&fs::read(path)
        .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    if suite.schema!="itir.rel1.source-acceptance.v1"
        || suite.source_fixture_set_ref.trim().is_empty()
        || suite.cases.len()<3
        || !suite.cases.iter().any(|v|v.negative_control)
    {return Err("suite needs a source fixture ref, three cases and a negative control".into());}
    let wiki_text=suite.cases.iter().any(|c|
        (c.left.source_family==SourceFamily::Wikidata
            &&c.right.source_family==SourceFamily::Wikipedia)
        ||(c.left.source_family==SourceFamily::Wikipedia
            &&c.right.source_family==SourceFamily::Wikidata));
    let biomedical=suite.cases.iter().any(|c|
        c.left.source_family==SourceFamily::Biomedical
            &&c.right.source_family==SourceFamily::Biomedical
            &&c.left.source_revision_ref!=c.right.source_revision_ref);
    let multilingual=suite.cases.iter().any(|c|
        c.left.source_family==SourceFamily::Wikipedia
            &&c.right.source_family==SourceFamily::Wikipedia
            &&c.left.context.language_ref.is_some()
            &&c.right.context.language_ref.is_some()
            &&c.left.context.language_ref!=c.right.context.language_ref);
    if !(wiki_text && biomedical && multilingual){
        return Err("ITIR-REL-1 requires Wikidata↔text, biomedical pair, and two Wikipedia language views".into());
    }
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    let mut failures=Vec::new();
    for case in &suite.cases {
        if case.case_ref.trim().is_empty()||case.family_purpose.trim().is_empty()
            || case.independent_source_receipt_refs.is_empty()
            || !case.independent_source_receipt_refs.iter()
                .all(|r|!r.trim().is_empty()){
            failures.push(format!("{}: missing independent source/producer identity",
                case.case_ref));
            continue;
        }
        let result=(||->Result<_,String>{
            let pure=compare_relational_observations(
                &case.left,&case.right,&case.consumer)
                .map_err(|e|e.to_string())?;
            let persisted=persist_relational_comparison(
                &config,&case.left,&case.right,&case.consumer)
                .map_err(|e|e.to_string())?;
            let actual=persisted.comparison;
            let found=actual.residuals.iter()
                .map(|r|format!("{:?}",r.kind)).collect::<BTreeSet<_>>();
            let required=case.required_residual_kinds.iter()
                .map(|r|format!("{r:?}")).collect::<BTreeSet<_>>();
            let forbidden=case.forbidden_residual_kinds.iter()
                .map(|r|format!("{r:?}")).collect::<BTreeSet<_>>();
            let digest_match=persisted.left_source_content_sha256==
                    case.left_source_sha256
                &&persisted.right_source_content_sha256==
                    case.right_source_sha256;
            let good=actual==pure && digest_match
                && actual.finding==case.expected_finding
                && required.is_subset(&found)
                && forbidden.is_disjoint(&found)
                && !actual.creates_semantic_authority
                && !actual.claim_truth_promoted
                && !actual.merges_sources
                && !actual.proves_independence;
            let receipt=CaseReceipt{
                case_ref:&case.case_ref,
                status:if good{"pass"}else{"fail"},
                comparison_ref:actual.comparison_ref.clone(),
                finding:actual.finding,
                observed_residual_kinds:found.into_iter().collect(),
                source_digest_matched:digest_match,exact_pg_reopen:actual==pure,
                witness_independence_established:false,
                creates_semantic_authority:false,claim_truth_promoted:false,
            };
            Ok(receipt)
        })();
        match result{
            Ok(receipt)=>{
                if receipt.status=="fail"{
                    failures.push(format!("{}: wrong finding/residual/digest",
                        case.case_ref));
                }
                println!("{}",serde_json::to_string(&receipt)
                    .map_err(|e|e.to_string())?);
            },
            Err(e)=>failures.push(format!("{}: {e}",case.case_ref)),
        }
    }
    if !failures.is_empty(){
        for failure in &failures{eprintln!("FAIL {failure}");}
        return Err(format!("{} source-grounded checks failed",failures.len()));
    }
    println!("source_suite={} cases={} candidate_only=true",
        suite.source_fixture_set_ref,suite.cases.len());
    println!("NOTE: producer/permission/proof receipt authenticity is a separate gate");
    Ok(())
}
fn main(){
    if let Err(e)=run(){eprintln!("{e}");process::exit(1);}
}
