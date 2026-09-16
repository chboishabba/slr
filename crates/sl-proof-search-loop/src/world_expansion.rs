//! Residual-driven world expansion for bounded knowledge discovery.
//!
//! This module does not acquire sources, parse PNF, review evidence, persist
//! relations, or grant semantic/legal authority. It only ranks already-typed
//! candidate producer moves for one residual and accounts for explicitly
//! reviewed, disambiguated, novel world-object admissions.

use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResidualClass {
    Legal,
    Identity,
    Context,
    Provenance,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProducerLane {
    GovernedLegal,
    WikidataIdentity,
    WikipediaContext,
    SourceSpecificProvenance,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeObjectKind {
    Qid,
    Article,
    PrimaryLegalSource,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisambiguationOutcome {
    SameObject,
    NewRelatedObject,
    NewSourceManifestation,
    NewConceptualParent,
    NewEvidentiarySource,
    Ambiguous,
    WrongType,
    Duplicate,
    IrrelevantToResidual,
}

impl DisambiguationOutcome {
    const fn admission_capable(self) -> bool {
        matches!(
            self,
            Self::SameObject
                | Self::NewRelatedObject
                | Self::NewSourceManifestation
                | Self::NewConceptualParent
                | Self::NewEvidentiarySource
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewDecision {
    Reviewed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpansionCandidate {
    pub candidate_ref: String,
    pub object_ref: String,
    pub object_kind: KnowledgeObjectKind,
    pub discovery_parent_ref: String,
    pub triggering_residual_ref: String,
    pub residual_class: ResidualClass,
    pub producer_lane: ProducerLane,
    pub source_revision_ref: Option<String>,
    pub expected_residual_contraction: u64,
    pub provenance_quality: u64,
    pub same_object_confidence: u64,
    pub expected_new_world_value: u64,
    pub acquisition_cost: u64,
    pub admissible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldExpansionPolicy {
    pub target_novel_objects: usize,
    pub minimum_expected_residual_contraction: u64,
}

impl WorldExpansionPolicy {
    #[must_use]
    pub fn complete(self, ledger: &WorldExpansionLedger) -> bool {
        ledger.total_new_world_objects >= self.target_novel_objects
    }
}

#[must_use]
pub const fn mabo_world_expansion_policy() -> WorldExpansionPolicy {
    WorldExpansionPolicy {
        target_novel_objects: 100,
        minimum_expected_residual_contraction: 1,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldExpansionLedger {
    pub candidates_seen: usize,
    pub candidates_rejected: usize,
    pub duplicates_seen: usize,
    pub identity_ambiguous: usize,
    pub reviewed_objects: usize,
    pub new_qids_admitted: usize,
    pub new_articles_admitted: usize,
    pub new_primary_legal_sources_admitted: usize,
    pub new_other_world_objects_admitted: usize,
    pub total_new_world_objects: usize,
    admitted_object_refs: BTreeSet<String>,
}

impl WorldExpansionLedger {
    #[must_use]
    pub fn contains_object(&self, object_ref: &str) -> bool {
        self.admitted_object_refs.contains(object_ref)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionReceipt {
    pub candidate_ref: String,
    pub object_ref: String,
    pub triggering_residual_ref: String,
    pub producer_lane: ProducerLane,
    pub disambiguation_outcome: DisambiguationOutcome,
    pub review_decision: ReviewDecision,
    pub admitted: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

const fn domain_fit(residual_class: ResidualClass, producer_lane: ProducerLane) -> u8 {
    match (residual_class, producer_lane) {
        (ResidualClass::Legal, ProducerLane::GovernedLegal)
        | (ResidualClass::Identity, ProducerLane::WikidataIdentity)
        | (ResidualClass::Context, ProducerLane::WikipediaContext)
        | (ResidualClass::Provenance, ProducerLane::SourceSpecificProvenance) => 1,
        _ => 0,
    }
}

fn candidate_order(left: &ExpansionCandidate, right: &ExpansionCandidate) -> Ordering {
    right
        .expected_residual_contraction
        .cmp(&left.expected_residual_contraction)
        .then_with(|| {
            domain_fit(right.residual_class, right.producer_lane)
                .cmp(&domain_fit(left.residual_class, left.producer_lane))
        })
        .then_with(|| right.provenance_quality.cmp(&left.provenance_quality))
        .then_with(|| right.same_object_confidence.cmp(&left.same_object_confidence))
        .then_with(|| right.expected_new_world_value.cmp(&left.expected_new_world_value))
        .then_with(|| left.acquisition_cost.cmp(&right.acquisition_cost))
        .then_with(|| left.candidate_ref.cmp(&right.candidate_ref))
}

/// Select one already-declared acquisition/discovery candidate for a residual.
/// Expected residual contraction is primary. Residual-domain producer fit is a
/// tie-break coordinate, so legal-first is legal-residual-first rather than a
/// global OALC > Wikidata > Wikipedia ordering.
#[must_use]
pub fn select_expansion_candidate(
    candidates: &[ExpansionCandidate],
    minimum_expected_residual_contraction: u64,
) -> Option<&ExpansionCandidate> {
    let mut eligible: Vec<&ExpansionCandidate> = candidates
        .iter()
        .filter(|candidate| candidate.admissible)
        .filter(|candidate| candidate.expected_residual_contraction > 0)
        .filter(|candidate| {
            candidate.expected_residual_contraction >= minimum_expected_residual_contraction
        })
        .collect();
    eligible.sort_by(|left, right| candidate_order(left, right));
    eligible.first().copied()
}

/// Account for an explicit review/disambiguation result. Reachability never
/// counts as admission, and duplicate canonical object identities count once.
pub fn review_admission(
    ledger: &mut WorldExpansionLedger,
    candidate: &ExpansionCandidate,
    review_decision: ReviewDecision,
    disambiguation_outcome: DisambiguationOutcome,
) -> AdmissionReceipt {
    ledger.candidates_seen += 1;

    let mut admitted = false;
    if review_decision == ReviewDecision::Rejected {
        ledger.candidates_rejected += 1;
    } else {
        ledger.reviewed_objects += 1;
        match disambiguation_outcome {
            DisambiguationOutcome::Ambiguous => ledger.identity_ambiguous += 1,
            DisambiguationOutcome::WrongType | DisambiguationOutcome::IrrelevantToResidual => {
                ledger.candidates_rejected += 1;
            }
            DisambiguationOutcome::Duplicate => ledger.duplicates_seen += 1,
            outcome if outcome.admission_capable() => {
                if ledger.admitted_object_refs.contains(&candidate.object_ref) {
                    ledger.duplicates_seen += 1;
                } else {
                    ledger.admitted_object_refs.insert(candidate.object_ref.clone());
                    ledger.total_new_world_objects += 1;
                    match candidate.object_kind {
                        KnowledgeObjectKind::Qid => ledger.new_qids_admitted += 1,
                        KnowledgeObjectKind::Article => ledger.new_articles_admitted += 1,
                        KnowledgeObjectKind::PrimaryLegalSource => {
                            ledger.new_primary_legal_sources_admitted += 1;
                        }
                        KnowledgeObjectKind::Other => {
                            ledger.new_other_world_objects_admitted += 1;
                        }
                    }
                    admitted = true;
                }
            }
            _ => {}
        }
    }

    AdmissionReceipt {
        candidate_ref: candidate.candidate_ref.clone(),
        object_ref: candidate.object_ref.clone(),
        triggering_residual_ref: candidate.triggering_residual_ref.clone(),
        producer_lane: candidate.producer_lane,
        disambiguation_outcome,
        review_decision,
        admitted,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        candidate_ref: &str,
        object_ref: &str,
        residual_class: ResidualClass,
        producer_lane: ProducerLane,
        contraction: u64,
    ) -> ExpansionCandidate {
        ExpansionCandidate {
            candidate_ref: candidate_ref.into(),
            object_ref: object_ref.into(),
            object_kind: KnowledgeObjectKind::Qid,
            discovery_parent_ref: "Q1501525".into(),
            triggering_residual_ref: "residual:mabo:test".into(),
            residual_class,
            producer_lane,
            source_revision_ref: None,
            expected_residual_contraction: contraction,
            provenance_quality: 5,
            same_object_confidence: 5,
            expected_new_world_value: 5,
            acquisition_cost: 1,
            admissible: true,
        }
    }

    #[test]
    fn legal_residual_prefers_legal_lane_when_contraction_is_equal() {
        let legal = candidate(
            "legal",
            "source:mabo:hca:1992:23",
            ResidualClass::Legal,
            ProducerLane::GovernedLegal,
            3,
        );
        let wiki = candidate(
            "wiki",
            "wiki:Mabo_v_Queensland_(No_2)",
            ResidualClass::Legal,
            ProducerLane::WikipediaContext,
            3,
        );
        let selected = select_expansion_candidate(&[wiki, legal], 1).unwrap();
        assert_eq!(selected.candidate_ref, "legal");
    }

    #[test]
    fn higher_contraction_beats_domain_prior() {
        let legal = candidate(
            "legal",
            "source:mabo:hca:1992:23",
            ResidualClass::Legal,
            ProducerLane::GovernedLegal,
            2,
        );
        let identity = candidate(
            "identity",
            "Q975866",
            ResidualClass::Legal,
            ProducerLane::WikidataIdentity,
            4,
        );
        let selected = select_expansion_candidate(&[legal, identity], 1).unwrap();
        assert_eq!(selected.candidate_ref, "identity");
    }

    #[test]
    fn identity_residual_prefers_wikidata_on_equal_contraction() {
        let wiki = candidate(
            "wiki",
            "wiki:Eddie_Mabo",
            ResidualClass::Identity,
            ProducerLane::WikipediaContext,
            2,
        );
        let qid = candidate(
            "qid",
            "Q975866",
            ResidualClass::Identity,
            ProducerLane::WikidataIdentity,
            2,
        );
        let selected = select_expansion_candidate(&[wiki, qid], 1).unwrap();
        assert_eq!(selected.candidate_ref, "qid");
    }

    #[test]
    fn context_residual_prefers_wikipedia_on_equal_contraction() {
        let qid = candidate(
            "qid",
            "Q975866",
            ResidualClass::Context,
            ProducerLane::WikidataIdentity,
            2,
        );
        let wiki = candidate(
            "wiki",
            "wiki:Eddie_Mabo",
            ResidualClass::Context,
            ProducerLane::WikipediaContext,
            2,
        );
        let selected = select_expansion_candidate(&[qid, wiki], 1).unwrap();
        assert_eq!(selected.candidate_ref, "wiki");
    }

    #[test]
    fn ambiguous_wrong_type_duplicate_and_irrelevant_do_not_count() {
        let mut ledger = WorldExpansionLedger::default();
        for outcome in [
            DisambiguationOutcome::Ambiguous,
            DisambiguationOutcome::WrongType,
            DisambiguationOutcome::Duplicate,
            DisambiguationOutcome::IrrelevantToResidual,
        ] {
            let receipt = review_admission(
                &mut ledger,
                &candidate(
                    "c",
                    "Q1",
                    ResidualClass::Identity,
                    ProducerLane::WikidataIdentity,
                    1,
                ),
                ReviewDecision::Reviewed,
                outcome,
            );
            assert!(!receipt.admitted);
        }
        assert_eq!(ledger.total_new_world_objects, 0);
    }

    #[test]
    fn duplicate_object_identity_counts_once() {
        let mut ledger = WorldExpansionLedger::default();
        let first = candidate(
            "first",
            "Q975866",
            ResidualClass::Identity,
            ProducerLane::WikidataIdentity,
            1,
        );
        let second = candidate(
            "second",
            "Q975866",
            ResidualClass::Context,
            ProducerLane::WikipediaContext,
            1,
        );
        assert!(review_admission(
            &mut ledger,
            &first,
            ReviewDecision::Reviewed,
            DisambiguationOutcome::NewRelatedObject,
        )
        .admitted);
        assert!(!review_admission(
            &mut ledger,
            &second,
            ReviewDecision::Reviewed,
            DisambiguationOutcome::SameObject,
        )
        .admitted);
        assert_eq!(ledger.total_new_world_objects, 1);
        assert_eq!(ledger.duplicates_seen, 1);
    }

    #[test]
    fn target_is_object_cardinality_not_depth() {
        let policy = mabo_world_expansion_policy();
        assert_eq!(policy.target_novel_objects, 100);
        let mut ledger = WorldExpansionLedger::default();
        for index in 0..100 {
            let row = candidate(
                &format!("c:{index}"),
                &format!("Q{index}"),
                ResidualClass::Identity,
                ProducerLane::WikidataIdentity,
                1,
            );
            review_admission(
                &mut ledger,
                &row,
                ReviewDecision::Reviewed,
                DisambiguationOutcome::NewRelatedObject,
            );
        }
        assert!(policy.complete(&ledger));
        assert_eq!(ledger.total_new_world_objects, 100);
    }

    #[test]
    fn ninety_nine_objects_is_not_complete() {
        let policy = mabo_world_expansion_policy();
        let mut ledger = WorldExpansionLedger::default();
        for index in 0..99 {
            let row = candidate(
                &format!("c:{index}"),
                &format!("Q{index}"),
                ResidualClass::Identity,
                ProducerLane::WikidataIdentity,
                1,
            );
            review_admission(
                &mut ledger,
                &row,
                ReviewDecision::Reviewed,
                DisambiguationOutcome::NewRelatedObject,
            );
        }
        assert!(!policy.complete(&ledger));
    }
}
