# Mabo Consumer Diagnosis 100-hop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compile the persisted 100-hop Mabo world into a canonical, consumer-indexed identity-review residual frontier and mirror the semantic boundary in Agda.

**Architecture:** Keep traversal in `sl-pg-source-store`, residual semantics in `sl-consumer-residual`/`sl-proof-search-loop`, and add only integration code to `sl-world-expansion-runtime`. The Agda owner reuses Snowball plural-lens discovery/admission, intersectional non-factorability, experiment-design/proof-search and existing Mabo payment/context owners; it does not create another ontology.

**Tech Stack:** Rust workspace crates, PostgreSQL, Agda 2.x source contracts.

**Spec:** `docs/superpowers/specs/2026-09-17-mabo-consumer-diagnosis-100hop-design.md`

## Global Constraints

- Do not infer identity review from a reviewed relation edge.
- Do not infer consumer requirements from arbitrary graph adjacency.
- Only reviewed `context:wikidata:*` edges enter the first identity-diagnosis consumer.
- Durable known identities are quotiented before open requirements are emitted.
- Proof search / failed FactorsThrough / experiment design / WrongType / affected-subject / external-knowledge routes are proposal/diagnostic only.
- Preserve `creates_semantic_authority=false`, `applicability_promoted=false`, `claim_truth_promoted=false`.
- No CI polling.

---

### Task 1: RED contract for Rust diagnosis

**Files:**
- Create: `crates/sl-world-expansion-runtime/tests/mabo_consumer_diagnosis.rs`

**Interfaces:**
- Consumes: `LatentWorldRows`, `LatentWorldEdgeRow`, `DiscoveryIdentityBaseline`.
- Produces expected API: `diagnose_mabo_context_world_identity(&LatentWorldRows, &DiscoveryIdentityBaseline) -> MaboConsumerDiagnosis`.

- [ ] Write tests requiring: reviewed Wikidata targets become SameObject requirements; known baseline targets are skipped; duplicate target relations yield one requirement; non-context edges are counted wrong-type/out-of-scope; source revision is retained in `RequirementScope::SourceManifestation`; no promotion flags are true.
- [ ] Confirm production symbol is absent at source-order RED point.
- [ ] Commit RED contract.

### Task 2: Minimal Rust diagnosis implementation

**Files:**
- Modify: `crates/sl-world-expansion-runtime/src/lib.rs`

**Interfaces:**
- Produces: `MaboConsumerDiagnosis`, `MaboIdentityDiagnosisRow`, `diagnose_mabo_context_world_identity`.

- [ ] Implement deterministic target de-duplication over reviewed `context:wikidata:*` edges.
- [ ] Extract exact reviewed source-revision provenance from `context:wikidata:<revision-ref>` provenance entries.
- [ ] Emit `ConsumerSpec` SameObject requirements only for targets absent from the durable representation→identity map.
- [ ] Emit matching open `ProofResidual`s with `ResidualClass::Identity` metadata retained in diagnosis rows.
- [ ] Keep non-context edges in explicit out-of-scope/wrong-type counts.
- [ ] Keep all promotion booleans false.

### Task 3: Runnable 100-hop diagnosis executable

**Files:**
- Create: `crates/sl-world-expansion-runtime/examples/mabo_100hop_consumer_diagnosis.rs`

**Interfaces:**
- Consumes: configured PostgreSQL DB, durable identity baseline, `load_latent_world_rows_with_budget`, diagnosis API, residual compiler.
- Produces: stdout execution receipt and binary residual stream only in-memory.

- [ ] Run from seed `Q1501525` with `max_hops=100`, `max_nodes=10000`, `max_edges=50000`.
- [ ] Compile diagnosis `ConsumerSpec` against an empty payment stream so every unknown identity remains an honest open gap/obligation.
- [ ] Print traversal depth/counts, diagnosis counts, residual counts, first residual refs, and non-promotion booleans.
- [ ] Exit nonzero if the traversal receipt or diagnosis claims semantic authority/applicability/truth promotion.

### Task 4: Agda RED validation then parity owner

**Files:**
- Create first: `DASHI/Wikimedia/MaboConsumerResidualDiagnosisValidation.agda`
- Create second: `DASHI/Wikimedia/MaboConsumerResidualDiagnosisExact.agda`
- Modify: `DASHI/Wikimedia/AgdaSlrJmdLeanBidiEverything.agda`

**Interfaces:**
- Reuses `SnowballPluralLensDiscoveryAdmissionExact`, `IntersectionalNonFactorability`, Mabo reviewed-context/payment owners, and existing experiment-design/proof-search surfaces.

- [ ] Commit validation import before production owner and observe production path absent.
- [ ] Formalise explicit `contextWorldIdentityConsumer` and diagnostic route receipts.
- [ ] Prove finite `realisedCarrier` non-factorability against an eligible-population/missing-carrier outcome.
- [ ] Pin firewalls: relation review != identity review; adjacency != consumer requirement; experiment plan != evidence; external comparison != payment; WrongType adjacency != type identity.
- [ ] Record the SLR executable identifier and 100-hop budget without claiming execution GREEN.
- [ ] Import validation into the existing Wikimedia BIDI rollup.

### Task 5: Verification boundary and handoff

- [ ] Do not poll CI.
- [ ] Attempt local source execution only if GitHub/network is reachable; otherwise record DNS as the execution blocker.
- [ ] Provide exact local commands:
  - `cargo test -p sensiblaw-world-expansion-runtime --test mabo_consumer_diagnosis`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo run -p sensiblaw-world-expansion-runtime --example mabo_100hop_consumer_diagnosis`
- [ ] Report the first runtime-generated residual frontier as the next production input, not as already-paid identity review.
