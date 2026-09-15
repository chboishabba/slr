//! Governed-online readiness gates for the legal research engine.
//!
//! This module does not perform network I/O. It distinguishes readiness to
//! begin bounded experimental provider acquisition from readiness to treat the
//! online path as production-governed. A bounded live receipt may calibrate a
//! later locally-validated head through an explicit no-network lineage audit
//! when the governed provider runtime source is unchanged; this is not the same
//! as claiming exact-head live execution or byte-for-byte build identity.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnlineReadinessInputs {
    pub multi_residual_frontier_validated: bool,
    pub dialectical_hypotheses_validated: bool,
    pub provider_neutral_query_algebra_validated: bool,
    pub local_corpus_validated: bool,
    pub immutable_source_revisions_validated: bool,
    pub reasoning_graph_enrichment_validated: bool,
    pub deterministic_receipts_validated: bool,
    pub local_compounding_iteration_validated: bool,
    pub scheduler_cannot_own_http: bool,
    pub provider_interface_separate: bool,
    pub pacing_policy_pinned: bool,
    pub explicit_follow_bounds_pinned: bool,
    pub cache_and_persisted_first_pinned: bool,
    pub no_crawl_no_polling_pinned: bool,
    pub search_returns_references_pinned: bool,
    pub fetch_returns_bytes_pinned: bool,
    pub parser_only_after_local_ingestion_pinned: bool,
    pub live_provider_adapter_implemented: bool,
    pub live_provider_fixture_validated: bool,
    pub live_receipt_lineage_validated: bool,
    pub exact_head_live_execution_receipt: bool,
    pub exact_head_agda_kernel_receipt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlineReadiness {
    OfflineOnly,
    ExperimentalLiveAcquisitionReady,
    ProductionGovernedOnlineReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlineReadinessBlocker {
    OfflineResearchEngineIncomplete,
    GovernanceContractIncomplete,
    LiveProviderAdapterMissing,
    LiveProviderFixtureMissing,
    LiveReceiptLineageMissing,
    ExactHeadLiveExecutionReceiptMissing,
    ExactHeadAgdaKernelReceiptMissing,
}

pub fn blockers(inputs: OnlineReadinessInputs) -> Vec<OnlineReadinessBlocker> {
    let offline_complete = inputs.multi_residual_frontier_validated
        && inputs.dialectical_hypotheses_validated
        && inputs.provider_neutral_query_algebra_validated
        && inputs.local_corpus_validated
        && inputs.immutable_source_revisions_validated
        && inputs.reasoning_graph_enrichment_validated
        && inputs.deterministic_receipts_validated
        && inputs.local_compounding_iteration_validated;

    let governance_complete = inputs.scheduler_cannot_own_http
        && inputs.provider_interface_separate
        && inputs.pacing_policy_pinned
        && inputs.explicit_follow_bounds_pinned
        && inputs.cache_and_persisted_first_pinned
        && inputs.no_crawl_no_polling_pinned
        && inputs.search_returns_references_pinned
        && inputs.fetch_returns_bytes_pinned
        && inputs.parser_only_after_local_ingestion_pinned;

    let mut out = Vec::new();
    if !offline_complete {
        out.push(OnlineReadinessBlocker::OfflineResearchEngineIncomplete);
    }
    if !governance_complete {
        out.push(OnlineReadinessBlocker::GovernanceContractIncomplete);
    }
    if !inputs.live_provider_adapter_implemented {
        out.push(OnlineReadinessBlocker::LiveProviderAdapterMissing);
    }
    if !inputs.live_provider_fixture_validated {
        out.push(OnlineReadinessBlocker::LiveProviderFixtureMissing);
    }
    if inputs.live_provider_fixture_validated
        && !inputs.exact_head_live_execution_receipt
        && !inputs.live_receipt_lineage_validated
    {
        out.push(OnlineReadinessBlocker::LiveReceiptLineageMissing);
    }
    if !inputs.exact_head_live_execution_receipt {
        out.push(OnlineReadinessBlocker::ExactHeadLiveExecutionReceiptMissing);
    }
    if !inputs.exact_head_agda_kernel_receipt {
        out.push(OnlineReadinessBlocker::ExactHeadAgdaKernelReceiptMissing);
    }
    out
}

pub fn readiness(inputs: OnlineReadinessInputs) -> OnlineReadiness {
    let bs = blockers(inputs);
    let experimental_blocked = bs.iter().any(|b| {
        matches!(
            b,
            OnlineReadinessBlocker::OfflineResearchEngineIncomplete
                | OnlineReadinessBlocker::GovernanceContractIncomplete
                | OnlineReadinessBlocker::LiveProviderAdapterMissing
                | OnlineReadinessBlocker::LiveProviderFixtureMissing
                | OnlineReadinessBlocker::LiveReceiptLineageMissing
        )
    });
    if experimental_blocked {
        return OnlineReadiness::OfflineOnly;
    }

    let production_blocked = bs.iter().any(|b| {
        matches!(
            b,
            OnlineReadinessBlocker::ExactHeadLiveExecutionReceiptMissing
                | OnlineReadinessBlocker::ExactHeadAgdaKernelReceiptMissing
        )
    });
    if production_blocked {
        return OnlineReadiness::ExperimentalLiveAcquisitionReady;
    }
    OnlineReadiness::ProductionGovernedOnlineReady
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validated_offline() -> OnlineReadinessInputs {
        OnlineReadinessInputs {
            multi_residual_frontier_validated: true,
            dialectical_hypotheses_validated: true,
            provider_neutral_query_algebra_validated: true,
            local_corpus_validated: true,
            immutable_source_revisions_validated: true,
            reasoning_graph_enrichment_validated: true,
            deterministic_receipts_validated: true,
            local_compounding_iteration_validated: true,
            scheduler_cannot_own_http: true,
            provider_interface_separate: true,
            pacing_policy_pinned: true,
            explicit_follow_bounds_pinned: true,
            cache_and_persisted_first_pinned: true,
            no_crawl_no_polling_pinned: true,
            search_returns_references_pinned: true,
            fetch_returns_bytes_pinned: true,
            parser_only_after_local_ingestion_pinned: true,
            live_provider_adapter_implemented: false,
            live_provider_fixture_validated: false,
            live_receipt_lineage_validated: false,
            exact_head_live_execution_receipt: false,
            exact_head_agda_kernel_receipt: false,
        }
    }

    #[test]
    fn no_live_fixture_remains_offline_only() {
        let inputs = validated_offline();
        assert_eq!(readiness(inputs), OnlineReadiness::OfflineOnly);
        assert!(blockers(inputs).contains(&OnlineReadinessBlocker::LiveProviderFixtureMissing));
    }

    #[test]
    fn validated_fixture_plus_provider_source_lineage_is_experimental_ready() {
        let mut inputs = validated_offline();
        inputs.live_provider_adapter_implemented = true;
        inputs.live_provider_fixture_validated = true;
        inputs.live_receipt_lineage_validated = true;
        assert_eq!(readiness(inputs), OnlineReadiness::ExperimentalLiveAcquisitionReady);
        assert!(blockers(inputs).contains(&OnlineReadinessBlocker::ExactHeadLiveExecutionReceiptMissing));
        assert!(blockers(inputs).contains(&OnlineReadinessBlocker::ExactHeadAgdaKernelReceiptMissing));
    }

    #[test]
    fn exact_head_live_receipt_does_not_need_lineage_bridge() {
        let mut inputs = validated_offline();
        inputs.live_provider_adapter_implemented = true;
        inputs.live_provider_fixture_validated = true;
        inputs.exact_head_live_execution_receipt = true;
        assert_eq!(readiness(inputs), OnlineReadiness::ExperimentalLiveAcquisitionReady);
        assert!(!blockers(inputs).contains(&OnlineReadinessBlocker::LiveReceiptLineageMissing));
    }

    #[test]
    fn production_online_requires_exact_head_live_and_agda_receipts() {
        let mut inputs = validated_offline();
        inputs.live_provider_adapter_implemented = true;
        inputs.live_provider_fixture_validated = true;
        inputs.exact_head_live_execution_receipt = true;
        inputs.exact_head_agda_kernel_receipt = true;
        assert_eq!(readiness(inputs), OnlineReadiness::ProductionGovernedOnlineReady);
    }
}
