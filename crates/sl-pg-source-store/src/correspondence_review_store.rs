//! M10.4 — relation-focused source-correspondence review *using S29*.
//!
//! The sidecar owns immutable identity and witness coordinates of one proposed
//! relation. S29 alone owns review commands, status, receipts, locking and
//! reopen. Neither the sidecar nor Accept establishes subject/event identity,
//! copying, statistical independence or proposition truth.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;
use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewCommand, ReviewItem, ReviewItemKind, ReviewReceipt,
    ReviewStatus,
};
use crate::{DatabaseConfig, ContextVisibility, load_mixed_source_comparison};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrespondenceAxis {
    SameSubject,
    SameEvent,
    Quotation,
    SourceDependency,
}
impl CorrespondenceAxis {
    pub fn key(self) -> &'static str {
        match self {
            Self::SameSubject => "same_subject",
            Self::SameEvent => "same_event",
            Self::Quotation => "quotation",
            Self::SourceDependency => "source_dependency",
        }
    }
    fn parse(raw: &str) -> Result<Self, CorrespondenceReviewError> {
        match raw {
            "same_subject" => Ok(Self::SameSubject),
            "same_event" => Ok(Self::SameEvent),
            "quotation" => Ok(Self::Quotation),
            "source_dependency" => Ok(Self::SourceDependency),
            _ => Err(CorrespondenceReviewError::InvalidAxis),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrespondenceReviewProposal {
    pub relation_ref: String,
    pub review_item_ref: String,
    pub left_source_revision_ref: String,
    pub right_source_revision_ref: String,
    pub axis: CorrespondenceAxis,
    pub evidence_ref: String,
    pub consumer_scope_ref: String,
    pub review_status: ReviewStatus,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
    pub independence_established: bool,
}

#[derive(Debug, Error)]
pub enum CorrespondenceReviewError {
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    S29(#[from] crate::ReviewWorkstationStoreError),
    #[error(transparent)]
    Comparison(#[from] crate::MixedSourceReviewError),
    #[error("relation axis is unknown")]
    InvalidAxis,
    #[error("scope or required coordinates missing")]
    MissingCoordinate,
    #[error("witness is not part of the reopened pair's PNF or native source joins")]
    UnownedEvidence,
    #[error("candidate relation conflicts with already persisted coordinates")]
    ImmutableRelationConflict,
    #[error("review item is not an owned source-correspondence relation")]
    UnownedReviewItem,
    #[error("S29 review receipt crossed non-promotion boundary")]
    ReviewPromotion,
}

pub const CORRESPONDENCE_REVIEW_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.source_correspondence_review (
    relation_ref TEXT PRIMARY KEY,
    review_item_ref TEXT NOT NULL UNIQUE REFERENCES semantic.review_item(review_item_ref),
    left_source_revision_ref TEXT NOT NULL,
    right_source_revision_ref TEXT NOT NULL,
    axis_ref TEXT NOT NULL CHECK (axis_ref IN
      ('same_subject','same_event','quotation','source_dependency')),
    evidence_ref TEXT NOT NULL,
    consumer_scope_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    independence_established BOOLEAN NOT NULL CHECK (NOT independence_established),
    CHECK (left_source_revision_ref <> right_source_revision_ref)
);
CREATE INDEX IF NOT EXISTS source_correspondence_review_pair_idx
  ON semantic.source_correspondence_review
  (left_source_revision_ref, right_source_revision_ref, consumer_scope_ref);
"#;

fn hash_ref(domain: &str, values: &[&str]) -> String {
    let mut h=Sha256::new();
    for value in std::iter::once(domain).chain(values.iter().copied()) {
        h.update((value.len() as u64).to_be_bytes());
        h.update(value.as_bytes());
    }
    format!("sha256:{:x}",h.finalize())
}
fn nonempty(s:&str)->bool { !s.trim().is_empty() }

/// This is *review admission*, not semantic admission. The operator explicitly
/// selects a typed hypothesis and one witness among the existing persisted
/// PNF fingerprints or registered native joins. An unrelated shared string is
/// not a witness. Event/subject hypotheses require matching candidate kind.
/// A source-backreference can nominate a quotation/dependency question, but
/// does not prove quotation or copying.
pub fn propose_correspondence_review(
    config: &DatabaseConfig,
    left: &str,
    right: &str,
    chat_message_ref: Option<&str>,
    axis: CorrespondenceAxis,
    evidence_ref: &str,
    consumer_scope_ref: &str,
) -> Result<CorrespondenceReviewProposal,CorrespondenceReviewError> {
    if !nonempty(consumer_scope_ref) || !nonempty(evidence_ref)
        || !nonempty(left) || !nonempty(right)
    {return Err(CorrespondenceReviewError::MissingCoordinate);}
    let comparison=load_mixed_source_comparison(
        config,left,right,chat_message_ref,ContextVisibility::ExcludedByScope
    )?;
    let owned=match axis {
        CorrespondenceAxis::SameSubject =>
            comparison.shared_entity_candidate_refs.iter()
                .any(|v|v==evidence_ref),
        CorrespondenceAxis::SameEvent =>
            comparison.shared_event_candidate_refs.iter()
                .any(|v|v==evidence_ref),
        CorrespondenceAxis::Quotation | CorrespondenceAxis::SourceDependency =>
            comparison.native_join_refs.iter().any(|v|v==evidence_ref),
    };
    if !owned {return Err(CorrespondenceReviewError::UnownedEvidence);}

    let relation_ref=format!("candidate-correspondence:{}",hash_ref(
        "s29:source-correspondence:v1",
        &[left,right,axis.key(),evidence_ref,consumer_scope_ref],
    ));
    let item_ref=format!("review-item:{}",relation_ref);
    let item=ReviewItem {
        review_item_ref:item_ref.clone(),
        semantic_ref:relation_ref.clone(),
        item_kind:ReviewItemKind::SourceCorrespondence,
        reason:format!("Review candidate {} relationship; witness is not admission",
            axis.key()),
        provenance_refs:vec![evidence_ref.into()],
        source_refs:vec![left.into(),right.into()],
        current_status:ReviewStatus::Pending,
        available_actions:vec![
            ReviewAction::Accept,ReviewAction::Reject,ReviewAction::Abstain,
            ReviewAction::Qualify,ReviewAction::Supersede,
            ReviewAction::RequestEvidence,ReviewAction::OpenSource,
        ],
        affected_consumer_refs:vec![consumer_scope_ref.into()],
        candidate_only:true,creates_semantic_authority:false,
        applicability_promoted:false,claim_truth_promoted:false,
    };
    // Existing S29 item persistence handles canonical source/provenance refs.
    // On repeat requests, S29 retains its existing mutable workflow status.
    crate::install_review_workstation_schema(config)?;
    let mut client=Client::connect(config.database_url(),NoTls)?;
    client.batch_execute(CORRESPONDENCE_REVIEW_SQL)?;

    // Persist review state via S29 only. If sidecar creation fails a harmless
    // candidate review item can remain, but no fabricated relation is exposed.
    if let Some(existing)=crate::load_review_item(config,&item_ref)? {
        if existing.item_kind!=ReviewItemKind::SourceCorrespondence
            || existing.semantic_ref!=relation_ref
            || existing.provenance_refs!=item.provenance_refs
            || existing.source_refs!=item.source_refs
            || existing.affected_consumer_refs!=item.affected_consumer_refs
        {
            return Err(CorrespondenceReviewError::ImmutableRelationConflict);
        }
    } else {
        let _=crate::persist_review_item(config,&item)?;
    }
    client.execute(
        "INSERT INTO semantic.source_correspondence_review
         (relation_ref,review_item_ref,left_source_revision_ref,
          right_source_revision_ref,axis_ref,evidence_ref,consumer_scope_ref,
          candidate_only,creates_semantic_authority,claim_truth_promoted,
          independence_established)
         VALUES ($1,$2,$3,$4,$5,$6,$7,TRUE,FALSE,FALSE,FALSE)
         ON CONFLICT (relation_ref) DO NOTHING",
        &[&relation_ref,&item_ref,&left,&right,&axis.key(),&evidence_ref,
          &consumer_scope_ref],
    )?;
    load_correspondence_review(config,&relation_ref)?
        .ok_or(CorrespondenceReviewError::ImmutableRelationConflict)
}

/// Reopen exact immutable relation identity and its *current* S29 review item.
/// No heuristic or silent source merge participates in replay.
pub fn load_correspondence_review(
    config: &DatabaseConfig,
    relation_ref: &str,
) -> Result<Option<CorrespondenceReviewProposal>,CorrespondenceReviewError> {
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let Some(row)=client.query_opt(
        "SELECT review_item_ref,left_source_revision_ref,right_source_revision_ref,
                axis_ref,evidence_ref,consumer_scope_ref,candidate_only,
                creates_semantic_authority,claim_truth_promoted,
                independence_established
         FROM semantic.source_correspondence_review WHERE relation_ref=$1",
         &[&relation_ref],
    )? else {return Ok(None)};
    let item_ref: String=row.get(0);
    let left:String=row.get(1);
    let right:String=row.get(2);
    let axis_ref:String=row.get(3);
    let axis=CorrespondenceAxis::parse(&axis_ref)?;
    let evidence_ref:String=row.get(4);
    let consumer_scope_ref:String=row.get(5);
    let item=crate::load_review_item(config,&item_ref)?
        .ok_or(CorrespondenceReviewError::UnownedReviewItem)?;
    if item.item_kind!=ReviewItemKind::SourceCorrespondence
        || item.semantic_ref!=relation_ref
        || item.source_refs!=vec![left.clone(),right.clone()].into_iter()
            .collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>()
        || !item.provenance_refs.contains(&evidence_ref)
        || !item.affected_consumer_refs.contains(&consumer_scope_ref)
        || !item.candidate_only || item.creates_semantic_authority
        || item.applicability_promoted || item.claim_truth_promoted
        || !row.get::<_,bool>(6) || row.get::<_,bool>(7)
        || row.get::<_,bool>(8) || row.get::<_,bool>(9)
        || format!("candidate-correspondence:{}",hash_ref(
            "s29:source-correspondence:v1",
            &[&left,&right,axis.key(),&evidence_ref,&consumer_scope_ref],
        ))!=relation_ref
    {return Err(CorrespondenceReviewError::ImmutableRelationConflict);}
    Ok(Some(CorrespondenceReviewProposal {
        relation_ref:relation_ref.into(),review_item_ref:item_ref,
        left_source_revision_ref:left,right_source_revision_ref:right,
        axis,evidence_ref,consumer_scope_ref,review_status:item.current_status,
        creates_semantic_authority:false,claim_truth_promoted:false,
        independence_established:false,
    }))
}

/// Review this relation, not either source or underlying proposition. S29
/// performs locking, status reduction, receipt storage and replay. Even an
/// accepted review item says *the operator accepted that review proposal*,
/// never that both sources became one canonical source or independent witness.
pub fn apply_correspondence_review(
    config: &DatabaseConfig,
    relation_ref: &str,
    command: &ReviewCommand,
) -> Result<(ReviewReceipt,CorrespondenceReviewProposal),CorrespondenceReviewError> {
    let original=load_correspondence_review(config,relation_ref)?
        .ok_or(CorrespondenceReviewError::UnownedReviewItem)?;
    if command.review_item_ref!=original.review_item_ref {
        return Err(CorrespondenceReviewError::UnownedReviewItem);
    }
    let (receipt,item)=crate::apply_persisted_review_command(config,command)?;
    if receipt.creates_semantic_authority || receipt.applicability_promoted
        || receipt.claim_truth_promoted || !receipt.candidate_only
        || item.item_kind!=ReviewItemKind::SourceCorrespondence
    {return Err(CorrespondenceReviewError::ReviewPromotion);}
    let reopened=load_correspondence_review(config,relation_ref)?
        .ok_or(CorrespondenceReviewError::UnownedReviewItem)?;
    if reopened.review_status != item.current_status {
        return Err(CorrespondenceReviewError::ImmutableRelationConflict);
    }
    Ok((receipt,reopened))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relation_hash_separates_axis_witness_and_consumer_scope() {
        let one=hash_ref("s29:source-correspondence:v1",
            &["source:a","source:b","same_subject","candidate:1","scope:private"]);
        let two=hash_ref("s29:source-correspondence:v1",
            &["source:a","source:b","same_event","candidate:1","scope:private"]);
        let three=hash_ref("s29:source-correspondence:v1",
            &["source:a","source:b","same_subject","candidate:1","scope:professional"]);
        assert_ne!(one,two);
        assert_ne!(one,three);
    }
}
