# Digital-ESD live retrieval and real-study parsing

**Status:** executable live-network path for the Digital-ESD application workload.

This document resolves the retrieval questions that remained after the
43,996-record metadata/screening pipeline and ESD-4 scholarly parser landed.

The authority boundary remains:

```text
fetch success
!= screening decision
!= source truth
!= SourceAuditAdmission

parser success
!= review
!= SourceAuditAdmission
```

## 1. ERIC API authentication

No API key is required or transmitted by the documented public ERIC API path
used here.

Endpoint:

```text
https://api.ies.ed.gov/eric/
```

The live fetcher retains the old `--api-key` argument only for compatibility;
it is deprecated and never included in requests.

## 2. Exact Q1-Q7 query authority

The canonical query set is checked in at:

`fixtures/digital_esd_eric_queries.json`

Its strings are copied from the formal DASHI owner:

`DASHI/Education/DigitalESDDatabaseTranslatedQueriesExact.agda`

on the Digital-ESD methodology branch.

The fetcher hashes the exact unencoded query and records it in each retained
query summary.

## 3. ERIC metadata retrieval

Network access is opt-in:

```bash
python3 scripts/digital_esd_fetcher.py eric --live
```

Smoke-test one query without coupling it to Q4:

```bash
python3 scripts/digital_esd_fetcher.py eric \
  --live \
  --query Q1 \
  --force
```

The public API is called using:

```text
search=<exact frozen query>
rows=200
format=json
start=<offset>
```

with a default four-second request interval.

Per query the retained layout is:

```text
artifacts/digital-esd/eric/Q1/page-000000.json
artifacts/digital-esd/eric/Q1/page-000001.json
...
artifacts/digital-esd/eric/Q1/summary.json
```

`summary.json` retains:

- exact canonical query;
- query SHA-256;
- request URLs;
- observed `numFound`;
- page paths and SHA-256 digests;
- pagination completeness;
- execution timestamps.

Historical 2026-09-19 counts are retained as provenance only:

```text
Q1    642
Q2    290
Q3   1594
Q4  41889
Q5    214
Q6    293
Q7   1675
```

A fresh execution may legitimately drift as ERIC changes. Historical count
mismatch is not automatically treated as corruption; retained page hashes and
the execution summary describe the actual run.

## 4. Network policy

All new network traffic is gated behind `--live`.

Without `--live`:

- ERIC retrieval writes no network result;
- full-text retrieval writes no artifact;
- the existing offline research/query paths remain zero-network.

The live path defaults to one request every four seconds.

## 5. Screening authority

Machine candidate assessments and Pareto ranking do not select the retained
study corpus.

Authoritative decisions are applied only through:

`scripts/apply_digital_esd_screening_decisions.py`

A decision overlay row must explicitly carry:

```json
{
  "source_identity_reference": "ERIC:EJ...",
  "reviewed": true,
  "decision": "include",
  "reason_code": "...",
  "reviewer_or_model_reference": "...",
  "decision_timestamp": "..."
}
```

Allowed decisions remain:

`include | probable | exclude | unresolved`

The default storage is filesystem-first TSV/JSONL. This live retrieval tranche
does not write screening state to PostgreSQL.

Review packets/templates remain available through:

`scripts/prepare_digital_esd_review_packets.py`

## 6. Full-text eligibility

Only authoritative `include|probable` screening rows enter the worklist.

`scripts/prepare_digital_esd_fulltext_index.py` now emits:

`artifacts/digital-esd/fulltext/fulltext-worklist.jsonl`

Excluded and unresolved records cannot enter a fetch batch.

## 7. Full-text sources

Default automatic retrieval is deliberately narrow:

```text
https://files.eric.ed.gov/fulltext/<ERIC-accession>.pdf
```

for `ERIC:ED...` / `ERIC:EJ...` identities.

Publisher or institutional URLs are not crawled by default.

An explicit URL map may be supplied:

```json
{"source_identity_reference":"ERIC:EJ...","url":"https://host.example/paper.pdf"}
```

