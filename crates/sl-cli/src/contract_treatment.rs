//! Generic citation-treatment review gate for recursive LegalFollow campaigns.
//!
//! The queue is parameterised by source authority identity and target citation;
//! there is no Waltons, Sidhu, Giumelli or other case-specific runtime here.

use serde::{Deserialize, Serialize};
use sensiblaw_governed_legal_provider::OalcResolvedSourceReceipt;
use sensiblaw_legal_follow_plan::AustralianContractTrace;
use sensiblaw_proof_search_loop::contract_review_expansion::{
    compile_treatment_receipts_to_contract_hops_with_aliases, ContractReviewedHopCompilation,
};
use sensiblaw_proof_search_loop::judgment_candidates::CitationOccurrenceCandidate;
use sensiblaw_proof_search_loop::oalc_judgment_materialization::materialize_oalc_judgment;
use sensiblaw_proof_search_loop::reasoning::{
    CitationUse, ConditionCoordinate, ReasoningRole,
};
use sensiblaw_proof_search_loop::residual_review_shortlist::ResidualShortlistedCitation;
use sensiblaw_proof_search_loop::review_unit_review::{
    compile_reviewed_unit_receipt, ReviewedCitationReviewUnitDecision,
    ReviewedCitationReviewUnitReceipt,
};
use sensiblaw_proof_search_loop::review_units::{
    cluster_shortlisted_citations, CitationReviewUnit,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

type CliResult<T = ()> = Result<T, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactTreatmentQueue {
    pub schema_version: String,
    pub source_receipt_path: String,
    pub source_document_ref: String,
    pub source_semantic_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_sha256: String,
    pub target_medium_neutral_citation: String,
    pub target_semantic_ref: String,
    pub review_unit_count: usize,
    pub candidate_only: bool,
    pub treatment_classified: bool,
    pub review_units: Vec<CitationReviewUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatmentReviewWorksheet {
    pub schema_version: String,
    pub source_queue: String,
    pub source_semantic_ref: String,
    pub target_semantic_ref: String,
    pub target_medium_neutral_citation: String,
    pub candidate_only: bool,
    #[serde(default)]
    pub review_complete: bool,
    pub rows: Vec<TreatmentReviewWorksheetRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatmentReviewWorksheetRow {
    pub include: bool,
    pub review_unit_ref: String,
    pub document_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_sha256: String,
    pub citation_text: String,
    pub citation_locator_refs: Vec<String>,
    pub anchor_paragraph_locator_refs: Vec<String>,
    pub anchor_paragraph_texts: Vec<String>,
    pub matched_criterion_refs: Vec<String>,
    pub selected_anchor_paragraph_locator_ref: Option<String>,
    pub citing_proposition_ref: String,
    pub cited_document_ref: String,
    pub cited_proposition_ref: String,
    pub citation_use: Option<CitationUse>,
    pub reasoning_role: Option<ReasoningRole>,
    pub condition_coordinates: Vec<ConditionCoordinate>,
    pub judge_or_speaker_ref: Option<String>,
    pub court_ref: Option<String>,
    pub jurisdiction_ref: Option<String>,
    pub temporal_ref: Option<String>,
    pub outcome_ref: Option<String>,
    pub remedy_ref: Option<String>,
    pub burden_refs: Vec<String>,
    pub exception_refs: Vec<String>,
    pub lexical_realisation: String,
    pub reviewer_ref: String,
    pub evidence_refs: Vec<String>,
    pub review_notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatmentDecisionFile {
    pub schema_version: String,
    pub decisions: Vec<ReviewedCitationReviewUnitDecision>,
}

pub struct CompiledTreatmentReview {
    pub receipts: Vec<ReviewedCitationReviewUnitReceipt>,
    pub compilation: ContractReviewedHopCompilation,
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> CliResult<T> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode {}: {error}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> CliResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(value)
            .map_err(|error| format!("encode {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("write {}: {error}", path.display()))
}

fn exact_candidate(
    candidate: &CitationOccurrenceCandidate,
    target_mnc: &str,
) -> Option<ResidualShortlistedCitation> {
    (candidate.citation_text.trim() == target_mnc).then(|| ResidualShortlistedCitation {
        candidate: candidate.clone(),
        matched_criterion_refs: vec![format!(
            "criterion:treatment:exact-citation:{}",
            target_mnc
        )],
    })
}

pub fn prepare_exact_treatment_queue(
    source_receipt_path: &Path,
    source_semantic_ref: &str,
    target_mnc: &str,
    target_semantic_ref: &str,
    output: &Path,
) -> CliResult<ExactTreatmentQueue> {
    if source_semantic_ref.trim().is_empty()
        || target_mnc.trim().is_empty()
        || target_semantic_ref.trim().is_empty()
    {
        return Err("generic treatment queue requires source identity, target MNC and target identity".into());
    }
    let receipt: OalcResolvedSourceReceipt = read_json(source_receipt_path)?;
    let text = fs::read_to_string(&receipt.local_artifact_ref).map_err(|error| {
        format!(
            "read retained source {}: {error}",
            receipt.local_artifact_ref.display()
        )
    })?;
    let materialized = materialize_oalc_judgment(&receipt, &text, &[])
        .map_err(|error| format!("materialize {}: {error:?}", receipt.citation))?;
    let shortlisted = materialized
        .citation_candidates
        .iter()
        .filter_map(|candidate| exact_candidate(candidate, target_mnc))
        .collect::<Vec<_>>();
    let units = cluster_shortlisted_citations(&shortlisted);
    if units.is_empty() {
        return Err(format!(
            "retained source {} contains no exact citation occurrence for {}",
            receipt.citation, target_mnc
        ));
    }
    let queue = ExactTreatmentQueue {
        schema_version: "sl.contract_follow.exact_treatment_queue.v0_1".into(),
        source_receipt_path: source_receipt_path.display().to_string(),
        source_document_ref: materialized.document_ref,
        source_semantic_ref: source_semantic_ref.into(),
        source_revision_ref: materialized.source_revision_ref,
        canonical_text_sha256: materialized.canonical_text_sha256,
        target_medium_neutral_citation: target_mnc.into(),
        target_semantic_ref: target_semantic_ref.into(),
        review_unit_count: units.len(),
        candidate_only: true,
        treatment_classified: false,
        review_units: units,
    };
    write_json(output, &queue)?;
    Ok(queue)
}

pub fn prepare_treatment_worksheet(
    queue_path: &Path,
    output: &Path,
) -> CliResult<TreatmentReviewWorksheet> {
    let queue: ExactTreatmentQueue = read_json(queue_path)?;
    if queue.schema_version != "sl.contract_follow.exact_treatment_queue.v0_1" {
        return Err(format!("unsupported treatment queue schema {}", queue.schema_version));
    }
    let rows = queue
        .review_units
        .iter()
        .map(|unit| TreatmentReviewWorksheetRow {
            include: false,
            review_unit_ref: unit.review_unit_ref.clone(),
            document_ref: unit.document_ref.clone(),
            source_revision_ref: unit.source_revision_ref.clone(),
            canonical_text_sha256: unit.canonical_text_sha256.clone(),
            citation_text: unit.citation_text.clone(),
            citation_locator_refs: unit.citation_locator_refs.clone(),
            anchor_paragraph_locator_refs: unit.anchor_paragraph_locator_refs.clone(),
            anchor_paragraph_texts: unit.anchor_paragraph_texts.clone(),
            matched_criterion_refs: unit.matched_criterion_refs.clone(),
            selected_anchor_paragraph_locator_ref: None,
            citing_proposition_ref: String::new(),
            cited_document_ref: queue.target_semantic_ref.clone(),
            cited_proposition_ref: String::new(),
            citation_use: None,
            reasoning_role: None,
            condition_coordinates: Vec::new(),
            judge_or_speaker_ref: None,
            court_ref: None,
            jurisdiction_ref: None,
            temporal_ref: None,
            outcome_ref: None,
            remedy_ref: None,
            burden_refs: Vec::new(),
            exception_refs: Vec::new(),
            lexical_realisation: String::new(),
            reviewer_ref: String::new(),
            evidence_refs: Vec::new(),
            review_notes: String::new(),
        })
        .collect();
    let worksheet = TreatmentReviewWorksheet {
        schema_version: "sl.contract_follow.treatment_review_worksheet.v0_1".into(),
        source_queue: queue_path.display().to_string(),
        source_semantic_ref: queue.source_semantic_ref,
        target_semantic_ref: queue.target_semantic_ref,
        target_medium_neutral_citation: queue.target_medium_neutral_citation,
        candidate_only: true,
        review_complete: false,
        rows,
    };
    write_json(output, &worksheet)?;
    Ok(worksheet)
}

pub fn finalize_treatment_review(
    worksheet_path: &Path,
    output: &Path,
) -> CliResult<TreatmentDecisionFile> {
    let worksheet: TreatmentReviewWorksheet = read_json(worksheet_path)?;
    if worksheet.schema_version != "sl.contract_follow.treatment_review_worksheet.v0_1" {
        return Err(format!(
            "unsupported treatment worksheet schema {}",
            worksheet.schema_version
        ));
    }
    if !worksheet.review_complete {
        return Err("generic treatment review worksheet is not marked review_complete=true".into());
    }
    let mut decisions = Vec::new();
    for (index, row) in worksheet.rows.into_iter().enumerate() {
        if !row.include {
            continue;
        }
        let selected_anchor = row
            .selected_anchor_paragraph_locator_ref
            .ok_or_else(|| format!("treatment row {} missing selected anchor", index + 1))?;
        if !row.anchor_paragraph_locator_refs.contains(&selected_anchor) {
            return Err(format!(
                "treatment row {} selected anchor is not owned by review unit",
                index + 1
            ));
        }
        let citation_use = row
            .citation_use
            .ok_or_else(|| format!("treatment row {} missing citation_use", index + 1))?;
        let reasoning_role = row
            .reasoning_role
            .ok_or_else(|| format!("treatment row {} missing reasoning_role", index + 1))?;
        for (label, value) in [
            ("citing_proposition_ref", row.citing_proposition_ref.as_str()),
            ("cited_document_ref", row.cited_document_ref.as_str()),
            ("cited_proposition_ref", row.cited_proposition_ref.as_str()),
            ("lexical_realisation", row.lexical_realisation.as_str()),
            ("reviewer_ref", row.reviewer_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("treatment row {} missing {label}", index + 1));
            }
        }
        if row.evidence_refs.is_empty()
            || row.evidence_refs.iter().any(|value| value.trim().is_empty())
        {
            return Err(format!("treatment row {} requires evidence_refs", index + 1));
        }
        decisions.push(ReviewedCitationReviewUnitDecision {
            review_unit_ref: row.review_unit_ref,
            source_revision_ref: row.source_revision_ref,
            citation_text: row.citation_text,
            selected_anchor_paragraph_locator_ref: selected_anchor,
            citing_proposition_ref: row.citing_proposition_ref,
            cited_document_ref: row.cited_document_ref,
            cited_proposition_ref: row.cited_proposition_ref,
            citation_use,
            reasoning_role,
            condition_coordinates: row.condition_coordinates,
            judge_or_speaker_ref: row.judge_or_speaker_ref,
            court_ref: row.court_ref,
            jurisdiction_ref: row.jurisdiction_ref,
            temporal_ref: row.temporal_ref,
            outcome_ref: row.outcome_ref,
            remedy_ref: row.remedy_ref,
            burden_refs: row.burden_refs,
            exception_refs: row.exception_refs,
            lexical_realisation: row.lexical_realisation,
            reviewer_ref: row.reviewer_ref,
            evidence_refs: row.evidence_refs,
        });
    }
    let file = TreatmentDecisionFile {
        schema_version: "sl.contract_follow.treatment_review_decisions.v0_1".into(),
        decisions,
    };
    write_json(output, &file)?;
    Ok(file)
}

pub fn compile_treatment_review(
    queue_path: &Path,
    decisions_path: &Path,
    trace: &AustralianContractTrace,
) -> CliResult<CompiledTreatmentReview> {
    let queue: ExactTreatmentQueue = read_json(queue_path)?;
    let decisions: TreatmentDecisionFile = read_json(decisions_path)?;
    if decisions.schema_version != "sl.contract_follow.treatment_review_decisions.v0_1" {
        return Err(format!(
            "unsupported treatment decision schema {}",
            decisions.schema_version
        ));
    }
    let units = queue
        .review_units
        .iter()
        .map(|unit| (unit.review_unit_ref.clone(), unit))
        .collect::<BTreeMap<_, _>>();
    let receipts = decisions
        .decisions
        .iter()
        .map(|decision| {
            let unit = units
                .get(&decision.review_unit_ref)
                .ok_or_else(|| format!("unknown review unit {}", decision.review_unit_ref))?;
            compile_reviewed_unit_receipt(unit, decision)
                .map_err(|error| format!("compile generic treatment review: {error:?}"))
        })
        .collect::<CliResult<Vec<_>>>()?;

    let aliases = BTreeMap::from([(
        queue.source_document_ref.clone(),
        queue.source_semantic_ref.clone(),
    )]);
    let compilation =
        compile_treatment_receipts_to_contract_hops_with_aliases(trace, &receipts, &aliases);
    Ok(CompiledTreatmentReview {
        receipts,
        compilation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_treatment_types_contain_no_case_specific_identity() {
        let text = format!(
            "{} {}",
            std::any::type_name::<ExactTreatmentQueue>(),
            std::any::type_name::<TreatmentReviewWorksheet>()
        );
        assert!(!text.to_ascii_lowercase().contains("waltons"));
        assert!(!text.to_ascii_lowercase().contains("sidhu"));
        assert!(!text.to_ascii_lowercase().contains("giumelli"));
    }
}
