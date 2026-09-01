//! Candidate-to-admitted semantic boundary for SensibLaw.
//!
//! Expanded parser semantics remain candidate evidence. Admission requires an
//! explicit non-parser receipt. Missing/bad receipts preserve candidates,
//! residuals and alternative fibres. This crate has no publication API.

use sensiblaw_core::{FibreAddress, TextSpan};
use sensiblaw_semantic_expansion::{
    ExpansionResidualKind, ExpandedCandidateKind, ExpandedConsumerObservation,
    StableAlternativeFibreObservation, StableCandidateObservation, StableHeadRelation,
    StableResidualObservation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionAuthority {
    DeterministicReviewedPolicy,
    HumanReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedScope {
    LocalSyntactic,
    ScopeResolved,
    AttachmentResolved,
    ContextResolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionReceipt {
    pub address: FibreAddress,
    pub source_span: TextSpan,
    pub kind: ExpandedCandidateKind,
    pub resolved_scope: ResolvedScope,
    pub authority: ResolutionAuthority,
    pub policy_reference: String,
    pub resolver_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedNormativeDelta {
    pub kind: ExpandedCandidateKind,
    pub source_span: TextSpan,
    pub address: FibreAddress,
    pub head: StableHeadRelation,
    pub resolved_scope: ResolvedScope,
    pub authority: ResolutionAuthority,
    pub policy_reference: String,
    pub resolver_reference: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    CandidateWasNotMarkedCandidateOnly,
    AddressMismatch,
    SourceSpanMismatch,
    KindMismatch,
    ScopeResolutionMismatch,
    MissingPolicyReference,
    MissingResolverReference,
}

fn scope_matches(candidate: sensiblaw_semantic_expansion::ScopeState, resolved: ResolvedScope) -> bool {
    use sensiblaw_semantic_expansion::ScopeState;
    matches!(
        (candidate, resolved),
        (ScopeState::SyntacticallyLocal, ResolvedScope::LocalSyntactic)
            | (ScopeState::ScopeUnresolved, ResolvedScope::ScopeResolved)
            | (ScopeState::AttachmentUnresolved, ResolvedScope::AttachmentResolved)
            | (ScopeState::ContextRequired, ResolvedScope::ContextResolved)
    )
}

pub fn admit_candidate(
    candidate: &StableCandidateObservation,
    receipt: &AdmissionReceipt,
) -> Result<AdmittedNormativeDelta, AdmissionError> {
    if !candidate.candidate_only {
        return Err(AdmissionError::CandidateWasNotMarkedCandidateOnly);
    }
    if candidate.address != receipt.address {
        return Err(AdmissionError::AddressMismatch);
    }
    if candidate.span != receipt.source_span {
        return Err(AdmissionError::SourceSpanMismatch);
    }
    if candidate.kind != receipt.kind {
        return Err(AdmissionError::KindMismatch);
    }
    if !scope_matches(candidate.scope, receipt.resolved_scope) {
        return Err(AdmissionError::ScopeResolutionMismatch);
    }
    if receipt.policy_reference.trim().is_empty() {
        return Err(AdmissionError::MissingPolicyReference);
    }
    if receipt.resolver_reference.trim().is_empty() {
        return Err(AdmissionError::MissingResolverReference);
    }
    Ok(AdmittedNormativeDelta {
        kind: candidate.kind,
        source_span: candidate.span,
        address: candidate.address,
        head: candidate.head,
        resolved_scope: receipt.resolved_scope,
        authority: receipt.authority,
        policy_reference: receipt.policy_reference.clone(),
        resolver_reference: receipt.resolver_reference.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionFailure {
    pub candidate: StableCandidateObservation,
    pub error: AdmissionError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionOutcome {
    pub sentence_id: u64,
    pub admitted: Vec<AdmittedNormativeDelta>,
    pub retained_candidates: Vec<StableCandidateObservation>,
    pub retained_residuals: Vec<StableResidualObservation>,
    pub retained_alternative_fibres: Vec<StableAlternativeFibreObservation>,
    pub failures: Vec<AdmissionFailure>,
}

/// Sparse fail-closed admission. Rejection never deletes evidence.
pub fn admit_with_receipts(
    observation: &ExpandedConsumerObservation,
    receipts: &[AdmissionReceipt],
) -> AdmissionOutcome {
    let mut admitted = Vec::new();
    let mut retained_candidates = Vec::new();
    let mut failures = Vec::new();
    for candidate in &observation.candidates {
        let matching = receipts.iter().find(|receipt| {
            receipt.address == candidate.address
                && receipt.source_span == candidate.span
                && receipt.kind == candidate.kind
        });
        match matching {
            None => retained_candidates.push(candidate.clone()),
            Some(receipt) => match admit_candidate(candidate, receipt) {
                Ok(delta) => admitted.push(delta),
                Err(error) => {
                    retained_candidates.push(candidate.clone());
                    failures.push(AdmissionFailure { candidate: candidate.clone(), error });
                }
            },
        }
    }
    AdmissionOutcome {
        sentence_id: observation.sentence_id,
        admitted,
        retained_candidates,
        retained_residuals: observation.residuals.clone(),
        retained_alternative_fibres: observation.alternative_fibres.clone(),
        failures,
    }
}

pub const RESIDUAL_KINDS: [ExpansionResidualKind; 8] = [
    ExpansionResidualKind::NegationScopeUnresolved,
    ExpansionResidualKind::ModalityScopeUnresolved,
    ExpansionResidualKind::TemporalAnchorUnresolved,
    ExpansionResidualKind::ConditionalScopeUnresolved,
    ExpansionResidualKind::ClauseInterpretationAmbiguous,
    ExpansionResidualKind::ReferenceAttachmentUnresolved,
    ExpansionResidualKind::QualifierAttachmentUnresolved,
    ExpansionResidualKind::UnsupportedDependency,
];

fn residual_index(kind: ExpansionResidualKind) -> usize {
    match kind {
        ExpansionResidualKind::NegationScopeUnresolved => 0,
        ExpansionResidualKind::ModalityScopeUnresolved => 1,
        ExpansionResidualKind::TemporalAnchorUnresolved => 2,
        ExpansionResidualKind::ConditionalScopeUnresolved => 3,
        ExpansionResidualKind::ClauseInterpretationAmbiguous => 4,
        ExpansionResidualKind::ReferenceAttachmentUnresolved => 5,
        ExpansionResidualKind::QualifierAttachmentUnresolved => 6,
        ExpansionResidualKind::UnsupportedDependency => 7,
    }
}

pub fn residual_kind_name(kind: ExpansionResidualKind) -> &'static str {
    match kind {
        ExpansionResidualKind::NegationScopeUnresolved => "negation_scope_unresolved",
        ExpansionResidualKind::ModalityScopeUnresolved => "modality_scope_unresolved",
        ExpansionResidualKind::TemporalAnchorUnresolved => "temporal_anchor_unresolved",
        ExpansionResidualKind::ConditionalScopeUnresolved => "conditional_scope_unresolved",
        ExpansionResidualKind::ClauseInterpretationAmbiguous => "clause_interpretation_ambiguous",
        ExpansionResidualKind::ReferenceAttachmentUnresolved => "reference_attachment_unresolved",
        ExpansionResidualKind::QualifierAttachmentUnresolved => "qualifier_attachment_unresolved",
        ExpansionResidualKind::UnsupportedDependency => "unsupported_dependency",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResidualFrontier {
    counts: [u64; 8],
}

impl ResidualFrontier {
    pub fn observe_residual(&mut self, residual: &StableResidualObservation) {
        let index = residual_index(residual.kind);
        self.counts[index] = self.counts[index].saturating_add(1);
    }
    pub fn observe_consumer(&mut self, observation: &ExpandedConsumerObservation) {
        for residual in &observation.residuals {
            self.observe_residual(residual);
        }
    }
    pub fn count(&self, kind: ExpansionResidualKind) -> u64 {
        self.counts[residual_index(kind)]
    }
    pub fn total(&self) -> u64 {
        self.counts.iter().copied().sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontierWeight {
    pub kind: ExpansionResidualKind,
    pub legal_importance: u32,
    pub resolvability: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankedResidualFrontierEntry {
    pub kind: ExpansionResidualKind,
    pub count: u64,
    pub legal_importance: u32,
    pub resolvability: u32,
    pub priority_score: u128,
}

/// Work-selection score only; never semantic confidence/truth/authority.
pub fn rank_residual_frontier(
    frontier: &ResidualFrontier,
    weights: &[FrontierWeight],
) -> Vec<RankedResidualFrontierEntry> {
    let mut ranked: Vec<_> = RESIDUAL_KINDS
        .into_iter()
        .map(|kind| {
            let weight = weights.iter().find(|weight| weight.kind == kind);
            let legal_importance = weight.map_or(0, |weight| weight.legal_importance);
            let resolvability = weight.map_or(0, |weight| weight.resolvability);
            let count = frontier.count(kind);
            RankedResidualFrontierEntry {
                kind,
                count,
                legal_importance,
                resolvability,
                priority_score: u128::from(count)
                    * u128::from(legal_importance)
                    * u128::from(resolvability),
            }
        })
        .collect();
    ranked.sort_by(|left, right| {
        right
            .priority_score
            .cmp(&left.priority_score)
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| residual_index(left.kind).cmp(&residual_index(right.kind)))
    });
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::{Annotation, Capability, HeadDeclaration, TokenObservation};
    use sensiblaw_semantic_expansion::{
        ExpansionSignal, ExpandedSemanticRole, ScopeState, StableAlternativeFibreObservation,
        observe_direct_expanded,
    };

    #[derive(Debug)]
    struct GoldRow<'a> {
        id: &'a str,
        status: &'a str,
        signal: &'a str,
        local: u32,
        head: u32,
        start: u32,
        end: u32,
        candidate: &'a str,
        scope: &'a str,
        residual: &'a str,
        alternatives: &'a str,
    }

    fn rows() -> Vec<GoldRow<'static>> {
        include_str!("../../../fixtures/legal_semantic_conformance_v0_1.tsv")
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let f: Vec<_> = line.split('\t').collect();
                assert_eq!(f.len(), 11, "bad fixture row: {line}");
                GoldRow {
                    id: f[0], status: f[1], signal: f[2],
                    local: f[3].parse().unwrap(), head: f[4].parse().unwrap(),
                    start: f[5].parse().unwrap(), end: f[6].parse().unwrap(),
                    candidate: f[7], scope: f[8], residual: f[9], alternatives: f[10],
                }
            })
            .collect()
    }

    fn parse_signal(name: &str) -> ExpansionSignal {
        match name {
            "NominalSubject" => ExpansionSignal::NominalSubject,
            "DirectObject" => ExpansionSignal::DirectObject,
            "Negation" => ExpansionSignal::Negation,
            "ModalAuxiliary" => ExpansionSignal::ModalAuxiliary,
            "TemporalModifier" => ExpansionSignal::TemporalModifier,
            "ConditionalMarker" => ExpansionSignal::ConditionalMarker,
            "ClausalModifier" => ExpansionSignal::ClausalModifier,
            "ReferenceAttachment" => ExpansionSignal::ReferenceAttachment,
            "QualifierAttachment" => ExpansionSignal::QualifierAttachment,
            "Unsupported" => ExpansionSignal::Unsupported,
            other => panic!("unknown signal {other}"),
        }
    }

    fn parse_candidate(name: &str) -> Option<ExpandedCandidateKind> {
        match name {
            "-" => None,
            "Role:Actor" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Actor)),
            "Role:Object" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Object)),
            "Role:Modality" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Modality)),
            "Role:Temporal" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Temporal)),
            "ScopedNegation" => Some(ExpandedCandidateKind::ScopedNegation),
            "ConditionalRelation" => Some(ExpandedCandidateKind::ConditionalRelation),
            "ReferenceRelation" => Some(ExpandedCandidateKind::ReferenceRelation),
            "Role:Qualifier" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Qualifier)),
            other => panic!("unknown covered candidate {other}"),
        }
    }

    fn parse_scope(name: &str) -> Option<ScopeState> {
        match name {
            "-" => None,
            "SyntacticallyLocal" => Some(ScopeState::SyntacticallyLocal),
            "ScopeUnresolved" => Some(ScopeState::ScopeUnresolved),
            "AttachmentUnresolved" => Some(ScopeState::AttachmentUnresolved),
            "ContextRequired" => Some(ScopeState::ContextRequired),
            other => panic!("unknown scope {other}"),
        }
    }

    fn parse_residual(name: &str) -> Option<ExpansionResidualKind> {
        match name {
            "-" => None,
            "NegationScopeUnresolved" => Some(ExpansionResidualKind::NegationScopeUnresolved),
            "ModalityScopeUnresolved" => Some(ExpansionResidualKind::ModalityScopeUnresolved),
            "TemporalAnchorUnresolved" => Some(ExpansionResidualKind::TemporalAnchorUnresolved),
            "ConditionalScopeUnresolved" => Some(ExpansionResidualKind::ConditionalScopeUnresolved),
            "ClauseInterpretationAmbiguous" => Some(ExpansionResidualKind::ClauseInterpretationAmbiguous),
            "ReferenceAttachmentUnresolved" => Some(ExpansionResidualKind::ReferenceAttachmentUnresolved),
            "QualifierAttachmentUnresolved" => Some(ExpansionResidualKind::QualifierAttachmentUnresolved),
            "UnsupportedDependency" => Some(ExpansionResidualKind::UnsupportedDependency),
            other => panic!("unknown residual {other}"),
        }
    }

    fn parse_alternatives(value: &str) -> Vec<ExpandedSemanticRole> {
        if value == "-" { return Vec::new(); }
        value.split(',').map(|name| match name {
            "Condition" => ExpandedSemanticRole::Condition,
            "Temporal" => ExpandedSemanticRole::Temporal,
            "Qualifier" => ExpandedSemanticRole::Qualifier,
            other => panic!("unknown alternative {other}"),
        }).collect()
    }

    fn actual(row: &GoldRow<'_>, sentence_id: u64) -> ExpandedConsumerObservation {
        let source = TokenObservation {
            token_id: sentence_id * 10 + 1,
            sentence_id,
            local_ordinal: row.local,
            span: TextSpan::new(1, row.start, row.end).unwrap(),
            orth: 1,
            lemma: Annotation::Unavailable(Capability::Lemma),
            pos: Annotation::Unavailable(Capability::Pos),
            tag: Annotation::Unavailable(Capability::Tag),
            dependency: Annotation::Present(1),
            declared_head: HeadDeclaration::LocalOrdinal(row.head),
        };
        let root = TokenObservation {
            token_id: sentence_id * 10 + 2,
            sentence_id,
            local_ordinal: row.head,
            span: TextSpan::new(1, row.end + 1, row.end + 2).unwrap(),
            orth: 2,
            lemma: Annotation::Unavailable(Capability::Lemma),
            pos: Annotation::Unavailable(Capability::Pos),
            tag: Annotation::Unavailable(Capability::Tag),
            dependency: Annotation::Unavailable(Capability::Dependency),
            declared_head: HeadDeclaration::SelfHead,
        };
        let signal = parse_signal(row.signal);
        observe_direct_expanded(vec![source, root], |_| signal)
    }

    fn expected(row: &GoldRow<'_>, sentence_id: u64) -> ExpandedConsumerObservation {
        let address = FibreAddress { sentence_id, local_ordinal: row.local };
        let candidates = match (parse_candidate(row.candidate), parse_scope(row.scope)) {
            (Some(kind), Some(scope)) => vec![StableCandidateObservation {
                kind,
                span: TextSpan::new(1, row.start, row.end).unwrap(),
                address,
                head: StableHeadRelation::LocalOrdinal(row.head),
                scope,
                candidate_only: true,
            }],
            (None, None) => Vec::new(),
            _ => panic!("bad candidate/scope fixture {}", row.id),
        };
        let residuals = parse_residual(row.residual)
            .map(|kind| vec![StableResidualObservation { kind, address }])
            .unwrap_or_default();
        let alternatives = parse_alternatives(row.alternatives);
        let alternative_fibres = if alternatives.is_empty() { Vec::new() } else {
            vec![StableAlternativeFibreObservation { address, alternatives }]
        };
        ExpandedConsumerObservation {
            sentence_id,
            candidates,
            residuals,
            alternative_fibres,
            projection_failures: Vec::new(),
        }
    }

    #[test]
    fn covered_gold_fixtures_match_exact_consumer_objects() {
        let mut covered = 0;
        for (index, row) in rows().iter().enumerate() {
            if row.status != "covered" { continue; }
            covered += 1;
            let sentence_id = index as u64 + 1;
            assert_eq!(actual(row, sentence_id), expected(row, sentence_id), "{}", row.id);
        }
        assert_eq!(covered, 10);
    }

    #[test]
    fn gold_corpus_keeps_missing_producers_explicit() {
        let gaps: Vec<_> = rows().into_iter()
            .filter(|row| row.status == "producer_gap")
            .map(|row| row.candidate)
            .collect();
        assert_eq!(gaps, vec![
            "Role:Action", "Role:Exception", "Role:Jurisdiction",
            "Role:Speaker", "Role:Evidence", "Role:Provenance",
        ]);
    }

    fn negation_candidate(scope: ScopeState) -> StableCandidateObservation {
        StableCandidateObservation {
            kind: ExpandedCandidateKind::ScopedNegation,
            span: TextSpan::new(7, 10, 13).unwrap(),
            address: FibreAddress { sentence_id: 11, local_ordinal: 2 },
            head: StableHeadRelation::LocalOrdinal(3),
            scope,
            candidate_only: true,
        }
    }

    fn receipt(resolved_scope: ResolvedScope) -> AdmissionReceipt {
        AdmissionReceipt {
            address: FibreAddress { sentence_id: 11, local_ordinal: 2 },
            source_span: TextSpan::new(7, 10, 13).unwrap(),
            kind: ExpandedCandidateKind::ScopedNegation,
            resolved_scope,
            authority: ResolutionAuthority::HumanReview,
            policy_reference: "gold:v0.1:negation-scope".into(),
            resolver_reference: "review:fixture-negation".into(),
        }
    }

    #[test]
    fn unresolved_scope_cannot_be_admitted_as_local_syntax() {
        assert_eq!(
            admit_candidate(&negation_candidate(ScopeState::ScopeUnresolved), &receipt(ResolvedScope::LocalSyntactic)),
            Err(AdmissionError::ScopeResolutionMismatch),
        );
    }

    #[test]
    fn exact_resolution_receipt_admits_without_publication_authority() {
        let delta = admit_candidate(
            &negation_candidate(ScopeState::ScopeUnresolved),
            &receipt(ResolvedScope::ScopeResolved),
        ).unwrap();
        assert_eq!(delta.resolved_scope, ResolvedScope::ScopeResolved);
        assert_eq!(delta.authority, ResolutionAuthority::HumanReview);
    }

    #[test]
    fn no_receipt_preserves_candidate_residual_and_alternative() {
        let candidate = negation_candidate(ScopeState::ScopeUnresolved);
        let address = candidate.address;
        let observation = ExpandedConsumerObservation {
            sentence_id: 11,
            candidates: vec![candidate.clone()],
            residuals: vec![StableResidualObservation {
                kind: ExpansionResidualKind::NegationScopeUnresolved,
                address,
            }],
            alternative_fibres: vec![StableAlternativeFibreObservation {
                address,
                alternatives: vec![ExpandedSemanticRole::Condition, ExpandedSemanticRole::Qualifier],
            }],
            projection_failures: Vec::new(),
        };
        let outcome = admit_with_receipts(&observation, &[]);
        assert!(outcome.admitted.is_empty());
        assert_eq!(outcome.retained_candidates, vec![candidate]);
        assert_eq!(outcome.retained_residuals, observation.residuals);
        assert_eq!(outcome.retained_alternative_fibres, observation.alternative_fibres);
    }

    #[test]
    fn residual_frequency_does_not_force_priority() {
        let address = FibreAddress { sentence_id: 1, local_ordinal: 0 };
        let observation = ExpandedConsumerObservation {
            sentence_id: 1,
            candidates: Vec::new(),
            residuals: vec![
                StableResidualObservation { kind: ExpansionResidualKind::UnsupportedDependency, address },
                StableResidualObservation { kind: ExpansionResidualKind::UnsupportedDependency, address },
                StableResidualObservation { kind: ExpansionResidualKind::ConditionalScopeUnresolved, address },
            ],
            alternative_fibres: Vec::new(),
            projection_failures: Vec::new(),
        };
        let mut frontier = ResidualFrontier::default();
        frontier.observe_consumer(&observation);
        let ranked = rank_residual_frontier(&frontier, &[
            FrontierWeight { kind: ExpansionResidualKind::UnsupportedDependency, legal_importance: 1, resolvability: 1 },
            FrontierWeight { kind: ExpansionResidualKind::ConditionalScopeUnresolved, legal_importance: 5, resolvability: 5 },
        ]);
        assert_eq!(frontier.total(), 3);
        assert_eq!(ranked[0].kind, ExpansionResidualKind::ConditionalScopeUnresolved);
        assert_eq!(ranked[0].priority_score, 25);
    }
}
