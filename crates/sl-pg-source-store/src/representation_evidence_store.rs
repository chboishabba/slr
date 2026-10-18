use std::collections::BTreeSet;

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{load_legal_controversy_matter, DatabaseConfig, LegalControversyStoreError};

pub const SMITH_AFFECTED_PERSON_CONSUMER_REF: &str = "consumer:smith:affected-person";
pub const SMITH_INTERPRETIVE_QUERY_FAMILY_REF: &str =
    "query-family:understand-live-smith-interpretive-controversy";

const SMITH_REQUIRED_DISTINCTIONS: [&str; 5] = [
    "smith:distinction:purpose-connected-v-direct-assistance",
    "smith:distinction:joint-reasons-v-reported-party-submission",
    "smith:distinction:meaning-v-severance-partial-disapplication",
    "smith:distinction:opinion-level-provenance",
    "smith:distinction:open-residual-v-decided-merits",
];

pub fn smith_required_distinctions() -> BTreeSet<String> {
    SMITH_REQUIRED_DISTINCTIONS
        .into_iter()
        .map(str::to_owned)
        .collect()
}

pub const REPRESENTATION_EVIDENCE_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS consumer_projection;

CREATE TABLE IF NOT EXISTS consumer_projection.representation_evidence (
  representation_ref TEXT PRIMARY KEY,
  matter_ref TEXT NOT NULL,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref),
  kind_ref TEXT NOT NULL,
  consumer_ref TEXT NOT NULL,
  query_family_ref TEXT NOT NULL,
  content_text TEXT NOT NULL,
  content_sha256 TEXT NOT NULL,
  source_statement_refs TEXT[] NOT NULL,
  source_admission_receipt_refs TEXT[] NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS consumer_projection.reviewed_representation_admission (
  admission_receipt_ref TEXT PRIMARY KEY,
  representation_ref TEXT NOT NULL REFERENCES consumer_projection.representation_evidence(representation_ref),
  review_command_ref TEXT NOT NULL REFERENCES semantic.review_receipt(command_ref),
  reviewer_ref TEXT NOT NULL,
  consumer_ref TEXT NOT NULL,
  query_family_ref TEXT NOT NULL,
  accepted BOOLEAN NOT NULL CHECK (accepted),
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  UNIQUE (representation_ref, review_command_ref)
);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentationEvidenceKind {
    AccessibleDistinctionLedger,
    FullExactSourceBundle,
}

