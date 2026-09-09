//! Runtime parity carrier for SensibLaw atomic legal attribution.
//!
//! This crate operationalises a deliberately small subset of the Agda legal
//! spine. It preserves, for one exact case/revision atom:
//!
//! source-defined proposition -> case-outcome evidence -> repository evaluation
//! -> balanced ternary gate, while keeping semantic/legal status orthogonal.
//!
//! It does not implement the universal legal-rule theorem algebra and it does
//! not create legal authority, applicability, violation, liability or remedy.

pub mod admission;
pub mod priority;
pub mod rule;

use std::collections::BTreeMap;

use sensiblaw_core::{PromotionReceipt as CorePromotionReceipt, PromotionStatus};
use sensiblaw_semantic_status::{
    ApplicabilityStatus, AttributionRole, AuthorityKind, BurdenKind, ConditionKind, EvidenceKind,
    EvidencePolarity, JudicialDiscourseStatus, JurisdictionKind, LegalStatusProduct,
    LiabilityStatus, ModalForce, ModalityKind, NormativeRelation, PropositionStatus,
    PropositionStatusProduct, StandardOfProof, StatusScope, TruthStatus, ViolationStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProvenanceStage {
    ExternalSourceClaim,
    SecondaryInterpretation,
    RepositoryReconstruction,
    CrossSourceInference,
    RepositoryTheoremExtension,
    PromotionOrExternalAdjudication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AtomicGate {
    FailsThisAtom,
    UnresolvedThisAtom,
    FitsThisAtom,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceAttachment {
    pub source_id: String,
    pub exact_locator: String,
    pub source_form: String,
    pub proposition_authority_role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimLineage {
    pub proposition_id: String,
    pub stage: ProvenanceStage,
    pub supporting_source: Option<SourceAttachment>,
    pub lineage_reference: String,
}

impl ClaimLineage {
    pub fn reconstruction(
        proposition_id: impl Into<String>,
        source: SourceAttachment,
        lineage_reference: impl Into<String>,
    ) -> Self {
        Self {
            proposition_id: proposition_id.into(),
            stage: ProvenanceStage::RepositoryReconstruction,
            supporting_source: Some(source),
            lineage_reference: lineage_reference.into(),
        }
    }

    pub fn cross_source_inference(
        proposition_id: impl Into<String>,
        lineage_reference: impl Into<String>,
    ) -> Self {
        Self {
            proposition_id: proposition_id.into(),
            stage: ProvenanceStage::CrossSourceInference,
            supporting_source: None,
            lineage_reference: lineage_reference.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicRegistryEntry {
    pub case_context: String,
    pub atom_id: String,
    /// Provenance of the repository proposition defining the legal atom.
    pub definition_lineage: ClaimLineage,
    /// Provenance of the concrete proposition used as case-outcome evidence.
    /// `None` is required for an unresolved gate; runtime must not invent an
    /// outcome source simply because an atom exists.
    pub outcome_lineage: Option<ClaimLineage>,
    /// Provenance of the repository act that maps definition + evidence to the
    /// atomic gate. Source material does not own this gate automatically.
    pub evaluation_lineage: ClaimLineage,
    pub gate: AtomicGate,
    pub proposition_status: PropositionStatusProduct,
    pub legal_status: LegalStatusProduct,
}

impl AtomicRegistryEntry {
    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.definition_lineage.proposition_id != self.atom_id {
            return Err(RegistryError::DefinitionAtomMismatch);
        }
        if self.evaluation_lineage.proposition_id != self.atom_id {
            return Err(RegistryError::EvaluationAtomMismatch);
        }
        if self.proposition_status.reference != self.atom_id {
            return Err(RegistryError::SemanticStatusAtomMismatch);
        }
        match self.gate {
            AtomicGate::UnresolvedThisAtom if self.outcome_lineage.is_some() => {
                Err(RegistryError::UnresolvedAtomHasOutcomeEvidence)
            }
            AtomicGate::FailsThisAtom | AtomicGate::FitsThisAtom
                if self.outcome_lineage.is_none() =>
            {
                Err(RegistryError::ResolvedAtomMissingOutcomeEvidence)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    DefinitionAtomMismatch,
    EvaluationAtomMismatch,
    SemanticStatusAtomMismatch,
    UnresolvedAtomHasOutcomeEvidence,
    ResolvedAtomMissingOutcomeEvidence,
    ConflictingCanonicalEntry,
    PromotionTargetMismatch,
    PromotionStageMismatch,
    PromotionSpanMismatch,
    PromotionStatusNotPromoted,
    MissingPromotionPolicy,
    MissingPromotionResolver,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AtomicCaseRegistry {
    entries: BTreeMap<(String, String), AtomicRegistryEntry>,
}

impl AtomicCaseRegistry {
    pub fn register(&mut self, entry: AtomicRegistryEntry) -> Result<(), RegistryError> {
        entry.validate()?;
        let key = (entry.case_context.clone(), entry.atom_id.clone());
        if let Some(existing) = self.entries.get(&key) {
            if existing == &entry {
                return Ok(());
            }
            return Err(RegistryError::ConflictingCanonicalEntry);
        }
        self.entries.insert(key, entry);
        Ok(())
    }

    pub fn get(&self, case_context: &str, atom_id: &str) -> Option<&AtomicRegistryEntry> {
        self.entries
            .get(&(case_context.to_owned(), atom_id.to_owned()))
    }

    pub fn entries(&self) -> impl Iterator<Item = &AtomicRegistryEntry> {
        self.entries.values()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Atom/context/provenance weld around the canonical `sensiblaw_core`
/// promotion receipt. This adds no second promotion authority type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicPromotionWeld {
    pub case_context: String,
    pub from_stage: ProvenanceStage,
    pub source_weld: admission::AdmittedAtomicSourceWeld,
    pub receipt: CorePromotionReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotedAtomicRecord {
    pub canonical_entry: AtomicRegistryEntry,
    pub promotion: AtomicPromotionWeld,
}

pub fn promote_atomic_entry(
    registry: &AtomicCaseRegistry,
    promotion: AtomicPromotionWeld,
) -> Result<PromotedAtomicRecord, RegistryError> {
    let entry = registry
        .get(&promotion.case_context, &promotion.source_weld.atom_id)
        .ok_or(RegistryError::PromotionTargetMismatch)?;
    if promotion.from_stage != entry.evaluation_lineage.stage {
        return Err(RegistryError::PromotionStageMismatch);
    }
    if promotion.receipt.status != PromotionStatus::Promoted {
        return Err(RegistryError::PromotionStatusNotPromoted);
    }
    if promotion.receipt.source_span != promotion.source_weld.source_anchor.span {
        return Err(RegistryError::PromotionSpanMismatch);
    }
    if promotion.receipt.policy_reference.trim().is_empty() {
        return Err(RegistryError::MissingPromotionPolicy);
    }
    if promotion
        .receipt
        .reviewer_or_resolver_reference
        .trim()
        .is_empty()
    {
        return Err(RegistryError::MissingPromotionResolver);
    }
    Ok(PromotedAtomicRecord {
        canonical_entry: entry.clone(),
        promotion,
    })
}

fn represented_source_status(atom_id: &str) -> PropositionStatusProduct {
    PropositionStatusProduct {
        reference: atom_id.to_owned(),
        proposition_status: PropositionStatus::Represented,
        truth_status: TruthStatus::Unresolved,
        attribution: AttributionRole::PropositionSource,
        evidence_polarity: EvidencePolarity::Neutral,
        evidence_kind: EvidenceKind::Source,
        modality_kind: ModalityKind::Unresolved,
        modal_force: ModalForce::Unresolved,
        modal_scope: StatusScope::Resolved,
    }
}

fn unresolved_legal_status() -> LegalStatusProduct {
    LegalStatusProduct {
        jurisdiction: JurisdictionKind::Court,
        authority: AuthorityKind::Unresolved,
        condition: ConditionKind::Unresolved,
        applicability: ApplicabilityStatus::Unresolved,
        violation: ViolationStatus::Unresolved,
        liability: LiabilityStatus::Unresolved,
        burden: BurdenKind::Unresolved,
        standard_of_proof: StandardOfProof::Unresolved,
        judicial_status: JudicialDiscourseStatus::Unresolved,
        normative_relation: NormativeRelation::Unresolved,
    }
}

fn source(
    source_id: &str,
    locator: &str,
    source_form: &str,
    role: &str,
) -> SourceAttachment {
    SourceAttachment {
        source_id: source_id.to_owned(),
        exact_locator: locator.to_owned(),
        source_form: source_form.to_owned(),
        proposition_authority_role: role.to_owned(),
    }
}

fn cullen_entry(
    atom_id: &str,
    gate: AtomicGate,
    definition_source: SourceAttachment,
    outcome_id: &str,
    outcome_source: SourceAttachment,
    evaluation_reference: &str,
) -> AtomicRegistryEntry {
    AtomicRegistryEntry {
        case_context: CULLEN_CONTEXT.to_owned(),
        atom_id: atom_id.to_owned(),
        definition_lineage: ClaimLineage::reconstruction(
            atom_id,
            definition_source,
            format!("DASHI/SLR formal reconstruction of legal test atom {atom_id}"),
        ),
        outcome_lineage: Some(ClaimLineage::reconstruction(
            outcome_id,
            outcome_source,
            format!("DASHI/SLR formal reconstruction of case-outcome evidence {outcome_id}"),
        )),
        evaluation_lineage: ClaimLineage::cross_source_inference(atom_id, evaluation_reference),
        gate,
        proposition_status: represented_source_status(atom_id),
        legal_status: unresolved_legal_status(),
    }
}

pub const CULLEN_CONTEXT: &str = "case:Cullen:[2026]HCA19:retained-fibre:r1";
pub const CLA_SOURCE_ID: &str = "source:NSW:Civil-Liability-Act-2002";
pub const VICARIOUS_ACT_SOURCE_ID: &str = "source:NSW:Law-Reform-Vicarious-Liability-Act-1983";
pub const CULLEN_SOURCE_ID: &str = "source:HCA:Cullen-v-NSW:2026:HCA19";

/// Bounded runtime fixture matching the current Agda Cullen atomic registry.
///
/// The five expected gates are (+,+,-,-,+). Their signs are not aggregated:
/// the two negative entries have different propositions, sources and legal
/// meanings, while the positive vicarious-family classification cannot repair
/// the failed breach atom.
pub fn cullen_gold_registry() -> AtomicCaseRegistry {
    let mut registry = AtomicCaseRegistry::default();

    let entries = [
        cullen_entry(
            "atom:NSW:CLA:s5B1a:risk-foreseeable",
            AtomicGate::FitsThisAtom,
            source(CLA_SOURCE_ID, "s 5B(1)(a)", "legislation", "legislative"),
            "evidence:Cullen:joint:39:foreseeable",
            source(CULLEN_SOURCE_ID, "joint reasons [39]", "judgment", "binding-ratio"),
            "SLR source-conditioned evaluation: s 5B(1)(a) + Cullen [39] => +1",
        ),
        cullen_entry(
            "atom:NSW:CLA:s5B1b:risk-not-insignificant",
            AtomicGate::FitsThisAtom,
            source(CLA_SOURCE_ID, "s 5B(1)(b)", "legislation", "legislative"),
            "evidence:Cullen:joint:39:not-insignificant",
            source(CULLEN_SOURCE_ID, "joint reasons [39]", "judgment", "binding-ratio"),
            "SLR source-conditioned evaluation: s 5B(1)(b) + Cullen [39] => +1",
        ),
        cullen_entry(
            "atom:NSW:CLA:s5B1c:reasonable-person-would-take-proposed-precautions",
            AtomicGate::FailsThisAtom,
            source(CLA_SOURCE_ID, "s 5B(1)(c)", "legislation", "legislative"),
            "evidence:Cullen:joint:42-48:proposed-precautions-fail",
            source(CULLEN_SOURCE_ID, "joint reasons [42]-[48]", "judgment", "binding-ratio"),
            "SLR source-conditioned evaluation: s 5B(1)(c) + Cullen [42]-[48] => -1",
        ),
        cullen_entry(
            "atom:NSW:CLA:s43A:liability-based-on-special-statutory-power",
            AtomicGate::FailsThisAtom,
            source(CLA_SOURCE_ID, "s 43A", "legislation", "legislative"),
            "evidence:Cullen:Edelman:64:not-pursuant-to-statutory-power",
            source(CULLEN_SOURCE_ID, "Edelman J [64]", "judgment", "concurrence"),
            "SLR source-conditioned evaluation: s 43A test + Edelman [64] => -1",
        ),
        cullen_entry(
            "atom:NSW:vicarious-liability:family-recognised",
            AtomicGate::FitsThisAtom,
            source(
                VICARIOUS_ACT_SOURCE_ID,
                "s 8",
                "legislation",
                "legislative",
            ),
            "evidence:Cullen:Edelman:100:vicarious-family",
            source(CULLEN_SOURCE_ID, "Edelman J [100]", "judgment", "concurrence"),
            "SLR source-conditioned evaluation: statutory family test + Edelman [100] => +1",
        ),
    ];

    for entry in entries {
        registry
            .register(entry)
            .expect("bounded Cullen gold fixture must be internally coherent");
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::{FibreAddress, TextSpan};
    use sensiblaw_semantic_admission::{
        AdmittedNormativeDelta, ResolutionAuthority, ResolvedScope,
    };
    use sensiblaw_semantic_expansion::{ExpandedCandidateKind, StableHeadRelation};

    #[test]
    fn cullen_gold_vector_is_exact_and_non_aggregated() {
        let registry = cullen_gold_registry();
        assert_eq!(registry.len(), 5);
        let expected = [
            ("atom:NSW:CLA:s5B1a:risk-foreseeable", AtomicGate::FitsThisAtom),
            ("atom:NSW:CLA:s5B1b:risk-not-insignificant", AtomicGate::FitsThisAtom),
            (
                "atom:NSW:CLA:s5B1c:reasonable-person-would-take-proposed-precautions",
                AtomicGate::FailsThisAtom,
            ),
            (
                "atom:NSW:CLA:s43A:liability-based-on-special-statutory-power",
                AtomicGate::FailsThisAtom,
            ),
            ("atom:NSW:vicarious-liability:family-recognised", AtomicGate::FitsThisAtom),
        ];
        for (atom, gate) in expected {
            assert_eq!(registry.get(CULLEN_CONTEXT, atom).unwrap().gate, gate);
        }
    }

    #[test]
    fn same_case_atom_cannot_be_reintroduced_with_opposite_gate() {
        let mut registry = cullen_gold_registry();
        let atom = "atom:NSW:CLA:s5B1c:reasonable-person-would-take-proposed-precautions";
        let mut conflicting = registry.get(CULLEN_CONTEXT, atom).unwrap().clone();
        conflicting.gate = AtomicGate::FitsThisAtom;
        assert_eq!(
            registry.register(conflicting),
            Err(RegistryError::ConflictingCanonicalEntry)
        );
    }

    #[test]
    fn negative_is_failure_of_exact_atom_not_an_opposite_proposition() {
        let registry = cullen_gold_registry();
        let breach_atom = registry
            .get(
                CULLEN_CONTEXT,
                "atom:NSW:CLA:s5B1c:reasonable-person-would-take-proposed-precautions",
            )
            .unwrap();
        let s43a_atom = registry
            .get(
                CULLEN_CONTEXT,
                "atom:NSW:CLA:s43A:liability-based-on-special-statutory-power",
            )
            .unwrap();
        assert_eq!(breach_atom.gate, AtomicGate::FailsThisAtom);
        assert_eq!(s43a_atom.gate, AtomicGate::FailsThisAtom);
        assert_ne!(breach_atom.atom_id, s43a_atom.atom_id);
    }

    #[test]
    fn primary_source_support_does_not_promote_semantic_or_legal_status() {
        let registry = cullen_gold_registry();
        for entry in registry.entries() {
            assert_eq!(
                entry.definition_lineage.stage,
                ProvenanceStage::RepositoryReconstruction
            );
            assert_eq!(
                entry.proposition_status.proposition_status,
                PropositionStatus::Represented
            );
            assert_eq!(entry.proposition_status.truth_status, TruthStatus::Unresolved);
            assert_eq!(entry.legal_status.authority, AuthorityKind::Unresolved);
            assert_eq!(entry.legal_status.applicability, ApplicabilityStatus::Unresolved);
            assert_eq!(entry.legal_status.violation, ViolationStatus::Unresolved);
            assert_eq!(entry.legal_status.liability, LiabilityStatus::Unresolved);
        }
    }

    #[test]
    fn repository_gate_is_not_attributed_back_to_primary_source() {
        let registry = cullen_gold_registry();
        for entry in registry.entries() {
            assert_eq!(
                entry.evaluation_lineage.stage,
                ProvenanceStage::CrossSourceInference
            );
            assert!(entry.evaluation_lineage.supporting_source.is_none());
        }
    }

    #[test]
    fn promotion_reuses_core_receipt_and_exact_admitted_source_span() {
        let registry = cullen_gold_registry();
        let atom_id = "atom:NSW:vicarious-liability:family-recognised";
        let span = TextSpan::new(1, 100, 140).unwrap();
        let address = FibreAddress {
            sentence_id: 100,
            local_ordinal: 1,
        };
        let admitted = AdmittedNormativeDelta {
            kind: ExpandedCandidateKind::ReferenceRelation,
            source_span: span,
            address,
            head: StableHeadRelation::Root,
            resolved_scope: ResolvedScope::ContextResolved,
            authority: ResolutionAuthority::HumanReview,
            policy_reference: "policy:cullen-atomic-admission".into(),
            resolver_reference: "resolver:cullen-atomic-review".into(),
        };
        let source_weld = admission::weld_admitted_candidate_to_source(
            atom_id,
            admission::SourceSpanAnchor {
                source_id: CULLEN_SOURCE_ID.into(),
                source_revision: "HCA-2026-19-official-pdf".into(),
                exact_locator: "Edelman J [100]".into(),
                address,
                span,
            },
            admitted,
            "weld:Cullen:100:vicarious-family",
        )
        .unwrap();
        let promoted = promote_atomic_entry(
            &registry,
            AtomicPromotionWeld {
                case_context: CULLEN_CONTEXT.into(),
                from_stage: ProvenanceStage::CrossSourceInference,
                source_weld,
                receipt: CorePromotionReceipt {
                    status: PromotionStatus::Promoted,
                    source_span: span,
                    policy_reference: "policy:atomic-promotion".into(),
                    reviewer_or_resolver_reference: "reviewer:fixture".into(),
                },
            },
        )
        .unwrap();
        assert_eq!(promoted.canonical_entry.atom_id, atom_id);
    }

    #[test]
    fn unresolved_atom_cannot_carry_invented_outcome_evidence() {
        let atom_id = "atom:test:unresolved";
        let entry = AtomicRegistryEntry {
            case_context: "case:test:r1".into(),
            atom_id: atom_id.into(),
            definition_lineage: ClaimLineage::reconstruction(
                atom_id,
                source("source:test", "s 1", "legislation", "legislative"),
                "definition",
            ),
            outcome_lineage: Some(ClaimLineage::reconstruction(
                "evidence:test",
                source("source:test", "p 2", "judgment", "ratio"),
                "invented outcome",
            )),
            evaluation_lineage: ClaimLineage::cross_source_inference(atom_id, "unresolved"),
            gate: AtomicGate::UnresolvedThisAtom,
            proposition_status: represented_source_status(atom_id),
            legal_status: unresolved_legal_status(),
        };
        assert_eq!(
            entry.validate(),
            Err(RegistryError::UnresolvedAtomHasOutcomeEvidence)
        );
    }
}
