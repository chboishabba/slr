use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sensiblaw_governed_legal_provider::{
    citation_traversal_plan, run_live_oalc_case_follow, OalcCaseFollowRequest,
    OalcResolvedSourceReceipt,
};
use sensiblaw_legal_follow_plan::{
    apply_contract_landscape_expansion, waltons_estoppel_trace, AuthorityLevel,
    AustralianContractTrace, ContractDoctrine, SourceRole,
};
use sensiblaw_legal_runtime::project_waltons_reviewed_receipts_to_issue;
use sensiblaw_proof_search_loop::contract_review_expansion::{
    compile_reviewed_authority_identity_to_contract_hop,
    compile_treatment_receipts_to_contract_hops_with_aliases,
    compile_waltons_proposition_receipts_to_contract_hops,
    ContractReviewedHopCompilation, ReviewedContractAuthorityIdentity,
};
use sensiblaw_proof_search_loop::judgment_candidates::CitationOccurrenceCandidate;
use sensiblaw_proof_search_loop::oalc_judgment_materialization::{
    later_treatment_cited_by_demand, materialize_oalc_judgment,
    OalcJudgmentMaterialisation,
};
use sensiblaw_proof_search_loop::reasoning::{
    CitationUse, ConditionCoordinate, ReasoningRole,
};
use sensiblaw_proof_search_loop::residual_review_shortlist::ResidualShortlistedCitation;
use sensiblaw_proof_search_loop::review_unit_review::{
    compile_reviewed_unit_receipt, ReviewedCitationReviewUnitDecision,
};
use sensiblaw_proof_search_loop::review_units::{
    cluster_shortlisted_citations, CitationReviewUnit,
};
use sensiblaw_proof_search_loop::treatment_genealogy::build_temporal_treatment_genealogy;
use sensiblaw_proof_search_loop::waltons_proposition_review::{
    compile_reviewed_waltons_paragraph, waltons_estoppel_research_criteria,
    EstoppelRequirementRole, PropositionEvidenceDisposition,
    ReviewedWaltonsParagraphDecision, ReviewedWaltonsPropositionEvidenceReceipt,
};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const WALTONS_MNC: &str = "[1988] HCA 7";
pub const WALTONS_AUTHORITY_REF: &str = "case:au:hca:1988:7";
pub const DEFAULT_AS_AT: &str = "2026-09-20";

type CliResult<T = ()> = Result<T, String>;

#[derive(Debug, Clone)]
pub struct WaltonsPaths {
    pub base: PathBuf,
    pub receipt: PathBuf,
    pub text: PathBuf,
    pub queue: PathBuf,
    pub worksheet: PathBuf,
    pub decisions: PathBuf,
    pub reviewed: PathBuf,
    pub payments: PathBuf,
    pub frontier: PathBuf,
    pub proposition_hops: PathBuf,
    pub citedby_manifest: PathBuf,
    pub citedby_candidates: PathBuf,
    pub later_dir: PathBuf,
    pub identity_worksheet: PathBuf,
    pub identity_decisions: PathBuf,
    pub identity_hops: PathBuf,
    pub merged_treatment_queue: PathBuf,
    pub treatment_worksheet: PathBuf,
    pub treatment_decisions: PathBuf,
    pub genealogy: PathBuf,
    pub treatment_hops: PathBuf,
}

impl WaltonsPaths {
    pub fn from_base(base: PathBuf) -> Self {
        Self {
            receipt: base.join("waltons-oalc-source-receipt.json"),
            text: base.join("waltons-stores-v-maher.txt"),
            queue: base.join("waltons-oalc-candidate-review-queue.json"),
            worksheet: base.join("waltons-review-worksheet.json"),
            decisions: base.join("waltons-reviewed-decisions.json"),
            reviewed: base.join("waltons-reviewed-proposition-receipts.json"),
            payments: base.join("waltons-reviewed-evidence-payments.slrw"),
            frontier: base.join("waltons-wrongtype-frontier.json"),
            proposition_hops: base.join("waltons-reviewed-proposition-contract-hops.json"),
            citedby_manifest: base.join("waltons-cited-by-work-manifest.json"),
            citedby_candidates: base.join("waltons-cited-by-candidates.json"),
            later_dir: base.join("later-authorities"),
            identity_worksheet: base.join("waltons-authority-identity-review-worksheet.json"),
            identity_decisions: base.join("waltons-authority-identity-reviewed-decisions.json"),
            identity_hops: base.join("waltons-reviewed-authority-identity-contract-hops.json"),
            merged_treatment_queue: base.join("waltons-treatment-review-queue.json"),
            treatment_worksheet: base.join("waltons-treatment-review-worksheet.json"),
            treatment_decisions: base.join("waltons-treatment-reviewed-decisions.json"),
            genealogy: base.join("waltons-temporal-treatment-genealogy.json"),
            treatment_hops: base.join("waltons-reviewed-treatment-contract-hops.json"),
            base,
        }
    }

}

