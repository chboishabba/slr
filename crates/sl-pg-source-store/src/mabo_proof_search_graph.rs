//! Read-only semantic graph materialisation for one existing Mabo controversy.
//! This is a persisted projection of SLR source/review/fibre/residual/proof search
//! objects, never a proof search, admission, finding or residual discharge.

use std::collections::{BTreeMap, BTreeSet};
use postgres::{types::Json, Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    build_investigation_graph_binding, load_legal_controversy_matter,
    load_legal_controversy_residual, load_legal_proof_obligation,
    load_legal_proposition_fibre, load_reviewed_evidence_coordinate,
    load_reverse_legal_proof_search,
    persist_legal_proof_investigation_graph_binding,
    DatabaseConfig, InvestigationGraphBindingStoreError,
};

pub const MABO_CONTROVERSY_REF: &str =
    "legal-controversy:sha256:c4d59a6dff1949672c6a340b40aa90b6914cf10d521eb339bd21a2d75d45b542";
const CONTRACT: &str = "slr:mabo:proof-search-graph:v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboProofGraphNode {
    pub semantic_ref: String,
    pub kind: &'static str,
    pub label: &'static str,
    pub source_ref: Option<String>,
    pub factor_revision_ref: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboProofGraphEdge {
    pub from_ref: String,
    pub to_ref: String,
    pub relation: &'static str,
    pub provenance_ref: String,
    pub evidence_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboProofGraph {
    pub matter_ref: String,
    pub controversy_ref: String,
    pub obligation_ref: String,
    pub residual_ref: String,
    pub nodes: Vec<MaboProofGraphNode>,
    pub edges: Vec<MaboProofGraphEdge>,
    pub derived_only: bool,
    pub challengeable: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub residual_resolved: bool,
    pub search_executed: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboProofGraphReceipt {
    pub matter_ref: String,
    pub controversy_ref: String,
    pub legal_obligation_ref: String,
    pub residual_ref: String,
    pub projection_ref: String,
    pub binding_ref: String,
    pub node_count: i64,
    pub edge_count: i64,
    pub derived_only: bool,
    pub challengeable: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub residual_resolved: bool,
    pub search_executed: bool,
}
#[derive(Debug, Error)]
pub enum MaboProofGraphError {
    #[error("postgres: {0}")] Pg(#[from] postgres::Error),
    #[error("binding: {0}")] Binding(#[from] InvestigationGraphBindingStoreError),
    #[error("json: {0}")] Json(#[from] serde_json::Error),
    #[error("persisted Mabo owner is absent, mismatched, promoted or not open")] WrongOwner,
    #[error("source/review/fibre/reverse-search ancestry is incomplete")] MissingAncestry,
    #[error("immutable graph replay differs from persisted graph")] ChangedReplay,
    #[error("graph projection storage is not installed")] MissingSchema,
}

fn stable_ref(prefix: &str, parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{prefix}:sha256:{:x}", hash.finalize())
}
fn node(reference: &str, kind: &'static str, label: &'static str,
        source: Option<&str>, pnf: Option<&str>) -> MaboProofGraphNode {
    MaboProofGraphNode {
        semantic_ref: reference.into(), kind, label,
        source_ref: source.map(str::to_owned),
        factor_revision_ref: pnf.map(str::to_owned),
    }
}
fn edge(from: &str, to: &str, relation: &'static str,
        provenance: &str, evidence: &str) -> MaboProofGraphEdge {
    MaboProofGraphEdge {
        from_ref: from.into(), to_ref: to.into(), relation,
        provenance_ref: provenance.into(), evidence_ref: evidence.into(),
    }
}
fn insert_node(nodes: &mut BTreeMap<String, MaboProofGraphNode>,
               value: MaboProofGraphNode) -> Result<(), MaboProofGraphError> {
    if value.semantic_ref.trim().is_empty() {
        return Err(MaboProofGraphError::MissingAncestry);
    }
    if let Some(prior) = nodes.insert(value.semantic_ref.clone(), value.clone()) {
        if prior != value { return Err(MaboProofGraphError::MissingAncestry); }
    }
    Ok(())
}

impl MaboProofGraph {
    /// Deterministic refusal guard; never reinterprets formal-premise payment
    /// as a merits decision, truth, applicability or a completed search.
    pub fn validate(&self) -> Result<(), MaboProofGraphError> {
        if self.controversy_ref != MABO_CONTROVERSY_REF
            || self.matter_ref.trim().is_empty()
            || self.obligation_ref.trim().is_empty()
            || self.residual_ref.trim().is_empty()
            || self.nodes.is_empty() || self.edges.is_empty()
            || !self.derived_only || !self.challengeable
            || self.creates_semantic_authority || self.creates_legal_authority
            || self.applicability_promoted || self.claim_truth_promoted
            || self.residual_resolved || self.search_executed {
            return Err(MaboProofGraphError::WrongOwner);
        }
        let refs = self.nodes.iter().map(|n| n.semantic_ref.as_str())
            .collect::<BTreeSet<_>>();
        if refs.len() != self.nodes.len() ||
            self.edges.iter().any(|e|
                !refs.contains(e.from_ref.as_str()) ||
                !refs.contains(e.to_ref.as_str()) ||
                e.provenance_ref.is_empty() || e.evidence_ref.is_empty()) {
            return Err(MaboProofGraphError::MissingAncestry);
        }
        Ok(())
    }
}

/// Reopens only persisted legal objects and their source/review antecedents.
/// No paper or judicial proposition is inferred from a filename or search result.
pub fn derive_mabo_proof_search_graph(
    config: &DatabaseConfig,
    controversy_ref: &str,
    obligation_ref: &str,
) -> Result<MaboProofGraph, MaboProofGraphError> {
    if controversy_ref != MABO_CONTROVERSY_REF {
        return Err(MaboProofGraphError::WrongOwner);
    }
    let matter = load_legal_controversy_matter(config, controversy_ref)
        .map_err(|_| MaboProofGraphError::WrongOwner)?;
    let obligation = load_legal_proof_obligation(config, obligation_ref)
        .map_err(|_| MaboProofGraphError::WrongOwner)?;
    if obligation.controversy_ref != controversy_ref
        || !matter.obligation_refs.contains(&obligation.obligation_ref)
        || !obligation.is_open || !obligation.candidate_only
        || obligation.creates_semantic_authority || obligation.creates_legal_authority
        || obligation.applicability_promoted || obligation.claim_truth_promoted
        || !matter.candidate_only || matter.creates_semantic_authority
        || matter.creates_legal_authority || matter.applicability_promoted
        || matter.claim_truth_promoted {
        return Err(MaboProofGraphError::WrongOwner);
    }
    let residual = load_legal_controversy_residual(config, &obligation.required_by)
        .map_err(|_| MaboProofGraphError::MissingAncestry)?;
    if residual.controversy_ref != controversy_ref
        || !matter.residual_refs.contains(&residual.residual_ref)
        || !residual.candidate_only || residual.creates_semantic_authority
        || residual.creates_legal_authority || residual.applicability_promoted
        || residual.claim_truth_promoted
        || ![&residual.applicant_fibre_ref, &residual.respondent_fibre_ref]
            .contains(&&obligation.proposition_fibre_ref) {
        return Err(MaboProofGraphError::MissingAncestry);
    }

    let mut nodes = BTreeMap::new();
    let mut edges = Vec::new();
    let mut reviewed_pnf_refs = BTreeSet::<String>::new();
    let mut reviewed_source_refs = BTreeSet::<String>::new();
    insert_node(&mut nodes, node(&matter.controversy_ref, "legal_controversy",
        "Reviewed controversy", None, None))?;
    insert_node(&mut nodes, node(&residual.residual_ref, "unresolved_legal_residual",
        "Unresolved legal residual", None, None))?;
    insert_node(&mut nodes, node(&obligation.obligation_ref, "open_legal_proof_obligation",
        "Open proof obligation", None, None))?;
    edges.push(edge(&matter.controversy_ref, &residual.residual_ref,
        "controversy_contains_residual", &matter.controversy_ref, &residual.residual_ref));
    edges.push(edge(&residual.residual_ref, &obligation.obligation_ref,
        "residual_requires_proof", &residual.residual_ref, &obligation.obligation_ref));

    for fibre_ref in [&residual.applicant_fibre_ref, &residual.respondent_fibre_ref] {
        let fibre = load_legal_proposition_fibre(config, fibre_ref)
            .map_err(|_| MaboProofGraphError::MissingAncestry)?;
        let reviewed_ref = fibre.reviewed_evidence_ref.as_deref()
            .ok_or(MaboProofGraphError::MissingAncestry)?;
        let reviewed = load_reviewed_evidence_coordinate(config, reviewed_ref)
            .map_err(|_| MaboProofGraphError::MissingAncestry)?;
        if !matter.proposition_fibre_refs.contains(&fibre.fibre_ref)
            || !fibre.candidate_only || fibre.creates_semantic_authority
            || fibre.creates_legal_authority || fibre.applicability_promoted
            || fibre.claim_truth_promoted || !reviewed.candidate_only
            || reviewed.creates_semantic_authority
            || reviewed.applicability_promoted || reviewed.claim_truth_promoted
            || fibre.relation_reference != reviewed.proposition_ref
            || fibre.source_reference != reviewed.exact_span_ref {
            return Err(MaboProofGraphError::MissingAncestry);
        }
        reviewed_pnf_refs.insert(reviewed.pnf_revision_ref.clone());
        reviewed_source_refs.insert(reviewed.source_revision_ref.clone());
        insert_node(&mut nodes, node(&reviewed.source_revision_ref,
            "source_revision", "Primary source revision",
            Some(&reviewed.source_revision_ref), None))?;
        insert_node(&mut nodes, node(&reviewed.reviewed_evidence_ref,
            "reviewed_evidence", "Reviewed source evidence",
            Some(&reviewed.source_revision_ref), Some(&reviewed.pnf_revision_ref)))?;
        insert_node(&mut nodes, node(&fibre.fibre_ref, "proposition_fibre",
            "Reviewed proposition fibre", Some(&reviewed.source_revision_ref), None))?;
        edges.push(edge(&reviewed.source_revision_ref, &reviewed.reviewed_evidence_ref,
            "source_reviewed_as", &reviewed.review_receipt_ref, &reviewed.exact_span_ref));
        edges.push(edge(&reviewed.reviewed_evidence_ref, &fibre.fibre_ref,
            "reviewed_evidence_anchors_fibre", &reviewed.review_receipt_ref,
            &reviewed.reviewed_evidence_ref));
        edges.push(edge(&fibre.fibre_ref, &residual.residual_ref,
            "fibre_participates_in_residual", &fibre.fibre_ref,
            &reviewed.reviewed_evidence_ref));
    }

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let reverses = client.query(
        "SELECT reverse_ref FROM legal_controversy.reverse_search
         WHERE controversy_ref=$1 AND $2=ANY(open_obligation_refs)
           AND $3=ANY(candidate_residual_refs)
           AND candidate_only AND NOT creates_semantic_authority
           AND NOT creates_legal_authority
           AND NOT applicability_promoted AND NOT claim_truth_promoted
         ORDER BY reverse_ref",
        &[&controversy_ref, &obligation_ref, &residual.residual_ref],
    )?;
    if reverses.is_empty() {
        return Err(MaboProofGraphError::MissingAncestry);
    }
    for row in reverses {
        let reverse_ref: String = row.get(0);
        let reverse = load_reverse_legal_proof_search(config, &reverse_ref)
            .map_err(|_| MaboProofGraphError::MissingAncestry)?;
        if reverse.controversy_ref != controversy_ref
            || !reverse.open_obligation_refs.contains(&obligation.obligation_ref)
            || !reverse.candidate_residual_refs.contains(&residual.residual_ref)
            || !reverse.candidate_only || reverse.creates_semantic_authority
            || reverse.creates_legal_authority || reverse.applicability_promoted
            || reverse.claim_truth_promoted {
            return Err(MaboProofGraphError::MissingAncestry);
        }
        insert_node(&mut nodes, node(&reverse.reverse_ref, "reverse_proof_search",
            "Reverse search target (not executed)", None, None))?;
        edges.push(edge(&obligation.obligation_ref, &reverse.reverse_ref,
            "reverse_search_targets_open_obligation", &residual.residual_ref,
            &reverse.reverse_ref));
    }

    // Formal payment is a distinct, exact formal-premise fact, not a discharge.
    // Only show it when both demand and independent kernel evidence/payment
    // already exist and their digest/obligation joins agree.
    if client.query_one("SELECT to_regclass('formal_bridge.demand')::text",
        &[])?.get::<_, Option<String>>(0).is_some() {
        for row in client.query(
            "SELECT d.demand_ref, d.statement_digest, d.pnf_revision_ref,
                    d.source_revision_refs, p.evidence_ref
             FROM formal_bridge.demand d
             LEFT JOIN formal_bridge.payment p
               ON p.demand_ref=d.demand_ref AND p.obligation_ref=d.obligation_ref
              AND p.statement_digest=d.statement_digest AND p.formal_premise_paid
              AND NOT p.creates_legal_authority
              AND NOT p.applicability_promoted AND NOT p.claim_truth_promoted
             WHERE d.obligation_ref=$1 AND d.residual_ref=$2
               AND d.candidate_only AND NOT d.creates_semantic_authority
               AND NOT d.creates_legal_authority
               AND NOT d.applicability_promoted AND NOT d.claim_truth_promoted
             ORDER BY d.demand_ref",
            &[&obligation_ref, &residual.residual_ref],
        )? {
            let demand_ref: String = row.get(0);
            let statement_digest: String = row.get(1);
            let pnf_ref: String = row.get(2);
            let demand_sources: Vec<String> = row.get(3);
            let payment_ref: Option<String> = row.get(4);
            if !reviewed_pnf_refs.contains(&pnf_ref)
                || !demand_sources.iter().any(|r| reviewed_source_refs.contains(r)) {
                return Err(MaboProofGraphError::MissingAncestry);
            }
            let factor_row = client.query_opt(
                "SELECT encode(factor_sha256,'hex') FROM algebra.factor_revision
                 WHERE factor_revision_ref=$1", &[&pnf_ref],
            )?.ok_or(MaboProofGraphError::MissingAncestry)?;
            let factor_hex: String = factor_row.get(0);
            if format!("sha256:{factor_hex}") != statement_digest {
                return Err(MaboProofGraphError::MissingAncestry);
            }
            insert_node(&mut nodes, node(&demand_ref, "formal_proof_demand",
                "Formal premise demand", None, Some(&pnf_ref)))?;
            edges.push(edge(&obligation_ref, &demand_ref, "formal_demand_for_obligation",
                &residual.residual_ref, &statement_digest));
            if let Some(evidence_ref) = payment_ref {
                let certified = client.query_opt(
                    "SELECT 1 FROM formal_bridge.evidence
                     WHERE evidence_ref=$1 AND demand_ref=$2
                       AND obligation_ref=$3 AND statement_digest=$4
                       AND certification_state='kernel-certified'
                       AND establishes_formal_premise
                       AND NOT creates_legal_authority
                       AND NOT applicability_promoted AND NOT claim_truth_promoted",
                    &[&evidence_ref, &demand_ref, &obligation_ref, &statement_digest],
                )?;
                if certified.is_none() { return Err(MaboProofGraphError::MissingAncestry); }
                insert_node(&mut nodes, node(&evidence_ref, "kernel_formal_premise_evidence",
                    "Formal premise paid (not legal truth)", None, Some(&pnf_ref)))?;
                edges.push(edge(&demand_ref, &evidence_ref,
                    "kernel_certified_formal_premise_payment",
                    &demand_ref, &statement_digest));
            }
        }
    }
    let result = MaboProofGraph {
        matter_ref: matter.matter_ref,
        controversy_ref: matter.controversy_ref,
        obligation_ref: obligation.obligation_ref,
        residual_ref: residual.residual_ref,
        nodes: nodes.into_values().collect(),
        edges,
        derived_only: true, challengeable: true,
        creates_semantic_authority: false, creates_legal_authority: false,
        applicability_promoted: false, claim_truth_promoted: false,
        residual_resolved: false, search_executed: false,
    };
    result.validate()?;
    Ok(result)
}

/// Persist only the canonical legal-follow tables and the already-owned
/// InvestigationGraphBinding. Reopen all stored rows to detect stale replay.
pub fn persist_mabo_proof_search_graph(
    config: &DatabaseConfig,
    graph: &MaboProofGraph,
) -> Result<MaboProofGraphReceipt, MaboProofGraphError> {
    graph.validate()?;
    // Prevent callers from writing a synthetic or selectively-pruned graph.
    if derive_mabo_proof_search_graph(config, &graph.controversy_ref,
        &graph.obligation_ref)? != *graph {
        return Err(MaboProofGraphError::ChangedReplay);
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let source_revision_refs = graph
        .nodes
        .iter()
        .filter_map(|node| node.source_ref.as_deref())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let document_rows = client.query(
        "SELECT DISTINCT document_ref
         FROM source_provenance.legal_source_manifestation
         WHERE source_revision_ref = ANY($1)
           AND candidate_only
           AND NOT creates_semantic_authority
           AND NOT creates_legal_authority
           AND NOT applicability_promoted
           AND NOT claim_truth_promoted
         ORDER BY document_ref",
        &[&source_revision_refs],
    )?;
    if document_rows.len() != 1 {
        return Err(MaboProofGraphError::MissingAncestry);
    }
    let document_ref: String = document_rows[0].get(0);
    for table in ["pnf_follow_projection", "pnf_follow_node",
                  "pnf_follow_edge", "pnf_follow_edge_provenance"] {
        if client.query_one("SELECT to_regclass($1)::text", &[&table])?
            .get::<_,Option<String>>(0).is_none() {
            return Err(MaboProofGraphError::MissingSchema);
        }
    }
    let mut identity = Vec::new();
    identity.extend(graph.nodes.iter().map(|n| n.semantic_ref.clone()));
    identity.extend(graph.edges.iter().map(|e|
        format!("{}|{}|{}|{}|{}", e.from_ref, e.to_ref, e.relation,
            e.provenance_ref, e.evidence_ref)));
    let refs = identity.iter().map(String::as_str).collect::<Vec<_>>();
    let payload = serde_json::json!({
        "schema": CONTRACT, "matter_ref": graph.matter_ref,
        "controversy_ref": graph.controversy_ref,
        "obligation_ref": graph.obligation_ref, "residual_ref": graph.residual_ref,
        "evidence_ancestry": identity,
        "derived_only": true, "challengeable": true,
        "creates_semantic_authority": false, "creates_legal_authority": false,
        "applicability_promoted": false, "claim_truth_promoted": false,
        "residual_resolved": false, "search_executed": false
    });
    let payload_text = serde_json::to_string(&payload)?;
    let projection_ref = stable_ref("mabo-proof-search-legal-follow",
        &[CONTRACT, &graph.matter_ref, &graph.obligation_ref, &graph.residual_ref,
          &payload_text, &refs.join("\u{1f}")]);
    let projection_sha: Vec<u8> = Sha256::digest(payload_text.as_bytes()).to_vec();
    let mut tx = client.transaction()?;
    tx.execute(
        "INSERT INTO pnf_follow_projection
         (projection_ref,document_ref,projection_kind,authority_ceiling,
          promotion_allowed,execution_allowed,payload,projection_sha256)
         VALUES($1,$2,'legal_follow','derived_only_challengeable',FALSE,FALSE,$3::jsonb,$4)
         ON CONFLICT(projection_ref) DO NOTHING",
        &[&projection_ref, &document_ref, &Json(&payload), &projection_sha],
    )?;
    for n in &graph.nodes {
        tx.execute(
            "INSERT INTO pnf_follow_node
             (projection_ref,node_ref,node_kind,label,document_ref,
              factor_revision_ref,domain_ir_ref,source_record_ref)
             VALUES($1,$2,$3,$4,NULL,$5,NULL,$6)
             ON CONFLICT(projection_ref,node_ref) DO NOTHING",
            &[&projection_ref,&n.semantic_ref,&n.kind,&n.label,
              &n.factor_revision_ref,&n.source_ref],
        )?;
    }
    for e in &graph.edges {
        let edge_ref = stable_ref("mabo-proof-search-edge",
            &[&projection_ref,&e.from_ref,&e.to_ref,e.relation,
              &e.provenance_ref,&e.evidence_ref]);
        tx.execute(
            "INSERT INTO pnf_follow_edge
             (edge_ref,projection_ref,from_node_ref,to_node_ref,
              relation_kind,admissibility_state)
             VALUES($1,$2,$3,$4,$5,'challengeable')
             ON CONFLICT(edge_ref) DO NOTHING",
            &[&edge_ref,&projection_ref,&e.from_ref,&e.to_ref,&e.relation],
        )?;
        tx.execute(
            "INSERT INTO pnf_follow_edge_provenance
             (edge_ref,provenance_ref,evidence_ref)
             VALUES($1,$2,$3) ON CONFLICT DO NOTHING",
            &[&edge_ref,&e.provenance_ref,&e.evidence_ref],
        )?;
    }
    tx.commit()?;

    let row = client.query_one(
        "SELECT projection_sha256,authority_ceiling,promotion_allowed,execution_allowed
         FROM pnf_follow_projection WHERE projection_ref=$1",
        &[&projection_ref],
    )?;
    let count_nodes: i64 = client.query_one(
        "SELECT COUNT(*)::bigint FROM pnf_follow_node WHERE projection_ref=$1",
        &[&projection_ref])?.get(0);
    let count_edges: i64 = client.query_one(
        "SELECT COUNT(*)::bigint FROM pnf_follow_edge WHERE projection_ref=$1",
        &[&projection_ref])?.get(0);
    if row.get::<_,Vec<u8>>(0) != projection_sha
        || row.get::<_,String>(1) != "derived_only_challengeable"
        || row.get::<_,bool>(2) || row.get::<_,bool>(3)
        || count_nodes != graph.nodes.len() as i64
        || count_edges != graph.edges.len() as i64 {
        return Err(MaboProofGraphError::ChangedReplay);
    }
    // A conflict-ignored INSERT is not a replay receipt. Verify every
    // persisted node, edge and provenance tuple, not only row counts.
    for n in &graph.nodes {
        let row = client.query_opt(
            "SELECT node_kind,label,document_ref,factor_revision_ref,
                    domain_ir_ref,source_record_ref FROM pnf_follow_node
             WHERE projection_ref=$1 AND node_ref=$2",
            &[&projection_ref, &n.semantic_ref],
        )?.ok_or(MaboProofGraphError::ChangedReplay)?;
        if row.get::<_, String>(0) != n.kind
            || row.get::<_, String>(1) != n.label
            || row.get::<_, Option<String>>(2).is_some()
            || row.get::<_, Option<String>>(3) != n.factor_revision_ref
            || row.get::<_, Option<String>>(4).is_some()
            || row.get::<_, Option<String>>(5) != n.source_ref {
            return Err(MaboProofGraphError::ChangedReplay);
        }
    }
    let mut expected_provenance_count = 0_i64;
    for e in &graph.edges {
        let edge_ref = stable_ref("mabo-proof-search-edge",
            &[&projection_ref,&e.from_ref,&e.to_ref,e.relation,
              &e.provenance_ref,&e.evidence_ref]);
        let row = client.query_opt(
            "SELECT from_node_ref,to_node_ref,relation_kind,admissibility_state
             FROM pnf_follow_edge WHERE projection_ref=$1 AND edge_ref=$2",
            &[&projection_ref,&edge_ref],
        )?.ok_or(MaboProofGraphError::ChangedReplay)?;
        if row.get::<_,String>(0)!=e.from_ref
            || row.get::<_,String>(1)!=e.to_ref
            || row.get::<_,String>(2)!=e.relation
            || row.get::<_,String>(3)!="challengeable" {
            return Err(MaboProofGraphError::ChangedReplay);
        }
        let provenance = client.query(
            "SELECT provenance_ref,evidence_ref FROM pnf_follow_edge_provenance
             WHERE edge_ref=$1", &[&edge_ref],
        )?;
        if provenance.len()!=1
            || provenance[0].get::<_,String>(0)!=e.provenance_ref
            || provenance[0].get::<_,String>(1)!=e.evidence_ref {
            return Err(MaboProofGraphError::ChangedReplay);
        }
        expected_provenance_count += 1;
    }
    let actual_provenance_count: i64 = client.query_one(
        "SELECT COUNT(*)::bigint FROM pnf_follow_edge_provenance p
         JOIN pnf_follow_edge e ON e.edge_ref=p.edge_ref
         WHERE e.projection_ref=$1",
        &[&projection_ref],
    )?.get(0);
    if actual_provenance_count!=expected_provenance_count {
        return Err(MaboProofGraphError::ChangedReplay);
    }

    let binding_ref = stable_ref("mabo-proof-search-graph-binding",
        &[CONTRACT,&graph.obligation_ref,&projection_ref,&graph.matter_ref]);
    let mut evidence_refs = graph.edges.iter().flat_map(|e|
        [&e.provenance_ref,&e.evidence_ref]).cloned().collect::<Vec<_>>();
    evidence_refs.sort();
    evidence_refs.dedup();
    let binding = build_investigation_graph_binding(
        &binding_ref,&graph.obligation_ref,&projection_ref,&graph.matter_ref,
        evidence_refs).map_err(|_| MaboProofGraphError::MissingAncestry)?;
    persist_legal_proof_investigation_graph_binding(config,&binding)?;
    Ok(MaboProofGraphReceipt {
        matter_ref: graph.matter_ref.clone(),
        controversy_ref: graph.controversy_ref.clone(),
        legal_obligation_ref: graph.obligation_ref.clone(),
        residual_ref: graph.residual_ref.clone(),
        projection_ref, binding_ref, node_count: count_nodes,
        edge_count: count_edges, derived_only: true, challengeable: true,
        creates_semantic_authority: false, creates_legal_authority: false,
        applicability_promoted: false, claim_truth_promoted: false,
        residual_resolved: false, search_executed: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_foreign_controversy_and_promoted_graph() {
        let mut g = MaboProofGraph {
            matter_ref: "matter:mabo".into(), controversy_ref: MABO_CONTROVERSY_REF.into(),
            obligation_ref: "legal-obligation:real".into(),
            residual_ref: "legal-residual:real".into(),
            nodes: vec![node("source:one","source_revision","Source",None,None),
                        node("obligation:one","open_obligation","Obligation",None,None)],
            edges: vec![edge("source:one","obligation:one","supports","review:one","source:one")],
            derived_only: true,challengeable: true,
            creates_semantic_authority:false,creates_legal_authority:false,
            applicability_promoted:false,claim_truth_promoted:false,
            residual_resolved:false,search_executed:false,
        };
        assert!(g.validate().is_ok());
        g.claim_truth_promoted=true;
        assert!(g.validate().is_err());
        g.claim_truth_promoted=false;
        g.search_executed=true;
        assert!(g.validate().is_err());
        g.search_executed=false;
        g.residual_resolved=true;
        assert!(g.validate().is_err());
        g.residual_resolved=false;
        g.controversy_ref="legal-controversy:foreign".into();
        assert!(g.validate().is_err());
    }
    #[test]
    fn refuses_dangling_graph_and_missing_evidence_ancestry() {
        let g = MaboProofGraph {
            matter_ref:"matter:mabo".into(),controversy_ref:MABO_CONTROVERSY_REF.into(),
            obligation_ref:"obligation:real".into(),residual_ref:"residual:real".into(),
            nodes: vec![node("a","source_revision","Source",None,None)],
            edges: vec![edge("a","nonexistent","unknown","review:one","evidence:one")],
            derived_only:true,challengeable:true,
            creates_semantic_authority:false,creates_legal_authority:false,
            applicability_promoted:false,claim_truth_promoted:false,
            residual_resolved:false,search_executed:false,
        };
        assert!(matches!(g.validate(),Err(MaboProofGraphError::MissingAncestry)));
    }
}
