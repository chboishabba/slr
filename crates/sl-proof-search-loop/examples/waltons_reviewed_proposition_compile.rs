use sensiblaw_governed_legal_provider::OalcResolvedSourceReceipt;
use sensiblaw_proof_search_loop::oalc_judgment_materialization::{
    materialize_oalc_judgment, ParagraphResearchCriterion,
};
use sensiblaw_proof_search_loop::waltons_proposition_review::{
    compile_reviewed_waltons_paragraph, EstoppelRequirementRole,
    PropositionEvidenceDisposition, ReviewedWaltonsParagraphDecision,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReviewDecisionFile {
    schema_version: String,
    decisions: Vec<ReviewedWaltonsParagraphDecision>,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewReceiptSummary {
    role: EstoppelRequirementRole,
    proposition_ref: String,
    paragraph_locator_ref: String,
    source_revision_ref: String,
    canonical_text_sha256: String,
    disposition: PropositionEvidenceDisposition,
    reviewer_ref: String,
    review_evidence_refs: Vec<String>,
    payments_emitted: u64,
    candidate_only: bool,
    creates_legal_authority: bool,
    applicability_promoted: bool,
    claim_truth_promoted: bool,
}

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

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
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
    let decisions_path = env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("waltons-reviewed-decisions.json"));
    let output_path = env::args()
        .nth(3)
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("waltons-reviewed-proposition-receipts.json"));
    let payment_path = env::args()
        .nth(4)
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("waltons-reviewed-evidence-payments.slrw"));

    let receipt: OalcResolvedSourceReceipt =
        serde_json::from_slice(&fs::read(&receipt_path)?)?;
    let canonical_text = fs::read_to_string(&receipt.local_artifact_ref)?;
    let materialization = materialize_oalc_judgment(&receipt, &canonical_text, &criteria())
        .map_err(|error| format!("Waltons materialization failed: {error:?}"))?;

    let decisions: ReviewDecisionFile =
        serde_json::from_slice(&fs::read(&decisions_path)?)?;
    if decisions.schema_version != "sl.waltons.review_decisions.v0_1" {
        return Err(format!(
            "unsupported review decision schema {}",
            decisions.schema_version
        )
        .into());
    }

    let mut summaries = Vec::new();
    let mut payment_bytes = Vec::new();
    for (index, decision) in decisions.decisions.iter().enumerate() {
        let compiled = compile_reviewed_waltons_paragraph(
            &materialization,
            decision,
            index as i64 + 1,
        )
        .map_err(|error| format!("review decision {} failed: {error:?}", index + 1))?;
        payment_bytes.extend_from_slice(&compiled.payment_bytes);
        summaries.push(ReviewReceiptSummary {
            role: compiled.role,
            proposition_ref: compiled.proposition_ref,
            paragraph_locator_ref: compiled.paragraph_locator_ref,
            source_revision_ref: compiled.source_revision_ref,
            canonical_text_sha256: compiled.canonical_text_sha256,
            disposition: compiled.disposition,
            reviewer_ref: compiled.reviewer_ref,
            review_evidence_refs: compiled.review_evidence_refs,
            payments_emitted: compiled
                .payment_receipt
                .as_ref()
                .map_or(0, |receipt| receipt.payments_emitted),
            candidate_only: compiled.candidate_only,
            creates_legal_authority: compiled.creates_legal_authority,
            applicability_promoted: compiled.applicability_promoted,
            claim_truth_promoted: compiled.claim_truth_promoted,
        });
    }

    let output = serde_json::json!({
        "schema_version": "sl.waltons.reviewed_proposition_receipts.v0_1",
        "source_receipt_sha256": sha256(&fs::read(&receipt_path)?),
        "decision_file_sha256": sha256(&fs::read(&decisions_path)?),
        "source_revision_ref": materialization.source_revision_ref,
        "canonical_text_sha256": materialization.canonical_text_sha256,
        "reviewed_receipt_count": summaries.len(),
        "candidate_only": true,
        "creates_legal_authority": false,
        "applicability_promoted": false,
        "claim_truth_promoted": false,
        "receipts": summaries,
    });

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&output)?)?;
    fs::write(&payment_path, payment_bytes)?;

    println!(
        "waltons_reviewed_receipts={} count={} payments={} authority=experimental_candidate_only",
        output_path.display(),
        output["reviewed_receipt_count"],
        payment_path.display(),
    );
    Ok(())
}
