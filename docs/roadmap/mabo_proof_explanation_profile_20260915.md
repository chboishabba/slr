# SLR roadmap: thin Mabo proof/explanation profile

Date: 2026-09-15
Updated: 2026-09-16 — parser-neutral candidate-PNF / Dioxus production path

## Active ownership

```text
DASHI/Agda          = golden semantic / proof contract
SLR/Rust            = production parser / residual / payment / recurrence engine
PostgreSQL          = sole active semantic persistence spine
Dioxus/Rust         = production reader + ordinary interaction surface
wgpu/WGSL           = production chart / graph / proof-cone / world visualisation backend
SensibLaw Python    = reference + PostgreSQL regression surface
ITIR/Svelte         = reference/regression prototype only
```

Detached TSV and JSON are ingestion/export/transport manifestations only. They are
not semantic identities and must not be re-ingested as world, source-follow, or
evidence-payment authority.

The stable production path is:

```text
PDF / HTML / XML / text / provider payload / legacy TSV / legacy JSON
-> source/parser adapter
-> typed CandidatePNF
-> review/admission
-> ReviewedPNFRevision
-> PostgreSQL canonical semantic persistence
-> PropositionEvidenceObservation
-> SLR consumer-relative PropositionChainPayment
-> portable Rust PropositionPayment / Reader ABI
-> Dioxus human reader
-> optional wgpu ProofCone/world projection
```

Stable semantic boundaries are typed Rust values plus canonical persisted
coordinates:

```text
ExactSourceSpan
-> CandidatePnfFactor
-> ReviewedPnfFactorRevision
-> PropositionEvidenceObservation
-> PropositionChainPayment
-> PropositionPayment
```

TSV and JSON do not appear in that chain.

For native/full-stack Rust, pass typed values directly. If a browser/backend
boundary needs serialization, the encoding is a transport manifestation of the
typed ABI, not its definition:

```text
Reader ABI -> transport encoding -> Reader ABI
```

`serde_json(PropositionPayment)` is therefore acceptable transport; JSON fields
do not define `PropositionPayment` semantics.

## Candidate-PNF producer ABI

P3a0 is parser-neutral:

```rust
trait CandidatePnfProducer {
    fn produce(
        &self,
        source: &ExactSourceSpan,
    ) -> Result<CandidatePnfBatch, CandidatePnfError>;
}
```

Temporary/replaceable producers may include:

```text
SpacyTsvAdapter        compatibility/conformance only
LegacyJsonAdapter      compatibility/conformance only
PdfTextAdapter         source adapter
NativeRustParser       preferred native producer when adequate
ProviderAdapter        governed external producer
```

All terminate at the same typed candidate carrier. Removing TSV or JSON later
must not affect review, persistence, payment, Dioxus, wgpu, or Agda.

Firewalls:

```text
input/serialization representation != semantic identity
CandidatePNF                    != ReviewedPNF
ReviewedPNF                     != PropositionSupport
transport encoding              != semantic authority
```

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

Observed source payment remains:

```text
exact_authority_span_paid = true
OpenSource                = execute
ApplicabilityPaid         = false
ClaimTruthPaid            = false
```

The live P3 test subsequently established the remaining persistence gap:
`legal_ir.semantic_build`, `legal_ir.projection`, `legal_ir.observation`, and
`legal_ir.graph_revision` were empty, so `Why` correctly remained unpaid.

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

## Current PR / branch ledger

### SLR #14 — proposition calculus — PAID

Branch:

```text
agent/mabo-radical-title-proposition-payment
```

Observed:

```text
mabo_proposition_payment: 4/4 passed
cargo test --workspace --no-fail-fast: passed
cargo clippy --workspace --all-targets -- -D warnings: passed
```

### DASHI #963 — proposition-chain parity — PAID

Branch:

```text
agent/mabo-radical-title-proposition-chain-parity
```

Head recorded after focused certification:

```text
9c3f59fbdf742e7fc474c98a929c3a1a9a9bdc01
```

