# Mabo P7d world-identity / observation hardening — 2026-09-17

This tranche follows P7d.2–P7d.4 on SLR PR #23 and pays the final source-level
correctness seams identified before the recurrent >=100-object run.

## 1. Novelty means reviewed world identity classes

P7d does not count representation strings.  A reviewed identity resolution may
bind representations such as:

```text
Q975866
https://en.wikipedia.org/wiki/Eddie_Mabo
provider-specific identity
```

to one:

```text
world-object:eddie-mabo
```

`WorldExpansionLedger` therefore counts `identity_class_ref` values in the
identity-aware admission path.  The older representation-equality admission API
remains only as a compatibility path for already-paid callers.

```text
100 distinct strings != 100 novel world objects
```

## 2. Revisioned producer hardening

Wikidata candidate projection now requires an acquired entity receipt carrying:

```text
qid
source_revision_ref
content_digest_ref
candidate_only
semantic_promotion
```

and checks:

```text
route_family == WikidataProperty
producer == IdentitySource
acquired.qid == route.source_ref
candidate_only == true
semantic_promotion == false
```

A free-standing revision string is no longer sufficient to weld a property route
to a revisioned Wikidata entity.

The OALC adapter likewise requires:

```text
receipt_authority == experimental_candidate_only
```

before emitting an expansion candidate.

## 3. Backend-independent WorldObservation

`world_observation` defines one normalized production ABI:

```text
request
object
relation
source
source revision
content digest
observed value
retrieval state
freshness state
provenance class
```

The backend coordinate is deliberately outside normalized observation semantics.
Current backend tags are:

```text
SlrNative
LeanInterop
Other
```

`LeanInterop` is only one worked interop backend.  P7 does not depend on JMD/Lean
for discovery or truth.

A disagreement produces `GetterParityResidual`; it does not select world truth.
`getter_parity_to_proof_residual(...)` maps such a disagreement onto the existing
open `ProofResidual` surface with producer class `producer:getter-parity`, so it
enters the ordinary frontier/Ibrahim recurrence.

## 4. Real fixtures

The Mabo interop fixture is:

```text
Q1501525
P710
Q975866
wikidata:Q1501525:oldid:2333409615
```

The same ABI is exercised by the existing Nat Climate test family:

```text
Q10884
migration:P5991->P14143
provided_snapshot_2026-04-01
review-required
```

The Nat Climate fixture remains review-only; observation does not imply safe
migration, source authority or semantic equivalence.

## 5. Identity-aware atomic cycle and lineage

`world_expansion_identity_session` provides the P7d.5 path:

```text
open residual
-> select candidate
-> reviewed identity resolution
-> stage identity-class admission
-> post-acquisition PNF/world re-entry
-> commit ledger/frontier/lineage together
```

`world_identity_lineage` retains the representation and reviewed world identity
class separately.

The provider-neutral PostgreSQL `context.discovery_lineage_receipt` schema is
extended additively with `identity_class_ref`.  Receipt hashing is bumped to
`mabo-discovery-lineage:v2` so identity-class changes necessarily change the
receipt digest.

## 6. Status

```text
P7d.0 reviewed residual admission                   PAID
P7d.1 producer adapters                             PAID
P7d.2 observed re-entry                             PAID
P7d.3 frontier recurrence                           PAID
P7d.4 durable lineage / atomic session              PAID
P7d.5a identity-class novelty semantics             SOURCE-WRITTEN
P7d.5b backend-independent WorldObservation         SOURCE-WRITTEN
P7d.5c revision-welded Wikidata/OALC adapters       SOURCE-WRITTEN
P7d.5d parity residual -> ProofResidual bridge      SOURCE-WRITTEN
P7d.5e live recurrent >=100 identity-class run      ACTUAL-RUN TARGET
```

No new source-written item above is an execution-GREEN claim.  The next receipt
must come from Cargo/clippy/workspace execution and the additive live PostgreSQL
migration/materialisation, followed by real Mabo/Nat Climate observation runs.
