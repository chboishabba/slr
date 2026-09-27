#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def load_json(path):
    return json.loads(Path(path).read_text())


def chars(path):
    return list(Path(path).read_text())


def edit_window(before, after):
    prefix = 0
    max_prefix = min(len(before), len(after))
    while prefix < max_prefix and before[prefix] == after[prefix]:
        prefix += 1

    suffix = 0
    max_suffix = min(len(before) - prefix, len(after) - prefix)
    while (
        suffix < max_suffix
        and before[len(before) - 1 - suffix] == after[len(after) - 1 - suffix]
    ):
        suffix += 1

    old_end = len(before) - suffix
    new_end = len(after) - suffix
    return prefix, old_end, prefix, new_end


def classify(sentences, start, end):
    unaffected_before = []
    affected = []
    unaffected_after = []
    for sentence in sentences:
        s = int(sentence["start_char"])
        e = int(sentence["end_char"])
        if e <= start:
            unaffected_before.append(sentence)
        elif s >= end:
            unaffected_after.append(sentence)
        else:
            affected.append(sentence)
    return unaffected_before, affected, unaffected_after


def transported_key(sentence, delta=0):
    return (
        int(sentence["start_char"]) + delta,
        int(sentence["end_char"]) + delta,
        sentence["content_digest_ref"],
    )


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--before-text", required=True)
    ap.add_argument("--after-text", required=True)
    ap.add_argument("--before-segmentation", required=True)
    ap.add_argument("--after-segmentation", required=True)
    ap.add_argument("--after-receipt", required=True)
    ap.add_argument("--output", required=True)
    args = ap.parse_args()

    before_chars = chars(args.before_text)
    after_chars = chars(args.after_text)
    old_start, old_end, new_start, new_end = edit_window(before_chars, after_chars)
    delta = (new_end - new_start) - (old_end - old_start)

    before_seg = load_json(args.before_segmentation)
    after_seg = load_json(args.after_segmentation)
    receipt = load_json(args.after_receipt)

    before_sentences = before_seg["sentences"]
    after_sentences = after_seg["sentences"]

    before_prefix, affected_before, before_suffix = classify(
        before_sentences, old_start, old_end
    )
    after_prefix, affected_after, after_suffix = classify(
        after_sentences, new_start, new_end
    )

    after_keys = {
        (
            int(sentence["start_char"]),
            int(sentence["end_char"]),
            sentence["content_digest_ref"],
        )
        for sentence in after_sentences
    }

    expected = []
    missing = []
    for sentence in before_prefix:
        key = transported_key(sentence, 0)
        expected.append(key)
        if key not in after_keys:
            missing.append(key)
    for sentence in before_suffix:
        key = transported_key(sentence, delta)
        expected.append(key)
        if key not in after_keys:
            missing.append(key)

    parser = receipt["parser"]
    semantic_regions = int(receipt["integrity"]["semantic_eligible_regions"])
    cross_reused = int(parser.get("cross_revision_reused_jobs", 0))
    same_reused = int(parser.get("same_revision_reused_jobs", 0))
    new_jobs = int(parser["new_jobs"])

    transported_unaffected = len(expected)
    parser_locality_green = (
        not missing
        and cross_reused >= transported_unaffected
        and new_jobs <= len(affected_after)
        and cross_reused + same_reused + new_jobs == semantic_regions
    )

    integrity = receipt["integrity"]
    residual_regions = int(integrity.get("parser_residual_regions", 0))
    product_hits = int(integrity.get("candidate_product_reuse_hits_this_run", 0))
    new_products = int(integrity.get("candidate_product_new_this_run", 0))
    product_factor_writes = int(
        integrity.get("candidate_product_factor_rows_inserted_this_run", 0)
    )
    statement_writes = int(
        integrity.get("source_statement_rows_inserted_this_run", 0)
    )
    batch_writes = int(
        integrity.get("candidate_batch_rows_inserted_this_run", 0)
    )

    # With no parser residuals, every transported semantic-eligible sentence
    # should bind an already exact-reopened candidate semantic product.  When
    # residuals exist we preserve parser-locality evidence but report semantic
    # product locality as indeterminate rather than manufacturing a mapping
    # from aggregate residual counts to individual transported occurrences.
    if residual_regions == 0:
        semantic_product_locality = (
            product_hits >= transported_unaffected
            and new_products <= len(affected_after)
        )
        semantic_product_locality_status = (
            "green" if semantic_product_locality else "failed"
        )
    else:
        semantic_product_locality = None
        semantic_product_locality_status = "indeterminate_parser_residuals"

    result = {
        "schema": "sensiblaw.scale1.small-edit-locality-audit.v0_1",
        "edit_transport": {
            "old_start_char": old_start,
            "old_end_char": old_end,
            "new_start_char": new_start,
            "new_end_char": new_end,
            "delta_chars": delta,
            "old_changed_chars": old_end - old_start,
            "new_changed_chars": new_end - new_start,
        },
        "segmentation": {
            "before_sentence_count": len(before_sentences),
            "after_sentence_count": len(after_sentences),
            "transported_unaffected_sentence_count": transported_unaffected,
            "affected_before_sentence_count": len(affected_before),
            "affected_after_sentence_count": len(affected_after),
            "missing_transported_sentence_count": len(missing),
        },
        "parser": {
            "semantic_regions": semantic_regions,
            "same_revision_reused_jobs": same_reused,
            "cross_revision_reused_jobs": cross_reused,
            "new_jobs": new_jobs,
            "parser_locality_green": parser_locality_green,
        },
        "semantic_product": {
            "status": semantic_product_locality_status,
            "locality_green": semantic_product_locality,
            "reuse_hits_this_run": product_hits,
            "new_products_this_run": new_products,
            "product_factor_rows_inserted_this_run": product_factor_writes,
        },
        "occurrence_binding": {
            "source_statement_rows_inserted_this_run": statement_writes,
            "candidate_batch_rows_inserted_this_run": batch_writes,
            "corpus_linear_binding_observed": (
                statement_writes > len(affected_after)
                or batch_writes > len(affected_after)
            ),
        },
        "boundary": {
            "transport_uses_source_coordinates": True,
            "semantic_value_used_as_occurrence_identity": False,
            "candidate_only": True,
            "creates_semantic_authority": False,
            "claim_truth_promoted": False,
        },
    }

    Path(args.output).write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    if not parser_locality_green:
        raise SystemExit(2)
    if semantic_product_locality is False:
        raise SystemExit(3)


if __name__ == "__main__":
    main()
