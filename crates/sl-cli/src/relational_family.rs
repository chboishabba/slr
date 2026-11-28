//! Native family/workbench operations over existing PG owners.
use sensiblaw_pg_source_store::{
    load_database_config,run_persisted_relational_family,inspect_persisted_relational_family,
    prepare_relational_family_revision,publish_relational_family_revision,inspect_relational_family_revision,
    apply_persisted_review_command,ReviewAction,ReviewCommand,load_rel_acceptance_corpus,
    seed_relational_pnf_pair,seed_relational_pnf_pair_scoped,inspect_recorded_family_parity,inspect_family_local_corpus,
};
fn print_json<T:serde::Serialize>(value:&T)->Result<(),String>{
    println!("{}",serde_json::to_string_pretty(value).map_err(|e|e.to_string())?);Ok(())
}

/// Project the exact native family residuals into the existing proof-search
/// frontier. Acquisition completion is not semantic payment: it defers the
/// old residual for reviewed re-entry. No similarity score is a proof input.
fn family_research_frontier_native(family:&sensiblaw_pg_source_store::PersistedRelationalFamily)
    ->Result<(sensiblaw_proof_search_loop::frontier::ProofFrontier,Vec<serde_json::Value>),String>{
    use sensiblaw_proof_search_loop::frontier::{ProofFrontier,ProofResidual,ResidualStatus};
    use std::collections::BTreeSet;
    use sha2::{Digest,Sha256};
    use serde_json::json;
    let p=&family.projection;
    if !p.candidate_only || p.formal_consumer_equivalence_paid
        || p.all_live_reading_coverage_paid || p.creates_edit_authority
        || family.creates_access_authority || family.applies_external_edit {
        return Err("promotion-bearing family".into());
    }
    let mut frontier=ProofFrontier{consumer_ref:p.consumer_ref.clone(),
        frontier_ref:format!("frontier:relational:{}",family.family_ref),residuals:vec![],
        satisfied_payment_refs:vec![],contested_coordinate_refs:vec![],
        authority_blocked_refs:vec![],authority:"experimental_candidate_only"};
    let mut inverse=Vec::new();
    let mut seen=BTreeSet::new();
    for pair in &p.directed_comparisons {
        let c=&pair.comparison;
        if c.consumer_ref!=p.consumer_ref || c.creates_semantic_authority
            || c.claim_truth_promoted || c.merges_sources || c.proves_independence {
            return Err("wrong consumer or comparison promotion".into());
        }
        for (ordinal,r) in c.residuals.iter().enumerate(){
            let mut demands=p.acquisition_demands.iter().filter(|d|
                d.obligation.comparison_ref==c.comparison_ref &&
                d.obligation.residual_obligation_ref==r.obligation_ref &&
                d.exact_residuals.contains(r));
            let d=demands.next().ok_or("missing exact acquisition demand")?;
            if demands.next().is_some() || !d.obligation.candidate_only
                || d.obligation.creates_semantic_authority || d.obligation.claim_truth_promoted {
                return Err("ambiguous or promoted acquisition demand".into());
            }
            let mut lifecycles=family.acquisition_lifecycles.iter()
                .filter(|l|l.obligation_ref==d.obligation.obligation_ref);
            let l=lifecycles.next().ok_or("missing durable acquisition lifecycle")?;
            if lifecycles.next().is_some(){return Err("ambiguous lifecycle".into());}
            let encoded=serde_json::to_vec(&(&c.comparison_ref,ordinal,r))
                .map_err(|e|e.to_string())?;
            let residual_ref=format!("residual:relational:sha256:{:x}",Sha256::digest(encoded));
            if !seen.insert(residual_ref.clone()){return Err("duplicate exact residual".into());}
            let mut dependencies:BTreeSet<String>=d.obligation.source_revision_refs.iter().cloned().collect();
            dependencies.extend(d.obligation.dependency_target_refs.iter().cloned());
            dependencies.extend([c.comparison_ref.clone(),d.obligation.obligation_ref.clone(),
                d.obligation.authority_or_access_constraint_ref.clone()]);
            dependencies.extend(r.left_ref.iter().chain(r.right_ref.iter()).cloned());
            let deferred=family.superseded_by.is_some() || l.terminal_update.is_some();
            let status=if deferred{ResidualStatus::Underidentified}else{ResidualStatus::Open};
            frontier.residuals.push(ProofResidual{residual_ref:residual_ref.clone(),
                proposition_ref:format!("consumer-obligation:{}",r.obligation_ref),
                producer_class_ref:"producer:relational-discriminator".into(),
                jurisdiction_ref:None,authority_requirement_ref:None,salience:1,
                dependency_refs:dependencies.into_iter().collect(),status});
            inverse.push(json!({"residual_ref":residual_ref,"comparison_ref":c.comparison_ref,
                "ordinal":ordinal,"exact_residual":r,"obligation":d.obligation,
                "lifecycle":l,"requires_reviewed_reentry":deferred}));
        }
    }
    Ok((frontier,inverse))
}
fn family_research_frontier(family:&sensiblaw_pg_source_store::PersistedRelationalFamily)
    ->Result<serde_json::Value,String>{
    use sensiblaw_proof_search_loop::{frontier::ResidualStatus,hypothesis::families_for_frontier};
    use serde_json::json;
    let (frontier,inverse)=family_research_frontier_native(family)?;
    let residuals:Vec<_>=frontier.residuals.iter().map(|r|json!({
        "residual_ref":r.residual_ref,"proposition_ref":r.proposition_ref,
        "producer_class_ref":r.producer_class_ref,"dependency_refs":r.dependency_refs,
        "status":if r.status==ResidualStatus::Open{"open"}else{"underidentified"}
    })).collect();
    let hypotheses:Vec<_>=families_for_frontier(&frontier).iter().map(|h|json!({
        "hypothesis_ref":h.hypothesis_ref,"residual_ref":h.residual_ref,
        "target_proposition_ref":h.target_proposition_ref,"kind":format!("{:?}",h.kind),
        "candidate_only":true
    })).collect();
    Ok(json!({"schema":"slr.relational-research-frontier.v1","family_ref":family.family_ref,
        "consumer_ref":frontier.consumer_ref,"frontier_ref":frontier.frontier_ref,
        "superseded_by":family.superseded_by,"residuals":residuals,"inverse":inverse,
        "hypotheses":hypotheses,"satisfied_payment_refs":frontier.satisfied_payment_refs,
        "candidate_only":true,"formal_consumer_equivalence_paid":false,
        "world_truth_paid":false,"creates_access_authority":false,"writes_database":false,
        "planning_inputs_required":["consumer-owned lexical contexts","declared calibrated values","local index","research world snapshot"]}))
}


