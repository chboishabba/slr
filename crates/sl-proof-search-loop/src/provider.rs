//! Provider boundary for governed legal acquisition.
//!
//! This module owns contracts only. It deliberately contains no HTTP client and
//! performs no network I/O. Concrete AustLII/JADE adapters must implement this
//! boundary outside the scheduler and must return references from search and
//! bytes from fetch.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegalHostPacingPolicy {
    /// Number of requests allowed in each four-second legal-host window.
    pub requests_per_four_seconds: u32,
    pub burst_limit: u32,
}

impl Default for LegalHostPacingPolicy {
    fn default() -> Self {
        Self {
            requests_per_four_seconds: 1,
            burst_limit: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchBounds {
    pub max_depth: u32,
    pub max_new_documents: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernedSearchPlan {
    pub provider_ref: String,
    pub provider_query: String,
    pub pacing: LegalHostPacingPolicy,
    pub bounds: SearchBounds,
    pub cache_checked_first: bool,
    pub persisted_receipts_checked_first: bool,
    pub crawling_permitted: bool,
    pub ad_hoc_polling_permitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchReferenceReceipt {
    pub provider_ref: String,
    pub references: Vec<String>,
    pub network_requests: u64,
    pub pacing_observed: bool,
    pub bounds_observed: bool,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchBytesReceipt {
    pub provider_ref: String,
    pub reference: String,
    pub bytes: Vec<u8>,
    pub network_requests: u64,
    pub locally_ingested: bool,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderBoundaryError {
    MissingExplicitBounds,
    InvalidLegalHostPacing,
    CacheNotCheckedFirst,
    PersistedReceiptsNotCheckedFirst,
    CrawlingEnabled,
    AdHocPollingEnabled,
    SearchReceiptClaimsAuthority,
    FetchReceiptClaimsAuthority,
    FetchBypassedLocalIngestion,
}

pub fn validate_search_plan(plan: &GovernedSearchPlan) -> Result<(), ProviderBoundaryError> {
    if plan.bounds.max_depth == 0 || plan.bounds.max_new_documents == 0 {
        return Err(ProviderBoundaryError::MissingExplicitBounds);
    }
    if plan.pacing.requests_per_four_seconds != 1 || plan.pacing.burst_limit != 1 {
        return Err(ProviderBoundaryError::InvalidLegalHostPacing);
    }
    if !plan.cache_checked_first {
        return Err(ProviderBoundaryError::CacheNotCheckedFirst);
    }
    if !plan.persisted_receipts_checked_first {
        return Err(ProviderBoundaryError::PersistedReceiptsNotCheckedFirst);
    }
    if plan.crawling_permitted {
        return Err(ProviderBoundaryError::CrawlingEnabled);
    }
    if plan.ad_hoc_polling_permitted {
        return Err(ProviderBoundaryError::AdHocPollingEnabled);
    }
    Ok(())
}

pub fn validate_search_receipt(receipt: &SearchReferenceReceipt) -> Result<(), ProviderBoundaryError> {
    if receipt.receipt_authority != "experimental_candidate_only" {
        return Err(ProviderBoundaryError::SearchReceiptClaimsAuthority);
    }
    Ok(())
}

pub fn validate_fetch_receipt(receipt: &FetchBytesReceipt) -> Result<(), ProviderBoundaryError> {
    if receipt.receipt_authority != "experimental_candidate_only" {
        return Err(ProviderBoundaryError::FetchReceiptClaimsAuthority);
    }
    if !receipt.locally_ingested {
        return Err(ProviderBoundaryError::FetchBypassedLocalIngestion);
    }
    Ok(())
}

pub trait GovernedLegalProvider {
    type Error;

    /// Search returns references only. Implementations must enforce pacing and bounds.
    fn search(&mut self, plan: &GovernedSearchPlan) -> Result<SearchReferenceReceipt, Self::Error>;

    /// Fetch returns bytes for one explicit reference. Parser/PNF runs only after
    /// the resulting bytes have been locally ingested.
    fn fetch(&mut self, reference: &str) -> Result<FetchBytesReceipt, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> GovernedSearchPlan {
        GovernedSearchPlan {
            provider_ref: "austlii".into(),
            provider_query: "\"positive operational act\"".into(),
            pacing: LegalHostPacingPolicy::default(),
            bounds: SearchBounds {
                max_depth: 1,
                max_new_documents: 5,
            },
            cache_checked_first: true,
            persisted_receipts_checked_first: true,
            crawling_permitted: false,
            ad_hoc_polling_permitted: false,
        }
    }

    #[test]
    fn canonical_plan_pins_rate_bounds_and_no_crawl() {
        assert_eq!(validate_search_plan(&plan()), Ok(()));
    }

    #[test]
    fn missing_bounds_fail_closed() {
        let mut p = plan();
        p.bounds.max_new_documents = 0;
        assert_eq!(
            validate_search_plan(&p),
            Err(ProviderBoundaryError::MissingExplicitBounds)
        );
    }

    #[test]
    fn noncanonical_legal_host_rate_fails_closed() {
        let mut p = plan();
        p.pacing.requests_per_four_seconds = 2;
        assert_eq!(
            validate_search_plan(&p),
            Err(ProviderBoundaryError::InvalidLegalHostPacing)
        );
    }
}
