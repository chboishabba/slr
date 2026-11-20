#!/usr/bin/env python3
"""Materialise one real legal PDF paragraph through the existing spaCy boundary.

This is a fixture harness, not a legal interpreter. It records parser evidence and
candidate status only; truth, occurrence admission, and governed admission remain
unresolved.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import spacy


REPORTING_LEMMAS = {"allege", "claim", "report", "say", "state", "submit", "testify"}


def materialise(path: Path, source_reference: str) -> dict[str, object]:
    text = path.read_text(encoding="utf-8")
    nlp = spacy.load("en_core_web_sm")
    doc = nlp(text)
    sentences = []
    reporting = []
    for sentence_id, sent in enumerate(doc.sents):
        tokens = []
        for token in sent:
            tokens.append(
                {
                    "local": token.i - sent.start,
                    "text": token.text,
                    "lemma": token.lemma_,
                    "dependency": token.dep_,
                    "head_local": token.head.i - sent.start,
                    "start": token.idx,
                    "end": token.idx + len(token.text),
                }
            )
            if token.lemma_.lower() in REPORTING_LEMMAS and token.pos_ in {"VERB", "AUX"}:
                reporting.append(
                    {
                        "sentence_id": sentence_id,
                        "token": token.text,
                        "lemma": token.lemma_.lower(),
                        "pos": token.pos_,
                        "dependency": token.dep_,
                        "span": [token.idx, token.idx + len(token.text)],
                        "source_candidate": "applicant" if "applicant" in sent.text.lower() else None,
                        "embedded_proposition_candidate": True,
                    }
                )
        sentences.append(
            {
                "sentence_id": sentence_id,
                "start": sent.start_char,
                "end": sent.end_char,
                "text": sent.text,
                "tokens": tokens,
            }
        )

    return {
        "schema_version": "sensiblaw.reporting-attribution-fixture.v0_1",
        "authority": "parser_observation_and_candidate_status_only",
        "source_reference": source_reference,
        "text_sha256": __import__("hashlib").sha256(text.encode("utf-8")).hexdigest(),
        "paragraph": {"start": 0, "end": len(text), "sentences": len(sentences)},
        "sentences": sentences,
        "reporting_candidates": reporting,
        "status": {
            "proposition": "asserted_by_source",
            "occurrence": "asserted",
            "truth": "unresolved",
            "candidate_only": True,
            "governed_admission_present": False,
            "parser_alone_authorizes_truth": False,
            "parser_alone_authorizes_occurrence": False,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", type=Path, default=Path("fixtures/reporting_attribution_paragraph.txt"))
    parser.add_argument(
        "--source-reference",
        default="../ITIR-suite/SensibLaw/Native Title (New South Wales) Act 1994 (NSW).pdf",
    )
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    receipt = materialise(args.fixture, args.source_reference)
    payload = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(payload, encoding="utf-8")
        print(f"REPORTING_ATTRIBUTION_HARNESS PASS receipt={args.output}")
    else:
        print(payload, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
