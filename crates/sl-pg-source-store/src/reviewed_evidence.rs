//! Generic durable reviewed-evidence spine.
//!
//! `observation + requirement + explicit review -> reviewed evidence`.
//! Neither source retrieval nor PNF similarity may choose an evidentiary role.

use postgres::{Client, NoTls, Row, Transaction};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{DatabaseConfig, MaterializedLegalIrRefs, ReviewedPropositionSupport};

pub const REVIEWED_EVIDENCE_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.reviewed_evidence_coordinate (
  reviewed_evidence_ref TEXT PRIMARY KEY,
  review_receipt_ref TEXT NOT NULL REFERENCES semantic.review_receipt(command_ref),
  review_item_ref TEXT NOT NULL REFERENCES semantic.review_item(review_item_ref),
  reviewer_ref TEXT NOT NULL,
  observation_ref TEXT NOT NULL,
  consumer_ref TEXT NOT NULL,
  requirement_ref TEXT NOT NULL,
  evidence_role_ref TEXT NOT NULL,
  normative_order_ref TEXT NOT NULL CHECK (length(btrim(normative_order_ref)) > 0),
  proposition_ref TEXT NOT NULL,
  source_revision_ref TEXT NOT NULL,
  document_ref TEXT NOT NULL,
  statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref),
  exact_span_ref TEXT NOT NULL,
  candidate_pnf_batch_ref TEXT NOT NULL REFERENCES pnf.statement_candidate_batch(batch_ref),
  candidate_factor_ref TEXT NOT NULL,
  parser_build_ref TEXT NOT NULL,
  pnf_build_ref TEXT NOT NULL,
  refined_pnf_graph_ref TEXT NOT NULL,
  pnf_factor_ref TEXT NOT NULL,
  pnf_revision_ref TEXT NOT NULL,
  structural_signature_ref TEXT NOT NULL,
  predicate_ref TEXT NOT NULL,
  observation_provenance_refs TEXT[] NOT NULL,
  observation_residual_refs TEXT[] NOT NULL,
  legal_system_refs TEXT[] NOT NULL,
  jurisdiction_refs TEXT[] NOT NULL,
  temporal_refs TEXT[] NOT NULL,
  author_ref TEXT NOT NULL,
  institution_ref TEXT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  coordinate_sha256 BYTEA NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS reviewed_evidence_review_requirement_idx
