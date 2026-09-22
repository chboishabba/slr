use sensiblaw_proof_search_loop::review_unit_review::{
    compile_reviewed_unit_receipt, ReviewedCitationReviewUnitDecision,
};
use sensiblaw_proof_search_loop::review_units::CitationReviewUnit;
use sensiblaw_proof_search_loop::treatment_genealogy::build_temporal_treatment_genealogy;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct TreatmentQueue {
    schema_version: String,
    review_units: Vec<CitationReviewUnit>,
}

#[derive(Debug, Deserialize)]
struct TreatmentDecisionFile {
    schema_version: String,
    decisions: Vec<ReviewedCitationReviewUnitDecision>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queue_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: citation_treatment_review_compile <queue.json> <decisions.json> <root-authority-ref> <as-at> [output.json]")?;
    let decisions_path = env::args()
        .nth(2)
        .map(PathBuf::from)
        .ok_or("review decisions path is required")?;
    let root_authority_ref = env::args()
        .nth(3)
        .ok_or("root authority ref is required")?;
    let as_at = env::args().nth(4).ok_or("as-at is required")?;
    let output_path = env::args()
        .nth(5)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("treatment-genealogy.json"));

    let queue: TreatmentQueue = serde_json::from_slice(&fs::read(&queue_path)?)?;
    if queue.schema_version != "sl.authority_treatment_review_queue.v0_1" {
        return Err(format!("unsupported treatment queue schema {}", queue.schema_version).into());
    }
    let decisions: TreatmentDecisionFile =
        serde_json::from_slice(&fs::read(&decisions_path)?)?;
    if decisions.schema_version != "sl.citation_treatment_review_decisions.v0_1" {
        return Err(format!(
            "unsupported treatment decision schema {}",
            decisions.schema_version
        )
        .into());
    }

    let units = queue
        .review_units
        .iter()
        .map(|unit| (unit.review_unit_ref.clone(), unit))
        .collect::<BTreeMap<_, _>>();

    let mut receipts = Vec::new();
    for decision in &decisions.decisions {
        let unit = units
            .get(&decision.review_unit_ref)
            .ok_or_else(|| format!("decision names unknown review unit {}", decision.review_unit_ref))?;
        receipts.push(
            compile_reviewed_unit_receipt(unit, decision)
                .map_err(|error| format!("review {} failed: {error:?}", decision.review_unit_ref))?,
        );
    }

    let genealogy = build_temporal_treatment_genealogy(
        &root_authority_ref,
        &as_at,
        &receipts,
    )
    .map_err(|error| format!("genealogy build failed: {error:?}"))?;

    let receipt_summaries = receipts
        .iter()
        .map(|receipt| {
            json!({
                "review_unit_ref": receipt.review_unit_ref.clone(),
                "document_ref": receipt.document_ref.clone(),
                "source_revision_ref": receipt.source_revision_ref.clone(),
                "canonical_text_sha256": receipt.canonical_text_sha256.clone(),
                "citation_text": receipt.citation_text.clone(),
                "selected_anchor_paragraph_locator_ref": receipt.selected_anchor_paragraph_locator_ref.clone(),
                "reviewer_ref": receipt.reviewer_ref.clone(),
                "evidence_refs": receipt.evidence_refs.clone(),
                "citation_use": format!("{:?}", receipt.edge.citation_use),
                "reasoning_role": format!("{:?}", receipt.edge.reasoning_role),
                "temporal_ref": receipt.edge.temporal_ref.clone(),
                "candidate_only": receipt.edge.candidate_only,
                "reviewed": receipt.edge.reviewed,
            })
        })
        .collect::<Vec<_>>();

    let output = json!({
        "schema_version": "sl.temporal_treatment_genealogy.v0_1",
        "queue_path": queue_path,
        "decision_path": decisions_path,
        "reviewed_receipt_count": receipts.len(),
        "candidate_only": true,
        "creates_legal_authority": false,
        "current_law_conclusion": false,
        "receipts": receipt_summaries,
        "genealogy": genealogy,
    });

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&output)?)?;
    println!(
        "treatment_genealogy={} reviewed={} root={} as_at={} authority=experimental_candidate_only",
        output_path.display(),
        receipts.len(),
        root_authority_ref,
        as_at,
    );
    Ok(())
}
