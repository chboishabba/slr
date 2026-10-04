//! SOURCE-MATERIALISATION-1 operator runner.
//!
//! Normal source path:
//! governed OALC provider -> immutable pinned SHA -> exact retained row ->
//! provider materialisation -> legal-source registration.
//!
//! No JSON packet or local corpus mirror is accepted. On a later reopen, the
//! runner reuses the revision/version already persisted in PostgreSQL and asks
//! the governed provider for that historical pin; it never substitutes the
//! current OALC head silently.

use std::{env, process};

use sensiblaw_governed_legal_provider::{
    resolve_oalc_dataset_revision, run_pinned_oalc_stream, OalcCaseFollowError,
    OalcCitationMatch, PinnedOalcStreamRequest, OALC_DATASET_ID, OALC_SPLIT,
};
use sensiblaw_legal_follow_plan::OALC_PROVIDER_PROFILE;
use sensiblaw_pg_source_store::{
    load_database_config, load_provider_materialization, materialization_ref_for_demand,
    persist_provider_legal_source_registration, resolve_provider_materialization,
    AcquiredSourceBundle, CacheFirstAcquirer, CacheLookupDemand, ExactResolutionReceiptOwned,
    ProviderLegalSourceRegistrationDraft, ResolutionPath, ResolvedExternalDocumentOwned,
    TemporalCoverage,
};

#[derive(Debug)]
struct PinnedGovernedOalcAcquirer {
    revision: String,
    citation_match: OalcCitationMatch,
    document_type: String,
    source: Option<String>,
    oalc_jurisdiction: Option<String>,
    expected_version_id: Option<String>,
    revision_resolution_network_requests: i64,
}

impl CacheFirstAcquirer for PinnedGovernedOalcAcquirer {
    type Error = OalcCaseFollowError;

    fn acquire(
        &mut self,
        demand: &CacheLookupDemand<'_>,
    ) -> Result<AcquiredSourceBundle, Self::Error> {
        let receipt = run_pinned_oalc_stream(&PinnedOalcStreamRequest {
            revision: self.revision.clone(),
            citation: demand.citation.to_owned(),
            citation_match: self.citation_match,
            document_type: self.document_type.clone(),
            source: self.source.clone(),
            jurisdiction: self.oalc_jurisdiction.clone(),
        })?;
        if let Some(expected) = self.expected_version_id.as_deref() {
            if receipt.row.version_id != expected {
                return Err(OalcCaseFollowError::Validation(format!(
                    "pinned OALC rematerialisation returned version_id {} instead of persisted {}",
                    receipt.row.version_id, expected
                )));
            }
        }
        if receipt.row.text.trim().is_empty() {
            return Err(OalcCaseFollowError::Validation(
                "pinned OALC row has empty canonical text".into(),
            ));
        }
        let temporal_coverage = match demand.temporal_ref {
            Some(requested) if receipt.row.date.as_deref() == Some(requested) => {
                TemporalCoverage::HistoricallyVerified
            }
            _ => TemporalCoverage::LatestKnownOnly,
        };
        let evidence_ref = format!(
            "oalc:pinned:{}:{}",
            self.revision, receipt.row.version_id
        );
        Ok(AcquiredSourceBundle {
            document: ResolvedExternalDocumentOwned {
                provider_ref: OALC_PROVIDER_PROFILE.into(),
                dataset_ref: OALC_DATASET_ID.into(),
                dataset_revision_ref: self.revision.clone(),
                external_version_ref: receipt.row.version_id,
                citation: receipt.row.citation,
                source_ref: receipt.row.source,
                jurisdiction_ref: receipt.row.jurisdiction,
                document_type_ref: receipt.row.document_type,
                temporal_coverage,
                resolution_path: ResolutionPath::RevisionPinnedStreamingLegacy,
                source_url: receipt.row.url,
                canonical_text: receipt.row.text,
            },
            receipt: ExactResolutionReceiptOwned {
                demand_ref: demand.demand_ref.to_owned(),
                consumer_ref: None,
                requested_citation: demand.citation.to_owned(),
                requested_jurisdiction_ref: demand.jurisdiction_ref.to_owned(),
                requested_source_role_ref: demand.source_role_ref.to_owned(),
                requested_authority_level_ref: demand.authority_level_ref.to_owned(),
                requested_temporal_ref: demand.temporal_ref.map(str::to_owned),
                exact_demand_match: true,
                acquisition_authority_ref: format!("oalc:pinned-revision:{}", self.revision),
                receipt_authority_ref: "source-observation-only".into(),
                network_request_count: 1 + self.revision_resolution_network_requests,
                resolver_ref: "sensiblaw-governed-legal-provider:oalc-pinned-stream".into(),
                resolution_evidence_ref: evidence_ref,
            },
        })
    }
}

