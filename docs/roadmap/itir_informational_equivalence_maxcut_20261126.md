# ITIR informational equivalence, wiki repair and automatic proving — P0 sprint

Date: 2026-11-26. Status: ACTIVE integration baseline; production acceptance OPEN.
This file is the controlling sprint roadmap. Every implementation commit must
update the affected deliverable dispositions, exact owner references, and test
or evidence status. Do not mark conditional mathematics as production evidence.

## 0. Objective and definition of complete

Deliver a generic proof-directed informational equivalence, reconciliation and
repair workflow serving ITIR user stories. GWB is a real integration specimen,
not the product boundary. Given source revisions and a consumer question, the
system obtains original observations, compiles native relational/PNF structure,
constructs applicable proof or exact typed obstruction, compares independent
sources, generates governed repair/acquisition candidates, and reopens only
dependent results when evidence changes. It retains reversible source and
proof coordinates. It must work across language editions, Wikipedia, Wikidata,
Wiktionary, legal records, transcripts, books and other producer families.
Do not demand that every empirical claim be decidable or globally resolve all
ambiguities: an explicit, correctly scoped hold is a legitimate outcome.
An all-hold implementation is not completion: demonstrate positive licensed
answers, negative controls, and actual repair/replay on original sources.

Product authority: [ITIR user stories](https://github.com/chboishabba/ITIR-suite/blob/main/docs/user_stories.md),
[PNF hierarchical fibres / Wikimedia bridge](https://github.com/chboishabba/ITIR-suite/blob/main/docs/planning/rpd_pnf_hierarchical_fibres_slr_wikidata_bridge_20261030.md),
[Wikidata Nat + Assist](https://github.com/chboishabba/ITIR-suite/blob/main/docs/planning/wikidata_combined_roadmap_nat_and_assist_20260401.md),
[progressive proof workbench](https://github.com/chboishabba/ITIR-suite/blob/main/docs/planning/progressive_proof_explanation_workbench_20260915.md).

- ITIR defines user outcomes; Agda and Lean are golden formal specifications.
- SLR Rust/typed PostgreSQL is production; SensibLaw Python is historical
  prototype, behavioral donor and comparator ONLY. No new prototype runtime.
- JMD's original executable Wikipedia/Wikidata importer, property checker,
  derivation and review machinery remain attributed to JMD; DASHI owns its
  actual extensions/transport, not the original discoveries or algorithms.
- ITIR/MCP and itir-dioxus expose the existing governed read/write boundaries.
  Reuse the established workbench and DA51 artifact/referee interfaces.
- Geometry-forward, weld-first, steelman, bidirectional, reverse-solve:
  recover actual theorem bodies and existing constructors on relevant PRs and
  default branches before claiming a missing lemma. Preserve the zero mode,
  signed residual, source/revision/language/context/consumer/time/scale indices,
  typed hyperfabric restrictions and inverse fibres. No invented wordlists,
  arbitrary scalar collapse, proof by similar names, or duplicate engines.
- Raw UTF-8 source digest is distinct from normalized content digest. Preserve
  exact original manifestations, source layers, immutable revision and span
  coordinates, including all qualifiers/ranks/references and provenance.
- Evidence of same entity, same manifestation, same proposition, semantic
  equivalence, independently verified truth and lawful edit authority are
  different relations. No autonomous live Wikidata/Wikipedia writes.
- Proofs establish their actual conclusions under their actual premises.
  Native compilation, checker execution, external evidence and database
  transactions need separate verification; never claim them from a theorem.

## 1. Integration baseline and disposition

SLR current main at audit: `3c01278943e73f26fa0cb3c11136cfd2229af9f7`.
Shared ancestor for #78/#76/#72: `1db8aeee801708c98f76ed94c064942cba6798d9`.
Main's sole subsequent commit adds PRODUCER-CONFORMANCE-1; its entire section
is already in #78 except the deliberate clarification that both Agda and Lean
are golden. Retain that clarified wording.

| SLR donor | Pinned head | Disposition |
|---|---|---|
| [#78](https://github.com/chboishabba/slr/pull/78) | 531e177b0b0964ac9e5f39809c89e25ff3b63d2a | Integration branch parent: all native PNF, checked M12, identity owner, original receipts, formal bridge, GWB roadmap; prior bounded reporting specimen retained pending genericization. |
| [#76](https://github.com/chboishabba/slr/pull/76) | cc928e008734fea63fbda098dda16c16700eb520 | Incorporated exact six-path delta: epistemic deliberation, transactional review sidecar, S29 writer, tests, WIKI diagnostics, shared lib exports. |
| [#72](https://github.com/chboishabba/slr/pull/72) | c5cdb8af069cb2c4230b5a7d5db86c2f8a8751a6 | Incorporated exact six-path delta: declared-domain manifestation coverage, document PNF overlap, tests/docs, shared lib exports. |
| [#55](https://github.com/chboishabba/slr/pull/55) / [#54](https://github.com/chboishabba/slr/pull/54) | 1b5275a9a845 / 964adf1e7599 | Both heads confirmed ancestors of current main despite open PR state. Reuse persisted REL acceptance and real INV/reopening examples, no re-port. |
| [#44](https://github.com/chboishabba/slr/pull/44), [#42](https://github.com/chboishabba/slr/pull/42), [#22](https://github.com/chboishabba/slr/pull/22), [#18](https://github.com/chboishabba/slr/pull/18) | 7ec10d20c8d1 / 2a26e6d54327 / 03518443cc99 / 1712885cb53d | Confirmed ancestors of main: chronology, typed producers, native semantic reader; do not duplicate. |
| [#53](https://github.com/chboishabba/slr/pull/53), [#52](https://github.com/chboishabba/slr/pull/52) | 413b46862e77 / 5e8d81c3b309 | Merged REL→INV→GOV graph binding, control case, Pareto and workbench. |
| [#62](https://github.com/chboishabba/slr/pull/62), [#67](https://github.com/chboishabba/slr/pull/67), [#69](https://github.com/chboishabba/slr/pull/69) | 360b8570998e / 91645ea4878b / 6d496e07582c | Merged representation parity, exhaustive HCA PNF manifestation comparison, reviewed same-Matter and residual history. |
| [#65](https://github.com/chboishabba/slr/pull/65), [#70](https://github.com/chboishabba/slr/pull/70) | 6d3656efce41 / 006e795df77d | Merged authenticated formal routing/reopening and persisted source-exact Mabo proof graph. |
| [#45](https://github.com/chboishabba/slr/pull/45), [#41](https://github.com/chboishabba/slr/pull/41), [#27](https://github.com/chboishabba/slr/pull/27) | f245575a5aad / 1a793af93d2d / 0181535427a3 | Merged event/review workflow, M12 intake, bounded external Wikidata closure. |
| [#57](https://github.com/chboishabba/slr/pull/57), [#56](https://github.com/chboishabba/slr/pull/56), [#61](https://github.com/chboishabba/slr/pull/61) | 1b0bdbd99161 / 7ea29229d2e5 / 9c41abec6181 | Merged source materialization, reviewed durability, external legal binding. |
| [#59](https://github.com/chboishabba/slr/pull/59) | 53a59041ebc3b6cdbe7a0d5e69e3941537da4d96 | Divergent 116 ahead/115 behind at audit. Typed controversy, reverse proof and Woogaroo signed synthesis donor. Reconcile actual current owner paths against merged #60/#69/#70; do not blindly replace current interlingua/workbench. |
| [#77](https://github.com/chboishabba/slr/pull/77) | e8cd02fe3127e2ce7ad52bb624d097b71cdf51ea | Separate X369 PG grammar export Python sidecar; optional producer/conformance donor, not a prerequisite or canonical semantic runtime. |
| [#73](https://github.com/chboishabba/slr/pull/73), [#71](https://github.com/chboishabba/slr/pull/71), [#75](https://github.com/chboishabba/slr/pull/75) | 87665e8a0ad9 / dcaffe6d1d89 / e443a7ffb11c | Relevant archive/candidate-only/fleet producer lanes; audit exact current contracts before porting, outside core wiki proof path. |

Discovery scope: all 78 listed SLR PRs, latest 100 SensibLaw PRs plus
targeted PR searches, selected changed-file inventories and source reads.
This is not an exhaustive body-level audit of all historical PRs; complete
remaining owner reconciliation within P0.1, not by inventing missing code.

### SensibLaw donor ledger (no Python runtime extension)

| Donor PR | Recover behavior / exact owner |
|---|---|
| [#486](https://github.com/chboishabba/SensibLaw/pull/486) b38abf144856 | `src/ontology/lean_wikidata_certificate.py`: LeanOntologyCertificate, compare_external_relation, load_certificate_packet; source contract and original JMD snapshot. Port to existing SLR formal/ontology bridge, never trust textual flags. |
| [#495](https://github.com/chboishabba/SensibLaw/pull/495) e6ed9dfab9ac | `src/ontology/wikimedia_world_walk.py`: bounded walk_world, WorldWalkPolicy, edge_candidates_from_wikidata_bundles, build_publishable_bucket_manifest. Recover world-walk behavior, typed direction and original statement references. |
| [#511](https://github.com/chboishabba/SensibLaw/pull/511) 674bf12d60cc | `src/ontology/wikidata_signal_review_bundle.py`: build_disjointness_bundle, attach_deliberation_context, project_historical_source_anchors, build_historical_source_handoff. Align with SLR #76; actual P10322 historical source anchors. |
| [#514](https://github.com/chboishabba/SensibLaw/pull/514) 345b6f12c8dd | `src/ontology/wikidata_nora_review_handoff.py`: build/replay_nora_review_handoff, check_nora_revision, nora_acquisition_route, plan_nora_source_bound_persisted_review. Nat/NORA original source, user-story and governed review behavior. |
| [#441](https://github.com/chboishabba/SensibLaw/pull/441) e97051c64320 | `src/ontology/wikimedia_providers.py`: WikidataProvider, WiktionaryProvider, WikimediaMicrobatchRunner, request/snapshot receipts; enrichment and bounded legal follow. |
| [#474](https://github.com/chboishabba/SensibLaw/pull/474) f527fd1f987e | `src/policy/reopenable_runtime.py`: SignedEvidence, CandidateKey, progressive_signed_residual, assert_same_candidate_fibre, find_terminalisation_witness; migrations 086–088. |
| [#475](https://github.com/chboishabba/SensibLaw/pull/475), [#470](https://github.com/chboishabba/SensibLaw/pull/470), [#485](https://github.com/chboishabba/SensibLaw/pull/485) | Consumer-sufficient numeric runtime, numeric streaming PNF, work-conserving execution; large donor deltas, reconcile against #78 actual owner/migrations. |
| [#515](https://github.com/chboishabba/SensibLaw/pull/515), [#489](https://github.com/chboishabba/SensibLaw/pull/489), [#510](https://github.com/chboishabba/SensibLaw/pull/510) | Raw UTF-8 digest, numeric spaCy evidence export, content/profile/version cache keys. Compare native source correspondence, not second parser. |
| [#506](https://github.com/chboishabba/SensibLaw/pull/506), [#483](https://github.com/chboishabba/SensibLaw/pull/483), [#481](https://github.com/chboishabba/SensibLaw/pull/481), [#476](https://github.com/chboishabba/SensibLaw/pull/476) | Resolved-or-explicit targeting, trigger/target provenance, residual-driven H6/H9. Reconcile exact demand keys and current owner. |
| [#493](https://github.com/chboishabba/SensibLaw/pull/493), [#478](https://github.com/chboishabba/SensibLaw/pull/478), [#498](https://github.com/chboishabba/SensibLaw/pull/498) | Nat prerequisites/partial acquisition, Zelph snapshot, GOV-1 information governance/CAPA. Review upstream contracts before native integration. |
| [#464](https://github.com/chboishabba/SensibLaw/pull/464), [#461](https://github.com/chboishabba/SensibLaw/pull/461), [#459](https://github.com/chboishabba/SensibLaw/pull/459), [#439](https://github.com/chboishabba/SensibLaw/pull/439), [#437](https://github.com/chboishabba/SensibLaw/pull/437), [#436](https://github.com/chboishabba/SensibLaw/pull/436), [#434](https://github.com/chboishabba/SensibLaw/pull/434) | Unified source/proof fibres, graph operators, constraint fixed point, candidate sets, semantic algebra, typed resolution, review grounding. Audit against native current owners; no automatic assumption every Python feature is absent. |

### ITIR and formal owner pins

- ITIR #55/#56 merged PNF ontology routing and generic fibre construction;
  #57/#58 merged sole PG authority and referee receipt architecture;
  #65/#69 merged governed 2–10-parent revival; #20 merged DA51/SLR/JMD
  handoff; #18/#17 INV/federated world and #70 epistemic deliberation are
  integration crosswalks, not permission to duplicate runtime.
- DASHI Lean [#112](https://github.com/chboishabba/dashi_lean4/pull/112),
  head `f4f2075a87fbdefd12d4a3d094821ece69f8a977` at audit:
  SourceQueryLicence, GwbTypedSemanticFibre, GwbHyperfabricTransport,
  GwbCoreferenceFormalRepair, GwbP5P10ExactWeld, GwbArticleSystemSoundness,
  native M12 correspondence and canonical article population. Original JMD
  PKB.supportsB_iff, KB.checkSubChain_sound and importer correspondence are
  existing theorem owners. Bounded reporting/office specimen is a regression,
  not production general semantic alignment. Reconcile its independent branch
  before requesting operator execution.
- Golden Agda actual bodies previously inspected on default-branch snapshot
  `4c9310a4d94783595499a27cc9c8fe4c929920cd`:
  `DASHI/Cognition/PNF/ITIRRelationalInterlinguaConsumerFibreExact.agda`;
  `DASHI/Core/HeterogeneousPNFObservableBridgeExact.agda`,
  `ContextIndexedPNFComparisonTransportExact.agda`,
  `ConsumerDescentMinimalObserverExact.agda`, `ConsumerFibreRepairExact.agda`,
  `ConsumerFamilyRefinementKernelExact.agda`, `ITIRInvestigationAcquisitionParetoExact.agda`;
  `DASHI/Wikimedia/IbrahimMultilingualHyperfabricWeldExact.agda`,
  `IbrahimITIRConsumerRepairExact.agda`, `ConceptEntityAlignmentExact.agda`;
  `DASHI/Reasoning/TernaryComparisonSynthesisExact.agda`.
  Check current PRs and exact theorem bodies for all additional obligations,
  including VerifiedClassSelection, IdentityEvidenceProduction, numeric
  owner/Level-3, consumer-indexed reopening and source identity.
- Concrete laws: composeHeterogeneous requires matching intermediate
  observation and common consumer embedding; sharedQidCannotPayFirstLinkConsumer;
  firstLinkRepairRetainsOriginalView; firstLinkRepairPaysSelectedConsumer;
  firstLinkRepairDoesNotPayAllConsumers; refinementRepairSeparatesWitness
  is necessary, not alone sufficient; candidateRepairPreservesSelectedConsumer;
  signedClassificationRoundtrip and signedSynthesisRetainsComparison.
  Do not strengthen these beyond their proved domains.

## 2. Internal implementation phases and deliverables

One coordinated max-cut sprint; these are internal partitions, not separate
operator runs. Status vocabulary: inherited source / integrated source /
implementation open / formal checked (specific receipt) / native accepted
(specific receipt) / external evidence hold. Current phase P0.1 integration
baseline; downstream implementation proceeds against existing owners.

| ID | Phase / deliverable | Existing production owner to reuse | Exit obligation | Status |
|---|---|---|---|---|
| D01 | P0.1 exact golden parity and donor reconciliation | #78, #76, #72, #54/#55, #65, SensibLaw donor ledger, Agda/Lean | Per-operation exact theorem, runtime type, producer and inverse, source/receipt disposition; reconcile remaining #59 and cross-repo heads | integrated baseline; audit open |
| D02 | P0.2 universal native typed substrate | M12, numeric_pnf_*, statement_pnf_spine, candidate_pnf_store, checked_parser_world, chronology_contestation_store | Complete observed-key accounting, relation-family inverse, distinct manifestation/entity/event/proposition identity, role/qualifier/scope/time/modality/negation, signed and unknown residuals | inherited source; weld open |
| D03 | P0.3 multilingual and heterogeneous informational equivalence | relational_interlingua, persisted_relational_observation, relational_interlingua_store, consumer_adequacy, consumer_theorem_bridge | Language/revision/context-indexed common meet, explicit directional difference, known missing vs unknown, licensed bidirectional common-consumer evaluation, all nine signed states and 27 synthesis states with originals retained | inherited source; generic transport weld open |
| D04 | P0.4 proof-directed acquisition and repair | investigation_acquisition/store, investigation_graph_binding/store, governance_control_case/store, epistemic_deliberation/store, review_workstation_store | Collision or exact residual → five-axis Pareto route → source follow → typed candidate → review → selective replay; no candidate-only promotion | inherited + #76 integrated; orchestration open |
| D05 | P0.5 generic JMD article/KB proof and ontology routing | Lean #112 and original JMD, formal-proof-bridge, wikidata_ontology_review, native source-local consumer | Actual proof constructor populated from original data; statements/qualifiers/ranks/references; P279/P361/P2738/P1709/P2888 distinct; local candidate concepts, verified lemma routing under transport; no hard-coded article/wordlists | bounded regression source; generic elaboration open |
| D06 | P0.6 persistence and reversible proof/identity state | numeric_pnf_identity_audit/SQL, reviewed evidence stores, sl-formal-proof-reopen, reviewed_pnf, Mabo proof graph | Immutable L1, typed L2 candidate, governed L3 admitted derivation with dependency closure; same-source inverse and exact withdrawal/replay; no unrelated reprocessing | inherited source; full endpoint open |
| D07 | P0.7 product integration | ITIR user story contracts, existing MCP/DA51, SLR workbench, native semantic reader, itir-dioxus | Existing facades for review packets, ontology migration, compare, acquire, route proof, explain/inspect and revision history; exact data not presentation-owned authority | cross-repo weld open |
| D08 | P0.8 consolidated campaign | Existing native GWB scripts, REL/INV/GOV examples, formal suite, legal and producer-conformance tests | Integrated checkpoints, positive and negative real-source journeys, raw committed receipts and no repeated whole-corpus runs | acceptance not run |

### Required constructive approach

For each consumer Q: reverse-solve its endpoint constructor to exact premises;
find the production objects that inhabit them; use original source projection,
typed transport and independent evidence; establish nonempty live fibres
and all-live constancy only where warranted. A query that factors through a
coarser observer must not trigger global identity resolution. A collision
with different answers requires a separating coordinate in every sufficient
repair, but one separation is not global adequacy. Outside-model live
alternatives remain explicit. Preserve separate positive and counter-support
ledgers (including both), and keep zero/undetermined separate from negative.
A finite exact enumeration is sufficient only with a completeness witness.
Prioritize real endpoint implementations, not generic conditional theorem
wrappers, repeated fixture construction or performance-only campaigns.

### Current GWB evidence (inherited; not newly executed)

Use original #78 receipts and its detailed
[gwb_native_antecedent_s29_review_20261123.md](gwb_native_antecedent_s29_review_20261123.md).
The reported v8 evidence has 378/378 entity inverse, 30/36 eventuality numeric
materialization, 4/4 proposition numeric materialization, 548 unresolved
numeric keys, zero governed identity/Level-3. Separate earlier 36/36
eventuality factor inverse from 30/36 numeric carrier; do not treat the six
as a stand-alone P0 or equate counts with coverage. Book and pinned Wikipedia
sources are distinct. The original article accounting target is 643 sentences,
17,547 tokens and 1,468 predicate instances (reconfirm exact raw receipts);
51-QID graph and broader 10-document/12,742-paragraph/41,134-sentence candidate
corpus are acceptance scope, not proof of semantic closure. No new claim of
source evidence, runtime acceptance or kernel checks in this integration.

## 3. User-story deliverables and integrated acceptance

| Journey | Stories | Acceptance (positive and obstruction paths) |
|---|---|---|
| J01 | ITIR-US-17, WIKI-US-04 / SL-US-41 | English/Simple English/another edition and available Abstract Wikipedia representation: revision-locked multilingual comparison, common information and directed differences; missing acquisition explicitly recorded. Shared QID never pays parity. |
| J02 | WIKI-US-01 / SL-US-38; SL-US-19 | Article/statement repair, exact source anchor, candidate modification and independent validation; qualification/omission/conflict not collapsed; strict malformed reject. |
| J03 | WIKI-US-02 / SL-US-39; WIKI-US-03 / SL-US-40 | Existing six WikiProject tool profiles, Nat action Families A full_auto candidate / B split_auto / C repair+migrate / D typed hold / E manual; P2738 and class-order diagnostics and migration candidate. Source cohorts not action families. Zero unreviewed live writes. |
| J04 | ITIR-US-17; SLR-US-02 / ITIR-US-19 | Generic original GWB article-content proofs for mention, actor, pronoun, event participants, attribution, denial, time/order, linked KB and discrepancy. All 51 QIDs accounted for under declared bounds; no hard-coded names or predicates in production. |
| J05 | SL-US-15/18/19; WIKI-US-04 | Wikipedia vs transcript/legal record/primary filings; actual procedural posture, source genealogy, allegation/denial/finding, legal authority firewall. Source-exact Mabo/Smith and existing corpus examples. |
| J06 | SL-US-16/17 | Distinct person, office, institution, jurisdiction, responsibility, role capacity and temporal qualifications. |
| J07 | ITIR #65/#69 | Two-to-ten parent synthesis with fresh child, explicit complete lineage, retained pairwise signs/zero mode and no invented agreement. Use existing revival contracts; not a new chat service. |
| J08 | ITIR-INV-1, GOV-1 | Exact missing premise or separating witness drives lawful acquisition; maximize information/dependency/coverage/novelty, minimize cost; provenance and typed review retained. |
| J09 | SLR-US-08 / ITIR-US-25 | Typed ontology/lemma discovery, candidate-only route/refine, exact authenticated independent proof receipt. QID or lexical match is not theorem applicability. |
| J10 | SL-US-16, WIKI-US-01/04 | Revision, withdrawal and deterministic replay, unaffected source and consumer state unchanged; stale or forged proof rejected for exact reason. |
| J11 | ITIR progressive proof workbench | One canonical object drives explain/inspect; expand exact source, proof and alternative readings; no semantic change from view depth. |

Global adversarial controls: QID collision across languages; same words but
different scope; opposite polarity vs missingness; both support and counter
retained; same members different type; actor vs office/institution; source
revision/hash change; stale licence and independent-witness substitution;
omitted live rival; incomplete declared domain; candidate review cannot
self-promote; source→candidate→source and candidate→all original observations
round-trip; withdrawal commutes with unaffected query. Check negative reasons,
not arbitrary exit failures.

## 4. Verification and handoff protocol

Before operator handoff: reconcile actual PR/default branch source bodies;
inspect all new cross-owner type and ABI bindings; check tree/merge and
source-coordinate correspondence; use available local exact mathematical
checks and published original receipts. Implement all D01–D07 known seams
before requesting the consolidated D08 campaign. If genuine empirical evidence
is missing, record exact acquisition obligation rather than fabricate it.
Keep local-only execution, no GitHub CI requests, no sandbox artifact handoffs.
Bound costly steps to 90-second checkpointed units where feasible; retain
source/contract/environment/dependency fingerprints, reusable receipts and
resume state. The operator runs one integrated acceptance campaign; assistant
inspects raw committed outputs and resolves discrepancies, not the operator.
Subsequent runs execute only affected dependencies and failed stages.

Acceptance layers: (1) golden theorem/constructors and proof premises,
(2) source implementation and static ABI parity, (3) independent kernel and
native unit/build checks, (4) original PG/SLR source and reversible persistence,
(5) ITIR user-story integration and original corpus coverage. Record exact
commit, command, result, residual keys and artifact refs. No PASS from a
roadmap, conditional theorem, fixture or candidate-only receipt. The existing
GWB and formal suite scripts are inputs to the consolidated campaign, not
instructions to run them now.

## 5. Changelog

- 2026-11-26: audited SLR recent PR inventory and SensibLaw prototype donors;
  integrated #78, #76 and #72 with current-main roadmap retained; committed
  this controlling sprint plan. No implementation campaign run or proof
  promotion. D01 remaining reconciliation and D02–D07 actual welds next.

- 2026-11-26 integration readback: [SLR #79](https://github.com/chboishabba/slr/pull/79) opened as draft against main. Integration merge commit `dc72973d347e7aa77a2ac54c3963a9e85d6a4fd7` is ahead of all four pinned parents, behind each by zero; compare against #78 shows exactly 13 changed paths. All ten non-shared #76/#72 donor paths were reopened at the integrated commit and their blob SHAs matched the corresponding donor heads (10/10). Shared lib exports and roadmap were reconciled rather than copied wholesale. No compilation, kernel acceptance or database execution claimed. GitHub initially reported mergeable=false on PR creation; ancestry was independently verified, review/merge readiness is not claimed. #59 remains a divergent donor, not merged; an attempted multi-path comparison returned 404, so exact file-level disposition remains open.

## 6. Implementation cut MC-REL-1 — native source → relational family → INV

2026-11-26, source implementation only (not native accepted).
- D02: added checked persisted-batch predicate projection using the existing
  native parser evidence audit, source UTF-8/digest owner and exact M12
  factors. All predicate occurrences in a selected batch are enumerated;
  direct dependency fibres preserve nested predicates, parent coordinates,
  literal negation and source-local role occurrences without lexical
  reporting dictionaries. Syntax roles remain syntax, not agent/truth
  admission. Legacy coarse adapter remains for historical receipt replay.
- D03: composed existing comparator for both directions of every distinct
  2–10-source family; reversible nine-state SOURCE polarity classification,
  explicit missing required context, distinct original selectors and
  observations. Source polarity is not a reviewed ternary warrant. Actual
  all-live-reading sufficiency and common-observable proof construction remain
  open; no 27-state synthesis or semantic equivalence claim from this cut.
- D04/D06: reuse durable comparison and INV queues for grouped exact
  residuals; retain all residual coordinates under each existing obligation,
  preserve already existing routes on replay, derive family-local potential
  reopening cones with the existing graph function. No routes/authority
  fabricated, no actual retraction performed. Modeled repair assessment now
  rejects erasing signed evidence or silently dropping unknowns.
- D07: native CLI `relational-family CONSUMER_REF ACCESS_CONSTRAINT_REF
  SELECTOR_REF SELECTOR_REF ...` uses existing typed PG-owned selectors
  and consumer contracts (currently acceptance/control ownership); no
  additional JSON control plane. Native comparison rows/INV queues are
  durable per-pair/per-obligation, not a family-wide atomic commit.
- Verification prepared: direct nested dependency/negation, invalid head,
  all-nine source polarity round trips, asymmetric licence, group residual
  multiplicity, missing context and unrelated reopening, signed-evidence
  erasure regressions. Not run: no Rust/Lean/PG toolchain in this session.
- Remaining max-cut: generic source family/provider acquisition and complete
  numeric factor-to-REL/semantic licensed transport, genuine live-fibre
  coverage and independent proof construction, reviewed multilingual
  common-meet/synthesis, complete candidate repair + governed review + actual
  retraction, JMD ontology/KB qualifier alignment and ITIR facade. Existing
  native predicate syntax is not sufficient to pay any of these by itself.
  No operator campaign requested.

### MC-REL-1 continuation — strict comparison and atomic queue publication

- D03: new v2 strict comparison contract uses the existing v1 comparator,
  then accounts for each required missing context axis including both-none.
  Unknown context emits a real consumer residual routed to INV, not just
  an annotation. Full left/right/consumer payload is content-addressed.
  Historic v1 comparison IDs and replay remain unchanged.
- D04/D06: existing REL/INV owners now share one snapshot for a family:
  exact source content digest cached by revision in that transaction,
  atomic pair insert/readback plus obligation queues, lock and validate
  immutable obligation before updating routes, preserve existing Pareto
  frontier on family replay. A failed replay rolls back the family. The
  ordinary queue update now verifies the stored reopening receipt and
  commits it with the acquisition update. This changes no semantic or
  access authority. Actual PostgreSQL acceptance remains unrun.
- Added strict missing-context, changed-consumer identity, and acquisition
  routing assertions; existing pure tests retained. No production proof
  or kernel claims. The earlier per-pair-only persistence limitation is
  superseded for the new family path by this atomic transaction.

### MC-REL-1 / D05 continuation — JMD original statement inverse

- Companion [Lean #112 commit 1f10108017a5e](https://github.com/chboishabba/dashi_lean4/commit/1f10108017a5e906df35148e109b42659d4468f1) extends the existing original JMD world-graph proof runner, not a new prover. It binds the raw graph and cached EntityData to SHA-256/readback, retains all node and edge ordinals, per-node revision availability, and original matching statement JSON with rank/snak/qualifiers/references (deprecated matches retained as excluded evidence). Positive PKB support requires a nondeprecated direct original import inverse; unsupported edges remain explicit. Reuses JMD's importer and existing checkedWorldPropertySound; no qualifier alignment, article equivalence, historical truth or synchronous-snapshot proof. Kernel and native acceptance not run.
- D04/D06 hardening: stable comparison/obligation lock order, order-independent bounded family graph reference; existing acquisition priority/route state must validate before extension and old routes cannot silently disappear. Source selection still precedes the publication transaction: source bytes are rechecked, but full parser/selector snapshot replay in that transaction remains an open weld.
- Static source-table audit: 9/9 actual Rust ordered polarity match arms have exact inverse, all distinct; independent combinatorial oracle covers all family sizes 2–10 and 54 source-local reopening cardinalities (2(n−1) incident directional pairs). This is source inspection/oracle verification, not compiled Rust, PG execution or proof acceptance.
- D01–D07 remain active; D08 not requested. No product completion claimed.

### MC-REL-2 — one source/selector/publication snapshot

Reused the existing candidate batch, statement, native parser evidence, generic source, S29 review and Wikidata diagnostic readers through GenericClient. The checked family path now opens its transaction before selecting the consumer and selectors, reconstructs both native parser and native Wikidata observations in that same repeatable-read transaction, then compares and persists REL/INV. No alternate loader or DB schema. Public standalone parser and Wikidata readers also gain read-only snapshot consistency. This addresses the previous split-snapshot source selection seam at implementation level; no native PG acceptance yet.

### MC-REL-2 — finite common observer and S29 integration

- D03: native selected-observation incidence transpose retains exact predicate, role/occurrence/filler, role-type, candidate-type and context coordinates with every present/not-recorded selector. Its common part is the intersection on recorded coordinates, not proof of a common semantic interpretation. Full original observations and separate signed source ledger remain available. Every directed pair carries the three independently selectable synthesis indices (existing Agda TernaryComparisonSynthesisExact); no selection is made without an interaction witness. Do not conflate these with positional X/Y/Z roles or logical negation (Woogaroo donor #59).
- D04/D07: every grouped INV residual now materializes an existing S29 ResearchAcquisition review item in the same transaction, with original source revisions, comparison ref, exact residuals and consumer. Current review status is reloaded, never reset by family replay. No automatic Accept action, external edit, or semantic admission. Native CLI output includes workbench review and meet projections. S29 receipt persistence now rejects command-ref collisions before status mutation and does not reapply an old command to overwrite a later state.
- New pure source tests prepared for common/difference incidence and unselected synthesis. Kernel/DB acceptance remains unrun. D03 semantic transport and D06 governed L3 are not paid by this source-only observer.

### MC-REL-2 — effective acquisition lifecycle

- Existing INV terminal update table is now authoritative for effective availability without mutating immutable obligation or historical route receipts. A second incompatible terminal receipt fails closed; the same receipt is replayed, and Present must retain its ingested revision/reopening receipt, KnownAbsent must have no reopening. A terminal queue exposes no executable frontier; new routes cannot be added after terminal update. Legacy multiple terminal updates are explicit conflicts, not last-writer-wins. The family projection exposes this lifecycle alongside the existing historical queue. No source authenticity or absence of a world fact inferred from an acquisition status. Native SQL execution pending.
- Companion Lean #112 commit ca9f61406d86 independently reconstructs selected native observed coordinates, checks incidence and uses the existing finite consumer theorem. This is source-only proof construction, not all-live semantic coverage. No operator campaign requested.

### MC-REL-3 — durable family lineage and governed candidate revision

- New native lineage index stores only exact selector/observation/revision and existing pair identities, content-addressed with full consumer-bearing pair refs and access constraint. Existing REL/INV/S29 rows remain authoritative. `inspect` performs read-only exact source/selector/comparison/queue/review replay in one snapshot; it rejects a changed manifest. All native family receipts now expose a stable family ref and optional successor.
- `prepare-revision` constructs a candidate child by replacing one selector, preserving every other source observation and exact unaffected pair identity. It records retained, retired-in-parent and new pair refs, and publishes a real S29 SourceCorrespondence candidate review. `review-revision` uses existing S29 command owner; `publish-revision` requires actual accepted nonpromoting S29 receipt, current parent/child replay, and serialized successor insertion. Old family is actually marked superseded through the lineage index; immutable source, comparison, review and numeric L3 rows remain untouched. No same-entity, truth, or external edit authority implied. Unrelated rows are not mutated, although bounded comparison readback/recomputation still occurs. This is family-selection supersession, not whole-document governed L3 retraction.
- Added cardinality/delta regression, native inspection and candidate review CLI; no ITIR web/MCP adapter or end-to-end native execution yet. All remaining D01–D07 must still be judged against actual golden semantics and original source evidence.

- Family inspection distinguishes prepared inbound revisions from published parent transitions; a candidate child is not evidence of successor publication.

### Native original relation consumer

The existing numeric relation reopen owner now provides a generic recorded-prefix query by exact relation type (or all), retaining the original relation, ordered role occurrence, atom-to-token inverse, factor alternatives, constraints and typed residuals. It checks that all recorded relation owners are accounted for in the factor index, and returns the whole type inventory even for narrower queries. No extra parser, lexical whitelist, or inferred identity. Recorded-query completeness is not semantic live-fibre completeness. CLI: `numeric-pnf-native relation-query RUN DOC SNAPSHOT PACKET_SHA256 RELATION_TYPE_OR_all`. Source-level implementation only.
