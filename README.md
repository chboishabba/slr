# sensiblaw-rs

Local from-scratch Rust foundation for SensibLaw.

## Current slice

- `sensiblaw-core`: revision-scoped spans, numeric parser projection, typed head failures, candidate semantic fragments, promotion receipt types, deterministic symbol table.
- `sensiblaw-stream`: streaming parser-observation consumer with packed direct sentence compilation, residual emission, outward-only paragraph fusion, and candidate-generation staging.
- `python/spacy_stream.py`: replaceable spaCy sidecar using sentence-framed TSV.
- `scripts/bench_stream.py`: concurrent spaCy/Rust benchmark with the hard `<= 2x spaCy parse walltime` SensibLaw-active-work gate.

The parser sidecar never owns canonical semantic state; Rust owns deterministic compilation and publication boundaries.

## Build

```sh
cargo test --workspace
cargo build --workspace
python3 scripts/bench_stream.py fixtures/sample.txt
```

## Direct-delta execution laws

See `docs/DIRECT_DELTA.md`. The current mandatory path is designed around zero sentence-local DB crossings, zero production parser-token writes, zero unchanged-relation writes, and zero parent rescans of closed sentence interiors. Candidate generations remain invisible until separately certified and published.
