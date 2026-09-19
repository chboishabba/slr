#!/usr/bin/env python3
"""Return one exact OALC legislation row from a revision-pinned stream.

This helper is an acquisition fallback for an incomplete Hugging Face Dataset
Viewer index. It deliberately uses ``streaming=True`` and an immutable revision
so it never requires an operator-supplied local ``corpus.jsonl``. A full stream
with no exact match exits as a source residual; it never emits a fabricated
negative legal result.
"""
from __future__ import annotations

import argparse
import json
import sys


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dataset-id", required=True)
    parser.add_argument("--config", required=True)
    parser.add_argument("--split", required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--citation", required=True)
    args = parser.parse_args()

    try:
        from datasets import load_dataset
    except ImportError:
        print(
            "SOURCE_RESIDUAL: revision-pinned streaming fallback requires the "
            "Hugging Face 'datasets' package",
            file=sys.stderr,
        )
        return 2

    try:
        rows = load_dataset(
            args.dataset_id,
            name=args.config,
            split=args.split,
            revision=args.revision,
            streaming=True,
        )
        matches = []
        for row in rows:
            if (
                row.get("citation") == args.citation
                and row.get("source") == "nsw_legislation"
                and row.get("jurisdiction") == "new_south_wales"
                and row.get("type") == "primary_legislation"
            ):
                matches.append(row)
                if len(matches) > 1:
                    print(
                        "SOURCE_RESIDUAL: revision-pinned stream returned "
                        "multiple exact legislation rows",
                        file=sys.stderr,
                    )
                    return 4
    except Exception as exc:  # provider failure remains an acquisition residual
        print(f"SOURCE_RESIDUAL: revision-pinned streaming fallback failed: {exc}", file=sys.stderr)
        return 2

    if not matches:
        print(
            "SOURCE_RESIDUAL: revision-pinned streaming completed with no exact "
            "legislation row",
            file=sys.stderr,
        )
        return 3

    json.dump(matches[0], sys.stdout, ensure_ascii=False)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
