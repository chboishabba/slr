# SensibLaw -> slr feature-parity frontier

This document tracks behavioural/contract parity. It is not a mandate to port
Python/PostgreSQL implementation details into the Rust hot path.

## Governing performance rule

`slr` remains the direct-delta execution kernel. New legal workflow capabilities
must compile from already-produced semantic/status/residual carriers and must not
force parser reruns, document rescans, DB crossings, provider I/O or publication
on sentence-local execution.

## Already present in slr

- revision-scoped parser spans and packed sentence carriers;
- direct/reference parity and direct-delta execution;
- candidate-only semantic expansion with unresolved/alternative fibres;
- fail-closed governed semantic admission;
- typed residual frontier and producer work selection;
- relation-attachment candidate production;
- orthogonal semantic/legal status product, including proposition/truth,
  occurrence, jurisdiction, authority, applicability, violation and liability.

## Added in this tranche

### Admissible counterfactual world families

- arbitrary alternative != admissible counterfactual;
- event deletion alone != corrected conduct/institutional relation;
- held-fixed and comparison-alignment gates remain explicit;
- optional physical-compatibility gate for real-world claims that depend on
  physics/engineering/medicine/climate/etc.;
- multiple admissible corrected worlds may leave the outcome underidentified;
- one located world does not imply unique identification while enumeration is
  open;
- causal identification remains separate from scope, violation, liability,
  remedy and authority;
- consumer-relative closure and first-live-residual routing.

### Typed legal-source planning

Parity target: `SensibLaw/src/pnf/legal_adjunct.py` plus
`SensibLaw/src/sources/legal_follow.py` at the planning boundary only.

- ready persisted source;
- blocked missing context;
- blocked acquisition required;
- exact jurisdiction/source-role/authority/provider/time compatibility;
- missing compatible source is work, not negative legal evidence;
- source plans have acquisition-plan authority only;
- only source/authority residual producers may emit legal-follow demands;
- no HTTP/network acquisition capability in the Rust planning crate.

## Existing SensibLaw capability still to port as contracts

Priority is ordered to preserve the current performance constitution.

### P0 - typed/sparse, no I/O

1. Evidential PNF correspondence receipt:
   numeric/parser structure -> separately reviewed proposition correspondence,
   with parser structure != semantic/legal truth.
2. Consumer/query/policy indexed residual contract:
   closure for one consumer does not mutate unresolved global axes.
3. Sparse reverse-dependency reopening:
   new evidence wakes only consumers/questions that declared dependency on the
   affected coordinates.
4. Proof-graph/common-ground/controversy residual surface:
   exact typed meet may close source-comparison work; partial/no-meet/conflict
   remain distinct and compile to targeted follow work.
5. Authority/citation treatment graph carriers:
   source relationship/follow/discuss/distinguish/overrule etc. remain evidence
   carriers and do not independently create legal applicability.

### P1 - store-neutral execution interfaces

1. Append-only evidence/provenance revision interface.
2. Residual H3/H6/H9-equivalent horizon interface without encoding PostgreSQL
   schema into core semantics.
3. Consumer-local wakeup index keyed by stable semantic/source coordinates.
4. Persisted legal-source revision lookup interface consumed by the planner.

### P2 - external adapters outside semantic hot path

1. Legal-source acquisition adapter implementing a typed `LegalSourcePlan`.
2. World-evidence/provider acquisition adapter for explicitly consumer-observed
   residuals only.
3. Cache/freshness adapters preserving source revision and acquisition provenance.

### P3 - durable publication/governance

1. Append-only durable receipt store.
2. Governed promotion/publication boundary.
3. Replay/migration/parity receipts across the Python and Rust implementations.

## Explicit non-goals

- no Python-to-Rust line-by-line port;
- no PostgreSQL in sentence-local compilation;
- no parser token writes as production semantic state;
- no web crawler in legal counterfactual analysis;
- no regex/lexical legal inference;
- no `proper noun -> external provider` shortcut;
- no missing source/evidence -> negative fact conversion;
- no causal dependence -> liability/remedy/authority promotion;
- no physically imagined alternative -> realised-world promotion.

## Cross-domain formal parent

DASHI owns the reusable counterfactual contract. Legal and physical bridges are
separate interpretations of that generic discipline. TSFV/multiverse machinery
supplies a realisation/admissibility warning only: a parameter point or viable
history is not automatically a physically realised world. It does not supply
legal doctrine or authority.
