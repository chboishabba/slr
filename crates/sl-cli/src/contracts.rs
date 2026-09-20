use serde::Deserialize;
use serde_json::json;
use sensiblaw_governed_legal_provider::{
    resolve_live_oalc_exact_source, OalcCitationMatch, OalcExactSourceRequest,
};
use sensiblaw_proof_search_loop::contract_review_expansion::ContractReviewedHopResidual;
use sensiblaw_legal_follow_plan::{
    australian_contract_landscape_seed, compile_australian_contract_landscape_worklist,
    apply_contract_landscape_expansion, legal_follow_demand_for_trace_node, plan_legal_sources,
    AuthorityLevel, AustralianContractLandscapeWorklist, AustralianContractTrace,
    ContractDoctrine, ContractLandscapeExpansionDelta, ContractLandscapeWorkItem,
    ContractTraceEdge, ContractTraceNode, PlanState, SourceRole, TraceNodeKind, TreatmentKind,
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

fn values(args: &[String], flag: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        if arg == flag {
            if let Some(value) = args.get(index + 1) {
                out.push(value.clone());
            }
        }
    }
    out
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
    trace: &AustralianContractTrace,
    work: &AustralianContractLandscapeWorklist,
) -> serde_json::Value {
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
    let output = plan_json(&trace, &work);
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
    let mut residuals = Vec::new();

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

        let request = OalcExactSourceRequest {
            citation: citation.clone(),
            citation_match,
            document_type,
            source: None,
            jurisdiction: oalc_jurisdiction(&item.jurisdiction_ref),
        };
        let resolved = match resolve_live_oalc_exact_source(&request) {
            Ok(resolved) => resolved,
            Err(error) => {
                residuals.push(json!({
                    "work_ref": item.work_ref,
                    "semantic_ref": item.semantic_ref,
                    "jurisdiction_ref": item.jurisdiction_ref,
                    "as_at": item.as_at,
                    "source_role": format!("{:?}", item.source_role),
                    "requested_citation": citation,
                    "section_ref": section_ref,
                    "residual": format!("{error:?}"),
                    "missing_source_is_negative_legal_evidence": false,
                    "candidate_only": true,
                    "creates_legal_authority": false,
                    "creates_current_law_conclusion": false,
                }));
                continue;
            }
        };

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
        "source_work_count": work.source_items.len(),
        "acquired_source_count": receipts.len(),
        "source_residual_count": residuals.len(),
        "candidate_only": true,
        "creates_legal_authority": false,
        "creates_current_law_conclusion": false,
        "section_receipts_paid": false,
        "treatment_review_paid": false,
        "missing_source_is_negative_legal_evidence": false,
        "receipts": receipts,
        "residuals": residuals,
    }))
}

#[derive(Debug, Deserialize)]
struct ExpansionInput {
    provenance_ref: String,
    #[serde(default)]
    nodes: Vec<ExpansionNodeInput>,
    #[serde(default)]
    edges: Vec<ExpansionEdgeInput>,
}

#[derive(Debug, Deserialize)]
struct ExpansionNodeInput {
    semantic_ref: String,
    label: String,
    kind: String,
    doctrine: Option<String>,
    jurisdiction_ref: String,
    court_ref: Option<String>,
    decision_or_effective_date: Option<String>,
    valid_from: Option<String>,
    valid_to: Option<String>,
    source_role: String,
    authority_level: String,
    source_citation: String,
}

#[derive(Debug, Deserialize)]
struct ExpansionEdgeInput {
    from_ref: String,
    to_ref: String,
    treatment: String,
}

