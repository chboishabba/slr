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


## WIKI-EP-1 — Epistemic deliberation/source history (2026-11-01)

Roszkowski's [2024 *Journal of Documentation* study](https://doi.org/10.1108/JD-01-2024-0008) and [2026 CEE conference abstract](https://meta.wikimedia.org/wiki/Open_Knowledge:_Wikimedia_%26_Research_in_the_CEE_Region/Submissions/%E2%80%9CDone._Make_Good_Use_of_It.%E2%80%9D_Epistemic_Practices_Behind_Wikidata%27s_Ontology_Evolution) motivate retaining *initiation → knowledge work → closure* and the institutional/social division of epistemic labour. **That historical and sociological account belongs to Roszkowski**; the following typed validator, holds and S29 bridge are new SLR implementation, not his formal theorems.

`epistemic_deliberation.rs` exports `DeliberationPacket`, `assess_deliberation`, and `project_s29_diagnostic_review_item` through the existing `sensiblaw-pg-source-store` crate API. One packet holds the proposal identity, **historical policy revision**, caller-supplied minimum-age rule, source-bound ordered proposer/support/objection/amendment/decision events, event actor and immutable source-revision refs. A recorded consensus or actor-authority ref is an observation to verify, **not** evidence of actually verified consensus/authority. Event timestamps are producer-supplied Unix seconds, never inferred from revision IDs.

The validator rejects malformed/duplicate/nonchronological events, unknown or self-referential amendments, missing event source refs, decisions outside the terminal position and decision fields on nondeliberative events. It separately holds missing independent participation, pending closure, self-approval, policy age, missing consensus/authority references, and absence of observed property identity for a reported approval. An objection remains visible even when the source contains an amendment. Negative decisions do not require a created property. A missing decision is not deemed rejection.

The resulting `DeliberationAssessment` is **record completeness only**. `record_reviewable=true` does *not* verify the community's historical policy, authenticating actor roles, a source revision, consensus, property creation or arbitrary instance truth. Every result permanently fixes `candidate_only=true`, `community_consensus_verified=false`, `actor_authority_verified=false`, `semantic_truth_promoted=false`, `wikidata_edit_authorised=false`, and `ao_dispatch_allowed=false`.

`project_s29_diagnostic_review_item` returns the **existing** `ReviewItemKind::OntologyDiagnostic` carrier with original event-source/policy references and an explicit consumer scope. It offers evidence request, qualification, abstention and source-opening **only**; it does not offer approval of the external Wikidata proposal. The ordinary S29 reducer/store remains the workflow owner, and WIKI-1 source/scope verification remains prerequisite for materialization. The new projection is not a new PostgreSQL store, parser or source-authority service.

ITIR-suite consumes the same logical record as a pure read-only MCP governance assessment (`itir.governance.deliberation_assessment`) for bounded AO diagnostic/hold planning. Its ISO-8601 timestamps and this module's Unix-second timestamps are **different producer wire representations**, requiring explicit source-backed conversion rather than a fictional same-object digest or automatic cross-language receipt match. AO's own `frontier_reference_validation` still does not accept this record as a verified repository/formal/operation reference, and no dispatch is triggered.

**Verification:** `cargo test -p sensiblaw-pg-source-store epistemic_deliberation` (not run via GitHub connector). The physical SLR source lookup and S29 persisted review replay code plus disposable-PG acceptance regression are now source-written, **not executed**. A real historical `P10322` source corpus, run receipts, and Agda kernel receipt remain outstanding; the unit regression uses fictional event refs rather than inventing historical witnesses. JMD Lean/Wikidata original code ownership and the source-pinned WIKI-1 model/real-world distinction above remain unchanged.

### Existing Wiki/OCTF user-story integration and work not yet paid

This proposal-history model should **consume** existing canonical source/revision and S29 evidence rather than define a second repair/approval pipeline. Reviewed concrete source paths:

- ITIR `docs/user_stories.md`: **ITIR-US-17** revision-locked wiki/followed-source assist; **SL-US-15/16** contested public-figure and claim source moderation; **WIKI-US-01–04 / SL-US-38–41** statement review, tooling governance, ontology/disjointness triage and cross-corpus wiki reconciliation.
- SensibLaw `docs/wikidata/README.md`: five runnable cases (climate `P5991→P14143` migration, PNF/residual climate review, Rosario-style hotspot packs, Ege/Peter `P2738/P11260` contradictions, review-only `ChangeReviewPacket` compare-candidates).
- SensibLaw `docs/planning/wikidata_octf_entrypoint_20260421.md`: explicit Dave/general OCTF, Peter, Ege and Rosario handoffs.
- ITIR `docs/planning/wikidata_assist_lane_reviewer_packet_alignment_20260401.md`: existing Nat-style assist packet fields `culprit_summary`, `qualifier_reference_index`, `progress_claim=reviewable_packet`, `promotion_guard=hold`.
- ITIR `docs/planning/generic_world_model_compiler_convergence_20260716.md`: one generic source/world-model/review/receipt owner and domain-specific profiles rather than duplicate Wikidata code.

**What is implemented:** source-native *deliberation completeness*, deterministic event/source identities, explicit objection/amendment/terminal-decision history, separate policy and attribution refs, and a pure S29 `OntologyDiagnostic` review-item constructor. This is useful for presenting the social/policy provenance of a **new-property obligation** suggested by ChangeReviewPacket, or a **contested proposed rule** following a P2738 violation.

