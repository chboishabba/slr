#!/usr/bin/env python3
"""Acquire historical Wikifunctions definitions for actual Abstract call sites.
No execution or semantic assertion. The selected revision is the latest
revision at or before the source timestamp, not a proof of runtime resolution.
"""
import argparse
import hashlib
import json
from pathlib import Path
import urllib.parse
import urllib.request
from inspect_abstract_constructor import BASE, inventory

def require(ok, reason):
    if not ok:
        raise ValueError(reason)

def write(path, data):
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temp.replace(path)

def reference(value):
    if isinstance(value, str):
        return value
    if isinstance(value, dict) and value.get("Z1K1") == "Z9":
        return value.get("Z9K1")
    return None

def signature(obj, zid):
    require(reference(obj.get("Z1K1")) == "Z2" and reference(obj.get("Z2K1")) == zid,
            "wrong persistent object")
    body = obj["Z2K2"]
    require(reference(body.get("Z1K1")) == "Z8", "not a function")
    args = body["Z8K1"]
    require(isinstance(args, list) and args, "unsupported argument list representation")
    # Canonical typed lists have a type tag as first element.
    if isinstance(args[0], str) or (isinstance(args[0], dict) and reference(args[0]) == "Z17"):
        require(reference(args[0]) == "Z17", "wrong argument list type")
        args = args[1:]
    result = {}
    for arg in args:
        require(reference(arg.get("Z1K1")) == "Z17", "wrong argument declaration")
        key = arg["Z17K2"]
        if isinstance(key, dict):
            require(key.get("Z1K1") == "Z6", "unsupported argument key")
            key = key["Z6K1"]
        require(isinstance(key,str) and key.startswith(zid+"K") and key not in result, "foreign/duplicate argument")
        result[key] = arg["Z17K1"]
    return {"arguments":result,"return_type":body["Z8K2"],
            "tester_refs":body.get("Z8K3"),"implementation_refs":body.get("Z8K4")}

def acquire_object(zid, timestamp, out):
    query = urllib.parse.urlencode({"action":"query","prop":"revisions",
        "titles":zid,"rvstart":timestamp,"rvdir":"older","rvlimit":1,
        "rvprop":"ids|timestamp|content","rvslots":"main","format":"json","formatversion":2})
    url = "https://www.wikifunctions.org/w/api.php?" + query
    req = urllib.request.Request(url,headers={"User-Agent":"DASHI-SLR-abstract-acquisition/1.0","Accept":"application/json"})
    with urllib.request.urlopen(req,timeout=25) as response:
        payload = response.read(8*1024*1024+1)
    require(len(payload)<=8*1024*1024,"oversized response")
    pages = json.loads(payload)["query"]["pages"]
    require(len(pages)==1 and pages[0]["title"]==zid,"foreign function page")
    rev = pages[0]["revisions"][0]
    require(rev["revid"]>0 and rev["timestamp"]<=timestamp,"wrong revision window")
    slot=rev["slots"]["main"]
    require(slot["contentmodel"]=="zobject","unexpected content model")
    raw=slot["content"].encode("utf-8")
    digest=hashlib.sha256(raw).hexdigest()
    path=out/f"{zid}.{rev['revid']}.{digest}.json"
    if path.exists():
        require(path.read_bytes()==raw,"immutable definition collision")
    else:
        path.write_bytes(raw)
    return {"function":zid,"revision":rev["revid"],"timestamp":rev["timestamp"],
            "sha256":digest,"path":path.name,"api_url":url}

def acquire(zid, timestamp, out):
    record=acquire_object(zid,timestamp,out)
    record["signature"]=signature(json.loads((out/record["path"]).read_bytes()),zid)
    return record

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out",required=True,type=Path)
    args=ap.parse_args()
    args.out.mkdir(parents=True,exist_ok=True)
    receipt=json.loads((BASE/"Q207-abstract-constructor-receipt.json").read_bytes())
    raw=(BASE/"Q207-abstract-constructor.json").read_bytes()
    require(hashlib.sha256(raw).hexdigest()==receipt["sha256"] and len(raw)==receipt["byte_length"],"source mismatch")
    calls=inventory(json.loads(raw))
    report={"schema":"slr.abstract-definition-acquisition.v1","source_sha256":receipt["sha256"],
            "source_revision_ref":receipt["source_revision_ref"],"as_of":receipt["timestamp"],
            "definitions":{},"calls":[],"candidate_only":True,"semantic_alignment_paid":False,
            "runtime_dependency_resolution_paid":False,"world_truth_paid":False}
    for zid in sorted({c["function"] for c in calls}):
        try:
            report["definitions"][zid]=acquire(zid,receipt["timestamp"],args.out)
        except (OSError,ValueError,KeyError,TypeError) as exc:
            report["definitions"][zid]={"held":str(exc)}
        write(args.out/"manifest.json",report)
    for c in calls:
        d=report["definitions"][c["function"]]
        row={"pointer":c["pointer"],"function":c["function"],"arguments":c["arguments"],
             "type_check_paid":False,"interpretation_paid":False}
        if "signature" in d:
            row.update(definition_revision=d["revision"],definition_sha256=d["sha256"],
                       argument_keys_match=set(c["arguments"])==set(d["signature"]["arguments"]))
        else:
            row["held"]="definition unavailable"
        report["calls"].append(row)
    write(args.out/"manifest.json",report)
    print(json.dumps(report,ensure_ascii=False,indent=2))
    return 0 if all(c.get("argument_keys_match") is True for c in report["calls"]) else 3

if __name__=="__main__":
    raise SystemExit(main())
