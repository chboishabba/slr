//! Multi-consumer bounded repair vector.
//!
//! A repair can improve one consumer and regress another. This aggregator
//! refuses an overall "good repair" label unless every checked consumer is
//! preserved or improved and at least one is improved. Unchecked consumers
//! stay explicit. No source edit is performed.

use serde::{Deserialize,Serialize};
use thiserror::Error;
use crate::{
    assess_bounded_repair, BoundedRepairAssessment, BoundedRepairCandidate,
    RelationalComparison, RepairAssessmentError,
};

#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum RepairConsumerOutcome {
    Improved,
    Preserved,
    Regressed,
    Unchecked,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct RepairVectorEntry {
    pub consumer_ref:String,
    pub before:RelationalComparison,
    pub after:RelationalComparison,
    pub proposal:Option<BoundedRepairCandidate>,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct RepairVectorConsumerReceipt {
    pub consumer_ref:String,
    pub outcome:RepairConsumerOutcome,
    pub discharged_obligation_refs:Vec<String>,
    pub new_obligation_refs:Vec<String>,
    pub runtime_preservation_witness_ref:Option<String>,
    pub preservation_formally_verified:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct RepairVectorReceipt {
    pub repair_vector_ref:String,
    pub consumers:Vec<RepairVectorConsumerReceipt>,
    pub checked_consumer_count:usize,
    pub improved_consumer_count:usize,
    pub regressed_consumer_count:usize,
    pub unchecked_consumer_count:usize,
    pub no_checked_regressions:bool,
    pub at_least_one_checked_improvement:bool,
    pub acceptable_for_further_review:bool,
    pub applies_external_edit:bool,
    pub creates_semantic_authority:bool,
}
#[derive(Debug,Error)]
pub enum RepairVectorError {
    #[error(transparent)]
    Assessment(#[from]RepairAssessmentError),
    #[error("repair vector needs a stable identity and unique consumer refs")]
    InvalidIdentity,
    #[error("entry consumer ref does not match its before/after comparison")]
    ConsumerMismatch,
}
fn same_debt(a:&BoundedRepairAssessment)->bool{
    a.before_debt==a.after_debt
        &&a.discharged_obligations.is_empty()
        &&a.newly_created_obligations.is_empty()
}
pub fn assess_repair_vector(
    repair_vector_ref:&str,entries:&[RepairVectorEntry],
)->Result<RepairVectorReceipt,RepairVectorError>{
    if repair_vector_ref.trim().is_empty()||entries.is_empty(){
        return Err(RepairVectorError::InvalidIdentity);
    }
    let mut unique=std::collections::BTreeSet::new();
    let mut consumers=Vec::with_capacity(entries.len());
    for entry in entries{
        if entry.consumer_ref.trim().is_empty()
            ||!unique.insert(entry.consumer_ref.clone()){
            return Err(RepairVectorError::InvalidIdentity);
        }
        if entry.before.consumer_ref!=entry.consumer_ref
            ||entry.after.consumer_ref!=entry.consumer_ref{
            return Err(RepairVectorError::ConsumerMismatch);
        }
        let Some(proposal)=entry.proposal.as_ref() else{
            consumers.push(RepairVectorConsumerReceipt{
                consumer_ref:entry.consumer_ref.clone(),
                outcome:RepairConsumerOutcome::Unchecked,
                discharged_obligation_refs:vec![],
                new_obligation_refs:vec![],
                runtime_preservation_witness_ref:None,
                preservation_formally_verified:false,
            });
            continue;
        };
        let assessment=assess_bounded_repair(
            &entry.before,&entry.after,proposal)?;
        let outcome=if !assessment.newly_created_obligations.is_empty(){
            RepairConsumerOutcome::Regressed
        }else if assessment.strictly_improves_checked_debt{
            RepairConsumerOutcome::Improved
        }else if same_debt(&assessment){
            RepairConsumerOutcome::Preserved
        }else{
            RepairConsumerOutcome::Regressed
        };
        consumers.push(RepairVectorConsumerReceipt{
            consumer_ref:entry.consumer_ref.clone(),outcome,
            discharged_obligation_refs:assessment.discharged_obligation_refs,
            new_obligation_refs:assessment.newly_created_obligation_refs,
            runtime_preservation_witness_ref:
                Some(proposal.consumer_observation_preservation_ref.clone()),
            preservation_formally_verified:
                assessment.preservation_formally_verified,
        });
    }
    let checked=consumers.iter()
        .filter(|c|c.outcome!=RepairConsumerOutcome::Unchecked).count();
    let improved=consumers.iter()
        .filter(|c|c.outcome==RepairConsumerOutcome::Improved).count();
    let regressed=consumers.iter()
        .filter(|c|c.outcome==RepairConsumerOutcome::Regressed).count();
    let unchecked=consumers.iter()
        .filter(|c|c.outcome==RepairConsumerOutcome::Unchecked).count();
    let no_regressions=regressed==0;
    Ok(RepairVectorReceipt{
        repair_vector_ref:repair_vector_ref.into(),
        consumers,checked_consumer_count:checked,
        improved_consumer_count:improved,
        regressed_consumer_count:regressed,
        unchecked_consumer_count:unchecked,
        no_checked_regressions:no_regressions,
        at_least_one_checked_improvement:improved>0,
        acceptable_for_further_review:no_regressions&&improved>0,
        applies_external_edit:false,
        creates_semantic_authority:false,
    })
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::{ComparisonFinding,ComparisonResidual,ResidualKind};
    fn cmp(consumer:&str,refs:&[&str])->RelationalComparison{
        RelationalComparison{
            schema:"itir.relational-comparison.v1".into(),
            consumer_ref:consumer.into(),left_observation_ref:"a".into(),
            right_observation_ref:"b".into(),comparison_ref:format!("cmp:{consumer}"),
            finding:ComparisonFinding::PartialResidual,
            residuals:refs.iter().map(|r|ComparisonResidual{
                kind:ResidualKind::UnalignedType,left_ref:None,right_ref:None,
                obligation_ref:(*r).into(),
            }).collect(),used_alignment_witness_refs:vec![],
            role_type_evidence_refs:vec![],positive_support_refs:vec![],
            counter_support_refs:vec![],explicit_unknown_refs:vec![],
            creates_semantic_authority:false,merges_sources:false,
            proves_independence:false,claim_truth_promoted:false,
        }
    }
    fn proposal(id:&str)->BoundedRepairCandidate{
        BoundedRepairCandidate{
            repair_candidate_ref:id.into(),before_model_ref:"before".into(),
            after_model_ref:"after".into(),
            consumer_observation_preservation_ref:format!("preserve:{id}"),
            preservation_producer_ref:"producer".into(),
            rerun_receipt_ref:"rerun".into(),
            source_edit_authorized:false,applies_external_edit:false,
        }
    }
    #[test]fn one_improvement_plus_preservation_is_reviewable(){
        let entries=vec![
            RepairVectorEntry{consumer_ref:"q1".into(),before:cmp("q1",&["x","y"]),
                after:cmp("q1",&["y"]),proposal:Some(proposal("r1"))},
            RepairVectorEntry{consumer_ref:"q2".into(),before:cmp("q2",&["z"]),
                after:cmp("q2",&["z"]),proposal:Some(proposal("r2"))},
        ];
        let r=assess_repair_vector("repair-vector:1",&entries).unwrap();
        assert!(r.acceptable_for_further_review);
        assert_eq!(r.improved_consumer_count,1);
        assert_eq!(r.regressed_consumer_count,0);
    }
    #[test]fn regression_blocks_vector_acceptability(){
        let entries=vec![RepairVectorEntry{consumer_ref:"q1".into(),
            before:cmp("q1",&["x"]),after:cmp("q1",&["new"]),
            proposal:Some(proposal("r1"))}];
        let r=assess_repair_vector("repair-vector:2",&entries).unwrap();
        assert!(!r.acceptable_for_further_review);
        assert_eq!(r.regressed_consumer_count,1);
    }
}
