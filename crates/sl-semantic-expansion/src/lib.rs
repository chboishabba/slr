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
    SymbolId, TextSpan, TokenObservation, project_sentence,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExpandedCandidateKind {
    Role(ExpandedSemanticRole),
    ScopedNegation,
    ConditionalRelation,
    ReferenceRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeState {
    SyntacticallyLocal,
    ScopeUnresolved,
    AttachmentUnresolved,
    ContextRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StableHeadRelation {
    Root,
    LocalOrdinal(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StableCandidateObservation {
    pub kind: ExpandedCandidateKind,
    pub span: TextSpan,
    pub address: FibreAddress,
    pub head: StableHeadRelation,
    pub scope: ScopeState,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StableResidualObservation {
    pub kind: ExpansionResidualKind,
    pub address: FibreAddress,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StableAlternativeFibreObservation {
    pub address: FibreAddress,
    pub alternatives: Vec<ExpandedSemanticRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedConsumerObservation {
    pub sentence_id: u64,
    pub candidates: Vec<StableCandidateObservation>,
    pub residuals: Vec<StableResidualObservation>,
    pub alternative_fibres: Vec<StableAlternativeFibreObservation>,
    pub projection_failures: Vec<ProjectionError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedParityReceipt {
    pub sentence_id: u64,
    pub direct: ExpandedConsumerObservation,
    pub reference: ExpandedConsumerObservation,
}

impl ExpandedParityReceipt {
    pub fn holds(&self) -> bool {
        self.direct == self.reference
    }
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

fn stable_head_relation(
    observation: &TokenObservation,
    committed_head: HeadCommit,
    token_to_local: &HashMap<u64, u32>,
) -> StableHeadRelation {
    match committed_head {
        HeadCommit::Root => StableHeadRelation::Root,
        HeadCommit::Dependency(token_id) => token_to_local
            .get(&token_id)
            .copied()
            .map(StableHeadRelation::LocalOrdinal)
            .unwrap_or_else(|| match observation.declared_head {
                HeadDeclaration::SelfHead => StableHeadRelation::Root,
                HeadDeclaration::LocalOrdinal(local) => StableHeadRelation::LocalOrdinal(local),
            }),
    }
}

fn observe_emission(
    tokens: &[TokenObservation],
    emission: &ExpandedSentenceEmission,
) -> ExpandedConsumerObservation {
    let token_to_local: HashMap<u64, u32> = tokens
        .iter()
        .map(|token| (token.token_id, token.local_ordinal))
        .collect();
    let token_by_local: HashMap<u32, &TokenObservation> = tokens
        .iter()
        .map(|token| (token.local_ordinal, token))
        .collect();

    let mut candidates: Vec<_> = emission
        .candidates
        .iter()
        .map(|candidate| {
            let source = token_by_local
                .get(&candidate.evidence.address.local_ordinal)
                .copied();
            let head = source
                .map(|observation| {
                    stable_head_relation(observation, candidate.committed_head, &token_to_local)
                })
                .unwrap_or(StableHeadRelation::Root);
            StableCandidateObservation {
                kind: candidate.kind,
                span: candidate.evidence.span,
                address: candidate.evidence.address,
                head,
                scope: candidate.scope,
                candidate_only: candidate.candidate_only,
            }
        })
        .collect();
    candidates.sort();

    let mut residuals: Vec<_> = emission
        .residuals
        .iter()
        .map(|residual| StableResidualObservation {
            kind: residual.kind,
            address: residual.address,
        })
        .collect();
    residuals.sort();

    let mut alternative_fibres: Vec<_> = emission
        .alternative_fibres
        .iter()
        .map(|fibre| StableAlternativeFibreObservation {
            address: fibre.address,
            alternatives: fibre.alternatives.clone(),
        })
        .collect();
    alternative_fibres.sort();

    let mut projection_failures = emission.projection_failures.clone();
    projection_failures.sort_by_key(|failure| match failure {
        ProjectionError::MissingDependentHead { local_ordinal, head_ordinal } => {
            (0u8, *local_ordinal, *head_ordinal)
        }
        ProjectionError::DuplicateLocalOrdinal(local_ordinal) => (1u8, *local_ordinal, 0),
    });

    ExpandedConsumerObservation {
        sentence_id: emission.sentence_id,
        candidates,
        residuals,
        alternative_fibres,
        projection_failures,
    }
}

/// Reference implementation: project through the canonical row projection,
/// then emit richer candidate semantics from those committed rows.
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

/// Independent direct implementation: build the local ordinal map and committed
/// heads in one sentence-local pass rather than consuming `project_sentence`.
pub fn compile_expanded_direct(
    tokens: Vec<TokenObservation>,
    classify: impl Fn(SymbolId) -> ExpansionSignal,
) -> ExpandedSentenceEmission {
    let sentence_id = tokens.first().map(|token| token.sentence_id).unwrap_or(0);
    let max_local = tokens.iter().map(|token| token.local_ordinal).max().unwrap_or(0) as usize;
    let mut by_local = vec![None; max_local.saturating_add(1)];
    let mut failures = Vec::new();
    for token in &tokens {
        if by_local[token.local_ordinal as usize]
            .replace(token.token_id)
            .is_some()
        {
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
                match by_local.get(head_local as usize).and_then(|entry| *entry) {
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
        let Some(committed_head) = committed_head else {
            continue;
        };
        let symbol = match observation.dependency {
            Annotation::Present(symbol) => symbol,
            Annotation::Unavailable(_) => continue,
        };
        emit_signal(&mut out, observation, committed_head, classify(symbol));
    }
    out
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

pub fn check_expanded_parity(
    tokens: Vec<TokenObservation>,
    classify: impl Copy + Fn(SymbolId) -> ExpansionSignal,
) -> ExpandedParityReceipt {
    let sentence_id = tokens.first().map(|token| token.sentence_id).unwrap_or(0);
    let reference = compile_expanded_candidates(tokens.clone(), classify);
    let direct = compile_expanded_direct(tokens.clone(), classify);
    ExpandedParityReceipt {
        sentence_id,
        direct: observe_emission(&tokens, &direct),
        reference: observe_emission(&tokens, &reference),
    }
}

pub fn observe_direct_expanded(
    tokens: Vec<TokenObservation>,
    classify: impl Fn(SymbolId) -> ExpansionSignal,
) -> ExpandedConsumerObservation {
    let original = tokens.clone();
    let direct = compile_expanded_direct(tokens, classify);
    observe_emission(&original, &direct)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::{Capability, TextSpan};

    fn token_with_id(
        token_id: u64,
        local: u32,
        head: HeadDeclaration,
        dep: Annotation,
    ) -> TokenObservation {
        TokenObservation {
            token_id,
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

    fn token(local: u32, head: HeadDeclaration, dep: Annotation) -> TokenObservation {
        token_with_id(100 + local as u64, local, head, dep)
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
    fn direct_and_reference_match_on_stable_semantic_observation() {
        let receipt = check_expanded_parity(
            vec![
                token(0, HeadDeclaration::LocalOrdinal(1), Annotation::Present(10)),
                token(1, HeadDeclaration::SelfHead, Annotation::Present(20)),
            ],
            |symbol| match symbol {
                10 => ExpansionSignal::Negation,
                20 => ExpansionSignal::ConditionalMarker,
                _ => ExpansionSignal::Unsupported,
            },
        );
        assert!(receipt.holds());
    }

    #[test]
    fn transient_token_ids_are_not_semantic_parity_authority() {
        let first = vec![
            token_with_id(100, 0, HeadDeclaration::LocalOrdinal(1), Annotation::Present(10)),
            token_with_id(101, 1, HeadDeclaration::SelfHead, Annotation::Present(20)),
        ];
        let second = vec![
            token_with_id(900, 0, HeadDeclaration::LocalOrdinal(1), Annotation::Present(10)),
            token_with_id(901, 1, HeadDeclaration::SelfHead, Annotation::Present(20)),
        ];
        let classify = |symbol| match symbol {
            10 => ExpansionSignal::Negation,
            20 => ExpansionSignal::ConditionalMarker,
            _ => ExpansionSignal::Unsupported,
        };
        assert_eq!(
            observe_direct_expanded(first, classify),
            observe_direct_expanded(second, classify),
        );
    }

    #[test]
    fn source_span_change_is_visible_to_semantic_parity() {
        let first = vec![token_with_id(
            100,
            0,
            HeadDeclaration::SelfHead,
            Annotation::Present(10),
        )];
        let mut second = first.clone();
        second[0].span = TextSpan::new(1, 10, 11).unwrap();
        assert_ne!(
            observe_direct_expanded(first, |_| ExpansionSignal::Negation),
            observe_direct_expanded(second, |_| ExpansionSignal::Negation),
        );
    }
}
