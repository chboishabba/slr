use postgres::{Client, NoTls, Transaction};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedPnfRevision {
    pub review_receipt_ref: String,
    pub document_ref: String,
    pub exact_span_ref: String,
    pub graph_ref: String,
    pub graph_type_ref: String,
    pub schema_version_ref: String,
    pub graph_closure_state_ref: String,
    pub factor_ref: String,
    pub factor_revision_ref: String,
    pub factor_type_ref: String,
    pub factor_closure_state_ref: String,
    pub graph_role_ref: String,
}

impl ReviewedPnfRevision {
    #[must_use]
    pub const fn proposition_support_paid(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn applicability_paid(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn claim_truth_paid(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReviewedPnfValidationError {
    #[error("required reviewed PNF coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
}

#[derive(Debug, Error)]
pub enum ReviewedPnfMaterializationError {
    #[error(transparent)]
    Invalid(#[from] ReviewedPnfValidationError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("reviewed PNF exact span does not belong to the declared document")]
    SourceSpanMismatch,
    #[error("existing reviewed PNF row conflicts with the requested coordinate: {0}")]
    ExistingRowConflict(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedPnfMaterializationReceipt {
    pub graph_ref: String,
    pub factor_ref: String,
    pub factor_revision_ref: String,
    pub review_receipt_ref: String,
    pub proposition_support_paid: bool,
    pub applicability_paid: bool,
    pub claim_truth_paid: bool,
}

pub fn validate_reviewed_pnf_revision(
    reviewed: &ReviewedPnfRevision,
) -> Result<(), ReviewedPnfValidationError> {
    for (name, value) in [
        ("review_receipt_ref", reviewed.review_receipt_ref.as_str()),
        ("document_ref", reviewed.document_ref.as_str()),
        ("exact_span_ref", reviewed.exact_span_ref.as_str()),
        ("graph_ref", reviewed.graph_ref.as_str()),
        ("graph_type_ref", reviewed.graph_type_ref.as_str()),
        ("schema_version_ref", reviewed.schema_version_ref.as_str()),
        (
            "graph_closure_state_ref",
            reviewed.graph_closure_state_ref.as_str(),
        ),
        ("factor_ref", reviewed.factor_ref.as_str()),
        ("factor_revision_ref", reviewed.factor_revision_ref.as_str()),
        ("factor_type_ref", reviewed.factor_type_ref.as_str()),
        (
            "factor_closure_state_ref",
            reviewed.factor_closure_state_ref.as_str(),
        ),
        ("graph_role_ref", reviewed.graph_role_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ReviewedPnfValidationError::EmptyCoordinate(name));
        }
    }
    Ok(())
}

/// Persist an explicitly reviewed PNF revision into the existing generic PNF
/// schema. This function does not review candidates, derive proposition support,
/// or create legal/reader authority.
pub fn materialize_reviewed_pnf_revision(
    config: &DatabaseConfig,
    reviewed: &ReviewedPnfRevision,
) -> Result<ReviewedPnfMaterializationReceipt, ReviewedPnfMaterializationError> {
    validate_reviewed_pnf_revision(reviewed)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    require_owned_span(&mut tx, reviewed)?;

    let factor_digest = digest(&[
        "reviewed-pnf-factor:v1",
        &reviewed.review_receipt_ref,
        &reviewed.document_ref,
        &reviewed.exact_span_ref,
        &reviewed.factor_ref,
        &reviewed.factor_type_ref,
    ]);
    tx.execute(
        r#"
        INSERT INTO algebra.factor
          (factor_ref, document_ref, factor_type_ref, closure_state_ref, factor_sha256)
        VALUES ($1,$2,$3,$4,$5)
        ON CONFLICT (factor_ref) DO NOTHING
        "#,
        &[
            &reviewed.factor_ref,
            &reviewed.document_ref,
            &reviewed.factor_type_ref,
            &reviewed.factor_closure_state_ref,
            &&factor_digest[..],
        ],
    )?;

    let revision_digest = digest(&[
        "reviewed-pnf-factor-revision:v1",
        &reviewed.review_receipt_ref,
        &reviewed.factor_ref,
        &reviewed.factor_revision_ref,
        &reviewed.factor_closure_state_ref,
        &reviewed.exact_span_ref,
    ]);
    tx.execute(
        r#"
        INSERT INTO algebra.factor_revision
          (factor_revision_ref, factor_ref, closure_state_ref, factor_sha256)
        VALUES ($1,$2,$3,$4)
        ON CONFLICT (factor_revision_ref) DO NOTHING
        "#,
        &[
            &reviewed.factor_revision_ref,
            &reviewed.factor_ref,
            &reviewed.factor_closure_state_ref,
            &&revision_digest[..],
        ],
    )?;

    let graph_digest = digest(&[
        "reviewed-pnf-graph:v1",
        &reviewed.review_receipt_ref,
        &reviewed.document_ref,
        &reviewed.graph_ref,
        &reviewed.graph_type_ref,
        &reviewed.schema_version_ref,
        &reviewed.graph_closure_state_ref,
    ]);
    tx.execute(
        r#"
        INSERT INTO pnf.graph
          (graph_ref, document_ref, graph_type_ref, schema_version_ref,
           closure_state_ref, graph_sha256)
        VALUES ($1,$2,$3,$4,$5,$6)
        ON CONFLICT (graph_ref) DO NOTHING
        "#,
        &[
            &reviewed.graph_ref,
            &reviewed.document_ref,
            &reviewed.graph_type_ref,
            &reviewed.schema_version_ref,
            &reviewed.graph_closure_state_ref,
            &&graph_digest[..],
        ],
    )?;

    tx.execute(
        r#"
        INSERT INTO pnf.graph_factor_revision
          (graph_ref, factor_revision_ref, graph_role_ref)
        VALUES ($1,$2,$3)
        ON CONFLICT DO NOTHING
        "#,
        &[
            &reviewed.graph_ref,
            &reviewed.factor_revision_ref,
            &reviewed.graph_role_ref,
        ],
    )?;

    verify_rows(&mut tx, reviewed)?;
    tx.commit()?;

    Ok(ReviewedPnfMaterializationReceipt {
        graph_ref: reviewed.graph_ref.clone(),
        factor_ref: reviewed.factor_ref.clone(),
        factor_revision_ref: reviewed.factor_revision_ref.clone(),
        review_receipt_ref: reviewed.review_receipt_ref.clone(),
        proposition_support_paid: false,
        applicability_paid: false,
        claim_truth_paid: false,
    })
}

fn require_owned_span(
    tx: &mut Transaction<'_>,
    reviewed: &ReviewedPnfRevision,
) -> Result<(), ReviewedPnfMaterializationError> {
    let present: bool = tx
        .query_one(
            r#"
            SELECT EXISTS (
              SELECT 1
              FROM corpus.span
              WHERE span_ref=$1 AND document_ref=$2
            )
            "#,
            &[&reviewed.exact_span_ref, &reviewed.document_ref],
        )?
        .get(0);
    if !present {
        return Err(ReviewedPnfMaterializationError::SourceSpanMismatch);
    }
    Ok(())
}

fn verify_rows(
    tx: &mut Transaction<'_>,
    reviewed: &ReviewedPnfRevision,
) -> Result<(), ReviewedPnfMaterializationError> {
    let factor = tx.query_one(
        "SELECT document_ref, factor_type_ref FROM algebra.factor WHERE factor_ref=$1",
        &[&reviewed.factor_ref],
    )?;
    if factor.get::<_, Option<String>>(0).as_deref() != Some(reviewed.document_ref.as_str())
        || factor.get::<_, String>(1) != reviewed.factor_type_ref
    {
        return Err(ReviewedPnfMaterializationError::ExistingRowConflict("factor"));
    }

    let revision = tx.query_one(
        "SELECT factor_ref, closure_state_ref FROM algebra.factor_revision WHERE factor_revision_ref=$1",
        &[&reviewed.factor_revision_ref],
    )?;
    if revision.get::<_, String>(0) != reviewed.factor_ref
        || revision.get::<_, String>(1) != reviewed.factor_closure_state_ref
    {
        return Err(ReviewedPnfMaterializationError::ExistingRowConflict(
            "factor_revision",
        ));
    }

    let graph = tx.query_one(
        "SELECT document_ref, graph_type_ref, schema_version_ref, closure_state_ref FROM pnf.graph WHERE graph_ref=$1",
        &[&reviewed.graph_ref],
    )?;
    if graph.get::<_, String>(0) != reviewed.document_ref
        || graph.get::<_, String>(1) != reviewed.graph_type_ref
        || graph.get::<_, String>(2) != reviewed.schema_version_ref
        || graph.get::<_, String>(3) != reviewed.graph_closure_state_ref
    {
        return Err(ReviewedPnfMaterializationError::ExistingRowConflict("graph"));
    }

    let weld: bool = tx
        .query_one(
            r#"
            SELECT EXISTS (
              SELECT 1 FROM pnf.graph_factor_revision
              WHERE graph_ref=$1 AND factor_revision_ref=$2 AND graph_role_ref=$3
            )
            "#,
            &[
                &reviewed.graph_ref,
                &reviewed.factor_revision_ref,
                &reviewed.graph_role_ref,
            ],
        )?
        .get(0);
    if !weld {
        return Err(ReviewedPnfMaterializationError::ExistingRowConflict(
            "graph_factor_revision",
        ));
    }
    Ok(())
}

fn digest(parts: &[&str]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        let bytes = part.as_bytes();
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    hasher.finalize().into()
}
