use std::{env, fs};

use sensiblaw_core::{
    chat_source::{
        ArchivedChatMessage, ChatBranchMembership, ChatContentKind, ChatMessageRole,
        ChatStatementCandidateSpan,
    },
    chronology_contestation::{
        ClaimLeaf, ClaimLeafKind, ClaimReviewState, ContestationRelation,
        ContestationRelationKind, PropositionRoot, TemporalAssertion, TemporalForm,
    },
    event_discovery::{
        CandidateEventJoinProposal, EventJoinSignal, EventJoinSignalKind,
    },
    operational_state::{
        OperationalEvent, OperationalEventKind, OperationalOutstandingKind,
        OperationalOutstandingState, OperationalSemanticLink,
        OperationalSemanticRelationKind, OperationalTargetKind,
    },
};
use sensiblaw_pg_source_store::{
    canonical_observation_event_link_ref, canonical_statement_observation_link_ref,
    install_m13_empirical_schema, load_database_config, materialize_chat_statement,
    persist_chat_archive_message, persist_claim_leaf, persist_contestation_relation,
    persist_event_claim_link, persist_event_join_proposal_with_review,
    persist_event_temporal_link, persist_observation_event_link,
    persist_operational_event, persist_operational_outstanding_state,
    persist_operational_semantic_link, persist_proposition_root,
    persist_statement_observation_link, persist_temporal_assertion, EventClaimLink,
    EventTemporalLink, ObservationEventLink, StatementObservationDisposition,
    StatementObservationLink, StatementOrigin,
};
use serde_json::json;

const MATTER_REF: &str = "matter:m13:operator-capstone";
const CONVERSATION_REF: &str = "conversation:m13:operator-capstone";
const EVENT_REVIEWED: &str = "event:m13:operator:reviewed";
const EVENT_UNDATED: &str = "event:m13:operator:undated";
const PROPOSAL_REF: &str = "proposal:m13:operator:auto";
const REVIEW_ITEM_REF: &str = "review-item:proposal:m13:operator:auto";
const OP_EVENT_REF: &str = "operational:statibaker:2026-09-24:m13-capstone";
const OUTSTANDING_REF: &str = "outstanding:m13:operator:followup";
const SEALED_REF: &str = "sealed:m13:operator:private-coordinate";

