use sensiblaw_reader_model::{
    CoordinateCoverage, PropositionPayment, ResidualRef, SemanticRef, SourcePayment, SpanRef,
};
use thiserror::Error;

use crate::{PropositionChainPayment, PropositionProofRole, PropositionRoleResidual};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReaderPaymentProjectionError {
    #[error("exact source payment is required before constructing a Reader ABI payment")]
    ExactSourceUnpaid,
    #[error("paid proposition chain is missing coverage for {0:?}")]
    MissingRoleCoverage(PropositionProofRole),
}

fn coverage(
    paid_refs: &[String],
    role: PropositionProofRole,
    residuals: &[PropositionRoleResidual],
) -> Result<CoordinateCoverage, ReaderPaymentProjectionError> {
    if !paid_refs.is_empty() {
        return Ok(CoordinateCoverage::paid(paid_refs.to_vec()));
    }
    if let Some(residual) = residuals.iter().find(|residual| residual.role == role) {
        return Ok(CoordinateCoverage::Residualised(ResidualRef::new(
            residual.residual_ref.clone(),
        )));
    }
    Err(ReaderPaymentProjectionError::MissingRoleCoverage(role))
}

/// Project an already-evaluated SLR proposition payment into the portable Reader
/// ABI. This function cannot make an unpaid exact source paid and never promotes
/// applicability or claim truth.
pub fn project_reader_payment(
    chain: &PropositionChainPayment,
    source_revision_ref: &str,
    span_ref: &str,
    role_residuals: &[PropositionRoleResidual],
) -> Result<PropositionPayment, ReaderPaymentProjectionError> {
    if !chain.exact_source_paid {
        return Err(ReaderPaymentProjectionError::ExactSourceUnpaid);
    }

    let source = SourcePayment::paid(
        SemanticRef::new(chain.proposition_ref.clone()),
        source_revision_ref,
        SpanRef::new(span_ref),
    );

    if !chain.proposition_chain_paid {
        return Ok(PropositionPayment::source_only(source));
    }

    let qualifier = coverage(
        &chain.qualifier_observation_refs,
        PropositionProofRole::Qualifier,
        role_residuals,
    )?;
    let defeater = coverage(
        &chain.defeater_observation_refs,
        PropositionProofRole::Defeater,
        role_residuals,
    )?;
    let comparator = coverage(
        &chain.comparator_observation_refs,
        PropositionProofRole::Comparator,
        role_residuals,
    )?;

    Ok(PropositionPayment::bounded(
        source,
        chain.support_observation_refs.clone(),
        qualifier,
        defeater,
        comparator,
    ))
}
