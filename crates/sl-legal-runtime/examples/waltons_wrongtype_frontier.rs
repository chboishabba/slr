use sensiblaw_governed_legal_provider::OalcResolvedSourceReceipt;
use sensiblaw_legal_runtime::project_waltons_reviewed_receipts_to_issue;
use sensiblaw_proof_search_loop::oalc_judgment_materialization::{
    materialize_oalc_judgment, ParagraphResearchCriterion,
};
use sensiblaw_proof_search_loop::waltons_proposition_review::{
    compile_reviewed_waltons_paragraph, ReviewedWaltonsParagraphDecision,
};
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct ReviewDecisionFile {
    schema_version: String,
    decisions: Vec<ReviewedWaltonsParagraphDecision>,
}

fn criteria() -> Vec<ParagraphResearchCriterion> {
    vec![
        ParagraphResearchCriterion {
            criterion_ref: "requirement:estoppel:assumption".into(),
            needles: vec!["assumption".into(), "expectation".into(), "representation".into()],
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
    let decisions_path = env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("waltons-reviewed-decisions.json"));
    let output_path = env::args()
        .nth(3)
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("waltons-wrongtype-frontier.json"));

    let source_receipt: OalcResolvedSourceReceipt =
        serde_json::from_slice(&fs::read(&receipt_path)?)?;
    let canonical_text = fs::read_to_string(&source_receipt.local_artifact_ref)?;
    let materialization = materialize_oalc_judgment(&source_receipt, &canonical_text, &criteria())
        .map_err(|error| format!("Waltons materialization failed: {error:?}"))?;
    let decisions: ReviewDecisionFile =
        serde_json::from_slice(&fs::read(&decisions_path)?)?;
    if decisions.schema_version != "sl.waltons.review_decisions.v0_1" {
        return Err(format!("unsupported decision schema {}", decisions.schema_version).into());
    }

    let mut reviewed = Vec::new();
    for (index, decision) in decisions.decisions.iter().enumerate() {
        reviewed.push(
            compile_reviewed_waltons_paragraph(&materialization, decision, index as i64 + 1)
                .map_err(|error| format!("Waltons review {} failed: {error:?}", index + 1))?,
        );
    }
    let issue = project_waltons_reviewed_receipts_to_issue(&reviewed)
        .map_err(|error| format!("Waltons WrongType projection failed: {error:?}"))?;

    let elements = issue
        .elements
        .iter()
        .map(|element| {
            json!({
                "element_ref": element.element.element_ref.clone(),
                "proposition_ref": element.element.proposition_ref.clone(),
                "disposition": format!("{:?}", element.disposition),
                "reviewed_evidence_count": element.evidence.len(),
                "source_revision_refs": element.evidence.iter().map(|e| e.source_revision_ref.clone()).collect::<Vec<_>>(),
                "span_refs": element.evidence.iter().map(|e| e.span_ref.clone()).collect::<Vec<_>>(),
                "candidate_only": element.candidate_only,
                "creates_liability": element.creates_liability,
            })
        })
        .collect::<Vec<_>>();

    let output = json!({
        "schema_version": "sl.waltons.wrongtype_frontier.v0_1",
        "wrong_type_ref": issue.wrong_type_ref,
        "reviewed_receipt_count": reviewed.len(),
        "unresolved_or_contested_element_refs": issue.unresolved_element_refs(),
        "candidate_only": issue.candidate_only,
        "applicability_promoted": issue.applicability_promoted,
        "violation_promoted": issue.violation_promoted,
        "liability_promoted": issue.liability_promoted,
        "elements": elements,
    });
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&output)?)?;
    println!(
        "waltons_wrongtype_frontier={} reviewed={} open={}",
        output_path.display(),
        reviewed.len(),
        output["unresolved_or_contested_element_refs"]
            .as_array()
            .map_or(0, Vec::len)
    );
    Ok(())
}
