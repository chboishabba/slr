# Semantic Reader Rust production ledger — 2026-09-16

This is the current successor ledger for the Rust production tranche requested
against `docs/roadmap/mabo_proof_explanation_profile_20260915.md`.
The parent roadmap remains the detailed provenance sheet for P0–P4/P3. This file
tracks the cross-cutting runtime implementation added on top of SLR #17.

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

## Latest PR audit

```text
SLR #13  governed Australian acquisition / anchored citations   DONOR ACQUISITION LANE
SLR #14  bounded proposition calculus                            PAID
SLR #15  portable Reader ABI                                    PAID
SLR #17  live PG proposition weld / legal_ir materialiser       PARENT / CURRENT P3
SLR #18  Rust Semantic Reader runtime successor                 FOCUSED GREEN; LIVE WELD STANDBY
DASHI #963 bounded proposition-chain parity                     PAID
DASHI #982 legal_ir materialisation parity                      CURRENT FORMAL PARENT
Dioxus upstream #18 native Rust/backend mediation               MERGED P5 PARENT
Dioxus agent/mabo-semantic-reader-v1                             SOURCE-WRITTEN / downstream
wgpu ProofCone                                                   NOT STARTED / correctly downstream
```

SLR #18 branch:

```text
agent/semantic-reader-runtime-v1
base: agent/mabo-pg-proposition-weld-v1
validated head: 816b5a870c130089003c6574b4e917bef3441c5c
```

## Australian legal corpus / HF acquisition lane

The existing Australian legal corpus work is part of the production acquisition
plane and must remain available to the Semantic Reader retry loop. It is not a
parallel semantic database.

Intended role:

```text
stable AU legal catalogue / HF snapshot / bounded AustLII + official-source seeds
-> governed source candidate or offline replay artefact
-> retained provider/source-role provenance
-> canonical PostgreSQL document + revision + exact spans
-> PNF / reviewed semantic materialisation
-> consumer-relative payment
```

The durable corpus includes the existing Australian/Commonwealth/Queensland and
HCA-oriented legal tranches. Stable catalogue revisions should be built/restored
by explicit acquisition workflows and read by ordinary CI/runtime code rather
than being rebuilt opportunistically on every test run.

Provider role remains explicit. In particular, AustLII/HF-hosted material may be
a supporting or research-index manifestation while an official court/legislation
source remains the controlling authority manifestation for a consumer that
requires it. Therefore:

```text
corpus membership          != authority
HF publication/download    != semantic payment
AustLII availability       != official-source equivalence
source agreement           != independent provenance
acquisition receipt        != proposition support
```

The generic `ReaderDisposition::Defer -> ProofFrontier` recurrence in #18 should
therefore consider the durable AU corpus/local catalogue before requesting live
network acquisition. A locally available corpus hit can pay acquisition/locality
debt, but semantic payment still requires the ordinary exact-span, review,
provenance, and consumer gates.

For the current Mabo radical-title proposition, the already-persisted exact
Brennan source coordinate remains authoritative for `OpenSource`. The broader AU
corpus is a donor for proposition neighbourhoods, comparators, qualifiers,
defeaters, citation follow, and later Mabo-stage acquisition; it must not replace
the exact source coordinate merely because the same text is present in a corpus
snapshot.

## Requested Rust focus — exact status

### 1. Materialise Mabo proof rows in PostgreSQL — FOCUSED GREEN, LIVE REVIEWED SEED OUTSTANDING

New Rust owners:

```text
crates/sl-pg-source-store/src/candidate_pnf.rs
crates/sl-pg-source-store/src/reviewed_pnf.rs
crates/sl-pg-source-store/examples/materialize_mabo_reviewed_pnf.rs
```

The inherited #17 owner remains:

```text
crates/sl-pg-source-store/src/legal_ir_materialization.rs
crates/sl-pg-source-store/examples/materialize_mabo_radical_title_support.rs
```

The implemented recurrence is:

```text
ExactSourceSpan
-> CandidatePnfProducer
-> CandidatePnfBatch                    candidate only
-> explicit external review receipt
-> ReviewedPnfRevision
-> algebra.factor
-> algebra.factor_revision
-> pnf.graph
-> pnf.graph_factor_revision
-> ReviewedPropositionSupport
-> legal_ir.semantic_build
-> legal_ir.projection
-> legal_ir.observation
-> legal_ir.graph_revision
```

There is deliberately no automatic `CandidatePnfBatch -> ReviewedPnfRevision`
promotion. Reviewed persistence still reports proposition support, applicability,
and claim truth as false; support is paid only by the later consumer-relative weld.

Observed focused receipts on `816b5a8...`:

```text
mabo_pnf_candidate_materialization   2 passed
reviewed_pnf_materialization         2 passed
```

The live TrueNAS database still lacks the reviewed Mabo PNF/legal_ir coordinate
needed by the proposition weld. This is a seed/materialisation obligation, not a
parser/runtime failure.