but the host must also be explicitly allowlisted with `--allow-host`.

This prevents an ERIC metadata record from silently becoming permission to
crawl arbitrary publisher/authentication surfaces.

## 8. Full-text cache envelope

The existing conservative defaults are retained:

```text
batch size          20
cache cap           2 GiB
free-space reserve  5 GiB
planning size       10 MiB/paper
```

Default cache:

`artifacts/digital-esd/fulltext/cache`

The location remains overrideable; no NFS/TrueNAS dependency is required for
the first real slice.

The cache planner now correctly treats the reserve as a free-space condition,
not as bytes subtracted from the cache cap.

## 9. Retrieval receipt

Successful full-text downloads are merged into:

`artifacts/digital-esd/fulltext/retrieved-manifest.jsonl`

Each row retains:

```text
source_identity_reference
artifact_path
sha256
source_revision_reference = fulltext-sha256:<digest>
retrieval_reference
retrieval_timestamp
candidate_only = true
creates_source_truth = false
creates_source_audit_admission = false
```

## 10. Registration and same-object weld

`interop_scripts/digital_esd_fulltext_cache.py register`

rehashes the exact artifact.

If a retrieval receipt exists, its SHA-256 must equal the observed bytes or
registration fails closed.

Registered rows now retain:

```text
source_revision_reference
content_sha256
retrieval_reference
artifact_path
```

so the next scholarly parser handoff is same-object rather than title-based.

## 11. ESD-4 parser

The first validation format is PDF using the parser already in the repo.

No PyMuPDF dependency is introduced in this tranche.

The existing ESD-4 wrapper accepts the registered/handoff ledger:

`interop_scripts/digital_esd/scholarly_fulltext.py`

and verifies the file digest before preparing a parse request.

The pre-Sprint-2/Python parser remains candidate-only parsing machinery; parse
success creates no review or semantic authority.

## 12. Thin end-to-end reviewed-study controller

Use:

```bash
python3 scripts/run_digital_esd_reviewed_study_ingestion.py \
  --ledger artifacts/digital-esd/real-eric/screening_ledger_reviewed.tsv
```

without `--live` to inspect the bounded plan.

To permit actual download:

```bash
python3 scripts/run_digital_esd_reviewed_study_ingestion.py \
  --ledger artifacts/digital-esd/real-eric/screening_ledger_reviewed.tsv \
  --live
```

The controller performs:

```text
reviewed ledger
-> include/probable worklist
-> cache-envelope plan
-> full-text fetch
-> retrieval manifest
-> exact digest/revision registration
-> P0-G full-text verification
-> ESD-4 scholarly parse
-> parser verification receipts
```

If the authoritative ledger has zero `include|probable` decisions, it stops
with:

`awaiting-authoritative-include-probable-decisions`

and parses zero papers.

The first real ESD-4 slice therefore needs no hard-coded accession list: the
bounded batch is drawn from the actual reviewed retained set, with the existing
priority queue used when present.

## 13. Artifact-root convention

The live lane uses one canonical application artifact root:

`artifacts/digital-esd/`

The earlier standalone underscore default
`artifacts/digital_esd/` has been removed from the full-text gate.

## 14. What is not added

This tranche does not add:

- a new PDF parser architecture;
- automatic publisher crawling;
- authenticated publisher sessions;
- PostgreSQL screening writes;
- automatic screening decisions;
- automatic SourceAuditAdmission;
- automatic legal/source authority;
- a requirement for an ERIC API secret.

## 15. Verification

Targeted regressions:

```bash
pytest -q \
  tests/test_digital_esd_fetcher.py \
  tests/test_digital_esd_live_retrieval_pipeline.py \
  tests/test_digital_esd_eric_real_exports.py \
  interop_scripts/digital_esd/tests/test_scholarly_fulltext.py
```

Then run the normal workspace tests / clippy before merge.

A GitHub status or draft-review bot is not a substitute for these execution
receipts.
