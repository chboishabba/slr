use sensiblaw_governed_legal_provider::{
    oalc_legislation_contract::{
        OalcSectionSliceReceipt, OalcTemporalCoverage, CULLEN_CLA_CITATION,
        CULLEN_VICARIOUS_CITATION, OALC_PARSER_AUTHORITY,
    },
    resolve_live_oalc_exact_source, OalcCitationMatch, OalcExactSourceRequest,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type CliResult<T = ()> = Result<T, String>;

const TARGETS: [(&str, &[&str]); 2] = [
    (CULLEN_CLA_CITATION, &["5A", "5B", "5C", "5D", "43A"]),
    (CULLEN_VICARIOUS_CITATION, &["6", "7", "8"]),
];

fn value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn slug(value: &str) -> String {
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

fn section_token(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let token = parts.next()?;
    let rest = parts.next()?.trim();
    if rest.is_empty() {
        return None;
    }

    let bytes = token.as_bytes();
    let digit_count = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_count == 0 {
        return None;
    }
    let suffix = &bytes[digit_count..];
    let valid_suffix = suffix.is_empty()
        || (suffix.len() == 1 && suffix[0].is_ascii_uppercase());
    valid_suffix.then_some(token)
}

fn section_spans(text: &str) -> BTreeMap<String, (usize, usize)> {
    let mut headings = Vec::new();
    let mut offset = 0usize;
    for line in text.split_inclusive('
') {
        let logical = line.strip_suffix('
').unwrap_or(line);
        if let Some(section) = section_token(logical) {
            headings.push((section.to_string(), offset));
        }
        offset += line.len();
    }
    if offset < text.len() {
        let logical = &text[offset..];
        if let Some(section) = section_token(logical) {
            headings.push((section.to_string(), offset));
        }
    }

    let mut occurrences: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    for (index, (section, start)) in headings.iter().enumerate() {
        let end = headings
            .get(index + 1)
            .map_or(text.len(), |(_, next)| *next);
        occurrences
            .entry(section.clone())
            .or_default()
            .push((*start, end));
    }
    occurrences
        .into_iter()
        .filter_map(|(section, spans)| {
            (spans.len() == 1).then(|| (section, spans[0]))
        })
        .collect()
}

fn write_readonly(path: &Path, bytes: &[u8]) -> CliResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|error| format!("write {}: {error}", path.display()))?;
    let mut permissions = fs::metadata(path)
        .map_err(|error| format!("metadata {}: {error}", path.display()))?
        .permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("set readonly {}: {error}", path.display()))
}

#[derive(Debug)]
struct ParentDocument {
    citation: String,
    version_id: String,
    corpus_revision: String,
    source: String,
    jurisdiction: String,
    document_type: String,
    date: Option<String>,
    url: Option<String>,
    when_scraped: Option<String>,
    canonical_text_digest: String,
    artifact: PathBuf,
    resolution_path: String,
    network_requests: u64,
    text: String,
}

fn acquire_parent_documents(materialised: &Path) -> CliResult<Vec<ParentDocument>> {
    fs::create_dir_all(materialised)
        .map_err(|error| format!("create {}: {error}", materialised.display()))?;

    let mut parents = Vec::new();
    let mut common_revision: Option<String> = None;
    for (citation, _) in TARGETS {
        let resolved = resolve_live_oalc_exact_source(&OalcExactSourceRequest {
            citation: citation.to_string(),
            citation_match: OalcCitationMatch::Exact,
            document_type: "primary_legislation".into(),
            source: Some("nsw_legislation".into()),
            jurisdiction: Some("new_south_wales".into()),
        })
        .map_err(|error| format!("acquire {citation}: {error:?}"))?;
        let corpus_revision = format!(
            "isaacus/open-australian-legal-corpus@{}",
            resolved.corpus_revision_sha
        );
        if let Some(existing) = common_revision.as_deref() {
            if existing != corpus_revision {
                return Err(format!(
                    "Cullen source acquisition crossed OALC revisions: {existing} vs {corpus_revision}"
                ));
            }
        } else {
            common_revision = Some(corpus_revision.clone());
        }

        let row = resolved.row;
        if row.citation != citation || row.text.trim().is_empty() {
            return Err(format!("OALC returned wrong or empty record for {citation}"));
        }
        let digest = sha256(row.text.as_bytes());
        let artifact = materialised.join(format!("{}.txt", slug(citation)));
        write_readonly(&artifact, row.text.as_bytes())?;
        parents.push(ParentDocument {
            citation: citation.to_string(),
            version_id: row.version_id,
            corpus_revision,
            source: row.source,
            jurisdiction: row.jurisdiction,
            document_type: row.document_type,
            date: row.date,
            url: row.url,
            when_scraped: row.when_scraped,
            canonical_text_digest: digest,
            artifact,
            resolution_path: resolved.resolution_path,
            network_requests: resolved.network_requests,
            text: row.text,
        });
    }

    let receipt_path = materialised.join("oalc_legislation_receipts.tsv");
    let mut receipt = String::from(
        "citation\tversion_id\tcorpus_revision\tsource\tjurisdiction\ttype\tdate\turl\twhen_scraped\tcanonical_text_digest\tlocal_artifact_ref\ttemporal_status\tresolution_path\tnetwork_requests\treceipt_authority\n",
    );
    for parent in &parents {
        let fields = [
            parent.citation.clone(),
            parent.version_id.clone(),
            parent.corpus_revision.clone(),
            parent.source.clone(),
            parent.jurisdiction.clone(),
            parent.document_type.clone(),
            parent.date.clone().unwrap_or_default(),
            parent.url.clone().unwrap_or_default(),
            parent.when_scraped.clone().unwrap_or_default(),
            parent.canonical_text_digest.clone(),
            parent.artifact.to_string_lossy().into_owned(),
            "latest_known_only".into(),
            parent.resolution_path.clone(),
            parent.network_requests.to_string(),
            "experimental_candidate_only".into(),
        ];
        receipt.push_str(
            &fields
                .iter()
                .map(|value| value.replace(['\t', '\r', '\n'], " "))
                .collect::<Vec<_>>()
                .join("\t"),
        );
        receipt.push('\n');
    }
    fs::write(&receipt_path, receipt)
        .map_err(|error| format!("write {}: {error}", receipt_path.display()))?;
    Ok(parents)
}

