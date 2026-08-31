# sensiblaw-rs

From-scratch Rust foundation for SensibLaw, built against the current DASHI/SensibLaw proof and optimisation contracts rather than porting the legacy Python module graph one-for-one.

## Current slice

- `sensiblaw-core`: revision-scoped spans, packed fibre-local sentence carrier, typed head failures, candidate semantic fibres, direct PNF deltas/residuals, natural child-to-parent transport, outward-only paragraph fusion, and fail-closed generation publication.
- `sensiblaw-parity`: opt-in direct/reference certification over consumer-visible semantic observations; it is not part of the mandatory production hot path.
- `sensiblaw-stream`: streaming parser-observation consumer that compiles each closed sentence immediately and stages candidate generations without granting publication authority.
- `python/spacy_stream.py`: replaceable spaCy sidecar using paragraph/sentence-framed TSV; parser output is observation evidence only.
- `python/gwb_prepare.py`: canonical order-independent GWB v0.1 source projection with family-membership validation and source/projected hashes.
- `python/gwb_full_run.py`: lower-level deadlock-safe, preload/hash-verified sustained spaCy→Rust GWB runner.
- `python/gwb_certify.py`: strict user-facing full-GWB certification entrypoint; exposes no subset option and rechecks complete-corpus receipt identity.
- `scripts/bench_stream.py`: concurrent spaCy/Rust benchmark enforcing the published total-walltime gate `T_total <= 2 * T_spaCy_parse`.

The parser sidecar never owns canonical semantic state; Rust owns deterministic compilation and publication boundaries.

## Build

```sh
cargo test --workspace
cargo build --workspace
python3 scripts/bench_stream.py fixtures/sample.txt
# Full GWB: see docs/GWB_TRANCHE.md
```

## Direct-delta execution laws

See `docs/DIRECT_DELTA.md`, `docs/PROOF_OBLIGATIONS.md`, and `docs/GWB_TRANCHE.md`. The current mandatory path is designed around zero sentence-local DB crossings, zero production parser-token writes, zero unchanged-relation writes, and zero parent rescans of closed sentence interiors. Candidate generations remain invisible until separately certified and published.

Direct/reference parity is implemented as an opt-in certification path; it is not part of the mandatory production stream. Full-corpus parity and stricter performance tiers remain run-derived receipts rather than source-code claims.
