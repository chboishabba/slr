# ITIR structural interlingua — consumer-indexed evidence transport

This is the cross-domain compiler/comparator contract, **not a legal or
Wikidata ontology**. Original observations come from existing source
adapters; their source revision/region identity and creator remain
separate. The compiler *proposes* roles, predicates, modality, temporal
and attribution coordinates; ontology constraints are one **consumer
fibre**, not a global starting ontology.

The implementation is
`crates/sl-pg-source-store/src/relational_interlingua.rs` (generic
comparator), `relational_interlingua_store.rs` (durable native-source
read/reopen), `examples/itir_relational_compare.rs` (typed offline
comparison). The existing `CandidatePnfBatch` adapter and WIKI-1
`OntologyNativeStatement` adapter feed the **same** generic carrier.

DASHI Agda companion:
`DASHI/Cognition/PNF/ITIRRelationalInterlinguaConsumerFibreExact.agda`
specifies structurally indexed candidate observations, consumer
alignment licences, separate support/counter-support and non-promotion
bounds. The formal source has not yet been kernel-checked.

## Contract

A source-native relational observation retains:

- source-family and **original source revision/span identity**;
- original parser/producer reference;
- selected PNF predicate candidate, ordered occurrence-indexed roles;
- candidate type hypotheses **with native evidence references**;
- original scope, time, modality, quantification, attribution,
  ontology and language contexts;
- native rank/revision/qualifier and source references, where available;
- support / counter-support / explicit unknown reports, kept separate.

One `RelationalConsumer` provides the **specific requested operation**:
required roles; witnessed role/predicate/filler/type alignments with
consumer-and-licence refs; requested context comparison coordinates;
and expected role types. The generic comparator never consults a
hard-coded table equating legal, biomedical or Wikidata terms.

For left and right observations we emit distinct coordinates:

```
source A ──> role/predicate candidate ─┐
                                      ├─> consumer-indexed comparison
source B ──> role/predicate candidate ─┘            │
                                                residuals
                                                witnesses
                                                separate + / - / ?
```

A same-shaped role set can be a candidate agreement only. Shared QIDs,
surface words or observed members do not create ontology type identity.
Role matching across different labels requires **explicit alignment
witnesses**; ambiguous one-to-many role matches remain unpaid residuals.
Polarity conflicts are merely conflict *candidates* and only when
roles/predicates and the selected comparison context are compatible.

Type pressure is particularly important. Suppose both readings have
the same `patient` role but candidate types `companion` and
`organism`. A particular consumer requires an `organism` patient.
The relation can therefore be structurally comparable while the
consumer reports `role_type_contract_pressure` for the first
reading. Neither type nor source is changed. A missing type witness
yields `missing_role_type_evidence`, not negative membership.

`comparison.residuals` preserves incompatibilities, missing
roles, unaligned type candidates, scope/time/modality/quantification/
attribution/ontology/language differences, source-provenance gaps
and unresolved polarity. No destructive global hypothesis pruning is
performed. Licensed equivalence is **operation-relative** and does
not imply substitutability under another contract.

## Real owner integration

- spaCy/PNF: `observation_from_candidate_pnf` consumes the existing
  `CandidatePnfBatch`; selecting the already-produced predicate
  candidate is required. This is not a new parser or an inferred
  polarity classifier.
- Wikidata: `observation_from_native_wikidata` carries the real
  `OntologyNativeStatement` witness, including native qualifier
  role bindings; rank and revision remain inspectable metadata, not
  fabricated attribution.
- Other families: submit original PNF candidate observations using the
  *same* `RelationalObservation` carrier. A lexical match, shared
  QID or co-occurrence does not create a licensed cross-source join.
- Durable execution: `persist_relational_comparison` checks that
  both source revisions already exist in the generic-source or native
  chat store; writes a candidate-only SQL sidecar; and reopens by
  deterministic comparison ref. Replay recomputes the generic
  comparison and validates the original packet checksum; changed
  content for the same identity fails closed.
