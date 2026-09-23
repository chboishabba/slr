use std::collections::{BTreeMap, BTreeSet};

use postgres::GenericClient;
use sensiblaw_reader_model::{
    PersistedWorkbenchEdge, PersistedWorkbenchGraph, PersistedWorkbenchNode,
    PersistedWorkbenchProjection,
};

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