**What is not implemented:** consuming persisted `ChangeReviewPacket` or `sl.wikidata_signal_review_bundle.v0_1` artifacts, linking a real property-creation revision to a real proposal archive, altering the climate A–E action families, importing P2738 pairs/culprits, or linking Rosario benchmark runs. Those workloads each have their own source-specific contracts and must not be treated as evidence of consensus. In particular, the broader repair policy `severity(after) <= severity(before)` from the SensibLaw OCTF roadmap is a *bounded structural coherence criterion*, not a substitute for community legitimacy or statement truth. Neither this helper nor the ITIR consumer asserts full global-index monotonicity.

**Current integration cut:** a **separate** `semantic.epistemic_deliberation_diagnostic` sidecar has now been implemented through `epistemic_deliberation_store.rs`. It is *not* a second semantic or source store: it reopens the **existing generic source** bytes with exact content digest/acquisition receipts, requires a cryptographically matched **event excerpt byte interval** per proposal/comment/decision, and persists a candidate diagnostic through the **existing S29 review table and reducer**. `persist_deliberation_diagnostic`, `load_deliberation_diagnostic_scoped` and `apply_deliberation_diagnostic_review` are the canonical SLR API. Scope is bound into diagnostic/review identity; the UI still must independently enforce S30 recipient/role/purpose/knowledge-cut membership. The older WIKI-1 `persist_ontology_diagnostic` remains exclusively for actual externally executed JMD checker outputs and must never receive fabricated `lean_source_commit`, `producer_run_ref` or `raw_checker_output` from community debate. No new Wikidata edit/approval action exists.

**Downstream acceptance:** pin a real P10322 proposal revision and every claimed actor/time source; preserve contested comments and amendments; read/reopen all event sources through SLR; keep S30 consumer-scope isolation; compare with a real change-review or P2738 case without claiming a same-object identity; and run native tests. Do not expose a phantom approval or external edit button merely because the S29 diagnostic was accepted.


### New physical replay acceptance and cross-repository formal bridge

`crates/sl-pg-source-store/tests/epistemic_deliberation_replay.rs` is a source-written **opt-in** PG integration test. It runs only when `SLR_DELIBERATION_PG_ACCEPTANCE=1` and `DATABASE_URL` points to a **disposable, pre-migrated** test database with canonical corpus tables. It creates fictional source revisions through `persist_generic_text_source`, reopens exact content/byte spans, checks idempotent replay and negative checksum/scope witnesses, requests additional evidence with S29, and reopens the same status. No public Wikidata data is touched; the test has not been executed in this GitHub-only session.

ITIR-suite [PR #70](https://github.com/chboishabba/ITIR-suite/pull/70) now supplies a **candidate-only** `deliberation_slr_handoff` with exact whole-second timezone conversion, source seal and event-span records matching this Rust ABI. Strings and hashes supplied to ITIR are not authenticated until this SLR byte-reopening path. Its generic `deliberation_consumer_link` links only explicitly selected property/consumer references and cannot pay `ChangeReviewPacket` structural correctness, Nat climate A–E migration gates, Ege/Peter P2738 contradictions or Rosario hotspot scores.

DASHI Agda [PR #1447](https://github.com/chboishabba/dashi_agda/pull/1447) reuses `EpistemicInquiryGovernance` and `WikidataDiagnosticGovernanceExact` and states conditional independent source, creator, consensus, historical policy and age obligations. This is **source-written, unverified** Agda, not a certified correspondence theorem between Rust/Python data and community governance. A real P10322 historical proposal/policy corpus, source-to-semantic interpretation and S30-scoped consumer replay are still separate acceptance gates.

**Atomicity correction (2026-11-01):** `persist_deliberation_diagnostic` now calls `review_workstation_store::persist_review_projection_on_tx` and inserts the deliberation sidecar in **one PostgreSQL transaction**. A sidecar failure rolls back the newly inserted S29 item and its source/action/provenance references. This is an S29-owned generic transactional helper, not a second review implementation. The optional physical regression remains not yet executed.


### Real Wikidata source capture to S29: direct native intake

The SLR PR now exposes `DeliberationSourceInput { envelope: SourceIngestEnvelope, canonical_text: String }` and `persist_deliberation_from_source_inputs`. It validates exact source revision coverage (proposal, the then-effective policy and every discussion event), Wiki/ContentSource roles, unique source IDs and source SHA-256 **before writes**. It calls the existing generic INGEST-1 canonical writer for every source and then the same original `persist_deliberation_diagnostic` byte-span checker and **single S29+sidecar transaction**. There is no new canonical corpus store, S29 reducer or Wikidata edit authority.

SensibLaw [PR #511](https://github.com/chboishabba/SensibLaw/pull/511) now has the original archived P10322 community history at oldid `1572799769` and uses its *existing* read-only MediaWiki pull helper to fetch exact original wikitext and an as-of historical policy revision, then project a native source-handoff envelope through its `history-snapshot-handoff` CLI command. ITIR-suite [PR #70](https://github.com/chboishabba/ITIR-suite/pull/70) validates that envelope through `itir.governance.wikidata_source_replay_handoff`, reusing generic native packet/byte anchor validation. This supplies the source-compatible field layout to SLR; source text and `SourceIngestEnvelope` must still be independently supplied to INGEST-1 for actual persistence/reopening.

**Unpaid:** real post-acquisition source snapshots, policy applicability at 2022-02-04, Jura's actual property-creator role, independently weighed opposition/consensus, real P10322 property-creation log, S30 matter-authorized propagation and actual execution of the opt-in physical PG test. No code output alone pays those obligations; historical exact byte hashes and acquisition/network receipts have not been physically verified in the session.
