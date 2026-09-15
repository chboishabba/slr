//! Diagnosis-fibre and prerequisite-cut search policy for Wikidata bundles.
//!
//! This crate is deliberately downstream of `sensiblaw-source-handoff`.
//! Source capture/bundle identity remain stable while search policy may evolve.
//! It combines sparse reverse-dependency wake with Think/Look/Review planning,
//! a first-missing prerequisite DAG, and bounded Zelph/HF selector requests.
//! Proof routes are least-privilege admitted and cannot substitute for missing
//! source/provenance acquisition.

use sensiblaw_evidential_reopen::WakeRequest;
use sensiblaw_source_handoff::bundle::{
    sparse_wake_bundle_changes, BundleCoordinate, BundleCoordinateKind, BundleDependency,
};

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

pub const fn route_allowed_for_producer(producer: ProducerKind, route: RouteKind) -> bool {
    match producer {
        ProducerKind::EstablishSemanticCorrespondence | ProducerKind::ProveQualifierTransport => {
            matches!(route, RouteKind::Think)
        }
        ProducerKind::AcquireReferenceEvidence | ProducerKind::AcquireProvenanceEvidence => {
            matches!(route, RouteKind::Look)
        }
        ProducerKind::InterpretRank => matches!(route, RouteKind::Review),
    }
}

