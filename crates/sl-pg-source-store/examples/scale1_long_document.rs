use std::error::Error;
use std::fs;
use std::io::{Error as IoError, ErrorKind, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use sensiblaw_pg_source_store::{
    canonical_generic_source_revision_ref, claim_parser_jobs,
    defer_parser_job_retry, finalize_db_native_long_document,
    load_claimed_job_text,
    load_database_config, parser_run_state, persist_parser_residual,
    persist_parser_success_with_entities, prepare_db_native_long_document,
    ParserArtifactRecord, ParserEntityRecord, ParserTokenRecord,
    ReviewAction, ReviewCommand, ReviewStatus, apply_persisted_review_command,
};

#[derive(Debug, Deserialize)]
struct ParserDescription {
    parser_family: String,
    parser_version: String,
    model_ref: String,
}

#[derive(Debug, Deserialize)]
struct WireArtifact {
    parser_family: String,
    parser_version: String,
    model_ref: String,
    tokens: Vec<WireToken>,
    #[serde(default)]
    entities: Vec<WireEntity>,
}

#[derive(Debug, Deserialize)]
struct WireToken {
    token_ordinal: u32,
    start_char: u32,
    end_char: u32,
    surface: String,
    lemma: String,
    pos: String,
    morph: Value,
    head_ordinal: Option<u32>,
    dependency_ref: String,
}

#[derive(Debug, Deserialize)]
struct WireEntity {
    start_char: u32,
    end_char: u32,
    text: String,
    label: String,
}

#[derive(Debug, Deserialize)]
struct GwbProjectionManifest {
    schema_version: String,
    authority: String,
    profile_ref: String,
    documents: Vec<GwbProjectionDocument>,
}

#[derive(Debug, Deserialize)]
struct GwbProjectionDocument {
    document_ordinal: u64,
    source_path: String,
    source_sha256: String,
    source_bytes: u64,
    projector: String,
    projected_path: String,
    projected_sha256: String,
    projected_bytes: u64,
}

fn digest_ref(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
}

fn parser_description(
    script: &str,
    model_ref: &str,
) -> Result<ParserDescription, Box<dyn Error>> {
    let output = Command::new("python3")
        .arg(script)
        .arg("--model")
        .arg(model_ref)
        .arg("--describe")
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "parser describe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn run_parser(
    script: &str,
    model_ref: &str,
    config_json: &str,
    text: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut child = Command::new("python3")
        .arg(script)
        .arg("--model")
        .arg(model_ref)
        .arg("--config-json")
        .arg(config_json)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| IoError::new(ErrorKind::BrokenPipe, "parser stdin unavailable"))?
        .write_all(text.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!(
            "parser process failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output.stdout)
}

fn prepare_spacy(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 7 {
        return Err(
            "prepare-spacy <text-file> <source-ref> <provider-ref> <acquisition-receipt-ref> <model-ref> [config-json] [parser-script]"
                .into(),
        );
    }
    let text_file = &args[2];
    let source_ref = &args[3];
    let provider_ref = &args[4];
    let acquisition_receipt_ref = &args[5];
    let model_ref = &args[6];
    let config_json = args.get(7).map(String::as_str).unwrap_or("{}");
    let parser_script = args
        .get(8)
        .map(String::as_str)
        .unwrap_or("scripts/scale1_spacy_json_parser.py");

    let canonical_text = fs::read_to_string(text_file)?;
    let content_digest_ref = digest_ref(canonical_text.as_bytes());
    let source_revision_ref = canonical_generic_source_revision_ref(
        source_ref,
        provider_ref,
        acquisition_receipt_ref,
        &content_digest_ref,
        "text/plain",
    );
    let description = parser_description(parser_script, model_ref)?;
    if description.parser_family != "spacy" || description.model_ref != model_ref.as_str() {
        return Err("spaCy parser description did not match requested model".into());
    }

    let config = load_database_config(None)?;
    let prepared = prepare_db_native_long_document(
        &config,
        source_ref,
        &source_revision_ref,
        provider_ref,
        acquisition_receipt_ref,
        Path::new(text_file)
            .file_name()
            .map(|value| value.to_string_lossy().into_owned()),
        None,
        &canonical_text,
        &description.parser_family,
        &description.parser_version,
        model_ref,
        config_json,
    )?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "sensiblaw.scale1.long-document-prepare.v0_1",
            "source_ref": prepared.source.source_ref,
            "source_revision_ref": prepared.source.source_revision_ref,
            "content_digest_ref": prepared.source.content_digest_ref,
            "canonical_ref": prepared.source.canonical_ref,
            "document_ref": prepared.source.document_ref,
            "parser_run_ref": prepared.parser_run.parser_run_ref,
            "parser_family": prepared.parser_run.parser_family,
            "parser_version": prepared.parser_run.parser_version,
            "model_ref": prepared.parser_run.model_ref,
            "config_digest_ref": prepared.parser_run.config_digest_ref,
            "total_regions": prepared.structural.structural_region_count,
            "semantic_regions": prepared.semantic_region_count,
            "structural_regions": prepared.structural_region_count,
            "new_jobs": prepared.newly_enqueued_job_count,
            "reused_jobs": prepared.reused_existing_job_count,
            "candidate_only": prepared.candidate_only,
            "creates_semantic_authority": prepared.creates_semantic_authority,
            "applicability_promoted": prepared.applicability_promoted,
            "claim_truth_promoted": prepared.claim_truth_promoted
        }))?
    );
    Ok(())
}



