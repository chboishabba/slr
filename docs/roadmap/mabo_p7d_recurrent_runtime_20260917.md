# Mabo P7d recurrent runtime delta — 2026-09-17

This delta is stacked on SLR #24 at `efba015c78480c324c4f99d7ec8dab4a31020640`.
It records the source-written P7d.5f recurrence tranche on SLR #25.

## Paid before this tranche

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

`GREEN` above refers to the operator-certified #24 receipt, not #25.

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

Terminal reasons are typed:

```text
TargetComplete
FrontierExhausted
CycleBudgetExhausted
ProviderUnavailable
IdentityReviewRequired
WorldDiagnosisRequired
PersistenceBlocked
NoPreparedCycle
Other
```

The run receipt carries the complete novelty ledger:

```text
candidates_seen
candidates_rejected
duplicates_seen
identity_ambiguous
reviewed_objects
new_qids_admitted
new_articles_admitted
new_primary_legal_sources_admitted
new_other_world_objects_admitted
final_novel_identity_classes
```

### Explicit reviewed-evidence payment

A producer observation does not infer its own semantic evidence role.

```text
WorldObservation
+ exact ConsumerRequirement
+ explicit reviewed EvidenceCoordinate
-> Review record
-> Payment(gap)
-> Payment(obligation)
-> review-aware residual recompilation
```

For the concrete Mabo fixture:

```text
Q1501525 P710 Q975866
+ reviewed SameObject
-> participant-identity requirement paid candidate-only
```

This does not promote authority, applicability or claim truth.

### Durable identity baseline

Restart safety is based on `context.discovery_lineage_receipt`, not process-local counters.

```text
unique identity_class_ref cardinality
!= lineage row cardinality
!= representation string cardinality
```

The baseline retains a representation -> identity-class map and fails closed if one durable representation maps to two identity classes.

The remaining novelty target is:

```text
max(100 - durable_unique_identity_classes, 0)
```

and the campaign total is:

```text
durable_baseline + newly_committed_identity_classes
```

### Identity-coherence guard

Within a run and across a loaded durable baseline:

```text
same representation + different class
-> IdentityReviewRequired
```

A durably known identity is not passed through novel admission. If it is relevant to a newly open residual it requires the non-novel payment lane.

### Non-novel known-identity payment

A known identity can pay a new residual without becoming a new object:

```text
known reviewed identity
+ reviewed evidence payment
+ observed residual contraction
-> persist Review/Payment wire records
-> frontier transition
```

but:

```text
novel identity count unchanged
discovery lineage unchanged
semantic authority unchanged
claim truth unchanged
```

The runtime stages the frontier transition and commits it only after `sl-world-store` accepts the reviewed payment stream.

### Resume-aware first Mabo cycle

`mabo_first_recurrent_cycle` now loads the durable identity baseline before acting.

On the current TrueNAS lineage, `world-object:eddie-mabo` is already durable, so the intended branch is:

```text
live pinned P710 observation
-> explicit SameObject review
-> Review/Payment stream
-> known-identity residual payment
-> no novel admission
```

On a clean database the same executable takes the novel identity-admission lane.

## Current substantive frontier

The recurrence machinery is no longer the missing semantic controller.

The remaining source-level blocker for an autonomous multi-cycle Mabo campaign is a canonical checked-in Mabo PNF/consumer requirement surface that can diagnose the next open residuals after each world update.

The repository currently contains:

```text
generic ConsumerSpec / residual compiler
reviewed Mabo Wikidata context relations
reviewed PNF persistence machinery
producer routing / acquisition machinery
```

but not a broad canonical Mabo `ConsumerSpec` (or equivalent reviewed PNF-derived requirement set) that determines the next residual frontier.

Therefore the campaign must NOT generate the next residual merely from:

```text
Wikidata adjacency
reviewed context neighbours
Wikipedia links
legal citation links
```

Those remain candidate producers/navigation surfaces only.

The correct next semantic payment is:

```text
reviewed Mabo PNF / consumer obligations
-> current world
-> canonical residual diagnosis
-> Ibrahim/Pareto producer selection
-> acquisition/review/payment
-> recompute
```

Until that diagnosis input exists, a recurrent campaign should terminate with `WorldDiagnosisRequired` rather than manufacture residuals.

## #25 execution status

No Cargo, Clippy, live provider or live DB GREEN is claimed for #25 in this document.
The exact head must be executed locally.

Recommended tranche:

```sh
git fetch origin
git checkout agent/mabo-p7d-recurrent-runner-v1
git reset --hard origin/agent/mabo-p7d-recurrent-runner-v1

cargo test -p sensiblaw-pg-source-store --test discovery_identity_baseline
cargo test -p sensiblaw-proof-search-loop --test world_expansion_runner
cargo test -p sensiblaw-proof-search-loop --test world_identity_guard
cargo test -p sensiblaw-proof-search-loop --test world_identity_guard_resume
cargo test -p sensiblaw-proof-search-loop --test known_identity_payment
cargo test -p sensiblaw-reviewed-evidence-payment
cargo test -p sensiblaw-world-expansion-runtime
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cargo run -p sensiblaw-world-expansion-runtime --example mabo_p710_identity_residual
cargo run -p sensiblaw-world-expansion-runtime --example mabo_first_recurrent_cycle
```

The second example requires the configured PostgreSQL/world-store environment and is expected to exercise the known-identity branch on the existing TrueNAS lineage.