`MaboRadicalTitlePropositionChainPaymentExact.agda` kernel-checks and keeps
bounded explanation separate from applicability and claim truth.

### SLR #15 — portable Rust Reader ABI — PAID

Branch:

```text
agent/mabo-reader-abi-v1
```

Head:

```text
a05b13726087187e202dbefad1f7f79abd065767
```

Observed RED first with the eight missing reader types, then GREEN:

```text
cargo test -p sensiblaw-reader-model --test reader_model: 3 passed
cargo test --workspace --no-fail-fast: passed
cargo clippy --workspace --all-targets -- -D warnings: passed
```

The ABI is Rust-only and has no PostgreSQL, Dioxus, wgpu, Python, Svelte, or
transport authority.

### SLR #17 — live PG proposition weld — CURRENT

Branch:

```text
agent/mabo-pg-proposition-weld-v1
```

The read-side query consumes persisted `legal_ir` rows without assigning reader
proof roles. A live TrueNAS run compiled and failed at the intended semantic
boundary because the four `legal_ir` materialisation tables contained zero rows.

P3 is now refined without making any serialization format part of the semantic
architecture:

```text
P3a0 = ExactSourceSpan -> parser-neutral CandidatePNF
P3a1 = CandidatePNF + review/admission -> ReviewedPNFRevision
P3a2 = ReviewedPNFRevision -> legal_ir persisted support
P3b  = live legal_ir rows -> PropositionChainPayment
P3c  = PropositionChainPayment -> PropositionPayment / Reader ABI
```

Current RED contract:

```text
crates/sl-pg-source-store/tests/mabo_pnf_candidate_materialization.rs
```

It now tests the generic `CandidatePnfProducer` boundary. `SpacyTsvAdapter` is
explicitly temporary and exists only to prove that an older parser manifestation
can be converted into the canonical candidate carrier. Candidate output remains
candidate-only and cannot pay support, applicability, or truth.

P3a2 owner already source-written:

```text
crates/sl-pg-source-store/src/legal_ir_materialization.rs
```

It persists one *reviewed* PNF support coordinate into the existing SensibLaw
schema:

```text
legal_ir.semantic_build
-> legal_ir.projection
-> legal_ir.observation
-> legal_ir.graph_revision
```

The materialiser:

- validates the exact persisted source revision/document/span against canonical
  PostgreSQL text;
- requires explicit reviewed PNF build/graph/factor/revision coordinates;
- requires observation provenance already containing the exact span;
- retains the same span independently in `graph_revision.source_span_refs`;
- uses candidate persistence state only;
- creates no applicability, holding, claim truth, or reader authority;
- is idempotent by deterministic identities and rejects conflicting existing
  rows.

Operator entrypoint:

```text
crates/sl-pg-source-store/examples/materialize_mabo_radical_title_support.rs
```

The entrypoint deliberately requires reviewed PNF coordinates from the
operator/admission path rather than deriving them from source text.

P3b read/payment boundary:

`load_mabo_proposition_rows` reads only storage-owned coordinates: exact
canonical source/span readiness, PNF observation identity/provenance, and an
independent `legal_ir.graph_revision` source-span weld. It deliberately does
not interpret PNF `role_bindings` as reader proof roles.

Qualifier/defeater/comparator remain explicit consumer-relative SLR residual
debts. The production storage crate does not evaluate proposition payment.

Expected live result after one reviewed materialisation:

```text
exactSourcePaid       = true
propositionChainPaid  = true
whyExecutable         = true
ApplicabilityPaid     = false
ClaimTruthPaid        = false
```

### DASHI #982 — legal-IR materialisation parity — CURRENT

Branch:

```text
agent/mabo-legal-ir-materialisation-parity
```

Owner:

```text
DASHI/Interop/MaboRadicalTitleLegalIRMaterialisationExact.agda
```

It reuses #963 and proves that persisted rows pay support only when PNF revision
identity, observation exact-span provenance, and independent graph exact-span
provenance are all present. Missing any one leaves support unpaid. The canonical
materialised receipt executes bounded Why while applicability and claim truth
remain false.

