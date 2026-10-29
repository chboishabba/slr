use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::{context_federation::CONTEXT_RECEIPT_SCHEMA_SQL, DatabaseConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatentWorldBudget {
    pub max_hops: u32,
    pub max_nodes: usize,
    pub max_edges: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatentWorldEdgeRow {
    pub from_ref: String,
    pub to_ref: String,
    pub relation_ref: String,
    /// Persisted row/source coordinates that caused this edge to exist. These
    /// retain traversal lineage only; they do not create legal authority.
    pub provenance_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatentWorldRows {
    pub seed_ref: String,
    /// Compatibility alias for the requested hop cap.
    pub max_hops: u32,
    pub requested_max_hops: u32,
    pub visited_refs: Vec<String>,
    pub deepest_observed_hop: u32,
    pub frontier_exhausted: bool,
    pub frontier_refs: Vec<String>,
    pub residual_refs: Vec<String>,
    pub edges: Vec<LatentWorldEdgeRow>,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum LatentWorldError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("seed ref must not be empty")]
    EmptySeed,
    #[error("max_nodes and max_edges must both be greater than zero")]
    EmptyBudget,
    #[error("max_hops exceeds the reader safety cap of 1000")]
    HopLimitTooLarge,
    #[error("invalid context revision coordinate for {source_ref}: {source_revision_ref}")]
    InvalidContextRevisionCoordinate {
        source_ref: String,
        source_revision_ref: String,
    },
}

const NEIGHBOUR_SQL: &str = r#"
WITH edge_source(from_ref, to_ref, relation_ref, provenance_ref) AS (
    SELECT relation.left_ref, relation.right_ref, relation.relation_type_ref,
           COALESCE(
               concat('context:', receipt.source_family_ref, ':', receipt.source_revision_ref),
               concat('algebra.relation:', relation.left_ref, ':', relation.relation_type_ref, ':', relation.right_ref)
           )
    FROM algebra.relation AS relation
    LEFT JOIN context.reviewed_relation_receipt AS receipt
      ON receipt.relation_ref = relation.relation_ref
    UNION ALL
    SELECT dependent_ref, prerequisite_ref, 'execution:dependency',
           concat('execution.dependency:', dependent_ref, ':', prerequisite_ref)
    FROM execution.dependency
    UNION ALL
    SELECT subject_ref, build_ref, 'legal_ir:build', build_ref
    FROM legal_ir.graph_revision
    UNION ALL
    SELECT gr.subject_ref, span_ref, 'legal_ir:source_span', span_ref
    FROM legal_ir.graph_revision AS gr,
         LATERAL unnest(gr.source_span_refs) AS span_ref
    UNION ALL
    SELECT build_ref, document_ref, 'legal_ir:document', build_ref
    FROM legal_ir.semantic_build
    UNION ALL
    SELECT build_ref, source_revision_ref, 'legal_ir:source_revision', source_revision_ref
    FROM legal_ir.semantic_build
    UNION ALL
    SELECT build_ref, pnf_build_ref, 'legal_ir:pnf_build', pnf_build_ref
    FROM legal_ir.semantic_build
    UNION ALL
    SELECT build_ref, refined_pnf_graph_ref, 'legal_ir:pnf_graph', refined_pnf_graph_ref
    FROM legal_ir.semantic_build
    UNION ALL
    SELECT build_ref, legal_ir_projection_ref, 'legal_ir:projection', legal_ir_projection_ref
    FROM legal_ir.semantic_build
    UNION ALL
    SELECT p.projection_ref, o.observation_ref, 'legal_ir:observation', o.observation_ref
    FROM legal_ir.projection AS p
    JOIN legal_ir.observation AS o
      ON o.projection_ref = p.projection_ref
    UNION ALL
    SELECT observation_ref, pnf_factor_ref, 'legal_ir:pnf_factor', observation_ref
    FROM legal_ir.observation
    UNION ALL
    SELECT observation_ref, pnf_revision_ref, 'legal_ir:pnf_revision', observation_ref
    FROM legal_ir.observation
    UNION ALL
    SELECT o.observation_ref, provenance_ref, 'legal_ir:provenance', provenance_ref
    FROM legal_ir.observation AS o,
         LATERAL unnest(o.provenance_refs) AS provenance_ref
    UNION ALL
    SELECT o.observation_ref, residual_ref, 'legal_ir:residual', residual_ref
    FROM legal_ir.observation AS o,
         LATERAL unnest(o.residual_refs) AS residual_ref
)
SELECT DISTINCT from_ref, to_ref, relation_ref, provenance_ref
FROM edge_source
WHERE from_ref = ANY($1::text[]) OR to_ref = ANY($1::text[])
ORDER BY from_ref, to_ref, relation_ref, provenance_ref
LIMIT $2
"#;

const REVISION_SLICED_NEIGHBOUR_SQL: &str = r#"
WITH selected_wikidata_revision(source_ref, source_revision_ref) AS (
    SELECT source_ref, source_revision_ref
    FROM unnest($3::text[], $4::text[]) AS selected(source_ref, source_revision_ref)
),
edge_source(from_ref, to_ref, relation_ref, provenance_ref) AS (
    SELECT relation.left_ref, relation.right_ref, relation.relation_type_ref,
           COALESCE(
               concat('context:', receipt.source_family_ref, ':', receipt.source_revision_ref),
               concat('algebra.relation:', relation.left_ref, ':', relation.relation_type_ref, ':', relation.right_ref)
           )
    FROM algebra.relation AS relation
    LEFT JOIN context.reviewed_relation_receipt AS receipt
      ON receipt.relation_ref = relation.relation_ref
    LEFT JOIN selected_wikidata_revision AS selected
      ON selected.source_ref = relation.left_ref
    WHERE receipt.relation_ref IS NULL
       OR receipt.source_family_ref <> 'wikidata'
       OR (
            receipt.source_family_ref = 'wikidata'
        AND selected.source_ref IS NOT NULL
        AND receipt.source_revision_ref = selected.source_revision_ref
       )
    UNION ALL
    SELECT dependent_ref, prerequisite_ref, 'execution:dependency',
           concat('execution.dependency:', dependent_ref, ':', prerequisite_ref)
    FROM execution.dependency
    UNION ALL
    SELECT subject_ref, build_ref, 'legal_ir:build', build_ref
    FROM legal_ir.graph_revision
    UNION ALL
    SELECT gr.subject_ref, span_ref, 'legal_ir:source_span', span_ref
    FROM legal_ir.graph_revision AS gr,
         LATERAL unnest(gr.source_span_refs) AS span_ref
    UNION ALL
    SELECT build_ref, document_ref, 'legal_ir:document', build_ref
    FROM legal_ir.semantic_build
    UNION ALL
    SELECT build_ref, source_revision_ref, 'legal_ir:source_revision', source_revision_ref
    FROM legal_ir.semantic_build
    UNION ALL
    SELECT build_ref, pnf_build_ref, 'legal_ir:pnf_build', pnf_build_ref
    FROM legal_ir.semantic_build
    UNION ALL
    SELECT build_ref, refined_pnf_graph_ref, 'legal_ir:pnf_graph', refined_pnf_graph_ref
    FROM legal_ir.semantic_build
    UNION ALL
    SELECT build_ref, legal_ir_projection_ref, 'legal_ir:projection', legal_ir_projection_ref
    FROM legal_ir.semantic_build
    UNION ALL
    SELECT p.projection_ref, o.observation_ref, 'legal_ir:observation', o.observation_ref
    FROM legal_ir.projection AS p
    JOIN legal_ir.observation AS o
      ON o.projection_ref = p.projection_ref
    UNION ALL
    SELECT observation_ref, pnf_factor_ref, 'legal_ir:pnf_factor', observation_ref
    FROM legal_ir.observation
    UNION ALL
    SELECT observation_ref, pnf_revision_ref, 'legal_ir:pnf_revision', observation_ref
    FROM legal_ir.observation
    UNION ALL
    SELECT o.observation_ref, provenance_ref, 'legal_ir:provenance', provenance_ref
    FROM legal_ir.observation AS o,
         LATERAL unnest(o.provenance_refs) AS provenance_ref
    UNION ALL
    SELECT o.observation_ref, residual_ref, 'legal_ir:residual', residual_ref
    FROM legal_ir.observation AS o,
         LATERAL unnest(o.residual_refs) AS residual_ref
)
SELECT DISTINCT from_ref, to_ref, relation_ref, provenance_ref
FROM edge_source
WHERE from_ref = ANY($1::text[]) OR to_ref = ANY($1::text[])
ORDER BY from_ref, to_ref, relation_ref, provenance_ref
LIMIT $2
"#;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContextRevisionWorldSlice {
    /// Wikidata source QID -> exact reviewed source revision ref selected into
    /// this world. Historical reviewed revisions remain durable but invisible
    /// to context traversal for this slice.
    pub wikidata_source_revisions: BTreeMap<String, String>,
}

impl ContextRevisionWorldSlice {
    pub fn validate(&self) -> Result<(), LatentWorldError> {
        for (source_ref, revision_ref) in &self.wikidata_source_revisions {
            let fields = revision_ref.split(':').collect::<Vec<_>>();
            let valid_qid = source_ref
                .strip_prefix('Q')
                .is_some_and(|digits| {
                    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
                });
            let valid_revision = fields.len() == 4
                && fields[0] == "wikidata"
                && fields[1] == source_ref
                && fields[2] == "oldid"
                && fields[3]
                    .parse::<u64>()
                    .is_ok_and(|revision_id| revision_id > 0);
            if !valid_qid || !valid_revision {
                return Err(LatentWorldError::InvalidContextRevisionCoordinate {
                    source_ref: source_ref.clone(),
                    source_revision_ref: revision_ref.clone(),
                });
            }
        }
        Ok(())
    }

    fn parallel_arrays(&self) -> (Vec<String>, Vec<String>) {
        self.wikidata_source_revisions
            .iter()
            .map(|(source, revision)| (source.clone(), revision.clone()))
            .unzip()
    }
}

/// Read a deterministic, bounded latent semantic neighbourhood from already-
/// existing semantic relations, execution dependencies, and legal-IR lineage.
/// Traversal policy lives in Rust; PostgreSQL remains persistence/query
/// infrastructure. The returned frontier makes truncation explicit.
pub fn load_latent_world_rows_with_budget(
    config: &DatabaseConfig,
    seed_ref: &str,
    budget: LatentWorldBudget,
) -> Result<LatentWorldRows, LatentWorldError> {
    load_latent_world_rows_with_budget_and_context_slice(
        config,
        seed_ref,
        budget,
        None,
    )
}

pub fn load_latent_world_rows_with_context_revision_slice(
    config: &DatabaseConfig,
    seed_ref: &str,
    budget: LatentWorldBudget,
    context_slice: &ContextRevisionWorldSlice,
) -> Result<LatentWorldRows, LatentWorldError> {
    context_slice.validate()?;
    load_latent_world_rows_with_budget_and_context_slice(
        config,
        seed_ref,
        budget,
        Some(context_slice),
    )
}

fn load_latent_world_rows_with_budget_and_context_slice(
    config: &DatabaseConfig,
    seed_ref: &str,
    budget: LatentWorldBudget,
    context_slice: Option<&ContextRevisionWorldSlice>,
) -> Result<LatentWorldRows, LatentWorldError> {
    if seed_ref.trim().is_empty() {
        return Err(LatentWorldError::EmptySeed);
    }
    if budget.max_nodes == 0 || budget.max_edges == 0 {
        return Err(LatentWorldError::EmptyBudget);
    }
    if budget.max_hops > 1000 {
        return Err(LatentWorldError::HopLimitTooLarge);
    }

    let mut client = Client::connect(config.database_url(), NoTls)?;
    // The receipt table is infrastructure only. Its boolean checks preserve
    // the candidate-only firewall; traversal remains read-only after this
    // idempotent schema guard.
    client.batch_execute(CONTEXT_RECEIPT_SCHEMA_SQL)?;
    let mut visited_depths = BTreeMap::from([(seed_ref.to_owned(), 0_u32)]);
    let mut frontier = vec![seed_ref.to_owned()];
    let mut edge_keys: BTreeMap<(String, String, String), BTreeSet<String>> = BTreeMap::new();
    let mut retained_frontier = BTreeSet::new();
    let mut residual_refs = BTreeSet::new();
    let mut deepest_observed_hop = 0_u32;
    let mut stopped_for_budget = false;

    for depth in 0..budget.max_hops {
        if frontier.is_empty() {
            break;
        }
        if edge_keys.len() >= budget.max_edges {
            retained_frontier.extend(frontier.iter().cloned());
            residual_refs.insert("world-residual:edge-budget".to_owned());
            stopped_for_budget = true;
            break;
        }

        let remaining = budget.max_edges - edge_keys.len();
        let query_limit = i64::try_from(remaining.saturating_add(1)).unwrap_or(i64::MAX);
        let rows = if let Some(context_slice) = context_slice {
            let (source_refs, revision_refs) = context_slice.parallel_arrays();
            client.query(
                REVISION_SLICED_NEIGHBOUR_SQL,
                &[&frontier, &query_limit, &source_refs, &revision_refs],
            )?
        } else {
            client.query(NEIGHBOUR_SQL, &[&frontier, &query_limit])?
        };
        let edge_budget_hit = rows.len() > remaining;
        if edge_budget_hit {
            residual_refs.insert("world-residual:edge-budget".to_owned());
            retained_frontier.extend(frontier.iter().cloned());
            for row in rows.iter().skip(remaining) {
                retained_frontier.insert(row.get::<_, String>(0));
                retained_frontier.insert(row.get::<_, String>(1));
            }
        }

        let mut next_frontier = BTreeSet::new();
        let mut node_budget_hit = false;

        for row in rows.into_iter().take(remaining) {
            let from_ref: String = row.get(0);
            let to_ref: String = row.get(1);
            let relation_ref: String = row.get(2);
            let provenance_ref: String = row.get(3);

            let new_candidate = if !visited_depths.contains_key(&from_ref) {
                Some(from_ref.clone())
            } else if !visited_depths.contains_key(&to_ref) {
                Some(to_ref.clone())
            } else {
                None
            };

            if let Some(candidate) = new_candidate {
                if visited_depths.len() >= budget.max_nodes {
                    node_budget_hit = true;
                    retained_frontier.insert(candidate);
                    continue;
                }
                let next_depth = depth + 1;
                visited_depths.insert(candidate.clone(), next_depth);
                deepest_observed_hop = deepest_observed_hop.max(next_depth);
                next_frontier.insert(candidate);
            }

            edge_keys
                .entry((from_ref, to_ref, relation_ref))
                .or_default()
                .insert(provenance_ref);
        }

        if node_budget_hit {
            residual_refs.insert("world-residual:node-budget".to_owned());
            retained_frontier.extend(frontier.iter().cloned());
            retained_frontier.extend(next_frontier.iter().cloned());
        }

        frontier = next_frontier.into_iter().collect();
        if node_budget_hit || edge_budget_hit {
            stopped_for_budget = true;
            retained_frontier.extend(frontier.iter().cloned());
            break;
        }
    }

    if !stopped_for_budget && !frontier.is_empty() {
        retained_frontier.extend(frontier.iter().cloned());
        residual_refs.insert("world-residual:hop-limit".to_owned());
    }

    let frontier_exhausted = retained_frontier.is_empty() && frontier.is_empty();
    if frontier_exhausted {
        residual_refs.clear();
    }

    let mut visited_refs: Vec<String> = visited_depths.keys().cloned().collect();
    visited_refs.sort_by(|left, right| {
        visited_depths[left]
            .cmp(&visited_depths[right])
            .then_with(|| left.cmp(right))
    });

    Ok(LatentWorldRows {
        seed_ref: seed_ref.to_owned(),
        max_hops: budget.max_hops,
        requested_max_hops: budget.max_hops,
        visited_refs,
        deepest_observed_hop,
        frontier_exhausted,
        frontier_refs: retained_frontier.into_iter().collect(),
        residual_refs: residual_refs.into_iter().collect(),
        edges: edge_keys
            .into_iter()
            .map(
                |((from_ref, to_ref, relation_ref), provenance_refs)| LatentWorldEdgeRow {
                    from_ref,
                    to_ref,
                    relation_ref,
                    provenance_refs: provenance_refs.into_iter().collect(),
                },
            )
            .collect(),
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

/// Compatibility wrapper for the original #18 latent-world API. Existing
/// callers keep their edge-budget behavior while gaining the richer receipt.
pub fn load_latent_world_rows(
    config: &DatabaseConfig,
    seed_ref: &str,
    max_hops: u32,
    max_edges: usize,
) -> Result<LatentWorldRows, LatentWorldError> {
    load_latent_world_rows_with_budget(
        config,
        seed_ref,
        LatentWorldBudget {
            max_hops,
            max_nodes: max_edges.saturating_add(1),
            max_edges,
        },
    )
}

#[cfg(test)]
mod revision_slice_tests {
    use super::*;

    #[test]
    fn context_revision_slice_requires_exact_same_qid_revision() {
        let valid = ContextRevisionWorldSlice {
            wikidata_source_revisions: BTreeMap::from([(
                "Q1501525".into(),
                "wikidata:Q1501525:oldid:2333409615".into(),
            )]),
        };
        assert!(valid.validate().is_ok());

        let invalid = ContextRevisionWorldSlice {
            wikidata_source_revisions: BTreeMap::from([(
                "Q1501525".into(),
                "wikidata:Q999:oldid:2333409615".into(),
            )]),
        };
        assert!(matches!(
            invalid.validate(),
            Err(LatentWorldError::InvalidContextRevisionCoordinate { .. })
        ));
    }

    #[test]
    fn context_revision_slice_parallel_arrays_are_stable() {
        let slice = ContextRevisionWorldSlice {
            wikidata_source_revisions: BTreeMap::from([
                ("Q2".into(), "wikidata:Q2:oldid:20".into()),
                ("Q1".into(), "wikidata:Q1:oldid:10".into()),
            ]),
        };
        let (sources, revisions) = slice.parallel_arrays();
        assert_eq!(sources, vec!["Q1", "Q2"]);
        assert_eq!(
            revisions,
            vec!["wikidata:Q1:oldid:10", "wikidata:Q2:oldid:20"]
        );
    }
}
