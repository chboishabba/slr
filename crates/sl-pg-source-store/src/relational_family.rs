//! Compose existing REL and INV owners across a bounded family of original
//! source observations. This is a native candidate read/repair work surface,
//! not another ontology, truth authority, or a proof of all-live sufficiency.
use std::collections::{BTreeMap,BTreeSet};
use serde::{Serialize,Deserialize};
use sensiblaw_core::review_workstation::{ReviewItemKind,ReviewItem,ReviewStatus,ReviewAction};
use crate::review_workstation_store::{install_review_workstation_on_client,persist_review_projection_on_tx,ReviewWorkstationStoreError};
use sha2::{Digest,Sha256};
use thiserror::Error;
use postgres::{Client,GenericClient,NoTls};
use crate::workbench_projection::acceptance_control_store::load_relational_selection_on_client;
use crate::workbench_projection::persisted_relational_observation::{relational_observations_from_checked_batch_on_client,relational_observation_from_persisted_wikidata_on_client};
use crate::relational_interlingua_store::{RelationalSourceDigests,persist_relational_comparison_on_client};
use crate::investigation_acquisition_store::ensure_acquisition_obligation_on_client;
use crate::{
    AcceptanceObservationKind, AcceptanceObservationSelector, DatabaseConfig,
    RelationalObservation, RelationalConsumer, Polarity, ComparisonResidual,
    RelationalComparison, compare_relational_observations_strict, RelationalComparisonError,
    DurableRelationalComparison, DurableRelationalError, RELATIONAL_COMPARISON_SQL,
    STRICT_RELATIONAL_INTERLINGUA_SCHEMA,
    PersistedWikidataObservationRequest, PersistedRelationalObservationError,
    AcceptanceControlStoreError, AcquisitionObligation, InvestigationAcquisitionError,
    InvestigationStoreError, DurableAcquisitionQueue, obligation_from_residual,
    INVESTIGATION_ACQUISITION_SQL,
    SelectiveReopeningReceipt, selective_reopening,
};

#[derive(Debug, Error)]
pub enum RelationalFamilyError {
    #[error("family requires 2–10 distinct selectors, observations and source revisions")]
    InvalidFamily,
    #[error("typed selector missing a required coordinate")]
    InvalidSelector,
    #[error("explicit access constraint reference required; no acquisition authority inferred")]
    MissingAccessConstraint,
    #[error("existing durable result or acquisition obligation differs from current scope")]
    ChangedReplay,
    #[error(transparent)] Selection(#[from] AcceptanceControlStoreError),
    #[error(transparent)] Observation(#[from] PersistedRelationalObservationError),
    #[error(transparent)] Comparison(#[from] RelationalComparisonError),
    #[error(transparent)] Store(#[from] DurableRelationalError),
    #[error(transparent)] Investigation(#[from] InvestigationAcquisitionError),
    #[error(transparent)] InvestigationStore(#[from] InvestigationStoreError),
    #[error(transparent)] Pg(#[from] postgres::Error),
    #[error(transparent)] Review(#[from] ReviewWorkstationStoreError),
}

