use crate::frontier::{ProofFrontier, ProofResidual, ResidualStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchHypothesisKind {
    Support,
    Defeater,
    Comparator,
    Contradiction,
    AuthorityTreatment,
    TerminologyExpansion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHypothesis {
    pub hypothesis_ref: String,
    pub residual_ref: String,
    pub target_proposition_ref: String,
    pub kind: SearchHypothesisKind,
    pub producer_class_ref: String,
    pub jurisdiction_ref: Option<String>,
    pub authority_requirement_ref: Option<String>,
    pub hypothesis_authority: &'static str,
}

pub fn family_for_residual(residual: &ProofResidual) -> Vec<SearchHypothesis> {
    if residual.status != ResidualStatus::Open {
        return Vec::new();
    }
    let mk = |kind: SearchHypothesisKind, suffix: &str| SearchHypothesis {
        hypothesis_ref: format!("hyp:{}:{suffix}", residual.residual_ref),
        residual_ref: residual.residual_ref.clone(),
        target_proposition_ref: residual.proposition_ref.clone(),
        kind,
        producer_class_ref: residual.producer_class_ref.clone(),
        jurisdiction_ref: residual.jurisdiction_ref.clone(),
        authority_requirement_ref: residual.authority_requirement_ref.clone(),
        hypothesis_authority: "experimental_candidate_only",
    };
    vec![
        mk(SearchHypothesisKind::Support, "support"),
        mk(SearchHypothesisKind::Defeater, "defeater"),
        mk(SearchHypothesisKind::Comparator, "comparator"),
        mk(SearchHypothesisKind::Contradiction, "contradiction"),
        mk(SearchHypothesisKind::AuthorityTreatment, "treatment"),
    ]
}

pub fn families_for_frontier(frontier: &ProofFrontier) -> Vec<SearchHypothesis> {
    frontier.open_residuals().flat_map(family_for_residual).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontier::ProofResidual;

    #[test]
    fn open_residual_gets_opposing_and_comparator_searches() {
        let r = ProofResidual {
            residual_ref: "r".into(),
            proposition_ref: "p".into(),
            producer_class_ref: "producer".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: None,
            salience: 1,
            dependency_refs: vec![],
            status: ResidualStatus::Open,
        };
        let family = family_for_residual(&r);
        assert!(family.iter().any(|h| h.kind == SearchHypothesisKind::Support));
        assert!(family.iter().any(|h| h.kind == SearchHypothesisKind::Defeater));
        assert!(family.iter().any(|h| h.kind == SearchHypothesisKind::Comparator));
    }
}
