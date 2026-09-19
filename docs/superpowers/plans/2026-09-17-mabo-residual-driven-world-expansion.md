# Mabo Residual-Driven World Expansion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an SLR controller that turns typed open PNF/world residuals into residual-sensitive producer selections and counts 100 novel reviewed world objects, then add Agda parity for the routing/admission firewalls.

**Architecture:** Reuse the existing `sl-consumer-residual`, `sl-proof-search-loop::frontier`, `sl-proof-search-scheduler`, `sl-legal-follow-plan`, provider, world, and PostgreSQL layers. Add one focused `world_expansion` orchestration module in `sl-proof-search-loop`; it owns only residual routing metadata, producer-lane ranking, disambiguation/admission accounting, and the 100-object completion criterion. Acquisition, PNF parsing, review, persistence, proof payment, and authority stay in their current owners.

**Tech Stack:** Rust workspace crates (`sensiblaw-proof-search-loop`, existing scheduler/legal/provider crates), Agda (`dashi_agda` Wikimedia/PNF/Ibrahim owners).

**Spec:** `docs/superpowers/specs/2026-09-17-mabo-residual-driven-world-expansion-design.md`

## Global Constraints

- `100` means novel admitted world-object cardinality, not hop depth.
- No new crawler, ontology, truth rank, or semantic-authority mechanism.
- OALC/governed legal gets first-refusal only for legal residuals and only while expected residual contraction remains positive/high-value.
- Existing PNF/world residual and Pareto machinery remains authoritative.
- First-link and raw adjacency remain candidate/navigation priors only.
- Candidate reachability does not imply review, admission, authority, applicability, proof payment, or truth.
- Duplicate object identities never increment the novel-object counter.
- Agda parity reuses existing Ibrahim, PNF, Wikimedia review, and source-boundary owners.

---

### Task 1: Residual-Sensitive World Expansion Core

**Files:**
- Create: `crates/sl-proof-search-loop/src/world_expansion.rs`
- Modify: `crates/sl-proof-search-loop/src/lib.rs`
- Test: unit tests in `world_expansion.rs`

**Interfaces:**
- Consumes: existing `frontier::ProofResidual` identity/producer coordinates and `sensiblaw_proof_search_scheduler::{ExecutionCostVector, ProofValueVector}` semantics.
- Produces: `ResidualClass`, `ProducerLane`, `DisambiguationOutcome`, `KnowledgeObjectKind`, `ExpansionCandidate`, `WorldExpansionPolicy`, `WorldExpansionLedger`, `select_expansion_candidate`, `review_admission`.

- [ ] **Step 1: Write failing tests**

Tests must assert:

```rust
#[test]
fn legal_residual_prefers_legal_lane_when_contraction_is_equal() { /* legal beats wiki only on equal contraction */ }

#[test]
fn higher_contraction_beats_domain_prior() { /* wiki can beat legal if it contracts more */ }

#[test]
fn identity_residual_prefers_wikidata_on_equal_contraction() { /* typed domain tie-break */ }

#[test]
fn ambiguous_wrong_type_duplicate_and_irrelevant_do_not_count() { /* fail closed */ }

#[test]
fn duplicate_object_identity_counts_once() { /* canonical object identity cardinality */ }

#[test]
fn target_is_object_cardinality_not_depth() { /* 100 admissions completes regardless of depth */ }
```

- [ ] **Step 2: Run focused test to verify RED**

Run:

```sh
cargo test -p sensiblaw-proof-search-loop world_expansion -- --nocapture
```

Expected: compile failure because `world_expansion` symbols do not exist.

- [ ] **Step 3: Implement minimal routing/admission types**

Implement the public enums/records named above. `select_expansion_candidate` must:

1. reject non-admissible candidates and candidates with zero expected residual contraction;
2. rank by `expected_residual_contraction` descending first;
3. on ties, rank producer-domain fit (`GovernedLegal` for `Legal`, `WikidataIdentity` for `Identity`, `WikipediaContext` for `Context`, `SourceSpecificProvenance` for `Provenance`);
4. then prefer higher provenance quality, same-object confidence, expected new-world value, lower acquisition/network cost, and deterministic `candidate_ref` ordering.

`review_admission` must increment counters only when review is explicit and disambiguation is one of: `SameObject`, `NewRelatedObject`, `NewSourceManifestation`, `NewConceptualParent`, `NewEvidentiarySource`; canonical object IDs are de-duplicated.

- [ ] **Step 4: Run focused tests to verify GREEN**

```sh
cargo test -p sensiblaw-proof-search-loop world_expansion -- --nocapture
```

Expected: all `world_expansion` tests pass.

- [ ] **Step 5: Commit**

```sh
git add crates/sl-proof-search-loop/src/world_expansion.rs crates/sl-proof-search-loop/src/lib.rs
git commit -m "feat(proof-search): add residual-driven world expansion controller"
```

---

### Task 2: Mabo 100-Object Policy Fixture

**Files:**
- Add tests to: `crates/sl-proof-search-loop/src/world_expansion.rs`
- Update: `docs/roadmap/mabo_context_federation_20260917.md`

**Interfaces:**
- Consumes: Task 1 world-expansion API.
- Produces: `mabo_world_expansion_policy()` and a fixture proving legal-first is residual-sensitive, plus roadmap state `P7c/P7d`.

- [ ] **Step 1: Write failing fixture tests**

Add tests that construct:

```rust
let policy = mabo_world_expansion_policy();
assert_eq!(policy.target_novel_objects, 100);
```

