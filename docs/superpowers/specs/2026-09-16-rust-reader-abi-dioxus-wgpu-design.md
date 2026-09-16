# Rust Reader ABI → Dioxus/wgpu Design

Date: 2026-09-16

## Purpose

Freeze the production boundary for the Semantic Reader before adding more UI code.

The production trajectory is:

```text
judgment/source bytes
-> PostgreSQL semantic persistence
-> SLR/Rust parsing + residual + evidence payment
-> typed Rust Reader ABI
-> Dioxus ordinary UI
-> wgpu visualisation/interaction surfaces
```

ITIR/Svelte and SensibLaw Python remain reference/regression surfaces. They do not own the production reader ABI.

## Ownership

```text
DASHI / Agda       golden semantic/proof sufficiency contracts
SLR / Rust         production parsing, residuals, payment, recurrence
PostgreSQL         active semantic persistence spine
sl-reader-model    typed read projection owned inside SLR
Dioxus / Rust      production reader and ordinary interaction shell
wgpu               all charts/graphs/proof/world visualisation and GPU compute
SensibLaw Python   PG/reference/regression projection
ITIR/Svelte        compatibility/reference prototype only
```

`sl-reader-model` is not a new semantic authority. It is a query-indexed read projection over SLR-owned payment state.

## Why the ABI lives in SLR

The shared model must be consumable by Dioxus without importing PostgreSQL, parser internals, or proof-search machinery. It also must not be recreated independently in the UI.

Therefore add a dependency-light workspace crate:

```text
crates/sl-reader-model/
```

package name:

```text
sensiblaw-reader-model
```

The crate may depend on `serde` behind an optional feature for transport projection. It must not depend on:

- PostgreSQL/`sqlx`/`tokio-postgres`;
- Dioxus;
- wgpu;
- SensibLaw Python;
- HTTP/MCP transport;
- model/LLM clients.

SLR payment/proof crates construct reader values. Dioxus reads them. wgpu receives visual projections derived from them.

## Core types

The first ABI is intentionally small and consumer-relative.

```rust
pub struct SemanticRef(pub String);
pub struct SourceRevisionRef(pub String);
pub struct SpanRef(pub String);
pub struct ResidualRef(pub String);
pub struct PropositionRef(pub String);

pub struct SourcePayment {
    pub source_revision_ref: SourceRevisionRef,
    pub span_ref: SpanRef,
    pub exact_authority_span_paid: bool,
}

pub enum CoordinateCoverage {
    Paid { evidence_refs: Vec<String> },
    Residual { residual_ref: ResidualRef },
}

pub struct PropositionPayment {
    pub proposition_ref: PropositionRef,
    pub source: SourcePayment,
    pub support_paid: bool,
    pub qualifier: CoordinateCoverage,
    pub defeater: CoordinateCoverage,
    pub comparator: CoordinateCoverage,
    pub applicability_paid: bool,
    pub claim_truth_paid: bool,
}

pub enum ReaderIntent {
    Explain,
    WhyClaim(PropositionRef),
    OpenSource(PropositionRef),
    ExpandProofCone(PropositionRef),
    Back,
}

pub enum ReaderDisposition {
    ExecuteSource {
        proposition_ref: PropositionRef,
        source_revision_ref: SourceRevisionRef,
        span_ref: SpanRef,
    },
    ExecuteBoundedWhy {
        cone: ExplanationCone,
    },
    Defer {
        residuals: Vec<ResidualRef>,
    },
    Reject {
        reason: String,
    },
}

pub struct ExplanationCone {
    pub proposition_ref: PropositionRef,
    pub source: SourcePayment,
    pub support_refs: Vec<String>,
    pub qualifier: CoordinateCoverage,
    pub defeater: CoordinateCoverage,
    pub comparator: CoordinateCoverage,
    pub applicability_paid: bool,
    pub claim_truth_paid: bool,
}
```

Names may be refined during implementation only if the same semantic separation remains explicit.

## Payment rule

For the first Mabo specimen:

```text
WhyPaid =
    ExactSourcePaid
 && SupportPaid
 && Covered(Qualifier)
 && Covered(Defeater)
 && Covered(Comparator)
```

where:

```text
Covered(role) = Paid(role) || ExplicitResidual(role)
```

Support is mandatory and cannot be replaced by a residual.

The reader ABI must preserve:

```text
ExactSourcePaid !=> PropositionChainPaid
PNFParity !=> SourceProvenance
BoundedWhyPaid !=> ApplicabilityPaid
BoundedWhyPaid !=> ClaimTruthPaid
ExplicitResidual !=> PropositionFalse
ReaderProjection !=> SemanticAuthority
```

## SLR adapter boundary

The existing `sensiblaw-evidence-payment` crate remains the payment engine. It should expose/construct the typed reader projection only after the existing proposition-payment test contract is satisfied.

The first production adapter is conceptually:

```rust
pub fn project_reader_payment(
    evidence: &MaboPropositionEvidence,
) -> PropositionPayment;

pub fn resolve_reader_intent(
    payment: &PropositionPayment,
    intent: ReaderIntent,
) -> ReaderDisposition;
```

