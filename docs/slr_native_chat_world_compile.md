# SLR-native archived-chat world compile (candidate-only v1)

## Decision / production ownership
SLR Rust is the production compiler. Python SensibLaw and re-gpt `PnfFleetCompiler` spaCy tokenization are legacy/reference. Golden Agda contracts are unchanged.

Existing `sensiblaw-world-compiler compile` accepts only binary SLRO (stdin or `--input`); it has no SQLite/JSONL producer or PostgreSQL target. Existing `sensiblaw-world-store ingest-wire` separately ingests SLRW to PostgreSQL via `DATABASE_URL` or `--env-file`. No database credentials are embedded.

This tranche adds `sensiblaw-world-compiler compile-chat` accepting strict `sensiblaw.chat-archive-message.v0_1` JSONL already defined by **in-flight re-gpt PR #34** and used by **in-flight slr PR #46**. No changes to those branches or Agda. Source identity must be verified; no fallback to canonical hashes.

## Invocation / bounded five-thread example
The exporter in re-gpt PR #34 supports `--conversation-id` but **not a global --limit**. Never run its unfiltered export against 250k rows. Select five source-thread IDs from a bounded metadata-only query, then export one thread at a time. For example, from the re-gpt checkout containing PR #34:

```bash
mkdir -p /tmp/slr-chat5
for cid in "$CID1" "$CID2" "$CID3" "$CID4" "$CID5"; do
  python scripts/export_archive_messages_for_slr.py \
    --db ~/chat_archive.sqlite --conversation-id "$cid" \
    --output "/tmp/slr-chat5/$cid.jsonl"
done
cat /tmp/slr-chat5/*.jsonl > /tmp/slr-chat5/tranche.jsonl
cd /home/c/Documents/code/slr
cargo build --release -p sensiblaw-world-compiler
./target/release/sensiblaw-world-compiler compile-chat \
  --iteration 1 --max-messages 10000 \
  --input /tmp/slr-chat5/tranche.jsonl \
  --output /tmp/slr-chat5/world.slrw
```

`CID1` through `CID5` must be actual exact source conversation IDs, not titles or canonical-hashed IDs. `--max-messages` is a hard cap on total messages in the selected tranche, **not** a thread selector. The exporter currently materializes one selected thread in Python; a fully Rust/SQLite source adapter remains follow-on if eliminating all Python from the acquisition boundary is required. This path eliminates Python/spaCy from the **SLRO/world compilation stage**, not from source export.

The Rust command validates source coordinates using `sl_core::chat_source::ArchivedChatMessage`, rejects unverified identity, duplicate exact message IDs, invalid schema, unbounded message counts and individual records above 4 MiB. It emits SLRO into an anonymous temporary file, then invokes the existing Rust `compile_observation_stream` to write SLRW. The anonymous intermediate is deleted on close and not a persistent duplicate corpus.

**Critical semantic boundary:** The native lexer emits only `UnresolvedDependency` token observations. It does not invent dependency heads, POS, lemmas or PNF claims. Consequently the resulting SLRW currently contains source manifestations and iteration receipt, with **zero PNF candidates and zero world atoms**. This is a safe source/world-store ingestion path, **not** equivalent semantic PNF compilation. A certified SLR-native parser-to-PNF producer remains the next gate. Branch/node/parent/time are hashed into revision lineage and message coordinates into the manifestation reference; do not mistake this for full graph persistence.

## PostgreSQL (optional, separately governed)
```bash
# Requires DATABASE_URL in environment or --env-file PATH.
./target/release/sensiblaw-world-store ingest-wire \
  --input /tmp/slr-chat5/world.slrw
```
World compilation itself only emits `.slrw`; it does not write PG. `sl-world-store` owns PG persistence and uses `DATABASE_URL`, with `--env-file` as explicit alternative. If PG is unavailable, omit ingestion; no PG validation is claimed.

## Verification commands (calling lane)
```bash
cd /home/c/Documents/code/slr
git status --short
git log -3 --oneline
gh pr list --state open
cargo build --release -p sensiblaw-world-compiler --locked
cargo test -p sensiblaw-world-compiler --locked
cargo test --workspace --locked
# Then the five-thread command above, with actual IDs and exporter PR #34 checkout.
```

No checkout was mounted in the GitHub-connector implementation environment. **No build, tests, live five-thread execution, PG smoke or Agda kernel run was performed.** There is no live compile receipt; only source-written commands and synthetic tests.

## Golden reference / formal gap
SLR `docs/ROADMAP.md` Sprint 2 requires one manifestation→revision→span→observation substrate; `docs/PROOF_OBLIGATIONS.md` forbids parser publication effects. `sl-core::chat_source` fixtures enforce message identity ≠ statement identity, and candidate-only source state. Tests reuse that source-domain example; no Agda source is modified and no Agda-derived kernel receipt is claimed. Full source/lineage graph preservation and certified semantic PNF remain open.

## Receipt fields
The CLI emits `SLR_CHAT_NATIVE_RECEIPT` to stderr with message count, unresolved token count, manifestation count, zero PNF/atoms, `candidate_only=true` and `semantic_promotion=false`. Capture stderr in the calling lane after local verification; never fabricate receipt values.
