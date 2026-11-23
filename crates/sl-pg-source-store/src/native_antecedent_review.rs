//! C4.11: same-source anaphor candidate -> canonical S29 review workstation.
//!
//! This is a source-addressed *review proposal*, not an identity resolver.
//! The existing S29 store alone owns mutable review state and review commands.
//! Even S29 Accept does not mint an AntecedentResolutionReceipt or external QID.

use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewItem, ReviewItemKind, ReviewStatus,
};
use thiserror::Error;

use crate::{DatabaseConfig, ReviewWorkstationStoreError};

pub const SCHEMA: &str = "slr-native-antecedent-source-span-candidate-v1";
pub const ANTECEDENT_REVIEW_SCHEMA: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;
CREATE TABLE IF NOT EXISTS semantic.native_antecedent_candidate_review (
  proposal_ref TEXT PRIMARY KEY,
  review_item_ref TEXT NOT NULL UNIQUE REFERENCES semantic.review_item(review_item_ref),
  original_source_ref TEXT NOT NULL,
  source_revision_ref TEXT NOT NULL,
  document_ref TEXT NOT NULL,
  canonical_content_sha256 TEXT NOT NULL,
  original_context_sha256 TEXT NOT NULL,
  focus_mention_ref TEXT NOT NULL,
  focus_candidate_ref TEXT NOT NULL,
  focus_statement_ref TEXT NOT NULL,
  focus_region_ref TEXT NOT NULL,
  focus_start_char BIGINT NOT NULL,
  focus_end_char BIGINT NOT NULL,
  focus_surface TEXT NOT NULL,
  candidate_statement_ref TEXT NOT NULL,
  candidate_region_ref TEXT NOT NULL,
  candidate_start_char BIGINT NOT NULL,
  candidate_end_char BIGINT NOT NULL,
  candidate_surface TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK(candidate_only),
  semantic_authority_promoted BOOLEAN NOT NULL CHECK(NOT semantic_authority_promoted),
  qid_admitted BOOLEAN NOT NULL CHECK(NOT qid_admitted),
  CHECK (candidate_start_char < candidate_end_char),
  CHECK (focus_start_char < focus_end_char),
  CHECK (candidate_end_char <= focus_start_char)
);
"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeSourceMention {
    pub mention_ref: String,
    pub candidate_ref: String,
    pub statement_ref: String,
    pub region_ref: String,
    pub start_char: u64,
    pub end_char: u64,
    pub surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeSourceSpan {
    pub statement_ref: String,
    pub region_ref: String,
    pub start_char: u64,
    pub end_char: u64,
    pub surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeAntecedentSourceCandidate {
    pub schema: String,
    pub original_context_sha256: String,
    pub original_source_ref: String,
    pub source_revision_ref: String,
    pub document_ref: String,
    pub canonical_content_sha256: String,
    pub focus: NativeSourceMention,
    pub antecedent_source_span: NativeSourceSpan,
    pub candidate_only: bool,
    pub same_source_span_geometry_checked: bool,
    pub antecedent_resolution: String,
    pub identity_admission: String,
    pub external_qid: String,
    pub claim_truth_paid: bool,
    pub semantic_authority_promoted: bool,
    pub review_owner: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAntecedentReviewProposal {
    pub proposal_ref: String,
    pub review_item_ref: String,
    pub source_revision_ref: String,
    pub focus_statement_ref: String,
    pub candidate_statement_ref: String,
    pub review_status: ReviewStatus,
    pub antecedent_resolution: &'static str,
    pub identity_admission_paid: bool,
    pub external_qid_paid: bool,
}

#[derive(Debug, Error)]
pub enum NativeAntecedentReviewError {
    #[error("invalid or promoted original source antecedent coordinates")]
    InvalidCandidate,
    #[error("canonical SLR source/span/statement not found or does not match")]
    OriginalSourceMismatch,
    #[error("immutable review proposal conflict")]
    ImmutableConflict,
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    S29(#[from] ReviewWorkstationStoreError),
}

fn nonempty(s: &str) -> bool { !s.trim().is_empty() }
fn same_set<T: Ord + Clone>(a: &[T], b: &[T]) -> bool {
    use std::collections::BTreeSet;
    a.iter().cloned().collect::<BTreeSet<_>>()
        == b.iter().cloned().collect::<BTreeSet<_>>()
}
fn digest(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in std::iter::once("native-antecedent-review:v1").chain(parts.iter().copied()) {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn sha256(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}

fn valid_digest(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// A reviewed antecedent remains a *separate* semantic obligation.
pub fn validate_native_antecedent_candidate(
    c: &NativeAntecedentSourceCandidate,
) -> Result<(), NativeAntecedentReviewError> {
    let f = &c.focus;
    let a = &c.antecedent_source_span;
    let refs = [
        &c.original_source_ref, &c.source_revision_ref, &c.document_ref,
        &f.mention_ref, &f.candidate_ref, &f.statement_ref, &f.region_ref,
        &a.statement_ref, &a.region_ref, &f.surface, &a.surface,
    ];
    if c.schema != SCHEMA
        || !refs.iter().all(|s| nonempty(s))
        || !valid_digest(&c.canonical_content_sha256)
        || !valid_digest(&c.original_context_sha256)
        || c.document_ref != format!("document:sha256:{}", c.canonical_content_sha256)
        || c.candidate_only != true || c.same_source_span_geometry_checked != true
        || c.semantic_authority_promoted || c.claim_truth_paid
        || c.antecedent_resolution != "REVIEW_REQUIRED"
        || c.identity_admission != "NOT_PAID"
        || c.external_qid != "NOT_TESTED"
        || c.review_owner != "SLR_GOVERNED_ANTECEDENT_REVIEW"
        || a.start_char >= a.end_char || f.start_char >= f.end_char
        || a.end_char > f.start_char
        || a.statement_ref == f.statement_ref && a.region_ref != f.region_ref
        || a.start_char > i64::MAX as u64 || a.end_char > i64::MAX as u64
        || f.start_char > i64::MAX as u64 || f.end_char > i64::MAX as u64
    {
        return Err(NativeAntecedentReviewError::InvalidCandidate);
    }
    Ok(())
}

fn original_statement(
    client: &mut Client,
    c: &NativeAntecedentSourceCandidate,
    statement_ref: &str,
    region_ref: &str,
    start_char: u64,
    end_char: u64,
    surface: &str,
    text: &str,
) -> Result<(), NativeAntecedentReviewError> {
    let rows=client.query(
        r#"SELECT r.start_char,r.end_char,s.literal_text,s.candidate_only,
                s.creates_semantic_authority,s.applicability_promoted,
                s.claim_truth_promoted,r.candidate_only,r.creates_semantic_authority,
                r.claim_truth_promoted
         FROM ingest.long_document_region r
         JOIN corpus.source_statement s
           ON s.source_revision_ref=r.source_revision_ref
          AND s.exact_span_ref=r.region_ref
          AND s.document_ref=$1
         WHERE r.source_revision_ref=$2 AND r.region_ref=$3
           AND s.statement_ref=$4 AND r.source_ref=$5
           AND r.region_kind='sentence'"#,
        &[&c.document_ref,&c.source_revision_ref,&region_ref,&statement_ref,
          &c.original_source_ref],
    )?;
    if rows.len()!=1 { return Err(NativeAntecedentReviewError::OriginalSourceMismatch); }
    let row=&rows[0];
    let st: i64=row.get(0);
    let en: i64=row.get(1);
    let original: String=row.get(2);
    if st<0 || en<=st || end_char>en as u64 || start_char<st as u64
        || !row.get::<_,bool>(3) || row.get::<_,bool>(4)
        || row.get::<_,bool>(5) || row.get::<_,bool>(6)
        || !row.get::<_,bool>(7) || row.get::<_,bool>(8)
        || row.get::<_,bool>(9)
    { return Err(NativeAntecedentReviewError::OriginalSourceMismatch); }
    let full=text.chars().skip(st as usize).take((en-st) as usize).collect::<String>();
    let found=text.chars().skip(start_char as usize)
        .take((end_char-start_char) as usize).collect::<String>();
    if original != full || found != surface
        || original.chars().count() != (en-st) as usize
        || surface.chars().count() != (end_char-start_char) as usize
    {
        return Err(NativeAntecedentReviewError::OriginalSourceMismatch);
    }
    Ok(())
}

fn original_source_match(
    client: &mut Client,
    c: &NativeAntecedentSourceCandidate,
) -> Result<String, NativeAntecedentReviewError> {
    let rows=client.query(
        r#"SELECT g.source_ref,g.document_ref,g.content_digest_ref,
                g.candidate_only,g.creates_semantic_authority,
                g.applicability_promoted,g.claim_truth_promoted,c.payload
         FROM ingest.generic_source_revision g
         JOIN corpus.document d ON d.document_ref=g.document_ref
         JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
         WHERE g.source_revision_ref=$1"#,
        &[&c.source_revision_ref],
    )?;
    if rows.len()!=1 { return Err(NativeAntecedentReviewError::OriginalSourceMismatch); }
    let row=&rows[0];
    let bytes:Vec<u8>=row.get(7);
    let body=String::from_utf8(bytes.clone())
        .map_err(|_| NativeAntecedentReviewError::OriginalSourceMismatch)?;
    if row.get::<_,String>(0)!=c.original_source_ref
        || row.get::<_,String>(1)!=c.document_ref
        || row.get::<_,String>(2)!=format!("sha256:{}",c.canonical_content_sha256)
        || sha256(&bytes)!=c.canonical_content_sha256
        || !row.get::<_,bool>(3) || row.get::<_,bool>(4)
        || row.get::<_,bool>(5) || row.get::<_,bool>(6)
    { return Err(NativeAntecedentReviewError::OriginalSourceMismatch); }
    original_statement(client,c,&c.focus.statement_ref,&c.focus.region_ref,
                       c.focus.start_char,c.focus.end_char,&c.focus.surface,&body)?;
    original_statement(client,c,&c.antecedent_source_span.statement_ref,
                       &c.antecedent_source_span.region_ref,
                       c.antecedent_source_span.start_char,
                       c.antecedent_source_span.end_char,
                       &c.antecedent_source_span.surface,&body)?;
    Ok(body)
}

fn proposal_identity(c: &NativeAntecedentSourceCandidate) -> String {
    let f=&c.focus;
    let a=&c.antecedent_source_span;
    let xs=[
        c.original_source_ref.as_str(),c.source_revision_ref.as_str(),
        c.document_ref.as_str(),c.canonical_content_sha256.as_str(),
        c.original_context_sha256.as_str(),
        f.mention_ref.as_str(),f.candidate_ref.as_str(),
        f.statement_ref.as_str(),f.region_ref.as_str(),
        a.statement_ref.as_str(),a.region_ref.as_str(),
        f.surface.as_str(),a.surface.as_str(),
    ];
    let mut input=xs.to_vec();
    let fs=f.start_char.to_string();
    let fe=f.end_char.to_string();
    let as_=a.start_char.to_string();
    let ae=a.end_char.to_string();
    input.extend_from_slice(&[&fs,&fe,&as_,&ae]);
    format!("native-antecedent-candidate:sha256:{}",digest(&input))
}

/// Read and verify the original SLR source first, then persist only
/// a candidate-bound immutable sidecar and ordinary S29 review item.
pub fn prepare_native_antecedent_review(
    config: &DatabaseConfig,
    candidate: &NativeAntecedentSourceCandidate,
) -> Result<NativeAntecedentReviewProposal, NativeAntecedentReviewError> {
    validate_native_antecedent_candidate(candidate)?;
    let mut client=Client::connect(config.database_url(),NoTls)?;
    original_source_match(&mut client,candidate)?;

    let proposal_ref=proposal_identity(candidate);
    let review_item_ref=format!("review-item:{proposal_ref}");
    let f=&candidate.focus;
    let a=&candidate.antecedent_source_span;
    let item=ReviewItem{
        review_item_ref: review_item_ref.clone(),
        semantic_ref: proposal_ref.clone(),
        item_kind: ReviewItemKind::Observation,
        reason: "Review SAME-SOURCE antecedent candidate; S29 acceptance alone does not resolve coreference".into(),
        provenance_refs:vec![
            format!("original-context-sha256:{}",candidate.original_context_sha256),
            f.mention_ref.clone(),f.candidate_ref.clone(),
        ],
        source_refs:vec![
            candidate.source_revision_ref.clone(),candidate.document_ref.clone(),
            a.statement_ref.clone(),f.statement_ref.clone(),
        ],
        current_status:ReviewStatus::Pending,
        available_actions:vec![
            ReviewAction::Accept,ReviewAction::Reject,
            ReviewAction::Abstain,ReviewAction::Qualify,
            ReviewAction::RequestEvidence,ReviewAction::OpenSource,
        ],
        affected_consumer_refs:vec![f.mention_ref.clone()],
        candidate_only:true,creates_semantic_authority:false,
        applicability_promoted:false,claim_truth_promoted:false,
    };
    crate::install_review_workstation_schema(config)?;
    if let Some(existing)=crate::load_review_item(config,&review_item_ref)? {
        if existing.semantic_ref != item.semantic_ref
            || existing.item_kind != item.item_kind
            || existing.reason != item.reason
            || !same_set(&existing.provenance_refs, &item.provenance_refs)
            || !same_set(&existing.source_refs, &item.source_refs)
            || !same_set(&existing.available_actions, &item.available_actions)
            || !same_set(&existing.affected_consumer_refs, &item.affected_consumer_refs)
            || !existing.candidate_only || existing.creates_semantic_authority
            || existing.applicability_promoted || existing.claim_truth_promoted
        { return Err(NativeAntecedentReviewError::ImmutableConflict); }
    } else {
        let _=crate::persist_review_item(config,&item)?;
    }
    client.batch_execute(ANTECEDENT_REVIEW_SCHEMA)?;
    client.execute(
        "INSERT INTO semantic.native_antecedent_candidate_review
         (proposal_ref,review_item_ref,original_source_ref,source_revision_ref,
          document_ref,canonical_content_sha256,original_context_sha256,
          focus_mention_ref,focus_candidate_ref,focus_statement_ref,focus_region_ref,
          focus_start_char,focus_end_char,focus_surface,
          candidate_statement_ref,candidate_region_ref,candidate_start_char,
          candidate_end_char,candidate_surface,candidate_only,
          semantic_authority_promoted,qid_admitted)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,TRUE,FALSE,FALSE)
         ON CONFLICT(proposal_ref) DO NOTHING",
        &[
            &proposal_ref,&review_item_ref,&candidate.original_source_ref,
            &candidate.source_revision_ref,&candidate.document_ref,
            &candidate.canonical_content_sha256,&candidate.original_context_sha256,
            &f.mention_ref,&f.candidate_ref,&f.statement_ref,&f.region_ref,
            &(f.start_char as i64),&(f.end_char as i64),&f.surface,
            &a.statement_ref,&a.region_ref,
            &(a.start_char as i64),&(a.end_char as i64),&a.surface
        ],
    )?;
    let rows=client.query(
        "SELECT review_item_ref,original_source_ref,source_revision_ref,document_ref,
                canonical_content_sha256,original_context_sha256,focus_mention_ref,
                focus_candidate_ref,focus_statement_ref,focus_region_ref,
                focus_start_char,focus_end_char,focus_surface,
                candidate_statement_ref,candidate_region_ref,
                candidate_start_char,candidate_end_char,candidate_surface,
                candidate_only,semantic_authority_promoted,qid_admitted
         FROM semantic.native_antecedent_candidate_review
         WHERE proposal_ref=$1",
        &[&proposal_ref],
    )?;
    if rows.len()!=1 {return Err(NativeAntecedentReviewError::ImmutableConflict);}
    let row=&rows[0];
    if row.get::<_,String>(0)!=review_item_ref
        || row.get::<_,String>(1)!=candidate.original_source_ref
        || row.get::<_,String>(2)!=candidate.source_revision_ref
        || row.get::<_,String>(3)!=candidate.document_ref
        || row.get::<_,String>(4)!=candidate.canonical_content_sha256
        || row.get::<_,String>(5)!=candidate.original_context_sha256
        || row.get::<_,String>(6)!=f.mention_ref
        || row.get::<_,String>(7)!=f.candidate_ref
        || row.get::<_,String>(8)!=f.statement_ref
        || row.get::<_,String>(9)!=f.region_ref
        || row.get::<_,i64>(10)!=f.start_char as i64
        || row.get::<_,i64>(11)!=f.end_char as i64
        || row.get::<_,String>(12)!=f.surface
        || row.get::<_,String>(13)!=a.statement_ref
        || row.get::<_,String>(14)!=a.region_ref
        || row.get::<_,i64>(15)!=a.start_char as i64
        || row.get::<_,i64>(16)!=a.end_char as i64
        || row.get::<_,String>(17)!=a.surface
        || !row.get::<_,bool>(18) || row.get::<_,bool>(19)
        || row.get::<_,bool>(20)
    {return Err(NativeAntecedentReviewError::ImmutableConflict);}
    let owned=crate::load_review_item(config,&review_item_ref)?
        .ok_or(NativeAntecedentReviewError::ImmutableConflict)?;
    Ok(NativeAntecedentReviewProposal{
        proposal_ref,review_item_ref,
        source_revision_ref:candidate.source_revision_ref.clone(),
        focus_statement_ref:f.statement_ref.clone(),
        candidate_statement_ref:a.statement_ref.clone(),
        review_status:owned.current_status,
        antecedent_resolution:"REVIEW_REQUIRED",
        identity_admission_paid:false,external_qid_paid:false,
    })
}

/// Immutable, source-revalidated view of one persisted C4.11 candidate.
/// S29 workflow status is exposed as workflow status, not as identity proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NativeAntecedentReviewInspection {
    pub schema: &'static str,
    pub review_item_ref: String,
    pub proposal_ref: String,
    pub original_source_ref: String,
    pub source_revision_ref: String,
    pub document_ref: String,
    pub canonical_content_sha256: String,
    pub original_context_sha256: String,
    pub focus: NativeSourceMention,
    pub candidate_antecedent: NativeSourceSpan,
    pub before: Vec<NativeReviewSentence>,
    pub focus_statement: NativeReviewSentence,
    pub after: Vec<NativeReviewSentence>,
    pub s29_review_status: String,
    pub s29_receipt_commands: Vec<String>,
    pub candidate_only: bool,
    pub antecedent_resolution: &'static str,
    pub identity_admission_paid: bool,
    pub external_qid_paid: bool,
    pub claim_truth_paid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NativeReviewSentence {
    pub statement_ref: String,
    pub region_ref: String,
    pub start_char: u64,
    pub end_char: u64,
    pub literal: String,
}

/// Read the *original* SLR text and source statements, plus the actual S29
/// queue state, without creating review decisions, identity or new parse rows.
pub fn inspect_native_antecedent_review(
    config: &DatabaseConfig,
    review_item_ref: &str,
) -> Result<NativeAntecedentReviewInspection, NativeAntecedentReviewError> {
    if !nonempty(review_item_ref) || !review_item_ref.starts_with(
        "review-item:native-antecedent-candidate:sha256:"
    ) {
        return Err(NativeAntecedentReviewError::InvalidCandidate);
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let rows = client.query(
        r#"SELECT proposal_ref,original_source_ref,source_revision_ref,
                  document_ref,canonical_content_sha256,original_context_sha256,
                  focus_mention_ref,focus_candidate_ref,focus_statement_ref,
                  focus_region_ref,focus_start_char,focus_end_char,focus_surface,
                  candidate_statement_ref,candidate_region_ref,
                  candidate_start_char,candidate_end_char,candidate_surface,
                  candidate_only,semantic_authority_promoted,qid_admitted
           FROM semantic.native_antecedent_candidate_review
           WHERE review_item_ref=$1"#,
        &[&review_item_ref],
    )?;
    if rows.len()!=1 {return Err(NativeAntecedentReviewError::ImmutableConflict);}
    let row=&rows[0];
    let coords:Vec<i64>=[10_usize,11,15,16].iter()
        .map(|&i|row.get(i)).collect();
    if coords.iter().any(|&v|v<0)
        || !row.get::<_,bool>(18) || row.get::<_,bool>(19)
        || row.get::<_,bool>(20) {
        return Err(NativeAntecedentReviewError::ImmutableConflict);
    }
    let proposal_ref:String=row.get(0);
    let candidate=NativeAntecedentSourceCandidate {
        schema:SCHEMA.into(),
        original_context_sha256:row.get(5),
        original_source_ref:row.get(1),
        source_revision_ref:row.get(2),
        document_ref:row.get(3),
        canonical_content_sha256:row.get(4),
        focus:NativeSourceMention {
            mention_ref:row.get(6),candidate_ref:row.get(7),
            statement_ref:row.get(8),region_ref:row.get(9),
            start_char:coords[0] as u64,end_char:coords[1] as u64,
            surface:row.get(12),
        },
        antecedent_source_span:NativeSourceSpan {
            statement_ref:row.get(13),region_ref:row.get(14),
            start_char:coords[2] as u64,end_char:coords[3] as u64,
            surface:row.get(17),
        },
        candidate_only:true,same_source_span_geometry_checked:true,
        antecedent_resolution:"REVIEW_REQUIRED".into(),
        identity_admission:"NOT_PAID".into(),
        external_qid:"NOT_TESTED".into(),
        claim_truth_paid:false,semantic_authority_promoted:false,
        review_owner:"SLR_GOVERNED_ANTECEDENT_REVIEW".into(),
    };
    validate_native_antecedent_candidate(&candidate)?;
    if proposal_identity(&candidate)!=proposal_ref
        || review_item_ref!=format!("review-item:{proposal_ref}") {
        return Err(NativeAntecedentReviewError::ImmutableConflict);
    }
    let original_text=original_source_match(&mut client,&candidate)?;
    let rows=client.query(
        r#"SELECT r.region_ref,r.start_char,r.end_char,s.statement_ref,
                  s.literal_text,s.candidate_only,s.creates_semantic_authority,
                  s.applicability_promoted,s.claim_truth_promoted,
                  r.candidate_only,r.creates_semantic_authority,r.claim_truth_promoted
           FROM ingest.long_document_region r
           JOIN corpus.source_statement s
             ON s.source_revision_ref=r.source_revision_ref
            AND s.exact_span_ref=r.region_ref
            AND s.document_ref=$1
           WHERE r.source_revision_ref=$2 AND r.source_ref=$3
             AND r.region_kind='sentence'
           ORDER BY r.start_char,r.end_char,r.region_ref,s.statement_ref"#,
        &[&candidate.document_ref,&candidate.source_revision_ref,
          &candidate.original_source_ref],
    )?;
    let mut sentences=Vec::with_capacity(rows.len());
    let original_chars=original_text.chars().collect::<Vec<_>>();
    let mut last_end=0usize;
    for row in rows {
        let st:i64=row.get(1);
        let en:i64=row.get(2);
        let literal:String=row.get(4);
        if st<0 || en<=st || (en as usize)>original_chars.len()
            || st as usize<last_end
            || !row.get::<_,bool>(5) || row.get::<_,bool>(6)
            || row.get::<_,bool>(7) || row.get::<_,bool>(8)
            || !row.get::<_,bool>(9) || row.get::<_,bool>(10)
            || row.get::<_,bool>(11)
            || original_chars[st as usize..en as usize]
                .iter().collect::<String>() != literal {
            return Err(NativeAntecedentReviewError::OriginalSourceMismatch);
        }
        last_end=en as usize;
        sentences.push(NativeReviewSentence {
            region_ref:row.get(0),start_char:st as u64,
            end_char:en as u64,statement_ref:row.get(3),
            literal,
        });
    }
    let positions=sentences.iter().enumerate()
        .filter_map(|(i,s)| {
            if s.statement_ref==candidate.focus.statement_ref
                && s.region_ref==candidate.focus.region_ref {
                Some(i)
            } else {None}
        }).collect::<Vec<_>>();
    if positions.len()!=1 {return Err(NativeAntecedentReviewError::OriginalSourceMismatch);}
    let focus_idx=positions[0];
    let target=&candidate.antecedent_source_span;
    let target_found=sentences.iter().take(focus_idx+1).any(|s|
        s.statement_ref==target.statement_ref
        && s.region_ref==target.region_ref
        && s.start_char<=target.start_char
        && s.end_char>=target.end_char
    );
    if !target_found {return Err(NativeAntecedentReviewError::OriginalSourceMismatch);}
    let item=crate::load_review_item(config,review_item_ref)?
        .ok_or(NativeAntecedentReviewError::ImmutableConflict)?;
    let wanted_provenance=[
        format!("original-context-sha256:{}",candidate.original_context_sha256),
        candidate.focus.mention_ref.clone(),candidate.focus.candidate_ref.clone()
    ];
    let wanted_sources=[
        candidate.source_revision_ref.clone(),candidate.document_ref.clone(),
        candidate.focus.statement_ref.clone(),target.statement_ref.clone()
    ];
    if item.item_kind!=ReviewItemKind::Observation
        || item.semantic_ref!=proposal_ref
        || !item.candidate_only || item.creates_semantic_authority
        || item.applicability_promoted || item.claim_truth_promoted
        || !same_set(&item.provenance_refs,&wanted_provenance)
        || !same_set(&item.source_refs,&wanted_sources)
        || !same_set(&item.affected_consumer_refs,
                     &[candidate.focus.mention_ref.clone()])
    {return Err(NativeAntecedentReviewError::ImmutableConflict);}
    // Audit history is stable-reference sorted, NOT falsely timestamp ordered.
    let receipt_rows=client.query(
        r#"SELECT command_ref,semantic_ref,candidate_only,
                  creates_semantic_authority,applicability_promoted,
                  claim_truth_promoted
           FROM semantic.review_receipt
           WHERE review_item_ref=$1 ORDER BY command_ref"#,
        &[&review_item_ref],
    )?;
    let mut s29_receipt_commands=Vec::with_capacity(receipt_rows.len());
    for r in receipt_rows {
        if r.get::<_,String>(1)!=proposal_ref
            || !r.get::<_,bool>(2) || r.get::<_,bool>(3)
            || r.get::<_,bool>(4) || r.get::<_,bool>(5) {
            return Err(NativeAntecedentReviewError::ImmutableConflict);
        }
        s29_receipt_commands.push(r.get(0));
    }
    let before=sentences[focus_idx.saturating_sub(4)..focus_idx].to_vec();
    let focus_statement=sentences[focus_idx].clone();
    let after=sentences[focus_idx+1..(focus_idx+3).min(sentences.len())].to_vec();
    Ok(NativeAntecedentReviewInspection{
        schema:"slr-native-antecedent-s29-inspection-v1",
        review_item_ref:review_item_ref.into(),proposal_ref,
        original_source_ref:candidate.original_source_ref,
        source_revision_ref:candidate.source_revision_ref,
        document_ref:candidate.document_ref,
        canonical_content_sha256:candidate.canonical_content_sha256,
        original_context_sha256:candidate.original_context_sha256,
        focus:candidate.focus,candidate_antecedent:target.clone(),
        before,focus_statement,after,
        s29_review_status:format!("{:?}",item.current_status),
        s29_receipt_commands,candidate_only:true,
        antecedent_resolution:"REVIEW_REQUIRED",
        identity_admission_paid:false,external_qid_paid:false,
        claim_truth_paid:false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn example() -> NativeAntecedentSourceCandidate {
        NativeAntecedentSourceCandidate {
            schema:SCHEMA.into(),original_context_sha256:"a".repeat(64),
            original_source_ref:"source:original".into(),
            source_revision_ref:"source-revision:original".into(),
            document_ref:format!("document:sha256:{}","b".repeat(64)),
            canonical_content_sha256:"b".repeat(64),
            focus:NativeSourceMention {
                mention_ref:"mention:he".into(),candidate_ref:"candidate:he".into(),
                statement_ref:"statement:23".into(),region_ref:"region:23".into(),
                start_char:4534,end_char:4536,surface:"he".into(),
            },
            antecedent_source_span:NativeSourceSpan{
                statement_ref:"statement:22".into(),region_ref:"region:22".into(),
                start_char:4310,end_char:4314,surface:"Bush".into(),
            },
            candidate_only:true,same_source_span_geometry_checked:true,
            antecedent_resolution:"REVIEW_REQUIRED".into(),
            identity_admission:"NOT_PAID".into(),external_qid:"NOT_TESTED".into(),
            claim_truth_paid:false,semantic_authority_promoted:false,
            review_owner:"SLR_GOVERNED_ANTECEDENT_REVIEW".into(),
        }
    }
    #[test]
    fn exact_gwb_candidate_is_a_pending_review_only() {
        let c=example();
        validate_native_antecedent_candidate(&c).unwrap();
        assert_eq!(c.focus.start_char,4534);
        assert_eq!(c.antecedent_source_span.start_char,4310);
        assert!(proposal_identity(&c).starts_with("native-antecedent-candidate:sha256:"));
    }
    #[test]
    fn semantic_promotion_from_source_span_is_rejected() {
        let mut c=example();c.identity_admission="PAID".into();
        assert!(validate_native_antecedent_candidate(&c).is_err());
        c=example();c.external_qid="Q23505".into();
        assert!(validate_native_antecedent_candidate(&c).is_err());
        c=example();c.semantic_authority_promoted=true;
        assert!(validate_native_antecedent_candidate(&c).is_err());
    }
    #[test]
    fn foreign_or_future_antecedent_rejected() {
        let mut c=example();c.antecedent_source_span.start_char=4536;
        c.antecedent_source_span.end_char=4540;
        assert!(validate_native_antecedent_candidate(&c).is_err());
        c=example();c.original_context_sha256="malformed".into();
        assert!(validate_native_antecedent_candidate(&c).is_err());
    }
    #[test]
    fn proposal_key_distinguishes_candidate_positions() {
        let c=example();let mut d=c.clone();d.antecedent_source_span.start_char=4309;
        assert_ne!(proposal_identity(&c),proposal_identity(&d));
    }
}
