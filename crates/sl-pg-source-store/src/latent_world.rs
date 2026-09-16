use postgres::{Client, NoTls};
use thiserror::Error;

use crate::DatabaseConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatentWorldEdgeRow {
    pub from_ref: String,
    pub to_ref: String,
    pub relation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatentWorldRows {
    pub seed_ref: String,
    pub max_hops: u32,
    pub edges: Vec<LatentWorldEdgeRow>,
    pub creates_semantic_authority: bool,
}

#[derive(Debug, Error)]
pub enum LatentWorldError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("seed ref must not be empty")]
    EmptySeed,
    #[error("max_edges must be greater than zero")]
    EmptyBudget,
    #[error("max_hops exceeds the reader safety cap of 1000")]
    HopLimitTooLarge,
}

/// Read a bounded latent semantic neighbourhood from already-existing semantic
/// relations, execution dependencies, and legal-IR provenance. This is a view:
/// it inserts nothing and does not create proof payment or authority.
pub fn load_latent_world_rows(
    config: &DatabaseConfig,
    seed_ref: &str,
    max_hops: u32,
    max_edges: usize,
) -> Result<LatentWorldRows, LatentWorldError> {
    if seed_ref.trim().is_empty() {
        return Err(LatentWorldError::EmptySeed);
    }
    if max_edges == 0 {
        return Err(LatentWorldError::EmptyBudget);
    }
    if max_hops > 1000 {
        return Err(LatentWorldError::HopLimitTooLarge);
    }

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let hop_limit = i32::try_from(max_hops).unwrap_or(1000);
    let edge_limit = i64::try_from(max_edges).unwrap_or(i64::MAX);
    let rows = client.query(
        r#"
        WITH RECURSIVE edge_source(from_ref, to_ref, relation_ref) AS (
            SELECT left_ref, right_ref, relation_type_ref
            FROM algebra.relation
            UNION ALL
            SELECT dependent_ref, prerequisite_ref, 'execution:dependency'
            FROM execution.dependency
            UNION ALL
            SELECT subject_ref, build_ref, 'legal_ir:build'
            FROM legal_ir.graph_revision
            UNION ALL
            SELECT gr.subject_ref, span_ref, 'legal_ir:source_span'
            FROM legal_ir.graph_revision AS gr,
                 LATERAL unnest(gr.source_span_refs) AS span_ref
            UNION ALL
            SELECT build_ref, document_ref, 'legal_ir:document'
            FROM legal_ir.semantic_build
            UNION ALL
            SELECT build_ref, source_revision_ref, 'legal_ir:source_revision'
            FROM legal_ir.semantic_build
            UNION ALL
            SELECT build_ref, pnf_build_ref, 'legal_ir:pnf_build'
            FROM legal_ir.semantic_build
            UNION ALL
            SELECT build_ref, refined_pnf_graph_ref, 'legal_ir:pnf_graph'
            FROM legal_ir.semantic_build
            UNION ALL
            SELECT build_ref, legal_ir_projection_ref, 'legal_ir:projection'
            FROM legal_ir.semantic_build
            UNION ALL
            SELECT p.projection_ref, o.observation_ref, 'legal_ir:observation'
            FROM legal_ir.projection AS p
            JOIN legal_ir.observation AS o
              ON o.projection_ref = p.projection_ref
            UNION ALL
            SELECT observation_ref, pnf_factor_ref, 'legal_ir:pnf_factor'
            FROM legal_ir.observation
            UNION ALL
            SELECT observation_ref, pnf_revision_ref, 'legal_ir:pnf_revision'
            FROM legal_ir.observation
            UNION ALL
            SELECT o.observation_ref, provenance_ref, 'legal_ir:provenance'
            FROM legal_ir.observation AS o,
                 LATERAL unnest(o.provenance_refs) AS provenance_ref
            UNION ALL
            SELECT o.observation_ref, residual_ref, 'legal_ir:residual'
            FROM legal_ir.observation AS o,
                 LATERAL unnest(o.residual_refs) AS residual_ref
        ),
        walk(node_ref, depth, path) AS (
            SELECT $1::text, 0::integer, ARRAY[$1::text]
            UNION ALL
            SELECT
                CASE WHEN edge_source.from_ref = walk.node_ref
                     THEN edge_source.to_ref ELSE edge_source.from_ref END,
                walk.depth + 1,
                walk.path || CASE WHEN edge_source.from_ref = walk.node_ref
                                  THEN edge_source.to_ref ELSE edge_source.from_ref END
            FROM walk
            JOIN edge_source
              ON edge_source.from_ref = walk.node_ref
              OR edge_source.to_ref = walk.node_ref
            WHERE walk.depth < $2
              AND NOT (
                  CASE WHEN edge_source.from_ref = walk.node_ref
                       THEN edge_source.to_ref ELSE edge_source.from_ref END
                  = ANY(walk.path)
              )
        ),
        visited AS (
            SELECT node_ref, MIN(depth) AS depth
            FROM walk
            GROUP BY node_ref
        )
        SELECT DISTINCT edge_source.from_ref,
                        edge_source.to_ref,
                        edge_source.relation_ref
        FROM edge_source
        JOIN visited AS left_seen ON left_seen.node_ref = edge_source.from_ref
        JOIN visited AS right_seen ON right_seen.node_ref = edge_source.to_ref
        ORDER BY edge_source.from_ref, edge_source.to_ref, edge_source.relation_ref
        LIMIT $3
        "#,
        &[&seed_ref, &hop_limit, &edge_limit],
    )?;

    Ok(LatentWorldRows {
        seed_ref: seed_ref.to_owned(),
        max_hops,
        edges: rows
            .into_iter()
            .map(|row| LatentWorldEdgeRow {
                from_ref: row.get(0),
                to_ref: row.get(1),
                relation_ref: row.get(2),
            })
            .collect(),
        creates_semantic_authority: false,
    })
}
