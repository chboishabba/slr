use sensiblaw_governed_legal_provider::OalcResolvedSourceReceipt;
use sensiblaw_proof_search_loop::judgment_candidates::CitationOccurrenceCandidate;
use sensiblaw_proof_search_loop::oalc_judgment_materialization::materialize_oalc_judgment;
use sensiblaw_proof_search_loop::residual_review_shortlist::ResidualShortlistedCitation;
use sensiblaw_proof_search_loop::review_units::cluster_shortlisted_citations;
use serde::Serialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct TreatmentQueue<'a> {
    schema_version: &'static str,
    root_authority_citation: &'a str,
    source_document_ref: &'a str,
    source_revision_ref: &'a str,
    canonical_text_sha256: &'a str,
    review_unit_count: usize,
    candidate_only: bool,
    treatment_classified: bool,
    review_units: Vec<sensiblaw_proof_search_loop::review_units::CitationReviewUnit>,
}

fn shortlist(
    candidate: &CitationOccurrenceCandidate,
    target_citation: &str,
) -> Option<ResidualShortlistedCitation> {
    (candidate.citation_text.trim() == target_citation.trim()).then(|| {
        ResidualShortlistedCitation {
            candidate: candidate.clone(),
            matched_criterion_refs: vec![format!(
                "criterion:treatment:exact-citation:{}",
                target_citation.replace(' ', "-")
            )],
        }
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let receipt_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: oalc_authority_treatment_review_queue <oalc-source-receipt.json> <target-mnc> [output.json]")?;
    let target_citation = env::args()
        .nth(2)
        .ok_or("target medium-neutral citation is required")?;
    let output_path = env::args()
        .nth(3)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("treatment-review-queue.json"));

    let receipt: OalcResolvedSourceReceipt =
        serde_json::from_slice(&fs::read(&receipt_path)?)?;
    let canonical_text = fs::read_to_string(&receipt.local_artifact_ref)?;
    let materialized = materialize_oalc_judgment(&receipt, &canonical_text, &[])
        .map_err(|error| format!("materialize later authority: {error:?}"))?;

    let shortlisted = materialized
        .citation_candidates
        .iter()
        .filter_map(|candidate| shortlist(candidate, &target_citation))
        .collect::<Vec<_>>();
    let review_units = cluster_shortlisted_citations(&shortlisted);

    let queue = TreatmentQueue {
        schema_version: "sl.authority_treatment_review_queue.v0_1",
        root_authority_citation: &target_citation,
        source_document_ref: &materialized.document_ref,
        source_revision_ref: &materialized.source_revision_ref,
        canonical_text_sha256: &materialized.canonical_text_sha256,
        review_unit_count: review_units.len(),
        candidate_only: true,
        treatment_classified: false,
        review_units,
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&queue)?)?;
    println!(
        "treatment_review_queue={} units={} root={} authority=experimental_candidate_only",
        output_path.display(),
        queue.review_unit_count,
        target_citation,
    );
    Ok(())
}
