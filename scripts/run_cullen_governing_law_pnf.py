#!/usr/bin/env python3
"""Run the existing spaCy -> SensibLaw PNF stream over verified Cullen governing-law text.

The harness is deliberately fail-closed. It does not fetch legislation and it does not
substitute a current consolidation for the point-in-time source. Each manifest row must
already have a verified local text file for the requested 2017-01-26 version.

For every source row:
  verified local statutory text
    -> python/spacy_stream.py
    -> target/debug/sensiblaw-stream
    -> retained parser TSV + SLR/PNF output + stderr metrics

This produces observation/candidate evidence only. It does not admit legal atoms, assign
balanced-ternary gates, create authority, or promote results.
"""
from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
from pathlib import Path


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--manifest",
        default="fixtures/cullen_governing_law_parser_manifest_v0_1.tsv",
    )
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--rust-bin", default="target/debug/sensiblaw-stream")
    ap.add_argument("--out-dir", default="target/cullen_governing_law_pnf")
    ns = ap.parse_args()

    root = Path(__file__).resolve().parents[1]
    manifest = root / ns.manifest
    rust_bin = root / ns.rust_bin
    out_dir = root / ns.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    if not manifest.exists():
        print(f"missing parser manifest: {manifest}", file=sys.stderr)
        return 2
    if not rust_bin.exists():
        print(
            f"missing {rust_bin}; build with cargo build -p sensiblaw-stream",
            file=sys.stderr,
        )
        return 2

    rows = list(csv.DictReader(manifest.read_text(encoding="utf-8").splitlines(), delimiter="\t"))
    if not rows:
        print("empty parser manifest", file=sys.stderr)
        return 2

    summary = []
    for row in rows:
        key = row["source_key"]
        text_path = root / row["local_text_path"]
        if row["temporal_status"] != "verified_point_in_time_source":
            print(
                f"BLOCKED {key}: temporal_status={row['temporal_status']} "
                f"for point-in-time {row['point_in_time']}",
                file=sys.stderr,
            )
            return 4
        if not text_path.exists():
            print(
                f"BLOCKED {key}: missing verified point-in-time source text {text_path}",
                file=sys.stderr,
            )
            return 4
        if not text_path.read_text(encoding="utf-8").strip():
            print(f"BLOCKED {key}: empty source text {text_path}", file=sys.stderr)
            return 4

        revision_id = row["parser_revision_id"]
        spacy_cmd = [
            sys.executable,
            str(root / "python/spacy_stream.py"),
            "--model",
            ns.model,
            "--revision-id",
            revision_id,
            str(text_path),
        ]
        spacy = subprocess.run(spacy_cmd, capture_output=True, text=True, check=False)
        if spacy.returncode != 0:
            print(spacy.stderr, file=sys.stderr)
            return spacy.returncode

        parser_tsv = out_dir / f"{key}.spacy.tsv"
        parser_tsv.write_text(spacy.stdout, encoding="utf-8")

        slr = subprocess.run(
            [str(rust_bin)],
            input=spacy.stdout,
            capture_output=True,
            text=True,
            check=False,
        )
        if slr.returncode != 0:
            print(slr.stderr, file=sys.stderr)
            return slr.returncode

        (out_dir / f"{key}.pnf.stdout").write_text(slr.stdout, encoding="utf-8")
        (out_dir / f"{key}.pnf.stderr").write_text(slr.stderr, encoding="utf-8")
        (out_dir / f"{key}.spacy.stderr").write_text(spacy.stderr, encoding="utf-8")

        summary.append(
            {
                "source_key": key,
                "act_id": row["act_id"],
                "point_in_time": row["point_in_time"],
                "locator": row["locator"],
                "local_text_path": row["local_text_path"],
                "parser_revision_id": int(revision_id),
                "expected_atom_refs": row["expected_atom_refs"].split(";"),
                "parser_tsv": str(parser_tsv.relative_to(root)),
                "pnf_stdout": str((out_dir / f"{key}.pnf.stdout").relative_to(root)),
                "pnf_stderr": str((out_dir / f"{key}.pnf.stderr").relative_to(root)),
                "semantic_authority": "observation_candidate_only",
            }
        )

    summary_path = out_dir / "receipt.json"
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"CULLEN_LAW_PNF sources={len(summary)} receipt={summary_path.relative_to(root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
