use sensiblaw_proof_search_loop::online::{
    blockers, readiness, OnlineReadiness, OnlineReadinessBlocker, OnlineReadinessInputs,
};

fn main() {
    let current = OnlineReadinessInputs {
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
        // sensiblaw-governed-legal-provider now contains the separately gated
        // AustLII/JADE execution implementation. The live fixture remains false
        // until an operator explicitly runs and records the bounded smoke test.
        live_provider_adapter_implemented: true,
        live_provider_fixture_validated: false,
        exact_head_agda_kernel_receipt: false,
    };

    let state = readiness(current);
    let remaining = blockers(current);
    assert_eq!(state, OnlineReadiness::OfflineOnly);
    assert_eq!(
        remaining,
        vec![
            OnlineReadinessBlocker::LiveProviderFixtureMissing,
            OnlineReadinessBlocker::ExactHeadAgdaKernelReceiptMissing,
        ]
    );

    println!("readiness={state:?} blockers={remaining:?}");
}
