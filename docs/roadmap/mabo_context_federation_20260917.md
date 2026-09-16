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
P7b.2 Wikidata candidate -> explicit review gate     SOURCE-WRITTEN @ cc5adfe
P7b.3 reviewed Wikidata -> live PG materialisation   UNPAID LIVE RECEIPT
P7b.4 Wikipedia revision-pinned producer/review      UNPAID
P7b.5 OALC exact-MNC producer/review                  UNPAID
P7b.6 enriched live walker breadth receipt            BLOCKED on 3-5
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

## Next live receipt

The next operator payment is not another provider implementation.  It is:

1. select the desired candidate IDs from the revision-pinned Q1501525 SLRG;
2. explicitly review those exact candidate coordinates;
3. construct `ReviewedContextEdge` rows through the review gate;
4. call `materialize_reviewed_context_edges(...)` against the live PG;
5. rerun the unchanged 100-hop walker;
6. report source-family coverage and graph size.

Required receipt fields:

```text
wikidata revision
wikidata candidates observed
wikidata candidates reviewed
wikidata edges persisted
source family / source revision per edge
candidate_only = true
creates_semantic_authority = false
applicability_promoted = false
claim_truth_promoted = false

walker requested hops
walker deepest observed hop
walker nodes
walker edges
wikidata-provenanced edges
legal_ir/proof edges
context-only edges
frontier exhausted / residuals
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
