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

# 2026-10-23 cross-suite research-cockpit interface addendum

**Owner boundary:** `itir-dioxus` owns operator visual projections and wgpu interaction; ITIR MCP/AO owns governed fleet dispatch, receipt lookup and reconciliation; StatiBaker owns temporal observer memory; SLR owns producer-side canonical evidence, scoped semantic admission and **experimental candidate-only** proof-search scheduler/loop. Neither Dioxus nor the fleet may turn model output, PNF lexical hints or a good-looking graph into admitted truth.

**New operator user story:** trace from mission/obligation -> competing producer attempts -> source/revision/span observations -> proof-search frontier deltas -> review/admission -> exact compiler/kernel receipt -> next residual. Preserve alternative unsuccessful attempts, source-local identity and explicit abstentions. Operator may choose a candidate strategy or request a review through governed AO commands, not a direct semantic write.

**Roadmap acceptance**
- P0: produce a bounded, stable typed read projection keyed by canonical producer references and exact revisions; preserve residual kind, provenance, status, scope, cost-vector dimensions and prior-hop receipt links; explicit unavailable fields are better than inferred claims.
- P0: demonstrate restart/replay consistency and deep-link to observation/span/receipt for a real persisted mission; no second canonical store or JSON-as-semantic-ABI (JSON is permitted for export/debug).
- P1: connect proof-search schedule/return receipts to Dioxus `VisualisationIr` through a framework-neutral typed projection. Provide independent axes for verification status, information/proof gap reduction, time and cost; no scalar score as authority.
- P1: distinguish source acquisition, reviewed correspondence, admission, publication, model-generated proof, local test and exact-head Agda/Lean kernel certification.
- P2: experimental Cogentic-style prover/critic population and Self-Harness/MESH-inspired optimisation may only select **candidate** strategies; verify on held-out tasks with explicit resource budgets, negative-result retention and a human/typed admission gate.

**References:** `itir-dioxus/docs/M10_UNIFIED_WORKBENCH.md`; `ITIR-suite/itir-mcp/README.md`; `ITIR-suite/docs/planning/governed_revival_handoff_v1.md`; `StatiBaker/docs/user_stories.md`; Cogentic https://arxiv.org/abs/2609.40324, Self-Harness https://arxiv.org/abs/2606.09498, MESH https://arxiv.org/abs/2610.05300. External studies motivate research hypotheses, not production performance claims. **Status:** planning only; original Sprint 1 certification and all existing acceptance gates unchanged.
## PRODUCER-CONFORMANCE-1 — ubiquitous external-producer acceptance (2026-11-21)

**Status:** proposed targets / OPEN; no live CourtListener ingestion, differential
benchmark, or full-stack acceptance is claimed by this roadmap update.

**Goal:** make every external or internal competing system a possible upstream
producer and independent comparator for the existing SLR/SensibLaw/ITIR world
fabric. Integration must not create another canonical parser, PNF engine,
semantic store, legal truth authority, reviewer, or fleet controller. SLR remains
the production legal evidence/admission consumer; PostgreSQL is the canonical
semantic persistence authority; DASHI/Agda is the golden reference; ITIR/MCP
owns governed operation receipts and `itir-dioxus` M10/M11 owns new production
user-facing projections. Producer agreement corroborates but does not prove;
disagreement is preserved as a reviewable obligation.

**Existing implementation donors (reuse before writing adapters):**