- Reader: `itir-dioxus/workbench::relational` loads the persisted
  typed packet under the existing S30 `MatterContext`, requiring
  both original source revisions visible. It reuses `GraphIr`,
  `VisualObjectId` and shell/GPU command vocabulary. This creates
  no second review or semantic authority.

**Important:** Merely supplying source IDs and a PG row cannot certify
the *accuracy of parser-inferred roles*, an alignment licence's
issuer, or independent provenance. Those remain separate producer,
review, and domain-specific acceptance obligations.

## Use

For an already produced, source-referenced pair:

```bash
cargo run -p sensiblaw-pg-source-store \
  --example itir_relational_compare -- /path/to/typed-pnf-pair.json
```

This runner is a *pure computation*, not evidence that its input source
IDs have been independently authenticated. PG persistence checks
source presence separately. The Dioxus reader may open a persisted
comparison with:

```bash
SENSIBLAW_MATTER_SCOPE=/path/to/scope-manifest.json \
ITIR_RELATIONAL_COMPARISON_REF='relation-comparison:sha256:...' \
  cargo run --features desktop
```

The Matter ref must equal the comparator's consumer ref, and the
selected source revisions must be visible under its recipient, role,
purpose, and knowledge cut. GUI inspection does not grant permission
to modify sources, review state or ontology constraints.

## Programme-facing specimens

1. Class versus set with identical observed incidence: type-contract
   residual until a consumer-specific equivalence obligation is paid.
2. Ordinary vs Simple vs other-language Wikipedia revision: shared
   entity is a *retrieval* coordinate only. Compare source-grounded PNF
   predicates, roles, scopes, time and negation with explicit linguistic
   alignments; source-local omissions remain residuals.
3. Biomedical vs ordinary-language source: compare a relation shape
   across distinct native source families; expected species/lifecycle
   or role types are **consumer constraints**, not global automatic
   definitions.
4. Opposing legal accounts and repeated chat quotations: retain
   opposing polarities and source genealogy; never count copied reports
   as independent witnesses.
5. Animal observation: retain recurring signal/behavior context as
   candidate roles without inventing human-language propositional
   content.

These specimens are **requested acceptance scenarios**, not proof
of completed multilingual, biological or animal semantics. Each
requires independently sourced data and a matching consumer licence.

## Explicit acceptance gaps

- exact-head Rust/Agda compilation and real PostgreSQL persistence
  and reopen receipts;
- real two-domain PNF specimens with externally verified source spans,
  consumer licences and residual interpretations;
- licensed transport under real multilingual revisions and
  forward/backward explanation parity;
- no new quadratic all-pairs source search (consumer selects pairs,
  existing corpus indices nominate neighbours);
- Dioxus and physical wgpu execution and review-authority integration;
- separately held SCALE-1 production economy measurement.

A compiler implementation is not runtime or scientific validation.

## ITIR-REL-1 runnable acceptance and soft-type refinement (new tranche)

The new `scoped_soft_type.rs` evaluates a **consumer-scoped single-value
contract** without inferring a violation from an ontology label or
different raw values. Each selected pair carries independently supplied
subject-identity, property-alignment, scope-comparability, applicability
and value-distinctness witness references. Their absence yields an
`undetermined` receipt with a specific missing-premise ref. Different
year/scope tags similarly block the violation outcome. Only after
these separate payments are present does it emit
`candidate_violation`; this is still a source-backed *candidate*, not a
kernel proof of the producer's assertion or a Wikidata edit licence.

The formal statement in
`dashi_lean4/DASHI/output-final_aristotle/RequestProject/
DASHIScopedSoftTyping.lean` requires actual propositions:
two observations in the same consumer scope **and a constructive proof
that their values differ**. The `violationHasConcreteCounterexample`
and `missingCannotBePromotedToViolation` lemmas characterize the
contract. Source-provided Rust receipt strings do not magically satisfy
the stronger Lean proof; the producer-to-formal-evidence translation
remains a distinct validation gate. The Agda partner
`DASHI/Core/ScopedSoftTypeContractExact.agda` states the same
proof-relevant counterexample boundary and a residual-eliminating
repair record.

