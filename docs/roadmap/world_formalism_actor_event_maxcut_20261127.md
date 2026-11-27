# World formalism: actor/event max-cut (P0, 2026-11-27)

Supersedes the endpoint ordering (not the infrastructure) in the informational-equivalence max-cut roadmap. Stack on SLR #79; Lean #112 and Agda golden specifications, Rust/SQL production, ITIR native consumer; SensibLaw historical donor only. No new proof transport, no Python semantics, no GitHub CI. Local consolidated acceptance deferred.

## Endpoint
Use actual independently revision-pinned GWB and GHWB articles, then expand through other referents, event and institution articles, Wikidata, primary records and law. A source is a partial chart, not world truth. Preserve original offsets, revision, language, participant roles, modality, polarity, time, scope, attribution, provenance dependencies and unresolved alternatives. Never infer actor from article subject or personal responsibility from office membership.

## Acceptance cuts
| P0 | Demonstration | Initial status |
|---|---|---|
| W01 | GWB actions vs other actors' actions and mentions | open |
| W02 | GHWB independent action extraction | open |
| W03 | Shared event correspondence without string-only identity | open |
| W04 | Reciprocal article comparison, both directions | open |
| W05 | Reverse traversal through people, event, institution, source | open |
| W06 | Signed overlap, explicit denial, absence, conflict, both-supported; 2–10 parents with dependency provenance | open |
| W07 | Time-indexed institutional/state transitions; distinguish chronology from causal evidence | open |
| W08 | Independent JMD Wikidata original statements and qualifiers | partial Lean #112 |
| W09 | Primary records, legislation and judgment stage fidelity | open |
| W10 | Law as typed conditional algebra; reverse-solve premises and exceptions, no automated verdict | open |
| W11 | Source inverse, selective withdrawal/replay through existing authenticated proof-to-PG owners | open |

## Existing owners and welds
- M12/S28 source correspondence, numeric PNF, relational interlingua, persisted relational observations and relation-family owners: use actual source-bound carriers, preserve ambiguity.
- World compiler checked_parser_world and existing observation stream: group exact predicate incidence, distinguish grammatical passive patient from actor, preserve all local coordinates and unlicensed scope. Candidate-only until governed correspondence.
- Indexed hyperfabric, heterogeneous PNF bridge, signed ternary synthesis, mereology and consumer-relative closure: reuse golden constructors, retain zero and signed residual. Compare only common proposition/role/time/scope fibre under explicit correspondence. A source may assert and deny; absence is not denial.
- JMD importer and original property/qualifier proof constructors: independent KB corroboration, not automatic identity or truth.
- Existing formal-proof-bridge, referee authentication, PG admission and reopen: consume existing interfaces; proof transport is not the endpoint.
- ITIR docs/user_stories.md SL-US-04/05/08/09–31, especially 15–21, WIKI-US-01–04 and ITIR-US-17: actor/capacity, competing narrative, legal fidelity, chronology, argument navigation, governance. Review source-local claims, do not adjudicate.
- Reverse legal rule: match established conduct, jurisdiction, capacity, effective rule version and exceptions; report satisfied/contradicted/unresolved premises with evidence, retain procedural posture and alternate interpretations.

## Acceptance discipline
Real source receipts, no synthetic fixture promoted to real-world answer. Require source-to-result-to-source round trips, actor swap/passive/negation/quotation counterexamples, source-order invariance, missing source vs denial, same-source dependence, withdrawal of one support without deleting independent support. Report executable implementation separately from local Rust/SQL and golden kernel acceptance.

## Implementation ledger
- 1fd282a7be10: CheckedRelationalBatch::participant_incidence and predicates_for_participant expose exact role/factor inverse on existing checked native batches. W01/W02/W05 partial, not entity-resolved. No actor promotion, no corpus execution.
- Existing CheckedDependencyFibre already validates source bytes, parser parity, direct predicate children, passive syntax and nested local negation. Reuse rather than duplicate.
- Existing relational_family.rs already owns 2–10 source directed comparisons, nine-state polarity, observed-coordinate incidence, declared transport meet, synthesis, acquisition and selective reopening. W03/W04/W06/W11 integration targets, not new engines.
- Remaining: real GWB/GHWB source acquisition and batch selection, licensed cross-source referent/event alignment, temporal and legal premise composition, ITIR native exposure, local consolidated acceptance. No native tests executed in this round.

## Binding priority correction: Wikipedia first
W01–W08 constitute the immediate P0 gate; W11 original-source inverse and proof reopening are cross-cutting invariants, not a separate infrastructure programme. W09–W10 legal execution is gated on substantive real Wikipedia proof results. Keep legal context/capacity/time coordinates from the beginning; do not strip them from the wiki representation. Independent Wikidata corroboration runs alongside wiki proofs, not instead of them. No claim of world truth or automated legal adjudication.

