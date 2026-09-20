use sensiblaw_governed_legal_provider::OalcResolvedSourceReceipt;
use sensiblaw_proof_search_loop::oalc_judgment_materialization::{
    exact_mnc_candidate_follow_demand, materialize_oalc_judgment, ParagraphResearchCriterion,
};
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

fn criteria() -> Vec<ParagraphResearchCriterion> {
    vec![
        ParagraphResearchCriterion {
            criterion_ref: "requirement:estoppel:assumption".into(),
            needles: vec![
                "assumption".into(),
                "expectation".into(),
                "representation".into(),
            ],
        },
        ParagraphResearchCriterion {
            criterion_ref: "requirement:estoppel:reliance".into(),
            needles: vec!["reliance".into(), "relied".into(), "acted".into()],
        },
        ParagraphResearchCriterion {
            criterion_ref: "requirement:estoppel:detriment".into(),
            needles: vec!["detriment".into(), "detrimental".into()],
        },
        ParagraphResearchCriterion {
            criterion_ref: "requirement:estoppel:unconscionability".into(),
            needles: vec!["unconscionable".into(), "unconscionability".into()],
        },
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = PathBuf::from(
        env::var("SENSIBLAW_OALC_OUTPUT")
            .unwrap_or_else(|_| "artifacts/oalc/contracts/waltons".into()),
    );
    let receipt_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("waltons-oalc-source-receipt.json"));
    let output_path = env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("waltons-oalc-candidate-review-queue.json"));

    let receipt: OalcResolvedSourceReceipt =
        serde_json::from_slice(&fs::read(&receipt_path)?)?;
    let canonical_text = fs::read_to_string(&receipt.local_artifact_ref)?;
    let materialized = materialize_oalc_judgment(&receipt, &canonical_text, &criteria())
        .map_err(|error| format!("Waltons OALC materialization failed: {error:?}"))?;

    let matched_paragraphs = materialized
        .paragraph_candidates
        .iter()
        .filter(|paragraph| !paragraph.matched_research_criterion_refs.is_empty())
        .map(|paragraph| {
            json!({
                "paragraph_locator_ref": paragraph.paragraph_locator_ref.clone(),
                "reported_paragraph_label": paragraph.reported_paragraph_label.clone(),
                "matched_research_criterion_refs": paragraph.matched_research_criterion_refs.clone(),
                "candidate_only": paragraph.candidate_only,
                "creates_legal_authority": paragraph.creates_legal_authority,
                "creates_claim_truth": paragraph.creates_claim_truth,
            })
        })
        .collect::<Vec<_>>();

    let citation_candidates = materialized
        .citation_candidates
        .iter()
        .map(|candidate| {
            let follow = exact_mnc_candidate_follow_demand(candidate, "AU");
            json!({
                "paragraph_locator_ref": candidate.paragraph_locator_ref.clone(),
                "reported_paragraph_label": candidate.reported_paragraph_label.clone(),
                "citation_text": candidate.citation_text.clone(),
                "lexical_treatment_hints": format!("{:?}", candidate.lexical_treatment_hints),
                "candidate_only": candidate.candidate_only,
                "reviewed": candidate.reviewed,
                "exact_mnc_follow_scheduled": follow.is_some(),
                "treatment_classified": false,
            })
        })
        .collect::<Vec<_>>();

    let output = json!({
        "schema_version": "sl.oalc_judgment_materialization.v0_1",
        "document_ref": materialized.document_ref,
        "source_revision_ref": materialized.source_revision_ref,
        "canonical_text_sha256": materialized.canonical_text_sha256,
        "paragraph_count": materialized.paragraph_candidates.len(),
        "matched_research_paragraph_count": matched_paragraphs.len(),
        "citation_candidate_count": materialized.citation_candidates.len(),
        "candidate_only": materialized.candidate_only,
        "creates_legal_authority": materialized.creates_legal_authority,
        "creates_claim_truth": materialized.creates_claim_truth,
        "research_match_claims_estoppel_payment": false,
        "citation_extraction_claims_treatment": false,
        "matched_paragraphs": matched_paragraphs,
        "citation_candidates": citation_candidates,
    });

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&output)?)?;
    println!(
        "waltons_oalc_materialization={} paragraphs={} matched={} citations={} authority=experimental_candidate_only",
        output_path.display(),
        materialized.paragraph_candidates.len(),
        output["matched_research_paragraph_count"],
        materialized.citation_candidates.len(),
    );
    Ok(())
}
