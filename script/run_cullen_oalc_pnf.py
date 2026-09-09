#!/usr/bin/env python3
"""Materialise OALC Cullen governing legislation and run section slices through spaCy/PNF.

OALC is the operational default corpus. Its NSW legislation documents are typed
`latest_known_only`; this runner therefore does not claim that the materialised
text is the exact 2017-01-26 point-in-time consolidation.

Pipeline:
  local OALC corpus.jsonl
    -> exact two-document materialisation
    -> source-preserving section slices
    -> python/spacy_stream.py
    -> sensiblaw-stream

No parser/PNF output creates legal authority, historical applicability or Atomic
case gates by itself.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import os
from pathlib import Path
import re
import subprocess
import sys

TARGET_SLICES = {
    "civil-liability-act-2002-nsw": ["5A", "5B", "5C", "5D", "43A"],
    "law-reform-vicarious-liability-act-1983-nsw": ["6", "7", "8"],
}

SECTION_HEADING = re.compile(r"(?m)^\s*(\d+[A-Z]?)\s+[^\n]+$")


def sha256(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def run(cmd: list[str], *, env: dict[str, str] | None = None, stdin=None, stdout=None, stderr=None):
    print("+", " ".join(cmd), file=sys.stderr)
    return subprocess.run(cmd, check=True, env=env, stdin=stdin, stdout=stdout, stderr=stderr)


def section_spans(text: str) -> dict[str, tuple[int, int]]:
    matches = list(SECTION_HEADING.finditer(text))
    occurrences: dict[str, list[tuple[int, int]]] = {}
    for i, match in enumerate(matches):
        section = match.group(1)
        start = match.start()
        end = matches[i + 1].start() if i + 1 < len(matches) else len(text)
        occurrences.setdefault(section, []).append((start, end))

    spans: dict[str, tuple[int, int]] = {}
    for section, candidates in occurrences.items():
        if len(candidates) != 1:
            # Ambiguity is preserved as a source/parser residual. Never choose
            # the first matching heading merely because its number fits.
            continue
        spans[section] = candidates[0]
    return spans


def load_receipts(path: Path) -> dict[str, dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as fh:
        return {row["citation"]: row for row in csv.DictReader(fh, delimiter="\t")}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--oalc-jsonl", required=True, help="local OALC corpus.jsonl")
    ap.add_argument("--oalc-revision", required=True, help="pinned OALC revision/tag/digest")
    ap.add_argument("--output-dir", default="artifacts/oalc/cullen-governing-law")
    ap.add_argument("--spacy-model", default="en_core_web_sm")
    ns = ap.parse_args()

    root = Path(__file__).resolve().parents[1]
    output = root / ns.output_dir
    materialised = output / "materialised"
    slices_dir = output / "slices"
    parser_dir = output / "parser"
    pnf_dir = output / "pnf"
    for directory in (materialised, slices_dir, parser_dir, pnf_dir):
        directory.mkdir(parents=True, exist_ok=True)

    env = os.environ.copy()
    env["SENSIBLAW_OALC_JSONL"] = str(Path(ns.oalc_jsonl).resolve())
    env["SENSIBLAW_OALC_REVISION"] = ns.oalc_revision
    env["SENSIBLAW_OALC_OUTPUT"] = str(materialised)

    run(
        [
            "cargo", "run", "-q",
            "-p", "sensiblaw-governed-legal-provider",
            "--example", "oalc_cullen_legislation_materialize",
        ],
        env=env,
    )

    receipt_path = materialised / "oalc_legislation_receipts.tsv"
    receipts = load_receipts(receipt_path)
    by_slug = {
        "civil-liability-act-2002-nsw": "Civil Liability Act 2002 (NSW)",
        "law-reform-vicarious-liability-act-1983-nsw": "Law Reform (Vicarious Liability) Act 1983 (NSW)",
    }

    slice_receipts = []
    revision_id = 1000
    for slug, sections in TARGET_SLICES.items():
        citation = by_slug[slug]
        parent = receipts[citation]
        if parent["temporal_status"] != "latest_known_only":
            raise RuntimeError(f"unexpected OALC temporal status for {citation}")
        source_path = Path(parent["local_artifact_ref"])
        text = source_path.read_text(encoding="utf-8")
        spans = section_spans(text)

        for section in sections:
            if section not in spans:
                raise RuntimeError(
                    f"could not uniquely locate section {section} in OALC text for {citation}; "
                    "preserve as section-boundary residual rather than guessing"
                )
            start, end = spans[section]
            section_text = text[start:end].rstrip() + "\n"
            slice_path = slices_dir / f"{slug}-s{section}.txt"
            slice_path.write_text(section_text, encoding="utf-8")
            digest = sha256(section_text.encode("utf-8"))

            parser_path = parser_dir / f"{slug}-s{section}.tsv"
            parser_err = parser_dir / f"{slug}-s{section}.stderr.txt"
            pnf_path = pnf_dir / f"{slug}-s{section}.receipts.tsv"
            pnf_err = pnf_dir / f"{slug}-s{section}.stderr.txt"

            with parser_path.open("wb") as parser_out, parser_err.open("wb") as parser_stderr:
                run(
                    [
                        sys.executable,
                        str(root / "python/spacy_stream.py"),
                        "--model", ns.spacy_model,
                        "--revision-id", str(revision_id),
                        str(slice_path),
                    ],
                    stdout=parser_out,
                    stderr=parser_stderr,
                )

            with parser_path.open("rb") as parser_in, pnf_path.open("wb") as pnf_out, pnf_err.open("wb") as pnf_stderr:
                run(
                    ["cargo", "run", "-q", "-p", "sensiblaw-stream"],
                    stdin=parser_in,
                    stdout=pnf_out,
                    stderr=pnf_stderr,
                )

            slice_receipts.append({
                "citation": citation,
                "section": section,
                "parent_version_id": parent["version_id"],
                "corpus_revision": parent["corpus_revision"],
                "parent_digest": parent["canonical_text_digest"],
                "slice_start": str(start),
                "slice_end": str(end),
                "slice_digest": digest,
                "slice_artifact": str(slice_path),
                "parser_artifact": str(parser_path),
                "pnf_artifact": str(pnf_path),
                "temporal_status": "latest_known_only",
                "parser_authority": "source_observation_only",
            })
            revision_id += 1

    out_receipts = output / "oalc_section_pnf_receipts.tsv"
    fields = list(slice_receipts[0].keys())
    with out_receipts.open("w", encoding="utf-8", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
        writer.writeheader()
        writer.writerows(slice_receipts)

    print(
        f"parsed {len(slice_receipts)} OALC statutory sections through spaCy/PNF; "
        f"temporal_status=latest_known_only; receipts={out_receipts}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
