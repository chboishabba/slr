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
