# Mabo 100-hop latent-world scale ledger — 2026-09-16

This is the focused scale successor to `docs/roadmap/semantic_reader_runtime_20260916.md` on SLR #18.
The #18 runtime remains authoritative for the paid source/PNF/payment/Reader-ABI path; this ledger tracks only the bounded-world -> deterministic 100-hop extension.

## Production ownership

```text
DASHI / Agda       golden semantic/proof contract
SLR / Rust         semantic traversal policy, residuals, payment, recurrence
PostgreSQL         active semantic persistence/query spine
Dioxus / Rust      downstream reader/runtime shell
wgpu / WGSL        downstream visual exploration/rendering
SensibLaw Python   reference/regression surface
ITIR / Svelte      reference/regression prototype only
```

## PR lineage

```text
SLR #18  agent/semantic-reader-runtime-v1   bounded latent-world primitive + live PG receipt
SLR #19  agent/mabo-100hop-world-v1        deterministic 100-hop scale successor
```

#19 is stacked directly on #18 and introduces no new persistence schema or semantic authority layer.

## Active frontier

```text
P0-P4          PAID
P7_bounded     PAID LIVE on #18
P7_100hop      ACTIVE FRONTIER on #19
P5 Dioxus      DOWNSTREAM after P7_100hop receipt
P6 wgpu        DOWNSTREAM after typed world/reader state exists
P8 publication LATER
```

The dependency graph is therefore:

```text
Mabo seed proposition
-> existing PostgreSQL semantic relation surface
-> deterministic SLR BFS
-> explicit 100-hop bounded world receipt
-> Dioxus consumption
-> optional wgpu exploration
```

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

The original `load_latent_world_rows(...)` remains as a compatibility wrapper over the richer traversal.

## Traversal semantics

SLR owns deterministic BFS policy. PostgreSQL only supplies persisted adjacent semantic relations.

```text
reachable at hop n != true
reachable at hop n != controlling
reachable at hop n != applicable
100-hop request != complete closure
```

Cycle handling is state-deduplicating: each semantic ref enters the visited map at most once. Edge order is deterministic from PostgreSQL ordering plus Rust ordered collections.

Budgets fail closed:

```text
node budget -> world-residual:node-budget
edge budget -> world-residual:edge-budget
hop cap with live frontier -> world-residual:hop-limit
```

If no frontier remains, `frontier_exhausted=true` and no truncation residual is retained.

Per-edge `provenance_refs` retain the persisted row/source coordinate that caused the edge to exist. These are traversal-lineage coordinates only and never promote legal authority.

## TDD / validation ledger

RED contract commit:

```text
a236833fcaad4a29e70347dd23722a413281987d
crates/sl-pg-source-store/tests/latent_world_100hop.rs
```

The contract requires a real Mabo run with:

```text
requestedMaxHops = 100
seedPreserved = true
deepestObservedHop > 0
visited <= node budget
edges <= edge budget
all traversed edges retain provenance
createsSemanticAuthority = false
applicabilityPromoted = false
claimTruthPromoted = false
frontier exhausted OR explicit world-residual retained
```

GREEN source candidate:

```text
00efb6108281dbd7616511c6a88692fb24b8ad44
```

GitHub Actions had not produced a run for the branch when this ledger was written, so no compile/live/workspace/clippy receipt is claimed yet.

Required focused live receipt:

```bash
set -a
source ../ITIR-suite/SensibLaw/.env
set +a

cargo test -p sensiblaw-pg-source-store \
  --test latent_world_100hop \
  -- --ignored --nocapture
```

Then regression verification:

```bash
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
```

Only after those receipts are observed may `P7_100hop` be marked paid.

## Downstream handoff

After P7_100hop is paid, Dioxus receives typed world/Reader state but does not recompute membership, payment, applicability, or truth.

wgpu later receives a projection such as:

```text
LatentWorldRows / ReaderWorldProjection
-> WorldVisualIR / ProofConeVisualIR
-> wgpu
```

Rendered visibility, picking, layout, or GPU reachability never creates semantic authority or evidence payment.
