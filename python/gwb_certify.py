#!/usr/bin/env python3
"""Strict user-facing GWB v0.1 certification command.

Unlike the lower-level runner this command exposes no document-limit option. A successful
exit therefore always means the complete prepared projection manifest was consumed and
the resulting canonical full-run receipt passed its declared invariants.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gwb_full_run import FULL_RUN_SCHEMA, run
from gwb_tranche import PROFILE_REF


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", required=True)
    ap.add_argument("--rust-bin", default="target/release/sensiblaw-stream")
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--output", required=True)
    ap.add_argument("--max-parity-diagnostics", type=int, default=20)
    args = ap.parse_args()

    ns = SimpleNamespace(
        manifest=args.manifest,
        rust_bin=args.rust_bin,
        model=args.model,
        output=args.output,
        limit=None,
        max_parity_diagnostics=args.max_parity_diagnostics,
    )
    rc = run(ns)
    receipt_path = Path(args.output).resolve()
    if not receipt_path.exists():
        raise SystemExit("GWB certification produced no receipt")
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    manifest = json.loads(Path(args.manifest).resolve().read_text(encoding="utf-8"))

    expected_documents = int(manifest.get("document_count", -1))
    checks = {
        "schema": receipt.get("schema_version") == FULL_RUN_SCHEMA,
        "profile": receipt.get("profile_ref") == PROFILE_REF,
        "driver": receipt.get("canonical_driver") == "python/gwb_full_run.py",
        "complete_document_count": receipt.get("document_count") == expected_documents,
        "full_gate": receipt.get("invariants", {}).get("full_gate_pass") is True,
        "parity": receipt.get("invariants", {}).get("direct_reference_parity") is True,
        "no_publication": receipt.get("invariants", {}).get("parser_did_not_publish") is True,
        "hashes_preverified": receipt.get("invariants", {}).get("all_projected_text_hashes_verified_before_timing") is True,
    }
    failed = [name for name, ok in checks.items() if not ok]
    if rc != 0 or failed:
        raise SystemExit(f"GWB full certification failed: rc={rc} failed_checks={failed}")
    print(f"GWB_CERTIFIED receipt={receipt_path} documents={expected_documents}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
