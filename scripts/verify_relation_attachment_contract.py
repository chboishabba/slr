#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
producer = (root / "crates/sl-expanded-cert/src/relation_attachment.rs").read_text()
runner = (root / "python/gwb_relation_attachment_certify.py").read_text()
main = (root / "crates/sl-expanded-cert/src/main.rs").read_text()

checks = {
    "seven structural relation kinds": all(
        name in producer
        for name in (
            "Preposition",
            "PrepositionalObject",
            "PrepositionalComplement",
            "PassiveAgentMarker",
            "Dative",
            "CaseMarker",
            "Particle",
        )
    ),
    "candidate only": "candidate_only: true" in producer,
    "context resolution required": "context_resolution_required: true" in producer,
    "direct and reference producers exist": "pub fn direct_candidates" in producer and "pub fn reference_candidates" in producer,
    "producer does not choose legal roles": "ExpandedSemanticRole" not in producer and "Jurisdiction" not in producer and "Evidence" not in producer and "Provenance" not in producer,
    "producer has no publication API": "GenerationPublisher" not in producer and ".publish(" not in producer,
    "binary checks relation parity": "SL_RELATION_ATTACHMENT_PARITY_FAIL" in main and "relation_parity_failed" in main,
    "binary reports zero semantic authority": "semantic_authority=0" in main,
    "runner requires v04 source receipt": "sensiblaw.gwb-expanded-semantic-certification-receipt.v0_4" in runner,
    "runner preserves canonical observation": "same_canonical_parser_observation_as_source" in runner,
    "runner requires source relation queue equality": "relation_counts_equal_source_relative_fibre" in runner,
    "runner keeps producer candidate-only": '"candidate_only": True' in runner,
    "runner blocks admission authority": '"grants_admission_authority": False' in runner,
}

for name, ok in checks.items():
    print(("PASS" if ok else "FAIL"), name)
raise SystemExit(0 if all(checks.values()) else 1)
