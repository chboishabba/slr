# SensibLaw -> slr feature-parity frontier

This document tracks behavioural/contract parity and the forward Rust roadmap. It
is not a mandate to port historical Python/PostgreSQL implementation details into
the Rust hot path.

## Governing runtime rule

`slr` remains the direct-delta execution kernel. Legal research is downstream of
sentence-local parsing:

`proof frontier -> candidate moves -> Pareto schedule -> local/OALC/official acquisition -> local ingestion -> PNF/reviewed correspondence -> reasoning graph delta -> frontier delta -> sparse wake -> reschedule`

Provider execution is never implicit. Search returns references, fetch returns
bytes, and semantic/legal interpretation begins only after local ingestion/review.

## Current runtime status

The experimental offline research spine materially implements R0-R5 and the
offline/return portions of R7. Local validation covers multi-residual scheduling,
dialectical hypotheses, provider-neutral queries, local corpus execution,
proposition/citation/reasoning enrichment, immutable research memory and
multi-iteration compounding.

The preferred Australian acquisition order is now:

`persisted/local -> installed OALC exact MNC -> official court -> optional sanctioned/specialist provider -> unresolved`

AustLII and JADE are not mandatory runtime dependencies. Provider failures such as
`PolicyBlocked` and `TlsInvalid` are acquisition coordinates only and never become
negative proposition evidence.

### Experimental online milestone reached

A bounded official High Court acquisition succeeded for `Cullen v New South Wales
[2026] HCA 19`:

`official HCA landing -> 1 request -> SHA256-bound local ingestion -> same-demand persisted replay -> 0 requests`

Observed artifact digest:

`sha256:5959632dbf3d88c3ddd96addb200dacdccf9d4aaf56cdccfacd8ca2b92cda991`

The retained receipt is `sl.governed_legal_acquisition.v0_1` with
`experimental_candidate_only` authority and no semantic/legal authority claims.
It embeds runtime head `9c3007be97f7e4a1e9a8bc9c7c85b92368515935`.

The later repair head `bb6de859ca82700cba70d2784f11c39a2c4c1826` passed local CI. The
`9c3007..bb6de85` delta does not change the governed provider runtime library; a
separate no-network lineage audit records that fact. This is sufficient for
experimental capability calibration, but it is deliberately not called
byte-for-byte build identity or exact-current-head live execution.

The readiness state is therefore:

`ExperimentalLiveAcquisitionReady`

while production remains blocked by exact-head live/runtime provenance and the
corresponding exact-head Agda/kernel receipt.

## Already present in slr

- revision-scoped parser spans and direct/reference parity;
- fail-closed governed semantic admission;
- typed residual/producer frontier and sparse wake dependencies;
- admissible counterfactual world families;
- offline-first costed/Pareto proof-search scheduler;
- deterministic single-gap and whole-frontier receipts;
- multi-residual `ProofFrontier`;
- support/defeater/comparator/contradiction/treatment/terminology hypotheses;
- provider-neutral `QueryExpr` and local execution;
- proposition-level citation/reasoning/condition graph deltas;
- append-only source/research memory with revisable conclusions;
- multi-iteration offline knowledge compounding;
- separately governed provider crate; scheduler owns no HTTP;
- OALC exact-MNC streaming/index path;
- official HCA/FCA provider/fetch paths;
- typed provider-access/failover states;
- SHA256-bound local ingestion and persisted zero-network replay;
- successful bounded official HCA landing-page acquisition;
- acquisition -> immutable research-world handoff;
- zero-network HCA landing-page judgment-resource discovery;
- bounded full-HCA-DOCX acquisition runner/receipt path, source-written pending
  local validation and intentional execution.

## Core research-growth rule

Every newly parsed legal source should enrich the reusable legal/world graph even
when it does not close the current proof gap.

A parsed case may contribute source/revision identity, court/jurisdiction/time,
opinion segment, propositions, facts/conditions/circumstances, tests/rules,
reasoning roles, citation identities and treatments, pinpoints, outcome/remedy,
exceptions/defeaters/burdens and lexical realisations.

`retrieve -> ingest -> parse -> PNF -> citation/reasoning extraction -> reviewed typed graph deltas -> frontier assessment -> next search`

The graph learned today becomes zero-network context tomorrow.

Firewalls remain:

`citation != adoption != ratio != current authority`

`parsed proposition != truth`

`provider failure != proposition false`

`descriptive frequency != doctrine`

`statistical separator != legal cause`

`candidate WrongType != liability`.

## Rust implementation roadmap

### R0 - deterministic offline spine

**Status: implemented and validated on the stacked offline line.**

Deterministic candidate-only receipts, replayable scheduler/return loop,
source/document/digest/PNF welds, and zero-network fixtures are present.

### R1 - multi-residual proof frontier

**Status: materially implemented.**

Whole-frontier scheduling, shared-dependency value, contested/authority-blocked/
underidentified states, candidate satisfaction and budget/saturation boundaries are
present. Candidate closure is not legal proof closure.

### R2 - dialectical hypothesis families

**Status: materially implemented.**

Support, defeater/exception, comparator/analogy, contradiction/counterexample,
authority-treatment and terminology/identity hypotheses are distinct first-class
moves.

### R3 - provider-neutral query algebra

**Status: materially implemented.**

Terms, phrases, Boolean structure, proximity, citation, provision, court,
jurisdiction and date filters compile independently of provider execution. Query
hits never manufacture proof value.

### R4 - local corpus and immutable authority store

