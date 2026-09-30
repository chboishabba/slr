//! ITIR-REL-1 soft-type obligation evaluator.
//!
//! Separate observed incidence, declared type, scoped applicability and
//! value-distinctness evidence. A pair of conflicting values is a candidate
//! violation *only if the relevant identity, scope, applicability and
//! distinctness receipts are separately supplied*. A missing receipt means
//! incomplete evidence, not "false" and not a violation.
//!
//! This module does not claim runtime receipt authentication or globally
//! checked Wikidata truth. The formal counterpart is
//! RequestProject/DASHIScopedSoftTyping.lean (a proof about supplied
//! counterexample certificates, not the source acquisition pipeline).
use std::collections::BTreeSet;
use serde::{Deserialize,Serialize};
use thiserror::Error;

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct ScopedIncidence {
    pub source_revision_ref:String,
    pub statement_ref:String,
    pub subject_candidate_ref:String,
    pub property_candidate_ref:String,
    pub value_candidate_ref:String,
    pub declared_type_ref:Option<String>,
    pub scope_ref:Option<String>,
    pub applicability_witness_ref:Option<String>,
    pub provenance_refs:Vec<String>,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct SingleValueContract {
    pub consumer_ref:String,
    pub contract_ref:String,
    pub property_ref:String,
    pub subject_ref:String,
    pub scope_ref:String,
    pub contract_source_ref:String,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct ScopedPairEvidence {
    pub shared_subject_witness_ref:Option<String>,
    pub property_alignment_witness_ref:Option<String>,
    pub scope_comparability_witness_ref:Option<String>,
    pub value_distinctness_witness_ref:Option<String>,
    /// Positive evidence that the contract does not apply to this source pair.
    /// This is categorically different from failure to establish comparability.
    #[serde(default)]
    pub positive_outside_scope_witness_ref:Option<String>,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ContractResult {
    CandidateViolation,
    CompatibleObservedValues,
    Undetermined,
    OutsideScope,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct ScopedContractReceipt {
    pub consumer_ref:String,
    pub contract_ref:String,
    pub left_statement_ref:String,
    pub right_statement_ref:String,
    pub status:ContractResult,
    pub witness_refs:Vec<String>,
    pub missing_premise_refs:Vec<String>,
    pub observed_value_refs:Vec<String>,
    pub suggested_retyping:bool,
    pub establishes_claim_truth:bool,
    pub establishes_source_identity:bool,
    pub grants_repair_authority:bool,
}
#[derive(Debug,Error,PartialEq,Eq)]
pub enum SoftTypeError {
    #[error("missing source, consumer, contract or statement identity")]
    InvalidIdentity,
    #[error("two source-local incidences must have distinct statement IDs")]
    DuplicateStatement,
}
fn valid(v:&str)->bool{!v.trim().is_empty()}
fn optional_witness(v:&Option<String>)->Option<String>{
    v.as_ref().filter(|x|valid(x)).cloned()
}
fn needs(debt:&mut Vec<String>,name:&str,value:&Option<String>)->Option<String>{
    let receipt=optional_witness(value);
    if receipt.is_none(){debt.push(name.into());}
    receipt
}
/// A literal value match is a *candidate compatibility only*.
/// Different identifiers alone do not prove distinct real-world values,
/// and different time coordinates alone do not create a counterexample.
pub fn evaluate_single_value_contract(
    contract:&SingleValueContract,left:&ScopedIncidence,
    right:&ScopedIncidence,pair:&ScopedPairEvidence,
)->Result<ScopedContractReceipt,SoftTypeError>{
    if [
        &contract.consumer_ref,&contract.contract_ref,
        &contract.property_ref,&contract.subject_ref,
        &contract.scope_ref,&contract.contract_source_ref,
        &left.source_revision_ref,&right.source_revision_ref,
        &left.statement_ref,&right.statement_ref,
        &left.subject_candidate_ref,&right.subject_candidate_ref,
        &left.property_candidate_ref,&right.property_candidate_ref,
        &left.value_candidate_ref,&right.value_candidate_ref,
    ].iter().any(|v|!valid(v)){
        return Err(SoftTypeError::InvalidIdentity);
    }
    if left.statement_ref==right.statement_ref{
        return Err(SoftTypeError::DuplicateStatement);
    }
    let mut debt=Vec::new();
    let mut witnesses=BTreeSet::new();
    witnesses.insert(contract.contract_source_ref.clone());
    if let Some(outside)=optional_witness(&pair.positive_outside_scope_witness_ref){
        witnesses.insert(outside);
        return Ok(ScopedContractReceipt{
            consumer_ref:contract.consumer_ref.clone(),
            contract_ref:contract.contract_ref.clone(),
            left_statement_ref:left.statement_ref.clone(),
            right_statement_ref:right.statement_ref.clone(),
            status:ContractResult::OutsideScope,
            witness_refs:witnesses.into_iter().collect(),
            missing_premise_refs:vec![],
            observed_value_refs:vec![left.value_candidate_ref.clone(),
                right.value_candidate_ref.clone()],
            suggested_retyping:false,establishes_claim_truth:false,
            establishes_source_identity:false,grants_repair_authority:false,
        });
    }
    for receipt in [
        needs(&mut debt,"subject-identity-unpaid",
            &pair.shared_subject_witness_ref),
        needs(&mut debt,"property-alignment-unpaid",
            &pair.property_alignment_witness_ref),
        needs(&mut debt,"scope-comparability-unpaid",
            &pair.scope_comparability_witness_ref),
        needs(&mut debt,"left-applicability-unpaid",
            &left.applicability_witness_ref),
        needs(&mut debt,"right-applicability-unpaid",
            &right.applicability_witness_ref),
    ].into_iter().flatten(){witnesses.insert(receipt);}
    if left.subject_candidate_ref!=contract.subject_ref
        || right.subject_candidate_ref!=contract.subject_ref{
        debt.push("consumer-subject-alignment-unpaid".into());
    }
    if left.property_candidate_ref!=contract.property_ref
        || right.property_candidate_ref!=contract.property_ref{
        debt.push("consumer-property-alignment-unpaid".into());
    }
    // Scope tags are used only when asserted comparable; the scope
    // comparability receipt is still needed even for equal literal tags.
    if left.scope_ref.as_deref()!=Some(contract.scope_ref.as_str())
        || right.scope_ref.as_deref()!=Some(contract.scope_ref.as_str()){
        debt.push("source-scope-or-time-alignment-unpaid".into());
    }
    // Two different values under the same claim need a witnessed
    // non-identity relation; strings alone cannot pay this proof.
    let different=left.value_candidate_ref!=right.value_candidate_ref;
    if different{
        if let Some(ref_id)=needs(&mut debt,
            "value-distinctness-unpaid",&pair.value_distinctness_witness_ref){
            witnesses.insert(ref_id);
        }
    }
    let status=if !debt.is_empty(){
        ContractResult::Undetermined
    }else if different{
        ContractResult::CandidateViolation
    }else{
        ContractResult::CompatibleObservedValues
    };
    Ok(ScopedContractReceipt{
        consumer_ref:contract.consumer_ref.clone(),
        contract_ref:contract.contract_ref.clone(),
        left_statement_ref:left.statement_ref.clone(),
        right_statement_ref:right.statement_ref.clone(),
        status,witness_refs:witnesses.into_iter().collect(),
        missing_premise_refs:debt,
        observed_value_refs:vec![left.value_candidate_ref.clone(),
            right.value_candidate_ref.clone()],
        suggested_retyping:false,establishes_claim_truth:false,
        establishes_source_identity:false,grants_repair_authority:false,
    })
}

#[cfg(test)]
mod tests{
    use super::*;
    fn specimen()->(SingleValueContract,ScopedIncidence,ScopedIncidence,
        ScopedPairEvidence){
        let c=SingleValueContract{
            consumer_ref:"matter:1".into(),contract_ref:"contract:single".into(),
            property_ref:"property:P".into(),subject_ref:"entity:Q".into(),
            scope_ref:"year:1901".into(),contract_source_ref:"rule:ref".into(),
        };
        let mk=|id:&str,value:&str|ScopedIncidence{
            source_revision_ref:format!("source:{id}"),
            statement_ref:format!("statement:{id}"),
            subject_candidate_ref:"entity:Q".into(),
            property_candidate_ref:"property:P".into(),
            value_candidate_ref:value.into(),declared_type_ref:None,
            scope_ref:Some("year:1901".into()),
            applicability_witness_ref:Some(format!("applies:{id}")),
            provenance_refs:vec![format!("receipt:{id}")],
        };
        let pair=ScopedPairEvidence{
            shared_subject_witness_ref:Some("subject:proof".into()),
            property_alignment_witness_ref:Some("property:proof".into()),
            scope_comparability_witness_ref:Some("scope:proof".into()),
            value_distinctness_witness_ref:Some("values:nonidentity".into()),
            positive_outside_scope_witness_ref:None,
        };
        (c,mk("a","A"),mk("b","B"),pair)
    }
    #[test]fn counterexample_requires_all_four_payments(){
        let(c,a,b,p)=specimen();
        let result=evaluate_single_value_contract(&c,&a,&b,&p).unwrap();
        assert_eq!(result.status,ContractResult::CandidateViolation);
        assert_eq!(result.missing_premise_refs.len(),0);
        assert!(!result.establishes_claim_truth&&!result.grants_repair_authority);
    }
    #[test]fn missing_identity_cannot_become_violation(){
        let(c,a,b,mut p)=specimen();
        p.shared_subject_witness_ref=None;
        let result=evaluate_single_value_contract(&c,&a,&b,&p).unwrap();
        assert_eq!(result.status,ContractResult::Undetermined);
        assert!(result.missing_premise_refs.contains(
            &"subject-identity-unpaid".to_owned()));
    }
    #[test]fn different_year_is_not_value_contract_failure(){
        let(c,a,mut b,p)=specimen();
        b.scope_ref=Some("year:1902".into());
        let result=evaluate_single_value_contract(&c,&a,&b,&p).unwrap();
        assert_eq!(result.status,ContractResult::Undetermined);
        assert!(result.missing_premise_refs.contains(
            &"source-scope-or-time-alignment-unpaid".to_owned()));
    }
    #[test]fn positive_nonapplicability_is_outside_scope_not_unknown(){
        let(c,a,b,mut p)=specimen();
        p.positive_outside_scope_witness_ref=Some("scope:does-not-apply".into());
        p.scope_comparability_witness_ref=None;
        let result=evaluate_single_value_contract(&c,&a,&b,&p).unwrap();
        assert_eq!(result.status,ContractResult::OutsideScope);
        assert!(result.missing_premise_refs.is_empty());
        assert!(result.witness_refs.contains(&"scope:does-not-apply".to_owned()));
    }

    #[test]fn distinctness_receipt_not_optional(){
        let(c,a,b,mut p)=specimen();
        p.value_distinctness_witness_ref=None;
        let result=evaluate_single_value_contract(&c,&a,&b,&p).unwrap();
        assert_eq!(result.status,ContractResult::Undetermined);
    }
}
