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

# Sprint 2 — Canonical evidence convergence — ACTIVE

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

Sprint 2 yields the **evidence machine**.

# Sprint 3 — Full legal issue reasoning

**Goal:** given reviewed matter evidence and law valid at the relevant time,
produce a reviewable legal issue graph.

Milestones:

- **M3.1 Reviewed factual state** preserving asserted/supported/contradicted/
  unknown and contestation.
- **M3.2 Authoritative rule state** from pinned source -> provision -> duty/rule
  -> WrongElementRequirement with jurisdiction and temporal validity.
- **M3.3 Fact -> element candidate projection**, never fact -> conclusion.
- **M3.4 Ternary issue state**:
  `supported | contradicted | unknown`, with provenance.
- **M3.5 Conditions, exceptions and defences** as typed structural roles.
- **M3.6 Temporal + jurisdictional applicability** using as-at source state.
- **M3.7 Authority topology** (cites/applies/distinguishes/overrules/amends)
  without citation-graph rank becoming legal priority.
- **M3.8 Full Australian issue capstone**:
  matter evidence + as-at law -> issue/elements -> support/contradiction/unknown
  -> conditions/defences -> provisional analysis -> remaining acquisition
  residuals.

Permanent firewalls:

```text
event existence       != wrong
harm                  != wrong
Wikidata class         != legal category
source presence        != applicability
candidate element match != satisfied element
unknown                != false
```

Sprint 3 yields the **legal reasoning machine**.

# Sprint 4 — Product

**Goal:** expose the reasoning machine without introducing a second UI ontology.

Milestones:

- **M4.1 Matter workspace** — people/events/documents/claims/timeline.
- **M4.2 Issue workspace** — claim -> issues -> elements/status.
- **M4.3 Evidence drill-down** — conclusion -> reasoning edge -> observation ->
  span -> exact source revision.
- **M4.4 As-at/change view** — legal position at date X and amendment deltas.
- **M4.5 Residual-driven research UX** — what is missing, possible acquisition,
  expected residual reduction, without promising proof.
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
unify evidence machine
  -> build legal reasoning machine
  -> ship product
```


## Sprint 2 current cut

M2.1 and M2.2 are paid on the stacked Sprint-2 branch:

- `sensiblaw-core::canonical_evidence::EvidenceManifestation` owns one revision/digest/receipt-pinned manifestation envelope;
- the existing Wikidata, Wikipedia and OALC observation adapters lower into that same envelope;
- the envelope is candidate-only and cannot carry semantic authority, applicability promotion or claim-truth promotion;
- M2.2 source/revision/anchor/observation convergence is now source-written: text uses exact character ranges, structured graph evidence uses exact structured coordinates, compiler tokens require a preceding exact revision, and persisted PG legal slices project into the same carrier.
- M2.3 shared reducer ABI is now source-written: one reviewed canonical evidence object fans out to optional world/matter/legal projection slots, with exact observation/revision/span/review/payment identity repeated in each projection receipt; abstention is explicit and no projection or reducer creates semantic authority, applicability, or claim truth.

M2.1 and M2.2 have exact Rust and Agda receipts. M2.3 is source-written only on the new head and remains `implementedAwaitingRuntime` until fresh Rust/Agda execution receipts are observed. Sprint 2 remains open pending M2.4, M2.5 and the persisted cross-family replay capstone.


## Digital-ESD application workload

Digital-ESD is an application consumer of the canonical evidence machine, not a
new core substrate.

Its current production frontier is the real ERIC review loop:

```text
retained Q1-Q7 ERIC API exports
-> 46,597 query occurrences
-> 43,996 accession-deduplicated metadata studies
-> 43,996-row authoritative screening denominator
-> candidate-only title/abstract assessments
-> candidate duplicate/report-family fibres
-> reviewed calibration tranche
-> non-scalar Pareto reviewer queue
-> explicit include/probable/exclude/unresolved decisions
-> real full-text retrieval for include/probable
-> SHA-verified P0-G gate
-> canonical SLR evidence / source audit
```

The 43,996 metadata studies do **not** count as 43,996 verified full texts.
P0-G is paid only by real retrieved artifacts whose digests match their
retrieval receipts.

Operational details: `docs/digital-esd-real-eric-screening.md`.

# Shared User/World convergence (2026-09-22)

Canonical production addendum: `docs/shared_user_world_runtime_20260922.md`.

The next critical path is no longer revision-driven Mabo reopening.  Revision
and replay remain supporting infrastructure.  The production recurrence is:

```text
consumer/question
  -> exact dependency/residual
  -> shared-world lookup
  -> quotient reviewed in-scope coordinates
  -> residual-sensitive producer for missing coordinates
  -> explicit review/payment
  -> reviewed world delta
  -> affected-consumer recomputation
```

Current capability priority:

1. shared user/world lookup + affected-consumer propagation;
2. adversarial legal proof-runner weld over the existing dialectical,
   WrongType, minimal-cut and LegalFollow machinery;
3. heterogeneous empirical battery (Pabai; Yindjibarndi/Yunupingu/Mabo;
   Munkara/Tipakalippa; Murujuga; colonisation; personal handoff; mission);
4. unified workbench projections;
5. revision/replay maintenance as a cross-cutting invariant.

Permanent distinction:

```text
world availability != consumer dependency
consumer dependency != scope permission
scope permission != review/payment
review/payment != legal applicability
reachable legal route != predicted judicial outcome
```
