//! S28 generic chronology and human-contestation domain.
//!
//! These carriers sit above parser observations and below legal/product-specific
//! interpretation. They preserve source/review ancestry and never manufacture
//! authority, applicability, or claim truth.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalForm {
    ExactInstant { instant_ref: String },
    ExactDate { date_ref: String },
    Interval { start_ref: String, end_ref: String },
    Approximate { label: String },
    RelativeBefore { event_ref: String },
    RelativeAfter { event_ref: String },
    Contemporaneous { event_ref: String },
    Undated,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalAssertion {
    pub temporal_ref: String,
    pub form: TemporalForm,
    pub statement_refs: Vec<String>,
    pub observation_refs: Vec<String>,
    pub review_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimLeafKind {
    Affirmation,
    Denial,
    Qualification,
    AlternativeAccount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimReviewState {
    Unreviewed,
    Accepted,
    Rejected,
    Abstained,
    Qualified,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionRoot {
    pub proposition_ref: String,
    pub label: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimLeaf {
    pub claim_ref: String,
    pub proposition_ref: String,
    pub kind: ClaimLeafKind,
    pub speaker_ref: Option<String>,
    pub statement_refs: Vec<String>,
    pub observation_refs: Vec<String>,
    pub temporal_refs: Vec<String>,
    pub scope_refs: Vec<String>,
    pub review_state: ClaimReviewState,
    pub review_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContestationRelationKind {
    Supports,
    Qualifies,
    Denies,
    Contradicts,
    Adjacent,
    Supersedes,
    SameIncidentDifferentAccount,
    UnresolvedRelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContestationRelation {
    pub relation_ref: String,
    pub from_claim_ref: String,
    pub to_claim_ref: String,
    pub kind: ContestationRelationKind,
    pub statement_refs: Vec<String>,
    pub observation_refs: Vec<String>,
    pub review_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChronologyContestationError {
    EmptyCoordinate(&'static str),
    EmptyEvidenceAncestry(&'static str),
    SelfRelation,
    ReviewReceiptRequired,
    PromotionNotAllowed,
}

fn require_nonempty(name: &'static str, value: &str) -> Result<(), ChronologyContestationError> {
    if value.trim().is_empty() {
        Err(ChronologyContestationError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn validate_refs(
    name: &'static str,
    refs: &[String],
    may_be_empty: bool,
) -> Result<(), ChronologyContestationError> {
    if !may_be_empty && refs.is_empty() {
        return Err(ChronologyContestationError::EmptyEvidenceAncestry(name));
    }
    if refs.iter().any(|value| value.trim().is_empty()) {
        return Err(ChronologyContestationError::EmptyCoordinate(name));
    }
    Ok(())
}

fn validate_non_promotion(
    candidate_only: bool,
    creates_semantic_authority: bool,
    applicability_promoted: bool,
    claim_truth_promoted: bool,
) -> Result<(), ChronologyContestationError> {
    if !candidate_only
        || creates_semantic_authority
        || applicability_promoted
        || claim_truth_promoted
    {
        Err(ChronologyContestationError::PromotionNotAllowed)
    } else {
        Ok(())
    }
}

impl TemporalAssertion {
    pub fn validate(&self) -> Result<(), ChronologyContestationError> {
        require_nonempty("temporal_ref", &self.temporal_ref)?;
        validate_refs("statement_refs", &self.statement_refs, true)?;
        validate_refs("observation_refs", &self.observation_refs, true)?;
        if self.statement_refs.is_empty() && self.observation_refs.is_empty() {
            return Err(ChronologyContestationError::EmptyEvidenceAncestry(
                "temporal_assertion",
            ));
        }
        if let Some(review_ref) = &self.review_ref {
            require_nonempty("review_ref", review_ref)?;
        }
        match &self.form {
            TemporalForm::ExactInstant { instant_ref } => {
                require_nonempty("instant_ref", instant_ref)?;
            }
            TemporalForm::ExactDate { date_ref } => {
                require_nonempty("date_ref", date_ref)?;
            }
            TemporalForm::Interval { start_ref, end_ref } => {
                require_nonempty("start_ref", start_ref)?;
                require_nonempty("end_ref", end_ref)?;
            }
            TemporalForm::Approximate { label } => {
                require_nonempty("approximate_label", label)?;
            }
            TemporalForm::RelativeBefore { event_ref }
            | TemporalForm::RelativeAfter { event_ref }
            | TemporalForm::Contemporaneous { event_ref } => {
                require_nonempty("relative_event_ref", event_ref)?;
            }
            TemporalForm::Undated | TemporalForm::Unknown => {}
        }
        validate_non_promotion(
            self.candidate_only,
            self.creates_semantic_authority,
            self.applicability_promoted,
            self.claim_truth_promoted,
        )
    }
}

impl PropositionRoot {
    pub fn validate(&self) -> Result<(), ChronologyContestationError> {
        require_nonempty("proposition_ref", &self.proposition_ref)?;
        require_nonempty("proposition_label", &self.label)?;
        validate_non_promotion(
            self.candidate_only,
            self.creates_semantic_authority,
            self.applicability_promoted,
            self.claim_truth_promoted,
        )
    }
}

impl ClaimLeaf {
    pub fn validate(&self) -> Result<(), ChronologyContestationError> {
        require_nonempty("claim_ref", &self.claim_ref)?;
        require_nonempty("proposition_ref", &self.proposition_ref)?;
        if let Some(speaker_ref) = &self.speaker_ref {
            require_nonempty("speaker_ref", speaker_ref)?;
        }
        validate_refs("statement_refs", &self.statement_refs, false)?;
        validate_refs("observation_refs", &self.observation_refs, true)?;
        validate_refs("temporal_refs", &self.temporal_refs, true)?;
        validate_refs("scope_refs", &self.scope_refs, true)?;
        if !matches!(self.review_state, ClaimReviewState::Unreviewed)
            && self
                .review_ref
                .as_deref()
                .map_or(true, |value| value.trim().is_empty())
        {
            return Err(ChronologyContestationError::ReviewReceiptRequired);
        }
        validate_non_promotion(
            self.candidate_only,
            self.creates_semantic_authority,
            self.applicability_promoted,
            self.claim_truth_promoted,
        )
    }
}

impl ContestationRelation {
    pub fn validate(&self) -> Result<(), ChronologyContestationError> {
        require_nonempty("relation_ref", &self.relation_ref)?;
        require_nonempty("from_claim_ref", &self.from_claim_ref)?;
        require_nonempty("to_claim_ref", &self.to_claim_ref)?;
        if self.from_claim_ref == self.to_claim_ref {
            return Err(ChronologyContestationError::SelfRelation);
        }
        validate_refs("statement_refs", &self.statement_refs, true)?;
        validate_refs("observation_refs", &self.observation_refs, true)?;
        if let Some(review_ref) = &self.review_ref {
            require_nonempty("review_ref", review_ref)?;
        }
        validate_non_promotion(
            self.candidate_only,
            self.creates_semantic_authority,
            self.applicability_promoted,
            self.claim_truth_promoted,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporal(form: TemporalForm) -> TemporalAssertion {
        TemporalAssertion {
            temporal_ref: "temporal:1".into(),
            form,
            statement_refs: vec!["statement:1".into()],
            observation_refs: vec!["observation:1".into()],
            review_ref: Some("review:time:1".into()),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn undated_and_unknown_are_distinct_states() {
        assert_ne!(
            temporal(TemporalForm::Undated).form,
            temporal(TemporalForm::Unknown).form
        );
    }

    #[test]
    fn relative_order_does_not_require_fake_timestamp() {
        let value = temporal(TemporalForm::RelativeAfter {
            event_ref: "event:earlier".into(),
        });
        assert!(value.validate().is_ok());
    }

    #[test]
    fn claim_leaf_keeps_wording_and_review_separate_from_root_identity() {
        let root = PropositionRoot {
            proposition_ref: "proposition:1".into(),
            label: "The call occurred".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        let a = ClaimLeaf {
            claim_ref: "claim:a".into(),
            proposition_ref: root.proposition_ref.clone(),
            kind: ClaimLeafKind::Affirmation,
            speaker_ref: Some("actor:a".into()),
            statement_refs: vec!["statement:a".into()],
            observation_refs: vec![],
            temporal_refs: vec![],
            scope_refs: vec![],
            review_state: ClaimReviewState::Accepted,
            review_ref: Some("review:a".into()),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        let b = ClaimLeaf {
            claim_ref: "claim:b".into(),
            proposition_ref: root.proposition_ref.clone(),
            kind: ClaimLeafKind::Denial,
            speaker_ref: Some("actor:b".into()),
            statement_refs: vec!["statement:b".into()],
            observation_refs: vec![],
            temporal_refs: vec![],
            scope_refs: vec![],
            review_state: ClaimReviewState::Unreviewed,
            review_ref: None,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        root.validate().unwrap();
        a.validate().unwrap();
        b.validate().unwrap();
        assert_eq!(a.proposition_ref, b.proposition_ref);
        assert_ne!(a.claim_ref, b.claim_ref);
        assert_ne!(a.kind, b.kind);
        assert_ne!(a.review_state, b.review_state);
    }

    #[test]
    fn contestation_is_a_relation_not_a_root_flag() {
        let relation = ContestationRelation {
            relation_ref: "relation:a:b".into(),
            from_claim_ref: "claim:a".into(),
            to_claim_ref: "claim:b".into(),
            kind: ContestationRelationKind::Contradicts,
            statement_refs: vec!["statement:a".into(), "statement:b".into()],
            observation_refs: vec![],
            review_ref: Some("review:relation:1".into()),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        assert!(relation.validate().is_ok());
    }

    #[test]
    fn claim_review_never_promotes_truth() {
        let mut claim = ClaimLeaf {
            claim_ref: "claim:1".into(),
            proposition_ref: "proposition:1".into(),
            kind: ClaimLeafKind::Qualification,
            speaker_ref: None,
            statement_refs: vec!["statement:1".into()],
            observation_refs: vec![],
            temporal_refs: vec![],
            scope_refs: vec![],
            review_state: ClaimReviewState::Qualified,
            review_ref: Some("review:claim:1".into()),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: true,
        };
        assert_eq!(
            claim.validate(),
            Err(ChronologyContestationError::PromotionNotAllowed)
        );
        claim.claim_truth_promoted = false;
        assert!(claim.validate().is_ok());
    }
}
