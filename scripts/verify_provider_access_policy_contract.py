#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
POLICY = ROOT / "crates/sl-proof-search-loop/src/provider_access_policy.rs"
text = POLICY.read_text(encoding="utf-8")

required = [
    "SELF_IMPOSED_MINIMUM_INTERVAL_SECONDS: u64 = 4",
    "SELF_IMPOSED_BURST_LIMIT: u32 = 1",
    "PublishedRateLimit::Unknown",
    "ProviderNetworkMode::BoundedLive",
    "ProviderNetworkMode::BulkSnapshotLocalFirst",
    "ProviderNetworkMode::AuthorisationRequired",
    "ProviderNetworkMode::NotConfigured",
    "ReuseObligations",
    'provider_ref: "HighCourtAustralia"',
    'policy_source_ref: "https://www.hcourt.gov.au/terms-use"',
    'reuse_policy_source_ref: "https://www.hcourt.gov.au/terms-use"',
    'provider_ref: "FederalCourtAustralia"',
    'policy_source_ref: "https://www.fedcourt.gov.au/robots.txt"',
    'reuse_policy_source_ref: "https://www.fedcourt.gov.au/copyright"',
    'provider_ref: "OALC"',
    'provider_ref: "AustLII"',
    'provider_ref: "JADE"',
    "effective_minimum_interval_seconds",
    "missing_published_numeric_limit_means_unlimited() -> bool { false }",
    "provider_permission_implies_semantic_authority() -> bool { false }",
    "reuse_permission_implies_current_authority() -> bool { false }",
    "attribution_required: true",
    "original_source_url_required: true",
    "accuracy_or_unaltered_copy_required: true",
    "third_party_rights_may_apply: true",
]
missing = [needle for needle in required if needle not in text]
if missing:
    raise SystemExit(f"provider access policy contract missing: {missing}")

if "per_residual_network_preferred: false" not in text:
    raise SystemExit("provider policy lost local/cache-first research discipline")

for forbidden in ("reqwest", "ureq", "TcpStream", "std::net", "sleep("):
    if forbidden in text:
        raise SystemExit(f"policy ABI unexpectedly gained transport behavior: {forbidden}")

print("provider access policy contract PASS")
