#!/usr/bin/env python3
"""Deprecated compatibility shim for native cited-by OALC worklist/acquisition."""
from __future__ import annotations
import argparse, subprocess
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]

def main()->int:
    p=argparse.ArgumentParser()
    p.add_argument("candidates")
    p.add_argument("--execute",action="store_true")
    p.add_argument("--output-dir")
    a=p.parse_args()
    base=Path(a.candidates).resolve().parent
    sub=["cited-by","acquire"] if a.execute else ["cited-by","worklist"]
    cmd=["cargo","run","-p","sensiblaw-cli","--bin","sensiblaw","--","legal-follow","waltons","--base",str(base),*sub]
    return subprocess.run(cmd,cwd=ROOT,check=False).returncode
if __name__=="__main__":
    raise SystemExit(main())
