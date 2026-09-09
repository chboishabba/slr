use std::env;
use std::path::{Path, PathBuf};

use postgres::{Client, NoTls, Transaction};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    ProcessEnvironment,
    ExplicitEnvFile(PathBuf),
    LocalDotEnv(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    database_url: String,
    pub source: ConfigSource,
}

impl DatabaseConfig {
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    pub fn redacted_description(&self) -> &'static str {
        "DATABASE_URL configured"
    }
}

#[derive(Debug, Error)]
pub enum SourceStoreError {
    #[error("DATABASE_URL is not configured")]
    MissingDatabaseUrl,
    #[error("explicit env file does not exist: {0}")]
    MissingExplicitEnvFile(PathBuf),
    #[error("failed to load env file: {0}")]
    EnvFile(String),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("migration 181 OALC persistence tables are not installed")]
    Migration181Missing,
    #[error("invalid source slice {locator_ref}: [{start_char},{end_char}) for document length {document_len}")]
    InvalidSlice {
        locator_ref: String,
        start_char: usize,
        end_char: usize,
        document_len: usize,
    },
    #[error("slice digest mismatch for {0}")]
    SliceDigestMismatch(String),
}

pub fn load_database_config(explicit_env_file: Option<&Path>) -> Result<DatabaseConfig, SourceStoreError> {
    if let Ok(value) = env::var("DATABASE_URL") {
        if !value.trim().is_empty() {
            return Ok(DatabaseConfig {
                database_url: value,
                source: ConfigSource::ProcessEnvironment,
            });
        }
    }

    if let Some(path) = explicit_env_file {
        if !path.exists() {
            return Err(SourceStoreError::MissingExplicitEnvFile(path.to_path_buf()));
        }
        dotenvy::from_path(path).map_err(|error| SourceStoreError::EnvFile(error.to_string()))?;
        let value = env::var("DATABASE_URL").map_err(|_| SourceStoreError::MissingDatabaseUrl)?;
        if value.trim().is_empty() {
            return Err(SourceStoreError::MissingDatabaseUrl);
        }
        return Ok(DatabaseConfig {
            database_url: value,
            source: ConfigSource::ExplicitEnvFile(path.to_path_buf()),
        });
    }

    let local = PathBuf::from(".env");
    if local.exists() {
        dotenvy::from_path(&local).map_err(|error| SourceStoreError::EnvFile(error.to_string()))?;
        let value = env::var("DATABASE_URL").map_err(|_| SourceStoreError::MissingDatabaseUrl)?;
        if value.trim().is_empty() {
            return Err(SourceStoreError::MissingDatabaseUrl);
        }
        return Ok(DatabaseConfig {
            database_url: value,
            source: ConfigSource::LocalDotEnv(local),
        });
    }

    Err(SourceStoreError::MissingDatabaseUrl)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalCoverage {
    LatestKnownOnly,
    HistoricallyVerified,
}

impl TemporalCoverage {
    fn as_db(self) -> &'static str {
        match self {
            Self::LatestKnownOnly => "latest_known_only",
            Self::HistoricallyVerified => "historically_verified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionPath {
    FilterExact,
    NativeParquetScan,
    OfflineJsonlReplay,
    RevisionPinnedStreamingLegacy,
}

impl ResolutionPath {
    fn as_db(self) -> &'static str {
        match self {
            Self::FilterExact => "filter_exact",
            Self::NativeParquetScan => "native_parquet_scan",
            Self::OfflineJsonlReplay => "offline_jsonl_replay",
            Self::RevisionPinnedStreamingLegacy => "revision_pinned_streaming_legacy",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedExternalDocument<'a> {
    pub provider_ref: &'a str,
    pub dataset_ref: &'a str,
    pub dataset_revision_ref: &'a str,
    pub external_version_ref: &'a str,
    pub citation: &'a str,
    pub source_ref: &'a str,
    pub jurisdiction_ref: &'a str,
    pub document_type_ref: &'a str,
    pub temporal_coverage: TemporalCoverage,
    pub resolution_path: ResolutionPath,
    pub source_url: Option<&'a str>,
    pub canonical_text: &'a str,
}

#[derive(Debug, Clone)]
pub struct ExactResolutionReceipt<'a> {
    pub demand_ref: &'a str,
    pub consumer_ref: Option<&'a str>,
    pub requested_citation: &'a str,
    pub requested_jurisdiction_ref: &'a str,
    pub requested_source_role_ref: &'a str,
    pub requested_authority_level_ref: &'a str,
    pub requested_temporal_ref: Option<&'a str>,
    pub exact_demand_match: bool,
    pub acquisition_authority_ref: &'a str,
    pub receipt_authority_ref: &'a str,
    pub network_request_count: i64,
    pub resolver_ref: &'a str,
    pub resolution_evidence_ref: &'a str,
}

#[derive(Debug, Clone)]
pub struct SourceSlice<'a> {
    pub locator_ref: &'a str,
    pub start_char: usize,
    pub end_char: usize,
    pub projection_ref: &'a str,
    pub slice_sha256_hex: &'a str,
    pub parser_authority_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedSourceRefs {
    pub document_ref: String,
    pub external_source_revision_ref: String,
    pub source_resolution_ref: String,
    pub source_slice_refs: Vec<String>,
}

pub struct PostgresSourceStore {
    client: Client,
}

impl PostgresSourceStore {
    pub fn connect(config: &DatabaseConfig) -> Result<Self, SourceStoreError> {
        Ok(Self {
            client: Client::connect(config.database_url(), NoTls)?,
        })
    }

    pub fn ensure_schema_ready(&mut self) -> Result<(), SourceStoreError> {
        let row = self.client.query_one(
            "SELECT to_regclass('corpus.external_source_revision')::text",
            &[],
        )?;
        let table: Option<String> = row.get(0);
        if table.is_none() {
            return Err(SourceStoreError::Migration181Missing);
        }
        Ok(())
    }

    pub fn persist_resolved_source(
        &mut self,
        document: &ResolvedExternalDocument<'_>,
        resolution: &ExactResolutionReceipt<'_>,
        slices: &[SourceSlice<'_>],
    ) -> Result<PersistedSourceRefs, SourceStoreError> {
        self.ensure_schema_ready()?;
        validate_slices(document.canonical_text, slices)?;

        let mut tx = self.client.transaction()?;
        let content_digest = sha256_bytes(document.canonical_text.as_bytes());
        let content_hex = hex(&content_digest);
        let canonical_ref = format!("canonical:sha256:{content_hex}");
        let document_ref = format!("document:sha256:{content_hex}");

        tx.execute(
            "INSERT INTO corpus.canonical_content \
             (canonical_ref, content_sha256, encoding_ref, normalization_ref, compression_ref, payload, uncompressed_byte_length) \
             VALUES ($1,$2,'utf-8','identity',NULL,$3,$4) \
             ON CONFLICT (content_sha256, encoding_ref, normalization_ref) DO NOTHING",
            &[&canonical_ref, &&content_digest[..], &document.canonical_text.as_bytes(), &(document.canonical_text.len() as i64)],
        )?;

        let canonical_ref: String = tx.query_one(
            "SELECT canonical_ref FROM corpus.canonical_content \
             WHERE content_sha256=$1 AND encoding_ref='utf-8' AND normalization_ref='identity'",
            &[&&content_digest[..]],
        )?.get(0);

        tx.execute(
            "INSERT INTO corpus.document \
             (document_ref, source_content_ref, canonical_ref, media_type, adapter_ref, adapter_version, compiler_context_ref, document_sha256) \
             VALUES ($1,NULL,$2,'text/plain','oalc-governed-provider','1','sensiblaw-oalc',$3) \
             ON CONFLICT (document_sha256) DO NOTHING",
            &[&document_ref, &canonical_ref, &&content_digest[..]],
        )?;

        let document_ref: String = tx.query_one(
            "SELECT document_ref FROM corpus.document WHERE document_sha256=$1",
            &[&&content_digest[..]],
        )?.get(0);

        let revision_identity = format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}",
            document.provider_ref,
            document.dataset_ref,
            document.dataset_revision_ref,
            document.external_version_ref,
            document.citation,
            document.jurisdiction_ref,
            content_hex,
        );
        let revision_digest = sha256_bytes(revision_identity.as_bytes());
        let revision_ref = format!("external-source-revision:sha256:{}", hex(&revision_digest));

        tx.execute(
            "INSERT INTO corpus.external_source_revision \
             (external_source_revision_ref, document_ref, provider_ref, dataset_ref, dataset_revision_ref, external_version_ref, citation, source_ref, jurisdiction_ref, document_type_ref, temporal_coverage_ref, resolution_path_ref, source_url, receipt_sha256) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) \
             ON CONFLICT (provider_ref,dataset_ref,dataset_revision_ref,external_version_ref,citation,jurisdiction_ref) DO NOTHING",
            &[&revision_ref, &document_ref, &document.provider_ref, &document.dataset_ref, &document.dataset_revision_ref, &document.external_version_ref, &document.citation, &document.source_ref, &document.jurisdiction_ref, &document.document_type_ref, &document.temporal_coverage.as_db(), &document.resolution_path.as_db(), &document.source_url, &&revision_digest[..]],
        )?;

        let revision_ref: String = tx.query_one(
            "SELECT external_source_revision_ref FROM corpus.external_source_revision \
             WHERE provider_ref=$1 AND dataset_ref=$2 AND dataset_revision_ref=$3 \
               AND external_version_ref=$4 AND citation=$5 AND jurisdiction_ref=$6",
            &[&document.provider_ref, &document.dataset_ref, &document.dataset_revision_ref, &document.external_version_ref, &document.citation, &document.jurisdiction_ref],
        )?.get(0);

        let resolution_identity = format!(
            "{}\n{}\n{}\n{}\n{}",
            resolution.demand_ref,
            revision_ref,
            resolution.resolver_ref,
            resolution.resolution_evidence_ref,
            resolution.exact_demand_match,
        );
        let resolution_digest = sha256_bytes(resolution_identity.as_bytes());
        let resolution_ref = format!("source-resolution:sha256:{}", hex(&resolution_digest));
        tx.execute(
            "INSERT INTO evidence.external_source_resolution \
             (source_resolution_ref, external_source_revision_ref, demand_ref, consumer_ref, requested_citation, requested_jurisdiction_ref, requested_source_role_ref, requested_authority_level_ref, requested_temporal_ref, exact_demand_match, acquisition_authority_ref, receipt_authority_ref, network_request_count, resolver_ref, resolution_evidence_ref, receipt_sha256) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) \
             ON CONFLICT (demand_ref, external_source_revision_ref) DO NOTHING",
            &[&resolution_ref, &revision_ref, &resolution.demand_ref, &resolution.consumer_ref, &resolution.requested_citation, &resolution.requested_jurisdiction_ref, &resolution.requested_source_role_ref, &resolution.requested_authority_level_ref, &resolution.requested_temporal_ref, &resolution.exact_demand_match, &resolution.acquisition_authority_ref, &resolution.receipt_authority_ref, &resolution.network_request_count, &resolution.resolver_ref, &resolution.resolution_evidence_ref, &&resolution_digest[..]],
        )?;

        let resolution_ref: String = tx.query_one(
            "SELECT source_resolution_ref FROM evidence.external_source_resolution \
             WHERE demand_ref=$1 AND external_source_revision_ref=$2",
            &[&resolution.demand_ref, &revision_ref],
        )?.get(0);

        let mut slice_refs = Vec::with_capacity(slices.len());
        for slice in slices {
            let slice_ref = persist_slice(&mut tx, &document_ref, &revision_ref, document.canonical_text, slice)?;
            slice_refs.push(slice_ref);
        }

        tx.commit()?;
        Ok(PersistedSourceRefs {
            document_ref,
            external_source_revision_ref: revision_ref,
            source_resolution_ref: resolution_ref,
            source_slice_refs: slice_refs,
        })
    }
}

fn persist_slice(
    tx: &mut Transaction<'_>,
    document_ref: &str,
    revision_ref: &str,
    document_text: &str,
    slice: &SourceSlice<'_>,
) -> Result<String, SourceStoreError> {
    let span_identity = format!("{document_ref}\n{}\n{}\n{}", slice.start_char, slice.end_char, slice.locator_ref);
    let span_digest = sha256_bytes(span_identity.as_bytes());
    let span_ref = format!("span:sha256:{}", hex(&span_digest));
    tx.execute(
        "INSERT INTO corpus.span (span_ref, document_ref, start_char, end_char, start_token, end_token, span_type_ref) \
         VALUES ($1,$2,$3,$4,NULL,NULL,'statutory_section') ON CONFLICT (span_ref) DO NOTHING",
        &[&span_ref, &document_ref, &(slice.start_char as i32), &(slice.end_char as i32)],
    )?;

    let actual = sha256_bytes(document_text[slice.start_char..slice.end_char].as_bytes());
    let expected = decode_hex_32(slice.slice_sha256_hex)
        .ok_or_else(|| SourceStoreError::SliceDigestMismatch(slice.locator_ref.to_owned()))?;
    if actual != expected {
        return Err(SourceStoreError::SliceDigestMismatch(slice.locator_ref.to_owned()));
    }

    let slice_identity = format!("{revision_ref}\n{}\n{span_ref}\n{}", slice.locator_ref, slice.projection_ref);
    let receipt_digest = sha256_bytes(slice_identity.as_bytes());
    let source_slice_ref = format!("source-slice:sha256:{}", hex(&receipt_digest));
    tx.execute(
        "INSERT INTO corpus.external_source_slice \
         (source_slice_ref, external_source_revision_ref, span_ref, locator_ref, projection_ref, slice_sha256, parser_authority_ref, receipt_sha256) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
         ON CONFLICT (external_source_revision_ref, locator_ref, span_ref) DO NOTHING",
        &[&source_slice_ref, &revision_ref, &span_ref, &slice.locator_ref, &slice.projection_ref, &&actual[..], &slice.parser_authority_ref, &&receipt_digest[..]],
    )?;

    Ok(tx.query_one(
        "SELECT source_slice_ref FROM corpus.external_source_slice \
         WHERE external_source_revision_ref=$1 AND locator_ref=$2 AND span_ref=$3",
        &[&revision_ref, &slice.locator_ref, &span_ref],
    )?.get(0))
}

fn validate_slices(document_text: &str, slices: &[SourceSlice<'_>]) -> Result<(), SourceStoreError> {
    for slice in slices {
        if slice.start_char >= slice.end_char
            || slice.end_char > document_text.len()
            || !document_text.is_char_boundary(slice.start_char)
            || !document_text.is_char_boundary(slice.end_char)
        {
            return Err(SourceStoreError::InvalidSlice {
                locator_ref: slice.locator_ref.to_owned(),
                start_char: slice.start_char,
                end_char: slice.end_char,
                document_len: document_text.len(),
            });
        }
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn decode_hex_32(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut out = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk).ok()?;
        out[index] = u8::from_str_radix(text, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        prior_database_url: Option<OsString>,
        prior_dir: PathBuf,
    }

    impl EnvGuard {
        fn capture() -> Self {
            Self {
                prior_database_url: env::var_os("DATABASE_URL"),
                prior_dir: env::current_dir().expect("cwd"),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prior_database_url {
                Some(value) => env::set_var("DATABASE_URL", value),
                None => env::remove_var("DATABASE_URL"),
            }
            let _ = env::set_current_dir(&self.prior_dir);
        }
    }

    #[test]
    fn process_database_url_wins_over_env_file() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture();
        env::set_var("DATABASE_URL", "postgresql://process/test");
        let dir = tempfile::tempdir().unwrap();
        let env_file = dir.path().join("explicit.env");
        std::fs::write(&env_file, "DATABASE_URL=postgresql://file/test\n").unwrap();

        let config = load_database_config(Some(&env_file)).unwrap();
        assert_eq!(config.database_url(), "postgresql://process/test");
        assert_eq!(config.source, ConfigSource::ProcessEnvironment);
    }

    #[test]
    fn explicit_env_file_is_used_when_process_value_is_absent() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture();
        env::remove_var("DATABASE_URL");
        let dir = tempfile::tempdir().unwrap();
        let env_file = dir.path().join("explicit.env");
        std::fs::write(&env_file, "DATABASE_URL=postgresql://explicit/test\n").unwrap();

        let config = load_database_config(Some(&env_file)).unwrap();
        assert_eq!(config.database_url(), "postgresql://explicit/test");
        assert_eq!(config.source, ConfigSource::ExplicitEnvFile(env_file));
    }

    #[test]
    fn local_dotenv_is_used_when_no_explicit_source_exists() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture();
        env::remove_var("DATABASE_URL");
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env"), "DATABASE_URL=postgresql://local/test\n").unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let config = load_database_config(None).unwrap();
        assert_eq!(config.database_url(), "postgresql://local/test");
        assert_eq!(config.source, ConfigSource::LocalDotEnv(PathBuf::from(".env")));
    }

    #[test]
    fn missing_explicit_file_fails_closed() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture();
        env::remove_var("DATABASE_URL");
        let result = load_database_config(Some(Path::new("/definitely/not/present/.env")));
        assert!(matches!(result, Err(SourceStoreError::MissingExplicitEnvFile(_))));
    }

    #[test]
    fn slice_validation_rejects_non_boundaries_and_out_of_range() {
        let bad = SourceSlice {
            locator_ref: "s5B",
            start_char: 0,
            end_char: 100,
            projection_ref: "source-preserving",
            slice_sha256_hex: &"00".repeat(32),
            parser_authority_ref: "source_observation_only",
        };
        assert!(validate_slices("short", &[bad]).is_err());
    }
}
