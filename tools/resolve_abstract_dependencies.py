#!/usr/bin/env python3
"""Bounded historical dependency graph, not a Wikifunctions evaluator.

The root manifest is immutable input. Traverse exact typed Z references and
call targets, retaining paths, cycles, failures, and pending work. A historical
revision window is an acquisition policy, not runtime resolution.
"""
import argparse
import hashlib
import json
import re
from collections import deque
from pathlib import Path
from acquire_abstract_function_definitions import acquire_object, write, require, reference, BASE

ZID=re.compile(r"Z[1-9][0-9]*\Z")
def escape(s):
    return str(s).replace("~","~0").replace("/","~1")

def edges(node, path=""):
    """Only actual Z reference positions; never strings in code or labels."""
    if isinstance(node,dict):
        typ=reference(node.get("Z1K1"))
        if typ=="Z9":
            zid=reference(node)
            if isinstance(zid,str) and ZID.fullmatch(zid):
                yield path,zid,"explicit_reference"
        for key,val in node.items():
            p=path+"/"+escape(key)
            # Literal types, function targets, persistent type and implementation
            # payloads; recurse into structured values, not arbitrary text.
            if key in ("Z1K1","Z7K1","Z17K1","Z8K2","Z14K1","Z20K1"):
                z=reference(val)
                if isinstance(z,str) and ZID.fullmatch(z):
                    yield p,z,"typed_reference"
            if key in ("Z8K3","Z8K4") and isinstance(val,list):
                for idx,item in enumerate(val):
                    z=reference(item)
                    if isinstance(z,str) and ZID.fullmatch(z):
                        yield p+"/"+str(idx),z,"tester_or_implementation"
            if isinstance(val,(dict,list)):
                yield from edges(val,p)
    elif isinstance(node,list):
        for idx,val in enumerate(node):
            p=path+"/"+str(idx)
            if idx==0 and isinstance(val,str) and ZID.fullmatch(val):
                yield p,val,"typed_list_head"
            if isinstance(val,(dict,list)):
                yield from edges(val,p)

def verified_object(directory, row, zid):
    require(row["function"]==zid,"foreign object identity")
    path=directory/row["path"]
    require(path.parent.resolve()==directory.resolve() and path.name==row["path"],"unsafe object path")
    raw=path.read_bytes()
    require(hashlib.sha256(raw).hexdigest()==row["sha256"],"definition digest mismatch")
    obj=json.loads(raw)
    require(reference(obj.get("Z1K1"))=="Z2" and reference(obj.get("Z2K1"))==zid,"wrong persistent Z object")
    return obj

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--definitions",type=Path,required=True)
    p.add_argument("--out",type=Path,required=True)
    p.add_argument("--max-objects",type=int,default=24)
    p.add_argument("--max-depth",type=int,default=3)
    p.add_argument("--online",action="store_true")
    args=p.parse_args()
    require(1<=args.max_objects<=128 and 0<=args.max_depth<=8,"invalid work bound")
    args.out.mkdir(parents=True,exist_ok=True)
    manifest=json.loads((args.definitions/"manifest.json").read_bytes())
    source=json.loads((BASE/"Q207-abstract-constructor-receipt.json").read_bytes())
    require(manifest["schema"]=="slr.abstract-definition-acquisition.v1" and
            manifest["source_sha256"]==source["sha256"] and manifest["as_of"]==source["timestamp"],"foreign manifest")
    roots=sorted({row["function"] for row in manifest["calls"]})
    queue=deque((z,0) for z in roots)
    seen=set()
    result={"schema":"slr.abstract-historical-dependency-graph.v1",
            "source_sha256":source["sha256"],"as_of":source["timestamp"],
            "roots":roots,"nodes":{},"edges":[],"pending":[],"candidate_only":True,
            "runtime_dependency_resolution_paid":False,"semantic_alignment_paid":False,
            "world_truth_paid":False}
    # Resume only immutable verified historical objects from prior checkpoint.
    previous=args.out/"graph.json"
    prior=json.loads(previous.read_bytes()) if previous.exists() else {}
    require(not prior or (prior.get("schema")==result["schema"] and prior.get("source_sha256")==source["sha256"]
                          and prior.get("as_of")==source["timestamp"]),"foreign checkpoint")
    while queue and len(seen)<args.max_objects:
        zid,depth=queue.popleft()
        if zid in seen: continue
        seen.add(zid)
        try:
            row=manifest["definitions"].get(zid,{})
            directory=args.definitions
            if "path" not in row:
                row=prior.get("nodes",{}).get(zid,{})
                directory=args.out
            if "path" not in row:
                require(args.online,"not acquired; explicit --online required")
                row=acquire_object(zid,source["timestamp"],args.out)
            require(row["timestamp"]<=source["timestamp"],"future revision")
            obj=verified_object(directory,row,zid)
            if directory!=args.out:
                dest=args.out/row["path"]
                raw=(directory/row["path"]).read_bytes()
                if dest.exists(): require(dest.read_bytes()==raw,"immutable cache conflict")
                else: dest.write_bytes(raw)
            result["nodes"][zid]=dict(row,depth=depth)
            for pointer,target,kind in sorted(set(edges(obj))):
                result["edges"].append({"source":zid,"target":target,"pointer":pointer,"kind":kind})
                if target not in seen:
                    if depth<args.max_depth: queue.append((target,depth+1))
                    else: result["pending"].append({"target":target,"reason":"depth_bound"})
        except (OSError,ValueError,KeyError,TypeError) as exc:
            result["nodes"][zid]={"held":str(exc),"depth":depth}
        write(previous,result)
    result["pending"].extend({"target":z,"reason":"object_bound"} for z,_ in queue if z not in seen)
    result["pending"]=[row for row in result["pending"] if "path" not in result["nodes"].get(row["target"],{})]
    write(previous,result)
    print(json.dumps(result,ensure_ascii=False,indent=2))
    return 3 if result["pending"] or any("held" in row for row in result["nodes"].values()) else 0

if __name__=="__main__":
    raise SystemExit(main())
