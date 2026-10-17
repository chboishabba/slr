//! Materialise one official HCA DOCX after pinned OALC-compatible extraction.
//!
//! The DOCX is an official sibling manifestation of the same judgment. It is
//! not a second legal authority and never claims OALC corpus membership.

use std::{env, fs, process};

use sha2::{Digest, Sha256};
use sensiblaw_pg_source_store::{
    canonical_official_primary_docx_extraction_ref, load_database_config,
    persist_official_primary_docx_extraction, persist_provider_legal_source_registration,
    persist_provider_materialization, ExactResolutionReceipt,
    OfficialPrimaryDocxExtractionReceipt, ProviderLegalSourceRegistrationDraft,
    ProviderMaterializationIdentity, ResolutionPath, ResolvedExternalDocument,
    TemporalCoverage,
};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 10 {
        return Err(
            "usage: materialize_hca_official_docx <demand-ref> <citation> <external-version-ref> <official-docx-url> <extracted-text-path> <extraction-receipt-json> <source-role> <authority-level> <corpus-ref> <admission-profile-ref>"
                .into(),
        );
    }

    let demand_ref = &args[0];
    let citation = &args[1];
    let external_version_ref = &args[2];
    let official_docx_url = &args[3];
    let extracted_text = fs::read_to_string(&args[4]).map_err(|error| error.to_string())?;
    let receipt_bytes = fs::read(&args[5]).map_err(|error| error.to_string())?;
    let extraction: OfficialPrimaryDocxExtractionReceipt =
        serde_json::from_slice(&receipt_bytes).map_err(|error| error.to_string())?;

    if extraction.citation != *citation {
        return Err("extraction receipt citation does not match requested citation".into());
    }
    if extraction.source_url != *official_docx_url {
        return Err("extraction receipt source URL does not match official DOCX URL".into());
    }
    if extraction.output_sha256_hex != sha256_hex(extracted_text.as_bytes()) {
        return Err("extracted UTF-8 digest does not match extraction receipt".into());
    }
    if extraction.output_size_bytes != extracted_text.as_bytes().len() as u64 {
        return Err("extracted UTF-8 byte length does not match extraction receipt".into());
    }

    let extraction_ref = canonical_official_primary_docx_extraction_ref(&extraction)
        .map_err(|error| error.to_string())?;
    let identity = ProviderMaterializationIdentity {
        provider_ref: extraction.source_provider_ref.clone(),
        dataset_ref: "hca:official-judgment-docx".into(),
        dataset_revision_ref: format!("sha256:{}", extraction.source_sha256_hex),
        split_ref: "single-judgment".into(),
        external_version_ref: external_version_ref.clone(),
        citation: citation.clone(),
        source_ref: "high_court_of_australia".into(),
        jurisdiction_ref: "commonwealth".into(),
        source_url: Some(official_docx_url.clone()),
        acquisition_receipt_ref: extraction_ref.clone(),
    };
    let document = ResolvedExternalDocument {
        provider_ref: &identity.provider_ref,
        dataset_ref: &identity.dataset_ref,
        dataset_revision_ref: &identity.dataset_revision_ref,
        external_version_ref: &identity.external_version_ref,
        citation: &identity.citation,
        source_ref: &identity.source_ref,
        jurisdiction_ref: &identity.jurisdiction_ref,
        document_type_ref: "decision",
        temporal_coverage: TemporalCoverage::HistoricallyVerified,
        resolution_path: ResolutionPath::FilterExact,
        source_url: identity.source_url.as_deref(),
        canonical_text: &extracted_text,
    };
    let resolution = ExactResolutionReceipt {
        demand_ref,
        consumer_ref: None,
        requested_citation: citation,
        requested_jurisdiction_ref: "commonwealth",
        requested_source_role_ref: &args[6],
        requested_authority_level_ref: &args[7],
        requested_temporal_ref: None,
        exact_demand_match: true,
        acquisition_authority_ref: "official-hca-docx",
        receipt_authority_ref: "source-observation-only",
        network_request_count: 0,
        resolver_ref: "sensiblaw:official-hca-docx:oalc-compatible-extractor",
        resolution_evidence_ref: &extraction_ref,
    };

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let materialization = persist_provider_materialization(
        &config,
        &identity,
        &document,
        &resolution,
        &[],
    )
    .map_err(|error| error.to_string())?;
    let extraction_row = persist_official_primary_docx_extraction(
        &config,
        &materialization.materialization_ref,
        &extraction,
    )
    .map_err(|error| error.to_string())?;
    let legal = persist_provider_legal_source_registration(
        &config,
        &ProviderLegalSourceRegistrationDraft {
            materialization_ref: materialization.materialization_ref.clone(),
            corpus_ref: args[8].clone(),
            admission_profile_ref: args[9].clone(),
            source_role: args[6].clone(),
            authority_level: args[7].clone(),
            semantic_scope: "official-primary-docx-extracted-text".into(),
            temporal_refs: vec![],
            provider_profile_refs: vec![
                extraction.source_provider_ref.clone(),
                format!("{}@{}", extraction.extractor_repo, extraction.extractor_revision),
            ],
            media_type: "text/plain".into(),
        },
    )
    .map_err(|error| error.to_string())?;

    println!("materialization_ref={}", materialization.materialization_ref);
    println!("extraction_ref={}", extraction_row.extraction_ref);
    println!("official_docx_sha256={}", extraction.source_sha256_hex);
    println!("canonical_text_sha256={}", extraction.output_sha256_hex);
    println!("extractor_revision={}", extraction.extractor_revision);
    println!("legal_source_revision_ref={}", legal.source_revision_ref);
    println!("source_is_official_hca_docx=true");
    println!("source_is_oalc_corpus=false");
    println!("independent_source_count_increment=0");
    println!("creates_semantic_authority=false");
    println!("creates_legal_authority=false");
    println!("claim_truth_promoted=false");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
