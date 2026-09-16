# Semantic Reader Rust production ledger — 2026-09-16

This is the current successor ledger for the Rust production tranche requested
against `docs/roadmap/mabo_proof_explanation_profile_20260915.md`.
The parent roadmap remains the detailed provenance sheet for P0–P4/P3. This file
tracks the cross-cutting runtime implementation added on top of SLR #17.

Updated: 2026-09-16 — live TrueNAS Mabo proposition weld paid; Dioxus P5 is the
active production frontier.

## Ownership (unchanged)

```text
DASHI / Agda       golden semantic/proof contract
SLR / Rust         production parse/residual/payment/recurrence/read ABI
PostgreSQL         sole active semantic persistence spine
Dioxus / Rust      production reader + ordinary interaction surface
wgpu / WGSL        accelerated graph/chart/proof/world rendering
SensibLaw Python   reference + PG regression
ITIR / Svelte      reference/regression prototype only
```

Production path:

```text
governed source providers / durable AU legal corpus
-> source bytes + provider/source-role provenance
-> canonical PostgreSQL source/span
-> CandidatePNF
-> explicit review/admission
-> ReviewedPNFRevision
-> legal_ir materialisation
-> PropositionChainPayment
-> PropositionPayment / Reader ABI
-> Dioxus
-> optional wgpu projection
```

No JSON/TSV/Svelte/Python object is semantic identity in this path. A Hugging
Face/downloaded AU legal corpus snapshot is likewise a governed acquisition or
offline-replay provider, not semantic persistence and not legal authority by
storage location alone.

## Latest PR / branch ledger

```text
SLR #13  governed Australian acquisition / anchored citations   DONOR ACQUISITION LANE
SLR #14  bounded proposition calculus                            PAID
SLR #15  portable Reader ABI                                    PAID
SLR #17  live PG proposition weld / legal_ir materialiser       PAID LIVE ON TRUENAS
SLR #18  Rust Semantic Reader runtime successor                 FOCUSED GREEN; LIVE WELD PAID
DASHI #963 bounded proposition-chain parity                     PAID
DASHI #982 legal_ir materialisation parity                      FORMAL PARENT
Dioxus upstream #18 native Rust/backend mediation               MERGED P5 PARENT
Dioxus agent/mabo-semantic-reader-v1                             SOURCE-WRITTEN / P5 ACTIVE
wgpu ProofCone                                                   NOT STARTED / correctly downstream
```

SLR #18 branch:

```text
agent/semantic-reader-runtime-v1
```

The live database receipt is now part of the current state, not a pending
operator obligation.

## Flagship coordinate

```text
proposition:
  mabo:proposition:radical-title-native-title

exact span:
  span:mabo:brennan:radical-title:no-automatic-beneficial-ownership

source revision:
  source-revision:mabo:1992:hca:23:wikisource:page-39:rev-16058297:2026-06-29

document:
  document:mabo:1992:hca:23:brennan:wikisource-page-39
```

The live TrueNAS path has now crossed:

```text
existing Brennan span
-> ReviewedPnfRevision
-> ReviewedPropositionSupport
-> legal_ir.semantic_build
-> legal_ir.projection
-> legal_ir.observation
-> legal_ir.graph_revision
-> PropositionChainPayment
-> PropositionPayment / Reader ABI
```

Observed live result:

```text
exactSourcePaid       = true
propositionChainPaid  = true
whyExecutable         = true
OpenSource            = ExecuteSource
Why                   = ExecuteBoundedWhy
ApplicabilityPaid     = false
ClaimTruthPaid        = false
```

This pays the end-to-end P3 storage/payment/runtime weld without promoting the
bounded explanation into applicability or claim truth.

## Proposition-payment contract

Support cannot be residualised. It requires:

```text
exact source paid
+ retained reviewed PNF observation
+ retained PNF revision identity
+ observation provenance containing exact span
+ independent graph revision containing the same exact span
```

Qualifier, defeater, and comparator may each be either a paid observation or an
explicit retained residual for this bounded reader query.

```text
WhyPaid =
  ExactSourcePaid
  && SupportPaid
  && Covered(Qualifier)
  && Covered(Defeater)
  && Covered(Comparator)
```

with:

```text
SupportPaid != ExplicitResidual(Support)
BoundedWhyPaid != ApplicabilityPaid
BoundedWhyPaid != ClaimTruthPaid
ExplicitResidual != PropositionFalse
```

## P3 — live PG proposition weld — PAID

The completed recurrence is:

```text
P3a0 ExactSourceSpan -> parser-neutral CandidatePNF
P3a1 CandidatePNF + explicit review/admission -> ReviewedPNFRevision
P3a2 ReviewedPNFRevision -> legal_ir persisted support
P3b  live legal_ir rows -> PropositionChainPayment
P3c  PropositionChainPayment -> PropositionPayment / Reader ABI
```

### Reviewed PNF materialisation

Rust owners:

```text
crates/sl-pg-source-store/src/candidate_pnf.rs
crates/sl-pg-source-store/src/reviewed_pnf.rs
crates/sl-pg-source-store/examples/materialize_mabo_reviewed_pnf.rs
crates/sl-pg-source-store/src/legal_ir_materialization.rs
crates/sl-pg-source-store/examples/materialize_mabo_radical_title_support.rs
```

The materialisation remains explicit and reviewed. There is no automatic
`CandidatePnfBatch -> ReviewedPnfRevision` promotion.

Live operator sequence used for the flagship coordinate:

```text
existing Brennan exact span
-> materialize_mabo_reviewed_pnf
-> materialize_mabo_radical_title_support
-> live proposition-weld test
```

The persisted semantic state remains candidate/reviewed support only; no
applicability, holding, or claim-truth state is created by persistence.

### Live PG -> SLR -> Reader ABI

Runtime owners include:

```text
load_proposition_rows(...)
load_mabo_proposition_rows(...)
project_reader_payment(...)
```

The read projection consumes storage-owned exact-span, PNF identity/provenance,
and independent graph-span coordinates. It does not reinterpret PNF
`role_bindings` as reader proof roles.

Qualifier/defeater/comparator remain consumer-relative SLR residual debts.

Observed live TrueNAS receipt:

```text
cargo test -p sensiblaw-pg-source-store \
  --test mabo_proposition_weld \
  -- --ignored --nocapture

1 passed; 0 failed
```

The test verifies the complete live path through both Reader dispositions while
retaining:

```text
ApplicabilityPaid = false
ClaimTruthPaid     = false
```

## Adaptive Explanation Cone — GREEN

Owner:

```text
crates/sl-reader-model/src/semantic_runtime.rs
```

Rule:

```text
Keep(x) = mandatory(x)
       || distance(x) <= base_depth
       || elucidatory_score(x) >= threshold
```

The result is deterministic and bounded. Mandatory payment/provenance nodes are
ordered before optional explanatory nodes. The runtime is proposition-generic and
contains no Mabo-specific selection logic.

## Automatic acquisition/retry — GREEN

Owner:

```text
crates/sl-proof-search-loop/src/reader_retry.rs
```

```text
reader Defer(rho)
-> ProofResidual / ProofFrontier
-> existing hypothesis/provider/acquisition/world machinery
-> local AU corpus / governed provider lookup where admissible
-> persistence/world extension
-> mandatory reader re-evaluation
-> Execute | Defer | Reject
```

Hard firewall:

```text
acquisition receipt != semantic payment
```

## Context rabbit holes — SOURCE-WRITTEN

Portable Reader ABI types distinguish:

```text
ExactSource       PrimaryAuthority
Wikipedia         BackgroundContext
Wikidata          IdentityOnly
Historical        HistoricalContext
SemanticFocus     SemanticProjection
```

All context links report:

```text
createsEvidencePayment = false
createsApplicability    = false
```

The known Mabo identity/context coordinates remain `Q1501525` and
`wiki:en:Mabo_v_Queensland_(No_2)`; they do not replace the exact High Court
source coordinate.

## Latent Mabo world — LIVE GREEN READ PROJECTION

Portable projection:

```text
bounded_neighbourhood(seed, nodes, edges, max_hops, max_nodes)
```

PG read owner:

```text
load_latent_world_rows(...)
```

The PostgreSQL query traverses existing coordinates only:

```text
algebra.relation
execution.dependency
legal_ir.graph_revision
legal_ir.semantic_build
legal_ir.projection
legal_ir.observation
source spans / observation provenance / PNF refs / residual refs
```

No new graph/world persistence schema was introduced. A 100-hop reader view is a
bounded read projection of existing semantic state and cannot create semantic
authority.

Observed live TrueNAS receipt:

```text
cargo test -p sensiblaw-pg-source-store \
  --test latent_world_live \
  -- --ignored --nocapture

1 passed; 0 failed
```

This means the former P7 storage/read-projection prerequisite is already paid.
The remaining P7 work is product-scale population/exploration, not invention of
a new world-store abstraction.

