//! GWB ambiguity-directed adaptive campaign.
//!
//! This module welds existing GWB world-research obligations and typed route
//! candidates to the canonical proof-search Pareto frontier. It does not crawl
//! breadth-first, scalarize ambiguity, review evidence, persist world changes,
//! or promote route candidates to truth.

use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_proof_search_loop::frontier::{
    select_frontier_move, FrontierCandidateMove, ProofFrontier, ProofResidual, ResidualStatus,
};
use sensiblaw_proof_search_scheduler::{
    CandidateMove, ExecutionCostVector, ExecutionStrategy, ProofValueVector,
};
use sensiblaw_pg_source_store::{
    gwb_ambiguity_state_row, GwbAmbiguityStateError, GwbAmbiguityStateInput,
    GwbAmbiguityStateRow,
};
use sensiblaw_route_selector::{ProducerFamily, RouteCandidate, RouteFamily};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const GWB_ADAPTIVE_HOP_TARGET: usize = 100;
pub const GWB_ADAPTIVE_CAMPAIGN_REF: &str = "campaign:gwb-ambiguity-directed-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GwbAmbiguityKind {
    Identity,
    SourceWorkIdentity,
    TypeClass,
    Superclass,
    Subclass,
    PropertySupport,
    CrossLanguageGap,
    UnsupportedDependency,
    Provenance,
    CompetingAlternatives,
    ConsumerSemanticGap,
}