## Phase / deliverable / exact remaining lemmas and ownership
| Phase | Deliverable | Existing producer → consumer (reuse) | Exact remaining lemma/weld |
|---|---|---|---|
| A W01–02 | Independently pinned Q207/Q23685 articles, complete source-local action/mention inventory | SLR #79 persisted_relational_observation::CheckedDependencyFibre, relational_observations_from_checked_batch_on_client → Lean #112 WikipediaArticleProof::checkedIndexedArticle?, GwbCanonicalSourceProof, GwbArticleIdentity | Original raw revision and parser coordinates commute with predicate/role projection; prove actor attribution over every live reading for each query, not article subject or nominal subject alone; passive, quotation, modality and coreference residuals |
| B W03–04 | Reciprocal event and action comparisons | Lean #112 GwbCoreferenceFormalRepair, GwbTypedSemanticFibre, GwbRelationTransport, GwbP4SourceLicence → SLR #79 relational_interlingua::compare_relational_observations_strict and relational_family::project_relational_family | Actual licensed referent and event correspondence preserves participant role, capacity, time, attribution, polarity and scope; construct both directional query licences independently (one-way licence is not symmetric) |
| C W05–06 | Bidirectional source/participant/event traversal and signed multi-parent comparison | SLR #80 participant_incidence; #79 observed_family_meet, declared_transport_meet, source_polarity_pair, durable family consumers → Lean #112 RelationalObservedMeet, GwbHyperfabricTransport, FiniteConsumerCertificate | Incidence transpose round trip over actual checked sources; lift licensed common proposition fibre to signed support sets preserving both signs and unknown; no independent-source claim from repeated citation; all-live coverage remains separate |
| D W07 | Indexed temporal/institutional transition composition | Lean #112 GwbProofComposition, GwbWorldGraphProof, GwbHyperfabricTransport; Agda indexed hyperfabric donor #1291 (candidate donor, exact theorem audit pending) → SLR #79 native relation carriers | Shared boundary state and compatible capacity/time indices; distinguish chronology from evidenced dependency/causation; retain residual when transition is not licensed |
| E W08 | Original article/KB independent corroboration | Lean #112 GwbWorldGraphProof::originalEntityPropertyProof?, originalQualifiedYearWitnesses, GwbP5P10ExactWeld::indexedArticleKBAlignmentSound, GwbProofRunner indexed-article-kb-check → existing SLR formal-proof-bridge and referee consumer | Bind original sitelink/revision and exact article reading to same typed property/qualifier; construct actual SourceQueryLicence, not merely conditional theorem or separate checks |
| F W11 cross-cut | Authenticated replay of actual query proof | SLR #79 formal-proof-bridge::reopen_authenticated_formal_evidence_tx, relational_family::inspect_persisted_relational_family, existing selective_reopening and H6 current authority | Exact proof/consumer/source binding; retraction invalidates only dependent answers; independent support remains; do not substitute rollback probe for durable proof lifecycle |
| G W09–10 gated | Legal context and algebra | ITIR user stories SL-US-15–21, WIKI-US-01–04; SLR M12 statement PNF and checked primary manifestation overlap → existing legal predicate consumers (exact audit pending) | Rule-version/jurisdiction/capacity/exception and procedural-stage matching, reverse premise acquisition; no verdict promotion |

Relevant PRs: [SLR #79](https://github.com/chboishabba/slr/pull/79) native base; [SLR #80](https://github.com/chboishabba/slr/pull/80) endpoint; [Lean #112](https://github.com/chboishabba/dashi_lean4/pull/112) golden wiki/JMD proof producers; [Agda #1291](https://github.com/chboishabba/dashi_agda/pull/1291) indexed donor to reconcile, not asserted accepted. ITIR docs/user_stories.md is the downstream acceptance authority; SensibLaw historical donor only. Lean #112 files GwbNativeFibreReceipt, GwbNativeAntecedentMaxCut, GwbRealSourceConsumer, GwbSourceLocalConsumer and GwbConsumerFibreProof are additional existing candidates to reconcile before writing new coverage machinery. No claim that named candidate donors already solve the universal endpoint.

Gate receipt: actual original pinned source inventory, reciprocal action query outputs, exact matched/unmatched role and event coordinates, signed differences, source inverse, independent KB check and explicit remaining obligations. Distinguish candidate syntax from checked semantic answer. Local compiler/kernel/DB acceptance deferred; no CI.
