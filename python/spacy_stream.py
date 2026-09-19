#!/usr/bin/env python3
"""spaCy observation producer for SensibLaw's delta-native stream.

The worker is intentionally outside canonical state. It parses independently-owned
text blocks (paragraphs by default), emits complete sentences immediately, then moves
on to the next block. Rust can therefore process/fuse closed sentence deltas while
spaCy is parsing later blocks.

Protocol (TSV):
 D <revision_id>
 P <paragraph_id>
 S <sentence_id> <absolute_start> <absolute_end>
 T <local_ord> <absolute_start> <absolute_end> <head_local_ord> <orth> <lemma> <pos|- > <tag|- > <dep|- >
 E <sentence_id>
 Q <paragraph_id>
 M spacy_parse_ns=<sum of parser walltime over blocks>

No candidate is promoted here. Whitespace-only sentences are never emitted.
"""
from __future__ import annotations
import argparse
import re
import sys
import time
import spacy


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n")


def load_pipeline(model: str):
    try:
        return spacy.load(model)
    except OSError:
        nlp = spacy.blank("en")
        nlp.add_pipe("sentencizer")
        return nlp


def paragraph_blocks(text: str):
    """Yield (absolute_start, block_text) without changing source offsets."""
    pos = 0
    for m in re.finditer(r"(?:\r?\n){2,}", text):
        block = text[pos:m.start()]
        if block.strip():
            lead = len(block) - len(block.lstrip())
            trail_end = len(block.rstrip())
            yield pos + lead, block[lead:trail_end]
        pos = m.end()
    block = text[pos:]
    if block.strip():
        lead = len(block) - len(block.lstrip())
        trail_end = len(block.rstrip())
        yield pos + lead, block[lead:trail_end]


def emit_doc(nlp, text: str, revision_id: int) -> int:
    print(f"D\t{revision_id}", flush=True)
    total_parse_ns = 0
    sid = 0
    paragraph_id = 0
    for block_start, block in paragraph_blocks(text):
        print(f"P\t{paragraph_id}", flush=True)
        t0 = time.perf_counter_ns()
        doc = nlp(block)
        total_parse_ns += time.perf_counter_ns() - t0
        for sent in doc.sents:
            if not sent.text.strip():
                continue
            abs_sent_start = block_start + sent.start_char
            abs_sent_end = block_start + sent.end_char
            print(f"S\t{sid}\t{abs_sent_start}\t{abs_sent_end}")
            for local, tok in enumerate(sent):
                if tok.dep_:
                    head_local = tok.head.i - sent.start
                    dep = tok.dep_
                else:
                    head_local = local
                    dep = "-"
                lemma = tok.lemma_ if tok.lemma_ else tok.text
                pos = tok.pos_ if tok.pos_ else "-"
                tag = tok.tag_ if tok.tag_ else "-"
                abs_start = block_start + tok.idx
                abs_end = abs_start + len(tok.text)
                print("\t".join([
                    "T", str(local), str(abs_start), str(abs_end), str(head_local),
                    esc(tok.text), esc(lemma), pos, tag, dep,
                ]))
            print(f"E\t{sid}", flush=True)
            sid += 1
        print(f"Q\t{paragraph_id}", flush=True)
        paragraph_id += 1
    print(f"M\tspacy_parse_ns={total_parse_ns}", flush=True)
    return total_parse_ns


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--revision-id", type=int, default=1)
    ap.add_argument("path", nargs="?")
    ns = ap.parse_args()
    text = open(ns.path, encoding="utf-8").read() if ns.path else sys.stdin.read()
    nlp = load_pipeline(ns.model)
    parse_ns = emit_doc(nlp, text, ns.revision_id)
    print(f"SPACY_METRIC parse_ns={parse_ns}", file=sys.stderr)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
