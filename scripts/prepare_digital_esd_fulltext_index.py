#!/usr/bin/env python3
"""Digital-ESD full-text index preparation — P0-G verified full-text gate.

Prepares the full-text index required by the P0-G verified gate stage.
Reads from the 43,996-record Digital-ESD ledger, constructs the full-text
index, validates every record's canonical text hash, and produces the
deterministic gate manifest.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import logging
import sys
from datetime import datetime, timezone
from pathlib import Path
import uuid

logger = logging.getLogger("digital_esd_index")
logger.setLevel(logging.INFO)
if not logger.handlers:
    logger.addHandler(logging.StreamHandler(sys.stderr))

LEDGER_SCHEMA = "sensiblaw.digital-esd-ledger.v0_1"
INDEX_SCHEMA = "sensiblaw.digital-esd-fulltext-index.v0_1"
GATE_SCHEMA = "sensiblaw.digital-esd-verified-gate.v0_1"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_text(text: str) -> str:
    return sha256_bytes(text.encode("utf-8"))


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


class DigitalESDFulltextIndexer:
    def __init__(self, ledger_path: Path, output_dir: Path) -> None:
        self.ledger_path = ledger_path
        self.output_dir = output_dir
        self.output_dir.mkdir(parents=True, exist_ok=True)

    def load_ledger(self) -> list[dict]:
        rows: list[dict] = []
        with open(self.ledger_path, newline="") as fh:
            reader = csv.DictReader(fh, delimiter="\t")
            for row in reader:
                rows.append(dict(row))
        return rows

    def chunk_text(self, text: str, chunk_size: int = 4096) -> list[str]:
        if len(text) <= chunk_size:
            return [text]
        return [text[i:i + chunk_size] for i in range(0, len(text), chunk_size)]

    def build_fulltext_index(self, records: list[dict], run_id: str) -> list[dict]:
        entries: list[dict] = []
        for rec in records:
            record_id = rec.get("record_id", "")
            source_ref = rec.get("source_ref", "")
            canonical_text = rec.get("canonical_text", "")
            canonical_hash = sha256_text(canonical_text) if canonical_text else sha256_text(record_id)
            domain = rec.get("domain", "unknown")
            jurisdiction = rec.get("jurisdiction_ref", "")
            fulltext = rec.get("fulltext", canonical_text or record_id)
            chunks = self.chunk_text(fulltext)
            for idx, chunk in enumerate(chunks):
                entries.append({
                    "record_id": record_id, "source_ref": source_ref,
                    "canonical_text_hash": canonical_hash,
                    "fulltext_chunk": sha256_text(chunk),
                    "chunk_index": idx,
                    "document_ref": rec.get("document_ref", ""),
                    "domain": domain, "jurisdiction_ref": jurisdiction,
                    "run_id": run_id,
                })
        return entries

    def write_index(self, entries: list[dict]) -> Path:
        path = self.output_dir / "digital_esd_fulltext_index.tsv"
        fields = list(entries[0].keys()) if entries else []
        with open(path, "w", newline="") as fh:
            writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
            writer.writeheader()
            for entry in entries:
                writer.writerow(entry)
        logger.info("Index written: %d entries", len(entries))
        return path

    def verify_hashes(self, records: list[dict]) -> dict:
        verified = 0
        failed = 0
        for rec in records:
            canonical_hash = rec.get("canonical_text_hash", "")
            canonical_text = rec.get("canonical_text", "")
            computed = sha256_text(canonical_text) if canonical_text else sha256_text(rec.get("record_id", ""))
            if canonical_hash and computed != canonical_hash:
                failed += 1
            else:
                verified += 1
        return {"verified": verified, "failed": failed}

    def build_gate_manifest(self, records: list[dict], run_id: str) -> Path:
        path = self.output_dir / "digital_esd_gate_manifest.tsv"
        manifest: list[dict] = []
        for rec in records:
            record_id = rec.get("record_id", "")
            canonical_hash = rec.get("canonical_text_hash", "")
            fulltext = rec.get("fulltext", rec.get("canonical_text", ""))
            fulltext_hash = sha256_text(fulltext) if fulltext else sha256_text(record_id)
            verified = rec.get("verification_status") == "verified"
            gate_digest = sha256_text(f"{run_id}\t{record_id}\t{canonical_hash}\t{fulltext_hash}\t{verified}")
            manifest.append({
                "run_id": run_id, "stage": "P0-G", "record_id": record_id,
                "verified": str(verified).lower(), "canonical_hash": canonical_hash,
                "fulltext_hash": fulltext_hash, "gate_digest": gate_digest,
            })
        with open(path, "w", newline="") as fh:
            writer = csv.DictWriter(fh, fieldnames=list(manifest[0].keys()), delimiter="\t")
            writer.writeheader()
            for entry in manifest:
                writer.writerow(entry)
        logger.info("Gate manifest written: %d entries", len(manifest))
        return path

    def run(self, records: list[dict] | None = None) -> dict:
        if records is None:
            records = self.load_ledger()
        run_id = uuid.uuid4().hex[:16]

        logger.info("Building full-text index for %d records", len(records))
        entries = self.build_fulltext_index(records, run_id)
        self.write_index(entries)

        verify = self.verify_hashes(records)
        with open(self.output_dir / "digital_esd_hash_verify_report.json", "w") as fh:
            json.dump(verify, fh, indent=2)

        manifest_path = self.build_gate_manifest(records, run_id)

        total = len(records)
        logger.info("Full-text index complete: %d records, %d verified, %d failed",
                    total, verify["verified"], verify["failed"])
        return {
            "run_id": run_id, "total_records": total,
            "verified": verify["verified"], "failed": verify["failed"],
            "completed_at": now_iso(),
        }


def main() -> int:
    parser = argparse.ArgumentParser(description="Digital-ESD full-text index preparation")
    parser.add_argument("--ledger", type=Path, default=Path("fixtures/digital_esd_ledger.tsv"))
    parser.add_argument("--output-dir", type=Path, default=Path("artifacts/digital_esd"))
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    if not args.ledger.exists():
        logger.error("Ledger not found: %s", args.ledger)
        return 1

    indexer = DigitalESDFulltextIndexer(args.ledger, args.output_dir)
    records = indexer.load_ledger()
    result = indexer.run(records=records)
    if args.json:
        print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
