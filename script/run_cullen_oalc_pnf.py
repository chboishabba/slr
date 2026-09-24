#!/usr/bin/env python3
"""Deprecated compatibility shim for native Cullen OALC/PNF orchestration.

Canonical owner:
    sensiblaw legal-follow cullen pnf ...

python/spacy_stream.py remains a parser producer. This file owns no LegalFollow,
OALC acquisition, legal section slicing, receipt validation, or PNF semantics.
"""
from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--operator-opt-in", action="store_true")
    parser.add_argument("--output-dir", default="artifacts/oalc/cullen-governing-law")
    parser.add_argument("--spacy-model", default="en_core_web_sm")
    args = parser.parse_args()
    if not args.operator_opt_in:
        parser.error("--operator-opt-in is required")

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
        "cullen",
        "pnf",
        "--operator-opt-in",
        "--output-dir",
        args.output_dir,
        "--spacy-model",
        args.spacy_model,
    ]
    return subprocess.run(cmd, cwd=ROOT, check=False).returncode


if __name__ == "__main__":
    raise SystemExit(main())
