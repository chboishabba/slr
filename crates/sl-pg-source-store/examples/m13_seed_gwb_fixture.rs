use std::{env, fs};

use sensiblaw_core::{
    chat_source::{
        ArchivedChatMessage, ChatBranchMembership, ChatContentKind, ChatMessageRole,
    },
    event_discovery::{
        CandidateEventJoinProposal, EventJoinSignal, EventJoinSignalKind,
    },
};
use sensiblaw_pg_source_store::{
    canonical_statement_ref, install_m13_empirical_schema, load_database_config,
    materialize_gwb_chronology_capstone, persist_chat_archive_message,
    persist_event_join_proposal_with_review, ExactSourceSpan,
    GwbChronologyCapstoneManifest, GwbClaimLeaf, GwbClaimLeafKind,
    GwbClaimReviewState, GwbContestationRelation, GwbContestationRelationKind,
    GwbPropositionRoot, GwbReviewAction, GwbReviewItem, GwbReviewItemKind,
    GwbReviewStatus, GwbReviewedEventJoin, GwbReviewedStatement,
    GwbStatementDisposition, GwbStatementOrigin, GwbTemporalAssertion,
    GwbTemporalForm, SourceStatementEnvelope, StatementOrigin, GWB_CAPSTONE_SCHEMA,
};
use serde_json::json;

const MATTER_REF: &str = "matter:gwb:m13-capstone";
const EVENT_REF: &str = "event:gwb:m13:capstone:1";
const PROPOSAL_REF: &str = "proposal:gwb:m13:capstone:auto";
const AUTO_REVIEW_REF: &str = "review-item:proposal:gwb:m13:capstone:auto";
const GWB_REVIEW_REF: &str = "review:gwb:m13:capstone";

