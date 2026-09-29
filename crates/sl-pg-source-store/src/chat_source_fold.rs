//! StatiBaker chat/source-fold contract: source-owned materials stay owned
//! by their producers. These immutable links are evidence-qualified
//! *backreferences*, not extra canonical source records or corroboration.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{load_chat_message_source, DatabaseConfig};

pub const CHAT_SOURCE_JOIN_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS corpus;
CREATE TABLE IF NOT EXISTS corpus.chat_source_join (
    join_ref TEXT PRIMARY KEY,
    message_ref TEXT NOT NULL REFERENCES corpus.chat_archive_message(message_ref),
    source_kind_ref TEXT NOT NULL,
    source_locator_ref TEXT NOT NULL,
    source_event_ref TEXT NULL,
    start_char BIGINT NOT NULL CHECK (start_char >= 0),
    end_char BIGINT NOT NULL CHECK (end_char > start_char),
    join_type_ref TEXT NOT NULL,
    evidence_digest_ref TEXT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    UNIQUE (message_ref, start_char, end_char, source_kind_ref,
            source_locator_ref, join_type_ref)
);
CREATE INDEX IF NOT EXISTS chat_source_join_message_idx
  ON corpus.chat_source_join(message_ref, start_char, end_char);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceJoinType {
    ExactDigest,
    NearText,
    TimeWindowToolCall,
    UserDeclared,
    HeuristicShape,
}

