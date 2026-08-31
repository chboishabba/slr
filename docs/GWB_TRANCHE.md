# GWB full tranche v0.1

The GWB proving corpus is run as two explicit phases.

## 1. Source projection

`python/gwb_tranche.py prepare` consumes the historical `tranche-profile:gwb:v0_1`
source inventory. In strict v0.1 mode it requires the two declared source families and
exactly ten unique raw narrative sources after path deduplication: six biography HTML
files plus four books (EPUB/PDF). Derived JSON/timeline/manifest artifacts under the
broad directory are not recursively treated as narrative source documents.

Every projected document records source SHA-256, source byte count, projection method,
projection time, projected SHA-256 and projected byte count. This phase has
`authority = source_projection_only`.

Example:

```sh
python3 python/gwb_tranche.py prepare \
  --inventory /path/to/source_inventory.json \
  --source-root /path/to/SensibLaw \
  --output .tmp/gwb-rust
```

## 2. Direct semantic run

Build release first:

```sh
cargo build --release --workspace
```

Then run the projected corpus through one loaded spaCy model and one long-lived Rust
stream process:

```sh
python3 python/gwb_tranche.py run \
  --manifest .tmp/gwb-rust/source_projection.json \
  --rust-bin target/release/sensiblaw-stream \
  --model en_core_web_sm \
  --output .tmp/gwb-rust/run-receipt.json
```

The receipt is fail-closed. Exit success requires all of:

1. Rust process success.
2. Direct/reference consumer parity for every emitted sentence.
3. Parser evidence published zero semantic generations.
4. `T_total <= 2*T_spaCy_parse` over the sustained full-corpus stream.

The receipt additionally classifies `<=1.5x` and `<=1.2x` performance tiers without
claiming either from a single unreviewed run. Source projection time is deliberately
reported in the projection manifest and excluded from the parser-relative semantic
compiler gate rather than being silently assigned to either parser or post-parser work.

Parity is an opt-in certification path. The ordinary stream remains direct-only unless
it receives the `C\tparity=1` control frame, so the reference compiler is not part of
the mandatory production hot path.