#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalCalibration {
    family_ref:String,
    consumer_ref:String,
    minimum_expected_reduction:u64,
    hypotheses:std::collections::BTreeMap<String,LocalHypothesisCalibration>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalHypothesisCalibration {
    calibration_ref:String,
    expected_residual_reduction:u64,
    discriminative_value:u64,
    authority_fitness:u64,
    novelty:u64,
    coverage_gain:u64,
    local_bytes_read_cost:u64,
    parser_pnf_cost:u64,
    semantic_assessment_cost:u64,
    operator_review_cost:u64,
    // Explicit additional terms, never inferred evidence or a semantic licence.
    target_terms:Vec<String>,
    discriminator_terms:Vec<String>,
}

/// Local operator-only research over the original persisted text. Without
/// explicit calibration execute the existing lexical candidate retrieval,
/// but do not rank it, claim a separator or create executable provider routes.
fn inspect_local_research(config:&sensiblaw_pg_source_store::DatabaseConfig,
    family_ref:&str,calibration:Option<LocalCalibration>)->Result<serde_json::Value,String>{
    use sensiblaw_proof_search_loop::{local::{LocalIndex,LocalDocument},
        world::ResearchWorldSnapshot,hypothesis::{families_for_frontier,SearchHypothesisKind},
        planner::{QueryLexicalContext,synthesize_queries,execute_local_candidates,deterministic_query_key},
        engine::{DeclaredResearchValue,plan_local_frontier_research},
        frontier::{frontier_pareto,select_frontier_move}};
    use std::collections::{BTreeMap,BTreeSet};
    use serde_json::json;
    let (family,corpus)=inspect_family_local_corpus(config,family_ref).map_err(|e|e.to_string())?;
    let (frontier,inverse)=family_research_frontier_native(&family)?;
    let hypotheses=families_for_frontier(&frontier);
    let mut index=LocalIndex::default();
    let mut texts=BTreeMap::new();
    let mut source_manifest=Vec::new();
    for (source,text) in corpus{
        source_manifest.push(json!({"document_ref":source.document_ref,"source_revision_ref":source.source_revision_ref,
            "content_digest_ref":source.content_digest_ref,"source_family":source.source_family_ref}));
        texts.insert(source.source_revision_ref.clone(),text.clone());
        index.insert(LocalDocument{document_ref:source.document_ref,source_revision_ref:source.source_revision_ref,
            jurisdiction_ref:None,court_ref:None,date_ref:None,canonical_text:text,citation_refs:vec![]});
    }
    let world=ResearchWorldSnapshot{snapshot_ref:format!("local-family-corpus:{family_ref}"),
        authority:"experimental_candidate_only",..Default::default()};
    if let Some(c)=&calibration{
        let expected:BTreeSet<_>=hypotheses.iter().map(|h|h.hypothesis_ref.as_str()).collect();
        if c.family_ref!=family_ref||c.consumer_ref!=frontier.consumer_ref||
            c.hypotheses.keys().map(String::as_str).collect::<BTreeSet<_>>()!=expected{
            return Err("calibration must bind exactly to the current open hypothesis universe".into());
        }
    }
    let mut contexts=BTreeMap::new();let mut values=BTreeMap::new();
    let mut seeds=Vec::new();let mut deferred=Vec::new();
    for h in &hypotheses{
        let row=inverse.iter().find(|r|r["residual_ref"].as_str()==Some(h.residual_ref.as_str()))
            .ok_or("missing native inverse")?;
        let pair=family.projection.directed_comparisons.iter().find(|p|
            row["comparison_ref"].as_str()==Some(p.comparison.comparison_ref.as_str()))
            .ok_or("missing native comparison")?;
        let obs=&family.projection.sources.iter().find(|s|s.selector_ref==pair.left_selector_ref)
            .ok_or("missing source")?.observation;
        let owner=obs.parser_or_producer_ref.strip_prefix("db-parser:").ok_or("not a checked native parser")?;
        let prefix=format!("candidate-pnf-db:{owner}:");
        let mut parts=obs.predicate_candidate_ref.strip_prefix(prefix.as_str()).ok_or("foreign predicate coordinate")?.split(':');
        let _ordinal=parts.next().ok_or("missing token ordinal")?.parse::<u32>().map_err(|e|e.to_string())?;
        let start=parts.next().ok_or("missing span start")?.parse::<usize>().map_err(|e|e.to_string())?;
        let end=parts.next().ok_or("missing span end")?.parse::<usize>().map_err(|e|e.to_string())?;
        if end<=start||parts.next().is_some(){return Err("invalid predicate span".into());}
        let text=texts.get(&obs.source_revision_ref).ok_or("missing original source bytes")?;
        let literal=text.chars().skip(start).take(end-start).collect::<String>();
        if literal.chars().count()!=end-start||!literal.chars().any(char::is_alphanumeric){
            return Err("predicate has no bounded searchable literal".into());
        }
        let mut ctx=QueryLexicalContext{target_phrases:vec![literal.clone()],..Default::default()};
        if let Some(c)=&calibration{
            let v=&c.hypotheses[&h.hypothesis_ref];
            if v.calibration_ref.trim().is_empty()||v.target_terms.iter().chain(v.discriminator_terms.iter()).any(|t|t.trim().is_empty()){
                return Err("invalid declared lexical or calibration coordinate".into());
            }
            ctx.target_terms=v.target_terms.clone();
            match h.kind{
                SearchHypothesisKind::Support=>ctx.support_terms=v.discriminator_terms.clone(),
                SearchHypothesisKind::Defeater=>ctx.defeater_terms=v.discriminator_terms.clone(),
                SearchHypothesisKind::Comparator=>ctx.comparator_terms=v.discriminator_terms.clone(),
                SearchHypothesisKind::Contradiction=>ctx.contradiction_terms=v.discriminator_terms.clone(),
                SearchHypothesisKind::AuthorityTreatment=>ctx.treatment_terms=v.discriminator_terms.clone(),
                SearchHypothesisKind::TerminologyExpansion=>ctx.target_terms.extend(v.discriminator_terms.clone()),
            }
            let mut value=DeclaredResearchValue{expected_residual_reduction:v.expected_residual_reduction,
                discriminative_value:v.discriminative_value,authority_fitness:v.authority_fitness,
                novelty:v.novelty,coverage_gain:v.coverage_gain,execution_cost:Default::default(),
                calibration_ref:v.calibration_ref.clone()};
            value.execution_cost.local_bytes_read_cost=v.local_bytes_read_cost;
            value.execution_cost.parser_pnf_cost=v.parser_pnf_cost;
            value.execution_cost.semantic_assessment_cost=v.semantic_assessment_cost;
            value.execution_cost.operator_review_cost=v.operator_review_cost;
            values.insert(h.hypothesis_ref.clone(),value);
        }
        seeds.push(json!({"hypothesis_ref":h.hypothesis_ref,"source_revision_ref":obs.source_revision_ref,
            "predicate_candidate_ref":obs.predicate_candidate_ref,"literal":literal,"span":[start,end],
            "candidate_only":true}));
        contexts.insert(h.hypothesis_ref.clone(),ctx);
    }
    let mut retrieval=Vec::new();
    let mut queries=BTreeMap::new();
    for h in &hypotheses{
        for q in synthesize_queries(h,&contexts[&h.hypothesis_ref],&world){
            let key=deterministic_query_key(&q);
            let entry=queries.entry(key).or_insert_with(||(q.clone(),Vec::<String>::new()));
            entry.1.push(h.hypothesis_ref.clone());
        }
    }
    for (key,(query,owners)) in queries{
        for x in execute_local_candidates(&index,&[query]){
            let passages:Vec<_>=x.passages.iter().map(|p|json!({"document_ref":p.document_ref,
                "source_revision_ref":p.source_revision_ref,"matched_expression_ref":p.matched_expression_ref,
                "candidate_only":p.candidate_only})).collect();
            if passages.is_empty(){deferred.extend(owners.iter().cloned());}
            retrieval.push(json!({"query_key":key,"hypothesis_refs":owners,"passages":passages,
                "network_requests":x.network_requests,"candidate_only":true}));
        }
    }
    let calibrated=calibration.is_some();
    let (planned,pareto,selected)=if let Some(c)=calibration{
        let moves=plan_local_frontier_research(&frontier,&world,&index,&contexts,&values)
            .map_err(|e|format!("{e:?}"))?;
        let candidates:Vec<_>=moves.iter().map(|m|m.frontier_move.clone()).collect();
        let pareto:Vec<_>=frontier_pareto(&frontier,&candidates,c.minimum_expected_reduction)
            .iter().map(|m|m.move_.move_ref.clone()).collect();
        let selected=select_frontier_move(&frontier,&candidates,c.minimum_expected_reduction)
            .map(|m|m.move_.move_ref.clone());
        let planned:Vec<_>=moves.iter().map(|m|json!({"move_ref":m.frontier_move.move_.move_ref,
            "query_key":m.query_key,"target_residual_refs":m.target_residual_refs,
            "supporting_hypothesis_refs":m.supporting_hypothesis_refs,
            "declared_calibration_ref":m.frontier_move.move_.calibration_ref,
            "expected_whole_frontier_reduction":m.frontier_move.expected_whole_frontier_reduction,
            "candidate_only":true})).collect();
        (planned,pareto,selected)
    }else{(vec![],vec![],None)};
    Ok(json!({"schema":"slr.relational-local-research.v1","family_ref":family_ref,
        "consumer_ref":frontier.consumer_ref,"frontier_ref":frontier.frontier_ref,
        "source_manifest":source_manifest,"lexical_seeds":seeds,"retrieval":retrieval,
        "no_hit_hypothesis_refs":deferred,"planned_moves":planned,"pareto_move_refs":pareto,
        "selected_move_ref":selected,"scope":"selected persisted generic source revisions only",
        "retrieval_limitations":["source-local predicate literal seeds, not proposition correspondence","no separately loaded citations or world vocabulary","no provider dispatch; additional calibrated discriminator terms required for ranked planning"],
        "calibration_is_operator_declared":calibrated,"candidate_only":true,"network_requests":0,
        "creates_access_authority":false,"world_truth_paid":false,"formal_consumer_equivalence_paid":false,
        "writes_database":false,"provider_acquisition_executed":false}))
}


