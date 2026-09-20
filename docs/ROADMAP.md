# SensibLaw / SLR Production Roadmap

**Production implementation:** `chboishabba/slr`  
**Golden semantic/reference contracts:** `chboishabba/dashi_agda`

SLR is the production SensibLaw runtime. DASHI/Agda is the golden semantic and
architectural reference. The old Python SensibLaw code remains useful product
and historical evidence, but it is not the production runtime authority.

## Program invariant

```text
source/evidence
  -> revision-pinned acquisition receipt
  -> canonical source/span/observation state
  -> active consumer residual
  -> residual-driven producer plan
  -> bounded acquisition/execution
  -> candidate evidence
  -> explicit review/payment
  -> updated world state
  -> typed legal/normative projection
  -> source-backed reasoning/explanation
```

Transport, parsers, ontology providers, caches, graph adjacency, classifiers and
acquisition receipts never silently create claim truth, legal applicability or
publication authority.

## Roadmap hierarchy

```text
PROGRAM
  └─ Sprint
       └─ Milestone
            └─ implementation tranches / PRs / commits
```

Milestones are capability gates. A provider, enum, migration, fixture, telemetry
counter or one-file bridge is not a milestone by itself.

# Sprint 1 — Complete the recurrent acquisition machine

**Goal:** SensibLaw autonomously and safely executes bounded evidence-gathering
cycles and can stop/restart without changing production-relevant state.

```text
Residual
  -> ProducerPlan
  -> bounded physical acquisition
  -> candidate evidence
  -> explicit review/payment
  -> W(n+1)
  -> fresh diagnosis
  -> repeat
```

Milestones:

- **M1.1 Canonical physical acquisition plan**
  - P31/P279 specialised slice first;
  - route-aware Zelph/HF general snapshot second;
  - governed live fallback third;
  - semantic requests resolve to physical objects before scheduling;
  - no ordinary path means "load all adjacency".
- **M1.2 Bounded transport**
  - physical-object dedupe/coalescing before concurrency;
  - cache hit = zero network;
  - at most five distinct cold objects per batch;
  - six cold objects => 5 + 1;
  - truncation => abstain;
  - transport receipts expose logical requests, nodes, object refs, cache,
    coalescing, remote gets, bytes, live fallback and peak cold width.
- **M1.3 Generic producer execution**
  - one controller ABI executes at least classification, identity/source and
    authority/legal-source producer families;
  - route selection owns producer choice;
  - acquisition remains candidate-only; review owns semantic payment.
- **M1.4 Reviewed recurrent world expansion**
  - reviewed hop -> persist -> fresh re-diagnosis;
  - producer family may change;
  - new residuals may emerge;
  - rejected/blocked evidence remains recorded;
  - no fixed queue masquerades as adaptivity.
- **M1.5 Restart/replay equivalence**
  - durable hop indexes are consecutive;
  - each nonzero hop names the prior receipt;
  - `world_before(n+1) == world_after(n)`;
  - replay reconstructs the exact latest production head before continuation.

**Current state:** **certified closed** at `4bc8ce04eee4d79ccf15ef9131d80c8e19647bfb` with a green workspace test receipt, live PostgreSQL replay receipt, and matching Agda kernel receipt on the golden Sprint-1 head.

After Sprint 1, transport is infrastructure unless telemetry supplies a concrete
counterexample.

# Sprint 2 — Canonical evidence convergence — CLOSURE GATE

**Goal:** anything SensibLaw learns enters one evidence/provenance substrate.

Milestones:

- **M2.1 One manifestation envelope** for HF/Wikidata/Wikipedia/OALC/legal
  sources/PDFs/transcripts/user evidence.
- **M2.2 One source/span substrate**
  `EvidenceManifestation -> SourceRevision -> TextSpan -> Observation`.
- **M2.3 Shared reducer production ABI** feeding world, matter and law
  projections without internal shortcuts.
- **M2.4 Legal source providers become ordinary producers**
  (PG hit/no-network; miss/acquire/persist; second request reuses exact
  revision).
- **M2.5 Cross-family replay capstone** over classification evidence, an
  Australian authority revision and matter/narrative evidence through the same
  source/revision/span/observation/review/projection spine.

