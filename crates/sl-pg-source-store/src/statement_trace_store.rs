use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{DatabaseConfig, SourceStatementEnvelope, StatementOrigin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementObservationDisposition {
    Candidate,
    ParseReviewed,
    SemanticallyAdmitted,
    Rejected,
    Abstained,
    Qualified,
}

impl StatementObservationDisposition {
    fn as_db(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::ParseReviewed => "parse_reviewed",
            Self::SemanticallyAdmitted => "semantically_admitted",
            Self::Rejected => "rejected",
            Self::Abstained => "abstained",
            Self::Qualified => "qualified",
        }
    }

    fn from_db(value: &str) -> Result<Self, StatementTraceStoreError> {
        match value {
            "candidate" => Ok(Self::Candidate),
            "parse_reviewed" => Ok(Self::ParseReviewed),
            "semantically_admitted" => Ok(Self::SemanticallyAdmitted),
            "rejected" => Ok(Self::Rejected),
            "abstained" => Ok(Self::Abstained),
            "qualified" => Ok(Self::Qualified),
            _ => Err(StatementTraceStoreError::UnknownDisposition(value.to_owned())),
        }
    }
}

fn origin_as_db(origin: StatementOrigin) -> &'static str {
    match origin {
        StatementOrigin::InitialIntake => "initial_intake",
        StatementOrigin::ResearchReentry => "research_reentry",
    }
}