### Dioxus P5 — source-written, downstream of P3

Repository:

```text
chboishabba/solfunmeme-dioxus
```

Branch:

```text
agent/mabo-semantic-reader-v1
```

Known source-written head:

```text
5c2f6a4df1dd350657950bc4558e19eed0aac7d2
```

Dioxus consumes the SLR Reader ABI directly and fails closed while no live
`PropositionPayment` is supplied. Do not add presentation work until P3b/P3c are
paid.

### wgpu P6 — NOT STARTED

Correctly downstream:

```text
ExplanationCone -> ProofConeVisualIR -> wgpu/WGSL
```

wgpu renders already-typed projections. Rendered nodes/edges do not create
semantic authority or evidence payment.

## SensibLaw Python reference donors

Production Rust reuses the contracts, not the Python object model.

Existing reference donors include:

```text
database/postgres_migrations/015_legal_ir_federation.sql
src/pnf/legal_semantic_build.py
src/pnf/legal_adjunct.py
src/pnf/legal_ir_projection_bridge.py
src/storage/postgres/semantic_store.py
```

Key inherited rule:

```text
refined PNF = candidate semantic state
Legal IR    = deterministic/materialised PNF projection
legacy extraction = diagnostic witness only
```

The older Mabo proof-graph specimen remains a correspondence/regression donor;
it is a repository-summary fixture and is not substituted for the exact Brennan
judgment span.

## Legal-follow / hosted Australian legal corpus

Use the existing governed `legal-follow` / acquisition architecture. Do not add
a second provider planner.

```text
legal-follow residual
-> existing governed provider plan
-> local PG cache if available
-> governed provider candidate
-> exact source/revision admission
-> canonical PG materialisation
-> CandidatePNF producer
-> review/admission
-> consumer payment
```

Provider or corpus hits improve acquisition/search but never pay legal authority
or proposition support by themselves.

## High-alpha order

```text
P0    exact source/span                                      PAID
P1    bounded SLR proposition calculus                       PAID
P2    Agda proposition-chain contract                         PAID
P4    portable Rust Reader ABI                               PAID
P3a0  parser-neutral exact-span CandidatePNF producer         RED contract current
P3a1  reuse/find CandidatePNF -> reviewed revision admission  ARCHAEOLOGY NEXT
P3a2  reviewed revision -> legal_ir support                   SOURCE-WRITTEN
P3b   live PG -> PropositionChainPayment                      NEXT AFTER P3a
P3c   chain payment -> PropositionPayment / Reader ABI        TINY WELD
P5    live Dioxus Semantic Reader
P6    ExplanationCone -> VisualIR -> wgpu
P7    100-hop latent Mabo world
P8    selected federation/publication fibre
```

P3 and P4 were completed partly out of numeric order; the dependency graph, not
phase numbering, is authoritative.

Do not deepen TSV or JSON infrastructure. They are compatibility adapters that
should be removable without semantic consequences.

## Immediate acceptance

P3a0 acceptance is representation-neutral:

```text
ExactSourceSpan + CandidatePnfProducer
-> typed candidate batch scoped to the exact span
-> candidateOnly = true
-> propositionSupportPaid = false
-> applicabilityPaid = false
-> claimTruthPaid = false
```

After a reviewed revision is admitted and materialised, rerun:

```sh
cargo test -p sensiblaw-pg-source-store \
  --test mabo_proposition_weld \
  -- --ignored --nocapture
```

Required live result:

```text
exactSourcePaid       = true
propositionChainPaid  = true
whyExecutable         = true
ApplicabilityPaid     = false
ClaimTruthPaid        = false
```

## End-to-end acceptance

The flagship is paid when one real proposition crosses:

```text
judgment bytes
-> PostgreSQL exact source
-> parser-neutral CandidatePNF
-> reviewed PNF revision
-> legal_ir materialisation
-> SLR proposition-chain payment
-> PropositionPayment / Reader ABI
-> Dioxus Why? ExecuteBoundedWhy
-> optional wgpu ProofCone
```

while preserving:

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
