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
        if informativeness.left.representation_ref != left.representation_ref
            || informativeness.right.representation_ref != right.representation_ref
            || informativeness.left.consumer_ref != left.adequacy.consumer_ref
            || informativeness.right.consumer_ref != right.adequacy.consumer_ref
            || informativeness.left.query_family_ref != left.adequacy.query_family_ref
            || informativeness.right.query_family_ref != right.adequacy.query_family_ref
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
