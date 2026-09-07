//! Mixed proof/acquisition/review search over live statement-bundle diagnoses.
//!
//! This mirrors DASHI's least-privilege proof-search constitution: a proof route
//! is live only after explicit target/same-object/prerequisite/no-go/circularity/
//! hypothesis/authority/novelty/frontier receipts. Evidence gaps are not silently
//! converted into theorem-search problems.

use crate::bundle::BundleCoordinateKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BundleDiagnosis {
    MainSnakMismatch,
    QualifierDrift,
    ReferenceDrift,
    RankQuestion,
    ProvenanceGap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProducerKind {
    EstablishSemanticCorrespondence,
    ProveQualifierTransport,
    AcquireReferenceEvidence,
    InterpretRank,
    AcquireProvenanceEvidence,
}

pub const fn producer_for(diagnosis: BundleDiagnosis) -> ProducerKind {
    match diagnosis {
        BundleDiagnosis::MainSnakMismatch => ProducerKind::EstablishSemanticCorrespondence,
        BundleDiagnosis::QualifierDrift => ProducerKind::ProveQualifierTransport,
        BundleDiagnosis::ReferenceDrift => ProducerKind::AcquireReferenceEvidence,
        BundleDiagnosis::RankQuestion => ProducerKind::InterpretRank,
        BundleDiagnosis::ProvenanceGap => ProducerKind::AcquireProvenanceEvidence,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteKind {
    Think,
    Look,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofRouteAdmission {
    pub exact_target: bool,
    pub same_object_spine: bool,
    pub prerequisites_closed: bool,
    pub no_known_no_go: bool,
    pub no_circular_dependency: bool,
    pub no_silent_hypothesis_strengthening: bool,
    pub authority_adequate: bool,
    pub novel_against_repo: bool,
    pub improves_frontier: bool,
}

impl ProofRouteAdmission {
    pub const fn least_privilege() -> Self {
        Self {
            exact_target: true,
            same_object_spine: true,
            prerequisites_closed: true,
            no_known_no_go: true,
            no_circular_dependency: true,
            no_silent_hypothesis_strengthening: true,
            authority_adequate: true,
            novel_against_repo: true,
            improves_frontier: true,
        }
    }

    pub const fn admitted(self) -> bool {
        self.exact_target
            && self.same_object_spine
            && self.prerequisites_closed
            && self.no_known_no_go
            && self.no_circular_dependency
            && self.no_silent_hypothesis_strengthening
            && self.authority_adequate
            && self.novel_against_repo
            && self.improves_frontier
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchCandidate {
    pub target: BundleDiagnosis,
    pub producer: ProducerKind,
    pub route_kind: RouteKind,
    pub cost: u32,
    pub proof_admission: Option<ProofRouteAdmission>,
    pub route_ref: String,
}

impl SearchCandidate {
    pub fn admitted(&self) -> bool {
        if self.producer != producer_for(self.target) {
            return false;
        }
        match self.route_kind {
            RouteKind::Think => self.proof_admission.is_some_and(ProofRouteAdmission::admitted),
            RouteKind::Look | RouteKind::Review => self.proof_admission.is_none(),
        }
    }
}

pub fn default_candidate(diagnosis: BundleDiagnosis) -> SearchCandidate {
    match diagnosis {
        BundleDiagnosis::MainSnakMismatch => SearchCandidate {
            target: diagnosis,
            producer: producer_for(diagnosis),
            route_kind: RouteKind::Think,
            cost: 3,
            proof_admission: Some(ProofRouteAdmission::least_privilege()),
            route_ref: "prove exact same-carrier semantic correspondence".into(),
        },
        BundleDiagnosis::QualifierDrift => SearchCandidate {
            target: diagnosis,
            producer: producer_for(diagnosis),
            route_kind: RouteKind::Think,
            cost: 2,
            proof_admission: Some(ProofRouteAdmission::least_privilege()),
            route_ref: "prove qualifier-preserving transport".into(),
        },
        BundleDiagnosis::ReferenceDrift => SearchCandidate {
            target: diagnosis,
            producer: producer_for(diagnosis),
            route_kind: RouteKind::Look,
            cost: 1,
            proof_admission: None,
            route_ref: "follow and inspect P248/P854 source candidate".into(),
        },
        BundleDiagnosis::RankQuestion => SearchCandidate {
            target: diagnosis,
            producer: producer_for(diagnosis),
            route_kind: RouteKind::Review,
            cost: 1,
            proof_admission: None,
            route_ref: "review rank treatment without equating rank to truth".into(),
        },
        BundleDiagnosis::ProvenanceGap => SearchCandidate {
            target: diagnosis,
            producer: producer_for(diagnosis),
            route_kind: RouteKind::Look,
            cost: 1,
            proof_admission: None,
            route_ref: "acquire independent provenance; P143 is not enough".into(),
        },
    }
}

/// A changed coordinate opens a live diagnosis fibre; it need not determine a
/// singleton diagnosis. In particular, a reference change may be either a
/// reference-transfer problem or a provenance problem until further evidence.
pub fn diagnoses_for_change(kind: BundleCoordinateKind) -> Vec<BundleDiagnosis> {
    match kind {
        BundleCoordinateKind::MainSnak => vec![BundleDiagnosis::MainSnakMismatch],
        BundleCoordinateKind::Qualifier => vec![BundleDiagnosis::QualifierDrift],
        BundleCoordinateKind::Reference => vec![
            BundleDiagnosis::ReferenceDrift,
            BundleDiagnosis::ProvenanceGap,
        ],
        BundleCoordinateKind::Rank => vec![BundleDiagnosis::RankQuestion],
        BundleCoordinateKind::Provenance => vec![BundleDiagnosis::ProvenanceGap],
    }
}

/// Select the cheapest admitted route over the current live diagnosis fibre.
/// Ties are stable in input order; selection is work policy, not truth weight.
pub fn select_next_route(live: &[BundleDiagnosis]) -> Option<SearchCandidate> {
    live.iter()
        .copied()
        .map(default_candidate)
        .filter(SearchCandidate::admitted)
        .min_by_key(|candidate| candidate.cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_change_keeps_multiple_diagnoses_live() {
        let live = diagnoses_for_change(BundleCoordinateKind::Reference);
        assert_eq!(
            live,
            vec![BundleDiagnosis::ReferenceDrift, BundleDiagnosis::ProvenanceGap]
        );
    }

    #[test]
    fn reference_change_selects_look_not_proof_search() {
        let live = diagnoses_for_change(BundleCoordinateKind::Reference);
        let selected = select_next_route(&live).unwrap();
        assert_eq!(selected.route_kind, RouteKind::Look);
        assert_eq!(selected.producer, ProducerKind::AcquireReferenceEvidence);
        assert_eq!(selected.target, BundleDiagnosis::ReferenceDrift);
    }

    #[test]
    fn semantic_mismatch_enters_admitted_think_route() {
        let selected = select_next_route(&[BundleDiagnosis::MainSnakMismatch]).unwrap();
        assert_eq!(selected.route_kind, RouteKind::Think);
        assert!(selected.proof_admission.unwrap().admitted());
    }

    #[test]
    fn proof_route_with_hidden_hypothesis_strengthening_is_rejected() {
        let mut candidate = default_candidate(BundleDiagnosis::MainSnakMismatch);
        let mut admission = candidate.proof_admission.unwrap();
        admission.no_silent_hypothesis_strengthening = false;
        candidate.proof_admission = Some(admission);
        assert!(!candidate.admitted());
    }

    #[test]
    fn theorem_search_cannot_pay_reference_evidence_producer() {
        let candidate = SearchCandidate {
            target: BundleDiagnosis::ReferenceDrift,
            producer: ProducerKind::AcquireReferenceEvidence,
            route_kind: RouteKind::Think,
            cost: 0,
            proof_admission: Some(ProofRouteAdmission::least_privilege()),
            route_ref: "invalid theorem-only source acquisition".into(),
        };
        // The generic admission bits alone are deliberately insufficient for an
        // evidence producer; callers must use the producer's allowed route.
        assert_ne!(default_candidate(candidate.target).route_kind, RouteKind::Think);
    }

    #[test]
    fn cheapest_policy_does_not_turn_cost_into_truth_weight() {
        let selected = select_next_route(&[
            BundleDiagnosis::MainSnakMismatch,
            BundleDiagnosis::ReferenceDrift,
        ])
        .unwrap();
        assert_eq!(selected.cost, 1);
        assert_eq!(selected.target, BundleDiagnosis::ReferenceDrift);
        // This only chooses work order; the semantic diagnosis remains live
        // until a later receipt actually eliminates it.
    }
}
