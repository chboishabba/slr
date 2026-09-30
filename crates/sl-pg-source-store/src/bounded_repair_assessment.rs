//! Consumer-indexed, *modeled* repair improvement assessment.
//! This is neither a patch generator nor source edit/admission authority.
//! The same producer must supply before and after typed comparisons and
//! a separately recorded preservation witness. A narrower apparent debt set
//! alone is insufficient to infer that the source was corrected.
use std::collections::BTreeSet;
use serde::{Serialize,Deserialize};
use thiserror::Error;
use crate::{RelationalComparison,ComparisonResidual};

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct BoundedRepairCandidate {
    pub repair_candidate_ref:String,
    pub before_model_ref:String,
    pub after_model_ref:String,
    pub consumer_observation_preservation_ref:String,
    pub preservation_producer_ref:String,
    pub rerun_receipt_ref:String,
    pub source_edit_authorized:bool,
    pub applies_external_edit:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct BoundedRepairAssessment {
    pub repair_candidate_ref:String,
    pub consumer_ref:String,
    pub before_model_ref:String,
    pub after_model_ref:String,
    pub before_debt:Vec<String>,
    pub after_debt:Vec<String>,
    pub discharged_obligations:Vec<String>,
    pub newly_created_obligations:Vec<String>,
    pub strictly_improves_checked_debt:bool,
    pub producer_declares_consumer_observation_preservation:bool,
    /// A named witness ref is not yet a checked formal transport proof.
    pub preservation_formally_verified:bool,
    pub applies_external_edit:bool,
    pub creates_semantic_authority:bool,
    pub establishes_world_truth:bool,
}
#[derive(Debug,Error,PartialEq,Eq)]
pub enum RepairAssessmentError {
    #[error("the repair is not scoped to the same source pair and consumer")]
    WrongConsumerOrSourcePair,
    #[error("missing distinct modeled states or independently supplied preservation evidence")]
    MissingWitness,
    #[error("a modeled assessment cannot grant or perform an external source edit")]
    UnauthorizedEdit,
}
fn debt_key(r:&ComparisonResidual)->String{
    // Reproducible, lossless key using all relevant coordinates.
    serde_json::to_string(r).expect("serializable typed residual")
}
fn debts(c:&RelationalComparison)->BTreeSet<String>{
    c.residuals.iter().map(debt_key).collect()
}
/// The proof obligation is an *actual strict improvement* in the typed
/// obligation set, plus an independently given, producer-backed observation
/// preservation witness. New debt is returned, not erased or called clean.
pub fn assess_bounded_repair(
    before:&RelationalComparison,after:&RelationalComparison,
    proposal:&BoundedRepairCandidate,
)->Result<BoundedRepairAssessment,RepairAssessmentError>{
    if before.consumer_ref!=after.consumer_ref
        || before.left_observation_ref!=after.left_observation_ref
        || before.right_observation_ref!=after.right_observation_ref
    {return Err(RepairAssessmentError::WrongConsumerOrSourcePair);}
    if proposal.source_edit_authorized||proposal.applies_external_edit{
        return Err(RepairAssessmentError::UnauthorizedEdit);
    }
    if proposal.before_model_ref.trim().is_empty()
        || proposal.after_model_ref.trim().is_empty()
        || proposal.before_model_ref==proposal.after_model_ref
        || proposal.repair_candidate_ref.trim().is_empty()
        || proposal.consumer_observation_preservation_ref.trim().is_empty()
        || proposal.preservation_producer_ref.trim().is_empty()
        || proposal.rerun_receipt_ref.trim().is_empty()
    {return Err(RepairAssessmentError::MissingWitness);}
    let x=debts(before);
    let y=debts(after);
    let eliminated=x.difference(&y).cloned().collect::<Vec<_>>();
    let new=y.difference(&x).cloned().collect::<Vec<_>>();
    let strict=y.is_subset(&x)&&y!=x;
    Ok(BoundedRepairAssessment{
        repair_candidate_ref:proposal.repair_candidate_ref.clone(),
        consumer_ref:before.consumer_ref.clone(),
        before_model_ref:proposal.before_model_ref.clone(),
        after_model_ref:proposal.after_model_ref.clone(),
        before_debt:x.into_iter().collect(),
        after_debt:y.into_iter().collect(),
        discharged_obligations:eliminated,
        newly_created_obligations:new,
        strictly_improves_checked_debt:strict,
        // A named receipt is supplied, not yet an authenticated theorem;
        // this field records the producer's asserted preservation claim.
        producer_declares_consumer_observation_preservation:true,
        preservation_formally_verified:false,
        applies_external_edit:false,
        creates_semantic_authority:false,
        establishes_world_truth:false,
    })
}
#[cfg(test)]
mod tests{
    use super::*;
    use crate::{ComparisonFinding,ResidualKind};
    fn compare(ref_id:&str,obligations:&[&str])->RelationalComparison{
        RelationalComparison{
            schema:"itir.relational-comparison.v1".into(),
            consumer_ref:"consumer:q".into(),
            left_observation_ref:"obs:A".into(),
            right_observation_ref:"obs:B".into(),
            comparison_ref:ref_id.into(),
            finding:ComparisonFinding::PartialResidual,
            residuals:obligations.iter().map(|name|ComparisonResidual{
                kind:ResidualKind::UnalignedPredicate,left_ref:None,right_ref:None,
                obligation_ref:(*name).into(),
            }).collect(),
            used_alignment_witness_refs:vec![],
            role_type_evidence_refs:vec![],
            positive_support_refs:vec![],counter_support_refs:vec![],
            explicit_unknown_refs:vec![],
            creates_semantic_authority:false,merges_sources:false,
            proves_independence:false,claim_truth_promoted:false,
        }
    }
    fn candidate()->BoundedRepairCandidate{
        BoundedRepairCandidate{
            repair_candidate_ref:"repair:model".into(),
            before_model_ref:"state:before".into(),
            after_model_ref:"state:after".into(),
            consumer_observation_preservation_ref:"witness:consumer-observations".into(),
            preservation_producer_ref:"producer:model-check".into(),
            rerun_receipt_ref:"receipt:after-evaluation".into(),
            source_edit_authorized:false,applies_external_edit:false,
        }
    }
    #[test]fn strict_debt_reduction_is_not_external_edit(){
        let result=assess_bounded_repair(&compare("a",&["x","y"]),
            &compare("b",&["y"]),&candidate()).unwrap();
        assert!(result.strictly_improves_checked_debt);
        assert_eq!(result.discharged_obligations.len(),1);
        assert!(!result.applies_external_edit);
    }
    #[test]fn new_regression_blocks_no_worsening_claim(){
        let result=assess_bounded_repair(&compare("a",&["x"]),
            &compare("b",&["y"]),&candidate()).unwrap();
        assert!(!result.strictly_improves_checked_debt);
        assert_eq!(result.newly_created_obligations.len(),1);
    }
}
