# SLR roadmap: thin Mabo proof/explanation profile

Date: 2026-09-15
Updated: 2026-09-16 — P3a legal-IR materialisation tranche

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

Detached JSON is presentation/export only. It is not re-ingested as semantic,
world, source-follow, or evidence-payment authority.

The production path is:

```text
judgment/source bytes
-> PostgreSQL canonical source + exact span
-> SLR reviewed PNF / legal_ir materialisation
-> SLR consumer-relative proposition payment
-> portable Rust Reader ABI
-> Dioxus human reader
-> optional wgpu ProofCone/world projection
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
+ retained PNF observation
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

The read-side query already consumes persisted `legal_ir` rows without assigning
reader proof roles. A live TrueNAS run compiled and failed at the intended
semantic boundary because the four `legal_ir` materialisation tables contained
zero rows.

P3 is therefore split:

```text
P3a = SLR-owned legal_ir materialisation       <- current
P3b = live PG rows -> PropositionPayment
```

P3a owner:

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

The entrypoint deliberately requires the reviewed PNF coordinates from the
operator/environment rather than deriving them from source text.

### DASHI #982 — legal-IR materialisation parity — CURRENT

Branch:

```text
agent/mabo-legal-ir-materialisation-parity
```

Owner:

```text
DASHI/Interop/MaboRadicalTitleLegalIRMaterialisationExact.agda
```

It reuses #963 and proves that persisted build/projection/observation rows pay
support only when PNF revision identity, observation exact-span provenance, and
independent graph exact-span provenance are all present. Missing any one leaves
support unpaid. The canonical materialised receipt executes bounded Why while
applicability and claim truth remain false.

Source is written; the new file's focused Agda kernel receipt is not yet claimed.

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
`PropositionPayment` is supplied. Do not add presentation work until P3b is
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

Use the existing governed `legal-follow` / acquisition architecture. Do not add a
second provider planner.

The hosted `isaacus/open-australian-legal-corpus` is a useful acquisition and
retrieval producer for Australian legislation and decisions, including High
Court material. Its dataset/source/version coordinates should be retained on
source receipts and then admitted through the same revisioned PostgreSQL source
path.

```text
legal-follow residual
-> existing governed provider plan
-> local PG cache if available
-> hosted Australian legal corpus candidate
-> exact source/revision admission
-> canonical PG materialisation
-> PNF / consumer payment
```

Firewalls:

```text
HF corpus hit      != legal authority payment
HF text            != proposition support
QID identity       != legal applicability
WrongType(tort) -> candidate QID(tort) is navigation/type context only
Wikidata identity  != evidence payment
```

HF and Wikidata/QIDs may therefore improve follow/search/type navigation without
becoming proof or legal authority.

## High-alpha order

```text
P0   exact source/span                         PAID
P1   bounded SLR proposition calculus          PAID
P2   Agda proposition-chain contract            PAID
P4   portable Rust Reader ABI                   PAID
P3a  SLR legal_ir materialisation               CURRENT
P3b  live PG -> PropositionPayment              NEXT
P5   live Dioxus Semantic Reader
P6   ExplanationCone -> VisualIR -> wgpu
P7   100-hop latent Mabo world
P8   selected federation/publication fibre
```

P3 and P4 were completed partly out of numeric order; the dependency graph, not
phase numbering, is authoritative.

## Immediate acceptance commands

First materialise one *reviewed* PNF support coordinate using the Rust example.
Then rerun:

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

Focused Agda target:

```sh
agda -i . DASHI/Interop/MaboRadicalTitleLegalIRMaterialisationExact.agda
```

Do not mark P3a/P3b or DASHI #982 paid until those exact receipts are observed.

## End-to-end acceptance

The flagship is paid when one real proposition crosses:

```text
judgment bytes
-> PostgreSQL exact source
-> reviewed PNF coordinate
-> legal_ir materialisation
-> SLR proposition payment
-> Agda-valid bounded explanation contract
-> portable Rust Reader ABI
-> Dioxus Why? ExecuteBoundedWhy
-> optional wgpu ProofCone
```

while preserving:

```text
bounded explanation != applicability
bounded explanation != claim truth
reader projection    != semantic authority
Dioxus event         != evidence payment
wgpu render state    != semantic authority
```
