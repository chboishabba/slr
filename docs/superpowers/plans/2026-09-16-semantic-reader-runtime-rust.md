# Semantic Reader Runtime Rust Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement roadmap items 1, 2, 6, 7, 10, 13, 14, and 15 in the existing Rust runtime without introducing a second semantic owner or production Svelte path.

**Architecture:** Reuse the existing SLR crates. PostgreSQL owns persisted source/PNF/legal-IR state; `sensiblaw-evidence-payment` owns proposition payment; `sensiblaw-reader-model` owns portable read projections; `sensiblaw-proof-search-loop` owns acquisition/retry recurrence. Dioxus/wgpu stay downstream.

**Tech Stack:** Rust 1.78+, PostgreSQL, existing SLR workspace crates, Dioxus Reader ABI downstream.

**Spec:** `docs/roadmap/mabo_proof_explanation_profile_20260915.md`

## Global Constraints

- PostgreSQL is the sole active semantic persistence spine.
- CandidatePNF != ReviewedPNF != PropositionSupport != Applicability != ClaimTruth.
- Storage must not assign consumer proof roles or semantic payment.
- Support cannot be residualised; qualifier/defeater/comparator may remain explicit query-relative residuals.
- Reader projection does not create semantic authority.
- Acquisition does not itself pay a residual; payment is established only after persistence and re-evaluation.
- Wikipedia/Wikidata/history context never becomes legal authority or evidence payment by navigation alone.
- Core APIs must be proposition/material-generic; Mabo is a flagship fixture only.
- Svelte remains reference/regression only; no production dependency on it.

---

### Task 1: CandidatePNF and reviewed PNF persistence

**Files:**
- Create: `crates/sl-pg-source-store/src/candidate_pnf.rs`
- Create: `crates/sl-pg-source-store/src/reviewed_pnf.rs`
- Modify: `crates/sl-pg-source-store/src/lib.rs`
- Test: `crates/sl-pg-source-store/tests/mabo_pnf_candidate_materialization.rs`
- Test: `crates/sl-pg-source-store/tests/reviewed_pnf_materialization.rs`

**Interfaces:**
- Produces `ExactSourceSpan`, `CandidatePnfProducer`, `CandidatePnfBatch`, `ReviewedPnfRevision`, `materialize_reviewed_pnf_revision`.
- Reviewed persistence writes only existing `algebra.factor`, `algebra.factor_revision`, `pnf.graph`, and `pnf.graph_factor_revision` rows.

- [ ] Observe existing CandidatePNF RED.
- [ ] Implement minimal parser-neutral candidate carrier and temporary spaCy TSV compatibility adapter.
- [ ] Add reviewed-PNF persistence RED requiring an explicit review receipt.
- [ ] Implement idempotent reviewed-PNF persistence with conflict checks and no support/truth promotion.
- [ ] Run focused tests, workspace tests, Clippy.

### Task 2: Generic live PG -> payment -> Reader ABI weld

**Files:**
- Modify: `crates/sl-pg-source-store/src/proposition_rows.rs`
- Modify: `crates/sl-pg-source-store/src/lib.rs`
- Modify: `crates/sl-evidence-payment/Cargo.toml`
- Modify: `crates/sl-evidence-payment/src/lib.rs`
- Test: `crates/sl-evidence-payment/tests/reader_payment_projection.rs`

**Interfaces:**
- Produces `load_proposition_rows` with `load_mabo_proposition_rows` retained as compatibility alias.
- Produces `project_reader_payment(chain, source_revision_ref, span_ref, role_residuals)`.

- [ ] Write RED proving a paid chain projects to `PropositionPayment` without promoting applicability/truth.
- [ ] Generalise storage loader naming without changing semantics.
- [ ] Implement projector using original role-tagged residuals so qualifier/defeater/comparator are never reconstructed from a flattened list.
- [ ] Run focused tests, workspace tests, Clippy.

### Task 3: Adaptive explanation cone + context bundle

