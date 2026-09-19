# SensibLaw / SLR Sprint Board

This board is capability-sized. A sprint is not a file, adapter, semaphore or
provider patch; it closes only when the corresponding production capability is
demonstrated, persisted and replayable.

## Sprint 1 — Complete the recurrent acquisition machine — CERTIFIED CLOSED

**Deliverable**

```text
Residual
  -> ProducerPlan
  -> bounded physical acquisition
  -> candidate evidence
  -> explicit review
  -> payment
  -> W(n+1)
  -> re-diagnose
  -> repeat
```

restartably, without transport or acquisition evidence acquiring semantic
authority.

### M1.1 — Canonical physical acquisition plan

Semantic requests are resolved before scheduling:

```text
semantic request
  -> provider-specific resolution
  -> internal node / exact source coordinate
  -> physical acquisition objects
```

For Zelph/HF classification the preferred path is:

```text
P31/P279 specialised slice
  -> route-aware general snapshot
  -> governed revision-pinned live fallback
```

Production contract:
- logical QIDs are not the concurrency domain;
- physical objects are deduplicated before concurrency;
- repeated logical requests for one object are coalesced;
- no ordinary path is defined as "load all adjacency";
- acquisition plans and transport are inspectable/receipted.

Runtime owner:
`sl-world-expansion-runtime::sprint1_acquisition_machine` plus
`gwb_supervised_type_closure`.

### M1.2 — Bounded transport

The canonical transport ABI enforces:
- cache hit -> zero network;
- at most five distinct cold physical objects per transport batch;
- same physical object x N requests -> at most one acquisition;
- mixed snapshot + live -> not snapshot-simultaneous;
- incomplete/truncated object -> abstain;
- retrieval failure is not ontology negation.

The transport receipt exposes:
- logical request count;
- resolved-node count;
- planned object references;
- unique physical objects;
- cache hits/misses;
- coalesced gets;
- remote gets;
- bytes fetched;
- live fallbacks;
- peak cold-batch width;
- truncation/abstention state;
- deterministic receipt digest.

After this gate, transport is infrastructure unless telemetry supplies a
counterexample.

### M1.3 — Generic producer execution

One controller ABI executes materially different producer families. The Sprint 1
gate is demonstrated for at least:
- `ClassificationEvidence` -> P31/P279;
- `IdentitySource` -> Wikidata/persisted identity evidence;
- `AuthoritySource` -> governed legal authority/OALC producer.

The controller validates candidate-only/non-promoting evidence regardless of
family. The planner/review ontology remains owned by the existing
`sl-consumer-residual`, `sl-residual-planner`, `sl-route-selector`,
`sl-route-executor`, and `sl-reviewed-evidence-payment` crates.

### M1.4 — Reviewed recurrent world expansion

The production invariant is:

```text
W0
 -> diagnose R0
 -> select/execute producer
 -> candidate evidence
 -> explicit review/payment
 -> persist reviewed hop
 -> W1
 -> diagnose R1
 -> ...
```

Existing GWB adaptive runtime already provides the reviewed atomic-hop and
post-hop re-diagnosis machinery. Sprint 1 treats it as a production invariant:
- producer families may change between hops;
- new residuals may emerge;
- rejected/blocked evidence remains ledgered;
- acquisition alone never pays a semantic residual;
- no fixed queue may masquerade as adaptivity.

### M1.5 — Restart/replay equivalence

The existing PostgreSQL `context.gwb_adaptive_hop_receipt` chain is the durable
campaign receipt. Sprint 1 adds strict replay validation:
- hop indexes must be consecutive;
- each hop must name the previous receipt;
- `world_before(n+1) == world_after(n)`;
- every receipt remains candidate-only/non-promoting;
- replay reconstructs the exact latest world digest, receipt head, producer
  history and review outcomes.

`load_and_replay_campaign_head` performs this check directly against the
persisted PostgreSQL ledger.

### Sprint 1 exit gate

```text
M1.1 physical plan             source-written + tests
M1.2 bounded transport        source-written + tests
M1.3 generic producer exec    source-written + tests
M1.4 reviewed recurrence      existing generic runner + adaptive capstone test
M1.5 restart/replay           strict validator + capstone replay + durable PG audit
```

Certified at exact SLR head `4bc8ce04eee4d79ccf15ef9131d80c8e19647bfb`: workspace tests passed, the persisted PostgreSQL campaign replayed with an intact hop/world chain and non-promotion invariants, and the corresponding Agda golden modules typechecked green. The certified Sprint-1 branch is intentionally left unchanged after those receipts.

## Sprint 2 — Canonical evidence convergence — ACTIVE

**Single deliverable:** world, legal-source and narrative evidence converge on
one canonical evidence/provenance substrate.

Work package:
- **M2.1 one evidence manifestation envelope — source-written awaiting runtime certification**;
- one SourceRevision -> TextSpan -> Observation substrate;
- `shared_reducer` as the supported production ABI;
- legal providers as ordinary producer implementations;
- PG-hit/no-network, PG-miss/acquire/persist/reuse;
- cross-family exact replay.

## Sprint 3 — World-to-law weld

**Single deliverable:** one reviewed matter projects into a legal issue graph
without promoting event/harm/classification into legal conclusions.

## Sprint 4 — Legal reasoning + product surface

**Single deliverable:** complete legal-issue reasoning and user navigation over
matter -> issue -> rule -> evidence -> receipt, with why/what-missing/as-at
queries and no parallel UI ontology.

## Definition of sprint complete

A sprint closes only when:
- Rust implementation exists;
- relevant Agda golden contract is at parity;
- deterministic tests pass;
- persisted/replayable receipt exists;
- at least one end-to-end campaign demonstrates the capability;
- semantic/promotion firewalls remain fail-closed.

A compile-only or unit-only tranche is progress inside a sprint, not a sprint.


### Current M2.1 tranche

The first convergence owner is `sensiblaw-core::canonical_evidence::EvidenceManifestation`.

Existing Wikidata, Wikipedia and OALC producer artifacts now lower into that carrier through the current world-observation adapters. This is a convergence adapter over already-paid acquisition paths, not a new producer or scheduler.

M2.1 remains open for the remaining manifestation families and executable receipts; M2.2 remains the next structural min-cut.
