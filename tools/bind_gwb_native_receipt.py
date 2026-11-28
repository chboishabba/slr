#!/usr/bin/env python3
"""Bind a native JMD v1 receipt to its explicit external edition coordinate.
Transport only: verify pinned files, never reissue or extend a kernel proof.
"""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "artifacts/gwb/multiview-native"

def load(path):
    return json.loads(path.read_bytes())

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def require(condition, message):
    if not condition:
        raise ValueError(message)

def bind(edition, receipt, manifest_path=BASE / "native-acquisition-manifest-v1.json"):
    manifest = load(manifest_path)
    entries = [s for s in manifest["sources"] if s["edition"] == edition]
    require(len(entries) == 1 and edition in ("en", "simple", "es"), "edition missing or ambiguous")
    s = entries[0]
    a = BASE / "acquired"
    raw = load(receipt)
    require(raw["schema"] == "slr.jmd-indexed-world-fibre-receipt.v1", "expected original native v1 receipt")
    require(raw["qid"] == manifest["qid"] == "Q207", "qid mismatch")
    require(raw["source_observation_checked"] is True and raw["cross_source_alignment_paid"] is False
            and raw["world_truth_paid"] is False, "unexpected proof scope")
    rev, digest = s["source_revision_ref"], s["canonical_sha256"]
    unit = s["source_ref"]
    prefix = f"source:wikipedia:{edition}:{raw['title'].replace(' ', '_')}:revid:"
    require(unit.startswith(prefix) and unit[len(prefix):].isdigit(), "external source coordinate mismatch")
    ext = int(unit[len(prefix):])
    require(ext > 0 and raw["inventory"]["article_revision"] == rev, "revision mismatch")
    require(raw["source_sha256"] == digest == sha(a / f"{edition}-canonical.txt"), "canonical digest mismatch")
    source = a / f"{edition}-slr-record.json"
    projection = a / f"{edition}-article-candidates.json"
    require(sha(source) == raw["original_slr_sha256"], "original SLR digest mismatch")
    require(sha(projection) == raw["projection_sha256"], "projection digest mismatch")
    require(sha(a / "Q207-EntityData.json") == raw["entity_sha256"], "EntityData digest mismatch")
    record = load(source)
    m = record["manifestation"]
    require(record["revision_ref"] == m["revision_ref"] == rev
            and record["source_unit_ref"] == m["source_unit_ref"] == unit
            and m["language"] == edition and m["source_text_sha256"] == digest
            and m["document_ref"] == s["document_ref"] == f"document:sha256:{digest}", "original source coordinate mismatch")
    return dict(raw, schema="slr.jmd-indexed-world-fibre-receipt.v3", edition=edition,
                site=f"{edition}wiki", external_revision=ext, native_revision=rev,
                source_unit_ref=unit, document_ref=s["document_ref"],
                original_receipt_sha256=sha(receipt), binding_creates_semantic_authority=False)

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--edition", required=True)
    p.add_argument("--receipt", required=True, type=Path)
    p.add_argument("--output", required=True, type=Path)
    args = p.parse_args()
    result = bind(args.edition, args.receipt)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    tmp = args.output.with_suffix(args.output.suffix + ".tmp")
    tmp.write_text(json.dumps(result, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
    tmp.replace(args.output)

if __name__ == "__main__":
    main()
