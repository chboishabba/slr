#!/usr/bin/env python3
"""Read-only source-manifestation comparison for the indexed/native meet.

This deliberately reports literal-span candidates separately from licensed
crosswalk edges. A repeated word in two different works is not a source weld.
"""
import argparse, base64, hashlib, json, os, subprocess
from pathlib import Path

def native_payload(db_url, canonical_ref):
    sql = ("select encode(payload,'base64') from corpus.canonical_content "
           "where canonical_ref='" + canonical_ref.replace("'", "''") + "'")
    p = subprocess.run(["psql", db_url, "-Atc", sql], check=True,
                       capture_output=True, text=True)
    return base64.b64decode(p.stdout)

def occurrences(text, literal):
    out, start = [], 0
    while True:
        i = text.find(literal, start)
        if i < 0: return out
        out.append(i); start = i + max(1, len(literal))

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--database-url", default=os.environ.get("DATABASE_URL"), required=False)
    ap.add_argument("--article", required=True)
    ap.add_argument("--indexed-record", required=True)
    ap.add_argument("--native-canonical-ref", required=True)
    ap.add_argument("--native-revision-ref", required=True)
    ap.add_argument("--native-document-ref", required=True)
    ap.add_argument("--native-source-ref", required=True)
    ap.add_argument("--native-provider-ref", required=True)
    ap.add_argument("--native-acquisition-receipt-ref", required=True)
    ap.add_argument("--output", required=True)
    args = ap.parse_args()
    if not args.database_url: raise SystemExit("DATABASE_URL is required")
    article_bytes = Path(args.article).read_bytes()
    native_bytes = native_payload(args.database_url, args.native_canonical_ref)
    article = article_bytes.decode("utf-8")
    native = native_bytes.decode("utf-8")
    record = json.loads(Path(args.indexed_record).read_text())
    predicates = [x["predicate"] for x in record.get("event_candidates", [])]
    candidates = []
    for p in predicates:
        literal = p["surface"]
        hits = occurrences(native, literal)
        if len(hits) == 1:
            i = hits[0]
            candidates.append({"indexed_span": [p["start_char"], p["end_char"]],
                "literal": literal, "native_span": [i, i + len(literal)],
                "indexed_context": article[max(0,p["start_char"]-60):p["end_char"]+60],
                "native_context": native[max(0,i-60):i+len(literal)+60],
                "status": "literal-unique-context-not-equated"})
    result = {
        "schema": "slr.jmd-native-source-alignment.v1",
        "jmd_source": {
            "revision_ref": record.get("revision_ref", "enwiki-revid:1385850169"),
            "source_unit_ref": record.get("source_unit_ref", "wikipedia:en:George_W._Bush:1385850169"),
            "sha256": hashlib.sha256(article_bytes).hexdigest(),
            "byte_length": len(article_bytes)},
        "native_source": {"revision_ref": args.native_revision_ref,
            "document_ref": args.native_document_ref, "source_ref": args.native_source_ref,
            "provider_ref": args.native_provider_ref,
            "acquisition_receipt_ref": args.native_acquisition_receipt_ref,
            "canonical_ref": args.native_canonical_ref,
            "sha256": hashlib.sha256(native_bytes).hexdigest(),
            "byte_length": len(native_bytes)},
        "exact_digest_match": hashlib.sha256(article_bytes).digest() == hashlib.sha256(native_bytes).digest(),
        "source_work_match": False,
        "literal_candidate_count": len(candidates),
        "licensed_crosswalk_count": 0,
        "crosswalk_status": "blocked-different-source-manifestations",
        "reason": "The JMD input is a George W. Bush Wikipedia article; the native input is a GWB book/PDF projection. Literal overlap is retained as diagnostic evidence, never promoted to source-coordinate identity.",
        "literal_candidates": candidates}
    Path(args.output).write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({k: result[k] for k in ("exact_digest_match", "literal_candidate_count", "licensed_crosswalk_count", "crosswalk_status")}))

if __name__ == "__main__": main()
