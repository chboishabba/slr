# sensiblaw-rs

From-scratch Rust foundation for SensibLaw, built against the current DASHI/SensibLaw proof and optimisation contracts rather than porting the legacy Python module graph one-for-one.

## Current slice

- `sensiblaw-core`: revision-scoped spans, packed fibre-local sentence carrier, typed head failures, candidate semantic fibres, direct PNF deltas/residuals, natural child-to-parent transport, outward-only paragraph fusion, and fail-closed generation publication.
- `sensiblaw-parity`: opt-in direct/reference certification over consumer-visible semantic observations; it is not part of the mandatory production hot path.
- `sensiblaw-semantic-expansion`: richer candidate-only semantic surface with explicit scope/attachment residuals and retained alternative fibres.
- `sensiblaw-semantic-admission`: fail-closed candidate→admitted-local-delta boundary requiring an explicit non-parser resolution receipt; rejected/unresolved fibres retain evidence.
- `sensiblaw-expanded-cert`: expanded semantic parity/performance runner plus typed residual-frontier accounting.
- `sensiblaw-stream`: streaming parser-observation consumer that compiles each closed sentence immediately and stages candidate generations without granting publication authority.
- `fixtures/legal_semantic_conformance_v0_1.tsv`: exact gold consumer-object fixtures plus explicit producer gaps.
- `python/spacy_stream.py`: replaceable spaCy sidecar using paragraph/sentence-framed TSV; parser output is observation evidence only.
- `python/gwb_prepare.py`: canonical order-independent GWB v0.1 source projection with family-membership validation and source/projected hashes.
- `python/gwb_full_run.py`: lower-level deadlock-safe, preload/hash-verified sustained spaCy→Rust GWB runner.
- `python/gwb_certify.py`: strict user-facing full-GWB certification entrypoint; exposes no subset option and rechecks complete-corpus receipt identity.
- `python/gwb_expanded_certify.py`: two-pass expanded semantic certification; v0.3 also requires exact residual-kind accounting across passes.
- `scripts/bench_stream.py`: concurrent spaCy/Rust benchmark enforcing the published total-walltime gate `T_total <= 2 * T_spaCy_parse`.

The parser sidecar never owns canonical semantic state; Rust owns deterministic compilation and publication boundaries.

## Build

```sh
cargo test --workspace
cargo build --workspace
python3 scripts/verify_source_contract.py
# Full GWB / semantic frontier runs: see docs/GWB_TRANCHE.md and docs/LEGAL_SEMANTIC_ADMISSION.md
```

## Direct-delta execution laws

See `docs/DIRECT_DELTA.md`, `docs/PROOF_OBLIGATIONS.md`, `docs/GWB_TRANCHE.md`, and `docs/LEGAL_SEMANTIC_ADMISSION.md`. The mandatory direct path is designed around zero sentence-local DB crossings, zero production parser-token writes, zero unchanged-relation writes, and zero parent rescans of closed sentence interiors.

Candidate semantics are not legal authority. The admission layer requires an exact candidate-matched resolution receipt, preserves unresolved/rejected evidence, and still exposes no publication API. Residual counts and priority scores are workflow diagnostics only, never semantic confidence or truth scores.