fn prepare_stdin(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 7 {
        return Err(
            "prepare-stdin <source-ref> <provider-ref> <acquisition-receipt-ref> <title> <model-ref> [config-json] [parser-script]"
                .into(),
        );
    }
    let source_ref = &args[2];
    let provider_ref = &args[3];
    let acquisition_receipt_ref = &args[4];
    let title = &args[5];
    let model_ref = &args[6];
    let config_json = args.get(7).map(String::as_str).unwrap_or("{}");
    let parser_script = args
        .get(8)
        .map(String::as_str)
        .unwrap_or("scripts/scale1_spacy_json_parser.py");

    let mut canonical_text = String::new();
    std::io::stdin().read_to_string(&mut canonical_text)?;
    if canonical_text.is_empty() {
        return Err("prepare-stdin received empty canonical text".into());
    }

    let content_digest_ref = digest_ref(canonical_text.as_bytes());
    let source_revision_ref = canonical_generic_source_revision_ref(
        source_ref,
        provider_ref,
        acquisition_receipt_ref,
        &content_digest_ref,
        "text/plain",
    );
    let description = parser_description(parser_script, model_ref)?;
    if description.parser_family != "spacy" || description.model_ref != model_ref.as_str() {
        return Err("spaCy parser description did not match requested model".into());
    }

    let config = load_database_config(None)?;
    let prepared = prepare_db_native_long_document(
        &config,
        source_ref,
        &source_revision_ref,
        provider_ref,
        acquisition_receipt_ref,
        (!title.is_empty()).then(|| title.clone()),
        None,
        &canonical_text,
        &description.parser_family,
        &description.parser_version,
        model_ref,
        config_json,
    )?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "sensiblaw.scale1.long-document-prepare.v0_1",
            "input_transport": "stdin",
            "source_ref": prepared.source.source_ref,
            "source_revision_ref": prepared.source.source_revision_ref,
            "content_digest_ref": prepared.source.content_digest_ref,
            "canonical_ref": prepared.source.canonical_ref,
            "document_ref": prepared.source.document_ref,
            "parser_run_ref": prepared.parser_run.parser_run_ref,
            "parser_family": prepared.parser_run.parser_family,
            "parser_version": prepared.parser_run.parser_version,
            "model_ref": prepared.parser_run.model_ref,
            "semantic_regions": prepared.semantic_region_count,
            "structural_regions": prepared.structural_region_count,
            "new_jobs": prepared.newly_enqueued_job_count,
            "reused_jobs": prepared.reused_existing_job_count,
            "candidate_only": prepared.candidate_only,
            "creates_semantic_authority": prepared.creates_semantic_authority,
            "applicability_promoted": prepared.applicability_promoted,
            "claim_truth_promoted": prepared.claim_truth_promoted
        }))?
    );
    Ok(())
}

