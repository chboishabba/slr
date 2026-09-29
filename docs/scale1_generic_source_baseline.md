# Generic canonical-source baseline (SCALE-1 / SCALE-2 boundary)

The executable `python/scale1_source_baseline.py` invokes the **existing** Rust
`scale1_source_compiler` example's `compile-source-stdin` operation. It is
not a second compiler or a new book ingestion implementation.

A source adapter first supplies UTF-8 canonical text and stable source,
provider, and acquisition identifiers. The runner passes its **actual**
`--source-family` through unchanged and validates the resulting source
family, exact digest/byte count, reload, source-region accounting, candidate
reopening, parser partition, and nonpromotion to semantic authority.

Example (after building `cargo build --release -p sensiblaw-pg-source-store
--example scale1_source_compiler`):

```sh
python3 python/scale1_source_baseline.py \
  --canonical-input /path/to/adapter-output.txt \
  --source-family chat \
  --source-ref 'source:example:revision1' \
  --provider-ref 'provider:example:1' \
  --acquisition-receipt-ref 'acquisition:example:1' \
  --label 'conversation'
```

The legacy `gwb_scale1_book_baseline.py` remains a **GWB-specific fixture
driver** and PDF/EPUB projector; it must not be adopted as the generic ingestion
interface.

## What this gate does not prove

This gate cannot independently verify authorship, quoted-text ownership,
speaker timing, media offsets, revision lineage, source acquisition, or
source-adapter determinism, because it observes **only the adapter's canonical
text**. Those require source-family preservation witnesses, real adapter
fixtures, and comparison against native source provenance. It also does
not authenticate the benchmark clock or replace same-head economy/worker/archive
acceptance receipts.

No inference of real semantic authority is made from parser completion.
Unresolved semantic regions may be explicitly residual, not silently dropped.
