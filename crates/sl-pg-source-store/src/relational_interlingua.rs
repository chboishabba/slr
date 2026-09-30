//! ITIR generic relational interlingua: context-indexed, residual-bearing
//! observations and consumer-specific compatibility. No domain-specific
//! equivalence rules. Ontology types constrain *queries*, not parser output.
//!
//! Domain sources supply existing PNF role, relation and source-locator refs;
//! this module cannot create admission, copy evidence, causal independence,
//! class identity or truth. Both positive and negative support are retained.

use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const RELATIONAL_INTERLINGUA_SCHEMA: &str = "itir.relational-comparison.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum SourceFamily {
    Wikidata, Wikipedia, Legal, Biomedical, Transcript, Chat,
    StructuredKnowledge, AnimalObservation, Other,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalObservation {
    pub observation_ref: String,
    pub source_revision_ref: String,
    pub source_family: SourceFamily,
    pub source_span_ref: String,
    pub parser_or_producer_ref: String,
    pub predicate_candidate_ref: String,
    /// Roles are NOT sorted or flattened to a set. Repeated roles and
    /// distinct participants are permitted (agent, patient, instrument...).
    pub role_bindings: Vec<RoleBinding>,
    pub context: ObservationContext,
    pub candidate_type_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub polarity: Polarity,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleBinding {
    pub role_ref: String,
    pub filler_candidate_ref: String,
    /// Source-local index disambiguates e.g. two co-agents.
    pub occurrence: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ObservationContext {
    pub scope_ref: Option<String>,
    pub time_ref: Option<String>,
    pub modality_ref: Option<String>,
    pub quantifier_ref: Option<String>,
    pub attribution_ref: Option<String>,
    pub ontology_ref: Option<String>,
    pub language_ref: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Polarity { Supports, Counters, Undetermined }

/// One explicitly selected *consumer fibre*. An operation licences some
/// coordinates but never silently identifies dissimilar predicates/types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalConsumer {
    pub consumer_ref: String,
    pub permitted_source_families: Vec<SourceFamily>,
    pub required_roles: Vec<String>,
    pub compare_scope: bool,
    pub compare_time: bool,
    pub compare_modality: bool,
    pub compare_quantifier: bool,
    pub compare_attribution: bool,
    pub compare_ontology: bool,
    pub compare_language: bool,
    /// Explicit reviewed/licensed alignment pairs. No lexical fallback.
    pub predicate_alignments: Vec<LicensedAlignment>,
    pub filler_alignments: Vec<LicensedAlignment>,
    pub type_alignments: Vec<LicensedAlignment>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicensedAlignment {
    pub left_ref: String,
    pub right_ref: String,
    pub witness_ref: String,
    pub consumer_ref: String,
    pub licence_ref: String,
    pub direction: AlignmentDirection,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum AlignmentDirection { Symmetric, LeftToRight }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ComparisonFinding {
    ExactCandidateShape,
    LicensedCompatible,
    CompatibleQualification,
    PolarityConflictCandidate,
    PartialResidual,
    MissingTypedMeet,
    Undetermined,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ResidualKind {
    UnalignedPredicate,
    UnalignedRoleFiller,
    MissingRequiredRole,
    UnalignedType,
    ScopeMismatch,
    TimeMismatch,
    ModalityMismatch,
    QuantifierMismatch,
    AttributionMismatch,
    OntologyMismatch,
    LanguageMismatch,
    UnresolvedPolarity,
    MissingProvenance,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparisonResidual {
    pub kind: ResidualKind,
    pub left_ref: Option<String>,
    pub right_ref: Option<String>,
    pub obligation_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalComparison {
    pub schema: String,
    pub consumer_ref: String,
    pub left_observation_ref: String,
    pub right_observation_ref: String,
    pub comparison_ref: String,
    pub finding: ComparisonFinding,
    pub residuals: Vec<ComparisonResidual>,
    pub used_alignment_witness_refs: Vec<String>,
    pub positive_support_refs: Vec<String>,
    pub counter_support_refs: Vec<String>,
    pub explicit_unknown_refs: Vec<String>,
    pub creates_semantic_authority: bool,
    pub merges_sources: bool,
    pub proves_independence: bool,
    pub claim_truth_promoted: bool,
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RelationalComparisonError {
    #[error("missing or reused source/observation identity")]
    InvalidIdentity,
    #[error("candidate producer tried to promote truth or semantic authority")]
    Promotion,
    #[error("observation's source family is not permitted by consumer fibre")]
    OutsideConsumer,
    #[error("ambiguous or missing role occurrences")]
    InvalidRoles,
    #[error("alignment licence is missing or improperly scoped")]
    InvalidLicence,
}
fn present(s:&str)->bool { !s.trim().is_empty() }
fn valid_observation(o:&RelationalObservation)->Result<(),RelationalComparisonError> {
    if !present(&o.observation_ref) || !present(&o.source_revision_ref)
        || !present(&o.source_span_ref) || !present(&o.parser_or_producer_ref)
        || !present(&o.predicate_candidate_ref) {
        return Err(RelationalComparisonError::InvalidIdentity);
    }
    if !o.candidate_only || o.creates_semantic_authority || o.claim_truth_promoted {
        return Err(RelationalComparisonError::Promotion);
    }
    let mut roles=BTreeSet::new();
    for role in &o.role_bindings {
        if !present(&role.role_ref) || !present(&role.filler_candidate_ref)
            || !roles.insert((&role.role_ref,role.occurrence)) {
            return Err(RelationalComparisonError::InvalidRoles);
        }
    }
    Ok(())
}
fn alignment<'a>(left:&str,right:&str,c:&RelationalConsumer,
    list:&'a [LicensedAlignment])->Option<&'a LicensedAlignment> {
    list.iter().find(|a| {
        a.consumer_ref==c.consumer_ref && present(&a.witness_ref)
            && present(&a.licence_ref) && present(&a.left_ref)
            && present(&a.right_ref)
            && ((a.left_ref==left && a.right_ref==right)
                || (a.direction==AlignmentDirection::Symmetric
                    && a.left_ref==right && a.right_ref==left))
    })
}
fn align_or_equal(left:&str,right:&str,c:&RelationalConsumer,
    list:&[LicensedAlignment],witnesses:&mut BTreeSet<String>)->bool {
    if left==right {return true;}
    if let Some(a)=alignment(left,right,c,list) {
        witnesses.insert(a.witness_ref.clone());
        return true;
    }
    false
}
fn field_delta(
    left:&Option<String>,right:&Option<String>, enabled:bool,
    kind:ResidualKind,residuals:&mut Vec<ComparisonResidual>,
) {
    if enabled && left!=right {
        residuals.push(ComparisonResidual {
            kind,
            left_ref:left.clone(),right_ref:right.clone(),
            obligation_ref:"consumer:context-compatibility-unpaid".into(),
        });
    }
}
fn role_index(o:&RelationalObservation)->BTreeMap<(&str,u32),&str> {
    o.role_bindings.iter()
        .map(|r|((r.role_ref.as_str(),r.occurrence),r.filler_candidate_ref.as_str()))
        .collect()
}
fn hash(parts:&[&str])->String {
    let mut h=Sha256::new();
    for p in parts {
        h.update((p.len() as u64).to_be_bytes());
        h.update(p.as_bytes());
    }
    format!("relation-comparison:sha256:{:x}",h.finalize())
}

/// Consumer-indexed, bilateral comparison. Missingness survives; a
/// contradictory report is *not* eliminated from the candidate fibre.
/// Equal QIDs/text do not establish independently supported equivalence.
pub fn compare_relational_observations(
    left:&RelationalObservation,right:&RelationalObservation,
    consumer:&RelationalConsumer,
)->Result<RelationalComparison,RelationalComparisonError> {
    valid_observation(left)?;
    valid_observation(right)?;
    if !present(&consumer.consumer_ref)
        || left.observation_ref==right.observation_ref
        || (left.source_revision_ref==right.source_revision_ref
            && left.source_span_ref==right.source_span_ref)
    {return Err(RelationalComparisonError::InvalidIdentity);}
    if !consumer.permitted_source_families.contains(&left.source_family)
        || !consumer.permitted_source_families.contains(&right.source_family)
    {return Err(RelationalComparisonError::OutsideConsumer);}
    for a in consumer.predicate_alignments.iter()
        .chain(&consumer.filler_alignments).chain(&consumer.type_alignments) {
        if a.consumer_ref!=consumer.consumer_ref || !present(&a.witness_ref)
            || !present(&a.licence_ref) || !present(&a.left_ref)
            || !present(&a.right_ref) {
            return Err(RelationalComparisonError::InvalidLicence);
        }
    }
    let mut residuals=vec![];
    let mut witnesses=BTreeSet::new();
    if !align_or_equal(&left.predicate_candidate_ref,&right.predicate_candidate_ref,
        consumer,&consumer.predicate_alignments,&mut witnesses) {
        residuals.push(ComparisonResidual {
            kind:ResidualKind::UnalignedPredicate,
            left_ref:Some(left.predicate_candidate_ref.clone()),
            right_ref:Some(right.predicate_candidate_ref.clone()),
            obligation_ref:"predicate-alignment-review".into(),
        });
    }
    let l=role_index(left);
    let r=role_index(right);
    let keys=l.keys().chain(r.keys()).copied().collect::<BTreeSet<_>>();
    let required=consumer.required_roles.iter().map(String::as_str)
        .collect::<BTreeSet<_>>();
    for role in required {
        if !l.keys().any(|(name,_)|*name==role)
            || !r.keys().any(|(name,_)|*name==role) {
            residuals.push(ComparisonResidual {
                kind:ResidualKind::MissingRequiredRole,
                left_ref:l.keys().find(|(name,_)|*name==role)
                    .map(|(_,occ)|format!("{role}:{occ}")),
                right_ref:r.keys().find(|(name,_)|*name==role)
                    .map(|(_,occ)|format!("{role}:{occ}")),
                obligation_ref:format!("role:{role}"),
            });
        }
    }
    for key in keys {
        match (l.get(&key),r.get(&key)) {
            (Some(&a),Some(&b)) if align_or_equal(a,b,consumer,
                &consumer.filler_alignments,&mut witnesses) => {},
            (a,b)=>residuals.push(ComparisonResidual {
                kind:ResidualKind::UnalignedRoleFiller,
                left_ref:a.map(|value|(*value).into()),
                right_ref:b.map(|value|(*value).into()),
                obligation_ref:format!("role:{}:{}",key.0,key.1),
            }),
        }
    }
    // A missing type in either source is not a type error, and shared
    // observed members are never enough to establish class/set identity.
    let left_types=left.candidate_type_refs.iter().map(String::as_str)
        .collect::<BTreeSet<_>>();
    let right_types=right.candidate_type_refs.iter().map(String::as_str)
        .collect::<BTreeSet<_>>();
    for t in left_types.difference(&right_types) {
        if !right_types.iter().any(|u|align_or_equal(t,u,consumer,
            &consumer.type_alignments,&mut witnesses)) {
            residuals.push(ComparisonResidual {
                kind:ResidualKind::UnalignedType,left_ref:Some((*t).into()),
                right_ref:None,obligation_ref:"type-contract-compatibility".into(),
            });
        }
    }
    for t in right_types.difference(&left_types) {
        if !left_types.iter().any(|u|align_or_equal(u,t,consumer,
            &consumer.type_alignments,&mut witnesses)) {
            residuals.push(ComparisonResidual {
                kind:ResidualKind::UnalignedType,left_ref:None,
                right_ref:Some((*t).into()),
                obligation_ref:"type-contract-compatibility".into(),
            });
        }
    }
    let a=&left.context;
    let b=&right.context;
    field_delta(&a.scope_ref,&b.scope_ref,consumer.compare_scope,
        ResidualKind::ScopeMismatch,&mut residuals);
    field_delta(&a.time_ref,&b.time_ref,consumer.compare_time,
        ResidualKind::TimeMismatch,&mut residuals);
    field_delta(&a.modality_ref,&b.modality_ref,consumer.compare_modality,
        ResidualKind::ModalityMismatch,&mut residuals);
    field_delta(&a.quantifier_ref,&b.quantifier_ref,consumer.compare_quantifier,
        ResidualKind::QuantifierMismatch,&mut residuals);
    field_delta(&a.attribution_ref,&b.attribution_ref,consumer.compare_attribution,
        ResidualKind::AttributionMismatch,&mut residuals);
    field_delta(&a.ontology_ref,&b.ontology_ref,consumer.compare_ontology,
        ResidualKind::OntologyMismatch,&mut residuals);
    field_delta(&a.language_ref,&b.language_ref,consumer.compare_language,
        ResidualKind::LanguageMismatch,&mut residuals);
    if left.provenance_refs.is_empty() || right.provenance_refs.is_empty() {
        residuals.push(ComparisonResidual {
            kind:ResidualKind::MissingProvenance,
            left_ref:left.provenance_refs.first().cloned(),
            right_ref:right.provenance_refs.first().cloned(),
            obligation_ref:"source-provenance-receipt".into(),
        });
    }
    let opposite=matches!(
        (left.polarity,right.polarity),
        (Polarity::Supports,Polarity::Counters)
        |(Polarity::Counters,Polarity::Supports)
    );
    let unknown=left.polarity==Polarity::Undetermined
        ||right.polarity==Polarity::Undetermined;
    if unknown {
        residuals.push(ComparisonResidual {
            kind:ResidualKind::UnresolvedPolarity,
            left_ref:Some(left.observation_ref.clone()),
            right_ref:Some(right.observation_ref.clone()),
            obligation_ref:"evidence-polarity-unknown".into(),
        });
    }
    // A conflict requires structurally compatible predicates/role
    // bindings in this selected consumer fibre; context debt blocks an
    // automatic contradiction claim.
    let structural_debt=residuals.iter().any(|r|
        matches!(r.kind,ResidualKind::UnalignedPredicate
            |ResidualKind::UnalignedRoleFiller|ResidualKind::MissingRequiredRole
            |ResidualKind::UnalignedType));
    let finding=if unknown {
        ComparisonFinding::Undetermined
    } else if opposite && !structural_debt && residuals.is_empty() {
        ComparisonFinding::PolarityConflictCandidate
    } else if structural_debt {
        ComparisonFinding::MissingTypedMeet
    } else if !residuals.is_empty() {
        ComparisonFinding::PartialResidual
    } else if !witnesses.is_empty() {
        ComparisonFinding::LicensedCompatible
    } else if left.context!=right.context {
        ComparisonFinding::CompatibleQualification
    } else {
        ComparisonFinding::ExactCandidateShape
    };
    let support=[left,right].iter().filter(|o|o.polarity==Polarity::Supports)
        .map(|o|o.observation_ref.clone()).collect();
    let counters=[left,right].iter().filter(|o|o.polarity==Polarity::Counters)
        .map(|o|o.observation_ref.clone()).collect();
    let undetermined=[left,right].iter().filter(|o|o.polarity==Polarity::Undetermined)
        .map(|o|o.observation_ref.clone()).collect();
    Ok(RelationalComparison {
        schema:RELATIONAL_INTERLINGUA_SCHEMA.into(),
        consumer_ref:consumer.consumer_ref.clone(),
        left_observation_ref:left.observation_ref.clone(),
        right_observation_ref:right.observation_ref.clone(),
        comparison_ref:hash(&[&consumer.consumer_ref,&left.observation_ref,
            &right.observation_ref]),
        finding,residuals,
        used_alignment_witness_refs:witnesses.into_iter().collect(),
        positive_support_refs:support,counter_support_refs:counters,
        explicit_unknown_refs:undetermined,
        creates_semantic_authority:false,merges_sources:false,
        proves_independence:false,claim_truth_promoted:false,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn obs(id:&str,family:SourceFamily)->RelationalObservation {
        RelationalObservation {
            observation_ref:id.into(),source_revision_ref:format!("rev:{id}"),
            source_family:family,source_span_ref:"span:0".into(),
            parser_or_producer_ref:"pnf:producer".into(),
            predicate_candidate_ref:"walk".into(),
            role_bindings:vec![
                RoleBinding{role_ref:"agent".into(),filler_candidate_ref:"I".into(),occurrence:0},
                RoleBinding{role_ref:"patient".into(),filler_candidate_ref:"dog".into(),occurrence:0},
            ],context:ObservationContext::default(),
            candidate_type_refs:vec![],provenance_refs:vec!["receipt:source".into()],
            polarity:Polarity::Supports,candidate_only:true,
            creates_semantic_authority:false,claim_truth_promoted:false,
        }
    }
    fn consumer()->RelationalConsumer {
        RelationalConsumer{
            consumer_ref:"consumer:member-observation".into(),
            permitted_source_families:vec![
                SourceFamily::Wikipedia,SourceFamily::Wikidata,SourceFamily::Biomedical],
            required_roles:vec!["agent".into(),"patient".into()],
            compare_scope:true,compare_time:true,compare_modality:true,
            compare_quantifier:true,compare_attribution:true,
            compare_ontology:true,compare_language:false,
            predicate_alignments:vec![],filler_alignments:vec![],type_alignments:vec![],
        }
    }
    #[test] fn cross_family_same_shape_is_candidate_only() {
        let x=compare_relational_observations(
            &obs("a",SourceFamily::Wikipedia),&obs("b",SourceFamily::Biomedical),
            &consumer()).unwrap();
        assert_eq!(x.finding,ComparisonFinding::ExactCandidateShape);
        assert!(!x.merges_sources && !x.proves_independence && !x.claim_truth_promoted);
    }
    #[test] fn conflicting_context_blocks_contradiction() {
        let a=obs("a",SourceFamily::Wikipedia);
        let mut b=obs("b",SourceFamily::Biomedical);
        b.polarity=Polarity::Counters;
        b.context.time_ref=Some("1902".into());
        let r=compare_relational_observations(&a,&b,&consumer()).unwrap();
        assert_eq!(r.finding,ComparisonFinding::PartialResidual);
        assert!(r.residuals.iter().any(|x|x.kind==ResidualKind::TimeMismatch));
        assert_eq!(r.counter_support_refs,vec!["b"]);
    }
    #[test] fn licensed_predicate_transport_keeps_receipt() {
        let a=obs("a",SourceFamily::Wikipedia);
        let mut b=obs("b",SourceFamily::Biomedical);
        b.predicate_candidate_ref="ambulare".into();
        let mut c=consumer();
        c.predicate_alignments=vec![LicensedAlignment{
            left_ref:"walk".into(),right_ref:"ambulare".into(),
            witness_ref:"alignment:witness".into(),consumer_ref:c.consumer_ref.clone(),
            licence_ref:"consumer:licence".into(),direction:AlignmentDirection::Symmetric,
        }];
        let r=compare_relational_observations(&a,&b,&c).unwrap();
        assert_eq!(r.finding,ComparisonFinding::LicensedCompatible);
        assert_eq!(r.used_alignment_witness_refs,vec!["alignment:witness"]);
    }
    #[test] fn class_and_set_incidence_does_not_prove_type_identity() {
        let mut a=obs("a",SourceFamily::Wikidata);
        let mut b=obs("b",SourceFamily::Wikipedia);
        a.candidate_type_refs=vec!["class".into()];
        b.candidate_type_refs=vec!["set".into()];
        let r=compare_relational_observations(&a,&b,&consumer()).unwrap();
        assert_eq!(r.finding,ComparisonFinding::MissingTypedMeet);
        assert_eq!(r.residuals.len(),2);
    }
}
