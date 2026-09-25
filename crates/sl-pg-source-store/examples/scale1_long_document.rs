#![recursion_limit = "256"]

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io::{Error as IoError, ErrorKind, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sensiblaw_core::source_ingest::SourceFamily;

use sensiblaw_pg_source_store::{
    canonical_generic_source_revision_ref, claim_parser_jobs,
    defer_parser_job_retry, finalize_db_native_long_document,
    load_claimed_job_text,
    load_database_config, materialize_accepted_event_join,
    materialize_accepted_reconciliation_proposition, parser_run_state,
    persist_parser_residual,
    persist_parser_success_with_entities, prepare_db_native_long_document,
    prepare_db_native_long_source,
    DbNativeParserWriter, ParserArtifactRecord, ParserEntityRecord, ParserTokenRecord,
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
struct WireBatchArtifact {
    request_ref: String,
    artifact: Value,
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
    source_kind: String,
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

fn parser_python_bin() -> String {
    std::env::var("PYTHON_BIN").unwrap_or_else(|_| "python3".to_owned())
}

fn parser_description(
    script: &str,
    model_ref: &str,
) -> Result<ParserDescription, Box<dyn Error>> {
    let output = Command::new(parser_python_bin())
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

fn run_parser_batch(
    script: &str,
    model_ref: &str,
    config_json: &str,
    requests: &[(String, String)],
) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    let mut child = Command::new(parser_python_bin())
        .arg(script)
        .arg("--model")
        .arg(model_ref)
        .arg("--config-json")
        .arg(config_json)
        .arg("--batch-jsonl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| IoError::new(ErrorKind::BrokenPipe, "parser stdin unavailable"))?;
    for (request_ref, text) in requests {
        serde_json::to_writer(&mut stdin, &json!({
            "request_ref": request_ref,
            "text": text,
        }))?;
        stdin.write_all(b"\n")?;
    }
    drop(stdin);

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!(
            "batched parser process failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let mut artifacts = BTreeMap::new();
    for line in output.stdout.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let artifact: WireBatchArtifact = serde_json::from_slice(line)?;
        if artifacts
            .insert(artifact.request_ref.clone(), serde_json::to_vec(&artifact.artifact)?)
            .is_some()
        {
            return Err(format!("batched parser emitted duplicate request {}", artifact.request_ref).into());
        }
    }
    if artifacts.len() != requests.len()
        || requests
            .iter()
            .any(|(request_ref, _)| !artifacts.contains_key(request_ref))
    {
        return Err(format!(
            "batched parser response mismatch: requested={} returned={}",
            requests.len(),
            artifacts.len()
        )
        .into());
    }
    Ok(artifacts)
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




fn source_family_ref(family: SourceFamily) -> &'static str {
    match family {
        SourceFamily::Document => "document",
        SourceFamily::Mail => "mail",
        SourceFamily::Chat => "chat",
        SourceFamily::SocialMessage => "social_message",
        SourceFamily::Transcript => "transcript",
        SourceFamily::Audio => "audio",
        SourceFamily::ImageOcr => "image_ocr",
        SourceFamily::Web => "web",
        SourceFamily::Wiki => "wiki",
        SourceFamily::LegalAuthority => "legal_authority",
        SourceFamily::NoteResearch => "note_research",
        SourceFamily::FieldCapture => "field_capture",
        SourceFamily::Calendar => "calendar",
        SourceFamily::FinancialRecord => "financial_record",
        SourceFamily::StructuredDataset => "structured_dataset",
        SourceFamily::MachineArtifact => "machine_artifact",
    }
}

fn parse_source_family(value: &str) -> Result<SourceFamily, Box<dyn Error>> {
    Ok(match value {
        "document" => SourceFamily::Document,
        "web" => SourceFamily::Web,
        "wiki" => SourceFamily::Wiki,
        "transcript" => SourceFamily::Transcript,
        "note_research" => SourceFamily::NoteResearch,
        "legal_authority" => SourceFamily::LegalAuthority,
        "image_ocr" => SourceFamily::ImageOcr,
        "mail" => SourceFamily::Mail,
        "chat" => SourceFamily::Chat,
        "social_message" => SourceFamily::SocialMessage,
        "audio" => SourceFamily::Audio,
        "field_capture" => SourceFamily::FieldCapture,
        "calendar" => SourceFamily::Calendar,
        "financial_record" => SourceFamily::FinancialRecord,
        "structured_dataset" => SourceFamily::StructuredDataset,
        "machine_artifact" => SourceFamily::MachineArtifact,
        _ => return Err(format!("unknown source family: {value}").into()),
    })
}

#[allow(clippy::too_many_arguments)]
fn prepare_stdin_values(
    source_ref: &str,
    provider_ref: &str,
    acquisition_receipt_ref: &str,
    title: &str,
    model_ref: &str,
    source_family: SourceFamily,
    config_json: &str,
    parser_script: &str,
) -> Result<(), Box<dyn Error>> {
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
    if description.parser_family != "spacy" || description.model_ref != model_ref {
        return Err("spaCy parser description did not match requested model".into());
    }

    let config = load_database_config(None)?;
    let prepared = prepare_db_native_long_source(
        &config,
        source_ref,
        &source_revision_ref,
        provider_ref,
        acquisition_receipt_ref,
        source_family,
        (!title.is_empty()).then(|| title.to_owned()),
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
            "source_family": source_family_ref(source_family),
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

fn prepare_stdin(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 7 {
        return Err(
            "prepare-stdin <source-ref> <provider-ref> <acquisition-receipt-ref> <title> <model-ref> [config-json] [parser-script]"
                .into(),
        );
    }
    prepare_stdin_values(
        &args[2],
        &args[3],
        &args[4],
        &args[5],
        &args[6],
        SourceFamily::Document,
        args.get(7).map(String::as_str).unwrap_or("{}"),
        args.get(8)
            .map(String::as_str)
            .unwrap_or("scripts/scale1_spacy_json_parser.py"),
    )
}

fn prepare_stdin_family(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 8 {
        return Err(
            "prepare-stdin-family <source-family> <source-ref> <provider-ref> <acquisition-receipt-ref> <title> <model-ref> [config-json] [parser-script]"
                .into(),
        );
    }
    let source_family = parse_source_family(&args[2])?;
    prepare_stdin_values(
        &args[3],
        &args[4],
        &args[5],
        &args[6],
        &args[7],
        source_family,
        args.get(8).map(String::as_str).unwrap_or("{}"),
        args.get(9)
            .map(String::as_str)
            .unwrap_or("scripts/scale1_spacy_json_parser.py"),
    )
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

    let source_family = match document.source_kind.as_str() {
        "public_biography_html" => SourceFamily::Web,
        "book" => SourceFamily::Document,
        other => return Err(format!("unsupported GWB source kind: {other}").into()),
    };

    let config = load_database_config(None)?;
    let prepared = prepare_db_native_long_source(
        &config,
        &source_ref,
        &source_revision_ref,
        &provider_ref,
        &acquisition_receipt_ref,
        source_family,
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
            "source_kind": document.source_kind,
            "source_family": source_family_ref(source_family),
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

#[derive(Debug)]
struct LocalWorkerReceipt {
    succeeded: usize,
    residual: usize,
    deferred_retry: usize,
    token_count: usize,
    entity_count: usize,
    parser_process_ns: u128,
    parser_persist_ns: u128,
    parser_job_ns: Vec<u128>,
}

fn percentile_ns(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() - 1) * percentile + 99) / 100;
    sorted[index.min(sorted.len() - 1)]
}

fn concentration_ratio(values: &[u128], top_n: usize) -> f64 {
    let total: u128 = values.iter().copied().sum();
    if total == 0 {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    let top: u128 = sorted.into_iter().take(top_n).sum();
    top as f64 / total as f64
}

fn drain_local_worker(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
    parser_run_ref: &str,
    worker_ref: &str,
    batch_size: usize,
    parser_script: &str,
) -> Result<LocalWorkerReceipt, Box<dyn Error>> {
    let mut succeeded = 0usize;
    let mut residual = 0usize;
    let mut deferred_retry = 0usize;
    let mut token_count = 0usize;
    let mut entity_count = 0usize;
    let mut parser_process_ns = 0u128;
    let mut parser_persist_ns = 0u128;
    let mut parser_job_ns = Vec::new();
    let mut parser_writer = DbNativeParserWriter::connect(config)?;

    loop {
        let jobs = claim_parser_jobs(
            config,
            parser_run_ref,
            worker_ref,
            batch_size,
        )?;
        if jobs.is_empty() {
            break;
        }

        // Parser jobs in a run share an identity/configuration, but retain the
        // per-region lease and persistence boundary.  Feed each homogeneous
        // lease batch through one spaCy process so model initialisation is not
        // paid once per sentence.
        let mut requests_by_identity: BTreeMap<
            (String, String, String, String),
            Vec<(String, String)>,
        > = BTreeMap::new();
        for job in &jobs {
            if job.parser_family == "spacy" {
                let region_text = load_claimed_job_text(config, job)?;
                requests_by_identity
                    .entry((
                        job.parser_family.clone(),
                        job.parser_version.clone(),
                        job.model_ref.clone(),
                        job.config_json.clone(),
                    ))
                    .or_default()
                    .push((job.compilation_key.clone(), region_text));
            }
        }

        let mut batched_outputs: BTreeMap<String, Result<Vec<u8>, String>> = BTreeMap::new();
        for ((parser_family, parser_version, model_ref, config_json), requests) in requests_by_identity {
            let description = parser_description(parser_script, &model_ref);
            let identity_error = match description {
                Ok(description)
                    if description.parser_family == parser_family
                        && description.parser_version == parser_version
                        && description.model_ref == model_ref => None,
                Ok(description) => Some(format!(
                    "worker-parser-identity-mismatch:expected={}/{}/{}:actual={}/{}/{}",
                    parser_family,
                    parser_version,
                    model_ref,
                    description.parser_family,
                    description.parser_version,
                    description.model_ref
                )),
                Err(error) => Some(format!("parser-describe-error:{error}")),
            };
            if let Some(error) = identity_error {
                for (request_ref, _) in requests {
                    batched_outputs.insert(request_ref, Err(error.clone()));
                }
                continue;
            }

            let parser_started = Instant::now();
            let output = run_parser_batch(
                parser_script,
                &model_ref,
                &config_json,
                &requests,
            );
            parser_process_ns += parser_started.elapsed().as_nanos();
            match output {
                Ok(output) => {
                    for (request_ref, artifact) in output {
                        batched_outputs.insert(request_ref, Ok(artifact));
                    }
                }
                Err(error) => {
                    let error = format!("parser-process-error:{error}");
                    for (request_ref, _) in requests {
                        batched_outputs.insert(request_ref, Err(error.clone()));
                    }
                }
            }
        }

        for job in jobs {
            let job_started = Instant::now();
            if job.parser_family != "spacy" {
                persist_parser_residual(
                    config,
                    &job,
                    worker_ref,
                    &format!("unsupported-parser-family:{}", job.parser_family),
                )?;
                residual += 1;
                parser_job_ns.push(job_started.elapsed().as_nanos());
                continue;
            }

            let output = match batched_outputs.remove(&job.compilation_key) {
                Some(Ok(value)) => value,
                Some(Err(error)) => {
                    defer_parser_job_retry(config, &job, worker_ref, &error)?;
                    deferred_retry += 1;
                    parser_job_ns.push(job_started.elapsed().as_nanos());
                    continue;
                }
                None => {
                    defer_parser_job_retry(
                        config,
                        &job,
                        worker_ref,
                        "missing-batched-parser-output",
                    )?;
                    deferred_retry += 1;
                    parser_job_ns.push(job_started.elapsed().as_nanos());
                    continue;
                }
            };

            let wire: WireArtifact = match serde_json::from_slice(&output) {
                Ok(value) => value,
                Err(error) => {
                    defer_parser_job_retry(
                        config,
                        &job,
                        worker_ref,
                        &format!("parser-json-error:{error}"),
                    )?;
                    deferred_retry += 1;
                    parser_job_ns.push(job_started.elapsed().as_nanos());
                    continue;
                }
            };
            if wire.parser_family != job.parser_family
                || wire.parser_version != job.parser_version
                || wire.model_ref != job.model_ref
            {
                defer_parser_job_retry(
                    config,
                    &job,
                    worker_ref,
                    "parser-output-identity-mismatch",
                )?;
                deferred_retry += 1;
                parser_job_ns.push(job_started.elapsed().as_nanos());
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
                persist_parser_residual(config, &job, worker_ref, "parser-offset-overflow")?;
                residual += 1;
                parser_job_ns.push(job_started.elapsed().as_nanos());
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
                    config,
                    &job,
                    worker_ref,
                    "parser-entity-offset-overflow",
                )?;
                residual += 1;
                parser_job_ns.push(job_started.elapsed().as_nanos());
                continue;
            }

            token_count += tokens.len();
            entity_count += entities.len();
            let output_text = String::from_utf8(output.clone())?;
            let artifact = ParserArtifactRecord {
                format_ref: "application/vnd.sensiblaw.spacy-region+json".into(),
                content_digest_ref: digest_ref(&output),
                artifact_json: Some(output_text),
                object_locator: None,
            };
            let persist_started = Instant::now();
            parser_writer.persist_success_with_entities(
                &job,
                worker_ref,
                &tokens,
                &entities,
                Some(&artifact),
            )?;
            parser_persist_ns += persist_started.elapsed().as_nanos();
            succeeded += 1;
            parser_job_ns.push(job_started.elapsed().as_nanos());
        }
    }

    Ok(LocalWorkerReceipt {
        succeeded,
        residual,
        deferred_retry,
        token_count,
        entity_count,
        parser_process_ns,
        parser_persist_ns,
        parser_job_ns,
    })
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
    let worker = drain_local_worker(
        &config,
        parser_run_ref,
        worker_ref,
        batch_size,
        parser_script,
    )?;
    let state = parser_run_state(&config, parser_run_ref)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "sensiblaw.scale1.worker-receipt.v0_2",
            "parser_run_ref": parser_run_ref,
            "worker_ref": worker_ref,
            "succeeded_this_worker": worker.succeeded,
            "residual_this_worker": worker.residual,
            "deferred_retry_this_worker": worker.deferred_retry,
            "token_count": worker.token_count,
            "entity_count": worker.entity_count,
            "parser_process_ns": worker.parser_process_ns,
            "parser_persist_ns": worker.parser_persist_ns,
            "parser_job_p50_ns": percentile_ns(&worker.parser_job_ns, 50),
            "parser_job_p95_ns": percentile_ns(&worker.parser_job_ns, 95),
            "parser_job_p99_ns": percentile_ns(&worker.parser_job_ns, 99),
            "parser_job_max_ns": worker.parser_job_ns.iter().copied().max().unwrap_or(0),
            "parser_job_c1": concentration_ratio(&worker.parser_job_ns, 1),
            "parser_job_c10": concentration_ratio(&worker.parser_job_ns, 10),
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


#[allow(clippy::too_many_arguments)]
fn compile_source_values(
    canonical_text: String,
    title: Option<String>,
    source_family: SourceFamily,
    source_ref: &str,
    provider_ref: &str,
    acquisition_receipt_ref: &str,
    model_ref: &str,
    config_json: &str,
    parser_script: &str,
    batch_size: usize,
    input_transport: &str,
) -> Result<(), Box<dyn Error>> {
    let total_started = Instant::now();
    if canonical_text.is_empty() {
        return Err("compile-source received empty canonical text".into());
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
    if description.parser_family != "spacy" || description.model_ref != model_ref {
        return Err("spaCy parser description did not match requested model".into());
    }

    let config = load_database_config(None)?;

    let prepare_started = Instant::now();
    let prepared = prepare_db_native_long_source(
        &config,
        source_ref,
        &source_revision_ref,
        provider_ref,
        acquisition_receipt_ref,
        source_family,
        title,
        None,
        &canonical_text,
        &description.parser_family,
        &description.parser_version,
        model_ref,
        config_json,
    )?;
    let prepare_ns = prepare_started.elapsed().as_nanos();

    let worker_ref = format!(
        "worker:scale1-source:{}",
        digest_ref(prepared.parser_run.parser_run_ref.as_bytes())
    );
    let worker_started = Instant::now();
    let worker = drain_local_worker(
        &config,
        &prepared.parser_run.parser_run_ref,
        &worker_ref,
        batch_size,
        parser_script,
    )?;
    let worker_ns = worker_started.elapsed().as_nanos();

    let state = parser_run_state(&config, &prepared.parser_run.parser_run_ref)?;
    if state.queued != 0
        || state.leased != 0
        || state.unattempted_semantic_regions != 0
        || worker.deferred_retry != 0
    {
        return Err(format!(
            "source compile did not drain parser ledger: queued={} leased={} unattempted={} deferred_retry={}",
            state.queued,
            state.leased,
            state.unattempted_semantic_regions,
            worker.deferred_retry
        )
        .into());
    }

    let finalize_started = Instant::now();
    let receipt =
        finalize_db_native_long_document(&config, &prepared.parser_run.parser_run_ref)?;
    let finalize_ns = finalize_started.elapsed().as_nanos();
    let total_ns = total_started.elapsed().as_nanos();

    let parser_jobs = worker.parser_job_ns.len();
    let total_job_ns: u128 = worker.parser_job_ns.iter().copied().sum();
    let tokens = worker.token_count;
    // A replay can correctly reuse every durable parser job.  In that case
    // there is no parser-token work in this run, so a made-up denominator of
    // one would falsely report the whole replay wall time as ns/token.
    let wall_ns_per_token = (tokens != 0).then(|| total_ns / tokens as u128);
    let worker_ns_per_token = (tokens != 0).then(|| worker_ns / tokens as u128);
    let runtime_head = std::env::var("SENSIBLAW_RUNTIME_HEAD").ok();

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "sensiblaw.scale1.source-compile-baseline.v0_1",
            "source_family": source_family_ref(source_family),
            "authority": "execution_and_measurement_receipt_only",
            "runtime_head": runtime_head,
            "input_transport": input_transport,
            "source": {
                "source_ref": receipt.source.source_ref,
                "source_revision_ref": receipt.source.source_revision_ref,
                "content_digest_ref": receipt.source.content_digest_ref,
                "canonical_ref": receipt.source.canonical_ref,
                "document_ref": receipt.source.document_ref,
                "canonical_bytes": canonical_text.len(),
                "canonical_chars": canonical_text.chars().count(),
                "canonical_bytes_reload_identically": receipt.canonical_bytes_reload_identically
            },
            "parser": {
                "parser_run_ref": receipt.parser_run_ref,
                "parser_family": prepared.parser_run.parser_family,
                "parser_version": prepared.parser_run.parser_version,
                "model_ref": prepared.parser_run.model_ref,
                "config_digest_ref": prepared.parser_run.config_digest_ref,
                "worker_ref": worker_ref,
                "batch_size": batch_size,
                "new_jobs": prepared.newly_enqueued_job_count,
                "reused_jobs": prepared.reused_existing_job_count,
                "jobs_observed_this_run": parser_jobs,
                "succeeded_this_run": worker.succeeded,
                "residual_this_run": worker.residual,
                "deferred_retry_this_run": worker.deferred_retry,
                "tokens_this_run": worker.token_count,
                "entities_this_run": worker.entity_count
            },
            "integrity": {
                "total_structural_regions": receipt.total_structural_regions,
                "semantic_eligible_regions": receipt.semantic_eligible_regions,
                "parser_success_regions": receipt.parser_success_regions,
                "parser_residual_regions": receipt.parser_residual_regions,
                "unattempted_semantic_regions": receipt.unattempted_semantic_regions,
                "structural_only_regions": receipt.structural_only_regions,
                "source_region_loss_count": receipt.source_region_loss_count,
                "every_region_reloaded": receipt.every_region_reloaded,
                "candidate_pnf_reopen_complete": receipt.candidate_pnf_reopen_complete,
                "candidate_persistence_reused": receipt.candidate_persistence_reused,
                "l2_reconciliation_reused": receipt.reconciliation.stage_reused,
                "auto_event_projection_reused": receipt.auto_event.stage_reused,
                "compiled_statement_count": receipt.compiled_statement_count,
                "candidate_pnf_count": receipt.candidate_pnf_count,
                "persisted_statement_count": receipt.persisted_statement_count,
                "persisted_candidate_batch_count": receipt.persisted_candidate_batch_count,
                "persisted_candidate_factor_count": receipt.persisted_candidate_factor_count,
                "candidate_only": receipt.candidate_only,
                "creates_semantic_authority": receipt.creates_semantic_authority,
                "applicability_promoted": receipt.applicability_promoted,
                "claim_truth_promoted": receipt.claim_truth_promoted
            },
            "candidate_cardinality": {
                "entity_mentions": receipt.reconciliation.entity_mention_count,
                "entity_fingerprints": receipt.reconciliation.entity_fingerprint_count,
                "named_entity_mentions": receipt.reconciliation.named_entity_mention_count,
                "named_entity_fingerprints": receipt.reconciliation.named_entity_fingerprint_count,
                "temporal_mentions": receipt.reconciliation.temporal_mention_count,
                "proposition_occurrences": receipt.reconciliation.proposition_occurrence_count,
                "proposition_fingerprints": receipt.reconciliation.proposition_fingerprint_count,
                "event_occurrences": receipt.reconciliation.event_occurrence_count,
                "event_fingerprints": receipt.reconciliation.event_fingerprint_count,
                "polarity_conflicts": receipt.reconciliation.polarity_conflict_candidate_count,
                "review_pressure": receipt.reconciliation.review_pressure_candidate_count,
                "review_items": receipt.reconciliation_review.review_item_refs.len(),
                "auto_event_observations": receipt.auto_event.observations_materialized,
                "auto_event_bounded_pairs": receipt.auto_event.bounded_pair_count,
                "auto_event_proposals": receipt.auto_event.proposal_count
            },
            "performance": {
                "prepare_ns": prepare_ns,
                "worker_ns": worker_ns,
                "parser_process_ns": worker.parser_process_ns,
                "parser_persist_ns": worker.parser_persist_ns,
                "finalize_ns": finalize_ns,
                "finalize_load_and_validate_ns": receipt.timings.load_and_validate_ns,
                "m12_compile_ns": receipt.timings.m12_compile_ns,
                "candidate_persist_ns": receipt.timings.candidate_persist_ns,
                "l2_reconciliation_ns": receipt.timings.reconciliation_ns,
                "review_projection_ns": receipt.timings.review_projection_ns,
                "auto_event_ns": receipt.timings.auto_event_ns,
                "compilation_receipt_persist_ns": receipt.timings.compilation_persist_ns,
                "reload_verify_ns": receipt.timings.reload_verify_ns,
                "finalize_internal_total_ns": receipt.timings.finalize_total_ns,
                "total_ns": total_ns,
                "parser_job_sum_ns": total_job_ns,
                "parser_job_p50_ns": percentile_ns(&worker.parser_job_ns, 50),
                "parser_job_p95_ns": percentile_ns(&worker.parser_job_ns, 95),
                "parser_job_p99_ns": percentile_ns(&worker.parser_job_ns, 99),
                "parser_job_max_ns": worker.parser_job_ns.iter().copied().max().unwrap_or(0),
                "parser_job_c1": concentration_ratio(&worker.parser_job_ns, 1),
                "parser_job_c10": concentration_ratio(&worker.parser_job_ns, 10),
                "token_normalized_metrics_available": tokens != 0,
                "wall_ns_per_token": wall_ns_per_token,
                "worker_ns_per_token": worker_ns_per_token,
                "db_native_reuse_ratio": if prepared.semantic_region_count == 0 {
                    1.0
                } else {
                    prepared.reused_existing_job_count as f64
                        / prepared.semantic_region_count as f64
                },
                "downstream_candidate_persistence_reused": receipt.candidate_persistence_reused,
                "downstream_l2_reused": receipt.reconciliation.stage_reused,
                "downstream_auto_reused": receipt.auto_event.stage_reused,
                "recompute_ratio": if prepared.semantic_region_count == 0 {
                    0.0
                } else {
                    prepared.newly_enqueued_job_count as f64
                        / prepared.semantic_region_count as f64
                }
            }
        }))?
    );
    Ok(())
}

fn ingest_book(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 7 {
        return Err(
            "ingest-book <text-file> <source-ref> <provider-ref> <acquisition-receipt-ref> <model-ref> [config-json] [parser-script] [batch-size]"
                .into(),
        );
    }
    let text_file = &args[2];
    let canonical_text = fs::read_to_string(text_file)?;
    compile_source_values(
        canonical_text,
        Path::new(text_file)
            .file_name()
            .map(|value| value.to_string_lossy().into_owned()),
        SourceFamily::Document,
        &args[3],
        &args[4],
        &args[5],
        &args[6],
        args.get(7).map(String::as_str).unwrap_or("{}"),
        args.get(8)
            .map(String::as_str)
            .unwrap_or("scripts/scale1_spacy_json_parser.py"),
        args.get(9)
            .map(|value| value.parse::<usize>())
            .transpose()?
            .unwrap_or(32),
        "file",
    )
}

fn ingest_book_stdin(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 7 {
        return Err(
            "ingest-book-stdin <source-ref> <provider-ref> <acquisition-receipt-ref> <title> <model-ref> [config-json] [parser-script] [batch-size]"
                .into(),
        );
    }
    let mut canonical_text = String::new();
    std::io::stdin().read_to_string(&mut canonical_text)?;
    compile_source_values(
        canonical_text,
        (!args[5].is_empty()).then(|| args[5].clone()),
        SourceFamily::Document,
        &args[2],
        &args[3],
        &args[4],
        &args[6],
        args.get(7).map(String::as_str).unwrap_or("{}"),
        args.get(8)
            .map(String::as_str)
            .unwrap_or("scripts/scale1_spacy_json_parser.py"),
        args.get(9)
            .map(|value| value.parse::<usize>())
            .transpose()?
            .unwrap_or(32),
        "stdin",
    )
}


fn compile_source(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 8 {
        return Err(
            "compile-source <source-family> <text-file> <source-ref> <provider-ref> <acquisition-receipt-ref> <model-ref> [config-json] [parser-script] [batch-size]"
                .into(),
        );
    }
    let family = parse_source_family(&args[2])?;
    let text_file = &args[3];
    let canonical_text = fs::read_to_string(text_file)?;
    compile_source_values(
        canonical_text,
        Path::new(text_file)
            .file_name()
            .map(|value| value.to_string_lossy().into_owned()),
        family,
        &args[4],
        &args[5],
        &args[6],
        &args[7],
        args.get(8).map(String::as_str).unwrap_or("{}"),
        args.get(9)
            .map(String::as_str)
            .unwrap_or("scripts/scale1_spacy_json_parser.py"),
        args.get(10)
            .map(|value| value.parse::<usize>())
            .transpose()?
            .unwrap_or(32),
        "file",
    )
}

fn compile_source_stdin(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 8 {
        return Err(
            "compile-source-stdin <source-family> <source-ref> <provider-ref> <acquisition-receipt-ref> <title> <model-ref> [config-json] [parser-script] [batch-size]"
                .into(),
        );
    }
    let family = parse_source_family(&args[2])?;
    let mut canonical_text = String::new();
    std::io::stdin().read_to_string(&mut canonical_text)?;
    compile_source_values(
        canonical_text,
        (!args[6].is_empty()).then(|| args[6].clone()),
        family,
        &args[3],
        &args[4],
        &args[5],
        &args[7],
        args.get(8).map(String::as_str).unwrap_or("{}"),
        args.get(9)
            .map(String::as_str)
            .unwrap_or("scripts/scale1_spacy_json_parser.py"),
        args.get(10)
            .map(|value| value.parse::<usize>())
            .transpose()?
            .unwrap_or(32),
        "stdin",
    )
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
            "candidate_persistence_reused": receipt.candidate_persistence_reused,
            "l2_reconciliation_reused": receipt.reconciliation.stage_reused,
            "auto_event_projection_reused": receipt.auto_event.stage_reused,
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
            "reconciliation_review_item_refs": receipt.reconciliation_review.review_item_refs,
            "reconciliation_cluster_review_items": receipt.reconciliation_review.cluster_review_items,
            "reconciliation_contestation_review_items": receipt.reconciliation_review.contestation_review_items,
            "reconciliation_review_creates_event_identity": receipt.reconciliation_review.creates_event_identity,
            "reconciliation_review_creates_semantic_authority": receipt.reconciliation_review.creates_semantic_authority,
            "reconciliation_review_claim_truth_promoted": receipt.reconciliation_review.claim_truth_promoted,
            "auto_event_observations_materialized": receipt.auto_event.observations_materialized,
            "auto_event_bounded_pairs": receipt.auto_event.bounded_pair_count,
            "auto_event_proposals": receipt.auto_event.proposal_count,
            "auto_event_review_items": receipt.auto_event.review_item_count,
            "auto_event_proposal_refs": receipt.auto_event.proposal_refs,
            "auto_event_review_item_refs": receipt.auto_event.review_item_refs,
            "auto_event_creates_observation_identity": receipt.auto_event.creates_observation_identity,
            "auto_event_creates_event_identity": receipt.auto_event.creates_event_identity,
            "auto_event_creates_semantic_authority": receipt.auto_event.creates_semantic_authority,
            "auto_event_claim_truth_promoted": receipt.auto_event.claim_truth_promoted,
            "timing_load_and_validate_ns": receipt.timings.load_and_validate_ns,
            "timing_m12_compile_ns": receipt.timings.m12_compile_ns,
            "timing_candidate_persist_ns": receipt.timings.candidate_persist_ns,
            "timing_l2_reconciliation_ns": receipt.timings.reconciliation_ns,
            "timing_review_projection_ns": receipt.timings.review_projection_ns,
            "timing_auto_event_ns": receipt.timings.auto_event_ns,
            "timing_compilation_receipt_persist_ns": receipt.timings.compilation_persist_ns,
            "timing_reload_verify_ns": receipt.timings.reload_verify_ns,
            "timing_finalize_total_ns": receipt.timings.finalize_total_ns,
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


fn materialize_event(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 4 || args.len() > 5 {
        return Err(
            "materialize-event <proposal-ref> <accepted-review-command-ref> [event-ref]"
                .into(),
        );
    }
    let proposal_ref = &args[2];
    let accepted_review_command_ref = &args[3];
    let event_ref = args.get(4).cloned().unwrap_or_else(|| {
        format!(
            "event:reviewed:{}",
            digest_ref(
                format!(
                    "{proposal_ref}\u{1f}{accepted_review_command_ref}"
                )
                .as_bytes(),
            )
        )
    });

    let config = load_database_config(None)?;
    let receipt = materialize_accepted_event_join(
        &config,
        proposal_ref,
        &event_ref,
        accepted_review_command_ref,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "sensiblaw.scale1.reviewed-event-materialization.v0_1",
            "assembly_ref": receipt.assembly_ref,
            "proposal_ref": receipt.proposal_ref,
            "event_ref": receipt.event_ref,
            "review_item_ref": receipt.review_item_ref,
            "accepted_review_command_ref": receipt.accepted_review_command_ref,
            "observation_refs": receipt.observation_refs,
            "statement_refs": receipt.statement_refs,
            "reviewed_event_identity": receipt.reviewed_event_identity,
            "creates_semantic_authority": receipt.creates_semantic_authority,
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
         scale1_long_document compile-source <source-family> <text-file> <source-ref> <provider-ref> <acquisition-receipt-ref> <model-ref> [config-json] [parser-script] [batch-size]\n  \
         scale1_long_document compile-source-stdin <source-family> <source-ref> <provider-ref> <acquisition-receipt-ref> <title> <model-ref> [config-json] [parser-script] [batch-size]\n  \
         scale1_long_document ingest-book <text-file> <source-ref> <provider-ref> <acquisition-receipt-ref> <model-ref> [config-json] [parser-script] [batch-size]\n  \
         scale1_long_document ingest-book-stdin <source-ref> <provider-ref> <acquisition-receipt-ref> <title> <model-ref> [config-json] [parser-script] [batch-size]\n  \
         scale1_long_document prepare-spacy <text-file> <source-ref> <provider-ref> <acquisition-receipt-ref> <model-ref> [config-json] [parser-script]\n  \
         scale1_long_document prepare-gwb <projection-manifest> <document-ordinal> <model-ref> [config-json] [parser-script]\n  \
         scale1_long_document prepare-stdin <source-ref> <provider-ref> <acquisition-receipt-ref> <title> <model-ref> [config-json] [parser-script]\n  \
         scale1_long_document prepare-stdin-family <source-family> <source-ref> <provider-ref> <acquisition-receipt-ref> <title> <model-ref> [config-json] [parser-script]\n  \
         scale1_long_document status <parser-run-ref>\n  \
         scale1_long_document worker <parser-run-ref> <worker-ref> [batch-size] [parser-script]\n  \
         scale1_long_document finalize <parser-run-ref>\n  \
         scale1_long_document review <review-item-ref> <action> <reviewer-ref> [qualification-ref] [evidence-request-ref]\n  \
         scale1_long_document materialize-event <proposal-ref> <accepted-review-command-ref> [event-ref]\n  \
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
        "compile-source" => compile_source(&args),
        "compile-source-stdin" => compile_source_stdin(&args),
        "ingest-book" => ingest_book(&args),
        "ingest-book-stdin" => ingest_book_stdin(&args),
        "prepare-spacy" => prepare_spacy(&args),
        "prepare-gwb" => prepare_gwb_projection(&args),
        "prepare-stdin" => prepare_stdin(&args),
        "prepare-stdin-family" => prepare_stdin_family(&args),
        "status" => status(&args),
        "worker" => worker(&args),
        "finalize" => finalize(&args),
        "review" => apply_review(&args),
        "materialize-event" => materialize_event(&args),
        "materialize-proposition" => materialize_proposition(&args),
        _ => {
            usage();
            Err(format!("unknown command: {command}").into())
        }
    }
}