fn diagnostic_lane(kind:&sensiblaw_pg_source_store::ResidualKind,unobserved_context:bool,
    recorded_transport:bool)->(&'static str,&'static str){
    use sensiblaw_pg_source_store::ResidualKind::*;
    if unobserved_context{return ("unavailable_observation","reopen_native_context_producers");}
    match kind{
        UnalignedPredicate|UnalignedRoleFiller if recorded_transport=>
            ("recorded_correspondence_unlicensed","consumer_factorisation_or_semantic_review"),
        UnalignedPredicate|UnalignedRoleFiller|UnalignedType=>
            ("correspondence_ambiguity","typed_candidate_retrieval_and_discriminator"),
        MissingRequiredRole|MissingRoleTypeEvidence=>
            ("unavailable_observation","reopen_native_role_evidence"),
        MissingProvenance=>("provenance_gap","reopen_source_owner"),
        RoleTypeContractPressure=>("semantic_discriminator","consumer_role_type_review"),
        UnresolvedPolarity=>("semantic_discriminator","signed_source_reading_review"),
        ScopeMismatch|TimeMismatch|ModalityMismatch|QuantifierMismatch|
        AttributionMismatch|OntologyMismatch|LanguageMismatch=>
            ("context_discriminator","consumer_context_review"),
    }
}

