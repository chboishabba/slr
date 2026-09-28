#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def load(path):
    return json.loads(Path(path).read_text())


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--same-run", required=True)
    ap.add_argument("--small-edit", required=True)
    ap.add_argument("--same-domain", required=True)
    ap.add_argument("--output", required=True)
    args = ap.parse_args()

    same_run = load(args.same_run)
    small_edit = load(args.small_edit)
    same_domain = load(args.same_domain)

    require(
        same_run.get("schema") == "sensiblaw.scale1.source-compile-baseline.v0_1",
        "same-run receipt schema mismatch",
    )
    require(
        small_edit.get("schema") == "sensiblaw.scale1.small-edit-locality-audit.v0_3",
        "small-edit receipt schema mismatch",
    )
    require(
        same_domain.get("schema") == "sensiblaw.scale1.source-compile-baseline.v0_1",
        "same-domain receipt schema mismatch",
    )

    heads = {
        same_run.get("runtime_head"),
        small_edit.get("runtime_head"),
        same_domain.get("runtime_head"),
    }
    require(None not in heads and len(heads) == 1, "receipts are not from one runtime head")
    runtime_head = next(iter(heads))

    sr_perf = same_run["performance"]
    sr_card = same_run["candidate_cardinality"]
    sr_integrity = same_run["integrity"]
    require(sr_perf.get("same_run_parser_relative_metrics_available") is True,
            "same-run parser-relative metric unavailable")
    require(sr_perf.get("post_spacy_target_met") is True,
            "same-run post-spaCy target not met")
    require(sr_perf.get("full_post_spacy_to_spacy_ratio") is not None,
            "same-run full post-spaCy ratio missing")
    require(sr_card.get("review_delta_fibre_input_used") is True,
            "same-run review projection did not use reconciliation delta")
    require(
        int(sr_card.get("review_delta_fibre_count", -1))
        == int(sr_card.get("review_target_fibre_count", -2)),
        "same-run review target differs from reconciliation delta",
    )
    require(int(sr_integrity.get("unattempted_semantic_regions", -1)) == 0,
            "same-run left semantic regions unattempted")

    se_parser = small_edit["parser"]
    se_semantic = small_edit["semantic_product"]
    se_l2 = small_edit["l2_reconciliation"]
    se_review = small_edit["review_projection"]
    require(se_parser.get("parser_locality_green") is True,
            "small-edit parser locality failed")
    require(se_semantic.get("locality_green") is not False,
            "small-edit semantic-product locality failed")
    require(se_l2.get("status") == "green" and se_l2.get("summary_creation_local") is True,
            "small-edit L2 locality failed")
    require(se_review.get("status") == "green",
            "small-edit review locality failed")
    require(se_review.get("delta_fibre_input_used") is True,
            "small-edit review did not use reconciliation delta")
    require(
        int(se_review.get("delta_fibre_count", -1))
        == int(se_review.get("target_fibre_count", -2)),
        "small-edit review target differs from reconciliation delta",
    )

    sd_parser = same_domain["parser"]
    sd_perf = same_domain["performance"]
    sd_card = same_domain["candidate_cardinality"]
    sd_integrity = same_domain["integrity"]
    require(int(sd_parser.get("same_revision_reused_jobs", -1)) == 0,
            "same-domain fixture was not genuinely new")
    require(int(sd_perf.get("candidate_product_reuse_hits_this_run", 0)) > 0,
            "same-domain source reused no semantic products")
    require(
        int(sd_card.get("l2_product_summary_reuse_hits_this_run", 0))
        >= int(sd_perf.get("candidate_product_reuse_hits_this_run", 0)),
        "same-domain L2 failed to reuse reused semantic products",
    )
    require(
        int(sd_card.get("l2_factor_rows_scanned_this_run", 0))
        <= int(sd_perf.get("candidate_product_factor_rows_inserted_this_run", 0)),
        "same-domain L2 scanned factors outside newly created semantic products",
    )
    require(sd_card.get("review_delta_fibre_input_used") is True,
            "same-domain review projection did not use reconciliation delta")
    require(
        int(sd_card.get("review_delta_fibre_count", -1))
        == int(sd_card.get("review_target_fibre_count", -2)),
        "same-domain review target differs from reconciliation delta",
    )
    require(int(sd_integrity.get("unattempted_semantic_regions", -1)) == 0,
            "same-domain source left semantic regions unattempted")

    result = {
        "schema": "sensiblaw.scale1.economy-closure.v0_1",
        "runtime_head": runtime_head,
        "authority": "empirical_execution_receipt_only",
        "same_run_parser_dominance": {
            "green": True,
            "full_post_spacy_to_spacy_ratio": sr_perf["full_post_spacy_to_spacy_ratio"],
            "target_ratio": sr_perf["post_spacy_target_ratio"],
            "review_delta_fibre_input_used": True,
        },
        "small_edit_locality": {
            "green": True,
            "parser_locality": True,
            "semantic_product_locality": se_semantic.get("locality_green"),
            "l2_product_locality": True,
            "review_delta_locality": True,
        },
        "same_domain_new_source": {
            "green": True,
            "candidate_product_reuse_hits": sd_perf["candidate_product_reuse_hits_this_run"],
            "l2_product_summary_reuse_hits": sd_card["l2_product_summary_reuse_hits_this_run"],
            "review_delta_locality": True,
        },
        "boundary": {
            "candidate_only": True,
            "creates_semantic_authority": False,
            "applicability_promoted": False,
            "claim_truth_promoted": False,
        },
        "scale1_economy_closed": True,
    }

    Path(args.output).write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
