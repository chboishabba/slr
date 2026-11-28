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

def indexed_event_lane(record, article):
    """Accept the actual nested JMD receipt or the original projection.
    An unavailable lane is not an empty observed event inventory."""
    if record.get("schema") == "slr.jmd-indexed-world-fibre-receipt.v1":
        inv = record.get("inventory")
        if (not isinstance(inv, dict)
                or inv.get("schema") != "slr.jmd-indexed-world-fibre.v1"
                or record.get("source_observation_checked") is not True
                or record.get("cross_source_alignment_paid") is not False
                or record.get("world_truth_paid") is not False):
            raise ValueError("invalid indexed JMD receipt")
        if record.get("source_sha256") != hashlib.sha256(article.encode("utf-8")).hexdigest():
            raise ValueError("indexed JMD source digest mismatch")
        lanes = inv.get("article_lanes")
        if not isinstance(lanes, dict):
            raise ValueError("missing article lanes")
        events = lanes.get("event_candidates")
        revision = inv.get("article_revision")
        unit = None  # runner does not export the original source-unit field
    else:
        events = record.get("event_candidates")
        revision = record.get("revision_ref")
        unit = record.get("source_unit_ref")
        declared = record.get("source_text_sha256")
        if declared is not None and declared != hashlib.sha256(article.encode("utf-8")).hexdigest():
            raise ValueError("original projection source digest mismatch")
    if events is None:
        return revision, unit, None
    if not isinstance(events, list):
        raise ValueError("event_candidates must be an array or unavailable")
    predicates = []
    for event in events:
        p = event["predicate"]
        a, b, literal = p["start_char"], p["end_char"], p["surface"]
        if (type(a) is not int or type(b) is not int or not 0 <= a < b <= len(article)
                or not isinstance(literal, str) or article[a:b] != literal):
            raise ValueError("indexed predicate does not match original source span")
        predicates.append(p)
    return revision, unit, predicates


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
    revision, unit, predicates = indexed_event_lane(record, article)
    candidates = []
    diagnostics = []
    for p in predicates or []:
        literal = p["surface"]
        hits = occurrences(native, literal)
        diagnostics.append({"indexed_span": [p["start_char"], p["end_char"]],
                            "literal": literal, "native_occurrence_count": len(hits)})
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
            "revision_ref": revision,
            "source_unit_ref": unit,
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
        "source_work_match": None,
        "event_lane_available": predicates is not None,
        "indexed_predicate_count": None if predicates is None else len(predicates),
        "literal_candidate_count": None if predicates is None else len(candidates),
        "literal_candidate_count_policy": "unique native literal occurrences only; not exhaustive semantic overlap",
        "literal_occurrence_diagnostics": diagnostics,
        "licensed_crosswalk_count": 0,
        "crosswalk_status": ("held-unavailable-event-lane" if predicates is None else
            "blocked-different-content" if article_bytes != native_bytes else
            "held-source-revision-licence-required"),
        "reason": "Content identity, source-work identity, and reviewed coordinate transport are distinct. Literal diagnostics never establish semantic or source-coordinate identity. Native source labels here are operator-supplied.",
        "literal_candidates": candidates}
    Path(args.output).write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({k: result[k] for k in ("exact_digest_match", "literal_candidate_count", "licensed_crosswalk_count", "crosswalk_status")}))

if __name__ == "__main__": main()