- SLR [#57](https://github.com/chboishabba/slr/pull/57) provider-pinned
  source materialisation; [#56](https://github.com/chboishabba/slr/pull/56)
  reviewed-evidence durability; [#55](https://github.com/chboishabba/slr/pull/55)
  heterogeneous REL/interlingua;
  [#61](https://github.com/chboishabba/slr/pull/61) provider-authoritative
  external bindings;
  [#62](https://github.com/chboishabba/slr/pull/62) consumer-relative parity;
  [#65](https://github.com/chboishabba/slr/pull/65) formal proof routing;
  [#67](https://github.com/chboishabba/slr/pull/67) sibling-manifestation
  equivalence; [#78](https://github.com/chboishabba/slr/pull/78)
  source-bound parser evidence reopening.
- SensibLaw
  [#494](https://github.com/chboishabba/SensibLaw/pull/494) atomic legal parity,
  [#514](https://github.com/chboishabba/SensibLaw/pull/514) cross-repo
  source-preserving user-story handoff, and
  [#515](https://github.com/chboishabba/SensibLaw/pull/515) raw UTF-8 source digest
  alignment.
- ITIR-suite [#17](https://github.com/chboishabba/ITIR-suite/pull/17)
  federated typed world,
  [#18](https://github.com/chboishabba/ITIR-suite/pull/18)
  proof-directed acquisition, and
  [#57](https://github.com/chboishabba/ITIR-suite/pull/57)/
  [#58](https://github.com/chboishabba/ITIR-suite/pull/58) canonical PostgreSQL
  and formal referee boundaries.
- User-story coverage: ITIR-US-01–03/17, SL-US-09–14/18–24/31,
  WIKI-US-01–04, SLR-US-01–08 in
  [ITIR suite stories](https://github.com/chboishabba/ITIR-suite/blob/main/docs/user_stories.md);
  platform integrator, institutional buyer, auditor, annotation and
  cross-institutional handoff personas in
  [SensibLaw stories](https://github.com/chboishabba/SensibLaw/blob/main/docs/user_stories.md).

### PC1 acceptance targets (all OPEN until independently executed)

- **PC1.01 — Producer inventory and contract:** enumerate source families,
  record API/schema/library versions, permissions, terms/licences, access
  limits, export availability, and trusted identifier scope; distinguish a
  source producer, derived extractor, retrieval index, and formal referee.
- **PC1.02 — Lossless, revision-pinned intake:** a live externally fetched
  object enters existing manifestation/source-revision/observation paths with
  producer-native IDs, original bytes or immutable provider reference,
  digest, precise source coordinates, acquisition timestamps, access scope,
  provider revision and transformation ancestry. A cache hit performs no
  network work. No inference or ingestion receipt promotes semantic truth.
- **PC1.03 — Native-output preservation:** retain provider assertions,
  metadata, document/entry relationships and extractor output separately from
  canonical reviewed evidence; refuse guessed identity equivalence and
  distinguish multiple manifestations of one authority from independent
  authorities. Missing/partial/restricted input must remain explicit.
- **PC1.04 — Differential reference corpus:** for the *same* source bytes and
  pinned producer versions, compare native SLR output against independent
  producer outputs on text/OCR, structural segmentation, citation boundaries
  and targets, case/actor identity, docket chronology, assertion status and
  citation-graph edges. Record exact mismatches, false joins, omissions,
  abstentions and reference-provider errors; manually adjudicate selected
  disputed cases. Third-party agreement is not ground truth.
- **PC1.05 — Correction, replay and nonpromotion:** restart, cursor pagination,
  retries, deduplication, provider amendments, retractions, access revocation
  and deletion signals reproduce durable lineage; previous receipts remain
  auditable under access policy. Updated evidence reopens only dependent
  consumer fibres; no producer or UI silently overwrites reviewed history.
- **PC1.06 — Cross-provider contradiction:** inject contradictory and
  partially overlapping source assertions; persist independent competing
  claims and residuals with provenance. Convergence must not manufacture
  consensus, case identity, legal holding, or authority from common phrasing.
- **PC1.07 — Reviewed semantic and formal handoff:** reuse existing SLR
  candidate PNF/REL, review/payment, normative order, proof-gap and DA51
  referee boundaries. Independently certified proof artifacts are distinct
  from parser evidence, source retrieval, and proposed legal conclusions.
- **PC1.08 — Consumer and UX acceptance:** project the *same* persisted Matter
  into client/community, solicitor/counsel, and court/associate read-only
  views in production `itir-dioxus`; exact source and discrepancy receipts
  reopen. Unknown, absent, blocked and restricted are distinct. Display,
  sorting or graph depth cannot create authority.
- **PC1.09 — Repeatable campaign receipt:** deliver an exact-head, pinned
  corpus, source hashes, upstream versions, deterministic test outputs,
  latency/request/byte cost, failure cases and independently reviewable
  acceptance report. Unit tests, mock API payloads or documentation alone
  do not close a live empirical gate.

### First candidate campaign — US federal judicial producers

- **CourtListener/RECAP:** a governed, bounded read-only ingestion specimen
  using available documented API/bulk interfaces for court, docket, docket
  entry, RECAP document, opinion and citation relationships. Preserve native
  IDs and docket/document revision ancestry. Authenticate and respect provider
  limits; do not assume every PACER document is publicly available.
- **Eyecite:** independent citation extraction/annotation comparator.
- **Centralia:** independent opinion segmentation/structure comparator.
- **Doctor:** document conversion/OCR/text-extraction comparator.
- Keep the original source PDF or authoritative docket record, upstream
  derived output, and SLR derived output as three separately identifiable
  comparison layers. Sample before considering bulk embeddings or full
  archive replication. Check licence compatibility before incorporating
  upstream source code.
- Compare on the same judicial records; keep US federal jurisdiction separate
  from existing Australian OALC/HCA/legislation authority and applicability
  targets. Include a second provider family after the first campaign passes,
  without adding a provider-specific semantic engine.

**Delivery boundaries:** PC1 is a cross-cutting acceptance campaign for
Sprint-2 acquisition, Sprint-3 legal semantics, and Sprint-4 product
projections, not a new sprint or a reason to reopen paid canonical
architecture. Prefer a bounded test against an actual consumer residual over
indiscriminate corpus crawling. All targets remain OPEN until exact producer
receipts and locally repeatable empirical results are recorded.

