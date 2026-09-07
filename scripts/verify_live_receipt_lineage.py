#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

SCHEMA = "sl.governed_legal_receipt_lineage.v0_1"
AUTHORITY = "experimental_candidate_only"

# A prior bounded live receipt may calibrate a later locally-validated head only
# when the actual governed provider implementation is unchanged.  Receipt-format,
# lockfile and downstream handoff-validation repairs may differ, but this script
# never treats that as byte-for-byte build identity or exact-head live execution.
ALLOWED_CHANGED_PATHS = {
    "Cargo.lock",
    "crates/sl-governed-legal-provider/examples/live_austlii_smoke.rs",
    "crates/sl-proof-search-loop/src/acquisition.rs",
    "scripts/verify_official_acquisition_handoff_contract.py",
}
PROVIDER_RUNTIME_PATHS = {
    "crates/sl-governed-legal-provider/src/lib.rs",
    "crates/sl-governed-legal-provider/Cargo.toml",
}


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt-head", required=True)
    parser.add_argument("--validated-head", default="HEAD")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    receipt_head = git("rev-parse", args.receipt_head)
    validated_head = git("rev-parse", args.validated_head)
    changed = [
        line for line in git("diff", "--name-only", f"{receipt_head}..{validated_head}").splitlines()
        if line
    ]
    unexpected = sorted(set(changed) - ALLOWED_CHANGED_PATHS)
    provider_runtime_changed = sorted(set(changed) & PROVIDER_RUNTIME_PATHS)

    if unexpected:
        raise SystemExit(f"unexpected post-receipt source changes: {unexpected}")
    if provider_runtime_changed:
        raise SystemExit(f"governed provider runtime changed since receipt: {provider_runtime_changed}")

    receipt = {
        "schema_version": SCHEMA,
        "authority": AUTHORITY,
        "receipt_head": receipt_head,
        "validated_head": validated_head,
        "changed_paths": changed,
        "provider_runtime_source_unchanged": True,
        "exact_head_live_execution_claimed": False,
        "byte_for_byte_build_identity_claimed": False,
        "semantic_or_legal_authority_claimed": False,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(
        "live receipt lineage PASS "
        f"receipt_head={receipt_head} validated_head={validated_head} "
        f"changed_paths={len(changed)} provider_runtime_source_unchanged=true"
    )


if __name__ == "__main__":
    main()
