# SLR production user stories — existing ITIR MCP / Wikidata / Zelph / NORA

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

## Evidence / verification status

This document crosswalks existing source contracts and newly proposed exact-route MCP handling. It does **not** claim a new SLR Rust implementation or a live HF/PG run in PR #10. Original source and code authorship remains with the respective contributors; do not attribute NORA to SLR or any Zelph implementation to NORA.
