# Semantic Reader Rust production ledger — 2026-09-16

This is the current SLR production-state ledger for the Semantic Reader path.
The historical P0–P8 identifiers are retained for provenance, but the dependency
graph below is authoritative.

Updated: 2026-09-16 — the SLR reader-runtime substrate is paid through the live
PostgreSQL weld, portable Reader ABI, bounded ExplanationCone runtime, and
100-hop latent-world projection. The active product frontier has moved downstream
to Dioxus, then wgpu.

## Production ownership

```text
DASHI / Agda       golden semantic/proof contract
SLR / Rust         production parsing, residuals, payment, recurrence, typed read/runtime state
PostgreSQL         sole active semantic persistence spine
Dioxus / Rust      production reader + ordinary interaction surface
wgpu / WGSL        accelerated graph/chart/proof/world rendering
SensibLaw Python   reference + PG regression
ITIR / Svelte      reference/regression prototype only
```

The production path is:

```text
governed source providers / durable AU legal corpus
-> canonical PostgreSQL source/span
-> CandidatePNF
-> explicit review/admission
-> ReviewedPNFRevision
-> legal_ir materialisation
-> PropositionChainPayment
-> PropositionPayment / Reader ABI
-> bounded ExplanationCone / latent-world projection
-> Dioxus
-> optional wgpu visual projection
```

No JSON/TSV/Svelte/Python object is semantic identity in this path.

## Current PR / owner ledger

```text
SLR #14  bounded proposition calculus                         PAID
SLR #15  portable Rust Reader ABI                            PAID / CANONICAL ABI OWNER
SLR #18  production Semantic Reader runtime                  PAID / CANONICAL PG->PAYMENT RUNTIME OWNER
SLR #19  deterministic 100-hop latent-world projection       PAID / CANONICAL WORLD-PROJECTION OWNER
DASHI #963 bounded proposition-chain parity                  PAID
DASHI #982 legal_ir materialisation parity                   FORMAL PARENT
Dioxus upstream #18 native Rust/backend mediation            MERGED CONSUMER PARENT
Dioxus agent/mabo-semantic-reader-v1                         DOWNSTREAM P5 CONSUMER
wgpu ProofCone                                                DOWNSTREAM P6 CONSUMER
```

The SLR production chain is therefore:

```text
#15 Reader ABI
-> #18 live PG/payment/runtime
-> #19 latent-world projection
-> Dioxus
-> wgpu
```

## Flagship Mabo coordinate

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

The paid live path is:

```text
existing Brennan span
-> ReviewedPnfRevision
-> ReviewedPropositionSupport
-> legal_ir semantic materialisation
-> PropositionChainPayment
-> PropositionPayment / Reader ABI
-> ExplanationCone / latent-world read projection
```

Observed runtime semantics retain:

```text
exactSourcePaid       = true
propositionChainPaid  = true
OpenSource            = ExecuteSource
Why                   = ExecuteBoundedWhy
ApplicabilityPaid     = false
ClaimTruthPaid        = false
```

## SLR completion surface

The reader-path responsibilities inside SLR are now:

```text
PG evidence
-> bounded proposition payment
-> portable Reader ABI
-> bounded ExplanationCone/runtime projection
-> latent-world projection
```

Current SLR state:

```text
S0 source/PG acquisition + canonical persistence       PAID
S1 proposition payment calculus                        PAID
S2 live PG -> payment runtime                          PAID
S3 portable Reader ABI                                 PAID
S4 bounded ExplanationCone/runtime projection          PAID
S5 100-hop latent-world substrate                      PAID
S6 stabilise/freeze production consumer boundary       ACTIVE SLR PRIORITY
S7 publication/federation support                      ONLY WHEN P8 REQUIRES IT
```

This means P5/P6 are no longer SLR milestones:

```text
SLR terminates at typed semantic/runtime state.
Dioxus owns P5.
wgpu owns P6.
```

## S6 — production consumer-boundary freeze

S6 is a stabilisation tranche, not a new semantics tranche.

Completion criteria:

```text
#18 remains canonical PG -> proposition-payment/runtime owner
#15 remains canonical portable Reader ABI owner
#19 remains canonical latent-world projection owner
Dioxus consumes those typed surfaces without copying payment logic
applicability_paid remains non-promoted across every projection
claim_truth_paid remains non-promoted across every projection
no PostgreSQL DTO leaks into the Reader ABI
no Dioxus/wgpu type leaks backward into SLR
compatibility/regression tests pin these boundaries
```

