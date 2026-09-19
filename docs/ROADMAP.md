# SensibLaw / SLR Production Roadmap

**Status:** current production roadmap  
**Production implementation:** `chboishabba/slr`  
**Golden semantic/reference contracts:** `chboishabba/dashi_agda`

SLR is SensibLaw in production. DASHI/Agda is the golden reference layer: it owns typed semantic, promotion, epistemic, and architectural contracts that the Rust runtime must implement or conservatively refine. The Python SensibLaw repository remains useful as historical/product/reference material, but it is not the production runtime authority.

## Product invariant

The production path is:

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

No transport, parser, ontology provider, cache hit, graph adjacency, classifier output, or review artifact may silently create claim truth, legal authority, applicability, or publication authority.

## What is already paid

The roadmap must not reopen these as greenfield work unless a regression demonstrates a real defect:

- canonical source revisions, spans, exact provenance and append-only receipts;
- binary world store / compiler / consumer-residual pipeline;
- typed ConsumerRequirement v2 separation between parser/PNF debt and substantive evidence debt;
- residual-driven producer planning with distinct producer families;
- route selection and bounded route execution;
- explicit reviewed evidence payment;
- append-only active residual frontier and restart-safe recurrent expansion;
- PostgreSQL persistence/replay and durable hop lineage;
- CandidateWorld projection and review/promotion firewalls;
- GWB adaptive residual selection, post-hop re-diagnosis and producer switching;
- supervised bounded P31/P279 closure with truncation -> abstention;
- snapshot-first -> live-fallback type provider seam;
- external-ontology fallback as advisory candidate evidence only;
- legal-source/provider contracts and PG-backed source storage already present in the workspace.

The architectural problem is now **composition and throughput**, not another isolated extractor.

## Sprint A — Bounded ontology transport closure

**Purpose:** make GWB cheap and deterministic enough that transport no longer dominates world-expansion behaviour.

Deliver as one capability tranche:

1. specialised P31/P279 predicate-slice provider;
2. general Zelph/HF route-aware fallback using `nodeRouteIndex`;
3. physical-object planning after logical QID/name resolution;
4. deduplication/coalescing by physical HF object;
5. bounded remote scheduler with an initial cap of five distinct cold objects in flight;
6. cache-hit = zero network I/O;
7. transport receipt with semantic targets, internal nodes, shard/object plan, cache hits/misses, coalesced gets, remote gets, bytes, fallback count and closure observations;
8. preserve snapshot/live provenance and revoke snapshot simultaneity if any live fallback occurs;
9. truncation/retrieval gap remains abstention, never ontology negation.

### Sprint A exit gate

A representative GWB replay must demonstrate all of:

- repeated logical requests sharing a physical object cause at most one SLR acquisition;
- six distinct cold objects never exceed five concurrent remote fills;
- cached replay performs zero remote object fetches;
- ordinary P31/P279 classification does not pull the complete adjacency surface;
- mixed snapshot/live evidence is visibly mixed;
- no new semantic-authority or claim-truth path exists;
- transport cost is reported independently of semantic payoff.

After this gate, **stop optimising Wikidata/HF unless campaign telemetry identifies transport as the dominant residual cost again**.

## Sprint B — Generic epistemic scheduler

**Purpose:** promote the successful GWB controller from a Wikidata-oriented experiment into the generic SensibLaw acquisition control plane.

Unify the existing Rust/Agda machinery around:

```text
ConsumerRequirement
  -> ResidualFamily
  -> ProducerPlan
  -> boundedness/cost declaration
  -> execution
  -> EvidenceManifestation
  -> review/payment
  -> residual recomputation
```

Required producer capabilities include:

- local deterministic/parser repair;
- PostgreSQL replay;
- specialist snapshot/slice;
- general snapshot;
- identity/source discovery;
- authority/legal-source acquisition;
- classification/ontology evidence;
- mechanism/measurement/comparator evidence;
- live authoritative source;
- human review handoff.

The planner selects evidence-producing work. It does not decide truth.

### Sprint B exit gate

One recurrent campaign must traverse at least three producer families through the same scheduler ABI, persist every hop, restart from PostgreSQL without changing the active frontier, and distinguish:

- producer unavailable;
- retrieval failure;
- candidate absence;
- reviewed rejection;
- paid requirement.

No family-specific GWB control loop may be required for the campaign.

## Sprint C — Canonical evidence reducer + legal acquisition convergence

