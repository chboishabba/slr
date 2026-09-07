//! Exact residual binding for governed authority acquisition.
//!
//! This is the Rust counterpart of the introspective proof-loop repair:
//! observing that a source might be useful is not enough.  Before the research
//! engine schedules governed acquisition, the live residual, selected search
//! hypothesis, and provider-neutral authority demand must agree on the exact
//! residual/proposition/producer/jurisdiction coordinates.

use crate::frontier::{ProofResidual, ResidualStatus};
use crate::hypothesis::SearchHypothesis;
use sensiblaw_governed_legal_provider::KnownAuthorityDemand;

pub const BOUND_ACQUISITION_AUTHORITY: &str = "experimental_candidate_only";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquisitionBindingError {
    ResidualNotOpen,
    HypothesisResidualMismatch,
    HypothesisPropositionMismatch,
    HypothesisProducerMismatch,
    HypothesisJurisdictionMismatch,
    DemandPropositionMissing,
    DemandPropositionMismatch,
    DemandJurisdictionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualBoundAuthorityDemand {
    pub residual_ref: String,
    pub proposition_ref: String,
    pub scheduled_producer_ref: String,
    pub jurisdiction_ref: Option<String>,
    pub hypothesis_ref: String,
    pub demand: KnownAuthorityDemand,
    pub binding_authority: &'static str,
}

impl ResidualBoundAuthorityDemand {
    pub const fn source_route_pays_scheduled_gap(&self) -> bool {
        true
    }

    pub const fn source_route_uses_scheduled_producer(&self) -> bool {
        true
    }

    pub const fn acquisition_is_semantic_payment(&self) -> bool {
        false
    }

    pub const fn acquisition_closes_consumer(&self) -> bool {
        false
    }
}

pub fn bind_authority_demand(
    residual: &ProofResidual,
    hypothesis: &SearchHypothesis,
    demand: KnownAuthorityDemand,
) -> Result<ResidualBoundAuthorityDemand, AcquisitionBindingError> {
    if residual.status != ResidualStatus::Open {
        return Err(AcquisitionBindingError::ResidualNotOpen);
    }
    if hypothesis.residual_ref != residual.residual_ref {
        return Err(AcquisitionBindingError::HypothesisResidualMismatch);
    }
    if hypothesis.target_proposition_ref != residual.proposition_ref {
        return Err(AcquisitionBindingError::HypothesisPropositionMismatch);
    }
    if hypothesis.producer_class_ref != residual.producer_class_ref {
        return Err(AcquisitionBindingError::HypothesisProducerMismatch);
    }
    if hypothesis.jurisdiction_ref != residual.jurisdiction_ref {
        return Err(AcquisitionBindingError::HypothesisJurisdictionMismatch);
    }
    let Some(demand_proposition) = demand.proposition_ref.as_deref() else {
        return Err(AcquisitionBindingError::DemandPropositionMissing);
    };
    if demand_proposition != residual.proposition_ref {
        return Err(AcquisitionBindingError::DemandPropositionMismatch);
    }
    if residual
        .jurisdiction_ref
        .as_deref()
        .is_some_and(|jurisdiction| jurisdiction != demand.jurisdiction_ref)
    {
        return Err(AcquisitionBindingError::DemandJurisdictionMismatch);
    }

    Ok(ResidualBoundAuthorityDemand {
        residual_ref: residual.residual_ref.clone(),
        proposition_ref: residual.proposition_ref.clone(),
        scheduled_producer_ref: residual.producer_class_ref.clone(),
        jurisdiction_ref: residual.jurisdiction_ref.clone(),
        hypothesis_ref: hypothesis.hypothesis_ref.clone(),
        demand,
        binding_authority: BOUND_ACQUISITION_AUTHORITY,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypothesis::{family_for_residual, SearchHypothesisKind};
    use sensiblaw_governed_legal_provider::{
        CitationTreatmentIntent, PropositionUseIntent,
    };

    fn residual() -> ProofResidual {
        ProofResidual {
            residual_ref: "residual:cullen-positive-operational-act".into(),
            proposition_ref: "prop:cullen-positive-operational-duty".into(),
            producer_class_ref: "producer:exact-primary-authority".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: Some("official-primary-case".into()),
            salience: 10,
            dependency_refs: vec!["source:[2026]-HCA-19".into()],
            status: ResidualStatus::Open,
        }
    }

    fn demand(proposition: &str) -> KnownAuthorityDemand {
        KnownAuthorityDemand {
            demand_ref: "demand:cullen-official-source".into(),
            jurisdiction_ref: "AU".into(),
            source_identity_ref: "case:[2026]-HCA-19".into(),
            medium_neutral_citation: Some("[2026] HCA 19".into()),
            explicit_austlii_ref: None,
            proposition_ref: Some(proposition.into()),
            use_intent: PropositionUseIntent::SourceProposition,
            treatment_intent: CitationTreatmentIntent::None,
        }
    }

    #[test]
    fn exact_live_residual_hypothesis_and_source_demand_bind() {
        let residual = residual();
        let hypothesis = family_for_residual(&residual)
            .into_iter()
            .find(|h| h.kind == SearchHypothesisKind::Support)
            .unwrap();
        let bound = bind_authority_demand(
            &residual,
            &hypothesis,
            demand("prop:cullen-positive-operational-duty"),
        )
        .unwrap();
        assert_eq!(bound.residual_ref, residual.residual_ref);
        assert_eq!(bound.scheduled_producer_ref, residual.producer_class_ref);
        assert!(bound.source_route_pays_scheduled_gap());
        assert!(bound.source_route_uses_scheduled_producer());
        assert!(!bound.acquisition_is_semantic_payment());
        assert!(!bound.acquisition_closes_consumer());
    }

    #[test]
    fn wrong_proposition_cannot_be_acquired_as_payment_for_live_residual() {
        let residual = residual();
        let hypothesis = family_for_residual(&residual)
            .into_iter()
            .find(|h| h.kind == SearchHypothesisKind::Support)
            .unwrap();
        assert_eq!(
            bind_authority_demand(&residual, &hypothesis, demand("prop:other")),
            Err(AcquisitionBindingError::DemandPropositionMismatch)
        );
    }

    #[test]
    fn closed_residual_cannot_schedule_source_acquisition() {
        let mut residual = residual();
        residual.status = ResidualStatus::SatisfiedCandidate;
        let mut hypothesis = family_for_residual(&ProofResidual {
            status: ResidualStatus::Open,
            ..residual.clone()
        })
        .into_iter()
        .next()
        .unwrap();
        hypothesis.residual_ref = residual.residual_ref.clone();
        assert_eq!(
            bind_authority_demand(
                &residual,
                &hypothesis,
                demand("prop:cullen-positive-operational-duty")
            ),
            Err(AcquisitionBindingError::ResidualNotOpen)
        );
    }
}
