# SensibLaw / SLR Sprint Board

This board is intentionally capability-sized. It exists to prevent the production roadmap degrading into one-file or one-adapter micro-sprints.

## A — Bounded ontology transport closure — ACTIVE

**Single deliverable:** GWB P31/P279 acquisition is physically bounded, deduplicated, replayable and measurable.

Work package:
- predicate-slice fast path;
- route-aware general HF fallback;
- physical-object plan + dedupe/coalescing;
- <=5 distinct remote object fills;
- cache-hit zero-network invariant;
- transport receipt;
- replay/campaign proof that full adjacency is not the normal path.

Do not split these into separate roadmap milestones.

## B — Generic epistemic scheduler — NEXT

**Single deliverable:** one scheduler executes multiple residual/producer families and persists/replays the resulting world-expansion campaign.

Reuse:
- `sl-consumer-residual`;
- `sl-residual-planner`;
- `sl-route-selector`;
- `sl-route-executor`;
- `sl-reviewed-evidence-payment`;
- `sl-world-expansion-runtime`.

The work is ABI/convergence and campaign execution, not a new planner ontology.

## C — Canonical reducer + legal source convergence

**Single deliverable:** world, legal-source and narrative evidence all enter one canonical evidence substrate and replay by exact provenance.

Reuse the existing PG source/world stores and governed legal providers.

## D — World-to-law weld

**Single deliverable:** one reviewed matter can project into a legal issue graph without promoting event/harm/classification into legal conclusions.

## E — Legal reasoning kernel

**Single deliverable:** one complete legal issue is reasoned as support/contradiction/unknown across elements, conditions/exceptions/defences, time, jurisdiction and authority.

## F — Productisation

**Single deliverable:** a user can navigate matter -> issue -> rule -> evidence -> receipt and ask why/what-missing/as-at without a parallel UI ontology.

## Definition of sprint complete

A sprint closes only when:
- Rust implementation exists;
- relevant Agda golden contract is at parity;
- deterministic tests pass;
- persisted/replayable receipt exists;
- at least one end-to-end campaign demonstrates the capability;
- semantic/promotion firewalls remain fail-closed.

A compile-only or unit-only tranche is progress inside a sprint, not a new sprint.
