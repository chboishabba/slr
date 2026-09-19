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

## Canonical parser-observation identity

The first v0.1 strict attempt hashed every frame emitted by `emit_document`, including the
per-document runtime telemetry frame:

```text
M\tspacy_parse_ns=...
```

That made cross-pass raw-stream equality too strong: parity and direct-only passes can
have identical parser observations while legitimately reporting different parse timing.
The failed v0.1 receipt is retained as a negative receipt rather than treated as semantic
failure.

The corrected v0.2 receipt hashes only the declared semantic parser-observation language:

```text
D P S T E Q
```

`M` runtime telemetry and control frames are excluded from semantic observation identity.
They remain separately recorded as performance evidence.

A successful v0.2 receipt therefore requires both passes to have the exact same canonical
parser-observation SHA-256 and hashed byte count, the same direct semantic accounting,
full direct/reference parity on the parity pass, zero publication effects, exact
Rust/controller sentence+paragraph accounting, and the direct-only total semantic pipeline
gate `T_total <= 2*T_spaCy_parse`.

Reference certification cost is not included in the production-speed claim. The receipt
reports framing-active time, direct semantic active time, reference semantic active time,
total pipeline walltime, parser occupancy, post-parser tail, candidate count, residual
count, alternative-fibre count and projection-failure count separately.

The observed v0.1 run on `0833fb4b56a63ee5f9780ad355949b7352b54f25` already established
41,044/41,044 expanded parity, 236,232 candidates, 706,246 residuals, 27,618 alternative
fibres, zero projection failures, zero publication effects, and a 1.0644x direct-only
performance result. Full strict certification remained false only because the old digest
included runtime timing telemetry. The corrected v0.2 digest must still be rerun; the
v0.1 observations do not pre-certify it.

A successful expanded receipt is bounded to the exact code identity, GWB corpus,
projection manifest, spaCy model/version and stable expanded observation surface used by
the run. It does not authorize semantic publication or universal production cutover.