impl RepresentationEvidenceKind {
    fn as_db(self) -> &'static str {
        match self {
            Self::AccessibleDistinctionLedger => "accessible_distinction_ledger",
            Self::FullExactSourceBundle => "full_exact_source_bundle",
        }
    }

    fn from_db(value: &str) -> Result<Self, RepresentationEvidenceStoreError> {
        match value {
            "accessible_distinction_ledger" => Ok(Self::AccessibleDistinctionLedger),
            "full_exact_source_bundle" => Ok(Self::FullExactSourceBundle),
            other => Err(RepresentationEvidenceStoreError::UnknownKind(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationEvidence {
    pub representation_ref: String,
    pub matter_ref: String,
    pub controversy_ref: String,
    pub kind: RepresentationEvidenceKind,
    pub consumer_ref: String,
    pub query_family_ref: String,
    pub content_text: String,
    pub content_sha256: String,
    pub source_statement_refs: BTreeSet<String>,
    pub source_admission_receipt_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedRepresentationAdmission {
    pub admission_receipt_ref: String,
    pub representation_ref: String,
    pub review_command_ref: String,
    pub reviewer_ref: String,
    pub consumer_ref: String,
    pub query_family_ref: String,
    pub accepted: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum RepresentationEvidenceStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("legal controversy prerequisite failed: {0}")]
    LegalControversy(#[from] LegalControversyStoreError),
    #[error("empty representation evidence coordinate: {0}")]
    EmptyCoordinate(&'static str),
    #[error("unknown representation evidence kind: {0}")]
    UnknownKind(String),
    #[error("representation source statement not found: {0}")]
    MissingStatement(String),
    #[error("representation source statement crossed a non-promotion boundary: {0}")]
    InvalidStatementBoundary(String),
    #[error("representation source statement lacks a semantic admission receipt: {0}")]
    MissingSemanticAdmission(String),
    #[error("representation matter/controversy binding mismatch")]
    MatterBindingMismatch,
    #[error("representation review command does not accept the named representation")]
    ReviewCommandMismatch,
    #[error("persisted representation evidence conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("representation evidence not found: {0}")]
    NotFound(String),
}

fn digest_parts(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn canonical_vec(values: &BTreeSet<String>) -> Vec<String> {
    values.iter().cloned().collect()
}

fn set_from_vec(values: Vec<String>) -> BTreeSet<String> {
    values.into_iter().collect()
}

fn require_matter_binding(
    config: &DatabaseConfig,
    matter_ref: &str,
    controversy_ref: &str,
) -> Result<(), RepresentationEvidenceStoreError> {
    let matter = load_legal_controversy_matter(config, controversy_ref)?;
    if matter.matter_ref != matter_ref {
        return Err(RepresentationEvidenceStoreError::MatterBindingMismatch);
    }
    Ok(())
}

pub fn install_representation_evidence_schema(
    config: &DatabaseConfig,
) -> Result<(), RepresentationEvidenceStoreError> {
    require_matter_binding(config, "__schema_probe__", "__schema_probe__").ok();
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(REPRESENTATION_EVIDENCE_SCHEMA_SQL)?;
    Ok(())
}

fn source_backing(
    config: &DatabaseConfig,
    statement_refs: &BTreeSet<String>,
) -> Result<(Vec<(String, String)>, BTreeSet<String>), RepresentationEvidenceStoreError> {
    if statement_refs.is_empty() {
        return Err(RepresentationEvidenceStoreError::EmptyCoordinate("source_statement_refs"));
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut statements = Vec::new();
    let mut admissions = BTreeSet::new();
    for statement_ref in statement_refs {
        let row = client.query_opt(
            "SELECT literal_text,candidate_only,creates_semantic_authority,applicability_promoted,claim_truth_promoted FROM corpus.source_statement WHERE statement_ref=$1",
            &[statement_ref],
        )?.ok_or_else(|| RepresentationEvidenceStoreError::MissingStatement(statement_ref.clone()))?;
        let candidate_only: bool = row.get(1);
        let creates_semantic_authority: bool = row.get(2);
        let applicability_promoted: bool = row.get(3);
        let claim_truth_promoted: bool = row.get(4);
        if !candidate_only || creates_semantic_authority || applicability_promoted || claim_truth_promoted {
            return Err(RepresentationEvidenceStoreError::InvalidStatementBoundary(statement_ref.clone()));
        }
        let admission_rows = client.query(
            "SELECT admission_receipt_ref FROM pnf.statement_observation_link WHERE statement_ref=$1 AND disposition_ref='semantically_admitted' AND admission_receipt_ref IS NOT NULL ORDER BY admission_receipt_ref",
            &[statement_ref],
        )?;
        if admission_rows.is_empty() {
            return Err(RepresentationEvidenceStoreError::MissingSemanticAdmission(statement_ref.clone()));
        }
        for admission in admission_rows {
            admissions.insert(admission.get::<_, String>(0));
        }
        statements.push((statement_ref.clone(), row.get::<_, String>(0)));
    }
    Ok((statements, admissions))
}

fn representation_content(
    kind: RepresentationEvidenceKind,
    statements: &[(String, String)],
) -> String {
    match kind {
        RepresentationEvidenceKind::FullExactSourceBundle => statements
            .iter()
            .map(|(statement_ref, text)| format!("[{statement_ref}]\n{text}"))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n"),
        RepresentationEvidenceKind::AccessibleDistinctionLedger => {
            let distinctions = SMITH_REQUIRED_DISTINCTIONS.join("\n");
            let sources = statements
                .iter()
                .map(|(statement_ref, _)| statement_ref.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "consumer={SMITH_AFFECTED_PERSON_CONSUMER_REF}\nquery={SMITH_INTERPRETIVE_QUERY_FAMILY_REF}\nrequired_distinctions:\n{distinctions}\nsource_statements:\n{sources}"
            )
        }
    }
}

pub fn materialize_smith_representation_evidence(
    config: &DatabaseConfig,
    matter_ref: &str,
    controversy_ref: &str,
    kind: RepresentationEvidenceKind,
    statement_refs: BTreeSet<String>,
) -> Result<RepresentationEvidence, RepresentationEvidenceStoreError> {
    require_matter_binding(config, matter_ref, controversy_ref)?;
    let (statements, source_admissions) = source_backing(config, &statement_refs)?;
    let content_text = representation_content(kind, &statements);
    let content_sha256 = format!("sha256:{}", digest_parts(&[&content_text]));
    let statement_key = statement_refs.iter().cloned().collect::<Vec<_>>().join("\u{1f}");
    let admission_key = source_admissions.iter().cloned().collect::<Vec<_>>().join("\u{1f}");
    let representation_ref = format!(
        "representation-evidence:smith-hca37:sha256:{}",
        digest_parts(&[
            matter_ref,
            controversy_ref,
            kind.as_db(),
            SMITH_AFFECTED_PERSON_CONSUMER_REF,
            SMITH_INTERPRETIVE_QUERY_FAMILY_REF,
            &content_sha256,
            &statement_key,
            &admission_key,
        ])
    );
    let value = RepresentationEvidence {
        representation_ref: representation_ref.clone(),
        matter_ref: matter_ref.to_owned(),
        controversy_ref: controversy_ref.to_owned(),
        kind,
        consumer_ref: SMITH_AFFECTED_PERSON_CONSUMER_REF.into(),
        query_family_ref: SMITH_INTERPRETIVE_QUERY_FAMILY_REF.into(),
        content_text,
        content_sha256,
        source_statement_refs: statement_refs,
        source_admission_receipt_refs: source_admissions,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    install_representation_evidence_schema(config)?;
    let source_statements = canonical_vec(&value.source_statement_refs);
    let source_admissions = canonical_vec(&value.source_admission_receipt_refs);
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO consumer_projection.representation_evidence
           (representation_ref,matter_ref,controversy_ref,kind_ref,consumer_ref,query_family_ref,
            content_text,content_sha256,source_statement_refs,source_admission_receipt_refs,
            candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,TRUE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(representation_ref) DO NOTHING"#,
        &[&value.representation_ref,&value.matter_ref,&value.controversy_ref,&value.kind.as_db(),
          &value.consumer_ref,&value.query_family_ref,&value.content_text,&value.content_sha256,
          &source_statements,&source_admissions],
    )?;
    let reopened = load_representation_evidence(config, &representation_ref)?;
    if reopened != value {
        return Err(RepresentationEvidenceStoreError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_representation_evidence(
    config: &DatabaseConfig,
    representation_ref: &str,
) -> Result<RepresentationEvidence, RepresentationEvidenceStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT matter_ref,controversy_ref,kind_ref,consumer_ref,query_family_ref,content_text,
                  content_sha256,source_statement_refs,source_admission_receipt_refs,candidate_only,
                  creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted
           FROM consumer_projection.representation_evidence WHERE representation_ref=$1"#,
        &[&representation_ref],
    )?.ok_or_else(|| RepresentationEvidenceStoreError::NotFound(representation_ref.to_owned()))?;
    let value = RepresentationEvidence {
        representation_ref: representation_ref.to_owned(),
        matter_ref: row.get(0),
        controversy_ref: row.get(1),
        kind: RepresentationEvidenceKind::from_db(&row.get::<_, String>(2))?,
        consumer_ref: row.get(3),
        query_family_ref: row.get(4),
        content_text: row.get(5),
        content_sha256: row.get(6),
        source_statement_refs: set_from_vec(row.get(7)),
        source_admission_receipt_refs: set_from_vec(row.get(8)),
        candidate_only: row.get(9),
        creates_semantic_authority: row.get(10),
        creates_legal_authority: row.get(11),
        applicability_promoted: row.get(12),
        claim_truth_promoted: row.get(13),
    };
    require_matter_binding(config, &value.matter_ref, &value.controversy_ref)?;
    if !value.candidate_only || value.creates_semantic_authority || value.creates_legal_authority
        || value.applicability_promoted || value.claim_truth_promoted
        || value.source_statement_refs.is_empty() || value.source_admission_receipt_refs.is_empty()
        || value.content_text.trim().is_empty()
        || value.content_sha256 != format!("sha256:{}", digest_parts(&[&value.content_text]))
    {
        return Err(RepresentationEvidenceStoreError::ExistingRowConflict);
    }
    Ok(value)
}

pub fn persist_reviewed_representation_admission(
    config: &DatabaseConfig,
    representation_ref: &str,
    review_command_ref: &str,
) -> Result<ReviewedRepresentationAdmission, RepresentationEvidenceStoreError> {
    let representation = load_representation_evidence(config, representation_ref)?;
    install_representation_evidence_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let review = client.query_opt(
        "SELECT semantic_ref,reviewer_ref,action_ref,effect_ref,effect_value_ref,candidate_only,creates_semantic_authority,applicability_promoted,claim_truth_promoted FROM semantic.review_receipt WHERE command_ref=$1",
        &[&review_command_ref],
    )?.ok_or(RepresentationEvidenceStoreError::ReviewCommandMismatch)?;
    let semantic_ref: String = review.get(0);
    let reviewer_ref: String = review.get(1);
    let action_ref: String = review.get(2);
    let effect_ref: String = review.get(3);
    let effect_value_ref: Option<String> = review.get(4);
    if semantic_ref != representation_ref
        || action_ref != "accept"
        || effect_ref != "status_changed"
        || effect_value_ref.as_deref() != Some("accepted")
        || !review.get::<_, bool>(5)
        || review.get::<_, bool>(6)
        || review.get::<_, bool>(7)
        || review.get::<_, bool>(8)
    {
        return Err(RepresentationEvidenceStoreError::ReviewCommandMismatch);
    }
    let admission_receipt_ref = format!(
        "representation-admission:sha256:{}",
        digest_parts(&[
            representation_ref,
            review_command_ref,
            &reviewer_ref,
            &representation.consumer_ref,
            &representation.query_family_ref,
        ])
    );
    let value = ReviewedRepresentationAdmission {
        admission_receipt_ref: admission_receipt_ref.clone(),
        representation_ref: representation_ref.to_owned(),
        review_command_ref: review_command_ref.to_owned(),
        reviewer_ref,
        consumer_ref: representation.consumer_ref.clone(),
        query_family_ref: representation.query_family_ref.clone(),
        accepted: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    client.execute(
        r#"INSERT INTO consumer_projection.reviewed_representation_admission
           (admission_receipt_ref,representation_ref,review_command_ref,reviewer_ref,consumer_ref,
            query_family_ref,accepted,candidate_only,creates_semantic_authority,creates_legal_authority,
            applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,TRUE,TRUE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(admission_receipt_ref) DO NOTHING"#,
        &[&value.admission_receipt_ref,&value.representation_ref,&value.review_command_ref,
          &value.reviewer_ref,&value.consumer_ref,&value.query_family_ref],
    )?;
    let reopened = load_reviewed_representation_admission(config, &admission_receipt_ref)?;
    if reopened != value {
        return Err(RepresentationEvidenceStoreError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_reviewed_representation_admission(
    config: &DatabaseConfig,
    admission_receipt_ref: &str,
) -> Result<ReviewedRepresentationAdmission, RepresentationEvidenceStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT representation_ref,review_command_ref,reviewer_ref,consumer_ref,query_family_ref,
                  accepted,candidate_only,creates_semantic_authority,creates_legal_authority,
                  applicability_promoted,claim_truth_promoted
           FROM consumer_projection.reviewed_representation_admission WHERE admission_receipt_ref=$1"#,
        &[&admission_receipt_ref],
    )?.ok_or_else(|| RepresentationEvidenceStoreError::NotFound(admission_receipt_ref.to_owned()))?;
    let value = ReviewedRepresentationAdmission {
        admission_receipt_ref: admission_receipt_ref.to_owned(),
        representation_ref: row.get(0),
        review_command_ref: row.get(1),
        reviewer_ref: row.get(2),
        consumer_ref: row.get(3),
        query_family_ref: row.get(4),
        accepted: row.get(5),
        candidate_only: row.get(6),
        creates_semantic_authority: row.get(7),
        creates_legal_authority: row.get(8),
        applicability_promoted: row.get(9),
        claim_truth_promoted: row.get(10),
    };
    if !value.accepted || !value.candidate_only || value.creates_semantic_authority
        || value.creates_legal_authority || value.applicability_promoted || value.claim_truth_promoted
    {
        return Err(RepresentationEvidenceStoreError::ExistingRowConflict);
    }
    let representation = load_representation_evidence(config, &value.representation_ref)?;
    if representation.consumer_ref != value.consumer_ref
        || representation.query_family_ref != value.query_family_ref
    {
        return Err(RepresentationEvidenceStoreError::ExistingRowConflict);
    }
    Ok(value)
}
