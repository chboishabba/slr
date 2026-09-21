//! Domain-generic LegalFollow campaign kernel.
//!
//! This deliberately extracts only the pure recursive mechanics shared by
//! multiple legal/research domains:
//!
//!   world -> recompute residuals -> select fresh demand
//!         -> externally reviewed delta -> apply -> recompute
//!
//! Source acquisition, human/LLM review, doctrine semantics and persistence
//! remain adapters.  The kernel cannot manufacture authority or claim truth.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericCampaignBudget {
    pub max_reviewed_deltas: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericCampaignStop {
    NoFreshDemand,
    BudgetExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericCampaignState<W, R> {
    pub world: W,
    pub residuals: Vec<R>,
    pub accepted_reviewed_deltas: usize,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_claim_truth: bool,
}

pub trait LegalFollowCampaignDomain {
    type World: Clone;
    type Residual: Clone;
    type Demand: Clone;
    type Delta;

    fn recompute_residuals(&self, world: &Self::World) -> Vec<Self::Residual>;

    fn select_fresh(
        &self,
        world: &Self::World,
        residuals: &[Self::Residual],
    ) -> Option<Self::Demand>;

    fn apply_reviewed_delta(
        &self,
        world: &Self::World,
        delta: &Self::Delta,
    ) -> Result<Self::World, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericLegalFollowCampaign<D>
where
    D: LegalFollowCampaignDomain,
{
    domain: D,
    budget: GenericCampaignBudget,
    state: GenericCampaignState<D::World, D::Residual>,
}

impl<D> GenericLegalFollowCampaign<D>
where
    D: LegalFollowCampaignDomain,
{
    pub fn new(
        domain: D,
        world: D::World,
        budget: GenericCampaignBudget,
    ) -> Result<Self, String> {
        if budget.max_reviewed_deltas == 0 {
            return Err("generic LegalFollow campaign budget must be positive".into());
        }
        let residuals = domain.recompute_residuals(&world);
        Ok(Self {
            domain,
            budget,
            state: GenericCampaignState {
                world,
                residuals,
                accepted_reviewed_deltas: 0,
                candidate_only: true,
                creates_legal_authority: false,
                creates_claim_truth: false,
            },
        })
    }

    pub fn state(&self) -> &GenericCampaignState<D::World, D::Residual> {
        &self.state
    }

    pub fn next_demand(&self) -> Result<D::Demand, GenericCampaignStop> {
        if self.state.accepted_reviewed_deltas >= self.budget.max_reviewed_deltas {
            return Err(GenericCampaignStop::BudgetExhausted);
        }
        self.domain
            .select_fresh(&self.state.world, &self.state.residuals)
            .ok_or(GenericCampaignStop::NoFreshDemand)
    }

    /// Accept a delta only after the domain adapter's review gate has paid it.
    /// The generic kernel does not contain an API for turning raw source bytes,
    /// unreviewed candidates, or research demands into a Delta.
    pub fn accept_reviewed_delta(&mut self, delta: &D::Delta) -> Result<(), String> {
        if self.state.accepted_reviewed_deltas >= self.budget.max_reviewed_deltas {
            return Err("generic LegalFollow campaign reviewed-delta budget exhausted".into());
        }
        let world = self.domain.apply_reviewed_delta(&self.state.world, delta)?;
        let residuals = self.domain.recompute_residuals(&world);
        self.state.world = world;
        self.state.residuals = residuals;
        self.state.accepted_reviewed_deltas += 1;
        self.state.candidate_only = true;
        self.state.creates_legal_authority = false;
        self.state.creates_claim_truth = false;
        Ok(())
    }
}

#[derive(Clone)]
struct KernelSelfCheckDomain;

impl LegalFollowCampaignDomain for KernelSelfCheckDomain {
    type World = Vec<bool>;
    type Residual = usize;
    type Demand = usize;
    type Delta = usize;

    fn recompute_residuals(&self, world: &Self::World) -> Vec<Self::Residual> {
        world
            .iter()
            .enumerate()
            .filter_map(|(index, paid)| (!paid).then_some(index))
            .collect()
    }

    fn select_fresh(
        &self,
        _world: &Self::World,
        residuals: &[Self::Residual],
    ) -> Option<Self::Demand> {
        residuals.first().copied()
    }

    fn apply_reviewed_delta(
        &self,
        world: &Self::World,
        delta: &Self::Delta,
    ) -> Result<Self::World, String> {
        let mut next = world.clone();
        let paid = next
            .get_mut(*delta)
            .ok_or_else(|| format!("unknown self-check coordinate {delta}"))?;
        *paid = true;
        Ok(next)
    }
}

pub fn generic_campaign_kernel_self_check() -> Result<(), String> {
    let mut campaign = GenericLegalFollowCampaign::new(
        KernelSelfCheckDomain,
        vec![false, false],
        GenericCampaignBudget {
            max_reviewed_deltas: 3,
        },
    )?;
    if campaign.next_demand() != Ok(0) {
        return Err("generic campaign self-check failed first demand".into());
    }
    campaign.accept_reviewed_delta(&0)?;
    if campaign.next_demand() != Ok(1) {
        return Err("generic campaign self-check failed recomputed demand".into());
    }
    campaign.accept_reviewed_delta(&1)?;
    if campaign.next_demand() != Err(GenericCampaignStop::NoFreshDemand) {
        return Err("generic campaign self-check failed terminal state".into());
    }
    if campaign.state().creates_legal_authority || campaign.state().creates_claim_truth {
        return Err("generic campaign self-check crossed promotion boundary".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Deliberately non-contract toy domain: unresolved integer coordinates.
    // This regression exists only to prove that the kernel itself does not
    // require AustralianContractTrace, doctrine enums or case identities.
    #[derive(Clone)]
    struct CoordinateDomain;

    impl LegalFollowCampaignDomain for CoordinateDomain {
        type World = Vec<bool>;
        type Residual = usize;
        type Demand = usize;
        type Delta = usize;

        fn recompute_residuals(&self, world: &Self::World) -> Vec<Self::Residual> {
            world
                .iter()
                .enumerate()
                .filter_map(|(index, paid)| (!paid).then_some(index))
                .collect()
        }

        fn select_fresh(
            &self,
            _world: &Self::World,
            residuals: &[Self::Residual],
        ) -> Option<Self::Demand> {
            residuals.first().copied()
        }

        fn apply_reviewed_delta(
            &self,
            world: &Self::World,
            delta: &Self::Delta,
        ) -> Result<Self::World, String> {
            let mut next = world.clone();
            let paid = next
                .get_mut(*delta)
                .ok_or_else(|| format!("unknown coordinate {delta}"))?;
            *paid = true;
            Ok(next)
        }
    }

    #[test]
    fn generic_kernel_recomputes_after_each_reviewed_delta() {
        let mut campaign = GenericLegalFollowCampaign::new(
            CoordinateDomain,
            vec![false, false],
            GenericCampaignBudget {
                max_reviewed_deltas: 2,
            },
        )
        .unwrap();

        assert_eq!(campaign.next_demand(), Ok(0));
        campaign.accept_reviewed_delta(&0).unwrap();
        assert_eq!(campaign.state().residuals, vec![1]);
        assert_eq!(campaign.next_demand(), Ok(1));
        campaign.accept_reviewed_delta(&1).unwrap();
        assert!(campaign.state().residuals.is_empty());
        assert_eq!(
            campaign.next_demand(),
            Err(GenericCampaignStop::BudgetExhausted)
        );
        assert!(!campaign.state().creates_legal_authority);
        assert!(!campaign.state().creates_claim_truth);
    }

    #[test]
    fn generic_kernel_has_no_raw_source_or_review_bypass_operation() {
        let campaign = GenericLegalFollowCampaign::new(
            CoordinateDomain,
            vec![true],
            GenericCampaignBudget {
                max_reviewed_deltas: 1,
            },
        )
        .unwrap();
        assert_eq!(
            campaign.next_demand(),
            Err(GenericCampaignStop::NoFreshDemand)
        );
        assert!(campaign.state().candidate_only);
    }
}