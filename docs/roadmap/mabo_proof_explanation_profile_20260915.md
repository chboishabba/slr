# SLR roadmap: thin Mabo proof/explanation profile

Date: 2026-09-15
Updated: 2026-09-16

## Purpose

Define the legal flagship around the active production split rather than the legacy Python/Svelte module graph.

The active ownership rule is:

```text
DASHI/Agda          = golden semantic / proof contract
SLR/Rust            = production parser / residual / evidence-payment / recurrence engine
PostgreSQL          = sole active semantic persistence spine
Dioxus/Rust         = production reader + ordinary interaction surface
wgpu/WGSL           = production chart / graph / proof-cone / world visualisation backend
SensibLaw Python    = reference + PostgreSQL regression surface
ITIR/Svelte         = reference/regression prototype only
```

Detached JSON is presentation/export only. It must not be re-ingested as semantic, world, source-follow, or evidence-payment authority.

The profile is deliberately thin:

```text
exact persisted source/span
-> Rust SLR consumer-relative proposition payment
-> portable typed Reader ABI
-> Dioxus human reader
-> optional wgpu proof/world projection
```

SLR owns deterministic compilation, consumer-relative residual production, evidence-payment execution, bounded recurrence, and provenance-preserving runtime receipts.

DASHI owns the golden laws that say when those receipts are sufficient for the declared consumer query and which promotions remain impossible.

PostgreSQL owns the durable source/revision/span and semantic-materialisation coordinates required by the runtime. Dioxus consumes typed results; it is not a semantic owner. wgpu consumes already-typed visual projections; it never creates evidence payment, applicability, or truth.

## Why Mabo remains the flagship

The point is not to globally "prove Mabo". The first complete specimen should take one concrete proposition and cross every active layer:

```text
judgment bytes
-> PostgreSQL canonical source + exact span
-> SLR PNF/proposition support
-> independent source-span provenance weld
-> support + qualifier/defeater/comparator coverage
-> bounded Why? payment
-> typed Rust Reader ABI
-> Dioxus explanation
-> optional wgpu proof cone
```

The currently paid source coordinate is:

```text
mabo:proposition:radical-title-native-title
span:mabo:brennan:radical-title:no-automatic-beneficial-ownership
```

The exact-source path is already observed live:

```text
exact_authority_span_paid = true
OpenSource                = execute
Why                       = defer
proposition_chain_paid     = false
claim_truth_paid           = false
```

The bounded proposition-payment evaluator and its Agda parity owner are now toolchain-certified, so the remaining runtime frontier is no longer proposition calculus design. It is live PG -> typed SLR payment -> portable Reader ABI -> Dioxus.

## Production proposition-payment contract

The bounded explanation consumer is intentionally weaker than legal truth/applicability closure.

Support is mandatory and cannot be residualised away. It must retain:

```text
PNF observation
+ PNF revision identity
+ observation provenance containing the exact paid span
+ independent graph/source-span provenance containing the same exact span
```

Qualifier, defeater, and comparator coordinates may each be either:

```text
paid observation
or
explicit retained residual
```