fn prepare_gwb_projection(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 5 {
        return Err(
            "prepare-gwb <projection-manifest> <document-ordinal> <model-ref> [config-json] [parser-script]"
                .into(),
        );
    }
    let manifest_path = &args[2];
    let document_ordinal = args[3].parse::<u64>()?;
    let model_ref = &args[4];
    let config_json = args.get(5).map(String::as_str).unwrap_or("{}");
    let parser_script = args
        .get(6)
        .map(String::as_str)
        .unwrap_or("scripts/scale1_spacy_json_parser.py");

    let manifest_bytes = fs::read(manifest_path)?;
    let manifest: GwbProjectionManifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest.authority != "source_projection_only"
        || manifest.profile_ref != "tranche-profile:gwb:v0_1"
        || !manifest.schema_version.starts_with("sensiblaw.gwb")
    {
        return Err("not a canonical GWB source-projection manifest".into());
    }
    let document = manifest
        .documents
        .iter()
        .find(|document| document.document_ordinal == document_ordinal)
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "GWB document ordinal not found"))?;

    let canonical_text = fs::read_to_string(&document.projected_path)?;
    let projected_digest = digest_ref(canonical_text.as_bytes());
    let expected_projected_digest = format!("sha256:{}", document.projected_sha256);
    if projected_digest != expected_projected_digest
        || canonical_text.as_bytes().len() as u64 != document.projected_bytes
    {
        return Err("GWB projected text digest/byte count mismatch".into());
    }

    let manifest_digest = digest_ref(&manifest_bytes);
    let source_ref = format!("source:gwb:raw-sha256:{}", document.source_sha256);
    let provider_ref = format!("gwb-source-projection:{}", document.projector);
    let acquisition_receipt_ref = format!(
        "gwb-projection:{}:document:{}:raw-sha256:{}:raw-bytes:{}",
        manifest_digest,
        document.document_ordinal,
        document.source_sha256,
        document.source_bytes
    );
    let source_revision_ref = canonical_generic_source_revision_ref(
        &source_ref,
        &provider_ref,
        &acquisition_receipt_ref,
        &projected_digest,
        "text/plain",
    );

    let description = parser_description(parser_script, model_ref)?;
    if description.parser_family != "spacy" || description.model_ref != model_ref.as_str() {
        return Err("spaCy parser description did not match requested model".into());
    }

    let config = load_database_config(None)?;
    let prepared = prepare_db_native_long_document(
        &config,
        &source_ref,
        &source_revision_ref,
        &provider_ref,
        &acquisition_receipt_ref,
        Path::new(&document.source_path)
            .file_name()
            .map(|value| value.to_string_lossy().into_owned()),
        None,
        &canonical_text,
        &description.parser_family,
        &description.parser_version,
        model_ref,
        config_json,
    )?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "sensiblaw.scale1.gwb-book-prepare.v0_1",
            "gwb_projection_manifest": manifest_path,
            "gwb_projection_manifest_digest": manifest_digest,
            "gwb_document_ordinal": document.document_ordinal,
            "raw_source_path": document.source_path,
            "raw_source_sha256": document.source_sha256,
            "raw_source_bytes": document.source_bytes,
            "projector": document.projector,
            "projected_sha256": document.projected_sha256,
            "projected_bytes": document.projected_bytes,
            "source_ref": prepared.source.source_ref,
            "source_revision_ref": prepared.source.source_revision_ref,
            "canonical_ref": prepared.source.canonical_ref,
            "document_ref": prepared.source.document_ref,
            "parser_run_ref": prepared.parser_run.parser_run_ref,
            "parser_family": prepared.parser_run.parser_family,
            "parser_version": prepared.parser_run.parser_version,
            "model_ref": prepared.parser_run.model_ref,
            "semantic_regions": prepared.semantic_region_count,
            "structural_regions": prepared.structural_region_count,
            "new_jobs": prepared.newly_enqueued_job_count,
            "reused_jobs": prepared.reused_existing_job_count,
            "candidate_only": prepared.candidate_only,
            "creates_semantic_authority": prepared.creates_semantic_authority,
            "claim_truth_promoted": prepared.claim_truth_promoted
        }))?
    );
    Ok(())
}