fn optional(value: &str) -> Option<String> {
    (value != "-").then(|| value.to_owned())
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 10 {
        return Err(
            "usage: materialize_oalc_provider <demand-ref> <citation-or-terminal-mnc> <exact|contains> <oalc-document-type> <oalc-source-or-dash> <oalc-jurisdiction-or-dash> <source-role> <authority-level> <corpus-ref> <admission-profile-ref>"
                .into(),
        );
    }
    let citation_match = match args[2].as_str() {
        "exact" => OalcCitationMatch::Exact,
        "contains" => OalcCitationMatch::Contains,
        _ => return Err("citation match must be exact or contains".into()),
    };
    let oalc_jurisdiction = optional(&args[5]);
    let requested_jurisdiction = oalc_jurisdiction
        .clone()
        .unwrap_or_else(|| "provider-scoped".into());
    let demand = CacheLookupDemand {
        demand_ref: &args[0],
        citation: &args[1],
        jurisdiction_ref: &requested_jurisdiction,
        source_role_ref: &args[6],
        authority_level_ref: &args[7],
        temporal_ref: None,
    };
    let config = load_database_config(None).map_err(|error| error.to_string())?;

    // Reopen the durable pin first. Only a genuinely new demand resolves the
    // provider's current dataset SHA, and that SHA is immediately frozen.
    let existing_ref = materialization_ref_for_demand(&config, &demand, OALC_SPLIT)
        .map_err(|error| error.to_string())?;
    let (revision, expected_version_id, revision_resolution_network_requests) =
        if let Some(reference) = existing_ref.as_deref() {
            let existing = load_provider_materialization(&config, reference)
                .map_err(|error| error.to_string())?;
            (
                existing.identity.dataset_revision_ref,
                Some(existing.identity.external_version_ref),
                0,
            )
        } else {
            (
                resolve_oalc_dataset_revision().map_err(|error| format!("{error:?}"))?,
                None,
                1,
            )
        };

    let mut acquirer = PinnedGovernedOalcAcquirer {
        revision,
        citation_match,
        document_type: args[3].clone(),
        source: optional(&args[4]),
        oalc_jurisdiction,
        expected_version_id,
        revision_resolution_network_requests,
    };
    let resolved = resolve_provider_materialization(&config, &mut acquirer, &demand, OALC_SPLIT)
        .map_err(|error| error.to_string())?;

    let legal = persist_provider_legal_source_registration(
        &config,
        &ProviderLegalSourceRegistrationDraft {
            materialization_ref: resolved.materialization.materialization_ref.clone(),
            corpus_ref: args[8].clone(),
            admission_profile_ref: args[9].clone(),
            source_role: args[6].clone(),
            authority_level: args[7].clone(),
            semantic_scope: "provider-materialised-legal-source".into(),
            temporal_refs: vec![],
            provider_profile_refs: vec![OALC_PROVIDER_PROFILE.into()],
            media_type: "text/plain".into(),
        },
    )
    .map_err(|error| error.to_string())?;

    println!("materialization_ref={}", resolved.materialization.materialization_ref);
    println!("provider_revision={}", resolved.materialization.identity.dataset_revision_ref);
    println!("provider_version={}", resolved.materialization.identity.external_version_ref);
    println!("canonical_sha256={}", resolved.materialization.canonical_sha256_hex);
    println!("document_ref={}", resolved.materialization.document_ref);
    println!("external_source_revision_ref={}", resolved.materialization.external_source_revision_ref);
    println!("legal_source_revision_ref={}", legal.source_revision_ref);
    println!("admission_receipt_ref={}", legal.admission_receipt_ref);
    println!("bytes_resident={}", resolved.materialization.bytes_resident);
    println!("rematerialized_evicted_bytes={}", resolved.rematerialized_evicted_bytes);
    println!("reused_durable_identity={}", resolved.reused_durable_identity);
    println!("network_requests={}", resolved.acquisition_network_requests);
    println!("json_control_plane=false");
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
