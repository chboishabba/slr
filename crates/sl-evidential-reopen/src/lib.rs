//! Evidential-PNF handoff and sparse consumer reopening for SensibLaw.
//!
//! This crate is store-neutral and I/O-free.  It preserves two mature SensibLaw
//! contracts without importing the Python/PostgreSQL runtime:
//!
//! 1. parser/numeric PNF is a source-anchored computational observation, not
//!    semantic correspondence, world truth or cross-document identity authority;
//! 2. new evidence wakes only consumer/query/policy fibres that explicitly
//!    registered reverse dependencies on changed source coordinates.

use std::collections::BTreeSet;

pub const EVIDENTIAL_BRIDGE_SCHEMA: &str = "sl.evidential_pnf_bridge.v0_1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidentialBridgeReceipt {
    pub schema_version: &'static str,
    pub run_ref: String,
    pub document_ref: String,
    pub canonical_text_sha256: String,
    pub parser_contract_ref: String,
    pub numeric_pnf_compiler_contract_ref: String,
    pub graph_ref: String,
    pub residual_demand_refs: Vec<String>,
    pub representation: String,
    pub world_resolution_deferred: bool,
    pub cross_document_identity_closed: bool,
    pub legacy_document_materialisation: bool,
    pub parser_observation_is_semantic_authority: bool,
    pub semantic_correspondence_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeError {
    WorldResolutionNotDeferred,
    CrossDocumentIdentityClaimedClosed,
    LegacyMaterialisationRequired,
    EmptyRequiredReference,
    EmptyResidualReference,
}

