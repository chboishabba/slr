# X369 — SLR-native PostgreSQL grammar-to-Leech handoff

**Owner:** `chboishabba/slr` / `sensiblaw-pg-source-store` (Rust production persistence).
**Golden semantic contracts:** DASHI/Agda. **Consumer:** LeechTransformer PR #4.
**Status:** Source projection implemented; live original TinyStories source export not yet observed. No new storage or parser introduced.

## Actual PG interconnect (inspected, not hypothetical)

The native Rust service persists source bytes in `corpus.canonical_content.payload`,
joined via `corpus.document` and `ingest.generic_source_revision`.
The full native parser authority is `ingest.parser_run` → `ingest.parser_job` →
`ingest.parser_token`, defined in `crates/sl-pg-source-store/src/db_native_parser.rs`.
Its token primary key is **(`compilation_key`,`token_ordinal`)**. It stores
`start_char`, `end_char`, `surface`, `lemma`, `pos`, `dependency_ref`,
nullable `morph_json`, and nullable `head_ordinal`. Jobs carry source region,
revision, parser status and candidate-only flags; parser runs bind model/config
identity and source revision. The canonical text is already SHA256-bound by
`GenericSourceContentStore`. **No `execution.semantic_parser_*` table is
consulted by the native SLR exporter**: that is a *different legacy SensibLaw*
PostgreSQL schema with a separate adapter/producer (SensibLaw PR #489).

## Actual source projection

`python/slr_pg_leech_grammar_export.py` is a **read-only export boundary**
for the existing Rust PG state, not another Python parser or authoritative
state engine. It selects `ingest.parser_run`, joins canonical source
`corpus.canonical_content`, and reads all succeeded parser jobs and tokens
from **one `REPEATABLE READ READ ONLY` snapshot**. The SLR production Rust
writer and PostgreSQL tables remain the authority.

The JSONL `slr.numeric_spacy.leech_grammar.v1` carries: canonical
`document_id`, source revision/parser-run refs, full UTF8 original source
text and its SHA256, parser model/contract and token-structured receipt
digests, source-Unicode character spans, source token occurrence refs, POS/DEP
and optional morphology stable SHA256 labels, exact original head links,
and explicit **`world_resolution_deferred=True` /
`parser_observation_is_semantic_authority=False`**.

The Rust parser token key is composite; `token_id=1..N` in the Leech export
is an **export-local positive occurrence rank**, NOT a fictitious native PG
BIGINT token key. The exact real `(compilation_key,token_ordinal)` is
retained on each token, and each head is resolved within the **same
compilation region**. Numeric POS/DEP/MORPH label IDs are likewise
export-local ranks; **full SHA256 category digests** are the portable
cross-run label identity used by Leech's train-only compact vocabulary.

Native `morph_json` and `head_ordinal` are nullable. Their absence
remains absent/masked, NOT a fabricated morphology or "neutral zero".
The consumer now supports the class-0-only morph observation channel.

## Physical manifest and run

This manifest references **existing source and parser rows**, not external
re-parsed text. Source bytes are reopened directly from PostgreSQL.

```json
[
  {"source_revision_ref":"<original-train-revision>",
   "parser_run_ref":"<original-complete-run>",
   "split":"train","source_sha256":"<64 lowercase hex SHA256>"},
  {"source_revision_ref":"<source-disjoint-development-revision>",
   "parser_run_ref":"<different-complete-run>",
   "split":"development","source_sha256":"<64 lowercase hex SHA256>"}
]
```

On the SLR host (no password in argv):

```bash
DATABASE_URL="$DATABASE_URL" python3 -m python.slr_pg_leech_grammar_export \
  --manifest "$PINNED_NATIVE_SLR_MANIFEST" \
  --output "$NEW_OUTPUT_ROOT/slr_original_train_dev.jsonl"

python3 -m unittest discover -s python/tests \
  -p 'test_slr_pg_leech_grammar_export.py' -v
```

Then Leech PR #4 physical runner uses `--slr-repo`, `--slr-python`
and `--slr-revision` to select the native SLR authority instead of the
legacy `--sensiblaw-*` fallback. Both connect to the same Leech JSONL
observation contract, but **their PG tables and occurrence identities are
different and are never interchanged**.

## Exact remaining gates

| Gate | Meaning | State |
|---|---|---|
| S0 | Native canonical source, parser job/token schema inspected | **Closed (source)** |
| S1 | Strict export from existing native PG rows, no new schema | **Source implemented** |
| S2 | SLR source/manifest/Unicode/head/F5 regression tests | **Written; host run open** |
| S3 | Physical PG completed parser runs for original train/dev TinyStories | **Open** |
| S4 | Real SLR-PG → Leech SentencePiece alignment & lexical coverage | **Open** |
| S5 | Original frozen model syntax CE/F1/UAS/LAS vs controls | **Open** |
| S6 | Five-arm original full-backbone long grokking and RX580 C3 F3-B | **Open** |

Fail closed for incomplete/residual PG runs, revision/source SHA mismatch,
source token surface mismatch, overlapping/invalid spans, nonlocal heads,
missing POS/DEP labels, schema promotion flags, repeated source revisions,
cross-split hashes, and forbidden reserved/F5 manifests **before DB access**.
Stored parser observations remain distinct from reviewed propositions,
legal applicability, world truth and FLM/Monster representation claims.
