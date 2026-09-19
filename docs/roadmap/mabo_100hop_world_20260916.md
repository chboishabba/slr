# Mabo 100-hop latent-world scale ledger — 2026-09-16

This is the focused scale successor to
`docs/roadmap/semantic_reader_runtime_20260916.md` on SLR #18.
The #18 runtime remains authoritative for the paid source/PNF/payment/Reader-ABI
path; this ledger tracks the deterministic 100-hop extension owned by SLR #19
and the now-separate context/source breadth obligation.

Updated: 2026-09-16 — the 100-hop traversal substrate is paid, but live corpus
breadth is not. The observed TrueNAS receipt reached only 11 nodes / 11 edges,
with deepest observed hop 3 and an exhausted frontier. This proves deterministic
bounded traversal over the currently persisted semantic relation surface; it does
not prove that Wikipedia, Wikidata, OALC, or other available context/source
producers have been explored or materialised into that surface.

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
SLR #19  agent/mabo-100hop-world-v1         canonical deterministic 100-hop traversal
successor agent/mabo-context-federation-v1  context/source breadth correction
```

#19 introduces no new persistence schema and creates no semantic authority.
The successor must likewise reuse existing acquisition/review/persistence surfaces
rather than teaching the world walker to perform network I/O.

## Current state

```text
P0-P4          PAID
P7_bounded     PAID LIVE on #18
P7a_100hop     PAID LIVE on #19 (traversal substrate only)
P7b_federation UNPAID: Wikimedia + OALC/context producers are not yet materialised into the live world
P5 Dioxus      ACTIVE / typed consumer exists downstream
P6 wgpu        VisualIR boundary staged downstream
P8 publication LATER
```

Current dependency graph:

```text
Mabo seed proposition
-> reviewed/existing PostgreSQL semantic relation surface
-> deterministic SLR traversal
-> typed bounded/100-hop world receipt
-> Dioxus consumption
-> optional wgpu exploration
```

Required breadth extension:

```text
Wikidata revision/bundle review ----\
Wikipedia revision/context links -----+-> reviewed/candidate-only context relations -> PostgreSQL
Open Australian Legal Corpus --------/                                     |
                                                                            v
                                                              existing 100-hop traversal
```

SLR traversal stops at the typed world receipt. Acquisition/review producers
must populate the persistence surface first.

## Breadth audit: what #19 currently does and does not explore

The live `latent_world` neighbour query currently unions persisted edges from:

```text
algebra.relation
execution.dependency
legal_ir.graph_revision
legal_ir.semantic_build
legal_ir.projection
legal_ir.observation
```

It does not perform network/provider operations and does not itself read
Wikipedia, Wikidata, or OALC/Hugging Face.

The current Mabo context bundle names:

```text
Wikidata Q1501525
Wikipedia wiki:en:Mabo_v_Queensland_(No_2)
source:mabo:1992:hca:23
context:mabo:history
```

but those identifiers are navigation coordinates, not proof that their source
neighbourhoods were acquired, reviewed, or materialised into PostgreSQL.

Existing reusable producer machinery should be preferred:

```text
Wikidata/source-unit review:
  historical SLR source-handoff ABI (revision-locked SourceUnit,
  statement-bundle review, reference/provenance separation)

Australian legal corpus:
  sensiblaw-governed-legal-provider OALC snapshot + exact-MNC lookup
  persisted/local -> installed OALC -> official court -> other governed fallback

Offline/legal neighbourhood expansion:
  existing proof-search/acquisition/query machinery
