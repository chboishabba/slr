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
