# SensibLaw -> slr feature-parity frontier

This document tracks behavioural/contract parity and the forward Rust roadmap. It
is not a mandate to port Python/PostgreSQL implementation details into the Rust
hot path.

## Governing performance rule

`slr` remains the direct-delta execution kernel. New legal workflow capabilities
must compile from already-produced semantic/status/residual carriers and must not
force parser reruns, document rescans, DB crossings, provider I/O or publication
on sentence-local execution.

The legal-research loop is downstream of sentence-local parsing:

`proof frontier -> candidate research moves -> Pareto schedule -> local/persisted/live execution strategy -> local ingestion -> PNF/reviewed correspondence -> frontier delta -> sparse wake -> reschedule`

Live provider execution is never implicit. Network access is a separately governed
strategy and remains outside the semantic hot path.

## Already present in slr

- revision-scoped parser spans and packed sentence carriers;
- direct/reference parity and direct-delta execution;
- candidate-only semantic expansion with unresolved/alternative fibres;
- fail-closed governed semantic admission;
- typed residual frontier and producer work selection;
- relation-attachment candidate production;
- orthogonal semantic/legal status product, including proposition/truth,
  occurrence, jurisdiction, authority, applicability, violation and liability;
- admissible counterfactual world families with underidentification preserved;
- typed legal-source planning over persisted compatible revisions;
- evidential PNF handoff preserving document/hash/graph identities;
- sparse consumer/query/policy reopening over explicit reverse dependencies;
- experimental offline-first costed proof-search scheduler;
- experimental offline frontier-return loop;
- deterministic offline Pabai JSON receipt at
  `sensiblaw.offline-proof-search-loop-receipt.v0_1`.

Current validated experimental path:

`Pabai proof gap -> persisted Cullen comparator -> local artefact -> PNF/reviewed correspondence -> Narrowed frontier -> next persisted authority move`

with `network_requests = 0` and `experimental_candidate_only` authority throughout.

## Core research-growth rule

Every newly parsed legal source should enrich the reusable legal/world graph even
when it does not close the current proof gap.

A parsed case should be able to contribute candidate coordinates for:

- source/case identity and exact revision;
- court, jurisdiction and temporal context;
- opinion/judge/speaker segment;
- issues and propositions discussed;
- facts/conditions/circumstances relevant to each proposition;
- rules/tests/standards articulated;
- reasoning step or argumentative role;
- authorities cited;
- pinpoint citation context;
- citation-use/treatment candidate (`mentioned`, `quoted`, `relied_on`, `adopted`,
  `applied`, `followed`, `distinguished`, `criticised`, `rejected`, `overruled`,
  `party_submission`, `historical_background`, `unresolved`);
- holding/ratio/obiter candidate status, without automatic promotion;
- outcome/remedy/disposition candidate;
- conditions, exceptions, defeaters and burden/standard coordinates;
- lexical realisations used by that court/time/jurisdiction for known concepts.

The intended accumulation loop is therefore:

`retrieve -> ingest -> parse -> PNF -> citation/reasoning extraction -> reviewed typed graph deltas -> frontier assessment -> next search`

The graph learned from one research run becomes zero-network context for later
runs. Search therefore gradually shifts from text retrieval toward direct graph
proof search and missing-data acquisition.

Important firewalls remain:

`citation != adoption != ratio != current authority`

`parsed proposition != truth`

`descriptive frequency != doctrine`

`statistical separator != legal cause`

`candidate WrongType != liability`.

## Rust implementation roadmap

Priority is ordered to maximise proof-search utility while preserving the current
performance and authority boundaries.

### R0 - freeze and receipt the offline spine

Status: materially implemented by PR #8/#9.

1. Keep `sensiblaw-proof-search-scheduler` and `sensiblaw-proof-search-loop`
   candidate-only until the corresponding Agda ABI has an exact-head kernel receipt.
2. Standardise deterministic iteration receipts containing:
   - runtime head/schema;
   - proof gap/frontier identity;
   - candidate moves and cost/value vectors;
   - Pareto frontier and selected move;
   - source revision/document/digest identity;
   - PNF/reviewed-correspondence receipts;
   - assessment/frontier delta;
   - sparse wake set;
   - next move;
   - network count and authority boundary.
3. Require deterministic replay: same typed inputs/policy -> same candidate receipt.
4. Preserve source/runtime SHA provenance so later code cannot silently inherit an
   older validation receipt.

