# Smith HCA 37 — declared-domain coverage gate (candidate only)

## O/R/C/S/L/P/G/F

- **O** SensibLaw owns official manifestation and PNF evidence; ITIR owns acceptance.
- **R** Source-bound whole-judgment or explicitly declared complete-aligned-span coverage.
- **C** `official_primary_manifestation_coverage_domain` adds a pure assessor over existing v3 comparator references.
- **S** PR #67 pays one selected aligned statutory span, not the whole judgment.
- **L** Official extraction -> partition -> aligned exact spans -> v3 candidate PNF -> aggregate -> review.
- **P** Bind the assessor to persisted extraction lengths/digests and v3 overlap rows, persist and reopen the aggregate receipt.
- **G** No new independent authority, truth, applicability, legal controversy or automatic review.
- **F** Missing spans, incomplete partitions, unverified Other exclusions, semantic mismatch, source/receipt binding and live persistence.

## Important limitation

The new assessor is a deterministic **declared-domain** computation, not a durable
PostgreSQL receipt and not a live-verified whole-judgment result. It checks
coverage of a *caller-declared* partition, including holes, overlaps and
duplicate alignments. It hashes the declared extraction coordinates and
per-span v3 references, but does **not** load and verify those references
against persisted source-provenance rows. The caller must not promote
`complete_declared_domain` to `whole_judgment_complete` until a trusted
producer verifies the source domain against official extraction length/digest,
reopens every v3 candidate batch, and persists/replays the aggregate receipt.

An `Other` mismatch requires a nonempty debris-review reference. Such a
reference is only an assertion at this pure seam; downstream must verify
the referenced review policy and decision. Excluded spans likewise require
explicit review/policy references; exclusion never proves their semantic
equivalence.

## Acceptance commands

```sh
cargo test -p sensiblaw-pg-source-store --test official_primary_manifestation_coverage_domain
cargo test -p sensiblaw-pg-source-store --test official_primary_manifestation_pnf_overlap
cargo test -p sensiblaw-pg-source-store --test official_primary_manifestation_pnf_exhaustive_overlap
cargo test -p sensiblaw-pg-source-store --test smith_hca37_real_matter_live -- --ignored
cargo fmt --all -- --check
```

The ignored live gate requires configured PostgreSQL and reviewed Smith
coordinates; it is **not** a whole-manifestation coverage test. No commands
were executed in this GitHub-only authoring turn.

## Formalisation residual

The Agda Smith interpretation and legal-follow regression modules encode
information-loss and authority boundaries, not the empirical coverage of
PDF/DOCX partitions. No Agda/Lean kernel receipt is claimed. A future
formalisation can specify partition coverage and one-to-one accounting
independently of the Rust assessor, with the live source/receipt binding as
the explicit empirical premise.
