//! Australian contract-law LegalFollow fixtures and trace contracts.
//!
//! This is not a contract-law ontology and not legal authority.  It is a
//! typed, source-seeking trace specification used to test whether generic
//! LegalFollow can reconstruct Australian contract doctrine across common-law,
//! equitable, statutory, jurisdictional, and temporal branches without
//! acquiring case-specific runtime machinery.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AuthorityLevel, LegalSourceDemand, LegalSourcePlan, PlanState, SourceRole,
    OALC_DATASET_ID, OALC_PROVIDER_PROFILE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractDoctrine {
    Formation,
    Intention,
    TermsAndIncorporation,
    Construction,
    Estoppel,
    Unconscionability,
    Penalties,
    RepudiationAndTermination,
    Damages,
    Restitution,
    Privity,
    ConsumerLaw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TraceNodeKind {
    Doctrine,
    CaseAuthority,
    Legislation,
    ResearchRequirement,
    Matter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TreatmentKind {
    Seeds,
    Supports,
    Applies,
    Follows,
    Distinguishes,
    Qualifies,
    Displaces,
    TemporalSuccessor,
    Requires,
    Intersects,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractTraceNode {
    pub semantic_ref: String,
    pub label: String,
    pub kind: TraceNodeKind,
    pub doctrine: Option<ContractDoctrine>,
    pub jurisdiction_ref: String,
    pub court_ref: Option<String>,
    pub decision_or_effective_date: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub source_role: SourceRole,
    pub authority_level: AuthorityLevel,
    pub source_citation: String,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractTraceEdge {
    pub from_ref: String,
    pub to_ref: String,
    pub treatment: TreatmentKind,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractFollowWorkClass {
    LivePrimarySourceResidual,
    AuthorityTreatmentResidual,
    ExternalIdentityMetadata,
    ContextNavigation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractFollowParetoCoordinate {
    pub work_class: ContractFollowWorkClass,
    pub closes_live_legal_residual: bool,
    pub preserves_exact_source_revision: bool,
    pub enriches_external_identity: bool,
    pub network_cost_hint: u16,
    pub scalar_rank_is_legal_truth_rank: bool,
}

pub fn primary_source_pareto_coordinate() -> ContractFollowParetoCoordinate {
    ContractFollowParetoCoordinate {
        work_class: ContractFollowWorkClass::LivePrimarySourceResidual,
        closes_live_legal_residual: true,
        preserves_exact_source_revision: true,
        enriches_external_identity: false,
        network_cost_hint: 1,
        scalar_rank_is_legal_truth_rank: false,
    }
}

pub fn external_identity_pareto_coordinate() -> ContractFollowParetoCoordinate {
    ContractFollowParetoCoordinate {
        work_class: ContractFollowWorkClass::ExternalIdentityMetadata,
        closes_live_legal_residual: false,
        preserves_exact_source_revision: false,
        enriches_external_identity: true,
        network_cost_hint: 1,
        scalar_rank_is_legal_truth_rank: false,
    }
}

pub fn primary_source_precedes_optional_identity_by_default() -> bool {
    let source = primary_source_pareto_coordinate();
    let identity = external_identity_pareto_coordinate();
    source.closes_live_legal_residual
        && source.preserves_exact_source_revision
        && !identity.closes_live_legal_residual
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalIdentityLookupPriority {
    NotApplicable,
    Opportunistic,
    WorthChecking,
    HighValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalIdentityKind {
    WikidataQid,
    CanonicalUrl,
    OfficialIdentifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalIdentityStatus {
    Candidate,
    Verified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalIdentityAttachment {
    pub semantic_ref: String,
    pub kind: ExternalIdentityKind,
    pub value: String,
    pub status: ExternalIdentityStatus,
    pub verification_ref: String,
    pub supplemental_only: bool,
    pub creates_legal_authority: bool,
    pub creates_applicability: bool,
}

impl ExternalIdentityAttachment {
    pub fn validate_for(&self, node: &ContractTraceNode) -> Result<(), String> {
        if self.semantic_ref != node.semantic_ref {
            return Err("external identity attached to the wrong semantic object".into());
        }
        if self.value.trim().is_empty() || self.verification_ref.trim().is_empty() {
            return Err("external identity requires non-empty value and verification reference".into());
        }
        if !self.supplemental_only
            || self.creates_legal_authority
            || self.creates_applicability
        {
            return Err("external identity crossed the legal authority/applicability boundary".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalIdentityLookupHint {
    pub semantic_ref: String,
    pub wikidata_qid_priority: ExternalIdentityLookupPriority,
    pub canonical_url_priority: ExternalIdentityLookupPriority,
    pub lookup_is_existence_claim: bool,
    pub lookup_creates_legal_authority: bool,
}

pub fn external_identity_lookup_hint(node: &ContractTraceNode) -> ExternalIdentityLookupHint {
    use ExternalIdentityLookupPriority::*;
    let qid = match node.kind {
        TraceNodeKind::ResearchRequirement => NotApplicable,
        TraceNodeKind::Doctrine => HighValue,
        TraceNodeKind::CaseAuthority | TraceNodeKind::Matter => {
            if node.court_ref.as_deref() == Some("court:HCA") {
                WorthChecking
            } else {
                Opportunistic
            }
        }
        TraceNodeKind::Legislation => Opportunistic,
    };
    let canonical = match node.kind {
        TraceNodeKind::ResearchRequirement => NotApplicable,
        TraceNodeKind::CaseAuthority | TraceNodeKind::Matter | TraceNodeKind::Legislation => HighValue,
        TraceNodeKind::Doctrine => WorthChecking,
    };
    ExternalIdentityLookupHint {
        semantic_ref: node.semantic_ref.clone(),
        wikidata_qid_priority: qid,
        canonical_url_priority: canonical,
        lookup_is_existence_claim: false,
        lookup_creates_legal_authority: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AustralianContractTrace {
    pub root_ref: String,
    pub nodes: BTreeMap<String, ContractTraceNode>,
    pub edges: Vec<ContractTraceEdge>,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

impl AustralianContractTrace {
    pub fn validate(&self) -> Result<(), String> {
        if !self.candidate_only || self.creates_legal_authority {
            return Err("contract trace crossed authority boundary".into());
        }
        if !self.nodes.contains_key(&self.root_ref) {
            return Err("contract trace root is absent".into());
        }
        for (key, node) in &self.nodes {
            if key != &node.semantic_ref || key.trim().is_empty() {
                return Err("contract trace contains unstable/empty semantic identity".into());
            }
            if node.jurisdiction_ref.trim().is_empty() || node.source_citation.trim().is_empty() {
                return Err(format!("{} lost jurisdiction/source citation", node.semantic_ref));
            }
            if !node.candidate_only || node.creates_legal_authority {
                return Err(format!("{} crossed candidate-only authority boundary", node.semantic_ref));
            }
        }
        for edge in &self.edges {
            if !self.nodes.contains_key(&edge.from_ref) || !self.nodes.contains_key(&edge.to_ref) {
                return Err(format!("trace edge references unknown node: {} -> {}", edge.from_ref, edge.to_ref));
            }
            if !edge.candidate_only || edge.creates_legal_authority {
                return Err("trace edge crossed authority boundary".into());
            }
        }
        Ok(())
    }

    pub fn outgoing(&self, semantic_ref: &str) -> Vec<&ContractTraceEdge> {
        self.edges.iter().filter(|edge| edge.from_ref == semantic_ref).collect()
    }

    pub fn reachable(&self, seed: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut frontier = vec![seed.to_owned()];
        while let Some(current) = frontier.pop() {
            for edge in self.outgoing(&current) {
                if seen.insert(edge.to_ref.clone()) {
                    frontier.push(edge.to_ref.clone());
                }
            }
        }
        seen
    }

    pub fn active_at(&self, semantic_ref: &str, as_at: &str) -> bool {
        let Some(node) = self.nodes.get(semantic_ref) else { return false; };
        if let Some(from) = &node.valid_from {
            if as_at < from { return false; }
        }
        if let Some(to) = &node.valid_to {
            if as_at > to { return false; }
        }
        true
    }
}

fn node(
    semantic_ref: &str,
    label: &str,
    kind: TraceNodeKind,
    doctrine: Option<ContractDoctrine>,
    jurisdiction_ref: &str,
    court_ref: Option<&str>,
    date: Option<&str>,
    valid_from: Option<&str>,
    valid_to: Option<&str>,
    source_role: SourceRole,
    authority_level: AuthorityLevel,
    source_citation: &str,
) -> ContractTraceNode {
    ContractTraceNode {
        semantic_ref: semantic_ref.into(),
        label: label.into(),
        kind,
        doctrine,
        jurisdiction_ref: jurisdiction_ref.into(),
        court_ref: court_ref.map(str::to_owned),
        decision_or_effective_date: date.map(str::to_owned),
        valid_from: valid_from.map(str::to_owned),
        valid_to: valid_to.map(str::to_owned),
        source_role,
        authority_level,
        source_citation: source_citation.into(),
        candidate_only: true,
        creates_legal_authority: false,
    }
}

fn edge(from: &str, to: &str, treatment: TreatmentKind) -> ContractTraceEdge {
    ContractTraceEdge {
        from_ref: from.into(),
        to_ref: to.into(),
        treatment,
        candidate_only: true,
        creates_legal_authority: false,
    }
}

/// Narrow S8a trace.  The requirement nodes are research coordinates, not a
/// declaration that Australian equitable estoppel has a mechanically complete
/// closed list of elements.
pub fn waltons_estoppel_trace() -> AustralianContractTrace {
    let root = "doctrine:au:contract:estoppel";
    let mut nodes = BTreeMap::new();
    for n in [
        node(root, "Australian equitable/promissory estoppel", TraceNodeKind::Doctrine,
            Some(ContractDoctrine::Estoppel), "AU", None, None, None, None,
            SourceRole::ResearchIndex, AuthorityLevel::Supporting, "doctrine-query:estoppel"),
        node("case:au:hca:1988:7", "Waltons Stores (Interstate) Ltd v Maher", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Estoppel), "AU", Some("court:HCA"), Some("1988-02-19"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[1988] HCA 7; 164 CLR 387"),
        node("requirement:estoppel:assumption", "assumption / expectation induced or adopted", TraceNodeKind::ResearchRequirement,
            Some(ContractDoctrine::Estoppel), "AU", None, None, None, None,
            SourceRole::ResearchIndex, AuthorityLevel::Supporting, "research-requirement:estoppel:assumption"),
        node("requirement:estoppel:reliance", "reliance / action on the assumption", TraceNodeKind::ResearchRequirement,
            Some(ContractDoctrine::Estoppel), "AU", None, None, None, None,
            SourceRole::ResearchIndex, AuthorityLevel::Supporting, "research-requirement:estoppel:reliance"),
        node("requirement:estoppel:detriment", "detriment if the assumption is departed from", TraceNodeKind::ResearchRequirement,
            Some(ContractDoctrine::Estoppel), "AU", None, None, None, None,
            SourceRole::ResearchIndex, AuthorityLevel::Supporting, "research-requirement:estoppel:detriment"),
        node("requirement:estoppel:unconscionability", "unconscionability / equitable restraint question", TraceNodeKind::ResearchRequirement,
            Some(ContractDoctrine::Estoppel), "AU", None, None, None, None,
            SourceRole::ResearchIndex, AuthorityLevel::Supporting, "research-requirement:estoppel:unconscionability"),
        node("case:au:hca:2014:19", "Sidhu v Van Dyke", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Estoppel), "AU", Some("court:HCA"), Some("2014-05-16"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[2014] HCA 19; 251 CLR 505"),
        node("case:au:hca:2016:26", "Crown Melbourne Ltd v Cosmopolitan Hotel (Vic) Pty Ltd", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Estoppel), "AU", Some("court:HCA"), Some("2016-07-20"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[2016] HCA 26; 260 CLR 1"),
    ] {
        nodes.insert(n.semantic_ref.clone(), n);
    }
    let edges = vec![
        edge(root, "case:au:hca:1988:7", TreatmentKind::Seeds),
        edge("case:au:hca:1988:7", "requirement:estoppel:assumption", TreatmentKind::Requires),
        edge("case:au:hca:1988:7", "requirement:estoppel:reliance", TreatmentKind::Requires),
        edge("case:au:hca:1988:7", "requirement:estoppel:detriment", TreatmentKind::Requires),
        edge("case:au:hca:1988:7", "requirement:estoppel:unconscionability", TreatmentKind::Requires),
        edge("case:au:hca:1988:7", "case:au:hca:2014:19", TreatmentKind::Follows),
        edge("case:au:hca:1988:7", "case:au:hca:2016:26", TreatmentKind::Qualifies),
    ];
    let trace = AustralianContractTrace {
        root_ref: root.into(), nodes, edges, candidate_only: true, creates_legal_authority: false,
    };
    trace.validate().expect("static Waltons trace is structurally valid");
    trace
}

/// S8b unseen-matter trace.  This intentionally crosses common law and a
/// Victorian statutory overlay without changing the generic runtime.
pub fn mann_paterson_trace() -> AustralianContractTrace {
    let root = "matter:au:hca:2019:32";
    let mut nodes = BTreeMap::new();
    for n in [
        node(root, "Mann v Paterson Constructions Pty Ltd", TraceNodeKind::Matter,
            Some(ContractDoctrine::Restitution), "AU", Some("court:HCA"), Some("2019-10-09"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[2019] HCA 32"),
        node("doctrine:au:contract:repudiation-termination", "repudiation and termination", TraceNodeKind::Doctrine,
            Some(ContractDoctrine::RepudiationAndTermination), "AU", None, None, None, None,
            SourceRole::ResearchIndex, AuthorityLevel::Supporting, "doctrine-query:repudiation-termination"),
        node("doctrine:au:contract:restitution-after-termination", "restitution / quantum meruit after termination", TraceNodeKind::Doctrine,
            Some(ContractDoctrine::Restitution), "AU", None, None, None, None,
            SourceRole::ResearchIndex, AuthorityLevel::Supporting, "doctrine-query:restitution-after-termination"),
        node("legislation:vic:domestic-building-contracts-act-1995:s38",
            "Domestic Building Contracts Act 1995 (Vic) s 38", TraceNodeKind::Legislation,
            Some(ContractDoctrine::Restitution), "AU-VIC", None, None, None, None,
            SourceRole::PrimaryLegislation, AuthorityLevel::Official,
            "Domestic Building Contracts Act 1995 (Vic) s 38"),
        node("case:vic:vsca:2018:231", "Paterson Constructions Pty Ltd v Mann", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Restitution), "AU-VIC", Some("court:VSCA"), Some("2018-09-12"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[2018] VSCA 231"),
    ] {
        nodes.insert(n.semantic_ref.clone(), n);
    }
    let edges = vec![
        edge(root, "doctrine:au:contract:repudiation-termination", TreatmentKind::Requires),
        edge(root, "doctrine:au:contract:restitution-after-termination", TreatmentKind::Requires),
        edge(root, "legislation:vic:domestic-building-contracts-act-1995:s38", TreatmentKind::Intersects),
        edge("case:vic:vsca:2018:231", root, TreatmentKind::Requires),
    ];
    let trace = AustralianContractTrace {
        root_ref: root.into(), nodes, edges, candidate_only: true, creates_legal_authority: false,
    };
    trace.validate().expect("static Mann trace is structurally valid");
    trace
}

/// A bounded seed for the later S14 reconstruction benchmark.  It intentionally
/// contains only enough landmarks to force LegalFollow to branch by doctrine,
/// jurisdiction, and time; it is not represented as a complete statement of
/// Australian contract law.
pub fn australian_contract_landscape_seed() -> AustralianContractTrace {
    let root = "landscape:au:contract-law";
    let mut nodes = BTreeMap::new();
    let seeds = [
        node(root, "Australian contract-law reconstruction target", TraceNodeKind::Doctrine,
            None, "AU", None, None, None, None, SourceRole::ResearchIndex,
            AuthorityLevel::Supporting, "research-target:Australian-contract-law"),
        node("case:au:hca:1954:20", "Australian Woollen Mills Pty Ltd v Commonwealth", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Formation), "AU", Some("court:HCA"), Some("1954-09-30"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[1954] HCA 20; 92 CLR 424"),
        node("case:au:hca:1954:72", "Masters v Cameron", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Formation), "AU", Some("court:HCA"), Some("1954-11-30"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[1954] HCA 72; 91 CLR 353"),
        node("case:au:hca:2002:8", "Ermogenous v Greek Orthodox Community of SA Inc", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Intention), "AU", Some("court:HCA"), Some("2002-03-14"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[2002] HCA 8; 209 CLR 95"),
        node("case:au:hca:2004:52", "Toll (FGCT) Pty Ltd v Alphapharm Pty Ltd", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::TermsAndIncorporation), "AU", Some("court:HCA"), Some("2004-11-11"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[2004] HCA 52; 219 CLR 165"),
        node("case:au:hca:1988:7", "Waltons Stores (Interstate) Ltd v Maher", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Estoppel), "AU", Some("court:HCA"), Some("1988-02-19"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[1988] HCA 7; 164 CLR 387"),
        node("case:au:hca:2007:61", "Koompahtoo Local Aboriginal Land Council v Sanpine Pty Ltd", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::RepudiationAndTermination), "AU", Some("court:HCA"), Some("2007-12-13"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[2007] HCA 61; 233 CLR 115"),
        node("case:au:hca:1991:54", "Commonwealth v Amann Aviation Pty Ltd", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Damages), "AU", Some("court:HCA"), Some("1991-12-12"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[1991] HCA 54; 174 CLR 64"),
        node("case:au:hca:2019:32", "Mann v Paterson Constructions Pty Ltd", TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Restitution), "AU", Some("court:HCA"), Some("2019-10-09"), None, None,
            SourceRole::PrimaryCaseLaw, AuthorityLevel::Official, "[2019] HCA 32"),
        node("legislation:qld:property-law-act-1974:s55", "Property Law Act 1974 (Qld) s 55",
            TraceNodeKind::Legislation, Some(ContractDoctrine::Privity), "AU-QLD", None, None,
            None, Some("2025-07-31"), SourceRole::PrimaryLegislation, AuthorityLevel::Official,
            "Property Law Act 1974 (Qld) s 55"),
        node("legislation:qld:property-law-act-2023:s68", "Property Law Act 2023 (Qld) s 68",
            TraceNodeKind::Legislation, Some(ContractDoctrine::Privity), "AU-QLD", None, Some("2025-08-01"),
            Some("2025-08-01"), None, SourceRole::PrimaryLegislation, AuthorityLevel::Official,
            "Property Law Act 2023 (Qld) s 68"),
    ];
    for n in seeds { nodes.insert(n.semantic_ref.clone(), n); }
    let mut edges = Vec::new();
    for key in nodes.keys().filter(|key| key.as_str() != root) {
        edges.push(edge(root, key, TreatmentKind::Seeds));
    }
    edges.push(edge(
        "legislation:qld:property-law-act-1974:s55",
        "legislation:qld:property-law-act-2023:s68",
        TreatmentKind::TemporalSuccessor,
    ));
    let trace = AustralianContractTrace {
        root_ref: root.into(), nodes, edges, candidate_only: true, creates_legal_authority: false,
    };
    trace.validate().expect("static Australian contract seed is structurally valid");
    trace
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractLandscapeWorkKind {
    AcquirePrimarySource,
    ReviewAuthorityTreatment,
    ExpandResearchContext,
    RetainTemporalAlternative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractLandscapeWorkItem {
    pub work_ref: String,
    pub kind: ContractLandscapeWorkKind,
    pub semantic_ref: String,
    pub related_ref: Option<String>,
    pub doctrine: Option<ContractDoctrine>,
    pub jurisdiction_ref: String,
    pub as_at: String,
    pub source_role: SourceRole,
    pub source_citation: String,
    pub court_ref: Option<String>,
    pub treatment: Option<TreatmentKind>,
    pub active_at_as_at: bool,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AustralianContractLandscapeWorklist {
    pub root_ref: String,
    pub as_at: String,
    pub jurisdiction_filter: Option<String>,
    pub source_items: Vec<ContractLandscapeWorkItem>,
    pub treatment_items: Vec<ContractLandscapeWorkItem>,
    pub context_items: Vec<ContractLandscapeWorkItem>,
    pub temporal_alternatives: Vec<ContractLandscapeWorkItem>,
    pub bounded_seed_only: bool,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

const ALL_CONTRACT_DOCTRINES: [ContractDoctrine; 12] = [
    ContractDoctrine::Formation,
    ContractDoctrine::Intention,
    ContractDoctrine::TermsAndIncorporation,
    ContractDoctrine::Construction,
    ContractDoctrine::Estoppel,
    ContractDoctrine::Unconscionability,
    ContractDoctrine::Penalties,
    ContractDoctrine::RepudiationAndTermination,
    ContractDoctrine::Damages,
    ContractDoctrine::Restitution,
    ContractDoctrine::Privity,
    ContractDoctrine::ConsumerLaw,
];

fn doctrine_slug(doctrine: ContractDoctrine) -> &'static str {
    match doctrine {
        ContractDoctrine::Formation => "formation",
        ContractDoctrine::Intention => "intention",
        ContractDoctrine::TermsAndIncorporation => "terms-and-incorporation",
        ContractDoctrine::Construction => "construction",
        ContractDoctrine::Estoppel => "estoppel",
        ContractDoctrine::Unconscionability => "unconscionability",
        ContractDoctrine::Penalties => "penalties",
        ContractDoctrine::RepudiationAndTermination => "repudiation-and-termination",
        ContractDoctrine::Damages => "damages",
        ContractDoctrine::Restitution => "restitution",
        ContractDoctrine::Privity => "privity",
        ContractDoctrine::ConsumerLaw => "consumer-law",
    }
}

fn jurisdiction_matches(filter: Option<&str>, node_jurisdiction: &str) -> bool {
    match filter {
        None => true,
        Some("AU") => true,
        Some(filter) => node_jurisdiction == filter || node_jurisdiction == "AU",
    }
}

pub fn compile_australian_contract_landscape_worklist(
    trace: &AustralianContractTrace,
    as_at: &str,
    jurisdiction_filter: Option<&str>,
) -> Result<AustralianContractLandscapeWorklist, String> {
    trace.validate()?;
    if as_at.trim().is_empty() {
        return Err("contract landscape worklist requires an as-at date".into());
    }

    let mut source_items = Vec::new();
    let mut treatment_items = Vec::new();
    let mut context_items = Vec::new();
    let mut temporal_alternatives = Vec::new();

    for node in trace.nodes.values() {
        if node.semantic_ref == trace.root_ref {
            continue;
        }
        if !jurisdiction_matches(jurisdiction_filter, &node.jurisdiction_ref) {
            continue;
        }

        let active = trace.active_at(&node.semantic_ref, as_at);
        let base = ContractLandscapeWorkItem {
            work_ref: format!("contracts:landscape:source:{}", node.semantic_ref),
            kind: ContractLandscapeWorkKind::AcquirePrimarySource,
            semantic_ref: node.semantic_ref.clone(),
            related_ref: None,
            doctrine: node.doctrine,
            jurisdiction_ref: node.jurisdiction_ref.clone(),
            as_at: as_at.to_string(),
            source_role: node.source_role,
            source_citation: node.source_citation.clone(),
            court_ref: node.court_ref.clone(),
            treatment: None,
            active_at_as_at: active,
            candidate_only: true,
            creates_legal_authority: false,
            creates_current_law_conclusion: false,
        };

        if !active {
            let mut item = base;
            item.work_ref = format!("contracts:landscape:temporal:{}", node.semantic_ref);
            item.kind = ContractLandscapeWorkKind::RetainTemporalAlternative;
            temporal_alternatives.push(item);
            continue;
        }

        match node.source_role {
            SourceRole::PrimaryCaseLaw | SourceRole::PrimaryLegislation => {
                source_items.push(base);
            }
            SourceRole::OfficialRecord
            | SourceRole::ResearchIndex
            | SourceRole::SecondaryAnalysis => {
                let mut item = base;
                item.work_ref = format!("contracts:landscape:context:{}", node.semantic_ref);
                item.kind = ContractLandscapeWorkKind::ExpandResearchContext;
                context_items.push(item);
            }
        }
    }

    for (index, edge) in trace.edges.iter().enumerate() {
        let Some(from) = trace.nodes.get(&edge.from_ref) else { continue };
        let Some(to) = trace.nodes.get(&edge.to_ref) else { continue };
        if !jurisdiction_matches(jurisdiction_filter, &from.jurisdiction_ref)
            && !jurisdiction_matches(jurisdiction_filter, &to.jurisdiction_ref)
        {
            continue;
        }
        if !trace.active_at(&from.semantic_ref, as_at)
            || !trace.active_at(&to.semantic_ref, as_at)
        {
            continue;
        }
        treatment_items.push(ContractLandscapeWorkItem {
            work_ref: format!("contracts:landscape:treatment:{index}:{}:{}", edge.from_ref, edge.to_ref),
            kind: ContractLandscapeWorkKind::ReviewAuthorityTreatment,
            semantic_ref: edge.from_ref.clone(),
            related_ref: Some(edge.to_ref.clone()),
            doctrine: from.doctrine.or(to.doctrine),
            jurisdiction_ref: if from.jurisdiction_ref == "AU" {
                to.jurisdiction_ref.clone()
            } else {
                from.jurisdiction_ref.clone()
            },
            as_at: as_at.to_string(),
            source_role: from.source_role,
            source_citation: from.source_citation.clone(),
            court_ref: from.court_ref.clone(),
            treatment: Some(edge.treatment),
            active_at_as_at: true,
            candidate_only: true,
            creates_legal_authority: false,
            creates_current_law_conclusion: false,
        });
    }

    let represented_doctrines = trace
        .nodes
        .values()
        .filter_map(|node| node.doctrine)
        .collect::<BTreeSet<_>>();
    for doctrine in ALL_CONTRACT_DOCTRINES {
        if represented_doctrines.contains(&doctrine) {
            continue;
        }
        let slug = doctrine_slug(doctrine);
        context_items.push(ContractLandscapeWorkItem {
            work_ref: format!("contracts:landscape:context:doctrine:{slug}"),
            kind: ContractLandscapeWorkKind::ExpandResearchContext,
            semantic_ref: format!("doctrine:au:contract:{slug}"),
            related_ref: Some(trace.root_ref.clone()),
            doctrine: Some(doctrine),
            jurisdiction_ref: jurisdiction_filter.unwrap_or("AU").to_string(),
            as_at: as_at.to_string(),
            source_role: SourceRole::ResearchIndex,
            source_citation: format!("doctrine-query:{slug}"),
            court_ref: None,
            treatment: None,
            active_at_as_at: true,
            candidate_only: true,
            creates_legal_authority: false,
            creates_current_law_conclusion: false,
        });
    }

    source_items.sort_by(|left, right| left.semantic_ref.cmp(&right.semantic_ref));
    treatment_items.sort_by(|left, right| left.work_ref.cmp(&right.work_ref));
    context_items.sort_by(|left, right| left.semantic_ref.cmp(&right.semantic_ref));
    temporal_alternatives.sort_by(|left, right| left.semantic_ref.cmp(&right.semantic_ref));

    Ok(AustralianContractLandscapeWorklist {
        root_ref: trace.root_ref.clone(),
        as_at: as_at.to_string(),
        jurisdiction_filter: jurisdiction_filter.map(str::to_owned),
        source_items,
        treatment_items,
        context_items,
        temporal_alternatives,
        bounded_seed_only: true,
        candidate_only: true,
        creates_legal_authority: false,
        creates_current_law_conclusion: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractLandscapeExpansionDelta {
    pub discovered_nodes: Vec<ContractTraceNode>,
    pub discovered_edges: Vec<ContractTraceEdge>,
    pub provenance_ref: String,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractLandscapeExpansionReceipt {
    pub provenance_ref: String,
    pub added_node_count: usize,
    pub added_edge_count: usize,
    pub recompute_frontier_required: bool,
    pub old_source_history_preserved: bool,
    pub old_conclusions_frozen: bool,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

pub fn trace_extension_delta(
    base: &AustralianContractTrace,
    extension: &AustralianContractTrace,
    provenance_ref: impl Into<String>,
) -> Result<ContractLandscapeExpansionDelta, String> {
    base.validate()?;
    extension.validate()?;
    let provenance_ref = provenance_ref.into();
    if provenance_ref.trim().is_empty() {
        return Err("trace extension requires provenance".into());
    }

    let mut discovered_nodes = Vec::new();
    for node in extension.nodes.values() {
        match base.nodes.get(&node.semantic_ref) {
            Some(existing) if existing == node => {}
            Some(existing) => {
                let compatible = existing.kind == node.kind
                    && existing.doctrine == node.doctrine
                    && existing.jurisdiction_ref == node.jurisdiction_ref
                    && existing.source_role == node.source_role
                    && existing.authority_level == node.authority_level;
                if !compatible {
                    return Err(format!(
                        "trace extension conflicts with existing semantic identity: {}",
                        node.semantic_ref
                    ));
                }
            }
            None => discovered_nodes.push(node.clone()),
        }
    }

    let discovered_edges = extension
        .edges
        .iter()
        .filter(|edge| !base.edges.contains(edge))
        .cloned()
        .collect();

    Ok(ContractLandscapeExpansionDelta {
        discovered_nodes,
        discovered_edges,
        provenance_ref,
        candidate_only: true,
        creates_legal_authority: false,
    })
}

pub fn apply_contract_landscape_expansion(
    trace: &AustralianContractTrace,
    delta: &ContractLandscapeExpansionDelta,
) -> Result<(AustralianContractTrace, ContractLandscapeExpansionReceipt), String> {
    trace.validate()?;
    if delta.provenance_ref.trim().is_empty() {
        return Err("contract landscape expansion requires provenance".into());
    }
    if !delta.candidate_only || delta.creates_legal_authority {
        return Err("contract landscape expansion crossed authority boundary".into());
    }

    let mut expanded = trace.clone();
    let mut added_node_count = 0usize;
    for node in &delta.discovered_nodes {
        if !node.candidate_only || node.creates_legal_authority {
            return Err(format!(
                "discovered node {} crossed candidate-only boundary",
                node.semantic_ref
            ));
        }
        match expanded.nodes.get(&node.semantic_ref) {
            Some(existing) if existing == node => {}
            Some(_) => {
                return Err(format!(
                    "discovered node conflicts with existing semantic identity: {}",
                    node.semantic_ref
                ))
            }
            None => {
                expanded.nodes.insert(node.semantic_ref.clone(), node.clone());
                added_node_count += 1;
            }
        }
    }

    let mut added_edge_count = 0usize;
    for edge in &delta.discovered_edges {
        if !edge.candidate_only || edge.creates_legal_authority {
            return Err("discovered edge crossed candidate-only boundary".into());
        }
        if !expanded.nodes.contains_key(&edge.from_ref)
            || !expanded.nodes.contains_key(&edge.to_ref)
        {
            return Err(format!(
                "discovered edge references unresolved semantic identity: {} -> {}",
                edge.from_ref, edge.to_ref
            ));
        }
        if !expanded.edges.contains(edge) {
            expanded.edges.push(edge.clone());
            added_edge_count += 1;
        }
    }
    expanded.validate()?;

    Ok((
        expanded,
        ContractLandscapeExpansionReceipt {
            provenance_ref: delta.provenance_ref.clone(),
            added_node_count,
            added_edge_count,
            recompute_frontier_required: true,
            old_source_history_preserved: true,
            old_conclusions_frozen: false,
            candidate_only: true,
            creates_legal_authority: false,
            creates_current_law_conclusion: false,
        },
    ))
}

pub fn legal_follow_demand_for_trace_node(
    node: &ContractTraceNode,
    as_at: &str,
) -> Option<LegalSourceDemand> {
    let requested_facets = match node.source_role {
        SourceRole::PrimaryCaseLaw => vec![
            "case.full_text".into(),
            "case.citation_graph".into(),
            "case.treatment".into(),
        ],
        SourceRole::PrimaryLegislation => vec![
            "legislation.text".into(),
            "legislation.version_history".into(),
        ],
        SourceRole::OfficialRecord | SourceRole::ResearchIndex | SourceRole::SecondaryAnalysis => {
            return None;
        }
    };
    Some(LegalSourceDemand {
        demand_ref: format!("contract-follow:{}", node.semantic_ref),
        origin_ref: node.semantic_ref.clone(),
        jurisdiction_ref: Some(node.jurisdiction_ref.clone()),
        source_roles: vec![node.source_role],
        authority_levels: vec![node.authority_level],
        provider_profile_refs: vec![OALC_PROVIDER_PROFILE.into()],
        requested_facets,
        temporal_refs: vec![format!("as_at:{as_at}")],
        provenance_refs: vec!["australian-contracts-follow:v1".into()],
        priority: 100,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactCaseLawSourceDemand {
    pub demand_ref: String,
    pub origin_ref: String,
    pub jurisdiction_ref: String,
    pub citation: String,
    pub court_ref: String,
    pub source_role: SourceRole,
    pub authority_level: AuthorityLevel,
    pub provider_profile_ref: String,
    pub dataset_ref: String,
    pub requested_temporal_ref: Option<String>,
    pub authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactCaseLawDemandError {
    NotAcquisitionReady,
    WrongSourceRole,
    WrongAuthorityLevel,
    ProviderNotAdmitted,
    EmptyCitation,
    EmptyCourt,
}

pub fn exact_oalc_case_law_demand(
    plan: &LegalSourcePlan,
    origin_ref: impl Into<String>,
    citation: impl Into<String>,
    court_ref: impl Into<String>,
) -> Result<ExactCaseLawSourceDemand, ExactCaseLawDemandError> {
    if !matches!(plan.state, PlanState::BlockedAcquisitionRequired | PlanState::ReadyPersisted) {
        return Err(ExactCaseLawDemandError::NotAcquisitionReady);
    }
    let Some(jurisdiction_ref) = plan.jurisdiction_ref.clone() else {
        return Err(ExactCaseLawDemandError::NotAcquisitionReady);
    };
    if !plan.source_roles.contains(&SourceRole::PrimaryCaseLaw) {
        return Err(ExactCaseLawDemandError::WrongSourceRole);
    }
    if !plan.authority_levels.contains(&AuthorityLevel::Official) {
        return Err(ExactCaseLawDemandError::WrongAuthorityLevel);
    }
    if !plan.provider_profile_refs.is_empty()
        && !plan.provider_profile_refs.iter().any(|p| p == OALC_PROVIDER_PROFILE)
    {
        return Err(ExactCaseLawDemandError::ProviderNotAdmitted);
    }
    let citation = citation.into();
    if citation.trim().is_empty() { return Err(ExactCaseLawDemandError::EmptyCitation); }
    let court_ref = court_ref.into();
    if court_ref.trim().is_empty() { return Err(ExactCaseLawDemandError::EmptyCourt); }
    Ok(ExactCaseLawSourceDemand {
        demand_ref: plan.demand_ref.clone(),
        origin_ref: origin_ref.into(),
        jurisdiction_ref,
        citation,
        court_ref,
        source_role: SourceRole::PrimaryCaseLaw,
        authority_level: AuthorityLevel::Official,
        provider_profile_ref: OALC_PROVIDER_PROFILE.into(),
        dataset_ref: OALC_DATASET_ID.into(),
        requested_temporal_ref: plan.temporal_refs.first().cloned(),
        authority: "acquisition_plan_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plan_legal_sources, LegalSourceDemand};

    #[test]
    fn waltons_trace_keeps_context_requirements_separate_from_authority() {
        let trace = waltons_estoppel_trace();
        assert_eq!(
            trace.nodes["case:au:hca:1988:7"].authority_level,
            AuthorityLevel::Official
        );
        assert_eq!(
            trace.nodes["requirement:estoppel:detriment"].source_role,
            SourceRole::ResearchIndex
        );
        assert!(!trace.creates_legal_authority);
    }

    #[test]
    fn mann_trace_crosses_hca_common_law_and_victorian_statute_without_new_runtime_family() {
        let trace = mann_paterson_trace();
        assert!(trace.nodes.contains_key("matter:au:hca:2019:32"));
        assert!(trace.nodes.contains_key("legislation:vic:domestic-building-contracts-act-1995:s38"));
        assert!(trace.edges.iter().any(|edge| edge.treatment == TreatmentKind::Intersects));
        assert!(!trace.creates_legal_authority);
    }

    #[test]
    fn queensland_privity_seed_is_temporally_sliced() {
        let trace = australian_contract_landscape_seed();
        assert!(trace.active_at("legislation:qld:property-law-act-1974:s55", "2025-07-31"));
        assert!(!trace.active_at("legislation:qld:property-law-act-1974:s55", "2025-08-01"));
        assert!(!trace.active_at("legislation:qld:property-law-act-2023:s68", "2025-07-31"));
        assert!(trace.active_at("legislation:qld:property-law-act-2023:s68", "2025-08-01"));
    }

    #[test]
    fn live_primary_source_residual_precedes_optional_qid_cleanup_by_default() {
        assert!(primary_source_precedes_optional_identity_by_default());
        assert!(!primary_source_pareto_coordinate().scalar_rank_is_legal_truth_rank);
        assert!(!external_identity_pareto_coordinate().scalar_rank_is_legal_truth_rank);
    }

    #[test]
    fn verified_qid_attachment_is_supplemental_to_legal_source_identity() {
        let trace = waltons_estoppel_trace();
        let waltons = &trace.nodes["case:au:hca:1988:7"];
        let attachment = ExternalIdentityAttachment {
            semantic_ref: waltons.semantic_ref.clone(),
            kind: ExternalIdentityKind::WikidataQid,
            value: "QID:fixture-not-a-claim".into(),
            status: ExternalIdentityStatus::Candidate,
            verification_ref: "wikidata-lookup:fixture".into(),
            supplemental_only: true,
            creates_legal_authority: false,
            creates_applicability: false,
        };
        assert!(attachment.validate_for(waltons).is_ok());
    }

    #[test]
    fn qid_lookup_priority_is_opportunistic_identity_not_existential_or_authoritative() {
        let waltons = waltons_estoppel_trace();
        let case = external_identity_lookup_hint(&waltons.nodes["case:au:hca:1988:7"]);
        assert_eq!(case.wikidata_qid_priority, ExternalIdentityLookupPriority::WorthChecking);
        assert_eq!(case.canonical_url_priority, ExternalIdentityLookupPriority::HighValue);
        assert!(!case.lookup_is_existence_claim);
        assert!(!case.lookup_creates_legal_authority);

        let requirement =
            external_identity_lookup_hint(&waltons.nodes["requirement:estoppel:detriment"]);
        assert_eq!(
            requirement.wikidata_qid_priority,
            ExternalIdentityLookupPriority::NotApplicable
        );
    }

    #[test]
    fn landscape_worklist_separates_source_treatment_context_and_temporal_frontiers() {
        let trace = australian_contract_landscape_seed();
        let work = compile_australian_contract_landscape_worklist(
            &trace,
            "2026-09-20",
            None,
        )
        .unwrap();

        assert!(work.source_items.iter().any(|item| {
            item.semantic_ref == "case:au:hca:1988:7"
                && item.kind == ContractLandscapeWorkKind::AcquirePrimarySource
        }));
        assert!(work.temporal_alternatives.iter().any(|item| {
            item.semantic_ref == "legislation:qld:property-law-act-1974:s55"
                && !item.active_at_as_at
        }));
        assert!(work.source_items.iter().any(|item| {
            item.semantic_ref == "legislation:qld:property-law-act-2023:s68"
                && item.active_at_as_at
        }));
        assert!(!work.treatment_items.is_empty());
        assert!(work.context_items.iter().any(|item| {
            item.doctrine == Some(ContractDoctrine::Construction)
                && item.kind == ContractLandscapeWorkKind::ExpandResearchContext
        }));
        assert!(work.context_items.iter().any(|item| {
            item.doctrine == Some(ContractDoctrine::ConsumerLaw)
        }));
        assert!(work.bounded_seed_only);
        assert!(work.candidate_only);
        assert!(!work.creates_legal_authority);
        assert!(!work.creates_current_law_conclusion);
    }

    #[test]
    fn qld_landscape_filter_keeps_national_authorities_and_qld_temporal_branch() {
        let trace = australian_contract_landscape_seed();
        let work = compile_australian_contract_landscape_worklist(
            &trace,
            "2026-09-20",
            Some("AU-QLD"),
        )
        .unwrap();

        assert!(work.source_items.iter().any(|item| {
            item.jurisdiction_ref == "AU" && item.source_role == SourceRole::PrimaryCaseLaw
        }));
        assert!(work.source_items.iter().any(|item| {
            item.semantic_ref == "legislation:qld:property-law-act-2023:s68"
        }));
        assert!(work.temporal_alternatives.iter().any(|item| {
            item.semantic_ref == "legislation:qld:property-law-act-1974:s55"
        }));
    }

    #[test]
    fn waltons_trace_extension_adds_requirements_without_replacing_seeded_authority() {
        let landscape = australian_contract_landscape_seed();
        let waltons = waltons_estoppel_trace();
        let delta = trace_extension_delta(
            &landscape,
            &waltons,
            "bootstrap:waltons-estoppel-materialisation",
        )
        .unwrap();
        assert!(delta.discovered_nodes.iter().any(|node| {
            node.semantic_ref == "requirement:estoppel:reliance"
        }));
        assert!(!delta.discovered_nodes.iter().any(|node| {
            node.semantic_ref == "case:au:hca:1988:7"
        }));
        let (expanded, _) = apply_contract_landscape_expansion(&landscape, &delta).unwrap();
        assert!(expanded.nodes.contains_key("requirement:estoppel:detriment"));
        assert!(expanded.nodes.contains_key("case:au:hca:1988:7"));
    }

    #[test]
    fn reviewed_expansion_candidate_recomputes_missing_doctrine_frontier() {
        let trace = australian_contract_landscape_seed();
        let before = compile_australian_contract_landscape_worklist(
            &trace,
            "2026-09-20",
            None,
        )
        .unwrap();
        assert!(before.context_items.iter().any(|item| {
            item.doctrine == Some(ContractDoctrine::Construction)
        }));

        let construction = node(
            "case:fixture:construction",
            "fixture construction authority candidate",
            TraceNodeKind::CaseAuthority,
            Some(ContractDoctrine::Construction),
            "AU",
            Some("court:fixture"),
            Some("2000-01-01"),
            None,
            None,
            SourceRole::PrimaryCaseLaw,
            AuthorityLevel::Official,
            "fixture:construction-primary-case",
        );
        let delta = ContractLandscapeExpansionDelta {
            discovered_nodes: vec![construction],
            discovered_edges: vec![edge(
                "landscape:au:contract-law",
                "case:fixture:construction",
                TreatmentKind::Seeds,
            )],
            provenance_ref: "reviewed-hop:fixture:construction".into(),
            candidate_only: true,
            creates_legal_authority: false,
        };
        let (expanded, receipt) =
            apply_contract_landscape_expansion(&trace, &delta).unwrap();
        let after = compile_australian_contract_landscape_worklist(
            &expanded,
            "2026-09-20",
            None,
        )
        .unwrap();

        assert!(!after.context_items.iter().any(|item| {
            item.doctrine == Some(ContractDoctrine::Construction)
        }));
        assert!(after.source_items.iter().any(|item| {
            item.semantic_ref == "case:fixture:construction"
        }));
        assert!(receipt.recompute_frontier_required);
        assert!(receipt.old_source_history_preserved);
        assert!(!receipt.old_conclusions_frozen);
        assert!(!receipt.creates_legal_authority);
        assert!(!receipt.creates_current_law_conclusion);
    }

    #[test]
    fn trace_nodes_compile_into_existing_legal_follow_demands() {
        let waltons = waltons_estoppel_trace();
        let waltons_node = &waltons.nodes["case:au:hca:1988:7"];
        let case_demand = legal_follow_demand_for_trace_node(waltons_node, "2026-09-20").unwrap();
        assert_eq!(case_demand.source_roles, vec![SourceRole::PrimaryCaseLaw]);
        assert!(case_demand.requested_facets.contains(&"case.citation_graph".to_string()));

        let landscape = australian_contract_landscape_seed();
        let qld = &landscape.nodes["legislation:qld:property-law-act-2023:s68"];
        let statute_demand = legal_follow_demand_for_trace_node(qld, "2026-09-20").unwrap();
        assert_eq!(statute_demand.jurisdiction_ref.as_deref(), Some("AU-QLD"));
        assert_eq!(statute_demand.source_roles, vec![SourceRole::PrimaryLegislation]);
    }

    #[test]
    fn exact_waltons_case_demand_is_source_work_not_authority() {
        let demand = LegalSourceDemand {
            demand_ref: "demand:waltons:hca7".into(),
            origin_ref: "doctrine:au:contract:estoppel".into(),
            jurisdiction_ref: Some("AU".into()),
            source_roles: vec![SourceRole::PrimaryCaseLaw],
            authority_levels: vec![AuthorityLevel::Official],
            provider_profile_refs: vec![OALC_PROVIDER_PROFILE.into()],
            requested_facets: vec!["case.full_text".into(), "case.citation_graph".into()],
            temporal_refs: vec!["as_at:2026-09-20".into()],
            provenance_refs: vec!["contract-follow:waltons".into()],
            priority: 100,
        };
        let plan = plan_legal_sources(&demand, &[]);
        let exact = exact_oalc_case_law_demand(
            &plan,
            &demand.origin_ref,
            "[1988] HCA 7",
            "court:HCA",
        ).unwrap();
        assert_eq!(exact.authority, "acquisition_plan_only");
        assert_eq!(exact.source_role, SourceRole::PrimaryCaseLaw);
    }
}
