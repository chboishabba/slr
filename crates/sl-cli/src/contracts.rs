use serde_json::json;
use sensiblaw_governed_legal_provider::{
    resolve_live_oalc_exact_source, OalcCitationMatch, OalcExactSourceRequest,
};
use sensiblaw_legal_follow_plan::{
    australian_contract_landscape_seed, compile_australian_contract_landscape_worklist,
    legal_follow_demand_for_trace_node, plan_legal_sources, AustralianContractLandscapeWorklist,
    ContractLandscapeWorkItem, PlanState, SourceRole,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

type CliResult<T = ()> = Result<T, String>;

fn value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn work_item_json(item: &ContractLandscapeWorkItem) -> serde_json::Value {
    json!({
        "work_ref": item.work_ref,
        "kind": format!("{:?}", item.kind),
        "semantic_ref": item.semantic_ref,
        "related_ref": item.related_ref,
        "doctrine": item.doctrine.map(|value| format!("{value:?}")),
        "jurisdiction_ref": item.jurisdiction_ref,
        "as_at": item.as_at,
        "source_role": format!("{:?}", item.source_role),
        "source_citation": item.source_citation,
        "court_ref": item.court_ref,
        "treatment": item.treatment.map(|value| format!("{value:?}")),
        "active_at_as_at": item.active_at_as_at,
        "candidate_only": item.candidate_only,
        "creates_legal_authority": item.creates_legal_authority,
        "creates_current_law_conclusion": item.creates_current_law_conclusion,
    })
}

fn plan_json(
    work: &AustralianContractLandscapeWorklist,
) -> serde_json::Value {
    let trace = australian_contract_landscape_seed();
    let source = work
        .source_items
        .iter()
        .map(|item| {
            let node = trace
                .nodes
                .get(&item.semantic_ref)
                .expect("worklist source must refer to seed node");
            let demand = legal_follow_demand_for_trace_node(node, &work.as_at)
                .expect("primary source work must compile to LegalFollow demand");
            let plan = plan_legal_sources(&demand, &[]);
            let state = match plan.state {
                PlanState::ReadyPersisted => "ready_persisted",
                PlanState::BlockedMissingContext => "blocked_missing_context",
                PlanState::BlockedAcquisitionRequired => "blocked_acquisition_required",
            };
            json!({
                "work": work_item_json(item),
                "legal_follow": {
                    "demand_ref": demand.demand_ref,
                    "origin_ref": demand.origin_ref,
                    "jurisdiction_ref": demand.jurisdiction_ref,
                    "source_roles": demand.source_roles.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>(),
                    "authority_levels": demand.authority_levels.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>(),
                    "provider_profile_refs": demand.provider_profile_refs,
                    "requested_facets": demand.requested_facets,
                    "temporal_refs": demand.temporal_refs,
                    "plan_state": state,
                    "blocked_reasons": plan.blocked_reasons,
                    "authority": plan.authority,
                }
            })
        })
        .collect::<Vec<_>>();

    json!({
        "schema_version": "sl.australian_contract_landscape_worklist.v0_1",
        "root_ref": work.root_ref,
        "as_at": work.as_at,
        "jurisdiction_filter": work.jurisdiction_filter,
        "bounded_seed_only": work.bounded_seed_only,
        "candidate_only": work.candidate_only,
        "creates_legal_authority": work.creates_legal_authority,
        "creates_current_law_conclusion": work.creates_current_law_conclusion,
        "frontier_counts": {
            "primary_source_acquisition": work.source_items.len(),
            "authority_treatment_review": work.treatment_items.len(),
            "context_expansion": work.context_items.len(),
            "temporal_alternatives": work.temporal_alternatives.len(),
        },
        "primary_source_acquisition": source,
        "authority_treatment_review": work.treatment_items.iter().map(work_item_json).collect::<Vec<_>>(),
        "context_expansion": work.context_items.iter().map(work_item_json).collect::<Vec<_>>(),
        "temporal_alternatives": work.temporal_alternatives.iter().map(work_item_json).collect::<Vec<_>>(),
    })
}

fn compile(args: &[String]) -> CliResult<(AustralianContractLandscapeWorklist, serde_json::Value)> {
    let as_at = value(args, "--as-at").unwrap_or_else(|| "2026-09-20".into());
    let jurisdiction = value(args, "--jurisdiction");
    let trace = australian_contract_landscape_seed();
    let work = compile_australian_contract_landscape_worklist(
        &trace,
        &as_at,
        jurisdiction.as_deref(),
    )?;
    let output = plan_json(&work);
    Ok((work, output))
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn safe_ref(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn case_neutral_citation(source_citation: &str) -> String {
    source_citation
        .split(';')
        .next()
        .unwrap_or(source_citation)
        .trim()
        .to_string()
}

fn legislation_act_citation(source_citation: &str) -> (String, Option<String>) {
    if let Some((act, section)) = source_citation.rsplit_once(" s ") {
        let section = section.trim();
        if !section.is_empty()
            && section
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '(' || ch == ')' || ch == '-')
        {
            return (act.trim().to_string(), Some(format!("s {section}")));
        }
    }
    (source_citation.trim().to_string(), None)
}

fn oalc_jurisdiction(jurisdiction_ref: &str) -> Option<String> {
    match jurisdiction_ref {
        "AU" => Some("commonwealth".into()),
        "AU-NSW" => Some("new_south_wales".into()),
        "AU-VIC" => Some("victoria".into()),
        "AU-QLD" => Some("queensland".into()),
        "AU-WA" => Some("western_australia".into()),
        "AU-SA" => Some("south_australia".into()),
        "AU-TAS" => Some("tasmania".into()),
        "AU-ACT" => Some("australian_capital_territory".into()),
        "AU-NT" => Some("northern_territory".into()),
        _ => None,
    }
}

fn acquire_primary_sources(
    work: &AustralianContractLandscapeWorklist,
    output_dir: &Path,
) -> CliResult<serde_json::Value> {
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("create {}: {error}", output_dir.display()))?;
    let mut receipts = Vec::new();

    for item in &work.source_items {
        let (citation, citation_match, document_type, section_ref) = match item.source_role {
            SourceRole::PrimaryCaseLaw => (
                case_neutral_citation(&item.source_citation),
                OalcCitationMatch::Contains,
                "decision".to_string(),
                None,
            ),
            SourceRole::PrimaryLegislation => {
                let (act, section) = legislation_act_citation(&item.source_citation);
                (
                    act,
                    OalcCitationMatch::Exact,
                    "primary_legislation".to_string(),
                    section,
                )
            }
            _ => continue,
        };

        let resolved = resolve_live_oalc_exact_source(&OalcExactSourceRequest {
            citation: citation.clone(),
            citation_match,
            document_type,
            source: None,
            jurisdiction: oalc_jurisdiction(&item.jurisdiction_ref),
        })
        .map_err(|error| {
            format!(
                "acquire {} ({citation}): {error:?}",
                item.semantic_ref
            )
        })?;

        let artifact_dir = output_dir.join(safe_ref(&item.semantic_ref));
        fs::create_dir_all(&artifact_dir)
            .map_err(|error| format!("create {}: {error}", artifact_dir.display()))?;
        let artifact = artifact_dir.join("source.txt");
        fs::write(&artifact, resolved.row.text.as_bytes())
            .map_err(|error| format!("write {}: {error}", artifact.display()))?;
        let digest = sha256(resolved.row.text.as_bytes());

        let receipt = json!({
            "semantic_ref": item.semantic_ref,
            "work_ref": item.work_ref,
            "doctrine": item.doctrine.map(|value| format!("{value:?}")),
            "jurisdiction_ref": item.jurisdiction_ref,
            "as_at": item.as_at,
            "source_role": format!("{:?}", item.source_role),
            "requested_citation": citation,
            "section_ref": section_ref,
            "oalc_version_id": resolved.row.version_id,
            "oalc_corpus_revision": format!(
                "isaacus/open-australian-legal-corpus@{}",
                resolved.corpus_revision_sha
            ),
            "oalc_source": resolved.row.source,
            "oalc_jurisdiction": resolved.row.jurisdiction,
            "oalc_document_type": resolved.row.document_type,
            "resolved_citation": resolved.row.citation,
            "decision_or_effective_date": resolved.row.date,
            "canonical_url": resolved.row.url,
            "when_scraped": resolved.row.when_scraped,
            "canonical_text_sha256": digest,
            "local_artifact_ref": artifact,
            "resolution_path": resolved.resolution_path,
            "network_requests": resolved.network_requests,
            "candidate_only": true,
            "creates_legal_authority": false,
            "creates_current_law_conclusion": false,
            "section_receipt_paid": false,
            "treatment_review_paid": false,
        });
        let receipt_path = artifact_dir.join("source-receipt.json");
        write_output(&receipt_path, &receipt)?;
        receipts.push(receipt);
    }

    Ok(json!({
        "schema_version": "sl.australian_contract_landscape_acquisition.v0_1",
        "root_ref": work.root_ref,
        "as_at": work.as_at,
        "jurisdiction_filter": work.jurisdiction_filter,
        "bounded_seed_only": work.bounded_seed_only,
        "acquired_source_count": receipts.len(),
        "candidate_only": true,
        "creates_legal_authority": false,
        "creates_current_law_conclusion": false,
        "section_receipts_paid": false,
        "treatment_review_paid": false,
        "receipts": receipts,
    }))
}

fn write_output(path: &Path, output: &serde_json::Value) -> CliResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(output)
            .map_err(|error| format!("encode contract landscape worklist: {error}"))?,
    )
    .map_err(|error| format!("write {}: {error}", path.display()))
}

