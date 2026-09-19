# Mabo Adaptive Hop/Re-entry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the Mabo campaign from precomputed layer batching into one-hop adaptive recurrence that re-diagnoses and Pareto-selects after every committed cycle.

**Architecture:** Keep the existing proof frontier, Pareto selector, reviewed payment, identity guard, recurrent runner and PG lineage owners. Add only the missing projections: a generalized bounded reviewed-Wikidata context edge, exact current-revision acquisition for a newly admitted target, pure current-frontier move selection, and an outer operator loop that invokes the recurrent runner for exactly one novel cycle before rebuilding durable state.

**Tech Stack:** Rust workspace, PostgreSQL via `postgres`, Wikidata RDF/XML via `quick-xml` + `ureq`, existing SensibLaw proof-search scheduler/frontier/runtime crates.

**Spec:** `docs/superpowers/specs/2026-09-17-mabo-adaptive-hop-reentry-design.md`

## Global Constraints

- `100` is the adaptive committed-cycle budget, not BFS depth.
- SameObject identity review is distinct from outgoing context review.
- No unpinned latest RDF bytes become a durable semantic source.
- Only P1001/P710/P4884/P1594/P4006 participate in bounded reviewed context.
- Candidate context creates no authority, applicability, proposition payment or claim truth.
- Reuse `select_frontier_move`; do not introduce another ranking ontology.
- Reuse `run_recurrent_world_expansion` with `max_cycles: 1`; do not duplicate its staged commit logic.

---

### Task 1: Generalize bounded reviewed Wikidata context

**Files:**
- Modify: `crates/sl-pg-source-store/src/context_federation.rs`
- Test: `crates/sl-pg-source-store/tests/adaptive_wikidata_context.rs`

**Interfaces:**
- Produces: `bounded_wikidata_relation_type(property_ref: &str) -> Option<&'static str>`
- Produces: `review_bounded_wikidata_candidate(source_revision_ref, candidate_id, source_ref, target_ref, property_ref, review_decision) -> Result<ReviewedContextEdge, ContextFederationError>`
- Keeps: `review_mabo_wikidata_candidate(...)` as compatibility wrapper.

- [ ] Write failing tests proving arbitrary exact QID revisions work for the finite property set and mismatched source/QID fails closed.
- [ ] Run `cargo test -p sensiblaw-pg-source-store --test adaptive_wikidata_context` and observe RED because the generalized function does not exist.
- [ ] Implement the minimal generalized reviewer and delegate the existing Mabo wrapper through it after retaining its fixed source/revision guard.
- [ ] Run the focused test and the existing `mabo_context_property_inverse` test GREEN.
- [ ] Commit.

### Task 2: Pin the newly admitted target's own current revision

**Files:**
- Modify: `crates/sl-wikimedia-candidate-provider/Cargo.toml`
- Modify: `crates/sl-wikimedia-candidate-provider/src/lib.rs`

**Interfaces:**
- Produces: `parse_latest_revision_id(qid: &str, json: &[u8]) -> Result<u64, ProviderError>`
- Produces: `fetch_latest_revision_id(qid: &str) -> Result<u64, ProviderError>`
- Produces: `fetch_latest_entity_rdf_revision_receipt(qid: &str) -> Result<AcquiredEntityRdf, ProviderError>`

- [ ] Add a failing unit test using a minimal MediaWiki JSON response and malformed/mismatched QID cases.
- [ ] Run the provider crate tests and observe RED.
- [ ] Add `serde_json` and implement revision-coordinate lookup followed by exact RDF revision fetch.
- [ ] Run provider tests GREEN.
- [ ] Commit.

### Task 3: Compile reviewed identity rows into the canonical Pareto frontier selector

**Files:**
- Create: `crates/sl-world-expansion-runtime/src/adaptive_campaign.rs`
- Modify: `crates/sl-world-expansion-runtime/src/lib.rs`
- Test: `crates/sl-world-expansion-runtime/tests/mabo_adaptive_selection.rs`

**Interfaces:**
- Produces: `MaboAdaptiveSelection { residual_ref, representation_ref, move_ref }`
- Produces: `select_next_reviewed_mabo_gap(frontier: &ProofFrontier, plan: &MaboIdentityReviewPlan) -> Option<MaboAdaptiveSelection>`

Candidate moves are one reviewed identity payment each:
- `target_residual_refs = [row.residual_ref]`
- `ExecutionStrategy::GovernedExactAuthorityFetch` only as a governed live-fetch coordinate, never legal authority
- `network_requests = 1`
- `operator_review_cost = 0` because the identity assignment is already explicitly reviewed
- `expected_proof_reduction = 1`
- `expected_whole_frontier_reduction = 1`
- `shared_dependency_gain = max(relation_type_refs.len(), 1)` as a structural tie-break only
- `admissible = true`

- [ ] Write failing tests proving only matched/open reviewed rows are selectable and selection changes after the previously selected residual is removed from a rebuilt frontier.
- [ ] Observe RED.
- [ ] Implement candidate projection and call `select_frontier_move(frontier, &moves, 1)`.
- [ ] Run focused/runtime tests GREEN.
- [ ] Commit.

### Task 4: Parse a target manifestation into bounded outgoing context candidates

**Files:**
- Modify: `crates/sl-world-expansion-runtime/src/adaptive_campaign.rs`
- Test: `crates/sl-world-expansion-runtime/tests/mabo_adaptive_target_context.rs`

