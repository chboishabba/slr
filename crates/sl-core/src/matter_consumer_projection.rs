// Consumer-relative representation receipts bound to one persisted Matter.
//
// This is a pure receipt layer. It does not persist or manufacture admission /
// factorisation evidence: callers must supply the already-reviewed
// ConsumerAdequacyReceipt / ConsumerInformativenessReceipt owned by the
// canonical evidence substrate. A later PG/UI acceptance may reopen such a
// receipt only when its evidence ancestry has a real durable owner. Until that
// producer exists, this module must not turn environment strings or UI wording
// into a purported live parity receipt.
//
// `MatterPersona` is presentation/consumer metadata, not an authority class.
// Switching persona cannot create a new Matter, change evidentiary status, or
// confer adjudicative authority.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatterPersona {
    AffectedPerson,
    SolicitorCounsel,
    CourtAssociate,
    Novice,
    Expert,
    DomainSpecialist,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterRepresentationProjection {
    pub matter_ref: String,
    pub controversy_ref: String,
    pub persona: MatterPersona,
    pub representation_ref: String,
    pub adequacy: ConsumerAdequacyReceipt,
    pub projection_receipt_ref: String,
    pub creates_new_matter: bool,
    pub creates_semantic_authority: bool,
    pub determines_credibility: bool,
    pub enters_final_judgment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SameMatterInformativenessReceipt {
    pub left: MatterRepresentationProjection,
    pub right: MatterRepresentationProjection,
    pub informativeness: ConsumerInformativenessReceipt,
    pub receipt_ref: String,
    pub creates_new_matter: bool,
    pub creates_semantic_authority: bool,
    pub determines_credibility: bool,
    pub enters_final_judgment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatterConsumerProjectionError {
    EmptyCoordinate(&'static str),
    InvalidAdequacy,
    IneligibleRepresentation,
    RepresentationProjectionMismatch,
    MatterMismatch,
    ControversyMismatch,
    InformativenessProjectionMismatch,
}

impl MatterRepresentationProjection {
    pub fn new(
        matter_ref: impl Into<String>,
        controversy_ref: impl Into<String>,
        persona: MatterPersona,
        representation_ref: impl Into<String>,
        adequacy: ConsumerAdequacyReceipt,
        projection_receipt_ref: impl Into<String>,
    ) -> Result<Self, MatterConsumerProjectionError> {
        let projection = Self {
            matter_ref: matter_ref.into(),
            controversy_ref: controversy_ref.into(),
            persona,
            representation_ref: representation_ref.into(),
            adequacy,
            projection_receipt_ref: projection_receipt_ref.into(),
            creates_new_matter: false,
            creates_semantic_authority: false,
            determines_credibility: false,
            enters_final_judgment: false,
        };
        projection.validate()?;
        Ok(projection)
    }

    pub fn validate(&self) -> Result<(), MatterConsumerProjectionError> {
        for (name, value) in [
            ("matter_ref", self.matter_ref.as_str()),
            ("controversy_ref", self.controversy_ref.as_str()),
            ("representation_ref", self.representation_ref.as_str()),
            ("projection_receipt_ref", self.projection_receipt_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(MatterConsumerProjectionError::EmptyCoordinate(name));
            }
        }
        self.adequacy
            .validate()
            .map_err(|_| MatterConsumerProjectionError::InvalidAdequacy)?;
        if !self.adequacy.eligible() {
            return Err(MatterConsumerProjectionError::IneligibleRepresentation);
        }
        if self.representation_ref != self.adequacy.representation_ref {
            return Err(MatterConsumerProjectionError::RepresentationProjectionMismatch);
        }
        Ok(())
    }
}

impl SameMatterInformativenessReceipt {
    pub fn new(
        left: MatterRepresentationProjection,
        right: MatterRepresentationProjection,
        informativeness: ConsumerInformativenessReceipt,
        receipt_ref: impl Into<String>,
    ) -> Result<Self, MatterConsumerProjectionError> {
        left.validate()?;
        right.validate()?;
        if left.matter_ref != right.matter_ref {
            return Err(MatterConsumerProjectionError::MatterMismatch);
        }
        if left.controversy_ref != right.controversy_ref {
            return Err(MatterConsumerProjectionError::ControversyMismatch);
        }

        // Re-run the canonical consumer-informativeness constructor rather than
        // trusting a hand-built struct. This rechecks exact consumer/query
        // identity, eligibility, non-empty factorisation ancestry and coverage.
        let validated = ConsumerInformativenessReceipt::new(
            informativeness.left.clone(),
            informativeness.right.clone(),
            informativeness.right_preserved_distinction_refs.clone(),
            informativeness.left_preserved_distinction_refs.clone(),
            informativeness.reviewed_factorisation_receipt_ref.clone(),
        )
        .map_err(|_| MatterConsumerProjectionError::InformativenessProjectionMismatch)?;
        if validated != informativeness
            || informativeness.left != left.adequacy
            || informativeness.right != right.adequacy
            || !informativeness.left_at_least_as_informative()
        {
            return Err(MatterConsumerProjectionError::InformativenessProjectionMismatch);
        }
        let receipt_ref = receipt_ref.into();
        if receipt_ref.trim().is_empty() {
            return Err(MatterConsumerProjectionError::EmptyCoordinate("receipt_ref"));
        }
        Ok(Self {
            left,
            right,
            informativeness,
            receipt_ref,
            creates_new_matter: false,
            creates_semantic_authority: false,
            determines_credibility: false,
            enters_final_judgment: false,
        })
    }

    pub fn matter_ref(&self) -> &str {
        &self.left.matter_ref
    }

    pub fn controversy_ref(&self) -> &str {
        &self.left.controversy_ref
    }
}