**Current cut:** M2.1–M2.4 are paid at the production capability level. Digital-ESD has completed its application workload and is now a regression/capstone corpus, not a core-development lane. The only remaining Sprint-2 closure question is M2.5: either locate the differently named mixed-family persisted replay owner or implement exactly that replay capstone.

Digital-ESD must not introduce a parallel evidence substrate. New PDF/document requirements belong in the generic document-ingestion lane and must be justified by a failing fixture.

Sprint 2 yields the **evidence machine**.

# Sprint 3 — Reviewed world -> legal issue graph

**Goal:** consume already-typed, provenance-bearing, potentially ambiguous world
material and perform the production SensibLaw legal projection over it. SLR does
not build a second global world model.

The formal/reference side already contains substantial paid machinery:
`WrongType`, `WrongElementRequirement`, source-conditioned atomic tests,
satisfied/unsatisfied/contested/unresolved element dispositions, proof-relevant
premises/exceptions/defeaters, burden/remedy structure, temporal/jurisdictional
scope, precedent applicability/distinguishing, reopening, and finite legal
search. Sprint 3 therefore focuses on production composition/parity rather than
re-inventing those semantics.

## M3.A — Canonical reviewed world -> WrongType issue state

Compose the existing evidence machine with legal element interpretation:

```text
ReviewedCanonicalEvidence
        ↓
reviewed fact/event state
        ↓
WrongType / WrongElementRequirement candidate projection
        ↓
element evidence disposition
        ↓
Satisfied | Unsatisfied | Contested | Unresolved
```

Requirements:
- asserted != established;
- contradicted != false;
- contested evidence may leave a legal element unresolved;
- empirical/statistical, health, financial, narrative, Wikidata and other
  coordinates are optional typed facets over source-addressed observations,
  not separate canonical evidence universes;
- a derived value remains linked to its exact source revision/span/structured
  coordinate and producer/review receipt.

## M3.B — Source-realised legal evaluator

Bring the existing Agda legal algebra into production SLR parity:

```text
reviewed exact legal source
        ↓
source-realised LegalRule
        ├ premises
        ├ exceptions
        ├ defeaters
        ├ burdens/conditions
        ├ jurisdiction
        ├ temporal validity
        └ authority role
        ↓
applicability -> violation -> liability -> remedy
```

A defence is not flattened into one universal boolean. Existing WrongType/rule
roles remain canonical: defence element, exception, defeater, independent rule,
burden shift, remedy limiter, or procedural/jurisdictional gate as appropriate.

## M3.C — Adaptive Australian legal capstone

Unify existing Mabo/Pabai/Cullen/GLJ machinery through one production runner:

```text
reviewed matter/world observations
+
reviewed as-at legal sources
        ↓
issue/element state
        ↓
proof/search over retained alternatives
        ↓
minimal cut / residual
        ↓
candidate information moves
        ↓
review/payment
        ↓
rerun
        ↓
persist / restart / replay
```

Use the existing case family for different shapes:
- **Mabo** — positive doctrinal route;
- **Pabai** — live defeater/counterfactual/reformulation residual;
- **Cullen + NSW CLA** — duty, breach, atomic statutory tests, negative
  coordinate, precedent use, source roles and reopening;
- **GLJ** — procedural/stay/limitation, source correction and
  majority/dissent/procedural posture.

Residual-driven research is a Sprint-3 reasoning capability. Sprint 4 only
renders its UX.

Permanent firewalls:

```text
observation            != occurrence
event existence        != wrong
harm                   != wrong
Wikidata class         != legal category
formal proof receipt   != external-world truth
source presence        != applicability
candidate element match != satisfied element
exception/defence live != all-elements-implies-liability
missing observation    != observed absence
cross-stream alignment != causation
```

Sprint 3 yields the **legal reasoning machine**.

# Sprint 4 — Product

**Goal:** expose the reasoning machine without introducing a second UI ontology.

Milestones:

- **M4.1 Matter workspace** — people/events/documents/claims/timeline.
- **M4.2 Issue workspace** — claim -> issues -> elements/status.
- **M4.3 Source-addressable semantic-node drill-down** — every displayed
  derived value supports source preview/open, dependency path, reverse/downstream
  use, revision lineage and residual inspection. Examples include `n = 500`,
  a bank-flow segment, HCA paragraph, Wikidata statement or sensor observation,
  all retaining one semantic identity across Explain/Why/Source/Context/Graph
  projections.
