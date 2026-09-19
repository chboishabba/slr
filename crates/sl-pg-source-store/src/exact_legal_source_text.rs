use postgres::{Client, NoTls};
use thiserror::Error;

use crate::DatabaseConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactLegalSourceText {
    pub source_revision_ref: String,
    pub document_ref: String,
    pub canonical_text_sha256: String,
    pub canonical_text: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ExactLegalSourceTextError {
    #[error("source revision reference must not be empty")]
    EmptySourceRevision,
    #[error("exact compile-eligible legal source revision not found: {0}")]
    SourceNotFound(String),
    #[error("canonical legal source text is not UTF-8")]
    InvalidUtf8,
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for ExactLegalSourceTextError {
    fn from(value: postgres::Error) -> Self {
        Self::Postgres(value.to_string())
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Load the exact canonical text already retained for one compile-eligible
/// legal source revision.
///
/// This is a read projection only. Returning source text does not create legal
/// authority, applicability, claim truth, proposition support, or review.
pub fn load_exact_legal_source_text(
    config: &DatabaseConfig,
    source_revision_ref: &str,
) -> Result<ExactLegalSourceText, ExactLegalSourceTextError> {
    if source_revision_ref.trim().is_empty() {
        return Err(ExactLegalSourceTextError::EmptySourceRevision);
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        "SELECT l.source_revision_ref, l.document_ref, c.content_sha256, c.payload          FROM legal_source_revision AS l          JOIN corpus.document AS d ON d.document_ref = l.document_ref          JOIN corpus.canonical_content AS c ON c.canonical_ref = d.canonical_ref          WHERE l.source_revision_ref = $1            AND l.compile_eligible = TRUE",
        &[&source_revision_ref],
    )?;
    let Some(row) = row else {
        return Err(ExactLegalSourceTextError::SourceNotFound(
            source_revision_ref.to_owned(),
        ));
    };
    let digest: Vec<u8> = row.get(2);
    let payload: Vec<u8> = row.get(3);
    let canonical_text =
        String::from_utf8(payload).map_err(|_| ExactLegalSourceTextError::InvalidUtf8)?;
    Ok(ExactLegalSourceText {
        source_revision_ref: row.get(0),
        document_ref: row.get(1),
        canonical_text_sha256: format!("sha256:{}", hex(&digest)),
        canonical_text,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
