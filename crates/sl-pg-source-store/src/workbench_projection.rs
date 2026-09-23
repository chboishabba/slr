use std::collections::{BTreeMap, BTreeSet};

use postgres::GenericClient;
use sensiblaw_reader_model::{
    PersistedWorkbenchEdge, PersistedWorkbenchGraph, PersistedWorkbenchNode,
    PersistedWorkbenchProjection,
};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalFollowProjectionSummary {
    pub projection_ref: String,
    pub document_ref: String,
    pub created_at: String,
    pub node_count: i64,
    pub edge_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsecutiveProjectionPair {
    pub document_ref: String,
    pub before_projection_ref: String,
    pub after_projection_ref: String,
    pub before_created_at: String,
    pub after_created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsecutiveProjectionTriple {
    pub document_ref: String,
    pub w0_projection_ref: String,
    pub w1_projection_ref: String,
    pub w2_projection_ref: String,
    pub w0_created_at: String,
    pub w1_created_at: String,
    pub w2_created_at: String,
}

pub fn list_legal_follow_projection_summaries<C: GenericClient>(
    client: &mut C,
    limit: i64,
) -> Result<Vec<LegalFollowProjectionSummary>, WorkbenchProjectionError> {
    if limit <= 0 {
        return Err(WorkbenchProjectionError::InvalidProjection(
            "projection discovery limit must be positive".into(),
        ));
    }
    let rows = client.query(
        "
        SELECT p.projection_ref,
               p.document_ref,
               p.created_at::text AS created_at,
               COUNT(DISTINCT n.node_ref)::bigint AS node_count,
               COUNT(DISTINCT e.edge_ref)::bigint AS edge_count
        FROM pnf_follow_projection p
        JOIN pnf_follow_node n ON n.projection_ref = p.projection_ref
        LEFT JOIN pnf_follow_edge e ON e.projection_ref = p.projection_ref
        WHERE p.projection_kind = 'legal_follow'
          AND p.authority_ceiling = 'derived_only_challengeable'
          AND p.promotion_allowed = FALSE
          AND p.execution_allowed = FALSE
        GROUP BY p.projection_ref, p.document_ref, p.created_at
        ORDER BY p.document_ref, p.created_at, p.projection_ref
        LIMIT $1
        ",
        &[&limit],
    )?;
    Ok(rows
        .into_iter()
        .map(|row| LegalFollowProjectionSummary {
            projection_ref: row.get("projection_ref"),
            document_ref: row.get("document_ref"),
            created_at: row.get("created_at"),
            node_count: row.get("node_count"),
            edge_count: row.get("edge_count"),
        })
        .collect())
}

pub fn consecutive_projection_pairs(
    summaries: &[LegalFollowProjectionSummary],
) -> Vec<ConsecutiveProjectionPair> {
    let mut pairs = Vec::new();
    for window in summaries.windows(2) {
        let before = &window[0];
        let after = &window[1];
        if before.document_ref != after.document_ref {
            continue;
        }
        pairs.push(ConsecutiveProjectionPair {
            document_ref: before.document_ref.clone(),
            before_projection_ref: before.projection_ref.clone(),
            after_projection_ref: after.projection_ref.clone(),
            before_created_at: before.created_at.clone(),
            after_created_at: after.created_at.clone(),
        });
    }
    pairs
}

pub fn consecutive_projection_triples(
    summaries: &[LegalFollowProjectionSummary],
) -> Vec<ConsecutiveProjectionTriple> {
    let mut triples = Vec::new();
    for window in summaries.windows(3) {
        let w0 = &window[0];
        let w1 = &window[1];
        let w2 = &window[2];
        if w0.document_ref != w1.document_ref || w1.document_ref != w2.document_ref {
            continue;
        }
        triples.push(ConsecutiveProjectionTriple {
            document_ref: w0.document_ref.clone(),
            w0_projection_ref: w0.projection_ref.clone(),
            w1_projection_ref: w1.projection_ref.clone(),
            w2_projection_ref: w2.projection_ref.clone(),
            w0_created_at: w0.created_at.clone(),
            w1_created_at: w1.created_at.clone(),
            w2_created_at: w2.created_at.clone(),
        });
    }
    triples
}

#[derive(Debug, thiserror::Error)]
pub enum WorkbenchProjectionError {
    #[error("projection_ref and world_ref must be non-empty")]
    InvalidIdentity,
    #[error("legal follow projection not found: {0}")]
    ProjectionNotFound(String),
    #[error("postgres workbench query failed: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("typed workbench projection invalid: {0}")]
    InvalidProjection(String),
}

pub fn load_persisted_workbench_projection<C: GenericClient>(
    client: &mut C,
    projection_ref: &str,
    world_ref: &str,
) -> Result<PersistedWorkbenchProjection, WorkbenchProjectionError> {
    if projection_ref.trim().is_empty() || world_ref.trim().is_empty() {
        return Err(WorkbenchProjectionError::InvalidIdentity);
    }

    // Deliberately select only typed row columns. The serialized projection
    // blob is not part of the production semantic carrier.
    let projection = client
        .query_opt(
            "
            SELECT projection_ref, document_ref, authority_ceiling,
                   promotion_allowed, execution_allowed
            FROM pnf_follow_projection
            WHERE projection_ref = $1
              AND projection_kind = 'legal_follow'
            ",
            &[&projection_ref],
        )?
        .ok_or_else(|| WorkbenchProjectionError::ProjectionNotFound(projection_ref.into()))?;

    let authority_ceiling: String = projection.get("authority_ceiling");
    let promotion_allowed: bool = projection.get("promotion_allowed");
    let execution_allowed: bool = projection.get("execution_allowed");
    if authority_ceiling != "derived_only_challengeable"
        || promotion_allowed
        || execution_allowed
    {
        return Err(WorkbenchProjectionError::InvalidProjection(
            "persisted legal-follow projection crossed derived-only authority boundary".into(),
        ));
    }

    let document_ref: String = projection.get("document_ref");

    let node_rows = client.query(
        "
        SELECT node_ref, node_kind, label, document_ref,
               factor_revision_ref, domain_ir_ref, source_record_ref
        FROM pnf_follow_node
        WHERE projection_ref = $1
        ORDER BY node_ref
        ",
        &[&projection_ref],
    )?;

    if node_rows.is_empty() {
        return Err(WorkbenchProjectionError::InvalidProjection(
            "persisted legal-follow projection has no typed nodes".into(),
        ));
    }

    let mut nodes = Vec::with_capacity(node_rows.len());
    let mut source_refs = BTreeSet::new();
    for row in node_rows {
        let semantic_ref: String = row.get("node_ref");
        let node_document_ref: Option<String> = row.get("document_ref");
        let factor_revision_ref: Option<String> = row.get("factor_revision_ref");
        let domain_ir_ref: Option<String> = row.get("domain_ir_ref");
        let source_record_ref: Option<String> = row.get("source_record_ref");

        let mut node_sources = BTreeSet::new();
        if let Some(reference) = source_record_ref.filter(|value| !value.trim().is_empty()) {
            node_sources.insert(reference);
        }
        if let Some(reference) = node_document_ref.filter(|value| !value.trim().is_empty()) {
            node_sources.insert(reference);
        }
        source_refs.extend(node_sources.iter().cloned());

        let mut provenance_refs = BTreeSet::from([projection_ref.to_owned()]);
        if let Some(reference) = factor_revision_ref.filter(|value| !value.trim().is_empty()) {
            provenance_refs.insert(reference);
        }
        if let Some(reference) = domain_ir_ref.filter(|value| !value.trim().is_empty()) {
            provenance_refs.insert(reference);
        }

        nodes.push(PersistedWorkbenchNode {
            semantic_ref,
            semantic_kind: row.get("node_kind"),
            label: row.get("label"),
            source_refs: node_sources.into_iter().collect(),
            provenance_refs: provenance_refs.into_iter().collect(),
            candidate_only: true,
        });
    }

    let edge_rows = client.query(
        "
        SELECT edge_ref, from_node_ref, to_node_ref, relation_kind,
               admissibility_state
        FROM pnf_follow_edge
        WHERE projection_ref = $1
        ORDER BY edge_ref
        ",
        &[&projection_ref],
    )?;

    let provenance_rows = client.query(
        "
        SELECT provenance.edge_ref, provenance.provenance_ref,
               provenance.evidence_ref
        FROM pnf_follow_edge_provenance provenance
        JOIN pnf_follow_edge edge ON edge.edge_ref = provenance.edge_ref
        WHERE edge.projection_ref = $1
        ORDER BY provenance.edge_ref, provenance.provenance_ref, provenance.evidence_ref
        ",
        &[&projection_ref],
    )?;

    let mut edge_provenance = BTreeMap::<String, BTreeSet<String>>::new();
    let mut edge_sources = BTreeMap::<String, BTreeSet<String>>::new();
    for row in provenance_rows {
        let edge_ref: String = row.get("edge_ref");
        let provenance_ref: String = row.get("provenance_ref");
        if !provenance_ref.trim().is_empty() {
            edge_provenance
                .entry(edge_ref.clone())
                .or_default()
                .insert(provenance_ref);
        }
        let evidence_ref: Option<String> = row.get("evidence_ref");
        if let Some(reference) = evidence_ref.filter(|value| !value.trim().is_empty()) {
            edge_sources.entry(edge_ref).or_default().insert(reference);
        }
    }

    let mut edges = Vec::with_capacity(edge_rows.len());
    for row in edge_rows {
        let edge_ref: String = row.get("edge_ref");
        let admissibility_state: String = row.get("admissibility_state");
        edges.push(PersistedWorkbenchEdge {
            semantic_ref: edge_ref.clone(),
            from_ref: row.get("from_node_ref"),
            to_ref: row.get("to_node_ref"),
            relation: row.get("relation_kind"),
            source_refs: edge_sources
                .remove(&edge_ref)
                .unwrap_or_default()
                .into_iter()
                .collect(),
            provenance_refs: edge_provenance
                .remove(&edge_ref)
                .unwrap_or_default()
                .into_iter()
                .collect(),
            challengeable: admissibility_state == "challengeable",
            candidate_only: true,
        });
    }

    let typed = PersistedWorkbenchProjection {
        world_ref: world_ref.into(),
        source_refs: source_refs.into_iter().collect(),
        event_refs: Vec::new(),
        handoff_refs: Vec::new(),
        research_residual_refs: Vec::new(),
        legal_follow_graph: PersistedWorkbenchGraph {
            projection_ref: projection_ref.into(),
            document_ref,
            nodes,
            edges,
            derived_only: true,
            challengeable: true,
        },
        candidate_only: true,
        projection_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
        pays_residual: false,
    };
    typed
        .validate_read_only()
        .map_err(WorkbenchProjectionError::InvalidProjection)?;
    Ok(typed)
}

#[cfg(test)]
mod tests {

    #[test]
    fn discovery_windows_only_follow_consecutive_revisions_of_same_document() {
        let summaries = vec![
            LegalFollowProjectionSummary {
                projection_ref: "p0".into(),
                document_ref: "doc:a".into(),
                created_at: "2026-09-23 10:00:00+10".into(),
                node_count: 1,
                edge_count: 0,
            },
            LegalFollowProjectionSummary {
                projection_ref: "p1".into(),
                document_ref: "doc:a".into(),
                created_at: "2026-09-23 11:00:00+10".into(),
                node_count: 2,
                edge_count: 1,
            },
            LegalFollowProjectionSummary {
                projection_ref: "p2".into(),
                document_ref: "doc:a".into(),
                created_at: "2026-09-23 12:00:00+10".into(),
                node_count: 3,
                edge_count: 2,
            },
            LegalFollowProjectionSummary {
                projection_ref: "q0".into(),
                document_ref: "doc:b".into(),
                created_at: "2026-09-23 13:00:00+10".into(),
                node_count: 1,
                edge_count: 0,
            },
        ];

        let pairs = consecutive_projection_pairs(&summaries);
        let triples = consecutive_projection_triples(&summaries);

        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].before_projection_ref, "p0");
        assert_eq!(pairs[0].after_projection_ref, "p1");
        assert_eq!(pairs[1].before_projection_ref, "p1");
        assert_eq!(pairs[1].after_projection_ref, "p2");

        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].w0_projection_ref, "p0");
        assert_eq!(triples[0].w1_projection_ref, "p1");
        assert_eq!(triples[0].w2_projection_ref, "p2");
    }

    #[test]
    fn production_loader_module_contains_no_json_payload_dependency() {
        let source = include_str!("workbench_projection.rs");
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source before test module");
        assert!(!production_source.contains("serde_json::"));
        assert!(!production_source.contains("SELECT payload"));
    }
}
