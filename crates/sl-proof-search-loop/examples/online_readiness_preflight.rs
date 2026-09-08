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
        live_provider_adapter_implemented: true,
        // A bounded official HCA acquisition succeeded and replayed locally with
        // zero network. The receipt is pinned to 9c3007..., while bb6de85...
        // locally validates the repaired branch. The separate lineage audit
        // certifies that the governed provider runtime source itself did not
        // change across those heads.
        live_provider_fixture_validated: true,
        live_receipt_lineage_validated: true,
        exact_head_live_execution_receipt: false,
        exact_head_agda_kernel_receipt: false,
    };

    let state = readiness(current);
    let remaining = blockers(current);
    assert_eq!(state, OnlineReadiness::ExperimentalLiveAcquisitionReady);
    assert_eq!(
        remaining,
        vec![
            OnlineReadinessBlocker::ExactHeadLiveExecutionReceiptMissing,
            OnlineReadinessBlocker::ExactHeadAgdaKernelReceiptMissing,
        ]
    );

    println!("readiness={state:?} blockers={remaining:?}");
}
