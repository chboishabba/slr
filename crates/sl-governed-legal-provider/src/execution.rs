use std::collections::BTreeSet;
use std::thread;
use std::time::{Duration, Instant};

use crate::query::{
    build_sino_url, is_austlii_url, is_jade_url, jade_search_url, SinoQuery,
    AUSTLII_BROWSER_UA, AUSTLII_REFERER, SENSIBLAW_UA,
};
use crate::resolver::{LegalProvider, LiveGovernanceBounds};
use crate::transport::{HttpRequest, HttpTransport};
use crate::RECEIPT_AUTHORITY;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceError {
    OperatorOptInRequired,
    CacheNotCheckedFirst,
    PersistedReceiptsNotCheckedFirst,
    InvalidBounds,
    RequestBudgetExceeded,
    InvalidProviderUrl,
    Transport(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernedExecutionContext {
    pub operator_opt_in: bool,
    pub cache_checked_first: bool,
    pub persisted_receipts_checked_first: bool,
    pub bounds: LiveGovernanceBounds,
}

impl GovernedExecutionContext {
    pub fn validate(&self) -> Result<(), GovernanceError> {
        if !self.operator_opt_in {
            return Err(GovernanceError::OperatorOptInRequired);
        }
        if !self.cache_checked_first {
            return Err(GovernanceError::CacheNotCheckedFirst);
        }
        if !self.persisted_receipts_checked_first {
            return Err(GovernanceError::PersistedReceiptsNotCheckedFirst);
        }
        if self.bounds.minimum_pacing_seconds < 4
            || self.bounds.burst != 1
            || self.bounds.max_depth == 0
            || self.bounds.max_new_documents == 0
            || self.bounds.max_network_requests == 0
        {
            return Err(GovernanceError::InvalidBounds);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchReference {
    pub url: String,
    pub title: Option<String>,
    pub citation: Option<String>,
    pub database_heading: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchReferenceReceipt {
    pub provider: LegalProvider,
    pub request_url: String,
    pub references: Vec<SearchReference>,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub network_requests: u64,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchBytesReceipt {
    pub provider: LegalProvider,
    pub reference: String,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub bytes: Vec<u8>,
    pub network_requests: u64,
    pub locally_ingested: bool,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalIngestionReceipt {
    pub provider: LegalProvider,
    pub source_identity_ref: String,
    pub source_revision_ref: String,
    pub explicit_reference: String,
    pub canonical_bytes_digest: String,
    pub locally_ingested: bool,
    pub network_requests_used_to_acquire: u64,
    pub receipt_authority: &'static str,
}

pub fn mark_locally_ingested(
    fetch: &FetchBytesReceipt,
    source_identity_ref: impl Into<String>,
    source_revision_ref: impl Into<String>,
    canonical_bytes_digest: impl Into<String>,
) -> LocalIngestionReceipt {
    LocalIngestionReceipt {
        provider: fetch.provider,
        source_identity_ref: source_identity_ref.into(),
        source_revision_ref: source_revision_ref.into(),
        explicit_reference: fetch.reference.clone(),
        canonical_bytes_digest: canonical_bytes_digest.into(),
        locally_ingested: true,
        network_requests_used_to_acquire: fetch.network_requests,
        receipt_authority: RECEIPT_AUTHORITY,
    }
}

fn normalize_reference(provider: LegalProvider, href: &str) -> Option<String> {
    let href = href.trim();
    match provider {
        LegalProvider::AustLII => {
            if is_austlii_url(href) {
                Some(href.to_string())
            } else if href.starts_with('/') {
                Some(format!("https://www.austlii.edu.au{href}"))
            } else {
                None
            }
        }
        LegalProvider::Jade => {
            if is_jade_url(href) {
                Some(href.to_string())
            } else if href.starts_with('/') {
                Some(format!("https://jade.io{href}"))
            } else {
                None
            }
        }
        LegalProvider::Local => None,
    }
}

pub fn extract_search_references(
    provider: LegalProvider,
    body: &[u8],
    max_results: usize,
) -> Vec<SearchReference> {
    let html = String::from_utf8_lossy(body);
    let bytes = html.as_bytes();
    let mut index = 0usize;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();

    while index + 6 < bytes.len() && out.len() < max_results {
        let tail = &html[index..];
        let Some(relative) = tail.find("href=") else { break };
        let start = index + relative + 5;
        let quote = bytes.get(start).copied();
        if quote != Some(b'\"') && quote != Some(b'\'') {
            index = start.saturating_add(1);
            continue;
        }
        let quote = quote.unwrap();
        let value_start = start + 1;
        let Some(end_relative) = bytes[value_start..].iter().position(|b| *b == quote) else {
            break;
        };
        let value_end = value_start + end_relative;
        let href = &html[value_start..value_end];
        if let Some(url) = normalize_reference(provider, href) {
            if seen.insert(url.clone()) {
                out.push(SearchReference {
                    url,
                    title: None,
                    citation: None,
                    database_heading: None,
                });
            }
        }
        index = value_end.saturating_add(1);
    }
    out
}

pub struct GovernedExecutor<T> {
    transport: T,
    context: GovernedExecutionContext,
    last_request: Option<Instant>,
    network_requests: u64,
}

impl<T: HttpTransport> GovernedExecutor<T>
where
    T::Error: ToString,
{
    pub fn new(transport: T, context: GovernedExecutionContext) -> Result<Self, GovernanceError> {
        context.validate()?;
        Ok(Self {
            transport,
            context,
            last_request: None,
            network_requests: 0,
        })
    }

    pub const fn network_requests(&self) -> u64 {
        self.network_requests
    }

    fn pace(&mut self) {
        if let Some(last) = self.last_request {
            let minimum = Duration::from_secs(self.context.bounds.minimum_pacing_seconds);
            let elapsed = last.elapsed();
            if elapsed < minimum {
                thread::sleep(minimum - elapsed);
            }
        }
    }

    fn governed_get(&mut self, request: HttpRequest) -> Result<crate::transport::HttpResponse, GovernanceError> {
        if self.network_requests >= self.context.bounds.max_network_requests {
            return Err(GovernanceError::RequestBudgetExceeded);
        }
        self.pace();
        let response = self
            .transport
            .get(&request)
            .map_err(|err| GovernanceError::Transport(err.to_string()))?;
        self.network_requests += 1;
        self.last_request = Some(Instant::now());
        Ok(response)
    }

    pub fn austlii_search(
        &mut self,
        query: &SinoQuery,
    ) -> Result<SearchReferenceReceipt, GovernanceError> {
        let url = build_sino_url(query);
        let response = self.governed_get(HttpRequest {
            url: url.clone(),
            user_agent: AUSTLII_BROWSER_UA.into(),
            referer: Some(AUSTLII_REFERER.into()),
            timeout_seconds: 45,
        })?;
        if !is_austlii_url(&response.final_url) {
            return Err(GovernanceError::InvalidProviderUrl);
        }
        let references = extract_search_references(
            LegalProvider::AustLII,
            &response.body,
            query.results as usize,
        );
        Ok(SearchReferenceReceipt {
            provider: LegalProvider::AustLII,
            request_url: url,
            references,
            status_code: response.status_code,
            content_type: response.content_type,
            network_requests: 1,
            receipt_authority: RECEIPT_AUTHORITY,
        })
    }

    pub fn jade_search(
        &mut self,
        term: &str,
        max_results: usize,
    ) -> Result<SearchReferenceReceipt, GovernanceError> {
        let url = jade_search_url(term);
        let response = self.governed_get(HttpRequest {
            url: url.clone(),
            user_agent: SENSIBLAW_UA.into(),
            referer: None,
            timeout_seconds: 45,
        })?;
        if !is_jade_url(&response.final_url) {
            return Err(GovernanceError::InvalidProviderUrl);
        }
        let references = extract_search_references(LegalProvider::Jade, &response.body, max_results);
        Ok(SearchReferenceReceipt {
            provider: LegalProvider::Jade,
            request_url: url,
            references,
            status_code: response.status_code,
            content_type: response.content_type,
            network_requests: 1,
            receipt_authority: RECEIPT_AUTHORITY,
        })
    }

    pub fn fetch_austlii(&mut self, reference: &str) -> Result<FetchBytesReceipt, GovernanceError> {
        if !is_austlii_url(reference) {
            return Err(GovernanceError::InvalidProviderUrl);
        }
        let response = self.governed_get(HttpRequest {
            url: reference.into(),
            user_agent: SENSIBLAW_UA.into(),
            referer: Some(AUSTLII_REFERER.into()),
            timeout_seconds: 45,
        })?;
        if !is_austlii_url(&response.final_url) {
            return Err(GovernanceError::InvalidProviderUrl);
        }
        Ok(FetchBytesReceipt {
            provider: LegalProvider::AustLII,
            reference: reference.into(),
            status_code: response.status_code,
            content_type: response.content_type,
            bytes: response.body,
            network_requests: 1,
            locally_ingested: false,
            receipt_authority: RECEIPT_AUTHORITY,
        })
    }

    pub fn fetch_jade(&mut self, reference: &str) -> Result<FetchBytesReceipt, GovernanceError> {
        if !is_jade_url(reference) {
            return Err(GovernanceError::InvalidProviderUrl);
        }
        let response = self.governed_get(HttpRequest {
            url: reference.into(),
            user_agent: SENSIBLAW_UA.into(),
            referer: None,
            timeout_seconds: 45,
        })?;
        if !is_jade_url(&response.final_url) {
            return Err(GovernanceError::InvalidProviderUrl);
        }
        Ok(FetchBytesReceipt {
            provider: LegalProvider::Jade,
            reference: reference.into(),
            status_code: response.status_code,
            content_type: response.content_type,
            bytes: response.body,
            network_requests: 1,
            locally_ingested: false,
            receipt_authority: RECEIPT_AUTHORITY,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{AUSTLII_SINO_ENDPOINT, SinoMethod};
    use crate::transport::HttpResponse;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct MockTransport {
        responses: Vec<HttpResponse>,
    }

    impl HttpTransport for MockTransport {
        type Error = String;

        fn get(&mut self, _request: &HttpRequest) -> Result<HttpResponse, Self::Error> {
            if self.responses.is_empty() {
                return Err("no response".into());
            }
            Ok(self.responses.remove(0))
        }
    }

    #[test]
    fn search_returns_references_not_raw_html() {
        let transport = MockTransport {
            responses: vec![HttpResponse {
                final_url: AUSTLII_SINO_ENDPOINT.into(),
                status_code: 200,
                content_type: Some("text/html".into()),
                body: br#"<a href=\"/cgi-bin/viewdoc/au/cases/cth/HCA/2026/19.html\">Cullen</a>"#.to_vec(),
            }],
        };
        let context = GovernedExecutionContext {
            operator_opt_in: true,
            cache_checked_first: true,
            persisted_receipts_checked_first: true,
            bounds: LiveGovernanceBounds::HISTORICAL_DEFAULT,
        };
        let mut executor = GovernedExecutor::new(transport, context).unwrap();
        let receipt = executor
            .austlii_search(&SinoQuery {
                meta: "/au".into(),
                query: "[2026] HCA 19".into(),
                method: SinoMethod::Phrase,
                results: 1,
                offset: 0,
                rank: None,
                callback: None,
                mask_path: Vec::new(),
                mask_by_phc: BTreeMap::new(),
            })
            .unwrap();
        assert_eq!(receipt.references.len(), 1);
        assert_eq!(receipt.network_requests, 1);
        assert_eq!(receipt.receipt_authority, RECEIPT_AUTHORITY);
    }

    #[test]
    fn execution_requires_operator_opt_in() {
        let context = GovernedExecutionContext {
            operator_opt_in: false,
            cache_checked_first: true,
            persisted_receipts_checked_first: true,
            bounds: LiveGovernanceBounds::HISTORICAL_DEFAULT,
        };
        assert!(matches!(
            GovernedExecutor::new(MockTransport::default(), context),
            Err(GovernanceError::OperatorOptInRequired)
        ));
    }
}
