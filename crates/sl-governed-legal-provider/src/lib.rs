//! Governed legal-provider runtime for SensibLaw.
//!
//! This crate is the only place in the proof-search stack allowed to host live
//! legal-provider execution.  The scheduler remains network-free.  Search
//! returns references; fetch returns bytes; semantics begin only after local
//! ingestion.  All results remain experimental/candidate-only until separately
//! reviewed and promoted.

use std::collections::BTreeMap;
use std::thread;
use std::time::{Duration, Instant};

pub const RECEIPT_AUTHORITY: &str = "experimental_candidate_only";
pub const AUSTLII_SINO_ENDPOINT: &str = "https://www.austlii.edu.au/cgi-bin/sinosrch.cgi";
pub const AUSTLII_REFERER: &str = "https://www.austlii.edu.au/";
pub const AUSTLII_BROWSER_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
pub const SENSIBLAW_UA: &str = "SensibLaw/0.1 governed-legal-provider";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegalProvider {
    Local,
    AustLII,
    Jade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderOperation {
    LocalLookup,
    ExplicitReference,
    ExactMediumNeutralCitationLookup,
    DeterministicMediumNeutralCitationLowering,
    TextReferenceSearch,
    CitedBy,
    CasesCited,
    LegislationCited,
    ExactDocumentFetch,
    BoundedCitationFollow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionAuthority {
    ExperimentalCandidateOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropositionUseIntent {
    IdentityOnly,
    SourceProposition,
    CitationTreatment,
    MaterialFeatureCorrespondence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitationTreatmentIntent {
    None,
    CitedBy,
    CasesCited,
    LegislationCited,
    AppliedOrFollowedCandidate,
    DistinguishedOrNarrowedCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownAuthorityDemand {
    pub demand_ref: String,
    pub jurisdiction_ref: String,
    pub source_identity_ref: String,
    pub medium_neutral_citation: Option<String>,
    pub explicit_austlii_ref: Option<String>,
    pub proposition_ref: Option<String>,
    pub use_intent: PropositionUseIntent,
    pub treatment_intent: CitationTreatmentIntent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedAuthorityReceipt {
    pub source_identity_ref: String,
    pub source_revision_ref: String,
    pub jurisdiction_ref: String,
    pub compile_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionContext {
    pub persisted: Vec<PersistedAuthorityReceipt>,
    pub jade_exact_mnc_available: bool,
    pub deterministic_austlii_mnc_available: bool,
    pub austlii_search_allowed: bool,
}

impl Default for ResolutionContext {
    fn default() -> Self {
        Self {
            persisted: Vec::new(),
            jade_exact_mnc_available: true,
            deterministic_austlii_mnc_available: true,
            austlii_search_allowed: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveGovernanceBounds {
    pub minimum_pacing_seconds: u64,
    pub burst: u64,
    pub max_depth: u64,
    pub max_new_documents: u64,
    pub max_network_requests: u64,
}

impl LiveGovernanceBounds {
    pub const HISTORICAL_DEFAULT: Self = Self {
        minimum_pacing_seconds: 4,
        burst: 1,
        max_depth: 1,
        max_new_documents: 5,
        max_network_requests: 6,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionStage {
    Persisted { source_revision_ref: String },
    ExplicitAustLII { reference: String },
    JadeExactMnc { citation: String },
    DeterministicMncToAustLII { citation: String },
    AustLIIReferenceSearch { citation: String },
    Unresolved,
}

impl ResolutionStage {
    pub const fn provider(&self) -> Option<LegalProvider> {
        match self {
            Self::Persisted { .. } => Some(LegalProvider::Local),
            Self::ExplicitAustLII { .. }
            | Self::DeterministicMncToAustLII { .. }
            | Self::AustLIIReferenceSearch { .. } => Some(LegalProvider::AustLII),
            Self::JadeExactMnc { .. } => Some(LegalProvider::Jade),
            Self::Unresolved => None,
        }
    }

    pub const fn is_live_candidate(&self) -> bool {
        matches!(self, Self::JadeExactMnc { .. } | Self::AustLIIReferenceSearch { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownAuthorityResolution {
    pub demand_ref: String,
    pub source_identity_ref: String,
    pub proposition_ref: Option<String>,
    pub use_intent: PropositionUseIntent,
    pub treatment_intent: CitationTreatmentIntent,
    pub stage: ResolutionStage,
    pub governance: LiveGovernanceBounds,
    pub acquisition_authority: AcquisitionAuthority,
    pub receipt_authority: &'static str,
}

impl KnownAuthorityResolution {
    pub const fn search_is_semantic_payment(&self) -> bool { false }
    pub const fn acquisition_is_authority_receipt(&self) -> bool { false }
}

pub fn resolve_known_authority(
    demand: &KnownAuthorityDemand,
    context: &ResolutionContext,
) -> KnownAuthorityResolution {
    let persisted = context.persisted.iter().find(|receipt| {
        receipt.compile_eligible
            && receipt.source_identity_ref == demand.source_identity_ref
            && receipt.jurisdiction_ref == demand.jurisdiction_ref
    });

    let stage = if let Some(receipt) = persisted {
        ResolutionStage::Persisted { source_revision_ref: receipt.source_revision_ref.clone() }
    } else if let Some(reference) = demand.explicit_austlii_ref.as_ref() {
        ResolutionStage::ExplicitAustLII { reference: reference.clone() }
    } else if let Some(citation) = demand.medium_neutral_citation.as_ref() {
        if context.jade_exact_mnc_available {
            ResolutionStage::JadeExactMnc { citation: citation.clone() }
        } else if context.deterministic_austlii_mnc_available {
            ResolutionStage::DeterministicMncToAustLII { citation: citation.clone() }
        } else if context.austlii_search_allowed {
            ResolutionStage::AustLIIReferenceSearch { citation: citation.clone() }
        } else {
            ResolutionStage::Unresolved
        }
    } else {
        ResolutionStage::Unresolved
    };

    KnownAuthorityResolution {
        demand_ref: demand.demand_ref.clone(),
        source_identity_ref: demand.source_identity_ref.clone(),
        proposition_ref: demand.proposition_ref.clone(),
        use_intent: demand.use_intent,
        treatment_intent: demand.treatment_intent,
        stage,
        governance: LiveGovernanceBounds::HISTORICAL_DEFAULT,
        acquisition_authority: AcquisitionAuthority::ExperimentalCandidateOnly,
        receipt_authority: RECEIPT_AUTHORITY,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinoMethod {
    Any,
    Or,
    All,
    Near,
    Phrase,
    Legis,
    Title,
    Boolean,
}

impl SinoMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Or => "or",
            Self::All => "all",
            Self::Near => "near",
            Self::Phrase => "phrase",
            Self::Legis => "legis",
            Self::Title => "title",
            Self::Boolean => "boolean",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinoQuery {
    pub meta: String,
    pub query: String,
    pub method: SinoMethod,
    pub results: u32,
    pub offset: u32,
    pub rank: Option<String>,
    pub callback: Option<String>,
    pub mask_path: Vec<String>,
    pub mask_by_phc: BTreeMap<String, Vec<String>>,
}

fn form_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            b' ' => out.push('+'),
            other => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push('%');
                out.push(HEX[(other >> 4) as usize] as char);
                out.push(HEX[(other & 0x0f) as usize] as char);
            }
        }
    }
    out
}

pub fn build_sino_url(query: &SinoQuery) -> String {
    let mut params = vec![
        ("meta".to_string(), query.meta.clone()),
        ("method".to_string(), query.method.as_str().to_string()),
        ("query".to_string(), query.query.clone()),
        ("results".to_string(), query.results.to_string()),
        ("offset".to_string(), query.offset.to_string()),
    ];
    if let Some(rank) = &query.rank { params.push(("rank".into(), rank.clone())); }
    if let Some(callback) = &query.callback { params.push(("callback".into(), callback.clone())); }
    for path in &query.mask_path { params.push(("mask_path".into(), path.clone())); }
    for (alias, paths) in &query.mask_by_phc {
        for path in paths { params.push((format!("mask_{alias}"), path.clone())); }
    }
    let query_string = params.into_iter()
        .map(|(k, v)| format!("{}={}", form_encode(&k), form_encode(&v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{AUSTLII_SINO_ENDPOINT}?{query_string}")
}

pub fn deterministic_mnc_to_austlii(citation: &str) -> Option<String> {
    let parts: Vec<_> = citation.split_whitespace().collect();
    if parts.len() != 3 { return None; }
    let year = parts[0].trim_matches(['[', ']']);
    let court = parts[1];
    let number = parts[2];
    if year.len() != 4 || year.parse::<u32>().is_err() || number.parse::<u32>().is_err() { return None; }
    let jurisdiction = match court {
        "HCA" | "FCA" | "FCAFC" => "cth",
        "QCA" | "QSC" => "qld",
        "NSWCA" | "NSWSC" => "nsw",
        "VSCA" | "VSC" => "vic",
        "WASCA" | "WASC" => "wa",
        "SASCA" | "SASC" => "sa",
        "TASFC" | "TASSC" => "tas",
        "NTCA" | "NTSC" => "nt",
        "ACTCA" | "ACTSC" => "act",
        _ => return None,
    };
    Some(format!("https://www.austlii.edu.au/cgi-bin/viewdoc/au/cases/{jurisdiction}/{court}/{year}/{number}.html"))
}

pub fn jade_exact_mnc_url(citation: &str) -> String {
    format!("https://jade.io/search/{}", form_encode(citation))
}

pub fn jade_search_url(term: &str) -> String {
    format!("https://jade.io/search/{}", form_encode(term))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationTraversalPlan {
    pub source_identity_ref: String,
    pub proposition_ref: Option<String>,
    pub treatment_intent: CitationTreatmentIntent,
    pub provider: LegalProvider,
    pub operation: ProviderOperation,
    pub bounds: LiveGovernanceBounds,
    pub acquisition_authority: AcquisitionAuthority,
}

pub fn citation_traversal_plan(demand: &KnownAuthorityDemand) -> Option<CitationTraversalPlan> {
    let operation = match demand.treatment_intent {
        CitationTreatmentIntent::None => return None,
        CitationTreatmentIntent::CitedBy
        | CitationTreatmentIntent::AppliedOrFollowedCandidate
        | CitationTreatmentIntent::DistinguishedOrNarrowedCandidate => ProviderOperation::CitedBy,
        CitationTreatmentIntent::CasesCited => ProviderOperation::CasesCited,
        CitationTreatmentIntent::LegislationCited => ProviderOperation::LegislationCited,
    };
    Some(CitationTraversalPlan {
        source_identity_ref: demand.source_identity_ref.clone(),
        proposition_ref: demand.proposition_ref.clone(),
        treatment_intent: demand.treatment_intent,
        provider: LegalProvider::Jade,
        operation,
        bounds: LiveGovernanceBounds::HISTORICAL_DEFAULT,
        acquisition_authority: AcquisitionAuthority::ExperimentalCandidateOnly,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub user_agent: String,
    pub referer: Option<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub final_url: String,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

pub trait HttpTransport {
    type Error;
    fn get(&mut self, request: &HttpRequest) -> Result<HttpResponse, Self::Error>;
}

#[cfg(feature = "live-network")]
pub struct UreqTransport;

#[cfg(feature = "live-network")]
impl HttpTransport for UreqTransport {
    type Error = String;

    fn get(&mut self, request: &HttpRequest) -> Result<HttpResponse, Self::Error> {
        use std::io::Read;
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(request.timeout_seconds))
            .build();
        let mut req = agent.get(&request.url).set("User-Agent", &request.user_agent);
        if let Some(referer) = &request.referer { req = req.set("Referer", referer); }
        let response = req.call().map_err(|err| err.to_string())?;
        let status_code = response.status();
        let final_url = response.get_url().to_string();
        let content_type = response.header("Content-Type").map(str::to_string);
        let mut body = Vec::new();
        response.into_reader().read_to_end(&mut body).map_err(|err| err.to_string())?;
        Ok(HttpResponse { final_url, status_code, content_type, body })
    }
}

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
        if !self.operator_opt_in { return Err(GovernanceError::OperatorOptInRequired); }
        if !self.cache_checked_first { return Err(GovernanceError::CacheNotCheckedFirst); }
        if !self.persisted_receipts_checked_first { return Err(GovernanceError::PersistedReceiptsNotCheckedFirst); }
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

pub struct GovernedExecutor<T> {
    transport: T,
    context: GovernedExecutionContext,
    last_request: Option<Instant>,
    network_requests: u64,
}

impl<T: HttpTransport> GovernedExecutor<T>
where T::Error: ToString {
    pub fn new(transport: T, context: GovernedExecutionContext) -> Result<Self, GovernanceError> {
        context.validate()?;
        Ok(Self { transport, context, last_request: None, network_requests: 0 })
    }

    pub const fn network_requests(&self) -> u64 { self.network_requests }

    fn pace(&mut self) {
        if let Some(last) = self.last_request {
            let minimum = Duration::from_secs(self.context.bounds.minimum_pacing_seconds);
            let elapsed = last.elapsed();
            if elapsed < minimum { thread::sleep(minimum - elapsed); }
        }
    }

    fn governed_get(&mut self, request: HttpRequest) -> Result<HttpResponse, GovernanceError> {
        if self.network_requests >= self.context.bounds.max_network_requests {
            return Err(GovernanceError::RequestBudgetExceeded);
        }
        self.pace();
        let response = self.transport.get(&request).map_err(|err| GovernanceError::Transport(err.to_string()))?;
        self.network_requests += 1;
        self.last_request = Some(Instant::now());
        Ok(response)
    }

    pub fn austlii_search(&mut self, query: &SinoQuery) -> Result<RawSearchReceipt, GovernanceError> {
        let url = build_sino_url(query);
        let response = self.governed_get(HttpRequest {
            url: url.clone(),
            user_agent: AUSTLII_BROWSER_UA.into(),
            referer: Some(AUSTLII_REFERER.into()),
            timeout_seconds: 45,
        })?;
        if !is_austlii_url(&response.final_url) { return Err(GovernanceError::InvalidProviderUrl); }
        Ok(RawSearchReceipt {
            provider: LegalProvider::AustLII,
            request_url: url,
            status_code: response.status_code,
            content_type: response.content_type,
            raw_body: response.body,
            network_requests: 1,
            receipt_authority: RECEIPT_AUTHORITY,
        })
    }

    pub fn fetch_austlii(&mut self, reference: &str) -> Result<FetchBytesReceipt, GovernanceError> {
        if !is_austlii_url(reference) { return Err(GovernanceError::InvalidProviderUrl); }
        let response = self.governed_get(HttpRequest {
            url: reference.into(),
            user_agent: SENSIBLAW_UA.into(),
            referer: Some(AUSTLII_REFERER.into()),
            timeout_seconds: 45,
        })?;
        if !is_austlii_url(&response.final_url) { return Err(GovernanceError::InvalidProviderUrl); }
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

    pub fn jade_search(&mut self, term: &str) -> Result<RawSearchReceipt, GovernanceError> {
        let url = jade_search_url(term);
        let response = self.governed_get(HttpRequest {
            url: url.clone(),
            user_agent: SENSIBLAW_UA.into(),
            referer: None,
            timeout_seconds: 45,
        })?;
        if !is_jade_url(&response.final_url) { return Err(GovernanceError::InvalidProviderUrl); }
        Ok(RawSearchReceipt {
            provider: LegalProvider::Jade,
            request_url: url,
            status_code: response.status_code,
            content_type: response.content_type,
            raw_body: response.body,
            network_requests: 1,
            receipt_authority: RECEIPT_AUTHORITY,
        })
    }

    pub fn fetch_jade(&mut self, reference: &str) -> Result<FetchBytesReceipt, GovernanceError> {
        if !is_jade_url(reference) { return Err(GovernanceError::InvalidProviderUrl); }
        let response = self.governed_get(HttpRequest {
            url: reference.into(),
            user_agent: SENSIBLAW_UA.into(),
            referer: None,
            timeout_seconds: 45,
        })?;
        if !is_jade_url(&response.final_url) { return Err(GovernanceError::InvalidProviderUrl); }
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

pub fn is_austlii_url(url: &str) -> bool {
    url.starts_with("https://www.austlii.edu.au/") || url.starts_with("https://austlii.edu.au/")
}

pub fn is_jade_url(url: &str) -> bool {
    url.starts_with("https://jade.io/") || url.starts_with("https://www.jade.io/")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSearchReceipt {
    pub provider: LegalProvider,
    pub request_url: String,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub raw_body: Vec<u8>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationFollowState {
    pub depth: u64,
    pub new_documents: u64,
    pub visited_source_identity_refs: Vec<String>,
}

pub fn may_follow_next(state: &CitationFollowState, bounds: LiveGovernanceBounds) -> bool {
    state.depth < bounds.max_depth && state.new_documents < bounds.max_new_documents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockTransport { responses: Vec<HttpResponse> }

    impl HttpTransport for MockTransport {
        type Error = String;
        fn get(&mut self, _request: &HttpRequest) -> Result<HttpResponse, Self::Error> {
            if self.responses.is_empty() { return Err("no response".into()); }
            Ok(self.responses.remove(0))
        }
    }

    fn cullen_demand() -> KnownAuthorityDemand {
        KnownAuthorityDemand {
            demand_ref: "duty:cullen:treatment".into(),
            jurisdiction_ref: "AU".into(),
            source_identity_ref: "case:Cullen-v-State-of-Queensland-2026-HCA-19".into(),
            medium_neutral_citation: Some("[2026] HCA 19".into()),
            explicit_austlii_ref: None,
            proposition_ref: Some("prop:cullen-positive-operational-duty".into()),
            use_intent: PropositionUseIntent::CitationTreatment,
            treatment_intent: CitationTreatmentIntent::CitedBy,
        }
    }

    #[test]
    fn persisted_wins_before_live_resolution() {
        let demand = cullen_demand();
        let context = ResolutionContext {
            persisted: vec![PersistedAuthorityReceipt {
                source_identity_ref: demand.source_identity_ref.clone(),
                source_revision_ref: "source:cullen:rev:1".into(),
                jurisdiction_ref: "AU".into(),
                compile_eligible: true,
            }],
            ..ResolutionContext::default()
        };
        assert!(matches!(resolve_known_authority(&demand, &context).stage, ResolutionStage::Persisted { .. }));
    }

    #[test]
    fn sino_url_matches_historical_parameter_order() {
        let query = SinoQuery {
            meta: "/au".into(), query: "positive operational act".into(), method: SinoMethod::Phrase,
            results: 50, offset: 0, rank: None, callback: None, mask_path: Vec::new(), mask_by_phc: BTreeMap::new(),
        };
        assert_eq!(build_sino_url(&query), "https://www.austlii.edu.au/cgi-bin/sinosrch.cgi?meta=%2Fau&method=phrase&query=positive+operational+act&results=50&offset=0");
    }

    #[test]
    fn deterministic_hca_mnc_lowering_is_reference_only() {
        assert_eq!(deterministic_mnc_to_austlii("[2026] HCA 19").as_deref(), Some("https://www.austlii.edu.au/cgi-bin/viewdoc/au/cases/cth/HCA/2026/19.html"));
    }

    #[test]
    fn treatment_intent_preserves_proposition_target() {
        let plan = citation_traversal_plan(&cullen_demand()).unwrap();
        assert_eq!(plan.provider, LegalProvider::Jade);
        assert_eq!(plan.operation, ProviderOperation::CitedBy);
        assert_eq!(plan.proposition_ref.as_deref(), Some("prop:cullen-positive-operational-duty"));
    }

    #[test]
    fn governed_execution_requires_explicit_opt_in() {
        let context = GovernedExecutionContext {
            operator_opt_in: false, cache_checked_first: true, persisted_receipts_checked_first: true,
            bounds: LiveGovernanceBounds::HISTORICAL_DEFAULT,
        };
        assert!(matches!(GovernedExecutor::new(MockTransport::default(), context), Err(GovernanceError::OperatorOptInRequired)));
    }

    #[test]
    fn one_bounded_austlii_search_is_candidate_only() {
        let transport = MockTransport { responses: vec![HttpResponse {
            final_url: AUSTLII_SINO_ENDPOINT.into(), status_code: 200,
            content_type: Some("text/html".into()), body: b"<html>fixture</html>".to_vec(),
        }]};
        let context = GovernedExecutionContext {
            operator_opt_in: true, cache_checked_first: true, persisted_receipts_checked_first: true,
            bounds: LiveGovernanceBounds::HISTORICAL_DEFAULT,
        };
        let mut executor = GovernedExecutor::new(transport, context).unwrap();
        let receipt = executor.austlii_search(&SinoQuery {
            meta: "/au".into(), query: "[2026] HCA 19".into(), method: SinoMethod::Phrase,
            results: 1, offset: 0, rank: None, callback: None, mask_path: Vec::new(), mask_by_phc: BTreeMap::new(),
        }).unwrap();
        assert_eq!(receipt.network_requests, 1);
        assert_eq!(receipt.receipt_authority, RECEIPT_AUTHORITY);
        assert_eq!(executor.network_requests(), 1);
    }

    #[test]
    fn fetched_bytes_require_separate_local_ingestion_receipt() {
        let fetch = FetchBytesReceipt {
            provider: LegalProvider::AustLII,
            reference: "https://www.austlii.edu.au/example".into(), status_code: 200,
            content_type: Some("text/html".into()), bytes: b"case".to_vec(), network_requests: 1,
            locally_ingested: false, receipt_authority: RECEIPT_AUTHORITY,
        };
        assert!(!fetch.locally_ingested);
        let ingested = mark_locally_ingested(&fetch, "case:cullen", "source:cullen:rev:1", "sha256:fixture");
        assert!(ingested.locally_ingested);
        assert_eq!(ingested.network_requests_used_to_acquire, 1);
    }
}
