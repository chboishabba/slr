# SLR roadmap: semantic world vs local materialisation vs replication

Date: 2026-09-15

## Why this exists

The current SLR/SensibLaw architecture can now execute a consumer-relative research recurrence and can place parts of that work across local/federated capabilities. That does **not** imply that every deployment should discover, parse, mirror, or retain every source locally.

The roadmap therefore distinguishes three separate questions:

1. **Semantic/world capability** — what the system can represent, relate, query, and prove/refute relative to a consumer.
2. **Materialisation policy** — which parts of that world should exist as local bytes/rows/artifacts right now.
3. **Replication/retention policy** — which materialised objects should be retained, mirrored, encrypted, content-addressed, or allowed to expire.

These must not be collapsed.

```text
semantic representability
!= local possession
!= retention
!= replication
!= authority
```

## Already-paid architectural base

The validated inner recurrence is:

```text
read
-> residual
-> select
-> acquire
-> read again
-> pay
-> stop
```

The validated outer acquisition/federation layer is:

```text
consumer residual
-> acquisition policy
-> eligible local/federated capability
-> acquisition depth
-> existing research recurrence
```

The federation layer may expose storage, parse/compute, routing/discovery, ontology-candidate, or legal-authority-provider capabilities without gaining semantic authority by doing so.

## New roadmap axis: storage/materialisation profiles

A deployment should choose an explicit `MaterialisationProfile` rather than inherit one hidden global corpus assumption.

Conceptually:

```text
MaterialisationProfile =
  consumer
  x corpus class
  x confidentiality
  x freshness
  x latency tolerance
  x local storage budget
  x network availability
  x compute budget
  x replication policy
  x retention policy
  x authority policy
```

Useful modes include:

### Full local mirror

Use when offline operation, institutional continuity, low-latency repeated access, or source custody justifies local bytes.

Examples:
- a CLC retaining an authorised matter bundle;
- a public-law institution mirroring legislation/case sources;
- a research node volunteering storage for public objects.

### Selective cache

Keep frequently used, expensive-to-reacquire, or policy-important objects locally; keep the rest as retrievable identities/locators.

### Skeletal/minimal corpus

Keep only enough local structure to support navigation, residual computation, provenance, and exact re-acquisition.

A skeletal legal corpus may retain, for example:

```text
legislation segment identity
+ hierarchy/parentage
+ jurisdiction
+ temporal/version coordinates
+ named entities/QIDs
+ PNF/claim anchors
+ exact source locator/content digest
+ authority/source role
+ evidence/payment history
```

without retaining the full source text.

This means a node can know that a consumer residual points to a particular legislation segment or case proposition while fetching the actual text only when required.

The same principle applies outside law:

- Obsidian: note/node graph + refs while note bodies remain in the vault;
- medicine: encounter/referral/lab graph while protected record bytes stay in the clinical system;
- science: DOI/QID/OEIS/ontology graph + source-role metadata while PDFs remain on-demand.

### Reference-only/federated

Keep content identity, locator set, capability route, and history, but no durable local bytes except transient processing buffers allowed by policy.

This should be a first-class supported mode, not a degraded fallback.

## Corpus compression is not one problem

Treat these separately:

1. **semantic compression** — retaining graph/PNF/provenance coordinates instead of every repeated textual manifestation;
2. **binary/storage compression** — compressing retained byte payloads;
3. **deduplication** — content-addressed reuse of identical objects;
4. **projection** — keeping only consumer-relevant coordinates locally;
5. **cold storage** — retaining bytes but moving them out of the hot working set;
6. **non-possession** — retaining only identity/locator metadata and reacquiring later.

Do not report these as interchangeable "compression ratios".

A source can be semantically projected without its bytes being compressed; two byte-identical sources can deduplicate without being semantically equivalent; a skeletal graph can remain useful without proving that omitted text is unnecessary for every consumer.

## Retention policy

Retention must be policy and consumer indexed.

Suggested object states:

```text
HOT_LOCAL
COLD_LOCAL
CONTENT_ADDRESSED_REPLICA
ENCRYPTED_REMOTE_REPLICA
REFERENCE_ONLY
TRANSIENT_ONLY
PURGED_BYTES_RETAINING_RECEIPT
```

Retention decisions must preserve append-only observation/provenance history even when source bytes are no longer retained locally.

Required firewall:

```text
bytes purged
!= observation never existed
!= provenance erased
!= prior payment invalidated automatically
```

If future verification requires the original source and it is no longer retrievable, that becomes a new availability/verification residual rather than silently rewriting history.

## Replication policy

Replication is independent of semantic authority.

```text
replica count
!= truth
!= source authority
!= evidence payment
```

