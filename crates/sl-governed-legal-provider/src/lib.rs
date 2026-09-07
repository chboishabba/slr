//! Source-grounded, governed legal-provider contracts for SensibLaw.
//!
//! This crate intentionally performs no network I/O. It represents the acquisition
//! architecture required by the Agda-first SensibLaw proof-search stack and by the
//! historical SensibLaw source adapters. Search produces references; fetch produces
//! bytes elsewhere; neither establishes truth, authority, applicability, treatment,
//! proposition correspondence, or proof payment.
//!
//! Preferred known-authority escalation:
//!
//! local/persisted
//! -> explicit AustLII reference
//! -> exact MNC via JADE
//! -> deterministic MNC-to-AustLII reference
//! -> bounded AustLII reference search
//! -> unresolved.

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
}

impl LiveGovernanceBounds {
    pub const HISTORICAL_DEFAULT: Self = Self {
        minimum_pacing_seconds: 4,
        burst: 1,
        max_depth: 1,
        max_new_documents: 5,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionStage {
    Persisted {
        source_revision_ref: String,
    },
    ExplicitAustLII {
        reference: String,
    },
    JadeExactMnc {
        citation: String,
    },
    DeterministicMncToAustLII {
        citation: String,
    },
    AustLIIReferenceSearch {
        citation: String,
    },
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

    pub const fn operation(&self) -> Option<ProviderOperation> {
        match self {
            Self::Persisted { .. } => Some(ProviderOperation::LocalLookup),
            Self::ExplicitAustLII { .. } => Some(ProviderOperation::ExplicitReference),
            Self::JadeExactMnc { .. } => Some(ProviderOperation::ExactMediumNeutralCitationLookup),
            Self::DeterministicMncToAustLII { .. } => {
                Some(ProviderOperation::DeterministicMediumNeutralCitationLowering)
            }
            Self::AustLIIReferenceSearch { .. } => Some(ProviderOperation::TextReferenceSearch),
            Self::Unresolved => None,
        }
    }

    pub const fn is_live_candidate(&self) -> bool {
        matches!(
            self,
            Self::JadeExactMnc { .. } | Self::AustLIIReferenceSearch { .. }
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
    pub const fn search_is_semantic_payment(&self) -> bool {
        false
    }

    pub const fn acquisition_is_authority_receipt(&self) -> bool {
        false
    }
}

fn compatible_persisted<'a>(
    demand: &KnownAuthorityDemand,
    context: &'a ResolutionContext,
) -> Option<&'a PersistedAuthorityReceipt> {
    context.persisted.iter().find(|receipt| {
        receipt.compile_eligible
            && receipt.source_identity_ref == demand.source_identity_ref
            && receipt.jurisdiction_ref == demand.jurisdiction_ref
    })
}

pub fn resolve_known_authority(
    demand: &KnownAuthorityDemand,
    context: &ResolutionContext,
) -> KnownAuthorityResolution {
    let stage = if let Some(receipt) = compatible_persisted(demand, context) {
        ResolutionStage::Persisted {
            source_revision_ref: receipt.source_revision_ref.clone(),
        }
    } else if let Some(reference) = demand.explicit_austlii_ref.as_ref() {
        ResolutionStage::ExplicitAustLII {
            reference: reference.clone(),
        }
    } else if let Some(citation) = demand.medium_neutral_citation.as_ref() {
        if context.jade_exact_mnc_available {
            ResolutionStage::JadeExactMnc {
                citation: citation.clone(),
            }
        } else if context.deterministic_austlii_mnc_available {
            ResolutionStage::DeterministicMncToAustLII {
                citation: citation.clone(),
            }
        } else if context.austlii_search_allowed {
            ResolutionStage::AustLIIReferenceSearch {
                citation: citation.clone(),
            }
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
        receipt_authority: "experimental_candidate_only",
    }
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

pub fn citation_traversal_plan(
    demand: &KnownAuthorityDemand,
) -> Option<CitationTraversalPlan> {
    let (provider, operation) = match demand.treatment_intent {
        CitationTreatmentIntent::None => return None,
        CitationTreatmentIntent::CitedBy
        | CitationTreatmentIntent::AppliedOrFollowedCandidate
        | CitationTreatmentIntent::DistinguishedOrNarrowedCandidate => {
            (LegalProvider::Jade, ProviderOperation::CitedBy)
        }
        CitationTreatmentIntent::CasesCited => (LegalProvider::Jade, ProviderOperation::CasesCited),
        CitationTreatmentIntent::LegislationCited => {
            (LegalProvider::Jade, ProviderOperation::LegislationCited)
        }
    };

    Some(CitationTraversalPlan {
        source_identity_ref: demand.source_identity_ref.clone(),
        proposition_ref: demand.proposition_ref.clone(),
        treatment_intent: demand.treatment_intent,
        provider,
        operation,
        bounds: LiveGovernanceBounds::HISTORICAL_DEFAULT,
        acquisition_authority: AcquisitionAuthority::ExperimentalCandidateOnly,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn persisted_receipt_wins_before_any_live_candidate() {
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
        let resolution = resolve_known_authority(&demand, &context);
        assert_eq!(
            resolution.stage,
            ResolutionStage::Persisted {
                source_revision_ref: "source:cullen:rev:1".into()
            }
        );
        assert!(!resolution.stage.is_live_candidate());
        assert!(!resolution.search_is_semantic_payment());
    }

    #[test]
    fn explicit_austlii_reference_precedes_provider_search() {
        let mut demand = cullen_demand();
        demand.explicit_austlii_ref = Some("austlii:explicit:cullen".into());
        let resolution = resolve_known_authority(&demand, &ResolutionContext::default());
        assert_eq!(
            resolution.stage,
            ResolutionStage::ExplicitAustLII {
                reference: "austlii:explicit:cullen".into()
            }
        );
    }

    #[test]
    fn exact_mnc_prefers_jade_then_deterministic_austlii_then_search() {
        let demand = cullen_demand();

        let jade = resolve_known_authority(&demand, &ResolutionContext::default());
        assert!(matches!(jade.stage, ResolutionStage::JadeExactMnc { .. }));

        let deterministic = resolve_known_authority(
            &demand,
            &ResolutionContext {
                jade_exact_mnc_available: false,
                deterministic_austlii_mnc_available: true,
                austlii_search_allowed: true,
                ..ResolutionContext::default()
            },
        );
        assert!(matches!(
            deterministic.stage,
            ResolutionStage::DeterministicMncToAustLII { .. }
        ));

        let searched = resolve_known_authority(
            &demand,
            &ResolutionContext {
                jade_exact_mnc_available: false,
                deterministic_austlii_mnc_available: false,
                austlii_search_allowed: true,
                ..ResolutionContext::default()
            },
        );
        assert!(matches!(
            searched.stage,
            ResolutionStage::AustLIIReferenceSearch { .. }
        ));
    }

    #[test]
    fn unresolved_is_not_negative_legal_evidence() {
        let demand = KnownAuthorityDemand {
            medium_neutral_citation: None,
            explicit_austlii_ref: None,
            ..cullen_demand()
        };
        let resolution = resolve_known_authority(&demand, &ResolutionContext::default());
        assert_eq!(resolution.stage, ResolutionStage::Unresolved);
        assert!(!resolution.acquisition_is_authority_receipt());
    }

    #[test]
    fn treatment_intent_preserves_proposition_level_lineage_target() {
        let demand = cullen_demand();
        let plan = citation_traversal_plan(&demand).expect("cited-by intent should plan traversal");
        assert_eq!(plan.provider, LegalProvider::Jade);
        assert_eq!(plan.operation, ProviderOperation::CitedBy);
        assert_eq!(
            plan.proposition_ref.as_deref(),
            Some("prop:cullen-positive-operational-duty")
        );
        assert_eq!(plan.bounds.max_depth, 1);
        assert_eq!(plan.bounds.max_new_documents, 5);
    }

    #[test]
    fn source_identity_and_proposition_are_not_collapsed() {
        let demand = cullen_demand();
        let resolution = resolve_known_authority(&demand, &ResolutionContext::default());
        assert_ne!(
            resolution.source_identity_ref,
            resolution.proposition_ref.clone().unwrap()
        );
        assert_eq!(
            resolution.acquisition_authority,
            AcquisitionAuthority::ExperimentalCandidateOnly
        );
    }
}
