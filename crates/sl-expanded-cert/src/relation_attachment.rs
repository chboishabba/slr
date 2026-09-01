use sensiblaw_core::{
    Annotation, FibreAddress, HeadCommit, HeadDeclaration, ProjectionError, SymbolId, TextSpan,
    TokenObservation, project_sentence,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationAttachmentKind {
    Preposition,
    PrepositionalObject,
    PrepositionalComplement,
    PassiveAgentMarker,
    Dative,
    CaseMarker,
    Particle,
}

impl RelationAttachmentKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Preposition => "preposition",
            Self::PrepositionalObject => "prepositional_object",
            Self::PrepositionalComplement => "prepositional_complement",
            Self::PassiveAgentMarker => "passive_agent_marker",
            Self::Dative => "dative",
            Self::CaseMarker => "case_marker",
            Self::Particle => "particle",
        }
    }
}

pub const RELATION_ATTACHMENT_KINDS: [RelationAttachmentKind; 7] = [
    RelationAttachmentKind::Preposition,
    RelationAttachmentKind::PrepositionalObject,
    RelationAttachmentKind::PrepositionalComplement,
    RelationAttachmentKind::PassiveAgentMarker,
    RelationAttachmentKind::Dative,
    RelationAttachmentKind::CaseMarker,
    RelationAttachmentKind::Particle,
];

pub fn kind_from_dependency_label(label: &str) -> Option<RelationAttachmentKind> {
    match label {
        "prep" => Some(RelationAttachmentKind::Preposition),
        "pobj" => Some(RelationAttachmentKind::PrepositionalObject),
        "pcomp" => Some(RelationAttachmentKind::PrepositionalComplement),
        "agent" => Some(RelationAttachmentKind::PassiveAgentMarker),
        "dative" => Some(RelationAttachmentKind::Dative),
        "case" => Some(RelationAttachmentKind::CaseMarker),
        "prt" => Some(RelationAttachmentKind::Particle),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StableRelationHead {
    Root,
    LocalOrdinal(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelationAttachmentCandidate {
    pub kind: RelationAttachmentKind,
    pub span: TextSpan,
    pub address: FibreAddress,
    pub head: StableRelationHead,
    pub candidate_only: bool,
    pub context_resolution_required: bool,
}

fn stable_head(
    observation: &TokenObservation,
    committed: HeadCommit,
    token_to_local: &HashMap<u64, u32>,
) -> StableRelationHead {
    match committed {
        HeadCommit::Root => StableRelationHead::Root,
        HeadCommit::Dependency(token_id) => token_to_local
            .get(&token_id)
            .copied()
            .map(StableRelationHead::LocalOrdinal)
            .unwrap_or_else(|| match observation.declared_head {
                HeadDeclaration::SelfHead => StableRelationHead::Root,
                HeadDeclaration::LocalOrdinal(local) => StableRelationHead::LocalOrdinal(local),
            }),
    }
}

fn candidate(
    observation: &TokenObservation,
    committed: HeadCommit,
    kind: RelationAttachmentKind,
    token_to_local: &HashMap<u64, u32>,
) -> RelationAttachmentCandidate {
    RelationAttachmentCandidate {
        kind,
        span: observation.span,
        address: FibreAddress {
            sentence_id: observation.sentence_id,
            local_ordinal: observation.local_ordinal,
        },
        head: stable_head(observation, committed, token_to_local),
        candidate_only: true,
        context_resolution_required: true,
    }
}

pub fn reference_candidates(
    tokens: Vec<TokenObservation>,
    classify: impl Fn(SymbolId) -> Option<RelationAttachmentKind>,
) -> Vec<RelationAttachmentCandidate> {
    let token_to_local: HashMap<u64, u32> = tokens
        .iter()
        .map(|token| (token.token_id, token.local_ordinal))
        .collect();
    let receipt = project_sentence(tokens);
    let mut out = Vec::new();
    for row in receipt.rows {
        let observation = &row.observation;
        let Annotation::Present(symbol) = observation.dependency else {
            continue;
        };
        if let Some(kind) = classify(symbol) {
            out.push(candidate(
                observation,
                row.committed_head,
                kind,
                &token_to_local,
            ));
        }
    }
    out.sort();
    out
}

pub fn direct_candidates(
    tokens: &[TokenObservation],
    classify: impl Fn(SymbolId) -> Option<RelationAttachmentKind>,
) -> Vec<RelationAttachmentCandidate> {
    let token_to_local: HashMap<u64, u32> = tokens
        .iter()
        .map(|token| (token.token_id, token.local_ordinal))
        .collect();
    let local_to_token: HashMap<u32, u64> = tokens
        .iter()
        .map(|token| (token.local_ordinal, token.token_id))
        .collect();
    let mut out = Vec::new();
    for observation in tokens {
        let committed = match observation.declared_head {
            HeadDeclaration::SelfHead => HeadCommit::Root,
            HeadDeclaration::LocalOrdinal(local) => {
                let Some(token_id) = local_to_token.get(&local).copied() else {
                    continue;
                };
                HeadCommit::Dependency(token_id)
            }
        };
        let Annotation::Present(symbol) = observation.dependency else {
            continue;
        };
        if let Some(kind) = classify(symbol) {
            out.push(candidate(observation, committed, kind, &token_to_local));
        }
    }
    out.sort();
    out
}

pub fn projection_failure_count(tokens: Vec<TokenObservation>) -> usize {
    let receipt = project_sentence(tokens);
    receipt
        .failures
        .iter()
        .filter(|failure| matches!(failure, ProjectionError::MissingDependentHead { .. }))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::{Capability, HeadDeclaration};

    fn token(local: u32, head: HeadDeclaration, dep: u32) -> TokenObservation {
        TokenObservation {
            token_id: 100 + u64::from(local),
            sentence_id: 7,
            local_ordinal: local,
            span: TextSpan::new(1, local, local + 1).unwrap(),
            orth: local,
            lemma: Annotation::Unavailable(Capability::Lemma),
            pos: Annotation::Unavailable(Capability::Pos),
            tag: Annotation::Unavailable(Capability::Tag),
            dependency: Annotation::Present(dep),
            declared_head: head,
        }
    }

    #[test]
    fn relation_candidate_is_context_required_and_candidate_only() {
        let tokens = vec![
            token(0, HeadDeclaration::LocalOrdinal(1), 10),
            token(1, HeadDeclaration::SelfHead, 99),
        ];
        let out = direct_candidates(&tokens, |symbol| {
            (symbol == 10).then_some(RelationAttachmentKind::Preposition)
        });
        assert_eq!(out.len(), 1);
        assert!(out[0].candidate_only);
        assert!(out[0].context_resolution_required);
    }

    #[test]
    fn relation_direct_and_reference_observations_match() {
        let tokens = vec![
            token(0, HeadDeclaration::LocalOrdinal(1), 10),
            token(1, HeadDeclaration::SelfHead, 99),
        ];
        let classify = |symbol| (symbol == 10).then_some(RelationAttachmentKind::Preposition);
        assert_eq!(
            direct_candidates(&tokens, classify),
            reference_candidates(tokens, classify),
        );
    }

    #[test]
    fn parser_label_does_not_choose_legal_role() {
        assert_eq!(kind_from_dependency_label("prep"), Some(RelationAttachmentKind::Preposition));
        assert_eq!(kind_from_dependency_label("pobj"), Some(RelationAttachmentKind::PrepositionalObject));
        assert_eq!(kind_from_dependency_label("ROOT"), None);
    }
}
