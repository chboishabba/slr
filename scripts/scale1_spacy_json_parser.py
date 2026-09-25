#!/usr/bin/env python3
"""SCALE-1 spaCy worker adapter.

Local process IPC only:
  UTF-8 region text on stdin -> one JSON parser artifact on stdout.

Nothing is persisted here. The Rust SCALE-1 worker owns leases and writes both
normalized hot token rows and this optional JSON artifact directly to Postgres.
"""

from __future__ import annotations

import argparse
import json
import sys


def load_spacy(model_ref: str):
    try:
        import spacy
    except ImportError as exc:
        raise SystemExit("spaCy is not installed in this worker environment") from exc

    try:
        nlp = spacy.load(model_ref)
    except Exception as exc:
        raise SystemExit(f"failed to load spaCy model {model_ref!r}: {exc}") from exc
    return spacy, nlp


def artifact_for_doc(spacy, nlp, args, config, text, doc):
    tokens = []
    for token in doc:
        tokens.append(
            {
                "token_ordinal": token.i,
                "start_char": token.idx,
                "end_char": token.idx + len(token.text),
                "surface": token.text,
                "lemma": token.lemma_,
                "pos": token.pos_,
                "morph": token.morph.to_dict(),
                "head_ordinal": token.head.i,
                "dependency_ref": token.dep_,
            }
        )

    entities = [
        {
            "start_char": ent.start_char,
            "end_char": ent.end_char,
            "text": ent.text,
            "label": ent.label_,
        }
        for ent in doc.ents
    ]

    return {
        "schema": "sensiblaw.scale1.spacy-region.v0_2",
        "parser_family": "spacy",
        "parser_version": spacy.__version__,
        "model_ref": args.model,
        "model_version": nlp.meta.get("version"),
        "config": config,
        "text_char_count": len(text),
        "tokens": tokens,
        "entities": entities,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True)
    parser.add_argument("--config-json", default="{}")
    parser.add_argument("--describe", action="store_true")
    parser.add_argument(
        "--batch-jsonl",
        action="store_true",
        help="read {request_ref,text} JSON lines and emit one artifact line per request",
    )
    args = parser.parse_args()

    spacy, nlp = load_spacy(args.model)
    config = json.loads(args.config_json)

    if "max_length" in config:
        nlp.max_length = int(config["max_length"])

    if args.describe:
        print(
            json.dumps(
                {
                    "parser_family": "spacy",
                    "parser_version": spacy.__version__,
                    "model_ref": args.model,
                    "model_version": nlp.meta.get("version"),
                },
                sort_keys=True,
                separators=(",", ":"),
            )
        )
        return 0

    disable = config.get("disable", [])
    if not isinstance(disable, list) or not all(isinstance(item, str) for item in disable):
        raise SystemExit('config field "disable" must be an array of pipeline names')

    if args.batch_jsonl:
        requests = []
        for line in sys.stdin:
            payload = json.loads(line)
            request_ref = payload.get("request_ref")
            text = payload.get("text")
            if not isinstance(request_ref, str) or not isinstance(text, str):
                raise SystemExit("batch request must contain string request_ref and text")
            requests.append((request_ref, text))

        batch_size = int(config.get("batch_size", len(requests) or 1))
        with nlp.select_pipes(disable=disable):
            docs = nlp.pipe((text for _, text in requests), batch_size=batch_size)
            for (request_ref, text), doc in zip(requests, docs, strict=True):
                print(
                    json.dumps(
                        {
                            "request_ref": request_ref,
                            "artifact": artifact_for_doc(spacy, nlp, args, config, text, doc),
                        },
                        sort_keys=True,
                        separators=(",", ":"),
                        ensure_ascii=False,
                    )
                )
        return 0

    text = sys.stdin.read()
    with nlp.select_pipes(disable=disable):
        doc = nlp(text)
    print(
        json.dumps(
            artifact_for_doc(spacy, nlp, args, config, text, doc),
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