**Status: materially implemented at the store-neutral/runtime layer.**

Append-only source revisions, local execution/reuse, vocabulary/authority
neighbourhood accumulation and fail-closed rewrite handling are present. Durable
production persistence/migration remains R10.

### R5 - proposition-level citation/reasoning graph

**Status: materially implemented at candidate/review boundary.**

Source/opinion/proposition/citation/treatment, conditions/circumstances, reasoning
roles, outcomes/remedies, burden/exception and lexical coordinates are retained.
Graph enrichment does not itself establish ratio, binding force, applicability or
truth.

### R6 - governed Australian authority acquisition/failover

**Status: experimental live acquisition achieved.**

Implemented and exercised:

1. persisted/local first;
2. installed OALC exact-MNC lane;
3. official HCA/FCA providers;
4. optional sanctioned AustLII/JADE lanes;
5. `0.25 rps`, burst `1`, explicit depth/document/request budgets;
6. no crawl/ad-hoc polling;
7. search -> references;
8. fetch -> bytes;
9. local ingestion before parser/PNF;
10. SHA256-bound source revisions;
11. same-demand replay with zero network;
12. typed `ProviderAccessStatus` and failover;
13. successful official HCA landing fetch for `[2026] HCA 19`.

The old R6 acceptance gate is therefore achieved.

### R7 - iterative online/offline research engine

**Status: offline compounding implemented; online return path source-written and
partly calibrated. This is now the primary implementation frontier.**

Current path:

`official landing source -> local resource discovery -> preferred DOCX -> bounded document acquisition -> local source revision -> PNF -> citation/reasoning/treatment delta -> affected residual update -> next local-first iteration`

The landing page for Cullen exposes both official DOCX and PDF judgment files. The
new zero-network discovery fixture identifies both and deliberately prefers DOCX
for text/PNF processing. The next bounded live action should reuse the already
persisted landing page and spend exactly one request on that DOCX.

Next R7 tasks, in order:

1. validate the HCA resource-discovery/full-DOCX source tranche locally;
2. run one explicitly opted-in DOCX acquisition;
3. pin its SHA256/source revision and prove same-document replay is zero-network;
4. transform DOCX bytes into canonical local text with a parser receipt;
5. feed canonical text through existing PNF;
6. extract real Cullen citation/proposition/reasoning candidates with exact
   paragraph/source locators;
7. compare those source-grounded edges against the Mallonland/Cullen/Pabai/Woolcock
   lineage graph rather than fixture-only reasoning edges;
8. reschedule the surviving proof frontier using newly learned authorities,
   treatments, conditions and lexical terms.

Saturation remains frontier-relative and requires repeated stability across
independent search families.

### R8 - precedent geometry/discriminator search

After sufficient source-grounded structured cases exist:

- nearest comparable/opposite-outcome cases;
- minimal differing coordinate;
- candidate minimal separator and counterexample search;
- feed discriminators back into R2/R3;
- never treat statistical/geometric separation itself as doctrine or legal cause.

### R9 - corpus-derived lexical fibres/descriptive jurisprudence

- learn court/time/jurisdiction lexical realisations;
- use them for query expansion while preserving `query expansion != proof expansion`;
- descriptive treatment/outcome statistics may rank research but cannot create
  holdings, rules, truth or causation.

### R10 - durable governance/publication

- append-only durable receipt/world-model store;
- replay/migration/parity across Rust versions and Agda owners;
- promotion/publication boundary distinct from acquisition/parsing/reasoning;
- cryptographic identity/signature belongs at receipt/promotion boundaries.

## Acceptance milestones

### Offline Research Engine v0.1

**Achieved.**

Multi-residual frontier, opposing/comparator hypotheses, provider-neutral/local
queries, local execution, graph enrichment, sparse updates, Pareto rescheduling and
deterministic zero-network iteration receipts are demonstrated.

### Governed Official Acquisition v0.1

**Achieved experimentally.**

`[2026] HCA 19 -> official HCA landing -> 1 governed fetch -> SHA256/local source revision -> persisted replay=0`

### Full Judgment Materialization v0.1

**Current milestone.**

Acceptance:

1. reuse the persisted HCA landing artifact with `network_requests=0`;
2. discover official DOCX/PDF references locally;
3. select DOCX as the preferred full-text carrier;
4. perform exactly one governed document fetch;
5. SHA256-bind and locally ingest the DOCX;
6. replay the same document demand with zero network;
7. emit `sl.governed_official_judgment_acquisition.v0_1` candidate-only receipt;
8. produce canonical text/PNF receipt without semantic promotion;
9. begin source-grounded proposition/citation/reasoning extraction.

## Canonical formal counterparts

Agda remains the parity/authority layer over Rust runtime semantics. Current owners
cover proof-directed corpus search/query algebra, governed network policy, Pareto
scheduling, multi-residual iteration, reasoning/citation graph enrichment,
immutable research memory, official Australian provider order, observed HCA live
acquisition, acquisition-to-world handoff and official judgment resource discovery.

Historical SensibLaw remains reference-only for acquisition architecture. It does
not define current Rust or Agda semantics.

## Explicit non-goals

- no Python-to-Rust line-by-line port;
- no uncontrolled crawler;
- no provider failure -> negative fact conversion;
- no citation -> adoption/ratio/current-authority conversion;
- no retrieval/parser/statistics -> holding/truth conversion;
- no acquired bytes -> proposition correspondence shortcut;
- no causal dependence -> liability/remedy promotion.
