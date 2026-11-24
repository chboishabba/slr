//! Indexed, bounded native PNF owner engine.
//! - Owner is exact (semantic coordinate, fibre, type, structural signature).
//! - Global observation/dependency validity is tracked in reverse indexes.
//! - Only dirty owners are rebuilt, preserving original first-compatible order.
//! - No budget may split the canonical reduction of a single owner.
//! - An execution budget does not refute, hide, or admit semantic candidates.
//! - A checkpoint receipt is not yet a durable SQL checkpoint or identity.

use crate::{checked_numeric_proposal_owner,reduce_canonical,
    typed_canonical_sha256,DependencyMode,NativeManifest,
    NativeReduction,NativeWorkCounter,NumericPnfError};
use serde::{Deserialize,Serialize};
use serde_json::{json,Value};
use sha2::{Digest,Sha256};
use std::collections::{BTreeMap,BTreeSet};

#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Serialize,Deserialize)]
pub struct NumericOwnerKey {
    pub coordinate:String,
    pub fibre:String,
    pub factor_type:String,
    pub signature:String,
}
impl From<(String,String,String,String)> for NumericOwnerKey {
    fn from(k:(String,String,String,String))->Self{
        Self{coordinate:k.0,fibre:k.1,factor_type:k.2,signature:k.3}
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct NumericWorkBudget {
    pub max_owners:usize,
    pub max_proposals_scanned:usize,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct NumericOwnerWorkReceipt {
    pub schema:&'static str,
    pub manifest_sha256:String,
    pub epoch:u64,
    pub source_original_byte_count:usize,
    pub owners_reduced:usize,
    pub proposals_scanned:usize,
    pub candidate_comparisons:usize,
    pub dependency_edges_checked:usize,
    pub factors_emitted:usize,
    pub residuals_emitted:usize,
    pub pending_owner_count:usize,
    pub stopped_at_owner_boundary:bool,
    pub semantic_identity_promoted:bool,
    pub external_qid_paid:bool,
    pub current_state_ref:String,
}

#[derive(Debug)]
pub struct NumericOwnerRuntime {
    manifest:NativeManifest,
    proposals:BTreeMap<String,Value>,
    owners:BTreeMap<NumericOwnerKey,BTreeSet<String>>,
    observations:BTreeSet<String>,
    dependencies:BTreeSet<String>,
    observation_reverse:BTreeMap<String,BTreeSet<NumericOwnerKey>>,
    dependency_reverse:BTreeMap<String,BTreeSet<NumericOwnerKey>>,
    dirty:BTreeSet<NumericOwnerKey>,
    reduced:BTreeMap<NumericOwnerKey,NativeReduction>,
    epoch:u64,
}

impl NumericOwnerRuntime {
    /// Original source bytes are checked exactly once at the admission boundary.
    pub fn new(manifest:NativeManifest,bytes:&[u8])
        ->Result<Self,NumericPnfError> {
        manifest.verify_source_bytes(bytes)?;
        Ok(Self{
            manifest,proposals:BTreeMap::new(),
            owners:BTreeMap::new(),observations:BTreeSet::new(),
            dependencies:BTreeSet::new(),observation_reverse:BTreeMap::new(),
            dependency_reverse:BTreeMap::new(),dirty:BTreeSet::new(),
            reduced:BTreeMap::new(),epoch:0,
        })
    }

    pub fn manifest(&self)->&NativeManifest{&self.manifest}
    pub fn pending_owner_count(&self)->usize{self.dirty.len()}
    pub fn is_reduction_current(&self)->bool{self.dirty.is_empty()}

    fn owner_key(&self,p:&Value)->Result<NumericOwnerKey,NumericPnfError>{
        Ok(checked_numeric_proposal_owner(&self.manifest,p)?.into())
    }

    pub fn ingest_proposal(&mut self,p:Value)->Result<(),NumericPnfError>{
        let owner=self.owner_key(&p)?;
        let ref_id=p.get("proposal_ref").and_then(Value::as_str)
            .ok_or(NumericPnfError::MissingField("proposal_ref"))?.to_owned();
        if let Some(old)=self.proposals.get(&ref_id) {
            if old!=&p {return Err(NumericPnfError::ConflictingProposal);}
            return Ok(());
        }
        for (field,index) in [
            ("input_observation_refs",true),
            ("dependency_factor_refs",false)
        ] {
            let values=p.get(field).and_then(Value::as_array)
                .ok_or(NumericPnfError::InvalidField("input_observation_refs"))?;
            for value in values {
                let id=value.as_str()
                    .ok_or(NumericPnfError::InvalidField("input_observation_refs"))?
                    .to_owned();
                if index {
                    self.observation_reverse.entry(id).or_default().insert(owner.clone());
                } else {
                    self.dependency_reverse.entry(id).or_default().insert(owner.clone());
                }
            }
        }
        self.proposals.insert(ref_id.clone(),p);
        self.owners.entry(owner.clone()).or_default().insert(ref_id);
        self.dirty.insert(owner);
        self.epoch+=1;
        Ok(())
    }

    fn unset_reverse(&mut self,owner:&NumericOwnerKey,p:&Value){
        for (key,index) in [("input_observation_refs",true),
            ("dependency_factor_refs",false)] {
            let ids=p.get(key).and_then(Value::as_array);
            if let Some(ids)=ids{
                for value in ids{
                    if let Some(id)=value.as_str(){
                        let map=if index{&mut self.observation_reverse}
                            else{&mut self.dependency_reverse};
                        if let Some(owners)=map.get_mut(id){
                            owners.remove(owner);
                            if owners.is_empty(){map.remove(id);}
                        }
                    }
                }
            }
        }
    }

    /// Removing a proposal cannot be completed by treating the old factor
    /// as present; first-compatible grouping must be canonically rebuilt.
    pub fn withdraw_proposal(&mut self,ref_id:&str)->Result<bool,NumericPnfError>{
        let Some(p)=self.proposals.remove(ref_id) else{return Ok(false)};
        let owner=self.owner_key(&p)?;
        self.unset_reverse(&owner,&p);
        if let Some(keys)=self.owners.get_mut(&owner){
            keys.remove(ref_id);
            if keys.is_empty(){self.owners.remove(&owner);}
        }
        self.dirty.insert(owner);
        self.epoch+=1;
        Ok(true)
    }

    pub fn set_observation_available(&mut self,id:&str,yes:bool){
        let changed=if yes{self.observations.insert(id.into())}
                    else{self.observations.remove(id)};
        if changed{
            if let Some(owners)=self.observation_reverse.get(id){
                self.dirty.extend(owners.iter().cloned());
            }
            self.epoch+=1;
        }
    }

    pub fn set_dependency_available(&mut self,id:&str,yes:bool){
        let changed=if yes{self.dependencies.insert(id.into())}
                    else{self.dependencies.remove(id)};
        if changed{
            if let Some(owners)=self.dependency_reverse.get(id){
                self.dirty.extend(owners.iter().cloned());
            }
            self.epoch+=1;
        }
    }

    pub fn read_current_owner(&self,owner:&NumericOwnerKey)
        ->Option<&NativeReduction>{
        if self.dirty.contains(owner){None}else{self.reduced.get(owner)}
    }

    /// No partial owner execution: max_proposals_scanned is checked before
    /// each owner. If it cannot fit, leave the owner pending and return a
    /// bounded receipt rather than doing an invisible full-document scan.
    pub fn rebuild_dirty(&mut self,budget:NumericWorkBudget)
        ->Result<NumericOwnerWorkReceipt,NumericPnfError>{
        let mut owner_count=0;
        let mut scan_count=0;
        let mut group_count=0;
        let mut dep_count=0;
        let mut factor_count=0;
        let mut residual_count=0;
        while owner_count<budget.max_owners{
            let Some(owner)=self.dirty.iter().next().cloned() else{break};
            let refs=self.owners.get(&owner).cloned().unwrap_or_default();
            if refs.len()>budget.max_proposals_scanned.saturating_sub(scan_count){
                break;
            }
            let items=refs.iter().map(|id|self.proposals.get(id).unwrap().clone())
                .collect::<Vec<_>>();
            let next=reduce_canonical(
                &self.manifest,&items,&self.observations,&self.dependencies,
                DependencyMode::Strict
            )?;
            let previous=self.reduced.insert(owner.clone(),next.clone());
            self.dirty.remove(&owner);
            scan_count+=items.len();owner_count+=1;
            group_count+=next.work.candidate_group_comparisons;
            dep_count+=next.work.dependency_edges_checked;
            factor_count+=next.work.factors_emitted;
            residual_count+=next.work.residuals_emitted;

            // Exact source-level reverse dependency edge propagation.
            let old_factors=previous.as_ref().map(|r|r.factors.iter()
                .filter_map(|v|v.get("factor_ref").and_then(Value::as_str))
                .map(str::to_owned).collect::<BTreeSet<_>>())
                .unwrap_or_default();
            let new_factors=next.factors.iter()
                .filter_map(|v|v.get("factor_ref").and_then(Value::as_str))
                .map(str::to_owned).collect::<BTreeSet<_>>();
            for changed in old_factors.symmetric_difference(&new_factors){
                if let Some(affected)=self.dependency_reverse.get(changed) {
                    self.dirty.extend(affected.iter().cloned());
                }
            }
        }
        let digest=self.current_state_ref()?;
        Ok(NumericOwnerWorkReceipt{
            schema:"slr-native-numeric-owner-work-v1",
            manifest_sha256:self.manifest.canonical_content_sha256.clone(),
            epoch:self.epoch,
            source_original_byte_count:self.manifest.source_byte_length,
            owners_reduced:owner_count,proposals_scanned:scan_count,
            candidate_comparisons:group_count,dependency_edges_checked:dep_count,
            factors_emitted:factor_count,residuals_emitted:residual_count,
            pending_owner_count:self.dirty.len(),
            stopped_at_owner_boundary:!self.dirty.is_empty(),
            semantic_identity_promoted:false,external_qid_paid:false,
            current_state_ref:digest,
        })
    }

    /// This is a read-only receipt, not a durable checkpoint file.
    /// Persistent restart must replay the original immutable source deltas.
    pub fn current_state_ref(&self)->Result<String,NumericPnfError>{
        let rows=self.reduced.iter().map(|(k,r)|json!({
            "coordinate":k.coordinate,"fibre":k.fibre,
            "factor_type":k.factor_type,"signature":k.signature,
            "factors":r.factors,"residuals":r.residuals,
        })).collect::<Vec<_>>();
        let pending=self.dirty.iter().map(|o|json!({
            "coordinate":o.coordinate,"fibre":o.fibre,
            "factor_type":o.factor_type,"signature":o.signature,
        })).collect::<Vec<_>>();
        typed_canonical_sha256(&json!({
            "manifest":self.manifest,
            "epoch":self.epoch,
            "reduced":rows,
            "pending":pending,
        }))
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use serde_json::json;

    fn manifest()->NativeManifest{
        NativeManifest{
            document_ref:"doc:exact".into(),source_revision_ref:"rev:exact".into(),
            canonical_content_sha256:format!("{:x}",Sha256::digest(b"abc")),
            source_byte_length:3,source_codepoint_length:3,
            coordinate_contract_ref:"unicode-codepoints:v1".into(),
            parser_run_ref:"run:exact".into(),rule_set_revision:"rule:v1".into(),
        }
    }
    fn make_proposal(coord:&str,subj:&str,obs:&str)->Value {
        let mut p=json!({
            "document_ref":"doc:exact","source_revision_ref":"rev:exact",
            "semantic_coordinate_ref":coord,"scope_ref":coord,
            "statement_role":"main","coordinate_kind":"object",
            "fibre_kind":"hypothesis","derivation_role":"support",
            "factor_type_ref":"role","source_span_refs":[],
            "input_observation_refs":[obs],"dependency_factor_refs":[],
            "structural_signature":"signature","role_bindings":{"subject":subj},
            "qualifier_state":{},"producer_contract":"integrated-semantic-producer:v0_1",
            "producer_scope":"integrated","operation_contract":"test",
            "declaration_revision":"v1","ontology_axis_refs":[],
            "transport_refs":[],"support_state":"candidate",
            "confidence":null,"assumptions":[],"coverage_requirements":[],
            "candidate_payload":{},"residuals":[]
        });
        let id=typed_canonical_sha256(&p).unwrap();
        p["proposal_ref"]=Value::String(format!("factor-proposal:{id}"));
        p
    }

    #[test]
    fn owner_budget_never_splits_first_compatible_group(){
        let mut runtime=NumericOwnerRuntime::new(manifest(),b"abc").unwrap();
        runtime.ingest_proposal(make_proposal("c","Bush","o")).unwrap();
        runtime.ingest_proposal(make_proposal("c","Reagan","o")).unwrap();
        runtime.set_observation_available("o",true);
        let early=runtime.rebuild_dirty(NumericWorkBudget{
            max_owners:10,max_proposals_scanned:1}).unwrap();
        assert_eq!(early.owners_reduced,0);
        assert_eq!(early.pending_owner_count,1);
        let full=runtime.rebuild_dirty(NumericWorkBudget{
            max_owners:10,max_proposals_scanned:2}).unwrap();
        assert_eq!(full.owners_reduced,1);
        assert_eq!(full.pending_owner_count,0);
        assert_eq!(full.residuals_emitted,1);
    }

    #[test]
    fn no_op_replay_does_no_semantic_work(){
        let mut runtime=NumericOwnerRuntime::new(manifest(),b"abc").unwrap();
        runtime.ingest_proposal(make_proposal("c","Bush","o")).unwrap();
        runtime.set_observation_available("o",true);
        runtime.rebuild_dirty(NumericWorkBudget{
            max_owners:10,max_proposals_scanned:10}).unwrap();
        let idle=runtime.rebuild_dirty(NumericWorkBudget{
            max_owners:10,max_proposals_scanned:10}).unwrap();
        assert_eq!(idle.proposals_scanned,0);
        assert_eq!(idle.candidate_comparisons,0);
        assert_eq!(idle.owners_reduced,0);
    }

    #[test]
    fn unavailable_observation_withholds_factor_but_keeps_residual(){
        let mut runtime=NumericOwnerRuntime::new(manifest(),b"abc").unwrap();
        runtime.ingest_proposal(make_proposal("c","Bush","o")).unwrap();
        let before=runtime.rebuild_dirty(NumericWorkBudget{
            max_owners:10,max_proposals_scanned:10}).unwrap();
        assert_eq!(before.factors_emitted,0);
        assert_eq!(before.residuals_emitted,1);
        runtime.set_observation_available("o",true);
        let after=runtime.rebuild_dirty(NumericWorkBudget{
            max_owners:10,max_proposals_scanned:10}).unwrap();
        assert_eq!(after.factors_emitted,1);
    }

    #[test]
    fn distinct_owner_not_rebuilt_after_unrelated_change(){
        let mut runtime=NumericOwnerRuntime::new(manifest(),b"abc").unwrap();
        runtime.ingest_proposal(make_proposal("first","Bush","one")).unwrap();
        runtime.ingest_proposal(make_proposal("second","Reagan","two")).unwrap();
        runtime.set_observation_available("one",true);
        runtime.set_observation_available("two",true);
        runtime.rebuild_dirty(NumericWorkBudget{
            max_owners:10,max_proposals_scanned:10}).unwrap();
        runtime.set_observation_available("one",false);
        let after=runtime.rebuild_dirty(NumericWorkBudget{
            max_owners:10,max_proposals_scanned:10}).unwrap();
        assert_eq!(after.owners_reduced,1);
        assert_eq!(after.proposals_scanned,1);
    }

    #[test]
    fn forged_document_is_refused_at_initial_source_boundary(){
        let mut wrong=manifest();
        wrong.canonical_content_sha256="0".repeat(64);
        assert!(NumericOwnerRuntime::new(wrong,b"abc").is_err());
    }
}
