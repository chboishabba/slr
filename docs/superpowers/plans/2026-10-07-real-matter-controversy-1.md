# REAL-MATTER-CONTROVERSY-1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist one reviewed adversarial legal controversy over a shared Matter and expose client/solicitor/judge projections without automated adjudication.

**Architecture:** Reuse existing generic chronology/contestation and reviewed-evidence/source ancestry. Add a narrow legal controversy persistence layer matching the Justice-Lee Agda owner, then project that same persisted object into Dioxus persona views. Formal parity imports the Justice-Lee, two-order, and REAL-MATTER review owners and pins the no-adjudication/non-collapse boundaries.

**Tech Stack:** Rust 1.78-compatible workspace, PostgreSQL, Dioxus 0.7, Agda.

**Spec:** `docs/superpowers/specs/2026-10-07-real-matter-controversy-design.md`

## Global Constraints

- Reuse persisted source/review/claim ancestry; do not build another source or review subsystem.
- Response opposition is typed, never Boolean negation.
- All persona projections consume the same persisted Matter controversy.
- Preserve normative-order identity; Crown municipal and Indigenous normative orders do not collapse.
- Machine operations may structure, compare and search; they may not determine credibility, ultimate fact, normative weight or final judgment.
- Mabo empirical materialisation must use genuine provider-backed persisted coordinates and fail closed when human-reviewed coordinates are absent.
- Existing workspace-wide clippy debt is not part of this tranche; new touched code should not introduce additional lint debt.

## Review Focus

1. Same-observation/different-claim replay must not let a response or residual bind to the wrong claim owner.
2. `admitOccurrenceDisputeCharacterisation` must remain distinguishable from denial and causation dispute in persistence/replay/UI.
3. Common-ground projection must not promote an admission to claim truth or legal applicability.
4. Cross-order propositions must display distinct normative orders and never infer priority/creation from recognition.
5. Reverse proof search must only return persisted obligations/residuals/query targets and never synthesize a merits outcome.

---

### Task 1: Durable legal controversy substrate

**Files:**
- Create: `crates/sl-core/src/legal_controversy.rs`
- Modify: `crates/sl-core/src/lib.rs`
- Create: `crates/sl-pg-source-store/src/legal_controversy_store.rs`
- Modify: `crates/sl-pg-source-store/src/workbench_projection.rs`
- Test: `crates/sl-pg-source-store/tests/legal_controversy_contract.rs`

**Interfaces:**
- Consumes: existing `PropositionRoot`, `ClaimLeaf`, review/source/reviewed-evidence refs and `DatabaseConfig`.
- Produces: `LegalPropositionFibre`, `TypedResponseEdge`, `LegalControversyResidual`, `LegalProofObligation`, `LegalControversyMatter`, `ReverseLegalProofSearch`, plus persist/load functions by stable ref.

- [ ] **Step 1: Write failing contract tests**
  - typed response modes are pairwise distinct for denial/characterisation/causation;
  - common-ground/admission carriers remain non-promoting;
  - normative-order mismatch is an explicit residual kind;
  - reverse search has no verdict/winner field;
  - public store API types are exported.

- [ ] **Step 2: Run RED**
  Run: `cargo test -p sensiblaw-pg-source-store --test legal_controversy_contract`
  Expected: FAIL because legal controversy public types/API do not exist.

- [ ] **Step 3: Implement core domain**
  Add exact enums matching Agda `PartyRole`, `LegalRole`, `EpistemicStatus`, `EvidenceKind`, `ResponseMode`, `DisagreementKind`, `ObligationKind`, `ProceduralGoal`, with `NormativeOrderMismatch` as the explicit two-order runtime extension. Domain validators reject empty refs, self-response edges, promotion flags, and missing provenance/review ancestry required by reviewed states.

- [ ] **Step 4: Implement PostgreSQL persistence/replay**
  Install a dedicated `legal_controversy` schema with immutable rows referencing existing semantic claim/proposition coordinates. Persist fibres, responses, residuals, obligations, matter membership and reverse-search receipts; replay exact enum and ownership coordinates; no JSON control ABI.

- [ ] **Step 5: Run GREEN and package suite**
  Run:
  `cargo test -p sensiblaw-pg-source-store --test legal_controversy_contract`
  `cargo test -p sensiblaw-pg-source-store`
  Expected: PASS; existing explicitly live PostgreSQL ignores may remain ignored.

- [ ] **Step 6: Commit**
  Commit message: `REAL-MATTER-CONTROVERSY-1: persist typed legal controversy`

### Task 2: Strict real-matter controversy materialiser and reverse search

**Files:**
- Create: `crates/sl-pg-source-store/src/real_matter_controversy.rs`
- Modify: `crates/sl-pg-source-store/src/workbench_projection.rs`
- Test: `crates/sl-pg-source-store/tests/real_matter_controversy_contract.rs`

**Interfaces:**
- Consumes: Task 1 persistence API plus existing `LegalEvidenceReviewDecision` / reviewed-evidence / legal-follow refs.
- Produces: `RealMatterControversyDraft`, `RealMatterControversyReceipt`, `materialize_real_matter_controversy`, `reverse_search_real_matter_controversy`.

- [ ] **Step 1: Write failing tests**
  Pin that the strict draft requires persisted reviewed coordinates for proposition/response support, a typed response mode, a residual question, and an explicit procedural goal; no function accepts a winner/credibility/final-judgment parameter.

- [ ] **Step 2: Run RED**
  Run: `cargo test -p sensiblaw-pg-source-store --test real_matter_controversy_contract`
  Expected: FAIL because materialiser/reverse-search API is absent.

