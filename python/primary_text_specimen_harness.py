#!/usr/bin/env python3
"""Parse bounded primary-text specimens without assigning legal authority."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import spacy


REPORTING_LEMMAS = {"allege", "claim", "report", "say", "state", "submit", "testify"}


def parse_specimen(specimen: dict[str, str], root: Path, nlp) -> dict[str, object]:
    fixture = root / specimen["fixture"]
    text = fixture.read_text(encoding="utf-8")
    doc = nlp(text)
    sentences = []
    reporting_candidates = []
    for sentence_id, sent in enumerate(doc.sents):
        tokens = []
        for token in sent:
            tokens.append({
                "local_ordinal": token.i - sent.start,
                "sentence_id": sentence_id,
                "start": token.idx,
                "end": token.idx + len(token.text),
                "orth": token.text,
                "lemma": token.lemma_,
                "pos": token.pos_,
                "tag": token.tag_,
                "morphology": str(token.morph),
                "dependency": token.dep_,
                "head_local_ordinal": token.head.i - sent.start,
            })
            if token.lemma_.lower() in REPORTING_LEMMAS and token.pos_ in {"VERB", "AUX"}:
                reporting_candidates.append({
                    "sentence_id": sentence_id,
                    "local_ordinal": token.i - sent.start,
                    "span": [token.idx, token.idx + len(token.text)],
                    "kind": "reporting_predicate_candidate",
                })
        sentences.append({
            "sentence_id": sentence_id,
            "start": sent.start_char,
            "end": sent.end_char,
            "text": sent.text,
            "tokens": tokens,
        })
    encoded = text.encode("utf-8")
    return {
        "id": specimen["id"],
        "schema_version": "sensiblaw.primary-text-parser-receipt.v0_1",
        "authority": "parser_observation_and_candidate_status_only",
        "source_metadata": {
            "source_pdf": specimen["source_pdf"],
            "source_passage": specimen["source_passage"],
            "reviewed_context": specimen["reviewed_context"],
        },
        "canonical_input": {
            "fixture": specimen["fixture"],
            "sha256": hashlib.sha256(encoded).hexdigest(),
            "bytes": len(encoded),
            "paragraphs": len(text.split("\n\n")),
        },
        "sentences": sentences,
        "parser_candidates": {"reporting_predicates": reporting_candidates},
        "status": {
            "candidate_only": True,
            "governed_admission_present": False,
            "parser_alone_authorizes_truth": False,
            "parser_alone_authorizes_occurrence": False,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=Path("fixtures/primary_text_specimen_manifest.json"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--model", default="en_core_web_sm")
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    nlp = spacy.load(args.model)
    receipt = {
        "schema_version": "sensiblaw.primary-text-parser-batch-receipt.v0_1",
        "authority": "parser_observation_and_candidate_status_only",
        "model": args.model,
        "specimens": [parse_specimen(specimen, args.manifest.parent.parent, nlp) for specimen in manifest["specimens"]],
    }
    args.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"PRIMARY_TEXT_SPECIMENS PASS documents={len(receipt['specimens'])} receipt={args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
