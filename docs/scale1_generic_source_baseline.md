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

## SCALE-2 persistent integration (this PR)

### Chat

The retained `corpus.chat_archive_message` source remains the source
authority. Call `prepare_chat_selection_parser_run` with explicitly
selected `ChatStatementCandidateSpan` values, run the existing SCALE-1
PostgreSQL parser worker against its returned parser run reference, and
call `compile_and_persist_chat_from_parser_run` with that reference
and the same selections. The latter uses the existing
`DbNativeParserSnapshot`, generic source-region compiler, canonical
chat statement materializer and candidate PNF store. No separate chat
parser or TSV state is introduced.

The selected span hash is **identical** in the compiler and PG
materializer. The compiler preserves Unicode character offsets and
unselected structural spans. Inactive assistant generations, tool
output and system roles cannot be selected. A parser residual is
reported, not turned into a statement.

### Chat artifact folding

`persist_chat_source_join` and `load_chat_source_joins_for_message`
implement a small, append-only, reference-only instance of
StatiBaker's `chat_source_join_v1` rule. The producer-owned source is
identified by kind, locator, optional event, selected character range,
join type and evidence. The chat canonical bytes are never overwritten.
`exact_digest` verifies the **chat half** of the link; independently
authenticating the producer's event is not claimed.

PG statement selection refuses overlap with an attached backreference.
The source join is not an independent corroborating observation. Other
join strengths (`near_text`, `time_window_tool_call`,
`user_declared`, `heuristic_shape`) remain evidence-qualified and
cannot be promoted by this store. An unregistered pasted artifact is
still an outstanding classification/review obligation, not proof that
the message text is independent authorship.

### WhisperX transcript donor

`plan_whisperx_source` reads the *existing* SensibLaw
`src/sensiblaw/ingest/asr_adapter.py` execution-envelope shape:
`model`, optional `language`, and `segments` with `start`,
`end`, `text`, optional `speaker` and `confidence`.
Extra envelope/segment fields are retained as provider JSON values,
including word-alignment extensions when supplied.

`persist_whisperx_source_plan` writes the canonical transcript text
through `persist_generic_text_source` and the original provider JSON
verbatim into an immutable `ingest.whisperx_execution_source`
provenance sidecar. `reopen_whisperx_source_plan` revalidates both
halves. Audio is not ingested by this adapter; an audio hash is
accepted only when supplied by the capture owner. Transcript source
coverage is **not** audio-level completeness or person identity.

`compile_whisperx_source_lossless` hands literal segment character
intervals into the same `compile_source_regions_lossless` used by
document, mail and selected chat. The generic M12 route remains
candidate-only; any downstream actor mapping or timeline assertion
needs separate authority and review.

The separate, more ambitious normalized TiRCorder session/utterance
packet in `docs/tircorder_connector.md` is **not** falsely treated as
interchangeable with the implemented WhisperX envelope; its
speaker/word/utterance schema needs an explicit versioned bridge.

### Remaining acceptance boundaries

This change is source-written and pushed, not a claim of exact-head
compiler/SQL/CI certification. The PG-native chat prepare/finalize
steps need a running migrated database and finished parser-worker run.
The transcript producer-side packet is persisted and reopens.
`prepare_whisperx_parser_run`, the existing SCALE-1 worker, and
`finalize_whisperx_parser_run` provide the PG-native segment-to-M12
statement/candidate route. The generic persistence layer rechecks
each candidate against reopened canonical text and reopens existing
candidate-store batches. **Actual PostgreSQL execution and source-native
word/utterance parity acceptance remain unverified**; the richer
TiRCorder normalized packet still needs its own adapter.

SCALE-1's separate economy, worker-scaling and archive-scaling receipts,
and PRODUCT review/chronology user stories, remain separate gates.

### Source-neutral candidate durability

`persist_lossless_generic_candidates` provides a single generic persistence
path for completed source-region compilations. It validates all candidate
identities against the reopened `ingest.generic_source_revision` canonical
text, inserts exact source spans into `corpus.span`, persists statements
through the established statement-trace store, and persists/reopens PNF
candidate batches through the existing candidate-product machinery.
Parser residuals do not become statements, and none of the persistence
receipts pays semantic admission or claims truth. This implementation
does **not** claim a global single-transaction corpus commit.

## SCALE-2D — TiRCorder normalized session packet (versioned bridge)

`tircorder_session_bridge` implements
`tircorder-normalized-session.v1` as an **explicitly separate producer
contract** from the deployed SensibLaw WhisperX `asr_adapter.py` envelope.
Its source donor is the *session/utterance packet* at the start of
`SensibLaw/docs/tircorder_connector.md`; it intentionally does not ingest
the unrelated graph-oriented packet appended later in that document.

`plan_tircorder_sessions` retains the original producer packet and each
session's ID, capture-device ID, optional audio digest, utterance ID,
ordered text, timestamp strings, optional speaker label/confidence, word
alignment, native sentence splits, and extra provider fields. The
`source_revision_ref` identifies the immutable capture packet/session
presentation. No speaker label is promoted to a person and no transcript
is called complete relative to audio. No word alignment is fabricated.

Native sentence splits are accepted only when ranges fit and do not
overlap the actual Unicode character text. Supplied gaps are structural
regions; absent splits fall back conservatively to the entire native
utterance as a parser **candidate** region, not a verified sentence.
Projection-inserted separators are also structural. These enter the
existing `compile_source_regions_lossless` classification, not a
TiRCorder-specific M12 implementation.

`persist_tircorder_session` records the canonical text through the
existing generic source store and the original packet as an immutable
source sidecar; `reopen_tircorder_session` reconstructs and checks the
native region plan against the reopened source. Producer-scoped batch
identity rejects a changed packet under an existing batch ID.
`prepare_tircorder_parser_run` / the ordinary PG parser worker /
`finalize_tircorder_parser_run` reuse the same job, M12 and candidate
persistence owners as chat and WhisperX.

This is an integration of the **documented** TiRCorder packet contract.
It is not evidence that TiRCorder's active deployed capture service
currently emits that exact JSON variant. That requires an actual
producer fixture and integration receipt.

## SCALE-2E — cross-family exact correspondence

`persist_exact_transcript_chat_quote` reopens both the immutable chat
message and the independently stored native WhisperX segment or TiRCorder
utterance. Only a byte-exact UTF-8 match of the selected character span to
the original native event text allows an `exact_digest` chat-source join.
The join carries the original transcript revision ID and native event ID,
and returns `independent_witness_increment = 0`. It does **not** create
a new transcript, observational authority or corroborating statement.

Near-text, heuristic and user-declared references remain weaker
StatiBaker-style joins and do not pass through this exact-correspondence
function. Operational state and tool activity remain observer inputs
rather than content-bearing world facts. PRODUCT chronology, uncertainty,
and review acceptance remain separate work.

None of the above claims runtime certification: the new Rust/SQL source
routes remain uncompiled/unexecuted in this integration tranche.
