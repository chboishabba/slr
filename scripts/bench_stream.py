#!/usr/bin/env python3
"""Benchmark harness. Requires a built target/debug/sensiblaw-stream.
Gate: overlapped total pipeline walltime <= 2 * spaCy parse walltime.
The processes overlap: Rust starts before spaCy and consumes each flushed sentence immediately.
"""
from __future__ import annotations
import argparse, re, subprocess, sys
from pathlib import Path

SPACY_RE = re.compile(r"SPACY_METRIC parse_ns=(\d+)")
SL_RE = re.compile(r"SL_METRIC active_ns=(\d+) pipeline_wall_ns=(\d+)")

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("path")
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--rust-bin", default="target/debug/sensiblaw-stream")
    ns = ap.parse_args()
    root = Path(__file__).resolve().parents[1]
    rust_bin = root / ns.rust_bin
    if not rust_bin.exists():
        print(f"missing {rust_bin}; build with cargo build --workspace", file=sys.stderr)
        return 2
    spacy_cmd = [sys.executable, str(root / "python/spacy_stream.py"), "--model", ns.model, ns.path]
    sp = subprocess.Popen(spacy_cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
    sl = subprocess.Popen([str(rust_bin)], stdin=sp.stdout, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
    sp.stdout.close()
    sl_out, sl_err = sl.communicate()
    sp_err = sp.stderr.read(); sp.wait()
    sm = SPACY_RE.search(sp_err); lm = SL_RE.search(sl_err)
    if not sm or not lm:
        print(sp_err, file=sys.stderr); print(sl_err, file=sys.stderr); return 3
    spacy_ns = int(sm.group(1)); active_ns = int(lm.group(1)); total_ns = int(lm.group(2))
    ratio = total_ns / max(spacy_ns, 1)
    print(f"spaCy parse occupancy: {spacy_ns/1e6:.3f} ms")
    print(f"SensibLaw active work: {active_ns/1e6:.3f} ms")
    print(f"overlapped total pipeline: {total_ns/1e6:.3f} ms")
    print(f"total/spaCy ratio: {ratio:.4f}x (architectural limit 2.0000x)")
    if total_ns > 2 * spacy_ns:
        print("FAIL: published T_total <= 2*T_spaCy gate exceeded")
        return 1
    print("PASS: T_total <= 2*T_spaCy")
    return 0

if __name__ == "__main__": raise SystemExit(main())