### R1 - multi-residual proof frontier

Next highest-priority runtime tranche.

1. Add a first-class `ProofFrontier` containing multiple simultaneous residuals,
   payments, contested coordinates, authority-blocked coordinates and dependency
   edges.
2. Each residual carries target proposition, producer class, jurisdiction,
   authority requirement, salience, dependencies and current status.
3. Schedule across the whole frontier rather than one residual at a time.
4. Preserve non-monotone conclusions over append-only evidence/provenance.
5. Add terminal classifications: `Closed`, `Contested`, `AuthorityBlocked`,
   `Underidentified`, `Saturated`, `BudgetExhausted`; budget exhaustion never means
   proposition false.

### R2 - dialectical research hypothesis families

1. Compile every live residual into typed search hypotheses rather than one
   confirmation-oriented query.
2. Native hypothesis kinds should include:
   - support;
   - defeater/exception;
   - comparator/analogy;
   - contradiction/counterexample;
   - authority-treatment/noting-up;
   - terminology/identity discovery.
3. Rank all admissible hypotheses on the existing proof-value/cost Pareto surface.
4. A supporting result and an opposing/defeating result update different frontier
   coordinates rather than being collapsed into one relevance score.

### R3 - provider-neutral query algebra

1. Port the Agda query algebra as store/provider-neutral Rust types.
2. Support term, phrase, conjunction, disjunction, negation, proximity, ordered
   proximity, citation identity, provision, court, jurisdiction and date filters.
3. Keep proximity candidate-generating only:
   textual proximity does not imply a semantic/legal relation.
4. Add pure compilers for:
   - local corpus queries;
   - AustLII SINO (`near`, `w/n`, `/n/`, `pre/n`);
   - JADE-style search/citation traversal plans.
5. Provider lowering never grants semantic authority.

### R4 - local corpus and immutable authority store

This should precede live provider execution.

1. Build a store-neutral append-only `SourceRevision`/`AuthorityArtifact` interface.
2. Persist canonical bytes/text digests, provider/source receipts and jurisdiction/
   role/authority candidates.
3. Execute the same query algebra against local material first.
4. Index phrase/proximity/citation/source/jurisdiction/time coordinates.
5. Reuse already-parsed PNF and graph receipts whenever identity/digest contracts
   match; do not rerun parsing merely because a new consumer appears.
6. Missing local source remains acquisition work, never negative evidence.

### R5 - proposition-level citation and reasoning graph

This is the main knowledge-compounding stage.

1. Add stable carriers for `Case`, `OpinionSegment`, `Proposition`,
   `CitationOccurrence`, `CitationIdentity`, `CitationUse`, `AuthorityTreatment`,
   `Condition/Circumstance`, `ReasoningStep`, `Outcome` and provenance receipts.
2. When a newly parsed judgment cites another authority, emit a candidate citation
   edge immediately and route unresolved citation identity to the appropriate
   producer.
3. For each cited proposition, retain:
   - who invoked it;
   - where in the judgment;
   - what proposition it was used for;
   - under what factual/doctrinal conditions;
   - whether the court followed, applied, distinguished, criticised, rejected,
     overruled or merely mentioned it;
   - whether the text is the court's reasoning, a party submission, quotation or
     historical background.
4. Extract candidate reasoning topology:
   `facts/conditions -> rule/test -> comparison/distinction -> conclusion/outcome`.
5. Graph enrichment does not itself establish ratio, binding force, applicability
   or truth; those remain separately reviewed/typed coordinates.
6. Newly discovered citation edges feed directly back into authority-treatment,
   comparator and cited-by search moves.

### R6 - governed AustLII/JADE execution

Only after R1-R5 are stable enough that live acquisition is a bounded extension
rather than the reasoning engine itself.

1. Introduce a separate governed legal-provider interface; the scheduler never
   owns HTTP.
2. Preserve existing legal-network constitution:
   - default legal-host pacing `0.25 rps`, burst `1`;
   - explicit `max_depth` and `max_new_docs`;
   - local cache/persisted receipts first;
   - no crawl and no repeated ad-hoc polling;
   - search returns references;
   - fetch returns bytes;
   - parser/PNF begins only after local ingestion.
3. Live strategy cost exposes requests, pacing delay, breadth/depth, cache misses
   and expected downstream parse/review cost to Pareto scheduling.
