//! Governed legal-provider runtime for SensibLaw.
//!
//! Preferred Australian authority order:
//! persisted/local -> installed OALC -> official court -> optional sanctioned
//! specialist provider -> unresolved.
//!
//! The scheduler owns no HTTP. Search returns references, fetch returns bytes,
//! and semantic/legal interpretation begins only after local ingestion/review.

use std::collections::BTreeMap;
use std::thread;
use std::time::{Duration, Instant};

pub const RECEIPT_AUTHORITY: &str = "experimental_candidate_only";
pub const AUSTLII_SINO_ENDPOINT: &str = "https://www.austlii.edu.au/cgi-bin/sinosrch.cgi";
pub const AUSTLII_REFERER: &str = "https://www.austlii.edu.au/";
pub const AUSTLII_BROWSER_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
pub const SENSIBLAW_UA: &str = "SensibLaw/0.1 governed-legal-provider";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegalProvider {
    Local,
    Oalc,
    HighCourtAustralia,
    FederalCourtAustralia,
    AustLII,
    Jade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderOperation {
    LocalLookup,
    CorpusExactMediumNeutralCitationLookup,
    OfficialExactMediumNeutralCitationLookup,
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
pub enum ProviderAccessStatus {
    Available,
    PolicyBlocked,
    TlsInvalid,
    TemporarilyUnavailable,
    AuthorisationRequired,
    NotConfigured,
}

impl ProviderAccessStatus {
    pub const fn permits_failover(self) -> bool {
        !matches!(self, Self::Available)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AustliiAccessMode {
    PublicReferenceOnly,
    AuthorisedVirtualLab,
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
pub struct OalcRecord {
    pub citation: String,
    pub source_identity_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_digest: String,
    pub local_artifact_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcSnapshot {
    pub corpus_revision_ref: String,
    pub records: Vec<OalcRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcLookupReceipt {
    pub corpus_revision_ref: String,
    pub citation: String,
    pub source_identity_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_digest: String,
    pub local_artifact_ref: String,
    pub network_requests: u64,
    pub receipt_authority: &'static str,
}

pub fn lookup_oalc_exact_mnc(snapshot: &OalcSnapshot, citation: &str) -> Option<OalcLookupReceipt> {
    snapshot
        .records
        .iter()
        .find(|record| record.citation == citation)
        .map(|record| OalcLookupReceipt {
            corpus_revision_ref: snapshot.corpus_revision_ref.clone(),
            citation: citation.to_string(),
            source_identity_ref: record.source_identity_ref.clone(),
            source_revision_ref: record.source_revision_ref.clone(),
            canonical_text_digest: record.canonical_text_digest.clone(),
            local_artifact_ref: record.local_artifact_ref.clone(),
            network_requests: 0,
            receipt_authority: RECEIPT_AUTHORITY,
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionContext {
    pub persisted: Vec<PersistedAuthorityReceipt>,
    pub oalc_exact_mnc_citations: Vec<String>,
    pub official_hca_available: bool,
    pub official_fca_available: bool,
    pub jade_exact_mnc_available: bool,
    pub deterministic_austlii_mnc_available: bool,
    pub austlii_search_allowed: bool,
}

impl Default for ResolutionContext {
    fn default() -> Self {
        Self {
            persisted: Vec::new(),
            oalc_exact_mnc_citations: Vec::new(),
            official_hca_available: true,
            official_fca_available: true,
            jade_exact_mnc_available: false,
            deterministic_austlii_mnc_available: true,
            austlii_search_allowed: false,
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
    OalcExactMnc { citation: String },
    OfficialHighCourt { citation: String, reference: Option<String> },
    OfficialFederalCourt { citation: String, reference: Option<String> },
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
            Self::OalcExactMnc { .. } => Some(LegalProvider::Oalc),
            Self::OfficialHighCourt { .. } => Some(LegalProvider::HighCourtAustralia),
            Self::OfficialFederalCourt { .. } => Some(LegalProvider::FederalCourtAustralia),
            Self::ExplicitAustLII { .. }
            | Self::DeterministicMncToAustLII { .. }
            | Self::AustLIIReferenceSearch { .. } => Some(LegalProvider::AustLII),
            Self::JadeExactMnc { .. } => Some(LegalProvider::Jade),
            Self::Unresolved => None,
        }
    }

    pub const fn is_live_candidate(&self) -> bool {
        matches!(
            self,
            Self::OfficialHighCourt { .. }
                | Self::OfficialFederalCourt { .. }
                | Self::JadeExactMnc { .. }
                | Self::AustLIIReferenceSearch { .. }
        )
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

fn citation_court(citation: &str) -> Option<&str> {
    citation.split_whitespace().nth(1)
}

pub fn official_hca_known_reference(citation: &str) -> Option<String> {
    match citation {
        "[2026] HCA 19" => Some(
            "https://www.hcourt.gov.au/cases-and-judgments/judgments/judgments-1998-current/cullen-v-new-south-wales".into(),
        ),
        _ => None,
    }
}

pub fn official_fca_known_reference(citation: &str) -> Option<String> {
    match citation {
        "[2025] FCA 796" => Some(
            "https://www.fedcourt.gov.au/services/access-to-files-and-transcripts/online-files/pabai-v-australia".into(),
        ),
        _ => None,
    }
}

pub fn resolution_candidates(
    demand: &KnownAuthorityDemand,
    context: &ResolutionContext,
) -> Vec<ResolutionStage> {
    if let Some(receipt) = context.persisted.iter().find(|receipt| {
        receipt.compile_eligible
            && receipt.source_identity_ref == demand.source_identity_ref
            && receipt.jurisdiction_ref == demand.jurisdiction_ref
    }) {
        return vec![ResolutionStage::Persisted {
            source_revision_ref: receipt.source_revision_ref.clone(),
        }];
    }

    let mut stages = Vec::new();
    if let Some(citation) = demand.medium_neutral_citation.as_ref() {
        if context.oalc_exact_mnc_citations.iter().any(|known| known == citation) {
            stages.push(ResolutionStage::OalcExactMnc { citation: citation.clone() });
        }
        match citation_court(citation) {
            Some("HCA") if context.official_hca_available => stages.push(
                ResolutionStage::OfficialHighCourt {
                    citation: citation.clone(),
                    reference: official_hca_known_reference(citation),
                },
            ),
            Some("FCA" | "FCAFC") if context.official_fca_available => stages.push(
                ResolutionStage::OfficialFederalCourt {
                    citation: citation.clone(),
                    reference: official_fca_known_reference(citation),
                },
            ),
            _ => {}
        }
    }

    if let Some(reference) = demand.explicit_austlii_ref.as_ref() {
        stages.push(ResolutionStage::ExplicitAustLII { reference: reference.clone() });
    }
    if let Some(citation) = demand.medium_neutral_citation.as_ref() {
        if context.jade_exact_mnc_available {
            stages.push(ResolutionStage::JadeExactMnc { citation: citation.clone() });
        }
        if context.deterministic_austlii_mnc_available {
            stages.push(ResolutionStage::DeterministicMncToAustLII { citation: citation.clone() });
        }
        if context.austlii_search_allowed {
            stages.push(ResolutionStage::AustLIIReferenceSearch { citation: citation.clone() });
        }
    }
    stages.push(ResolutionStage::Unresolved);
    stages
}

pub fn resolve_known_authority(
    demand: &KnownAuthorityDemand,
    context: &ResolutionContext,
) -> KnownAuthorityResolution {
    KnownAuthorityResolution {
        demand_ref: demand.demand_ref.clone(),
        source_identity_ref: demand.source_identity_ref.clone(),
        proposition_ref: demand.proposition_ref.clone(),
        use_intent: demand.use_intent,
        treatment_intent: demand.treatment_intent,
        stage: resolution_candidates(demand, context)
            .into_iter()
            .next()
            .unwrap_or(ResolutionStage::Unresolved),
        governance: LiveGovernanceBounds::HISTORICAL_DEFAULT,
        acquisition_authority: AcquisitionAuthority::ExperimentalCandidateOnly,
        receipt_authority: RECEIPT_AUTHORITY,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinoMethod { Any, Or, All, Near, Phrase, Legis, Title, Boolean }

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
    let query_string = params
        .into_iter()
        .map(|(key, value)| format!("{}={}", form_encode(&key), form_encode(&value)))
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
    if year.len() != 4 || year.parse::<u32>().is_err() || number.parse::<u32>().is_err() {
        return None;
    }
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
    Some(format!(
        "https://www.austlii.edu.au/cgi-bin/viewdoc/au/cases/{jurisdiction}/{court}/{year}/{number}.html"
    ))
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

pub const fn citation_traversal_is_treatment_proof(_plan: &CitationTraversalPlan) -> bool { false }
pub const fn provider_failure_is_negative_legal_evidence(_status: ProviderAccessStatus) -> bool { false }

pub fn classify_http_status(status: u16) -> ProviderAccessStatus {
    match status {
        401 => ProviderAccessStatus::AuthorisationRequired,
        403 => ProviderAccessStatus::PolicyBlocked,
        408 | 425 | 429 | 500..=599 => ProviderAccessStatus::TemporarilyUnavailable,
        _ => ProviderAccessStatus::Available,
    }
}

pub fn classify_transport_error(message: &str) -> ProviderAccessStatus {
    let lower = message.to_ascii_lowercase();
    if lower.contains("certificate") || lower.contains("tls") || lower.contains("x509") {
        ProviderAccessStatus::TlsInvalid
    } else {
        ProviderAccessStatus::TemporarilyUnavailable
    }
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
        let response = match req.call() {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(err) => return Err(err.to_string()),
        };
        let status_code = response.status();
        let final_url = response.get_url().to_string();
        let content_type = response.header("Content-Type").map(str::to_string);
        let mut body = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut body)
            .map_err(|err| err.to_string())?;
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
    ProviderUnavailable { provider: LegalProvider, status: ProviderAccessStatus },
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
        if !self.persisted_receipts_checked_first {
            return Err(GovernanceError::PersistedReceiptsNotCheckedFirst);
        }
        if self.bounds.minimum_pacing_seconds < 4
            || self.bounds.burst != 1
            || self.bounds.max_depth > 1
            || self.bounds.max_new_documents > 5
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
where
    T::Error: ToString,
{
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

    fn governed_get(
        &mut self,
        provider: LegalProvider,
        request: HttpRequest,
    ) -> Result<HttpResponse, GovernanceError> {
        if self.network_requests >= self.context.bounds.max_network_requests {
            return Err(GovernanceError::RequestBudgetExceeded);
        }
        self.pace();
        let response = self.transport.get(&request).map_err(|err| {
            let status = classify_transport_error(&err.to_string());
            GovernanceError::ProviderUnavailable { provider, status }
        })?;
        self.network_requests += 1;
        self.last_request = Some(Instant::now());
        let status = classify_http_status(response.status_code);
        if status != ProviderAccessStatus::Available {
            return Err(GovernanceError::ProviderUnavailable { provider, status });
        }
        Ok(response)
    }

    pub fn austlii_search(&mut self, query: &SinoQuery) -> Result<SearchReferenceReceipt, GovernanceError> {
        let url = build_sino_url(query);
        let response = self.governed_get(
            LegalProvider::AustLII,
            HttpRequest {
                url: url.clone(),
                user_agent: AUSTLII_BROWSER_UA.into(),
                referer: Some(AUSTLII_REFERER.into()),
                timeout_seconds: 45,
            },
        )?;
        if !is_austlii_url(&response.final_url) { return Err(GovernanceError::InvalidProviderUrl); }
        Ok(SearchReferenceReceipt {
            provider: LegalProvider::AustLII,
            request_url: url,
            status_code: response.status_code,
            content_type: response.content_type,
            references: extract_references(LegalProvider::AustLII, &response.body),
            network_requests: 1,
            receipt_authority: RECEIPT_AUTHORITY,
        })
    }

    pub fn jade_search(&mut self, term: &str) -> Result<SearchReferenceReceipt, GovernanceError> {
        let url = jade_search_url(term);
        let response = self.governed_get(
            LegalProvider::Jade,
            HttpRequest { url: url.clone(), user_agent: SENSIBLAW_UA.into(), referer: None, timeout_seconds: 45 },
        )?;
        if !is_jade_url(&response.final_url) { return Err(GovernanceError::InvalidProviderUrl); }
        Ok(SearchReferenceReceipt {
            provider: LegalProvider::Jade,
            request_url: url,
            status_code: response.status_code,
            content_type: response.content_type,
            references: extract_references(LegalProvider::Jade, &response.body),
            network_requests: 1,
            receipt_authority: RECEIPT_AUTHORITY,
        })
    }

    pub fn fetch_austlii(&mut self, reference: &str) -> Result<FetchBytesReceipt, GovernanceError> {
        self.fetch_explicit(LegalProvider::AustLII, reference, Some(AUSTLII_REFERER))
    }
    pub fn fetch_jade(&mut self, reference: &str) -> Result<FetchBytesReceipt, GovernanceError> {
        self.fetch_explicit(LegalProvider::Jade, reference, None)
    }
    pub fn fetch_hca(&mut self, reference: &str) -> Result<FetchBytesReceipt, GovernanceError> {
        self.fetch_explicit(
            LegalProvider::HighCourtAustralia,
            reference,
            Some("https://www.hcourt.gov.au/"),
        )
    }
    pub fn fetch_fca(&mut self, reference: &str) -> Result<FetchBytesReceipt, GovernanceError> {
        self.fetch_explicit(
            LegalProvider::FederalCourtAustralia,
            reference,
            Some("https://www.fedcourt.gov.au/"),
        )
    }

    fn fetch_explicit(
        &mut self,
        provider: LegalProvider,
        reference: &str,
        referer: Option<&str>,
    ) -> Result<FetchBytesReceipt, GovernanceError> {
        if !is_provider_url(provider, reference) { return Err(GovernanceError::InvalidProviderUrl); }
        let response = self.governed_get(
            provider,
            HttpRequest {
                url: reference.into(),
                user_agent: SENSIBLAW_UA.into(),
                referer: referer.map(str::to_string),
                timeout_seconds: 45,
            },
        )?;
        if !is_provider_url(provider, &response.final_url) { return Err(GovernanceError::InvalidProviderUrl); }
        Ok(FetchBytesReceipt {
            provider,
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
    url.starts_with("https://jade.io/")
        || url.starts_with("https://www.jade.io/")
        || url.starts_with("https://jade.barnet.com.au/")
}
pub fn is_hca_url(url: &str) -> bool {
    url.starts_with("https://www.hcourt.gov.au/") || url.starts_with("https://hcourt.gov.au/")
}
pub fn is_fca_url(url: &str) -> bool {
    url.starts_with("https://www.fedcourt.gov.au/") || url.starts_with("https://fedcourt.gov.au/")
}

pub fn is_provider_url(provider: LegalProvider, url: &str) -> bool {
    match provider {
        LegalProvider::AustLII => is_austlii_url(url),
        LegalProvider::Jade => is_jade_url(url),
        LegalProvider::HighCourtAustralia => is_hca_url(url),
        LegalProvider::FederalCourtAustralia => is_fca_url(url),
        LegalProvider::Local | LegalProvider::Oalc => false,
    }
}

fn absolutize(provider: LegalProvider, href: &str) -> Option<String> {
    if href.starts_with("https://") {
        return is_provider_url(provider, href).then(|| href.to_string());
    }
    let base = match provider {
        LegalProvider::AustLII => "https://www.austlii.edu.au",
        LegalProvider::Jade => "https://jade.io",
        _ => return None,
    };
    href.starts_with('/').then(|| format!("{base}{href}"))
}

pub fn extract_references(provider: LegalProvider, body: &[u8]) -> Vec<String> {
    let html = String::from_utf8_lossy(body);
    let mut references = Vec::new();
    for marker in ["href=\"", "href='"] {
        let mut rest = html.as_ref();
        while let Some(start) = rest.find(marker) {
            rest = &rest[start + marker.len()..];
            let quote = if marker.ends_with('"') { '"' } else { '\'' };
            let Some(end) = rest.find(quote) else { break };
            if let Some(reference) = absolutize(provider, &rest[..end]) {
                if !references.contains(&reference) { references.push(reference); }
            }
            rest = &rest[end + 1..];
        }
    }
    references
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchReferenceReceipt {
    pub provider: LegalProvider,
    pub request_url: String,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub references: Vec<String>,
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
    struct MockTransport { responses: Vec<Result<HttpResponse, String>> }

    impl HttpTransport for MockTransport {
        type Error = String;
        fn get(&mut self, _request: &HttpRequest) -> Result<HttpResponse, Self::Error> {
            if self.responses.is_empty() { return Err("mock exhausted".into()); }
            self.responses.remove(0)
        }
    }

    #[test]
    fn oalc_is_zero_network_and_preferred_to_live_resolution() {
        let snapshot = OalcSnapshot {
            corpus_revision_ref: "oalc:v7.0.2".into(),
            records: vec![OalcRecord {
                citation: "[2026] HCA 19".into(),
                source_identity_ref: "case:[2026]-HCA-19".into(),
                source_revision_ref: "source:oalc:cullen".into(),
                canonical_text_digest: "sha256:oalc-cullen".into(),
                local_artifact_ref: "oalc://cullen".into(),
            }],
        };
        let receipt = lookup_oalc_exact_mnc(&snapshot, "[2026] HCA 19").unwrap();
        assert_eq!(receipt.network_requests, 0);
        assert_eq!(receipt.source_identity_ref, "case:[2026]-HCA-19");
    }

    #[test]
    fn policy_and_tls_failures_are_typed_and_non_evidential() {
        assert_eq!(classify_http_status(403), ProviderAccessStatus::PolicyBlocked);
        assert_eq!(
            classify_transport_error("certificate expired while connecting"),
            ProviderAccessStatus::TlsInvalid
        );
        assert!(!provider_failure_is_negative_legal_evidence(
            ProviderAccessStatus::PolicyBlocked
        ));
    }
}
