//! WIKI-1 — source-pinned transport for an *externally executed* finite Lean
//! ontology checker. The transport does not run Lean, certify a proof, or
//! authorize a Wikidata edit. All diagnostic witnesses are producer-supplied.
//!
//! Original Lean development: JMD (github.com/meta-introspector), attributed in
//! dashi_lean4/DASHI/output-final_aristotle/RequestProject/{Diagnostics,
//! ConstraintSuite,RepairReview}.lean. This Rust bridge is DASHI integration.
//! A theorem about KB.valid and a checked concrete Wikidata graph are distinct.

use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewCommand, ReviewItem, ReviewItemKind, ReviewReceipt, ReviewStatus,
};
use thiserror::Error;

use crate::{DatabaseConfig, ReviewWorkstationStoreError};

pub const WIKI_DIAGNOSTIC_SCHEMA: &str = "itir.wikidata.ontology-diagnostic.v1";
const LEAN_DIAGNOSTICS: &str = "RequestProject.Diagnostics";
const LEAN_CONSTRAINTS: &str = "RequestProject.ConstraintSuite";
const LEAN_REPAIR: &str = "RequestProject.RepairReview";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum WikidataGraphView {
    FullStatements,
    TruthyRank,
    ScopedSlice,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum OntologyCheckerKind {
    FiniteKbDiagnostics,
    StatementConstraintSuite,
    AdvisoryRepairReview,
}
impl OntologyCheckerKind {
    fn lean_owner(&self) -> &'static str {
        match self {
            Self::FiniteKbDiagnostics=>LEAN_DIAGNOSTICS,
            Self::StatementConstraintSuite=>LEAN_CONSTRAINTS,
            Self::AdvisoryRepairReview=>LEAN_REPAIR,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum OntologyDiagnosticDisposition {
    Violation,
    Warning,
    NoViolationInCheckedSlice,
    Undetermined,
    Inapplicable,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OntologyWitness {
    /// An ID supplied by the checker for the precise finite-KB issue.
    pub witness_ref: String,
    /// Exact stable source-statement GUIDs or bounded native identifiers;
    /// never substitute an inferred item-level equality for statement refs.
    pub statement_refs: Vec<String>,
    pub rule_ref: String,
    pub explanation: String,
    pub evidence_refs: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OntologyRepairCandidate {
    pub candidate_ref: String,
    pub proposed_edit_description: String,
    pub rationale: String,
    pub modeled_verdict_ref: String,
    pub modeled_before_ref: String,
    pub modeled_after_ref: String,
    pub changes_wikidata: bool,
    pub grants_edit_authority: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OntologyDiagnosticPacket {
    pub schema: String,
    pub source_revision_ref: String,
    pub source_snapshot_digest_ref: String,
    pub wikidata_entity_ref: String,
    pub graph_view: WikidataGraphView,
    /// For scoped slices, the explicit query/selection identity.
    pub graph_slice_ref: Option<String>,
    pub checker_kind: OntologyCheckerKind,
    pub lean_owner_ref: String,
    /// Exact dashi_lean4 commit for the checker implementation.
    pub lean_source_commit: String,
    pub producer_run_ref: String,
    pub producer_receipt_ref: String,
    pub producer_output_digest_ref: String,
    /// The source of *execution*, not a mere citation to a Lean theorem.
    pub executed_checker: bool,
    /// True ONLY with separately provided real Lean kernel receipt.
    pub lean_kernel_checked: bool,
    pub lean_kernel_receipt_ref: Option<String>,
    pub disposition: OntologyDiagnosticDisposition,
    pub witnesses: Vec<OntologyWitness>,
    pub missing_obligations: Vec<String>,
    pub repair_candidates: Vec<OntologyRepairCandidate>,
    pub original_author_ref: String,
    pub integration_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
    pub grants_wikidata_edit_authority: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyDiagnosticRead {
    pub diagnostic_ref: String,
    pub review_item_ref: String,
    pub consumer_scope_ref: String,
    pub packet: OntologyDiagnosticPacket,
    pub s29_status: ReviewStatus,
    pub creates_semantic_authority: bool,
    pub grants_wikidata_edit_authority: bool,
}

#[derive(Debug, Error)]
pub enum OntologyReviewError {
    #[error(transparent)]
    Pg(#[from] postgres::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    S29(#[from] ReviewWorkstationStoreError),
    #[error(transparent)]
    SourceStore(#[from] crate::GenericSourceContentStoreError),
    #[error("invalid ontology diagnostic producer contract")]
    InvalidPacket,
    #[error("source snapshot is absent or its bytes do not match")]
    SnapshotMismatch,
    #[error("persisted ontology diagnostic or review identity conflicts")]
    IdentityConflict,
    #[error("no ontology diagnostic was recorded")]
    UnknownDiagnostic,
}

pub const WIKI_DIAGNOSTIC_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.wikidata_ontology_diagnostic (
    diagnostic_ref TEXT PRIMARY KEY,
    review_item_ref TEXT NOT NULL UNIQUE REFERENCES semantic.review_item(review_item_ref),
    source_revision_ref TEXT NOT NULL,
    consumer_scope_ref TEXT NOT NULL,
    source_snapshot_digest_ref TEXT NOT NULL,
    graph_view_ref TEXT NOT NULL CHECK (graph_view_ref IN
      ('full_statements','truthy_rank','scoped_slice')),
    checker_owner_ref TEXT NOT NULL,
    producer_run_ref TEXT NOT NULL,
    producer_receipt_ref TEXT NOT NULL,
    packet_digest_ref TEXT NOT NULL,
    packet_json TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK(candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK(NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK(NOT claim_truth_promoted),
    grants_edit_authority BOOLEAN NOT NULL CHECK(NOT grants_edit_authority)
);
CREATE INDEX IF NOT EXISTS ontology_diagnostic_source_scope_idx
 ON semantic.wikidata_ontology_diagnostic
 (source_revision_ref, consumer_scope_ref);
"#;

fn digest(raw: &[u8]) -> String {
    format!("sha256:{:x}",Sha256::digest(raw))
}
fn digest_coords(parts: &[&str]) -> String {
    let mut h=Sha256::new();
    for part in parts {
        h.update((part.len() as u64).to_be_bytes());
        h.update(part.as_bytes());
    }
    format!("ontology-diagnostic:sha256:{:x}",h.finalize())
}
fn valid_ref(value:&str)->bool { !value.trim().is_empty() }
fn unique_nonempty(values:&[String])->bool {
    use std::collections::BTreeSet;
    values.iter().all(|s|valid_ref(s)) &&
        values.iter().collect::<BTreeSet<_>>().len()==values.len()
}
fn graph_tag(view:&WikidataGraphView)->&'static str {
    match view {
        WikidataGraphView::FullStatements=>"full_statements",
        WikidataGraphView::TruthyRank=>"truthy_rank",
        WikidataGraphView::ScopedSlice=>"scoped_slice",
    }
}
/// Fail closed on modeled-vs-executed confusion and claim promotion.
/// No violation reported is bounded to the declared snapshot and graph view.
pub fn validate_ontology_packet(p:&OntologyDiagnosticPacket)
    -> Result<(),OntologyReviewError> {
    if p.schema!=WIKI_DIAGNOSTIC_SCHEMA
        || p.lean_owner_ref!=p.checker_kind.lean_owner()
        || p.original_author_ref!="JMD (github.com/meta-introspector)"
        || !valid_ref(&p.integration_ref)
        || !valid_ref(&p.source_revision_ref)
        || !valid_ref(&p.wikidata_entity_ref)
        || !valid_ref(&p.source_snapshot_digest_ref)
        || !valid_ref(&p.lean_source_commit)
        || !valid_ref(&p.producer_run_ref)
        || !valid_ref(&p.producer_receipt_ref)
        || !valid_ref(&p.producer_output_digest_ref)
        || !p.executed_checker
        || (p.lean_kernel_checked != p.lean_kernel_receipt_ref.is_some())
        || p.lean_kernel_receipt_ref.as_deref().is_some_and(|s|!valid_ref(s))
        || !p.candidate_only || p.creates_semantic_authority
        || p.claim_truth_promoted || p.grants_wikidata_edit_authority
        || (matches!(p.graph_view,WikidataGraphView::ScopedSlice)
            != p.graph_slice_ref.as_deref().is_some_and(valid_ref))
        || !unique_nonempty(&p.missing_obligations)
    { return Err(OntologyReviewError::InvalidPacket); }
    let mut seen=std::collections::BTreeSet::new();
    for w in &p.witnesses {
        if !valid_ref(&w.witness_ref)||!seen.insert(&w.witness_ref)
            || !valid_ref(&w.rule_ref)||!valid_ref(&w.explanation)
            || !unique_nonempty(&w.statement_refs)
            || w.statement_refs.is_empty()
            || !unique_nonempty(&w.evidence_refs)
        {return Err(OntologyReviewError::InvalidPacket);}
    }
    let mut repairs=std::collections::BTreeSet::new();
    for r in &p.repair_candidates {
        if !valid_ref(&r.candidate_ref)||!repairs.insert(&r.candidate_ref)
            || !valid_ref(&r.rationale)||!valid_ref(&r.proposed_edit_description)
            || !valid_ref(&r.modeled_verdict_ref)
            || !valid_ref(&r.modeled_before_ref)
            || !valid_ref(&r.modeled_after_ref)
            || r.changes_wikidata||r.grants_edit_authority
        {return Err(OntologyReviewError::InvalidPacket);}
    }
    if matches!(p.disposition,OntologyDiagnosticDisposition::Violation
        | OntologyDiagnosticDisposition::Warning) && p.witnesses.is_empty() {
        return Err(OntologyReviewError::InvalidPacket);
    }
    if matches!(p.disposition,OntologyDiagnosticDisposition::Undetermined)
        && p.missing_obligations.is_empty() {
        return Err(OntologyReviewError::InvalidPacket);
    }
    Ok(())
}
fn refs_for_review(p:&OntologyDiagnosticPacket)->Vec<String> {
    let mut refs=vec![p.source_revision_ref.clone()];
    for w in &p.witnesses {
        refs.extend(w.statement_refs.iter().cloned());
    }
    refs.sort();
    refs.dedup();
    refs
}
pub fn persist_ontology_diagnostic(
    config:&DatabaseConfig,
    p:&OntologyDiagnosticPacket,
    consumer_scope_ref:&str,
)->Result<OntologyDiagnosticRead,OntologyReviewError> {
    validate_ontology_packet(p)?;
    if !valid_ref(consumer_scope_ref) {return Err(OntologyReviewError::InvalidPacket);}
    // An ontology diagnostic may not bootstrap or change a source. Reopen
    // native canonical content and require the packet's pinned digest.
    let (source,_)=crate::load_generic_text_source(config,&p.source_revision_ref)?;
    if source.content_digest_ref!=p.source_snapshot_digest_ref {
        return Err(OntologyReviewError::SnapshotMismatch);
    }
    let canonical=serde_json::to_string(p)?;
    let packet_digest=digest(canonical.as_bytes());
    let diagnostic_ref=digest_coords(&[
        "wiki-1-ontology-diagnostic-v1",&p.source_revision_ref,
        &p.source_snapshot_digest_ref,graph_tag(&p.graph_view),
        p.graph_slice_ref.as_deref().unwrap_or(""),
        &p.lean_owner_ref,&p.lean_source_commit,&p.producer_run_ref,
        &p.producer_output_digest_ref,consumer_scope_ref,
    ]);
    let item_ref=format!("review-item:{diagnostic_ref}");
    let item=ReviewItem {
        review_item_ref:item_ref.clone(),
        semantic_ref:diagnostic_ref.clone(),
        item_kind:ReviewItemKind::OntologyDiagnostic,
        reason:format!(
            "Review finite Wikidata ontology {:?} on {:?} (not an edit authorization)",
            p.disposition,p.graph_view),
        provenance_refs:vec![p.producer_receipt_ref.clone(),
            p.source_snapshot_digest_ref.clone(),p.lean_source_commit.clone()],
        source_refs:refs_for_review(p),
        current_status:ReviewStatus::Pending,
        available_actions:vec![
            ReviewAction::Accept,ReviewAction::Reject,ReviewAction::Abstain,
            ReviewAction::Qualify,ReviewAction::RequestEvidence,
            ReviewAction::Supersede,ReviewAction::OpenSource,
        ],
        affected_consumer_refs:vec![consumer_scope_ref.into()],
        candidate_only:true,creates_semantic_authority:false,
        applicability_promoted:false,claim_truth_promoted:false,
    };
    crate::install_review_workstation_schema(config)?;
    let mut client=Client::connect(config.database_url(),NoTls)?;
    client.batch_execute(WIKI_DIAGNOSTIC_SQL)?;
    if let Some(existing)=crate::load_review_item(config,&item_ref)? {
        if existing.semantic_ref!=item.semantic_ref
            || existing.item_kind!=ReviewItemKind::OntologyDiagnostic
            || existing.source_refs!=item.source_refs
            || existing.provenance_refs!=item.provenance_refs
            || existing.affected_consumer_refs!=item.affected_consumer_refs
        {return Err(OntologyReviewError::IdentityConflict);}
    } else {
        let _=crate::persist_review_item(config,&item)?;
    }
    client.execute(
        "INSERT INTO semantic.wikidata_ontology_diagnostic
         (diagnostic_ref,review_item_ref,source_revision_ref,
          consumer_scope_ref,source_snapshot_digest_ref,graph_view_ref,
          checker_owner_ref,producer_run_ref,producer_receipt_ref,
          packet_digest_ref,packet_json,candidate_only,
          creates_semantic_authority,claim_truth_promoted,grants_edit_authority)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,TRUE,FALSE,FALSE,FALSE)
         ON CONFLICT (diagnostic_ref) DO NOTHING",
        &[&diagnostic_ref,&item_ref,&p.source_revision_ref,
          &consumer_scope_ref,&p.source_snapshot_digest_ref,
          &graph_tag(&p.graph_view),&p.lean_owner_ref,&p.producer_run_ref,
          &p.producer_receipt_ref,&packet_digest,&canonical],
    )?;
    load_ontology_diagnostic(config,&diagnostic_ref)?
        .ok_or(OntologyReviewError::IdentityConflict)
}
/// The ontology issue uses the ordinary S29 transactional review reducer:
/// this never changes a Wikidata statement or treats an advisory repair as
/// approved. Receipt and status are reopened from S29 after the command.
pub fn apply_ontology_diagnostic_review(
    config:&DatabaseConfig, diagnostic_ref:&str, command:&ReviewCommand,
)->Result<(ReviewReceipt,OntologyDiagnosticRead),OntologyReviewError> {
    let before=load_ontology_diagnostic(config,diagnostic_ref)?
        .ok_or(OntologyReviewError::UnknownDiagnostic)?;
    if command.review_item_ref!=before.review_item_ref {
        return Err(OntologyReviewError::IdentityConflict);
    }
    let (receipt,item)=crate::apply_persisted_review_command(config,command)?;
    if !receipt.candidate_only||receipt.creates_semantic_authority
        || receipt.applicability_promoted||receipt.claim_truth_promoted
        || item.item_kind!=ReviewItemKind::OntologyDiagnostic
    {return Err(OntologyReviewError::IdentityConflict);}
    let after=load_ontology_diagnostic(config,diagnostic_ref)?
        .ok_or(OntologyReviewError::UnknownDiagnostic)?;
    if after.s29_status!=item.current_status
        || after.packet!=before.packet
        || after.consumer_scope_ref!=before.consumer_scope_ref
    {return Err(OntologyReviewError::IdentityConflict);}
    Ok((receipt,after))
}
pub fn load_ontology_diagnostic(
    config:&DatabaseConfig, diagnostic_ref:&str,
)->Result<Option<OntologyDiagnosticRead>,OntologyReviewError> {
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let Some(row)=client.query_opt(
        "SELECT review_item_ref,source_revision_ref,consumer_scope_ref,
                source_snapshot_digest_ref,graph_view_ref,checker_owner_ref,
                producer_run_ref,producer_receipt_ref,packet_digest_ref,
                packet_json,candidate_only,creates_semantic_authority,
                claim_truth_promoted,grants_edit_authority
         FROM semantic.wikidata_ontology_diagnostic WHERE diagnostic_ref=$1",
        &[&diagnostic_ref],
    )? else {return Ok(None)};
    let json:String=row.get(9);
    if digest(json.as_bytes())!=row.get::<_,String>(8)
        || !row.get::<_,bool>(10)||row.get::<_,bool>(11)
        || row.get::<_,bool>(12)||row.get::<_,bool>(13)
    {return Err(OntologyReviewError::IdentityConflict);}
    let packet:OntologyDiagnosticPacket=serde_json::from_str(&json)?;
    validate_ontology_packet(&packet)?;
    let scope:String=row.get(2);
    let expected=digest_coords(&[
        "wiki-1-ontology-diagnostic-v1",&packet.source_revision_ref,
        &packet.source_snapshot_digest_ref,graph_tag(&packet.graph_view),
        packet.graph_slice_ref.as_deref().unwrap_or(""),
        &packet.lean_owner_ref,&packet.lean_source_commit,
        &packet.producer_run_ref,&packet.producer_output_digest_ref,&scope,
    ]);
    let item_ref:String=row.get(0);
    let review=crate::load_review_item(config,&item_ref)?
        .ok_or(OntologyReviewError::IdentityConflict)?;
    if diagnostic_ref!=expected
        || item_ref!=format!("review-item:{diagnostic_ref}")
        || review.item_kind!=ReviewItemKind::OntologyDiagnostic
        || review.semantic_ref!=diagnostic_ref
        || review.source_refs!=refs_for_review(&packet)
        || review.affected_consumer_refs!=vec![scope.clone()]
        || review.provenance_refs!={
            let mut v=vec![packet.producer_receipt_ref.clone(),
                packet.source_snapshot_digest_ref.clone(),packet.lean_source_commit.clone()];
            v.sort();v.dedup();v
        }
        || row.get::<_,String>(1)!=packet.source_revision_ref
        || row.get::<_,String>(3)!=packet.source_snapshot_digest_ref
        || row.get::<_,String>(4)!=graph_tag(&packet.graph_view)
        || row.get::<_,String>(5)!=packet.lean_owner_ref
        || row.get::<_,String>(6)!=packet.producer_run_ref
        || row.get::<_,String>(7)!=packet.producer_receipt_ref
    {return Err(OntologyReviewError::IdentityConflict);}
    let (source,_)=crate::load_generic_text_source(config,&packet.source_revision_ref)?;
    if source.content_digest_ref!=packet.source_snapshot_digest_ref {
        return Err(OntologyReviewError::SnapshotMismatch);
    }
    Ok(Some(OntologyDiagnosticRead {
        diagnostic_ref:diagnostic_ref.into(),review_item_ref:item_ref,
        consumer_scope_ref:scope,packet,s29_status:review.current_status,
        creates_semantic_authority:false,grants_wikidata_edit_authority:false,
    }))
}
