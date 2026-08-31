//! Opt-in direct/reference certification for SensibLaw.
//! This crate is not part of the mandatory production compiler path.

use sensiblaw_core::{
    compile_packed_sentence, project_sentence, Annotation, DependencyShape, PackedSentence,
    PnfRole, SentenceId, SymbolId, TokenObservation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConsumerObservation {
    pub roles: [u32; 5],
    pub projection_failure_count: u32,
    pub residual_count: u32,
}

fn role_index(shape: DependencyShape) -> Option<usize> {
    match shape {
        DependencyShape::NominalSubject | DependencyShape::PassiveSubject => Some(0),
        DependencyShape::DirectObject => Some(1),
        DependencyShape::Negation => Some(2),
        DependencyShape::ModalAuxiliary => Some(3),
        DependencyShape::TemporalModifier => Some(4),
        _ => None,
    }
}

fn direct_observation(
    tokens: Vec<TokenObservation>,
    dependency_shape: impl Fn(SymbolId) -> DependencyShape,
) -> ConsumerObservation {
    let compiled = compile_packed_sentence(PackedSentence::from_observations(tokens), dependency_shape);
    ConsumerObservation {
        roles: [
            compiled.outward.boundary_delta.get(PnfRole::Actor),
            compiled.outward.boundary_delta.get(PnfRole::Patient),
            compiled.outward.boundary_delta.get(PnfRole::Negation),
            compiled.outward.boundary_delta.get(PnfRole::Modality),
            compiled.outward.boundary_delta.get(PnfRole::Temporal),
        ],
        projection_failure_count: compiled.projection_failures.len() as u32,
        residual_count: compiled.interior.residuals.len() as u32,
    }
}

/// Reference observation is intentionally built from the row-oriented reference
/// projection, not by delegating to the direct packed compiler.
pub fn reference_observation(
    tokens: Vec<TokenObservation>,
    dependency_shape: impl Fn(SymbolId) -> DependencyShape,
) -> ConsumerObservation {
    let receipt = project_sentence(tokens);
    let failure_count = receipt.failures.len() as u32;
    let mut roles = [0u32; 5];
    let mut residual_count = failure_count;
    for row in receipt.rows {
        let symbol = match row.observation.dependency {
            Annotation::Present(symbol) => symbol,
            Annotation::Unavailable(_) => continue,
        };
        if let Some(index) = role_index(dependency_shape(symbol)) {
            roles[index] = roles[index].saturating_add(1);
        } else {
            residual_count = residual_count.saturating_add(1);
        }
    }
    ConsumerObservation { roles, projection_failure_count: failure_count, residual_count }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParityReceipt {
    pub sentence_id: SentenceId,
    pub direct: ConsumerObservation,
    pub reference: ConsumerObservation,
}

impl ParityReceipt {
    pub fn holds(&self) -> bool { self.direct == self.reference }
}

pub fn check_direct_reference_parity(
    tokens: Vec<TokenObservation>,
    dependency_shape: impl Copy + Fn(SymbolId) -> DependencyShape,
) -> ParityReceipt {
    let sentence_id = tokens.first().map(|token| token.sentence_id).unwrap_or(0);
    let reference = reference_observation(tokens.clone(), dependency_shape);
    let direct = direct_observation(tokens, dependency_shape);
    ParityReceipt { sentence_id, direct, reference }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::{Capability, HeadDeclaration, TextSpan, TokenObservation};

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
    fn direct_reference_consumer_parity_holds() {
        let tokens = vec![
            token(0, HeadDeclaration::LocalOrdinal(1), Annotation::Present(10)),
            token(1, HeadDeclaration::SelfHead, Annotation::Present(11)),
        ];
        let receipt = check_direct_reference_parity(tokens, |id| {
            if id == 10 { DependencyShape::NominalSubject } else { DependencyShape::Unresolved }
        });
        assert!(receipt.holds());
    }

    #[test]
    fn missing_head_parity_preserves_failure() {
        let tokens = vec![token(0, HeadDeclaration::LocalOrdinal(99), Annotation::Present(10))];
        let receipt = check_direct_reference_parity(tokens, |_| DependencyShape::NominalSubject);
        assert!(receipt.holds());
        assert_eq!(receipt.direct.projection_failure_count, 1);
        assert_eq!(receipt.direct.residual_count, 1);
    }
}
