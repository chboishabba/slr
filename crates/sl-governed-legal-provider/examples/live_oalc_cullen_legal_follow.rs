#[cfg(not(feature = "live-network"))]
fn main() {
    eprintln!("enable --features live-network to run governed OALC resolution");
}

#[cfg(feature = "live-network")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use sensiblaw_governed_legal_provider::{
        oalc_legislation_contract::{
            CULLEN_CLA_CITATION, CULLEN_VICARIOUS_CITATION, OALC_RECEIPT_AUTHORITY,
        },
        resolve_live_oalc_exact_source, OalcCitationMatch, OalcExactSourceRequest,
    };
    use sha2::{Digest, Sha256};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};

    if !env::args().any(|arg| arg == "--operator-opt-in") {
        return Err("explicit --operator-opt-in required".into());
    }

    fn slug(citation: &str) -> String {
        citation
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

    fn sha256(bytes: &[u8]) -> String {
        format!("sha256:{:x}", Sha256::digest(bytes))
    }

    fn write_readonly(path: &Path, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, text.as_bytes())?;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_readonly(true);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }

    fn tsv(value: &str) -> String {
        value.replace(['\t', '\r', '\n'], " ")
    }

    let output_dir = PathBuf::from(
        env::var("SENSIBLAW_OALC_OUTPUT")
            .unwrap_or_else(|_| "artifacts/oalc/cullen-governing-law/materialised".into()),
    );
    fs::create_dir_all(&output_dir)?;

    let targets = [CULLEN_CLA_CITATION, CULLEN_VICARIOUS_CITATION];
    let mut common_revision: Option<String> = None;
    let mut total_network_requests = 0u64;
    let mut receipt = String::from(
        "citation\tversion_id\tcorpus_revision\tsource\tjurisdiction\ttype\tdate\turl\twhen_scraped\tcanonical_text_digest\tlocal_artifact_ref\ttemporal_status\tresolution_path\tnetwork_requests\treceipt_authority\n",
    );

    for citation in targets {
        let resolved = resolve_live_oalc_exact_source(&OalcExactSourceRequest {
            citation: citation.into(),
            citation_match: OalcCitationMatch::Exact,
            document_type: "primary_legislation".into(),
            source: Some("nsw_legislation".into()),
            jurisdiction: Some("new_south_wales".into()),
        })
        .map_err(|error| format!("OALC source resolution failed for {citation}: {error:?}"))?;

        let corpus_revision = format!(
            "isaacus/open-australian-legal-corpus@{}",
            resolved.corpus_revision_sha
        );
        if let Some(existing) = common_revision.as_deref() {
            if existing != corpus_revision {
                return Err(format!(
                    "Cullen acquisition crossed OALC revisions: {existing} vs {corpus_revision}"
                )
                .into());
            }
        } else {
            common_revision = Some(corpus_revision.clone());
        }

        let row = resolved.row;
        if row.citation != citation || row.text.trim().is_empty() {
            return Err(format!("OALC returned wrong or empty record for {citation}").into());
        }
        let artifact = output_dir.join(format!("{}.txt", slug(citation)));
        write_readonly(&artifact, &row.text)?;
        let digest = sha256(row.text.as_bytes());
        total_network_requests += resolved.network_requests;

        let fields = [
            row.citation,
            row.version_id,
            corpus_revision,
            row.source,
            row.jurisdiction,
            row.document_type,
            row.date.unwrap_or_default(),
            row.url.unwrap_or_default(),
            row.when_scraped.unwrap_or_default(),
            digest,
            artifact.to_string_lossy().into_owned(),
            "latest_known_only".into(),
            resolved.resolution_path,
            resolved.network_requests.to_string(),
            OALC_RECEIPT_AUTHORITY.into(),
        ];
        receipt.push_str(
            &fields
                .iter()
                .map(|value| tsv(value))
                .collect::<Vec<_>>()
                .join("\t"),
        );
        receipt.push('\n');
    }

    let receipt_path = output_dir.join("oalc_legislation_receipts.tsv");
    fs::write(&receipt_path, receipt)?;
    println!(
        "LegalFollow -> governed OALC resolved 2 legislation documents; corpus_revision={}; network_requests={}; receipts={}",
        common_revision.unwrap_or_default(),
        total_network_requests,
        receipt_path.display()
    );
    Ok(())
}
