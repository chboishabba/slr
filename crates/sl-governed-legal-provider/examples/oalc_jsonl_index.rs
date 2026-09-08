use sensiblaw_governed_legal_provider::{lookup_oalc_exact_mnc, OalcRecord, OalcSnapshot, RECEIPT_AUTHORITY};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct OalcJsonLine {
    version_id: String,
    source: String,
    citation: String,
    text: String,
}

fn extract_mnc(citation: &str) -> Option<String> {
    let parts: Vec<_> = citation.split_whitespace().collect();
    for window in parts.windows(3) {
        let year = window[0].trim_matches(['[', ']']);
        let court = window[1];
        let number = window[2].trim_matches(|c: char| !c.is_ascii_digit());
        if window[0].starts_with('[')
            && window[0].ends_with(']')
            && year.len() == 4
            && year.parse::<u32>().is_ok()
            && !court.is_empty()
            && court.chars().all(|c| c.is_ascii_uppercase())
            && number.parse::<u32>().is_ok()
        {
            return Some(format!("[{year}] {court} {number}"));
        }
    }
    None
}

fn main() {
    let path = PathBuf::from(
        std::env::var("SENSIBLAW_OALC_JSONL")
            .unwrap_or_else(|_| "crates/sl-governed-legal-provider/fixtures/oalc-mini.jsonl".into()),
    );
    let revision = std::env::var("SENSIBLAW_OALC_REVISION")
        .unwrap_or_else(|_| "oalc:fixture-v1".into());

    let reader = BufReader::new(File::open(&path).expect("open OALC JSONL"));
    let mut records = Vec::new();
    let mut line_count = 0usize;
    for (index, line) in reader.lines().enumerate() {
        let line = line.expect("read OALC JSONL line");
        if line.trim().is_empty() {
            continue;
        }
        line_count += 1;
        let record: OalcJsonLine = serde_json::from_str(&line)
            .unwrap_or_else(|err| panic!("invalid OALC JSONL line {}: {err}", index + 1));
        let Some(mnc) = extract_mnc(&record.citation) else {
            continue;
        };
        let digest = format!("sha256:{:x}", Sha256::digest(record.text.as_bytes()));
        records.push(OalcRecord {
            citation: mnc,
            source_identity_ref: format!("oalc:{}:{}", record.source, record.version_id),
            source_revision_ref: format!("{}:{}", revision, record.version_id),
            canonical_text_digest: digest,
            local_artifact_ref: format!("oalc://{}/{}", revision, record.version_id),
        });
    }

    let snapshot = OalcSnapshot {
        corpus_revision_ref: revision.clone(),
        records,
    };
    let cullen = lookup_oalc_exact_mnc(&snapshot, "[2026] HCA 19")
        .expect("fixture must index Cullen by MNC");
    let pabai = lookup_oalc_exact_mnc(&snapshot, "[2025] FCA 796")
        .expect("fixture must index Pabai by MNC");

    assert_eq!(cullen.network_requests, 0);
    assert_eq!(pabai.network_requests, 0);
    assert_eq!(cullen.receipt_authority, RECEIPT_AUTHORITY);
    assert_ne!(cullen.source_identity_ref, pabai.source_identity_ref);

    println!(
        "oalc_revision={} lines={} indexed={} cullen_network=0 pabai_network=0 authority={}",
        revision,
        line_count,
        snapshot.records.len(),
        RECEIPT_AUTHORITY
    );
}
