//! Rust-native structural contracts for the governed-online SensibLaw lane.
//!
//! These tests replace the Python source-contract scripts introduced by PR #13.
//! They are intentionally static complements to the typed/unit/runtime tests:
//! the scheduler must not gain transport, provider access must remain bounded,
//! source observation must not auto-promote legal semantics, and retained live
//! artifacts must remain bound to the exact residual before acquisition/review.

const ONLINE: &str = include_str!("../src/online.rs");
const PROVIDER_BOUNDARY: &str = include_str!("../src/provider.rs");
const PROVIDER_ACCESS: &str = include_str!("../src/provider_access_policy.rs");
const BOUND_ACQUISITION: &str = include_str!("../src/bound_acquisition.rs");
const JUDGMENT_CANDIDATES: &str = include_str!("../src/judgment_candidates.rs");
const JUDGMENT_REVIEW: &str = include_str!("../src/judgment_review.rs");
const SHORTLIST: &str = include_str!("../src/residual_review_shortlist.rs");
const LIVE_ARTIFACT: &str = include_str!("../src/live_artifact.rs");
const PROVIDER_LIB: &str = include_str!("../../sl-governed-legal-provider/src/lib.rs");
const PROVIDER_EXECUTION: &str = include_str!("../../sl-governed-legal-provider/src/execution.rs");
const PROVIDER_TRANSPORT: &str = include_str!("../../sl-governed-legal-provider/src/transport.rs");
const DOCX_TEXT: &str = include_str!("../../sl-governed-legal-provider/src/docx_text.rs");
const OFFICIAL_RESOURCE: &str = include_str!("../../sl-governed-legal-provider/src/official_resource.rs");

fn assert_contains_all(haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(haystack.contains(needle), "missing source contract token: {needle}");
    }
}

#[test]
fn scheduler_and_policy_surface_remain_transport_free() {
    assert_contains_all(
        PROVIDER_BOUNDARY,
        &[
            "GovernedLegalProvider",
            "SearchReferenceReceipt",
            "FetchBytesReceipt",
            "cache_checked_first",
            "persisted_receipts_checked_first",
            "crawling_permitted",
            "ad_hoc_polling_permitted",
        ],
    );
    assert_contains_all(
        PROVIDER_ACCESS,
        &[
            "SELF_IMPOSED_MINIMUM_INTERVAL_SECONDS: u64 = 4",
            "SELF_IMPOSED_BURST_LIMIT: u32 = 1",
            "PublishedRateLimit::Unknown",
            "ProviderNetworkMode::BoundedLive",
            "ProviderNetworkMode::BulkSnapshotLocalFirst",
            "ProviderNetworkMode::AuthorisationRequired",
            "ProviderNetworkMode::NotConfigured",
            "missing_published_numeric_limit_means_unlimited() -> bool",
            "provider_permission_implies_semantic_authority() -> bool",
            "reuse_permission_implies_current_authority() -> bool",
        ],
    );
    for forbidden in ["reqwest", "TcpStream", "std::net", "sleep("] {
        assert!(!PROVIDER_BOUNDARY.contains(forbidden));
        assert!(!PROVIDER_ACCESS.contains(forbidden));
    }
}

#[test]
fn experimental_and_production_online_readiness_stay_distinct() {
    assert_contains_all(
        ONLINE,
        &[
            "ExperimentalLiveAcquisitionReady",
            "ProductionGovernedOnlineReady",
            "exact_head_live_execution_receipt",
            "exact_head_agda_kernel_receipt",
            "LiveReceiptLineageMissing",
        ],
    );
}

#[test]
fn acquisition_must_bind_to_exact_live_residual_before_provider_execution() {
    assert_contains_all(
        BOUND_ACQUISITION,
        &[
            "ResidualBoundAuthorityDemand",
            "bind_authority_demand",
            "residual_ref",
            "proposition_ref",
            "producer",
            "jurisdiction",
            "ResidualStatus::Open",
        ],
    );
    assert_contains_all(
        LIVE_ARTIFACT,
        &[
            "CULLEN_RESIDUAL_REF",
            "CULLEN_PROPOSITION_REF",
            "CANDIDATE_ONLY_AUTHORITY",
            "landing_page_network_requests",
            "document_fetch",
            "replay_run",
            "UnexpectedResidual",
            "UnexpectedProposition",
        ],
    );
}

#[test]
fn source_observation_shortlisting_and_review_do_not_collapse() {
    assert_contains_all(
        JUDGMENT_CANDIDATES,
        &[
            "CitationOccurrenceCandidate",
            "FootnoteAnchorObservation",
            "reviewed: bool",
            "candidate_only: bool",
            "anchor_paragraph_locator_refs",
            "anchor_paragraph_texts",
        ],
    );
    assert_contains_all(
        SHORTLIST,
        &[
            "ResidualCitationReviewDemand",
            "ResidualAnchorCriterion",
            "ResidualShortlistedCitation",
            "shortlist_anchored_citations_for_residual",
        ],
    );
    assert_contains_all(
        JUDGMENT_REVIEW,
        &[
            "ReviewedCitationDecision",
            "compile_reviewed_citation_edge",
            "candidate_only",
        ],
    );
}

#[test]
fn provider_acquisition_stays_reference_bytes_then_local_materialization() {
    assert_contains_all(
        PROVIDER_LIB,
        &[
            "experimental_candidate_only",
            "network_requests",
        ],
    );
    assert_contains_all(
        PROVIDER_EXECUTION,
        &[
            "network_requests",
            "locally_ingested",
        ],
    );
    assert_contains_all(
        PROVIDER_TRANSPORT,
        &[
            "live-network",
        ],
    );
    assert_contains_all(
        OFFICIAL_RESOURCE,
        &[
            "docx",
            "pdf",
        ],
    );
    assert_contains_all(
        DOCX_TEXT,
        &[
            "footnotes.xml",
            "footnoteReference",
        ],
    );
}
