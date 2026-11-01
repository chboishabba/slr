//! Durable source-bound community deliberation review on the existing S29 carrier.
//!
//! This is deliberately separate from WIKI-1's *executed JMD Lean checker*
//! packet. A community proposal must never fabricate a Lean producer receipt.
//! Roszkowski's empirical framework motivates the event history; this Rust
//! transport and its replay checks are locally authored candidate governance.
use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};
use sensiblaw_core::review_workstation::{
    ReviewCommand, ReviewItem, ReviewItemKind, ReviewReceipt, ReviewStatus,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    assess_deliberation, project_s29_diagnostic_review_item, DatabaseConfig,
    DeliberationAssessment, DeliberationError, DeliberationPacket,
    GenericSourceContentStoreError, ReviewWorkstationStoreError,
};

pub const DELIBERATION_SIDECAR_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.epistemic_deliberation_diagnostic (
  diagnostic_ref TEXT PRIMARY KEY,
  review_item_ref TEXT NOT NULL UNIQUE REFERENCES semantic.review_item(review_item_ref),
  packet_ref TEXT NOT NULL,
  source_revision_ref TEXT NOT NULL,
  consumer_scope_ref TEXT NOT NULL,
  packet_digest_ref TEXT NOT NULL,
  packet_json TEXT NOT NULL,
  source_seals_json TEXT NOT NULL,
  source_seals_digest_ref TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK(candidate_only),
  semantic_truth_promoted BOOLEAN NOT NULL CHECK(NOT semantic_truth_promoted),
  consensus_verified BOOLEAN NOT NULL CHECK(NOT consensus_verified),
  actor_authority_verified BOOLEAN NOT NULL CHECK(NOT actor_authority_verified),
  grants_wikidata_edit_authority BOOLEAN NOT NULL CHECK(NOT grants_wikidata_edit_authority),
  ao_dispatch_allowed BOOLEAN NOT NULL CHECK(NOT ao_dispatch_allowed)
);
CREATE INDEX IF NOT EXISTS deliberation_source_scope_idx
 ON semantic.epistemic_deliberation_diagnostic(source_revision_ref,consumer_scope_ref);
