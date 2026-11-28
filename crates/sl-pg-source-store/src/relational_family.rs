//! Compose existing REL and INV owners across a bounded family of original
//! source observations. This is a native candidate read/repair work surface,
//! not another ontology, truth authority, or a proof of all-live sufficiency.
use std::collections::{BTreeMap,BTreeSet};
use serde::{Serialize,Deserialize};
use sensiblaw_core::review_workstation::{ReviewItemKind,ReviewItem,ReviewStatus,ReviewAction};
use crate::review_workstation_store::{install_review_workstation_on_client,persist_review_projection_on_tx,load_review_item_with_client,canonical_review_item_for_persistence,same_review_projection_shape,ReviewWorkstationStoreError};
use sha2::{Digest,Sha256};
use thiserror::Error;
use postgres::{Client,GenericClient,NoTls};
use crate::workbench_projection::acceptance_control_store::load_relational_selection_on_client;
use crate::workbench_projection::persisted_relational_observation::{relational_observations_from_checked_batch_on_client,relational_observation_from_persisted_wikidata_on_client};
use crate::relational_interlingua_store::{RelationalSourceDigests,persist_relational_comparison_on_client,load_relational_comparison_on_client};
use crate::investigation_acquisition_store::{ensure_acquisition_obligation_on_client,load_acquisition_lifecycle_on_client,load_acquisition_queue_rows,AcquisitionLifecycle};
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
    #[error(transparent)] Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    FormalProof(#[from] sensiblaw_formal_proof_bridge::FormalProofBridgeError),
    #[error("independent reciprocal query proof is not bound to the current source pair, query and answer")]
    ReciprocalProofBinding,
    #[error("family or revision transition not found")]
    MissingFamily,
    #[error("revision transition requires the current exact S29 candidate review to be accepted")]
    ReviewRequired,
    #[error("family already superseded or replacement does not preserve all unaffected selections")]
    InvalidTransition,
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
    pub declared_transport_meet:DeclaredTransportMeet,
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
    pub family_ref:String,
    pub superseded_by:Option<String>,
    /// A prepared child is not a published replacement.
    pub prepared_revision_refs:Vec<String>,
    pub published_parent_transition_refs:Vec<String>,
    pub projection:RelationalFamilyProjection,
    pub durable_comparisons:Vec<DurableRelationalComparison>,
    pub durable_acquisition_queues:Vec<DurableAcquisitionQueue>,
    pub residual_review_items:Vec<FamilyReviewProjection>,
    pub acquisition_lifecycles:Vec<AcquisitionLifecycle>,
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

