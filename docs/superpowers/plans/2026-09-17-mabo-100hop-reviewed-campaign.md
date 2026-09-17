# Mabo 100-hop Reviewed Campaign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the existing 100-hop consumer diagnosis into a runnable, review-driven recurrent Mabo campaign that can consume explicit identity reviews, persist reviewed payment before novelty admission, commit durable lineage through the existing runner, and stop honestly with a concrete review queue when review is missing.

**Architecture:** Keep the current `LatentWorldRows -> MaboConsumerDiagnosis -> ProofFrontier` compiler unchanged as the semantic diagnosis entrypoint. Add a small explicit review-manifest/parser and a payment-persisting prepared-cycle queue in `sl-world-expansion-runtime`; the executable reacquires the exact pinned Wikidata source manifestation for reviewed rows, prepares existing `PreparedWorldExpansionCycle` values, seeds `IdentityCoherentCycleSource` from the durable baseline, and runs the existing recurrent runner with `PgDiscoveryLineageSink`. No graph adjacency, QID shape, experiment design, proof search, Perplexity/external comparison, or semi-formal reasoning self-certifies identity review.

**Tech Stack:** Rust workspace (`sl-world-expansion-runtime`, `sl-proof-search-loop`, `sl-pg-source-store`, `sl-reviewed-evidence-payment`, Wikidata candidate provider) plus DASHI Agda parity owners.

**Spec:** `docs/superpowers/specs/2026-09-17-mabo-consumer-diagnosis-100hop-design.md` plus the user-approved continuation: diagnosis -> explicit reviewed identity assignments -> reviewed payment -> identity-coherent recurrent admission -> durable lineage -> re-diagnose.

## Global Constraints

- GitHub CI is not used or queried.
- `100 hops` is a traversal budget, not 100 reviewed identities.
- Novelty is counted only by reviewed durable `identity_class_ref`.
- A reviewed context relation may create a `SameObject` demand but never pays it.
- Explicit review input is required before a diagnosed representation can become a reviewed identity cycle.
- Reviewed payment must be persisted before the corresponding novel cycle is handed to the recurrent runner.
- `candidate_only=true`, `creates_semantic_authority=false`, `applicability_promoted=false`, `claim_truth_promoted=false` remain invariant.
- Existing proof-search, experiment-design, `FactorsThrough`, `Admissible`/`WrongType`, missing-carrier, SFM, and external-knowledge/Perplexity lanes are proposal/diagnostic routes only.
- Connector-only implementation may establish source-order RED and source wiring, but must not claim Cargo/Agda GREEN without local execution.

---

### Task 1: Explicit identity-review manifest and queue semantics

**Files:**
- Modify: `crates/sl-world-expansion-runtime/src/lib.rs`
- Create: `crates/sl-world-expansion-runtime/tests/mabo_review_manifest.rs`

**Interfaces:**
- Produces `MaboIdentityReviewAssignment { representation_ref, identity_class_ref, review_ref }`.
- Produces `parse_mabo_identity_review_tsv(&str) -> Result<Vec<MaboIdentityReviewAssignment>, MaboReviewManifestError>`.
- Produces `plan_mabo_identity_reviews(&MaboConsumerDiagnosis, &[MaboIdentityReviewAssignment]) -> Result<MaboIdentityReviewPlan, MaboReviewManifestError>`.
- The plan separates matched reviewed rows from unmatched explicit review records and never invents a review for an unlisted diagnosis row.

- [ ] **Step 1: Write the failing tests**

Tests must pin: comments/blank lines are ignored; three non-empty tab-separated fields are required; conflicting duplicate representations fail closed; exact duplicate assignments de-duplicate; only diagnosed representations enter `matched`; unmatched review records remain visible; no review means every diagnosis row remains pending.

- [ ] **Step 2: Observe source-order RED**

Confirm the production symbols do not exist at the test commit before implementation. In this connector-only session this is a source-order RED receipt, not a compiler receipt.

- [ ] **Step 3: Implement the minimal parser/planner**

Use only `std` collections. Do not add serde or a second review ontology.

- [ ] **Step 4: Local operator verification**

Run later in the local checkout:

```sh
cargo test -p sensiblaw-world-expansion-runtime --test mabo_review_manifest
```

### Task 2: Payment-persisting reviewed cycle queue

**Files:**
- Modify: `crates/sl-world-expansion-runtime/src/lib.rs`
- Create: `crates/sl-world-expansion-runtime/tests/reviewed_cycle_queue.rs`

**Interfaces:**
- Produces `ReviewedPreparedCycle { prepared, reviewed_payment_wire }`.
- Produces `ReviewedCycleQueueSource<K>` implementing the existing `WorldExpansionCycleSource` when `K: KnownIdentityPaymentSink`.
- Empty queue returns `RecurrentRunBlockerKind::IdentityReviewRequired`.
- Before returning a prepared cycle, the source persists that cycle's reviewed-payment wire; persistence failure leaves the cycle queued and returns `PersistenceBlocked`.

- [ ] **Step 1: Write failing tests**

Pin empty-queue blocking, payment-before-cycle ordering, and retry preservation after a fake payment-sink failure.

- [ ] **Step 2: Observe source-order RED**

Confirm `ReviewedCycleQueueSource` is absent at the test commit.

- [ ] **Step 3: Implement minimal queue source**

Use `VecDeque`; do not change `world_expansion_runner` ABI.

