#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
core = (root/'crates/sl-core/src/lib.rs').read_text()
parity = (root/'crates/sl-parity/src/lib.rs').read_text()
stream = (root/'crates/sl-stream/src/main.rs').read_text()
worker = (root/'python/spacy_stream.py').read_text()
gwb = (root/'python/gwb_tranche.py').read_text()
gwb_full = (root/'python/gwb_full_run.py').read_text()
gwb_certify = (root/'python/gwb_certify.py').read_text()
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
    'full gwb explicitly enables parity': 'proc.stdin.write("C\\tparity=1\\n")' in gwb_full,
    'paragraph framing': 'print(f"P\\t{paragraph_id}"' in worker and 'print(f"Q\\t{paragraph_id}"' in worker,
    'whitespace sentences filtered': 'if not sent.text.strip()' in worker,
    'parser unavailable marker': 'dep = "-"' in worker,
    'published total-time gate': 'T_total <= 2 * T_spaCy_parse' in arch,
    'direct-delta contract documented': 'Paragraph fusion accepts only `SentenceOutwardDelta`' in direct,
    'gwb raw payload strict': 'bio_count != 6 or book_count != 4' in gwb,
    'gwb derived artifacts excluded': 'derived_inventory_artifacts_reingested": False' in gwb,
    'full gwb preloads hashes before timing': 'preload_verified_documents' in gwb_full and 'all_projected_text_hashes_verified_before_timing' in gwb_full,
    'full gwb avoids stdout pipe deadlock': 'stdout=subprocess.DEVNULL' in gwb_full,
    'full gwb uses file-backed stderr': 'NamedTemporaryFile' in gwb_full and 'stderr=err_file' in gwb_full,
    'full gwb parser cannot publish': '"parser_did_not_publish": publication_ok' in gwb_full,
    'full gwb parity fail closed': 'parity_failed == 0 and parity_checked == sentences' in gwb_full,
    'full gwb performance fail closed': 'pipeline_wall_ns <= 2 * max(parse_ns, 1)' in gwb_full,
    'full gwb distinct receipt schema': 'sensiblaw.gwb-full-certification-receipt.v0_1' in gwb_full,
    'strict certify exposes no subset option': 'add_argument("--limit"' not in gwb_certify and 'limit=None' in gwb_certify,
    'strict certify checks complete document count': 'complete_document_count' in gwb_certify,
    'gwb phase split documented': 'Source projection' in gwb_doc and 'Full certification run' in gwb_doc,
    'cargo lock committed': (root/'Cargo.lock').exists(),
}
for k,v in checks.items(): print(('PASS' if v else 'FAIL'), k)
raise SystemExit(0 if all(checks.values()) else 1)
