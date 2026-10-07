//! Typed adversarial legal controversy domain.
//!
//! Runtime counterpart of the Justice-Lee/DASHI adversarial proof-graph owner.
//! This layer structures party propositions, typed responses, unresolved
//! controversy and proof obligations. It never creates semantic/legal authority,
//! applicability, claim truth, credibility findings, ultimate facts, normative
//! weight, or final judgment.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyRole {
    Applicant,
    Respondent,
    Court,
    ExternalWitness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegalRole {
    PleadedFact,
    EvidentiaryFact,
    LegalProposition,
    CausalLink,
    RequestedOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpistemicStatus {
    Alleged,
    Admitted,
    Disputed,
    Supported,
    Proved,
    Rejected,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    SourceText,
    AccountRecord,
    Message,
    Report,
    Testimony,
    EventRecord,
    OtherEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseMode {
    DenyOccurrence,
    AdmitOccurrenceDisputeCharacterisation,
    AdmitConductAddContext,
    DisputeCausation,
    ChallengeEvidenceReliability,
    OfferAlternativeEvent,
    AdmitProposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisagreementKind {
    Node,
    Relation,
    Evidence,
    Characterisation,
    Causal,
    LegalConsequence,
    NormativeOrderMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationKind {
    Premise,
    Evidence,
    Response,
    Discriminator,
    Adjudicative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProceduralGoal {
    IdentifyCommonGround,
    IsolateResidualControversy,
    DecideEvidenceNeeded,
    PrepareForAdjudication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegalControversyError {
    EmptyCoordinate(&'static str),
    MissingReviewedEvidence,
    SelfResponse,
    EmptyMembership(&'static str),
    ObligationDischargeMismatch,
    PromotionNotAllowed,
}

fn required(name: &'static str, value: &str) -> Result<(), LegalControversyError> {
    if value.trim().is_empty() {
        Err(LegalControversyError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn validate_refs(
    name: &'static str,
    values: &[String],
    may_be_empty: bool,
) -> Result<(), LegalControversyError> {
    if !may_be_empty && values.is_empty() {
        return Err(LegalControversyError::EmptyMembership(name));
    }
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(LegalControversyError::EmptyCoordinate(name));
    }
    Ok(())
}

fn non_promoting(
    candidate_only: bool,
    creates_semantic_authority: bool,
    creates_legal_authority: bool,
    applicability_promoted: bool,
    claim_truth_promoted: bool,
) -> Result<(), LegalControversyError> {
    if !candidate_only
        || creates_semantic_authority
        || creates_legal_authority
        || applicability_promoted
        || claim_truth_promoted
    {
        Err(LegalControversyError::PromotionNotAllowed)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalPropositionFibre {
    pub fibre_ref: String,
    pub claim_ref: String,
    pub party: PartyRole,
    pub legal_role: LegalRole,
    pub epistemic_status: EpistemicStatus,
    pub evidence_kind_ref: String,
    pub source_reference: String,
    pub reviewed_evidence_ref: Option<String>,
    pub normative_order_ref: String,
    pub temporal_reference: String,
    pub relation_reference: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl LegalPropositionFibre {
    pub fn validate(&self) -> Result<(), LegalControversyError> {
        for (name, value) in [
            ("fibre_ref", self.fibre_ref.as_str()),
            ("claim_ref", self.claim_ref.as_str()),
            ("evidence_kind_ref", self.evidence_kind_ref.as_str()),
            ("source_reference", self.source_reference.as_str()),
            ("normative_order_ref", self.normative_order_ref.as_str()),
            ("temporal_reference", self.temporal_reference.as_str()),
            ("relation_reference", self.relation_reference.as_str()),
        ] {
            required(name, value)?;
        }
        if let Some(reviewed) = self.reviewed_evidence_ref.as_deref() {
            required("reviewed_evidence_ref", reviewed)?;
        }
        if matches!(
            self.epistemic_status,
            EpistemicStatus::Admitted
                | EpistemicStatus::Supported
                | EpistemicStatus::Proved
                | EpistemicStatus::Rejected
        ) && self.reviewed_evidence_ref.is_none()
        {
            return Err(LegalControversyError::MissingReviewedEvidence);
        }
        non_promoting(
            self.candidate_only,
            self.creates_semantic_authority,
            self.creates_legal_authority,
            self.applicability_promoted,
            self.claim_truth_promoted,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedResponseEdge {
    pub response_ref: String,
    pub controversy_ref: String,
    pub target_fibre_ref: String,
    pub response_fibre_ref: String,
    pub mode: ResponseMode,
    pub response_reference: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl TypedResponseEdge {
    pub fn validate(&self) -> Result<(), LegalControversyError> {
        for (name, value) in [
            ("response_ref", self.response_ref.as_str()),
            ("controversy_ref", self.controversy_ref.as_str()),
            ("target_fibre_ref", self.target_fibre_ref.as_str()),
            ("response_fibre_ref", self.response_fibre_ref.as_str()),
            ("response_reference", self.response_reference.as_str()),
        ] {
            required(name, value)?;
        }
        if self.target_fibre_ref == self.response_fibre_ref {
            return Err(LegalControversyError::SelfResponse);
        }
        non_promoting(
            self.candidate_only,
            self.creates_semantic_authority,
            self.creates_legal_authority,
            self.applicability_promoted,
            self.claim_truth_promoted,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalControversyResidual {
    pub residual_ref: String,
    pub controversy_ref: String,
    pub kind: DisagreementKind,
    pub applicant_fibre_ref: String,
    pub respondent_fibre_ref: String,
    pub residual_level_ref: String,
    pub unresolved_question: String,
    pub requested_discriminator: Option<String>,
    pub target_evidence_query: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl LegalControversyResidual {
    pub fn validate(&self) -> Result<(), LegalControversyError> {
        for (name, value) in [
            ("residual_ref", self.residual_ref.as_str()),
            ("controversy_ref", self.controversy_ref.as_str()),
            ("applicant_fibre_ref", self.applicant_fibre_ref.as_str()),
            ("respondent_fibre_ref", self.respondent_fibre_ref.as_str()),
            ("residual_level_ref", self.residual_level_ref.as_str()),
            ("unresolved_question", self.unresolved_question.as_str()),
        ] {
            required(name, value)?;
        }
        if let Some(value) = self.requested_discriminator.as_deref() {
            required("requested_discriminator", value)?;
        }
        if let Some(value) = self.target_evidence_query.as_deref() {
            required("target_evidence_query", value)?;
        }
        non_promoting(
            self.candidate_only,
            self.creates_semantic_authority,
            self.creates_legal_authority,
            self.applicability_promoted,
            self.claim_truth_promoted,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalProofObligation {
    pub obligation_ref: String,
    pub controversy_ref: String,
    pub proposition_fibre_ref: String,
    pub kind: ObligationKind,
    pub required_by: String,
    pub discharge_reference: Option<String>,
    pub is_open: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl LegalProofObligation {
    pub fn validate(&self) -> Result<(), LegalControversyError> {
        for (name, value) in [
            ("obligation_ref", self.obligation_ref.as_str()),
            ("controversy_ref", self.controversy_ref.as_str()),
            ("proposition_fibre_ref", self.proposition_fibre_ref.as_str()),
            ("required_by", self.required_by.as_str()),
        ] {
            required(name, value)?;
        }
        if let Some(value) = self.discharge_reference.as_deref() {
            required("discharge_reference", value)?;
        }
        if self.is_open == self.discharge_reference.is_some() {
            return Err(LegalControversyError::ObligationDischargeMismatch);
        }
        non_promoting(
            self.candidate_only,
            self.creates_semantic_authority,
            self.creates_legal_authority,
            self.applicability_promoted,
            self.claim_truth_promoted,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalControversyMatter {
    pub controversy_ref: String,
    pub matter_ref: String,
    pub root_fibre_ref: String,
    pub proposition_fibre_refs: Vec<String>,
    pub response_refs: Vec<String>,
    pub residual_refs: Vec<String>,
    pub obligation_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl LegalControversyMatter {
    pub fn validate(&self) -> Result<(), LegalControversyError> {
        for (name, value) in [
            ("controversy_ref", self.controversy_ref.as_str()),
            ("matter_ref", self.matter_ref.as_str()),
            ("root_fibre_ref", self.root_fibre_ref.as_str()),
        ] {
            required(name, value)?;
        }
        validate_refs("proposition_fibre_refs", &self.proposition_fibre_refs, false)?;
        validate_refs("response_refs", &self.response_refs, true)?;
        validate_refs("residual_refs", &self.residual_refs, true)?;
        validate_refs("obligation_refs", &self.obligation_refs, true)?;
        if !self.proposition_fibre_refs.iter().any(|value| value == &self.root_fibre_ref) {
            return Err(LegalControversyError::EmptyMembership("root_fibre_ref"));
        }
        non_promoting(
            self.candidate_only,
            self.creates_semantic_authority,
            self.creates_legal_authority,
            self.applicability_promoted,
            self.claim_truth_promoted,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseLegalProofSearch {
    pub reverse_ref: String,
    pub controversy_ref: String,
    pub goal: ProceduralGoal,
    pub open_obligation_refs: Vec<String>,
    pub candidate_residual_refs: Vec<String>,
    pub requested_discriminator: String,
    pub target_evidence_query: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl ReverseLegalProofSearch {
    pub fn validate(&self) -> Result<(), LegalControversyError> {
        required("reverse_ref", &self.reverse_ref)?;
        required("controversy_ref", &self.controversy_ref)?;
        validate_refs("open_obligation_refs", &self.open_obligation_refs, true)?;
        validate_refs("candidate_residual_refs", &self.candidate_residual_refs, true)?;
        required("requested_discriminator", &self.requested_discriminator)?;
        required("target_evidence_query", &self.target_evidence_query)?;
        non_promoting(
            self.candidate_only,
            self.creates_semantic_authority,
            self.creates_legal_authority,
            self.applicability_promoted,
            self.claim_truth_promoted,
        )
    }
}
