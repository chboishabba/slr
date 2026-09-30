//! ITIR-REL-1B runtime witness certification boundary.
//!
//! A checked runtime witness is stronger than an opaque "scope:proof" string:
//! it is source-pinned, kind-indexed, payload-digested and validator-attributed.
//! It is still NOT a Lean/Agda proof term. Formal theorem application must
//! separately translate the validated evidence object into formal premises.

use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use thiserror::Error;

#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum RuntimeWitnessKind {
    SubjectIdentity,
    PropertyAlignment,
    ScopeComparability,
    LeftApplicability,
    RightApplicability,
    ValueDistinctness,
    PositiveOutsideScope,
    ConsumerObservationPreservation,
    HeterogeneousObservableBridge,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct RuntimeWitnessCertificate {
    pub witness_ref:String,
    pub kind:RuntimeWitnessKind,
    pub consumer_ref:String,
    pub source_revision_refs:Vec<String>,
    pub native_evidence_refs:Vec<String>,
    /// Deterministic UTF-8 certificate payload emitted by the validator.
    pub checked_payload:String,
    pub checked_payload_sha256:String,
    pub validator_ref:String,
    pub validator_version_ref:String,
    pub validation_receipt_ref:String,
    /// Checked evidence may be valid at runtime without constituting a formal proof.
    pub runtime_validated:bool,
    pub formal_premise_established:bool,
    pub creates_semantic_authority:bool,
    pub claim_truth_promoted:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct CheckedRuntimeWitness {
    pub witness_ref:String,
    pub kind:RuntimeWitnessKind,
    pub consumer_ref:String,
    pub source_revision_refs:Vec<String>,
    pub validation_receipt_ref:String,
    pub payload_sha256:String,
    pub runtime_validated:bool,
    pub formal_premise_established:bool,
}
#[derive(Debug,Error,PartialEq,Eq)]
pub enum RuntimeWitnessError {
    #[error("runtime witness certificate has missing identity/provenance")]
    MissingIdentity,
    #[error("runtime witness payload digest mismatch")]
    DigestMismatch,
    #[error("runtime witness attempted truth/authority promotion")]
    Promotion,
    #[error("runtime witness was not positively validated")]
    NotValidated,
    #[error("runtime certificate may not claim formal-premise establishment")]
    PretendsFormalProof,
}
fn valid(s:&str)->bool{!s.trim().is_empty()}
fn digest(s:&str)->String{
    format!("sha256:{:x}",Sha256::digest(s.as_bytes()))
}
pub fn validate_runtime_witness(
    c:&RuntimeWitnessCertificate,
)->Result<CheckedRuntimeWitness,RuntimeWitnessError>{
    if !valid(&c.witness_ref)||!valid(&c.consumer_ref)
        ||c.source_revision_refs.is_empty()
        ||c.source_revision_refs.iter().any(|x|!valid(x))
        ||c.native_evidence_refs.is_empty()
        ||c.native_evidence_refs.iter().any(|x|!valid(x))
        ||!valid(&c.checked_payload)||!valid(&c.checked_payload_sha256)
        ||!valid(&c.validator_ref)||!valid(&c.validator_version_ref)
        ||!valid(&c.validation_receipt_ref){
        return Err(RuntimeWitnessError::MissingIdentity);
    }
    if digest(&c.checked_payload)!=c.checked_payload_sha256{
        return Err(RuntimeWitnessError::DigestMismatch);
    }
    if c.creates_semantic_authority||c.claim_truth_promoted{
        return Err(RuntimeWitnessError::Promotion);
    }
    if !c.runtime_validated{
        return Err(RuntimeWitnessError::NotValidated);
    }
    // Runtime validation can discharge an executable gate but cannot simply
    // assert that a proposition in an external proof assistant is inhabited.
    if c.formal_premise_established{
        return Err(RuntimeWitnessError::PretendsFormalProof);
    }
    let mut revisions=c.source_revision_refs.clone();
    revisions.sort();revisions.dedup();
    Ok(CheckedRuntimeWitness{
        witness_ref:c.witness_ref.clone(),kind:c.kind,
        consumer_ref:c.consumer_ref.clone(),source_revision_refs:revisions,
        validation_receipt_ref:c.validation_receipt_ref.clone(),
        payload_sha256:c.checked_payload_sha256.clone(),
        runtime_validated:true,formal_premise_established:false,
    })
}

pub fn require_witness(
    checked:&[CheckedRuntimeWitness],
    witness_ref:&str,kind:RuntimeWitnessKind,consumer_ref:&str,
    required_sources:&[String],
)->Result<(),RuntimeWitnessError>{
    let Some(w)=checked.iter().find(|w|
        w.witness_ref==witness_ref&&w.kind==kind&&w.consumer_ref==consumer_ref
    ) else{return Err(RuntimeWitnessError::NotValidated)};
    if required_sources.iter().any(|source|
        !w.source_revision_refs.iter().any(|r|r==source)){
        return Err(RuntimeWitnessError::MissingIdentity);
    }
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    fn cert()->RuntimeWitnessCertificate{
        let payload="{\"subject\":\"Q1\",\"scope\":\"1901\"}".to_owned();
        RuntimeWitnessCertificate{
            witness_ref:"witness:subject".into(),
            kind:RuntimeWitnessKind::SubjectIdentity,
            consumer_ref:"matter:1".into(),
            source_revision_refs:vec!["source:a".into(),"source:b".into()],
            native_evidence_refs:vec!["statement:a".into(),"statement:b".into()],
            checked_payload_sha256:digest(&payload),checked_payload:payload,
            validator_ref:"validator:source-pair".into(),
            validator_version_ref:"v1".into(),
            validation_receipt_ref:"validation:1".into(),
            runtime_validated:true,formal_premise_established:false,
            creates_semantic_authority:false,claim_truth_promoted:false,
        }
    }
    #[test]fn checked_runtime_evidence_remains_nonformal(){
        let checked=validate_runtime_witness(&cert()).unwrap();
        assert!(checked.runtime_validated);
        assert!(!checked.formal_premise_established);
    }
    #[test]fn cannot_self_assert_formal_proof(){
        let mut c=cert();c.formal_premise_established=true;
        assert_eq!(validate_runtime_witness(&c).unwrap_err(),
            RuntimeWitnessError::PretendsFormalProof);
    }
}
