# Mabo Reader ABI + Dioxus/wgpu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the production Mabo reader path onto a shared Rust ABI consumed by Dioxus, with wgpu downstream for visualisation-heavy surfaces and Svelte retained only as a regression oracle.

**Architecture:** `sl-evidence-payment` remains the semantic/evidence evaluator. A new dependency-light `sensiblaw-reader-model` crate carries portable reader intents, source/proposition payments, dispositions, and bounded explanation cones. Dioxus consumes that crate directly; wgpu consumes a later visualisation projection and never determines evidence payment or semantic authority.

**Tech Stack:** Rust 1.78 workspace, PostgreSQL-backed SLR, Dioxus 0.7.3, later wgpu/WGSL.

**Spec:** `docs/superpowers/specs/2026-09-16-rust-reader-abi-dioxus-wgpu-design.md`

## Global Constraints

- PostgreSQL remains the sole active semantic persistence spine.
- Detached JSON/JS/Svelte objects are reference/regression only and are never re-ingested as semantic authority.
- Dioxus events, reader intents, SLR payment state, and PG evidence remain distinct types/ownership layers.
- The UI cannot set applicability, claim truth, support payment, or evidence-payment fields.
- wgpu receives already-typed projection objects and never promotes graph/render state into semantic authority.
- `BoundedWhyPaid != ApplicabilityPaid` and `BoundedWhyPaid != ClaimTruthPaid` remain invariant.
- No transport schema is introduced in this tranche; web serialization is a later projection.

---

### Task 1: Add the portable reader-model crate

**Files:**
- Create: `crates/sl-reader-model/Cargo.toml`
- Create: `crates/sl-reader-model/src/lib.rs`
- Create: `crates/sl-reader-model/tests/reader_model.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Produces: `SemanticRef`, `SourceRevisionRef`, `SpanRef`, `ResidualRef`, `ReaderIntent`, `CoordinateCoverage`, `SourcePayment`, `PropositionPayment`, `ExplanationCone`, `ReaderDisposition`.
- No dependency on PostgreSQL, Dioxus, wgpu, or SensibLaw Python.

- [ ] **Step 1: Write the failing ABI tests**

```rust
use sensiblaw_reader_model::{
    CoordinateCoverage, ExplanationCone, PropositionPayment, ReaderDisposition,
    ReaderIntent, SemanticRef, SourcePayment, SpanRef,
};

#[test]
fn why_executes_only_for_a_paid_bounded_chain() {
    let source = SourcePayment::paid(
        SemanticRef::new("mabo:proposition:radical-title-native-title"),
        "source-revision:mabo-hca23",
        SpanRef::new("span:mabo:brennan:radical-title:no-automatic-beneficial-ownership"),
    );
    let payment = PropositionPayment::bounded(
        source,
        vec!["observation:support".into()],
        CoordinateCoverage::Residualised("residual:qualifier".into()),
        CoordinateCoverage::Residualised("residual:defeater".into()),
        CoordinateCoverage::Residualised("residual:comparator".into()),
    );

    assert!(matches!(
        payment.resolve(ReaderIntent::WhyClaim),
        ReaderDisposition::ExecuteBoundedWhy(ExplanationCone { .. })
    ));
    assert!(!payment.applicability_paid());
    assert!(!payment.claim_truth_paid());
}

