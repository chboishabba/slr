//! SCALE-2E exact transcript ↔ chat correspondence over *two independently
//! retained source revisions*. A chat copy is transport, never new
//! corroboration. No semantic proposition/admission authority is minted.
//!
//! The chat join is only emitted after checking both source payloads: unlike
//! a chat-only digest it checks the producer's native segment/utterance text.

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    ChatSourceJoin, ChatSourceJoinError,
    DatabaseConfig, SourceJoinType, load_chat_message_source,
    persist_chat_source_join,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptNativeEvent {
    WhisperxSegment { ordinal: usize },
    TircorderUtterance { utterance_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptChatExactQuote {
    pub message_ref: String,
    pub chat_start_char: u64,
    pub chat_end_char: u64,
    pub transcript_revision_ref: String,
    pub native_event: TranscriptNativeEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptChatCorrespondenceReceipt {
    pub join_ref: String,
    pub chat_message_ref: String,
    pub transcript_revision_ref: String,
    pub native_event_ref: String,
    pub evidence_digest_ref: String,
    pub independent_witness_increment: u8,
    pub semantic_authority_created: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum TranscriptChatCorrespondenceError {
    #[error(transparent)]
    Chat(#[from] crate::ChatSourceStoreError),
    #[error(transparent)]
    Join(#[from] ChatSourceJoinError),
    #[error(transparent)]
    Whisperx(#[from] crate::WhisperxSourceError),
    #[error(transparent)]
    Tircorder(#[from] crate::TircorderBridgeError),
    #[error("no source-owned event matches the supplied native coordinate")]
    UnknownNativeEvent,
    #[error("native transcript and selected chat literal differ")]
    NonExactCorrespondence,
    #[error("chat character offsets are invalid")]
    InvalidChatSpan,
}

/// Does not infer quotations by textual similarity. A caller supplies
/// explicit source/event linkage; this method checks exact producer text and
/// the original canonical chat substring before storing the correspondence.
pub fn persist_exact_transcript_chat_quote(
    config: &DatabaseConfig,
    request: &TranscriptChatExactQuote,
) -> Result<TranscriptChatCorrespondenceReceipt,TranscriptChatCorrespondenceError> {
    let message = load_chat_message_source(config, &request.message_ref)?
        .ok_or(crate::ChatSourceStoreError::UnknownMessage(request.message_ref.clone()))?;
    let text = crate::generic_source_compiler::canonical_char_subspans(
        &message.literal_text,
        &[(request.chat_start_char,request.chat_end_char)]
    ).ok_or(TranscriptChatCorrespondenceError::InvalidChatSpan)?
     .into_iter().next()
     .ok_or(TranscriptChatCorrespondenceError::InvalidChatSpan)?;

    let (native_text, source_kind, event_ref) = match &request.native_event {
        TranscriptNativeEvent::WhisperxSegment { ordinal } => {
            let p = crate::reopen_whisperx_source_plan(config,&request.transcript_revision_ref)?;
            let seg = p.native.segments.get(*ordinal)
                .ok_or(TranscriptChatCorrespondenceError::UnknownNativeEvent)?;
            (seg.text.clone(), "whisperx_transcript",
             format!("whisperx-segment:{ordinal}"))
        }
        TranscriptNativeEvent::TircorderUtterance { utterance_id } => {
            let p = crate::reopen_tircorder_session(config,&request.transcript_revision_ref)?;
            let utt = p.native.utterances.iter().find(|u|&u.utterance_id==utterance_id)
                .ok_or(TranscriptChatCorrespondenceError::UnknownNativeEvent)?;
            (utt.text.clone(),"tircorder_utterance",
             format!("tircorder-utterance:{utterance_id}"))
        }
    };
    if native_text != text {
        return Err(TranscriptChatCorrespondenceError::NonExactCorrespondence);
    }
    let digest = format!("sha256:{:x}", Sha256::digest(text.as_bytes()));
    let join = ChatSourceJoin {
        message_ref: request.message_ref.clone(),
        source_kind_ref: source_kind.into(),
        source_locator_ref: request.transcript_revision_ref.clone(),
        source_event_ref: Some(event_ref.clone()),
        start_char: request.chat_start_char,
        end_char: request.chat_end_char,
        join_type: SourceJoinType::ExactDigest,
        evidence_digest_ref: Some(digest.clone()),
    };
    let join_ref = persist_chat_source_join(config,&join)?;
    Ok(TranscriptChatCorrespondenceReceipt {
        join_ref,
        chat_message_ref:request.message_ref.clone(),
        transcript_revision_ref:request.transcript_revision_ref.clone(),
        native_event_ref:event_ref,
        evidence_digest_ref:digest,
        independent_witness_increment:0,
        semantic_authority_created:false,
        claim_truth_promoted:false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn correspondence_is_not_independent_corroboration() {
        // Explicit contract: even a verified exact transcript/chat join
        // contributes no independent witness or claim-truth authority.
        let receipt = TranscriptChatCorrespondenceReceipt {
            join_ref:"join:1".into(),chat_message_ref:"chat:1".into(),
            transcript_revision_ref:"transcript:1".into(),
            native_event_ref:"utterance:1".into(),
            evidence_digest_ref:"sha256:x".into(),
            independent_witness_increment:0,
            semantic_authority_created:false,claim_truth_promoted:false,
        };
        assert_eq!(receipt.independent_witness_increment,0);
        assert!(!receipt.claim_truth_promoted);
    }
}