- [ ] **Step 3: Implement strict materialiser**
  Reopen source/reviewed-evidence/legal-follow ownership, then persist one proposition, one typed response, one authority/support coordinate, one unresolved residual and one proof obligation/query target into Task 1. Reject historical-source relabelling or mismatched source/review owners.

- [ ] **Step 4: Implement reverse proof search**
  Select persisted open obligations/residuals for the requested procedural goal and return requested discriminator + target evidence query. Do not infer proof/admission/credibility or a merits result.

- [ ] **Step 5: Run GREEN and package suite**
  Run:
  `cargo test -p sensiblaw-pg-source-store --test real_matter_controversy_contract`
  `cargo test -p sensiblaw-pg-source-store`
  Expected: PASS.

- [ ] **Step 6: Commit**
  Commit message: `REAL-MATTER-CONTROVERSY-1: materialize reviewed controversy`

### Task 3: Dioxus shared-Matter persona projections

**Files:**
- Modify: `Cargo.toml` to pin SLR controversy head after Task 2.
- Create: `src/workbench/controversy.rs`
- Modify: `src/workbench/mod.rs`
- Modify: `src/workbench/matter.rs`
- Modify: `src/matter_ui.rs`
- Test: `tests/controversy_persona_projection.rs`

**Interfaces:**
- Consumes: persisted `LegalControversyMatter` + reverse-search reads from Task 2.
- Produces: `MatterControversyWorkspace` with `ClientControversyProjection`, `SolicitorControversyProjection`, `JudgeControversyProjection`; Dioxus controversy panel/persona switch.

- [ ] **Step 1: Write failing projection tests**
  Assert all personas share one `matter_ref`/controversy ref; client projection exposes reviewed-vs-candidate and normative-order labels; solicitor projection exposes typed response/residual/proof obligations/reverse search; judge projection isolates common ground and dispute categories but has no winner/credibility/final-judgment field.

- [ ] **Step 2: Run RED**
  Run: `cargo test --features production-data --test controversy_persona_projection`
  Expected: FAIL because controversy workspace projections are absent.

- [ ] **Step 3: Implement projection loader**
  Load persisted controversy by Matter ref through SLR APIs; construct persona-specific read-only projections without re-deriving review/adjudication state.

- [ ] **Step 4: Add Matter UI controversy panel**
  Add a `Controversy` panel and persona selector (`Client`, `Solicitor / Counsel`, `Court`). Display source/review/normative-order context, typed response categories, residuals and reverse-search targets. Include explicit UI copy that the projection does not determine who should win.

- [ ] **Step 5: Run GREEN and crate suite**
  Run:
  `cargo test --features production-data --test controversy_persona_projection`
  `cargo test --features production-data`
  Expected: PASS.

- [ ] **Step 6: Commit**
  Commit message: `REAL-MATTER-CONTROVERSY-1: add shared-Matter persona views`

### Task 4: Agda runtime parity and non-collapse

**Files:**
- Create: `DASHI/Law/SensibLawRealMatterControversyProjectionExact.agda`
- Create: `DASHI/Law/SensibLawRealMatterControversyEverything.agda`

**Interfaces:**
- Consumes: `JusticeLeeSensibLawAdversarialProofGraphBidiExact`, `SensibLawRealMatterReviewDecisionExact`, `SensibLawMaboTwoLegalOrderFibreExact`.
- Produces: formal runtime-parity boundary for persisted controversy and persona projections.

- [ ] **Step 1: Write formal owner source**
  Record same-Matter projection identity, typed response preservation, common-ground non-truth promotion, normative-order preservation, reverse-search non-adjudication, and persona projection fields.

- [ ] **Step 2: Add uninhabited collapse carriers**
  Include at least: typed response = Boolean negation; admission = truth; recognition = creation; client/solicitor/judge projections are different Matter objects; reverse search = final judgment; machine synthesis = credibility determination.

- [ ] **Step 3: Add focused aggregate**
  `SensibLawRealMatterControversyEverything` imports the donors and new owner.

- [ ] **Step 4: Verify preflight / typecheck boundary**
  Run: `agda DASHI/Law/SensibLawRealMatterControversyEverything.agda`
  Expected: either PASS or the already-known unrelated `TypedDependencyCore.agda:72 Not in scope: ℓ`; no new earlier error from the tranche.

- [ ] **Step 5: Commit**
  Commit message: `Formalise REAL-MATTER controversy persona boundary`

### Task 5: User-story/empirical acceptance surface

**Files:**
- Create: `docs/acceptance/REAL-MATTER-CONTROVERSY-1.md`
- Create: `crates/sl-pg-source-store/examples/itir_real_matter_controversy.rs`

**Interfaces:**
- Consumes: Task 2 strict APIs.
- Produces: operator-facing persisted-ref runner and acceptance story mapping.

- [ ] **Step 1: Write source-level CLI contract test or example compile assertion**
  Runner accepts persisted refs/coordinates only; no JSON fixture and no invented human review decision.

- [ ] **Step 2: Implement runner**
  Materialise/reopen the miniature controversy and print refs for proposition, typed response, residual, obligation, reverse-search query and persona-consumable matter ref. Fail closed if provider-backed review coordinates are absent/mismatched.

- [ ] **Step 3: Write acceptance document**
  Record explicit stories for controversy decomposition, backward proof search, common-ground isolation, normative-order preservation, reviewed legal meaning, acquisition impact and judicial reconstruction, with the real Mabo specimen as P0 acceptance.

- [ ] **Step 4: Verify package suite**
  Run: `cargo test -p sensiblaw-pg-source-store`
  Expected: PASS.

- [ ] **Step 5: Commit**
  Commit message: `docs: define REAL-MATTER controversy acceptance`
