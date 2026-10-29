#!/usr/bin/env python3
"""Deprecated compatibility shim for native revision-pinned OALC streaming.

Canonical owner:
    sensiblaw legal-follow oalc stream-pinned ...

This file validates the historical wrapper arguments and forwards them to the
Rust provider. It contains no corpus iteration, filtering, or legal semantics.
"""
from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPECTED_DATASET = "isaacus/open-australian-legal-corpus"
EXPECTED_CONFIG = "corpus"
EXPECTED_SPLIT = "corpus"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dataset-id", required=True)
    parser.add_argument("--config", required=True)
    parser.add_argument("--split", required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--citation", required=True)
    parser.add_argument("--citation-match", choices=("exact", "contains"), default="exact")
    parser.add_argument("--document-type", default="primary_legislation")
    parser.add_argument("--source", default="nsw_legislation")
    parser.add_argument("--jurisdiction", default="new_south_wales")
    args = parser.parse_args()

    if args.dataset_id != EXPECTED_DATASET:
        parser.error(f"native shim only supports {EXPECTED_DATASET}")
    if args.config != EXPECTED_CONFIG or args.split != EXPECTED_SPLIT:
        parser.error("native shim requires config=corpus split=corpus")

    cmd = [
        "cargo",
        "run",
        "-p",
        "sensiblaw-cli",
        "--bin",
        "sensiblaw",
        "--features",
        "live-network",
        "--",
        "legal-follow",
        "oalc",
        "stream-pinned",
        "--revision",
        args.revision,
        "--citation",
        args.citation,
        "--citation-match",
        args.citation_match,
        "--document-type",
        args.document_type,
        "--source",
        args.source,
        "--jurisdiction",
        args.jurisdiction,
    ]
    return subprocess.run(cmd, cwd=ROOT, check=False).returncode


if __name__ == "__main__":
    raise SystemExit(main())
