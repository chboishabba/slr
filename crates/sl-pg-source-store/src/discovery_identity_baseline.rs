use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::{non_novel_identity_alias::NON_NOVEL_IDENTITY_ALIAS_SCHEMA_SQL, DatabaseConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryIdentityBaselineRow {
    pub object_ref: String,
    pub identity_class_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscoveryIdentityBaseline {
    pub identity_class_refs: BTreeSet<String>,
    pub representation_identity_class_refs: BTreeMap<String, String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DiscoveryIdentityBaselineError {
    #[error("durable discovery identity coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("one durable representation maps to two identity classes")]
    RepresentationIdentityConflict {
        object_ref: String,
        existing_identity_class_ref: String,
        conflicting_identity_class_ref: String,
    },
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for DiscoveryIdentityBaselineError {
    fn from(value: postgres::Error) -> Self {
        Self::Postgres(value.to_string())
    }
}

pub fn collapse_discovery_identity_baseline(
    rows: &[DiscoveryIdentityBaselineRow],
) -> Result<DiscoveryIdentityBaseline, DiscoveryIdentityBaselineError> {
    let mut baseline = DiscoveryIdentityBaseline::default();
    for row in rows {
        if row.object_ref.trim().is_empty() {
            return Err(DiscoveryIdentityBaselineError::EmptyCoordinate("object_ref"));
        }
        if row.identity_class_ref.trim().is_empty() {
            return Err(DiscoveryIdentityBaselineError::EmptyCoordinate("identity_class_ref"));
        }
        if let Some(existing) = baseline.representation_identity_class_refs.get(&row.object_ref) {
            if existing != &row.identity_class_ref {
                return Err(DiscoveryIdentityBaselineError::RepresentationIdentityConflict {
                    object_ref: row.object_ref.clone(),
                    existing_identity_class_ref: existing.clone(),
                    conflicting_identity_class_ref: row.identity_class_ref.clone(),
                });
            }
        }
        baseline
            .representation_identity_class_refs
            .insert(row.object_ref.clone(), row.identity_class_ref.clone());
        baseline.identity_class_refs.insert(row.identity_class_ref.clone());
    }
    Ok(baseline)
}

/// Load the durable identity quotient from both novel discovery lineage and
/// reviewed non-novel aliases. Alias rows contribute representation coverage
/// only: because the baseline cardinality is a set of identity-class refs, an
/// alias to an existing class cannot increment durable novelty.
pub fn load_discovery_identity_baseline(
    config: &DatabaseConfig,
) -> Result<DiscoveryIdentityBaseline, DiscoveryIdentityBaselineError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(NON_NOVEL_IDENTITY_ALIAS_SCHEMA_SQL)?;
    let rows = client.query(
        "SELECT object_ref, identity_class_ref FROM (\
           SELECT object_ref, identity_class_ref \
           FROM context.discovery_lineage_receipt \
           WHERE identity_class_ref IS NOT NULL \
             AND candidate_only = TRUE \
             AND creates_semantic_authority = FALSE \
             AND applicability_promoted = FALSE \
             AND claim_truth_promoted = FALSE \
           UNION ALL \
           SELECT representation_ref AS object_ref, identity_class_ref \
           FROM context.reviewed_identity_alias_receipt \
           WHERE candidate_only = TRUE \
             AND creates_semantic_authority = FALSE \
             AND applicability_promoted = FALSE \
             AND claim_truth_promoted = FALSE \
             AND counts_as_novel_discovery = FALSE \
             AND creates_discovery_lineage = FALSE\
         ) AS durable_identity \
         ORDER BY identity_class_ref, object_ref",
        &[],
    )?;
    let rows = rows
        .into_iter()
        .map(|row| DiscoveryIdentityBaselineRow {
            object_ref: row.get::<_, String>(0),
            identity_class_ref: row.get::<_, String>(1),
        })
        .collect::<Vec<_>>();
    collapse_discovery_identity_baseline(&rows)
}