fn message(
    message_ref: &str,
    node_ref: &str,
    time: &str,
    content: &str,
) -> ArchivedChatMessage {
    ArchivedChatMessage {
        conversation_ref: "conversation:gwb:m13:fixture".into(),
        message_ref: message_ref.into(),
        node_ref: node_ref.into(),
        parent_node_ref: None,
        branch_membership: ChatBranchMembership::Active,
        message_time_ref: time.into(),
        role: ChatMessageRole::User,
        thread_title: "GWB M13 bounded public-corpus fixture".into(),
        content: content.into(),
        content_kind: ChatContentKind::Message,
        citation_token: None,
        filename: None,
        asset_pointer: None,
        source_scope: Some("gwb-public-fixture".into()),
        body_storage_ref: Some(message_ref.into()),
        mime_type: Some("text/plain".into()),
        archive_source_id: Some("m13-gwb-fixture-v1".into()),
        provenance_ref: Some("fixture:m13:gwb:v1".into()),
        identity_verified: true,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn statement_ref(
    document_ref: &str,
    source_revision_ref: &str,
    span_ref: &str,
    text: &str,
) -> String {
    let mut statement = SourceStatementEnvelope {
        statement_ref: String::new(),
        document_ref: document_ref.into(),
        source_revision_ref: source_revision_ref.into(),
        span: ExactSourceSpan {
            span_ref: span_ref.into(),
            start_char: 0,
            end_char: text.len() as u32,
        },
        literal_text: text.into(),
        origin: StatementOrigin::ResearchReentry,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    statement.statement_ref = canonical_statement_ref(&statement);
    statement.statement_ref
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let capstone_path = args
        .next()
        .unwrap_or_else(|| "m13-gwb-capstone.generated.json".into());
    let scope_path = args
        .next()
        .unwrap_or_else(|| "m13-gwb-matter-scope.generated.json".into());

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    install_m13_empirical_schema(&config)?;

    let book_msg = message(
        "message:gwb:m13:book",
        "node:gwb:m13:book",
        "2026-09-24T09:00:00+10:00",
        "Book account: the demonstration occurred on the recorded date.",
    );
    let web_msg = message(
        "message:gwb:m13:web",
        "node:gwb:m13:web",
        "2026-09-24T09:05:00+10:00",
        "Web account: the demonstration date is disputed.",
    );
    let book =
        persist_chat_archive_message(&config, &book_msg).map_err(|e| e.to_string())?;
    let web =
        persist_chat_archive_message(&config, &web_msg).map_err(|e| e.to_string())?;

    let book_statement_ref = statement_ref(
        &book.document_ref,
        &book.source_revision_ref,
        &book.full_message_span_ref,
        &book_msg.content,
    );
    let web_statement_ref = statement_ref(
        &web.document_ref,
        &web.source_revision_ref,
        &web.full_message_span_ref,
        &web_msg.content,
    );

    let manifest = GwbChronologyCapstoneManifest {
        schema: GWB_CAPSTONE_SCHEMA.into(),
        matter_ref: MATTER_REF.into(),
        handoff_ref: "handoff:gwb:m13-capstone".into(),
        max_selected_statements: 4,
        statements: vec![
            GwbReviewedStatement {
                statement_key: "book".into(),
                document_ordinal: 1,
                document_ref: book.document_ref.clone(),
                source_revision_ref: book.source_revision_ref.clone(),
                span_ref: book.full_message_span_ref.clone(),
                start_char: 0,
                end_char: book_msg.content.len() as u32,
                literal_text: book_msg.content.clone(),
                source_family_ref: "book".into(),
                source_role_ref: "historical-account".into(),
                candidate_pnf_ref: "candidate-pnf:gwb:m13:book".into(),
                observation_ref: "observation:gwb:m13:book".into(),
                parser_receipt_ref: "parser-receipt:gwb:m13:book".into(),
                parse_review_ref: Some("parse-review:gwb:m13:book".into()),
                admission_receipt_ref: None,
                disposition: GwbStatementDisposition::ParseReviewed,
                qualification_ref: None,
                reviewed: true,
                origin: GwbStatementOrigin::ResearchReentry,
            },
            GwbReviewedStatement {
                statement_key: "web".into(),
                document_ordinal: 2,
                document_ref: web.document_ref.clone(),
                source_revision_ref: web.source_revision_ref.clone(),
                span_ref: web.full_message_span_ref.clone(),
                start_char: 0,
                end_char: web_msg.content.len() as u32,
                literal_text: web_msg.content.clone(),
                source_family_ref: "web".into(),
                source_role_ref: "public-web-account".into(),
                candidate_pnf_ref: "candidate-pnf:gwb:m13:web".into(),
                observation_ref: "observation:gwb:m13:web".into(),
                parser_receipt_ref: "parser-receipt:gwb:m13:web".into(),
                parse_review_ref: Some("parse-review:gwb:m13:web".into()),
                admission_receipt_ref: None,
                disposition: GwbStatementDisposition::ParseReviewed,
                qualification_ref: None,
                reviewed: true,
                origin: GwbStatementOrigin::ResearchReentry,
            },
        ],
        event_joins: vec![GwbReviewedEventJoin {
            event_ref: EVENT_REF.into(),
            observation_refs: vec![
                "observation:gwb:m13:book".into(),
                "observation:gwb:m13:web".into(),
            ],
            assembly_receipt_ref: "assembly-receipt:gwb:m13:1".into(),
            join_basis: "reviewed bounded fixture join".into(),
            reviewed: true,
            automatic_join: false,
        }],
        temporal_assertions: vec![GwbTemporalAssertion {
            temporal_ref: "temporal:gwb:m13:1".into(),
            event_ref: EVENT_REF.into(),
            form: GwbTemporalForm::ExactDate {
                date_ref: "1997-01-01".into(),
            },
            statement_keys: vec!["book".into(), "web".into()],
            observation_refs: vec![],
            review_ref: Some("review:gwb:m13:temporal".into()),
        }],
        proposition_roots: vec![GwbPropositionRoot {
            proposition_ref: "proposition:gwb:m13:demonstration".into(),
            label: "The demonstration occurred on the recorded date".into(),
        }],
        claim_leaves: vec![
            GwbClaimLeaf {
                claim_ref: "claim:gwb:m13:capstone:a".into(),
                proposition_ref: "proposition:gwb:m13:demonstration".into(),
                kind: GwbClaimLeafKind::Affirmation,
                speaker_ref: Some("source-role:book".into()),
                statement_keys: vec!["book".into()],
                observation_refs: vec!["observation:gwb:m13:book".into()],
                temporal_refs: vec!["temporal:gwb:m13:1".into()],
                scope_refs: vec![MATTER_REF.into()],
                review_state: GwbClaimReviewState::Accepted,
                review_ref: Some("review:gwb:m13:claim-a".into()),
                event_ref: Some(EVENT_REF.into()),
                event_assembly_receipt_ref: Some(
                    "assembly-receipt:gwb:m13:1".into(),
                ),
            },
            GwbClaimLeaf {
                claim_ref: "claim:gwb:m13:capstone:b".into(),
                proposition_ref: "proposition:gwb:m13:demonstration".into(),
                kind: GwbClaimLeafKind::Qualification,
                speaker_ref: Some("source-role:web".into()),
                statement_keys: vec!["web".into()],
                observation_refs: vec!["observation:gwb:m13:web".into()],
                temporal_refs: vec!["temporal:gwb:m13:1".into()],
                scope_refs: vec![MATTER_REF.into()],
                review_state: GwbClaimReviewState::Qualified,
                review_ref: Some("review:gwb:m13:claim-b".into()),
                event_ref: Some(EVENT_REF.into()),
                event_assembly_receipt_ref: Some(
                    "assembly-receipt:gwb:m13:1".into(),
                ),
            },
        ],
        contestation_relations: vec![GwbContestationRelation {
            relation_ref: "relation:gwb:m13:contradiction".into(),
            from_claim_ref: "claim:gwb:m13:capstone:a".into(),
            to_claim_ref: "claim:gwb:m13:capstone:b".into(),
            kind: GwbContestationRelationKind::Contradicts,
            statement_keys: vec!["book".into(), "web".into()],
            observation_refs: vec![],
            review_ref: Some("review:gwb:m13:relation".into()),
        }],
        review_items: vec![GwbReviewItem {
            review_item_ref: GWB_REVIEW_REF.into(),
            semantic_ref: "relation:gwb:m13:contradiction".into(),
            item_kind: GwbReviewItemKind::ClaimContestation,
            reason: "heterogeneous public accounts require explicit review".into(),
            provenance_refs: vec!["fixture:m13:gwb:v1".into()],
            source_statement_keys: vec!["book".into(), "web".into()],
            current_status: GwbReviewStatus::Pending,
            available_actions: vec![
                GwbReviewAction::Accept,
                GwbReviewAction::Reject,
                GwbReviewAction::Qualify,
                GwbReviewAction::RequestEvidence,
                GwbReviewAction::OpenSource,
            ],
            affected_consumer_refs: vec![MATTER_REF.into()],
        }],
        candidate_only: true,
        semantic_promotion: false,
    };

    materialize_gwb_chronology_capstone(&config, &manifest)
        .map_err(|error| error.to_string())?;

    persist_event_join_proposal_with_review(
        &config,
        &CandidateEventJoinProposal {
            proposal_ref: PROPOSAL_REF.into(),
            observation_refs: vec![
                "observation:gwb:m13:book".into(),
                "observation:gwb:m13:web".into(),
            ],
            statement_refs: vec![
                book_statement_ref.clone(),
                web_statement_ref.clone(),
            ],
            source_family_refs: vec!["book".into(), "web".into()],
            signals: vec![
                EventJoinSignal {
                    kind: EventJoinSignalKind::SharedTemporalBucket,
                    evidence_ref: "signal:gwb:m13:date".into(),
                    detector_ref: "fixture:m13:gwb".into(),
                },
                EventJoinSignal {
                    kind: EventJoinSignalKind::SharedEntity,
                    evidence_ref: "signal:gwb:m13:demonstration".into(),
                    detector_ref: "fixture:m13:gwb".into(),
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

    fs::write(
        &capstone_path,
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let visible = [
        EVENT_REF,
        "claim:gwb:m13:capstone:a",
        "claim:gwb:m13:capstone:b",
        PROPOSAL_REF,
        AUTO_REVIEW_REF,
        GWB_REVIEW_REF,
    ]
    .into_iter()
    .map(|reference| {
        json!({
            "semantic_ref": reference,
            "allowed_roles": ["researcher"],
            "allowed_purposes": ["research-publication"],
            "knowledge_membership": "known-at-cut",
            "explicitly_selected": true,
            "sealed": false
        })
    })
    .collect::<Vec<_>>();

    let scope = json!({
        "schema": "sensiblaw.matter-scope.v0_1",
        "matter_ref": MATTER_REF,
        "event_refs": [EVENT_REF],
        "operational_dates": [],
        "context": {
            "purpose": "research-publication",
            "role": "researcher",
            "disclosure_boundary": "role-scoped",
            "knowledge_time_cut_ref": "knowledge-cut:gwb:m13",
            "sealed_refs": []
        },
        "visibility": visible,
        "knowledge_timeline": [
            {
                "semantic_ref": EVENT_REF,
                "source_revision_ref": book.source_revision_ref,
                "knowledge_time_ref": "knowledge-time:gwb:m13:book",
                "knowledge_membership": "known-at-cut",
                "source_role_ref": "book"
            },
            {
                "semantic_ref": "claim:gwb:m13:capstone:a",
                "source_revision_ref": web.source_revision_ref,
                "knowledge_time_ref": "knowledge-time:gwb:m13:web",
                "knowledge_membership": "known-at-cut",
                "source_role_ref": "web"
            }
        ],
        "legal_proof_refs": [],
        "research_refs": [GWB_REVIEW_REF],
        "work_product_refs": [],
        "handoff_refs": [],
        "no_event_refs": [],
        "acceptance_roles": [],
        "procedural_significance_review_refs": []
    });

    fs::write(
        &scope_path,
        serde_json::to_string_pretty(&scope).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    println!("gwb_capstone_manifest={capstone_path}");
    println!("matter_scope={scope_path}");
    println!("matter_ref={MATTER_REF}");
    println!("selected_refs={EVENT_REF},claim:gwb:m13:capstone:a");
    println!("redacted_refs=claim:gwb:m13:capstone:b");
    println!("source_family_count=2");
    println!("canonical_world_mutated=false");
    println!("creates_semantic_authority=false");
    println!("claim_truth_promoted=false");

    Ok(())
}
