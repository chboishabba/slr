//! Typed legal counterfactual families for SensibLaw.
//!
//! This crate is deliberately downstream of parser/semantic admission. It does
//! not parse text, infer legal meaning from dependency labels, access a DB, or
//! publish legal conclusions. It preserves the distinction between:
//!
//! admissible corrected world -> causal identification -> scope -> violation ->
//! liability -> remedy -> authority.
//!
//! Multiple admissible corrected worlds may remain live. If they disagree on
//! the consumer-relevant outcome, the result is underidentified rather than
//! resolved by selecting a convenient world.

use sensiblaw_semantic_status::{ApplicabilityStatus, LiabilityStatus, ViolationStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldAdmissibility {
    Unresolved,
    Admissible,
    Inadmissible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrectionKind {
    Unresolved,
    EventDeletionOnly,
    CorrectedConduct,
    OmittedPrecautionSupplied,
    CorrectedInstitutionalRelation,
    CorrectedDecisionProcedure,
    CorrectedPolicyRegime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhysicalCompatibility {
    NotRequired,
    Unresolved,
    Compatible,
    Incompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CausalIdentificationStatus {
    Unresolved,
    NoAdmissibleWorldLocated,
    AlternativeWorldSearchIncomplete,
    Underidentified,
    IdentifiedSameOutcome,
    IdentifiedDependence,
    IdentifiedNoDependence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeStatus {
    Unresolved,
    Candidate,
    Satisfied,
    NotSatisfied,
    Underidentified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsequenceStatus {
    Unresolved,
    Candidate,
    Satisfied,
    NotSatisfied,
    Underidentified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CounterfactualResidual {
    Admissibility,
    CorrectedRelation,
    HeldFixedJustification,
    ComparisonAlignment,
    PhysicalCompatibility,
    AlternativeWorldSearch,
    OutcomeComparison,
    CausalIdentification,
    Scope,
    Violation,
    Liability,
    Remedy,
    Authority,
    ClosedForConsumer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequiredProducer {
    LegalSourceResolver,
    CorrectedConductResolver,
    InstitutionalRelationResolver,
    FactualEvidenceResolver,
    ComparisonGeometryResolver,
    PhysicalModelResolver,
    WorldFamilyEnumerator,
    OutcomeComparator,
    CausalIdentifier,
    ScopeResolver,
    ViolationResolver,
    LiabilityResolver,
    RemedyResolver,
    AuthorityResolver,
    None,
}

pub const fn producer_for(residual: CounterfactualResidual) -> RequiredProducer {
    match residual {
        CounterfactualResidual::Admissibility => RequiredProducer::LegalSourceResolver,
        CounterfactualResidual::CorrectedRelation => RequiredProducer::CorrectedConductResolver,
        CounterfactualResidual::HeldFixedJustification => RequiredProducer::FactualEvidenceResolver,
        CounterfactualResidual::ComparisonAlignment => RequiredProducer::ComparisonGeometryResolver,
        CounterfactualResidual::PhysicalCompatibility => RequiredProducer::PhysicalModelResolver,
        CounterfactualResidual::AlternativeWorldSearch => RequiredProducer::WorldFamilyEnumerator,
        CounterfactualResidual::OutcomeComparison => RequiredProducer::OutcomeComparator,
        CounterfactualResidual::CausalIdentification => RequiredProducer::CausalIdentifier,
        CounterfactualResidual::Scope => RequiredProducer::ScopeResolver,
        CounterfactualResidual::Violation => RequiredProducer::ViolationResolver,
        CounterfactualResidual::Liability => RequiredProducer::LiabilityResolver,
        CounterfactualResidual::Remedy => RequiredProducer::RemedyResolver,
        CounterfactualResidual::Authority => RequiredProducer::AuthorityResolver,
        CounterfactualResidual::ClosedForConsumer => RequiredProducer::None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterfactualWorld {
    pub world_ref: String,
    pub outcome_ref: String,
    pub admissibility: WorldAdmissibility,
    pub correction: CorrectionKind,
    pub corrected_relation_ref: Option<String>,
    pub held_fixed_justified: bool,
    pub comparison_aligned: bool,
    pub physical_compatibility: PhysicalCompatibility,
    pub source_refs: Vec<String>,
}

impl CounterfactualWorld {
    /// A world is eligible for outcome comparison only after all non-outcome
    /// admission gates have been paid. Event deletion alone is never enough.
    pub fn is_comparison_eligible(&self) -> bool {
        self.admissibility == WorldAdmissibility::Admissible
            && !matches!(
                self.correction,
                CorrectionKind::Unresolved | CorrectionKind::EventDeletionOnly
            )
            && self.corrected_relation_ref.is_some()
            && self.held_fixed_justified
            && self.comparison_aligned
            && matches!(
                self.physical_compatibility,
                PhysicalCompatibility::NotRequired | PhysicalCompatibility::Compatible
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterfactualFamily {
    pub question_ref: String,
    pub observed_world_ref: String,
    pub observed_outcome_ref: String,
    pub wrong_or_cause_of_action_ref: String,
    pub harmed_interest_ref: String,
    pub jurisdiction_ref: String,
    pub legal_source_ref: String,
    pub authority_receipt_ref: String,
    pub enumeration_complete: bool,
    pub worlds: Vec<CounterfactualWorld>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyEvaluation {
    pub status: CausalIdentificationStatus,
    pub eligible_world_refs: Vec<String>,
    pub live_outcome_refs: Vec<String>,
    pub first_residual: CounterfactualResidual,
    pub required_producer: RequiredProducer,
}

fn unique_push(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}

impl CounterfactualFamily {
    pub fn evaluate(&self) -> FamilyEvaluation {
        if self
            .worlds
            .iter()
            .any(|world| world.admissibility == WorldAdmissibility::Unresolved)
        {
            return self.residual_evaluation(CounterfactualResidual::Admissibility);
        }

        if self.worlds.iter().any(|world| {
            world.admissibility == WorldAdmissibility::Admissible
                && matches!(
                    world.correction,
                    CorrectionKind::Unresolved | CorrectionKind::EventDeletionOnly
                )
        }) {
            return self.residual_evaluation(CounterfactualResidual::CorrectedRelation);
        }

        if self.worlds.iter().any(|world| {
            world.admissibility == WorldAdmissibility::Admissible
                && !world.held_fixed_justified
        }) {
            return self.residual_evaluation(CounterfactualResidual::HeldFixedJustification);
        }

        if self.worlds.iter().any(|world| {
            world.admissibility == WorldAdmissibility::Admissible && !world.comparison_aligned
        }) {
            return self.residual_evaluation(CounterfactualResidual::ComparisonAlignment);
        }

        if self.worlds.iter().any(|world| {
            world.admissibility == WorldAdmissibility::Admissible
                && world.physical_compatibility == PhysicalCompatibility::Unresolved
        }) {
            return self.residual_evaluation(CounterfactualResidual::PhysicalCompatibility);
        }

        let eligible: Vec<&CounterfactualWorld> = self
            .worlds
            .iter()
            .filter(|world| world.is_comparison_eligible())
            .collect();

        if eligible.is_empty() {
            return FamilyEvaluation {
                status: CausalIdentificationStatus::NoAdmissibleWorldLocated,
                eligible_world_refs: Vec::new(),
                live_outcome_refs: Vec::new(),
                first_residual: CounterfactualResidual::AlternativeWorldSearch,
                required_producer: RequiredProducer::WorldFamilyEnumerator,
            };
        }

        if !self.enumeration_complete {
            return FamilyEvaluation {
                status: CausalIdentificationStatus::AlternativeWorldSearchIncomplete,
                eligible_world_refs: eligible.iter().map(|world| world.world_ref.clone()).collect(),
                live_outcome_refs: eligible.iter().fold(Vec::new(), |mut outcomes, world| {
                    unique_push(&mut outcomes, &world.outcome_ref);
                    outcomes
                }),
                first_residual: CounterfactualResidual::AlternativeWorldSearch,
                required_producer: RequiredProducer::WorldFamilyEnumerator,
            };
        }

        let mut outcomes = Vec::new();
        for world in &eligible {
            unique_push(&mut outcomes, &world.outcome_ref);
        }

        let eligible_refs = eligible
            .iter()
            .map(|world| world.world_ref.clone())
            .collect::<Vec<_>>();

        if outcomes.len() > 1 {
            return FamilyEvaluation {
                status: CausalIdentificationStatus::Underidentified,
                eligible_world_refs: eligible_refs,
                live_outcome_refs: outcomes,
                first_residual: CounterfactualResidual::CausalIdentification,
                required_producer: RequiredProducer::CausalIdentifier,
            };
        }

        let counterfactual_outcome = &outcomes[0];
        let status = if counterfactual_outcome == &self.observed_outcome_ref {
            CausalIdentificationStatus::IdentifiedNoDependence
        } else {
            CausalIdentificationStatus::IdentifiedDependence
        };

        FamilyEvaluation {
            status,
            eligible_world_refs: eligible_refs,
            live_outcome_refs: outcomes,
            first_residual: CounterfactualResidual::ClosedForConsumer,
            required_producer: RequiredProducer::None,
        }
    }

    fn residual_evaluation(&self, residual: CounterfactualResidual) -> FamilyEvaluation {
        FamilyEvaluation {
            status: CausalIdentificationStatus::Unresolved,
            eligible_world_refs: Vec::new(),
            live_outcome_refs: Vec::new(),
            first_residual: residual,
            required_producer: producer_for(residual),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalCausationAssessment {
    pub applicability: ApplicabilityStatus,
    pub factual_causation: CausalIdentificationStatus,
    pub scope: ScopeStatus,
    pub violation: ViolationStatus,
    pub liability: LiabilityStatus,
    pub remedy: ConsequenceStatus,
    pub authority: ConsequenceStatus,
}

impl LegalCausationAssessment {
    pub fn from_family(
        applicability: ApplicabilityStatus,
        family: &CounterfactualFamily,
    ) -> Self {
        Self {
            applicability,
            factual_causation: family.evaluate().status,
            scope: ScopeStatus::Unresolved,
            violation: ViolationStatus::Unresolved,
            liability: LiabilityStatus::Unresolved,
            remedy: ConsequenceStatus::Unresolved,
            authority: ConsequenceStatus::Unresolved,
        }
    }

    /// Consumer closure is deliberately indexed. A consumer that asks only for
    /// factual causation may close while liability/remedy/authority remain open.
    pub fn first_residual_for(&self, consumer: LegalConsumer) -> CounterfactualResidual {
        match consumer {
            LegalConsumer::FactualCausation => match self.factual_causation {
                CausalIdentificationStatus::IdentifiedDependence
                | CausalIdentificationStatus::IdentifiedNoDependence
                | CausalIdentificationStatus::IdentifiedSameOutcome => {
                    CounterfactualResidual::ClosedForConsumer
                }
                CausalIdentificationStatus::AlternativeWorldSearchIncomplete
                | CausalIdentificationStatus::NoAdmissibleWorldLocated => {
                    CounterfactualResidual::AlternativeWorldSearch
                }
                CausalIdentificationStatus::Underidentified => {
                    CounterfactualResidual::CausalIdentification
                }
                CausalIdentificationStatus::Unresolved => CounterfactualResidual::CausalIdentification,
            },
            LegalConsumer::ScopeOfLiability => {
                if self.scope == ScopeStatus::Unresolved {
                    CounterfactualResidual::Scope
                } else {
                    CounterfactualResidual::ClosedForConsumer
                }
            }
            LegalConsumer::Violation => {
                if self.violation == ViolationStatus::Unresolved {
                    CounterfactualResidual::Violation
                } else {
                    CounterfactualResidual::ClosedForConsumer
                }
            }
            LegalConsumer::Liability => {
                if self.liability == LiabilityStatus::Unresolved {
                    CounterfactualResidual::Liability
                } else {
                    CounterfactualResidual::ClosedForConsumer
                }
            }
            LegalConsumer::Remedy => {
                if self.remedy == ConsequenceStatus::Unresolved {
                    CounterfactualResidual::Remedy
                } else {
                    CounterfactualResidual::ClosedForConsumer
                }
            }
            LegalConsumer::Authority => {
                if self.authority == ConsequenceStatus::Unresolved {
                    CounterfactualResidual::Authority
                } else {
                    CounterfactualResidual::ClosedForConsumer
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegalConsumer {
    FactualCausation,
    ScopeOfLiability,
    Violation,
    Liability,
    Remedy,
    Authority,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_world(world_ref: &str, outcome_ref: &str) -> CounterfactualWorld {
        CounterfactualWorld {
            world_ref: world_ref.into(),
            outcome_ref: outcome_ref.into(),
            admissibility: WorldAdmissibility::Admissible,
            correction: CorrectionKind::CorrectedConduct,
            corrected_relation_ref: Some(format!("relation:{world_ref}")),
            held_fixed_justified: true,
            comparison_aligned: true,
            physical_compatibility: PhysicalCompatibility::NotRequired,
            source_refs: vec!["source:reviewed".into()],
        }
    }

    fn family(worlds: Vec<CounterfactualWorld>, enumeration_complete: bool) -> CounterfactualFamily {
        CounterfactualFamily {
            question_ref: "q:causation".into(),
            observed_world_ref: "world:observed".into(),
            observed_outcome_ref: "outcome:harm".into(),
            wrong_or_cause_of_action_ref: "wrong:typed".into(),
            harmed_interest_ref: "interest:harm".into(),
            jurisdiction_ref: "jurisdiction:reviewed".into(),
            legal_source_ref: "source:reviewed".into(),
            authority_receipt_ref: "authority:reviewed".into(),
            enumeration_complete,
            worlds,
        }
    }

    #[test]
    fn event_deletion_only_does_not_count_as_corrected_world() {
        let mut world = good_world("w:1", "outcome:no-harm");
        world.correction = CorrectionKind::EventDeletionOnly;
        world.corrected_relation_ref = None;
        let evaluation = family(vec![world], true).evaluate();
        assert_eq!(evaluation.status, CausalIdentificationStatus::Unresolved);
        assert_eq!(evaluation.first_residual, CounterfactualResidual::CorrectedRelation);
    }

    #[test]
    fn multiple_admissible_worlds_with_different_outcomes_are_underidentified() {
        let evaluation = family(
            vec![
                good_world("w:1", "outcome:no-harm"),
                good_world("w:2", "outcome:harm"),
            ],
            true,
        )
        .evaluate();
        assert_eq!(evaluation.status, CausalIdentificationStatus::Underidentified);
        assert_eq!(evaluation.live_outcome_refs.len(), 2);
        assert_eq!(evaluation.first_residual, CounterfactualResidual::CausalIdentification);
    }

    #[test]
    fn one_located_world_does_not_prove_unique_counterfactual_if_search_is_open() {
        let evaluation = family(vec![good_world("w:1", "outcome:no-harm")], false).evaluate();
        assert_eq!(
            evaluation.status,
            CausalIdentificationStatus::AlternativeWorldSearchIncomplete
        );
        assert_eq!(
            evaluation.first_residual,
            CounterfactualResidual::AlternativeWorldSearch
        );
    }

    #[test]
    fn complete_same_outcome_family_identifies_dependence() {
        let evaluation = family(
            vec![
                good_world("w:1", "outcome:no-harm"),
                good_world("w:2", "outcome:no-harm"),
            ],
            true,
        )
        .evaluate();
        assert_eq!(evaluation.status, CausalIdentificationStatus::IdentifiedDependence);
        assert_eq!(evaluation.required_producer, RequiredProducer::None);
    }

    #[test]
    fn physical_incompatibility_does_not_become_eligible_world() {
        let mut impossible = good_world("w:impossible", "outcome:no-harm");
        impossible.physical_compatibility = PhysicalCompatibility::Incompatible;
        let evaluation = family(vec![impossible], true).evaluate();
        assert_eq!(
            evaluation.status,
            CausalIdentificationStatus::NoAdmissibleWorldLocated
        );
    }

    #[test]
    fn causal_dependence_does_not_close_liability_or_remedy() {
        let family = family(vec![good_world("w:1", "outcome:no-harm")], true);
        let assessment =
            LegalCausationAssessment::from_family(ApplicabilityStatus::Admitted, &family);
        assert_eq!(
            assessment.factual_causation,
            CausalIdentificationStatus::IdentifiedDependence
        );
        assert_eq!(assessment.liability, LiabilityStatus::Unresolved);
        assert_eq!(assessment.remedy, ConsequenceStatus::Unresolved);
        assert_eq!(
            assessment.first_residual_for(LegalConsumer::FactualCausation),
            CounterfactualResidual::ClosedForConsumer
        );
        assert_eq!(
            assessment.first_residual_for(LegalConsumer::Liability),
            CounterfactualResidual::Liability
        );
    }

    #[test]
    fn residual_routes_to_exact_producer() {
        assert_eq!(
            producer_for(CounterfactualResidual::PhysicalCompatibility),
            RequiredProducer::PhysicalModelResolver
        );
        assert_eq!(
            producer_for(CounterfactualResidual::Remedy),
            RequiredProducer::RemedyResolver
        );
    }
}
