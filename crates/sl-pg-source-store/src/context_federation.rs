//! Revision-attributed candidate context edges for the latent-world walker.
//!
//! The materialiser deliberately owns only durable candidate context. It does
//! not fetch providers, infer legal authority, or pay a reader/proof claim.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

pub(crate) const CONTEXT_RECEIPT_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS context;
CREATE TABLE IF NOT EXISTS context.reviewed_relation_receipt (
  relation_ref TEXT NOT NULL REFERENCES algebra.relation(relation_ref),
  source_family_ref TEXT NOT NULL,
  source_revision_ref TEXT NOT NULL,
  source_content_digest TEXT,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claim_truth_promoted),
  receipt_sha256 BYTEA NOT NULL UNIQUE,
  PRIMARY KEY (relation_ref, source_revision_ref)
);
ALTER TABLE context.reviewed_relation_receipt
  ADD COLUMN IF NOT EXISTS source_content_digest TEXT
"#;

const MABO_WIKIDATA_QID: &str = "Q1501525";
const MABO_WIKIDATA_REVISION_REF: &str = "wikidata:Q1501525:oldid:2333409615";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceFamily {
    Wikidata,
    Wikipedia,
    Oalc,
}

impl SourceFamily {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wikidata => "wikidata",
            Self::Wikipedia => "wikipedia",
            Self::Oalc => "oalc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextReviewDecision {
    NotReviewed,
    Reviewed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedContextEdge {
    pub source_family: SourceFamily,
    pub source_revision_ref: String,
    pub source_content_digest: Option<String>,
    pub relation_ref: String,
    pub relation_type_ref: String,
    pub left_ref: String,
    pub right_ref: String,
    pub relation_sha256: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMaterializationReceipt {
    pub materialized_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum ContextFederationError {
    #[error("coordinate must not be empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("candidate has not been explicitly reviewed: {0}")]
    CandidateNotReviewed(String),
    #[error("unsupported Mabo Wikidata revision: {0}")]
    UnsupportedMaboWikidataRevision(String),
    #[error("unsupported Mabo Wikidata property: {0}")]
    UnsupportedMaboWikidataProperty(String),
    #[error("Mabo Wikidata candidate coordinate mismatch: {0}")]
    MaboWikidataCandidateMismatch(String),
    #[error("invalid sha256 hex: {0}")]
    InvalidHex(String),
    #[error("source revision must be immutable, not alias: {0}")]
    MutableSourceRevision(String),
    #[error("source receipt authority must remain experimental candidate-only: {0}")]
    InvalidReceiptAuthority(String),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
}

fn mabo_wikidata_relation_type(property_ref: &str) -> Option<&'static str> {
    match property_ref {
        "P1001" => Some("context:wikidata:jurisdiction"),
        "P710" => Some("context:wikidata:participant"),
        "P4884" => Some("context:wikidata:court"),
        "P1594" => Some("context:wikidata:judge"),
        "P4006" => Some("context:wikidata:overrules"),
        _ => None,
    }
}

fn mutable_source_revision_alias(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "latest" | "current" | "head" | "main" | "master"
    )
}

/// Convert one exact OALC legal-source receipt into durable candidate context
/// after an explicit review decision.
///
/// This is a federation/provenance edge only.  A governed legal provider
/// receipt does not itself establish legal authority, applicability, a
/// proposition payment, or claim truth.
pub fn review_mabo_oalc_exact_source(
    discovery_parent_ref: impl Into<String>,
    citation: impl Into<String>,
    source_identity_ref: impl Into<String>,
    source_revision_ref: impl Into<String>,
    canonical_text_digest: impl Into<String>,
    receipt_authority: impl Into<String>,
    review_decision: ContextReviewDecision,
) -> Result<ReviewedContextEdge, ContextFederationError> {
    let discovery_parent_ref = discovery_parent_ref.into();
    let citation = citation.into();
    let source_identity_ref = source_identity_ref.into();
    let source_revision_ref = source_revision_ref.into();
    let canonical_text_digest = canonical_text_digest.into();
    let receipt_authority = receipt_authority.into();

    if review_decision != ContextReviewDecision::Reviewed {
        return Err(ContextFederationError::CandidateNotReviewed(format!(
            "oalc:{source_revision_ref}"
        )));
    }
    for (name, value) in [
        ("discovery_parent_ref", discovery_parent_ref.as_str()),
        ("citation", citation.as_str()),
        ("source_identity_ref", source_identity_ref.as_str()),
        ("source_revision_ref", source_revision_ref.as_str()),
        ("canonical_text_digest", canonical_text_digest.as_str()),
        ("receipt_authority", receipt_authority.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ContextFederationError::EmptyCoordinate(name));
        }
    }
    if mutable_source_revision_alias(&source_revision_ref) {
        return Err(ContextFederationError::MutableSourceRevision(
            source_revision_ref,
        ));
    }
    if receipt_authority != "experimental_candidate_only" {
        return Err(ContextFederationError::InvalidReceiptAuthority(
            receipt_authority,
        ));
    }

