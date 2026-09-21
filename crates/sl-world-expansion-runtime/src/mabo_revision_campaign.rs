//! Revision-aware Mabo context reopening.
//!
//! A reviewed source expansion is pinned to one exact Wikidata revision.  A QID
//! having *some* historical reviewed expansion is therefore not enough to close
//! future work.  This module compares the highest reviewed revision per source
//! against a bounded latest-revision lookup and emits candidate-only context
//! reopening residuals when the source has advanced.
//!
//! A revision change reopens source/context observation only.  It does not
//! directly invalidate or fabricate SameObject/identity conclusions.

use std::collections::BTreeMap;

use sensiblaw_pg_source_store::ReviewedSourceExpansionRow;
use sensiblaw_wikimedia_candidate_provider::fetch_latest_revision_id;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboReviewedRevisionCoordinate {
    pub source_ref: String,
    pub reviewed_revision_id: u64,
    pub reviewed_revision_ref: String,
    pub review_ref: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboRevisionReopenResidual {
    pub residual_ref: String,
    pub source_ref: String,
    pub reviewed_revision_id: u64,
    pub reviewed_revision_ref: String,
    pub latest_revision_id: u64,
    pub latest_revision_ref: String,
    pub triggering_review_ref: String,
    pub triggering_receipt_sha256: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboRevisionProbeReceipt {
    pub reviewed_source_count: usize,
    pub probed_source_count: usize,
    pub reopen_residuals: Vec<MaboRevisionReopenResidual>,
    pub unchanged_source_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MaboRevisionCampaignError {
    #[error("invalid reviewed Wikidata revision ref: {0}")]
    InvalidReviewedRevision(String),
    #[error(
        "latest revision {latest_revision_id} for {source_ref} precedes highest reviewed revision {reviewed_revision_id}"
    )]
    LatestRevisionRegressed {
        source_ref: String,
        reviewed_revision_id: u64,
        latest_revision_id: u64,
    },
    #[error("latest revision missing for reviewed source {0}")]
    MissingLatestRevision(String),
    #[error("latest revision lookup failed for {source_ref}: {detail}")]
    LatestRevisionLookup { source_ref: String, detail: String },
    #[error("revision probe max_sources must be positive")]
    InvalidProbeBudget,
    #[error("reviewed source expansion crossed non-promotion boundary")]
    ReviewedExpansionPromoted,
}

fn parse_revision_ref(
    source_ref: &str,
    source_revision_ref: &str,
) -> Result<u64, MaboRevisionCampaignError> {
    let fields = source_revision_ref.split(':').collect::<Vec<_>>();
    let valid = fields.len() == 4
        && fields[0] == "wikidata"
        && fields[1] == source_ref
        && fields[2] == "oldid";
    if !valid {
        return Err(MaboRevisionCampaignError::InvalidReviewedRevision(
            source_revision_ref.to_owned(),
        ));
    }
    fields[3]
        .parse::<u64>()
        .ok()
        .filter(|revision| *revision > 0)
        .ok_or_else(|| {
            MaboRevisionCampaignError::InvalidReviewedRevision(
                source_revision_ref.to_owned(),
            )
        })
}

pub fn highest_reviewed_mabo_revisions(
    rows: &[ReviewedSourceExpansionRow],
) -> Result<Vec<MaboReviewedRevisionCoordinate>, MaboRevisionCampaignError> {
    let mut highest: BTreeMap<String, MaboReviewedRevisionCoordinate> = BTreeMap::new();

    for row in rows {
        if !row.candidate_only
            || row.creates_semantic_authority
            || row.applicability_promoted
            || row.claim_truth_promoted
            || row.counts_as_novel_identity
            || row.pays_claim_residual
        {
            return Err(MaboRevisionCampaignError::ReviewedExpansionPromoted);
        }
        let revision_id = parse_revision_ref(&row.source_ref, &row.source_revision_ref)?;
        let coordinate = MaboReviewedRevisionCoordinate {
            source_ref: row.source_ref.clone(),
            reviewed_revision_id: revision_id,
            reviewed_revision_ref: row.source_revision_ref.clone(),
            review_ref: row.review_ref.clone(),
            receipt_sha256: row.receipt_sha256.clone(),
        };

        match highest.get(&row.source_ref) {
            Some(existing) if existing.reviewed_revision_id >= revision_id => {}
            _ => {
                highest.insert(row.source_ref.clone(), coordinate);
            }
        }
    }

    Ok(highest.into_values().collect())
}

pub fn compile_mabo_revision_probe(
    reviewed: &[MaboReviewedRevisionCoordinate],
    latest_revision_ids: &BTreeMap<String, u64>,
) -> Result<MaboRevisionProbeReceipt, MaboRevisionCampaignError> {
    let mut reopen_residuals = Vec::new();
    let mut unchanged_source_refs = Vec::new();

    for coordinate in reviewed {
        let latest_revision_id = *latest_revision_ids
            .get(&coordinate.source_ref)
            .ok_or_else(|| {
                MaboRevisionCampaignError::MissingLatestRevision(
                    coordinate.source_ref.clone(),
                )
            })?;
        if latest_revision_id < coordinate.reviewed_revision_id {
            return Err(MaboRevisionCampaignError::LatestRevisionRegressed {
                source_ref: coordinate.source_ref.clone(),
                reviewed_revision_id: coordinate.reviewed_revision_id,
                latest_revision_id,
            });
        }
        if latest_revision_id == coordinate.reviewed_revision_id {
            unchanged_source_refs.push(coordinate.source_ref.clone());
            continue;
        }

        let latest_revision_ref =
            format!("wikidata:{}:oldid:{latest_revision_id}", coordinate.source_ref);
        reopen_residuals.push(MaboRevisionReopenResidual {
            residual_ref: format!(
                "residual:mabo:source-revision:{}:{}->{}",
                coordinate.source_ref,
                coordinate.reviewed_revision_id,
                latest_revision_id
            ),
            source_ref: coordinate.source_ref.clone(),
            reviewed_revision_id: coordinate.reviewed_revision_id,
            reviewed_revision_ref: coordinate.reviewed_revision_ref.clone(),
            latest_revision_id,
            latest_revision_ref,
            triggering_review_ref: coordinate.review_ref.clone(),
            triggering_receipt_sha256: coordinate.receipt_sha256.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        });
    }

    reopen_residuals.sort_by(|left, right| {
        left.source_ref
            .cmp(&right.source_ref)
            .then_with(|| left.latest_revision_id.cmp(&right.latest_revision_id))
    });
    unchanged_source_refs.sort();
    unchanged_source_refs.dedup();

    Ok(MaboRevisionProbeReceipt {
        reviewed_source_count: reviewed.len(),
        probed_source_count: latest_revision_ids.len(),
        reopen_residuals,
        unchanged_source_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

/// Bounded live latest-revision probe.
///
/// Only the highest reviewed coordinate per source is probed.  This call does
/// not acquire RDF content and cannot by itself pay the reopened residual.
pub fn probe_latest_mabo_revision_changes(
    rows: &[ReviewedSourceExpansionRow],
    max_sources: usize,
) -> Result<MaboRevisionProbeReceipt, MaboRevisionCampaignError> {
    if max_sources == 0 {
        return Err(MaboRevisionCampaignError::InvalidProbeBudget);
    }
    let reviewed = highest_reviewed_mabo_revisions(rows)?;
    let bounded = reviewed
        .iter()
        .take(max_sources)
        .cloned()
        .collect::<Vec<_>>();
    let mut latest = BTreeMap::new();
    for coordinate in &bounded {
        let revision_id = fetch_latest_revision_id(&coordinate.source_ref).map_err(|error| {
            MaboRevisionCampaignError::LatestRevisionLookup {
                source_ref: coordinate.source_ref.clone(),
                detail: error.to_string(),
            }
        })?;
        latest.insert(coordinate.source_ref.clone(), revision_id);
    }
    compile_mabo_revision_probe(&bounded, &latest)
}

#[must_use]
pub fn select_next_mabo_revision_reopen(
    receipt: &MaboRevisionProbeReceipt,
) -> Option<&MaboRevisionReopenResidual> {
    receipt.reopen_residuals.first()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(source: &str, revision: u64, review: &str) -> ReviewedSourceExpansionRow {
        ReviewedSourceExpansionRow {
            source_ref: source.into(),
            source_revision_ref: format!("wikidata:{source}:oldid:{revision}"),
            review_ref: review.into(),
            bounded_candidate_count: 2,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            counts_as_novel_identity: false,
            pays_claim_residual: false,
            receipt_authority: "reviewed_bounded_context_expansion_only",
            receipt_sha256: format!("sha256:{source}:{revision}"),
        }
    }

    #[test]
    fn only_highest_reviewed_revision_per_source_is_current_coordinate() {
        let reviewed = highest_reviewed_mabo_revisions(&[
            row("Q1", 100, "review:old"),
            row("Q1", 120, "review:new"),
            row("Q2", 50, "review:q2"),
        ])
        .unwrap();
        assert_eq!(reviewed.len(), 2);
        assert_eq!(reviewed[0].source_ref, "Q1");
        assert_eq!(reviewed[0].reviewed_revision_id, 120);
        assert_eq!(reviewed[1].source_ref, "Q2");
        assert_eq!(reviewed[1].reviewed_revision_id, 50);
    }

    #[test]
    fn newer_latest_revision_reopens_context_not_identity_truth() {
        let reviewed = highest_reviewed_mabo_revisions(&[
            row("Q1", 120, "review:q1"),
            row("Q2", 50, "review:q2"),
        ])
        .unwrap();
        let latest = BTreeMap::from([("Q1".into(), 121_u64), ("Q2".into(), 50_u64)]);
        let receipt = compile_mabo_revision_probe(&reviewed, &latest).unwrap();
        assert_eq!(receipt.reopen_residuals.len(), 1);
        let reopened = &receipt.reopen_residuals[0];
        assert_eq!(reopened.source_ref, "Q1");
        assert_eq!(reopened.reviewed_revision_id, 120);
        assert_eq!(reopened.latest_revision_id, 121);
        assert_eq!(
            reopened.latest_revision_ref,
            "wikidata:Q1:oldid:121"
        );
        assert_eq!(receipt.unchanged_source_refs, vec!["Q2"]);
        assert!(!reopened.creates_semantic_authority);
        assert!(!reopened.applicability_promoted);
        assert!(!reopened.claim_truth_promoted);
    }

    #[test]
    fn latest_revision_regression_fails_closed() {
        let reviewed = highest_reviewed_mabo_revisions(&[row("Q1", 120, "review:q1")])
            .unwrap();
        let latest = BTreeMap::from([("Q1".into(), 119_u64)]);
        assert!(matches!(
            compile_mabo_revision_probe(&reviewed, &latest),
            Err(MaboRevisionCampaignError::LatestRevisionRegressed { .. })
        ));
    }
}
