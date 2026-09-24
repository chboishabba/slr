//! M14.A Matter-native work-product coverage projection.
//!
//! This module is deliberately a reader-model projection over an already
//! reviewed `MatterWorkspaceProjection`. It does not add a new world carrier,
//! mutate Matter, or promote work-product wording into semantic authority.
//!
//! The fine relation vocabulary is migrated from the historical SensibLaw
//! typed-claim reconciliation lane as evidence metadata. Forward work-product
//! coverage and reverse Matter-to-product omission are separate axes.

use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_core::chronology_contestation::ClaimReviewState;

use crate::{MatterWorkspaceProjection, PropositionContestationView, SemanticTracePath};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkProductMatterRelation {
    ExactSupport,
    EquivalentSupport,
    ExplicitDispute,
    ImplicitDispute,
    PartialOverlap,
    AdjacentEvent,
    Substitution,
    ProceduralNonanswer,
    Unrelated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkProductRelationRoot {
    Supports,
    Invalidates,
    NonResolving,
    Unanswered,
}

impl WorkProductMatterRelation {
    #[must_use]
    pub const fn root(self) -> WorkProductRelationRoot {
        match self {
            Self::ExactSupport | Self::EquivalentSupport | Self::PartialOverlap => {
                WorkProductRelationRoot::Supports
            }
            Self::ExplicitDispute | Self::ImplicitDispute => {
                WorkProductRelationRoot::Invalidates
            }
            Self::AdjacentEvent | Self::Substitution | Self::ProceduralNonanswer => {
                WorkProductRelationRoot::NonResolving
            }
            Self::Unrelated => WorkProductRelationRoot::Unanswered,
        }
    }

    #[must_use]
    pub const fn resolves_coverage(self) -> bool {
        matches!(
            self,
            Self::ExactSupport
                | Self::EquivalentSupport
                | Self::ExplicitDispute
                | Self::ImplicitDispute
                | Self::PartialOverlap
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkProductPropositionOccurrence {
    pub occurrence_ref: String,
    pub work_product_ref: String,
    pub statement_ref: String,
    pub source_revision_ref: String,
    pub exact_span_ref: String,
    pub product_proposition_ref: String,
    /// Receipt for the bounded Matter candidate lookup. This remains required
    /// even when no Matter proposition is matched, so Unsupported is reopenable
    /// to the actual search rather than read as a truth judgment.
    pub candidate_search_receipt_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkProductMatterMatch {
    pub occurrence_ref: String,
    pub matter_proposition_ref: String,
    pub relation: WorkProductMatterRelation,
    pub comparison_receipt_ref: String,
    pub review_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForwardCoverageStatus {
    Supported,
    Qualified,
    Contradicted,
    Unreviewed,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkProductCoverageJudgment {
    pub occurrence_ref: String,
    pub work_product_ref: String,
    pub product_proposition_ref: String,
    pub matter_proposition_ref: Option<String>,
    pub relation: Option<WorkProductMatterRelation>,
    pub status: ForwardCoverageStatus,
    pub work_product_source_refs: Vec<String>,
    pub matter_ancestry_refs: Vec<String>,
    pub comparison_receipt_refs: Vec<String>,
    pub review_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatterWorkProductExpectationKind {
    ExpectedInProduct,
    ExcludedByScope,
    NotExpectedForProduct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterWorkProductExpectation {
    pub matter_proposition_ref: String,
    pub kind: MatterWorkProductExpectationKind,
    pub basis_ref: String,
    pub review_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReverseCoverageStatus {
    Represented,
    PossiblyOmitted,
    ExcludedByScope,
    NotExpectedForProduct,
    UnreviewedForOmission,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterOmissionJudgment {
    pub matter_proposition_ref: String,
    pub status: ReverseCoverageStatus,
    pub expectation_basis_ref: String,
    pub resolving_occurrence_refs: Vec<String>,
    pub matter_ancestry_refs: Vec<String>,
    pub review_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkProductCoverageProjection {
    pub matter_ref: String,
    pub forward: Vec<WorkProductCoverageJudgment>,
    pub reverse: Vec<MatterOmissionJudgment>,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub canonical_world_mutated: bool,
    pub supported_implies_legal_sufficiency: bool,
    pub contradicted_implies_false: bool,
    pub unsupported_implies_false: bool,
    pub omission_implies_should_include: bool,
    pub redaction_deletes_canonical_source: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkProductCoverageError {
    MatterPromotionBoundary,
    EmptyCoordinate(&'static str),
    OccurrencePromotionBoundary(String),
    MatchPromotionBoundary(String),
    ExpectationPromotionBoundary(String),
    DuplicateOccurrence(String),
    DuplicateMatch(String),
    UnknownOccurrence(String),
    UnknownMatterProposition(String),
    MissingMatterAncestry(String),
    DuplicateExpectation(String),
}

fn require(name: &'static str, value: &str) -> Result<(), WorkProductCoverageError> {
    if value.trim().is_empty() {
        Err(WorkProductCoverageError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

impl WorkProductPropositionOccurrence {
    pub fn validate(&self) -> Result<(), WorkProductCoverageError> {
        for (name, value) in [
            ("occurrence_ref", self.occurrence_ref.as_str()),
            ("work_product_ref", self.work_product_ref.as_str()),
            ("statement_ref", self.statement_ref.as_str()),
            ("source_revision_ref", self.source_revision_ref.as_str()),
            ("exact_span_ref", self.exact_span_ref.as_str()),
            ("product_proposition_ref", self.product_proposition_ref.as_str()),
            (
                "candidate_search_receipt_ref",
                self.candidate_search_receipt_ref.as_str(),
            ),
        ] {
            require(name, value)?;
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(WorkProductCoverageError::OccurrencePromotionBoundary(
                self.occurrence_ref.clone(),
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn source_chain(&self) -> Vec<String> {
        vec![
            self.source_revision_ref.clone(),
            self.exact_span_ref.clone(),
            self.statement_ref.clone(),
        ]
    }
}

impl WorkProductMatterMatch {
    pub fn validate(&self) -> Result<(), WorkProductCoverageError> {
        for (name, value) in [
            ("occurrence_ref", self.occurrence_ref.as_str()),
            ("matter_proposition_ref", self.matter_proposition_ref.as_str()),
            ("comparison_receipt_ref", self.comparison_receipt_ref.as_str()),
        ] {
            require(name, value)?;
        }
        if self
            .review_ref
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(WorkProductCoverageError::EmptyCoordinate("review_ref"));
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(WorkProductCoverageError::MatchPromotionBoundary(
                self.occurrence_ref.clone(),
            ));
        }
        Ok(())
    }
}

impl MatterWorkProductExpectation {
    pub fn validate(&self) -> Result<(), WorkProductCoverageError> {
        require("matter_proposition_ref", &self.matter_proposition_ref)?;
        require("basis_ref", &self.basis_ref)?;
        if self
            .review_ref
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(WorkProductCoverageError::EmptyCoordinate("review_ref"));
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(WorkProductCoverageError::ExpectationPromotionBoundary(
                self.matter_proposition_ref.clone(),
            ));
        }
        Ok(())
    }
}

fn matter_is_reviewed(view: &PropositionContestationView) -> bool {
    view.leaves
        .iter()
        .any(|leaf| !matches!(leaf.review_state, ClaimReviewState::Unreviewed))
}

fn forward_status(
    view: &PropositionContestationView,
    relation: WorkProductMatterRelation,
) -> ForwardCoverageStatus {
    if !matter_is_reviewed(view) {
        return ForwardCoverageStatus::Unreviewed;
    }
    match relation {
        WorkProductMatterRelation::ExactSupport
        | WorkProductMatterRelation::EquivalentSupport => ForwardCoverageStatus::Supported,
        WorkProductMatterRelation::PartialOverlap => ForwardCoverageStatus::Qualified,
        WorkProductMatterRelation::ExplicitDispute
        | WorkProductMatterRelation::ImplicitDispute => {
            ForwardCoverageStatus::Contradicted
        }
        WorkProductMatterRelation::AdjacentEvent
        | WorkProductMatterRelation::Substitution
        | WorkProductMatterRelation::ProceduralNonanswer
        | WorkProductMatterRelation::Unrelated => ForwardCoverageStatus::Unsupported,
    }
}

fn matter_ancestry(
    view: &PropositionContestationView,
    traces: &[SemanticTracePath],
) -> Vec<String> {
    let claim_refs = view
        .leaves
        .iter()
        .map(|leaf| leaf.claim_ref.clone())
        .collect::<BTreeSet<_>>();
    let statement_refs = view
        .leaves
        .iter()
        .flat_map(|leaf| leaf.statement_refs.iter().cloned())
        .collect::<BTreeSet<_>>();

    let mut ancestry = BTreeSet::new();
    ancestry.insert(view.root.proposition_ref.clone());
    ancestry.extend(claim_refs.iter().cloned());
    ancestry.extend(
        view.relations
            .iter()
            .map(|relation| relation.relation_ref.clone()),
    );

    for trace in traces {
        let trace_matches = statement_refs.contains(&trace.statement.statement_ref)
            || trace
                .claim_refs
                .iter()
                .any(|claim_ref| claim_refs.contains(claim_ref));
        if trace_matches {
            ancestry.extend(
                trace
                    .reverse_source_chain()
                    .into_iter()
                    .map(str::to_owned),
            );
        }
    }

    ancestry.into_iter().collect()
}

pub fn project_work_product_coverage(
    matter: &MatterWorkspaceProjection,
    occurrences: &[WorkProductPropositionOccurrence],
    matches: &[WorkProductMatterMatch],
    expectations: &[MatterWorkProductExpectation],
) -> Result<WorkProductCoverageProjection, WorkProductCoverageError> {
    if matter.creates_semantic_authority
        || matter.claim_truth_promoted
        || matter.canonical_world_mutated
        || matter.event_timeline.creates_semantic_authority
        || matter.event_timeline.claim_truth_promoted
    {
        return Err(WorkProductCoverageError::MatterPromotionBoundary);
    }

    let proposition_views = matter
        .event_timeline
        .proposition_views
        .iter()
        .map(|view| (view.root.proposition_ref.clone(), view))
        .collect::<BTreeMap<_, _>>();

    let mut occurrence_by_ref = BTreeMap::new();
    for occurrence in occurrences {
        occurrence.validate()?;
        if occurrence_by_ref
            .insert(occurrence.occurrence_ref.clone(), occurrence)
            .is_some()
        {
            return Err(WorkProductCoverageError::DuplicateOccurrence(
                occurrence.occurrence_ref.clone(),
            ));
        }
    }

    let mut match_by_occurrence = BTreeMap::new();
    for relation_match in matches {
        relation_match.validate()?;
        if !occurrence_by_ref.contains_key(&relation_match.occurrence_ref) {
            return Err(WorkProductCoverageError::UnknownOccurrence(
                relation_match.occurrence_ref.clone(),
            ));
        }
        if !proposition_views.contains_key(&relation_match.matter_proposition_ref) {
            return Err(WorkProductCoverageError::UnknownMatterProposition(
                relation_match.matter_proposition_ref.clone(),
            ));
        }
        if match_by_occurrence
            .insert(relation_match.occurrence_ref.clone(), relation_match)
            .is_some()
        {
            // M14.A deliberately fails closed instead of choosing silently
            // between multiple Matter propositions for one product occurrence.
            return Err(WorkProductCoverageError::DuplicateMatch(
                relation_match.occurrence_ref.clone(),
            ));
        }
    }

    let mut forward = Vec::new();
    for occurrence in occurrences {
        let (matter_proposition_ref, relation, status, ancestry, comparison_refs, review_ref) =
            if let Some(relation_match) = match_by_occurrence.get(&occurrence.occurrence_ref) {
                let view = proposition_views
                    .get(&relation_match.matter_proposition_ref)
                    .expect("validated Matter proposition");
                let ancestry = matter_ancestry(view, &matter.source_traces);
                if ancestry.iter().all(|value| {
                    value == &view.root.proposition_ref
                        || view.leaves.iter().any(|leaf| &leaf.claim_ref == value)
                        || view
                            .relations
                            .iter()
                            .any(|item| &item.relation_ref == value)
                }) {
                    return Err(WorkProductCoverageError::MissingMatterAncestry(
                        relation_match.matter_proposition_ref.clone(),
                    ));
                }
                (
                    Some(relation_match.matter_proposition_ref.clone()),
                    Some(relation_match.relation),
                    forward_status(view, relation_match.relation),
                    ancestry,
                    vec![
                        occurrence.candidate_search_receipt_ref.clone(),
                        relation_match.comparison_receipt_ref.clone(),
                    ],
                    relation_match.review_ref.clone(),
                )
            } else {
                (
                    None,
                    None,
                    ForwardCoverageStatus::Unsupported,
                    vec![],
                    vec![occurrence.candidate_search_receipt_ref.clone()],
                    None,
                )
            };

        forward.push(WorkProductCoverageJudgment {
            occurrence_ref: occurrence.occurrence_ref.clone(),
            work_product_ref: occurrence.work_product_ref.clone(),
            product_proposition_ref: occurrence.product_proposition_ref.clone(),
            matter_proposition_ref,
            relation,
            status,
            work_product_source_refs: occurrence.source_chain(),
            matter_ancestry_refs: ancestry,
            comparison_receipt_refs: comparison_refs,
            review_ref,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        });
    }
    forward.sort_by(|left, right| left.occurrence_ref.cmp(&right.occurrence_ref));

    let mut seen_expectations = BTreeSet::new();
    let mut reverse = Vec::new();
    for expectation in expectations {
        expectation.validate()?;
        if !seen_expectations.insert(expectation.matter_proposition_ref.clone()) {
            return Err(WorkProductCoverageError::DuplicateExpectation(
                expectation.matter_proposition_ref.clone(),
            ));
        }
        let view = proposition_views
            .get(&expectation.matter_proposition_ref)
            .ok_or_else(|| {
                WorkProductCoverageError::UnknownMatterProposition(
                    expectation.matter_proposition_ref.clone(),
                )
            })?;
        let ancestry = matter_ancestry(view, &matter.source_traces);
        if ancestry.iter().all(|value| {
            value == &view.root.proposition_ref
                || view.leaves.iter().any(|leaf| &leaf.claim_ref == value)
                || view
                    .relations
                    .iter()
                    .any(|item| &item.relation_ref == value)
        }) {
            return Err(WorkProductCoverageError::MissingMatterAncestry(
                expectation.matter_proposition_ref.clone(),
            ));
        }

        let mut resolving_occurrence_refs = forward
            .iter()
            .filter(|judgment| {
                judgment.matter_proposition_ref.as_deref()
                    == Some(expectation.matter_proposition_ref.as_str())
                    && matches!(
                        judgment.status,
                        ForwardCoverageStatus::Supported
                            | ForwardCoverageStatus::Qualified
                            | ForwardCoverageStatus::Contradicted
                    )
            })
            .map(|judgment| judgment.occurrence_ref.clone())
            .collect::<Vec<_>>();
        resolving_occurrence_refs.sort();
        resolving_occurrence_refs.dedup();

        let status = match expectation.kind {
            MatterWorkProductExpectationKind::ExcludedByScope => {
                ReverseCoverageStatus::ExcludedByScope
            }
            MatterWorkProductExpectationKind::NotExpectedForProduct => {
                ReverseCoverageStatus::NotExpectedForProduct
            }
            MatterWorkProductExpectationKind::ExpectedInProduct => {
                if !matter_is_reviewed(view) {
                    ReverseCoverageStatus::UnreviewedForOmission
                } else if resolving_occurrence_refs.is_empty() {
                    ReverseCoverageStatus::PossiblyOmitted
                } else {
                    ReverseCoverageStatus::Represented
                }
            }
        };

        reverse.push(MatterOmissionJudgment {
            matter_proposition_ref: expectation.matter_proposition_ref.clone(),
            status,
            expectation_basis_ref: expectation.basis_ref.clone(),
            resolving_occurrence_refs,
            matter_ancestry_refs: ancestry,
            review_ref: expectation.review_ref.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        });
    }
    reverse.sort_by(|left, right| {
        left.matter_proposition_ref
            .cmp(&right.matter_proposition_ref)
    });

    Ok(WorkProductCoverageProjection {
        matter_ref: matter.matter_ref.clone(),
        forward,
        reverse,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        canonical_world_mutated: false,
        supported_implies_legal_sufficiency: false,
        contradicted_implies_false: false,
        unsupported_implies_false: false,
        omission_implies_should_include: false,
        redaction_deletes_canonical_source: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::{
        chronology_contestation::{
            ClaimLeaf, ClaimLeafKind, ContestationRelation, ContestationRelationKind,
            PropositionRoot,
        },
        matter_context::MatterContextProjection,
    };

    use crate::{
        ChronologyProjection, EventDiscoveryProjection, OperationalOutstandingProjection,
        OperationalTimelineProjection, ReviewQueueProjection, SemanticTracePath,
        StatementTraceCoordinate,
    };

    fn occurrence(reference: &str) -> WorkProductPropositionOccurrence {
        WorkProductPropositionOccurrence {
            occurrence_ref: reference.into(),
            work_product_ref: "work-product:draft-1".into(),
            statement_ref: format!("wp-statement:{reference}"),
            source_revision_ref: "wp-revision:1".into(),
            exact_span_ref: format!("wp-span:{reference}"),
            product_proposition_ref: format!("wp-proposition:{reference}"),
            candidate_search_receipt_ref: format!("search:{reference}"),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn root(reference: &str) -> PropositionRoot {
        PropositionRoot {
            proposition_ref: reference.into(),
            label: format!("label:{reference}"),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn claim(
        claim_ref: &str,
        proposition_ref: &str,
        statement_ref: &str,
        review_state: ClaimReviewState,
    ) -> ClaimLeaf {
        ClaimLeaf {
            claim_ref: claim_ref.into(),
            proposition_ref: proposition_ref.into(),
            kind: ClaimLeafKind::Affirmation,
            speaker_ref: Some("actor:1".into()),
            statement_refs: vec![statement_ref.into()],
            observation_refs: vec![format!("observation:{claim_ref}")],
            temporal_refs: vec![],
            scope_refs: vec!["matter:1".into()],
            review_state,
            review_ref: (!matches!(review_state, ClaimReviewState::Unreviewed))
                .then(|| format!("review:{claim_ref}")),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn trace(statement_ref: &str, claim_ref: &str) -> SemanticTracePath {
        SemanticTracePath {
            focus_ref: claim_ref.into(),
            statement: StatementTraceCoordinate {
                statement_ref: statement_ref.into(),
                document_ref: format!("document:{statement_ref}"),
                source_revision_ref: format!("revision:{statement_ref}"),
                span_ref: format!("span:{statement_ref}"),
                literal_text: format!("text:{statement_ref}"),
            },
            parse: None,
            observation_ref: format!("observation:{claim_ref}"),
            event_ref: Some("event:1".into()),
            claim_refs: vec![claim_ref.into()],
            downstream_use_refs: vec![],
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn matter() -> MatterWorkspaceProjection {
        let reviewed_root = root("proposition:reviewed");
        let unreviewed_root = root("proposition:unreviewed");
        let reviewed_claim = claim(
            "claim:reviewed",
            &reviewed_root.proposition_ref,
            "statement:reviewed",
            ClaimReviewState::Accepted,
        );
        let unreviewed_claim = claim(
            "claim:unreviewed",
            &unreviewed_root.proposition_ref,
            "statement:unreviewed",
            ClaimReviewState::Unreviewed,
        );
        let relation = ContestationRelation {
            relation_ref: "relation:reviewed".into(),
            from_claim_ref: reviewed_claim.claim_ref.clone(),
            to_claim_ref: "claim:other".into(),
            kind: ContestationRelationKind::Contradicts,
            statement_refs: vec![reviewed_claim.statement_refs[0].clone()],
            observation_refs: vec![],
            review_ref: Some("review:relation".into()),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };

        MatterWorkspaceProjection {
            matter_ref: "matter:1".into(),
            context_projection: MatterContextProjection {
                matter_ref: "matter:1".into(),
                included_refs: vec![],
                exclusions: vec![],
                canonical_world_mutated: false,
                invisibility_means_false: false,
                unshared_means_absent: false,
                role_visibility_creates_truth: false,
                later_knowledge_rewrites_cut: false,
            },
            source_traces: vec![
                trace("statement:reviewed", "claim:reviewed"),
                trace("statement:unreviewed", "claim:unreviewed"),
            ],
            event_timeline: ChronologyProjection {
                entries: vec![],
                proposition_views: vec![
                    PropositionContestationView {
                        root: reviewed_root,
                        leaves: vec![reviewed_claim],
                        relations: vec![relation],
                        orphan_relation_refs: vec![],
                        candidate_only: true,
                        creates_semantic_authority: false,
                        claim_truth_promoted: false,
                    },
                    PropositionContestationView {
                        root: unreviewed_root,
                        leaves: vec![unreviewed_claim],
                        relations: vec![],
                        orphan_relation_refs: vec![],
                        candidate_only: true,
                        creates_semantic_authority: false,
                        claim_truth_promoted: false,
                    },
                ],
                candidate_only: true,
                creates_semantic_authority: false,
                claim_truth_promoted: false,
            },
            knowledge_timeline: vec![],
            operational_timeline: OperationalTimelineProjection::default(),
            operational_outstanding: OperationalOutstandingProjection::default(),
            join_proposals: EventDiscoveryProjection::default(),
            review_queue: ReviewQueueProjection {
                items: vec![],
                count_by_kind: BTreeMap::new(),
                count_by_status: BTreeMap::new(),
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
            legal_proof_refs: vec![],
            research_refs: vec![],
            work_product_refs: vec!["work-product:draft-1".into()],
            handoff_refs: vec![],
            creates_semantic_authority: false,
            claim_truth_promoted: false,
            canonical_world_mutated: false,
        }
    }

    fn relation_match(
        occurrence_ref: &str,
        proposition_ref: &str,
        relation: WorkProductMatterRelation,
    ) -> WorkProductMatterMatch {
        WorkProductMatterMatch {
            occurrence_ref: occurrence_ref.into(),
            matter_proposition_ref: proposition_ref.into(),
            relation,
            comparison_receipt_ref: format!("comparison:{occurrence_ref}"),
            review_ref: Some(format!("review:{occurrence_ref}")),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn expectation(
        proposition_ref: &str,
        kind: MatterWorkProductExpectationKind,
    ) -> MatterWorkProductExpectation {
        MatterWorkProductExpectation {
            matter_proposition_ref: proposition_ref.into(),
            kind,
            basis_ref: format!("scope:{proposition_ref}"),
            review_ref: None,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn historical_relation_algebra_keeps_non_resolving_and_truth_separate() {
        assert_eq!(
            WorkProductMatterRelation::ExactSupport.root(),
            WorkProductRelationRoot::Supports
        );
        assert_eq!(
            WorkProductMatterRelation::ExplicitDispute.root(),
            WorkProductRelationRoot::Invalidates
        );
        assert_eq!(
            WorkProductMatterRelation::AdjacentEvent.root(),
            WorkProductRelationRoot::NonResolving
        );
        assert_eq!(
            WorkProductMatterRelation::Unrelated.root(),
            WorkProductRelationRoot::Unanswered
        );
        assert!(!WorkProductMatterRelation::AdjacentEvent.resolves_coverage());
        assert!(!WorkProductMatterRelation::Unrelated.resolves_coverage());
    }

    #[test]
    fn forward_and_reverse_are_distinct_and_dually_reopenable() {
        let matter = matter();
        let occurrences = vec![
            occurrence("occ:supported"),
            occurrence("occ:contradicted"),
            occurrence("occ:unsupported"),
            occurrence("occ:unreviewed"),
        ];
        let matches = vec![
            relation_match(
                "occ:supported",
                "proposition:reviewed",
                WorkProductMatterRelation::ExactSupport,
            ),
            relation_match(
                "occ:contradicted",
                "proposition:reviewed",
                WorkProductMatterRelation::ExplicitDispute,
            ),
            relation_match(
                "occ:unreviewed",
                "proposition:unreviewed",
                WorkProductMatterRelation::ExactSupport,
            ),
        ];
        let expectations = vec![
            expectation(
                "proposition:reviewed",
                MatterWorkProductExpectationKind::ExpectedInProduct,
            ),
            expectation(
                "proposition:unreviewed",
                MatterWorkProductExpectationKind::ExpectedInProduct,
            ),
        ];

        let projection =
            project_work_product_coverage(&matter, &occurrences, &matches, &expectations).unwrap();

        let by_occurrence = projection
            .forward
            .iter()
            .map(|item| (item.occurrence_ref.as_str(), item))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(
            by_occurrence["occ:supported"].status,
            ForwardCoverageStatus::Supported
        );
        assert_eq!(
            by_occurrence["occ:contradicted"].status,
            ForwardCoverageStatus::Contradicted
        );
        assert_eq!(
            by_occurrence["occ:unsupported"].status,
            ForwardCoverageStatus::Unsupported
        );
        assert_eq!(
            by_occurrence["occ:unreviewed"].status,
            ForwardCoverageStatus::Unreviewed
        );

        assert_eq!(
            by_occurrence["occ:unsupported"].matter_proposition_ref,
            None
        );
        assert!(by_occurrence["occ:unsupported"].matter_ancestry_refs.is_empty());
        assert_eq!(
            by_occurrence["occ:unsupported"].comparison_receipt_refs,
            vec!["search:occ:unsupported"]
        );

        let supported = by_occurrence["occ:supported"];
        assert!(supported
            .work_product_source_refs
            .contains(&"wp-span:occ:supported".to_owned()));
        assert!(supported
            .matter_ancestry_refs
            .contains(&"span:statement:reviewed".to_owned()));

        let reverse = projection
            .reverse
            .iter()
            .map(|item| (item.matter_proposition_ref.as_str(), item))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            reverse["proposition:reviewed"].status,
            ReverseCoverageStatus::Represented
        );
        assert_eq!(
            reverse["proposition:unreviewed"].status,
            ReverseCoverageStatus::UnreviewedForOmission
        );

        assert!(!projection.creates_semantic_authority);
        assert!(!projection.applicability_promoted);
        assert!(!projection.claim_truth_promoted);
        assert!(!projection.canonical_world_mutated);
        assert!(!projection.supported_implies_legal_sufficiency);
        assert!(!projection.contradicted_implies_false);
        assert!(!projection.unsupported_implies_false);
        assert!(!projection.omission_implies_should_include);
        assert!(!projection.redaction_deletes_canonical_source);
    }

    #[test]
    fn omission_only_appears_in_reverse_expected_reviewed_direction() {
        let matter = matter();
        let projection = project_work_product_coverage(
            &matter,
            &[],
            &[],
            &[
                expectation(
                    "proposition:reviewed",
                    MatterWorkProductExpectationKind::ExpectedInProduct,
                ),
                expectation(
                    "proposition:unreviewed",
                    MatterWorkProductExpectationKind::ExcludedByScope,
                ),
            ],
        )
        .unwrap();

        assert_eq!(
            projection.reverse[0].status,
            ReverseCoverageStatus::PossiblyOmitted
        );
        assert_eq!(
            projection.reverse[1].status,
            ReverseCoverageStatus::ExcludedByScope
        );
        assert!(!projection.omission_implies_should_include);
    }

    #[test]
    fn multiple_matter_matches_fail_closed_instead_of_silently_selecting() {
        let matter = matter();
        let occurrence = occurrence("occ:ambiguous");
        let error = project_work_product_coverage(
            &matter,
            &[occurrence],
            &[
                relation_match(
                    "occ:ambiguous",
                    "proposition:reviewed",
                    WorkProductMatterRelation::ExactSupport,
                ),
                relation_match(
                    "occ:ambiguous",
                    "proposition:unreviewed",
                    WorkProductMatterRelation::PartialOverlap,
                ),
            ],
            &[],
        )
        .unwrap_err();

        assert_eq!(
            error,
            WorkProductCoverageError::DuplicateMatch("occ:ambiguous".into())
        );
    }
}
