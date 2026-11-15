# S29 live PostgreSQL original-source candidate/review integration (2026-11-15)

**Owner:** existing [SLR PR #10](https://github.com/chboishabba/slr/pull/10), stacked on #7, not a new repo/DB/container. ITIR #14 emits the original `source-bound-cohort-candidate.json`; DASHI #1505/1506 own mathematics; SensibLaw #514 owns historical NORA reference. No automatically sourced author/publication authority.

## What is implemented

`scripts/s29_source_review_postgres.py` uses *the supplied existing PostgreSQL DATABASE_URL* and psycopg v3 to persist a source-bound original ITIR candidate in a transaction. It creates versioned **bridging tables in the existing database**, only with explicit `--apply-schema`:

- `sl_s29_observations_v1`: canonical immutable JSON candidate with false source-payment/edit flags, SHA256 receipt;
- `sl_s29_sources_v1`: independent immutable original graph SHA and native Wikibase entity SHA, with their **different** revision spaces and source refs;
- `sl_s29_review_events_v1`: explicit bounded human reviewer disposition, no automatic approval or guessed reviewer;
- `sl_s29_dependency_v1`: exact source-to-consumer/query/policy dependency and pinned baseline revision;
- `sl_s29_reopen_events_v1`: exact changed-source revision → affected consumer only, idempotent on source receipt identity.

A source/ref revision with a conflicting raw-byte hash is rejected under a source-scoped advisory transaction lock. Missing dependencies trigger zero wakes (never a negative evidence claim). Reopen events are recorded, **not** automatic adjudication/promotion; the old pinned dependency remains until an explicit follow-up review.

`readback` opens an *independent second PostgreSQL connection after commit*, verifies the canonical candidate digest and exact two source-coordinate rows, then reads saved reopen events. This is genuine DB durability, not a Python dictionary clone. Still, success is only this **versioned bridge**, not proof of compatibility with a pre-existing legacy S29 table/consumer transaction interface; map that schema separately, without duplicating a production database.

## Run full source and PostgreSQL evidence

First obtain and check the original source under ITIR's [live operator runbook](https://github.com/chboishabba/ITIR-suite/blob/agent/nat-filtered-route-producer/docs/planning/zelph_live_original_cohort_operator_runbook_20261115.md).

```bash
# This URL must point to the ALREADY AVAILABLE PostgreSQL instance.
# Do not create or start a new DB/container for this acceptance.
export DATABASE_URL='<existing-postgresql-connection-string>'
python3 -m pip install 'psycopg[binary]>=3,<4'

python3 scripts/s29_source_review_postgres.py \
  --candidate /path/to/verified-cohort/source-bound-cohort-candidate.json \
  --apply-schema \
  --consumer-ref 'nora:climate-review' \
  --query-ref 'nat:Q10403939:P14143' \
  --policy-ref 'candidate-only:v1' \
  --receipt /path/to/new-slr-source-db-readback.json
```

No `--reviewer-ref` means **no human review is invented**. To attach one genuine human disposition, an operator can additionally pass `--reviewer-ref`, `--disposition held|rejected|ambiguous|supported-candidate` and `--rationale`; even `supported-candidate` remains candidate-only without source verification and proper separate authorization.

Run existing source crate and new pure tests:

```bash
cargo test -p sensiblaw-source-handoff
bash scripts/check_s29_source_handoff.sh
```

To test **actual PostgreSQL commit→close→reconnect→revision-driven reopening**, set a disposable test database connection *in the same existing PostgreSQL server*, ideally a dedicated test schema/DB identity approved by the operator. This test writes versioned fixture rows with clearly marked test QIDs; it leaves them for inspection.

```bash
export SLR_S29_TEST_DATABASE_URL="$DATABASE_URL"
export SLR_S29_REQUIRE_REAL_PG=1
bash scripts/check_s29_source_handoff.sh
```

**No `SLR_S29_TEST_DATABASE_URL`, no real P5 PASS**: the live case is skipped by the Python unit-test runner; the wrapper explicitly requires it when `SLR_S29_REQUIRE_REAL_PG=1`.

## Acceptance matrix

| Gate | What closes it | Current scope |
|---|---|---|
| S29-A source parsing | candidate schema and SHA, duplicate JSON refusal, no authority claims | Python pure offline tests |
| S29-B actual commit/readback | actual psycopg transaction plus separate PostgreSQL connection replay | Live operator test required |
| S29-C sparse revision reopen | change only one source revision; wake only subscribed graph/native fibre; unrelated unchanged | Live operator test required |
| S29-D human judgment | explicit reviewer action and separately reviewed primary P854 document | Operator/primary-source action required |
| S29-E production legacy consumer | map to pre-existing S29 schema/semantics and running SLR consumers | Separate integration; not inferred by these bridge tables |

**False under all automatic paths:** `source_support_paid`, `creates_world_truth`, `edit_authority`, native HTTP provenance, P854 primary-document semantic support, original Wikimedia FLN↔Zelph graph identity.

## Source-owner distinction

SLR's Rust `sensiblaw-source-handoff` crate stays I/O-free. This Python tool is an opt-in *integration adapter* for the already configured PostgreSQL endpoint, not a fabricated Rust database driver or claim that unrelated S29 production tables were patched. The existing `sensiblaw-evidential-reopen` Rust dependency contract is the semantic owner for sparse consumer reopening; the SQL record faithfully stores an explicit reverse dependency and revision-triggered wake evidence for later production wiring.
