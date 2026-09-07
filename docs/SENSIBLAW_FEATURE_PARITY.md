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

## Current runtime status

The experimental offline research spine now materially implements R0-R5 and the
offline portion of R7. Validated/local fixtures cover multi-residual scheduling,
dialectical hypotheses, provider-neutral queries, local corpus execution,
proposition/citation/reasoning enrichment, immutable research memory and
multi-iteration compounding.

R6 is now an acquisition/failover layer rather than an AustLII dependency. Its
preferred Australian authority order is:

`persisted/local -> installed OALC exact MNC -> official court source -> optional sanctioned/specialist provider -> unresolved`

Current provider policy:

- OALC is the preferred bulk/local corpus substrate and is zero-network once installed;
- High Court of Australia and Federal Court of Australia are first-class official providers;
- AustLII public automated case-law access is not a required runtime dependency;
- JADE remains an optional citation/treatment discovery lane;
- provider failures such as `PolicyBlocked` and `TlsInvalid` are acquisition states,
  never negative proposition evidence.

The first experimental-online certification target is one bounded official HCA
fetch followed by immutable local ingestion and same-demand zero-network replay.
Production promotion remains separately gated by exact-head Agda/kernel receipts.

## Already present in slr

- revision-scoped parser spans and packed sentence carriers;
- direct/reference parity and direct-delta execution;
- candidate-only semantic expansion with unresolved/alternative fibres;
- fail-closed governed semantic admission;
- typed residual frontier and producer work selection;
- relation-attachment candidate production;
- orthogonal semantic/legal status product;
- admissible counterfactual world families with underidentification preserved;
- typed legal-source planning over persisted compatible revisions;
- evidential PNF handoff preserving document/hash/graph identities;
- sparse consumer/query/policy reopening over explicit reverse dependencies;
- offline-first costed proof-search scheduler and frontier-return loop;
- multi-residual `ProofFrontier` and whole-frontier scheduling;
- support/defeater/comparator/contradiction/treatment/terminology hypotheses;
- provider-neutral `QueryExpr` and local execution;
- proposition-level citation/reasoning/condition graph deltas;
- append-only source/research memory with revisable conclusions;
- deterministic iteration receipts through the v0.2 whole-frontier ABI;
- compounding offline iterations where learned vocabulary/authority neighbourhoods
  become later zero-network search context;
- governed-provider crate with explicit network isolation, pacing/bounds and replay;
- OALC exact-MNC snapshot/index path;
- official HCA/FCA provider/fetch paths;
- typed provider-access states and failover candidates.

## Core research-growth rule

Every newly parsed legal source should enrich the reusable legal/world graph even
when it does not close the current proof gap.

A parsed case should be able to contribute candidate coordinates for source/case
identity and revision, court/jurisdiction/time, opinion segment, propositions,
facts/conditions/circumstances, rules/tests, reasoning roles, authorities cited,
pinpoints, citation treatment, holding/ratio/obiter candidates, outcome/remedy,
exceptions/defeaters/burdens and lexical realisations.

The intended accumulation loop is:

`retrieve -> ingest -> parse -> PNF -> citation/reasoning extraction -> reviewed typed graph deltas -> frontier assessment -> next search`

The graph learned from one research run becomes zero-network context for later
runs. Search should therefore move progressively from text retrieval toward graph
proof search and missing-data acquisition.

Firewalls remain:

`citation != adoption != ratio != current authority`

`parsed proposition != truth`

`descriptive frequency != doctrine`

`statistical separator != legal cause`

`candidate WrongType != liability`.

## Rust implementation roadmap

### R0 - deterministic offline spine

Status: implemented and locally validated on the stacked proof-search PRs.

- deterministic source/runtime-pinned receipts;
- candidate-only authority boundary;
- replayable scheduler and return loop;
- source/document/digest/PNF/correspondence welds;
- zero-network fixtures.

### R1 - multi-residual proof frontier

Status: materially implemented in Offline Research Engine v0.1.

- first-class multiple residuals/payments/contested/authority-blocked coordinates;
- whole-frontier scheduling and shared-dependency value;
- candidate satisfaction distinct from legal proof closure;
- terminal states include contested, authority-blocked, underidentified, saturated
  and budget-exhausted, with budget exhaustion never meaning false.

### R2 - dialectical research hypothesis families

Status: materially implemented.

Native hypothesis kinds include support, defeater/exception, comparator/analogy,
contradiction/counterexample, authority treatment and terminology/identity discovery.
Supporting and defeating results update different coordinates rather than one
relevance score.

### R3 - provider-neutral query algebra

Status: materially implemented for local and pure provider lowering.

The query carrier supports terms, phrases, Boolean structure, proximity, ordered
proximity, citation, provision, court, jurisdiction and dates. Provider lowering
grants no semantic authority. Equivalent local queries may be coalesced across
residuals without inferring proof value from hit count.

### R4 - local corpus and immutable authority store

Status: materially implemented at the store-neutral/runtime-receipt layer.

- append-only source revisions and artifact digests;
- local query execution and reuse before live acquisition;
- query vocabulary and authority-neighbourhood accumulation;
- source rewrite attempts fail closed;
- missing local material remains acquisition work.

Durable production storage/migrations remain R10 work.

### R5 - proposition-level citation and reasoning graph

