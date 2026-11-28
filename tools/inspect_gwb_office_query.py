#!/usr/bin/env python3
"""Run the existing read-only checked-predicate and numeric planning owners
against the three exact positive query candidates. This is not a semantic parser.
"""
import hashlib
import json
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "artifacts/gwb/multiview-native"
LIMIT = int(os.environ.get("GWB_TIMEOUT_SECONDS", "90"))
BIN = os.environ.get("SLR_NATIVE_EXECUTABLE", str(ROOT / "target/debug/sensiblaw"))

def main():
    cohort = json.loads((BASE / "gwb-office-query-candidate-cohort-v1.json").read_bytes())
    out = Path(os.environ.get("GWB_OFFICE_OUT", str(BASE / "office-query-local")))
    out.mkdir(parents=True, exist_ok=True)
    summary = []
    for v in cohort["views"]:
        edition = v["edition"]
        raw = (BASE / "acquired" / f"{edition}-canonical.txt").read_bytes()
        if hashlib.sha256(raw).hexdigest() != v["source_sha256"]:
            raise ValueError(f"{edition}: digest mismatch")
        text = raw.decode("utf-8")
        a, b = v["predicate_span"]
        c, d = v["clause_span"]
        if not 0 <= c <= a < b <= d <= len(text) or text[a:b] != v["predicate"] or text[c:d] != v["clause"]:
            raise ValueError(f"{edition}: exact inverse mismatch")
        commands = {
            "checked-inverse": ["relational-family", "discover-checked-predicate", v["source_revision_ref"],
                                v["source_sha256"], str(a), str(b), v["predicate"], v["parser_run_ref"]],
            "numeric-plan": ["numeric-pnf-native", "plan", v["source_revision_ref"], v["parser_run_ref"]],
        }
        results = {"edition": edition}
        for lane, args in commands.items():
            target = out / f"{edition}-{lane}.json"
            target.unlink(missing_ok=True)
            try:
                p = subprocess.run([BIN, *args], capture_output=True, timeout=LIMIT, check=True)
                data = json.loads(p.stdout)
                if not isinstance(data, dict):
                    raise ValueError("expected native receipt object")
                if lane == "checked-inverse":
                    if (data.get("source_revision_ref") != v["source_revision_ref"]
                        or data.get("source_sha256") != v["source_sha256"]
                        or data.get("parser_run_constraint") != v["parser_run_ref"]
                        or data.get("source_start_char") != a or data.get("source_end_char") != b
                        or data.get("literal") != v["predicate"]
                        or data.get("writes_database") is not False
                        or data.get("world_truth_paid") is not False):
                        raise ValueError("checked inverse binding mismatch")
                target.write_bytes(p.stdout)
                results[lane] = {"status": data.get("status", "native_plan_returned"), "path": str(target)}
            except (subprocess.SubprocessError, ValueError) as exc:
                results[lane] = {"status": "held", "error": str(exc)}
        summary.append(results)
    print(json.dumps({"views": summary, "semantic_alignment_paid": False, "world_truth_paid": False}, indent=2))
    return 0 if all(v["checked-inverse"]["status"] == "unique_checked_candidate" and
                    v["numeric-plan"]["status"] != "held" for v in summary) else 3

if __name__ == "__main__":
    raise SystemExit(main())
