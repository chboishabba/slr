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
    pub origins: Vec<StatementOrigin>,
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
    pub parser_receipt_ref: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationEventLink {
    pub link_ref: String,
    pub observation_ref: String,
    pub event_ref: String,
    pub assembly_receipt_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl ObservationEventLink {
    pub fn validate(&self) -> Result<(), StatementTraceStoreError> {
        for (name, value) in [
            ("link_ref", self.link_ref.as_str()),
            ("observation_ref", self.observation_ref.as_str()),
            ("event_ref", self.event_ref.as_str()),
            ("assembly_receipt_ref", self.assembly_receipt_ref.as_str()),
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
    #[error("existing trace link conflicts with requested coordinates")]
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

pub fn canonical_observation_event_link_ref(
    observation_ref: &str,
    event_ref: &str,
) -> String {
    let digest = digest_parts(&[
        "observation-event-link:v1",
        observation_ref,
        event_ref,
    ]);
    format!("observation-event-link:sha256:{}", hex(&digest))
}

/// Additive normalized M12.2 schema.
///
/// Statement identity is structural. Initial-intake/research-reentry are
/// separate origin rows, so seeing the same statement through a second route
/// does not manufacture a second statement identity.
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
          statement_sha256 BYTEA NOT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
          UNIQUE (source_revision_ref, exact_span_ref, statement_sha256)
        );

        CREATE TABLE IF NOT EXISTS corpus.source_statement_origin (
          statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref),
          origin_ref TEXT NOT NULL,
          PRIMARY KEY (statement_ref, origin_ref)
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
          parser_receipt_ref TEXT NULL,
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

        CREATE TABLE IF NOT EXISTS pnf.observation_event_link (
          link_ref TEXT PRIMARY KEY,
          observation_ref TEXT NOT NULL,
          event_ref TEXT NOT NULL,
          assembly_receipt_ref TEXT NOT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
          UNIQUE (observation_ref, event_ref)
        );

        CREATE INDEX IF NOT EXISTS observation_event_observation_idx
          ON pnf.observation_event_link (observation_ref);
        CREATE INDEX IF NOT EXISTS observation_event_event_idx
          ON pnf.observation_event_link (event_ref);
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
           literal_text, statement_sha256,
           candidate_only, creates_semantic_authority,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,true,false,false,false)
        ON CONFLICT (statement_ref) DO NOTHING
        "#,
        &[
            &statement.statement_ref,
            &statement.document_ref,
            &statement.source_revision_ref,
            &statement.span.span_ref,
            &statement.literal_text,
            &&digest[..],
        ],
    )?;
    client.execute(
        r#"
        INSERT INTO corpus.source_statement_origin (statement_ref, origin_ref)
        VALUES ($1,$2)
        ON CONFLICT DO NOTHING
        "#,
        &[&statement.statement_ref, &origin_as_db(statement.origin)],
    )?;

    let persisted = load_source_statement_with_client(&mut client, &statement.statement_ref)?
        .ok_or(StatementTraceStoreError::ExistingStatementConflict)?;

    if persisted.document_ref != statement.document_ref
        || persisted.source_revision_ref != statement.source_revision_ref
        || persisted.exact_span_ref != statement.span.span_ref
        || persisted.literal_text != statement.literal_text
        || persisted.statement_sha256 != digest
        || !persisted.origins.contains(&statement.origin)
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
           parser_receipt_ref, parse_review_ref, admission_receipt_ref,
           disposition_ref, qualification_ref, candidate_only,
           creates_semantic_authority, applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,true,false,false,false)
        ON CONFLICT (link_ref) DO NOTHING
        "#,
        &[
            &link.link_ref,
            &link.statement_ref,
            &link.candidate_pnf_ref,
            &link.observation_ref,
            &link.parser_receipt_ref,
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

pub fn persist_observation_event_link(
    config: &DatabaseConfig,
    link: &ObservationEventLink,
) -> Result<ObservationEventLink, StatementTraceStoreError> {
    link.validate()?;
    let expected = canonical_observation_event_link_ref(&link.observation_ref, &link.event_ref);
    if link.link_ref != expected {
        return Err(StatementTraceStoreError::ExistingLinkConflict);
    }

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"
        INSERT INTO pnf.observation_event_link
          (link_ref, observation_ref, event_ref, assembly_receipt_ref,
           candidate_only, creates_semantic_authority,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,true,false,false,false)
        ON CONFLICT (link_ref) DO NOTHING
        "#,
        &[
            &link.link_ref,
            &link.observation_ref,
            &link.event_ref,
            &link.assembly_receipt_ref,
        ],
    )?;

    let persisted = load_observation_event_link_with_client(&mut client, &link.link_ref)?
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

pub fn load_source_statements_for_document(
    config: &DatabaseConfig,
    document_ref: &str,
) -> Result<Vec<PersistedSourceStatement>, StatementTraceStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let refs = client
        .query(
            "SELECT statement_ref FROM corpus.source_statement WHERE document_ref=$1 ORDER BY exact_span_ref, statement_ref",
            &[&document_ref],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    refs.into_iter()
        .map(|statement_ref| {
            load_source_statement_with_client(&mut client, &statement_ref)?
                .ok_or(StatementTraceStoreError::ExistingStatementConflict)
        })
        .collect()
}

pub fn load_statement_observation_links_for_statement(
    config: &DatabaseConfig,
    statement_ref: &str,
) -> Result<Vec<StatementObservationLink>, StatementTraceStoreError> {
    load_statement_observation_links(config, "statement_ref", statement_ref)
}

pub fn load_statement_observation_links_for_observation(
    config: &DatabaseConfig,
    observation_ref: &str,
) -> Result<Vec<StatementObservationLink>, StatementTraceStoreError> {
    load_statement_observation_links(config, "observation_ref", observation_ref)
}

pub fn load_observation_event_links_for_observation(
    config: &DatabaseConfig,
    observation_ref: &str,
) -> Result<Vec<ObservationEventLink>, StatementTraceStoreError> {
    load_observation_event_links(config, "observation_ref", observation_ref)
}

pub fn load_observation_event_links_for_event(
    config: &DatabaseConfig,
    event_ref: &str,
) -> Result<Vec<ObservationEventLink>, StatementTraceStoreError> {
    load_observation_event_links(config, "event_ref", event_ref)
}

fn load_statement_observation_links(
    config: &DatabaseConfig,
    coordinate: &str,
    value: &str,
) -> Result<Vec<StatementObservationLink>, StatementTraceStoreError> {
    let sql = match coordinate {
        "statement_ref" => statement_observation_select("statement_ref"),
        "observation_ref" => statement_observation_select("observation_ref"),
        _ => unreachable!("internal fixed statement trace query"),
    };
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client
        .query(&sql, &[&value])?
        .into_iter()
        .map(row_to_statement_observation_link)
        .collect()
}

fn statement_observation_select(coordinate: &str) -> String {
    format!(
        "SELECT link_ref, statement_ref, candidate_pnf_ref, observation_ref,          parser_receipt_ref, parse_review_ref, admission_receipt_ref,          disposition_ref, qualification_ref, candidate_only,          creates_semantic_authority, applicability_promoted, claim_truth_promoted          FROM pnf.statement_observation_link WHERE {coordinate}=$1 ORDER BY link_ref"
    )
}

fn load_observation_event_links(
    config: &DatabaseConfig,
    coordinate: &str,
    value: &str,
) -> Result<Vec<ObservationEventLink>, StatementTraceStoreError> {
    let sql = match coordinate {
        "observation_ref" => observation_event_select("observation_ref"),
        "event_ref" => observation_event_select("event_ref"),
        _ => unreachable!("internal fixed observation-event query"),
    };
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client
        .query(&sql, &[&value])?
        .into_iter()
        .map(row_to_observation_event_link)
        .collect()
}

fn observation_event_select(coordinate: &str) -> String {
    format!(
        "SELECT link_ref, observation_ref, event_ref, assembly_receipt_ref,          candidate_only, creates_semantic_authority, applicability_promoted,          claim_truth_promoted FROM pnf.observation_event_link          WHERE {coordinate}=$1 ORDER BY link_ref"
    )
}

fn load_source_statement_with_client(
    client: &mut Client,
    statement_ref: &str,
) -> Result<Option<PersistedSourceStatement>, StatementTraceStoreError> {
    let row = client.query_opt(
        r#"
        SELECT statement_ref, document_ref, source_revision_ref, exact_span_ref,
               literal_text, statement_sha256, candidate_only,
               creates_semantic_authority, applicability_promoted, claim_truth_promoted
        FROM corpus.source_statement
        WHERE statement_ref=$1
        "#,
        &[&statement_ref],
    )?;
    let Some(row) = row else {
        return Ok(None);
    };

    let digest: Vec<u8> = row.get(5);
    let digest: [u8; 32] = digest
        .try_into()
        .map_err(|_| StatementTraceStoreError::ExistingStatementConflict)?;
    let origins = client
        .query(
            "SELECT origin_ref FROM corpus.source_statement_origin              WHERE statement_ref=$1 ORDER BY origin_ref",
            &[&statement_ref],
        )?
        .into_iter()
        .map(|row| origin_from_db(row.get::<_, String>(0).as_str()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(PersistedSourceStatement {
        statement_ref: row.get(0),
        document_ref: row.get(1),
        source_revision_ref: row.get(2),
        exact_span_ref: row.get(3),
        literal_text: row.get(4),
        origins,
        statement_sha256: digest,
        candidate_only: row.get(6),
        creates_semantic_authority: row.get(7),
        applicability_promoted: row.get(8),
        claim_truth_promoted: row.get(9),
    }))
}

fn load_statement_observation_link_with_client(
    client: &mut Client,
    link_ref: &str,
) -> Result<Option<StatementObservationLink>, StatementTraceStoreError> {
    client
        .query_opt(
            r#"
            SELECT link_ref, statement_ref, candidate_pnf_ref, observation_ref,
                   parser_receipt_ref, parse_review_ref, admission_receipt_ref,
                   disposition_ref, qualification_ref, candidate_only,
                   creates_semantic_authority, applicability_promoted, claim_truth_promoted
            FROM pnf.statement_observation_link
            WHERE link_ref=$1
            "#,
            &[&link_ref],
        )?
        .map(row_to_statement_observation_link)
        .transpose()
}

fn row_to_statement_observation_link(
    row: postgres::Row,
) -> Result<StatementObservationLink, StatementTraceStoreError> {
    let link = StatementObservationLink {
        link_ref: row.get(0),
        statement_ref: row.get(1),
        candidate_pnf_ref: row.get(2),
        observation_ref: row.get(3),
        parser_receipt_ref: row.get(4),
        parse_review_ref: row.get(5),
        admission_receipt_ref: row.get(6),
        disposition: StatementObservationDisposition::from_db(row.get::<_, String>(7).as_str())?,
        qualification_ref: row.get(8),
        candidate_only: row.get(9),
        creates_semantic_authority: row.get(10),
        applicability_promoted: row.get(11),
        claim_truth_promoted: row.get(12),
    };
    link.validate()?;
    Ok(link)
}

fn load_observation_event_link_with_client(
    client: &mut Client,
    link_ref: &str,
) -> Result<Option<ObservationEventLink>, StatementTraceStoreError> {
    client
        .query_opt(
            r#"
            SELECT link_ref, observation_ref, event_ref, assembly_receipt_ref,
                   candidate_only, creates_semantic_authority,
                   applicability_promoted, claim_truth_promoted
            FROM pnf.observation_event_link
            WHERE link_ref=$1
            "#,
            &[&link_ref],
        )?
        .map(row_to_observation_event_link)
        .transpose()
}

fn row_to_observation_event_link(
    row: postgres::Row,
) -> Result<ObservationEventLink, StatementTraceStoreError> {
    let link = ObservationEventLink {
        link_ref: row.get(0),
        observation_ref: row.get(1),
        event_ref: row.get(2),
        assembly_receipt_ref: row.get(3),
        candidate_only: row.get(4),
        creates_semantic_authority: row.get(5),
        applicability_promoted: row.get(6),
        claim_truth_promoted: row.get(7),
    };
    link.validate()?;
    Ok(link)
}

fn require_source_ancestry(
    client: &mut Client,
    statement: &SourceStatementEnvelope,
) -> Result<(), StatementTraceStoreError> {
    // SCALE-1 generic long-document ancestry uses character coordinates over
    // the immutable UTF-8 canonical text. Do not reinterpret those coordinates
    // as byte offsets through corpus.span.
    let generic = client.query_opt(
        r#"
        SELECT r.start_char, r.end_char, c.payload
        FROM ingest.long_document_region r
        JOIN ingest.generic_source_revision g
          ON g.source_revision_ref = r.source_revision_ref
        JOIN corpus.document d ON d.document_ref = g.document_ref
        JOIN corpus.canonical_content c ON c.canonical_ref = d.canonical_ref
        WHERE r.source_revision_ref=$1
          AND r.region_ref=$2
          AND g.document_ref=$3
        "#,
        &[
            &statement.source_revision_ref,
            &statement.span.span_ref,
            &statement.document_ref,
        ],
    )?;

    if let Some(row) = generic {
        let start = row.get::<_, i64>(0);
        let end = row.get::<_, i64>(1);
        if start < 0
            || end < 0
            || start as u32 != statement.span.start_char
            || end as u32 != statement.span.end_char
        {
            return Err(StatementTraceStoreError::SpanDocumentMismatch);
        }
        let payload: Vec<u8> = row.get(2);
        let canonical_text = String::from_utf8(payload)
            .map_err(|_| StatementTraceStoreError::LiteralTextMismatch)?;
        let literal = canonical_text
            .chars()
            .skip(start as usize)
            .take((end - start) as usize)
            .collect::<String>();
        if literal != statement.literal_text {
            return Err(StatementTraceStoreError::LiteralTextMismatch);
        }
        return Ok(());
    }

    // Existing legal/source slices retain the historical corpus.span byte
    // coordinate contract. This fallback remains byte-for-byte compatible.
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

    let source_text: String = client
        .query_one(
            r#"
            SELECT convert_from(
              substring(c.payload FROM (s.start_char + 1) FOR (s.end_char - s.start_char)),
              'UTF8'
            )
            FROM corpus.span s
            JOIN corpus.document d ON d.document_ref=s.document_ref
            JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
            WHERE s.span_ref=$1 AND s.document_ref=$2
            "#,
            &[&statement.span.span_ref, &statement.document_ref],
        )?
        .get(0);
    if source_text != statement.literal_text {
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
        let reentry = statement(StatementOrigin::ResearchReentry);
        assert_eq!(intake.statement_ref, reentry.statement_ref);
    }

    #[test]
    fn observation_link_identity_is_structural() {
        assert_eq!(
            canonical_statement_observation_link_ref(
                "statement:x",
                "candidate:y",
                "observation:z"
            ),
            canonical_statement_observation_link_ref(
                "statement:x",
                "candidate:y",
                "observation:z"
            )
        );
    }

    #[test]
    fn observation_event_link_identity_is_stable_and_non_promoting() {
        let link = ObservationEventLink {
            link_ref: canonical_observation_event_link_ref("observation:1", "event:1"),
            observation_ref: "observation:1".into(),
            event_ref: "event:1".into(),
            assembly_receipt_ref: "event-assembly:1".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        link.validate().unwrap();
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
            parser_receipt_ref: Some("parser:run:1".into()),
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
