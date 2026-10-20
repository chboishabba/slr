# Smith HCA 37 — document-pair declared-domain coverage (candidate only)

## Roadmap and O/R/C/S/L/P/G/F

- **O** SLR owns official extraction, exact slices, candidate-PNF, review and transformation receipts. ITIR owns suite acceptance.
- **R** Auditable comparator-scoped PDF↔DOCX **document-pair** coverage for the same official judgment.
- **C** `official_primary_manifestation_coverage_domain.rs` assesses independently complete source partitions and many-to-many alignment groups. Existing v3 selected-span comparison remains an immutable witness.
- **S** Runner verified one selected statutory span, 22/22 semantic factors; whole judgment remains unmeasured.
- **L** Two official extractions -> two complete partitions -> many-to-many groups -> candidate-PNF/v3 witnesses -> document-pair aggregate -> review.
- **P** Bind declared coordinates to persisted official extraction and per-group comparison rows, then persist/reopen immutable aggregate.
- **G** Candidate/provenance diagnostics only; never legal controversy, authority, applicability, truth, independent corroboration or consumer-informativeness payment.
- **F** Official extraction binding, complete live Smith partitions, verified grouped PNF comparison, reviewed Other/exclusion records, PG aggregate/reopen and Agda empirical coverage proof.

## Exact semantics

`assess_document_pair_manifestation_coverage` emits a **single document-pair**
digest over both declared extraction identities, SHA-256 values, parser
revisions, document lengths, interval partitions, grouped span membership,
candidate batch refs, comparator revision, comparison receipt refs and review
coordinates. Each side's intervals must independently partition its declared
document length without gaps or overlaps. Each interval must appear in
**exactly one** comparison group or reviewed exclusion. Groups may include
one or many spans from each side, so offset and segmentation differences do
not force one-to-one matching.

A group marked `semantic_multiset_exact` is a caller-supplied v3-scoped
comparison outcome, **not** an executed grouped semantic compiler. Group
candidate ancestry and v3 receipt references are required but are not yet
independently reopened. The legacy singleton function remains for historical
callers and never establishes whole-document scope by itself.

A nonsemantic `Other` mismatch requires an explicit debris-review reference;
non-debris or unreviewed differences remain residuals. Reviewed exclusions
must carry policy and review references, but those references are **not yet
verified** against the review store. A pure successful receipt only says
`complete_declared_domain`, not `whole_judgment_complete`.

## Remaining producer/persistence acceptance

A future PostgreSQL-bound operator must:

1. Reopen both persisted official extraction refs and verify actual bytes,
   digest, extraction identity, length, document/manifestation relationship
   and parser revision; never trust caller assertions.
2. Produce a complete independent partition of **each full extraction** and
   verify every exact slice's byte/span ancestry.
3. Reopen every candidate batch and v3 comparison, and aggregate PNF factors
   across each many-to-many group using the **existing** semantic comparator.
   Do not infer grouped equality from a list of singleton equality booleans.
4. Reopen human review decisions for nonsemantic debris and exclusions; leave
   unreviewed material as typed residual.
5. Persist a versioned document-pair receipt with immutable coordinates,
   revalidate on reopen, and reject identity conflicts or stale extraction
   digests. A successful live Smith receipt is still outstanding.

## Intended local commands (not executed in this GitHub authoring turn)

```sh
cargo test -p sensiblaw-pg-source-store --test official_primary_manifestation_coverage_domain
cargo test -p sensiblaw-pg-source-store --test official_primary_manifestation_pnf_overlap
cargo test -p sensiblaw-pg-source-store --test official_primary_manifestation_pnf_exhaustive_overlap
cargo test -p sensiblaw-pg-source-store --test smith_hca37_real_matter_live -- --ignored
cargo fmt --all -- --check
```

The ignored real-matter test requires PostgreSQL and reviewed Smith coordinates;
it is not a whole-document acceptance gate.

## Multi-view and formalisation boundaries