## Remaining Mabo propositions — GENERIC REGISTRY SOURCE-WRITTEN

`mabo_five_stage_registry()` mirrors the five bounded Agda reading roles:

```text
challenged premise
historical input
authority proposition
immediate implication
downstream application
```

Only the radical-title/native-title authority proposition currently carries the
fully paid live source/support coordinate. The other stages retain explicit
source/proof residuals until their own exact spans and reviewed proposition welds
are persisted.

## Generalisation beyond Mabo — GREEN BY CONSTRUCTION

Core types remain proposition-generic:

```text
ReaderPropositionSpec
SourceCoordinate
AdaptiveConePolicy / ConeCandidate
ContextBundle / ContextLink
WorldNode / WorldEdge / ReaderWorldProjection
ReaderIntent / ReaderDisposition / PropositionPayment
```

A non-Mabo research-paper fixture remains the regression that these APIs do not
branch on Mabo identity.

## Australian legal corpus / acquisition lane

The existing Australian legal corpus work remains part of the acquisition plane,
not a parallel semantic database.

```text
stable AU legal catalogue / governed provider candidate
-> retained provider/source-role provenance
-> canonical PostgreSQL document + revision + exact spans
-> CandidatePNF producer
-> review/admission
-> consumer payment
```

Provider/corpus presence never directly pays legal authority or proposition
support:

```text
corpus membership       != authority
HF publication/download != semantic payment
AustLII availability    != official-source equivalence
source agreement        != independent provenance
acquisition receipt     != proposition support
```

## Current Pareto frontier

```text
P0    exact source/span                                      PAID
P1    bounded SLR proposition calculus                       PAID
P2    Agda proposition-chain contract                         PAID
P4    portable Rust Reader ABI                               PAID
P3a0  parser-neutral CandidatePNF producer                    PAID FOR FLAGSHIP
P3a1  reviewed PNF admission/materialisation                  PAID FOR FLAGSHIP
P3a2  reviewed revision -> legal_ir support                   PAID FOR FLAGSHIP
P3b   live PG -> PropositionChainPayment                      PAID LIVE
P3c   chain payment -> PropositionPayment / Reader ABI        PAID LIVE
P5    live Dioxus Semantic Reader                             ACTIVE FRONTIER
P6    ExplanationCone -> ProofConeVisualIR -> wgpu            NEXT AFTER P5
P7    bounded 100-hop latent-world read projection            PAID LIVE
      broader world population/exploration                    LATER SCALE-UP
P8    selected federation/publication fibre                   LATER
```

The dependency graph, not the historical phase numbering, is authoritative.
P7's read-projection primitive is already available and live-tested, so Dioxus
may consume it after the primary reader path is proven without waiting for a new
world persistence design.

## Immediate next acceptance — P5 Dioxus

The production frontier is now the already source-written Dioxus consumer:

```text
live PropositionPayment
-> Dioxus ReaderIntent
-> ReaderDisposition
-> ReaderViewState
```

Required flagship behavior:

```text
OpenSource -> ExecuteSource(exact revision/span)
WhyClaim   -> ExecuteBoundedWhy(ExplanationCone)
ApplicabilityPaid = false
ClaimTruthPaid     = false
```

Dioxus must not recompute proposition payment and must not instantiate a fixture
to make the screen appear complete.

After the ordinary reader path is validated, proceed to:

```text
ExplanationCone
-> ProofConeVisualIR
-> wgpu/WGSL
```

wgpu remains a renderer/interaction backend over already-typed semantic state;
rendered nodes, edges, picks, or layout state do not create semantic authority.

## End-to-end flagship state

Paid today:

```text
judgment bytes
-> PostgreSQL exact source/span
-> parser-neutral candidate PNF
-> reviewed PNF revision
-> legal_ir materialisation
-> SLR proposition-chain payment
-> PropositionPayment / Reader ABI
```

Next production receipt:

```text
-> Dioxus Why? = ExecuteBoundedWhy
```

Then optional visual exploration:

```text
-> ExplanationCone / bounded latent world
-> ProofConeVisualIR
-> wgpu
```

Hard firewalls remain:

```text
serialization/input representation != semantic identity
CandidatePNF                     != ReviewedPNF
ReviewedPNF                      != proposition support
bounded explanation              != applicability
bounded explanation              != claim truth
reader projection                != semantic authority
Dioxus event                     != evidence payment
wgpu render state                != semantic authority
```