fn parse_doctrine(value: &str) -> CliResult<ContractDoctrine> {
    match value {
        "Formation" | "formation" => Ok(ContractDoctrine::Formation),
        "Intention" | "intention" => Ok(ContractDoctrine::Intention),
        "TermsAndIncorporation" | "terms-and-incorporation" => {
            Ok(ContractDoctrine::TermsAndIncorporation)
        }
        "Construction" | "construction" => Ok(ContractDoctrine::Construction),
        "Estoppel" | "estoppel" => Ok(ContractDoctrine::Estoppel),
        "Unconscionability" | "unconscionability" => Ok(ContractDoctrine::Unconscionability),
        "Penalties" | "penalties" => Ok(ContractDoctrine::Penalties),
        "RepudiationAndTermination" | "repudiation-and-termination" => {
            Ok(ContractDoctrine::RepudiationAndTermination)
        }
        "Damages" | "damages" => Ok(ContractDoctrine::Damages),
        "Restitution" | "restitution" => Ok(ContractDoctrine::Restitution),
        "Privity" | "privity" => Ok(ContractDoctrine::Privity),
        "ConsumerLaw" | "consumer-law" => Ok(ContractDoctrine::ConsumerLaw),
        other => Err(format!("unsupported contract doctrine {other:?}")),
    }
}

fn parse_node_kind(value: &str) -> CliResult<TraceNodeKind> {
    match value {
        "Doctrine" | "doctrine" => Ok(TraceNodeKind::Doctrine),
        "CaseAuthority" | "case-authority" => Ok(TraceNodeKind::CaseAuthority),
        "Legislation" | "legislation" => Ok(TraceNodeKind::Legislation),
        "ResearchRequirement" | "research-requirement" => {
            Ok(TraceNodeKind::ResearchRequirement)
        }
        "Matter" | "matter" => Ok(TraceNodeKind::Matter),
        other => Err(format!("unsupported trace node kind {other:?}")),
    }
}

fn parse_source_role(value: &str) -> CliResult<SourceRole> {
    match value {
        "PrimaryCaseLaw" | "primary-case-law" => Ok(SourceRole::PrimaryCaseLaw),
        "PrimaryLegislation" | "primary-legislation" => Ok(SourceRole::PrimaryLegislation),
        "OfficialRecord" | "official-record" => Ok(SourceRole::OfficialRecord),
        "ResearchIndex" | "research-index" => Ok(SourceRole::ResearchIndex),
        "SecondaryAnalysis" | "secondary-analysis" => Ok(SourceRole::SecondaryAnalysis),
        other => Err(format!("unsupported source role {other:?}")),
    }
}

fn parse_authority_level(value: &str) -> CliResult<AuthorityLevel> {
    match value {
        "Official" | "official" => Ok(AuthorityLevel::Official),
        "Supporting" | "supporting" => Ok(AuthorityLevel::Supporting),
        "Secondary" | "secondary" => Ok(AuthorityLevel::Secondary),
        other => Err(format!("unsupported authority level {other:?}")),
    }
}

fn parse_treatment(value: &str) -> CliResult<TreatmentKind> {
    match value {
        "Seeds" | "seeds" => Ok(TreatmentKind::Seeds),
        "Supports" | "supports" => Ok(TreatmentKind::Supports),
        "Applies" | "applies" => Ok(TreatmentKind::Applies),
        "Follows" | "follows" => Ok(TreatmentKind::Follows),
        "Distinguishes" | "distinguishes" => Ok(TreatmentKind::Distinguishes),
        "Qualifies" | "qualifies" => Ok(TreatmentKind::Qualifies),
        "Displaces" | "displaces" => Ok(TreatmentKind::Displaces),
        "TemporalSuccessor" | "temporal-successor" => Ok(TreatmentKind::TemporalSuccessor),
        "Requires" | "requires" => Ok(TreatmentKind::Requires),
        "Intersects" | "intersects" => Ok(TreatmentKind::Intersects),
        other => Err(format!("unsupported treatment kind {other:?}")),
    }
}

