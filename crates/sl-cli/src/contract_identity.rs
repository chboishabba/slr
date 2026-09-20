use serde::{Deserialize, Serialize};
use sensiblaw_governed_legal_provider::OalcResolvedSourceReceipt;
use sensiblaw_legal_follow_plan::{
    apply_contract_landscape_expansion, AuthorityLevel, AustralianContractTrace,
    ContractDoctrine, SourceRole,
};
use sensiblaw_proof_search_loop::contract_review_expansion::{
    compile_reviewed_authority_identity_to_contract_hop, ContractReviewedHopCompilation,
    ReviewedContractAuthorityIdentity,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

type CliResult<T = ()> = Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityIdentityReviewWorksheet {
    pub schema_version: String,
    pub candidate_only: bool,
    #[serde(default)]
    pub review_complete: bool,
    pub rows: Vec<AuthorityIdentityReviewRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityIdentityReviewRow {
    pub include: bool,
    pub source_receipt_path: String,
    pub source_document_ref: String,
    pub citation: String,
    pub version_id: String,
    pub date: Option<String>,
    pub canonical_url: Option<String>,
    pub suggested_semantic_ref: Option<String>,
    pub semantic_ref: String,
    pub label: String,
    pub doctrine: Option<String>,
    pub jurisdiction_ref: String,
    pub court_ref: Option<String>,
    pub reviewer_ref: String,
    pub evidence_refs: Vec<String>,
    pub review_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityIdentityDecisionFile {
    pub schema_version: String,
    pub decisions: Vec<AuthorityIdentityReviewRow>,
}

pub struct CompiledAuthorityIdentities {
    pub trace: AustralianContractTrace,
    pub compilation: ContractReviewedHopCompilation,
    pub reviewed_document_aliases: BTreeMap<String, String>,
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

fn embedded_mnc_parts(citation: &str) -> Option<(String, String, String)> {
    let fields = citation.split_whitespace().collect::<Vec<_>>();
    for window in fields.windows(3) {
        let Some(year) = window[0]
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        else {
            continue;
        };
        if year.len() != 4 || !year.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let court = window[1].trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        let number = window[2].trim_matches(|ch: char| !ch.is_ascii_digit());
        if court.is_empty()
            || !court.bytes().all(|byte| byte.is_ascii_alphanumeric())
            || number.is_empty()
            || !number.bytes().all(|byte| byte.is_ascii_digit())
        {
            continue;
        }
        return Some((year.to_string(), court.to_string(), number.to_string()));
    }
    None
}

pub fn semantic_ref_suggestion(citation: &str) -> Option<(String, String)> {
    let (year, court, number) = embedded_mnc_parts(citation)?;
    let jurisdiction = jurisdiction_for_court_code(&court)?;
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

pub fn prepare(
    receipt_paths: &[PathBuf],
    output: &Path,
) -> CliResult {
    if receipt_paths.is_empty() {
        return Err("authority identity review requires at least one OALC source receipt".into());
    }
    let mut sorted = receipt_paths.to_vec();
    sorted.sort();
    sorted.dedup();

    let mut rows = Vec::new();
    for receipt_path in sorted {
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
        output,
        &AuthorityIdentityReviewWorksheet {
            schema_version: "sl.contract_authority_identity_review_worksheet.v0_1".into(),
            candidate_only: true,
            review_complete: false,
            rows,
        },
    )
}

pub fn finalize(worksheet_path: &Path, output: &Path) -> CliResult {
    let worksheet: AuthorityIdentityReviewWorksheet = read_json(worksheet_path)?;
    if worksheet.schema_version != "sl.contract_authority_identity_review_worksheet.v0_1" {
        return Err(format!(
            "unsupported identity worksheet schema {}",
            worksheet.schema_version
        ));
    }

    if !worksheet.review_complete {
        return Err(
            "authority identity review worksheet is not marked review_complete=true".into(),
        );
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
        output,
        &AuthorityIdentityDecisionFile {
            schema_version: "sl.contract_authority_identity_review_decisions.v0_1".into(),
            decisions,
        },
    )
}

pub fn compile_against(
    decisions_path: &Path,
    mut trace: AustralianContractTrace,
) -> CliResult<CompiledAuthorityIdentities> {
    let decisions: AuthorityIdentityDecisionFile = read_json(decisions_path)?;
    if decisions.schema_version != "sl.contract_authority_identity_review_decisions.v0_1" {
        return Err(format!(
            "unsupported identity decision schema {}",
            decisions.schema_version
        ));
    }

    let mut deltas = Vec::new();
    let mut residuals = Vec::new();
    let mut aliases = BTreeMap::new();

    for decision in decisions.decisions {
        let receipt: OalcResolvedSourceReceipt =
            read_json(Path::new(&decision.source_receipt_path))?;
        if let (Some(source_date), Some(reviewed_date)) =
            (receipt.date.as_deref(), decision.date.as_deref())
        {
            if source_date.trim() != reviewed_date.trim() {
                return Err(format!(
                    "identity review date conflicts with source receipt for {}: source {:?}, reviewed {:?}",
                    decision.semantic_ref, source_date, reviewed_date
                ));
            }
        }
        if receipt.version_id != decision.version_id || receipt.citation != decision.citation {
            return Err(format!(
                "identity review source receipt changed for {}",
                decision.semantic_ref
            ));
        }
        let expected_source_document_ref = format!("document:oalc:{}", receipt.version_id);
        if decision.source_document_ref != expected_source_document_ref {
            return Err(format!(
                "identity review source document mismatch for {}: expected {}, got {}",
                decision.semantic_ref,
                expected_source_document_ref,
                decision.source_document_ref,
            ));
        }

        let reviewed = ReviewedContractAuthorityIdentity {
            semantic_ref: decision.semantic_ref.clone(),
            label: decision.label,
            doctrine: parse_contract_doctrine(decision.doctrine.as_deref())?,
            jurisdiction_ref: decision.jurisdiction_ref,
            court_ref: decision.court_ref,
            decision_or_effective_date: decision
                .date
                .clone()
                .or_else(|| receipt.date.clone()),
            source_role: match receipt.document_type.as_str() {
                "decision" => SourceRole::PrimaryCaseLaw,
                "primary_legislation" => SourceRole::PrimaryLegislation,
                other => {
                    return Err(format!(
                        "unsupported reviewed OALC document type {other:?}"
                    ))
                }
            },
            authority_level: AuthorityLevel::Official,
            reviewer_ref: decision.reviewer_ref,
            evidence_refs: decision.evidence_refs,
            source_receipt: receipt,
            candidate_only: true,
            creates_legal_authority: false,
        };
        let compiled =
            compile_reviewed_authority_identity_to_contract_hop(&trace, &reviewed);
        let identity_admitted = compiled.residuals.is_empty();
        residuals.extend(compiled.residuals);
        for delta in compiled.deltas {
            let (next, _) = apply_contract_landscape_expansion(&trace, &delta)
                .map_err(|error| format!("apply reviewed identity hop: {error}"))?;
            trace = next;
            deltas.push(delta);
        }
        if identity_admitted {
            aliases.insert(decision.source_document_ref, decision.semantic_ref);
        }
    }

    Ok(CompiledAuthorityIdentities {
        trace,
        compilation: ContractReviewedHopCompilation {
            deltas,
            residuals,
            candidate_only: true,
            creates_legal_authority: false,
            creates_current_law_conclusion: false,
        },
        reviewed_document_aliases: aliases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_case_citation_yields_semantic_ref_suggestion_without_claiming_identity() {
        let (semantic_ref, jurisdiction) =
            semantic_ref_suggestion("Sidhu v Van Dyke [2014] HCA 19").unwrap();
        assert_eq!(semantic_ref, "case:au:hca:2014:19");
        assert_eq!(jurisdiction, "AU");
    }

    #[test]
    fn state_case_citation_preserves_jurisdiction_axis() {
        let (semantic_ref, jurisdiction) =
            semantic_ref_suggestion("Example v Example [2020] QCA 7").unwrap();
        assert_eq!(semantic_ref, "case:qld:qca:2020:7");
        assert_eq!(jurisdiction, "AU-QLD");
    }
}