/// Mabo-style source -> observation -> comparison -> exact residual ->
/// obligation/review/hypothesis/route ancestry, derived from the existing
/// reopened family only. This is a workbench projection, not another store,
/// formal demand constructor, search scheduler or access authority.
fn family_investigation_projection(family:&sensiblaw_pg_source_store::PersistedRelationalFamily)
    ->Result<serde_json::Value,String>{
    use sensiblaw_pg_source_store::{route_frontier_dispositions,potential_reopening_cone};
    use sensiblaw_proof_search_loop::hypothesis::families_for_frontier;
    use std::collections::{BTreeMap,BTreeSet};
    use serde_json::json;
    let (frontier,inverse)=family_research_frontier_native(family)?;
    let hypotheses=families_for_frontier(&frontier);
    let p=&family.projection;
    let mut edges=BTreeSet::<(String,String,String)>::new();
    let mut edge=|from:&str,to:&str,relation:&str|{
        edges.insert((from.to_owned(),to.to_owned(),relation.to_owned()));
    };
    for s in &p.sources{
        edge(&s.observation.source_revision_ref,&s.observation.observation_ref,"source_observation");
        edge(&s.observation.observation_ref,&s.selector_ref,"observation_selector");
    }
    for pair in &p.directed_comparisons{
        edge(&pair.left_selector_ref,&pair.comparison.comparison_ref,"left_input");
        edge(&pair.right_selector_ref,&pair.comparison.comparison_ref,"right_input");
    }
    let mut rows=Vec::new();
    let mut counts=BTreeMap::<&str,usize>::new();
    let mut routes=BTreeMap::new();
    for q in &family.durable_acquisition_queues{
        let disposition=route_frontier_dispositions(&q.obligation,&q.routes).map_err(|e|e.to_string())?;
        if routes.insert(q.obligation.obligation_ref.as_str(),(q,disposition)).is_some(){
            return Err("duplicate durable acquisition queue".into());
        }
    }
    for item in &inverse{
        let residual_ref=item["residual_ref"].as_str().ok_or("missing residual ref")?;
        let comparison_ref=item["comparison_ref"].as_str().ok_or("missing comparison ref")?;
        let ordinal=item["ordinal"].as_u64().ok_or("missing residual ordinal")? as usize;
        let pair=p.directed_comparisons.iter().find(|c|c.comparison.comparison_ref==comparison_ref)
            .ok_or("missing comparison")?;
        let r=pair.comparison.residuals.get(ordinal).ok_or("missing residual")?;
        let obligation_ref=item["obligation"]["obligation_ref"].as_str().ok_or("missing obligation")?;
        let (queue,dispositions)=routes.get(obligation_ref).ok_or("missing durable queue")?;
        let mut reviews=family.residual_review_items.iter().filter(|v|
            v.obligation_ref==obligation_ref && v.comparison_ref==comparison_ref && v.exact_residuals.contains(r));
        let review=reviews.next().ok_or("missing exact review")?;
        if reviews.next().is_some()||!review.candidate_only{return Err("ambiguous or promoted review".into());}
        let lifecycle=family.acquisition_lifecycles.iter().find(|l|l.obligation_ref==obligation_ref)
            .ok_or("missing lifecycle")?;
        let unobserved=r.obligation_ref.strip_prefix("consumer:context-unobserved:")
            .is_some_and(|axis|pair.unobserved_required_context_axes.iter().any(|a|a==axis));
        let transport:Vec<_>=p.declared_transport_meet.rows.iter()
            .filter(|row|row.complete_declared_coordinate_transport)
            .flat_map(|row|row.matches.iter())
            .filter(|m|m.from_selector_ref==pair.left_selector_ref && m.to_selector_ref==pair.right_selector_ref
                && r.left_ref.as_deref()==Some(m.from.value_ref.as_str())
                && r.right_ref.as_deref()==Some(m.to.value_ref.as_str())
                && matches!(m.from.axis.as_str(),"predicate"|"role"))
            .collect();
        let (lane,next_owner)=diagnostic_lane(&r.kind,unobserved,!transport.is_empty());
        *counts.entry(lane).or_default()+=1;
        edge(comparison_ref,residual_ref,"exact_comparison_residual");
        edge(residual_ref,obligation_ref,"residual_requires_investigation");
        edge(obligation_ref,&review.review_item_ref,"obligation_review");
        for q in &queue.routes{edge(obligation_ref,&q.route_ref,"candidate_route");}
        let related_hypotheses:Vec<_>=hypotheses.iter().filter(|h|h.residual_ref==residual_ref)
            .map(|h|{edge(residual_ref,&h.hypothesis_ref,"candidate_hypothesis");h.hypothesis_ref.clone()}).collect();
        let route_state=if family.superseded_by.is_some(){"superseded_family"}
            else if lifecycle.terminal_update.is_some(){"terminal_acquisition_requires_reviewed_reentry"}
            else if queue.routes.is_empty(){"no_candidate_routes"}
            else if lifecycle.effective_executable_frontier_route_refs.is_empty(){"no_executable_frontier_routes"}
            else{"executable_candidates_require_governed_dispatch"};
        rows.push(json!({"residual_ref":residual_ref,"comparison_ref":comparison_ref,
            "ordinal":ordinal,"left_selector_ref":pair.left_selector_ref,"right_selector_ref":pair.right_selector_ref,
            "exact_residual":r,"obligation_ref":obligation_ref,"review":review,
            "lifecycle":lifecycle,"diagnostic_lane":lane,"suggested_existing_producer":next_owner,
            "recorded_transport_matches":transport,"hypothesis_refs":related_hypotheses,
            "route_state":route_state,"route_dispositions":dispositions,
            "formal_route_eligible":false,"candidate_only":true,"residual_paid":false}));
    }
    drop(edge);
    let deps:Vec<_>=edges.iter().map(|(a,b,_)|(a.clone(),b.clone())).collect();
    let sources:BTreeSet<_>=p.sources.iter().map(|s|s.observation.source_revision_ref.as_str()).collect();
    let cones=sources.into_iter().map(|s|potential_reopening_cone(s,&deps))
        .collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let graph_edges:Vec<_>=edges.into_iter().map(|(a,b,r)|json!({"from_ref":a,"to_ref":b,"relation":r})).collect();
    Ok(json!({"schema":"slr.relational-investigation.v1","family_ref":family.family_ref,
        "consumer_ref":p.consumer_ref,"frontier_ref":frontier.frontier_ref,
        "superseded_by":family.superseded_by,"gap_rows":rows,"gap_counts":counts,
        "dependency_edges":graph_edges,"potential_reopening_cones":cones,
        "existing_source_reopening":p.potential_reopening_by_source,
        "derived_only":true,"challengeable":true,"candidate_only":true,
        "creates_graph_edges":false,"creates_actual_reopening":false,
        "formal_consumer_equivalence_paid":false,"world_truth_paid":false,
        "creates_access_authority":false,"provider_acquisition_executed":false,
        "writes_database":false}))
}

#[cfg(test)]
mod investigation_diagnostic_tests{
    use super::*;
    use sensiblaw_pg_source_store::ResidualKind::*;
    #[test]
    fn recorded_match_routes_to_factorisation_not_formal_or_semantic_payment(){
        assert_eq!(diagnostic_lane(&UnalignedPredicate,false,true).0,"recorded_correspondence_unlicensed");
        assert_eq!(diagnostic_lane(&UnalignedPredicate,false,false).0,"correspondence_ambiguity");
        assert_eq!(diagnostic_lane(&TimeMismatch,true,false).0,"unavailable_observation");
        assert_eq!(diagnostic_lane(&TimeMismatch,false,false).0,"context_discriminator");
        assert_eq!(diagnostic_lane(&UnresolvedPolarity,false,false).0,"semantic_discriminator");
    }
}


