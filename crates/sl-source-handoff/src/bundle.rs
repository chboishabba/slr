//! Full Wikidata statement-bundle review carrier and selective reopen bridge.
//!
//! SensibLaw's Nat/Climate protocol treats the migration unit as the whole
//! statement bundle, not a naked `(subject, property, value)` triple.  This
//! module preserves mainsnak, qualifiers, references, rank and provenance as
//! independent review coordinates and compiles bundle changes into the existing
//! sparse reverse-dependency wake engine.

use sensiblaw_evidential_reopen::{
    sparse_wake, ConsumerFibreKey, ReverseDependency, SourceCoordinate, SourceCoordinateKind,
    WakeRequest,
};

use crate::MigrationDisposition;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BundleCoordinateKind {
    MainSnak,
    Qualifier,
    Reference,
    Rank,
    Provenance,
}

impl BundleCoordinateKind {
    pub const fn code(self) -> &'static str {
        match self {
            Self::MainSnak => "mainsnak",
            Self::Qualifier => "qualifier",
            Self::Reference => "reference",
            Self::Rank => "rank",
            Self::Provenance => "provenance",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BundleCoordinate {
    pub kind: BundleCoordinateKind,
    pub stable_ref: String,
}

impl BundleCoordinate {
    pub fn source_coordinate(&self) -> SourceCoordinate {
        SourceCoordinate {
            // Bundle coordinates are source-facing evidence coordinates.  The
            // stable_ref preserves the finer bundle kind for exact matching.
            kind: SourceCoordinateKind::Evidence,
            stable_ref: format!("wikidata-bundle:{}:{}", self.kind.code(), self.stable_ref),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikidataRank {
    Preferred,
    Normal,
    Deprecated,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementBundleReview {
    pub bundle_id: String,
    pub source_unit_id: String,
    pub subject_qid: String,
    pub property_id: String,
    pub mainsnak_ref: String,
    pub qualifier_refs: Vec<String>,
    pub reference_refs: Vec<String>,
    pub rank: WikidataRank,
    pub provenance_refs: Vec<String>,
    pub disposition: MigrationDisposition,
    pub unresolved_reasons: Vec<String>,
    pub creates_world_truth: bool,
    pub owns_source_authority: bool,
    pub owns_semantic_promotion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleError {
    EmptyRequiredField,
    InvalidQid,
    InvalidPid,
    MissingProvenance,
}

impl StatementBundleReview {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bundle_id: String,
        source_unit_id: String,
        subject_qid: String,
        property_id: String,
        mainsnak_ref: String,
        qualifier_refs: Vec<String>,
        reference_refs: Vec<String>,
        rank: WikidataRank,
        provenance_refs: Vec<String>,
        disposition: MigrationDisposition,
        unresolved_reasons: Vec<String>,
    ) -> Result<Self, BundleError> {
        if [&bundle_id, &source_unit_id, &mainsnak_ref]
            .iter()
            .any(|value| value.is_empty())
        {
            return Err(BundleError::EmptyRequiredField);
        }
        if !is_qid(&subject_qid) {
            return Err(BundleError::InvalidQid);
        }
        if !is_pid(&property_id) {
            return Err(BundleError::InvalidPid);
        }
        if provenance_refs.is_empty() || provenance_refs.iter().any(String::is_empty) {
            return Err(BundleError::MissingProvenance);
        }
        Ok(Self {
            bundle_id,
            source_unit_id,
            subject_qid,
            property_id,
            mainsnak_ref,
            qualifier_refs,
            reference_refs,
            rank,
            provenance_refs,
            disposition,
            unresolved_reasons,
            creates_world_truth: false,
            owns_source_authority: false,
            owns_semantic_promotion: false,
        })
    }

    pub fn coordinates(&self) -> Vec<BundleCoordinate> {
        let mut out = vec![BundleCoordinate {
            kind: BundleCoordinateKind::MainSnak,
            stable_ref: format!("{}:{}", self.bundle_id, self.mainsnak_ref),
        }];
        out.extend(self.qualifier_refs.iter().cloned().map(|stable_ref| BundleCoordinate {
            kind: BundleCoordinateKind::Qualifier,
            stable_ref: format!("{}:{}", self.bundle_id, stable_ref),
        }));
        out.extend(self.reference_refs.iter().cloned().map(|stable_ref| BundleCoordinate {
            kind: BundleCoordinateKind::Reference,
            stable_ref: format!("{}:{}", self.bundle_id, stable_ref),
        }));
        out.push(BundleCoordinate {
            kind: BundleCoordinateKind::Rank,
            stable_ref: format!("{}:{:?}", self.bundle_id, self.rank),
        });
        out.extend(self.provenance_refs.iter().cloned().map(|stable_ref| BundleCoordinate {
            kind: BundleCoordinateKind::Provenance,
            stable_ref: format!("{}:{}", self.bundle_id, stable_ref),
        }));
        out
    }
}

fn is_qid(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('Q') else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

fn is_pid(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('P') else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleDependency {
    pub coordinate: BundleCoordinate,
    pub consumer: ConsumerFibreKey,
    pub minimum_horizon: u8,
}

impl BundleDependency {
    pub fn reverse_dependency(&self) -> ReverseDependency {
        ReverseDependency {
            source: self.coordinate.source_coordinate(),
            consumer: self.consumer.clone(),
            minimum_horizon: self.minimum_horizon,
        }
    }
}

/// Wake only consumers that declared dependency on one of the changed bundle
/// coordinates.  A changed P854/reference coordinate therefore does not imply
/// re-review of mainsnak/rank/provenance consumers unless they declared that
/// dependency explicitly.
pub fn sparse_wake_bundle_changes(
    changed: &[BundleCoordinate],
    dependencies: &[BundleDependency],
) -> Vec<WakeRequest> {
    let changed_sources: Vec<_> = changed.iter().map(BundleCoordinate::source_coordinate).collect();
    let reverse_dependencies: Vec<_> = dependencies
        .iter()
        .map(BundleDependency::reverse_dependency)
        .collect();
    sparse_wake(&changed_sources, &reverse_dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consumer(name: &str) -> ConsumerFibreKey {
        ConsumerFibreKey {
            demand_ref: format!("demand:{name}"),
            consumer_ref: format!("consumer:{name}"),
            query_ref: "query:nat-p5991-p14143".into(),
            policy_ref: "policy:nat-review-v1".into(),
        }
    }

    fn nat_bundle() -> StatementBundleReview {
        StatementBundleReview::new(
            "bundle:nat:q10403939:scope1:2018".into(),
            "unit:wikidata_user_sandbox:nat_wdu:p5991_p14143:2026-04-01".into(),
            "Q10403939".into(),
            "P5991".into(),
            "quantity:scope1:2018".into(),
            vec!["P3831:scope1".into(), "P459:ghg-protocol".into(), "P580:2018".into()],
            vec!["P854:https://example.test/report.pdf".into()],
            WikidataRank::Normal,
            vec!["source-unit:nat-p5991-p14143".into()],
            MigrationDisposition::SplitRequired,
            vec!["scope/time decomposition unresolved".into()],
        )
        .unwrap()
    }

    #[test]
    fn bundle_keeps_review_coordinates_separate_and_non_authoritative() {
        let bundle = nat_bundle();
        let coordinates = bundle.coordinates();
        assert!(coordinates.iter().any(|c| c.kind == BundleCoordinateKind::MainSnak));
        assert!(coordinates.iter().any(|c| c.kind == BundleCoordinateKind::Qualifier));
        assert!(coordinates.iter().any(|c| c.kind == BundleCoordinateKind::Reference));
        assert!(coordinates.iter().any(|c| c.kind == BundleCoordinateKind::Rank));
        assert!(coordinates.iter().any(|c| c.kind == BundleCoordinateKind::Provenance));
        assert!(!bundle.creates_world_truth);
        assert!(!bundle.owns_source_authority);
        assert!(!bundle.owns_semantic_promotion);
    }

    #[test]
    fn changed_reference_wakes_only_reference_dependent_consumer() {
        let bundle = nat_bundle();
        let reference_coordinate = bundle
            .coordinates()
            .into_iter()
            .find(|c| c.kind == BundleCoordinateKind::Reference)
            .unwrap();
        let mainsnak_coordinate = bundle
            .coordinates()
            .into_iter()
            .find(|c| c.kind == BundleCoordinateKind::MainSnak)
            .unwrap();

        let reference_consumer = consumer("reference-transfer");
        let model_consumer = consumer("semantic-equivalence");
        let dependencies = vec![
            BundleDependency {
                coordinate: reference_coordinate.clone(),
                consumer: reference_consumer.clone(),
                minimum_horizon: 3,
            },
            BundleDependency {
                coordinate: mainsnak_coordinate,
                consumer: model_consumer,
                minimum_horizon: 6,
            },
        ];

        let wakes = sparse_wake_bundle_changes(&[reference_coordinate], &dependencies);
        assert_eq!(wakes.len(), 1);
        assert_eq!(wakes[0].consumer, reference_consumer);
        assert_eq!(wakes[0].minimum_horizon, 3);
    }

    #[test]
    fn missing_bundle_dependency_means_zero_work_not_negative_evidence() {
        let bundle = nat_bundle();
        let changed = bundle
            .coordinates()
            .into_iter()
            .find(|c| c.kind == BundleCoordinateKind::Rank)
            .unwrap();
        assert!(sparse_wake_bundle_changes(&[changed], &[]).is_empty());
    }
}