"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationSourceSeal {
    pub source_revision_ref: String,
    pub content_digest_ref: String,
    pub acquisition_receipt_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedDeliberation {
    pub diagnostic_ref: String,
    pub consumer_scope_ref: String,
    pub packet: DeliberationPacket,
    pub assessment: DeliberationAssessment,
    pub source_seals: Vec<DeliberationSourceSeal>,
    pub review_item: ReviewItem,
    pub s29_status: ReviewStatus,
    pub candidate_only: bool,
    pub grants_wikidata_edit_authority: bool,
    pub ao_dispatch_allowed: bool,
}
#[derive(Debug, Error)]
pub enum DeliberationStoreError {
    #[error(transparent)]
    Pg(#[from] postgres::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Diagnostic(#[from] DeliberationError),
    #[error(transparent)]
    Source(#[from] GenericSourceContentStoreError),
    #[error(transparent)]
    Review(#[from] ReviewWorkstationStoreError),
    #[error("event, policy or proposal source evidence does not match pinned source revisions")]
    SourceBindingMismatch,
    #[error("deliberation diagnostic or S29 review identity conflicts")]
    IdentityConflict,
    #[error("unknown deliberation diagnostic")]
    UnknownDiagnostic,
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn bound_ref(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 512 && !s.chars().any(char::is_control)
}
fn source_refs(packet: &DeliberationPacket) -> BTreeSet<String> {
    let mut refs: BTreeSet<String> = [
        packet.source_revision_ref.clone(),
        packet.policy_revision_ref.clone(),
    ].into_iter().collect();
    refs.extend(packet.events.iter().map(|e| e.source_revision_ref.clone()));
    refs
}

/// Require exactly one independently reopenable content seal for every input
/// revision, including the historical policy. No source omission is ignored.
pub fn validate_deliberation_source_seals(
    packet: &DeliberationPacket, seals: &[DeliberationSourceSeal],
) -> Result<Vec<DeliberationSourceSeal>, DeliberationStoreError> {
    assess_deliberation(packet)?;
    let mut lookup = BTreeMap::new();
    for seal in seals {
        if !bound_ref(&seal.source_revision_ref)
            || !bound_ref(&seal.content_digest_ref)
            || !bound_ref(&seal.acquisition_receipt_ref)
            || !seal.content_digest_ref.starts_with("sha256:")
            || seal.content_digest_ref.len() != 71
            || !seal.content_digest_ref[7..].bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || lookup.insert(seal.source_revision_ref.clone(), seal.clone()).is_some()
        {
            return Err(DeliberationStoreError::SourceBindingMismatch);
        }
    }
    if source_refs(packet) != lookup.keys().cloned().collect() {
        return Err(DeliberationStoreError::SourceBindingMismatch);
    }
    Ok(lookup.into_values().collect())
}

/// Reopen canonical source *bytes* through SLR's one generic source owner.
/// A caller-supplied seal cannot authenticate itself.
fn reopen_sources(
    config: &DatabaseConfig, seals: &[DeliberationSourceSeal],
) -> Result<(), DeliberationStoreError> {
    for seal in seals {
        let (record, _) = crate::load_generic_text_source(config, &seal.source_revision_ref)?;
        if record.content_digest_ref != seal.content_digest_ref
            || record.acquisition_receipt_ref != seal.acquisition_receipt_ref
            || record.source_revision_ref != seal.source_revision_ref
            || !record.candidate_only || record.creates_semantic_authority
            || record.applicability_promoted || record.claim_truth_promoted
        {
            return Err(DeliberationStoreError::SourceBindingMismatch);
        }
    }
    Ok(())
}
fn identity(packet_ref: &str, scope: &str) -> String {
    let mut h = Sha256::new();
    for s in ["epistemic-deliberation-s29-v1",packet_ref,scope] {
        h.update((s.len() as u64).to_be_bytes());
        h.update(s.as_bytes());
    }
    format!("deliberation-review:sha256:{:x}", h.finalize())
}
fn projected_item(
    assessment: &DeliberationAssessment, scope: &str,
) -> Result<ReviewItem, DeliberationStoreError> {
    let mut item = project_s29_diagnostic_review_item(assessment, scope)?;
    item.review_item_ref = format!("review-item:{}", identity(&assessment.packet_ref,scope));
    item.source_refs.push(assessment.policy_revision_ref.clone());
    item.source_refs.sort();
    item.source_refs.dedup();
    Ok(item)
}
fn item_equivalent(expected: &ReviewItem, actual: &ReviewItem) -> bool {
    expected.review_item_ref == actual.review_item_ref
    && expected.semantic_ref == actual.semantic_ref
    && expected.item_kind == actual.item_kind
    && expected.reason == actual.reason
    && expected.provenance_refs == actual.provenance_refs
    && expected.source_refs == actual.source_refs
    && expected.available_actions == actual.available_actions
    && expected.affected_consumer_refs == actual.affected_consumer_refs
    && actual.candidate_only && !actual.creates_semantic_authority
    && !actual.applicability_promoted && !actual.claim_truth_promoted
}

/// Record a deliberation diagnostic only after reopening every original
/// proposal/event/policy source. The review item is the existing S29 object;
/// a source/review receipt is not a Wikidata community role attestation.
pub fn persist_deliberation_diagnostic(
    config: &DatabaseConfig,
    packet: &DeliberationPacket,
    seals: &[DeliberationSourceSeal],
    consumer_scope_ref: &str,
) -> Result<PersistedDeliberation, DeliberationStoreError> {
    if !bound_ref(consumer_scope_ref) {
        return Err(DeliberationStoreError::IdentityConflict);
    }
    let ordered = validate_deliberation_source_seals(packet,seals)?;
    reopen_sources(config,&ordered)?;
    let assessment = assess_deliberation(packet)?;
    let diagnostic_ref = identity(&assessment.packet_ref,consumer_scope_ref);
    let item = projected_item(&assessment,consumer_scope_ref)?;
    let json = serde_json::to_string(packet)?;
    let seal_json = serde_json::to_string(&ordered)?;

    crate::install_review_workstation_schema(config)?;
    let mut client = Client::connect(config.database_url(),NoTls)?;
    client.batch_execute(DELIBERATION_SIDECAR_SQL)?;
    if let Some(existing) = crate::load_review_item(config,&item.review_item_ref)? {
        if !item_equivalent(&item,&existing) {
            return Err(DeliberationStoreError::IdentityConflict);
        }
    } else {
        let _ = crate::persist_review_item(config,&item)?;
    }
    client.execute(
        "INSERT INTO semantic.epistemic_deliberation_diagnostic
         (diagnostic_ref,review_item_ref,packet_ref,source_revision_ref,consumer_scope_ref,
          packet_digest_ref,packet_json,source_seals_json,source_seals_digest_ref,
          candidate_only,semantic_truth_promoted,consensus_verified,actor_authority_verified,
          grants_wikidata_edit_authority,ao_dispatch_allowed)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,TRUE,FALSE,FALSE,FALSE,FALSE,FALSE)
         ON CONFLICT (diagnostic_ref) DO NOTHING",
        &[&diagnostic_ref,&item.review_item_ref,&assessment.packet_ref,
          &packet.source_revision_ref,&consumer_scope_ref,&digest(json.as_bytes()),
          &json,&seal_json,&digest(seal_json.as_bytes())],
    )?;
    let reopened = load_deliberation_diagnostic(config,&diagnostic_ref)?
        .ok_or(DeliberationStoreError::IdentityConflict)?;
    if reopened.packet != *packet
        || reopened.source_seals != ordered
        || reopened.consumer_scope_ref != consumer_scope_ref
    {
        return Err(DeliberationStoreError::IdentityConflict);
    }
    Ok(reopened)
}

/// Replay checks content and source seals again, not only a PG sidecar row.
/// All S29 status changes remain in its existing review-workstation tables.
pub fn load_deliberation_diagnostic(
    config: &DatabaseConfig, diagnostic_ref: &str,
) -> Result<Option<PersistedDeliberation>, DeliberationStoreError> {
    let mut client = Client::connect(config.database_url(),NoTls)?;
    client.batch_execute(DELIBERATION_SIDECAR_SQL)?;
    let Some(row) = client.query_opt(
        "SELECT review_item_ref,packet_ref,source_revision_ref,consumer_scope_ref,
                packet_digest_ref,packet_json,source_seals_json,source_seals_digest_ref,
                candidate_only,semantic_truth_promoted,consensus_verified,
                actor_authority_verified,grants_wikidata_edit_authority,ao_dispatch_allowed
         FROM semantic.epistemic_deliberation_diagnostic WHERE diagnostic_ref=$1",
        &[&diagnostic_ref],
    )? else {return Ok(None)};
    let json:String = row.get(5);
    let seal_json:String = row.get(6);
    if digest(json.as_bytes()) != row.get::<_,String>(4)
        || digest(seal_json.as_bytes()) != row.get::<_,String>(7)
        || !row.get::<_,bool>(8) || (9..=13).any(|i| row.get::<_,bool>(i))
    {
        return Err(DeliberationStoreError::IdentityConflict);
    }
    let packet:DeliberationPacket = serde_json::from_str(&json)?;
    let seals:Vec<DeliberationSourceSeal> = serde_json::from_str(&seal_json)?;
    let seals = validate_deliberation_source_seals(&packet,&seals)?;
    reopen_sources(config,&seals)?;
    let assessment = assess_deliberation(&packet)?;
    let scope:String = row.get(3);
    let expected_ref = identity(&assessment.packet_ref,&scope);
    let item = projected_item(&assessment,&scope)?;
    let stored_item: String = row.get(0);
    let s29 = crate::load_review_item(config,&stored_item)?
        .ok_or(DeliberationStoreError::IdentityConflict)?;
    if expected_ref != diagnostic_ref
        || !bound_ref(&scope)
        || assessment.packet_ref != row.get::<_,String>(1)
        || packet.source_revision_ref != row.get::<_,String>(2)
        || stored_item != item.review_item_ref
        || s29.item_kind != ReviewItemKind::OntologyDiagnostic
        || !item_equivalent(&item,&s29)
    {
        return Err(DeliberationStoreError::IdentityConflict);
    }
    let status = s29.current_status;
    Ok(Some(PersistedDeliberation {
        diagnostic_ref:diagnostic_ref.into(),consumer_scope_ref:scope,
        packet,assessment,source_seals:seals,review_item:s29,
        s29_status:status,candidate_only:true,
        grants_wikidata_edit_authority:false,ao_dispatch_allowed:false,
    }))
}
/// Apply only an S29 action already offered by the projection. A deliberate
/// omission of Accept prevents a local diagnostic from being mistaken for
/// community approval. Locking and review receipts are owned by S29.
pub fn apply_deliberation_diagnostic_review(
    config: &DatabaseConfig, diagnostic_ref: &str, command: &ReviewCommand,
) -> Result<(ReviewReceipt,PersistedDeliberation),DeliberationStoreError> {
    let before=load_deliberation_diagnostic(config,diagnostic_ref)?
        .ok_or(DeliberationStoreError::UnknownDiagnostic)?;
    if command.review_item_ref!=before.review_item.review_item_ref {
        return Err(DeliberationStoreError::IdentityConflict);
    }
    let (receipt,item)=crate::apply_persisted_review_command(config,command)?;
    let after=load_deliberation_diagnostic(config,diagnostic_ref)?
        .ok_or(DeliberationStoreError::IdentityConflict)?;
    if !receipt.candidate_only || receipt.creates_semantic_authority
        || receipt.applicability_promoted || receipt.claim_truth_promoted
        || item.item_kind != ReviewItemKind::OntologyDiagnostic
        || after.s29_status != item.current_status
        || after.packet != before.packet || after.source_seals != before.source_seals
        || after.consumer_scope_ref != before.consumer_scope_ref
    {
        return Err(DeliberationStoreError::IdentityConflict);
    }
    Ok((receipt,after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeliberationEvent,DeliberationKind,DeliberationJudgement};
    fn sample() -> DeliberationPacket {
        DeliberationPacket {
            proposal_ref:"proposal:demo".into(),source_revision_ref:"source:main".into(),
            policy_revision_ref:"source:policy".into(),minimum_age_seconds:86400,
            created_property_ref:None,
            events:vec![
                DeliberationEvent{event_ref:"event:1".into(),source_revision_ref:"source:1".into(),
                    actor_ref:"author".into(),at_unix_seconds:1,kind:DeliberationKind::Proposal,
                    supersedes_ref:None,judgement:None,consensus_assessment_ref:None,
                    authority_receipt_ref:None},
                DeliberationEvent{event_ref:"event:2".into(),source_revision_ref:"source:2".into(),
                    actor_ref:"critic".into(),at_unix_seconds:100000,kind:DeliberationKind::Objection,
                    supersedes_ref:None,judgement:None,consensus_assessment_ref:None,
                    authority_receipt_ref:None},
                DeliberationEvent{event_ref:"event:3".into(),source_revision_ref:"source:3".into(),
                    actor_ref:"closer".into(),at_unix_seconds:200000,kind:DeliberationKind::Decision,
                    supersedes_ref:None,judgement:Some(DeliberationJudgement::Defer),
                    consensus_assessment_ref:None,authority_receipt_ref:None},
            ],
        }
    }
    fn seal(r: &str) -> DeliberationSourceSeal {
        DeliberationSourceSeal {
            source_revision_ref:r.into(),
            content_digest_ref:format!("sha256:{}", "a".repeat(64)),
            acquisition_receipt_ref:"acquire:fixture".into(),
        }
    }
    #[test]
    fn exact_source_revisions_required_including_historical_policy() {
        let p=sample();
        let refs=source_refs(&p);
        let seals=refs.iter().map(|s|seal(s)).collect::<Vec<_>>();
        let result=validate_deliberation_source_seals(&p,&seals).unwrap();
        assert_eq!(result.len(),5);
        assert_eq!(result[0].source_revision_ref,"source:1");
        let mut missing=seals.clone();missing.pop();
        assert!(matches!(validate_deliberation_source_seals(&p,&missing),
            Err(DeliberationStoreError::SourceBindingMismatch)));
        let mut duplicates=seals.clone();duplicates.push(seal("source:1"));
        assert!(matches!(validate_deliberation_source_seals(&p,&duplicates),
            Err(DeliberationStoreError::SourceBindingMismatch)));
        let mut invented=seals.clone();invented.push(seal("source:invented"));
        assert!(matches!(validate_deliberation_source_seals(&p,&invented),
            Err(DeliberationStoreError::SourceBindingMismatch)));
        let mut fake=seals;fake[0].content_digest_ref="sha256:bad".into();
        assert!(matches!(validate_deliberation_source_seals(&p,&fake),
            Err(DeliberationStoreError::SourceBindingMismatch)));
    }
    #[test]
    fn scopes_never_alias_and_s29_does_not_offer_external_acceptance() {
        let p=sample();
        let a=assess_deliberation(&p).unwrap();
        let one=projected_item(&a,"matter:one").unwrap();
        let two=projected_item(&a,"matter:two").unwrap();
        assert_ne!(one.review_item_ref,two.review_item_ref);
        assert_eq!(one.item_kind,ReviewItemKind::OntologyDiagnostic);
        assert!(one.source_refs.contains(&"source:policy".to_owned()));
        assert!(!one.available_actions.contains(&sensiblaw_core::review_workstation::ReviewAction::Accept));
        assert!(one.validate().is_ok());
    }
}
