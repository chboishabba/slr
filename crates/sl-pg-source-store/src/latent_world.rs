use std::collections::BTreeSet;

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

const NEIGHBOUR_SQL: &str = r#"
WITH edge_source(from_ref, to_ref, relation_ref) AS (
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
)
SELECT DISTINCT from_ref, to_ref, relation_ref
FROM edge_source
WHERE from_ref = ANY($1::text[]) OR to_ref = ANY($1::text[])
ORDER BY from_ref, to_ref, relation_ref
LIMIT $2
"#;

/// Read a bounded latent semantic neighbourhood from already-existing semantic
/// relations, execution dependencies, and legal-IR provenance. Traversal is
/// performed as bounded Rust BFS over batched neighbour queries so the declared
/// edge budget constrains database work rather than only final display output.
/// This inserts nothing and creates no proof payment or authority.
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
    let mut visited_nodes = BTreeSet::from([seed_ref.to_owned()]);
    let mut frontier = vec![seed_ref.to_owned()];
    let mut edge_keys: BTreeSet<(String, String, String)> = BTreeSet::new();

    for _depth in 0..max_hops {
        if frontier.is_empty() || edge_keys.len() >= max_edges {
            break;
        }
        let remaining = max_edges - edge_keys.len();
        let query_limit = i64::try_from(remaining).unwrap_or(i64::MAX);
        let rows = client.query(NEIGHBOUR_SQL, &[&frontier, &query_limit])?;
        let mut next_frontier = BTreeSet::new();

        for row in rows {
            let from_ref: String = row.get(0);
            let to_ref: String = row.get(1);
            let relation_ref: String = row.get(2);
            edge_keys.insert((from_ref.clone(), to_ref.clone(), relation_ref));

            for candidate in [from_ref, to_ref] {
                if visited_nodes.insert(candidate.clone()) {
                    next_frontier.insert(candidate);
                }
            }
            if edge_keys.len() >= max_edges {
                break;
            }
        }
        frontier = next_frontier.into_iter().collect();
    }

    Ok(LatentWorldRows {
        seed_ref: seed_ref.to_owned(),
        max_hops,
        edges: edge_keys
            .into_iter()
            .map(|(from_ref, to_ref, relation_ref)| LatentWorldEdgeRow {
                from_ref,
                to_ref,
                relation_ref,
            })
            .collect(),
        creates_semantic_authority: false,
    })
}
