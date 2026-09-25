//! Provider adapter for the public Jmail email dataset.
//!
//! Jmail fields are preserved as provider coordinates and compiled into the
//! provider-neutral MailMessageSource contract. Provider record IDs, document
//! IDs and account addresses never become universal semantic/message identity.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use sensiblaw_core::source_ingest::{
    IngestRoleClass, MailBodySegment, MailBodySegmentKind, MailMessageSource,
    MailParticipant, SourceFamily, SourceIngestEnvelope,
};
use thiserror::Error;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct JmailEmailRecord {
    pub id: String,
    #[serde(default)]
    pub doc_id: Option<String>,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub to_recipients: Vec<String>,
    #[serde(default)]
    pub cc_recipients: Vec<String>,
    #[serde(default)]
    pub bcc_recipients: Vec<String>,
    #[serde(default)]
    pub sent_at: Option<String>,
    #[serde(default)]
    pub account_email: Option<String>,
    #[serde(default)]
    pub email_drop_id: Option<String>,
    #[serde(default)]
    pub epstein_is_sender: Option<bool>,
    #[serde(default)]
    pub content_markdown: Option<String>,
    #[serde(default)]
    pub content_html: Option<String>,
    #[serde(default)]
    pub attachments: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum JmailAdapterError {
    #[error("Jmail record id is empty")]
    EmptyId,
    #[error("Jmail dataset revision is empty")]
    EmptyDatasetRevision,
    #[error("Jmail email has no body text")]
    MissingBody,
    #[error("generic mail source validation failed: {0:?}")]
    InvalidMailSource(sensiblaw_core::source_ingest::SourceIngestError),
}

fn sha256_ref(text: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(text.as_bytes());
    format!("sha256:{:x}", hash.finalize())
}

fn participant(value: &str) -> MailParticipant {
    MailParticipant {
        display_name: None,
        address: value.trim().to_owned(),
    }
}

fn participants(values: &[String]) -> Vec<MailParticipant> {
    values
        .iter()
        .filter(|value| !value.trim().is_empty())
        .map(|value| participant(value))
        .collect()
}

fn line_ranges(text: &str) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let mut char_start = 0usize;
    for line in text.split_inclusive('\n') {
        let char_len = line.chars().count();
        out.push((char_start, char_start + char_len, line));
        char_start += char_len;
    }
    if text.is_empty() {
        return out;
    }
    if !text.ends_with('\n') {
        // split_inclusive already emitted the final non-newline-terminated line.
        return out;
    }
    out
}

