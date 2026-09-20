        creates_legal_authority: false,
        creates_current_law_conclusion: false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OutboundFrontierEnvelope {
    schema_version: String,
    parent_campaign_receipt: String,
    source_receipt_path: String,
    source_semantic_ref: String,
    residual_count: usize,
    selected: Option<OutboundCitationResidual>,
    residuals: Vec<OutboundCitationResidual>,
    selector_is_legal_truth_rank: bool,
    budget: CampaignBudget,
    accepted_hop_count: usize,
    source_acquisition_count: usize,
    network_request_count: u64,
    candidate_only: bool,
    creates_legal_authority: bool,
    creates_current_law_conclusion: bool,
}

pub fn run(args: Vec<String>) -> CampaignResult<()> {
    match args.as_slice() {
        [command, rest @ ..] if command == "discover" => {
            let trajectory = PathBuf::from(required_arg(rest, "--trajectory")?);
            let source_receipt = PathBuf::from(required_arg(rest, "--source-receipt")?);
            let source_semantic_ref = required_arg(rest, "--source-semantic-ref")?;
            let output = PathBuf::from(required_arg(rest, "--output")?);
            let snapshot = trace_snapshot_from_trajectory(&trajectory)?;
            let trace = restore_trace(&snapshot)?;
            let config = config_from_trajectory(&trajectory)?;
            let counters = counters_from_trajectory(&trajectory)?;
            let (_, materialization) = materialize_retained_oalc_receipt(&source_receipt)?;
            let residuals =
                discover_outbound_citation_residuals(&trace, &source_semantic_ref, &materialization);
            let selected = select_fresh_outbound_citation(&residuals).cloned();
            let envelope = OutboundFrontierEnvelope {
                schema_version: "sl.contract_follow.outbound_frontier.v0_2".into(),
                parent_campaign_receipt: trajectory.display().to_string(),
                source_receipt_path: source_receipt.display().to_string(),
                source_semantic_ref,
                residual_count: residuals.len(),
                selected,
                residuals,
                selector_is_legal_truth_rank: false,
                budget: config.budget,
                accepted_hop_count: counters.accepted_hops,
                source_acquisition_count: counters.source_acquisitions,
                network_request_count: counters.network_requests,
                candidate_only: true,
                creates_legal_authority: false,
                creates_current_law_conclusion: false,
            };
            write_json(&output, &envelope)?;
            println!(
                "contract_follow_outbound_frontier={} residuals={} selected={} authority=false current_law_conclusion=false",
                output.display(),
                envelope.residual_count,
                envelope
                    .selected
                    .as_ref()
                    .map(|value| value.medium_neutral_citation.as_str())
                    .unwrap_or("none"),
            );
            Ok(())
        }
        [command, rest @ ..] if command == "acquire-next" => {
            let frontier = PathBuf::from(required_arg(rest, "--frontier")?);
            let output_dir = PathBuf::from(required_arg(rest, "--output-dir")?);
            let trajectory = PathBuf::from(required_arg(rest, "--trajectory")?);
            let requested_as_at = arg_value(rest, "--as-at");
            let envelope: OutboundFrontierEnvelope = {
                let bytes = fs::read(&frontier)
                    .map_err(|error| format!("read {}: {error}", frontier.display()))?;
                serde_json::from_slice(&bytes)
                    .map_err(|error| format!("decode {}: {error}", frontier.display()))?
            };
            if envelope.parent_campaign_receipt != trajectory.display().to_string() {
                return Err("outbound frontier parent campaign does not match --trajectory".into());
            }
            let mut campaign = resume_from_trajectory(&trajectory)?;
            if let Some(requested) = requested_as_at.as_deref() {
                if requested != campaign.config.as_at {
                    return Err(format!(
                        "recursive acquisition as-at {requested:?} differs from parent campaign {:?}; create an explicit temporal branch instead",
                        campaign.config.as_at
                    ));
                }
            }
            let as_at = campaign.config.as_at.clone();
            if campaign.config.budget != envelope.budget
                || campaign.accepted_hop_count() != envelope.accepted_hop_count
                || campaign.source_acquisitions != envelope.source_acquisition_count
                || campaign.network_requests != envelope.network_request_count
            {
                return Err("outbound frontier budget/counter snapshot no longer matches parent campaign".into());
            }
            campaign.ensure_source_acquisition_budget(
                RECURSIVE_OALC_MAX_REQUESTS_PER_ACQUISITION,
            )?;
            let selected = envelope
                .selected
                .ok_or_else(|| "outbound frontier has no selected candidate".to_string())?;
            let receipt = acquire_outbound_citation(&selected, output_dir.clone(), &as_at)?;
            campaign.record_source_acquisition(receipt.network_requests)?;
            let target_source_receipt = output_dir.join("oalc-source-receipt.json");
            let identity_worksheet =
                output_dir.join("authority-identity-review-worksheet.json");
            crate::contract_identity::prepare(
                &[target_source_receipt.clone()],
                &identity_worksheet,
            )?;
            let pending = PendingRecursiveReview {
                discovery_source_receipt: envelope.source_receipt_path.clone(),
                source_semantic_ref: envelope.source_semantic_ref.clone(),
                target_source_receipt: target_source_receipt.display().to_string(),
                target_medium_neutral_citation: selected.medium_neutral_citation.clone(),
                target_semantic_ref: None,
            };

            let mut next_campaign = campaign.receipt_json()?;
            next_campaign["parent_campaign_receipt"] =
                json!(trajectory.display().to_string());
            next_campaign["pending_recursive_review"] =
                serde_json::to_value(&pending)
                    .map_err(|error| format!("encode pending recursive review: {error}"))?;
            next_campaign["identity_review_worksheet"] =
                json!(identity_worksheet.display().to_string());
            next_campaign["next_operator_gate"] =
                serde_json::to_value(campaign_step(CampaignOperatorGate::AuthorityIdentityReview))
                    .map_err(|error| format!("encode identity operator gate: {error}"))?;
            next_campaign["continuation_only"] = json!(true);
            next_campaign["last_action"] = json!("governed_source_acquisition");
            next_campaign["last_acquired_medium_neutral_citation"] =
                json!(selected.medium_neutral_citation.clone());
            let next_campaign_path = output_dir.join("campaign-after-source-acquisition.json");
            write_json(&next_campaign_path, &next_campaign)?;

            let summary = json!({
                "schema_version": "sl.contract_follow.recursive_source_acquisition.v0_2",
                "selected_residual_ref": selected.residual_ref,
                "selected_medium_neutral_citation": selected.medium_neutral_citation,
                "source_receipt": output_dir.join("oalc-source-receipt.json"),
                "next_campaign_receipt": next_campaign_path,
                "identity_review_worksheet": identity_worksheet,
                "version_id": receipt.version_id,
                "corpus_revision_ref": receipt.corpus_revision_ref,
                "resolution_path": receipt.resolution_path,
                "network_requests": receipt.network_requests,
                "stream_rows_examined": receipt.stream_rows_examined,
                "stream_bytes_read": receipt.stream_bytes_read,
                "stream_terminated_after_match": receipt.stream_terminated_after_match,
                "stream_uniqueness_exhaustively_verified":
                    receipt.stream_uniqueness_exhaustively_verified,
                "campaign_source_acquisition_count": campaign.source_acquisitions,
                "campaign_network_request_count": campaign.network_requests,
                "campaign_budget": campaign.config.budget,
                "parent_campaign_receipt": trajectory,
                "candidate_only": true,
                "creates_legal_authority": false,
                "creates_current_law_conclusion": false,
                "missing_source_is_negative_legal_evidence": false,
            });
            let summary_path = output_dir.join("campaign-source-acquisition.json");
            write_json(&summary_path, &summary)?;
            println!(
                "contract_follow_recursive_source={} next_campaign={} citation={} network={} authority=false",
                summary_path.display(),
                next_campaign_path.display(),
                receipt.citation,
                receipt.network_requests,
            );
            Ok(())
        }
        [command, rest @ ..] if command == "identity-prepare" => {
            let receipt = PathBuf::from(required_arg(rest, "--receipt")?);
            let output = PathBuf::from(required_arg(rest, "--output")?);
            crate::contract_identity::prepare(&[receipt], &output)?;
            println!("contract_follow_identity_worksheet={}", output.display());
            Ok(())
        }
        [command, rest @ ..] if command == "identity-reviewed" => {
            let trajectory = PathBuf::from(required_arg(rest, "--trajectory")?);
            let worksheet = PathBuf::from(required_arg(rest, "--worksheet")?);