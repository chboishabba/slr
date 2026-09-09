//! Bind an already-scheduled proof residual to the governed NSW point-in-time
//! legislation provider.
//!
//! This module does not perform network I/O. It preserves the existing chain:
//! residual -> hypothesis -> provider-neutral KnownAuthorityDemand -> exact
//! historical legislation demand -> governed provider acquisition.

use crate::bound_acquisition::ResidualBoundAuthorityDemand;
use sensiblaw_governed_legal_provider::official_resource::nsw_legislation::{
    bind_known_authority_to_historical_legislation, HistoricalLegislationDemand,
    HistoricalLegislationError,
};

pub const HISTORICAL_LEGISLATION_BINDING_AUTHORITY: &str = "experimental_candidate_only";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualBoundHistoricalLegislationDemand {
    pub residual_ref: String,
    pub proposition_ref: String,
    pub scheduled_producer_ref: String,
    pub hypothesis_ref: String,
    pub historical: HistoricalLegislationDemand,
    pub binding_authority: &'static str,
}

impl ResidualBoundHistoricalLegislationDemand {
    pub const fn acquisition_is_semantic_payment(&self) -> bool {
        false
    }

    pub const fn acquisition_closes_consumer(&self) -> bool {
        false
    }
}

pub fn bind_residual_to_nsw_historical_legislation(
    bound: &ResidualBoundAuthorityDemand,
    historical: HistoricalLegislationDemand,
) -> Result<ResidualBoundHistoricalLegislationDemand, HistoricalLegislationError> {
    let historical =
        bind_known_authority_to_historical_legislation(&bound.demand, historical)?;

    Ok(ResidualBoundHistoricalLegislationDemand {
        residual_ref: bound.residual_ref.clone(),
        proposition_ref: bound.proposition_ref.clone(),
        scheduled_producer_ref: bound.scheduled_producer_ref.clone(),
        hypothesis_ref: bound.hypothesis_ref.clone(),
        historical,
        binding_authority: HISTORICAL_LEGISLATION_BINDING_AUTHORITY,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bound_acquisition::BOUND_ACQUISITION_AUTHORITY;
    use sensiblaw_governed_legal_provider::{
        CitationTreatmentIntent, KnownAuthorityDemand, PropositionUseIntent,
    };

    fn bound() -> ResidualBoundAuthorityDemand {
        ResidualBoundAuthorityDemand {
            residual_ref: "residual:cullen:cla:s5b:pit-source".into(),
            proposition_ref: "prop:NSW:CLA:s5B:definition".into(),
            scheduled_producer_ref: "producer:official-point-in-time-legislation".into(),
            jurisdiction_ref: Some("AU-NSW".into()),
            hypothesis_ref: "hypothesis:cullen:cla:s5b:2017".into(),
            demand: KnownAuthorityDemand {
                demand_ref: "demand:cullen:cla:s5b:2017".into(),
                jurisdiction_ref: "AU-NSW".into(),
                source_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
                medium_neutral_citation: None,
                explicit_austlii_ref: None,
                proposition_ref: Some("prop:NSW:CLA:s5B:definition".into()),
                use_intent: PropositionUseIntent::SourceProposition,
                treatment_intent: CitationTreatmentIntent::None,
            },
            binding_authority: BOUND_ACQUISITION_AUTHORITY,
        }
    }

    fn historical() -> HistoricalLegislationDemand {
        HistoricalLegislationDemand {
            demand_ref: "demand:cullen:cla:s5b:2017".into(),
            jurisdiction_ref: "AU-NSW".into(),
            act_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
            official_document_id: "act-2002-022".into(),
            requested_locator: "s 5B".into(),
            in_force_on: "2017-01-26".into(),
            proposition_ref: "prop:NSW:CLA:s5B:definition".into(),
            source_identity_ref: "act:NSW:Civil-Liability-Act-2002".into(),
        }
    }

    #[test]
    fn scheduled_residual_reuses_provider_neutral_demand_then_adds_pit_requirement() {
        let welded = bind_residual_to_nsw_historical_legislation(&bound(), historical()).unwrap();
        assert_eq!(welded.residual_ref, "residual:cullen:cla:s5b:pit-source");
        assert_eq!(welded.historical.in_force_on, "2017-01-26");
        assert_eq!(welded.historical.official_document_id, "act-2002-022");
        assert_eq!(
            welded.scheduled_producer_ref,
            "producer:official-point-in-time-legislation"
        );
        assert!(!welded.acquisition_is_semantic_payment());
        assert!(!welded.acquisition_closes_consumer());
    }
}
