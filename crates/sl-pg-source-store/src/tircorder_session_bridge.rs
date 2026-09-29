//! Versioned boundary for the *documented* TiRCorder normalized session packet.
//! This is not the deployed SensibLaw WhisperX ASR envelope: they are separate
//! producers with separate source identities. No NLP, person identification,
//! audio completeness claim or semantic admission happens at this boundary.
//!
//! Donor: SensibLaw/docs/tircorder_connector.md, initial
//! NormalizedTiRCEventPacket sessions[*].utterances[*] shape.
//! This bridge intentionally does NOT consume the legal-graph packet appended
//! later in that document.

use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;
use sensiblaw_core::source_ingest::{IngestRoleClass, SourceFamily, SourceIngestEnvelope};
use crate::{
    SourceExecutionRegion, SourceRegionExecutionClass, GenericSourceCompilerError,
    CandidatePnfProducer, LosslessBulkSourceCompilation,
    compile_source_regions_lossless,
};

pub const TIRCORDER_SESSION_BRIDGE_VERSION: &str = "tircorder-normalized-session.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TircorderPacket {
    pub connector: String,
    pub batch_id: String,
    pub ingested_at: String,
    #[serde(default)]
    pub source: Option<Value>,
    pub sessions: Vec<TircorderSession>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TircorderSession {
    pub session_id: String,
    pub device_id: String,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub audio_sha256: Option<String>,
    pub utterances: Vec<TircorderUtterance>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TircorderUtterance {
    pub utterance_id: String,
    pub start: String,
    pub end: String,
    pub text: String,
    #[serde(default)]
    pub speaker_label: Option<String>,
    #[serde(default)]
    pub speaker_confidence: Option<f64>,
    #[serde(default)]
    pub words: Vec<TircorderWord>,
    #[serde(default)]
    pub sentence_splits: Vec<TircorderSentenceSplit>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TircorderWord {
    pub w: String,
    pub start: f64,
    pub end: f64,
    #[serde(default)]
    pub conf: Option<f64>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TircorderSentenceSplit {
    pub char_start: u64,
    pub char_end: u64,
}

#[derive(Debug, Clone)]
pub struct TircorderSourcePlan {
    pub bridge_version: &'static str,
    pub source_collection_ref: String,
    pub envelope: SourceIngestEnvelope,
    /// Original packet is retained, including data irrelevant to M12.
    pub raw_packet: String,
    pub raw_packet_sha256: String,
    pub session_index: usize,
    pub native: TircorderSession,
    pub canonical_text: String,
    pub regions: Vec<SourceExecutionRegion>,
    pub missing_speaker_count: usize,
    pub audio_hash_ref: Option<String>,
    pub speaker_identity_established: bool,
    pub audio_completeness_established: bool,
}

#[derive(Debug, Error)]
pub enum TircorderBridgeError {
    #[error("packet is not a normalized TiRCorder session packet")]
    InvalidPacket,
    #[error("source or acquisition identity missing")]
    MissingIdentity,
    #[error("invalid native session or utterance coordinate")]
    InvalidCoordinate,
    #[error("duplicate session or utterance identity")]
    DuplicateIdentity,
    #[error("utterance words have invalid or nonmonotonic times")]
    InvalidWordTiming,
    #[error("sentence splits overlap or escape native utterance text")]
    InvalidSentenceSplit,
    #[error("source exceeds u32 M12 character coordinates")]
    SpanOverflow,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Compiler(#[from] GenericSourceCompilerError),
    #[error(transparent)]
    SourceStore(#[from] crate::GenericSourceContentStoreError),
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    Parser(#[from] crate::DbNativeParserError),
    #[error(transparent)]
    CandidatePersistence(#[from] crate::GenericCandidatePersistenceError),
    #[error("raw session packet does not reopen to the same source revision")]
    ReopenMismatch,
    #[error("parser run source identity, completeness or counts mismatched")]
    ParserMismatch,
    #[error("same TiRCorder collection and batch identity contains conflicting packet bytes")]
    BatchIdentityConflict,
}

fn digest(raw: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(raw.as_bytes()))
}
fn normalized_audio_hash(hash: &str) -> Result<String, TircorderBridgeError> {
    let hex = hash.strip_prefix("sha256:").unwrap_or(hash);
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(TircorderBridgeError::InvalidCoordinate);
    }
    Ok(format!("sha256:{}", hex.to_ascii_lowercase()))
}
fn present(v: &str) -> bool { !v.trim().is_empty() }

/// Compile one session into text regions. The source coordinates in `native`
/// remain immutable and independently reopenable, not inferred back from text.
/// For supplied sentence splits, only the given non-overlapping intervals are
/// candidate text. Missing/partial splits leave structural source residual
/// *regions*, not silently invented sentences.
pub fn plan_tircorder_sessions(
    raw_packet: &str,
    source_collection_ref: &str,
    acquisition_receipt_ref: &str,
) -> Result<Vec<TircorderSourcePlan>, TircorderBridgeError> {
    if !present(source_collection_ref) || !present(acquisition_receipt_ref) {
        return Err(TircorderBridgeError::MissingIdentity);
    }
    let packet: TircorderPacket = serde_json::from_str(raw_packet)?;
    if packet.connector != "tircorder"
        || !present(&packet.batch_id) || !present(&packet.ingested_at)
        || packet.sessions.is_empty()
    {
        return Err(TircorderBridgeError::InvalidPacket);
    }
    let packet_digest = digest(raw_packet);
    let mut seen_sessions = BTreeSet::new();
    let mut out = Vec::with_capacity(packet.sessions.len());
    for (session_index, session) in packet.sessions.iter().enumerate() {
        if !present(&session.session_id) || !present(&session.device_id)
            || session.utterances.is_empty()
            || !seen_sessions.insert(session.session_id.clone())
        {
            return Err(TircorderBridgeError::DuplicateIdentity);
        }
        let audio_hash_ref = session.audio_sha256.as_deref()
            .map(normalized_audio_hash).transpose()?;
        // Source revision derives from original packet provenance plus this
        // session, so a changed device/audio/word/timing packet is a revision.
        // Length-tagged tuple avoids cross-collection/session delimiter collisions.
        let coordinate = format!(
            "{}:{}|{}:{}",source_collection_ref.len(),source_collection_ref,
            session.session_id.len(),session.session_id,
        );
        let source_ref = format!("source:tircorder:{}",digest(&coordinate));
        let source_revision_ref = format!(
            "source-revision:tircorder:{}:{}",
            digest(&source_ref), digest(&format!("{packet_digest}:{session_index}"))
        );
        let mut canonical_text = String::new();
        let mut regions = Vec::new();
        let mut position = 0u64;
        let mut speaker_missing = 0usize;
        let mut seen_utterances = BTreeSet::new();
        for (index, utt) in session.utterances.iter().enumerate() {
            if !present(&utt.utterance_id) || !seen_utterances.insert(utt.utterance_id.clone()) {
                return Err(TircorderBridgeError::DuplicateIdentity);
            }
            if !present(&utt.start) || !present(&utt.end) || utt.text.is_empty()
                || utt.speaker_confidence.is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
            {
                return Err(TircorderBridgeError::InvalidCoordinate);
            }
            if !utt.speaker_label.as_deref().is_some_and(present) {
                speaker_missing += 1;
            }
            let mut word_end = 0.0_f64;
            for (widx, word) in utt.words.iter().enumerate() {
                if !present(&word.w) || !word.start.is_finite() || !word.end.is_finite()
                    || word.start < 0.0 || word.end <= word.start
                    || (widx > 0 && word.start < word_end)
                    || word.conf.is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
                {
                    return Err(TircorderBridgeError::InvalidWordTiming);
                }
                word_end = word.end;
            }
            if index != 0 {
                let begin = position;
                canonical_text.push('\n');
                position += 1;
                regions.push(SourceExecutionRegion {
                    region_ref: format!("tircorder-boundary:{source_revision_ref}:{index}"),
                    source_revision_ref: source_revision_ref.clone(),
                    start_char: begin, end_char: position,
                    class: SourceRegionExecutionClass::StructuralOnly,
                });
            }
            let utterance_start = position;
            canonical_text.push_str(&utt.text);
            position += utt.text.chars().count() as u64;
            if position > u32::MAX as u64 { return Err(TircorderBridgeError::SpanOverflow); }
            let mut splits = utt.sentence_splits.clone();
            if splits.is_empty() {
                splits.push(TircorderSentenceSplit {
                    char_start: 0, char_end: utt.text.chars().count() as u64,
                });
            }
            splits.sort_by_key(|split| (split.char_start,split.char_end));
            let mut cursor = 0u64;
            let text_len = utt.text.chars().count() as u64;
            for (split_index, split) in splits.iter().enumerate() {
                if split.char_start < cursor || split.char_start >= split.char_end
                    || split.char_end > text_len
                {
                    return Err(TircorderBridgeError::InvalidSentenceSplit);
                }
                if cursor < split.char_start {
                    regions.push(SourceExecutionRegion {
                        region_ref: format!("tircorder-gap:{source_revision_ref}:{index}:{cursor}-{}",split.char_start),
                        source_revision_ref: source_revision_ref.clone(),
                        start_char: utterance_start + cursor,
                        end_char: utterance_start + split.char_start,
                        class: SourceRegionExecutionClass::StructuralOnly,
                    });
                }
                regions.push(SourceExecutionRegion {
                    region_ref: format!(
                        "tircorder-sentence:{source_revision_ref}:{}:{}",
                        utt.utterance_id,split_index
                    ),
                    source_revision_ref: source_revision_ref.clone(),
                    start_char: utterance_start + split.char_start,
                    end_char: utterance_start + split.char_end,
                    class: SourceRegionExecutionClass::SemanticCandidate,
                });
                cursor = split.char_end;
            }
            if cursor < text_len {
                regions.push(SourceExecutionRegion {
                    region_ref: format!("tircorder-gap:{source_revision_ref}:{index}:{cursor}-{text_len}"),
                    source_revision_ref: source_revision_ref.clone(),
                    start_char: utterance_start + cursor,
                    end_char: utterance_start + text_len,
                    class: SourceRegionExecutionClass::StructuralOnly,
                });
            }
        }
        out.push(TircorderSourcePlan {
            bridge_version: TIRCORDER_SESSION_BRIDGE_VERSION,
            source_collection_ref: source_collection_ref.to_owned(),
            envelope: SourceIngestEnvelope {
                source_ref, source_revision_ref,
                provider_ref: "tircorder:normalized-session".into(),
                family: SourceFamily::Transcript,
                role_class: IngestRoleClass::ContentSource,
                content_digest_ref: digest(&canonical_text),
                acquisition_receipt_ref: acquisition_receipt_ref.to_owned(),
                media_type_ref: "text/plain".into(),
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
            raw_packet: raw_packet.to_owned(),
            raw_packet_sha256: packet_digest.clone(),
            session_index, native: session.clone(), canonical_text, regions,
            missing_speaker_count: speaker_missing,
            audio_hash_ref,
            speaker_identity_established: false,
            audio_completeness_established: false,
        });
    }
    Ok(out)
}

pub fn compile_tircorder_session_lossless<P: CandidatePnfProducer>(
    producer: &P, plan: &TircorderSourcePlan,
    statement_document_ref: &str, parser_receipt_prefix: &str,
) -> Result<LosslessBulkSourceCompilation,TircorderBridgeError> {
    Ok(compile_source_regions_lossless(
        producer, &plan.envelope.source_ref, &plan.envelope.source_revision_ref,
        statement_document_ref, &plan.canonical_text, &plan.regions,
        parser_receipt_prefix,
    )?)
}

pub const TIRCORDER_PROVENANCE_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS ingest;
CREATE TABLE IF NOT EXISTS ingest.tircorder_batch_identity (
    source_collection_ref TEXT NOT NULL,
    batch_id TEXT NOT NULL,
    packet_digest_ref TEXT NOT NULL,
    PRIMARY KEY (source_collection_ref,batch_id)
);
CREATE TABLE IF NOT EXISTS ingest.tircorder_session_source (
    source_revision_ref TEXT PRIMARY KEY
        REFERENCES ingest.generic_source_revision(source_revision_ref),
    bridge_version TEXT NOT NULL,
    source_collection_ref TEXT NOT NULL,
    packet_digest_ref TEXT NOT NULL,
    raw_packet TEXT NOT NULL,
    session_index BIGINT NOT NULL CHECK (session_index >= 0),
    audio_hash_ref TEXT,
    missing_speaker_count BIGINT NOT NULL CHECK (missing_speaker_count >= 0),
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);
"#;

pub fn persist_tircorder_session(
    config: &crate::DatabaseConfig,
    plan: &TircorderSourcePlan,
) -> Result<crate::PersistedGenericSourceContent,TircorderBridgeError> {
    let packet: TircorderPacket = serde_json::from_str(&plan.raw_packet)?;
    let collection = &plan.source_collection_ref;
    let mut client = postgres::Client::connect(config.database_url(), postgres::NoTls)?;
    client.batch_execute(TIRCORDER_PROVENANCE_SQL)?;
    // Native packet batch ids have producer-scoped immutable identity. A
    // same-batch/different-content resend must not be admitted as a revision.
    client.execute(
        "INSERT INTO ingest.tircorder_batch_identity
         (source_collection_ref,batch_id,packet_digest_ref)
         VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        &[collection,&packet.batch_id,&plan.raw_packet_sha256],
    )?;
    let recorded: String = client.query_one(
        "SELECT packet_digest_ref FROM ingest.tircorder_batch_identity
         WHERE source_collection_ref=$1 AND batch_id=$2",
        &[collection,&packet.batch_id],
    )?.get(0);
    if recorded != plan.raw_packet_sha256 {
        return Err(TircorderBridgeError::BatchIdentityConflict);
    }
    let source = crate::persist_generic_text_source(
        config, &plan.envelope, &plan.canonical_text
    )?;
    client.execute(
        "INSERT INTO ingest.tircorder_session_source
         (source_revision_ref, bridge_version, source_collection_ref,
          packet_digest_ref, raw_packet, session_index, audio_hash_ref,
          missing_speaker_count, candidate_only,
          creates_semantic_authority, claim_truth_promoted)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,TRUE,FALSE,FALSE)
         ON CONFLICT (source_revision_ref) DO NOTHING",
        &[
            &plan.envelope.source_revision_ref,
            &plan.bridge_version,
            &plan.source_collection_ref,
            &plan.raw_packet_sha256,
            &plan.raw_packet,
            &(plan.session_index as i64),
            &plan.audio_hash_ref,
            &(plan.missing_speaker_count as i64),
        ],
    )?;
    let row = client.query_one(
        "SELECT bridge_version, source_collection_ref, packet_digest_ref,
                raw_packet, session_index, audio_hash_ref, missing_speaker_count,
                candidate_only, creates_semantic_authority, claim_truth_promoted
         FROM ingest.tircorder_session_source WHERE source_revision_ref=$1",
        &[&plan.envelope.source_revision_ref],
    )?;
    if row.get::<_, String>(0) != plan.bridge_version
        || row.get::<_, String>(1) != plan.source_collection_ref
        || row.get::<_, String>(2) != plan.raw_packet_sha256
        || row.get::<_, String>(3) != plan.raw_packet
        || row.get::<_, i64>(4) != plan.session_index as i64
        || row.get::<_, Option<String>>(5) != plan.audio_hash_ref
        || row.get::<_, i64>(6) != plan.missing_speaker_count as i64
        || !row.get::<_, bool>(7) || row.get::<_, bool>(8) || row.get::<_, bool>(9)
    {
        return Err(TircorderBridgeError::ReopenMismatch);
    }
    Ok(source)
}

pub fn reopen_tircorder_session(
    config: &crate::DatabaseConfig, source_revision_ref: &str,
) -> Result<TircorderSourcePlan,TircorderBridgeError> {
    let (source, canonical) = crate::load_generic_text_source(config, source_revision_ref)?;
    let mut client = postgres::Client::connect(config.database_url(), postgres::NoTls)?;
    let row = client.query_one(
        "SELECT bridge_version, source_collection_ref, packet_digest_ref,
                raw_packet, session_index, audio_hash_ref, missing_speaker_count
         FROM ingest.tircorder_session_source WHERE source_revision_ref=$1",
        &[&source_revision_ref],
    )?;
    let raw: String = row.get(3);
    let collection_ref: String = row.get(1);
    let plans = plan_tircorder_sessions(
        &raw, &collection_ref, &source.acquisition_receipt_ref,
    )?;
    let index = row.get::<_,i64>(4) as usize;
    let plan = plans.into_iter().nth(index).ok_or(TircorderBridgeError::ReopenMismatch)?;
    if plan.bridge_version != row.get::<_, String>(0)
        || plan.source_collection_ref != collection_ref
        || plan.raw_packet_sha256 != row.get::<_, String>(2)
        || plan.audio_hash_ref != row.get::<_, Option<String>>(5)
        || plan.missing_speaker_count as i64 != row.get::<_,i64>(6)
        || plan.envelope.source_revision_ref != source_revision_ref
        || plan.envelope.source_ref != source.source_ref
        || plan.envelope.content_digest_ref != source.content_digest_ref
        || plan.canonical_text != canonical
    {
        return Err(TircorderBridgeError::ReopenMismatch);
    }
    Ok(plan)
}

pub fn prepare_tircorder_parser_run(
    config: &crate::DatabaseConfig, plan: &TircorderSourcePlan,
    parser_family: &str, parser_version: &str,
    model_ref: &str, parser_config_json: &str,
) -> Result<(crate::ParserRunReceipt,crate::ParserEnqueueReceipt),TircorderBridgeError> {
    persist_tircorder_session(config,plan)?;
    let run = crate::start_parser_run(
        config,&plan.envelope.source_revision_ref,
        parser_family,parser_version,model_ref,parser_config_json,
    )?;
    let jobs = plan.regions.iter().filter(|r|
        r.class == SourceRegionExecutionClass::SemanticCandidate
    ).map(|r| crate::ParserRegionJobSpec {
        region_ref:r.region_ref.clone(),
        start_char:r.start_char,end_char:r.end_char,
    }).collect::<Vec<_>>();
    let queued = crate::enqueue_parser_regions_with_content_reuse(
        config,&run,&jobs,&plan.canonical_text
    )?;
    Ok((run,queued))
}

pub fn finalize_tircorder_parser_run(
    config: &crate::DatabaseConfig, parser_run_ref: &str,
) -> Result<crate::GenericCandidatePersistenceReceipt,TircorderBridgeError> {
    let snapshot = crate::DbNativeParserSnapshot::load(config,parser_run_ref)?;
    let plan = reopen_tircorder_session(config,snapshot.source_revision_ref())?;
    let state = crate::parser_run_state(config,parser_run_ref)?;
    let n = plan.regions.iter().filter(|r|
        r.class == SourceRegionExecutionClass::SemanticCandidate
    ).count();
    if state.queued != 0 || state.leased != 0
        || state.unattempted_semantic_regions != 0
        || state.succeeded + state.residual != n
        || snapshot.compiled_region_count() != state.succeeded
        || snapshot.residual_region_count() != state.residual
    { return Err(TircorderBridgeError::ParserMismatch); }
    let (source,_) = crate::load_generic_text_source(
        config,snapshot.source_revision_ref()
    )?;
    let compiled = compile_tircorder_session_lossless(
        &snapshot,&plan,&source.document_ref,
        &format!("db-parser:{parser_run_ref}")
    )?;
    if !compiled.source_coverage_complete() {
        return Err(TircorderBridgeError::ParserMismatch);
    }
    Ok(crate::persist_lossless_generic_candidates(config,&compiled)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalized_packet_keeps_native_session_word_and_missing_speaker() {
        let raw = r#"{"connector":"tircorder","batch_id":"batch:1","ingested_at":"2026-09-29T00:00:00Z","sessions":[{"session_id":"sess:1","device_id":"device:1","audio_sha256":null,"utterances":[{"utterance_id":"utt:1","start":"2026-09-29T00:00:01Z","end":"2026-09-29T00:00:02Z","text":"Écho. Later.","speaker_label":null,"words":[{"w":"Écho","start":0.1,"end":0.4}],"sentence_splits":[{"char_start":0,"char_end":5},{"char_start":6,"char_end":12}]}]}]}"#;
        let plans=plan_tircorder_sessions(raw,"collection:1","acquisition:1").unwrap();
        let p=&plans[0];
        assert_eq!(p.native.utterances[0].words[0].w,"Écho");
        assert!(p.native.audio_sha256.is_none());
        assert_eq!(p.missing_speaker_count,1);
        assert_eq!(p.regions.iter().filter(|r|r.class == SourceRegionExecutionClass::SemanticCandidate).count(),2);
        assert!(p.regions.iter().any(|r|r.class == SourceRegionExecutionClass::StructuralOnly));
        assert!(!p.speaker_identity_established);
        assert!(!p.audio_completeness_established);
    }
}