ITIR's world-model convergence roadmap says projections retain source/version,
role-predicate shape, omissions/additions, authority class and rebuildability;
comparisons emit diagnostic residuals. Simple Wikipedia supplies
simplification pressure and Abstract Wikipedia constructor-level pressure.
Neither makes PDF/DOCX agreement independent corroboration or establishes
consumer-informativeness parity. DASHI Smith interpretation/legal-follow
regressions bound coarse information loss and non-promotion; there is **no
Agda/Lean kernel receipt** for full Smith document-partition coverage.

Some historical SensibLaw context files referenced by the controller were
not available at their requested GitHub paths; no claim of reviewing their
exact line ranges is made.


## DB-native document-pair producer (implementation tranche)

`official_primary_manifestation_document_pnf_overlap.rs` now adds
`compare_and_persist_document_pnf_pair` and `load_document_pnf_pair`.
The producer reopens the official manifestation comparison, canonical source
revisions and text, lossless long-document compilation receipts, exact candidate
persistence stage and **all** statement candidate batches for each document.
It reuses the existing v3 `measure_semantic_pnf_coverage` semantics on the
concatenated per-document factor inventory; sentence spans are ancestry only.

A new PostgreSQL `source_provenance.official_primary_document_pnf_overlap`
table binds the extraction refs/digests, source revisions, parser run refs,
candidate batch inventories and ancestry digests, comparator/projection,
semantic counts, parser residual counts, review coordinates and non-promotion
flags. Insert is idempotent and the read path recomputes the expected receipt
from reopened source/compilation/candidate evidence before comparing columns.
A different immutable coordinate under the same document-pair unique key
cannot silently replace the previous row.

**Corrected v3 semantics:** `semantic_pnf_overlap_closed` follows the existing
non-`Other` semantic multiset comparator, provided parser coverage is complete.
Unequal `Other` counts remain countable provenance residuals and do **not**
make semantic PNF overlap unequal. `transformation_closed` is a distinct,
stricter review-dependent conclusion: unresolved `Other` materiality still
blocks it. A caller-supplied review reference does not pay that obligation;
only an independently reopened established nonsemantic-debris disposition
could eventually permit closure. Parser residuals remain conservative.

**Conservative closure rule:** any parser residual or any unclassified `Other` factor
prevents full transformation closure. Review refs are retained but are not
interpreted as authorising omission. The live producer currently passes no
review refs. This deliberately leaves potential layout debris unresolved
rather than falsely declaring it nonsemantic.

**Unpaid/known limitations:** no live Smith full-document compilation receipt
was supplied in this turn; the operator requires those persisted source
revisions and exact candidate stage rows. The producer does not generate
parser jobs or run the parser itself. A complete document-level multiset
match is exact under v3 projection, not proof of proposition correspondence,
consumer informativeness, source truth or legal equivalence. A formal Agda
empirical coverage kernel receipt is also absent.

### Local commands — intended, not executed here

```sh
cargo test -p sensiblaw-pg-source-store --test official_primary_manifestation_document_pnf_overlap
cargo test -p sensiblaw-pg-source-store --test official_primary_manifestation_pnf_exhaustive_overlap
cargo test -p sensiblaw-pg-source-store --test official_primary_manifestation_pnf_overlap
cargo test -p sensiblaw-pg-source-store --lib db_native_long_document
cargo test -p sensiblaw-pg-source-store --lib generic_source_compiler
cargo fmt --all -- --check
cargo clippy -p sensiblaw-pg-source-store --all-targets -- -D warnings
```

### Ignored live Smith PG gate

```sh
DATABASE_URL=... \
ITIR_SMITH_HCA37_MANIFESTATION_COMPARISON_REF=... \
ITIR_SMITH_HCA37_PDF_SOURCE_REVISION_REF=... \
ITIR_SMITH_HCA37_DOCX_SOURCE_REVISION_REF=... \
cargo test -p sensiblaw-pg-source-store --test official_primary_manifestation_document_pnf_overlap -- --ignored
```

Requires the real official extraction comparison, source revisions whose
acquisition receipts match each extraction, full lossless compilation and
single exact parser-stage candidate inventories. The selected statutory
span's v3 receipt is preserved separately. No tests, PG replay or kernel
verification were executed by this GitHub-only authoring turn.
