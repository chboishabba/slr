# Mabo 100-hop latent-world scale ledger — 2026-09-16

This is the focused scale successor to
`docs/roadmap/semantic_reader_runtime_20260916.md` on SLR #18.
The #18 runtime remains authoritative for the paid source/PNF/payment/Reader-ABI
path; this ledger tracks the deterministic 100-hop extension owned by SLR #19.

Updated: 2026-09-16 — the 100-hop latent-world substrate is paid. The active
product frontier has moved downstream to Dioxus, then wgpu.

## Production ownership

```text
DASHI / Agda       golden semantic/proof contract
SLR / Rust         semantic traversal policy, residuals, payment, recurrence, typed world state
PostgreSQL         active semantic persistence/query spine
Dioxus / Rust      downstream production reader/runtime shell
wgpu / WGSL        downstream visual exploration/rendering
SensibLaw Python   reference/regression surface
ITIR / Svelte      reference/regression prototype only
```

## PR lineage

```text
SLR #18  agent/semantic-reader-runtime-v1   canonical runtime + bounded world primitive
SLR #19  agent/mabo-100hop-world-v1         canonical deterministic 100-hop successor
```

#19 introduces no new persistence schema and creates no semantic authority.

## Current state

```text
P0-P4          PAID
P7_bounded     PAID LIVE on #18
P7_100hop      PAID on #19
P5 Dioxus      DOWNSTREAM ACTIVE CONSUMER
P6 wgpu        DOWNSTREAM AFTER P5
P8 publication LATER
```

Current dependency graph:

```text
Mabo seed proposition
-> existing PostgreSQL semantic relation surface
-> deterministic SLR traversal
-> typed bounded/100-hop world receipt
-> Dioxus consumption
-> optional wgpu exploration
```

SLR stops at the typed world receipt.

## #19 contract

The additive API is:

```text
LatentWorldBudget {
  max_hops,
  max_nodes,
  max_edges
}

load_latent_world_rows_with_budget(...)
  -> LatentWorldRows
```

The returned receipt retains:

```text
seed_ref
requested_max_hops
visited_refs
deepest_observed_hop
edges + per-edge provenance_refs
frontier_exhausted
frontier_refs
residual_refs
creates_semantic_authority = false
applicability_promoted = false
claim_truth_promoted = false
```

The original `load_latent_world_rows(...)` remains a compatibility wrapper over
the richer traversal.

## Traversal semantics

SLR owns deterministic traversal policy. PostgreSQL supplies persisted adjacent
semantic relations.

```text
reachable at hop n != true
reachable at hop n != controlling
reachable at hop n != applicable
100-hop request != complete closure
```

Cycle handling is state-deduplicating: each semantic ref enters the visited map
at most once. Edge order is deterministic from PostgreSQL ordering plus Rust
ordered collections.

Budgets fail closed:

```text
node budget -> world-residual:node-budget
edge budget -> world-residual:edge-budget
hop cap with live frontier -> world-residual:hop-limit
```

If no frontier remains, `frontier_exhausted=true` and no truncation residual is
retained.

Per-edge `provenance_refs` retain the persisted lineage coordinate that caused
the edge to exist. These are traversal-lineage coordinates only and never
promote legal authority.

## Validation status

The 100-hop world tranche is now treated as paid in the current SLR roadmap.
The canonical regression obligations remain:

```bash
cargo test -p sensiblaw-pg-source-store \
  --test latent_world_100hop \
  -- --ignored --nocapture

cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
```

Future changes to #19-owned traversal semantics must preserve those receipts and
the non-promotion invariants.

## Consumer-boundary freeze

The next SLR work is not another world-semantics tranche. #19 should now be
stabilised as the canonical latent-world producer for downstream consumers.

Freeze criteria:

```text
LatentWorldRows remains storage/consumer neutral
no Dioxus types enter sl-pg-source-store or sl-reader-model
no wgpu/rendering types enter SLR
creates_semantic_authority remains false
applicability_promoted remains false
claim_truth_promoted remains false
budget/truncation residuals remain explicit
per-edge provenance remains retained
```

A new SLR world tranche is justified only if Dioxus/wgpu exposes a concrete
missing typed coordinate that cannot be derived from the current receipt.

## Downstream handoff

Dioxus receives typed world/Reader state but does not recompute membership,
payment, applicability, or truth.

The intended next visual boundary is:

```text
LatentWorldRows / ReaderWorldProjection
-> WorldVisualIR / ProofConeVisualIR
-> wgpu
```

Rendered visibility, picking, layout, GPU reachability, and viewport state never
create semantic authority or evidence payment.

## Current implementation priority

```text
1. Freeze/regression-pin #19's public world receipt.
2. Follow the Dioxus consumer; do not add speculative SLR semantics.
3. Add a tiny SLR extension only if Dioxus names a missing typed coordinate.
4. Defer publication/federation-specific additions until P8 selects a concrete producer.
```

Explicitly out of scope for this ledger:

```text
Dioxus component/layout implementation
wgpu shader/layout/picking implementation
new world persistence schema
visual authority semantics
reader payment recomputation
```

SLR's 100-hop obligation is therefore complete enough to hand off:

```text
typed deterministic latent-world state
-> downstream consumer
```
