# Mabo governed context federation delta — 2026-09-17

This is a status delta over `mabo_100hop_world_20260916.md` on SLR #20.
The former "100-hop" contract is now explicitly demoted to a traversal-capacity
primitive. The product target is residual-driven admission of 100 novel reviewed
knowledge objects from the Mabo seed surface.

## Corrected P7 decomposition

```text
P7a Persisted World Traversal Engine                  PAID
P7b Governed Source Federation                        ACTIVE / PARTIALLY PAID
P7b.0 reviewed-context persistence/provenance         PAID
P7b.1 Wikidata revision-pinned candidate producer     PAID @ bbb155a
P7b.2 Wikidata candidate -> explicit review gate      PAID @ 9538eec/cc5adfe (5/5 tests green)
P7b.3 reviewed Wikidata -> live PG materialisation    PAID LIVE RECEIPT (12 relations on TrueNAS PG)
P7b.4 Wikipedia revision/hash producer/review         UNPAID
P7b.5 OALC exact legal producer/review                 UNPAID
P7c Residual-Driven World Expansion Controller        PAID @ d3f15bf (65/65 tests green, 0 clippy warnings)
P7d Mabo 100-Novel-Object Discovery Receipt           UNPAID LIVE RECEIPT
```

`100` in P7d means:

```text
starting from Mabo, admit >= 100 unique reviewed world objects
```

not:

```text
walk to depth 100
```

Traversal depth remains an observed topology metric only.

## Existing Wikidata federation receipt

The Wikidata provider GREEN receipt supplied by the operator is:

```text
Q1501525 @ revision 2333409615
direct property candidates: 14
Wikipedia article candidates: 1
search candidates: 5
parser-repair candidates: 1
JSON transport: false
regex parser: false
semantic promotion: false
```

The bounded direct-property review surface is:

```text
P1001 -> context:wikidata:jurisdiction
P710  -> context:wikidata:participant
P4884 -> context:wikidata:court
P1594 -> context:wikidata:judge
P4006 -> context:wikidata:overrules
```

`P4006` may have upstream `AuthoritySource` candidate routing, but that producer
family name does not create legal authority. The review boundary stores it only
as candidate context.

## Residual-driven recurrence

The semantic controller is now:

```text
PNF / consumer requirements
-> current world
-> open residual frontier
-> quotient against already-known/admitted objects
-> residual routing context
-> producer candidate moves
-> residual-sensitive Pareto selection
-> governed acquisition
-> parse / PNF
-> world / same-object disambiguation
-> explicit review
-> persist admitted object + discovery lineage
-> recompute frontier
-> repeat until >= 100 novel admitted objects
```

Raw graph adjacency, Wikidata neighbours, Wikipedia first-link, citation links,
and route order are candidate producers/navigation priors only. They do not
control semantics.

## Producer routing policy

Producer priority is consumer/residual-relative rather than globally ordered.

```text
legal / doctrinal / authority / source-text residual
    -> governed legal lane gets first refusal
       (official/OALC/provider machinery)

identity / same-object / entity residual
    -> Wikidata identity lane gets first refusal

context / explanatory / article residual
    -> Wikipedia/article-semantic lane gets first refusal

provenance / source-history residual
    -> source-specific provenance lane gets first refusal
```

Expected residual contraction is the primary selection coordinate. Domain fit is
only a residual-indexed tie-break/Pareto coordinate. Therefore a non-legal lane
may beat the legal lane when it is expected to contract the current residual
more strongly.

Legal-first means:

```text
follow legal material while it contracts an open legal residual or exposes a
higher-value legal residual after quotienting against the current world
```

not:

```text
remain inside OALC/legal citation traversal until exhausted
```

## P7c controller contract

`sl-proof-search-loop::world_expansion` owns only:

- residual classes;
- producer-lane routing metadata;
- contraction-first deterministic candidate selection;
- explicit disambiguation outcomes;
- review/admission accounting;
- canonical-object de-duplication;
- the Mabo `target_novel_objects = 100` completion criterion.

It does not fetch providers, parse PNF, infer legal authority, persist rows, or
pay proof/applicability/truth obligations.

