# Mabo Adaptive Hop/Re-entry Design

## Goal

Replace the batch first-layer campaign shape with a genuinely adaptive world-expansion loop:

```text
current persisted world
-> diagnose current residual frontier
-> Pareto-select exactly one reviewed residual/move
-> acquire exactly one governed source manifestation
-> parse exactly that source
-> assess contraction + newly exposed world context
-> persist payment/lineage/context
-> re-diagnose from the updated durable world
-> repeat
```

`100` means a maximum of 100 committed adaptive acquisition/re-entry cycles. It does not mean pre-walking graph adjacency to depth 100.

## Counter separation

The implementation keeps three independent coordinates:

1. `traversal_depth`: descriptive graph-distance metadata only.
2. `campaign_cycles`: committed acquire/parse/assess/re-entry iterations, bounded by 100.
3. `durable_novel_identity_classes`: reviewed durable world-object cardinality, target 100.

None may be used as an alias for another.

## Existing owners retained

No new planner ontology is introduced.

- `ProofFrontier` remains the canonical current gap surface.
- `frontier_pareto` / `select_frontier_move` remain the canonical Pareto selector.
- `run_recurrent_world_expansion` remains the canonical single-cycle staged execution/persistence owner.
- `MaboConsumerDiagnosis` remains the first identity consumer.
- reviewed SameObject payment, identity coherence, known-identity non-novel payment and PG lineage remain unchanged.

The adaptive campaign is an outer recurrence that rebuilds the frontier before every one-cycle invocation of the existing runner.

## One-cycle semantics

For cycle `i`:

1. Reload the durable identity baseline.
2. Read the currently persisted reviewed world reachable from the Mabo seed. The reader hop budget is inspection metadata, not the campaign budget.
3. Re-run `diagnose_mabo_context_world_identity` and project it to a fresh `ProofFrontier`.
4. Match only explicit operator identity reviews currently applicable to that diagnosis.
5. Compile matched reviews into `FrontierCandidateMove`s and use `select_frontier_move` to select exactly one move.
6. Reacquire the exact pinned parent manifestation that proves the selected relation and prepare the reviewed SameObject cycle.
7. If the reviewed identity class is already durable, pay it through the existing non-novel lane and restart diagnosis without incrementing novelty.
8. Otherwise execute exactly one novel cycle through `run_recurrent_world_expansion(... max_cycles: 1)`.
9. Only after durable admission may the target representation become a candidate source for the next hop.
10. Acquire the target QID's own current revision as a pinned receipt, parse its outgoing bounded Wikidata properties, and expose them as candidate context for the next diagnosis. Persisting those outgoing context edges requires a context-review authority distinct from the SameObject identity review.

## Critical review boundary

Identity review and context expansion review are different authorities:

```text
SameObject identity review
!=
outgoing relation/context review
```

The campaign must never silently treat an identity review as approval of all outgoing relations of that object.

Therefore the first implementation may terminate with a typed `ContextReviewRequired` blocker after successfully admitting and parsing a target whose outgoing bounded context has not yet been explicitly reviewed. This is preferable to manufacturing context authority.

## Bounded Wikidata generalisation

The current context producer hard-codes `Q1501525` and one Mabo revision. Adaptive hopping requires the same exact finite property-to-role mapping for arbitrary reviewed QID manifestations:

```text
P1001 -> context:wikidata:jurisdiction
P710  -> context:wikidata:participant
P4884 -> context:wikidata:court
P1594 -> context:wikidata:judge
P4006 -> context:wikidata:overrules
```

The generalised function must:

- accept only exact `wikidata:<QID>:oldid:<positive revision>` source refs;
- require that the ref's QID equals the route source QID;
- accept only the finite property mapping above;
- require exact candidate-id/source/property/target coherence;
- remain candidate-only;
- create no legal authority, applicability, proposition payment or claim truth.

The existing Mabo-specific function remains as a compatibility wrapper with its stronger fixed-QID/fixed-revision check.

## Target-source acquisition

The provider must be able to obtain the current target revision ID first, then fetch that exact RDF manifestation. A latest-revision lookup is only a coordinate-discovery step; the semantic input is the subsequent exact revision receipt.

```text
latest revision lookup
-> revision id
-> exact revision RDF fetch
-> AcquiredEntityRdf(source_revision_ref = wikidata:Q...:oldid:N)
```

No unpinned `latest` RDF bytes may be admitted as the durable source manifestation.

## Stop conditions

The adaptive operator terminates on the first of:

- durable identity target reached;
- 100 committed adaptive cycles reached;
- current frontier genuinely empty after re-diagnosis;
- identity review required;
- context review required;
- provider unavailable;
- persistence blocked;
- identity coherence conflict;
- no admissible Pareto move.

## Restart semantics

Every cycle re-reads durable state. A crash after lineage persistence but before in-memory session commit is recoverable because the next process reconstructs the durable baseline and re-diagnoses. Newly reviewed context, when persisted, is likewise idempotent.

## Non-goals

- No automatic identity inference.
- No automatic context review.
- No adjacency -> semantic requirement shortcut.
- No review/payment -> legal authority shortcut.
- No requirement that breadth-first siblings be resolved before a newly exposed higher-value branch.
- No claim that 100 graph edges or depth 100 equals 100 durable identities.
