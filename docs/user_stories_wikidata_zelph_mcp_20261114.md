# SLR production user stories — existing ITIR MCP / Wikidata / Zelph / NORA

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