The consumer gate is:

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
Covered(role) = PaidObservation(role) || ExplicitResidual(role)
SupportPaid != ExplicitResidual(Support)
```

Firewalls:

```text
ExactSourcePaid !=> PropositionChainPaid
PNFParity !=> SourceProvenance
BoundedWhyPaid !=> ApplicabilityPaid
BoundedWhyPaid !=> ClaimTruthPaid
ExplicitResidual !=> PropositionFalse
```

## Portable Rust Reader ABI

The production UI must not reconstruct semantic payment from Dioxus state or from a serialized Svelte-shaped object.

The retained Rust boundary is:

```text
PostgreSQL
-> SLR evidence/proposition state
-> sensiblaw-reader-model
-> Dioxus
```

The new `sensiblaw-reader-model` crate owns only portable reader contracts such as:

```text
SemanticRef
SourceRevisionRef
SpanRef
ResidualRef
ReaderIntent
SourcePayment
PropositionPayment
CoordinateCoverage
ExplanationCone
ReaderDisposition
```

It must have no dependency on PostgreSQL, Dioxus, wgpu, SensibLaw Python, or Svelte.

Dioxus may emit:

```text
OpenSource
WhyClaim
ExpandProofCone
Back
```

but it must not construct or mutate payment coordinates.

The central ownership firewall is:

```text
Dioxus event != ReaderIntent != SLR payment != PG evidence
```

and:

```text
wgpu visual object != semantic object
GPU pick != semantic mutation
rendered edge != evidence payment
```

## Current TDD / formal state

### Rust SLR proposition payment

Draft PR #14 on:

```text
agent/mabo-radical-title-proposition-payment
```

contains the typed evaluator and tests for:

1. exact source + provenance-welded support + explicit residuals for qualifier/defeater/comparator -> bounded Why paid;
2. PNF support without independent graph/source-span provenance -> Why unpaid;
3. exact source alone -> Why unpaid;
4. a paid qualifier may replace its explicit residual without paying applicability/truth.

Observed validation:

```text
mabo_proposition_payment: 4/4 passed
cargo test --workspace --no-fail-fast: passed
cargo clippy --workspace --all-targets -- -D warnings: passed
```

### DASHI/Agda proposition-chain parity

Draft PR #963 on:

```text
agent/mabo-radical-title-proposition-chain-parity
```

adds:

```text
DASHI/Interop/MaboRadicalTitlePropositionChainPaymentExact.agda
```

The owner formalises the bounded proposition payment and non-promotion firewalls. Its exact-head Agda kernel receipt is observed through the full focused dependency chain.

### Rust Reader ABI

Draft PR #15 on:

```text
agent/mabo-reader-abi-v1
```

is the RED-first successor. It currently contains only:

```text
crates/sl-reader-model/Cargo.toml
crates/sl-reader-model/src/lib.rs   # intentionally empty RED surface
crates/sl-reader-model/tests/reader_model.rs
```

The required RED command is:

```sh
cargo test -p sensiblaw-reader-model --test reader_model
```

Production ABI structs must not be implemented until that failure is observed. The current ChatGPT execution environment has no Cargo/Rust toolchain, and GitHub produced no workflow run for the PR head, so the RED receipt remains unpaid at this point.

## PostgreSQL boundary

PostgreSQL remains the sole active semantic persistence spine.

The retained source/proof shapes remain the existing persisted coordinates; no second proof-store schema is introduced merely for the reader ABI.

The live Mabo adapter must fail closed unless it can establish:

```text
exact persisted source/span
+ PNF support observation
+ observation provenance carrying exact span
+ independent graph/source-span provenance carrying exact span
+ qualifier/defeater/comparator paid-or-residual coverage
```

The resulting typed payment must still report:

```text
ApplicabilityPaid = false
ClaimTruthPaid    = false
```

## Dioxus production reader

The human-facing rule remains:

```text
argument-first
-> graph-second
-> provenance-on-demand
```

Dioxus is the production ordinary UI. It should initially expose:

```text
What changed?
Why does this step follow here?
What exact source supports it?
What limits / contradicts / compares with it?
What remains unresolved?
```

The Dioxus component consumes `ReaderDisposition`; it does not calculate evidence payment.

For the focal proposition:

```text
OpenSource -> ExecuteSource(revision, span)
WhyClaim   -> ExecuteBoundedWhy(cone) | Defer(residual)
```

Svelte remains useful only as a behavior/regression oracle while Dioxus reaches parity. It is not on the production critical path.

## wgpu production visualisation

All charting and interaction-heavy visualisation belongs on the wgpu side, including:

```text
ordinary charts
timelines
proof cones
PNF graphs
Sankey/hyperfabric
100-hop latent Mabo world
picking / brushing / zoom / animation
GPU layout / culling / compute
```

Dioxus remains responsible for ordinary application chrome, forms, document panes, menus, textual source/provenance views, settings, and accessible controls.

The first visual seam is renderer-neutral:

```text
ExplanationCone
-> ProofConeVisualIR
-> wgpu/WGSL
```

not:

```text
Dioxus component tree
-> semantic graph
```

## World-bucket relation

The 100-hop Mabo world bucket is downstream/parallel substrate, not the blocking proof task.

The revised order is:

```text
exact source payment
-> bounded proposition-chain payment
-> live PG typed payment
-> portable Rust Reader ABI
-> Dioxus live Why?/Explanation Cone
-> wgpu ProofCone / visualisation
-> larger latent Mabo world bucket
-> selected Kant/eRDFa/IPFS publication fibre
```

The bucket can eventually supply a much larger latent world while the reader shows only a query-indexed local projection:

```text
available world >> displayed world
```

Publication still obeys:

```text
publish selected world != publish browsing history
```

and insertion into a global graph does not create truth, authority, applicability, or evidence payment.

## High-alpha implementation order

### P0 — exact source payment (paid)

One real Mabo judgment span is materialised through PG and executes `OpenSource` while leaving Why/applicability/truth unpaid.

### P1 — bounded SLR proposition-payment gate (paid)

Observed:

```text
mabo_proposition_payment: 4/4 passed
cargo test --workspace --no-fail-fast: passed
cargo clippy --workspace --all-targets -- -D warnings: passed
```

### P2 — Agda proposition-chain certification (paid)

`MaboRadicalTitlePropositionChainPaymentExact.agda` kernel-checks successfully.

### P3 — live PG -> typed SLR proposition payment

Bind the evaluator to the real persisted Mabo PNF/source-span rows and observe:

```text
OpenSource        = execute
Why               = executeBoundedWhy
ApplicabilityPaid = false
ClaimTruthPaid    = false
```

### P4 — portable Rust Reader ABI

Implement `sensiblaw-reader-model` only after the RED contract is observed. Project the live SLR payment into `ReaderDisposition` without adding transport, UI, PostgreSQL, or wgpu dependencies.

### P5 — Dioxus Semantic Reader

Consume the Rust ABI in `chboishabba/solfunmeme-dioxus`. Dioxus emits semantic intents and renders dispositions; it cannot manufacture payment state.

### P6 — ProofCone VisualIR -> wgpu

Project a paid `ExplanationCone` into renderer-neutral `ProofConeVisualIR`, then feed that to wgpu/WGSL. Ordinary charts migrate to the same wgpu-owned visualisation substrate over time; `dioxus-charts` is prototype/legacy, not target architecture.

### P7 — world growth

Use the paid proposition as a real node in the bounded Mabo world walk and scale toward the 100-hop latent world.

### P8 — federation/publication

Package selected fibres through Kant/eRDFa/IPFS and query through Zelph without publishing browsing history or promoting insertion into truth.

## Acceptance criterion

The flagship production path is paid when one real proposition crosses:

```text
judgment bytes
-> PostgreSQL
-> exact source span
-> SLR PNF/provenance payment
-> Agda-valid bounded explanation contract
-> typed Rust Reader ABI
-> Dioxus Why? ExecuteBoundedWhy
-> optional wgpu ProofCone projection
```

while preserving:

```text
bounded explanation != applicability
bounded explanation != claim truth
reader projection != semantic authority
Dioxus event != evidence payment
wgpu render state != semantic authority
```

That is the shortest end-to-end demonstration of the current SensibLaw/SLR/DASHI/Dioxus/wgpu architecture.
