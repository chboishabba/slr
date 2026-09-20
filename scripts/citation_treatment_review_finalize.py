#!/usr/bin/env python3
"""Deprecated compatibility shim for native treatment review finalization."""
from __future__ import annotations
import argparse, shutil, subprocess
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]

def main()->int:
    p=argparse.ArgumentParser()
    p.add_argument("worksheet")
    p.add_argument("output",nargs="?")
    a=p.parse_args()
    base=Path(a.worksheet).resolve().parent
    cmd=["cargo","run","-p","sensiblaw-cli","--bin","sensiblaw","--","legal-follow","waltons","--base",str(base),"treatment","finalize"]
    rc=subprocess.run(cmd,cwd=ROOT,check=False).returncode
    if rc: return rc
    canonical=base/"waltons-treatment-reviewed-decisions.json"
    if a.output and Path(a.output).resolve()!=canonical.resolve():
        shutil.copy2(canonical,a.output)
    return 0
if __name__=="__main__":
    raise SystemExit(main())
