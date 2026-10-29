#!/usr/bin/env python3
"""Shared Digital-ESD screening semantics for real ERIC study records.

This module is intentionally lower-authority than the screening ledger.
It creates:
- authoritative *unresolved* ledger rows from parsed ERIC metadata;
- candidate-only title/abstract assessments for reviewer work allocation.

It never creates an authoritative include/exclude decision.
"""

from __future__ import annotations

import hashlib
import json
import re
from typing import Any

from interop_scripts.digital_esd_eric import ERICRecord

DIGITAL_TERMS = (
    "digital education",
    "digital learning",
    "educational technology",
    "edtech",
    "online learning",
    "blended learning",
    "learning platform",
    "artificial intelligence",
    "generative ai",
    "digital technology",
    "digital technologies",
)

SUSTAINABILITY_TERMS = (
    "education for sustainable development",
    "sustainability education",
    "sustainable development",
    "environmental education",
    "sustainability",
    "environmental sustainability",
    "e-waste",
    "circularity",
    "repairability",
    "life cycle",
    "lifecycle",
    "carbon",
    "emissions",
)

EMPIRICAL_TERMS = (
    "study",
    "participants",
    "sample",
    "survey",
    "interview",
    "experiment",
    "randomized",
    "randomised",
    "case study",
    "longitudinal",
    "evaluation",
    "mixed methods",
    "qualitative",
    "quantitative",
    "students",
    "teachers",
)

_SPACE_RE = re.compile(r"\s+")


def sha256_json(value: Any) -> str:
    payload = json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _normalise(text: str) -> str:
    return _SPACE_RE.sub(" ", text.lower()).strip()


def _hits(text: str, terms: tuple[str, ...]) -> list[str]:
    haystack = _normalise(text)
    return sorted(term for term in terms if term in haystack)


def screening_row_from_eric(record: ERICRecord) -> dict[str, Any]:
    """Create one authoritative unresolved screening row.

    Parsing metadata pays presence/identity only.  It does not pay a screening
    decision, source truth, full-text retrieval, or SourceAuditAdmission.
    """
    snapshot = {
        "title": record.title,
        "abstract": record.abstract,
        "authors": list(record.authors),
        "subjects": list(record.subjects),
        "publication_date": record.publication_date,
        "journal": record.journal,
        "publication_type": record.publication_type,
        "language": record.language,
        "query_memberships": sorted(set(record.query_memberships)),
    }
    snapshot_hash = sha256_json(snapshot)
    row = {
        "source_identity_reference": f"ERIC:{record.accession}",
        "eric_accession": record.accession,
        "metadata_revision_reference": f"eric-metadata-sha256:{record.canonical_hash}",
        "metadata_sha256": record.canonical_hash,
        "title_abstract_snapshot_reference": f"title-abstract-sha256:{snapshot_hash}",
        "title_abstract_snapshot_sha256": snapshot_hash,
        "title": record.title,
        "abstract": record.abstract,
        "authors": ";".join(record.authors),
        "subjects": ";".join(record.subjects),
        "publication_date": record.publication_date,
        "journal": record.journal,
        "publication_type": record.publication_type,
        "language": record.language,
        "query_memberships": ";".join(sorted(set(record.query_memberships))),
        "decision": "unresolved",
        "reason_code": "awaitingScreeningReview",
        "reviewer_or_model_reference": "unassigned",
        "candidate_only": False,
        "creates_source_truth": False,
        "creates_source_audit_admission": False,
        "fulltext_retrieved": False,
        "fulltext_verified": False,
    }
    row["decision_reference"] = "screening-decision:" + sha256_json(
        {
            "source_identity_reference": row["source_identity_reference"],
            "metadata_sha256": row["metadata_sha256"],
            "decision": row["decision"],
            "reason_code": row["reason_code"],
        }
    )
    return row


def candidate_assessment_from_screening_row(row: dict[str, Any]) -> dict[str, Any]:
    """Create a deterministic advisory title/abstract assessment.

    The result is candidate-only and cannot alter the authoritative decision.
    """
    title = str(row.get("title") or "")
    abstract = str(row.get("abstract") or "")
    text = f"{title} {abstract}".strip()

    digital_hits = _hits(text, DIGITAL_TERMS)
    sustainability_hits = _hits(text, SUSTAINABILITY_TERMS)
    empirical_hits = _hits(text, EMPIRICAL_TERMS)

    reasons: list[str] = []
    if not abstract.strip():
        candidate = "unresolved"
        reasons.append("inaccessibleAbstract")
        confidence = "insufficient-text"
        margin = 0
    elif digital_hits and sustainability_hits:
        candidate = "probable"
        reasons.extend(["potentiallyRelevant", "requiresFullText"])
        confidence = "high-candidate-relevance" if empirical_hits else "moderate-candidate-relevance"
        margin = len(digital_hits) + len(sustainability_hits) + len(empirical_hits)
    elif not digital_hits and not sustainability_hits and len(text) >= 160:
        candidate = "exclude"
        reasons.extend(["educationContextMismatch", "sustainabilityQuestionMismatch"])
        confidence = "high-candidate-mismatch"
        margin = min(20, max(1, len(text) // 100))
    else:
        candidate = "unresolved"
        reasons.append("insufficientTitleAbstractEvidence")
        confidence = "boundary"
        margin = abs(len(digital_hits) - len(sustainability_hits))

    feature_evidence = {
        "digital_hits": digital_hits,
        "sustainability_hits": sustainability_hits,
        "empirical_hits": empirical_hits,
        "title_length": len(title),
        "abstract_length": len(abstract),
        "publication_type": row.get("publication_type", ""),
        "query_memberships": row.get("query_memberships", ""),
    }
    basis = {
        "source_identity_reference": row["source_identity_reference"],
        "metadata_sha256": row["metadata_sha256"],
        "candidate_decision": candidate,
        "candidate_reason_codes": reasons,
        "feature_evidence": feature_evidence,
    }
    return {
        "assessment_reference": "screening-candidate-assessment:" + sha256_json(basis),
        "source_identity_reference": row["source_identity_reference"],
        "metadata_revision_reference": row["metadata_revision_reference"],
        "candidate_decision": candidate,
        "candidate_reason_codes": reasons,
        "feature_evidence": feature_evidence,
        "feature_evidence_reference": "feature-evidence:" + sha256_json(feature_evidence),
        "confidence_reference": confidence,
        "margin_reference": str(margin),
        "model_or_process_reference": "deterministic-title-abstract-signals:v1",
        "candidate_only": True,
        "explicitly_reviewed": False,
        "creates_screening_decision": False,
        "creates_exclusion": False,
        "creates_source_truth": False,
    }
