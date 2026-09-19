#!/usr/bin/env python3
"""Derive a work-routing receipt from an already certified GWB v0.4 residual fibre.

This is a post-certification diagnostic transform. It does not run spaCy, Rust, or
semantic admission and therefore cannot create semantic authority. The mapping is
consumer/policy-indexed and is intended only to choose the next producer/reviewer
family for the current SensibLaw legal-PNF workflow.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

SCHEMA = "sensiblaw.gwb-unsupported-demand-routing-receipt.v0_1"
SOURCE_SCHEMA = "sensiblaw.gwb-expanded-semantic-certification-receipt.v0_4"
POLICY_REF = "unsupported-demand-routing:legal-pnf:v0_1"

ROUTES = {
    "surface_structural": {"punct", "det", "predet", "intj", "meta"},
    "relation_attachment": {"prep", "pobj", "pcomp", "agent", "dative", "case", "prt"},
    "nominal_structure": {"compound", "poss", "nummod", "quantmod"},
    "predicate_action": {"ROOT", "attr", "acomp", "oprd", "csubj", "csubjpass", "expl", "parataxis"},
    "coordination": {"conj", "cc", "preconj"},
    "parser_unknown": {"dep"},
}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--input", required=True)
    ap.add_argument("--output", required=True)
    ns = ap.parse_args()

    source_path = Path(ns.input).resolve()
    source_bytes = source_path.read_bytes()
    source = json.loads(source_bytes)
    if source.get("schema_version") != SOURCE_SCHEMA:
        raise SystemExit(f"expected {SOURCE_SCHEMA}")
    if not source.get("invariants", {}).get("full_expanded_gate_pass"):
        raise SystemExit("source v0.4 receipt is not fully certified")
    if not source.get("invariants", {}).get("same_unsupported_dependency_relative_fibre_across_passes"):
        raise SystemExit("source v0.4 receipt does not certify a stable fine residual fibre")

    direct = source["direct_only_performance_pass"]
    fibre = direct["unsupported_dependency_relative_fibre"]
    coarse_total = direct["residual_frontier"]["unsupported_dependency"]

    label_to_route: dict[str, str] = {}
    for route, labels in ROUTES.items():
        for label in labels:
            if label in label_to_route:
                raise SystemExit(f"routing policy overlaps on label {label}")
            label_to_route[label] = route

    observed_labels = set(fibre)
    routed_labels = set(label_to_route)
    missing_policy_labels = sorted(observed_labels - routed_labels)
    unused_policy_labels = sorted(routed_labels - observed_labels)
    if missing_policy_labels:
        raise SystemExit(f"unrouted observed dependency labels: {missing_policy_labels}")

    route_counts = {route: 0 for route in ROUTES}
    route_labels: dict[str, dict[str, int]] = {route: {} for route in ROUTES}
    for label, count in fibre.items():
        route = label_to_route[label]
        route_counts[route] += int(count)
        route_labels[route][label] = int(count)

    routing_total = sum(route_counts.values())
    invariants = {
        "source_v04_full_gate_passed": True,
        "source_fine_fibre_stable_across_passes": True,
        "all_observed_labels_routed_exactly_once": not missing_policy_labels,
        "routing_total_equals_unsupported_dependency_total": routing_total == coarse_total,
        "routing_changes_semantic_authority": False,
        "routing_changes_canonical_consumer_observation": False,
        "routing_is_work_selection_not_legal_meaning": True,
    }
    gate_pass = all(
        value for key, value in invariants.items()
        if key not in {"routing_changes_semantic_authority", "routing_changes_canonical_consumer_observation"}
    ) and not invariants["routing_changes_semantic_authority"] and not invariants["routing_changes_canonical_consumer_observation"]

    receipt = {
        "schema_version": SCHEMA,
        "authority": "consumer_indexed_work_routing_only",
        "policy_ref": POLICY_REF,
        "source_receipt": str(source_path),
        "source_receipt_sha256": sha256_bytes(source_bytes),
        "source_schema_version": SOURCE_SCHEMA,
        "source_observation_digest": direct["canonical_parser_observation_sha256"],
        "coarse_residual": "unsupported_dependency",
        "coarse_residual_total": coarse_total,
        "relative_fine_coordinate": "spacy_dependency_label",
        "route_counts": route_counts,
        "route_labels": route_labels,
        "unused_policy_labels": unused_policy_labels,
        "invariants": invariants,
        "full_routing_gate_pass": gate_pass,
    }

    output = Path(ns.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    for route, count in sorted(route_counts.items(), key=lambda item: (-item[1], item[0])):
        print(f"GWB_UNSUPPORTED_ROUTE route={route} count={count}")
    print(f"GWB_UNSUPPORTED_ROUTE_TOTAL count={routing_total} expected={coarse_total}")
    print(f"GWB_UNSUPPORTED_ROUTING {'PASS' if gate_pass else 'FAIL'} receipt={output}")
    return 0 if gate_pass else 1


if __name__ == "__main__":
    raise SystemExit(main())