Architectural firewall:

```text
storage row
!= payment
!= reader disposition
!= latent-world projection
!= visual representation
```

No new SLR PR should be opened merely because Dioxus or wgpu work is next.
A new SLR tranche is justified only by one of:

1. a real missing typed coordinate exposed by the Dioxus consumer;
2. a regression/compatibility failure at the frozen consumer boundary;
3. a P8 federation/publication producer that cannot be expressed by the current runtime state.

## Proposition-payment contract

Support cannot be residualised. It requires exact source payment plus retained
reviewed PNF identity/provenance and an independent graph-span weld.
Qualifier, defeater, and comparator may each be paid or explicitly residualised
for the bounded reader query.

```text
WhyPaid =
  ExactSourcePaid
  && SupportPaid
  && Covered(Qualifier)
  && Covered(Defeater)
  && Covered(Comparator)
```

Firewalls remain:

```text
SupportPaid != ExplicitResidual(Support)
BoundedWhyPaid != ApplicabilityPaid
BoundedWhyPaid != ClaimTruthPaid
ExplicitResidual != PropositionFalse
```

## Adaptive Explanation Cone

Canonical SLR owner:

```text
crates/sl-reader-model/src/semantic_runtime.rs
```

Rule:

```text
Keep(x) = mandatory(x)
       || distance(x) <= base_depth
       || elucidatory_score(x) >= threshold
```

The result is deterministic, bounded, proposition-generic, and downstream of
payment. Mandatory payment/provenance coordinates precede optional explanatory
coordinates.

## Automatic acquisition/retry

Canonical recurrence:

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

## Latent world

The bounded/100-hop world is a read projection over existing persisted semantic
relations, not a new semantic database.

Canonical owner lineage:

```text
#18 bounded latent-world read projection
-> #19 deterministic 100-hop scale projection
```

The world projection retains explicit budgets, frontier/truncation residuals,
visited refs, deterministic edges, and per-edge provenance. It never promotes
applicability, claim truth, or semantic authority.

## Current-state summary using historical P identifiers

```text
P0    exact source/span                                    PAID
P1    bounded SLR proposition calculus                     PAID
P2    Agda proposition-chain contract                      PAID
P3    live PG -> SLR proposition payment/runtime           PAID
P4    portable Rust Reader ABI                             PAID
P7    deterministic bounded/100-hop latent-world substrate PAID
P5    Dioxus Semantic Reader                               DOWNSTREAM ACTIVE CONSUMER
P6    wgpu VisualIR / ProofCone                            DOWNSTREAM AFTER P5
P8    selected federation/publication fibre                LATER
```

For current execution planning, read this as:

```text
(P3 + P4 + P7) SLR production substrate paid
-> P5 Dioxus
-> P6 wgpu
-> P8 selected publication/federation
```

The historical numbering is provenance only; it is not a required execution
order.

## Implementation priority assessment

### Priority 0 — freeze SLR consumer ABI

Do now, but keep it small:

- pin regression tests for `PropositionPayment`, `ReaderDisposition`,
  `ExplanationCone`, and latent-world projection;
- assert non-promotion of applicability/truth across every public projection;
- assert storage DTOs do not appear in `sensiblaw-reader-model` public types;
- assert Dioxus/wgpu dependencies are absent from the SLR runtime crates;
- document the canonical owner PRs (#15/#18/#19) and compatibility expectations.

This is the only proactive SLR implementation priority.

### Priority 1 — follow the Dioxus consumer

Do not add semantics pre-emptively. If P5 exposes a missing typed coordinate,
add the smallest consumer-driven SLR extension and a regression proving why it
is required.

### Priority 2 — federation/publication only on demand

P8 may require a new producer. Do not design it until the selected publication
fibre names a concrete missing output from the frozen runtime state.

### Explicitly not priorities

```text
more Svelte/JS reader work
another PG/world schema
Dioxus-side payment logic
wgpu-side semantic/payment logic
new generic reader planner
new proposition/payment ontology
```

## End-state criterion

From the SLR perspective, the target state is now:

```text
reader-runtime substrate complete;
consumer ABI frozen;
follow Dioxus;
reopen SLR only on typed evidence.
```

Hard firewalls:

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