fn status(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 {
        return Err("status <parser-run-ref>".into());
    }
    let config = load_database_config(None)?;
    let state = parser_run_state(&config, &args[2])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "parser_run_ref": state.parser_run_ref,
            "queued": state.queued,
            "leased": state.leased,
            "succeeded": state.succeeded,
            "residual": state.residual,
            "unattempted_semantic_regions": state.unattempted_semantic_regions,
            "complete": state.complete
        }))?
    );
    Ok(())
}

fn worker(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 4 {
        return Err(
            "worker <parser-run-ref> <worker-ref> [batch-size] [parser-script]".into(),
        );
    }
    let parser_run_ref = &args[2];
    let worker_ref = &args[3];
    let batch_size = args
        .get(4)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(32);
    let parser_script = args
        .get(5)
        .map(String::as_str)
        .unwrap_or("scripts/scale1_spacy_json_parser.py");

    let config = load_database_config(None)?;
    let mut succeeded = 0usize;
    let mut residual = 0usize;
    let mut deferred_retry = 0usize;

    loop {
        let jobs = claim_parser_jobs(
            &config,
            parser_run_ref,
            worker_ref,
            batch_size,
        )?;
        if jobs.is_empty() {
            break;
        }

        for job in jobs {
            if job.parser_family != "spacy" {
                persist_parser_residual(
                    &config,
                    &job,
                    worker_ref,
                    &format!("unsupported-parser-family:{}", job.parser_family),
                )?;
                residual += 1;
                continue;
            }

            let description = match parser_description(parser_script, &job.model_ref) {
                Ok(value) => value,
                Err(error) => {
                    defer_parser_job_retry(
                        &config,
                        &job,
                        worker_ref,
                        &format!("parser-describe-error:{error}"),
                    )?;
                    deferred_retry += 1;
                    continue;
                }
            };
            if description.parser_family != job.parser_family
                || description.parser_version != job.parser_version
                || description.model_ref != job.model_ref
            {
                defer_parser_job_retry(
                    &config,
                    &job,
                    worker_ref,
                    &format!(
                        "worker-parser-identity-mismatch:expected={}/{}/{}:actual={}/{}/{}",
                        job.parser_family,
                        job.parser_version,
                        job.model_ref,
                        description.parser_family,
                        description.parser_version,
                        description.model_ref
                    ),
                )?;
                deferred_retry += 1;
                continue;
            }

            let region_text = load_claimed_job_text(&config, &job)?;
            let output = match run_parser(
                parser_script,
                &job.model_ref,
                &job.config_json,
                &region_text,
            ) {
                Ok(value) => value,
                Err(error) => {
                    defer_parser_job_retry(
                        &config,
                        &job,
                        worker_ref,
                        &format!("parser-process-error:{error}"),
                    )?;
                    deferred_retry += 1;
                    continue;
                }
            };

            let wire: WireArtifact = match serde_json::from_slice(&output) {
                Ok(value) => value,
                Err(error) => {
                    defer_parser_job_retry(
                        &config,
                        &job,
                        worker_ref,
                        &format!("parser-json-error:{error}"),
                    )?;
                    deferred_retry += 1;
                    continue;
                }
            };
            if wire.parser_family != job.parser_family
                || wire.parser_version != job.parser_version
                || wire.model_ref != job.model_ref
            {
                defer_parser_job_retry(
                    &config,
                    &job,
                    worker_ref,
                    "parser-output-identity-mismatch",
                )?;
                deferred_retry += 1;
                continue;
            }

            let global_offset = u32::try_from(job.start_char).map_err(|_| {
                IoError::new(ErrorKind::InvalidData, "region start exceeds u32 M12 ABI")
            })?;
            let mut tokens = Vec::with_capacity(wire.tokens.len());
            let mut overflow = false;
            for token in wire.tokens {
                let Some(start_char) = global_offset.checked_add(token.start_char) else {
                    overflow = true;
                    break;
                };
                let Some(end_char) = global_offset.checked_add(token.end_char) else {
                    overflow = true;
                    break;
                };
                tokens.push(ParserTokenRecord {
                    token_ordinal: token.token_ordinal,
                    start_char,
                    end_char,
                    surface: token.surface,
                    lemma: token.lemma,
                    pos: token.pos,
                    morph_json: Some(serde_json::to_string(&token.morph)?),
                    head_ordinal: token.head_ordinal,
                    dependency_ref: token.dependency_ref,
                });
            }
            if overflow {
                persist_parser_residual(
                    &config,
                    &job,
                    worker_ref,
                    "parser-offset-overflow",
                )?;
                residual += 1;
                continue;
            }

            let mut entities = Vec::with_capacity(wire.entities.len());
            let mut entity_overflow = false;
            for (entity_ordinal, entity) in wire.entities.into_iter().enumerate() {
                let Some(start_char) = global_offset.checked_add(entity.start_char) else {
                    entity_overflow = true;
                    break;
                };
                let Some(end_char) = global_offset.checked_add(entity.end_char) else {
                    entity_overflow = true;
                    break;
                };
                entities.push(ParserEntityRecord {
                    entity_ordinal: entity_ordinal as u32,
                    start_char,
                    end_char,
                    surface: entity.text,
                    label_ref: entity.label,
                });
            }
            if entity_overflow {
                persist_parser_residual(
                    &config,
                    &job,
                    worker_ref,
                    "parser-entity-offset-overflow",
                )?;
                residual += 1;
                continue;
            }

            let output_text = String::from_utf8(output.clone())?;
            let artifact = ParserArtifactRecord {
                format_ref: "application/vnd.sensiblaw.spacy-region+json".into(),
                content_digest_ref: digest_ref(&output),
                artifact_json: Some(output_text),
                object_locator: None,
            };
            persist_parser_success_with_entities(
                &config,
                &job,
                worker_ref,
                &tokens,
                &entities,
                Some(&artifact),
            )?;
            succeeded += 1;
        }
    }

    let state = parser_run_state(&config, parser_run_ref)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "sensiblaw.scale1.worker-receipt.v0_1",
            "parser_run_ref": parser_run_ref,
            "worker_ref": worker_ref,
            "succeeded_this_worker": succeeded,
            "residual_this_worker": residual,
            "deferred_retry_this_worker": deferred_retry,
            "queued": state.queued,
            "leased": state.leased,
            "succeeded_total": state.succeeded,
            "residual_total": state.residual,
            "unattempted_semantic_regions": state.unattempted_semantic_regions,
            "complete": state.complete
        }))?
    );
    Ok(())
}