**Interfaces:**
- Produces: `ParsedBoundedContextCandidate { candidate_id, source_qid, target_qid, property_ref, source_revision_ref }`
- Produces: `parse_bounded_target_context(acquired: &AcquiredEntityRdf) -> Result<Vec<ParsedBoundedContextCandidate>, ...>`

- [ ] Write a failing fixture test with one allowed and one unsupported Wikidata property.
- [ ] Observe RED.
- [ ] Reuse `emit_candidates_from_rdf` + `decode_route_candidate`; retain only exact `WikidataProperty` routes whose source equals the acquired QID and whose property is in the bounded context map.
- [ ] Verify deterministic sorting/deduplication and no semantic promotion.
- [ ] Commit.

### Task 5: Add explicit context-review preparation without conflating identity review

**Files:**
- Modify: `crates/sl-world-expansion-runtime/src/adaptive_campaign.rs`
- Test: `crates/sl-world-expansion-runtime/tests/mabo_adaptive_context_review.rs`

**Interfaces:**
- Produces: `prepare_reviewed_target_context(candidates, review_decision) -> Result<Vec<ReviewedContextEdge>, RecurrentRunBlocker>`

- [ ] Write failing test that `NotReviewed` returns `ContextReviewRequired` and `Reviewed` returns candidate-only edges.
- [ ] Add `ContextReviewRequired` to `RecurrentRunBlockerKind` if absent, with regression tests.
- [ ] Implement using `review_bounded_wikidata_candidate` only.
- [ ] Run focused tests GREEN.
- [ ] Commit.

### Task 6: Make PG cycle persistence optionally carry reviewed outgoing context

**Files:**
- Modify: `crates/sl-world-expansion-runtime/src/lib.rs`
- Test: `crates/sl-world-expansion-runtime/tests/lineage_sink.rs`

**Interfaces:**
- Extend `PgDiscoveryLineageSink` with optional staged `ReviewedContextEdge`s for the currently selected object.
- Persist discovery lineage first; then materialize reviewed context edges. A context failure blocks in-memory session commit but leaves durable identity lineage recoverable on restart.

- [ ] Write failing sink test around staged context coordinates using a test seam/pure projection rather than requiring live PG.
- [ ] Observe RED.
- [ ] Implement `with_reviewed_context_edges(...)` and exact selected-object/source checks.
- [ ] Run focused tests GREEN.
- [ ] Commit.

### Task 7: Replace batch queue execution with adaptive one-cycle outer recurrence

**Files:**
- Modify: `crates/sl-world-expansion-runtime/examples/mabo_100hop_recurrent_campaign.rs`
- Test: `crates/sl-world-expansion-runtime/tests/mabo_adaptive_campaign.rs`

**Interfaces / algorithm:**

```text
for campaign_cycle in 0..100:
    reload durable baseline
    reload persisted world for diagnosis
    diagnosis = diagnose(current world, baseline)
    session.frontier = fresh diagnosis frontier
    plan = match explicit identity reviews to current diagnosis
    selected = canonical Pareto selector(plan, frontier)
    if no selected:
        stop IdentityReviewRequired / FrontierExhausted
    prepare exact reviewed SameObject cycle
    if known identity class:
        pay non-novel transaction
        continue
    target_source = fetch_latest_entity_rdf_revision_receipt(selected representation)
    parsed_context = parse bounded outgoing context
    if context not explicitly reviewed:
        stop ContextReviewRequired after reporting exact candidate rows
    reviewed_context = prepare context
    run existing recurrent runner with exactly one queued novel cycle and max_cycles=1
    persist lineage + reviewed context
    loop, reloading durable state
```

- [ ] Write failing source-level regression proving the second selection is computed from a rebuilt frontier rather than a prefilled queue.
- [ ] Remove `Vec<ReviewedPreparedCycle>` batch preparation from the production operator.
- [ ] Keep `ReviewedCycleQueueSource` as a one-element payment-before-release adapter; it is no longer the campaign scheduler.
- [ ] Set campaign budget to 100 cycles; rename graph read budget constants so they cannot be mistaken for campaign hops.
- [ ] Print per-cycle `selected_residual`, `selected_representation`, target revision, parsed context count and stop reason.
- [ ] Run runtime tests, workspace tests and clippy GREEN.
- [ ] Execute live campaign; use its first real typed blocker/receipt to drive the next tranche.
- [ ] Commit and update PR receipt.

### Task 8: Agda parity

**Files:**
- Modify or extend: `DASHI/Wikimedia/Mabo100HopReviewedCampaignExact.agda`
- Modify validation: `DASHI/Wikimedia/Mabo100HopReviewedCampaignValidation.agda`

**Required equations/boundaries:**

```text
campaignCycle != traversalDepth
durableNovelIdentityCount != campaignCycle
nextHop depends on postAcquisitionAssessment
precomputedSiblingQueue != adaptive recurrence
identityReview != outgoingContextReview
latestRevisionLookup != admittedSourceManifestation
```

- [ ] Add validation obligations first.
- [ ] Add the minimal campaign owner terms reusing canonical proof-search/context owners.
- [ ] Run Agda locally if available; otherwise mark source-written/kernel-unobserved.
- [ ] Commit and update #991 parity receipt.
