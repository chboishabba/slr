//! Candidate-to-admitted semantic boundary for SensibLaw.
//!
//! Parser-supported expanded semantics remain candidate evidence. Admission requires
//! an explicit receipt whose authority is outside the parser. Rejected or unresolved
//! candidates retain their evidence/residual/alternative fibres, and this crate has
//! no publication API or durable authority-id materialization.

use sensiblaw_core::{FibreAddress, TextSpan};
use sensiblaw_semantic_expansion::{
    ExpandedCandidateKind, ExpandedConsumerObservation, ExpandedResidualKind,
    ExpandedSemanticRole, ScopeState, StableAlternativeFibreObservation,
    StableCandidateObservation, StableHeadRelation, StableResidualObservation,
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

fn scope_matches(candidate: ScopeState, resolved: ResolvedScope) -> bool {
    matches!(
        (candidate, resolved),
        (ScopeState::SyntacticallyLocal, ResolvedScope::LocalSyntactic)
            | (ScopeState::ScopeUnresolved, ResolvedScope::ScopeResolved)
            | (ScopeState::AttachmentUnresolved, ResolvedScope::AttachmentResolved)
            | (ScopeState::ContextRequired, ResolvedScope::ContextResolved)
    )
}

/// Admit one candidate only when a non-parser resolution receipt identifies the
/// exact stable candidate and resolves the kind of scope/attachment it carries.
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

/// Sparse, fail-closed admission. A candidate with no matching receipt stays a
/// candidate. A bad receipt also leaves the candidate in place and records a
/// failure. Residuals and alternative fibres are never deleted by admission.
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
                    failures.push(AdmissionFailure {
                        candidate: candidate.clone(),
                        error,
                    });
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidualFrontierEntry {
    pub kind: ExpandedResidualKind,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResidualFrontier {
    counts: [u64; 8],
}

fn residual_index(kind: ExpandedResidualKind) -> usize {
    match kind {
        ExpandedResidualKind::NegationScopeUnresolved => 0,
        ExpandedResidualKind::ModalityScopeUnresolved => 1,
        ExpandedResidualKind::TemporalAnchorUnresolved => 2,
        ExpandedResidualKind::ConditionalScopeUnresolved => 3,
        ExpandedResidualKind::ClauseInterpretationAmbiguous => 4,
        ExpandedResidualKind::ReferenceAttachmentUnresolved => 5,
        ExpandedResidualKind::QualifierAttachmentUnresolved => 6,
        ExpandedResidualKind::UnsupportedDependency => 7,
    }
}

pub const RESIDUAL_KINDS: [ExpandedResidualKind; 8] = [
    ExpandedResidualKind::NegationScopeUnresolved,
    ExpandedResidualKind::ModalityScopeUnresolved,
    ExpandedResidualKind::TemporalAnchorUnresolved,
    ExpandedResidualKind::ConditionalScopeUnresolved,
    ExpandedResidualKind::ClauseInterpretationAmbiguous,
    ExpandedResidualKind::ReferenceAttachmentUnresolved,
    ExpandedResidualKind::QualifierAttachmentUnresolved,
    ExpandedResidualKind::UnsupportedDependency,
];

pub fn residual_kind_name(kind: ExpandedResidualKind) -> &'static str {
    match kind {
        ExpandedResidualKind::NegationScopeUnresolved => "negation_scope_unresolved",
        ExpandedResidualKind::ModalityScopeUnresolved => "modality_scope_unresolved",
        ExpandedResidualKind::TemporalAnchorUnresolved => "temporal_anchor_unresolved",
        ExpandedResidualKind::ConditionalScopeUnresolved => "conditional_scope_unresolved",
        ExpandedResidualKind::ClauseInterpretationAmbiguous => "clause_interpretation_ambiguous",
        ExpandedResidualKind::ReferenceAttachmentUnresolved => "reference_attachment_unresolved",
        ExpandedResidualKind::QualifierAttachmentUnresolved => "qualifier_attachment_unresolved",
        ExpandedResidualKind::UnsupportedDependency => "unsupported_dependency",
    }
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

    pub fn count(&self, kind: ExpandedResidualKind) -> u64 {
        self.counts[residual_index(kind)]
    }

    pub fn total(&self) -> u64 {
        self.counts.iter().copied().sum()
    }

    pub fn entries(&self) -> Vec<ResidualFrontierEntry> {
        RESIDUAL_KINDS
            .into_iter()
            .map(|kind| ResidualFrontierEntry {
                kind,
                count: self.count(kind),
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontierWeight {
    pub kind: ExpandedResidualKind,
    pub legal_importance: u32,
    pub resolvability: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankedResidualFrontierEntry {
    pub kind: ExpandedResidualKind,
    pub count: u64,
    pub legal_importance: u32,
    pub resolvability: u32,
    pub priority_score: u128,
}

/// Policy-weighted prioritization only. The score is a work-selection observer,
/// not semantic quality, confidence, truth, or authority.
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

pub fn semantic_role_name(role: ExpandedSemanticRole) -> &'static str {
    match role {
        ExpandedSemanticRole::Actor => "Actor",
        ExpandedSemanticRole::Action => "Action",
        ExpandedSemanticRole::Object => "Object",
        ExpandedSemanticRole::Modality => "Modality",
        ExpandedSemanticRole::Condition => "Condition",
        ExpandedSemanticRole::Exception => "Exception",
        ExpandedSemanticRole::Qualifier => "Qualifier",
        ExpandedSemanticRole::Jurisdiction => "Jurisdiction",
        ExpandedSemanticRole::Speaker => "Speaker",
        ExpandedSemanticRole::Evidence => "Evidence",
        ExpandedSemanticRole::Temporal => "Temporal",
        ExpandedSemanticRole::Provenance => "Provenance",
        ExpandedSemanticRole::Reference => "Reference",
        ExpandedSemanticRole::Unknown => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::{
        Annotation, Capability, HeadDeclaration, TextSpan, TokenObservation,
    };
    use sensiblaw_semantic_expansion::{
        ExpansionSignal, StableCandidateObservation, StableResidualObservation,
        observe_direct_expanded,
    };

    #[derive(Debug)]
    struct GoldRow<'a> {
        fixture_id: &'a str,
        status: &'a str,
        signal: &'a str,
        local: u32,
        head: u32,
        start: u32,
        end: u32,
        expected_candidate: &'a str,
        expected_scope: &'a str,
        expected_residual: &'a str,
        expected_alternatives: &'a str,
    }

    fn gold_rows() -> Vec<GoldRow<'static>> {
        include_str!("../../../fixtures/legal_semantic_conformance_v0_1.tsv")
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let fields: Vec<_> = line.split('\t').collect();
                assert_eq!(fields.len(), 11, "bad gold fixture row: {line}");
                GoldRow {
                    fixture_id: fields[0],
                    status: fields[1],
                    signal: fields[2],
                    local: fields[3].parse().unwrap(),
                    head: fields[4].parse().unwrap(),
                    start: fields[5].parse().unwrap(),
                    end: fields[6].parse().unwrap(),
                    expected_candidate: fields[7],
                    expected_scope: fields[8],
                    expected_residual: fields[9],
                    expected_alternatives: fields[10],
                }
            })
            .collect()
    }

    fn signal(name: &str) -> ExpansionSignal {
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
            other => panic!("unknown covered signal {other}"),
        }
    }

    fn candidate_kind(name: &str) -> Option<ExpandedCandidateKind> {
        match name {
            "-" => None,
            "Role:Actor" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Actor)),
            "Role:Action" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Action)),
            "Role:Object" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Object)),
            "Role:Modality" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Modality)),
            "Role:Exception" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Exception)),
            "Role:Qualifier" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Qualifier)),
            "Role:Jurisdiction" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Jurisdiction)),
            "Role:Speaker" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Speaker)),
            "Role:Evidence" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Evidence)),
            "Role:Temporal" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Temporal)),
            "Role:Provenance" => Some(ExpandedCandidateKind::Role(ExpandedSemanticRole::Provenance)),
            "ScopedNegation" => Some(ExpandedCandidateKind::ScopedNegation),
            "ConditionalRelation" => Some(ExpandedCandidateKind::ConditionalRelation),
            "ReferenceRelation" => Some(ExpandedCandidateKind::ReferenceRelation),
            other => panic!("unknown candidate kind {other}"),
        }
    }

    fn scope(name: &str) -> Option<ScopeState> {
        match name {
            "-" => None,
            "SyntacticallyLocal" => Some(ScopeState::SyntacticallyLocal),
            "ScopeUnresolved" => Some(ScopeState::ScopeUnresolved),
            "AttachmentUnresolved" => Some(ScopeState::AttachmentUnresolved),
            "ContextRequired" => Some(ScopeState::ContextRequired),
            other => panic!("unknown scope {other}"),
        }
    }

    fn residual(name: &str) -> Option<ExpandedResidualKind> {
        match name {
            "-" => None,
            "NegationScopeUnresolved" => Some(ExpandedResidualKind::NegationScopeUnresolved),
            "ModalityScopeUnresolved" => Some(ExpandedResidualKind::ModalityScopeUnresolved),
            "TemporalAnchorUnresolved" => Some(ExpandedResidualKind::TemporalAnchorUnresolved),
            "ConditionalScopeUnresolved" => Some(ExpandedResidualKind::ConditionalScopeUnresolved),
            "ClauseInterpretationAmbiguous" => Some(ExpandedResidualKind::ClauseInterpretationAmbiguous),
            "ReferenceAttachmentUnresolved" => Some(ExpandedResidualKind::ReferenceAttachmentUnresolved),
            "QualifierAttachmentUnresolved" => Some(ExpandedResidualKind::QualifierAttachmentUnresolved),
            "UnsupportedDependency" => Some(ExpandedResidualKind::UnsupportedDependency),
            other => panic!("unknown residual {other}"),
        }
    }

    fn alternative_roles(value: &str) -> Vec<ExpandedSemanticRole> {
        if value == "-" {
            return Vec::new();
        }
        value
            .split(',')
            .map(|name| match name {
                "Condition" => ExpandedSemanticRole::Condition,
                "Temporal" => ExpandedSemanticRole::Temporal,
                "Qualifier" => ExpandedSemanticRole::Qualifier,
                other => panic!("unknown alternative role {other}"),
            })
            .collect()
    }

    fn covered_observation(row: &GoldRow<'_>, sentence_id: u64) -> ExpandedConsumerObservation {
        let signal = signal(row.signal);
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
        observe_direct_expanded(vec![source, root], |_| signal)
    }

    fn expected_observation(row: &GoldRow<'_>, sentence_id: u64) -> ExpandedConsumerObservation {
        let address = FibreAddress {
            sentence_id,
            local_ordinal: row.local,
        };
        let candidates = match (candidate_kind(row.expected_candidate), scope(row.expected_scope)) {
            (Some(kind), Some(scope)) => vec![StableCandidateObservation {
                kind,
                span: TextSpan::new(1, row.start, row.end).unwrap(),
                address,
                head: StableHeadRelation::LocalOrdinal(row.head),
                scope,
                candidate_only: true,
            }],
            (None, None) => Vec::new(),
            _ => panic!("inconsistent candidate/scope fixture {}", row.fixture_id),
        };
        let residuals = residual(row.expected_residual)
            .map(|kind| vec![StableResidualObservation { kind, address }])
            .unwrap_or_default();
        let alternatives = alternative_roles(row.expected_alternatives);
        let alternative_fibres = if alternatives.is_empty() {
            Vec::new()
        } else {
            vec![StableAlternativeFibreObservation {
                address,
                alternatives,
            }]
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
        let rows = gold_rows();
        let mut covered = 0usize;
        for (index, row) in rows.iter().enumerate() {
            if row.status != "covered" {
                continue;
            }
            covered += 1;
            let sentence_id = index as u64 + 1;
            assert_eq!(
                covered_observation(row, sentence_id),
                expected_observation(row, sentence_id),
                "gold fixture failed: {}",
                row.fixture_id,
            );
        }
        assert_eq!(covered, 10);
    }

    #[test]
    fn gold_corpus_keeps_unimplemented_legal_roles_visible_as_gaps() {
        let gaps: Vec<_> = gold_rows()
            .into_iter()
            .filter(|row| row.status == "producer_gap")
            .map(|row| row.expected_candidate)
            .collect();
        assert_eq!(
            gaps,
            vec![
                "Role:Action",
                "Role:Exception",
                "Role:Jurisdiction",
                "Role:Speaker",
                "Role:Evidence",
                "Role:Provenance",
            ]
        );
    }

    fn candidate(scope: ScopeState) -> StableCandidateObservation {
        StableCandidateObservation {
            kind: ExpandedCandidateKind::ScopedNegation,
            span: TextSpan::new(7, 10, 13).unwrap(),
            address: FibreAddress {
                sentence_id: 11,
                local_ordinal: 2,
            },
            head: StableHeadRelation::LocalOrdinal(3),
            scope,
            candidate_only: true,
        }
    }

    fn receipt(resolved_scope: ResolvedScope) -> AdmissionReceipt {
        AdmissionReceipt {
            address: FibreAddress {
                sentence_id: 11,
                local_ordinal: 2,
            },
            source_span: TextSpan::new(7, 10, 13).unwrap(),
            kind: ExpandedCandidateKind::ScopedNegation,
            resolved_scope,
            authority: ResolutionAuthority::HumanReview,
            policy_reference: "legal-semantic-gold:v0.1:negation-scope".to_owned(),
            resolver_reference: "review:fixture-negation-scope".to_owned(),
        }
    }

    #[test]
    fn unresolved_scope_cannot_be_admitted_as_local_syntax() {
        assert_eq!(
            admit_candidate(&candidate(ScopeState::ScopeUnresolved), &receipt(ResolvedScope::LocalSyntactic)),
            Err(AdmissionError::ScopeResolutionMismatch),
        );
    }

    #[test]
    fn exact_resolution_receipt_admits_candidate_without_publication() {
        let admitted = admit_candidate(
            &candidate(ScopeState::ScopeUnresolved),
            &receipt(ResolvedScope::ScopeResolved),
        )
        .unwrap();
        assert_eq!(admitted.resolved_scope, ResolvedScope::ScopeResolved);
        assert_eq!(admitted.authority, ResolutionAuthority::HumanReview);
    }

    #[test]
    fn no_receipt_preserves_candidate_residual_and_alternatives() {
        let candidate = candidate(ScopeState::ScopeUnresolved);
        let address = candidate.address;
        let observation = ExpandedConsumerObservation {
            sentence_id: 11,
            candidates: vec![candidate.clone()],
            residuals: vec![StableResidualObservation {
                kind: ExpandedResidualKind::NegationScopeUnresolved,
                address,
            }],
            alternative_fibres: vec![StableAlternativeFibreObservation {
                address,
                alternatives: vec![
                    ExpandedSemanticRole::Condition,
                    ExpandedSemanticRole::Qualifier,
                ],
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
    fn residual_frontier_counts_and_policy_ranking_are_separate_from_quality() {
        let address = FibreAddress {
            sentence_id: 1,
            local_ordinal: 0,
        };
        let observation = ExpandedConsumerObservation {
            sentence_id: 1,
            candidates: Vec::new(),
            residuals: vec![
                StableResidualObservation {
                    kind: ExpandedResidualKind::UnsupportedDependency,
                    address,
                },
                StableResidualObservation {
                    kind: ExpandedResidualKind::UnsupportedDependency,
                    address,
                },
                StableResidualObservation {
                    kind: ExpandedResidualKind::ConditionalScopeUnresolved,
                    address,
                },
            ],
            alternative_fibres: Vec::new(),
            projection_failures: Vec::new(),
        };
        let mut frontier = ResidualFrontier::default();
        frontier.observe_consumer(&observation);
        assert_eq!(frontier.total(), 3);
        assert_eq!(frontier.count(ExpandedResidualKind::UnsupportedDependency), 2);
        let ranked = rank_residual_frontier(
            &frontier,
            &[
                FrontierWeight {
                    kind: ExpandedResidualKind::UnsupportedDependency,
                    legal_importance: 1,
                    resolvability: 1,
                },
                FrontierWeight {
                    kind: ExpandedResidualKind::ConditionalScopeUnresolved,
                    legal_importance: 5,
                    resolvability: 5,
                },
            ],
        );
        assert_eq!(ranked[0].kind, ExpandedResidualKind::ConditionalScopeUnresolved);
        assert_eq!(ranked[0].priority_score, 25);
    }
}
