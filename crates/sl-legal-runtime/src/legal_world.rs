//! First-class legal world coordinates, authority validity and revision impact.
//!
//! S18 makes time, jurisdiction and source revision part of the semantic world
//! rather than incidental CLI strings.  Revision invalidation is dependency
//! propagation only: a changed source marks dependent evidence/propositions
//! stale; it does not decide the new proposition truth.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{ConsumerAxis, ConsumerCoverage};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalWorldCoordinate {
    pub world_ref: String,
    pub matter_ref: String,
    pub jurisdiction_ref: String,
    /// ISO-8601 calendar date (YYYY-MM-DD).
    pub as_at: String,
    /// semantic/source identity -> exact pinned revision
    pub source_revisions: BTreeMap<String, String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn valid_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| {
                index == 4
                    || index == 7
                    || byte.is_ascii_digit()
            })
}

impl LegalWorldCoordinate {
    pub fn validate(&self) -> Result<(), String> {
        if self.world_ref.trim().is_empty()
            || self.matter_ref.trim().is_empty()
            || self.jurisdiction_ref.trim().is_empty()
            || !valid_iso_date(&self.as_at)
        {
            return Err("legal world has invalid identity/jurisdiction/as-at coordinate".into());
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("legal world crossed non-promotion boundary".into());
        }
        if self
            .source_revisions
            .iter()
            .any(|(source, revision)| source.trim().is_empty() || revision.trim().is_empty())
        {
            return Err("legal world contains empty source/revision coordinate".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityValidity {
    pub authority_ref: String,
    pub jurisdiction_ref: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub supersedes: BTreeSet<String>,
    pub displaced_by: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

impl AuthorityValidity {
    pub fn validate(&self) -> Result<(), String> {
        if self.authority_ref.trim().is_empty() || self.jurisdiction_ref.trim().is_empty() {
            return Err("authority validity requires authority and jurisdiction refs".into());
        }
        for date in self.valid_from.iter().chain(self.valid_to.iter()) {
            if !valid_iso_date(date) {
                return Err(format!("authority validity has invalid date {date}"));
            }
        }
        if let (Some(from), Some(to)) = (&self.valid_from, &self.valid_to) {
            if from > to {
                return Err("authority validity interval is inverted".into());
            }
        }
        if !self.candidate_only || self.creates_legal_authority {
            return Err("authority validity crossed non-promotion boundary".into());
        }
        Ok(())
    }

    #[must_use]
    pub fn active_at(&self, world: &LegalWorldCoordinate) -> bool {
        let jurisdiction_matches = self.jurisdiction_ref == "AU"
            || world.jurisdiction_ref == "AU"
            || self.jurisdiction_ref == world.jurisdiction_ref;
        let after_start = self
            .valid_from
            .as_ref()
            .map_or(true, |from| &world.as_at >= from);
        let before_end = self
            .valid_to
            .as_ref()
            .map_or(true, |to| &world.as_at <= to);
        jurisdiction_matches && after_start && before_end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalWorldSlice {
    pub coordinate: LegalWorldCoordinate,
    pub active_authority_refs: BTreeSet<String>,
    pub inactive_authority_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn temporal_world_slice(
    coordinate: LegalWorldCoordinate,
    authorities: &[AuthorityValidity],
) -> Result<TemporalWorldSlice, String> {
    coordinate.validate()?;
    let mut active = BTreeSet::new();
    let mut inactive = BTreeSet::new();
    for authority in authorities {
        authority.validate()?;
        if authority.active_at(&coordinate) {
            active.insert(authority.authority_ref.clone());
        } else {
            inactive.insert(authority.authority_ref.clone());
        }
    }
    Ok(TemporalWorldSlice {
        coordinate,
        active_authority_refs: active,
        inactive_authority_refs: inactive,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryWorldScope {
    pub query_ref: String,
    pub required_jurisdiction_ref: String,
    pub required_as_at: String,
}

pub fn world_scope_coverage(
    world: &LegalWorldCoordinate,
    scope: &QueryWorldScope,
) -> Result<ConsumerCoverage, String> {
    world.validate()?;
    if scope.query_ref.trim().is_empty()
        || scope.required_jurisdiction_ref.trim().is_empty()
        || !valid_iso_date(&scope.required_as_at)
    {
        return Err("query world scope is invalid".into());
    }

    let mut coverage = ConsumerCoverage::default();
    if world.jurisdiction_ref == scope.required_jurisdiction_ref {
        coverage.paid_axes.insert(ConsumerAxis::Jurisdiction);
    }
    if world.as_at == scope.required_as_at {
        coverage.paid_axes.insert(ConsumerAxis::Temporal);
    }
    Ok(coverage)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRevisionChange {
    pub source_ref: String,
    pub old_revision_ref: Option<String>,
    pub new_revision_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionInvalidationReceipt {
    pub from_world_ref: String,
    pub to_world_ref: String,
    pub changes: Vec<SourceRevisionChange>,
    pub unchanged_source_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn diff_world_revisions(
    old: &LegalWorldCoordinate,
    new: &LegalWorldCoordinate,
) -> Result<RevisionInvalidationReceipt, String> {
    old.validate()?;
    new.validate()?;
    if old.matter_ref != new.matter_ref {
        return Err("revision diff requires the same matter_ref".into());
    }

    let all_sources = old
        .source_revisions
        .keys()
        .chain(new.source_revisions.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();
    let mut unchanged = BTreeSet::new();

    for source_ref in all_sources {
        let old_revision = old.source_revisions.get(&source_ref).cloned();
        let new_revision = new.source_revisions.get(&source_ref).cloned();
        if old_revision == new_revision {
            unchanged.insert(source_ref);
        } else {
            changes.push(SourceRevisionChange {
                source_ref,
                old_revision_ref: old_revision,
                new_revision_ref: new_revision,
            });
        }
    }

    Ok(RevisionInvalidationReceipt {
        from_world_ref: old.world_ref.clone(),
        to_world_ref: new.world_ref.clone(),
        changes,
        unchanged_source_refs: unchanged,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RevisionDependencyIndex {
    /// source identity -> proposition refs that directly depend on it
    pub source_to_propositions: BTreeMap<String, BTreeSet<String>>,
    /// proposition ref -> downstream proposition/proof refs
    pub proposition_dependents: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedProofCone {
    pub changed_source_refs: BTreeSet<String>,
    pub directly_affected_proposition_refs: BTreeSet<String>,
    pub transitively_affected_refs: BTreeSet<String>,
    pub stale_evidence_refs: BTreeSet<String>,
    pub re_review_required: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn affected_proof_cone(
    invalidation: &RevisionInvalidationReceipt,
    dependencies: &RevisionDependencyIndex,
) -> Result<AffectedProofCone, String> {
    if !invalidation.candidate_only
        || invalidation.creates_semantic_authority
        || invalidation.creates_claim_truth
    {
        return Err("revision invalidation crossed non-promotion boundary".into());
    }

    let changed_source_refs = invalidation
        .changes
        .iter()
        .map(|change| change.source_ref.clone())
        .collect::<BTreeSet<_>>();
    let directly_affected = changed_source_refs
        .iter()
        .flat_map(|source| {
            dependencies
                .source_to_propositions
                .get(source)
                .into_iter()
                .flat_map(|refs| refs.iter().cloned())
        })
        .collect::<BTreeSet<_>>();

    let mut affected = directly_affected.clone();
    let mut queue = VecDeque::from_iter(directly_affected.iter().cloned());
    while let Some(reference) = queue.pop_front() {
        if let Some(dependents) = dependencies.proposition_dependents.get(&reference) {
            for dependent in dependents {
                if affected.insert(dependent.clone()) {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }

    let stale_evidence_refs = directly_affected
        .iter()
        .map(|reference| format!("evidence-stale:{reference}"))
        .collect::<BTreeSet<_>>();

    Ok(AffectedProofCone {
        changed_source_refs,
        directly_affected_proposition_refs: directly_affected,
        transitively_affected_refs: affected,
        stale_evidence_refs,
        re_review_required: !invalidation.changes.is_empty(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(world_ref: &str, jurisdiction: &str, as_at: &str, revision: &str) -> LegalWorldCoordinate {
        LegalWorldCoordinate {
            world_ref: world_ref.into(),
            matter_ref: "matter:fixture".into(),
            jurisdiction_ref: jurisdiction.into(),
            as_at: as_at.into(),
            source_revisions: BTreeMap::from([("source:case".into(), revision.into())]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    #[test]
    fn authority_activity_is_world_time_and_jurisdiction_relative() {
        let authority = AuthorityValidity {
            authority_ref: "case:fixture".into(),
            jurisdiction_ref: "AU-QLD".into(),
            valid_from: Some("2020-01-01".into()),
            valid_to: Some("2025-12-31".into()),
            supersedes: BTreeSet::new(),
            displaced_by: BTreeSet::from(["case:successor".into()]),
            candidate_only: true,
            creates_legal_authority: false,
        };
        assert!(authority.active_at(&world("world:2024", "AU-QLD", "2024-01-01", "rev:1")));
        assert!(!authority.active_at(&world("world:2026", "AU-QLD", "2026-01-01", "rev:1")));
        assert!(!authority.active_at(&world("world:nsw", "AU-NSW", "2024-01-01", "rev:1")));
    }

    #[test]
    fn same_query_scope_does_not_get_jurisdiction_axis_from_wrong_world() {
        let scope = QueryWorldScope {
            query_ref: "query:fixture".into(),
            required_jurisdiction_ref: "AU-QLD".into(),
            required_as_at: "2026-09-21".into(),
        };
        let qld = world("world:qld", "AU-QLD", "2026-09-21", "rev:1");
        let nsw = world("world:nsw", "AU-NSW", "2026-09-21", "rev:1");
        let qld_coverage = world_scope_coverage(&qld, &scope).unwrap();
        let nsw_coverage = world_scope_coverage(&nsw, &scope).unwrap();
        assert!(qld_coverage.paid_axes.contains(&ConsumerAxis::Jurisdiction));
        assert!(qld_coverage.paid_axes.contains(&ConsumerAxis::Temporal));
        assert!(!nsw_coverage.paid_axes.contains(&ConsumerAxis::Jurisdiction));
        assert!(nsw_coverage.paid_axes.contains(&ConsumerAxis::Temporal));
    }

    #[test]
    fn revision_change_reopens_direct_and_transitive_proof_cone() {
        let old = world("world:old", "AU", "2026-09-20", "rev:1");
        let new = world("world:new", "AU", "2026-09-21", "rev:2");
        let invalidation = diff_world_revisions(&old, &new).unwrap();
        assert_eq!(invalidation.changes.len(), 1);

        let dependencies = RevisionDependencyIndex {
            source_to_propositions: BTreeMap::from([(
                "source:case".into(),
                BTreeSet::from(["prop:a".into()]),
            )]),
            proposition_dependents: BTreeMap::from([
                ("prop:a".into(), BTreeSet::from(["prop:b".into()])),
                ("prop:b".into(), BTreeSet::from(["proof:query".into()])),
            ]),
        };
        let cone = affected_proof_cone(&invalidation, &dependencies).unwrap();
        assert_eq!(
            cone.directly_affected_proposition_refs,
            BTreeSet::from(["prop:a".into()])
        );
        assert_eq!(
            cone.transitively_affected_refs,
            BTreeSet::from(["prop:a".into(), "prop:b".into(), "proof:query".into()])
        );
        assert!(cone.re_review_required);
        assert!(!cone.creates_claim_truth);
    }

    #[test]
    fn unchanged_revision_does_not_schedule_re_review() {
        let old = world("world:old", "AU", "2026-09-20", "rev:1");
        let new = world("world:new", "AU", "2026-09-21", "rev:1");
        let invalidation = diff_world_revisions(&old, &new).unwrap();
        let cone = affected_proof_cone(&invalidation, &RevisionDependencyIndex::default()).unwrap();
        assert!(invalidation.changes.is_empty());
        assert!(!cone.re_review_required);
        assert!(cone.transitively_affected_refs.is_empty());
    }
}
