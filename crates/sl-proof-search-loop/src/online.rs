//! Governed-online readiness gates for the legal research engine.
//!
//! This module does not perform network I/O. It distinguishes readiness to
//! begin bounded experimental provider acquisition from readiness to treat the
//! online path as production-governed. The latter additionally requires the
//! exact-head Agda/kernel semantic receipt and provider execution receipts.

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
    if !inputs.exact_head_agda_kernel_receipt {
        out.push(OnlineReadinessBlocker::ExactHeadAgdaKernelReceiptMissing);
    }
    out
}

pub fn readiness(inputs: OnlineReadinessInputs) -> OnlineReadiness {
    let bs = blockers(inputs);
    let structural_blocked = bs.iter().any(|b| matches!(
        b,
        OnlineReadinessBlocker::OfflineResearchEngineIncomplete
            | OnlineReadinessBlocker::GovernanceContractIncomplete
            | OnlineReadinessBlocker::LiveProviderAdapterMissing
            | OnlineReadinessBlocker::LiveProviderFixtureMissing
    ));
    if structural_blocked {
        return OnlineReadiness::OfflineOnly;
    }
    if bs.contains(&OnlineReadinessBlocker::ExactHeadAgdaKernelReceiptMissing) {
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
            exact_head_agda_kernel_receipt: false,
        }
    }

    #[test]
    fn current_state_is_offline_only_until_live_adapter_and_fixture_exist() {
        let inputs = validated_offline();
        assert_eq!(readiness(inputs), OnlineReadiness::OfflineOnly);
        assert_eq!(
            blockers(inputs),
            vec![
                OnlineReadinessBlocker::LiveProviderAdapterMissing,
                OnlineReadinessBlocker::LiveProviderFixtureMissing,
                OnlineReadinessBlocker::ExactHeadAgdaKernelReceiptMissing,
            ]
        );
    }

    #[test]
    fn agda_receipt_is_not_required_to_test_bounded_live_acquisition_experimentally() {
        let mut inputs = validated_offline();
        inputs.live_provider_adapter_implemented = true;
        inputs.live_provider_fixture_validated = true;
        assert_eq!(readiness(inputs), OnlineReadiness::ExperimentalLiveAcquisitionReady);
    }

    #[test]
    fn production_online_requires_exact_head_agda_kernel_receipt() {
        let mut inputs = validated_offline();
        inputs.live_provider_adapter_implemented = true;
        inputs.live_provider_fixture_validated = true;
        inputs.exact_head_agda_kernel_receipt = true;
        assert_eq!(readiness(inputs), OnlineReadiness::ProductionGovernedOnlineReady);
    }
}