ON semantic.reviewed_evidence_coordinate
(review_receipt_ref, consumer_ref, requirement_ref, evidence_role_ref,
 normative_order_ref, proposition_ref, source_revision_ref, exact_span_ref,
 pnf_revision_ref);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedEvidenceDraft {
    pub review_receipt_ref: String,
    pub observation_ref: String,
    pub consumer_ref: String,
    pub requirement_ref: String,
    pub evidence_role_ref: String,
    pub normative_order_ref: String,
    pub proposition_ref: String,
    pub source_revision_ref: String,
    pub document_ref: String,
    pub statement_ref: String,
    pub exact_span_ref: String,
    pub candidate_pnf_batch_ref: String,
    pub candidate_factor_ref: String,
    pub parser_build_ref: String,
    pub pnf_build_ref: String,
    pub refined_pnf_graph_ref: String,
    pub pnf_factor_ref: String,
    pub pnf_revision_ref: String,
    pub structural_signature_ref: String,
    pub predicate_ref: String,
    pub observation_provenance_refs: Vec<String>,
    pub observation_residual_refs: Vec<String>,
    pub legal_system_refs: Vec<String>,
    pub jurisdiction_refs: Vec<String>,
    pub temporal_refs: Vec<String>,
    pub author_ref: String,
    pub institution_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedReviewedEvidenceCoordinate {
    pub reviewed_evidence_ref: String,
    pub review_receipt_ref: String,
    pub review_item_ref: String,
    pub reviewer_ref: String,
    pub observation_ref: String,
    pub consumer_ref: String,
    pub requirement_ref: String,
    pub evidence_role_ref: String,
    pub normative_order_ref: String,
    pub proposition_ref: String,
    pub source_revision_ref: String,
    pub document_ref: String,
    pub statement_ref: String,
    pub exact_span_ref: String,
    pub candidate_pnf_batch_ref: String,
    pub candidate_factor_ref: String,
    pub parser_build_ref: String,
    pub pnf_build_ref: String,
    pub refined_pnf_graph_ref: String,
    pub pnf_factor_ref: String,
    pub pnf_revision_ref: String,
    pub structural_signature_ref: String,
    pub predicate_ref: String,
    pub observation_provenance_refs: Vec<String>,
    pub observation_residual_refs: Vec<String>,
    pub legal_system_refs: Vec<String>,
    pub jurisdiction_refs: Vec<String>,
    pub temporal_refs: Vec<String>,
    pub author_ref: String,
    pub institution_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl PersistedReviewedEvidenceCoordinate {
    pub fn as_legal_ir_support(&self) -> ReviewedPropositionSupport {
        ReviewedPropositionSupport {
            proposition_ref: self.proposition_ref.clone(),
            source_revision_ref: self.source_revision_ref.clone(),
            document_ref: self.document_ref.clone(),
            exact_span_ref: self.exact_span_ref.clone(),
            parser_build_ref: self.parser_build_ref.clone(),
            pnf_build_ref: self.pnf_build_ref.clone(),
            refined_pnf_graph_ref: self.refined_pnf_graph_ref.clone(),
            pnf_factor_ref: self.pnf_factor_ref.clone(),
            pnf_revision_ref: self.pnf_revision_ref.clone(),
            structural_signature_ref: self.structural_signature_ref.clone(),
            predicate_ref: self.predicate_ref.clone(),
            observation_provenance_refs: self.observation_provenance_refs.clone(),
            observation_residual_refs: self.observation_residual_refs.clone(),
            legal_system_refs: self.legal_system_refs.clone(),
            jurisdiction_refs: self.jurisdiction_refs.clone(),
            temporal_refs: self.temporal_refs.clone(),
            author_ref: self.author_ref.clone(),
            institution_ref: self.institution_ref.clone(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ReviewedEvidenceStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("upstream persistence prerequisite failed: {0}")]
    Upstream(String),
    #[error("required reviewed-evidence coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("review receipt is not an accepted/qualified review of the declared observation")]
    ReviewReceiptMismatch,
    #[error("candidate batch/statement/span/factor coordinates do not reopen exactly")]
    CandidateCoordinateMismatch,
    #[error("source revision/document/exact-span coordinates do not reopen exactly")]
    SourceCoordinateMismatch,
    #[error("reviewed PNF factor/revision/graph coordinates do not reopen exactly")]
    ReviewedPnfCoordinateMismatch,
    #[error("observation provenance does not contain the exact source span")]
    ObservationMissingExactSpan,
    #[error("persisted reviewed-evidence row conflicts with requested immutable coordinates")]
    ExistingRowConflict,
    #[error("reviewed-evidence coordinate not found: {0}")]
    NotFound(String),
    #[error("legal IR materialisation failed: {0}")]
    LegalIr(#[from] crate::LegalIrMaterializationError),
}

pub fn install_reviewed_evidence_schema(config: &DatabaseConfig) -> Result<(), ReviewedEvidenceStoreError> {
    crate::install_review_workstation_schema(config)
        .map_err(|error| ReviewedEvidenceStoreError::Upstream(error.to_string()))?;
    crate::install_candidate_pnf_schema(config)
        .map_err(|error| ReviewedEvidenceStoreError::Upstream(error.to_string()))?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(REVIEWED_EVIDENCE_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_reviewed_evidence_coordinate(
    config: &DatabaseConfig,
    draft: &ReviewedEvidenceDraft,
) -> Result<PersistedReviewedEvidenceCoordinate, ReviewedEvidenceStoreError> {
    validate_draft(draft)?;
    install_reviewed_evidence_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    let (review_item_ref, reviewer_ref) = require_review(&mut tx, draft)?;
    require_candidate(&mut tx, draft)?;
    require_source(&mut tx, draft)?;
    require_reviewed_pnf(&mut tx, draft)?;

    let reviewed_evidence_ref = stable_ref(&[
        "slr-reviewed-evidence:v1", &draft.review_receipt_ref, &draft.observation_ref,
        &draft.consumer_ref, &draft.requirement_ref, &draft.evidence_role_ref,
        &draft.normative_order_ref, &draft.proposition_ref, &draft.source_revision_ref,
        &draft.exact_span_ref, &draft.candidate_pnf_batch_ref, &draft.pnf_revision_ref,
    ]);
    let digest = digest(&[
        &reviewed_evidence_ref, &review_item_ref, &reviewer_ref, &draft.document_ref,
        &draft.statement_ref, &draft.candidate_factor_ref, &draft.refined_pnf_graph_ref,
    ]);
    tx.execute(
        r#"INSERT INTO semantic.reviewed_evidence_coordinate
        (reviewed_evidence_ref,review_receipt_ref,review_item_ref,reviewer_ref,observation_ref,
         consumer_ref,requirement_ref,evidence_role_ref,normative_order_ref,proposition_ref,
         source_revision_ref,document_ref,statement_ref,exact_span_ref,candidate_pnf_batch_ref,
         candidate_factor_ref,parser_build_ref,pnf_build_ref,refined_pnf_graph_ref,pnf_factor_ref,
         pnf_revision_ref,structural_signature_ref,predicate_ref,observation_provenance_refs,
         observation_residual_refs,legal_system_refs,jurisdiction_refs,temporal_refs,author_ref,
         institution_ref,candidate_only,creates_semantic_authority,applicability_promoted,
         claim_truth_promoted,coordinate_sha256)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
                $21,$22,$23,$24,$25,$26,$27,$28,$29,$30,true,false,false,false,$31)
        ON CONFLICT (reviewed_evidence_ref) DO NOTHING"#,
        &[&reviewed_evidence_ref,&draft.review_receipt_ref,&review_item_ref,&reviewer_ref,
          &draft.observation_ref,&draft.consumer_ref,&draft.requirement_ref,&draft.evidence_role_ref,
          &draft.normative_order_ref,&draft.proposition_ref,&draft.source_revision_ref,&draft.document_ref,
          &draft.statement_ref,&draft.exact_span_ref,&draft.candidate_pnf_batch_ref,&draft.candidate_factor_ref,
          &draft.parser_build_ref,&draft.pnf_build_ref,&draft.refined_pnf_graph_ref,&draft.pnf_factor_ref,
          &draft.pnf_revision_ref,&draft.structural_signature_ref,&draft.predicate_ref,
          &draft.observation_provenance_refs,&draft.observation_residual_refs,&draft.legal_system_refs,
          &draft.jurisdiction_refs,&draft.temporal_refs,&draft.author_ref,&draft.institution_ref,&&digest[..]],
    )?;
    let loaded = load_with_tx(&mut tx, &reviewed_evidence_ref)?;
    if loaded.review_receipt_ref != draft.review_receipt_ref
        || loaded.observation_ref != draft.observation_ref
        || loaded.consumer_ref != draft.consumer_ref
        || loaded.requirement_ref != draft.requirement_ref
        || loaded.evidence_role_ref != draft.evidence_role_ref
        || loaded.normative_order_ref != draft.normative_order_ref
        || loaded.proposition_ref != draft.proposition_ref
        || loaded.source_revision_ref != draft.source_revision_ref
        || loaded.document_ref != draft.document_ref
        || loaded.statement_ref != draft.statement_ref
        || loaded.exact_span_ref != draft.exact_span_ref
        || loaded.candidate_pnf_batch_ref != draft.candidate_pnf_batch_ref
        || loaded.candidate_factor_ref != draft.candidate_factor_ref
        || loaded.pnf_revision_ref != draft.pnf_revision_ref
        || !loaded.candidate_only || loaded.creates_semantic_authority
        || loaded.applicability_promoted || loaded.claim_truth_promoted
    { return Err(ReviewedEvidenceStoreError::ExistingRowConflict); }
    tx.commit()?;
    Ok(loaded)
}

pub fn load_reviewed_evidence_coordinate(
    config: &DatabaseConfig,
    reviewed_evidence_ref: &str,
) -> Result<PersistedReviewedEvidenceCoordinate, ReviewedEvidenceStoreError> {
    required("reviewed_evidence_ref", reviewed_evidence_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    let value = load_with_tx(&mut tx, reviewed_evidence_ref)?;
    tx.commit()?;
    Ok(value)
}

/// Normal production legal-IR entrypoint: the caller supplies a durable review
/// reference, never an ad-hoc support packet.
pub fn materialize_legal_ir_from_reviewed_evidence(
    config: &DatabaseConfig,
    reviewed_evidence_ref: &str,
) -> Result<MaterializedLegalIrRefs, ReviewedEvidenceStoreError> {
    let reviewed = load_reviewed_evidence_coordinate(config, reviewed_evidence_ref)?;
    Ok(crate::materialize_reviewed_proposition_support(config, &reviewed.as_legal_ir_support())?)
}

fn validate_draft(d: &ReviewedEvidenceDraft) -> Result<(), ReviewedEvidenceStoreError> {
    for (name,value) in [
        ("review_receipt_ref",d.review_receipt_ref.as_str()),("observation_ref",d.observation_ref.as_str()),
        ("consumer_ref",d.consumer_ref.as_str()),("requirement_ref",d.requirement_ref.as_str()),
        ("evidence_role_ref",d.evidence_role_ref.as_str()),("normative_order_ref",d.normative_order_ref.as_str()),
        ("proposition_ref",d.proposition_ref.as_str()),("source_revision_ref",d.source_revision_ref.as_str()),
        ("document_ref",d.document_ref.as_str()),("statement_ref",d.statement_ref.as_str()),
        ("exact_span_ref",d.exact_span_ref.as_str()),("candidate_pnf_batch_ref",d.candidate_pnf_batch_ref.as_str()),
        ("candidate_factor_ref",d.candidate_factor_ref.as_str()),("parser_build_ref",d.parser_build_ref.as_str()),
        ("pnf_build_ref",d.pnf_build_ref.as_str()),("refined_pnf_graph_ref",d.refined_pnf_graph_ref.as_str()),
        ("pnf_factor_ref",d.pnf_factor_ref.as_str()),("pnf_revision_ref",d.pnf_revision_ref.as_str()),
        ("structural_signature_ref",d.structural_signature_ref.as_str()),("predicate_ref",d.predicate_ref.as_str()),
        ("author_ref",d.author_ref.as_str()),
    ] { required(name,value)?; }
    if !d.observation_provenance_refs.iter().any(|value| value == &d.exact_span_ref) {
        return Err(ReviewedEvidenceStoreError::ObservationMissingExactSpan);
    }
    Ok(())
}
fn required(name:&'static str,value:&str)->Result<(),ReviewedEvidenceStoreError>{
    if value.trim().is_empty(){Err(ReviewedEvidenceStoreError::EmptyCoordinate(name))}else{Ok(())}
}

fn require_review(tx:&mut Transaction<'_>,d:&ReviewedEvidenceDraft)->Result<(String,String),ReviewedEvidenceStoreError>{
    let row=tx.query_opt(r#"SELECT review_item_ref,reviewer_ref,semantic_ref,action_ref,candidate_only,
      creates_semantic_authority,applicability_promoted,claim_truth_promoted
      FROM semantic.review_receipt WHERE command_ref=$1"#,&[&d.review_receipt_ref])?
      .ok_or(ReviewedEvidenceStoreError::ReviewReceiptMismatch)?;
    let action:String=row.get(3);
    if row.get::<_,String>(2)!=d.observation_ref || !matches!(action.as_str(),"accept"|"qualify")
      || !row.get::<_,bool>(4)||row.get::<_,bool>(5)||row.get::<_,bool>(6)||row.get::<_,bool>(7){
      return Err(ReviewedEvidenceStoreError::ReviewReceiptMismatch);
    }
    Ok((row.get(0),row.get(1)))
}
fn require_candidate(tx:&mut Transaction<'_>,d:&ReviewedEvidenceDraft)->Result<(),ReviewedEvidenceStoreError>{
    let ok:bool=tx.query_one(r#"SELECT EXISTS(SELECT 1 FROM pnf.statement_candidate_batch b
      JOIN pnf.statement_candidate_factor f ON f.batch_ref=b.batch_ref AND f.statement_ref=b.statement_ref
      WHERE b.batch_ref=$1 AND b.statement_ref=$2 AND b.exact_span_ref=$3 AND f.candidate_ref=$4
      AND b.candidate_only AND NOT b.semantic_admission_paid AND NOT b.proposition_support_paid
      AND NOT b.applicability_paid AND NOT b.claim_truth_paid AND f.candidate_only)"#,
      &[&d.candidate_pnf_batch_ref,&d.statement_ref,&d.exact_span_ref,&d.candidate_factor_ref])?.get(0);
    if ok{Ok(())}else{Err(ReviewedEvidenceStoreError::CandidateCoordinateMismatch)}
}
fn require_source(tx:&mut Transaction<'_>,d:&ReviewedEvidenceDraft)->Result<(),ReviewedEvidenceStoreError>{
    let ok:bool=tx.query_one(r#"SELECT EXISTS(SELECT 1 FROM legal_source_revision l
      JOIN corpus.span s ON s.document_ref=l.document_ref AND s.span_ref=$3
      WHERE l.source_revision_ref=$1 AND l.document_ref=$2 AND l.compile_eligible=TRUE
      AND s.start_char<s.end_char)"#,&[&d.source_revision_ref,&d.document_ref,&d.exact_span_ref])?.get(0);
    if ok{Ok(())}else{Err(ReviewedEvidenceStoreError::SourceCoordinateMismatch)}
}
fn require_reviewed_pnf(tx:&mut Transaction<'_>,d:&ReviewedEvidenceDraft)->Result<(),ReviewedEvidenceStoreError>{
    let ok:bool=tx.query_one(r#"SELECT EXISTS(SELECT 1 FROM algebra.factor_revision fr
      JOIN pnf.graph_factor_revision gfr ON gfr.factor_revision_ref=fr.factor_revision_ref
      JOIN pnf.graph g ON g.graph_ref=gfr.graph_ref
      WHERE fr.factor_revision_ref=$1 AND fr.factor_ref=$2 AND g.graph_ref=$3 AND g.document_ref=$4)"#,
      &[&d.pnf_revision_ref,&d.pnf_factor_ref,&d.refined_pnf_graph_ref,&d.document_ref])?.get(0);
    if ok{Ok(())}else{Err(ReviewedEvidenceStoreError::ReviewedPnfCoordinateMismatch)}
}

fn load_with_tx(tx:&mut Transaction<'_>,reference:&str)->Result<PersistedReviewedEvidenceCoordinate,ReviewedEvidenceStoreError>{
    let row=tx.query_opt(r#"SELECT reviewed_evidence_ref,review_receipt_ref,review_item_ref,reviewer_ref,
      observation_ref,consumer_ref,requirement_ref,evidence_role_ref,normative_order_ref,proposition_ref,
      source_revision_ref,document_ref,statement_ref,exact_span_ref,candidate_pnf_batch_ref,candidate_factor_ref,
      parser_build_ref,pnf_build_ref,refined_pnf_graph_ref,pnf_factor_ref,pnf_revision_ref,structural_signature_ref,
      predicate_ref,observation_provenance_refs,observation_residual_refs,legal_system_refs,jurisdiction_refs,
      temporal_refs,author_ref,institution_ref,candidate_only,creates_semantic_authority,applicability_promoted,
      claim_truth_promoted FROM semantic.reviewed_evidence_coordinate WHERE reviewed_evidence_ref=$1"#,&[&reference])?
      .ok_or_else(||ReviewedEvidenceStoreError::NotFound(reference.to_owned()))?;
    Ok(from_row(&row))
}
fn from_row(r:&Row)->PersistedReviewedEvidenceCoordinate{PersistedReviewedEvidenceCoordinate{
    reviewed_evidence_ref:r.get(0),review_receipt_ref:r.get(1),review_item_ref:r.get(2),reviewer_ref:r.get(3),
    observation_ref:r.get(4),consumer_ref:r.get(5),requirement_ref:r.get(6),evidence_role_ref:r.get(7),normative_order_ref:r.get(8),
    proposition_ref:r.get(9),source_revision_ref:r.get(10),document_ref:r.get(11),statement_ref:r.get(12),exact_span_ref:r.get(13),
    candidate_pnf_batch_ref:r.get(14),candidate_factor_ref:r.get(15),parser_build_ref:r.get(16),pnf_build_ref:r.get(17),
    refined_pnf_graph_ref:r.get(18),pnf_factor_ref:r.get(19),pnf_revision_ref:r.get(20),structural_signature_ref:r.get(21),predicate_ref:r.get(22),
    observation_provenance_refs:r.get(23),observation_residual_refs:r.get(24),legal_system_refs:r.get(25),jurisdiction_refs:r.get(26),
    temporal_refs:r.get(27),author_ref:r.get(28),institution_ref:r.get(29),candidate_only:r.get(30),creates_semantic_authority:r.get(31),
    applicability_promoted:r.get(32),claim_truth_promoted:r.get(33)}}

fn stable_ref(parts:&[&str])->String{format!("reviewed-evidence:sha256:{}",hex(&digest(parts)))}
fn digest(parts:&[&str])->[u8;32]{let mut h=Sha256::new();for p in parts{h.update((p.len() as u64).to_be_bytes());h.update(p.as_bytes());}h.finalize().into()}
fn hex(bytes:&[u8])->String{const D:&[u8;16]=b"0123456789abcdef";let mut o=String::with_capacity(bytes.len()*2);for &b in bytes{o.push(D[(b>>4)as usize]as char);o.push(D[(b&15)as usize]as char);}o}

#[cfg(test)]
mod tests{use super::*;fn draft()->ReviewedEvidenceDraft{ReviewedEvidenceDraft{review_receipt_ref:"review:1".into(),observation_ref:"observation:1".into(),consumer_ref:"consumer:1".into(),requirement_ref:"requirement:1".into(),evidence_role_ref:"role:support".into(),normative_order_ref:"order:crown-municipal".into(),proposition_ref:"proposition:1".into(),source_revision_ref:"source:1".into(),document_ref:"document:1".into(),statement_ref:"statement:1".into(),exact_span_ref:"span:1".into(),candidate_pnf_batch_ref:"batch:1".into(),candidate_factor_ref:"candidate:1".into(),parser_build_ref:"parser:1".into(),pnf_build_ref:"pnf:1".into(),refined_pnf_graph_ref:"graph:1".into(),pnf_factor_ref:"factor:1".into(),pnf_revision_ref:"revision:1".into(),structural_signature_ref:"signature:1".into(),predicate_ref:"predicate:1".into(),observation_provenance_refs:vec!["span:1".into()],observation_residual_refs:vec![],legal_system_refs:vec![],jurisdiction_refs:vec![],temporal_refs:vec![],author_ref:"author:1".into(),institution_ref:None}}
#[test]fn requires_normative_order(){let mut d=draft();d.normative_order_ref.clear();assert!(matches!(validate_draft(&d),Err(ReviewedEvidenceStoreError::EmptyCoordinate("normative_order_ref"))))}
#[test]fn exact_span_must_be_reviewed_provenance(){let mut d=draft();d.observation_provenance_refs=vec![];assert!(matches!(validate_draft(&d),Err(ReviewedEvidenceStoreError::ObservationMissingExactSpan)))}}