`bounded_repair_assessment.rs` adds a measured *model-only* repair
assessment. It checks that the new typed residual set is a **strict
subset** of the old residual set and reports both discharged and newly
introduced obligations. An unchanged or regression-creating candidate
fails the `strictly_improves_checked_debt` result. A separately supplied
consumer-observation preservation witness is required, but remains
producer-declared until independently validated. No external source
edit, ontology rewrite, admission or repair execution follows from
the assessment. The Lean `MeasuredImprovement` theorem combines
consumer-observation preservation with a strict decrease under an
explicit debt measure, rather than claiming every proposed repair
improves anything.

### Acceptance executable

The new `examples/itir_rel1_acceptance.rs` consumes a JSON suite
`itir.rel1.source-acceptance.v1` with **at least three real native
source cases** and one negative control. Each case declares:

- `case_ref`, `family_purpose`, `negative_control` and independently
  obtained acquisition/producer receipt refs;
- an original `left`/`right` `RelationalObservation` plus the
  *same* typed `RelationalConsumer` carrier across domains;
- expected `finding`, required and forbidden residual kinds;
- expected original-source `left_source_sha256` and
  `right_source_sha256` for the already persisted canonical bytes.

Run with the actual production-postgres configuration **after**
persisting both native sources through their existing adapters:

```bash
cargo run -p sensiblaw-pg-source-store \
  --example itir_rel1_acceptance -- \
  /path/to/independent-source-suite.json
```

The runner uses the generic comparator, persists in the **existing SLR
authority database**, reopens the resulting packet, recomputes the typed
finding and checks every expected source digest/residual, failing the
process if any case fails. The persisted comparison now contains the
actual SHA-256 of **both** native source payloads (rather than merely
proof of source-row existence), which must still match upon replay.
For generic source revisions, the native ingestion digest stored by
SLR must also agree with those bytes; tampered content is rejected.

The first three independently sourced test families should be:

1. A Wikidata native full statement and a separately sourced
   prose-derived PNF observation, retaining GUID, qualifiers, source
   references and role licences; include an unmet scope/cardinality
   negative control.
2. A biomedical cross-species source pair with *independently sourced*
   organism/lifecycle contracts. Shared argument structure is not a
   licence to transfer organism-specific propositions.
3. Ordinary / Simple / another-language Wikipedia with pinned article
   revisions and separate licensed predicate/role correspondences.
   The same QID or translated surface word is never the equivalence
   certificate. An omitted qualification must survive as a residual.

**No fabricated provenance is bundled with this runner.** Its
independent source receipts, approved licences and expected semantic
outcomes have to originate in actual producer/source/reviewer
machinery. An executable binary without those fixtures is not a
passed acceptance suite. The external run/kernel/GPU reports and
SCALE-1 performance closure remain outstanding.

## ITIR-REL-1B integrated contract acceptance

The acceptance schema has advanced to
`itir.rel1.integrated-acceptance.v1`. Each case still carries the
same revision-pinned `RelationalObservation` pair and one
`RelationalConsumer`, but may now add:

- **typed runtime witness certificates**, source-pinned to the same
  fixture and validator-attributed;
- one optional **scoped single-value contract** with expected status,
  required missing premises and forbidden missing premises;
- one optional **modeled repair** with a recomputed after-comparison,
  expected strict-improvement state, expected discharged obligations
  and expected new obligations.

The executable now performs the intended graph in one run:

```
native source revisions
  -> generic relational comparison
  -> optional source-scoped contract judgment
  -> PostgreSQL persist + real reopen/recompute/source-digest verification
  -> optional modeled repair re-comparison + debt assessment
```

At least one suite case must exercise a scoped contract and at least
one must exercise repair; the suite must still contain the Wikidata↔
text, biomedical, and multilingual Wikipedia families plus a negative
control. This pays the integration gap where comparator, soft typing
and repair existed but were previously exercised independently.