/// Runtime image of DASHI's least-privilege proof-route admission obligations.
/// These booleans are implementation gates, not substitutes for Agda proofs.
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
        if self.producer != producer_for(self.target)
            || !route_allowed_for_producer(self.producer, self.route_kind)
        {
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

/// A changed coordinate opens a live diagnosis fibre; it need not select one
/// explanation. Reference change is deliberately ambiguous between reference
/// drift and provenance gap until another receipt discriminates them.
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

/// Select the cheapest admitted move over a live diagnosis fibre. Cost is work
/// policy only; it is not probability, confidence, truth, or authority weight.
pub fn select_next_route(live: &[BundleDiagnosis]) -> Option<SearchCandidate> {
    live.iter()
        .copied()
        .map(default_candidate)
        .filter(SearchCandidate::admitted)
        .min_by_key(|candidate| candidate.cost)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosisSearchPlan {
    pub changed: BundleCoordinate,
    pub wakes: Vec<WakeRequest>,
    pub live_diagnoses: Vec<BundleDiagnosis>,
    pub selected: Option<SearchCandidate>,
}

/// Compile one changed bundle coordinate through the pre-existing sparse-wake
/// engine and then into mixed proof/acquisition/review search. If no consumer
/// declared dependency, the plan contains no selected work: zero work is not
/// negative evidence.
pub fn plan_changed_coordinate(
    changed: BundleCoordinate,
    dependencies: &[BundleDependency],
) -> DiagnosisSearchPlan {
    let wakes = sparse_wake_bundle_changes(std::slice::from_ref(&changed), dependencies);
    let live_diagnoses = diagnoses_for_change(changed.kind);
    let selected = if wakes.is_empty() {
        None
    } else {
        select_next_route(&live_diagnoses)
    };
    DiagnosisSearchPlan {
        changed,
        wakes,
        live_diagnoses,
        selected,
    }
}

// ---------------------------------------------------------------------------
// Proof-obligation prerequisite DAG.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Obligation {
    SameCarrier,
    SourceSupport,
    QualifierTransport,
    RankTreatment,
    ProvenanceSupport,
    SemanticCorrespondence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObligationStatus {
    pub same_carrier: bool,
    pub source_support: bool,
    pub qualifier_transport: bool,
    pub rank_treatment: bool,
    pub provenance_support: bool,
    pub semantic_correspondence: bool,
}

impl ObligationStatus {
    pub const fn nat_current() -> Self {
        Self {
            same_carrier: true,
            source_support: false,
            qualifier_transport: false,
            rank_treatment: false,
            provenance_support: false,
            semantic_correspondence: false,
        }
    }

    pub const fn semantic_prerequisites_closed(self) -> bool {
        self.same_carrier
            && self.source_support
            && self.qualifier_transport
            && self.rank_treatment
            && self.provenance_support
    }

    pub const fn first_missing(self) -> Option<Obligation> {
        if !self.same_carrier {
            Some(Obligation::SameCarrier)
        } else if !self.source_support {
            Some(Obligation::SourceSupport)
        } else if !self.qualifier_transport {
            Some(Obligation::QualifierTransport)
        } else if !self.rank_treatment {
            Some(Obligation::RankTreatment)
        } else if !self.provenance_support {
            Some(Obligation::ProvenanceSupport)
        } else if !self.semantic_correspondence {
            Some(Obligation::SemanticCorrespondence)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrerequisiteProducer {
    VerifySameCarrier,
    AcquireSourceSupport,
    ProveQualifierTransport,
    ReviewRankTreatment,
    AcquireProvenanceSupport,
    ProveSemanticCorrespondence,
}

pub const fn producer_for_obligation(obligation: Obligation) -> PrerequisiteProducer {
    match obligation {
        Obligation::SameCarrier => PrerequisiteProducer::VerifySameCarrier,
        Obligation::SourceSupport => PrerequisiteProducer::AcquireSourceSupport,
        Obligation::QualifierTransport => PrerequisiteProducer::ProveQualifierTransport,
        Obligation::RankTreatment => PrerequisiteProducer::ReviewRankTreatment,
        Obligation::ProvenanceSupport => PrerequisiteProducer::AcquireProvenanceSupport,
        Obligation::SemanticCorrespondence => PrerequisiteProducer::ProveSemanticCorrespondence,
    }
}

pub const fn route_for_prerequisite(producer: PrerequisiteProducer) -> RouteKind {
    match producer {
        PrerequisiteProducer::VerifySameCarrier
        | PrerequisiteProducer::AcquireSourceSupport
        | PrerequisiteProducer::AcquireProvenanceSupport => RouteKind::Look,
        PrerequisiteProducer::ProveQualifierTransport
        | PrerequisiteProducer::ProveSemanticCorrespondence => RouteKind::Think,
        PrerequisiteProducer::ReviewRankTreatment => RouteKind::Review,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrerequisiteEdge {
    pub prerequisite: Obligation,
    pub dependent: Obligation,
}

pub const SEMANTIC_PREREQUISITE_EDGES: [PrerequisiteEdge; 5] = [
    PrerequisiteEdge { prerequisite: Obligation::SameCarrier, dependent: Obligation::SemanticCorrespondence },
    PrerequisiteEdge { prerequisite: Obligation::SourceSupport, dependent: Obligation::SemanticCorrespondence },
    PrerequisiteEdge { prerequisite: Obligation::QualifierTransport, dependent: Obligation::SemanticCorrespondence },
    PrerequisiteEdge { prerequisite: Obligation::RankTreatment, dependent: Obligation::SemanticCorrespondence },
    PrerequisiteEdge { prerequisite: Obligation::ProvenanceSupport, dependent: Obligation::SemanticCorrespondence },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZelphHfOperation {
    HeaderProbe,
    SelectedChunkRead,
    NodeRouteSelection,
    SparqlSubset,
    TransitivePropertyPaths,
    PartialLoading,
    QualifierImport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZelphHfSelectorRequest {
    pub selector_id: String,
    pub qids: Vec<String>,
    pub pids: Vec<String>,
    pub operations: Vec<ZelphHfOperation>,
    pub manifest_ref: String,
    pub target_obligation: Obligation,
    pub candidate_only: bool,
    pub review_first: bool,
    pub creates_semantic_proof: bool,
    pub creates_source_authority: bool,
    pub creates_edit_authority: bool,
    pub full_reasoning_safe: bool,
    pub hosted_manifest_final: bool,
}

pub fn nat_zelph_hf_selector(obligation: Obligation) -> Option<ZelphHfSelectorRequest> {
    match obligation {
        Obligation::SameCarrier => Some(ZelphHfSelectorRequest {
            selector_id: "sel:nat:q10403939:same-carrier".into(),
            qids: vec!["Q10403939".into()],
            pids: vec!["P5991".into(), "P14143".into()],
            operations: vec![ZelphHfOperation::NodeRouteSelection, ZelphHfOperation::PartialLoading],
            manifest_ref: "zelph-hf-layout/v2".into(),
            target_obligation: obligation,
            candidate_only: true,
            review_first: true,
            creates_semantic_proof: false,
            creates_source_authority: false,
            creates_edit_authority: false,
            full_reasoning_safe: false,
            hosted_manifest_final: false,
        }),
        Obligation::SourceSupport => Some(ZelphHfSelectorRequest {
            selector_id: "sel:nat:q10403939:p5991-p14143:source-support".into(),
            qids: vec!["Q10403939".into()],
            pids: vec!["P5991".into(), "P14143".into()],
            operations: vec![
                ZelphHfOperation::SparqlSubset,
                ZelphHfOperation::NodeRouteSelection,
                ZelphHfOperation::PartialLoading,
            ],
            manifest_ref: "zelph-hf-layout/v2".into(),
            target_obligation: obligation,
            candidate_only: true,
            review_first: true,
            creates_semantic_proof: false,
            creates_source_authority: false,
            creates_edit_authority: false,
            full_reasoning_safe: false,
            hosted_manifest_final: false,
        }),
        Obligation::ProvenanceSupport => Some(ZelphHfSelectorRequest {
            selector_id: "sel:nat:q10403939:provenance-support".into(),
            qids: vec!["Q10403939".into()],
            pids: vec!["P248".into(), "P854".into(), "P143".into()],
            operations: vec![ZelphHfOperation::SelectedChunkRead, ZelphHfOperation::SparqlSubset],
            manifest_ref: "zelph-hf-layout/v2".into(),
            target_obligation: obligation,
            candidate_only: true,
            review_first: true,
            creates_semantic_proof: false,
            creates_source_authority: false,
            creates_edit_authority: false,
            full_reasoning_safe: false,
            hosted_manifest_final: false,
        }),
        Obligation::QualifierTransport | Obligation::RankTreatment | Obligation::SemanticCorrespondence => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrerequisitePlan {
    pub status: ObligationStatus,
    pub first_missing: Option<Obligation>,
    pub producer: Option<PrerequisiteProducer>,
    pub route_kind: Option<RouteKind>,
    pub selector: Option<ZelphHfSelectorRequest>,
    pub terminal_semantic_candidate: Option<SearchCandidate>,
}

pub fn semantic_candidate_for_status(status: ObligationStatus) -> SearchCandidate {
    let mut admission = ProofRouteAdmission::least_privilege();
    admission.prerequisites_closed = status.semantic_prerequisites_closed();
    SearchCandidate {
        target: BundleDiagnosis::MainSnakMismatch,
        producer: ProducerKind::EstablishSemanticCorrespondence,
        route_kind: RouteKind::Think,
        cost: 3,
        proof_admission: Some(admission),
        route_ref: "terminal semantic correspondence after bundle prerequisite closure".into(),
    }
}

pub fn plan_prerequisite_cut(status: ObligationStatus) -> PrerequisitePlan {
    let first_missing = status.first_missing();
    let producer = first_missing.map(producer_for_obligation);
    let route_kind = producer.map(route_for_prerequisite);
    let selector = first_missing.and_then(nat_zelph_hf_selector);
    let terminal_semantic_candidate = if first_missing == Some(Obligation::SemanticCorrespondence) {
        Some(semantic_candidate_for_status(status))
    } else {
        None
    };
    PrerequisitePlan {
        status,
        first_missing,
        producer,
        route_kind,
        selector,
        terminal_semantic_candidate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_evidential_reopen::ConsumerFibreKey;

    fn consumer(name: &str) -> ConsumerFibreKey {
        ConsumerFibreKey {
            demand_ref: format!("demand:{name}"),
            consumer_ref: format!("consumer:{name}"),
            query_ref: "query:nat-p5991-p14143".into(),
            policy_ref: "policy:nat-review-v1".into(),
        }
    }

    fn reference_coordinate() -> BundleCoordinate {
        BundleCoordinate {
            kind: BundleCoordinateKind::Reference,
            stable_ref: "bundle:nat:q10403939:scope1:2018:P854:https://example.test/report.pdf".into(),
        }
    }

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
        let selected = select_next_route(&diagnoses_for_change(BundleCoordinateKind::Reference))
            .unwrap();
        assert_eq!(selected.route_kind, RouteKind::Look);
        assert_eq!(selected.producer, ProducerKind::AcquireReferenceEvidence);
    }

    #[test]
    fn semantic_mismatch_enters_admitted_think_route() {
        let selected = select_next_route(&[BundleDiagnosis::MainSnakMismatch]).unwrap();
        assert_eq!(selected.route_kind, RouteKind::Think);
        assert!(selected.proof_admission.unwrap().admitted());
    }

    #[test]
    fn hidden_hypothesis_strengthening_rejects_proof_route() {
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
        assert!(!candidate.admitted());
    }

    #[test]
    fn sparse_wake_then_search_stays_reference_local() {
        let changed = reference_coordinate();
        let reference_consumer = consumer("reference-transfer");
        let dependencies = vec![BundleDependency {
            coordinate: changed.clone(),
            consumer: reference_consumer.clone(),
            minimum_horizon: 3,
        }];
        let plan = plan_changed_coordinate(changed, &dependencies);
        assert_eq!(plan.wakes.len(), 1);
        assert_eq!(plan.wakes[0].consumer, reference_consumer);
        assert_eq!(plan.selected.as_ref().unwrap().route_kind, RouteKind::Look);
        assert_eq!(
            plan.selected.as_ref().unwrap().producer,
            ProducerKind::AcquireReferenceEvidence
        );
    }

    #[test]
    fn no_dependency_means_no_search_work_not_negative_evidence() {
        let plan = plan_changed_coordinate(reference_coordinate(), &[]);
        assert!(plan.wakes.is_empty());
        assert!(plan.selected.is_none());
        assert_eq!(plan.live_diagnoses.len(), 2);
    }

    #[test]
    fn nat_first_missing_prerequisite_is_source_support() {
        let plan = plan_prerequisite_cut(ObligationStatus::nat_current());
        assert_eq!(plan.first_missing, Some(Obligation::SourceSupport));
        assert_eq!(plan.producer, Some(PrerequisiteProducer::AcquireSourceSupport));
        assert_eq!(plan.route_kind, Some(RouteKind::Look));
        let selector = plan.selector.unwrap();
        assert_eq!(selector.target_obligation, Obligation::SourceSupport);
        assert!(selector.candidate_only);
        assert!(selector.review_first);
        assert!(!selector.creates_semantic_proof);
        assert!(!selector.full_reasoning_safe);
        assert!(!selector.hosted_manifest_final);
    }

    #[test]
    fn paying_source_support_advances_to_qualifier_transport() {
        let mut status = ObligationStatus::nat_current();
        status.source_support = true;
        let plan = plan_prerequisite_cut(status);
        assert_eq!(plan.first_missing, Some(Obligation::QualifierTransport));
        assert_eq!(plan.producer, Some(PrerequisiteProducer::ProveQualifierTransport));
        assert_eq!(plan.route_kind, Some(RouteKind::Think));
        assert!(plan.selector.is_none());
    }

    #[test]
    fn terminal_semantic_theorem_is_not_admitted_while_source_support_missing() {
        let candidate = semantic_candidate_for_status(ObligationStatus::nat_current());
        assert!(!candidate.admitted());
        assert!(!candidate.proof_admission.unwrap().prerequisites_closed);
    }

    #[test]
    fn terminal_semantic_theorem_becomes_admissible_only_after_prerequisites() {
        let status = ObligationStatus {
            same_carrier: true,
            source_support: true,
            qualifier_transport: true,
            rank_treatment: true,
            provenance_support: true,
            semantic_correspondence: false,
        };
        let plan = plan_prerequisite_cut(status);
        assert_eq!(plan.first_missing, Some(Obligation::SemanticCorrespondence));
        assert_eq!(plan.route_kind, Some(RouteKind::Think));
        assert!(plan.selector.is_none());
        assert!(plan.terminal_semantic_candidate.unwrap().admitted());
    }

    #[test]
    fn zelph_hf_selector_never_grants_proof_or_authority() {
        let selector = nat_zelph_hf_selector(Obligation::SourceSupport).unwrap();
        assert!(!selector.creates_semantic_proof);
        assert!(!selector.creates_source_authority);
        assert!(!selector.creates_edit_authority);
        assert!(!selector.full_reasoning_safe);
    }
}
