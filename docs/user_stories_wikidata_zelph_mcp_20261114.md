# SLR production user stories — existing ITIR MCP / Wikidata / Zelph / NORA

## New B5 raw native statement bytes -> typed SLR candidate crosswalk — 15 November 2026

**Latest operator exact-head preflight:** SLR #10 `4e0f150` cargo `sensiblaw-source-handoff` **19/19 PASS**; ITIR #14 `7385872` **120/120 Python + full native Cap'n Proto original-mini-bin route**; DASHI #1506 Agda **9/9**; SensibLaw #514 NORA **17/17**. These are local checked source-specific gates.

**New committed source not yet cargo checked:** existing SLR source-handoff crate exports `native_raw_receipt_join.rs`. It accepts a `NativeEntityRawReceipt` from the *actual offline raw-native full Wikibase JSON verifier* in ITIR, matches QID, full native statement GUID, property, exact native entity revision, raw SHA256, P854 candidate URLs and nonempty P14143 quantity/unit to the already typed `GraphNativeStatementCandidate`; all claimed S29/source payment/edit authority is rejected and even source/document true flags remain untrusted. It does not reparse/re-hash the raw entity bytes in Rust; the read-only output explicitly reports `original_http_response_authenticated_in_slr=false`, `primary_p854_documents_verified=false`, `s29_persistent_review_verified=false` and `source_support_paid=false`. Four focused Rust test functions source-written. The previous 19/19 Rust receipt does not certify them.

**ITIR new raw-byte verifier and original physical driver:** `verify_wikidata_native_entity_snapshot.py` has 19 isolated local adversarial tests; `run_zelph_local_physical_maxcut.py` has six isolated orchestration tests. Both require new clean exact-head ITIR run. Neither a self-computed expected SHA nor a typed witness object establishes upstream original authenticity. Full original native record means *full Wikibase JSON*, not the restructured REST statement response. Preserve exact P14143 native quantity, qualifiers, references, P854 URLs and the 1-to-N migration split.

**Still OPEN:** authentic native entity raw bytes and trusted origin, independent P854 document fetch/verification, actual period/unit/source semantics, graph-original revision correspondence, S29 PostgreSQL persistence, restart/reopen and downstream reviewer decisions. Reuse existing production owners, no new database or action scheduler.


## P4 B5 concrete source-bound native statement candidate — 15 November 2026

**New Rust implementation committed on existing SLR #10 (not yet cargo-checked):** `crates/sl-source-handoff/src/native_statement_evidence.rs`, exported by the existing `sl-source-handoff` crate, makes a *read-only* full statement candidate join from the existing `GraphNativeEvidencePair`, `StatementBundleReview` and independently supplied `NativeWikibaseStatementObservation`. It refuses QID/property/native revision/mainsnak/rank/qualifier/reference disagreement, missing native statement GUID/snapshot, malformed SHA256 claim and mismatched P854 URL values. Four focused Rust test cases exercise a successful retained candidate and multiple fail-closed drifts. This is **not** an assertion that native entity bytes, source document contents or provenance were acquired in this chat. `source_support_paid`, promotion and edit rights remain hard false; no GUID is generated from a Zelph numeric node ID.

**Independent downstream gates STILL OPEN:** read actual original native entity JSON at a pinned entity revision, SHA256 its complete raw byte response, parse Wikibase Action API's statement `id` + `mainsnak` + rank + qualifiers + references, preserve every 1-to-N case, and inspect independent primary P854 document bytes. Then join to S29 reviewed state and test actual PostgreSQL persistence, restart/readback, selective dependency reopen, held/rejected alternatives and operator review. `P14143` is Wikidata's *annual greenhouse gas emissions* quantity property; its amount, unit, point-in-time, method and scope qualifiers are semantically essential. A graph-only `left/right` path is not proof of a P14143 statement.

**Parallel A2 update:** ITIR #14 has a finite left/right word boundary checker, explicit original-bin `--node` numeric frontier routing and new tests. Golden DASHI #1506 has an inductive finite-word equality lemma. The NEW code has not yet received the clean-head Agda/ITIR/cargo acceptance receipts. Follow the [current golden A/B/C P0–P5 roadmap](https://github.com/chboishabba/dashi_agda/blob/agent/nora-perspective-consumer-maxcut-20261114/Docs/ZelphTrieHyperfabricNatNoraMaximumCut20261114.md). Do not create another SLR database, review engine or source-of-truth.


