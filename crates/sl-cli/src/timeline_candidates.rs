//! Read-only native SLR S28 event chronology/contestation handoff to JMD Lean.
//! Reuses the existing PostgreSQL chronology and reviewed trace owners.
//! Query by *existing* event identity; never mine a new event or infer one
//! from person/date/label overlap. A review reference is not world truth.
use sensiblaw_core::chronology_contestation::TemporalForm;
use sensiblaw_pg_source_store::{
    load_claims_for_event, load_contestation_relations_for_claims,
    load_database_config, load_temporal_assertions_for_event,
};

fn temporal_form(value: &TemporalForm) -> serde_json::Value {
    match value {
        TemporalForm::ExactInstant { instant_ref } =>
            serde_json::json!({"kind":"exact-instant","instant_ref":instant_ref}),
        TemporalForm::ExactDate { date_ref } =>
            serde_json::json!({"kind":"exact-date","date_ref":date_ref}),
        TemporalForm::Interval { start_ref,end_ref } =>
            serde_json::json!({"kind":"interval","start_ref":start_ref,"end_ref":end_ref}),
        TemporalForm::Approximate { label } =>
            serde_json::json!({"kind":"approximate","label":label}),
        TemporalForm::RelativeBefore { event_ref } =>
            serde_json::json!({"kind":"relative-before","related_event_ref":event_ref}),
        TemporalForm::RelativeAfter { event_ref } =>
            serde_json::json!({"kind":"relative-after","related_event_ref":event_ref}),
        TemporalForm::Contemporaneous { event_ref } =>
            serde_json::json!({"kind":"contemporaneous","related_event_ref":event_ref}),
        TemporalForm::Undated => serde_json::json!({"kind":"undated"}),
        TemporalForm::Unknown => serde_json::json!({"kind":"unknown"}),
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    let [event_ref] = args else {
        return Err("usage: sensiblaw timeline-candidates EVENT_REF".into());
    };
    if event_ref.trim().is_empty() {
        return Err("empty event ref".into());
    }
    let config = load_database_config(None).map_err(|e| e.to_string())?;
    let assertions = load_temporal_assertions_for_event(&config,event_ref)
        .map_err(|e| e.to_string())?;
    let claims = load_claims_for_event(&config,event_ref)
        .map_err(|e| e.to_string())?;
    if assertions.is_empty() && claims.is_empty() {
        return Err("no persisted S28 claim or temporal observation for event".into());
    }
    let claim_refs: Vec<_> = claims.iter().map(|c| c.claim_ref.clone()).collect();
    let contestations = load_contestation_relations_for_claims(&config,&claim_refs)
        .map_err(|e| e.to_string())?;
    let times: Vec<_> = assertions.iter().map(|a| serde_json::json!({
        "temporal_ref":a.temporal_ref,
        "form":temporal_form(&a.form),
        "statement_refs":a.statement_refs,
        "observation_refs":a.observation_refs,
        "review_ref":a.review_ref,
        "candidate_only":a.candidate_only,
        "creates_semantic_authority":a.creates_semantic_authority,
        "claim_truth_promoted":a.claim_truth_promoted,
    })).collect();
    let leaves: Vec<_> = claims.iter().map(|c| serde_json::json!({
        "claim_ref":c.claim_ref,
        "proposition_ref":c.proposition_ref,
        "kind":format!("{:?}",c.kind),
        "review_state":format!("{:?}",c.review_state),
        "review_ref":c.review_ref,
        "speaker_ref":c.speaker_ref,
        "statement_refs":c.statement_refs,
        "observation_refs":c.observation_refs,
        "temporal_refs":c.temporal_refs,
        "scope_refs":c.scope_refs,
        "candidate_only":c.candidate_only,
        "claim_truth_promoted":c.claim_truth_promoted,
    })).collect();
    let relations: Vec<_> = contestations.iter().map(|r| serde_json::json!({
        "relation_ref":r.relation_ref,
        "from_claim_ref":r.from_claim_ref,
        "to_claim_ref":r.to_claim_ref,
        "kind":format!("{:?}",r.kind),
        "review_ref":r.review_ref,
        "statement_refs":r.statement_refs,
        "observation_refs":r.observation_refs,
        "candidate_only":r.candidate_only,
        "claim_truth_promoted":r.claim_truth_promoted,
    })).collect();
    println!("{}",serde_json::json!({
        "schema":"slr-s28-event-chronology-candidates-v1",
        "event_ref":event_ref,
        "temporal_assertions":times,
        "claim_leaves":leaves,
        "contestation_relations":relations,
        "candidate_only":true,
        "source_or_review_receipts_are_world_truth":false,
        "event_occurrence_paid":false,
        "semantic_promotion":false,
    }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{run,temporal_form};
    use sensiblaw_core::chronology_contestation::TemporalForm;

    #[test]
    fn event_identity_is_required() {
        assert!(run(&[]).is_err());
        assert!(run(&["".into()]).is_err());
    }

    #[test]
    fn relative_order_is_not_absolute_time() {
        let value=temporal_form(&TemporalForm::RelativeBefore{
            event_ref:"event:meeting".into()
        });
        assert_eq!(value["kind"],"relative-before");
        assert_eq!(value["related_event_ref"],"event:meeting");
        assert!(value.get("instant_ref").is_none());
    }
}