fn run_parser_and_pnf(
    root: &Path,
    spacy_model: &str,
    revision_id: u64,
    slice_path: &Path,
    parser_path: &Path,
    parser_err_path: &Path,
    pnf_path: &Path,
    pnf_err_path: &Path,
) -> CliResult {
    let parser_out = fs::File::create(parser_path)
        .map_err(|error| format!("create {}: {error}", parser_path.display()))?;
    let parser_err = fs::File::create(parser_err_path)
        .map_err(|error| format!("create {}: {error}", parser_err_path.display()))?;
    let status = Command::new("python3")
        .arg(root.join("python/spacy_stream.py"))
        .arg("--model")
        .arg(spacy_model)
        .arg("--revision-id")
        .arg(revision_id.to_string())
        .arg(slice_path)
        .stdout(Stdio::from(parser_out))
        .stderr(Stdio::from(parser_err))
        .status()
        .map_err(|error| format!("run spaCy producer: {error}"))?;
    if !status.success() {
        return Err(format!(
            "spaCy producer failed for {} with {status}",
            slice_path.display()
        ));
    }

    let parser_in = fs::File::open(parser_path)
        .map_err(|error| format!("open {}: {error}", parser_path.display()))?;
    let pnf_out = fs::File::create(pnf_path)
        .map_err(|error| format!("create {}: {error}", pnf_path.display()))?;
    let pnf_err = fs::File::create(pnf_err_path)
        .map_err(|error| format!("create {}: {error}", pnf_err_path.display()))?;
    let status = Command::new("cargo")
        .current_dir(root)
        .args(["run", "-q", "-p", "sensiblaw-stream"])
        .stdin(Stdio::from(parser_in))
        .stdout(Stdio::from(pnf_out))
        .stderr(Stdio::from(pnf_err))
        .status()
        .map_err(|error| format!("run sensiblaw-stream: {error}"))?;
    if !status.success() {
        return Err(format!(
            "sensiblaw-stream failed for {} with {status}",
            slice_path.display()
        ));
    }
    Ok(())
}

fn repo_root() -> CliResult<PathBuf> {
    let mut path = std::env::current_dir()
        .map_err(|error| format!("read current directory: {error}"))?;
    loop {
        if path.join("Cargo.toml").is_file() && path.join("crates").is_dir() {
            return Ok(path);
        }
        if !path.pop() {
            return Err("could not locate SensibLaw workspace root".into());
        }
    }
}

