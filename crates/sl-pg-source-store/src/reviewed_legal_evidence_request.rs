//! Resolve the database-owned portion of a reviewed legal-evidence request.
//!
//! The operator supplies only review/consumer/normative choices plus the
//! already-reviewed PNF coordinates. Source revision, document, exact span and
//! parser receipt are reopened from the persisted Statement/PNF batch and may
//! not be restated by an external manifest.

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::{
    load_legal_source_manifestation, DatabaseConfig, ReviewedEvidenceDraft,
    ReviewedLegalEvidenceRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedLegalEvidenceSelection {
    pub source_manifestation_ref: String,
    pub review_receipt_ref: String,
    pub observation_ref: String,
    pub consumer_ref: String,
    pub requirement_ref: String,
    pub evidence_role_ref: String,
    pub normative_order_ref: String,
    pub proposition_ref: String,
    pub candidate_pnf_batch_ref: String,
    pub candidate_factor_ref: String,
    pub pnf_build_ref: String,
    pub refined_pnf_graph_ref: String,
    pub pnf_factor_ref: String,
    pub pnf_revision_ref: String,
    pub structural_signature_ref: String,
    pub predicate_ref: String,
    pub legal_system_refs: Vec<String>,
    pub jurisdiction_refs: Vec<String>,
    pub temporal_refs: Vec<String>,
    pub author_ref: String,
    pub institution_ref: Option<String>,
}

#[derive(Debug, Error)]
pub enum ReviewedLegalEvidenceRequestError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("legal source manifestation failed: {0}")]
    Manifestation(#[from] crate::LegalSourceManifestationError),
    #[error("required selection coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("candidate batch/factor/statement does not reopen exactly")]
    CandidateMismatch,
    #[error("candidate source differs from the exact native legal manifestation")]
    SourceMismatch,
}

pub fn resolve_reviewed_legal_evidence_request(
    config: &DatabaseConfig,
    selection: &ReviewedLegalEvidenceSelection,
) -> Result<ReviewedLegalEvidenceRequest, ReviewedLegalEvidenceRequestError> {
    validate_selection(selection)?;
    let manifestation = load_legal_source_manifestation(config, &selection.source_manifestation_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT b.statement_ref,b.exact_span_ref,b.parser_receipt_ref,
                  s.source_revision_ref,s.document_ref,
                  b.candidate_only,b.semantic_admission_paid,b.proposition_support_paid,
                  b.applicability_paid,b.claim_truth_paid,
                  f.candidate_only
           FROM pnf.statement_candidate_batch b
           JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
           JOIN pnf.statement_candidate_factor f
             ON f.batch_ref=b.batch_ref AND f.statement_ref=b.statement_ref
           WHERE b.batch_ref=$1 AND f.candidate_ref=$2"#,
        &[&selection.candidate_pnf_batch_ref,&selection.candidate_factor_ref],
    )?.ok_or(ReviewedLegalEvidenceRequestError::CandidateMismatch)?;
    if !row.get::<_,bool>(5)
        || row.get::<_,bool>(6)||row.get::<_,bool>(7)||row.get::<_,bool>(8)||row.get::<_,bool>(9)
        || !row.get::<_,bool>(10)
    {
        return Err(ReviewedLegalEvidenceRequestError::CandidateMismatch);
    }
    let statement_ref:String=row.get(0);
    let exact_span_ref:String=row.get(1);
    let parser_receipt_ref:String=row.get(2);
    let source_revision_ref:String=row.get(3);
    let document_ref:String=row.get(4);
    if source_revision_ref!=manifestation.source_revision_ref || document_ref!=manifestation.document_ref {
        return Err(ReviewedLegalEvidenceRequestError::SourceMismatch);
    }

    Ok(ReviewedLegalEvidenceRequest {
        source_manifestation_ref: selection.source_manifestation_ref.clone(),
        evidence: ReviewedEvidenceDraft {
            review_receipt_ref: selection.review_receipt_ref.clone(),
            observation_ref: selection.observation_ref.clone(),
            consumer_ref: selection.consumer_ref.clone(),
            requirement_ref: selection.requirement_ref.clone(),
            evidence_role_ref: selection.evidence_role_ref.clone(),
            normative_order_ref: selection.normative_order_ref.clone(),
            proposition_ref: selection.proposition_ref.clone(),
            source_revision_ref,
            document_ref,
            statement_ref,
            exact_span_ref: exact_span_ref.clone(),
            candidate_pnf_batch_ref: selection.candidate_pnf_batch_ref.clone(),
            candidate_factor_ref: selection.candidate_factor_ref.clone(),
            // The parser receipt is the durable build/input receipt already tied
            // to the candidate batch. No caller-supplied parser identity is used.
            parser_build_ref: parser_receipt_ref,
            pnf_build_ref: selection.pnf_build_ref.clone(),
            refined_pnf_graph_ref: selection.refined_pnf_graph_ref.clone(),
            pnf_factor_ref: selection.pnf_factor_ref.clone(),
            pnf_revision_ref: selection.pnf_revision_ref.clone(),
            structural_signature_ref: selection.structural_signature_ref.clone(),
            predicate_ref: selection.predicate_ref.clone(),
            observation_provenance_refs: vec![exact_span_ref, manifestation.manifestation_ref.clone()],
            observation_residual_refs: vec![],
            legal_system_refs: selection.legal_system_refs.clone(),
            jurisdiction_refs: selection.jurisdiction_refs.clone(),
            temporal_refs: selection.temporal_refs.clone(),
            author_ref: selection.author_ref.clone(),
            institution_ref: selection.institution_ref.clone(),
        },
    })
}

fn validate_selection(s:&ReviewedLegalEvidenceSelection)->Result<(),ReviewedLegalEvidenceRequestError>{
    for (name,value) in [
        ("source_manifestation_ref",s.source_manifestation_ref.as_str()),
        ("review_receipt_ref",s.review_receipt_ref.as_str()),("observation_ref",s.observation_ref.as_str()),
        ("consumer_ref",s.consumer_ref.as_str()),("requirement_ref",s.requirement_ref.as_str()),
        ("evidence_role_ref",s.evidence_role_ref.as_str()),("normative_order_ref",s.normative_order_ref.as_str()),
        ("proposition_ref",s.proposition_ref.as_str()),("candidate_pnf_batch_ref",s.candidate_pnf_batch_ref.as_str()),
        ("candidate_factor_ref",s.candidate_factor_ref.as_str()),("pnf_build_ref",s.pnf_build_ref.as_str()),
        ("refined_pnf_graph_ref",s.refined_pnf_graph_ref.as_str()),("pnf_factor_ref",s.pnf_factor_ref.as_str()),
        ("pnf_revision_ref",s.pnf_revision_ref.as_str()),("structural_signature_ref",s.structural_signature_ref.as_str()),
        ("predicate_ref",s.predicate_ref.as_str()),("author_ref",s.author_ref.as_str()),
    ] { if value.trim().is_empty(){return Err(ReviewedLegalEvidenceRequestError::EmptyCoordinate(name));} }
    Ok(())
}
