# SLR roadmap: thin Mabo proof/explanation profile

Date: 2026-09-15
Updated: 2026-09-16

## Purpose

Define the next legal flagship around the current production split rather than the legacy Python/Svelte module graph.

The active ownership rule is:

```text
DASHI/Agda = golden semantic / proof contract
SLR/Rust   = production parser / residual / evidence-payment / recurrence engine
PostgreSQL = sole active semantic persistence spine
ITIR/Svelte + SensibLaw Python = legacy/reference and regression consumers unless a boundary is explicitly retained
```

Detached JSON is presentation/export only. It must not be re-ingested as semantic, world, source-follow, or evidence-payment authority.

The profile is deliberately thin:

```text
exact persisted source/span
-> Rust SLR consumer-relative proposition payment
-> bounded Why?/Explanation-Cone receipt
-> projection to a human reader
```

SLR owns deterministic compilation, consumer-relative residual production, evidence-payment execution, bounded recurrence, and provenance-preserving runtime receipts.

DASHI owns the golden laws that say when those receipts are sufficient for the declared consumer query and which promotions remain impossible.

PostgreSQL owns the durable source/revision/span and semantic-materialisation coordinates required by the runtime. The UI consumes typed results; it is not a semantic owner.

## Why Mabo remains the flagship

The point is not to globally "prove Mabo". The first complete specimen should take one concrete proposition and cross every active layer:

```text
judgment bytes
-> PostgreSQL canonical source + exact span
-> SLR PNF/proposition support
-> independent source-span provenance weld
-> support + qualifier/defeater/comparator coverage
-> bounded Why? payment
-> human-readable explanation
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

So source acquisition/payment is no longer the architectural frontier. Proposition-chain payment is.

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

This lets the reader open an honest bounded cone that says which limiting/contrary/comparison coordinates remain unresolved.

The consumer gate is therefore approximately:

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
```

but:

```text
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

## Current TDD / formal state

### Rust SLR

Draft PR #14:

```text
agent/mabo-radical-title-proposition-payment
```

started as a RED contract and now includes the minimal typed Rust evaluator in:

```text
crates/sl-evidence-payment/tests/mabo_proposition_payment.rs
```

The intended tests require:

1. exact source + provenance-welded support + explicit residuals for qualifier/defeater/comparator -> bounded Why paid;
2. PNF support without independent graph/source-span provenance -> Why unpaid;
3. exact source alone -> Why unpaid;
4. a paid qualifier may replace its explicit residual without paying applicability/truth.

The missing proposition-payment symbols were observed RED, then the focused contract passed 4/4, `cargo test --workspace --no-fail-fast` passed locally, and `cargo clippy --workspace --all-targets -- -D warnings` passed after small workspace lint repairs. The evaluator is intentionally read-side only: it consumes typed source/span and PNF coordinates, and does not add network I/O, JSON ingestion, UI code, applicability payment, or truth promotion.

### DASHI/Agda

Draft PR #963:

```text
agent/mabo-radical-title-proposition-chain-parity
```

adds:

```text
DASHI/Interop/MaboRadicalTitlePropositionChainPaymentExact.agda
```

as a successor to the kernel-certified exact-source owner.

The owner formalises the bounded proposition payment and all non-promotion firewalls above. Its exact-head Agda kernel receipt is observed.

## PostgreSQL boundary

PostgreSQL remains the sole active semantic persistence spine.

The relevant retained reference schema is the existing SensibLaw PostgreSQL design:

```text
legal_ir.semantic_build
-> legal_ir.projection
-> legal_ir.observation
-> legal_ir.graph_revision
```

The production Rust implementation should query/consume equivalent typed PG coordinates, not ingest detached JSON or port Python dataclasses one-for-one.

The legacy schema remains useful because it already identifies the required shapes:

- PNF factor/revision refs;
- structural signature and predicate;
- role bindings;
- qualifier/wrapper state;
- provenance refs;
- residual refs;
- graph source-span refs;
- jurisdiction/time/authorship/review state.

Do not add a second proof-store schema merely to pay the Mabo reader consumer. Add a new schema only if the existing PG coordinates provably cannot represent a required production obligation.

## UI / reader projection contract

The human-facing rule remains:

```text
argument-first
-> graph-second
-> provenance-on-demand
```

The UI is a projection consumer over the SLR/Agda payment state.

A paid bounded `Why?` means only that the declared explanation cone is adequate to display. It does not mean the court's proposition is globally proven or applicable to a new user's facts.

The reader should initially expose:

```text
What changed?
Why does this step follow here?
What exact source supports it?
What limits / contradicts / compares with it?
What remains unresolved?
```

and reopen deeper PNF, source, Wiki/Wikidata, proof, and world-bucket detail only on demand.

Wiki/Wikidata remain context/navigation coordinates:

```text
QID != applicability
Wikipedia != legal authority
context link != evidence payment
```

## World-bucket relation

The 100-hop Mabo world bucket is now downstream/parallel substrate, not the next blocking proof task.

The correct order is:

```text
exact source payment
-> bounded proposition-chain payment
-> live Why?/ProofCone
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

### P0 — Rust RED (paid)

Run the exact PR #14 contract with a real Rust toolchain:

```sh
cargo test -p sensiblaw-evidence-payment --test mabo_proposition_payment
```

Observed RED: unresolved proposition-payment symbols / missing implementation.

### P1 — minimal Rust GREEN (paid)

Implement only the read-side proposition-payment evaluator needed by those tests, using typed inputs corresponding to retained PG/PNF/source-span coordinates. Do not add network I/O, UI code, JSON ingestion, or legal truth promotion.

Observed: the focused contract passes 4/4, `cargo test --workspace --no-fail-fast` passes, and `cargo clippy --workspace --all-targets -- -D warnings` passes.

The normal validation commands remain:

```sh
cargo test -p sensiblaw-evidence-payment --test mabo_proposition_payment
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

### P2 — Agda exact-head kernel receipt

Kernel-check:

```text
DASHI/Interop/MaboRadicalTitlePropositionChainPaymentExact.agda
```

and update certification coordinates only from observed output.

### P3 — live PG -> Rust SLR payment

Bind the evaluator to the actual PG-backed PNF/source-span rows for the already-materialised Mabo radical-title coordinate and observe:

```text
OpenSource = execute
Why        = executeBoundedWhy
ApplicabilityPaid = false
ClaimTruthPaid    = false
```

### P4 — human Explanation Cone

Project that same typed payment into the reader so a normal user can click `Why?` and see the bounded support/limitation/comparator/residual structure with exact source reopening.

### P5 — world growth / federation

Use the proven proposition as a real semantic node in the bounded Mabo world walk, then later package only selected fibres through Kant/eRDFa/IPFS and query through Zelph.

## Acceptance criterion

The flagship is paid when one real proposition crosses:

```text
judgment bytes
-> PostgreSQL
-> exact source span
-> SLR PNF/provenance payment
-> Agda-valid bounded explanation contract
-> Why? Execute
-> human-readable Explanation Cone
```

while still preserving:

```text
bounded explanation != applicability
bounded explanation != claim truth
reader projection != semantic authority
```

That is the shortest end-to-end demonstration of the current SensibLaw/SLR/DASHI architecture.
