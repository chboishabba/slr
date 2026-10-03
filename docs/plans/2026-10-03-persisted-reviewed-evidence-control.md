# Persisted reviewed-evidence and acceptance-control plan

## Contract

Normal production and acceptance execution consumes durable PostgreSQL references. JSON is replay/import/export/test-fixture transport only and is never the authority surface for case membership, reviewed support, source selection, consumer context, or expected findings.

The reviewed-evidence boundary is:

```text
persisted observation + consumer requirement + explicit review
    -> persisted reviewed-evidence coordinate
    -> legal IR materialisation
    -> derived/challengeable legal_follow projection
```

A retrieved observation cannot select its own evidentiary role. Source/native identity, exact span, candidate/reviewed PNF coordinates, reviewer receipt and normative legal order remain independently reopenable.

## Invariants

1. `normative_order_ref` is required for every reviewed-evidence coordinate.
2. The coordinate is candidate/review metadata only: it creates no semantic authority, applicability, claim truth, holding, execution permission or access authority.
3. The referenced review receipt must already be persisted and must concern the reviewed observation.
4. The referenced source statement/candidate batch/exact span and reviewed PNF factor must already exist and agree.
5. Legal IR normal-path materialisation starts from `reviewed_evidence_ref`; callers do not supply an ad-hoc in-memory support packet.
6. Cross-order correspondence is never inferred. Crown municipal and Indigenous normative material can coexist but require explicit order coordinates and reviewed cross-order relations.
7. INV/REL acceptance expectations are persisted acceptance contracts, not semantic facts and never alter REL observations/findings.
8. Normal runners accept persisted refs only. Any JSON replay helper must be separately named and must first materialise typed persisted control rows.

## Shared persistence tranche

### Reviewed evidence

Add `semantic.reviewed_evidence_coordinate` keyed by `reviewed_evidence_ref`, retaining:

- review receipt/item/reviewer provenance;
- consumer, requirement and evidence-role refs;
- mandatory normative-order ref;
- source revision, document, statement and exact span refs;
- candidate batch/candidate factor refs;
- reviewed PNF graph/factor/revision/build coordinates;
- proposition/predicate/structural signature;
- legal-system, jurisdiction, temporal and provenance/residual coordinates;
- author/institution coordinates;
- hard non-promotion checks.

Persist/load functions re-open all required upstream rows and fail closed on coordinate disagreement.

### Legal IR

Expose a ref-based materialiser that loads `PersistedReviewedEvidenceCoordinate`, reconstructs the established `ReviewedPropositionSupport`, then calls the existing exact-source/PNF-validated legal-IR materialiser. The direct support-packet helper remains an internal compatibility seam; normal examples use the ref-based entrypoint.

### Acceptance control

Add durable acceptance rows for:

- INV case requests and their source/route/governance/binding coordinates;
- INV acquisition/reopen checks;
- REL corpora, cases, native-product selectors and required/forbidden residual contracts.

The store must distinguish acceptance expectations from semantic truth with explicit non-promotion checks.

## Consumer tranches

1. Retarget INV-CASE-REAL-1 to the shared persistence branch and replace `real-case.json`/reopening JSON execution with persisted request refs.
2. Retarget REL-CORPUS-1 and replace `rel1c-corpus.json` execution with a persisted corpus ref.
3. Use the existing OALC/AustLII/native legal ingest and Mabo formal semantics to materialise the first real case through the generic reviewed-evidence spine. Do not add a Mabo-specific persistence table or an OALC-specific semantic compiler.
4. Run empirical receipts only after genuine source/review/legal-follow rows exist; do not seed synthetic semantic rows merely to make acceptance green.