- [ ] **Step 4: Local operator verification**

```sh
cargo test -p sensiblaw-world-expansion-runtime --test reviewed_cycle_queue
```

### Task 3: Runnable 100-hop reviewed recurrent campaign

**Files:**
- Create: `crates/sl-world-expansion-runtime/examples/mabo_100hop_recurrent_campaign.rs`
- Modify: `crates/sl-world-expansion-runtime/Cargo.toml` only if an already-workspace dependency is actually required.

**Interfaces / behavior:**
- Invocation:

```sh
cargo run -p sensiblaw-world-expansion-runtime --example mabo_100hop_recurrent_campaign -- [IDENTITY_REVIEW_TSV]
```

- Always loads the configured PostgreSQL store, durable identity baseline, and deterministic `Q1501525` world with `max_hops=100`, `max_nodes=10000`, `max_edges=50000`.
- Always compiles `diagnose_mabo_context_world_identity` and the canonical `ProofFrontier`.
- With no manifest, prints exact pending review rows and exits successfully at `IdentityReviewRequired`; it performs no network acquisition and no writes.
- With a manifest, each matched diagnosis row is reacquired from the exact `wikidata:<QID>:oldid:<N>` source manifestation encoded by its requirement scope, then the actual Wikidata route whose target equals the reviewed representation is selected deterministically.
- Each reviewed row compiles a real `SameObject` reviewed-payment stream, checks that the review-aware residual compiler pays exactly that requirement, constructs the existing `ExpansionCandidate`, `WorldIdentityResolutionReceipt`, and `PostAcquisitionWorldObservation`, and queues a `PreparedWorldExpansionCycle`.
- The queue is wrapped by `IdentityCoherentCycleSource::with_baseline`; lineage goes through `PgDiscoveryLineageSink`; the existing recurrent runner owns commit/rollback of in-memory semantic state.
- The final receipt prints baseline count, newly committed count, durable total, open residuals, stop reason, and unmatched/pending review rows.

- [ ] **Step 1: Keep all preparation behind explicit review records**

No diagnosed row without a manifest assignment may cause provider I/O or identity admission.

- [ ] **Step 2: Reuse exact-source parsing**

Parse only `wikidata:<QID>:oldid:<N>`; unsupported source manifestations fail closed rather than silently moving to a latest revision.

- [ ] **Step 3: Preserve payment/admission separation**

A persisted review payment may survive a later lineage-persistence failure; this is acceptable because evidence payment is independent of novelty admission. The session itself must remain unchanged unless the existing runner's sink succeeds.

- [ ] **Step 4: Local operator verification**

```sh
cargo test -p sensiblaw-world-expansion-runtime --test mabo_review_manifest
cargo test -p sensiblaw-world-expansion-runtime --test reviewed_cycle_queue
cargo test -p sensiblaw-world-expansion-runtime --test mabo_consumer_diagnosis
cargo test -p sensiblaw-world-expansion-runtime
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p sensiblaw-world-expansion-runtime --example mabo_100hop_recurrent_campaign
```

The last command should produce a live 100-hop traversal/diagnosis receipt and a concrete identity-review queue without requiring a manifest.

### Task 4: Agda parity for reviewed campaign execution boundary

**Files:**
- Create: `DASHI/Wikimedia/Mabo100HopReviewedCampaignExact.agda`
- Create first: `DASHI/Wikimedia/Mabo100HopReviewedCampaignValidation.agda`
- Modify: `DASHI/Wikimedia/AgdaSlrJmdLeanBidiEverything.agda`

**Interfaces / semantics:**
- Reuse `MaboConsumerResidualDiagnosisExact`, `SnowballPluralLensDiscoveryAdmissionExact`, `ExperimentalCoordinateDesignExact` / active proof-search experiment owners, `IntersectionalNonFactorability`, `SensibLawWoogarooAdmissibleFactorsWrongTypeAtomBridgeExact`, and `SFMVerifiedClaimPresentation`.
- Formalise `diagnosis proposal != admissible reviewed identity != persisted payment != novel durable admission`.
- Retain the Round-14 acquisition lesson as a generic selection principle only: realised carrier cannot determine eligible-but-missing state; therefore missing-carrier/factorisation failure may raise observation/review demand but cannot manufacture the missing value.
- Record the runtime executable string and explicit review-manifest requirement.
- `externalKnowledgeComparison` / Perplexity and SFM remain proposal/presentation routes; neither is a review/payment authority.

- [ ] **Step 1: Add validation before production owner**

The validation imports the future owner and pins the campaign firewalls.

- [ ] **Step 2: Observe source-order RED**

Confirm the production path is absent at the validation commit.

- [ ] **Step 3: Add production owner and rollup import**

Do not claim Agda kernel GREEN.

### Task 5: Roadmap / PR handoff

**Files:**
- Modify: `docs/roadmap/mabo_p7d_recurrent_runtime_20260917.md`
- Update SLR #25 and DASHI #991 descriptions.

- [ ] **Step 1: Replace the stale semantic-wall wording**

State that diagnosis exists and the live boundary is now explicit identity review/payment plus campaign execution.

- [ ] **Step 2: Give the exact operator commands**

Include the no-manifest 100-hop command and the reviewed-manifest campaign command.

- [ ] **Step 3: Preserve verification language**

Source-written / source-order RED only for this connector tranche until the local operator runs Cargo/Agda.
