#!/usr/bin/env python3
"""Deprecated compatibility wrapper for the native SensibLaw Waltons CLI.

Canonical owner:
    cargo run -p sensiblaw-cli --bin sensiblaw -- legal-follow waltons ...

This file intentionally contains no legal, review, acquisition, payment,
WrongType, citation-treatment, or genealogy semantics.
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

ALIASES = {
    "status": ["status"],
    "acquire": ["acquire"],
    "queue": ["materialise"],
    "review-template": ["review", "prepare"],
    "review-finalize": ["review", "finalize"],
    "review-compile": ["review", "compile"],
    "frontier": ["frontier"],
    "citedby-manifest": ["cited-by", "plan"],
    "citedby-oalc": ["cited-by", "acquire"],
    "treatment-queues": ["treatment", "queue"],
    "merge-treatment": ["treatment", "merge"],
    "treatment-template": ["treatment", "prepare"],
    "treatment-finalize": ["treatment", "finalize"],
    "genealogy": ["genealogy"],
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base")
    parser.add_argument("command", choices=sorted([*ALIASES, "citedby-import"]))
    parser.add_argument("extra", nargs="*")
    args = parser.parse_args()

    native = [
        "cargo",
        "run",
        "-p",
        "sensiblaw-cli",
        "--bin",
        "sensiblaw",
    ]
    if args.command in {"acquire", "citedby-oalc"}:
        native.extend(["--features", "live-network"])
    native.extend([
        "--",
        "legal-follow",
        "waltons",
    ])
    if args.base:
        native.extend(["--base", args.base])

    if args.command == "citedby-import":
        if len(args.extra) != 1:
            parser.error("citedby-import requires PROVIDER_RESULTS.json")
        native.extend(["cited-by", "import", args.extra[0]])
    else:
        native.extend(ALIASES[args.command])
        tolerated = args.command == "citedby-oalc" and args.extra == ["--execute"]
        if args.extra and not tolerated:
            parser.error(f"{args.command} does not accept extra arguments")

    print("compatibility wrapper ->", " ".join(native), file=sys.stderr)
    return subprocess.run(native, cwd=ROOT, check=False).returncode


if __name__ == "__main__":
    raise SystemExit(main())
