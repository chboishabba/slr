# Mabo P7d recurrent runtime delta — 2026-09-17

This delta is stacked on operator-certified SLR #24 at `efba015c78480c324c4f99d7ec8dab4a31020640`.
It records the source-written P7d.5f recurrence + consumer-diagnosis tranche on SLR #25.

## Paid before #25

```text
P7d.0 exact residual -> reviewed admission                    GREEN
P7d.1 producer adapters                                      GREEN
P7d.2 post-acquisition PNF/world re-entry                    GREEN
P7d.3 observed-delta frontier recurrence                     GREEN
P7d.4 atomic session + durable discovery lineage             GREEN + LIVE PG
P7d.5a identity-class novelty                                GREEN
P7d.5b normalized revision/digest WorldObservation           GREEN + LIVE P710
P7d.5c producer/observation provenance hardening             GREEN
P7d.5d getter parity -> ProofResidual                         GREEN
P7d.5e v2 identity-class lineage                             GREEN + LIVE PG
```

`GREEN` above refers to the operator-certified #24 receipt, not the new #25 code.

## Source-written on #25

### Recurrent runner

```text
WorldExpansionSession
-> prepare residual-bound reviewed cycle
-> execute on cloned session
-> persist cycle lineage
-> commit cloned session only after sink success
-> repeat
```

Terminal reasons remain typed (`TargetComplete`, `FrontierExhausted`, `CycleBudgetExhausted`, provider/identity/world-diagnosis/persistence blockers).

### Explicit reviewed evidence + durable restart state

A producer observation does not infer its own evidence role:

```text
WorldObservation
+ exact ConsumerRequirement
+ explicit reviewed EvidenceCoordinate
-> Review
-> Payment(gap/obligation)
-> review-aware residual recompilation
```

Restart safety remains:

```text
unique identity_class_ref cardinality
!= lineage row cardinality
!= representation string cardinality
```

with:

```text
remaining novelty = max(100 - durable unique identities, 0)
campaign total     = durable baseline + newly committed identities
```

and a fail-closed representation -> identity map.

A durably known identity may pay a new residual without advancing novelty or discovery lineage.

## NEW: current-world -> canonical consumer diagnosis

The earlier architectural wall was:

```text
current world
-> ??? canonical Mabo consumer requirements
-> ProofResidual frontier
```

The first concrete instance is now source-written.

`sl-world-expansion-runtime::diagnose_mabo_context_world_identity` consumes:

```text
LatentWorldRows
+ DiscoveryIdentityBaseline
```

and produces:

```text
ConsumerSpec
+ SameObject ConsumerRequirements
+ open Identity ProofResiduals
+ diagnosis rows
```

Those residuals project directly to the existing `ProofFrontier` ABI through `mabo_consumer_diagnosis_frontier`, so existing frontier Pareto/scheduler machinery remains authoritative.

### Consumer definition

```text
consumer:mabo-context-world-identity
```

Question:

```text
for each reviewed Mabo Wikidata context target,
do we already have a reviewed durable world identity assignment
for that representation?
```

Only a deliberately narrow first lane is admissible:

```text
relation_type starts context:wikidata:
+ reviewed context provenance receipt exists
```

Then:

```text
durable-known representation -> quotient before residual emission
unknown reviewed target        -> SameObject requirement + Identity residual
duplicate reviewed target      -> one requirement only
other latent-world edge        -> out-of-scope / WrongType for this consumer
```

This does NOT make relation review equal identity review.

### Discovery routes are proposal-only

The corresponding Agda owner on DASHI #991 reuses the canonical plural-lens discovery/admission surface:

```text
proof search
failed FactorsThrough
experiment design
observed residual
WrongType diagnosis
affected-subject / missing-carrier analysis
external-knowledge comparison (including Perplexity-like retrieval)
```

These may propose or rank a missing diagnostic axis. They do not self-certify evidence or review.

The intersectional finite witness is the same structural lesson as the acquisition Pareto:

```text
realised analytic carrier not sufficient to recover
who in the eligible population is missing
```

and recharting a lossy carrier cannot reconstruct erased information.

## Runnable 100-hop diagnosis boundary

New executable:

```sh
cargo run -p sensiblaw-world-expansion-runtime --example mabo_100hop_consumer_diagnosis
```

It uses:

```text
seed      Q1501525
max_hops  100
max_nodes 10,000
max_edges 50,000
```

It prints:

```text
requested_max_hops
deepest_observed_hop
visited_refs / edges / traversal residuals
durable baseline identity count
reviewed context edges considered
known identities quotiented
duplicate target edges
out-of-scope/WrongType edges
open SameObject requirements
canonical ProofFrontier reference/count
GAP/OBL/payment counts
first open residuals
```

The residual compiler is intentionally fed an empty payment stream in this executable, so unknown identity requirements remain honest open gaps/obligations. It never auto-reviews them.

`max_hops=100` is a budget, not a fabricated observed depth; `deepest_observed_hop` reports the actual persisted traversal depth.

## Current production frontier

The broad `WorldDiagnosisRequired` architecture gap is therefore paid for the first identity consumer.

The next boundary is now empirical/runtime:

```text
run mabo_100hop_consumer_diagnosis on live PG
-> observe actual SameObject residual frontier
-> existing Pareto/proof search selects producer work
-> acquire/review/pay identity coordinates
-> recurrently admit novel reviewed identities
-> recompute current world and diagnosis
-> repeat toward durable total >= 100
```

Additional consumer classes (authority/source text/applicability/counterfactual/etc.) should be introduced only in response to real frontier demands. Graph adjacency remains candidate/navigation data, not semantic work generation.

## Verification boundary

GitHub CI is not used for this project. The connected ChatGPT environment cannot resolve `github.com` from its execution container, so the new #25 tranche is **SOURCE-WRITTEN / EXECUTION-UNOBSERVED**.

Run locally:

```sh
git fetch origin
git checkout agent/mabo-p7d-recurrent-runner-v1
git reset --hard origin/agent/mabo-p7d-recurrent-runner-v1

cargo test -p sensiblaw-world-expansion-runtime --test mabo_consumer_diagnosis
cargo test -p sensiblaw-world-expansion-runtime
cargo clippy --workspace --all-targets -- -D warnings

cargo run -p sensiblaw-world-expansion-runtime --example mabo_100hop_consumer_diagnosis
```

The runtime output, not another architecture tranche, should determine the next producer/review implementation work.