#[test]
fn source_can_execute_while_why_defers() {
    let payment = PropositionPayment::source_only(SourcePayment::paid(
        SemanticRef::new("mabo:proposition:radical-title-native-title"),
        "source-revision:mabo-hca23",
        SpanRef::new("span:mabo:brennan:radical-title:no-automatic-beneficial-ownership"),
    ));

    assert!(matches!(
        payment.resolve(ReaderIntent::OpenSource),
        ReaderDisposition::ExecuteSource { .. }
    ));
    assert!(matches!(
        payment.resolve(ReaderIntent::WhyClaim),
        ReaderDisposition::Defer(_)
    ));
}
```

- [ ] **Step 2: Run the tests and observe RED**

Run:

```sh
cargo test -p sensiblaw-reader-model --test reader_model
```

Expected: package/symbols missing.

- [ ] **Step 3: Implement the minimal typed ABI**

Use private fields for payment booleans where possible. Constructors enforce valid state transitions; there is no public constructor that accepts `claim_truth_paid=true` or `applicability_paid=true`.

- [ ] **Step 4: Run focused tests GREEN**

```sh
cargo test -p sensiblaw-reader-model --test reader_model
```

Expected: all reader-model tests pass.

- [ ] **Step 5: Commit**

```sh
git add Cargo.toml Cargo.lock crates/sl-reader-model
git commit -m "feat(reader): add portable typed reader ABI"
```

---

### Task 2: Project proposition-payment results into the reader ABI

**Files:**
- Modify: `crates/sl-evidence-payment/Cargo.toml`
- Modify: `crates/sl-evidence-payment/src/lib.rs`
- Create: `crates/sl-evidence-payment/tests/reader_projection.rs`

**Interfaces:**
- Consumes: existing `PropositionChainPayment` plus exact source revision/span identity.
- Produces: `sensiblaw_reader_model::PropositionPayment`.

- [ ] **Step 1: Write the failing projection test**

```rust
#[test]
fn paid_mabo_chain_projects_to_execute_bounded_why() {
    let chain = paid_mabo_chain_fixture();
    let reader = project_reader_payment(
        "source-revision:mabo-hca23",
        "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership",
        &chain,
    );

    assert!(matches!(
        reader.resolve(ReaderIntent::WhyClaim),
        ReaderDisposition::ExecuteBoundedWhy(_)
    ));
    assert!(!reader.applicability_paid());
    assert!(!reader.claim_truth_paid());
}
```

- [ ] **Step 2: Run RED**

```sh
cargo test -p sensiblaw-evidence-payment --test reader_projection
```

Expected: `project_reader_payment` missing.

- [ ] **Step 3: Implement the minimal projection**

Map the existing source/proposition payment into the ABI. Preserve support observation refs and explicit qualifier/defeater/comparator residuals. Do not infer applicability or truth.

- [ ] **Step 4: Run GREEN plus existing Mabo payment tests**

```sh
cargo test -p sensiblaw-evidence-payment --test reader_projection
cargo test -p sensiblaw-evidence-payment --test mabo_proposition_payment
```

- [ ] **Step 5: Commit**

```sh
git add crates/sl-evidence-payment Cargo.lock
git commit -m "feat(reader): project evidence payment into reader ABI"
```

---

### Task 3: Bind the live PG Mabo coordinate to the typed reader result

**Files:**
- Modify: `crates/sl-pg-source-store/Cargo.toml`
- Create: `crates/sl-pg-source-store/src/mabo_reader.rs`
- Modify: `crates/sl-pg-source-store/src/lib.rs`
- Create: `crates/sl-pg-source-store/tests/mabo_reader_projection.rs`

**Interfaces:**
- Consumes: exact persisted source/span, PNF support observation refs, graph/source-span provenance refs, explicit role residuals.
- Produces: a `PropositionPayment` through the existing evidence-payment evaluator and Task 2 projection.

- [ ] **Step 1: Write a failing row-projection test**

The fixture must represent the real paid coordinate:

```text
mabo:proposition:radical-title-native-title
span:mabo:brennan:radical-title:no-automatic-beneficial-ownership
```

and assert:

```rust
assert!(matches!(
    result.resolve(ReaderIntent::WhyClaim),
    ReaderDisposition::ExecuteBoundedWhy(_)
));
assert!(!result.applicability_paid());
assert!(!result.claim_truth_paid());
```

- [ ] **Step 2: Run RED**

```sh
cargo test -p sensiblaw-pg-source-store --test mabo_reader_projection
```

Expected: Mabo reader row adapter missing.

- [ ] **Step 3: Implement the narrow read adapter**

Query/reconstruct only the coordinates already required by the existing proposition evaluator. No new proof-store schema. Fail closed if the exact span, PNF observation provenance, or independent graph/source-span weld is absent.

- [ ] **Step 4: Run focused GREEN**

```sh
cargo test -p sensiblaw-pg-source-store --test mabo_reader_projection
```

- [ ] **Step 5: Run the live TrueNAS transaction/query when credentials are available**

Observe and record:

```text
OpenSource          = ExecuteSource
Why                 = ExecuteBoundedWhy
ApplicabilityPaid   = false
ClaimTruthPaid      = false
```

A fixture-only pass does not pay this live receipt.

- [ ] **Step 6: Commit**

```sh
git add crates/sl-pg-source-store Cargo.lock
git commit -m "feat(reader): bind live Mabo PG payment to reader ABI"
```

---

### Task 4: Add the Dioxus production consumer

**Repository:** `chboishabba/solfunmeme-dioxus`

**Files:**
- Modify: `Cargo.toml`
- Create: `src/semantic_reader/mod.rs`
- Create: `src/semantic_reader/mabo.rs`
- Modify: `src/main.rs`
- Modify: `src/playground/app.rs`

**Interfaces:**
- Consumes: `sensiblaw-reader_model::{ReaderIntent, ReaderDisposition, PropositionPayment}`.
- Emits: only `ReaderIntent`; never constructs semantic payment state.

- [ ] **Step 1: Add a failing Rust unit/component-state test**

Test a pure helper first: `render_state(&PropositionPayment, ReaderIntent) -> ReaderViewState` must show `Source` executable and `Why` executable/deferred according to the typed payment.

- [ ] **Step 2: Run RED**

```sh
cargo test semantic_reader
```

- [ ] **Step 3: Add the SLR reader-model git dependency**

Use the SLR branch/ref carrying `sensiblaw-reader-model`; do not duplicate the structs locally.

- [ ] **Step 4: Implement a minimal Mabo Semantic Reader component**

The Dioxus component owns ordinary UI only: explanation text, Source/Why buttons, residual labels, provenance disclosure. Button events lower to `ReaderIntent`; the resulting `ReaderDisposition` controls view state.

- [ ] **Step 5: Run Rust/Dioxus checks**

```sh
cargo test
cargo check --target wasm32-unknown-unknown
```

- [ ] **Step 6: Commit**

```sh
git add Cargo.toml Cargo.lock src
 git commit -m "feat(reader): consume typed SLR reader ABI in Dioxus"
