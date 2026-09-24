use std::{fs, path::Path};

use postgres::{Client, NoTls};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use sensiblaw_core::chat_source::{
    ArchivedChatMessage, ChatBranchMembership, ChatContentKind, ChatMessageRole,
    ChatSourceError, ChatStatementCandidateSpan,
};

use crate::{
    canonical_statement_ref, persist_source_statement, DatabaseConfig,
    ExactSourceSpan, PersistedSourceStatement, SourceStatementEnvelope,
    StatementOrigin, StatementTraceStoreError,
};

pub const CHAT_ARCHIVE_EXPORT_SCHEMA: &str =
    "sensiblaw.chat-archive-message.v0_1";

#[derive(Debug, Clone, Deserialize)]
pub struct ChatArchiveExportRow {
    pub schema: String,
    pub conversation_ref: String,
    pub message_ref: String,
    pub node_ref: String,
    #[serde(default)]
    pub parent_node_ref: String,
    pub branch_membership: String,
    pub message_time_ref: String,
    pub role_ref: String,
    #[serde(default)]
    pub thread_title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tool_kind: String,
    #[serde(default)]
    pub citation_token: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub asset_pointer: String,
    #[serde(default)]
    pub source_scope: String,
    #[serde(default)]
    pub body_storage_ref: String,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub archive_source_id: String,
    #[serde(default)]
    pub provenance_json: String,
    pub identity_verified: bool,
    pub candidate_only: bool,
    pub semantic_promotion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedChatMessageSource {
    pub message_ref: String,
    pub conversation_ref: String,
    pub node_ref: String,
    pub parent_node_ref: Option<String>,
    pub message_time_ref: String,
    pub thread_title: String,
    pub literal_text: String,
    pub document_ref: String,
    pub source_revision_ref: String,
    pub full_message_span_ref: String,
    pub branch_membership: ChatBranchMembership,
    pub role: ChatMessageRole,
    pub content_kind: ChatContentKind,
}

#[derive(Debug, Error)]
pub enum ChatSourceStoreError {
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("invalid chat archive JSON at line {0}")]
    InvalidJsonLine(usize),
    #[error("chat archive export schema mismatch")]
    SchemaMismatch,
    #[error("chat archive source object is invalid")]
    InvalidSource,
    #[error("base corpus tables are not installed")]
    BaseCorpusMissing,
    #[error("existing archived message conflicts with requested coordinates")]
    ExistingMessageConflict,
    #[error("unknown archived message: {0}")]
    UnknownMessage(String),
    #[error("statement selection does not belong to archived message")]
    StatementMessageMismatch,
    #[error("statement subspan does not match archived literal source")]
    LiteralSubspanMismatch,
    #[error("statement subspan is invalid")]
    InvalidStatementSpan,
    #[error(transparent)]
    StatementTrace(#[from] StatementTraceStoreError),
}

pub fn load_chat_archive_export_jsonl(
    path: impl AsRef<Path>,
) -> Result<Vec<ArchivedChatMessage>, ChatSourceStoreError> {
    let raw = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let row: ChatArchiveExportRow = serde_json::from_str(line)
            .map_err(|_| ChatSourceStoreError::InvalidJsonLine(index + 1))?;
        if row.schema != CHAT_ARCHIVE_EXPORT_SCHEMA {
            return Err(ChatSourceStoreError::SchemaMismatch);
        }
        if row.semantic_promotion || !row.candidate_only {
            return Err(ChatSourceStoreError::InvalidSource);
        }
        let message = row_to_domain(row)?;
        message
            .validate()
            .map_err(|_| ChatSourceStoreError::InvalidSource)?;
        out.push(message);
    }
    Ok(out)
}

