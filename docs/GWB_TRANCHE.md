# GWB full tranche v0.1

The GWB proving corpus is run as two explicit authority-separated phases.

## 1. Canonical source projection

Use `python/gwb_prepare.py`. It consumes the historical `tranche-profile:gwb:v0_1`
source inventory and tracks all family memberships per resolved path, so overlapping
families may appear in any inventory order without changing the admitted corpus.

The strict v0.1 payload is exactly ten unique raw narrative sources:

- six biography HTML files, each belonging to `source-family:gwb-public-bios:v1`;
- four EPUB/PDF books, each belonging to `source-family:gwb-books:v1`.

Derived JSON/timeline/manifest artifacts are not recursively treated as narrative source
documents. Deterministic document ordinals are assigned by source kind and resolved path,
not manifest ordering.

Every projected document records source SHA-256, source byte count, all family refs,
projection method, projection time, projected SHA-256 and projected byte count. This
phase has `authority = source_projection_only`.

```sh
python3 python/gwb_prepare.py \
  --inventory /path/to/source_inventory.json \
  --source-root /path/to/SensibLaw \
  --output .tmp/gwb-rust
```

## 2. Full certification run

Build release first:

```sh
cargo build --release --workspace
```

The user-facing full-tranche command is `python/gwb_certify.py`. It intentionally has
no subset/limit flag. It drives the lower-level `gwb_full_run.py` implementation and
then independently checks that the receipt covers the complete projection manifest.

```sh
python3 python/gwb_certify.py \
  --manifest .tmp/gwb-rust/source_projection.json \
  --rust-bin target/release/sensiblaw-stream \
  --model en_core_web_sm \
  --output .tmp/gwb-rust/full-certification.json
```

A successful full certification requires all of:

1. Every projected text is preloaded and its SHA-256 + byte count are reverified before timing starts.
2. Rust process success.
3. Direct/reference consumer parity for every emitted sentence.
4. Controller and Rust sentence/paragraph accounting agree.
5. Parser evidence published zero semantic generations.
6. `T_total <= 2*T_spaCy_parse` over the sustained full-corpus stream.
7. Receipt document count equals the prepared manifest document count.

The canonical receipt schema is:

```text
sensiblaw.gwb-full-certification-receipt.v0_1
```

The lower-level `gwb_full_run.py` and historical combined `gwb_tranche.py` are
implementation/debug surfaces. A debug subset must not be treated as tranche
certification merely because it produced JSON; `gwb_certify.py` is the complete-corpus
authority gate.

## Timing hygiene

Source projection and spaCy model cold-load are explicitly reported outside the
parser-relative semantic performance gate. All projected text is read and verified
before the Rust stream receives its first document frame, preventing disk I/O from
being charged to later document parsing. Rust sentence chatter is sent to `DEVNULL`
and tranche diagnostics use a file-backed stderr handle, so a large corpus or a large
parity-failure set cannot fill an unread pipe and deadlock the run.

The receipt classifies `<=1.5x` and `<=1.2x` tiers but does not promote those tiers from
source code alone. The run receipt is the evidence.

Parity remains opt-in at the Rust stream level (`C\tparity=1`); ordinary production
streaming remains direct-only, so the reference compiler is not part of the mandatory
hot path.