impl SourceJoinType {
    fn as_db(self) -> &'static str {
        match self {
            Self::ExactDigest => "exact_digest",
            Self::NearText => "near_text",
            Self::TimeWindowToolCall => "time_window_tool_call",
            Self::UserDeclared => "user_declared",
            Self::HeuristicShape => "heuristic_shape",
        }
    }
    fn from_db(value: &str) -> Result<Self, ChatSourceJoinError> {
        match value {
            "exact_digest" => Ok(Self::ExactDigest),
            "near_text" => Ok(Self::NearText),
            "time_window_tool_call" => Ok(Self::TimeWindowToolCall),
            "user_declared" => Ok(Self::UserDeclared),
            "heuristic_shape" => Ok(Self::HeuristicShape),
            _ => Err(ChatSourceJoinError::InvalidJoinType),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSourceJoin {
    pub message_ref: String,
    pub source_kind_ref: String,
    pub source_locator_ref: String,
    pub source_event_ref: Option<String>,
    pub start_char: u64,
    pub end_char: u64,
    pub join_type: SourceJoinType,
    pub evidence_digest_ref: Option<String>,
}

#[derive(Debug, Error)]
pub enum ChatSourceJoinError {
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    ChatStore(#[from] crate::ChatSourceStoreError),
    #[error("join identity/coordinates are missing or invalid")]
    InvalidJoin,
    #[error("source join type is not recognized")]
    InvalidJoinType,
    #[error("exact digest does not match the literal chat substring")]
    DigestMismatch,
    #[error("immutable source join identity conflicts with stored record")]
    ImmutableJoinConflict,
    #[error("chat message is not present")]
    UnknownMessage,
}

fn sha256_ref(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn char_subspan(text: &str, start: u64, end: u64) -> Option<&str> {
    if start >= end || end > text.chars().count() as u64 {
        return None;
    }
    let byte_at = |pos: usize| {
        text.char_indices().nth(pos).map(|(offset, _)| offset)
            .unwrap_or(text.len())
    };
    Some(&text[byte_at(start as usize)..byte_at(end as usize)])
}

impl ChatSourceJoin {
    fn validate_for_message(&self, text: &str) -> Result<(), ChatSourceJoinError> {
        if self.message_ref.trim().is_empty()
            || self.source_kind_ref.trim().is_empty()
            || self.source_locator_ref.trim().is_empty()
            || self.source_event_ref.as_deref().is_some_and(|v| v.trim().is_empty())
            || self.evidence_digest_ref.as_deref().is_some_and(|v| v.trim().is_empty())
        {
            return Err(ChatSourceJoinError::InvalidJoin);
        }
        let literal = char_subspan(text, self.start_char, self.end_char)
            .ok_or(ChatSourceJoinError::InvalidJoin)?;
        if self.join_type == SourceJoinType::ExactDigest {
            // Verifies the **chat half** of the join, not the external source.
            if self.evidence_digest_ref.as_deref() != Some(sha256_ref(literal.as_bytes()).as_str()) {
                return Err(ChatSourceJoinError::DigestMismatch);
            }
        }
        Ok(())
    }
}

fn join_ref(join: &ChatSourceJoin) -> String {
    let serial = format!(
        "chat-source-join:v1\\n{}\\n{}\\n{}\\n{}\\n{}\\n{}\\n{}\\n{}\\n{}",
        join.message_ref,
        join.source_kind_ref,
        join.source_locator_ref,
        join.source_event_ref.as_deref().unwrap_or(""),
        join.start_char,
        join.end_char,
        join.join_type.as_db(),
        join.evidence_digest_ref.as_deref().unwrap_or(""),
        "candidate-only",
    );
    format!("chat-source-join:{}", sha256_ref(serial.as_bytes()))
}

pub fn persist_chat_source_join(
    config: &DatabaseConfig,
    join: &ChatSourceJoin,
) -> Result<String, ChatSourceJoinError> {
    let message = load_chat_message_source(config, &join.message_ref)?
        .ok_or(ChatSourceJoinError::UnknownMessage)?;
    join.validate_for_message(&message.literal_text)?;

    let reference = join_ref(join);
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CHAT_SOURCE_JOIN_SQL)?;
    client.execute(
        "INSERT INTO corpus.chat_source_join
            (join_ref, message_ref, source_kind_ref, source_locator_ref,
             source_event_ref, start_char, end_char, join_type_ref,
             evidence_digest_ref, candidate_only,
             creates_semantic_authority, claim_truth_promoted)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,TRUE,FALSE,FALSE)
         ON CONFLICT (join_ref) DO NOTHING",
        &[
            &reference, &join.message_ref, &join.source_kind_ref,
            &join.source_locator_ref, &join.source_event_ref,
            &(join.start_char as i64), &(join.end_char as i64),
            &join.join_type.as_db(), &join.evidence_digest_ref,
        ],
    )?;
    let row = client.query_one(
        "SELECT message_ref, source_kind_ref, source_locator_ref,
                source_event_ref, start_char, end_char, join_type_ref,
                evidence_digest_ref, candidate_only,
                creates_semantic_authority, claim_truth_promoted
         FROM corpus.chat_source_join WHERE join_ref=$1", &[&reference],
    )?;
    if row.get::<_, String>(0) != join.message_ref
        || row.get::<_, String>(1) != join.source_kind_ref
        || row.get::<_, String>(2) != join.source_locator_ref
        || row.get::<_, Option<String>>(3) != join.source_event_ref
        || row.get::<_, i64>(4) != join.start_char as i64
        || row.get::<_, i64>(5) != join.end_char as i64
        || row.get::<_, String>(6) != join.join_type.as_db()
        || row.get::<_, Option<String>>(7) != join.evidence_digest_ref
        || !row.get::<_, bool>(8)
        || row.get::<_, bool>(9)
        || row.get::<_, bool>(10)
    {
        return Err(ChatSourceJoinError::ImmutableJoinConflict);
    }
    Ok(reference)
}

pub fn load_chat_source_joins_for_message(
    config: &DatabaseConfig,
    message_ref: &str,
) -> Result<Vec<ChatSourceJoin>, ChatSourceJoinError> {
    let source = load_chat_message_source(config, message_ref)?
        .ok_or(ChatSourceJoinError::UnknownMessage)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CHAT_SOURCE_JOIN_SQL)?;
    let rows = client.query(
        "SELECT source_kind_ref, source_locator_ref, source_event_ref,
                start_char, end_char, join_type_ref, evidence_digest_ref
         FROM corpus.chat_source_join WHERE message_ref=$1
         ORDER BY start_char, end_char, join_ref",
        &[&message_ref],
    )?;
    rows.into_iter().map(|row| {
        let join = ChatSourceJoin {
            message_ref: message_ref.to_owned(),
            source_kind_ref: row.get(0),
            source_locator_ref: row.get(1),
            source_event_ref: row.get(2),
            start_char: row.get::<_, i64>(3) as u64,
            end_char: row.get::<_, i64>(4) as u64,
            join_type: SourceJoinType::from_db(&row.get::<_, String>(5))?,
            evidence_digest_ref: row.get(6),
        };
        join.validate_for_message(&source.literal_text)?;
        Ok(join)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_chat_half_digest_must_match_original_unicode_span() {
        let text = "🌍 copied log";
        let join = ChatSourceJoin {
            message_ref: "chat:1".into(),
            source_kind_ref: "codex_tui_log".into(),
            source_locator_ref: "log:1".into(),
            source_event_ref: None,
            start_char: 2,
            end_char: 8,
            join_type: SourceJoinType::ExactDigest,
            evidence_digest_ref: Some(sha256_ref("copied".as_bytes())),
        };
        join.validate_for_message(text).unwrap();
        let mut altered = join;
        altered.evidence_digest_ref = Some(sha256_ref("different".as_bytes()));
        assert!(matches!(
            altered.validate_for_message(text),
            Err(ChatSourceJoinError::DigestMismatch),
        ));
    }
}
