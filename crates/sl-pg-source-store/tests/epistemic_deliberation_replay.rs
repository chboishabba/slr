//! Opt-in physical PostgreSQL acceptance: source bytes -> exact spans ->
//! S29 review -> persisted sidecar -> scoped reopen. Requires a disposable,
//! migrated test DB. No live Wikidata network call or external edit is made.
use sensiblaw_pg_source_store::{
    apply_deliberation_diagnostic_review, canonical_generic_source_revision_ref,
    load_database_config, load_deliberation_diagnostic_scoped,
    persist_deliberation_diagnostic, persist_deliberation_from_source_inputs,
    persist_deliberation_source_replay_json,
    DeliberationSourceInput,
    DeliberationEvent, DeliberationEventAnchor, DeliberationKind, DeliberationJudgement,
    DeliberationPacket, DeliberationSourceSeal, ReviewAction, ReviewCommand, ReviewStatus,
};
use sensiblaw_core::source_ingest::{IngestRoleClass, SourceFamily, SourceIngestEnvelope};
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}",Sha256::digest(bytes))
}
#[test]
fn persisted_event_spans_and_scoped_s29_review_reopen_without_promotion() {
    if std::env::var("SLR_DELIBERATION_PG_ACCEPTANCE").as_deref() != Ok("1") {
        eprintln!("skipped: set SLR_DELIBERATION_PG_ACCEPTANCE=1 only for disposable migrated DATABASE_URL");
        return;
    }
    let config=load_database_config(None).expect("explicit disposable DATABASE_URL required");
    let unique=format!("proposal-test:{}:{}",std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

    let source_kinds=["proposal","historical-policy","proposer","objection","decision"];
    let mut source_refs=Vec::new();
    let mut seals=Vec::new();
    let mut anchor_refs=Vec::new();
    let mut source_inputs=Vec::new();
    for label in source_kinds {
        let source_text=format!("Source revision for {label} from {unique}. Case under review.");
        let content_digest=hash(source_text.as_bytes());
        let source_ref=if label=="historical-policy" {
            "wikidata:page:1002".to_owned()
        } else {
            format!("source:{unique}:{label}")
        };
        let acquisition_ref=format!("acquisition:{unique}:{label}");
        let revision=canonical_generic_source_revision_ref(
            &source_ref,"wikidata:mediawiki-api",&acquisition_ref,
            &content_digest,"text/plain",
        );
        let envelope=SourceIngestEnvelope {
            source_ref,
            source_revision_ref:revision.clone(),
            provider_ref:"wikidata:mediawiki-api".into(),
            family:SourceFamily::Wiki,
            role_class:IngestRoleClass::ContentSource,
            content_digest_ref:content_digest.clone(),
            acquisition_receipt_ref:acquisition_ref.clone(),
            media_type_ref:"text/x-wiki".into(),
            candidate_only:true,creates_semantic_authority:false,
            applicability_promoted:false,claim_truth_promoted:false,
        };
        assert_eq!(envelope.content_digest_ref,content_digest);
        source_inputs.push(DeliberationSourceInput {
            envelope,canonical_text:source_text.clone(),
        });
        source_refs.push(revision.clone());
        seals.push(DeliberationSourceSeal {
            source_revision_ref:revision.clone(),content_digest_ref:content_digest,
            acquisition_receipt_ref:acquisition_ref,
        });
        anchor_refs.push(DeliberationEventAnchor {
            event_ref:format!("event:{label}"),
            source_revision_ref:revision,
            start_byte:0,end_byte:source_text.len(),
            excerpt_digest_ref:hash(source_text.as_bytes()),
        });
    }
    let decision=DeliberationEvent {
        event_ref:"event:decision".into(),source_revision_ref:source_refs[4].clone(),
        actor_ref:"editor:other-closer".into(),at_unix_seconds:400000,
        kind:DeliberationKind::Decision,supersedes_ref:None,
        judgement:Some(DeliberationJudgement::Defer),
        consensus_assessment_ref:None,authority_receipt_ref:None,
    };
    let packet=DeliberationPacket {
        proposal_ref:format!("proposal:{unique}"),
        source_revision_ref:source_refs[0].clone(),
        policy_revision_ref:source_refs[1].clone(),
        minimum_age_seconds:86400,
        events:vec![
            DeliberationEvent {
                event_ref:"event:proposer".into(),source_revision_ref:source_refs[2].clone(),
                actor_ref:"editor:proposer".into(),at_unix_seconds:1,
                kind:DeliberationKind::Proposal,supersedes_ref:None,
                judgement:None,consensus_assessment_ref:None,authority_receipt_ref:None,
            },
            DeliberationEvent {
                event_ref:"event:objection".into(),source_revision_ref:source_refs[3].clone(),
                actor_ref:"editor:critic".into(),at_unix_seconds:100000,
                kind:DeliberationKind::Objection,supersedes_ref:None,
                judgement:None,consensus_assessment_ref:None,authority_receipt_ref:None,
            },
            decision.clone(),
        ],
        created_property_ref:None,
    };
    // Drop the two non-event anchors; event anchors are ordered by event ref
    // on SLR persistence and refer only to source-native observed events.
    let anchors=anchor_refs.into_iter().skip(2).collect::<Vec<_>>();
    let scope=format!("matter:{unique}");
    let result=persist_deliberation_from_source_inputs(
        &config,&packet,&source_inputs,&anchors,&scope,
    ).expect("source, byte seals and S29 sidecar must persist and reopen");
    assert!(!result.assessment.record_reviewable);
    assert!(!result.grants_wikidata_edit_authority);
    assert!(!result.ao_dispatch_allowed);
    assert_eq!(result.s29_status,ReviewStatus::Pending);
    let again=persist_deliberation_from_source_inputs(
        &config,&packet,&source_inputs,&anchors,&scope,
    ).expect("idempotent source+sidecar replay");
    assert_eq!(again.diagnostic_ref,result.diagnostic_ref);
    assert_eq!(again.review_item.review_item_ref,result.review_item.review_item_ref);

    // Exact ITIR handoff format, with the same canonical source inputs that
    // the standalone INGEST-1 path just persisted. No faux 2022 policy is used.
    let replay_sources=source_inputs.iter().map(|input| {
        let e=&input.envelope;
        serde_json::json!({
            "envelope":{
                "source_ref":e.source_ref,"source_revision_ref":e.source_revision_ref,
                "provider_ref":e.provider_ref,"family":"wiki",
                "role_class":"content_source","content_digest_ref":e.content_digest_ref,
                "acquisition_receipt_ref":e.acquisition_receipt_ref,
                "media_type_ref":e.media_type_ref,
                "candidate_only":true,"creates_semantic_authority":false,
                "applicability_promoted":false,"claim_truth_promoted":false,
            },
            "canonical_text":input.canonical_text,
        })
    }).collect::<Vec<_>>();
    let replay=serde_json::json!({
        "version":"itir.governance.wikidata_source_replay_handoff.v1",
        "origin":"sl.wikidata_deliberation_slr_handoff_draft.v0_1",
        "slr_input":packet,
        "slr_source_seals":seals,
        "slr_event_anchors":anchors,
        "slr_source_inputs":replay_sources,
        "historical_policy_drift_status":"unchanged_across_proposal_window",
        "historical_policy_window_source_witness":{
            "opening_policy_page_oldid":1002,
            "closing_policy_page_oldid":1002,
            "opening_policy_revision_time":"2021-12-30T00:00:00Z",
            "closing_policy_revision_time":"2021-12-30T00:00:00Z",
            "policy_content_digest_ref":source_inputs[1].envelope.content_digest_ref,
            "closing_policy_content_digest_ref":source_inputs[1].envelope.content_digest_ref,
            "closing_acquisition_url":"https://www.wikidata.org/w/api.php?revid=1002",
            "comparison_status":"unchanged_across_proposal_window",
            "candidate_only":true,
            "policy_normatively_verified":false,
        },
        "normative_policy_hold":false,
        "holds":serde_json::to_value(&result.assessment.holds).unwrap(),
        "record_completeness":if result.assessment.record_reviewable {
            "reviewable"
        } else {"needs_evidence"},
        "scope":"not_assigned_until_s29",
        "independent_source_verification":"not_completed",
        "community_creator_role_verification":"not_completed",
        "community_consensus_verification":"not_completed",
        "authority":{
            "candidate_only":true,"slr_pg_reopened":false,
            "community_consensus_verified":false,
            "wikidata_edit_authorised":false,"ao_dispatch_allowed":false,
        },
    });
    let replayed=persist_deliberation_source_replay_json(
        &config,&replay.to_string(),&scope
    ).expect("exact full ITIR JSON must deserialize into same SLR review");
    assert_eq!(replayed.diagnostic_ref,result.diagnostic_ref);
    let mut bad_window=replay.clone();
    bad_window["historical_policy_window_source_witness"]["opening_policy_page_oldid"]=1003.into();
    assert!(persist_deliberation_source_replay_json(
        &config,&bad_window.to_string(),&scope
    ).is_err());
    let mut mismatched=replay.clone();
    mismatched["slr_source_inputs"][0]["canonical_text"]="MISMATCH".into();
    assert!(persist_deliberation_source_replay_json(
        &config,&mismatched.to_string(),&scope
    ).is_err());
    let mut unexamined=replay.clone();
    unexamined["historical_policy_drift_status"]="closure_policy_not_examined".into();
    unexamined["normative_policy_hold"]=true.into();
    assert!(persist_deliberation_source_replay_json(
        &config,&unexamined.to_string(),&scope,
    ).is_err());
    let mut changed=replay.clone();
    changed["historical_policy_drift_status"]=
        "changed_during_proposal_requires_independent_rule_adjudication".into();
    changed["normative_policy_hold"]=true.into();
    assert!(persist_deliberation_source_replay_json(
        &config,&changed.to_string(),&scope,
    ).is_err());
    let mut forged=replay;
    forged["authority"]["wikidata_edit_authorised"]=true.into();
    assert!(persist_deliberation_source_replay_json(
        &config,&forged.to_string(),&scope
    ).is_err());

    assert!(load_deliberation_diagnostic_scoped(
        &config,&result.diagnostic_ref,"matter:unrelated"
    ).is_err());
    let mut invalid_anchor=anchors.clone();
    invalid_anchor[0].excerpt_digest_ref=hash(b"modified source");
    assert!(persist_deliberation_diagnostic(
        &config,&packet,&seals,&invalid_anchor,&scope,
    ).is_err());
    let mut wrong_seal=seals.clone();
    wrong_seal[0].content_digest_ref=hash(b"wrong canonical content");
    assert!(persist_deliberation_diagnostic(
        &config,&packet,&wrong_seal,&anchors,&scope,
    ).is_err());

    let command=ReviewCommand {
        command_ref:format!("review-command:{unique}"),
        review_item_ref:result.review_item.review_item_ref.clone(),
        action:ReviewAction::RequestEvidence,
        reviewer_ref:"reviewer:fictional".into(),
        qualification_ref:None,
        evidence_request_ref:Some(format!("request:{unique}")),
    };
    let (receipt,reviewed)=apply_deliberation_diagnostic_review(
        &config,&result.diagnostic_ref,&scope,&command,
    ).expect("ordinary S29 reviewer must pay review-only transition");
    assert!(receipt.candidate_only && !receipt.claim_truth_promoted);
    assert_eq!(reviewed.s29_status,ReviewStatus::NeedsEvidence);
    let loaded=load_deliberation_diagnostic_scoped(
        &config,&result.diagnostic_ref,&scope
    ).unwrap().expect("bound source should reopen");
    assert_eq!(loaded.s29_status,ReviewStatus::NeedsEvidence);
    assert_eq!(loaded.packet,packet);
    assert_eq!(loaded.event_anchors.len(),3);
    assert!(!loaded.grants_wikidata_edit_authority);
}