pub fn run(args: Vec<String>) -> CliResult {
    if args.first().map(String::as_str) != Some("pnf") {
        return Err(
            "usage: sensiblaw legal-follow cullen pnf --operator-opt-in [--output-dir PATH] [--spacy-model MODEL]"
                .into(),
        );
    }
    if !args.iter().any(|arg| arg == "--operator-opt-in") {
        return Err("--operator-opt-in is required".into());
    }
    let output = value(&args, "--output-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("artifacts/oalc/cullen-governing-law"));
    let spacy_model = value(&args, "--spacy-model")
        .unwrap_or_else(|| "en_core_web_sm".into());

    let materialised = output.join("materialised");
    let slices_dir = output.join("slices");
    let parser_dir = output.join("parser");
    let pnf_dir = output.join("pnf");
    for directory in [&materialised, &slices_dir, &parser_dir, &pnf_dir] {
        fs::create_dir_all(directory)
            .map_err(|error| format!("create {}: {error}", directory.display()))?;
    }

    let parents = acquire_parent_documents(&materialised)?;
    let parent_by_citation = parents
        .iter()
        .map(|parent| (parent.citation.as_str(), parent))
        .collect::<BTreeMap<_, _>>();
    let root = repo_root()?;

    let mut receipt_rows = Vec::new();
    let mut seen = BTreeSet::new();
    let mut revision_id = 1000u64;

    for (citation, sections) in TARGETS {
        let parent = parent_by_citation
            .get(citation)
            .ok_or_else(|| format!("missing acquired parent {citation}"))?;
        let spans = section_spans(&parent.text);
        for section in sections {
            let (start, end) = spans.get(*section).copied().ok_or_else(|| {
                format!(
                    "could not uniquely locate section {section} in {citation}; preserve as section-boundary residual"
                )
            })?;
            let mut section_text = parent.text[start..end].trim_end().to_string();
            section_text.push('\n');
            let slice_digest = sha256(section_text.as_bytes());
            let file_slug = slug(citation);
            let slice_path = slices_dir.join(format!("{file_slug}-s{section}.txt"));
            fs::write(&slice_path, section_text.as_bytes())
                .map_err(|error| format!("write {}: {error}", slice_path.display()))?;

            let parser_path = parser_dir.join(format!("{file_slug}-s{section}.tsv"));
            let parser_err = parser_dir.join(format!("{file_slug}-s{section}.stderr.txt"));
            let pnf_path = pnf_dir.join(format!("{file_slug}-s{section}.receipts.tsv"));
            let pnf_err = pnf_dir.join(format!("{file_slug}-s{section}.stderr.txt"));

            run_parser_and_pnf(
                &root,
                &spacy_model,
                revision_id,
                &slice_path,
                &parser_path,
                &parser_err,
                &pnf_path,
                &pnf_err,
            )?;

            let slice_receipt = OalcSectionSliceReceipt {
                citation: citation.to_string(),
                section: section.to_string(),
                parent_version_id: parent.version_id.clone(),
                corpus_revision_ref: parent.corpus_revision.clone(),
                parent_digest: parent.canonical_text_digest.clone(),
                slice_start: start,
                slice_end: end,
                slice_digest: slice_digest.clone(),
                slice_artifact_ref: slice_path.clone(),
                temporal_coverage: OalcTemporalCoverage::LatestKnownOnly,
                parser_authority: OALC_PARSER_AUTHORITY,
            };
            if !slice_receipt.parser_eligible()
                || slice_receipt.pays_in_force_on("2017-01-26")
                || slice_receipt.creates_atomic_gate()
            {
                return Err(format!(
                    "section receipt violated parser/temporal boundary for {citation} s{section}"
                ));
            }
            seen.insert((citation.to_string(), section.to_string()));
            receipt_rows.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\tlatest_known_only\t{}",
                citation,
                section,
                parent.version_id,
                parent.corpus_revision,
                parent.canonical_text_digest,
                start,
                end,
                slice_digest,
                slice_path.display(),
                parser_path.display(),
                pnf_path.display(),
                OALC_PARSER_AUTHORITY,
            ));
            revision_id += 1;
        }
    }

    let expected = TARGETS
        .iter()
        .flat_map(|(citation, sections)| {
            sections
                .iter()
                .map(move |section| (citation.to_string(), section.to_string()))
        })
        .collect::<BTreeSet<_>>();
    if seen != expected {
        return Err(format!(
            "Cullen section set mismatch: expected {expected:?}, got {seen:?}"
        ));
    }

    let receipt_path = output.join("oalc_section_pnf_receipts.tsv");
    let mut file = fs::File::create(&receipt_path)
        .map_err(|error| format!("create {}: {error}", receipt_path.display()))?;
    writeln!(
        file,
        "citation\tsection\tparent_version_id\tcorpus_revision\tparent_digest\tslice_start\tslice_end\tslice_digest\tslice_artifact\tparser_artifact\tpnf_artifact\ttemporal_status\tparser_authority"
    )
    .map_err(|error| format!("write receipt header: {error}"))?;
    for row in receipt_rows {
        writeln!(file, "{row}").map_err(|error| format!("write receipt row: {error}"))?;
    }

    println!(
        "cullen_oalc_pnf={} sections={} temporal_status=latest_known_only parser_authority={}",
        receipt_path.display(),
        seen.len(),
        OALC_PARSER_AUTHORITY
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_heading_parser_matches_number_and_suffix_only() {
        assert_eq!(section_token("5B General principles"), Some("5B"));
        assert_eq!(section_token("  43A Proceedings against public authority"), Some("43A"));
        assert_eq!(section_token("Definitions"), None);
        assert_eq!(section_token("5B"), None);
        assert_eq!(section_token("5AB Invalid"), None);
        assert_eq!(section_token("5B2 Invalid"), None);
    }

    #[test]
    fn duplicate_section_headings_are_not_silently_selected() {
        let text = "5B First\nbody\n5C Next\nbody\n5B Duplicate\nbody\n";
        let spans = section_spans(text);
        assert!(!spans.contains_key("5B"));
        assert!(spans.contains_key("5C"));
    }
}
