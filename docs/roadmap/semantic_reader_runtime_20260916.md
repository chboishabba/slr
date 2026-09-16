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
source bytes
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

No JSON/TSV/Svelte/Python object is semantic identity in this path.

## Latest PR audit

```text
SLR #14  bounded proposition calculus                        PAID
SLR #15  portable Reader ABI                                PAID
SLR #17  live PG proposition weld / legal_ir materialiser   PARENT / CURRENT P3
SLR #18  Rust Semantic Reader runtime successor             SOURCE-WRITTEN
DASHI #963 bounded proposition-chain parity                  PAID
DASHI #982 legal_ir materialisation parity                   CURRENT FORMAL PARENT
Dioxus upstream #18 native Rust/backend mediation            MERGED P5 PARENT
Dioxus agent/mabo-semantic-reader-v1                         SOURCE-WRITTEN / downstream
wgpu ProofCone                                               NOT STARTED / correctly downstream
```

SLR #18 branch:

```text
agent/semantic-reader-runtime-v1
base: agent/mabo-pg-proposition-weld-v1
```

## Requested Rust focus — exact status

### 1. Materialise Mabo proof rows in PostgreSQL — SOURCE-WRITTEN, LIVE RECEIPT UNPAID

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

### 2. Live PG -> SLR -> PropositionPayment — SOURCE-WRITTEN, LIVE RECEIPT UNPAID

Changes:

```text
load_proposition_rows(...)                generic PG read projection
load_mabo_proposition_rows(...)           compatibility alias
project_reader_payment(...)               PropositionChainPayment -> Reader ABI
```

The projector consumes the original typed `PropositionRoleResidual` values. It
does not infer qualifier/defeater/comparator identity from the flattened
`role_residual_refs` list.

The inherited live TrueNAS test now continues through the Reader ABI and requires:

```text
exactSourcePaid      = true
propositionChainPaid = true
OpenSource           = ExecuteSource
Why                  = ExecuteBoundedWhy
ApplicabilityPaid    = false
ClaimTruthPaid       = false
```

### 6. Adaptive Explanation Cone — SOURCE-WRITTEN

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

### 7. Automatic acquisition/retry — SOURCE-WRITTEN ADAPTER

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
-> persistence/world extension
-> mandatory reader re-evaluation
-> Execute | Defer | Reject
```

Hard firewall:

```text
acquisition receipt != semantic payment
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

### 13. 100-hop latent Mabo world — SOURCE-WRITTEN READ PROJECTION

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

### 15. Generalise beyond Mabo — SOURCE-WRITTEN BY CONSTRUCTION

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

At this ledger revision the new #18 tranche is **source-written, not GREEN**.
The execution container used for this implementation cannot resolve GitHub and
there are no GitHub Actions runs on the exact #18 head. Do not upgrade this status
until the real Rust toolchain observes the focused tests, workspace suite, Clippy,
and live TrueNAS test.

Required focused checks:

```sh
cargo test -p sensiblaw-pg-source-store --test mabo_pnf_candidate_materialization
cargo test -p sensiblaw-pg-source-store --test reviewed_pnf_materialization
cargo test -p sensiblaw-evidence-payment --test reader_payment_projection
cargo test -p sensiblaw-reader-model --test semantic_runtime
cargo test -p sensiblaw-proof-search-loop --test reader_retry
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
```

After an explicitly reviewed Mabo PNF coordinate is materialised:

```sh
cargo test -p sensiblaw-pg-source-store \
  --test mabo_proposition_weld \
  -- --ignored --nocapture
```

That live receipt is the P3/P5 handoff gate. Dioxus should consume the resulting
`PropositionPayment`; wgpu remains downstream of the resulting `ExplanationCone`.
