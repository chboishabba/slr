#[cfg(not(feature = "live-network"))]
fn main() {
    eprintln!(
        "live_nsw_pit_cullen requires --features live-network and explicit --operator-opt-in"
    );
    std::process::exit(2);
}

#[cfg(feature = "live-network")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::Duration;

    use sensiblaw_governed_legal_provider::{
        GovernedExecutionContext, LiveGovernanceBounds, UreqTransport,
    };
    use sensiblaw_nsw_legislation_provider::{
        admit_historical_artifact, fetch_official_pit_xml, official_pit_xml_reference,
        parser_handoff, retain_official_fetch_candidate, HistoricalLegislationDemand,
        HistoricalLegislationReceipt,
    };

    #[derive(Debug, Clone)]
    struct Row {
        demand_ref: String,
        act_identity_ref: String,
        official_document_id: String,
        locator: String,
        in_force_on: String,
        expected_version_effective_from: Option<String>,
        proposition_ref: String,
        official_pit_xml: String,
    }

    impl Row {
        fn demand(&self) -> HistoricalLegislationDemand {
            HistoricalLegislationDemand {
                demand_ref: self.demand_ref.clone(),
                jurisdiction_ref: "AU-NSW".into(),
                act_identity_ref: self.act_identity_ref.clone(),
                official_document_id: self.official_document_id.clone(),
                requested_locator: self.locator.clone(),
                in_force_on: self.in_force_on.clone(),
                expected_version_effective_from: self.expected_version_effective_from.clone(),
                proposition_ref: self.proposition_ref.clone(),
                source_identity_ref: self.act_identity_ref.clone(),
            }
        }
    }

    fn value_after(args: &[String], flag: &str) -> Option<String> {
        args.windows(2)
            .find(|window| window[0] == flag)
            .map(|window| window[1].clone())
    }

    fn load_manifest(path: &Path) -> Result<Vec<Row>, Box<dyn std::error::Error>> {
        let input = fs::read_to_string(path)?;
        let mut rows = Vec::new();
        for (index, raw) in input.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || index == 0 {
                continue;
            }
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 8 {
                return Err(format!(
                    "{}:{} expected 8 TSV fields, got {}",
                    path.display(),
                    index + 1,
                    fields.len()
                )
                .into());
            }
            rows.push(Row {
                demand_ref: fields[0].trim().into(),
                act_identity_ref: fields[1].trim().into(),
                official_document_id: fields[2].trim().into(),
                locator: fields[3].trim().into(),
                in_force_on: fields[4].trim().into(),
                expected_version_effective_from: match fields[5].trim() {
                    "" => None,
                    value => Some(value.into()),
                },
                proposition_ref: fields[6].trim().into(),
                official_pit_xml: fields[7].trim().into(),
            });
        }
        if rows.is_empty() {
            return Err("PIT manifest contained no acquisition rows".into());
        }
        Ok(rows)
    }

    fn tsv_cell(value: &str) -> String {
        value.replace(['\t', '\r', '\n'], " ")
    }

    fn receipt_row(receipt: &HistoricalLegislationReceipt) -> String {
        [
            receipt.demand_ref.clone(),
            receipt.act_identity_ref.clone(),
            receipt.official_document_id.clone(),
            receipt.locator.clone(),
            receipt.in_force_on.clone(),
            receipt.version_effective_from.clone().unwrap_or_default(),
            receipt.source_revision_ref.clone(),
            receipt.canonical_bytes_digest.clone(),
            receipt.local_artifact_ref.clone(),
            receipt.point_in_time_evidence.evidence_reference.clone(),
            receipt.point_in_time_evidence.evidence_scope.clone(),
            receipt.parser_eligible.to_string(),
            receipt.compile_eligible.to_string(),
            receipt.receipt_authority.into(),
        ]
        .iter()
        .map(|value| tsv_cell(value))
        .collect::<Vec<_>>()
        .join("\t")
    }

    let args = env::args().collect::<Vec<_>>();
    if !args.iter().any(|arg| arg == "--operator-opt-in") {
        return Err(
            "explicit operator consent required: pass --operator-opt-in to permit live NSW Legislation requests"
                .into(),
        );
    }

    let manifest = PathBuf::from(
        value_after(&args, "--manifest")
            .unwrap_or_else(|| "fixtures/cullen_nsw_legislation_pit_v0_1.tsv".into()),
    );
    let output_dir = PathBuf::from(
        value_after(&args, "--output-dir")
            .unwrap_or_else(|| "artifacts/nsw-pit/cullen-2017-01-26".into()),
    );

    let rows = load_manifest(&manifest)?;
    let mut groups: BTreeMap<(String, String), Vec<Row>> = BTreeMap::new();
    for row in rows {
        let expected = official_pit_xml_reference(&row.demand())?;
        if expected != row.official_pit_xml {
            return Err(format!(
                "manifest official PIT URL mismatch for {}: expected {}, got {}",
                row.demand_ref, expected, row.official_pit_xml
            )
            .into());
        }
        groups
            .entry((row.official_document_id.clone(), row.in_force_on.clone()))
            .or_default()
            .push(row);
    }

    if groups.len() != 2 {
        return Err(format!(
            "Cullen PIT operator run must resolve exactly two document/date fetches after deduplication; got {}",
            groups.len()
        )
        .into());
    }

    let context = GovernedExecutionContext {
        operator_opt_in: true,
        cache_checked_first: true,
        persisted_receipts_checked_first: true,
        bounds: LiveGovernanceBounds {
            minimum_pacing_seconds: 4,
            burst: 1,
            max_depth: 1,
            max_new_documents: 2,
            max_network_requests: 2,
        },
    };
    context.validate().map_err(|err| format!("governance: {err:?}"))?;

    fs::create_dir_all(output_dir.join("raw"))?;
    fs::create_dir_all(output_dir.join("receipts"))?;

    let mut transport = UreqTransport;
    let mut emitted = Vec::<HistoricalLegislationReceipt>::new();
    let mut network_requests = 0u64;

    for ((document_id, in_force_on), group) in groups {
        if network_requests >= context.bounds.max_network_requests {
            return Err("bounded two-document PIT request budget exhausted".into());
        }
        if network_requests > 0 {
            thread::sleep(Duration::from_secs(context.bounds.minimum_pacing_seconds));
        }

        let canonical = group.first().ok_or("empty deduplicated PIT group")?;
        for row in &group {
            if row.act_identity_ref != canonical.act_identity_ref
                || row.official_document_id != canonical.official_document_id
                || row.in_force_on != canonical.in_force_on
                || row.expected_version_effective_from
                    != canonical.expected_version_effective_from
                || row.official_pit_xml != canonical.official_pit_xml
            {
                return Err(format!(
                    "deduplicated PIT group mixes incompatible source revisions: {} / {}",
                    document_id, in_force_on
                )
                .into());
            }
        }

        let fetch_demand = canonical.demand();
        eprintln!(
            "governed NSW PIT fetch {}/2: {} @ {}",
            network_requests + 1,
            document_id,
            in_force_on
        );
        let candidate = fetch_official_pit_xml(&mut transport, &context, &fetch_demand)?;
        network_requests += candidate.network_requests;

        let raw_path = output_dir
            .join("raw")
            .join(format!("{}_{}.xml", document_id, in_force_on));
        let retained = retain_official_fetch_candidate(&fetch_demand, &candidate, &raw_path)?;

        // One full-document PIT fetch can support multiple locator-specific demands.
        // The retained bytes/revision remain identical; each demand receives its
        // own locator-scoped historical receipt.
        for row in group {
            let demand = row.demand();
            let mut locator_artifact = retained.clone();
            locator_artifact.act_identity_ref = demand.act_identity_ref.clone();
            locator_artifact.source_identity_ref = demand.source_identity_ref.clone();
            locator_artifact.locator = demand.requested_locator.clone();
            locator_artifact.version_effective_from =
                demand.expected_version_effective_from.clone();
            emitted.push(admit_historical_artifact(&demand, &locator_artifact)?);
        }
    }

    if network_requests != 2 {
        return Err(format!(
            "expected exactly two live network requests after deduplication, got {network_requests}"
        )
        .into());
    }

    let receipt_path = output_dir.join("receipts/historical_artifacts.tsv");
    let mut receipt_tsv = String::from(
        "demand_ref\tact_identity_ref\tofficial_document_id\tlocator\tin_force_on\tversion_effective_from\tsource_revision_ref\tcanonical_bytes_digest\tlocal_artifact_ref\tpoint_in_time_evidence_reference\tpoint_in_time_evidence_scope\tparser_eligible\tcompile_eligible\treceipt_authority\n",
    );
    for receipt in &emitted {
        receipt_tsv.push_str(&receipt_row(receipt));
        receipt_tsv.push('\n');
        let handoff = parser_handoff(receipt).ok_or_else(|| {
            format!("historical receipt was not parser eligible: {}", receipt.demand_ref)
        })?;
        if handoff.source_revision_ref != receipt.source_revision_ref
            || handoff.canonical_bytes_digest != receipt.canonical_bytes_digest
        {
            return Err(format!(
                "parser handoff lost source revision/digest for {}",
                receipt.demand_ref
            )
            .into());
        }
    }
    fs::write(&receipt_path, receipt_tsv)?;

    println!(
        "retained {} official NSW PIT XML revisions; emitted {} locator-scoped historical receipts; network_requests={}; receipts={}",
        2,
        emitted.len(),
        network_requests,
        receipt_path.display()
    );
    Ok(())
}