**Purpose:** make world evidence and legal-source evidence enter one canonical evidence substrate without creating parallel identity stores.

Production contract:

```text
acquisition manifestations
   -> canonical reducer
   -> source/span/observation/evidence state
   -> lawful projections
        - world model
        - matter/timeline
        - legal/normative model
```

Converge the existing source-store/provider work so OALC/official/Jade/other jurisdictional providers are ordinary producer capabilities:

```text
SourceRequirement
 -> PG exact revision hit ? replay
 -> otherwise governed acquisition
 -> revision identity + receipt
 -> persist
 -> canonical TextSpan/observation
```

### Sprint C exit gate

The same canonical reducer must ingest and replay:

- one Wikidata/Wikimedia evidence manifestation;
- one legal authority/source revision;
- one narrative/matter observation;

with no duplicate canonical identity substrate and with provenance drill-down to the exact acquired revision/span for each.

## Sprint D — Reviewed world -> legal issue weld

**Purpose:** connect the already-existing L0-L6 legal model to reviewed world evidence by typed projection rather than inference-by-association.

Target relation:

```text
ReviewedObservation
 -> EventCandidate
 -> reviewed Event
 -> ClaimEvent / ActorConstraint / HarmInstance evidence
 -> candidate WrongElementRequirement satisfaction
```

Separately:

```text
revision-pinned legal source
 -> Provision
 -> duty / WrongTypeSourceLink / WrongElementRequirement
```

Firewalls:

- event existence != wrong;
- harm != wrong;
- Wikidata class != legal category;
- source presence != legal applicability;
- candidate element match != satisfied element;
- one jurisdiction's rule != another's rule.

### Sprint D exit gate

Run one complete Australian legal issue from matter evidence plus pinned authority to an element-by-element **support / contradiction / unknown** issue state, where every support edge is source-addressable and no unknown is silently coerced to false.

## Sprint E — Legal reasoning kernel

**Purpose:** make SensibLaw useful as a legal reasoning system rather than merely a world/evidence graph.

Implement composition across:

- elements;
- conditions and exceptions;
- defences;
- temporal applicability / as-at law;
- jurisdiction;
- competing authorities;
- authority/citation topology;
- burden/standard metadata where represented;
- remedy candidates;
- unresolved and contested fibres.

Outputs remain inspectable proof/explanation graphs, not opaque LLM verdicts.

### Sprint E exit gate

A reviewer can inspect a complete issue graph and answer:

- what proposition is being tested?
- what authoritative source supplies the rule?
- what evidence supports or contradicts each element?
- what is still unknown?
- what defence/exception changes the result?
- what source revision/date was used?
- what acquisition would most reduce the remaining residual?

## Sprint F — Productisation

Build the human-facing surface over the same receipts and typed state:

- matter/timeline;
- source viewer;
- issue/element graph;
- "why?";
- "show evidence";
- "what changed as at date X?";
- "which element is disputed?";
- "what source controls this?";
- "what should we acquire next?";
- cross-jurisdiction comparison;
- exportable receipt packs.

No separate UI reasoning ontology.

## Sprint discipline: stop taking baby steps

A sprint is not "add a struct", "add a provider", or "add one adapter". A sprint must close an **observable end-to-end capability** with a receipt.

Use these rules:

1. **Work by min-cut, not file count.** Identify the smallest set of missing capabilities blocking the next end-to-end behaviour and implement the whole cut.
2. **Parallelise independent owners.** Transport, formal contract, storage/replay, and validation can advance in parallel where their ABI is already fixed.
3. **Do not reopen paid architecture.** Reuse existing Agda owners and Rust crates unless an executable counterexample proves the contract inadequate.
4. **No speculative provider expansion.** Add providers only when a live residual family demands them.
5. **No transport rabbit holes.** Once boundedness/replay/cost gates pass, move upward to scheduler, reducer, and law.
6. **Every sprint ends in a campaign receipt.** Static types and unit tests are necessary but not the product milestone.
7. **Golden parity is continuous.** Any production semantic contract change either:
   - already refines an existing Agda owner, or
   - lands with the corresponding Agda golden update before the sprint is called complete.

## Current critical path

```text
A  bounded HF/P31-P279 transport
 -> B generic epistemic scheduler
 -> C canonical reducer + legal source convergence
 -> D reviewed world-to-law weld
 -> E legal reasoning kernel
 -> F product surface
```

The next sprint is **A as a complete tranche**, not five separate mini-projects.