```

These producers remain candidate/review surfaces and create no world truth,
applicability, or claim truth merely by acquisition or graph reachability.

## P7b concrete flagship producer set

### Wikidata — Q1501525

Acquire/review a revision-locked statement bundle and materialise candidate-only
context relations for at least the useful Mabo neighbourhood already represented
by Wikidata, including where present in the acquired revision:

```text
instance of legal case
country / applies-to jurisdiction Australia
participant Eddie Mabo
participant Queensland
court High Court of Australia
legal citation [1992] HCA 23
work-available-at URL / AustLII manifestation
judges
start/end dates
overrules Milirrpum v Nabalco Pty Ltd
```

Statement/reference/rank/provenance coordinates remain separate. A Wikidata
statement or reference is not primary legal authority.

### Wikipedia — Mabo v Queensland (No 2)

Acquire a revision-locked page/source unit and materialise background/navigation
candidates such as:

```text
Mabo No 1
Eddie Mabo
native title / historical context
High Court transcripts / referenced manifestations
related legislation/history links when actually present in the acquired revision
```

Wikipedia remains context, not evidence payment.

### Open Australian Legal Corpus (OALC / Hugging Face)

Use the existing installed/local OALC lane to resolve `[1992] HCA 23` against a
revision-pinned corpus snapshot, then expose the resulting local source
manifestation and bounded candidate legal neighbourhood to the existing review /
proof-search path.

```text
OALC lookup/acquisition receipt != reviewed proposition support
OALC corpus membership != controlling authority
OALC source text != claim truth
```

Prefer exact-MNC/local lookup and offline/corpus search before live-provider
fallback. Do not route Hugging Face or dataset serialization directly into the
Reader ABI.

## P7b acceptance receipt

P7b is paid only after an observed live PG/world run demonstrates that at least
one reviewed/candidate-only producer from each selected lane has crossed into the
persisted relation surface with retained provenance:

```text
Wikidata producer receipt present
Wikipedia producer receipt present
OALC revision/record receipt present
world edge provenance identifies the producer/source revision
context edges remain distinguishable from legal_ir proof/payment edges
creates_semantic_authority = false
applicability_promoted = false
claim_truth_promoted = false
```

The scale receipt should report source-family coverage as well as graph size; a
100-hop request over eleven pre-existing nodes is not a corpus-breadth receipt.

## #19 traversal contract

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
100-hop request != source breadth
frontier exhausted != all available external/context sources explored
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

If no persisted frontier remains, `frontier_exhausted=true`; this says nothing
about unacquired external producers.

Per-edge `provenance_refs` retain the persisted lineage coordinate that caused
the edge to exist. These are traversal-lineage coordinates only and never
promote legal authority.

## Validation status

P7a traversal is paid. Canonical regression obligations remain:

```bash
cargo test -p sensiblaw-pg-source-store \
  --test latent_world_100hop \
  -- --ignored --nocapture

cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
```

Future changes to #19-owned traversal semantics must preserve those receipts and
the non-promotion invariants. P7b should normally add producers/materialisation,
not change BFS semantics.

## Consumer-boundary freeze

#19 remains the canonical latent-world traversal producer. P7b is a breadth
producer tranche, not a reason to add Dioxus/wgpu semantics to SLR.

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

## Downstream handoff

Dioxus receives typed world/Reader state but does not recompute membership,
payment, applicability, or truth.

The intended visual boundary remains:

```text
LatentWorldRows / ReaderWorldProjection
-> WorldVisualIR / ProofConeVisualIR
-> wgpu
```

Rendered visibility, picking, layout, GPU reachability, and viewport state never
create semantic authority or evidence payment.

## Current implementation priority

```text
1. Keep #19 traversal frozen/regression-pinned.
2. P7b: materialise revision-pinned Wikidata Q1501525 context candidates.
3. P7b: materialise revision-pinned Wikipedia Mabo context candidates.
4. P7b: resolve [1992] HCA 23 through installed OALC and expose a bounded local legal neighbourhood.
5. Re-run the live 100-hop receipt and report source-family coverage, not only hop/node counts.
6. Feed the enriched typed world to Dioxus; keep wgpu downstream of typed state.
```

Explicitly out of scope:

```text
network I/O inside latent_world.rs
Wikipedia/Wikidata/OALC claims becoming legal authority automatically
Dioxus component/layout recomputing source review/payment
wgpu shader/layout/picking creating semantic authority
new persistence schema unless existing relation/provenance surfaces prove insufficient
```

The corrected milestone is therefore:

```text
P7a deterministic traversal substrate  PAID
+
P7b governed source/context federation UNPAID
-> enriched typed latent-world state
-> Dioxus
-> optional wgpu exploration
```