fn finalize(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 {
        return Err("finalize <parser-run-ref>".into());
    }
    let config = load_database_config(None)?;
    let receipt = finalize_db_native_long_document(&config, &args[2])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "sensiblaw.scale1.long-document-acceptance.v0_1",
            "source_ref": receipt.source.source_ref,
            "source_revision_ref": receipt.source.source_revision_ref,
            "content_digest_ref": receipt.source.content_digest_ref,
            "parser_run_ref": receipt.parser_run_ref,
            "total_structural_regions": receipt.total_structural_regions,
            "semantic_eligible_regions": receipt.semantic_eligible_regions,
            "parser_success_regions": receipt.parser_success_regions,
            "parser_residual_regions": receipt.parser_residual_regions,
            "unattempted_semantic_regions": receipt.unattempted_semantic_regions,
            "compiled_statement_count": receipt.compiled_statement_count,
            "candidate_pnf_count": receipt.candidate_pnf_count,
            "persisted_statement_count": receipt.persisted_statement_count,
            "persisted_candidate_batch_count": receipt.persisted_candidate_batch_count,
            "persisted_candidate_factor_count": receipt.persisted_candidate_factor_count,
            "candidate_pnf_reopen_complete": receipt.candidate_pnf_reopen_complete,
            "entity_mention_candidates": receipt.reconciliation.entity_mention_count,
            "entity_fingerprint_candidates": receipt.reconciliation.entity_fingerprint_count,
            "named_entity_mention_candidates": receipt.reconciliation.named_entity_mention_count,
            "named_entity_fingerprint_candidates": receipt.reconciliation.named_entity_fingerprint_count,
            "temporal_mention_candidates": receipt.reconciliation.temporal_mention_count,
            "proposition_occurrence_candidates": receipt.reconciliation.proposition_occurrence_count,
            "proposition_fingerprint_candidates": receipt.reconciliation.proposition_fingerprint_count,
            "event_occurrence_candidates": receipt.reconciliation.event_occurrence_count,
            "event_fingerprint_candidates": receipt.reconciliation.event_fingerprint_count,
            "polarity_conflict_candidates": receipt.reconciliation.polarity_conflict_candidate_count,
            "review_pressure_candidates": receipt.reconciliation.review_pressure_candidate_count,
            "reconciliation_review_items": receipt.reconciliation_review.review_item_refs.len(),
            "reconciliation_cluster_review_items": receipt.reconciliation_review.cluster_review_items,
            "reconciliation_contestation_review_items": receipt.reconciliation_review.contestation_review_items,
            "reconciliation_review_creates_event_identity": receipt.reconciliation_review.creates_event_identity,
            "reconciliation_review_creates_semantic_authority": receipt.reconciliation_review.creates_semantic_authority,
            "reconciliation_review_claim_truth_promoted": receipt.reconciliation_review.claim_truth_promoted,
            "reconciliation_creates_entity_identity": receipt.reconciliation.creates_entity_identity,
            "reconciliation_creates_proposition_identity": receipt.reconciliation.creates_proposition_identity,
            "reconciliation_creates_event_identity": receipt.reconciliation.creates_event_identity,
            "structural_only_regions": receipt.structural_only_regions,
            "source_region_loss_count": receipt.source_region_loss_count,
            "canonical_bytes_reload_identically": receipt.canonical_bytes_reload_identically,
            "every_region_reloaded": receipt.every_region_reloaded,
            "candidate_only": receipt.candidate_only,
            "creates_semantic_authority": receipt.creates_semantic_authority,
            "applicability_promoted": receipt.applicability_promoted,
            "claim_truth_promoted": receipt.claim_truth_promoted
        }))?
    );
    Ok(())
}



