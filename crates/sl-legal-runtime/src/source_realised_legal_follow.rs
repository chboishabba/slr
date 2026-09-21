//! Generic adapter from the source-realised Australian legal evaluator to the
//! domain-independent LegalFollow campaign kernel.
//!
//! The kernel owns recurrence only.  This adapter keeps legal semantics in the
//! existing evaluator:
//!
//!   LegalCampaignState residual/action
//!        -> generic demand
//!        -> explicit reviewed next LegalEvaluationContext
//!        -> compile_legal_campaign_state
//!        -> recompute residual/action
//!
//! No doctrine-specific reducer is introduced here.

use crate::{
    compile_legal_campaign_state, AustralianCalibrationKind, GenericCampaignBudget,
    GenericCampaignStop, GenericLegalFollowCampaign, InformationAction,
    LegalCampaignState, LegalEvaluationContext, LegalFollowCampaignDomain,
    LegalResidual, LegalResidualKind, SourceRealisedLegalRule,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRealisedLegalWorld {
    pub rule: SourceRealisedLegalRule,
    pub state: LegalCampaignState,
    pub jurisdiction_ref: String,
    pub as_at: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl SourceRealisedLegalWorld {
    pub fn from_state(
        rule: SourceRealisedLegalRule,
        state: LegalCampaignState,
        jurisdiction_ref: impl Into<String>,
        as_at: impl Into<String>,
    ) -> Result<Self, String> {
        let jurisdiction_ref = jurisdiction_ref.into();
        let as_at = as_at.into();
        if jurisdiction_ref.trim().is_empty() || as_at.trim().is_empty() {
            return Err("source-realised legal world requires jurisdiction and as-at".into());
        }
        if !rule.candidate_only
            || !state.candidate_only
            || state.creates_claim_truth
        {
            return Err("source-realised legal world crossed non-promotion boundary".into());
        }
        if rule.rule_ref != state.evaluation.rule_ref {
            return Err("source-realised legal world rule/state identity mismatch".into());
        }
        Ok(Self {
            rule,
            state,
            jurisdiction_ref,
            as_at,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedLegalContextDelta {
    pub review_ref: String,
    pub triggering_residual_ref: String,
    pub next_context: LegalEvaluationContext,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ReviewedLegalContextDelta {
    pub fn new(
        review_ref: impl Into<String>,
        triggering_residual_ref: impl Into<String>,
        next_context: LegalEvaluationContext,
    ) -> Result<Self, String> {
        let review_ref = review_ref.into();
        let triggering_residual_ref = triggering_residual_ref.into();
        if review_ref.trim().is_empty() || triggering_residual_ref.trim().is_empty() {
            return Err("reviewed legal context delta requires review and residual refs".into());
        }
        Ok(Self {
            review_ref,
            triggering_residual_ref,
            next_context,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRealisedLegalDomain;

impl LegalFollowCampaignDomain for SourceRealisedLegalDomain {
    type World = SourceRealisedLegalWorld;
    type Residual = LegalResidual;
    type Demand = InformationAction;
    type Delta = ReviewedLegalContextDelta;

    fn recompute_residuals(&self, world: &Self::World) -> Vec<Self::Residual> {
        world
            .state
            .residuals
            .iter()
            .filter(|residual| residual.kind != LegalResidualKind::ClosedForConsumer)
            .cloned()
            .collect()
    }

    fn select_fresh(
        &self,
        world: &Self::World,
        residuals: &[Self::Residual],
    ) -> Option<Self::Demand> {
        let selected = world.state.selected_action.as_ref()?;
        residuals
            .iter()
            .any(|residual| residual.residual_ref == selected.residual_ref)
            .then(|| selected.clone())
    }

    fn apply_reviewed_delta(
        &self,
        world: &Self::World,
        delta: &Self::Delta,
    ) -> Result<Self::World, String> {
        if !delta.candidate_only
            || delta.creates_semantic_authority
            || delta.creates_claim_truth
        {
            return Err("reviewed legal context delta crossed non-promotion boundary".into());
        }
        if delta.next_context.jurisdiction_ref != world.jurisdiction_ref
            || delta.next_context.as_at != world.as_at
        {
            return Err(
                "reviewed legal context delta may not silently change legal-world coordinates"
                    .into(),
            );
        }
        let selected = world
            .state
            .selected_action
            .as_ref()
            .ok_or_else(|| "legal campaign has no selected demand to pay".to_string())?;
        if selected.residual_ref != delta.triggering_residual_ref {
            return Err(format!(
                "reviewed delta pays {}, but selected demand is {}",
                delta.triggering_residual_ref, selected.residual_ref
            ));
        }
        if !world
            .state
            .residuals
            .iter()
            .any(|residual| {
                residual.residual_ref == delta.triggering_residual_ref
                    && residual.kind != LegalResidualKind::ClosedForConsumer
            })
        {
            return Err("reviewed delta does not pay a current open legal residual".into());
        }

        let next = compile_legal_campaign_state(
            world.state.campaign_ref.clone(),
            world.state.calibration,
            world.state.iteration.saturating_add(1),
            &world.rule,
            &delta.next_context,
            Some(world.state.receipt_head.clone()),
        )
        .map_err(|error| format!("recompile source-realised legal world: {error:?}"))?;

        SourceRealisedLegalWorld::from_state(
            world.rule.clone(),
            next,
            world.jurisdiction_ref.clone(),
            world.as_at.clone(),
        )
    }
}

pub type GenericSourceRealisedLegalCampaign =
    GenericLegalFollowCampaign<SourceRealisedLegalDomain>;

pub fn source_realised_legal_campaign(
    rule: SourceRealisedLegalRule,
    initial_state: LegalCampaignState,
    jurisdiction_ref: impl Into<String>,
    as_at: impl Into<String>,
    max_reviewed_deltas: usize,
) -> Result<GenericSourceRealisedLegalCampaign, String> {
    let world =
        SourceRealisedLegalWorld::from_state(rule, initial_state, jurisdiction_ref, as_at)?;
    GenericLegalFollowCampaign::new(
        SourceRealisedLegalDomain,
        world,
        GenericCampaignBudget {
            max_reviewed_deltas,
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRealisedLegalCampaignReceipt {
    pub calibration: AustralianCalibrationKind,
    pub accepted_reviewed_deltas: usize,
    pub residuals_remaining: usize,
    pub selected_action_ref: Option<String>,
    pub state_receipt_head: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[must_use]
pub fn source_realised_legal_campaign_receipt(
    campaign: &GenericSourceRealisedLegalCampaign,
) -> SourceRealisedLegalCampaignReceipt {
    SourceRealisedLegalCampaignReceipt {
        calibration: campaign.state().world.state.calibration,
        accepted_reviewed_deltas: campaign.state().accepted_reviewed_deltas,
        residuals_remaining: campaign.state().residuals.len(),
        selected_action_ref: campaign
            .state()
            .world
            .state
            .selected_action
            .as_ref()
            .map(|action| action.action_ref.clone()),
        state_receipt_head: campaign.state().world.state.receipt_head.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}


pub fn source_realised_legal_campaign_self_check() -> Result<(), String> {
    let capstone =
        crate::build_australian_calibration_capstone(AustralianCalibrationKind::CullenNswCla)
            .map_err(|error| format!("build Cullen source-realised capstone: {error:?}"))?;
    if capstone.campaign.hops.len() != 2 {
        return Err("Cullen generic LegalFollow self-check expected exactly two canonical hops".into());
    }
    let initial = capstone.campaign.hops[0].clone();
    let expected = capstone.campaign.hops[1].clone();
    let selected = initial
        .selected_action
        .clone()
        .ok_or_else(|| "Cullen generic LegalFollow self-check missing reviewed demand".to_string())?;

    let mut campaign = source_realised_legal_campaign(
        capstone.rule.clone(),
        initial,
        capstone.context.jurisdiction_ref.clone(),
        capstone.context.as_at.clone(),
        4,
    )?;
    if campaign.next_demand().map(|demand| demand.residual_ref)
        != Ok(selected.residual_ref.clone())
    {
        return Err("Cullen generic LegalFollow self-check selected the wrong residual".into());
    }

    let delta = ReviewedLegalContextDelta::new(
        "review:cullen:secondary-review",
        selected.residual_ref,
        capstone.context.clone(),
    )?;
    campaign.accept_reviewed_delta(&delta)?;
    if campaign.state().world.state != expected {
        return Err("generic Cullen replay diverged from canonical reviewed hop".into());
    }
    if campaign.next_demand() != Err(GenericCampaignStop::NoFreshDemand) {
        return Err("generic Cullen replay did not close its bounded reviewed frontier".into());
    }
    if campaign.state().creates_legal_authority
        || campaign.state().creates_claim_truth
        || campaign.state().world.creates_semantic_authority
        || campaign.state().world.creates_claim_truth
    {
        return Err("generic Cullen replay crossed non-promotion boundary".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build_australian_calibration_capstone, AustralianCalibrationKind,
        InformationActionKind,
    };

    #[test]
    fn cullen_reviewed_hop_replays_exactly_through_generic_kernel() {
        let capstone =
            build_australian_calibration_capstone(AustralianCalibrationKind::CullenNswCla)
                .unwrap();
        assert_eq!(capstone.campaign.hops.len(), 2);
        let initial = capstone.campaign.hops[0].clone();
        let expected = capstone.campaign.hops[1].clone();
        let selected = initial.selected_action.clone().unwrap();
        assert_eq!(selected.kind, InformationActionKind::Review);

        let mut campaign = source_realised_legal_campaign(
            capstone.rule.clone(),
            initial,
            capstone.context.jurisdiction_ref.clone(),
            capstone.context.as_at.clone(),
            4,
        )
        .unwrap();
        assert_eq!(
            campaign.next_demand().unwrap().residual_ref,
            selected.residual_ref
        );

        let delta = ReviewedLegalContextDelta::new(
            "review:cullen:secondary-review",
            selected.residual_ref,
            capstone.context.clone(),
        )
        .unwrap();
        campaign.accept_reviewed_delta(&delta).unwrap();

        assert_eq!(campaign.state().world.state, expected);
        assert_eq!(
            campaign.next_demand(),
            Err(GenericCampaignStop::NoFreshDemand)
        );
        let receipt = source_realised_legal_campaign_receipt(&campaign);
        assert_eq!(receipt.calibration, AustralianCalibrationKind::CullenNswCla);
        assert_eq!(receipt.accepted_reviewed_deltas, 1);
        assert_eq!(receipt.residuals_remaining, 0);
        assert_eq!(receipt.state_receipt_head, expected.receipt_head);
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.creates_claim_truth);
    }

    #[test]
    fn generic_adapter_rejects_silent_world_coordinate_change() {
        let capstone =
            build_australian_calibration_capstone(AustralianCalibrationKind::CullenNswCla)
                .unwrap();
        let initial = capstone.campaign.hops[0].clone();
        let selected = initial.selected_action.clone().unwrap();
        let mut campaign = source_realised_legal_campaign(
            capstone.rule.clone(),
            initial,
            capstone.context.jurisdiction_ref.clone(),
            capstone.context.as_at.clone(),
            4,
        )
        .unwrap();

        let mut moved = capstone.context.clone();
        moved.jurisdiction_ref = "AU-QLD".into();
        let delta = ReviewedLegalContextDelta::new(
            "review:bad-world-change",
            selected.residual_ref,
            moved,
        )
        .unwrap();
        assert!(campaign.accept_reviewed_delta(&delta).is_err());
        assert_eq!(campaign.state().accepted_reviewed_deltas, 0);
    }
}