and candidate sets showing:

```text
Legal residual + equal contraction: governed legal selected.
Legal residual + stronger Wikidata contraction: Wikidata selected.
Identity residual + equal contraction: Wikidata selected.
Context residual + equal contraction: Wikipedia selected.
99 admitted objects: incomplete.
100 admitted objects: complete.
```

- [ ] **Step 2: Run focused test to verify RED**

```sh
cargo test -p sensiblaw-proof-search-loop world_expansion -- --nocapture
```

Expected: missing `mabo_world_expansion_policy` or fixture behavior failure.

- [ ] **Step 3: Implement Mabo policy**

`mabo_world_expansion_policy()` returns:

```rust
WorldExpansionPolicy {
    target_novel_objects: 100,
    minimum_expected_residual_contraction: 1,
}
```

No hard-coded 100-hop requirement appears in this policy.

- [ ] **Step 4: Update roadmap**

Record:

```text
P7a Persisted World Traversal Engine                  PAID
P7b Governed Source Federation                        ACTIVE/partially paid
P7c Residual-Driven World Expansion Controller        SOURCE-WRITTEN / tested when verified
P7d Mabo 100-Novel-Object Discovery Receipt           UNPAID LIVE RECEIPT
```

State explicitly that P7d requires 100 unique reviewed/admitted objects with discovery-parent/residual/provenance/disambiguation lineage; traversal depth is reported but is not the target.

- [ ] **Step 5: Verify and commit**

```sh
cargo test -p sensiblaw-proof-search-loop world_expansion -- --nocapture
cargo clippy -p sensiblaw-proof-search-loop --all-targets -- -D warnings
git add crates/sl-proof-search-loop/src/world_expansion.rs docs/roadmap/mabo_context_federation_20260917.md
git commit -m "feat(mabo): target 100 residual-justified world objects"
```

---

### Task 3: Agda Residual-Routing Parity

**Files (repository `chboishabba/dashi_agda`, branch `agent/mabo-reviewed-context-federation`):**
- Create: `DASHI/Wikimedia/MaboResidualDrivenWorldExpansionExact.agda`
- Create: `DASHI/Wikimedia/MaboResidualDrivenWorldExpansionValidation.agda`
- Modify: `DASHI/Wikimedia/Everything.agda`

**Interfaces:**
- Imports/reuses: `IbrahimKnowledgeCoverageRoadmapExact`, `IbrahimSnowballParetoFrontierExact`, `PredicateNormalFormWikipediaQidBridgeExact`, `SensibLawSourceUnitReviewHandoffExact`, `SensibLawBoundaryArtifactMorphismExact`, `MaboReviewedContextFederationExact`.
- Produces: a thin formal owner for target-cardinality, residual-indexed producer priority, legal-first-not-legal-only, and reachability/admission/non-promotion firewalls.

- [ ] **Step 1: Add validation import first**

`MaboResidualDrivenWorldExpansionValidation.agda` imports the future owner and checks exported witnesses for:

```text
targetNovelObjects = 100
hopDepthIsNotTarget = true
legalFirstIsResidualIndexed = true
legalFirstMeansLegalOnly = false
firstLinkControlsSemantics = false
reachabilityCreatesAdmission = false
qidCreatesAuthority = false
admissionCreatesClaimTruth = false
```

- [ ] **Step 2: Observe RED**

Run the repository's standard Agda validation command for the validation module. Expected: module-not-found before owner creation.

- [ ] **Step 3: Add thin owner**

The owner defines finite data for residual classes and producer lanes, a `WorldExpansionBoundary` record with the booleans above, target cardinality `100`, and empty-type firewall witnesses showing no constructors for forbidden collapses:

```text
TraversalDepthDeterminesNovelObjectCount
GlobalProducerOrderDeterminesResidualPriority
LegalFirstImpliesLegalOnly
ReachabilityEqualsAdmission
QidEqualsLegalAuthority
AdmissionEqualsClaimTruth
```

It imports existing canonical owners but does not re-formalise PNF, Ibrahim, Wikimedia, or legal authority.

- [ ] **Step 4: Export and verify**

Import the owner from `DASHI/Wikimedia/Everything.agda`, then run the focused Agda validation command.

- [ ] **Step 5: Commit**

```sh
git add DASHI/Wikimedia/MaboResidualDrivenWorldExpansionExact.agda DASHI/Wikimedia/MaboResidualDrivenWorldExpansionValidation.agda DASHI/Wikimedia/Everything.agda
git commit -m "feat(wikimedia): formalise residual-driven Mabo world expansion"
```

---

### Task 4: Verification and Status Reconciliation

**Files:**
- SLR roadmap/PR #20 body/comment as needed.
- Agda branch status/PR only if focused validation is observed.

- [ ] **Step 1: Run SLR verification**

```sh
cargo test -p sensiblaw-proof-search-loop world_expansion -- --nocapture
cargo test -p sensiblaw-pg-source-store --test context_federation
cargo clippy -p sensiblaw-proof-search-loop --all-targets -- -D warnings
```

- [ ] **Step 2: Run Agda focused validation**

Use the repo-standard command against `DASHI/Wikimedia/MaboResidualDrivenWorldExpansionValidation.agda`.

- [ ] **Step 3: Reconcile statuses**

Only mark P7c GREEN if the Rust focused test/clippy outputs are freshly observed. Only mark Agda theorem/kernel certification if the focused Agda command is freshly observed. Keep P7d UNPAID until a live run actually admits >=100 unique reviewed objects and emits lineage/provenance counts.