impl GwbAmbiguityKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::SourceWorkIdentity => "source-work-identity",
            Self::TypeClass => "type-class",
            Self::Superclass => "superclass",
            Self::Subclass => "subclass",
            Self::PropertySupport => "property-support",
            Self::CrossLanguageGap => "cross-language-gap",
            Self::UnsupportedDependency => "unsupported-dependency",
            Self::Provenance => "provenance",
            Self::CompetingAlternatives => "competing-alternatives",
            Self::ConsumerSemanticGap => "consumer-semantic-gap",
        }
    }

    const fn producer_class_ref(self) -> &'static str {
        match self {
            Self::Identity | Self::SourceWorkIdentity => "producer:wikimedia-identity",
            Self::TypeClass | Self::Superclass | Self::Subclass | Self::PropertySupport => {
                "producer:wikidata-classification"
            }
            Self::CrossLanguageGap => "producer:wikipedia-surface",
            Self::UnsupportedDependency => "producer:parser-or-evidence-repair",
            Self::Provenance => "producer:source-provenance",
            Self::CompetingAlternatives | Self::ConsumerSemanticGap => {
                "producer:consumer-discriminator"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GwbAmbiguityResidual {
    pub residual_ref: String,
    pub subject_ref: String,
    pub proposition_ref: String,
    pub kind: GwbAmbiguityKind,
    pub root_qid: Option<String>,
    pub salience: u64,
    pub dependency_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GwbInvestigationKind {
    Identity,
    SourceWorkIdentity,
    TypeClass,
    Superclass,
    Subclass,
    Property,
    CrossLanguageSurface,
    SourceLookup,
    Provenance,
    ParserRepair,
    ExternalOntologyFallback,
    Snowball,
}

impl GwbInvestigationKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::SourceWorkIdentity => "source-work-identity",
            Self::TypeClass => "type-class",
            Self::Superclass => "superclass",
            Self::Subclass => "subclass",
            Self::Property => "property",
            Self::CrossLanguageSurface => "cross-language-surface",
            Self::SourceLookup => "source-lookup",
            Self::Provenance => "provenance",
            Self::ParserRepair => "parser-repair",
            Self::ExternalOntologyFallback => "external-ontology-fallback",
            Self::Snowball => "snowball",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbInvestigationCandidate {
    pub target_residual_refs: Vec<String>,
    pub move_ref: String,
    pub investigation_kind: GwbInvestigationKind,
    pub producer_ref: String,
    pub source_ref: Option<String>,
    pub source_revision_ref: Option<String>,
    pub target_ref: Option<String>,
    pub property_ref: Option<String>,
    pub expected_residual_contraction: u64,
    pub ambiguity_reduction: u64,
    pub type_closure_gain: u64,
    pub cross_surface_gap_gain: u64,
    pub source_support_gain: u64,
    pub shared_dependency_gain: u64,
    pub network_requests: u64,
    pub operator_review_cost: u64,
    pub admissible: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbCompiledAmbiguityFrontier {
    pub frontier: ProofFrontier,
    pub candidates: Vec<FrontierCandidateMove>,
    pub investigations: BTreeMap<String, GwbInvestigationCandidate>,
    pub pareto_dimensions_scalarized: bool,
    pub frontier_rank_is_truth_rank: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

fn route_investigation_kind(route: &RouteCandidate) -> GwbInvestigationKind {
    match (route.route_family, route.property_ref.as_str(), route.producer) {
        (RouteFamily::WikidataProperty, "P31", _) => GwbInvestigationKind::TypeClass,
        (RouteFamily::WikidataProperty, "P279", _) => GwbInvestigationKind::Superclass,
        (RouteFamily::WikidataProperty, _, _) => GwbInvestigationKind::Property,
        (RouteFamily::WikipediaArticle, _, _) => GwbInvestigationKind::CrossLanguageSurface,
        (RouteFamily::ParserRepair, _, _) => GwbInvestigationKind::ParserRepair,
        (
            RouteFamily::PrimarySourceSearch
            | RouteFamily::MeasurementSourceSearch
            | RouteFamily::ComparatorSourceSearch,
            _,
            ProducerFamily::IdentitySource,
        ) => GwbInvestigationKind::Identity,
        (
            RouteFamily::PrimarySourceSearch
            | RouteFamily::MeasurementSourceSearch
            | RouteFamily::ComparatorSourceSearch,
            _,
            _,
        ) => GwbInvestigationKind::SourceLookup,
        (RouteFamily::RevisionHistory, _, _) => GwbInvestigationKind::Provenance,
    }
}

fn producer_ref(route: &RouteCandidate) -> &'static str {
    match route.producer {
        ProducerFamily::ArticleSemantic => "producer:wikipedia-surface",
        ProducerFamily::RevisionTemporal => "producer:source-provenance",
        ProducerFamily::ParserRepair => "producer:parser-repair",
        ProducerFamily::IdentitySource => "producer:wikimedia-identity",
        ProducerFamily::AuthoritySource => "producer:authority-source",
        ProducerFamily::MechanismEvidence => "producer:mechanism-evidence",
        ProducerFamily::MeasurementEvidence => "producer:measurement-evidence",
        ProducerFamily::ComparatorEvidence => "producer:comparator-evidence",
        ProducerFamily::ClassificationEvidence => "producer:wikidata-classification",
    }
}

fn route_is_relevant(residual: &GwbAmbiguityResidual, route: &RouteCandidate) -> bool {
    if residual
        .root_qid
        .as_deref()
        .is_some_and(|qid| qid != route.source_ref)
    {
        return false;
    }
    match residual.kind {
        GwbAmbiguityKind::TypeClass => {
            route.route_family == RouteFamily::WikidataProperty
                || route.producer == ProducerFamily::ClassificationEvidence
        }
        GwbAmbiguityKind::Superclass => {
            route.route_family == RouteFamily::WikidataProperty
                && matches!(route.property_ref.as_str(), "P31" | "P279")
        }
        GwbAmbiguityKind::Subclass => {
            // Direct entity RDF exposes P279 towards a superclass. Inverse
            // subclass discovery remains a separately governed query producer.
            false
        }
        GwbAmbiguityKind::CrossLanguageGap => {
            route.route_family == RouteFamily::WikipediaArticle
        }
        GwbAmbiguityKind::Identity | GwbAmbiguityKind::SourceWorkIdentity => {
            route.producer == ProducerFamily::IdentitySource
                || route.route_family == RouteFamily::WikipediaArticle
                || (route.route_family == RouteFamily::WikidataProperty
                    && route.property_ref == "P31")
        }
        GwbAmbiguityKind::Provenance => {
            route.route_family == RouteFamily::RevisionHistory
                || matches!(
                    route.route_family,
                    RouteFamily::PrimarySourceSearch
                        | RouteFamily::MeasurementSourceSearch
                        | RouteFamily::ComparatorSourceSearch
                )
        }
        GwbAmbiguityKind::UnsupportedDependency => {
            route.route_family == RouteFamily::ParserRepair
                || matches!(
                    route.route_family,
                    RouteFamily::PrimarySourceSearch
                        | RouteFamily::MeasurementSourceSearch
                        | RouteFamily::ComparatorSourceSearch
                )
        }
        GwbAmbiguityKind::PropertySupport
        | GwbAmbiguityKind::CompetingAlternatives
        | GwbAmbiguityKind::ConsumerSemanticGap => true,
    }
}

#[must_use]
pub fn route_candidate_to_investigation(
    residual: &GwbAmbiguityResidual,
    route: &RouteCandidate,
    source_revision_ref: &str,
) -> Option<GwbInvestigationCandidate> {
    if !route_is_relevant(residual, route) || source_revision_ref.trim().is_empty() {
        return None;
    }
    let kind = route_investigation_kind(route);
    let network_requests = if route.route_family == RouteFamily::WikipediaArticle {
        1
    } else {
        u64::from(route.prior_network_requests)
    };
    Some(GwbInvestigationCandidate {
        target_residual_refs: vec![residual.residual_ref.clone()],
        move_ref: format!("move:gwb:{}", route.candidate_id),
        investigation_kind: kind,
        producer_ref: producer_ref(route).into(),
        source_ref: Some(route.source_ref.clone()),
        source_revision_ref: Some(source_revision_ref.to_owned()),
        target_ref: Some(route.target_ref.clone()),
        property_ref: (!route.property_ref.is_empty()).then(|| route.property_ref.clone()),
        expected_residual_contraction: 1.max(u64::from(route.prior_contracted_old_gaps)),
        ambiguity_reduction: u64::from(route.route_specificity),
        type_closure_gain: u64::from(route.typed_property_support),
        cross_surface_gap_gain: u64::from(route.cross_language_gap_coverage),
        source_support_gain: u64::from(route.source_surface_support),
        shared_dependency_gain: u64::from(route.root_qid_support),
        network_requests,
        operator_review_cost: 1,
        admissible: true,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

impl GwbInvestigationCandidate {
    #[must_use]
    pub fn governed_query(
        target_residual_ref: impl Into<String>,
        move_ref: impl Into<String>,
        investigation_kind: GwbInvestigationKind,
        producer_ref: impl Into<String>,
        source_ref: Option<String>,
        target_ref: Option<String>,
        provider_operation_ref: impl Into<String>,
        expected_residual_contraction: u64,
        ambiguity_reduction: u64,
        type_closure_gain: u64,
        cross_surface_gap_gain: u64,
        shared_dependency_gain: u64,
    ) -> Self {
        let producer_ref = producer_ref.into();
        let provider_operation_ref = provider_operation_ref.into();
        Self {
            target_residual_refs: vec![target_residual_ref.into()],
            move_ref: move_ref.into(),
            investigation_kind,
            producer_ref: format!("{producer_ref}|operation:{provider_operation_ref}"),
            source_ref,
            source_revision_ref: None,
            target_ref,
            property_ref: None,
            expected_residual_contraction,
            ambiguity_reduction,
            type_closure_gain,
            cross_surface_gap_gain,
            source_support_gain: 0,
            shared_dependency_gain,
            network_requests: 1,
            operator_review_cost: 1,
            admissible: true,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }
}

fn proof_residual(residual: &GwbAmbiguityResidual) -> ProofResidual {
    ProofResidual {
        residual_ref: residual.residual_ref.clone(),
        proposition_ref: residual.proposition_ref.clone(),
        producer_class_ref: residual.kind.producer_class_ref().into(),
        jurisdiction_ref: None,
        authority_requirement_ref: None,
        salience: residual.salience,
        dependency_refs: residual.dependency_refs.clone(),
        status: ResidualStatus::Open,
    }
}

fn execution_strategy(candidate: &GwbInvestigationCandidate) -> ExecutionStrategy {
    match candidate.investigation_kind {
        GwbInvestigationKind::TypeClass
        | GwbInvestigationKind::Superclass
        | GwbInvestigationKind::Property
            if candidate.source_revision_ref.is_some() =>
        {
            ExecutionStrategy::LocalWorldGraph
        }
        GwbInvestigationKind::CrossLanguageSurface
        | GwbInvestigationKind::Subclass
        | GwbInvestigationKind::SourceLookup
        | GwbInvestigationKind::ExternalOntologyFallback
        | GwbInvestigationKind::Snowball => ExecutionStrategy::GovernedLiveReferenceSearch,
        GwbInvestigationKind::Identity
        | GwbInvestigationKind::SourceWorkIdentity
        | GwbInvestigationKind::Provenance
        | GwbInvestigationKind::ParserRepair
        | GwbInvestigationKind::TypeClass
        | GwbInvestigationKind::Superclass
        | GwbInvestigationKind::Property => ExecutionStrategy::LocalWorldGraph,
    }
}

fn frontier_move(candidate: &GwbInvestigationCandidate) -> FrontierCandidateMove {
    FrontierCandidateMove {
        target_residual_refs: candidate.target_residual_refs.clone(),
        move_: CandidateMove {
            move_ref: candidate.move_ref.clone(),
            strategy: execution_strategy(candidate),
            source_ref: candidate.source_revision_ref.clone().or_else(|| candidate.source_ref.clone()),
            provider_operation_ref: candidate.investigation_kind.as_str().into(),
            cost: ExecutionCostVector {
                network_requests: candidate.network_requests,
                semantic_assessment_cost: 1,
                operator_review_cost: candidate.operator_review_cost,
                ..ExecutionCostVector::default()
            },
            value: ProofValueVector {
                expected_proof_reduction: candidate.expected_residual_contraction,
                discriminative_value: candidate.ambiguity_reduction,
                authority_fitness: candidate.source_support_gain,
                novelty: candidate.type_closure_gain,
                coverage_gain: candidate.cross_surface_gap_gain,
            },
            admissible: candidate.admissible,
            calibration_ref: "gwb-ambiguity-directed:v1".into(),
        },
        expected_whole_frontier_reduction: candidate.expected_residual_contraction,
        shared_dependency_gain: candidate.shared_dependency_gain,
    }
}

#[must_use]
pub fn compile_gwb_ambiguity_frontier(
    residuals: &[GwbAmbiguityResidual],
    investigations: &[GwbInvestigationCandidate],
    frontier_ref: impl Into<String>,
) -> GwbCompiledAmbiguityFrontier {
    let mut residuals = residuals.to_vec();
    residuals.sort();
    residuals.dedup();

    let frontier = ProofFrontier {
        consumer_ref: "consumer:gwb-ambiguity-directed-world-research".into(),
        frontier_ref: frontier_ref.into(),
        residuals: residuals.iter().map(proof_residual).collect(),
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };

    let open_refs = frontier
        .open_residuals()
        .map(|residual| residual.residual_ref.as_str())
        .collect::<BTreeSet<_>>();

    let mut investigation_map = BTreeMap::new();
    for investigation in investigations {
        if !investigation.candidate_only
            || investigation.creates_semantic_authority
            || investigation.applicability_promoted
            || investigation.claim_truth_promoted
            || investigation.target_residual_refs.is_empty()
            || !investigation
                .target_residual_refs
                .iter()
                .all(|residual_ref| open_refs.contains(residual_ref.as_str()))
        {
            continue;
        }
        investigation_map
            .entry(investigation.move_ref.clone())
            .or_insert_with(|| investigation.clone());
    }

    let candidates = investigation_map.values().map(frontier_move).collect();

    GwbCompiledAmbiguityFrontier {
        frontier,
        candidates,
        investigations: investigation_map,
        pareto_dimensions_scalarized: false,
        frontier_rank_is_truth_rank: false,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[must_use]
pub fn select_gwb_investigation(
    compiled: &GwbCompiledAmbiguityFrontier,
) -> Option<&GwbInvestigationCandidate> {
    let selected = select_frontier_move(&compiled.frontier, &compiled.candidates, 1)?;
    compiled.investigations.get(&selected.move_.move_ref)
}


#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GwbAmbiguityProjectionError {
    #[error("invalid durable GWB ambiguity kind: {0}")]
    InvalidKind(String),
}

fn parse_kind(value: &str) -> Option<GwbAmbiguityKind> {
    match value {
        "identity" => Some(GwbAmbiguityKind::Identity),
        "source-work-identity" => Some(GwbAmbiguityKind::SourceWorkIdentity),
        "type-class" => Some(GwbAmbiguityKind::TypeClass),
        "superclass" => Some(GwbAmbiguityKind::Superclass),
        "subclass" => Some(GwbAmbiguityKind::Subclass),
        "property-support" => Some(GwbAmbiguityKind::PropertySupport),
        "cross-language-gap" => Some(GwbAmbiguityKind::CrossLanguageGap),
        "unsupported-dependency" => Some(GwbAmbiguityKind::UnsupportedDependency),
        "provenance" => Some(GwbAmbiguityKind::Provenance),
        "competing-alternatives" => Some(GwbAmbiguityKind::CompetingAlternatives),
        "consumer-semantic-gap" => Some(GwbAmbiguityKind::ConsumerSemanticGap),
        _ => None,
    }
}

pub fn ambiguity_residuals_from_rows(
    rows: &[GwbAmbiguityStateRow],
) -> Result<Vec<GwbAmbiguityResidual>, GwbAmbiguityProjectionError> {
    rows.iter()
        .map(|row| {
            let kind = parse_kind(&row.kind_ref)
                .ok_or_else(|| GwbAmbiguityProjectionError::InvalidKind(row.kind_ref.clone()))?;
            Ok(GwbAmbiguityResidual {
                residual_ref: row.residual_ref.clone(),
                subject_ref: row.subject_ref.clone(),
                proposition_ref: row.proposition_ref.clone(),
                kind,
                root_qid: row.root_qid.clone(),
                salience: row.salience,
                dependency_refs: row.dependency_refs.clone(),
            })
        })
        .collect()
}

#[must_use]
pub fn seed_gwb_qid_ambiguities(root_qid: &str) -> Vec<GwbAmbiguityStateInput> {
    [
        (
            "type-class",
            format!("residual:gwb:{root_qid}:type-class"),
            format!("gwb:ambiguity:{root_qid}:type-class"),
            100u64,
        ),
        (
            "superclass",
            format!("residual:gwb:{root_qid}:superclass"),
            format!("gwb:ambiguity:{root_qid}:superclass"),
            90u64,
        ),
        (
            "property-support",
            format!("residual:gwb:{root_qid}:property-support"),
            format!("gwb:ambiguity:{root_qid}:property-support"),
            80u64,
        ),
        (
            "cross-language-gap",
            format!("residual:gwb:{root_qid}:cross-language"),
            format!("gwb:ambiguity:{root_qid}:cross-language"),
            70u64,
        ),
        (
            "consumer-semantic-gap",
            format!("residual:gwb:{root_qid}:consumer-semantic-gap"),
            format!("gwb:ambiguity:{root_qid}:consumer-semantic-gap"),
            60u64,
        ),
    ]
    .into_iter()
    .map(|(kind_ref, residual_ref, proposition_ref, salience)| GwbAmbiguityStateInput {
        campaign_ref: GWB_ADAPTIVE_CAMPAIGN_REF.into(),
        residual_ref,
        subject_ref: root_qid.into(),
        proposition_ref,
        kind_ref: kind_ref.into(),
        root_qid: Some(root_qid.into()),
        salience,
        dependency_refs: vec![format!("dependency:gwb:{root_qid}")],
        opened_by_hop: None,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
    .collect()
}

#[must_use]
pub fn residuals_opened_by_reviewed_investigation(
    hop_index: usize,
    selected: &GwbInvestigationCandidate,
    outcome_ref: &str,
) -> Vec<GwbAmbiguityStateInput> {
    let Some(target) = selected.target_ref.as_deref() else {
        return vec![];
    };
    let is_qid = target
        .strip_prefix('Q')
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()));
    if !is_qid {
        return vec![];
    }
    let opens_target = matches!(
        outcome_ref,
        "new-related-object" | "new-conceptual-parent" | "same-object"
    );
    if !opens_target {
        return vec![];
    }

    seed_gwb_qid_ambiguities(target)
        .into_iter()
        .map(|mut row| {
            row.opened_by_hop = Some(hop_index);
            row
        })
        .collect()
}

#[must_use]
pub fn gwb_ambiguity_world_sha256(rows: &[GwbAmbiguityStateRow]) -> String {
    let mut rows = rows.to_vec();
    rows.sort_by(|left, right| left.residual_ref.cmp(&right.residual_ref));
    let mut hasher = Sha256::new();
    hasher.update(b"gwb-ambiguity-world:v1\0");
    for row in rows {
        for value in [
            row.campaign_ref.as_str(),
            row.residual_ref.as_str(),
            row.subject_ref.as_str(),
            row.proposition_ref.as_str(),
            row.kind_ref.as_str(),
            row.root_qid.as_deref().unwrap_or(""),
            &row.salience.to_string(),
        ] {
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
        let mut dependencies = row.dependency_refs.clone();
        dependencies.sort();
        dependencies.dedup();
        for dependency in dependencies {
            hasher.update((dependency.len() as u64).to_be_bytes());
            hasher.update(dependency.as_bytes());
        }
    }
    let digest = hasher.finalize();
    format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}


pub fn project_open_gwb_state_after_review(
    before: &[GwbAmbiguityStateRow],
    close_residual_refs: &[String],
    open_residuals: &[GwbAmbiguityStateInput],
) -> Result<Vec<GwbAmbiguityStateRow>, GwbAmbiguityStateError> {
    let closed = close_residual_refs.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut rows = before
        .iter()
        .filter(|row| !closed.contains(row.residual_ref.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    rows.extend(
        open_residuals
            .iter()
            .map(gwb_ambiguity_state_row)
            .collect::<Result<Vec<_>, _>>()?,
    );
    rows.sort_by(|left, right| left.residual_ref.cmp(&right.residual_ref));
    rows.dedup_by(|left, right| left.residual_ref == right.residual_ref);
    Ok(rows)
}


fn merge_question(
    questions: &mut BTreeMap<String, GwbInvestigationCandidate>,
    residual: &GwbAmbiguityResidual,
    move_ref: String,
    kind: GwbInvestigationKind,
    producer_ref: &str,
    source_ref: Option<String>,
    target_ref: Option<String>,
    base_type_gain: u64,
    base_surface_gain: u64,
) {
    let question = questions.entry(move_ref.clone()).or_insert_with(|| {
        GwbInvestigationCandidate::governed_query(
            residual.residual_ref.clone(),
            move_ref,
            kind,
            producer_ref,
            source_ref,
            target_ref,
            "gwb:selected-question",
            1,
            residual.salience.max(1),
            base_type_gain,
            base_surface_gain,
            u64::try_from(residual.dependency_refs.len()).unwrap_or(u64::MAX).max(1),
        )
    });
    if !question
        .target_residual_refs
        .iter()
        .any(|value| value == &residual.residual_ref)
    {
        question.target_residual_refs.push(residual.residual_ref.clone());
        question.target_residual_refs.sort();
    }
    question.ambiguity_reduction = question.ambiguity_reduction.max(residual.salience);
    question.shared_dependency_gain = question.shared_dependency_gain.max(
        u64::try_from(residual.dependency_refs.len())
            .unwrap_or(u64::MAX)
            .max(1),
    );
}

#[must_use]
pub fn gwb_question_investigations(
    residuals: &[GwbAmbiguityResidual],
    reviewed_move_refs: &BTreeSet<String>,
) -> Vec<GwbInvestigationCandidate> {
    let mut questions = BTreeMap::new();

    for residual in residuals {
        let root = residual.root_qid.clone();
        match residual.kind {
            GwbAmbiguityKind::TypeClass | GwbAmbiguityKind::Superclass => {
                if let Some(qid) = root {
                    merge_question(
                        &mut questions,
                        residual,
                        format!("move:gwb:inspect:{qid}:classification"),
                        GwbInvestigationKind::TypeClass,
                        "producer:wikidata-classification",
                        Some(qid),
                        None,
                        1,
                        0,
                    );
                }
            }
            GwbAmbiguityKind::PropertySupport => {
                if let Some(qid) = root {
                    merge_question(
                        &mut questions,
                        residual,
                        format!("move:gwb:inspect:{qid}:properties"),
                        GwbInvestigationKind::Property,
                        "producer:wikidata-property",
                        Some(qid),
                        None,
                        0,
                        0,
                    );
                }
            }
            GwbAmbiguityKind::CrossLanguageGap => {
                if residual.subject_ref.starts_with("https://")
                    && residual.subject_ref.contains(".wikipedia.org/")
                {
                    merge_question(
                        &mut questions,
                        residual,
                        format!("move:gwb:acquire:{}", residual.residual_ref),
                        GwbInvestigationKind::CrossLanguageSurface,
                        "producer:wikipedia-surface",
                        root,
                        Some(residual.subject_ref.clone()),
                        0,
                        1,
                    );
                } else if let Some(qid) = root {
                    merge_question(
                        &mut questions,
                        residual,
                        format!("move:gwb:inspect:{qid}:wikipedia-surfaces"),
                        GwbInvestigationKind::CrossLanguageSurface,
                        "producer:wikidata-sitelink-surface",
                        Some(qid),
                        None,
                        0,
                        1,
                    );
                }
            }
            GwbAmbiguityKind::Identity | GwbAmbiguityKind::SourceWorkIdentity => {
                if let Some(qid) = root {
                    merge_question(
                        &mut questions,
                        residual,
                        format!("move:gwb:inspect:{qid}:identity"),
                        GwbInvestigationKind::Identity,
                        "producer:wikimedia-identity",
                        Some(qid),
                        None,
                        1,
                        0,
                    );
                }
            }
            GwbAmbiguityKind::Subclass => {
                if let Some(qid) = root {
                    merge_question(
                        &mut questions,
                        residual,
                        format!("move:gwb:query:{qid}:inverse-P279"),
                        GwbInvestigationKind::Subclass,
                        "producer:wikidata-classification",
                        Some(qid),
                        None,
                        1,
                        0,
                    );
                }
            }
            GwbAmbiguityKind::UnsupportedDependency => {
                merge_question(
                    &mut questions,
                    residual,
                    format!("move:gwb:repair:{}", residual.residual_ref),
                    GwbInvestigationKind::ParserRepair,
                    "producer:parser-repair",
                    root,
                    None,
                    0,
                    0,
                );
            }
            GwbAmbiguityKind::Provenance => {
                merge_question(
                    &mut questions,
                    residual,
                    format!("move:gwb:provenance:{}", residual.residual_ref),
                    GwbInvestigationKind::Provenance,
                    "producer:source-provenance",
                    root,
                    None,
                    0,
                    0,
                );
            }
            GwbAmbiguityKind::CompetingAlternatives
            | GwbAmbiguityKind::ConsumerSemanticGap => {
                if let Some(qid) = root.clone() {
                    merge_question(
                        &mut questions,
                        residual,
                        format!("move:gwb:inspect:{qid}:classification"),
                        GwbInvestigationKind::TypeClass,
                        "producer:wikidata-classification",
                        Some(qid.clone()),
                        None,
                        1,
                        0,
                    );
                    merge_question(
                        &mut questions,
                        residual,
                        format!("move:gwb:inspect:{qid}:wikipedia-surfaces"),
                        GwbInvestigationKind::CrossLanguageSurface,
                        "producer:wikidata-sitelink-surface",
                        Some(qid),
                        None,
                        0,
                        1,
                    );
                } else {
                    merge_question(
                        &mut questions,
                        residual,
                        format!("move:gwb:snowball:{}", residual.residual_ref),
                        GwbInvestigationKind::Snowball,
                        "producer:snowball",
                        None,
                        None,
                        0,
                        0,
                    );
                }
            }
        }
    }

    let mut active = questions
        .into_values()
        .filter(|candidate| !reviewed_move_refs.contains(&candidate.move_ref))
        .collect::<Vec<_>>();

    for residual in residuals {
        let already_targeted = active.iter().any(|candidate| {
            candidate
                .target_residual_refs
                .iter()
                .any(|value| value == &residual.residual_ref)
        });
        if already_targeted {
            continue;
        }

        let external_move_ref = format!("move:gwb:external:{}", residual.residual_ref);
        if !reviewed_move_refs.contains(&external_move_ref) {
            active.push(GwbInvestigationCandidate::governed_query(
                residual.residual_ref.clone(),
                external_move_ref,
                GwbInvestigationKind::ExternalOntologyFallback,
                "producer:external-ontology-advisory",
                residual.root_qid.clone(),
                None,
                "external-ontology:consumer-residual",
                1,
                residual.salience.max(1),
                matches!(
                    residual.kind,
                    GwbAmbiguityKind::TypeClass
                        | GwbAmbiguityKind::Superclass
                        | GwbAmbiguityKind::Subclass
                ) as u64,
                0,
                u64::try_from(residual.dependency_refs.len())
                    .unwrap_or(u64::MAX)
                    .max(1),
            ));
            continue;
        }

        let snowball_move_ref = format!("move:gwb:snowball:{}", residual.residual_ref);
        if !reviewed_move_refs.contains(&snowball_move_ref) {
            active.push(GwbInvestigationCandidate::governed_query(
                residual.residual_ref.clone(),
                snowball_move_ref,
                GwbInvestigationKind::Snowball,
                "producer:snowball",
                residual.root_qid.clone(),
                None,
                "snowball:surviving-consumer-debt",
                1,
                residual.salience.max(1),
                0,
                0,
                u64::try_from(residual.dependency_refs.len())
                    .unwrap_or(u64::MAX)
                    .max(1),
            ));
        }
    }

    active.sort_by(|left, right| left.move_ref.cmp(&right.move_ref));
    active
}

#[must_use]
pub fn compile_gwb_question_frontier(
    residuals: &[GwbAmbiguityResidual],
    reviewed_move_refs: &BTreeSet<String>,
    frontier_ref: impl Into<String>,
) -> GwbCompiledAmbiguityFrontier {
    let investigations = gwb_question_investigations(residuals, reviewed_move_refs);
    compile_gwb_ambiguity_frontier(residuals, &investigations, frontier_ref)
}

#[must_use]
pub fn residuals_opened_by_reviewed_routes(
    hop_index: usize,
    selected: &GwbInvestigationCandidate,
    outcome_ref: &str,
    observed_routes: &[RouteCandidate],
) -> Vec<GwbAmbiguityStateInput> {
    if matches!(
        outcome_ref,
        "wrong-type" | "duplicate" | "irrelevant-to-residual" | "empty" | "no-support" | "abstain"
    ) {
        return vec![];
    }
    let mut opened = Vec::new();
    for route in observed_routes {
        match route.route_family {
            RouteFamily::WikidataProperty => {
                let target = route.target_ref.as_str();
                if target
                    .strip_prefix('Q')
                    .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
                {
                    for mut row in seed_gwb_qid_ambiguities(target) {
                        row.opened_by_hop = Some(hop_index);
                        opened.push(row);
                    }
                }
            }
            RouteFamily::WikipediaArticle => {
                let url = route.target_ref.as_str();
                opened.push(GwbAmbiguityStateInput {
                    campaign_ref: GWB_ADAPTIVE_CAMPAIGN_REF.into(),
                    residual_ref: format!(
                        "residual:gwb:surface:{}",
                        Sha256::digest(url.as_bytes())
                            .iter()
                            .take(8)
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                    ),
                    subject_ref: url.into(),
                    proposition_ref: format!("gwb:surface-observation:{url}"),
                    kind_ref: "cross-language-gap".into(),
                    root_qid: Some(route.source_ref.clone()),
                    salience: 75,
                    dependency_refs: vec![selected.move_ref.clone()],
                    opened_by_hop: Some(hop_index),
                    candidate_only: true,
                    creates_semantic_authority: false,
                    applicability_promoted: false,
                    claim_truth_promoted: false,
                });
            }
            _ => {}
        }
    }
    opened.sort_by(|left, right| left.residual_ref.cmp(&right.residual_ref));
    opened.dedup_by(|left, right| left.residual_ref == right.residual_ref);
    opened
}