fn expansion_input_to_delta(
    input: ExpansionInput,
) -> CliResult<ContractLandscapeExpansionDelta> {
    if input.provenance_ref.trim().is_empty() {
        return Err("expansion delta requires non-empty provenance_ref".into());
    }

    let mut nodes = Vec::new();
    for node in input.nodes {
        nodes.push(ContractTraceNode {
            semantic_ref: node.semantic_ref,
            label: node.label,
            kind: parse_node_kind(&node.kind)?,
            doctrine: node
                .doctrine
                .as_deref()
                .map(parse_doctrine)
                .transpose()?,
            jurisdiction_ref: node.jurisdiction_ref,
            court_ref: node.court_ref,
            decision_or_effective_date: node.decision_or_effective_date,
            valid_from: node.valid_from,
            valid_to: node.valid_to,
            source_role: parse_source_role(&node.source_role)?,
            authority_level: parse_authority_level(&node.authority_level)?,
            source_citation: node.source_citation,
            candidate_only: true,
            creates_legal_authority: false,
        });
    }

    let mut edges = Vec::new();
    for edge in input.edges {
        edges.push(ContractTraceEdge {
            from_ref: edge.from_ref,
            to_ref: edge.to_ref,
            treatment: parse_treatment(&edge.treatment)?,
            candidate_only: true,
            creates_legal_authority: false,
        });
    }

    Ok(ContractLandscapeExpansionDelta {
        discovered_nodes: nodes,
        discovered_edges: edges,
        provenance_ref: input.provenance_ref,
        candidate_only: true,
        creates_legal_authority: false,
    })
}

struct LoadedExpansionArtifact {
    deltas: Vec<ContractLandscapeExpansionDelta>,
    reviewed_residuals: Vec<serde_json::Value>,
}

