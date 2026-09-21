//! Generic recursive LegalFollow campaign runtime for Australian contracts.
//!
//! This module owns the Phase-IV orchestration seam that used to live only in
//! the Waltons campaign shell.  It deliberately keeps research scheduling,
//! source acquisition, identity review and treatment review separate from legal
//! authority.  A selected frontier item is a research action, not a truth rank.

use sensiblaw_governed_legal_provider::{
    run_live_oalc_case_follow_with_mode, OalcCaseAcquisitionMode, OalcCaseFollowRequest,
    OalcResolvedSourceReceipt, OALC_RANGE_MAX_REQUESTS_PER_ACQUISITION,
};
use sensiblaw_legal_runtime::{
    ConsumerResearchDemand, ConsumerResearchDemandKind,
};
use sensiblaw_legal_follow_plan::{
    apply_contract_landscape_expansion, compile_australian_contract_landscape_worklist,
    AustralianContractLandscapeWorklist, AustralianContractTrace, AuthorityLevel, ContractDoctrine,
    ContractLandscapeExpansionDelta, ContractLandscapeExpansionReceipt, ContractLandscapeWorkItem,
    ContractLandscapeWorkKind, ContractTraceEdge, ContractTraceNode, SourceRole, TraceNodeKind,
    TreatmentKind,
};
use sensiblaw_proof_search_loop::oalc_judgment_materialization::{
    materialize_oalc_judgment, OalcJudgmentMaterialisation,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub type CampaignResult<T> = Result<T, String>;

const RECURSIVE_OALC_MAX_REQUESTS_PER_ACQUISITION: u64 =
    2 + OALC_RANGE_MAX_REQUESTS_PER_ACQUISITION;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignBudget {
    pub max_accepted_hops: usize,
    pub max_source_acquisitions: usize,
    pub max_network_requests: u64,
}

impl Default for CampaignBudget {
    fn default() -> Self {
        Self {
            max_accepted_hops: 128,
            max_source_acquisitions: 32,
            max_network_requests: 128,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignConfig {
    pub campaign_ref: String,
    pub as_at: String,
    pub jurisdiction_filter: Option<String>,
    pub budget: CampaignBudget,
}

impl CampaignConfig {
    pub fn validate(&self) -> CampaignResult<()> {
        if self.campaign_ref.trim().is_empty() {
            return Err("campaign_ref must be non-empty".into());
        }
        if self.as_at.trim().is_empty() {
            return Err("campaign as_at must be non-empty".into());
        }
        if self.budget.max_accepted_hops == 0
            || self.budget.max_source_acquisitions == 0
            || self.budget.max_network_requests == 0
        {
            return Err("campaign budgets must all be positive".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CampaignFrontierClass {
    PrimarySource,
    TreatmentReview,
    ContextExpansion,
    TemporalAlternative,
    OutboundCitation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignOperatorGate {
    PrimarySourceAcquisition,
    AuthorityIdentityReview,
    AuthorityTreatmentReview,
    ContextExpansion,
    TemporalAlternative,
    OutboundCitationAcquisition,
    BudgetExhausted,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerDemandCampaignRoute {
    pub demand_kind: ConsumerResearchDemandKind,
    pub target_ref: String,
    pub frontier_class: CampaignFrontierClass,
    pub gate: CampaignOperatorGate,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

pub fn route_consumer_research_demand(
    demand: &ConsumerResearchDemand,
) -> ConsumerDemandCampaignRoute {
    let (frontier_class, gate) = match demand.kind {
        ConsumerResearchDemandKind::AcquireSource
        | ConsumerResearchDemandKind::RecoverProvenance => (
            CampaignFrontierClass::PrimarySource,
            CampaignOperatorGate::PrimarySourceAcquisition,
        ),
        ConsumerResearchDemandKind::ReviewTreatment => (
            CampaignFrontierClass::TreatmentReview,
            CampaignOperatorGate::AuthorityTreatmentReview,
        ),
        ConsumerResearchDemandKind::ResolveTemporalCoordinate => (
            CampaignFrontierClass::TemporalAlternative,
            CampaignOperatorGate::TemporalAlternative,
        ),
        ConsumerResearchDemandKind::ResolveJurisdiction
        | ConsumerResearchDemandKind::ReviewFact
        | ConsumerResearchDemandKind::ReviewBurdenOrException
        | ConsumerResearchDemandKind::ResolveSemanticIdentity => (
            CampaignFrontierClass::ContextExpansion,
            CampaignOperatorGate::ContextExpansion,
        ),
    };
    ConsumerDemandCampaignRoute {
        demand_kind: demand.kind,
        target_ref: demand.target_ref.clone(),
        frontier_class,
        gate,
        candidate_only: true,
        creates_legal_authority: false,
        creates_current_law_conclusion: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshFrontierItem {
    pub frontier_ref: String,
    pub class: CampaignFrontierClass,
    pub semantic_ref: String,
    pub related_ref: Option<String>,
    pub source_citation: String,
    pub jurisdiction_ref: String,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignNextStep {
    pub gate: CampaignOperatorGate,
    pub selected: Option<FreshFrontierItem>,
    pub selector_is_legal_truth_rank: bool,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

fn frontier_priority(class: CampaignFrontierClass) -> u8 {
    match class {
        CampaignFrontierClass::PrimarySource => 0,
        CampaignFrontierClass::TreatmentReview => 1,
        CampaignFrontierClass::ContextExpansion => 2,
        CampaignFrontierClass::TemporalAlternative => 3,
        CampaignFrontierClass::OutboundCitation => 4,
    }
}

pub fn select_fresh_frontier_item(items: &[FreshFrontierItem]) -> Option<FreshFrontierItem> {
    let mut items = items.to_vec();
    items.sort_by(|left, right| {
        frontier_priority(left.class)
            .cmp(&frontier_priority(right.class))
            .then_with(|| left.frontier_ref.cmp(&right.frontier_ref))
    });
    items.into_iter().next()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignHopReceipt {
    pub hop_index: usize,
    pub source_ref: String,
    pub provenance_ref: String,
    pub added_node_count: usize,
    pub added_edge_count: usize,
    pub recompute_frontier_required: bool,
    pub old_source_history_preserved: bool,
    pub old_conclusions_frozen: bool,
    pub fresh_frontier: Vec<FreshFrontierItem>,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceNodeSnapshot {
    pub semantic_ref: String,
    pub label: String,
    pub kind: String,
    pub doctrine: Option<String>,
    pub jurisdiction_ref: String,
    pub court_ref: Option<String>,
    pub decision_or_effective_date: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub source_role: String,
    pub authority_level: String,
    pub source_citation: String,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEdgeSnapshot {
    pub from_ref: String,
    pub to_ref: String,
    pub treatment: String,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractTraceSnapshot {
    pub root_ref: String,
    pub nodes: Vec<TraceNodeSnapshot>,
    pub edges: Vec<TraceEdgeSnapshot>,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

fn doctrine_name(value: ContractDoctrine) -> &'static str {
    match value {
        ContractDoctrine::Formation => "Formation",
        ContractDoctrine::Intention => "Intention",
        ContractDoctrine::TermsAndIncorporation => "TermsAndIncorporation",
        ContractDoctrine::Construction => "Construction",
        ContractDoctrine::Estoppel => "Estoppel",
        ContractDoctrine::Unconscionability => "Unconscionability",
        ContractDoctrine::Penalties => "Penalties",
        ContractDoctrine::RepudiationAndTermination => "RepudiationAndTermination",
        ContractDoctrine::Damages => "Damages",
        ContractDoctrine::Restitution => "Restitution",
        ContractDoctrine::Privity => "Privity",
        ContractDoctrine::ConsumerLaw => "ConsumerLaw",
    }
}

fn parse_doctrine(value: &str) -> CampaignResult<ContractDoctrine> {
    match value {
        "Formation" => Ok(ContractDoctrine::Formation),
        "Intention" => Ok(ContractDoctrine::Intention),
        "TermsAndIncorporation" => Ok(ContractDoctrine::TermsAndIncorporation),
        "Construction" => Ok(ContractDoctrine::Construction),
        "Estoppel" => Ok(ContractDoctrine::Estoppel),
        "Unconscionability" => Ok(ContractDoctrine::Unconscionability),
        "Penalties" => Ok(ContractDoctrine::Penalties),
        "RepudiationAndTermination" => Ok(ContractDoctrine::RepudiationAndTermination),
        "Damages" => Ok(ContractDoctrine::Damages),
        "Restitution" => Ok(ContractDoctrine::Restitution),
        "Privity" => Ok(ContractDoctrine::Privity),
        "ConsumerLaw" => Ok(ContractDoctrine::ConsumerLaw),
        other => Err(format!(
            "unsupported doctrine in campaign snapshot {other:?}"
        )),
    }
}

fn node_kind_name(value: TraceNodeKind) -> &'static str {
    match value {
        TraceNodeKind::Doctrine => "Doctrine",
        TraceNodeKind::CaseAuthority => "CaseAuthority",
        TraceNodeKind::Legislation => "Legislation",
        TraceNodeKind::ResearchRequirement => "ResearchRequirement",
        TraceNodeKind::Matter => "Matter",
    }
}

fn parse_node_kind(value: &str) -> CampaignResult<TraceNodeKind> {
    match value {
        "Doctrine" => Ok(TraceNodeKind::Doctrine),
        "CaseAuthority" => Ok(TraceNodeKind::CaseAuthority),
        "Legislation" => Ok(TraceNodeKind::Legislation),
        "ResearchRequirement" => Ok(TraceNodeKind::ResearchRequirement),
        "Matter" => Ok(TraceNodeKind::Matter),
        other => Err(format!(
            "unsupported node kind in campaign snapshot {other:?}"
        )),
    }
}

fn source_role_name(value: SourceRole) -> &'static str {
    match value {
        SourceRole::PrimaryCaseLaw => "PrimaryCaseLaw",
        SourceRole::PrimaryLegislation => "PrimaryLegislation",
        SourceRole::OfficialRecord => "OfficialRecord",
        SourceRole::ResearchIndex => "ResearchIndex",
        SourceRole::SecondaryAnalysis => "SecondaryAnalysis",
    }
}

fn parse_source_role(value: &str) -> CampaignResult<SourceRole> {
    match value {
        "PrimaryCaseLaw" => Ok(SourceRole::PrimaryCaseLaw),
        "PrimaryLegislation" => Ok(SourceRole::PrimaryLegislation),
        "OfficialRecord" => Ok(SourceRole::OfficialRecord),
        "ResearchIndex" => Ok(SourceRole::ResearchIndex),
        "SecondaryAnalysis" => Ok(SourceRole::SecondaryAnalysis),
        other => Err(format!(
            "unsupported source role in campaign snapshot {other:?}"
        )),
    }
}

fn authority_level_name(value: AuthorityLevel) -> &'static str {
    match value {
        AuthorityLevel::Official => "Official",
        AuthorityLevel::Supporting => "Supporting",
        AuthorityLevel::Secondary => "Secondary",
    }
}

fn parse_authority_level(value: &str) -> CampaignResult<AuthorityLevel> {
    match value {
        "Official" => Ok(AuthorityLevel::Official),
        "Supporting" => Ok(AuthorityLevel::Supporting),
        "Secondary" => Ok(AuthorityLevel::Secondary),
        other => Err(format!(
            "unsupported authority level in campaign snapshot {other:?}"
        )),
    }
}

fn treatment_name(value: TreatmentKind) -> &'static str {
    match value {
        TreatmentKind::Seeds => "Seeds",
        TreatmentKind::Supports => "Supports",
        TreatmentKind::Applies => "Applies",
        TreatmentKind::Follows => "Follows",
        TreatmentKind::Distinguishes => "Distinguishes",
        TreatmentKind::Qualifies => "Qualifies",
        TreatmentKind::Displaces => "Displaces",
        TreatmentKind::TemporalSuccessor => "TemporalSuccessor",
        TreatmentKind::Requires => "Requires",
        TreatmentKind::Intersects => "Intersects",
    }
}

fn parse_treatment(value: &str) -> CampaignResult<TreatmentKind> {
    match value {
        "Seeds" => Ok(TreatmentKind::Seeds),
        "Supports" => Ok(TreatmentKind::Supports),
        "Applies" => Ok(TreatmentKind::Applies),
        "Follows" => Ok(TreatmentKind::Follows),
        "Distinguishes" => Ok(TreatmentKind::Distinguishes),
        "Qualifies" => Ok(TreatmentKind::Qualifies),
        "Displaces" => Ok(TreatmentKind::Displaces),
        "TemporalSuccessor" => Ok(TreatmentKind::TemporalSuccessor),
        "Requires" => Ok(TreatmentKind::Requires),
        "Intersects" => Ok(TreatmentKind::Intersects),
        other => Err(format!(
            "unsupported treatment in campaign snapshot {other:?}"
        )),
    }
}

pub fn snapshot_trace(trace: &AustralianContractTrace) -> ContractTraceSnapshot {
    ContractTraceSnapshot {
        root_ref: trace.root_ref.clone(),
        nodes: trace
            .nodes
            .values()
            .map(|node| TraceNodeSnapshot {
                semantic_ref: node.semantic_ref.clone(),
                label: node.label.clone(),
                kind: node_kind_name(node.kind).into(),
                doctrine: node.doctrine.map(doctrine_name).map(str::to_string),
                jurisdiction_ref: node.jurisdiction_ref.clone(),
                court_ref: node.court_ref.clone(),
                decision_or_effective_date: node.decision_or_effective_date.clone(),
                valid_from: node.valid_from.clone(),
                valid_to: node.valid_to.clone(),
                source_role: source_role_name(node.source_role).into(),
                authority_level: authority_level_name(node.authority_level).into(),
                source_citation: node.source_citation.clone(),
                candidate_only: node.candidate_only,
                creates_legal_authority: node.creates_legal_authority,
            })
            .collect(),
        edges: trace
            .edges
            .iter()
            .map(|edge| TraceEdgeSnapshot {
                from_ref: edge.from_ref.clone(),
                to_ref: edge.to_ref.clone(),
                treatment: treatment_name(edge.treatment).into(),
                candidate_only: edge.candidate_only,
                creates_legal_authority: edge.creates_legal_authority,
            })
            .collect(),
        candidate_only: trace.candidate_only,
        creates_legal_authority: trace.creates_legal_authority,
    }
}

pub fn restore_trace(snapshot: &ContractTraceSnapshot) -> CampaignResult<AustralianContractTrace> {
    let mut nodes = BTreeMap::new();
    for node in &snapshot.nodes {
        let restored = ContractTraceNode {
            semantic_ref: node.semantic_ref.clone(),
            label: node.label.clone(),
            kind: parse_node_kind(&node.kind)?,
            doctrine: node.doctrine.as_deref().map(parse_doctrine).transpose()?,
            jurisdiction_ref: node.jurisdiction_ref.clone(),
            court_ref: node.court_ref.clone(),
            decision_or_effective_date: node.decision_or_effective_date.clone(),
            valid_from: node.valid_from.clone(),
            valid_to: node.valid_to.clone(),
            source_role: parse_source_role(&node.source_role)?,
            authority_level: parse_authority_level(&node.authority_level)?,
            source_citation: node.source_citation.clone(),
            candidate_only: node.candidate_only,
            creates_legal_authority: node.creates_legal_authority,
        };
        nodes.insert(restored.semantic_ref.clone(), restored);
    }
    let edges = snapshot
        .edges
        .iter()
        .map(|edge| {
            Ok(ContractTraceEdge {
                from_ref: edge.from_ref.clone(),
                to_ref: edge.to_ref.clone(),
                treatment: parse_treatment(&edge.treatment)?,
                candidate_only: edge.candidate_only,
                creates_legal_authority: edge.creates_legal_authority,
            })
        })
        .collect::<CampaignResult<Vec<_>>>()?;
    let trace = AustralianContractTrace {
        root_ref: snapshot.root_ref.clone(),
        nodes,
        edges,
        candidate_only: snapshot.candidate_only,
        creates_legal_authority: snapshot.creates_legal_authority,
    };
    trace.validate()?;
    Ok(trace)
}

fn work_item_to_fresh(item: &ContractLandscapeWorkItem) -> FreshFrontierItem {
    let class = match item.kind {
        ContractLandscapeWorkKind::AcquirePrimarySource => CampaignFrontierClass::PrimarySource,
        ContractLandscapeWorkKind::ReviewAuthorityTreatment => {
            CampaignFrontierClass::TreatmentReview
        }
        ContractLandscapeWorkKind::ExpandResearchContext => CampaignFrontierClass::ContextExpansion,
        ContractLandscapeWorkKind::RetainTemporalAlternative => {
            CampaignFrontierClass::TemporalAlternative
        }
    };
    FreshFrontierItem {
        frontier_ref: item.work_ref.clone(),
        class,
        semantic_ref: item.semantic_ref.clone(),
        related_ref: item.related_ref.clone(),
        source_citation: item.source_citation.clone(),
        jurisdiction_ref: item.jurisdiction_ref.clone(),
        candidate_only: true,
        creates_legal_authority: false,
        creates_current_law_conclusion: false,
    }
}

fn work_refs(work: &AustralianContractLandscapeWorklist) -> BTreeSet<String> {
    work.source_items
        .iter()
        .chain(work.treatment_items.iter())
        .chain(work.context_items.iter())
        .chain(work.temporal_alternatives.iter())
        .map(|item| item.work_ref.clone())
        .collect()
}

fn fresh_work(
    work: &AustralianContractLandscapeWorklist,
    observed: &BTreeSet<String>,
) -> Vec<FreshFrontierItem> {
    work.source_items
        .iter()
        .chain(work.treatment_items.iter())
        .chain(work.context_items.iter())
        .chain(work.temporal_alternatives.iter())
        .filter(|item| !observed.contains(&item.work_ref))
        .map(work_item_to_fresh)
        .collect()
}

fn work_item_paid_by_delta(
    item: &ContractLandscapeWorkItem,
    delta: &ContractLandscapeExpansionDelta,
) -> bool {
    match item.kind {
        ContractLandscapeWorkKind::AcquirePrimarySource
        | ContractLandscapeWorkKind::ExpandResearchContext
        | ContractLandscapeWorkKind::RetainTemporalAlternative => delta
            .discovered_nodes
            .iter()
            .any(|node| node.semantic_ref == item.semantic_ref),
        ContractLandscapeWorkKind::ReviewAuthorityTreatment => {
            let Some(related_ref) = item.related_ref.as_deref() else {
                return false;
            };
            let Some(treatment) = item.treatment else {
                return false;
            };
            delta.discovered_edges.iter().any(|edge| {
                edge.from_ref == item.semantic_ref
                    && edge.to_ref == related_ref
                    && edge.treatment == treatment
            })
        }
    }
}

fn work_refs_paid_by_delta(
    work: &AustralianContractLandscapeWorklist,
    delta: &ContractLandscapeExpansionDelta,
) -> BTreeSet<String> {
    work.source_items
        .iter()
        .chain(work.treatment_items.iter())
        .chain(work.context_items.iter())
        .chain(work.temporal_alternatives.iter())
        .filter(|item| work_item_paid_by_delta(item, delta))
        .map(|item| item.work_ref.clone())
        .collect()
}

pub struct ContractFollowCampaign {
    config: CampaignConfig,
    trace: AustralianContractTrace,
    observed_work_refs: BTreeSet<String>,
    hops: Vec<CampaignHopReceipt>,
    accepted_hops_before: usize,
    reviewed_residuals: Vec<Value>,
    source_acquisitions: usize,
    network_requests: u64,
}

impl ContractFollowCampaign {
    pub fn new(config: CampaignConfig, trace: AustralianContractTrace) -> CampaignResult<Self> {
        config.validate()?;
        trace.validate()?;
        let work = compile_australian_contract_landscape_worklist(
            &trace,
            &config.as_at,
            config.jurisdiction_filter.as_deref(),
        )?;
        Ok(Self {
            config,
            trace,
            observed_work_refs: work_refs(&work),
            hops: Vec::new(),
            accepted_hops_before: 0,
            reviewed_residuals: Vec::new(),
            source_acquisitions: 0,
            network_requests: 0,
        })
    }

    pub fn from_snapshot(
        config: CampaignConfig,
        snapshot: &ContractTraceSnapshot,
    ) -> CampaignResult<Self> {
        Self::new(config, restore_trace(snapshot)?)
    }

    pub fn resume(
        config: CampaignConfig,
        trace: AustralianContractTrace,
        accepted_hops_before: usize,
        source_acquisitions: usize,
        network_requests: u64,
    ) -> CampaignResult<Self> {
        let mut campaign = Self::new(config, trace)?;
        if accepted_hops_before > campaign.config.budget.max_accepted_hops
            || source_acquisitions > campaign.config.budget.max_source_acquisitions
            || network_requests > campaign.config.budget.max_network_requests
        {
            return Err("parent campaign counters already exceed configured budget".into());
        }
        campaign.accepted_hops_before = accepted_hops_before;
        campaign.source_acquisitions = source_acquisitions;
        campaign.network_requests = network_requests;
        Ok(campaign)
    }

    pub fn accepted_hop_count(&self) -> usize {
        self.accepted_hops_before + self.hops.len()
    }

    pub fn trace(&self) -> &AustralianContractTrace {
        &self.trace
    }

    pub fn recomputed_worklist(&self) -> CampaignResult<AustralianContractLandscapeWorklist> {
        compile_australian_contract_landscape_worklist(
            &self.trace,
            &self.config.as_at,
            self.config.jurisdiction_filter.as_deref(),
        )
    }

    pub fn accept_delta(
        &mut self,
        source_ref: &str,
        delta: ContractLandscapeExpansionDelta,
    ) -> CampaignResult<&CampaignHopReceipt> {
        if self.accepted_hop_count() >= self.config.budget.max_accepted_hops {
            return Err("campaign accepted-hop budget exhausted".into());
        }
        let (next, receipt) = apply_contract_landscape_expansion(&self.trace, &delta)?;
        self.trace = next;
        let work = self.recomputed_worklist()?;

        // The delta being accepted is already reviewed work.  Do not feed the
        // node/edge it just paid straight back into the fresh frontier as if it
        // were a new acquisition/review demand.  Only consequences not paid by
        // this delta may become fresh work.
        let paid_by_delta = work_refs_paid_by_delta(&work, &delta);
        let mut observed_for_fresh = self.observed_work_refs.clone();
        observed_for_fresh.extend(paid_by_delta);
        let fresh = fresh_work(&work, &observed_for_fresh);
        self.observed_work_refs.extend(work_refs(&work));
        self.hops.push(campaign_hop_receipt(
            self.accepted_hop_count() + 1,
            source_ref,
            &receipt,
            fresh,
        ));
        Ok(self.hops.last().expect("just pushed campaign hop"))
    }

    pub fn preserve_reviewed_residual(&mut self, source_ref: &str, residual: Value) {
        self.reviewed_residuals.push(json!({
            "source_artifact": source_ref,
            "residual": residual,
        }));
    }

    pub fn accept_batch(
        &mut self,
        source_ref: &str,
        deltas: Vec<ContractLandscapeExpansionDelta>,
        reviewed_residuals: Vec<Value>,
    ) -> CampaignResult<Vec<CampaignHopReceipt>> {
        for residual in reviewed_residuals {
            self.preserve_reviewed_residual(source_ref, residual);
        }
        let start = self.hops.len();
        for delta in deltas {
            self.accept_delta(source_ref, delta)?;
        }
        Ok(self.hops[start..].to_vec())
    }

    pub fn next_fresh_step(&self) -> CampaignNextStep {
        if self.accepted_hop_count() >= self.config.budget.max_accepted_hops {
            return CampaignNextStep {
                gate: CampaignOperatorGate::BudgetExhausted,
                selected: None,
                selector_is_legal_truth_rank: false,
                candidate_only: true,
                creates_legal_authority: false,
                creates_current_law_conclusion: false,
            };
        }
        let selected = self
            .hops
            .last()
            .and_then(|hop| select_fresh_frontier_item(&hop.fresh_frontier));
        let gate = match selected.as_ref().map(|item| item.class) {
            Some(CampaignFrontierClass::PrimarySource) => {
                CampaignOperatorGate::PrimarySourceAcquisition
            }
            Some(CampaignFrontierClass::TreatmentReview) => {
                CampaignOperatorGate::AuthorityTreatmentReview
            }
            Some(CampaignFrontierClass::ContextExpansion) => CampaignOperatorGate::ContextExpansion,
            Some(CampaignFrontierClass::TemporalAlternative) => {
                CampaignOperatorGate::TemporalAlternative
            }
            Some(CampaignFrontierClass::OutboundCitation) => {
                CampaignOperatorGate::OutboundCitationAcquisition
            }
            None => CampaignOperatorGate::None,
        };
        CampaignNextStep {
            gate,
            selected,
            selector_is_legal_truth_rank: false,
            candidate_only: true,
            creates_legal_authority: false,
            creates_current_law_conclusion: false,
        }
    }

    pub fn ensure_source_acquisition_budget(
        &self,
        reserved_network_requests: u64,
    ) -> CampaignResult<()> {
        if self.source_acquisitions >= self.config.budget.max_source_acquisitions {
            return Err("campaign source-acquisition budget exhausted".into());
        }
        if self
            .network_requests
            .saturating_add(reserved_network_requests)
            > self.config.budget.max_network_requests
        {
            return Err("campaign network-request budget exhausted".into());
        }
        Ok(())
    }

    pub fn record_source_acquisition(&mut self, network_requests: u64) -> CampaignResult<()> {
        self.ensure_source_acquisition_budget(network_requests)?;
        self.source_acquisitions += 1;
        self.network_requests += network_requests;
        Ok(())
    }

    pub fn receipt_json(&self) -> CampaignResult<Value> {
        let work = self.recomputed_worklist()?;
        Ok(json!({
            "schema_version": "sl.contract_follow_campaign.v0_1",
            "campaign_ref": self.config.campaign_ref,
            "as_at": self.config.as_at,
            "jurisdiction_filter": self.config.jurisdiction_filter,
            "budget": self.config.budget,
            "accepted_hop_count": self.accepted_hop_count(),
            "segment_accepted_hop_count": self.hops.len(),
            "accepted_hops_before": self.accepted_hops_before,
            "source_acquisition_count": self.source_acquisitions,
            "network_request_count": self.network_requests,
            "reviewed_residual_count": self.reviewed_residuals.len(),
            "reviewed_residuals": self.reviewed_residuals,
            "trajectory": self.hops,
            "next_fresh_step": self.next_fresh_step(),
            "final_trace": snapshot_trace(&self.trace),
            // Legacy field retained as total worklist inventory for schema
            // compatibility.  Actionable frontier state is reported separately.
            "final_frontier_counts": {
                "primary_source_acquisition": work.source_items.len(),
                "authority_treatment_review": work.treatment_items.len(),
                "context_expansion": work.context_items.len(),
                "temporal_alternatives": work.temporal_alternatives.len(),
            },
            "final_worklist_inventory_counts": {
                "primary_source_acquisition": work.source_items.len(),
                "authority_treatment_review": work.treatment_items.len(),
                "context_expansion": work.context_items.len(),
                "temporal_alternatives": work.temporal_alternatives.len(),
            },
            "final_fresh_frontier_count": self
                .hops
                .last()
                .map(|hop| hop.fresh_frontier.len())
                .unwrap_or(0),
            "candidate_only": true,
            "creates_legal_authority": false,
            "creates_current_law_conclusion": false,
            "transport": "typed_rust_in_process",
            "json_is_semantic_command_transport": false,
        }))
    }
}

fn campaign_hop_receipt(
    hop_index: usize,
    source_ref: &str,
    receipt: &ContractLandscapeExpansionReceipt,
    fresh_frontier: Vec<FreshFrontierItem>,
) -> CampaignHopReceipt {
    CampaignHopReceipt {
        hop_index,
        source_ref: source_ref.into(),
        provenance_ref: receipt.provenance_ref.clone(),
        added_node_count: receipt.added_node_count,
        added_edge_count: receipt.added_edge_count,
        recompute_frontier_required: receipt.recompute_frontier_required,
        old_source_history_preserved: receipt.old_source_history_preserved,
        old_conclusions_frozen: receipt.old_conclusions_frozen,
        fresh_frontier,
        candidate_only: receipt.candidate_only,
        creates_legal_authority: receipt.creates_legal_authority,
        creates_current_law_conclusion: receipt.creates_current_law_conclusion,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundCitationResidual {
    pub residual_ref: String,
    pub source_document_ref: String,
    pub source_semantic_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_sha256: String,
    pub medium_neutral_citation: String,
    pub citation_locator_refs: Vec<String>,
    pub anchor_paragraph_locator_refs: Vec<String>,
    pub first_paragraph_ordinal: u64,
    pub research_priority: u16,
    pub priority_is_legal_truth_rank: bool,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

fn mnc_parts(value: &str) -> Option<(&str, &str, &str)> {
    let fields = value.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 3 {
        return None;
    }
    let year = fields[0];
    if year.len() != 6
        || !year.starts_with('[')
        || !year.ends_with(']')
        || !year[1..5].bytes().all(|byte| byte.is_ascii_digit())
        || !fields[1].bytes().all(|byte| byte.is_ascii_alphanumeric())
        || !fields[2].bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some((fields[0], fields[1], fields[2]))
}

fn australian_court_priority(court: &str) -> Option<u16> {
    match court {
        "HCA" => Some(0),
        "FCAFC" => Some(10),
        "NSWCA" | "VSCA" | "QCA" | "WASCA" | "SASCFC" | "TASFC" | "ACTCA" | "NTCA" => Some(20),
        "FCA" => Some(30),
        "NSWSC" | "VSC" | "QSC" | "WASC" | "SASC" | "TASSC" | "ACTSC" | "NTSC" => Some(40),
        _ => None,
    }
}

fn trace_contains_mnc(trace: &AustralianContractTrace, citation: &str) -> bool {
    trace
        .nodes
        .values()
        .any(|node| node.source_citation.split(';').next().map(str::trim) == Some(citation))
}

pub fn discover_outbound_citation_residuals(
    trace: &AustralianContractTrace,
    source_semantic_ref: &str,
    materialization: &OalcJudgmentMaterialisation,
) -> Vec<OutboundCitationResidual> {
    let mut grouped: BTreeMap<String, OutboundCitationResidual> = BTreeMap::new();
    for candidate in &materialization.citation_candidates {
        let Some((_, court, _)) = mnc_parts(candidate.citation_text.trim()) else {
            continue;
        };
        let Some(priority) = australian_court_priority(court) else {
            continue;
        };
        let citation = candidate.citation_text.trim().to_string();
        if trace_contains_mnc(trace, &citation) {
            continue;
        }
        let entry = grouped
            .entry(citation.clone())
            .or_insert_with(|| OutboundCitationResidual {
                residual_ref: format!(
                    "campaign:outbound-citation:{}:{}",
                    materialization.source_revision_ref, citation
                ),
                source_document_ref: materialization.document_ref.clone(),
                source_semantic_ref: source_semantic_ref.into(),
                source_revision_ref: materialization.source_revision_ref.clone(),
                canonical_text_sha256: materialization.canonical_text_sha256.clone(),
                medium_neutral_citation: citation,
                citation_locator_refs: Vec::new(),
                anchor_paragraph_locator_refs: Vec::new(),
                first_paragraph_ordinal: candidate.paragraph_ordinal,
                research_priority: priority,
                priority_is_legal_truth_rank: false,
                candidate_only: true,
                creates_legal_authority: false,
                creates_current_law_conclusion: false,
            });
        entry.first_paragraph_ordinal = entry
            .first_paragraph_ordinal
            .min(candidate.paragraph_ordinal);
        if !entry
            .citation_locator_refs
            .contains(&candidate.paragraph_locator_ref)
        {
            entry
                .citation_locator_refs
                .push(candidate.paragraph_locator_ref.clone());
        }
        for anchor in &candidate.anchor_paragraph_locator_refs {
            if !entry.anchor_paragraph_locator_refs.contains(anchor) {
                entry.anchor_paragraph_locator_refs.push(anchor.clone());
            }
        }
    }
    let mut residuals = grouped.into_values().collect::<Vec<_>>();
    residuals.sort_by(|left, right| {
        left.research_priority
            .cmp(&right.research_priority)
            .then_with(|| {
                left.first_paragraph_ordinal
                    .cmp(&right.first_paragraph_ordinal)
            })
            .then_with(|| {
                left.medium_neutral_citation
                    .cmp(&right.medium_neutral_citation)
            })
    });
    residuals
}

pub fn select_fresh_outbound_citation(
    residuals: &[OutboundCitationResidual],
) -> Option<&OutboundCitationResidual> {
    residuals.first()
}

pub fn materialize_retained_oalc_receipt(
    receipt_path: &Path,
) -> CampaignResult<(OalcResolvedSourceReceipt, OalcJudgmentMaterialisation)> {
    let bytes = fs::read(receipt_path)
        .map_err(|error| format!("read {}: {error}", receipt_path.display()))?;
    let receipt: OalcResolvedSourceReceipt = serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode {}: {error}", receipt_path.display()))?;
    let text = fs::read_to_string(&receipt.local_artifact_ref).map_err(|error| {
        format!(
            "read retained judgment {}: {error}",
            receipt.local_artifact_ref.display()
        )
    })?;
    let materialization = materialize_oalc_judgment(&receipt, &text, &[])
        .map_err(|error| format!("materialize {}: {error:?}", receipt.citation))?;
    Ok((receipt, materialization))
}

pub fn acquire_outbound_citation(
    residual: &OutboundCitationResidual,
    output_dir: PathBuf,
    as_at: &str,
) -> CampaignResult<OalcResolvedSourceReceipt> {
    let mut request =
        OalcCaseFollowRequest::for_citation(&residual.medium_neutral_citation, output_dir);
    request.as_at = as_at.into();
    let run = run_live_oalc_case_follow_with_mode(
        &request,
        OalcCaseAcquisitionMode::IndexedThenPinnedRangeIndex,
    )
    .map_err(|error| format!("recursive OALC acquisition: {error:?}"))?;
    let bytes = fs::read(&run.source_receipt_path)
        .map_err(|error| format!("read {}: {error}", run.source_receipt_path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode {}: {error}", run.source_receipt_path.display()))
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> CampaignResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(value)
            .map_err(|error| format!("encode {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("write {}: {error}", path.display()))
}

pub fn read_campaign_receipt(path: &Path) -> CampaignResult<Value> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("decode {}: {error}", path.display()))
}

pub fn snapshot_from_campaign_receipt(value: &Value) -> CampaignResult<ContractTraceSnapshot> {
    serde_json::from_value(
        value
            .get("final_trace")
            .cloned()
            .ok_or_else(|| "campaign receipt missing final_trace".to_string())?,
    )
    .map_err(|error| format!("decode campaign final_trace: {error}"))
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn required_arg(args: &[String], flag: &str) -> CampaignResult<String> {
    arg_value(args, flag).ok_or_else(|| format!("{flag} requires a value"))
}

fn trace_snapshot_from_trajectory(path: &Path) -> CampaignResult<ContractTraceSnapshot> {
    let value = read_campaign_receipt(path)?;
    if let Some(trace) = value.get("final_trace") {
        return serde_json::from_value(trace.clone())
            .map_err(|error| format!("decode final_trace from {}: {error}", path.display()));
    }
    if let Some(trace) = value
        .get("campaign_state")
        .and_then(|state| state.get("final_trace"))
    {
        return serde_json::from_value(trace.clone()).map_err(|error| {
            format!(
                "decode campaign_state.final_trace from {}: {error}",
                path.display()
            )
        });
    }
    Err(format!(
        "{} does not contain a resumable final_trace; rerun the typed trajectory with S14.5 code",
        path.display()
    ))
}

#[derive(Debug, Clone, Copy)]
struct CampaignCounters {
    accepted_hops: usize,
    source_acquisitions: usize,
    network_requests: u64,
}

fn counters_from_trajectory(path: &Path) -> CampaignResult<CampaignCounters> {
    let value = read_campaign_receipt(path)?;
    let state = value.get("campaign_state").unwrap_or(&value);
    Ok(CampaignCounters {
        accepted_hops: state
            .get("accepted_hop_count")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| {
                value
                    .get("hop_count")
                    .and_then(Value::as_u64)
                    .unwrap_or_default()
            }) as usize,
        source_acquisitions: state
            .get("source_acquisition_count")
            .and_then(Value::as_u64)
            .unwrap_or_default() as usize,
        network_requests: state
            .get("network_request_count")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
    })
}

fn resume_from_trajectory(path: &Path) -> CampaignResult<ContractFollowCampaign> {
    let snapshot = trace_snapshot_from_trajectory(path)?;
    let trace = restore_trace(&snapshot)?;
    let counters = counters_from_trajectory(path)?;
    ContractFollowCampaign::resume(
        config_from_trajectory(path)?,
        trace,
        counters.accepted_hops,
        counters.source_acquisitions,
        counters.network_requests,
    )
}

fn config_from_trajectory(path: &Path) -> CampaignResult<CampaignConfig> {
    let value = read_campaign_receipt(path)?;
    let state = value.get("campaign_state").unwrap_or(&value);
    let as_at = state
        .get("as_at")
        .and_then(Value::as_str)
        .or_else(|| value.get("as_at").and_then(Value::as_str))
        .unwrap_or("2026-09-20")
        .to_string();
    let jurisdiction_filter = state
        .get("jurisdiction_filter")
        .or_else(|| value.get("jurisdiction_filter"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let budget = match state.get("budget") {
        Some(value) => serde_json::from_value::<CampaignBudget>(value.clone())
            .map_err(|error| format!("decode campaign budget from {}: {error}", path.display()))?,
        None => CampaignBudget::default(),
    };
    Ok(CampaignConfig {
        campaign_ref: format!("campaign:recursive:{}", path.display()),
        as_at,
        jurisdiction_filter,
        budget,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PendingRecursiveReview {
    discovery_source_receipt: String,
    source_semantic_ref: String,
    target_source_receipt: String,
    target_medium_neutral_citation: String,
    target_semantic_ref: Option<String>,
}

fn pending_review_from_trajectory(path: &Path) -> CampaignResult<PendingRecursiveReview> {
    let value = read_campaign_receipt(path)?;
    serde_json::from_value(
        value
            .get("pending_recursive_review")
            .cloned()
            .ok_or_else(|| format!("{} has no pending_recursive_review", path.display()))?,
    )
    .map_err(|error| {
        format!(
            "decode pending recursive review from {}: {error}",
            path.display()
        )
    })
}

fn campaign_step(gate: CampaignOperatorGate) -> CampaignNextStep {
    CampaignNextStep {
        gate,
        selected: None,
        selector_is_legal_truth_rank: false,
        candidate_only: true,
        creates_legal_authority: false,
        creates_current_law_conclusion: false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignDriveDisposition {
    ExecutedOutboundAcquisition,
    AwaitIdentityReview,
    AwaitTreatmentReview,
    BudgetExhausted,
    Complete,
    AwaitExplicitOperator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignDriveReceipt {
    pub schema_version: String,
    pub trajectory: String,
    pub observed_gate: CampaignOperatorGate,
    pub disposition: CampaignDriveDisposition,
    pub review_artifact: Option<String>,
    pub deterministic_action_executed: bool,
    pub review_gate_bypassed: bool,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

fn campaign_gate_from_receipt(value: &Value) -> CampaignResult<CampaignOperatorGate> {
    let step: CampaignNextStep = serde_json::from_value(
        value
            .get("next_operator_gate")
            .cloned()
            .ok_or_else(|| "campaign receipt has no next_operator_gate".to_string())?,
    )
    .map_err(|error| format!("decode campaign next_operator_gate: {error}"))?;
    Ok(step.gate)
}

fn drive_receipt_for_gate(
    trajectory: &Path,
    value: &Value,
    gate: CampaignOperatorGate,
) -> CampaignDriveReceipt {
    let (disposition, review_artifact) = match gate {
        CampaignOperatorGate::AuthorityIdentityReview => (
            CampaignDriveDisposition::AwaitIdentityReview,
            value
                .get("identity_review_worksheet")
                .and_then(Value::as_str)
                .map(str::to_owned),
        ),
        CampaignOperatorGate::AuthorityTreatmentReview => (
            CampaignDriveDisposition::AwaitTreatmentReview,
            value
                .get("treatment_review_worksheet")
                .and_then(Value::as_str)
                .map(str::to_owned),
        ),
        CampaignOperatorGate::BudgetExhausted => (
            CampaignDriveDisposition::BudgetExhausted,
            None,
        ),
        CampaignOperatorGate::None => (
            CampaignDriveDisposition::Complete,
            None,
        ),
        _ => (
            CampaignDriveDisposition::AwaitExplicitOperator,
            None,
        ),
    };
    CampaignDriveReceipt {
        schema_version: "sl.contract_follow.campaign_drive.v0_1".into(),
        trajectory: trajectory.display().to_string(),
        observed_gate: gate,
        disposition,
        review_artifact,
        deterministic_action_executed: false,
        review_gate_bypassed: false,
        candidate_only: true,
        creates_legal_authority: false,
        creates_current_law_conclusion: false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OutboundFrontierEnvelope {
    schema_version: String,
    parent_campaign_receipt: String,
    source_receipt_path: String,
    source_semantic_ref: String,
    residual_count: usize,
    selected: Option<OutboundCitationResidual>,
    residuals: Vec<OutboundCitationResidual>,
    selector_is_legal_truth_rank: bool,
    budget: CampaignBudget,
    accepted_hop_count: usize,
    source_acquisition_count: usize,
    network_request_count: u64,
    candidate_only: bool,
    creates_legal_authority: bool,
    creates_current_law_conclusion: bool,
}

pub fn run(args: Vec<String>) -> CampaignResult<()> {
    match args.as_slice() {
        [command, rest @ ..] if command == "drive" => {
            let trajectory = PathBuf::from(required_arg(rest, "--trajectory")?);
            let status_output = arg_value(rest, "--status-output").map(PathBuf::from);
            let value = read_campaign_receipt(&trajectory)?;
            let gate = campaign_gate_from_receipt(&value)?;

            if gate == CampaignOperatorGate::OutboundCitationAcquisition {
                let frontier = arg_value(rest, "--frontier")
                    .map(PathBuf::from)
                    .or_else(|| {
                        value
                            .get("next_outbound_frontier")
                            .and_then(Value::as_str)
                            .map(PathBuf::from)
                    })
                    .ok_or_else(|| {
                        "campaign drive needs --frontier or trajectory next_outbound_frontier for outbound acquisition".to_string()
                    })?;
                let output_dir = PathBuf::from(required_arg(rest, "--output-dir")?);
                let mut nested = vec![
                    "acquire-next".to_string(),
                    "--trajectory".to_string(),
                    trajectory.display().to_string(),
                    "--frontier".to_string(),
                    frontier.display().to_string(),
                    "--output-dir".to_string(),
                    output_dir.display().to_string(),
                ];
                if let Some(as_at) = arg_value(rest, "--as-at") {
                    nested.push("--as-at".to_string());
                    nested.push(as_at);
                }
                if let Some(path) = status_output {
                    let receipt = CampaignDriveReceipt {
                        schema_version: "sl.contract_follow.campaign_drive.v0_1".into(),
                        trajectory: trajectory.display().to_string(),
                        observed_gate: gate,
                        disposition: CampaignDriveDisposition::ExecutedOutboundAcquisition,
                        review_artifact: Some(
                            output_dir
                                .join("authority-identity-review-worksheet.json")
                                .display()
                                .to_string(),
                        ),
                        deterministic_action_executed: true,
                        review_gate_bypassed: false,
                        candidate_only: true,
                        creates_legal_authority: false,
                        creates_current_law_conclusion: false,
                    };
                    write_json(&path, &receipt)?;
                }
                // Reuse the exact existing governed acquisition path.  The
                // nested command deterministically stops at identity review.
                return run(nested);
            }

            let receipt = drive_receipt_for_gate(&trajectory, &value, gate);
            if let Some(path) = status_output {
                write_json(&path, &receipt)?;
            }
            println!(
                "contract_follow_drive trajectory={} gate={:?} disposition={:?} review={} bypass=false authority=false current_law_conclusion=false",
                trajectory.display(),
                receipt.observed_gate,
                receipt.disposition,
                receipt.review_artifact.as_deref().unwrap_or("none"),
            );
            Ok(())
        }
        [command, rest @ ..] if command == "discover" => {
            let trajectory = PathBuf::from(required_arg(rest, "--trajectory")?);
            let source_receipt = PathBuf::from(required_arg(rest, "--source-receipt")?);
            let source_semantic_ref = required_arg(rest, "--source-semantic-ref")?;
            let output = PathBuf::from(required_arg(rest, "--output")?);
            let snapshot = trace_snapshot_from_trajectory(&trajectory)?;
            let trace = restore_trace(&snapshot)?;
            let config = config_from_trajectory(&trajectory)?;
            let counters = counters_from_trajectory(&trajectory)?;
            let (_, materialization) = materialize_retained_oalc_receipt(&source_receipt)?;
            let residuals =
                discover_outbound_citation_residuals(&trace, &source_semantic_ref, &materialization);
            let selected = select_fresh_outbound_citation(&residuals).cloned();
            let envelope = OutboundFrontierEnvelope {
                schema_version: "sl.contract_follow.outbound_frontier.v0_2".into(),
                parent_campaign_receipt: trajectory.display().to_string(),
                source_receipt_path: source_receipt.display().to_string(),
                source_semantic_ref,
                residual_count: residuals.len(),
                selected,
                residuals,
                selector_is_legal_truth_rank: false,
                budget: config.budget,
                accepted_hop_count: counters.accepted_hops,
                source_acquisition_count: counters.source_acquisitions,
                network_request_count: counters.network_requests,
                candidate_only: true,
                creates_legal_authority: false,
                creates_current_law_conclusion: false,
            };
            write_json(&output, &envelope)?;
            println!(
                "contract_follow_outbound_frontier={} residuals={} selected={} authority=false current_law_conclusion=false",
                output.display(),
                envelope.residual_count,
                envelope
                    .selected
                    .as_ref()
                    .map(|value| value.medium_neutral_citation.as_str())
                    .unwrap_or("none"),
            );
            Ok(())
        }
        [command, rest @ ..] if command == "acquire-next" => {
            let frontier = PathBuf::from(required_arg(rest, "--frontier")?);
            let output_dir = PathBuf::from(required_arg(rest, "--output-dir")?);
            let trajectory = PathBuf::from(required_arg(rest, "--trajectory")?);
            let requested_as_at = arg_value(rest, "--as-at");
            let envelope: OutboundFrontierEnvelope = {
                let bytes = fs::read(&frontier)
                    .map_err(|error| format!("read {}: {error}", frontier.display()))?;
                serde_json::from_slice(&bytes)
                    .map_err(|error| format!("decode {}: {error}", frontier.display()))?
            };
            if envelope.parent_campaign_receipt != trajectory.display().to_string() {
                return Err("outbound frontier parent campaign does not match --trajectory".into());
            }
            let mut campaign = resume_from_trajectory(&trajectory)?;
            if let Some(requested) = requested_as_at.as_deref() {
                if requested != campaign.config.as_at {
                    return Err(format!(
                        "recursive acquisition as-at {requested:?} differs from parent campaign {:?}; create an explicit temporal branch instead",
                        campaign.config.as_at
                    ));
                }
            }
            let as_at = campaign.config.as_at.clone();
            if campaign.config.budget != envelope.budget
                || campaign.accepted_hop_count() != envelope.accepted_hop_count
                || campaign.source_acquisitions != envelope.source_acquisition_count
                || campaign.network_requests != envelope.network_request_count
            {
                return Err("outbound frontier budget/counter snapshot no longer matches parent campaign".into());
            }
            campaign.ensure_source_acquisition_budget(
                RECURSIVE_OALC_MAX_REQUESTS_PER_ACQUISITION,
            )?;
            let selected = envelope
                .selected
                .ok_or_else(|| "outbound frontier has no selected candidate".to_string())?;
            let receipt = acquire_outbound_citation(&selected, output_dir.clone(), &as_at)?;
            campaign.record_source_acquisition(receipt.network_requests)?;
            let target_source_receipt = output_dir.join("oalc-source-receipt.json");
            let identity_worksheet =
                output_dir.join("authority-identity-review-worksheet.json");
            crate::contract_identity::prepare(
                &[target_source_receipt.clone()],
                &identity_worksheet,
            )?;
            let pending = PendingRecursiveReview {
                discovery_source_receipt: envelope.source_receipt_path.clone(),
                source_semantic_ref: envelope.source_semantic_ref.clone(),
                target_source_receipt: target_source_receipt.display().to_string(),
                target_medium_neutral_citation: selected.medium_neutral_citation.clone(),
                target_semantic_ref: None,
            };

            let mut next_campaign = campaign.receipt_json()?;
            next_campaign["parent_campaign_receipt"] =
                json!(trajectory.display().to_string());
            next_campaign["pending_recursive_review"] =
                serde_json::to_value(&pending)
                    .map_err(|error| format!("encode pending recursive review: {error}"))?;
            next_campaign["identity_review_worksheet"] =
                json!(identity_worksheet.display().to_string());
            next_campaign["next_operator_gate"] =
                serde_json::to_value(campaign_step(CampaignOperatorGate::AuthorityIdentityReview))
                    .map_err(|error| format!("encode identity operator gate: {error}"))?;
            next_campaign["continuation_only"] = json!(true);
            next_campaign["last_action"] = json!("governed_source_acquisition");
            next_campaign["last_acquired_medium_neutral_citation"] =
                json!(selected.medium_neutral_citation.clone());
            let next_campaign_path = output_dir.join("campaign-after-source-acquisition.json");
            write_json(&next_campaign_path, &next_campaign)?;

            let summary = json!({
                "schema_version": "sl.contract_follow.recursive_source_acquisition.v0_2",
                "selected_residual_ref": selected.residual_ref,
                "selected_medium_neutral_citation": selected.medium_neutral_citation,
                "source_receipt": output_dir.join("oalc-source-receipt.json"),
                "next_campaign_receipt": next_campaign_path,
                "identity_review_worksheet": identity_worksheet,
                "version_id": receipt.version_id,
                "corpus_revision_ref": receipt.corpus_revision_ref,
                "resolution_path": receipt.resolution_path,
                "network_requests": receipt.network_requests,
                "stream_rows_examined": receipt.stream_rows_examined,
                "stream_bytes_read": receipt.stream_bytes_read,
                "stream_terminated_after_match": receipt.stream_terminated_after_match,
                "stream_uniqueness_exhaustively_verified":
                    receipt.stream_uniqueness_exhaustively_verified,
                "range_index_hit": receipt.range_index_hit,
                "range_index_requests": receipt.range_index_requests,
                "range_index_rows_indexed_this_run":
                    receipt.range_index_rows_indexed_this_run,
                "range_index_bytes_indexed_this_run":
                    receipt.range_index_bytes_indexed_this_run,
                "range_index_byte_start": receipt.range_index_byte_start,
                "range_index_byte_len": receipt.range_index_byte_len,
                "campaign_source_acquisition_count": campaign.source_acquisitions,
                "campaign_network_request_count": campaign.network_requests,
                "campaign_budget": campaign.config.budget,
                "parent_campaign_receipt": trajectory,
                "candidate_only": true,
                "creates_legal_authority": false,
                "creates_current_law_conclusion": false,
                "missing_source_is_negative_legal_evidence": false,
            });
            let summary_path = output_dir.join("campaign-source-acquisition.json");
            write_json(&summary_path, &summary)?;
            println!(
                "contract_follow_recursive_source={} next_campaign={} citation={} network={} authority=false",
                summary_path.display(),
                next_campaign_path.display(),
                receipt.citation,
                receipt.network_requests,
            );
            Ok(())
        }
        [command, rest @ ..] if command == "identity-prepare" => {
            let receipt = PathBuf::from(required_arg(rest, "--receipt")?);
            let output = PathBuf::from(required_arg(rest, "--output")?);
            crate::contract_identity::prepare(&[receipt], &output)?;
            println!("contract_follow_identity_worksheet={}", output.display());
            Ok(())
        }
        [command, rest @ ..] if command == "identity-reviewed" => {
            let trajectory = PathBuf::from(required_arg(rest, "--trajectory")?);
            let worksheet = PathBuf::from(required_arg(rest, "--worksheet")?);
            let decisions = PathBuf::from(required_arg(rest, "--decisions")?);
            let output = PathBuf::from(required_arg(rest, "--output")?);
            crate::contract_identity::finalize(&worksheet, &decisions)?;
            let snapshot = trace_snapshot_from_trajectory(&trajectory)?;
            let trace = restore_trace(&snapshot)?;
            let compiled = crate::contract_identity::compile_against(&decisions, trace)?;
            let mut campaign = resume_from_trajectory(&trajectory)?;
            for residual in &compiled.compilation.residuals {
                campaign.preserve_reviewed_residual(
                    &decisions.display().to_string(),
                    serde_json::to_value(residual)
                        .map_err(|error| format!("encode identity residual: {error}"))?,
                );
            }
            for delta in compiled.compilation.deltas {
                campaign.accept_delta(&decisions.display().to_string(), delta)?;
            }

            let pending = pending_review_from_trajectory(&trajectory)?;
            let identity_decisions: crate::contract_identity::AuthorityIdentityDecisionFile = {
                let bytes = fs::read(&decisions)
                    .map_err(|error| format!("read {}: {error}", decisions.display()))?;
                serde_json::from_slice(&bytes)
                    .map_err(|error| format!("decode {}: {error}", decisions.display()))?
            };
            if identity_decisions.decisions.len() != 1 {
                return Err(format!(
                    "recursive identity gate requires exactly one reviewed authority, got {}",
                    identity_decisions.decisions.len()
                ));
            }
            let target_semantic_ref =
                identity_decisions.decisions[0].semantic_ref.clone();
            let review_dir = output.parent().unwrap_or_else(|| Path::new("."));
            let treatment_queue = review_dir.join("recursive-treatment-queue.json");
            let treatment_worksheet =
                review_dir.join("recursive-treatment-review-worksheet.json");
            crate::contract_treatment::prepare_exact_treatment_queue(
                Path::new(&pending.discovery_source_receipt),
                &pending.source_semantic_ref,
                &pending.target_medium_neutral_citation,
                &target_semantic_ref,
                &treatment_queue,
            )?;
            crate::contract_treatment::prepare_treatment_worksheet(
                &treatment_queue,
                &treatment_worksheet,
            )?;

            let pending = PendingRecursiveReview {
                target_semantic_ref: Some(target_semantic_ref),
                ..pending
            };
            let mut receipt = campaign.receipt_json()?;
            receipt["parent_campaign_receipt"] =
                json!(trajectory.display().to_string());
            receipt["continuation_only"] = json!(true);
            receipt["pending_recursive_review"] =
                serde_json::to_value(&pending)
                    .map_err(|error| format!("encode pending recursive review: {error}"))?;
            receipt["treatment_queue"] =
                json!(treatment_queue.display().to_string());
            receipt["treatment_review_worksheet"] =
                json!(treatment_worksheet.display().to_string());
            receipt["next_operator_gate"] =
                serde_json::to_value(campaign_step(CampaignOperatorGate::AuthorityTreatmentReview))
                    .map_err(|error| format!("encode treatment operator gate: {error}"))?;
            write_json(&output, &receipt)?;
            println!(
                "contract_follow_identity_continuation={} hops={} residuals={} authority=false",
                output.display(),
                receipt["accepted_hop_count"],
                receipt["reviewed_residual_count"],
            );
            Ok(())
        }
        [command, rest @ ..] if command == "treatment-prepare" => {
            let source_receipt = PathBuf::from(required_arg(rest, "--source-receipt")?);
            let source_semantic_ref = required_arg(rest, "--source-semantic-ref")?;
            let target_mnc = required_arg(rest, "--target-mnc")?;
            let target_semantic_ref = required_arg(rest, "--target-semantic-ref")?;
            let queue = PathBuf::from(required_arg(rest, "--queue")?);
            let worksheet = PathBuf::from(required_arg(rest, "--worksheet")?);
            let prepared = crate::contract_treatment::prepare_exact_treatment_queue(
                &source_receipt,
                &source_semantic_ref,
                &target_mnc,
                &target_semantic_ref,
                &queue,
            )?;
            crate::contract_treatment::prepare_treatment_worksheet(&queue, &worksheet)?;
            println!(
                "contract_follow_treatment_worksheet={} units={} target={} authority=false",
                worksheet.display(),
                prepared.review_unit_count,
                target_mnc,
            );
            Ok(())
        }
        [command, rest @ ..] if command == "treatment-reviewed" => {
            let trajectory = PathBuf::from(required_arg(rest, "--trajectory")?);
            let trajectory_value = read_campaign_receipt(&trajectory)?;
            let queue = arg_value(rest, "--queue")
                .map(PathBuf::from)
                .or_else(|| trajectory_value.get("treatment_queue").and_then(Value::as_str).map(PathBuf::from))
                .ok_or_else(|| "treatment-reviewed requires --queue or trajectory treatment_queue".to_string())?;
            let worksheet = arg_value(rest, "--worksheet")
                .map(PathBuf::from)
                .or_else(|| trajectory_value.get("treatment_review_worksheet").and_then(Value::as_str).map(PathBuf::from))
                .ok_or_else(|| "treatment-reviewed requires --worksheet or trajectory treatment_review_worksheet".to_string())?;
            let decisions = PathBuf::from(required_arg(rest, "--decisions")?);
            let output = PathBuf::from(required_arg(rest, "--output")?);
            crate::contract_treatment::finalize_treatment_review(&worksheet, &decisions)?;
            let snapshot = trace_snapshot_from_trajectory(&trajectory)?;
            let trace = restore_trace(&snapshot)?;
            let compiled =
                crate::contract_treatment::compile_treatment_review(&queue, &decisions, &trace)?;
            let mut campaign = resume_from_trajectory(&trajectory)?;
            for residual in &compiled.compilation.residuals {
                campaign.preserve_reviewed_residual(
                    &decisions.display().to_string(),
                    serde_json::to_value(residual)
                        .map_err(|error| format!("encode treatment residual: {error}"))?,
                );
            }
            for delta in compiled.compilation.deltas {
                campaign.accept_delta(&decisions.display().to_string(), delta)?;
            }

            let pending = pending_review_from_trajectory(&trajectory)?;
            let target_semantic_ref = pending
                .target_semantic_ref
                .clone()
                .ok_or_else(|| "treatment parent is missing reviewed target semantic identity".to_string())?;
            let (_, target_materialization) = materialize_retained_oalc_receipt(
                Path::new(&pending.target_source_receipt),
            )?;
            let next_residuals = discover_outbound_citation_residuals(
                campaign.trace(),
                &target_semantic_ref,
                &target_materialization,
            );
            let next_selected = select_fresh_outbound_citation(&next_residuals).cloned();
            let next_frontier = OutboundFrontierEnvelope {
                schema_version: "sl.contract_follow.outbound_frontier.v0_2".into(),
                parent_campaign_receipt: output.display().to_string(),
                source_receipt_path: pending.target_source_receipt.clone(),
                source_semantic_ref: target_semantic_ref,
                residual_count: next_residuals.len(),
                selected: next_selected,
                residuals: next_residuals,
                selector_is_legal_truth_rank: false,
                budget: campaign.config.budget.clone(),
                accepted_hop_count: campaign.accepted_hop_count(),
                source_acquisition_count: campaign.source_acquisitions,
                network_request_count: campaign.network_requests,
                candidate_only: true,
                creates_legal_authority: false,
                creates_current_law_conclusion: false,
            };
            let next_frontier_path = output
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("next-outbound-frontier.json");

            let mut receipt = campaign.receipt_json()?;
            receipt["parent_campaign_receipt"] =
                json!(trajectory.display().to_string());
            receipt["continuation_only"] = json!(true);
            receipt["pending_recursive_review"] = Value::Null;
            receipt["next_outbound_frontier"] =
                json!(next_frontier_path.display().to_string());
            receipt["next_operator_gate"] =
                serde_json::to_value(
                    if next_frontier.selected.is_some() {
                        campaign_step(CampaignOperatorGate::OutboundCitationAcquisition)
                    } else {
                        campaign_step(CampaignOperatorGate::None)
                    }
                )
                .map_err(|error| format!("encode next operator gate: {error}"))?;
            write_json(&output, &receipt)?;
            write_json(&next_frontier_path, &next_frontier)?;
            println!(
                "contract_follow_treatment_continuation={} hops={} residuals={} authority=false current_law_conclusion=false",
                output.display(),
                receipt["accepted_hop_count"],
                receipt["reviewed_residual_count"],
            );
            Ok(())
        }
        _ => Err(
            "usage: sensiblaw legal-follow contracts campaign <drive|discover|acquire-next|identity-prepare|identity-reviewed|treatment-prepare|treatment-reviewed> ...; drive executes deterministic outbound acquisition and stops at explicit review/terminal gates"
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_legal_follow_plan::{
        australian_contract_landscape_seed, trace_extension_delta, waltons_estoppel_trace,
    };

    #[test]
    fn campaign_persists_typed_final_trace_and_fresh_frontier() {
        let base = australian_contract_landscape_seed();
        let delta = trace_extension_delta(
            &base,
            &waltons_estoppel_trace(),
            "bootstrap:waltons-estoppel-materialisation",
        )
        .unwrap();
        let mut campaign = ContractFollowCampaign::new(
            CampaignConfig {
                campaign_ref: "campaign:test".into(),
                as_at: "2026-09-20".into(),
                jurisdiction_filter: None,
                budget: CampaignBudget::default(),
            },
            base,
        )
        .unwrap();
        let hop = campaign.accept_delta("bootstrap", delta).unwrap();
        assert!(hop.recompute_frontier_required);
        assert!(hop.old_source_history_preserved);
        assert!(!hop.old_conclusions_frozen);
        let receipt = campaign.receipt_json().unwrap();
        assert!(receipt.get("final_trace").is_some());
        assert_eq!(receipt["transport"], "typed_rust_in_process");
        assert_eq!(receipt["json_is_semantic_command_transport"], false);
    }

    #[test]
    fn snapshot_round_trip_preserves_trace() {
        let trace = waltons_estoppel_trace();
        let restored = restore_trace(&snapshot_trace(&trace)).unwrap();
        assert_eq!(trace, restored);
    }

    #[test]
    fn recursive_selector_skips_preexisting_sidhu_and_surfaces_giumelli() {
        use sensiblaw_proof_search_loop::judgment_candidates::CitationOccurrenceCandidate;

        let trace = waltons_estoppel_trace();
        let candidate = |ordinal: u64, citation: &str| CitationOccurrenceCandidate {
            document_ref: "document:oalc:nsw_caselaw:doueihi-fixture".into(),
            source_revision_ref: "oalc@fixture:nsw_caselaw:doueihi-fixture".into(),
            canonical_text_sha256: "sha256:fixture".into(),
            paragraph_ordinal: ordinal,
            paragraph_locator_ref: format!(
                "document:oalc:nsw_caselaw:doueihi-fixture#paragraph-{ordinal}"
            ),
            reported_paragraph_label: None,
            citation_text: citation.into(),
            paragraph_text: citation.into(),
            anchor_paragraph_locator_refs: vec![format!(
                "document:oalc:nsw_caselaw:doueihi-fixture#paragraph-{ordinal}"
            )],
            anchor_paragraph_texts: vec![citation.into()],
            lexical_treatment_hints: vec![],
            reviewed: false,
            candidate_only: true,
        };
        let materialization = OalcJudgmentMaterialisation {
            document_ref: "document:oalc:nsw_caselaw:doueihi-fixture".into(),
            source_revision_ref: "oalc@fixture:nsw_caselaw:doueihi-fixture".into(),
            canonical_text_sha256: "sha256:fixture".into(),
            paragraph_candidates: Vec::new(),
            citation_candidates: vec![
                candidate(10, "[2014] HCA 19"),
                candidate(20, "[1999] HCA 10"),
                candidate(30, "[1990] HCA 39"),
            ],
            candidate_only: true,
            creates_legal_authority: false,
            creates_claim_truth: false,
        };

        let residuals = discover_outbound_citation_residuals(
            &trace,
            "case:nsw:nswca:2016:105",
            &materialization,
        );
        assert!(residuals
            .iter()
            .all(|item| item.medium_neutral_citation != "[2014] HCA 19"));
        let selected = select_fresh_outbound_citation(&residuals).unwrap();
        assert_eq!(selected.medium_neutral_citation, "[1999] HCA 10");
        assert!(!selected.priority_is_legal_truth_rank);
        assert!(selected.candidate_only);
        assert!(!selected.creates_legal_authority);
    }

    #[test]
    fn resumed_campaign_cannot_reset_cumulative_hop_budget() {
        let trace = waltons_estoppel_trace();
        let config = CampaignConfig {
            campaign_ref: "campaign:budget".into(),
            as_at: "2026-09-20".into(),
            jurisdiction_filter: None,
            budget: CampaignBudget {
                max_accepted_hops: 2,
                max_source_acquisitions: 2,
                max_network_requests: 6,
            },
        };
        let campaign = ContractFollowCampaign::resume(config, trace, 2, 0, 0).unwrap();
        assert_eq!(campaign.accepted_hop_count(), 2);
        assert_eq!(
            campaign.next_fresh_step().gate,
            CampaignOperatorGate::BudgetExhausted
        );
    }

    #[test]
    fn recursive_oalc_preflight_reserves_three_request_worst_case() {
        let trace = waltons_estoppel_trace();
        let config = CampaignConfig {
            campaign_ref: "campaign:network-budget".into(),
            as_at: "2026-09-20".into(),
            jurisdiction_filter: None,
            budget: CampaignBudget {
                max_accepted_hops: 8,
                max_source_acquisitions: 2,
                max_network_requests: 4,
            },
        };
        let campaign = ContractFollowCampaign::resume(config, trace, 0, 0, 2).unwrap();
        assert!(campaign
            .ensure_source_acquisition_budget(RECURSIVE_OALC_MAX_REQUESTS_PER_ACQUISITION)
            .is_err());
    }

    #[test]
    fn campaign_drive_stops_at_identity_review_without_bypass() {
        let value = json!({
            "identity_review_worksheet": "/tmp/identity.json"
        });
        let receipt = drive_receipt_for_gate(
            Path::new("/tmp/campaign.json"),
            &value,
            CampaignOperatorGate::AuthorityIdentityReview,
        );
        assert_eq!(
            receipt.disposition,
            CampaignDriveDisposition::AwaitIdentityReview
        );
        assert_eq!(
            receipt.review_artifact.as_deref(),
            Some("/tmp/identity.json")
        );
        assert!(!receipt.deterministic_action_executed);
        assert!(!receipt.review_gate_bypassed);
        assert!(!receipt.creates_legal_authority);
    }

    #[test]
    fn campaign_drive_stops_at_treatment_review_without_bypass() {
        let value = json!({
            "treatment_review_worksheet": "/tmp/treatment.json"
        });
        let receipt = drive_receipt_for_gate(
            Path::new("/tmp/campaign.json"),
            &value,
            CampaignOperatorGate::AuthorityTreatmentReview,
        );
        assert_eq!(
            receipt.disposition,
            CampaignDriveDisposition::AwaitTreatmentReview
        );
        assert_eq!(
            receipt.review_artifact.as_deref(),
            Some("/tmp/treatment.json")
        );
        assert!(!receipt.review_gate_bypassed);
    }

    #[test]
    fn campaign_drive_classifies_terminal_gates_without_inventing_work() {
        for (gate, expected) in [
            (
                CampaignOperatorGate::BudgetExhausted,
                CampaignDriveDisposition::BudgetExhausted,
            ),
            (
                CampaignOperatorGate::None,
                CampaignDriveDisposition::Complete,
            ),
        ] {
            let receipt = drive_receipt_for_gate(
                Path::new("/tmp/campaign.json"),
                &json!({}),
                gate,
            );
            assert_eq!(receipt.disposition, expected);
            assert!(!receipt.deterministic_action_executed);
            assert!(!receipt.review_gate_bypassed);
        }
    }


    #[test]
    fn consumer_nonadequacy_routes_to_existing_campaign_gates() {
        use sensiblaw_legal_runtime::{ConsumerAxis, ConsumerResearchDemand};

        let route = |kind, axis| {
            route_consumer_research_demand(&ConsumerResearchDemand {
                axis,
                kind,
                target_ref: "case:fixture".into(),
                reason_ref: "consumer:fixture".into(),
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            })
        };

        let source = route(
            ConsumerResearchDemandKind::AcquireSource,
            ConsumerAxis::SourceRevision,
        );
        assert_eq!(source.frontier_class, CampaignFrontierClass::PrimarySource);
        assert_eq!(source.gate, CampaignOperatorGate::PrimarySourceAcquisition);

        let treatment = route(
            ConsumerResearchDemandKind::ReviewTreatment,
            ConsumerAxis::Treatment,
        );
        assert_eq!(
            treatment.frontier_class,
            CampaignFrontierClass::TreatmentReview
        );
        assert_eq!(
            treatment.gate,
            CampaignOperatorGate::AuthorityTreatmentReview
        );

        let temporal = route(
            ConsumerResearchDemandKind::ResolveTemporalCoordinate,
            ConsumerAxis::Temporal,
        );
        assert_eq!(
            temporal.frontier_class,
            CampaignFrontierClass::TemporalAlternative
        );
        assert_eq!(temporal.gate, CampaignOperatorGate::TemporalAlternative);

        let jurisdiction = route(
            ConsumerResearchDemandKind::ResolveJurisdiction,
            ConsumerAxis::Jurisdiction,
        );
        assert_eq!(
            jurisdiction.frontier_class,
            CampaignFrontierClass::ContextExpansion
        );
        assert_eq!(jurisdiction.gate, CampaignOperatorGate::ContextExpansion);

        for routed in [source, treatment, temporal, jurisdiction] {
            assert!(routed.candidate_only);
            assert!(!routed.creates_legal_authority);
            assert!(!routed.creates_current_law_conclusion);
        }
    }


    #[test]
    fn accepted_reviewed_treatment_edge_does_not_resurrect_as_fresh_review_work() {
        let mut trace = waltons_estoppel_trace();
        trace.nodes.insert(
            "case:nsw:fixture".into(),
            ContractTraceNode {
                semantic_ref: "case:nsw:fixture".into(),
                label: "Fixture appellate authority".into(),
                kind: TraceNodeKind::CaseAuthority,
                doctrine: Some(ContractDoctrine::Estoppel),
                jurisdiction_ref: "AU-NSW".into(),
                court_ref: Some("court:NSWCA".into()),
                decision_or_effective_date: Some("2016-01-01".into()),
                valid_from: None,
                valid_to: None,
                source_role: SourceRole::PrimaryCaseLaw,
                authority_level: AuthorityLevel::Official,
                source_citation: "[2016] NSWCA 999".into(),
                candidate_only: true,
                creates_legal_authority: false,
            },
        );
        trace.nodes.insert(
            "case:au:hca:fixture".into(),
            ContractTraceNode {
                semantic_ref: "case:au:hca:fixture".into(),
                label: "Fixture HCA authority".into(),
                kind: TraceNodeKind::CaseAuthority,
                doctrine: Some(ContractDoctrine::Estoppel),
                jurisdiction_ref: "AU".into(),
                court_ref: Some("court:HCA".into()),
                decision_or_effective_date: Some("1999-01-01".into()),
                valid_from: None,
                valid_to: None,
                source_role: SourceRole::PrimaryCaseLaw,
                authority_level: AuthorityLevel::Official,
                source_citation: "[1999] HCA 999".into(),
                candidate_only: true,
                creates_legal_authority: false,
            },
        );

        let mut campaign = ContractFollowCampaign::new(
            CampaignConfig {
                campaign_ref: "campaign:self-paid-treatment".into(),
                as_at: "2026-09-20".into(),
                jurisdiction_filter: None,
                budget: CampaignBudget::default(),
            },
            trace,
        )
        .unwrap();

        let delta = ContractLandscapeExpansionDelta {
            discovered_nodes: Vec::new(),
            discovered_edges: vec![ContractTraceEdge {
                from_ref: "case:nsw:fixture".into(),
                to_ref: "case:au:hca:fixture".into(),
                treatment: TreatmentKind::Supports,
                candidate_only: true,
                creates_legal_authority: false,
            }],
            provenance_ref: "reviewed-treatment-bundle:fixture".into(),
            candidate_only: true,
            creates_legal_authority: false,
        };

        let hop = campaign
            .accept_delta("reviewed-treatment-decisions:fixture", delta)
            .unwrap();

        assert_eq!(hop.added_edge_count, 1);
        assert!(hop.fresh_frontier.iter().all(|item| {
            !(item.class == CampaignFrontierClass::TreatmentReview
                && item.semantic_ref == "case:nsw:fixture"
                && item.related_ref.as_deref() == Some("case:au:hca:fixture"))
        }));
        assert_eq!(campaign.next_fresh_step().gate, CampaignOperatorGate::None);
    }

    #[test]
    fn accepted_reviewed_identity_node_does_not_resurrect_as_source_acquisition() {
        let trace = waltons_estoppel_trace();
        let mut campaign = ContractFollowCampaign::new(
            CampaignConfig {
                campaign_ref: "campaign:self-paid-identity".into(),
                as_at: "2026-09-20".into(),
                jurisdiction_filter: None,
                budget: CampaignBudget::default(),
            },
            trace,
        )
        .unwrap();

        let delta = ContractLandscapeExpansionDelta {
            discovered_nodes: vec![ContractTraceNode {
                semantic_ref: "case:au:hca:2099:1".into(),
                label: "Reviewed identity fixture".into(),
                kind: TraceNodeKind::CaseAuthority,
                doctrine: Some(ContractDoctrine::Estoppel),
                jurisdiction_ref: "AU".into(),
                court_ref: Some("court:HCA".into()),
                decision_or_effective_date: Some("2099-01-01".into()),
                valid_from: None,
                valid_to: None,
                source_role: SourceRole::PrimaryCaseLaw,
                authority_level: AuthorityLevel::Official,
                source_citation: "[2099] HCA 1".into(),
                candidate_only: true,
                creates_legal_authority: false,
            }],
            discovered_edges: Vec::new(),
            provenance_ref: "reviewed-source-identity:fixture".into(),
            candidate_only: true,
            creates_legal_authority: false,
        };

        let hop = campaign
            .accept_delta("reviewed-identity-decisions:fixture", delta)
            .unwrap();

        assert_eq!(hop.added_node_count, 1);
        assert!(hop.fresh_frontier.iter().all(|item| {
            !(item.class == CampaignFrontierClass::PrimarySource
                && item.semantic_ref == "case:au:hca:2099:1")
        }));
    }

}