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
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claim_truth_promoted),
  receipt_sha256 BYTEA NOT NULL UNIQUE,
  PRIMARY KEY (relation_ref, source_revision_ref)
)
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
    #[error("Wikidata candidate coordinate mismatch: {0}")]
    WikidataCandidateMismatch(String),
    #[error("Mabo Wikidata candidate coordinate mismatch: {0}")]
    MaboWikidataCandidateMismatch(String),
    #[error("invalid sha256 hex: {0}")]
    InvalidHex(String),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
}

/// Finite provider-property -> durable context-role mapping used by the Mabo
/// world-expansion surface. This is intentionally not a general Wikidata
/// semantic inference rule.
#[must_use]
pub fn bounded_wikidata_relation_type(property_ref: &str) -> Option<&'static str> {
    match property_ref {
        "P1001" => Some("context:wikidata:jurisdiction"),
        "P710" => Some("context:wikidata:participant"),
        "P4884" => Some("context:wikidata:court"),
        "P1594" => Some("context:wikidata:judge"),
        "P4006" => Some("context:wikidata:overrules"),
        _ => None,
    }
}

/// Exact inverse of the bounded reviewed Wikidata relation mapping above.
///
/// This is provider-coordinate recovery for relations created by the bounded
/// context producer; it is not a general semantic-label -> Wikidata-property
/// inference rule. Unknown labels fail closed.
#[must_use]
pub fn mabo_wikidata_property_ref(relation_type_ref: &str) -> Option<&'static str> {
    match relation_type_ref {
        "context:wikidata:jurisdiction" => Some("P1001"),
        "context:wikidata:participant" => Some("P710"),
        "context:wikidata:court" => Some("P4884"),
        "context:wikidata:judge" => Some("P1594"),
        "context:wikidata:overrules" => Some("P4006"),
        _ => None,
    }
}

fn exact_wikidata_revision_matches_source(source_revision_ref: &str, source_ref: &str) -> bool {
    let fields = source_revision_ref.split(':').collect::<Vec<_>>();
    fields.len() == 4
        && fields[0] == "wikidata"
        && fields[1] == source_ref
        && fields[2] == "oldid"
        && fields[3]
            .parse::<u64>()
            .is_ok_and(|revision_id| revision_id > 0)
}

/// Convert one exact bounded Wikidata property candidate into durable context
/// only after an explicit relation/context review decision.
///
/// Unlike `review_mabo_wikidata_candidate`, this accepts any QID provided the
/// source manifestation is pinned as `wikidata:<same-QID>:oldid:<positive>`.
/// The finite property mapping remains exactly the Mabo bounded context surface.
/// Identity review is not consulted here and cannot substitute for this review.
pub fn review_bounded_wikidata_candidate(
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
    if !exact_wikidata_revision_matches_source(&source_revision_ref, &source_ref) {
        return Err(ContextFederationError::WikidataCandidateMismatch(format!(
            "source revision {source_revision_ref} does not pin source {source_ref}"
        )));
    }

    let relation_type_ref = bounded_wikidata_relation_type(&property_ref).ok_or_else(|| {
        ContextFederationError::UnsupportedMaboWikidataProperty(property_ref.clone())
    })?;
    let expected_candidate_id = format!("wikidata:{source_ref}:{property_ref}:{target_ref}");
    if candidate_id != expected_candidate_id {
        return Err(ContextFederationError::WikidataCandidateMismatch(format!(
            "expected candidate id {expected_candidate_id}, got {candidate_id}"
        )));
    }

    reviewed_context_edge(
        SourceFamily::Wikidata,
        source_revision_ref,
        source_ref,
        target_ref,
        relation_type_ref,
    )
}

/// Convert one exact Mabo Wikidata property candidate into durable context only
/// after an explicit review decision.
///
/// This compatibility wrapper retains the stronger original contract: the
/// source must be Q1501525 at the exact reviewed Mabo revision. The generalized
/// bounded reviewer above is used only after these Mabo-specific checks pass.
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

    review_bounded_wikidata_candidate(
        source_revision_ref,
        candidate_id,
        source_ref,
        target_ref,
        property_ref,
        review_decision,
    )
    .map_err(|error| match error {
        ContextFederationError::WikidataCandidateMismatch(detail) => {
            ContextFederationError::MaboWikidataCandidateMismatch(detail)
        }
        other => other,
    })
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
             (relation_ref, source_family_ref, source_revision_ref, candidate_only, \
              creates_semantic_authority, applicability_promoted, claim_truth_promoted, receipt_sha256) \
             VALUES ($1, $2, $3, TRUE, FALSE, FALSE, FALSE, $4) \
             ON CONFLICT DO NOTHING",
            &[
                &edge.relation_ref,
                &edge.source_family.as_str(),
                &edge.source_revision_ref,
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
