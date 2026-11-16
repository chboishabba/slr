//! Read-only native SLR S28 event chronology/contestation handoff to JMD Lean.
//! Reuses the existing PostgreSQL chronology and reviewed trace owners.
//! Query by *existing* event identity; never mine a new event or infer one
//! from person/date/label overlap. A review reference is not world truth.
use sensiblaw_core::chronology_contestation::TemporalForm;
use sensiblaw_pg_source_store::{
    load_claims_for_event, load_contestation_relations_for_claims,
    load_database_config, load_temporal_assertions_for_event,
    load_source_statement, load_observation_event_links_for_event,
    load_statement_observation_links_for_observation,
};
use std::collections::BTreeSet;

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
    for observation in &assertions {
        observation.validate().map_err(|e| format!("S28 temporal row invalid: {e:?}"))?;
    }
    for claim in &claims {
        claim.validate().map_err(|e| format!("S28 claim row invalid: {e:?}"))?;
    }
    let claim_refs: Vec<_> = claims.iter().map(|c| c.claim_ref.clone()).collect();
    let contestations = load_contestation_relations_for_claims(&config,&claim_refs)
        .map_err(|e| e.to_string())?;
    for relation in &contestations {
        relation.validate().map_err(|e| format!("S28 relation row invalid: {e:?}"))?;
    }
    // Reopen canonical M12 source identities for every statement referenced
    // by the S28 overlay. These are provenance joins, never occurrence proofs.
    let mut source_refs = BTreeSet::<String>::new();
    for assertion in &assertions {
        source_refs.extend(assertion.statement_refs.iter().cloned());
    }
    for claim in &claims {
        source_refs.extend(claim.statement_refs.iter().cloned());
    }
    for relation in &contestations {
        source_refs.extend(relation.statement_refs.iter().cloned());
    }
    let mut source_rows = Vec::new();
    for statement_ref in source_refs {
        let statement = load_source_statement(&config, &statement_ref)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("missing original M12 source statement: {statement_ref}"))?;
        if !statement.candidate_only || statement.creates_semantic_authority
            || statement.applicability_promoted || statement.claim_truth_promoted
        {
            return Err("M12 source row attempts authority promotion".into());
        }
        let digest = statement.statement_sha256.iter()
            .map(|b| format!("{b:02x}")).collect::<String>();
        source_rows.push(serde_json::json!({
            "statement_ref": statement.statement_ref,
            "document_ref": statement.document_ref,
            "source_revision_ref": statement.source_revision_ref,
            "exact_span_ref": statement.exact_span_ref,
            "statement_sha256": digest,
            "candidate_only": statement.candidate_only,
            "creates_semantic_authority": statement.creates_semantic_authority,
            "applicability_promoted": statement.applicability_promoted,
            "claim_truth_promoted": statement.claim_truth_promoted,
        }));
    }
    let event_links = load_observation_event_links_for_event(&config, event_ref)
        .map_err(|e| e.to_string())?;
    for link in &event_links {
        link.validate().map_err(|e| format!("invalid M12 event assembly link: {e:?}"))?;
        if link.event_ref != *event_ref {
            return Err("M12 observation/event join changed requested event identity".into());
        }
    }
    let assembly_rows: Vec<_> = event_links.iter().map(|link| serde_json::json!({
        "link_ref": link.link_ref,
        "observation_ref": link.observation_ref,
        "event_ref": link.event_ref,
        "assembly_receipt_ref": link.assembly_receipt_ref,
        "candidate_only": link.candidate_only,
        "creates_semantic_authority": link.creates_semantic_authority,
        "applicability_promoted": link.applicability_promoted,
        "claim_truth_promoted": link.claim_truth_promoted,
    })).collect();
    // Preserve the *original* M12 statement -> observation relationship.
    // A PNF label, shared QID, or event-ref alone never fabricates this join.
    let mut observation_refs = BTreeSet::<String>::new();
    for assertion in &assertions {
        observation_refs.extend(assertion.observation_refs.iter().cloned());
    }
    for claim in &claims {
        observation_refs.extend(claim.observation_refs.iter().cloned());
    }
    for relation in &contestations {
        observation_refs.extend(relation.observation_refs.iter().cloned());
    }
    let linked_observations: BTreeSet<_> = event_links.iter()
        .map(|link| link.observation_ref.as_str()).collect();
    let obs_to_event_join_complete = observation_refs.iter()
        .all(|reference| linked_observations.contains(reference.as_str()));
    let mut statement_observation_rows = Vec::new();
    let mut observations_with_source = BTreeSet::<String>::new();
    for observation_ref in observation_refs {
        let relations = load_statement_observation_links_for_observation(
            &config, &observation_ref
        ).map_err(|e| e.to_string())?;
        for link in relations {
            link.validate().map_err(|e| format!("invalid M12 statement/observation link: {e:?}"))?;
            // A source row must be reopened independently, even if it was
            // absent from the S28 statementRefs lists.
            let source = load_source_statement(&config, &link.statement_ref)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("missing M12 source for observation {}", link.observation_ref))?;
            if source.statement_ref != link.statement_ref {
                return Err("M12 source/observation identity mismatch".into());
            }
            observations_with_source.insert(observation_ref.clone());
            statement_observation_rows.push(serde_json::json!({
                "link_ref":link.link_ref,
                "statement_ref":link.statement_ref,
                "source_revision_ref":source.source_revision_ref,
                "exact_span_ref":source.exact_span_ref,
                "observation_ref":link.observation_ref,
                "candidate_pnf_ref":link.candidate_pnf_ref,
                "parser_receipt_ref":link.parser_receipt_ref,
                "parse_review_ref":link.parse_review_ref,
                "admission_receipt_ref":link.admission_receipt_ref,
                "disposition":format!("{:?}",link.disposition),
                "candidate_only":link.candidate_only,
                "creates_semantic_authority":link.creates_semantic_authority,
                "applicability_promoted":link.applicability_promoted,
                "claim_truth_promoted":link.claim_truth_promoted,
            }));
        }
    }
    let source_trace_join_complete =
        !event_links.is_empty() && !statement_observation_rows.is_empty()
        && obs_to_event_join_complete
        && event_links.iter().all(|link|
            observations_with_source.contains(&link.observation_ref));
    let times: Vec<_> = assertions.iter().map(|a| serde_json::json!({
        "temporal_ref":a.temporal_ref,
        "form":temporal_form(&a.form),
        "statement_refs":a.statement_refs,
        "observation_refs":a.observation_refs,
        "review_ref":a.review_ref,
        "candidate_only":a.candidate_only,
        "creates_semantic_authority":a.creates_semantic_authority,
        "applicability_promoted":a.applicability_promoted,
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
        "creates_semantic_authority":c.creates_semantic_authority,
        "applicability_promoted":c.applicability_promoted,
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
        "creates_semantic_authority":r.creates_semantic_authority,
        "applicability_promoted":r.applicability_promoted,
        "claim_truth_promoted":r.claim_truth_promoted,
    })).collect();
    println!("{}",serde_json::json!({
        "schema":"slr-s28-event-chronology-candidates-v1",
        "event_ref":event_ref,
        "temporal_assertions":times,
        "claim_leaves":leaves,
        "contestation_relations":relations,
        "source_statements":source_rows,
        "observation_event_links":assembly_rows,
        "statement_observation_links":statement_observation_rows,
        "source_trace_join_complete":source_trace_join_complete,
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
