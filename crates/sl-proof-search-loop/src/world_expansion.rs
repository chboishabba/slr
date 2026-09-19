//! Residual-driven world expansion for bounded knowledge discovery.
//!
//! This module does not acquire sources, parse PNF, review evidence, persist
//! relations, or grant semantic/legal authority. It only ranks already-typed
//! candidate producer moves for one residual and accounts for explicitly
//! reviewed, disambiguated, novel world-object admissions.

use crate::frontier::ProofResidual;
use crate::world_identity::{WorldIdentityResolutionKind, WorldIdentityResolutionReceipt, WorldObjectIdentity};
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResidualClass { Legal, Identity, Context, Provenance, Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProducerLane { GovernedLegal, WikidataIdentity, WikipediaContext, SourceSpecificProvenance, Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeObjectKind { Qid, Article, PrimaryLegalSource, Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisambiguationOutcome {
    SameObject, NewRelatedObject, NewSourceManifestation, NewConceptualParent,
    NewEvidentiarySource, Ambiguous, WrongType, Duplicate, IrrelevantToResidual,
}

impl DisambiguationOutcome {
    const fn admission_capable(self) -> bool {
        matches!(self, Self::SameObject | Self::NewRelatedObject | Self::NewSourceManifestation | Self::NewConceptualParent | Self::NewEvidentiarySource)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewDecision { Reviewed, Rejected }

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
pub struct WorldExpansionPolicy { pub target_novel_objects: usize, pub minimum_expected_residual_contraction: u64 }

impl WorldExpansionPolicy {
    #[must_use]
    pub fn complete(self, ledger: &WorldExpansionLedger) -> bool { ledger.total_new_world_objects >= self.target_novel_objects }
}

#[must_use]
pub const fn mabo_world_expansion_policy() -> WorldExpansionPolicy {
    WorldExpansionPolicy { target_novel_objects: 100, minimum_expected_residual_contraction: 1 }
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
    admitted_identity_class_refs: BTreeSet<String>,
}

impl WorldExpansionLedger {
    #[must_use]
    pub fn contains_object(&self, object_ref: &str) -> bool { self.admitted_identity_class_refs.contains(object_ref) }

    #[must_use]
    pub fn contains_identity_class(&self, identity_class_ref: &str) -> bool {
        self.admitted_identity_class_refs.contains(identity_class_ref)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionReceipt {
    pub candidate_ref: String,
    pub object_ref: String,
    pub identity_class_ref: String,
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
    right.expected_residual_contraction.cmp(&left.expected_residual_contraction)
        .then_with(|| domain_fit(right.residual_class, right.producer_lane).cmp(&domain_fit(left.residual_class, left.producer_lane)))
        .then_with(|| right.provenance_quality.cmp(&left.provenance_quality))
        .then_with(|| right.same_object_confidence.cmp(&left.same_object_confidence))
        .then_with(|| right.expected_new_world_value.cmp(&left.expected_new_world_value))
        .then_with(|| left.acquisition_cost.cmp(&right.acquisition_cost))
        .then_with(|| left.candidate_ref.cmp(&right.candidate_ref))
}

fn select_from<'a>(candidates: impl Iterator<Item = &'a ExpansionCandidate>, minimum_expected_residual_contraction: u64) -> Option<&'a ExpansionCandidate> {
    let mut eligible: Vec<&ExpansionCandidate> = candidates
        .filter(|candidate| candidate.admissible)
        .filter(|candidate| candidate.expected_residual_contraction > 0)
        .filter(|candidate| candidate.expected_residual_contraction >= minimum_expected_residual_contraction)
        .collect();
    eligible.sort_by(|left, right| candidate_order(left, right));
    eligible.first().copied()
}

#[must_use]
pub fn select_expansion_candidate(candidates: &[ExpansionCandidate], minimum_expected_residual_contraction: u64) -> Option<&ExpansionCandidate> {
    select_from(candidates.iter(), minimum_expected_residual_contraction)
}

#[must_use]
pub fn select_for_proof_residual<'a>(residual: &ProofResidual, residual_class: ResidualClass, candidates: &'a [ExpansionCandidate], minimum_expected_residual_contraction: u64) -> Option<&'a ExpansionCandidate> {
    select_from(candidates.iter().filter(|candidate| candidate.triggering_residual_ref == residual.residual_ref && candidate.residual_class == residual_class), minimum_expected_residual_contraction)
}

fn count_new_identity(
    ledger: &mut WorldExpansionLedger,
    candidate: &ExpansionCandidate,
    identity_class_ref: &str,
) -> bool {
    if ledger.admitted_identity_class_refs.contains(identity_class_ref) {
        ledger.duplicates_seen += 1;
        return false;
    }
    ledger.admitted_identity_class_refs.insert(identity_class_ref.to_owned());
    ledger.total_new_world_objects += 1;
    match candidate.object_kind {
        KnowledgeObjectKind::Qid => ledger.new_qids_admitted += 1,
        KnowledgeObjectKind::Article => ledger.new_articles_admitted += 1,
        KnowledgeObjectKind::PrimaryLegalSource => ledger.new_primary_legal_sources_admitted += 1,
        KnowledgeObjectKind::Other => ledger.new_other_world_objects_admitted += 1,
    }
    true
}

/// Identity-aware reviewed admission. The candidate representation must occur in
/// the explicit identity-resolution receipt; novelty is counted by the reviewed
/// world identity class, never by the presentation string alone.
pub fn review_admission_with_identity(
    ledger: &mut WorldExpansionLedger,
    candidate: &ExpansionCandidate,
    review_decision: ReviewDecision,
    disambiguation_outcome: DisambiguationOutcome,
    identity_resolution: &WorldIdentityResolutionReceipt,
) -> AdmissionReceipt {
    ledger.candidates_seen += 1;
    let identity_class_ref = identity_resolution.identity_class_ref().to_owned();
    let identity_usable = identity_resolution.candidate_only
        && !identity_resolution.creates_semantic_authority
        && !identity_resolution.applicability_promoted
        && !identity_resolution.claim_truth_promoted
        && identity_resolution.identity.contains_representation(&candidate.object_ref)
        && !matches!(identity_resolution.resolution_kind, WorldIdentityResolutionKind::Ambiguous | WorldIdentityResolutionKind::WrongType | WorldIdentityResolutionKind::Unresolved);

    let mut admitted = false;
    if review_decision == ReviewDecision::Rejected || !identity_usable {
        ledger.candidates_rejected += 1;
    } else {
        ledger.reviewed_objects += 1;
        match disambiguation_outcome {
            DisambiguationOutcome::Ambiguous => ledger.identity_ambiguous += 1,
            DisambiguationOutcome::WrongType | DisambiguationOutcome::IrrelevantToResidual => ledger.candidates_rejected += 1,
            DisambiguationOutcome::Duplicate => ledger.duplicates_seen += 1,
            outcome if outcome.admission_capable() => admitted = count_new_identity(ledger, candidate, &identity_class_ref),
            _ => {}
        }
    }

    AdmissionReceipt {
        candidate_ref: candidate.candidate_ref.clone(),
        object_ref: candidate.object_ref.clone(),
        identity_class_ref,
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

/// Compatibility path for already-paid callers whose reviewed identity class is
/// exactly the candidate representation string. P7d.5 should prefer
/// `review_admission_with_identity` when aliases/sitelinks/provider IDs exist.
pub fn review_admission(
    ledger: &mut WorldExpansionLedger,
    candidate: &ExpansionCandidate,
    review_decision: ReviewDecision,
    disambiguation_outcome: DisambiguationOutcome,
) -> AdmissionReceipt {
    let identity = WorldIdentityResolutionReceipt::same_object(
        format!("identity-resolution:compat:{}", candidate.candidate_ref),
        WorldObjectIdentity::new(candidate.object_ref.clone(), candidate.object_ref.clone()),
        "compat:representation-equality",
    );
    review_admission_with_identity(ledger, candidate, review_decision, disambiguation_outcome, &identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontier::{ProofResidual, ResidualStatus};

    fn candidate(candidate_ref: &str, object_ref: &str, residual_class: ResidualClass, producer_lane: ProducerLane, contraction: u64) -> ExpansionCandidate {
        ExpansionCandidate {
            candidate_ref: candidate_ref.into(), object_ref: object_ref.into(), object_kind: KnowledgeObjectKind::Qid,
            discovery_parent_ref: "Q1501525".into(), triggering_residual_ref: "residual:mabo:test".into(),
            residual_class, producer_lane, source_revision_ref: None, expected_residual_contraction: contraction,
            provenance_quality: 5, same_object_confidence: 5, expected_new_world_value: 5, acquisition_cost: 1, admissible: true,
        }
    }

    fn proof_residual() -> ProofResidual {
        ProofResidual { residual_ref: "residual:mabo:test".into(), proposition_ref: "mabo:proposition:radical-title-native-title".into(), producer_class_ref: "producer:world-expansion".into(), jurisdiction_ref: Some("AU".into()), authority_requirement_ref: Some("primary-case".into()), salience: 100, dependency_refs: Vec::new(), status: ResidualStatus::Open }
    }

    #[test]
    fn legal_residual_prefers_legal_lane_when_contraction_is_equal() {
        let legal = candidate("legal", "source:mabo:hca:1992:23", ResidualClass::Legal, ProducerLane::GovernedLegal, 3);
        let wiki = candidate("wiki", "wiki:Mabo_v_Queensland_(No_2)", ResidualClass::Legal, ProducerLane::WikipediaContext, 3);
        let candidates = [wiki, legal];
        assert_eq!(select_expansion_candidate(&candidates, 1).unwrap().candidate_ref, "legal");
    }

    #[test]
    fn higher_contraction_beats_domain_prior() {
        let legal = candidate("legal", "source:mabo:hca:1992:23", ResidualClass::Legal, ProducerLane::GovernedLegal, 2);
        let identity = candidate("identity", "Q975866", ResidualClass::Legal, ProducerLane::WikidataIdentity, 4);
        let candidates = [legal, identity];
        assert_eq!(select_expansion_candidate(&candidates, 1).unwrap().candidate_ref, "identity");
    }

    #[test]
    fn exact_open_proof_residual_binds_candidate_selection() {
        let matching = candidate("matching", "source:mabo:hca:1992:23", ResidualClass::Legal, ProducerLane::GovernedLegal, 2);
        let mut other = candidate("other", "Q975866", ResidualClass::Legal, ProducerLane::WikidataIdentity, 99);
        other.triggering_residual_ref = "residual:other".into();
        let candidates = [other, matching];
        assert_eq!(select_for_proof_residual(&proof_residual(), ResidualClass::Legal, &candidates, 1).unwrap().candidate_ref, "matching");
    }

    #[test]
    fn identity_residual_prefers_wikidata_on_equal_contraction() {
        let candidates = [candidate("wiki", "wiki:Eddie_Mabo", ResidualClass::Identity, ProducerLane::WikipediaContext, 2), candidate("qid", "Q975866", ResidualClass::Identity, ProducerLane::WikidataIdentity, 2)];
        assert_eq!(select_expansion_candidate(&candidates, 1).unwrap().candidate_ref, "qid");
    }

    #[test]
    fn context_residual_prefers_wikipedia_on_equal_contraction() {
        let candidates = [candidate("qid", "Q975866", ResidualClass::Context, ProducerLane::WikidataIdentity, 2), candidate("wiki", "wiki:Eddie_Mabo", ResidualClass::Context, ProducerLane::WikipediaContext, 2)];
        assert_eq!(select_expansion_candidate(&candidates, 1).unwrap().candidate_ref, "wiki");
    }

    #[test]
    fn ambiguous_wrong_type_duplicate_and_irrelevant_do_not_count() {
        let mut ledger = WorldExpansionLedger::default();
        for outcome in [DisambiguationOutcome::Ambiguous, DisambiguationOutcome::WrongType, DisambiguationOutcome::Duplicate, DisambiguationOutcome::IrrelevantToResidual] {
            assert!(!review_admission(&mut ledger, &candidate("c", "Q1", ResidualClass::Identity, ProducerLane::WikidataIdentity, 1), ReviewDecision::Reviewed, outcome).admitted);
        }
        assert_eq!(ledger.total_new_world_objects, 0);
    }

    #[test]
    fn duplicate_object_identity_counts_once() {
        let mut ledger = WorldExpansionLedger::default();
        let first = candidate("first", "Q975866", ResidualClass::Identity, ProducerLane::WikidataIdentity, 1);
        let second = candidate("second", "Q975866", ResidualClass::Context, ProducerLane::WikipediaContext, 1);
        assert!(review_admission(&mut ledger, &first, ReviewDecision::Reviewed, DisambiguationOutcome::NewRelatedObject).admitted);
        assert!(!review_admission(&mut ledger, &second, ReviewDecision::Reviewed, DisambiguationOutcome::SameObject).admitted);
        assert_eq!(ledger.total_new_world_objects, 1);
        assert_eq!(ledger.duplicates_seen, 1);
    }

    #[test]
    fn qid_and_wikipedia_alias_count_as_one_reviewed_identity_class() {
        let mut ledger = WorldExpansionLedger::default();
        let qid = candidate("qid", "Q975866", ResidualClass::Identity, ProducerLane::WikidataIdentity, 1);
        let mut article = candidate("article", "https://en.wikipedia.org/wiki/Eddie_Mabo", ResidualClass::Context, ProducerLane::WikipediaContext, 1);
        article.object_kind = KnowledgeObjectKind::Article;
        let identity = WorldIdentityResolutionReceipt::same_object(
            "identity-resolution:mabo:eddie",
            WorldObjectIdentity::new("world-object:eddie-mabo", "Q975866").with_alias("https://en.wikipedia.org/wiki/Eddie_Mabo"),
            "reviewed-qid-sitelink",
        );
        assert!(review_admission_with_identity(&mut ledger, &qid, ReviewDecision::Reviewed, DisambiguationOutcome::NewRelatedObject, &identity).admitted);
        assert!(!review_admission_with_identity(&mut ledger, &article, ReviewDecision::Reviewed, DisambiguationOutcome::SameObject, &identity).admitted);
        assert_eq!(ledger.total_new_world_objects, 1);
        assert!(ledger.contains_identity_class("world-object:eddie-mabo"));
    }

    #[test]
    fn target_is_object_cardinality_not_depth() {
        let policy = mabo_world_expansion_policy();
        assert_eq!(policy.target_novel_objects, 100);
        let mut ledger = WorldExpansionLedger::default();
        for index in 0..100 {
            let row = candidate(&format!("c:{index}"), &format!("Q{index}"), ResidualClass::Identity, ProducerLane::WikidataIdentity, 1);
            review_admission(&mut ledger, &row, ReviewDecision::Reviewed, DisambiguationOutcome::NewRelatedObject);
        }
        assert!(policy.complete(&ledger));
        assert_eq!(ledger.total_new_world_objects, 100);
    }

    #[test]
    fn ninety_nine_objects_is_not_complete() {
        let policy = mabo_world_expansion_policy();
        let mut ledger = WorldExpansionLedger::default();
        for index in 0..99 {
            let row = candidate(&format!("c:{index}"), &format!("Q{index}"), ResidualClass::Identity, ProducerLane::WikidataIdentity, 1);
            review_admission(&mut ledger, &row, ReviewDecision::Reviewed, DisambiguationOutcome::NewRelatedObject);
        }
        assert!(!policy.complete(&ledger));
    }
}
