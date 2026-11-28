#!/usr/bin/env python3
"""Acquire one revision-pinned Abstract Wikipedia constructor.

Abstract Wikipedia is a structured constructor representation, not prose.  The
receipt therefore records the exact API revision and constructor bytes but does
not claim a spaCy/parser or prose-world interpretation.
"""
import argparse, hashlib, json, urllib.parse, urllib.request
from pathlib import Path

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--qid", default="Q207")
    ap.add_argument("--output-dir", required=True)
    args = ap.parse_args()
    query = urllib.parse.urlencode({
        "action": "query", "prop": "revisions", "rvprop": "ids|timestamp|content",
        "rvslots": "main", "format": "json", "titles": args.qid,
    })
    url = "https://abstract.wikipedia.org/w/api.php?" + query
    req = urllib.request.Request(url, headers={
        "User-Agent": "DASHI-SLR-abstract-acquisition/1.0",
        "Accept": "application/json",
    })
    payload = json.loads(urllib.request.urlopen(req, timeout=30).read())
    pages = payload.get("query", {}).get("pages", {})
    page = next(iter(pages.values()))
    rev = page["revisions"][0]
    content = rev["slots"]["main"]["*"]
    raw = content.encode("utf-8")
    digest = hashlib.sha256(raw).hexdigest()
    out = Path(args.output_dir); out.mkdir(parents=True, exist_ok=True)
    source_name = f"{args.qid}-abstract-constructor.json"
    source_path = out / source_name
    source_path.write_bytes(raw)
    receipt = {
        "schema": "slr.abstract-wikipedia-constructor-acquisition.v1",
        "qid": args.qid,
        "site": "abstractwiki",
        "source_ref": f"source:abstract-wikipedia:{args.qid}:revid:{rev['revid']}",
        "source_revision_ref": f"source-revision:abstractwiki:revid:{rev['revid']}",
        "document_ref": f"document:sha256:{digest}",
        "canonical_ref": f"canonical:sha256:{digest}",
        "provider_ref": "provider:wikimedia:abstract-wikipedia-api",
        "acquisition_receipt_ref": f"acquisition:abstractwiki:{args.qid}:revid:{rev['revid']}:sha256:{digest}",
        "api_url": url,
        "page_id": page["pageid"], "title": page["title"],
        "revision_id": rev["revid"], "parent_revision_id": rev.get("parentid"),
        "timestamp": rev.get("timestamp"), "content_model": "abstractwiki",
        "media_type": "application/json", "encoding": "utf-8",
        "sha256": digest, "byte_length": len(raw),
        "constructor_json_valid": isinstance(json.loads(content), dict),
        "candidate_only": True, "creates_semantic_authority": False,
        "claim_truth_promoted": False,
        "prose_parser_run": None,
        "jmd_indexed_world_fibre": None,
        "status": "source-acquired-structured-constructor-parser-held",
        "source_path": str(source_path),
    }
    (out / f"{args.qid}-abstract-constructor-receipt.json").write_text(
        json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({k: receipt[k] for k in ("source_ref", "revision_id", "sha256", "byte_length", "status")}))

if __name__ == "__main__": main()
