# SOURCE-MATERIALISATION-1 Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the provider-pinned on-demand materialisation boundary so evicted provider bytes are safe for legacy readers, provider identity/source revision binding is fail-closed under concurrency, eviction is atomic with ownership checks, and exact-slice failures are diagnostically precise.

**Architecture:** Keep the existing PR #57 provider materialisation contract and downstream review/legal gates. Harden the shared storage boundary rather than adding a new source stack: legacy cache lookup treats evicted bytes as a cache miss, provider lifecycle transitions serialize and revalidate immutable identity/digest ownership, and eviction/rehydration remain byte-cache operations only.

**Tech Stack:** Rust, PostgreSQL, existing `sensiblaw-pg-source-store` source/cache APIs, OALC/HF governed provider adapter.

**Spec:** Uploaded `SOURCE-MATERIALISATION-1` cut plus formal owners `SensibLawMaboDistributedLegalCorpusMaterialisationExact.agda` and `SensibLawOALCPostgresPersistenceExact.agda`.

## Global Constraints

- Provider revision must be immutable and pinned; never silently substitute `main`, `master`, `latest`, `HEAD`, or a branch ref.
- Durable state keeps provider/version identity, canonical digest/length, acquisition receipt, source revision, exact span/slice and derived refs; full source bytes are evictable cache state.
- Navigation may operate on the durable skeleton without resident full text; exact quotation/parser/review requires exact verified bytes.
- Materialisation, cache state, acquisition, and persistence create no semantic/legal authority, applicability, claim truth, evidence role, or normative-order inference.
- Existing local/non-provider canonical payloads must never be evicted by provider lifecycle operations.

## Review Focus

- Legacy cache-first lookup after provider-byte eviction returns a controlled miss, never a PostgreSQL decode failure.
- Concurrent first materialisations for the same provider/version cannot bind conflicting documents/digests.
- Split-specific materialisations cannot silently reuse a conflicting source revision whose legacy key omitted split.
- Persistence, eviction, and direct rehydration share one provider-revision session lock; eviction/rehydration additionally row-lock the canonical payload.
- Exact-slice ref mismatch reports a dedicated mismatch error rather than `MissingResolution`.
- Exact-slice payment reopens the resolution identified by the materialisation's acquisition receipt rather than whichever receipt is newest.

---

### Task 1: Make legacy cache lookup eviction-aware

**Files:**
- Modify: `crates/sl-pg-source-store/src/cache_first.rs`
- Test: `crates/sl-pg-source-store/tests/provider_materialization_contract.rs`

**Interfaces:**
- Consumes: nullable `corpus.canonical_content.payload` introduced by provider materialisation.
- Produces: `PostgresSourceStore::lookup_exact_source(...) -> Ok(None)` when the selected exact source has evicted bytes.

- [x] Add source-order regression coverage for nullable payload handling.
- [x] Change cache lookup to decode `Option<Vec<u8>>`; return `Ok(None)` for an evicted payload so provider-aware resolution can reacquire the pinned bytes.
- [x] Preserve existing invalid-UTF-8 and normal resident-hit behaviour in the source path.
- [ ] Run the focused provider materialisation contract test.

### Task 2: Serialize provider identity/source-revision lifecycle

**Files:**
- Modify: `crates/sl-pg-source-store/src/provider_materialization.rs`
- Test: `crates/sl-pg-source-store/tests/provider_materialization_contract.rs`

**Interfaces:**
- Consumes: immutable provider identity + acquired document + exact resolution receipt.
- Produces: persisted provider materialisation whose source revision/document/canonical digest are revalidated under one serialized provider-pin transition.

- [x] Add source-order tests for immutable OALC/HF pin syntax and legacy-key lock identity.
- [x] Acquire an advisory **session** lock derived from the legacy immutable provider/version identity before checking or creating the provider materialisation.
- [x] Revalidate any existing external source revision against provider/document/canonical digest coordinates and reject conflict before accepting the materialisation row.
- [x] Revalidate the inserted/reopened materialisation, exact source revision and exact resolution before releasing the session lock.
- [ ] Run focused tests.

### Task 3: Make eviction and rehydration transaction-safe

**Files:**
- Modify: `crates/sl-pg-source-store/src/provider_materialization.rs`
- Test: `crates/sl-pg-source-store/tests/provider_materialization_contract.rs`

**Interfaces:**
- Consumes: one persisted provider materialisation.
- Produces: atomic payload eviction/reinstall while durable identity/digest/derived coordinates remain unchanged.

- [x] Preserve shared-local-payload refusal and durable metadata after eviction in the source contract.
- [x] Make persistence, eviction and direct rehydration take the same provider-revision session lock.
- [x] Lock the canonical-content row and perform ownership/share checks plus payload NULL update in one eviction transaction.
- [x] Rehydrate under the same provider session lock plus canonical row lock after exact identity/digest verification.
- [ ] Run focused tests.

### Task 4: Correct exact-slice ownership/mismatch semantics

**Files:**
- Modify: `crates/sl-pg-source-store/src/provider_exact_slice.rs`
- Test: existing unit/integration coverage in `provider_materialization_contract.rs` or module tests.

**Interfaces:**
- Produces: dedicated `ProviderExactSliceError::RefMismatch` for persisted writer refs that disagree with the owning materialisation.
- Reopens only the exact source-resolution whose `resolution_evidence_ref` equals the materialisation's durable acquisition receipt.

- [x] Add the dedicated error variant and regression assertion.
- [x] Return it only for post-write ref disagreement; keep `MissingResolution` for actual missing resolution ownership.
- [x] Bind exact-slice payment to the materialisation acquisition receipt instead of latest-receipt ordering.
- [ ] Run focused tests.

### Task 5: Verify and reconcile PR contract

**Files:**
- Modify if needed: PR description/docs only.

**Interfaces:**
- Consumes all prior hardening.
- Produces a branch whose claimed verification state exactly matches observed commands/CI.

- [ ] Run `cargo test -p sensiblaw-pg-source-store --test provider_materialization_contract`.
- [ ] Run `cargo test -p sensiblaw-pg-source-store`.
- [ ] Run `cargo clippy -p sensiblaw-pg-source-store --all-targets -- -D warnings`.
- [x] Inspect PR review threads and address the source-level findings currently raised; do not equate review with compilation.
- [ ] Obtain exact-head executable verification evidence.
- [ ] Update PR description with the precise verified state and remaining empirical/provider-network boundary after those commands run.
