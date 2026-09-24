//! S28.CHAT conversational source-family boundary.
//!
//! An archived chat message is a source object, not a proposition and not a
//! world fact. Statements are exact subspans selected from the message and then
//! enter the ordinary M12 PNF membrane.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatBranchMembership {
    Active,
    Inactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMessageRole {
    User,
    Assistant,
    Tool,
    System,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatContentKind {
    Message,
    ToolOutput,
    FileContext,
    ArtifactOutput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedChatMessage {
    pub conversation_ref: String,
    pub message_ref: String,
    pub node_ref: String,
    pub parent_node_ref: Option<String>,
    pub branch_membership: ChatBranchMembership,
    pub message_time_ref: String,
    pub role: ChatMessageRole,
    pub thread_title: String,
    pub content: String,
    pub content_kind: ChatContentKind,
    pub citation_token: Option<String>,
    pub filename: Option<String>,
    pub asset_pointer: Option<String>,
    pub source_scope: Option<String>,
    pub body_storage_ref: Option<String>,
    pub mime_type: Option<String>,
    pub archive_source_id: Option<String>,
    pub provenance_ref: Option<String>,
    pub identity_verified: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatStatementCandidateSpan {
    pub statement_candidate_ref: String,
    pub message_ref: String,
    pub start_char: u32,
    pub end_char: u32,
    pub literal_text: String,
    pub splitter_receipt_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatSourceError {
    EmptyCoordinate(&'static str),
    IdentityNotVerified,
    InvalidSpan,
    PromotionNotAllowed,
}

fn require(name: &'static str, value: &str) -> Result<(), ChatSourceError> {
    if value.trim().is_empty() {
        Err(ChatSourceError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn validate_optional(
    name: &'static str,
    value: &Option<String>,
) -> Result<(), ChatSourceError> {
    if value.as_deref().is_some_and(|value| value.trim().is_empty()) {
        Err(ChatSourceError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

impl ArchivedChatMessage {
    pub fn validate(&self) -> Result<(), ChatSourceError> {
        require("conversation_ref", &self.conversation_ref)?;
        require("message_ref", &self.message_ref)?;
        require("node_ref", &self.node_ref)?;
        require("message_time_ref", &self.message_time_ref)?;
        validate_optional("parent_node_ref", &self.parent_node_ref)?;
        validate_optional("citation_token", &self.citation_token)?;
        validate_optional("filename", &self.filename)?;
        validate_optional("asset_pointer", &self.asset_pointer)?;
        validate_optional("source_scope", &self.source_scope)?;
        validate_optional("body_storage_ref", &self.body_storage_ref)?;
        validate_optional("mime_type", &self.mime_type)?;
        validate_optional("archive_source_id", &self.archive_source_id)?;
        validate_optional("provenance_ref", &self.provenance_ref)?;

        if !self.identity_verified {
            return Err(ChatSourceError::IdentityNotVerified);
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(ChatSourceError::PromotionNotAllowed);
        }
        Ok(())
    }

    #[must_use]
    pub fn eligible_for_world_evidence_without_review(&self) -> bool {
        false
    }

    #[must_use]
    pub fn is_inactive_generated_branch(&self) -> bool {
        self.branch_membership == ChatBranchMembership::Inactive
            && self.role == ChatMessageRole::Assistant
    }
}

impl ChatStatementCandidateSpan {
    pub fn validate(&self) -> Result<(), ChatSourceError> {
        require("statement_candidate_ref", &self.statement_candidate_ref)?;
        require("message_ref", &self.message_ref)?;
        require("splitter_receipt_ref", &self.splitter_receipt_ref)?;
        if self.start_char >= self.end_char || self.literal_text.is_empty() {
            return Err(ChatSourceError::InvalidSpan);
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(ChatSourceError::PromotionNotAllowed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message() -> ArchivedChatMessage {
        ArchivedChatMessage {
            conversation_ref: "conversation:1".into(),
            message_ref: "message:1".into(),
            node_ref: "node:1".into(),
            parent_node_ref: Some("node:0".into()),
            branch_membership: ChatBranchMembership::Active,
            message_time_ref: "2026-09-24T01:02:03Z".into(),
            role: ChatMessageRole::User,
            thread_title: "Matter discussion".into(),
            content: "One claim. Another hypothesis.".into(),
            content_kind: ChatContentKind::Message,
            citation_token: None,
            filename: None,
            asset_pointer: None,
            source_scope: None,
            body_storage_ref: Some("message:1".into()),
            mime_type: None,
            archive_source_id: Some("pull:1".into()),
            provenance_ref: Some("payload-identity:verified".into()),
            identity_verified: true,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn archived_message_is_not_world_evidence_by_itself() {
        let value = message();
        value.validate().unwrap();
        assert!(!value.eligible_for_world_evidence_without_review());
    }

    #[test]
    fn inactive_assistant_generation_remains_representation_history() {
        let mut value = message();
        value.branch_membership = ChatBranchMembership::Inactive;
        value.role = ChatMessageRole::Assistant;
        assert!(value.is_inactive_generated_branch());
        assert!(!value.eligible_for_world_evidence_without_review());
    }

    #[test]
    fn unverified_payload_identity_fails_closed() {
        let mut value = message();
        value.identity_verified = false;
        assert_eq!(value.validate(), Err(ChatSourceError::IdentityNotVerified));
    }

    #[test]
    fn message_identity_is_distinct_from_statement_span_identity() {
        let span = ChatStatementCandidateSpan {
            statement_candidate_ref: "chat-statement-candidate:1".into(),
            message_ref: "message:1".into(),
            start_char: 0,
            end_char: 9,
            literal_text: "One claim".into(),
            splitter_receipt_ref: "splitter:1".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        span.validate().unwrap();
        assert_ne!(span.statement_candidate_ref, span.message_ref);
    }
}
