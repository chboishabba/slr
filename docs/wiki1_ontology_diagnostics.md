# WIKI-1 — general Wikidata finite-ontology diagnostic → S29/M10

**Authority boundary:** Original Lean code was authored by JMD
(`github.com/meta-introspector`) and is preserved in
`dashi_lean4/DASHI/output-final_aristotle/RequestProject/`.
DASHI's JSON packet, SLR persistence, UI and Agda bridge are **new
integration work**, not a new attribution claim for JMD's definitions.
An executable finite-KB theorem does not establish the state of live
Wikidata. Source references are neither semantic identity nor truth.

## Existing sources (do not rebuild)

- `Diagnostics.lean` — finite-KB error/warning report and concrete
  witnesses, including dangling items, metaclass mismatches, cycles,
  disjointness and redundant subclass edges.
- `ConstraintSuite.lean` — qualifier, scope, range, format,
  cardinality and contemporary constraints. Its documented
  `contemporary_not_truthy_closed` counterexample means truthy and
  full-graph checks MUST retain separate graph-view coordinates.
- `RepairWorkflow.lean` and `RepairReview.lean` — advisory
  modeled patches, before/after debt and no-regression check. Their
  `proven` verdict is **relative to the modeled finite KB** and is
  not authorization for a public edit.

The generic SLR source revision owner supplies native text, content
hash, provider and acquisition receipt. WIKI-1's ontology review
packet carries the original checker output bytes/digest, pinned Lean
commit, producer run and execution receipt, exact native statement
GUIDs when supplied, ranks, qualifiers, references and missing
obligations. If original statement IDs are absent, the adapter
returns `undetermined` rather than inventing a source match.

`OntologyDiagnosticPacket` has these critical coordinates:

- source revision + source snapshot digest;
- exact Wikidata entity ref and **full_statements**, **truthy_rank**
  or explicitly identified **scoped_slice** graph;
- finite checker owner (`Diagnostics`, `ConstraintSuite`,
  `RepairReview`) and code commit;
- producer execution ref and output digest checked against retained
  literal checker output;
- JMD original attribution and separate DASHI integration ref;
- concrete witness refs and native statement bundles;
- uncertainty/residual obligations and advisory repair candidates;
- immutable non-promotion and no-public-edit flags.

The store's `persist_ontology_diagnostic` reopens the pre-existing
`ingest.generic_source_revision` and validates the supplied digest
against its canonical text. It does **not** create a new source or
replicate a parser. Only afterward does it create/reopen a
`semantic.wikidata_ontology_diagnostic` sidecar and an ordinary
`ReviewItemKind::OntologyDiagnostic` S29 item. The unchanged S29
reducer owns workflow statuses, reviewer attribution, row locking
and receipt persistence. `apply_ontology_diagnostic_review` reopens
the packet after the S29 action; neither accept nor a modeled repair
verdict pays truth or edit permission.

## JMD repair-review CSV producer adapter

The baseline adapter accepts the **actual** output from
`RequestProject.RepairReview.reviewCsv` and a native Wikidata
statement mapping. It does not execute Lean or manufacture a kernel
receipt. The caller must supply genuine run and acquisition records
from the actual producer.

```bash
python python/wiki1_jmd_repair_review_adapter.py \
  --lean-review-csv ./actual-jmd-review.csv \
  --canonical-source ./persisted-source-utf8.txt \
  --statement-map ./statement-guids-and-qualifiers.json \
  --source-revision-ref 'source-revision:...' \
  --wikidata-entity-ref 'Q...' \
  --lean-source-commit '<REAL LEAN SOURCE COMMIT SHA>' \
  --producer-run-ref '<REAL EXECUTION REF>' \
  --producer-receipt-ref '<REAL EXECUTION RECEIPT REF>' \
  --graph-view full_statements \
  --output-jsonl ./wiki-1-diagnostics.jsonl
```

The mapping is indexed by **zero-based review CSV row number**:

```json
{
  "0": {
    "native_statements": [
      {
        "statement_ref": "Q...$REAL-GUID",
        "subject_ref": "Q...",
        "property_ref": "P279",
        "value_ref": "Q...",
        "rank_ref": "normal",
        "qualifiers": [],
        "reference_refs": [],
        "statement_revision_ref": "revision:..."
      }
    ],
    "evidence_refs": ["source-revision:..."]
  }
}
```

Empty `native_statements` yields an explicitly `undetermined`
packet with a missing GUID obligation; a QID is not a statement GUID.
The adapted packet retains the exact CSV row payload serialized
deterministically. The digest is of that serialized row and is not
misreported as the checksum of the entire CSV file. A `proven`
modeled verdict is kept as such; it is **not** called a verified edit.

Import into migrated PostgreSQL:

```bash
cargo run -p sensiblaw-pg-source-store \
  --example wiki1_ontology_diagnostic_import -- \
  ./wiki-1-diagnostics.jsonl 'matter:explicit-consumer-scope'
```

The importer uses the already-connected DB configuration and
persists via the actual generic source/S29 stores. No direct
Wikidata network call, patch application, or live edit is performed.

## S30 Matter and Dioxus

The S30 Matter projection filters `OntologyDiagnostic` review items
unless **every** native witness/source coordinate is visible. Dioxus
requires an existing `SENSIBLAW_MATTER_SCOPE` and checks the actual
`project_matter_context` membership for the original source and
all witness statement refs **before displaying** a packet or
submitting any S29 review action.

Dioxus's environment-gated entrypoint is
`ITIR_WIKI_DIAGNOSTIC_REF=<persisted diagnostic ref>`. It shows:
graph view; pinned execution/model; full statement/qualifier/rank
bundles; diagnostic witnesses; missing obligations; proposed modeled
repair and S29 status/dispositions. There is no Edit Wikidata button
and no separate review persistence or semantic transport.

## Physical acceptance obligations

1. Verify the actual JMD Lean owner on the exact git head and
   capture a real executable checker output plus *independently
   identifiable* run/kernel receipt (a citation is not a run).
2. Hydrate revision-pinned full Wikidata statement GUIDs, ranks,
   qualifiers and references; retain gaps explicitly.
3. Persist the source and packet, reopen original bytes, validate
   deterministic digest/identity and S29 history.
4. Compare **full vs truthy** on a real regression specimen;
   do not substitute one checker output for the other.
5. Enforce S30 scope isolation, redaction, no cross-source leaks and
   dual-UI/graph selection where appropriate.
6. Test the real Nat P5991→P14143 packet as a later *application*
   with its independent statement-family, split, unit, source,
   governance and after-state checks.

These are unexecuted acceptance conditions. This tranche is
source-written; it is not Lean/Agda kernel, PostgreSQL or public edit
certification. No SCALE-1 economy claim follows from WIKI-1.
