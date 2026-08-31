//! Experimental richer semantic candidate emission.
//!
//! This crate is deliberately outside the certified direct production path.
//! It consumes one sentence-local observation fibre, emits candidate semantic
//! structure plus explicit residuals/alternatives, and has no publication API.

use sensiblaw_core::{
    Annotation, FibreAddress, HeadCommit, ProjectionError, StableSourceEvidence, SymbolId,
    TokenObservation, project_sentence,
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

/// Compile one sentence-local fibre into richer semantic candidates.
///
/// Dependency labels are classifier input only. This function never publishes,
/// never promotes a candidate to a world fact, and retains ambiguity/residuals
/// instead of silently selecting scope or attachment.
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
        match classify(symbol) {
            ExpansionSignal::NominalSubject => {
                push_candidate(
                    &mut out,
                    observation,
                    row.committed_head,
                    ExpandedCandidateKind::Role(ExpandedSemanticRole::Actor),
                    ScopeState::SyntacticallyLocal,
                );
            }
            ExpansionSignal::DirectObject => {
                push_candidate(
                    &mut out,
                    observation,
                    row.committed_head,
                    ExpandedCandidateKind::Role(ExpandedSemanticRole::Object),
                    ScopeState::SyntacticallyLocal,
                );
            }
            ExpansionSignal::Negation => {
                push_candidate(
                    &mut out,
                    observation,
                    row.committed_head,
                    ExpandedCandidateKind::ScopedNegation,
                    ScopeState::ScopeUnresolved,
                );
                push_residual(&mut out, observation, ExpansionResidualKind::NegationScopeUnresolved);
            }
            ExpansionSignal::ModalAuxiliary => {
                push_candidate(
                    &mut out,
                    observation,
                    row.committed_head,
                    ExpandedCandidateKind::Role(ExpandedSemanticRole::Modality),
                    ScopeState::ScopeUnresolved,
                );
                push_residual(&mut out, observation, ExpansionResidualKind::ModalityScopeUnresolved);
            }
            ExpansionSignal::TemporalModifier => {
                push_candidate(
                    &mut out,
                    observation,
                    row.committed_head,
                    ExpandedCandidateKind::Role(ExpandedSemanticRole::Temporal),
                    ScopeState::ContextRequired,
                );
                push_residual(&mut out, observation, ExpansionResidualKind::TemporalAnchorUnresolved);
            }
            ExpansionSignal::ConditionalMarker => {
                push_candidate(
                    &mut out,
                    observation,
                    row.committed_head,
                    ExpandedCandidateKind::ConditionalRelation,
                    ScopeState::ScopeUnresolved,
                );
                push_residual(&mut out, observation, ExpansionResidualKind::ConditionalScopeUnresolved);
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
                push_residual(&mut out, observation, ExpansionResidualKind::ClauseInterpretationAmbiguous);
            }
            ExpansionSignal::ReferenceAttachment => {
                push_candidate(
                    &mut out,
                    observation,
                    row.committed_head,
                    ExpandedCandidateKind::ReferenceRelation,
                    ScopeState::AttachmentUnresolved,
                );
                push_residual(&mut out, observation, ExpansionResidualKind::ReferenceAttachmentUnresolved);
            }
            ExpansionSignal::QualifierAttachment => {
                push_candidate(
                    &mut out,
                    observation,
                    row.committed_head,
                    ExpandedCandidateKind::Role(ExpandedSemanticRole::Qualifier),
                    ScopeState::AttachmentUnresolved,
                );
                push_residual(&mut out, observation, ExpansionResidualKind::QualifierAttachmentUnresolved);
            }
            ExpansionSignal::Unsupported => {
                push_residual(&mut out, observation, ExpansionResidualKind::UnsupportedDependency);
            }
        }
    }

    out
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
}