fn message(
    message_ref: &str,
    node_ref: &str,
    parent_node_ref: Option<&str>,
    time: &str,
    content: &str,
) -> ArchivedChatMessage {
    ArchivedChatMessage {
        conversation_ref: CONVERSATION_REF.into(),
        message_ref: message_ref.into(),
        node_ref: node_ref.into(),
        parent_node_ref: parent_node_ref.map(str::to_owned),
        branch_membership: ChatBranchMembership::Active,
        message_time_ref: time.into(),
        role: ChatMessageRole::User,
        thread_title: "M13 operator empirical capstone".into(),
        content: content.into(),
        content_kind: ChatContentKind::Message,
        citation_token: None,
        filename: None,
        asset_pointer: None,
        source_scope: Some("m13-empirical-fixture".into()),
        body_storage_ref: Some(message_ref.into()),
        mime_type: Some("text/plain".into()),
        archive_source_id: Some("m13-operator-fixture-v1".into()),
        provenance_ref: Some("fixture:m13:operator:v1".into()),
        identity_verified: true,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn statement(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
    message: &ArchivedChatMessage,
    candidate_ref: &str,
) -> Result<sensiblaw_pg_source_store::PersistedSourceStatement, String> {
    persist_chat_archive_message(config, message).map_err(|error| error.to_string())?;
    materialize_chat_statement(
        config,
        &ChatStatementCandidateSpan {
            statement_candidate_ref: candidate_ref.into(),
            message_ref: message.message_ref.clone(),
            start_char: 0,
            end_char: message.content.len() as u32,
            literal_text: message.content.clone(),
            splitter_receipt_ref: format!("splitter:{candidate_ref}"),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
        StatementOrigin::InitialIntake,
    )
    .map_err(|error| error.to_string())
}

fn observation_link(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
    statement_ref: &str,
    observation_ref: &str,
    ordinal: &str,
) -> Result<(), String> {
    let candidate_pnf_ref = format!("candidate-pnf:m13:operator:{ordinal}");
    persist_statement_observation_link(
        config,
        &StatementObservationLink {
            link_ref: canonical_statement_observation_link_ref(
                statement_ref,
                &candidate_pnf_ref,
                observation_ref,
            ),
            statement_ref: statement_ref.into(),
            candidate_pnf_ref,
            observation_ref: observation_ref.into(),
            parser_receipt_ref: Some(format!("parser-receipt:m13:operator:{ordinal}")),
            parse_review_ref: Some(format!("parse-review:m13:operator:{ordinal}")),
            admission_receipt_ref: None,
            disposition: StatementObservationDisposition::ParseReviewed,
            qualification_ref: None,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn event_link(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
    observation_ref: &str,
    event_ref: &str,
    receipt_ref: &str,
) -> Result<(), String> {
    persist_observation_event_link(
        config,
        &ObservationEventLink {
            link_ref: canonical_observation_event_link_ref(observation_ref, event_ref),
            observation_ref: observation_ref.into(),
            event_ref: event_ref.into(),
            assembly_receipt_ref: receipt_ref.into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn main() -> Result<(), String> {
    let output = env::args()
        .nth(1)
        .unwrap_or_else(|| "m13-operator-matter-scope.generated.json".into());

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    install_m13_empirical_schema(&config)?;

    let a_msg = message(
        "message:m13:operator:a",
        "node:m13:operator:a",
        None,
        "2026-09-24T10:00:00+10:00",
        "Operator account: the appointment occurred on 24 September 2026.",
    );
    let b_msg = message(
        "message:m13:operator:b",
        "node:m13:operator:b",
        Some("node:m13:operator:a"),
        "2026-09-24T10:05:00+10:00",
        "Second account: the appointment did not occur.",
    );
    let c_msg = message(
        "message:m13:operator:c",
        "node:m13:operator:c",
        Some("node:m13:operator:b"),
        "2026-09-24T10:10:00+10:00",
        "Undated account: a follow-up call occurred.",
    );
    let no_event_msg = message(
        "message:m13:operator:no-event",
        "node:m13:operator:no-event",
        Some("node:m13:operator:c"),
        "2026-09-24T10:15:00+10:00",
        "Context note: this material is relevant but is not assembled into an event.",
    );

    let a_source =
        persist_chat_archive_message(&config, &a_msg).map_err(|e| e.to_string())?;
    let b_source =
        persist_chat_archive_message(&config, &b_msg).map_err(|e| e.to_string())?;

    let a = statement(&config, &a_msg, "chat-statement:m13:operator:a")?;
    let b = statement(&config, &b_msg, "chat-statement:m13:operator:b")?;
    let c = statement(&config, &c_msg, "chat-statement:m13:operator:c")?;
    let no_event = statement(
        &config,
        &no_event_msg,
        "chat-statement:m13:operator:no-event",
    )?;

    let obs_a = "observation:m13:operator:a";
    let obs_b = "observation:m13:operator:b";
    let obs_c = "observation:m13:operator:c";
    observation_link(&config, &a.statement_ref, obs_a, "a")?;
    observation_link(&config, &b.statement_ref, obs_b, "b")?;
    observation_link(&config, &c.statement_ref, obs_c, "c")?;

    event_link(
        &config,
        obs_a,
        EVENT_REVIEWED,
        "assembly-receipt:m13:operator:reviewed",
    )?;
    event_link(
        &config,
        obs_b,
        EVENT_REVIEWED,
        "assembly-receipt:m13:operator:reviewed",
    )?;
    event_link(
        &config,
        obs_c,
        EVENT_UNDATED,
        "assembly-receipt:m13:operator:undated",
    )?;

    let exact_temporal = TemporalAssertion {
        temporal_ref: "temporal:m13:operator:exact".into(),
        form: TemporalForm::ExactDate {
            date_ref: "2026-09-24".into(),
        },
        statement_refs: vec![a.statement_ref.clone(), b.statement_ref.clone()],
        observation_refs: vec![obs_a.into(), obs_b.into()],
        review_ref: Some("review:m13:operator:temporal-exact".into()),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    persist_temporal_assertion(&config, &exact_temporal)
        .map_err(|error| error.to_string())?;
    persist_event_temporal_link(
        &config,
        &EventTemporalLink {
            event_ref: EVENT_REVIEWED.into(),
            temporal_ref: exact_temporal.temporal_ref.clone(),
            assembly_receipt_ref: "assembly-receipt:m13:operator:reviewed".into(),
        },
    )
    .map_err(|error| error.to_string())?;

    let undated_temporal = TemporalAssertion {
        temporal_ref: "temporal:m13:operator:undated".into(),
        form: TemporalForm::Undated,
        statement_refs: vec![c.statement_ref.clone()],
        observation_refs: vec![obs_c.into()],
        review_ref: Some("review:m13:operator:temporal-undated".into()),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    persist_temporal_assertion(&config, &undated_temporal)
        .map_err(|error| error.to_string())?;
    persist_event_temporal_link(
        &config,
        &EventTemporalLink {
            event_ref: EVENT_UNDATED.into(),
            temporal_ref: undated_temporal.temporal_ref.clone(),
            assembly_receipt_ref: "assembly-receipt:m13:operator:undated".into(),
        },
    )
    .map_err(|error| error.to_string())?;

    let proposition_ref = "proposition:m13:operator:appointment";
    persist_proposition_root(
        &config,
        &PropositionRoot {
            proposition_ref: proposition_ref.into(),
            label: "The appointment occurred".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
    )
    .map_err(|error| error.to_string())?;

    let claim_a = ClaimLeaf {
        claim_ref: "claim:m13:operator:a".into(),
        proposition_ref: proposition_ref.into(),
        kind: ClaimLeafKind::Affirmation,
        speaker_ref: Some("actor:m13:operator:a".into()),
        statement_refs: vec![a.statement_ref.clone()],
        observation_refs: vec![obs_a.into()],
        temporal_refs: vec![exact_temporal.temporal_ref.clone()],
        scope_refs: vec![MATTER_REF.into()],
        review_state: ClaimReviewState::Accepted,
        review_ref: Some("review:m13:operator:claim-a".into()),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let claim_b = ClaimLeaf {
        claim_ref: "claim:m13:operator:b".into(),
        proposition_ref: proposition_ref.into(),
        kind: ClaimLeafKind::Denial,
        speaker_ref: None,
        statement_refs: vec![b.statement_ref.clone()],
        observation_refs: vec![obs_b.into()],
        temporal_refs: vec![exact_temporal.temporal_ref.clone()],
        scope_refs: vec![MATTER_REF.into()],
        review_state: ClaimReviewState::Accepted,
        review_ref: Some("review:m13:operator:claim-b".into()),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    persist_claim_leaf(&config, &claim_a).map_err(|error| error.to_string())?;
    persist_claim_leaf(&config, &claim_b).map_err(|error| error.to_string())?;
    for claim in [&claim_a, &claim_b] {
        persist_event_claim_link(
            &config,
            &EventClaimLink {
                event_ref: EVENT_REVIEWED.into(),
                claim_ref: claim.claim_ref.clone(),
                assembly_receipt_ref: "assembly-receipt:m13:operator:reviewed".into(),
            },
        )
        .map_err(|error| error.to_string())?;
    }

    persist_contestation_relation(
        &config,
        &ContestationRelation {
            relation_ref: "relation:m13:operator:contradiction".into(),
            from_claim_ref: claim_a.claim_ref.clone(),
            to_claim_ref: claim_b.claim_ref.clone(),
            kind: ContestationRelationKind::Contradicts,
            statement_refs: vec![a.statement_ref.clone(), b.statement_ref.clone()],
            observation_refs: vec![obs_a.into(), obs_b.into()],
            review_ref: Some("review:m13:operator:contradiction".into()),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
    )
    .map_err(|error| error.to_string())?;

    persist_event_join_proposal_with_review(
        &config,
        &CandidateEventJoinProposal {
            proposal_ref: PROPOSAL_REF.into(),
            observation_refs: vec![obs_a.into(), obs_b.into()],
            statement_refs: vec![a.statement_ref.clone(), b.statement_ref.clone()],
            source_family_refs: vec!["chat".into(), "operator-note".into()],
            signals: vec![
                EventJoinSignal {
                    kind: EventJoinSignalKind::SharedTemporalBucket,
                    evidence_ref: "signal:m13:operator:date".into(),
                    detector_ref: "fixture:m13:operator".into(),
                },
                EventJoinSignal {
                    kind: EventJoinSignalKind::UserDeclaredSameIncident,
                    evidence_ref: "signal:m13:operator:same-incident".into(),
                    detector_ref: "fixture:m13:operator".into(),
                },
            ],
            policy_ref: "event-discovery-policy:v1".into(),
            candidate_only: true,
            requires_review: true,
            creates_event_identity: false,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
        vec![MATTER_REF.into()],
    )
    .map_err(|error| error.to_string())?;

    persist_operational_event(
        &config,
        &OperationalEvent {
            operational_event_ref: OP_EVENT_REF.into(),
            producer_event_ref: "m13-capstone".into(),
            producer_ref: "statibaker:m13-fixture".into(),
            state_date: "2026-09-24".into(),
            start_time_ref: "2026-09-24T14:01:00+10:00".into(),
            end_time_ref: "2026-09-24T14:07:00+10:00".into(),
            primary_app_ref: Some("app:sensiblaw".into()),
            label: "M13 fixture review activity".into(),
            provenance_refs: vec!["fixture:m13:operator:v1".into()],
            producer_observed: true,
            creates_semantic_authority: false,
            pays_evidence: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            kind: OperationalEventKind::Session,
        },
    )
    .map_err(|error| error.to_string())?;
    persist_operational_semantic_link(
        &config,
        &OperationalSemanticLink {
            link_ref: "operational-link:m13:operator:event".into(),
            operational_event_ref: OP_EVENT_REF.into(),
            target_ref: EVENT_REVIEWED.into(),
            target_kind: OperationalTargetKind::SemanticEvent,
            relation_kind: OperationalSemanticRelationKind::Reviewed,
            relationship_receipt_ref: "review:m13:operator:operational-link".into(),
            reviewed_link: true,
            creates_semantic_identity: false,
            creates_semantic_authority: false,
            pays_evidence: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
    )
    .map_err(|error| error.to_string())?;
    persist_operational_outstanding_state(
        &config,
        &OperationalOutstandingState {
            operational_state_ref: OUTSTANDING_REF.into(),
            state_date: "2026-09-24".into(),
            subject_ref: EVENT_REVIEWED.into(),
            label: "M13 fixture follow-up remains open".into(),
            provenance_refs: vec!["fixture:m13:operator:v1".into()],
            kind: OperationalOutstandingKind::Carryover,
            producer_observed: true,
            creates_review_pending: false,
            creates_semantic_unresolved: false,
            creates_user_priority: false,
        },
    )
    .map_err(|error| error.to_string())?;

    let visibility = vec![
        EVENT_REVIEWED,
        EVENT_UNDATED,
        claim_a.claim_ref.as_str(),
        claim_b.claim_ref.as_str(),
        PROPOSAL_REF,
        REVIEW_ITEM_REF,
        OP_EVENT_REF,
        OUTSTANDING_REF,
        no_event.statement_ref.as_str(),
        SEALED_REF,
    ]
    .into_iter()
    .map(|reference| {
        json!({
            "semantic_ref": reference,
            "allowed_roles": ["support-operator", "lawyer"],
            "allowed_purposes": ["handoff-preparation", "legal-advocacy"],
            "knowledge_membership": "known-at-cut",
            "explicitly_selected": true,
            "sealed": reference == SEALED_REF
        })
    })
    .collect::<Vec<_>>();

    let manifest = json!({
        "schema": "sensiblaw.matter-scope.v0_1",
        "matter_ref": MATTER_REF,
        "event_refs": [EVENT_REVIEWED, EVENT_UNDATED],
        "operational_dates": ["2026-09-24"],
        "context": {
            "purpose": "handoff-preparation",
            "role": "support-operator",
            "disclosure_boundary": "role-scoped",
            "knowledge_time_cut_ref": "knowledge-cut:m13:operator:2026-09-24",
            "sealed_refs": [SEALED_REF]
        },
        "visibility": visibility,
        "knowledge_timeline": [
            {
                "semantic_ref": claim_a.claim_ref,
                "source_revision_ref": a_source.source_revision_ref,
                "knowledge_time_ref": "knowledge-time:2026-09-24T10:00:00+10:00",
                "knowledge_membership": "known-at-cut",
                "source_role_ref": "chat"
            },
            {
                "semantic_ref": EVENT_REVIEWED,
                "source_revision_ref": b_source.source_revision_ref,
                "knowledge_time_ref": "knowledge-time:2026-09-24T10:05:00+10:00",
                "knowledge_membership": "known-at-cut",
                "source_role_ref": "operator-note"
            }
        ],
        "legal_proof_refs": [],
        "research_refs": [],
        "work_product_refs": [],
        "handoff_refs": [],
        "no_event_refs": [no_event.statement_ref],
        "acceptance_roles": [
            {
                "semantic_ref": "claim:m13:operator:a",
                "role": "party-assertion",
                "review_ref": "review:m13:operator:party-assertion"
            },
            {
                "semantic_ref": EVENT_REVIEWED,
                "role": "procedural-outcome",
                "review_ref": "review:m13:operator:procedural-outcome"
            }
        ],
        "procedural_significance_review_refs": [REVIEW_ITEM_REF]
    });

    fs::write(
        &output,
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    println!("matter_scope={output}");
    println!("matter_ref={MATTER_REF}");
    println!("conversation_ref={CONVERSATION_REF}");
    println!("review_item_ref={REVIEW_ITEM_REF}");
    println!("reviewer_ref=reviewer:m13:operator");
    println!("selected_refs={EVENT_REVIEWED},claim:m13:operator:a");
    println!("redacted_refs=claim:m13:operator:b");
    println!("sealed_ref={SEALED_REF}");
    println!("canonical_world_mutated=false");
    println!("creates_semantic_authority=false");
    println!("claim_truth_promoted=false");

    Ok(())
}