Status: materially implemented at candidate/review boundary.

Stable carriers retain source/opinion/proposition/citation/treatment,
condition/circumstance, reasoning role, outcome/remedy, burden/exception and lexical
coordinates. Graph enrichment does not itself establish ratio, binding force,
applicability or truth.

### R6 - governed Australian authority acquisition and failover

Status: implementation present; live official-source receipt still requires an
explicit operator run.

Preferred acquisition order:

1. compatible persisted/local source revision;
2. installed OALC exact-MNC index;
3. official court publication (currently HCA/FCA first-class);
4. explicit/sanctioned specialist provider when available;
5. unresolved.

Implemented contracts:

- separate `sensiblaw-governed-legal-provider`; scheduler owns no HTTP;
- default legal-host pacing `0.25 rps`, burst `1`;
- explicit `max_depth`, `max_new_documents`, and network-request budget;
- cache and persisted receipts checked first;
- no crawling/ad-hoc polling;
- search returns references only;
- fetch returns bytes only;
- parser/PNF runs only after local ingestion;
- SHA256-bound source revisions and same-demand zero-network replay;
- optional live transport behind `live-network` + explicit operator opt-in;
- OALC streaming JSONL indexing against the published corpus schema, retaining
  MNC/source/version/text-digest coordinates rather than loading full corpus text;
- official HCA calibration for `[2026] HCA 19` and FCA calibration for
  `[2025] FCA 796`;
- `ProviderAccessStatus` distinguishes `Available`, `PolicyBlocked`, `TlsInvalid`,
  `TemporarilyUnavailable`, `AuthorisationRequired`, and `NotConfigured`;
- provider failure never becomes negative legal evidence;
- AustLII public/reference and authorised/sanctioned access modes remain explicit;
- JADE is optional treatment/citation discovery rather than a mandatory document lane.

R6 acceptance gate:

`official HCA known authority -> exactly one governed fetch -> SHA256/local source revision -> replay same demand -> network_requests=0`

The fixture form is locally deterministic. Experimental-online readiness is reached
only after an intentional real official-source fetch emits a receipt accepted by
`scripts/verify_live_legal_receipt.py`.

### R7 - iterative online/offline research engine

Status: offline compounding materially implemented; online acquisition handoff is
next after the R6 live receipt.

Next work after R6 certification:

- let a scheduled live acquisition return through immutable ingestion/PNF;
- extract citation/reasoning/treatment deltas from the newly acquired authority;
- update only affected proof residuals;
- add newly learned source/citation/lexical coordinates to local memory;
- require the next iteration to prefer the newly persisted artifact at zero network.

Saturation remains frontier-relative and must require repeated stability across
independent search families.

### R8 - precedent geometry and discriminator search

After enough cases are structurally represented:

- nearest comparable/opposite-outcome cases;
- minimal differing coordinate;
- candidate minimal separator/counterexample searches;
- separators remain candidate discriminators, not doctrine or legal cause.

### R9 - corpus-derived lexical fibres and descriptive jurisprudence

- learn court/time/jurisdiction lexical realisations;
- use them for query expansion while preserving `query expansion != proof expansion`;
- descriptive authority-treatment and outcome statistics may rank search but cannot
  create legal rules, holdings, truth or causation.

### R10 - durable governance/publication

- append-only durable receipt/world-model store;
- replay/migration/parity across Rust versions and Agda owners;
- promotion/publication boundary distinct from parsing/retrieval/graph/statistics;
- cryptographic identity/signature at receipt/promotion boundaries, not inference.

## Immediate acceptance milestone: Offline Research Engine v0.1

Status: achieved on the experimental Rust line.

The engine now demonstrates a real multi-residual frontier, opposing/comparator
hypotheses, provider-neutral/local queries, persisted/local execution, graph
richment, sparse frontier updates, Pareto rescheduling and deterministic JSON with
`network_requests=0`. Subsequent compounding fixtures reuse knowledge learned by
prior iterations.

The next acceptance milestone is **Governed Official Acquisition v0.1**:

1. resolve a known MNC locally/OALC first;
2. if absent, select an official HCA/FCA source;
3. execute one explicitly bounded live fetch;
4. bind bytes to SHA256 and immutable source revision;
5. ingest locally before PNF;
6. replay the identical demand from persisted material with zero network;
7. emit a deterministic candidate-only acquisition receipt;
8. route the acquired authority into the normal R7 reasoning/frontier loop.

## Canonical formal counterparts

The formal direction remains Agda parity/authority over Rust semantics, not a
Python port. Relevant owners cover proof-directed corpus search/query algebra,
governed legal-network strategy, Pareto scheduling, iterative frontier search,
reasoning/citation graph enrichment, immutable research memory, bounded Rust receipt
ABIs and governed-online provider boundaries.

Historical SensibLaw remains reference-only for architecture/acquisition behaviour;
it does not define current Rust or Agda semantics.

## Explicit non-goals

- no Python-to-Rust line-by-line port;
- no PostgreSQL in sentence-local compilation;
- no parser token writes as production semantic state;
- no uncontrolled crawler;
- no missing source/provider failure -> negative fact conversion;
- no citation -> adoption/ratio/current-authority conversion;
- no parser/proximity/statistics -> holding/truth conversion;
- no causal dependence -> liability/remedy/authority promotion.
