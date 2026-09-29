//! Source-preserving projection of the *implemented* SensibLaw WhisperX
//! execution envelope (asr_adapter.py), not the separate draft TiRCorder
//! session/event packet specification. Interpretation remains in M12.
//!
//! Native segment timestamps, confidence, diarizer labels, model, language,
//! optional audio hash, and unrecognized provider keys are retained in the
//! source plan. None of them establishes person identity or audio completeness.

use serde::{Deserialize, Serialize};
use postgres::{Client, NoTls};
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
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    SourceStore(#[from] crate::GenericSourceContentStoreError),
    #[error("reopened WhisperX source has conflicting raw envelope identity")]
    RawEnvelopeIdentityConflict,
    #[error(transparent)]
    Parser(#[from] crate::DbNativeParserError),
    #[error("parser run source revision differs from transcript")]
    ParserRevisionMismatch,
    #[error("parser worker has not attempted all transcript segments")]
    IncompleteParserRun,
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

/// Sidecar stores the producer-owned execution packet verbatim. The compiler
/// text representation is stored through the *existing* generic source store;
/// this table is only an immutable provenance backreference. Do not treat
/// diarization labels or this source record as semantic observations.
pub const WHISPERX_PROVENANCE_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS ingest;
CREATE TABLE IF NOT EXISTS ingest.whisperx_execution_source (
    source_revision_ref TEXT PRIMARY KEY
        REFERENCES ingest.generic_source_revision(source_revision_ref),
    raw_payload_digest_ref TEXT NOT NULL,
    raw_payload TEXT NOT NULL,
    audio_hash_ref TEXT NULL,
    missing_speaker_count BIGINT NOT NULL CHECK (missing_speaker_count >= 0),
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);
"#;

/// Idempotent PG persistence of both canonical *transcript text* and the exact
/// native WhisperX packet. Audio itself is never synthesized or ingested here.
pub fn persist_whisperx_source_plan(
    config: &crate::DatabaseConfig,
    plan: &WhisperxSourcePlan,
) -> Result<crate::PersistedGenericSourceContent, WhisperxSourceError> {
    // This generic substrate owns all immutable source/content identities.
    let persisted = crate::persist_generic_text_source(
        config, &plan.envelope, &plan.canonical_text,
    )?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(WHISPERX_PROVENANCE_SQL)?;
    client.execute(
        "INSERT INTO ingest.whisperx_execution_source
         (source_revision_ref, raw_payload_digest_ref, raw_payload,
          audio_hash_ref, missing_speaker_count, candidate_only,
          creates_semantic_authority, claim_truth_promoted)
         VALUES ($1,$2,$3,$4,$5,TRUE,FALSE,FALSE)
         ON CONFLICT (source_revision_ref) DO NOTHING",
        &[
            &plan.envelope.source_revision_ref,
            &plan.raw_payload_digest_ref,
            &plan.raw_payload,
            &plan.audio_hash_ref,
            &(plan.missing_speaker_count as i64),
        ],
    )?;
    let row = client.query_one(
        "SELECT raw_payload_digest_ref, raw_payload, audio_hash_ref,
                missing_speaker_count, candidate_only,
                creates_semantic_authority, claim_truth_promoted
         FROM ingest.whisperx_execution_source WHERE source_revision_ref=$1",
        &[&plan.envelope.source_revision_ref],
    )?;
    if row.get::<_, String>(0) != plan.raw_payload_digest_ref
        || row.get::<_, String>(1) != plan.raw_payload
        || row.get::<_, Option<String>>(2) != plan.audio_hash_ref
        || row.get::<_, i64>(3) != plan.missing_speaker_count as i64
        || !row.get::<_, bool>(4)
        || row.get::<_, bool>(5)
        || row.get::<_, bool>(6)
    {
        return Err(WhisperxSourceError::RawEnvelopeIdentityConflict);
    }
    Ok(persisted)
}

/// Reopen both source coordinates and the exact producer-owned packet. Never
/// infer missing audio identity from transcript text or a speaker alias.
pub fn reopen_whisperx_source_plan(
    config: &crate::DatabaseConfig,
    source_revision_ref: &str,
) -> Result<WhisperxSourcePlan, WhisperxSourceError> {
    let (persisted, canonical) = crate::load_generic_text_source(
        config, source_revision_ref,
    )?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_one(
        "SELECT raw_payload, raw_payload_digest_ref, audio_hash_ref,
                missing_speaker_count
         FROM ingest.whisperx_execution_source WHERE source_revision_ref=$1",
        &[&source_revision_ref],
    )?;
    let raw: String = row.get(0);
    let audio_hash_ref: Option<String> = row.get(2);
    let recovered = plan_whisperx_source(
        &raw, &persisted.source_ref, &persisted.acquisition_receipt_ref,
        audio_hash_ref.as_deref(),
    )?;
    if recovered.raw_payload_digest_ref != row.get::<_, String>(1)
        || recovered.missing_speaker_count as i64 != row.get::<_, i64>(3)
        || recovered.envelope.source_revision_ref != source_revision_ref
        || recovered.canonical_text != canonical
        || recovered.envelope.content_digest_ref != persisted.content_digest_ref
    {
        return Err(WhisperxSourceError::RawEnvelopeIdentityConflict);
    }
    Ok(recovered)
}

/// Use the *existing* DB-native parser queue for the native WhisperX
/// segment boundaries. These are source segment regions, not guessed sentence
/// segmentation; model/speaker/time never enter parser input as free text.
pub fn prepare_whisperx_parser_run(
    config: &crate::DatabaseConfig,
    plan: &WhisperxSourcePlan,
    parser_family: &str,
    parser_version: &str,
    model_ref: &str,
    parser_config_json: &str,
) -> Result<(crate::ParserRunReceipt, crate::ParserEnqueueReceipt), WhisperxSourceError> {
    let _persisted = persist_whisperx_source_plan(config, plan)?;
    let run = crate::start_parser_run(
        config,
        &plan.envelope.source_revision_ref,
        parser_family,
        parser_version,
        model_ref,
        parser_config_json,
    )?;
    let regions = plan.regions.iter().filter(|region| {
        region.class == SourceRegionExecutionClass::SemanticCandidate
    }).map(|region| crate::ParserRegionJobSpec {
        region_ref: region.region_ref.clone(),
        start_char: region.start_char,
        end_char: region.end_char,
    }).collect::<Vec<_>>();
    let enqueued = crate::enqueue_parser_regions_with_content_reuse(
        config, &run, &regions, &plan.canonical_text,
    )?;
    Ok((run, enqueued))
}

/// Compile the reopened immutable source via the same DB-native M12 parser
/// snapshot as books and chat. This returns a lossless compilation receipt;
/// durable candidate batches and review projection remain a separate step.
pub fn compile_whisperx_from_parser_run(
    config: &crate::DatabaseConfig,
    parser_run_ref: &str,
) -> Result<LosslessBulkSourceCompilation, WhisperxSourceError> {
    let snapshot = crate::DbNativeParserSnapshot::load(config, parser_run_ref)?;
    let source_revision_ref = snapshot.source_revision_ref();
    let plan = reopen_whisperx_source_plan(config, source_revision_ref)?;
    let state = crate::parser_run_state(config, parser_run_ref)?;
    if state.queued != 0 || state.leased != 0
        || state.unattempted_semantic_regions != 0
        || state.succeeded + state.residual != plan.regions.len()
    {
        return Err(WhisperxSourceError::IncompleteParserRun);
    }
    let (persisted, _) = crate::load_generic_text_source(config, source_revision_ref)?;
    if persisted.source_revision_ref != plan.envelope.source_revision_ref {
        return Err(WhisperxSourceError::ParserRevisionMismatch);
    }
    let compilation = compile_whisperx_source_lossless(
        &snapshot,
        &plan,
        &persisted.document_ref,
        &format!("db-parser:{parser_run_ref}"),
    )?;
    if !compilation.source_coverage_complete() {
        return Err(WhisperxSourceError::IncompleteParserRun);
    }
    Ok(compilation)
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