/// Actual finite bidirectional coordinate transport under the existing
/// consumer's explicitly declared alignments. No inferred transitive
/// closure, source identity, lexical equivalence or semantic authority.
/// Distinct source coordinates are retained even when transport matches.
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct TransportCoordinateMember{
    pub selector_ref:String,
    pub source_revision_ref:String,
    pub coordinate:ObservedCoordinate,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct TransportCoordinateMatch{
    pub from_selector_ref:String,
    pub to_selector_ref:String,
    pub from:ObservedCoordinate,
    pub to:ObservedCoordinate,
    pub witness_refs:Vec<String>,
    pub licence_refs:Vec<String>,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct TransportCoordinateRow{
    pub anchor_coordinate:ObservedCoordinate,
    pub members:Vec<TransportCoordinateMember>,
    pub matches:Vec<TransportCoordinateMatch>,
    pub held_selector_refs:Vec<String>,
    pub holds:Vec<String>,
    pub complete_declared_coordinate_transport:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct DeclaredTransportMeet{
    pub anchor_selector_ref:String,
    pub consumer_ref:String,
    pub rows:Vec<TransportCoordinateRow>,
    pub required_context_and_pair_debt_refs:Vec<String>,
    pub candidate_only:bool,
    pub formal_preservation_paid:bool,
}
fn shared_content_digest(a:&RelationalObservation,b:&RelationalObservation)->bool{
    let left=a.provenance_refs.iter().filter(|r|r.starts_with("sha256:")).collect::<BTreeSet<_>>();
    let right=b.provenance_refs.iter().filter(|r|r.starts_with("sha256:")).collect::<BTreeSet<_>>();
    !left.is_disjoint(&right)
}
fn native_coordinate_key(value:&str)->Option<&str>{
    let (_,tail)=value.split_once("candidate-pnf-db:")?;
    let (_,tail)=tail.rsplit_once(":sentence:")?;
    Some(tail)
}
fn coordinate_value_matches(left:&str,right:&str,source_coordinate_bridge:bool)->bool{
    left==right || (source_coordinate_bridge
        && native_coordinate_key(left).is_some()
        && native_coordinate_key(left)==native_coordinate_key(right))
}
fn coordinate_match(a:&ObservedCoordinate,b:&ObservedCoordinate,c:&RelationalConsumer,
    source_coordinate_bridge:bool)
    ->Option<(Vec<String>,Vec<String>)>{
    use crate::relational_interlingua::alignment;
    if a.axis!=b.axis||a.occurrence!=b.occurrence{return None;}
    let mut witnesses=BTreeSet::new();
    let mut licences=BTreeSet::new();
    let mut match_ref=|x:&str,y:&str,list:&[crate::LicensedAlignment]|{
        if coordinate_value_matches(x,y,source_coordinate_bridge){return true;}
        let Some(forward)=alignment(x,y,c,list) else{return false};
        let Some(reverse)=alignment(y,x,c,list) else{return false};
        for w in [forward,reverse]{
            witnesses.insert(w.witness_ref.clone());licences.insert(w.licence_ref.clone());
        }
        true
    };
    let matched=match a.axis.as_str(){
        "predicate"=>a.slot==b.slot&&match_ref(&a.value_ref,&b.value_ref,&c.predicate_alignments),
        "role"=>match_ref(&a.slot,&b.slot,&c.role_alignments)
            &&match_ref(&a.value_ref,&b.value_ref,&c.filler_alignments),
        "role_type"=>match_ref(&a.slot,&b.slot,&c.role_alignments)
            &&match_ref(&a.value_ref,&b.value_ref,&c.type_alignments),
        "candidate_type"=>a.slot==b.slot&&match_ref(&a.value_ref,&b.value_ref,&c.type_alignments),
        "context"=>a==b,
        _=>false,
    };
    matched.then(||(witnesses.into_iter().collect(),licences.into_iter().collect()))
}
/// The first selector in stable lexical order is a chart only, not a
/// preferred source. For every anchor coordinate require one forward and
/// inverse match in each source, then check every cross-source pair
/// directly. Thus a path A↔B↔C cannot manufacture A↔C, and a many-to-one
/// alias cannot silently erase a coordinate. Failed rows remain inspectable.
fn declared_transport_meet(sources:&[SelectedRelationalSource],consumer:&RelationalConsumer,
    comparisons:&[DirectionalRelationalComparison])->DeclaredTransportMeet{
    let mut charts=sources.iter().map(|s|(s,observed_coordinates(&s.observation))).collect::<Vec<_>>();
    charts.sort_by(|a,b|a.0.selector_ref.cmp(&b.0.selector_ref));
    let (anchor,coordinates)=&charts[0];
    let mut rows=Vec::new();
    for coordinate in coordinates{
        let mut members=Vec::new();
        let mut held=BTreeSet::new();
        let mut holds=BTreeSet::new();
        for (source,source_coords) in &charts{
            let source_coordinate_bridge=shared_content_digest(&anchor.observation,&source.observation);
            let matching=source_coords.iter().filter(|candidate|coordinate_match(coordinate,candidate,consumer,source_coordinate_bridge).is_some())
                .collect::<Vec<_>>();
            let [candidate]=matching.as_slice() else{
                held.insert(source.selector_ref.clone());
                holds.insert(if matching.is_empty(){"missing_bidirectional_coordinate"}else{"ambiguous_bidirectional_coordinate"}.to_owned());
                continue;
            };
            if coordinates.iter().filter(|a|coordinate_match(a,candidate,consumer,source_coordinate_bridge).is_some()).count()!=1{
                held.insert(source.selector_ref.clone());holds.insert("noninjective_inverse".into());continue;
            }
            members.push(TransportCoordinateMember{selector_ref:source.selector_ref.clone(),
                source_revision_ref:source.observation.source_revision_ref.clone(),coordinate:(**candidate).clone()});
        }
        let mut matches=Vec::new();
        for a in &members{
            for b in &members{
                if a.selector_ref==b.selector_ref{continue;}
                let source_a=&sources.iter().find(|s|s.selector_ref==a.selector_ref).expect("member selector").observation;
                let source_b=&sources.iter().find(|s|s.selector_ref==b.selector_ref).expect("member selector").observation;
                if let Some((witness_refs,licence_refs))=coordinate_match(&a.coordinate,&b.coordinate,consumer,shared_content_digest(source_a,source_b)){
                    matches.push(TransportCoordinateMatch{from_selector_ref:a.selector_ref.clone(),
                        to_selector_ref:b.selector_ref.clone(),from:a.coordinate.clone(),to:b.coordinate.clone(),
                        witness_refs,licence_refs});
                }else{
                    held.insert(a.selector_ref.clone());held.insert(b.selector_ref.clone());
                    holds.insert("missing_direct_cross_chart_transport".into());
                }
            }
        }
        let complete=held.is_empty()&&members.len()==sources.len()
            &&matches.len()==sources.len()*(sources.len()-1);
        rows.push(TransportCoordinateRow{anchor_coordinate:coordinate.clone(),members,matches,
            held_selector_refs:held.into_iter().collect(),holds:holds.into_iter().collect(),
            complete_declared_coordinate_transport:complete});
    }
    let required_context_and_pair_debt_refs=comparisons.iter()
        .flat_map(|pair|pair.comparison.residuals.iter().map(move |r|
            format!("{}:{}",pair.comparison.comparison_ref,r.obligation_ref)))
        .collect::<BTreeSet<_>>().into_iter().collect();
    DeclaredTransportMeet{anchor_selector_ref:anchor.selector_ref.clone(),consumer_ref:consumer.consumer_ref.clone(),
        rows,required_context_and_pair_debt_refs,candidate_only:true,formal_preservation_paid:false}
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
            // A caller's unrelated digest is not the canonical source digest.
            // Keep the legacy receipt shape, but fail closed rather than
            // allowing injected raw digests to enable same-content transport.
            if s.provenance_refs.iter().any(|r|r.starts_with("sha256:")&&r!=&batch.source_content_digest_ref){
                return Err(RelationalFamilyError::InvalidSelector);
            }
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
    let declared_transport_meet=declared_transport_meet(sources,consumer,&directed_comparisons);
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
        potential_reopening_by_source,observed_coordinate_meet,declared_transport_meet,source_sign_ledger,synthesis_candidates,
        candidate_only:true,all_live_reading_coverage_paid:false,
        formal_consumer_equivalence_paid:false,source_independence_paid:false,creates_edit_authority:false,
    })
}

// These are lineage/index sidecars, not a second observation or proof store.
// Every source, comparison, obligation and review is read from its existing
// owner. A family replacement retires that family selection only; L1,
// unrelated pair rows and numeric governed L3 are never mutated here.
const FAMILY_LINEAGE_SQL:&str=r#"
CREATE TABLE IF NOT EXISTS semantic.itir_relational_family (
  family_ref TEXT PRIMARY KEY,
  packet_sha256 TEXT NOT NULL,
  packet_json TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK(candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK(NOT creates_semantic_authority)
);
CREATE TABLE IF NOT EXISTS semantic.itir_relational_family_revision (
  transition_ref TEXT PRIMARY KEY,
  old_family_ref TEXT NOT NULL REFERENCES semantic.itir_relational_family(family_ref),
  new_family_ref TEXT NOT NULL REFERENCES semantic.itir_relational_family(family_ref),
  review_item_ref TEXT NOT NULL REFERENCES semantic.review_item(review_item_ref),
  packet_sha256 TEXT NOT NULL,
  packet_json TEXT NOT NULL,
  CHECK(old_family_ref<>new_family_ref)
);
CREATE TABLE IF NOT EXISTS semantic.itir_relational_family_successor (
  old_family_ref TEXT PRIMARY KEY REFERENCES semantic.itir_relational_family(family_ref),
  transition_ref TEXT NOT NULL UNIQUE REFERENCES semantic.itir_relational_family_revision(transition_ref),
  new_family_ref TEXT NOT NULL REFERENCES semantic.itir_relational_family(family_ref),
  CHECK(old_family_ref<>new_family_ref)
);
"#;

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
struct FamilyMember{
    selector_ref:String,observation_ref:String,source_revision_ref:String,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
struct FamilyManifest{
    schema:String,family_ref:String,consumer_ref:String,access_constraint_ref:String,
    members:Vec<FamilyMember>,comparison_refs:Vec<String>,
}
fn digest_bytes(bytes:&[u8])->String{format!("sha256:{:x}",Sha256::digest(bytes))}
fn family_manifest(projection:&RelationalFamilyProjection,access:&str)
    ->Result<FamilyManifest,RelationalFamilyError>{
    let mut members=projection.sources.iter().map(|s|FamilyMember{
        selector_ref:s.selector_ref.clone(),observation_ref:s.observation.observation_ref.clone(),
        source_revision_ref:s.observation.source_revision_ref.clone(),
    }).collect::<Vec<_>>();
    members.sort_by(|a,b|a.selector_ref.cmp(&b.selector_ref));
    let mut comparison_refs=projection.directed_comparisons.iter()
        .map(|p|p.comparison.comparison_ref.clone()).collect::<Vec<_>>();
    comparison_refs.sort();
    let mut m=FamilyManifest{
        schema:"itir.relational-family-manifest.v1".into(),family_ref:String::new(),
        consumer_ref:projection.consumer_ref.clone(),access_constraint_ref:access.into(),
        members,comparison_refs,
    };
    m.family_ref=format!("relational-family:{}",digest_bytes(&serde_json::to_vec(&m)?));
    Ok(m)
}
fn read_manifest(client:&mut impl GenericClient,family_ref:&str)
    ->Result<FamilyManifest,RelationalFamilyError>{
    let row=client.query_opt(
        "SELECT packet_sha256,packet_json,candidate_only,creates_semantic_authority
         FROM semantic.itir_relational_family WHERE family_ref=$1",&[&family_ref])?
        .ok_or(RelationalFamilyError::MissingFamily)?;
    let body:String=row.get(1);
    let m:FamilyManifest=serde_json::from_str(&body)?;
    let mut blank=m.clone();blank.family_ref.clear();
    if !row.get::<_,bool>(2)||row.get::<_,bool>(3)
        ||row.get::<_,String>(0)!=digest_bytes(body.as_bytes())
        ||m.family_ref!=family_ref
        ||m.family_ref!=format!("relational-family:{}",digest_bytes(&serde_json::to_vec(&blank)?)){
        return Err(RelationalFamilyError::ChangedReplay);
    }
    Ok(m)
}
fn successor(client:&mut impl GenericClient,family_ref:&str)
    ->Result<Option<String>,RelationalFamilyError>{
    let row=client.query_opt(
        "SELECT s.new_family_ref,r.old_family_ref,r.new_family_ref
         FROM semantic.itir_relational_family_successor s
         JOIN semantic.itir_relational_family_revision r ON r.transition_ref=s.transition_ref
         WHERE s.old_family_ref=$1",&[&family_ref])?;
    if let Some(row)=row{
        let new:String=row.get(0);
        if row.get::<_,String>(1)!=family_ref||row.get::<_,String>(2)!=new{
            return Err(RelationalFamilyError::ChangedReplay);
        }
        Ok(Some(new))
    }else{Ok(None)}
}
fn ensure_family_schema(client:&mut impl GenericClient)->Result<(),RelationalFamilyError>{
    client.batch_execute(RELATIONAL_COMPARISON_SQL)?;
    client.batch_execute(INVESTIGATION_ACQUISITION_SQL)?;
    install_review_workstation_on_client(client)?;
    client.batch_execute(FAMILY_LINEAGE_SQL)?;
    Ok(())
}
fn current_projection(client:&mut impl GenericClient,consumer_ref:&str,
    access:&str,selector_refs:&[String])
    ->Result<(RelationalConsumer,RelationalFamilyProjection),RelationalFamilyError>{
    if !(2..=10).contains(&selector_refs.len())
        ||selector_refs.iter().collect::<BTreeSet<_>>().len()!=selector_refs.len(){
        return Err(RelationalFamilyError::InvalidFamily);
    }
    if access.trim().is_empty(){return Err(RelationalFamilyError::MissingAccessConstraint);}
    let mut ordered=selector_refs.to_vec();ordered.sort();
    let (consumer,selectors)=load_relational_selection_on_client(client,consumer_ref,&ordered)?;
    let sources=selectors.iter().map(|s|Ok(SelectedRelationalSource{
        selector_ref:s.selector_ref.clone(),observation:checked_observation(client,s)?,
    })).collect::<Result<Vec<_>,RelationalFamilyError>>()?;
    let projection=project_relational_family(&sources,&consumer,access)?;
    Ok((consumer,projection))
}

fn inspect_review(client:&mut impl GenericClient,item:&ReviewItem)
    ->Result<ReviewItem,RelationalFamilyError>{
    let expected=canonical_review_item_for_persistence(item.clone());
    let actual=load_review_item_with_client(client,&expected.review_item_ref)?
        .ok_or(RelationalFamilyError::ChangedReplay)?;
    if !same_review_projection_shape(&expected,&actual){return Err(RelationalFamilyError::ChangedReplay);}
    Ok(actual)
}

fn materialize_family(client:&mut impl GenericClient,consumer:&RelationalConsumer,
    projection:RelationalFamilyProjection,access:&str,publish:bool)
    ->Result<PersistedRelationalFamily,RelationalFamilyError>{
    let manifest=family_manifest(&projection,access)?;
    let by_selector=projection.sources.iter().map(|s|(s.selector_ref.as_str(),&s.observation))
        .collect::<BTreeMap<_,_>>();
    let mut digests=RelationalSourceDigests::new(client)?;
    let mut durable_comparisons=Vec::with_capacity(projection.directed_comparisons.len());
    let mut ordered_pairs=projection.directed_comparisons.iter().collect::<Vec<_>>();
    ordered_pairs.sort_by(|a,b|a.comparison.comparison_ref.cmp(&b.comparison.comparison_ref));
    for pair in ordered_pairs{
        let left=by_selector[pair.left_selector_ref.as_str()];
        let right=by_selector[pair.right_selector_ref.as_str()];
        let row=if publish{
            persist_relational_comparison_on_client(client,&mut digests,left,right,consumer,
                STRICT_RELATIONAL_INTERLINGUA_SCHEMA)?
        }else{
            load_relational_comparison_on_client(client,&mut digests,&pair.comparison.comparison_ref)?
                .ok_or(RelationalFamilyError::ChangedReplay)?
        };
        if row.comparison!=pair.comparison||row.left!=*left||row.right!=*right||row.consumer!=*consumer{
            return Err(RelationalFamilyError::ChangedReplay);
        }
        durable_comparisons.push(row);
    }
    let parents=durable_comparisons.iter().map(|p|(p.comparison.comparison_ref.as_str(),p))
        .collect::<BTreeMap<_,_>>();
    let mut durable_acquisition_queues=Vec::new();
    let mut residual_review_items=Vec::new();
    let mut acquisition_lifecycles=Vec::new();
    let mut demands=projection.acquisition_demands.iter().collect::<Vec<_>>();
    demands.sort_by(|a,b|a.obligation.obligation_ref.cmp(&b.obligation.obligation_ref));
    for demand in demands{
        let parent=parents.get(demand.obligation.comparison_ref.as_str())
            .ok_or(RelationalFamilyError::ChangedReplay)?;
        let queue=if publish{
            ensure_acquisition_obligation_on_client(client,&demand.obligation,parent)?
        }else{
            load_acquisition_queue_rows(client,&demand.obligation.obligation_ref)?
                .ok_or(RelationalFamilyError::ChangedReplay)?
        };
        if queue.obligation!=demand.obligation{return Err(RelationalFamilyError::ChangedReplay);}
        acquisition_lifecycles.push(load_acquisition_lifecycle_on_client(client,&queue)?);
        durable_acquisition_queues.push(queue);
        let expected=residual_review_item(demand,&consumer.consumer_ref);
        let item=if publish{persist_review_projection_on_tx(client,&expected)?}
            else{inspect_review(client,&expected)?};
        residual_review_items.push(FamilyReviewProjection{
            review_item_ref:item.review_item_ref,obligation_ref:demand.obligation.obligation_ref.clone(),
            comparison_ref:demand.obligation.comparison_ref.clone(),
            current_status:review_status_tag(item.current_status).into(),
            exact_residuals:demand.exact_residuals.clone(),
            source_revision_refs:demand.obligation.source_revision_refs.clone(),
            consumer_ref:consumer.consumer_ref.clone(),candidate_only:true,
        });
    }
    if publish{
        let body=serde_json::to_string(&manifest)?;
        client.execute(
            "INSERT INTO semantic.itir_relational_family(family_ref,packet_sha256,packet_json,
             candidate_only,creates_semantic_authority) VALUES($1,$2,$3,TRUE,FALSE)
             ON CONFLICT(family_ref) DO NOTHING",
            &[&manifest.family_ref,&digest_bytes(body.as_bytes()),&body])?;
    }
    if read_manifest(client,&manifest.family_ref)?!=manifest{
        return Err(RelationalFamilyError::ChangedReplay);
    }
    let superseded_by=successor(client,&manifest.family_ref)?;
    let prepared_revision_refs=client.query("SELECT transition_ref FROM semantic.itir_relational_family_revision WHERE new_family_ref=$1 ORDER BY transition_ref", &[&manifest.family_ref])?.into_iter().map(|r|r.get::<_,String>(0)).collect();
    let published_parent_transition_refs=client.query("SELECT transition_ref FROM semantic.itir_relational_family_successor WHERE new_family_ref=$1 ORDER BY transition_ref", &[&manifest.family_ref])?.into_iter().map(|r|r.get::<_,String>(0)).collect();
    Ok(PersistedRelationalFamily{
        family_ref:manifest.family_ref,superseded_by,prepared_revision_refs,published_parent_transition_refs,projection,durable_comparisons,
        durable_acquisition_queues,residual_review_items,acquisition_lifecycles,
        creates_access_authority:false,applies_external_edit:false,
    })
}

pub fn run_persisted_relational_family(
    config:&DatabaseConfig,consumer_ref:&str,access_constraint_ref:&str,selector_refs:&[String],
)->Result<PersistedRelationalFamily,RelationalFamilyError>{
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut tx=connection.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ; SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
    ensure_family_schema(&mut tx)?;
    let (consumer,projection)=current_projection(&mut tx,consumer_ref,access_constraint_ref,selector_refs)?;
    let result=materialize_family(&mut tx,&consumer,projection,access_constraint_ref,true)?;
    tx.commit()?;
    Ok(result)
}
fn read_family_on_client(client:&mut impl GenericClient,m:&FamilyManifest)
    ->Result<PersistedRelationalFamily,RelationalFamilyError>{
    let refs=m.members.iter().map(|x|x.selector_ref.clone()).collect::<Vec<_>>();
    let (consumer,projection)=current_projection(client,&m.consumer_ref,&m.access_constraint_ref,&refs)?;
    if family_manifest(&projection,&m.access_constraint_ref)?!=*m{
        return Err(RelationalFamilyError::ChangedReplay);
    }
    materialize_family(client,&consumer,projection,&m.access_constraint_ref,false)
}
/// Reopen exact native source selection, full consumer, comparison/INV and
/// S29 rows in one read-only snapshot. Changed source/contract fails closed.
pub fn inspect_persisted_relational_family(config:&DatabaseConfig,family_ref:&str)
    ->Result<PersistedRelationalFamily,RelationalFamilyError>{
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut tx=connection.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let m=read_manifest(&mut tx,family_ref)?;
    let result=read_family_on_client(&mut tx,&m)?;
    tx.commit()?;
    Ok(result)
}



/// Native, read-only corpus for local research; never fetch outside sources
/// or accept caller-supplied source text. Both selected revisions survive
/// even when they share a canonical document ID or digest.
pub fn inspect_family_local_corpus(config:&DatabaseConfig,family_ref:&str)
    ->Result<(PersistedRelationalFamily,Vec<(crate::PersistedGenericSourceContent,String)>),RelationalFamilyError>{
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut tx=connection.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let m=read_manifest(&mut tx,family_ref)?;
    let family=read_family_on_client(&mut tx,&m)?;
    let mut corpus=Vec::new();
    for s in &family.projection.sources{
        let (source,text)=crate::generic_source_content_store::load_generic_text_source_on_client(
            &mut tx,&s.observation.source_revision_ref).map_err(PersistedRelationalObservationError::from)?;
        if !s.observation.provenance_refs.contains(&source.content_digest_ref){
            return Err(RelationalFamilyError::ChangedReplay);
        }
        corpus.push((source,text));
    }
    tx.commit()?;
    Ok((family,corpus))
}

/// Recorded syntax is a separate bounded consumer, never a semantic
/// alignment licence. Verify the candidate key against its checked owner
/// rather than treating arbitrary suffixes or provenance refs as digests.
fn checked_recorded_key(o:&RelationalObservation,value:&str)->Option<String>{
    let owner=o.parser_or_producer_ref.strip_prefix("db-parser:")?;
    let suffix=format!(":{}",o.source_span_ref);
    let run=owner.strip_suffix(suffix.as_str())?;
    if !run.starts_with("parser-run:") {return None;}
    let prefix=format!("candidate-pnf-db:{owner}:");
    let token=value.strip_prefix(prefix.as_str())?;
    let mut parts=token.split(':');
    let ordinal=parts.next()?.parse::<u32>().ok()?;
    let start=parts.next()?.parse::<u64>().ok()?;
    let end=parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some()||end<=start{return None;}
    let (_,region)=o.source_span_ref.rsplit_once(":sentence:")?;
    let (sentence,span)=region.split_once(':')?;
    let sentence=sentence.parse::<u64>().ok()?;
    let (lo,hi)=span.split_once('-')?;
    let (lo,hi)=(lo.parse::<u64>().ok()?,hi.parse::<u64>().ok()?);
    if lo>start||end>hi{return None;}
    Some(format!("{sentence}:{lo}-{hi}:{ordinal}:{start}:{end}"))
}

fn recorded_syntax_chart(o:&RelationalObservation)->Option<BTreeMap<(String,String,u32,String),String>>{
    let mut chart=BTreeMap::new();
    for coord in observed_coordinates(o).into_iter().filter(|c|c.axis=="predicate"||c.axis=="role"){
        let key=checked_recorded_key(o,&coord.value_ref)?;
        if chart.insert((coord.axis,coord.slot,coord.occurrence,key),coord.value_ref).is_some(){
            return None;
        }
    }
    if chart.len()!=1+o.role_bindings.len(){return None;}
    Some(chart)
}

/// Compare the selected checked predicate and all direct recorded role
/// occurrences. Both complete finite charts must be covered; a complete
/// anchor-only transport cannot conceal extra coordinates in another chart.
fn recorded_syntax_pair(left:&SelectedRelationalSource,right:&SelectedRelationalSource,
    left_digest:&str,right_digest:&str)->serde_json::Value{
    use serde_json::json;
    let base=json!({"left_selector_ref":left.selector_ref,"right_selector_ref":right.selector_ref,
        "left_source_revision_ref":left.observation.source_revision_ref,
        "right_source_revision_ref":right.observation.source_revision_ref,
        "left_content_digest":left_digest,"right_content_digest":right_digest});
    let mut out=base;
    let same=left_digest==right_digest&&!left_digest.is_empty();
    let charts=recorded_syntax_chart(&left.observation).zip(recorded_syntax_chart(&right.observation));
    let Some((a,b))=charts.filter(|_|same) else {
        out["status"]=json!(if same{"invalid_checked_coordinate"}else{"different_source_content"});
        out["recorded_answer_equal"]=serde_json::Value::Null;
        return out;
    };
    let common:Vec<_>=a.iter().filter_map(|(key,value)|b.get(key).map(|other|
        json!({"coordinate":key,"left_original":value,"right_original":other}))).collect();
    let left_only:Vec<_>=a.iter().filter(|(key,_)|!b.contains_key(*key))
        .map(|(key,v)|json!({"coordinate":key,"original":v})).collect();
    let right_only:Vec<_>=b.iter().filter(|(key,_)|!a.contains_key(*key))
        .map(|(key,v)|json!({"coordinate":key,"original":v})).collect();
    let equal=left_only.is_empty()&&right_only.is_empty();
    out["status"]=json!(if equal{"equal_recorded_syntax"}else{"different_recorded_syntax"});
    out["left_count"]=json!(a.len());out["right_count"]=json!(b.len());
    out["common"]=json!(common);out["left_only"]=json!(left_only);out["right_only"]=json!(right_only);
    out["recorded_answer_equal"]=json!(equal);
    out
}

/// Read the real family and original generic source content digests within
/// one read-only snapshot. No caller-supplied digest, semantic witness or
/// review receipt can be used to pay the bounded recorded-syntax query.
pub fn inspect_recorded_family_parity(config:&DatabaseConfig,family_ref:&str)
    ->Result<serde_json::Value,RelationalFamilyError>{
    use serde_json::json;
    use sensiblaw_core::consumer_parity::ConsumerAdequacyReceipt;
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut tx=connection.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let m=read_manifest(&mut tx,family_ref)?;
    let family=read_family_on_client(&mut tx,&m)?;
    let mut digests=BTreeMap::new();
    for s in &family.projection.sources{
        let (source,_)=crate::generic_source_content_store::load_generic_text_source_on_client(
            &mut tx,&s.observation.source_revision_ref)
            .map_err(PersistedRelationalObservationError::from)?;
        if !s.observation.provenance_refs.contains(&source.content_digest_ref){
            return Err(RelationalFamilyError::ChangedReplay);
        }
        digests.insert(s.selector_ref.clone(),source.content_digest_ref);
    }
    let sources=&family.projection.sources;
    let query_family="query-family:checked-selected-predicate-direct-role-coordinates:v1";
    let bounded_consumer=format!("consumer:recorded-syntax:{family_ref}");
    let mut pairs=Vec::new();
    for (i,left) in sources.iter().enumerate(){
        for right in sources.iter().skip(i+1){
            pairs.push(recorded_syntax_pair(left,right,&digests[&left.selector_ref],&digests[&right.selector_ref]));
        }
    }
    let equal=family.superseded_by.is_none()&&!pairs.is_empty()&&pairs.iter()
        .all(|p|p["recorded_answer_equal"]==true);
    // The exact chart comparison is its own inspectable finite witness.
    // Use the existing adequacy constructor for this declared observed query
    // only. Do not issue RepresentationParityReceipt without actual review.
    let evidence_ref=format!("recorded-syntax-factorisation:{}",digest_bytes(&serde_json::to_vec(&pairs)?));
    let mut adequacy=Vec::new();
    if equal{
        for s in sources{
            let a=ConsumerAdequacyReceipt::new(&s.observation.observation_ref,&bounded_consumer,
                query_family,&evidence_ref,[evidence_ref.as_str()],true,true,None)
                .map_err(|_|RelationalFamilyError::ChangedReplay)?;
            adequacy.push(json!({"representation_ref":a.representation_ref,"consumer_ref":a.consumer_ref,
                "query_family_ref":a.query_family_ref,"admissibility_receipt_ref":a.admissibility_receipt_ref,
                "factorisation_receipt_refs":a.factorisation_receipt_refs,"admissible":a.admissible,
                "consumer_adequate":a.consumer_adequate}));
        }
    }
    tx.commit()?;
    Ok(json!({"schema":"slr.recorded-family-parity.v1","family_ref":family_ref,
        "consumer_ref":bounded_consumer,"query_family_ref":query_family,
        "original_semantic_consumer_ref":family.projection.consumer_ref,
        "superseded_by":family.superseded_by,"pairs":pairs,"all_recorded_answers_equal":equal,
        "factorisation_evidence_ref":evidence_ref,"recorded_consumer_adequacy":adequacy,
        "reviewed_representation_parity_paid":false,"formal_consumer_equivalence_paid":false,
        "semantic_residuals_retained":family.projection.directed_comparisons.iter().map(|p|p.comparison.residuals.len()).sum::<usize>(),
        "candidate_only":true,"source_independence_paid":false,"world_truth_paid":false,"writes_database":false}))
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct FamilyRevisionTransition{
    pub schema:String,
    pub transition_ref:String,
    pub old_family_ref:String,
    pub new_family_ref:String,
    pub old_selector_ref:String,
    pub new_selector_ref:String,
    pub old_source_revision_ref:String,
    pub new_source_revision_ref:String,
    pub retained_comparison_refs:Vec<String>,
    pub retired_family_comparison_refs:Vec<String>,
    pub new_comparison_refs:Vec<String>,
    pub review_item_ref:String,
    pub candidate_only:bool,
    pub creates_semantic_authority:bool,
}
fn revision_transition(old:&FamilyManifest,new:&FamilyManifest,old_selector:&str,new_selector:&str)
    ->Result<FamilyRevisionTransition,RelationalFamilyError>{
    let a=old.members.iter().find(|s|s.selector_ref==old_selector)
        .ok_or(RelationalFamilyError::InvalidTransition)?;
    let b=new.members.iter().find(|s|s.selector_ref==new_selector)
        .ok_or(RelationalFamilyError::InvalidTransition)?;
    if old.family_ref==new.family_ref||old.consumer_ref!=new.consumer_ref
        ||old.access_constraint_ref!=new.access_constraint_ref
        ||a.source_revision_ref==b.source_revision_ref
        ||old_selector==new_selector
        ||old.members.iter().filter(|s|s.selector_ref!=old_selector).collect::<Vec<_>>()
            !=new.members.iter().filter(|s|s.selector_ref!=new_selector).collect::<Vec<_>>(){
        return Err(RelationalFamilyError::InvalidTransition);
    }
    let x=old.comparison_refs.iter().cloned().collect::<BTreeSet<_>>();
    let y=new.comparison_refs.iter().cloned().collect::<BTreeSet<_>>();
    let n=old.members.len();
    let mut t=FamilyRevisionTransition{
        schema:"itir.relational-family-revision.v1".into(),transition_ref:String::new(),
        old_family_ref:old.family_ref.clone(),new_family_ref:new.family_ref.clone(),
        old_selector_ref:old_selector.into(),new_selector_ref:new_selector.into(),
        old_source_revision_ref:a.source_revision_ref.clone(),new_source_revision_ref:b.source_revision_ref.clone(),
        retained_comparison_refs:x.intersection(&y).cloned().collect(),
        retired_family_comparison_refs:x.difference(&y).cloned().collect(),
        new_comparison_refs:y.difference(&x).cloned().collect(),
        review_item_ref:String::new(),candidate_only:true,creates_semantic_authority:false,
    };
    if n<2||t.retained_comparison_refs.len()!=(n-1)*(n-2)
        ||t.retired_family_comparison_refs.len()!=2*(n-1)||t.new_comparison_refs.len()!=2*(n-1){
        return Err(RelationalFamilyError::InvalidTransition);
    }
    t.transition_ref=format!("relational-family-revision:{}",digest_bytes(&serde_json::to_vec(&t)?));
    t.review_item_ref=format!("review-item:{}",t.transition_ref);
    Ok(t)
}
fn transition_review(t:&FamilyRevisionTransition,consumer:&str)->ReviewItem{
    ReviewItem{
        review_item_ref:t.review_item_ref.clone(),semantic_ref:t.transition_ref.clone(),
        item_kind:ReviewItemKind::SourceCorrespondence,
        reason:"Review a replacement of one selected source revision; not a same-entity or truth assertion".into(),
        provenance_refs:vec![t.old_family_ref.clone(),t.new_family_ref.clone()],
        source_refs:vec![t.old_source_revision_ref.clone(),t.new_source_revision_ref.clone()],
        current_status:ReviewStatus::Pending,
        available_actions:vec![ReviewAction::Accept,ReviewAction::Reject,ReviewAction::Abstain,ReviewAction::OpenSource],
        affected_consumer_refs:vec![consumer.into()],candidate_only:true,creates_semantic_authority:false,
        applicability_promoted:false,claim_truth_promoted:false,
    }
}
fn read_transition(client:&mut impl GenericClient,transition_ref:&str)
    ->Result<FamilyRevisionTransition,RelationalFamilyError>{
    let row=client.query_opt(
        "SELECT old_family_ref,new_family_ref,review_item_ref,packet_sha256,packet_json
         FROM semantic.itir_relational_family_revision WHERE transition_ref=$1",&[&transition_ref])?
        .ok_or(RelationalFamilyError::MissingFamily)?;
    let body:String=row.get(4);
    let t:FamilyRevisionTransition=serde_json::from_str(&body)?;
    if row.get::<_,String>(3)!=digest_bytes(body.as_bytes())||t.transition_ref!=transition_ref
        ||t.old_family_ref!=row.get::<_,String>(0)||t.new_family_ref!=row.get::<_,String>(1)
        ||t.review_item_ref!=row.get::<_,String>(2){
        return Err(RelationalFamilyError::ChangedReplay);
    }
    let old=read_manifest(client,&t.old_family_ref)?;
    let new=read_manifest(client,&t.new_family_ref)?;
    if t!=revision_transition(&old,&new,&t.old_selector_ref,&t.new_selector_ref)?{
        return Err(RelationalFamilyError::ChangedReplay);
    }
    Ok(t)
}
/// Prepare a candidate child from one changed selector. Exact unaffected
/// pair refs survive; old family remains live until explicit S29 review.
pub fn prepare_relational_family_revision(config:&DatabaseConfig,family_ref:&str,
    old_selector:&str,new_selector:&str)->Result<FamilyRevisionTransition,RelationalFamilyError>{
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut tx=connection.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ; SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
    let old=read_manifest(&mut tx,family_ref)?;
    if successor(&mut tx,family_ref)?.is_some(){return Err(RelationalFamilyError::InvalidTransition);}
    read_family_on_client(&mut tx,&old)?;
    let mut selectors=old.members.iter().map(|s|s.selector_ref.clone()).collect::<Vec<_>>();
    let slot=selectors.iter_mut().find(|s|s.as_str()==old_selector)
        .ok_or(RelationalFamilyError::InvalidTransition)?;
    *slot=new_selector.into();
    let (consumer,projection)=current_projection(&mut tx,&old.consumer_ref,&old.access_constraint_ref,&selectors)?;
    let new=family_manifest(&projection,&old.access_constraint_ref)?;
    let t=revision_transition(&old,&new,old_selector,new_selector)?;
    materialize_family(&mut tx,&consumer,projection,&old.access_constraint_ref,true)?;
    if successor(&mut tx,&new.family_ref)?.is_some(){return Err(RelationalFamilyError::InvalidTransition);}
    persist_review_projection_on_tx(&mut tx,&transition_review(&t,&old.consumer_ref))?;
    let body=serde_json::to_string(&t)?;
    tx.execute(
        "INSERT INTO semantic.itir_relational_family_revision
         (transition_ref,old_family_ref,new_family_ref,review_item_ref,packet_sha256,packet_json)
         VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(transition_ref) DO NOTHING",
        &[&t.transition_ref,&t.old_family_ref,&t.new_family_ref,&t.review_item_ref,&digest_bytes(body.as_bytes()),&body])?;
    if read_transition(&mut tx,&t.transition_ref)?!=t{return Err(RelationalFamilyError::ChangedReplay);}
    tx.commit()?;
    Ok(t)
}
/// A local S29 candidate review can retire an old *family selection*,
/// without deleting any source, pair, evidence, or governed L3 derivation.
pub fn publish_relational_family_revision(config:&DatabaseConfig,transition_ref:&str)
    ->Result<FamilyRevisionTransition,RelationalFamilyError>{
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut tx=connection.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE; SET LOCAL statement_timeout='90000ms'; SET LOCAL lock_timeout='5000ms'")?;
    let t=read_transition(&mut tx,transition_ref)?;
    // Stable row locks; both the parent and child must remain unsuperseded,
    // except an exact replay of this same accepted transition.
    let mut keys=vec![t.old_family_ref.clone(),t.new_family_ref.clone()];keys.sort();
    for k in keys{tx.query_one("SELECT family_ref FROM semantic.itir_relational_family WHERE family_ref=$1 FOR UPDATE",&[&k])?;}
    let old=read_manifest(&mut tx,&t.old_family_ref)?;
    let new=read_manifest(&mut tx,&t.new_family_ref)?;
    tx.query_one("SELECT review_item_ref FROM semantic.review_item WHERE review_item_ref=$1 FOR UPDATE",&[&t.review_item_ref])?;
    let item=inspect_review(&mut tx,&transition_review(&t,&old.consumer_ref))?;
    if item.current_status!=ReviewStatus::Accepted{return Err(RelationalFamilyError::ReviewRequired);}
    // Accepted status must have an actual non-promoting S29 command receipt.
    let accepted=tx.query_one(
        "SELECT EXISTS(SELECT 1 FROM semantic.review_receipt WHERE review_item_ref=$1
          AND semantic_ref=$2 AND action_ref='accept' AND effect_ref='status_changed'
          AND effect_value_ref='accepted' AND candidate_only AND NOT creates_semantic_authority
          AND NOT applicability_promoted AND NOT claim_truth_promoted)",&[&t.review_item_ref,&t.transition_ref])?
        .get::<_,bool>(0);
    if !accepted{return Err(RelationalFamilyError::ReviewRequired);}
    read_family_on_client(&mut tx,&old)?;
    read_family_on_client(&mut tx,&new)?;
    if let Some(next)=successor(&mut tx,&t.old_family_ref)?{
        if next!=t.new_family_ref{return Err(RelationalFamilyError::InvalidTransition);}
        let prior:String=tx.query_one("SELECT transition_ref FROM semantic.itir_relational_family_successor WHERE old_family_ref=$1",&[&t.old_family_ref])?.get(0);
        if prior!=t.transition_ref{return Err(RelationalFamilyError::InvalidTransition);}
        tx.commit()?;return Ok(t);
    }
    if successor(&mut tx,&t.new_family_ref)?.is_some(){return Err(RelationalFamilyError::InvalidTransition);}
    tx.execute("INSERT INTO semantic.itir_relational_family_successor
      (old_family_ref,transition_ref,new_family_ref) VALUES($1,$2,$3)",
        &[&t.old_family_ref,&t.transition_ref,&t.new_family_ref])?;
    if successor(&mut tx,&t.old_family_ref)?.as_deref()!=Some(t.new_family_ref.as_str()){
        return Err(RelationalFamilyError::ChangedReplay);
    }
    tx.commit()?;
    Ok(t)
}

/// Load the exact prepared candidate review coordinate for existing S29 UI.
pub fn inspect_relational_family_revision(config:&DatabaseConfig,transition_ref:&str)
    ->Result<FamilyRevisionTransition,RelationalFamilyError>{
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut tx=connection.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let t=read_transition(&mut tx,transition_ref)?;
    tx.commit()?;
    Ok(t)
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
    fn one_revision_replacement_preserves_exact_unaffected_pairs(){
        let a=source("a","p",Polarity::Supports);
        let b=source("b","p",Polarity::Supports);
        let c=source("c","p",Polarity::Counters);
        let d=source("d","p",Polarity::Undetermined);
        let q=consumer();
        let old=family_manifest(&project_relational_family(&[a,b.clone(),c.clone()],&q,"review").unwrap(),"review").unwrap();
        let new=family_manifest(&project_relational_family(&[b,c,d],&q,"review").unwrap(),"review").unwrap();
        let t=revision_transition(&old,&new,"selector:a","selector:d").unwrap();
        assert_eq!(t.retained_comparison_refs.len(),2);
        assert_eq!(t.retired_family_comparison_refs.len(),4);
        assert_eq!(t.new_comparison_refs.len(),4);
        assert!(!t.creates_semantic_authority);
        assert!(revision_transition(&old,&new,"selector:b","selector:d").is_err());
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
    fn declared_transport_requires_direct_bijective_cross_chart_evidence(){
        let a=source("a","p",Polarity::Supports);
        let b=source("b","q",Polarity::Counters);
        let c=source("c","r",Polarity::Undetermined);
        let mut q=consumer();
        for (left,right) in [("p","q"),("p","r")]{
            q.predicate_alignments.push(LicensedAlignment{left_ref:left.into(),right_ref:right.into(),
                witness_ref:format!("w:{left}:{right}"),licence_ref:"declared".into(),
                consumer_ref:q.consumer_ref.clone(),direction:AlignmentDirection::Symmetric});
        }
        let first=project_relational_family(&[a.clone(),b.clone(),c.clone()],&q,"review").unwrap();
        let row=&first.declared_transport_meet.rows[0];
        assert!(!row.complete_declared_coordinate_transport);
        assert!(row.holds.contains(&"missing_direct_cross_chart_transport".into()));
        q.predicate_alignments.push(LicensedAlignment{left_ref:"q".into(),right_ref:"r".into(),
            witness_ref:"w:q:r".into(),licence_ref:"declared".into(),consumer_ref:q.consumer_ref.clone(),
            direction:AlignmentDirection::Symmetric});
        let next=project_relational_family(&[a,b,c],&q,"review").unwrap();
        assert!(next.declared_transport_meet.rows[0].complete_declared_coordinate_transport);
        assert_eq!(next.declared_transport_meet.rows[0].matches.len(),6);
        assert!(!next.declared_transport_meet.formal_preservation_paid);
    }
    #[test]
    fn native_run_specific_refs_match_only_through_shared_source_coordinates(){
        let mut a=source("a","candidate-pnf-db:parser-run:run-a:document-region:source-revision:rev-a:sentence:1000:10-20:8:12:20",Polarity::Supports);
        let mut b=source("b","candidate-pnf-db:parser-run:run-b:document-region:source-revision:rev-b:sentence:1000:10-20:8:12:20",Polarity::Supports);
        a.observation.provenance_refs=vec!["sha256:shared-content".into()];
        b.observation.provenance_refs=vec!["sha256:shared-content".into()];
        let q=consumer();
        let result=project_relational_family(&[a,b],&q,"coordinate-review").unwrap();
        let row=result.declared_transport_meet.rows.iter()
            .find(|r|r.anchor_coordinate.axis=="predicate").unwrap();
        assert!(row.complete_declared_coordinate_transport);
        assert!(row.holds.is_empty());

        let mut c=source("c","candidate-pnf-db:parser-run:run-c:document-region:source-revision:rev-c:sentence:1000:10-20:8:13:20",Polarity::Supports);
        c.observation.provenance_refs=vec!["sha256:shared-content".into()];
        let unmatched=project_relational_family(&[source("a","candidate-pnf-db:parser-run:run-a:document-region:source-revision:rev-a:sentence:1000:10-20:8:12:20",Polarity::Supports),c],&q,"coordinate-review").unwrap();
        let row=unmatched.declared_transport_meet.rows.iter()
            .find(|r|r.anchor_coordinate.axis=="predicate").unwrap();
        assert!(!row.complete_declared_coordinate_transport);
        assert!(row.holds.contains(&"missing_bidirectional_coordinate".into()));
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


/// Query the exact selected source inventory, not all possible readings.
/// The coordinate retains role and occurrence; no referent alias is inferred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecordedParticipantAnswer {
    pub selector_ref: String,
    pub observation_ref: String,
    pub revision_ref: String,
    pub predicate_candidate_ref: String,
    pub coordinate: ObservedCoordinate,
}

pub fn recorded_participant_query(
    family: &PersistedRelationalFamily, filler_ref: &str,
) -> serde_json::Value {
    let answers: Vec<RecordedParticipantAnswer> = family.projection.sources.iter()
        .flat_map(|s| s.observation.role_bindings.iter().filter_map(move |r| {
            if r.filler_candidate_ref != filler_ref { return None; }
            Some(RecordedParticipantAnswer {
                selector_ref: s.selector_ref.clone(),
                observation_ref: s.observation.observation_ref.clone(),
                revision_ref: s.observation.source_revision_ref.clone(),
                predicate_candidate_ref: s.observation.predicate_candidate_ref.clone(),
                coordinate: ObservedCoordinate {
                    axis: "role".into(), slot: r.role_ref.clone(),
                    occurrence: r.occurrence, value_ref: r.filler_candidate_ref.clone(),
                },
            })
        })).collect();
    serde_json::json!({
        "schema": "slr.recorded-participant-query.v1",
        "family_ref": family.family_ref,
        "consumer_ref": family.projection.consumer_ref,
        "filler_ref": filler_ref,
        "answers": answers,
        "selected_source_count": family.projection.sources.len(),
        "all_live_semantic_readings_covered": false,
        "governed_identity_paid": false,
        "world_truth_paid": false
    })
}


/// Preserve both directional comparisons and their residuals for a selected
/// source pair. A polarity conflict is not an established contradiction
/// until semantic correspondence and context have independently been paid.
pub fn recorded_reciprocal_query(
    family: &PersistedRelationalFamily, left: &str, right: &str,
) -> Result<serde_json::Value, RelationalFamilyError> {
    if left == right { return Err(RelationalFamilyError::InvalidSelector); }
    let p = &family.projection;
    let a = p.sources.iter().find(|s| s.selector_ref == left)
        .ok_or(RelationalFamilyError::InvalidSelector)?;
    let b = p.sources.iter().find(|s| s.selector_ref == right)
        .ok_or(RelationalFamilyError::InvalidSelector)?;
    let f = p.directed_comparisons.iter().find(|c|
        c.left_selector_ref == left && c.right_selector_ref == right)
        .ok_or(RelationalFamilyError::ChangedReplay)?;
    let r = p.directed_comparisons.iter().find(|c|
        c.left_selector_ref == right && c.right_selector_ref == left)
        .ok_or(RelationalFamilyError::ChangedReplay)?;
    Ok(serde_json::json!({
        "schema": "slr.recorded-reciprocal-query.v1",
        "family_ref": family.family_ref,
        "consumer_ref": p.consumer_ref,
        "left": a, "right": b, "forward": f, "backward": r,
        "source_polarity_pair": [a.observation.polarity, b.observation.polarity],
        "all_live_semantic_readings_covered": false,
        "semantic_equivalence_paid": false,
        "world_truth_paid": false
    }))
}


/// Exact binding of a bounded semantic answer to the existing native source
/// pair. The independently issued referee receipt, not caller JSON, supplies
/// the answer and confirms the actual scoped theorem. This read-only weld
/// does not create an admission path or a new proof store.
fn reciprocal_proof_binding(
    family:&PersistedRelationalFamily,left:&str,right:&str,query:&str,
    statement_digest:&str,evidence_ref:&str,answer:&serde_json::Value,
)->Result<serde_json::Value,RelationalFamilyError>{
    use serde_json::json;
    if query.trim().is_empty()||statement_digest.trim().is_empty()||evidence_ref.trim().is_empty()
        ||answer.is_null()||family.superseded_by.is_some(){
        return Err(RelationalFamilyError::ReciprocalProofBinding);
    }
    let recorded=recorded_reciprocal_query(family,left,right)?;
    let p=&family.projection;
    let a=p.sources.iter().find(|s|s.selector_ref==left).ok_or(RelationalFamilyError::InvalidSelector)?;
    let b=p.sources.iter().find(|s|s.selector_ref==right).ok_or(RelationalFamilyError::InvalidSelector)?;
    Ok(json!({
        "schema":"slr-reciprocal-source-query-binding-v1",
        "proof_scope":"bounded-source-proposition",
        "family_ref":family.family_ref,"consumer_ref":p.consumer_ref,"query_ref":query,
        "left_selector_ref":left,"right_selector_ref":right,
        "left_source_revision_ref":a.observation.source_revision_ref,
        "right_source_revision_ref":b.observation.source_revision_ref,
        "left_observation_ref":a.observation.observation_ref,
        "right_observation_ref":b.observation.observation_ref,
        "left_observation_sha256":digest_bytes(&serde_json::to_vec(&a.observation)?),
        "right_observation_sha256":digest_bytes(&serde_json::to_vec(&b.observation)?),
        "forward_comparison_ref":recorded["forward"]["comparison"]["comparison_ref"],
        "backward_comparison_ref":recorded["backward"]["comparison"]["comparison_ref"],
        "statement_digest":statement_digest,"evidence_ref":evidence_ref,
        "answer_sha256":digest_bytes(&serde_json::to_vec(answer)?)
    }))
}

/// Reopen current native family and the existing independently authenticated
/// formal evidence in ONE serializable read-only transaction. The exact
/// domain binding is issued by the trusted referee for the same pair/query.
/// A checked generic theorem, historical proof or differently scoped answer
/// cannot pay this query. The existing family lineage is the currentness gate.
pub fn read_authenticated_reciprocal_query(
    config:&DatabaseConfig,family_ref:&str,left:&str,right:&str,query:&str,
    evidence_ref:&str,locator:&sensiblaw_formal_proof_bridge::RefereeReceiptLocator,
)->Result<serde_json::Value,RelationalFamilyError>{
    use serde_json::{json,Value};
    use sensiblaw_formal_proof_bridge::{reopen_authenticated_formal_evidence_tx,
        read_source_bound_referee_receipt};
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut tx=connection.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let manifest=read_manifest(&mut tx,family_ref)?;
    let family=read_family_on_client(&mut tx,&manifest)?;
    let recorded=recorded_reciprocal_query(&family,left,right)?;
    let checked=reopen_authenticated_formal_evidence_tx(&mut tx,evidence_ref,locator)?;
    let payment=checked.payment();
    let referee:Value=serde_json::from_str(&read_source_bound_referee_receipt(&mut tx,locator)?)?;
    let answer=referee.get("slr_reciprocal_query_answer").ok_or(RelationalFamilyError::ReciprocalProofBinding)?;
    let binding=reciprocal_proof_binding(&family,left,right,query,&payment.statement_digest,evidence_ref,answer)?;
    let premise=&referee["slr_premise_binding"];
    let revisions=premise["source_revisions"].as_array().ok_or(RelationalFamilyError::ReciprocalProofBinding)?;
    if referee["slr_reciprocal_query_binding"]!=binding
        ||premise["consumer_ref"]!=family.projection.consumer_ref
        ||!revisions.contains(&binding["left_source_revision_ref"])
        ||!revisions.contains(&binding["right_source_revision_ref"])
        ||!payment.formal_premise_paid||payment.creates_legal_authority
        ||payment.applicability_promoted||payment.claim_truth_promoted{
        return Err(RelationalFamilyError::ReciprocalProofBinding);
    }
    let authenticated_formal_premise=json!({
        "demand_ref":payment.demand_ref,
        "obligation_ref":payment.obligation_ref,
        "evidence_ref":payment.evidence_ref,
        "statement_digest":payment.statement_digest,
        "formal_premise_paid":payment.formal_premise_paid,
        "reopen_refs":payment.reopen_refs,
        "creates_legal_authority":payment.creates_legal_authority,
        "applicability_promoted":payment.applicability_promoted,
        "claim_truth_promoted":payment.claim_truth_promoted
    });
    let result=json!({
        "schema":"slr.authenticated-bounded-reciprocal-query.v1",
        "family_ref":family_ref,"query_ref":query,"binding":binding,
        "answer":answer,"recorded_comparison":recorded,
        "authenticated_formal_premise":authenticated_formal_premise,"proof_scope":"bounded-source-proposition",
        "creates_semantic_authority":false,"all_live_semantic_readings_covered":false,
        "governed_identity_paid":false,"world_truth_paid":false,
        "writes_database":false
    });
    tx.commit()?;
    Ok(result)
}


/// Bind the exact candidate receipt and one original predicate/query to the
/// independently issued bounded answer. Receipt digest is over the complete
/// reconstructed native JSON, including outside-model and residual lanes.
fn source_semantic_proof_binding(receipt:&serde_json::Value,revision:&str,
    observation:&str,consumer:&str,query:&str,statement_digest:&str,evidence_ref:&str,
    answer:&serde_json::Value)->Result<serde_json::Value,RelationalFamilyError>{
    use serde_json::json;
    if [revision,observation,consumer,query,statement_digest,evidence_ref].iter()
        .any(|s|s.trim().is_empty()) || answer.is_null()
        ||receipt["schema"]!="slr.source-semantic-candidate-fragments.v1"
        ||receipt["original_transport"]["source_coordinate_meet"]["source_revision_ref"]!=revision{
        return Err(RelationalFamilyError::ReciprocalProofBinding);
    }
    let rows=receipt["rows"].as_array().ok_or(RelationalFamilyError::ReciprocalProofBinding)?;
    let mut selected=rows.iter().filter(|r|
        r["original_predicate_transport"]["checked_observation"]["observation_ref"]==observation);
    let row=selected.next().ok_or(RelationalFamilyError::ReciprocalProofBinding)?;
    if selected.next().is_some(){return Err(RelationalFamilyError::ReciprocalProofBinding);}
    Ok(json!({"schema":"slr-source-semantic-query-binding-v1",
        "proof_scope":"bounded-source-predicate-query","source_revision_ref":revision,
        "observation_ref":observation,"consumer_ref":consumer,"query_ref":query,
        "candidate_receipt_sha256":digest_bytes(&serde_json::to_vec(receipt)?),
        "predicate_row_sha256":digest_bytes(&serde_json::to_vec(row)?),
        "statement_digest":statement_digest,"evidence_ref":evidence_ref,
        "answer_sha256":digest_bytes(&serde_json::to_vec(answer)?)}))
}

/// Reopen immutable source candidate receipt, then existing formal evidence.
/// The source producers use independent read snapshots: this is a bounded
/// historical source answer, not a current shared authority snapshot or a
/// durable semantic admission. Referee must issue the exact domain binding.
pub fn read_authenticated_source_semantic_query(
    config:&DatabaseConfig,revision:&str,batches:&[String],run:&str,doc:&str,
    snapshot:&str,digest:&str,observation:&str,consumer:&str,query:&str,
    evidence_ref:&str,locator:&sensiblaw_formal_proof_bridge::RefereeReceiptLocator,
)->Result<serde_json::Value,RelationalFamilyError>{
    use serde_json::{json,Value};
    use sensiblaw_formal_proof_bridge::{reopen_authenticated_formal_evidence_tx,
        read_source_bound_referee_receipt};
    let receipt=crate::read_article_semantic_candidates(config,revision,batches,run,doc,snapshot,digest)?;
    let mut connection=Client::connect(config.database_url(),NoTls)?;
    let mut tx=connection.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE READ ONLY; SET LOCAL statement_timeout='90000ms'")?;
    let checked=reopen_authenticated_formal_evidence_tx(&mut tx,evidence_ref,locator)?;
    let payment=checked.payment();
    let referee:Value=serde_json::from_str(&read_source_bound_referee_receipt(&mut tx,locator)?)?;
    let answer=referee.get("slr_source_semantic_query_answer").ok_or(RelationalFamilyError::ReciprocalProofBinding)?;
    let binding=source_semantic_proof_binding(&receipt,revision,observation,consumer,query,
        &payment.statement_digest,evidence_ref,answer)?;
    let premise=&referee["slr_premise_binding"];
    let revisions=premise["source_revisions"].as_array().ok_or(RelationalFamilyError::ReciprocalProofBinding)?;
    if referee["slr_source_semantic_query_binding"]!=binding
        ||premise["consumer_ref"]!=consumer
        ||!revisions.contains(&json!(revision))
        ||!payment.formal_premise_paid||payment.creates_legal_authority
        ||payment.applicability_promoted||payment.claim_truth_promoted{
        return Err(RelationalFamilyError::ReciprocalProofBinding);
    }
    let result=json!({"schema":"slr.authenticated-bounded-source-semantic-query.v1",
        "binding":binding,"answer":answer,"candidate_receipt":receipt,
        "formal_premise_paid":true,"reopen_refs":payment.reopen_refs,
        "common_current_authority_snapshot":false,"creates_semantic_authority":false,
        "all_live_semantic_readings_covered":false,"governed_identity_paid":false,
        "world_truth_paid":false,"writes_database":false});
    tx.commit()?;
    Ok(result)
}

#[cfg(test)]
mod source_semantic_binding_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn exact_predicate_and_answer_binding_changes_on_residual_or_answer_change(){
        let receipt=json!({"schema":"slr.source-semantic-candidate-fragments.v1",
            "original_transport":{"source_coordinate_meet":{"source_revision_ref":"rev"}},
            "rows":[{"original_predicate_transport":{"checked_observation":{"observation_ref":"obs"}},
                "outside_model_possible":true}]});
        let bind=|r:&serde_json::Value,a:&serde_json::Value|
            source_semantic_proof_binding(r,"rev","obs","consumer","query","stmt","ev",a).unwrap();
        let a=json!({"sign":0});
        let baseline=bind(&receipt,&a);
        assert_ne!(baseline,bind(&receipt,&json!({"sign":1})));
        let mut changed=receipt.clone();
        changed["rows"][0]["outside_model_possible"]=json!(false);
        assert_ne!(baseline,bind(&changed,&a));
        assert!(source_semantic_proof_binding(&receipt,"rev","foreign","consumer","query","stmt","ev",&a).is_err());
    }
}

#[cfg(test)]
mod recorded_syntax_parity_tests{
    use super::*;
    use crate::relational_interlingua::{ObservationContext,RoleBinding};
    fn source(id:&str,extra:bool)->SelectedRelationalSource{
        let span=format!("document-region:source-revision:{id}:sentence:1:0-20");
        let owner=format!("parser-run:{id}:{span}");
        let candidate=|tail:&str|format!("candidate-pnf-db:{owner}:{tail}");
        let mut roles=vec![RoleBinding{role_ref:"syntax:nsubj".into(),
            filler_candidate_ref:candidate("0:0:3"),occurrence:0}];
        if extra{roles.push(RoleBinding{role_ref:"syntax:dobj".into(),
            filler_candidate_ref:candidate("2:9:12"),occurrence:0});}
        SelectedRelationalSource{selector_ref:format!("selector:{id}"),observation:RelationalObservation{
            observation_ref:format!("obs:{id}"),source_revision_ref:format!("source-revision:{id}"),
            source_family:crate::RelationalSourceFamily::Other,source_span_ref:span,
            parser_or_producer_ref:format!("db-parser:{owner}"),predicate_candidate_ref:candidate("1:4:8"),
            role_bindings:roles,role_type_hypotheses:vec![],context:ObservationContext::default(),
            candidate_type_refs:vec![],provenance_refs:vec![],native_metadata_refs:vec![],
            polarity:Polarity::Undetermined,candidate_only:true,creates_semantic_authority:false,
            claim_truth_promoted:false}}
    }
    #[test]
    fn recorded_query_preserves_inverse_and_rejects_extra_right_member(){
        let a=source("a",false);let b=source("b",false);
        let equal=recorded_syntax_pair(&a,&b,"digest","digest");
        assert_eq!(equal["recorded_answer_equal"],true);
        assert_eq!(equal["common"].as_array().unwrap().len(),2);
        let more=recorded_syntax_pair(&a,&source("b",true),"digest","digest");
        assert_eq!(more["recorded_answer_equal"],false);
        assert_eq!(more["right_only"].as_array().unwrap().len(),1);
        let foreign=recorded_syntax_pair(&a,&b,"digest","different");
        assert!(foreign["recorded_answer_equal"].is_null());
        let mut forged=b;forged.observation.predicate_candidate_ref=a.observation.predicate_candidate_ref.clone();
        assert_eq!(recorded_syntax_pair(&a,&forged,"digest","digest")["status"],"invalid_checked_coordinate");
    }
}
