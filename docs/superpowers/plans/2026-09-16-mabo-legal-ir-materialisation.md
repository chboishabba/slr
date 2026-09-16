# Mabo LegalIR binary materialisation implementation plan

Date: 2026-09-16

## Goal

Close the missing production seam between an already-admitted SensibLaw LegalIR projection and the PostgreSQL runtime queried by the Mabo proof/source weld, without turning SLR into a second legal-semantics engine.

```text
canonical source/span
-> ordinary fibred PNF
-> SensibLaw-owned LegalIR semantics
-> typed binary LegalIR materialisation
-> append-only PostgreSQL legal_ir v2 rows
-> Mabo source/proposition weld
-> bounded WHY projection
```

## Authority boundary

SensibLaw owns legal proposition identity, applicability, support/defeater/comparator roles, authority semantics, source-role review, payment/promotion, and human/legal explanation.

SLR owns the binary transport and append-only PostgreSQL materialisation of an already-admitted typed LegalIR projection. It must not derive a legal conclusion from the fact that a row was persisted.

The existing Python `curated_legal_ir_flow.py` and historical `015_legal_ir_federation.sql` are semantic/reference fixtures. They are not the active production transport.

## Production constraints

- no JSON, JSONB, NDJSON, JSONL, or regex semantic parsing;
- fixed versioned binary LegalIR ABI;
- PostgreSQL scalar/TEXT[]/BYTEA storage only;
- append-only/idempotent `ON CONFLICT DO NOTHING` persistence;
- exact source provenance does not imply proposition truth or applicability;
- no hand-authored `seed_mabo.sql` graph;
- Mabo is the first fixture, not a special database ontology.

## Tasks

### 1. RED: typed materialiser tests

Add `crates/sl-legal-ir-materializer/tests/legal_ir_materializer.rs` before implementation. Require:

- `SLRI` v1 round-trip for SemanticBuild, Projection, Observation, GraphRevision;
- historical JSONB fields are represented as fixed binary observation payloads rather than JSON maps;
- v2 schema contains the four runtime tables and no JSON/UPDATE/DELETE;
- source-weld query requires one graph revision and the same semantic build/source revision/span;
- exact-source payment may become true while proposition truth/applicability remain false;
- Cargo dependencies contain neither `serde_json` nor `regex`.

Expected initial failure: crate/API does not exist.

### 2. GREEN: implement `sensiblaw-legal-ir-materializer`

Add a Rust crate with:

- magic `SLRI`, version 1;
- record tags 1 SemanticBuild, 2 Projection, 3 Observation, 4 GraphRevision;
- explicit little-endian length-prefixed fields;
- typed arrays for provenance/residual/span/jurisdiction/temporal refs;
- Observation body retained as a fixed binary body (`OBS1`) plus indexed scalar coordinates, never a generic JSON map;
- PostgreSQL `legal_ir.*_v2` tables with BYTEA payload/digest columns;
- append-only ingest;
- read-only exact-source weld query.

### 3. Thin Mabo runner

Add `scripts/run_slr_mabo_proof_graph.sh` as a control-plane wrapper only. It must:

- accept an admitted binary LegalIR stream rather than calling the legacy Python semantic runner;
- ingest through `sensiblaw-legal-ir-materializer`;
- verify the exact Mabo source/span weld through the Rust read path;
- emit a bounded receipt saying whether exact source provenance is paid;
- never set proposition truth or applicability paid.

The generic SLR research recurrence remains separate and supplies/reacquires evidence. The runner does not reimplement SensibLaw legal semantics.

### 4. Agda parity

Add a shallow DASHI owner `DASHI/Interop/SLRMaboLegalIRMaterialisationExact.agda` pinning:

- `SLRI` magic/version/tags;
- four typed materialisation kinds;
- append-only binary v2 storage;
- exact source weld requires source revision + span + same semantic build;
- `exactSourcePaid ->/ propositionTruthPaid` and `exactSourcePaid ->/ applicabilityPaid`;
- persistence is neither legal authority nor semantic promotion;
- active production path has no JSON/JSONB/regex parser.

Export through `DASHI.Interop.Everything`.

### 5. Verification gate

Do not stack legal applicability/promotion logic until local receipts establish:

```text
cargo test -p sensiblaw-legal-ir-materializer
cargo build --release -p sensiblaw-legal-ir-materializer
```

and focused Agda type-checking.

The first live GREEN target is deliberately narrow:

```text
source/span exists
semantic build v2 persisted
projection v2 persisted
observation v2 persisted
graph revision v2 persisted
exact_source_paid=true
proposition_truth_paid=false
applicability_paid=false
```
