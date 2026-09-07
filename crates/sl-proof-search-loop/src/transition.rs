use crate::frontier::{ProofFrontier, ResidualStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualAssessmentKind {
    Narrowed,
    SatisfiedCandidate,
    Contested,
    AuthorityBlocked,
    Underidentified,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualAssessment {
    pub residual_ref: String,
    pub kind: ResidualAssessmentKind,
    pub observed_proof_reduction: u64,
    pub assessment_ref: String,
    pub assessment_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontierTransitionError {
    UnknownResidual(String),
    AssessmentMayNotClaimAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchTermination {
    Continue,
    ClosedCandidate,
    Contested,
    AuthorityBlocked,
    Underidentified,
    SaturatedCandidate,
    BudgetExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontierTransitionReceipt {
    pub prior_frontier_ref: String,
    pub next_frontier_ref: String,
    pub changed_residual_refs: Vec<String>,
    pub termination: ResearchTermination,
    pub transition_authority: &'static str,
}

fn status_for(kind: ResidualAssessmentKind, prior: ResidualStatus) -> ResidualStatus {
    match kind {
        ResidualAssessmentKind::Narrowed | ResidualAssessmentKind::Unchanged => prior,
        ResidualAssessmentKind::SatisfiedCandidate => ResidualStatus::SatisfiedCandidate,
        ResidualAssessmentKind::Contested => ResidualStatus::Contested,
        ResidualAssessmentKind::AuthorityBlocked => ResidualStatus::AuthorityBlocked,
        ResidualAssessmentKind::Underidentified => ResidualStatus::Underidentified,
    }
}

pub fn classify_frontier(frontier: &ProofFrontier) -> ResearchTermination {
    if frontier.residuals.is_empty() {
        return ResearchTermination::Underidentified;
    }
    if frontier
        .residuals
        .iter()
        .all(|residual| residual.status == ResidualStatus::SatisfiedCandidate)
    {
        return ResearchTermination::ClosedCandidate;
    }
    if frontier
        .residuals
        .iter()
        .any(|residual| residual.status == ResidualStatus::Open)
    {
        return ResearchTermination::Continue;
    }
    if frontier
        .residuals
        .iter()
        .any(|residual| residual.status == ResidualStatus::Contested)
    {
        return ResearchTermination::Contested;
    }
    if frontier
        .residuals
        .iter()
        .any(|residual| residual.status == ResidualStatus::AuthorityBlocked)
    {
        return ResearchTermination::AuthorityBlocked;
    }
    ResearchTermination::Underidentified
}

pub fn apply_assessments(
    frontier: &ProofFrontier,
    next_frontier_ref: impl Into<String>,
    assessments: &[ResidualAssessment],
) -> Result<(ProofFrontier, FrontierTransitionReceipt), FrontierTransitionError> {
    if assessments
        .iter()
        .any(|assessment| assessment.assessment_authority != "experimental_candidate_only")
    {
        return Err(FrontierTransitionError::AssessmentMayNotClaimAuthority);
    }

    let mut next = frontier.clone();
    next.frontier_ref = next_frontier_ref.into();
    let mut changed = Vec::new();

    for assessment in assessments {
        let residual = next
            .residuals
            .iter_mut()
            .find(|residual| residual.residual_ref == assessment.residual_ref)
            .ok_or_else(|| FrontierTransitionError::UnknownResidual(assessment.residual_ref.clone()))?;
        let prior = residual.status;
        let updated = status_for(assessment.kind, prior);
        if updated != prior || assessment.observed_proof_reduction > 0 {
            changed.push(assessment.residual_ref.clone());
        }
        residual.status = updated;
        match updated {
            ResidualStatus::SatisfiedCandidate => {
                if !next
                    .satisfied_payment_refs
                    .iter()
                    .any(|reference| reference == &assessment.assessment_ref)
                {
                    next.satisfied_payment_refs.push(assessment.assessment_ref.clone());
                }
            }
            ResidualStatus::Contested => {
                if !next
                    .contested_coordinate_refs
                    .iter()
                    .any(|reference| reference == &assessment.residual_ref)
                {
                    next.contested_coordinate_refs.push(assessment.residual_ref.clone());
                }
            }
            ResidualStatus::AuthorityBlocked => {
                if !next
                    .authority_blocked_refs
                    .iter()
                    .any(|reference| reference == &assessment.residual_ref)
                {
                    next.authority_blocked_refs.push(assessment.residual_ref.clone());
                }
            }
            ResidualStatus::Open | ResidualStatus::Underidentified => {}
        }
    }

    changed.sort();
    changed.dedup();
    let receipt = FrontierTransitionReceipt {
        prior_frontier_ref: frontier.frontier_ref.clone(),
        next_frontier_ref: next.frontier_ref.clone(),
        changed_residual_refs: changed,
        termination: classify_frontier(&next),
        transition_authority: "experimental_candidate_only",
    };
    Ok((next, receipt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontier::ProofResidual;

    fn frontier() -> ProofFrontier {
        ProofFrontier {
            consumer_ref: "consumer:pabai".into(),
            frontier_ref: "frontier:0".into(),
            residuals: vec![
                ProofResidual {
                    residual_ref: "r:comparator".into(),
                    proposition_ref: "p:comparator".into(),
                    producer_class_ref: "comparator".into(),
                    jurisdiction_ref: Some("AU".into()),
                    authority_requirement_ref: None,
                    salience: 5,
                    dependency_refs: vec![],
                    status: ResidualStatus::Open,
                },
                ProofResidual {
                    residual_ref: "r:treatment".into(),
                    proposition_ref: "p:treatment".into(),
                    producer_class_ref: "authority-treatment".into(),
                    jurisdiction_ref: Some("AU".into()),
                    authority_requirement_ref: Some("current-appellate".into()),
                    salience: 4,
                    dependency_refs: vec![],
                    status: ResidualStatus::Open,
                },
            ],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        }
    }

    #[test]
    fn satisfying_one_residual_preserves_other_open_work() {
        let (next, receipt) = apply_assessments(
            &frontier(),
            "frontier:1",
            &[ResidualAssessment {
                residual_ref: "r:comparator".into(),
                kind: ResidualAssessmentKind::SatisfiedCandidate,
                observed_proof_reduction: 2,
                assessment_ref: "assessment:cullen".into(),
                assessment_authority: "experimental_candidate_only",
            }],
        )
        .unwrap();
        assert_eq!(next.residuals[0].status, ResidualStatus::SatisfiedCandidate);
        assert_eq!(next.residuals[1].status, ResidualStatus::Open);
        assert_eq!(receipt.termination, ResearchTermination::Continue);
    }

    #[test]
    fn all_candidate_satisfied_is_only_closed_candidate() {
        let assessments = [
            ResidualAssessment {
                residual_ref: "r:comparator".into(),
                kind: ResidualAssessmentKind::SatisfiedCandidate,
                observed_proof_reduction: 2,
                assessment_ref: "assessment:a".into(),
                assessment_authority: "experimental_candidate_only",
            },
            ResidualAssessment {
                residual_ref: "r:treatment".into(),
                kind: ResidualAssessmentKind::SatisfiedCandidate,
                observed_proof_reduction: 1,
                assessment_ref: "assessment:b".into(),
                assessment_authority: "experimental_candidate_only",
            },
        ];
        let (_, receipt) = apply_assessments(&frontier(), "frontier:1", &assessments).unwrap();
        assert_eq!(receipt.termination, ResearchTermination::ClosedCandidate);
        assert_eq!(receipt.transition_authority, "experimental_candidate_only");
    }
}