impl EvidentialBridgeReceipt {
    pub fn new(
        run_ref: String,
        document_ref: String,
        canonical_text_sha256: String,
        parser_contract_ref: String,
        numeric_pnf_compiler_contract_ref: String,
        graph_ref: String,
        residual_demand_refs: Vec<String>,
        representation: String,
        world_resolution_deferred: bool,
        cross_document_identity_closed: bool,
        legacy_document_materialisation: bool,
    ) -> Result<Self, BridgeError> {
        if !world_resolution_deferred {
            return Err(BridgeError::WorldResolutionNotDeferred);
        }
        if cross_document_identity_closed {
            return Err(BridgeError::CrossDocumentIdentityClaimedClosed);
        }
        if legacy_document_materialisation {
            return Err(BridgeError::LegacyMaterialisationRequired);
        }
        if [
            &run_ref,
            &document_ref,
            &canonical_text_sha256,
            &parser_contract_ref,
            &numeric_pnf_compiler_contract_ref,
            &graph_ref,
            &representation,
        ]
        .iter()
        .any(|value| value.is_empty())
        {
            return Err(BridgeError::EmptyRequiredReference);
        }
        if residual_demand_refs.iter().any(String::is_empty) {
            return Err(BridgeError::EmptyResidualReference);
        }
        Ok(Self {
            schema_version: EVIDENTIAL_BRIDGE_SCHEMA,
            run_ref,
            document_ref,
            canonical_text_sha256,
            parser_contract_ref,
            numeric_pnf_compiler_contract_ref,
            graph_ref,
            residual_demand_refs,
            representation,
            world_resolution_deferred,
            cross_document_identity_closed,
            legacy_document_materialisation,
            parser_observation_is_semantic_authority: false,
            semantic_correspondence_required: true,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrespondenceStatus {
    Unreviewed,
    ReviewedSupported,
    ReviewedRejected,
    ReviewedAmbiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedCorrespondenceReceipt {
    pub bridge_document_ref: String,
    pub graph_ref: String,
    pub source_span_ref: String,
    pub proposition_ref: String,
    pub status: CorrespondenceStatus,
    pub reviewer_ref: String,
    pub evidence_refs: Vec<String>,
    pub world_truth_claimed: bool,
    pub legal_holding_claimed: bool,
}

impl ReviewedCorrespondenceReceipt {
    pub fn supported(
        bridge: &EvidentialBridgeReceipt,
        source_span_ref: String,
        proposition_ref: String,
        reviewer_ref: String,
        evidence_refs: Vec<String>,
    ) -> Self {
        Self {
            bridge_document_ref: bridge.document_ref.clone(),
            graph_ref: bridge.graph_ref.clone(),
            source_span_ref,
            proposition_ref,
            status: CorrespondenceStatus::ReviewedSupported,
            reviewer_ref,
            evidence_refs,
            world_truth_claimed: false,
            legal_holding_claimed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceCoordinateKind {
    Evidence,
    SourceRegion,
    SourceInterface,
    Proposition,
    AuthorityReceipt,
    CounterfactualWorld,
    CounterfactualRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceCoordinate {
    pub kind: SourceCoordinateKind,
    pub stable_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConsumerFibreKey {
    pub demand_ref: String,
    pub consumer_ref: String,
    pub query_ref: String,
    pub policy_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseDependency {
    pub source: SourceCoordinate,
    pub consumer: ConsumerFibreKey,
    pub minimum_horizon: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WakeRequest {
    pub consumer: ConsumerFibreKey,
    pub minimum_horizon: u8,
}

/// Compute only the consumer fibres whose declared reverse dependencies match
/// one of the changed coordinates.  Missing dependency == zero work.
pub fn sparse_wake(
    changed: &[SourceCoordinate],
    dependencies: &[ReverseDependency],
) -> Vec<WakeRequest> {
    let changed_set: BTreeSet<&SourceCoordinate> = changed.iter().collect();
    let mut wakes = BTreeSet::new();
    for dependency in dependencies {
        if changed_set.contains(&dependency.source) {
            wakes.insert(WakeRequest {
                consumer: dependency.consumer.clone(),
                minimum_horizon: dependency.minimum_horizon,
            });
        }
    }
    wakes.into_iter().collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Horizon {
    H3,
    H6,
    H9,
}

impl Horizon {
    pub const fn code(self) -> u8 {
        match self {
            Self::H3 => 3,
            Self::H6 => 6,
            Self::H9 => 9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerResidualContract {
    pub consumer: ConsumerFibreKey,
    pub required_coordinates: Vec<SourceCoordinate>,
    pub minimum_horizon: Horizon,
}

pub fn dependencies_for(contract: &ConsumerResidualContract) -> Vec<ReverseDependency> {
    contract
        .required_coordinates
        .iter()
        .cloned()
        .map(|source| ReverseDependency {
            source,
            consumer: contract.consumer.clone(),
            minimum_horizon: contract.minimum_horizon.code(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge() -> EvidentialBridgeReceipt {
        EvidentialBridgeReceipt::new(
            "run:1".into(),
            "doc:1".into(),
            "sha256:abc".into(),
            "parser:v1".into(),
            "numeric-pnf:v1".into(),
            "graph:1".into(),
            vec!["residual:1".into()],
            "numeric-pnf".into(),
            true,
            false,
            false,
        )
        .unwrap()
    }

    #[test]
    fn parser_bridge_never_claims_semantic_authority() {
        let receipt = bridge();
        assert!(!receipt.parser_observation_is_semantic_authority);
        assert!(receipt.semantic_correspondence_required);
        assert!(receipt.world_resolution_deferred);
        assert!(!receipt.cross_document_identity_closed);
    }

    #[test]
    fn reviewed_correspondence_still_does_not_claim_world_truth_or_holding() {
        let receipt = ReviewedCorrespondenceReceipt::supported(
            &bridge(),
            "span:10-20".into(),
            "proposition:p".into(),
            "reviewer:r".into(),
            vec!["source:primary".into()],
        );
        assert_eq!(receipt.status, CorrespondenceStatus::ReviewedSupported);
        assert!(!receipt.world_truth_claimed);
        assert!(!receipt.legal_holding_claimed);
    }

    #[test]
    fn missing_reverse_dependency_means_zero_work_not_negative_evidence() {
        let changed = vec![SourceCoordinate {
            kind: SourceCoordinateKind::Evidence,
            stable_ref: "evidence:new".into(),
        }];
        assert!(sparse_wake(&changed, &[]).is_empty());
    }

    #[test]
    fn evidence_wakes_only_explicitly_dependent_consumer() {
        let source = SourceCoordinate {
            kind: SourceCoordinateKind::Evidence,
            stable_ref: "evidence:42".into(),
        };
        let interested = ConsumerFibreKey {
            demand_ref: "d:1".into(),
            consumer_ref: "consumer:factual-causation".into(),
            query_ref: "q:1".into(),
            policy_ref: "policy:1".into(),
        };
        let unrelated = ConsumerFibreKey {
            demand_ref: "d:2".into(),
            consumer_ref: "consumer:remedy".into(),
            query_ref: "q:2".into(),
            policy_ref: "policy:1".into(),
        };
        let dependencies = vec![
            ReverseDependency {
                source: source.clone(),
                consumer: interested.clone(),
                minimum_horizon: 6,
            },
            ReverseDependency {
                source: SourceCoordinate {
                    kind: SourceCoordinateKind::Evidence,
                    stable_ref: "evidence:other".into(),
                },
                consumer: unrelated,
                minimum_horizon: 9,
            },
        ];
        let wakes = sparse_wake(&[source], &dependencies);
        assert_eq!(wakes.len(), 1);
        assert_eq!(wakes[0].consumer, interested);
        assert_eq!(wakes[0].minimum_horizon, 6);
    }

    #[test]
    fn one_contract_compiles_to_exact_reverse_dependencies() {
        let contract = ConsumerResidualContract {
            consumer: ConsumerFibreKey {
                demand_ref: "d:cf".into(),
                consumer_ref: "consumer:liability".into(),
                query_ref: "q:liability".into(),
                policy_ref: "policy:au".into(),
            },
            required_coordinates: vec![
                SourceCoordinate {
                    kind: SourceCoordinateKind::AuthorityReceipt,
                    stable_ref: "authority:qld:s11".into(),
                },
                SourceCoordinate {
                    kind: SourceCoordinateKind::CounterfactualRelation,
                    stable_ref: "relation:corrected-conduct".into(),
                },
            ],
            minimum_horizon: Horizon::H6,
        };
        let dependencies = dependencies_for(&contract);
        assert_eq!(dependencies.len(), 2);
        assert!(dependencies.iter().all(|row| row.minimum_horizon == 6));
    }
}
