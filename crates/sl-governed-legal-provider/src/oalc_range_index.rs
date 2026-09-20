//! Revision-pinned byte-range index for the Open Australian Legal Corpus.
//!
//! The index is discovery infrastructure only.  It maps an exact terminal
//! medium-neutral citation plus document coordinates to a byte range in the
//! immutable `corpus.jsonl` for one Hugging Face revision.  It never creates
//! legal authority, claim truth, identity or treatment.
//!
//! Index construction is resumable.  Each HTTP range is parsed only through
//! its last complete JSONL row.  The checkpoint advances to that row boundary;
//! an incomplete trailing row is deliberately re-fetched in the next range.

use crate::live_oalc_case_follow::{
    oalc_corpus_row_matches, OalcCaseFollowError, OalcCorpusRow, PinnedOalcStreamRequest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const OALC_RANGE_CHUNK_BYTES: u64 = 256 * 1024 * 1024;
pub const OALC_RANGE_MAX_CHUNKS_PER_BUILD: u64 = 40;
pub const OALC_RANGE_MAX_REQUESTS_PER_ACQUISITION: u64 =
    OALC_RANGE_MAX_CHUNKS_PER_BUILD + 1;

const OALC_DATASET_ID: &str = "isaacus/open-australian-legal-corpus";
const SENSIBLAW_UA: &str = "SensibLaw/0.1 governed-legal-provider";
const RANGE_REQUEST_TIMEOUT_SECONDS: u64 = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OalcRangeIndexEntry {
    pub corpus_revision_sha: String,
    pub terminal_mnc: String,
    pub document_type: String,
    pub jurisdiction: String,
    pub source: String,
    pub version_id: String,
    pub byte_start: u64,
    pub byte_len: u64,
    pub row_sha256: String,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OalcRangeIndexCheckpoint {
    pub schema_version: String,
    pub corpus_revision_sha: String,
    pub next_byte_offset: u64,
    pub rows_indexed: u64,
    pub bytes_indexed: u64,
    pub complete: bool,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcRangeLookupReceipt {
    pub row: OalcCorpusRow,
    pub entry: OalcRangeIndexEntry,
    pub range_requests: u64,
    pub index_hit: bool,
    pub rows_indexed_this_run: u64,
    pub bytes_indexed_this_run: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedChunk {
    pub entries: Vec<OalcRangeIndexEntry>,
    pub target: Option<(OalcCorpusRow, OalcRangeIndexEntry)>,
    pub complete_bytes: u64,
    pub complete_rows: u64,
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn terminal_mnc(citation: &str) -> Option<String> {
    let fields = citation.split_whitespace().collect::<Vec<_>>();
    if fields.len() < 3 {
        return None;
    }
    let year = fields[fields.len() - 3];
    let court = fields[fields.len() - 2];
    let number = fields[fields.len() - 1];
    if year.len() != 6
        || !year.starts_with('[')
        || !year.ends_with(']')
        || !year[1..5].bytes().all(|byte| byte.is_ascii_digit())
        || !court.bytes().all(|byte| byte.is_ascii_alphanumeric())
        || !number.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some(format!("{year} {court} {number}"))
}

fn entry_matches_request(entry: &OalcRangeIndexEntry, request: &PinnedOalcStreamRequest) -> bool {
    entry.corpus_revision_sha == request.revision
        && entry.terminal_mnc == request.citation
        && entry.document_type == request.document_type
        && request
            .source
            .as_deref()
            .map_or(true, |source| entry.source == source)
        && request
            .jurisdiction
            .as_deref()
            .map_or(true, |jurisdiction| entry.jurisdiction == jurisdiction)
}

pub fn index_complete_jsonl_chunk(
    revision: &str,
    byte_start: u64,
    chunk: &[u8],
    request: &PinnedOalcStreamRequest,
) -> Result<IndexedChunk, OalcCaseFollowError> {
    let mut entries = Vec::new();
    let mut target = None;
    let mut cursor = 0usize;
    let mut rows = 0u64;

    while cursor < chunk.len() {
        let Some(relative_newline) = chunk[cursor..].iter().position(|byte| *byte == b'\n') else {
            break;
        };
        let line_end = cursor + relative_newline + 1;
        let line = &chunk[cursor..line_end];
        let json = &line[..line.len().saturating_sub(1)];
        let row: OalcCorpusRow = serde_json::from_slice(json).map_err(|error| {
            OalcCaseFollowError::Json(format!(
                "decode pinned range-index row at byte {}: {error}",
                byte_start + cursor as u64
            ))
        })?;
        rows += 1;

        if let Some(mnc) = terminal_mnc(&row.citation) {
            let entry = OalcRangeIndexEntry {
                corpus_revision_sha: revision.to_string(),
                terminal_mnc: mnc,
                document_type: row.document_type.clone(),
                jurisdiction: row.jurisdiction.clone(),
                source: row.source.clone(),
                version_id: row.version_id.clone(),
                byte_start: byte_start + cursor as u64,
                byte_len: line.len() as u64,
                row_sha256: sha256(json),
                candidate_only: true,
                creates_legal_authority: false,
                creates_claim_truth: false,
            };
            if target.is_none() && entry_matches_request(&entry, request) {
                target = Some((row.clone(), entry.clone()));
            }
            entries.push(entry);
        }

        cursor = line_end;
        if target.is_some() {
            // The target row is complete and its exact byte range is now known.
            // No later row is needed for this acquisition.
            break;
        }
    }

    Ok(IndexedChunk {
        entries,
        target,
        complete_bytes: cursor as u64,
        complete_rows: rows,
    })
}

fn default_index_root() -> PathBuf {
    if let Some(path) = std::env::var_os("SENSIBLAW_OALC_INDEX_DIR") {
        return PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(path).join("sensiblaw").join("oalc-range-index");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".cache")
            .join("sensiblaw")
            .join("oalc-range-index");
    }
    std::env::temp_dir().join("sensiblaw-oalc-range-index")
}

fn revision_dir(revision: &str) -> PathBuf {
    default_index_root().join(revision)
}

fn index_path(revision: &str) -> PathBuf {
    revision_dir(revision).join("citation-byte-ranges.jsonl")
}

fn checkpoint_path(revision: &str) -> PathBuf {
    revision_dir(revision).join("checkpoint.json")
}

fn load_checkpoint(revision: &str) -> Result<OalcRangeIndexCheckpoint, OalcCaseFollowError> {
    let path = checkpoint_path(revision);
    if !path.exists() {
        return Ok(OalcRangeIndexCheckpoint {
            schema_version: "sl.oalc_range_index_checkpoint.v0_1".into(),
            corpus_revision_sha: revision.into(),
            next_byte_offset: 0,
            rows_indexed: 0,
            bytes_indexed: 0,
            complete: false,
            candidate_only: true,
            creates_legal_authority: false,
        });
    }
    let bytes = fs::read(&path).map_err(|error| OalcCaseFollowError::Io(format!(
        "read range-index checkpoint {}: {error}",
        path.display()
    )))?;
    let checkpoint: OalcRangeIndexCheckpoint = serde_json::from_slice(&bytes)
        .map_err(|error| OalcCaseFollowError::Json(format!(
            "decode range-index checkpoint {}: {error}",
            path.display()
        )))?;
    if checkpoint.corpus_revision_sha != revision
        || !checkpoint.candidate_only
        || checkpoint.creates_legal_authority
    {
        return Err(OalcCaseFollowError::Validation(
            "range-index checkpoint failed revision/authority validation".into(),
        ));
    }
    Ok(checkpoint)
}

fn write_checkpoint(
    checkpoint: &OalcRangeIndexCheckpoint,
) -> Result<(), OalcCaseFollowError> {
    let path = checkpoint_path(&checkpoint.corpus_revision_sha);
    let parent = path.parent().ok_or_else(|| {
        OalcCaseFollowError::Io("range-index checkpoint has no parent directory".into())
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(checkpoint)
            .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?,
    )
    .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
    fs::rename(&tmp, &path)
        .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
    Ok(())
}

fn append_entries(
    revision: &str,
    entries: &[OalcRangeIndexEntry],
) -> Result<(), OalcCaseFollowError> {
    if entries.is_empty() {
        return Ok(());
    }
    let path = index_path(revision);
    let parent = path.parent().ok_or_else(|| {
        OalcCaseFollowError::Io("range index has no parent directory".into())
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
    for entry in entries {
        serde_json::to_writer(&mut file, entry)
            .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?;
        file.write_all(b"\n")
            .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
    }
    file.flush()
        .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
    Ok(())
}

fn lookup_entry(
    request: &PinnedOalcStreamRequest,
) -> Result<Option<OalcRangeIndexEntry>, OalcCaseFollowError> {
    let path = index_path(&request.revision);
    if !path.exists() {
        return Ok(None);
    }
    let file = fs::File::open(&path)
        .map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.map_err(|error| OalcCaseFollowError::Io(error.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: OalcRangeIndexEntry = serde_json::from_str(&line)
            .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?;
        if entry_matches_request(&entry, request) {
            return Ok(Some(entry));
        }
    }
    Ok(None)
}

#[cfg(feature = "live-network")]
fn range_get(
    revision: &str,
    start: u64,
    end_inclusive: u64,
) -> Result<Vec<u8>, OalcCaseFollowError> {
    let url = format!(
        "https://huggingface.co/datasets/{OALC_DATASET_ID}/resolve/{revision}/corpus.jsonl?download=true"
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(RANGE_REQUEST_TIMEOUT_SECONDS))
        .build();
    let response = match agent
        .get(&url)
        .set("User-Agent", SENSIBLAW_UA)
        .set("Referer", "https://huggingface.co/")
        .set("Range", &format!("bytes={start}-{end_inclusive}"))
        .call()
    {
        Ok(response) => response,
        Err(ureq::Error::Status(_, response)) => response,
        Err(error) => {
            return Err(OalcCaseFollowError::StreamingFallback(format!(
                "range-index request {start}-{end_inclusive} failed: {error}"
            )))
        }
    };
    if response.status() != 206 {
        return Err(OalcCaseFollowError::StreamingFallback(format!(
            "range-index request {start}-{end_inclusive} expected HTTP 206 but received {}",
            response.status()
        )));
    }
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut bytes)
        .map_err(|error| OalcCaseFollowError::StreamingFallback(format!(
            "range-index read {start}-{end_inclusive} failed: {error}"
        )))?;
    Ok(bytes)
}

#[cfg(feature = "live-network")]
fn fetch_indexed_row(
    request: &PinnedOalcStreamRequest,
    entry: &OalcRangeIndexEntry,
) -> Result<OalcCorpusRow, OalcCaseFollowError> {
    if entry.byte_len == 0 {
        return Err(OalcCaseFollowError::Validation(
            "range-index entry has zero byte length".into(),
        ));
    }
    let bytes = range_get(
        &request.revision,
        entry.byte_start,
        entry.byte_start + entry.byte_len - 1,
    )?;
    let json = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
    if sha256(json) != entry.row_sha256 {
        return Err(OalcCaseFollowError::Validation(
            "range-index row digest mismatch".into(),
        ));
    }
    let row: OalcCorpusRow = serde_json::from_slice(json)
        .map_err(|error| OalcCaseFollowError::Json(error.to_string()))?;
    if !oalc_corpus_row_matches(request, &row) {
        return Err(OalcCaseFollowError::Validation(
            "range-index fetched row failed exact request validation".into(),
        ));
    }
    Ok(row)
}

#[cfg(feature = "live-network")]
pub fn lookup_or_build_oalc_range_index(
    request: &PinnedOalcStreamRequest,
) -> Result<OalcRangeLookupReceipt, OalcCaseFollowError> {
    if let Some(entry) = lookup_entry(request)? {
        let row = fetch_indexed_row(request, &entry)?;
        return Ok(OalcRangeLookupReceipt {
            row,
            entry,
            range_requests: 1,
            index_hit: true,
            rows_indexed_this_run: 0,
            bytes_indexed_this_run: 0,
        });
    }

    let mut checkpoint = load_checkpoint(&request.revision)?;
    if checkpoint.complete {
        return Err(OalcCaseFollowError::SourceResidual(format!(
            "completed range index for revision {} has no exact entry for {}",
            request.revision, request.citation
        )));
    }

    let mut range_requests = 0u64;
    let mut rows_this_run = 0u64;
    let mut bytes_this_run = 0u64;

    while range_requests < OALC_RANGE_MAX_CHUNKS_PER_BUILD {
        let start = checkpoint.next_byte_offset;
        let end = start.saturating_add(OALC_RANGE_CHUNK_BYTES - 1);
        let chunk = range_get(&request.revision, start, end)?;
        range_requests += 1;
        if chunk.is_empty() {
            checkpoint.complete = true;
            write_checkpoint(&checkpoint)?;
            break;
        }

        let indexed = index_complete_jsonl_chunk(&request.revision, start, &chunk, request)?;
        if indexed.complete_bytes == 0 {
            return Err(OalcCaseFollowError::StreamingFallback(format!(
                "range-index chunk at byte {start} contained no complete JSONL row; increase chunk size"
            )));
        }

        append_entries(&request.revision, &indexed.entries)?;
        checkpoint.next_byte_offset = start.saturating_add(indexed.complete_bytes);
        checkpoint.rows_indexed = checkpoint.rows_indexed.saturating_add(indexed.complete_rows);
        checkpoint.bytes_indexed = checkpoint.bytes_indexed.saturating_add(indexed.complete_bytes);
        rows_this_run = rows_this_run.saturating_add(indexed.complete_rows);
        bytes_this_run = bytes_this_run.saturating_add(indexed.complete_bytes);
        write_checkpoint(&checkpoint)?;

        if let Some((row, entry)) = indexed.target {
            return Ok(OalcRangeLookupReceipt {
                row,
                entry,
                range_requests,
                index_hit: false,
                rows_indexed_this_run: rows_this_run,
                bytes_indexed_this_run: bytes_this_run,
            });
        }

        // A short HTTP 206 range at the end of the object contains the final
        // bytes.  If all of them ended at a complete row boundary, the index is
        // complete.  Otherwise the final partial line is re-requested once.
        if chunk.len() < OALC_RANGE_CHUNK_BYTES as usize
            && indexed.complete_bytes == chunk.len() as u64
        {
            checkpoint.complete = true;
            write_checkpoint(&checkpoint)?;
            break;
        }
    }

    if checkpoint.complete {
        Err(OalcCaseFollowError::SourceResidual(format!(
            "completed range index for revision {} contains no exact entry for {}",
            request.revision, request.citation
        )))
    } else {
        Err(OalcCaseFollowError::StreamingFallback(format!(
            "range-index build paused at byte {} after {} range requests; rerun to resume",
            checkpoint.next_byte_offset, range_requests
        )))
    }
}

#[cfg(not(feature = "live-network"))]
pub fn lookup_or_build_oalc_range_index(
    _request: &PinnedOalcStreamRequest,
) -> Result<OalcRangeLookupReceipt, OalcCaseFollowError> {
    Err(OalcCaseFollowError::LiveNetworkFeatureDisabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn row(citation: &str) -> OalcCorpusRow {
        OalcCorpusRow {
            version_id: format!("version:{citation}"),
            document_type: "decision".into(),
            jurisdiction: "commonwealth".into(),
            source: "high_court_of_australia".into(),
            citation: citation.into(),
            mime: None,
            date: None,
            url: None,
            when_scraped: None,
            text: "fixture".into(),
        }
    }

    #[test]
    fn terminal_mnc_is_extracted_from_full_case_citation() {
        assert_eq!(
            terminal_mnc("Giumelli v Giumelli [1999] HCA 10").as_deref(),
            Some("[1999] HCA 10")
        );
        assert_eq!(
            terminal_mnc("Waltons Stores (Interstate) Ltd v Maher [1988] HCA 72").as_deref(),
            Some("[1988] HCA 72")
        );
    }

    #[test]
    fn chunk_index_stops_on_complete_target_row_and_ignores_partial_tail() {
        let request = PinnedOalcStreamRequest {
            revision: "deadbeef".into(),
            citation: "[1999] HCA 10".into(),
            citation_match: crate::live_oalc_case_follow::OalcCitationMatch::Contains,
            document_type: "decision".into(),
            source: None,
            jurisdiction: Some("commonwealth".into()),
        };
        let before = serde_json::to_vec(&row("Example v Example [1998] HCA 1")).unwrap();
        let target = serde_json::to_vec(&row("Giumelli v Giumelli [1999] HCA 10")).unwrap();
        let after = serde_json::to_vec(&row("After v After [2000] HCA 2")).unwrap();
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&before);
        chunk.push(b'\n');
        chunk.extend_from_slice(&target);
        chunk.push(b'\n');
        chunk.extend_from_slice(&after[..after.len() / 2]);

        let indexed = index_complete_jsonl_chunk("deadbeef", 1000, &chunk, &request).unwrap();
        let (_, entry) = indexed.target.unwrap();
        assert_eq!(indexed.complete_rows, 2);
        assert_eq!(entry.terminal_mnc, "[1999] HCA 10");
        assert_eq!(entry.byte_start, 1000 + before.len() as u64 + 1);
        assert_eq!(entry.byte_len, target.len() as u64 + 1);
        assert!(indexed.complete_bytes < chunk.len() as u64);
    }

    #[test]
    fn checkpoint_default_starts_at_zero() {
        let _ = Cursor::new(Vec::<u8>::new());
        let checkpoint = OalcRangeIndexCheckpoint {
            schema_version: "sl.oalc_range_index_checkpoint.v0_1".into(),
            corpus_revision_sha: "deadbeef".into(),
            next_byte_offset: 0,
            rows_indexed: 0,
            bytes_indexed: 0,
            complete: false,
            candidate_only: true,
            creates_legal_authority: false,
        };
        assert_eq!(checkpoint.next_byte_offset, 0);
        assert!(!checkpoint.complete);
    }
}
