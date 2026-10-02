//! ITIR-REL-1B integrated source-grounded acceptance runner.
//!
//! One fixture graph now exercises:
//! native source -> relational comparison -> optional scoped contract ->
//! durable PG persist/reopen -> optional modeled repair assessment.
//!
//! Runtime witness certificates are validator-attributed and source-pinned;
//! they are NOT Lean/Agda proof terms. No source edit or semantic promotion.

use serde::{Deserialize,Serialize};
use sensiblaw_pg_source_store::{
    assess_bounded_repair, compare_relational_observations,
    evaluate_single_value_contract, load_database_config,
    persist_relational_comparison, require_witness, validate_runtime_witness,
    BoundedRepairCandidate, ComparisonFinding, ContractResult,
    RelationalConsumer, RelationalObservation, ResidualKind,
    RuntimeWitnessCertificate, RuntimeWitnessKind, ScopedIncidence,
    ScopedPairEvidence, SingleValueContract, RelationalSourceFamily,
};
use std::{collections::BTreeSet,env,fs,process};

#[derive(Debug,Deserialize)]
struct Suite{
    schema:String,
    source_fixture_set_ref:String,
    cases:Vec<Case>,
}
#[derive(Debug,Deserialize)]
struct SoftTypeAcceptance{
    contract:SingleValueContract,
    left:ScopedIncidence,
    right:ScopedIncidence,
    pair:ScopedPairEvidence,
    expected_status:ContractResult,
    required_missing_premises:Vec<String>,
    forbidden_missing_premises:Vec<String>,
}
#[derive(Debug,Deserialize)]
struct RepairAcceptance{
    after_left:RelationalObservation,
    after_right:RelationalObservation,
    proposal:BoundedRepairCandidate,
    expected_strict_improvement:bool,
    expected_discharged_obligation_refs:Vec<String>,
    expected_new_obligation_refs:Vec<String>,
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
    #[serde(default)]
    runtime_witness_certificates:Vec<RuntimeWitnessCertificate>,
    #[serde(default)]
    soft_type:Option<SoftTypeAcceptance>,
    #[serde(default)]
    repair:Option<RepairAcceptance>,
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
    soft_type_status:Option<ContractResult>,
    soft_type_missing_premises:Vec<String>,
    runtime_witnesses_validated:usize,
    formal_premises_established_by_runtime:bool,
    repair_checked:bool,
    repair_strictly_improves:Option<bool>,
    repair_discharged_obligation_refs:Vec<String>,
    repair_new_obligation_refs:Vec<String>,
    repair_preservation_runtime_validated:bool,
    witness_independence_established:bool,
    creates_semantic_authority:bool,
    claim_truth_promoted:bool,
}

fn required_sources(case:&Case)->Vec<String>{
    vec![case.left.source_revision_ref.clone(),case.right.source_revision_ref.clone()]
}
fn certify_runtime_witnesses(case:&Case)
    ->Result<Vec<sensiblaw_pg_source_store::CheckedRuntimeWitness>,String>{
    case.runtime_witness_certificates.iter()
        .map(|c|validate_runtime_witness(c).map_err(|e|e.to_string()))
        .collect()
}
fn require_optional(
    checked:&[sensiblaw_pg_source_store::CheckedRuntimeWitness],
    reference:&Option<String>,kind:RuntimeWitnessKind,case:&Case,
    required_sources:&[String],
)->Result<(),String>{
    if let Some(reference)=reference{
        require_witness(checked,reference,kind,&case.consumer.consumer_ref,
            required_sources).map_err(|e|e.to_string())?;
    }
    Ok(())
}
fn check_soft_type(case:&Case,
    checked:&[sensiblaw_pg_source_store::CheckedRuntimeWitness])
    ->Result<(ContractResult,Vec<String>),String>{
    let spec=case.soft_type.as_ref()
        .ok_or_else(||"soft-type acceptance not supplied".to_owned())?;
    if spec.contract.consumer_ref!=case.consumer.consumer_ref{
        return Err("soft-type contract consumer differs from relational consumer".into());
    }
    let pair_sources=required_sources(case);
    let left_source=vec![case.left.source_revision_ref.clone()];
    let right_source=vec![case.right.source_revision_ref.clone()];
    require_optional(checked,&spec.pair.shared_subject_witness_ref,
        RuntimeWitnessKind::SubjectIdentity,case,&pair_sources)?;
    require_optional(checked,&spec.pair.property_alignment_witness_ref,
        RuntimeWitnessKind::PropertyAlignment,case,&pair_sources)?;
    require_optional(checked,&spec.pair.scope_comparability_witness_ref,
        RuntimeWitnessKind::ScopeComparability,case,&pair_sources)?;
    require_optional(checked,&spec.left.applicability_witness_ref,
        RuntimeWitnessKind::LeftApplicability,case,&left_source)?;
    require_optional(checked,&spec.right.applicability_witness_ref,
        RuntimeWitnessKind::RightApplicability,case,&right_source)?;
    require_optional(checked,&spec.pair.value_distinctness_witness_ref,
        RuntimeWitnessKind::ValueDistinctness,case,&pair_sources)?;
    require_optional(checked,&spec.pair.positive_outside_scope_witness_ref,
        RuntimeWitnessKind::PositiveOutsideScope,case,&pair_sources)?;
    let result=evaluate_single_value_contract(
        &spec.contract,&spec.left,&spec.right,&spec.pair)
        .map_err(|e|e.to_string())?;
    let missing=result.missing_premise_refs.iter().cloned()
        .collect::<BTreeSet<_>>();
    let required=spec.required_missing_premises.iter().cloned()
        .collect::<BTreeSet<_>>();
    let forbidden=spec.forbidden_missing_premises.iter().cloned()
        .collect::<BTreeSet<_>>();
    if result.status!=spec.expected_status
        ||!required.is_subset(&missing)||!forbidden.is_disjoint(&missing){
        return Err(format!(
            "soft-type mismatch: status={:?} missing={:?}",
            result.status,missing));
    }
    Ok((result.status,missing.into_iter().collect()))
}