    let mut edge = reviewed_context_edge(
        SourceFamily::Oalc,
        source_revision_ref,
        discovery_parent_ref,
        source_identity_ref,
        "context:oalc:exact-mnc",
    )?;
    edge.source_content_digest = Some(canonical_text_digest);
    Ok(edge)
}

/// Convert one exact Wikipedia article source receipt into durable candidate context
/// after an explicit review decision.
///
/// Wikipedia article context provides background/explanatory narrative only.
/// It does not create legal authority, applicability, proposition payment, or claim truth.
pub fn review_mabo_wikipedia_exact_source(
    discovery_parent_ref: impl Into<String>,
    canonical_url: impl Into<String>,
    source_identity_ref: impl Into<String>,
    source_revision_ref: impl Into<String>,
    canonical_text_digest: impl Into<String>,
    receipt_authority: impl Into<String>,
    review_decision: ContextReviewDecision,
) -> Result<ReviewedContextEdge, ContextFederationError> {
    let discovery_parent_ref = discovery_parent_ref.into();
    let canonical_url = canonical_url.into();
    let source_identity_ref = source_identity_ref.into();
    let source_revision_ref = source_revision_ref.into();
    let canonical_text_digest = canonical_text_digest.into();
    let receipt_authority = receipt_authority.into();

    if review_decision != ContextReviewDecision::Reviewed {
        return Err(ContextFederationError::CandidateNotReviewed(format!(
            "wikipedia:{source_revision_ref}"
        )));
    }
    for (name, value) in [
        ("discovery_parent_ref", discovery_parent_ref.as_str()),
        ("canonical_url", canonical_url.as_str()),
        ("source_identity_ref", source_identity_ref.as_str()),
        ("source_revision_ref", source_revision_ref.as_str()),
        ("canonical_text_digest", canonical_text_digest.as_str()),
        ("receipt_authority", receipt_authority.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ContextFederationError::EmptyCoordinate(name));
        }
    }
    if mutable_source_revision_alias(&source_revision_ref) {
        return Err(ContextFederationError::MutableSourceRevision(
            source_revision_ref,
        ));
    }
    if receipt_authority != "experimental_candidate_only" {
        return Err(ContextFederationError::InvalidReceiptAuthority(
            receipt_authority,
        ));
    }

    let mut edge = reviewed_context_edge(
        SourceFamily::Wikipedia,
        source_revision_ref,
        discovery_parent_ref,
        source_identity_ref,
        "context:wikipedia:article",
    )?;
    edge.source_content_digest = Some(canonical_text_digest);
    Ok(edge)
}

/// Convert one exact Mabo Wikidata property candidate into durable context only
/// after an explicit review decision.
///
/// This function deliberately accepts primitive candidate coordinates rather
/// than depending on the route-selector crate. Acquisition/SLRG decoding stays
/// upstream; PostgreSQL persistence owns only the reviewed boundary artifact.
/// `P4006` is therefore stored as an `overrules` context relation, but neither
/// its upstream `AuthoritySource` producer family nor this review creates legal
/// authority, applicability, proposition payment, or claim truth.
pub fn review_mabo_wikidata_candidate(
    source_revision_ref: impl Into<String>,
    candidate_id: impl Into<String>,
    source_ref: impl Into<String>,
    target_ref: impl Into<String>,
    property_ref: impl Into<String>,
    review_decision: ContextReviewDecision,
) -> Result<ReviewedContextEdge, ContextFederationError> {
    let source_revision_ref = source_revision_ref.into();
    let candidate_id = candidate_id.into();
    let source_ref = source_ref.into();
    let target_ref = target_ref.into();
    let property_ref = property_ref.into();

    if review_decision != ContextReviewDecision::Reviewed {
        return Err(ContextFederationError::CandidateNotReviewed(candidate_id));
    }
    if source_revision_ref != MABO_WIKIDATA_REVISION_REF {
        return Err(ContextFederationError::UnsupportedMaboWikidataRevision(
            source_revision_ref,
        ));
    }
    if source_ref != MABO_WIKIDATA_QID {
        return Err(ContextFederationError::MaboWikidataCandidateMismatch(
            format!("expected source {MABO_WIKIDATA_QID}, got {source_ref}"),
        ));
    }

    let relation_type_ref = mabo_wikidata_relation_type(&property_ref).ok_or_else(|| {
        ContextFederationError::UnsupportedMaboWikidataProperty(property_ref.clone())
    })?;
    let expected_candidate_id = format!("wikidata:{source_ref}:{property_ref}:{target_ref}");
    if candidate_id != expected_candidate_id {
        return Err(ContextFederationError::MaboWikidataCandidateMismatch(
            format!("expected candidate id {expected_candidate_id}, got {candidate_id}"),
        ));
    }

    reviewed_context_edge(
        SourceFamily::Wikidata,
        source_revision_ref,
        source_ref,
        target_ref,
        relation_type_ref,
    )
}