fn classify_line(line: &str, quoted_mode: bool, forwarded_mode: bool) -> MailBodySegmentKind {
    let trimmed = line.trim();
    if forwarded_mode {
        return MailBodySegmentKind::ForwardedMessage;
    }
    if quoted_mode || trimmed.starts_with('>') {
        return MailBodySegmentKind::QuotedPriorMessage;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower == "--" || lower == "-- " || lower.starts_with("sent from my ") {
        return MailBodySegmentKind::Signature;
    }
    if lower.contains("confidentiality notice")
        || lower.contains("this email and any attachments")
    {
        return MailBodySegmentKind::Disclaimer;
    }
    if trimmed.is_empty() {
        return MailBodySegmentKind::Unknown;
    }
    MailBodySegmentKind::AuthoredHere
}

/// Conservative source-structure segmentation.
///
/// It deliberately does not infer authorship identity or semantic truth. It
/// recognizes only strong textual transport markers and preserves every byte
/// of body text inside exactly one segment.
pub fn segment_jmail_body(body_revision_ref: &str, body: &str) -> Vec<MailBodySegment> {
    if body.is_empty() {
        return vec![];
    }

    let lines = line_ranges(body);
    let mut segments = Vec::new();
    let mut quoted_mode = false;
    let mut forwarded_mode = false;
    let mut current_kind: Option<MailBodySegmentKind> = None;
    let mut current_start = 0usize;
    let mut current_end = 0usize;

    for (start, end, line) in lines {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();

        if lower.starts_with("----- forwarded message")
            || lower.starts_with("begin forwarded message")
        {
            forwarded_mode = true;
            quoted_mode = false;
        } else if lower.starts_with("on ") && lower.ends_with("wrote:") {
            quoted_mode = true;
        }

        let kind = classify_line(line, quoted_mode, forwarded_mode);

        match current_kind {
            Some(existing) if existing == kind => {
                current_end = end;
            }
            Some(existing) => {
                let segment_index = segments.len();
                segments.push(MailBodySegment {
                    segment_ref: format!(
                        "mail-segment:{body_revision_ref}:{segment_index}:{current_start}-{current_end}"
                    ),
                    body_revision_ref: body_revision_ref.to_owned(),
                    kind: existing,
                    start_char: current_start as u64,
                    end_char: current_end as u64,
                    derived_from_message_ref: None,
                    canonical_text_lineage_ref: None,
                });
                current_kind = Some(kind);
                current_start = start;
                current_end = end;
            }
            None => {
                current_kind = Some(kind);
                current_start = start;
                current_end = end;
            }
        }
    }

    if let Some(kind) = current_kind {
        let segment_index = segments.len();
        segments.push(MailBodySegment {
            segment_ref: format!(
                "mail-segment:{body_revision_ref}:{segment_index}:{current_start}-{current_end}"
            ),
            body_revision_ref: body_revision_ref.to_owned(),
            kind,
            start_char: current_start as u64,
            end_char: current_end as u64,
            derived_from_message_ref: None,
            canonical_text_lineage_ref: None,
        });
    }

    segments
}

pub fn jmail_record_to_mail_source(
    record: &JmailEmailRecord,
    dataset_revision_ref: &str,
) -> Result<(MailMessageSource, String), JmailAdapterError> {
    if record.id.trim().is_empty() {
        return Err(JmailAdapterError::EmptyId);
    }
    if dataset_revision_ref.trim().is_empty() {
        return Err(JmailAdapterError::EmptyDatasetRevision);
    }

    let body = record
        .content_markdown
        .as_deref()
        .filter(|value| !value.is_empty())
        .or_else(|| record.content_html.as_deref().filter(|value| !value.is_empty()))
        .ok_or(JmailAdapterError::MissingBody)?
        .to_owned();

    let message_ref = format!("mail:jmail:{}", record.id);
    let source_ref = format!("source:jmail:email:{}", record.id);
    let body_revision_ref = format!(
        "source-revision:jmail:email:{}:{}",
        record.id, dataset_revision_ref
    );

    let from = record
        .sender
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(participant)
        .into_iter()
        .collect::<Vec<_>>();

    let account_or_collection_ref = record
        .account_email
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("jmail-account:{value}"))
        .unwrap_or_else(|| "jmail-account:unknown".to_owned());

    let message = MailMessageSource {
        ingest: SourceIngestEnvelope {
            source_ref,
            source_revision_ref: body_revision_ref.clone(),
            provider_ref: "jmail-data-api".into(),
            family: SourceFamily::Mail,
            role_class: IngestRoleClass::ContentSource,
            content_digest_ref: sha256_ref(&body),
            acquisition_receipt_ref: format!("jmail-dataset-revision:{dataset_revision_ref}"),
            media_type_ref: if record.content_markdown.as_deref().is_some_and(|v| !v.is_empty()) {
                "text/markdown".into()
            } else {
                "text/html".into()
            },
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
        message_ref,
        account_or_collection_ref,
        provider_message_id: Some(record.id.clone()),
        internet_message_id: None,
        thread_ref: None,
        reply_to_ref: None,
        reference_message_refs: vec![],
        from,
        to: participants(&record.to_recipients),
        cc: participants(&record.cc_recipients),
        bcc: participants(&record.bcc_recipients),
        sent_time_ref: record.sent_at.clone(),
        received_time_ref: None,
        subject_revision_ref: record
            .subject
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|_| format!("subject-revision:jmail:{}:{}", record.id, dataset_revision_ref)),
        body_revision_ref: body_revision_ref.clone(),
        attachment_refs: vec![],
        raw_source_ref: record
            .doc_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("jmail-document:{value}"))
            .unwrap_or_else(|| format!("jmail-email-row:{}", record.id)),
        segments: segment_jmail_body(&body_revision_ref, &body),
    };

    message
        .validate()
        .map_err(JmailAdapterError::InvalidMailSource)?;

    Ok((message, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::source_ingest::MailBodySegmentKind;

    fn record(body: &str) -> JmailEmailRecord {
        JmailEmailRecord {
            id: "42".into(),
            doc_id: Some("doc-7".into()),
            sender: Some("alice@example.test".into()),
            subject: Some("Test".into()),
            to_recipients: vec!["bob@example.test".into()],
            cc_recipients: vec![],
            bcc_recipients: vec![],
            sent_at: Some("2004-06-18T10:00:00".into()),
            account_email: Some("archive@example.test".into()),
            email_drop_id: Some("drop:1".into()),
            epstein_is_sender: Some(false),
            content_markdown: Some(body.into()),
            content_html: None,
            attachments: serde_json::Value::Null,
        }
    }

    #[test]
    fn adapter_preserves_provider_id_without_using_it_as_message_identity() {
        let (message, body) =
            jmail_record_to_mail_source(&record("Hello Bob."), "dataset:2026-09-25").unwrap();
        assert_eq!(message.provider_message_id.as_deref(), Some("42"));
        assert_eq!(message.message_ref, "mail:jmail:42");
        assert_ne!(message.message_ref, "42");
        assert_eq!(message.raw_source_ref, "jmail-document:doc-7");
        assert_eq!(body, "Hello Bob.");
        assert!(!message.ingest.creates_semantic_authority);
        assert!(!message.ingest.claim_truth_promoted);
    }

    #[test]
    fn adapter_separates_new_authorship_from_quoted_transport() {
        let body = "New statement.\nOn Tue someone wrote:\n> Old statement.\n";
        let (message, _) =
            jmail_record_to_mail_source(&record(body), "dataset:fixture").unwrap();

        assert!(message
            .segments
            .iter()
            .any(|segment| segment.kind == MailBodySegmentKind::AuthoredHere));
        assert!(message
            .segments
            .iter()
            .any(|segment| segment.kind == MailBodySegmentKind::QuotedPriorMessage));
        assert_eq!(message.authored_semantic_spans().unwrap().len(), 1);
        assert!(message.quoted_transport_occurrence_count() >= 1);
    }

    #[test]
    fn adapter_marks_forwarded_block_as_transport() {
        let body = "My note.\n----- Forwarded message -----\nOld content\n";
        let (message, _) =
            jmail_record_to_mail_source(&record(body), "dataset:fixture").unwrap();

        assert!(message
            .segments
            .iter()
            .any(|segment| segment.kind == MailBodySegmentKind::ForwardedMessage));
        assert_eq!(message.authored_semantic_spans().unwrap().len(), 1);
    }
}
