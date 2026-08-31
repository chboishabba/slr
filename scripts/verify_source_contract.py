#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
core = (root/'crates/sl-core/src/lib.rs').read_text()
parity = (root/'crates/sl-parity/src/lib.rs').read_text()
stream = (root/'crates/sl-stream/src/main.rs').read_text()
worker = (root/'python/spacy_stream.py').read_text()
gwb = (root/'python/gwb_tranche.py').read_text()
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
    'gwb explicitly enables parity': 'proc.stdin.write("C\\tparity=1\\n")' in gwb,
    'paragraph framing': 'print(f"P\\t{paragraph_id}"' in worker and 'print(f"Q\\t{paragraph_id}"' in worker,
    'whitespace sentences filtered': 'if not sent.text.strip()' in worker,
    'parser unavailable marker': 'dep = "-"' in worker,
    'published total-time gate': 'T_total <= 2 * T_spaCy_parse' in arch,
    'direct-delta contract documented': 'Paragraph fusion accepts only `SentenceOutwardDelta`' in direct,
    'gwb raw payload strict': 'bio_count != 6 or book_count != 4' in gwb,
    'gwb derived artifacts excluded': 'derived_inventory_artifacts_reingested": False' in gwb,
    'gwb parser cannot publish': '"parser_did_not_publish": published == 0' in gwb,
    'gwb parity fail closed': 'parity_failed == 0 and parity_checked == sentences' in gwb,
    'gwb performance fail closed': 'pipeline_wall_ns <= 2 * parse_ns' in gwb,
    'gwb phase split documented': 'Source projection' in gwb_doc and 'Direct semantic run' in gwb_doc,
    'cargo lock committed': (root/'Cargo.lock').exists(),
}
for k,v in checks.items(): print(('PASS' if v else 'FAIL'), k)
raise SystemExit(0 if all(checks.values()) else 1)
