# Mabo adaptive campaign scope audit — 2026-09-17

## Corrected production invariants

The adaptive campaign has two distinct durable identity coordinates:

```text
global durable identity baseline
    = SameObject quotient / alias reuse / identity coherence universe

Mabo campaign identity classes
    = discovery-lineage identity classes transitively rooted at Q1501525
```

These are intentionally not interchangeable.

The Mabo >=100 target, `TargetComplete`, and remaining-novelty policy must use only the seed-rooted campaign cardinality. Unrelated identities persisted by other campaigns may still prevent duplicate novelty through the global quotient, but cannot pay the Mabo target.

Likewise, context-expansion residuals must satisfy:

```text
durable representation
AND valid QID
AND member of current Mabo world.visited_refs
AND not already durably expanded
```

The global durable identity table is not itself the Mabo frontier.

## Adaptive cycle semantics

`100` remains the maximum number of completed adaptive epistemic transitions, not BFS depth.

A completed cycle may be either:

1. a reviewed identity transition, using the identity-coherent one-cycle recurrent runner for novel identity admission; or
2. a reviewed source/context-expansion transition, using its exact revision + candidate-set-digest review and durable source-expansion receipt.

The second must not be forced through the novel-identity runner: source expansion may add zero identities and remains distinct from discovery lineage.

Both transition kinds must return to the same outer loop before another move is selected:

```text
reload durable state
-> diagnose current world
-> rebuild ProofFrontier
-> Pareto-select exactly one move
-> execute/review/persist exactly that transition
-> reload and re-diagnose
```

## Verification boundary

This file records source semantics only. No new Cargo/Clippy/live-PG execution receipt is claimed for the adaptive scope fixes until the exact branch head is run locally.