/// Classify the ordered SOURCE polarity pair, not truth or a reviewed ternary
/// warrant. All 9 states survive; (0,0) is not negative agreement.
#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize)]
#[serde(rename_all="snake_case")]
pub enum SourcePolarityClass {
    AgreeNegative, LeftNegativeRightUnknown, NegativeToPositive,
    LeftUnknownRightNegative, BothUnknown, LeftUnknownRightPositive,
    PositiveToNegative, LeftPositiveRightUnknown, AgreePositive,
}
pub fn classify_source_polarity(left:Polarity,right:Polarity)->SourcePolarityClass {
    use Polarity::*;
    use SourcePolarityClass::*;
    match (left,right) {
        (Counters,Counters)=>AgreeNegative,
        (Counters,Undetermined)=>LeftNegativeRightUnknown,
        (Counters,Supports)=>NegativeToPositive,
        (Undetermined,Counters)=>LeftUnknownRightNegative,
        (Undetermined,Undetermined)=>BothUnknown,
        (Undetermined,Supports)=>LeftUnknownRightPositive,
        (Supports,Counters)=>PositiveToNegative,
        (Supports,Undetermined)=>LeftPositiveRightUnknown,
        (Supports,Supports)=>AgreePositive,
    }
}
pub fn source_polarity_pair(class:SourcePolarityClass)->(Polarity,Polarity){
    use Polarity::*;
    use SourcePolarityClass::*;
    match class {
        AgreeNegative=>(Counters,Counters),
        LeftNegativeRightUnknown=>(Counters,Undetermined),
        NegativeToPositive=>(Counters,Supports),
        LeftUnknownRightNegative=>(Undetermined,Counters),
        BothUnknown=>(Undetermined,Undetermined),
        LeftUnknownRightPositive=>(Undetermined,Supports),
        PositiveToNegative=>(Supports,Counters),
        LeftPositiveRightUnknown=>(Supports,Undetermined),
        AgreePositive=>(Supports,Supports),
    }
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct SelectedRelationalSource {
    pub selector_ref:String,
    pub observation:RelationalObservation,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct DirectionalRelationalComparison {
    pub left_selector_ref:String,
    pub right_selector_ref:String,
    pub source_polarity_class:SourcePolarityClass,
    /// Missing required context on both sides is not parity merely because
    /// the legacy comparison treats equal optional values as equal shape.
    pub unobserved_required_context_axes:Vec<String>,
    pub comparison:RelationalComparison,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct FamilyAcquisitionDemand {
    pub obligation:AcquisitionObligation,
    /// One existing acquisition key may own several exact residuals. Never
    /// reduce those to one arbitrary representative coordinate.
    pub exact_residuals:Vec<ComparisonResidual>,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct RelationalFamilyProjection {
    pub schema:&'static str,
    pub consumer_ref:String,
    pub sources:Vec<SelectedRelationalSource>,
    pub directed_comparisons:Vec<DirectionalRelationalComparison>,
    pub acquisition_demands:Vec<FamilyAcquisitionDemand>,
    pub potential_reopening_by_source:Vec<SelectiveReopeningReceipt>,
    /// The common part of the exact OBSERVED coordinate inventory, not a
    /// cross-language or world-semantic equivalence certificate.
    pub observed_coordinate_meet:Vec<ObservedCoordinateFibre>,
    pub source_sign_ledger:Vec<SourceSignEntry>,
    pub synthesis_candidates:Vec<FamilySynthesisColumn>,
    pub candidate_only:bool,
    pub all_live_reading_coverage_paid:bool,
    pub formal_consumer_equivalence_paid:bool,
    pub source_independence_paid:bool,
    pub creates_edit_authority:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct PersistedRelationalFamily {
    pub projection:RelationalFamilyProjection,
    pub durable_comparisons:Vec<DurableRelationalComparison>,
    pub durable_acquisition_queues:Vec<DurableAcquisitionQueue>,
    pub residual_review_items:Vec<FamilyReviewProjection>,
    /// Existing queues are reloaded; no fabricated routes or access grants.
    pub creates_access_authority:bool,
    pub applies_external_edit:bool,
}

/// Typed native observable: exact equality only, never string-based entity
/// resolution. The source observation itself remains the inverse carrier.
#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Serialize,Deserialize)]
pub struct ObservedCoordinate {
    pub axis:String,
    pub slot:String,
    pub occurrence:u32,
    pub value_ref:String,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct ObservedCoordinateFibre {
    pub coordinate:ObservedCoordinate,
    pub present_in:Vec<String>,
    pub not_recorded_in:Vec<String>,
}
fn observed_coordinates(o:&RelationalObservation)->BTreeSet<ObservedCoordinate>{
    let mut out=BTreeSet::new();
    let mut put=|axis:&str,slot:&str,occurrence:u32,value:&str|{
        if !value.trim().is_empty(){out.insert(ObservedCoordinate{
            axis:axis.into(),slot:slot.into(),occurrence,value_ref:value.into(),
        });}
    };
    put("predicate","predicate",0,&o.predicate_candidate_ref);
    for role in &o.role_bindings{
        put("role",&role.role_ref,role.occurrence,&role.filler_candidate_ref);
    }
    for typ in &o.role_type_hypotheses{
        put("role_type",&typ.role_ref,typ.occurrence,&typ.candidate_type_ref);
    }
    for typ in &o.candidate_type_refs{put("candidate_type","type",0,typ);}
    let c=&o.context;
    for (slot,value) in [("scope",&c.scope_ref),("time",&c.time_ref),
        ("modality",&c.modality_ref),("quantifier",&c.quantifier_ref),
        ("attribution",&c.attribution_ref),("ontology",&c.ontology_ref),
        ("language",&c.language_ref)]{
        if let Some(value)=value{put("context",slot,0,value);}
    }
    out
}

/// A lossless incidence transpose of the selected observations, indexed by
/// their immutable selector refs. A common coordinate is present in every
/// selected row; absent means not recorded, NEVER semantic refutation.
/// This is the finite source-observer specialization of
/// ConsumerDescentMinimalObserverExact, not a live semantic enumerator.
pub fn observed_family_meet(sources:&[SelectedRelationalSource])->Vec<ObservedCoordinateFibre>{
    let universe=sources.iter().map(|s|s.selector_ref.clone()).collect::<BTreeSet<_>>();
    let mut incidence=BTreeMap::<ObservedCoordinate,BTreeSet<String>>::new();
    for s in sources{
        for coordinate in observed_coordinates(&s.observation){
            incidence.entry(coordinate).or_default().insert(s.selector_ref.clone());
        }
    }
    incidence.into_iter().map(|(coordinate,owners)|ObservedCoordinateFibre{
        coordinate,present_in:owners.iter().cloned().collect(),
        not_recorded_in:universe.difference(&owners).cloned().collect(),
    }).collect()
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct SourceSignEntry{
    pub selector_ref:String,
    pub observation_ref:String,
    pub source_revision_ref:String,
    pub polarity:Polarity,
}
/// Existing Agda TernaryComparisonSynthesisExact has three possible
/// synthesis choices for each ordered pair. Here choices are only indices
/// of a future candidate (not signed proof, verdict, or inferred Z).
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct FamilySynthesisColumn{
    pub comparison_ref:String,
    pub left_selector_ref:String,
    pub right_selector_ref:String,
    pub source_polarity_class:SourcePolarityClass,
    pub possible_synthesis_signs:[i8;3],
    pub selected_synthesis_sign:Option<i8>,
    pub requires_independent_interaction_witness:bool,
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct FamilyReviewProjection{
    pub review_item_ref:String,
    pub obligation_ref:String,
    pub comparison_ref:String,
    pub current_status:String,
    pub exact_residuals:Vec<ComparisonResidual>,
    pub source_revision_refs:Vec<String>,
    pub consumer_ref:String,
    pub candidate_only:bool,
}
fn review_status_tag(status:ReviewStatus)->&'static str{
    match status{
        ReviewStatus::Pending=>"pending",ReviewStatus::Accepted=>"accepted",
        ReviewStatus::Rejected=>"rejected",ReviewStatus::Abstained=>"abstained",
        ReviewStatus::Qualified=>"qualified",ReviewStatus::Superseded=>"superseded",
        ReviewStatus::NeedsEvidence=>"needs_evidence",
    }
}
fn residual_review_item(demand:&FamilyAcquisitionDemand,consumer:&str)->ReviewItem{
    let obligation=&demand.obligation;
    ReviewItem{
        review_item_ref:format!("review-item:{}",obligation.obligation_ref),
        semantic_ref:obligation.obligation_ref.clone(),
        item_kind:ReviewItemKind::ResearchAcquisition,
        reason:obligation.target_description.clone(),
        provenance_refs:vec![obligation.comparison_ref.clone(),obligation.residual_obligation_ref.clone()],
        source_refs:obligation.source_revision_refs.clone(),
        current_status:ReviewStatus::Pending,
        available_actions:vec![ReviewAction::Reject,ReviewAction::Abstain,
            ReviewAction::Qualify,ReviewAction::Supersede,ReviewAction::RequestEvidence,
            ReviewAction::OpenSource,ReviewAction::FollowAuthority],
        affected_consumer_refs:vec![consumer.into()],
        candidate_only:true,creates_semantic_authority:false,
        applicability_promoted:false,claim_truth_promoted:false,
    }
}

fn checked_observation(client:&mut impl GenericClient,s:&AcceptanceObservationSelector)
    ->Result<RelationalObservation,RelationalFamilyError>{
    let required=|x:&Option<String>|x.clone().ok_or(RelationalFamilyError::InvalidSelector);
    match s.kind {
        AcceptanceObservationKind::CandidatePnf=>{
            let selected=required(&s.selected_predicate_candidate_ref)?;
            let batch=relational_observations_from_checked_batch_on_client(client,
                &required(&s.batch_ref)?,s.source_family.ok_or(RelationalFamilyError::InvalidSelector)?,
                &s.context,&s.provenance_refs)?;
            batch.observations.into_iter().find(|o|o.predicate_candidate_ref==selected)
                .ok_or(RelationalFamilyError::InvalidSelector)
        },
        AcceptanceObservationKind::WikidataNative=>Ok(relational_observation_from_persisted_wikidata_on_client(
            client,&PersistedWikidataObservationRequest{
                diagnostic_ref:required(&s.diagnostic_ref)?,
                native_statement_ref:required(&s.native_statement_ref)?,
                observation_ref:s.observation_ref.clone(),
                context:s.context.clone(),provenance_refs:s.provenance_refs.clone(),
            })?),
    }
}
fn unobserved_context(a:&RelationalObservation,b:&RelationalObservation,c:&RelationalConsumer)->Vec<String>{
    let a=&a.context;let b=&b.context;
    [
        ("scope",c.compare_scope,&a.scope_ref,&b.scope_ref),
        ("time",c.compare_time,&a.time_ref,&b.time_ref),
        ("modality",c.compare_modality,&a.modality_ref,&b.modality_ref),
        ("quantifier",c.compare_quantifier,&a.quantifier_ref,&b.quantifier_ref),
        ("attribution",c.compare_attribution,&a.attribution_ref,&b.attribution_ref),
        ("ontology",c.compare_ontology,&a.ontology_ref,&b.ontology_ref),
        ("language",c.compare_language,&a.language_ref,&b.language_ref),
    ].into_iter().filter(|(_,required,l,r)|*required &&
        (l.as_deref().is_none_or(|v|v.trim().is_empty())
        ||r.as_deref().is_none_or(|v|v.trim().is_empty())))
        .map(|(axis,_,_,_)|axis.to_owned()).collect()
}

pub fn project_relational_family(
    sources:&[SelectedRelationalSource],consumer:&RelationalConsumer,access_constraint_ref:&str,
)->Result<RelationalFamilyProjection,RelationalFamilyError>{
    if !(2..=10).contains(&sources.len())
        ||sources.iter().any(|s|s.selector_ref.trim().is_empty())
        ||sources.iter().map(|s|&s.selector_ref).collect::<BTreeSet<_>>().len()!=sources.len()
        ||sources.iter().map(|s|&s.observation.observation_ref).collect::<BTreeSet<_>>().len()!=sources.len()
        ||sources.iter().map(|s|&s.observation.source_revision_ref).collect::<BTreeSet<_>>().len()!=sources.len()
    {return Err(RelationalFamilyError::InvalidFamily);}
    if access_constraint_ref.trim().is_empty(){return Err(RelationalFamilyError::MissingAccessConstraint);}
    let mut directed_comparisons=Vec::with_capacity(sources.len()*(sources.len()-1));
    let mut acquisition_demands=Vec::new();
    let mut edges=Vec::new();
    let mut universe=BTreeSet::new();
    for (i,left) in sources.iter().enumerate(){
        for (j,right) in sources.iter().enumerate(){
            if i==j{continue;}
            let l=&left.observation;let r=&right.observation;
            // Directional licences are not reversed by swapping a label.
            // Evaluate both directions through the existing comparison owner.
            let comparison=compare_relational_observations_strict(l,r,consumer)?;
            for rev in [&l.source_revision_ref,&r.source_revision_ref]{
                edges.push((rev.clone(),comparison.comparison_ref.clone()));
            }
            universe.insert(comparison.comparison_ref.clone());
            let mut groups=BTreeMap::<String,Vec<ComparisonResidual>>::new();
            for residual in &comparison.residuals{
                groups.entry(residual.obligation_ref.clone()).or_default().push(residual.clone());
            }
            for (key,exact_residuals) in groups{
                let obligation=obligation_from_residual(&comparison,&exact_residuals[0],
                    vec![l.source_revision_ref.clone(),r.source_revision_ref.clone()],
                    &format!("Resolve consumer obligation {key} at its retained source coordinates"),
                    access_constraint_ref,vec![consumer.consumer_ref.clone()])?;
                edges.push((comparison.comparison_ref.clone(),obligation.obligation_ref.clone()));
                universe.insert(obligation.obligation_ref.clone());
                acquisition_demands.push(FamilyAcquisitionDemand{obligation,exact_residuals});
            }
            directed_comparisons.push(DirectionalRelationalComparison{
                left_selector_ref:left.selector_ref.clone(),right_selector_ref:right.selector_ref.clone(),
                source_polarity_class:classify_source_polarity(l.polarity,r.polarity),
                unobserved_required_context_axes:unobserved_context(l,r,consumer),
                comparison,
            });
        }
    }
    let universe=universe.into_iter().collect::<Vec<_>>();
    let mut potential_reopening_by_source=Vec::new();
    // This is a derived cone over this family only, not an actual database
    // retraction, and does not supersede the existing dependency owner.
    let mut graph_keys=directed_comparisons.iter().map(|p|p.comparison.comparison_ref.as_str()).collect::<Vec<_>>();
    graph_keys.sort_unstable();
    // Every key is a content digest of its full directional contract.
    let graph_ref=format!("relational-family:sha256:{:x}",Sha256::digest(graph_keys.join("|").as_bytes()));
    for source in sources{
        potential_reopening_by_source.push(selective_reopening(
            &source.observation.source_revision_ref,&graph_ref,&edges,&universe)?);
    }
    let observed_coordinate_meet=observed_family_meet(sources);
    let source_sign_ledger=sources.iter().map(|s|SourceSignEntry{
        selector_ref:s.selector_ref.clone(),observation_ref:s.observation.observation_ref.clone(),
        source_revision_ref:s.observation.source_revision_ref.clone(),polarity:s.observation.polarity,
    }).collect();
    let synthesis_candidates=directed_comparisons.iter().map(|p|FamilySynthesisColumn{
        comparison_ref:p.comparison.comparison_ref.clone(),left_selector_ref:p.left_selector_ref.clone(),
        right_selector_ref:p.right_selector_ref.clone(),source_polarity_class:p.source_polarity_class,
        possible_synthesis_signs:[-1,0,1],selected_synthesis_sign:None,
        requires_independent_interaction_witness:true,
    }).collect();
    Ok(RelationalFamilyProjection{
        schema:"itir.relational-family.v2",consumer_ref:consumer.consumer_ref.clone(),
        sources:sources.to_vec(),directed_comparisons,acquisition_demands,
        potential_reopening_by_source,observed_coordinate_meet,source_sign_ledger,synthesis_candidates,
        candidate_only:true,all_live_reading_coverage_paid:false,
        formal_consumer_equivalence_paid:false,source_independence_paid:false,creates_edit_authority:false,
    })
}

/// Actual PG-backed path. The old pair store owns persistence and replay,
/// and the existing INV queue owns route/priority state. The operation is
/// atomic across the newly selected comparison family and acquisition queues.
pub fn run_persisted_relational_family(
    config:&DatabaseConfig,consumer_ref:&str,access_constraint_ref:&str,selector_refs:&[String],
)->Result<PersistedRelationalFamily,RelationalFamilyError>{
    if !(2..=10).contains(&selector_refs.len())
        ||selector_refs.iter().collect::<BTreeSet<_>>().len()!=selector_refs.len()
    {return Err(RelationalFamilyError::InvalidFamily);}
    if access_constraint_ref.trim().is_empty(){return Err(RelationalFamilyError::MissingAccessConstraint);}
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let mut tx=client.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ; SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
    let (consumer,selectors)=load_relational_selection_on_client(&mut tx,consumer_ref,selector_refs)?;
    let sources=selectors.iter().map(|s|Ok(SelectedRelationalSource{
        selector_ref:s.selector_ref.clone(),observation:checked_observation(&mut tx,s)?,
    })).collect::<Result<Vec<_>,RelationalFamilyError>>()?;
    let projection=project_relational_family(&sources,&consumer,access_constraint_ref)?;
    let by_selector=sources.iter().map(|s|(s.selector_ref.as_str(),&s.observation))
        .collect::<BTreeMap<_,_>>();
    // Selection, parser inverse, Wikidata diagnostic, comparison and queues
    // use this same snapshot. A failed replay rolls back the family.
    tx.batch_execute(RELATIONAL_COMPARISON_SQL)?;
    tx.batch_execute(INVESTIGATION_ACQUISITION_SQL)?;
    install_review_workstation_on_client(&mut tx)?;
    let mut digests=RelationalSourceDigests::new(&mut tx)?;
    let mut durable_comparisons=Vec::with_capacity(projection.directed_comparisons.len());
    let mut ordered_pairs=projection.directed_comparisons.iter().collect::<Vec<_>>();
    ordered_pairs.sort_by(|a,b|a.comparison.comparison_ref.cmp(&b.comparison.comparison_ref));
    for pair in ordered_pairs{
        let row=persist_relational_comparison_on_client(&mut tx,&mut digests,
            by_selector[pair.left_selector_ref.as_str()],
            by_selector[pair.right_selector_ref.as_str()],&consumer,STRICT_RELATIONAL_INTERLINGUA_SCHEMA)?;
        if row.comparison!=pair.comparison{return Err(RelationalFamilyError::ChangedReplay);}
        durable_comparisons.push(row);
    }
    let parents=durable_comparisons.iter().map(|p|(p.comparison.comparison_ref.as_str(),p))
        .collect::<BTreeMap<_,_>>();
    let mut durable_acquisition_queues=Vec::new();
    let mut residual_review_items=Vec::new();
    // Stable lock order avoids family-dependent queue locking order.
    let mut demands=projection.acquisition_demands.iter().collect::<Vec<_>>();
    demands.sort_by(|a,b|a.obligation.obligation_ref.cmp(&b.obligation.obligation_ref));
    for demand in demands{
        let parent=parents.get(demand.obligation.comparison_ref.as_str())
            .ok_or(RelationalFamilyError::ChangedReplay)?;
        durable_acquisition_queues.push(ensure_acquisition_obligation_on_client(
            &mut tx,&demand.obligation,parent)?);
        let item=persist_review_projection_on_tx(&mut tx,&residual_review_item(demand,consumer_ref))?;
        residual_review_items.push(FamilyReviewProjection{
            review_item_ref:item.review_item_ref,obligation_ref:demand.obligation.obligation_ref.clone(),
            comparison_ref:demand.obligation.comparison_ref.clone(),
            current_status:review_status_tag(item.current_status).into(),
            exact_residuals:demand.exact_residuals.clone(),
            source_revision_refs:demand.obligation.source_revision_refs.clone(),
            consumer_ref:consumer_ref.into(),candidate_only:true,
        });
    }
    tx.commit()?;
    Ok(PersistedRelationalFamily{
        projection,durable_comparisons,durable_acquisition_queues,residual_review_items,
        creates_access_authority:false,applies_external_edit:false,
    })
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::{RelationalSourceFamily,ObservationContext,LicensedAlignment,AlignmentDirection,
        ComparisonFinding,ResidualKind};
    fn source(id:&str,predicate:&str,polarity:Polarity)->SelectedRelationalSource{
        SelectedRelationalSource{selector_ref:format!("selector:{id}"),observation:RelationalObservation{
            observation_ref:format!("obs:{id}"),source_revision_ref:format!("rev:{id}"),
            source_family:RelationalSourceFamily::Wikipedia,source_span_ref:format!("span:{id}"),
            parser_or_producer_ref:"parser".into(),predicate_candidate_ref:predicate.into(),
            role_bindings:vec![],role_type_hypotheses:vec![],context:ObservationContext::default(),
            candidate_type_refs:vec![],provenance_refs:vec![format!("prov:{id}")],native_metadata_refs:vec![],
            polarity,candidate_only:true,creates_semantic_authority:false,claim_truth_promoted:false}}
    }
    fn consumer()->RelationalConsumer{
        RelationalConsumer{consumer_ref:"query".into(),permitted_source_families:vec![RelationalSourceFamily::Wikipedia],
            required_roles:vec![],required_role_types:vec![],compare_scope:false,compare_time:false,
            compare_modality:false,compare_quantifier:false,compare_attribution:false,compare_ontology:false,
            compare_language:false,predicate_alignments:vec![],role_alignments:vec![],filler_alignments:vec![],type_alignments:vec![]}
    }
    #[test]
    fn observed_meet_retains_all_incidence_and_never_forges_synthesis(){
        let mut a=source("a","p",Polarity::Supports);
        let mut b=source("b","p",Polarity::Counters);
        a.observation.context.time_ref=Some("t1".into());
        b.observation.context.time_ref=Some("t2".into());
        let q=consumer();
        let result=project_relational_family(&[a,b],&q,"review").unwrap();
        let common=result.observed_coordinate_meet.iter().find(|r|r.coordinate.axis=="predicate").unwrap();
        assert_eq!(common.present_in,vec!["selector:a","selector:b"]);
        assert!(common.not_recorded_in.is_empty());
        let diff=result.observed_coordinate_meet.iter().find(|r|r.coordinate.value_ref=="t1").unwrap();
        assert_eq!(diff.present_in,vec!["selector:a"]);
        assert_eq!(diff.not_recorded_in,vec!["selector:b"]);
        assert_eq!(result.source_sign_ledger.len(),2);
        assert!(result.synthesis_candidates.iter().all(|r|r.selected_synthesis_sign.is_none()
            &&r.possible_synthesis_signs==[-1,0,1]&&r.requires_independent_interaction_witness));
        assert!(!result.formal_consumer_equivalence_paid);
    }
    #[test]
    fn all_nine_source_states_roundtrip(){
        for a in [Polarity::Counters,Polarity::Undetermined,Polarity::Supports]{
            for b in [Polarity::Counters,Polarity::Undetermined,Polarity::Supports]{
                assert_eq!(source_polarity_pair(classify_source_polarity(a,b)),(a,b));
            }
        }
    }
    #[test]
    fn directional_licence_does_not_pay_reverse_or_unrelated_pairs(){
        let a=source("a","p",Polarity::Supports);
        let b=source("b","q",Polarity::Supports);
        let c=source("c","r",Polarity::Counters);
        let mut q=consumer();
        q.predicate_alignments.push(LicensedAlignment{left_ref:"p".into(),right_ref:"q".into(),
            witness_ref:"w".into(),licence_ref:"licence".into(),consumer_ref:q.consumer_ref.clone(),
            direction:AlignmentDirection::LeftToRight});
        let result=project_relational_family(&[a,b,c],&q,"public-review").unwrap();
        assert_eq!(result.directed_comparisons.len(),6);
        let get=|l:&str,r:&str| result.directed_comparisons.iter().find(|p|
            p.left_selector_ref==l&&p.right_selector_ref==r).unwrap();
        assert_eq!(get("selector:a","selector:b").comparison.finding,ComparisonFinding::LicensedCompatible);
        assert_eq!(get("selector:b","selector:a").comparison.finding,ComparisonFinding::MissingTypedMeet);
        assert_eq!(get("selector:a","selector:c").source_polarity_class,SourcePolarityClass::PositiveToNegative);
        let other=get("selector:b","selector:c").comparison.comparison_ref.clone();
        assert!(result.potential_reopening_by_source[0].unrelated_refs_not_reopened.contains(&other));
        assert!(!result.formal_consumer_equivalence_paid);
    }
    #[test]
    fn grouped_residuals_and_missing_context_survive(){
        let mut a=source("a","p",Polarity::Undetermined);
        let mut b=source("b","p",Polarity::Undetermined);
        a.observation.candidate_type_refs=vec!["type:A".into(),"type:B".into()];
        b.observation.candidate_type_refs=vec!["type:C".into()];
        let mut q=consumer();q.compare_time=true;
        let result=project_relational_family(&[a.clone(),b],&q,"review").unwrap();
        assert_eq!(result.directed_comparisons[0].unobserved_required_context_axes,vec!["time"]);
        assert!(result.acquisition_demands.iter().any(|d|d.obligation.residual_obligation_ref=="consumer:context-unobserved:time"));
        assert!(result.acquisition_demands.iter().any(|d|d.exact_residuals.len()==3
            &&d.exact_residuals.iter().all(|r|r.kind==ResidualKind::UnalignedType)));
        assert!(project_relational_family(&[a.clone(),a],&q,"review").is_err());
    }
}
