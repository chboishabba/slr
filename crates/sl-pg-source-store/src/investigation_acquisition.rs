//! ITIR-INV-1 — proof-directed investigation/acquisition over REL residuals.
//!
//! Generalizes the already-existing ESD/Eskridge pattern:
//! residual -> acquisition obligation -> Pareto route frontier -> lawful
//! acquisition -> new source receipt -> selective reopening.
//!
//! This module never performs scraping, bypasses access control, establishes
//! source truth, or turns search priority into evidentiary weight.

use std::collections::{BTreeMap,BTreeSet,VecDeque};
use serde::{Deserialize,Serialize};
use thiserror::Error;
use crate::{ComparisonResidual,RelationalComparison};

#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum RecordAvailability {
    Present,
    NotLocated,
    KnownAbsent,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum AccessDisposition {
    Public,
    Authorized,
    RequiresAuthorization,
    ProhibitedOrUnavailable,
    Unknown,
}
impl AccessDisposition {
    pub fn executable(self)->bool{
        matches!(self,Self::Public|Self::Authorized)
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum EvidenceIndependence {
    KnownIndependent,
    KnownDependent,
    Unknown,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum DuplicateRelation {
    MetadataDuplicate,
    PublicationDuplicate,
    ReportFamilyDuplicate,
    SameEmpiricalStudy,
    DerivativeOfCommonSource,
    NoKnownDuplicateRelation,
    Unknown,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct AcquisitionObligation {
    pub obligation_ref:String,
    pub comparison_ref:String,
    pub residual_obligation_ref:String,
    pub source_revision_refs:Vec<String>,
    pub target_description:String,
    pub current_availability:RecordAvailability,
    pub authority_or_access_constraint_ref:String,
    pub dependency_target_refs:Vec<String>,
    pub candidate_only:bool,
    pub creates_semantic_authority:bool,
    pub claim_truth_promoted:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct AcquisitionRouteCandidate {
    pub route_ref:String,
    pub obligation_ref:String,
    pub route_description:String,
    pub source_locator_ref:String,
    pub access_disposition:AccessDisposition,
    pub authority_receipt_ref:Option<String>,
    pub provenance_genealogy_ref:String,
    pub independence:EvidenceIndependence,
    pub independence_receipt_ref:Option<String>,
    pub duplicate_relation:DuplicateRelation,
    /// Larger is better: expected discrimination/information gain.
    pub information_gain:u32,
    /// Larger is better: number/weight of explicit dependency consumers.
    pub dependency_closure_impact:u32,
    /// Larger is better: currently unpaid coordinates this route can address.
    pub residual_coverage:u32,
    /// Larger is better, but it does NOT itself establish independence.
    pub provenance_novelty:u32,
    /// Smaller is better: lawful/reviewer/resource burden.
    pub acquisition_cost:u32,
    pub axis_estimation_receipt_ref:String,
    pub candidate_only:bool,
    pub creates_acquisition_authority:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct AcquisitionPriorityReceipt {
    pub obligation_ref:String,
    pub frontier_route_refs:Vec<String>,
    pub executable_frontier_route_refs:Vec<String>,
    pub blocked_frontier_route_refs:Vec<String>,
    pub scalar_score_used:bool,
    pub creates_semantic_authority:bool,
    pub creates_screening_or_acquisition_decision:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct AcquisitionUpdate {
    pub obligation_ref:String,
    pub before:RecordAvailability,
    pub after:RecordAvailability,
    pub acquired_source_revision_ref:Option<String>,
    pub acquisition_receipt_ref:String,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct SelectiveReopeningReceipt {
    pub changed_source_revision_ref:String,
    pub directly_affected_refs:Vec<String>,
    pub transitively_affected_refs:Vec<String>,
    pub unrelated_refs_not_reopened:Vec<String>,
    pub dependency_graph_ref:String,
    pub creates_semantic_authority:bool,
}
#[derive(Debug,Error,PartialEq,Eq)]
pub enum InvestigationAcquisitionError {
    #[error("missing or promotion-bearing acquisition identity")]
    InvalidIdentity,
    #[error("only not-located evidence may create an open acquisition obligation")]
    WrongAvailability,
    #[error("route is not scoped to the requested obligation")]
    WrongObligation,
    #[error("authorized access requires a non-empty authority receipt")]
    MissingAuthorityReceipt,
    #[error("known independence/dependence requires a genealogy receipt")]
    MissingIndependenceReceipt,
    #[error("acquisition update is not a permitted availability transition")]
    InvalidUpdate,
}
fn valid(s:&str)->bool{!s.trim().is_empty()}

pub fn obligation_from_residual(
    comparison:&RelationalComparison,
    residual:&ComparisonResidual,
    target_description:&str,
    authority_or_access_constraint_ref:&str,
    dependency_target_refs:Vec<String>,
)->Result<AcquisitionObligation,InvestigationAcquisitionError>{
    if !valid(&comparison.comparison_ref)
        ||!valid(&residual.obligation_ref)
        ||!valid(target_description)
        ||!valid(authority_or_access_constraint_ref)
    {return Err(InvestigationAcquisitionError::InvalidIdentity);}
    let mut sources=vec![
        comparison.left_observation_ref.clone(),
        comparison.right_observation_ref.clone(),
    ];
    sources.sort();sources.dedup();
    Ok(AcquisitionObligation{
        obligation_ref:format!(
            "acquisition:{}:{}",
            comparison.comparison_ref,residual.obligation_ref),
        comparison_ref:comparison.comparison_ref.clone(),
        residual_obligation_ref:residual.obligation_ref.clone(),
        source_revision_refs:sources,
        target_description:target_description.into(),
        current_availability:RecordAvailability::NotLocated,
        authority_or_access_constraint_ref:authority_or_access_constraint_ref.into(),
        dependency_target_refs,
        candidate_only:true,creates_semantic_authority:false,
        claim_truth_promoted:false,
    })
}
fn validate_route(
    obligation:&AcquisitionObligation,route:&AcquisitionRouteCandidate,
)->Result<(),InvestigationAcquisitionError>{
    if !obligation.candidate_only||obligation.creates_semantic_authority
        ||obligation.claim_truth_promoted
        ||!route.candidate_only||route.creates_acquisition_authority
        ||!valid(&route.route_ref)||!valid(&route.route_description)
        ||!valid(&route.source_locator_ref)
        ||!valid(&route.provenance_genealogy_ref)
        ||!valid(&route.axis_estimation_receipt_ref)
    {return Err(InvestigationAcquisitionError::InvalidIdentity);}
    if obligation.current_availability!=RecordAvailability::NotLocated{
        return Err(InvestigationAcquisitionError::WrongAvailability);
    }
    if route.obligation_ref!=obligation.obligation_ref{
        return Err(InvestigationAcquisitionError::WrongObligation);
    }
    if route.access_disposition==AccessDisposition::Authorized
        &&route.authority_receipt_ref.as_deref().is_none_or(|s|!valid(s)){
        return Err(InvestigationAcquisitionError::MissingAuthorityReceipt);
    }
    if route.independence!=EvidenceIndependence::Unknown
        &&route.independence_receipt_ref.as_deref().is_none_or(|s|!valid(s)){
        return Err(InvestigationAcquisitionError::MissingIndependenceReceipt);
    }
    Ok(())
}
/// left weakly dominates right on every declared acquisition axis.
/// First four axes maximize; acquisition cost minimizes. No weighted sum.
pub fn weakly_dominates(
    left:&AcquisitionRouteCandidate,right:&AcquisitionRouteCandidate,
)->bool{
    left.information_gain>=right.information_gain
        &&left.dependency_closure_impact>=right.dependency_closure_impact
        &&left.residual_coverage>=right.residual_coverage
        &&left.provenance_novelty>=right.provenance_novelty
        &&left.acquisition_cost<=right.acquisition_cost
}
fn strictly_better_some_axis(
    left:&AcquisitionRouteCandidate,right:&AcquisitionRouteCandidate,
)->bool{
    left.information_gain>right.information_gain
        ||left.dependency_closure_impact>right.dependency_closure_impact
        ||left.residual_coverage>right.residual_coverage
        ||left.provenance_novelty>right.provenance_novelty
        ||left.acquisition_cost<right.acquisition_cost
}
pub fn acquisition_pareto_frontier(
    obligation:&AcquisitionObligation,
    routes:&[AcquisitionRouteCandidate],
)->Result<AcquisitionPriorityReceipt,InvestigationAcquisitionError>{
    for route in routes{validate_route(obligation,route)?;}
    let mut frontier=Vec::new();
    for (i,candidate) in routes.iter().enumerate(){
        let dominated=routes.iter().enumerate().any(|(j,other)|
            i!=j&&weakly_dominates(other,candidate)
                &&strictly_better_some_axis(other,candidate));
        if !dominated{frontier.push(candidate);}
    }
    frontier.sort_by(|a,b|a.route_ref.cmp(&b.route_ref));
    let frontier_refs=frontier.iter().map(|r|r.route_ref.clone()).collect();
    let executable=frontier.iter().filter(|r|r.access_disposition.executable())
        .map(|r|r.route_ref.clone()).collect();
    let blocked=frontier.iter().filter(|r|!r.access_disposition.executable())
        .map(|r|r.route_ref.clone()).collect();
    Ok(AcquisitionPriorityReceipt{
        obligation_ref:obligation.obligation_ref.clone(),
        frontier_route_refs:frontier_refs,
        executable_frontier_route_refs:executable,
        blocked_frontier_route_refs:blocked,
        scalar_score_used:false,
        creates_semantic_authority:false,
        creates_screening_or_acquisition_decision:false,
    })
}
pub fn apply_acquisition_update(
    obligation:&AcquisitionObligation,update:&AcquisitionUpdate,
)->Result<(),InvestigationAcquisitionError>{
    if update.obligation_ref!=obligation.obligation_ref
        ||!valid(&update.acquisition_receipt_ref)
    {return Err(InvestigationAcquisitionError::WrongObligation);}
    let valid_transition=matches!(
        (update.before,update.after),
        (RecordAvailability::NotLocated,RecordAvailability::Present)
        |(RecordAvailability::NotLocated,RecordAvailability::KnownAbsent)
    );
    if !valid_transition||update.before!=obligation.current_availability{
        return Err(InvestigationAcquisitionError::InvalidUpdate);
    }
    if update.after==RecordAvailability::Present
        &&update.acquired_source_revision_ref.as_deref().is_none_or(|s|!valid(s)){
        return Err(InvestigationAcquisitionError::InvalidUpdate);
    }
    if update.after==RecordAvailability::KnownAbsent
        &&update.acquired_source_revision_ref.is_some(){
        return Err(InvestigationAcquisitionError::InvalidUpdate);
    }
    Ok(())
}

/// Runtime dependency graph is explicit. Only declared dependency paths reopen.
/// Correlation/similarity does not create edges.
pub fn selective_reopening(
    changed_source_revision_ref:&str,
    dependency_graph_ref:&str,
    edges:&[(String,String)],
    universe_refs:&[String],
)->Result<SelectiveReopeningReceipt,InvestigationAcquisitionError>{
    if !valid(changed_source_revision_ref)||!valid(dependency_graph_ref){
        return Err(InvestigationAcquisitionError::InvalidIdentity);
    }
    let mut adjacency:BTreeMap<String,Vec<String>>=BTreeMap::new();
    for (from,to) in edges{
        if !valid(from)||!valid(to){
            return Err(InvestigationAcquisitionError::InvalidIdentity);
        }
        adjacency.entry(from.clone()).or_default().push(to.clone());
    }
    let direct=adjacency.get(changed_source_revision_ref)
        .cloned().unwrap_or_default();
    let mut visited=BTreeSet::new();
    let mut queue=VecDeque::from(direct.clone());
    while let Some(node)=queue.pop_front(){
        if !visited.insert(node.clone()){continue;}
        if let Some(next)=adjacency.get(&node){
            queue.extend(next.iter().cloned());
        }
    }
    let direct_set=direct.iter().cloned().collect::<BTreeSet<_>>();
    let transitive=visited.iter().filter(|r|!direct_set.contains(*r))
        .cloned().collect::<Vec<_>>();
    let unrelated=universe_refs.iter()
        .filter(|r|*r!=changed_source_revision_ref&&!visited.contains(*r))
        .cloned().collect();
    Ok(SelectiveReopeningReceipt{
        changed_source_revision_ref:changed_source_revision_ref.into(),
        directly_affected_refs:direct,
        transitively_affected_refs:transitive,
        unrelated_refs_not_reopened:unrelated,
        dependency_graph_ref:dependency_graph_ref.into(),
        creates_semantic_authority:false,
    })
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::{ComparisonFinding,ResidualKind};
    fn comparison()->RelationalComparison{
        RelationalComparison{
            schema:"itir.relational-comparison.v1".into(),
            consumer_ref:"matter:investigation".into(),
            left_observation_ref:"source:a".into(),
            right_observation_ref:"source:b".into(),
            comparison_ref:"comparison:1".into(),
            finding:ComparisonFinding::PartialResidual,
            residuals:vec![],used_alignment_witness_refs:vec![],
            role_type_evidence_refs:vec![],positive_support_refs:vec![],
            counter_support_refs:vec![],explicit_unknown_refs:vec![],
            creates_semantic_authority:false,merges_sources:false,
            proves_independence:false,claim_truth_promoted:false,
        }
    }
    fn obligation()->AcquisitionObligation{
        obligation_from_residual(&comparison(),&ComparisonResidual{
            kind:ResidualKind::MissingProvenance,left_ref:None,right_ref:None,
            obligation_ref:"missing:original-record".into(),
        },"obtain original record","lawful-access:public-records",
        vec!["assessment:case".into()]).unwrap()
    }
    fn route(id:&str,gain:u32,cost:u32)->AcquisitionRouteCandidate{
        AcquisitionRouteCandidate{
            route_ref:id.into(),obligation_ref:obligation().obligation_ref,
            route_description:"lawful route".into(),
            source_locator_ref:format!("locator:{id}"),
            access_disposition:AccessDisposition::Public,
            authority_receipt_ref:None,
            provenance_genealogy_ref:format!("genealogy:{id}"),
            independence:EvidenceIndependence::Unknown,
            independence_receipt_ref:None,
            duplicate_relation:DuplicateRelation::Unknown,
            information_gain:gain,dependency_closure_impact:3,
            residual_coverage:2,provenance_novelty:2,acquisition_cost:cost,
            axis_estimation_receipt_ref:"estimate:reviewed".into(),
            candidate_only:true,creates_acquisition_authority:false,
        }
    }
    #[test]fn pareto_is_not_scalar_and_removes_strictly_dominated_route(){
        let o=obligation();
        let r=acquisition_pareto_frontier(&o,&[
            route("better",5,2),route("worse",4,3),route("cheap",2,1),
        ]).unwrap();
        assert!(!r.scalar_score_used);
        assert_eq!(r.frontier_route_refs,vec!["better","cheap"]);
    }
    #[test]fn unknown_access_remains_blocked_not_deleted(){
        let o=obligation();let mut r=route("unknown",5,1);
        r.access_disposition=AccessDisposition::Unknown;
        let p=acquisition_pareto_frontier(&o,&[r]).unwrap();
        assert_eq!(p.frontier_route_refs,vec!["unknown"]);
        assert!(p.executable_frontier_route_refs.is_empty());
        assert_eq!(p.blocked_frontier_route_refs,vec!["unknown"]);
    }
    #[test]fn known_absence_closes_only_exact_acquisition_branch(){
        let o=obligation();
        let update=AcquisitionUpdate{
            obligation_ref:o.obligation_ref.clone(),
            before:RecordAvailability::NotLocated,
            after:RecordAvailability::KnownAbsent,
            acquired_source_revision_ref:None,
            acquisition_receipt_ref:"receipt:known-absent".into(),
        };
        apply_acquisition_update(&o,&update).unwrap();
    }
    #[test]fn selective_reopening_uses_only_dependency_paths(){
        let r=selective_reopening("source:new","graph:1",&[
            ("source:new".into(),"assessment:a".into()),
            ("assessment:a".into(),"assessment:case".into()),
        ],&["assessment:a".into(),"assessment:case".into(),
            "assessment:unrelated".into()]).unwrap();
        assert_eq!(r.directly_affected_refs,vec!["assessment:a"]);
        assert_eq!(r.transitively_affected_refs,vec!["assessment:case"]);
        assert_eq!(r.unrelated_refs_not_reopened,vec!["assessment:unrelated"]);
    }
}