Public content may be replicated into IPFS or equivalent content-addressed stores. Private content may require encrypted storage, restricted peers, local-only retention, or no replication at all.

Potential future secure-storage variants include encrypted object stores, institution-controlled cloud, IPFS/private pin sets, and ZKP-backed disclosure/proof protocols. These are storage/disclosure mechanisms, not automatic semantic-payment mechanisms.

## Capability-specialised federation

Nodes need not be mirrors of one another.

Examples:

```text
storage-rich node -> public mirror/content-addressed retention
compute-rich node -> parse/PNF/world deltas
network-rich node -> candidate discovery/routing
ontology node -> Wikidata/DBpedia/medical/science/OEIS candidates
authority node -> primary-source/legal-source cache
```

The scheduler should choose an admitted producer for the live consumer residual, not attempt universal replication.

## Practical deployment profiles

### Personal / Obsidian

Default should favour local source-of-truth ownership by the user's existing vault.

Possible profiles:

```text
minimal:
  vault remains canonical
  SLR stores refs, graph, tags, provenance, residuals
  external identity enrichment on demand

backup-enhanced:
  above + encrypted replicated object store / DB backup

research-heavy:
  above + selected public-source cache and ontology mirrors
```

Auto-tagging/linking is a consumer output, not a reason to duplicate every note body into a second mandatory corpus.

### Legal / CLC / self-represented litigant

Priorities:
- exact source identity;
- temporal/jurisdiction/authority coordinates;
- matter-scoped confidentiality;
- material fact / proposition / evidence / conflict / residual graph;
- reproducible access to the source span used for advice or filing.

Full local matter storage may be justified, while general public authority material can often be skeletal + on-demand or institutionally mirrored.

### Medical / clinical

Priorities:
- protected patient bytes remain under the clinical confidentiality policy;
- public ontology/guideline discovery can be remote;
- source role must distinguish patient report, clinician observation, measurement, inference, and external guideline;
- replication should be conservative by default.

## Admissibility and FactorsThrough implications

Local possession is not an adequate observer for consumer adequacy.

```text
ConsumerAdequacy
not FactorsThrough LocalPossession
```

The refinement must expose at least:

```text
LocalWorld
x FederatedReachability
x AuthorityPolicy
x Privacy/DisclosurePolicy
x RetrievalState
```

Likewise, a skeletal corpus may be adequate for one consumer and inadequate for another:

```text
Q_navigation may FactorsThrough skeleton
Q_exactQuotation may not FactorsThrough skeleton
Q_primaryAuthorityReview may require full verified source span
```

This should reuse the existing query-indexed projection/non-factorability machinery, not create a separate storage adequacy theory.

## High-alpha implementation order

### Complete now

1. **Roadmap/policy contracts** for materialisation, retention, replication, and skeletal corpus.
2. **Typed skeleton identity model** for source/segment/node identities, locators, content digest, authority/source role, temporal coordinates, and availability state.
3. **Query-indexed adequacy proofs** showing which consumers factor through skeletons and which require full source bytes/spans.
4. **Retention receipts** so transitions such as hot -> reference-only are append-only and auditable.
5. **Federated capability discovery/job transport**, reusing the already-paid distributed job/ZOS/eRDFa infrastructure rather than inventing another semantic layer.

### Defer until demanded by measurements/use-cases

- aggressive bespoke binary compression algorithms;
- universal local corpus mirroring;
- blockchain/on-chain storage as a default;
- complex ZKP storage/disclosure protocols before a concrete confidentiality consumer requires them;
- global replication optimisation independent of real workload receipts;
- large Nat-scale mirroring before policy/retention behaviour has been measured on smaller cohorts.

### Measure before optimising

Record:
- bytes hot/cold/reference-only;
- deduplication ratio;
- reacquisition latency;
- parse cost per source unit;
- cache hit/re-fetch rate;
- residual contraction per acquired byte/request;
- replication benefit vs storage/network cost;
- source unavailability/failure rate;
- consumer-specific skeleton sufficiency/failure witnesses.

Then optimise the actual Pareto frontier rather than assuming storage is the bottleneck.

## Roadmap implication

The long-term architecture is not "everyone has a complete corpus".

It is:

```text
local private/materially useful state
+ append-only evidence/provenance history
+ consumer-adequate skeletal graph
+ policy-admitted caches/replicas
+ federated reacquisition capability
```

The target property is therefore:

```text
self-populating != self-hoarding
```

and also:

```text
world representability != world materialisation
```

The next generic seam after the already-paid federation tranche is cross-machine capability discovery and admitted typed-job transport. The next storage-specific seam is a typed skeletal-corpus/materialisation policy with query-indexed adequacy witnesses. Both should precede broad corpus compression work.