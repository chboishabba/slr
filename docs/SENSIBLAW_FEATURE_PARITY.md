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
- one located world does not imply unique identification while enumeration is open;
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

### Evidential PNF handoff

Parity target: mature SensibLaw evidential bridge semantics.

- parser/numeric run, document, hash and graph identities are retained;
- world resolution must remain deferred;
- document-local numeric PNF may not claim cross-document identity closure;
- parser observation is not semantic authority;
- reviewed semantic correspondence is separately required;
- reviewed correspondence still does not automatically claim world truth or a
  legal holding.

### Consumer/query/policy residual contracts + sparse reopening

Parity target: mature SensibLaw reverse-dependency/wakeup semantics, including
migration 139, without making PostgreSQL the semantic owner.

- stable source/evidence/proposition/authority/counterfactual coordinates;
- exact `(demand, consumer, query, policy)` fibre keys;
- each consumer declares required coordinates and minimum horizon;
- reverse dependencies compile from that declaration;
- new evidence wakes only matching dependencies;
- missing dependency means zero affected work, not negative evidence;
- no document/corpus scan and no semantic inference occurs in wake selection.

## Existing SensibLaw capability still to port as contracts

Priority is ordered to preserve the current performance constitution.

### P0 - typed/sparse, no I/O

1. Proof-graph/common-ground/controversy residual surface:
   exact typed meet may close source-comparison work; partial/no-meet/conflict
   remain distinct and compile to targeted follow work.
2. Authority/citation treatment graph carriers:
   source relationship/follow/discuss/distinguish/overrule etc. remain evidence
   carriers and do not independently create legal applicability.
3. Consumer-local non-monotone conclusion recomputation:
   append evidence/provenance while allowing exceptions/defeaters to revise the
   currently reachable conclusion without erasing prior receipts.

### P1 - store-neutral execution interfaces

1. Append-only evidence/provenance revision interface.
2. Residual H3/H6/H9-equivalent horizon execution interface without encoding
   PostgreSQL schema into core semantics.
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

## Canonical formal counterparts

The repo-state audit identifies the current DASHI owners rather than creating a
parallel counterfactual ontology:

- #791: legal factual/but-for causation;
- #794: generic corrected-world/realised-repair + Country/Two-Eyed/POSIWID;
- #808: physical compatibility/realisation and experimental discrimination,
  stacked on #794.

Closed #805 was a stale-snapshot duplicate and is explicitly superseded.

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

## Cross-domain formal boundary

Legal and physical counterfactual bridges reuse a common discipline while
remaining semantically and authoritatively distinct. TSFV/multiverse machinery
supplies a realisation/admissibility warning only: a parameter point or viable
history is not automatically a physically realised world. It does not supply
legal doctrine or authority.