impl Default for WaltonsPaths {
    fn default() -> Self {
        let base = env::var("SENSIBLAW_OALC_OUTPUT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("artifacts/oalc/contracts/waltons"));
        Self::from_base(base)
    }
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
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("encode {}: {error}", path.display()))?;
    fs::write(path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
}

fn contract_hop_json(compiled: &ContractReviewedHopCompilation) -> Value {
    let deltas = compiled
        .deltas
        .iter()
        .map(|delta| {
            json!({
                "provenance_ref": delta.provenance_ref,
                "candidate_only": delta.candidate_only,
                "creates_legal_authority": delta.creates_legal_authority,
                "nodes": delta.discovered_nodes.iter().map(|node| json!({
                    "semantic_ref": node.semantic_ref,
                    "label": node.label,
                    "kind": format!("{:?}", node.kind),
                    "doctrine": node.doctrine.map(|value| format!("{value:?}")),
                    "jurisdiction_ref": node.jurisdiction_ref,
                    "court_ref": node.court_ref,
                    "decision_or_effective_date": node.decision_or_effective_date,
                    "valid_from": node.valid_from,
                    "valid_to": node.valid_to,
                    "source_role": format!("{:?}", node.source_role),
                    "authority_level": format!("{:?}", node.authority_level),
                    "source_citation": node.source_citation,
                })).collect::<Vec<_>>(),
                "edges": delta.discovered_edges.iter().map(|edge| json!({
                    "from_ref": edge.from_ref,
                    "to_ref": edge.to_ref,
                    "treatment": format!("{:?}", edge.treatment),
                    "candidate_only": edge.candidate_only,
                    "creates_legal_authority": edge.creates_legal_authority,
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": "sl.australian_contracts.reviewed_hops.v0_1",
        "delta_count": deltas.len(),
        "residual_count": compiled.residuals.len(),
        "candidate_only": compiled.candidate_only,
        "creates_legal_authority": compiled.creates_legal_authority,
        "creates_current_law_conclusion": compiled.creates_current_law_conclusion,
        "deltas": deltas,
        "residuals": compiled.residuals,
    })
}

fn load_waltons_materialisation(paths: &WaltonsPaths) -> CliResult<OalcJudgmentMaterialisation> {
    let receipt: OalcResolvedSourceReceipt = read_json(&paths.receipt)?;
    let text = fs::read_to_string(&receipt.local_artifact_ref).map_err(|error| {
        format!(
            "read retained Waltons text {}: {error}",
            receipt.local_artifact_ref.display()
        )
    })?;
    materialize_oalc_judgment(&receipt, &text, &waltons_estoppel_research_criteria())
        .map_err(|error| format!("materialize Waltons judgment: {error:?}"))
}

pub fn status(paths: &WaltonsPaths) {
    let rows = [
        ("1 source receipt", &paths.receipt),
        ("1 source text", &paths.text),
        ("1 candidate queue", &paths.queue),
        ("2 review worksheet", &paths.worksheet),
        ("2 finalized decisions", &paths.decisions),
        ("3 reviewed receipts", &paths.reviewed),
        ("3 payment wire", &paths.payments),
        ("4/5 WrongType frontier", &paths.frontier),
        ("5 proposition S14 hops", &paths.proposition_hops),
        ("6 cited-by manifest", &paths.citedby_manifest),
        ("6 cited-by candidates", &paths.citedby_candidates),
        ("6/7 identity worksheet", &paths.identity_worksheet),
        ("6/7 identity decisions", &paths.identity_decisions),
        ("6/7 identity S14 hops", &paths.identity_hops),
        ("7 merged treatment queue", &paths.merged_treatment_queue),
        ("7 treatment worksheet", &paths.treatment_worksheet),
        ("7 treatment decisions", &paths.treatment_decisions),
        ("8 genealogy", &paths.genealogy),
        ("8 treatment S14 hops", &paths.treatment_hops),
    ];
    for (label, path) in rows {
        println!(
            "{:2} {:30} {}",
            if path.exists() { "OK" } else { "--" },
            label,
            path.display()
        );
    }
}

pub fn acquire(paths: &WaltonsPaths) -> CliResult {
    fs::create_dir_all(&paths.base)
        .map_err(|error| format!("create {}: {error}", paths.base.display()))?;
    let mut request = OalcCaseFollowRequest::for_citation(WALTONS_MNC, paths.base.clone());
    request.court_ref = "court:HCA".into();
    request.oalc_jurisdiction = "commonwealth".into();
    request.legal_jurisdiction = "AU".into();
    request.as_at = DEFAULT_AS_AT.into();
    let run = run_live_oalc_case_follow(&request)
        .map_err(|error| format!("OALC Waltons acquisition: {error:?}"))?;

    let generic_receipt = run.source_receipt_path;
    let generic_text = run.canonical_text_path;
    if generic_receipt != paths.receipt {
        fs::rename(&generic_receipt, &paths.receipt).map_err(|error| {
            format!(
                "rename {} -> {}: {error}",
                generic_receipt.display(),
                paths.receipt.display()
            )
        })?;
    }
    if generic_text != paths.text {
        fs::rename(&generic_text, &paths.text).map_err(|error| {
            format!(
                "rename {} -> {}: {error}",
                generic_text.display(),
                paths.text.display()
            )
        })?;
    }

    let mut receipt: OalcResolvedSourceReceipt = read_json(&paths.receipt)?;
    receipt.local_artifact_ref = paths.text.clone();
    write_json(&paths.receipt, &receipt)?;
    println!(
        "waltons_acquired={} revision={} version={} path={} network={} authority=experimental_candidate_only",
        paths.receipt.display(),
        receipt.corpus_revision_ref,
        receipt.version_id,
        receipt.resolution_path,
        receipt.network_requests
    );
    Ok(())
}

fn hint_names(candidate: &CitationOccurrenceCandidate) -> Vec<String> {
    candidate
        .lexical_treatment_hints
        .iter()
        .map(|value| format!("{value:?}"))
        .collect()
}

pub fn materialise(paths: &WaltonsPaths) -> CliResult {
    let materialized = load_waltons_materialisation(paths)?;
    let matched_paragraphs = materialized
        .paragraph_candidates
        .iter()
        .filter(|paragraph| !paragraph.matched_research_criterion_refs.is_empty())
        .map(|paragraph| {
            json!({
                "paragraph_locator_ref": paragraph.paragraph_locator_ref,
                "reported_paragraph_label": paragraph.reported_paragraph_label,
                "matched_research_criterion_refs": paragraph.matched_research_criterion_refs,
                "paragraph_text": paragraph.paragraph_text,
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
            json!({
                "paragraph_locator_ref": candidate.paragraph_locator_ref,
                "reported_paragraph_label": candidate.reported_paragraph_label,
                "citation_text": candidate.citation_text,
                "paragraph_text": candidate.paragraph_text,
                "anchor_paragraph_locator_refs": candidate.anchor_paragraph_locator_refs,
                "anchor_paragraph_texts": candidate.anchor_paragraph_texts,
                "lexical_treatment_hints": hint_names(candidate),
                "candidate_only": candidate.candidate_only,
                "reviewed": candidate.reviewed,
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
        "candidate_only": true,
        "creates_legal_authority": false,
        "creates_claim_truth": false,
        "research_match_claims_estoppel_payment": false,
        "citation_extraction_claims_treatment": false,
        "matched_paragraphs": matched_paragraphs,
        "citation_candidates": citation_candidates,
    });
    write_json(&paths.queue, &output)?;
    println!(
        "waltons_review_queue={} matched={} citations={}",
        paths.queue.display(),
        output["matched_research_paragraph_count"],
        output["citation_candidate_count"]
    );
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WaltonsReviewWorksheet {
    schema_version: String,
    source_queue: String,
    candidate_only: bool,
    rows: Vec<WaltonsReviewWorksheetRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WaltonsReviewWorksheetRow {
    include: bool,
    paragraph_locator_ref: String,
    reported_paragraph_label: Option<String>,
    paragraph_text: String,
    source_revision_ref: String,
    canonical_text_sha256: String,
    requirement_ref: String,
    role: EstoppelRequirementRole,
    disposition: Option<PropositionEvidenceDisposition>,
    reviewer_ref: String,
    review_evidence_refs: Vec<String>,
    review_notes: String,
}

fn role_for_requirement(value: &str) -> Option<EstoppelRequirementRole> {
    match value {
        "requirement:estoppel:assumption" => Some(EstoppelRequirementRole::AssumptionOrExpectation),
        "requirement:estoppel:reliance" => Some(EstoppelRequirementRole::Reliance),
        "requirement:estoppel:detriment" => Some(EstoppelRequirementRole::Detriment),
        "requirement:estoppel:unconscionability" => {
            Some(EstoppelRequirementRole::Unconscionability)
        }
        _ => None,
    }
}

pub fn review_prepare(paths: &WaltonsPaths) -> CliResult {
    let queue: Value = read_json(&paths.queue)?;
    let source_revision_ref = queue["source_revision_ref"]
        .as_str()
        .ok_or_else(|| "Waltons queue missing source_revision_ref".to_string())?;
    let canonical_text_sha256 = queue["canonical_text_sha256"]
        .as_str()
        .ok_or_else(|| "Waltons queue missing canonical_text_sha256".to_string())?;
    let paragraphs = queue["matched_paragraphs"]
        .as_array()
        .ok_or_else(|| "Waltons queue missing matched_paragraphs".to_string())?;

    let mut rows = Vec::new();
    for paragraph in paragraphs {
        let requirements = paragraph["matched_research_criterion_refs"]
            .as_array()
            .ok_or_else(|| "paragraph missing matched_research_criterion_refs".to_string())?;
        for requirement in requirements {
            let Some(requirement_ref) = requirement.as_str() else {
                continue;
            };
            let Some(role) = role_for_requirement(requirement_ref) else {
                continue;
            };
            rows.push(WaltonsReviewWorksheetRow {
                include: false,
                paragraph_locator_ref: paragraph["paragraph_locator_ref"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                reported_paragraph_label: paragraph["reported_paragraph_label"]
                    .as_str()
                    .map(str::to_string),
                paragraph_text: paragraph["paragraph_text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                source_revision_ref: source_revision_ref.to_string(),
                canonical_text_sha256: canonical_text_sha256.to_string(),
                requirement_ref: requirement_ref.to_string(),
                role,
                disposition: None,
                reviewer_ref: String::new(),
                review_evidence_refs: Vec::new(),
                review_notes: String::new(),
            });
        }
    }

    write_json(
        &paths.worksheet,
        &WaltonsReviewWorksheet {
            schema_version: "sl.waltons.review_worksheet.v0_1".into(),
            source_queue: paths.queue.display().to_string(),
            candidate_only: true,
            rows,
        },
    )?;
    println!("waltons_review_worksheet={}", paths.worksheet.display());
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WaltonsDecisionFile {
    schema_version: String,
    decisions: Vec<ReviewedWaltonsParagraphDecision>,
}

pub fn review_finalize(paths: &WaltonsPaths) -> CliResult {
    let worksheet: WaltonsReviewWorksheet = read_json(&paths.worksheet)?;
    if worksheet.schema_version != "sl.waltons.review_worksheet.v0_1" {
        return Err(format!(
            "unsupported Waltons review worksheet schema {}",
            worksheet.schema_version
        ));
    }
    let mut decisions = Vec::new();
    for (index, row) in worksheet.rows.into_iter().enumerate() {
        if !row.include {
            continue;
        }
        let disposition = row
            .disposition
            .ok_or_else(|| format!("review row {} missing disposition", index + 1))?;
        if row.reviewer_ref.trim().is_empty() {
            return Err(format!("review row {} missing reviewer_ref", index + 1));
        }
        if row.review_evidence_refs.is_empty()
            || row
                .review_evidence_refs
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err(format!(
                "review row {} requires review_evidence_refs",
                index + 1
            ));
        }
        decisions.push(ReviewedWaltonsParagraphDecision {
            paragraph_locator_ref: row.paragraph_locator_ref,
            source_revision_ref: row.source_revision_ref,
            canonical_text_sha256: row.canonical_text_sha256,
            role: row.role,
            disposition,
            reviewer_ref: row.reviewer_ref,
            review_evidence_refs: row.review_evidence_refs,
        });
    }
    write_json(
        &paths.decisions,
        &WaltonsDecisionFile {
            schema_version: "sl.waltons.review_decisions.v0_1".into(),
            decisions,
        },
    )?;
    println!("waltons_review_decisions={}", paths.decisions.display());
    Ok(())
}

fn compile_waltons_reviewed(
    paths: &WaltonsPaths,
) -> CliResult<Vec<ReviewedWaltonsPropositionEvidenceReceipt>> {
    let materialized = load_waltons_materialisation(paths)?;
    let decisions: WaltonsDecisionFile = read_json(&paths.decisions)?;
    if decisions.schema_version != "sl.waltons.review_decisions.v0_1" {
        return Err(format!(
            "unsupported Waltons decision schema {}",
            decisions.schema_version
        ));
    }
    decisions
        .decisions
        .iter()
        .enumerate()
        .map(|(index, decision)| {
            compile_reviewed_waltons_paragraph(&materialized, decision, index as i64 + 1)
                .map_err(|error| {
                    format!(
                        "compile Waltons review decision {}: {error:?}",
                        index + 1
                    )
                })
        })
        .collect()
}

pub fn review_compile(paths: &WaltonsPaths) -> CliResult {
    let reviewed = compile_waltons_reviewed(paths)?;
    let mut payment_bytes = Vec::new();
    let summaries = reviewed
        .iter()
        .map(|receipt| {
            payment_bytes.extend_from_slice(&receipt.payment_bytes);
            json!({
                "role": receipt.role,
                "proposition_ref": receipt.proposition_ref,
                "paragraph_locator_ref": receipt.paragraph_locator_ref,
                "source_revision_ref": receipt.source_revision_ref,
                "canonical_text_sha256": receipt.canonical_text_sha256,
                "disposition": receipt.disposition,
                "reviewer_ref": receipt.reviewer_ref,
                "review_evidence_refs": receipt.review_evidence_refs,
                "payments_emitted": receipt.payment_receipt.as_ref().map_or(0, |value| value.payments_emitted),
                "candidate_only": receipt.candidate_only,
                "creates_legal_authority": receipt.creates_legal_authority,
                "applicability_promoted": receipt.applicability_promoted,
                "claim_truth_promoted": receipt.claim_truth_promoted,
            })
        })
        .collect::<Vec<_>>();

    let output = json!({
        "schema_version": "sl.waltons.reviewed_proposition_receipts.v0_1",
        "reviewed_receipt_count": summaries.len(),
        "candidate_only": true,
        "creates_legal_authority": false,
        "applicability_promoted": false,
        "claim_truth_promoted": false,
        "receipts": summaries,
    });
    write_json(&paths.reviewed, &output)?;
    fs::write(&paths.payments, payment_bytes)
        .map_err(|error| format!("write {}: {error}", paths.payments.display()))?;

    let trace = waltons_estoppel_trace();
    let reviewed_hops = compile_waltons_proposition_receipts_to_contract_hops(
        &trace,
        WALTONS_AUTHORITY_REF,
        &reviewed,
    );
    write_json(&paths.proposition_hops, &contract_hop_json(&reviewed_hops))?;

    println!(
        "waltons_reviewed_receipts={} payments={} contract_hops={}",
        paths.reviewed.display(),
        paths.payments.display(),
        paths.proposition_hops.display()
    );
    Ok(())
}

pub fn frontier(paths: &WaltonsPaths) -> CliResult {
    let reviewed = compile_waltons_reviewed(paths)?;
    let issue = project_waltons_reviewed_receipts_to_issue(&reviewed)
        .map_err(|error| format!("project Waltons WrongType frontier: {error:?}"))?;

    let elements = issue
        .elements
        .iter()
        .map(|element| {
            json!({
                "element_ref": element.element.element_ref,
                "proposition_ref": element.element.proposition_ref,
                "disposition": format!("{:?}", element.disposition),
                "reviewed_evidence_count": element.evidence.len(),
                "source_revision_refs": element.evidence.iter().map(|e| e.source_revision_ref.clone()).collect::<Vec<_>>(),
                "span_refs": element.evidence.iter().map(|e| e.span_ref.clone()).collect::<Vec<_>>(),
                "candidate_only": element.candidate_only,
                "creates_liability": element.creates_liability,
            })
        })
        .collect::<Vec<_>>();
    let reviewed_but_unpaid = reviewed
        .iter()
        .filter(|receipt| receipt.payment_receipt.is_none())
        .map(|receipt| {
            json!({
                "role": receipt.role,
                "proposition_ref": receipt.proposition_ref,
                "paragraph_locator_ref": receipt.paragraph_locator_ref,
                "source_revision_ref": receipt.source_revision_ref,
                "disposition": receipt.disposition,
                "review_ref": receipt.review_ref,
                "observation_ref": receipt.observation_ref,
                "evidence_coordinate_paid": false,
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
        "reviewed_but_unpaid": reviewed_but_unpaid,
    });
    write_json(&paths.frontier, &output)?;
    println!("waltons_wrongtype_frontier={}", paths.frontier.display());
    Ok(())
}

pub fn cited_by_plan(paths: &WaltonsPaths) -> CliResult {
    let receipt: OalcResolvedSourceReceipt = read_json(&paths.receipt)?;
    let demand = later_treatment_cited_by_demand(
        &receipt,
        Some("prop:estoppel:waltons-treatment".into()),
    );
    let traversal = citation_traversal_plan(&demand)
        .ok_or_else(|| "Waltons cited-by demand produced no traversal plan".to_string())?;
    let output = json!({
        "schema_version": "sl.cited_by_work_manifest.v0_1",
        "root_authority": {
            "source_identity_ref": demand.source_identity_ref,
            "medium_neutral_citation": demand.medium_neutral_citation,
            "source_revision_ref": format!("{}:{}", receipt.corpus_revision_ref, receipt.version_id),
            "canonical_text_digest": receipt.canonical_text_digest,
        },
        "demand": {
            "demand_ref": demand.demand_ref,
            "jurisdiction_ref": demand.jurisdiction_ref,
            "proposition_ref": demand.proposition_ref,
            "use_intent": format!("{:?}", demand.use_intent),
            "treatment_intent": format!("{:?}", demand.treatment_intent),
        },
        "traversal": {
            "provider": format!("{:?}", traversal.provider),
            "operation": format!("{:?}", traversal.operation),
            "max_depth": traversal.bounds.max_depth,
            "max_new_documents": traversal.bounds.max_new_documents,
            "max_network_requests": traversal.bounds.max_network_requests,
            "minimum_pacing_seconds": traversal.bounds.minimum_pacing_seconds,
            "acquisition_authority": format!("{:?}", traversal.acquisition_authority),
        },
        "execution": {
            "state": "provider_adapter_required",
            "text_search_is_not_cited_by_traversal": true,
            "provider_failure_is_negative_legal_evidence": false,
            "candidate_only": true,
            "creates_legal_authority": false,
        }
    });
    write_json(&paths.citedby_manifest, &output)?;
    println!(
        "waltons_cited_by_manifest={}",
        paths.citedby_manifest.display()
    );
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitedByProviderCandidate {
    pub medium_neutral_citation: String,
    pub explicit_reference: Option<String>,
    pub provider_record_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitedByProviderResults {
    pub provider: String,
    pub operation: String,
    pub root_medium_neutral_citation: String,
    pub candidates: Vec<CitedByProviderCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedCitedByCandidate {
    pub medium_neutral_citation: String,
    pub explicit_reference: Option<String>,
    pub provider_record_ref: Option<String>,
    pub candidate_only: bool,
    pub treatment_classified: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedCitedByCandidates {
    pub schema_version: String,
    pub provider: String,
    pub operation: String,
    pub root_medium_neutral_citation: String,
    pub candidate_count: usize,
    pub candidate_only: bool,
    pub provider_result_is_treatment: bool,
    pub candidates: Vec<NormalizedCitedByCandidate>,
}

fn looks_like_mnc(value: &str) -> bool {
    let fields = value.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 3 {
        return false;
    }
    let year = fields[0];
    year.len() == 6
        && year.starts_with('[')
        && year.ends_with(']')
        && year[1..5].bytes().all(|byte| byte.is_ascii_digit())
        && !fields[1].is_empty()
        && fields[1].bytes().all(|byte| byte.is_ascii_alphanumeric())
        && fields[2].bytes().all(|byte| byte.is_ascii_digit())
}

pub fn cited_by_import(paths: &WaltonsPaths, provider_results: &Path) -> CliResult {
    let raw: CitedByProviderResults = read_json(provider_results)?;
    if raw.operation != "CitedBy" {
        return Err("provider result operation must be CitedBy".into());
    }
    if raw.root_medium_neutral_citation != WALTONS_MNC {
        return Err("provider result root citation does not match Waltons".into());
    }
    let mut unique = BTreeMap::new();
    for candidate in raw.candidates {
        let citation = candidate.medium_neutral_citation.trim().to_string();
        if !looks_like_mnc(&citation) {
            return Err(format!("invalid medium-neutral citation {citation:?}"));
        }
        unique.entry(citation.clone()).or_insert(NormalizedCitedByCandidate {
            medium_neutral_citation: citation,
            explicit_reference: candidate.explicit_reference,
            provider_record_ref: candidate.provider_record_ref,
            candidate_only: true,
            treatment_classified: false,
            creates_legal_authority: false,
        });
    }
    let candidates = unique.into_values().collect::<Vec<_>>();
    write_json(
        &paths.citedby_candidates,
        &NormalizedCitedByCandidates {
            schema_version: "sl.cited_by_candidates.v0_1".into(),
            provider: raw.provider,
            operation: raw.operation,
            root_medium_neutral_citation: WALTONS_MNC.into(),
            candidate_count: candidates.len(),
            candidate_only: true,
            provider_result_is_treatment: false,
            candidates,
        },
    )?;
    println!(
        "waltons_cited_by_candidates={}",
        paths.citedby_candidates.display()
    );
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthorityIdentityReviewWorksheet {
    schema_version: String,
    candidate_only: bool,
    rows: Vec<AuthorityIdentityReviewRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthorityIdentityReviewRow {
    include: bool,
    source_receipt_path: String,
    source_document_ref: String,
    citation: String,
    version_id: String,
    date: Option<String>,
    canonical_url: Option<String>,
    suggested_semantic_ref: Option<String>,
    semantic_ref: String,
    label: String,
    doctrine: Option<String>,
    jurisdiction_ref: String,
    court_ref: Option<String>,
    reviewer_ref: String,
    evidence_refs: Vec<String>,
    review_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthorityIdentityDecisionFile {
    schema_version: String,
    decisions: Vec<AuthorityIdentityReviewRow>,
}

fn jurisdiction_for_court_code(code: &str) -> Option<&'static str> {
    match code {
        "HCA" | "FCA" | "FCAFC" => Some("AU"),
        "NSWCA" | "NSWSC" => Some("AU-NSW"),
        "VSCA" | "VSC" => Some("AU-VIC"),
        "QCA" | "QSC" => Some("AU-QLD"),
        "WASCA" | "WASC" => Some("AU-WA"),
        "SASCFC" | "SASC" => Some("AU-SA"),
        "TASFC" | "TASSC" => Some("AU-TAS"),
        "ACTCA" | "ACTSC" => Some("AU-ACT"),
        "NTCA" | "NTSC" => Some("AU-NT"),
        _ => None,
    }
}

fn semantic_ref_suggestion(citation: &str) -> Option<(String, String)> {
    let fields = citation.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 3 {
        return None;
    }
    let year = fields[0].strip_prefix('[')?.strip_suffix(']')?;
    if year.len() != 4 || !year.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let court = fields[1];
    if !court.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return None;
    }
    let number = fields[2];
    if !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let jurisdiction = jurisdiction_for_court_code(court)?;
    let jurisdiction_slug = match jurisdiction {
        "AU" => "au",
        "AU-NSW" => "nsw",
        "AU-VIC" => "vic",
        "AU-QLD" => "qld",
        "AU-WA" => "wa",
        "AU-SA" => "sa",
        "AU-TAS" => "tas",
        "AU-ACT" => "act",
        "AU-NT" => "nt",
        _ => return None,
    };
    Some((
        format!(
            "case:{jurisdiction_slug}:{}:{year}:{number}",
            court.to_ascii_lowercase()
        ),
        jurisdiction.to_string(),
    ))
}

fn parse_contract_doctrine(value: Option<&str>) -> CliResult<Option<ContractDoctrine>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let doctrine = match value {
        "Formation" | "formation" => ContractDoctrine::Formation,
        "Intention" | "intention" => ContractDoctrine::Intention,
        "TermsAndIncorporation" | "terms-and-incorporation" => {
            ContractDoctrine::TermsAndIncorporation
        }
        "Construction" | "construction" => ContractDoctrine::Construction,
        "Estoppel" | "estoppel" => ContractDoctrine::Estoppel,
        "Unconscionability" | "unconscionability" => ContractDoctrine::Unconscionability,
        "Penalties" | "penalties" => ContractDoctrine::Penalties,
        "RepudiationAndTermination" | "repudiation-and-termination" => {
            ContractDoctrine::RepudiationAndTermination
        }
        "Damages" | "damages" => ContractDoctrine::Damages,
        "Restitution" | "restitution" => ContractDoctrine::Restitution,
        "Privity" | "privity" => ContractDoctrine::Privity,
        "ConsumerLaw" | "consumer-law" => ContractDoctrine::ConsumerLaw,
        other => return Err(format!("unsupported contract doctrine {other:?}")),
    };
    Ok(Some(doctrine))
}

fn later_authority_receipt_paths(paths: &WaltonsPaths) -> CliResult<Vec<PathBuf>> {
    let mut receipts = Vec::new();
    let entries = fs::read_dir(&paths.later_dir)
        .map_err(|error| format!("read {}: {error}", paths.later_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read later-authority entry: {error}"))?;
        let path = entry.path().join("oalc-source-receipt.json");
        if path.exists() {
            receipts.push(path);
        }
    }
    receipts.sort();
    Ok(receipts)
}

pub fn identity_prepare(paths: &WaltonsPaths) -> CliResult {
    let receipt_paths = later_authority_receipt_paths(paths)?;
    if receipt_paths.is_empty() {
        return Err("no later-authority OALC receipts found".into());
    }

    let mut rows = Vec::new();
    for receipt_path in receipt_paths {
        let receipt: OalcResolvedSourceReceipt = read_json(&receipt_path)?;
        let suggestion = semantic_ref_suggestion(&receipt.citation);
        rows.push(AuthorityIdentityReviewRow {
            include: false,
            source_receipt_path: receipt_path.display().to_string(),
            source_document_ref: format!("document:oalc:{}", receipt.version_id),
            citation: receipt.citation.clone(),
            version_id: receipt.version_id.clone(),
            date: receipt.date.clone(),
            canonical_url: receipt.canonical_url.clone(),
            suggested_semantic_ref: suggestion.as_ref().map(|(semantic_ref, _)| semantic_ref.clone()),
            semantic_ref: suggestion
                .as_ref()
                .map(|(semantic_ref, _)| semantic_ref.clone())
                .unwrap_or_default(),
            label: receipt.citation.clone(),
            doctrine: None,
            jurisdiction_ref: suggestion
                .map(|(_, jurisdiction)| jurisdiction)
                .unwrap_or_default(),
            court_ref: receipt.court.clone(),
            reviewer_ref: String::new(),
            evidence_refs: Vec::new(),
            review_notes: String::new(),
        });
    }

    write_json(
        &paths.identity_worksheet,
        &AuthorityIdentityReviewWorksheet {
            schema_version: "sl.contract_authority_identity_review_worksheet.v0_1".into(),
            candidate_only: true,
            rows,
        },
    )?;
    println!(
        "authority_identity_review_worksheet={}",
        paths.identity_worksheet.display()
    );
    Ok(())
}

pub fn identity_finalize(paths: &WaltonsPaths) -> CliResult {
    let worksheet: AuthorityIdentityReviewWorksheet = read_json(&paths.identity_worksheet)?;
    if worksheet.schema_version != "sl.contract_authority_identity_review_worksheet.v0_1" {
        return Err(format!(
            "unsupported identity worksheet schema {}",
            worksheet.schema_version
        ));
    }

    let mut decisions = Vec::new();
    for (index, row) in worksheet.rows.into_iter().enumerate() {
        if !row.include {
            continue;
        }
        for (label, value) in [
            ("semantic_ref", row.semantic_ref.as_str()),
            ("label", row.label.as_str()),
            ("jurisdiction_ref", row.jurisdiction_ref.as_str()),
            ("reviewer_ref", row.reviewer_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("identity row {} missing {label}", index + 1));
            }
        }
        if row.evidence_refs.is_empty()
            || row.evidence_refs.iter().any(|value| value.trim().is_empty())
        {
            return Err(format!(
                "identity row {} requires evidence_refs",
                index + 1
            ));
        }
        parse_contract_doctrine(row.doctrine.as_deref())?;
        decisions.push(row);
    }

    write_json(
        &paths.identity_decisions,
        &AuthorityIdentityDecisionFile {
            schema_version: "sl.contract_authority_identity_review_decisions.v0_1".into(),
            decisions,
        },
    )?;
    println!(
        "authority_identity_review_decisions={}",
        paths.identity_decisions.display()
    );
    Ok(())
}

fn compile_identity_reviews(
    paths: &WaltonsPaths,
) -> CliResult<(AustralianContractTrace, ContractReviewedHopCompilation, BTreeMap<String, String>)> {
    let decisions: AuthorityIdentityDecisionFile = read_json(&paths.identity_decisions)?;
    if decisions.schema_version != "sl.contract_authority_identity_review_decisions.v0_1" {
        return Err(format!(
            "unsupported identity decision schema {}",
            decisions.schema_version
        ));
    }

    let mut trace = waltons_estoppel_trace();
    let mut deltas = Vec::new();
    let mut residuals = Vec::new();
    let mut aliases = BTreeMap::new();

    for decision in decisions.decisions {
        let receipt: OalcResolvedSourceReceipt =
            read_json(Path::new(&decision.source_receipt_path))?;
        if receipt.version_id != decision.version_id || receipt.citation != decision.citation {
            return Err(format!(
                "identity review source receipt changed for {}",
                decision.semantic_ref
            ));
        }
        let reviewed = ReviewedContractAuthorityIdentity {
            semantic_ref: decision.semantic_ref.clone(),
            label: decision.label,
            doctrine: parse_contract_doctrine(decision.doctrine.as_deref())?,
            jurisdiction_ref: decision.jurisdiction_ref,
            court_ref: decision.court_ref,
            source_role: SourceRole::PrimaryCaseLaw,
            authority_level: AuthorityLevel::Official,
            reviewer_ref: decision.reviewer_ref,
            evidence_refs: decision.evidence_refs,
            source_receipt: receipt,
            candidate_only: true,
            creates_legal_authority: false,
        };
        let compiled =
            compile_reviewed_authority_identity_to_contract_hop(&trace, &reviewed);
        residuals.extend(compiled.residuals);
        for delta in compiled.deltas {
            let (next, _) = apply_contract_landscape_expansion(&trace, &delta)
                .map_err(|error| format!("apply reviewed identity hop: {error}"))?;
            trace = next;
            deltas.push(delta);
        }
        aliases.insert(decision.source_document_ref, decision.semantic_ref);
    }

    Ok((
        trace,
        ContractReviewedHopCompilation {
            deltas,
            residuals,
            candidate_only: true,
            creates_legal_authority: false,
            creates_current_law_conclusion: false,
        },
        aliases,
    ))
}

pub fn identity_compile(paths: &WaltonsPaths) -> CliResult {
    let (_, compiled, aliases) = compile_identity_reviews(paths)?;
    let mut output = contract_hop_json(&compiled);
    output["reviewed_document_aliases"] = serde_json::to_value(aliases)
        .map_err(|error| format!("encode reviewed document aliases: {error}"))?;
    write_json(&paths.identity_hops, &output)?;
    println!("authority_identity_contract_hops={}", paths.identity_hops.display());
    Ok(())
}

fn safe_citation_dir(citation: &str) -> String {
    citation
        .replace('[', "")
        .replace(']', "")
        .replace(' ', "-")
        .to_ascii_lowercase()
}

pub fn cited_by_worklist(paths: &WaltonsPaths) -> CliResult {
    let normalized: NormalizedCitedByCandidates = read_json(&paths.citedby_candidates)?;
    if normalized.schema_version != "sl.cited_by_candidates.v0_1" {
        return Err(format!(
            "unsupported cited-by candidate schema {}",
            normalized.schema_version
        ));
    }
    fs::create_dir_all(&paths.later_dir)
        .map_err(|error| format!("create {}: {error}", paths.later_dir.display()))?;
    let work = normalized
        .candidates
        .iter()
        .map(|candidate| {
            json!({
                "medium_neutral_citation": candidate.medium_neutral_citation,
                "output_dir": paths.later_dir.join(safe_citation_dir(&candidate.medium_neutral_citation)),
                "state": "primary_source_acquisition_required",
            })
        })
        .collect::<Vec<_>>();
    let output = json!({
        "schema_version": "sl.oalc_case_acquisition_worklist.v0_1",
        "root_medium_neutral_citation": normalized.root_medium_neutral_citation,
        "candidate_only": true,
        "work": work,
    });
    let path = paths.later_dir.join("oalc-acquisition-worklist.json");
    write_json(&path, &output)?;
    println!("oalc_acquisition_worklist={}", path.display());
    Ok(())
}

pub fn cited_by_acquire(paths: &WaltonsPaths) -> CliResult {
    let normalized: NormalizedCitedByCandidates = read_json(&paths.citedby_candidates)?;
    if normalized.schema_version != "sl.cited_by_candidates.v0_1" {
        return Err(format!(
            "unsupported cited-by candidate schema {}",
            normalized.schema_version
        ));
    }
    fs::create_dir_all(&paths.later_dir)
        .map_err(|error| format!("create {}: {error}", paths.later_dir.display()))?;
    for candidate in normalized.candidates {
        let output_dir = paths
            .later_dir
            .join(safe_citation_dir(&candidate.medium_neutral_citation));
        let mut request =
            OalcCaseFollowRequest::for_citation(&candidate.medium_neutral_citation, output_dir.clone());
        request.as_at = DEFAULT_AS_AT.into();
        let receipt = run_live_oalc_case_follow(&request).map_err(|error| {
            format!(
                "acquire cited-by candidate {}: {error:?}",
                candidate.medium_neutral_citation
            )
        })?;
        println!(
            "later_authority_acquired={} receipt={}",
            candidate.medium_neutral_citation,
            receipt.source_receipt_path.display()
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TreatmentQueue {
    schema_version: String,
    root_authority_citation: String,
    source_document_ref: String,
    source_revision_ref: String,
    canonical_text_sha256: String,
    #[serde(default)]
    source_queues: Vec<TreatmentQueueSource>,
    review_unit_count: usize,
    candidate_only: bool,
    treatment_classified: bool,
    review_units: Vec<CitationReviewUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TreatmentQueueSource {
    queue: String,
    source_document_ref: String,
    source_revision_ref: String,
    canonical_text_sha256: String,
}

fn shortlist_exact_waltons(
    candidate: &CitationOccurrenceCandidate,
) -> Option<ResidualShortlistedCitation> {
    (candidate.citation_text.trim() == WALTONS_MNC).then(|| ResidualShortlistedCitation {
        candidate: candidate.clone(),
        matched_criterion_refs: vec!["criterion:treatment:exact-citation:1988-hca-7".into()],
    })
}

pub fn treatment_queue(paths: &WaltonsPaths) -> CliResult {
    let mut receipt_paths = Vec::new();
    let entries = fs::read_dir(&paths.later_dir)
        .map_err(|error| format!("read {}: {error}", paths.later_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read later-authority entry: {error}"))?;
        let receipt = entry.path().join("oalc-source-receipt.json");
        if receipt.exists() {
            receipt_paths.push(receipt);
        }
    }
    receipt_paths.sort();
    if receipt_paths.is_empty() {
        return Err("no later-authority OALC receipts found".into());
    }

    for receipt_path in receipt_paths {
        let receipt: OalcResolvedSourceReceipt = read_json(&receipt_path)?;
        let text = fs::read_to_string(&receipt.local_artifact_ref).map_err(|error| {
            format!(
                "read later authority {}: {error}",
                receipt.local_artifact_ref.display()
            )
        })?;
        let materialized = materialize_oalc_judgment(&receipt, &text, &[])
            .map_err(|error| format!("materialize {}: {error:?}", receipt.citation))?;
        let shortlisted = materialized
            .citation_candidates
            .iter()
            .filter_map(shortlist_exact_waltons)
            .collect::<Vec<_>>();
        let units = cluster_shortlisted_citations(&shortlisted);
        let queue = TreatmentQueue {
            schema_version: "sl.authority_treatment_review_queue.v0_1".into(),
            root_authority_citation: WALTONS_MNC.into(),
            source_document_ref: materialized.document_ref,
            source_revision_ref: materialized.source_revision_ref,
            canonical_text_sha256: materialized.canonical_text_sha256,
            source_queues: Vec::new(),
            review_unit_count: units.len(),
            candidate_only: true,
            treatment_classified: false,
            review_units: units,
        };
        let output = receipt_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("waltons-treatment-review-queue.json");
        write_json(&output, &queue)?;
        println!("treatment_queue={}", output.display());
    }
    Ok(())
}

pub fn treatment_merge(paths: &WaltonsPaths) -> CliResult {
    let mut queues = Vec::new();
    let entries = fs::read_dir(&paths.later_dir)
        .map_err(|error| format!("read {}: {error}", paths.later_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read treatment entry: {error}"))?;
        let path = entry.path().join("waltons-treatment-review-queue.json");
        if path.exists() {
            queues.push(path);
        }
    }
    queues.sort();
    if queues.is_empty() {
        return Err("no per-authority treatment queues found".into());
    }

    let mut units = BTreeMap::new();
    let mut sources = Vec::new();
    for path in queues {
        let queue: TreatmentQueue = read_json(&path)?;
        if queue.schema_version != "sl.authority_treatment_review_queue.v0_1"
            || queue.root_authority_citation != WALTONS_MNC
        {
            return Err(format!("invalid treatment queue {}", path.display()));
        }
        sources.push(TreatmentQueueSource {
            queue: path.display().to_string(),
            source_document_ref: queue.source_document_ref.clone(),
            source_revision_ref: queue.source_revision_ref.clone(),
            canonical_text_sha256: queue.canonical_text_sha256.clone(),
        });
        for unit in queue.review_units {
            if let Some(existing) = units.get(&unit.review_unit_ref) {
                if existing != &unit {
                    return Err(format!("conflicting review unit {}", unit.review_unit_ref));
                }
            } else {
                units.insert(unit.review_unit_ref.clone(), unit);
            }
        }
    }
    let review_units = units.into_values().collect::<Vec<_>>();
    write_json(
        &paths.merged_treatment_queue,
        &TreatmentQueue {
            schema_version: "sl.authority_treatment_review_queue.v0_1".into(),
            root_authority_citation: WALTONS_MNC.into(),
            source_document_ref: "aggregate:later-authorities".into(),
            source_revision_ref: "aggregate:review-unit-preserved-source-revisions".into(),
            canonical_text_sha256: "aggregate:per-unit-digests-preserved".into(),
            source_queues: sources,
            review_unit_count: review_units.len(),
            candidate_only: true,
            treatment_classified: false,
            review_units,
        },
    )?;
    println!(
        "merged_treatment_queue={}",
        paths.merged_treatment_queue.display()
    );
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TreatmentReviewWorksheet {
    schema_version: String,
    source_queue: String,
    root_authority_ref: String,
    candidate_only: bool,
    rows: Vec<TreatmentReviewWorksheetRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TreatmentReviewWorksheetRow {
    include: bool,
    review_unit_ref: String,
    source_revision_ref: String,
    citation_text: String,
    citation_locator_refs: Vec<String>,
    anchor_paragraph_locator_refs: Vec<String>,
    anchor_paragraph_texts: Vec<String>,
    selected_anchor_paragraph_locator_ref: Option<String>,
    citing_proposition_ref: String,
    cited_document_ref: String,
    cited_proposition_ref: String,
    citation_use: Option<CitationUse>,
    reasoning_role: Option<ReasoningRole>,
    condition_coordinates: Vec<ConditionCoordinate>,
    judge_or_speaker_ref: Option<String>,
    court_ref: Option<String>,
    jurisdiction_ref: Option<String>,
    temporal_ref: Option<String>,
    outcome_ref: Option<String>,
    remedy_ref: Option<String>,
    burden_refs: Vec<String>,
    exception_refs: Vec<String>,
    lexical_realisation: String,
    reviewer_ref: String,
    evidence_refs: Vec<String>,
    review_notes: String,
}

pub fn treatment_prepare(paths: &WaltonsPaths) -> CliResult {
    let queue: TreatmentQueue = read_json(&paths.merged_treatment_queue)?;
    let rows = queue
        .review_units
        .into_iter()
        .map(|unit| TreatmentReviewWorksheetRow {
            include: false,
            review_unit_ref: unit.review_unit_ref,
            source_revision_ref: unit.source_revision_ref,
            citation_text: unit.citation_text,
            citation_locator_refs: unit.citation_locator_refs,
            anchor_paragraph_locator_refs: unit.anchor_paragraph_locator_refs,
            anchor_paragraph_texts: unit.anchor_paragraph_texts,
            selected_anchor_paragraph_locator_ref: None,
            citing_proposition_ref: String::new(),
            cited_document_ref: WALTONS_AUTHORITY_REF.into(),
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
    write_json(
        &paths.treatment_worksheet,
        &TreatmentReviewWorksheet {
            schema_version: "sl.citation_treatment_review_worksheet.v0_1".into(),
            source_queue: paths.merged_treatment_queue.display().to_string(),
            root_authority_ref: WALTONS_AUTHORITY_REF.into(),
            candidate_only: true,
            rows,
        },
    )?;
    println!(
        "treatment_review_worksheet={}",
        paths.treatment_worksheet.display()
    );
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TreatmentDecisionFile {
    schema_version: String,
    decisions: Vec<ReviewedCitationReviewUnitDecision>,
}

pub fn treatment_finalize(paths: &WaltonsPaths) -> CliResult {
    let worksheet: TreatmentReviewWorksheet = read_json(&paths.treatment_worksheet)?;
    if worksheet.schema_version != "sl.citation_treatment_review_worksheet.v0_1" {
        return Err(format!(
            "unsupported treatment worksheet schema {}",
            worksheet.schema_version
        ));
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
            return Err(format!(
                "treatment row {} requires evidence_refs",
                index + 1
            ));
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
    write_json(
        &paths.treatment_decisions,
        &TreatmentDecisionFile {
            schema_version: "sl.citation_treatment_review_decisions.v0_1".into(),
            decisions,
        },
    )?;
    println!(
        "treatment_review_decisions={}",
        paths.treatment_decisions.display()
    );
    Ok(())
}

pub fn genealogy(paths: &WaltonsPaths) -> CliResult {
    let queue: TreatmentQueue = read_json(&paths.merged_treatment_queue)?;
    let decisions: TreatmentDecisionFile = read_json(&paths.treatment_decisions)?;
    if decisions.schema_version != "sl.citation_treatment_review_decisions.v0_1" {
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
    let mut receipts = Vec::new();
    for decision in &decisions.decisions {
        let unit = units
            .get(&decision.review_unit_ref)
            .ok_or_else(|| format!("unknown review unit {}", decision.review_unit_ref))?;
        receipts.push(
            compile_reviewed_unit_receipt(unit, decision)
                .map_err(|error| format!("compile treatment review: {error:?}"))?,
        );
    }
    let genealogy = build_temporal_treatment_genealogy(
        WALTONS_AUTHORITY_REF,
        DEFAULT_AS_AT,
        &receipts,
    )
    .map_err(|error| format!("build treatment genealogy: {error:?}"))?;
    let output = json!({
        "schema_version": "sl.temporal_treatment_genealogy.v0_1",
        "reviewed_receipt_count": receipts.len(),
        "candidate_only": true,
        "creates_legal_authority": false,
        "current_law_conclusion": false,
        "genealogy": genealogy,
    });
    write_json(&paths.genealogy, &output)?;

    let (trace, _identity_hops, aliases) = if paths.identity_decisions.exists() {
        compile_identity_reviews(paths)?
    } else {
        (
            waltons_estoppel_trace(),
            ContractReviewedHopCompilation {
                deltas: Vec::new(),
                residuals: Vec::new(),
                candidate_only: true,
                creates_legal_authority: false,
                creates_current_law_conclusion: false,
            },
            BTreeMap::new(),
        )
    };
    let reviewed_hops =
        compile_treatment_receipts_to_contract_hops_with_aliases(&trace, &receipts, &aliases);
    write_json(&paths.treatment_hops, &contract_hop_json(&reviewed_hops))?;

    println!(
        "waltons_genealogy={} contract_hops={}",
        paths.genealogy.display(),
        paths.treatment_hops.display()
    );
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_neutral_citation_gate_accepts_expected_shape_only() {
        assert!(looks_like_mnc("[1988] HCA 7"));
        assert!(looks_like_mnc("[2014] HCA 19"));
        assert!(!looks_like_mnc("Waltons Stores [1988] HCA 7"));
        assert!(!looks_like_mnc("[1988] HCA"));
        assert!(!looks_like_mnc("1988 HCA 7"));
    }

    #[test]
    fn native_paths_keep_review_json_as_surface_artifacts() {
        let paths = WaltonsPaths::from_base(PathBuf::from("/tmp/waltons-native-test"));
        assert_eq!(
            paths.decisions,
            PathBuf::from("/tmp/waltons-native-test/waltons-reviewed-decisions.json")
        );
        assert_eq!(
            paths.genealogy,
            PathBuf::from("/tmp/waltons-native-test/waltons-temporal-treatment-genealogy.json")
        );
    }

    #[test]
    fn requirement_roles_are_typed_and_not_inferred_from_arbitrary_strings() {
        assert_eq!(
            role_for_requirement("requirement:estoppel:reliance"),
            Some(EstoppelRequirementRole::Reliance)
        );
        assert_eq!(role_for_requirement("estoppel-ish"), None);
    }
}
