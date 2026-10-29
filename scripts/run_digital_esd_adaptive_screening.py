#!/usr/bin/env python3
"""Digital-ESD adaptive screening controller over the real screening artifacts.

Stages:
  P0-A exact unresolved screening universe
  P0-B candidate-only title/abstract assessments
  P0-C candidate publication/report/study-family hypotheses
  P0-D stratified calibration selection
  P0-E reviewed-subset calibration diagnostics
  P0-F non-scalar Pareto review queue
  P0-G fail-closed full-text gate

The controller summarizes persisted artifacts. It does not manufacture screening
or full-text facts. Missing artifacts/stages remain explicitly unpaid.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


STAGES = ("P0-A", "P0-B", "P0-C", "P0-D", "P0-E", "P0-F", "P0-G")


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_tsv(path: Path) -> list[dict[str, str]]:
    if not path.exists():
        return []
    with path.open(newline="", encoding="utf-8") as fh:
        return [dict(row) for row in csv.DictReader(fh, delimiter="	")]


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    out: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as fh:
        for n, line in enumerate(fh, 1):
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{n}: expected object")
            out.append(row)
    return out


class DigitalESDAdaptiveScreening:
    def __init__(self, artifact_dir: Path) -> None:
        self.artifact_dir = artifact_dir

    def _artifact(self, name: str) -> Path:
        return self.artifact_dir / name

    def _stage(self, stage: str, status: str, count: int, path: Path | None, details: dict[str, Any]) -> dict[str, Any]:
        return {
            "stage": stage,
            "status": status,
            "record_count": count,
            "artifact_reference": str(path) if path else None,
            "artifact_sha256": sha256_file(path) if path and path.exists() else None,
            "details": details,
            "creates_screening_decision": False,
            "creates_source_truth": False,
            "creates_source_audit_admission": False,
        }

    def stage_p0a(self) -> dict[str, Any]:
        path = self._artifact("screening_ledger.tsv")
        rows = read_tsv(path)
        allowed = {"include", "probable", "exclude", "unresolved"}
        decision_counts = {
            decision: sum(1 for row in rows if row.get("decision") == decision)
            for decision in sorted(allowed)
        }
        valid = bool(rows) and all(row.get("decision") in allowed for row in rows)
        return self._stage(
            "P0-A",
            "paid" if valid else "missing-or-mutated",
            len(rows),
            path if path.exists() else None,
            {
                "decision_counts": decision_counts,
                "denominator_integrity": valid,
                "all_records_accounted_for": sum(decision_counts.values()) == len(rows),
            },
        )

    def stage_p0b(self) -> dict[str, Any]:
        path = self._artifact("candidate_assessments.jsonl")
        rows = read_jsonl(path)
        non_authoritative = all(
            row.get("candidate_only") is True
            and row.get("creates_screening_decision") is False
            for row in rows
        )
        return self._stage(
            "P0-B",
            "paid" if rows and non_authoritative else "missing-or-invalid",
            len(rows),
            path if path.exists() else None,
            {"candidate_only": non_authoritative},
        )

    def stage_p0c(self) -> dict[str, Any]:
        path = self._artifact("study_family_hypotheses.jsonl")
        rows = read_jsonl(path)
        non_authoritative = all(
            row.get("candidate_only") is True
            and row.get("creates_same_empirical_study") is False
            for row in rows
        )
        return self._stage(
            "P0-C",
            "paid" if path.exists() and non_authoritative else "missing-or-invalid",
            len(rows),
            path if path.exists() else None,
            {"hypothesis_only": non_authoritative},
        )

    def stage_p0d(self) -> dict[str, Any]:
        path = self._artifact("calibration_selection.jsonl")
        rows = read_jsonl(path)
        non_authoritative = all(
            row.get("selection_creates_decision") is False for row in rows
        )
        return self._stage(
            "P0-D",
            "paid" if path.exists() and non_authoritative else "missing-or-invalid",
            len(rows),
            path if path.exists() else None,
            {"selection_creates_decision": False if non_authoritative else None},
        )

    def stage_p0e(self) -> dict[str, Any]:
        path = self._artifact("calibration_estimate.json")
        if not path.exists():
            return self._stage("P0-E", "missing", 0, None, {})
        payload = json.loads(path.read_text(encoding="utf-8"))
        reviewed = int(payload.get("reviewed_pair_count") or 0)
        status = "paid-diagnostic" if reviewed > 0 else "awaiting-reviewed-calibration"
        return self._stage(
            "P0-E",
            status,
            reviewed,
            path,
            {
                "candidate_false_negative_proxy": payload.get("candidate_false_negative_proxy"),
                "candidate_review_disagreement_rate": payload.get("candidate_review_disagreement_rate"),
                "estimate_creates_population_truth": payload.get("estimate_creates_population_truth", False),
            },
        )

    def stage_p0f(self) -> dict[str, Any]:
        path = self._artifact("screening_pareto_queue.jsonl")
        rows = read_jsonl(path)
        front = sum(1 for row in rows if row.get("pareto_front") is True)
        valid = all(
            row.get("selection_creates_screening_decision") is False
            and row.get("selection_creates_exclusion") is False
            for row in rows
        )
        return self._stage(
            "P0-F",
            "paid" if rows and valid else "missing-or-invalid",
            len(rows),
            path if path.exists() else None,
            {"pareto_front_count": front, "scalar_score_used": False},
        )

    def stage_p0g(self) -> dict[str, Any]:
        path = self.artifact_dir / "fulltext" / "digital_esd_fulltext_gate.json"
        if not path.exists():
            return self._stage(
                "P0-G",
                "awaiting-authoritative-decisions-and-fulltext",
                0,
                None,
                {"verified": 0, "pending": 0, "failed": 0},
            )
        payload = json.loads(path.read_text(encoding="utf-8"))
        verified = int(payload.get("verified") or 0)
        pending = int(payload.get("pending") or 0)
        failed = int(payload.get("failed") or 0)
        eligible = int(payload.get("eligible_for_fulltext") or 0)
        if failed:
            status = "failed"
        elif eligible == 0:
            status = "awaiting-authoritative-decisions"
        elif pending:
            status = "in-progress"
        else:
            status = "paid"
        return self._stage(
            "P0-G",
            status,
            eligible,
            path,
            {
                "retrieved": int(payload.get("retrieved") or 0),
                "verified": verified,
                "pending": pending,
                "failed": failed,
            },
        )

    def run_all(self) -> dict[str, Any]:
        stages = [
            self.stage_p0a(),
            self.stage_p0b(),
            self.stage_p0c(),
            self.stage_p0d(),
            self.stage_p0e(),
            self.stage_p0f(),
            self.stage_p0g(),
        ]
        result = {
            "schema": "sensiblaw.digital-esd-adaptive-screening.v0_2",
            "generated_at": now_iso(),
            "artifact_dir": str(self.artifact_dir),
            "stages": stages,
            "all_pre_fulltext_stages_materialised": all(
                stage["status"] not in {"missing", "missing-or-invalid", "missing-or-mutated"}
                for stage in stages[:6]
            ),
            "p0g_paid": stages[6]["status"] == "paid",
        }
        manifest = self.artifact_dir / "adaptive_screening_manifest.json"
        manifest.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return result


def main() -> int:
    parser = argparse.ArgumentParser(description="Summarize real Digital-ESD P0-A..G artifacts")
    parser.add_argument(
        "--artifact-dir",
        type=Path,
        default=Path("artifacts/digital-esd/real-eric"),
    )
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    args.artifact_dir.mkdir(parents=True, exist_ok=True)
    result = DigitalESDAdaptiveScreening(args.artifact_dir).run_all()
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        for stage in result["stages"]:
            print(f"{stage['stage']} {stage['status']} count={stage['record_count']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