**Files:**
- Modify: `crates/sl-reader-model/src/lib.rs`
- Test: `crates/sl-reader-model/tests/adaptive_cone.rs`
- Test: `crates/sl-reader-model/tests/context_navigation.rs`

**Interfaces:**
- Produces `ConeCandidate`, `ConePolicy`, `AdaptiveExplanationCone`, `select_adaptive_cone`.
- Produces `ContextKind`, `ContextAuthority`, `ContextLink`, `ContextBundle`.

- [ ] RED: near nodes kept, far-high-elucidatory nodes kept, far-low nodes excluded, bounded budget deterministic.
- [ ] GREEN: implement exact `(distance <= base_depth) || (elucidatory_score >= threshold)` rule with mandatory provenance-bearing nodes admitted before optional nodes.
- [ ] RED/GREEN context firewalls: exact source may be primary authority; Wikipedia/Wikidata/history cannot pay evidence or applicability.

### Task 4: Defer -> acquisition/retry adapter

**Files:**
- Modify: `crates/sl-proof-search-loop/Cargo.toml`
- Create: `crates/sl-proof-search-loop/src/reader_retry.rs`
- Modify: `crates/sl-proof-search-loop/src/lib.rs`
- Test: `crates/sl-proof-search-loop/tests/reader_retry.rs`

**Interfaces:**
- Consumes `ReaderDisposition::Defer`, existing `ProofFrontier`, hypothesis/provider machinery.
- Produces `ReaderRetryPlan` and `retry_after_extension`.

- [ ] RED: an exact-authority-span residual routes to the existing primary-authority producer; proposition-support routes to reviewed PNF/support producer; qualifier/defeater/comparator retain distinct producer demands.
- [ ] GREEN: compile reader residuals into open `ProofResidual`s without executing network I/O or declaring payment.
- [ ] RED/GREEN: after an external world extension, re-evaluation is mandatory before `Execute`; acquisition receipt alone cannot close the reader.

### Task 5: Generic latent-world read projection

**Files:**
- Modify: `crates/sl-reader-model/src/lib.rs`
- Test: `crates/sl-reader-model/tests/latent_world.rs`

**Interfaces:**
- Produces `WorldNode`, `WorldEdge`, `WorldEdgeKind`, `ReaderWorldProjection`, `bounded_neighbourhood`.

- [ ] RED: a chain longer than 100 hops is truncated at the requested hop cap and node budget.
- [ ] GREEN: deterministic BFS projection over supplied typed edges; no persistence or authority semantics.
- [ ] Prove graph/world visibility is a read projection only.

### Task 6: Five-stage Mabo registry and arbitrary-material generalisation

**Files:**
- Create: `crates/sl-reader-model/src/specimen.rs`
- Modify: `crates/sl-reader-model/src/lib.rs`
- Test: `crates/sl-reader-model/tests/specimen_registry.rs`

**Interfaces:**
- Produces generic `ReaderPropositionSpec`, `ReadingRole`, `SourceCoordinate` and a Mabo five-stage registry.
- Radical-title proposition uses the exact paid coordinate; remaining stages retain explicit source/proof residuals until paid.

- [ ] RED: five bounded Mabo roles are stable and independently reopenable; only paid coordinates may claim executable source readiness.
- [ ] GREEN: implement generic registry with no Mabo branching in core resolution.
- [ ] RED/GREEN: a non-Mabo ordinary/research specimen uses the same types and cone/world functions.

### Task 7: Bookkeeping / PR ledger

**Files:**
- Modify: `docs/roadmap/mabo_proof_explanation_profile_20260915.md`

- [ ] Record latest PR audit (#14/#15 paid, #17 current, DASHI #982 parity, Dioxus P5 downstream).
- [ ] Add status table for requested items 1/2/6/7/10/13/14/15 with exact Rust owner and validation receipt.
- [ ] Keep Dioxus/wgpu explicitly downstream and Svelte reference-only.