fn load_expansion_artifact(path: &Path) -> CliResult<LoadedExpansionArtifact> {
    let bytes = fs::read(path)
        .map_err(|error| format!("read expansion delta {}: {error}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode expansion delta {}: {error}", path.display()))?;

    let reviewed_residuals = value
        .get("residuals")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let inputs = if let Some(deltas) = value.get("deltas").and_then(|value| value.as_array()) {
        deltas
            .iter()
            .cloned()
            .map(|value| {
                serde_json::from_value::<ExpansionInput>(value)
                    .map_err(|error| format!("decode reviewed-hop delta {}: {error}", path.display()))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![serde_json::from_value::<ExpansionInput>(value)
            .map_err(|error| format!("decode expansion delta {}: {error}", path.display()))?]
    };

    Ok(LoadedExpansionArtifact {
        deltas: inputs
            .into_iter()
            .map(expansion_input_to_delta)
            .collect::<CliResult<Vec<_>>>()?,
        reviewed_residuals,
    })
}

#[derive(Debug)]
pub struct NativeExpansionBatch {
    pub source_ref: String,
    pub deltas: Vec<ContractLandscapeExpansionDelta>,
    pub reviewed_residuals: Vec<ContractReviewedHopResidual>,
}

pub fn run_native_expansion_trajectory(
    mut expanded: AustralianContractTrace,
    batches: Vec<NativeExpansionBatch>,
    as_at: &str,
    jurisdiction: Option<&str>,
) -> CliResult<serde_json::Value> {
    expanded.validate()?;
    if as_at.trim().is_empty() {
        return Err("native contracts trajectory requires an as-at date".into());
    }

    let mut trajectory = Vec::new();
    let mut reviewed_residuals = Vec::new();
    let mut hop_index = 0usize;

    for batch in batches {
        reviewed_residuals.extend(
            batch
                .reviewed_residuals
                .into_iter()
                .map(|residual| {
                    json!({
                        "source_artifact": batch.source_ref,
                        "residual": residual,
                    })
                }),
        );

        for delta in batch.deltas {
            hop_index += 1;
            let (next, receipt) =
                apply_contract_landscape_expansion(&expanded, &delta)?;
            expanded = next;
            let work = compile_australian_contract_landscape_worklist(
                &expanded,
                as_at,
                jurisdiction,
            )?;
            trajectory.push(json!({
                "hop_index": hop_index,
                "source_ref": batch.source_ref,
                "expansion_receipt": expansion_receipt_json(&receipt),
                "frontier_counts": {
                    "primary_source_acquisition": work.source_items.len(),
                    "authority_treatment_review": work.treatment_items.len(),
                    "context_expansion": work.context_items.len(),
                    "temporal_alternatives": work.temporal_alternatives.len(),
                },
            }));
        }
    }

    let work = compile_australian_contract_landscape_worklist(
        &expanded,
        as_at,
        jurisdiction,
    )?;
    Ok(json!({
        "schema_version": "sl.australian_contract_landscape_native_trajectory.v0_1",
        "transport": "typed_rust_in_process",
        "json_is_semantic_command_transport": false,
        "hop_count": trajectory.len(),
        "reviewed_residual_count": reviewed_residuals.len(),
        "reviewed_residuals": reviewed_residuals,
        "trajectory": trajectory,
        "final_recomputed_worklist": plan_json(&expanded, &work),
        "candidate_only": true,
        "creates_legal_authority": false,
        "creates_current_law_conclusion": false,
    }))
}

fn expansion_receipt_json(
    receipt: &sensiblaw_legal_follow_plan::ContractLandscapeExpansionReceipt,
) -> serde_json::Value {
    json!({
        "provenance_ref": receipt.provenance_ref,
        "added_node_count": receipt.added_node_count,
        "added_edge_count": receipt.added_edge_count,
        "recompute_frontier_required": receipt.recompute_frontier_required,
        "old_source_history_preserved": receipt.old_source_history_preserved,
        "old_conclusions_frozen": receipt.old_conclusions_frozen,
        "candidate_only": receipt.candidate_only,
        "creates_legal_authority": receipt.creates_legal_authority,
        "creates_current_law_conclusion": receipt.creates_current_law_conclusion,
    })
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
        [scope, command, rest @ ..] if scope == "landscape" && command == "expand" => {
            let delta_paths = values(rest, "--delta")
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            if delta_paths.is_empty() {
                return Err("landscape expand requires at least one --delta PATH".into());
            }
            let as_at = value(rest, "--as-at").unwrap_or_else(|| "2026-09-20".into());
            let jurisdiction = value(rest, "--jurisdiction");
            let mut expanded = australian_contract_landscape_seed();
            let mut trajectory = Vec::new();
            let mut reviewed_residuals = Vec::new();

            let mut hop_index = 0usize;
            for delta_path in &delta_paths {
                let artifact = load_expansion_artifact(delta_path)?;
                reviewed_residuals.extend(
                    artifact
                        .reviewed_residuals
                        .into_iter()
                        .map(|residual| json!({
                            "source_artifact": delta_path,
                            "residual": residual,
                        })),
                );
                for delta in artifact.deltas {
                    hop_index += 1;
                    let (next, receipt) =
                        apply_contract_landscape_expansion(&expanded, &delta)?;
                    expanded = next;
                    let work = compile_australian_contract_landscape_worklist(
                        &expanded,
                        &as_at,
                        jurisdiction.as_deref(),
                    )?;
                    trajectory.push(json!({
                        "hop_index": hop_index,
                        "delta_path": delta_path,
                        "expansion_receipt": expansion_receipt_json(&receipt),
                        "frontier_counts": {
                            "primary_source_acquisition": work.source_items.len(),
                            "authority_treatment_review": work.treatment_items.len(),
                            "context_expansion": work.context_items.len(),
                            "temporal_alternatives": work.temporal_alternatives.len(),
                        },
                    }));
                }
            }

            let work = compile_australian_contract_landscape_worklist(
                &expanded,
                &as_at,
                jurisdiction.as_deref(),
            )?;
            let output = json!({
                "schema_version": "sl.australian_contract_landscape_adaptive_trajectory.v0_1",
                "hop_count": trajectory.len(),
                "reviewed_residual_count": reviewed_residuals.len(),
                "reviewed_residuals": reviewed_residuals,
                "trajectory": trajectory,
                "final_recomputed_worklist": plan_json(&expanded, &work),
                "candidate_only": true,
                "creates_legal_authority": false,
                "creates_current_law_conclusion": false,
            });
            if let Some(path) = value(rest, "--output").map(PathBuf::from) {
                write_output(&path, &output)?;
                println!("contracts_landscape_adaptive_trajectory={}", path.display());
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output)
                        .map_err(|error| format!("encode adaptive trajectory: {error}"))?
                );
            }
            println!(
                "contracts_landscape_recomputed hops={} source={} treatment={} context={} temporal={} authority=false current_law_conclusion=false",
                output["hop_count"],
                work.source_items.len(),
                work.treatment_items.len(),
                work.context_items.len(),
                work.temporal_alternatives.len(),
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
            "usage: sensiblaw legal-follow contracts landscape <plan|status|expand|acquire> [--as-at YYYY-MM-DD] [--jurisdiction AU-QLD] [--delta PATH] [--output PATH] [--output-dir PATH]"
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
    fn expansion_input_cannot_supply_authority_bits() {
        let input = ExpansionNodeInput {
            semantic_ref: "case:fixture:construction".into(),
            label: "fixture".into(),
            kind: "case-authority".into(),
            doctrine: Some("construction".into()),
            jurisdiction_ref: "AU".into(),
            court_ref: Some("court:fixture".into()),
            decision_or_effective_date: None,
            valid_from: None,
            valid_to: None,
            source_role: "primary-case-law".into(),
            authority_level: "official".into(),
            source_citation: "fixture:construction".into(),
        };
        let node = ContractTraceNode {
            semantic_ref: input.semantic_ref,
            label: input.label,
            kind: parse_node_kind(&input.kind).unwrap(),
            doctrine: input.doctrine.as_deref().map(parse_doctrine).transpose().unwrap(),
            jurisdiction_ref: input.jurisdiction_ref,
            court_ref: input.court_ref,
            decision_or_effective_date: input.decision_or_effective_date,
            valid_from: input.valid_from,
            valid_to: input.valid_to,
            source_role: parse_source_role(&input.source_role).unwrap(),
            authority_level: parse_authority_level(&input.authority_level).unwrap(),
            source_citation: input.source_citation,
            candidate_only: true,
            creates_legal_authority: false,
        };
        assert!(node.candidate_only);
        assert!(!node.creates_legal_authority);
    }

    #[test]
    fn reviewed_hop_envelope_loads_without_manual_reshaping() {
        let path = std::env::temp_dir().join(format!(
            "sensiblaw-reviewed-hop-envelope-{}.json",
            std::process::id()
        ));
        let envelope = json!({
            "schema_version": "sl.australian_contracts.reviewed_hops.v0_1",
            "delta_count": 1,
            "residual_count": 0,
            "candidate_only": true,
            "creates_legal_authority": false,
            "creates_current_law_conclusion": false,
            "deltas": [{
                "provenance_ref": "review:fixture",
                "candidate_only": true,
                "creates_legal_authority": false,
                "nodes": [{
                    "semantic_ref": "case:fixture:reviewed-hop",
                    "label": "Reviewed hop fixture",
                    "kind": "CaseAuthority",
                    "doctrine": "Construction",
                    "jurisdiction_ref": "AU",
                    "court_ref": "court:fixture",
                    "decision_or_effective_date": "2000-01-01",
                    "valid_from": null,
                    "valid_to": null,
                    "source_role": "PrimaryCaseLaw",
                    "authority_level": "Official",
                    "source_citation": "fixture:reviewed-hop"
                }],
                "edges": []
            }],
            "residuals": []
        });
        write_output(&path, &envelope).unwrap();
        let artifact = load_expansion_artifact(&path).unwrap();
        let deltas = artifact.deltas;
        let _ = fs::remove_file(&path);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0].discovered_nodes[0].semantic_ref,
            "case:fixture:reviewed-hop"
        );
        assert!(!deltas[0].creates_legal_authority);
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