- **M4.4 As-at/change view** — legal position at date X and amendment deltas.
- **M4.5 Residual-driven research UX** — render the Sprint-3 planner's missing
  coordinate, candidate acquisition/check move and expected discrimination/
  residual reduction without moving the reasoning engine into the UI.
- **M4.6 Receipt/export surface** — sources, exact revisions, spans, reasoning
  edges, review state and hashes.
- **M4.7 Cross-jurisdiction projection** over one factual/world state and
  multiple normative systems.

Sprint 4 yields a usable SensibLaw product.

## Sprint discipline

1. Work by min-cut, not file count.
2. Parallelise independent owners once their ABI is fixed.
3. Do not reopen paid architecture without an executable counterexample.
4. Add providers only for live residual demand.
5. Stop transport optimisation once boundedness/replay/cost gates pass.
6. Every sprint ends in a persisted/replayable end-to-end campaign receipt.
7. Golden parity is continuous.
8. Compile-only or unit-only work is progress inside a milestone, never sprint
   completion.

## Current critical path

```text
verify/close M2.5 mixed-family replay
  -> M3.A reviewed world -> WrongType state
  -> M3.B production legal evaluator
  -> M3.C persisted adaptive Australian capstone
  -> product projections
```


## Sprint 2 current cut

M2.1–M2.4 are now treated as paid production capabilities:

- `sensiblaw-core::canonical_evidence::EvidenceManifestation` owns one
  revision/digest/receipt-pinned manifestation envelope;
- Wikidata, Wikipedia, OALC, cache-first legal sources and locally ingested
  official documents lower into the same canonical revision/span/observation
  substrate;
- M2.3 shared reviewed-evidence reduction is certified and preserves exact
  observation/revision/span/review/payment identity across world/matter/legal
  projections;
- M2.4 provider normalisation is exercised on the final SLR merge, including
  exact PG-hit/no-network and acquire->persist->reuse behaviour;
- Digital-ESD P0-A–P0-G is an application/regression workload. Its final
  full-text index receipt is **43,996 / 43,996 verified**, so the older
  8,724/8,758 intermediate counts are not roadmap state.

M2.5 remains the sole Sprint-2 closure gate until an existing differently named
owner is found or the explicit mixed-family persisted replay is implemented.
That capstone must traverse structured Wikidata evidence, legal authority/OALC
evidence and PDF/narrative/document evidence through one canonical
manifestation/revision/span/observation/review/reducer path, persist it, restart,
and replay exact identities/outcomes without semantic promotion.


## 2026-09-20 legal-runtime capstone tranche

The consolidated production implementation is now source-written in
`sensiblaw-legal-runtime` rather than split across new micro-crates.

It composes the existing canonical evidence/review substrate into:

```text
M2.5 mixed-family canonical evidence persist/replay
  -> M3.A reviewed world -> WrongType/element state
  -> M3.B source-realised rule evaluator
  -> M3.C one adaptive/persisted Australian campaign ABI
  -> M4.A projection-only matter/issue workspace
```

The runtime surface keeps four Australian calibration shapes behind one runner:
Mabo, Pabai, Cullen/NSW CLA and GLJ. These are calibration coordinates over
existing repository machinery; the new crate does not manufacture new external
legal propositions.

The M3.B evaluator retains premises, exceptions, defeaters, burdens,
jurisdiction, temporal scope, applicability, violation, liability and remedy as
distinct coordinates. A later live exception/defeater can reopen an earlier
result; all elements being satisfied does not erase negative legal structure.

The M4.A workspace is explicitly projection-only. Its nodes retain source
revision/span references, dependencies, downstream uses and residuals.

**Certification state:** source-written/static-audited in the connector session.
Do not mark M2.5/M3.A/M3.B/M3.C/M4.A runtime-certified until the exact branch
head passes:

```bash
cargo test -p sensiblaw-legal-runtime
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p sensiblaw-legal-runtime --example legal_runtime_capstone -- \
  artifacts/legal-runtime-capstone
```

The operator writes the M2.5 persisted replay artifact, one legal-campaign
ledger per calibration, and a consolidated capability report.
