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

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use sensiblaw_pg_source_store::{
    bounded_wikidata_relation_type, LatentWorldEdgeRow, LatentWorldRows,
    ReviewedContextSourceRevisionCoordinate,
};
use sensiblaw_wikimedia_candidate_provider::fetch_latest_revision_id;

use crate::adaptive_campaign::ParsedBoundedContextCandidate;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboReviewedRevisionCoordinate {
    pub source_ref: String,
    pub reviewed_revision_id: u64,
    pub reviewed_revision_ref: String,
    pub coordinate_ref: String,
    pub coordinate_origin: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboRevisionReopenResidual {
    pub residual_ref: String,
    pub source_ref: String,
    pub reviewed_revision_id: u64,
    pub reviewed_revision_ref: String,
    pub latest_revision_id: u64,
    pub latest_revision_ref: String,
    pub triggering_coordinate_ref: String,
    pub triggering_coordinate_origin: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboRevisionProbeBlocker {
    pub source_ref: String,
    pub detail: String,
    pub retryable: bool,
    pub http_status_code: Option<u16>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboRevisionProbeReceipt {
    pub reviewed_source_count: usize,
    pub probed_source_count: usize,
    pub reopen_residuals: Vec<MaboRevisionReopenResidual>,
    pub unchanged_source_refs: Vec<String>,
    pub unprobed_source_refs: Vec<String>,
    pub blockers: Vec<MaboRevisionProbeBlocker>,
    pub probe_truncated: bool,
    pub probe_complete: bool,
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
    rows: &[ReviewedContextSourceRevisionCoordinate],
) -> Result<Vec<MaboReviewedRevisionCoordinate>, MaboRevisionCampaignError> {
    let mut highest: BTreeMap<String, MaboReviewedRevisionCoordinate> = BTreeMap::new();

    for row in rows {
        if !row.candidate_only
            || row.creates_semantic_authority
            || row.applicability_promoted
            || row.claim_truth_promoted
        {
            return Err(MaboRevisionCampaignError::ReviewedExpansionPromoted);
        }
        let revision_id = parse_revision_ref(&row.source_ref, &row.source_revision_ref)?;
        let coordinate = MaboReviewedRevisionCoordinate {
            source_ref: row.source_ref.clone(),
            reviewed_revision_id: revision_id,
            reviewed_revision_ref: row.source_revision_ref.clone(),
            coordinate_ref: row.coordinate_ref.clone(),
            coordinate_origin: row.coordinate_origin.to_owned(),
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
            triggering_coordinate_ref: coordinate.coordinate_ref.clone(),
            triggering_coordinate_origin: coordinate.coordinate_origin.clone(),
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
        unprobed_source_refs: Vec::new(),
        blockers: Vec::new(),
        probe_truncated: false,
        probe_complete: true,
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
    rows: &[ReviewedContextSourceRevisionCoordinate],
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
    let unprobed_source_refs = reviewed
        .iter()
        .skip(max_sources)
        .map(|coordinate| coordinate.source_ref.clone())
        .collect::<Vec<_>>();
    let mut latest = BTreeMap::new();
    let mut blockers = Vec::new();
    for coordinate in &bounded {
        match fetch_latest_revision_id(&coordinate.source_ref) {
            Ok(revision_id) => {
                latest.insert(coordinate.source_ref.clone(), revision_id);
            }
            Err(error) => {
                blockers.push(MaboRevisionProbeBlocker {
                    source_ref: coordinate.source_ref.clone(),
                    detail: error.to_string(),
                    retryable: error.network_is_retryable(),
                    http_status_code: error.network_status_code(),
                    candidate_only: true,
                    creates_semantic_authority: false,
                    claim_truth_promoted: false,
                });
            }
        }
    }

    // Compile only coordinates whose latest revision was actually observed.
    // Blocked sources remain explicit and therefore cannot be counted unchanged.
    let observed = bounded
        .iter()
        .filter(|coordinate| latest.contains_key(&coordinate.source_ref))
        .cloned()
        .collect::<Vec<_>>();
    let mut receipt = compile_mabo_revision_probe(&observed, &latest)?;
    receipt.reviewed_source_count = reviewed.len();
    receipt.probed_source_count = latest.len();
    receipt.unprobed_source_refs = unprobed_source_refs;
    receipt.blockers = blockers;
    receipt.probe_truncated =
        bounded.len() < receipt.reviewed_source_count;
    receipt.probe_complete =
        !receipt.probe_truncated && receipt.blockers.is_empty();
    Ok(receipt)
}

#[must_use]
pub fn select_next_mabo_revision_reopen(
    receipt: &MaboRevisionProbeReceipt,
) -> Option<&MaboRevisionReopenResidual> {
    receipt.reopen_residuals.first()
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowContextRevisionReceipt {
    pub source_ref: String,
    pub source_revision_ref: String,
    pub removed_current_context_edges: usize,
    pub added_shadow_context_edges: usize,
    pub base_edge_count: usize,
    pub shadow_edge_count: usize,
    pub base_visited_count: usize,
    pub shadow_visited_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

fn shadow_edge_from_candidate(
    source_ref: &str,
    source_revision_ref: &str,
    candidate: &ParsedBoundedContextCandidate,
) -> Result<LatentWorldEdgeRow, MaboRevisionCampaignError> {
    if candidate.source_qid != source_ref || candidate.source_revision_ref != source_revision_ref {
        return Err(MaboRevisionCampaignError::InvalidReviewedRevision(format!(
            "shadow candidate coordinate mismatch: source={} revision={}",
            candidate.source_qid, candidate.source_revision_ref
        )));
    }
    let relation_type_ref = bounded_wikidata_relation_type(&candidate.property_ref)
        .ok_or_else(|| {
            MaboRevisionCampaignError::InvalidReviewedRevision(format!(
                "shadow candidate uses unsupported bounded property {}",
                candidate.property_ref
            ))
        })?;
    Ok(LatentWorldEdgeRow {
        from_ref: source_ref.to_owned(),
        to_ref: candidate.target_qid.clone(),
        relation_ref: relation_type_ref.to_owned(),
        provenance_refs: vec![format!("shadow-context:{source_revision_ref}")],
    })
}

fn recompute_shadow_reachability(
    seed_ref: &str,
    max_hops: u32,
    edges: &[LatentWorldEdgeRow],
) -> (Vec<String>, u32, Vec<String>, bool) {
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edge in edges {
        adjacency
            .entry(edge.from_ref.clone())
            .or_default()
            .insert(edge.to_ref.clone());
        adjacency
            .entry(edge.to_ref.clone())
            .or_default()
            .insert(edge.from_ref.clone());
    }

    let mut depth = BTreeMap::from([(seed_ref.to_owned(), 0_u32)]);
    let mut queue = VecDeque::from([seed_ref.to_owned()]);
    let mut retained_frontier = BTreeSet::new();

    while let Some(reference) = queue.pop_front() {
        let current_depth = depth[&reference];
        let Some(neighbours) = adjacency.get(&reference) else {
            continue;
        };
        if current_depth >= max_hops {
            for neighbour in neighbours {
                if !depth.contains_key(neighbour) {
                    retained_frontier.insert(neighbour.clone());
                }
            }
            continue;
        }
        for neighbour in neighbours {
            if depth.contains_key(neighbour) {
                continue;
            }
            depth.insert(neighbour.clone(), current_depth + 1);
            queue.push_back(neighbour.clone());
        }
    }

    let deepest = depth.values().copied().max().unwrap_or(0);
    let mut visited = depth.keys().cloned().collect::<Vec<_>>();
    visited.sort_by(|left, right| {
        depth[left]
            .cmp(&depth[right])
            .then_with(|| left.cmp(right))
    });
    (
        visited,
        deepest,
        retained_frontier.iter().cloned().collect(),
        retained_frontier.is_empty(),
    )
}

/// Construct a non-persisted historical/context counterfactual over an already
/// loaded revision-sliced world.
///
/// The fixture replaces only one source's bounded Wikidata context edges, then
/// recomputes reachability over the same finite edge universe.  It cannot
/// create reviewed evidence, authority, applicability or claim truth and must
/// never be persisted as if it were an operator review.
pub fn shadow_context_revision_world(
    base: &LatentWorldRows,
    source_ref: &str,
    source_revision_ref: &str,
    candidates: &[ParsedBoundedContextCandidate],
) -> Result<(LatentWorldRows, ShadowContextRevisionReceipt), MaboRevisionCampaignError> {
    if !base.creates_semantic_authority && !base.applicability_promoted && !base.claim_truth_promoted
    {
        // expected non-promoting base
    } else {
        return Err(MaboRevisionCampaignError::ReviewedExpansionPromoted);
    }
    parse_revision_ref(source_ref, source_revision_ref)?;

    let mut removed = 0usize;
    let mut edge_map: BTreeMap<
        (String, String, String),
        BTreeSet<String>,
    > = BTreeMap::new();

    for edge in &base.edges {
        if edge.from_ref == source_ref && edge.relation_ref.starts_with("context:wikidata:") {
            removed += 1;
            continue;
        }
        edge_map
            .entry((
                edge.from_ref.clone(),
                edge.to_ref.clone(),
                edge.relation_ref.clone(),
            ))
            .or_default()
            .extend(edge.provenance_refs.iter().cloned());
    }

    let mut added = 0usize;
    let mut canonical = candidates.to_vec();
    canonical.sort();
    canonical.dedup();
    for candidate in &canonical {
        let edge = shadow_edge_from_candidate(source_ref, source_revision_ref, candidate)?;
        let key = (
            edge.from_ref.clone(),
            edge.to_ref.clone(),
            edge.relation_ref.clone(),
        );
        let entry = edge_map.entry(key).or_default();
        let before = entry.len();
        entry.extend(edge.provenance_refs);
        if entry.len() > before {
            added += 1;
        }
    }

    let edges = edge_map
        .into_iter()
        .map(
            |((from_ref, to_ref, relation_ref), provenance_refs)| LatentWorldEdgeRow {
                from_ref,
                to_ref,
                relation_ref,
                provenance_refs: provenance_refs.into_iter().collect(),
            },
        )
        .collect::<Vec<_>>();

    let (visited_refs, deepest_observed_hop, frontier_refs, frontier_exhausted) =
        recompute_shadow_reachability(
            &base.seed_ref,
            base.requested_max_hops,
            &edges,
        );
    let residual_refs = if frontier_exhausted {
        Vec::new()
    } else {
        vec!["world-residual:hop-limit".into()]
    };

    let shadow = LatentWorldRows {
        seed_ref: base.seed_ref.clone(),
        max_hops: base.max_hops,
        requested_max_hops: base.requested_max_hops,
        visited_refs,
        deepest_observed_hop,
        frontier_exhausted,
        frontier_refs,
        residual_refs,
        edges,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };

    let receipt = ShadowContextRevisionReceipt {
        source_ref: source_ref.to_owned(),
        source_revision_ref: source_revision_ref.to_owned(),
        removed_current_context_edges: removed,
        added_shadow_context_edges: added,
        base_edge_count: base.edges.len(),
        shadow_edge_count: shadow.edges.len(),
        base_visited_count: base.visited_refs.len(),
        shadow_visited_count: shadow.visited_refs.len(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    Ok((shadow, receipt))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(source: &str, revision: u64, review: &str) -> ReviewedContextSourceRevisionCoordinate {
        ReviewedContextSourceRevisionCoordinate {
            source_ref: source.into(),
            source_revision_ref: format!("wikidata:{source}:oldid:{revision}"),
            coordinate_ref: format!("coord:{source}:{revision}:{review}"),
            coordinate_origin: "source_expansion_receipt",
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
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
        assert!(receipt.probe_complete);
        assert!(receipt.blockers.is_empty());
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

    #[test]
    fn probe_receipt_can_explicitly_report_unprobed_sources() {
        let reviewed = highest_reviewed_mabo_revisions(&[
            row("Q1", 100, "r1"),
            row("Q2", 200, "r2"),
        ])
        .unwrap();
        let latest = BTreeMap::from([("Q1".into(), 100_u64)]);
        let receipt = compile_mabo_revision_probe(&reviewed[..1], &latest).unwrap();
        assert_eq!(receipt.reviewed_source_count, 1);
        assert!(!receipt.probe_truncated);
        assert!(receipt.probe_complete);
    }


    #[test]
    fn shadow_revision_replaces_source_context_and_recomputes_reachability() {
        let base = LatentWorldRows {
            seed_ref: "QROOT".into(),
            max_hops: 10,
            requested_max_hops: 10,
            visited_refs: vec!["QROOT".into(), "Q1".into(), "QOLD".into()],
            deepest_observed_hop: 2,
            frontier_exhausted: true,
            frontier_refs: vec![],
            residual_refs: vec![],
            edges: vec![
                LatentWorldEdgeRow {
                    from_ref: "QROOT".into(),
                    to_ref: "Q1".into(),
                    relation_ref: "context:wikidata:participant".into(),
                    provenance_refs: vec!["persisted:root".into()],
                },
                LatentWorldEdgeRow {
                    from_ref: "Q1".into(),
                    to_ref: "QOLD".into(),
                    relation_ref: "context:wikidata:court".into(),
                    provenance_refs: vec!["context:wikidata:Q1:oldid:100".into()],
                },
            ],
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        let candidates = vec![ParsedBoundedContextCandidate {
            candidate_id: "wikidata:Q1:P4884:QNEW".into(),
            source_qid: "Q1".into(),
            target_qid: "QNEW".into(),
            property_ref: "P4884".into(),
            source_revision_ref: "wikidata:Q1:oldid:90".into(),
        }];
        let (shadow, receipt) = shadow_context_revision_world(
            &base,
            "Q1",
            "wikidata:Q1:oldid:90",
            &candidates,
        )
        .unwrap();
        assert!(shadow.visited_refs.contains(&"QNEW".into()));
        assert!(!shadow.visited_refs.contains(&"QOLD".into()));
        assert_eq!(receipt.removed_current_context_edges, 1);
        assert_eq!(receipt.added_shadow_context_edges, 1);
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.claim_truth_promoted);
    }

}