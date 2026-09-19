use crate::{
    ExactResolutionReceipt, PostgresSourceStore, ResolvedExternalDocument, ResolutionPath,
    SourceStoreError, TemporalCoverage,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheLookupDemand<'a> {
    pub demand_ref: &'a str,
    pub citation: &'a str,
    pub jurisdiction_ref: &'a str,
    pub source_role_ref: &'a str,
    pub authority_level_ref: &'a str,
    pub temporal_ref: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedResolvedDocument {
    pub document_ref: String,
    pub external_source_revision_ref: String,
    pub source_resolution_ref: String,
    pub provider_ref: String,
    pub dataset_ref: String,
    pub dataset_revision_ref: String,
    pub external_version_ref: String,
    pub citation: String,
    pub source_ref: String,
    pub jurisdiction_ref: String,
    pub document_type_ref: String,
    pub temporal_coverage: TemporalCoverage,
    pub resolution_path: ResolutionPath,
    pub source_url: Option<String>,
    pub canonical_text: String,
}

#[derive(Debug)]
pub enum CacheLookupError {
    Postgres(postgres::Error),
    InvalidTemporalCoverage(String),
    InvalidResolutionPath(String),
    InvalidCanonicalText(std::string::FromUtf8Error),
}

impl From<postgres::Error> for CacheLookupError {
    fn from(value: postgres::Error) -> Self {
        Self::Postgres(value)
    }
}

#[derive(Debug, Clone)]
pub struct AcquiredSourceBundle {
    pub document: ResolvedExternalDocumentOwned,
    pub receipt: ExactResolutionReceiptOwned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExternalDocumentOwned {
    pub provider_ref: String,
    pub dataset_ref: String,
    pub dataset_revision_ref: String,
    pub external_version_ref: String,
    pub citation: String,
    pub source_ref: String,
    pub jurisdiction_ref: String,
    pub document_type_ref: String,
    pub temporal_coverage: TemporalCoverage,
    pub resolution_path: ResolutionPath,
    pub source_url: Option<String>,
    pub canonical_text: String,
}

impl ResolvedExternalDocumentOwned {
    fn borrowed(&self) -> ResolvedExternalDocument<'_> {
        ResolvedExternalDocument {
            provider_ref: &self.provider_ref,
            dataset_ref: &self.dataset_ref,
            dataset_revision_ref: &self.dataset_revision_ref,
            external_version_ref: &self.external_version_ref,
            citation: &self.citation,
            source_ref: &self.source_ref,
            jurisdiction_ref: &self.jurisdiction_ref,
            document_type_ref: &self.document_type_ref,
            temporal_coverage: self.temporal_coverage,
            resolution_path: self.resolution_path,
            source_url: self.source_url.as_deref(),
            canonical_text: &self.canonical_text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactResolutionReceiptOwned {
    pub demand_ref: String,
    pub consumer_ref: Option<String>,
    pub requested_citation: String,
    pub requested_jurisdiction_ref: String,
    pub requested_source_role_ref: String,
    pub requested_authority_level_ref: String,
    pub requested_temporal_ref: Option<String>,
    pub exact_demand_match: bool,
    pub acquisition_authority_ref: String,
    pub receipt_authority_ref: String,
    pub network_request_count: i64,
    pub resolver_ref: String,
    pub resolution_evidence_ref: String,
}

impl ExactResolutionReceiptOwned {
    fn borrowed(&self) -> ExactResolutionReceipt<'_> {
        ExactResolutionReceipt {
            demand_ref: &self.demand_ref,
            consumer_ref: self.consumer_ref.as_deref(),
            requested_citation: &self.requested_citation,
            requested_jurisdiction_ref: &self.requested_jurisdiction_ref,
            requested_source_role_ref: &self.requested_source_role_ref,
            requested_authority_level_ref: &self.requested_authority_level_ref,
            requested_temporal_ref: self.requested_temporal_ref.as_deref(),
            exact_demand_match: self.exact_demand_match,
            acquisition_authority_ref: &self.acquisition_authority_ref,
            receipt_authority_ref: &self.receipt_authority_ref,
            network_request_count: self.network_request_count,
            resolver_ref: &self.resolver_ref,
            resolution_evidence_ref: &self.resolution_evidence_ref,
        }
    }
}

pub trait CacheFirstAcquirer {
    type Error;

    fn acquire(
        &mut self,
        demand: &CacheLookupDemand<'_>,
    ) -> Result<AcquiredSourceBundle, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheFirstResolution {
    PgHit {
        source: CachedResolvedDocument,
        network_requests: u64,
    },
    AcquiredPersisted {
        source: CachedResolvedDocument,
        acquisition_network_requests: u64,
        verification_network_requests: u64,
    },
}

#[derive(Debug)]
pub enum CacheFirstError<E> {
    Lookup(CacheLookupError),
    Store(SourceStoreError),
    Acquire(E),
    PersistenceVerificationMiss,
    InvalidNetworkRequestCount(i64),
}

impl<E> From<SourceStoreError> for CacheFirstError<E> {
    fn from(value: SourceStoreError) -> Self {
        Self::Store(value)
    }
}

impl<E> From<CacheLookupError> for CacheFirstError<E> {
    fn from(value: CacheLookupError) -> Self {
        Self::Lookup(value)
    }
}

impl PostgresSourceStore {
    pub fn lookup_exact_source(
        &mut self,
        demand: &CacheLookupDemand<'_>,
    ) -> Result<Option<CachedResolvedDocument>, CacheLookupError> {
        let row = self.client.query_opt(
            "SELECT
                 revision.document_ref,
                 revision.external_source_revision_ref,
                 resolution.source_resolution_ref,
                 revision.provider_ref,
                 revision.dataset_ref,
                 revision.dataset_revision_ref,
                 revision.external_version_ref,
                 revision.citation,
                 revision.source_ref,
                 revision.jurisdiction_ref,
                 revision.document_type_ref,
                 revision.temporal_coverage_ref,
                 revision.resolution_path_ref,
                 revision.source_url,
                 canonical.payload
             FROM evidence.external_source_resolution AS resolution
             JOIN corpus.external_source_revision AS revision
               ON revision.external_source_revision_ref = resolution.external_source_revision_ref
             JOIN corpus.document AS document
               ON document.document_ref = revision.document_ref
             JOIN corpus.canonical_content AS canonical
               ON canonical.canonical_ref = document.canonical_ref
            WHERE resolution.demand_ref = $1
              AND resolution.requested_citation = $2
              AND resolution.requested_jurisdiction_ref = $3
              AND resolution.requested_source_role_ref = $4
              AND resolution.requested_authority_level_ref = $5
              AND resolution.requested_temporal_ref IS NOT DISTINCT FROM $6
              AND resolution.exact_demand_match = TRUE
            ORDER BY resolution.created_at DESC
            LIMIT 1",
            &[
                &demand.demand_ref,
                &demand.citation,
                &demand.jurisdiction_ref,
                &demand.source_role_ref,
                &demand.authority_level_ref,
                &demand.temporal_ref,
            ],
        )?;

        let Some(row) = row else {
            return Ok(None);
        };

        let temporal_raw: String = row.get(11);
        let temporal_coverage = match temporal_raw.as_str() {
            "latest_known_only" => TemporalCoverage::LatestKnownOnly,
            "historically_verified" => TemporalCoverage::HistoricallyVerified,
            other => return Err(CacheLookupError::InvalidTemporalCoverage(other.to_owned())),
        };

        let path_raw: String = row.get(12);
        let resolution_path = match path_raw.as_str() {
            "filter_exact" => ResolutionPath::FilterExact,
            "native_parquet_scan" => ResolutionPath::NativeParquetScan,
            "offline_jsonl_replay" => ResolutionPath::OfflineJsonlReplay,
            "revision_pinned_streaming_legacy" => ResolutionPath::RevisionPinnedStreamingLegacy,
            other => return Err(CacheLookupError::InvalidResolutionPath(other.to_owned())),
        };

        let canonical_bytes: Vec<u8> = row.get(14);
        let canonical_text = String::from_utf8(canonical_bytes)
            .map_err(CacheLookupError::InvalidCanonicalText)?;

        Ok(Some(CachedResolvedDocument {
            document_ref: row.get(0),
            external_source_revision_ref: row.get(1),
            source_resolution_ref: row.get(2),
            provider_ref: row.get(3),
            dataset_ref: row.get(4),
            dataset_revision_ref: row.get(5),
            external_version_ref: row.get(6),
            citation: row.get(7),
            source_ref: row.get(8),
            jurisdiction_ref: row.get(9),
            document_type_ref: row.get(10),
            temporal_coverage,
            resolution_path,
            source_url: row.get(13),
            canonical_text,
        }))
    }
}

pub fn resolve_cache_first<A: CacheFirstAcquirer>(
    store: &mut PostgresSourceStore,
    acquirer: &mut A,
    demand: &CacheLookupDemand<'_>,
) -> Result<CacheFirstResolution, CacheFirstError<A::Error>> {
    if let Some(source) = store.lookup_exact_source(demand)? {
        return Ok(CacheFirstResolution::PgHit {
            source,
            network_requests: 0,
        });
    }

    let acquired = acquirer.acquire(demand).map_err(CacheFirstError::Acquire)?;
    if acquired.receipt.network_request_count < 0 {
        return Err(CacheFirstError::InvalidNetworkRequestCount(
            acquired.receipt.network_request_count,
        ));
    }
    let acquisition_network_requests = acquired.receipt.network_request_count as u64;

    let document = acquired.document.borrowed();
    let receipt = acquired.receipt.borrowed();
    store.persist_resolved_source(&document, &receipt, &[])?;

    let Some(source) = store.lookup_exact_source(demand)? else {
        return Err(CacheFirstError::PersistenceVerificationMiss);
    };

    Ok(CacheFirstResolution::AcquiredPersisted {
        source,
        acquisition_network_requests,
        verification_network_requests: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CountingAcquirer {
        calls: usize,
    }

    impl CacheFirstAcquirer for CountingAcquirer {
        type Error = &'static str;

        fn acquire(
            &mut self,
            demand: &CacheLookupDemand<'_>,
        ) -> Result<AcquiredSourceBundle, Self::Error> {
            self.calls += 1;
            Ok(AcquiredSourceBundle {
                document: ResolvedExternalDocumentOwned {
                    provider_ref: "provider:oalc".into(),
                    dataset_ref: "isaacus/open-australian-legal-corpus".into(),
                    dataset_revision_ref: "sha:fixture".into(),
                    external_version_ref: "version:fixture".into(),
                    citation: demand.citation.into(),
                    source_ref: "nsw_legislation".into(),
                    jurisdiction_ref: demand.jurisdiction_ref.into(),
                    document_type_ref: "primary_legislation".into(),
                    temporal_coverage: TemporalCoverage::LatestKnownOnly,
                    resolution_path: ResolutionPath::NativeParquetScan,
                    source_url: None,
                    canonical_text: "fixture legal text".into(),
                },
                receipt: ExactResolutionReceiptOwned {
                    demand_ref: demand.demand_ref.into(),
                    consumer_ref: Some("consumer:fixture".into()),
                    requested_citation: demand.citation.into(),
                    requested_jurisdiction_ref: demand.jurisdiction_ref.into(),
                    requested_source_role_ref: demand.source_role_ref.into(),
                    requested_authority_level_ref: demand.authority_level_ref.into(),
                    requested_temporal_ref: demand.temporal_ref.map(str::to_owned),
                    exact_demand_match: true,
                    acquisition_authority_ref: "governed-provider".into(),
                    receipt_authority_ref: "source-observation-only".into(),
                    network_request_count: 1,
                    resolver_ref: "fixture-acquirer".into(),
                    resolution_evidence_ref: "fixture-evidence".into(),
                },
            })
        }
    }

    #[test]
    fn owned_receipt_roundtrips_into_borrowed_contract() {
        let receipt = ExactResolutionReceiptOwned {
            demand_ref: "d".into(),
            consumer_ref: None,
            requested_citation: "c".into(),
            requested_jurisdiction_ref: "AU-NSW".into(),
            requested_source_role_ref: "primary_legislation".into(),
            requested_authority_level_ref: "official".into(),
            requested_temporal_ref: Some("latest_known_only".into()),
            exact_demand_match: true,
            acquisition_authority_ref: "a".into(),
            receipt_authority_ref: "r".into(),
            network_request_count: 2,
            resolver_ref: "resolver".into(),
            resolution_evidence_ref: "evidence".into(),
        };
        let borrowed = receipt.borrowed();
        assert_eq!(borrowed.demand_ref, "d");
        assert_eq!(borrowed.network_request_count, 2);
        assert!(borrowed.exact_demand_match);
    }

    #[test]
    fn counting_acquirer_is_not_semantic_authority() {
        let demand = CacheLookupDemand {
            demand_ref: "demand:cullen:cla",
            citation: "Civil Liability Act 2002 (NSW)",
            jurisdiction_ref: "AU-NSW",
            source_role_ref: "primary_legislation",
            authority_level_ref: "official",
            temporal_ref: Some("latest_known_only"),
        };
        let mut acquirer = CountingAcquirer { calls: 0 };
        let bundle = acquirer.acquire(&demand).expect("fixture acquisition");
        assert_eq!(acquirer.calls, 1);
        assert_eq!(bundle.receipt.receipt_authority_ref, "source-observation-only");
    }
}