/// Build one reviewed, revision-pinned external context edge. Different source
/// revisions necessarily produce different durable relation identities.
pub fn reviewed_context_edge(
    source_family: SourceFamily,
    source_revision_ref: impl Into<String>,
    left_ref: impl Into<String>,
    right_ref: impl Into<String>,
    relation_type_ref: impl Into<String>,
) -> Result<ReviewedContextEdge, ContextFederationError> {
    let source_revision_ref = source_revision_ref.into();
    let left_ref = left_ref.into();
    let right_ref = right_ref.into();
    let relation_type_ref = relation_type_ref.into();

    for (name, value) in [
        ("source_revision_ref", source_revision_ref.as_str()),
        ("left_ref", left_ref.as_str()),
        ("right_ref", right_ref.as_str()),
        ("relation_type_ref", relation_type_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ContextFederationError::EmptyCoordinate(name));
        }
    }

    let mut hasher = Sha256::new();
    for value in [
        "reviewed-context-edge:v1",
        source_family.as_str(),
        source_revision_ref.as_str(),
        relation_type_ref.as_str(),
        left_ref.as_str(),
        right_ref.as_str(),
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let relation_sha256 = crate::hex(&digest);
    let relation_ref = format!(
        "relation:context:{}:{}",
        source_family.as_str(),
        &relation_sha256[..16]
    );

    Ok(ReviewedContextEdge {
        source_family,
        source_revision_ref,
        source_content_digest: None,
        relation_ref,
        relation_type_ref,
        left_ref,
        right_ref,
        relation_sha256,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

/// Persist reviewed context as `algebra.relation` plus an immutable provenance
/// receipt. The receipt makes source family/revision available to the walker
/// without converting context navigation into legal-IR proof or authority.
pub fn materialize_reviewed_context_edges(
    config: &DatabaseConfig,
    edges: &[ReviewedContextEdge],
) -> Result<ContextMaterializationReceipt, ContextFederationError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(CONTEXT_RECEIPT_SCHEMA_SQL)?;

    let mut materialized_count = 0;
    for edge in edges {
        let digest_bytes = crate::decode_hex_32(&edge.relation_sha256)
            .ok_or_else(|| ContextFederationError::InvalidHex(edge.relation_sha256.clone()))?;
        tx.execute(
            "INSERT INTO algebra.relation \
             (relation_ref, relation_type_ref, left_ref, right_ref, relation_sha256) \
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
            &[
                &edge.relation_ref,
                &edge.relation_type_ref,
                &edge.left_ref,
                &edge.right_ref,
                &&digest_bytes[..],
            ],
        )?;
        materialized_count += tx.execute(
            "INSERT INTO context.reviewed_relation_receipt \
             (relation_ref, source_family_ref, source_revision_ref, source_content_digest, candidate_only, \
              creates_semantic_authority, applicability_promoted, claim_truth_promoted, receipt_sha256) \
             VALUES ($1, $2, $3, $4, TRUE, FALSE, FALSE, FALSE, $5) \
             ON CONFLICT DO NOTHING",
            &[
                &edge.relation_ref,
                &edge.source_family.as_str(),
                &edge.source_revision_ref,
                &edge.source_content_digest,
                &&digest_bytes[..],
            ],
        )?;
    }
    tx.commit()?;

    Ok(ContextMaterializationReceipt {
        materialized_count: materialized_count as usize,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