pub fn install_chat_source_schema(
    config: &DatabaseConfig,
) -> Result<(), ChatSourceStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    for table in [
        "corpus.canonical_content",
        "corpus.document",
        "corpus.span",
    ] {
        let present: Option<String> = client
            .query_one("SELECT to_regclass($1)::text", &[&table])?
            .get(0);
        if present.is_none() {
            return Err(ChatSourceStoreError::BaseCorpusMissing);
        }
    }

    client.batch_execute(
        r#"
        CREATE TABLE IF NOT EXISTS corpus.chat_archive_message (
          message_ref TEXT PRIMARY KEY,
          conversation_ref TEXT NOT NULL,
          node_ref TEXT NOT NULL,
          parent_node_ref TEXT NULL,
          branch_membership_ref TEXT NOT NULL,
          message_time_ref TEXT NOT NULL,
          role_ref TEXT NOT NULL,
          thread_title TEXT NOT NULL,
          content_kind_ref TEXT NOT NULL,
          citation_token TEXT NULL,
          filename TEXT NULL,
          asset_pointer TEXT NULL,
          source_scope TEXT NULL,
          body_storage_ref TEXT NULL,
          mime_type TEXT NULL,
          archive_source_id TEXT NULL,
          provenance_ref TEXT NULL,
          document_ref TEXT NOT NULL REFERENCES corpus.document(document_ref),
          source_revision_ref TEXT NOT NULL,
          full_message_span_ref TEXT NOT NULL REFERENCES corpus.span(span_ref),
          identity_verified BOOLEAN NOT NULL CHECK (identity_verified),
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
          UNIQUE (conversation_ref, node_ref)
        );

        CREATE INDEX IF NOT EXISTS chat_archive_message_conversation_idx
          ON corpus.chat_archive_message (conversation_ref, message_time_ref);
        "#,
    )?;
    Ok(())
}

pub fn persist_chat_archive_message(
    config: &DatabaseConfig,
    message: &ArchivedChatMessage,
) -> Result<PersistedChatMessageSource, ChatSourceStoreError> {
    message
        .validate()
        .map_err(|_| ChatSourceStoreError::InvalidSource)?;
    install_chat_source_schema(config)?;

    let content_digest = sha256(message.content.as_bytes());
    let content_hex = hex(&content_digest);
    let canonical_ref = format!("canonical:sha256:{content_hex}");
    let document_ref = format!("document:sha256:{content_hex}");

    let revision_digest = sha256(
        format!(
            "chat-message-revision:v1\n{}\n{}\n{}\n{}",
            message.conversation_ref,
            message.message_ref,
            message.node_ref,
            content_hex,
        )
        .as_bytes(),
    );
    let source_revision_ref =
        format!("chat-message-revision:sha256:{}", hex(&revision_digest));
    let span_digest = sha256(
        format!(
            "chat-message-full-span:v1\n{}\n{}\n{}",
            source_revision_ref,
            document_ref,
            message.message_ref,
        )
        .as_bytes(),
    );
    let full_message_span_ref =
        format!("span:chat-message:sha256:{}", hex(&span_digest));

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;

    tx.execute(
        r#"
        INSERT INTO corpus.canonical_content
          (canonical_ref, content_sha256, encoding_ref, normalization_ref,
           compression_ref, payload, uncompressed_byte_length)
        VALUES ($1,$2,'utf-8','identity',NULL,$3,$4)
        ON CONFLICT (content_sha256, encoding_ref, normalization_ref) DO NOTHING
        "#,
        &[
            &canonical_ref,
            &&content_digest[..],
            &message.content.as_bytes(),
            &(message.content.len() as i64),
        ],
    )?;
    let canonical_ref: String = tx
        .query_one(
            r#"
            SELECT canonical_ref FROM corpus.canonical_content
            WHERE content_sha256=$1
              AND encoding_ref='utf-8'
              AND normalization_ref='identity'
            "#,
            &[&&content_digest[..]],
        )?
        .get(0);

    tx.execute(
        r#"
        INSERT INTO corpus.document
          (document_ref, source_content_ref, canonical_ref, media_type,
           adapter_ref, adapter_version, compiler_context_ref, document_sha256)
        VALUES ($1,NULL,$2,'text/plain','chat-archive-message','1',
                'sensiblaw-chat-source-v1',$3)
        ON CONFLICT (document_sha256) DO NOTHING
        "#,
        &[&document_ref, &canonical_ref, &&content_digest[..]],
    )?;
    let document_ref: String = tx
        .query_one(
            "SELECT document_ref FROM corpus.document WHERE document_sha256=$1",
            &[&&content_digest[..]],
        )?
        .get(0);

    tx.execute(
        r#"
        INSERT INTO corpus.span
          (span_ref, document_ref, start_char, end_char,
           start_token, end_token, span_type_ref)
        VALUES ($1,$2,0,$3,NULL,NULL,'chat_message')
        ON CONFLICT (span_ref) DO NOTHING
        "#,
        &[
            &full_message_span_ref,
            &document_ref,
            &(message.content.len() as i32),
        ],
    )?;

    tx.execute(
        r#"
        INSERT INTO corpus.chat_archive_message
          (message_ref, conversation_ref, node_ref, parent_node_ref,
           branch_membership_ref, message_time_ref, role_ref, thread_title,
           content_kind_ref, citation_token, filename, asset_pointer,
           source_scope, body_storage_ref, mime_type, archive_source_id,
           provenance_ref, document_ref, source_revision_ref,
           full_message_span_ref, identity_verified, candidate_only,
           creates_semantic_authority, applicability_promoted,
           claim_truth_promoted)
        VALUES
          ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,
           $18,$19,$20,true,true,false,false,false)
        ON CONFLICT (message_ref) DO NOTHING
        "#,
        &[
            &message.message_ref,
            &message.conversation_ref,
            &message.node_ref,
            &message.parent_node_ref,
            &branch_db(message.branch_membership),
            &message.message_time_ref,
            &role_db(message.role),
            &message.thread_title,
            &kind_db(message.content_kind),
            &message.citation_token,
            &message.filename,
            &message.asset_pointer,
            &message.source_scope,
            &message.body_storage_ref,
            &message.mime_type,
            &message.archive_source_id,
            &message.provenance_ref,
            &document_ref,
            &source_revision_ref,
            &full_message_span_ref,
        ],
    )?;
    tx.commit()?;

    let persisted = load_chat_message_source(config, &message.message_ref)?
        .ok_or(ChatSourceStoreError::ExistingMessageConflict)?;
    if persisted.conversation_ref != message.conversation_ref
        || persisted.document_ref != document_ref
        || persisted.source_revision_ref != source_revision_ref
        || persisted.full_message_span_ref != full_message_span_ref
        || persisted.branch_membership != message.branch_membership
        || persisted.role != message.role
        || persisted.content_kind != message.content_kind
    {
        return Err(ChatSourceStoreError::ExistingMessageConflict);
    }
    Ok(persisted)
}

