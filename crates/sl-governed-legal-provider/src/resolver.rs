use crate::RECEIPT_AUTHORITY;

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
