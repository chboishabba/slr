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

## Shared region execution (Rust)

`generic_source_compiler::compile_source_regions_lossless` is now the
single lossless semantic region loop used by document and mail adapters.
Semantic/transport/structural classification remains adapter-owned.
The engine explicitly rejects duplicate region identity, incompatible
revisions and invalid bounds; semantic parse errors are retained as
parser residuals, never interpreted as absent source evidence.

The engine builds a sparse UTF-8 character-to-byte offset lookup once per
canonical text, avoiding repeated document-prefix scans for each semantic
region.

`compile_chat_message_lossless` now accepts persisted chat sources **and
explicit ChatStatementCandidateSpan selections**. Neither every message nor
every assistant response becomes an independent statement automatically.
Selected spans must match the original Unicode character positions and
literal bytes; overlapping, misattributed, or forged selections fail closed.
Unselected message intervals are kept in the structural accounting partition.
Inactive generated branches, tool outputs and system text cannot be selected
for statement compilation through this helper.

This is a Rust in-memory M12 compiler weld. It does not claim that the
existing chat archive PostgreSQL importer already invokes the weld, nor
that SensibLaw's separate WhisperX/TiRCorder source adapter is present
in this crate. Running the integration through persistent SQL is an
additional acceptance task.
