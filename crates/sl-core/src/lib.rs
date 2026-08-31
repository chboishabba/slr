//! SensibLaw deterministic direct-delta core.
//! Parser output is observation evidence; it is never semantic authority.

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub type RevisionId = u64;
pub type SentenceId = u64;
pub type ParagraphId = u64;
pub type TokenId = u64;
pub type SymbolId = u32;
pub type GenerationId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextSpan {
    pub revision_id: RevisionId,
    pub start_char: u32,
    pub end_char: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanError { Inverted }

impl TextSpan {
    pub fn new(revision_id: RevisionId, start_char: u32, end_char: u32) -> Result<Self, SpanError> {
        if end_char < start_char { return Err(SpanError::Inverted); }
        Ok(Self { revision_id, start_char, end_char })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Tokenization, SentenceSegmentation, Lemma, Pos, Tag, Dependency, Morphology, NamedEntity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Annotation {
    Present(SymbolId),
    Unavailable(Capability),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentenceOwnership { FullyOwned, BoundaryCrossing, OutsideOwner }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentenceDisposition { Commit, QueueBoundaryRepair, IgnoreOutside }

pub fn dispose_sentence(owner: SentenceOwnership) -> SentenceDisposition {
    match owner {
        SentenceOwnership::FullyOwned => SentenceDisposition::Commit,
        SentenceOwnership::BoundaryCrossing => SentenceDisposition::QueueBoundaryRepair,
        SentenceOwnership::OutsideOwner => SentenceDisposition::IgnoreOutside,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadDeclaration { SelfHead, LocalOrdinal(u32) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadCommit { Root, Dependency(TokenId) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    MissingDependentHead { local_ordinal: u32, head_ordinal: u32 },
    DuplicateLocalOrdinal(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenObservation {
    pub token_id: TokenId,
    pub sentence_id: SentenceId,
    pub local_ordinal: u32,
    pub span: TextSpan,
    pub orth: SymbolId,
    pub lemma: Annotation,
    pub pos: Annotation,
    pub tag: Annotation,
    pub dependency: Annotation,
    pub declared_head: HeadDeclaration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericTokenRow {
    pub observation: TokenObservation,
    pub committed_head: HeadCommit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionReceipt {
    pub sentence_id: SentenceId,
    pub rows: Vec<NumericTokenRow>,
    pub failures: Vec<ProjectionError>,
}

/// Reference projection: a missing dependent head is a typed failure, never a root fallback.
pub fn project_sentence(tokens: Vec<TokenObservation>) -> ProjectionReceipt {
    let sentence_id = tokens.first().map(|t| t.sentence_id).unwrap_or(0);
    let max_local = tokens.iter().map(|t| t.local_ordinal).max().unwrap_or(0) as usize;
    let mut by_local = vec![None; max_local.saturating_add(1)];
    let mut failures = Vec::new();
    for token in &tokens {
        let slot = &mut by_local[token.local_ordinal as usize];
        if slot.replace(token.token_id).is_some() {
            failures.push(ProjectionError::DuplicateLocalOrdinal(token.local_ordinal));
        }
    }
    let mut rows = Vec::with_capacity(tokens.len());
    for observation in tokens {
        let committed_head = match observation.declared_head {
            HeadDeclaration::SelfHead => Some(HeadCommit::Root),
            HeadDeclaration::LocalOrdinal(head_local) => {
                match by_local.get(head_local as usize).and_then(|x| *x) {
                    Some(head) => Some(HeadCommit::Dependency(head)),
                    None => {
                        failures.push(ProjectionError::MissingDependentHead {
                            local_ordinal: observation.local_ordinal,
                            head_ordinal: head_local,
                        });
                        None
                    }
                }
            }
        };
        if let Some(committed_head) = committed_head {
            rows.push(NumericTokenRow { observation, committed_head });
        }
    }
    ProjectionReceipt { sentence_id, rows, failures }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyShape {
    NominalSubject, DirectObject, PassiveSubject, AdjectivalModifier, NominalModifier,
    Conjunction, Negation, ModalAuxiliary, Determiner, TemporalModifier, Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParserRef { pub sentence_id: SentenceId, pub local_ordinal: u32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependencyWitness {
    pub dependent: TokenId,
    pub head: TokenId,
    pub shape: DependencyShape,
    pub parser_reference: ParserRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticFragmentKind { Actor, Patient, Negation, Modality, Temporal, Unresolved }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateSemanticFragment {
    pub kind: SemanticFragmentKind,
    pub witness: DependencyWitness,
    pub candidate_only: bool,
}

pub fn candidate_from_dependency(witness: DependencyWitness) -> CandidateSemanticFragment {
    let kind = match witness.shape {
        DependencyShape::NominalSubject | DependencyShape::PassiveSubject => SemanticFragmentKind::Actor,
        DependencyShape::DirectObject => SemanticFragmentKind::Patient,
        DependencyShape::Negation => SemanticFragmentKind::Negation,
        DependencyShape::ModalAuxiliary => SemanticFragmentKind::Modality,
        DependencyShape::TemporalModifier => SemanticFragmentKind::Temporal,
        _ => SemanticFragmentKind::Unresolved,
    };
    CandidateSemanticFragment { kind, witness, candidate_only: true }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CandidateFibre { pub alternatives: Vec<CandidateSemanticFragment> }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotionStatus { Candidate, Promoted, Disputed, Abstained, Superseded }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionReceipt {
    pub status: PromotionStatus,
    pub source_span: TextSpan,
    pub policy_reference: String,
    pub reviewer_or_resolver_reference: String,
}

/// IDs are assigned solely by first occurrence; hash iteration never determines output order.
#[derive(Debug, Default)]
pub struct SymbolTable {
    ids: HashMap<String, SymbolId>,
    values: Vec<String>,
}

impl SymbolTable {
    pub fn intern(&mut self, value: &str) -> SymbolId {
        if let Some(id) = self.ids.get(value) { return *id; }
        let id = self.values.len() as SymbolId;
        self.values.push(value.to_owned());
        self.ids.insert(value.to_owned(), id);
        id
    }
    pub fn get(&self, id: SymbolId) -> Option<&str> { self.values.get(id as usize).map(String::as_str) }
    pub fn len(&self) -> usize { self.values.len() }
    pub fn is_empty(&self) -> bool { self.values.is_empty() }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ActiveTimer { pub active: Duration }

impl ActiveTimer {
    pub fn measure<T>(&mut self, f: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let out = f();
        self.active += start.elapsed();
        out
    }
}

// ---- packed direct sentence compiler ----------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FibreAddress { pub sentence_id: SentenceId, pub local_ordinal: u32 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedSentence {
    pub sentence_id: SentenceId,
    token_ids: Vec<TokenId>,
    local_ordinals: Vec<u32>,
    spans: Vec<TextSpan>,
    dependency: Vec<Annotation>,
    heads: Vec<HeadDeclaration>,
}

impl PackedSentence {
    pub fn from_observations(tokens: Vec<TokenObservation>) -> Self {
        let sentence_id = tokens.first().map(|t| t.sentence_id).unwrap_or(0);
        let mut out = Self {
            sentence_id,
            token_ids: Vec::with_capacity(tokens.len()),
            local_ordinals: Vec::with_capacity(tokens.len()),
            spans: Vec::with_capacity(tokens.len()),
            dependency: Vec::with_capacity(tokens.len()),
            heads: Vec::with_capacity(tokens.len()),
        };
        for token in tokens {
            out.token_ids.push(token.token_id);
            out.local_ordinals.push(token.local_ordinal);
            out.spans.push(token.span);
            out.dependency.push(token.dependency);
            out.heads.push(token.declared_head);
        }
        out
    }
    pub fn len(&self) -> usize { self.token_ids.len() }
    pub fn is_empty(&self) -> bool { self.token_ids.is_empty() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PnfRole { Actor, Patient, Negation, Modality, Temporal }

impl PnfRole {
    const COUNT: usize = 5;
    fn index(self) -> usize {
        match self { Self::Actor => 0, Self::Patient => 1, Self::Negation => 2, Self::Modality => 3, Self::Temporal => 4 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RoleCounts { counts: [u32; PnfRole::COUNT] }

impl RoleCounts {
    pub fn get(&self, role: PnfRole) -> u32 { self.counts[role.index()] }
    pub fn increment(&mut self, role: PnfRole) { self.counts[role.index()] = self.counts[role.index()].saturating_add(1); }
    pub fn add_assign(&mut self, other: &Self) {
        for (left, right) in self.counts.iter_mut().zip(other.counts.iter()) { *left = left.saturating_add(*right); }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StableSourceEvidence { pub span: TextSpan, pub address: FibreAddress }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormativeDelta {
    pub role: PnfRole,
    pub dependent: TokenId,
    pub head: TokenId,
    pub evidence: StableSourceEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualKind { MissingDependentHead, DuplicateLocalOrdinal, UnsupportedDependency }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticResidual { pub kind: ResidualKind, pub address: FibreAddress }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicalCounters {
    pub sentence_local_db_crossings: u64,
    pub production_parser_token_writes: u64,
    pub unchanged_relation_writes: u64,
    pub closed_child_interior_reads_by_parent: u64,
}

impl PhysicalCounters {
    pub fn direct_constitution_holds(&self) -> bool {
        self.sentence_local_db_crossings == 0
            && self.production_parser_token_writes == 0
            && self.unchanged_relation_writes == 0
            && self.closed_child_interior_reads_by_parent == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentenceInterior {
    pub sentence_id: SentenceId,
    pub deltas: Vec<NormativeDelta>,
    pub residuals: Vec<SemanticResidual>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentenceOutwardDelta {
    pub sentence_id: SentenceId,
    pub boundary_delta: RoleCounts,
    pub residual_delta: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectSentenceCompilation {
    pub interior: SentenceInterior,
    pub outward: SentenceOutwardDelta,
    pub projection_failures: Vec<ProjectionError>,
    pub physical: PhysicalCounters,
}

fn role_for(shape: DependencyShape) -> Option<PnfRole> {
    match shape {
        DependencyShape::NominalSubject | DependencyShape::PassiveSubject => Some(PnfRole::Actor),
        DependencyShape::DirectObject => Some(PnfRole::Patient),
        DependencyShape::Negation => Some(PnfRole::Negation),
        DependencyShape::ModalAuxiliary => Some(PnfRole::Modality),
        DependencyShape::TemporalModifier => Some(PnfRole::Temporal),
        _ => None,
    }
}

/// Mandatory direct path: packed in-memory solve, no DB API, no parser-token persistence.
pub fn compile_packed_sentence(
    packed: PackedSentence,
    dependency_shape: impl Fn(SymbolId) -> DependencyShape,
) -> DirectSentenceCompilation {
    let sentence_id = packed.sentence_id;
    let max_local = packed.local_ordinals.iter().copied().max().unwrap_or(0) as usize;
    let mut local_to_index = vec![None; max_local.saturating_add(1)];
    let mut failures = Vec::new();
    let mut residuals = Vec::new();

    for (index, local) in packed.local_ordinals.iter().copied().enumerate() {
        if local_to_index[local as usize].replace(index).is_some() {
            failures.push(ProjectionError::DuplicateLocalOrdinal(local));
            residuals.push(SemanticResidual {
                kind: ResidualKind::DuplicateLocalOrdinal,
                address: FibreAddress { sentence_id, local_ordinal: local },
            });
        }
    }

    let mut deltas = Vec::with_capacity(packed.len());
    let mut boundary_delta = RoleCounts::default();
    for index in 0..packed.len() {
        let local = packed.local_ordinals[index];
        let head = match packed.heads[index] {
            HeadDeclaration::SelfHead => Some(packed.token_ids[index]),
            HeadDeclaration::LocalOrdinal(head_local) => {
                match local_to_index.get(head_local as usize).and_then(|idx| *idx) {
                    Some(head_index) => Some(packed.token_ids[head_index]),
                    None => {
                        failures.push(ProjectionError::MissingDependentHead { local_ordinal: local, head_ordinal: head_local });
                        residuals.push(SemanticResidual {
                            kind: ResidualKind::MissingDependentHead,
                            address: FibreAddress { sentence_id, local_ordinal: local },
                        });
                        None
                    }
                }
            }
        };
        let Some(head) = head else { continue };
        let dep_symbol = match &packed.dependency[index] {
            Annotation::Present(symbol) => *symbol,
            Annotation::Unavailable(_) => continue,
        };
        let Some(role) = role_for(dependency_shape(dep_symbol)) else {
            residuals.push(SemanticResidual {
                kind: ResidualKind::UnsupportedDependency,
                address: FibreAddress { sentence_id, local_ordinal: local },
            });
            continue;
        };
        boundary_delta.increment(role);
        deltas.push(NormativeDelta {
            role,
            dependent: packed.token_ids[index],
            head,
            evidence: StableSourceEvidence {
                span: packed.spans[index],
                address: FibreAddress { sentence_id, local_ordinal: local },
            },
        });
    }

    let residual_count = residuals.len() as u32;
    DirectSentenceCompilation {
        interior: SentenceInterior { sentence_id, deltas, residuals },
        outward: SentenceOutwardDelta { sentence_id, boundary_delta, residual_delta: residual_count },
        projection_failures: failures,
        physical: PhysicalCounters::default(),
    }
}

// ---- natural child -> parent transport -------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SentenceBoundary { pub roles: RoleCounts }

pub fn apply_sentence_delta(state: &mut SentenceBoundary, delta: &NormativeDelta) { state.roles.increment(delta.role); }
pub fn restrict_sentence(state: &SentenceBoundary) -> RoleCounts { state.roles }
pub fn transport_delta(delta: &NormativeDelta) -> RoleCounts {
    let mut out = RoleCounts::default();
    out.increment(delta.role);
    out
}
pub fn apply_parent_delta(parent: &mut RoleCounts, delta: &RoleCounts) { parent.add_assign(delta); }

pub fn transport_commutes(state: SentenceBoundary, delta: NormativeDelta) -> bool {
    let mut child_after = state.clone();
    apply_sentence_delta(&mut child_after, &delta);
    let lhs = restrict_sentence(&child_after);
    let mut rhs = restrict_sentence(&state);
    apply_parent_delta(&mut rhs, &transport_delta(&delta));
    lhs == rhs
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagraphAccumulator {
    pub paragraph_id: ParagraphId,
    pub boundary: RoleCounts,
    pub residuals: u64,
    pub accepted_children: u64,
    pub physical: PhysicalCounters,
}

impl ParagraphAccumulator {
    pub fn new(paragraph_id: ParagraphId) -> Self {
        Self { paragraph_id, boundary: RoleCounts::default(), residuals: 0, accepted_children: 0, physical: PhysicalCounters::default() }
    }

    /// Outward-only API: the closed sentence interior is not available to the parent.
    pub fn absorb(&mut self, child: SentenceOutwardDelta) {
        self.boundary.add_assign(&child.boundary_delta);
        self.residuals = self.residuals.saturating_add(child.residual_delta as u64);
        self.accepted_children = self.accepted_children.saturating_add(1);
    }
}

// ---- append-only generation staging/publication -----------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertificationReceipt { pub policy_reference: u64, pub reviewer_reference: u64 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationState { Staged, Certified, Published, Rejected }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationRecord {
    pub generation_id: GenerationId,
    pub state: GenerationState,
    pub candidate_delta_count: u64,
    pub residual_count: u64,
    pub certification: Option<CertificationReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationError { UnknownGeneration, NotCertified, AlreadyTerminal }

#[derive(Debug, Default)]
pub struct GenerationPublisher {
    generations: Vec<GenerationRecord>,
    current: Option<GenerationId>,
    publication_transitions: u64,
}

impl GenerationPublisher {
    pub fn stage(&mut self, candidate_delta_count: u64, residual_count: u64) -> GenerationId {
        let generation_id = self.generations.len() as GenerationId;
        self.generations.push(GenerationRecord {
            generation_id,
            state: GenerationState::Staged,
            candidate_delta_count,
            residual_count,
            certification: None,
        });
        generation_id
    }

    pub fn certify(&mut self, id: GenerationId, receipt: CertificationReceipt) -> Result<(), PublicationError> {
        let Some(record) = self.generations.get_mut(id as usize) else { return Err(PublicationError::UnknownGeneration); };
        match record.state {
            GenerationState::Staged => {
                record.state = GenerationState::Certified;
                record.certification = Some(receipt);
                Ok(())
            }
            GenerationState::Certified => Ok(()),
            GenerationState::Published | GenerationState::Rejected => Err(PublicationError::AlreadyTerminal),
        }
    }

    pub fn reject(&mut self, id: GenerationId) -> Result<(), PublicationError> {
        let Some(record) = self.generations.get_mut(id as usize) else { return Err(PublicationError::UnknownGeneration); };
        match record.state {
            GenerationState::Staged | GenerationState::Certified => { record.state = GenerationState::Rejected; Ok(()) }
            GenerationState::Published | GenerationState::Rejected => Err(PublicationError::AlreadyTerminal),
        }
    }

    pub fn publish(&mut self, id: GenerationId) -> Result<(), PublicationError> {
        let Some(record) = self.generations.get_mut(id as usize) else { return Err(PublicationError::UnknownGeneration); };
        if record.state != GenerationState::Certified || record.certification.is_none() {
            return Err(match record.state {
                GenerationState::Published | GenerationState::Rejected => PublicationError::AlreadyTerminal,
                _ => PublicationError::NotCertified,
            });
        }
        record.state = GenerationState::Published;
        self.current = Some(id);
        self.publication_transitions = self.publication_transitions.saturating_add(1);
        Ok(())
    }

    pub fn current(&self) -> Option<GenerationId> { self.current }
    pub fn publication_transitions(&self) -> u64 { self.publication_transitions }
    pub fn record(&self, id: GenerationId) -> Option<&GenerationRecord> { self.generations.get(id as usize) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(local: u32, head: HeadDeclaration, dep: Annotation) -> TokenObservation {
        TokenObservation {
            token_id: 100 + local as u64,
            sentence_id: 7,
            local_ordinal: local,
            span: TextSpan::new(1, local, local + 1).unwrap(),
            orth: local,
            lemma: Annotation::Unavailable(Capability::Lemma),
            pos: Annotation::Unavailable(Capability::Pos),
            tag: Annotation::Unavailable(Capability::Tag),
            dependency: dep,
            declared_head: head,
        }
    }

    #[test]
    fn crossing_sentence_never_commits() {
        assert_eq!(dispose_sentence(SentenceOwnership::BoundaryCrossing), SentenceDisposition::QueueBoundaryRepair);
    }

    #[test]
    fn missing_head_is_failure_not_root() {
        let receipt = project_sentence(vec![token(0, HeadDeclaration::LocalOrdinal(99), Annotation::Unavailable(Capability::Dependency))]);
        assert!(receipt.rows.is_empty());
        assert!(matches!(receipt.failures[0], ProjectionError::MissingDependentHead { .. }));
    }

    #[test]
    fn candidate_never_auto_promotes() {
        let candidate = candidate_from_dependency(DependencyWitness {
            dependent: 1, head: 2, shape: DependencyShape::NominalSubject,
            parser_reference: ParserRef { sentence_id: 7, local_ordinal: 0 },
        });
        assert!(candidate.candidate_only);
    }

    #[test]
    fn direct_compile_has_zero_mandatory_bus_counters() {
        let packed = PackedSentence::from_observations(vec![
            token(0, HeadDeclaration::LocalOrdinal(1), Annotation::Present(10)),
            token(1, HeadDeclaration::SelfHead, Annotation::Present(11)),
        ]);
        let compiled = compile_packed_sentence(packed, |id| if id == 10 { DependencyShape::NominalSubject } else { DependencyShape::Unresolved });
        assert!(compiled.physical.direct_constitution_holds());
        assert_eq!(compiled.outward.boundary_delta.get(PnfRole::Actor), 1);
        assert_eq!(compiled.interior.deltas.len(), 1);
        assert_eq!(compiled.interior.residuals.len(), 1);
    }

    #[test]
    fn child_parent_transport_commutes() {
        let delta = NormativeDelta {
            role: PnfRole::Actor,
            dependent: 1,
            head: 2,
            evidence: StableSourceEvidence {
                span: TextSpan::new(1, 0, 1).unwrap(),
                address: FibreAddress { sentence_id: 1, local_ordinal: 0 },
            },
        };
        assert!(transport_commutes(SentenceBoundary::default(), delta));
    }

    #[test]
    fn paragraph_accepts_only_outward_delta() {
        let mut paragraph = ParagraphAccumulator::new(4);
        let mut roles = RoleCounts::default();
        roles.increment(PnfRole::Actor);
        paragraph.absorb(SentenceOutwardDelta { sentence_id: 1, boundary_delta: roles, residual_delta: 0 });
        assert_eq!(paragraph.accepted_children, 1);
        assert_eq!(paragraph.boundary.get(PnfRole::Actor), 1);
        assert_eq!(paragraph.physical.closed_child_interior_reads_by_parent, 0);
    }

    #[test]
    fn generation_publication_fails_closed() {
        let mut publisher = GenerationPublisher::default();
        let generation = publisher.stage(3, 1);
        assert_eq!(publisher.current(), None);
        assert_eq!(publisher.publish(generation), Err(PublicationError::NotCertified));
        publisher.certify(generation, CertificationReceipt { policy_reference: 4, reviewer_reference: 9 }).unwrap();
        publisher.publish(generation).unwrap();
        assert_eq!(publisher.current(), Some(generation));
        assert_eq!(publisher.publication_transitions(), 1);
        assert_eq!(publisher.publish(generation), Err(PublicationError::AlreadyTerminal));
    }
}
