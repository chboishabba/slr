# Mabo governed context federation delta — 2026-09-17

This is a status delta over `mabo_100hop_world_20260916.md` on SLR #20.
The older scale ledger remains authoritative for the 100-hop traversal contract;
this file records the producer/review/materialisation frontier after the
revision-pinned Mabo Wikidata provider became GREEN.

## Current cut

```text
P7a deterministic 100-hop traversal                 PAID
P7b.0 reviewed-context persistence/provenance        PAID
P7b.1 Wikidata revision-pinned candidate producer    PAID @ bbb155a
P7b.2 Wikidata candidate -> explicit review gate     PAID @ 9538eec/cc5adfe (5/5 tests green)
P7b.3 reviewed Wikidata -> live PG materialisation   PAID LIVE RECEIPT (12 relations on TrueNAS PG)
P7b.4 Wikipedia revision-pinned producer/review      UNPAID
P7b.5 OALC exact-MNC producer/review                  UNPAID
P7b.6 enriched live walker breadth receipt            BLOCKED on 4-5
```

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
family name does not create legal authority.  The review boundary stores it only
as candidate context.

## Runtime recurrence

```text
Q1501525 @ oldid 2333409615
-> sl-wikimedia-candidate-provider
-> candidate SLRG rows
-> explicit candidate review
-> ReviewedContextEdge
-> algebra.relation + context.reviewed_relation_receipt
-> unchanged latent-world walker
-> Dioxus
-> optional wgpu
```

`review_mabo_wikidata_candidate(...)` now requires:

- the exact Mabo revision receipt;
- source QID `Q1501525`;
- exact candidate-id/source/property/target consistency;
- one of the five bounded property IDs above;
- `ContextReviewDecision::Reviewed`.

Unreviewed candidates fail closed.  The storage layer does not decode/fetch SLRG
or infer review; acquisition and review remain upstream concerns.

## Non-promotion boundary

```text
candidate property edge != reviewed context edge
reviewed context edge != legal_ir proposition support
AuthoritySource candidate != legal authority
persisted context relation != applicability
persisted context relation != claim truth
```

Therefore this P7b lane remains distinct from the Mabo `legal_ir` proposition
support/materialisation lane used by Reader `Why?` payment.

## Observed live receipt (P7b.3)

Observed on TrueNAS PostgreSQL (`truenas.local:5432/sensiblaw_sparse_ready_20260818`):

```text
wikidata revision: wikidata:Q1501525:oldid:2333409615
wikidata candidates observed: 14 direct property candidates
wikidata candidates reviewed: 12 (all bounded properties P1001, P710, P4884, P1594, P4006)
wikidata candidates unreviewed/rejected: 2 (P31, P17 fail-closed)
wikidata edges persisted into algebra.relation: 12
receipt rows persisted into context.reviewed_relation_receipt: 12
source family per edge: wikidata
source revision per edge: wikidata:Q1501525:oldid:2333409615
candidate_only: true
creates_semantic_authority: false
applicability_promoted: false
claim_truth_promoted: false

Walker (seed = Q1501525):
  requested hops: 100
  deepest observed hop: 1
  visited nodes: 13 (Q1501525 + 12 targets: Q408, Q1358798, Q4773043, Q3778295,
                     Q267745, Q5226153, Q6261017, Q15527343, Q6832720, Q6851910,
                     Q975866, Q36074)
  edges: 12
  wikidata-provenanced edges: 12
  legal_ir/proof edges: 0
  context-only edges: 12
  frontier exhausted: true
  residuals: []

Walker (seed = mabo:proposition:radical-title-native-title):
  requested hops: 100
  deepest observed hop: 3
  visited nodes: 11
  edges: 11
  wikidata-provenanced edges: 0
  legal_ir/proof edges: 11
  context-only edges: 0
  frontier exhausted: true
  residuals: []

Persisted frontier broader from mabo proposition: false
Explanation: Q1501525 context edges are currently persisted as a reviewed Wikidata cluster;
no cross-source anchor edge between the legal_ir Mabo proposition and Q1501525 has been
materialized yet (scheduled for Wikipedia/OALC federation in P7b.4-P7b.6).
```

## Broader roadmap consequence

The storage/traversal architecture is no longer the P7b bottleneck.  The remaining
critical path is producer breadth plus explicit review/materialisation:

```text
Wikidata live materialisation
-> Wikipedia revision/hash acquisition + review/materialisation
-> OALC exact-MNC acquisition + review/materialisation
-> enriched world receipt
-> downstream Dioxus/world consumer validation
-> optional wgpu visual exploration
-> later publication/federation
```

No new crawler, persistence ontology, legal-IR promotion path, Dioxus semantics,
or walker network I/O is justified by this frontier.