## Verified upstream offline 95/95 + physical/native-source boundary — 15 November 2026

**New operator exact-head receipt:** ITIR #14 at `e64c04f32f791502f6feccd3d918ece9a426c122` passes `bash tools/check_zelph_physical_maxcut_offline.sh` on a clean worktree: **95/95 Python** across ten modules plus compiled upstream-schema Cap'n Proto **four packed roundtrips and five fail-closed controls** using Clang/lld. Golden DASHI A1–A6 conditional Agda source remains checked at source commit `4c89edb` (7/7). This **closes only the offline input/decoder contract**. No real published HF byte-range equality or complete query-family closure has been reported.

**SLR P4 decisive obligations:** B5 `zelphGraphNativeStatementJoinIsEvidenceOnly` must consume independently sourced same-revision `(QID, statement GUID)` with native mainsnak, rank, amount/units, qualifiers/period, references and actual P854 source-evidence receipt; graph candidate route/one-hop edge equality does not pay a proposition. B6 `natZelphNoraReviewReplaySameSource` and C4 need persisted S29 review, restart/readback and dependency-indexed selective reopening while preserving held/rejected alternatives. C3 requires the real source-bound split receipts, not fixture-only Rust tests. Reuse existing SLR source and S29 owners without creating parallel stores.

**Run order:** ITIR real manifest/source-bin/range/native-selected receipt (P0–P2) and native Wikibase statement evidence can be gathered independently. Join only after source/revision/QID/GUID/consumer correspondence is established. Then do S29 durable replay. See the [authoritative A/B/C exact lemma and phase cut](https://github.com/chboishabba/dashi_agda/blob/agent/nora-perspective-consumer-maxcut-20261114/Docs/ZelphTrieHyperfabricNatNoraMaximumCut20261114.md) and [real physical execution record](https://github.com/chboishabba/ITIR-suite/blob/agent/nat-filtered-route-producer/docs/planning/zelph_original_bin_hf_ranges_maxcut_20261114.md).


## 2026-11-15 ITIR native-P0 verification linkage

Operator has reported ITIR's prior 58 regression tests plus 32 newer synthetic physical suites **all passing after two local worktree repairs** (90/90). The upstream `bcbf7c3` Git tree failed 9 of those before the fixes were committed. The originally validated native Zelph C++ decoder compiles on this workstation with `clang++ -fuse-ld=lld`; a genuine native packed fixture roundtrip and same-process rehash have since been source-written and remain **not yet run**. Run [ITIR's exact-head offline acceptance](https://github.com/chboishabba/ITIR-suite/blob/agent/nat-filtered-route-producer/tools/check_zelph_physical_maxcut_offline.sh) then execute the [real physical original-bin ↔ pinned HF witness](https://github.com/chboishabba/ITIR-suite/blob/agent/nat-filtered-route-producer/docs/planning/zelph_original_bin_hf_ranges_maxcut_20261114.md).

SLR's independent production gate remains unchanged: native Wikidata `(QID, GUID)`, qualified quantitative/period facts, P854 evidence and persisted S29 reviewed dependencies. Selected graph neighbourhood equality cannot pay that evidence. No new SLR database or source-of-truth duplicate is justified.




## Agda/kernel and physical stage gate (after 58-test reported ITIR replay)

Upstream ITIR PR #14's new raw-replay, full-manifest-listed name-cover and synthetic adjacency tests are **reported 58/58 green at head `a119add`**. This is stronger evidence of deterministic validation behaviour, **not** proof that the original graph's entire name universe or the real selected physical HF shards were inspected. DASHI PR #1506 now contains a corrected Agda source import chain and a pinned exact-head local kernel runner; the operator reported a clean **local patched-overlay** Agda 2.9 check, with current remote head replay still pending.

**SLR production acceptance still requires:** original native `(QID, statement GUID)` values and qualified P14143/P854 provenance, true graph/native source correspondence, persisted S29 review and restart, sparse dependency reopening, and the exact query/consumer scope. The four reported Rust tests pay the *candidate join* only. Maintain the prior single production Rust/PostgreSQL owner and avoid a second transport scheduler or review store.



## 2026-11-14 verification checkpoint / upstream A1–A2 development

Operator reported four passing `sensiblaw-source-handoff` candidate-join tests at PR #10 head `27c706c`. That validates the tested Rust source separation, **not** PostgreSQL S29 persistence, native P854 document inspection, the completeness of Zelph graph answers or external Wikidata actions.

[ITIR #14](https://github.com/chboishabba/ITIR-suite/pull/14) now adds: (i) exact raw four-file source/route/manifest/receipt replay, (ii) manifest name-cover structural audit, (iii) bounded original-bin one-hop `left/right` adjacency oracle, and (iv) byte-checked selected physical adjacency comparison. None of these new additions has a native replay receipt here yet.

Next SLR acceptance must ingest the *verified* same-object route identity, then independently bind the native `(QID,GUID)` statement, mainsnak, quantity/period/qualifiers/rank/P854 source and review dependency graph. Same-QID presence or source-bin one-hop equality never pays native source support. Reuse existing S29 persistence and selective sparse reopening; **do not** introduce another production database or acquisition scheduler.



Date: 2026-11-14. **Owner hierarchy:** DASHI/Agda golden formal contracts; SLR (Rust/PostgreSQL) production execution; ITIR-suite existing MCP tool-library and operator workflows; SensibLaw (Python) historical reference and Nat fixtures. This file cross-links rather than duplicates the [ITIR user stories](https://github.com/chboishabba/ITIR-suite/blob/agent/nat-filtered-route-producer/docs/user_stories.md).

## SLR-WIKI-US-01 — Production acquisition from an already registered ITIR MCP tool

As an SLR source/ontology operator, I want the production recurrent loop to reuse the [ITIR MCP registered tools](https://github.com/chboishabba/ITIR-suite/blob/main/itir-mcp/src/itir_mcp/governance_tools.py) and [tool registry](https://github.com/chboishabba/ITIR-suite/blob/main/itir-mcp/src/itir_mcp/registry.py) instead of recreating the same services in Rust or Python.

Existing source-contract owners:
- [SLR Nat/source-unit and complete Wikidata statement bundle PR #10](https://github.com/chboishabba/slr/pull/10) preserves main snak, rank, qualifiers, references, provenance, source revisions and exact coordinates.
- [SLR recurrent acquisition PR #28](https://github.com/chboishabba/slr/pull/28) owns bounded physical acquisitions and tiered transport; [SLR ontology PR #51](https://github.com/chboishabba/slr/pull/51) owns source-native disjointness/relational diagnostics; [SLR S29 PR #76](https://github.com/chboishabba/slr/pull/76) owns review and governance rather than a new NORA review database.
- [SLR production roadmap](https://github.com/chboishabba/slr/blob/main/docs/ROADMAP.md) demands physical-object coalescing, bounded width, restart/replay and explicit source/admissibility payment.
- [DASHI SLR physical-object acquisition policy](https://github.com/chboishabba/dashi_agda/blob/master/DASHI/Interop/SLRGWBPhysicalObjectAcquisitionPolicyExact.agda) and [Zelph bounded coverage](https://github.com/chboishabba/dashi_agda/blob/master/DASHI/Interop/ZelphBoundedGraphCoverageExact.agda) remain the golden formal references.

Available MCP intent-to-ABI mapping:

| Need | Existing ITIR MCP ABI |
|---|---|
| Source/statement review | `itir.wikidata.review_packet`, `itir.wikidata.object_review_bundle` |
| Candidate migration and disjointness | `itir.wikidata.migration_candidate`, `itir.wikidata.disjointness_report` |
| Governance and external tools | `itir.wikidata.tooling_profile`, `itir.wikiproject.tooling_profile` |
| Hosted Zelph manifest and typed partial-graph plan | `itir.zelph.hf_manifest_contract`, `itir.shard.route_selector`, `itir.shard.bounded_graph_slice_plan`, `itir.shard.payload_probe`, `itir.zelph.partial_closure` |
| Exact Nat QID name route (new, draft) | `itir.shard.exact_wikidata_name_route` — [ITIR #14](https://github.com/chboishabba/ITIR-suite/pull/14), explicit source sidecar+Wikidata language; no family-only fallback |

Acceptance: actual runtime must preserve source revision, object/digest/route provenance, statement-GUID identity and the distinction between **candidate graph observation**, **native Wikidata statement**, **independently verified P854 source support**, **reviewed semantic admission** and **operator-authorised edit**. Discovery or transport never implies later authority.

## SLR-WIKI-US-02 — Exact HF physical shard, budget and coverage

As a source worker, I want a specific QID resolved to an **exact physical object** for one immutable Zelph HF graph revision, with accurate bytes/GETs and a fail-closed fallback, not a best-effort wildcard lookup.

Start from [ITIR canonical HF locator](https://github.com/chboishabba/ITIR-suite/blob/agent/nat-filtered-route-producer/docs/planning/zelph_hf_wikidata_mcp_locator_20261114.md), [published dataset](https://huggingface.co/datasets/acrion/zelph) and [upstream selector semantics](https://zelph.org/sharding/).

Acceptance:
- Bind a dataset SHA, manifest+ETag/sha256, route-sidecar identity, selector (external QID and `lang=wikidata`), section-global chunk index, and physical shard object/digest as one artifact; reject mixed revisions.
- An unadvertised route index or unresolved QID is a positive **routing gap**, not an ontology negation. If routing fails, attempt the existing governed live Wikidata fallback with separate revision/provenance receipts.
- Execute direct Cap'n Proto shard decode with a pinned binary; never count sequential `.bin` fallback as direct success.
- Deduplicate physical requests **before** at-most-five cold-object scheduling; count physical remote GETs and downloaded bytes separately from logical QID requests.
- Test a healthy route, a QID not in a partial shard, a malformed section-global name index, a stale mutable manifest, and ambiguous/missing route metadata.
- `v2` historical median node transfer estimate 51.95 MiB, p95 60.63 MiB, max 700.53 MiB is *not* current accepted performance. `v3` bucket proposals are optimisation candidates, not a proven prerequisite.
- The old July 2026 missing-manifest observation was a **dated checkpoint**; upstream now documents canonical hosted paths. Exact present SHA and payload decode still require a live receipt.

## SLR-WIKI-US-03 — Review-first NORA and the full Nat programme

As a Wikidata editor, I want NORA's real reviewer dispositions to flow through **SLR's existing** Nat source-unit, S29 and ITIR review contracts without a second production runtime or an unreviewed edit.

Reuse [NORA author-facing handoff #514](https://github.com/chboishabba/SensibLaw/pull/514), [ITIR Wiki stories](https://github.com/chboishabba/ITIR-suite/blob/agent/nat-filtered-route-producer/docs/user_stories.md) and [Agda NORA formal candidate #1506](https://github.com/chboishabba/dashi_agda/pull/1506). Do not claim Agda kernel acceptance before a pinned checker run.

Acceptance: real NORA source/revision/fixture; exact same-source, same-QID and GUID mapping; rank/qualifier/reference preserving 1-to-N split; independent source-proposition adjudication; source-version scoped selective reopen; read-only candidate output. The original **57,989** Nat statements are historical programme scale, whereas the pinned five-QID/57-slot sample is only a regression fixture. Full population completion requires gap-free `(QID, GUID)` cursor exhaustion on the contemporary source.

## Reported live name-shard execution (2026-11-14)

An ITIR local operator has reported pinned full-graph HF manifest revision `0d4bd670959ebddae72f70d973410fdea8917ec1`, 21 targeted passing tests, direct C++ Cap'n Proto `nodeOfName` decoding for five Nat QIDs and a packaged exact route receipt. [ITIR physical-route report, including QID→node→chunk and sha256 receipts](https://github.com/chboishabba/ITIR-suite/blob/agent/nat-filtered-route-producer/docs/planning/nat_zelph_hf_physical_route_execution_20261114.md).

**These receipts are reported by the operator, not locally rerun in SLR.** Raw route/request/manifest bytes remain local. The original strict MCP route blocked as designed because no `nodeRouteIndex` was advertised; a separate physical shard scan established exact name mappings. Before SLR admission, verify physical source-manifest consistency, raw route digest and claimed producer lineage, decode adjacency/nameOfNode sections, collect bytes/GET metrics and bind native P14143 statement/qualifier/reference source family independently. This entity-linked graph does not carry all required quantitative or reference claims. S29 persisted same-source review and Agda golden kernel check remain unexecuted.

## Golden hyperfabric/trie execution alignment

The [full DASHI golden theorem and implementation cut](https://github.com/chboishabba/dashi_agda/blob/agent/nora-perspective-consumer-maxcut-20261114/Docs/ZelphTrieHyperfabricNatNoraMaximumCut20261114.md) now directly maps the physical Zelph transport to [balanced prefix ultrametric](https://github.com/chboishabba/dashi_agda/blob/master/DASHI/Foundations/BalancedTernaryUltrametricExact.agda), [residual-preserving coarse tree](https://github.com/chboishabba/dashi_agda/blob/master/DASHI/Foundations/BalancedTernaryResidualTreeExact.agda), [indexed hyperfabric](https://github.com/chboishabba/dashi_agda/blob/master/DASHI/Core/IndexedRelationalHyperfabricSpineExact.agda), [query-indexed adequacy](https://github.com/chboishabba/dashi_agda/blob/master/DASHI/Core/QueryIndexedProjectionAdequacyExact.agda) and [Nat nine-axis balanced admissibility](https://github.com/chboishabba/dashi_agda/blob/master/DASHI/Interop/SensibLawWikidataBalancedTernaryAdmissibilityHyperfabricExact.agda).

**SLR-specific obligations:** B4 physical object deduplication/budget, B5 graph/native statement evidence-plane join, B6 revision/GUID-scoped S29/ITIR/NORA replay, C3 native source and C4 native PG/kernel gate. Name-only QID routing cannot reconstruct quantitative P14143 values, qualifiers, periods, P854 source support or reviewed legal/semantic authority.

## Evidence / verification status

This document crosswalks existing source contracts and exact-route MCP handling. PR #10 now contains a source-written SLR Rust **candidate graph/native join and tests** (see implementation delta below), but it does **not** claim a live HF/PG/S29 run or source-support adjudication. Original source and code authorship remains with the respective contributors; do not attribute NORA to SLR or any Zelph implementation to NORA.


## 2026-11-14 concrete implementation delta — native/graph candidate join

On existing **SLR PR #10**, `crates/sl-source-handoff/src/zelph_graph_native_join.rs` is now registered under the existing source-handoff crate. `candidate_graph_native_join` checks subject QID, physical name-shard witness, graph revision and separately asserted native source revision correspondence. It transports bundle-level qualifiers and references without manufacturing native statement GUIDs from internal node IDs. Four Rust negative-control/unit tests cover evidence retention, source/QID mismatch, graph/native revision mismatch and missing physical routes.

The correspondence *receipt string* is a dependency for future verification, not a cryptographically checked graph-native identity proof. `GraphCoverage::DeclaredQueryFamily` is not original-graph completeness, and even a successful join remains `candidate_only=true`, with zero truth, source-support or edit authority. There is **no new PostgreSQL/S29 write** or P854 content adjudication in this change.

Local acceptance, at this PR's exact head:
```bash
cargo test -p sensiblaw-source-handoff
```

**Next decisive source/production cuts**: (1) import real ITIR raw digests + source/route object identities; (2) bind native Wikidata actual GUID/mainsnak/rank/period/quantity/qualifiers/P854 and revision proof; (3) register the existing S29 persisted review and sparse dependency-based reopen; (4) compare real saved/reloaded review with a pinned Agda certificate, and keep external NORA review as separately authorised. Reuse SLR #28/#57/#76; do not build another store, scheduler or invented NORA API.

[DASHI consolidated A1–A6, B1–B6, C1–C6 and P0–P5 owner roadmap](https://github.com/chboishabba/dashi_agda/blob/agent/nora-perspective-consumer-maxcut-20261114/Docs/ZelphTrieHyperfabricNatNoraMaximumCut20261114.md).