The adapter consumes typed SLR/PG/PNF coordinates. It must not ingest detached JSON or re-parse UI prose.

## Dioxus boundary

Dioxus owns ordinary application UI:

- routes;
- menus/forms/search;
- reading/document panes;
- disclosure controls;
- textual source/provenance cards;
- accessibility/chrome;
- semantic intent dispatch.

Dioxus does not decide whether a proposition/source/proof coordinate is paid.

The component flow is:

```text
Dioxus event
-> ReaderIntent
-> ReaderDisposition
-> Dioxus textual/readable projection
-> optional VisualisationProjection for wgpu
```

For browser/WASM deployment, backend/HTTP/WebSocket/MCP may transport a serialized `ReaderDisposition`. For native/fullstack Rust, direct Rust integration may be used. Transport must not redefine the semantic contract.

The writable implementation target is `chboishabba/solfunmeme-dioxus`; `meta-introspector/solfunmeme-dioxus` remains the upstream/reference source where write access is unavailable.

## wgpu boundary

wgpu owns all data visualisation and interaction-heavy visual surfaces, not merely 3D graphs:

- line/bar/area/scatter/timeline charts;
- proof cones;
- PNF graphs;
- Sankey/hyperfabric views;
- 100-hop latent Mabo world;
- picking/brushing/selection;
- pan/zoom/orbit;
- animation;
- LOD/culling;
- GPU layout/compute.

The first ABI tranche does not add wgpu code. It only ensures reader semantics are independent of future renderer choice.

A later visual projection must have the shape:

```text
Reader/World state
-> VisualisationIR
-> wgpu
```

not:

```text
Dioxus DOM/RSX tree
-> graph reconstruction
-> wgpu
```

and:

```text
wgpu output != semantic authority
rendered edge != evidence payment
GPU pick != semantic mutation
```

A GPU pick may decode to the same `ReaderIntent` as a Dioxus button.

## Serialization and transport

Serialization is optional and downstream:

```text
Rust Reader ABI
-> serde transport projection
-> HTTP/WebSocket/MCP/backend bridge
-> Rust Reader ABI
```

The first implementation should use an optional `serde` feature and derive serialisation only for the stable read types needed by Dioxus web transport.

Rules:

- no JSON file is a persistence authority;
- transport round-trip must not promote payment state;
- unknown/unsupported enum variants fail closed at the boundary;
- semantic IDs/revision/span refs survive transport exactly;
- native Rust consumers should not be forced through JSON.

## First vertical specimen

Use the already-materialised Mabo coordinate:

```text
mabo:proposition:radical-title-native-title
span:mabo:brennan:radical-title:no-automatic-beneficial-ownership
```

Required states:

1. exact source only:

```text
OpenSource -> ExecuteSource
WhyClaim   -> Defer(detailed proposition chain)
ApplicabilityPaid = false
ClaimTruthPaid    = false
```

2. paid bounded proposition chain:

```text
OpenSource -> ExecuteSource
WhyClaim   -> ExecuteBoundedWhy
ApplicabilityPaid = false
ClaimTruthPaid    = false
```

3. PNF support without independent source-span provenance:

```text
WhyClaim -> Defer
```

4. unresolved qualifier/defeater/comparator represented by explicit residual:

```text
WhyClaim may ExecuteBoundedWhy
```

and the residual remains visible in `ExplanationCone`.

## Testing

### SLR

TDD order:

1. observe the existing `mabo_proposition_payment` RED;
2. implement the minimal proposition-payment evaluator;
3. add `sl-reader-model` contract tests;
4. add evidence-payment -> reader-model adapter tests;
5. run crate tests, workspace tests, then clippy.

### Dioxus

Add pure Rust tests before component rendering tests:

- a source-paid disposition renders source action availability;
- a deferred Why retains residual IDs;
- a bounded Why exposes support/qualifier/defeater/comparator without promoting applicability/truth;
- UI events compile to `ReaderIntent` rather than editing payment flags;
- optional serde roundtrip preserves exact IDs and non-promotion flags.

No wgpu test is required in this first ABI tranche.

## Roadmap consequence

The previous P4 `human Explanation Cone` milestone is too broad. Split it:

```text
P0 observe SLR RED
P1 make SLR proposition payment GREEN
P2 certify Agda proposition-chain owner
P3 live PG -> SLR proposition payment
P4 typed Rust Reader ABI (`sl-reader-model`)
P5 Dioxus Semantic Reader consumes Reader ABI
P6 wgpu VisualisationIR + first proof-cone/chart surface
P7 100-hop Mabo latent world through same projection
P8 selected eRDFa/IPFS/Kant publication fibre
```

Svelte is not on the production critical path after P3. It remains a regression oracle until equivalent Rust/Dioxus behavior is independently certified.

## Acceptance criterion for this ABI tranche

One real Mabo proposition must cross:

```text
PG/SLR paid coordinates
-> PropositionPayment
-> ReaderIntent
-> ReaderDisposition
```

with source and bounded-Why states correctly separated and with:

```text
ApplicabilityPaid = false
ClaimTruthPaid    = false
```

The next tranche may then bind those exact Rust values into Dioxus without recreating the payment calculus.