#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeChartRequest {
    revision:String,run:String,document:String,snapshot:String,packet_digest:String,batches:Vec<String>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HeterogeneousSelectionRequest {
    indexed_receipt_path:String,consumer_ref:String,indexed_member_ref:String,other_relation_ref:String,
    article:NativeChartRequest,other:NativeChartRequest,context_axes:Vec<String>,
}
fn bounded_json<T:serde::de::DeserializeOwned>(path:&str,max:u64)->Result<T,String>{
    use std::io::Read;
    let file=std::fs::File::open(path).map_err(|e|e.to_string())?;
    let mut bytes=Vec::new();
    file.take(max+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len() as u64>max{return Err("bounded input exceeded".into());}
    serde_json::from_slice(&bytes).map_err(|e|e.to_string())
}
fn inspect_heterogeneous_selection(config:&sensiblaw_pg_source_store::DatabaseConfig,
    request:&HeterogeneousSelectionRequest)->Result<serde_json::Value,String>{
    for chart in [&request.article,&request.other]{
        if chart.batches.is_empty()||chart.batches.len()>4096||
            chart.batches.iter().any(|r|r.trim().is_empty()||r.len()>2048){
            return Err("invalid bounded native chart batches".into());
        }
    }
    let receipt:serde_json::Value=bounded_json(&request.indexed_receipt_path,64*1024*1024)?;
    let transport=|c:&NativeChartRequest|sensiblaw_pg_source_store::read_article_numeric_predicate_transport(
        config,&c.revision,&c.batches,&c.run,&c.document,&c.snapshot,&c.packet_digest).map_err(|e|e.to_string());
    let article=transport(&request.article)?;
    let other=transport(&request.other)?;
    let c=&request.other;
    let predicates=sensiblaw_pg_source_store::query_native_original_relations(
        config.database_url(),&c.run,&c.document,&c.snapshot,&c.packet_digest,"predicate").map_err(|e|e.to_string())?;
    let mut selected=sensiblaw_proof_search_loop::world_observation::select_heterogeneous_native_pair(
        &receipt,&article,&other,&predicates,&request.consumer_ref,&request.indexed_member_ref,
        &request.other_relation_ref).map_err(|e|format!("{e:?}"))?;
    use sha2::{Digest,Sha256};
    use std::collections::BTreeSet;
    let axes:BTreeSet<_>=request.context_axes.iter().map(String::as_str).collect();
    if axes.len()!=request.context_axes.len()||axes.iter().any(|a|
        !["scope","time","modality","quantifier","attribution","ontology","language"].contains(a)){
        return Err("invalid explicit context demand".into());
    }
    let selection_ref=selected["selection_ref"].clone();
    let binding=serde_json::to_vec(&(&selection_ref,&axes)).map_err(|e|e.to_string())?;
    selected["coordinate_selection_ref"]=selection_ref;
    selected["requested_context_axes"]=serde_json::json!(axes);
    selected["selection_ref"]=serde_json::json!(format!("selection:scoped:sha256:{:x}",Sha256::digest(binding)));
    Ok(selected)
}
/// Explicit operator action. A prior JSON selection cannot be substituted
/// for native readback; reopen both original charts and derive the selection
/// again. Existing selector/family persistence remains the only writer.
fn run_heterogeneous_family(config:&sensiblaw_pg_source_store::DatabaseConfig,
    request:&HeterogeneousSelectionRequest,access:&str,expected_selection:&str)
    ->Result<serde_json::Value,String>{
    use serde_json::json;
    if access.trim().is_empty(){return Err("explicit access constraint required".into());}
    let selection=inspect_heterogeneous_selection(config,request)?;
    if selection["candidate_selector_ready"]!=true||selection["selection_ref"]!=expected_selection{
        return Err("heterogeneous candidate held or changed since inspection".into());
    }
    let coordinate=|side:&str,key:&str|selection[side][0][key].as_str()
        .map(str::to_owned).ok_or_else(||"missing unique native selector coordinate".to_owned());
    let lb=coordinate("left_inverse","batch_ref")?;
    let lp=coordinate("left_inverse","predicate_candidate_ref")?;
    let rb=coordinate("right_inverse","batch_ref")?;
    let rp=coordinate("right_inverse","predicate_candidate_ref")?;
    let axes:Vec<_>=request.context_axes.iter().map(String::as_str).collect();
    let seed=seed_relational_pnf_pair_scoped(config,&request.consumer_ref,&lb,&lp,&rb,&rp,&axes)
        .map_err(|e|e.to_string())?;
    let refs= ["left_selector_ref","right_selector_ref"].iter().map(|k|
        seed[*k].as_str().map(str::to_owned).ok_or("missing persisted selector"))
        .collect::<Result<Vec<_>,_>>()?;
    let family=run_persisted_relational_family(config,&request.consumer_ref,access,&refs)
        .map_err(|e|e.to_string())?;
    // Seed and family writes are existing separate transactions. Reopen and
    // retain the exact evidence path rather than claiming atomic publication.
    let reopened=inspect_persisted_relational_family(config,&family.family_ref).map_err(|e|e.to_string())?;
    if reopened!=family{return Err("family changed on readback".into());}
    let investigation=family_investigation_projection(&reopened)?;
    Ok(json!({"schema":"slr.heterogeneous-family-journey.v1",
        "selection":selection,"selector_seed":seed,"family":reopened,"investigation":investigation,
        "candidate_only":true,"original_checker_replayed":false,"atomic_publication":false,
        "formal_consumer_equivalence_paid":false,"source_independence_paid":false,
        "world_truth_paid":false,"provider_acquisition_executed":false}))
}

pub fn run(args:&[String])->Result<(),String>{
    // Local operator-only bounded cohort manifest. No arbitrary remote
    // filesystem paths, DB writes, network acquisition or proof promotion.
    if let [mode,manifest,consumer]=args{
        if mode=="inspect-indexed-cohort"{
            let paths:Vec<String>=bounded_json(manifest,64*1024)?;
            if paths.len()<2||paths.len()>8{return Err("cohort requires 2–8 pinned indexed receipts".into());}
            let receipts=paths.iter().map(|p|bounded_json::<serde_json::Value>(p,64*1024*1024))
                .collect::<Result<Vec<_>,_>>()?;
            let out=sensiblaw_proof_search_loop::world_observation::inspect_indexed_view_cohort(
                &receipts,consumer).map_err(|e|format!("{e:?}"))?;
            return print_json(&out);
        }
    }
    // Local operator-only receipt inspection; no DB or authority access.
    if let [mode,receipt_path,consumer,required_json]=args{
        if mode=="inspect-indexed-inventory"{
            let raw=std::fs::read(receipt_path).map_err(|e|e.to_string())?;
            let receipt:serde_json::Value=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
            let required:Vec<String>=serde_json::from_str(required_json).map_err(|e|e.to_string())?;
            let out=sensiblaw_proof_search_loop::world_observation::inspect_indexed_inventory(
                &receipt,consumer,&required).map_err(|e|format!("{e:?}"))?;
            return print_json(&out);
        }
    }
    // Operator supplies an original JMD receipt; native chart is reopened
    // from persisted source owners, not supplied as arbitrary JSON.
    if let [mode,receipt_path,consumer,revision,run,doc,snapshot,digest,batches_json]=args{
        if mode=="inspect-indexed-native-meet"{
            let raw=std::fs::read(receipt_path).map_err(|e|e.to_string())?;
            let receipt:serde_json::Value=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
            if batches_json.len()>262144{return Err("batch reference request too large".into());}
            let batches:Vec<String>=serde_json::from_str(batches_json).map_err(|e|e.to_string())?;
            if batches.is_empty()||batches.len()>4096||batches.iter().any(|b|b.trim().is_empty()||b.len()>2048){
                return Err("invalid bounded batch references".into());
            }
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let native=sensiblaw_pg_source_store::read_article_numeric_predicate_transport(
                &config,revision,&batches,run,doc,snapshot,digest).map_err(|e|e.to_string())?;
            let out=sensiblaw_proof_search_loop::world_observation::inspect_indexed_native_predicate_meet(
                &receipt,&native,consumer).map_err(|e|format!("{e:?}"))?;
            return print_json(&out);
        }
    }
    // Local operator supplies indexed article receipt; reopen independent
    // native corpus predicate inventory through its existing immutable owner.
    if let [mode,receipt_path,consumer,run,doc,snapshot,digest]=args{
        if mode=="inspect-heterogeneous-overlap"{
            let raw=std::fs::read(receipt_path).map_err(|e|e.to_string())?;
            let receipt:serde_json::Value=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
            let config=load_database_config(None).map_err(|e|e.to_string())?;
            let native=sensiblaw_pg_source_store::query_native_original_relations(
                config.database_url(),run,doc,snapshot,digest,"predicate").map_err(|e|e.to_string())?;
            let out=sensiblaw_proof_search_loop::world_observation::inspect_heterogeneous_predicate_overlap(
                &receipt,&native,consumer).map_err(|e|format!("{e:?}"))?;
            return print_json(&out);
        }
    }
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    match args {
        [mode,revision,sha,start,end,literal,run] if mode=="discover-checked-predicate"=>{
            let start=start.parse::<u32>().map_err(|e|e.to_string())?;
            let end=end.parse::<u32>().map_err(|e|e.to_string())?;
            let run=if run=="--any-run"{None}else{Some(run.as_str())};
            print_json(&sensiblaw_pg_source_store::discover_checked_predicate(&config,revision,sha,start,end,literal,run)
                .map_err(|e|e.to_string())?)
        },
        [mode,path] if mode=="inspect-heterogeneous-selection"=>{
            let request:HeterogeneousSelectionRequest=bounded_json(path,4*1024*1024)?;
            print_json(&inspect_heterogeneous_selection(&config,&request)?)
        },
        [mode,path,access,expected,flag] if mode=="run-heterogeneous-family"=>{
            if flag!="--commit"{return Err("explicit --commit required".into());}
            let request:HeterogeneousSelectionRequest=bounded_json(path,4*1024*1024)?;
            print_json(&run_heterogeneous_family(&config,&request,access,expected)?)
        },
        [mode,consumer,left_batch,left_pred,right_batch,right_pred,axes_json] if mode=="seed-pnf-pair-scoped"=>{
            let axes:Vec<String>=serde_json::from_str(axes_json).map_err(|e|e.to_string())?;
            let refs:Vec<_>=axes.iter().map(String::as_str).collect();
            print_json(&seed_relational_pnf_pair_scoped(&config,consumer,left_batch,left_pred,right_batch,right_pred,&refs)
                .map_err(|e|e.to_string())?)
        },
        [mode,obligation] if mode=="inspect-mabo-formal-demands"=>
            print_json(&sensiblaw_pg_source_store::inspect_mabo_formal_demands(&config,obligation)
                .map_err(|e|e.to_string())?),
        [mode,family] if mode=="inspect-investigation"=>{
            let reopened=inspect_persisted_relational_family(&config,family).map_err(|e|e.to_string())?;
            print_json(&family_investigation_projection(&reopened)?)
        },
        [mode,family] if mode=="inspect-recorded-parity"=>
            print_json(&inspect_recorded_family_parity(&config,family).map_err(|e|e.to_string())?),
        [mode,family] if mode=="inspect-local-research"=>print_json(&inspect_local_research(&config,family,None)?),
        [mode,family,calibration_path] if mode=="plan-local-research"=>{
            let file=std::fs::read(calibration_path).map_err(|e|e.to_string())?;
            if file.len()>4*1024*1024{return Err("calibration packet too large".into());}
            let calibration:LocalCalibration=serde_json::from_slice(&file).map_err(|e|e.to_string())?;
            print_json(&inspect_local_research(&config,family,Some(calibration))?)
        },
        [mode,consumer,left_batch,left_pred,right_batch,right_pred] if mode=="seed-pnf-pair"=>
            print_json(&seed_relational_pnf_pair(&config,consumer,left_batch,left_pred,right_batch,right_pred).map_err(|e|e.to_string())?),
        [mode,revision,batches_path,run,doc,snapshot,digest,observation,consumer,query,evidence,repo,sha,path,thread] if mode=="licensed-semantic"=>{
            let raw=std::fs::read_to_string(batches_path).map_err(|e|e.to_string())?;
            let batches:Vec<String>=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
            let locator=sensiblaw_pg_source_store::RefereeReceiptLocator{
                repo:repo.clone(),sha:sha.clone(),path:path.clone(),canonical_thread_id:thread.clone(),
            };
            print_json(&sensiblaw_pg_source_store::read_authenticated_source_semantic_query(
                &config,revision,&batches,run,doc,snapshot,digest,observation,consumer,query,evidence,&locator)
                .map_err(|e|e.to_string())?)
        },
        [mode,revision,run,doc,snapshot,digest,batches_json] if mode=="inspect-source-fibre"=>{
            // Inline bounded list of persisted batch references, not a caller
            // supplied local filesystem path. Reuses the existing native owners.
            if batches_json.len()>262144{return Err("batch reference request too large".into());}
            let batches:Vec<String>=serde_json::from_str(batches_json).map_err(|e|e.to_string())?;
            if batches.is_empty()||batches.len()>4096||batches.iter().any(|b|b.trim().is_empty()||b.len()>2048){
                return Err("invalid bounded batch references".into());
            }
            print_json(&sensiblaw_pg_source_store::read_article_semantic_candidates(
                &config,revision,&batches,run,doc,snapshot,digest).map_err(|e|e.to_string())?)
        },
        [mode,revision,batches_path,run,doc,snapshot,digest] if mode=="predicate-transport"||mode=="semantic-candidates"=>{
            let raw=std::fs::read_to_string(batches_path).map_err(|e|e.to_string())?;
            let batches:Vec<String>=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
            let answer=if mode=="predicate-transport"{
                sensiblaw_pg_source_store::read_article_numeric_predicate_transport(
                    &config,revision,&batches,run,doc,snapshot,digest)
            }else{
                sensiblaw_pg_source_store::read_article_semantic_candidates(
                    &config,revision,&batches,run,doc,snapshot,digest)
            }.map_err(|e|e.to_string())?;
            print_json(&answer)
        },
        [mode,revision,batches_path,run,doc,snapshot,digest] if mode=="coordinate-meet"=>{
            let raw=std::fs::read_to_string(batches_path).map_err(|e|e.to_string())?;
            let batches:Vec<String>=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
            print_json(&sensiblaw_pg_source_store::read_article_numeric_coordinate_meet(
                &config,revision,&batches,run,doc,snapshot,digest).map_err(|e|e.to_string())?)
        },
        [mode,run,doc,snapshot,digest,mentions_path] if mode=="numeric-action-roles"=>{
            let raw=std::fs::read_to_string(mentions_path).map_err(|e|e.to_string())?;
            let mentions:Vec<i64>=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
            print_json(&sensiblaw_pg_source_store::query_native_recorded_action_roles(
                config.database_url(),run,doc,snapshot,digest,&mentions).map_err(|e|e.to_string())?)
        },
        [mode,revision,batches_path,mentions_path] if mode=="article-actions"=>{
            let read = |path:&str| -> Result<Vec<String>,String> {
                let raw=std::fs::read_to_string(path).map_err(|e|e.to_string())?;
                serde_json::from_str(&raw).map_err(|e|e.to_string())
            };
            let batches=read(batches_path)?;
            let mentions=read(mentions_path)?;
            print_json(&sensiblaw_pg_source_store::read_recorded_article_actions(
                &config,revision,&batches,&mentions).map_err(|e|e.to_string())?)
        },
        [mode,corpus,access] if mode=="acceptance-corpus"=>{
            let suite=load_rel_acceptance_corpus(&config,corpus).map_err(|e|e.to_string())?;
            let mut receipts=Vec::new();
            for case in suite.cases{
                let left=case.left.selector_ref;
                let right=case.right.selector_ref;
                let family=run_persisted_relational_family(&config,&case.consumer.consumer_ref,access,&[left.clone(),right.clone()])
                    .map_err(|e|format!("{}: {e}",case.case_ref))?;
                let reopened=inspect_persisted_relational_family(&config,&family.family_ref).map_err(|e|e.to_string())?;
                if reopened!=family{return Err(format!("{}: exact family reopen mismatch",case.case_ref));}
                let actual=family.projection.directed_comparisons.iter().find(|p|p.left_selector_ref==left&&p.right_selector_ref==right)
                    .ok_or("original directed pair missing")?;
                // Old REL-1C expected labels are historical controls; strict
                // native projection may expose additional real residuals.
                let required=case.required_residual_kinds.iter().all(|k|actual.comparison.residuals.iter().any(|r|&r.kind==k));
                let forbidden=case.forbidden_residual_kinds.iter().any(|k|actual.comparison.residuals.iter().any(|r|&r.kind==k));
                receipts.push(serde_json::json!({"case_ref":case.case_ref,"purpose":case.purpose_ref,
                    "legacy_expected_finding":case.expected_finding,
                    "strict_actual_finding":actual.comparison.finding,
                    "legacy_required_residuals_present":required,"legacy_forbidden_residual_present":forbidden,
                    "family":family,"exact_native_family_reopen":true}));
            }
            if receipts.is_empty(){return Err("empty persisted acceptance corpus".into());}
            print_json(&serde_json::json!({"schema":"itir.native-relational-corpus.v1","corpus_ref":corpus,
                "cases":receipts,"semantic_coverage_paid":false}))
        },
        [mode,consumer,access,views_path,axes_json,flag] if mode=="run-pnf-cohort"=>{
            if flag!="--commit"||access.trim().is_empty(){return Err("explicit --commit and access constraint required".into());}
            let views:Vec<sensiblaw_pg_source_store::NativePnfViewCoordinate>=bounded_json(views_path,262144)?;
            let axes:Vec<String>=serde_json::from_str(axes_json).map_err(|e|e.to_string())?;
            let axes:Vec<_>=axes.iter().map(String::as_str).collect();
            let seed=sensiblaw_pg_source_store::seed_relational_pnf_cohort(&config,consumer,&views,&axes)
                .map_err(|e|e.to_string())?;
            let refs:Vec<String>=serde_json::from_value(seed["selector_refs"].clone()).map_err(|e|e.to_string())?;
            let family=run_persisted_relational_family(&config,consumer,access,&refs).map_err(|e|e.to_string())?;
            let reopened=inspect_persisted_relational_family(&config,&family.family_ref).map_err(|e|e.to_string())?;
            if reopened!=family{return Err("cohort family changed on readback".into());}
            let investigation=family_investigation_projection(&reopened)?;
            print_json(&serde_json::json!({"schema":"slr.native-pnf-cohort-journey.v1",
                "selector_seed":seed,"family":reopened,"investigation":investigation,
                "candidate_only":true,"atomic_publication":false,
                "formal_consumer_equivalence_paid":false,"source_independence_paid":false,
                "world_truth_paid":false,"provider_acquisition_executed":false}))
        },
        [mode,family,query,requests_json] if mode=="inspect-licensed-view-cohort"=>{
            // Inline exact native selectors and existing referee locators;
            // never accept proof answers or arbitrary local input files.
            if requests_json.len()>65536{return Err("cohort evidence request too large".into());}
            let requests:Vec<sensiblaw_pg_source_store::ViewPairEvidence>=
                serde_json::from_str(requests_json).map_err(|e|e.to_string())?;
            print_json(&sensiblaw_pg_source_store::read_authenticated_view_cohort(
                &config,family,query,&requests).map_err(|e|e.to_string())?)
        },
        [mode,family,left,right,query,evidence,repo,sha,path,thread] if mode=="licensed-reciprocal"=>{
            let locator=sensiblaw_pg_source_store::RefereeReceiptLocator{
                repo:repo.clone(),sha:sha.clone(),path:path.clone(),canonical_thread_id:thread.clone(),
            };
            print_json(&sensiblaw_pg_source_store::read_authenticated_reciprocal_query(
                &config,family,left,right,query,evidence,&locator).map_err(|e|e.to_string())?)
        },
        [mode,family,left,right] if mode=="reciprocal"=>{
            let reopened=inspect_persisted_relational_family(&config,family).map_err(|e|e.to_string())?;
            print_json(&sensiblaw_pg_source_store::recorded_reciprocal_query(&reopened,left,right).map_err(|e|e.to_string())?)
        },
        [mode,family,filler] if mode=="participant"=>{
            if filler.trim().is_empty(){return Err("empty exact participant factor".into());}
            let reopened=inspect_persisted_relational_family(&config,family).map_err(|e|e.to_string())?;
            print_json(&sensiblaw_pg_source_store::recorded_participant_query(&reopened,filler))
        },
        [mode,family] if mode=="inspect-research-frontier"=>{
            let reopened=inspect_persisted_relational_family(&config,family).map_err(|e|e.to_string())?;
            print_json(&family_research_frontier(&reopened)?)
        },
        [mode,family] if mode=="inspect"=>print_json(&inspect_persisted_relational_family(&config,family).map_err(|e|e.to_string())?),
        [mode,family,old,new] if mode=="prepare-revision"=>print_json(&prepare_relational_family_revision(&config,family,old,new).map_err(|e|e.to_string())?),
        [mode,transition] if mode=="inspect-revision"=>print_json(&inspect_relational_family_revision(&config,transition).map_err(|e|e.to_string())?),
        [mode,transition] if mode=="publish-revision"=>print_json(&publish_relational_family_revision(&config,transition).map_err(|e|e.to_string())?),
        [mode,transition,command,reviewer,action] if mode=="review-revision"=>{
            let t=inspect_relational_family_revision(&config,transition).map_err(|e|e.to_string())?;
            let action=match action.as_str(){"accept"=>ReviewAction::Accept,"reject"=>ReviewAction::Reject,
                "abstain"=>ReviewAction::Abstain,_=>return Err("invalid candidate review action".into())};
            let (receipt,item)=apply_persisted_review_command(&config,&ReviewCommand{
                command_ref:command.clone(),review_item_ref:t.review_item_ref,
                action,reviewer_ref:reviewer.clone(),qualification_ref:None,evidence_request_ref:None,
            }).map_err(|e|e.to_string())?;
            print_json(&serde_json::json!({"command_ref":receipt.command_ref,"review_item_ref":item.review_item_ref,
                "candidate_review_status":format!("{:?}",item.current_status),"creates_semantic_authority":false}))
        },
        _ if (4..=12).contains(&args.len())=>print_json(&run_persisted_relational_family(&config,&args[0],&args[1],&args[2..]).map_err(|e|e.to_string())?),
        _=>Err("usage: relational-family inspect-indexed-cohort MANIFEST_JSON CONSUMER | inspect-heterogeneous-selection REQUEST_JSON | run-heterogeneous-family REQUEST_JSON ACCESS EXPECTED_SELECTION --commit | inspect-heterogeneous-overlap RECEIPT CONSUMER RUN DOC SNAPSHOT DIGEST | inspect-indexed-native-meet RECEIPT CONSUMER REVISION RUN DOC SNAPSHOT DIGEST BATCH_REFS_JSON | inspect-indexed-inventory RECEIPT CONSUMER REQUIRED_LANES_JSON | CONSUMER ACCESS SELECTOR SELECTOR ... | licensed-semantic REVISION BATCH_REFS_JSON RUN DOC SNAPSHOT DIGEST OBSERVATION CONSUMER QUERY EVIDENCE REPO SHA PATH THREAD | predicate-transport REVISION BATCH_REFS_JSON RUN DOC SNAPSHOT DIGEST | semantic-candidates REVISION BATCH_REFS_JSON RUN DOC SNAPSHOT DIGEST | coordinate-meet REVISION BATCH_REFS_JSON RUN DOC SNAPSHOT DIGEST | numeric-action-roles RUN DOC SNAPSHOT DIGEST MENTION_TOKEN_IDS_JSON | article-actions REVISION BATCH_REFS_JSON MENTION_REFS_JSON | run-pnf-cohort CONSUMER ACCESS VIEWS_JSON AXES_JSON --commit | inspect-licensed-view-cohort FAMILY QUERY EVIDENCE_JSON | licensed-reciprocal FAMILY LEFT RIGHT QUERY EVIDENCE REPO SHA PATH THREAD | reciprocal FAMILY LEFT RIGHT | participant FAMILY FILLER_REF | seed-pnf-pair-scoped CONSUMER LEFT_BATCH LEFT_PRED RIGHT_BATCH RIGHT_PRED AXES_JSON | inspect-mabo-formal-demands OBLIGATION | inspect-investigation FAMILY | inspect-recorded-parity FAMILY | inspect-local-research FAMILY | plan-local-research FAMILY CALIBRATION_JSON_PATH | inspect-research-frontier FAMILY | inspect FAMILY | prepare-revision FAMILY OLD_SELECTOR NEW_SELECTOR | inspect-revision TRANSITION | review-revision TRANSITION COMMAND REVIEWER accept|reject|abstain | publish-revision TRANSITION".into())
    }
}
