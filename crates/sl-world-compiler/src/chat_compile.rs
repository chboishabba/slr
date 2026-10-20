//! Strict bounded archived-chat JSONL -> native SLRO -> SLRW.
//! No spaCy, no inferred dependencies, no semantic promotion.
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use sensiblaw_core::chat_source::{
    ArchivedChatMessage, ChatBranchMembership, ChatContentKind, ChatMessageRole,
};
use sensiblaw_world_compiler::{
    compile_observation_stream, encode_observation, DependencyShape, ObservationRecord,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_MESSAGES: usize = 10000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchivedLine {
    schema: String,
    conversation_ref: String,
    message_ref: String,
    node_ref: String,
    parent_node_ref: String,
    branch_membership: String,
    message_time_ref: String,
    role_ref: String,
    thread_title: String,
    content: String,
    tool_kind: String,
    citation_token: String,
    filename: String,
    asset_pointer: String,
    source_scope: String,
    body_storage_ref: String,
    mime_type: String,
    archive_source_id: String,
    provenance_json: String,
    identity_verified: bool,
    candidate_only: bool,
    semantic_promotion: bool,
}

fn optional(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn parse_line(line: &str) -> io::Result<(ArchivedChatMessage, bool)> {
    let raw: ArchivedLine = serde_json::from_str(line)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if raw.schema != "sensiblaw.chat-archive-message.v0_1"
        || !raw.candidate_only || raw.semantic_promotion {
        return Err(io::Error::new(io::ErrorKind::InvalidData,
                                  "invalid schema or attempted semantic promotion"));
    }
    let branch = match raw.branch_membership.as_str() {
        "active" => ChatBranchMembership::Active,
        "inactive" => ChatBranchMembership::Inactive,
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid branch")),
    };
    let role = match raw.role_ref.as_str() {
        "user" => ChatMessageRole::User,
        "assistant" => ChatMessageRole::Assistant,
        "tool" => ChatMessageRole::Tool,
        "system" => ChatMessageRole::System,
        _ => ChatMessageRole::Other,
    };
    let kind = if !raw.tool_kind.is_empty() {
        ChatContentKind::ToolOutput
    } else if !raw.asset_pointer.is_empty() {
        ChatContentKind::ArtifactOutput
    } else {
        ChatContentKind::Message
    };
    let archived = ArchivedChatMessage {
        conversation_ref: raw.conversation_ref,
        message_ref: raw.message_ref,
        node_ref: raw.node_ref,
        parent_node_ref: optional(raw.parent_node_ref),
        branch_membership: branch,
        message_time_ref: raw.message_time_ref,
        role,
        thread_title: raw.thread_title,
        content: raw.content,
        content_kind: kind,
        citation_token: optional(raw.citation_token),
        filename: optional(raw.filename),
        asset_pointer: optional(raw.asset_pointer),
        source_scope: optional(raw.source_scope),
        body_storage_ref: optional(raw.body_storage_ref),
        mime_type: optional(raw.mime_type),
        archive_source_id: optional(raw.archive_source_id),
        provenance_ref: optional(raw.provenance_json),
        identity_verified: raw.identity_verified,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    archived.validate().map_err(|e| io::Error::new(
        io::ErrorKind::InvalidData, format!("invalid archived chat source: {e:?}"),
    ))?;
    Ok((archived, raw.identity_verified))
}

fn emit_message<W: Write>(writer: &mut W, msg: &ArchivedChatMessage) -> io::Result<u64> {
    // Preserve producer coordinates. Never substitute a title or canonical hash for identity.
    let document_ref = format!("chat:{}:message:{}:node:{}",
                               msg.conversation_ref, msg.message_ref, msg.node_ref);
    let digest: [u8; 32] = Sha256::digest(msg.content.as_bytes()).into();
    let branch = match msg.branch_membership {
        ChatBranchMembership::Active => "active",
        ChatBranchMembership::Inactive => "inactive",
    };
    let lineage = format!("{}:{}:{}:{}", msg.node_ref,
        msg.parent_node_ref.as_deref().unwrap_or(""), branch, msg.message_time_ref);
    let lineage_digest: [u8; 32] = Sha256::digest(lineage.as_bytes()).into();
    let revision_ref = format!("chat-revision:sha256:{}:lineage:{}",
                               hex_digest(&digest), hex_digest(&lineage_digest));
    encode_observation(writer, &ObservationRecord::Manifestation {
        document_ref: document_ref.clone(),
        qid: "Q_CHAT".into(),
        language: "und".into(),
        revision_ref,
        source_sha256: digest,
    }).map_err(io::Error::other)?;

    // Native conservative span lexer: Unicode scalar offsets, no dependency inference.
    // Every lexical token is UnresolvedDependency, never a fabricated actor/predicate.
    let mut ordinal = 0u32;
    let mut start = None;
    let mut count = 0u64;
    for (byte_index, ch) in msg.content.char_indices()
        .chain(std::iter::once((msg.content.len(), ' '))) {
        if ch.is_whitespace() {
            if let Some(begin) = start.take() {
                let orth = &msg.content[begin..byte_index];
                let start_char = u32::try_from(msg.content[..begin].chars().count())
                    .map_err(|_| io::Error::other("span start overflow"))?;
                let end_char = u32::try_from(msg.content[..byte_index].chars().count())
                    .map_err(|_| io::Error::other("span end overflow"))?;
                encode_observation(writer, &ObservationRecord::Token {
                    document_ref: document_ref.clone(),
                    sentence_id: 0,
                    local_ordinal: ordinal,
                    start_char,
                    end_char,
                    head_ordinal: ordinal,
                    shape: DependencyShape::UnresolvedDependency,
                    orth: orth.into(), lemma: orth.into(),
                    head_orth: orth.into(), head_lemma: orth.into(),
                }).map_err(io::Error::other)?;
                ordinal = ordinal.checked_add(1).ok_or_else(|| io::Error::other("token overflow"))?;
                count += 1;
            }
        } else if start.is_none() {
            start = Some(byte_index);
        }
    }
    Ok(count)
}

fn hex_digest(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn compile_chat(
    input: &Path, output: &Path, iteration: i64, max_messages: usize,
) -> io::Result<()> {
    if max_messages == 0 || max_messages > MAX_MESSAGES {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "max_messages must be 1..10000"));
    }
    let reader = BufReader::new(File::open(input)?);
    // Anonymous tempfile: no persistent SLRO or duplicate transcript corpus.
    let mut observation_stream = tempfile::tempfile()?;
    let mut messages = 0usize;
    let mut seen = HashSet::new();
    let mut unresolved = 0u64;
    for line in reader.lines() {
        let line = line?;
        if line.len() > MAX_MESSAGE_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "message exceeds 4MiB"));
        }
        if line.trim().is_empty() { continue; }
        if messages >= max_messages {
            return Err(io::Error::new(io::ErrorKind::InvalidData,
                                      "tranche limit exceeded: input must be bounded"));
        }
        let (message, _) = parse_line(&line)?;
        let identity = (message.conversation_ref.clone(), message.message_ref.clone(),
                        message.node_ref.clone());
        if !seen.insert(identity) {
            return Err(io::Error::new(io::ErrorKind::InvalidData,
                                      "duplicate exact archived message identity"));
        }
        unresolved += emit_message(&mut observation_stream, &message)?;
        messages += 1;
    }
    if messages == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "empty tranche"));
    }
    observation_stream.seek(SeekFrom::Start(0))?;
    let mut world = tempfile::tempfile()?;
    let receipt = compile_observation_stream(&mut observation_stream, &mut world, iteration)
        .map_err(io::Error::other)?;
    if receipt.manifestations != messages as u64
        || receipt.unresolved_dependencies != unresolved
        || receipt.pnf_candidates != 0 || receipt.world_atoms != 0
        || receipt.semantic_promotion {
        return Err(io::Error::other("unexpected semantic publication in source-only compile"));
    }
    world.seek(SeekFrom::Start(0))?;
    let mut out = BufWriter::new(File::create(output)?);
    io::copy(&mut world, &mut out)?;
    out.flush()?;
    eprintln!(
        "SLR_CHAT_NATIVE_RECEIPT schema=sensiblaw.chat-archive-message.v0_1 messages={} unresolved={} manifestations={} pnf_candidates={} world_atoms={} candidate_only=true semantic_promotion=false",
        messages, unresolved, receipt.manifestations, receipt.pnf_candidates, receipt.world_atoms
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(content: &str) -> String {
        serde_json::json!({
            "schema":"sensiblaw.chat-archive-message.v0_1",
            "conversation_ref":"c1","message_ref":"m1","node_ref":"n1",
            "parent_node_ref":"","branch_membership":"active",
            "message_time_ref":"2026-10-20T00:00:00Z","role_ref":"user",
            "thread_title":"case","content":content,"tool_kind":"",
            "citation_token":"","filename":"","asset_pointer":"",
            "source_scope":"","body_storage_ref":"","mime_type":"",
            "archive_source_id":"","provenance_json":"",
            "identity_verified":true,"candidate_only":true,"semantic_promotion":false
        }).to_string()
    }

    #[test]
    fn native_chat_source_compiles_without_inventing_pnf() {
        let mut input = tempfile::NamedTempFile::new().unwrap();
        writeln!(input, "{}", fixture("One claim. Another hypothesis.")).unwrap();
        let output = tempfile::NamedTempFile::new().unwrap();
        compile_chat(input.path(), output.path(), 1, 5).unwrap();
        let mut reader = File::open(output.path()).unwrap();
        let mut kinds = Vec::new();
        while let Some(record) = sensiblaw_world_store::decode_record(&mut reader).unwrap() {
            kinds.push(record.kind);
        }
        assert_eq!(kinds.len(), 2); // source manifestation + iteration only
    }

    #[test]
    fn unverified_identity_and_overlimit_fail_closed() {
        let invalid = fixture("test").replace("\"identity_verified\":true", "\"identity_verified\":false");
        assert!(parse_line(&invalid).is_err());
        let mut input = tempfile::NamedTempFile::new().unwrap();
        writeln!(input, "{}", fixture("first")).unwrap();
        writeln!(input, "{}", fixture("second")).unwrap();
        let output = tempfile::NamedTempFile::new().unwrap();
        assert!(compile_chat(input.path(), output.path(), 1, 1).is_err());
    }
}