pub fn materialize_chat_statement(
    config: &DatabaseConfig,
    selection: &ChatStatementCandidateSpan,
    origin: StatementOrigin,
) -> Result<PersistedSourceStatement, ChatSourceStoreError> {
    selection
        .validate()
        .map_err(|_| ChatSourceStoreError::InvalidStatementSpan)?;
    let source = load_chat_message_source(config, &selection.message_ref)?
        .ok_or_else(|| ChatSourceStoreError::UnknownMessage(selection.message_ref.clone()))?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let content: String = client
        .query_one(
            r#"
            SELECT convert_from(c.payload, 'UTF8')
            FROM corpus.document d
            JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
            WHERE d.document_ref=$1
            "#,
            &[&source.document_ref],
        )?
        .get(0);

    let start = selection.start_char as usize;
    let end = selection.end_char as usize;
    if start >= end
        || end > content.len()
        || !content.is_char_boundary(start)
        || !content.is_char_boundary(end)
    {
        return Err(ChatSourceStoreError::InvalidStatementSpan);
    }
    if &content[start..end] != selection.literal_text {
        return Err(ChatSourceStoreError::LiteralSubspanMismatch);
    }

    let span_digest = sha256(
        format!(
            "chat-statement-span:v1\n{}\n{}\n{}\n{}",
            source.source_revision_ref,
            selection.statement_candidate_ref,
            selection.start_char,
            selection.end_char,
        )
        .as_bytes(),
    );
    let span_ref = format!("span:chat-statement:sha256:{}", hex(&span_digest));
    client.execute(
        r#"
        INSERT INTO corpus.span
          (span_ref, document_ref, start_char, end_char,
           start_token, end_token, span_type_ref)
        VALUES ($1,$2,$3,$4,NULL,NULL,'chat_statement_candidate')
        ON CONFLICT (span_ref) DO NOTHING
        "#,
        &[
            &span_ref,
            &source.document_ref,
            &(selection.start_char as i32),
            &(selection.end_char as i32),
        ],
    )?;

    let mut statement = SourceStatementEnvelope {
        statement_ref: String::new(),
        document_ref: source.document_ref,
        source_revision_ref: source.source_revision_ref,
        span: ExactSourceSpan {
            span_ref,
            start_char: selection.start_char,
            end_char: selection.end_char,
        },
        literal_text: selection.literal_text.clone(),
        origin,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    statement.statement_ref = canonical_statement_ref(&statement);
    persist_source_statement(config, &statement).map_err(Into::into)
}

pub fn load_chat_message_source(
    config: &DatabaseConfig,
    message_ref: &str,
) -> Result<Option<PersistedChatMessageSource>, ChatSourceStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client
        .query_opt(
            r#"
            SELECT m.message_ref, m.conversation_ref, m.node_ref,
                   m.parent_node_ref, m.message_time_ref, m.thread_title,
                   convert_from(c.payload, 'UTF8') AS literal_text,
                   m.document_ref, m.source_revision_ref,
                   m.full_message_span_ref, m.branch_membership_ref,
                   m.role_ref, m.content_kind_ref
            FROM corpus.chat_archive_message m
            JOIN corpus.document d ON d.document_ref=m.document_ref
            JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
            WHERE m.message_ref=$1
            "#,
            &[&message_ref],
        )?
        .map(|row| {
            Ok(PersistedChatMessageSource {
                message_ref: row.get(0),
                conversation_ref: row.get(1),
                node_ref: row.get(2),
                parent_node_ref: row.get(3),
                message_time_ref: row.get(4),
                thread_title: row.get(5),
                literal_text: row.get(6),
                document_ref: row.get(7),
                source_revision_ref: row.get(8),
                full_message_span_ref: row.get(9),
                branch_membership: branch_from_db(
                    row.get::<_, String>(10).as_str(),
                )?,
                role: role_from_db(row.get::<_, String>(11).as_str())?,
                content_kind: kind_from_db(row.get::<_, String>(12).as_str())?,
            })
        })
        .transpose()
}

