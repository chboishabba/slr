# Shared Matter Controversy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist one typed adversarial controversy per Matter, derive backward proof-search/acquisition impact from it, and expose read-only client/solicitor/court projections from the same substrate.

**Architecture:** Add a narrow PostgreSQL controversy store in `sl-pg-source-store` that references existing reviewed evidence, REL residuals and source/review coordinates rather than recreating them. Add pure projection functions for reverse proof search and persona views. Dioxus consumes those projections; Agda mirrors the non-collapse/authority boundaries.

**Tech Stack:** Rust, PostgreSQL, existing SensibLaw core/REL/INV types, Dioxus, Agda.

**Spec:** `docs/superpowers/specs/2026-10-07-matter-controversy-design.md`

## Global Constraints

- One shared persisted Matter; no persona-specific backend copies.
- Typed responses are not Boolean negation.
- Normative-order context is preserved and never ranked/coerced by persistence or UI.
- Existing reviewed-evidence, REL, INV and legal-follow owners remain authoritative.
- No credibility, ultimate-fact, normative-weight, applicability, truth or final-judgment promotion.
- Mabo empirical coordinates must be new OALC/provider-backed coordinates, never relabelled Wikisource coordinates.

## Review Focus

- Same `matter_ref` but proposition/reference from another matter must fail closed.
- A typed response may not silently target itself or a proposition in another matter.
- A residual may reference an existing REL residual only if the comparison/obligation coordinates reopen exactly.
- Reverse proof search may describe acquisition impact but cannot create an INV obligation or access authority by itself.
- Persona projections must preserve source/review/normative-order coordinates and never mutate epistemic status.

---

### Task 1: Persisted controversy substrate

**Files:**
- Create: `crates/sl-pg-source-store/src/matter_controversy.rs`
- Modify: `crates/sl-pg-source-store/src/workbench_projection.rs`
- Test: `crates/sl-pg-source-store/tests/matter_controversy_contract.rs`

**Interfaces:**
- Consumes: existing `reviewed_evidence_coordinate`, persisted REL comparison/residual coordinates and matter refs.
- Produces: `MatterControversyDraft`, `PersistedMatterControversy`, proposition/response/residual/obligation coordinates, `persist_matter_controversy`, `load_matter_controversy`.

- [ ] Write public-contract tests pinning typed response modes, disagreement kinds, non-promotion flags and same-matter ownership.
- [ ] Verify RED: test fails because controversy API does not exist.
- [ ] Implement schema + immutable persistence/reopen checks; keep referenced source/review/REL coordinates external-owned.
- [ ] Verify focused test then full `sensiblaw-pg-source-store` suite.
- [ ] Commit.

### Task 2: Reverse proof search and acquisition impact

**Files:**
- Create: `crates/sl-pg-source-store/src/matter_reverse_proof.rs`
- Modify: `crates/sl-pg-source-store/src/workbench_projection.rs`
- Test: extend `crates/sl-pg-source-store/tests/matter_controversy_contract.rs`

**Interfaces:**
- Consumes: `PersistedMatterControversy`, existing REL residual refs and INV `PotentialReopeningCone` semantics.
- Produces: `MatterProceduralGoal`, `MatterReverseProofSearch`, `project_matter_reverse_proof_search`.

- [ ] Add failing tests for common-ground isolation, open obligations, residual-targeted query and no actual reopening/access authority.
- [ ] Verify RED.
- [ ] Implement deterministic projection from the persisted controversy only.
- [ ] Verify focused + full tests.
- [ ] Commit.

### Task 3: Persona projections

**Files:**
- Create: `crates/sl-pg-source-store/src/matter_controversy_projection.rs`
- Modify: `crates/sl-pg-source-store/src/workbench_projection.rs`
- Test: extend `crates/sl-pg-source-store/tests/matter_controversy_contract.rs`

**Interfaces:**
- Consumes: persisted controversy + reverse proof-search projection.
- Produces: `ClientMatterProjection`, `SolicitorMatterProjection`, `CourtMatterProjection`, `project_matter_personas`.

- [ ] Add failing tests proving all three projections share one controversy ref and preserve normative/source/review coordinates.
- [ ] Add failing judicial-boundary test proving no field represents winner/credibility/ultimate fact/normative weight.
- [ ] Implement read-only projections with persona-specific grouping only.
- [ ] Verify focused + full tests.
- [ ] Commit.

### Task 4: Dioxus Matter controversy surface

**Files:**
- Modify: `itir-dioxus/Cargo.toml`
- Create: `itir-dioxus/src/workbench/controversy.rs`
- Modify: `itir-dioxus/src/workbench/mod.rs`
- Modify: `itir-dioxus/src/workbench/matter.rs`
- Modify: `itir-dioxus/src/matter_ui.rs`
- Test: add/extend Dioxus workbench tests.

**Interfaces:**
- Consumes: SLR persona projections for a persisted `matter_ref`.
- Produces: one Matter tab/surface with Client, Solicitor/Counsel and Court views; all are projections of the same controversy ref.

- [ ] Write projection/load tests before UI code.
- [ ] Verify RED.
- [ ] Pin SLR branch/head and load optional controversy for the active MatterContext.
- [ ] Add a `Controversy` panel with persona switch and explicit non-adjudication copy.
- [ ] Verify Dioxus tests/build if available.
- [ ] Commit.

### Task 5: Agda runtime-parity owner

**Files:**
- Create: `DASHI/Law/SensibLawMatterControversyRuntimeParityExact.agda`
- Create: `DASHI/Law/SensibLawMatterControversyEverything.agda`

**Interfaces:**
- Consumes: `JusticeLeeSensibLawAdversarialProofGraphBidiExact`, Mabo two-order/review-decision owners, REL/INV formal owners where already available.
- Produces: explicit same-Matter, typed-response, reverse-search and persona-projection non-collapse boundaries.

- [ ] Encode runtime-parity records and uninhabited collapse types.
- [ ] Add focused aggregate.
- [ ] Run Agda preflight/typecheck if core universe migration permits; otherwise record exact external blocker.
- [ ] Commit.

### Task 6: Empirical Mabo fixture boundary

**Files:**
- Add a live ignored PostgreSQL test or example under `crates/sl-pg-source-store/tests/` / `examples/` using only persisted refs.

**Interfaces:**
- Consumes: real provider-backed Mabo reviewed evidence + one persisted typed response/support + REL residual.
- Produces: one persisted controversy ref and persona projection receipts.

- [ ] Add a live test that requires explicit environment/DB state and refuses historical Wikisource source refs for the OALC specimen.
- [ ] Assert proposition + typed response + support + unresolved residual + reverse target query exist.
- [ ] Assert client/solicitor/court projections share one controversy ref and remain non-promoting.
- [ ] Keep ignored until the actual human review/provider rows exist; do not fabricate them.
- [ ] Commit.