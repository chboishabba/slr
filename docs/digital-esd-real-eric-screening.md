# Digital-ESD real ERIC screening runbook

This is the operational Digital-ESD path over the retained ERIC Q1-Q7 exports.

## Authority boundary

```text
ERIC metadata parse
!= screening decision
!= full-text retrieval
!= source truth
!= SourceAuditAdmission
```

Candidate assessment, duplicate/report-family hypotheses, calibration selection
and Pareto priority are reviewer-work allocation surfaces only.

## 1. Parse the retained ERIC studies

Expected retained layout:

```text
artifacts/digital-esd/eric/
  Q1/
    page-000000.json
    ...
    summary.json
  ...
  Q7/
```

Run:

```bash
python3 scripts/run_digital_esd_real_eric.py \
  --export-root artifacts/digital-esd/eric \
  --output-root artifacts/digital-esd/real-eric \
  --expect-occurrences 46597 \
  --expect-unique 43996 \
  --json
```

This performs:

```text
retained response.docs
-> page SHA / pagination validation
-> ERIC field normalisation
-> 46,597 query occurrences
-> accession dedup
-> 43,996 unique metadata studies
-> 43,996 unresolved screening rows
-> candidate assessments
-> candidate family hypotheses
-> calibration worklist
-> Pareto review queue
```

No authoritative include/exclude decision is created.

## 2. Compile an observed Agda execution witness

Only after the real execution receipt exists:

```bash
python3 interop_scripts/emit_digital_esd_eric_execution_agda.py \
  --receipt artifacts/digital-esd/real-eric/execution_receipt.json \
  --output DASHI/Generated/DigitalESDERICStudyExecutionObserved.agda
```

The compiler re-opens every artifact bound by the receipt and re-computes its
SHA-256 before emitting the Agda value. Missing or drifted artifacts fail.

The generated value inhabits the canonical
`DASHI.Education.DigitalESDERICStudyExecutionExact.RealERICStudyExecutionReceipt`
type; it does not redefine a second receipt ontology.

## 3. Review a bounded tranche

Reviewers work from:

```text
artifacts/digital-esd/real-eric/calibration_selection.jsonl
artifacts/digital-esd/real-eric/screening_pareto_queue.jsonl
```

Write an explicit decision overlay, for example:

```json
{
  "source_identity_reference": "ERIC:EJ1234567",
  "reviewed": true,
  "decision": "include",
  "reason_code": "potentiallyRelevant",
  "reviewer_or_process_reference": "reviewer:johl",
  "decision_timestamp": "2026-09-20T12:00:00+10:00"
}
```

Allowed authoritative decisions:

```text
include
probable
exclude
unresolved
```

Candidate/model output cannot be supplied with `reviewed=false` and promoted.

## 4. Apply reviewed decisions without changing the denominator

```bash
python3 scripts/apply_digital_esd_screening_decisions.py \
  --ledger artifacts/digital-esd/real-eric/screening_ledger.tsv \
  --decisions artifacts/digital-esd/review/reviewer-decisions.jsonl \
  --output artifacts/digital-esd/review/screening_ledger.tsv
```

Unmentioned records remain unchanged. Unknown source identities fail. The output
must contain the same number of source identities as the input.

## 5. Refresh calibration diagnostics and the review queue

```bash
python3 scripts/refresh_digital_esd_review_queue.py \
  --ledger artifacts/digital-esd/review/screening_ledger.tsv \
  --assessments artifacts/digital-esd/real-eric/candidate_assessments.jsonl \
  --hypotheses artifacts/digital-esd/real-eric/study_family_hypotheses.jsonl \
  --output-dir artifacts/digital-esd/review
```

The calibration estimate is computed only over explicitly reviewed rows.
False-negative and disagreement values are reviewed-subset diagnostics, not
population truth and not automatic thresholds.

The refreshed Pareto queue contains only still-unresolved records.

## 6. Retrieve full text only for authoritative include/probable rows

The retrieval manifest is JSONL:

```json
{
  "source_identity_reference": "ERIC:EJ1234567",
  "artifact_path": "/retained/fulltext/EJ1234567.pdf",
  "sha256": "<64 hex characters>",
  "retrieval_reference": "retrieval:publisher-or-repository-receipt"
}
```

Then:

```bash
python3 scripts/prepare_digital_esd_fulltext_index.py \
  --ledger artifacts/digital-esd/review/screening_ledger.tsv \
  --retrieved-manifest artifacts/digital-esd/fulltext/retrieved.jsonl \
  --output-dir artifacts/digital-esd/real-eric/fulltext \
  --json
```

A study is `verified` only when:

```text
decision = include | probable
+
real artifact exists
+
observed SHA-256 = retrieval-manifest SHA-256
```

Missing artifacts remain pending. Metadata/accession IDs are never substituted
for full text.

## 7. Summarize P0-A through P0-G

```bash
python3 scripts/run_digital_esd_adaptive_screening.py \
  --artifact-dir artifacts/digital-esd/real-eric \
  --json
```

P0-A remains paid as long as every record stays accounted for in exactly one
authoritative screening state. Reviewed records leave the work queue but never
leave the denominator.

## 8. Downstream SLR / source audit

Only verified retained/probable full-text artifacts proceed to the canonical
evidence lane:

```text
full-text artifact + SHA-256
-> EvidenceManifestation
-> EvidenceSourceRevision
-> EvidenceSpan
-> EvidenceObservation
-> review
-> SLR review packet
-> independent SourceAuditAdmission
```

Digital-ESD screening/retrieval does not construct SourceAuditAdmission.
