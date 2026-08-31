//! Experimental richer semantic candidate emission.
//!
//! This crate is deliberately outside the certified direct production path.
//! It consumes one sentence-local observation fibre, emits candidate semantic
//! structure plus explicit residuals/alternatives, and has no publication API.
//!
//! Two projection implementations are provided for bounded certification:
//! - `compile_expanded_candidates`: row/reference projection through `project_sentence`;
//! - `compile_expanded_direct`: independent local-map/direct projection.
//!
//! Parity is evaluated only after both are normalized to a stable consumer-visible
//! observation that excludes transient token IDs.

use std::collections::HashMap;

use sensiblaw_core::{
    Annotation, FibreAddress, HeadCommit, HeadDeclaration, ProjectionError, StableSourceEvidence,
    SymbolId, TextSpan, TokenId, TokenObservation, project_sentence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandedSemanticRole {
    Actor,
    Action,
    Object,
    Modality,
    Condition,
    Exception,
    Qualifier,
    Jurisdiction,
    Speaker,
    Evidence,
    Temporal,
    Provenance,
    Reference,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionSignal {
    NominalSubject,
    DirectObject,
    Negation,
    ModalAuxiliary,
    TemporalModifier,
    ConditionalMarker,
    ClausalModifier,
    ReferenceAttachment,
    QualifierAttachment,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandedCandidateKind {
    Role(ExpandedSemanticRole),
    ScopedNegation,
    ConditionalRelation,
    ReferenceRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeState {
    SyntacticallyLocal,
    ScopeUnresolved,
    AttachmentUnresolved,
    ContextRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionResidualKind {
    NegationScopeUnresolved,
    ModalityScopeUnresolved,
    TemporalAnchorUnresolved,
    ConditionalScopeUnresolved,
    ClauseInterpretationAmbiguous,
    ReferenceAttachmentUnresolved,
    QualifierAttachmentUnresolved,
    UnsupportedDependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpandedCandidateDelta {
    pub kind: ExpandedCandidateKind,
    pub dependent: u64,
    pub committed_head: HeadCommit,
    pub evidence: StableSourceEvidence,
    pub scope: ScopeState,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionResidual {
    pub kind: ExpansionResidualKind,
    pub address: FibreAddress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateAlternativeFibre {
    pub address: FibreAddress,
    pub alternatives: Vec<ExpandedSemanticRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedSentenceEmission {
    pub sentence_id: u64,
    pub candidates: Vec<ExpandedCandidateDelta>,
    pub residuals: Vec<ExpansionResidual>,
    pub alternative_fibres: Vec<CandidateAlternativeFibre>,
    pub projection_failures: Vec<ProjectionError>,
}

fn evidence(observation: &TokenObservation) -> StableSourceEvidence {
    StableSourceEvidence {
        span: observation.span,
        address: FibreAddress {
            sentence_id: observation.sentence_id,
            local_ordinal: observation.local_ordinal,
        },
    }
}

fn push_candidate(
    out: &mut ExpandedSentenceEmission,
    observation: &TokenObservation,
    committed_head: HeadCommit,
    kind: ExpandedCandidateKind,
    scope: ScopeState,
) {
    out.candidates.push(ExpandedCandidateDelta {
        kind,
        dependent: observation.token_id,
        committed_head,
        evidence: evidence(observation),
        scope,
        candidate_only: true,
    });
}

fn push_residual(
    out: &mut ExpandedSentenceEmission,
    observation: &TokenObservation,
    kind: ExpansionResidualKind,
) {
    out.residuals.push(ExpansionResidual {
        kind,
        address: FibreAddress {
            sentence_id: observation.sentence_id,
            local_ordinal: observation.local_ordinal,
        },
    });
}

fn emit_signal(
    out: &mut ExpandedSentenceEmission,
    observation: &TokenObservation,
    committed_head: HeadCommit,
    signal: ExpansionSignal,
) {
    match signal {
        ExpansionSignal::NominalSubject => {
            push_candidate(
                out,
                observation,
                committed_head,
                ExpandedCandidateKind::Role(ExpandedSemanticRole::Actor),
                ScopeState::SyntacticallyLocal,
            );
        }
        ExpansionSignal::DirectObject => {
            push_candidate(
                out,
                observation,
                committed_head,
                ExpandedCandidateKind::Role(ExpandedSemanticRole::Object),
                ScopeState::SyntacticallyLocal,
            );
        }
        ExpansionSignal::Negation => {
            push_candidate(
                out,
                observation,
                committed_head,
                ExpandedCandidateKind::ScopedNegation,
                ScopeState::ScopeUnresolved,
            );
            push_residual(out, observation, ExpansionResidualKind::NegationScopeUnresolved);
        }
        ExpansionSignal::ModalAuxiliary => {
            push_candidate(
                out,
                observation,
                committed_head,
                ExpandedCandidateKind::Role(ExpandedSemanticRole::Modality),
                ScopeState::ScopeUnresolved,
            );
            push_residual(out, observation, ExpansionResidualKind::ModalityScopeUnresolved);
        }
        ExpansionSignal::TemporalModifier => {
            push_candidate(
                out,
                observation,
                committed_head,
                ExpandedCandidateKind::Role(ExpandedSemanticRole::Temporal),
                ScopeState::ContextRequired,
            );
            push_residual(out, observation, ExpansionResidualKind::TemporalAnchorUnresolved);
        }
        ExpansionSignal::ConditionalMarker => {
            push_candidate(
                out,
                observation,
                committed_head,
                ExpandedCandidateKind::ConditionalRelation,
                ScopeState::ScopeUnresolved,
            );
            push_residual(out, observation, ExpansionResidualKind::ConditionalScopeUnresolved);
        }
        ExpansionSignal::ClausalModifier => {
            let address = FibreAddress {
                sentence_id: observation.sentence_id,
                local_ordinal: observation.local_ordinal,
            };
            out.alternative_fibres.push(CandidateAlternativeFibre {
                address,
                alternatives: vec![
                    ExpandedSemanticRole::Condition,
                    ExpandedSemanticRole::Temporal,
                    ExpandedSemanticRole::Qualifier,
                ],
            });
            push_residual(out, observation, ExpansionResidualKind::ClauseInterpretationAmbiguous);
        }
        ExpansionSignal::ReferenceAttachment => {
            push_candidate(
                out,
                observation,
                committed_head,
                ExpandedCandidateKind::ReferenceRelation,
                ScopeState::AttachmentUnresolved,
            );
            push_residual(out, observation, ExpansionResidualKind::ReferenceAttachmentUnresolved);
        }
        ExpansionSignal::QualifierAttachment => {
            push_candidate(
                out,
                observation,
                committed_head,
                ExpandedCandidateKind::Role(ExpandedSemanticRole::Qualifier),
                ScopeState::AttachmentUnresolved,
            );
            push_residual(out, observation, ExpansionResidualKind::QualifierAttachmentUnresolved);
        }
        ExpansionSignal::Unsupported => {
            push_residual(out, observation, ExpansionResidualKind::UnsupportedDependency);
        }
    }
}

/// Reference compiler: project through the row-oriented reference projection.
pub fn compile_expanded_candidates(
    tokens: Vec<TokenObservation>,
    classify: impl Fn(SymbolId) -> ExpansionSignal,
) -> ExpandedSentenceEmission {
    let sentence_id = tokens.first().map(|token| token.sentence_id).unwrap_or(0);
    let receipt = project_sentence(tokens);
    let mut out = ExpandedSentenceEmission {
        sentence_id,
        candidates: Vec::new(),
        residuals: Vec::new(),
        alternative_fibres: Vec::new(),
        projection_failures: receipt.failures,
    };

    for row in receipt.rows {
        let observation = &row.observation;
        let symbol = match observation.dependency {
            Annotation::Present(symbol) => symbol,
            Annotation::Unavailable(_) => continue,
        };
        emit_signal(&mut out, observation, row.committed_head, classify(symbol));
    }

    out
}

/// Direct compiler: independently resolves sentence-local heads from a packed local
/// ordinal map without calling `project_sentence`.
///
/// This is the candidate semantic expansion path whose representation parity is
/// checked against `compile_expanded_candidates`.
pub fn compile_expanded_direct(
    tokens: Vec<TokenObservation>,
    classify: impl Fn(SymbolId) -> ExpansionSignal,
) -> ExpandedSentenceEmission {
    let sentence_id = tokens.first().map(|token| token.sentence_id).unwrap_or(0);
    let max_local = tokens.iter().map(|token| token.local_ordinal).max().unwrap_or(0) as usize;
    let mut by_local = vec![None; max_local.saturating_add(1)];
    let mut failures = Vec::new();

    for token in &tokens {
        let slot = &mut by_local[token.local_ordinal as usize];
        if slot.replace(token.token_id).is_some() {
            failures.push(ProjectionError::DuplicateLocalOrdinal(token.local_ordinal));
        }
    }

    let mut out = ExpandedSentenceEmission {
        sentence_id,
        candidates: Vec::new(),
        residuals: Vec::new(),
        alternative_fibres: Vec::new(),
        projection_failures: failures,
    };

    for observation in &tokens {
        let committed_head = match observation.declared_head {
            HeadDeclaration::SelfHead => Some(HeadCommit::Root),
            HeadDeclaration::LocalOrdinal(head_local) => {
                match by_local.get(head_local as usize).and_then(|token| *token) {
                    Some(head) => Some(HeadCommit::Dependency(head)),
                    None => {
                        out.projection_failures.push(ProjectionError::MissingDependentHead {
                            local_ordinal: observation.local_ordinal,
                            head_ordinal: head_local,
                        });
                        None
                    }
                }
            }
        };
        let Some(committed_head) = committed_head else { continue };
        let symbol = match observation.dependency {
            Annotation::Present(symbol) => symbol,
            Annotation::Unavailable(_) => continue,
        };
        emit_signal(&mut out, observation, committed_head, classify(symbol));
    }

    out
}

// ---- stable expanded parity observation ------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StableHeadRelation {
    Root,
    LocalOrdinal(u32),
    UnknownExternal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StableExpandedCandidate {
    pub kind: ExpandedCandidateKind,
    pub evidence: StableSourceEvidence,
    pub head: StableHeadRelation,
    pub scope: ScopeState,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedConsumerObservation {
    pub sentence_id: u64,
    pub candidates: Vec<StableExpandedCandidate>,
    pub residuals: Vec<ExpansionResidual>,
    pub alternative_fibres: Vec<CandidateAlternativeFibre>,
    pub projection_failures: Vec<ProjectionError>,
}

fn stable_head_relation(
    head: HeadCommit,
    token_locality: &HashMap<TokenId, u32>,
) -> StableHeadRelation {
    match head {
        HeadCommit::Root => StableHeadRelation::Root,
        HeadCommit::Dependency(token_id) => token_locality
            .get(&token_id)
            .copied()
            .map(StableHeadRelation::LocalOrdinal)
            .unwrap_or(StableHeadRelation::UnknownExternal),
    }
}

pub fn expanded_consumer_observation(
    tokens: &[TokenObservation],
    emission: &ExpandedSentenceEmission,
) -> ExpandedConsumerObservation {
    let token_locality: HashMap<TokenId, u32> = tokens
        .iter()
        .map(|token| (token.token_id, token.local_ordinal))
        .collect();
    let candidates = emission
        .candidates
        .iter()
        .map(|candidate| StableExpandedCandidate {
            kind: candidate.kind,
            evidence: candidate.evidence,
            head: stable_head_relation(candidate.committed_head, &token_locality),
            scope: candidate.scope,
            candidate_only: candidate.candidate_only,
        })
        .collect();
    ExpandedConsumerObservation {
        sentence_id: emission.sentence_id,
        candidates,
        residuals: emission.residuals.clone(),
        alternative_fibres: emission.alternative_fibres.clone(),
        projection_failures: emission.projection_failures.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedParityReceipt {
    pub sentence_id: u64,
    pub direct: ExpandedConsumerObservation,
    pub reference: ExpandedConsumerObservation,
}

impl ExpandedParityReceipt {
    pub fn holds(&self) -> bool { self.direct == self.reference }
}

pub fn check_expanded_parity(
    tokens: Vec<TokenObservation>,
    classify: impl Copy + Fn(SymbolId) -> ExpansionSignal,
) -> ExpandedParityReceipt {
    let sentence_id = tokens.first().map(|token| token.sentence_id).unwrap_or(0);
    let direct_emission = compile_expanded_direct(tokens.clone(), classify);
    let reference_emission = compile_expanded_candidates(tokens.clone(), classify);
    ExpandedParityReceipt {
        sentence_id,
        direct: expanded_consumer_observation(&tokens, &direct_emission),
        reference: expanded_consumer_observation(&tokens, &reference_emission),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::{Capability, HeadDeclaration, TextSpan};

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
    fn scoped_negation_remains_candidate_and_residual() {
        let emission = compile_expanded_candidates(
            vec![
                token(0, HeadDeclaration::LocalOrdinal(1), Annotation::Present(10)),
                token(1, HeadDeclaration::SelfHead, Annotation::Unavailable(Capability::Dependency)),
            ],
            |symbol| if symbol == 10 { ExpansionSignal::Negation } else { ExpansionSignal::Unsupported },
        );
        assert_eq!(emission.candidates.len(), 1);
        assert!(emission.candidates[0].candidate_only);
        assert_eq!(emission.candidates[0].kind, ExpandedCandidateKind::ScopedNegation);
        assert_eq!(emission.candidates[0].scope, ScopeState::ScopeUnresolved);
        assert_eq!(emission.residuals[0].kind, ExpansionResidualKind::NegationScopeUnresolved);
    }

    #[test]
    fn ambiguous_clause_retains_alternative_fibre() {
        let emission = compile_expanded_candidates(
            vec![
                token(0, HeadDeclaration::LocalOrdinal(1), Annotation::Present(20)),
                token(1, HeadDeclaration::SelfHead, Annotation::Unavailable(Capability::Dependency)),
            ],
            |symbol| if symbol == 20 { ExpansionSignal::ClausalModifier } else { ExpansionSignal::Unsupported },
        );
        assert!(emission.candidates.is_empty());
        assert_eq!(emission.alternative_fibres.len(), 1);
        assert_eq!(
            emission.alternative_fibres[0].alternatives,
            vec![
                ExpandedSemanticRole::Condition,
                ExpandedSemanticRole::Temporal,
                ExpandedSemanticRole::Qualifier,
            ]
        );
        assert_eq!(emission.residuals[0].kind, ExpansionResidualKind::ClauseInterpretationAmbiguous);
    }

    #[test]
    fn reference_attachment_is_never_auto_admitted() {
        let emission = compile_expanded_candidates(
            vec![
                token(0, HeadDeclaration::LocalOrdinal(1), Annotation::Present(30)),
                token(1, HeadDeclaration::SelfHead, Annotation::Unavailable(Capability::Dependency)),
            ],
            |symbol| if symbol == 30 { ExpansionSignal::ReferenceAttachment } else { ExpansionSignal::Unsupported },
        );
        assert_eq!(emission.candidates.len(), 1);
        assert!(emission.candidates.iter().all(|candidate| candidate.candidate_only));
        assert_eq!(emission.residuals[0].kind, ExpansionResidualKind::ReferenceAttachmentUnresolved);
    }

    #[test]
    fn direct_reference_expanded_parity_holds_on_rich_fixture() {
        let tokens = vec![
            token(0, HeadDeclaration::LocalOrdinal(1), Annotation::Present(10)),
            token(1, HeadDeclaration::SelfHead, Annotation::Present(11)),
            token(2, HeadDeclaration::LocalOrdinal(1), Annotation::Present(12)),
            token(3, HeadDeclaration::LocalOrdinal(1), Annotation::Present(13)),
        ];
        let receipt = check_expanded_parity(tokens, |symbol| match symbol {
            10 => ExpansionSignal::NominalSubject,
            11 => ExpansionSignal::Unsupported,
            12 => ExpansionSignal::Negation,
            13 => ExpansionSignal::ClausalModifier,
            _ => ExpansionSignal::Unsupported,
        });
        assert!(receipt.holds());
        assert_eq!(receipt.direct.candidates.len(), 2);
        assert_eq!(receipt.direct.alternative_fibres.len(), 1);
    }

    #[test]
    fn direct_reference_expanded_parity_preserves_missing_head_failure() {
        let tokens = vec![token(
            0,
            HeadDeclaration::LocalOrdinal(99),
            Annotation::Present(10),
        )];
        let receipt = check_expanded_parity(tokens, |_| ExpansionSignal::NominalSubject);
        assert!(receipt.holds());
        assert_eq!(receipt.direct.projection_failures.len(), 1);
        assert!(receipt.direct.candidates.is_empty());
    }

    #[test]
    fn stable_observation_does_not_depend_on_token_id_values() {
        let mut left = vec![
            token(0, HeadDeclaration::LocalOrdinal(1), Annotation::Present(10)),
            token(1, HeadDeclaration::SelfHead, Annotation::Unavailable(Capability::Dependency)),
        ];
        let mut right = left.clone();
        left[0].token_id = 1000;
        left[1].token_id = 2000;
        right[0].token_id = 9000;
        right[1].token_id = 8000;
        let classify = |symbol| if symbol == 10 { ExpansionSignal::NominalSubject } else { ExpansionSignal::Unsupported };
        let left_emission = compile_expanded_direct(left.clone(), classify);
        let right_emission = compile_expanded_direct(right.clone(), classify);
        assert_eq!(
            expanded_consumer_observation(&left, &left_emission),
            expanded_consumer_observation(&right, &right_emission)
        );
    }

    #[test]
    fn source_span_remains_part_of_stable_observation() {
        let left = vec![token(0, HeadDeclaration::SelfHead, Annotation::Present(10))];
        let mut right = left.clone();
        right[0].span = TextSpan::new(1, 10, 11).unwrap();
        let classify = |_| ExpansionSignal::NominalSubject;
        let left_emission = compile_expanded_direct(left.clone(), classify);
        let right_emission = compile_expanded_direct(right.clone(), classify);
        assert_ne!(
            expanded_consumer_observation(&left, &left_emission),
            expanded_consumer_observation(&right, &right_emission)
        );
    }
}
