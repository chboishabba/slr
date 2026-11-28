#!/usr/bin/env python3
"""Original Z-object source inventory with exact JSON Pointer inverses.

No signature inference, renderer invocation, prose parsing or semantic
licence. Unknown constructor meaning stays a source acquisition obligation.
"""
import hashlib
import json
from pathlib import Path

BASE = Path(__file__).resolve().parents[1] / "artifacts/gwb/multiview-native/acquired"

def require(ok, message):
    if not ok:
        raise ValueError(message)

def esc(key):
    return str(key).replace("~", "~0").replace("/", "~1")

def reopen(root, path):
    for part in path.split("/")[1:]:
        part = part.replace("~1", "/").replace("~0", "~")
        root = root[int(part)] if isinstance(root, list) else root[part]
    return root

def inventory(root):
    calls = []
    def walk(node, path):
        if isinstance(node, dict):
            if node.get("Z1K1") == "Z7":
                fn = node.get("Z7K1")
                require(isinstance(fn, str) and fn.startswith("Z") and fn[1:].isdigit(), "unsupported function reference")
                args = {key: {"pointer": path + "/" + esc(key), "value": value}
                        for key, value in node.items() if key not in ("Z1K1", "Z7K1")}
                require(all(k.startswith(fn + "K") and k[len(fn)+1:].isdigit() for k in args), "unexpected argument key")
                calls.append({"pointer": path, "function": fn, "arguments": args,
                              "signature_status": "unresolved", "interpretation_status": "held"})
            for key, value in node.items():
                walk(value, path + "/" + esc(key))
        elif isinstance(node, list):
            for idx, value in enumerate(node):
                walk(value, path + "/" + str(idx))
    walk(root, "")
    for row in calls:
        require(reopen(root,row["pointer"])["Z7K1"] == row["function"], "function inverse mismatch")
        for arg in row["arguments"].values():
            require(reopen(root,arg["pointer"]) == arg["value"], "argument inverse mismatch")
    return calls

def main():
    raw = (BASE / "Q207-abstract-constructor.json").read_bytes()
    receipt = json.loads((BASE / "Q207-abstract-constructor-receipt.json").read_bytes())
    digest = hashlib.sha256(raw).hexdigest()
    require(digest == receipt["sha256"] and len(raw) == receipt["byte_length"], "original source mismatch")
    root = json.loads(raw)
    require(root["qid"] == receipt["qid"] == "Q207" and receipt["content_model"] == "abstractwiki", "source identity mismatch")
    calls = inventory(root)
    require(calls, "no original calls")
    print(json.dumps({"schema":"slr.abstract-constructor-structural-inverse.v1",
                      "source_revision_ref":receipt["source_revision_ref"],
                      "source_sha256":digest,"calls":calls,
                      "signature_acquisition_required":sorted({row["function"] for row in calls}),
                      "candidate_only":True,"semantic_alignment_paid":False,
                      "world_truth_paid":False}, ensure_ascii=False, indent=2))

if __name__ == "__main__":
    main()
