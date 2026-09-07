//! Typed legal-source planning for SensibLaw.
//!
//! This crate mirrors the established SensibLaw rule that legal-follow is an
//! acquisition backend, not a semantic classifier. It never fetches a URL.
//! It compiles a typed demand into either:
//! - a persisted compatible source selection;
//! - a missing-context residual; or
//! - an acquisition-required residual.
//!
//! Missing sources are work, never negative legal evidence.

use sensiblaw_legal_counterfactual::{producer_for, CounterfactualResidual, RequiredProducer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlanState {
    ReadyPersisted,
    BlockedMissingContext,
    BlockedAcquisitionRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceRole {
    PrimaryLegislation,
    PrimaryCaseLaw,
    OfficialRecord,
    ResearchIndex,
    SecondaryAnalysis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthorityLevel {
    Official,
    Supporting,
    Secondary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalSourceDemand {
    pub demand_ref: String,
    pub origin_ref: String,
    pub jurisdiction_ref: Option<String>,
    pub source_roles: Vec<SourceRole>,
    pub authority_levels: Vec<AuthorityLevel>,
    pub provider_profile_refs: Vec<String>,
    pub requested_facets: Vec<String>,
    pub temporal_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub priority: u16,
}

impl LegalSourceDemand {
    pub fn acquisition_ready(&self) -> bool {
        self.jurisdiction_ref.is_some()
            && !self.source_roles.is_empty()
            && !self.authority_levels.is_empty()
    }

    pub fn open_slots(&self) -> Vec<&'static str> {
        let mut slots = Vec::new();
        if self.jurisdiction_ref.is_none() {
            slots.push("jurisdiction_unresolved");
        }
        if self.source_roles.is_empty() {
            slots.push("source_role_unresolved");
        }
        if self.authority_levels.is_empty() {
            slots.push("authority_level_unresolved");
        }
        slots
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLegalSource {
    pub source_revision_ref: String,
    pub jurisdiction_ref: String,
    pub source_role: SourceRole,
    pub authority_level: AuthorityLevel,
    pub temporal_refs: Vec<String>,
    pub provider_profile_refs: Vec<String>,
    pub compile_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalSourcePlan {
    pub demand_ref: String,
    pub jurisdiction_ref: Option<String>,
    pub source_roles: Vec<SourceRole>,
    pub authority_levels: Vec<AuthorityLevel>,
    pub provider_profile_refs: Vec<String>,
    pub requested_facets: Vec<String>,
    pub temporal_refs: Vec<String>,
    pub state: PlanState,
    pub blocked_reasons: Vec<String>,
    pub selected_source_revision_refs: Vec<String>,
    pub authority: &'static str,
}

fn intersects(left: &[String], right: &[String]) -> bool {
    left.iter().any(|value| right.iter().any(|candidate| candidate == value))
}

pub fn plan_legal_sources(
    demand: &LegalSourceDemand,
    persisted_sources: &[PersistedLegalSource],
) -> LegalSourcePlan {
    let open_slots = demand.open_slots();
    if !open_slots.is_empty() {
        return LegalSourcePlan {
            demand_ref: demand.demand_ref.clone(),
            jurisdiction_ref: demand.jurisdiction_ref.clone(),
            source_roles: demand.source_roles.clone(),
            authority_levels: demand.authority_levels.clone(),
            provider_profile_refs: demand.provider_profile_refs.clone(),
            requested_facets: demand.requested_facets.clone(),
            temporal_refs: demand.temporal_refs.clone(),
            state: PlanState::BlockedMissingContext,
            blocked_reasons: open_slots.into_iter().map(str::to_owned).collect(),
            selected_source_revision_refs: Vec::new(),
            authority: "acquisition_plan_only",
        };
    }

    let jurisdiction = demand
        .jurisdiction_ref
        .as_ref()
        .expect("open-slot gate requires jurisdiction");
    let mut selected = persisted_sources
        .iter()
        .filter(|source| source.compile_eligible)
        .filter(|source| &source.jurisdiction_ref == jurisdiction)
        .filter(|source| demand.source_roles.contains(&source.source_role))
        .filter(|source| demand.authority_levels.contains(&source.authority_level))
        .filter(|source| {
            demand.provider_profile_refs.is_empty()
                || intersects(&demand.provider_profile_refs, &source.provider_profile_refs)
        })
        .filter(|source| {
            demand.temporal_refs.is_empty()
                || source.temporal_refs.is_empty()
                || intersects(&demand.temporal_refs, &source.temporal_refs)
        })
        .map(|source| source.source_revision_ref.clone())
        .collect::<Vec<_>>();
    selected.sort();
    selected.dedup();

    let (state, blocked_reasons) = if selected.is_empty() {
        (
            PlanState::BlockedAcquisitionRequired,
            vec!["compatible_persisted_legal_source_absent".to_owned()],
        )
    } else {
        (PlanState::ReadyPersisted, Vec::new())
    };

    LegalSourcePlan {
        demand_ref: demand.demand_ref.clone(),
        jurisdiction_ref: demand.jurisdiction_ref.clone(),
        source_roles: demand.source_roles.clone(),
        authority_levels: demand.authority_levels.clone(),
        provider_profile_refs: demand.provider_profile_refs.clone(),
        requested_facets: demand.requested_facets.clone(),
        temporal_refs: demand.temporal_refs.clone(),
        state,
        blocked_reasons,
        selected_source_revision_refs: selected,
        authority: "acquisition_plan_only",
    }
}

/// Compile only those counterfactual residuals whose producer is genuinely a
/// legal-source/authority resolver into a legal-source demand. Other residuals
/// remain on their native producer lane rather than being broadened into web
/// search.
pub fn demand_from_counterfactual_residual(
    demand_ref: impl Into<String>,
    origin_ref: impl Into<String>,
    residual: CounterfactualResidual,
    jurisdiction_ref: Option<String>,
) -> Option<LegalSourceDemand> {
    let producer = producer_for(residual);
    if !matches!(producer, RequiredProducer::LegalSourceResolver | RequiredProducer::AuthorityResolver) {
        return None;
    }

    let requested_facets = match residual {
        CounterfactualResidual::Admissibility => vec!["legal.counterfactual_admissibility".into()],
        CounterfactualResidual::Authority => vec!["legal.authority_unresolved".into()],
        _ => return None,
    };

    Some(LegalSourceDemand {
        demand_ref: demand_ref.into(),
        origin_ref: origin_ref.into(),
        jurisdiction_ref,
        source_roles: vec![SourceRole::PrimaryLegislation, SourceRole::PrimaryCaseLaw],
        authority_levels: vec![AuthorityLevel::Official],
        provider_profile_refs: Vec::new(),
        requested_facets,
        temporal_refs: Vec::new(),
        provenance_refs: vec!["sensiblaw-legal-counterfactual:v0_1".into()],
        priority: 100,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AcquisitionRequirement {
    None,
    MissingContext,
    CompatiblePersistedSourceAbsent,
}

pub fn acquisition_requirement(plan: &LegalSourcePlan) -> AcquisitionRequirement {
    match plan.state {
        PlanState::ReadyPersisted => AcquisitionRequirement::None,
        PlanState::BlockedMissingContext => AcquisitionRequirement::MissingContext,
        PlanState::BlockedAcquisitionRequired => {
            AcquisitionRequirement::CompatiblePersistedSourceAbsent
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demand() -> LegalSourceDemand {
        LegalSourceDemand {
            demand_ref: "d:1".into(),
            origin_ref: "counterfactual:q:1".into(),
            jurisdiction_ref: Some("AU".into()),
            source_roles: vec![SourceRole::PrimaryCaseLaw],
            authority_levels: vec![AuthorityLevel::Official],
            provider_profile_refs: vec!["au:high-court".into()],
            requested_facets: vec!["legal.counterfactual_admissibility".into()],
            temporal_refs: vec!["current".into()],
            provenance_refs: vec!["p:1".into()],
            priority: 100,
        }
    }

    fn source() -> PersistedLegalSource {
        PersistedLegalSource {
            source_revision_ref: "source:hca:rev:1".into(),
            jurisdiction_ref: "AU".into(),
            source_role: SourceRole::PrimaryCaseLaw,
            authority_level: AuthorityLevel::Official,
            temporal_refs: vec!["current".into()],
            provider_profile_refs: vec!["au:high-court".into()],
            compile_eligible: true,
        }
    }

    #[test]
    fn missing_context_is_not_acquisition_failure() {
        let mut demand = demand();
        demand.jurisdiction_ref = None;
        let plan = plan_legal_sources(&demand, &[]);
        assert_eq!(plan.state, PlanState::BlockedMissingContext);
        assert_eq!(acquisition_requirement(&plan), AcquisitionRequirement::MissingContext);
    }

    #[test]
    fn typed_plan_selects_only_compatible_persisted_revision() {
        let plan = plan_legal_sources(&demand(), &[source()]);
        assert_eq!(plan.state, PlanState::ReadyPersisted);
        assert_eq!(plan.selected_source_revision_refs, vec!["source:hca:rev:1"]);
        assert_eq!(plan.authority, "acquisition_plan_only");
    }

    #[test]
    fn absent_compatible_source_is_work_not_negative_evidence() {
        let plan = plan_legal_sources(&demand(), &[]);
        assert_eq!(plan.state, PlanState::BlockedAcquisitionRequired);
        assert_eq!(
            acquisition_requirement(&plan),
            AcquisitionRequirement::CompatiblePersistedSourceAbsent
        );
        assert!(plan.selected_source_revision_refs.is_empty());
    }

    #[test]
    fn non_source_residual_does_not_broaden_into_legal_follow() {
        let demand = demand_from_counterfactual_residual(
            "d:scope",
            "cf:q:1",
            CounterfactualResidual::Scope,
            Some("AU".into()),
        );
        assert!(demand.is_none());
    }

    #[test]
    fn legal_admissibility_residual_compiles_to_official_primary_source_demand() {
        let demand = demand_from_counterfactual_residual(
            "d:admissibility",
            "cf:q:1",
            CounterfactualResidual::Admissibility,
            Some("AU".into()),
        )
        .expect("admissibility should require a legal-source resolver");
        assert_eq!(demand.authority_levels, vec![AuthorityLevel::Official]);
        assert!(demand.source_roles.contains(&SourceRole::PrimaryCaseLaw));
        assert!(demand.source_roles.contains(&SourceRole::PrimaryLegislation));
    }
}
