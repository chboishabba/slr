//! Source-preserving projection of the *implemented* SensibLaw WhisperX
//! execution envelope (asr_adapter.py), not the separate draft TiRCorder
//! session/event packet specification. Interpretation remains in M12.
//!
//! Native segment timestamps, confidence, diarizer labels, model, language,
//! optional audio hash, and unrecognized provider keys are retained in the
//! source plan. None of them establishes person identity or audio completeness.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sensiblaw_core::source_ingest::{
    IngestRoleClass, SourceFamily, SourceIngestEnvelope,
};
use thiserror::Error;

use crate::{
    compile_source_regions_lossless, CandidatePnfProducer,
    GenericSourceCompilerError, LosslessBulkSourceCompilation,
    SourceExecutionRegion, SourceRegionExecutionClass,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhisperxSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(flatten)]
    pub additional: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhisperxExecutionEnvelope {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    pub segments: Vec<WhisperxSegment>,
    #[serde(flatten)]
    pub additional: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct WhisperxSourcePlan {
    pub envelope: SourceIngestEnvelope,
    /// SHA-256 of the exact supplied provider payload, not only its text.
    pub raw_payload_digest_ref: String,
    /// Retain the *original bytes* separately from compiler canonical text.
    pub raw_payload: String,
    pub native: WhisperxExecutionEnvelope,
    pub canonical_text: String,
    pub regions: Vec<SourceExecutionRegion>,
    pub missing_speaker_count: usize,
    pub audio_hash_ref: Option<String>,
    pub transcript_completeness_claimed: bool,
    pub diarization_establishes_person_identity: bool,
}

#[derive(Debug, Error)]
pub enum WhisperxSourceError {
    #[error("source identity is required")]
    MissingSourceIdentity,
    #[error("malformed WhisperX execution envelope: {0}")]
    InvalidPayload(#[from] serde_json::Error),
    #[error("execution envelope has no segments")]
    EmptySegments,
    #[error("invalid transcript segment at ordinal {0}")]
    InvalidSegment(usize),
    #[error("invalid audio digest; provide sha256 hex or leave absent")]
    InvalidAudioDigest,
    #[error("source text exceeds u32 M12 character-coordinate ABI")]
    SpanOverflow,
    #[error(transparent)]
    Compiler(#[from] GenericSourceCompilerError),
}

fn hash_ref(raw: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(raw.as_bytes()))
}

/// Adapter input is the raw WhisperX JSON already produced by the existing
/// SensibLaw path. It does *not* run ASR, re-diarize, or re-segment speech.
/// The optional audio digest must come from the actual audio owner, not text.
pub fn plan_whisperx_source(
    raw_json: &str,
    source_ref: &str,
    acquisition_receipt_ref: &str,
    audio_hash_ref: Option<&str>,
) -> Result<WhisperxSourcePlan, WhisperxSourceError> {
    if source_ref.trim().is_empty() || acquisition_receipt_ref.trim().is_empty() {
        return Err(WhisperxSourceError::MissingSourceIdentity);
    }
    if let Some(hash) = audio_hash_ref {
        let hex = hash.strip_prefix("sha256:").unwrap_or(hash);
        if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(WhisperxSourceError::InvalidAudioDigest);
        }
    }
    let native: WhisperxExecutionEnvelope = serde_json::from_str(raw_json)?;
    if native.segments.is_empty() {
        return Err(WhisperxSourceError::EmptySegments);
    }
    let raw_digest = hash_ref(raw_json);
    let source_revision_ref = format!(
        "source-revision:whisperx:{}:{}",
        source_ref,
        raw_digest,
    );
    let mut canonical_text = String::new();
    let mut regions = Vec::new();
    let mut position = 0u64;
    let mut missing_speaker_count = 0usize;

    for (index, segment) in native.segments.iter().enumerate() {
        if !segment.start.is_finite()
            || !segment.end.is_finite()
            || segment.start < 0.0
            || segment.end <= segment.start
            || segment.text.is_empty()
            || segment.confidence.is_some_and(|value| !value.is_finite())
        {
            return Err(WhisperxSourceError::InvalidSegment(index));
        }
        if segment.speaker.is_none() {
            missing_speaker_count += 1;
        }
        if index > 0 {
            canonical_text.push('\n');
            position += 1;
        }
        let start = position;
        canonical_text.push_str(&segment.text);
        position += segment.text.chars().count() as u64;
        if position > u32::MAX as u64 {
            return Err(WhisperxSourceError::SpanOverflow);
        }
        regions.push(SourceExecutionRegion {
            region_ref: format!("transcript-segment:{source_revision_ref}:{index}"),
            source_revision_ref: source_revision_ref.clone(),
            start_char: start,
            end_char: position,
            class: SourceRegionExecutionClass::SemanticCandidate,
        });
    }
    let envelope = SourceIngestEnvelope {
        source_ref: source_ref.to_owned(),
        source_revision_ref,
        provider_ref: "sensiblaw:whisperx_importer_v1".into(),
        family: SourceFamily::Transcript,
        role_class: IngestRoleClass::ContentSource,
        content_digest_ref: hash_ref(&canonical_text),
        acquisition_receipt_ref: acquisition_receipt_ref.to_owned(),
        media_type_ref: "text/plain".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    Ok(WhisperxSourcePlan {
        envelope,
        raw_payload_digest_ref: raw_digest,
        raw_payload: raw_json.to_owned(),
        native,
        canonical_text,
        regions,
        missing_speaker_count,
        audio_hash_ref: audio_hash_ref.map(str::to_owned),
        transcript_completeness_claimed: false,
        diarization_establishes_person_identity: false,
    })
}

pub fn compile_whisperx_source_lossless<P: CandidatePnfProducer>(
    producer: &P,
    plan: &WhisperxSourcePlan,
    statement_document_ref: &str,
    parser_receipt_prefix: &str,
) -> Result<LosslessBulkSourceCompilation, WhisperxSourceError> {
    Ok(compile_source_regions_lossless(
        producer,
        &plan.envelope.source_ref,
        &plan.envelope.source_revision_ref,
        statement_document_ref,
        &plan.canonical_text,
        &plan.regions,
        parser_receipt_prefix,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_speaker_absence_unicode_and_native_timing() {
        let raw = r#"{"model":"whisperx-small","language":"en","segments":[{"start":0.0,"end":1.2,"text":"Écho 🛰","confidence":0.91,"speaker":"SPEAKER_00","words":[{"start":0.1,"end":0.3}]},{"start":1.2,"end":2.4,"text":"Uncertain.","confidence":0.4}]}"#;
        let plan = plan_whisperx_source(raw, "source:audio:1", "receipt:asr:1", None).unwrap();
        assert_eq!(plan.canonical_text, "Écho 🛰\nUncertain.");
        assert_eq!(plan.regions[0].end_char, 6);
        assert_eq!(plan.regions[1].start_char, 7);
        assert_eq!(plan.native.segments[0].additional["words"][0]["start"], 0.1);
        assert_eq!(plan.missing_speaker_count, 1);
        assert!(plan.audio_hash_ref.is_none());
        assert!(!plan.transcript_completeness_claimed);
        assert!(!plan.diarization_establishes_person_identity);
        assert!(plan.envelope.source_revision_ref.contains(&plan.raw_payload_digest_ref));
    }

    #[test]
    fn rejects_invalid_timing_without_fabricating_repairs() {
        let bad = r#"{"segments":[{"start":2.0,"end":1.0,"text":"x"}]}"#;
        assert!(matches!(
            plan_whisperx_source(bad, "source:1", "receipt:1", None),
            Err(WhisperxSourceError::InvalidSegment(0))
        ));
    }
}
