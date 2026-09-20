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