```

---

### Task 5: Establish the wgpu projection boundary without building the full renderer

**Repository:** `chboishabba/solfunmeme-dioxus`

**Files:**
- Create: `src/visualisation/mod.rs`
- Create: `src/visualisation/proof_cone.rs`
- Test: module unit tests

**Interfaces:**
- Consumes: `ExplanationCone` and typed semantic IDs.
- Produces: `ProofConeVisualIR` only.
- Does not determine payment, applicability, truth, or provenance validity.

- [ ] **Step 1: Write RED test**

Assert that one `ExplanationCone` deterministically projects to visual nodes/edges while preserving semantic refs and residual status.

- [ ] **Step 2: Implement `ProofConeVisualIR`**

Keep it renderer-neutral. No wgpu dependency yet.

- [ ] **Step 3: Run GREEN**

```sh
cargo test visualisation
```

- [ ] **Step 4: Commit**

```sh
git add src/visualisation
git commit -m "feat(viz): add typed proof-cone visual IR"
```

---

### Task 6: Update the Mabo roadmap around the production Dioxus/wgpu path

**Files:**
- Modify: `docs/roadmap/mabo_proof_explanation_profile_20260915.md`

**Required revised order:**

```text
P0 exact source payment                         DONE
P1 bounded SLR proposition-payment gate        DONE
P2 Agda proposition-chain kernel certification DONE
P3 live PG -> typed SLR proposition payment    NEXT/receipt gate
P4 shared Rust Reader ABI                      implementation tranche
P5 Dioxus Semantic Reader                      production UI
P6 ProofCone VisualIR -> wgpu renderer          production visualisation
P7 100-hop latent Mabo world                    downstream/parallel
P8 selected Kant/eRDFa/IPFS publication fibre  later
```

Remove Svelte from the production critical path and state explicitly that it remains a regression oracle only.

- [ ] **Step 1: Update roadmap text and acceptance criteria**
- [ ] **Step 2: Check for contradictory Svelte-first wording**
- [ ] **Step 3: Commit**

```sh
git add docs/roadmap/mabo_proof_explanation_profile_20260915.md
git commit -m "docs: move Mabo reader roadmap to Dioxus and wgpu"
```

---

### Task 7: Full verification

- [ ] **Step 1: SLR workspace tests**

```sh
cargo test --workspace --no-fail-fast
```

- [ ] **Step 2: SLR strict Clippy**

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 3: Dioxus tests/check**

```sh
cargo test
cargo check --target wasm32-unknown-unknown
```

- [ ] **Step 4: Confirm firewalls in observed results**

```text
Why = ExecuteBoundedWhy
ApplicabilityPaid = false
ClaimTruthPaid = false
Dioxus does not construct payment state
ProofConeVisualIR does not promote authority
```

- [ ] **Step 5: Record only observed receipts in the roadmap/PR descriptions**