fn parse_review_action(value: &str) -> Result<ReviewAction, Box<dyn Error>> {
    Ok(match value {
        "accept" => ReviewAction::Accept,
        "reject" => ReviewAction::Reject,
        "abstain" => ReviewAction::Abstain,
        "qualify" => ReviewAction::Qualify,
        "request-evidence" => ReviewAction::RequestEvidence,
        "open-source" => ReviewAction::OpenSource,
        _ => return Err(format!("unsupported review action: {value}").into()),
    })
}

fn review_status_ref(value: ReviewStatus) -> &'static str {
    match value {
        ReviewStatus::Pending => "pending",
        ReviewStatus::Accepted => "accepted",
        ReviewStatus::Rejected => "rejected",
        ReviewStatus::Abstained => "abstained",
        ReviewStatus::Qualified => "qualified",
        ReviewStatus::Superseded => "superseded",
        ReviewStatus::NeedsEvidence => "needs_evidence",
    }
}

fn apply_review(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 5 {
        return Err(
            "review <review-item-ref> <action> <reviewer-ref> [qualification-ref] [evidence-request-ref]"
                .into(),
        );
    }
    let review_item_ref = &args[2];
    let action = parse_review_action(&args[3])?;
    let reviewer_ref = &args[4];
    let qualification_ref = args.get(5).filter(|value| !value.is_empty()).cloned();
    let evidence_request_ref = args.get(6).filter(|value| !value.is_empty()).cloned();

    let action_ref = match action {
        ReviewAction::Accept => "accept",
        ReviewAction::Reject => "reject",
        ReviewAction::Abstain => "abstain",
        ReviewAction::Qualify => "qualify",
        ReviewAction::Supersede => "supersede",
        ReviewAction::RequestEvidence => "request-evidence",
        ReviewAction::OpenSource => "open-source",
        ReviewAction::FollowAuthority => "follow-authority",
    };
    let command_ref = format!(
        "review-command:scale1:{}",
        digest_ref(
            format!(
                "{review_item_ref}\u{1f}{action_ref}\u{1f}{reviewer_ref}\u{1f}{}\u{1f}{}",
                qualification_ref.as_deref().unwrap_or(""),
                evidence_request_ref.as_deref().unwrap_or("")
            )
            .as_bytes(),
        )
    );

    let command = ReviewCommand {
        command_ref: command_ref.clone(),
        review_item_ref: review_item_ref.clone(),
        action,
        reviewer_ref: reviewer_ref.clone(),
        qualification_ref,
        evidence_request_ref,
    };

    let config = load_database_config(None)?;
    let (receipt, item) = apply_persisted_review_command(&config, &command)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "sensiblaw.scale1.review-command.v0_1",
            "command_ref": receipt.command_ref,
            "review_item_ref": receipt.review_item_ref,
            "semantic_ref": receipt.semantic_ref,
            "reviewer_ref": receipt.reviewer_ref,
            "action": action_ref,
            "current_status": review_status_ref(item.current_status),
            "candidate_only": receipt.candidate_only,
            "creates_semantic_authority": receipt.creates_semantic_authority,
            "applicability_promoted": receipt.applicability_promoted,
            "claim_truth_promoted": receipt.claim_truth_promoted
        }))?
    );
    Ok(())
}