### 2. Live PG -> SLR -> PropositionPayment — RUNTIME GREEN, LIVE MABO WELD STANDBY

Changes:

```text
load_proposition_rows(...)                generic PG read projection
load_mabo_proposition_rows(...)           compatibility alias
project_reader_payment(...)               PropositionChainPayment -> Reader ABI
```

The projector consumes the original typed `PropositionRoleResidual` values. It
does not infer qualifier/defeater/comparator identity from the flattened
`role_residual_refs` list.

Observed focused receipt:

```text
reader_payment_projection            2 passed
```

The live TrueNAS weld currently reaches the expected fail-closed state:

```text
exact Brennan source/span persisted  true
legal_ir reviewed support rows       absent
rows.exact_source_paid               false at proposition-weld query
Why                                  not executable yet
```

After the reviewed Mabo PNF/support coordinate is materialised, the inherited
live test must require:

```text
exactSourcePaid      = true
propositionChainPaid = true
OpenSource           = ExecuteSource
Why                  = ExecuteBoundedWhy
ApplicabilityPaid    = false
ClaimTruthPaid       = false
```

### 6. Adaptive Explanation Cone — GREEN

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

Observed receipts:

```text
semantic_runtime                     5 passed
explanation_selection                1 passed
```

### 7. Automatic acquisition/retry — GREEN

Owner:

```text
crates/sl-proof-search-loop/src/reader_retry.rs
```

Reader `Defer` residuals compile into the existing `ProofFrontier`; no second
planner/provider loop was added.

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

Observed receipt:

```text
reader_retry                         2 passed
```

### 10. Context rabbit holes — SOURCE-WRITTEN

Portable Reader ABI types now distinguish:

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

### 13. 100-hop latent Mabo world — LIVE GREEN READ PROJECTION

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
projection of existing semantic state and cannot create semantic authority.

Observed live TrueNAS receipt:

```text
latent_world_live  1 passed
```

### 14. Remaining Mabo propositions — GENERIC REGISTRY SOURCE-WRITTEN

`mabo_five_stage_registry()` mirrors the five bounded Agda reading roles:

```text
challenged premise
historical input
authority proposition
immediate implication
downstream application
```

Only the already-paid radical-title/native-title authority proposition currently
carries a `SourceCoordinate::Paid`. The other four remain explicit source
residuals until their own exact spans/reviewed proposition welds are persisted.
This prevents the registry itself from manufacturing evidence.

The Australian legal corpus lane should be searched/replayed for these unpaid
coordinates before unnecessary live acquisition, while preserving provider role,
revision identity, source lineage, and exact-span requirements.

### 15. Generalise beyond Mabo — GREEN BY CONSTRUCTION

Core types are generic:

```text
ReaderPropositionSpec
SourceCoordinate
AdaptiveConePolicy / ConeCandidate
ContextBundle / ContextLink
WorldNode / WorldEdge / ReaderWorldProjection
ReaderIntent / ReaderDisposition / PropositionPayment
```

A non-Mabo research-paper fixture is included in the Reader-model contract tests
to ensure these APIs do not branch on Mabo identity.

## Validation status

Focused/runtime validation on SLR #18 head `816b5a8...` is GREEN:

```text
mabo_pnf_candidate_materialization   2 passed
reviewed_pnf_materialization         2 passed
reader_payment_projection            2 passed
semantic_runtime                     5 passed
explanation_selection                1 passed
reader_retry                         2 passed
cargo test --workspace --no-fail-fast    passed
cargo clippy --workspace --all-targets -- -D warnings    passed
latent_world_live                    1 passed (live TrueNAS)
```

The remaining live receipt is deliberately narrow:

```text
mabo_proposition_weld                STANDBY
reason: reviewed Mabo PNF / legal_ir support coordinate not yet persisted
```

Current live database observation:

```text
exact source revision/document/span  present
legal_ir.semantic_build              0 relevant rows
legal_ir.graph_revision              0 relevant rows
legal_ir.projection                  0 relevant rows
legal_ir.observation                 0 relevant rows
```

Therefore do not treat the failed live weld as a runtime regression. The next
producer is the explicit reviewed Mabo PNF/materialisation step, followed by the
same live weld test.

Required final P3/P5 handoff receipt:

```sh
cargo test -p sensiblaw-pg-source-store \
  --test mabo_proposition_weld \
  -- --ignored --nocapture
```

That receipt must cross:

```text
reviewed PNF seed
-> legal_ir rows
-> PropositionChainPayment
-> PropositionPayment
-> OpenSource = ExecuteSource
-> Why = ExecuteBoundedWhy
```

while retaining:

```text
ApplicabilityPaid = false
ClaimTruthPaid    = false
```

After that receipt, the production frontier moves to the already source-written
Dioxus consumer. wgpu remains correctly downstream of the resulting
`ExplanationCone` / future `ProofConeVisualIR`.
