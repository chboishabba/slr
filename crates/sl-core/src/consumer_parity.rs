use std::collections::BTreeSet;

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
pub enum ConsumerParityError {
    EmptyCoordinate(&'static str),
    EmptyFactorisationReceipt,
    AdequacyWithoutFactorisation,
    ConsumerMismatch,
    QueryFamilyMismatch,
    IneligibleRepresentation,
    NoExpressiveAsymmetry,
    CounterexampleDoesNotSeparateConsumer,
}
