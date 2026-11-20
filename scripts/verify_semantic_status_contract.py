#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "crates" / "sl-semantic-status" / "src" / "lib.rs"
text = SOURCE.read_text(encoding="utf-8")

required = [
    "pub enum ParticipantRole",
    "pub enum LegalParticipantRole",
    "pub enum ReferentKind",
    "pub enum IdentityStatus",
    "pub enum AntecedentStatus",
    "pub enum OccurrenceStatus",
    "pub enum PropositionStatus",
    "pub enum TruthStatus",
    "pub enum AttributionRole",
    "pub enum EvidenceKind",
    "pub enum ModalityKind",
    "pub enum ModalForce",
    "pub enum TemporalRelationKind",
    "pub enum JurisdictionKind",
    "pub enum AuthorityKind",
    "pub enum ApplicabilityStatus",
    "pub enum ViolationStatus",
    "pub enum LiabilityStatus",
    "pub enum JudicialDiscourseStatus",
    "pub enum NormativeRelation",
    "pub struct SemanticCommitmentState",
    "pub struct IdentityResolutionReceipt",
    "pub struct AntecedentResolutionReceipt",
    "pub struct OccurrenceResolutionReceipt",
    "pub struct PropositionResolutionReceipt",
    "pub struct ApplicabilityResolutionReceipt",
    "pub candidate_only: bool",
    "pub governed_admission_present: bool",
]
missing = [needle for needle in required if needle not in text]
if missing:
    raise SystemExit(f"semantic status contract missing: {missing}")

for forbidden in ["regex::", "Regex::", "publication_effect", "publish(", "auto_admit", "automatic_admission"]:
    if forbidden in text:
        raise SystemExit(f"forbidden semantic-status shortcut present: {forbidden}")

print("semantic status contract PASS")