### Runtime witness certification is not a proof cast

`runtime_witness_certificate.rs` introduces typed certificates for
subject identity, property alignment, scope comparability, each
source-side applicability, value distinctness, positive
non-applicability/outside-scope, consumer-observation preservation,
and heterogeneous observable bridges. Validation requires native
source refs, evidence refs, a deterministic payload digest, validator
identity/version and a validation receipt.

The validator deliberately rejects a certificate that sets
`formal_premise_established=true`. A checked runtime witness can pay
an **executable acceptance premise**, but cannot be silently coerced
into a Lean/Agda proposition. Formal companions:

- Lean:
  `RequestProject/DASHIRuntimeWitnessRefinement.lean`;
- Agda:
  `DASHI/Core/RuntimeWitnessFormalRefinementExact.agda`.

Both require an independently supplied formal premise to construct the
proof-carrying refined witness. This is the explicit
runtime-witness → checked-evidence → formal-premise boundary.

### Outside scope is positive evidence, not missing evidence

`ScopedPairEvidence.positive_outside_scope_witness_ref` distinguishes
positive non-applicability from unresolved comparability. If an
authorized validator establishes that the selected contract does not
apply to the pair, `evaluate_single_value_contract` returns
`outside_scope`. If scope/applicability is merely missing, it returns
`undetermined` with explicit unpaid premise refs. Neither state is a
violation.

### Multi-consumer repair vector

A modeled transformation can now be assessed over a vector of named
consumer fibres. `repair_vector.rs` classifies each consumer:

- `improved`: residual debt strictly decreased with no new debt;
- `preserved`: checked residual debt is unchanged;
- `regressed`: new/replacement residual debt appears;
- `unchecked`: no assessment exists for that consumer.

`acceptable_for_further_review` is true only when **no checked
consumer regresses and at least one checked consumer improves**.
Unchecked consumers remain explicit and cannot be treated as
preserved. This is still a modeled review candidate, not external edit
authority. Formal owners are
`DASHIRepairVector.lean` and
`MultiConsumerRepairVectorExact.agda`.

No exact-head runtime/kernel receipt or real three-domain source suite
is claimed by committing these integration surfaces.

## Live fixture acquisition/preflight

REL-1B deliberately does **not** synthesize a "real" cross-domain suite from
whatever happens to be present in the authority database. The current live
database may contain unrelated generic/chat sources and still be wholly
unsuitable for the three-domain acceptance contract.

`examples/itir_rel1_fixture_preflight.rs` checks a separately prepared
`itir.rel1.fixture-acquisition.v1` manifest against the live PostgreSQL
authority. For each required source it reports:

- source revision presence;
- canonical payload SHA-256;
- native source family;
- stored acquisition receipt agreement where the native store exposes it;
- explicit revision locator;
- parser/adapter receipt;
- consumer and alignment-licence coordinates.

A case is **not ready** when any source is missing, hashes differ, the stored
source family differs, a generic-source acquisition receipt differs, or the
consumer/licence/producer coordinates are absent. The tool never substitutes
a chat quote, cached example, same-QID page, fixture from another matter, or
model-generated text for a missing source.

Run:

```bash
cargo run -p sensiblaw-pg-source-store \
  --example itir_rel1_fixture_preflight -- \
  /path/to/real-acquisition-manifest.json
```

A deliberately non-runnable planning specimen lives at
`docs/itir_rel1_fixture_acquisition.template.json`; its schema is
`itir.rel1.fixture-acquisition.template.v1`, so the executable will reject it
until the caller replaces all placeholders and deliberately changes the
schema to `itir.rel1.fixture-acquisition.v1`.

This separates two states that must not be conflated:

1. **implementation ready to evaluate a fixture**, and
2. **the required independently sourced fixture actually exists in PG**.

A blocked preflight is therefore an honest acquisition result, not an
acceptance failure of the relational mathematics.
