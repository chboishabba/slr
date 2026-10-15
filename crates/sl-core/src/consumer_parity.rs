use std::collections::BTreeSet;
use std::fmt;

fn nonempty(name: &'static str, value: &str) -> Result<(), ConsumerParityError> {
    if value.trim().is_empty() {
        Err(ConsumerParityError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerAdequacyReceipt {
    pub representation_ref: String,
    pub consumer_ref: String,
    pub query_family_ref: String,
    pub admissibility_receipt_ref: String,
    pub factorisation_receipt_refs: BTreeSet<String>,
    pub admissible: bool,
    pub consumer_adequate: bool,
    pub description_length_rank: Option<u64>,
}

impl ConsumerAdequacyReceipt {
    pub fn new<I, S>(
        representation_ref: impl Into<String>,
        consumer_ref: impl Into<String>,
        query_family_ref: impl Into<String>,
        admissibility_receipt_ref: impl Into<String>,
        factorisation_receipt_refs: I,
        admissible: bool,
        consumer_adequate: bool,
        description_length_rank: Option<u64>,
    ) -> Result<Self, ConsumerParityError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let receipt = Self {
            representation_ref: representation_ref.into(),
            consumer_ref: consumer_ref.into(),
            query_family_ref: query_family_ref.into(),
            admissibility_receipt_ref: admissibility_receipt_ref.into(),
            factorisation_receipt_refs: factorisation_receipt_refs
                .into_iter()
                .map(Into::into)
                .collect(),
            admissible,
            consumer_adequate,
            description_length_rank,
        };
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), ConsumerParityError> {
        nonempty("representation_ref", &self.representation_ref)?;
        nonempty("consumer_ref", &self.consumer_ref)?;
        nonempty("query_family_ref", &self.query_family_ref)?;
        nonempty("admissibility_receipt_ref", &self.admissibility_receipt_ref)?;
        if self.factorisation_receipt_refs.iter().any(|value| value.trim().is_empty()) {
            return Err(ConsumerParityError::EmptyFactorisationReceipt);
        }
        if self.consumer_adequate && self.factorisation_receipt_refs.is_empty() {
            return Err(ConsumerParityError::AdequacyWithoutFactorisation);
        }
        Ok(())
    }

    pub fn eligible(&self) -> bool {
        self.admissible && self.consumer_adequate
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationParityReceipt {
    pub left: ConsumerAdequacyReceipt,
    pub right: ConsumerAdequacyReceipt,
    pub review_ref: String,
    pub creates_world_complete_equivalence: bool,
    pub promotes_claim_truth: bool,
}

impl RepresentationParityReceipt {
    pub fn new(
        left: ConsumerAdequacyReceipt,
        right: ConsumerAdequacyReceipt,
        review_ref: impl Into<String>,
    ) -> Result<Self, ConsumerParityError> {
        left.validate()?;
        right.validate()?;
        let review_ref = review_ref.into();
        nonempty("review_ref", &review_ref)?;
        if left.consumer_ref != right.consumer_ref {
            return Err(ConsumerParityError::ConsumerMismatch);
        }
        if left.query_family_ref != right.query_family_ref {
            return Err(ConsumerParityError::QueryFamilyMismatch);
        }
        if !left.eligible() || !right.eligible() {
            return Err(ConsumerParityError::IneligibleRepresentation);
        }
        Ok(Self {
            left,
            right,
            review_ref,
            creates_world_complete_equivalence: false,
            promotes_claim_truth: false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressiveAsymmetryReceipt {
    pub left_source_ref: String,
    pub right_source_ref: String,
    pub shared_semantic_coordinate_ref: String,
    pub left_complexity_ref: String,
    pub right_complexity_ref: String,
    pub factorisation_receipt_ref: String,
    pub creates_semantic_authority: bool,
    pub creates_evidential_authority: bool,
    pub promotes_claim_truth: bool,
}

impl ExpressiveAsymmetryReceipt {
    pub fn new(
        left_source_ref: impl Into<String>,
        right_source_ref: impl Into<String>,
        shared_semantic_coordinate_ref: impl Into<String>,
        left_complexity_ref: impl Into<String>,
        right_complexity_ref: impl Into<String>,
        factorisation_receipt_ref: impl Into<String>,
    ) -> Result<Self, ConsumerParityError> {
        let receipt = Self {
            left_source_ref: left_source_ref.into(),
            right_source_ref: right_source_ref.into(),
            shared_semantic_coordinate_ref: shared_semantic_coordinate_ref.into(),
            left_complexity_ref: left_complexity_ref.into(),
            right_complexity_ref: right_complexity_ref.into(),
            factorisation_receipt_ref: factorisation_receipt_ref.into(),
            creates_semantic_authority: false,
            creates_evidential_authority: false,
            promotes_claim_truth: false,
        };
        for (name, value) in [
            ("left_source_ref", receipt.left_source_ref.as_str()),
            ("right_source_ref", receipt.right_source_ref.as_str()),
            ("shared_semantic_coordinate_ref", receipt.shared_semantic_coordinate_ref.as_str()),
            ("left_complexity_ref", receipt.left_complexity_ref.as_str()),
            ("right_complexity_ref", receipt.right_complexity_ref.as_str()),
            ("factorisation_receipt_ref", receipt.factorisation_receipt_ref.as_str()),
        ] {
            nonempty(name, value)?;
        }
        if receipt.left_complexity_ref == receipt.right_complexity_ref {
            return Err(ConsumerParityError::NoExpressiveAsymmetry);
        }
        Ok(receipt)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerCounterexampleReceipt {
    pub representation_ref: String,
    pub consumer_ref: String,
    pub query_ref: String,
    pub left_state_ref: String,
    pub right_state_ref: String,
    pub left_outcome_ref: String,
    pub right_outcome_ref: String,
    pub residual_ref: String,
    pub candidate_consumer_adequate: bool,
    pub promotes_claim_truth: bool,
    pub refutes_source_truth: bool,
}

impl ConsumerCounterexampleReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        representation_ref: impl Into<String>,
        consumer_ref: impl Into<String>,
        query_ref: impl Into<String>,
        left_state_ref: impl Into<String>,
        right_state_ref: impl Into<String>,
        left_outcome_ref: impl Into<String>,
        right_outcome_ref: impl Into<String>,
        residual_ref: impl Into<String>,
    ) -> Result<Self, ConsumerParityError> {
        let receipt = Self {
            representation_ref: representation_ref.into(),
            consumer_ref: consumer_ref.into(),
            query_ref: query_ref.into(),
            left_state_ref: left_state_ref.into(),
            right_state_ref: right_state_ref.into(),
            left_outcome_ref: left_outcome_ref.into(),
            right_outcome_ref: right_outcome_ref.into(),
            residual_ref: residual_ref.into(),
            candidate_consumer_adequate: false,
            promotes_claim_truth: false,
            refutes_source_truth: false,
        };
        for (name, value) in [
            ("representation_ref", receipt.representation_ref.as_str()),
            ("consumer_ref", receipt.consumer_ref.as_str()),
            ("query_ref", receipt.query_ref.as_str()),
            ("left_state_ref", receipt.left_state_ref.as_str()),
            ("right_state_ref", receipt.right_state_ref.as_str()),
            ("left_outcome_ref", receipt.left_outcome_ref.as_str()),
            ("right_outcome_ref", receipt.right_outcome_ref.as_str()),
            ("residual_ref", receipt.residual_ref.as_str()),
        ] {
            nonempty(name, value)?;
        }
        if receipt.left_outcome_ref == receipt.right_outcome_ref {
            return Err(ConsumerParityError::CounterexampleDoesNotSeparateConsumer);
        }
        Ok(receipt)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerturbationExpectation {
    SemanticPreserving,
    SemanticChanging,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticPerturbationReceipt {
    pub perturbation_ref: String,
    pub expectation: PerturbationExpectation,
    pub consumer_ref: String,
    pub before_outcome_ref: String,
    pub after_outcome_ref: String,
    pub factorisation_receipt_ref: String,
    pub creates_causal_mechanism: bool,
    pub creates_scientific_authority: bool,
}

impl SemanticPerturbationReceipt {
    pub fn new(
        perturbation_ref: impl Into<String>,
        expectation: PerturbationExpectation,
        consumer_ref: impl Into<String>,
        before_outcome_ref: impl Into<String>,
        after_outcome_ref: impl Into<String>,
        factorisation_receipt_ref: impl Into<String>,
    ) -> Result<Self, ConsumerParityError> {
        let receipt = Self {
            perturbation_ref: perturbation_ref.into(),
            expectation,
            consumer_ref: consumer_ref.into(),
            before_outcome_ref: before_outcome_ref.into(),
            after_outcome_ref: after_outcome_ref.into(),
            factorisation_receipt_ref: factorisation_receipt_ref.into(),
            creates_causal_mechanism: false,
            creates_scientific_authority: false,
        };
        for (name, value) in [
            ("perturbation_ref", receipt.perturbation_ref.as_str()),
            ("consumer_ref", receipt.consumer_ref.as_str()),
            ("before_outcome_ref", receipt.before_outcome_ref.as_str()),
            ("after_outcome_ref", receipt.after_outcome_ref.as_str()),
            ("factorisation_receipt_ref", receipt.factorisation_receipt_ref.as_str()),
        ] {
            nonempty(name, value)?;
        }
        Ok(receipt)
    }

    pub fn expectation_satisfied(&self) -> bool {
        match self.expectation {
            PerturbationExpectation::SemanticPreserving => {
                self.before_outcome_ref == self.after_outcome_ref
            }
            PerturbationExpectation::SemanticChanging => {
                self.before_outcome_ref != self.after_outcome_ref
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerInformativenessReceipt {
    pub left: ConsumerAdequacyReceipt,
    pub right: ConsumerAdequacyReceipt,
    pub right_preserved_distinction_refs: BTreeSet<String>,
    pub left_preserved_distinction_refs: BTreeSet<String>,
    pub reviewed_factorisation_receipt_ref: String,
    pub creates_universal_superiority: bool,
    pub promotes_claim_truth: bool,
}

impl ConsumerInformativenessReceipt {
    pub fn new<RI, RS, LI, LS>(
        left: ConsumerAdequacyReceipt,
        right: ConsumerAdequacyReceipt,
        right_preserved_distinction_refs: RI,
        left_preserved_distinction_refs: LI,
        reviewed_factorisation_receipt_ref: impl Into<String>,
    ) -> Result<Self, ConsumerParityError>
    where
        RI: IntoIterator<Item = RS>,
        RS: Into<String>,
        LI: IntoIterator<Item = LS>,
        LS: Into<String>,
    {
        left.validate()?;
        right.validate()?;
        if left.consumer_ref != right.consumer_ref {
            return Err(ConsumerParityError::ConsumerMismatch);
        }
        if left.query_family_ref != right.query_family_ref {
            return Err(ConsumerParityError::QueryFamilyMismatch);
        }
        if !left.eligible() || !right.eligible() {
            return Err(ConsumerParityError::IneligibleRepresentation);
        }
        let right_preserved_distinction_refs: BTreeSet<String> = right_preserved_distinction_refs
            .into_iter()
            .map(Into::into)
            .collect();
        let left_preserved_distinction_refs: BTreeSet<String> = left_preserved_distinction_refs
            .into_iter()
            .map(Into::into)
            .collect();
        if right_preserved_distinction_refs
            .iter()
            .chain(left_preserved_distinction_refs.iter())
            .any(|value| value.trim().is_empty())
        {
            return Err(ConsumerParityError::EmptyDistinctionReference);
        }
        let reviewed_factorisation_receipt_ref = reviewed_factorisation_receipt_ref.into();
        nonempty(
            "reviewed_factorisation_receipt_ref",
            &reviewed_factorisation_receipt_ref,
        )?;
        if !right_preserved_distinction_refs.is_subset(&left_preserved_distinction_refs) {
            return Err(ConsumerParityError::InformativenessCoverageMissing);
        }
        Ok(Self {
            left,
            right,
            right_preserved_distinction_refs,
            left_preserved_distinction_refs,
            reviewed_factorisation_receipt_ref,
            creates_universal_superiority: false,
            promotes_claim_truth: false,
        })
    }

    pub fn left_at_least_as_informative(&self) -> bool {
        self.right_preserved_distinction_refs
            .is_subset(&self.left_preserved_distinction_refs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationDominanceReceipt {
    pub informativeness: ConsumerInformativenessReceipt,
    pub separating_distinction_ref: String,
    pub left_preserves_separator_receipt_ref: String,
    pub right_fails_separator_receipt_ref: String,
    pub creates_universal_superiority: bool,
    pub complexity_rank_creates_authority: bool,
    pub promotes_claim_truth: bool,
}

impl RepresentationDominanceReceipt {
    pub fn new(
        informativeness: ConsumerInformativenessReceipt,
        separating_distinction_ref: impl Into<String>,
        left_preserves_separator_receipt_ref: impl Into<String>,
        right_fails_separator_receipt_ref: impl Into<String>,
    ) -> Result<Self, ConsumerParityError> {
        let separating_distinction_ref = separating_distinction_ref.into();
        let left_preserves_separator_receipt_ref = left_preserves_separator_receipt_ref.into();
        let right_fails_separator_receipt_ref = right_fails_separator_receipt_ref.into();
        nonempty("separating_distinction_ref", &separating_distinction_ref)?;
        nonempty(
            "left_preserves_separator_receipt_ref",
            &left_preserves_separator_receipt_ref,
        )?;
        nonempty(
            "right_fails_separator_receipt_ref",
            &right_fails_separator_receipt_ref,
        )?;
        if !informativeness.left_at_least_as_informative()
            || !informativeness
                .left_preserved_distinction_refs
                .contains(&separating_distinction_ref)
            || informativeness
                .right_preserved_distinction_refs
                .contains(&separating_distinction_ref)
        {
            return Err(ConsumerParityError::NoStrictSeparator);
        }
        Ok(Self {
            informativeness,
            separating_distinction_ref,
            left_preserves_separator_receipt_ref,
            right_fails_separator_receipt_ref,
            creates_universal_superiority: false,
            complexity_rank_creates_authority: false,
            promotes_claim_truth: false,
        })
    }

    pub fn is_strict_for_declared_consumer(&self) -> bool {
        self.informativeness.left_at_least_as_informative()
            && self
                .informativeness
                .left_preserved_distinction_refs
                .contains(&self.separating_distinction_ref)
            && !self
                .informativeness
                .right_preserved_distinction_refs
                .contains(&self.separating_distinction_ref)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscriminationHandoffReceipt {
    pub consumer_ref: String,
    pub query_family_ref: String,
    pub residual_ref: String,
    pub discriminator_design_ref: String,
    pub investigation_obligation_ref: String,
    pub pareto_frontier_ref: String,
    pub legal_follow_intent_ref: String,
    pub source_role_ref: String,
    pub jurisdiction_ref: String,
    pub experimental_design_creates_truth: bool,
    pub pareto_priority_creates_truth: bool,
    pub creates_access_authority: bool,
    pub creates_legal_authority: bool,
    pub resolves_residual: bool,
}

impl DiscriminationHandoffReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        consumer_ref: impl Into<String>,
        query_family_ref: impl Into<String>,
        residual_ref: impl Into<String>,
        discriminator_design_ref: impl Into<String>,
        investigation_obligation_ref: impl Into<String>,
        pareto_frontier_ref: impl Into<String>,
        legal_follow_intent_ref: impl Into<String>,
        source_role_ref: impl Into<String>,
        jurisdiction_ref: impl Into<String>,
    ) -> Result<Self, ConsumerParityError> {
        let receipt = Self {
            consumer_ref: consumer_ref.into(),
            query_family_ref: query_family_ref.into(),
            residual_ref: residual_ref.into(),
            discriminator_design_ref: discriminator_design_ref.into(),
            investigation_obligation_ref: investigation_obligation_ref.into(),
            pareto_frontier_ref: pareto_frontier_ref.into(),
            legal_follow_intent_ref: legal_follow_intent_ref.into(),
            source_role_ref: source_role_ref.into(),
            jurisdiction_ref: jurisdiction_ref.into(),
            experimental_design_creates_truth: false,
            pareto_priority_creates_truth: false,
            creates_access_authority: false,
            creates_legal_authority: false,
            resolves_residual: false,
        };
        for (name, value) in [
            ("consumer_ref", receipt.consumer_ref.as_str()),
            ("query_family_ref", receipt.query_family_ref.as_str()),
            ("residual_ref", receipt.residual_ref.as_str()),
            ("discriminator_design_ref", receipt.discriminator_design_ref.as_str()),
            ("investigation_obligation_ref", receipt.investigation_obligation_ref.as_str()),
            ("pareto_frontier_ref", receipt.pareto_frontier_ref.as_str()),
            ("legal_follow_intent_ref", receipt.legal_follow_intent_ref.as_str()),
            ("source_role_ref", receipt.source_role_ref.as_str()),
            ("jurisdiction_ref", receipt.jurisdiction_ref.as_str()),
        ] {
            nonempty(name, value)?;
        }
        Ok(receipt)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumerParityError {
    EmptyCoordinate(&'static str),
    EmptyFactorisationReceipt,
    AdequacyWithoutFactorisation,
    ConsumerMismatch,
    QueryFamilyMismatch,
    IneligibleRepresentation,
    NoExpressiveAsymmetry,
    CounterexampleDoesNotSeparateConsumer,
    EmptyDistinctionReference,
    InformativenessCoverageMissing,
    NoStrictSeparator,
}

impl fmt::Display for ConsumerParityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCoordinate(name) => write!(f, "empty coordinate: {name}"),
            Self::EmptyFactorisationReceipt => write!(f, "empty factorisation receipt"),
            Self::AdequacyWithoutFactorisation => write!(f, "adequacy without factorisation"),
            Self::ConsumerMismatch => write!(f, "consumer mismatch"),
            Self::QueryFamilyMismatch => write!(f, "query family mismatch"),
            Self::IneligibleRepresentation => write!(f, "ineligible representation"),
            Self::NoExpressiveAsymmetry => write!(f, "no expressive asymmetry"),
            Self::CounterexampleDoesNotSeparateConsumer => {
                write!(f, "counterexample does not separate consumer outcomes")
            }
            Self::EmptyDistinctionReference => write!(f, "empty distinction reference"),
            Self::InformativenessCoverageMissing => {
                write!(f, "left representation does not preserve every right-side distinction")
            }
            Self::NoStrictSeparator => write!(f, "no strict consumer-relative separator"),
        }
    }
}

impl std::error::Error for ConsumerParityError {}
