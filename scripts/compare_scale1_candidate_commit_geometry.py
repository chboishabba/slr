#!/usr/bin/env python3
"""Compare cold SCALE-1 candidate commit-economy receipts.

This tool is intentionally receipt-only. It does not mutate PostgreSQL and it
will refuse to compare runs whose semantic/integrity coordinates differ.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def load(path: str) -> tuple[str, dict]:
    data = json.loads(Path(path).read_text())
    receipt = data.get("compiler_receipt", data)
    integrity = receipt.get("integrity", receipt)
    performance = receipt.get("performance", receipt)
    source = receipt.get("source", {})
    return path, {
        "source_revision_ref": source.get("source_revision_ref"),
        "content_digest_ref": source.get("content_digest_ref"),
        "canonical_ref": source.get("canonical_ref"),
        "semantic_eligible_regions": integrity.get("semantic_eligible_regions"),
        "persisted_statement_count": integrity.get("persisted_statement_count"),
        "persisted_candidate_batch_count": integrity.get("persisted_candidate_batch_count"),
        "persisted_candidate_factor_count": integrity.get("persisted_candidate_factor_count"),
        "candidate_pnf_reopen_complete": integrity.get("candidate_pnf_reopen_complete"),
        "source_region_loss_count": integrity.get("source_region_loss_count"),
        "creates_semantic_authority": integrity.get("creates_semantic_authority"),
        "applicability_promoted": integrity.get("applicability_promoted"),
        "claim_truth_promoted": integrity.get("claim_truth_promoted"),
        "candidate_persistence_reused": integrity.get("candidate_persistence_reused"),
        "candidate_commit_batch_size": integrity.get("candidate_commit_batch_size"),
        "candidate_commit_count": integrity.get("candidate_commit_count"),
        "candidate_postcommit_reopen_query_count": integrity.get("candidate_postcommit_reopen_query_count"),
        "candidate_persist_ns": performance.get("candidate_persist_ns"),
        "candidate_precommit_write_ns": performance.get("candidate_precommit_write_ns"),
        "candidate_commit_wait_ns": performance.get("candidate_commit_wait_ns"),
        "candidate_postcommit_reopen_ns": performance.get("candidate_postcommit_reopen_ns"),
        "candidate_mean_commit_wait_ns": performance.get("candidate_mean_commit_wait_ns"),
    }


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main(argv: list[str]) -> int:
    require(len(argv) >= 3, "usage: compare_scale1_candidate_commit_geometry.py <receipt-a.json> <receipt-b.json> [...]")
    rows = [load(path) for path in argv[1:]]
    _, baseline = rows[0]

    comparable = [
        "source_revision_ref",
        "content_digest_ref",
        "canonical_ref",
        "semantic_eligible_regions",
        "persisted_statement_count",
        "persisted_candidate_batch_count",
        "persisted_candidate_factor_count",
    ]
    for path, row in rows:
        for key in comparable:
            require(
                row[key] == baseline[key],
                f"{path}: incomparable {key}: {row[key]!r} != {baseline[key]!r}",
            )
        require(row["candidate_persistence_reused"] is False, f"{path}: expected a cold/non-stage-reused candidate persistence run")
        require(row["candidate_pnf_reopen_complete"] is True, f"{path}: candidate reopen incomplete")
        require(row["source_region_loss_count"] == 0, f"{path}: source region loss is nonzero")
        require(row["creates_semantic_authority"] is False, f"{path}: semantic authority promoted")
        require(row["applicability_promoted"] is False, f"{path}: applicability promoted")
        require(row["claim_truth_promoted"] is False, f"{path}: claim truth promoted")
        require((row["candidate_commit_batch_size"] or 0) > 0, f"{path}: invalid commit batch size")
        require((row["candidate_commit_count"] or 0) > 0, f"{path}: invalid commit count")
        reopen_queries = row["candidate_postcommit_reopen_query_count"]
        require(isinstance(reopen_queries, int) and reopen_queries >= 0, f"{path}: invalid reopen query count")
        require(
            reopen_queries <= 3 * row["candidate_commit_count"],
            f"{path}: reopen query geometry is not bounded by 3x commit count",
        )
        for key in (
            "candidate_persist_ns",
            "candidate_precommit_write_ns",
            "candidate_commit_wait_ns",
            "candidate_postcommit_reopen_ns",
        ):
            require(isinstance(row[key], int) and row[key] >= 0, f"{path}: missing/invalid {key}")

    print(
        "\t".join(
            [
                "receipt",
                "batch_size",
                "commit_count",
                "candidate_persist_ns",
                "precommit_write_ns",
                "commit_wait_ns",
                "mean_commit_wait_ns",
                "postcommit_reopen_ns",
                "postcommit_reopen_queries",
                "reopen_queries_per_commit",
                "commit_wait_fraction",
            ]
        )
    )
    for path, row in sorted(rows, key=lambda item: item[1]["candidate_commit_batch_size"]):
        total = row["candidate_persist_ns"]
        commit_wait = row["candidate_commit_wait_ns"]
        fraction = (commit_wait / total) if total else 0.0
        reopen_per_commit = (
            row["candidate_postcommit_reopen_query_count"] / row["candidate_commit_count"]
            if row["candidate_commit_count"]
            else 0.0
        )
        print(
            "\t".join(
                [
                    path,
                    str(row["candidate_commit_batch_size"]),
                    str(row["candidate_commit_count"]),
                    str(total),
                    str(row["candidate_precommit_write_ns"]),
                    str(commit_wait),
                    str(row["candidate_mean_commit_wait_ns"]),
                    str(row["candidate_postcommit_reopen_ns"]),
                    str(row["candidate_postcommit_reopen_query_count"]),
                    f"{reopen_per_commit:.6f}",
                    f"{fraction:.6f}",
                ]
            )
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