pub fn run(args: Vec<String>) -> CliResult {
    match args.as_slice() {
        [scope, command, rest @ ..] if scope == "landscape" && command == "plan" => {
            let (work, output) = compile(rest)?;
            if let Some(path) = value(rest, "--output").map(PathBuf::from) {
                write_output(&path, &output)?;
                println!("contracts_landscape_worklist={}", path.display());
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output)
                        .map_err(|error| format!("encode landscape worklist: {error}"))?
                );
            }
            println!(
                "contracts_landscape_frontier source={} treatment={} context={} temporal={} bounded_seed_only={} authority=false current_law_conclusion=false",
                work.source_items.len(),
                work.treatment_items.len(),
                work.context_items.len(),
                work.temporal_alternatives.len(),
                work.bounded_seed_only,
            );
            Ok(())
        }
        [scope, command, rest @ ..] if scope == "landscape" && command == "status" => {
            let (work, _) = compile(rest)?;
            println!(
                "root={} as_at={} jurisdiction={} source={} treatment={} context={} temporal={} candidate_only={} authority={} current_law_conclusion={}",
                work.root_ref,
                work.as_at,
                work.jurisdiction_filter.as_deref().unwrap_or("ALL"),
                work.source_items.len(),
                work.treatment_items.len(),
                work.context_items.len(),
                work.temporal_alternatives.len(),
                work.candidate_only,
                work.creates_legal_authority,
                work.creates_current_law_conclusion,
            );
            Ok(())
        }
        [scope, command, rest @ ..] if scope == "landscape" && command == "acquire" => {
            let (work, _) = compile(rest)?;
            let output_dir = value(rest, "--output-dir")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from("artifacts/oalc/contracts/landscape")
                        .join(&work.as_at)
                });
            let receipt = acquire_primary_sources(&work, &output_dir)?;
            let receipt_path = output_dir.join("landscape-acquisition-receipt.json");
            write_output(&receipt_path, &receipt)?;
            println!(
                "contracts_landscape_acquisition={} sources={} bounded_seed_only={} authority=false current_law_conclusion=false",
                receipt_path.display(),
                receipt["acquired_source_count"],
                work.bounded_seed_only,
            );
            Ok(())
        }
        _ => Err(
            "usage: sensiblaw legal-follow contracts landscape <plan|status|acquire> [--as-at YYYY-MM-DD] [--jurisdiction AU-QLD] [--output PATH] [--output-dir PATH]"
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qld_2026_plan_keeps_successor_active_and_predecessor_temporal() {
        let args = vec![
            "--as-at".into(),
            "2026-09-20".into(),
            "--jurisdiction".into(),
            "AU-QLD".into(),
        ];
        let (work, output) = compile(&args).unwrap();
        assert!(work.source_items.iter().any(|item| {
            item.semantic_ref == "legislation:qld:property-law-act-2023:s68"
        }));
        assert!(work.temporal_alternatives.iter().any(|item| {
            item.semantic_ref == "legislation:qld:property-law-act-1974:s55"
        }));
        assert_eq!(output["bounded_seed_only"], true);
        assert_eq!(output["creates_legal_authority"], false);
        assert_eq!(output["creates_current_law_conclusion"], false);
    }

    #[test]
    fn citation_helpers_preserve_section_as_downstream_residual() {
        assert_eq!(
            case_neutral_citation("[1988] HCA 7; 164 CLR 387"),
            "[1988] HCA 7"
        );
        assert_eq!(
            legislation_act_citation("Property Law Act 2023 (Qld) s 68"),
            ("Property Law Act 2023 (Qld)".into(), Some("s 68".into()))
        );
    }

    #[test]
    fn source_frontier_compiles_to_acquisition_only_legal_follow_plans() {
        let (work, output) = compile(&["--as-at".into(), "2026-09-20".into()]).unwrap();
        assert!(!work.source_items.is_empty());
        for entry in output["primary_source_acquisition"].as_array().unwrap() {
            assert_eq!(
                entry["legal_follow"]["plan_state"],
                "blocked_acquisition_required"
            );
            assert_eq!(entry["legal_follow"]["authority"], "acquisition_plan_only");
        }
    }
}