pub fn load_chat_messages_for_conversation(
    config: &DatabaseConfig,
    conversation_ref: &str,
) -> Result<Vec<PersistedChatMessageSource>, ChatSourceStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let refs = client
        .query(
            r#"
            SELECT message_ref FROM corpus.chat_archive_message
            WHERE conversation_ref=$1
            ORDER BY message_time_ref, message_ref
            "#,
            &[&conversation_ref],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    refs.into_iter()
        .map(|message_ref| {
            load_chat_message_source(config, &message_ref)?
                .ok_or(ChatSourceStoreError::ExistingMessageConflict)
        })
        .collect()
}

fn row_to_domain(row: ChatArchiveExportRow) -> Result<ArchivedChatMessage, ChatSourceStoreError> {
    let branch_membership = match row.branch_membership.as_str() {
        "active" => ChatBranchMembership::Active,
        "inactive" => ChatBranchMembership::Inactive,
        _ => return Err(ChatSourceStoreError::InvalidSource),
    };
    let role = match row.role_ref.as_str() {
        "user" => ChatMessageRole::User,
        "assistant" => ChatMessageRole::Assistant,
        "tool" => ChatMessageRole::Tool,
        "system" => ChatMessageRole::System,
        _ => ChatMessageRole::Other,
    };
    let content_kind = match row.tool_kind.as_str() {
        "tool" => ChatContentKind::ToolOutput,
        "file_context" => ChatContentKind::FileContext,
        "artifact_output" => ChatContentKind::ArtifactOutput,
        _ => ChatContentKind::Message,
    };

    Ok(ArchivedChatMessage {
        conversation_ref: row.conversation_ref,
        message_ref: row.message_ref,
        node_ref: row.node_ref,
        parent_node_ref: nonempty(row.parent_node_ref),
        branch_membership,
        message_time_ref: row.message_time_ref,
        role,
        thread_title: row.thread_title,
        content: row.content,
        content_kind,
        citation_token: nonempty(row.citation_token),
        filename: nonempty(row.filename),
        asset_pointer: nonempty(row.asset_pointer),
        source_scope: nonempty(row.source_scope),
        body_storage_ref: nonempty(row.body_storage_ref),
        mime_type: nonempty(row.mime_type),
        archive_source_id: nonempty(row.archive_source_id),
        provenance_ref: nonempty(row.provenance_json),
        identity_verified: row.identity_verified,
        candidate_only: row.candidate_only,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

fn nonempty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn branch_db(value: ChatBranchMembership) -> &'static str {
    match value {
        ChatBranchMembership::Active => "active",
        ChatBranchMembership::Inactive => "inactive",
    }
}

fn branch_from_db(value: &str) -> Result<ChatBranchMembership, ChatSourceStoreError> {
    match value {
        "active" => Ok(ChatBranchMembership::Active),
        "inactive" => Ok(ChatBranchMembership::Inactive),
        _ => Err(ChatSourceStoreError::InvalidSource),
    }
}

fn role_db(value: ChatMessageRole) -> &'static str {
    match value {
        ChatMessageRole::User => "user",
        ChatMessageRole::Assistant => "assistant",
        ChatMessageRole::Tool => "tool",
        ChatMessageRole::System => "system",
        ChatMessageRole::Other => "other",
    }
}

fn role_from_db(value: &str) -> Result<ChatMessageRole, ChatSourceStoreError> {
    match value {
        "user" => Ok(ChatMessageRole::User),
        "assistant" => Ok(ChatMessageRole::Assistant),
        "tool" => Ok(ChatMessageRole::Tool),
        "system" => Ok(ChatMessageRole::System),
        "other" => Ok(ChatMessageRole::Other),
        _ => Err(ChatSourceStoreError::InvalidSource),
    }
}

fn kind_db(value: ChatContentKind) -> &'static str {
    match value {
        ChatContentKind::Message => "message",
        ChatContentKind::ToolOutput => "tool_output",
        ChatContentKind::FileContext => "file_context",
        ChatContentKind::ArtifactOutput => "artifact_output",
    }
}