Admission-capable disambiguation outcomes are:

```text
SameObject
NewRelatedObject
NewSourceManifestation
NewConceptualParent
NewEvidentiarySource
```

Fail-closed outcomes are:

```text
Ambiguous
WrongType
Duplicate
IrrelevantToResidual
```

Each admitted object must retain discovery parent, triggering residual, producer
lane, source revision when applicable, and disambiguation/review lineage.

## Existing runtime recurrence for reviewed Wikidata context

```text
Q1501525 @ oldid 2333409615
-> sl-wikimedia-candidate-provider
-> candidate SLRG rows
-> explicit candidate review
-> ReviewedContextEdge
-> algebra.relation + context.reviewed_relation_receipt
-> unchanged latent-world walker
```

`review_mabo_wikidata_candidate(...)` requires:

- the exact Mabo revision receipt;
- source QID `Q1501525`;
- exact candidate-id/source/property/target consistency;
- one of the five bounded property IDs above;
- `ContextReviewDecision::Reviewed`.

Unreviewed candidates fail closed. The storage layer does not decode/fetch SLRG
or infer review; acquisition and review remain upstream concerns.

## Non-promotion boundary

```text
100 objects != 100 hops
100 objects != 100 paid propositions
100 objects != 100 authorities
producer preference != source authority
candidate reachability != reviewed admission
candidate property edge != reviewed context edge
reviewed context edge != legal_ir proposition support
AuthoritySource candidate != legal authority
persisted context relation != applicability
persisted context relation != claim truth
```

Therefore P7 remains distinct from the Mabo `legal_ir` proposition-support lane
used by Reader `Why?` payment.

## Observed live Wikidata materialisation receipt

Observed on TrueNAS PostgreSQL (`truenas.local:5432/sensiblaw_sparse_ready_20260818`):

```text
wikidata revision: wikidata:Q1501525:oldid:2333409615
wikidata candidates observed: 14 direct property candidates
wikidata candidates reviewed: 12
wikidata candidates unreviewed/rejected: 2 (P31, P17 fail-closed)
wikidata edges persisted into algebra.relation: 12
receipt rows persisted into context.reviewed_relation_receipt: 12
candidate_only: true
creates_semantic_authority: false
applicability_promoted: false
claim_truth_promoted: false

Walker (seed = Q1501525):
  hop budget: 100
  deepest observed hop: 1
  visited nodes: 13
  edges: 12
  context-only edges: 12
  frontier exhausted: true

Walker (seed = mabo:proposition:radical-title-native-title):
  hop budget: 100
  deepest observed hop: 3
  visited nodes: 11
  edges: 11
  legal_ir/proof edges: 11
  frontier exhausted: true

Persisted frontier broader from mabo proposition: false
```

These receipts prove traversal/materialisation behavior but do not pay P7d.

## P7d acceptance receipt

A successful Mabo discovery run must report at least:

```text
target_novel_objects = 100
total_new_world_objects >= 100
candidates_seen
candidates_rejected
duplicates_seen
identity_ambiguous
reviewed_objects
new_qids_admitted
new_articles_admitted
new_primary_legal_sources_admitted
new_other_world_objects_admitted
```

and retain per-object:

```text
object identity + kind
discovery parent
triggering residual
PNF/evidence obligation class
producer lane
source manifestation/revision when applicable
routing reason / selected move
disambiguation outcome
review/admission status
post-admission residual delta when observed
```

No duplicate object identity counts twice.

## Next highest-alpha tranche

The current next step is no longer "make the walker deeper." It is to bind the
controller to live residual production and governed acquisition:

```text
Mabo PNF/world residual
-> frontier selection
-> if legal residual: LegalFollow/OALC/official source first-refusal
-> otherwise identity/context/provenance producer as appropriate
-> acquire + parse + PNF
-> disambiguate against world
-> review + persist
-> recompute residuals
-> continue until 100 novel objects
```

The existing Wikipedia/OALC federation obligations become producer lanes inside
this loop rather than sequential milestones that must always run in a fixed
order.

No new crawler, persistence ontology, legal-IR promotion path, Dioxus semantics,
or walker network I/O is justified by this frontier.
