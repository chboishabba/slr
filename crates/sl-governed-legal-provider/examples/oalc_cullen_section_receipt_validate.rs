#[path = "../src/oalc_legislation_contract.rs"]
mod oalc_legislation_contract;

use oalc_legislation_contract::{
    OalcSectionSliceReceipt, OalcTemporalCoverage, CULLEN_CLA_CITATION,
    CULLEN_VICARIOUS_CITATION, OALC_PARSER_AUTHORITY,
};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(
        env::var("SENSIBLAW_OALC_SECTION_RECEIPTS")
            .unwrap_or_else(|_| "artifacts/oalc/cullen-governing-law/oalc_section_pnf_receipts.tsv".into()),
    );
    let input = fs::read_to_string(&path)?;
    let mut rows = input.lines();
    let header = rows.next().ok_or("empty OALC section receipt TSV")?;
    let columns = header.split('\t').collect::<Vec<_>>();
    let index = |name: &str| {
        columns
            .iter()
            .position(|column| *column == name)
            .ok_or_else(|| format!("missing TSV column: {name}"))
    };

    let citation_i = index("citation")?;
    let section_i = index("section")?;
    let version_i = index("parent_version_id")?;
    let revision_i = index("corpus_revision")?;
    let parent_digest_i = index("parent_digest")?;
    let start_i = index("slice_start")?;
    let end_i = index("slice_end")?;
    let slice_digest_i = index("slice_digest")?;
    let artifact_i = index("slice_artifact")?;
    let temporal_i = index("temporal_status")?;
    let parser_authority_i = index("parser_authority")?;

    let expected = BTreeSet::from([
        (CULLEN_CLA_CITATION.to_string(), "5A".to_string()),
        (CULLEN_CLA_CITATION.to_string(), "5B".to_string()),
        (CULLEN_CLA_CITATION.to_string(), "5C".to_string()),
        (CULLEN_CLA_CITATION.to_string(), "5D".to_string()),
        (CULLEN_CLA_CITATION.to_string(), "43A".to_string()),
        (CULLEN_VICARIOUS_CITATION.to_string(), "6".to_string()),
        (CULLEN_VICARIOUS_CITATION.to_string(), "7".to_string()),
        (CULLEN_VICARIOUS_CITATION.to_string(), "8".to_string()),
    ]);
    let mut seen = BTreeSet::new();

    for (line_no, line) in rows.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != columns.len() {
            return Err(format!(
                "{}:{} field count mismatch: expected {}, got {}",
                path.display(),
                line_no + 2,
                columns.len(),
                fields.len()
            )
            .into());
        }
        if fields[temporal_i] != "latest_known_only" {
            return Err(format!("unexpected temporal status at line {}", line_no + 2).into());
        }
        if fields[parser_authority_i] != OALC_PARSER_AUTHORITY {
            return Err(format!("unexpected parser authority at line {}", line_no + 2).into());
        }

        let receipt = OalcSectionSliceReceipt {
            citation: fields[citation_i].to_string(),
            section: fields[section_i].to_string(),
            parent_version_id: fields[version_i].to_string(),
            corpus_revision_ref: fields[revision_i].to_string(),
            parent_digest: fields[parent_digest_i].to_string(),
            slice_start: fields[start_i].parse()?,
            slice_end: fields[end_i].parse()?,
            slice_digest: fields[slice_digest_i].to_string(),
            slice_artifact_ref: PathBuf::from(fields[artifact_i]),
            temporal_coverage: OalcTemporalCoverage::LatestKnownOnly,
            parser_authority: OALC_PARSER_AUTHORITY,
        };
        if !receipt.parser_eligible() {
            return Err(format!("non-parser-eligible section receipt at line {}", line_no + 2).into());
        }
        if receipt.pays_in_force_on("2017-01-26") {
            return Err("latest-known-only OALC section incorrectly paid historical date".into());
        }
        if receipt.creates_atomic_gate() {
            return Err("OALC section receipt incorrectly created Atomic gate".into());
        }
        seen.insert((receipt.citation, receipt.section));
    }

    if seen != expected {
        return Err(format!("Cullen OALC section set mismatch: expected {expected:?}, got {seen:?}").into());
    }

    println!(
        "validated {} Cullen OALC statutory section receipts; temporal_status=latest_known_only; parser_authority={}",
        seen.len(),
        OALC_PARSER_AUTHORITY
    );
    Ok(())
}
