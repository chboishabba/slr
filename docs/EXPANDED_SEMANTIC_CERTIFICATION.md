# Expanded semantic certification

This tranche starts from the software-validated semantic-expansion identity
`49c09dfffabadc909c05c0f8db27b051a0c84c01`. It does not alter the meaning of the
older GWB runtime receipt earned by `60777f637732f28fed46458a30853d35b88a8a09`.

The new certification compares two implementations of the richer candidate semantic
surface:

- row/reference projection through `project_sentence`;
- direct sentence-local ordinal-map projection without `project_sentence`.

Both are normalized before comparison. The stable observation retains semantic kind,
scope, revision-scoped source span, fibre address, local head relation, residual kinds,
alternative fibres and projection failures. Transient token IDs are not parity authority.

## Static/compiler validation

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace
python3 scripts/verify_source_contract.py
python3 -m py_compile python/gwb_expanded_certify.py
```

## Full GWB expanded certification

Reuse the already prepared and hash-verified GWB v0.1 projection manifest. No new source
projection is required unless the source corpus itself changed.

```sh
python3 python/gwb_expanded_certify.py \
  --manifest /tmp/opencode/gwb-rust/source_projection.json \
  --rust-bin target/release/sensiblaw-expanded-cert \
  --model en_core_web_sm \
  --output /tmp/opencode/gwb-rust/expanded-semantic-certification.json
```

The driver performs two complete passes with one loaded spaCy model:

1. parity pass: direct + reference expanded compilers;
2. direct-only performance pass: direct expanded compiler only.

Each emitted parser frame is hashed. A successful receipt requires both passes to have
the exact same parser-observation SHA-256 and byte count, the same direct semantic
accounting, full direct/reference parity on the parity pass, zero publication effects,
exact Rust/controller sentence+paragraph accounting, and the direct-only total semantic
pipeline gate `T_total <= 2*T_spaCy_parse`.

Reference certification cost is not included in the production-speed claim. The receipt
reports framing-active time, direct semantic active time, reference semantic active time,
total pipeline walltime, parser occupancy, post-parser tail, candidate count, residual
count, alternative-fibre count and projection-failure count separately.

A successful expanded receipt is bounded to the exact code identity, GWB corpus,
projection manifest, spaCy model/version and stable expanded observation surface used by
the run. It does not authorize semantic publication or universal production cutover.