4. Direct utility scripts such as raw `urlopen` acquisition are not batch
   proof-follow executors unless wrapped by the governed layer.

### R7 - iterative research engine

1. Allow the scheduler/return loop to iterate across the multi-residual frontier.
2. Each iteration emits a deterministic receipt and updates only affected fibres.
3. Search may move between local graph traversal, persisted authorities and
   governed live acquisition according to Pareto value/cost.
4. Saturation is frontier-relative and must require repeated stability across
   independent search families; one unchanged result is insufficient.
5. The world/legal graph grows monotonically in provenance/artifacts while current
   conclusions remain revisable when new exceptions, defeaters or treatments are
   discovered.

### R8 - precedent geometry and argument/discriminator search

After enough cases are structurally represented:

1. Represent case fibres over facts, issues, rules, arguments, holdings, citation
   treatments, conditions/circumstances and outcomes.
2. Support nearest comparable case, nearest opposite-outcome case and minimal
   differing-coordinate searches.
3. Search for candidate minimal separators between otherwise similar outcome
   classes.
4. Treat those separators as proof-search/discriminator candidates, never as
   doctrine or legal causation without authority/reasoning receipts.
5. Feed discovered discriminators back into R2 hypothesis generation and R3 query
   compilation.

### R9 - corpus-derived lexical fibres and descriptive jurisprudence

1. Learn court/time/jurisdiction-specific lexical realisations of typed concepts
   from the accumulated corpus.
2. Use those lexical fibres for query expansion while keeping
   `query expansion != proof expansion`.
3. Compute descriptive statistics only over provenance-preserving structured
   coordinates, for example authority treatment frequencies, argument/outcome
   associations and conditional case clusters.
4. Statistical output may rank searches and reveal candidate distinctions, but
   cannot create legal rules, holdings, truth or causation.

### R10 - durable governance/publication

1. Append-only durable receipt/world-model store.
2. Replay/migration/parity receipts across Rust versions and formal Agda owners.
3. Governed promotion/publication boundary distinct from parser, retrieval,
   graph-enrichment and statistical layers.
4. Cryptographic identity/signature work belongs at receipt/promotion boundaries,
   not inside semantic inference.

## Immediate acceptance milestone: Offline Research Engine v0.1

Before enabling live network execution, the runtime should demonstrate:

1. start from a real multi-residual Pabai/Mabo proof frontier;
2. generate support/defeater/comparator/treatment hypotheses;
3. compile them to provider-neutral queries;
4. execute against persisted/local authority material only;
5. parse or reuse locally ingested sources;
6. enrich proposition/citation/reasoning/condition/outcome graph coordinates;
7. assess the resulting frontier deltas;
8. reopen only dependent consumer fibres;
9. choose the next Pareto-optimal move;
10. emit a deterministic JSON receipt for each iteration with `network_requests=0`.

That milestone proves the reasoning/search loop independently of network behaviour.
Governed AustLII/JADE then becomes an additional acquisition strategy rather than
an architectural dependency.

## Canonical formal counterparts

The current formal direction is DASHI/SensibLaw Agda-first. Relevant owners include
legal factual/but-for causation, corrected-world counterfactual discipline,
semantic-status products, proof-directed corpus search/query algebra, governed
legal-network strategy, costed/Pareto proof-search scheduling, iterative frontier
search, and exact bounded Rust JSON receipt owners.

Historical SensibLaw remains reference-only for architecture/acquisition behaviour;
it does not define current Rust or Agda semantics.

## Explicit non-goals

- no Python-to-Rust line-by-line port;
- no PostgreSQL in sentence-local compilation;
- no parser token writes as production semantic state;
- no uncontrolled crawler in legal research;
- no regex/lexical shortcut to legal truth or roles;
- no `proper noun -> external provider` shortcut;
- no missing source/evidence -> negative fact conversion;
- no citation -> adoption/ratio/current-authority conversion;
- no parser/proximity/statistics -> holding/truth conversion;
- no causal dependence -> liability/remedy/authority promotion;
- no physically imagined alternative -> realised-world promotion.

## Cross-domain formal boundary

Legal and physical counterfactual bridges reuse a common discipline while
remaining semantically and authoritatively distinct. TSFV/multiverse machinery
supplies a realisation/admissibility warning only: a parameter point or viable
history is not automatically a physically realised world. It does not supply
legal doctrine or authority.
