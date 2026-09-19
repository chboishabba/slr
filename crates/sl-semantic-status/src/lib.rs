//! Orthogonal semantic status carriers for SensibLaw.
//!
//! This crate sits after parser/semantic candidate emission and before governed
//! admission.  It intentionally separates role, identity, antecedent,
//! occurrence, proposition status/truth, attribution, evidence, modality,
//! temporal state and legal applicability so no one coordinate silently closes
//! another.  It contains no regex semantics, admission shortcut or publication API.

use sensiblaw_core::{FibreAddress, TextSpan};
use sensiblaw_semantic_expansion::{ExpandedCandidateKind, ScopeState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParticipantRole {
    Agent,
    Patient,
    Theme,
    Experiencer,
    Recipient,
    Beneficiary,
    Instrument,
    Location,
    Source,
    Goal,
    Cause,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegalParticipantRole {
    Claimant,
    Respondent,
    Applicant,
    Authority,
    DecisionMaker,
    RightsBearer,
    DutyBearer,
    PowerHolder,
    LiabilityBearer,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferentKind {
    Entity,
    Eventuality,
    Proposition,
    Time,
    Place,
    Rule,
    Document,
    Span,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IdentityStatus {
    Unresolved,
    CandidateSet,
    ResolvedSame,
    ResolvedDistinct,
    NotMaterialToConsumer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AntecedentStatus {
    Unresolved,
    CandidateSet,
    Narrowed,
    Resolved,
    NotMaterialToConsumer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OccurrenceStatus {
    Unresolved,
    MentionedEventuality,
    Asserted,
    Reported,
    Alleged,
    Hypothetical,
    Conditional,
    Negated,
    Counterfactual,
    PlannedFuture,
    Questioned,
    Admitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PropositionStatus {
    Unresolved,
    Represented,
    AssertedBySource,
    Alleged,
    Admitted,
    Denied,
    FoundAsFact,
    HeldByCourt,
    Assumed,
    Hypothetical,
    QuotedReported,
    Distinguished,
    Rejected,
    NotDetermined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TruthStatus {
    Unresolved,
    CandidateTrue,
    CandidateFalse,
    AdmittedTrue,
    AdmittedFalse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributionRole {
    Author,
    Speaker,
    Reporter,
    QuotedSpeaker,
    PropositionSource,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidencePolarity {
    For,
    Against,
    Neutral,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceKind {
    Source,
    Testimonial,
    Documentary,
    Parser,
    Provenance,
    External,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModalityKind {
    Deontic,
    Epistemic,
    DynamicAbility,
    Bouletic,
    Teleological,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModalForce {
    Obligation,
    Permission,
    Prohibition,
    Possibility,
    Necessity,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatusScope {
    Unresolved,
    LocalCandidate,
    Resolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TemporalRelationKind {
    EventTime,
    ReferenceTime,
    DocumentTime,
    LegalEffectiveTime,
    ValidityInterval,
    Commencement,
    Expiry,
    Repeal,
    Amendment,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConditionKind {
    Antecedent,
    Exception,
    Defeater,
    Unless,
    ProvidedThat,
    SubjectTo,
    Override,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JurisdictionKind {
    Geographic,
    LegalSystem,
    Court,
    Personal,
    SubjectMatter,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthorityKind {
    Source,
    Legal,
    Institutional,
    Promotion,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApplicabilityStatus {
    Unresolved,
    Candidate,
    Admitted,
    InapplicableAdmitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationStatus {
    Unresolved,
    Candidate,
    Admitted,
    NoViolationAdmitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiabilityStatus {
    Unresolved,
    Candidate,
    Admitted,
    NoLiabilityAdmitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BurdenKind {
    Evidential,
    Persuasive,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StandardOfProof {
    BeyondReasonableDoubt,
    BalanceOfProbabilities,
    ClearAndConvincing,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JudicialDiscourseStatus {
    Holding,
    RatioCandidate,
    Obiter,
    FindingOfFact,
    Submission,
    Allegation,
    Order,
    Disposition,
    Distinguished,
    Followed,
    Overruled,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NormativeRelation {
    Duty,
    Permission,
    Power,
    Liability,
    Right,
    Privilege,
    Immunity,
    Disability,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticSubjectStatus {
    pub reference: String,
    pub referent_kind: ReferentKind,
    pub participant_role: ParticipantRole,
    pub legal_role: LegalParticipantRole,
    pub identity: IdentityStatus,
    pub antecedent: AntecedentStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStatusProduct {
    pub reference: String,
    pub occurrence: OccurrenceStatus,
    pub temporal_relation: TemporalRelationKind,
    pub scope: StatusScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionStatusProduct {
    pub reference: String,
    pub proposition_status: PropositionStatus,
    pub truth_status: TruthStatus,
    pub attribution: AttributionRole,
    pub evidence_polarity: EvidencePolarity,
    pub evidence_kind: EvidenceKind,
    pub modality_kind: ModalityKind,
    pub modal_force: ModalForce,
    pub modal_scope: StatusScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalStatusProduct {
    pub jurisdiction: JurisdictionKind,
    pub authority: AuthorityKind,
    pub condition: ConditionKind,
    pub applicability: ApplicabilityStatus,
    pub violation: ViolationStatus,
    pub liability: LiabilityStatus,
    pub burden: BurdenKind,
    pub standard_of_proof: StandardOfProof,
    pub judicial_status: JudicialDiscourseStatus,
    pub normative_relation: NormativeRelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateOrigin {
    pub kind: ExpandedCandidateKind,
    pub span: TextSpan,
    pub address: FibreAddress,
    pub expansion_scope: ScopeState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticCommitmentState {
    pub origin: CandidateOrigin,
    pub subjects: Vec<SemanticSubjectStatus>,
    pub events: Vec<EventStatusProduct>,
    pub propositions: Vec<PropositionStatusProduct>,
    pub legal: Vec<LegalStatusProduct>,
    pub candidate_only: bool,
    pub governed_admission_present: bool,
}

impl SemanticCommitmentState {
    pub fn unresolved(origin: CandidateOrigin) -> Self {
        Self {
            origin,
            subjects: Vec::new(),
            events: Vec::new(),
            propositions: Vec::new(),
            legal: Vec::new(),
            candidate_only: true,
            governed_admission_present: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityResolutionReceipt {
    pub subject_reference: String,
    pub resulting_status: IdentityStatus,
    pub candidate_set_reference: String,
    pub evidence_references: Vec<String>,
    pub resolver_reference: String,
    pub policy_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntecedentResolutionReceipt {
    pub subject_reference: String,
    pub resulting_status: AntecedentStatus,
    pub candidate_set_reference: String,
    pub accessibility_witness_references: Vec<String>,
    pub resolver_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccurrenceResolutionReceipt {
    pub event_reference: String,
    pub resulting_status: OccurrenceStatus,
    pub proposition_support_references: Vec<String>,
    pub evidence_references: Vec<String>,
    pub resolver_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionResolutionReceipt {
    pub proposition_reference: String,
    pub resulting_proposition_status: PropositionStatus,
    pub resulting_truth_status: TruthStatus,
    pub attribution_reference: String,
    pub evidence_references: Vec<String>,
    pub resolver_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicabilityResolutionReceipt {
    pub resulting_status: ApplicabilityStatus,
    pub typed_meet_reference: String,
    pub jurisdiction_reference: String,
    pub temporal_reference: String,
    pub authority_reference: String,
    pub exception_reference: String,
    pub resolver_reference: String,
}

/// Consumer-facing projections may ignore axes, but ignored information remains
/// on the commitment state. This function never promotes or mutates authority.
pub fn occurrence_projection(state: &SemanticCommitmentState) -> Vec<OccurrenceStatus> {
    state.events.iter().map(|event| event.occurrence).collect()
}

pub fn truth_projection(state: &SemanticCommitmentState) -> Vec<TruthStatus> {
    state
        .propositions
        .iter()
        .map(|proposition| proposition.truth_status)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin() -> CandidateOrigin {
        CandidateOrigin {
            kind: ExpandedCandidateKind::ReferenceRelation,
            span: TextSpan::new(1, 10, 20).unwrap(),
            address: FibreAddress {
                sentence_id: 7,
                local_ordinal: 3,
            },
            expansion_scope: ScopeState::ContextRequired,
        }
    }

    #[test]
    fn unresolved_state_is_candidate_only_and_non_admitted() {
        let state = SemanticCommitmentState::unresolved(origin());
        assert!(state.candidate_only);
        assert!(!state.governed_admission_present);
        assert!(state.subjects.is_empty());
        assert!(state.events.is_empty());
        assert!(state.propositions.is_empty());
        assert!(state.legal.is_empty());
    }

    #[test]
    fn asserted_proposition_does_not_force_truth_axis() {
        let proposition = PropositionStatusProduct {
            reference: "p:1".into(),
            proposition_status: PropositionStatus::AssertedBySource,
            truth_status: TruthStatus::Unresolved,
            attribution: AttributionRole::Speaker,
            evidence_polarity: EvidencePolarity::Neutral,
            evidence_kind: EvidenceKind::Source,
            modality_kind: ModalityKind::Unresolved,
            modal_force: ModalForce::Unresolved,
            modal_scope: StatusScope::Unresolved,
        };
        assert_eq!(proposition.truth_status, TruthStatus::Unresolved);
    }

    #[test]
    fn mentioned_event_does_not_force_occurrence() {
        let event = EventStatusProduct {
            reference: "e:1".into(),
            occurrence: OccurrenceStatus::MentionedEventuality,
            temporal_relation: TemporalRelationKind::Unresolved,
            scope: StatusScope::Unresolved,
        };
        assert_ne!(event.occurrence, OccurrenceStatus::Admitted);
    }

    #[test]
    fn linguistic_and_legal_roles_are_independent_axes() {
        let subject = SemanticSubjectStatus {
            reference: "x:1".into(),
            referent_kind: ReferentKind::Entity,
            participant_role: ParticipantRole::Agent,
            legal_role: LegalParticipantRole::Unresolved,
            identity: IdentityStatus::CandidateSet,
            antecedent: AntecedentStatus::CandidateSet,
        };
        assert_eq!(subject.participant_role, ParticipantRole::Agent);
        assert_eq!(subject.legal_role, LegalParticipantRole::Unresolved);
    }

    #[test]
    fn applicability_does_not_force_violation_or_liability() {
        let legal = LegalStatusProduct {
            jurisdiction: JurisdictionKind::Unresolved,
            authority: AuthorityKind::Unresolved,
            condition: ConditionKind::Unresolved,
            applicability: ApplicabilityStatus::Candidate,
            violation: ViolationStatus::Unresolved,
            liability: LiabilityStatus::Unresolved,
            burden: BurdenKind::Unresolved,
            standard_of_proof: StandardOfProof::Unresolved,
            judicial_status: JudicialDiscourseStatus::Unresolved,
            normative_relation: NormativeRelation::Unresolved,
        };
        assert_eq!(legal.violation, ViolationStatus::Unresolved);
        assert_eq!(legal.liability, LiabilityStatus::Unresolved);
    }
}
