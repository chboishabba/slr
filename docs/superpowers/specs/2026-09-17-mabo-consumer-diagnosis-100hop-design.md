# Mabo Consumer Diagnosis over the 100-hop World

## Goal

Turn the already-working persisted 100-hop Mabo latent-world read surface into an explicit, consumer-indexed residual-diagnosis surface without allowing graph adjacency, relation review, identifier shape, proof search, experimental design, or external/Perplexity comparison to manufacture evidence payment or reviewed world identity.

## Existing owners reused

- `sl-pg-source-store::latent_world` owns deterministic bounded traversal and provenance.
- `sl-consumer-residual` owns `ConsumerSpec`, `ConsumerRequirement`, PNF/evidence coordinates, gaps and obligations.
- `sl-proof-search-loop` owns `ProofResidual`, residual classes, producer selection and recurrence.
- `sl-reviewed-evidence-payment` owns explicit reviewed payment.
- `sl-world-expansion-runtime` remains the thin integration crate.
- DASHI Agda remains the golden semantic contract; the new Agda owner must reuse `SnowballPluralLensDiscoveryAdmissionExact`, `IntersectionalNonFactorability`, experiment-design/proof-search machinery and the existing Mabo reviewed-context/payment owners.

## Diagnosis semantics

The production consumer is explicitly named `consumer:mabo-context-world-identity`. It asks a bounded question:

> For each *reviewed Mabo context relation target* present in the persisted latent world, do we have an already-reviewed durable world-identity assignment for that representation?

A reviewed context edge may therefore create an **identity-review demand** for its target representation. It does not create the identity review itself.

Only persisted relations whose type begins with `context:wikidata:` and whose provenance is a reviewed Wikidata context receipt are admissible to this first diagnosis lane. Other latent-world edges remain visible in the traversal receipt but are classified as out-of-scope/wrong-type for this consumer rather than coerced into identity work.

The diagnosis de-duplicates target representations before emitting requirements. A representation already present in the durable identity baseline is reported as known and does not create a new SameObject requirement. An unknown reviewed-context target emits:

- requirement id `world-identity:<representation>`;
- need `EvidenceCoordinate(SameObject)`;
- scope bound to the exact reviewed source-revision provenance when available;
- open proof residual `residual:mabo:world-identity:<representation>` with residual class `Identity`.

The diagnosis is proposal/admission aware:

- proof search, failed `FactorsThrough`, experiment design, observed residuals, `WrongType`, affected-subject/missing-carrier analysis, and external-knowledge/Perplexity comparison may propose or rank a diagnostic axis;
- none may self-certify an evidence coordinate or identity review;
- `WrongType` may reject a latent-world edge for this consumer;
- failed factorisation may justify adding a missing observer/axis;
- experiment design may say what evidence to acquire next but is not evidence.

## Intersectional / missing-carrier boundary

The Agda side carries the reusable finite witness:

`realised analytic carrier` does not in general determine `eligible population state`.

Two eligible populations can produce the same realised/observed carrier while differing in who is missing. Re-labelling or re-charting the realised carrier cannot recover that missing state. This is used as a discovery diagnostic only: it can demand a new observation/consumer axis, but it cannot invent facts about absent people or objects.

For the Mabo runtime this means `visited_refs`, `reviewed relation targets`, and `durable identity classes` are kept separate. A missing identity assignment is an explicit residual, not a silently inferred identity.

## 100-hop executable

Add a runnable example that:

1. loads the configured PostgreSQL store;
2. loads the durable identity baseline;
3. executes `load_latent_world_rows_with_budget` from `Q1501525` with `max_hops = 100` and generous node/edge caps;
4. compiles the consumer-indexed diagnosis;
5. emits the corresponding residual stream through `sl-consumer-residual` without pretending it is paid;
6. prints traversal, provenance, known-identity, wrong-type/out-of-scope and open-identity-review counts plus the first open residual references;
7. preserves `creates_semantic_authority=false`, `applicability_promoted=false`, and `claim_truth_promoted=false`.

This executable is the substantial execution boundary for this tranche. It should be usable immediately in the user's local SLR checkout. The subsequent recurrent campaign consumes reviewed payments for these residuals; this tranche does not auto-review them.

## Agda parity owner

Add `DASHI/Wikimedia/MaboConsumerResidualDiagnosisExact.agda` plus validation. The owner must:

- reuse the existing Snowball plural-lens discovery routes rather than inventing a second route ontology;
- make the context-identity consumer explicit;
- expose context-review != identity-review and adjacency != requirement/payment firewalls;
- expose the finite eligible-population / realised-carrier non-factorability witness;
- retain experiment-design, WrongType, proof-search and external-knowledge comparison as discovery-only routes;
- record the SLR executable/consumer identifiers without claiming runtime or kernel GREEN.

## Acceptance boundary

Paid by this tranche:

- deterministic 100-hop world -> canonical explicit Mabo identity-diagnosis frontier;
- durable-baseline quotient before requirements are emitted;
- wrong-type/out-of-scope accounting;
- source-revision-scoped SameObject requirements;
- Agda parity of the semantic boundary.

Not paid by this tranche:

- automatic identity review;
- 100 novel reviewed identity classes;
- legal authority, applicability or claim truth;
- any claim that Perplexity/external comparison is itself evidence;
- Agda kernel or Rust execution GREEN without a local receipt.
