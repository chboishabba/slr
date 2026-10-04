#!/usr/bin/env python3
"""Digital-ESD full-text index — fail-closed P0-G gate.

A record is eligible for P0-G only after an authoritative screening decision
of include|probable.  Verification requires a real retrieved artifact and a
matching SHA-256 supplied by a retrieval manifest.

Missing artifacts remain pending.  No record-id or metadata fallback is ever
treated as full text.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


RETAINED = {"include", "probable"}


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as fh:
        return [dict(row) for row in csv.DictReader(fh, delimiter="\t")]


def read_jsonl(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as fh:
        for n, line in enumerate(fh, 1):
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{n}: expected JSON object")
            rows.append(row)
    return rows


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


class DigitalESDFulltextIndexer:
    def __init__(
        self,
        ledger_path: Path,
        output_dir: Path,
        retrieved_manifest_path: Path | None = None,
    ) -> None:
        self.ledger_path = ledger_path
        self.output_dir = output_dir
        self.retrieved_manifest_path = retrieved_manifest_path
        self.output_dir.mkdir(parents=True, exist_ok=True)

    def load_ledger(self) -> list[dict[str, str]]:
        return read_tsv(self.ledger_path)

    def load_retrieved(self) -> dict[str, dict[str, Any]]:
        rows = read_jsonl(self.retrieved_manifest_path)
        out: dict[str, dict[str, Any]] = {}
        for row in rows:
            ref = str(row.get("source_identity_reference") or "")
            if not ref:
                raise ValueError("retrieved manifest row lacks source_identity_reference")
            if ref in out:
                raise ValueError(f"duplicate retrieved manifest entry for {ref}")
            out[ref] = row
        return out

    def run(self) -> dict[str, Any]:
        ledger = self.load_ledger()
        retrieved = self.load_retrieved()

        ledger_by_ref = {
            str(row.get("source_identity_reference") or ""): row
            for row in ledger
        }
        eligible = {
            ref: row
            for ref, row in ledger_by_ref.items()
            if str(row.get("decision") or "") in RETAINED
        }

        extra = sorted(set(retrieved) - set(eligible))
        if extra:
            raise ValueError(
                "retrieved artifacts supplied for non-retained records: "
                + ", ".join(extra[:20])
            )

        index_rows: list[dict[str, Any]] = []
        verified = 0
        failed = 0
        pending = 0

        for ref, row in sorted(eligible.items()):
            item = retrieved.get(ref)
            if item is None:
                pending += 1
                index_rows.append({
                    "source_identity_reference": ref,
                    "decision_reference": row.get("decision_reference", ""),
                    "status": "pending",
                    "artifact_path": "",
                    "expected_sha256": "",
                    "observed_sha256": "",
                    "retrieval_reference": "",
                    "creates_source_truth": False,
                    "creates_source_audit_admission": False,
                })
                continue

            artifact = Path(str(item.get("artifact_path") or ""))
            expected = str(item.get("sha256") or "").lower().removeprefix("sha256:")
            retrieval_ref = str(item.get("retrieval_reference") or "")
            if len(expected) != 64:
                raise ValueError(f"{ref}: invalid expected sha256")
            try:
                int(expected, 16)
            except ValueError as exc:
                raise ValueError(f"{ref}: invalid expected sha256") from exc

            if not artifact.exists() or not artifact.is_file():
                failed += 1
                index_rows.append({
                    "source_identity_reference": ref,
                    "decision_reference": row.get("decision_reference", ""),
                    "status": "failed-missing-artifact",
                    "artifact_path": str(artifact),
                    "expected_sha256": expected,
                    "observed_sha256": "",
                    "retrieval_reference": retrieval_ref,
                    "creates_source_truth": False,
                    "creates_source_audit_admission": False,
                })
                continue

            observed = sha256_file(artifact)
            if observed != expected:
                failed += 1
                status = "failed-digest-mismatch"
            else:
                verified += 1
                status = "verified"

            index_rows.append({
                "source_identity_reference": ref,
                "decision_reference": row.get("decision_reference", ""),
                "status": status,
                "artifact_path": str(artifact),
                "expected_sha256": expected,
                "observed_sha256": observed,
                "retrieval_reference": retrieval_ref,
                "artifact_size_bytes": artifact.stat().st_size,
                "creates_source_truth": False,
                "creates_source_audit_admission": False,
            })

        index_path = self.output_dir / "digital_esd_fulltext_index.tsv"
        fields = [
            "source_identity_reference",
            "decision_reference",
            "status",
            "artifact_path",
            "expected_sha256",
            "observed_sha256",
            "retrieval_reference",
            "artifact_size_bytes",
            "creates_source_truth",
            "creates_source_audit_admission",
        ]
        with index_path.open("w", newline="", encoding="utf-8") as fh:
            writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t", extrasaction="ignore")
            writer.writeheader()
            for row in index_rows:
                writer.writerow(row)

        worklist_rows = [
            {
                "source_identity_reference": ref,
                "decision": row.get("decision", ""),
                "decision_reference": row.get("decision_reference", ""),
                "metadata_revision_reference": row.get("metadata_revision_reference", ""),
                "candidate_only": True,
                "creates_source_truth": False,
                "creates_source_audit_admission": False,
            }
            for ref, row in sorted(eligible.items())
        ]
        worklist_path = self.output_dir / "fulltext-worklist.jsonl"
        write_jsonl(worklist_path, worklist_rows)

        result = {
            "schema": "sensiblaw.digital-esd-fulltext-index.v0_3",
            "completed_at": now_iso(),
            "input_screening_records": len(ledger),
            "eligible_for_fulltext": len(eligible),
            "retrieved": len(retrieved),
            "verified": verified,
            "failed": failed,
            "pending": pending,
            "index_reference": str(index_path),
            "worklist_reference": str(worklist_path),
            "worklist_count": len(worklist_rows),
            "fulltext_retrieval_creates_source_truth": False,
            "fulltext_retrieval_creates_source_audit_admission": False,
        }
        (self.output_dir / "digital_esd_fulltext_gate.json").write_text(
            json.dumps(result, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        return result


def main() -> int:
    parser = argparse.ArgumentParser(description="Digital-ESD fail-closed full-text gate")
    parser.add_argument("--ledger", type=Path, required=True)
    parser.add_argument("--retrieved-manifest", type=Path)
    parser.add_argument("--output-dir", type=Path, default=Path("artifacts/digital-esd/fulltext"))
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    if not args.ledger.exists():
        raise SystemExit(f"ledger not found: {args.ledger}")

    result = DigitalESDFulltextIndexer(
        ledger_path=args.ledger,
        output_dir=args.output_dir,
        retrieved_manifest_path=args.retrieved_manifest,
    ).run()
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["failed"] == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
