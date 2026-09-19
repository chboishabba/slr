#!/usr/bin/env python3
"""Canonical, order-independent GWB v0.1 source projection.

The historical inventory may list overlapping families in any order. This preparer
tracks all family memberships per resolved path, validates the exact ten-source raw
payload, then assigns deterministic document ordinals from source kind + path rather
than manifest ordering.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import time

from gwb_tranche import (
    BOOKS,
    PROFILE_REF,
    PROJECTION_SCHEMA,
    PUBLIC_BIOS,
    RAW_SUFFIXES,
    load_inventory,
    project_source,
    sha256_bytes,
    sha256_text,
)


def collect_memberships(inventory: dict, source_root: Path) -> dict[Path, set[str]]:
    memberships: dict[Path, set[str]] = {}
    for family in inventory.get("source_families", []):
        family_ref = str(family.get("family_ref", ""))
        for rel in family.get("files", []):
            path = (source_root / rel).resolve()
            if path.suffix.lower() in RAW_SUFFIXES:
                memberships.setdefault(path, set()).add(family_ref)
    return memberships


def source_kind(path: Path) -> str:
    suffix = path.suffix.lower()
    if suffix in {".html", ".htm"}:
        return "public_biography_html"
    if suffix in {".epub", ".pdf"}:
        return "book"
    return "unsupported_raw"


def validate_v01(memberships: dict[Path, set[str]]) -> list[Path]:
    paths = list(memberships)
    bios = [p for p in paths if source_kind(p) == "public_biography_html"]
    books = [p for p in paths if source_kind(p) == "book"]
    unsupported = [p for p in paths if source_kind(p) == "unsupported_raw"]
    if len(paths) != 10 or len(bios) != 6 or len(books) != 4 or unsupported:
        raise SystemExit(
            "GWB v0.1 requires exactly 6 biography HTML + 4 EPUB/PDF books; "
            f"found total={len(paths)} bios={len(bios)} books={len(books)} unsupported={len(unsupported)}"
        )
    wrong_bios = [p for p in bios if PUBLIC_BIOS not in memberships[p]]
    wrong_books = [p for p in books if BOOKS not in memberships[p]]
    if wrong_bios or wrong_books:
        raise SystemExit(
            f"GWB family-membership mismatch: bios_without_public_family={wrong_bios} "
            f"books_without_books_family={wrong_books}"
        )
    missing = [p for p in paths if not p.exists()]
    if missing:
        raise SystemExit(f"GWB raw source path(s) missing: {missing}")
    return sorted(paths, key=lambda p: (0 if source_kind(p) == "public_biography_html" else 1, p.as_posix()))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--inventory", required=True)
    ap.add_argument("--source-root", required=True)
    ap.add_argument("--output", required=True)
    ns = ap.parse_args()

    inventory_path = Path(ns.inventory).resolve()
    source_root = Path(ns.source_root).resolve()
    output = Path(ns.output).resolve()
    text_dir = output / "text"
    text_dir.mkdir(parents=True, exist_ok=True)

    inventory = load_inventory(inventory_path)
    memberships = collect_memberships(inventory, source_root)
    ordered_paths = validate_v01(memberships)

    documents = []
    for ordinal, path in enumerate(ordered_paths, 1):
        raw = path.read_bytes()
        started = time.perf_counter_ns()
        text, projector = project_source(path)
        projection_ns = time.perf_counter_ns() - started
        source_hash = sha256_bytes(raw)
        projected_hash = sha256_text(text)
        out_path = text_dir / f"{ordinal:04d}_{source_hash[:12]}.txt"
        out_path.write_text(text, encoding="utf-8")
        documents.append(
            {
                "document_ordinal": ordinal,
                "source_kind": source_kind(path),
                "family_refs": sorted(memberships[path]),
                "source_path": str(path),
                "source_sha256": source_hash,
                "source_bytes": len(raw),
                "projector": projector,
                "projection_ns": projection_ns,
                "projected_path": str(out_path),
                "projected_sha256": projected_hash,
                "projected_bytes": len(text.encode("utf-8")),
            }
        )

    manifest = {
        "schema_version": PROJECTION_SCHEMA,
        "authority": "source_projection_only",
        "profile_ref": PROFILE_REF,
        "canonical_preparer": "python/gwb_prepare.py",
        "source_inventory": str(inventory_path),
        "source_inventory_sha256": sha256_bytes(inventory_path.read_bytes()),
        "source_family_order_independent": True,
        "document_order": "source_kind_then_resolved_path",
        "document_count": len(documents),
        "documents": documents,
        "derived_inventory_artifacts_reingested": False,
    }
    manifest_path = output / "source_projection.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"GWB_PROJECTED documents={len(documents)} bytes={sum(d['projected_bytes'] for d in documents)}")
    print(manifest_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
