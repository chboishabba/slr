use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use sensiblaw_pg_source_store::{
    claim_parser_jobs, finalize_db_native_long_document, load_claimed_job_text,
    load_database_config, parser_run_state, persist_parser_residual,
    persist_parser_success, prepare_db_native_long_document, ParserArtifactRecord,
    ParserTokenRecord,
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

fn digest_ref(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
}

fn content_revision_ref(text: &str) -> String {
    format!("source-revision:{}", digest_ref(text.as_bytes()))
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
        .ok_or("parser stdin unavailable")?
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
    let source_revision_ref = content_revision_ref(&canonical_text);
    let description = parser_description(parser_script, model_ref)?;
    if description.parser_family != "spacy" || description.model_ref != *model_ref {
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
                    persist_parser_residual(
                        &config,
                        &job,
                        worker_ref,
                        &format!("parser-describe-error:{error}"),
                    )?;
                    residual += 1;
                    continue;
                }
            };
            if description.parser_family != job.parser_family
                || description.parser_version != job.parser_version
                || description.model_ref != job.model_ref
            {
                persist_parser_residual(
                    &config,
                    &job,
                    worker_ref,
                    &format!(
                        "parser-identity-mismatch:expected={}/{}/{}:actual={}/{}/{}",
                        job.parser_family,
                        job.parser_version,
                        job.model_ref,
                        description.parser_family,
                        description.parser_version,
                        description.model_ref
                    ),
                )?;
                residual += 1;
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
                    persist_parser_residual(
                        &config,
                        &job,
                        worker_ref,
                        &format!("parser-process-error:{error}"),
                    )?;
                    residual += 1;
                    continue;
                }
            };

            let wire: WireArtifact = match serde_json::from_slice(&output) {
                Ok(value) => value,
                Err(error) => {
                    persist_parser_residual(
                        &config,
                        &job,
                        worker_ref,
                        &format!("parser-json-error:{error}"),
                    )?;
                    residual += 1;
                    continue;
                }
            };
            if wire.parser_family != job.parser_family
                || wire.parser_version != job.parser_version
                || wire.model_ref != job.model_ref
            {
                persist_parser_residual(
                    &config,
                    &job,
                    worker_ref,
                    "parser-output-identity-mismatch",
                )?;
                residual += 1;
                continue;
            }

            let global_offset =
                u32::try_from(job.start_char).map_err(|_| "region start exceeds u32 M12 ABI")?;
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

            let output_text = String::from_utf8(output.clone())?;
            let artifact = ParserArtifactRecord {
                format_ref: "application/vnd.sensiblaw.spacy-region+json".into(),
                content_digest_ref: digest_ref(&output),
                artifact_json: Some(output_text),
                object_locator: None,
            };
            persist_parser_success(
                &config,
                &job,
                worker_ref,
                &tokens,
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

fn usage() {
    eprintln!(
        "usage:\n  \
         scale1_long_document prepare-spacy <text-file> <source-ref> <provider-ref> <acquisition-receipt-ref> <model-ref> [config-json] [parser-script]\n  \
         scale1_long_document status <parser-run-ref>\n  \
         scale1_long_document worker <parser-run-ref> <worker-ref> [batch-size] [parser-script]\n  \
         scale1_long_document finalize <parser-run-ref>"
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
        "status" => status(&args),
        "worker" => worker(&args),
        "finalize" => finalize(&args),
        _ => {
            usage();
            Err(format!("unknown command: {command}").into())
        }
    }
}
