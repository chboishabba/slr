#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
core = (root/'crates/sl-core/src/lib.rs').read_text()
parity = (root/'crates/sl-parity/src/lib.rs').read_text()
expansion = (root/'crates/sl-semantic-expansion/src/lib.rs').read_text()
admission = (root/'crates/sl-semantic-admission/src/lib.rs').read_text()
expanded_cert = (root/'crates/sl-expanded-cert/src/main.rs').read_text()
gold = (root/'fixtures/legal_semantic_conformance_v0_1.tsv').read_text()
stream = (root/'crates/sl-stream/src/main.rs').read_text()
worker = (root/'python/spacy_stream.py').read_text()
gwb_tranche = (root/'python/gwb_tranche.py').read_text()
gwb_prepare = (root/'python/gwb_prepare.py').read_text()
gwb_full = (root/'python/gwb_full_run.py').read_text()
gwb_certify = (root/'python/gwb_certify.py').read_text()
gwb_expanded = (root/'python/gwb_expanded_certify.py').read_text()
arch = (root/'docs/ARCHITECTURE.md').read_text()
direct = (root/'docs/DIRECT_DELTA.md').read_text()
gwb_doc = (root/'docs/GWB_TRANCHE.md').read_text()
checks = {
    'revision-scoped TextSpan': 'pub revision_id: RevisionId' in core,
    'boundary crossing queues repair': 'BoundaryCrossing => SentenceDisposition::QueueBoundaryRepair' in core,
    'missing head typed failure': 'MissingDependentHead' in core and 'ResidualKind::MissingDependentHead' in core,
    'candidate-only semantics': 'candidate_only: true' in core,
    'packed sentence carrier': 'pub struct PackedSentence' in core and 'pub struct FibreAddress' in core,
    'zero physical constitution': 'direct_constitution_holds' in core and 'sentence_local_db_crossings == 0' in core,
    'outward-only paragraph API': 'pub fn absorb(&mut self, child: SentenceOutwardDelta)' in core,
    'transport commuting check': 'pub fn transport_commutes' in core,
    'fail-closed generation publication': 'PublicationError::NotCertified' in core and 'pub fn certify' in core,
    'stream does not auto publish': 'publisher.publish' not in stream,
    'parity compiler exists': 'pub fn check_direct_reference_parity' in parity and 'pub struct ConsumerObservation' in parity,
    'parity is opt-in not production default': 'let mut parity_enabled = false' in stream and '"C" =>' in stream,
    'semantic expansion is separate crate': 'pub fn compile_expanded_candidates' in expansion,
    'semantic expansion has no publication API': 'GenerationPublisher' not in expansion and '.publish(' not in expansion,
    'semantic expansion candidates stay candidate-only': 'candidate_only: true' in expansion,
    'semantic expansion retains ambiguous fibres': 'CandidateAlternativeFibre' in expansion and 'ClauseInterpretationAmbiguous' in expansion,
    'semantic expansion preserves unresolved scope': 'NegationScopeUnresolved' in expansion and 'ConditionalScopeUnresolved' in expansion and 'ReferenceAttachmentUnresolved' in expansion,
    'expanded direct compiler independent of reference projection': 'pub fn compile_expanded_direct' in expansion and 'rather than consuming `project_sentence`' in expansion,
    'expanded parity uses stable consumer observation': 'pub struct ExpandedConsumerObservation' in expansion and 'StableHeadRelation' in expansion,
    'expanded parity excludes transient token ids': 'transient_token_ids_are_not_semantic_parity_authority' in expansion,
    'expanded parity retains source span authority': 'source_span_change_is_visible_to_semantic_parity' in expansion,
    'semantic admission is separate and non-publishing': 'pub struct AdmissionReceipt' in admission and 'GenerationPublisher' not in admission and '.publish(' not in admission,
    'semantic admission has no parser authority variant': 'enum ResolutionAuthority' in admission and 'Parser' not in admission.split('pub enum ResolutionAuthority', 1)[1].split('}', 1)[0],
    'semantic admission requires policy and resolver refs': 'MissingPolicyReference' in admission and 'MissingResolverReference' in admission,
    'semantic admission resolves declared scope exactly': 'ScopeResolutionMismatch' in admission and 'scope_matches' in admission,
    'semantic admission rejection retains evidence': 'retained_candidates' in admission and 'retained_residuals' in admission and 'retained_alternative_fibres' in admission,
    'legal semantic gold corpus exists': 'actor_subject\tcovered' in gold and 'clause_ambiguity\tcovered' in gold,
    'legal semantic gold retains producer gaps': gold.count('\tproducer_gap\t') == 6 and 'action_predicate\tproducer_gap' in gold and 'provenance_relation\tproducer_gap' in gold,
    'gold tests compare exact consumer objects': 'covered_gold_fixtures_match_exact_consumer_objects' in admission and 'expected(row, sentence_id)' in admission,
    'residual frontier is typed by residual kind': 'pub struct ResidualFrontier' in admission and 'RESIDUAL_KINDS' in admission,
    'residual ranking is not semantic quality': 'Work-selection score only; never semantic confidence/truth/authority.' in admission,
    'expanded cert supports parity/direct modes': 'parity_enabled' in expanded_cert and 'direct_active_ns=' in expanded_cert and 'reference_active_ns=' in expanded_cert,
    'expanded cert has zero publication effects': 'publication_effects=0' in expanded_cert and 'GenerationPublisher' not in expanded_cert,
    'expanded cert emits residual frontier': 'SL_EXPANDED_RESIDUAL kind=' in expanded_cert and 'ResidualFrontier' in expanded_cert,
    'expanded gwb uses canonical observation digest': 'CanonicalObservationHashingSink' in gwb_expanded and 'SEMANTIC_FRAME_KINDS = {"D", "P", "S", "T", "E", "Q"}' in gwb_expanded,
    'expanded gwb excludes timing telemetry from semantic identity': 'excluded_runtime_telemetry_frame_kinds": ["M"]' in gwb_expanded and 'runtime_timing_telemetry_excluded_from_semantic_observation_identity' in gwb_expanded,
    'expanded gwb requires same canonical parser observation': 'same_canonical_parser_observation_stream_across_passes' in gwb_expanded and 'canonical_parser_observation_sha256' in gwb_expanded,
    'expanded gwb separates parity and performance passes': 'reference_certification_cost_excluded_from_production_speed_claim' in gwb_expanded and 'direct_only_performance_pass' in gwb_expanded,
    'expanded gwb performance uses direct-only pass': 'direct_only_architectural_2x_gate_pass' in gwb_expanded,
    'expanded gwb strict full corpus': 'complete 10-document GWB v0.1 corpus' in gwb_expanded and 'add_argument("--limit"' not in gwb_expanded,
    'expanded gwb v0.3 residual frontier accounted': 'sensiblaw.gwb-expanded-semantic-certification-receipt.v0_3' in gwb_expanded and 'residual_frontier_sums_to_residual_total' in gwb_expanded and 'same_residual_frontier_across_passes' in gwb_expanded,
    'full gwb explicitly enables parity': 'proc.stdin.write("C\\tparity=1\\n")' in gwb_full,
    'paragraph framing': 'print(f"P\\t{paragraph_id}"' in worker and 'print(f"Q\\t{paragraph_id}"' in worker,
    'whitespace sentences filtered': 'if not sent.text.strip()' in worker,
    'parser unavailable marker': 'dep = "-"' in worker,
    'published total-time gate': 'T_total <= 2 * T_spaCy_parse' in arch,
    'direct-delta contract documented': 'Paragraph fusion accepts only `SentenceOutwardDelta`' in direct,
    'canonical gwb payload exact': 'len(paths) != 10 or len(bios) != 6 or len(books) != 4' in gwb_prepare,
    'canonical gwb family membership explicit': 'PUBLIC_BIOS not in memberships[p]' in gwb_prepare and 'BOOKS not in memberships[p]' in gwb_prepare,
    'canonical gwb source order independent': 'source_family_order_independent": True' in gwb_prepare and 'source_kind_then_resolved_path' in gwb_prepare,
    'canonical gwb derived artifacts excluded': 'derived_inventory_artifacts_reingested": False' in gwb_prepare,
    'full gwb preloads hashes before timing': 'preload_verified_documents' in gwb_full and 'all_projected_text_hashes_verified_before_timing' in gwb_full,
    'full gwb projected text read is byte-faithful': 'path.open("r", encoding="utf-8", newline="")' in gwb_full,
    'legacy gwb projected text read is byte-faithful': 'path.open("r", encoding="utf-8", newline="")' in gwb_tranche,
    'full gwb avoids stdout pipe deadlock': 'stdout=subprocess.DEVNULL' in gwb_full,
    'full gwb uses file-backed stderr': 'NamedTemporaryFile' in gwb_full and 'stderr=err_file' in gwb_full,
    'full gwb parser cannot publish': '"parser_did_not_publish": publication_ok' in gwb_full,
    'full gwb parity fail closed': 'parity_failed == 0 and parity_checked == sentences' in gwb_full,
    'full gwb performance fail closed': 'pipeline_wall_ns <= 2 * max(parse_ns, 1)' in gwb_full,
    'full gwb distinct receipt schema': 'sensiblaw.gwb-full-certification-receipt.v0_1' in gwb_full,
    'strict certify exposes no subset option': 'add_argument("--limit"' not in gwb_certify and 'limit=None' in gwb_certify,
    'strict certify checks complete document count': 'complete_document_count' in gwb_certify,
    'gwb phase split documented': 'Canonical source projection' in gwb_doc and 'Full certification run' in gwb_doc,
    'cargo lock committed': (root/'Cargo.lock').exists(),
}
for k,v in checks.items(): print(('PASS' if v else 'FAIL'), k)
raise SystemExit(0 if all(checks.values()) else 1)
