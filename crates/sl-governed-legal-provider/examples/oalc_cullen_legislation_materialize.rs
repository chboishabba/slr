#[path = "../src/oalc_legislation_contract.rs"]
mod oalc_legislation_contract;

use oalc_legislation_contract::{
    OalcLegislationDemand, OalcResolvedDocumentReceipt, OalcTemporalCoverage,
    OfflineOalcJsonlBackend, PinnedOalcDatasetSelection, CULLEN_CLA_CITATION,
    CULLEN_VICARIOUS_CITATION, OALC_CONFIG, OALC_DATASET_ID, OALC_RECEIPT_AUTHORITY,
    OALC_SPLIT,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const TARGETS: [&str; 2] = [CULLEN_CLA_CITATION, CULLEN_VICARIOUS_CITATION];

#[derive(Debug, Deserialize)]
struct OalcJsonLine {
    version_id: String,
    #[serde(rename = "type")]
    document_type: String,
    jurisdiction: String,
    source: String,
    citation: String,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    when_scraped: Option<String>,
    text: String,
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn slug(citation: &str) -> String {
    citation
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn readonly_write(path: &Path, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text.as_bytes())?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn tsv(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], " ")
}

fn demand(citation: &str) -> OalcLegislationDemand {
    OalcLegislationDemand {
        demand_ref: format!("offline:oalc:{citation}"),
        citation: citation.into(),
        jurisdiction: "new_south_wales".into(),
        source: "nsw_legislation".into(),
        document_type: "primary_legislation".into(),
        temporal_coverage_required: OalcTemporalCoverage::LatestKnownOnly,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = OfflineOalcJsonlBackend {
        corpus_jsonl_path: PathBuf::from(
            env::var("SENSIBLAW_OALC_JSONL").unwrap_or_else(|_| "corpus.jsonl".to_string()),
        ),
    };
    let corpus_revision = env::var("SENSIBLAW_OALC_REVISION")
        .unwrap_or_else(|_| "oalc:revision-unset".to_string());
    let output_dir = PathBuf::from(
        env::var("SENSIBLAW_OALC_OUTPUT")
            .unwrap_or_else(|_| "artifacts/oalc/cullen-governing-law".to_string()),
    );

    let dataset = PinnedOalcDatasetSelection {
        dataset_id: OALC_DATASET_ID.into(),
        config: OALC_CONFIG.into(),
        split: OALC_SPLIT.into(),
        corpus_revision_ref: corpus_revision.clone(),
    };
    dataset
        .validate()
        .map_err(|err| format!("invalid pinned OALC dataset selection: {err:?}"))?;
    if !backend.corpus_jsonl_path.is_file() {
        return Err(format!(
            "offline OALC JSONL backend missing: {}",
            backend.corpus_jsonl_path.display()
        )
        .into());
    }

    let reader = BufReader::new(File::open(&backend.corpus_jsonl_path)?);
    let mut found = BTreeMap::<String, OalcJsonLine>::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: OalcJsonLine = serde_json::from_str(&line)
            .map_err(|err| format!("invalid OALC JSONL line {}: {err}", index + 1))?;
        if !TARGETS.contains(&record.citation.as_str()) {
            continue;
        }
        if found.insert(record.citation.clone(), record).is_some() {
            return Err("duplicate exact OALC citation in selected corpus revision".into());
        }
    }

    for target in TARGETS {
        if !found.contains_key(target) {
            return Err(format!("offline OALC backend missing exact citation: {target}").into());
        }
    }

    fs::create_dir_all(&output_dir)?;
    let receipt_path = output_dir.join("oalc_legislation_receipts.tsv");
    let mut receipt = String::from(
        "citation\tversion_id\tcorpus_revision\tsource\tjurisdiction\ttype\tdate\turl\twhen_scraped\tcanonical_text_digest\tlocal_artifact_ref\ttemporal_status\tnetwork_requests\treceipt_authority\n",
    );

    for target in TARGETS {
        let record = found.remove(target).expect("target checked above");
        if record.text.trim().is_empty() {
            return Err(format!("OALC target had empty text: {target}").into());
        }
        let artifact = output_dir.join(format!("{}.txt", slug(&record.citation)));
        readonly_write(&artifact, &record.text)?;
        let digest = sha256(record.text.as_bytes());
        let source_demand = demand(target);
        let document_receipt = OalcResolvedDocumentReceipt {
            demand_ref: source_demand.demand_ref.clone(),
            citation: record.citation.clone(),
            version_id: record.version_id.clone(),
            corpus_revision_ref: corpus_revision.clone(),
            source: record.source.clone(),
            jurisdiction: record.jurisdiction.clone(),
            document_type: record.document_type.clone(),
            canonical_text_digest: digest.clone(),
            local_artifact_ref: artifact.clone(),
            temporal_coverage: OalcTemporalCoverage::LatestKnownOnly,
            network_requests: 0,
            receipt_authority: OALC_RECEIPT_AUTHORITY,
        };
        document_receipt
            .validate_against(&dataset, &source_demand)
            .map_err(|err| format!("invalid offline OALC receipt for {target}: {err:?}"))?;

        let row = [
            record.citation,
            record.version_id,
            corpus_revision.clone(),
            record.source,
            record.jurisdiction,
            record.document_type,
            record.date.unwrap_or_default(),
            record.url.unwrap_or_default(),
            record.when_scraped.unwrap_or_default(),
            digest,
            artifact.to_string_lossy().into_owned(),
            "latest_known_only".to_string(),
            "0".to_string(),
            OALC_RECEIPT_AUTHORITY.to_string(),
        ]
        .iter()
        .map(|value| tsv(value))
        .collect::<Vec<_>>()
        .join("\t");
        receipt.push_str(&row);
        receipt.push('\n');
    }

    fs::write(&receipt_path, receipt)?;
    println!(
        "offline OALC backend materialized 2 NSW legislation documents; temporal_status=latest_known_only; network_requests=0; receipts={}",
        receipt_path.display()
    );
    Ok(())
}