fn kind_from_db(value: &str) -> Result<ChatContentKind, ChatSourceStoreError> {
    match value {
        "message" => Ok(ChatContentKind::Message),
        "tool_output" => Ok(ChatContentKind::ToolOutput),
        "file_context" => Ok(ChatContentKind::FileContext),
        "artifact_output" => Ok(ChatContentKind::ArtifactOutput),
        _ => Err(ChatSourceStoreError::InvalidSource),
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
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

    #[test]
    fn export_row_preserves_inactive_branch_without_promoting_it() {
        let row = ChatArchiveExportRow {
            schema: CHAT_ARCHIVE_EXPORT_SCHEMA.into(),
            conversation_ref: "conversation:1".into(),
            message_ref: "message:1".into(),
            node_ref: "node:1".into(),
            parent_node_ref: "node:0".into(),
            branch_membership: "inactive".into(),
            message_time_ref: "2026-09-24T00:00:00Z".into(),
            role_ref: "assistant".into(),
            thread_title: "thread".into(),
            content: "alternate answer".into(),
            tool_kind: String::new(),
            citation_token: String::new(),
            filename: String::new(),
            asset_pointer: String::new(),
            source_scope: String::new(),
            body_storage_ref: "message:1".into(),
            mime_type: String::new(),
            archive_source_id: "pull:1".into(),
            provenance_json: "{\"identity_verified\":true}".into(),
            identity_verified: true,
            candidate_only: true,
            semantic_promotion: false,
        };
        let value = row_to_domain(row).unwrap();
        assert!(value.is_inactive_generated_branch());
        assert!(!value.eligible_for_world_evidence_without_review());
    }

    #[test]
    fn tool_and_file_context_are_distinct_source_kinds() {
        let mut tool = ChatArchiveExportRow {
            schema: CHAT_ARCHIVE_EXPORT_SCHEMA.into(),
            conversation_ref: "c".into(),
            message_ref: "m".into(),
            node_ref: "n".into(),
            parent_node_ref: String::new(),
            branch_membership: "active".into(),
            message_time_ref: "t".into(),
            role_ref: "tool".into(),
            thread_title: String::new(),
            content: "output".into(),
            tool_kind: "tool".into(),
            citation_token: String::new(),
            filename: String::new(),
            asset_pointer: String::new(),
            source_scope: "tool_output".into(),
            body_storage_ref: "m".into(),
            mime_type: String::new(),
            archive_source_id: "s".into(),
            provenance_json: String::new(),
            identity_verified: true,
            candidate_only: true,
            semantic_promotion: false,
        };
        assert_eq!(
            row_to_domain(tool.clone()).unwrap().content_kind,
            ChatContentKind::ToolOutput
        );
        tool.tool_kind = "file_context".into();
        tool.source_scope = "project_file_context".into();
        assert_eq!(
            row_to_domain(tool).unwrap().content_kind,
            ChatContentKind::FileContext
        );
    }

    #[test]
    fn subspan_literal_mismatch_is_a_distinct_failure() {
        let selection = ChatStatementCandidateSpan {
            statement_candidate_ref: "candidate:1".into(),
            message_ref: "message:1".into(),
            start_char: 0,
            end_char: 4,
            literal_text: "nope".into(),
            splitter_receipt_ref: "splitter:1".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        selection.validate().unwrap();
    }
}