fn materialize_proposition(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 {
        return Err(
            "materialize-proposition <review-item-ref> <accepted-review-command-ref>"
                .into(),
        );
    }
    let config = load_database_config(None)?;
    let receipt = materialize_accepted_reconciliation_proposition(
        &config,
        &args[2],
        &args[3],
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "sensiblaw.scale1.reviewed-proposition-materialization.v0_1",
            "materialization_ref": receipt.materialization_ref,
            "proposition_ref": receipt.proposition_ref,
            "proposition_fingerprint_ref": receipt.proposition_fingerprint_ref,
            "source_revision_ref": receipt.source_revision_ref,
            "review_item_ref": receipt.review_item_ref,
            "accepted_review_command_ref": receipt.accepted_review_command_ref,
            "claim_refs": receipt.claim_refs,
            "reviewed_grouping_identity": receipt.reviewed_grouping_identity,
            "claim_review_state_unreviewed": receipt.claim_review_state_unreviewed,
            "grouping_review_is_claim_review": receipt.grouping_review_is_claim_review,
            "creates_semantic_authority": receipt.creates_semantic_authority,
            "applicability_promoted": receipt.applicability_promoted,
            "claim_truth_promoted": receipt.claim_truth_promoted
        }))?
    );
    Ok(())
}

fn usage() {
    eprintln!(
        "usage:\n  \
         scale1_long_document prepare-spacy <text-file> <source-ref> <provider-ref> <acquisition-receipt-ref> <model-ref> [config-json] [parser-script]\n  \
         scale1_long_document prepare-gwb <projection-manifest> <document-ordinal> <model-ref> [config-json] [parser-script]\n  \
         scale1_long_document prepare-stdin <source-ref> <provider-ref> <acquisition-receipt-ref> <title> <model-ref> [config-json] [parser-script]\n  \
         scale1_long_document status <parser-run-ref>\n  \
         scale1_long_document worker <parser-run-ref> <worker-ref> [batch-size] [parser-script]\n  \
         scale1_long_document finalize <parser-run-ref>\n  \
         scale1_long_document review <review-item-ref> <action> <reviewer-ref> [qualification-ref] [evidence-request-ref]\n  \
         scale1_long_document materialize-proposition <review-item-ref> <accepted-review-command-ref>"
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let Some(command) = args.get(1).map(String::as_str) else {
        usage();
        return Ok(());
    };

    match command {
        "prepare-spacy" => prepare_spacy(&args),
        "prepare-gwb" => prepare_gwb_projection(&args),
        "prepare-stdin" => prepare_stdin(&args),
        "status" => status(&args),
        "worker" => worker(&args),
        "finalize" => finalize(&args),
        "review" => apply_review(&args),
        "materialize-proposition" => materialize_proposition(&args),
        _ => {
            usage();
            Err(format!("unknown command: {command}").into())
        }
    }
}
