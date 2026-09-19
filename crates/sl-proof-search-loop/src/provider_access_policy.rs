//! Provider access policy for governed online legal research.
//!
//! This module separates four coordinates that must not be collapsed:
//! 1. provider-published access/rate guidance;
//! 2. SensibLaw's stricter self-imposed pacing floor;
//! 3. whether a provider should be used per-residual at all;
//! 4. reuse/attribution obligations that travel with acquired artifacts.
//!
//! Absence of a published numeric limit is `PublishedRateLimit::Unknown`, never
//! zero or unlimited.  For legal hosts with no stricter published minimum,
//! SensibLaw retains the historical 4-second floor (0.25 requests/second,
//! burst 1).  Bulk corpora such as OALC should be acquired/cached as snapshots
//! and queried locally rather than hit once per residual.

pub const SELF_IMPOSED_MINIMUM_INTERVAL_SECONDS: u64 = 4;
pub const SELF_IMPOSED_BURST_LIMIT: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishedRateLimit {
    Unknown,
    MinimumIntervalSeconds(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderNetworkMode {
    BoundedLive,
    BulkSnapshotLocalFirst,
    AuthorisationRequired,
    NotConfigured,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReuseObligations {
    pub attribution_required: bool,
    pub original_source_url_required: bool,
    pub accuracy_or_unaltered_copy_required: bool,
    pub non_misleading_use_required: bool,
    pub third_party_rights_may_apply: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderAccessPolicy {
    pub provider_ref: &'static str,
    pub policy_source_ref: &'static str,
    pub reuse_policy_source_ref: &'static str,
    pub published_rate_limit: PublishedRateLimit,
    pub self_imposed_minimum_interval_seconds: u64,
    pub burst_limit: u32,
    pub network_mode: ProviderNetworkMode,
    pub cache_first: bool,
    pub per_residual_network_preferred: bool,
    pub reuse: ReuseObligations,
}

const HCA_REUSE: ReuseObligations = ReuseObligations {
    attribution_required: true,
    original_source_url_required: true,
    accuracy_or_unaltered_copy_required: true,
    non_misleading_use_required: true,
    third_party_rights_may_apply: true,
};

const FCA_REUSE: ReuseObligations = ReuseObligations {
    attribution_required: true,
    original_source_url_required: true,
    accuracy_or_unaltered_copy_required: true,
    non_misleading_use_required: false,
    third_party_rights_may_apply: true,
};

const OALC_REUSE: ReuseObligations = ReuseObligations {
    attribution_required: true,
    original_source_url_required: false,
    accuracy_or_unaltered_copy_required: false,
    non_misleading_use_required: false,
    third_party_rights_may_apply: true,
};

const UNKNOWN_REUSE: ReuseObligations = ReuseObligations {
    attribution_required: false,
    original_source_url_required: false,
    accuracy_or_unaltered_copy_required: false,
    non_misleading_use_required: false,
    third_party_rights_may_apply: true,
};

pub const fn high_court_australia_policy() -> ProviderAccessPolicy {
    ProviderAccessPolicy {
        provider_ref: "HighCourtAustralia",
        policy_source_ref: "https://www.hcourt.gov.au/terms-use",
        reuse_policy_source_ref: "https://www.hcourt.gov.au/terms-use",
        published_rate_limit: PublishedRateLimit::Unknown,
        self_imposed_minimum_interval_seconds: SELF_IMPOSED_MINIMUM_INTERVAL_SECONDS,
        burst_limit: SELF_IMPOSED_BURST_LIMIT,
        network_mode: ProviderNetworkMode::BoundedLive,
        cache_first: true,
        per_residual_network_preferred: false,
        reuse: HCA_REUSE,
    }
}

pub const fn federal_court_australia_policy() -> ProviderAccessPolicy {
    ProviderAccessPolicy {
        provider_ref: "FederalCourtAustralia",
        policy_source_ref: "https://www.fedcourt.gov.au/robots.txt",
        reuse_policy_source_ref: "https://www.fedcourt.gov.au/copyright",
        published_rate_limit: PublishedRateLimit::Unknown,
        self_imposed_minimum_interval_seconds: SELF_IMPOSED_MINIMUM_INTERVAL_SECONDS,
        burst_limit: SELF_IMPOSED_BURST_LIMIT,
        network_mode: ProviderNetworkMode::BoundedLive,
        cache_first: true,
        per_residual_network_preferred: false,
        reuse: FCA_REUSE,
    }
}

pub const fn oalc_policy() -> ProviderAccessPolicy {
    ProviderAccessPolicy {
        provider_ref: "OALC",
        policy_source_ref: "https://huggingface.co/datasets/isaacus/open-australian-legal-corpus",
        reuse_policy_source_ref: "https://huggingface.co/datasets/isaacus/open-australian-legal-corpus",
        published_rate_limit: PublishedRateLimit::Unknown,
        self_imposed_minimum_interval_seconds: SELF_IMPOSED_MINIMUM_INTERVAL_SECONDS,
        burst_limit: SELF_IMPOSED_BURST_LIMIT,
        network_mode: ProviderNetworkMode::BulkSnapshotLocalFirst,
        cache_first: true,
        per_residual_network_preferred: false,
        reuse: OALC_REUSE,
    }
}

pub const fn austlii_policy() -> ProviderAccessPolicy {
    ProviderAccessPolicy {
        provider_ref: "AustLII",
        policy_source_ref: "austlii:authorised-research-access-required",
        reuse_policy_source_ref: "austlii:source-specific-reuse-policy-unresolved",
        published_rate_limit: PublishedRateLimit::Unknown,
        self_imposed_minimum_interval_seconds: SELF_IMPOSED_MINIMUM_INTERVAL_SECONDS,
        burst_limit: SELF_IMPOSED_BURST_LIMIT,
        network_mode: ProviderNetworkMode::AuthorisationRequired,
        cache_first: true,
        per_residual_network_preferred: false,
        reuse: UNKNOWN_REUSE,
    }
}

pub const fn jade_policy() -> ProviderAccessPolicy {
    ProviderAccessPolicy {
        provider_ref: "JADE",
        policy_source_ref: "jade:optional-specialist-provider",
        reuse_policy_source_ref: "jade:source-specific-reuse-policy-unresolved",
        published_rate_limit: PublishedRateLimit::Unknown,
        self_imposed_minimum_interval_seconds: SELF_IMPOSED_MINIMUM_INTERVAL_SECONDS,
        burst_limit: SELF_IMPOSED_BURST_LIMIT,
        network_mode: ProviderNetworkMode::NotConfigured,
        cache_first: true,
        per_residual_network_preferred: false,
        reuse: UNKNOWN_REUSE,
    }
}

pub const fn effective_minimum_interval_seconds(policy: ProviderAccessPolicy) -> u64 {
    match policy.published_rate_limit {
        PublishedRateLimit::Unknown => policy.self_imposed_minimum_interval_seconds,
        PublishedRateLimit::MinimumIntervalSeconds(provider_seconds) => {
            if provider_seconds > policy.self_imposed_minimum_interval_seconds {
                provider_seconds
            } else {
                policy.self_imposed_minimum_interval_seconds
            }
        }
    }
}

pub const fn may_schedule_bounded_live(policy: ProviderAccessPolicy) -> bool {
    matches!(policy.network_mode, ProviderNetworkMode::BoundedLive)
}

pub const fn missing_published_numeric_limit_means_unlimited() -> bool { false }
pub const fn provider_permission_implies_semantic_authority() -> bool { false }
pub const fn reuse_permission_implies_current_authority() -> bool { false }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hca_and_fca_keep_four_second_floor_when_no_numeric_limit_is_published() {
        for policy in [high_court_australia_policy(), federal_court_australia_policy()] {
            assert_eq!(policy.published_rate_limit, PublishedRateLimit::Unknown);
            assert_eq!(effective_minimum_interval_seconds(policy), 4);
            assert_eq!(policy.burst_limit, 1);
            assert!(policy.cache_first);
            assert!(may_schedule_bounded_live(policy));
        }
    }

    #[test]
    fn stricter_provider_minimum_wins_over_our_floor() {
        let mut policy = high_court_australia_policy();
        policy.published_rate_limit = PublishedRateLimit::MinimumIntervalSeconds(10);
        assert_eq!(effective_minimum_interval_seconds(policy), 10);
    }

    #[test]
    fn weaker_provider_minimum_never_relaxes_our_floor() {
        let mut policy = high_court_australia_policy();
        policy.published_rate_limit = PublishedRateLimit::MinimumIntervalSeconds(1);
        assert_eq!(effective_minimum_interval_seconds(policy), 4);
    }

    #[test]
    fn hca_and_fca_reuse_obligations_are_retained_separately_from_rate_policy() {
        let hca = high_court_australia_policy();
        assert!(hca.reuse.attribution_required);
        assert!(hca.reuse.original_source_url_required);
        assert!(hca.reuse.accuracy_or_unaltered_copy_required);
        assert!(hca.reuse.non_misleading_use_required);
        assert!(hca.reuse.third_party_rights_may_apply);

        let fca = federal_court_australia_policy();
        assert!(fca.reuse.attribution_required);
        assert!(fca.reuse.original_source_url_required);
        assert!(fca.reuse.accuracy_or_unaltered_copy_required);
        assert!(fca.reuse.third_party_rights_may_apply);
    }

    #[test]
    fn oalc_is_bulk_snapshot_local_first_not_per_residual_network() {
        let policy = oalc_policy();
        assert_eq!(policy.network_mode, ProviderNetworkMode::BulkSnapshotLocalFirst);
        assert!(!policy.per_residual_network_preferred);
        assert!(!may_schedule_bounded_live(policy));
    }

    #[test]
    fn austlii_and_jade_are_not_default_live_lanes() {
        assert_eq!(austlii_policy().network_mode, ProviderNetworkMode::AuthorisationRequired);
        assert_eq!(jade_policy().network_mode, ProviderNetworkMode::NotConfigured);
        assert!(!may_schedule_bounded_live(austlii_policy()));
        assert!(!may_schedule_bounded_live(jade_policy()));
    }

    #[test]
    fn access_and_reuse_policy_never_manufacture_legal_semantics() {
        assert!(!missing_published_numeric_limit_means_unlimited());
        assert!(!provider_permission_implies_semantic_authority());
        assert!(!reuse_permission_implies_current_authority());
    }
}