fn origin_from_db(value: &str) -> Result<StatementOrigin, StatementTraceStoreError> {
    match value {
        "initial_intake" => Ok(StatementOrigin::InitialIntake),
        "research_reentry" => Ok(StatementOrigin::ResearchReentry),
        _ => Err(StatementTraceStoreError::UnknownOrigin(value.to_owned())),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedSourceStatement {
    pub statement_ref: String,
    pub document_ref: String,
    pub source_revision_ref: String,
    pub exact_span_ref: String,
    pub literal_text: String,
    pub origin: StatementOrigin,
    pub statement_sha256: [u8; 32],
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementObservationLink {
    pub link_ref: String,
    pub statement_ref: String,
    pub candidate_pnf_ref: String,
    pub observation_ref: String,
    pub parse_review_ref: Option<String>,
    pub admission_receipt_ref: Option<String>,
    pub disposition: StatementObservationDisposition,
    pub qualification_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl StatementObservationLink {
    pub fn validate(&self) -> Result<(), StatementTraceStoreError> {
        for (name, value) in [
            ("link_ref", self.link_ref.as_str()),
            ("statement_ref", self.statement_ref.as_str()),
            ("candidate_pnf_ref", self.candidate_pnf_ref.as_str()),
            ("observation_ref", self.observation_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(StatementTraceStoreError::EmptyCoordinate(name));
            }
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(StatementTraceStoreError::PromotionNotAllowed);
        }
        if matches!(self.disposition, StatementObservationDisposition::SemanticallyAdmitted)
            && self
                .admission_receipt_ref
                .as_deref()
                .map_or(true, |value| value.trim().is_empty())
        {
            return Err(StatementTraceStoreError::MissingAdmissionReceipt);
        }
        if matches!(self.disposition, StatementObservationDisposition::Qualified)
            && self
                .qualification_ref
                .as_deref()
                .map_or(true, |value| value.trim().is_empty())
        {
            return Err(StatementTraceStoreError::MissingQualification);
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum StatementTraceStoreError {
    #[error("required statement trace coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("statement/observation trace crossed a non-promotion boundary")]
    PromotionNotAllowed,
    #[error("statement span does not belong to the declared document")]
    SpanDocumentMismatch,
    #[error("source revision does not belong to the declared document")]
    RevisionDocumentMismatch,
    #[error("statement literal text does not match the exact persisted source span")]
    LiteralTextMismatch,
    #[error("semantically admitted link requires an admission receipt")]
    MissingAdmissionReceipt,
    #[error("qualified link requires a qualification reference")]
    MissingQualification,
    #[error("statement observation link references a missing statement")]
    MissingStatement,
    #[error("existing statement identity conflicts with requested immutable content")]
    ExistingStatementConflict,
    #[error("existing statement-observation link conflicts with requested coordinates")]
    ExistingLinkConflict,
    #[error("unknown statement origin: {0}")]
    UnknownOrigin(String),
    #[error("unknown statement-observation disposition: {0}")]
    UnknownDisposition(String),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
}

pub fn canonical_statement_ref(statement: &SourceStatementEnvelope) -> String {
    let digest = statement_digest(statement);
    format!("statement:sha256:{}", hex(&digest))
}

pub fn canonical_statement_observation_link_ref(
    statement_ref: &str,
    candidate_pnf_ref: &str,
    observation_ref: &str,
) -> String {
    let digest = digest_parts(&[
        "statement-observation-link:v1",
        statement_ref,
        candidate_pnf_ref,
        observation_ref,
    ]);
    format!("statement-observation-link:sha256:{}", hex(&digest))
}

/// Additive normalized schema owned by M12.2.
///
/// This is deliberately explicit rather than auto-running during ordinary
/// persistence. Deployments can run it under their normal migration control.
pub fn install_statement_trace_schema(
    config: &DatabaseConfig,
) -> Result<(), StatementTraceStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(
        r#"
        CREATE TABLE IF NOT EXISTS corpus.source_statement (
          statement_ref TEXT PRIMARY KEY,
          document_ref TEXT NOT NULL,
          source_revision_ref TEXT NOT NULL,
          exact_span_ref TEXT NOT NULL,
          literal_text TEXT NOT NULL,
          origin_ref TEXT NOT NULL,
          statement_sha256 BYTEA NOT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
          UNIQUE (source_revision_ref, exact_span_ref, statement_sha256)
        );

        CREATE INDEX IF NOT EXISTS source_statement_document_idx
          ON corpus.source_statement (document_ref, source_revision_ref);
        CREATE INDEX IF NOT EXISTS source_statement_span_idx
          ON corpus.source_statement (exact_span_ref);

        CREATE TABLE IF NOT EXISTS pnf.statement_observation_link (
          link_ref TEXT PRIMARY KEY,
          statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref),
          candidate_pnf_ref TEXT NOT NULL,
          observation_ref TEXT NOT NULL,
          parse_review_ref TEXT NULL,
          admission_receipt_ref TEXT NULL,
          disposition_ref TEXT NOT NULL,
          qualification_ref TEXT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
          UNIQUE (statement_ref, candidate_pnf_ref, observation_ref)
        );

        CREATE INDEX IF NOT EXISTS statement_observation_statement_idx
          ON pnf.statement_observation_link (statement_ref);
        CREATE INDEX IF NOT EXISTS statement_observation_observation_idx
          ON pnf.statement_observation_link (observation_ref);
        "#,
    )?;
    Ok(())
}

pub fn persist_source_statement(
    config: &DatabaseConfig,
    statement: &SourceStatementEnvelope,
) -> Result<PersistedSourceStatement, StatementTraceStoreError> {
    statement
        .validate()
        .map_err(|_| StatementTraceStoreError::PromotionNotAllowed)?;

    let expected_ref = canonical_statement_ref(statement);
    if statement.statement_ref != expected_ref {
        return Err(StatementTraceStoreError::ExistingStatementConflict);
    }

    let mut client = Client::connect(config.database_url(), NoTls)?;
    require_source_ancestry(&mut client, statement)?;

    let digest = statement_digest(statement);
    client.execute(
        r#"
        INSERT INTO corpus.source_statement
          (statement_ref, document_ref, source_revision_ref, exact_span_ref,
           literal_text, origin_ref, statement_sha256,
           candidate_only, creates_semantic_authority,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,true,false,false,false)
        ON CONFLICT (statement_ref) DO NOTHING
        "#,
        &[
            &statement.statement_ref,
            &statement.document_ref,
            &statement.source_revision_ref,
            &statement.span.span_ref,
            &statement.literal_text,
            &origin_as_db(statement.origin),
            &&digest[..],
        ],
    )?;

    let persisted = load_source_statement_with_client(&mut client, &statement.statement_ref)?
        .ok_or(StatementTraceStoreError::ExistingStatementConflict)?;

    if persisted.document_ref != statement.document_ref
        || persisted.source_revision_ref != statement.source_revision_ref
        || persisted.exact_span_ref != statement.span.span_ref
        || persisted.literal_text != statement.literal_text
        || persisted.origin != statement.origin
        || persisted.statement_sha256 != digest
    {
        return Err(StatementTraceStoreError::ExistingStatementConflict);
    }
    Ok(persisted)
}

pub fn persist_statement_observation_link(
    config: &DatabaseConfig,
    link: &StatementObservationLink,
) -> Result<StatementObservationLink, StatementTraceStoreError> {
    link.validate()?;
    let expected = canonical_statement_observation_link_ref(
        &link.statement_ref,
        &link.candidate_pnf_ref,
        &link.observation_ref,
    );
    if link.link_ref != expected {
        return Err(StatementTraceStoreError::ExistingLinkConflict);
    }

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let statement_exists: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM corpus.source_statement WHERE statement_ref=$1)",
            &[&link.statement_ref],
        )?
        .get(0);
    if !statement_exists {
        return Err(StatementTraceStoreError::MissingStatement);
    }

    client.execute(
        r#"
        INSERT INTO pnf.statement_observation_link
          (link_ref, statement_ref, candidate_pnf_ref, observation_ref,
           parse_review_ref, admission_receipt_ref, disposition_ref,
           qualification_ref, candidate_only, creates_semantic_authority,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,true,false,false,false)
        ON CONFLICT (link_ref) DO NOTHING
        "#,
        &[
            &link.link_ref,
            &link.statement_ref,
            &link.candidate_pnf_ref,
            &link.observation_ref,
            &link.parse_review_ref,
            &link.admission_receipt_ref,
            &link.disposition.as_db(),
            &link.qualification_ref,
        ],
    )?;

    let persisted = load_statement_observation_link_with_client(&mut client, &link.link_ref)?
        .ok_or(StatementTraceStoreError::ExistingLinkConflict)?;
    if &persisted != link {
        return Err(StatementTraceStoreError::ExistingLinkConflict);
    }
    Ok(persisted)
}

pub fn load_source_statement(
    config: &DatabaseConfig,
    statement_ref: &str,
) -> Result<Option<PersistedSourceStatement>, StatementTraceStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    load_source_statement_with_client(&mut client, statement_ref)
}

pub fn load_statement_observation_links_for_statement(
    config: &DatabaseConfig,
    statement_ref: &str,
) -> Result<Vec<StatementObservationLink>, StatementTraceStoreError> {
    load_links(config, "statement_ref", statement_ref)
}

pub fn load_statement_observation_links_for_observation(
    config: &DatabaseConfig,
    observation_ref: &str,
) -> Result<Vec<StatementObservationLink>, StatementTraceStoreError> {
    load_links(config, "observation_ref", observation_ref)
}

fn load_links(
    config: &DatabaseConfig,
    coordinate: &str,
    value: &str,
) -> Result<Vec<StatementObservationLink>, StatementTraceStoreError> {
    let sql = match coordinate {
        "statement_ref" => {
            "SELECT link_ref, statement_ref, candidate_pnf_ref, observation_ref,              parse_review_ref, admission_receipt_ref, disposition_ref, qualification_ref,              candidate_only, creates_semantic_authority, applicability_promoted, claim_truth_promoted              FROM pnf.statement_observation_link WHERE statement_ref=$1 ORDER BY link_ref"
        }
        "observation_ref" => {
            "SELECT link_ref, statement_ref, candidate_pnf_ref, observation_ref,              parse_review_ref, admission_receipt_ref, disposition_ref, qualification_ref,              candidate_only, creates_semantic_authority, applicability_promoted, claim_truth_promoted              FROM pnf.statement_observation_link WHERE observation_ref=$1 ORDER BY link_ref"
        }
        _ => unreachable!("internal fixed statement trace query"),
    };
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client
        .query(sql, &[&value])?
        .into_iter()
        .map(row_to_link)
        .collect()
}

fn load_source_statement_with_client(
    client: &mut Client,
    statement_ref: &str,
) -> Result<Option<PersistedSourceStatement>, StatementTraceStoreError> {
    let row = client.query_opt(
        r#"
        SELECT statement_ref, document_ref, source_revision_ref, exact_span_ref,
               literal_text, origin_ref, statement_sha256,
               candidate_only, creates_semantic_authority,
               applicability_promoted, claim_truth_promoted
        FROM corpus.source_statement
        WHERE statement_ref=$1
        "#,
        &[&statement_ref],
    )?;
    row.map(|row| {
        let digest: Vec<u8> = row.get(6);
        let digest: [u8; 32] = digest
            .try_into()
            .map_err(|_| StatementTraceStoreError::ExistingStatementConflict)?;
        Ok(PersistedSourceStatement {
            statement_ref: row.get(0),
            document_ref: row.get(1),
            source_revision_ref: row.get(2),
            exact_span_ref: row.get(3),
            literal_text: row.get(4),
            origin: origin_from_db(row.get::<_, String>(5).as_str())?,
            statement_sha256: digest,
            candidate_only: row.get(7),
            creates_semantic_authority: row.get(8),
            applicability_promoted: row.get(9),
            claim_truth_promoted: row.get(10),
        })
    })
    .transpose()
}

fn load_statement_observation_link_with_client(
    client: &mut Client,
    link_ref: &str,
) -> Result<Option<StatementObservationLink>, StatementTraceStoreError> {
    client
        .query_opt(
            r#"
            SELECT link_ref, statement_ref, candidate_pnf_ref, observation_ref,
                   parse_review_ref, admission_receipt_ref, disposition_ref, qualification_ref,
                   candidate_only, creates_semantic_authority,
                   applicability_promoted, claim_truth_promoted
            FROM pnf.statement_observation_link
            WHERE link_ref=$1
            "#,
            &[&link_ref],
        )?
        .map(row_to_link)
        .transpose()
}

fn row_to_link(row: postgres::Row) -> Result<StatementObservationLink, StatementTraceStoreError> {
    let link = StatementObservationLink {
        link_ref: row.get(0),
        statement_ref: row.get(1),
        candidate_pnf_ref: row.get(2),
        observation_ref: row.get(3),
        parse_review_ref: row.get(4),
        admission_receipt_ref: row.get(5),
        disposition: StatementObservationDisposition::from_db(row.get::<_, String>(6).as_str())?,
        qualification_ref: row.get(7),
        candidate_only: row.get(8),
        creates_semantic_authority: row.get(9),
        applicability_promoted: row.get(10),
        claim_truth_promoted: row.get(11),
    };
    link.validate()?;
    Ok(link)
}

fn require_source_ancestry(
    client: &mut Client,
    statement: &SourceStatementEnvelope,
) -> Result<(), StatementTraceStoreError> {
    let revision_matches: bool = client
        .query_one(
            r#"
            SELECT EXISTS (
              SELECT 1 FROM corpus.external_source_revision
              WHERE external_source_revision_ref=$1 AND document_ref=$2
            )
            "#,
            &[&statement.source_revision_ref, &statement.document_ref],
        )?
        .get(0);
    if !revision_matches {
        return Err(StatementTraceStoreError::RevisionDocumentMismatch);
    }

    let span = client.query_opt(
        r#"
        SELECT start_char, end_char
        FROM corpus.span
        WHERE span_ref=$1 AND document_ref=$2
        "#,
        &[&statement.span.span_ref, &statement.document_ref],
    )?;
    let Some(span) = span else {
        return Err(StatementTraceStoreError::SpanDocumentMismatch);
    };
    let start: i32 = span.get(0);
    let end: i32 = span.get(1);
    if start < 0
        || end < 0
        || start as u32 != statement.span.start_char
        || end as u32 != statement.span.end_char
    {
        return Err(StatementTraceStoreError::SpanDocumentMismatch);
    }

    let source_text: Option<String> = client
        .query_one(
            r#"
            SELECT convert_from(substring(c.payload FROM (s.start_char + 1) FOR (s.end_char - s.start_char)), 'UTF8')
            FROM corpus.span s
            JOIN corpus.document d ON d.document_ref=s.document_ref
            JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
            WHERE s.span_ref=$1 AND s.document_ref=$2
            "#,
            &[&statement.span.span_ref, &statement.document_ref],
        )?
        .get(0);
    if source_text.as_deref() != Some(statement.literal_text.as_str()) {
        return Err(StatementTraceStoreError::LiteralTextMismatch);
    }
    Ok(())
}

fn statement_digest(statement: &SourceStatementEnvelope) -> [u8; 32] {
    digest_parts(&[
        "source-statement:v1",
        &statement.document_ref,
        &statement.source_revision_ref,
        &statement.span.span_ref,
        &statement.span.start_char.to_string(),
        &statement.span.end_char.to_string(),
        &statement.literal_text,
    ])
}

fn digest_parts(parts: &[&str]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    hasher.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExactSourceSpan;

    fn statement(origin: StatementOrigin) -> SourceStatementEnvelope {
        let mut statement = SourceStatementEnvelope {
            statement_ref: String::new(),
            document_ref: "document:1".into(),
            source_revision_ref: "revision:1".into(),
            span: ExactSourceSpan {
                span_ref: "span:1".into(),
                start_char: 3,
                end_char: 17,
            },
            literal_text: "literal source".into(),
            origin,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        statement.statement_ref = canonical_statement_ref(&statement);
        statement
    }

    #[test]
    fn statement_identity_ignores_parser_run_and_origin() {
        let intake = statement(StatementOrigin::InitialIntake);
        let mut reentry = statement(StatementOrigin::ResearchReentry);
        reentry.statement_ref = canonical_statement_ref(&reentry);
        assert_eq!(intake.statement_ref, reentry.statement_ref);
    }

    #[test]
    fn observation_link_identity_is_structural() {
        let a = canonical_statement_observation_link_ref(
            "statement:x",
            "candidate:y",
            "observation:z",
        );
        let b = canonical_statement_observation_link_ref(
            "statement:x",
            "candidate:y",
            "observation:z",
        );
        assert_eq!(a, b);
    }

    #[test]
    fn admitted_link_requires_receipt_but_still_does_not_promote_truth() {
        let link = StatementObservationLink {
            link_ref: canonical_statement_observation_link_ref(
                "statement:x",
                "candidate:y",
                "observation:z",
            ),
            statement_ref: "statement:x".into(),
            candidate_pnf_ref: "candidate:y".into(),
            observation_ref: "observation:z".into(),
            parse_review_ref: Some("review:p".into()),
            admission_receipt_ref: None,
            disposition: StatementObservationDisposition::SemanticallyAdmitted,
            qualification_ref: None,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        assert!(matches!(
            link.validate(),
            Err(StatementTraceStoreError::MissingAdmissionReceipt)
        ));
    }
}
