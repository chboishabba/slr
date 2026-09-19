//! Production adapters for Sprint 1 producer-family execution.
//!
//! These adapters intentionally stop at candidate evidence.  They reuse the
//! existing supervised classification, revision-pinned Wikidata identity and
//! governed OALC authority surfaces and present them through one controller ABI.

use sensiblaw_governed_legal_provider::{lookup_oalc_exact_mnc, OalcSnapshot};
use sensiblaw_route_selector::ProducerFamily;
use sensiblaw_wikimedia_candidate_provider::{
    fetch_latest_entity_rdf_revision_receipt, AcquiredEntityRdf,
};

use crate::gwb_supervised_type_closure::{
    acquire_supervised_type_closure_with, TypeClosureNodeProvider, TypeClosureQuestion,
    TypeClosureRequest,
};
use crate::sprint1_acquisition_machine::{
    CandidateProducerEvidence, ProducerExecutionPlan, ProducerExecutor, Sprint1AcquisitionError,
    Sprint1ProducerController,
};

pub struct ClassificationProducerExecutor<P> {
    provider: P,
    max_depth: usize,
    max_nodes: usize,
}

impl<P> ClassificationProducerExecutor<P> {
    #[must_use]
    pub const fn new(provider: P, max_depth: usize, max_nodes: usize) -> Self {
        Self {
            provider,
            max_depth,
            max_nodes,
        }
    }

    #[must_use]
    pub fn provider(&self) -> &P {
        &self.provider
    }
}

impl<P: TypeClosureNodeProvider> ProducerExecutor for ClassificationProducerExecutor<P> {
    fn execute(
        &mut self,
        plan: &ProducerExecutionPlan,
    ) -> Result<CandidateProducerEvidence, Sprint1AcquisitionError> {
        if plan.producer != ProducerFamily::ClassificationEvidence {
            return Err(Sprint1AcquisitionError::ProducerFamilyMismatch);
        }
        let closure = acquire_supervised_type_closure_with(
            &TypeClosureRequest {
                root_qid: plan.target_ref.clone(),
                question: TypeClosureQuestion::TypeClass,
                max_depth: self.max_depth,
                max_nodes: self.max_nodes,
            },
            &mut self.provider,
        )
        .map_err(|error| {
            Sprint1AcquisitionError::Provider(format!("classification-provider:{error}"))
        })?;

        Ok(CandidateProducerEvidence {
            producer: ProducerFamily::ClassificationEvidence,
            evidence_ref: closure.evidence_digest_ref,
            source_revision_ref: closure.source_manifest_ref,
            candidate_only: closure.candidate_only,
            creates_semantic_authority: closure.creates_semantic_authority,
            applicability_promoted: closure.applicability_promoted,
            claim_truth_promoted: closure.claim_truth_promoted,
            complete: !closure.truncated,
        })
    }
}

type IdentityFetcher =
    dyn FnMut(&str) -> Result<AcquiredEntityRdf, Sprint1AcquisitionError> + Send;

pub struct WikidataIdentityProducerExecutor {
    fetcher: Box<IdentityFetcher>,
}

impl WikidataIdentityProducerExecutor {
    #[must_use]
    pub fn live() -> Self {
        Self::with_fetcher(|qid| {
            fetch_latest_entity_rdf_revision_receipt(qid).map_err(|error| {
                Sprint1AcquisitionError::Provider(format!("wikidata-identity:{error}"))
            })
        })
    }

    #[must_use]
    pub fn with_fetcher<F>(fetcher: F) -> Self
    where
        F: FnMut(&str) -> Result<AcquiredEntityRdf, Sprint1AcquisitionError>
            + Send
            + 'static,
    {
        Self {
            fetcher: Box::new(fetcher),
        }
    }
}

impl ProducerExecutor for WikidataIdentityProducerExecutor {
    fn execute(
        &mut self,
        plan: &ProducerExecutionPlan,
    ) -> Result<CandidateProducerEvidence, Sprint1AcquisitionError> {
        if plan.producer != ProducerFamily::IdentitySource {
            return Err(Sprint1AcquisitionError::ProducerFamilyMismatch);
        }
        let acquired = (self.fetcher)(&plan.target_ref)?;
        if !acquired.candidate_only || acquired.semantic_promotion {
            return Err(Sprint1AcquisitionError::PromotingProducerEvidence);
        }
        Ok(CandidateProducerEvidence {
            producer: ProducerFamily::IdentitySource,
            evidence_ref: acquired.content_digest_ref,
            source_revision_ref: acquired.source_revision_ref,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            complete: true,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OalcAuthorityProducerExecutor {
    snapshot: OalcSnapshot,
}

impl OalcAuthorityProducerExecutor {
    #[must_use]
    pub const fn new(snapshot: OalcSnapshot) -> Self {
        Self { snapshot }
    }
}

impl ProducerExecutor for OalcAuthorityProducerExecutor {
    fn execute(
        &mut self,
        plan: &ProducerExecutionPlan,
    ) -> Result<CandidateProducerEvidence, Sprint1AcquisitionError> {
        if plan.producer != ProducerFamily::AuthoritySource {
            return Err(Sprint1AcquisitionError::ProducerFamilyMismatch);
        }
        let receipt = lookup_oalc_exact_mnc(&self.snapshot, &plan.target_ref).ok_or_else(|| {
            Sprint1AcquisitionError::Provider(format!(
                "oalc exact authority miss: {}",
                plan.target_ref
            ))
        })?;

        Ok(CandidateProducerEvidence {
            producer: ProducerFamily::AuthoritySource,
            evidence_ref: receipt.canonical_text_digest,
            source_revision_ref: receipt.source_revision_ref,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            complete: true,
        })
    }
}

/// Register the three Sprint 1 exit-test producer families on one controller.
/// Review/payment still happens outside this acquisition boundary.
pub fn register_sprint1_production_families<P>(
    controller: &mut Sprint1ProducerController,
    classification: ClassificationProducerExecutor<P>,
    identity: WikidataIdentityProducerExecutor,
    authority: OalcAuthorityProducerExecutor,
) where
    P: TypeClosureNodeProvider + 'static,
{
    controller.register(
        ProducerFamily::ClassificationEvidence,
        Box::new(classification),
    );
    controller.register(ProducerFamily::IdentitySource, Box::new(identity));
    controller.register(ProducerFamily::AuthoritySource, Box::new(authority));
}