fn run()->Result<(),String>{
    let path=env::args().nth(1).ok_or_else(||
        "usage: itir_rel1_acceptance <original-source-suite.json>".to_owned())?;
    let suite:Suite=serde_json::from_slice(&fs::read(path)
        .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    if suite.schema!="itir.rel1.integrated-acceptance.v1"
        ||suite.source_fixture_set_ref.trim().is_empty()
        ||suite.cases.len()<3
        ||!suite.cases.iter().any(|v|v.negative_control)
        ||!suite.cases.iter().any(|v|v.soft_type.is_some())
        ||!suite.cases.iter().any(|v|v.repair.is_some())
    {
        return Err("REL-1B requires three source cases, a negative control, a scoped contract case and a repair case".into());
    }
    let wiki_text=suite.cases.iter().any(|c|
        (c.left.source_family==RelationalSourceFamily::Wikidata
            &&c.right.source_family==RelationalSourceFamily::Wikipedia)
        ||(c.left.source_family==RelationalSourceFamily::Wikipedia
            &&c.right.source_family==RelationalSourceFamily::Wikidata));
    let biomedical=suite.cases.iter().any(|c|
        c.left.source_family==RelationalSourceFamily::Biomedical
            &&c.right.source_family==RelationalSourceFamily::Biomedical
            &&c.left.source_revision_ref!=c.right.source_revision_ref);
    let multilingual=suite.cases.iter().any(|c|
        c.left.source_family==RelationalSourceFamily::Wikipedia
            &&c.right.source_family==RelationalSourceFamily::Wikipedia
            &&c.left.context.language_ref.is_some()
            &&c.right.context.language_ref.is_some()
            &&c.left.context.language_ref!=c.right.context.language_ref);
    if !(wiki_text&&biomedical&&multilingual){
        return Err("REL-1B requires Wikidata↔text, biomedical pair, and two Wikipedia language views".into());
    }

    let config=load_database_config(None).map_err(|e|e.to_string())?;
    let mut failures=Vec::new();

    for case in &suite.cases{
        if case.case_ref.trim().is_empty()||case.family_purpose.trim().is_empty()
            ||case.independent_source_receipt_refs.is_empty()
            ||!case.independent_source_receipt_refs.iter().all(|r|!r.trim().is_empty()){
            failures.push(format!("{}: missing independent source/producer identity",case.case_ref));
            continue;
        }

        let result=(||->Result<CaseReceipt<'_>,String>{
            // Runtime evidence certificates are validated before any contract
            // status can consume their witness refs.
            let checked=certify_runtime_witnesses(case)?;
            if checked.iter().any(|w|w.formal_premise_established){
                return Err("runtime certificate illegally asserted a formal proof premise".into());
            }

            let pure=compare_relational_observations(
                &case.left,&case.right,&case.consumer)
                .map_err(|e|e.to_string())?;

            // Contract is evaluated on the same fixture before persistence,
            // then the comparison is persisted/reopened and checked equal.
            let (soft_status,soft_missing)=if case.soft_type.is_some(){
                let (status,missing)=check_soft_type(case,&checked)?;
                (Some(status),missing)
            }else{(None,vec![])};

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

            let mut repair_strict=None;
            let mut discharged=vec![];
            let mut regressions=vec![];
            let mut preservation_runtime_validated=false;
            if let Some(spec)=&case.repair{
                if spec.proposal.consumer_observation_preservation_ref.trim().is_empty(){
                    return Err("repair preservation witness ref missing".into());
                }
                require_witness(&checked,
                    &spec.proposal.consumer_observation_preservation_ref,
                    RuntimeWitnessKind::ConsumerObservationPreservation,
                    &case.consumer.consumer_ref,&required_sources(case))
                    .map_err(|e|e.to_string())?;
                preservation_runtime_validated=true;
                let after=compare_relational_observations(
                    &spec.after_left,&spec.after_right,&case.consumer)
                    .map_err(|e|e.to_string())?;
                let assessment=assess_bounded_repair(
                    &actual,&after,&spec.proposal).map_err(|e|e.to_string())?;
                repair_strict=Some(assessment.strictly_improves_checked_debt);
                discharged=assessment.discharged_obligation_refs.clone();
                regressions=assessment.newly_created_obligation_refs.clone();
                let expected_d=spec.expected_discharged_obligation_refs.iter()
                    .cloned().collect::<BTreeSet<_>>();
                let observed_d=discharged.iter().cloned().collect::<BTreeSet<_>>();
                let expected_n=spec.expected_new_obligation_refs.iter()
                    .cloned().collect::<BTreeSet<_>>();
                let observed_n=regressions.iter().cloned().collect::<BTreeSet<_>>();
                if assessment.strictly_improves_checked_debt
                        !=spec.expected_strict_improvement
                    ||expected_d!=observed_d||expected_n!=observed_n
                    ||assessment.preservation_formally_verified
                    ||assessment.applies_external_edit
                    ||assessment.creates_semantic_authority
                    ||assessment.establishes_world_truth{
                    return Err(format!(
                        "repair assessment mismatch: strict={} discharged={:?} new={:?}",
                        assessment.strictly_improves_checked_debt,
                        observed_d,observed_n));
                }
            }

            let good=actual==pure&&digest_match
                &&actual.finding==case.expected_finding
                &&required.is_subset(&found)&&forbidden.is_disjoint(&found)
                &&!actual.creates_semantic_authority
                &&!actual.claim_truth_promoted
                &&!actual.merges_sources&&!actual.proves_independence;

            Ok(CaseReceipt{
                case_ref:&case.case_ref,status:if good{"pass"}else{"fail"},
                comparison_ref:actual.comparison_ref.clone(),
                finding:actual.finding.clone(),
                observed_residual_kinds:found.into_iter().collect(),
                source_digest_matched:digest_match,
                exact_pg_reopen:actual==pure,
                soft_type_status:soft_status,
                soft_type_missing_premises:soft_missing,
                runtime_witnesses_validated:checked.len(),
                formal_premises_established_by_runtime:false,
                repair_checked:case.repair.is_some(),
                repair_strictly_improves:repair_strict,
                repair_discharged_obligation_refs:discharged,
                repair_new_obligation_refs:regressions,
                repair_preservation_runtime_validated:preservation_runtime_validated,
                witness_independence_established:false,
                creates_semantic_authority:false,claim_truth_promoted:false,
            })
        })();

        match result{
            Ok(receipt)=>{
                if receipt.status=="fail"{
                    failures.push(format!("{}: wrong finding/residual/digest",case.case_ref));
                }
                println!("{}",serde_json::to_string(&receipt)
                    .map_err(|e|e.to_string())?);
            },
            Err(error)=>failures.push(format!("{}: {error}",case.case_ref)),
        }
    }

    if !failures.is_empty(){
        for failure in &failures{eprintln!("FAIL {failure}");}
        return Err(format!("{} integrated source-grounded checks failed",failures.len()));
    }
    println!("source_suite={} cases={} integrated_contract_acceptance=true",
        suite.source_fixture_set_ref,suite.cases.len());
    println!("runtime_witnesses_are_not_formal_proofs=true");
    println!("source_edits_applied=false");
    Ok(())
}
fn main(){
    if let Err(error)=run(){eprintln!("{error}");process::exit(1);}
}
