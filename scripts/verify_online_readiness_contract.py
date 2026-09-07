#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "crates" / "sl-proof-search-loop" / "src"
text = "\n".join(
    (BASE / name).read_text(encoding="utf-8")
    for name in ["online.rs", "provider.rs"]
)

required = [
    "pub enum OnlineReadiness",
    "ExperimentalLiveAcquisitionReady",
    "ProductionGovernedOnlineReady",
    "LiveProviderAdapterMissing",
    "LiveProviderFixtureMissing",
    "LiveReceiptLineageMissing",
    "ExactHeadLiveExecutionReceiptMissing",
    "ExactHeadAgdaKernelReceiptMissing",
    "live_receipt_lineage_validated",
    "exact_head_live_execution_receipt",
    "pub trait GovernedLegalProvider",
    "pub struct LegalHostPacingPolicy",
    "requests_per_four_seconds: 1",
    "burst_limit: 1",
    "pub struct SearchBounds",
    "cache_checked_first",
    "persisted_receipts_checked_first",
    "crawling_permitted",
    "ad_hoc_polling_permitted",
    "pub struct SearchReferenceReceipt",
    "pub struct FetchBytesReceipt",
    'receipt_authority != "experimental_candidate_only"',
    "FetchBypassedLocalIngestion",
]
missing = [needle for needle in required if needle not in text]
if missing:
    raise SystemExit(f"online readiness contract missing: {missing}")

for forbidden in [
    "reqwest",
    "ureq",
    "hyper::",
    "TcpStream",
    "std::net",
    "urlopen",
    "curl",
    "publish(",
    "auto_admit",
    "automatic_admission",
]:
    if forbidden in text:
        raise SystemExit(f"online readiness boundary acquired forbidden capability: {forbidden}")

print("governed online readiness contract PASS")
