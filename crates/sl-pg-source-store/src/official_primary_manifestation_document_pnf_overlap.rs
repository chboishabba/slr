//! Durable comparator-scoped document-pair candidate-PNF diagnostic.
//! Uses DB-native source/compilation products; no new parser or span alignment.
use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;
use crate::{
    DatabaseConfig, CandidatePnfFactor, CandidatePnfStoreError,
    load_candidate_pnf_batch, load_generic_text_source,
    load_long_document_ingest_receipt, load_official_primary_manifestation_comparison,
    measure_semantic_pnf_coverage, semantic_pnf_coverage_is_consistent,
    SemanticPnfCoverage, CANDIDATE_PNF_COMPILER_REF,
};

pub const DOCUMENT_PNF_COMPARATOR_REF: &str =
    "official-primary-manifestation-document-pnf:v1";
pub const DOCUMENT_PNF_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS source_provenance;
CREATE TABLE IF NOT EXISTS source_provenance.official_primary_document_pnf_overlap (
 receipt_ref TEXT PRIMARY KEY,
 manifestation_comparison_ref TEXT NOT NULL
   REFERENCES source_provenance.official_primary_manifestation_comparison(comparison_ref),
 pdf_source_revision_ref TEXT NOT NULL,
 docx_source_revision_ref TEXT NOT NULL,
 pdf_extraction_ref TEXT NOT NULL,
 docx_extraction_ref TEXT NOT NULL,
 pdf_digest TEXT NOT NULL,
 docx_digest TEXT NOT NULL,
 pdf_parser_run_ref TEXT NOT NULL,
 docx_parser_run_ref TEXT NOT NULL,
 pdf_batch_refs TEXT[] NOT NULL,
 docx_batch_refs TEXT[] NOT NULL,
 pdf_inventory_sha256 TEXT NOT NULL,
 docx_inventory_sha256 TEXT NOT NULL,
 comparator_ref TEXT NOT NULL,
 projection_ref TEXT NOT NULL,
 pdf_semantic_count BIGINT NOT NULL,
 docx_semantic_count BIGINT NOT NULL,
 matched_count BIGINT NOT NULL,
 unmatched_pdf_count BIGINT NOT NULL,
 unmatched_docx_count BIGINT NOT NULL,
 pdf_other_count BIGINT NOT NULL,
 docx_other_count BIGINT NOT NULL,
 pdf_parser_residual_count BIGINT NOT NULL,
 docx_parser_residual_count BIGINT NOT NULL,
 review_refs TEXT[] NOT NULL,
 semantic_exact BOOLEAN NOT NULL,
 semantic_pnf_overlap_closed BOOLEAN NOT NULL,
 transformation_closed BOOLEAN NOT NULL,
 candidate_only BOOLEAN NOT NULL CHECK(candidate_only),
 creates_legal_controversy BOOLEAN NOT NULL CHECK(NOT creates_legal_controversy),
 creates_semantic_authority BOOLEAN NOT NULL CHECK(NOT creates_semantic_authority),
 creates_legal_authority BOOLEAN NOT NULL CHECK(NOT creates_legal_authority),
 applicability_promoted BOOLEAN NOT NULL CHECK(NOT applicability_promoted),
 claim_truth_promoted BOOLEAN NOT NULL CHECK(NOT claim_truth_promoted),
 consumer_informativeness_paid BOOLEAN NOT NULL CHECK(NOT consumer_informativeness_paid),
 independent_source_count_increment INTEGER NOT NULL CHECK(independent_source_count_increment=0),
 UNIQUE(manifestation_comparison_ref,pdf_source_revision_ref,docx_source_revision_ref,comparator_ref)
);
ALTER TABLE source_provenance.official_primary_document_pnf_overlap
  ADD COLUMN IF NOT EXISTS semantic_pnf_overlap_closed BOOLEAN NOT NULL DEFAULT FALSE;
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPnfSide {
    pub source_revision_ref: String,
    pub extraction_ref: String,
    pub digest: String,
    pub parser_run_ref: String,
    pub batch_refs: Vec<String>,
    pub inventory_sha256: String,
    pub parser_residual_count: usize,
    pub factors: Vec<CandidatePnfFactor>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPnfReceipt {
    pub receipt_ref: String,
    pub manifestation_comparison_ref: String,
    pub pdf: DocumentPnfSide,
    pub docx: DocumentPnfSide,
    pub coverage: SemanticPnfCoverage,
    pub review_refs: Vec<String>,
    pub semantic_pnf_overlap_closed: bool,
    pub transformation_closed: bool,
    pub candidate_only: bool,
    pub creates_legal_controversy: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub consumer_informativeness_paid: bool,
    pub independent_source_count_increment: i32,
}
#[derive(Debug, Error)]
pub enum DocumentPnfError {
    #[error("postgres: {0}")] Postgres(#[from] postgres::Error),
    #[error("candidate store: {0}")] Candidate(#[from] CandidatePnfStoreError),
    #[error("source store: {0}")] Source(#[from] crate::GenericSourceContentStoreError),
    #[error("compilation: {0}")] Compilation(#[from] crate::LongDocumentIngestStoreError),
    #[error("manifestation: {0}")] Manifestation(#[from] crate::OfficialPrimaryManifestationComparisonError),
    #[error("source/compilation/candidate identity or coverage mismatch: {0}")] Invalid(String),
    #[error("immutable document-pair receipt conflicts with persisted identity")] Conflict,
}
fn hash_values(values: &[String]) -> String {
    let mut h = Sha256::new();
    for v in values { h.update((v.len() as u64).to_be_bytes()); h.update(v.as_bytes()); }
    format!("{:x}",h.finalize())
}
fn load_side(config: &DatabaseConfig, revision: &str, extraction: &str, digest: &str)
    -> Result<DocumentPnfSide,DocumentPnfError> {
    let (source,text) = load_generic_text_source(config,revision)?;
    if source.acquisition_receipt_ref != extraction
        || source.content_digest_ref.is_empty()
        || source.document_ref.is_empty()
        || format!("{:x}",Sha256::digest(text.as_bytes())) != digest {
        return Err(DocumentPnfError::Invalid("official extraction/source digest mismatch".into()));
    }
    let compiled = load_long_document_ingest_receipt(config,revision)?;
    if compiled.source_ref != source.source_ref || compiled.region_count == 0
        || compiled.source_region_loss_count != 0 || !compiled.candidate_only
        || compiled.creates_semantic_authority || compiled.applicability_promoted
        || compiled.claim_truth_promoted {
        return Err(DocumentPnfError::Invalid("lossless compilation mismatch".into()));
    }
    let mut c = Client::connect(config.database_url(),NoTls)?;
    let stages = c.query(
        "SELECT parser_run_ref,statement_count,batch_count,factor_count
         FROM pnf.candidate_persistence_stage_receipt
         WHERE source_revision_ref=$1 AND compiler_ref=$2 AND exact_reopen_validated
           AND candidate_only AND NOT creates_semantic_authority
           AND NOT applicability_promoted AND NOT claim_truth_promoted
         ORDER BY parser_run_ref", &[&revision,&CANDIDATE_PNF_COMPILER_REF])?;
    if stages.len()!=1 { return Err(DocumentPnfError::Invalid("missing/ambiguous exact parser stage".into())); }
    let stage=&stages[0];
    let parser_run_ref: String=stage.get(0);
    let expected_statements: i64=stage.get(1);
    let expected_batches: i64=stage.get(2);
    let expected_factors: i64=stage.get(3);
    if expected_statements < 0 || expected_batches < 0 || expected_factors < 0
        || expected_statements as usize != compiled.compiled_count
        || expected_batches != expected_statements {
        return Err(DocumentPnfError::Invalid("stage/compilation count mismatch".into()));
    }
    let rows=c.query(
        "SELECT b.batch_ref
         FROM pnf.statement_candidate_batch b
         JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
         WHERE s.source_revision_ref=$1
         ORDER BY b.batch_ref", &[&revision])?;
    if rows.len()!=expected_batches as usize {
        return Err(DocumentPnfError::Invalid("missing or stale candidate batch".into()));
    }
    let mut refs=Vec::new();
    let mut factors=Vec::new();
    let mut ancestry=Vec::new();
    for row in rows {
        let reference:String=row.get(0);
        let batch=load_candidate_pnf_batch(config,&reference)?
            .ok_or_else(||DocumentPnfError::Invalid(format!("missing batch {reference}")))?;
        if batch.batch_ref != reference || !batch.candidate_only
            || batch.semantic_admission_paid || batch.proposition_support_paid
            || batch.applicability_paid || batch.claim_truth_paid
            || batch.factors.iter().any(|f|!f.candidate_only)
            || !batch.parser_receipt_ref.starts_with(&format!("db-parser:{parser_run_ref}:")) {
            return Err(DocumentPnfError::Invalid(format!("stale/promoted batch {reference}")));
        }
        ancestry.extend([reference.clone(),batch.statement_ref.clone(),
            batch.exact_span_ref.clone(),batch.parser_receipt_ref.clone()]);
        for f in &batch.factors {
            ancestry.extend([f.candidate_ref.clone(),format!("{:?}",f.role),
                f.lemma.clone(),f.dependency_ref.clone(),f.surface.clone()]);
        }
        factors.extend(batch.factors);
        refs.push(reference);
    }
    if factors.len()!=expected_factors as usize {
        return Err(DocumentPnfError::Invalid("candidate factor count mismatch".into()));
    }
    Ok(DocumentPnfSide {
        source_revision_ref:revision.into(),extraction_ref:extraction.into(),
        digest:digest.into(),parser_run_ref,batch_refs:refs,
        inventory_sha256:hash_values(&ancestry),
        parser_residual_count:compiled.residual_count,factors,
    })
}
/// Candidate-only, source-bound pure result. Persistence reopens the same
/// evidence before accepting identity; review refs cannot silently pay Other.
pub fn assess_document_pnf_pair(
    comparison_ref:&str,pdf:DocumentPnfSide,docx:DocumentPnfSide,
    review_refs:Vec<String>,
) -> Result<DocumentPnfReceipt,DocumentPnfError> {
    if comparison_ref.is_empty() || review_refs.iter().any(String::is_empty)
        || pdf.source_revision_ref == docx.source_revision_ref
        || pdf.batch_refs.is_empty() || docx.batch_refs.is_empty() {
        return Err(DocumentPnfError::Invalid("invalid document-pair coordinates".into()));
    }
    let coverage=measure_semantic_pnf_coverage(&pdf.factors,&docx.factors);
    if !semantic_pnf_coverage_is_consistent(&coverage) {
        return Err(DocumentPnfError::Invalid("invalid comparator coverage".into()));
    }
    // Review coordinates are retained as residual evidence, not trusted as
    // a materiality/closure decision until independently reopened.
    // The established v3 semantic projection excludes Other. Keep semantic
    // exactness separate from review-dependent whole-transformation closure.
    let semantic_pnf_overlap_closed=coverage.semantic_factor_multiset_exact
        && pdf.parser_residual_count==0 && docx.parser_residual_count==0;
    // No trusted persisted Other-materiality review loader is wired yet.
    // An arbitrary caller-supplied review ref must not authorize closure.
    let transformation_closed=semantic_pnf_overlap_closed
        && coverage.ignored_pdf_nonsemantic_count==0
        && coverage.ignored_docx_nonsemantic_count==0;
    let mut identity=vec![DOCUMENT_PNF_COMPARATOR_REF.into(),
        crate::EXHAUSTIVE_PNF_COMPARATOR_REVISION_REF.into(),
        coverage.projection_ref.clone(),comparison_ref.into()];
    for side in [&pdf,&docx] {
        identity.extend([side.source_revision_ref.clone(),side.extraction_ref.clone(),
            side.digest.clone(),side.parser_run_ref.clone(),side.inventory_sha256.clone(),
            side.parser_residual_count.to_string(),side.batch_refs.len().to_string()]);
        identity.extend(side.batch_refs.clone());
    }
    identity.extend([coverage.pdf_semantic_factor_count.to_string(),
        coverage.docx_semantic_factor_count.to_string(),
        coverage.matched_semantic_factor_count.to_string(),
        coverage.unmatched_pdf_semantic_count.to_string(),
        coverage.unmatched_docx_semantic_count.to_string(),
        coverage.ignored_pdf_nonsemantic_count.to_string(),
        coverage.ignored_docx_nonsemantic_count.to_string()]);
    let mut review_refs=review_refs;
    review_refs.sort();
    identity.extend(review_refs.clone());
    identity.extend([semantic_pnf_overlap_closed.to_string(),transformation_closed.to_string()]);
    let receipt_ref=format!("official-primary-document-pnf:sha256:{}",hash_values(&identity));
    Ok(DocumentPnfReceipt {receipt_ref,manifestation_comparison_ref:comparison_ref.into(),
        pdf,docx,coverage,review_refs,semantic_pnf_overlap_closed,transformation_closed,candidate_only:true,
        creates_legal_controversy:false,creates_semantic_authority:false,
        creates_legal_authority:false,applicability_promoted:false,
        claim_truth_promoted:false,consumer_informativeness_paid:false,
        independent_source_count_increment:0})
}
pub fn install_document_pnf_schema(config:&DatabaseConfig)->Result<(),DocumentPnfError>{
    let mut c=Client::connect(config.database_url(),NoTls)?;
    c.batch_execute(DOCUMENT_PNF_SQL)?;
    Ok(())
}
/// Recompute from persisted canonical source and batches on each invocation.
pub fn compare_and_persist_document_pnf_pair(
    config:&DatabaseConfig,comparison_ref:&str,
    pdf_revision:&str,docx_revision:&str,
)->Result<DocumentPnfReceipt,DocumentPnfError>{
    install_document_pnf_schema(config)?;
    let comparison=load_official_primary_manifestation_comparison(config,comparison_ref)?;
    let pdf=load_side(config,pdf_revision,&comparison.pdf_extraction_ref,&comparison.pdf_output_sha256_hex)?;
    let docx=load_side(config,docx_revision,&comparison.docx_extraction_ref,&comparison.docx_output_sha256_hex)?;
    let expected=assess_document_pnf_pair(comparison_ref,pdf,docx,Vec::new())?;
    let mut c=Client::connect(config.database_url(),NoTls)?;
    let k=&expected.coverage;
    let n=|x:usize|i64::try_from(x).map_err(|_|DocumentPnfError::Invalid("count overflow".into()));
    c.execute(
        "INSERT INTO source_provenance.official_primary_document_pnf_overlap
        (receipt_ref,manifestation_comparison_ref,pdf_source_revision_ref,docx_source_revision_ref,
         pdf_extraction_ref,docx_extraction_ref,pdf_digest,docx_digest,
         pdf_parser_run_ref,docx_parser_run_ref,pdf_batch_refs,docx_batch_refs,
         pdf_inventory_sha256,docx_inventory_sha256,comparator_ref,projection_ref,
         pdf_semantic_count,docx_semantic_count,matched_count,unmatched_pdf_count,unmatched_docx_count,
         pdf_other_count,docx_other_count,pdf_parser_residual_count,docx_parser_residual_count,
         review_refs,semantic_exact,semantic_pnf_overlap_closed,transformation_closed,candidate_only,
         creates_legal_controversy,creates_semantic_authority,creates_legal_authority,
         applicability_promoted,claim_truth_promoted,consumer_informativeness_paid,
         independent_source_count_increment)
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
                $21,$22,$23,$24,$25,$26,$27,$28,$29,TRUE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,0)
         ON CONFLICT(receipt_ref) DO NOTHING",
        &[&expected.receipt_ref,&expected.manifestation_comparison_ref,
          &expected.pdf.source_revision_ref,&expected.docx.source_revision_ref,
          &expected.pdf.extraction_ref,&expected.docx.extraction_ref,
          &expected.pdf.digest,&expected.docx.digest,
          &expected.pdf.parser_run_ref,&expected.docx.parser_run_ref,
          &expected.pdf.batch_refs,&expected.docx.batch_refs,
          &expected.pdf.inventory_sha256,&expected.docx.inventory_sha256,
          &DOCUMENT_PNF_COMPARATOR_REF,&k.projection_ref,
          &n(k.pdf_semantic_factor_count)?,&n(k.docx_semantic_factor_count)?,
          &n(k.matched_semantic_factor_count)?,&n(k.unmatched_pdf_semantic_count)?,
          &n(k.unmatched_docx_semantic_count)?,&n(k.ignored_pdf_nonsemantic_count)?,
          &n(k.ignored_docx_nonsemantic_count)?,&n(expected.pdf.parser_residual_count)?,
          &n(expected.docx.parser_residual_count)?,&expected.review_refs,
          &k.semantic_factor_multiset_exact,&expected.semantic_pnf_overlap_closed,&expected.transformation_closed])?;
    let reopened=load_document_pnf_pair(config,&expected.receipt_ref)?;
    if reopened != expected {return Err(DocumentPnfError::Conflict);}
    Ok(reopened)
}
/// Reopen persisted coordinates, then independently recompute from source.
/// Any stale batch, digest or parser revision fails before equality comparison.
pub fn load_document_pnf_pair(
    config:&DatabaseConfig,receipt_ref:&str
)->Result<DocumentPnfReceipt,DocumentPnfError>{
    let mut c=Client::connect(config.database_url(),NoTls)?;
    let row=c.query_opt(
        "SELECT manifestation_comparison_ref,pdf_source_revision_ref,docx_source_revision_ref,
                pdf_extraction_ref,docx_extraction_ref,pdf_digest,docx_digest,
                pdf_parser_run_ref,docx_parser_run_ref,pdf_batch_refs,docx_batch_refs,
                pdf_inventory_sha256,docx_inventory_sha256,comparator_ref,projection_ref,
                pdf_semantic_count,docx_semantic_count,matched_count,unmatched_pdf_count,unmatched_docx_count,
                pdf_other_count,docx_other_count,pdf_parser_residual_count,docx_parser_residual_count,
                review_refs,semantic_exact,semantic_pnf_overlap_closed,transformation_closed,candidate_only,
                creates_legal_controversy,creates_semantic_authority,creates_legal_authority,
                applicability_promoted,claim_truth_promoted,consumer_informativeness_paid,
                independent_source_count_increment
         FROM source_provenance.official_primary_document_pnf_overlap WHERE receipt_ref=$1",
        &[&receipt_ref])?.ok_or(DocumentPnfError::Conflict)?;
    let comparison_ref:String=row.get(0);
    let comparison=load_official_primary_manifestation_comparison(config,&comparison_ref)?;
    let pdf_revision:String=row.get(1);
    let docx_revision:String=row.get(2);
    let pdf=load_side(config,&pdf_revision,&comparison.pdf_extraction_ref,&comparison.pdf_output_sha256_hex)?;
    let docx=load_side(config,&docx_revision,&comparison.docx_extraction_ref,&comparison.docx_output_sha256_hex)?;
    let reviews:Vec<String>=row.get(24);
    let expected=assess_document_pnf_pair(&comparison_ref,pdf,docx,reviews)?;
    if expected.receipt_ref!=receipt_ref
        || row.get::<_,String>(3)!=expected.pdf.extraction_ref
        || row.get::<_,String>(4)!=expected.docx.extraction_ref
        || row.get::<_,String>(5)!=expected.pdf.digest
        || row.get::<_,String>(6)!=expected.docx.digest
        || row.get::<_,String>(7)!=expected.pdf.parser_run_ref
        || row.get::<_,String>(8)!=expected.docx.parser_run_ref
        || row.get::<_,Vec<String>>(9)!=expected.pdf.batch_refs
        || row.get::<_,Vec<String>>(10)!=expected.docx.batch_refs
        || row.get::<_,String>(11)!=expected.pdf.inventory_sha256
        || row.get::<_,String>(12)!=expected.docx.inventory_sha256
        || row.get::<_,String>(13)!=DOCUMENT_PNF_COMPARATOR_REF
        || row.get::<_,String>(14)!=expected.coverage.projection_ref
        || row.get::<_,i64>(15)!=expected.coverage.pdf_semantic_factor_count as i64
        || row.get::<_,i64>(16)!=expected.coverage.docx_semantic_factor_count as i64
        || row.get::<_,i64>(17)!=expected.coverage.matched_semantic_factor_count as i64
        || row.get::<_,i64>(18)!=expected.coverage.unmatched_pdf_semantic_count as i64
        || row.get::<_,i64>(19)!=expected.coverage.unmatched_docx_semantic_count as i64
        || row.get::<_,i64>(20)!=expected.coverage.ignored_pdf_nonsemantic_count as i64
        || row.get::<_,i64>(21)!=expected.coverage.ignored_docx_nonsemantic_count as i64
        || row.get::<_,i64>(22)!=expected.pdf.parser_residual_count as i64
        || row.get::<_,i64>(23)!=expected.docx.parser_residual_count as i64
        || row.get::<_,bool>(25)!=expected.coverage.semantic_factor_multiset_exact
        || row.get::<_,bool>(26)!=expected.semantic_pnf_overlap_closed
        || row.get::<_,bool>(27)!=expected.transformation_closed
        || !row.get::<_,bool>(28)
        || (29..=34).any(|i|row.get::<_,bool>(i))
        || row.get::<_,i32>(35)!=0 {
        return Err(DocumentPnfError::Conflict);
    }
    Ok(expected)
}
